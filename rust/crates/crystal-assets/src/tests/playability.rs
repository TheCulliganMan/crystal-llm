    #[test]
    fn verifier_allows_objects_on_special_collision_runtime_tiles() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 1;
        module.attributes.height = 1;
        module.blocks = vec![1];
        module.objects = vec![test_object("START_NPC", "-1", 0, 0)];
        let mut tileset = test_tileset_definition();
        tileset.collision.insert(
            "1".to_string(),
            vec![
                "WALL".to_string(),
                "WALL".to_string(),
                "WALL".to_string(),
                "WALL".to_string(),
            ],
        );
        let data = GameDataSet {
            tilesets: [("johto".to_string(), tileset)].into_iter().collect(),
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "unwalkable_object_tile")
        );
    }

    #[test]
    fn verifier_rejects_command_records_with_missing_source_scripts() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "KnownScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.script_movements = vec![
            ScriptMovement {
                label: "KnownMovement".to_string(),
                source_script: Some("KnownScript".to_string()),
                steps: Vec::new(),
            },
            ScriptMovement {
                label: "MissingMovement".to_string(),
                source_script: Some("MissingMovementScript".to_string()),
                steps: Vec::new(),
            },
            ScriptMovement {
                label: "MalformedMovement".to_string(),
                source_script: Some("Missing Movement Script".to_string()),
                steps: Vec::new(),
            },
            ScriptMovement {
                label: "SharedMovement".to_string(),
                source_script: None,
                steps: Vec::new(),
            },
        ];
        module.script_field_pickups = vec![
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_HIDDEN_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "KnownScript".to_string(),
                command_index: 2,
            },
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_HIDDEN_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "MissingPickupScript".to_string(),
                command_index: 3,
            },
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_HIDDEN_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "Missing Pickup Script".to_string(),
                command_index: 4,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_command_source_script"
                && diagnostic.subject == "Start:script_movement:MissingMovementScript"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_command_source_script"
                && diagnostic.subject == "Start:script_field_pickup:MissingPickupScript:3"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_command_source_script"
                && diagnostic.subject == "Start:script_movement:Missing Movement Script"
                && diagnostic.message.contains("Missing Movement Script")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_command_source_script"
                && diagnostic.subject == "Start:script_field_pickup:Missing Pickup Script:4"
                && diagnostic.message.contains("Missing Pickup Script")
        }));
        for rejected in [
            "Start:script_movement:Missing Movement Script",
            "Start:script_field_pickup:Missing Pickup Script:4",
        ] {
            assert!(
                !report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "unknown_command_source_script"
                        && diagnostic.subject == rejected
                }),
                "malformed source script should not cascade to unknown: {rejected}"
            );
        }
        for accepted in [
            "Start:script_movement:KnownScript",
            "Start:script_movement:SharedMovement",
            "Start:script_field_pickup:KnownScript:2",
        ] {
            assert!(
                !report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "unknown_command_source_script"
                        && diagnostic.subject == accepted
                }),
                "accepted source reference was rejected: {accepted}"
            );
        }
    }

    #[test]
    fn verifier_allows_duplicate_runtime_script_labels_across_maps() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.scripts.insert(
            "SharedRuntimeScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "verbosegiveitem",
                "args": ["POTION"]
            })]),
        );
        start.script_item_grants = vec![ScriptItemGrant {
            command: "verbosegiveitem".to_string(),
            item_id: "POTION".to_string(),
            quantity: 1,
            source_script: "SharedRuntimeScript".to_string(),
            command_index: 0,
            verbose: true,
        }];
        let mut route = test_map_module("Route29", "ROUTE_29", None);
        route.scripts.insert(
            "SharedRuntimeScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "verbosegiveitem",
                "args": ["POTION"]
            })]),
        );
        route.script_item_grants = vec![ScriptItemGrant {
            command: "verbosegiveitem".to_string(),
            item_id: "POTION".to_string(),
            quantity: 1,
            source_script: "SharedRuntimeScript".to_string(),
            command_index: 0,
            verbose: true,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), start), ("Route29".to_string(), route)]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.severity == VerificationSeverity::Error
                    && diagnostic.code == "duplicate_runtime_script_label"
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn verifier_rejects_elevator_floors_with_missing_target_warps() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.scripts.insert(
            "ElevatorScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "elevator",
                "args": ["ElevatorData"]
            })]),
        );
        start
            .scripts
            .insert("ElevatorData".to_string(), Value::Array(Vec::new()));
        start.script_elevators.insert(
            "ElevatorScript:0".to_string(),
            ScriptElevatorDefinition {
                source_script: "ElevatorScript".to_string(),
                elevator_command_index: 0,
                data_label: "ElevatorData".to_string(),
                floors: vec![ScriptRuntimeElevatorFloor {
                    floor: "1F".to_string(),
                    warp: 9,
                    target_map: "Start".to_string(),
                    source_script: "ElevatorData".to_string(),
                    command_index: 0,
                }],
            },
        );
        let data = GameDataSet {
            maps: [("Start".to_string(), start)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == VerificationSeverity::Error
                && diagnostic.code == "unknown_script_elevator_warp"
                && diagnostic.subject == "Start:ElevatorScript:0:0"
                && diagnostic.message.contains("missing warp 9 on Start")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_fruit_tree_catalog_items_without_case_coercion() {
        let data = GameDataSet {
            fruit_trees: FruitTreeCatalog(
                [
                    (" FRUITTREE_ROUTE_29".to_string(), "BERRY".to_string()),
                    ("FRUITTREE_ROUTE_30".to_string(), "GOLD BERRY".to_string()),
                    ("FRUITTREE_ROUTE_29".to_string(), "berry".to_string()),
                ]
                .into_iter()
                .collect(),
            ),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_fruit_tree_item"
                && diagnostic.subject == "fruit_trees:FRUITTREE_ROUTE_29"
                && diagnostic.message.contains("berry")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_fruit_tree_item"
                && diagnostic.subject == "fruit_trees:FRUITTREE_ROUTE_30"
                && diagnostic.message.contains("GOLD BERRY")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_fruit_tree_id"
                && diagnostic.subject == "fruit_trees: FRUITTREE_ROUTE_29"
        }));
    }

    #[test]
    fn verifier_rejects_referenced_fruit_tree_without_catalog() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![
            ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some("FRUITTREE_ROUTE_29".to_string()),
                source_script: "Route29FruitTree".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some("FRUITTREE ROUTE_29".to_string()),
                source_script: "Route29BadFruitTree".to_string(),
                command_index: 1,
            },
        ];
        let data = GameDataSet {
            maps: [("Route29".to_string(), module)].into_iter().collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            fruit_trees: FruitTreeCatalog::default(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_field_fruit_tree"
                && diagnostic.subject == "Route29:Route29FruitTree:0"
                && diagnostic.message.contains("FRUITTREE_ROUTE_29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_field_pickup_invalid_fruit_tree"
                && diagnostic.subject == "Route29:Route29BadFruitTree:1"
                && diagnostic.message.contains("FRUITTREE ROUTE_29")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_field_fruit_tree"
                && diagnostic.subject == "Route29:Route29BadFruitTree:1"
        }));
    }

    #[test]
    fn verifier_rejects_unresolved_script_economy_constants_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![
            ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["route43gate_toll".to_string()],
                source_script: "TollScript".to_string(),
                command_index: 2,
            },
            ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some(" YOUR_MONEY".to_string()),
                amount_tokens: vec!["ROUTE43GATE_TOLL".to_string()],
                source_script: "PaddedAccountScript".to_string(),
                command_index: 3,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog(
                [("ROUTE43GATE_TOLL".to_string(), 1_000)]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unresolved_script_currency_amount"
                && diagnostic.subject == "Start:TollScript:2"
                && diagnostic.severity == VerificationSeverity::Error
                && diagnostic.message.contains("route43gate_toll")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_money_account"
                && diagnostic.subject == "Start:PaddedAccountScript:3"
                && diagnostic.message.contains(" YOUR_MONEY")
        }));
    }

    #[test]
    fn verifier_rejects_money_mutation_without_pack_max_money() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "takemoney".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec!["PRICE".to_string()],
            source_script: "BuyScript".to_string(),
            command_index: 4,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect()),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_money_cap"
                && diagnostic.subject == "Start:BuyScript:4"
                && diagnostic.message.contains("MAX_MONEY")
        }));
    }

    #[test]
    fn verifier_rejects_coin_mutation_without_pack_max_coins() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "givecoins".to_string(),
            account: None,
            amount_tokens: vec!["PRICE".to_string()],
            source_script: "PrizeScript".to_string(),
            command_index: 5,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect()),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_coin_cap"
                && diagnostic.subject == "Start:PrizeScript:5"
                && diagnostic.message.contains("MAX_COINS")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_gift_pokemon_facts_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.gift_pokemon_scripts = vec![
            GiftPokemonScript {
                species_id: "cyndaquil".to_string(),
                level_token: "5".to_string(),
                level: 5,
                held_item_id: Some("berry".to_string()),
                nickname_label: Some("giftstartername".to_string()),
                ot_label: Some(" GiftOtText".to_string()),
                source_script: "StarterScript".to_string(),
                command_index: 2,
                egg: false,
            },
            GiftPokemonScript {
                species_id: "CYNDA QUIL".to_string(),
                level_token: "5".to_string(),
                level: 5,
                held_item_id: Some("BERRY JUICE".to_string()),
                nickname_label: None,
                ot_label: None,
                source_script: "StarterScript".to_string(),
                command_index: 3,
                egg: false,
            },
        ];
        module
            .scripts
            .insert("GiftStarterName".to_string(), Value::Array(Vec::new()));
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            pokemon: [("CYNDAQUIL".to_string(), species())].into_iter().collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_species"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains("cyndaquil")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_item"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains("berry")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_label"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains("giftstartername")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_gift_pokemon_label"
                && diagnostic.subject == "Start:StarterScript:2"
                && diagnostic.message.contains(" GiftOtText")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_gift_pokemon_species"
                && diagnostic.subject == "Start:StarterScript:3"
                && diagnostic.message.contains("CYNDA QUIL")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_gift_pokemon_item"
                && diagnostic.subject == "Start:StarterScript:3"
                && diagnostic.message.contains("BERRY JUICE")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_species"
                && diagnostic.subject == "Start:StarterScript:3"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_gift_pokemon_item"
                && diagnostic.subject == "Start:StarterScript:3"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_flag_commands_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_flag_commands = vec![
            ScriptFlagCommand {
                command: "SET_EVENT".to_string(),
                flag_id: "EVENT_ROUTE_29_POTION".to_string(),
                source_script: "RouteScript".to_string(),
                command_index: 4,
            },
            ScriptFlagCommand {
                command: "toggleevent".to_string(),
                flag_id: "EVENT_ROUTE_29_POTION".to_string(),
                source_script: "RouteScript".to_string(),
                command_index: 7,
            },
            ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: String::new(),
                source_script: "RouteScript".to_string(),
                command_index: 5,
            },
            ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: " EVENT_ROUTE_29_POTION".to_string(),
                source_script: "RouteScript".to_string(),
                command_index: 6,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_flag_command"
                && diagnostic.subject == "Start:RouteScript:4"
                && diagnostic.message.contains("SET_EVENT")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_flag_command"
                && diagnostic.subject == "Start:RouteScript:7"
                && diagnostic.message.contains("toggleevent")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_script_flag_id" && diagnostic.subject == "Start:RouteScript:5"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_flag_id"
                && diagnostic.subject == "Start:RouteScript:6"
                && diagnostic.message.contains(" EVENT_ROUTE_29_POTION")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_scene_commands_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_scene_commands = vec![
            ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: Some("START_MAP".to_string()),
                scene_id: None,
                source_script: "SceneScript".to_string(),
                command_index: 1,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: None,
                scene_id: None,
                source_script: "SceneScript".to_string(),
                command_index: 2,
            },
            ScriptSceneCommand {
                command: "checkscene".to_string(),
                map_id: None,
                scene_id: Some("SCENE_START_OPEN".to_string()),
                source_script: "SceneScript".to_string(),
                command_index: 3,
            },
            ScriptSceneCommand {
                command: "resetscene".to_string(),
                map_id: None,
                scene_id: None,
                source_script: "SceneScript".to_string(),
                command_index: 4,
            },
            ScriptSceneCommand {
                command: "SetScene".to_string(),
                map_id: None,
                scene_id: Some("SCENE_START_OPEN".to_string()),
                source_script: "SceneScript".to_string(),
                command_index: 8,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some(" START_MAP".to_string()),
                scene_id: Some("SCENE_START_OPEN".to_string()),
                source_script: "SceneScript".to_string(),
                command_index: 5,
            },
            ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some(" SCENE_START_OPEN".to_string()),
                source_script: "SceneScript".to_string(),
                command_index: 6,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("START MAP".to_string()),
                scene_id: Some("SCENE START_OPEN".to_string()),
                source_script: "SceneScript".to_string(),
                command_index: 7,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unexpected_script_scene_map"
                && diagnostic.subject == "Start:SceneScript:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_scene_id"
                && diagnostic.subject == "Start:SceneScript:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_scene_map"
                && diagnostic.subject == "Start:SceneScript:2"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_scene_id"
                && diagnostic.subject == "Start:SceneScript:2"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unexpected_script_scene_id"
                && diagnostic.subject == "Start:SceneScript:3"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_command"
                && diagnostic.subject == "Start:SceneScript:4"
                && diagnostic.message.contains("resetscene")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_scene_command"
                && diagnostic.subject == "Start:SceneScript:8"
                && diagnostic.message.contains("SetScene")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_scene_map"
                && diagnostic.subject == "Start:SceneScript:5"
                && diagnostic.message.contains(" START_MAP")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_scene_id"
                && diagnostic.subject == "Start:SceneScript:6"
                && diagnostic.message.contains(" SCENE_START_OPEN")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_scene_map"
                && diagnostic.subject == "Start:SceneScript:7"
                && diagnostic.message.contains("START MAP")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_scene_id"
                && diagnostic.subject == "Start:SceneScript:7"
                && diagnostic.message.contains("SCENE START_OPEN")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.subject == "Start:SceneScript:7"
                && (diagnostic.code == "unknown_script_scene_map"
                    || diagnostic.code == "unknown_script_scene_id")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_warp_targets_without_normalization() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.script_map_commands = vec![
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("destination".to_string()),
                x: Some(4),
                y: Some(5),
                facing: None,
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 2,
            },
            ScriptMapCommand {
                command: "warpfacing".to_string(),
                target_map: Some("Destination".to_string()),
                x: Some(4),
                y: Some(5),
                facing: Some("up".to_string()),
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 3,
            },
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("NONE".to_string()),
                x: Some(1),
                y: Some(0),
                facing: None,
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 4,
            },
        ];
        let destination = test_map_module("Destination", "DESTINATION", None);
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), start),
                ("Destination".to_string(), destination),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_warp_map"
                && diagnostic.subject == "Start:WarpScript:2"
                && diagnostic.message.contains("destination")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_warp_facing"
                && diagnostic.subject == "Start:WarpScript:3"
                && diagnostic.message.contains("up")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "malformed_script_no_warp_sentinel"
                && diagnostic.subject == "Start:WarpScript:4"
        }));
    }

    #[test]
    fn verifier_rejects_script_warp_destinations_outside_runtime_map_bounds() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.script_map_commands = vec![ScriptMapCommand {
            command: "warpfacing".to_string(),
            target_map: Some("Destination".to_string()),
            x: Some(4),
            y: Some(0),
            facing: Some("RIGHT".to_string()),
            map_setup: None,
            source_script: "WarpScript".to_string(),
            command_index: 3,
        }];
        let destination = test_map_module("Destination", "DESTINATION", None);
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), start),
                ("Destination".to_string(), destination),
            ]
            .into_iter()
            .collect(),
            tilesets: [("johto".to_string(), test_tileset_definition())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_warp_destination_out_of_bounds"
                && diagnostic.subject == "Start:WarpScript:3"
                && diagnostic
                    .message
                    .contains("target Destination raw coordinate (4, 0)")
                && diagnostic.message.contains("outside map bounds 2x2")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_map_commands_without_normalization() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.script_map_commands = vec![
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: None,
                x: Some(1),
                y: None,
                facing: None,
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 1,
            },
            ScriptMapCommand {
                command: "warpfacing".to_string(),
                target_map: Some("START_MAP".to_string()),
                x: Some(1),
                y: Some(2),
                facing: None,
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 2,
            },
            ScriptMapCommand {
                command: "warpcheck".to_string(),
                target_map: Some("START_MAP".to_string()),
                x: Some(1),
                y: Some(2),
                facing: Some("DOWN".to_string()),
                map_setup: Some("MAPSETUP_WARP".to_string()),
                source_script: "MapScript".to_string(),
                command_index: 3,
            },
            ScriptMapCommand {
                command: "newloadmap".to_string(),
                target_map: Some("START_MAP".to_string()),
                x: None,
                y: None,
                facing: None,
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 4,
            },
            ScriptMapCommand {
                command: "loadmap".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 5,
            },
            ScriptMapCommand {
                command: "Warp".to_string(),
                target_map: Some("START_MAP".to_string()),
                x: Some(1),
                y: Some(2),
                facing: None,
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 9,
            },
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some(" START_MAP".to_string()),
                x: Some(1),
                y: Some(2),
                facing: None,
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 6,
            },
            ScriptMapCommand {
                command: "warpfacing".to_string(),
                target_map: Some("START_MAP".to_string()),
                x: Some(1),
                y: Some(2),
                facing: Some(" DOWN".to_string()),
                map_setup: None,
                source_script: "MapScript".to_string(),
                command_index: 7,
            },
            ScriptMapCommand {
                command: "reanchormap".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: Some(" MAPSETUP_WARP".to_string()),
                source_script: "MapScript".to_string(),
                command_index: 8,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), start)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            ("missing_script_warp_map", "Start:MapScript:1"),
            ("missing_script_warp_coordinates", "Start:MapScript:1"),
            ("missing_script_warp_facing", "Start:MapScript:2"),
            ("unexpected_script_warp_destination", "Start:MapScript:3"),
            ("unexpected_script_warp_facing", "Start:MapScript:3"),
            ("unexpected_script_map_setup", "Start:MapScript:3"),
            ("unexpected_script_warp_destination", "Start:MapScript:4"),
            ("missing_script_map_setup", "Start:MapScript:4"),
            ("unknown_script_map_command", "Start:MapScript:5"),
            ("invalid_script_map_command", "Start:MapScript:9"),
            ("invalid_script_warp_map", "Start:MapScript:6"),
            ("invalid_script_warp_facing", "Start:MapScript:7"),
            ("invalid_script_map_setup", "Start:MapScript:8"),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| { diagnostic.code == code && diagnostic.subject == subject }),
                "missing {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_script_text_labels_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "GreetingText".to_string(),
            serde_json::json!([
                {"command": "text", "args": "\"Hello.\""},
                {"command": "done", "args": []}
            ]),
        );
        module.scripts.insert(
            ".LocalText@GreetingScript".to_string(),
            serde_json::json!([
                {"command": "text", "args": "\"Local.\""},
                {"command": "done", "args": []}
            ]),
        );
        module.script_text_commands = vec![
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("greetingtext".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 2,
            },
            ScriptTextCommand {
                command: "waitbutton".to_string(),
                text_label: Some("GreetingText".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 3,
            },
            ScriptTextCommand {
                command: "jumptext".to_string(),
                text_label: None,
                source_script: "GreetingScript".to_string(),
                command_index: 4,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some(".MissingLocal".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 5,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some(" GreetingText".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 6,
            },
            ScriptTextCommand {
                command: "text".to_string(),
                text_label: Some("GreetingText".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 7,
            },
            ScriptTextCommand {
                command: "JumpText".to_string(),
                text_label: Some("GreetingText".to_string()),
                source_script: "GreetingScript".to_string(),
                command_index: 8,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:2"
                && diagnostic.message.contains("greetingtext")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unexpected_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:3"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:4"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:5"
                && diagnostic.message.contains(".MissingLocal")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_text_label"
                && diagnostic.subject == "Start:GreetingScript:6"
                && diagnostic.message.contains(" GreetingText")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_text_command"
                && diagnostic.subject == "Start:GreetingScript:7"
                && diagnostic.message.contains("text")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_text_command"
                && diagnostic.subject == "Start:GreetingScript:8"
                && diagnostic.message.contains("JumpText")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_variable_commands_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_variable_commands = vec![
            ScriptVariableCommand {
                command: "checktime".to_string(),
                target: None,
                value_tokens: vec!["night".to_string()],
                source_script: "VarScript".to_string(),
                command_index: 1,
            },
            ScriptVariableCommand {
                command: "readvar".to_string(),
                target: Some(String::new()),
                value_tokens: Vec::new(),
                source_script: "VarScript".to_string(),
                command_index: 2,
            },
            ScriptVariableCommand {
                command: "setval".to_string(),
                target: Some("VAR_BADGES".to_string()),
                value_tokens: vec!["7".to_string()],
                source_script: "VarScript".to_string(),
                command_index: 3,
            },
            ScriptVariableCommand {
                command: "readmem".to_string(),
                target: Some(" wVanceFightCount".to_string()),
                value_tokens: Vec::new(),
                source_script: "VarScript".to_string(),
                command_index: 4,
            },
            ScriptVariableCommand {
                command: "setval".to_string(),
                target: None,
                value_tokens: vec![" TRUE".to_string()],
                source_script: "VarScript".to_string(),
                command_index: 5,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for index in [1, 2, 3, 4, 5] {
            let subject = format!("Start:VarScript:{index}");
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "invalid_script_variable_command"
                        && diagnostic.subject == subject
                }),
                "missing diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_malformed_script_control_commands_without_target_fallbacks() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "MainScript".to_string(),
            serde_json::json!([
                {"command": "iftrue", "args": [".Done"]},
                {"command": "end", "args": []}
            ]),
        );
        module.script_control_commands = vec![
            ScriptControlCommand {
                command: "iftrue".to_string(),
                compare_value: Some("TRUE".to_string()),
                target_label: Some(".Done".to_string()),
                resolved_target_script: Some(".Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 0,
            },
            ScriptControlCommand {
                command: "ifequal".to_string(),
                compare_value: Some("TRUE".to_string()),
                target_label: Some(".missing".to_string()),
                resolved_target_script: Some(".missing@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 1,
            },
            ScriptControlCommand {
                command: "sjump".to_string(),
                compare_value: None,
                target_label: Some(".Done".to_string()),
                resolved_target_script: None,
                source_script: "MainScript".to_string(),
                command_index: 2,
            },
            ScriptControlCommand {
                command: "ifequal".to_string(),
                compare_value: Some(" TRUE".to_string()),
                target_label: Some(".Done".to_string()),
                resolved_target_script: Some(".Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 3,
            },
            ScriptControlCommand {
                command: "iftrue".to_string(),
                compare_value: None,
                target_label: Some(" .Done".to_string()),
                resolved_target_script: Some(".Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 4,
            },
            ScriptControlCommand {
                command: "iftrue".to_string(),
                compare_value: None,
                target_label: Some(".Done".to_string()),
                resolved_target_script: Some(" .Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 5,
            },
            ScriptControlCommand {
                command: String::new(),
                compare_value: None,
                target_label: Some(".Done".to_string()),
                resolved_target_script: Some(".Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 6,
            },
            ScriptControlCommand {
                command: " iftrue".to_string(),
                compare_value: None,
                target_label: Some(".Done".to_string()),
                resolved_target_script: Some(".Done@MainScript".to_string()),
                source_script: "MainScript".to_string(),
                command_index: 7,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_control_command"
                && diagnostic.subject == "Start:MainScript:0"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_control_target"
                && diagnostic.subject == "Start:MainScript:1"
                && diagnostic.message.contains(".missing@MainScript")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_control_command"
                && diagnostic.subject == "Start:MainScript:2"
        }));
        for index in [3, 4, 5, 6, 7] {
            let subject = format!("Start:MainScript:{index}");
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "invalid_script_control_command"
                        && diagnostic.subject == subject
                }),
                "missing invalid control diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_script_scene_targets_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.map_script_section_commands = vec![MapScriptSectionCommand {
            command: "scene_script".to_string(),
            args: vec!["StartNoopScene".to_string()],
            command_index: 0,
        }];
        module.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_START_OPEN".to_string(),
                script_name: None,
            }],
        };
        module.script_scene_commands = vec![
            ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some("scene_start_open".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 2,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("route_43".to_string()),
                scene_id: Some("0".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 3,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("Route43Gate".to_string()),
                scene_id: Some("0".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 4,
            },
            ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some("0".to_string()),
                source_script: "StartScript".to_string(),
                command_index: 5,
            },
        ];
        let mut target = test_map_module("Route43Gate", "ROUTE_43_GATE", None);
        target.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN".to_string(),
                script_name: None,
            }],
        };
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), module),
                ("Route43Gate".to_string(), target),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_id"
                && diagnostic.subject == "Start:StartScript:2"
                && diagnostic.severity == VerificationSeverity::Error
                && diagnostic.message.contains("scene_start_open")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_map"
                && diagnostic.subject == "Start:StartScript:3"
                && diagnostic.message.contains("route_43")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_map"
                && diagnostic.subject == "Start:StartScript:4"
                && diagnostic.message.contains("Route43Gate")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_scene_id"
                && diagnostic.subject == "Start:StartScript:5"
        }));
    }

    #[test]
    fn verifier_rejects_scene_table_entries_that_reference_missing_scripts() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "KnownSceneScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.scenes = MapSceneTable {
            scenes: vec![
                MapScene {
                    scene_id: "SCENE_START_KNOWN".to_string(),
                    script_name: Some("KnownSceneScript".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_START_CASE_CHANGED".to_string(),
                    script_name: Some("knownscenescript".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_START_MISSING".to_string(),
                    script_name: Some("MissingSceneScript".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_START_MALFORMED".to_string(),
                    script_name: Some("Missing Scene Script".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_START_CONST_ONLY".to_string(),
                    script_name: None,
                },
                MapScene {
                    scene_id: "SCENE_START_KNOWN".to_string(),
                    script_name: None,
                },
            ],
        };
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_scene_script"
                && diagnostic.subject == "Start:SCENE_START_CASE_CHANGED"
                && diagnostic.message.contains("knownscenescript")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_scene_script"
                && diagnostic.subject == "Start:SCENE_START_MISSING"
                && diagnostic.message.contains("MissingSceneScript")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_scene_script"
                && diagnostic.subject == "Start:SCENE_START_MALFORMED"
                && diagnostic.message.contains("Missing Scene Script")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_scene_id"
                && diagnostic.subject == "Start:SCENE_START_KNOWN"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_scene_script"
                && diagnostic.subject == "Start:SCENE_START_MALFORMED"
        }));
        for accepted in ["Start:SCENE_START_KNOWN", "Start:SCENE_START_CONST_ONLY"] {
            assert!(
                !report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "unknown_scene_script" && diagnostic.subject == accepted
                }),
                "accepted scene script was rejected: {accepted}"
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_script_audio_ids_without_normalization() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_audio_commands = vec![
            ScriptAudioCommand {
                command: "playmusic".to_string(),
                audio_id: Some("music_route_29".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 1,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("lugia".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 2,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("LUGIA".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 3,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("CHIKORITA".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 4,
            },
            ScriptAudioCommand {
                command: "playmusic".to_string(),
                audio_id: Some("MUSIC ROUTE 29".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 5,
            },
            ScriptAudioCommand {
                command: "playsound".to_string(),
                audio_id: Some("SFX GET BADGE".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 6,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("HO OH".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 7,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("CELEBI".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 8,
            },
            ScriptAudioCommand {
                command: "PlaySound".to_string(),
                audio_id: Some("SFX_GET_BADGE".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 9,
            },
            ScriptAudioCommand {
                command: "fadeaudio".to_string(),
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "AudioScript".to_string(),
                command_index: 10,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            pokemon: [
                ("LUGIA".to_string(), species()),
                ("CHIKORITA".to_string(), species()),
                ("CELEBI".to_string(), species()),
            ]
            .into_iter()
            .collect(),
            pokemon_cries: [
                (
                    "LUGIA".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY_LUGIA".to_string(),
                        pitch: 0,
                        length: 0,
                    },
                ),
                (
                    "CELEBI".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY CELEBI".to_string(),
                        pitch: 0,
                        length: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            audio: vec![
                ModpackAudioAsset {
                    id: "MUSIC_ROUTE_29".to_string(),
                    path: "content-packs/test/music/MUSIC_ROUTE_29.mid".to_string(),
                    kind: ModpackAudioKind::Music,
                    source: ModpackAudioSource::Midi,
                    pcm_format: None,
                    pcm_frame_count: None,
                    payload_hash: None,
                    loop_start_sample: None,
                    loop_end_sample: None,
                },
                ModpackAudioAsset {
                    id: "CRY_HO_OH".to_string(),
                    path: "content-packs/test/cries/CRY_HO_OH.mid".to_string(),
                    kind: ModpackAudioKind::Cry,
                    source: ModpackAudioSource::Midi,
                    pcm_format: None,
                    pcm_frame_count: None,
                    payload_hash: None,
                    loop_start_sample: None,
                    loop_end_sample: None,
                },
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_music_id"
                && diagnostic.subject == "Start:AudioScript:1"
                && diagnostic.message.contains("music_route_29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_cry_species"
                && diagnostic.subject == "Start:AudioScript:2"
                && diagnostic.message.contains("lugia")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_cry_audio"
                && diagnostic.subject == "Start:AudioScript:3"
                && diagnostic.message.contains("CRY_LUGIA")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_script_cry_metadata"
                && diagnostic.subject == "Start:AudioScript:4"
                && diagnostic.message.contains("CHIKORITA")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_music_id"
                && diagnostic.subject == "Start:AudioScript:5"
                && diagnostic.message.contains("MUSIC ROUTE 29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_sfx_id"
                && diagnostic.subject == "Start:AudioScript:6"
                && diagnostic.message.contains("SFX GET BADGE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_cry_species"
                && diagnostic.subject == "Start:AudioScript:7"
                && diagnostic.message.contains("HO OH")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_cry_audio"
                && diagnostic.subject == "Start:AudioScript:8"
                && diagnostic.message.contains("CRY CELEBI")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_audio_command"
                && diagnostic.subject == "Start:AudioScript:9"
                && diagnostic.message.contains("PlaySound")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_audio_command"
                && diagnostic.subject == "Start:AudioScript:10"
                && diagnostic.message.contains("fadeaudio")
        }));
    }

    #[test]
    fn verifier_requires_every_species_cry_metadata_and_declared_cry_audio() {
        let mut lugia = species();
        lugia.id = "LUGIA".to_string();
        lugia.tmhm_learnset.clear();
        let mut chikorita = species();
        chikorita.id = "CHIKORITA".to_string();
        chikorita.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [
                ("LUGIA".to_string(), lugia),
                ("CHIKORITA".to_string(), chikorita),
            ]
            .into_iter()
            .collect(),
            learnsets: [
                ("LUGIA".to_string(), Vec::new()),
                ("CHIKORITA".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
            evolutions: EvolutionTable(
                [
                    ("LUGIA".to_string(), Vec::new()),
                    ("CHIKORITA".to_string(), Vec::new()),
                ]
                .into_iter()
                .collect(),
            ),
            pokemon_cries: [(
                "LUGIA".to_string(),
                PokemonCryMetadata {
                    cry: "CRY_LUGIA".to_string(),
                    pitch: 0,
                    length: 0,
                },
            )]
            .into_iter()
            .collect(),
            audio: vec![ModpackAudioAsset {
                id: "CRY_HO_OH".to_string(),
                path: "content-packs/test/cries/CRY_HO_OH.mid".to_string(),
                kind: ModpackAudioKind::Cry,
                source: ModpackAudioSource::Midi,
                pcm_format: None,
                pcm_frame_count: None,
                payload_hash: None,
                loop_start_sample: None,
                loop_end_sample: None,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_species_cry_audio"
                && diagnostic.subject == "LUGIA"
                && diagnostic.message.contains("CRY_LUGIA")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_cry_metadata" && diagnostic.subject == "CHIKORITA"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_species_cry_tokens_before_lookup() {
        let mut lugia = species();
        lugia.id = "LUGIA".to_string();
        lugia.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [("LUGIA".to_string(), lugia)].into_iter().collect(),
            learnsets: [("LUGIA".to_string(), Vec::new())].into_iter().collect(),
            evolutions: EvolutionTable([("LUGIA".to_string(), Vec::new())].into_iter().collect()),
            pokemon_cries: [
                (
                    "LUGIA".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY LUGIA".to_string(),
                        pitch: 0,
                        length: 0,
                    },
                ),
                (
                    "HO OH".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY_HO_OH".to_string(),
                        pitch: 0,
                        length: 0,
                    },
                ),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokemon_cry_species"
                && diagnostic.subject == "HO OH"
                && diagnostic.message.contains("HO OH")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_species_cry_audio"
                && diagnostic.subject == "LUGIA"
                && diagnostic.message.contains("CRY LUGIA")
        }));
    }

    #[test]
    fn verifier_requires_script_audio_ids_declared_by_pack_not_path_aliases() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic".to_string(),
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 1,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            audio: vec![ModpackAudioAsset {
                id: "MUSIC_CUSTOM_ROUTE29".to_string(),
                path: "content-packs/test/music/MUSIC_CUSTOM_ROUTE29.mid".to_string(),
                kind: ModpackAudioKind::Music,
                source: ModpackAudioSource::Midi,
                pcm_format: None,
                pcm_frame_count: None,
                payload_hash: None,
                loop_start_sample: None,
                loop_end_sample: None,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_music_id"
                && diagnostic.subject == "Start:AudioScript:1"
                && diagnostic.message.contains("MUSIC_ROUTE_29")
        }));
    }

    #[test]
    fn verifier_requires_map_music_declared_as_exact_music_asset() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.music = Some("MUSIC_ROUTE_29".to_string());
        let wrong_kind_module = {
            let mut module = test_map_module("WrongKind", "WRONG_KIND", None);
            module.attributes.music = Some("SFX_TACKLE".to_string());
            module
        };
        let invalid_module = {
            let mut module = test_map_module("Invalid", "INVALID_MAP", None);
            module.attributes.music = Some("MUSIC ROUTE 29".to_string());
            module
        };
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), module),
                ("WrongKind".to_string(), wrong_kind_module),
                ("Invalid".to_string(), invalid_module),
            ]
            .into_iter()
            .collect(),
            audio: vec![ModpackAudioAsset {
                id: "SFX_TACKLE".to_string(),
                path: "content-packs/test/sfx/SFX_TACKLE.mid".to_string(),
                kind: ModpackAudioKind::SoundEffect,
                source: ModpackAudioSource::Midi,
                pcm_format: None,
                pcm_frame_count: None,
                payload_hash: None,
                loop_start_sample: None,
                loop_end_sample: None,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_map_music_id"
                && diagnostic.subject == "Start"
                && diagnostic.message.contains("MUSIC_ROUTE_29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_map_music_id"
                && diagnostic.subject == "WrongKind"
                && diagnostic.message.contains("SFX_TACKLE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_map_music_id"
                && diagnostic.subject == "Invalid"
                && diagnostic.message.contains("MUSIC ROUTE 29")
        }));
    }

    #[test]
    fn verifier_rejects_audio_assets_not_referenced_by_definitive_pack_data() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.music = Some("MUSIC_ROUTE_29".to_string());
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playsound".to_string(),
            audio_id: Some("SFX_GET_BADGE".to_string()),
            fade_frames: None,
            source_script: "AudioScript".to_string(),
            command_index: 1,
        }];
        let mut chikorita = species();
        chikorita.id = "CHIKORITA".to_string();
        chikorita.tmhm_learnset.clear();
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            pokemon: [("CHIKORITA".to_string(), chikorita)].into_iter().collect(),
            learnsets: [("CHIKORITA".to_string(), Vec::new())]
                .into_iter()
                .collect(),
            evolutions: EvolutionTable(
                [("CHIKORITA".to_string(), Vec::new())]
                    .into_iter()
                    .collect(),
            ),
            pokemon_cries: [(
                "CHIKORITA".to_string(),
                PokemonCryMetadata {
                    cry: "CRY_CHIKORITA".to_string(),
                    pitch: 0,
                    length: 0,
                },
            )]
            .into_iter()
            .collect(),
            oak_ratings: vec![OakRatingEntry {
                caught_count_limit: 1,
                fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                text_label: "OakRating01".to_string(),
            }],
            special_routines: special_routine_rules(["SnorlaxAwake", "GetMysteryGiftItem"]),
            audio: vec![
                ModpackAudioAsset::music(
                    "MUSIC_ROUTE_29",
                    "content-packs/test/music/MUSIC_ROUTE_29.mid",
                )
                .expect("map music asset"),
                ModpackAudioAsset::sound_effect(
                    "SFX_GET_BADGE",
                    "content-packs/test/sfx/SFX_GET_BADGE.mid",
                )
                .expect("script sfx asset"),
                ModpackAudioAsset::sound_effect(
                    "SFX_DEX_FANFARE_LESS_THAN_20",
                    "content-packs/test/sfx/SFX_DEX_FANFARE_LESS_THAN_20.mid",
                )
                .expect("Oak fanfare asset"),
                ModpackAudioAsset::cry(
                    "CRY_CHIKORITA",
                    "content-packs/test/cries/CRY_CHIKORITA.mid",
                )
                .expect("species cry asset"),
                ModpackAudioAsset::music(
                    "MUSIC_POKE_FLUTE_CHANNEL",
                    "content-packs/test/music/MUSIC_POKE_FLUTE_CHANNEL.mid",
                )
                .expect("Poke Flute channel asset"),
                ModpackAudioAsset::sound_effect("SFX_ITEM", "content-packs/test/sfx/SFX_ITEM.mid")
                    .expect("Mystery Gift item sound asset"),
                ModpackAudioAsset::music(
                    "MUSIC_UNUSED",
                    "content-packs/test/music/MUSIC_UNUSED.mid",
                )
                .expect("unused music asset"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unused_audio_asset" && diagnostic.subject == "MUSIC_UNUSED"
        }));
        for used in [
            "MUSIC_ROUTE_29",
            "SFX_GET_BADGE",
            "SFX_DEX_FANFARE_LESS_THAN_20",
            "CRY_CHIKORITA",
            "MUSIC_POKE_FLUTE_CHANNEL",
            "SFX_ITEM",
        ] {
            assert!(
                !report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == "unused_audio_asset"
                        && diagnostic.subject == used),
                "referenced audio asset {used} was reported unused: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_requires_music_none_to_be_declared_by_pack() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "musicfadeout".to_string(),
            audio_id: Some("MUSIC_NONE".to_string()),
            fade_frames: Some(2),
            source_script: "FadeScript".to_string(),
            command_index: 1,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            special_routines: special_routine_rules(["FadeOutMusic"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_music_id"
                && diagnostic.subject == "Start:FadeScript:1"
                && diagnostic.message.contains("MUSIC_NONE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_special_routine_music_id"
                && diagnostic.subject == "special_routines:FadeOutMusic"
                && diagnostic.message.contains("MUSIC_NONE")
        }));
    }

    #[test]
    fn verifier_rejects_init_roam_mons_without_roaming_pokemon_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["InitRoamMons"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_roaming_pokemon_definitions"
                && diagnostic.subject == "special_routines:InitRoamMons"
                && diagnostic.message.contains("roaming Pokemon")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_roaming_pokemon_without_coercion() {
        let data = GameDataSet {
            pokemon: [("RAIKOU".to_string(), species())].into_iter().collect(),
            roaming_pokemon: BTreeMap::from([
                (
                    String::new(),
                    RoamingPokemonDefinition {
                        level: 0,
                        map_group: 1,
                        map_number: 1,
                    },
                ),
                (
                    "RAI KOU".to_string(),
                    RoamingPokemonDefinition {
                        level: 40,
                        map_group: 1,
                        map_number: 2,
                    },
                ),
                (
                    "raikou".to_string(),
                    RoamingPokemonDefinition {
                        level: 40,
                        map_group: 1,
                        map_number: 3,
                    },
                ),
                (
                    "RAIKOU".to_string(),
                    RoamingPokemonDefinition {
                        level: 40,
                        map_group: 1,
                        map_number: 4,
                    },
                ),
            ]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_roaming_pokemon_species"
                && diagnostic.subject == "roaming_pokemon:"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_roaming_pokemon_level"
                && diagnostic.subject == "roaming_pokemon:"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_roaming_pokemon_species"
                && diagnostic.subject == "roaming_pokemon:RAI KOU"
                && diagnostic.message.contains("RAI KOU")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_roaming_pokemon_species"
                && diagnostic.subject == "roaming_pokemon:raikou"
                && diagnostic.message.contains("raikou")
        }));
    }

    #[test]
    fn verifier_rejects_declared_special_routine_unknown_to_rust_runtime() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["ModpackOnlyRoutine", ""]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_declared_special_routine"
                && diagnostic.subject == "special_routines:ModpackOnlyRoutine"
                && diagnostic
                    .message
                    .contains("is not implemented by the Rust runtime")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_special_routine" && diagnostic.subject == "special_routines"
        }));
    }

    #[test]
    fn verifier_rejects_buena_prize_without_buena_prize_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["BuenaPrize"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_buena_prize_definitions"
                && diagnostic.subject == "special_routines:BuenaPrize"
                && diagnostic.message.contains("Buena prize")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_buena_prizes_without_coercion() {
        let data = GameDataSet {
            items: [("ULTRA_BALL".to_string(), test_item("ULTRA_BALL"))]
                .into_iter()
                .collect(),
            buena_prizes: BTreeMap::from([
                (String::new(), 0),
                (" ULTRA_BALL".to_string(), 2),
                ("ultra_ball".to_string(), 2),
                ("ULTRA_BALL".to_string(), 2),
            ]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_buena_prize_item" && diagnostic.subject == "buena_prizes:"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_buena_prize_cost" && diagnostic.subject == "buena_prizes:"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_buena_prize_item"
                && diagnostic.subject == "buena_prizes: ULTRA_BALL"
                && diagnostic.message.contains(" ULTRA_BALL")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_buena_prize_item"
                && diagnostic.subject == "buena_prizes:ultra_ball"
                && diagnostic.message.contains("ultra_ball")
        }));
    }

    #[test]
    fn verifier_rejects_buenas_password_without_buena_password_category_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["BuenasPassword"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_buena_password_categories"
                && diagnostic.subject == "special_routines:BuenasPassword"
                && diagnostic.message.contains("Buena password")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_buena_password_categories_without_coercion() {
        let data = GameDataSet {
            pokemon: [("PIKACHU".to_string(), species())].into_iter().collect(),
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            moves: [("THUNDERBOLT".to_string(), test_move("THUNDERBOLT"))]
                .into_iter()
                .collect(),
            buena_password_categories: BuenaPasswordCategories {
                order: vec![
                    String::new(),
                    " MON".to_string(),
                    "ITEM".to_string(),
                    "MOVE".to_string(),
                    "UNKNOWN".to_string(),
                ],
                categories: BTreeMap::from([
                    (
                        String::new(),
                        BuenaPasswordCategoryDefinition {
                            category_type: "buena mon".to_string(),
                            points: 0,
                            options: Vec::new(),
                        },
                    ),
                    (
                        " MON".to_string(),
                        BuenaPasswordCategoryDefinition {
                            category_type: BUENA_PASSWORD_CATEGORY_MON.to_string(),
                            points: 1,
                            options: vec![
                                String::new(),
                                "PIKACHU ".to_string(),
                                "pikachu".to_string(),
                            ],
                        },
                    ),
                    (
                        "ITEM".to_string(),
                        BuenaPasswordCategoryDefinition {
                            category_type: BUENA_PASSWORD_CATEGORY_ITEM.to_string(),
                            points: 1,
                            options: vec![" POTION".to_string(), "potion".to_string()],
                        },
                    ),
                    (
                        "MOVE".to_string(),
                        BuenaPasswordCategoryDefinition {
                            category_type: BUENA_PASSWORD_CATEGORY_MOVE.to_string(),
                            points: 1,
                            options: vec!["THUNDERBOLT ".to_string(), "thunderbolt".to_string()],
                        },
                    ),
                    (
                        "UNKNOWN".to_string(),
                        BuenaPasswordCategoryDefinition {
                            category_type: "BUENA_UNKNOWN".to_string(),
                            points: 1,
                            options: vec!["TEXT".to_string()],
                        },
                    ),
                ]),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            (
                "empty_buena_password_category_id",
                "buena_password_categories:",
            ),
            (
                "invalid_buena_password_category_type",
                "buena_password_categories:",
            ),
            (
                "invalid_buena_password_points",
                "buena_password_categories:",
            ),
            ("empty_buena_password_options", "buena_password_categories:"),
            (
                "invalid_buena_password_category_id",
                "buena_password_categories: MON",
            ),
            (
                "empty_buena_password_option",
                "buena_password_categories: MON:option:0",
            ),
            (
                "invalid_buena_password_option",
                "buena_password_categories: MON:option:1",
            ),
            (
                "unknown_buena_password_species",
                "buena_password_categories: MON:option:2",
            ),
            (
                "invalid_buena_password_option",
                "buena_password_categories:ITEM:option:0",
            ),
            (
                "unknown_buena_password_item",
                "buena_password_categories:ITEM:option:1",
            ),
            (
                "invalid_buena_password_option",
                "buena_password_categories:MOVE:option:0",
            ),
            (
                "unknown_buena_password_move",
                "buena_password_categories:MOVE:option:1",
            ),
            (
                "unknown_buena_password_category_type",
                "buena_password_categories:UNKNOWN",
            ),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_select_apricorn_without_kurt_apricorn_recipe_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["SelectApricornForKurt"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_kurt_apricorn_recipes"
                && diagnostic.subject == "special_routines:SelectApricornForKurt"
                && diagnostic.message.contains("Kurt apricorn")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_kurt_apricorn_recipes_without_coercion() {
        let data = GameDataSet {
            items: [
                ("BLU_APRICORN".to_string(), test_item("BLU_APRICORN")),
                ("LURE_BALL".to_string(), test_item("LURE_BALL")),
            ]
            .into_iter()
            .collect(),
            kurt_apricorn_recipes: BTreeMap::from([
                (String::new(), String::new()),
                (" BLU_APRICORN".to_string(), "LURE_BALL ".to_string()),
                ("blu_apricorn".to_string(), "lure_ball".to_string()),
                ("BLU_APRICORN".to_string(), "LURE_BALL".to_string()),
            ]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            (
                "empty_kurt_apricorn_recipe_apricorn",
                "kurt_apricorn_recipes:",
            ),
            ("empty_kurt_apricorn_recipe_ball", "kurt_apricorn_recipes:"),
            (
                "invalid_kurt_apricorn_recipe_apricorn",
                "kurt_apricorn_recipes: BLU_APRICORN",
            ),
            (
                "invalid_kurt_apricorn_recipe_ball",
                "kurt_apricorn_recipes: BLU_APRICORN",
            ),
            (
                "unknown_kurt_apricorn_recipe_apricorn",
                "kurt_apricorn_recipes:blu_apricorn",
            ),
            (
                "unknown_kurt_apricorn_recipe_ball",
                "kurt_apricorn_recipes:blu_apricorn",
            ),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_shuckie_routines_without_shuckie_gift_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["GiveShuckle", "ReturnShuckie"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_shuckie_gift"
                && diagnostic.subject == "special_routines:Shuckie"
                && diagnostic.message.contains("Shuckie gift")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_shuckie_gift_without_coercion() {
        let data = GameDataSet {
            pokemon: [("SHUCKLE".to_string(), species())].into_iter().collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            initialize_events: InitializeEventsConfig {
                engine_flags: vec!["ENGINE_GOT_SHUCKIE_TODAY".to_string()],
                ..InitializeEventsConfig::default()
            },
            shuckie_gift: Some(ShuckieGiftDefinition {
                species: String::new(),
                level: 0,
                held_item: String::new(),
                nickname: String::new(),
                original_trainer_name: String::new(),
                original_trainer_id: 518,
                got_today_engine_flag: String::new(),
            }),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for code in [
            "empty_shuckie_gift_species",
            "invalid_shuckie_gift_level",
            "empty_shuckie_gift_item",
            "empty_shuckie_gift_name",
            "empty_shuckie_gift_engine_flag",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code
                        && diagnostic.subject == "shuckie_gift"),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }

        let data = GameDataSet {
            pokemon: [("SHUCKLE".to_string(), species())].into_iter().collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            initialize_events: InitializeEventsConfig {
                engine_flags: vec!["ENGINE_GOT_SHUCKIE_TODAY".to_string()],
                ..InitializeEventsConfig::default()
            },
            shuckie_gift: Some(ShuckieGiftDefinition {
                species: "shuckle".to_string(),
                level: 15,
                held_item: "berry".to_string(),
                nickname: "SHUCKIE".to_string(),
                original_trainer_name: "MANIA".to_string(),
                original_trainer_id: 518,
                got_today_engine_flag: "engine_got_shuckie_today".to_string(),
            }),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for code in [
            "unknown_shuckie_gift_species",
            "unknown_shuckie_gift_item",
            "unknown_shuckie_gift_engine_flag",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code
                        && diagnostic.subject == "shuckie_gift"),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_give_dratini_without_dratini_move_sets_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["GiveDratini"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_dratini_move_sets"
                && diagnostic.subject == "special_routines:GiveDratini"
                && diagnostic.message.contains("Dratini move sets")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_dratini_move_sets_without_unknown_fallback() {
        let data = GameDataSet {
            moves: [("SURF".to_string(), test_move("SURF"))]
                .into_iter()
                .collect(),
            dratini_move_sets: BTreeMap::from([
                (0, Vec::new()),
                (
                    1,
                    vec![
                        String::new(),
                        "EXTREME SPEED".to_string(),
                        "EXTREMESPEED".to_string(),
                    ],
                ),
                (2, vec!["SURF".to_string()]),
            ]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            ("empty_dratini_move_set", "dratini_move_sets:0"),
            ("invalid_dratini_move", "dratini_move_sets:1:move:0"),
            ("invalid_dratini_move", "dratini_move_sets:1:move:1"),
            ("unknown_dratini_move", "dratini_move_sets:1:move:2"),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_malformed_bug_contest_flags_without_unknown_fallback() {
        let data = GameDataSet {
            initialize_events: InitializeEventsConfig {
                event_flags: vec!["EVENT_BUG_CONTESTANT_1".to_string()],
                ..InitializeEventsConfig::default()
            },
            bug_contest_config: Some(BugContestConfig {
                park_balls: 20,
                timer_minutes: 20,
                timer_seconds: 0,
                selected_contestant_count: 4,
                contestant_flags: vec![
                    String::new(),
                    "EVENT_BUG_CONTESTANT_1".to_string(),
                    "EVENT_BUG_CONTESTANT_1".to_string(),
                    "EVENT BUG".to_string(),
                    "EVENT_MISSING".to_string(),
                ],
            }),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, subject) in [
            (
                "invalid_bug_contest_contestant_flag",
                "bug_contest_config:contestant_flags:0",
            ),
            (
                "duplicate_bug_contest_contestant_flag",
                "bug_contest_config:contestant_flags:2",
            ),
            (
                "invalid_bug_contest_contestant_flag",
                "bug_contest_config:contestant_flags:3",
            ),
            (
                "unknown_bug_contest_contestant_flag",
                "bug_contest_config:contestant_flags:4",
            ),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_battle_tower_action_without_battle_tower_rules_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["BattleTowerAction"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_battle_tower_rules"
                && diagnostic.subject == "special_routines:BattleTowerRules"
                && diagnostic.message.contains("Battle Tower rules")
        }));
    }

    #[test]
    fn verifier_rejects_battle_tower_rule_check_without_battle_tower_rules_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["CheckForBattleTowerRules"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_battle_tower_rules"
                && diagnostic.subject == "special_routines:BattleTowerRules"
                && diagnostic.message.contains("Battle Tower rules")
        }));
    }

    #[test]
    fn verifier_rejects_prof_oaks_pc_without_oak_rating_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["ProfOaksPCBoot"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_oak_rating_table"
                && diagnostic.subject == "special_routines:ProfOaksPCBoot"
                && diagnostic.message.contains("Oak rating")
        }));
    }

    #[test]
    fn verifier_rejects_oak_ratings_without_case_or_order_coercion() {
        let mut chikorita = species();
        chikorita.id = "CHIKORITA".to_string();
        let mut cyndaquil = species();
        cyndaquil.id = "CYNDAQUIL".to_string();
        let data = GameDataSet {
            pokemon: [
                ("CHIKORITA".to_string(), chikorita),
                ("CYNDAQUIL".to_string(), cyndaquil),
            ]
            .into_iter()
            .collect(),
            oak_ratings: vec![
                OakRatingEntry {
                    caught_count_limit: 1,
                    fanfare: " SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                    text_label: "OakRating01".to_string(),
                },
                OakRatingEntry {
                    caught_count_limit: 1,
                    fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                    text_label: "".to_string(),
                },
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_oak_rating_fanfare" && diagnostic.subject == "oak_ratings:0"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_oak_rating_text_label"
                && diagnostic.subject == "oak_ratings:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_oak_rating_order" && diagnostic.subject == "oak_ratings:1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "incomplete_oak_rating_coverage"
                && diagnostic.subject == "oak_ratings"
        }));
    }

    #[test]
    fn verifier_rejects_battle_tower_banned_species_without_case_coercion() {
        let mut mewtwo = species();
        mewtwo.id = "MEWTWO".to_string();
        let data = GameDataSet {
            pokemon: [("MEWTWO".to_string(), mewtwo)].into_iter().collect(),
            battle_tower_rules: Some(BattleTowerRules {
                banned_species: BTreeMap::from([
                    (
                        "mewtwo".to_string(),
                        BattleTowerBannedSpeciesRule::default(),
                    ),
                    (
                        " MEWTWO".to_string(),
                        BattleTowerBannedSpeciesRule::default(),
                    ),
                ]),
                required_party_count: 0,
                challenge_streak_length: 0,
                minimum_level_group: 2,
                maximum_level_group: 1,
                level_group_size: 0,
                party_count_failure_text: " OnlyThreeMonMayBeEnteredText".to_string(),
                duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
                duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
                egg_failure_text: "".to_string(),
                trainers: Vec::new(),
                mon_groups: Vec::new(),
            }),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_battle_tower_banned_species"
                && diagnostic.subject == "battle_tower_rules:bannedSpecies:mewtwo"
                && diagnostic.message.contains("mewtwo")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_banned_species"
                && diagnostic.subject == "battle_tower_rules:bannedSpecies: MEWTWO"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_required_party_count"
                && diagnostic.subject == "battle_tower_rules:required_party_count"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_challenge_streak_length"
                && diagnostic.subject == "battle_tower_rules:challengeStreakLength"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_level_group_size"
                && diagnostic.subject == "battle_tower_rules:levelGroupSize"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_level_group_range"
                && diagnostic.subject == "battle_tower_rules:levelGroupRange"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_failure_text"
                && diagnostic.subject == "battle_tower_rules:partyCountFailureText"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_tower_failure_text"
                && diagnostic.subject == "battle_tower_rules:eggFailureText"
        }));
    }

    #[test]
    fn verifier_rejects_give_odd_egg_without_odd_egg_pack_data() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["GiveOddEgg"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_odd_egg_definitions"
                && diagnostic.subject == "special_routines:GiveOddEgg"
                && diagnostic.message.contains("Odd Egg definitions")
        }));
    }

    #[test]
    fn verifier_rejects_odd_egg_species_and_moves_without_case_coercion() {
        let mut cleffa = species();
        cleffa.id = "CLEFFA".to_string();
        let data = GameDataSet {
            pokemon: [("CLEFFA".to_string(), cleffa)].into_iter().collect(),
            moves: [("POUND".to_string(), test_move("POUND"))]
                .into_iter()
                .collect(),
            odd_egg_definitions: vec![OddEggDefinition {
                species: "cleffa".to_string(),
                moves: vec!["pound".to_string()],
                original_trainer_id: 768,
                dvs: [2, 10, 10, 10],
                probability: 100,
                level: 5,
                experience: 125,
                hatch_cycles: 20,
                nickname: "EGG".to_string(),
                original_trainer_name: "ODD".to_string(),
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_odd_egg_species"
                && diagnostic.subject == "odd_egg_definitions:0"
                && diagnostic.message.contains("cleffa")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_odd_egg_move"
                && diagnostic.subject == "odd_egg_definitions:0:move:0"
                && diagnostic.message.contains("pound")
        }));
    }

    #[test]
    fn verifier_rejects_out_of_bounds_script_block_changes_without_resizing() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 2, 3, 4];
        module.script_block_changes = vec![ScriptBlockChange {
            x: 4,
            y: 2,
            block_id: 0x2e,
            source_script: "DoorScript".to_string(),
            command_index: 6,
        }];
        module.scripts.insert(
            "DoorScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_block_change_out_of_bounds"
                && diagnostic.subject == "Start:DoorScript:6"
                && diagnostic.message.contains("(4, 2)")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_runtime_special_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_runtime_commands = vec![
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec!["fadeoutmusic".to_string()],
                source_script: "StartScript".to_string(),
                command_index: 0,
            },
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec!["$FadeOutMusic".to_string()],
                source_script: "StartScript".to_string(),
                command_index: 1,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            special_routines: special_routine_rules(["FadeOutMusic"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_special_routine"
                && diagnostic.subject == "Start:StartScript:0"
                && diagnostic.message.contains("fadeoutmusic")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "malformed_script_runtime_command"
                && diagnostic.subject == "Start:StartScript:1"
                && diagnostic
                    .message
                    .contains("special requires exact nonempty args")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_special_routine"
                && diagnostic.subject == "Start:StartScript:1"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_runtime_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_runtime_commands = vec![
            ScriptRuntimeCommand {
                command: "Special".to_string(),
                args: vec!["FadeOutMusic".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            },
            ScriptRuntimeCommand {
                command: String::new(),
                args: vec!["FadeOutMusic".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 4,
            },
            ScriptRuntimeCommand {
                command: " special".to_string(),
                args: vec!["FadeOutMusic".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 5,
            },
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: Vec::new(),
                source_script: "RuntimeScript".to_string(),
                command_index: 1,
            },
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec![String::new()],
                source_script: "RuntimeScript".to_string(),
                command_index: 2,
            },
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec![" FadeOutMusic".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 3,
            },
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec!["Function11ac3e".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 6,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            special_routines: special_routine_rules(["FadeOutMusic", "Function11ac3e"]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_runtime_command"
                && diagnostic.subject == "Start:RuntimeScript:0"
                && diagnostic.message.contains("Special")
        }));
        for index in [1, 2, 3, 4, 5] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "malformed_script_runtime_command"
                        && diagnostic.subject == format!("Start:RuntimeScript:{index}")
                }),
                "missing malformed runtime command diagnostic for index {index}: {:?}",
                report.diagnostics
            );
        }
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "inactive_script_special_routine"
                && diagnostic.subject == "Start:RuntimeScript:6"
                && diagnostic.message.contains("Function11ac3e")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_text_bodies_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_text_bodies.insert(
            "GreetingText".to_string(),
            ScriptTextBody {
                label: " greetingtext".to_string(),
                commands: vec![
                    ScriptTextBodyCommand {
                        command: "Text".to_string(),
                        args: vec!["\"Hi!\"".to_string()],
                        command_index: 0,
                    },
                    ScriptTextBodyCommand {
                        command: "done".to_string(),
                        args: vec!["\"extra\"".to_string()],
                        command_index: 1,
                    },
                ],
            },
        );
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_text_body_label_mismatch"
                && diagnostic.subject == "Start:GreetingText"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_text_body_label"
                && diagnostic.subject == "Start: greetingtext"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_text_body_command"
                && diagnostic.subject == "Start:GreetingText:0"
                && diagnostic.message.contains("Text")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "malformed_script_text_body_command"
                && diagnostic.subject == "Start:GreetingText:1"
                && diagnostic.message.contains("done expects 0 args")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_menu_definitions_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_menu_definitions.insert(
            "ChoiceMenu".to_string(),
            ScriptMenuDefinition {
                label: " choicemenu".to_string(),
                commands: vec![
                    ScriptMenuCommand {
                        command: "verticalmenu".to_string(),
                        args: Vec::new(),
                        command_index: 0,
                    },
                    ScriptMenuCommand {
                        command: "db".to_string(),
                        args: vec!["one".to_string(), "two".to_string()],
                        command_index: 1,
                    },
                    ScriptMenuCommand {
                        command: "menu_coords".to_string(),
                        args: vec![
                            "0".to_string(),
                            "0".to_string(),
                            "SCREEN_RIGHT".to_string(),
                            "8".to_string(),
                        ],
                        command_index: 2,
                    },
                ],
            },
        );
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_menu_label_mismatch"
                && diagnostic.subject == "Start:ChoiceMenu"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_menu_label"
                && diagnostic.subject == "Start: choicemenu"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_menu_command"
                && diagnostic.subject == "Start:ChoiceMenu:0"
                && diagnostic.message.contains("verticalmenu")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "malformed_script_menu_command"
                && diagnostic.subject == "Start:ChoiceMenu:1"
                && diagnostic.message.contains("db expects one of")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_menu_coordinates"
                && diagnostic.subject == "Start:ChoiceMenu:2"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_map_section_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts = BTreeMap::from([("KnownScript".to_string(), Value::Array(Vec::new()))]);
        module.map_script_section_commands = vec![
            MapScriptSectionCommand {
                command: "scene_script".to_string(),
                args: vec!["missing_script".to_string()],
                command_index: 1,
            },
            MapScriptSectionCommand {
                command: "callback".to_string(),
                args: vec![
                    "MAPCALLBACK_OBJECTS".to_string(),
                    "MissingCallback".to_string(),
                ],
                command_index: 2,
            },
            MapScriptSectionCommand {
                command: "scene_script".to_string(),
                args: vec![" KnownScript".to_string()],
                command_index: 6,
            },
        ];
        module.map_event_section_commands = vec![
            MapEventSectionCommand {
                command: "warp_event".to_string(),
                args: vec!["1".to_string(), "2".to_string()],
                command_index: 3,
            },
            MapEventSectionCommand {
                command: "bg_event".to_string(),
                args: vec![
                    "1".to_string(),
                    "2".to_string(),
                    "BGEVENT_READ".to_string(),
                    "MissingSign".to_string(),
                ],
                command_index: 4,
            },
            MapEventSectionCommand {
                command: "object_event".to_string(),
                args: vec![
                    "0".to_string(),
                    "0".to_string(),
                    "SPRITE_MON".to_string(),
                    "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "-1".to_string(),
                    "-1".to_string(),
                    "PAL_NPC_RED".to_string(),
                    "OBJECTTYPE_SCRIPT".to_string(),
                    "0".to_string(),
                    "MissingObjectScript".to_string(),
                    "-1".to_string(),
                ],
                command_index: 5,
            },
            MapEventSectionCommand {
                command: "bg_event".to_string(),
                args: vec![
                    "1".to_string(),
                    "2".to_string(),
                    "BGEVENT_READ".to_string(),
                    " KnownScript".to_string(),
                ],
                command_index: 6,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_map_scene_script",
            "unknown_map_callback_script",
            "malformed_map_event_section_command",
            "unknown_map_event_script",
            "unknown_map_object_event_script",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
        for subject in ["Start:map_scripts:6", "Start:map_events:6"] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code.starts_with("malformed_map_")
                        && diagnostic.subject == subject
                        && diagnostic.message.contains(" KnownScript")
                }),
                "missing malformed exact operand for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_small_runtime_references_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_runtime_commands = vec![
            ScriptRuntimeCommand {
                command: "addcellnum".to_string(),
                args: vec!["phone_elm".to_string()],
                source_script: "PhoneScript".to_string(),
                command_index: 0,
            },
            ScriptRuntimeCommand {
                command: "specialphonecall".to_string(),
                args: vec!["specialcall_masterball".to_string()],
                source_script: "PhoneScript".to_string(),
                command_index: 1,
            },
            ScriptRuntimeCommand {
                command: "checkpoke".to_string(),
                args: vec!["pikachu".to_string()],
                source_script: "SpeciesScript".to_string(),
                command_index: 2,
            },
            ScriptRuntimeCommand {
                command: "checkpoke".to_string(),
                args: vec!["PIKA+CHU".to_string()],
                source_script: "SpeciesScript".to_string(),
                command_index: 5,
            },
            ScriptRuntimeCommand {
                command: "getitemname".to_string(),
                args: vec!["BUFFER_1".to_string(), "$POTION".to_string()],
                source_script: "ItemScript".to_string(),
                command_index: 6,
            },
            ScriptRuntimeCommand {
                command: "getmonname".to_string(),
                args: vec!["BUFFER_1".to_string(), "$PIKACHU".to_string()],
                source_script: "MonNameScript".to_string(),
                command_index: 7,
            },
            ScriptRuntimeCommand {
                command: "trade".to_string(),
                args: vec!["npc_trade_mike".to_string()],
                source_script: "TradeScript".to_string(),
                command_index: 3,
            },
            ScriptRuntimeCommand {
                command: "addcellnum".to_string(),
                args: vec!["PHONE ELM".to_string()],
                source_script: "PhoneScript".to_string(),
                command_index: 8,
            },
            ScriptRuntimeCommand {
                command: "specialphonecall".to_string(),
                args: vec!["SPECIAL CALL MASTERBALL".to_string()],
                source_script: "PhoneScript".to_string(),
                command_index: 9,
            },
            ScriptRuntimeCommand {
                command: "trade".to_string(),
                args: vec!["NPC TRADE MIKE".to_string()],
                source_script: "TradeScript".to_string(),
                command_index: 10,
            },
            ScriptRuntimeCommand {
                command: "gettrainername".to_string(),
                args: vec![
                    "STRING_BUFFER_4".to_string(),
                    "$YOUNGSTER".to_string(),
                    "$YOUNGSTER_JOEY".to_string(),
                ],
                source_script: "TrainerScript".to_string(),
                command_index: 11,
            },
            ScriptRuntimeCommand {
                command: "callasm".to_string(),
                args: vec![".missing".to_string()],
                source_script: "AsmScript".to_string(),
                command_index: 4,
            },
            ScriptRuntimeCommand {
                command: "callasm".to_string(),
                args: vec!["$Missing".to_string()],
                source_script: "AsmScript".to_string(),
                command_index: 12,
            },
        ];
        module.scripts = BTreeMap::from([("AsmScript".to_string(), Value::Array(Vec::new()))]);
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            phone_contacts: PhoneContactCatalog(BTreeMap::from([(
                "PHONE_ELM".to_string(),
                PhoneContactRecord {
                    contact_id: "PHONE_ELM".to_string(),
                    trainer_class: Some("TRAINER_NONE".to_string()),
                    trainer_label: Some("PHONECONTACT_ELM".to_string()),
                    lines: vec!["ELM:".to_string()],
                    primary_label: "ELM".to_string(),
                    map_constant: Some("ELMS_LAB".to_string()),
                    callee_time_mask: 7,
                    callee_script: Some("ElmPhoneCalleeScript".to_string()),
                    caller_time_mask: 0,
                    caller_script: None,
                },
            )])),
            special_phone_calls: BTreeMap::from([(
                "SPECIALCALL_MASTERBALL".to_string(),
                SpecialPhoneCallRule::default(),
            )]),
            npc_trades: npc_trade_rules(["NPC_TRADE_MIKE"]),
            pokemon: [("PIKACHU".to_string(), species())].into_iter().collect(),
            ..GameDataSet::default()
        };
        let mut data = data;
        add_test_trainer(&mut data, "");

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "unknown_script_addcellnum_contact",
            "invalid_script_addcellnum_contact",
            "unknown_script_special_phone_call",
            "invalid_script_special_phone_call",
            "unknown_script_species_runtime_command",
            "invalid_script_species_runtime_command",
            "invalid_script_item_name",
            "invalid_script_mon_name",
            "invalid_script_trainer_class",
            "invalid_script_trainer_name",
            "unknown_script_npc_trade",
            "invalid_script_npc_trade",
            "unknown_script_runtime_target",
            "invalid_script_runtime_target",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
        for (code, subject) in [
            ("unknown_script_addcellnum_contact", "Start:PhoneScript:8"),
            ("unknown_script_special_phone_call", "Start:PhoneScript:9"),
            ("unknown_script_npc_trade", "Start:TradeScript:10"),
            ("unknown_script_trainer_name", "Start:TrainerScript:11"),
            ("unknown_script_runtime_target", "Start:AsmScript:12"),
        ] {
            assert!(
                !report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "malformed token should not cascade to {code} at {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_phone_contact_maps_without_case_coercion() {
        fn phone_contact(contact_id: &str, map_constant: Option<&str>) -> PhoneContactRecord {
            PhoneContactRecord {
                contact_id: contact_id.to_string(),
                trainer_class: Some("TRAINER_NONE".to_string()),
                trainer_label: Some(format!("PHONECONTACT_{contact_id}")),
                lines: vec![format!("{contact_id}:")],
                primary_label: contact_id.to_string(),
                map_constant: map_constant.map(str::to_string),
                callee_time_mask: 7,
                callee_script: None,
                caller_time_mask: 0,
                caller_script: None,
            }
        }

        let mut empty_lines = phone_contact("PHONE_LINES", None);
        empty_lines.lines = vec![String::new()];
        let mut mismatch = phone_contact("PHONE_MISMATCH", None);
        mismatch.primary_label = "OTHER_LABEL".to_string();
        let data = GameDataSet {
            maps: [(
                "ElmsLab".to_string(),
                test_map_module("ElmsLab", "ELMS_LAB", None),
            )]
            .into_iter()
            .collect(),
            phone_contacts: PhoneContactCatalog(BTreeMap::from([
                (
                    "PHONE_ELM".to_string(),
                    phone_contact("PHONE_ELM", Some("ELMS_LAB")),
                ),
                (
                    "PHONE_CASE".to_string(),
                    phone_contact("PHONE_CASE", Some("elms_lab")),
                ),
                (
                    "PHONE_EMPTY".to_string(),
                    phone_contact("PHONE_EMPTY", Some("")),
                ),
                (
                    "PHONE_BAD_MAP".to_string(),
                    phone_contact("PHONE_BAD_MAP", Some("ELMS LAB")),
                ),
                ("PHONE_LINES".to_string(), empty_lines),
                ("PHONE_MISMATCH".to_string(), mismatch),
                (
                    " PHONE_PADDED".to_string(),
                    phone_contact(" PHONE_PADDED", None),
                ),
            ])),
            permanent_phone_numbers: BTreeMap::from([
                ("PHONE MOM".to_string(), PermanentPhoneNumberRule::default()),
                ("phone_mom".to_string(), PermanentPhoneNumberRule::default()),
            ]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_phone_contact_map"
                && diagnostic.subject == "phone_contacts:PHONE_CASE"
                && diagnostic.message.contains("elms_lab")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "empty_phone_contact_map"
                && diagnostic.subject == "phone_contacts:PHONE_EMPTY"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_phone_contact_map"
                && diagnostic.subject == "phone_contacts:PHONE_BAD_MAP"
                && diagnostic.message.contains("ELMS LAB")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_phone_contact_map"
                && diagnostic.subject == "phone_contacts:PHONE_BAD_MAP"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_phone_contact_lines"
                && diagnostic.subject == "phone_contacts:PHONE_LINES"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "phone_contact_primary_label_mismatch"
                && diagnostic.subject == "phone_contacts:PHONE_MISMATCH"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_phone_contact_id"
                && diagnostic.subject == "phone_contacts: PHONE_PADDED"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_permanent_phone_contact"
                && diagnostic.subject == "PHONE MOM"
                && diagnostic.message.contains("PHONE MOM")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_permanent_phone_contact"
                && diagnostic.subject == "phone_mom"
                && diagnostic.message.contains("phone_mom")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_permanent_phone_contact"
                && diagnostic.subject == "PHONE MOM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            (diagnostic.code == "unknown_phone_contact_map"
                || diagnostic.code == "empty_phone_contact_map"
                || diagnostic.code == "invalid_phone_contact_lines"
                || diagnostic.code == "phone_contact_primary_label_mismatch")
                && diagnostic.subject == "phone_contacts:PHONE_ELM"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_script_object_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.objects = vec![
            test_object("START_RIVAL", "EVENT_START_RIVAL", 1, 1),
            test_object("START_ALWAYS_VISIBLE", "0", 2, 1),
        ];
        module.script_object_commands = vec![
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("start_rival".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "LowercaseScript".to_string(),
                command_index: 4,
            },
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("START RIVAL".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "MalformedObjectScript".to_string(),
                command_index: 5,
            },
            ScriptObjectCommand {
                command: "appear".to_string(),
                object_id: Some("START_ALWAYS_VISIBLE".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "UnhideableScript".to_string(),
                command_index: 7,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("MissingMovement".to_string()),
                emote: None,
                duration: None,
                source_script: "MovementScript".to_string(),
                command_index: 9,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("Missing Movement".to_string()),
                emote: None,
                duration: None,
                source_script: "MalformedMovementScript".to_string(),
                command_index: 10,
            },
            ScriptObjectCommand {
                command: "follow".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: Some("start_player".to_string()),
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "FollowScript".to_string(),
                command_index: 11,
            },
            ScriptObjectCommand {
                command: "follow".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: Some("START PLAYER".to_string()),
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "MalformedFollowScript".to_string(),
                command_index: 12,
            },
            ScriptObjectCommand {
                command: "spinobject".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "UnknownCommandScript".to_string(),
                command_index: 13,
            },
            ScriptObjectCommand {
                command: "MoveObject".to_string(),
                object_id: Some("START_RIVAL".to_string()),
                target_object_id: None,
                x: Some(1),
                y: Some(1),
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "MalformedCommandScript".to_string(),
                command_index: 14,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_object_id"
                && diagnostic.subject == "Start:LowercaseScript:4"
                && diagnostic.message.contains("start_rival")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_object_unhideable"
                && diagnostic.subject == "Start:UnhideableScript:7"
                && diagnostic.message.contains("START_ALWAYS_VISIBLE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_object_id"
                && diagnostic.subject == "Start:MalformedObjectScript:5"
                && diagnostic.message.contains("START RIVAL")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_movement"
                && diagnostic.subject == "Start:MovementScript:9"
                && diagnostic.message.contains("MissingMovement")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_movement"
                && diagnostic.subject == "Start:MalformedMovementScript:10"
                && diagnostic.message.contains("Missing Movement")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_object_id"
                && diagnostic.subject == "Start:FollowScript:11"
                && diagnostic.message.contains("start_player")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_target_object_id"
                && diagnostic.subject == "Start:MalformedFollowScript:12"
                && diagnostic.message.contains("START PLAYER")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_object_command"
                && diagnostic.subject == "Start:UnknownCommandScript:13"
                && diagnostic.message.contains("spinobject")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_object_command"
                && diagnostic.subject == "Start:MalformedCommandScript:14"
                && diagnostic.message.contains("MoveObject")
        }));
    }

    #[test]
    fn verifier_rejects_object_events_that_reference_missing_scripts() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "ObjectScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        let mut exact_object = test_object("START_EXACT", "EVENT_START_EXACT", 1, 1);
        exact_object.script = "ObjectScript".to_string();
        let mut lowercase_object = test_object("START_LOWERCASE", "EVENT_START_LOWERCASE", 2, 1);
        lowercase_object.script = "objectscript".to_string();
        let mut missing_object = test_object("START_MISSING", "EVENT_START_MISSING", 3, 1);
        missing_object.script = "MissingObjectScript".to_string();
        let mut malformed_object = test_object("START_MALFORMED", "EVENT_START_MALFORMED", 4, 1);
        malformed_object.script = "Object Script".to_string();
        let mut sentinel_object = test_object("START_SENTINEL", "-1", 5, 1);
        sentinel_object.script = "-1".to_string();
        let mut asm_handler_object = test_object("START_OBJECT_EVENT", "-1", 6, 1);
        asm_handler_object.script = "ObjectEvent".to_string();
        module.objects = vec![
            exact_object,
            lowercase_object,
            missing_object,
            malformed_object,
            sentinel_object,
            asm_handler_object,
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_object_event_script"
                && diagnostic.subject == "Start:START_LOWERCASE"
                && diagnostic.message.contains("objectscript")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_object_event_script"
                && diagnostic.subject == "Start:START_MISSING"
                && diagnostic.message.contains("MissingObjectScript")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_object_event_script"
                && diagnostic.subject == "Start:START_MALFORMED"
                && diagnostic.message.contains("Object Script")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_object_event_script"
                && diagnostic.subject == "Start:START_MALFORMED"
        }));
        for accepted in [
            "Start:START_EXACT",
            "Start:START_SENTINEL",
            "Start:START_OBJECT_EVENT",
        ] {
            assert!(
                !report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "unknown_object_event_script"
                        && diagnostic.subject == accepted
                }),
                "accepted object script was rejected: {accepted}"
            );
        }
    }

    #[test]
    fn verifier_rejects_duplicate_or_malformed_object_identifiers() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 5;
        module.attributes.height = 2;
        module.blocks = vec![1; 10];
        module.scripts.insert(
            "ObjectScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        let duplicate_one = test_object("START_DUPLICATE", "EVENT_START_DUPLICATE_ONE", 1, 1);
        let duplicate_two = test_object("START_DUPLICATE", "EVENT_START_DUPLICATE_TWO", 2, 1);
        let mut malformed = test_object("START OBJECT", "EVENT_START_MALFORMED", 3, 1);
        malformed.script = "ObjectScript".to_string();
        let mut anonymous = test_object("START_ANON", "EVENT_START_ANON", 4, 1);
        anonymous.object_identifier = None;
        let duplicate_position =
            test_object("START_DUPLICATE_POSITION", "EVENT_START_POSITION", 4, 1);
        module.objects = vec![
            duplicate_one,
            duplicate_two,
            malformed,
            anonymous,
            duplicate_position,
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_object_identifier"
                && diagnostic.subject == "Start:START_DUPLICATE"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_object_identifier"
                && diagnostic.subject == "Start:START OBJECT"
                && diagnostic.message.contains("START OBJECT")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == VerificationSeverity::Error
                && diagnostic.code == "duplicate_object_position"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_object_identifier" && diagnostic.subject == "Start:<none>"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_movement_steps_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_movements = vec![ScriptMovement {
            label: "BadMovement".to_string(),
            source_script: Some("MovementScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("north".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "spin_forever".to_string(),
                    direction: None,
                    duration: None,
                    index: 3,
                },
            ],
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for (code, index) in [
            ("missing_script_direction", 0),
            ("unknown_script_direction", 1),
            ("script_movement_unexpected_direction", 2),
            ("unsupported_script_movement_command", 3),
        ] {
            let subject = format!("Start:BadMovement:{index}");
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unterminated_script_movement"
                && diagnostic.subject == "Start:BadMovement:MovementScript"
        }));
    }

    #[test]
    fn verifier_rejects_duplicate_script_movements_for_exact_source() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_movements = vec![
            ScriptMovement {
                label: "SharedMovement".to_string(),
                source_script: Some("ObjectScript".to_string()),
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
            ScriptMovement {
                label: "SharedMovement".to_string(),
                source_script: Some("ObjectScript".to_string()),
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
            ScriptMovement {
                label: "SharedMovement".to_string(),
                source_script: Some("OtherScript".to_string()),
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
            ScriptMovement {
                label: "GlobalMovement".to_string(),
                source_script: None,
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
            ScriptMovement {
                label: "GlobalMovement".to_string(),
                source_script: Some("ObjectScript".to_string()),
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
        ];
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("RUNTIME_NPC".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("GlobalMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "ObjectScript".to_string(),
            command_index: 7,
        }];
        module.scripts.insert(
            "ObjectScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.objects = vec![test_object("RUNTIME_NPC", "-1", 1, 1)];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        let duplicate_movements = report
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.code == "duplicate_script_movement"
                    && diagnostic.subject == "Start:SharedMovement:ObjectScript"
                    && diagnostic.severity == VerificationSeverity::Error
            })
            .count();
        assert_eq!(duplicate_movements, 1, "{:?}", report.diagnostics);
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_script_movement"
                && diagnostic.subject == "Start:SharedMovement:OtherScript"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "ambiguous_script_movement"
                && diagnostic.subject == "Start:ObjectScript:7"
        }));
    }

    #[test]
    fn verifier_rejects_moveobject_destinations_outside_runtime_map_bounds() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.objects = vec![test_object("RUNTIME_NPC", "-1", 1, 1)];
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "moveobject".to_string(),
            object_id: Some("RUNTIME_NPC".to_string()),
            target_object_id: None,
            x: Some(4),
            y: Some(0),
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "ObjectScript".to_string(),
            command_index: 7,
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_moveobject_destination_out_of_bounds"
                && diagnostic.subject == "Start:ObjectScript:7"
                && diagnostic
                    .message
                    .contains("moveobject raw coordinate (4, 0)")
                && diagnostic.message.contains("outside map bounds 2x2")
        }));
    }

    #[test]
    fn verifier_allows_applymovement_endpoints_outside_runtime_map_bounds() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1; 4];
        module.objects = vec![test_object("RUNTIME_NPC", "-1", 3, 0)];
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("RUNTIME_NPC".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("OutOfBoundsMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "ObjectScript".to_string(),
            command_index: 8,
        }];
        module.script_movements = vec![ScriptMovement {
            label: "OutOfBoundsMovement".to_string(),
            source_script: Some("ObjectScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
            ],
        }];
        module.scripts.insert(
            "ObjectScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_applymovement_endpoint_out_of_bounds"
                && diagnostic.subject == "Start:ObjectScript:8"
        }));
    }

    #[test]
    fn verifier_rejects_applymovement_endpoint_coordinate_overflow() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.objects = vec![test_object("RUNTIME_NPC", "-1", 32767, 0)];
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("RUNTIME_NPC".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("OverflowMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "ObjectScript".to_string(),
            command_index: 9,
        }];
        module.script_movements = vec![ScriptMovement {
            label: "OverflowMovement".to_string(),
            source_script: Some("ObjectScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
            ],
        }];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_applymovement_endpoint_overflow"
                && diagnostic.subject == "Start:ObjectScript:9"
                && diagnostic
                    .message
                    .contains("object 'RUNTIME_NPC' overflows supported runtime coordinates")
                && diagnostic.message.contains("(32767, 0)")
        }));
    }

    #[test]
    fn runtime_script_movement_requires_exact_source_bound_rows() {
        let mut start = test_map_module("Start", "START_MAP", None);
        start.script_movements = vec![
            ScriptMovement {
                label: "LocalMovement".to_string(),
                source_script: Some("StartScript".to_string()),
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
            ScriptMovement {
                label: "GlobalMovement".to_string(),
                source_script: Some("StartScript".to_string()),
                steps: vec![ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                }],
            },
        ];
        let mut tower = test_map_module("Tower", "TOWER_MAP", None);
        tower.script_movements = vec![ScriptMovement {
            label: "TowerOnlyMovement".to_string(),
            source_script: None,
            steps: vec![ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        }];
        let data = GameDataSet {
            maps: [
                ("Start".to_string(), start),
                ("Tower".to_string(), tower.clone()),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let local = data
            .script_movement("Start", "StartScript", "LocalMovement")
            .expect("local movement");
        assert_eq!(local.label, "LocalMovement");
        let global = data
            .script_movement("Start", "StartScript", "GlobalMovement")
            .expect("exact source-bound movement");
        assert_eq!(global.label, "GlobalMovement");
        assert!(
            data.script_movement("Start", "StartScript", "globalmovement")
                .is_err()
        );
        assert!(
            data.script_movement("Start", "StartScript", "TowerOnlyMovement")
                .is_err()
        );

        let mut verifier_start = data.maps.get("Start").expect("start").clone();
        verifier_start.objects = vec![test_object("RUNTIME_NPC", "-1", 1, 1)];
        verifier_start.script_object_commands = vec![ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("RUNTIME_NPC".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("GlobalMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "StartScript".to_string(),
            command_index: 3,
        }];
        let verifier_data = GameDataSet {
            maps: [
                ("Start".to_string(), verifier_start),
                (
                    "Tower".to_string(),
                    data.maps.get("Tower").expect("tower").clone(),
                ),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &verifier_data,
            &PlayabilityRules::default(),
        );
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_movement"
                && diagnostic.subject == "Start:StartScript:3"
        }));

        let wrong_source = data
            .script_movement("Start", "OtherScript", "GlobalMovement")
            .expect_err("wrong source script must not select a movement");
        assert!(
            format!("{wrong_source:#}")
                .contains("map Start has no exact movement GlobalMovement for OtherScript"),
            "{wrong_source:#}"
        );
    }

    #[test]
    fn verifier_rejects_duplicate_script_command_positions() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_map_commands = vec![
            ScriptMapCommand {
                command: "loadmap".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 3,
            },
            ScriptMapCommand {
                command: "loadmap".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: None,
                source_script: "WarpScript".to_string(),
                command_index: 3,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_script_command_position"
                && diagnostic.subject == "Start:script_map_commands:WarpScript:3"
        }));
    }

    #[test]
    fn verifier_accepts_temporary_script_objects_and_last_talked_operand() {
        let mut module = test_map_module("CeladonGameCorner", "CELADON_GAME_CORNER", None);
        module.objects = vec![test_object("CELADONGAMECORNER_FISHER", "-1", 1, 1)];
        module.script_object_commands = vec![
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("CELADONGAMECORNER_FISHER".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "TemporaryScript".to_string(),
                command_index: 1,
            },
            ScriptObjectCommand {
                command: "turnobject".to_string(),
                object_id: Some("LAST_TALKED".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: Some("LEFT".to_string()),
                movement: None,
                emote: None,
                duration: None,
                source_script: "LastTalkedScript".to_string(),
                command_index: 2,
            },
        ];
        let data = GameDataSet {
            maps: [("CeladonGameCorner".to_string(), module)]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "script_object_unhideable"
                    || diagnostic.code == "unknown_script_object_id"
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn verifier_accepts_script_economy_commands_with_exact_pack_constants() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_economy_commands = vec![
            ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec![
                    "ROUTE43GATE_TOLL".to_string(),
                    "-".to_string(),
                    "1".to_string(),
                ],
                source_script: "TollScript".to_string(),
                command_index: 2,
            },
            ScriptEconomyCommand {
                command: "takecoins".to_string(),
                account: None,
                amount_tokens: vec!["MAX_COINS".to_string(), "-".to_string(), "1".to_string()],
                source_script: "PrizeScript".to_string(),
                command_index: 8,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            currency_constants: CurrencyCatalog(
                [
                    ("ROUTE43GATE_TOLL".to_string(), 1_000),
                    ("MAX_COINS".to_string(), 9_999),
                ]
                .into_iter()
                .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(
            !report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "unresolved_script_currency_amount"
                    || diagnostic.code == "unknown_script_money_account"
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn modpack_tmhm_items_require_explicit_index_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: item_payload(vec![tm]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        let error = data
            .apply_modpack(&manifest)
            .expect_err("missing tmhm index rejected");

        assert!(
            error
                .to_string()
                .contains("TM/HM item 'TM_MUD_SLAP' must declare explicit tmhm_index"),
            "{error}"
        );
    }

    #[test]
    fn modpack_tmhm_items_reject_zero_index_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = Some(0);
        tm.tmhm_move = Some("MUD_SLAP".to_string());
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: item_payload(vec![tm]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        let error = data
            .apply_modpack(&manifest)
            .expect_err("zero tmhm index rejected");

        assert!(
            error
                .to_string()
                .contains("TM/HM item 'TM_MUD_SLAP' must declare positive tmhm_index, found 0"),
            "{error}"
        );
    }

    #[test]
    fn modpack_tmhm_items_reject_whitespace_move_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = Some(30);
        tm.tmhm_move = Some(" MUD_SLAP".to_string());
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: item_payload(vec![tm]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        let error = data
            .apply_modpack(&manifest)
            .expect_err("whitespace tmhm move rejected");

        assert!(
            error.to_string().contains(
                "TM/HM item 'TM_MUD_SLAP' must declare exact tmhm_move, found ' MUD_SLAP'"
            ),
            "{error}"
        );
    }

