#[test]
fn modpack_items_reject_menu_usability_contradictions() {
    let mut item = test_item("MOD_MENU_ITEM");
    item.field_menu = "ITEMMENU_NOUSE".to_string();
    item.field_usable = true;
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
        .expect_err("field menu usability contradiction rejected");

    assert!(
        error.to_string().contains(
            "item 'MOD_MENU_ITEM' field_usable true contradicts field_menu 'ITEMMENU_NOUSE'"
        ),
        "{error}"
    );
}

#[test]
fn modpack_tmhm_items_require_explicit_move_data() {
    let mut tm = test_item("TM_MUD_SLAP");
    tm.pocket = item_pocket("TM_HM");
    tm.tmhm_index = Some(30);
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
        .expect_err("missing tmhm move rejected");

    assert!(
        error
            .to_string()
            .contains("TM/HM item 'TM_MUD_SLAP' must declare explicit tmhm_move"),
        "{error}"
    );
}

#[test]
fn modpack_tmhm_items_require_exact_move_references() {
    let mut tm = test_item("TM_MUD_SLAP");
    tm.pocket = item_pocket("TM_HM");
    tm.tmhm_index = Some(30);
    tm.tmhm_move = Some("mud_slap".to_string());
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            moves: move_payload(vec![test_move("MUD_SLAP")]),
            items: item_payload(vec![tm]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("unknown tmhm move rejected");

    assert!(
        error
            .to_string()
            .contains("TM/HM item 'TM_MUD_SLAP' references missing move 'mud_slap'"),
        "{error}"
    );
}

#[test]
fn modpack_moves_reject_malformed_ids_without_effect_enum_restriction() {
    let mut move_data = test_move("AETHER_PULSE");
    move_data.move_type = pokemon_type(" AETHER");
    move_data.effect = "MODDED_EFFECT".to_string();
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            moves: move_payload(vec![move_data]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    let error = data
        .apply_modpack(&manifest)
        .expect_err("malformed move type rejected")
        .to_string();

    assert!(
        error.contains("move 'AETHER_PULSE' has invalid type ' AETHER'"),
        "{error}"
    );
    assert!(data.moves.is_empty());
}

#[test]
fn modpack_tmhm_items_accept_exact_move_references() {
    let mut tm = test_item("TM_MUD_SLAP");
    tm.pocket = item_pocket("TM_HM");
    tm.tmhm_index = Some(30);
    tm.tmhm_move = Some("MUD_SLAP".to_string());
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            moves: move_payload(vec![test_move("MUD_SLAP")]),
            items: item_payload(vec![tm]),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    data.apply_modpack(&manifest)
        .expect("exact tmhm move accepted");

    assert!(data.items.contains_key("TM_MUD_SLAP"));
}

#[test]
fn modpack_symbolic_tm_grants_validate_against_exact_item_data() {
    let mut tm = test_item("TM_MUD_SLAP");
    tm.pocket = item_pocket("TM_HM");
    tm.tmhm_index = Some(30);
    tm.tmhm_move = Some("MUD_SLAP".to_string());
    let mut module = test_map_module("VioletGym", "VIOLET_GYM", None);
    module.script_item_grants = vec![ScriptItemGrant {
        command: "verbosegiveitem".to_string(),
        item_id: "TM_MUD_SLAP".to_string(),
        quantity: 1,
        source_script: "VioletGymFalknerScript".to_string(),
        command_index: 27,
        verbose: true,
    }];
    let data = GameDataSet {
        maps: [("VioletGym".to_string(), module)].into_iter().collect(),
        items: [("TM_MUD_SLAP".to_string(), tm)].into_iter().collect(),
        ..GameDataSet::default()
    };

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );

    assert!(
        !report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_item_grant_item"
                || diagnostic.code == "unindexed_tmhm_item"
        }),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn modpack_overlay_rejects_duplicate_currency_constants_by_exact_id() {
    let mut data = GameDataSet {
        currency_constants: CurrencyCatalog(
            [("ROUTE43GATE_TOLL".to_string(), 500)]
                .into_iter()
                .collect(),
        ),
        ..GameDataSet::default()
    };
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            currency_constants: CurrencyCatalog(
                [
                    ("ROUTE43GATE_TOLL".to_string(), 1_000),
                    ("route43gate_toll".to_string(), 1),
                ]
                .into_iter()
                .collect(),
            ),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };

    let error = data
        .apply_modpack(&manifest)
        .expect_err("duplicate currency constant manifest must not overwrite");

    assert!(
        format!("{error:#}").contains("duplicate currency constant 'ROUTE43GATE_TOLL'"),
        "{error:#}"
    );
    assert_eq!(data.currency_constants.get("route43gate_toll"), None);
}

#[test]
fn modpack_payload_replaces_fishing_catalog_as_definitive_data() {
    let catalog = FishingCatalog {
        groups: [(
            "FISHGROUP_NEW".to_string(),
            crystal_core::world::fishing::FishingGroup {
                bite_threshold: crystal_core::world::fishing::threshold(50, true),
                rod_tables: BTreeMap::new(),
            },
        )]
        .into_iter()
        .collect(),
        time_groups: BTreeMap::new(),
        swarm_rules: BTreeMap::new(),
        rod_items: BTreeMap::new(),
    };
    let manifest = ModpackManifest {
        payload: ModpackPayload {
            fishing: catalog.clone(),
            ..ModpackPayload::default()
        },
        ..ModpackManifest::default()
    };
    let mut data = GameDataSet::default();

    data.apply_modpack(&manifest)
        .expect("apply fishing catalog");

    assert_eq!(data.fishing, catalog);
}

#[test]
fn verifier_rejects_unknown_fishing_facts_without_case_coercion() {
    let mut known_species = species();
    known_species.id = "MAGIKARP".to_string();
    known_species.tmhm_learnset.clear();
    let data = GameDataSet {
        pokemon: [("MAGIKARP".to_string(), known_species)]
            .into_iter()
            .collect(),
        map_attributes: [
            (
                "Lake".to_string(),
                MapAttributes {
                    tileset_name: "johto".to_string(),
                    border_block: 0,
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
                    fishing_group: Some("fishgroup_lake".to_string()),
                    map_constant: Some("LAKE".to_string()),
                    map_group_constant: None,
                    blocks_label: None,
                    map_scripts_label: None,
                    map_events_label: None,
                    connection_flags: None,
                },
            ),
            (
                "BadLake".to_string(),
                MapAttributes {
                    tileset_name: "johto".to_string(),
                    border_block: 0,
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
                    fishing_group: Some(" fishgroup_lake".to_string()),
                    map_constant: Some("BAD_LAKE".to_string()),
                    map_group_constant: None,
                    blocks_label: None,
                    map_scripts_label: None,
                    map_events_label: None,
                    connection_flags: None,
                },
            ),
        ]
        .into_iter()
        .collect(),
        fishing: FishingCatalog {
            groups: [
                (
                    "FISHGROUP_LAKE".to_string(),
                    crystal_core::world::fishing::FishingGroup {
                        bite_threshold: 128,
                        rod_tables: [
                            (
                                " GOOD_ROD".to_string(),
                                crystal_core::world::fishing::RodTable {
                                    slots: vec![crystal_core::world::fishing::FishingSlot {
                                        threshold: 255,
                                        species: Some("MAGIKARP".to_string()),
                                        level: 10,
                                        time_group: None,
                                    }],
                                },
                            ),
                            (
                                "good_rod".to_string(),
                                crystal_core::world::fishing::RodTable {
                                    slots: vec![
                                        crystal_core::world::fishing::FishingSlot {
                                            threshold: 255,
                                            species: Some(" magikarp".to_string()),
                                            level: 10,
                                            time_group: None,
                                        },
                                        crystal_core::world::fishing::FishingSlot {
                                            threshold: 255,
                                            species: None,
                                            level: 0,
                                            time_group: Some("TIME_GROUP_0".to_string()),
                                        },
                                    ],
                                },
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    },
                ),
                (
                    " FISHGROUP_BAD".to_string(),
                    crystal_core::world::fishing::FishingGroup {
                        bite_threshold: 128,
                        rod_tables: [(
                            crystal_core::world::fishing::ROD_OLD.to_string(),
                            crystal_core::world::fishing::RodTable { slots: Vec::new() },
                        )]
                        .into_iter()
                        .collect(),
                    },
                ),
                (
                    "FISHGROUP_TABLE_BAD".to_string(),
                    crystal_core::world::fishing::FishingGroup {
                        bite_threshold: 128,
                        rod_tables: [(
                            crystal_core::world::fishing::ROD_OLD.to_string(),
                            crystal_core::world::fishing::RodTable {
                                slots: vec![
                                    crystal_core::world::fishing::FishingSlot {
                                        threshold: 0,
                                        species: Some("MAGIKARP".to_string()),
                                        level: 5,
                                        time_group: None,
                                    },
                                    crystal_core::world::fishing::FishingSlot {
                                        threshold: 10,
                                        species: Some("MAGIKARP".to_string()),
                                        level: 0,
                                        time_group: None,
                                    },
                                    crystal_core::world::fishing::FishingSlot {
                                        threshold: 5,
                                        species: None,
                                        level: 0,
                                        time_group: None,
                                    },
                                ],
                            },
                        )]
                        .into_iter()
                        .collect(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            time_groups: [(
                "TIME_GROUP_0".to_string(),
                crystal_core::world::fishing::TimeFishEntry {
                    day_species: " MAGIKARP".to_string(),
                    day_level: 10,
                    night_species: "staryu".to_string(),
                    night_level: 10,
                },
            )]
            .into_iter()
            .collect(),
            swarm_rules: [
                (
                    "SWARM_RULE_0".to_string(),
                    crystal_core::world::fishing::FishingSwarmRule {
                        daily_flag_bit: 8,
                        swarm: 1,
                        base_group: "fishgroup_lake".to_string(),
                        swarm_group: "FISHGROUP_MISSING".to_string(),
                    },
                ),
                (
                    "SWARM_RULE_1".to_string(),
                    crystal_core::world::fishing::FishingSwarmRule {
                        daily_flag_bit: 0,
                        swarm: 1,
                        base_group: "FISHGROUP_MISSING_BASE".to_string(),
                        swarm_group: " FISHGROUP_BAD_SWARM".to_string(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
            rod_items: [
                ("GOOD_ROD".to_string(), "good_rod".to_string()),
                (
                    " GOOD_ROD".to_string(),
                    crystal_core::world::fishing::ROD_GOOD.to_string(),
                ),
                ("MISSING_ROD_2".to_string(), "GOOD ROD".to_string()),
            ]
            .into_iter()
            .collect(),
        },
        ..GameDataSet::default()
    };

    let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

    for expected in [
        "invalid_fishing_rod_item_id",
        "unknown_map_fishing_group",
        "invalid_map_fishing_group",
        "invalid_fishing_group_id",
        "invalid_fishing_rod",
        "unknown_fishing_rod",
        "empty_fishing_rod_table",
        "invalid_fishing_slot_threshold",
        "invalid_fishing_slot_level",
        "unordered_fishing_slot_threshold",
        "missing_fishing_slot_species",
        "incomplete_fishing_rod_table",
        "invalid_fishing_species",
        "unknown_fishing_species",
        "invalid_fishing_time_group_species",
        "unknown_fishing_time_group_species",
        "invalid_fishing_swarm_flag_bit",
        "unknown_fishing_swarm_base_group",
        "invalid_fishing_swarm_group",
        "unknown_fishing_swarm_group",
        "invalid_fishing_rod_item_rod",
        "unknown_fishing_rod_item_rod",
        "unknown_fishing_rod_item_id",
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
}

#[test]
fn verifier_rejects_referenced_fishing_group_without_catalog() {
    let data = GameDataSet {
        map_attributes: [(
            "Lake".to_string(),
            MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 0,
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
                fishing_group: Some("FISHGROUP_LAKE".to_string()),
                map_constant: Some("LAKE".to_string()),
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
        )]
        .into_iter()
        .collect(),
        fishing: FishingCatalog::default(),
        ..GameDataSet::default()
    };

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_fishing_catalog" && diagnostic.subject == "Lake"
    }));
}

#[test]
fn verifier_rejects_malformed_topology_targets_without_unknown_cascade() {
    let mut start = test_map_module("Start", "START_MAP", Some("MissingTarget"));
    start.attributes.connections.push(MapConnection {
        direction: "east".to_string(),
        target_map: "Missing Target".to_string(),
        offset: 0,
    });
    start.attributes.connections.push(MapConnection {
        direction: "east".to_string(),
        target_map: "Start".to_string(),
        offset: 0,
    });
    start.attributes.connections.push(MapConnection {
        direction: "East".to_string(),
        target_map: "Start".to_string(),
        offset: 0,
    });
    start.events.warps = vec![
        WarpEvent {
            index: 1,
            x: 0,
            y: 0,
            target_map_constant: "MISSING_MAP".to_string(),
            target_map: "MISSING_MAP".to_string(),
            target_warp_id: 1,
        },
        WarpEvent {
            index: 2,
            x: 0,
            y: 0,
            target_map_constant: "MISSING MAP".to_string(),
            target_map: "MISSING MAP".to_string(),
            target_warp_id: 1,
        },
        WarpEvent {
            index: 3,
            x: 0,
            y: 0,
            target_map_constant: "START_MAP".to_string(),
            target_map: "START MAP".to_string(),
            target_warp_id: 1,
        },
        WarpEvent {
            index: 4,
            x: 0,
            y: 0,
            target_map_constant: "START_MAP".to_string(),
            target_map: "OTHER_MAP".to_string(),
            target_warp_id: 1,
        },
        WarpEvent {
            index: 4,
            x: 0,
            y: 0,
            target_map_constant: "START_MAP".to_string(),
            target_map: "START_MAP".to_string(),
            target_warp_id: 1,
        },
    ];
    let data = GameDataSet {
        maps: [("Start".to_string(), start)].into_iter().collect(),
        ..GameDataSet::default()
    };

    let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_connection_target"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("MissingTarget")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_connection_target"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("Missing Target")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_connection_direction"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("East")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "duplicate_connection_direction"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("east")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_warp_target"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("MISSING_MAP")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_warp_target_map"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("MISSING MAP")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_warp_target_map"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("START MAP")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "warp_target_map_mismatch"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("OTHER_MAP")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "duplicate_warp_index"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("4")
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "duplicate_warp_tile"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("0,0")
    }));
    assert!(!report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_connection_target"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("Missing Target")
    }));
    assert!(!report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unreachable_connection"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("Start")
    }));
    assert!(!report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_warp_target"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("MISSING MAP")
    }));
    assert!(!report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_warp_target"
            && diagnostic.subject == "Start"
            && diagnostic.message.contains("OTHER_MAP")
    }));
}

#[test]
fn verifier_builds_reachability_graph_and_rejects_unsolved_goals() {
    let mut known_species = species();
    known_species.tmhm_learnset.clear();
    let mut start = test_map_module("Start", "START_MAP", Some("Middle"));
    start.attributes.height = 2;
    start.blocks = vec![5, 1];
    let mut middle = test_map_module("Middle", "MIDDLE_MAP", None);
    middle.attributes.height = 2;
    middle.blocks = vec![1, 1];
    let data = GameDataSet {
        pokemon: [(known_species.id.clone(), known_species)]
            .into_iter()
            .collect(),
        moves: [("TACKLE".to_string(), test_move("TACKLE"))]
            .into_iter()
            .collect(),
        maps: [
            ("Start".to_string(), start),
            ("Middle".to_string(), middle),
            (
                "Goal".to_string(),
                test_map_module("Goal", "GOAL_MAP", None),
            ),
        ]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };

    let report = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            start_tiles: Vec::new(),
            goal_maps: vec!["Goal".to_string()],
            require_all_maps_reachable: true,
            require_walkable_maps: false,
            ..PlayabilityRules::default()
        },
    );

    assert_eq!(
        report.graph_edges,
        vec![PlayabilityGraphEdge {
            from: "Start".to_string(),
            to: "Middle".to_string(),
            kind: "connection".to_string(),
        }]
    );
    assert_eq!(
        report.reachable_maps,
        vec!["Middle".to_string(), "Start".to_string()]
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unreachable_goal_map" && diagnostic.subject == "Goal"
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unreachable_map" && diagnostic.subject == "Goal"
    }));
}

#[test]
fn verifier_rejects_connection_that_exists_only_on_blocked_collision() {
    let mut known_species = species();
    known_species.tmhm_learnset.clear();
    let mut blocked_start = test_map_module("Start", "START_MAP", Some("Goal"));
    blocked_start.blocks = vec![5];
    let mut tileset = test_tileset_definition();
    tileset.collision.insert(
        "5".to_string(),
        vec![
            "WALL".to_string(),
            "WALL".to_string(),
            "WALL".to_string(),
            "WALL".to_string(),
        ],
    );
    let data = GameDataSet {
        pokemon: [(known_species.id.clone(), known_species)]
            .into_iter()
            .collect(),
        moves: [("TACKLE".to_string(), test_move("TACKLE"))]
            .into_iter()
            .collect(),
        tilesets: [("johto".to_string(), tileset)].into_iter().collect(),
        maps: [
            ("Start".to_string(), blocked_start),
            (
                "Goal".to_string(),
                test_map_module("Goal", "GOAL_MAP", None),
            ),
        ]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            start_tiles: Vec::new(),
            goal_maps: vec!["Goal".to_string()],
            require_all_maps_reachable: false,
            require_walkable_maps: false,
            ..PlayabilityRules::default()
        },
    );

    assert!(report.graph_edges.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unreachable_connection" && diagnostic.subject == "Start"
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unreachable_goal_map" && diagnostic.subject == "Goal"
    }));
}

#[test]
fn verifier_uses_explicit_start_tiles_instead_of_whole_start_map() {
    let mut known_species = species();
    known_species.tmhm_learnset.clear();
    let mut start = test_map_module("Start", "START_MAP", Some("Goal"));
    start.attributes.width = 3;
    start.attributes.height = 2;
    start.blocks = vec![1, 5, 5, 5, 5, 1];
    let mut goal = test_map_module("Goal", "GOAL_MAP", None);
    goal.attributes.height = 2;
    goal.blocks = vec![1, 1];
    let data = GameDataSet {
        pokemon: [(known_species.id.clone(), known_species)]
            .into_iter()
            .collect(),
        moves: [("TACKLE".to_string(), test_move("TACKLE"))]
            .into_iter()
            .collect(),
        maps: [("Start".to_string(), start), ("Goal".to_string(), goal)]
            .into_iter()
            .collect(),
        ..GameDataSet::default()
    };

    let report = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: Vec::new(),
            start_tiles: vec![PlayabilityStart {
                map: "Start".to_string(),
                tile: TilePosition::new(0, 0),
            }],
            goal_maps: vec!["Goal".to_string()],
            require_all_maps_reachable: false,
            require_walkable_maps: false,
            ..PlayabilityRules::default()
        },
    );

    assert_eq!(report.reachable_maps, vec!["Start".to_string()]);
    assert!(
        report
            .graph_edges
            .iter()
            .any(|edge| { edge.from == "Start" && edge.to == "Goal" && edge.kind == "connection" })
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unreachable_goal_map" && diagnostic.subject == "Goal"
    }));
}

#[test]
fn verifier_rejects_unwalkable_explicit_start_tiles() {
    let mut known_species = species();
    known_species.tmhm_learnset.clear();
    let mut start = test_map_module("Start", "START_MAP", None);
    start.blocks = vec![5];
    let mut goal = test_map_module("Goal", "GOAL_MAP", None);
    goal.attributes.width = 2;
    goal.blocks = vec![1, 1];
    let data = GameDataSet {
        pokemon: [(known_species.id.clone(), known_species)]
            .into_iter()
            .collect(),
        moves: [("TACKLE".to_string(), test_move("TACKLE"))]
            .into_iter()
            .collect(),
        maps: [("Start".to_string(), start)].into_iter().collect(),
        ..GameDataSet::default()
    };

    let report = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: Vec::new(),
            start_tiles: vec![PlayabilityStart {
                map: "Start".to_string(),
                tile: TilePosition::new(0, 0),
            }],
            goal_maps: vec!["Start".to_string()],
            require_all_maps_reachable: false,
            require_walkable_maps: false,
            ..PlayabilityRules::default()
        },
    );

    assert!(report.reachable_maps.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "invalid_start_tile" && diagnostic.subject == "Start"
    }));
}

#[test]
fn verifier_solves_explicit_progression_event_and_item_goals() {
    let mut data = GameDataSet {
        maps: [(
            "Start".to_string(),
            test_map_module("Start", "START_MAP", None),
        )]
        .into_iter()
        .collect(),
        items: [(
            "KEY_CARD".to_string(),
            Item {
                name: "Key Card".to_string(),
                description: "Opens a required gate.".to_string(),
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
                price: 0,
                held_effect: "HELD_NONE".to_string(),
                parameter: 0,
                property: String::new(),
                pocket: item_pocket("KEY_ITEM"),
                field_menu: "ITEMMENU_NOUSE".to_string(),
                field_usable: false,
                battle_menu: "ITEMMENU_NOUSE".to_string(),
                battle_usable: false,
                script_name: "KEY_CARD".to_string(),
                consumable: false,
                tmhm_index: None,
                tmhm_move: None,
            },
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    add_runtime_species_and_move(&mut data);

    let report = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            goal_events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
            goal_items: vec!["KEY_CARD".to_string()],
            progression_rules: vec![ProgressionRule {
                id: "beat_champion".to_string(),
                requires: ProgressionRequirements {
                    maps: vec!["Start".to_string()],
                    ..ProgressionRequirements::default()
                },
                grants: ProgressionGrants {
                    events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
                    items: vec!["KEY_CARD".to_string()],
                    ..ProgressionGrants::default()
                },
            }],
            ..PlayabilityRules::default()
        },
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(report.solvable_maps, vec!["Start".to_string()]);
    assert_eq!(
        report.solvable_events,
        vec!["EVENT_CHAMPION_DEFEATED".to_string()]
    );
    assert_eq!(report.solvable_items, vec!["KEY_CARD".to_string()]);
}

#[test]
fn verifier_solves_events_from_script_granted_loaded_maps() {
    let mut data = GameDataSet {
        maps: [
            (
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            ),
            (
                "ScriptedGoal".to_string(),
                test_map_module("ScriptedGoal", "SCRIPTED_GOAL", None),
            ),
        ]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    add_runtime_species_and_move(&mut data);

    let report = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            goal_events: vec!["EVENT_SCRIPTED_ENDING".to_string()],
            progression_rules: vec![
                ProgressionRule {
                    id: "scripted_warp".to_string(),
                    requires: ProgressionRequirements {
                        maps: vec!["Start".to_string()],
                        ..ProgressionRequirements::default()
                    },
                    grants: ProgressionGrants {
                        maps: vec!["ScriptedGoal".to_string()],
                        ..ProgressionGrants::default()
                    },
                },
                ProgressionRule {
                    id: "scripted_goal_event".to_string(),
                    requires: ProgressionRequirements {
                        maps: vec!["ScriptedGoal".to_string()],
                        ..ProgressionRequirements::default()
                    },
                    grants: ProgressionGrants {
                        events: vec!["EVENT_SCRIPTED_ENDING".to_string()],
                        ..ProgressionGrants::default()
                    },
                },
            ],
            ..PlayabilityRules::default()
        },
    );

    assert!(!report.has_errors(), "{:?}", report.diagnostics);
    assert_eq!(
        report.solvable_maps,
        vec!["ScriptedGoal".to_string(), "Start".to_string()]
    );
    assert_eq!(
        report.solvable_events,
        vec!["EVENT_SCRIPTED_ENDING".to_string()]
    );
}

#[test]
fn verifier_rejects_unsolved_progression_event_goals() {
    let data = GameDataSet {
        maps: [(
            "Start".to_string(),
            test_map_module("Start", "START_MAP", None),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            goal_events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
            progression_rules: vec![ProgressionRule {
                id: "blocked_champion".to_string(),
                requires: ProgressionRequirements {
                    events: vec!["EVENT_NEVER_GRANTED".to_string()],
                    ..ProgressionRequirements::default()
                },
                grants: ProgressionGrants {
                    events: vec!["EVENT_CHAMPION_DEFEATED".to_string()],
                    ..ProgressionGrants::default()
                },
            }],
            ..PlayabilityRules::default()
        },
    );

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unsolved_goal_event" && diagnostic.subject == "EVENT_CHAMPION_DEFEATED"
    }));
}

#[test]
fn verifier_applies_map_access_requirements_to_reachable_maps() {
    let key_card = Item {
        name: "Key Card".to_string(),
        description: "Opens a required gate.".to_string(),
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
        price: 0,
        held_effect: "HELD_NONE".to_string(),
        parameter: 0,
        property: String::new(),
        pocket: item_pocket("KEY_ITEM"),
        field_menu: "ITEMMENU_NOUSE".to_string(),
        field_usable: false,
        battle_menu: "ITEMMENU_NOUSE".to_string(),
        battle_usable: false,
        script_name: "KEY_CARD".to_string(),
        consumable: false,
        tmhm_index: None,
        tmhm_move: None,
    };
    let mut start = test_map_module("Start", "START_MAP", Some("Goal"));
    start.attributes.height = 2;
    start.blocks = vec![5, 1];
    let mut goal = test_map_module("Goal", "GOAL_MAP", None);
    goal.attributes.height = 2;
    goal.blocks = vec![1, 1];
    let mut data = GameDataSet {
        maps: [("Start".to_string(), start), ("Goal".to_string(), goal)]
            .into_iter()
            .collect(),
        items: [("KEY_CARD".to_string(), key_card)].into_iter().collect(),
        ..GameDataSet::default()
    };
    add_runtime_species_and_move(&mut data);

    let blocked = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            goal_maps: vec!["Goal".to_string()],
            map_access: vec![MapAccessRule {
                map: "Goal".to_string(),
                requires: ProgressionRequirements {
                    items: vec!["KEY_CARD".to_string()],
                    ..ProgressionRequirements::default()
                },
            }],
            ..PlayabilityRules::default()
        },
    );

    assert_eq!(
        blocked.reachable_maps,
        vec!["Goal".to_string(), "Start".to_string()]
    );
    assert_eq!(blocked.solvable_maps, vec!["Start".to_string()]);
    assert!(blocked.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unsolved_goal_map" && diagnostic.subject == "Goal"
    }));

    let solved = verify_complete_test_game_data(
        &data,
        &PlayabilityRules {
            start_maps: vec!["Start".to_string()],
            goal_maps: vec!["Goal".to_string()],
            progression_rules: vec![ProgressionRule {
                id: "get_key_card".to_string(),
                requires: ProgressionRequirements {
                    maps: vec!["Start".to_string()],
                    ..ProgressionRequirements::default()
                },
                grants: ProgressionGrants {
                    items: vec!["KEY_CARD".to_string()],
                    ..ProgressionGrants::default()
                },
            }],
            map_access: vec![MapAccessRule {
                map: "Goal".to_string(),
                requires: ProgressionRequirements {
                    items: vec!["KEY_CARD".to_string()],
                    ..ProgressionRequirements::default()
                },
            }],
            ..PlayabilityRules::default()
        },
    );

    assert!(!solved.has_errors(), "{:?}", solved.diagnostics);
    assert_eq!(
        solved.solvable_maps,
        vec!["Goal".to_string(), "Start".to_string()]
    );
}

#[test]
fn verifier_requires_explicit_start_maps_for_solvability_rules() {
    let mut known_species = species();
    known_species.tmhm_learnset.clear();
    let data = GameDataSet {
        pokemon: [(known_species.id.clone(), known_species)]
            .into_iter()
            .collect(),
        moves: [("TACKLE".to_string(), test_move("TACKLE"))]
            .into_iter()
            .collect(),
        maps: [(
            "Goal".to_string(),
            test_map_module("Goal", "GOAL_MAP", None),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules {
            start_maps: Vec::new(),
            start_tiles: Vec::new(),
            goal_maps: vec!["Goal".to_string()],
            require_all_maps_reachable: false,
            require_walkable_maps: false,
            ..PlayabilityRules::default()
        },
    );

    assert!(report.reachable_maps.is_empty());
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_start_map" && diagnostic.subject == "playability"
    }));
}

#[test]
fn compiler_rejects_missing_manifest_dependencies() {
    let manifest = ModpackManifest {
        metadata: ModpackMetadata {
            id: "dependent".to_string(),
            name: "Dependent".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        dependencies: vec!["missing-base".to_string()],
        ..ModpackManifest::default()
    };

    let error = AssetRoot::new(repository_root_for_tests())
        .compile_modpacks(&[manifest], ModpackCompileOptions::default())
        .expect_err("missing dependency should fail compilation");

    assert!(
        error
            .to_string()
            .contains("depends on missing modpack 'missing-base'")
    );
}

#[test]
fn compiler_rejects_manifest_dependency_cycles() {
    let base = ModpackManifest {
        metadata: ModpackMetadata {
            id: "base".to_string(),
            name: "Base".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        dependencies: vec!["overlay".to_string()],
        ..ModpackManifest::default()
    };
    let overlay = ModpackManifest {
        metadata: ModpackMetadata {
            id: "overlay".to_string(),
            name: "Overlay".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        dependencies: vec!["base".to_string()],
        ..ModpackManifest::default()
    };

    let error = AssetRoot::new(repository_root_for_tests())
        .compile_modpacks(&[base, overlay], ModpackCompileOptions::default())
        .expect_err("dependency cycles must fail compilation")
        .to_string();

    assert!(
        error.contains("modpack dependency cycle detected: base -> overlay -> base"),
        "{error}"
    );
}

#[test]
fn manifest_application_order_honors_dependencies_before_priority() {
    let base = ModpackManifest {
        metadata: ModpackMetadata {
            id: "base".to_string(),
            name: "Base".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        priority: 10,
        ..ModpackManifest::default()
    };
    let overlay = ModpackManifest {
        metadata: ModpackMetadata {
            id: "overlay".to_string(),
            name: "Overlay".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
        },
        priority: -10,
        dependencies: vec!["base".to_string()],
        ..ModpackManifest::default()
    };

    let manifests = [overlay, base];
    let ordered_ids: Vec<&str> = ordered_manifests_for_application(&manifests)
        .expect("dependency graph is orderable")
        .into_iter()
        .map(ModpackManifest::id)
        .collect();

    assert_eq!(ordered_ids, vec!["base", "overlay"]);

    let report = ModpackCompileReport {
        manifests: ordered_ids.into_iter().map(str::to_string).collect(),
        ..ModpackCompileReport::default()
    };
    assert_eq!(
        compiled_game_pack_runtime_modpack_id(&report).unwrap(),
        "base+overlay"
    );
}

#[test]
fn compiler_rejects_malformed_manifest_metadata_without_coercion() {
    let cases = [
        (
            "id",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: " dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                ..ModpackManifest::default()
            },
            "metadata.id must be exact ASCII letters, numbers, underscores, hyphens, or dots",
        ),
        (
            "id_with_space",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent pack".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                ..ModpackManifest::default()
            },
            "metadata.id must be exact ASCII letters, numbers, underscores, hyphens, or dots",
        ),
        (
            "id_with_path_separator",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "packs/dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                ..ModpackManifest::default()
            },
            "metadata.id must be exact ASCII letters, numbers, underscores, hyphens, or dots",
        ),
        (
            "name",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent".to_string(),
                    name: " Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                ..ModpackManifest::default()
            },
            "metadata.name must be an exact non-empty value",
        ),
        (
            "version",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: " 1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                ..ModpackManifest::default()
            },
            "metadata.version must be an exact non-empty value",
        ),
        (
            "dependency",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                dependencies: vec![" missing-base".to_string()],
                ..ModpackManifest::default()
            },
            "dependency ' missing-base' must be exact ASCII letters, numbers, underscores, hyphens, or dots",
        ),
        (
            "dependency_with_path_separator",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                dependencies: vec!["base/game".to_string()],
                ..ModpackManifest::default()
            },
            "dependency 'base/game' must be exact ASCII letters, numbers, underscores, hyphens, or dots",
        ),
        (
            "duplicate_dependency",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                dependencies: vec!["base-game".to_string(), "base-game".to_string()],
                ..ModpackManifest::default()
            },
            "declares duplicate dependency 'base-game'",
        ),
        (
            "self_dependency",
            ModpackManifest {
                metadata: ModpackMetadata {
                    id: "dependent".to_string(),
                    name: "Dependent".to_string(),
                    version: "1.0.0".to_string(),
                    author: None,
                    description: None,
                },
                dependencies: vec!["dependent".to_string()],
                ..ModpackManifest::default()
            },
            "modpack 'dependent' must not depend on itself",
        ),
    ];

    for (field, manifest, expected) in cases {
        let error = AssetRoot::new(repository_root_for_tests())
            .compile_modpacks(&[manifest], ModpackCompileOptions::default())
            .unwrap_err();

        assert!(
            error.to_string().contains(expected),
            "unexpected error for {field}: {error}"
        );
    }
}

#[test]
fn base_game_data_loads_existing_exported_wild_encounter_json() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let route29 = data
        .wild_encounters
        .get("Route29")
        .expect("load Route 29 wild encounters");
    let slots = table_for_surface(route29, EncounterSurface::Grass, TimeOfDay::Day)
        .expect("Route 29 day grass table");
    assert_eq!(data.wild_encounters.len(), 114);
    assert_eq!(route29.grass_rates.as_ref().unwrap()["day"], 10);
    assert_eq!(slots.len(), 7);
    assert_eq!(slots[0].species, "PIDGEY");
}

#[test]
fn base_game_script_movements_are_all_supported_by_rust_runtime() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let mut checked_steps = 0usize;
    let mut saw_jump_step = false;
    let mut issues = Vec::new();
    for (map_name, module) in &data.maps {
        for movement in &module.script_movements {
            for step in &movement.steps {
                checked_steps += 1;
                saw_jump_step |= step.command == "jump_step";
                for issue in script_movement_step_issues(step) {
                    issues.push(format!(
                        "{map_name}:{}:{}:{}:{issue:?}",
                        movement.label, step.index, step.command
                    ));
                }
            }
        }
    }

    assert!(
        checked_steps > 0,
        "base game should export script movements"
    );
    assert!(
        saw_jump_step,
        "core Crystal script movements should include authored jump_step commands"
    );
    assert!(
        issues.is_empty(),
        "unsupported or malformed base-game movement steps: {}",
        issues.join(", ")
    );
}

#[test]
fn base_game_data_loads_trainers_into_exact_catalog() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let trainer = data
        .trainers
        .get("FALKNER1")
        .expect("FALKNER1 trainer data");
    assert_eq!(trainer.trainer_id, "FALKNER1");
    assert_eq!(trainer.trainer_class, "FALKNER");
    assert_eq!(trainer.party.len(), 2);
    assert_eq!(trainer.party[0].species, "PIDGEY");
    assert_eq!(trainer.party[1].species, "PIDGEOTTO");

    let start = data
        .trainer_battle_start(
            &crystal_core::state::GameState::default(),
            TrainerBattleRequest::new("FALKNER", "FALKNER1", "EVENT_BEAT_FALKNER"),
        )
        .expect("trainer battle start resolves from pack catalog");

    let TrainerBattleStartStatus::Started(start) = start else {
        panic!("FALKNER1 should not be defeated in default state");
    };
    assert_eq!(start.trainer_class, "FALKNER");
    assert_eq!(start.trainer_id, "FALKNER1");
    assert_eq!(start.enemy_party.len(), 2);
    assert_eq!(start.enemy_pokemon.species.id, "PIDGEY");
    assert_eq!(start.enemy_pokemon.moves[0].name, "TACKLE");
}

#[test]
fn route29_overworld_map_is_assembled_from_core_modular_pack() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let map = data.overworld_map("Route29").expect("assemble Route29");

    assert_eq!(map.name, "Route29");
    assert_eq!((map.width, map.height), (30, 9));
    assert_eq!(map.border_block, 5);
    assert_eq!(map.metatile_ids.len(), 270);
    assert_eq!(map.metatile_ids[0], 5);
    assert_eq!(map.tile_bounds(), (60, 18));
}

#[test]
fn route29_map_module_is_assembled_from_core_modular_pack() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data.map_module("Route29").expect("assemble Route29 module");

    assert_eq!(module.id, "Route29");
    assert_eq!(module.attributes.map_constant.as_deref(), Some("ROUTE_29"));
    assert_eq!(module.blocks.len(), 270);
    assert_eq!(module.objects.len(), 8);
    assert_eq!(module.objects[0].hram_x, -1);
    assert_eq!(
        module.objects[0].object_identifier.as_deref(),
        Some("ROUTE29_COOLTRAINER_M1")
    );
    assert_eq!(module.events.warps.len(), 1);
    assert_eq!(
        module.events.warps[0].target_map_constant,
        "ROUTE_29_ROUTE_46_GATE"
    );
    assert_eq!(module.events.warps[0].target_map, "ROUTE_29_ROUTE_46_GATE");
    assert_eq!(module.events.coord_events.len(), 2);
    assert_eq!(module.events.bg_events.len(), 2);
    assert_eq!(module.events.bg_events[0].event_type, "BGEVENT_READ");
    assert_eq!(module.events.bg_events[0].script, "Route29Sign1");
    assert!(
        module
            .map_event_section_commands
            .iter()
            .any(|command| command.command == "def_warp_events" && command.command_index == 1)
    );
    assert!(module.map_event_section_commands.iter().any(|command| {
        command.command == "warp_event"
            && command.args == vec!["27", "1", "ROUTE_29_ROUTE_46_GATE", "3"]
    }));
    assert!(module.map_event_section_commands.iter().any(|command| {
        command.command == "coord_event"
            && command.args
                == vec![
                    "53",
                    "8",
                    "SCENE_ROUTE29_CATCH_TUTORIAL",
                    "Route29Tutorial1",
                ]
    }));
    assert!(module.map_event_section_commands.iter().any(|command| {
        command.command == "object_event"
            && command.args[2] == "SPRITE_COOLTRAINER_M"
            && command.args[11] == "Route29CooltrainerMScript"
    }));
    assert!(module.scripts.contains_key("Route29_MapScripts"));
    assert!(module.scripts.contains_key("Route29YoungsterScript"));
}

#[test]
fn map_event_parser_rejects_coerced_event_operands() {
    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"def_warp_events","args":[]},
            {"command":"warp_event","args":["27","1","ROUTE_29_ROUTE_46_GATE,","3"]}
        ]),
    )
    .expect_err("warp targets must not be comma-stripped");
    assert!(
        format!("{error:#}").contains(
            "warp_event in Route29 arg 'ROUTE_29_ROUTE_46_GATE,' must be exact and non-empty"
        ),
        "{error:#}"
    );

    let error = parse_map_events(
            "Route29",
            &serde_json::json!([
                {"command":"def_coord_events","args":[]},
                {"command":"coord_event","args":[" 53","8","SCENE_ROUTE29_CATCH_TUTORIAL","Route29Tutorial1"]}
            ]),
        )
        .expect_err("coord event coordinates must not be trim-parsed");
    assert!(
        format!("{error:#}")
            .contains("coord_event in Route29 arg ' 53' must be exact and non-empty"),
        "{error:#}"
    );

    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"def_bg_events","args":[]},
            {"command":"bg_event","args":["1","2","BGEVENT_READ","Route29 Sign"]}
        ]),
    )
    .expect_err("bg event scripts must be exact section args");
    assert!(
        format!("{error:#}")
            .contains("bg_event in Route29 arg 'Route29 Sign' must be exact and non-empty"),
        "{error:#}"
    );

    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"warp_event","args":["27","1","ROUTE_29_ROUTE_46_GATE","3"]}
        ]),
    )
    .expect_err("warp events must not be silently ignored before def_warp_events");
    assert!(
        format!("{error:#}").contains(
            "Malformed warp_event in Route29: command appears outside its declared section."
        ),
        "{error:#}"
    );

    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"def_warp_events","args":["legacy"]},
        ]),
    )
    .expect_err("event section declarations must not carry ignored args");
    assert!(
        format!("{error:#}")
            .contains("Malformed def_warp_events in Route29: expected 0 args, found 1."),
        "{error:#}"
    );

    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"def_warp_events","args":[]},
            {"command":"object_event","args":["1","2","SPRITE_MON"]}
        ]),
    )
    .expect_err("object events must not be accepted outside def_object_events");
    assert!(
        format!("{error:#}").contains(
            "Malformed object_event in Route29: command appears outside its declared section."
        ),
        "{error:#}"
    );

    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"def_bg_events","args":[]},
            {"command":"legacy_bg_event","args":[]}
        ]),
    )
    .expect_err("unknown map event commands must not be silently ignored");
    assert!(
        format!("{error:#}")
            .contains("Malformed map events in Route29: unknown command 'legacy_bg_event'."),
        "{error:#}"
    );

    let error = parse_map_events(
        "Route29",
        &serde_json::json!([
            {"command":"def_bg_events","args":[]},
            {"command":"bg_event","args":["40000","2","BGEVENT_READ","Route29Sign"]}
        ]),
    )
    .expect_err("event coordinates must fit runtime tile coordinates");
    assert!(
        format!("{error:#}")
            .contains("bg_event coordinate (40000, 2) in Route29 overflows runtime tile range"),
        "{error:#}"
    );

    let scripts = BTreeMap::from([(
        "Route29ObjectScript".to_string(),
        serde_json::json!([
            {"command":"moveobject","args":["ROUTE_29_YOUNGSTER","40000","2"]}
        ]),
    )]);
    let error = parse_script_object_commands("Route29", &scripts)
        .expect_err("moveobject coordinates must fit runtime tile coordinates");
    assert!(
        format!("{error:#}")
            .contains("moveobject coordinate (40000, 2) in Route29 overflows runtime tile range"),
        "{error:#}"
    );

    let scripts = BTreeMap::from([(
        "Route29WarpScript".to_string(),
        serde_json::json!([
            {"command":"warp","args":["NEW_BARK_TOWN","40000","2"]}
        ]),
    )]);
    let map_name_by_constant =
        BTreeMap::from([("NEW_BARK_TOWN".to_string(), "NewBarkTown".to_string())]);
    let error = parse_script_map_commands("Route29", &scripts, &map_name_by_constant)
        .expect_err("warp coordinates must fit runtime tile coordinates");
    assert!(
        format!("{error:#}")
            .contains("warp coordinate (40000, 2) in Route29 overflows runtime tile range"),
        "{error:#}"
    );

    let scripts = BTreeMap::from([(
        "Route29WarpScript".to_string(),
        serde_json::json!([
            {"command":"warpfacing","args":["NEW_BARK_TOWN","40000","2","UP"]}
        ]),
    )]);
    let error = parse_script_map_commands("Route29", &scripts, &map_name_by_constant)
        .expect_err("warpfacing coordinates must fit runtime tile coordinates");
    assert!(
        format!("{error:#}")
            .contains("warpfacing coordinate (40000, 2) in Route29 overflows runtime tile range"),
        "{error:#}"
    );
}

#[test]
fn script_command_parsers_reject_padded_args_and_numbers() {
    let scripts = BTreeMap::from([(
        "Route29ItemScript".to_string(),
        serde_json::json!([
            {"command":"giveitem","args":["POTION"," 1"]}
        ]),
    )]);
    let error = parse_script_item_grants("Route29", &scripts)
        .expect_err("script command args must be exact before numeric parsing");
    assert!(
            format!("{error:#}").contains(
                "Malformed giveitem command in Route29ItemScript for Route29: arg 1 must be exact and non-empty."
            ),
            "{error:#}"
        );

    let error = parse_script_u16(" 1").expect_err("numeric tokens must not be trim-parsed");
    assert!(
        format!("{error:#}").contains("numeric token ' 1' must be exact and untrimmed"),
        "{error:#}"
    );

    let error = parse_script_u16("+1").expect_err("numeric tokens must not accept plus aliases");
    assert!(
        format!("{error:#}").contains("numeric token '+1' must not use an explicit plus sign"),
        "{error:#}"
    );

    let error =
        parse_script_u16("0x10").expect_err("numeric tokens must not accept C-style hex aliases");
    assert!(
        format!("{error:#}").contains("parse numeric token '0x10'"),
        "{error:#}"
    );

    assert_eq!(parse_script_u16("$10").expect("ASM hex numeric token"), 16);
    assert_eq!(
        parse_script_u16("%1010").expect("ASM binary numeric token"),
        10
    );
}

#[test]
fn map_module_extracts_trainer_battle_requests_from_exact_script_args() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("BlackthornGym2F")
        .expect("assemble BlackthornGym2F module");
    let request = module
        .trainer_scripts
        .get("TrainerCooltrainermCody")
        .expect("Cody trainer script");

    assert_eq!(request.trainer_class, "COOLTRAINERM");
    assert_eq!(request.trainer_id, "CODY");
    assert_eq!(request.event_flag, "EVENT_BEAT_COOLTRAINERM_CODY");
    assert_eq!(request.seen_text, "CooltrainermCodySeenText");
    assert_eq!(request.win_text, "CooltrainermCodyBeatenText");
    assert_eq!(request.loss_text, "");
    assert_eq!(request.callback, ".Script@TrainerCooltrainermCody");
    assert_eq!(request.source_script, "TrainerCooltrainermCody");

    let start = data
        .trainer_battle_start(&crystal_core::state::GameState::default(), request.clone())
        .expect("trainer battle start resolves from extracted map script");
    let TrainerBattleStartStatus::Started(start) = start else {
        panic!("Cody should not be defeated in default state");
    };
    assert_eq!(start.trainer_class, "COOLTRAINERM");
    assert_eq!(start.trainer_id, "CODY");
    assert_eq!(start.event_flag, "EVENT_BEAT_COOLTRAINERM_CODY");
    assert!(!start.enemy_party.is_empty());
}

#[test]
fn map_module_extracts_scripted_loadtrainer_battle_request_and_source_positions() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("VermilionGym")
        .expect("assemble VermilionGym module");
    let battle = module
        .scripted_trainer_battles
        .iter()
        .find(|battle| battle.source_script == "VermilionGymSurgeScript")
        .expect("Surge scripted trainer battle");

    assert_eq!(battle.request.battle_type, "BATTLETYPE_TRAINER");
    assert_eq!(battle.request.trainer_class, "LT_SURGE");
    assert_eq!(battle.request.trainer_id, "LT_SURGE1");
    assert_eq!(battle.request.event_flag, "");
    assert_eq!(battle.request.win_text, "LtSurgeWinLossText");
    assert_eq!(battle.request.loss_text, "");
    assert!(battle.loadtrainer_command_index < battle.startbattle_command_index);

    let start = data
        .trainer_battle_start(
            &crystal_core::state::GameState::default(),
            battle.request.clone(),
        )
        .expect("scripted trainer battle starts from pack data");
    let TrainerBattleStartStatus::Started(start) = start else {
        panic!("Surge should not be defeated by request event flag");
    };
    assert_eq!(start.trainer_class, "LT_SURGE");
    assert_eq!(start.trainer_id, "LT_SURGE1");
    assert_eq!(start.win_text, "LtSurgeWinLossText");
    assert_eq!(start.enemy_party.len(), 5);
}

#[test]
fn map_module_extracts_scripted_rival_battle_win_loss_text() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("AzaleaTown")
        .expect("assemble AzaleaTown module");
    let battle = module
        .scripted_trainer_battles
        .iter()
        .find(|battle| battle.source_script == "AzaleaTownRivalBattleScript")
        .expect("Azalea rival scripted trainer battle");

    assert_eq!(battle.request.trainer_class, "RIVAL1");
    assert_eq!(battle.request.trainer_id, "RIVAL1_2_TOTODILE");
    assert_eq!(battle.request.win_text, "AzaleaTownRivalWinText");
    assert_eq!(battle.request.loss_text, "AzaleaTownRivalLossText");
}

#[test]
fn map_module_extracts_static_lugia_battle_with_forceitem_metadata() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("WhirlIslandLugiaChamber")
        .expect("assemble Lugia chamber module");
    let battle = module
        .scripted_wild_battles
        .iter()
        .find(|battle| battle.source_script == "Lugia")
        .expect("Lugia scripted wild battle");

    assert_eq!(battle.request.battle_type, "BATTLETYPE_FORCEITEM");
    assert_eq!(battle.request.species, "LUGIA");
    assert_eq!(battle.request.level, 60);

    let mut request = battle.request.clone();
    request.battle_music = data
        .wild_battle_music_for_map_time("WhirlIslandLugiaChamber", TimeOfDay::Day)
        .expect("Lugia battle music from pack data");
    let mut divider = crystal_core::random::ReplayDivider::new([0, 0, 0, 0]);
    let start = data
        .static_wild_battle_start(
            request,
            crystal_core::random::CrystalRandomState::default(),
            &mut divider,
        )
        .expect("Lugia battle starts from pack data");
    assert_eq!(start.battle_type, "BATTLETYPE_FORCEITEM");
    assert_eq!(start.enemy_pokemon.species.id, "LUGIA");
    assert_eq!(start.enemy_pokemon.level, 60);
    assert_eq!(start.enemy_pokemon.original_trainer_name, "WILD");
    let lugia = data.pokemon.get("LUGIA").expect("LUGIA species");
    assert_eq!(
        start.enemy_pokemon.item,
        lugia.item1.clone().or_else(|| lugia.item2.clone())
    );

    let mut state = GameState::default();
    let mut divider = crystal_core::random::ReplayDivider::new([0, 0, 0, 0]);
    data.start_scripted_wild_battle(
        &mut state,
        "WhirlIslandLugiaChamber",
        "WhirlIslandLugiaChamber",
        "Lugia",
        battle.startbattle_command_index,
        &mut divider,
    )
    .expect("direct Lugia start uses only the source battle request");
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_FOUGHT_LUGIA"),
        Ok(false),
        "the preceding source setevent must not be fabricated by startbattle"
    );
}

#[test]
fn map_module_extracts_static_red_gyarados_forceshiny_battle() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("LakeOfRage")
        .expect("assemble LakeOfRage module");
    let battle = module
        .scripted_wild_battles
        .iter()
        .find(|battle| battle.source_script == "RedGyarados")
        .expect("Red Gyarados scripted wild battle");

    assert_eq!(battle.request.battle_type, "BATTLETYPE_FORCESHINY");
    assert_eq!(battle.request.species, "GYARADOS");
    assert_eq!(battle.request.level, 30);

    let mut request = battle.request.clone();
    request.battle_music = data
        .wild_battle_music_for_map_time("LakeOfRage", TimeOfDay::Day)
        .expect("Red Gyarados battle music from pack data");
    let mut divider = crystal_core::random::ReplayDivider::new([0, 0]);
    let start = data
        .static_wild_battle_start(
            request,
            crystal_core::random::CrystalRandomState::default(),
            &mut divider,
        )
        .expect("Red Gyarados battle starts from pack data");
    assert_eq!(start.enemy_pokemon.dvs, Dv::from_non_hp(14, 10, 10, 10));
}

#[test]
fn map_module_extracts_static_snorlax_post_battle_event() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("VermilionCity")
        .expect("assemble VermilionCity module");
    let battle = module
        .scripted_wild_battles
        .iter()
        .find(|battle| battle.source_script == "VermilionSnorlax")
        .expect("Snorlax scripted wild battle");

    assert_eq!(battle.request.battle_type, "BATTLETYPE_FORCEITEM");
    assert_eq!(battle.request.species, "SNORLAX");
    assert_eq!(battle.request.level, 50);
}

#[test]
fn scripted_wild_battle_requires_startbattle_without_silent_drop() {
    let scripts = BTreeMap::from([(
        "IncompleteWildBattle".to_string(),
        serde_json::json!([
            {"command":"loadwildmon","args":["RATTATA","3"]},
            {"command":"reloadmapafterbattle","args":[]}
        ]),
    )]);

    let error = parse_scripted_wild_battles("Route29", &scripts)
        .expect_err("loadwildmon without startbattle must not be silently skipped");

    assert!(
        format!("{error:#}").contains(
            "loadwildmon command in IncompleteWildBattle for Route29 is not followed by startbattle"
        ),
        "{error:#}"
    );
}

#[test]
fn scripted_trainer_battle_follows_exact_post_battle_jump_for_object_event_trainer() {
    let scripts = BTreeMap::from([
        (
            "RivalBattleScript".to_string(),
            serde_json::json!([
                {"command":"showemote","args":["EMOTE_SHOCK","START_RIVAL","15"]},
                {"command":"winlosstext","args":["RivalWinText","RivalLossText"]},
                {"command":"loadtrainer","args":["RIVAL1","RIVAL1_3_TOTODILE"]},
                {"command":"startbattle","args":[]},
                {"command":"reloadmapafterbattle","args":[]},
                {"command":"sjump","args":[".returnfrombattle"]}
            ]),
        ),
        (
            ".returnfrombattle@RivalBattleScript".to_string(),
            serde_json::json!([
                {"command":"setevent","args":["EVENT_RIVAL_BATTLE"]},
                {"command":"end","args":[]}
            ]),
        ),
    ]);

    let battles = parse_scripted_trainer_battles("Start", &scripts)
        .expect("exact post-battle continuation parses");
    assert_eq!(battles.len(), 1);
    assert_eq!(
        serde_json::to_value(&battles[0])
            .expect("serialize slim battle descriptor")
            .as_object()
            .expect("battle descriptor object")
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "loadtrainer_command_index".to_string(),
            "request".to_string(),
            "source_script".to_string(),
            "startbattle_command_index".to_string(),
        ])
    );

    let mut module = test_map_module("Start", "START_MAP", None);
    module.script_flag_commands =
        parse_script_flag_commands("Start", &scripts).expect("script flag commands parse");
    module.script_control_commands =
        parse_script_control_commands("Start", &scripts).expect("script control commands parse");
    module.scripts = scripts;
    module.scripted_trainer_battles = battles;
    let mut rival = test_object("START_RIVAL", "EVENT_RIVAL_BATTLE", 1, 1);
    rival.object_type = "OBJECTTYPE_TRAINER".to_string();
    rival.script = "ObjectEvent".to_string();
    module.objects = vec![rival];
    let data = GameDataSet {
        maps: [("Start".to_string(), module)].into_iter().collect(),
        ..GameDataSet::default()
    };

    let mut state = GameState::default();
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_RIVAL_BATTLE"),
        Ok(false)
    );

    let jump = data
        .apply_script_control_command(&mut state, "Start", "Start", "RivalBattleScript", 5)
        .expect("compiled cursor follows exact post-battle jump");
    let ScriptControlAction::Jump { target_script, .. } = jump else {
        panic!("post-battle sjump did not yield a compiled cursor jump");
    };
    assert_eq!(target_script, ".returnfrombattle@RivalBattleScript");
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_RIVAL_BATTLE"),
        Ok(false)
    );
    let flag_command = data
        .script_flag_command("Start", &target_script, 0)
        .expect("continued cursor resolves exact setevent")
        .clone();
    crystal_core::systems::script_flags::apply_script_flag_mutation(&mut state, flag_command)
        .expect("continued cursor applies exact setevent");
    assert_eq!(
        state.flags.is_event_flag_set("EVENT_RIVAL_BATTLE"),
        Ok(true)
    );

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );
    assert!(
        !report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "trainer_object_missing_battle_request"
                && diagnostic.subject == "Start:START_RIVAL"
        }),
        "exact ObjectEvent trainer bridge was rejected: {:#?}",
        report.diagnostics
    );
}

#[test]
fn scripted_battle_extractors_reject_malformed_loadvar_commands() {
    let scripts = BTreeMap::from([(
        "MalformedTrainerBattle".to_string(),
        serde_json::json!([
            {"command":"loadvar","args":["VAR_BATTLETYPE"]},
            {"command":"loadtrainer","args":["YOUNGSTER","JOEY1"]},
            {"command":"startbattle","args":[]}
        ]),
    )]);
    let error = parse_scripted_trainer_battles("Route30", &scripts)
        .expect_err("scripted trainer loadvar arity must be exact");
    assert!(
            format!("{error:#}").contains(
                "Malformed loadvar command in MalformedTrainerBattle for Route30: expected 2 args, found 1."
            ),
            "{error:#}"
        );

    let scripts = BTreeMap::from([(
        "MalformedWildBattle".to_string(),
        serde_json::json!([
            {"command":"loadvar","args":["VAR_BATTLETYPE","BATTLETYPE_NORMAL","EXTRA"]},
            {"command":"loadwildmon","args":["RATTATA","3"]},
            {"command":"startbattle","args":[]}
        ]),
    )]);
    let error = parse_scripted_wild_battles("Route29", &scripts)
        .expect_err("scripted wild loadvar arity must be exact");
    assert!(
            format!("{error:#}").contains(
                "Malformed loadvar command in MalformedWildBattle for Route29: expected 2 args, found 3."
            ),
            "{error:#}"
        );
}

#[test]
fn map_module_extracts_scene_table_from_generated_map_scripts() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");

    assert_eq!(module.scenes.scenes.len(), 7);
    assert_eq!(module.scenes.scenes[0].scene_id, "SCENE_ELMSLAB_MEET_ELM");
    assert_eq!(
        module.scenes.scenes[0].script_name.as_deref(),
        Some("ElmsLabMeetElmScene")
    );
    assert_eq!(
        module.scenes.scenes[6].scene_id,
        "SCENE_ELMSLAB_AIDE_GIVES_POKE_BALLS"
    );
    assert_eq!(
        module.scenes.scenes[6].script_name.as_deref(),
        Some("ElmsLabNoop5Scene")
    );
    assert!(
        module
            .map_script_section_commands
            .iter()
            .any(|command| command.command == "def_scene_scripts" && command.command_index == 0)
    );
    assert!(module.map_script_section_commands.iter().any(|command| {
        command.command == "scene_script"
            && command.args == vec!["ElmsLabMeetElmScene", "SCENE_ELMSLAB_MEET_ELM"]
    }));
    assert!(module.map_script_section_commands.iter().any(|command| {
        command.command == "scene_const"
            && command.args == vec!["SCENE_ELMSLAB_AIDE_GIVES_POKE_BALLS"]
    }));
    assert!(module.map_script_section_commands.iter().any(|command| {
        command.command == "callback"
            && command.args == vec!["MAPCALLBACK_OBJECTS", "ElmsLabMoveElmCallback"]
    }));
}

#[test]
fn one_arg_scene_scripts_do_not_synthesize_empty_scene_ids() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("AzaleaPokecenter1F")
        .expect("assemble AzaleaPokecenter1F module");

    assert!(module.map_script_section_commands.iter().any(|command| {
        command.command == "scene_script" && command.args == vec!["AzaleaPokecenter1FNoopScene"]
    }));
    assert!(module.scenes.scenes.is_empty());
}

#[test]
fn generated_scene_table_uses_current_cherrygrove_scene_script_binding() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let module = data
        .map_module("CherrygroveCity")
        .expect("assemble CherrygroveCity module");
    let rival_scene = module
        .scenes
        .scenes
        .iter()
        .find(|scene| scene.scene_id == "SCENE_CHERRYGROVECITY_MEET_RIVAL")
        .expect("Cherrygrove rival scene");

    assert_eq!(
        rival_scene.script_name.as_deref(),
        Some("CherrygroveCityNoop2Scene")
    );
}

#[test]
fn map_module_does_not_synthesize_numeric_scene_ids_from_scene_commands() {
    let mut module = test_map_module("TestMap", "TEST_MAP", None);
    module.script_scene_commands = vec![ScriptSceneCommand {
        command: "setscene".to_string(),
        map_id: None,
        scene_id: Some("1".to_string()),
        source_script: "TestMap_MapScripts".to_string(),
        command_index: 3,
    }];
    let data = GameDataSet {
        maps: [("TestMap".to_string(), module)].into_iter().collect(),
        map_attributes: [(
            "TestMap".to_string(),
            MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 0,
                width: 1,
                height: 1,
                connections: Vec::new(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: Some("route".to_string()),
                location: Some("johto".to_string()),
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: Some("TEST_MAP".to_string()),
                map_group_constant: None,
                blocks_label: Some("TestMap_Blocks".to_string()),
                map_scripts_label: Some("TestMap_MapScripts".to_string()),
                map_events_label: Some("TestMap_MapEvents".to_string()),
                connection_flags: None,
            },
        )]
        .into_iter()
        .collect(),
        map_scripts: [
            (
                "TestMap_MapScripts".to_string(),
                serde_json::json!([
                    {"command":"def_scene_scripts","args":[]},
                    {"command":"scene_script","args":["TestMapNoopScene"]},
                    {"command":"def_callbacks","args":[]}
                ]),
            ),
            (
                "TestMap_MapEvents".to_string(),
                serde_json::json!([
                    {"command":"def_warp_events","args":[]},
                    {"command":"def_coord_events","args":[]},
                    {"command":"def_bg_events","args":[]},
                    {"command":"def_object_events","args":[]}
                ]),
            ),
            (
                "OtherMapScript".to_string(),
                serde_json::json!([
                    {"command":"setmapscene","args":["TEST_MAP","2"]}
                ]),
            ),
        ]
        .into_iter()
        .collect(),
        npcs: [("TestMap".to_string(), serde_json::json!([]))]
            .into_iter()
            .collect(),
        map_blocks: [("TestMap_Blocks".to_string(), "AA==".to_string())]
            .into_iter()
            .collect(),
        ..GameDataSet::default()
    };

    let module = data
        .map_module("TestMap")
        .expect("assemble explicit scene fixture");

    assert!(module.scenes.scenes.is_empty());
    assert!(module.script_scene_commands.iter().any(|command| {
        command.command == "setscene"
            && command.scene_id.as_deref() == Some("1")
            && command.source_script == "TestMap_MapScripts"
    }));
}

#[test]
fn map_module_extracts_verbose_script_item_grants_with_exact_ids() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("VioletGym")
        .expect("assemble VioletGym module");

    let grant = module
        .script_item_grants
        .iter()
        .find(|grant| {
            grant.source_script == "VioletGymFalknerScript" && grant.item_id == "TM_MUD_SLAP"
        })
        .expect("Falkner TM grant");

    assert_eq!(grant.quantity, 1);
    assert_eq!(grant.command_index, 27);
    assert!(grant.verbose);

    assert!(!data.items.contains_key("tm_mud_slap"));
}

#[test]
fn map_module_extracts_quantity_script_item_grants() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data.map_module("ElmsLab").expect("assemble ElmsLab module");

    let grant = module
        .script_item_grants
        .iter()
        .find(|grant| grant.source_script == "AideScript_GiveYouBalls")
        .expect("aide Poke Ball grant");

    assert_eq!(grant.item_id, "POKE_BALL");
    assert_eq!(grant.quantity, 5);
    assert_eq!(grant.command_index, 5);
    assert!(!grant.verbose);

    let mut state = GameState::default();
    let outcome =
        grant_script_item(&mut state, &data.items, grant.clone()).expect("grant exact balls");

    assert_eq!(
        outcome,
        ScriptItemGrantOutcome::Granted {
            item_id: "POKE_BALL".to_string(),
            quantity: 5,
            source_script: "AideScript_GiveYouBalls".to_string(),
            command_index: 5,
            verbose: false,
        }
    );
    assert_eq!(state.bag.quantity(&data.items["POKE_BALL"]), 5);
}

#[test]
fn map_module_extracts_checkitem_commands_with_exact_ids() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("GoldenrodMagnetTrainStation")
        .expect("assemble GoldenrodMagnetTrainStation module");

    let access = module
        .script_item_checks
        .iter()
        .find(|access| {
            access.source_script == ".MagnetTrainToSaffron@GoldenrodMagnetTrainStationOfficerScript"
                && access.item_id == "PASS"
        })
        .expect("Magnet Train pass check");

    assert_eq!(access.command_index, 3);

    let mut state = GameState::default();
    let missing = check_script_item(&state, &data.items, access.clone()).expect("check pass");
    assert!(!missing.held);
    state
        .bag
        .add_item(&data.items["PASS"], 1)
        .expect("add pass");
    let held = check_script_item(&state, &data.items, access.clone()).expect("check pass");
    assert!(held.held);
}

#[test]
fn map_module_extracts_takeitem_commands_with_exact_ids() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("CopycatsHouse2F")
        .expect("assemble CopycatsHouse2F module");

    let access = module
        .script_item_takes
        .iter()
        .find(|access| access.source_script == "Copycat" && access.item_id == "LOST_ITEM")
        .expect("Copycat lost item take");

    assert_eq!(access.command_index, 65);

    let mut state = GameState::default();
    state
        .bag
        .add_item(&data.items["LOST_ITEM"], 1)
        .expect("add lost item");
    let outcome =
        take_script_item(&mut state, &data.items, access.clone()).expect("take lost item");

    assert!(outcome.removed);
    assert_eq!(state.bag.quantity(&data.items["LOST_ITEM"]), 0);
}

#[test]
fn map_module_extracts_givepoke_commands_with_exact_metadata() {
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

    assert_eq!(gift.species_id, "CYNDAQUIL");
    assert_eq!(gift.level_token, "5");
    assert_eq!(gift.level, 5);
    assert_eq!(gift.held_item_id.as_deref(), Some("BERRY"));
    assert_eq!(gift.command_index, 22);
    assert!(!gift.egg);
}

#[test]
fn map_module_extracts_custom_gift_metadata_labels() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("Route35GoldenrodGate")
        .expect("assemble Route35GoldenrodGate module");

    let gift = module
        .gift_pokemon_scripts
        .iter()
        .find(|gift| gift.source_script == "RandyScript")
        .expect("Randy Spearow gift");

    assert_eq!(gift.species_id, "SPEAROW");
    assert_eq!(gift.level_token, "10");
    assert_eq!(gift.level, 10);
    assert_eq!(gift.held_item_id, None);
    assert_eq!(gift.nickname_label.as_deref(), Some("GiftSpearowName"));
    assert_eq!(gift.ot_label.as_deref(), Some("GiftSpearowOTName"));
}

#[test]
fn map_module_extracts_giveegg_with_resolved_pack_level() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("VioletPokecenter1F")
        .expect("assemble VioletPokecenter1F module");

    let egg = module
        .gift_pokemon_scripts
        .iter()
        .find(|gift| gift.source_script == "VioletPokecenter1F_ElmsAideScript")
        .expect("Togepi egg gift");

    assert_eq!(egg.species_id, "TOGEPI");
    assert_eq!(egg.level_token, "EGG_LEVEL");
    assert_eq!(egg.level, 5);
    assert!(egg.egg);
}

#[test]
fn gift_level_tokens_resolve_only_from_exact_pack_constants() {
    let mut constants = StoryEventScriptConstants::default();
    assert!(resolve_gift_level_token("Start", "EGG_LEVEL", &constants).is_err());

    constants.global.insert("EGG_LEVEL".to_string(), 5);
    assert_eq!(
        resolve_gift_level_token("Start", "EGG_LEVEL", &constants).expect("global constant"),
        5
    );
    assert!(resolve_gift_level_token("Start", "egg_level", &constants).is_err());

    constants.maps.insert(
        "Start".to_string(),
        [("EGG_LEVEL".to_string(), 6)].into_iter().collect(),
    );
    assert_eq!(
        resolve_gift_level_token("Start", "EGG_LEVEL", &constants).expect("map constant"),
        6
    );
    assert!(resolve_gift_level_token("Start", "0", &constants).is_err());
}

#[test]
fn map_module_extracts_script_flag_commands_and_applies_exact_storage() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("RuinsOfAlphKabutoChamber")
        .expect("assemble RuinsOfAlphKabutoChamber module");

    let solved = module
        .script_flag_commands
        .iter()
        .find(|command| {
            command.source_script == "RuinsOfAlphKabutoChamberPuzzle"
                && command.command == "setevent"
                && command.flag_id == "EVENT_SOLVED_KABUTO_PUZZLE"
        })
        .expect("exact Kabuto puzzle setevent")
        .clone();
    let unlocked = module
        .script_flag_commands
        .iter()
        .find(|command| {
            command.source_script == "RuinsOfAlphKabutoChamberPuzzle"
                && command.command == "setflag"
                && command.flag_id == "ENGINE_UNLOCKED_UNOWNS_A_TO_K"
        })
        .expect("exact Kabuto puzzle setflag")
        .clone();

    assert_eq!(solved.command_index, 7);
    assert_eq!(unlocked.command_index, 8);

    let mut state = GameState::default();
    let solved_outcome =
        apply_script_flag_mutation(&mut state, solved).expect("apply exact event mutation");
    let unlocked_outcome =
        apply_script_flag_mutation(&mut state, unlocked).expect("apply exact engine mutation");

    assert!(!solved_outcome.engine_flag);
    assert!(unlocked_outcome.engine_flag);
    assert_eq!(
        check_script_flag(
            &state,
            ScriptFlagCommand {
                command: "checkevent".to_string(),
                flag_id: "EVENT_SOLVED_KABUTO_PUZZLE".to_string(),
                source_script: "RuinsOfAlphKabutoChamberHiddenDoorsCallback".to_string(),
                command_index: 3,
            },
        )
        .expect("check exact event flag")
        .set,
        true
    );
    assert_eq!(
        check_script_flag(
            &state,
            ScriptFlagCommand {
                command: "checkevent".to_string(),
                flag_id: "event_solved_kabuto_puzzle".to_string(),
                source_script: "RuinsOfAlphKabutoChamberHiddenDoorsCallback".to_string(),
                command_index: 3,
            },
        )
        .expect("case-changed flag remains distinct")
        .set,
        false
    );
}

#[test]
fn map_module_extracts_scene_commands_and_applies_exact_scene_tables() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let route43 = data.map_module("Route43").expect("assemble Route43 module");
    let gate = data
        .map_module("Route43Gate")
        .expect("assemble Route43Gate module");

    let command = route43
        .script_scene_commands
        .iter()
        .find(|command| {
            command.source_script == "Route43CheckIfRocketsScene"
                && command.command == "setmapscene"
                && command.map_id.as_deref() == Some("ROUTE_43_GATE")
                && command.scene_id.as_deref() == Some("SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN")
        })
        .expect("Route43 setmapscene to Route43Gate")
        .clone();
    assert_eq!(command.command_index, 2);

    let target_map = data
        .map_name_for_constant(command.map_id.as_deref().expect("target map id"))
        .expect("resolve exact target map constant");
    let mut state = GameState::default();
    let outcome = apply_script_scene_command(
        &mut state,
        "Route43",
        Some(&target_map),
        &gate.scenes,
        command,
    )
    .expect("apply setmapscene");

    assert_eq!(target_map, "Route43Gate");
    assert_eq!(outcome.scene_id, "SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN");
    assert_eq!(outcome.scene_index, 0);
    assert_eq!(
        state.scenes.map_scenes["Route43Gate"],
        "SCENE_ROUTE43GATE_ROCKET_SHAKEDOWN"
    );

    let gate_setscene = gate
        .script_scene_commands
        .iter()
        .find(|command| {
            command.source_script == "Route43GateRocketTakeoverScript"
                && command.command == "setscene"
                && command.scene_id.as_deref() == Some("SCENE_ROUTE43GATE_NOOP")
        })
        .expect("Route43Gate setscene noop")
        .clone();
    assert_eq!(gate_setscene.command_index, 4);
    state
        .scenes
        .enter_map("Route43Gate", &gate.scenes)
        .expect("enter gate map");
    let outcome =
        apply_script_scene_command(&mut state, "Route43Gate", None, &gate.scenes, gate_setscene)
            .expect("apply setscene");
    assert_eq!(outcome.scene_id, "SCENE_ROUTE43GATE_NOOP");
    assert_eq!(state.scenes.scene_name, "SCENE_ROUTE43GATE_NOOP");

    assert!(route43.scenes.scenes.is_empty());
    let route_setscene = gate
        .script_scene_commands
        .iter()
        .find(|command| {
            command.source_script == ".NoRockets@Route43GateCheckIfRocketsCallback"
                && command.command == "setmapscene"
                && command.map_id.as_deref() == Some("ROUTE_43")
                && command.scene_id.as_deref() == Some("1")
        })
        .expect("Route43Gate setmapscene back to Route43")
        .clone();
    let outcome = apply_script_scene_command(
        &mut state,
        "Route43Gate",
        Some("Route43"),
        &route43.scenes,
        route_setscene,
    )
    .expect("apply numeric Route43 setmapscene");
    assert_eq!(outcome.map_name, "Route43");
    assert_eq!(outcome.scene_id, "1");
    assert_eq!(outcome.scene_index, 1);
    assert_eq!(state.scenes.map_scenes["Route43"], "1");
}

#[test]
fn map_module_extracts_script_audio_commands_with_exact_tokens() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let gate = data
        .map_module("Route43Gate")
        .expect("assemble Route43Gate module");
    let music = gate
        .script_audio_commands
        .iter()
        .find(|command| command.source_script == "Route43GateRocketTakeoverScript")
        .expect("Route43Gate takeover music");
    assert_eq!(music.command, "playmusic");
    assert_eq!(music.audio_id.as_deref(), Some("MUSIC_ROCKET_ENCOUNTER"));
    assert_eq!(music.command_index, 0);

    let gym = data
        .map_module("MahoganyGym")
        .expect("assemble MahoganyGym module");
    let badge = gym
        .script_audio_commands
        .iter()
        .find(|command| command.audio_id.as_deref() == Some("SFX_GET_BADGE"))
        .expect("Mahogany badge sound");
    assert_eq!(badge.command, "playsound");
    assert!(
        gym.script_audio_commands
            .iter()
            .any(|command| command.command == "waitsfx" && command.audio_id.is_none())
    );

    let lugia = data
        .map_module("WhirlIslandLugiaChamber")
        .expect("assemble WhirlIslandLugiaChamber module");
    let cry = lugia
        .script_audio_commands
        .iter()
        .find(|command| command.command == "cry" && command.source_script == "Lugia")
        .expect("Lugia cry");
    assert_eq!(cry.audio_id.as_deref(), Some("LUGIA"));
    assert_eq!(cry.fade_frames, None);
}

#[test]
fn map_module_extracts_changeblock_commands_and_applies_exact_map_blocks() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("RuinsOfAlphKabutoChamber")
        .expect("assemble RuinsOfAlphKabutoChamber module");
    let change = module
        .script_block_changes
        .iter()
        .find(|change| {
            change.source_script == "RuinsOfAlphKabutoChamberHiddenDoorsCallback"
                && change.x == 4
                && change.y == 0
        })
        .expect("Kabuto chamber wall-open changeblock")
        .clone();

    assert_eq!(change.block_id, 0x2e);
    assert_eq!(change.command_index, 2);

    let mut map = data
        .overworld_map("RuinsOfAlphKabutoChamber")
        .expect("load Kabuto chamber map");
    let previous = map.metatile_at(2, 0).expect("block before change");
    let outcome = apply_script_block_change(&mut map, change).expect("apply exact block change");

    assert_eq!((outcome.metatile_x, outcome.metatile_y), (2, 0));
    assert_eq!(outcome.previous_block_id, previous);
    assert_eq!(outcome.block_id, 0x2e);
    assert_eq!(map.metatile_at(2, 0), Some(0x2e));
}

#[test]
fn map_module_extracts_script_map_commands_with_exact_destinations() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let gym = data
        .map_module("EcruteakGym")
        .expect("assemble EcruteakGym module");
    let warp = gym
        .script_map_commands
        .iter()
        .find(|command| command.source_script == "EcruteakGymClosed")
        .expect("Ecruteak gym closed warp");
    assert_eq!(warp.command, "warp");
    assert_eq!(warp.target_map.as_deref(), Some("EcruteakCity"));
    assert_eq!((warp.x, warp.y), (Some(6), Some(27)));
    assert_eq!(warp.command_index, 12);

    let train = data
        .map_module("SaffronMagnetTrainStation")
        .expect("assemble SaffronMagnetTrainStation module");
    assert!(train.script_map_commands.iter().any(|command| {
        command.command == "newloadmap" && command.map_setup.as_deref() == Some("MAPSETUP_TRAIN")
    }));

    let bedroom = data
        .map_module("PlayersHouse2F")
        .expect("assemble PlayersHouse2F module");
    assert!(bedroom.script_map_commands.iter().any(|command| {
        command.command == "warp"
            && command.target_map.as_deref() == Some("NONE")
            && command.x == Some(0)
            && command.y == Some(0)
    }));
}

#[test]
fn map_module_extracts_script_text_commands_with_exact_labels() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("BlackthornCity")
        .expect("assemble BlackthornCity module");

    let write = module
        .script_text_commands
        .iter()
        .find(|command| {
            command.source_script == "BlackthornSuperNerdScript"
                && command.command == "writetext"
                && command.text_label.as_deref() == Some("Text_ClairIsOut")
        })
        .expect("Blackthorn super nerd text");
    assert_eq!(write.command_index, 6);

    let sign = module
        .script_text_commands
        .iter()
        .find(|command| {
            command.source_script == "BlackthornCitySign"
                && command.command == "jumptext"
                && command.text_label.as_deref() == Some("BlackthornCitySignText")
        })
        .expect("Blackthorn city sign text");
    assert_eq!(sign.command_index, 0);

    let gramps = module
        .script_text_commands
        .iter()
        .find(|command| {
            command.source_script == "BlackthornGramps1Script"
                && command.command == "jumptextfaceplayer"
                && command.text_label.as_deref() == Some("BlackthornGrampsRefusesEntryText")
        })
        .expect("Blackthorn gramps face text");
    assert_eq!(gramps.command_index, 0);

    assert!(module.script_text_commands.iter().any(|command| {
        command.source_script == "BlackthornSuperNerdScript"
            && command.command == "opentext"
            && command.text_label.is_none()
    }));
    assert!(module.script_text_commands.iter().any(|command| {
        command.source_script == "BlackthornSuperNerdScript"
            && command.command == "waitbutton"
            && command.text_label.is_none()
    }));
    assert!(module.script_text_commands.iter().any(|command| {
        command.source_script == "BlackthornSuperNerdScript"
            && command.command == "closetext"
            && command.text_label.is_none()
    }));

    let clair = module
        .script_text_bodies
        .get("Text_ClairIsOut")
        .expect("typed Clair text body");
    assert_eq!(clair.label, "Text_ClairIsOut");
    assert_eq!(clair.commands[0].command, "text");
    assert_eq!(clair.commands[0].args, vec!["\"I am sorry.\""]);
    assert_eq!(clair.commands[1].command, "para");
    assert_eq!(clair.commands[1].args, vec!["\"CLAIR, our GYM\""]);
    assert!(
        clair
            .commands
            .iter()
            .any(|command| command.command == "done" && command.args.is_empty())
    );

    let vending = data
        .map_module("CeladonDeptStore6F")
        .expect("assemble CeladonDeptStore6F module");
    let menu_header = vending
        .script_menu_definitions
        .get(".MenuHeader@CeladonDeptStore6FVendingMachine")
        .expect("typed vending menu header");
    assert!(menu_header.commands.iter().any(|command| {
        command.command == "menu_coords"
            && command.args == vec!["0", "2", "SCREEN_WIDTH - 1", "TEXTBOX_Y - 1"]
    }));
    let menu_data = vending
        .script_menu_definitions
        .get(".MenuData@CeladonDeptStore6FVendingMachine")
        .expect("typed vending menu data");
    assert!(menu_data.commands.iter().any(|command| {
        command.command == "db"
            && command.args == vec!["\"FRESH WATER  ¥{d:CELADONDEPTSTORE6F_FRESH_WATER_PRICE}@\""]
    }));
}

#[test]
fn map_module_extracts_script_variable_commands_with_exact_tokens() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let route44 = data.map_module("Route44").expect("assemble Route44 module");
    let caller = route44
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == ".Script@TrainerBirdKeeperVance1"
                && command.command == "loadvar"
                && command.target.as_deref() == Some("VAR_CALLERID")
        })
        .expect("Vance caller variable");
    assert_eq!(caller.command_index, 1);
    assert_eq!(caller.value_tokens, vec!["PHONE_BIRDKEEPER_VANCE"]);

    let rematch_read = route44
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == ".WantsBattle@TrainerBirdKeeperVance1"
                && command.command == "readmem"
                && command.target.as_deref() == Some("wVanceFightCount")
        })
        .expect("Vance fight count read");
    assert_eq!(rematch_read.command_index, 2);
    assert!(rematch_read.value_tokens.is_empty());

    let rematch_load = route44
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == ".LoadFight1@TrainerBirdKeeperVance1"
                && command.command == "loadmem"
                && command.target.as_deref() == Some("wVanceFightCount")
        })
        .expect("Vance fight count load");
    assert_eq!(rematch_load.command_index, 3);
    assert_eq!(rematch_load.value_tokens, vec!["2"]);

    let route29 = data.map_module("Route29").expect("assemble Route29 module");
    let weekday = route29
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == "Route29TuscanyCallback"
                && command.command == "readvar"
                && command.target.as_deref() == Some("VAR_WEEKDAY")
        })
        .expect("Tuscany weekday read");
    assert_eq!(weekday.command_index, 4);
    assert!(
        route29
            .objects
            .iter()
            .any(|object| object.event_flag == "EVENT_ROUTE_29_TUSCANY_OF_TUESDAY")
    );
    assert!(data.saved_event_flag_exists("EVENT_ROUTE_29_TUSCANY_OF_TUESDAY"));
    let day_check = route29
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == "Route29CooltrainerMScript"
                && command.command == "checktime"
                && command.value_tokens == vec!["DAY"]
        })
        .expect("cooltrainer day check");
    assert_eq!(day_check.command_index, 2);

    let switches = data
        .map_module("GoldenrodUndergroundSwitchRoomEntrances")
        .expect("assemble underground switches");
    let setval = switches
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == "EmergencySwitchScript"
                && command.command == "setval"
                && command.value_tokens == vec!["7"]
        })
        .expect("emergency switch setval");
    assert_eq!(setval.command_index, 8);
    let write = switches
        .script_variable_commands
        .iter()
        .find(|command| {
            command.source_script == "EmergencySwitchScript"
                && command.command == "writemem"
                && command.target.as_deref() == Some("wUndergroundSwitchPositions")
        })
        .expect("emergency switch writemem");
    assert_eq!(write.command_index, 9);
    let callback = switches
        .scripts
        .get("GoldenrodUndergroundSwitchRoomEntrancesUpdateDoorPositionsCallback")
        .and_then(Value::as_array)
        .expect("expanded underground door callback body");
    assert!(
        callback.iter().any(|entry| {
            entry.get("command").and_then(Value::as_str) == Some("changeblock")
                && entry
                    .get("args")
                    .and_then(Value::as_array)
                    .is_some_and(|args| args.len() == 3)
        }),
        "changeugdoor must be expanded into concrete changeblock commands"
    );
}

#[test]
fn script_control_compares_party_count_to_pret_party_length_byte() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    assert_eq!(
        data.script_numeric_constants().get("PARTY_LENGTH"),
        Some(&(crystal_core::models::PARTY_SIZE as i32))
    );

    let command = data
        .map_module("VioletPokecenter1F")
        .expect("assemble VioletPokecenter1F module")
        .script_control_commands
        .iter()
        .find(|command| {
            command.source_script == "VioletPokecenter1F_ElmsAideScript"
                && command.command == "ifequal"
                && command.compare_value.as_deref() == Some("PARTY_LENGTH")
        })
        .expect("Togepi Egg party-full branch");

    let mut state = GameState::default();
    state.script_runtime.script_value = Some("6".to_string());
    let action = data
        .apply_script_control_command(
            &mut state,
            "VioletPokecenter1F",
            "VioletPokecenter1F",
            &command.source_script,
            command.command_index,
        )
        .expect("compare VAR_PARTYCOUNT with PARTY_LENGTH");

    assert!(matches!(
        action,
        ScriptControlAction::Jump { target_script, .. }
            if target_script == ".PartyFull@VioletPokecenter1F_ElmsAideScript"
    ));
}

#[test]
fn route44_endifjustbattled_only_ends_post_battle_dispatch() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let route44 = data
        .map_module("Route44")
        .expect("assemble Route44 module")
        .clone();
    let command = route44
        .script_control_commands
        .iter()
        .find(|command| {
            command.source_script == "TrainerBirdKeeperVance1"
                && command.command == "endifjustbattled"
        })
        .expect("Vance after-battle guard")
        .clone();
    let trainer_command_index = route44
        .scripts
        .get("TrainerBirdKeeperVance1")
        .and_then(serde_json::Value::as_array)
        .and_then(|commands| {
            commands.iter().position(|command| {
                command.get("command").and_then(serde_json::Value::as_str) == Some("trainer")
            })
        })
        .expect("Vance trainer table command");

    let mut state = GameState::default();
    state.storage.party.pokemon[0] = Some(crystal_core::models::Pokemon::new_for_tests(
        data.pokemon
            .get("PIDGEY")
            .expect("compiled PIDGEY species")
            .clone(),
        20,
        crystal_core::models::Dv::default(),
    ));
    let start = data
        .start_scripted_trainer_battle(
            &mut state,
            "Route44",
            "Route44",
            "TrainerBirdKeeperVance1",
            trainer_command_index,
        )
        .expect("start Vance's trainer-table battle");
    assert!(matches!(start, TrainerBattleStartStatus::Started(_)));
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wRunningTrainerBattleScript")
            .map(String::as_str),
        Some("0")
    );
    loop {
        let BattleMemory::Trainer { enemy_pokemon, .. } = &mut state.battle else {
            panic!("Vance trainer table did not activate a trainer battle");
        };
        enemy_pokemon.hp = 0;
        data.claim_active_trainer_battle_rewards_now(&mut state)
            .expect("claim Vance's trainer battle rewards");
        if data
            .advance_active_trainer_battle(&mut state)
            .expect("advance Vance's trainer battle")
            .trainer_defeated
        {
            break;
        }
    }

    let mut divider = crystal_core::random::ReplayDivider::new([]);
    let completion = data
        .complete_scripted_trainer_battle(
            &mut state,
            "Route44",
            "Route44",
            "TrainerBirdKeeperVance1",
            trainer_command_index,
            true,
            false,
            &mut divider,
        )
        .expect("complete Vance's trainer-table battle");
    assert!(completion.continued_after_battle);
    assert_eq!(state.battle_result, 0);
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("0"));
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wRunningTrainerBattleScript")
            .map(String::as_str),
        Some("-1")
    );
    let post_battle_action = data
        .apply_script_control_command(
            &mut state,
            "Route44",
            "Route44",
            &command.source_script,
            command.command_index,
        )
        .expect("post-battle Vance dispatch reaches its guard");
    assert!(matches!(
        post_battle_action,
        ScriptControlAction::End {
            just_battled_guard: true,
            ..
        }
    ));

    let manual_talk = data
        .start_scripted_trainer_battle(
            &mut state,
            "Route44",
            "Route44",
            "TrainerBirdKeeperVance1",
            trainer_command_index,
        )
        .expect("talk to beaten Vance");
    assert!(matches!(
        manual_talk,
        TrainerBattleStartStatus::AlreadyDefeated { .. }
    ));
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wRunningTrainerBattleScript")
            .map(String::as_str),
        Some("0")
    );
    let manual_action = data
        .apply_script_control_command(
            &mut state,
            "Route44",
            "Route44",
            &command.source_script,
            command.command_index,
        )
        .expect("manual talk reaches Vance's phone dialogue");
    assert!(matches!(
        manual_action,
        ScriptControlAction::Continue { .. }
    ));

    let mut lost_state = GameState::default();
    lost_state.storage.party.pokemon[0] = Some(crystal_core::models::Pokemon::new_for_tests(
        data.pokemon
            .get("PIDGEY")
            .expect("compiled PIDGEY species")
            .clone(),
        20,
        crystal_core::models::Dv::default(),
    ));
    data.start_scripted_trainer_battle(
        &mut lost_state,
        "Route44",
        "Route44",
        "TrainerBirdKeeperVance1",
        trainer_command_index,
    )
    .expect("start loss-continuation trainer battle");
    let mut divider = crystal_core::random::ReplayDivider::new([]);
    let lost = data
        .complete_scripted_trainer_battle(
            &mut lost_state,
            "Route44",
            "Route44",
            "TrainerBirdKeeperVance1",
            trainer_command_index,
            false,
            true,
            &mut divider,
        )
        .expect("complete can-lose trainer battle");
    assert!(lost.continued_after_battle);
    assert_eq!(lost_state.battle_result, 1);
    assert_eq!(lost_state.script_runtime.script_value.as_deref(), Some("1"));
    assert_eq!(
        lost_state
            .script_runtime
            .variables
            .get("_value")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn map_module_extracts_script_control_commands_with_exact_targets() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");

    let blackthorn = data
        .map_module("BlackthornCity")
        .expect("assemble BlackthornCity module");
    let santos = blackthorn
        .script_control_commands
        .iter()
        .find(|command| {
            command.source_script == "BlackthornCitySantosCallback"
                && command.command == "ifequal"
                && command.compare_value.as_deref() == Some("SATURDAY")
                && command.target_label.as_deref() == Some(".SantosAppears")
        })
        .expect("Santos local branch");
    assert_eq!(santos.command_index, 1);
    assert_eq!(
        santos.resolved_target_script.as_deref(),
        Some(".SantosAppears@BlackthornCitySantosCallback")
    );

    let route44 = data.map_module("Route44").expect("assemble Route44 module");
    let nested = route44
        .script_control_commands
        .iter()
        .find(|command| {
            command.source_script == ".WantsBattle@TrainerBirdKeeperVance1"
                && command.command == "ifequal"
                && command.compare_value.as_deref() == Some("2")
                && command.target_label.as_deref() == Some(".Fight2")
        })
        .expect("nested local branch resolves to parent script");
    assert_eq!(
        nested.resolved_target_script.as_deref(),
        Some(".Fight2@TrainerBirdKeeperVance1")
    );

    let call = route44
        .script_control_commands
        .iter()
        .find(|command| {
            command.source_script == ".Script@TrainerBirdKeeperVance1"
                && command.command == "scall"
                && command.target_label.as_deref() == Some("Route44AskNumber1M")
        })
        .expect("Route44 scall");
    assert_eq!(
        call.resolved_target_script.as_deref(),
        Some("Route44AskNumber1M")
    );

    let gym = data
        .map_module("EcruteakGym")
        .expect("assemble EcruteakGym module");
    let standard = gym
        .script_control_commands
        .iter()
        .find(|command| {
            command.source_script == "EcruteakGymStatue"
                && command.command == "jumpstd"
                && command.target_label.as_deref() == Some("GymStatue1Script")
        })
        .expect("gym statue jumpstd");
    assert_eq!(standard.resolved_target_script, None);
    assert!(
        !gym.script_runtime_commands
            .iter()
            .any(|command| command.command == "jumpstd"),
        "jumpstd must be owned by script control commands, not duplicated as runtime"
    );

    assert!(route44.script_control_commands.iter().any(|command| {
        command.source_script == ".Script@TrainerBirdKeeperVance1"
            && command.command == "endifjustbattled"
            && command.target_label.is_none()
            && command.compare_value.is_none()
    }));
    assert!(
        !route44
            .script_runtime_commands
            .iter()
            .any(|command| command.command == "endifjustbattled"),
        "endifjustbattled must be owned by script control commands, not duplicated as runtime"
    );
}

#[test]
fn map_module_extracts_object_commands_and_applies_exact_mutations() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("IndigoPlateauPokecenter1F")
        .expect("assemble IndigoPlateauPokecenter1F module");
    let moveobject = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "moveobject"
                && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival moveobject command")
        .clone();
    let appear = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "appear"
                && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival appear command")
        .clone();
    let disappear = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "disappear"
                && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival disappear command")
        .clone();
    let applymovement = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "applymovement"
                && command.object_id.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
                && command.movement.as_deref() == Some("PlateauRivalMovement1")
        })
        .expect("rival applymovement command")
        .clone();
    let rival_movement = module
        .script_movements
        .iter()
        .find(|movement| movement.label == "PlateauRivalMovement1")
        .expect("rival movement script")
        .clone();
    let turn_player = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "turnobject"
                && command.object_id.as_deref() == Some("PLAYER")
                && command.source_script == "PlateauRivalBattle1"
        })
        .expect("player turnobject command");
    let emote_player = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "showemote"
                && command.object_id.as_deref() == Some("PLAYER")
                && command.source_script == "PlateauRivalBattle1"
        })
        .expect("player showemote command");
    assert!(
        !module
            .script_runtime_commands
            .iter()
            .any(|command| command.command == "faceplayer"),
        "faceplayer must be owned by script object commands, not duplicated as runtime"
    );
    assert!(
        !module
            .script_runtime_commands
            .iter()
            .any(|command| command.command == "showemote"),
        "showemote must be owned by script object commands, not duplicated as runtime"
    );

    assert_eq!((moveobject.x, moveobject.y), (Some(17), Some(9)));
    assert_eq!(rival_movement.steps.len(), 7);
    assert_eq!(rival_movement.steps[0].command, "step");
    assert_eq!(rival_movement.steps[0].direction.as_deref(), Some("UP"));
    assert_eq!(rival_movement.steps[5].command, "turn_head");
    assert_eq!(rival_movement.steps[5].direction.as_deref(), Some("LEFT"));
    assert_eq!(turn_player.direction.as_deref(), Some("DOWN"));
    assert_eq!(emote_player.emote.as_deref(), Some("EMOTE_SHOCK"));
    assert_eq!(emote_player.duration, Some(15));

    let mut session = OverworldSession::with_events_and_objects(
        data.overworld_map("IndigoPlateauPokecenter1F")
            .expect("load IndigoPlateauPokecenter1F map"),
        module.events.clone(),
        module.objects.clone(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let mut state = GameState::default();

    let moved = apply_script_object_mutation(&mut state, &mut session, &moveobject)
        .expect("moveobject applies");
    assert_eq!((moved.x, moved.y), (Some(17), Some(9)));
    let rival = session
        .objects
        .iter()
        .find(|object| {
            object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival object after move");
    assert_eq!(
        session
            .object_runtime_tiles
            .get("INDIGOPLATEAUPOKECENTER1F_RIVAL"),
        Some(&TilePosition::new(17, 9))
    );
    assert_eq!(
        (rival.x, rival.y),
        (16, 9),
        "pack object coordinates remain definitive while runtime overlays move objects"
    );

    let moved_by_script = apply_script_movement(&mut session, &applymovement, &rival_movement)
        .expect("applymovement moves rival");
    assert_eq!(moved_by_script.previous_tile, TilePosition::new(17, 9));
    assert_eq!(moved_by_script.tile, TilePosition::new(17, 4));
    let rival = session
        .objects
        .iter()
        .find(|object| {
            object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival object after movement");
    assert_eq!(
        session
            .object_runtime_tiles
            .get("INDIGOPLATEAUPOKECENTER1F_RIVAL"),
        Some(&TilePosition::new(17, 4))
    );
    assert_eq!((rival.x, rival.y), (16, 9));
    assert_eq!(
        session
            .object_facings
            .get("INDIGOPLATEAUPOKECENTER1F_RIVAL"),
        Some(&Direction::Left)
    );

    apply_script_object_mutation(&mut state, &mut session, &disappear).expect("disappear applies");
    let rival = session
        .objects
        .iter()
        .find(|object| {
            object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival object after disappear");
    assert!(!session.is_object_visible(rival));

    apply_script_object_mutation(&mut state, &mut session, &appear).expect("appear applies");
    let rival = session
        .objects
        .iter()
        .find(|object| {
            object.object_identifier.as_deref() == Some("INDIGOPLATEAUPOKECENTER1F_RIVAL")
        })
        .expect("rival object after appear");
    assert!(session.is_object_visible(rival));
}

#[test]
fn map_module_extracts_fixed_facing_movement_without_turning_player() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let module = data
        .map_module("EcruteakGym")
        .expect("assemble EcruteakGym module");
    let command = module
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "applymovement"
                && command.object_id.as_deref() == Some("PLAYER")
                && command.movement.as_deref() == Some("EcruteakGymPlayerSlowStepDownMovement")
        })
        .expect("player fixed-facing applymovement")
        .clone();
    let movement = module
        .script_movements
        .iter()
        .find(|movement| movement.label == "EcruteakGymPlayerSlowStepDownMovement")
        .expect("player fixed-facing movement")
        .clone();

    assert_eq!(movement.steps[0].command, "fix_facing");
    assert_eq!(movement.steps[1].command, "slow_step");
    assert_eq!(movement.steps[1].direction.as_deref(), Some("DOWN"));
    assert_eq!(movement.steps[2].command, "remove_fixed_facing");

    let mut session = OverworldSession::with_events_and_objects(
        data.overworld_map("EcruteakGym")
            .expect("load EcruteakGym map"),
        module.events.clone(),
        module.objects.clone(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(8, 10),
    );
    session.player.facing = Direction::Left;

    let outcome =
        apply_script_movement(&mut session, &command, &movement).expect("movement applies");

    assert_eq!(outcome.previous_tile, TilePosition::new(8, 10));
    assert_eq!(outcome.tile, TilePosition::new(8, 11));
    assert_eq!(session.player.tile, TilePosition::new(8, 11));
    assert_eq!(session.player.facing, Direction::Left);
    assert_eq!(outcome.facing, Direction::Left);
    assert_eq!(
        outcome.effects,
        vec![
            crystal_core::systems::script_objects::ScriptMovementEffect {
                command: "fix_facing".to_string(),
                index: 0,
            },
            crystal_core::systems::script_objects::ScriptMovementEffect {
                command: "remove_fixed_facing".to_string(),
                index: 2,
            },
        ]
    );
}

#[test]
fn runtime_script_movement_visibility_steps_update_visibility_state() {
    let mut module = test_map_module("Route29", "ROUTE_29", None);
    module.objects = vec![test_object(
        "ROUTE_29_YOUNGSTER",
        "EVENT_HIDE_YOUNGSTER",
        2,
        2,
    )];
    module.script_object_commands = vec![ScriptObjectCommand {
        command: "applymovement".to_string(),
        object_id: Some("ROUTE_29_YOUNGSTER".to_string()),
        target_object_id: None,
        x: None,
        y: None,
        direction: None,
        movement: Some("Route29HideShowMovement".to_string()),
        emote: None,
        duration: None,
        source_script: "Route29ObjectScript".to_string(),
        command_index: 0,
    }];
    module.script_movements = vec![ScriptMovement {
        label: "Route29HideShowMovement".to_string(),
        source_script: Some("Route29ObjectScript".to_string()),
        steps: vec![
            ScriptMovementStep {
                command: "hide_object".to_string(),
                direction: None,
                duration: None,
                index: 0,
            },
            ScriptMovementStep {
                command: "show_object".to_string(),
                direction: None,
                duration: None,
                index: 1,
            },
            ScriptMovementStep {
                command: "hide_emote".to_string(),
                direction: None,
                duration: None,
                index: 2,
            },
            ScriptMovementStep {
                command: "remove_object".to_string(),
                direction: None,
                duration: None,
                index: 3,
            },
            ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 4,
            },
        ],
    }];
    let object = module.objects[0].clone();
    let mut data = GameDataSet::default();
    data.maps.insert("Route29".to_string(), module);
    let mut state = GameState::default();
    state
        .script_runtime
        .pending_emotes
        .push(ScriptRuntimeEmote {
            emote: "EMOTE_SHOCK".to_string(),
            object: "ROUTE_29_YOUNGSTER".to_string(),
            duration: 16,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 0,
        });
    state
        .script_runtime
        .pending_emotes
        .push(ScriptRuntimeEmote {
            emote: "EMOTE_HAPPY".to_string(),
            object: "OTHER_OBJECT".to_string(),
            duration: 8,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 0,
        });
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "Route29".to_string(),
            width: 2,
            height: 2,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0, 0, 0, 0],
        },
        MapEvents::default(),
        vec![object],
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );

    let outcome = data
        .apply_script_movement_in_session(
            &mut state,
            &mut session,
            "Route29",
            "Route29ObjectScript",
            0,
        )
        .expect("movement applies through runtime data");

    assert_eq!(
        outcome
            .effects
            .iter()
            .map(|effect| effect.command.as_str())
            .collect::<Vec<_>>(),
        vec!["hide_object", "show_object", "hide_emote", "remove_object"]
    );
    assert_eq!(state.script_runtime.pending_emotes.len(), 1);
    assert_eq!(
        state.script_runtime.pending_emotes[0].object,
        "OTHER_OBJECT"
    );
    assert_eq!(
        state.flags.event_flags.get("EVENT_HIDE_YOUNGSTER"),
        Some(&true)
    );
    let youngster = session
        .objects
        .iter()
        .find(|object| object.object_identifier.as_deref() == Some("ROUTE_29_YOUNGSTER"))
        .expect("youngster object");
    assert!(!session.is_object_visible(youngster));
}

#[test]
fn runtime_script_movement_teleport_to_clears_teleport_from_flag() {
    let mut module = test_map_module("Route29", "ROUTE_29", None);
    module.script_object_commands = vec![ScriptObjectCommand {
        command: "applymovement".to_string(),
        object_id: Some("PLAYER".to_string()),
        target_object_id: None,
        x: None,
        y: None,
        direction: None,
        movement: Some("Route29TeleportMovement".to_string()),
        emote: None,
        duration: None,
        source_script: "Route29ObjectScript".to_string(),
        command_index: 0,
    }];
    module.script_movements = vec![ScriptMovement {
        label: "Route29TeleportMovement".to_string(),
        source_script: Some("Route29ObjectScript".to_string()),
        steps: vec![
            ScriptMovementStep {
                command: "teleport_from".to_string(),
                direction: None,
                duration: None,
                index: 0,
            },
            ScriptMovementStep {
                command: "teleport_to".to_string(),
                direction: None,
                duration: None,
                index: 1,
            },
            ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 2,
            },
        ],
    }];
    let mut data = GameDataSet::default();
    data.maps.insert("Route29".to_string(), module);
    let mut state = GameState::default();
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "Route29".to_string(),
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

    let outcome = data
        .apply_script_movement_in_session(
            &mut state,
            &mut session,
            "Route29",
            "Route29ObjectScript",
            0,
        )
        .expect("teleport movement applies through runtime data");

    assert_eq!(
        outcome
            .effects
            .iter()
            .map(|effect| effect.command.as_str())
            .collect::<Vec<_>>(),
        vec!["teleport_from", "teleport_to"]
    );
    assert!(
        !state.script_runtime.teleport_from_queued,
        "teleport_to must close the queued teleport_from effect"
    );
}

#[test]
fn runtime_script_movement_dig_effects_sync_player_visibility() {
    let mut module = test_map_module("Route29", "ROUTE_29", None);
    module.script_object_commands = vec![
        ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29DigOutMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 0,
        },
        ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29DigReturnMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 1,
        },
    ];
    module.script_movements = vec![
        ScriptMovement {
            label: "Route29DigOutMovement".to_string(),
            source_script: Some("Route29ObjectScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "step_dig".to_string(),
                    direction: None,
                    duration: Some(32),
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
            ],
        },
        ScriptMovement {
            label: "Route29DigReturnMovement".to_string(),
            source_script: Some("Route29ObjectScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "return_dig".to_string(),
                    direction: None,
                    duration: Some(32),
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 1,
                },
            ],
        },
    ];
    let mut data = GameDataSet::default();
    data.maps.insert("Route29".to_string(), module);
    let mut state = GameState::default();
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "Route29".to_string(),
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

    data.apply_script_movement_in_session(
        &mut state,
        &mut session,
        "Route29",
        "Route29ObjectScript",
        0,
    )
    .expect("dig out movement applies through runtime data");
    assert!(session.player_hidden);
    assert_eq!(
        state
            .map_object_overrides
            .get("Route29")
            .map(|memory| memory.player_hidden),
        Some(true)
    );

    data.apply_script_movement_in_session(
        &mut state,
        &mut session,
        "Route29",
        "Route29ObjectScript",
        1,
    )
    .expect("dig return movement applies through runtime data");
    assert!(!session.player_hidden);
    assert_eq!(
        state
            .map_object_overrides
            .get("Route29")
            .map(|memory| memory.player_hidden),
        Some(false)
    );
}

#[test]
fn map_module_extracts_follow_and_last_talked_object_commands() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let tower = data
        .map_module("BattleTower1F")
        .expect("assemble BattleTower1F module");
    let follow = tower
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "follow"
                && command.source_script == "Script_WalkToBattleTowerElevator"
        })
        .expect("BattleTower follow command")
        .clone();
    let stopfollow = tower
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "stopfollow"
                && command.source_script == "Script_WalkToBattleTowerElevator"
        })
        .expect("BattleTower stopfollow command")
        .clone();

    assert_eq!(
        follow.object_id.as_deref(),
        Some("BATTLETOWER1F_RECEPTIONIST")
    );
    assert_eq!(follow.target_object_id.as_deref(), Some("PLAYER"));

    let mut session = OverworldSession::with_events_and_objects(
        data.overworld_map("BattleTower1F")
            .expect("load BattleTower1F map"),
        tower.events.clone(),
        tower.objects.clone(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let mut state = GameState::default();

    apply_script_object_mutation(&mut state, &mut session, &follow).expect("follow applies");
    assert_eq!(
        session.following,
        Some(crystal_core::world::session::OverworldFollowState {
            leader_object_id: "BATTLETOWER1F_RECEPTIONIST".to_string(),
            follower_object_id: "PLAYER".to_string(),
        })
    );
    apply_script_object_mutation(&mut state, &mut session, &stopfollow)
        .expect("stopfollow applies");
    assert_eq!(session.following, None);

    let pokecenter = data
        .map_module("Pokecenter2F")
        .expect("assemble Pokecenter2F module");
    let last_talked = pokecenter
        .script_object_commands
        .iter()
        .find(|command| {
            command.command == "applymovementlasttalked"
                && command.source_script == "BattleTradeMobile_WalkIn"
        })
        .expect("applymovementlasttalked command");
    assert_eq!(
        last_talked.movement.as_deref(),
        Some("Pokecenter2FMobileMobileMovementData_ReceptionistWalksUpAndLeft_LookDown")
    );
    assert!(pokecenter.script_movements.iter().any(|movement| {
        movement.label == "Pokecenter2FMobileMobileMovementData_ReceptionistWalksUpAndLeft_LookDown"
    }));
}

#[test]
fn map_module_extracts_runtime_commands_with_exact_tokens() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let azalea = data
        .map_module("AzaleaTown")
        .expect("assemble AzaleaTown module");
    let special = azalea
        .script_runtime_commands
        .iter()
        .find(|command| {
            command.command == "special" && command.source_script == "AzaleaTownRivalBattleScene1"
        })
        .expect("rival scene special command");
    assert_eq!(special.args, vec!["FadeOutMusic"]);
    let pause = azalea
        .script_runtime_commands
        .iter()
        .find(|command| {
            command.command == "pause" && command.source_script == "AzaleaTownRivalBattleScene1"
        })
        .expect("rival scene pause command");
    assert_eq!(pause.args, vec!["15"]);
    assert!(
        !azalea
            .script_runtime_commands
            .iter()
            .any(|command| command.command == "checkscene"),
        "checkscene must be owned by script scene commands, not duplicated as runtime"
    );

    let gym = data
        .map_module("AzaleaGym")
        .expect("assemble AzaleaGym module");
    let trainer_name = gym
        .script_runtime_commands
        .iter()
        .find(|command| {
            command.command == "gettrainername"
                && command.source_script == ".Beaten@AzaleaGymStatue"
        })
        .expect("gym statue trainer name command");
    assert_eq!(
        trainer_name.args,
        vec!["STRING_BUFFER_4", "BUGSY", "BUGSY1"]
    );

    let vending = data
        .map_module("CeladonDeptStore6F")
        .expect("assemble CeladonDeptStore6F module");
    assert!(vending.script_runtime_commands.iter().any(|command| {
        command.command == "loadmenu"
            && command.args == vec![".MenuHeader"]
            && command.source_script == "CeladonDeptStore6FVendingMachine"
    }));
    assert!(vending.script_runtime_commands.iter().any(|command| {
        command.command == "verticalmenu"
            && command.args.is_empty()
            && command.source_script == "CeladonDeptStore6FVendingMachine"
    }));
    assert!(vending.script_runtime_commands.iter().any(|command| {
        command.command == "menu_coords"
            && command.args == vec!["0", "2", "SCREEN_WIDTH - 1", "TEXTBOX_Y - 1"]
    }));

    let bills_family = data
        .map_module("BillsFamilysHouse")
        .expect("assemble Bill family house");
    assert!(bills_family.script_runtime_commands.iter().any(|command| {
        command.command == "addcellnum"
            && command.args == vec!["PHONE_BILL"]
            && command.source_script == "BillsYoungerSisterScript"
    }));

    let dragon_shrine = data
        .map_module("DragonShrine")
        .expect("assemble Dragon Shrine");
    assert!(dragon_shrine.script_runtime_commands.iter().any(|command| {
        command.command == "specialphonecall"
            && command.args == vec!["SPECIALCALL_MASTERBALL"]
            && command.source_script == ".PassedTheTest@DragonShrineTakeTestScript"
    }));

    let route39 = data.map_module("Route39").expect("assemble Route39");
    assert!(route39.script_runtime_commands.iter().any(|command| {
        command.command == "checkpoke"
            && command.args == vec!["PIKACHU"]
            && command.source_script == ".Script@TrainerPokefanmDerek"
    }));

    let elms_lab = data.map_module("ElmsLab").expect("assemble Elm's Lab");
    assert!(elms_lab.script_runtime_commands.iter().any(|command| {
        command.command == "pokepic"
            && command.args == vec!["CYNDAQUIL"]
            && command.source_script == "CyndaquilPokeBallScript"
    }));
    assert!(elms_lab.script_runtime_commands.iter().any(|command| {
        command.command == "closepokepic"
            && command.args.is_empty()
            && command.source_script == "CyndaquilPokeBallScript"
    }));

    let emy = data
        .map_module("BlackthornEmysHouse")
        .expect("assemble Emy trade house");
    assert!(emy.script_runtime_commands.iter().any(|command| {
        command.command == "trade"
            && command.args == vec!["NPC_TRADE_EMY"]
            && command.source_script == "Emy"
    }));

    let blackthorn_gym = data
        .map_module("BlackthornGym2F")
        .expect("assemble Blackthorn Gym 2F");
    assert!(
        blackthorn_gym
            .script_runtime_commands
            .iter()
            .any(|command| {
                command.command == "writecmdqueue"
                    && command.args == vec![".CommandQueue"]
                    && command.source_script == "BlackthornGym2FSetUpStoneTableCallback"
            })
    );
    assert!(
        blackthorn_gym
            .script_runtime_commands
            .iter()
            .any(|command| {
                command.command == "cmdqueue"
                    && command.args == vec!["CMDQUEUE_STONETABLE", ".StoneTable"]
                    && command.source_script
                        == ".CommandQueue@BlackthornGym2FSetUpStoneTableCallback"
            })
    );
    assert!(
        blackthorn_gym
            .script_runtime_commands
            .iter()
            .any(|command| {
                command.command == "stonetable"
                    && command.args == vec!["5", "BLACKTHORNGYM2F_BOULDER1", ".Boulder1"]
            })
    );

    let elevator = data
        .map_module("CeladonDeptStoreElevator")
        .expect("assemble Celadon elevator");
    assert!(elevator.script_runtime_commands.iter().any(|command| {
        command.command == "elevator"
            && command.args == vec!["CeladonDeptStoreElevatorData"]
            && command.source_script == "CeladonDeptStoreElevatorScript"
    }));
    assert!(elevator.script_runtime_commands.iter().any(|command| {
        command.command == "elevfloor"
            && command.args == vec!["FLOOR_1F", "4", "CELADON_DEPT_STORE_1F"]
    }));

    let bedroom = data
        .map_module("PlayersHouse2F")
        .expect("assemble player's bedroom");
    assert!(bedroom.script_runtime_commands.iter().any(|command| {
        command.command == "describedecoration"
            && command.args == vec!["DECODESC_LEFT_DOLL"]
            && command.source_script == "PlayersHouseDoll1Script"
    }));
    assert!(bedroom.script_runtime_commands.iter().any(|command| {
        command.command == "conditional_event"
            && command.args == vec!["EVENT_PLAYERS_ROOM_POSTER", ".Script"]
            && command.source_script == "PlayersHousePosterScript"
    }));

    let route31 = data.map_module("Route31").expect("assemble Route31");
    assert!(route31.script_runtime_commands.iter().any(|command| {
        command.command == "checkpokemail"
            && command.args == vec!["ReceivedSpearowMailText"]
            && command.source_script == ".TryGiveKenya@Route31MailRecipientScript"
    }));
    let route35_gate = data
        .map_module("Route35GoldenrodGate")
        .expect("assemble Route35 Goldenrod gate");
    assert!(route35_gate.script_runtime_commands.iter().any(|command| {
        command.command == "givepokemail"
            && command.args == vec!["GiftSpearowMail"]
            && command.source_script == "RandyScript"
    }));

    let hallway = data
        .map_module("BattleTowerHallway")
        .expect("assemble Battle Tower hallway");
    assert!(hallway.script_runtime_commands.iter().any(|command| {
        command.command == "callasm"
            && command.args == vec![".asm_load_battle_room"]
            && command.source_script == "BattleTowerHallwayChooseBattleRoomScript"
    }));
    assert!(hallway.script_runtime_commands.iter().any(|command| {
        command.command == "ldh"
            && command.args == vec!["a", "[rWBK]"]
            && command.source_script
                == ".asm_load_battle_room@BattleTowerHallwayChooseBattleRoomScript"
    }));
    assert!(hallway.script_runtime_commands.iter().any(|command| {
        command.command == "ret"
            && command.args.is_empty()
            && command.source_script
                == ".asm_load_battle_room@BattleTowerHallwayChooseBattleRoomScript"
    }));

    let academy = data
        .map_module("EarlsPokemonAcademy")
        .expect("assemble Earl's academy");
    assert!(academy.script_runtime_commands.iter().any(|command| {
        command.command == "_2dmenu"
            && command.args.is_empty()
            && command.source_script == "AcademyBlackboard"
    }));
    assert!(academy.script_runtime_commands.iter().any(|command| {
        command.command == "dba"
            && command.args == vec![".Text"]
            && command.source_script == ".MenuData@AcademyBlackboard"
    }));

    let radio_tower = data
        .map_module("RadioTower2F")
        .expect("assemble Radio Tower 2F");
    assert!(radio_tower.script_runtime_commands.iter().any(|command| {
        command.command == "writevar"
            && command.args == vec!["VAR_BLUECARDBALANCE"]
            && command.source_script == "Buena"
    }));

    let route35_gate = data
        .map_module("Route35NationalParkGate")
        .expect("assemble Route35 National Park gate");
    assert!(route35_gate.script_runtime_commands.iter().any(|command| {
        command.command == "getnum"
            && command.args == vec!["STRING_BUFFER_3"]
            && command.source_script == "Route35NationalParkGateLeavingContestEarlyScript"
    }));
}

#[test]
fn writecmdqueue_canonicalizes_map_local_target_before_state_validation() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let command = data
        .map_module("IcePathB1F")
        .expect("assemble Ice Path B1F module")
        .script_runtime_commands
        .iter()
        .find(|command| {
            command.command == "writecmdqueue"
                && command.source_script == "IcePathB1FSetUpStoneTableCallback"
        })
        .expect("Ice Path B1F command-queue callback")
        .clone();
    let mut state = GameState::default();
    let mut session = data
        .overworld_session("IcePathB1F", TilePosition::new(3, 14), 0)
        .expect("start Ice Path B1F session");

    data.apply_script_runtime_command_in_session(
        &mut state,
        &mut session,
        "IcePathB1F",
        &command.source_script,
        command.command_index,
        ScriptRuntimeInputs::default(),
    )
    .expect("execute Ice Path B1F command-queue callback");

    assert_eq!(state.script_runtime.command_queue.len(), 1);
    assert_eq!(
        state.script_runtime.command_queue[0].target,
        ".CommandQueue"
    );
    let audio_ids = BTreeSet::new();
    data.apply_runtime_mutation_command(
        &mut state,
        &mut session,
        RuntimeMutationCommand::ExecuteNextQueuedScriptCommand,
        &audio_ids,
        &audio_ids,
        &audio_ids,
    )
    .expect("execute queued local callback");
    assert_eq!(
        state
            .script_runtime
            .next_script
            .as_ref()
            .map(|location| location.script.as_str()),
        Some(".CommandQueue@IcePathB1FSetUpStoneTableCallback")
    );
}

#[test]
fn battle_tower_hallway_callasm_routes_all_level_groups_from_wram() {
    let root = repository_root_for_tests();
    let data = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let hallway = data
        .map_module("BattleTowerHallway")
        .expect("assemble Battle Tower hallway");
    let callasm = hallway
        .script_runtime_commands
        .iter()
        .find(|command| {
            command.command == "callasm"
                && command.source_script == "BattleTowerHallwayChooseBattleRoomScript"
        })
        .expect("hallway battle-room WRAM callasm");
    let session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "BattleTowerHallway".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        hallway.events.clone(),
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let walk_script = ".WalkToChosenBattleRoom@BattleTowerHallwayChooseBattleRoomScript";

    for (level_group, expected_branch, expected_movement) in [
        (
            1,
            walk_script,
            "MovementData_BattleTowerHallwayWalkTo1020Room",
        ),
        (
            2,
            walk_script,
            "MovementData_BattleTowerHallwayWalkTo1020Room",
        ),
        (
            3,
            ".L30L40@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo3040Room",
        ),
        (
            4,
            ".L30L40@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo3040Room",
        ),
        (
            5,
            ".L50L60@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo5060Room",
        ),
        (
            6,
            ".L50L60@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo5060Room",
        ),
        (
            7,
            ".L70L80@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo7080Room",
        ),
        (
            8,
            ".L70L80@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo7080Room",
        ),
        (
            9,
            ".L90L100@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo90100Room",
        ),
        (
            10,
            ".L90L100@BattleTowerHallwayChooseBattleRoomScript",
            "MovementData_BattleTowerHallwayWalkTo90100Room",
        ),
    ] {
        let mut state = GameState::default();
        state.battle_tower.level_group = level_group;
        state.script_runtime.script_value = Some("255".to_string());
        let mut session = session.clone();
        let expected_value = level_group.to_string();

        let (_, outcome) = data
            .apply_script_runtime_command_in_session(
                &mut state,
                &mut session,
                "BattleTowerHallway",
                &callasm.source_script,
                callasm.command_index,
                ScriptRuntimeInputs::default(),
            )
            .expect("load Battle Tower level group through compiled callasm");

        assert!(matches!(
            outcome,
            ScriptRuntimeOutcome::ScriptValueSet { ref value, .. }
                if value == &expected_value
        ));
        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some(expected_value.as_str())
        );
        assert_eq!(
            state
                .script_runtime
                .memory
                .get("wScriptVar")
                .map(String::as_str),
            Some(expected_value.as_str())
        );
        assert!(
            state.script_runtime.command_queue.is_empty(),
            "level group {level_group} callasm must finish synchronously"
        );

        let mut selected_script = walk_script.to_string();
        for command_index in 0..8 {
            match data
                .apply_script_control_command(
                    &mut state,
                    "BattleTowerHallway",
                    "BattleTowerHallway",
                    walk_script,
                    command_index,
                )
                .expect("route Battle Tower level group")
            {
                ScriptControlAction::Continue { .. } => {}
                ScriptControlAction::Jump { target_script, .. } => {
                    selected_script = target_script;
                    break;
                }
                action => panic!(
                    "level group {level_group} produced unexpected hallway action {action:?}"
                ),
            }
        }
        assert_eq!(
            selected_script, expected_branch,
            "level group {level_group}"
        );
        assert!(hallway.script_object_commands.iter().any(|command| {
            command.command == "applymovement"
                && command.source_script == selected_script
                && command.movement.as_deref() == Some(expected_movement)
        }));
    }
}

#[test]
fn callasm_conditional_return_uses_preserved_cpu_flags() {
    let mut module = test_map_module("CpuReturnMap", "CPU_RETURN_MAP", None);
    module.scripts = BTreeMap::from([
        (
            "CpuReturnScript".to_string(),
            serde_json::json!([
                {"command": "callasm", "args": ["CheckCanDeletePhoneNumber"]},
                {"command": "end", "args": []}
            ]),
        ),
        (
            "CheckCanDeletePhoneNumber".to_string(),
            serde_json::json!([
                {"command": "ld", "args": ["a", "$5"]},
                {"command": "ld", "args": ["[wScriptVar]", "a"]},
                {"command": "cp", "args": ["$0"]},
                {"command": "ld", "args": ["a", "$7"]},
                {"command": "ret", "args": ["nz"]},
                {"command": "ld", "args": ["a", "$9"]},
                {"command": "ld", "args": ["[wScriptVar]", "a"]},
                {"command": "ret", "args": []}
            ]),
        ),
    ]);
    module.script_runtime_commands = vec![ScriptRuntimeCommand {
        command: "callasm".to_string(),
        args: vec!["CheckCanDeletePhoneNumber".to_string()],
        source_script: "CpuReturnScript".to_string(),
        command_index: 0,
    }];
    let events = module.events.clone();
    let data = GameDataSet {
        maps: [("CpuReturnMap".to_string(), module)].into_iter().collect(),
        ..GameDataSet::default()
    };
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "CpuReturnMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        events,
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let mut state = GameState::default();

    let (_, outcome) = data
        .apply_script_runtime_command_in_session(
            &mut state,
            &mut session,
            "CpuReturnMap",
            "CpuReturnScript",
            0,
            ScriptRuntimeInputs::default(),
        )
        .expect("execute conditional-return callasm");

    assert!(matches!(
        outcome,
        ScriptRuntimeOutcome::ScriptValueSet { ref value, .. } if value == "5"
    ));
    assert_eq!(state.script_runtime.script_value.as_deref(), Some("5"));
    assert!(state.script_runtime.command_queue.is_empty());
}

#[test]
fn phone_callasm_targets_retain_non_accumulator_engine_dispatch() {
    let root = repository_root_for_tests();
    let base = AssetRoot::new(root)
        .load_base_game_data()
        .expect("load base game data");
    let scripts = base
        .phone_scripts
        .iter()
        .flat_map(|payload| {
            payload
                .as_object()
                .expect("phone script payload object")
                .iter()
                .map(|(label, body)| (label.clone(), body.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let callasms = scripts
        .iter()
        .flat_map(|(source_script, body)| {
            body.as_array()
                .expect("phone script body array")
                .iter()
                .enumerate()
                .filter_map(|(command_index, command)| {
                    (command.get("command").and_then(serde_json::Value::as_str) == Some("callasm"))
                        .then(|| ScriptRuntimeCommand {
                            command: "callasm".to_string(),
                            args: command
                                .get("args")
                                .and_then(serde_json::Value::as_array)
                                .expect("phone callasm args")
                                .iter()
                                .map(|arg| {
                                    arg.as_str().expect("phone callasm string arg").to_string()
                                })
                                .collect(),
                            source_script: source_script.clone(),
                            command_index,
                        })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        callasms
            .iter()
            .filter_map(|command| command.args.first().cloned())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ".LoadBillScript".to_string(),
            ".LoadElmScript".to_string(),
            "HangUp".to_string(),
            "InitCallReceiveDelay".to_string(),
            "RingTwice_StartCall".to_string(),
        ])
    );

    let mut module = test_map_module("PhoneRuntime", "PHONE_RUNTIME", None);
    module.scripts = scripts;
    module.script_runtime_commands = callasms.clone();
    let events = module.events.clone();
    let data = GameDataSet {
        maps: [("PhoneRuntime".to_string(), module)].into_iter().collect(),
        ..GameDataSet::default()
    };
    let session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "PhoneRuntime".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        events,
        Vec::new(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );

    for command in callasms {
        let mut state = GameState::default();
        let mut session = session.clone();
        let (_, outcome) = data
            .apply_script_runtime_command_in_session(
                &mut state,
                &mut session,
                "PhoneRuntime",
                &command.source_script,
                command.command_index,
                ScriptRuntimeInputs::default(),
            )
            .expect("dispatch non-accumulator phone callasm");

        assert!(matches!(
            outcome,
            ScriptRuntimeOutcome::EffectRecorded {
                ref command,
                ..
            } if command == "callasm"
        ));
        assert_eq!(state.script_runtime.script_value, None);
        assert_eq!(state.script_runtime.command_queue.len(), 1);
        assert_eq!(
            state.script_runtime.command_queue[0].target,
            command.args[0]
        );
    }
}

#[test]
fn script_runtime_name_commands_write_pack_owned_display_buffers() {
    let mut module = test_map_module("RuntimeNameMap", "RUNTIME_NAME_MAP", None);
    module.script_runtime_commands = vec![
        ScriptRuntimeCommand {
            command: "gettrainername".to_string(),
            args: vec![
                "STRING_BUFFER_1".to_string(),
                "YOUNGSTER".to_string(),
                "YOUNGSTER_JOEY".to_string(),
            ],
            source_script: "RuntimeNameScript".to_string(),
            command_index: 0,
        },
        ScriptRuntimeCommand {
            command: "getitemname".to_string(),
            args: vec!["STRING_BUFFER_2".to_string(), "POTION".to_string()],
            source_script: "RuntimeNameScript".to_string(),
            command_index: 1,
        },
        ScriptRuntimeCommand {
            command: "getmonname".to_string(),
            args: vec![
                "STRING_BUFFER_3".to_string(),
                SCRIPT_RUNTIME_USE_SCRIPT_VAR_ID.to_string(),
            ],
            source_script: "RuntimeNameScript".to_string(),
            command_index: 2,
        },
    ];
    let mut species = species();
    species.id = "CHIKORITA".to_string();
    let data = GameDataSet {
        maps: [("RuntimeNameMap".to_string(), module.clone())]
            .into_iter()
            .collect(),
        items: [("POTION".to_string(), test_item("POTION"))]
            .into_iter()
            .collect(),
        pokemon: [("CHIKORITA".to_string(), species)].into_iter().collect(),
        trainers: TrainerCatalog {
            trainers: [(
                "YOUNGSTER_JOEY".to_string(),
                test_trainer("YOUNGSTER_JOEY", "MUSIC_YOUNGSTER_ENCOUNTER"),
            )]
            .into_iter()
            .collect(),
        },
        ..GameDataSet::default()
    };
    let mut state = GameState::default();
    state.script_runtime.script_value = Some("CHIKORITA".to_string());
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeNameMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![1],
        },
        module.events,
        module.objects,
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
            ],
        },
        TilePosition::new(0, 0),
    );

    for command_index in 0..=2 {
        data.apply_script_runtime_command_in_session(
            &mut state,
            &mut session,
            "RuntimeNameMap",
            "RuntimeNameScript",
            command_index,
            ScriptRuntimeInputs::default(),
        )
        .expect("runtime name command applies");
    }

    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_1")
            .map(String::as_str),
        Some("Joey")
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_2")
            .map(String::as_str),
        Some("POTION")
    );
    assert_eq!(
        state
            .script_runtime
            .named_buffers
            .get("STRING_BUFFER_3")
            .map(String::as_str),
        Some("CHIKORITA")
    );
}

#[test]
fn blackoutmod_updates_last_spawn_from_compiled_spawn_table() {
    let mut module = test_map_module("RuntimeBlackoutMap", "RUNTIME_BLACKOUT_MAP", None);
    module.script_runtime_commands = vec![ScriptRuntimeCommand {
        command: "blackoutmod".to_string(),
        args: vec!["ROUTE_29".to_string()],
        source_script: "RuntimeBlackoutScript".to_string(),
        command_index: 0,
    }];
    let mut spawn = test_runtime_spawn_point(15, "Route29");
    spawn.map_constant = "ROUTE_29".to_string();
    let data = GameDataSet {
        maps: [("RuntimeBlackoutMap".to_string(), module.clone())]
            .into_iter()
            .collect(),
        runtime_spawn_points: [("15".to_string(), spawn)].into_iter().collect(),
        ..GameDataSet::default()
    };
    let mut state = GameState {
        last_spawn_identifier: Some(0),
        ..GameState::default()
    };
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeBlackoutMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        module.events.clone(),
        module.objects.clone(),
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );

    data.apply_script_runtime_command_in_session(
        &mut state,
        &mut session,
        "RuntimeBlackoutMap",
        "RuntimeBlackoutScript",
        0,
        ScriptRuntimeInputs::default(),
    )
    .expect("blackoutmod applies against compiled spawn table");

    assert_eq!(
        state.script_runtime.blackout_mod.as_deref(),
        Some("ROUTE_29")
    );
    assert_eq!(state.last_spawn_identifier, Some(15));

    let mut missing_spawn_state = GameState {
        last_spawn_identifier: Some(0),
        ..GameState::default()
    };
    let before_missing = missing_spawn_state.clone();
    let mut missing_spawn_session = session.clone();
    let missing_spawn_data = GameDataSet {
        maps: [("RuntimeBlackoutMap".to_string(), module.clone())]
            .into_iter()
            .collect(),
        ..GameDataSet::default()
    };
    let error = missing_spawn_data
        .apply_script_runtime_command_in_session(
            &mut missing_spawn_state,
            &mut missing_spawn_session,
            "RuntimeBlackoutMap",
            "RuntimeBlackoutScript",
            0,
            ScriptRuntimeInputs::default(),
        )
        .expect_err("blackoutmod requires an exact compiled spawn target");
    assert!(
        error
            .to_string()
            .contains("compiled game pack missing spawn point for ROUTE_29"),
        "{error:#}"
    );
    assert_eq!(missing_spawn_state, before_missing);

    let mut first_spawn = test_runtime_spawn_point(15, "Route29");
    first_spawn.map_constant = "ROUTE_29".to_string();
    let mut second_spawn = test_runtime_spawn_point(16, "Route29");
    second_spawn.map_constant = "ROUTE_29".to_string();
    let ambiguous_data = GameDataSet {
        maps: [("RuntimeBlackoutMap".to_string(), module)]
            .into_iter()
            .collect(),
        runtime_spawn_points: [
            ("15".to_string(), first_spawn),
            ("16".to_string(), second_spawn),
        ]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    let mut ambiguous_state = GameState {
        last_spawn_identifier: Some(0),
        ..GameState::default()
    };
    let before_ambiguous = ambiguous_state.clone();
    let mut ambiguous_session = session;
    let error = ambiguous_data
        .apply_script_runtime_command_in_session(
            &mut ambiguous_state,
            &mut ambiguous_session,
            "RuntimeBlackoutMap",
            "RuntimeBlackoutScript",
            0,
            ScriptRuntimeInputs::default(),
        )
        .expect_err("blackoutmod rejects ambiguous compiled spawn target");
    assert!(
        error
            .to_string()
            .contains("compiled game pack has multiple spawn points for ROUTE_29: 15 and 16"),
        "{error:#}"
    );
    assert_eq!(ambiguous_state, before_ambiguous);
}

#[test]
fn consuming_map_music_requested_restores_current_map_music() {
    let mut module = test_map_module("RuntimeMusicMap", "RUNTIME_MUSIC_MAP", None);
    module.attributes.music = Some("MUSIC_RUNTIME_MAP".to_string());
    let data = GameDataSet {
        maps: [("RuntimeMusicMap".to_string(), module.clone())]
            .into_iter()
            .collect(),
        ..GameDataSet::default()
    };
    let mut state = GameState {
        script_runtime: ScriptRuntimeMemory {
            current_music: Some("MUSIC_SCRIPTED_TOUR".to_string()),
            map_music_requested: true,
            ..ScriptRuntimeMemory::default()
        },
        ..GameState::default()
    };
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "RuntimeMusicMap".to_string(),
            width: 1,
            height: 1,
            border_block: 0,
            connections: Vec::new(),
            metatile_ids: vec![0],
        },
        module.events,
        module.objects,
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        },
        TilePosition::new(0, 0),
    );
    let music_ids = BTreeSet::from(["MUSIC_RUNTIME_MAP".to_string()]);
    let empty_audio = BTreeSet::new();

    let outcome = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ConsumeScriptRuntimeFlag(RuntimeScriptRuntimeFlagCommand {
                flag: RuntimeScriptRuntimeFlag::MapMusicRequested,
            }),
            &music_ids,
            &empty_audio,
            &empty_audio,
        )
        .expect("consume map music request");

    assert!(matches!(
        outcome.result,
        RuntimeMutationResult::ScriptRuntimeFlagConsumed(
            RuntimeScriptRuntimeFlagValue::MapMusicRequested
        )
    ));
    assert!(!state.script_runtime.map_music_requested);
    assert_eq!(
        state.script_runtime.current_music.as_deref(),
        Some("MUSIC_RUNTIME_MAP")
    );

    let mut missing_music_state = GameState {
        script_runtime: ScriptRuntimeMemory {
            current_music: Some("MUSIC_SCRIPTED_TOUR".to_string()),
            map_music_requested: true,
            ..ScriptRuntimeMemory::default()
        },
        ..GameState::default()
    };
    let before_missing_music = missing_music_state.clone();
    let error = data
        .apply_runtime_mutation_command(
            &mut missing_music_state,
            &mut session,
            RuntimeMutationCommand::ConsumeScriptRuntimeFlag(RuntimeScriptRuntimeFlagCommand {
                flag: RuntimeScriptRuntimeFlag::MapMusicRequested,
            }),
            &BTreeSet::from(["MUSIC_OTHER_MAP".to_string()]),
            &empty_audio,
            &empty_audio,
        )
        .expect_err("missing compiled map music id rejects consumption");
    assert!(
            error.to_string().contains(
                "saved maps.RuntimeMusicMap.attributes.music MUSIC_RUNTIME_MAP is missing from compiled pack audio"
            ),
            "{error:#}"
        );
    assert_eq!(missing_music_state, before_missing_music);
}
