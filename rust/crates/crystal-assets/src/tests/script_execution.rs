#[test]
fn execute_next_queued_script_command_enters_target_script_runtime() {
    let mut module = test_map_module("RuntimeQueueTestMap", "RUNTIME_QUEUE_TEST_MAP", None);
    module
        .scripts
        .insert("RuntimeQueuedScript".to_string(), serde_json::json!([]));
    let data = GameDataSet {
        maps: map_payload(vec![module]),
        ..GameDataSet::default()
    };
    let mut state = GameState::default();
    state
        .script_runtime
        .command_queue
        .push(ScriptRuntimeQueuedCommand {
            origin_map_name: "RuntimeQueueTestMap".to_string(),
            command: "writecmdqueue".to_string(),
            target: "RuntimeQueuedScript".to_string(),
            bank: None,
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 7,
        });
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeQueueTestMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        MapEvents::default(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let audio_ids = BTreeSet::new();

    let outcome = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ExecuteNextQueuedScriptCommand,
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect("queued script command executes");

    let RuntimeMutationResult::QueuedScriptCommandExecuted(queued) = outcome.result else {
        panic!("expected queued script execution result");
    };
    assert_eq!(queued.command, "writecmdqueue");
    assert_eq!(queued.target, "RuntimeQueuedScript");
    assert!(state.script_runtime.command_queue.is_empty());
    assert_eq!(
        state
            .script_runtime
            .next_script
            .as_ref()
            .map(|location| location.script.as_str()),
        Some("RuntimeQueuedScript")
    );
    assert_eq!(state.script_runtime.control_events.len(), 1);
    let event = &state.script_runtime.control_events[0];
    assert_eq!(event.kind, ScriptControlRuntimeKind::Jump);
    assert_eq!(event.target_script.as_deref(), Some("RuntimeQueuedScript"));
    assert_eq!(event.source_script, "RuntimePayloadScript");
    assert_eq!(event.command_index, 7);
}

#[test]
fn execute_next_queued_script_command_rejects_unresolved_target_atomically() {
    let data = GameDataSet {
        maps: map_payload(vec![test_map_module(
            "RuntimeQueueTestMap",
            "RUNTIME_QUEUE_TEST_MAP",
            None,
        )]),
        ..GameDataSet::default()
    };
    let mut state = GameState::default();
    state
        .script_runtime
        .command_queue
        .push(ScriptRuntimeQueuedCommand {
            origin_map_name: "RuntimeQueueTestMap".to_string(),
            command: "writecmdqueue".to_string(),
            target: "MissingQueuedScript".to_string(),
            bank: None,
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 7,
        });
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeQueueTestMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        MapEvents::default(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let before = (state.clone(), session.clone());
    let audio_ids = BTreeSet::new();

    let error = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ExecuteNextQueuedScriptCommand,
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("unresolved queued script pointer must reject");

    assert!(
        error
            .to_string()
            .contains("queued script target MissingQueuedScript cannot resolve"),
        "{error:#}"
    );
    assert_eq!((state, session), before);
}

#[test]
fn execute_next_queued_script_command_rejects_live_map_mismatch_atomically() {
    let mut origin = test_map_module("RuntimeQueueOrigin", "RUNTIME_QUEUE_ORIGIN", None);
    origin
        .scripts
        .insert("RuntimeQueuedScript".to_string(), serde_json::json!([]));
    let data = GameDataSet {
        maps: map_payload(vec![
            origin,
            test_map_module("RuntimeQueueDestination", "RUNTIME_QUEUE_DESTINATION", None),
        ]),
        ..GameDataSet::default()
    };
    let mut state = GameState::default();
    state
        .script_runtime
        .command_queue
        .push(ScriptRuntimeQueuedCommand {
            origin_map_name: "RuntimeQueueOrigin".to_string(),
            command: "writecmdqueue".to_string(),
            target: "RuntimeQueuedScript".to_string(),
            bank: None,
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 7,
        });
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeQueueDestination".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        MapEvents::default(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let before = (state.clone(), session.clone());
    let audio_ids = BTreeSet::new();

    let error = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ExecuteNextQueuedScriptCommand,
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("queued script cannot cross the live map boundary");

    assert!(
            error.to_string().contains(
                "script command map mismatch: session is on RuntimeQueueDestination, request was for RuntimeQueueOrigin"
            ),
            "{error:#}"
        );
    assert_eq!((state, session), before);
}

#[test]
fn runtime_currency_mutation_requires_exact_pack_cap_before_state_change() {
    let data = GameDataSet {
        currency_constants: CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect()),
        ..GameDataSet::default()
    };
    let mut state = GameState {
        money: 123,
        ..GameState::default()
    };
    let before = state.clone();
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeCurrencyMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        MapEvents::default(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let audio_ids = BTreeSet::new();

    let error = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::AddCurrency(RuntimeCurrencyDeltaCommand {
                account: RuntimeCurrencyAccount::Money,
                amount: 1,
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("runtime money mutation requires MAX_MONEY from the pack");

    assert!(error.to_string().contains("MAX_MONEY"), "{error}");
    assert_eq!(state, before);
}

#[test]
fn extracted_numeric_givepoke_materializes_exact_party_gift() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");
    let gift = module
        .gift_pokemon_scripts
        .iter()
        .find(|gift| gift.source_script == "CyndaquilPokeBallScript")
        .expect("Cyndaquil starter gift");
    let mut storage = crystal_core::models::PokemonStorage::default();

    let outcome = give_gift_pokemon(
        &mut storage,
        0,
        &data.pokemon,
        &data.learnsets,
        &data.moves,
        &data.growth_rates,
        &data.items,
        GiftPokemonRequest {
            species_id: gift.species_id.clone(),
            level: gift.level,
            held_item_id: gift.held_item_id.clone(),
            nickname: None,
            original_trainer_name: "PLAYER".to_string(),
            original_trainer_id: 1234,
            caught_data: None,
            source_script: gift.source_script.clone(),
            command_index: gift.command_index,
            egg: gift.egg,
            dvs: Dv::from_non_hp(10, 10, 10, 10),
        },
    )
    .expect("materialize starter gift");

    assert_eq!(outcome.pokemon.species.id, "CYNDAQUIL");
    assert_eq!(outcome.pokemon.level, 5);
    assert_eq!(outcome.pokemon.item.as_deref(), Some("BERRY"));
    assert_eq!(storage.party.filled_slots(), 1);
}

#[test]
fn map_module_extracts_money_script_commands_with_exact_tokens() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("Route43Gate")
        .expect("assemble Route43Gate module");

    let check = module
        .script_economy_commands
        .iter()
        .find(|command| {
            command.source_script == "RocketScript_Southbound" && command.command == "checkmoney"
        })
        .expect("Route43 toll checkmoney");

    assert_eq!(check.account.as_deref(), Some("YOUR_MONEY"));
    assert_eq!(check.amount_tokens, vec!["ROUTE43GATE_TOLL", "-", "1"]);
    assert_eq!(check.command_index, 9);

    let mut state = GameState {
        money: 1_000,
        ..GameState::default()
    };
    assert_eq!(CurrencyCatalog::default().get("ROUTE43GATE_TOLL"), None);
    let constants = economy_constants(&data);
    assert_eq!(constants.get("ROUTE43GATE_TOLL"), Some(1_000));
    assert_eq!(constants.get("route43gate_toll"), None);
    let account = MoneyAccount::from_script_id(check.account.as_deref().expect("account"))
        .expect("exact account");
    let outcome =
        check_money(&state, account, &check.amount_tokens, &constants).expect("check exact toll");
    assert_eq!(outcome.comparison, AmountComparison::HaveMore);
    assert_eq!(
        take_money(
            &mut state,
            account,
            &vec!["ROUTE43GATE_TOLL".to_string()],
            &constants
        ),
        Ok(0)
    );
}

#[test]
fn parser_extracts_givemoney_from_exact_money_command_class() {
    let scripts: BTreeMap<String, Value> = [(
        "PrizeScript".to_string(),
        serde_json::json!([
            {
                "command": "givemoney",
                "args": ["MOMS_MONEY", "MAX_MONEY", "-", "1"]
            },
            {
                "command": "GiveMoney",
                "args": ["YOUR_MONEY", "1"]
            }
        ]),
    )]
    .into_iter()
    .collect();

    let commands =
        parse_script_economy_commands("PrizeMap", &scripts).expect("parse economy commands");

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, "givemoney");
    assert_eq!(commands[0].account.as_deref(), Some("MOMS_MONEY"));
    assert_eq!(commands[0].amount_tokens, vec!["MAX_MONEY", "-", "1"]);
    assert_eq!(commands[0].source_script, "PrizeScript");
    assert_eq!(commands[0].command_index, 0);
}

#[test]
fn parser_extracts_phone_commands_from_exact_command_classes() {
    let scripts: BTreeMap<String, Value> = [(
        "PhoneScript".to_string(),
        serde_json::json!([
            {
                "command": "checkcellnum",
                "args": ["PHONE_MOM"]
            },
            {
                "command": "askforphonenumber",
                "args": ["PHONE_JOEY"]
            },
            {
                "command": "delcellnum",
                "args": ["PHONE_BILL"]
            },
            {
                "command": "CheckCellNum",
                "args": ["PHONE_ELM"]
            }
        ]),
    )]
    .into_iter()
    .collect();

    let commands = parse_script_phone_commands("PhoneMap", &scripts).expect("parse phone commands");

    assert_eq!(commands.len(), 3);
    assert_eq!(commands[0].command, "checkcellnum");
    assert_eq!(commands[0].contact_id, "PHONE_MOM");
    assert_eq!(commands[0].command_index, 0);
    assert_eq!(commands[1].command, "askforphonenumber");
    assert_eq!(commands[1].contact_id, "PHONE_JOEY");
    assert_eq!(commands[1].command_index, 1);
    assert_eq!(commands[2].command, "delcellnum");
    assert_eq!(commands[2].contact_id, "PHONE_BILL");
    assert_eq!(commands[2].command_index, 2);
}

#[test]
fn map_module_extracts_coin_script_commands_with_exact_tokens() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("CeladonGameCornerPrizeRoom")
        .expect("assemble CeladonGameCornerPrizeRoom module");

    let check = module
        .script_economy_commands
        .iter()
        .find(|command| {
            command.source_script == ".Pikachu@CeladonGameCornerPrizeRoomPokemonVendor"
                && command.command == "checkcoins"
        })
        .expect("Celadon Pokemon prize checkcoins");

    assert_eq!(check.account, None);
    assert_eq!(
        check.amount_tokens,
        vec!["CELADONGAMECORNERPRIZEROOM_PIKACHU_COINS"]
    );
    assert_eq!(check.command_index, 0);

    let state = GameState {
        coins: 2_222,
        ..GameState::default()
    };
    let constants = CurrencyCatalog(
        [(
            "CELADONGAMECORNERPRIZEROOM_PIKACHU_COINS".to_string(),
            2_222,
        )]
        .into_iter()
        .collect(),
    );
    let outcome = check_coins(&state, &check.amount_tokens, &constants).expect("check exact coins");
    assert_eq!(outcome.comparison, AmountComparison::HaveAmount);
}

#[test]
fn map_module_extracts_shop_script_commands_with_exact_mart_ids() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let cherrygrove = data
        .map_module("CherrygroveMart")
        .expect("assemble CherrygroveMart module");

    let pre_dex = cherrygrove
        .script_shop_commands
        .iter()
        .find(|command| {
            command.source_script == "CherrygroveMartClerkScript"
                && command.mart_id == "MART_CHERRYGROVE"
        })
        .expect("Cherrygrove pre-dex mart");
    assert_eq!(pre_dex.mart_type, "MARTTYPE_STANDARD");
    assert_eq!(pre_dex.command_index, 3);

    let dex = cherrygrove
        .script_shop_commands
        .iter()
        .find(|command| command.source_script == ".PokeBallsInStock@CherrygroveMartClerkScript")
        .expect("Cherrygrove dex mart branch");
    assert_eq!(dex.mart_type, "MARTTYPE_STANDARD");
    assert_eq!(dex.mart_id, "MART_CHERRYGROVE_DEX");
    assert!(data.marts.0.contains_key(&dex.mart_id));

    let roof = data
        .map_module("GoldenrodDeptStoreRoof")
        .expect("assemble GoldenrodDeptStoreRoof module");
    let rooftop = roof
        .script_shop_commands
        .iter()
        .find(|command| command.source_script == "GoldenrodDeptStoreRoofClerkScript")
        .expect("Goldenrod rooftop mart");
    assert_eq!(rooftop.mart_type, "MARTTYPE_ROOFTOP");
    assert_eq!(rooftop.mart_id, "0");
}

#[test]
fn map_module_extracts_script_field_pickups_with_exact_ids() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let route29 = data.map_module("Route29").expect("assemble Route29 module");
    let potion = route29
        .script_field_pickups
        .iter()
        .find(|pickup| pickup.source_script == "Route29Potion")
        .expect("Route29 itemball pickup");

    assert_eq!(potion.command, "itemball");
    assert_eq!(potion.item_id.as_deref(), Some("POTION"));
    assert_eq!(potion.quantity, 1);
    assert_eq!(potion.event_flag.as_deref(), Some("EVENT_ROUTE_29_POTION"));
    assert_eq!(
        potion.to_field_item_pickup().expect("executable pickup"),
        FieldItemPickup {
            item_id: "POTION".to_string(),
            quantity: 1,
            event_flag: "EVENT_ROUTE_29_POTION".to_string(),
            source: FieldItemSource::ItemBall,
        }
    );

    let fruit = route29
        .script_field_pickups
        .iter()
        .find(|pickup| pickup.source_script == "Route29FruitTree")
        .expect("Route29 fruit tree");
    assert_eq!(fruit.command, "fruittree");
    assert_eq!(fruit.item_id, None);
    assert_eq!(fruit.event_flag, None);
    assert_eq!(fruit.fruit_tree_id.as_deref(), Some("FRUITTREE_ROUTE_29"));
    assert_eq!(
        fruit
            .to_fruit_tree_pickup(&data.fruit_trees)
            .expect("executable fruit tree"),
        FieldItemPickup {
            item_id: "BERRY".to_string(),
            quantity: 1,
            event_flag: "FRUITTREE_ROUTE_29_COLLECTED".to_string(),
            source: FieldItemSource::FruitTree,
        }
    );

    let route13 = data.map_module("Route13").expect("assemble Route13 module");
    let hidden = route13
        .script_field_pickups
        .iter()
        .find(|pickup| pickup.source_script == "Route13HiddenCalcium")
        .expect("Route13 hidden item");
    assert_eq!(hidden.command, "hiddenitem");
    assert_eq!(hidden.item_id.as_deref(), Some("CALCIUM"));
    assert_eq!(
        hidden.event_flag.as_deref(),
        Some("EVENT_ROUTE_13_HIDDEN_CALCIUM")
    );
}

#[test]
fn route29_itemball_pickup_uses_exact_pack_item_and_object_flag() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble Route29 module");
    let object = module
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some("ROUTE29_POKE_BALL"))
        .expect("Route29 itemball object");
    let mut state = GameState::default();

    let outcome = pickup_field_item(
        &mut state,
        &data.items,
        FieldItemPickup {
            item_id: "POTION".to_string(),
            quantity: 1,
            event_flag: object.event_flag.clone(),
            source: FieldItemSource::ItemBall,
        },
    )
    .expect("pick up potion");

    assert_eq!(
        outcome,
        FieldItemPickupOutcome::Collected {
            item_id: "POTION".to_string(),
            quantity: 1,
            event_flag: "EVENT_ROUTE_29_POTION".to_string(),
            source: FieldItemSource::ItemBall,
        }
    );
    assert_eq!(state.bag.items["POTION"], 1);
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_ROUTE_29_POTION"),
        Ok(true)
    );
}

#[test]
fn route29_fruit_tree_pickup_uses_exact_pack_catalog_without_default_item() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let route29 = data.map_module("Route29").expect("assemble Route29 module");
    let pickup = route29
        .script_field_pickups
        .iter()
        .find(|pickup| pickup.source_script == "Route29FruitTree")
        .expect("Route29 fruit tree")
        .clone();
    let mut state = GameState::default();

    let outcome = pickup_script_field_item(&mut state, &data.items, &data.fruit_trees, pickup)
        .expect("fruit tree pickup");

    assert_eq!(
        outcome,
        FieldItemPickupOutcome::Collected {
            item_id: "BERRY".to_string(),
            quantity: 1,
            event_flag: "FRUITTREE_ROUTE_29_COLLECTED".to_string(),
            source: FieldItemSource::FruitTree,
        }
    );
    assert_eq!(state.bag.items["BERRY"], 1);
    assert_eq!(
        state
            .flags
            .is_event_flag_set("FRUITTREE_ROUTE_29_COLLECTED"),
        Ok(true)
    );
    assert!(
        data.saved_event_flag_exists("FRUITTREE_ROUTE_29_COLLECTED"),
        "fruit tree collected flags must be declared by the compiled fruit tree catalog"
    );
    data.validate_saved_flag_references(&state.flags)
        .expect("fruit tree collected flag validates against compiled pack");
}

#[test]
fn johto_tileset_collision_loads_from_controlled_runtime_assets() {
    let root = repository_root_for_tests();
    let tileset = AssetRoot::new(root)
        .load_tileset_collision("johto")
        .expect("load johto collision");

    assert!(tileset.metatiles.len() > 100);
    assert_eq!(
        tileset.metatiles[17].collision,
        [
            permissions::WALL,
            permissions::WALL,
            permissions::WALL,
            permissions::WALL
        ]
    );
    assert_eq!(resolve_collision_token("ICE").expect("resolve ice"), 0x23);
    assert_eq!(
        resolve_collision_token("ICE_2B").expect("resolve alternate ice"),
        0x2b
    );
    assert_eq!(
        resolve_collision_token("WATERFALL").expect("resolve waterfall"),
        permissions::WATERFALL
    );
    assert_eq!(
        resolve_collision_token("HOP_UP_LEFT").expect("resolve ledge"),
        permissions::HOP_UP_LEFT
    );
    assert_eq!(
        resolve_collision_token("5B").expect("resolve garbage collision"),
        0x5b
    );
    assert!(
        resolve_collision_token(" 5B")
            .expect_err("collision tokens must not be trim-parsed")
            .to_string()
            .contains("must be exact and non-empty")
    );
    assert!(
        parse_metatile_id(" 10")
            .expect_err("metatile ids must not be trim-parsed")
            .to_string()
            .contains("must be exact and non-empty")
    );
    assert_eq!(
        resolve_collision_token("unknown")
            .expect_err("unknown token")
            .to_string(),
        "unknown collision token unknown"
    );
}

#[test]
fn route29_overworld_session_steps_with_real_map_and_tileset_data() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let stride = StepOptions::default().stride_tiles;
    let (tile_width, tile_height) = map.tile_bounds();
    let tile_width =
        i16::try_from(tile_width).expect("Route29 runtime tile width fits i16 coordinates");
    let tile_height =
        i16::try_from(tile_height).expect("Route29 runtime tile height fits i16 coordinates");
    let start = (0..tile_height)
        .flat_map(|y| (0..tile_width).map(move |x| TilePosition::new(x, y)))
        .find(|tile| {
            let Some(target) = checked_move_by_stride(*tile, Direction::Right, stride) else {
                return false;
            };
            can_enter_tile(
                &map,
                &tileset,
                target,
                Direction::Right,
                PlayerTraversalState::Walk,
            )
        })
        .expect("walkable Route29 rightward step");
    let expected_target = checked_move_by_stride(start, Direction::Right, stride)
        .expect("selected Route29 start must have an in-bounds rightward target");
    let mut session = OverworldSession::new(map, tileset, start);

    let outcome = session.step(
        Direction::Right,
        StepOptions {
            force_step_after_turn: true,
            ..StepOptions::default()
        },
    );

    assert_eq!(
        outcome,
        StepOutcome::Moved {
            from: start,
            to: expected_target,
            speed_multiplier: 1,
        }
    );
    assert_eq!(session.snapshot().map_name, "Route29");
    assert_eq!(session.snapshot().frame, 1);
    assert_ne!(session.state_hash(), 0);
}

#[test]
fn route29_overworld_session_reports_pack_backed_warp_event() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let warp = module.events.warps[0].clone();
    let warp_tile = warp_tile_position_checked(&warp).expect("Route29 warp coordinate");
    let start_tile = checked_move_by_stride(
        warp_tile,
        Direction::Right,
        StepOptions::default().stride_tiles,
    )
    .expect("Route29 warp test tile must have an in-bounds rightward start");
    let mut session = OverworldSession::with_events(
        map.clone(),
        module.events.clone(),
        tileset.clone(),
        start_tile,
    );

    let standing_session =
        OverworldSession::with_events(map, module.events.clone(), tileset, warp_tile);
    let trigger = standing_session
        .check_warp_checked()
        .expect("checked Route29 warp")
        .expect("Route29 warp trigger");
    assert_eq!(trigger.map_name, "Route29");
    assert_eq!(trigger.tile, warp_tile);
    assert_eq!(trigger.warp.index, warp.index);
    assert_eq!(trigger.warp.target_map_constant, "ROUTE_29_ROUTE_46_GATE");
    assert_eq!(trigger.warp.target_map, "ROUTE_29_ROUTE_46_GATE");
    assert_eq!(trigger.warp.target_warp_id, 3);

    let result = session
        .step_and_check_warp_checked(
            Direction::Left,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        )
        .expect("checked Route29 step and warp");
    assert_eq!(
        result.outcome,
        StepOutcome::Moved {
            from: start_tile,
            to: warp_tile,
            speed_multiplier: 1,
        }
    );
    assert_eq!(
        result.warp.expect("warp fires after movement").tile,
        warp_tile
    );
}

#[test]
fn route29_warp_transition_resolves_destination_from_pack_constants() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let warp = module.events.warps[0].clone();
    let start = warp_tile_position_checked(&warp).expect("Route29 warp coordinate");
    let session = OverworldSession::with_events(map, module.events.clone(), tileset, start);
    let trigger = session
        .check_warp_checked()
        .expect("checked Route29 warp")
        .expect("Route29 warp trigger");

    let transition = data
        .resolve_warp_transition(&trigger)
        .expect("resolve Route29 warp destination");

    assert_eq!(transition.trigger, trigger);
    assert_eq!(transition.destination.map_name, "Route29Route46Gate");
    assert_eq!(transition.destination.warp.index, 3);
    assert_eq!(transition.destination.warp.target_map_constant, "ROUTE_29");
    assert_eq!(
        transition.destination.tile,
        warp_tile_position_checked(&transition.destination.warp)
            .expect("Route29 destination warp coordinate")
    );

    let destination_module = data
        .map_module(&transition.destination.map_name)
        .expect("load destination module");
    let destination_map = data
        .overworld_map(&transition.destination.map_name)
        .expect("load destination map");
    let destination_tileset = asset_root
        .load_tileset_collision(&destination_module.attributes.tileset_name)
        .expect("load destination tileset");
    let destination_session = transition.apply_to(
        destination_map,
        destination_module.events.clone(),
        destination_module.objects.clone(),
        destination_tileset,
        session.frame + 1,
        MovementMode::Normal,
    );

    assert_eq!(
        destination_session.snapshot().map_name,
        "Route29Route46Gate"
    );
    assert_eq!(
        destination_session.snapshot().tile,
        transition.destination.tile
    );
    assert_eq!(destination_session.snapshot().frame, 1);
}

#[test]
fn warp_transition_requires_declared_target_map_constant() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let trigger = WarpTrigger {
        map_name: "Route29".to_string(),
        tile: TilePosition::new(54, 2),
        permission: 0x71,
        warp: WarpEvent {
            index: 1,
            x: 27,
            y: 1,
            target_map_constant: "MISSING_TARGET_MAP".to_string(),
            target_map: "MISSING_TARGET_MAP".to_string(),
            target_warp_id: 1,
        },
    };

    let error = data
        .resolve_warp_transition(&trigger)
        .expect_err("missing target map constant");

    assert!(
        error
            .to_string()
            .contains("unknown target map constant 'MISSING_TARGET_MAP'")
    );
}

#[test]
fn warp_transition_rejects_mismatched_target_map_field() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let trigger = WarpTrigger {
        map_name: "Route29".to_string(),
        tile: TilePosition::new(54, 2),
        permission: 0x71,
        warp: WarpEvent {
            index: 1,
            x: 27,
            y: 1,
            target_map_constant: "ROUTE_29_ROUTE_46_GATE".to_string(),
            target_map: "ROUTE_29".to_string(),
            target_warp_id: 1,
        },
    };

    let error = data
        .resolve_warp_transition(&trigger)
        .expect_err("target_map must match target_map_constant");

    assert!(
        error.to_string().contains(
            "target_map \"ROUTE_29\" does not match target_map_constant \"ROUTE_29_ROUTE_46_GATE\""
        ),
        "{error}"
    );
}

#[test]
fn warp_transition_does_not_fallback_to_target_map_module_events() {
    let mut data = GameDataSet::default();
    data.maps.insert(
        "Target".to_string(),
        test_map_module("Target", "TARGET", None),
    );
    let trigger = WarpTrigger {
        map_name: "Source".to_string(),
        tile: TilePosition::new(2, 2),
        permission: 0x71,
        warp: WarpEvent {
            index: 1,
            x: 1,
            y: 1,
            target_map_constant: "TARGET".to_string(),
            target_map: "TARGET".to_string(),
            target_warp_id: 1,
        },
    };

    let error = data
        .resolve_warp_transition(&trigger)
        .expect_err("warp transition requires the map_attributes section");

    assert!(
        error
            .to_string()
            .contains("warp target 'Target' missing attributes"),
        "{error}"
    );
}

#[test]
fn warp_transition_requires_declared_target_map_events_label() {
    let mut data = GameDataSet::default();
    data.maps.insert(
        "Target".to_string(),
        test_map_module("Target", "TARGET", None),
    );
    data.map_attributes.insert(
        "Target".to_string(),
        MapAttributes {
            tileset_name: "johto".to_string(),
            border_block: 5,
            width: 1,
            height: 1,
            connections: Vec::new(),
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: None,
            location: None,
            music: None,
            palette: None,
            fishing_group: None,
            map_constant: Some("TARGET".to_string()),
            map_group_constant: None,
            blocks_label: None,
            map_scripts_label: None,
            map_events_label: None,
            connection_flags: None,
        },
    );
    let trigger = WarpTrigger {
        map_name: "Source".to_string(),
        tile: TilePosition::new(2, 2),
        permission: 0x71,
        warp: WarpEvent {
            index: 1,
            x: 1,
            y: 1,
            target_map_constant: "TARGET".to_string(),
            target_map: "TARGET".to_string(),
            target_warp_id: 1,
        },
    };

    let error = data
        .resolve_warp_transition(&trigger)
        .expect_err("warp transition requires target events label");

    assert!(
        format!("{error:#}").contains("missing map_events_label for map Target"),
        "{error}"
    );
}

#[test]
fn route29_east_connection_resolves_destination_from_pack_attributes() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let mut session = OverworldSession::with_events(
        map,
        module.events.clone(),
        tileset,
        TilePosition::new(59, 4),
    );

    let outcome = session.step(
        Direction::Right,
        StepOptions {
            force_step_after_turn: true,
            ..StepOptions::default()
        },
    );
    assert_eq!(
        outcome,
        StepOutcome::Moved {
            from: TilePosition::new(59, 4),
            to: TilePosition::new(60, 4),
            speed_multiplier: 1,
        }
    );

    let trigger = session
        .check_connection_checked()
        .expect("checked Route29 connection")
        .expect("Route29 east connection");
    assert_eq!(trigger.connection.direction, "east");
    assert_eq!(trigger.connection.target_map, "NewBarkTown");

    let transition = data
        .resolve_connection_transition(&trigger)
        .expect("resolve Route29 east connection");

    assert_eq!(transition.trigger, trigger);
    assert_eq!(transition.destination.map_name, "NewBarkTown");
    assert_eq!(transition.destination.tile, TilePosition::new(0, 4));

    let destination_module = data
        .map_module(&transition.destination.map_name)
        .expect("load destination module");
    let destination_map = data
        .overworld_map(&transition.destination.map_name)
        .expect("load destination map");
    let destination_tileset = asset_root
        .load_tileset_collision(&destination_module.attributes.tileset_name)
        .expect("load destination tileset");
    session = transition.apply_to(
        destination_map,
        destination_module.events.clone(),
        destination_module.objects.clone(),
        destination_tileset,
        session.frame,
        MovementMode::Normal,
    );

    assert_eq!(session.snapshot().map_name, "NewBarkTown");
    assert_eq!(session.snapshot().tile, TilePosition::new(0, 4));
    assert_eq!(session.snapshot().frame, 1);
}

#[test]
fn route_connections_arrive_on_the_continuous_asm_boundary_tile() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");

    for (source_map, direction, target_map, trigger_tile, expected_tile) in [
        (
            "NewBarkTown",
            "west",
            "Route29",
            TilePosition::new(-1, 4),
            TilePosition::new(59, 4),
        ),
        (
            "Route26",
            "west",
            "Route27",
            TilePosition::new(-1, 94),
            TilePosition::new(79, 4),
        ),
        (
            "Route29",
            "north",
            "Route46",
            TilePosition::new(20, -1),
            TilePosition::new(0, 35),
        ),
    ] {
        let source = data
            .map_module(source_map)
            .unwrap_or_else(|error| panic!("load {source_map}: {error:#}"));
        let connection = source
            .attributes
            .connections
            .iter()
            .find(|connection| {
                connection.direction == direction && connection.target_map == target_map
            })
            .unwrap_or_else(|| {
                panic!("missing {source_map} {direction} connection to {target_map}")
            });
        let trigger = ConnectionTrigger {
            map_name: source_map.to_string(),
            tile: trigger_tile,
            connection: connection.clone(),
        };

        let transition = data
            .resolve_connection_transition(&trigger)
            .unwrap_or_else(|error| panic!("resolve {source_map} -> {target_map}: {error:#}"));

        assert_eq!(transition.destination.map_name, target_map);
        assert_eq!(transition.destination.tile, expected_tile);
    }
}

#[test]
fn parser_extracts_callstd_as_a_standard_script_call() {
    let scripts: BTreeMap<String, Value> = [(
        "StandardCaller".to_string(),
        serde_json::json!([
            {"command": "callstd", "args": ["PokecenterNurseScript"]},
            {"command": "CallStd", "args": ["Ignored"]}
        ]),
    )]
    .into_iter()
    .collect();

    let commands =
        parse_script_control_commands("TestMap", &scripts).expect("parse standard call command");

    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].command, "callstd");
    assert_eq!(
        commands[0].target_label.as_deref(),
        Some("PokecenterNurseScript")
    );
    assert_eq!(commands[0].resolved_target_script, None);
    assert_eq!(commands[0].command_index, 0);
}

#[test]
fn route29_declared_connections_resolve_from_reachable_pack_edges() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let mut diagnostics = Vec::new();
    let context = map_playability_context_from_parts(
        &data,
        &module.id,
        &module.attributes,
        module.blocks.clone(),
        &PlayabilityRules::default(),
        &mut diagnostics,
    )
    .unwrap_or_else(|| panic!("Route29 playability diagnostics: {diagnostics:#?}"));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let mut resolved = BTreeMap::new();
    for connection in &module.attributes.connections {
        let target_module = data
            .map_module(&connection.target_map)
            .expect("load destination module");
        let Some(source) =
            connection_source_tile_for_target(&context, connection, &target_module.attributes)
        else {
            assert_eq!(connection.direction, "north");
            continue;
        };
        let direction = match connection.direction.as_str() {
            "north" => Direction::Up,
            "south" => Direction::Down,
            "west" => Direction::Left,
            "east" => Direction::Right,
            other => panic!("unsupported Route29 direction {other}"),
        };
        let mut session = OverworldSession::with_events(
            map.clone(),
            module.events.clone(),
            tileset.clone(),
            source,
        );
        let outcome = session.step(
            direction,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );
        let trigger_tile = connection_trigger_tile_from_source(source, connection)
            .unwrap_or_else(|| panic!("missing trigger tile for {connection:?}"));
        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: source,
                to: trigger_tile,
                speed_multiplier: 1,
            },
            "Route29 {} connection should step from the reachable border tile",
            connection.direction
        );

        let trigger = session
            .check_connection_checked()
            .expect("checked Route29 connection")
            .unwrap_or_else(|| {
                panic!(
                    "Route29 {} connection did not trigger",
                    connection.direction
                )
            });
        assert_eq!(trigger.connection, *connection);
        assert_eq!(trigger.tile, trigger_tile);

        let transition = data
            .resolve_connection_transition(&trigger)
            .expect("resolve Route29 connection");
        let destination_module = data
            .map_module(&transition.destination.map_name)
            .expect("load destination module");
        let destination_map = data
            .overworld_map(&transition.destination.map_name)
            .expect("load destination map");
        let destination_tileset = asset_root
            .load_tileset_collision(&destination_module.attributes.tileset_name)
            .expect("load destination tileset");
        let destination_session = transition.apply_to(
            destination_map,
            destination_module.events.clone(),
            destination_module.objects.clone(),
            destination_tileset,
            session.frame,
            MovementMode::Normal,
        );
        assert_eq!(
            destination_session.snapshot().map_name,
            connection.target_map
        );
        assert_eq!(
            destination_session.snapshot().tile,
            transition.destination.tile
        );
        assert_eq!(destination_session.snapshot().frame, 1);
        resolved.insert(
            connection.direction.clone(),
            (
                connection.target_map.clone(),
                source,
                trigger_tile,
                transition.destination.tile,
            ),
        );
    }

    assert_eq!(
        resolved.keys().cloned().collect::<Vec<_>>(),
        vec!["east".to_string(), "west".to_string()]
    );
    assert_eq!(resolved["east"].0, "NewBarkTown");
    assert_eq!(resolved["west"].0, "CherrygroveCity");
}

#[test]
fn compiled_core_pack_route29_east_connection_uses_reachable_runtime_edge() {
    let root = repository_root_for_tests();
    let pack =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack");
    let data = pack.data;
    data.overworld_session("Route29", TilePosition::new(59, 4), 0)
        .expect("compiled pack Route29 east source starts a runtime session");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = data
        .tileset_collision("johto")
        .expect("load compiled johto collision");
    let mut session = OverworldSession::with_events(
        map,
        module.events.clone(),
        tileset,
        TilePosition::new(59, 4),
    );

    let outcome = session.step(
        Direction::Right,
        StepOptions {
            force_step_after_turn: true,
            ..StepOptions::default()
        },
    );
    assert_eq!(
        outcome,
        StepOutcome::Moved {
            from: TilePosition::new(59, 4),
            to: TilePosition::new(60, 4),
            speed_multiplier: 1,
        }
    );

    let trigger = session
        .check_connection_checked()
        .expect("checked Route29 connection")
        .expect("Route29 east connection");
    assert_eq!(trigger.connection.direction, "east");
    assert_eq!(trigger.connection.target_map, "NewBarkTown");
}

#[test]
fn map_connection_preserves_the_live_step_direction_on_the_destination_map() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let mut state = GameState::default();
    let mut session = OverworldSession::with_events(
        map,
        module.events.clone(),
        tileset,
        TilePosition::new(59, 4),
    );
    session.player.facing = Direction::Right;
    let music_ids = data
        .audio
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<BTreeSet<_>>();

    let frame = data
        .apply_overworld_input(
            &mut state,
            &mut session,
            [GameButton::Right],
            &music_ids,
            &mut ReplayDivider::new([]),
        )
        .expect("cross Route29's east connection");

    assert!(frame.connection.is_some());
    assert_eq!(frame.snapshot.map_name, "NewBarkTown");
    assert_eq!(frame.snapshot.tile, TilePosition::new(0, 4));
    assert_eq!(frame.snapshot.facing, Direction::Right);
    assert_eq!(session.player.facing, Direction::Right);
    assert_eq!(session.last_step_direction, Some(Direction::Right));
}

#[test]
fn route29_input_blocks_connection_edge_outside_target_overlap() {
    let root = repository_root_for_tests();
    let asset_root = AssetRoot::new(root);
    let data = asset_root
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("Route29").expect("assemble route module");
    let map = data.overworld_map("Route29").expect("assemble route map");
    let tileset = asset_root
        .load_tileset_collision("johto")
        .expect("load johto collision");
    let north = module
        .attributes
        .connections
        .iter()
        .find(|connection| connection.direction == "north")
        .expect("Route29 north connection");
    let route46 = data.map_module("Route46").expect("assemble Route46 module");
    // x=0 is on Route 29's north edge but lies outside Route 46's
    // offset-10-metatile overlap. The canonical map currently has no
    // walkable tile at this edge position, so exercise the runtime's
    // rejected connection boundary directly instead of relying on a
    // historical collision layout.
    let source = TilePosition::new(0, 0);
    let blocked_target =
        connection_trigger_tile_from_source(source, north).expect("north trigger tile");
    assert!(
        !connection_destination_tile_in_bounds(
            blocked_target,
            &north.direction,
            north.offset,
            &route46.attributes,
        )
        .expect("check north destination bounds")
    );

    let mut state = GameState::default();
    let mut session = OverworldSession::with_events(map, module.events.clone(), tileset, source);
    session.player.facing = Direction::Up;
    let music_ids = data
        .audio
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<BTreeSet<_>>();

    let frame = data
        .apply_overworld_input(
            &mut state,
            &mut session,
            [GameButton::Up],
            &music_ids,
            &mut ReplayDivider::new([]),
        )
        .expect("apply blocked out-of-overlap connection input");

    assert_eq!(
        frame.movement,
        Some(StepOutcome::Blocked {
            at: blocked_target,
            facing: Direction::Up,
        })
    );
    assert_eq!(frame.connection, None);
    assert_eq!(session.player.tile, source);
    assert_eq!(session.frame, 1);
}

#[test]
fn connection_transition_requires_declared_target_map_attributes() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let trigger = ConnectionTrigger {
        map_name: "Route29".to_string(),
        tile: TilePosition::new(60, 4),
        connection: MapConnection {
            direction: "east".to_string(),
            target_map: "MissingTarget".to_string(),
            offset: 0,
        },
    };

    let error = data
        .resolve_connection_transition(&trigger)
        .expect_err("missing connection target");

    assert!(
        error
            .to_string()
            .contains("connection target 'MissingTarget' missing attributes")
    );
}

#[test]
fn connection_transition_does_not_fallback_to_target_map_module_attributes() {
    let mut data = GameDataSet::default();
    data.maps.insert(
        "Target".to_string(),
        test_map_module("Target", "TARGET", None),
    );
    let trigger = ConnectionTrigger {
        map_name: "Source".to_string(),
        tile: TilePosition::new(2, 2),
        connection: MapConnection {
            direction: "east".to_string(),
            target_map: "Target".to_string(),
            offset: 0,
        },
    };

    let error = data
        .resolve_connection_transition(&trigger)
        .expect_err("connection transition requires the map_attributes section");

    assert!(
        error
            .to_string()
            .contains("connection target 'Target' missing attributes"),
        "{error}"
    );
}

#[test]
fn connection_transition_rejects_out_of_bounds_destination_without_clamping() {
    let mut data = GameDataSet::default();
    data.map_attributes.insert(
        "Target".to_string(),
        MapAttributes {
            tileset_name: "johto".to_string(),
            border_block: 5,
            width: 2,
            height: 2,
            connections: Vec::new(),
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: None,
            location: None,
            music: None,
            palette: None,
            fishing_group: None,
            map_constant: Some("TARGET".to_string()),
            map_group_constant: None,
            blocks_label: None,
            map_scripts_label: None,
            map_events_label: None,
            connection_flags: None,
        },
    );
    let trigger = ConnectionTrigger {
        map_name: "Source".to_string(),
        tile: TilePosition::new(60, 99),
        connection: MapConnection {
            direction: "east".to_string(),
            target_map: "Target".to_string(),
            offset: 0,
        },
    };

    let error = data
        .resolve_connection_transition(&trigger)
        .expect_err("out-of-bounds destination must be rejected");

    assert!(
        error
            .to_string()
            .contains("connection destination tile (0, 99) is outside target map")
    );
}

#[test]
fn overworld_map_requires_explicit_blocks_label_and_payload() {
    let mut data = GameDataSet::default();
    data.map_attributes.insert(
        "MissingBlocks".to_string(),
        MapAttributes {
            tileset_name: "johto".to_string(),
            border_block: 5,
            width: 1,
            height: 1,
            connections: Vec::new(),
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: None,
            location: None,
            music: None,
            palette: None,
            fishing_group: None,
            map_constant: Some("MISSING_BLOCKS".to_string()),
            map_group_constant: None,
            blocks_label: None,
            map_scripts_label: None,
            map_events_label: None,
            connection_flags: None,
        },
    );

    let error = data
        .overworld_map("MissingBlocks")
        .expect_err("blocks label is required");

    assert!(error.to_string().contains("missing blocks_label"));
}

#[test]
fn runtime_map_assembly_rejects_malformed_attribute_labels_without_missing_fallback() {
    let mut data = GameDataSet::default();
    let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
    attributes.blocks_label = Some(" Route29_Blocks".to_string());
    data.map_attributes
        .insert("Route29".to_string(), attributes.clone());

    let error = data
        .overworld_map("Route29")
        .expect_err("malformed blocks label must not be treated as missing");
    assert!(
        format!("{error:#}")
            .contains("map attributes blocks_label ' Route29_Blocks' must be an exact map token"),
        "{error:#}"
    );

    attributes.blocks_label = Some("Route29_Blocks".to_string());
    attributes.map_scripts_label = Some(" Route29_MapScripts".to_string());
    data.map_attributes
        .insert("Route29".to_string(), attributes);

    let error = data
        .assemble_map_module_from_compiled_payloads("Route29")
        .expect_err("malformed map scripts label must not be treated as missing");
    assert!(
        format!("{error:#}").contains(
            "map attributes map_scripts_label ' Route29_MapScripts' must be an exact map token"
        ),
        "{error:#}"
    );
}

#[test]
fn map_block_base64_decoder_rejects_invalid_payloads() {
    assert_eq!(decode_base64_bytes("AQID").expect("decode"), vec![1, 2, 3]);
    assert!(decode_base64_bytes("AQI").is_err());
    assert!(decode_base64_bytes("AQ@D").is_err());
}

#[test]
fn modpack_overlay_rejects_duplicate_wild_encounters_by_map_name() {
    let route = WildEncounterData {
        map_name: "NEW_ROUTE".to_string(),
        grass_rates: Some(
            [
                ("morning".to_string(), 20),
                ("day".to_string(), 20),
                ("night".to_string(), 20),
            ]
            .into_iter()
            .collect(),
        ),
        grass: Some(WildEncounterTable {
            morning: vec![WildEncounter {
                level: 3,
                species: "NEW_MON".to_string(),
            }],
            day: vec![WildEncounter {
                level: 3,
                species: "NEW_MON".to_string(),
            }],
            night: vec![WildEncounter {
                level: 3,
                species: "NEW_MON".to_string(),
            }],
            ..WildEncounterTable::default()
        }),
        ..WildEncounterData::default()
    };
    let replacement = WildEncounterData {
        grass: Some(WildEncounterTable {
            morning: vec![WildEncounter {
                level: 5,
                species: "BULBASAUR".to_string(),
            }],
            day: vec![WildEncounter {
                level: 5,
                species: "BULBASAUR".to_string(),
            }],
            night: vec![WildEncounter {
                level: 5,
                species: "BULBASAUR".to_string(),
            }],
            ..WildEncounterTable::default()
        }),
        ..route.clone()
    };
    let mut data = GameDataSet {
        wild_encounters: [("NEW_ROUTE".to_string(), route)].into_iter().collect(),
        ..GameDataSet::default()
    };
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            wild_encounters: wild_encounter_payload(vec![replacement]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };

    let error = data
        .apply_modpack(&manifest)
        .expect_err("duplicate wild encounter manifest must not overwrite");

    assert!(
        format!("{error:#}").contains("duplicate wild encounter data for map 'NEW_ROUTE'"),
        "{error:#}"
    );
}

#[test]
fn modpack_overlay_rejects_duplicate_field_encounters_by_map_name() {
    let route = FieldEncounterData::for_crystal(
        "NEW_ROUTE",
        Some(FieldEncounterTable {
            common: vec![FieldEncounterEntry {
                weight: 100,
                species: "NEW_MON".to_string(),
                level: 5,
                sleep_turns_by_time: Default::default(),
            }],
            rare: Vec::new(),
        }),
        None,
    );
    let replacement = FieldEncounterData::for_crystal(
        "NEW_ROUTE",
        None,
        Some(FieldEncounterTable {
            common: vec![FieldEncounterEntry {
                weight: 100,
                species: "NEW_MON".to_string(),
                level: 5,
                sleep_turns_by_time: Default::default(),
            }],
            rare: Vec::new(),
        }),
    );
    let mut data = GameDataSet {
        field_encounters: [("NEW_ROUTE".to_string(), route)].into_iter().collect(),
        ..GameDataSet::default()
    };
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            field_encounters: field_encounter_payload(vec![replacement]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };

    let error = data
        .apply_modpack(&manifest)
        .expect_err("duplicate field encounter manifest must not overwrite");

    assert!(
        format!("{error:#}").contains("duplicate field encounter data for map 'NEW_ROUTE'"),
        "{error:#}"
    );
}

#[test]
fn modpack_items_require_explicit_script_name() {
    let item = Item {
        name: "Flash Step Charm".to_string(),
        description: "A modded item.".to_string(),
        effect: "NONE".to_string(),
        status_heals: Vec::new(),
        revive_hp_percent: None,
        party_revive_hp_percent: None,
        pp_restore_scope: None,
        pp_restore_points: None,
        pp_up_stages: None,
        vitamin_stat: None,
        vitamin_stat_exp: None,
        vitamin_max_stat_exp: None,
        rare_candy_level_gain: None,
        battle_stat_boost_stat: None,
        battle_stat_boost_stages: None,
        battle_escape_mode: None,
        battle_capture_ball: None,
        battle_focus_energy: None,
        battle_stat_drop_guard: None,
        battle_stat_drop_guard_turns: None,
        confusion_heal: None,
        repel_steps: None,
        escape_rope_mode: None,
        price: 100,
        held_effect: "HELD_NONE".to_string(),
        parameter: 0,
        property: String::new(),
        pocket: item_pocket("ITEM"),
        field_menu: "ITEMMENU_PARTY".to_string(),
        field_usable: true,
        battle_menu: "ITEMMENU_PARTY".to_string(),
        battle_usable: true,
        script_name: String::new(),
        consumable: false,
        tmhm_index: None,
        tmhm_move: None,
    };
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("missing item ids must not be derived from display names");

    assert!(
        error
            .to_string()
            .contains("item 'Flash Step Charm' is missing explicit script_name")
    );
}

#[test]
fn modpack_items_reject_invalid_script_name_without_coercion() {
    let mut item = test_item("MOD_ITEM");
    item.name = "Flash Step Charm".to_string();
    item.script_name = " MOD_ITEM".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item id must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'Flash Step Charm' has invalid script_name ' MOD_ITEM'")
    );
}

#[test]
fn modpack_items_reject_invalid_display_name_without_inference() {
    let mut item = test_item("MOD_ITEM");
    item.name = " Flash Step Charm".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item display name must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid name ' Flash Step Charm'")
    );
}

#[test]
fn modpack_items_reject_invalid_description_without_inference() {
    let mut item = test_item("MOD_ITEM");
    item.description = " A charm with exact text.".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item description must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid description ' A charm with exact text.'")
    );
}

#[test]
fn modpack_items_reject_invalid_pocket_without_enum_restriction() {
    let mut item = test_item("MOD_ITEM");
    item.pocket = " BATTLE_PASS".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item pocket must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid pocket ' BATTLE_PASS'")
    );
}

#[test]
fn modpack_items_reject_invalid_effect_without_enum_restriction() {
    let mut item = test_item("MOD_ITEM");
    item.effect = " MODDED_FLASH_STEP".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item effect must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid effect ' MODDED_FLASH_STEP'")
    );
}

#[test]
fn modpack_items_reject_invalid_held_effect_without_enum_restriction() {
    let mut item = test_item("MOD_ITEM");
    item.held_effect = " HELD_MODDED".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item held effect must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid held_effect ' HELD_MODDED'")
    );
}

#[test]
fn modpack_items_reject_invalid_property_without_requiring_property() {
    let mut item = test_item("MOD_ITEM");
    item.property = " CANT_SELECT".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item property must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid property ' CANT_SELECT'")
    );
}

#[test]
fn modpack_items_reject_invalid_menu_without_enum_restriction() {
    let mut item = test_item("MOD_ITEM");
    item.field_menu = " ITEMMENU_MODDED".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            items: item_payload(vec![item]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("invalid item menu must not be trimmed");

    assert!(
        error
            .to_string()
            .contains("item 'MOD_ITEM' has invalid field_menu ' ITEMMENU_MODDED'")
    );
}

#[test]
fn modpack_items_reject_invalid_structured_payloads_at_load_time() {
    let cases: Vec<(&str, fn(&mut Item), &str)> = vec![
        (
            "status heal",
            |item| item.status_heals.push(" PSN".to_string()),
            "item 'MOD_ITEM' status_heals[0] has invalid status ' PSN'",
        ),
        (
            "pp restore scope",
            |item| item.pp_restore_points = Some(10),
            "item 'MOD_ITEM' must declare explicit pp_restore_scope",
        ),
        (
            "vitamin stat",
            |item| {
                item.vitamin_stat = Some("SP ATK".to_string());
                item.vitamin_stat_exp = Some(2560);
                item.vitamin_max_stat_exp = Some(25600);
            },
            "item 'MOD_ITEM' has invalid vitamin_stat 'SP ATK'",
        ),
        (
            "battle boost stages",
            |item| {
                item.battle_stat_boost_stat = Some("ATTACK".to_string());
                item.battle_stat_boost_stages = Some(0);
            },
            "item 'MOD_ITEM' has invalid battle_stat_boost_stages 0",
        ),
        (
            "battle escape mode",
            |item| item.battle_escape_mode = Some("TRAINER_BATTLE".to_string()),
            "item 'MOD_ITEM' has invalid battle_escape_mode 'TRAINER_BATTLE'",
        ),
        (
            "repel steps",
            |item| item.repel_steps = Some(0),
            "item 'MOD_ITEM' has invalid repel_steps 0",
        ),
        (
            "false focus energy",
            |item| item.battle_focus_energy = Some(false),
            "item 'MOD_ITEM' has invalid battle_focus_energy false",
        ),
    ];

    for (label, mutate, expected) in cases {
        let mut item = test_item("MOD_ITEM");
        mutate(&mut item);
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: item_payload(vec![item]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        let error = match data.apply_modpack(&manifest) {
            Ok(()) => panic!("{label} must fail at modpack load time"),
            Err(error) => error,
        };

        assert!(
            error.to_string().contains(expected),
            "{label} produced unexpected error: {error:#}"
        );
    }
}

fn explicit_empty_manifest_json() -> Value {
    let mut manifest = serde_json::to_value(ModpackManifest {
        metadata: ModpackMetadata {
            id: "empty-pack".to_string(),
            name: "Empty Pack".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        payload: ModpackPayload {
            roaming_pokemon: roaming_catalog_for_tests("RAIKOU", "ENTEI"),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    })
    .expect("serialize complete empty manifest");
    manifest["payload"]["battle_reward_rules"] = serde_json::json!({
        "max_level": 100,
        "wild_exp_divisor": 7,
        "trainer_exp_numerator": 3,
        "trainer_exp_denominator": 2,
        "mom_money_increment": 2300,
        "mom_random_items": [{
            "trigger": 0,
            "cost": 600,
            "kind": "item",
            "target": "SUPER_POTION",
            "decoration_flag": null
        }],
        "mom_progression_items": [{
            "trigger": 900,
            "cost": 600,
            "kind": "item",
            "target": "SUPER_POTION",
            "decoration_flag": null
        }]
    });
    manifest["payload"]["battle_escape_rules"] = serde_json::json!({
        "player_speed_multiplier": 32,
        "enemy_speed_divisor": 4,
        "failed_attempt_bonus": 30,
        "rng_roll_values": 256
    });
    manifest["payload"]["battle_stat_multipliers"] =
        serde_json::to_value(test_battle_stat_multipliers())
            .expect("serialize battle stat multiplier fixture");
    manifest["payload"]["step_event_rules"] =
        serde_json::to_value(test_step_event_rules()).expect("serialize step event rules");
    manifest["payload"]["move_priorities"] =
        serde_json::to_value(test_move_priorities()).expect("serialize move priority fixture");
    manifest["payload"]["type_categories"] =
        serde_json::to_value(test_type_categories()).expect("serialize type categories fixture");
    manifest["payload"]["type_effectiveness"] = serde_json::to_value(test_type_effectiveness())
        .expect("serialize type effectiveness fixture");
    manifest["payload"]["weather_modifiers"] =
        serde_json::to_value(test_weather_modifiers()).expect("serialize weather fixture");
    manifest["payload"]["field_moves"] =
        serde_json::to_value(test_field_move_catalog()).expect("serialize field moves fixture");
    manifest["payload"]["buena_password_categories"] =
        serde_json::to_value(test_buena_password_categories())
            .expect("serialize Buena password fixture");
    manifest
}

#[test]
fn modpack_manifest_json_requires_explicit_top_level_metadata() {
    let mut missing_payload = explicit_empty_manifest_json();
    missing_payload
        .as_object_mut()
        .expect("manifest object")
        .remove("payload");
    let error = serde_json::from_value::<ModpackManifest>(missing_payload)
        .expect_err("missing payload must not default to an empty pack")
        .to_string();
    assert!(error.contains("missing field `payload`"), "{error}");

    let mut missing_author = explicit_empty_manifest_json();
    missing_author
        .get_mut("metadata")
        .expect("metadata")
        .as_object_mut()
        .expect("metadata object")
        .remove("author");
    let error = serde_json::from_value::<ModpackManifest>(missing_author)
        .expect_err("nullable author must be explicit")
        .to_string();
    assert!(error.contains("missing field `author`"), "{error}");
}

#[test]
fn modpack_manifest_json_requires_explicit_payload_categories() {
    let mut missing_audio = explicit_empty_manifest_json();
    missing_audio
        .get_mut("payload")
        .expect("payload")
        .as_object_mut()
        .expect("payload object")
        .remove("audio");
    let error = serde_json::from_value::<ModpackManifest>(missing_audio)
        .expect_err("missing payload category must not default to empty")
        .to_string();
    assert!(error.contains("missing field `audio`"), "{error}");
}

#[test]
fn modpack_manifest_json_rejects_unknown_fields() {
    let mut manifest = explicit_empty_manifest_json();
    manifest["metadata"]["displayName"] = Value::String("Coerced Name".to_string());
    let error = serde_json::from_value::<ModpackManifest>(manifest)
        .expect_err("unknown metadata fields must not be ignored")
        .to_string();
    assert!(error.contains("unknown field `displayName`"), "{error}");

    let mut manifest = explicit_empty_manifest_json();
    manifest["payload"]["wildPokemon"] = serde_json::json!([]);
    let error = serde_json::from_value::<ModpackManifest>(manifest)
        .expect_err("unknown payload fields must not be ignored")
        .to_string();
    assert!(error.contains("unknown field `wildPokemon`"), "{error}");

    let mut manifest = explicit_empty_manifest_json();
    manifest["fallback"] = Value::Bool(true);
    let error = serde_json::from_value::<ModpackManifest>(manifest)
        .expect_err("unknown manifest fields must not be ignored")
        .to_string();
    assert!(error.contains("unknown field `fallback`"), "{error}");
}

#[test]
fn content_pack_index_json_rejects_unknown_fields() {
    let index = serde_json::json!({
        "version": 1,
        "packs": [
            {
                "id": "bad-pack",
                "enabled": true,
                "priority": 0,
                "path": "content-packs/bad-pack",
                "compiled": null,
                "fallback": true,
                "files": ContentPackFiles::default()
            }
        ]
    });

    let error = serde_json::from_value::<ContentPackIndex>(index)
        .expect_err("unknown content pack fields must not be ignored")
        .to_string();
    assert!(error.contains("unknown field `fallback`"), "{error}");
}

#[test]
fn content_pack_audio_entries_require_explicit_metadata_json_without_path_inference() {
    let mut files = ContentPackFiles::default();
    files
        .audio
        .push("content-packs/test/music/MUSIC_ROUTE_29.pcm".to_string());
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: None,
            files,
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("audio file paths must not be inferred into metadata");

    assert!(
            format!("{error:#}").contains(
                "content pack test audio entry content-packs/test/music/MUSIC_ROUTE_29.pcm must point to explicit audio metadata JSON"
            ),
            "{error:#}"
        );
}

#[test]
fn content_pack_file_entries_must_not_repeat_within_category() {
    let mut files = ContentPackFiles::default();
    files
        .audio
        .push("content-packs/test/audio/MUSIC_ROUTE_29.json".to_string());
    files
        .audio
        .push("content-packs/test/audio/MUSIC_ROUTE_29.json".to_string());
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: None,
            files,
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("content pack file lists must be exact sets")
        .to_string();

    assert!(
            error.contains(
                "content pack test category audio includes duplicate file entry content-packs/test/audio/MUSIC_ROUTE_29.json"
            ),
            "{error}"
        );
}

#[test]
fn content_pack_data_entries_require_explicit_json_without_extension_inference() {
    let mut files = ContentPackFiles::default();
    files.pokemon.push("content-packs/test/pokemon".to_string());
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: None,
            files,
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("content pack data paths must be explicit JSON")
        .to_string();

    assert!(
            error.contains(
                "content pack test category pokemon entry content-packs/test/pokemon must point to explicit JSON data"
            ),
            "{error}"
        );
}

#[test]
fn content_pack_entries_reject_current_directory_path_aliases() {
    let mut files = ContentPackFiles::default();
    files
        .pokemon
        .push("content-packs/test/./pokemon.json".to_string());
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: None,
            files,
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("content pack paths must not accept aliases")
        .to_string();

    assert!(
        error.contains("must not include current-directory components"),
        "{error}"
    );
}

#[test]
fn content_pack_compiled_entries_require_explicit_crystalpack_without_json_fallback() {
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: Some("content-packs/test/compiled.compiled.json".to_string()),
            files: ContentPackFiles::default(),
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("compiled content paths must not use legacy JSON artifacts")
        .to_string();

    assert!(
            error.contains(
                "content pack test compiled entry content-packs/test/compiled.compiled.json must point to an explicit .crystalpack artifact"
            ),
            "{error}"
        );

    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: Some("content-packs/test/compiled.json".to_string()),
            files: ContentPackFiles::default(),
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("compiled content paths must not accept generic JSON metadata")
        .to_string();

    assert!(
            error.contains(
                "content pack test compiled entry content-packs/test/compiled.json must point to an explicit .crystalpack artifact"
            ),
            "{error}"
        );
}

#[test]
fn content_pack_compiled_entries_reject_path_aliases_after_suffix_check() {
    let cases = [
        (
            "assets/data/content-packs/test/core.crystalpack",
            "must not include the assets/data prefix",
        ),
        (
            "content-packs/test/../core.crystalpack",
            "must not traverse parent directories",
        ),
        (
            "content-packs/test/./core.crystalpack",
            "must not include current-directory components",
        ),
        (
            "content-packs/other/core.crystalpack",
            "must be under content-packs/test",
        ),
    ];

    for (compiled, expected) in cases {
        let index = ContentPackIndex {
            version: 1,
            packs: vec![ContentPack {
                id: "test".to_string(),
                enabled: true,
                priority: 0,
                path: "content-packs/test".to_string(),
                compiled: Some(compiled.to_string()),
                files: ContentPackFiles::default(),
            }],
        };

        let error = GameDataSet::default()
            .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
            .expect_err("compiled content paths must be canonical after suffix validation")
            .to_string();

        assert!(error.contains(expected), "{compiled}: {error}");
    }
}

#[test]
fn compiled_game_pack_application_requires_empty_runtime_dataset() {
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/test".to_string(),
            compiled: Some("content-packs/test/core.crystalpack".to_string()),
            files: ContentPackFiles::default(),
        }],
    };
    let mut data = GameDataSet {
        pokemon: [("STALE_MON".to_string(), species())].into_iter().collect(),
        ..GameDataSet::default()
    };

    let error = data
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("compiled packs must not layer on stale runtime data")
        .to_string();

    assert!(
        error.contains("compiled game pack 'test' must be applied to an empty runtime dataset"),
        "{error}"
    );
    assert!(data.pokemon.contains_key("STALE_MON"));
}

#[test]
fn apply_content_pack_index_rejects_noncanonical_pack_identity() {
    let index = ContentPackIndex {
        version: 1,
        packs: vec![ContentPack {
            id: "test".to_string(),
            enabled: true,
            priority: 0,
            path: "content-packs/other".to_string(),
            compiled: None,
            files: ContentPackFiles::default(),
        }],
    };

    let error = GameDataSet::default()
        .apply_content_pack_index(&AssetRoot::new(repository_root_for_tests()), &index)
        .expect_err("content pack application must validate index identity")
        .to_string();
    assert!(
        error.contains("must be exactly content-packs/test"),
        "{error}"
    );
}

#[test]
fn compiler_report_json_rejects_unknown_fields() {
    let edge_error = serde_json::from_value::<PlayabilityGraphEdge>(serde_json::json!({
        "from": "NewBarkTown",
        "to": "Route29",
        "kind": "walk",
        "fallback": true
    }))
    .expect_err("playability graph edges must not accept fallback metadata")
    .to_string();
    assert!(
        edge_error.contains("unknown field `fallback`"),
        "{edge_error}"
    );

    let diagnostic_error = serde_json::from_value::<VerificationError>(serde_json::json!({
        "severity": "error",
        "code": "unknown_map",
        "subject": "Route29",
        "message": "missing map",
        "normalizedSubject": "route29"
    }))
    .expect_err("verification diagnostics must not accept normalized aliases")
    .to_string();
    assert!(
        diagnostic_error.contains("unknown field `normalizedSubject`"),
        "{diagnostic_error}"
    );
}

#[test]
fn content_pack_paths_reject_legacy_prefix_absolute_and_traversal() {
    let asset_root = AssetRoot::new(repository_root_for_tests());

    let legacy = resolve_content_pack_data_path(
        &asset_root,
        "bad-pack",
        "assets/data/content-packs/bad/pokemon/a.json",
    )
    .expect_err("content pack paths must not accept assets/data-prefixed aliases")
    .to_string();
    assert!(
        legacy.contains("must not include the assets/data prefix"),
        "{legacy}"
    );

    let absolute = resolve_content_pack_data_path(
        &asset_root,
        "bad-pack",
        "/tmp/content-packs/bad/pokemon/a.json",
    )
    .expect_err("content pack paths must not be absolute")
    .to_string();
    assert!(
        absolute.contains("must be relative to assets/data"),
        "{absolute}"
    );

    let traversal =
        resolve_content_pack_data_path(&asset_root, "bad-pack", "content-packs/../bad.json")
            .expect_err("content pack paths must not traverse parent directories")
            .to_string();
    assert!(
        traversal.contains("must not traverse parent directories"),
        "{traversal}"
    );

    let cross_pack = resolve_content_pack_data_path(
        &asset_root,
        "good-pack",
        "content-packs/other-pack/pokemon/a.json",
    )
    .expect_err("content pack paths must remain under their declaring pack id")
    .to_string();
    assert!(
        cross_pack.contains("must be under content-packs/good-pack"),
        "{cross_pack}"
    );

    let shared = resolve_content_pack_data_path(&asset_root, "good-pack", "shared/a.json")
        .expect_err("content pack paths must not point at shared runtime data")
        .to_string();
    assert!(
        shared.contains("must be under content-packs/good-pack"),
        "{shared}"
    );

    let canonical =
        resolve_content_pack_data_path(&asset_root, "good-pack", "content-packs/good-pack/a.json")
            .expect("canonical content pack path");
    assert!(canonical.ends_with("apps/web/assets/data/content-packs/good-pack/a.json"));
}

#[test]
fn runtime_data_paths_reject_aliases_absolute_and_traversal() {
    let asset_root = AssetRoot::new(repository_root_for_tests());

    let legacy = asset_root
        .resolve_data_path("assets/data/content-packs/core-modular/music/MUSIC_ROUTE_29.pcm")
        .expect_err("runtime paths must not accept assets/data aliases")
        .to_string();
    assert!(
        legacy.contains("must not include the assets/data prefix"),
        "{legacy}"
    );

    let absolute = asset_root
        .resolve_data_path("/tmp/content-packs/core-modular/music/MUSIC_ROUTE_29.pcm")
        .expect_err("runtime paths must not be absolute")
        .to_string();
    assert!(
        absolute.contains("must be relative to assets/data"),
        "{absolute}"
    );

    let traversal = asset_root
        .resolve_data_path("content-packs/core-modular/../bad.pcm")
        .expect_err("runtime paths must not traverse")
        .to_string();
    assert!(
        traversal.contains("must not traverse parent directories"),
        "{traversal}"
    );

    let current_dir = asset_root
        .resolve_data_path("content-packs/core-modular/./music/MUSIC_ROUTE_29.pcm")
        .expect_err("runtime paths must not accept current-directory aliases")
        .to_string();
    assert!(
        current_dir.contains("must not include current-directory components"),
        "{current_dir}"
    );

    let canonical = asset_root
        .resolve_data_path("content-packs/core-modular/music/MUSIC_ROUTE_29.pcm")
        .expect("canonical runtime data path");
    assert!(
        canonical
            .ends_with("apps/web/assets/data/content-packs/core-modular/music/MUSIC_ROUTE_29.pcm")
    );
}

#[test]
fn game_data_set_json_requires_explicit_sections() {
    let asset_root = AssetRoot::new(repository_root_for_tests());
    let data = serde_json::to_value(
        asset_root
            .load_base_game_data()
            .expect("load base game data"),
    )
    .expect("serialize complete empty game data");

    for field in [
        "audio",
        "growth_rates",
        "marts",
        "currency_constants",
        "battle_reward_rules",
        "battle_escape_rules",
        "step_event_rules",
        "fruit_trees",
        "field_moves",
        "field_box_items",
        "fly_destinations",
        "runtime_spawn_points",
        "runtime_map_metadata",
        "buena_password_categories",
        "roaming_pokemon",
        "buena_prizes",
        "kurt_apricorn_recipes",
        "shuckie_gift",
        "dratini_move_sets",
        "bug_contest_config",
        "battle_tower_rules",
        "oak_ratings",
        "odd_egg_definitions",
        "magikarp_lengths",
        "happiness_data",
        "encounter_slot_tables",
        "encounter_music_modifiers",
        "battle_stat_multipliers",
        "capture_wobble_probabilities",
        "move_priorities",
        "type_categories",
        "type_effectiveness",
        "weather_modifiers",
        "pc_strings",
        "menu_icons",
        "pokedex_entries",
        "pokemon_frontpic_anim",
        "initialize_events",
        "story_event_script_constants",
        "asm_text",
        "move_names",
        "battle_animations",
        "battle_animation_table",
        "battle_anim_bundle",
        "sprite_anim_bundle",
        "sprite_palette_defaults",
        "pokegear_town_map_palette_map",
    ] {
        let mut missing = data.clone();
        missing
            .as_object_mut()
            .expect("game data object")
            .remove(field)
            .unwrap_or_else(|| panic!("fixture must include {field}"));

        let error = serde_json::from_value::<GameDataSet>(missing)
            .expect_err("missing game data sections must not default to empty")
            .to_string();
        let expected = format!("missing field `{field}`");
        assert!(error.contains(&expected), "{field} produced {error}");
    }
}

#[test]
fn pokedex_entry_json_requires_explicit_pages() {
    let mut entry = serde_json::json!({
        "species":"BULBASAUR",
        "classification":"SEED",
        "heightDigits":204,
        "weightDigits":150,
        "pages":["A strange seed was planted on its back."]
    });
    entry
        .as_object_mut()
        .expect("pokedex entry object")
        .remove("pages");

    let error = serde_json::from_value::<RuntimePokedexEntry>(entry)
        .expect_err("missing pages must not default to an empty entry")
        .to_string();
    assert!(error.contains("missing field `pages`"), "{error}");
}

#[test]
fn raw_script_command_json_requires_explicit_args() {
    let error = serde_json::from_value::<Vec<ScriptCommand>>(serde_json::json!([{
        "command":"end"
    }]))
    .expect_err("raw script commands must export args explicitly")
    .to_string();

    assert!(error.contains("missing field `args`"), "{error}");
}

#[test]
fn modpack_item_json_requires_explicit_effect_without_defaulting_to_none() {
    let mut manifest = explicit_empty_manifest_json();
    manifest["metadata"]["id"] = Value::String("bad-items".to_string());
    manifest["metadata"]["name"] = Value::String("Bad Items".to_string());
    manifest["payload"]["items"] = serde_json::json!({
        "FLASH_STEP_CHARM": {
            "name":"Flash Step Charm",
            "description":"A malformed modded item.",
            "price":100,
            "held_effect":"HELD_NONE",
            "parameter":0,
            "property":"",
            "pocket":"ITEM",
            "field_menu":"",
            "battle_menu":"",
            "script_name":"FLASH_STEP_CHARM",
            "tmhm_index":null
        }
    });

    let error = serde_json::from_value::<ModpackManifest>(manifest)
        .expect_err("missing item effect must not default to NONE")
        .to_string();

    assert!(error.contains("missing field `effect`"), "{error}");
}

#[test]
fn missing_core_pack_index_is_an_error_not_a_fallback() {
    let root = std::env::temp_dir().join(format!(
        "crystal-assets-missing-pack-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("apps/web/assets/data/content-packs"))
        .expect("create temp asset root");

    let error = AssetRoot::new(&root)
        .load_base_game_data()
        .expect_err("missing core pack index should fail");

    assert!(error.to_string().contains("content-packs/index.json"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn creates_pokemon_from_loaded_existing_json() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let pokemon = data
        .create_pokemon("BULBASAUR", 10, Dv::from_non_hp(10, 10, 10, 10))
        .expect("create bulbasaur");

    assert_eq!(pokemon.nickname, "BULBASAUR");
    assert_eq!(pokemon.level, 10);
    assert_eq!(
        pokemon
            .moves
            .iter()
            .map(|learned| learned.name.as_str())
            .collect::<Vec<_>>(),
        vec!["TACKLE", "GROWL", "LEECH_SEED", "VINE_WHIP"]
    );
    assert_eq!(pokemon.moves[0].current_pp, data.moves["TACKLE"].pp);
    assert_eq!(pokemon.experience, 560);
    assert_eq!(pokemon.happiness, 70);
}

#[test]
fn compiled_pack_executes_every_map_callback_on_map_entry() {
    let root = repository_root_for_tests();
    let pack =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack");
    let data = pack.data;
    let callback_maps: Vec<String> = data
        .maps
        .keys()
        .filter(|map_name| {
            data.map_module(map_name)
                .map(|module| {
                    module
                        .map_script_section_commands
                        .iter()
                        .any(|command| command.command == "callback")
                })
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    assert_eq!(callback_maps.len(), 89);

    for map_name in callback_maps {
        let map = data
            .overworld_map(&map_name)
            .unwrap_or_else(|error| panic!("assemble callback map {map_name}: {error:#}"));
        let (width, height) = map
            .checked_tile_bounds()
            .unwrap_or_else(|| panic!("callback map {map_name} has invalid bounds"));
        let mut session = None;
        'candidate: for y in 0..height {
            for x in 0..width {
                if let Ok(candidate) =
                    data.overworld_session(&map_name, TilePosition::new(x as i16, y as i16), 0)
                {
                    session = Some(candidate);
                    break 'candidate;
                }
            }
        }
        let mut session =
            session.unwrap_or_else(|| panic!("callback map {map_name} has no reachable test tile"));
        let mut state = GameState::default();
        apply_initialize_events(&mut state, &data.initialize_events)
            .unwrap_or_else(|error| panic!("initialize callback state: {error}"));
        state.overworld = crystal_core::state::OverworldMemory::from_snapshot(&session.snapshot());
        state
            .script_runtime
            .variables
            .insert("_player_gender".to_string(), "MALE".to_string());
        if let Some(module) = data.map_module(&map_name).ok() {
            for callback in module
                .map_script_section_commands
                .iter()
                .filter(|command| command.command == "callback")
            {
                if let Some(callback_name) = callback.args.get(1) {
                    if let Some(body) = module.scripts.get(callback_name).and_then(Value::as_array)
                    {
                        for entry in body {
                            if entry.get("command").and_then(Value::as_str) == Some("readmem") {
                                if let Some(memory) = entry
                                    .get("args")
                                    .and_then(Value::as_array)
                                    .and_then(|args| args.first())
                                    .and_then(Value::as_str)
                                {
                                    state
                                        .script_runtime
                                        .memory
                                        .insert(memory.to_string(), "0".to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        data.apply_map_setup_callbacks(&mut state, &mut session, &map_name, "MAPSETUP_WARP")
            .unwrap_or_else(|error| panic!("execute map callbacks on {map_name}: {error:#}"));
    }
}

#[test]
fn map_setup_callbacks_reject_unsupported_opcodes_instead_of_skipping_them() {
    let map_name = "StrictCallbackMap";
    let callback_name = "StrictCallbackScript";
    let mut module = test_map_module(map_name, "STRICT_CALLBACK_MAP", None);
    module
        .map_script_section_commands
        .push(MapScriptSectionCommand {
            command: "callback".to_string(),
            args: vec!["MAPCALLBACK_NEWMAP".to_string(), callback_name.to_string()],
            command_index: 0,
        });
    module.scripts.insert(
        callback_name.to_string(),
        serde_json::json!([{"command": "unsupportedcallbackop", "args": []}]),
    );
    let data = GameDataSet {
        maps: map_payload(vec![module]),
        ..GameDataSet::default()
    };
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: map_name.to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        MapEvents::default(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let mut state = GameState::default();

    let error = data
        .apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_WARP")
        .expect_err("unsupported callback opcode must fail");

    assert!(
        error.to_string().contains("unsupportedcallbackop"),
        "{error:#}"
    );
}

#[test]
fn endcallback_preserves_a_suspended_outer_script_call_frame() {
    let map_name = "CallbackStackMap";
    let callback_name = "CallbackStackScript";
    let mut module = test_map_module(map_name, "CALLBACK_STACK_MAP", None);
    module
        .map_script_section_commands
        .push(MapScriptSectionCommand {
            command: "callback".to_string(),
            args: vec!["MAPCALLBACK_NEWMAP".to_string(), callback_name.to_string()],
            command_index: 0,
        });
    module.scripts.insert(
        callback_name.to_string(),
        serde_json::json!([{"command": "endcallback", "args": []}]),
    );
    module.script_control_commands.push(ScriptControlCommand {
        command: "endcallback".to_string(),
        compare_value: None,
        target_label: None,
        resolved_target_script: None,
        source_script: callback_name.to_string(),
        command_index: 0,
    });
    let data = GameDataSet {
        maps: map_payload(vec![module]),
        ..GameDataSet::default()
    };
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: map_name.to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        MapEvents::default(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let suspended = crystal_core::state::ScriptReturnFrame {
        origin_map_name: map_name.to_string(),
        source_script: "SuspendedOuterScript".to_string(),
        next_command_index: 7,
    };
    let suspended_next = crystal_core::state::ScriptLocation {
        origin_map_name: map_name.to_string(),
        script: "SuspendedNextScript".to_string(),
    };
    let suspended_end = crystal_core::state::ScriptEndState {
        callback: false,
        just_battled_guard: false,
        source_script: "PriorEndedScript".to_string(),
        command_index: 3,
    };
    let mut state = GameState::default();
    state.script_runtime.call_stack.push(suspended.clone());
    state.script_runtime.next_script = Some(suspended_next.clone());
    state.script_runtime.script_ended = Some(suspended_end.clone());

    data.apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_WARP")
        .expect("callback must return without consuming its suspended caller");

    assert_eq!(state.script_runtime.call_stack, vec![suspended]);
    assert_eq!(state.script_runtime.next_script, Some(suspended_next));
    assert_eq!(state.script_runtime.script_ended, Some(suspended_end));
}

#[test]
fn compiled_pack_executes_tile_callback_block_changes_on_map_entry() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "RuinsOfAlphKabutoChamber";
    let map = data
        .overworld_map(map_name)
        .expect("assemble Kabuto chamber map");
    let mut session = data
        .overworld_session(map_name, TilePosition::new(4, 4), 0)
        .expect("create Kabuto chamber session");
    let mut state = GameState::default();
    state
        .flags
        .set_event_flag("EVENT_SOLVED_KABUTO_PUZZLE", false)
        .expect("set puzzle event");
    let before = session
        .map
        .metatile_at(1, 1)
        .expect("read Kabuto chamber floor block");
    data.apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_WARP")
        .expect("execute tile callback");
    assert_ne!(Some(before), session.map.metatile_at(1, 1));
    assert_eq!(session.map.metatile_at(1, 1), Some(0x01));
    assert_eq!(
        state
            .map_block_overrides
            .get(map_name)
            .and_then(|overrides| overrides.get(&(1, 1))),
        Some(&0x01)
    );
    assert_eq!(map.metatile_at(1, 1), Some(before));
}

#[test]
fn reload_map_setup_does_not_run_the_maps_newmap_callback() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "GoldenrodUndergroundWarehouse";
    let map = data
        .overworld_map(map_name)
        .expect("assemble warehouse map");
    let (width, height) = map.checked_tile_bounds().expect("warehouse map bounds");
    let mut session = (0..height)
        .flat_map(|y| (0..width).map(move |x| TilePosition::new(x as i16, y as i16)))
        .find_map(|tile| data.overworld_session(map_name, tile, 0).ok())
        .expect("find a walkable warehouse tile");
    let mut state = GameState::default();
    state.script_runtime.current_music = Some("MUSIC_GOLDENROD_CITY".to_string());
    state.script_runtime.map_music_restart_disabled = true;
    state
        .flags
        .set_event_flag("EVENT_SWITCH_1", true)
        .expect("set warehouse switch");

    data.apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_RELOADMAP")
        .expect("execute reload setup callbacks");

    assert!(
        state
            .flags
            .is_event_flag_set("EVENT_SWITCH_1")
            .expect("read warehouse switch")
    );
    assert_eq!(state.script_runtime.current_music, None);
    assert!(!state.script_runtime.map_music_restart_disabled);

    data.apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_WARP")
        .expect("execute warp setup callbacks");

    assert!(
        !state
            .flags
            .is_event_flag_set("EVENT_SWITCH_1")
            .expect("read reset warehouse switch")
    );
}

#[test]
fn runtime_map_setup_callback_mutation_runs_the_exact_current_map_plan() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "GoldenrodUndergroundWarehouse";
    let map = data
        .overworld_map(map_name)
        .expect("assemble warehouse map");
    let (width, height) = map.checked_tile_bounds().expect("warehouse map bounds");
    let mut session = (0..height)
        .flat_map(|y| (0..width).map(move |x| TilePosition::new(x as i16, y as i16)))
        .find_map(|tile| data.overworld_session(map_name, tile, 0).ok())
        .expect("find a walkable warehouse tile");
    let mut state = GameState::default();
    state
        .flags
        .set_event_flag("EVENT_SWITCH_1", true)
        .expect("set warehouse switch");
    let audio = BTreeSet::new();

    let reload = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ApplyMapSetupCallbacks {
                map_setup: "MAPSETUP_RELOADMAP".to_string(),
            },
            &audio,
            &audio,
            &audio,
        )
        .expect("apply reload callbacks");
    assert!(matches!(
        reload.result,
        RuntimeMutationResult::MapSetupCallbacksApplied(ref setup)
            if setup == "MAPSETUP_RELOADMAP"
    ));
    assert!(
        state
            .flags
            .is_event_flag_set("EVENT_SWITCH_1")
            .expect("read warehouse switch after reload")
    );

    data.apply_runtime_mutation_command(
        &mut state,
        &mut session,
        RuntimeMutationCommand::ApplyMapSetupCallbacks {
            map_setup: "MAPSETUP_WARP".to_string(),
        },
        &audio,
        &audio,
        &audio,
    )
    .expect("apply warp callbacks");
    assert!(
        !state
            .flags
            .is_event_flag_set("EVENT_SWITCH_1")
            .expect("read reset warehouse switch")
    );
}

#[test]
fn compiled_players_room_callback_installs_default_bed_and_town_map_blocks() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "PlayersHouse2F";
    let mut session = data
        .overworld_session(map_name, TilePosition::new(3, 3), 0)
        .expect("create upstairs bedroom session");
    assert_eq!(session.map.metatile_at(0, 2), Some(0x05));
    assert_eq!(session.map.metatile_at(3, 0), Some(0x02));
    let mut state = GameState::default();
    apply_initialize_events(&mut state, &data.initialize_events)
        .expect("initialize the default bedroom decorations");
    data.commit_overworld_snapshot(&mut state, &session, SpawnMemoryUpdate::Preserve);

    data.apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_WARP")
        .expect("execute player-room callbacks");

    assert_eq!(session.map.metatile_at(0, 2), Some(0x1b));
    assert_eq!(session.map.metatile_at(3, 0), Some(0x1f));
    assert_eq!(
        state.map_block_overrides.get(map_name),
        Some(&BTreeMap::from([((0, 2), 0x1b), ((3, 0), 0x1f)]))
    );
}

#[test]
fn player_decoration_actions_follow_owned_flags_slots_and_ornament_sides() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let mut state = GameState::default();
    for flag in [
        "EVENT_DECO_BED_1",
        "EVENT_DECO_BED_2",
        "EVENT_DECO_PIKACHU_DOLL",
        "EVENT_DECO_CLEFAIRY_DOLL",
    ] {
        state
            .flags
            .set_event_flag(flag, true)
            .expect("own decoration");
    }

    assert_eq!(
        data.owned_decoration_categories(&state)
            .expect("owned categories"),
        vec![DecorationCategory::Bed, DecorationCategory::Ornament]
    );
    assert_eq!(
        data.owned_decorations(&state, DecorationCategory::Bed)
            .expect("owned beds")
            .into_iter()
            .map(|decoration| decoration.id.as_str())
            .collect::<Vec<_>>(),
        vec!["DECO_FEATHERY_BED", "DECO_PINK_BED"]
    );

    assert_eq!(
        data.set_up_decoration(&mut state, "DECO_FEATHERY_BED", None)
            .expect("set up first bed"),
        DecorationActionOutcome::SetUp {
            decoration: "DECO_FEATHERY_BED".to_string()
        }
    );
    assert_eq!(
        data.set_up_decoration(&mut state, "DECO_PINK_BED", None)
            .expect("replace bed"),
        DecorationActionOutcome::Replaced {
            decoration: "DECO_PINK_BED".to_string(),
            previous: "DECO_FEATHERY_BED".to_string()
        }
    );
    assert_eq!(
        data.set_up_decoration(&mut state, "DECO_PINK_BED", None)
            .expect("reject already-set bed"),
        DecorationActionOutcome::AlreadySetUp {
            decoration: "DECO_PINK_BED".to_string()
        }
    );
    assert_eq!(
        data.put_away_decoration(&mut state, DecorationCategory::Bed, None)
            .expect("put away bed"),
        DecorationActionOutcome::PutAway {
            decoration: "DECO_PINK_BED".to_string()
        }
    );
    assert_eq!(
        data.put_away_decoration(&mut state, DecorationCategory::Bed, None)
            .expect("empty bed slot"),
        DecorationActionOutcome::NothingToPutAway
    );

    data.set_up_decoration(&mut state, "DECO_PIKACHU_DOLL", Some(DecorationSide::Left))
        .expect("set left ornament");
    data.set_up_decoration(
        &mut state,
        "DECO_CLEFAIRY_DOLL",
        Some(DecorationSide::Right),
    )
    .expect("set right ornament");
    assert_eq!(
        data.set_up_decoration(&mut state, "DECO_PIKACHU_DOLL", Some(DecorationSide::Right),)
            .expect("move ornament to right"),
        DecorationActionOutcome::Replaced {
            decoration: "DECO_PIKACHU_DOLL".to_string(),
            previous: "DECO_CLEFAIRY_DOLL".to_string()
        }
    );
    assert_eq!(
        state.script_runtime.memory.get("wDecoLeftOrnament"),
        Some(&"0".to_string())
    );
    assert_eq!(
        state.script_runtime.memory.get("wDecoRightOrnament"),
        Some(&"DECO_PIKACHU_DOLL".to_string())
    );

    let before = state.clone();
    let error = data
        .set_up_decoration(&mut state, "DECO_N64", None)
        .expect_err("unowned console must reject");
    assert!(
        error
            .to_string()
            .contains("decoration DECO_N64 is not owned")
    );
    assert_eq!(state, before);
}

#[test]
fn compiled_warpcheck_resolves_the_live_warp_under_the_player() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "RuinsOfAlphKabutoChamber";
    let module = data.map_module(map_name).expect("assemble Kabuto chamber");
    let command = module
        .script_map_commands
        .iter()
        .find(|command| {
            command.command == "warpcheck"
                && command.source_script == ".PuzzleComplete@RuinsOfAlphKabutoChamberPuzzle"
        })
        .expect("Kabuto puzzle warpcheck");
    let mut sessionless_state = GameState::default();
    let error = data
        .apply_script_map_command(
            &mut sessionless_state,
            map_name,
            map_name,
            &command.source_script,
            command.command_index,
        )
        .expect_err("warpcheck requires live collision and player state");
    assert!(
        format!("{error:#}").contains("requires a live overworld session"),
        "{error:#}"
    );
    assert_eq!(sessionless_state, GameState::default());
    let session = data
        .overworld_session(map_name, TilePosition::new(3, 9), 0)
        .expect("create session on live chamber exit warp");
    let trigger = session
        .check_warp_checked()
        .expect("check chamber warp")
        .expect("live chamber exit warp");
    let expected = data
        .resolve_warp_transition(&trigger)
        .expect("resolve chamber exit warp");
    let mut state = GameState::default();
    data.commit_overworld_snapshot(&mut state, &session, SpawnMemoryUpdate::Preserve);

    let action = data
        .apply_script_map_command_in_session(
            &mut state,
            &session,
            map_name,
            &command.source_script,
            command.command_index,
        )
        .expect("execute compiled warpcheck");

    assert!(matches!(action, ScriptMapAction::WarpCheck { .. }));
    assert_eq!(
        state.script_runtime.pending_script_warp,
        Some(ScriptWarpRequest {
            target_map: expected.destination.map_name,
            tile: expected.destination.tile,
            facing: None,
            source_script: command.source_script.clone(),
            command_index: command.command_index,
        })
    );
    let pending = state
        .script_runtime
        .pending_script_warp
        .as_ref()
        .expect("pending warpcheck transition");
    data.validate_saved_warpcheck_pending_warp_reference(
        &state,
        "script_runtime.pending_script_warp.source_script",
        pending,
    )
    .expect("validate source-exact saved warpcheck transition");

    let mut forged = pending.clone();
    forged.tile.x += 2;
    let error = data
        .validate_saved_warpcheck_pending_warp_reference(
            &state,
            "script_runtime.pending_script_warp.source_script",
            &forged,
        )
        .expect_err("saved warpcheck destination must match the live warp");
    assert!(
        format!("{error:#}").contains("does not match live warp destination"),
        "{error:#}"
    );
}

#[test]
fn compiled_warpcheck_does_nothing_without_a_live_warp_under_the_player() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "RuinsOfAlphKabutoChamber";
    let module = data.map_module(map_name).expect("assemble Kabuto chamber");
    let command = module
        .script_map_commands
        .iter()
        .find(|command| {
            command.command == "warpcheck"
                && command.source_script == ".PuzzleComplete@RuinsOfAlphKabutoChamberPuzzle"
        })
        .expect("Kabuto puzzle warpcheck");
    let session = data
        .overworld_session(map_name, TilePosition::new(3, 8), 0)
        .expect("create session beside chamber exit warp");
    assert_eq!(
        session.check_warp_checked().expect("check chamber warp"),
        None
    );
    let mut state = GameState::default();
    data.commit_overworld_snapshot(&mut state, &session, SpawnMemoryUpdate::Preserve);

    let action = data
        .apply_script_map_command_in_session(
            &mut state,
            &session,
            map_name,
            &command.source_script,
            command.command_index,
        )
        .expect("execute compiled warpcheck");

    assert!(matches!(action, ScriptMapAction::WarpCheck { .. }));
    assert_eq!(state.script_runtime.pending_script_warp, None);
}

#[test]
fn compiled_warpcheck_is_atomic_when_the_live_warp_cannot_resolve() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "RuinsOfAlphKabutoChamber";
    let module = data.map_module(map_name).expect("assemble Kabuto chamber");
    let command = module
        .script_map_commands
        .iter()
        .find(|command| {
            command.command == "warpcheck"
                && command.source_script == ".PuzzleComplete@RuinsOfAlphKabutoChamberPuzzle"
        })
        .expect("Kabuto puzzle warpcheck");
    let mut session = data
        .overworld_session(map_name, TilePosition::new(3, 9), 0)
        .expect("create session on live chamber exit warp");
    let live_warp = session
        .map_events
        .warps
        .iter_mut()
        .find(|warp| warp.x == 3 && warp.y == 9)
        .expect("chamber exit warp");
    live_warp.target_map = "MISSING_MAP".to_string();
    live_warp.target_map_constant = "MISSING_MAP".to_string();
    let mut state = GameState::default();
    data.commit_overworld_snapshot(&mut state, &session, SpawnMemoryUpdate::Preserve);
    let before = state.clone();

    let error = data
        .apply_script_map_command_in_session(
            &mut state,
            &session,
            map_name,
            &command.source_script,
            command.command_index,
        )
        .expect_err("unresolvable live warp must fail");

    assert!(format!("{error:#}").contains("unknown target map constant 'MISSING_MAP'"));
    assert_eq!(state, before);
}

#[test]
fn reloadmapafterbattle_executes_exact_whiteout_mom_and_bill_branches() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "Route30";
    let command = data
        .map_module(map_name)
        .expect("assemble Route30")
        .script_map_commands
        .iter()
        .find(|command| command.command == "reloadmapafterbattle")
        .expect("Route30 trainer reloadmapafterbattle")
        .clone();
    let session = data
        .overworld_session(map_name, TilePosition::new(10, 10), 0)
        .expect("create Route30 session");

    let mut lost = GameState::default();
    lost.battle_result = 1;
    lost.script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "129".to_string());
    let mut no_divider = ReplayDivider::new([]);
    let action = data
        .apply_script_map_command_with_divider_in_session(
            &mut lost,
            &session,
            map_name,
            &command.source_script,
            command.command_index,
            &mut no_divider,
        )
        .expect("execute losing reloadmapafterbattle");
    assert!(matches!(action, ScriptMapAction::BattleWhiteout { .. }));
    assert_eq!(lost.script_runtime.pending_map_load, None);
    assert_eq!(
        lost.script_runtime
            .memory
            .get("wBattleScriptFlags")
            .map(String::as_str),
        Some("0")
    );

    let mut ordinary_wild = GameState::default();
    ordinary_wild
        .script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "0".to_string());
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut ordinary_wild,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute ordinary wild reloadmapafterbattle");
    assert!(ordinary_wild.script_runtime.deferred_scripts.is_empty());
    assert!(ordinary_wild.script_runtime.map_reentry_script.is_none());
    assert!(ordinary_wild.pending_mom_purchase.is_none());
    assert_eq!(
        ordinary_wild
            .script_runtime
            .pending_map_load
            .as_ref()
            .map(|load| load.command.as_str()),
        Some("reloadmapafterbattle")
    );

    let mut wild_box_full = GameState::default();
    wild_box_full.battle_result = 0x80;
    wild_box_full
        .script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "0".to_string());
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut wild_box_full,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute box-full wild reloadmapafterbattle");
    assert_eq!(
        wild_box_full
            .script_runtime
            .map_reentry_script
            .as_ref()
            .map(|script| script.script.as_str()),
        Some("Script_SpecialBillCall")
    );
    assert!(wild_box_full.script_runtime.deferred_scripts.is_empty());
    assert_eq!(
        wild_box_full
            .script_runtime
            .pending_map_load
            .as_ref()
            .map(|load| load.command.as_str()),
        Some("reloadmapafterbattle")
    );

    let existing_reentry = ScriptLocation {
        origin_map_name: map_name.to_string(),
        script: "Route30YoungsterJoeyScript".to_string(),
    };
    let mut occupied_reentry = GameState::default();
    occupied_reentry.battle_result = 0x80;
    occupied_reentry.script_runtime.map_reentry_script = Some(existing_reentry.clone());
    occupied_reentry
        .script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "0".to_string());
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut occupied_reentry,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute box-full return with occupied map reentry queue");
    assert_eq!(
        occupied_reentry.script_runtime.map_reentry_script,
        Some(existing_reentry),
        "LoadMemScript preserves the first queued map reentry pointer"
    );

    let mut trainer = GameState::default();
    trainer.moms_money = 900;
    trainer
        .script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "129".to_string());
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut trainer,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute trainer Mom reloadmapafterbattle");
    assert_eq!(trainer.bag.pc_items.get("SUPER_POTION"), Some(&1));
    assert_eq!(
        trainer.moms_money, 900,
        "deduction waits for the memory script"
    );
    assert_eq!(
        trainer
            .pending_mom_purchase
            .as_ref()
            .map(|purchase| purchase.cost),
        Some(600)
    );
    assert_eq!(
        trainer
            .script_runtime
            .map_reentry_script
            .as_ref()
            .map(|script| script.script.as_str()),
        Some(".ItemScript@Mom_GetScriptPointer")
    );
    data.validate_saved_mom_purchase_references(&trainer)
        .expect("validate source-exact pending Mom transaction");
    let mut forged_purchase = trainer.clone();
    forged_purchase
        .pending_mom_purchase
        .as_mut()
        .expect("pending Mom transaction")
        .cost = 601;
    let error = data
        .validate_saved_mom_purchase_references(&forged_purchase)
        .expect_err("forged Mom transaction must not load");
    assert!(
        format!("{error:#}").contains("does not match compiled source row"),
        "{error:#}"
    );
    let mut reload_session = session.clone();
    let empty_audio = BTreeSet::new();
    data.apply_runtime_mutation_command(
        &mut trainer,
        &mut reload_session,
        RuntimeMutationCommand::TakePendingScriptRequest(RuntimePendingScriptRequestCommand {
            kind: RuntimePendingScriptRequestKind::MapLoad,
        }),
        &empty_audio,
        &empty_audio,
        &empty_audio,
    )
    .expect("run deferred Mom purchase memory script at map reload");
    assert_eq!(trainer.moms_money, 900);
    assert!(trainer.pending_mom_purchase.is_some());
    assert!(trainer.script_runtime.next_script.is_none());
    let reentry = data
        .apply_runtime_mutation_command(
            &mut trainer,
            &mut reload_session,
            RuntimeMutationCommand::DrainScriptRuntimeQueue(
                RuntimeScriptRuntimeQueueDrainCommand {
                    queue: RuntimeScriptRuntimeQueue::MapReentryScript,
                },
            ),
            &empty_audio,
            &empty_audio,
            &empty_audio,
        )
        .expect("run Mom purchase map reentry script at player-event boundary");
    assert!(matches!(
        reentry.result,
        RuntimeMutationResult::ScriptRuntimeQueueDrained(
            RuntimeScriptRuntimeQueueDrainResult::MapReentryScript(_)
        )
    ));
    assert_eq!(trainer.moms_money, 300);
    assert_eq!(trainer.mom_item_index, 1);
    assert_eq!(trainer.pending_mom_purchase, None);
    assert!(trainer.script_runtime.map_reentry_script.is_none());

    let mut full_pc = GameState::default();
    full_pc.moms_money = 900;
    full_pc
        .script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "129".to_string());
    for item in data
        .items
        .values()
        .take(crystal_core::models::PC_ITEM_CAPACITY)
    {
        assert!(full_pc.bag.add_pc_item(item, 1).expect("fill PC item slot"));
    }
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut full_pc,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute trainer return with full PC");
    assert_eq!(full_pc.pending_mom_purchase, None);
    assert_eq!(full_pc.moms_money, 900);

    let mut no_purchase = GameState::default();
    no_purchase.mom_item_index = 10;
    no_purchase.mom_item_trigger_balance = 2_300;
    no_purchase.moms_money = 1;
    no_purchase
        .script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "129".to_string());
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut no_purchase,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute trainer return below Mom trigger");
    assert_eq!(no_purchase.pending_mom_purchase, None);
    assert!(no_purchase.bag.pc_items.is_empty());

    let mut doll = GameState::default();
    doll.mom_item_index = 3;
    doll.moms_money = 10_000;
    doll.script_runtime
        .memory
        .insert("wBattleScriptFlags".to_string(), "129".to_string());
    let mut no_divider = ReplayDivider::new([]);
    data.apply_script_map_command_with_divider_in_session(
        &mut doll,
        &session,
        map_name,
        &command.source_script,
        command.command_index,
        &mut no_divider,
    )
    .expect("execute trainer Mom doll purchase");
    assert!(
        doll.flags
            .is_event_flag_set("EVENT_DECO_CHARMANDER_DOLL")
            .expect("read Mom doll flag")
    );
    assert_eq!(
        doll.pending_mom_purchase
            .as_ref()
            .and_then(|purchase| purchase.decoration_flag.as_deref()),
        Some("EVENT_DECO_CHARMANDER_DOLL")
    );
}

#[test]
fn compiled_pack_installs_stone_table_callback_on_map_entry() {
    let root = repository_root_for_tests();
    let data =
        read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
            .expect("load compiled core pack")
            .data;
    let map_name = "BlackthornGym2F";
    let map = data
        .overworld_map(map_name)
        .expect("assemble Blackthorn Gym second floor");
    let (width, height) = map.checked_tile_bounds().expect("map bounds");
    let mut session = None;
    'candidate: for y in 0..height {
        for x in 0..width {
            if let Ok(candidate) =
                data.overworld_session(map_name, TilePosition::new(x as i16, y as i16), 0)
            {
                session = Some(candidate);
                break 'candidate;
            }
        }
    }
    let mut session = session.expect("reachable Blackthorn Gym tile");
    let mut state = GameState::default();
    data.apply_map_setup_callbacks(&mut state, &mut session, map_name, "MAPSETUP_WARP")
        .expect("execute command-queue callback");
    assert!(state.script_runtime.command_queue.is_empty());
    assert_eq!(state.script_runtime.stone_table_entries.len(), 3);
    assert_eq!(state.script_runtime.stone_table_entries[0].warp, 5);
    assert_eq!(
        state.script_runtime.stone_table_entries[0].object_event,
        "BLACKTHORNGYM2F_BOULDER1"
    );
}

#[test]
fn day_care_egg_moves_follow_donor_order_and_all_three_source_eligibility_paths() {
    let data = AssetRoot::new(repository_root_for_tests())
        .load_base_game_data()
        .expect("load base game data");
    let species = data.pokemon.get("CHIKORITA").expect("Chikorita data");
    let learned = |move_id: &str| {
        let move_data = data.moves.get(move_id).expect("move data");
        LearnedMove {
            name: move_id.to_string(),
            current_pp: move_data.pp,
            pp_ups: 0,
        }
    };
    let donor_moves = vec![
        learned("RAZOR_LEAF"),
        learned("HEADBUTT"),
        learned("VINE_WHIP"),
        learned("SPLASH"),
    ];
    let mut egg = create_pokemon_from_known_dvs(
        species,
        5,
        Dv::default(),
        &data.learnsets,
        &data.moves,
        &data.growth_rates,
    )
    .expect("build pre-normalized egg");
    egg.is_egg = true;
    egg.moves = donor_moves.clone();

    let mut state = GameState::default();
    let mut father = egg.clone();
    father.is_egg = false;
    father.moves = donor_moves;
    let mut mother = father.clone();
    mother.moves = vec![learned("RAZOR_LEAF")];
    state.day_care.man.pokemon = Some(mother);
    state.day_care.lady.pokemon = Some(father);
    state.day_care.egg = Some(egg);

    data.normalize_day_care_egg_species(&mut state)
        .expect("normalize exact inherited moves");

    assert_eq!(
        state
            .day_care
            .egg
            .as_ref()
            .unwrap()
            .moves
            .iter()
            .map(|learned| learned.name.as_str())
            .collect::<Vec<_>>(),
        vec!["GROWL", "RAZOR_LEAF", "HEADBUTT", "VINE_WHIP"]
    );
}

fn repository_root_for_tests() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(3)
        .expect("workspace is nested under rust/crates/crystal-assets")
        .to_path_buf()
}
