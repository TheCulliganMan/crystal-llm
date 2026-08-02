    #[test]
    fn verifier_rejects_invalid_runtime_map_metadata_without_coercion() {
        let data = GameDataSet {
            maps: [
                (
                    "Route29".to_string(),
                    test_map_module("Route29", "ROUTE_29", None),
                ),
                (
                    "NewBarkTown".to_string(),
                    test_map_module("NewBarkTown", "NEW_BARK_TOWN", None),
                ),
            ]
            .into_iter()
            .collect(),
            runtime_map_metadata: [
                (
                    "NEW_BARK_TOWN".to_string(),
                    RuntimeMapMetadata {
                        constant: "NEW_BARK_TOWN".to_string(),
                        name: "WrongName".to_string(),
                        group_name: "GROUP_NEW_BARK".to_string(),
                        group_id: 1,
                        map_id: 1,
                        width: 10,
                        height: 9,
                        environment: "TOWN".to_string(),
                        phone_service: 1,
                    },
                ),
                (
                    "ROUTE_29".to_string(),
                    RuntimeMapMetadata {
                        constant: "ROUTE_29_ALIAS".to_string(),
                        name: "Route29".to_string(),
                        group_name: String::new(),
                        group_id: 1,
                        map_id: 2,
                        width: 10,
                        height: 9,
                        environment: "ROUTE".to_string(),
                        phone_service: 1,
                    },
                ),
                (
                    " ROUTE_30".to_string(),
                    RuntimeMapMetadata {
                        constant: " ROUTE_30".to_string(),
                        name: " Route30".to_string(),
                        group_name: "GROUP_ROUTE_30".to_string(),
                        group_id: 1,
                        map_id: 3,
                        width: 10,
                        height: 9,
                        environment: "ROUTE".to_string(),
                        phone_service: 1,
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
            diagnostic.code == "runtime_map_metadata_name_mismatch"
                && diagnostic.subject == "NEW_BARK_TOWN"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "runtime_map_metadata_constant_mismatch"
                && diagnostic.subject == "ROUTE_29"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_runtime_map_metadata_constant"
                && diagnostic.subject == "ROUTE_29"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_runtime_map_metadata" && diagnostic.subject == "ROUTE_29"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_runtime_map_metadata" && diagnostic.subject == " ROUTE_30"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_runtime_map_metadata_constant"
                && diagnostic.subject == " ROUTE_30"
        }));
    }

    #[test]
    fn verifier_requires_species_display_records_from_pack() {
        let species_id = species().id;
        let data = GameDataSet {
            pokemon: [(species_id.clone(), species())].into_iter().collect(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for expected in [
            "missing_species_menu_icon",
            "missing_species_pokedex_entry",
            "missing_species_frontpic_anim",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == expected && diagnostic.subject == species_id
                }),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_invalid_battle_animation_metadata_without_coercion() {
        let data = GameDataSet {
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            battle_animations: [
                (
                    " BattleAnim_Padded".to_string(),
                    vec!["anim_wait 1".to_string()],
                ),
                ("BattleAnim_Tackle".to_string(), Vec::new()),
            ]
            .into_iter()
            .collect(),
            battle_animation_table: vec![
                "BattleAnim_Dummy".to_string(),
                String::new(),
                "BattleAnim_Tackle ".to_string(),
                "BattleAnim_Missing".to_string(),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_animation"
                && diagnostic.subject == " BattleAnim_Padded"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_animation"
                && diagnostic.subject == "BattleAnim_Tackle"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_animation_table_entry" && diagnostic.subject == "1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_battle_animation_table_entry" && diagnostic.subject == "2"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == VerificationSeverity::Error
                && diagnostic.code == "unknown_battle_animation_table_entry"
                && diagnostic.subject == "3"
                && diagnostic.message.contains("BattleAnim_Missing")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "battle_animation_table_count_mismatch"
                && diagnostic.subject == "battle_animation_table"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_runtime_bundles_without_coercion() {
        let data = GameDataSet {
            battle_anim_bundle: "[".to_string(),
            sprite_anim_bundle: r#"{"oam_sets":{}}"#.to_string(),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_runtime_bundle"
                && diagnostic.subject == "battle_anim_bundle"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_runtime_bundle_section"
                && diagnostic.subject == "sprite_anim_bundle"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_move_name_metadata_without_coercion() {
        let data = GameDataSet {
            moves: [("POUND".to_string(), test_move("POUND"))]
                .into_iter()
                .collect(),
            move_names: vec![
                "POUND".to_string(),
                String::new(),
                "KARATE CHOP ".to_string(),
            ],
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "move_names_count_mismatch" && diagnostic.subject == "move_names"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_move_name" && diagnostic.subject == "1"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_move_name" && diagnostic.subject == "2"
        }));
    }

    #[test]
    fn verifier_rejects_move_payload_ids_without_effect_enum_restriction() {
        let mut move_data = test_move("AETHER_PULSE");
        move_data.name = " AETHER_PULSE".to_string();
        move_data.move_type = pokemon_type("AETHER ");
        move_data.effect = " MODDED_EFFECT".to_string();
        let data = GameDataSet {
            moves: [("AETHER_PULSE".to_string(), move_data)]
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
            "invalid_move_name",
            "invalid_move_type",
            "invalid_move_effect",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == expected && diagnostic.subject == "AETHER_PULSE"
                }),
                "missing diagnostic {expected}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_rejects_exact_move_effect_without_rust_runtime_mutation() {
        let mut move_data = test_move("AETHER_PULSE");
        move_data.move_type = pokemon_type("AETHER");
        move_data.effect = "MODDED_EFFECT".to_string();
        let data = GameDataSet {
            moves: [("AETHER_PULSE".to_string(), move_data)]
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
            diagnostic.code == "unsupported_battle_move_effect"
                && diagnostic.subject == "AETHER_PULSE"
                && diagnostic.message.contains("MODDED_EFFECT")
        }));
    }

    #[test]
    fn verifier_rejects_invalid_asm_text_without_coercion() {
        let data = GameDataSet {
            asm_text: [("GreetingText".to_string(), String::new())]
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
            diagnostic.code == "invalid_asm_text" && diagnostic.subject == "GreetingText"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_display_metadata_without_coercion() {
        let data = GameDataSet {
            sprite_palette_defaults: [
                (" SPRITE_CHRIS".to_string(), 0),
                ("SPRITE_CHRIS".to_string(), -1),
            ]
            .into_iter()
            .collect(),
            pokegear_town_map_palette_map: [
                (" NEW_BARK_TOWN".to_string(), vec!["PAL_GREEN".to_string()]),
                (
                    "NEW_BARK_TOWN".to_string(),
                    vec!["PAL_GREEN".to_string(), String::new()],
                ),
                ("ROUTE_29".to_string(), vec![" PAL_GREEN".to_string()]),
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
            diagnostic.code == "invalid_sprite_palette_default"
                && diagnostic.subject == "SPRITE_CHRIS"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_sprite_palette_default"
                && diagnostic.subject == " SPRITE_CHRIS"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_palette_map"
                && diagnostic.subject == "NEW_BARK_TOWN"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_palette_map"
                && diagnostic.subject == " NEW_BARK_TOWN"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_palette_map" && diagnostic.subject == "ROUTE_29"
        }));
    }

    #[test]
    fn verifier_rejects_pokegear_landmarks_for_unknown_maps_or_constants() {
        let data = GameDataSet {
            maps: [
                (
                    "Route29".to_string(),
                    test_map_module("Route29", "ROUTE_29", None),
                ),
                (
                    "Route31".to_string(),
                    test_map_module("Route31", "ROUTE_31", None),
                ),
            ]
            .into_iter()
            .collect(),
            pokegear_landmarks: PokegearLandmarksPayload {
                landmarks: vec![
                    PokegearLandmark {
                        id: 1,
                        constant: "LANDMARK_ROUTE_29".to_string(),
                        label: "ROUTE_29".to_string(),
                        name: "Route 29".to_string(),
                        x: 2,
                        y: 3,
                        region: "johto".to_string(),
                    },
                    PokegearLandmark {
                        id: 2,
                        constant: " route_30".to_string(),
                        label: " ROUTE_30".to_string(),
                        name: "Route 30".to_string(),
                        x: 4,
                        y: 5,
                        region: "johto".to_string(),
                    },
                    PokegearLandmark {
                        id: 3,
                        constant: "ROUTE_30".to_string(),
                        label: "ROUTE_30".to_string(),
                        name: "Route 30".to_string(),
                        x: 4,
                        y: 5,
                        region: "johto".to_string(),
                    },
                    PokegearLandmark {
                        id: 4,
                        constant: "LANDMARK_ROUTE_29".to_string(),
                        label: "ROUTE_29_DUPLICATE".to_string(),
                        name: "Route 29 Duplicate".to_string(),
                        x: 6,
                        y: 7,
                        region: "johto".to_string(),
                    },
                ],
                map_to_landmark: [
                    ("Route29".to_string(), "LANDMARK_ROUTE_30".to_string()),
                    ("MissingRoute".to_string(), "LANDMARK_ROUTE_29".to_string()),
                    (" Route29".to_string(), "LANDMARK_ROUTE_29".to_string()),
                    ("Route30".to_string(), " LANDMARK_ROUTE_29".to_string()),
                    ("Route31".to_string(), "ROUTE_30".to_string()),
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
            diagnostic.code == "invalid_pokegear_landmark" && diagnostic.subject == " route_30"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_landmark_constant"
                && diagnostic.subject == "ROUTE_30"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "duplicate_pokegear_landmark_constant"
                && diagnostic.subject == "LANDMARK_ROUTE_29"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_landmark_map" && diagnostic.subject == " Route29"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_landmark_reference"
                && diagnostic.subject == "Route30"
                && diagnostic.message.contains(" LANDMARK_ROUTE_29")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_constant"
                && diagnostic.subject == "Route29"
                && diagnostic.message.contains("LANDMARK_ROUTE_30")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_map"
                && diagnostic.subject == "MissingRoute"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_map" && diagnostic.subject == " Route29"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_constant"
                && diagnostic.subject == "Route30"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_pokegear_landmark_reference"
                && diagnostic.subject == "Route31"
                && diagnostic.message.contains("ROUTE_30")
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_pokegear_landmark_constant"
                && diagnostic.subject == "Route31"
        }));
    }

    #[test]
    fn verifier_accepts_exact_frontpic_animation_asset_keys_only() {
        let valid_program = FrontpicAnimProgram {
            commands: vec![FrontpicAnimCommand {
                kind: "endanim".to_string(),
                ..FrontpicAnimCommand::default()
            }],
        };
        let data = GameDataSet {
            pokemon: [(species().id.clone(), species())].into_iter().collect(),
            pokemon_frontpic_anim: [
                (" UNOWN_A".to_string(), valid_program.clone()),
                ("EGG".to_string(), valid_program.clone()),
                ("UNOWN_A".to_string(), valid_program.clone()),
                ("unown_a".to_string(), valid_program.clone()),
                ("UNOWN_AA".to_string(), valid_program.clone()),
                ("UNOWN_1".to_string(), valid_program),
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
        let invalid_frontpic_subjects: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "unknown_frontpic_anim_species")
            .map(|diagnostic| diagnostic.subject.as_str())
            .collect();
        let malformed_frontpic_subjects: BTreeSet<&str> = report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "invalid_frontpic_anim_species")
            .map(|diagnostic| diagnostic.subject.as_str())
            .collect();

        assert_eq!(
            invalid_frontpic_subjects,
            BTreeSet::from(["UNOWN_1", "UNOWN_AA", "unown_a"])
        );
        assert_eq!(malformed_frontpic_subjects, BTreeSet::from([" UNOWN_A"]));
    }

    #[test]
    fn entity_content_payloads_reject_object_map_fallback_shape() {
        let mut single = species();
        single.id = "SINGLE_MON".to_string();
        let single_payload = serde_json::to_value(&single).expect("serialize single species");
        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(ContentPackCategory::Pokemon, single_payload)
            .expect_err("Pokemon payloads must use definitive keyed object maps");
        assert!(
            format!("{error:#}").contains("parse object-map payload"),
            "{error:#}"
        );

        let mut array_entry = species();
        array_entry.id = "ARRAY_MON".to_string();
        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Pokemon,
                serde_json::json!([array_entry]),
            )
            .expect_err("Pokemon payloads must not use legacy arrays");
        assert!(
            format!("{error:#}").contains("parse object-map payload"),
            "{error:#}"
        );

        let mut mapped = species();
        mapped.id = "MAPPED_MON".to_string();
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Pokemon,
            serde_json::json!({ "MAPPED_MON": mapped }),
        )
        .expect("Pokemon category object maps are definitive");
        assert!(data.pokemon.contains_key("MAPPED_MON"));
    }

    #[test]
    fn move_content_payloads_reject_malformed_ids_without_effect_enum_restriction() {
        let mut move_data = test_move("AETHER_PULSE");
        move_data.effect = " MODDED_EFFECT".to_string();
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Moves,
                serde_json::json!({ "AETHER_PULSE": move_data }),
            )
            .expect_err("move category payload rejects malformed ids")
            .to_string();

        assert!(
            error.contains(
                "move token must be exact ASCII alphanumeric/underscore, found \" MODDED_EFFECT\""
            ),
            "{error}"
        );
        assert!(data.moves.is_empty());

        let move_data = test_move("fallback_move");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Moves,
                serde_json::json!({ "fallback_move": move_data }),
            )
            .expect_err("reserved move ids must fail at content-pack load time")
            .to_string();
        assert!(
            error.contains("move token 'fallback_move' uses reserved modpack payload prefix"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_evolutions_as_typed_definitive_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::Evolutions,
            serde_json::json!({
                "PIKACHU": {
                    "species": "PIKACHU",
                    "evolutions": [{
                        "method": "ITEM",
                        "species": "RAICHU",
                        "level": null,
                        "item": "THUNDERSTONE",
                        "held_item": null,
                        "happiness": null,
                        "stat_ratio": null
                    }]
                },
                "EEVEE": {
                    "species": "EEVEE",
                    "evolutions": [{
                        "method": "HAPPINESS",
                        "species": "ESPEON",
                        "level": null,
                        "item": null,
                        "held_item": null,
                        "happiness": "TR_MORNDAY",
                        "stat_ratio": null
                    }]
                }
            }),
        )
        .expect("apply evolution payload");

        assert_eq!(
            data.evolutions
                .entries_for("PIKACHU")
                .expect("PIKACHU evolutions")[0]
                .item
                .as_deref(),
            Some("THUNDERSTONE")
        );
        assert_eq!(
            data.evolutions
                .entries_for("EEVEE")
                .expect("EEVEE evolutions")[0]
                .species,
            "ESPEON"
        );
    }

    #[test]
    fn learnset_and_evolution_payloads_reject_object_map_fallback_shape() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Learnsets,
            serde_json::json!({
                "NEW_MON": {
                    "species": "NEW_MON",
                    "learnset": []
                }
            }),
        )
        .expect("species-keyed learnset payload is canonical");
        assert_eq!(data.learnsets["NEW_MON"], Vec::<LearnsetEntry>::new());

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "species": "NEW_MON",
                    "learnset": []
                }),
            )
            .expect_err("learnsets must not use single-entry compatibility shape")
            .to_string();
        assert!(error.contains("parse learnset entry"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "OTHER_MON",
                        "learnset": []
                    }
                }),
            )
            .expect_err("learnset key must match record species")
            .to_string();
        assert!(
            error.contains("learnset key 'NEW_MON' does not match record species 'OTHER_MON'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Evolutions,
            serde_json::json!({
                "NEW_MON": {
                    "species": "NEW_MON",
                    "evolutions": []
                }
            }),
        )
        .expect("species-keyed evolution entry is canonical");
        assert!(
            data.evolutions
                .entries_for("NEW_MON")
                .expect("NEW_MON evolutions")
                .is_empty()
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!({
                    "species": "NEW_MON",
                    "evolutions": []
                }),
            )
            .expect_err("evolutions must not use single-entry compatibility shape")
            .to_string();
        assert!(error.contains("parse evolution entry"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "OTHER_MON",
                        "evolutions": []
                    }
                }),
            )
            .expect_err("evolution key must match record species")
            .to_string();
        assert!(
            error.contains("evolution key 'NEW_MON' does not match record species 'OTHER_MON'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!([
                    {
                        "NEW_MON": {
                            "species": "NEW_MON",
                            "learnset": []
                        }
                    }
                ]),
            )
            .expect_err("learnsets must not use array compatibility shape")
            .to_string();
        assert!(
            error.contains("learnset payload must be a species-keyed object"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Learnsets,
            serde_json::json!({
                "NEW_MON": {
                    "species": "NEW_MON",
                    "learnset": []
                }
            }),
        )
        .expect("apply first learnset payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "learnset": []
                    }
                }),
            )
            .expect_err("duplicate learnset payload must not overwrite")
            .to_string();
        assert!(
            error.contains("duplicate learnset for species 'NEW_MON'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "NEW_MON ": {
                        "species": "NEW_MON ",
                        "learnset": []
                    }
                }),
            )
            .expect_err("learnset species keys must be exact")
            .to_string();
        assert!(
            error.contains(
                "learnset species 'NEW_MON ' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "fallback_species": {
                        "species": "fallback_species",
                        "learnset": []
                    }
                }),
            )
            .expect_err("learnset reserved species keys must fail at load time")
            .to_string();
        assert!(
            error.contains(
                "learnset species 'fallback_species' uses reserved modpack payload prefix"
            ),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "learnset": [[5, "TACKLE HIT"]]
                    }
                }),
            )
            .expect_err("learnset move ids must be exact tokens")
            .to_string();
        assert!(error.contains("parse learnset entry"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Learnsets,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "learnset": [[5, "legacyTackle"]]
                    }
                }),
            )
            .expect_err("learnset reserved move ids must fail at load time")
            .to_string();
        assert!(error.contains("parse learnset entry"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!([
                    {
                        "NEW_MON": {
                            "species": "NEW_MON",
                            "evolutions": []
                        }
                    }
                ]),
            )
            .expect_err("evolutions must not use array compatibility shape")
            .to_string();
        assert!(
            error.contains("evolution payload must be a species-keyed object"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::Evolutions,
            serde_json::json!({
                "NEW_MON": {
                    "species": "NEW_MON",
                    "evolutions": []
                }
            }),
        )
        .expect("apply first evolution payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": []
                    }
                }),
            )
            .expect_err("duplicate evolution payload must not overwrite")
            .to_string();
        assert!(
            error.contains("duplicate evolutions for species 'NEW_MON'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!({
                    " NEW_MON": {
                        "species": " NEW_MON",
                        "evolutions": []
                    }
                }),
            )
            .expect_err("evolution species keys must be exact")
            .to_string();
        assert!(
            error.contains(
                "evolution species ' NEW_MON' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Evolutions,
                serde_json::json!({
                    "legacySpecies": {
                        "species": "legacySpecies",
                        "evolutions": []
                    }
                }),
            )
            .expect_err("evolution reserved species keys must fail at load time")
            .to_string();
        assert!(
            error
                .contains("evolution species 'legacySpecies' uses reserved modpack payload prefix"),
            "{error}"
        );

        let malformed_evolution_cases = vec![
            (
                "target species",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "LEVEL",
                            "species": "NEW MON_2",
                            "level": 16,
                            "item": null,
                            "held_item": null,
                            "happiness": null,
                            "stat_ratio": null
                        }]
                    }
                }),
                "evolution entry 0 target species for 'NEW_MON' 'NEW MON_2' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "reserved target species",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "LEVEL",
                            "species": "fallbackTarget",
                            "level": 16,
                            "item": null,
                            "held_item": null,
                            "happiness": null,
                            "stat_ratio": null
                        }]
                    }
                }),
                "evolution entry 0 target species for 'NEW_MON' 'fallbackTarget' uses reserved modpack payload prefix",
            ),
            (
                "method",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "LEVEL UP",
                            "species": "NEW_MON_2",
                            "level": 16,
                            "item": null,
                            "held_item": null,
                            "happiness": null,
                            "stat_ratio": null
                        }]
                    }
                }),
                "evolution entry 0 method for 'NEW_MON' 'LEVEL UP' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "item",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "ITEM",
                            "species": "NEW_MON_2",
                            "level": null,
                            "item": "MOON STONE",
                            "held_item": null,
                            "happiness": null,
                            "stat_ratio": null
                        }]
                    }
                }),
                "evolution entry 0 item for 'NEW_MON' 'MOON STONE' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "reserved item",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "ITEM",
                            "species": "NEW_MON_2",
                            "level": null,
                            "item": "legacyStone",
                            "held_item": null,
                            "happiness": null,
                            "stat_ratio": null
                        }]
                    }
                }),
                "evolution entry 0 item for 'NEW_MON' 'legacyStone' uses reserved modpack payload prefix",
            ),
            (
                "happiness window",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "HAPPINESS",
                            "species": "NEW_MON_2",
                            "level": null,
                            "item": null,
                            "held_item": null,
                            "happiness": "TR MORNDAY",
                            "stat_ratio": null
                        }]
                    }
                }),
                "evolution entry 0 happiness window for 'NEW_MON' 'TR MORNDAY' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "stat ratio",
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "evolutions": [{
                            "method": "STAT",
                            "species": "NEW_MON_2",
                            "level": 20,
                            "item": null,
                            "held_item": null,
                            "happiness": null,
                            "stat_ratio": "ATK GT DEF"
                        }]
                    }
                }),
                "evolution entry 0 stat ratio for 'NEW_MON' 'ATK GT DEF' must be exact ASCII alphanumeric or underscore",
            ),
        ];

        for (label, payload, expected) in malformed_evolution_cases {
            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::Evolutions, payload)
                .expect_err(label)
                .to_string();
            assert!(error.contains(expected), "{label} produced {error}");
        }
    }

    #[test]
    fn species_move_payloads_require_explicit_species_and_moves_fields() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::LevelUpMoves,
            serde_json::json!({
                "NEW_MON": {
                    "species": "NEW_MON",
                    "moves": [
                        {
                            "level": 1,
                            "move": "TACKLE"
                        }
                    ]
                }
            }),
        )
        .expect("species-keyed level-up move entry is canonical");
        assert_eq!(
            data.level_up_moves["NEW_MON"],
            serde_json::json!([{
                "level": 1,
                "move": "TACKLE"
            }])
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EggMoves,
                serde_json::json!({
                    "NEW_MON": ["CHARM"]
                }),
            )
            .expect_err("egg moves must use species-keyed entry objects")
            .to_string();
        assert!(error.contains("parse species value payload"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EggMoves,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "OTHER_MON",
                        "moves": ["CHARM"]
                    }
                }),
            )
            .expect_err("egg move key must match record species")
            .to_string();
        assert!(
            error.contains("species value key 'NEW_MON' does not match record species 'OTHER_MON'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON"
                    }
                }),
            )
            .expect_err("species move payloads must declare moves explicitly");
        let error = format!("{error:#}");
        assert!(error.contains("missing field `moves`"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "moves": [{
                            "level": 1,
                            "move": " TACKLE"
                        }]
                    }
                }),
            )
            .expect_err("level-up moves must be exact move tokens")
            .to_string();
        assert!(
            error.contains("species value payload moves for 'NEW_MON' ' TACKLE'"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "moves": [{
                            "level": 1,
                            "move": "legacyTackle"
                        }]
                    }
                }),
            )
            .expect_err("species move payload reserved move ids must fail")
            .to_string();
        assert!(
            error.contains(
                "species value payload moves for 'NEW_MON' 'legacyTackle' uses reserved modpack payload prefix"
            ),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "NEW MON": {
                        "species": "NEW MON",
                        "moves": [{
                            "level": 1,
                            "move": "TACKLE"
                        }]
                    }
                }),
            )
            .expect_err("species move payload keys must be token ids")
            .to_string();
        assert!(
            error.contains(
                "species value payload species key 'NEW MON' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EggMoves,
                serde_json::json!({
                    "NEW_MON": {
                        "species": "NEW_MON",
                        "moves": ["CHARM"],
                        "fallback_moves": ["TACKLE"]
                    }
                }),
            )
            .expect_err("egg moves must reject non-exported fields");
        let error = format!("{error:#}");
        assert!(error.contains("unknown field `fallback_moves`"), "{error}");

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "NEW_MON\u{0007}": {
                        "species": "NEW_MON\u{0007}",
                        "moves": []
                    }
                }),
            )
            .expect_err("species move payload keys must be exact")
            .to_string();
        assert!(
            error.contains(
                "species value payload species key 'NEW_MON\u{0007}' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::LevelUpMoves,
                serde_json::json!({
                    "fallback_species": {
                        "species": "fallback_species",
                        "moves": [{
                            "level": 1,
                            "move": "TACKLE"
                        }]
                    }
                }),
            )
            .expect_err("species move payload reserved keys must fail at load time")
            .to_string();
        assert!(
            error.contains(
                "species value payload species key 'fallback_species' uses reserved modpack payload prefix"
            ),
            "{error}"
        );
    }

    #[test]
    fn map_like_payloads_reject_non_object_noop_shape() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapDimensions,
            serde_json::json!({
                "Route29": {
                    "width": 10,
                    "height": 9
                }
            }),
        )
        .expect("single exported object-map payload is canonical");
        assert!(data.map_dimensions.contains_key("Route29"));
        assert_eq!(
            data.map_dimensions["Route29"],
            serde_json::json!({
                "width": 10,
                "height": 9
            })
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MapDimensions,
                serde_json::json!([
                    {
                        "Route30": {
                            "width": 12,
                            "height": 10
                        }
                    }
                ]),
            )
            .expect_err("map-like payloads must not use array compatibility shape")
            .to_string();
        assert!(
            error.contains("map dimensions payload must be an object"),
            "{error}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(ContentPackCategory::MapDimensions, serde_json::json!(null))
            .expect_err("map-like payloads must not ignore malformed scalar payloads")
            .to_string();
        assert!(
            error.contains("map dimensions payload must be an object"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::Pokemon, serde_json::json!({}))
            .expect_err("empty object-map payloads must not be silent no-ops")
            .to_string();
        assert!(
            error.contains("object-map payload must contain at least one entry"),
            "{error}"
        );

        for (category, expected) in [
            (
                ContentPackCategory::MapDimensions,
                "map dimensions payload must contain at least one entry",
            ),
            (
                ContentPackCategory::LevelUpMoves,
                "species value payload must contain at least one entry",
            ),
            (
                ContentPackCategory::MapScripts,
                "map script payload must contain at least one entry",
            ),
            (
                ContentPackCategory::Npcs,
                "NPC payload must contain at least one entry",
            ),
            (
                ContentPackCategory::Pokedex,
                "pokedex payload must contain at least one entry",
            ),
        ] {
            let error = GameDataSet::default()
                .apply_content_pack_payload(category, serde_json::json!({}))
                .expect_err("empty custom object payloads must not be silent no-ops")
                .to_string();
            assert!(error.contains(expected), "{error}");
        }

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapDimensions,
            serde_json::json!({
                "Route29": {
                    "width": 10,
                    "height": 9
                }
            }),
        )
        .expect("apply first map dimension payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MapDimensions,
                serde_json::json!({
                    "Route29": {
                        "width": 11,
                        "height": 9
                    }
                }),
            )
            .expect_err("duplicate map-like payload key must not overwrite")
            .to_string();
        assert!(
            error.contains("duplicate object payload key 'Route29'"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapDimensions,
                serde_json::json!({
                    "Route29": {
                        "width": 10,
                        "height": 9,
                        "legacy_width": 10
                    }
                }),
            )
            .expect_err("map dimension records must reject unknown fields");
        assert!(
            format!("{error:#}").contains("unknown field `legacy_width`"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapDimensions,
                serde_json::json!({
                    "Route29": {
                        "width": 0,
                        "height": 9
                    }
                }),
            )
            .expect_err("map dimensions must be positive");
        assert!(
            format!("{error:#}").contains(
                "map dimensions payload for Route29 must declare positive width and height"
            ),
            "{error:#}"
        );

        for (category, payload, expected) in [
            (
                ContentPackCategory::MapDimensions,
                serde_json::json!({
                    "Route 29": {
                        "width": 10,
                        "height": 9
                    }
                }),
                "map dimensions payload 'Route 29' must be an exact map token",
            ),
            (
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29 MapScripts": []
                }),
                "map script payload 'Route29 MapScripts' must be an exact script label token",
            ),
            (
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "Route 29": []
                }),
                "NPC payload 'Route 29' must be an exact map token",
            ),
            (
                ContentPackCategory::MapDimensions,
                serde_json::json!({
                    "fallbackRoute": {
                        "width": 10,
                        "height": 9
                    }
                }),
                "map dimensions payload 'fallbackRoute' uses reserved modpack payload prefix",
            ),
            (
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "legacyMapScripts": []
                }),
                "map script payload 'legacyMapScripts' uses reserved modpack payload prefix",
            ),
            (
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "legacyRoute": []
                }),
                "NPC payload 'legacyRoute' uses reserved modpack payload prefix",
            ),
        ] {
            let mut data = GameDataSet::default();
            let error = data
                .apply_content_pack_payload(category, payload)
                .expect_err("map-like object keys must be exact")
                .to_string();
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn npc_payloads_reject_malformed_object_events_without_runtime_fallbacks() {
        let mut data = GameDataSet::default();
        let route29_object = test_object("ROUTE29_POKE_BALL", "EVENT_ROUTE_29_POTION", 3, 4);
        data.apply_content_pack_payload(
            ContentPackCategory::Npcs,
            serde_json::json!({
                "Route29": [route29_object.clone()]
            }),
        )
        .expect("canonical NPC object payload should load");
        assert!(data.npcs.contains_key("Route29"));
        assert_eq!(
            data.npcs["Route29"],
            serde_json::to_value(vec![route29_object]).expect("canonical NPC object payload")
        );

        let mut padded_movement = test_object("ROUTE29_POKE_BALL", "EVENT_ROUTE_29_POTION", 3, 4);
        padded_movement.spritemovedata = " SPRITEMOVEDATA_STANDING_DOWN".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "Route29": [padded_movement]
                }),
            )
            .expect_err("NPC movement tokens must not be trimmed");
        assert!(
            format!("{error:#}").contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \" SPRITEMOVEDATA_STANDING_DOWN\""
            ),
            "{error:#}"
        );

        let mut unknown_movement = test_object("ROUTE29_POKE_BALL", "EVENT_ROUTE_29_POTION", 3, 4);
        unknown_movement.spritemovedata = "SPRITEMOVEDATA_UNKNOWN".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "Route29": [unknown_movement]
                }),
            )
            .expect_err("NPC movement tokens must be known locally");
        assert!(
            format!("{error:#}").contains(
                "NPC object 0 on Route29 uses unknown spritemovedata 'SPRITEMOVEDATA_UNKNOWN'"
            ),
            "{error:#}"
        );

        let mut bad_script = test_object("ROUTE29_POKE_BALL", "EVENT_ROUTE_29_POTION", 3, 4);
        bad_script.script = "Route29 Script".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "Route29": [bad_script]
                }),
            )
            .expect_err("NPC scripts must be exact tokens");
        assert!(
            format!("{error:#}")
                .contains("map token must be exact ASCII alphanumeric/underscore/hyphen, found \"Route29 Script\""),
            "{error:#}"
        );

        let duplicate_a = test_object("ROUTE29_POKE_BALL", "EVENT_ROUTE_29_POTION", 3, 4);
        let duplicate_b = test_object("ROUTE29_POKE_BALL", "EVENT_ROUTE_29_RARE_CANDY", 5, 6);
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "Route29": [duplicate_a, duplicate_b]
                }),
            )
            .expect_err("NPC object identifiers must be unique per map");
        assert!(
            format!("{error:#}")
                .contains("NPC object identifier 'ROUTE29_POKE_BALL' is duplicated on map Route29"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Npcs,
                serde_json::json!({
                    "Route29": [{
                        "sprite": "SPRITE_POKE_BALL",
                        "x": 3,
                        "y": 4,
                        "spritemovedata": "SPRITEMOVEDATA_STANDING_DOWN",
                        "move_range_x": 0,
                        "move_range_y": 0,
                        "hram_x": 0,
                        "hram_y": 0,
                        "pal": 0,
                        "object_type": "OBJECTTYPE_SCRIPT",
                        "radius": 0,
                        "script": "ObjectEvent",
                        "label": null,
                        "event_flag": "EVENT_ROUTE_29_POTION",
                        "object_identifier": "ROUTE29_POKE_BALL",
                        "sightline_direction_override": null,
                        "legacy_sprite": "poke_ball"
                    }]
                }),
            )
            .expect_err("NPC object payloads must reject legacy fields");
        let error = format!("{error:#}");
        assert!(error.contains("unknown field `legacy_sprite`"), "{error}");
    }

    #[test]
    fn map_script_payloads_reject_malformed_command_lists_without_fallbacks() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapScripts,
            serde_json::json!({
                "Route29_MapScripts": [
                    {"command":"end","args":[]}
                ]
            }),
        )
        .expect("canonical map script payload should load");
        assert!(data.map_scripts.contains_key("Route29_MapScripts"));
        assert_eq!(
            data.map_scripts["Route29_MapScripts"],
            serde_json::json!([{"command":"end","args":[]}])
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapScripts,
            serde_json::json!({
                "Route29WarpScript": [
                    {"command":"warpfacing","args":["ROUTE_29","6","27","RIGHT"]}
                ]
            }),
        )
        .expect("canonical warpfacing payload should load");
        assert_eq!(
            data.map_scripts["Route29WarpScript"],
            serde_json::json!([
                {"command":"warpfacing","args":["ROUTE_29","6","27","RIGHT"]}
            ])
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29WarpScript": [
                        {"command":"warpfacing","args":["RIGHT","ROUTE_29","6","27"]}
                    ]
                }),
            )
            .expect_err("ASM-order warpfacing payload must not be normalized")
            .to_string();
        assert!(
            error.contains("unknown script facing direction '27'"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29WarpScript": [
                        {"command":"warpfacing","args":"RIGHT,ROUTE_29,6,27"}
                    ]
                }),
            )
            .expect_err("warpfacing args must be a typed array")
            .to_string();
        assert!(
            error.contains("warpfacing args must be an array"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29_MapScripts": [
                        {"command":"end"}
                    ]
                }),
            )
            .expect_err("map script commands must declare args explicitly");
        assert!(
            format!("{error:#}").contains("missing field `args`"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29_MapScripts": [
                        {"command":" end","args":[]}
                    ]
                }),
            )
            .expect_err("map script command names must be exact");
        assert!(
            format!("{error:#}").contains(
                "map script payload script 'Route29_MapScripts' command 0 name ' end' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29_MapScripts": [
                        {"command":"jump","args":[" Route29Script"]}
                    ]
                }),
            )
            .expect_err("map script command args must be exact");
        assert!(
            format!("{error:#}").contains(
                "map script payload script 'Route29_MapScripts' command 0 arg 0 ' Route29Script' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapScripts,
                serde_json::json!({
                    "Route29_MapScripts": [
                        {"command":"jump","args":["Route29Script"],"fallback_args":["DefaultScript"]}
                    ]
                }),
            )
            .expect_err("map script command objects must not carry fallback fields");
        assert!(
            format!("{error:#}").contains("unknown field `fallback_args`"),
            "{error:#}"
        );
    }

    #[test]
    fn map_attribute_and_block_payloads_reject_duplicate_exact_keys() {
        let mut data = GameDataSet::default();
        let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        attributes.environment = Some("TOWN".to_string());
        data.apply_content_pack_payload(
            ContentPackCategory::MapAttributes,
            serde_json::json!({
                "Route29": attributes
            }),
        )
        .expect("apply first map attributes payload");
        let mut duplicate_attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        duplicate_attributes.environment = Some("ROUTE".to_string());
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route29": duplicate_attributes
                }),
            )
            .expect_err("duplicate map attributes payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate map attributes for map 'Route29'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::MapBlocks,
            serde_json::json!({
                "Route29_Blocks": "AA=="
            }),
        )
        .expect("apply first map blocks payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MapBlocks,
                serde_json::json!({
                    "Route29_Blocks": "AQ=="
                }),
            )
            .expect_err("duplicate map block payload must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate map block data for label 'Route29_Blocks'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::MapBlocks, serde_json::json!(null))
            .expect_err("map blocks must use object payloads")
            .to_string();
        assert!(
            error.contains("map block payload must be an object"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapBlocks,
                serde_json::json!({
                    "Route29_Blocks": ["AA=="]
                }),
            )
            .expect_err("map block values must be strings")
            .to_string();
        assert!(
            error.contains("map block payload 'Route29_Blocks' must be a string"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapBlocks,
                serde_json::json!({
                    "Route29_Blocks": " AA=="
                }),
            )
            .expect_err("map block base64 must not be trim-decoded")
            .to_string();
        assert!(
            error.contains(
                "map block data for label 'Route29_Blocks' ' AA==' must be exact, non-empty, and untrimmed"
            ),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapBlocks,
                serde_json::json!({
                    "Route29_Blocks": "AA =="
                }),
            )
            .expect_err("map block base64 must not ignore interior whitespace")
            .to_string();
        assert!(
            error.contains("decode map block payload 'Route29_Blocks'"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapBlocks,
                serde_json::json!({
                    "Route29_Blocks": "AA@="
                }),
            )
            .expect_err("map block base64 must decode at load time")
            .to_string();
        assert!(
            error.contains("decode map block payload 'Route29_Blocks'"),
            "{error}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapBlocks,
                serde_json::json!({
                    "legacyBlocks": "AA=="
                }),
            )
            .expect_err("map block labels must reject reserved payload ids")
            .to_string();
        assert!(
            error.contains(
                "map block data for label 'legacyBlocks' uses reserved modpack payload prefix"
            ),
            "{error}"
        );
    }

    #[test]
    fn map_attribute_payloads_reject_malformed_values_without_trimming() {
        let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        attributes.tileset_name = "JOHTO".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route29": attributes
                }),
            )
            .expect_err("map attributes tileset ids must be exact asset ids");
        assert!(
            format!("{error:#}")
                .contains("map 'Route29' tileset_name 'JOHTO' must be an exact tileset id"),
            "{error:#}"
        );

        let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        attributes.connections = vec![MapConnection {
            direction: "up".to_string(),
            target_map: "Route30".to_string(),
            offset: 0,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route29": attributes
                }),
            )
            .expect_err("map connection directions must use Crystal direction names");
        assert!(
            format!("{error:#}").contains(
                "map 'Route29' connection direction 'up' must be one of north, south, west, east"
            ),
            "{error:#}"
        );

        let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        attributes.connections = vec![MapConnection {
            direction: "north".to_string(),
            target_map: " Route30".to_string(),
            offset: 0,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route29": attributes
                }),
            )
            .expect_err("map connection targets must be exact map ids");
        assert!(
            format!("{error:#}")
                .contains("map token must be exact ASCII alphanumeric/underscore/hyphen"),
            "{error:#}"
        );

        let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        attributes.connections = vec![MapConnection {
            direction: "north".to_string(),
            target_map: "fallbackRoute".to_string(),
            offset: 0,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route29": attributes
                }),
            )
            .expect_err("map connection fallback targets must be reserved");
        assert!(
            format!("{error:#}")
                .contains("map token must be exact ASCII alphanumeric/underscore/hyphen"),
            "{error:#}"
        );

        let attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route 29": attributes
                }),
            )
            .expect_err("map attribute keys must be map tokens");
        assert!(
            format!("{error:#}").contains(
                "map attributes map id 'Route 29' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let attributes = test_map_module("legacyRoute", "ROUTE_29", None).attributes;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "legacyRoute": attributes
                }),
            )
            .expect_err("map attribute reserved keys must fail at load time");
        assert!(
            format!("{error:#}").contains(
                "map attributes map id 'legacyRoute' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );

        let mut attributes = test_map_module("Route29", "ROUTE_29", None).attributes;
        attributes.height = 0;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MapAttributes,
                serde_json::json!({
                    "Route29": attributes
                }),
            )
            .expect_err("map attribute dimensions must be positive at load time");
        assert!(
            format!("{error:#}").contains("parse object-map payload: map johto has height 0"),
            "{error:#}"
        );
    }

    #[test]
    fn map_module_payloads_reject_malformed_ids_and_attributes_without_trimming() {
        let mut module = test_map_module(" Route29", "ROUTE_29", None);
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    " Route29": module
                }),
            )
            .expect_err("map module ids must be exact");
        assert!(
            format!("{error:#}").contains(
                "map module key ' Route29' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        module = test_map_module("Route 29", "ROUTE_29", None);
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route 29": module
                }),
            )
            .expect_err("map module ids must be map tokens");
        assert!(
            format!("{error:#}").contains(
                "map module key 'Route 29' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        module = test_map_module("fallbackMap", "ROUTE_29", None);
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "fallbackMap": module
                }),
            )
            .expect_err("map module reserved ids must fail at load time");
        assert!(
            format!("{error:#}")
                .contains("map module key 'fallbackMap' uses reserved modpack payload prefix"),
            "{error:#}"
        );

        module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.map_constant = Some(" ROUTE_29".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map module attributes must use exact map constants");
        assert!(
            format!("{error:#}")
                .contains("map token must be exact ASCII alphanumeric/underscore/hyphen"),
            "{error:#}"
        );

        module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.map_constant = Some("ROUTE 29".to_string());
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map module attributes must reject internal spaces");
        assert!(
            format!("{error:#}")
                .contains("map token must be exact ASCII alphanumeric/underscore/hyphen"),
            "{error:#}"
        );
    }

    #[test]
    fn map_module_payloads_validate_inline_block_dimensions_without_fallbacks() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 2, 3];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("inline map blocks must match declared dimensions");
        assert!(
            format!("{error:#}")
                .contains("map module 'Route29' has 3 inline blocks but dimensions require 4"),
            "{error:#}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 0;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map dimensions must be positive at load time");
        assert!(
            format!("{error:#}").contains("parse object-map payload: map johto has width 0"),
            "{error:#}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.blocks.clear();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map modules require inline blocks or an external block label");
        assert!(
            format!("{error:#}").contains(
                "map module 'Route29' must declare inline blocks or an exact blocks_label"
            ),
            "{error:#}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.attributes.blocks_label = Some("Route29_Blocks".to_string());
        module.blocks.clear();
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("externalized block payloads may leave inline blocks empty");
    }

    #[test]
    fn map_module_payloads_reject_malformed_section_commands_without_fallbacks() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.map_script_section_commands = vec![MapScriptSectionCommand {
            command: "scene_script".to_string(),
            args: vec![" Route29Scene0".to_string()],
            command_index: 0,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map script section command args must be exact");
        assert!(
            format!("{error:#}")
                .contains("map token must be exact ASCII alphanumeric/underscore/hyphen"),
            "{error:#}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.map_script_section_commands = vec![MapScriptSectionCommand {
            command: "scene_script".to_string(),
            args: Vec::new(),
            command_index: 1,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map script section command arity must be exact");
        assert!(
            format!("{error:#}")
                .contains("map script command scene_script has 0 args, expected {1, 2}"),
            "{error:#}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.map_event_section_commands = vec![MapEventSectionCommand {
            command: "object event".to_string(),
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
                "Route29ObjectScript".to_string(),
                "-1".to_string(),
            ],
            command_index: 2,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map event section command names must be known");
        assert!(
            format!("{error:#}").contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \"object event\""
            ),
            "{error:#}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.map_event_section_commands = vec![MapEventSectionCommand {
            command: "bg_event".to_string(),
            args: vec![
                "1".to_string(),
                "2".to_string(),
                "BGEVENT_READ".to_string(),
                "Route29 SignScript".to_string(),
            ],
            command_index: 3,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("map event section command args must be exact");
        assert!(
            format!("{error:#}").contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \"Route29 SignScript\""
            ),
            "{error:#}"
        );
    }

    #[test]
    fn map_module_payloads_validate_embedded_scripts_and_objects() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29_MapScripts".to_string(),
            serde_json::json!([
                {
                    "command": "jump",
                    "args": ["Route29Script"]
                }
            ]),
        );
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical embedded map scripts must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            " Route29_MapScripts".to_string(),
            serde_json::json!([
                {
                    "command": "jump",
                    "args": ["Route29Script"]
                }
            ]),
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("embedded map script keys must be exact")
            .to_string();
        assert!(
            error.contains(
                "map module script ' Route29_MapScripts' must be an exact script label token"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29_MapScripts".to_string(),
            serde_json::json!([
                {
                    "command": "jump"
                }
            ]),
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("embedded map script commands must be explicit")
            .to_string();
        assert!(
            error.contains("validate map module 'Route29' script 'Route29_MapScripts'"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29_MapScripts".to_string(),
            serde_json::json!([
                {
                    "command": "jump",
                    "args": [" Route29Script"]
                }
            ]),
        );
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("embedded map script args must be exact")
            .to_string();
        assert!(
            error.contains("validate map module 'Route29' script 'Route29_MapScripts'"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.objects = vec![ObjectEvent {
            sprite: " SPRITE_MON".to_string(),
            x: 0,
            y: 0,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "ObjectEvent".to_string(),
            label: None,
            event_flag: "-1".to_string(),
            object_identifier: None,
            sightline_direction_override: None,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("embedded map object events must be exact")
            .to_string();
        assert!(
            error.contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \" SPRITE_MON\""
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_scenes_and_events() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29SceneScript".to_string(),
            serde_json::json!([
                {
                    "command": "end",
                    "args": []
                }
            ]),
        );
        module.scripts.insert(
            "Route29CoordScript".to_string(),
            serde_json::json!([
                {
                    "command": "end",
                    "args": []
                }
            ]),
        );
        module.scripts.insert(
            "Route29BgScript".to_string(),
            serde_json::json!([
                {
                    "command": "end",
                    "args": []
                }
            ]),
        );
        module.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_ROUTE_29_START".to_string(),
                script_name: Some("Route29SceneScript".to_string()),
            }],
        };
        module.events = MapEvents {
            warps: vec![WarpEvent {
                index: 1,
                x: 1,
                y: 1,
                target_map_constant: "CHERRYGROVE_CITY".to_string(),
                target_map: "CherrygroveCity".to_string(),
                target_warp_id: 2,
            }],
            coord_events: vec![CoordEvent {
                x: 2,
                y: 2,
                scene_id: "SCENE_ROUTE_29_START".to_string(),
                script_name: "Route29CoordScript".to_string(),
            }],
            bg_events: vec![BackgroundEvent {
                x: 3,
                y: 3,
                event_type: "BGEVENT_READ".to_string(),
                script: "Route29BgScript".to_string(),
            }],
        };
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical scene and event records must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE ROUTE 29".to_string(),
                script_name: None,
            }],
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scene ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \"SCENE ROUTE 29\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_ROUTE_29_START".to_string(),
                script_name: Some("MissingSceneScript".to_string()),
            }],
        };
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scene scripts must resolve locally")
            .to_string();
        assert!(
            error
                .contains("map 'Route29' scene script 'MissingSceneScript' is not a loaded script"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.events.warps = vec![WarpEvent {
            index: 1,
            x: 1,
            y: 1,
            target_map_constant: "CHERRYGROVE CITY".to_string(),
            target_map: "CherrygroveCity".to_string(),
            target_warp_id: 2,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("warp target constants must be exact")
            .to_string();
        assert!(
            error.contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \"CHERRYGROVE CITY\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.events.coord_events = vec![CoordEvent {
            x: 1,
            y: 1,
            scene_id: "SCENE_ROUTE_29_START".to_string(),
            script_name: "Route29 CoordScript".to_string(),
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("coord event scripts must be exact")
            .to_string();
        assert!(
            error.contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \"Route29 CoordScript\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.scripts.insert(
            "Route29BgScript".to_string(),
            serde_json::json!([
                {
                    "command": "end",
                    "args": []
                }
            ]),
        );
        module.events.bg_events = vec![BackgroundEvent {
            x: 1,
            y: 1,
            event_type: "BGEVENT READ".to_string(),
            script: "Route29BgScript".to_string(),
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("background event types must be exact")
            .to_string();
        assert!(
            error.contains(
                "map token must be exact ASCII alphanumeric/underscore/hyphen, found \"BGEVENT READ\""
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_shop_and_phone_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_shop_commands = vec![ScriptShopCommand {
            command: "pokemart".to_string(),
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "MART_CHERRYGROVE".to_string(),
            source_script: "Route29MartScript".to_string(),
            command_index: 0,
        }];
        module.script_phone_commands = vec![
            ScriptPhoneCommand {
                command: "checkcellnum".to_string(),
                contact_id: "PHONE_MOM".to_string(),
                source_script: "Route29PhoneScript".to_string(),
                command_index: 1,
            },
            ScriptPhoneCommand {
                command: "askforphonenumber".to_string(),
                contact_id: "PHONE_ELM".to_string(),
                source_script: "Route29PhoneScript".to_string(),
                command_index: 2,
            },
        ];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted map script commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_shop_commands = vec![ScriptShopCommand {
            command: "pokemart ".to_string(),
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "MART_CHERRYGROVE".to_string(),
            source_script: "Route29MartScript".to_string(),
            command_index: 3,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script shop command names must be exact")
            .to_string();
        assert!(
            error.contains("invalid script shop command 'pokemart '"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_shop_commands = vec![ScriptShopCommand {
            command: "sellmart".to_string(),
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "MART_CHERRYGROVE".to_string(),
            source_script: "Route29MartScript".to_string(),
            command_index: 4,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("unknown script shop commands must be rejected")
            .to_string();
        assert!(
            error.contains("unknown script shop command 'sellmart'"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_shop_commands = vec![ScriptShopCommand {
            command: "pokemart".to_string(),
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "MART CHERRYGROVE".to_string(),
            source_script: "Route29MartScript".to_string(),
            command_index: 5,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script shop mart ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "script shop token must be exact ASCII alphanumeric/underscore, found \"MART CHERRYGROVE\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_shop_commands = vec![ScriptShopCommand {
            command: "pokemart".to_string(),
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "fallbackMart".to_string(),
            source_script: "Route29MartScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script shop reserved mart ids must fail")
            .to_string();
        assert!(
            error.contains(
                "script shop token must be exact ASCII alphanumeric/underscore, found \"fallbackMart\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_phone_commands = vec![ScriptPhoneCommand {
            command: "checkcellnum".to_string(),
            contact_id: "PHONE MOM".to_string(),
            source_script: "Route29PhoneScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script phone contact ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "phone contact record contactId 'PHONE MOM' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_phone_commands = vec![ScriptPhoneCommand {
            command: "checkcellnum".to_string(),
            contact_id: "legacyContact".to_string(),
            source_script: "Route29PhoneScript".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script phone reserved contact ids must fail")
            .to_string();
        assert!(
            error.contains(
                "phone contact record contactId 'legacyContact' must be exact ASCII alphanumeric or underscore"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_phone_commands = vec![ScriptPhoneCommand {
            command: "deletecellnum".to_string(),
            contact_id: "PHONE_MOM".to_string(),
            source_script: "Route29PhoneScript".to_string(),
            command_index: 8,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("unknown script phone commands must be rejected")
            .to_string();
        assert!(
            error.contains("unknown script phone command 'deletecellnum'"),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_variable_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_variable_commands = vec![
            ScriptVariableCommand {
                command: "setval".to_string(),
                target: None,
                value_tokens: vec!["SCREEN_WIDTH".to_string(), "-".to_string(), "1".to_string()],
                source_script: "Route29VarScript".to_string(),
                command_index: 0,
            },
            ScriptVariableCommand {
                command: "loadvar".to_string(),
                target: Some("VAR_BATTLETYPE".to_string()),
                value_tokens: vec!["BATTLETYPE_SHINY".to_string()],
                source_script: "Route29VarScript".to_string(),
                command_index: 1,
            },
        ];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted variable commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_variable_commands = vec![ScriptVariableCommand {
            command: "loadvar".to_string(),
            target: Some("VAR BATTLETYPE".to_string()),
            value_tokens: vec!["BATTLETYPE_SHINY".to_string()],
            source_script: "Route29VarScript".to_string(),
            command_index: 2,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script variable targets must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "script variable target must be exact ASCII alphanumeric/underscore, found \"VAR BATTLETYPE\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_variable_commands = vec![ScriptVariableCommand {
            command: "setval".to_string(),
            target: None,
            value_tokens: vec![" BAD".to_string()],
            source_script: "Route29VarScript".to_string(),
            command_index: 3,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script variable values must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "script variable value token must be exact visible ASCII, found \" BAD\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_variable_commands = vec![ScriptVariableCommand {
            command: "setvar".to_string(),
            target: Some("VAR_BATTLETYPE".to_string()),
            value_tokens: vec!["BATTLETYPE_SHINY".to_string()],
            source_script: "Route29VarScript".to_string(),
            command_index: 4,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("unknown script variable commands must be rejected")
            .to_string();
        assert!(
            error.contains("unknown script variable command 'setvar'"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_variable_commands = vec![ScriptVariableCommand {
            command: "readvar".to_string(),
            target: Some("VAR_BATTLETYPE".to_string()),
            value_tokens: Vec::new(),
            source_script: " Route29VarScript".to_string(),
            command_index: 5,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script variable source scripts must be exact")
            .to_string();
        assert!(
            error.contains(
                "script variable source must be exact ASM label syntax, found \" Route29VarScript\""
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_audio_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_audio_commands = vec![
            ScriptAudioCommand {
                command: "playmusic".to_string(),
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "Route29AudioScript".to_string(),
                command_index: 0,
            },
            ScriptAudioCommand {
                command: "playsound".to_string(),
                audio_id: Some("SFX_ITEM".to_string()),
                fade_frames: None,
                source_script: "Route29AudioScript".to_string(),
                command_index: 1,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("CHIKORITA".to_string()),
                fade_frames: None,
                source_script: "Route29AudioScript".to_string(),
                command_index: 2,
            },
            ScriptAudioCommand {
                command: "musicfadeout".to_string(),
                audio_id: Some("MUSIC_NONE".to_string()),
                fade_frames: Some(10),
                source_script: "Route29AudioScript".to_string(),
                command_index: 3,
            },
            ScriptAudioCommand {
                command: "waitsfx".to_string(),
                audio_id: None,
                fade_frames: None,
                source_script: "Route29AudioScript".to_string(),
                command_index: 4,
            },
        ];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted audio commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic ".to_string(),
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "Route29AudioScript".to_string(),
            command_index: 5,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script audio command names must be exact")
            .to_string();
        assert!(
            error.contains(
                "script audio command must be exact lowercase ASCII, found \"playmusic \""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic".to_string(),
            audio_id: Some("MUSIC ROUTE 29".to_string()),
            fade_frames: None,
            source_script: "Route29AudioScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script audio ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "script audio token must be exact ASCII alphanumeric/underscore, found \"MUSIC ROUTE 29\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "playmusic".to_string(),
            audio_id: Some("fallbackMusic".to_string()),
            fade_frames: None,
            source_script: "Route29AudioScript".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script audio reserved ids must fail")
            .to_string();
        assert!(
            error.contains(
                "script audio token must be exact ASCII alphanumeric/underscore, found \"fallbackMusic\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "musicfadeout".to_string(),
            audio_id: Some("MUSIC_ROUTE_29".to_string()),
            fade_frames: None,
            source_script: "Route29AudioScript".to_string(),
            command_index: 8,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("musicfadeout must include fade frames")
            .to_string();
        assert!(
            error.contains("script audio command musicfadeout requires fade_frames"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_audio_commands = vec![ScriptAudioCommand {
            command: "waitsfx".to_string(),
            audio_id: Some("SFX_ITEM".to_string()),
            fade_frames: None,
            source_script: "Route29AudioScript".to_string(),
            command_index: 9,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("waitsfx must not include an audio id")
            .to_string();
        assert!(
            error.contains("script audio command waitsfx must not declare audio_id"),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_item_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_item_grants = vec![ScriptItemGrant {
            command: "verbosegiveitem".to_string(),
            item_id: "POTION".to_string(),
            quantity: 2,
            source_script: "Route29ItemScript".to_string(),
            command_index: 0,
            verbose: true,
        }];
        module.script_item_checks = vec![ScriptItemAccess {
            command: "checkitem".to_string(),
            item_id: "ITEM_FROM_MEM".to_string(),
            source_script: "Route29ItemScript".to_string(),
            command_index: 1,
        }];
        module.script_item_takes = vec![ScriptItemAccess {
            command: "takeitem".to_string(),
            item_id: "POTION".to_string(),
            source_script: "Route29ItemScript".to_string(),
            command_index: 2,
        }];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted item commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_item_grants = vec![ScriptItemGrant {
            command: "verbosegiveitem".to_string(),
            item_id: "POTION".to_string(),
            quantity: 0,
            source_script: "Route29ItemScript".to_string(),
            command_index: 3,
            verbose: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script item grant quantities must be nonzero")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script item grant command 3 quantity must be greater than zero"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_item_grants = vec![ScriptItemGrant {
            command: "verbosegiveitem".to_string(),
            item_id: " POTION".to_string(),
            quantity: 1,
            source_script: "Route29ItemScript".to_string(),
            command_index: 4,
            verbose: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script item grant ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "script item token must be exact ASCII alphanumeric/underscore, found \" POTION\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_item_grants = vec![ScriptItemGrant {
            command: "verbosegiveitem".to_string(),
            item_id: "legacyPotion".to_string(),
            quantity: 1,
            source_script: "Route29ItemScript".to_string(),
            command_index: 5,
            verbose: false,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script item grant reserved ids must fail")
            .to_string();
        assert!(
            error.contains(
                "script item token must be exact ASCII alphanumeric/underscore, found \"legacyPotion\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_item_checks = vec![ScriptItemAccess {
            command: "checkitem".to_string(),
            item_id: "POTION".to_string(),
            source_script: " Route29ItemScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script item access source scripts must be exact")
            .to_string();
        assert!(
            error.contains(
                "script label token must be exact visible ASCII, found \" Route29ItemScript\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_item_takes = vec![ScriptItemAccess {
            command: "takeitem".to_string(),
            item_id: "POTION-1".to_string(),
            source_script: "Route29ItemScript".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script item take ids must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "script item token must be exact ASCII alphanumeric/underscore, found \"POTION-1\""
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_flag_scene_and_economy_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_flag_commands = vec![ScriptFlagCommand {
            command: "setevent".to_string(),
            flag_id: "EVENT_ROUTE_29_TUTORIAL".to_string(),
            source_script: "Route29FlagScript".to_string(),
            command_index: 0,
        }];
        module.script_scene_commands = vec![
            ScriptSceneCommand {
                command: "checkscene".to_string(),
                map_id: None,
                scene_id: None,
                source_script: "Route29SceneScript".to_string(),
                command_index: 1,
            },
            ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("ROUTE_29".to_string()),
                scene_id: Some("SCENE_ROUTE_29_DONE".to_string()),
                source_script: "Route29SceneScript".to_string(),
                command_index: 2,
            },
        ];
        module.script_economy_commands = vec![
            ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["MAX_MONEY".to_string(), "-".to_string(), "1".to_string()],
                source_script: "Route29MoneyScript".to_string(),
                command_index: 3,
            },
            ScriptEconomyCommand {
                command: "givecoins".to_string(),
                account: None,
                amount_tokens: vec!["10".to_string()],
                source_script: "Route29MoneyScript".to_string(),
                command_index: 4,
            },
        ];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted flag, scene, and economy commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_flag_commands = vec![ScriptFlagCommand {
            command: "setevent ".to_string(),
            flag_id: "EVENT_ROUTE_29_TUTORIAL".to_string(),
            source_script: "Route29FlagScript".to_string(),
            command_index: 5,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script flag command names must be exact")
            .to_string();
        assert!(
            error.contains(
                "script flag command must be exact lowercase ASCII/underscore, found \"setevent \""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_flag_commands = vec![ScriptFlagCommand {
            command: "setevent".to_string(),
            flag_id: "EVENT ROUTE 29".to_string(),
            source_script: "Route29FlagScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("script flag ids must be exact")
            .to_string();
        assert!(
            error.contains(
                "script flag token must be exact ASCII alphanumeric/underscore, found \"EVENT ROUTE 29\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_scene_commands = vec![ScriptSceneCommand {
            command: "checkscene".to_string(),
            map_id: Some("ROUTE_29".to_string()),
            scene_id: None,
            source_script: "Route29SceneScript".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("checkscene must not carry a target map")
            .to_string();
        assert!(
            error.contains("invalid script scene command: UnexpectedTargetMap"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_scene_commands = vec![ScriptSceneCommand {
            command: "setscene".to_string(),
            map_id: None,
            scene_id: Some("SCENE ROUTE 29".to_string()),
            source_script: "Route29SceneScript".to_string(),
            command_index: 8,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("scene ids must be exact")
            .to_string();
        assert!(
            error.contains(
                "script scene token must be exact ASCII alphanumeric/underscore, found \"SCENE ROUTE 29\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "checkmoney".to_string(),
            account: None,
            amount_tokens: vec!["100".to_string()],
            source_script: "Route29MoneyScript".to_string(),
            command_index: 9,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("money commands must include a money account")
            .to_string();
        assert!(
            error.contains("script economy command checkmoney requires money account"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "givecoins".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec!["10".to_string()],
            source_script: "Route29MoneyScript".to_string(),
            command_index: 10,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("coin commands must not include a money account")
            .to_string();
        assert!(
            error.contains("script economy command givecoins must not declare money account"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "takemoney".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec![" MAX_MONEY".to_string()],
            source_script: "Route29MoneyScript".to_string(),
            command_index: 11,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("economy amount tokens must be exact")
            .to_string();
        assert!(
            error.contains(
                "script economy amount token must be exact digits, '+', '-', or ASCII alphanumeric/underscore constant, found \" MAX_MONEY\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_economy_commands = vec![ScriptEconomyCommand {
            command: "takemoney".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec![String::new()],
            source_script: "Route29MoneyScript".to_string(),
            command_index: 12,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("economy amount tokens must be nonempty")
            .to_string();
        assert!(
            error.contains(
                "script economy amount token must be exact digits, '+', '-', or ASCII alphanumeric/underscore constant, found \"\""
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_field_pickups() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![
            ScriptFieldPickup {
                command: "itemball".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 2,
                event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "Route29PotionScript".to_string(),
                command_index: 0,
            },
            ScriptFieldPickup {
                command: "hiddenitem".to_string(),
                item_id: Some("RARE_CANDY".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_ROUTE_29_HIDDEN_RARE_CANDY".to_string()),
                fruit_tree_id: None,
                source_script: "Route29HiddenItemScript".to_string(),
                command_index: 1,
            },
            ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some("FRUITTREE_ROUTE_29".to_string()),
                source_script: "Route29FruitTreeScript".to_string(),
                command_index: 2,
            },
        ];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted field pickups must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "ITEMBALL".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 1,
            event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
            fruit_tree_id: None,
            source_script: "Route29PotionScript".to_string(),
            command_index: 3,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("field pickup command names must be known")
            .to_string();
        assert!(
            error.contains(
                "script field pickup command must be exact lowercase ASCII, found \"ITEMBALL\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "itemball".to_string(),
            item_id: Some("RARE CANDY".to_string()),
            quantity: 1,
            event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
            fruit_tree_id: None,
            source_script: "Route29PotionScript".to_string(),
            command_index: 4,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("field pickup item ids must be exact")
            .to_string();
        assert!(
            error.contains(
                "field item token must be exact ASCII alphanumeric/underscore, found \"RARE CANDY\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "hiddenitem".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 0,
            event_flag: Some("EVENT_ROUTE_29_POTION".to_string()),
            fruit_tree_id: None,
            source_script: "Route29HiddenItemScript".to_string(),
            command_index: 5,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("field pickup quantities must be nonzero")
            .to_string();
        assert!(
            error.contains("script field pickup quantity must be positive"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "itemball".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 1,
            event_flag: Some("-1".to_string()),
            fruit_tree_id: None,
            source_script: "Route29PotionScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("field pickup event flags must be exact tokens")
            .to_string();
        assert!(
            error.contains(
                "field item token must be exact ASCII alphanumeric/underscore, found \"-1\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "fruittree".to_string(),
            item_id: None,
            quantity: 2,
            event_flag: None,
            fruit_tree_id: Some("FRUITTREE_ROUTE_29".to_string()),
            source_script: "Route29FruitTreeScript".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("fruit tree pickup quantity must be canonical")
            .to_string();
        assert!(
            error.contains("fruit tree pickup quantity must be exactly 1"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_field_pickups = vec![ScriptFieldPickup {
            command: "fruittree".to_string(),
            item_id: Some("BERRY".to_string()),
            quantity: 1,
            event_flag: None,
            fruit_tree_id: Some("FRUITTREE_ROUTE_29".to_string()),
            source_script: "Route29FruitTreeScript".to_string(),
            command_index: 8,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("fruit trees must not inline derived item ids")
            .to_string();
        assert!(
            error.contains("fruit tree pickup must not inline item_id or event_flag"),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_block_and_object_commands() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.objects = vec![ObjectEvent {
            sprite: "SPRITE_YOUNGSTER".to_string(),
            x: 0,
            y: 0,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "YoungsterScript".to_string(),
            label: None,
            event_flag: "EVENT_ROUTE_29_YOUNGSTER_HIDDEN".to_string(),
            object_identifier: Some("ROUTE_29_YOUNGSTER".to_string()),
            sightline_direction_override: None,
        }];
        module.script_movements = vec![ScriptMovement {
            label: "Route29YoungsterMovement".to_string(),
            source_script: Some("Route29ObjectScript".to_string()),
            steps: vec![ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        }];
        module.script_block_changes = vec![ScriptBlockChange {
            x: 0,
            y: 0,
            block_id: 42,
            source_script: "Route29BlockScript".to_string(),
            command_index: 0,
        }];
        module.script_object_commands = vec![
            ScriptObjectCommand {
                command: "appear".to_string(),
                object_id: Some("ROUTE_29_YOUNGSTER".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "Route29ObjectScript".to_string(),
                command_index: 1,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("ROUTE_29_YOUNGSTER".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("Route29YoungsterMovement".to_string()),
                emote: None,
                duration: None,
                source_script: "Route29ObjectScript".to_string(),
                command_index: 2,
            },
            ScriptObjectCommand {
                command: "faceplayer".to_string(),
                object_id: None,
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "Route29ObjectScript".to_string(),
                command_index: 3,
            },
        ];
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted block and object commands must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_block_changes = vec![ScriptBlockChange {
            x: 2,
            y: 0,
            block_id: 42,
            source_script: "Route29BlockScript".to_string(),
            command_index: 4,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("block changes must be in bounds")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script block change command 4 in 'Route29BlockScript' is malformed: OutOfBounds"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_block_changes = vec![ScriptBlockChange {
            x: 0,
            y: 0,
            block_id: 42,
            source_script: " Route29BlockScript".to_string(),
            command_index: 5,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("block change source scripts must be exact")
            .to_string();
        assert!(
            error.contains(
                "script block label must be exact visible ASCII, found \" Route29BlockScript\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "appear ".to_string(),
            object_id: Some("ROUTE_29_YOUNGSTER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 6,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("object command names must be exact")
            .to_string();
        assert!(
            error
                .contains("script object command must be exact lowercase ASCII, found \"appear \""),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "moveobject".to_string(),
            object_id: Some("ROUTE_29_YOUNGSTER".to_string()),
            target_object_id: None,
            x: Some(1),
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 7,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("moveobject must include both coordinates")
            .to_string();
        assert!(
            error.contains("script object command moveobject requires x and y"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_object_commands = vec![ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("MissingMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29ObjectScript".to_string(),
            command_index: 8,
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("object movement labels must resolve exactly")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script object command 8 in 'Route29ObjectScript' is malformed: unknown_movement"
            ),
            "{error}"
        );
    }

    #[test]
    fn map_module_payloads_validate_extracted_movements() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: "Route29Movement".to_string(),
            source_script: Some("Route29MovementScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("DOWN".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step_sleep".to_string(),
                    direction: None,
                    duration: Some(4),
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
        GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect("canonical extracted movements must load");

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: " Route29Movement".to_string(),
            source_script: Some("Route29MovementScript".to_string()),
            steps: vec![ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("movement labels must be exact")
            .to_string();
        assert!(
            error.contains(
                "script object token must be exact ASCII alphanumeric/underscore, found \" Route29Movement\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: "Route29Movement".to_string(),
            source_script: Some(" Route29MovementScript".to_string()),
            steps: vec![ScriptMovementStep {
                command: "step_end".to_string(),
                direction: None,
                duration: None,
                index: 0,
            }],
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("movement source scripts must be exact")
            .to_string();
        assert!(
            error.contains(
                "script label token must be exact visible ASCII, found \" Route29MovementScript\""
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: "Route29Movement".to_string(),
            source_script: Some("Route29MovementScript".to_string()),
            steps: Vec::new(),
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("movements must include steps")
            .to_string();
        assert!(
            error.contains(
                "map 'Route29' script movement 'Route29Movement' must include at least one step"
            ),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: "Route29Movement".to_string(),
            source_script: Some("Route29MovementScript".to_string()),
            steps: vec![ScriptMovementStep {
                command: "step".to_string(),
                direction: None,
                duration: None,
                index: 3,
            }],
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("directional movements must include directions")
            .to_string();
        assert!(
            error.contains("invalid movement step: MissingDirection"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: "Route29Movement".to_string(),
            source_script: Some("Route29MovementScript".to_string()),
            steps: vec![ScriptMovementStep {
                command: "step_end".to_string(),
                direction: Some("DOWN".to_string()),
                duration: None,
                index: 4,
            }],
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("no-arg movements must not include directions")
            .to_string();
        assert!(
            error.contains("invalid movement step: UnexpectedDirection"),
            "{error}"
        );

        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.script_movements = vec![ScriptMovement {
            label: "Route29Movement".to_string(),
            source_script: Some("Route29MovementScript".to_string()),
            steps: vec![ScriptMovementStep {
                command: "moonwalk".to_string(),
                direction: None,
                duration: None,
                index: 5,
            }],
        }];
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Maps,
                serde_json::json!({
                    "Route29": module
                }),
            )
            .expect_err("unknown movements must be rejected")
            .to_string();
        assert!(
            error.contains("invalid movement step: UnsupportedCommand"),
            "{error}"
        );
    }

    #[test]
    fn raw_script_movement_extraction_rejects_malformed_payloads_without_silent_drop() {
        let movement_command = ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29Movement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29Script".to_string(),
            command_index: 0,
        };

        let error = parse_script_movements(
            "Route29",
            &BTreeMap::new(),
            std::slice::from_ref(&movement_command),
        )
        .expect_err("missing movement scripts must not be skipped");
        assert!(
            format!("{error:#}").contains(
                "movement reference 'Route29Movement' from Route29Script on Route29 resolves to missing script"
            ),
            "{error:#}"
        );

        let scripts = BTreeMap::from([(
            "Route29Movement".to_string(),
            serde_json::json!({"command": "step_end"}),
        )]);
        let error =
            parse_script_movements("Route29", &scripts, std::slice::from_ref(&movement_command))
                .expect_err("non-array movement scripts must not be skipped");
        assert!(
            format!("{error:#}")
                .contains("movement script Route29Movement for Route29 must be an array"),
            "{error:#}"
        );

        let scripts = BTreeMap::from([(
            "Route29Movement".to_string(),
            serde_json::json!([{"args": []}]),
        )]);
        let error = parse_script_movements("Route29", &scripts, &[movement_command])
            .expect_err("movement entries without commands must not be skipped");
        assert!(
            format!("{error:#}").contains(
                "Malformed movement script Route29Movement for Route29: command 0 is missing command."
            ),
            "{error:#}"
        );

        let movement_command = ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29Movement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29Script".to_string(),
            command_index: 0,
        };
        let scripts = BTreeMap::from([(
            "Route29Movement".to_string(),
            serde_json::json!([
                {"command": "step", "args": ["DOWN"]}
            ]),
        )]);
        let error = parse_script_movements("Route29", &scripts, &[movement_command])
            .expect_err("movement scripts without step_end must not be accepted");
        assert!(
            format!("{error:#}").contains(
                "Malformed movement script Route29Movement for Route29: movement must end with step_end."
            ),
            "{error:#}"
        );

        let movement_command = ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29Movement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29Script".to_string(),
            command_index: 0,
        };
        let scripts = BTreeMap::from([(
            "Route29Movement".to_string(),
            serde_json::json!([
                {"command": "step_dig", "args": []},
                {"command": "step_end", "args": []}
            ]),
        )]);
        let error = parse_script_movements("Route29", &scripts, &[movement_command])
            .expect_err("duration-bearing movement commands must require exact duration");
        assert!(
            format!("{error:#}").contains(
                "Malformed step_dig movement in Route29Movement for Route29: expected 1 arg, found 0."
            ),
            "{error:#}"
        );

        let movement_command = ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29Movement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29Script".to_string(),
            command_index: 0,
        };
        let scripts = BTreeMap::from([(
            "Route29Movement@Route29Script".to_string(),
            serde_json::json!([
                {"command": "step_end", "args": []}
            ]),
        )]);
        let error = parse_script_movements("Route29", &scripts, &[movement_command])
            .expect_err("non-relative movement references must not infer local labels");
        assert!(
            format!("{error:#}").contains(
                "movement reference 'Route29Movement' from Route29Script on Route29 resolves to missing script"
            ),
            "{error:#}"
        );

        let movement_command = ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some(".SpinMovement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29Script".to_string(),
            command_index: 0,
        };
        let scripts = BTreeMap::from([
            (
                ".SpinMovement".to_string(),
                serde_json::json!([
                    {"command": "step_end", "args": []}
                ]),
            ),
            (
                ".SpinMovement@Route29Script".to_string(),
                serde_json::json!([
                    {"command": "step_end", "args": []}
                ]),
            ),
        ]);
        let error = parse_script_movements("Route29", &scripts, &[movement_command])
            .expect_err("ambiguous relative movement reference must reject");
        assert!(
            format!("{error:#}").contains(
                "ambiguous movement reference '.SpinMovement' from Route29Script on Route29"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn raw_script_movement_extraction_preserves_crystal_effect_durations() {
        let movement_command = ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("Route29Movement".to_string()),
            emote: None,
            duration: None,
            source_script: "Route29Script".to_string(),
            command_index: 0,
        };
        let scripts = BTreeMap::from([(
            "Route29Movement".to_string(),
            serde_json::json!([
                {"command": "turn_waterfall", "args": ["UP"]},
                {"command": "step_dig", "args": ["32"]},
                {"command": "step_sleep_8", "args": []},
                {"command": "rock_smash", "args": ["10"]},
                {"command": "return_dig", "args": ["32"]},
                {"command": "step_end", "args": []}
            ]),
        )]);

        let movements =
            parse_script_movements("Route29", &scripts, &[movement_command]).expect("movement");

        assert_eq!(movements.len(), 1);
        assert_eq!(movements[0].label, "Route29Movement");
        assert_eq!(movements[0].source_script.as_deref(), Some("Route29Script"));
        assert_eq!(
            movements[0]
                .steps
                .iter()
                .map(|step| {
                    (
                        step.command.as_str(),
                        step.direction.as_deref(),
                        step.duration,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("turn_waterfall", Some("UP"), None),
                ("step_dig", None, Some(32)),
                ("step_sleep_8", None, None),
                ("rock_smash", None, Some(10)),
                ("return_dig", None, Some(32)),
                ("step_end", None, None),
            ]
        );
    }

    #[test]
    fn raw_script_movement_extraction_binds_global_labels_to_exact_source_scripts() {
        let movement_command = |source_script: &str, command_index: usize| ScriptObjectCommand {
            command: "applymovement".to_string(),
            object_id: Some("PLAYER".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: Some("SharedSpinMovement".to_string()),
            emote: None,
            duration: None,
            source_script: source_script.to_string(),
            command_index,
        };
        let scripts = BTreeMap::from([(
            "SharedSpinMovement".to_string(),
            serde_json::json!([
                {"command": "turn_head", "args": ["LEFT"]},
                {"command": "turn_head", "args": ["UP"]},
                {"command": "step_end", "args": []}
            ]),
        )]);

        let movements = parse_script_movements(
            "CopycatsHouse2F",
            &scripts,
            &[
                movement_command(".Default_Female_1@Copycat", 0),
                movement_command(".GotPass_Female_1@Copycat", 1),
            ],
        )
        .expect("shared movement parses for each exact source script");

        assert_eq!(movements.len(), 2);
        assert_eq!(
            movements
                .iter()
                .map(|movement| {
                    (
                        movement.label.as_str(),
                        movement.source_script.as_deref(),
                        movement.steps.len(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("SharedSpinMovement", Some(".Default_Female_1@Copycat"), 3),
                ("SharedSpinMovement", Some(".GotPass_Female_1@Copycat"), 3),
            ]
        );
    }

    #[test]
    fn raw_script_text_body_extraction_rejects_malformed_payloads_without_silent_drop() {
        let scripts = BTreeMap::from([(
            "Route29Text".to_string(),
            serde_json::json!([
                {"command": "text", "args": "Hello."},
                {"args": []}
            ]),
        )]);
        let error = parse_script_text_bodies("Route29", &scripts)
            .expect_err("text body entries without commands must not be skipped");
        assert!(
            format!("{error:#}").contains(
                "Malformed text body command in Route29Text for Route29: command 1 is missing command."
            ),
            "{error:#}"
        );

        let scripts = BTreeMap::from([(
            "Route29Text".to_string(),
            serde_json::json!([
                {"command": "text", "args": "Hello."},
                {"command": "legacy_text", "args": []}
            ]),
        )]);
        let error = parse_script_text_bodies("Route29", &scripts)
            .expect_err("unknown text body commands must not be skipped");
        assert!(
            format!("{error:#}").contains(
                "Malformed text body command in Route29Text for Route29: unknown command 'legacy_text' at index 1."
            ),
            "{error:#}"
        );
    }

    #[test]
    fn raw_script_menu_extraction_rejects_malformed_candidates_without_silent_drop() {
        let scripts = BTreeMap::from([(
            "Route29Menu".to_string(),
            serde_json::json!([
                {"command": "menu_coords", "args": ["0", "0", "10", "8"]},
                {"args": []}
            ]),
        )]);
        let error = parse_script_menu_definitions("Route29", &scripts)
            .expect_err("menu-labeled scripts without commands must not be skipped");
        assert!(
            format!("{error:#}").contains(
                "Malformed menu definition command in Route29Menu for Route29: command must be a string."
            ),
            "{error:#}"
        );

        let scripts = BTreeMap::from([(
            "Route29Menu".to_string(),
            serde_json::json!([
                {"command": "menu_coords", "args": ["0", "0", "10", "8"]},
                {"args": []}
            ]),
        )]);
        let error = parse_script_menu_definitions("Route29", &scripts)
            .expect_err("menu entries without commands must not be skipped");
        assert!(
            format!("{error:#}").contains(
                "Malformed menu definition command in Route29Menu for Route29: command must be a string."
            ),
            "{error:#}"
        );

        let scripts = BTreeMap::from([(
            "Route29Menu".to_string(),
            serde_json::json!([
                {"command": "legacy_menu", "args": []},
                {"command": "menu_coords", "args": ["0", "0", "10", "8"]}
            ]),
        )]);
        let menus = parse_script_menu_definitions("Route29", &scripts)
            .expect("unknown commands before the menu definition must not hide exported menu data");
        let menu = menus.get("Route29Menu").expect("Route29 menu");
        assert_eq!(menu.commands.len(), 1);
        assert_eq!(menu.commands[0].command, "menu_coords");
        assert_eq!(menu.commands[0].command_index, 1);

        let scripts = BTreeMap::from([(
            "Route29Menu".to_string(),
            serde_json::json!([
                {"command": "menu_coords", "args": ["SCREEN_LEFT", "TEXTBOX_Y - %1", "SCREEN_WIDTH - $1", "SCREEN_HEIGHT - +1"]},
                {"command": "dw", "args": ["Route29MenuData"]}
            ]),
        )]);
        parse_script_menu_definitions("Route29", &scripts)
            .expect("exact ASM numeric menu coordinate expressions");

        let scripts = BTreeMap::from([(
            "Route29Menu".to_string(),
            serde_json::json!([
                {"command": "menu_coords", "args": ["$0", "%10", "$13", "%10001"]},
                {"command": "dw", "args": ["Route29MenuData"]}
            ]),
        )]);
        parse_script_menu_definitions("Route29", &scripts)
            .expect("exact standalone ASM numeric menu coordinates");

        let scripts = BTreeMap::from([(
            "Route29Menu".to_string(),
            serde_json::json!([
                {"command": "menu_coords", "args": ["0", "0", "SCREEN_EDGE + LEFT", "8"]},
                {"command": "dw", "args": ["Route29MenuData"]}
            ]),
        )]);
        let error = parse_script_menu_definitions("Route29", &scripts)
            .expect_err("unsupported menu coordinate expressions must fail extraction");
        assert!(
            format!("{error:#}").contains(
                "Malformed menu_coords command in Route29Menu for Route29: menu coordinate 2 must be an exact supported expression"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn raw_script_vertical_menu_extraction_exports_definitive_options() {
        let scripts = BTreeMap::from([
            (
                "Route29Script".to_string(),
                serde_json::json!([
                    {"command": "opentext", "args": []},
                    {"command": "loadmenu", "args": ["Route29Menu"]},
                    {"command": "verticalmenu", "args": []},
                    {"command": "closewindow", "args": []}
                ]),
            ),
            (
                "Route29Menu".to_string(),
                serde_json::json!([
                    {"command": "db", "args": ["MENU_BACKUP_TILES"]},
                    {"command": "menu_coords", "args": ["0", "0", "10", "8"]},
                    {"command": "dw", "args": ["Route29MenuData"]}
                ]),
            ),
            (
                "Route29MenuData".to_string(),
                serde_json::json!([
                    {"command": "db", "args": ["STATICMENU_CURSOR"]},
                    {"command": "db", "args": ["\"First@\""]},
                    {"command": "db", "args": ["\"Second@\""]}
                ]),
            ),
        ]);
        let menu_scripts = BTreeMap::from([
            (
                "Route29Menu".to_string(),
                scripts.get("Route29Menu").expect("menu").clone(),
            ),
            (
                "Route29MenuData".to_string(),
                scripts.get("Route29MenuData").expect("menu data").clone(),
            ),
        ]);
        let menus = parse_script_menu_definitions("Route29", &menu_scripts)
            .expect("parse script menu definitions");
        let vertical =
            parse_script_vertical_menus("Route29", &scripts, &menus).expect("parse vertical menus");
        let menu = vertical
            .get("Route29Script:2")
            .expect("vertical menu definition");
        assert_eq!(menu.source_script, "Route29Script");
        assert_eq!(menu.loadmenu_command_index, 1);
        assert_eq!(menu.verticalmenu_command_index, 2);
        assert_eq!(menu.header_label, "Route29Menu");
        assert_eq!(menu.data_label, Some("Route29MenuData".to_string()));
        assert_eq!(
            menu.options,
            vec!["First".to_string(), "Second".to_string()]
        );
    }

    #[test]
    fn raw_script_2d_menu_follows_separate_text_table() {
        let scripts = BTreeMap::from([
            (
                "AcademyBlackboard".to_string(),
                serde_json::json!([
                    {"command": "loadmenu", "args": [".Header"]},
                    {"command": "_2dmenu", "args": []}
                ]),
            ),
            (
                ".Header@AcademyBlackboard".to_string(),
                serde_json::json!([
                    {"command": "menu_coords", "args": ["0", "0", "11", "8"]},
                    {"command": "dw", "args": [".Data"]}
                ]),
            ),
            (
                ".Data@AcademyBlackboard".to_string(),
                serde_json::json!([
                    {"command": "dn", "args": ["1", "2"]},
                    {"command": "db", "args": ["5"]},
                    {"command": "dba", "args": [".Text"]}
                ]),
            ),
            (
                ".Text@AcademyBlackboard".to_string(),
                serde_json::json!([
                    {"command": "db", "args": ["\"PSN@\""]},
                    {"command": "db", "args": ["\"QUIT@\""]}
                ]),
            ),
        ]);
        let menus = parse_script_menu_definitions("EarlsPokemonAcademy", &scripts)
            .expect("parse 2D menu definitions");
        let vertical = parse_script_vertical_menus("EarlsPokemonAcademy", &scripts, &menus)
            .expect("parse 2D menu");
        let menu = vertical.get("AcademyBlackboard:1").expect("2D menu definition");
        assert_eq!(menu.options, vec!["PSN".to_string(), "QUIT".to_string()]);
        assert_eq!((menu.rows, menu.columns, menu.spacing), (Some(1), Some(2), Some(5)));
    }
