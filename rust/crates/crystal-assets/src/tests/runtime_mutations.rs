    #[test]
    fn runtime_bills_grandfather_commands_use_exact_input_mode() {
        let selected_party = RuntimeBillsGrandfatherCommand {
            party_index: Some(1),
            species_id: None,
        };
        assert_eq!(
            runtime_bills_grandfather_inputs(&selected_party).expect("party mode"),
            (Some(1), None)
        );

        let manual_species = RuntimeBillsGrandfatherCommand {
            party_index: None,
            species_id: Some("LICKITUNG".to_string()),
        };
        assert_eq!(
            runtime_bills_grandfather_inputs(&manual_species).expect("species mode"),
            (None, Some("LICKITUNG".to_string()))
        );

        let mixed = RuntimeBillsGrandfatherCommand {
            party_index: Some(1),
            species_id: Some("LICKITUNG".to_string()),
        };
        let mixed_error = runtime_bills_grandfather_inputs(&mixed)
            .expect_err("Bills Grandfather must have one exact input mode");
        assert!(
            format!("{mixed_error:#}").contains(
                "Bills Grandfather command must declare either party_index or species_id, not both"
            ),
            "{mixed_error:#}"
        );

        let missing = RuntimeBillsGrandfatherCommand {
            party_index: None,
            species_id: None,
        };
        let missing_error = runtime_bills_grandfather_inputs(&missing)
            .expect_err("Bills Grandfather must have an explicit input");
        assert!(
            format!("{missing_error:#}")
                .contains("Bills Grandfather command requires party_index or species_id"),
            "{missing_error:#}"
        );
    }

    #[test]
    fn dynamic_warp_requires_an_explicit_nonzero_backup_index() {
        let mut state = GameState::default();
        let missing = GameDataSet::required_dynamic_backup_warp_index(
            &state,
            2,
            "GoldenrodDeptStoreElevator",
        )
        .expect_err("missing backup warp must not become warp one");
        assert!(
            format!("{missing:#}").contains("has no saved nonzero backup warp"),
            "{missing:#}"
        );

        state.backup_warp_index = Some(0);
        GameDataSet::required_dynamic_backup_warp_index(
            &state,
            2,
            "GoldenrodDeptStoreElevator",
        )
        .expect_err("zero backup warp must not become warp one");

        state.backup_warp_index = Some(3);
        assert_eq!(
            GameDataSet::required_dynamic_backup_warp_index(
                &state,
                2,
                "GoldenrodDeptStoreElevator",
            )
            .expect("explicit backup warp"),
            3
        );
    }

    #[test]
    fn runtime_battle_tower_action_commands_use_exact_payloads() {
        let save_level_group = RuntimeBattleTowerActionCommand {
            action: "BATTLETOWERACTION_SAVELEVELGROUP".to_string(),
            level_group: Some(5),
            selected_reward: None,
        };
        assert_eq!(
            runtime_battle_tower_action_inputs(&save_level_group).expect("save level group"),
            (Some(5), None)
        );

        let missing_level_group = RuntimeBattleTowerActionCommand {
            action: "BATTLETOWERACTION_SAVELEVELGROUP".to_string(),
            level_group: None,
            selected_reward: None,
        };
        let missing_level_error = runtime_battle_tower_action_inputs(&missing_level_group)
            .expect_err("SAVELEVELGROUP must carry level_group");
        assert!(
            format!("{missing_level_error:#}")
                .contains("Battle Tower SAVELEVELGROUP command requires level_group"),
            "{missing_level_error:#}"
        );

        let save_options = RuntimeBattleTowerActionCommand {
            action: "BATTLETOWERACTION_SAVEOPTIONS".to_string(),
            level_group: None,
            selected_reward: Some("HP_UP".to_string()),
        };
        assert_eq!(
            runtime_battle_tower_action_inputs(&save_options).expect("save options"),
            (None, Some("HP_UP".to_string()))
        );

        let missing_reward = RuntimeBattleTowerActionCommand {
            action: "BATTLETOWERACTION_SAVEOPTIONS".to_string(),
            level_group: None,
            selected_reward: None,
        };
        let missing_reward_error = runtime_battle_tower_action_inputs(&missing_reward)
            .expect_err("SAVEOPTIONS must carry selected_reward");
        assert!(
            format!("{missing_reward_error:#}")
                .contains("Battle Tower SAVEOPTIONS command requires selected_reward"),
            "{missing_reward_error:#}"
        );

        for command in [
            RuntimeBattleTowerActionCommand {
                action: "BATTLETOWERACTION_SAVEOPTIONS".to_string(),
                level_group: Some(5),
                selected_reward: Some("HP_UP".to_string()),
            },
            RuntimeBattleTowerActionCommand {
                action: "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
                level_group: Some(5),
                selected_reward: None,
            },
        ] {
            let error = runtime_battle_tower_action_inputs(&command)
                .expect_err("level_group must only appear on SAVELEVELGROUP");
            assert!(
                format!("{error:#}").contains(&format!(
                    "Battle Tower {} command must not declare level_group",
                    command.action
                )),
                "{error:#}"
            );
        }

        let unused_reward = RuntimeBattleTowerActionCommand {
            action: "BATTLETOWERACTION_SET_EXPLANATION_READ".to_string(),
            level_group: None,
            selected_reward: Some("HP_UP".to_string()),
        };
        let unused_reward_error = runtime_battle_tower_action_inputs(&unused_reward)
            .expect_err("selected_reward must only appear on SAVEOPTIONS");
        assert!(
            format!("{unused_reward_error:#}").contains(
                "Battle Tower BATTLETOWERACTION_SET_EXPLANATION_READ command must not declare selected_reward"
            ),
            "{unused_reward_error:#}"
        );
    }

    #[test]
    fn runtime_map_radio_command_requires_exact_station_token() {
        let command = RuntimeMapRadioCommand {
            station: "MAPRADIO_UNOWN".to_string(),
        };
        assert_eq!(
            runtime_map_radio_station(&command).expect("exact radio station"),
            "MAPRADIO_UNOWN"
        );

        for station in ["", " MAPRADIO_UNOWN", "MAPRADIO UNOWN", "fallback_radio"] {
            let command = RuntimeMapRadioCommand {
                station: station.to_string(),
            };
            let error = runtime_map_radio_station(&command)
                .expect_err("station must be an exact pack token");
            assert!(
                format!("{error:#}").contains("MapRadio station"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn runtime_special_cry_command_requires_exact_species_token() {
        let command = RuntimeSpecialCryCommand {
            species_id: "CHIKORITA".to_string(),
        };
        assert_eq!(
            runtime_special_cry_species(&command).expect("exact cry species"),
            "CHIKORITA"
        );

        for species_id in ["", " CHIKORITA", "CHIKO RITA", "legacy_mon"] {
            let command = RuntimeSpecialCryCommand {
                species_id: species_id.to_string(),
            };
            let error = runtime_special_cry_species(&command)
                .expect_err("special cry species must be an exact pack token");
            assert!(
                format!("{error:#}").contains("special cry species id"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn playability_components_use_exact_runtime_movement_steps() {
        let mut module = test_map_module("ExactStepMap", "EXACT_STEP_MAP", None);
        module.attributes.height = 2;
        module.blocks = vec![1, 1];
        let data = GameDataSet {
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut diagnostics = Vec::new();

        let context = map_playability_context_from_parts(
            &data,
            &module.id,
            &module.attributes,
            module.blocks,
            &PlayabilityRules::default(),
            &mut diagnostics,
        )
        .expect("playability context");

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert!(context.component_at(TilePosition::new(0, 0)).is_some());
        assert_eq!(
            context.component_at(TilePosition::new(0, 0)),
            context.component_at(TilePosition::new(1, 0)),
            "adjacent runtime tiles are valid standing tiles under stride-one movement"
        );
        assert_eq!(
            context.component_at(TilePosition::new(0, 0)),
            context.component_at(TilePosition::new(0, 2)),
            "tiles reachable through repeated exact runtime movement steps stay in the same component"
        );
    }

    #[test]
    fn playability_components_ignore_walkable_odd_quadrants_as_player_standing_tiles() {
        let module = test_map_module("OddQuadrantMap", "ODD_QUADRANT_MAP", None);
        let mut tileset = test_tileset_definition();
        tileset.collision.insert(
            "1".to_string(),
            vec![
                "WALL".to_string(),
                "FLOOR".to_string(),
                "WALL".to_string(),
                "WALL".to_string(),
            ],
        );
        let data = GameDataSet {
            tilesets: BTreeMap::from([("johto".to_string(), tileset)]),
            ..GameDataSet::default()
        };
        let mut diagnostics = Vec::new();

        let context = map_playability_context_from_parts(
            &data,
            &module.id,
            &module.attributes,
            module.blocks,
            &PlayabilityRules::default(),
            &mut diagnostics,
        )
        .expect("playability context");

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(context.component_count, 1);
        assert_eq!(context.component_at(TilePosition::new(0, 0)), None);
        assert_eq!(context.component_at(TilePosition::new(1, 0)), Some(0));
    }

    #[test]
    fn playability_context_reports_runtime_tile_bounds_overflow_without_component_scan() {
        let mut module = test_map_module("HugeStrideMap", "HUGE_STRIDE_MAP", None);
        module.attributes.width = u16::MAX;
        module.attributes.height = 1;
        module.blocks = vec![1; u16::MAX as usize];
        let data = GameDataSet {
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut diagnostics = Vec::new();

        let context = map_playability_context_from_parts(
            &data,
            &module.id,
            &module.attributes,
            module.blocks,
            &PlayabilityRules::default(),
            &mut diagnostics,
        );

        assert!(context.is_none());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "map_runtime_tile_bounds_overflow"
                && diagnostic.subject == "HugeStrideMap"
        }));
    }

    #[test]
    fn playability_context_rejects_u16_bounds_that_overflow_i16_runtime_tiles() {
        let mut module = test_map_module("I16OverflowStrideMap", "I16_OVERFLOW_STRIDE_MAP", None);
        module.attributes.width = 20_000;
        module.attributes.height = 1;
        module.blocks = vec![1; 20_000];
        let data = GameDataSet {
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut diagnostics = Vec::new();

        let context = map_playability_context_from_parts(
            &data,
            &module.id,
            &module.attributes,
            module.blocks,
            &PlayabilityRules::default(),
            &mut diagnostics,
        );

        assert!(context.is_none());
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "map_runtime_tile_bounds_overflow"
                && diagnostic.subject == "I16OverflowStrideMap"
        }));
    }

    #[test]
    fn connection_playability_source_tile_uses_exact_runtime_border() {
        let mut module = test_map_module("ExactConnectionMap", "EXACT_CONNECTION_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut diagnostics = Vec::new();
        let context = map_playability_context_from_parts(
            &data,
            &module.id,
            &module.attributes,
            module.blocks,
            &PlayabilityRules::default(),
            &mut diagnostics,
        )
        .unwrap_or_else(|| panic!("playability context diagnostics: {diagnostics:#?}"));
        let connection = MapConnection {
            direction: "east".to_string(),
            target_map: "TargetMap".to_string(),
            offset: 0,
        };

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(
            connection_source_tile(&context, &connection),
            Some(TilePosition::new(3, 0))
        );
        assert_eq!(
            connection_trigger_tile(&context, &connection),
            Some(TilePosition::new(4, 0))
        );
        let east_boundary =
            connection_source_boundary_tiles(4, 4, PLAYABILITY_RUNTIME_TILE_STRIDE, "east");
        assert_eq!(
            east_boundary,
            vec![
                TilePosition::new(3, 0),
                TilePosition::new(3, 1),
                TilePosition::new(3, 2),
                TilePosition::new(3, 3),
            ]
        );
        assert!(
            connection_source_boundary_tiles(u16::MAX, 4, PLAYABILITY_RUNTIME_TILE_STRIDE, "east")
                .is_empty()
        );
        assert_eq!(
            connection_source_boundary_tiles(1, 4, PLAYABILITY_RUNTIME_TILE_STRIDE, "east"),
            vec![
                TilePosition::new(0, 0),
                TilePosition::new(0, 1),
                TilePosition::new(0, 2),
                TilePosition::new(0, 3),
            ]
        );
    }

    #[test]
    fn connection_destinations_land_on_runtime_target_edge_tiles() {
        let mut attributes = test_map_module("TargetMap", "TARGET_MAP", None).attributes;
        attributes.width = 10;
        attributes.height = 9;

        assert_eq!(
            connection_destination_tile(TilePosition::new(6, -2), "north", 0, &attributes)
                .expect("north connection"),
            TilePosition::new(6, 16)
        );
        assert_eq!(
            connection_destination_tile(TilePosition::new(6, 18), "south", 0, &attributes)
                .expect("south connection"),
            TilePosition::new(6, 0)
        );
        assert_eq!(
            connection_destination_tile(TilePosition::new(-2, 4), "west", 0, &attributes)
                .expect("west connection"),
            TilePosition::new(18, 4)
        );
        assert_eq!(
            connection_destination_tile(TilePosition::new(20, 4), "east", 0, &attributes)
                .expect("east connection"),
            TilePosition::new(0, 4)
        );
    }

    #[test]
    fn connection_destination_rejects_offsets_that_overflow_runtime_tile_space() {
        let attributes = test_map_module("TargetMap", "TARGET_MAP", None).attributes;

        let error =
            connection_destination_tile(TilePosition::new(6, -2), "north", i32::MIN, &attributes)
                .expect_err("connection offset overflow must be rejected");

        assert!(
            format!("{error:#}")
                .contains("connection offset -2147483648 overflows runtime tile space"),
            "{error:#}"
        );
    }

    fn assert_map_module_requires_field(field: &'static str) {
        let module = test_map_module("NewRoute", "NEW_ROUTE", None);
        let mut json = serde_json::to_value(module).expect("serialize full map module");
        json.as_object_mut()
            .expect("map module json object")
            .remove(field)
            .unwrap_or_else(|| panic!("fixture must include {field}"));

        let error = serde_json::from_value::<MapModule>(json)
            .expect_err("map module fields must be explicit, even when empty")
            .to_string();
        let expected = format!("missing field `{field}`");
        assert!(error.contains(&expected), "{error}");
    }

    #[test]
    fn map_module_json_requires_explicit_script_sections() {
        assert_map_module_requires_field("scripts");
        assert_map_module_requires_field("trainer_scripts");
        assert_map_module_requires_field("scripted_trainer_battles");
        assert_map_module_requires_field("script_vertical_menus");
        assert_map_module_requires_field("script_elevators");
        assert_map_module_requires_field("script_field_pickups");
        assert_map_module_requires_field("script_shop_commands");
        assert_map_module_requires_field("script_phone_commands");
        assert_map_module_requires_field("script_runtime_commands");
        assert_map_module_requires_field("script_swarm_commands");
        assert_map_module_requires_field("map_script_section_commands");
        assert_map_module_requires_field("map_event_section_commands");
    }

    #[test]
    fn map_module_json_rejects_unknown_nested_script_command_fields() {
        let mut module = test_map_module("NewRoute", "NEW_ROUTE", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic".to_string(),
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "NewRouteScript".to_string(),
            command_index: 0,
        }];
        let mut json = serde_json::to_value(module).expect("serialize full map module");
        let command = json["script_audio_commands"]
            .as_array_mut()
            .expect("audio commands")
            .first_mut()
            .expect("first audio command")
            .as_object_mut()
            .expect("audio command object");
        command.insert(
            "mp3".to_string(),
            Value::String("music/route29.mp3".to_string()),
        );

        let error = serde_json::from_value::<MapModule>(json)
            .expect_err("nested script command fields must be definitive")
            .to_string();
        assert!(error.contains("unknown field `mp3`"), "{error}");
    }

    fn test_object(object_id: &str, event_flag: &str, x: u16, y: u16) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x,
            y,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "ObjectScript".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(object_id.to_string()),
            sightline_direction_override: None,
        }
    }

    #[test]
    fn overworld_input_keeps_directional_movement_and_a_interaction_separate() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.blocks = vec![1, 1];
        module.objects = vec![test_object("ROUTE29_TEACHER", "-1", 1, 0)];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("overworld session");
        session.player.facing = Direction::Right;

        let directed = data
            .apply_overworld_input(
                &mut state,
                &mut session,
                [GameButton::A, GameButton::Right],
                &BTreeSet::new(),
            )
            .expect("directed input");

        assert_eq!(directed.movement, None);
        assert_eq!(
            directed.interaction.as_ref().map(|interaction| interaction.script.as_str()),
            Some("ObjectScript")
        );

        let mut a_only_state = GameState::default();
        let mut a_only_session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("a-only overworld session");
        a_only_session.player.facing = Direction::Right;
        let a_only = data
            .apply_overworld_input(
                &mut a_only_state,
                &mut a_only_session,
                [GameButton::A],
                &BTreeSet::new(),
            )
            .expect("a-only input");

        assert_eq!(
            a_only
                .interaction
                .as_ref()
                .map(|interaction| interaction.script.as_str()),
            Some("ObjectScript")
        );
    }

    #[test]
    fn empty_overworld_frame_advances_without_cloning_or_mutating_gameplay_state() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.blocks = vec![1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("overworld session");
        let before_joypad = state.joypad.clone();

        let frame = data
            .apply_overworld_input(
                &mut state,
                &mut session,
                std::iter::empty(),
                &BTreeSet::new(),
            )
            .expect("idle frame");

        assert_eq!(frame.snapshot.frame, 1);
        assert_eq!(frame.input_mask, 0);
        assert_eq!(frame.pressed_mask, 0);
        assert!(!frame.autonomous_objects_changed);
        assert_eq!(state.frame_counter, 1);
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(state.joypad, before_joypad);
    }

    #[test]
    fn empty_overworld_frame_does_not_bypass_forced_tile_movement_without_npcs() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.blocks = vec![1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("overworld session");
        for metatile in &mut session.tileset.metatiles {
            metatile.collision = [
                crystal_core::world::collision::permissions::CURRENT_RIGHT;
                4
            ];
        }

        let frame = data
            .apply_overworld_input(
                &mut state,
                &mut session,
                std::iter::empty(),
                &BTreeSet::new(),
            )
            .expect("forced-current frame");

        assert_eq!(session.player.tile, TilePosition::new(1, 0));
        assert!(matches!(
            frame.movement,
            Some(StepOutcome::Moved {
                from: TilePosition { x: 0, y: 0 },
                to: TilePosition { x: 1, y: 0 },
                speed_multiplier: 1,
            })
        ));
    }

    #[test]
    fn overworld_input_does_not_move_or_interact_while_script_runtime_is_blocking() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.blocks = vec![1, 1];
        module.objects = vec![test_object("ROUTE29_TEACHER", "-1", 1, 0)];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        state.script_runtime.next_script = Some(ScriptLocation {
            origin_map_name: "Route29".to_string(),
            script: "BlockingScript".to_string(),
        });
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("overworld session");
        session.player.facing = Direction::Right;

        let frame = data
            .apply_overworld_input(
                &mut state,
                &mut session,
                [GameButton::A, GameButton::Right],
                &BTreeSet::new(),
            )
            .expect("locked overworld input");

        assert_eq!(
            frame.input_mask & (B_PAD_A | crystal_core::input::B_PAD_RIGHT),
            B_PAD_A | crystal_core::input::B_PAD_RIGHT
        );
        assert_eq!(frame.movement, None);
        assert_eq!(frame.interaction, None);
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 1);
        assert_eq!(
            state
                .script_runtime
                .next_script
                .as_ref()
                .map(|location| location.script.as_str()),
            Some("BlockingScript")
        );

        let idle_locked = data
            .apply_overworld_input(
                &mut state,
                &mut session,
                std::iter::empty(),
                &BTreeSet::new(),
            )
            .expect("locked idle frame");
        assert_eq!(idle_locked.snapshot.frame, 2);
        assert_eq!(idle_locked.movement, None);
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(
            state
                .script_runtime
                .next_script
                .as_ref()
                .map(|location| location.script.as_str()),
            Some("BlockingScript")
        );
    }

    #[test]
    fn overworld_input_rejects_invalid_pack_background_event_coordinates() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![0; 4];
        module.events.bg_events = vec![BackgroundEvent {
            x: u16::MAX,
            y: 0,
            event_type: "BGEVENT_READ".to_string(),
            script: "BrokenSignpostScript".to_string(),
        }];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("overworld session");
        session.player.facing = Direction::Right;

        let error = data
            .apply_overworld_input(&mut state, &mut session, [GameButton::A], &BTreeSet::new())
            .expect_err("invalid background event coordinate must reject pack-backed input");
        let error = format!("{error:#}");

        assert!(
            error.contains("check overworld interaction on Route29"),
            "{error}"
        );
        assert!(
            error.contains("background event 'BrokenSignpostScript' has out-of-range"),
            "{error}"
        );
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn overworld_input_rejects_invalid_pack_coord_event_without_committing_staged_movement() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![0; 4];
        module.events.coord_events = vec![CoordEvent {
            x: u16::MAX,
            y: 0,
            scene_id: String::new(),
            script_name: "BrokenCoordScript".to_string(),
        }];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("overworld session");
        session.player.facing = Direction::Right;

        let error = data
            .apply_overworld_input(
                &mut state,
                &mut session,
                [GameButton::Right],
                &BTreeSet::new(),
            )
            .expect_err("invalid coord event coordinate must reject pack-backed input");
        let error = format!("{error:#}");

        assert!(error.contains("check coord event on Route29"), "{error}");
        assert!(
            error.contains("coord event 'BrokenCoordScript' has out-of-range"),
            "{error}"
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn overworld_session_rejects_player_tiles_outside_runtime_bounds() {
        let mut module = test_map_module("RuntimeMap", "RUNTIME_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![0; 4];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };

        let session = data
            .overworld_session("RuntimeMap", TilePosition::new(2, 2), 0)
            .expect("runtime edge tile inside 4x4 bounds");
        assert_eq!(session.player.tile, TilePosition::new(2, 2));

        let negative = data
            .overworld_session("RuntimeMap", TilePosition::new(-1, 0), 0)
            .expect_err("negative runtime tile must fail session construction");
        assert!(
            format!("{negative:#}").contains(
                "runtime player tile (-1, 0) is outside compiled map RuntimeMap runtime tile bounds 4x4"
            ),
            "{negative:#}"
        );

        let overflow = data
            .overworld_session("RuntimeMap", TilePosition::new(4, 0), 0)
            .expect_err("runtime tile at exclusive bound must fail session construction");
        assert!(
            format!("{overflow:#}").contains(
                "runtime player tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds 4x4"
            ),
            "{overflow:#}"
        );
    }

    #[test]
    fn overworld_session_passability_uses_requested_traversal_state() {
        let mut module = test_map_module("RuntimeMap", "RUNTIME_MAP", None);
        module.attributes.width = 1;
        module.attributes.height = 1;
        module.blocks = vec![0];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([(
                "johto".to_string(),
                TilesetDefinition {
                    collision: [(
                        "0".to_string(),
                        vec![
                            "WATER".to_string(),
                            "WATER".to_string(),
                            "WATER".to_string(),
                            "WATER".to_string(),
                        ],
                    )]
                    .into_iter()
                    .collect(),
                    palette_map: vec![0],
                },
            )]),
            ..GameDataSet::default()
        };

        let walk_error = data
            .overworld_session("RuntimeMap", TilePosition::new(0, 0), 0)
            .expect_err("walking session must reject water tile")
            .to_string();
        assert!(
            walk_error
                .contains("runtime player tile (0, 0) is not walkable on compiled map RuntimeMap"),
            "{walk_error}"
        );

        let surf_session = data
            .overworld_session_for_traversal(
                "RuntimeMap",
                TilePosition::new(0, 0),
                7,
                PlayerTraversalState::Surf,
            )
            .expect("surf traversal accepts water tile");
        assert_eq!(surf_session.player.tile, TilePosition::new(0, 0));
        assert_eq!(surf_session.frame, 7);
    }

    #[test]
    fn pending_script_warp_uses_explicit_normal_traversal() {
        let mut source = test_map_module("SourceMap", "SOURCE_MAP", None);
        source.blocks = vec![0];
        let mut target = test_map_module("WaterMap", "WATER_MAP", None);
        target.blocks = vec![1];
        let data = GameDataSet {
            maps: map_payload(vec![source, target]),
            tilesets: BTreeMap::from([(
                "johto".to_string(),
                TilesetDefinition {
                    collision: [
                        (
                            "0".to_string(),
                            vec![
                                "FLOOR".to_string(),
                                "FLOOR".to_string(),
                                "FLOOR".to_string(),
                                "FLOOR".to_string(),
                            ],
                        ),
                        (
                            "1".to_string(),
                            vec![
                                "WATER".to_string(),
                                "WATER".to_string(),
                                "WATER".to_string(),
                                "WATER".to_string(),
                            ],
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    palette_map: vec![0],
                },
            )]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(ScriptWarpRequest {
            target_map: "WaterMap".to_string(),
            tile: TilePosition::new(0, 0),
            facing: Some(Direction::Right),
            source_script: "RuntimeWarpScript".to_string(),
            command_index: 0,
        });
        let mut session = data
            .overworld_session("SourceMap", TilePosition::new(0, 0), 12)
            .expect("source session");
        session.player.mode = MovementMode::Surf;

        let error = data
            .transition_pending_script_warp(&mut state, &mut session, &BTreeSet::new())
            .expect_err("pending script warp does not inherit surf traversal");

        assert!(
            format!("{error:#}")
                .contains("runtime player tile (0, 0) is not walkable on compiled map WaterMap"),
            "{error:#}"
        );
        assert_eq!(session.map.name, "SourceMap");
        assert_eq!(session.player.mode, MovementMode::Surf);
        assert!(state.script_runtime.pending_script_warp.is_some());
    }

    #[test]
    fn field_block_target_in_front_uses_runtime_tile_then_metatile_lookup() {
        let mut module = test_map_module("RuntimeMap", "RUNTIME_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 1;
        module.blocks = vec![0, 0];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut session = data
            .overworld_session("RuntimeMap", TilePosition::new(0, 0), 0)
            .expect("session on runtime tile");
        session.player.facing = Direction::Right;

        assert_eq!(
            data.field_block_target_metatile_in_front(&session)
                .expect("target metatile"),
            (0, 0)
        );

        session.player.tile = TilePosition::new(0, 0);
        session.player.facing = Direction::Left;
        let error = data
            .field_block_target_metatile_in_front(&session)
            .expect_err("target before map must reject");
        assert!(
            format!("{error:#}").contains(
                "field block target tile (-1, 0) is outside map RuntimeMap runtime tile bounds 4x2"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn runtime_jumptextfaceplayer_turns_last_talked_object_before_text_wait() {
        let mut module = test_map_module("RuntimeMap", "RUNTIME_MAP", None);
        module.script_text_bodies.insert(
            "RuntimeGreetingText".to_string(),
            ScriptTextBody {
                label: "RuntimeGreetingText".to_string(),
                commands: Vec::new(),
            },
        );
        module.script_text_commands = vec![ScriptTextCommand {
            command: "jumptextfaceplayer".to_string(),
            text_label: Some("RuntimeGreetingText".to_string()),
            source_script: "RuntimeNpcScript".to_string(),
            command_index: 0,
        }];
        let mut npc = test_object("RUNTIME_NPC", "-1", 0, 1);
        npc.script = "RuntimeNpcScript".to_string();
        module.objects = vec![npc.clone()];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeMap".to_string(),
                width: 1,
                height: 2,
                border_block: 0,
                connections: Vec::new(),
                metatile_ids: vec![0, 0],
            },
            MapEvents::default(),
            vec![npc],
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                }],
            },
            TilePosition::new(0, 0),
        );
        session.last_talked_object_identifier = Some("RUNTIME_NPC".to_string());

        let action = data
            .apply_script_text_command_in_session(
                &mut state,
                &mut session,
                "RuntimeMap",
                "RuntimeNpcScript",
                0,
            )
            .expect("jumptextfaceplayer applies");

        assert!(matches!(
            action,
            ScriptTextAction::Write {
                face_player: true,
                closes_text: true,
                ..
            }
        ));
        assert_eq!(
            session.object_facings.get("RUNTIME_NPC"),
            Some(&Direction::Up)
        );
        assert_eq!(state.script_runtime.text_events.len(), 1);
        assert!(state.script_runtime.text_events[0].face_player);
        assert_eq!(
            state.script_runtime.pending_text_wait.as_ref().map(|wait| {
                (
                    wait.source_script.as_str(),
                    wait.command_index,
                    wait.command.as_str(),
                )
            }),
            Some(("RuntimeNpcScript", 0, "jumptextfaceplayer"))
        );
    }

    fn temp_test_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "crystal-assets-{}-{unique}-{name}",
            std::process::id()
        ))
    }

    #[test]
    fn verifier_rejects_unknown_object_movement_data_without_direction_fallback() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts = BTreeMap::from([("ObjectScript".to_string(), Value::Array(Vec::new()))]);
        let mut object = test_object("START_OBJECT", "EVENT_START_OBJECT", 0, 0);
        object.spritemovedata = "spritemovedata_standing_down".to_string();
        let mut malformed = test_object("START_MALFORMED_OBJECT", "EVENT_START_MALFORMED", 1, 0);
        malformed.spritemovedata = "SPRITEMOVEDATA STANDING_DOWN".to_string();
        module.objects = vec![object, malformed];
        let mut middle = test_map_module("Middle", "MIDDLE_MAP", None);
        middle.attributes.width = 2;
        middle.blocks = vec![1, 1];
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
            diagnostic.code == "unknown_object_movement_data"
                && diagnostic.subject == "Start:START_OBJECT"
                && diagnostic.message.contains("spritemovedata_standing_down")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_object_movement_data"
                && diagnostic.subject == "Start:START_MALFORMED_OBJECT"
                && diagnostic.message.contains("SPRITEMOVEDATA STANDING_DOWN")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_object_movement_data"
                && diagnostic.subject == "Start:START_MALFORMED_OBJECT"
        }));
    }

    #[test]
    fn verifier_rejects_object_types_not_implemented_by_rust_runtime() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts = BTreeMap::from([("ObjectScript".to_string(), Value::Array(Vec::new()))]);
        let exact = test_object("START_OBJECT", "EVENT_START_OBJECT", 0, 0);
        let mut malformed = test_object("START_MALFORMED_OBJECT", "EVENT_START_MALFORMED", 1, 0);
        malformed.object_type = "OBJECTTYPE SCRIPT".to_string();
        let mut unsupported =
            test_object("START_UNSUPPORTED_OBJECT", "EVENT_START_UNSUPPORTED", 2, 0);
        unsupported.object_type = "OBJECTTYPE_MODDED".to_string();
        module.objects = vec![exact, malformed, unsupported];
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
            diagnostic.code == "invalid_object_type"
                && diagnostic.subject == "Start:START_MALFORMED_OBJECT"
                && diagnostic.message.contains("OBJECTTYPE SCRIPT")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsupported_object_type"
                && diagnostic.subject == "Start:START_UNSUPPORTED_OBJECT"
                && diagnostic.message.contains("OBJECTTYPE_MODDED")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsupported_object_type"
                && diagnostic.subject == "Start:START_OBJECT"
        }));
    }

    fn empty_content_pack_files_json() -> serde_json::Map<String, Value> {
        let mut json = serde_json::Map::new();
        for category in CONTENT_PACK_CATEGORIES {
            json.insert(category.as_str().to_string(), Value::Array(Vec::new()));
        }
        json
    }

    fn content_pack_json(id: &str, enabled: bool, priority: i32) -> Value {
        serde_json::json!({
            "id": id,
            "enabled": enabled,
            "priority": priority,
            "path": format!("content-packs/{id}"),
            "compiled": null,
            "files": Value::Object(empty_content_pack_files_json()),
        })
    }

    #[test]
    fn content_pack_index_requires_explicit_pack_metadata_and_sorts_enabled_packs() {
        let index: ContentPackIndex = serde_json::from_value(serde_json::json!({
          "version": 1,
          "packs": [
            content_pack_json("late", true, 10),
            content_pack_json("disabled", false, -100),
            content_pack_json("early", true, -10)
          ]
        }))
        .expect("parse content pack index");

        assert_eq!(index.version, 1);
        index.validate().expect("valid content pack index");
        let ids: Vec<&str> = index
            .enabled_packs_sorted()
            .into_iter()
            .map(|pack| pack.id.as_str())
            .collect();
        assert_eq!(ids, vec!["early", "late"]);

        let missing_version = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
            "packs": []
        }))
        .expect_err("content pack index version must be explicit")
        .to_string();
        assert!(
            missing_version.contains("missing field `version`"),
            "{missing_version}"
        );

        let unsupported_version = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
            "version": 2,
            "packs": []
        }))
        .expect("parse unsupported version content pack index")
        .validate()
        .expect_err("content pack index version must be exact")
        .to_string();
        assert!(
            unsupported_version.contains("version 2 is unsupported; expected 1"),
            "{unsupported_version}"
        );

        let mut missing_compiled = content_pack_json("missing-compiled", true, 0);
        missing_compiled
            .as_object_mut()
            .expect("pack object")
            .remove("compiled");
        let missing_compiled = serde_json::from_value::<ContentPack>(missing_compiled)
            .expect_err("content pack compiled field must be explicit, even when null")
            .to_string();
        assert!(
            missing_compiled.contains("missing field `compiled`"),
            "{missing_compiled}"
        );

        let duplicate_ids = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [
            content_pack_json("same", true, 0),
            content_pack_json("same", true, 1)
          ]
        }))
        .expect("parse duplicate id content pack index")
        .validate()
        .expect_err("content pack index must reject duplicate pack ids")
        .to_string();
        assert!(
            duplicate_ids.contains("duplicate pack id 'same'"),
            "{duplicate_ids}"
        );

        let mut wrong_path = content_pack_json("strict-pack", true, 0);
        wrong_path.as_object_mut().expect("pack object").insert(
            "path".to_string(),
            Value::String("content-packs/other".to_string()),
        );
        let wrong_path = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [wrong_path]
        }))
        .expect("parse wrong path content pack index")
        .validate()
        .expect_err("content pack path must be canonical for its id")
        .to_string();
        assert!(
            wrong_path.contains("must be exactly content-packs/strict-pack"),
            "{wrong_path}"
        );

        for malformed_id in [
            "",
            " padded",
            "padded ",
            "nested/pack",
            "joined+pack",
            "space pack",
        ] {
            let error = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
              "version": 1,
              "packs": [content_pack_json(malformed_id, true, 0)]
            }))
            .expect("parse malformed id content pack index")
            .validate()
            .expect_err("content pack ids must be exact path-safe tokens")
            .to_string();
            assert!(
                error.contains(
                    "must be exact ASCII letters, numbers, underscores, hyphens, or dots"
                ),
                "unexpected error for {malformed_id:?}: {error}"
            );
        }

        serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [content_pack_json("core.mod-pack_1", true, 0)]
        }))
        .expect("parse dotted id content pack index")
        .validate()
        .expect("dotted exact pack ids are valid");

        let mut mixed_source = content_pack_json("mixed-source", true, 0);
        mixed_source.as_object_mut().expect("pack object").insert(
            "compiled".to_string(),
            Value::String("content-packs/mixed-source/compiled.json".to_string()),
        );
        mixed_source
            .get_mut("files")
            .and_then(Value::as_object_mut)
            .expect("files object")
            .insert(
                "pokemon".to_string(),
                serde_json::json!(["content-packs/mixed-source/pokemon.json"]),
            );
        let mixed_source = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [mixed_source]
        }))
        .expect("parse mixed source content pack index")
        .validate()
        .expect_err("compiled game pack entries must not also declare raw files")
        .to_string();
        assert!(
            mixed_source.contains(
                "content pack mixed-source declares compiled content and raw pokemon file entry content-packs/mixed-source/pokemon.json; choose one source"
            ),
            "{mixed_source}"
        );

        let mut compiled = content_pack_json("compiled", true, 0);
        compiled.as_object_mut().expect("pack object").insert(
            "compiled".to_string(),
            Value::String("content-packs/compiled/core.crystalpack".to_string()),
        );
        let enabled_raw = content_pack_json("raw", true, 1);
        let mixed_enabled = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [compiled.clone(), enabled_raw]
        }))
        .expect("parse compiled plus raw index")
        .validate()
        .expect_err("compiled game pack must be definitive among enabled sources")
        .to_string();
        assert!(
            mixed_enabled.contains(
                "content pack index compiled game pack 'compiled' must be the only enabled content source"
            ),
            "{mixed_enabled}"
        );

        let disabled_raw = content_pack_json("raw", false, 1);
        serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [compiled.clone(), disabled_raw]
        }))
        .expect("parse compiled plus disabled raw index")
        .validate()
        .expect("disabled raw packs do not make the compiled runtime pack ambiguous");

        let mut second_compiled = content_pack_json("other-compiled", true, 1);
        second_compiled
            .as_object_mut()
            .expect("pack object")
            .insert(
                "compiled".to_string(),
                Value::String("content-packs/other-compiled/core.crystalpack".to_string()),
            );
        let multiple_compiled = serde_json::from_value::<ContentPackIndex>(serde_json::json!({
          "version": 1,
          "packs": [compiled, second_compiled]
        }))
        .expect("parse multiple compiled pack index")
        .validate()
        .expect_err("multiple compiled game packs are not a deterministic source");
        assert!(
            multiple_compiled.to_string().contains(
                "content pack index enables multiple compiled game packs: compiled, other-compiled"
            ),
            "{multiple_compiled}"
        );
    }

    #[test]
    fn content_pack_files_keep_existing_json_categories_and_add_game_asset_categories() {
        let mut json = empty_content_pack_files_json();
        json.insert(
            "pokemon".to_string(),
            serde_json::json!(["mods/new/pokemon.json"]),
        );
        json.insert(
            "map_attributes".to_string(),
            serde_json::json!(["mods/new/map_attributes.json"]),
        );
        json.insert(
            "map_scripts".to_string(),
            serde_json::json!(["mods/new/map_scripts/Route29.json"]),
        );
        json.insert(
            "audio".to_string(),
            serde_json::json!([
                "mods/new/audio/music.json",
                "mods/new/audio/sfx.json",
                "mods/new/audio/cries.json"
            ]),
        );
        json.insert(
            "tilesets".to_string(),
            serde_json::json!(["mods/new/tilesets.json"]),
        );
        json.insert(
            "playability".to_string(),
            serde_json::json!(["mods/new/playability/main.json"]),
        );

        let files: ContentPackFiles =
            serde_json::from_value(Value::Object(json.clone())).expect("parse files");

        assert_eq!(
            files.entries(ContentPackCategory::Pokemon),
            &["mods/new/pokemon.json".to_string()]
        );
        assert_eq!(
            files.entries(ContentPackCategory::Audio),
            &[
                "mods/new/audio/music.json".to_string(),
                "mods/new/audio/sfx.json".to_string(),
                "mods/new/audio/cries.json".to_string(),
            ]
        );
        assert_eq!(
            files.entries(ContentPackCategory::MapScripts),
            &["mods/new/map_scripts/Route29.json".to_string()]
        );
        assert!(files.entries(ContentPackCategory::Maps).is_empty());
        assert_eq!(
            files.entries(ContentPackCategory::Playability),
            &["mods/new/playability/main.json".to_string()]
        );
        assert!(files.entries(ContentPackCategory::Moves).is_empty());

        json.remove("moves");
        let error = serde_json::from_value::<ContentPackFiles>(Value::Object(json))
            .expect_err("content pack file categories must be explicit, even when empty")
            .to_string();
        assert!(error.contains("missing field `moves`"), "{error}");

        let mut missing_map_scripts = empty_content_pack_files_json();
        missing_map_scripts.remove("map_scripts");
        let error = serde_json::from_value::<ContentPackFiles>(Value::Object(missing_map_scripts))
            .expect_err("raw map script category must be explicit")
            .to_string();
        assert!(error.contains("missing field `map_scripts`"), "{error}");
    }

    #[test]
    fn compile_options_json_requires_explicit_playability_rules_and_rejects_verify_switches() {
        let missing_playability =
            serde_json::from_value::<ModpackCompileOptions>(serde_json::json!({}))
                .expect_err("compile options must not default playability")
                .to_string();
        assert!(
            missing_playability.contains("missing field `playability`"),
            "{missing_playability}"
        );

        let disabled_verify = serde_json::from_value::<ModpackCompileOptions>(serde_json::json!({
            "verify": false,
            "playability": PlayabilityRules::default()
        }))
        .expect_err("compile options must not accept verification bypasses")
        .to_string();
        assert!(
            disabled_verify.contains("unknown field `verify`"),
            "{disabled_verify}"
        );

        let unknown_fallback = serde_json::from_value::<ModpackCompileOptions>(serde_json::json!({
            "playability": PlayabilityRules::default(),
            "fallback_playability": true
        }))
        .expect_err("compile options must not accept fallback metadata")
        .to_string();
        assert!(
            unknown_fallback.contains("unknown field `fallback_playability`"),
            "{unknown_fallback}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_playability_rules_as_modpack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Playability,
            serde_json::json!({
                "start_maps": ["Start"],
                "start_tiles": [],
                "initial_events": [],
                "initial_items": [],
                "goal_maps": [],
                "goal_events": ["EVENT_DONE"],
                "goal_items": [],
                "progression_rules": [{
                    "id": "finish",
                    "requires": { "events": [], "items": [], "maps": ["Start"] },
                    "grants": { "events": ["EVENT_DONE"], "items": [], "maps": [] }
                }],
                "map_access": [],
                "require_all_maps_reachable": false,
                "require_walkable_maps": true
            }),
        )
        .expect("apply playability payload");

        assert_eq!(data.playability.start_maps, vec!["Start".to_string()]);
        assert_eq!(data.playability.goal_events, vec!["EVENT_DONE".to_string()]);
        assert_eq!(data.playability.progression_rules[0].id, "finish");
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_playability_entries() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "start_maps": ["Start"],
            "start_tiles": [],
            "initial_events": [],
            "initial_items": [],
            "goal_maps": [],
            "goal_events": [],
            "goal_items": [],
            "progression_rules": [{
                "id": "finish",
                "requires": { "events": [], "items": [], "maps": ["Start"] },
                "grants": { "events": ["EVENT_DONE"], "items": [], "maps": [] }
            }],
            "map_access": [{
                "map": "LockedMap",
                "requires": { "events": ["EVENT_DONE"], "items": [], "maps": [] }
            }],
            "require_all_maps_reachable": false,
            "require_walkable_maps": true
        });
        data.apply_content_pack_payload(ContentPackCategory::Playability, payload.clone())
            .expect("initial playability payload should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::Playability, payload)
            .expect_err("duplicate playability payload must not append");

        assert!(
            format!("{error:#}").contains("duplicate playability start map 'Start'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_playability_rule_values_without_trimming() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Playability,
                serde_json::json!({
                    "start_maps": ["Start"],
                    "start_tiles": [{ "map": " Start", "tile": { "x": 1, "y": 2 } }],
                    "initial_events": [],
                    "initial_items": [],
                    "goal_maps": [],
                    "goal_events": [],
                    "goal_items": [],
                    "progression_rules": [],
                    "map_access": [],
                    "require_all_maps_reachable": false,
                    "require_walkable_maps": true
                }),
            )
            .expect_err("playability start tile maps must not be trimmed");

        assert!(
            format!("{error:#}").contains("pack token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Playability,
                serde_json::json!({
                    "start_maps": ["fallbackStart"],
                    "start_tiles": [],
                    "initial_events": [],
                    "initial_items": [],
                    "goal_maps": [],
                    "goal_events": [],
                    "goal_items": [],
                    "progression_rules": [],
                    "map_access": [],
                    "require_all_maps_reachable": false,
                    "require_walkable_maps": true
                }),
            )
            .expect_err("playability reserved ids must fail at load time");

        assert!(
            format!("{error:#}")
                .contains("pack token 'fallbackStart' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Playability,
                serde_json::json!({
                    "start_maps": ["New Bark Town"],
                    "start_tiles": [],
                    "initial_events": [],
                    "initial_items": [],
                    "goal_maps": [],
                    "goal_events": [],
                    "goal_items": [],
                    "progression_rules": [],
                    "map_access": [],
                    "require_all_maps_reachable": false,
                    "require_walkable_maps": true
                }),
            )
            .expect_err("playability start maps must be pack tokens");

        assert!(
            format!("{error:#}").contains("pack token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Playability,
                serde_json::json!({
                    "start_maps": [],
                    "start_tiles": [],
                    "initial_events": [],
                    "initial_items": [],
                    "goal_maps": [],
                    "goal_events": [],
                    "goal_items": [],
                    "progression_rules": [{
                        "id": "finish",
                        "requires": { "events": [" EVENT_READY"], "items": [], "maps": [] },
                        "grants": { "events": ["EVENT_DONE"], "items": [], "maps": [] }
                    }],
                    "map_access": [],
                    "require_all_maps_reachable": false,
                    "require_walkable_maps": true
                }),
            )
            .expect_err("playability progression requirements must not be trimmed");

        assert!(
            format!("{error:#}").contains("pack token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Playability,
                serde_json::json!({
                    "start_maps": [],
                    "start_tiles": [],
                    "initial_events": [],
                    "initial_items": [],
                    "goal_maps": [],
                    "goal_events": [],
                    "goal_items": [],
                    "progression_rules": [],
                    "map_access": [{
                        "map": "LockedMap",
                        "requires": { "events": [], "items": ["PASS\u{0007}"], "maps": [] }
                    }],
                    "require_all_maps_reachable": false,
                    "require_walkable_maps": true
                }),
            )
            .expect_err("playability map access requirements must not contain control characters");

        assert!(
            format!("{error:#}").contains("pack token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_payloads_merge_playability_rules_as_modpack_data() {
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                playability: PlayabilityRules {
                    start_maps: vec!["Start".to_string()],
                    goal_items: vec!["PASS".to_string()],
                    ..PlayabilityRules::default()
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();

        data.apply_modpack(&manifest)
            .expect("apply playability manifest");

        assert_eq!(data.playability.start_maps, vec!["Start".to_string()]);
        assert_eq!(data.playability.goal_items, vec!["PASS".to_string()]);
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_playability_rule_ids() {
        let mut data = GameDataSet {
            playability: PlayabilityRules {
                progression_rules: vec![ProgressionRule {
                    id: "finish".to_string(),
                    requires: ProgressionRequirements::default(),
                    grants: ProgressionGrants::default(),
                }],
                ..PlayabilityRules::default()
            },
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                playability: PlayabilityRules {
                    progression_rules: vec![ProgressionRule {
                        id: "finish".to_string(),
                        requires: ProgressionRequirements::default(),
                        grants: ProgressionGrants::default(),
                    }],
                    ..PlayabilityRules::default()
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate playability rule id must not append");

        assert!(
            format!("{error:#}").contains("duplicate playability progression rule 'finish'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_raw_pokedex_species() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Pokedex,
            serde_json::json!({
                "CHIKORITA": {
                    "species": "CHIKORITA",
                    "classification": "LEAF",
                    "height": 0.89,
                    "weight": 6.35,
                    "text": "It loves to bask in the sunlight."
                }
            }),
        )
        .expect("apply first raw Pokedex payload");
        assert_eq!(
            data.pokedex,
            vec![serde_json::json!({
                "species": "CHIKORITA",
                "classification": "LEAF",
                "height": 0.89,
                "weight": 6.35,
                "text": "It loves to bask in the sunlight."
            })]
        );
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA": {
                        "species": "CHIKORITA",
                        "classification": "LEAF",
                        "height": 0.89,
                        "weight": 6.35,
                        "text": "Duplicate raw entry must not replace the first."
                    }
                }),
            )
            .expect_err("duplicate raw Pokedex entries must not overwrite during payload merge")
            .to_string();

        assert!(
            error.contains("duplicate pokedex payload for species 'CHIKORITA'"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_raw_pokedex_entries_without_species() {
        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA": {
                        "classification": "LEAF",
                        "height": 0.89,
                        "weight": 6.35,
                        "text": "Missing species must not be accepted."
                    }
                }),
            )
            .expect_err("raw Pokedex entries must declare species")
            .to_string();

        assert!(
            error.contains("parse pokedex entry payload for 'CHIKORITA'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA": {
                        "species": "BAYLEEF",
                        "classification": "LEAF",
                        "height": 1.19,
                        "weight": 15.8,
                        "text": "Key mismatch must not be accepted."
                    }
                }),
            )
            .expect_err("raw Pokedex key must match species")
            .to_string();

        assert!(
            error.contains("pokedex key 'CHIKORITA' does not match record species 'BAYLEEF'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA ": {
                        "species": "CHIKORITA ",
                        "classification": "LEAF",
                        "height": 0.89,
                        "weight": 6.35,
                        "text": "Padded species keys must not be accepted."
                    }
                }),
            )
            .expect_err("raw Pokedex species keys must be exact")
            .to_string();

        assert!(
            error.contains(
                "pokedex species key 'CHIKORITA ' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA ALT": {
                        "species": "CHIKORITA ALT",
                        "classification": "LEAF",
                        "height": 0.89,
                        "weight": 6.35,
                        "text": "Internal-space species keys must not be accepted."
                    }
                }),
            )
            .expect_err("raw Pokedex species keys must be pack tokens")
            .to_string();

        assert!(
            error.contains(
                "pokedex species key 'CHIKORITA ALT' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA": {
                        "species": "CHIKORITA",
                        "classification": " LEAF",
                        "height": 0.89,
                        "weight": 6.35,
                        "text": "Padded classification must not be accepted."
                    }
                }),
            )
            .expect_err("raw Pokedex classification must be exact");
        assert!(
            format!("{error:#}").contains("pokedex classification ' LEAF'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA": {
                        "species": "CHIKORITA",
                        "classification": "LEAF",
                        "height": 0,
                        "weight": 6.35,
                        "text": "Height must be positive."
                    }
                }),
            )
            .expect_err("raw Pokedex height must be positive");
        assert!(
            format!("{error:#}")
                .contains("pokedex entry for species 'CHIKORITA' must declare positive height"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokedex,
                serde_json::json!({
                    "CHIKORITA": {
                        "species": "CHIKORITA",
                        "classification": "LEAF",
                        "height": 0.89,
                        "weight": 6.35,
                        "text": "Unknown fields must not be accepted.",
                        "legacy_text": "fallback"
                    }
                }),
            )
            .expect_err("raw Pokedex payload must reject unknown fields");
        assert!(
            format!("{error:#}").contains("unknown field `legacy_text`"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_pokedex_entry_species() {
        let mut data = GameDataSet::default();
        data.pokedex_entries.insert(
            "CHIKORITA".to_string(),
            RuntimePokedexEntry {
                species: "CHIKORITA".to_string(),
                classification: "LEAF".to_string(),
                height_digits: 9,
                weight_digits: 64,
                pages: vec!["Existing entry must not be replaced.".to_string()],
            },
        );
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokedexEntries,
                serde_json::json!({
                    "CHIKORITA": {
                        "species": "CHIKORITA",
                        "classification": "LEAF",
                        "heightDigits": 9,
                        "weightDigits": 64,
                        "pages": ["A sweet aroma gently wafts from the leaf on its head."]
                    }
                }),
            )
            .expect_err("Pokedex entries must not overwrite existing species during payload merge")
            .to_string();

        assert!(
            error.contains("duplicate pokedex entry for species 'CHIKORITA'"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_pokedex_entries() {
        for (field, value, expected) in [
            (
                "species",
                serde_json::json!("CHIKORITA "),
                "pokedex species id must be exact ASCII alphanumeric/underscore",
            ),
            (
                "classification",
                serde_json::json!(" LEAF"),
                "pokedex text must be exact non-empty text",
            ),
        ] {
            let mut payload = serde_json::json!({
                "CHIKORITA": {
                    "species": "CHIKORITA",
                    "classification": "LEAF",
                    "heightDigits": 9,
                    "weightDigits": 64,
                    "pages": ["A sweet aroma gently wafts from the leaf on its head."]
                }
            });
            payload["CHIKORITA"][field] = value;

            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::PokedexEntries, payload)
                .expect_err("malformed Pokedex entries must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }

        for (pages, expected) in [
            (
                serde_json::json!([]),
                "pokedex pages must contain at least one page",
            ),
            (
                serde_json::json!([""]),
                "pokedex page must be exact non-empty text",
            ),
            (
                serde_json::json!([" A sweet aroma gently wafts from the leaf on its head."]),
                "pokedex page must be exact non-empty text",
            ),
        ] {
            let error = GameDataSet::default()
                .apply_content_pack_payload(
                    ContentPackCategory::PokedexEntries,
                    serde_json::json!({
                        "CHIKORITA": {
                            "species": "CHIKORITA",
                            "classification": "LEAF",
                            "heightDigits": 9,
                            "weightDigits": 64,
                            "pages": pages
                        }
                    }),
                )
                .expect_err("malformed Pokedex entry pages must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokedexEntries,
                serde_json::json!({
                    "CHIKORITA ALT": {
                        "species": "CHIKORITA ALT",
                        "classification": "LEAF",
                        "heightDigits": 9,
                        "weightDigits": 64,
                        "pages": ["A sweet aroma gently wafts from the leaf on its head."]
                    }
                }),
            )
            .expect_err("Pokedex entry species ids must be exact tokens");

        assert!(
            format!("{error:#}")
                .contains("pokedex species id must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_payloads_reject_duplicate_pokedex_entry_species() {
        let entry = RuntimePokedexEntry {
            species: "CHIKORITA".to_string(),
            classification: "LEAF".to_string(),
            height_digits: 9,
            weight_digits: 64,
            pages: vec!["A sweet aroma gently wafts from the leaf on its head.".to_string()],
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokedex_entries: [("CHIKORITA".to_string(), entry)].into_iter().collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();
        data.pokedex_entries.insert(
            "CHIKORITA".to_string(),
            RuntimePokedexEntry {
                species: "CHIKORITA".to_string(),
                classification: "LEAF".to_string(),
                height_digits: 9,
                weight_digits: 64,
                pages: vec!["Existing entry must not be replaced.".to_string()],
            },
        );
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Pokedex entries must not overwrite during manifest merge")
            .to_string();

        assert!(
            error.contains("duplicate pokedex entry for species 'CHIKORITA'"),
            "{error}"
        );
    }

    #[test]
    fn compiled_game_pack_round_trips_as_runtime_artifact() {
        let path = temp_test_path("runtime.crystalpack");
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let report = canonical_test_compile_report(&data, "base-game");
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report);

        write_compiled_game_pack(&path, &pack).expect("write compiled pack");
        let loaded = read_compiled_game_pack(&path).expect("read compiled pack");
        let loaded_artifact =
            read_loaded_compiled_game_pack(&path).expect("read loaded compiled pack");

        assert_eq!(loaded.data(), pack.data());
        assert_eq!(loaded.runtime_files(), pack.runtime_files());
        assert_eq!(
            loaded.identity().expect("loaded identity"),
            pack.identity().expect("original identity")
        );
        assert_eq!(loaded_artifact.pack().data(), pack.data());
        assert_eq!(
            loaded_artifact.pack().identity().expect("stored identity"),
            pack.identity().expect("original identity")
        );
        assert_eq!(
            loaded_artifact.bytes(),
            std::fs::read(&path).expect("read raw pack").as_slice()
        );
        let raw = loaded_artifact.bytes();
        assert_eq!(
            u16::from_be_bytes([
                raw[COMPILED_GAME_PACK_VERSION_OFFSET],
                raw[COMPILED_GAME_PACK_VERSION_OFFSET + 1],
            ]),
            COMPILED_GAME_PACK_FORMAT_VERSION
        );
        let identity = loaded_artifact
            .save_modpack_identity()
            .expect("loaded pack has canonical save identity");
        assert_eq!(identity.id(), "base-game");
        assert_eq!(identity.hash(), sha256_hex(raw));
        assert!(loaded.data().pokemon.contains_key("CHIKORITA"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn compiled_game_pack_rejects_empty_corrupt_and_legacy_unframed_payloads() {
        let path = temp_test_path("framed-runtime.crystalpack");
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let pack = CompiledGamePack::new_unchecked_for_tests(
            data.clone(),
            canonical_test_compile_report(&data, "base-game"),
        );
        write_compiled_game_pack(&path, &pack).expect("write compiled pack");

        let mut empty = Vec::with_capacity(COMPILED_GAME_PACK_HEADER_LEN);
        empty.extend_from_slice(COMPILED_GAME_PACK_MAGIC);
        empty.extend_from_slice(&COMPILED_GAME_PACK_FORMAT_VERSION.to_be_bytes());
        empty.extend_from_slice(&0_u32.to_be_bytes());
        empty.extend_from_slice(&fnv1a32_bytes(&[]).to_be_bytes());
        let empty_error = decode_compiled_game_pack(&empty, &path)
            .expect_err("empty compiled pack payload is invalid")
            .to_string();
        assert!(empty_error.contains("payload is empty"), "{empty_error}");

        let mut corrupt = std::fs::read(&path).expect("read compiled pack");
        let expected_hash = u32::from_be_bytes([
            corrupt[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET],
            corrupt[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 1],
            corrupt[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 2],
            corrupt[COMPILED_GAME_PACK_PAYLOAD_HASH_OFFSET + 3],
        ]);
        let last = corrupt.last_mut().expect("payload byte");
        *last ^= 0x01;
        let actual_hash = fnv1a32_bytes(&corrupt[COMPILED_GAME_PACK_HEADER_LEN..]);
        let corrupt_error = decode_compiled_game_pack(&corrupt, &path)
            .expect_err("compiled pack payload hash must match")
            .to_string();
        assert!(
            corrupt_error.contains(&format!("{actual_hash:#010x}"))
                && corrupt_error.contains(&format!("{expected_hash:#010x}")),
            "{corrupt_error}"
        );

        let mut legacy_payload = Vec::new();
        ciborium::into_writer(&pack, &mut legacy_payload).expect("encode legacy pack payload");
        let mut legacy = COMPILED_GAME_PACK_MAGIC.to_vec();
        legacy.extend_from_slice(&legacy_payload);
        let legacy_error = decode_compiled_game_pack(&legacy, &path)
            .expect_err("legacy unframed compiled packs are invalid")
            .to_string();
        assert!(
            legacy_error.contains("unsupported frame format version"),
            "{legacy_error}"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn compiled_game_pack_paths_are_exact_runtime_artifact_paths() {
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let pack = CompiledGamePack::new_unchecked_for_tests(
            data.clone(),
            canonical_test_compile_report(&data, "base-game"),
        );

        let json_path = temp_test_path("runtime.json");
        let json_error = write_compiled_game_pack(&json_path, &pack)
            .expect_err("compiled packs must not write JSON paths")
            .to_string();
        assert!(json_error.contains("must use .crystalpack"), "{json_error}");

        let parent_error = read_loaded_compiled_game_pack("../runtime.crystalpack")
            .expect_err("compiled pack reads must not traverse parents")
            .to_string();
        assert!(
            parent_error.contains("must not traverse parent directories"),
            "{parent_error}"
        );

        let current_error = read_loaded_compiled_game_pack("./runtime.crystalpack")
            .expect_err("compiled pack reads must not include current-directory components")
            .to_string();
        assert!(
            current_error.contains("must not include current-directory components"),
            "{current_error}"
        );
    }

    #[test]
    fn read_verified_compiled_game_pack_rejects_missing_runtime_sections() {
        let path = temp_test_path("missing-runtime.crystalpack");
        let data = GameDataSet::default();
        let pack = CompiledGamePack::new_unchecked_for_tests(
            data.clone(),
            canonical_test_compile_report(&data, "base-game"),
        );

        write_compiled_game_pack(&path, &pack).expect("write compiled pack");
        let error = read_verified_compiled_game_pack(&path)
            .expect_err("verified read must reject invalid runtime data")
            .to_string();

        assert!(error.contains("decode compiled game pack"), "{error}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn compiled_game_pack_rejects_stale_stored_identity() {
        let path = temp_test_path("stale-identity.crystalpack");
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let report = canonical_test_compile_report(&data, "base-game");
        let mut pack = CompiledGamePack::new_unchecked_for_tests(data, report);
        let derived = pack.identity().expect("derived identity");
        pack.identity.content_hash = "f".repeat(64);

        let write_error = write_compiled_game_pack(&path, &pack)
            .expect_err("stale identity must not be written")
            .to_string();
        assert!(
            write_error.contains("validate compiled game pack identity"),
            "{write_error}"
        );
        let verify_error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("stale identity must not verify")
            .to_string();
        assert!(
            verify_error.contains("does not match derived identity")
                && verify_error.contains(&derived.content_hash),
            "{verify_error}"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn verifier_rejects_ambiguous_runtime_map_bindings() {
        let mut first = test_map_module("FirstMap", "DUPLICATE_MAP", None);
        first.attributes.map_constant = Some("DUPLICATE_MAP".to_string());
        let mut second = test_map_module("SecondMap", "DUPLICATE_MAP", None);
        second.attributes.map_constant = Some("DUPLICATE_MAP".to_string());
        let mut data = GameDataSet {
            maps: [
                ("FirstMap".to_string(), first),
                ("SecondMap".to_string(), second),
            ]
            .into_iter()
            .collect(),
            runtime_map_metadata: [
                (
                    "FIRST_MAP".to_string(),
                    test_runtime_map_metadata("FIRST_MAP", "SharedMapName"),
                ),
                (
                    "SECOND_MAP".to_string(),
                    test_runtime_map_metadata("SECOND_MAP", "SharedMapName"),
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
            diagnostic.code == "duplicate_runtime_map_constant"
                && diagnostic.subject == "DUPLICATE_MAP"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_runtime_map_metadata_name"
                && diagnostic.subject == "SharedMapName"
        }));
    }

    #[test]
    fn verified_compiled_game_pack_rejects_missing_manifest_identity() {
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let mut report = canonical_test_compile_report(&data, "base-game");
        report.manifests.clear();
        let pack = CompiledGamePack::new_unchecked_for_tests(data.clone(), report);

        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("verified packs must reject missing manifest identity")
            .to_string();

        assert!(
            error.contains("must include at least one manifest id"),
            "{error}"
        );
        assert!(
            !error.contains("missing_runtime_pokemon"),
            "manifest identity must be rejected before runtime section checks: {error}"
        );
    }

    #[test]
    fn verified_compiled_game_pack_rejects_malformed_manifest_identity() {
        for (manifest_ids, expected) in [
            (
                vec![" base-game".to_string()],
                "must be exact ASCII letters, numbers, underscores, hyphens, or dots",
            ),
            (
                vec!["base+game".to_string()],
                "must be exact ASCII letters, numbers, underscores, hyphens, or dots",
            ),
            (
                vec!["bad id".to_string()],
                "must be exact ASCII letters, numbers, underscores, hyphens, or dots",
            ),
            (
                vec!["base/game".to_string()],
                "must be exact ASCII letters, numbers, underscores, hyphens, or dots",
            ),
            (
                vec!["base-game".to_string(), "base-game".to_string()],
                "duplicate manifest id 'base-game'",
            ),
        ] {
            let pack = CompiledGamePack::new_unchecked_for_tests(
                GameDataSet::default(),
                ModpackCompileReport {
                    manifests: manifest_ids,
                    ..ModpackCompileReport::default()
                },
            );
            let error = verify_compiled_game_pack_for_runtime(&pack)
                .expect_err("verified runtime packs must reject malformed manifest identity")
                .to_string();

            assert!(error.contains(expected), "{error}");
            assert!(
                !error.contains("missing_runtime_pokemon"),
                "manifest identity must be rejected before runtime section checks: {error}"
            );
        }
    }

    #[test]
    fn verified_compiled_game_pack_rejects_missing_runtime_sections() {
        let pack = CompiledGamePack::new_unchecked_for_tests(
            GameDataSet::default(),
            ModpackCompileReport {
                manifests: vec!["base-game".to_string()],
                ..ModpackCompileReport::default()
            },
        );

        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("runtime packs must embed Pokemon, moves, and maps")
            .to_string();

        assert!(error.contains("compiled game pack has no Pokemon species data"));

        let pack = CompiledGamePack::new_unchecked_for_tests(
            GameDataSet {
                pokemon: BTreeMap::from([("NEW_MON".to_string(), species())]),
                moves: BTreeMap::from([("TACKLE".to_string(), test_move("TACKLE"))]),
                ..GameDataSet::default()
            },
            ModpackCompileReport {
                manifests: vec!["base-game".to_string()],
                pokemon: 1,
                moves: 1,
                ..ModpackCompileReport::default()
            },
        );

        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("runtime packs must embed map modules")
            .to_string();

        assert!(error.contains("compiled game pack has no map modules"));
    }

    #[test]
    fn verified_compiled_game_pack_rejects_malformed_runtime_map_blocks() {
        let mut data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let map = data
            .maps
            .get_mut("Route29")
            .expect("base data has Route29 map");
        map.attributes.width = 2;
        map.attributes.height = 2;
        map.blocks = vec![1, 2, 3];
        let report = canonical_test_compile_report(&data, "base-game");
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report);

        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("runtime packs must embed exact map block dimensions")
            .to_string();

        assert!(
            error
                .contains("compiled game pack map 'Route29' has 3 blocks but dimensions require 4"),
            "{error}"
        );
    }

    #[test]
    fn saved_map_tile_bounds_rejects_subtile_overflow_without_wrapping() {
        let mut data = GameDataSet::default();
        let mut map = test_map_module("HugeMap", "HUGE_MAP", None);
        let metatile_width = u16::try_from(METATILE_WIDTH).expect("positive metatile width");
        map.attributes.width = (u16::MAX / metatile_width) + 1;
        map.attributes.height = 1;
        data.maps.insert("HugeMap".to_string(), map);

        assert_eq!(data.saved_map_tile_bounds("HugeMap"), None);
    }

    #[test]
    fn verified_compiled_game_pack_rejects_stale_report_counts() {
        let pack = CompiledGamePack::new_unchecked_for_tests(
            GameDataSet::default(),
            ModpackCompileReport {
                manifests: vec!["base-game".to_string()],
                maps: 1,
                ..ModpackCompileReport::default()
            },
        );

        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("verified runtime packs must reject stale report counts")
            .to_string();

        assert!(
            error.contains("report maps count 1 does not match embedded data count 0"),
            "{error}"
        );
        assert!(
            !error.contains("missing_runtime_pokemon"),
            "report/data mismatch must be rejected before runtime section checks: {error}"
        );
    }

    #[test]
    fn verified_compiled_game_pack_rejects_stale_report_map_references() {
        for (report, expected) in [
            (
                ModpackCompileReport {
                    manifests: vec!["base-game".to_string()],
                    maps: 1,
                    graph_edges: vec![PlayabilityGraphEdge {
                        from: "Start".to_string(),
                        to: "Ghost".to_string(),
                        kind: "warp".to_string(),
                    }],
                    ..ModpackCompileReport::default()
                },
                "graph_edges.to references map 'Ghost' that is not embedded in pack data",
            ),
            (
                ModpackCompileReport {
                    manifests: vec!["base-game".to_string()],
                    maps: 1,
                    reachable_maps: vec!["Start".to_string(), "Start".to_string()],
                    ..ModpackCompileReport::default()
                },
                "reachable_maps includes duplicate map 'Start'",
            ),
            (
                ModpackCompileReport {
                    manifests: vec!["base-game".to_string()],
                    maps: 1,
                    graph_edges: vec![PlayabilityGraphEdge {
                        from: "Start".to_string(),
                        to: "Start".to_string(),
                        kind: "warp edge".to_string(),
                    }],
                    ..ModpackCompileReport::default()
                },
                "graph_edges.kind 'warp edge' must be an exact token",
            ),
        ] {
            let pack = CompiledGamePack::new_unchecked_for_tests(
                GameDataSet {
                    maps: [(
                        "Start".to_string(),
                        test_map_module("Start", "START_MAP", None),
                    )]
                    .into_iter()
                    .collect(),
                    ..GameDataSet::default()
                },
                report,
            );

            let error = verify_compiled_game_pack_for_runtime(&pack)
                .expect_err("verified runtime packs must reject stale report map references")
                .to_string();

            assert!(error.contains(expected), "{error}");
            assert!(
                !error.contains("missing_runtime_pokemon"),
                "report map references must be rejected before runtime section checks: {error}"
            );
        }

        let pack = CompiledGamePack::new_unchecked_for_tests(
            GameDataSet {
                maps: [
                    (
                        "Start".to_string(),
                        test_map_module("Start", "START_MAP", None),
                    ),
                    (
                        "Ghost".to_string(),
                        test_map_module("Ghost", "GHOST_MAP", None),
                    ),
                ]
                .into_iter()
                .collect(),
                ..GameDataSet::default()
            },
            ModpackCompileReport {
                manifests: vec!["base-game".to_string()],
                maps: 2,
                reachable_maps: vec!["Start".to_string()],
                solvable_maps: vec!["Ghost".to_string()],
                ..ModpackCompileReport::default()
            },
        );
        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("verified runtime packs must reject stale solved map reports")
            .to_string();
        assert!(
            error.contains(
                "solvable_maps references map 'Ghost' that is neither reachable nor declared by embedded playability rules"
            ),
            "{error}"
        );
        assert!(
            !error.contains("missing_runtime_pokemon"),
            "stale solved maps must be rejected before runtime section checks: {error}"
        );
    }

    #[test]
    fn verified_compiled_game_pack_rejects_stale_progression_report_outputs() {
        for (report, expected) in [
            (
                ModpackCompileReport {
                    manifests: vec!["base-game".to_string()],
                    solvable_events: vec!["EVENT HALL_OF_FAME".to_string()],
                    ..ModpackCompileReport::default()
                },
                "solvable_events value 'EVENT HALL_OF_FAME' must be an exact token",
            ),
            (
                ModpackCompileReport {
                    manifests: vec!["base-game".to_string()],
                    solvable_items: vec!["PASS".to_string(), "PASS".to_string()],
                    ..ModpackCompileReport::default()
                },
                "solvable_items includes duplicate value 'PASS'",
            ),
            (
                ModpackCompileReport {
                    manifests: vec!["base-game".to_string()],
                    solvable_items: vec!["PASS".to_string()],
                    ..ModpackCompileReport::default()
                },
                "solvable_items references item 'PASS' that is not embedded in pack data",
            ),
        ] {
            let pack = CompiledGamePack::new_unchecked_for_tests(GameDataSet::default(), report);

            let error = verify_compiled_game_pack_for_runtime(&pack)
                .expect_err("verified runtime packs must reject stale progression report outputs")
                .to_string();

            assert!(error.contains(expected), "{error}");
            assert!(
                !error.contains("missing_runtime_pokemon"),
                "progression report outputs must be rejected before runtime section checks: {error}"
            );
        }

        let pack = CompiledGamePack::new_unchecked_for_tests(
            GameDataSet::default(),
            ModpackCompileReport {
                manifests: vec!["base-game".to_string()],
                solvable_events: vec!["EVENT_HALL_OF_FAME".to_string()],
                ..ModpackCompileReport::default()
            },
        );
        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("verified runtime packs must reject undeclared solvable events")
            .to_string();
        assert!(
            error.contains(
                "solvable_events references event 'EVENT_HALL_OF_FAME' that is not declared by embedded playability rules"
            ),
            "{error}"
        );

        let pack = CompiledGamePack::new_unchecked_for_tests(
            GameDataSet {
                items: [("PASS".to_string(), test_item("PASS"))]
                    .into_iter()
                    .collect(),
                ..GameDataSet::default()
            },
            ModpackCompileReport {
                manifests: vec!["base-game".to_string()],
                items: 1,
                solvable_items: vec!["PASS".to_string()],
                ..ModpackCompileReport::default()
            },
        );
        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("verified runtime packs must reject undeclared solvable items")
            .to_string();
        assert!(
            error.contains(
                "solvable_items references item 'PASS' that is not declared by embedded playability rules"
            ),
            "{error}"
        );
    }

    #[test]
    fn verified_compiled_game_pack_rejects_stored_error_diagnostics() {
        let path = temp_test_path("error-report.crystalpack");
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let mut report = canonical_test_compile_report(&data, "base-game");
        report.diagnostics.push(VerificationError::error(
            "bad_pack",
            "test",
            "test diagnostic",
        ));
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report);

        write_compiled_game_pack(&path, &pack).expect("write compiled pack");
        let error = read_loaded_verified_compiled_game_pack(&path)
            .expect_err("verified loaded read must reject error reports")
            .to_string();

        assert!(error.contains("bad_pack"), "{error}");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn verified_compiled_game_pack_revalidates_embedded_runtime_data() {
        let root = repository_root_for_tests();
        let mut data = AssetRoot::new(&root)
            .load_base_game_data()
            .expect("load base game data");
        let metadata = data
            .runtime_map_metadata
            .values_mut()
            .next()
            .expect("base data has runtime map metadata");
        metadata.group_name.clear();
        let report = canonical_test_compile_report(&data, "base-game");
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report);

        let error = verify_compiled_game_pack_for_runtime(&pack)
            .expect_err("verified runtime packs must validate embedded data, not just reports")
            .to_string();

        assert!(error.contains("invalid_runtime_map_metadata"), "{error}");
    }

    #[test]
    fn verification_rejects_empty_runtime_game_sections() {
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &GameDataSet::default(),
            &PlayabilityRules::default(),
        );

        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(codes.contains("missing_runtime_pokemon"));
        assert!(codes.contains("missing_runtime_moves"));
        assert!(codes.contains("missing_runtime_growth_rates"));
        assert!(codes.contains("missing_runtime_learnsets"));
        assert!(codes.contains("missing_runtime_evolutions"));
        assert!(codes.contains("missing_runtime_capture_rules"));
        assert!(codes.contains("missing_runtime_capture_wobble_probabilities"));
        assert!(codes.contains("missing_runtime_battle_stat_multipliers"));
        assert!(codes.contains("missing_runtime_move_priorities"));
        assert!(codes.contains("missing_runtime_type_categories"));
        assert!(codes.contains("missing_runtime_type_effectiveness"));
        assert!(codes.contains("missing_runtime_weather_modifiers"));
        assert!(codes.contains("missing_runtime_battle_reward_rules"));
        assert!(codes.contains("missing_runtime_battle_escape_rules"));
        assert!(codes.contains("missing_runtime_marts"));
        assert!(codes.contains("missing_runtime_currency_constants"));
        assert!(codes.contains("missing_runtime_step_event_rules"));
        assert!(codes.contains("missing_runtime_fishing_catalog"));
        assert!(codes.contains("missing_runtime_fruit_trees"));
        assert!(codes.contains("missing_runtime_field_moves"));
        assert!(codes.contains("missing_runtime_title_screen"));
        assert!(codes.contains("missing_runtime_items"));
        assert!(codes.contains("missing_runtime_trainers"));
        assert!(codes.contains("missing_runtime_audio"));
        assert!(codes.contains("missing_runtime_pokemon_cries"));
        assert!(codes.contains("missing_runtime_tilesets"));
        assert!(codes.contains("missing_runtime_scripts"));
        assert!(codes.contains("missing_runtime_map_geometry"));
        assert!(codes.contains("missing_runtime_map_objects"));
        assert!(codes.contains("missing_runtime_map_metadata"));
        assert!(codes.contains("missing_runtime_spawn_points"));
        assert!(codes.contains("missing_runtime_maps"));
        assert!(codes.contains("missing_runtime_pc_strings"));
        assert!(codes.contains("missing_runtime_menu_icons"));
        assert!(codes.contains("missing_runtime_pokedex_entries"));
        assert!(codes.contains("missing_runtime_pokemon_frontpic_animations"));
        assert!(codes.contains("missing_runtime_move_names"));
        assert!(codes.contains("missing_runtime_asm_text"));
        assert!(codes.contains("missing_runtime_battle_animations"));
        assert!(codes.contains("missing_runtime_battle_animation_table"));
        assert!(codes.contains("missing_runtime_battle_anim_bundle"));
        assert!(codes.contains("missing_runtime_sprite_anim_bundle"));
        assert!(codes.contains("missing_runtime_sprite_palette_defaults"));
        assert!(codes.contains("missing_runtime_pokegear_town_map_palettes"));
        assert!(codes.contains("missing_runtime_pokegear_landmarks"));
        assert!(codes.contains("missing_runtime_phone_contacts"));
        assert!(codes.contains("missing_runtime_permanent_phone_numbers"));
        assert!(codes.contains("missing_runtime_special_phone_calls"));
        assert!(codes.contains("missing_runtime_phone_scripts"));
        assert!(codes.contains("missing_runtime_flee_mons"));
        assert!(codes.contains("missing_runtime_buena_password_categories"));
        assert!(codes.contains("missing_runtime_roaming_pokemon"));
        assert!(codes.contains("missing_runtime_buena_prizes"));
        assert!(codes.contains("missing_runtime_kurt_apricorn_recipes"));
        assert!(codes.contains("missing_runtime_shuckie_gift"));
        assert!(codes.contains("missing_runtime_dratini_move_sets"));
        assert!(codes.contains("missing_runtime_bug_contest_config"));
        assert!(codes.contains("missing_runtime_battle_tower_rules"));
        assert!(codes.contains("missing_runtime_oak_ratings"));
        assert!(codes.contains("missing_runtime_odd_egg_definitions"));
        assert!(codes.contains("missing_runtime_magikarp_lengths"));
        assert!(codes.contains("missing_runtime_happiness_data"));
        assert!(codes.contains("missing_runtime_initialize_events"));
        assert!(codes.contains("missing_runtime_story_event_script_constants"));
    }

    #[test]
    fn runtime_pack_geometry_uses_declared_map_module_geometry() {
        let mut data = GameDataSet {
            maps: [(
                "Route29".to_string(),
                test_map_module("Route29", "ROUTE_29", None),
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(!codes.contains("missing_runtime_map_geometry"));
        assert!(codes.contains("missing_runtime_map_objects"));
        assert!(!codes.contains("missing_runtime_maps"));
    }

    #[test]
    fn runtime_pack_map_attributes_require_declared_tilesets() {
        let data = GameDataSet {
            map_attributes: [(
                "Route29".to_string(),
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
                    map_constant: Some("ROUTE_29".to_string()),
                    map_group_constant: None,
                    blocks_label: Some("Route29_Blocks".to_string()),
                    map_scripts_label: None,
                    map_events_label: None,
                    connection_flags: None,
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(codes.contains("unknown_map_tileset"));

        let data = GameDataSet {
            tilesets: [("johto".to_string(), test_tileset_definition())]
                .into_iter()
                .collect(),
            ..data
        };
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "unknown_map_tileset")
        );
    }

    #[test]
    fn map_playability_uses_pack_owned_tileset_collision() {
        let mut tileset = test_tileset_definition();
        tileset.collision.insert(
            "1".to_string(),
            vec![
                "NOT_A_COLLISION".to_string(),
                "FLOOR".to_string(),
                "FLOOR".to_string(),
                "FLOOR".to_string(),
            ],
        );
        let data = GameDataSet {
            maps: [(
                "Route29".to_string(),
                test_map_module("Route29", "ROUTE_29", None),
            )]
            .into_iter()
            .collect(),
            tilesets: [("johto".to_string(), tileset)].into_iter().collect(),
            ..GameDataSet::default()
        };
        let report = verify_complete_test_game_data(
            &data,
            &PlayabilityRules {
                require_walkable_maps: true,
                ..PlayabilityRules::default()
            },
        );

        assert!(
            report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "invalid_tileset_collision"
                    && diagnostic.subject == "Route29"
                    && diagnostic.message.contains("NOT_A_COLLISION")
            }),
            "{:?}",
            report.diagnostics
        );
    }

    #[test]
    fn runtime_pack_objects_use_declared_map_module_objects() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.objects = vec![test_object(
            "ROUTE29_YOUNGSTER",
            "EVENT_ROUTE29_YOUNGSTER",
            4,
            5,
        )];
        let data = GameDataSet {
            maps: [("Route29".to_string(), module)].into_iter().collect(),
            map_attributes: [(
                "Route29".to_string(),
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
                    map_constant: Some("ROUTE_29".to_string()),
                    map_group_constant: None,
                    blocks_label: Some("Route29_Blocks".to_string()),
                    map_scripts_label: None,
                    map_events_label: None,
                    connection_flags: None,
                },
            )]
            .into_iter()
            .collect(),
            map_blocks: [("Route29_Blocks".to_string(), "1".to_string())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(!codes.contains("missing_runtime_map_geometry"));
        assert!(!codes.contains("missing_runtime_map_objects"));
        assert!(!codes.contains("missing_runtime_maps"));
    }

    #[test]
    fn runtime_compiled_game_pack_rejects_json_extension() {
        let path = temp_test_path("runtime.json");
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let pack = CompiledGamePack::new_unchecked_for_tests(
            data.clone(),
            canonical_test_compile_report(&data, "core"),
        );

        let error = write_compiled_game_pack(&path, &pack)
            .expect_err("runtime compiled packs must not be JSON files")
            .to_string();

        assert!(error.contains("must use .crystalpack"));

        let extensionless = temp_test_path("runtime");
        let extensionless_error = write_compiled_game_pack(&extensionless, &pack)
            .expect_err("runtime compiled packs must declare an exact extension")
            .to_string();

        assert!(
            extensionless_error.contains("must have a file extension"),
            "{extensionless_error}"
        );
    }

    #[test]
    fn asset_root_compiled_game_pack_paths_reject_aliases_and_load_relative_pack() {
        let root = temp_test_path("compiled-pack-root");
        let _ = std::fs::remove_dir_all(&root);
        let data_root = root.join("apps/web/assets/data");
        std::fs::create_dir_all(data_root.join("content-packs")).expect("create data root");

        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let pack = CompiledGamePack::new_unchecked_for_tests(
            data.clone(),
            canonical_test_compile_report(&data, "core"),
        );
        write_compiled_game_pack(data_root.join("content-packs/core.crystalpack"), &pack)
            .expect("write compiled pack");
        let asset_root = AssetRoot::new(&root);

        let loaded = asset_root
            .load_loaded_compiled_game_pack("content-packs/core.crystalpack")
            .expect("load relative compiled pack");
        assert!(loaded.bytes().starts_with(COMPILED_GAME_PACK_MAGIC));

        let legacy = asset_root
            .load_compiled_game_pack("assets/data/content-packs/core.crystalpack")
            .expect_err("compiled pack paths must not accept assets/data aliases")
            .to_string();
        assert!(
            legacy.contains("must not include the assets/data prefix"),
            "{legacy}"
        );

        let traversal = asset_root
            .load_compiled_game_pack("content-packs/../core.crystalpack")
            .expect_err("compiled pack paths must not traverse")
            .to_string();
        assert!(
            traversal.contains("must not traverse parent directories"),
            "{traversal}"
        );

        let current_dir = asset_root
            .load_compiled_game_pack("content-packs/./core.crystalpack")
            .expect_err("compiled pack paths must not accept current-directory aliases")
            .to_string();
        assert!(
            current_dir.contains("must not include current-directory components"),
            "{current_dir}"
        );

        let absolute = asset_root
            .load_compiled_game_pack(data_root.join("content-packs/core.crystalpack"))
            .expect_err("compiled pack paths must not be absolute")
            .to_string();
        assert!(
            absolute.contains("must be relative to assets/data"),
            "{absolute}"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn compiled_report_requires_complete_exported_shape() {
        let mut report = serde_json::to_value(ModpackCompileReport::default())
            .expect("serialize compile report");
        report
            .as_object_mut()
            .expect("report object")
            .remove("reachable_maps");

        let error = serde_json::from_value::<ModpackCompileReport>(report)
            .expect_err("compiled reports must not default missing fields")
            .to_string();

        assert!(error.contains("missing field `reachable_maps`"), "{error}");
    }

    #[test]
    fn compiled_report_rejects_unknown_fields() {
        let mut report = serde_json::to_value(ModpackCompileReport::default())
            .expect("serialize compile report");
        report
            .as_object_mut()
            .expect("report object")
            .insert("legacy_summary".to_string(), serde_json::json!({}));

        let error = serde_json::from_value::<ModpackCompileReport>(report)
            .expect_err("compiled reports must use the exported report schema exactly")
            .to_string();

        assert!(error.contains("unknown field `legacy_summary`"), "{error}");
    }

    #[test]
    fn music_modpack_assets_must_be_midi_files_not_json_or_asm() {
        let midi = ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.mid")
            .expect("valid MIDI music asset");
        assert_eq!(midi.id, "MUSIC_ROUTE_29");
        assert_eq!(midi.kind, ModpackAudioKind::Music);
        let sfx = ModpackAudioAsset::sound_effect("SFX_TACKLE", "mods/new/sfx/SFX_TACKLE.mid")
            .expect("valid MIDI sfx asset");
        assert_eq!(sfx.kind, ModpackAudioKind::SoundEffect);

        let lowercase_error = ModpackAudioAsset::music("route29", "mods/new/music/route29.mid")
            .expect_err("lowercase music id is not accepted");
        assert!(lowercase_error.to_string().contains("must use an exact"));

        let padded_id_error =
            ModpackAudioAsset::music(" MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.mid")
                .expect_err("padded audio ids are not accepted");
        assert!(
            padded_id_error.to_string().contains("must use an exact"),
            "{padded_id_error}"
        );

        let mismatched_path =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_30.mid")
                .expect_err("explicit audio ids must match their file stems");
        assert!(
            mismatched_path
                .to_string()
                .contains("must match the exact audio id"),
            "{mismatched_path}"
        );

        let mismatched_directory =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/sfx/MUSIC_ROUTE_29.mid")
                .expect_err("music assets must live under the music directory");
        assert!(
            mismatched_directory
                .to_string()
                .contains("must live under music, found sfx"),
            "{mismatched_directory}"
        );

        let absolute_path =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "/tmp/music/MUSIC_ROUTE_29.mid")
                .expect_err("audio asset paths must be runtime relative");
        assert!(
            absolute_path
                .to_string()
                .contains("must be relative to assets/data"),
            "{absolute_path}"
        );

        let assets_data_prefix = ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "assets/data/content-packs/new/music/MUSIC_ROUTE_29.mid",
        )
        .expect_err("audio asset paths must not include resolver aliases");
        assert!(
            assets_data_prefix
                .to_string()
                .contains("must not include the assets/data prefix"),
            "{assets_data_prefix}"
        );

        let traversing_path = ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "content-packs/new/music/../music/MUSIC_ROUTE_29.mid",
        )
        .expect_err("audio asset paths must not traverse");
        assert!(
            traversing_path
                .to_string()
                .contains("must not traverse parent directories"),
            "{traversing_path}"
        );

        let current_directory_alias = ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "content-packs/new/music/./MUSIC_ROUTE_29.mid",
        )
        .expect_err("audio asset paths must not accept current-directory aliases");
        assert!(
            current_directory_alias
                .to_string()
                .contains("must not include current-directory components"),
            "{current_directory_alias}"
        );

        let asm_error =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.asm")
                .expect_err("music ASM is not accepted");
        assert!(
            asm_error
                .to_string()
                .contains("MIDI audio asset 'MUSIC_ROUTE_29' must use a .mid file")
        );

        let extensionless_error =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29")
                .expect_err("extensionless music is not accepted");
        assert!(
            extensionless_error
                .to_string()
                .contains("path must have a file extension"),
            "{extensionless_error}"
        );

        let wrong_extension_error =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.bin")
                .expect_err("non-MIDI music is not accepted");
        assert!(
            wrong_extension_error
                .to_string()
                .contains("must use a .mid file"),
            "{wrong_extension_error}"
        );

        let midi_error =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.midi")
                .expect_err(".midi music is not accepted");
        assert!(midi_error.to_string().contains("must use a .mid file"));

        let uppercase_mid_error =
            ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.MID")
                .expect_err("case-changed MIDI extensions are not accepted");
        assert!(
            uppercase_mid_error
                .to_string()
                .contains("must use a .mid file"),
            "{uppercase_mid_error}"
        );

        let cry = ModpackAudioAsset::cry("CRY_NIDORAN_M", "mods/new/cries/CRY_NIDORAN_M.mid")
            .expect("valid MIDI cry asset");
        assert_eq!(cry.kind, ModpackAudioKind::Cry);
        assert_eq!(cry.source, ModpackAudioSource::Midi);
        let pcm_cry = ModpackAudioAsset::pcm(
            "CRY_NIDORAN_M",
            "mods/new/cries/CRY_NIDORAN_M.pcm",
            ModpackAudioKind::Cry,
            ModpackPcmAudioFormat {
                sample_rate_hz: 32768,
                channels: 1,
                bits_per_sample: 16,
            },
        )
        .expect("valid PCM cry asset");
        assert_eq!(pcm_cry.kind, ModpackAudioKind::Cry);
        assert_eq!(pcm_cry.source, ModpackAudioSource::Pcm);

        let singular_cry_dir =
            ModpackAudioAsset::cry("CRY_NIDORAN_M", "mods/new/cry/CRY_NIDORAN_M.mid")
                .expect_err("singular cry directory is not a modpack audio category")
                .to_string();
        assert!(
            singular_cry_dir.contains("must live under cries, found cry"),
            "{singular_cry_dir}"
        );

        let missing_source = serde_json::from_str::<ModpackAudioAsset>(
            r#"{"id":"MUSIC_ROUTE_29","path":"mods/new/music/MUSIC_ROUTE_29.mid","kind":"music"}"#,
        )
        .expect_err("audio assets must declare an exact source");
        assert!(
            missing_source
                .to_string()
                .contains("missing field `source`"),
            "{missing_source}"
        );

        for (field, payload) in [
            (
                "sample_format",
                serde_json::json!({
                    "id": "MUSIC_ROUTE_29",
                    "path": "mods/new/music/MUSIC_ROUTE_29.mid",
                    "kind": "music",
                    "source": "midi",
                    "sample_format": null
                }),
            ),
            (
                "sample_rate_hz",
                serde_json::json!({
                    "id": "MUSIC_ROUTE_29",
                    "path": "mods/new/music/MUSIC_ROUTE_29.mid",
                    "kind": "music",
                    "source": "midi",
                    "sample_rate_hz": 44100
                }),
            ),
            (
                "channels",
                serde_json::json!({
                    "id": "MUSIC_ROUTE_29",
                    "path": "mods/new/music/MUSIC_ROUTE_29.mid",
                    "kind": "music",
                    "source": "midi",
                    "channels": 2
                }),
            ),
            (
                "pcm_format",
                serde_json::json!({
                    "id": "MUSIC_ROUTE_29",
                    "path": "mods/new/music/MUSIC_ROUTE_29.mid",
                    "kind": "music",
                    "source": "midi",
                    "pcm_format": {
                        "sample_rate_hz": 32768,
                        "channels": 1,
                        "bits_per_sample": 16
                    }
                }),
            ),
        ] {
            let error = serde_json::from_value::<ModpackAudioAsset>(payload)
                .expect_err("PCM metadata fields must not be accepted for MIDI")
                .to_string();
            if field == "pcm_format" {
                assert!(
                    error.contains("must not declare PCM metadata"),
                    "{field} produced unexpected error: {error}"
                );
            } else {
                assert!(
                    error.contains(&format!("unknown field `{field}`")),
                    "{field} produced unexpected error: {error}"
                );
            }
        }

        for (label, payload, expected) in [
            (
                "audio id",
                serde_json::json!({
                    "id": "music_route_29",
                    "path": "mods/new/music/music_route_29.mid",
                    "kind": "music",
                    "source": "midi",
                }),
                "must use an exact",
            ),
            (
                "reserved audio id",
                serde_json::json!({
                    "id": "MUSIC_FALLBACK_ROUTE_29",
                    "path": "mods/new/music/MUSIC_FALLBACK_ROUTE_29.mid",
                    "kind": "music",
                    "source": "midi",
                }),
                "uses reserved runtime pack prefix",
            ),
            (
                "audio path extension",
                serde_json::json!({
                    "id": "MUSIC_ROUTE_29",
                    "path": "mods/new/music/MUSIC_ROUTE_29.mp3",
                    "kind": "music",
                    "source": "midi",
                }),
                "must use a .mid file",
            ),
            (
                "PCM path extension",
                serde_json::json!({
                    "id": "CRY_NIDORAN_M",
                    "path": "mods/new/cries/CRY_NIDORAN_M.wav",
                    "kind": "cry",
                    "source": "pcm",
                    "pcm_format": {
                        "sample_rate_hz": 32768,
                        "channels": 1,
                        "bits_per_sample": 16
                    }
                }),
                "must use a .pcm file",
            ),
            (
                "missing PCM format",
                serde_json::json!({
                    "id": "CRY_NIDORAN_M",
                    "path": "mods/new/cries/CRY_NIDORAN_M.pcm",
                    "kind": "cry",
                    "source": "pcm"
                }),
                "must declare pcm_format",
            ),
            (
                "invalid PCM bits",
                serde_json::json!({
                    "id": "CRY_NIDORAN_M",
                    "path": "mods/new/cries/CRY_NIDORAN_M.pcm",
                    "kind": "cry",
                    "source": "pcm",
                    "pcm_format": {
                        "sample_rate_hz": 32768,
                        "channels": 1,
                        "bits_per_sample": 24
                    }
                }),
                "bits_per_sample must be 8 or 16",
            ),
            (
                "unsupported source",
                serde_json::json!({
                    "id": "MUSIC_ROUTE_29",
                    "path": "mods/new/music/MUSIC_ROUTE_29.mp3",
                    "kind": "music",
                    "source": "mp3",
                }),
                "unknown variant `mp3`",
            ),
        ] {
            let error = serde_json::from_value::<ModpackAudioAsset>(payload)
                .expect_err("malformed audio assets must fail during JSON load")
                .to_string();
            assert!(
                error.contains(expected),
                "{label} produced unexpected error: {error}"
            );
        }

        for (label, payload, expected) in [
            (
                "cry token",
                serde_json::json!({"cry": "CRY NIDORAN_M", "pitch": 128, "length": 64}),
                "audio reference token must be exact ASCII alphanumeric/underscore",
            ),
            (
                "reserved cry token",
                serde_json::json!({"cry": "fallbackCry", "pitch": 128, "length": 64}),
                "audio reference token 'fallbackCry' uses reserved modpack payload prefix",
            ),
            (
                "cry pitch",
                serde_json::json!({"cry": "CRY_NIDORAN_M", "pitch": 32768, "length": 64}),
                "invalid value",
            ),
            (
                "cry length",
                serde_json::json!({"cry": "CRY_NIDORAN_M", "pitch": 128, "length": -32769}),
                "invalid value",
            ),
        ] {
            let error = serde_json::from_value::<PokemonCryMetadata>(payload)
                .expect_err("malformed Pokemon cry metadata must fail during JSON load")
                .to_string();
            assert!(
                error.contains(expected),
                "{label} produced unexpected error: {error}"
            );
        }

        let kind_error = serde_json::from_value::<ModpackAudioKind>(serde_json::json!({
            "music": {
                "fallback_kind": "sound_effect"
            }
        }))
        .expect_err("audio kind must not accept fallback object payloads")
        .to_string();
        assert!(
            kind_error.contains("invalid type") || kind_error.contains("unknown variant"),
            "{kind_error}"
        );

        let source_error = serde_json::from_value::<ModpackAudioSource>(serde_json::json!({
            "midi": {
                "legacy_source": "mp3"
            }
        }))
        .expect_err("audio source must not accept legacy object payloads")
        .to_string();
        assert!(
            source_error.contains("invalid type") || source_error.contains("unknown variant"),
            "{source_error}"
        );

        let playback_id_error =
            serde_json::from_value::<ModpackAudioPlaybackEntry>(serde_json::json!({
                "id": "SFX_ROUTE_29",
                "kind": "music",
                "mode": "sequenced_midi",
                "loop_policy": "loop"
            }))
            .expect_err("playback entries must validate ids against kind")
            .to_string();
        assert!(
            playback_id_error.contains("must use an exact"),
            "{playback_id_error}"
        );

        let playback_reserved_error =
            serde_json::from_value::<ModpackAudioPlaybackEntry>(serde_json::json!({
                "id": "CRY_LEGACY_NIDORAN_M",
                "kind": "cry",
                "mode": "raw_pcm",
                "loop_policy": "once"
            }))
            .expect_err("playback entries must reject reserved ids")
            .to_string();
        assert!(
            playback_reserved_error.contains("uses reserved runtime pack prefix"),
            "{playback_reserved_error}"
        );

        let playback_key_error =
            serde_json::from_value::<ModpackAudioPlaybackPlan>(serde_json::json!({
                "music": {
                    "MUSIC_ROUTE_30": {
                        "id": "MUSIC_ROUTE_29",
                        "kind": "music",
                        "mode": "sequenced_midi",
                        "loop_policy": "loop"
                    }
                },
                "sound_effects": {},
                "cries": {}
            }))
            .expect_err("playback plan map keys must match entry ids")
            .to_string();
        assert!(
            playback_key_error
                .contains("map key MUSIC_ROUTE_30 does not match entry id MUSIC_ROUTE_29"),
            "{playback_key_error}"
        );

        let playback_bucket_error =
            serde_json::from_value::<ModpackAudioPlaybackPlan>(serde_json::json!({
                "music": {
                    "SFX_TACKLE": {
                        "id": "SFX_TACKLE",
                        "kind": "sound_effect",
                        "mode": "sequenced_midi",
                        "loop_policy": "loop"
                    }
                },
                "sound_effects": {},
                "cries": {}
            }))
            .expect_err("playback plan entries must live in their matching bucket")
            .to_string();
        assert!(
            playback_bucket_error.contains("entry SFX_TACKLE has kind SoundEffect, expected Music"),
            "{playback_bucket_error}"
        );
    }

    #[test]
    fn audio_manifest_entries_validate_exact_source_and_pcm_frame_metadata() {
        let valid_pcm = serde_json::json!({
            "id": "CRY_NIDORAN_M",
            "path": "mods/new/cries/CRY_NIDORAN_M.pcm",
            "kind": "cry",
            "source": "pcm",
            "pcm_format": {
                "sample_rate_hz": 32768,
                "channels": 1,
                "bits_per_sample": 16
            },
            "byte_len": 4,
            "payload_hash": "1234abcd",
            "pcm_frame_count": 2
        });
        let entry = serde_json::from_value::<ModpackAudioManifestEntry>(valid_pcm)
            .expect("valid PCM manifest entry");
        assert_eq!(entry.pcm_frame_count, Some(2));

        let mismatched_pcm = serde_json::json!({
            "id": "CRY_NIDORAN_M",
            "path": "mods/new/cries/CRY_NIDORAN_M.pcm",
            "kind": "cry",
            "source": "pcm",
            "pcm_format": {
                "sample_rate_hz": 32768,
                "channels": 1,
                "bits_per_sample": 16
            },
            "byte_len": 3,
            "payload_hash": "1234abcd",
            "pcm_frame_count": 2
        });
        let error = serde_json::from_value::<ModpackAudioManifestEntry>(mismatched_pcm)
            .expect_err("PCM manifest byte length must match frame count")
            .to_string();
        assert!(
            error.contains("byte_len 3 does not match 2 frames of 2 bytes"),
            "{error}"
        );

        let midi_with_pcm_frames = serde_json::json!({
            "id": "MUSIC_ROUTE_29",
            "path": "mods/new/music/MUSIC_ROUTE_29.mid",
            "kind": "music",
            "source": "midi",
            "pcm_format": null,
            "byte_len": 32,
            "payload_hash": "1234abcd",
            "pcm_frame_count": 1
        });
        let error = serde_json::from_value::<ModpackAudioManifestEntry>(midi_with_pcm_frames)
            .expect_err("MIDI manifest entries must not carry PCM frame metadata")
            .to_string();
        assert!(
            error.contains("must not declare pcm_frame_count"),
            "{error}"
        );
    }

    #[test]
    fn audio_playback_plan_loop_policy_must_match_manifest_kind() {
        let assets = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("valid music"),
            ModpackAudioAsset::sound_effect("SFX_TACKLE", "content-packs/test/sfx/SFX_TACKLE.mid")
                .expect("valid sound effect"),
            ModpackAudioAsset::pcm(
                "CRY_NIDORAN_M",
                "content-packs/test/cries/CRY_NIDORAN_M.pcm",
                ModpackAudioKind::Cry,
                ModpackPcmAudioFormat {
                    sample_rate_hz: 32768,
                    channels: 1,
                    bits_per_sample: 16,
                },
            )
            .expect("valid PCM cry"),
        ];
        let compiled_audio = [
            (
                "MUSIC_ROUTE_29".to_string(),
                b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60MTrk\x00\x00\x00\x0c\x00\x90\x3c\x40\x60\x80\x3c\x40\x00\xff\x2f\x00".to_vec(),
            ),
            (
                "SFX_TACKLE".to_string(),
                b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60MTrk\x00\x00\x00\x0c\x00\x90\x3c\x40\x60\x80\x3c\x40\x00\xff\x2f\x00".to_vec(),
            ),
            ("CRY_NIDORAN_M".to_string(), vec![0_u8, 0]),
        ]
        .into_iter()
        .collect();
        let manifest =
            ModpackAudioManifest::from_assets(&assets, &compiled_audio).expect("audio manifest");
        let mut plan = ModpackAudioPlaybackPlan::from_manifest(&manifest).expect("playback plan");

        plan.validate_for_manifest(&manifest)
            .expect("generated playback plan matches manifest");

        plan.music
            .get_mut("MUSIC_ROUTE_29")
            .expect("music entry")
            .loop_policy = ModpackAudioLoopPolicy::Once;
        let music_error = plan
            .validate_for_manifest(&manifest)
            .expect_err("music must loop")
            .to_string();
        assert!(
            music_error.contains("loop policy does not match manifest kind"),
            "{music_error}"
        );

        let mut plan = ModpackAudioPlaybackPlan::from_manifest(&manifest).expect("playback plan");
        plan.sound_effects
            .get_mut("SFX_TACKLE")
            .expect("sfx entry")
            .loop_policy = ModpackAudioLoopPolicy::Loop;
        let sfx_error = plan
            .validate_for_manifest(&manifest)
            .expect_err("sound effects must not loop")
            .to_string();
        assert!(
            sfx_error.contains("loop policy does not match manifest kind"),
            "{sfx_error}"
        );

        let mut plan = ModpackAudioPlaybackPlan::from_manifest(&manifest).expect("playback plan");
        plan.cries
            .get_mut("CRY_NIDORAN_M")
            .expect("cry entry")
            .loop_policy = ModpackAudioLoopPolicy::Loop;
        let cry_error = plan
            .validate_for_manifest(&manifest)
            .expect_err("cries must not loop")
            .to_string();
        assert!(
            cry_error.contains("loop policy does not match manifest kind"),
            "{cry_error}"
        );
    }

    #[test]
    fn definitive_runtime_payloads_require_explicit_pack_fields() {
        let missing_flee_bucket = serde_json::from_str::<FleeMonTables>(r#"{}"#)
            .expect_err("flee mon bucket map must be explicit")
            .to_string();
        assert!(
            missing_flee_bucket.contains("missing field `buckets`"),
            "{missing_flee_bucket}"
        );

        let missing_initialize_bucket =
            serde_json::from_str::<InitializeEventsConfig>(r#"{"eventFlags":[],"engineFlags":[]}"#)
                .expect_err("initialize event buckets must all be explicit")
                .to_string();
        assert!(
            missing_initialize_bucket.contains("missing field `variableSprites`"),
            "{missing_initialize_bucket}"
        );

        let missing_story_maps =
            serde_json::from_str::<StoryEventScriptConstants>(r#"{"global":{}}"#)
                .expect_err("story event constants must declare map constants explicitly")
                .to_string();
        assert!(
            missing_story_maps.contains("missing field `maps`"),
            "{missing_story_maps}"
        );
    }

    #[test]
    fn definitive_runtime_payloads_reject_unknown_pack_fields() {
        for (label, result) in [
            (
                "flee mons",
                serde_json::from_str::<FleeMonTables>(
                    r#"{"buckets":{"always":["RAIKOU"]},"fallback":[]}"#,
                )
                .map(|_| ()),
            ),
            (
                "initialize events",
                serde_json::from_str::<InitializeEventsConfig>(
                    r#"{"eventFlags":[],"engineFlags":[],"variableSprites":{},"legacy":true}"#,
                )
                .map(|_| ()),
            ),
            (
                "story event constants",
                serde_json::from_str::<StoryEventScriptConstants>(
                    r#"{"global":{},"maps":{},"legacy":{}}"#,
                )
                .map(|_| ()),
            ),
            (
                "pokemon cry metadata",
                serde_json::from_str::<PokemonCryMetadata>(
                    r#"{"cry":"CRY_NIDORAN_M","pitch":128,"length":64,"mp3":"nidoran.mp3"}"#,
                )
                .map(|_| ()),
            ),
            (
                "audio asset",
                serde_json::from_str::<ModpackAudioAsset>(
                    r#"{"id":"MUSIC_ROUTE_29","path":"mods/new/music/MUSIC_ROUTE_29.mid","kind":"music","source":"midi","mp3":"route29.mp3"}"#,
                )
                .map(|_| ()),
            ),
            (
                "runtime pokedex entry",
                serde_json::from_str::<RuntimePokedexEntry>(
                    r#"{"species":"NIDORAN_M","classification":"POISON PIN","heightDigits":4,"weightDigits":70,"pages":["It raises its big ears to check its surroundings."],"legacySpecies":"nidoran-m"}"#,
                )
                .map(|_| ()),
            ),
            (
                "runtime spawn point",
                serde_json::from_str::<RuntimeSpawnPoint>(
                    r#"{"identifier":1,"mapConstant":"ROUTE_29","mapName":"Route29","groupId":1,"mapId":1,"tileX":8,"tileY":8,"groupName":"GROUP","metatileX":4,"metatileY":4,"subtileX":0,"subtileY":0,"fallbackMap":"NewBarkTown"}"#,
                )
                .map(|_| ()),
            ),
            (
                "runtime map metadata",
                serde_json::from_str::<RuntimeMapMetadata>(
                    r#"{"constant":"ROUTE_29","name":"Route29","groupName":"GROUP","groupId":1,"mapId":1,"width":20,"height":18,"environment":"route","phoneService":1,"legacyWidth":10}"#,
                )
                .map(|_| ()),
            ),
        ] {
            let error = result.expect_err(label).to_string();
            assert!(error.contains("unknown field"), "{label}: {error}");
        }
    }

    #[test]
    fn frontpic_animation_json_requires_explicit_program_and_command_kind() {
        let missing_commands = serde_json::from_str::<FrontpicAnimProgram>(r#"{}"#)
            .expect_err("frontpic animation programs must declare command lists")
            .to_string();
        assert!(
            missing_commands.contains("missing field `commands`"),
            "{missing_commands}"
        );

        let missing_kind =
            serde_json::from_str::<FrontpicAnimProgram>(r#"{"commands":[{"frame":0}]}"#)
                .expect_err("frontpic animation commands must declare their opcode kind")
                .to_string();
        assert!(
            missing_kind.contains("missing field `kind`"),
            "{missing_kind}"
        );

        let explicit_command =
            serde_json::from_str::<FrontpicAnimProgram>(r#"{"commands":[{"kind":"endanim"}]}"#)
                .expect(
                    "optional command operands may be absent when the opcode does not use them",
                );
        assert_eq!(explicit_command.commands[0].kind, "endanim");

        let unknown_program_field = serde_json::from_str::<FrontpicAnimProgram>(
            r#"{"commands":[{"kind":"endanim"}],"fallback":[]}"#,
        )
        .expect_err("frontpic animation programs must not accept unknown fields")
        .to_string();
        assert!(
            unknown_program_field.contains("unknown field `fallback`"),
            "{unknown_program_field}"
        );

        let unknown_command_field = serde_json::from_str::<FrontpicAnimProgram>(
            r#"{"commands":[{"kind":"endanim","legacyOpcode":"end"}]}"#,
        )
        .expect_err("frontpic animation commands must not accept unknown fields")
        .to_string();
        assert!(
            unknown_command_field.contains("unknown field `legacyOpcode`"),
            "{unknown_command_field}"
        );
    }

    #[test]
    fn playability_json_requires_explicit_rule_fields() {
        let complete_rules = r#"{
          "start_maps":[],
          "start_tiles":[],
          "initial_events":[],
          "initial_items":[],
          "goal_maps":[],
          "goal_events":[],
          "goal_items":[],
          "progression_rules":[],
          "map_access":[],
          "require_all_maps_reachable":false,
          "require_walkable_maps":true
        }"#;
        serde_json::from_str::<PlayabilityRules>(complete_rules)
            .expect("complete playability payload should parse");

        let missing_goal_items = complete_rules.replace(r#"          "goal_items":[],"#, "");
        let missing_goal_items = serde_json::from_str::<PlayabilityRules>(&missing_goal_items)
            .expect_err("goal item rules must be explicit, even when empty")
            .to_string();
        assert!(
            missing_goal_items.contains("missing field `goal_items`"),
            "{missing_goal_items}"
        );

        let missing_requirement_buckets =
            serde_json::from_str::<ProgressionRequirements>(r#"{"maps":["Route29"]}"#)
                .expect_err("progression requirements must declare every bucket")
                .to_string();
        assert!(
            missing_requirement_buckets.contains("missing field `events`"),
            "{missing_requirement_buckets}"
        );

        let missing_grant_buckets =
            serde_json::from_str::<ProgressionGrants>(r#"{"events":["EVENT_DONE"]}"#)
                .expect_err("progression grants must declare every bucket")
                .to_string();
        assert!(
            missing_grant_buckets.contains("missing field `items`"),
            "{missing_grant_buckets}"
        );

        let missing_progression_grants = serde_json::from_str::<ProgressionRule>(
            r#"{"id":"script_Route29_Test","requires":{"events":[],"items":[],"maps":["Route29"]}}"#,
        )
        .expect_err("progression rules must declare grants explicitly")
        .to_string();
        assert!(
            missing_progression_grants.contains("missing field `grants`"),
            "{missing_progression_grants}"
        );

        let missing_map_access_requires =
            serde_json::from_str::<MapAccessRule>(r#"{"map":"Route29"}"#)
                .expect_err("map access rules must declare requirements explicitly")
                .to_string();
        assert!(
            missing_map_access_requires.contains("missing field `requires`"),
            "{missing_map_access_requires}"
        );

        let unknown_rule_field = complete_rules.replace(
            r#"          "require_walkable_maps":true"#,
            r#"          "require_walkable_maps":true,
          "fallback_maps":[]"#,
        );
        let unknown_rule_field = serde_json::from_str::<PlayabilityRules>(&unknown_rule_field)
            .expect_err("playability rules must not accept unknown fields")
            .to_string();
        assert!(
            unknown_rule_field.contains("unknown field `fallback_maps`"),
            "{unknown_rule_field}"
        );

        let unknown_start_field = serde_json::from_str::<PlayabilityStart>(
            r#"{"map":"Route29","tile":{"x":1,"y":2},"legacySpawn":"home"}"#,
        )
        .expect_err("playability starts must not accept unknown fields")
        .to_string();
        assert!(
            unknown_start_field.contains("unknown field `legacySpawn`"),
            "{unknown_start_field}"
        );

        let unknown_requirement_field = serde_json::from_str::<ProgressionRequirements>(
            r#"{"events":[],"items":[],"maps":[],"badges":[]}"#,
        )
        .expect_err("progression requirements must not accept unknown fields")
        .to_string();
        assert!(
            unknown_requirement_field.contains("unknown field `badges`"),
            "{unknown_requirement_field}"
        );

        for (label, result) in [
            (
                "start map",
                serde_json::from_str::<PlayabilityRules>(
                    r#"{
                      "start_maps":["New Bark Town"],
                      "start_tiles":[],
                      "initial_events":[],
                      "initial_items":[],
                      "goal_maps":[],
                      "goal_events":[],
                      "goal_items":[],
                      "progression_rules":[],
                      "map_access":[],
                      "require_all_maps_reachable":false,
                      "require_walkable_maps":true
                    }"#,
                )
                .map(|_| ()),
            ),
            (
                "start tile map",
                serde_json::from_str::<PlayabilityStart>(
                    r#"{"map":" Route29","tile":{"x":1,"y":2}}"#,
                )
                .map(|_| ()),
            ),
            (
                "requirement event",
                serde_json::from_str::<ProgressionRequirements>(
                    r#"{"events":[" EVENT_READY"],"items":[],"maps":[]}"#,
                )
                .map(|_| ()),
            ),
            (
                "grant item",
                serde_json::from_str::<ProgressionGrants>(
                    r#"{"events":[],"items":["PASS\u0007"],"maps":[]}"#,
                )
                .map(|_| ()),
            ),
            (
                "progression id",
                serde_json::from_str::<ProgressionRule>(
                    r#"{"id":"finish route","requires":{"events":[],"items":[],"maps":[]},"grants":{"events":[],"items":[],"maps":[]}}"#,
                )
                .map(|_| ()),
            ),
            (
                "map access map",
                serde_json::from_str::<MapAccessRule>(
                    r#"{"map":"Locked Map","requires":{"events":[],"items":[],"maps":[]}}"#,
                )
                .map(|_| ()),
            ),
        ] {
            let error = result
                .expect_err("malformed playability tokens must fail during JSON load")
                .to_string();
            assert!(
                error.contains("must be exact"),
                "{label} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn modpack_manifest_supports_typed_pokemon_and_map_additions() {
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "johto-plus".to_string(),
                name: "Johto Plus".to_string(),
                version: "0.1.0".to_string(),
                author: Some("Tester".to_string()),
                description: None,
            },
            payload: ModpackPayload {
                pokemon: pokemon_payload(vec![species()]),
                maps: map_payload(vec![MapModule {
                    id: "NEW_ROUTE".to_string(),
                    attributes: MapAttributes {
                        tileset_name: "johto".to_string(),
                        border_block: 1,
                        width: 10,
                        height: 9,
                        connections: vec![MapConnection {
                            direction: "north".to_string(),
                            target_map: "CHERRYGROVE_CITY".to_string(),
                            offset: 0,
                        }],
                        time_of_day: None,
                        phone_service: 0,
                        phone_flag: false,
                        environment: Some("route".to_string()),
                        location: Some("johto".to_string()),
                        music: Some("MUSIC_ROUTE_29".to_string()),
                        palette: None,
                        fishing_group: None,
                        map_constant: Some("NEW_ROUTE".to_string()),
                        map_group_constant: None,
                        blocks_label: None,
                        map_scripts_label: None,
                        map_events_label: None,
                        connection_flags: None,
                    },
                    scripts: BTreeMap::new(),
                    trainer_scripts: BTreeMap::new(),
                    scripted_trainer_battles: Vec::new(),
                    scripted_wild_battles: Vec::new(),
                    script_item_grants: Vec::new(),
                    script_item_checks: Vec::new(),
                    script_item_takes: Vec::new(),
                    script_economy_commands: Vec::new(),
                    gift_pokemon_scripts: Vec::new(),
                    script_flag_commands: Vec::new(),
                    script_scene_commands: Vec::new(),
                    script_audio_commands: Vec::new(),
                    script_block_changes: Vec::new(),
                    script_object_commands: Vec::new(),
                    script_movements: Vec::new(),
                    script_map_commands: Vec::new(),
                    script_text_commands: Vec::new(),
                    script_text_bodies: BTreeMap::new(),
                    script_menu_definitions: BTreeMap::new(),
                    script_vertical_menus: BTreeMap::new(),
                    script_elevators: BTreeMap::new(),
                    script_variable_commands: Vec::new(),
                    script_control_commands: Vec::new(),
                    script_field_pickups: Vec::new(),
                    script_shop_commands: Vec::new(),
                    script_phone_commands: Vec::new(),
                    script_runtime_commands: Vec::new(),
                    script_swarm_commands: Vec::new(),
                    map_script_section_commands: Vec::new(),
                    map_event_section_commands: Vec::new(),
                    scenes: MapSceneTable::default(),
                    events: MapEvents::default(),
                    objects: Vec::new(),
                    blocks: vec![0; 90],
                }]),
                items: item_payload(vec![Item {
                    name: "Spark Charm".to_string(),
                    description: "A charged charm.".to_string(),
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
                    field_menu: "ITEMMENU_NOUSE".to_string(),
                    field_usable: false,
                    battle_menu: "ITEMMENU_NOUSE".to_string(),
                    battle_usable: false,
                    script_name: "SPARK_CHARM".to_string(),
                    consumable: false,
                    tmhm_index: None,
                    tmhm_move: None,
                }]),
                moves: move_payload(vec![Move {
                    name: "SPARK".to_string(),
                    move_type: pokemon_type("ELECTRIC"),
                    power: 40,
                    accuracy: 100,
                    pp: 30,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                }]),
                battle_reward_rules: test_battle_reward_rules(),
                battle_escape_rules: test_battle_escape_rules(),
                step_event_rules: test_step_event_rules(),
                field_moves: test_field_move_catalog(),
                buena_password_categories: test_buena_password_categories(),
                battle_stat_multipliers: test_battle_stat_multipliers(),
                move_priorities: test_move_priorities(),
                type_categories: test_type_categories(),
                type_effectiveness: test_type_effectiveness(),
                weather_modifiers: test_weather_modifiers(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let json = serde_json::to_string(&manifest).expect("serialize modpack");
        let parsed: ModpackManifest = serde_json::from_str(&json).expect("parse modpack");
        assert_eq!(parsed.id(), "johto-plus");
        assert_eq!(parsed.payload.pokemon["NEW_MON"].id, "NEW_MON");
        assert_eq!(parsed.payload.moves["SPARK"].name, "SPARK");
        assert_eq!(parsed.payload.maps["NEW_ROUTE"].blocks.len(), 90);
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_capture_rules_as_definitive_pack_data() {
        let mut data = GameDataSet {
            capture_rules: CaptureRules {
                fast_ball_species: ["GRIMER".to_string()].into_iter().collect(),
                heavy_ball_modifiers: [("ONIX".to_string(), 20)].into_iter().collect(),
                ball_rules: BTreeMap::new(),
                guaranteed_capture_balls: ["MASTER_BALL".to_string()].into_iter().collect(),
                status_bonus: [("SLEEP".to_string(), 10)].into_iter().collect(),
            },
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                capture_rules: CaptureRules {
                    fast_ball_species: ["MAGNEMITE".to_string()].into_iter().collect(),
                    heavy_ball_modifiers: [("KADABRA".to_string(), 40)].into_iter().collect(),
                    ball_rules: BTreeMap::new(),
                    guaranteed_capture_balls: ["PARK_BALL".to_string()].into_iter().collect(),
                    status_bonus: [("FREEZE".to_string(), 10)].into_iter().collect(),
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate capture rules table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate capture rules table"),
            "{error:#}"
        );
        assert!(data.capture_rules.fast_ball_species.contains("GRIMER"));
        assert!(data.capture_rules.heavy_ball_modifiers.contains_key("ONIX"));
        assert!(
            data.capture_rules
                .guaranteed_capture_balls
                .contains("MASTER_BALL")
        );
        assert!(data.capture_rules.status_bonus.contains_key("SLEEP"));
    }

    #[test]
    fn verifier_rejects_runtime_pack_species_case_and_malformed_frontpic_commands() {
        let data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            runtime_spawn_points: [
                (
                    "1".to_string(),
                    RuntimeSpawnPoint {
                        identifier: 0,
                        map_constant: "MISSING_MAP".to_string(),
                        map_name: "MissingMap".to_string(),
                        group_id: 1,
                        map_id: 1,
                        tile_x: 0,
                        tile_y: 0,
                        group_name: String::new(),
                        metatile_x: 0,
                        metatile_y: 0,
                        subtile_x: 0,
                        subtile_y: 0,
                    },
                ),
                (
                    " 2".to_string(),
                    RuntimeSpawnPoint {
                        identifier: 2,
                        map_constant: " ROUTE_29".to_string(),
                        map_name: " Route29".to_string(),
                        group_id: 1,
                        map_id: 1,
                        tile_x: 0,
                        tile_y: 0,
                        group_name: "GROUP_ROUTE_29".to_string(),
                        metatile_x: 0,
                        metatile_y: 0,
                        subtile_x: 0,
                        subtile_y: 0,
                    },
                ),
                ("3".to_string(), test_runtime_spawn_point(3, "Route29")),
                ("4".to_string(), test_runtime_spawn_point(4, "Route29")),
            ]
            .into_iter()
            .collect(),
            runtime_map_metadata: [(
                "ROUTE_29".to_string(),
                test_runtime_map_metadata("ROUTE_29", "Route29"),
            )]
            .into_iter()
            .collect(),
            flee_mons: FleeMonTables {
                buckets: [(
                    "always".to_string(),
                    vec!["new_mon".to_string(), " NEW_MON".to_string()],
                )]
                .into_iter()
                .collect(),
            },
            pc_strings: [("PCString_ChooseaPKMN".to_string(), String::new())]
                .into_iter()
                .collect(),
            menu_icons: [
                ("New_Mon".to_string(), "ICON_NEW_MON".to_string()),
                (" NEW_MON".to_string(), "ICON_NEW_MON".to_string()),
                ("NEW_MON".to_string(), " ICON_NEW_MON".to_string()),
            ]
            .into_iter()
            .collect(),
            pokedex_entries: [
                (
                    "NEW_MON".to_string(),
                    RuntimePokedexEntry {
                        species: "new_mon".to_string(),
                        classification: String::new(),
                        height_digits: 1,
                        weight_digits: 1,
                        pages: Vec::new(),
                    },
                ),
                (
                    " NEW_MON".to_string(),
                    RuntimePokedexEntry {
                        species: " NEW_MON".to_string(),
                        classification: "Spark".to_string(),
                        height_digits: 1,
                        weight_digits: 1,
                        pages: vec![" A small test Pokemon.".to_string()],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            pokemon_frontpic_anim: [
                (
                    "NEW_MON".to_string(),
                    FrontpicAnimProgram {
                        commands: vec![FrontpicAnimCommand {
                            kind: "frame".to_string(),
                            frame: Some(0),
                            duration: None,
                            ..FrontpicAnimCommand::default()
                        }],
                    },
                ),
                (
                    " NEW_MON".to_string(),
                    FrontpicAnimProgram {
                        commands: vec![FrontpicAnimCommand {
                            kind: "endanim".to_string(),
                            ..FrontpicAnimCommand::default()
                        }],
                    },
                ),
            ]
            .into_iter()
            .collect(),
            initialize_events: InitializeEventsConfig {
                event_flags: vec![String::new()],
                ..InitializeEventsConfig::default()
            },
            story_event_script_constants: StoryEventScriptConstants {
                global: [(String::new(), 1)].into_iter().collect(),
                ..StoryEventScriptConstants::default()
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        let codes: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect();

        assert!(codes.contains("runtime_spawn_point_identifier_mismatch"));
        assert!(codes.contains("unknown_runtime_spawn_point_map"));
        assert!(codes.contains("invalid_runtime_spawn_point"));
        assert!(codes.contains("duplicate_runtime_spawn_point_map_binding"));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_runtime_spawn_point_map_binding"
                && diagnostic.subject == "4"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_runtime_spawn_point_map" && diagnostic.subject == " 2"
        }));
        assert!(codes.contains("invalid_initialize_event_flag"));
        assert!(codes.contains("invalid_story_event_script_constant"));
        assert!(codes.contains("unknown_flee_mon_species"));
        assert!(codes.contains("invalid_flee_mon_species"));
        assert!(codes.contains("invalid_pc_string"));
        assert!(codes.contains("unknown_menu_icon_species"));
        assert!(codes.contains("invalid_menu_icon_species"));
        assert!(codes.contains("invalid_menu_icon"));
        assert!(codes.contains("invalid_pokedex_entry_species"));
        assert!(codes.contains("pokedex_entry_species_mismatch"));
        assert!(codes.contains("invalid_pokedex_entry"));
        assert!(codes.contains("invalid_frontpic_anim_species"));
        assert!(codes.contains("malformed_frontpic_anim_command"));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokedex_entry_species" && diagnostic.subject == " NEW_MON"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_frontpic_anim_species" && diagnostic.subject == " NEW_MON"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_story_event_script_constants_without_coercion() {
        let data = GameDataSet {
            story_event_script_constants: StoryEventScriptConstants {
                global: [("TRUE".to_string(), 1)].into_iter().collect(),
                maps: [
                    (
                        "".to_string(),
                        [("EVENT_ONE".to_string(), 1)].into_iter().collect(),
                    ),
                    (
                        "ROUTE_29".to_string(),
                        [("".to_string(), 2)].into_iter().collect(),
                    ),
                ]
                .into_iter()
                .collect(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_story_event_script_constant_map"
                && diagnostic.subject.is_empty()
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_story_event_script_constant"
                && diagnostic.subject == "ROUTE_29:"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_initialize_event_sprites_without_coercion() {
        let data = GameDataSet {
            initialize_events: InitializeEventsConfig {
                event_flags: vec!["EVENT_GOT_STARTER".to_string()],
                engine_flags: vec!["ENGINE_POKEGEAR".to_string()],
                variable_sprites: [("SPRITE_ELM".to_string(), String::new())]
                    .into_iter()
                    .collect(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_initialize_event_sprite"
                && diagnostic.subject == "SPRITE_ELM"
        }));
    }

    #[test]
    fn verifier_rejects_runtime_spawn_point_map_name_mismatches() {
        let data = GameDataSet {
            runtime_map_metadata: [(
                "ROUTE_29".to_string(),
                RuntimeMapMetadata {
                    constant: "ROUTE_29".to_string(),
                    name: "Route29".to_string(),
                    group_name: "GROUP_ROUTE_29".to_string(),
                    group_id: 1,
                    map_id: 1,
                    width: 10,
                    height: 9,
                    environment: "TOWN".to_string(),
                    phone_service: 1,
                },
            )]
            .into_iter()
            .collect(),
            runtime_spawn_points: [(
                "2".to_string(),
                RuntimeSpawnPoint {
                    identifier: 2,
                    map_constant: "ROUTE_29".to_string(),
                    map_name: "WrongMap".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 0,
                    tile_y: 0,
                    group_name: "GROUP_ROUTE_29".to_string(),
                    metatile_x: 0,
                    metatile_y: 0,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]
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
            diagnostic.code == "runtime_spawn_point_map_mismatch" && diagnostic.subject == "2"
        }));
    }

    #[test]
    fn verifier_rejects_runtime_spawn_points_outside_declared_runtime_bounds() {
        let data = GameDataSet {
            runtime_map_metadata: [(
                "ROUTE_29".to_string(),
                RuntimeMapMetadata {
                    constant: "ROUTE_29".to_string(),
                    name: "Route29".to_string(),
                    group_name: "GROUP_ROUTE_29".to_string(),
                    group_id: 1,
                    map_id: 1,
                    width: 3,
                    height: 2,
                    environment: "TOWN".to_string(),
                    phone_service: 1,
                },
            )]
            .into_iter()
            .collect(),
            runtime_spawn_points: [(
                "2".to_string(),
                RuntimeSpawnPoint {
                    identifier: 2,
                    map_constant: "ROUTE_29".to_string(),
                    map_name: "Route29".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 6,
                    tile_y: 2,
                    group_name: "GROUP_ROUTE_29".to_string(),
                    metatile_x: 3,
                    metatile_y: 1,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]
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
            diagnostic.code == "runtime_spawn_point_out_of_bounds" && diagnostic.subject == "2"
        }));
    }

    #[test]
    fn verifier_rejects_runtime_spawn_points_on_unwalkable_tiles() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 1;
        module.blocks = vec![1, 0];
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
            maps: [("Route29".to_string(), module)].into_iter().collect(),
            runtime_map_metadata: [(
                "ROUTE_29".to_string(),
                RuntimeMapMetadata {
                    constant: "ROUTE_29".to_string(),
                    name: "Route29".to_string(),
                    group_name: "GROUP_ROUTE_29".to_string(),
                    group_id: 1,
                    map_id: 1,
                    width: 1,
                    height: 1,
                    environment: "TOWN".to_string(),
                    phone_service: 1,
                },
            )]
            .into_iter()
            .collect(),
            runtime_spawn_points: [(
                "2".to_string(),
                RuntimeSpawnPoint {
                    identifier: 2,
                    map_constant: "ROUTE_29".to_string(),
                    map_name: "Route29".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 0,
                    tile_y: 0,
                    group_name: "GROUP_ROUTE_29".to_string(),
                    metatile_x: 0,
                    metatile_y: 0,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]
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
            diagnostic.code == "unwalkable_runtime_spawn_point"
                && diagnostic.subject == "2"
                && diagnostic
                    .message
                    .contains("non-walkable tile (0, 0) on Route29")
        }));
    }
