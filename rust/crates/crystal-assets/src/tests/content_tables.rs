    #[test]
    fn content_pack_payloads_reject_duplicate_initialize_events_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "eventFlags": ["EVENT_INITIALIZED"],
            "engineFlags": ["ENGINE_INITIALIZED"],
            "variableSprites": {
                "SPRITE_A": "SPRITE_B"
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::InitializeEvents, payload.clone())
            .expect("initial initialize events table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::InitializeEvents, payload)
            .expect_err("duplicate initialize events table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate initialize events table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_initialize_event_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::InitializeEvents,
                serde_json::json!({
                    "eventFlags": ["EVENT INITIALIZED"],
                    "engineFlags": ["ENGINE_INITIALIZED"],
                    "variableSprites": {
                        "SPRITE_A": "SPRITE_B"
                    }
                }),
            )
            .expect_err("malformed initialize event flags must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("initialize_events.eventFlags must be a nonempty exact runtime token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::InitializeEvents,
                serde_json::json!({
                    "eventFlags": ["EVENT_INITIALIZED"],
                    "engineFlags": ["ENGINE_INITIALIZED"],
                    "variableSprites": {
                        "SPRITE A": "SPRITE_B"
                    }
                }),
            )
            .expect_err("malformed initialize variable sprites must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "initialize_events.variableSprites key must be a nonempty exact runtime token"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::InitializeEvents,
                serde_json::json!({
                    "eventFlags": ["EVENT_INITIALIZED"],
                    "engineFlags": ["ENGINE_INITIALIZED"],
                    "variableSprites": {
                        "legacySprite": "SPRITE_B"
                    }
                }),
            )
            .expect_err("reserved initialize variable sprite ids must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "initialize_events.variableSprites key must be a nonempty exact runtime token"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_story_event_script_constants_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "global": {
                "EVENT_ONE": 1
            },
            "maps": {
                "ROUTE_29": {
                    "ROUTE_EVENT": 2
                }
            }
        });
        data.apply_content_pack_payload(
            ContentPackCategory::StoryEventScriptConstants,
            payload.clone(),
        )
        .expect("initial story event script constants table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::StoryEventScriptConstants, payload)
            .expect_err("duplicate story event script constants table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate story event script constants table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_story_event_script_constants() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::StoryEventScriptConstants,
                serde_json::json!({
                    "global": {
                        "EVENT ONE": 1
                    },
                    "maps": {}
                }),
            )
            .expect_err("malformed global story constants must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "story_event_script_constants.global key must be a nonempty exact runtime token"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEventScriptConstants,
                serde_json::json!({
                    "global": {},
                    "maps": {
                        "ROUTE 29": {
                            "ROUTE_EVENT": 2
                        }
                    }
                }),
            )
            .expect_err("malformed story constant map ids must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "story_event_script_constants.maps key must be a nonempty exact runtime token"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEventScriptConstants,
                serde_json::json!({
                    "global": {
                        "fallbackGlobal": 1
                    },
                    "maps": {}
                }),
            )
            .expect_err("reserved story event global constants must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "story_event_script_constants.global key must be a nonempty exact runtime token"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEventScriptConstants,
                serde_json::json!({
                    "global": {},
                    "maps": {
                        "legacyRoute": {
                            "ROUTE_EVENT": 2
                        }
                    }
                }),
            )
            .expect_err("reserved story event map ids must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "story_event_script_constants.maps key must be a nonempty exact runtime token"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn verifier_rejects_weather_move_effect_modifiers_for_missing_move_effects() {
        let mut data = GameDataSet {
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        data.weather_modifiers = serde_json::from_value(serde_json::json!({
            "type_modifiers": {
                "WEATHER_RAIN": {
                    "WATER": { "numerator": 1, "denominator": 1 }
                }
            },
            "move_effect_modifiers": {
                "WEATHER_RAIN": {
                    "SOLARBEAM": { "numerator": 1, "denominator": 1 }
                }
            }
        }))
        .expect("weather modifier fixture should parse");

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_weather_modifier_move_effect"
                && diagnostic.subject == "weather_modifiers:move_effect_modifiers"
        }));
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_marts_by_exact_id() {
        let mut data = GameDataSet {
            marts: MartCatalog(
                [("MartNew".to_string(), vec!["POTION".to_string()])]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                marts: MartCatalog(
                    [("MartNew".to_string(), vec!["POKE_BALL".to_string()])]
                        .into_iter()
                        .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate mart manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate mart catalog entry for mart 'MartNew'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_fruit_trees_by_exact_id() {
        let mut data = GameDataSet {
            fruit_trees: FruitTreeCatalog(
                [("FruitTreeRoute29".to_string(), "BERRY".to_string())]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                fruit_trees: FruitTreeCatalog(
                    [("FruitTreeRoute29".to_string(), "PSNCUREBERRY".to_string())]
                        .into_iter()
                        .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate fruit tree manifest must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate fruit tree catalog entry for tree 'FruitTreeRoute29'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_phone_contacts_by_exact_id() {
        let mut data = GameDataSet {
            phone_contacts: PhoneContactCatalog(
                [("PhoneElm".to_string(), test_phone_contact("PhoneElm"))]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                phone_contacts: PhoneContactCatalog(
                    [("PhoneElm".to_string(), test_phone_contact("PhoneElm"))]
                        .into_iter()
                        .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate phone contact manifest must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate phone contact catalog entry for contact 'PhoneElm'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                phone_contacts: PhoneContactCatalog(
                    [("PhoneElm".to_string(), test_phone_contact("PhoneMom"))]
                        .into_iter()
                        .collect(),
                ),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("phone contact manifest key must match contactId");

        assert!(
            format!("{error:#}").contains(
                "phone contact key 'PhoneElm' does not match record contactId 'PhoneMom'"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_pokegear_landmarks_by_exact_constant() {
        let mut data = GameDataSet {
            pokegear_landmarks: PokegearLandmarksPayload {
                landmarks: vec![PokegearLandmark {
                    id: 1,
                    constant: "LANDMARK_ROUTE_29".to_string(),
                    label: "Route29Label".to_string(),
                    name: "Route 29".to_string(),
                    x: 1,
                    y: 2,
                    region: "JOHTO".to_string(),
                }],
                map_to_landmark: BTreeMap::new(),
            },
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokegear_landmarks: PokegearLandmarksPayload {
                    landmarks: vec![PokegearLandmark {
                        id: 2,
                        constant: "LANDMARK_ROUTE_29".to_string(),
                        label: "Route29OtherLabel".to_string(),
                        name: "Route 29 Other".to_string(),
                        x: 3,
                        y: 4,
                        region: "JOHTO".to_string(),
                    }],
                    map_to_landmark: BTreeMap::new(),
                },
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Pokegear landmark manifest must not overwrite");

        assert!(
            format!("{error:#}")
                .contains("duplicate Pokegear landmark constant 'LANDMARK_ROUTE_29'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_runtime_spawn_points_by_exact_key() {
        let mut data = GameDataSet {
            runtime_spawn_points: [("2".to_string(), test_runtime_spawn_point(2, "Route29"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                runtime_spawn_points: [("2".to_string(), test_runtime_spawn_point(2, "Route30"))]
                    .into_iter()
                    .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate runtime spawn point manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate runtime spawn point '2'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_runtime_map_metadata_by_exact_key() {
        let mut data = GameDataSet {
            runtime_map_metadata: [(
                "ROUTE_29".to_string(),
                test_runtime_map_metadata("ROUTE_29", "Route29"),
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                runtime_map_metadata: [(
                    "ROUTE_29".to_string(),
                    test_runtime_map_metadata("ROUTE_29", "Route29Other"),
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate runtime map metadata manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate runtime map metadata 'ROUTE_29'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_pc_strings_by_exact_key() {
        let mut data = GameDataSet {
            pc_strings: [(
                "PCStringChoose".to_string(),
                "Choose a Pokemon.".to_string(),
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pc_strings: [(
                    "PCStringChoose".to_string(),
                    "Choose another Pokemon.".to_string(),
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate PC string manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate PC string 'PCStringChoose'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_menu_icons_by_exact_species() {
        let mut data = GameDataSet {
            menu_icons: [("CHIKORITA".to_string(), "ICON_ODDISH".to_string())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                menu_icons: [("CHIKORITA".to_string(), "ICON_CHIKORITA".to_string())]
                    .into_iter()
                    .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate menu icon manifest must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate menu icon entry for species 'CHIKORITA'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_malformed_keyed_section_ids_without_trimming() {
        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                asm_text: [(" GreetingText".to_string(), "Hello.".to_string())]
                    .into_iter()
                    .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("modpack keyed payloads must not trim keys");

        assert!(
            format!("{error:#}").contains(
                "ASM text label ' GreetingText' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                asm_text: [("GreetingText".to_string(), String::new())]
                    .into_iter()
                    .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest ASM text values must not be empty");
        assert!(
            format!("{error:#}").contains(
                "ASM text value for label 'GreetingText' '' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                sprite_palette_defaults: [(" SPRITE_CHRIS".to_string(), 0)].into_iter().collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest sprite palette keys must not be trimmed");
        assert!(
            format!("{error:#}").contains(
                "sprite palette default sprite id ' SPRITE_CHRIS' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                sprite_palette_defaults: [("SPRITE_CHRIS".to_string(), -1)].into_iter().collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest sprite palette defaults must not be negative");
        assert!(
            format!("{error:#}").contains(
                "sprite palette default for sprite 'SPRITE_CHRIS' must be nonnegative, found -1"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_exact_object_maps() {
        let frontpic_program = FrontpicAnimProgram {
            commands: vec![FrontpicAnimCommand {
                kind: "endanim".to_string(),
                ..FrontpicAnimCommand::default()
            }],
        };
        let cry = PokemonCryMetadata {
            cry: "CRY_CHIKORITA".to_string(),
            pitch: 0,
            length: 0,
        };
        let mut data = GameDataSet {
            pokemon_frontpic_anim: [("CHIKORITA".to_string(), frontpic_program.clone())]
                .into_iter()
                .collect(),
            asm_text: [("GreetingText".to_string(), "Hello.".to_string())]
                .into_iter()
                .collect(),
            battle_animations: [(
                "BattleAnim_Pound".to_string(),
                vec!["anim_wait 1".to_string()],
            )]
            .into_iter()
            .collect(),
            sprite_palette_defaults: [("SPRITE_CHRIS".to_string(), 0)].into_iter().collect(),
            pokegear_town_map_palette_map: [(
                "town_map".to_string(),
                vec!["SPRITE_CHRIS".to_string()],
            )]
            .into_iter()
            .collect(),
            pokemon_cries: [("CHIKORITA".to_string(), cry.clone())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let mut manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon_frontpic_anim: [("CHIKORITA".to_string(), frontpic_program.clone())]
                    .into_iter()
                    .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate frontpic animation manifest must not overwrite");
        assert!(
            format!("{error:#}")
                .contains("duplicate frontpic animation program for species 'CHIKORITA'"),
            "{error:#}"
        );

        manifest.payload.pokemon_frontpic_anim.clear();
        manifest.payload.asm_text = [("GreetingText".to_string(), "Hi.".to_string())]
            .into_iter()
            .collect();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate ASM text manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate ASM text label 'GreetingText'"),
            "{error:#}"
        );

        manifest.payload.asm_text.clear();
        manifest.payload.battle_animations = [(
            "BattleAnim_Pound".to_string(),
            vec!["anim_wait 2".to_string()],
        )]
        .into_iter()
        .collect();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate battle animation manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate battle animation 'BattleAnim_Pound'"),
            "{error:#}"
        );

        manifest.payload.battle_animations.clear();
        manifest.payload.sprite_palette_defaults =
            [("SPRITE_CHRIS".to_string(), 1)].into_iter().collect();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate sprite palette default manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate sprite palette default 'SPRITE_CHRIS'"),
            "{error:#}"
        );

        manifest.payload.sprite_palette_defaults.clear();
        manifest.payload.pokegear_town_map_palette_map =
            [("town_map".to_string(), vec!["SPRITE_KRIS".to_string()])]
                .into_iter()
                .collect();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Pokegear town map palette manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate Pokegear town map palette entry 'town_map'"),
            "{error:#}"
        );

        manifest.payload.pokegear_town_map_palette_map.clear();
        manifest.payload.pokemon_cries = [(
            "CHIKORITA".to_string(),
            PokemonCryMetadata { pitch: 1, ..cry },
        )]
        .into_iter()
        .collect();
        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Pokemon cry manifest must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate Pokemon cry metadata for species 'CHIKORITA'"),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_malformed_frontpic_animation_programs() {
        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon_frontpic_anim: [(
                    " CHIKORITA".to_string(),
                    FrontpicAnimProgram {
                        commands: vec![FrontpicAnimCommand {
                            kind: "endanim".to_string(),
                            ..FrontpicAnimCommand::default()
                        }],
                    },
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest frontpic species ids must be exact");
        assert!(
            format!("{error:#}").contains(
                "frontpic animation program species id ' CHIKORITA' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon_frontpic_anim: [(
                    "CHIKORITA".to_string(),
                    FrontpicAnimProgram {
                        commands: vec![FrontpicAnimCommand {
                            kind: "dorepeat".to_string(),
                            target: Some(1),
                            ..FrontpicAnimCommand::default()
                        }],
                    },
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest frontpic repeat targets must resolve inside the program");
        assert!(
            format!("{error:#}").contains(
                "frontpic animation program for species 'CHIKORITA' command 0 'dorepeat' targets missing command 1"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn modpack_overlay_rejects_malformed_pokemon_cry_metadata() {
        let mut data = GameDataSet::default();
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon_cries: [(
                    " CHIKORITA".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY_CHIKORITA".to_string(),
                        pitch: 0,
                        length: 0,
                    },
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest Pokemon cry species keys must be exact");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata species id ' CHIKORITA' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon_cries: [(
                    "CHIKORITA".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY CHIKORITA".to_string(),
                        pitch: 0,
                        length: 0,
                    },
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        let error = data
            .apply_modpack(&manifest)
            .expect_err("manifest Pokemon cry audio ids must be exact");
        assert!(
            format!("{error:#}").contains(
                "Pokemon cry metadata audio id 'CRY CHIKORITA' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let manifest = ModpackManifest {
            payload: ModpackPayload {
                pokemon_cries: [(
                    "CHIKORITA".to_string(),
                    PokemonCryMetadata {
                        cry: "CRY_CHIKORITA".to_string(),
                        pitch: -461,
                        length: 416,
                    },
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };
        data.apply_modpack(&manifest)
            .expect("manifest Pokemon cry signed word metadata should merge");
        assert_eq!(
            data.pokemon_cries.get("CHIKORITA"),
            Some(&PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: -461,
                length: 416,
            })
        );
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_audio_asset_ids() {
        let mut data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music(
                    "MUSIC_DUPLICATE",
                    "content-packs/test/music/MUSIC_DUPLICATE.pcm",
                )
                .expect("valid base audio asset"),
            ],
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            payload: ModpackPayload {
                audio: [(
                    "MUSIC_DUPLICATE".to_string(),
                    ModpackAudioAsset::music(
                        "MUSIC_DUPLICATE",
                        "content-packs/test/music/MUSIC_DUPLICATE.pcm",
                    )
                    .expect("valid manifest audio asset"),
                )]
                .into_iter()
                .collect(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate audio asset manifest must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate audio asset id 'MUSIC_DUPLICATE'"),
            "{error:#}"
        );
    }

    #[test]
    fn base_game_data_is_loaded_from_the_core_modular_pack() {
        let root = repository_root_for_tests();
        let data = AssetRoot::new(root)
            .load_base_game_data()
            .expect("load base game data");

        assert_eq!(data.pokemon.len(), 251);
        assert_eq!(data.pokemon["BULBASAUR"].base_stats.hp, 45);
        assert_eq!(data.moves.len(), 251);
        assert_eq!(data.moves["POUND"].pp, 35);
        let source_indices = data
            .moves
            .values()
            .map(|move_data| move_data.source_index)
            .collect::<BTreeSet<_>>();
        assert_eq!(source_indices, (1..=251).collect::<BTreeSet<_>>());
        assert_eq!(data.learnsets.len(), 251);
        assert_eq!(data.learnsets["BULBASAUR"][0].1, "TACKLE");
        assert_eq!(
            data.evolutions
                .entries_for("BULBASAUR")
                .expect("BULBASAUR evolutions")[0]
                .species,
            "IVYSAUR"
        );
        assert_eq!(data.items.len(), 255);
        assert_eq!(
            data.decorations.category_order,
            vec![
                DecorationCategory::Bed,
                DecorationCategory::Carpet,
                DecorationCategory::Plant,
                DecorationCategory::Poster,
                DecorationCategory::GameConsole,
                DecorationCategory::Ornament,
                DecorationCategory::BigDoll,
            ]
        );
        assert_eq!(data.decorations.decorations.len(), 45);
        assert!(data.decorations.decorations.iter().any(|decoration| {
            decoration.id == "DECO_BIG_LAPRAS_DOLL"
                && decoration.display_name == "BIG LAPRAS"
                && decoration.event_flag == "EVENT_DECO_BIG_LAPRAS_DOLL"
        }));
        assert_eq!(
            data.map_attributes["Route29"].map_constant.as_deref(),
            Some("ROUTE_29")
        );
        assert!(data.map_scripts.contains_key("Route29_MapScripts"));
        assert!(data.map_blocks.contains_key("Route29_Blocks"));
        assert!(data.npcs.contains_key("Route29"));
        assert!(!data.phone_scripts.is_empty());
        assert_eq!(
            data.runtime_spawn_points
                .get("0")
                .map(|spawn| spawn.map_name.as_str()),
            Some("PlayersHouse2F")
        );
        assert_eq!(
            data.runtime_map_metadata
                .get("ROUTE_29")
                .map(|metadata| metadata.name.as_str()),
            Some("Route29")
        );
        assert!(
            data.initialize_events
                .event_flags
                .contains(&"EVENT_RIVAL_CHERRYGROVE_CITY".to_string())
        );
        assert_eq!(
            data.story_event_script_constants.global.get("TRUE"),
            Some(&1)
        );
        for (name, expected) in [
            ("SPAWN_HOME", 0_i64),
            ("SPAWN_LANCE", 1_i64),
            ("SPAWN_RED", 2),
            ("SPAWN_NEW_BARK", 14),
            ("SPAWN_MT_SILVER", 26),
        ] {
            assert_eq!(
                data.story_event_script_constants.global.get(name),
                Some(&expected),
                "compiled story constants must retain {name} from ASM"
            );
        }
        for identifier in [14_u16, 26] {
            assert!(
                data.runtime_spawn_points
                    .values()
                    .any(|spawn| spawn.identifier == identifier),
                "compiled spawn table must contain source identifier {identifier}"
            );
        }
        assert_eq!(
            data.asm_text
                .get("WildPokemonAppearedText")
                .map(String::as_str),
            Some("Wild <RAM:wEnemyMonNickname>\nappeared!")
        );
        assert_eq!(data.move_names.first().map(String::as_str), Some("POUND"));
        assert!(data.battle_animations.contains_key("BattleAnim_Pound"));
        assert_eq!(
            data.battle_animation_table.get(1).map(String::as_str),
            Some("BattleAnim_Pound")
        );
        let battle_anim_bundle: Value =
            serde_json::from_str(&data.battle_anim_bundle).expect("battle anim bundle json");
        let sprite_anim_bundle: Value =
            serde_json::from_str(&data.sprite_anim_bundle).expect("sprite anim bundle json");
        assert!(battle_anim_bundle.get("objects").is_some());
        assert!(sprite_anim_bundle.get("oam_sets").is_some());
        assert_eq!(data.sprite_palette_defaults.get("SPRITE_CHRIS"), Some(&0));
        assert!(
            data.pokegear_town_map_palette_map
                .get("town_map")
                .is_some_and(|entries| !entries.is_empty())
        );
        assert_eq!(
            data.pokemon_cries.get("CHIKORITA").map(|cry| (
                cry.cry.as_str(),
                cry.pitch,
                cry.length
            )),
            Some(("CRY_CHIKORITA", -16, 176))
        );
        assert_eq!(
            data.pokemon_cries
                .get("AMPHAROS")
                .map(|cry| (cry.cry.as_str(), cry.pitch, cry.length)),
            Some(("CRY_AMPHAROS", -124, 232))
        );
        assert!(!data.pokemon_cries.contains_key("252"));
        assert!(
            data.flee_mons
                .buckets
                .get("always")
                .is_some_and(|species| species.contains(&"RAIKOU".to_string()))
        );
        assert_eq!(
            data.pc_strings
                .get("PCString_ChooseaPKMN")
                .map(String::as_str),
            Some("Choose a <PK><MN>.")
        );
        assert_eq!(
            data.menu_icons.get("CHIKORITA").map(String::as_str),
            Some("ICON_ODDISH")
        );
        assert_eq!(
            data.pokedex_entries
                .get("CHIKORITA")
                .map(|entry| entry.classification.as_str()),
            Some("LEAF")
        );
        assert!(data.pokemon_frontpic_anim.contains_key("CHIKORITA"));
    }

    #[test]
    fn base_core_pack_compiles_with_exported_playability_rules() {
        let root = repository_root_for_tests();
        let compiled = AssetRoot::new(root)
            .compile_modpacks(&[], ModpackCompileOptions::default())
            .expect("compile base core pack");

        assert!(
            !compiled.report().has_errors(),
            "{:?}",
            compiled.report().diagnostics
        );
        assert!(
            compiled
                .report()
                .solvable_events
                .contains(&"EVENT_HALL_OF_FAME".to_string())
        );
    }

    #[test]
    fn modpack_payload_empty_sections_are_authoritative() {
        let existing_species = species();
        let mut data = GameDataSet {
            pokemon: [(existing_species.id.clone(), existing_species.clone())]
                .into_iter()
                .collect(),
            moves: [(
                "POUND".to_string(),
                Move {
                    source_index: 1,
                    name: "POUND".to_string(),
                    move_type: pokemon_type("NORMAL"),
                    power: 40,
                    accuracy: 100,
                    pp: 35,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                },
            )]
            .into_iter()
            .collect(),
            fishing: FishingCatalog {
                groups: [(
                    "FISHGROUP_LAKE".to_string(),
                    crystal_core::world::fishing::FishingGroup {
                        source_index: 1,
                        bite_threshold: crystal_core::world::fishing::threshold(50, true),
                        rod_tables: BTreeMap::new(),
                    },
                )]
                .into_iter()
                .collect(),
                time_groups: BTreeMap::new(),
                swarm_rules: BTreeMap::new(),
                rod_items: BTreeMap::new(),
            },
            flee_mons: FleeMonTables::for_crystal(
                vec!["RAIKOU".to_string()],
                vec!["ENTEI".to_string()],
                vec!["SUICUNE".to_string()],
            ),
            initialize_events: InitializeEventsConfig {
                event_flags: vec!["EVENT_GOT_A_POKEMON_FROM_ELM".to_string()],
                engine_flags: vec!["ENGINE_POKEGEAR".to_string()],
                variable_sprites: [(
                    "SPRITE_WEIRD_TREE".to_string(),
                    "SPRITE_SUDOWOODO".to_string(),
                )]
                .into_iter()
                .collect(),
            },
            story_event_script_constants: StoryEventScriptConstants {
                global: [("TRUE".to_string(), 1)].into_iter().collect(),
                maps: BTreeMap::new(),
            },
            battle_reward_rules: BattleRewardRules {
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
            },
            battle_escape_rules: BattleEscapeRules {
                player_speed_multiplier: 32,
                enemy_speed_divisor: 4,
                failed_attempt_bonus: 30,
                rng_roll_values: 256,
            },
            step_event_rules: StepEventRules {
                poison_step_interval: 4,
                egg_step_trigger: 1,
                hatched_egg_happiness: 120,
                poison_status: "POISON".to_string(),
                egg_nickname: "EGG".to_string(),
                happiness_step_counter_mask: 0xff,
                happiness_step_counter_target: 0,
            },
            capture_wobble_probabilities: vec![CaptureWobbleProbability {
                catch_rate: u8::MAX,
                chance: u8::MAX,
            }],
            move_priorities: MovePriorityTable {
                base_priority: 0,
                effect_priorities: [("EFFECT_QUICK_ATTACK".to_string(), 1)]
                    .into_iter()
                    .collect(),
                move_priorities: Vec::new(),
            },
            pc_strings: [("PCString_Deposit".to_string(), "Deposit".to_string())]
                .into_iter()
                .collect(),
            menu_icons: [("CHIKORITA".to_string(), "ICON_ODDISH".to_string())]
                .into_iter()
                .collect(),
            pokedex_entries: [(
                "CHIKORITA".to_string(),
                RuntimePokedexEntry {
                    species: "CHIKORITA".to_string(),
                    classification: "LEAF".to_string(),
                    height_digits: 9,
                    weight_digits: 64,
                    pages: vec![
                        "A sweet aroma gently wafts from the leaf on its head.".to_string(),
                    ],
                },
            )]
            .into_iter()
            .collect(),
            pokemon_frontpic_anim: [(
                "CHIKORITA".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: "endanim".to_string(),
                        ..FrontpicAnimCommand::default()
                    }],
                },
            )]
            .into_iter()
            .collect(),
            asm_text: [("Text_Greeting".to_string(), "Hello.".to_string())]
                .into_iter()
                .collect(),
            move_names: vec!["POUND".to_string()],
            battle_animations: [(
                "BattleAnim_Pound".to_string(),
                vec!["anim_wait 1".to_string()],
            )]
            .into_iter()
            .collect(),
            battle_animation_table: vec![
                "BattleAnim_0".to_string(),
                "BattleAnim_Pound".to_string(),
            ],
            battle_anim_bundle: "{\"objects\":[]}".to_string(),
            sprite_anim_bundle: "{\"oam_sets\":[]}".to_string(),
            sprite_palette_defaults: [("SPRITE_CHRIS".to_string(), 0)].into_iter().collect(),
            pokegear_town_map_palette_map: [(
                "town_map".to_string(),
                vec!["PAL_ROUTE".to_string()],
            )]
            .into_iter()
            .collect(),
            pokegear_landmarks: PokegearLandmarksPayload {
                landmarks: vec![PokegearLandmark {
                    id: 1,
                    constant: "LANDMARK_ROUTE_29".to_string(),
                    label: "ROUTE_29".to_string(),
                    name: "Route 29".to_string(),
                    x: 2,
                    y: 3,
                    region: "johto".to_string(),
                }],
                map_to_landmark: [("ROUTE_29".to_string(), "LANDMARK_ROUTE_29".to_string())]
                    .into_iter()
                    .collect(),
            },
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
            trainers: TrainerCatalog {
                trainers: [(
                    "YOUNGSTER_JOEY".to_string(),
                    test_trainer("JOEY", "MUSIC_HIKER_ENCOUNTER"),
                )]
                .into_iter()
                .collect(),
            },
            phone_contacts: PhoneContactCatalog(
                [("PHONE_MOM".to_string(), test_phone_contact("PHONE_MOM"))]
                    .into_iter()
                    .collect(),
            ),
            audio: vec![
                ModpackAudioAsset::music("MUSIC_ROUTE_29", "mods/new/music/MUSIC_ROUTE_29.pcm")
                    .expect("music asset"),
            ],
            tilesets: [("johto".to_string(), test_tileset_definition())]
                .into_iter()
                .collect(),
            playability: PlayabilityRules {
                start_maps: vec!["Route29".to_string()],
                goal_maps: vec!["Route30".to_string()],
                require_all_maps_reachable: true,
                require_walkable_maps: true,
                ..PlayabilityRules::default()
            },
            ..GameDataSet::default()
        };
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "empty-authoritative".to_string(),
                name: "Empty Authoritative".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            payload: ModpackPayload {
                pokemon: BTreeMap::new(),
                moves: BTreeMap::new(),
                fishing: FishingCatalog::default(),
                flee_mons: FleeMonTables::default(),
                initialize_events: InitializeEventsConfig::default(),
                story_event_script_constants: StoryEventScriptConstants::default(),
                battle_reward_rules: BattleRewardRules::default(),
                battle_escape_rules: BattleEscapeRules::default(),
                step_event_rules: StepEventRules::default(),
                capture_wobble_probabilities: Vec::new(),
                move_priorities: MovePriorityTable::default(),
                pc_strings: BTreeMap::new(),
                menu_icons: BTreeMap::new(),
                pokedex_entries: BTreeMap::new(),
                pokemon_frontpic_anim: BTreeMap::new(),
                asm_text: BTreeMap::new(),
                move_names: Vec::new(),
                battle_animations: BTreeMap::new(),
                battle_animation_table: Vec::new(),
                battle_anim_bundle: String::new(),
                sprite_anim_bundle: String::new(),
                sprite_palette_defaults: BTreeMap::new(),
                pokegear_town_map_palette_map: BTreeMap::new(),
                pokegear_landmarks: PokegearLandmarksPayload {
                    landmarks: Vec::new(),
                    map_to_landmark: BTreeMap::new(),
                },
                pokemon_cries: BTreeMap::new(),
                trainers: TrainerCatalog::default(),
                phone_contacts: PhoneContactCatalog::default(),
                audio: BTreeMap::new(),
                tilesets: BTreeMap::new(),
                playability: PlayabilityRules::default(),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        data.apply_modpack(&manifest)
            .expect("apply explicit empty authoritative sections");

        assert!(data.pokemon.is_empty());
        assert!(data.moves.is_empty());
        assert!(data.fishing.groups.is_empty());
        assert!(data.flee_mons.is_empty());
        assert!(data.initialize_events.event_flags.is_empty());
        assert!(data.initialize_events.engine_flags.is_empty());
        assert!(data.initialize_events.variable_sprites.is_empty());
        assert!(data.story_event_script_constants.global.is_empty());
        assert!(data.story_event_script_constants.maps.is_empty());
        assert_eq!(data.battle_reward_rules, BattleRewardRules::default());
        assert_eq!(data.battle_escape_rules, BattleEscapeRules::default());
        assert_eq!(data.step_event_rules, StepEventRules::default());
        assert!(data.capture_wobble_probabilities.is_empty());
        assert_eq!(data.move_priorities, MovePriorityTable::default());
        assert!(data.pc_strings.is_empty());
        assert!(data.menu_icons.is_empty());
        assert!(data.pokedex_entries.is_empty());
        assert!(data.pokemon_frontpic_anim.is_empty());
        assert!(data.asm_text.is_empty());
        assert!(data.move_names.is_empty());
        assert!(data.battle_animations.is_empty());
        assert!(data.battle_animation_table.is_empty());
        assert!(data.battle_anim_bundle.is_empty());
        assert!(data.sprite_anim_bundle.is_empty());
        assert!(data.sprite_palette_defaults.is_empty());
        assert!(data.pokegear_town_map_palette_map.is_empty());
        assert!(data.pokegear_landmarks.landmarks.is_empty());
        assert!(data.pokegear_landmarks.map_to_landmark.is_empty());
        assert!(data.pokemon_cries.is_empty());
        assert!(data.trainers.trainers.is_empty());
        assert!(data.phone_contacts.0.is_empty());
        assert!(data.audio.is_empty());
        assert!(data.tilesets.is_empty());
        assert_eq!(data.playability, PlayabilityRules::default());
    }

    #[test]
    fn modpack_overlay_rejects_duplicate_pokemon_by_stable_id() {
        let mut data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            moves: [(
                "SPARK".to_string(),
                Move {
                    source_index: 1,
                    name: "SPARK".to_string(),
                    move_type: pokemon_type("ELECTRIC"),
                    power: 40,
                    accuracy: 100,
                    pp: 30,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                },
            )]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        let replacement = PokemonSpecies {
            base_stats: BaseStats::new(99, 50, 40, 60, 70, 50),
            ..species()
        };
        let manifest = ModpackManifest {
            metadata: ModpackMetadata {
                id: "overlay".to_string(),
                name: "Overlay".to_string(),
                version: "1.0.0".to_string(),
                author: None,
                description: None,
            },
            payload: ModpackPayload {
                pokemon: pokemon_payload(vec![replacement]),
                moves: move_payload(vec![Move {
                    source_index: 1,
                    name: "NEW_MOVE".to_string(),
                    move_type: pokemon_type("NORMAL"),
                    power: 1,
                    accuracy: 100,
                    pp: 40,
                    effect: "NORMAL_HIT".to_string(),
                    effect_chance: 0,
                    stat: None,
                    amount: None,
                }]),
                ..ModpackPayload::default()
            },
            ..ModpackManifest::default()
        };

        let error = data
            .apply_modpack(&manifest)
            .expect_err("duplicate Pokemon species must not be overwritten");

        assert!(
            format!("{error:#}").contains("duplicate Pokemon species 'NEW_MON'"),
            "{error:#}"
        );
        assert_eq!(data.pokemon["NEW_MON"].base_stats.hp, 40);
        assert!(data.moves.contains_key("SPARK"));
        assert!(!data.moves.contains_key("NEW_MOVE"));
    }

    #[test]
    fn verifier_rejects_missing_encounter_species_before_pack_is_playable() {
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
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass: Some(WildEncounterTable {
                        morning: vec![
                            WildEncounter {
                                level: 3,
                                species: "MISSING_MON".to_string(),
                            },
                            WildEncounter {
                                level: 3,
                                species: " MISSING_MON".to_string(),
                            },
                        ],
                        ..WildEncounterTable::default()
                    }),
                    ..WildEncounterData::default()
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

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_encounter_species"
                && diagnostic.message.contains("MISSING_MON")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_encounter_species"
                && diagnostic.message.contains(" MISSING_MON")
        }));
    }

    #[test]
    fn verifier_rejects_wild_encounter_rate_table_mismatches_without_defaults() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [
                (
                    "Start".to_string(),
                    WildEncounterData {
                        map_name: "Start".to_string(),
                        grass_rates: Some(
                            [
                                (" DAY".to_string(), 5),
                                ("DAY".to_string(), 20),
                                ("night".to_string(), 10),
                            ]
                            .into_iter()
                            .collect(),
                        ),
                        water_rate: Some(15),
                        swarm_overrides: BTreeMap::new(),
                        zones: Vec::new(),
                        grass: Some(WildEncounterTable {
                            morning: vec![WildEncounter {
                                level: 3,
                                species: "NEW_MON".to_string(),
                            }],
                            ..WildEncounterTable::default()
                        }),
                        water: Some(WildEncounterTable {
                            morning: vec![WildEncounter {
                                level: 10,
                                species: "NEW_MON".to_string(),
                            }],
                            ..WildEncounterTable::default()
                        }),
                    },
                ),
                (
                    " Start".to_string(),
                    WildEncounterData {
                        map_name: " Start".to_string(),
                        ..WildEncounterData::default()
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

        assert!(report.has_errors());
        for code in [
            "invalid_encounter_map",
            "invalid_grass_encounter_rate_time",
            "unknown_grass_encounter_rate_time",
            "missing_grass_encounter_rate",
            "empty_grass_encounter_slots",
            "empty_water_encounter_slots",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing diagnostic {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_present_wild_encounter_tables_without_exact_rates() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass_rates: None,
                    water_rate: None,
                    swarm_overrides: BTreeMap::new(),
                    zones: Vec::new(),
                    grass: Some(WildEncounterTable::default()),
                    water: Some(WildEncounterTable::default()),
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

        assert!(report.has_errors());
        for time in ["morning", "day", "night"] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "missing_grass_encounter_rate"
                        && diagnostic.message.contains(time)
                }),
                "missing grass rate diagnostic for {time}: {:?}",
                report.diagnostics
            );
        }
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing_water_encounter_rate"),
            "missing water rate diagnostic: {:?}",
            report.diagnostics
        );
    }

    #[test]
    fn verifier_rejects_positive_wild_encounter_rates_without_tables() {
        let data = GameDataSet {
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass_rates: Some([("day".to_string(), 20)].into_iter().collect()),
                    water_rate: Some(15),
                    ..WildEncounterData::default()
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

        assert!(report.has_errors());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "missing_grass_encounter_table" })
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.code == "missing_water_encounter_table" })
        );
    }

    #[test]
    fn verifier_rejects_wild_encounter_tables_shorter_than_slot_tables() {
        let data = GameDataSet {
            pokemon: [("NEW_MON".to_string(), species())].into_iter().collect(),
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            encounter_slot_tables: EncounterSlotTables::for_crystal(
                vec![
                    crystal_core::world::encounters::EncounterSlotChance {
                        threshold: 50,
                        slot: 0,
                    },
                    crystal_core::world::encounters::EncounterSlotChance {
                        threshold: 100,
                        slot: 1,
                    },
                ],
                vec![crystal_core::world::encounters::EncounterSlotChance {
                    threshold: 100,
                    slot: 0,
                }],
            ),
            wild_encounters: [(
                "Start".to_string(),
                WildEncounterData {
                    map_name: "Start".to_string(),
                    grass_rates: Some(
                        [
                            ("morning".to_string(), 0),
                            ("day".to_string(), 10),
                            ("night".to_string(), 0),
                        ]
                        .into_iter()
                        .collect(),
                    ),
                    grass: Some(WildEncounterTable {
                        day: vec![WildEncounter {
                            level: 3,
                            species: "NEW_MON".to_string(),
                        }],
                        ..WildEncounterTable::default()
                    }),
                    ..WildEncounterData::default()
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
            diagnostic.code == "unresolved_encounter_slot_index"
                && diagnostic.subject == "Start:grass:day"
                && diagnostic.message.contains("slot 1")
        }));
    }

    #[test]
    fn verifier_rejects_present_field_encounter_tables_with_unusable_buckets() {
        let data = GameDataSet {
            pokemon: [("NEW_MON".to_string(), species())].into_iter().collect(),
            maps: [(
                "Start".to_string(),
                test_map_module("Start", "START_MAP", None),
            )]
            .into_iter()
            .collect(),
            field_encounters: [
                (
                    "Start".to_string(),
                    FieldEncounterData::for_crystal(
                        "Start",
                        Some(FieldEncounterTable {
                            common: vec![FieldEncounterEntry {
                                weight: 90,
                                species: "NEW_MON".to_string(),
                                level: 3,
                                sleep_turns_by_time: Default::default(),
                            }],
                            rare: Vec::new(),
                        }),
                        Some(FieldEncounterTable {
                            common: vec![FieldEncounterEntry {
                                weight: 0,
                                species: " NEW_MON".to_string(),
                                level: 8,
                                sleep_turns_by_time: Default::default(),
                            }],
                            rare: Vec::new(),
                        }),
                    ),
                ),
                (
                    " Start".to_string(),
                    FieldEncounterData::for_crystal(" Start", None, None),
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

        assert!(report.has_errors());
        for code in [
            "invalid_field_encounter_map",
            "invalid_field_encounter_species",
            "invalid_field_encounter_weight_total",
            "empty_field_encounter_bucket",
            "zero_weight_field_encounter",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing diagnostic {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_missing_pcm_asset_files() {
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music(
                    "MUSIC_MISSING_THEME",
                    "content-packs/test/music/MUSIC_MISSING_THEME.pcm",
                )
                .expect("valid PCM asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_audio_file" && diagnostic.subject == "MUSIC_MISSING_THEME"
        }));
    }

    #[test]
    fn verifier_rejects_duplicate_audio_asset_ids_before_catalog_collapse() {
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music(
                    "MUSIC_DUPLICATE",
                    "content-packs/test/music/MUSIC_DUPLICATE.pcm",
                )
                .expect("valid music asset shape"),
                ModpackAudioAsset::music(
                    "MUSIC_DUPLICATE",
                    "content-packs/test/music/MUSIC_DUPLICATE.pcm",
                )
                .expect("valid duplicate music asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_audio_asset" && diagnostic.subject == "MUSIC_DUPLICATE"
        }));
    }

    #[test]
    fn verifier_does_not_count_invalid_audio_assets_as_runtime_sections() {
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset {
                    id: "MUSIC_BAD".to_string(),
                    path: "content-packs/test/music/MUSIC_BAD.pcm".to_string(),
                    kind: ModpackAudioKind::Music,
                    source: ModpackAudioSource::Pcm,
                    sfx_priority: None,
                    pcm_format: None,
                    pcm_frame_count: None,
                    payload_hash: None,
                    loop_start_sample: None,
                    loop_end_sample: None,
                    midi_program: None,
                },
                ModpackAudioAsset {
                    id: "SFX_BAD".to_string(),
                    path: "content-packs/test/sfx/SFX_BAD.pcm".to_string(),
                    kind: ModpackAudioKind::SoundEffect,
                    source: ModpackAudioSource::Pcm,
                    sfx_priority: Some(0x41),
                    pcm_format: None,
                    pcm_frame_count: None,
                    payload_hash: None,
                    loop_start_sample: None,
                    loop_end_sample: None,
                    midi_program: None,
                },
                ModpackAudioAsset {
                    id: "CRY_BAD".to_string(),
                    path: "content-packs/test/cries/CRY_BAD.pcm".to_string(),
                    kind: ModpackAudioKind::Cry,
                    source: ModpackAudioSource::Pcm,
                    sfx_priority: None,
                    pcm_format: None,
                    pcm_frame_count: None,
                    payload_hash: None,
                    loop_start_sample: None,
                    loop_end_sample: None,
                    midi_program: None,
                },
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for code in [
            "missing_runtime_audio",
            "missing_runtime_music_audio",
            "missing_runtime_sound_effects",
            "missing_runtime_cry_audio",
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code),
                "missing diagnostic {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_invalid_pcm_asset_bytes() {
        let root = temp_test_path("invalid-pcm-root");
        let pcm_path = root.join("apps/web/assets/data/content-packs/test/music/MUSIC_BAD.pcm");
        std::fs::create_dir_all(pcm_path.parent().expect("pcm parent")).expect("create pcm dir");
        std::fs::write(&pcm_path, [0_u8, 1, 2]).expect("write partial PCM frame");
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music("MUSIC_BAD", "content-packs/test/music/MUSIC_BAD.pcm")
                    .expect("valid PCM asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(&AssetRoot::new(&root), &data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pcm_file" && diagnostic.subject == "MUSIC_BAD"
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn verifier_rejects_empty_pcm_asset_bytes() {
        let root = temp_test_path("empty-pcm-root");
        let pcm_path = root.join("apps/web/assets/data/content-packs/test/cries/CRY_EMPTY.pcm");
        std::fs::create_dir_all(pcm_path.parent().expect("pcm parent")).expect("create pcm dir");
        std::fs::write(&pcm_path, []).expect("write empty pcm");
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::pcm(
                    "CRY_EMPTY",
                    "content-packs/test/cries/CRY_EMPTY.pcm",
                    ModpackAudioKind::Cry,
                    ModpackPcmAudioFormat {
                        sample_rate_hz: 22_050,
                        channels: 2,
                        bits_per_sample: 16,
                    },
                )
                .expect("valid PCM asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(&AssetRoot::new(&root), &data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pcm_file" && diagnostic.subject == "CRY_EMPTY"
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn verifier_rejects_pcm_asset_bytes_that_do_not_match_declared_frames() {
        let root = temp_test_path("unaligned-pcm-root");
        let pcm_path = root.join("apps/web/assets/data/content-packs/test/cries/CRY_UNALIGNED.pcm");
        std::fs::create_dir_all(pcm_path.parent().expect("pcm parent")).expect("create pcm dir");
        std::fs::write(&pcm_path, [0_u8, 1, 2]).expect("write unaligned pcm");
        let data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::pcm(
                    "CRY_UNALIGNED",
                    "content-packs/test/cries/CRY_UNALIGNED.pcm",
                    ModpackAudioKind::Cry,
                    ModpackPcmAudioFormat {
                        sample_rate_hz: 22_050,
                        channels: 2,
                        bits_per_sample: 16,
                    },
                )
                .expect("valid PCM asset shape"),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(&AssetRoot::new(&root), &data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pcm_file"
                && diagnostic.subject == "CRY_UNALIGNED"
                && diagnostic
                    .message
                    .contains("not a whole number of 4-byte PCM frames")
        }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn verifier_rejects_capture_rules_for_missing_species() {
        let data = GameDataSet {
            capture_rules: CaptureRules {
                fast_ball_species: ["MISSING_FAST".to_string()].into_iter().collect(),
                heavy_ball_modifiers: [("MISSING_HEAVY".to_string(), 40)].into_iter().collect(),
                ball_rules: BTreeMap::new(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_fast_ball_species" && diagnostic.subject == "MISSING_FAST"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_heavy_ball_species" && diagnostic.subject == "MISSING_HEAVY"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_capture_rule_species_before_lookup() {
        let data = GameDataSet {
            capture_rules: CaptureRules {
                fast_ball_species: [" MISSING_FAST".to_string()].into_iter().collect(),
                heavy_ball_modifiers: [("HEAVY MON".to_string(), 40)].into_iter().collect(),
                ball_rules: BTreeMap::new(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_fast_ball_species" && diagnostic.subject == " MISSING_FAST"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_heavy_ball_species" && diagnostic.subject == "HEAVY MON"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_capture_ball_rules_without_coercion() {
        let mut ball = test_item("BAD_BALL");
        ball.pocket = item_pocket("BALL");
        let data = GameDataSet {
            items: [("BAD_BALL".to_string(), ball)].into_iter().collect(),
            capture_rules: CaptureRules {
                fast_ball_species: BTreeSet::new(),
                heavy_ball_modifiers: BTreeMap::new(),
                ball_rules: [(
                    " BAD_BALL".to_string(),
                    crystal_core::battle::capture::CaptureBallRule {
                        multiplier_numerator: 1,
                        multiplier_denominator: 0,
                        battle_type: " BATTLETYPE_FISH".to_string(),
                        skip_hp_calc: false,
                        use_heavy_ball_weight_modifier: false,
                        use_level_ball_multiplier: false,
                        require_same_species: false,
                        require_same_gender: false,
                        require_fast_species: false,
                    },
                )]
                .into_iter()
                .collect(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            capture_wobble_probabilities: vec![CaptureWobbleProbability {
                catch_rate: 255,
                chance: 255,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for code in [
            "invalid_capture_ball_rule_item",
            "invalid_capture_ball_id",
            "invalid_capture_ball_battle_type",
            "invalid_capture_ball_multiplier",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == code
                        && diagnostic.subject == "capture_rules:ball_rules: BAD_BALL"
                }),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_capture_rules_for_missing_ball_items() {
        let mut ball = test_item("POKE_BALL");
        ball.pocket = item_pocket("BALL");
        let rule = crystal_core::battle::capture::CaptureBallRule {
            multiplier_numerator: 1,
            multiplier_denominator: 1,
            battle_type: String::new(),
            skip_hp_calc: false,
            use_heavy_ball_weight_modifier: false,
            use_level_ball_multiplier: false,
            require_same_species: false,
            require_same_gender: false,
            require_fast_species: false,
        };
        let data = GameDataSet {
            items: [("POKE_BALL".to_string(), ball)].into_iter().collect(),
            capture_rules: CaptureRules {
                fast_ball_species: BTreeSet::new(),
                heavy_ball_modifiers: BTreeMap::new(),
                ball_rules: [
                    ("POKE_BALL".to_string(), rule.clone()),
                    ("MOD_BALL".to_string(), rule),
                ]
                .into_iter()
                .collect(),
                guaranteed_capture_balls: BTreeSet::from([
                    " MASTER_BALL".to_string(),
                    "MOD_BALL".to_string(),
                ]),
                status_bonus: BTreeMap::new(),
            },
            capture_wobble_probabilities: vec![CaptureWobbleProbability {
                catch_rate: 255,
                chance: 255,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            (
                "unknown_capture_ball_rule_item",
                "capture_rules:ball_rules:MOD_BALL",
            ),
            (
                "invalid_guaranteed_capture_ball",
                "capture_rules:guaranteed_capture_balls",
            ),
            (
                "unknown_guaranteed_capture_ball",
                "capture_rules:guaranteed_capture_balls",
            ),
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == expected.0 && diagnostic.subject == expected.1
                }),
                "missing {:?}: {:?}",
                expected,
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_malformed_capture_wobble_probabilities_without_coercion() {
        let mut ball = test_item("POKE_BALL");
        ball.pocket = item_pocket("BALL");
        let data = GameDataSet {
            items: [("POKE_BALL".to_string(), ball)].into_iter().collect(),
            capture_rules: CaptureRules {
                fast_ball_species: BTreeSet::new(),
                heavy_ball_modifiers: BTreeMap::new(),
                ball_rules: [(
                    "POKE_BALL".to_string(),
                    crystal_core::battle::capture::CaptureBallRule {
                        multiplier_numerator: 1,
                        multiplier_denominator: 1,
                        battle_type: String::new(),
                        skip_hp_calc: false,
                        use_heavy_ball_weight_modifier: false,
                        use_level_ball_multiplier: false,
                        require_same_species: false,
                        require_same_gender: false,
                        require_fast_species: false,
                    },
                )]
                .into_iter()
                .collect(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            capture_wobble_probabilities: vec![
                CaptureWobbleProbability {
                    catch_rate: 0,
                    chance: 0,
                },
                CaptureWobbleProbability {
                    catch_rate: 10,
                    chance: 20,
                },
                CaptureWobbleProbability {
                    catch_rate: 9,
                    chance: 30,
                },
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for code in [
            "invalid_capture_wobble_catch_rate",
            "unordered_capture_wobble_probability",
            "incomplete_capture_wobble_probabilities",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == code && diagnostic.subject == "capture_wobble_probabilities"
                }),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_ball_pocket_items_without_poke_ball_fallback() {
        let mut mod_ball = test_item("MOD_BALL");
        mod_ball.pocket = item_pocket("BALL");
        let mut poke_ball = test_item("POKE_BALL");
        poke_ball.pocket = item_pocket("BALL");
        let data = GameDataSet {
            items: [
                ("MOD_BALL".to_string(), mod_ball),
                ("POKE_BALL".to_string(), poke_ball),
            ]
            .into_iter()
            .collect(),
            capture_rules: CaptureRules {
                fast_ball_species: BTreeSet::new(),
                heavy_ball_modifiers: BTreeMap::new(),
                ball_rules: [(
                    "POKE_BALL".to_string(),
                    crystal_core::battle::capture::CaptureBallRule {
                        multiplier_numerator: 1,
                        multiplier_denominator: 1,
                        battle_type: String::new(),
                        skip_hp_calc: false,
                        use_heavy_ball_weight_modifier: false,
                        use_level_ball_multiplier: false,
                        require_same_species: false,
                        require_same_gender: false,
                        require_fast_species: false,
                    },
                )]
                .into_iter()
                .collect(),
                guaranteed_capture_balls: BTreeSet::new(),
                status_bonus: BTreeMap::new(),
            },
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_capture_ball_item" && diagnostic.subject == "MOD_BALL"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_capture_ball_item" && diagnostic.subject == "POKE_BALL"
        }));
    }

    #[test]
    fn verifier_does_not_report_unknown_capture_ball_for_invalid_item_script_id() {
        let mut bad_ball = test_item("BAD_BALL");
        bad_ball.pocket = item_pocket("BALL");
        bad_ball.script_name = " BAD_BALL".to_string();
        let data = GameDataSet {
            items: [("BAD_BALL".to_string(), bad_ball)].into_iter().collect(),
            capture_wobble_probabilities: vec![CaptureWobbleProbability {
                catch_rate: 255,
                chance: 255,
            }],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_script_name" && diagnostic.subject == "BAD_BALL"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_capture_ball_item" && diagnostic.subject == "BAD_BALL"
        }));
    }

    #[test]
    fn verifier_accepts_modpack_item_menu_ids_as_definitive_data() {
        let mut mod_menu = test_item("MOD_MENU_ITEM");
        mod_menu.field_menu = "ITEMMENU_MODDED".to_string();
        mod_menu.field_usable = true;
        mod_menu.battle_menu = "ITEMMENU_NOUSE".to_string();
        mod_menu.battle_usable = false;
        let mut exact_menu = test_item("EXACT_MENU_ITEM");
        exact_menu.field_menu = "ITEMMENU_CURRENT".to_string();
        exact_menu.field_usable = true;
        exact_menu.battle_menu = "ITEMMENU_PARTY".to_string();
        exact_menu.battle_usable = true;
        let data = GameDataSet {
            items: [
                ("MOD_MENU_ITEM".to_string(), mod_menu),
                ("EXACT_MENU_ITEM".to_string(), exact_menu),
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

        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_item_menu"
                && (diagnostic.subject == "MOD_MENU_ITEM"
                    || diagnostic.subject == "EXACT_MENU_ITEM")
        }));
    }

    #[test]
    fn verifier_rejects_item_menu_usability_contradictions() {
        let mut bad_field = test_item("BAD_FIELD_MENU");
        bad_field.field_menu = "ITEMMENU_NOUSE".to_string();
        bad_field.field_usable = true;
        let mut bad_battle = test_item("BAD_BATTLE_MENU");
        bad_battle.battle_menu = "ITEMMENU_PARTY".to_string();
        bad_battle.battle_usable = false;
        let data = GameDataSet {
            items: [
                ("BAD_FIELD_MENU".to_string(), bad_field),
                ("BAD_BATTLE_MENU".to_string(), bad_battle),
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
            diagnostic.code == "invalid_item_field_usable_menu"
                && diagnostic.subject == "BAD_FIELD_MENU"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_battle_usable_menu"
                && diagnostic.subject == "BAD_BATTLE_MENU"
        }));
    }

    #[test]
    fn verifier_rejects_item_script_name_whitespace_without_coercion() {
        let mut item = test_item("MOD_ITEM");
        item.script_name = " MOD_ITEM".to_string();
        let data = GameDataSet {
            items: [("MOD_ITEM".to_string(), item)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_script_name" && diagnostic.subject == "MOD_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_item_display_name_whitespace_without_inference() {
        let mut bad = test_item("BAD_NAME_ITEM");
        bad.name = " Flash Step Charm".to_string();
        let mut exact = test_item("EXACT_NAME_ITEM");
        exact.name = "Flash Step Charm".to_string();
        let data = GameDataSet {
            items: [
                ("BAD_NAME_ITEM".to_string(), bad),
                ("EXACT_NAME_ITEM".to_string(), exact),
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
            diagnostic.code == "invalid_item_name" && diagnostic.subject == "BAD_NAME_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_name" && diagnostic.subject == "EXACT_NAME_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_item_description_whitespace_without_inference() {
        let mut bad = test_item("BAD_DESCRIPTION_ITEM");
        bad.description = " A charm with exact text.".to_string();
        let mut exact = test_item("EXACT_DESCRIPTION_ITEM");
        exact.description = "A charm with exact text.".to_string();
        let data = GameDataSet {
            items: [
                ("BAD_DESCRIPTION_ITEM".to_string(), bad),
                ("EXACT_DESCRIPTION_ITEM".to_string(), exact),
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
            diagnostic.code == "invalid_item_description"
                && diagnostic.subject == "BAD_DESCRIPTION_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_description"
                && diagnostic.subject == "EXACT_DESCRIPTION_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_item_pocket_whitespace_without_enum_restriction() {
        let mut bad = test_item("BAD_POCKET_ITEM");
        bad.pocket = " BATTLE_PASS".to_string();
        let mut exact = test_item("EXACT_POCKET_ITEM");
        exact.pocket = "BATTLE_PASS".to_string();
        let data = GameDataSet {
            items: [
                ("BAD_POCKET_ITEM".to_string(), bad),
                ("EXACT_POCKET_ITEM".to_string(), exact),
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
            diagnostic.code == "invalid_item_pocket" && diagnostic.subject == "BAD_POCKET_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_pocket" && diagnostic.subject == "EXACT_POCKET_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_item_effect_whitespace_without_enum_restriction() {
        let mut bad = test_item("BAD_EFFECT_ITEM");
        bad.effect = " MODDED_FLASH_STEP".to_string();
        let mut exact = test_item("EXACT_EFFECT_ITEM");
        exact.effect = "MODDED_FLASH_STEP".to_string();
        let data = GameDataSet {
            items: [
                ("BAD_EFFECT_ITEM".to_string(), bad),
                ("EXACT_EFFECT_ITEM".to_string(), exact),
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
            diagnostic.code == "invalid_item_effect" && diagnostic.subject == "BAD_EFFECT_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_effect" && diagnostic.subject == "EXACT_EFFECT_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_item_held_effect_whitespace_without_enum_restriction() {
        let mut bad = test_item("BAD_HELD_EFFECT_ITEM");
        bad.held_effect = " HELD_MODDED".to_string();
        let mut exact = test_item("EXACT_HELD_EFFECT_ITEM");
        exact.held_effect = "HELD_MODDED".to_string();
        let data = GameDataSet {
            items: [
                ("BAD_HELD_EFFECT_ITEM".to_string(), bad),
                ("EXACT_HELD_EFFECT_ITEM".to_string(), exact),
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
            diagnostic.code == "invalid_item_held_effect"
                && diagnostic.subject == "BAD_HELD_EFFECT_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_held_effect"
                && diagnostic.subject == "EXACT_HELD_EFFECT_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_item_property_whitespace_without_requiring_property() {
        let mut bad = test_item("BAD_PROPERTY_ITEM");
        bad.property = " CANT_SELECT".to_string();
        let mut internal_space = test_item("BAD_INTERNAL_PROPERTY_ITEM");
        internal_space.property = "CANT SELECT".to_string();
        let mut exact = test_item("EXACT_PROPERTY_ITEM");
        exact.property = "CANT_SELECT".to_string();
        let mut empty = test_item("EMPTY_PROPERTY_ITEM");
        empty.property = String::new();
        let data = GameDataSet {
            items: [
                ("BAD_PROPERTY_ITEM".to_string(), bad),
                ("BAD_INTERNAL_PROPERTY_ITEM".to_string(), internal_space),
                ("EXACT_PROPERTY_ITEM".to_string(), exact),
                ("EMPTY_PROPERTY_ITEM".to_string(), empty),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_property" && diagnostic.subject == "BAD_PROPERTY_ITEM"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_property"
                && diagnostic.subject == "BAD_INTERNAL_PROPERTY_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_item_property"
                && (diagnostic.subject == "EXACT_PROPERTY_ITEM"
                    || diagnostic.subject == "EMPTY_PROPERTY_ITEM")
        }));
    }

    #[test]
    fn verifier_rejects_item_menu_whitespace_without_enum_restriction() {
        let mut bad = test_item("BAD_MENU_ITEM");
        bad.field_menu = " ITEMMENU_MODDED".to_string();
        bad.battle_menu = String::new();
        let mut exact = test_item("EXACT_MENU_ITEM");
        exact.field_menu = "ITEMMENU_MODDED".to_string();
        exact.field_usable = true;
        exact.battle_menu = "ITEMMENU_MODDED_BATTLE".to_string();
        exact.battle_usable = true;
        let data = GameDataSet {
            items: [
                ("BAD_MENU_ITEM".to_string(), bad),
                ("EXACT_MENU_ITEM".to_string(), exact),
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
            diagnostic.code == "invalid_item_field_menu" && diagnostic.subject == "BAD_MENU_ITEM"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_item_battle_menu" && diagnostic.subject == "BAD_MENU_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            (diagnostic.code == "invalid_item_field_menu"
                || diagnostic.code == "missing_item_battle_menu")
                && diagnostic.subject == "EXACT_MENU_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_tmhm_items_without_explicit_index_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = None;
        let data = GameDataSet {
            items: [("TM_MUD_SLAP".to_string(), tm)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_item_tmhm_index" && diagnostic.subject == "TM_MUD_SLAP"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_item_tmhm_move" && diagnostic.subject == "TM_MUD_SLAP"
        }));
    }

    #[test]
    fn verifier_rejects_tmhm_items_with_zero_index_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = Some(0);
        tm.tmhm_move = Some("MUD_SLAP".to_string());
        let data = GameDataSet {
            items: [("TM_MUD_SLAP".to_string(), tm)].into_iter().collect(),
            moves: [("MUD_SLAP".to_string(), test_move("MUD_SLAP"))]
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
            diagnostic.code == "invalid_item_tmhm_index" && diagnostic.subject == "TM_MUD_SLAP"
        }));
    }

    #[test]
    fn verifier_rejects_tmhm_items_with_whitespace_move_data() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = Some(30);
        tm.tmhm_move = Some(" MUD_SLAP".to_string());
        let data = GameDataSet {
            items: [("TM_MUD_SLAP".to_string(), tm)].into_iter().collect(),
            moves: [("MUD_SLAP".to_string(), test_move("MUD_SLAP"))]
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
            diagnostic.code == "invalid_item_tmhm_move" && diagnostic.subject == "TM_MUD_SLAP"
        }));
    }

    #[test]
    fn verifier_rejects_tmhm_items_with_unknown_move_without_coercion() {
        let mut tm = test_item("TM_MUD_SLAP");
        tm.pocket = item_pocket("TM_HM");
        tm.tmhm_index = Some(30);
        tm.tmhm_move = Some("mud_slap".to_string());
        let data = GameDataSet {
            items: [("TM_MUD_SLAP".to_string(), tm)].into_iter().collect(),
            moves: [("MUD_SLAP".to_string(), test_move("MUD_SLAP"))]
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
            diagnostic.code == "unknown_item_tmhm_move" && diagnostic.subject == "TM_MUD_SLAP"
        }));
    }

    #[test]
    fn verifier_rejects_items_without_explicit_script_name_data() {
        let mut item = test_item("FLASH_STEP_CHARM");
        item.script_name = String::new();
        let data = GameDataSet {
            items: [("FLASH_STEP_CHARM".to_string(), item)]
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
            diagnostic.code == "missing_item_script_name"
                && diagnostic.subject == "FLASH_STEP_CHARM"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_utility_item_payloads_without_effect_inference() {
        let mut bad_poke_doll = test_item("BAD_POKE_DOLL");
        bad_poke_doll.effect = "MOD_ESCAPE_ITEM".to_string();
        bad_poke_doll.battle_escape_mode = Some("ANY_BATTLE".to_string());
        let mut bad_repel = test_item("BAD_REPEL");
        bad_repel.effect = "MOD_REPEL_ITEM".to_string();
        bad_repel.repel_steps = Some(0);
        let mut bad_rope = test_item("BAD_ESCAPE_ROPE");
        bad_rope.effect = "ESCAPE_ROPE".to_string();
        let mut exact_poke_doll = test_item("POKE_DOLL");
        exact_poke_doll.effect = "MOD_ESCAPE_ITEM".to_string();
        exact_poke_doll.battle_escape_mode = Some("WILD_BATTLE".to_string());
        let mut exact_repel = test_item("REPEL");
        exact_repel.effect = "MOD_REPEL_ITEM".to_string();
        exact_repel.repel_steps = Some(100);
        let mut exact_rope = test_item("ESCAPE_ROPE");
        exact_rope.effect = "MOD_ESCAPE_ROPE".to_string();
        exact_rope.escape_rope_mode = Some("MOD_WARP".to_string());
        let data = GameDataSet {
            items: [
                ("BAD_POKE_DOLL".to_string(), bad_poke_doll),
                ("BAD_REPEL".to_string(), bad_repel),
                ("BAD_ESCAPE_ROPE".to_string(), bad_rope),
                ("POKE_DOLL".to_string(), exact_poke_doll),
                ("REPEL".to_string(), exact_repel),
                ("ESCAPE_ROPE".to_string(), exact_rope),
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

        for (code, subject) in [
            ("invalid_item_battle_escape_mode", "BAD_POKE_DOLL"),
            ("invalid_item_repel_steps", "BAD_REPEL"),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing diagnostic {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
        for subject in ["POKE_DOLL", "REPEL", "ESCAPE_ROPE"] {
            assert!(
                !report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.subject == subject)
            );
        }
    }

    #[test]
    fn verifier_rejects_field_usable_items_without_payload_or_rule() {
        let mut orphan = test_item("ORPHAN_FIELD_ITEM");
        orphan.field_menu = "ITEMMENU_CURRENT".to_string();
        orphan.field_usable = true;
        orphan.battle_menu = "ITEMMENU_NOUSE".to_string();
        orphan.battle_usable = false;
        let mut ruled = test_item("RULED_FIELD_ITEM");
        ruled.field_menu = "ITEMMENU_CURRENT".to_string();
        ruled.field_usable = true;
        ruled.battle_menu = "ITEMMENU_NOUSE".to_string();
        ruled.battle_usable = false;
        let mut data = GameDataSet {
            items: [
                ("ORPHAN_FIELD_ITEM".to_string(), orphan),
                ("RULED_FIELD_ITEM".to_string(), ruled),
            ]
            .into_iter()
            .collect(),
            ..GameDataSet::default()
        };
        data.field_moves.bicycle = FieldItemRule {
            item_id: "RULED_FIELD_ITEM".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_item_field_payload"
                && diagnostic.subject == "ORPHAN_FIELD_ITEM"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_item_field_payload"
                && diagnostic.subject == "RULED_FIELD_ITEM"
        }));
    }

    #[test]
    fn verifier_requires_field_item_rules_to_match_runtime_item_ids() {
        let mut item = test_item("RULED_FIELD_ITEM");
        item.script_name = "OTHER_SCRIPT_ITEM".to_string();
        item.field_menu = "ITEMMENU_CURRENT".to_string();
        item.field_usable = true;
        item.battle_menu = "ITEMMENU_NOUSE".to_string();
        item.battle_usable = false;
        let mut data = GameDataSet {
            items: [("RULED_FIELD_ITEM".to_string(), item)]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        data.field_moves.bicycle = FieldItemRule {
            item_id: "OTHER_SCRIPT_ITEM".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_item_id"
                && diagnostic.subject == "field_moves:bicycle"
                && diagnostic.message.contains("OTHER_SCRIPT_ITEM")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_item_field_payload"
                && diagnostic.subject == "RULED_FIELD_ITEM"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_battle_item_payloads_without_effect_inference() {
        let mut bad_restore_hp = test_item("BAD_RESTORE_HP");
        bad_restore_hp.effect = "MOD_HEAL".to_string();
        bad_restore_hp.parameter = -2;
        let mut bad_status_heal = test_item("BAD_STATUS_HEAL");
        bad_status_heal.effect = "STATUS_HEAL".to_string();
        bad_status_heal.status_heals = vec![String::new(), " POISON".to_string()];
        let mut bad_revive = test_item("BAD_REVIVE");
        bad_revive.effect = "MOD_REVIVE".to_string();
        bad_revive.revive_hp_percent = Some(0);
        let mut bad_sacred_ash = test_item("BAD_SACRED_ASH");
        bad_sacred_ash.effect = "MOD_PARTY_REVIVE".to_string();
        bad_sacred_ash.party_revive_hp_percent = Some(0);
        let mut bad_restore_pp = test_item("BAD_RESTORE_PP");
        bad_restore_pp.effect = "MOD_RESTORE_PP".to_string();
        bad_restore_pp.pp_restore_scope = Some("PARTY".to_string());
        bad_restore_pp.pp_restore_points = Some(0);
        let mut bad_pp_up = test_item("BAD_PP_UP");
        bad_pp_up.effect = "MOD_PP_UP".to_string();
        bad_pp_up.pp_up_stages = Some(4);
        let mut bad_vitamin = test_item("BAD_VITAMIN");
        bad_vitamin.effect = "MOD_VITAMIN".to_string();
        bad_vitamin.vitamin_stat = Some("LUCK".to_string());
        bad_vitamin.vitamin_stat_exp = Some(0);
        bad_vitamin.vitamin_max_stat_exp = Some(0);
        let mut bad_rare_candy = test_item("BAD_RARE_CANDY");
        bad_rare_candy.effect = "MOD_CANDY".to_string();
        bad_rare_candy.rare_candy_level_gain = Some(0);
        let mut bad_x_item = test_item("BAD_X_ITEM");
        bad_x_item.effect = "MOD_BATTLE_BOOST".to_string();
        bad_x_item.battle_stat_boost_stat = Some("LUCK".to_string());
        bad_x_item.battle_stat_boost_stages = Some(7);
        let mut bad_guard_spec = test_item("BAD_GUARD_SPEC");
        bad_guard_spec.effect = "MOD_GUARD".to_string();
        bad_guard_spec.battle_stat_drop_guard = Some(true);
        let mut bad_guard_spec_turns = test_item("BAD_GUARD_SPEC_TURNS");
        bad_guard_spec_turns.effect = "MOD_GUARD".to_string();
        bad_guard_spec_turns.battle_stat_drop_guard = Some(true);
        bad_guard_spec_turns.battle_stat_drop_guard_turns = Some(0);
        let mut bad_guard_spec_flag = test_item("BAD_GUARD_SPEC_FLAG");
        bad_guard_spec_flag.effect = "MOD_GUARD".to_string();
        bad_guard_spec_flag.battle_stat_drop_guard_turns = Some(5);
        let mut bad_dire_hit = test_item("BAD_DIRE_HIT");
        bad_dire_hit.effect = "MOD_FOCUS".to_string();
        bad_dire_hit.battle_focus_energy = Some(false);
        let mut bad_bitter_berry = test_item("BAD_BITTER_BERRY");
        bad_bitter_berry.effect = "MOD_CONFUSION_HEAL".to_string();
        bad_bitter_berry.confusion_heal = Some(false);
        let mut bad_battle_payload = test_item("BAD_BATTLE_PAYLOAD");
        bad_battle_payload.battle_menu = "ITEMMENU_PARTY".to_string();
        bad_battle_payload.battle_usable = true;

        let mut exact_restore_hp = test_item("EXACT_RESTORE_HP");
        exact_restore_hp.effect = "MOD_HEAL".to_string();
        exact_restore_hp.parameter = 20;
        let mut exact_status_heal = test_item("EXACT_STATUS_HEAL");
        exact_status_heal.effect = "MOD_STATUS_HEAL".to_string();
        exact_status_heal.status_heals = vec!["POISON".to_string()];
        let mut exact_revive = test_item("EXACT_REVIVE");
        exact_revive.effect = "MOD_REVIVE".to_string();
        exact_revive.revive_hp_percent = Some(50);
        let mut exact_sacred_ash = test_item("EXACT_SACRED_ASH");
        exact_sacred_ash.effect = "MOD_PARTY_REVIVE".to_string();
        exact_sacred_ash.party_revive_hp_percent = Some(100);
        let mut exact_restore_pp = test_item("EXACT_RESTORE_PP");
        exact_restore_pp.effect = "MOD_RESTORE_PP".to_string();
        exact_restore_pp.pp_restore_scope = Some("MOVE".to_string());
        exact_restore_pp.pp_restore_points = Some(10);
        let mut exact_pp_up = test_item("EXACT_PP_UP");
        exact_pp_up.effect = "MOD_PP_UP".to_string();
        exact_pp_up.pp_up_stages = Some(1);
        let mut exact_vitamin = test_item("EXACT_VITAMIN");
        exact_vitamin.effect = "MOD_VITAMIN".to_string();
        exact_vitamin.vitamin_stat = Some("SPECIAL".to_string());
        exact_vitamin.vitamin_stat_exp = Some(2560);
        exact_vitamin.vitamin_max_stat_exp = Some(25600);
        let mut exact_rare_candy = test_item("EXACT_RARE_CANDY");
        exact_rare_candy.effect = "MOD_CANDY".to_string();
        exact_rare_candy.rare_candy_level_gain = Some(1);
        let mut exact_x_item = test_item("EXACT_X_ITEM");
        exact_x_item.effect = "MOD_BATTLE_BOOST".to_string();
        exact_x_item.battle_stat_boost_stat = Some("SPECIAL_ATTACK".to_string());
        exact_x_item.battle_stat_boost_stages = Some(1);
        let mut exact_guard_spec = test_item("EXACT_GUARD_SPEC");
        exact_guard_spec.effect = "MOD_GUARD".to_string();
        exact_guard_spec.battle_stat_drop_guard = Some(true);
        exact_guard_spec.battle_stat_drop_guard_turns = Some(5);
        let mut exact_dire_hit = test_item("EXACT_DIRE_HIT");
        exact_dire_hit.effect = "MOD_FOCUS".to_string();
        exact_dire_hit.battle_focus_energy = Some(true);
        let mut exact_bitter_berry = test_item("EXACT_BITTER_BERRY");
        exact_bitter_berry.effect = "MOD_CONFUSION_HEAL".to_string();
        exact_bitter_berry.confusion_heal = Some(true);

        let data = GameDataSet {
            items: [
                ("BAD_RESTORE_HP".to_string(), bad_restore_hp),
                ("BAD_STATUS_HEAL".to_string(), bad_status_heal),
                ("BAD_REVIVE".to_string(), bad_revive),
                ("BAD_SACRED_ASH".to_string(), bad_sacred_ash),
                ("BAD_RESTORE_PP".to_string(), bad_restore_pp),
                ("BAD_PP_UP".to_string(), bad_pp_up),
                ("BAD_VITAMIN".to_string(), bad_vitamin),
                ("BAD_RARE_CANDY".to_string(), bad_rare_candy),
                ("BAD_X_ITEM".to_string(), bad_x_item),
                ("BAD_GUARD_SPEC".to_string(), bad_guard_spec),
                ("BAD_GUARD_SPEC_TURNS".to_string(), bad_guard_spec_turns),
                ("BAD_GUARD_SPEC_FLAG".to_string(), bad_guard_spec_flag),
                ("BAD_DIRE_HIT".to_string(), bad_dire_hit),
                ("BAD_BITTER_BERRY".to_string(), bad_bitter_berry),
                ("BAD_BATTLE_PAYLOAD".to_string(), bad_battle_payload),
                ("EXACT_RESTORE_HP".to_string(), exact_restore_hp),
                ("EXACT_STATUS_HEAL".to_string(), exact_status_heal),
                ("EXACT_REVIVE".to_string(), exact_revive),
                ("EXACT_SACRED_ASH".to_string(), exact_sacred_ash),
                ("EXACT_RESTORE_PP".to_string(), exact_restore_pp),
                ("EXACT_PP_UP".to_string(), exact_pp_up),
                ("EXACT_VITAMIN".to_string(), exact_vitamin),
                ("EXACT_RARE_CANDY".to_string(), exact_rare_candy),
                ("EXACT_X_ITEM".to_string(), exact_x_item),
                ("EXACT_GUARD_SPEC".to_string(), exact_guard_spec),
                ("EXACT_DIRE_HIT".to_string(), exact_dire_hit),
                ("EXACT_BITTER_BERRY".to_string(), exact_bitter_berry),
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

        for (code, subject) in [
            ("invalid_item_heal_amount", "BAD_RESTORE_HP"),
            ("invalid_item_status_heal", "BAD_STATUS_HEAL"),
            ("invalid_item_revive_hp_percent", "BAD_REVIVE"),
            ("invalid_item_party_revive_hp_percent", "BAD_SACRED_ASH"),
            ("invalid_item_pp_restore_scope", "BAD_RESTORE_PP"),
            ("invalid_item_pp_restore_points", "BAD_RESTORE_PP"),
            ("invalid_item_pp_up_stages", "BAD_PP_UP"),
            ("invalid_item_vitamin_stat", "BAD_VITAMIN"),
            ("invalid_item_vitamin_stat_exp", "BAD_VITAMIN"),
            ("invalid_item_vitamin_max_stat_exp", "BAD_VITAMIN"),
            ("invalid_item_rare_candy_level_gain", "BAD_RARE_CANDY"),
            ("invalid_item_battle_stat_boost_stat", "BAD_X_ITEM"),
            ("invalid_item_battle_stat_boost_stages", "BAD_X_ITEM"),
            (
                "missing_item_battle_stat_drop_guard_turns",
                "BAD_GUARD_SPEC",
            ),
            (
                "invalid_item_battle_stat_drop_guard_turns",
                "BAD_GUARD_SPEC_TURNS",
            ),
            ("missing_item_battle_stat_drop_guard", "BAD_GUARD_SPEC_FLAG"),
            ("invalid_item_battle_focus_energy", "BAD_DIRE_HIT"),
            ("invalid_item_confusion_heal", "BAD_BITTER_BERRY"),
            ("missing_item_battle_payload", "BAD_BATTLE_PAYLOAD"),
        ] {
            assert!(
                report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == code && diagnostic.subject == subject),
                "missing diagnostic {code} for {subject}: {:?}",
                report.diagnostics
            );
        }
        for subject in [
            "EXACT_RESTORE_HP",
            "EXACT_STATUS_HEAL",
            "EXACT_REVIVE",
            "EXACT_SACRED_ASH",
            "EXACT_RESTORE_PP",
            "EXACT_PP_UP",
            "EXACT_VITAMIN",
            "EXACT_RARE_CANDY",
            "EXACT_X_ITEM",
            "EXACT_GUARD_SPEC",
            "EXACT_DIRE_HIT",
            "EXACT_BITTER_BERRY",
        ] {
            assert!(
                !report
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.subject == subject),
                "unexpected diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_unknown_evolution_facts_without_case_coercion() {
        let mut source = species();
        source.id = "NEW_MON".to_string();
        source.tmhm_learnset.clear();
        let mut target = species();
        target.id = "NEW_FORM".to_string();
        target.tmhm_learnset.clear();
        let data = GameDataSet {
            pokemon: [
                ("NEW_MON".to_string(), source),
                ("NEW_FORM".to_string(), target),
            ]
            .into_iter()
            .collect(),
            items: [("THUNDERSTONE".to_string(), test_item("THUNDERSTONE"))]
                .into_iter()
                .collect(),
            evolutions: EvolutionTable(
                [
                    (
                        " new_mon".to_string(),
                        vec![EvolutionEntry::level("NEW_FORM", 20)],
                    ),
                    (
                        "new_mon".to_string(),
                        vec![EvolutionEntry::level("NEW_FORM", 20)],
                    ),
                    (
                        "NEW_MON".to_string(),
                        vec![
                            EvolutionEntry::item(" NEW_FORM", " THUNDERSTONE"),
                            EvolutionEntry::item("new_form", "thunderstone"),
                            EvolutionEntry::happiness("NEW_FORM", " MORNINGISH"),
                            EvolutionEntry::happiness("NEW_FORM", "MORNINGISH"),
                            EvolutionEntry::trade("NEW_FORM", Some(" THUNDERSTONE")),
                            EvolutionEntry::trade("NEW_FORM", Some("thunderstone")),
                            EvolutionEntry::stat("NEW_FORM", 20, " ATTACKIER"),
                            EvolutionEntry::stat("NEW_FORM", 20, "ATTACKIER"),
                            EvolutionEntry {
                                method: " MOON_PHASE".to_string(),
                                species: "NEW_FORM".to_string(),
                                level: None,
                                item: None,
                                held_item: None,
                                happiness: None,
                                stat_ratio: None,
                            },
                            EvolutionEntry {
                                method: "MOON_PHASE".to_string(),
                                species: "NEW_FORM".to_string(),
                                level: None,
                                item: None,
                                held_item: None,
                                happiness: None,
                                stat_ratio: None,
                            },
                        ],
                    ),
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

        for expected in [
            "invalid_evolution_source_species",
            "unknown_evolution_source_species",
            "invalid_evolution_target_species",
            "unknown_evolution_target_species",
            "invalid_evolution_item",
            "unknown_evolution_item",
            "invalid_evolution_happiness_window",
            "unknown_evolution_happiness_window",
            "invalid_trade_evolution_item",
            "unknown_trade_evolution_item",
            "invalid_evolution_stat_ratio",
            "unknown_evolution_stat_ratio",
            "invalid_evolution_method",
            "unknown_evolution_method",
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
    fn verifier_requires_explicit_empty_learnsets_and_evolutions() {
        let mut known_species = species();
        known_species.id = "FINAL_MON".to_string();
        known_species.tmhm_learnset.clear();
        let missing = GameDataSet {
            pokemon: [("FINAL_MON".to_string(), known_species.clone())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &missing,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_learnset" && diagnostic.subject == "FINAL_MON"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_evolutions" && diagnostic.subject == "FINAL_MON"
        }));

        let explicit_empty = GameDataSet {
            pokemon: [("FINAL_MON".to_string(), known_species)]
                .into_iter()
                .collect(),
            learnsets: [("FINAL_MON".to_string(), Vec::new())]
                .into_iter()
                .collect(),
            evolutions: EvolutionTable(
                [("FINAL_MON".to_string(), Vec::new())]
                    .into_iter()
                    .collect(),
            ),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &explicit_empty,
            &PlayabilityRules::default(),
        );

        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_learnset" && diagnostic.subject == "FINAL_MON"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_species_evolutions" && diagnostic.subject == "FINAL_MON"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_growth_rate_catalog_without_runtime_fallback() {
        let data = GameDataSet {
            growth_rates: [
                (
                    "GROWTH BAD".to_string(),
                    crystal_core::systems::experience::GrowthRateCurve {
                        id: "GROWTH BAD".to_string(),
                        numerator: 1,
                        denominator: 1,
                        quadratic: 0,
                        linear: 0,
                        constant: 0,
                    },
                ),
                (
                    "GROWTH_MISMATCH".to_string(),
                    crystal_core::systems::experience::GrowthRateCurve {
                        id: "GROWTH_OTHER".to_string(),
                        numerator: 1,
                        denominator: 1,
                        quadratic: 0,
                        linear: 0,
                        constant: 0,
                    },
                ),
                (
                    "GROWTH_ZERO".to_string(),
                    crystal_core::systems::experience::GrowthRateCurve {
                        id: "GROWTH_ZERO".to_string(),
                        numerator: 1,
                        denominator: 0,
                        quadratic: 0,
                        linear: 0,
                        constant: 0,
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

        for expected in [
            "invalid_growth_rate_id",
            "growth_rate_id_mismatch",
            "invalid_growth_rate_denominator",
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
    fn verifier_rejects_unknown_species_held_items_without_case_coercion() {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        known_species.item1 = Some(" POTION".to_string());
        known_species.item2 = Some("RARE_CANDY".to_string());
        let species_id = known_species.id.clone();
        let data = GameDataSet {
            pokemon: [(species_id.clone(), known_species)].into_iter().collect(),
            learnsets: [(species_id.clone(), Vec::new())].into_iter().collect(),
            evolutions: EvolutionTable([(species_id.clone(), Vec::new())].into_iter().collect()),
            items: [
                ("POTION".to_string(), test_item("POTION")),
                ("RARE_CANDY".to_string(), test_item("RARE_CANDY")),
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
            diagnostic.code == "invalid_species_held_item"
                && diagnostic.subject == species_id
                && diagnostic.message.contains(" POTION")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_species_held_item"
                && diagnostic.message.contains("RARE_CANDY")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_learnset_ids_without_coercion() {
        let mut known_species = species();
        known_species.tmhm_learnset = vec![" HEADBUTT".to_string(), "headbutt".to_string()];
        let species_id = known_species.id.clone();
        let data = GameDataSet {
            pokemon: [(species_id.clone(), known_species)].into_iter().collect(),
            learnsets: [
                (
                    species_id.clone(),
                    vec![
                        LearnsetEntry(1, "TACKLE ".to_string()),
                        LearnsetEntry(1, "tackle".to_string()),
                    ],
                ),
                (
                    " CHIKORITA".to_string(),
                    vec![LearnsetEntry(1, "TACKLE".to_string())],
                ),
            ]
            .into_iter()
            .collect(),
            evolutions: EvolutionTable([(species_id.clone(), Vec::new())].into_iter().collect()),
            moves: [
                ("TACKLE".to_string(), test_move("TACKLE")),
                ("HEADBUTT".to_string(), test_move("HEADBUTT")),
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

        for (code, subject) in [
            ("invalid_tmhm_move", species_id.as_str()),
            ("unknown_tmhm_move", species_id.as_str()),
            ("invalid_level_move", species_id.as_str()),
            ("unknown_level_move", species_id.as_str()),
            ("invalid_learnset_species", " CHIKORITA"),
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
    fn verifier_rejects_malformed_trainer_catalog_without_coercion() {
        let data = GameDataSet {
            pokemon: [("RATTATA".to_string(), species())].into_iter().collect(),
            items: [
                ("BERRY".to_string(), test_item("BERRY")),
                ("POTION".to_string(), test_item("POTION")),
            ]
            .into_iter()
            .collect(),
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            trainers: TrainerCatalog {
                trainers: [(
                    "YOUNGSTER_JOEY".to_string(),
                    Trainer {
                        trainer_id: " YOUNGSTER_JOEY".to_string(),
                        trainer_class: "YOUNGSTER ".to_string(),
                        party: vec![
                            crystal_core::models::TrainerPartyPokemon {
                                species: " RATTATA".to_string(),
                                level: 6,
                                item: Some(" BERRY".to_string()),
                                moves: vec![crystal_core::models::LearnedMove {
                                    name: " TACKLE".to_string(),
                                    current_pp: 35,
                                    pp_ups: 0,
                                }],
                                dvs: Dv::default(),
                            },
                            crystal_core::models::TrainerPartyPokemon {
                                species: "rattata".to_string(),
                                level: 6,
                                item: Some("berry".to_string()),
                                moves: vec![crystal_core::models::LearnedMove {
                                    name: "tackle".to_string(),
                                    current_pp: 35,
                                    pp_ups: 0,
                                }],
                                dvs: Dv::default(),
                            },
                        ],
                        items: vec![Some(" POTION".to_string()), Some("potion".to_string())],
                        ..Trainer::default()
                    },
                )]
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

        for (code, subject) in [
            ("trainer_catalog_key_mismatch", "YOUNGSTER_JOEY"),
            ("invalid_trainer_id", " YOUNGSTER_JOEY"),
            ("invalid_trainer_class", " YOUNGSTER_JOEY"),
            ("invalid_trainer_party_species", " YOUNGSTER_JOEY:party:0"),
            ("invalid_trainer_party_item", " YOUNGSTER_JOEY:party:0"),
            ("invalid_trainer_party_move", " YOUNGSTER_JOEY:party:0"),
            ("unknown_trainer_party_species", " YOUNGSTER_JOEY:party:1"),
            ("unknown_trainer_party_item", " YOUNGSTER_JOEY:party:1"),
            ("unknown_trainer_party_move", " YOUNGSTER_JOEY:party:1"),
            ("invalid_trainer_battle_item", " YOUNGSTER_JOEY:item:0"),
            ("unknown_trainer_battle_item", " YOUNGSTER_JOEY:item:1"),
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
    fn verifier_rejects_unknown_mart_items_without_case_coercion_or_pocket_enums() {
        let mut battle_pass = test_item("BATTLE_PASS");
        battle_pass.pocket = item_pocket("BATTLE_PASS");
        let data = GameDataSet {
            items: [
                ("POTION".to_string(), test_item("POTION")),
                ("BATTLE_PASS".to_string(), battle_pass),
            ]
            .into_iter()
            .collect(),
            marts: MartCatalog(
                [
                    (" MartNew".to_string(), vec!["POTION".to_string()]),
                    (
                        "MartNew".to_string(),
                        vec![
                            "RARE CANDY".to_string(),
                            "potion".to_string(),
                            "BATTLE_PASS".to_string(),
                        ],
                    ),
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

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_mart_item"
                && diagnostic.subject == "MartNew"
                && diagnostic.message.contains("RARE CANDY")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_mart_item"
                && diagnostic.subject == "MartNew"
                && diagnostic.message.contains("potion")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsupported_mart_item_pocket"
                || diagnostic.message.contains("unsupported shop pocket")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_mart_id" && diagnostic.subject == " MartNew"
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_shop_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_shop_commands = vec![
            ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "mart_cherrygrove".to_string(),
                source_script: "ClerkScript".to_string(),
                command_index: 1,
            },
            ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "0".to_string(),
                source_script: "ZeroScript".to_string(),
                command_index: 2,
            },
            ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "marttype_standard".to_string(),
                mart_id: "MART_CHERRYGROVE".to_string(),
                source_script: "LowerTypeScript".to_string(),
                command_index: 3,
            },
            ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: " MARTTYPE_STANDARD".to_string(),
                mart_id: "MART_CHERRYGROVE".to_string(),
                source_script: "PaddedTypeScript".to_string(),
                command_index: 4,
            },
            ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: " MART_CHERRYGROVE".to_string(),
                source_script: "PaddedMartScript".to_string(),
                command_index: 5,
            },
            ScriptShopCommand {
                command: "PokeMart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MART_CHERRYGROVE".to_string(),
                source_script: "PaddedCommandScript".to_string(),
                command_index: 6,
            },
            ScriptShopCommand {
                command: "sellmart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MART_CHERRYGROVE".to_string(),
                source_script: "UnknownCommandScript".to_string(),
                command_index: 7,
            },
        ];
        for script in [
            "Route29Potion",
            "HiddenPotion",
            "BadItemToken",
            "FruitTree",
            "BadFruitTree",
            "UppercasePickup",
            "UnknownPickup",
        ] {
            module.scripts.insert(
                script.to_string(),
                Value::Array(vec![serde_json::json!({
                    "command": "end",
                    "args": []
                })]),
            );
        }
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            marts: MartCatalog(
                [("MART_CHERRYGROVE".to_string(), vec!["POTION".to_string()])]
                    .into_iter()
                    .collect(),
            ),
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
            diagnostic.code == "unknown_script_shop_mart"
                && diagnostic.subject == "Start:ClerkScript:1"
                && diagnostic.message.contains("mart_cherrygrove")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "script_shop_invalid_zero_mart"
                && diagnostic.subject == "Start:ZeroScript:2"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_shop_mart_type"
                && diagnostic.subject == "Start:LowerTypeScript:3"
                && diagnostic.message.contains("marttype_standard")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_shop_mart_type"
                && diagnostic.subject == "Start:PaddedTypeScript:4"
                && diagnostic.message.contains(" MARTTYPE_STANDARD")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_shop_mart"
                && diagnostic.subject == "Start:PaddedMartScript:5"
                && diagnostic.message.contains(" MART_CHERRYGROVE")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_shop_command"
                && diagnostic.subject == "Start:PaddedCommandScript:6"
                && diagnostic.message.contains("PokeMart")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_shop_command"
                && diagnostic.subject == "Start:UnknownCommandScript:7"
                && diagnostic.message.contains("sellmart")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_phone_commands_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_phone_commands = vec![
            ScriptPhoneCommand {
                command: "CheckCellNum".to_string(),
                contact_id: "PHONE_MOM".to_string(),
                source_script: "PhoneScript".to_string(),
                command_index: 1,
            },
            ScriptPhoneCommand {
                command: "deletecellnum".to_string(),
                contact_id: "PHONE_MOM".to_string(),
                source_script: "UnknownPhoneScript".to_string(),
                command_index: 2,
            },
            ScriptPhoneCommand {
                command: "checkcellnum".to_string(),
                contact_id: "phone_mom".to_string(),
                source_script: "LowerContactScript".to_string(),
                command_index: 3,
            },
            ScriptPhoneCommand {
                command: "askforphonenumber".to_string(),
                contact_id: String::new(),
                source_script: "EmptyContactScript".to_string(),
                command_index: 4,
            },
            ScriptPhoneCommand {
                command: "askforphonenumber".to_string(),
                contact_id: " PHONE_MOM".to_string(),
                source_script: "PaddedContactScript".to_string(),
                command_index: 5,
            },
        ];
        for source_script in [
            "Route29Potion",
            "HiddenPotion",
            "BadItemToken",
            "FruitTree",
            "BadFruitTree",
            "UppercasePickup",
            "UnknownPickup",
        ] {
            module.scripts.insert(
                source_script.to_string(),
                Value::Array(vec![serde_json::json!({
                    "command": "end",
                    "args": []
                })]),
            );
        }
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            phone_contacts: PhoneContactCatalog(
                [("PHONE_MOM".to_string(), test_phone_contact("PHONE_MOM"))]
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
            diagnostic.code == "invalid_script_phone_command"
                && diagnostic.subject == "Start:PhoneScript:1"
                && diagnostic.message.contains("CheckCellNum")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_phone_command"
                && diagnostic.subject == "Start:UnknownPhoneScript:2"
                && diagnostic.message.contains("deletecellnum")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_phone_contact"
                && diagnostic.subject == "Start:LowerContactScript:3"
                && diagnostic.message.contains("phone_mom")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_phone_contact"
                && diagnostic.subject == "Start:EmptyContactScript:4"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_phone_contact"
                && diagnostic.subject == "Start:PaddedContactScript:5"
                && diagnostic.message.contains(" PHONE_MOM")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_item_grants_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_item_grants = vec![
            ScriptItemGrant {
                command: "verbosegiveitem".to_string(),
                item_id: "potion".to_string(),
                quantity: 1,
                source_script: "GiftScript".to_string(),
                command_index: 4,
                verbose: true,
            },
            ScriptItemGrant {
                command: "verbosegiveitem".to_string(),
                item_id: " POTION".to_string(),
                quantity: 1,
                source_script: "PaddedGiftScript".to_string(),
                command_index: 5,
                verbose: true,
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
            diagnostic.code == "unknown_script_item_grant_item"
                && diagnostic.subject == "Start:GiftScript:4"
                && diagnostic.message.contains("potion")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_item_grant_item"
                && diagnostic.subject == "Start:PaddedGiftScript:5"
                && diagnostic.message.contains(" POTION")
        }));
    }

    #[test]
    fn verifier_rejects_unknown_script_item_access_without_case_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_item_checks = vec![ScriptItemAccess {
            command: "checkitem".to_string(),
            item_id: "pass".to_string(),
            source_script: "GateScript".to_string(),
            command_index: 3,
        }];
        module.script_item_takes = vec![
            ScriptItemAccess {
                command: "takeitem".to_string(),
                item_id: "lost_item".to_string(),
                source_script: "CopycatScript".to_string(),
                command_index: 8,
            },
            ScriptItemAccess {
                command: "takeitem".to_string(),
                item_id: " LOST_ITEM".to_string(),
                source_script: "PaddedCopycatScript".to_string(),
                command_index: 9,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            items: [
                ("PASS".to_string(), test_item("PASS")),
                ("LOST_ITEM".to_string(), test_item("LOST_ITEM")),
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
            diagnostic.code == "unknown_script_item_access_item"
                && diagnostic.subject == "Start:GateScript:3"
                && diagnostic.message.contains("pass")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_item_access_item"
                && diagnostic.subject == "Start:CopycatScript:8"
                && diagnostic.message.contains("lost_item")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_item_access_item"
                && diagnostic.subject == "Start:PaddedCopycatScript:9"
                && diagnostic.message.contains(" LOST_ITEM")
        }));
    }

    #[test]
    fn verifier_rejects_malformed_script_field_pickups_without_coercion() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_field_pickups = vec![
            ScriptFieldPickup {
                command: "itemball".to_string(),
                item_id: Some("potion".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "Route29Potion".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 0,
                event_flag: Some("-1".to_string()),
                fruit_tree_id: None,
                source_script: "HiddenPotion".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "itemball".to_string(),
                item_id: Some("RARE CANDY".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_RARE_CANDY".to_string()),
                fruit_tree_id: None,
                source_script: "BadItemToken".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some(String::new()),
                source_script: "FruitTree".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some(" FRUITTREE_ROUTE_29".to_string()),
                source_script: "BadFruitTree".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "ITEMBALL".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "UppercasePickup".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "giveitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "UnknownPickup".to_string(),
                command_index: 0,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            items: [("POTION".to_string(), test_item("POTION"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        for expected in [
            "unknown_script_field_pickup_item",
            "invalid_script_field_pickup_item",
            "script_field_pickup_invalid_quantity",
            "script_field_pickup_uncollectible_event",
            "script_field_pickup_empty_fruit_tree",
            "script_field_pickup_invalid_fruit_tree",
            "invalid_script_field_pickup_command",
            "unknown_script_field_pickup_command",
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
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_script_field_pickup_command"
                && diagnostic.subject == "Start:UppercasePickup:0"
                && diagnostic.message.contains("ITEMBALL")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_script_field_pickup_command"
                && diagnostic.subject == "Start:UnknownPickup:0"
                && diagnostic.message.contains("giveitem")
        }));
    }

    #[test]
    fn verifier_rejects_hidden_item_bg_events_without_exact_pickup() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.events.bg_events = vec![
            BackgroundEvent {
                x: 4,
                y: 4,
                event_type: "BGEVENT_ITEM".to_string(),
                script: "MissingHiddenPotion".to_string(),
            },
            BackgroundEvent {
                x: 5,
                y: 4,
                event_type: "BGEVENT_ITEM".to_string(),
                script: "DuplicateHiddenPotion".to_string(),
            },
        ];
        module.script_field_pickups = vec![
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_DUPLICATE_HIDDEN_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "DuplicateHiddenPotion".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_DUPLICATE_HIDDEN_POTION_2".to_string()),
                fruit_tree_id: None,
                source_script: "DuplicateHiddenPotion".to_string(),
                command_index: 1,
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
            diagnostic.code == "hidden_item_bg_event_missing_pickup"
                && diagnostic.subject == "Start:MissingHiddenPotion:4,4"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "hidden_item_bg_event_duplicate_pickup"
                && diagnostic.subject == "Start:DuplicateHiddenPotion:5,4"
        }));
    }

    #[test]
    fn verifier_reports_duplicate_warps_by_runtime_tile_coordinates() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.events.warps = vec![
            WarpEvent {
                index: 1,
                x: 2,
                y: 3,
                target_map_constant: "START_MAP".to_string(),
                target_map: "START_MAP".to_string(),
                target_warp_id: 1,
            },
            WarpEvent {
                index: 2,
                x: 2,
                y: 3,
                target_map_constant: "START_MAP".to_string(),
                target_map: "START_MAP".to_string(),
                target_warp_id: 1,
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            tilesets: [("johto".to_string(), test_tileset_definition())]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_warp_tile"
                && diagnostic.subject == "Start"
                && diagnostic.message.contains("runtime tile 2,3")
        }));
    }

    #[test]
    fn verifier_rejects_itemball_objects_without_exact_pickups() {
        let mut module = test_map_module("Start", "START_MAP", None);
        let mut item_object = test_object("START_POKE_BALL", "EVENT_START_POTION", 1, 1);
        item_object.object_type = "OBJECTTYPE_ITEMBALL".to_string();
        item_object.script = "StartPotion".to_string();
        module.objects = vec![item_object];
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "hiddenitem".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 1,
            event_flag: Some("EVENT_START_HIDDEN_POTION".to_string()),
            fruit_tree_id: None,
            source_script: "StartPotion".to_string(),
            command_index: 0,
        }];
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
            diagnostic.code == "itemball_object_missing_pickup"
                && diagnostic.subject == "Start:START_POKE_BALL"
                && diagnostic.message.contains("StartPotion")
        }));
    }

    #[test]
    fn verifier_rejects_itemball_pickups_without_exact_objects() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "itemball".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 1,
            event_flag: Some("EVENT_START_POTION".to_string()),
            fruit_tree_id: None,
            source_script: "StartPotion".to_string(),
            command_index: 0,
        }];
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
            diagnostic.code == "itemball_pickup_missing_object"
                && diagnostic.subject == "Start:script_field_pickup:StartPotion:0"
                && diagnostic.message.contains("StartPotion")
        }));
    }

    #[test]
    fn verifier_rejects_duplicate_itemball_object_scripts() {
        let mut module = test_map_module("Start", "START_MAP", None);
        let mut first_object = test_object("START_POKE_BALL_1", "EVENT_START_POTION_1", 1, 1);
        first_object.object_type = "OBJECTTYPE_ITEMBALL".to_string();
        first_object.script = "StartPotion".to_string();
        let mut second_object = test_object("START_POKE_BALL_2", "EVENT_START_POTION_2", 2, 1);
        second_object.object_type = "OBJECTTYPE_ITEMBALL".to_string();
        second_object.script = "StartPotion".to_string();
        module.objects = vec![first_object, second_object];
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "itemball".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 1,
            event_flag: Some("EVENT_START_POTION_1".to_string()),
            fruit_tree_id: None,
            source_script: "StartPotion".to_string(),
            command_index: 0,
        }];
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
            diagnostic.code == "itemball_duplicate_object_script"
                && diagnostic.subject == "Start:StartPotion"
                && diagnostic.message.contains("2 OBJECTTYPE_ITEMBALL objects")
        }));
    }

    #[test]
    fn verifier_rejects_map_events_that_reference_missing_scripts() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripts.insert(
            "KnownSignScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.events.coord_events = vec![
            CoordEvent {
                x: 2,
                y: 3,
                scene_id: "SCENE_START".to_string(),
                script_name: "knownsignscript".to_string(),
            },
            CoordEvent {
                x: 8,
                y: 9,
                scene_id: "SCENE_START".to_string(),
                script_name: "Known Sign Script".to_string(),
            },
            CoordEvent {
                x: 12,
                y: 13,
                scene_id: "".to_string(),
                script_name: "KnownSignScript".to_string(),
            },
            CoordEvent {
                x: 12,
                y: 13,
                scene_id: "SCENE_START".to_string(),
                script_name: "KnownSignScript".to_string(),
            },
        ];
        module.events.bg_events = vec![
            BackgroundEvent {
                x: 4,
                y: 5,
                event_type: "BGEVENT_READ".to_string(),
                script: "KnownSignScript".to_string(),
            },
            BackgroundEvent {
                x: 6,
                y: 7,
                event_type: "BGEVENT_READ".to_string(),
                script: "MissingSignScript".to_string(),
            },
            BackgroundEvent {
                x: 10,
                y: 11,
                event_type: "BGEVENT_READ".to_string(),
                script: "Missing Sign Script".to_string(),
            },
            BackgroundEvent {
                x: 12,
                y: 13,
                event_type: "BGEVENT_READ".to_string(),
                script: "KnownSignScript".to_string(),
            },
            BackgroundEvent {
                x: 12,
                y: 13,
                event_type: "BGEVENT_READ".to_string(),
                script: "KnownSignScript".to_string(),
            },
        ];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_coord_event_script"
                && diagnostic.subject == "Start:SCENE_START:knownsignscript:2,3"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_bg_event_script"
                && diagnostic.subject == "Start:BGEVENT_READ:MissingSignScript:6,7"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_coord_event_script"
                && diagnostic.subject == "Start:SCENE_START:Known Sign Script:8,9"
                && diagnostic.message.contains("Known Sign Script")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_bg_event_script"
                && diagnostic.subject == "Start:BGEVENT_READ:Missing Sign Script:10,11"
                && diagnostic.message.contains("Missing Sign Script")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_coord_event_position"
                && diagnostic.subject == "Start:12,13"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_bg_event_position" && diagnostic.subject == "Start:12,13"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_coord_event_script"
                && diagnostic.subject == "Start:SCENE_START:Known Sign Script:8,9"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_bg_event_script"
                && diagnostic.subject == "Start:BGEVENT_READ:Missing Sign Script:10,11"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_bg_event_script"
                && diagnostic.subject == "Start:BGEVENT_READ:KnownSignScript:4,5"
        }));
    }

    #[test]
    fn verifier_rejects_map_events_outside_runtime_tile_bounds() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1; 4];
        module.scripts.insert(
            "KnownScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.events.warps = vec![WarpEvent {
            index: 1,
            x: 4,
            y: 0,
            target_map_constant: "NEXT_MAP".to_string(),
            target_map: "NextMap".to_string(),
            target_warp_id: 1,
        }];
        module.events.coord_events = vec![CoordEvent {
            x: 0,
            y: 4,
            scene_id: "SCENE_START".to_string(),
            script_name: "KnownScript".to_string(),
        }];
        module.events.bg_events = vec![BackgroundEvent {
            x: 4,
            y: 4,
            event_type: "BGEVENT_READ".to_string(),
            script: "KnownScript".to_string(),
        }];
        module.objects = vec![test_object("START_NPC", "-1", 1, 4)];
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

        for subject in [
            "Start:warp:1:4,0",
            "Start:coord:SCENE_START:KnownScript:0,4",
            "Start:bg:BGEVENT_READ:KnownScript:4,4",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "map_event_runtime_position_out_of_bounds"
                        && diagnostic.subject == subject
                        && diagnostic.message.contains("outside map bounds 4x4")
                }),
                "missing out-of-bounds runtime position diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_accepts_raw_map_events_that_fit_exact_runtime_bounds() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![0; 4];
        module.scripts.insert(
            "KnownScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.events.warps = vec![WarpEvent {
            index: 1,
            x: 3,
            y: 3,
            target_map_constant: "START_MAP".to_string(),
            target_map: "Start".to_string(),
            target_warp_id: 1,
        }];
        module.events.coord_events = vec![CoordEvent {
            x: 3,
            y: 1,
            scene_id: "SCENE_START".to_string(),
            script_name: "KnownScript".to_string(),
        }];
        module.events.bg_events = vec![BackgroundEvent {
            x: 1,
            y: 3,
            event_type: "BGEVENT_READ".to_string(),
            script: "KnownScript".to_string(),
        }];
        module.objects = vec![test_object("START_NPC", "-1", 1, 1)];
        let data = GameDataSet {
            maps: [("Start".to_string(), module)].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for subject in [
            "Start:warp:1:3,3",
            "Start:coord:SCENE_START:KnownScript:3,1",
            "Start:bg:BGEVENT_READ:KnownScript:1,3",
        ] {
            assert!(
                !report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "map_event_runtime_position_out_of_bounds"
                        && diagnostic.subject == subject
                        && diagnostic.message.contains("outside map bounds 4x4")
                }),
                "exact in-bounds runtime event should not be rejected for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_reports_overflowing_map_event_tiles_without_panicking_in_playability() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1; 4];
        module.scripts.insert(
            "KnownScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.events.warps = vec![WarpEvent {
            index: 1,
            x: 40_000,
            y: 0,
            target_map_constant: "START_MAP".to_string(),
            target_map: "START_MAP".to_string(),
            target_warp_id: 1,
        }];
        module.events.coord_events = vec![CoordEvent {
            x: 0,
            y: 40_000,
            scene_id: "SCENE_START".to_string(),
            script_name: "KnownScript".to_string(),
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

        for subject in [
            "Start:warp:1:40000,0",
            "Start:coord:SCENE_START:KnownScript:0,40000",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "map_event_runtime_position_overflow"
                        && diagnostic.subject == subject
                }),
                "missing overflow runtime position diagnostic for {subject}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_allows_warp_events_on_special_collision_runtime_tiles() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 1;
        module.blocks = vec![1, 0];
        module.events.warps = vec![WarpEvent {
            index: 1,
            x: 0,
            y: 0,
            target_map_constant: "NEXT_MAP".to_string(),
            target_map: "NextMap".to_string(),
            target_warp_id: 1,
        }];
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
                .any(|diagnostic| diagnostic.code == "unwalkable_warp_tile")
        );
    }

    #[test]
    fn verifier_allows_coord_events_on_special_collision_runtime_tiles() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.attributes.width = 2;
        module.attributes.height = 1;
        module.blocks = vec![1, 0];
        module.scripts.insert(
            "ObjectScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.scripts.insert(
            "KnownScript".to_string(),
            Value::Array(vec![serde_json::json!({
                "command": "end",
                "args": []
            })]),
        );
        module.events.coord_events = vec![CoordEvent {
            x: 0,
            y: 0,
            scene_id: "SCENE_START".to_string(),
            script_name: "KnownScript".to_string(),
        }];
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
                .any(|diagnostic| diagnostic.code == "unwalkable_coord_event_tile")
        );
    }
