    #[test]
    fn check_can_delete_phone_number_compiles_typed_conditional_returns() {
        let scripts = BTreeMap::from([(
            "CheckCanDeletePhoneNumber".to_string(),
            serde_json::json!([
                {"command": "ld", "args": ["a", "c"]},
                {"command": "call", "args": ["GetCallerTrainerClass"]},
                {"command": "ld", "args": ["a", "c"]},
                {"command": "ret", "args": ["nz"]},
                {"command": "ld", "args": ["a", "b"]},
                {"command": "cp", "args": ["PHONECONTACT_MOM"]},
                {"command": "ret", "args": ["z"]},
                {"command": "cp", "args": ["PHONECONTACT_ELM"]},
                {"command": "ret", "args": ["z"]},
                {"command": "ld", "args": ["c", "$1"]},
                {"command": "ret", "args": []}
            ]),
        )]);

        let commands = parse_script_runtime_commands("GlobalPhoneScripts", &scripts)
            .expect("exclude CPU instructions from event runtime commands");
        assert!(commands.is_empty());
        assert_eq!(
            scripts["CheckCanDeletePhoneNumber"]
                .as_array()
                .expect("CPU routine body")
                .iter()
                .filter(|entry| entry["command"] == "ret")
                .map(|entry| {
                    let args = entry["args"]
                        .as_array()
                        .expect("ret args")
                        .iter()
                        .map(|arg| arg.as_str().expect("ret arg"))
                        .collect::<Vec<_>>();
                    match classify_accumulator_callasm_instruction("ret", &args) {
                        Some(AccumulatorCallasmInstruction::Return { condition }) => condition,
                        other => panic!("ret was not classified as a CPU return: {other:?}"),
                    }
                })
                .collect::<Vec<_>>(),
            vec![
                Some(ScriptRuntimeCpuCondition::Nz),
                Some(ScriptRuntimeCpuCondition::Z),
                Some(ScriptRuntimeCpuCondition::Z),
                None,
            ]
        );
    }

    #[test]
    fn trainer_battle_setup_opcodes_materialize_as_runtime_commands() {
        let scripts = BTreeMap::from([(
            "Route44VanceBattle".to_string(),
            serde_json::json!([
                {"command": "winlosstext", "args": ["BirdKeeperVance1BeatenText", "0"]},
                {"command": "loadtrainer", "args": ["BIRD_KEEPER", "VANCE3"]},
                {"command": "startbattle", "args": []}
            ]),
        )]);

        let commands = parse_script_runtime_commands("Route44", &scripts)
            .expect("materialize canonical trainer battle setup opcodes");
        assert_eq!(
            commands,
            vec![
                ScriptRuntimeCommand {
                    command: "winlosstext".to_string(),
                    args: vec![
                        "BirdKeeperVance1BeatenText".to_string(),
                        "0".to_string(),
                    ],
                    source_script: "Route44VanceBattle".to_string(),
                    command_index: 0,
                },
                ScriptRuntimeCommand {
                    command: "loadtrainer".to_string(),
                    args: vec!["BIRD_KEEPER".to_string(), "VANCE3".to_string()],
                    source_script: "Route44VanceBattle".to_string(),
                    command_index: 1,
                },
            ]
        );
    }

    #[test]
    fn scripted_wild_battle_setup_opcode_materializes_as_a_runtime_command() {
        let scripts = BTreeMap::from([(
            "WateredWeirdTreeScript".to_string(),
            serde_json::json!([
                {"command": "loadwildmon", "args": ["SUDOWOODO", "20"]},
                {"command": "startbattle", "args": []}
            ]),
        )]);

        let commands = parse_script_runtime_commands("Route36", &scripts)
            .expect("materialize canonical scripted wild battle setup opcode");
        assert_eq!(
            commands,
            vec![ScriptRuntimeCommand {
                command: "loadwildmon".to_string(),
                args: vec!["SUDOWOODO".to_string(), "20".to_string()],
                source_script: "WateredWeirdTreeScript".to_string(),
                command_index: 0,
            }]
        );
    }

    #[test]
    fn both_source_memcall_pointer_forms_materialize_as_runtime_commands() {
        let scripts = BTreeMap::from([
            (
                "LoadPhoneScriptBank".to_string(),
                serde_json::json!([
                    {"command": "memcall", "args": ["wPhoneScriptBank"]},
                    {"command": "endcallback", "args": []}
                ]),
            ),
            (
                "Script_ReceivePhoneCall".to_string(),
                serde_json::json!([
                    {"command": "memcall", "args": ["wCallerContact", "+", "PHONE_CONTACT_SCRIPT2_BANK"]},
                    {"command": "end", "args": []}
                ]),
            ),
        ]);

        let commands = parse_script_runtime_commands("GlobalScripts", &scripts)
            .expect("materialize both canonical memcall pointer forms");
        assert_eq!(commands.len(), 2);
        assert!(commands.iter().any(|command| {
            command.source_script == "LoadPhoneScriptBank"
                && command.args == ["wPhoneScriptBank"]
        }));
        assert!(commands.iter().any(|command| {
            command.source_script == "Script_ReceivePhoneCall"
                && command.args
                    == [
                        "wCallerContact",
                        "+",
                        "PHONE_CONTACT_SCRIPT2_BANK",
                    ]
        }));
    }

    #[test]
    fn every_source_loadwildmon_is_an_executable_runtime_mutation() {
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load base game data");
        let mut source_count = 0;

        for (map_name, module) in &data.maps {
            for (source_script, body) in &module.scripts {
                let commands = body.as_array().expect("compiled script command array");
                for (command_index, command) in commands.iter().enumerate() {
                    if command.get("command").and_then(serde_json::Value::as_str)
                        != Some("loadwildmon")
                    {
                        continue;
                    }
                    source_count += 1;
                    assert!(
                        module.script_runtime_commands.iter().any(|runtime| {
                            runtime.command == "loadwildmon"
                                && runtime.source_script == *source_script
                                && runtime.command_index == command_index
                        }),
                        "{map_name}/{source_script}:{command_index} must classify loadwildmon as an executable mutation"
                    );
                }
            }
        }

        assert_eq!(source_count, 18, "compiled loadwildmon corpus changed");
    }

    #[test]
    fn movement_references_from_local_scripts_materialize_in_parent_scope() {
        let scripts = BTreeMap::from([
            (
                ".AfterBattle@ParentA".to_string(),
                serde_json::json!([
                    {"command": "applymovement", "args": ["NPC", "SharedGlobalMovement"]},
                    {"command": "applymovement", "args": ["NPC", ".SharedMovement"]},
                    {"command": "end", "args": []}
                ]),
            ),
            (
                ".SharedMovement@ParentA".to_string(),
                serde_json::json!([
                    {"command": "step", "args": ["LEFT"]},
                    {"command": "step_end", "args": []}
                ]),
            ),
            (
                ".AfterBattle@ParentB".to_string(),
                serde_json::json!([
                    {"command": "applymovement", "args": ["NPC", "SharedGlobalMovement"]},
                    {"command": "applymovement", "args": ["NPC", ".SharedMovement"]},
                    {"command": "end", "args": []}
                ]),
            ),
            (
                ".SharedMovement@ParentB".to_string(),
                serde_json::json!([
                    {"command": "step", "args": ["RIGHT"]},
                    {"command": "step_end", "args": []}
                ]),
            ),
            (
                "SharedGlobalMovement".to_string(),
                serde_json::json!([
                    {"command": "step", "args": ["UP"]},
                    {"command": "step_end", "args": []}
                ]),
            ),
        ]);

        let object_commands =
            parse_script_object_commands("ScopeTest", &scripts).expect("parse object commands");
        let movements = parse_script_movements("ScopeTest", &scripts, &object_commands)
            .expect("materialize local and global movement references");
        let movement_keys = movements
            .iter()
            .map(|movement| (movement.label.as_str(), movement.source_script.as_deref()))
            .collect::<BTreeSet<_>>();

        assert_eq!(
            movement_keys,
            BTreeSet::from([
                (".SharedMovement", Some("ParentA")),
                (".SharedMovement", Some("ParentB")),
                ("SharedGlobalMovement", Some("ParentA")),
                ("SharedGlobalMovement", Some("ParentB")),
            ])
        );
    }

    #[test]
    fn generated_azalea_local_post_battle_movement_materializes_without_scope_drift() {
        let path = repository_root_for_tests().join("apps/web/assets/data/maps/AzaleaTown.json");
        let scripts: BTreeMap<String, Value> = serde_json::from_slice(
            &std::fs::read(&path).expect("read generated AzaleaTown story scripts"),
        )
        .expect("parse generated AzaleaTown story scripts");
        let object_commands = parse_script_object_commands("AzaleaTown", &scripts)
            .expect("parse AzaleaTown object commands");
        let movements = parse_script_movements("AzaleaTown", &scripts, &object_commands)
            .expect("materialize AzaleaTown movement references");
        let movement_keys = movements
            .iter()
            .map(|movement| (movement.label.clone(), movement.source_script.clone()))
            .collect::<BTreeSet<_>>();
        let command = object_commands
            .iter()
            .find(|command| {
                command.source_script == ".AfterBattle@AzaleaTownRivalBattleScript"
                    && command.command_index == 6
            })
            .expect("Azalea rival post-battle exit movement command");

        assert_eq!(
            script_object_command_issues(
                command,
                &BTreeMap::from([("AZALEATOWN_RIVAL".to_string(), "-1".to_string())]),
                &BTreeSet::new(),
                &movement_keys,
            ),
            []
        );
        assert!(movement_keys.contains(&(
            "AzaleaTownRivalBattleExitMovement".to_string(),
            Some("AzaleaTownRivalBattleScript".to_string()),
        )));
    }

    #[test]
    fn writeobjectxy_materializes_as_an_exact_typed_object_command() {
        let scripts = BTreeMap::from([(
            "SeenByTrainerScript".to_string(),
            serde_json::json!([
                {"command": "applymovementlasttalked", "args": ["wMovementBuffer"]},
                {"command": "writeobjectxy", "args": ["LAST_TALKED"]},
                {"command": "end", "args": []}
            ]),
        )]);

        let commands = parse_script_object_commands("GlobalScripts", &scripts)
            .expect("parse writeobjectxy");
        let command = commands
            .iter()
            .find(|command| command.command_index == 1)
            .expect("typed writeobjectxy command");

        assert_eq!(command.command, "writeobjectxy");
        assert_eq!(command.object_id.as_deref(), Some("LAST_TALKED"));
        assert_eq!(command.source_script, "SeenByTrainerScript");
    }

    #[test]
    fn surf_start_step_materializes_the_exact_dynamic_movement_buffer() {
        let scripts = BTreeMap::from([(
            "UsedSurfScript".to_string(),
            serde_json::json!([
                {"command": "special", "args": ["SurfStartStep"]},
                {"command": "applymovement", "args": ["PLAYER", "wMovementBuffer"]},
                {"command": "end", "args": []}
            ]),
        )]);
        let object_commands = parse_script_object_commands("GlobalScripts", &scripts)
            .expect("parse exact Surf movement command");
        let movements = parse_script_movements("GlobalScripts", &scripts, &object_commands)
            .expect("materialize SurfStartStep's dynamic movement buffer");

        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].label, "wMovementBuffer");
        assert_eq!(movements[0].source_script.as_deref(), Some("UsedSurfScript"));
        assert_eq!(
            movements[0]
                .steps
                .iter()
                .map(|step| (step.command.as_str(), step.direction.as_deref()))
                .collect::<Vec<_>>(),
            vec![("slow_step", Some("PLAYER_FACING")), ("step_end", None)]
        );
    }

    #[test]
    fn complete_exported_phone_catalog_materializes_without_cpu_parser_leakage() {
        for relative_root in [
            "apps/web/assets/data/phone_scripts",
            "apps/web/assets/data/content-packs/core-modular/phone_scripts",
        ] {
            let phone_root = repository_root_for_tests().join(relative_root);
            let mut paths = std::fs::read_dir(&phone_root)
                .expect("read exported phone catalog")
                .map(|entry| entry.expect("phone catalog entry").path())
                .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
                .collect::<Vec<_>>();
            paths.sort();
            assert!(!paths.is_empty(), "exported phone catalog must not be empty");

            let mut data = GameDataSet::default();
            for path in paths {
                let bytes = std::fs::read(&path).expect("read exported phone script payload");
                data.phone_scripts.push(
                    serde_json::from_slice(&bytes).expect("parse exported phone script payload"),
                );
            }

            data.materialize_global_scripts()
                .expect("materialize the complete exported phone catalog");
            let module = data.global_scripts.expect("global script module");
            assert!(module.scripts.contains_key("Script_ReceivePhoneCall"));
            assert!(module.script_text_bodies.contains_key("PhoneClickText"));
            assert!(
                module
                    .script_text_bodies
                    .contains_key(".PhoneWrongNumberText@WrongNumber")
            );
            assert!(
                !module.scripts.contains_key("CheckCanDeletePhoneNumber"),
                "CPU-only routines must not leak into script-bytecode parsing"
            );
            for (source_script, expected_command, expected_args) in [
                (
                    ".ReportSwarm@RalphPhoneCalleeScript",
                    "getlandmarkname",
                    ["STRING_BUFFER_5", "LANDMARK_ROUTE_32"],
                ),
                (
                    "GinaWantsBattle",
                    "getlandmarkname",
                    ["STRING_BUFFER_5", "LANDMARK_ROUTE_34"],
                ),
                (
                    "WadeWantsBattle2",
                    "getlandmarkname",
                    ["STRING_BUFFER_5", "LANDMARK_ROUTE_31"],
                ),
                (
                    ".AlreadySwarming@AnthonyPhoneCalleeScript",
                    "getlandmarkname",
                    ["STRING_BUFFER_5", "LANDMARK_ROUTE_33"],
                ),
                (
                    ".AlreadySwarming@ArniePhoneCalleeScript",
                    "getlandmarkname",
                    ["STRING_BUFFER_5", "LANDMARK_ROUTE_35"],
                ),
                (
                    ".HasMoney@MomSavingMoney",
                    "getmoney",
                    ["STRING_BUFFER_3", "MOMS_MONEY"],
                ),
                (
                    ".CoolTrainerM@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "COOLTRAINERM"],
                ),
                (
                    ".Beauty@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "BEAUTY"],
                ),
                (
                    ".Grunt@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "GRUNTM"],
                ),
                (
                    ".Teacher@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "TEACHER"],
                ),
                (
                    ".SwimmerF@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "SWIMMERF"],
                ),
                (
                    ".KimonoGirl@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "KIMONO_GIRL"],
                ),
                (
                    ".Skier@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "SKIER"],
                ),
                (
                    ".Medium@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "MEDIUM"],
                ),
                (
                    ".PokefanM@LizGossip",
                    "gettrainerclassname",
                    ["STRING_BUFFER_4", "POKEFANM"],
                ),
            ] {
                assert!(
                    module.script_runtime_commands.iter().any(|command| {
                        command.source_script == source_script
                            && command.command_index == 0
                            && command.command == expected_command
                            && command.args == expected_args
                    }),
                    "{source_script} must materialize exact {expected_command} runtime metadata",
                );
            }
        }
    }

    #[test]
    fn callasm_targets_remain_definitions_without_becoming_script_cursor_bodies() {
        let definitions = BTreeMap::from([
            (
                "FlyScript".to_string(),
                serde_json::json!([
                    {"command": "callasm", "args": ["FlyFromAnim"]},
                    {"command": "end", "args": []}
                ]),
            ),
            (
                "FlyFromAnim".to_string(),
                serde_json::json!([
                    {"command": "ld", "args": ["a", "[wStateFlags]"]},
                    {"command": "ret", "args": []}
                ]),
            ),
        ]);

        let scripts = runtime_module_script_subset(&definitions, ["FlyScript"], false);

        assert_eq!(scripts.keys().cloned().collect::<Vec<_>>(), ["FlyScript"]);
        assert!(definitions.contains_key("FlyFromAnim"));
    }

    #[test]
    fn runtime_script_subset_follows_scoped_local_over_bare_collision() {
        let definitions = BTreeMap::from([
            (
                "ParentScript".to_string(),
                serde_json::json!([
                    {"command": "sjump", "args": [".Done"]}
                ]),
            ),
            (
                ".Done".to_string(),
                serde_json::json!([{"command": "end", "args": []}]),
            ),
            (
                ".Done@ParentScript".to_string(),
                serde_json::json!([{"command": "end", "args": []}]),
            ),
        ]);

        let scripts = runtime_module_script_subset(&definitions, ["ParentScript"], true);

        assert!(scripts.contains_key("ParentScript"));
        assert!(scripts.contains_key(".Done@ParentScript"));
        assert!(!scripts.contains_key(".Done"));
    }

    #[test]
    fn exported_rock_smash_randomwildmon_materializes_as_exact_runtime_command() {
        let path = repository_root_for_tests()
            .join("apps/web/assets/data/story_events/StandardScripts.json");
        let mut payload: Value = serde_json::from_slice(
            &std::fs::read(&path).expect("read exported StandardScripts catalog"),
        )
        .expect("parse exported StandardScripts catalog");
        let catalog = payload
            .get_mut("StandardScripts")
            .and_then(Value::as_object_mut)
            .expect("exported StandardScripts object");
        let smash_rock_pointer = catalog
            .get("StdScripts")
            .and_then(Value::as_array)
            .expect("exported standard-script pointer table")
            .iter()
            .find(|entry| {
                entry
                    .get("args")
                    .and_then(Value::as_array)
                    .is_some_and(|args| {
                        args == &[Value::String("SmashRockScript".to_string())]
                    })
            })
            .cloned()
            .expect("source-exact SmashRockScript pointer");
        catalog.insert(
            "StdScripts".to_string(),
            Value::Array(vec![smash_rock_pointer]),
        );
        catalog.insert("GlobalScriptRoots".to_string(), Value::Array(Vec::new()));
        let mut data = GameDataSet {
            story_events: vec![payload],
            ..GameDataSet::default()
        };

        data.materialize_global_scripts()
            .expect("materialize source-exact Rock Smash standard script");
        let module = data.global_scripts.expect("global standard-script module");
        let command = module
            .script_runtime_commands
            .iter()
            .find(|command| {
                command.source_script == "RockSmashScript"
                    && command.command_index == 11
            })
            .expect("RockSmashScript:11 must have typed runtime metadata");

        assert_eq!(command.command, "randomwildmon");
        assert_eq!(command.args, Vec::<String>::new());
    }

    #[test]
    fn global_phone_landmark_and_money_commands_execute_through_pack_owned_data() {
        let mut module = test_map_module("RuntimePhoneMap", "RUNTIME_PHONE_MAP", None);
        module.scripts.insert(
            "RuntimePhoneMapScript".to_string(),
            serde_json::json!([{"command": "end", "args": []}]),
        );
        let mut data = GameDataSet {
            maps: [("RuntimePhoneMap".to_string(), module.clone())]
                .into_iter()
                .collect(),
            phone_scripts: vec![serde_json::json!({
                ".ReportSwarm@RalphPhoneCalleeScript": [
                    {"command": "getlandmarkname", "args": ["STRING_BUFFER_5", "LANDMARK_ROUTE_32"]},
                    {"command": "end", "args": []}
                ],
                ".HasMoney@MomSavingMoney": [
                    {"command": "getmoney", "args": ["STRING_BUFFER_3", "MOMS_MONEY"]},
                    {"command": "end", "args": []}
                ],
                ".CoolTrainerM@LizGossip": [
                    {"command": "gettrainerclassname", "args": ["STRING_BUFFER_4", "COOLTRAINERM"]},
                    {"command": "end", "args": []}
                ],
                ".CurrentMap@MomPhoneCalleeScript": [
                    {"command": "getcurlandmarkname", "args": ["STRING_BUFFER_5"]},
                    {"command": "end", "args": []}
                ]
            })],
            trainer_class_names: BTreeMap::from([(
                "COOLTRAINERM".to_string(),
                "COOLTRAINER".to_string(),
            )]),
            pokegear_landmarks: crystal_core::models::display_metadata::PokegearLandmarksPayload {
                landmarks: vec![
                    crystal_core::models::display_metadata::PokegearLandmark {
                        id: 8,
                        constant: "LANDMARK_ROUTE_32".to_string(),
                        label: "ROUTE_32".to_string(),
                        name: "ROUTE 32".to_string(),
                        x: 92,
                        y: 76,
                        region: "JOHTO".to_string(),
                    },
                    crystal_core::models::display_metadata::PokegearLandmark {
                        id: 1,
                        constant: "LANDMARK_NEW_BARK_TOWN".to_string(),
                        label: "NEW_BARK_TOWN".to_string(),
                        name: "NEW BARK TOWN".to_string(),
                        x: 100,
                        y: 76,
                        region: "JOHTO".to_string(),
                    },
                ],
                map_to_landmark: BTreeMap::from([(
                    "RuntimePhoneMap".to_string(),
                    "LANDMARK_NEW_BARK_TOWN".to_string(),
                )]),
            },
            ..GameDataSet::default()
        };
        data.materialize_global_scripts()
            .expect("materialize exact global phone commands");
        let mut state = GameState {
            moms_money: 54_321,
            ..GameState::default()
        };
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimePhoneMap".to_string(),
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

        for source_script in [
            ".ReportSwarm@RalphPhoneCalleeScript",
            ".HasMoney@MomSavingMoney",
            ".CoolTrainerM@LizGossip",
        ] {
            data.apply_script_runtime_command_in_session(
                &mut state,
                &mut session,
                "RuntimePhoneMap",
                source_script,
                0,
                ScriptRuntimeInputs::default(),
            )
            .expect("execute exact global phone buffer command");
        }

        assert_eq!(
            state.script_runtime.named_buffers.get("STRING_BUFFER_5"),
            Some(&"ROUTE 32".to_string())
        );
        assert_eq!(
            state.script_runtime.named_buffers.get("STRING_BUFFER_3"),
            Some(&"54321".to_string())
        );
        assert_eq!(
            state.script_runtime.named_buffers.get("STRING_BUFFER_4"),
            Some(&"COOLTRAINER".to_string())
        );

        data.apply_script_runtime_command_in_session(
            &mut state,
            &mut session,
            "RuntimePhoneMap",
            ".CurrentMap@MomPhoneCalleeScript",
            0,
            ScriptRuntimeInputs::default(),
        )
        .expect("execute exact current-map landmark buffer command");
        assert_eq!(
            state.script_runtime.named_buffers.get("STRING_BUFFER_5"),
            Some(&"NEW BARK TOWN".to_string())
        );
    }

    #[test]
    fn global_phone_materialization_keeps_script_prefixes_and_excludes_cpu_bodies() {
        let mut data = GameDataSet {
            phone_scripts: vec![serde_json::json!({
                "PhoneCallerScript": [
                    {"command": "farwritetext", "args": ["PhoneCallerText"]},
                    {"command": "end", "args": []}
                ],
                "PhoneCallerText": [
                    {"command": "text_far", "args": ["_PhoneCallerText"]},
                    {"command": "text_end", "args": []}
                ],
                "Script_SpecialBillCall": [
                    {"command": "callasm", "args": [".LoadBillScript"]},
                    {"command": "sjump", "args": ["Script_ReceivePhoneCall"]},
                    {"command": "ld", "args": ["e", "PHONE_BILL"]},
                    {"command": "jp", "args": ["LoadCallerScript"]}
                ],
                ".LoadBillScript@Script_SpecialBillCall": [
                    {"command": "ld", "args": ["e", "PHONE_BILL"]},
                    {"command": "ret", "args": ["nz"]}
                ],
                "Script_ReceivePhoneCall": [
                    {"command": "end", "args": []}
                ]
            })],
            ..GameDataSet::default()
        };

        data.materialize_global_scripts()
            .expect("materialize script-only global phone module");
        let module = data.global_scripts.expect("global script module");

        assert_eq!(
            module.scripts["Script_SpecialBillCall"],
            serde_json::json!([
                {"command": "callasm", "args": [".LoadBillScript"]},
                {"command": "sjump", "args": ["Script_ReceivePhoneCall"]}
            ])
        );
        assert!(!module
            .scripts
            .contains_key(".LoadBillScript@Script_SpecialBillCall"));
        assert!(module.script_runtime_commands.iter().any(|command| {
            command.source_script == "Script_SpecialBillCall" && command.command == "callasm"
        }));
        assert!(module.script_runtime_commands.iter().any(|command| {
            command.source_script == "DecorationDesc_TownMapPoster"
                && command.command == "special"
                && command.args == ["OverworldTownMap"]
        }));
        assert!(module.script_text_bodies.contains_key("PhoneCallerText"));
    }

    #[test]
    fn map_module_payloads_validate_text_control_map_and_runtime_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29Script".to_string(),
            serde_json::json!([
                {
                    "command": "end",
                    "args": []
                }
            ]),
        );
        module.script_text_bodies.insert(
            "Route29Text".to_string(),
            ScriptTextBody {
                label: "Route29Text".to_string(),
                commands: Vec::new(),
            },
        );
        module.script_text_commands = vec![
            ScriptTextCommand {
                command: "opentext".to_string(),
                text_label: None,
                source_script: "Route29Script".to_string(),
                command_index: 0,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("Route29Text".to_string()),
                source_script: "Route29Script".to_string(),
                command_index: 1,
            },
        ];
        module.script_control_commands = vec![ScriptControlCommand {
            command: "sjump".to_string(),
            compare_value: None,
            target_label: Some("Route29Script".to_string()),
            resolved_target_script: Some("Route29Script".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 2,
        }];
        module.script_map_commands = vec![
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("Route29".to_string()),
                x: Some(0),
                y: Some(0),
                facing: None,
                map_setup: None,
                source_script: "Route29Script".to_string(),
                command_index: 3,
            },
            ScriptMapCommand {
                command: "newloadmap".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: Some("MAPSETUP_WARP".to_string()),
                source_script: "Route29Script".to_string(),
                command_index: 4,
            },
        ];
        module.script_runtime_commands = vec![ScriptRuntimeCommand {
            command: "special".to_string(),
            args: vec!["HealParty".to_string()],
            source_script: "Route29Script".to_string(),
            command_index: 5,
        }];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical text, control, map, and runtime commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_text_commands = vec![ScriptTextCommand {
            command: "opentext".to_string(),
            text_label: Some("Route29Text".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("no-label text commands must not include labels")
            .to_string();
        assert!(
            error.contains("script text command opentext must not declare text_label"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_text_commands = vec![ScriptTextCommand {
            command: "writetext".to_string(),
            text_label: Some("Route 29 Text".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("text labels must be exact")
            .to_string();
        assert!(
            error.contains(
                "script text label must be exact ASCII label syntax, found \"Route 29 Text\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_control_commands = vec![ScriptControlCommand {
            command: "sjump".to_string(),
            compare_value: Some("TRUE".to_string()),
            target_label: Some("Route29Script".to_string()),
            resolved_target_script: Some("Route29Script".to_string()),
            source_script: "Route29Script".to_string(),
            command_index: 8,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("jump commands must not include compare values")
            .to_string();
        assert!(
            error.contains("script control command 'sjump' has unexpected compare value"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_map_commands = vec![ScriptMapCommand {
            command: "warpfacing".to_string(),
            target_map: Some("Route29".to_string()),
            x: Some(0),
            y: Some(0),
            facing: Some("SIDEWAYS".to_string()),
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 9,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("warp facing directions must be known")
            .to_string();
        assert!(
            error.contains("unknown script facing direction 'SIDEWAYS'"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_map_commands = vec![ScriptMapCommand {
            command: "warp".to_string(),
            target_map: None,
            x: Some(0),
            y: Some(0),
            facing: None,
            map_setup: None,
            source_script: "Route29Script".to_string(),
            command_index: 10,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("warp commands must include a target")
            .to_string();
        assert!(
            error.contains("script map command 'warp' is missing a target map"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_runtime_commands = vec![ScriptRuntimeCommand {
            command: "special".to_string(),
            args: Vec::new(),
            source_script: "Route29Script".to_string(),
            command_index: 11,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("runtime commands must have exact arity")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script runtime command 11 in 'Route29Script' is malformed: WrongArgCount"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_runtime_commands = vec![ScriptRuntimeCommand {
            command: "special ".to_string(),
            args: vec!["HealParty".to_string()],
            source_script: "Route29Script".to_string(),
            command_index: 12,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("runtime command names must be exact")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script runtime command 12 in 'Route29Script' is malformed: PaddedCommand"
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_text_bodies_and_menu_definitions() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_text_bodies.insert(
            "Route29Text".to_string(),
            ScriptTextBody {
                label: "Route29Text".to_string(),
                commands: vec![
                    ScriptTextBodyCommand {
                        command: "text".to_string(),
                        args: vec!["Hello".to_string()],
                        command_index: 0,
                    },
                    ScriptTextBodyCommand {
                        command: "done".to_string(),
                        args: Vec::new(),
                        command_index: 1,
                    },
                ],
            },
        );
        module.script_menu_definitions.insert(
            "Route29Menu".to_string(),
            ScriptMenuDefinition {
                label: "Route29Menu".to_string(),
                commands: vec![
                    ScriptMenuCommand {
                        command: "menu_coords".to_string(),
                        args: vec![
                            "0".to_string(),
                            "0".to_string(),
                            "10".to_string(),
                            "8".to_string(),
                        ],
                        command_index: 0,
                    },
                    ScriptMenuCommand {
                        command: "dw".to_string(),
                        args: vec!["Route29MenuData".to_string()],
                        command_index: 1,
                    },
                ],
            },
        );
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical text bodies and menu definitions must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_text_bodies.insert(
            "Route29Text".to_string(),
            ScriptTextBody {
                label: "Route29 Text".to_string(),
                commands: Vec::new(),
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("text body labels must match exact keys")
            .to_string();
        assert!(
            error.contains(
                "script text label must be exact ASCII label syntax, found \"Route29 Text\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_text_bodies.insert(
            "Route29Text".to_string(),
            ScriptTextBody {
                label: "Route29Text".to_string(),
                commands: vec![ScriptTextBodyCommand {
                    command: "done".to_string(),
                    args: vec!["extra".to_string()],
                    command_index: 2,
                }],
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("text body command arity must be exact")
            .to_string();
        assert!(
            error.contains("script text body command done has 1 args, expected 0"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_text_bodies.insert(
            "Route29Text".to_string(),
            ScriptTextBody {
                label: "Route29Text".to_string(),
                commands: vec![ScriptTextBodyCommand {
                    command: "text".to_string(),
                    args: vec![" Hello".to_string()],
                    command_index: 3,
                }],
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("text body args must be exact")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script text body 'Route29Text' command 3 arg 0 ' Hello' must be exact, non-empty, and untrimmed"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_menu_definitions.insert(
            "Route29Menu".to_string(),
            ScriptMenuDefinition {
                label: "Route29Menu".to_string(),
                commands: vec![ScriptMenuCommand {
                    command: "menu_coords".to_string(),
                    args: vec!["0".to_string(), "0".to_string()],
                    command_index: 4,
                }],
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("menu command arity must be exact")
            .to_string();
        assert!(
            error.contains("script menu command menu_coords has 2 args, expected {4}"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_menu_definitions.insert(
            "Route29Menu".to_string(),
            ScriptMenuDefinition {
                label: "Route29Menu".to_string(),
                commands: vec![ScriptMenuCommand {
                    command: "dw".to_string(),
                    args: vec![" Route29MenuData".to_string()],
                    command_index: 5,
                }],
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("menu command args must be exact")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script menu definition 'Route29Menu' command 5 arg 0 ' Route29MenuData' must be exact, non-empty, and untrimmed"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_menu_definitions.insert(
            "Route29Menu".to_string(),
            ScriptMenuDefinition {
                label: "Route29Menu".to_string(),
                commands: vec![ScriptMenuCommand {
                    command: "menu_coords".to_string(),
                    args: vec![
                        "0".to_string(),
                        "TEXTBOX_Y".to_string(),
                        "SCREEN_EDGE".to_string(),
                        "SCREEN_HEIGHT - 1".to_string(),
                    ],
                    command_index: 6,
                }],
            },
        );
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("screen-edge menu coordinate constants must be accepted");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_menu_definitions.insert(
            "Route29Menu".to_string(),
            ScriptMenuDefinition {
                label: "Route29Menu".to_string(),
                commands: vec![ScriptMenuCommand {
                    command: "menu_coords".to_string(),
                    args: vec![
                        "0".to_string(),
                        "TEXTBOX_Y".to_string(),
                        "SCREEN_WIDTH  - 1".to_string(),
                        " SCREEN_HEIGHT".to_string(),
                    ],
                    command_index: 7,
                }],
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("menu coordinate expressions must not be whitespace-normalized")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script menu definition 'Route29Menu' command 7 arg 3 ' SCREEN_HEIGHT' must be exact, non-empty, and untrimmed"
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_gift_and_battle_records() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29GiftLabel".to_string(),
            serde_json::json!([
                {
                    "command": "end",
                    "args": []
                }
            ]),
        );
        let mut trainer_request =
            TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", "EVENT_BEAT_YOUNGSTER_JOEY");
        trainer_request.seen_text = "Route29SeenText".to_string();
        trainer_request.win_text = "Route29WinText".to_string();
        trainer_request.source_script = "Route29TrainerScript".to_string();
        module
            .trainer_scripts
            .insert("Route29TrainerScript".to_string(), trainer_request.clone());
        module.scripted_trainer_battles = vec![ScriptedTrainerBattle {
            source_script: "Route29TrainerScript".to_string(),
            loadtrainer_command_index: 1,
            startbattle_command_index: 2,
            request: trainer_request,
        }];
        let mut wild_request = StaticWildBattleRequest::new("PIDGEY", 3);
        wild_request.source_script = "Route29WildScript".to_string();
        module.scripted_wild_battles = vec![ScriptedWildBattle {
            source_script: "Route29WildScript".to_string(),
            loadwildmon_command_index: 3,
            startbattle_command_index: 4,
            request: wild_request,
        }];
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "TOGEPI".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: Some("BERRY".to_string()),
            nickname_label: Some("Route29GiftLabel".to_string()),
            ot_label: Some("Route29GiftLabel".to_string()),
            source_script: "Route29GiftScript".to_string(),
            command_index: 5,
            egg: false,
        }];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical gift and battle records must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "TO GEPI".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: None,
            nickname_label: None,
            ot_label: None,
            source_script: "Route29GiftScript".to_string(),
            command_index: 6,
            egg: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("gift species ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "gift Pokemon token must be exact ASCII alphanumeric/underscore, found \"TO GEPI\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "fallbackGift".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: None,
            nickname_label: None,
            ot_label: None,
            source_script: "Route29GiftScript".to_string(),
            command_index: 7,
            egg: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("gift reserved species ids must fail")
            .to_string();
        assert!(
            error.contains(
                "gift Pokemon token must be exact ASCII alphanumeric/underscore, found \"fallbackGift\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "TOGEPI".to_string(),
            level_token: " 5".to_string(),
            level: 5,
            held_item_id: None,
            nickname_label: None,
            ot_label: None,
            source_script: "Route29GiftScript".to_string(),
            command_index: 8,
            egg: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("gift level tokens must be exact")
            .to_string();
        assert!(
            error.contains("gift Pokemon value token must be exact visible ASCII, found \" 5\""),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "TOGEPI".to_string(),
            level_token: "0".to_string(),
            level: 0,
            held_item_id: None,
            nickname_label: None,
            ot_label: None,
            source_script: "Route29GiftScript".to_string(),
            command_index: 9,
            egg: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("gift level must be nonzero")
            .to_string();
        assert!(
            error.contains("gift Pokemon level must be positive"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.gift_pokemon_scripts = vec![GiftPokemonScript {
            species_id: "TOGEPI".to_string(),
            level_token: "5".to_string(),
            level: 5,
            held_item_id: None,
            nickname_label: Some("MissingGiftLabel".to_string()),
            ot_label: None,
            source_script: "Route29GiftScript".to_string(),
            command_index: 10,
            egg: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("gift labels must resolve within map scripts")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' gift Pokemon command 10 nickname label 'MissingGiftLabel' is not a loaded script in map 'Route29'"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.trainer_scripts.insert(
            " Route29TrainerScript".to_string(),
            TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", ""),
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("trainer script keys must be exact")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' trainer script key ' Route29TrainerScript' must be exact, non-empty, and untrimmed"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripted_trainer_battles = vec![ScriptedTrainerBattle {
            source_script: "Route29TrainerScript".to_string(),
            loadtrainer_command_index: 10,
            startbattle_command_index: 11,
            request: TrainerBattleRequest::new("YOUNG STER", "YOUNGSTER_JOEY", ""),
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scripted trainer classes must be exact")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' scripted trainer battle command 10 trainer class 'YOUNG STER' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripted_trainer_battles = vec![ScriptedTrainerBattle {
            source_script: "Route29TrainerScript".to_string(),
            loadtrainer_command_index: 11,
            startbattle_command_index: 12,
            request: TrainerBattleRequest::new("YOUNGSTER", "legacyTrainer", ""),
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scripted trainer reserved trainer ids must fail")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' scripted trainer battle command 11 trainer id 'legacyTrainer' uses reserved modpack payload prefix"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        let mut request = TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", "");
        request.battle_type = "BATTLETYPE_WILD".to_string();
        module.scripted_trainer_battles = vec![ScriptedTrainerBattle {
            source_script: "Route29TrainerScript".to_string(),
            loadtrainer_command_index: 13,
            startbattle_command_index: 14,
            request,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scripted trainer battle type must be trainer")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' scripted trainer battle command 13 battle type 'BATTLETYPE_WILD' is not a trainer battle type in map 'Route29'"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        let request = StaticWildBattleRequest::new("fallbackWild", 3);
        module.scripted_wild_battles = vec![ScriptedWildBattle {
            source_script: "Route29WildScript".to_string(),
            loadwildmon_command_index: 14,
            startbattle_command_index: 15,
            request,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scripted wild reserved species ids must fail")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' scripted wild battle command 14 species id 'fallbackWild' uses reserved modpack payload prefix"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        let mut request = StaticWildBattleRequest::new("PIDGEY", 0);
        request.battle_type = "BATTLETYPE_NORMAL".to_string();
        module.scripted_wild_battles = vec![ScriptedWildBattle {
            source_script: "Route29WildScript".to_string(),
            loadwildmon_command_index: 15,
            startbattle_command_index: 16,
            request,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scripted wild levels must be nonzero")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' scripted wild battle command 15 level must be greater than zero"
            ),
            "{error}"
        );

    }

    #[test]
    fn content_pack_payloads_reject_duplicate_primary_runtime_ids() {
        let mut data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            ..GameDataSet::default()
        };
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![species()])).expect("species json"),
            )
            .expect_err("duplicate Pokemon payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate Pokemon species 'NEW_MON'"),
            "{error:#}"
        );

        let mut invalid_species = species();
        invalid_species.id = " NEW_MON".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![invalid_species]))
                    .expect("invalid species json"),
            )
            .expect_err("Pokemon species ids must not be trimmed");
        assert!(
            format!("{error:#}")
                .contains("Pokemon token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut invalid_species = species();
        invalid_species.item1 = Some(" BERRY".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![invalid_species]))
                    .expect("invalid species json"),
            )
            .expect_err("Pokemon species held item ids must not be trimmed");
        assert!(
            format!("{error:#}")
                .contains("Pokemon token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut invalid_species = species();
        invalid_species.item1 = Some("legacyBerry".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![invalid_species]))
                    .expect("invalid species json"),
            )
            .expect_err("Pokemon species reserved held item ids must fail");
        assert!(
            format!("{error:#}")
                .contains("Pokemon token 'legacyBerry' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        let mut invalid_species = species();
        invalid_species.tmhm_learnset = vec!["THUNDERBOLT\u{0007}".to_string()];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![invalid_species]))
                    .expect("invalid species json"),
            )
            .expect_err("Pokemon species TM/HM move ids must not contain control characters");
        assert!(
            format!("{error:#}")
                .contains("Pokemon token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut invalid_species = species();
        invalid_species.tmhm_learnset = vec!["fallbackTmMove".to_string()];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![invalid_species]))
                    .expect("invalid species json"),
            )
            .expect_err("Pokemon species reserved TM/HM move ids must fail");
        assert!(
            format!("{error:#}")
                .contains("Pokemon token 'fallbackTmMove' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        let malformed_species_tokens: Vec<(&str, fn(&mut PokemonSpecies), &str)> = vec![
            (
                "primary type",
                |species| species.type1 = "FIRE TYPE".to_string(),
                "Pokemon token must be exact ASCII alphanumeric/underscore",
            ),
            (
                "secondary type",
                |species| species.type2 = " WATER".to_string(),
                "Pokemon token must be exact ASCII alphanumeric/underscore",
            ),
            (
                "growth rate",
                |species| species.growth_rate = "GROWTH MEDIUM_FAST".to_string(),
                "Pokemon token must be exact ASCII alphanumeric/underscore",
            ),
            (
                "primary egg group",
                |species| species.egg_group1 = "EGG MONSTER".to_string(),
                "Pokemon token must be exact ASCII alphanumeric/underscore",
            ),
            (
                "secondary egg group",
                |species| species.egg_group2 = "EGG_WATER_1 ".to_string(),
                "Pokemon token must be exact ASCII alphanumeric/underscore",
            ),
            (
                "ability",
                |species| species.ability = "MOD ABILITY".to_string(),
                "Pokemon token must be exact ASCII alphanumeric/underscore",
            ),
        ];

        for (label, mutate, expected) in malformed_species_tokens {
            let mut invalid_species = species();
            mutate(&mut invalid_species);
            let error = match GameDataSet::default().apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![invalid_species]))
                    .expect("invalid species json"),
            ) {
                Ok(()) => panic!("{label} must fail at species load time"),
                Err(error) => error,
            };
            assert!(
                format!("{error:#}").contains(expected),
                "{label} produced unexpected error: {error:#}"
            );
        }

        let mut reserved_species = species();
        reserved_species.id = "fallback_species".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::to_value(pokemon_payload(vec![reserved_species]))
                    .expect("reserved species json"),
            )
            .expect_err("reserved Pokemon species ids must fail at content-pack load time");
        assert!(
            format!("{error:#}")
                .contains("Pokemon token 'fallback_species' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Moves,
                serde_json::to_value(move_payload(vec![test_move("TACKLE")])).expect("move json"),
            )
            .expect_err("duplicate move payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate move 'TACKLE'"),
            "{error:#}"
        );

        let mut invalid_move = test_move("TACKLE");
        invalid_move.effect = "NORMAL HIT".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Moves,
                serde_json::to_value(move_payload(vec![invalid_move])).expect("invalid move json"),
            )
            .expect_err("move effects must be exact tokens");
        assert!(
            format!("{error:#}").contains("move token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let reserved_move = test_move("fallback_move");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Moves,
                serde_json::to_value(move_payload(vec![reserved_move]))
                    .expect("reserved move json"),
            )
            .expect_err("reserved move ids must fail at content-pack load time");
        assert!(
            format!("{error:#}")
                .contains("move token 'fallback_move' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        let growth_curve = crystal_core::systems::experience::GrowthRateCurve {
            id: "GROWTH_MEDIUM_FAST".to_string(),
            numerator: 1,
            denominator: 1,
            quadratic: 0,
            linear: 0,
            constant: 0,
        };
        let mut data = GameDataSet {
            growth_rates: [("GROWTH_MEDIUM_FAST".to_string(), growth_curve.clone())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::GrowthRates,
                serde_json::to_value(growth_rate_payload(vec![growth_curve.clone()]))
                    .expect("growth curve json"),
            )
            .expect_err("duplicate growth rate payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate growth rate curve 'GROWTH_MEDIUM_FAST'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::GrowthRates,
                serde_json::json!({
                    "WRONG_GROWTH": growth_curve
                }),
            )
            .expect_err("growth rate payload key must match id");
        assert!(
            format!("{error:#}").contains(
                "growth rate key 'WRONG_GROWTH' does not match record id 'GROWTH_MEDIUM_FAST'"
            ),
            "{error:#}"
        );

        let mut invalid_growth_curve = growth_curve.clone();
        invalid_growth_curve.id = " GROWTH_MEDIUM_FAST".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::GrowthRates,
                serde_json::to_value(growth_rate_payload(vec![invalid_growth_curve]))
                    .expect("invalid growth curve json"),
            )
            .expect_err("growth rate ids must not be trimmed");
        assert!(
            format!("{error:#}")
                .contains("growth-rate id must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut invalid_growth_curve = growth_curve.clone();
        invalid_growth_curve.denominator = 0;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::GrowthRates,
                serde_json::to_value(growth_rate_payload(vec![invalid_growth_curve]))
                    .expect("invalid growth curve json"),
            )
            .expect_err("growth rate denominators must be nonzero");
        assert!(
            format!("{error:#}")
                .contains("growth-rate curve GROWTH_MEDIUM_FAST has zero denominator"),
            "{error:#}"
        );

        let mut reserved_growth_curve = growth_curve.clone();
        reserved_growth_curve.id = "legacyGrowth".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::GrowthRates,
                serde_json::to_value(growth_rate_payload(vec![reserved_growth_curve]))
                    .expect("reserved growth curve json"),
            )
            .expect_err("reserved growth rate ids must fail at content-pack load time");
        assert!(
            format!("{error:#}")
                .contains("growth-rate id must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Items,
                serde_json::to_value(item_payload(vec![test_item("POTION")])).expect("item json"),
            )
            .expect_err("duplicate item payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate item 'POTION'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Items,
                serde_json::json!({
                    "WRONG_ITEM": test_item("POTION")
                }),
            )
            .expect_err("item payload key must match script_name");
        assert!(
            format!("{error:#}")
                .contains("item key 'WRONG_ITEM' does not match record script_name 'POTION'"),
            "{error:#}"
        );

        let mut invalid_item = test_item("POTION");
        invalid_item.effect = "HEAL HP".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Items,
                serde_json::to_value(item_payload(vec![invalid_item])).expect("invalid item json"),
            )
            .expect_err("item effects must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "item token must be exact ASCII alphanumeric/underscore, found \"HEAL HP\""
            ),
            "{error:#}"
        );

        let reserved_item = test_item("fallback_item");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Items,
                serde_json::to_value(item_payload(vec![reserved_item]))
                    .expect("reserved item json"),
            )
            .expect_err("reserved item ids must fail at content-pack load time");
        assert!(
            format!("{error:#}")
                .contains("item token 'fallback_item' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
                ContentPackCategory::Trainers,
                serde_json::json!({
                    "YOUNGSTER_JOEY": test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER")
                }),
            )
            .expect("apply first trainer payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Trainers,
                serde_json::json!({
                    "YOUNGSTER_JOEY": test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER")
                }),
            )
            .expect_err("duplicate trainer payload must not overwrite");
        assert!(
            format!("{error:#}").contains("trainer id 'YOUNGSTER_JOEY' is duplicated"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Trainers,
                serde_json::json!({
                    "YOUNGSTER_JOEY": test_trainer("BUG_CATCHER_DON", "MUSIC_HIKER_ENCOUNTER")
                }),
            )
            .expect_err("trainer payload key must match trainer_id");
        assert!(
            format!("{error:#}").contains(
                "trainer key 'YOUNGSTER_JOEY' does not match record trainer_id 'BUG_CATCHER_DON'"
            ),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Trainers,
                serde_json::to_value(vec![test_trainer(
                    "YOUNGSTER_JOEY",
                    "MUSIC_HIKER_ENCOUNTER",
                )])
                .expect("trainer json"),
            )
            .expect_err("trainer payload must not use array compatibility shape");
        assert!(
            format!("{error:#}").contains("parse object-map payload"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Trainers,
                serde_json::json!({
                    "legacyTrainer": test_trainer("legacyTrainer", "MUSIC_HIKER_ENCOUNTER")
                }),
            )
            .expect_err("reserved trainer ids must fail at content-pack load time");
        assert!(
            format!("{error:#}")
                .contains("trainer token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        for (trainer, expected) in [
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.trainer_class = "YOUNGSTER ".to_string();
                    trainer
                },
                "token must be exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.encounter_music = " MUSIC_HIKER_ENCOUNTER".to_string();
                    trainer
                },
                "exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.encounter_music = "legacyEncounter".to_string();
                    trainer
                },
                "exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party.clear();
                    trainer
                },
                "trainer YOUNGSTER_JOEY must declare a party",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party[0].species = "RATTATA ".to_string();
                    trainer
                },
                "token must be exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party[0].species = "fallbackRattata".to_string();
                    trainer
                },
                "token",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party[0].item = Some(" BERRY".to_string());
                    trainer
                },
                "token must be exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party[0].item = Some("legacyBerry".to_string());
                    trainer
                },
                "token",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party[0].moves[0].name = "TACKLE ".to_string();
                    trainer
                },
                "token must be exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.party[0].moves[0].name = "fallbackMove".to_string();
                    trainer
                },
                "token",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.items = vec![Some(" POTION".to_string())];
                    trainer
                },
                "token must be exact ASCII alphanumeric/underscore",
            ),
            (
                {
                    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER");
                    trainer.items = vec![Some("legacyPotion".to_string())];
                    trainer
                },
                "token",
            ),
        ] {
            let error = GameDataSet::default()
                .apply_content_pack_payload(
                    ContentPackCategory::Trainers,
                    serde_json::json!({
                        "YOUNGSTER_JOEY": trainer
                    }),
                )
                .expect_err("malformed trainer records must fail during pack load");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "WRONG_ROUTE": WildEncounterData {
                        map_name: "NEW_ROUTE".to_string(),
                        ..WildEncounterData::default()
                    }
                }),
            )
            .expect_err("wild encounter payload key must match map_name");
        assert!(
            format!("{error:#}").contains(
                "wild encounter key 'WRONG_ROUTE' does not match record map_name 'NEW_ROUTE'"
            ),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::FieldEncounters,
                serde_json::json!({
                    "WRONG_ROUTE": FieldEncounterData::for_crystal(
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
                        None
                    )
                }),
            )
            .expect_err("field encounter payload key must match map_name");
        assert!(
            format!("{error:#}").contains(
                "field encounter key 'WRONG_ROUTE' does not match record map_name 'NEW_ROUTE'"
            ),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "NEW ROUTE": WildEncounterData {
                        map_name: "NEW ROUTE".to_string(),
                        ..WildEncounterData::default()
                    }
                }),
            )
            .expect_err("wild encounter map names must be exact encounter tokens");
        assert!(
            format!("{error:#}")
                .contains("encounter token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let wild = WildEncounterData {
            map_name: "NEW_ROUTE".to_string(),
            grass: Some(crystal_core::world::encounters::WildEncounterTable {
                morning: vec![crystal_core::world::encounters::WildEncounter {
                    level: 5,
                    species: "NEW MON".to_string(),
                }],
                day: Vec::new(),
                night: Vec::new(),
            }),
            ..WildEncounterData::default()
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "NEW_ROUTE": wild
                }),
            )
            .expect_err("wild encounter species ids must be exact encounter tokens");
        assert!(
            format!("{error:#}")
                .contains("encounter token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let wild = WildEncounterData {
            map_name: "NEW_ROUTE".to_string(),
            grass_rates: Some(BTreeMap::from([("dawn".to_string(), 20)])),
            ..WildEncounterData::default()
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "NEW_ROUTE": wild
                }),
            )
            .expect_err("wild grass rate times must be known Crystal encounter keys");
        assert!(
            format!("{error:#}").contains(
                "wild encounter grass rate time for map NEW_ROUTE 'dawn' must be morning, day, or night"
            ),
            "{error:#}"
        );

        let field = FieldEncounterData::for_crystal(
            "NEW_ROUTE",
            Some(FieldEncounterTable {
                common: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "NEW_MON\u{0007}".to_string(),
                    level: 5,
                    sleep_turns_by_time: Default::default(),
                }],
                rare: Vec::new(),
            }),
            None,
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldEncounters,
                serde_json::json!({
                    "NEW_ROUTE": field
                }),
            )
            .expect_err("field encounter species ids must be exact encounter tokens");
        assert!(
            format!("{error:#}")
                .contains("encounter token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut field = FieldEncounterData::for_crystal("NEW_ROUTE", None, None);
        field.tables.insert(
            "hidden_tree".to_string(),
            FieldEncounterTable {
                common: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "NEW_MON".to_string(),
                    level: 5,
                    sleep_turns_by_time: Default::default(),
                }],
                rare: Vec::new(),
            },
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldEncounters,
                serde_json::json!({
                    "NEW_ROUTE": field
                }),
            )
            .expect_err("field encounter table kinds must be known Crystal encounter keys");
        assert!(
            format!("{error:#}").contains(
                "field encounter table kind for map NEW_ROUTE 'hidden_tree' must be headbutt or rock_smash"
            ),
            "{error:#}"
        );

        let wild = WildEncounterData {
            map_name: "NEW_ROUTE".to_string(),
            grass_rates: Some([("day".to_string(), 20)].into_iter().collect()),
            ..WildEncounterData::default()
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "NEW_ROUTE": wild
                }),
            )
            .expect_err("positive wild grass rates require a grass table");
        assert!(
            format!("{error:#}").contains("positive grass rates but no grass table"),
            "{error:#}"
        );

        let wild = WildEncounterData {
            map_name: "NEW_ROUTE".to_string(),
            water_rate: Some(20),
            ..WildEncounterData::default()
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "NEW_ROUTE": wild
                }),
            )
            .expect_err("positive wild water rates require a water table");
        assert!(
            format!("{error:#}").contains("positive water rate but no water table"),
            "{error:#}"
        );

        let wild = WildEncounterData {
            map_name: "NEW_ROUTE".to_string(),
            grass_rates: Some(
                [
                    ("morning".to_string(), 0),
                    ("day".to_string(), 20),
                    ("night".to_string(), 0),
                ]
                .into_iter()
                .collect(),
            ),
            grass: Some(crystal_core::world::encounters::WildEncounterTable::default()),
            ..WildEncounterData::default()
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::WildEncounters,
                serde_json::json!({
                    "NEW_ROUTE": wild
                }),
            )
            .expect_err("positive wild grass rates require matching slots");
        assert!(
            format!("{error:#}").contains("positive day grass rate but no day grass slots"),
            "{error:#}"
        );

        let field = FieldEncounterData::for_crystal(
            "NEW_ROUTE",
            Some(FieldEncounterTable {
                common: vec![FieldEncounterEntry {
                    weight: 90,
                    species: "NEW_MON".to_string(),
                    level: 5,
                    sleep_turns_by_time: Default::default(),
                }],
                rare: vec![FieldEncounterEntry {
                    weight: 100,
                    species: "NEW_MON".to_string(),
                    level: 5,
                    sleep_turns_by_time: Default::default(),
                }],
            }),
            None,
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldEncounters,
                serde_json::json!({
                    "NEW_ROUTE": field
                }),
            )
            .expect_err("field encounter bucket weights must total 100");
        assert!(
            format!("{error:#}").contains("headbutt common bucket weights must total 100"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_primary_runtime_ids() {
        let mut data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon: pokemon_payload(vec![species()]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Pokemon manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate Pokemon species 'NEW_MON'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::json!({
                    "WRONG_MON": species()
                }),
            )
            .expect_err("Pokemon payload key must match species id");
        assert!(
            format!("{error:#}")
                .contains("Pokemon species key 'WRONG_MON' does not match record id 'NEW_MON'"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                moves: move_payload(vec![test_move("TACKLE")]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate move manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate move 'TACKLE'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Moves,
                serde_json::json!({
                    "WRONG_MOVE": test_move("TACKLE")
                }),
            )
            .expect_err("move payload key must match name");
        assert!(
            format!("{error:#}")
                .contains("move key 'WRONG_MOVE' does not match record name 'TACKLE'"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                items: item_payload(vec![test_item("POTION")]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate item manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate item 'POTION'"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            maps: [(
                "Route29".to_string(),
                test_map_module("Route29", "ROUTE_29", None),
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate map manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate map module 'Route29'"),
            "{error:#}"
        );

        let manifest = ModpackManifest {
            payload: ModpackPayload {
                maps: [(
                    "WrongMap".to_string(),
                    test_map_module("Route29", "ROUTE_29", None),
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let mut data = GameDataSet::default();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("map manifest key must match module id");
        assert!(
            format!("{error:#}")
                .contains("map module key 'WrongMap' does not match record id 'Route29'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        data.trainers
            .insert(test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER"))
            .expect("insert base trainer");
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                trainers: TrainerCatalog {
                    trainers: [(
                        "YOUNGSTER_JOEY".to_string(),
                        test_trainer("YOUNGSTER_JOEY", "MUSIC_HIKER_ENCOUNTER"),
                    )]
                    .into_iter()
                    .collect(),
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate trainer manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("trainer id 'YOUNGSTER_JOEY' is duplicated"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                trainers: TrainerCatalog {
                    trainers: [(
                        "YOUNGSTER_JOEY".to_string(),
                        test_trainer("BUG_CATCHER_DON", "MUSIC_HIKER_ENCOUNTER"),
                    )]
                    .into_iter()
                    .collect(),
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("trainer manifest key must match trainer_id");
        assert!(
            format!("{error:#}").contains(
                "trainer key 'YOUNGSTER_JOEY' does not match record trainer_id 'BUG_CATCHER_DON'"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_roaming_pokemon_catalog() {
        let catalog = roaming_catalog_for_tests("RAIKOU", "ENTEI");
        let mut data = GameDataSet {
            roaming_pokemon: catalog.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                roaming_pokemon: catalog,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate roaming Pokemon catalog must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate roaming Pokemon catalog"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_shuckie_gift_definitions() {
        let gift = ShuckieGiftDefinition {
            species: "SHUCKLE".to_string(),
            level: 15,
            held_item: "BERRY".to_string(),
            nickname: "SHUCKIE".to_string(),
            original_trainer_name: "MANIA".to_string(),
            original_trainer_id: 518,
            got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
        };
        let mut data = GameDataSet {
            shuckie_gift: Some(gift.clone()),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                shuckie_gift: Some(gift),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Shuckie gift manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Shuckie gift definition"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_bug_contest_config() {
        let config = BugContestConfig {
            park_balls: 20,
            timer_minutes: 20,
            timer_seconds: 0,
            selected_contestant_count: 5,
            contestant_flags: vec!["EVENT_BUG_CATCHING_CONTESTANT_1A".to_string()],
            encounters: bug_contest_encounters_for_tests(),
        };
        let mut data = GameDataSet {
            bug_contest_config: Some(config.clone()),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                bug_contest_config: Some(config),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Bug-Catching Contest config manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Bug-Catching Contest config"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_battle_tower_rules() {
        let rules = BattleTowerRules {
            banned_species: BTreeMap::from([
                (
                    "MEWTWO".to_string(),
                    BattleTowerBannedSpeciesRule::default(),
                ),
                ("MEW".to_string(), BattleTowerBannedSpeciesRule::default()),
                ("LUGIA".to_string(), BattleTowerBannedSpeciesRule::default()),
                ("HO_OH".to_string(), BattleTowerBannedSpeciesRule::default()),
                (
                    "CELEBI".to_string(),
                    BattleTowerBannedSpeciesRule::default(),
                ),
            ]),
            required_party_count: 3,
            challenge_streak_length: 7,
            reward_candidates: vec!["HP_UP".to_string(), "LUCKY_PUNCH".to_string()],
            excluded_reward_items: vec!["LUCKY_PUNCH".to_string()],
            reward_quantity: 5,
            reward_failure_sentinel: "POTION".to_string(),
            reward_item_values: [("POTION".to_string(), 0x12), ("HP_UP".to_string(), 0x1a), ("LUCKY_PUNCH".to_string(), 0x1e)].into_iter().collect(),
            minimum_level_group: 1,
            maximum_level_group: 10,
            level_group_size: 10,
            party_count_failure_text: "OnlyThreeMonMayBeEnteredText".to_string(),
            duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
            duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
            egg_failure_text: "YouCantTakeAnEggText".to_string(),
            trainers: test_battle_tower_trainers(),
            mon_groups: test_battle_tower_mon_groups(),
        };
        let mut data = GameDataSet {
            battle_tower_rules: Some(rules.clone()),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_tower_rules: Some(rules),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Battle Tower rules manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Battle Tower rules"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_oak_rating_table() {
        let ratings = vec![OakRatingEntry {
            caught_count_limit: 9,
            fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
            text_label: "OakRating01".to_string(),
        }];
        let mut data = GameDataSet {
            oak_ratings: ratings.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                oak_ratings: ratings,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Oak rating manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Oak rating table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_odd_egg_definitions_table() {
        let definitions = vec![OddEggDefinition {
            species: "CLEFFA".to_string(),
            moves: vec![
                "POUND".to_string(),
                "CHARM".to_string(),
                "DIZZY_PUNCH".to_string(),
            ],
            original_trainer_id: 768,
            dvs: [2, 10, 10, 10],
            probability: 100,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        }];
        let mut data = GameDataSet {
            odd_egg_definitions: definitions.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                odd_egg_definitions: definitions,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Odd Egg definitions manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Odd Egg definitions table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_magikarp_length_table() {
        let lengths = magikarp_lengths_for_tests();
        let mut data = GameDataSet {
            magikarp_lengths: lengths.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                magikarp_lengths: lengths,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Magikarp length manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Magikarp length table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_capture_wobble_probability_table() {
        let probabilities = vec![CaptureWobbleProbability {
            catch_rate: 1,
            chance: 2,
        }];
        let mut data = GameDataSet {
            capture_wobble_probabilities: probabilities.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                capture_wobble_probabilities: probabilities,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate capture wobble probability manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate capture wobble probability table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_encounter_slot_table() {
        let tables = EncounterSlotTables::for_crystal(
            vec![EncounterSlotChance {
                threshold: 100,
                slot: 0,
            }],
            vec![EncounterSlotChance {
                threshold: 100,
                slot: 0,
            }],
        );
        let mut data = GameDataSet {
            encounter_slot_tables: tables.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                encounter_slot_tables: tables,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate encounter slot manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate encounter slot table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_encounter_music_modifier_table() {
        let modifiers = EncounterMusicModifiers {
            modifiers: BTreeMap::from([(
                "MUSIC_POKEMON_MARCH".to_string(),
                EncounterMusicModifier {
                    numerator: 2,
                    denominator: 1,
                },
            )]),
        };
        let mut data = GameDataSet {
            encounter_music_modifiers: modifiers.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                encounter_music_modifiers: modifiers,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate encounter music modifier manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate encounter music modifier table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_battle_stat_multiplier_table() {
        let multipliers = BattleStatMultiplierTables {
            stat: vec![BattleStatMultiplier {
                numerator: 1,
                denominator: 1,
            }],
            accuracy: vec![BattleStatMultiplier {
                numerator: 1,
                denominator: 1,
            }],
        };
        let mut data = GameDataSet {
            battle_stat_multipliers: multipliers.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_stat_multipliers: multipliers,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate battle stat multiplier manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle stat multiplier table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_capture_rules_table() {
        let rules: CaptureRules = serde_json::from_value(serde_json::json!({
            "fast_ball_species": ["MAGNEMITE"],
            "heavy_ball_modifiers": {},
            "ball_rules": {},
            "guaranteed_capture_balls": [],
            "status_bonus": {}
        }))
        .expect("capture rules fixture should parse");
        let mut data = GameDataSet {
            capture_rules: rules.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                capture_rules: rules,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate capture rules manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate capture rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_battle_escape_rules_table() {
        let rules = BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        };
        let mut data = GameDataSet {
            battle_escape_rules: rules.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_escape_rules: rules,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate battle escape rules manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle escape rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_move_priority_table() {
        let priorities: MovePriorityTable = serde_json::from_value(serde_json::json!({
            "base_priority": 1,
            "effect_priorities": {
                "PROTECT": 3
            },
            "move_priorities": [
                { "move": "VITAL_THROW", "priority": 0 }
            ]
        }))
        .expect("move priority fixture should parse");
        let mut data = GameDataSet {
            move_priorities: priorities.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                move_priorities: priorities,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate move priority manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate move priority table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_type_category_table() {
        let categories: TypeCategories = serde_json::from_value(serde_json::json!({
            "physical": ["NORMAL"],
            "special": ["FIRE"]
        }))
        .expect("type category fixture should parse");
        let mut data = GameDataSet {
            type_categories: categories.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                type_categories: categories,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate type category manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate type category table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_type_effectiveness_table() {
        let effectiveness = test_type_effectiveness();
        let mut data = GameDataSet {
            type_effectiveness: effectiveness.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                type_effectiveness: effectiveness,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate type effectiveness manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate type effectiveness table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_weather_modifier_table() {
        let modifiers: WeatherModifiers = serde_json::from_value(serde_json::json!({
            "type_modifiers": {
                "WEATHER_RAIN": {
                    "WATER": { "numerator": 3, "denominator": 2 }
                }
            },
            "move_effect_modifiers": {
                "WEATHER_RAIN": {
                    "SOLARBEAM": { "numerator": 1, "denominator": 2 }
                }
            }
        }))
        .expect("weather modifier fixture should parse");
        let mut data = GameDataSet {
            weather_modifiers: modifiers.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                weather_modifiers: modifiers,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate weather modifier manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate weather modifier table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_battle_reward_rules_table() {
        let rules = BattleRewardRules {
            max_level: 100,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
            mom_money_increment: 2_300,
            mom_random_items: vec![crystal_core::systems::battle_rewards::MomPurchaseRule {
                trigger: 0,
                cost: 600,
                kind: crystal_core::systems::battle_rewards::MomPurchaseKind::Item,
                target: "SUPER_POTION".to_string(),
                decoration_flag: None,
            }],
            mom_progression_items: vec![crystal_core::systems::battle_rewards::MomPurchaseRule {
                trigger: 900,
                cost: 600,
                kind: crystal_core::systems::battle_rewards::MomPurchaseKind::Item,
                target: "SUPER_POTION".to_string(),
                decoration_flag: None,
            }],
        };
        let mut data = GameDataSet {
            battle_reward_rules: rules.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_reward_rules: rules,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate battle reward rules manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle reward rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_step_event_rules_table() {
        let rules = StepEventRules {
            poison_step_interval: 4,
            egg_step_trigger: 1,
            hatched_egg_happiness: 120,
            poison_status: "PSN".to_string(),
            egg_nickname: "EGG".to_string(),
            happiness_step_counter_mask: 255,
            happiness_step_counter_target: 0,
        };
        let mut data = GameDataSet {
            step_event_rules: rules.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                step_event_rules: rules,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate step event rules manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate step event rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_fishing_table() {
        let catalog: FishingCatalog = serde_json::from_value(serde_json::json!({
            "groups": {},
            "time_groups": {},
            "swarm_rules": {},
            "rod_items": {
                "OLD_ROD": "OLD_ROD"
            }
        }))
        .expect("fishing fixture should parse");
        let mut data = GameDataSet {
            fishing: catalog.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                fishing: catalog,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate fishing manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate fishing table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_field_moves_table() {
        let mut catalog = FieldMoveCatalog::default();
        catalog.fly = FieldMoveRule {
            move_id: "FLY".to_string(),
            ..FieldMoveRule::default()
        };
        let mut data = GameDataSet {
            field_moves: catalog.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                field_moves: catalog,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate field moves manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate field moves table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_initialize_events_table() {
        let config = InitializeEventsConfig {
            event_flags: vec!["EVENT_INITIALIZED".to_string()],
            engine_flags: vec!["ENGINE_INITIALIZED".to_string()],
            variable_sprites: [("SPRITE_A".to_string(), "SPRITE_B".to_string())]
                .into_iter()
                .collect(),
        };
        let mut data = GameDataSet {
            initialize_events: config.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                initialize_events: config,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate initialize events manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate initialize events table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_story_event_script_constants_table() {
        let constants = StoryEventScriptConstants {
            global: [("EVENT_ONE".to_string(), 1)].into_iter().collect(),
            maps: [(
                "ROUTE_29".to_string(),
                [("ROUTE_EVENT".to_string(), 2)].into_iter().collect(),
            )]
            .into_iter()
            .collect(),
        };
        let mut data = GameDataSet {
            story_event_script_constants: constants.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                story_event_script_constants: constants,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate story event script constants manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate story event script constants table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_flee_mons_table() {
        let tables = FleeMonTables::for_crystal(
            vec!["RAIKOU".to_string()],
            vec!["ENTEI".to_string()],
            vec!["SUICUNE".to_string()],
        );
        let mut data = GameDataSet {
            flee_mons: tables.clone(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                flee_mons: tables,
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate flee mons manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate flee mons table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_happiness_data_table() {
        let data_table = HappinessData {
            changes: BTreeMap::from([(
                18,
                crystal_core::systems::special_routines::HappinessChangeEntry {
                    code: "HAPPINESS_GROOMING".to_string(),
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            )]),
            services: BTreeMap::from([(
                "DaisysGrooming".to_string(),
                vec![
                    crystal_core::systems::special_routines::HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 2,
                        change_code: 18,
                    },
                ],
            )]),
        };
        let mut data = GameDataSet {
            happiness_data: Some(data_table.clone()),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                happiness_data: Some(data_table),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate happiness data manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate happiness data table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_move_names_table() {
        let mut data = GameDataSet {
            move_names: vec!["POUND".to_string()],
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                move_names: vec!["KARATE_CHOP".to_string()],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate move names manifest table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate move names table"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_battle_animation_table() {
        let mut data = GameDataSet {
            battle_animation_table: vec!["BattleAnim_Pound".to_string()],
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_animation_table: vec!["BattleAnim_KarateChop".to_string()],
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate battle animation table manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle animation table"),
            "{error:#}"
        );
    }

    fn complete_battle_anim_bundle_payload() -> Value {
        serde_json::json!({
            "objects": { "BattleAnim_Pound": {} },
            "framesets": { "BattleAnim_PoundFrames": {} },
            "oam_sets": { "BattleAnim_PoundOam": {} },
            "gfx_table": { "BattleAnim_PoundGfx": {} },
            "gfx_sources": { "BattleAnim_PoundGfx": {} }
        })
    }

    fn complete_sprite_anim_bundle_payload() -> Value {
        serde_json::json!({
            "oam_sets": { "SpriteAnimFrame": {} },
            "framesets": { "SpriteAnimFrameSet": {} },
            "objects": { "SpriteAnimObject": {} }
        })
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_animation_bundles() {
        let mut data = GameDataSet {
            battle_anim_bundle: "{\"objects\":[]}".to_string(),
            sprite_anim_bundle: "{\"oam_sets\":[]}".to_string(),
            ..GameDataSet::default()
        };
        let mut manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_anim_bundle: "{\"objects\":[\"BattleAnim_Pound\"]}".to_string(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate battle animation bundle manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate battle animation bundle"),
            "{error:#}"
        );

        manifest.payload.battle_anim_bundle.clear();
        manifest.payload.sprite_anim_bundle = "{\"oam_sets\":[\"SpriteAnimFrame\"]}".to_string();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate sprite animation bundle manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate sprite animation bundle"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_malformed_animation_bundles_without_skipping() {
        let mut data = GameDataSet::default();
        let mut manifest = ModpackManifest {
            payload: ModpackPayload {
                battle_anim_bundle: "   ".to_string(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("whitespace battle animation bundle must fail instead of being skipped");
        assert!(
            format!("{error:#}").contains("battle animation bundle"),
            "{error:#}"
        );

        manifest.payload.battle_anim_bundle.clear();
        manifest.payload.sprite_anim_bundle = "\n".to_string();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("whitespace sprite animation bundle must fail instead of being skipped");
        assert!(
            format!("{error:#}").contains("sprite animation bundle"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_tileset_payloads() {
        let tileset = TilesetDefinition {
            collision: (0..=10)
                .map(|metatile_id| {
                    (
                        format!("{metatile_id:x}"),
                        vec![
                            "FLOOR".to_string(),
                            "FLOOR".to_string(),
                            "FLOOR".to_string(),
                            "FLOOR".to_string(),
                        ],
                    )
                })
                .collect(),
            palette_map: vec![0, 1, 2, 3],
        };
        let mut data = GameDataSet {
            tilesets: [("johto".to_string(), tileset.clone())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                tilesets: [("johto".to_string(), tileset)].into_iter().collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate tileset manifest payload must not append");

        assert!(
            format!("{error:#}").contains("duplicate tileset id 'johto'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_buena_password_category_ids() {
        let category = BuenaPasswordCategoryDefinition {
            category_type: "BUENA_ITEM".to_string(),
            points: 12,
            options: vec!["POTION".to_string()],
        };
        let mut data = GameDataSet {
            buena_password_categories: BuenaPasswordCategories {
                order: vec!["HealingItems".to_string()],
                categories: BTreeMap::from([("HealingItems".to_string(), category.clone())]),
            },
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                buena_password_categories: BuenaPasswordCategories {
                    order: vec!["HealingItems".to_string()],
                    categories: BTreeMap::from([("HealingItems".to_string(), category)]),
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Buena password category manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Buena password category id 'HealingItems'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_buena_prize_item_ids() {
        let mut data = GameDataSet {
            buena_prizes: BTreeMap::from([("RARE_CANDY".to_string(), 3)]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                buena_prizes: BTreeMap::from([("RARE_CANDY".to_string(), 5)]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Buena prize manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Buena prize item id 'RARE_CANDY'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_kurt_apricorn_recipe_ids() {
        let mut data = GameDataSet {
            kurt_apricorn_recipes: BTreeMap::from([(
                "RED_APRICORN".to_string(),
                "LEVEL_BALL".to_string(),
            )]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                kurt_apricorn_recipes: BTreeMap::from([(
                    "RED_APRICORN".to_string(),
                    "FRIEND_BALL".to_string(),
                )]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Kurt apricorn recipe manifest must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate Kurt apricorn recipe for apricorn 'RED_APRICORN'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_dratini_move_set_modes() {
        let mut data = GameDataSet {
            dratini_move_sets: BTreeMap::from([(
                0,
                vec!["WRAP".to_string(), "THUNDER_WAVE".to_string()],
            )]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                dratini_move_sets: BTreeMap::from([(
                    0,
                    vec!["LEER".to_string(), "TWISTER".to_string()],
                )]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Dratini move set manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Dratini move set mode 0"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_exact_string_catalog_entries() {
        let mut data = GameDataSet {
            permanent_phone_numbers: BTreeMap::from([(
                "PHONE_ELM".to_string(),
                PermanentPhoneNumberRule::default(),
            )]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                permanent_phone_numbers: BTreeMap::from([(
                    "PHONE_ELM".to_string(),
                    PermanentPhoneNumberRule::default(),
                )]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate permanent phone manifest must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate permanent phone number 'PHONE_ELM'"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            special_phone_calls: BTreeMap::from([(
                "SPECIALCALL_POKERUS".to_string(),
                SpecialPhoneCallRule::default(),
            )]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                special_phone_calls: BTreeMap::from([(
                    "SPECIALCALL_POKERUS".to_string(),
                    SpecialPhoneCallRule::default(),
                )]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate special phone call manifest must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate special phone call 'SPECIALCALL_POKERUS'"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            npc_trades: npc_trade_rules(["NPC_TRADE_ONIX"]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                npc_trades: npc_trade_rules(["NPC_TRADE_ONIX"]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate NPC trade manifest must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate NPC trade 'NPC_TRADE_ONIX'"),
            "{error:#}"
        );

        let mut data = GameDataSet {
            special_routines: special_routine_rules(["FadeOutMusic"]),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                special_routines: special_routine_rules(["FadeOutMusic"]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate special routine manifest must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate special routine 'FadeOutMusic'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpecialRoutines,
                serde_json::json!({"ModpackOnlyRoutine": {}}),
            )
            .expect_err("unknown special routine payload must not be accepted");
        assert!(
            format!("{error:#}").contains(
                "special routine 'ModpackOnlyRoutine' is not implemented by the Rust runtime"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_evolutions_by_exact_species_id() {
        let mut data = GameDataSet::default();
        data.evolutions.0.insert(
            "NEW_MON".to_string(),
            vec![EvolutionEntry::level("OLD_FORM", 20)],
        );
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                evolutions: EvolutionTable(
                    [(
                        "NEW_MON".to_string(),
                        vec![EvolutionEntry::level("NEW_FORM", 30)],
                    )]
                    .into_iter()
                    .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate evolution manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate evolutions for species 'NEW_MON'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_marts_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Marts,
            serde_json::json!({
                "MartCherrygroveDex": ["POKE_BALL", "POTION"]
            }),
        )
        .expect("apply mart payload");

        assert_eq!(
            data.marts
                .inventory_ids("MartCherrygroveDex")
                .expect("mart"),
            &["POKE_BALL".to_string(), "POTION".to_string()]
        );
        assert!(data.marts.inventory_ids("MART_CHERRYGROVE_DEX").is_err());
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_mart_ids() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Marts,
            serde_json::json!({
                "MartNew": ["POTION"]
            }),
        )
        .expect("apply first mart payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Marts,
                serde_json::json!({
                    "MartNew": ["POKE_BALL"]
                }),
            )
            .expect_err("duplicate mart payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate mart catalog entry for mart 'MartNew'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_catalog_keys_without_trimming() {
        let cases = [
            (
                ContentPackCategory::Marts,
                serde_json::json!({
                    " MartNew": ["POTION"]
                }),
                "mart catalog entry id ' MartNew' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "FruitTreeRoute29 ": "BERRY"
                }),
                "fruit tree catalog entry id 'FruitTreeRoute29 ' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::Marts,
                serde_json::json!({
                    "Mart New": ["POTION"]
                }),
                "mart catalog entry id 'Mart New' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "Fruit Tree Route29": "BERRY"
                }),
                "fruit tree catalog entry id 'Fruit Tree Route29' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm\u{0007}": test_phone_contact("PhoneElm")
                }),
                "phone contact catalog entry id 'PhoneElm\u{0007}' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::CurrencyConstants,
                serde_json::json!({
                    "ROUTE43GATE_TOLL ": 1000
                }),
                "currency constant 'ROUTE43GATE_TOLL ' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::CurrencyConstants,
                serde_json::json!({
                    "ROUTE43GATE TOLL": 1000
                }),
                "currency constant 'ROUTE43GATE TOLL' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::Marts,
                serde_json::json!({
                    "fallbackMart": ["POTION"]
                }),
                "mart catalog entry id 'fallbackMart' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "legacyTree": "BERRY"
                }),
                "fruit tree catalog entry id 'legacyTree' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "fallbackContact": test_phone_contact("PhoneElm")
                }),
                "phone contact catalog entry id 'fallbackContact' must be exact ASCII alphanumeric or underscore",
            ),
        ];

        for (category, payload, expected) in cases {
            let mut data = GameDataSet::default();
            let error = data
                .apply_content_pack_payload(category, payload)
                .expect_err("catalog keys must be exact")
                .to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn content_pack_payloads_reject_malformed_catalog_values_without_trimming() {
        let cases = [
            (
                ContentPackCategory::Marts,
                serde_json::json!({
                    "MartNew": [" POTION"]
                }),
                "mart item id ' POTION' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::Marts,
                serde_json::json!({
                    "MartNew": ["RARE CANDY"]
                }),
                "mart item id 'RARE CANDY' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::Marts,
                serde_json::json!({
                    "MartNew": ["legacyPotion"]
                }),
                "mart item id 'legacyPotion' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "FruitTreeRoute29": "BERRY\u{0007}"
                }),
                "fruit tree item id 'BERRY\u{0007}' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "FruitTreeRoute29": "GOLD BERRY"
                }),
                "fruit tree item id 'GOLD BERRY' must be exact ASCII alphanumeric or underscore",
            ),
            (
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "FruitTreeRoute29": "fallbackBerry"
                }),
                "fruit tree item id 'fallbackBerry' must be exact ASCII alphanumeric or underscore",
            ),
        ];

        for (category, payload, expected) in cases {
            let mut data = GameDataSet::default();
            let error = data
                .apply_content_pack_payload(category, payload)
                .expect_err("catalog values must be exact")
                .to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn content_pack_payloads_merge_fruit_trees_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::FruitTrees,
            serde_json::json!({
                "FruitTreeRoute29": "BERRY"
            }),
        )
        .expect("apply fruit tree payload");

        assert_eq!(
            data.fruit_trees.0.get("FruitTreeRoute29"),
            Some(&"BERRY".to_string())
        );
        assert!(!data.fruit_trees.0.contains_key("FRUITTREE_ROUTE_29"));
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_fruit_tree_ids() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::FruitTrees,
            serde_json::json!({
                "FruitTreeRoute29": "BERRY"
            }),
        )
        .expect("apply first fruit tree payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::FruitTrees,
                serde_json::json!({
                    "FruitTreeRoute29": "PSNCUREBERRY"
                }),
            )
            .expect_err("duplicate fruit tree payload must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate fruit tree catalog entry for tree 'FruitTreeRoute29'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_phone_contacts_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PhoneContacts,
            serde_json::json!({
                "PhoneElm": test_phone_contact("PhoneElm")
            }),
        )
        .expect("apply phone contact payload");

        assert!(data.phone_contacts.0.contains_key("PhoneElm"));
        assert!(!data.phone_contacts.0.contains_key("PHONE_ELM"));
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_phone_contact_ids() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PhoneContacts,
            serde_json::json!({
                "PhoneElm": test_phone_contact("PhoneElm")
            }),
        )
        .expect("apply first phone contact payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": test_phone_contact("PhoneElm")
                }),
            )
            .expect_err("duplicate phone contact payload must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate phone contact catalog entry for contact 'PhoneElm'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": test_phone_contact("PhoneMom")
                }),
            )
            .expect_err("phone contact key must match contactId");

        assert!(
            format!("{error:#}").contains(
                "phone contact catalog entry id 'PhoneElm' must match record contactId 'PhoneMom'"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_phone_contact_values_without_trimming() {
        let mut contact = test_phone_contact("PhoneElm");
        contact.trainer_class = Some("TRAINER NONE".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact trainerClass must be exact")
            .to_string();
        assert!(
            error.contains("phone contact trainerClass 'TRAINER NONE' must be exact ASCII alphanumeric or underscore"),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.trainer_label = Some("PHONECONTACT ELM".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact trainerLabel must be exact")
            .to_string();
        assert!(
            error.contains("phone contact trainerLabel 'PHONECONTACT ELM' must be exact ASCII alphanumeric or underscore"),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.lines.clear();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contacts must declare display lines")
            .to_string();
        assert!(
            error.contains("phone contact PhoneElm must declare nonempty dialogue lines"),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.primary_label = "OtherLabel".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact primaryLabel must match the first line")
            .to_string();
        assert!(
            error.contains(
                "phone contact PhoneElm primaryLabel \"OtherLabel\" does not match first line \"PhoneElm:\""
            ),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.lines = vec!["PhoneElm :".to_string()];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact first line must not be normalized to primaryLabel")
            .to_string();
        assert!(
            error.contains(
                "phone contact primaryLabel 'PhoneElm' does not match first display line 'PhoneElm :'"
            ),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.callee_script = Some("Elm Phone Script".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact calleeScript must be exact")
            .to_string();
        assert!(
            error.contains("phone contact calleeScript 'Elm Phone Script' must be exact ASCII alphanumeric or underscore"),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.map_constant = Some("ELMS LAB".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact mapConstant must be exact")
            .to_string();
        assert!(
            error.contains("phone contact mapConstant 'ELMS LAB' must be exact ASCII alphanumeric or underscore"),
            "{error}"
        );

        let mut contact = test_phone_contact("PhoneElm");
        contact.caller_script = Some("Unused Phone Script".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneContacts,
                serde_json::json!({
                    "PhoneElm": contact
                }),
            )
            .expect_err("phone contact callerScript must be exact")
            .to_string();
        assert!(
            error.contains("phone contact callerScript 'Unused Phone Script' must be exact ASCII alphanumeric or underscore"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_currency_constants_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::CurrencyConstants,
            serde_json::json!({
                "ROUTE43GATE_TOLL": 1000,
                "GOLDENRODGAMECORNER_TM25_COINS": 5500
            }),
        )
        .expect("apply currency constants");

        assert_eq!(data.currency_constants.get("ROUTE43GATE_TOLL"), Some(1000));
        assert_eq!(data.currency_constants.get("route43gate_toll"), None);
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_currency_constants() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::CurrencyConstants,
            serde_json::json!({
                "ROUTE43GATE_TOLL": 1000
            }),
        )
        .expect("apply first currency constant payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::CurrencyConstants,
                serde_json::json!({
                    "ROUTE43GATE_TOLL": 500
                }),
            )
            .expect_err("duplicate currency constant payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate currency constant 'ROUTE43GATE_TOLL'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_pokegear_landmarks_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PokegearLandmarks,
            serde_json::json!({
                "landmarks": [{
                    "id": 1,
                    "constant": "LANDMARK_ROUTE_29",
                    "label": "Route29Label",
                    "name": "Route 29",
                    "x": 1,
                    "y": 2,
                    "region": "JOHTO"
                }],
                "map_to_landmark": {
                    "Route29": "LANDMARK_ROUTE_29"
                }
            }),
        )
        .expect("apply Pokegear landmark payload");

        assert_eq!(
            data.pokegear_landmarks.landmarks[0].constant,
            "LANDMARK_ROUTE_29"
        );
        assert_eq!(
            data.pokegear_landmarks.map_to_landmark.get("Route29"),
            Some(&"LANDMARK_ROUTE_29".to_string())
        );
        assert!(
            !data
                .pokegear_landmarks
                .map_to_landmark
                .contains_key("ROUTE_29")
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_pokegear_landmark_constants() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PokegearLandmarks,
            serde_json::json!({
                "landmarks": [{
                    "id": 1,
                    "constant": "LANDMARK_ROUTE_29",
                    "label": "Route29Label",
                    "name": "Route 29",
                    "x": 1,
                    "y": 2,
                    "region": "JOHTO"
                }],
                "map_to_landmark": {}
            }),
        )
        .expect("apply first Pokegear landmark payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 2,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": "Route29OtherLabel",
                        "name": "Route 29 Other",
                        "x": 3,
                        "y": 4,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            )
            .expect_err("duplicate Pokegear landmark payload must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate Pokegear landmark constant 'LANDMARK_ROUTE_29'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_pokegear_landmark_map_assignments() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PokegearLandmarks,
            serde_json::json!({
                "landmarks": [{
                    "id": 1,
                    "constant": "LANDMARK_ROUTE_29",
                    "label": "Route29Label",
                    "name": "Route 29",
                    "x": 1,
                    "y": 2,
                    "region": "JOHTO"
                }],
                "map_to_landmark": {
                    "Route29": "LANDMARK_ROUTE_29"
                }
            }),
        )
        .expect("apply first Pokegear landmark map assignment");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 2,
                        "constant": "LANDMARK_ROUTE_30",
                        "label": "Route30Label",
                        "name": "Route 30",
                        "x": 3,
                        "y": 4,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {
                        "Route29": "LANDMARK_ROUTE_30"
                    }
                }),
            )
            .expect_err("duplicate Pokegear landmark map assignment must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate Pokegear landmark map assignment for map 'Route29'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_pokegear_landmark_keys_without_trimming() {
        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": " LANDMARK_ROUTE_29",
                        "label": "Route29Label",
                        "name": "Route 29",
                        "x": 1,
                        "y": 2,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            )
            .expect_err("Pokegear landmark constants must be exact")
            .to_string();
        assert!(
            error.contains("pokegear landmark constant must be an exact LANDMARK_ token"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "ROUTE_29",
                        "label": "Route29Label",
                        "name": "Route 29",
                        "x": 1,
                        "y": 2,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            )
            .expect_err("Pokegear landmark constants must use LANDMARK ids")
            .to_string();
        assert!(
            error.contains("pokegear landmark constant must be an exact LANDMARK_ token"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [],
                    "map_to_landmark": {
                        "Route29 ": "LANDMARK_ROUTE_29"
                    }
                }),
            )
            .expect_err("Pokegear map assignment keys must be exact")
            .to_string();
        assert!(
            error.contains("pokegear landmark map must be exact ASCII alphanumeric/underscore"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [],
                    "map_to_landmark": {
                        "Route29": " LANDMARK_ROUTE_29"
                    }
                }),
            )
            .expect_err("Pokegear map assignment targets must be exact")
            .to_string();
        assert!(
            error.contains("pokegear landmark map reference must be an exact LANDMARK_ token"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [],
                    "map_to_landmark": {
                        "Route29": "ROUTE_29"
                    }
                }),
            )
            .expect_err("Pokegear map assignment targets must use LANDMARK ids")
            .to_string();
        assert!(
            error.contains("pokegear landmark map reference must be an exact LANDMARK_ token"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_pokegear_landmark_values_without_trimming() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": " Route29Label",
                        "name": "Route 29",
                        "x": 1,
                        "y": 2,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            )
            .expect_err("Pokegear landmark labels must be exact")
            .to_string();
        assert!(
            error.contains("display token must be exact ASCII alphanumeric/underscore"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": "Route29Label",
                        "name": "Route 29 ",
                        "x": 1,
                        "y": 2,
                        "region": "JOHTO"
                    }],
                    "map_to_landmark": {}
                }),
            )
            .expect_err("Pokegear landmark names must be exact")
            .to_string();
        assert!(
            error.contains("display text must be exact non-empty text"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearLandmarks,
                serde_json::json!({
                    "landmarks": [{
                        "id": 1,
                        "constant": "LANDMARK_ROUTE_29",
                        "label": "Route29Label",
                        "name": "Route 29",
                        "x": 1,
                        "y": 2,
                        "region": "JOHTO\u{0007}"
                    }],
                    "map_to_landmark": {}
                }),
            )
            .expect_err("Pokegear landmark regions must be exact")
            .to_string();
        assert!(
            error.contains("display token must be exact ASCII alphanumeric/underscore"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_runtime_spawn_points() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::RuntimeSpawnPoints,
            serde_json::json!({
                "2": test_runtime_spawn_point(2, "Route29")
            }),
        )
        .expect("apply first runtime spawn point payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "2": test_runtime_spawn_point(2, "Route30")
                }),
            )
            .expect_err("duplicate runtime spawn point payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate runtime spawn point '2'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_runtime_spawn_points_without_trimming() {
        let mut spawn = test_runtime_spawn_point(3, "Route29");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "2": spawn
                }),
            )
            .expect_err("runtime spawn point key must match identifier");

        assert!(
            format!("{error:#}")
                .contains("runtime spawn point key '2' does not match identifier 3"),
            "{error:#}"
        );

        spawn = test_runtime_spawn_point(2, " Route29");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "2": spawn
                }),
            )
            .expect_err("runtime spawn point map names must not be trimmed");

        assert!(
            format!("{error:#}").contains(
                "runtime spawn point map name ' Route29' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        spawn = test_runtime_spawn_point(2, "Route 29");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "2": spawn
                }),
            )
            .expect_err("runtime spawn point map names must be exact tokens");

        assert!(
            format!("{error:#}").contains("runtime spawn point map name 'Route 29'"),
            "{error:#}"
        );

        let mut spawn = test_runtime_spawn_point(2, "Route29");
        spawn.tile_x = 10;
        spawn.tile_y = 8;
        spawn.metatile_x = 4;
        spawn.metatile_y = 4;
        spawn.subtile_x = 0;
        spawn.subtile_y = 0;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "2": spawn
                }),
            )
            .expect_err("runtime spawn point tile fields must agree");
        assert!(
            format!("{error:#}").contains(
                "runtime spawn point '2' tile (10, 8) does not match metatile/subtile-derived tile (8, 8)"
            ),
            "{error:#}"
        );

        let mut spawn = test_runtime_spawn_point(2, "Route29");
        spawn.tile_x = 0;
        spawn.tile_y = 0;
        spawn.metatile_x = 0;
        spawn.metatile_y = 0;
        spawn.subtile_x = METATILE_WIDTH;
        spawn.subtile_y = 0;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "2": spawn
                }),
            )
            .expect_err("runtime spawn point subtile must be in range");
        assert!(
            format!("{error:#}")
                .contains("runtime spawn point '2' subtile (2, 0) must be in range 0..2"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeSpawnPoints,
                serde_json::json!({
                    "spawn_points": {
                        "2": test_runtime_spawn_point(2, "Route29")
                    },
                    "fallback_spawn": "NewBarkTown"
                }),
            )
            .expect_err("runtime spawn point payload must be the compiler-emitted object map");
        assert!(
            format!("{error:#}").contains("parse runtime spawn points payload"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_runtime_map_metadata() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::RuntimeMapMetadata,
            serde_json::json!({
                "ROUTE_29": test_runtime_map_metadata("ROUTE_29", "Route29")
            }),
        )
        .expect("apply first runtime map metadata payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeMapMetadata,
                serde_json::json!({
                    "ROUTE_29": test_runtime_map_metadata("ROUTE_29", "Route29Other")
                }),
            )
            .expect_err("duplicate runtime map metadata payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate runtime map metadata 'ROUTE_29'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_runtime_map_metadata_without_trimming() {
        let mut metadata = test_runtime_map_metadata("ROUTE_30", "Route29");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeMapMetadata,
                serde_json::json!({
                    "ROUTE_29": metadata
                }),
            )
            .expect_err("runtime map metadata key must match record constant");

        assert!(
            format!("{error:#}").contains(
                "runtime map metadata key 'ROUTE_29' does not match record constant 'ROUTE_30'"
            ),
            "{error:#}"
        );

        metadata = test_runtime_map_metadata("ROUTE_29", "Route29");
        metadata.environment = " route".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeMapMetadata,
                serde_json::json!({
                    "ROUTE_29": metadata
                }),
            )
            .expect_err("runtime map metadata environment must not be trimmed");

        assert!(
            format!("{error:#}").contains(
                "runtime map metadata token must be exact ASCII alphanumeric/underscore, found \" route\""
            ),
            "{error:#}"
        );

        metadata = test_runtime_map_metadata("ROUTE_29", "Route 29");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeMapMetadata,
                serde_json::json!({
                    "ROUTE_29": metadata
                }),
            )
            .expect_err("runtime map metadata names must be exact tokens");

        assert!(
            format!("{error:#}").contains(
                "runtime map metadata token must be exact ASCII alphanumeric/underscore, found \"Route 29\""
            ),
            "{error:#}"
        );

        metadata = test_runtime_map_metadata("ROUTE 29", "Route29");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeMapMetadata,
                serde_json::json!({
                    "ROUTE 29": metadata
                }),
            )
            .expect_err("runtime map metadata keys must be exact tokens");

        assert!(
            format!("{error:#}").contains(
                "runtime map metadata token must be exact ASCII alphanumeric/underscore, found \"ROUTE 29\""
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RuntimeMapMetadata,
                serde_json::json!({
                    "metadata": {
                        "ROUTE_29": test_runtime_map_metadata("ROUTE_29", "Route29")
                    },
                    "fallback_map": "ROUTE_30"
                }),
            )
            .expect_err("runtime map metadata payload must be the compiler-emitted object map");
        assert!(
            format!("{error:#}").contains("parse runtime map metadata payload"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_pc_strings_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PcStrings,
            serde_json::json!({
                "PCStringChoose": "Choose a Pokemon."
            }),
        )
        .expect("apply PC string payload");

        assert_eq!(
            data.pc_strings.get("PCStringChoose"),
            Some(&"Choose a Pokemon.".to_string())
        );
        assert!(!data.pc_strings.contains_key("PCSTRINGCHOOSE"));
    }

    #[test]
    fn content_pack_payloads_merge_exact_trainer_class_display_names() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::TrainerClassNames,
            serde_json::json!({
                "COOLTRAINERM": "COOLTRAINER",
                "POKEMON_PROF": "POKéMON PROF."
            }),
        )
        .expect("apply source-exact trainer class display names");

        assert_eq!(
            data.trainer_class_names.get("COOLTRAINERM"),
            Some(&"COOLTRAINER".to_string())
        );
        assert_eq!(
            data.trainer_class_names.get("POKEMON_PROF"),
            Some(&"POKéMON PROF.".to_string())
        );

        let duplicate = data
            .apply_content_pack_payload(
                ContentPackCategory::TrainerClassNames,
                serde_json::json!({"COOLTRAINERM": "CHANGED"}),
            )
            .expect_err("trainer class display names must not overwrite");
        assert!(
            format!("{duplicate:#}")
                .contains("duplicate trainer class display name 'COOLTRAINERM'"),
            "{duplicate:#}"
        );

        let malformed = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::TrainerClassNames,
                serde_json::json!({"COOL TRAINER": " COOLTRAINER"}),
            )
            .expect_err("trainer class table ids and display names must stay exact");
        assert!(
            format!("{malformed:#}").contains(
                "trainer class name id 'COOL TRAINER' must be exact ASCII alphanumeric or underscore"
            ),
            "{malformed:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_pc_strings() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PcStrings,
            serde_json::json!({
                "PCStringChoose": "Choose a Pokemon."
            }),
        )
        .expect("apply first PC string payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PcStrings,
                serde_json::json!({
                    "PCStringChoose": "Choose another Pokemon."
                }),
            )
            .expect_err("duplicate PC string payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate PC string 'PCStringChoose'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_keyed_section_ids_without_trimming() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PcStrings,
                serde_json::json!({
                    " PCStringChoose": "Choose a Pokemon."
                }),
            )
            .expect_err("content pack keyed payloads must not trim keys");

        assert!(
            format!("{error:#}").contains(
                "PC string key ' PCStringChoose' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PcStrings,
                serde_json::json!({
                    "PCString Choose": "Choose a Pokemon."
                }),
            )
            .expect_err("PC string keys must be exact tokens");

        assert!(
            format!("{error:#}").contains("PC string key 'PCString Choose'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PcStrings,
                serde_json::json!({
                    "fallback_string": "Choose a Pokemon."
                }),
            )
            .expect_err("PC string fallback keys must be rejected as reserved payload ids");

        assert!(
            format!("{error:#}")
                .contains("PC string key 'fallback_string' uses reserved modpack payload prefix"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_empty_or_control_pc_string_values() {
        for value in ["", "Choose a Pokemon.\n"] {
            let error = GameDataSet::default()
                .apply_content_pack_payload(
                    ContentPackCategory::PcStrings,
                    serde_json::json!({
                        "PCStringChoose": value
                    }),
                )
                .expect_err("PC string values must be exact non-empty text");

            assert!(
                format!("{error:#}").contains("PC string value"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn content_pack_payloads_merge_menu_icons_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::MenuIcons,
            serde_json::json!({
                "CHIKORITA": "ICON_ODDISH"
            }),
        )
        .expect("apply menu icon payload");

        assert_eq!(
            data.menu_icons.get("CHIKORITA"),
            Some(&"ICON_ODDISH".to_string())
        );
        assert!(!data.menu_icons.contains_key("chikorita"));
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_menu_icons() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::MenuIcons,
            serde_json::json!({
                "CHIKORITA": "ICON_ODDISH"
            }),
        )
        .expect("apply first menu icon payload");

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MenuIcons,
                serde_json::json!({
                    "CHIKORITA": "ICON_CHIKORITA"
                }),
            )
            .expect_err("duplicate menu icon payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate menu icon entry for species 'CHIKORITA'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_menu_icon_values_without_trimming() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MenuIcons,
                serde_json::json!({
                    "CHIKORITA": " ICON_ODDISH"
                }),
            )
            .expect_err("menu icon ids must not be trimmed");

        assert!(
            format!("{error:#}").contains(
                "menu icon id ' ICON_ODDISH' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MenuIcons,
                serde_json::json!({
                    "CHIKORITA": "ICON ODDISH"
                }),
            )
            .expect_err("menu icon ids must be exact tokens");

        assert!(
            format!("{error:#}").contains("menu icon id 'ICON ODDISH'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MenuIcons,
                serde_json::json!({
                    "NEW MON": "ICON_ODDISH"
                }),
            )
            .expect_err("menu icon species ids must be exact tokens");

        assert!(
            format!("{error:#}").contains("menu icon species id 'NEW MON'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MenuIcons,
                serde_json::json!({
                    "legacySpecies": "ICON_ODDISH"
                }),
            )
            .expect_err("menu icon legacy keys must be rejected as reserved payload ids");

        assert!(
            format!("{error:#}").contains(
                "menu icon species id 'legacySpecies' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_string_id_merges_reject_malformed_values_without_trimming() {
        let mut values = vec!["EVENT_GOT_A_POKEMON_FROM_ELM".to_string()];
        let error = merge_exact_string_vec(
            &mut values,
            vec![" EVENT_BEAT_ELITE_FOUR".to_string()],
            "playability goal event",
        )
        .expect_err("string list values must not be trimmed");

        assert!(
            format!("{error:#}").contains(
                "playability goal event ' EVENT_BEAT_ELITE_FOUR' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let mut set = BTreeSet::from(["ITEM_POKE_BALL".to_string()]);
        let error = merge_exact_string_set(
            &mut set,
            vec!["ITEM_POTION\u{0007}".to_string()],
            "playability item",
        )
        .expect_err("string set values must not contain control characters");

        assert!(
            format!("{error:#}").contains(
                "playability item 'ITEM_POTION\u{0007}' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_exact_object_maps() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::PokemonFrontpicAnim,
            serde_json::json!({
                "CHIKORITA": { "commands": [{ "kind": "endanim" }] }
            }),
        )
        .expect("apply first frontpic animation payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokemonFrontpicAnim,
                serde_json::json!({
                    "CHIKORITA": { "commands": [{ "kind": "endanim" }] }
                }),
            )
            .expect_err("duplicate frontpic animation payload must not overwrite");
        assert!(
            format!("{error:#}")
                .contains("duplicate frontpic animation program for species 'CHIKORITA'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonFrontpicAnim,
                serde_json::json!({
                    " CHIKORITA": { "commands": [{ "kind": "endanim" }] }
                }),
            )
            .expect_err("frontpic animation species ids must be exact");
        assert!(
            format!("{error:#}").contains(
                "frontpic animation program species id ' CHIKORITA' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonFrontpicAnim,
                serde_json::json!({
                    "CHIKORITA": { "commands": [{ "kind": "frame", "frame": 0 }] }
                }),
            )
            .expect_err("frontpic frame commands must declare all operands");
        assert!(
            format!("{error:#}").contains(
                "frontpic animation program for species 'CHIKORITA' command 0 'frame' is invalid: MissingFrame"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonFrontpicAnim,
                serde_json::json!({
                    "CHIKORITA": {
                        "commands": [
                            { "kind": "dorepeat", "target": 2 },
                            { "kind": "endanim" }
                        ]
                    }
                }),
            )
            .expect_err("frontpic repeat targets must resolve inside the same program");
        assert!(
            format!("{error:#}").contains(
                "frontpic animation program for species 'CHIKORITA' command 0 'dorepeat' targets missing command 2"
            ),
            "{error:#}"
        );

        data.apply_content_pack_payload(
            ContentPackCategory::AsmText,
            serde_json::json!({
                "GreetingText": "Hello."
            }),
        )
        .expect("apply first ASM text payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::AsmText,
                serde_json::json!({
                    "GreetingText": "Hi."
                }),
            )
            .expect_err("duplicate ASM text payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate ASM text label 'GreetingText'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::AsmText,
                serde_json::json!({
                    "Greeting Text": "Hello."
                }),
            )
            .expect_err("ASM text labels must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "ASM text label 'Greeting Text' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::AsmText,
                serde_json::json!({
                    "GreetingText": ""
                }),
            )
            .expect_err("ASM text values must not be empty");
        assert!(
            format!("{error:#}").contains(
                "ASM text value for label 'GreetingText' '' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::AsmText,
                serde_json::json!({
                    "GreetingText": " Hello."
                }),
            )
            .expect_err("ASM text values must not be trimmed");
        assert!(
            format!("{error:#}").contains(
                "ASM text value for label 'GreetingText' ' Hello.' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::AsmText,
                serde_json::json!({
                    "legacyText": "Hello."
                }),
            )
            .expect_err("ASM text legacy keys must be rejected as reserved payload ids");
        assert!(
            format!("{error:#}")
                .contains("ASM text label 'legacyText' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        data.apply_content_pack_payload(
            ContentPackCategory::BattleAnimations,
            serde_json::json!({
                "BattleAnim_Pound": ["anim_wait 1"]
            }),
        )
        .expect("apply first battle animation payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimations,
                serde_json::json!({
                    "BattleAnim_Pound": ["anim_wait 2"]
                }),
            )
            .expect_err("duplicate battle animation payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate battle animation 'BattleAnim_Pound'"),
            "{error:#}"
        );

        data.apply_content_pack_payload(
            ContentPackCategory::SpritePaletteDefaults,
            serde_json::json!({
                "SPRITE_CHRIS": 0
            }),
        )
        .expect("apply first sprite palette default payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::SpritePaletteDefaults,
                serde_json::json!({
                    "SPRITE_CHRIS": 1
                }),
            )
            .expect_err("duplicate sprite palette default payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate sprite palette default 'SPRITE_CHRIS'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpritePaletteDefaults,
                serde_json::json!({
                    " SPRITE_CHRIS": 0
                }),
            )
            .expect_err("sprite palette default keys must be exact");
        assert!(
            format!("{error:#}").contains(
                "sprite palette default sprite id ' SPRITE_CHRIS' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpritePaletteDefaults,
                serde_json::json!({
                    "SPRITE_CHRIS": -1
                }),
            )
            .expect_err("sprite palette defaults must not be negative");
        assert!(
            format!("{error:#}").contains(
                "sprite palette default for sprite 'SPRITE_CHRIS' must be nonnegative, found -1"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpritePaletteDefaults,
                serde_json::json!({
                    "fallback_palette": 0
                }),
            )
            .expect_err("sprite palette fallback keys must be rejected as reserved payload ids");
        assert!(
            format!("{error:#}").contains(
                "sprite palette default sprite id 'fallback_palette' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );

        data.apply_content_pack_payload(
            ContentPackCategory::PokegearTownMapPaletteMap,
            serde_json::json!({
                "town_map": ["SPRITE_CHRIS"]
            }),
        )
        .expect("apply first Pokegear town map palette payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokegearTownMapPaletteMap,
                serde_json::json!({
                    "town_map": ["SPRITE_KRIS"]
                }),
            )
            .expect_err("duplicate Pokegear town map palette payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate Pokegear town map palette entry 'town_map'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearTownMapPaletteMap,
                serde_json::json!({
                    "town_map": [" SPRITE_CHRIS"]
                }),
            )
            .expect_err("Pokegear town map palette values must not be trimmed");
        assert!(
            format!("{error:#}").contains(
                "Pokegear town map palette value ' SPRITE_CHRIS' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearTownMapPaletteMap,
                serde_json::json!({
                    "town map": ["SPRITE_CHRIS"]
                }),
            )
            .expect_err("Pokegear town map palette keys must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "Pokegear town map palette entry 'town map' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearTownMapPaletteMap,
                serde_json::json!({
                    "town_map": ["SPRITE CHRIS"]
                }),
            )
            .expect_err("Pokegear town map palette values must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "Pokegear town map palette value 'SPRITE CHRIS' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearTownMapPaletteMap,
                serde_json::json!({
                    "town_map": []
                }),
            )
            .expect_err("Pokegear town map palette entries must not be empty");
        assert!(
            format!("{error:#}").contains(
                "Pokegear town map palette entry 'town_map' must declare at least one Pokegear town map palette value"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokegearTownMapPaletteMap,
                serde_json::json!({
                    "palettes": {
                        "town_map": ["SPRITE_CHRIS"]
                    },
                    "fallback_palette": "SPRITE_DEFAULT"
                }),
            )
            .expect_err("Pokegear town map palette payload must be the compiler-emitted token map");
        assert!(
            format!("{error:#}").contains("parse Pokegear town map palette payload"),
            "{error:#}"
        );

        data.apply_content_pack_payload(
            ContentPackCategory::PokemonCries,
            serde_json::json!({
                "CHIKORITA": { "cry": "CRY_CHIKORITA", "pitch": 0, "length": 0 }
            }),
        )
        .expect("apply first Pokemon cry payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    "CHIKORITA": { "cry": "CRY_CHIKORITA", "pitch": 1, "length": 0 }
                }),
            )
            .expect_err("duplicate Pokemon cry payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate Pokemon cry metadata for species 'CHIKORITA'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    " CHIKORITA": { "cry": "CRY_CHIKORITA", "pitch": 0, "length": 0 }
                }),
            )
            .expect_err("Pokemon cry species keys must be exact");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata species id ' CHIKORITA' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    "CHIKORITA": { "cry": "CRY CHIKORITA", "pitch": 0, "length": 0 }
                }),
            )
            .expect_err("Pokemon cry audio ids must be exact");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata audio id 'CRY CHIKORITA' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    "CHIKORITA": { "cry": "fallbackCry", "pitch": 0, "length": 0 }
                }),
            )
            .expect_err("Pokemon cry audio ids must reject reserved payload ids");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata audio id 'fallbackCry' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    "CHIKORITA": { "cry": "CRY_CHIKORITA", "pitch": 32768, "length": 0 }
                }),
            )
            .expect_err("Pokemon cry pitch must fit a Crystal word");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata for species 'CHIKORITA' pitch 32768 must fit an exact Crystal word"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    "CHIKORITA": { "cry": "CRY_CHIKORITA", "pitch": 0, "length": -32769 }
                }),
            )
            .expect_err("Pokemon cry length must fit a Crystal word");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata for species 'CHIKORITA' length -32769 must fit an exact Crystal word"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PokemonCries,
                serde_json::json!({
                    "fallback_cry": { "cry": "CRY_CHIKORITA", "pitch": 0, "length": 0 }
                }),
            )
            .expect_err("Pokemon cry fallback keys must be rejected as reserved payload ids");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata species id 'fallback_cry' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );
    }
