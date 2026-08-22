    #[test]
    fn content_pack_payloads_reject_duplicate_audio_asset_ids() {
        let mut data = GameDataSet {
            audio: vec![
                ModpackAudioAsset::music(
                    "MUSIC_DUPLICATE",
                    "content-packs/test/music/MUSIC_DUPLICATE.mid",
                )
                .expect("valid base audio asset"),
            ],
            ..GameDataSet::default()
        };

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Audio,
                serde_json::json!({
                    "MUSIC_DUPLICATE": {
                        "id": "MUSIC_DUPLICATE",
                        "path": "content-packs/test/music/MUSIC_DUPLICATE.mid",
                        "kind": "music",
                        "source": "midi",
                    }
                }),
            )
            .expect_err("duplicate audio asset payload must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate audio asset id 'MUSIC_DUPLICATE'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_audio_asset_keys_without_coercion() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Audio,
                serde_json::json!({
                    " MUSIC_ROUTE_29": {
                        "id": "MUSIC_ROUTE_29",
                        "path": "content-packs/test/music/MUSIC_ROUTE_29.mid",
                        "kind": "music",
                        "source": "midi",
                    }
                }),
            )
            .expect_err("audio keys must not be trimmed");
        assert!(
            format!("{error:#}").contains(
                "audio asset key ' MUSIC_ROUTE_29' must be an exact MUSIC_, SFX_, or CRY_ audio id"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Audio,
                serde_json::json!({
                    "ROUTE_29": {
                        "id": "MUSIC_ROUTE_29",
                        "path": "content-packs/test/music/MUSIC_ROUTE_29.mid",
                        "kind": "music",
                        "source": "midi",
                    }
                }),
            )
            .expect_err("audio keys must declare their audio namespace");
        assert!(
            format!("{error:#}").contains(
                "audio asset key 'ROUTE_29' must be an exact MUSIC_, SFX_, or CRY_ audio id"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Audio,
                serde_json::json!({
                    "MUSIC_ROUTE_29": {
                        "id": "MUSIC_ROUTE_30",
                        "path": "content-packs/test/music/MUSIC_ROUTE_30.mid",
                        "kind": "music",
                        "source": "midi",
                    }
                }),
            )
            .expect_err("audio key and record id must match exactly");
        assert!(
            format!("{error:#}").contains(
                "audio asset key 'MUSIC_ROUTE_29' does not match record id 'MUSIC_ROUTE_30'"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_exact_keyed_catalog_entries() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::PermanentPhoneNumbers,
            serde_json::json!({"PHONE_ELM": {"listIndex": 0}}),
        )
        .expect("first permanent phone payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PermanentPhoneNumbers,
                serde_json::json!({"PHONE_ELM": {"listIndex": 0}}),
            )
            .expect_err("duplicate permanent phone payload must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate permanent phone number 'PHONE_ELM'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::SpecialPhoneCalls,
            serde_json::json!({
                "SPECIALCALL_POKERUS": {
                    "value": 1,
                    "condition": "SpecialCallOnlyWhenOutside",
                    "contactId": "PHONE_ELM",
                    "callerScript": "ElmPhoneCallerScript"
                }
            }),
        )
        .expect("first special phone call payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::SpecialPhoneCalls,
                serde_json::json!({
                    "SPECIALCALL_POKERUS": {
                        "value": 1,
                        "condition": "SpecialCallOnlyWhenOutside",
                        "contactId": "PHONE_ELM",
                        "callerScript": "ElmPhoneCallerScript"
                    }
                }),
            )
            .expect_err("duplicate special phone call payload must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate special phone call 'SPECIALCALL_POKERUS'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PermanentPhoneNumbers,
                serde_json::json!({"PHONE MOM": {"listIndex": 0}}),
            )
            .expect_err("permanent phone number ids must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "permanent phone number 'PHONE MOM' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PermanentPhoneNumbers,
                serde_json::json!({
                    "contacts": {
                        "PHONE_ELM": {}
                    },
                    "fallback_contact": "PHONE_MOM"
                }),
            )
            .expect_err("permanent phone payload must be the compiler-emitted token map");
        assert!(
            format!("{error:#}").contains("parse permanent phone number payload"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpecialPhoneCalls,
                serde_json::json!({
                    "SPECIALCALL POKERUS": {
                        "value": 1,
                        "condition": "SpecialCallOnlyWhenOutside",
                        "contactId": "PHONE_ELM",
                        "callerScript": "ElmPhoneCallerScript"
                    }
                }),
            )
            .expect_err("special phone call ids must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "special phone call 'SPECIALCALL POKERUS' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpecialPhoneCalls,
                serde_json::json!({
                    "calls": {
                        "SPECIALCALL_POKERUS": {}
                    },
                    "fallback_call": "SPECIALCALL_NONE"
                }),
            )
            .expect_err("special phone payload must be the compiler-emitted token map");
        assert!(
            format!("{error:#}").contains("parse special phone call payload"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::NpcTrades,
            serde_json::json!({"NPC_TRADE_ONIX": {}}),
        )
        .expect("first NPC trade payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::NpcTrades,
                serde_json::json!({"NPC_TRADE_ONIX": {}}),
            )
            .expect_err("duplicate NPC trade payload must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate NPC trade 'NPC_TRADE_ONIX'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::NpcTrades,
                serde_json::json!({"NPC TRADE ONIX": {}}),
            )
            .expect_err("NPC trade ids must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "NPC trade 'NPC TRADE ONIX' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::NpcTrades,
                serde_json::json!({
                    "trades": {
                        "NPC_TRADE_ONIX": {}
                    },
                    "fallback_trade": "NPC_TRADE_NONE"
                }),
            )
            .expect_err("NPC trade payload must be the compiler-emitted token map");
        assert!(
            format!("{error:#}").contains("parse NPC trade payload"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::SpecialRoutines,
            serde_json::json!({"FadeOutMusic": {}}),
        )
        .expect("first special routine payload");
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::SpecialRoutines,
                serde_json::json!({"FadeOutMusic": {}}),
            )
            .expect_err("duplicate special routine payload must not be accepted");
        assert!(
            format!("{error:#}").contains("duplicate special routine 'FadeOutMusic'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpecialRoutines,
                serde_json::json!({"Fade Out Music": {}}),
            )
            .expect_err("special routine ids must be exact tokens");
        assert!(
            format!("{error:#}").contains(
                "special routine 'Fade Out Music' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpecialRoutines,
                serde_json::json!({"fallback_routine": {}}),
            )
            .expect_err("special routine fallback keys must be rejected as reserved payload ids");
        assert!(
            format!("{error:#}").contains(
                "special routine 'fallback_routine' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpecialRoutines,
                serde_json::json!({
                    "routines": {
                        "FadeOutMusic": {}
                    },
                    "fallback_routine": "NoOp"
                }),
            )
            .expect_err("special routine payload must be the compiler-emitted token map");
        assert!(
            format!("{error:#}").contains("parse special routine payload"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_legacy_array_catalog_entries() {
        for category in [
            ContentPackCategory::PermanentPhoneNumbers,
            ContentPackCategory::SpecialPhoneCalls,
            ContentPackCategory::NpcTrades,
            ContentPackCategory::SpecialRoutines,
        ] {
            let mut data = GameDataSet::default();
            let error = data
                .apply_content_pack_payload(category, serde_json::json!(["LEGACY_ENTRY"]))
                .expect_err("keyed catalog payloads must reject legacy arrays");
            assert!(
                format!("{error:#}").contains("invalid type: sequence"),
                "{error:#}"
            );
        }
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_story_event_payload_keys() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::StoryEvents,
            serde_json::json!({
                "Route29": {
                    "Route29_MapScripts": []
                }
            }),
        )
        .expect("initial story event payload should load");
        assert_eq!(
            data.story_events,
            vec![serde_json::json!({
                "Route29": {
                    "Route29_MapScripts": []
                }
            })]
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route29": {
                        "Route29_OtherScript": []
                    }
                }),
            )
            .expect_err("duplicate story event payload key must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate story event payload key 'Route29'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_phone_script_payload_keys() {
        let mut data = GameDataSet::default();
        data.apply_content_pack_payload(
            ContentPackCategory::PhoneScripts,
            serde_json::json!({
                "PhoneScript_Elm": []
            }),
        )
        .expect("initial phone script payload should load");
        assert_eq!(
            data.phone_scripts,
            vec![serde_json::json!({
                "PhoneScript_Elm": []
            })]
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "PhoneScript_Elm": []
                }),
            )
            .expect_err("duplicate phone script payload key must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate phone script payload key 'PhoneScript_Elm'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_raw_script_payload_keys_without_trimming() {
        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route 29": {
                        "Route29_MapScripts": []
                    }
                }),
            )
            .expect_err("story event payload keys must be exact");
        assert!(
            format!("{error:#}")
                .contains("story event payload 'Route 29' must be an exact map token"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "PhoneScript Elm": []
                }),
            )
            .expect_err("phone script payload keys must be exact");
        assert!(
            format!("{error:#}").contains(
                "phone script payload 'PhoneScript Elm' must be an exact script label token"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "fallbackPhoneScript": []
                }),
            )
            .expect_err("phone script payload keys must reject reserved payload ids");
        assert!(
            format!("{error:#}").contains(
                "phone script payload 'fallbackPhoneScript' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_raw_script_commands_without_fallbacks() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route29": {
                        "Route29 MapScripts": []
                    }
                }),
            )
            .expect_err("story event inner script keys must be exact");
        assert!(
            format!("{error:#}").contains(
                "story event script 'Route29 MapScripts' must be an exact script label token"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route29": {
                        "legacyStoryScript": []
                    }
                }),
            )
            .expect_err("story event inner script keys must reject reserved payload ids");
        assert!(
            format!("{error:#}").contains(
                "story event script 'legacyStoryScript' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route29": {
                        "Route29_MapScripts": [
                            {"command":"sjump","args":[" Route29Script"]}
                        ]
                    }
                }),
            )
            .expect_err("story event command args must be exact");
        assert!(
            format!("{error:#}").contains(
                "story event payload script 'Route29_MapScripts' command 0 arg 0 ' Route29Script' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route29": {
                        "Route29_MapScripts": [
                            {"command":"sjump","args":["Route29Script "]}
                        ]
                    }
                }),
            )
            .expect_err("story event command args must reject trimmed values");
        assert!(
            format!("{error:#}").contains(
                "story event payload script 'Route29_MapScripts' command 0 arg 0 'Route29Script ' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StoryEvents,
                serde_json::json!({
                    "Route29": {
                        "Route29_MapScripts": [
                            {"command":"sjump","args":["Route29Script"],"fallback_args":["DefaultScript"]}
                        ]
                    }
                }),
            )
            .expect_err("story event command objects must not carry fallback fields");
        assert!(
            format!("{error:#}").contains("unknown field `fallback_args`"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "PhoneScript_Elm": [
                        {"command":"sjump","args":[],"legacyArgs":[]}
                    ]
                }),
            )
            .expect_err("phone script command objects must not carry legacy fields");
        assert!(
            format!("{error:#}").contains("unknown field `legacyArgs`"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "PhoneScript_Elm": [
                        {"args":[]}
                    ]
                }),
            )
            .expect_err("phone script command objects must carry an explicit command name");
        assert!(
            format!("{error:#}").contains("missing field `command`"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "PhoneScript_Elm": [
                        {"command":" jump","args":[]}
                    ]
                }),
            )
            .expect_err("phone script command names must be exact");
        assert!(
            format!("{error:#}").contains(
                "phone script payload script 'PhoneScript_Elm' command 0 name ' jump' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::PhoneScripts,
                serde_json::json!({
                    "PhoneScript_Elm": [
                        {"command":"jump\nif_false","args":[]}
                    ]
                }),
            )
            .expect_err("phone script command names must reject control characters");
        let error = format!("{error:#}");
        assert!(
            error.contains("phone script payload script 'PhoneScript_Elm' command 0 name 'jump")
                && error.contains("if_false' must be exact, non-empty, and untrimmed"),
            "{error}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_encounter_map_names() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "NEW_ROUTE": {
                "map_name": "NEW_ROUTE",
                "grass_rates": null,
                "water_rate": null,
                "grass": null,
                "water": null
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::WildEncounters, payload.clone())
            .expect("initial wild encounter payload should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::WildEncounters, payload)
            .expect_err("duplicate wild encounter payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate wild encounter data for map 'NEW_ROUTE'"),
            "{error:#}"
        );

        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "NEW_ROUTE": {
                "map_name": "NEW_ROUTE",
                "tables": {}
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::FieldEncounters, payload.clone())
            .expect("initial field encounter payload should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::FieldEncounters, payload)
            .expect_err("duplicate field encounter payload must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate field encounter data for map 'NEW_ROUTE'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_roaming_pokemon_as_exact_pack_data() {
        let mut data = GameDataSet::default();
        let catalog = roaming_catalog_for_tests("RAIKOU", "ENTEI");

        data.apply_content_pack_payload(
            ContentPackCategory::RoamingPokemon,
            serde_json::to_value(&catalog).expect("serialize exact roaming catalog"),
        )
        .expect("apply roaming Pokemon payload");

        assert_eq!(data.roaming_pokemon, catalog);
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_roaming_pokemon_catalog() {
        let catalog = roaming_catalog_for_tests("RAIKOU", "ENTEI");
        let mut data = GameDataSet {
            roaming_pokemon: catalog.clone(),
            ..GameDataSet::default()
        };
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::RoamingPokemon,
                serde_json::to_value(catalog).expect("serialize duplicate roaming catalog"),
            )
            .expect_err("duplicate roaming Pokemon catalog must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate roaming Pokemon catalog"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_roaming_pokemon_at_load_time() {
        let mut half_zero = serde_json::to_value(roaming_catalog_for_tests("RAIKOU", "ENTEI"))
            .expect("serialize roaming catalog");
        half_zero["inactiveMap"] = serde_json::json!({ "mapGroup": 0, "mapNumber": 1 });
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RoamingPokemon,
                half_zero,
            )
            .expect_err("half-zero roaming inactiveMap must be rejected");
        assert!(
            format!("{error:#}").contains("inactiveMap must not be the pre-init"),
            "{error:#}"
        );

        let mut unknown = serde_json::to_value(roaming_catalog_for_tests("RAIKOU", "ENTEI"))
            .expect("serialize roaming catalog");
        unknown
            .as_object_mut()
            .expect("catalog object")
            .insert("fallbackRoamer".to_string(), serde_json::json!("ENTEI"));
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RoamingPokemon,
                unknown,
            )
            .expect_err("unknown roaming catalog fields must be rejected");
        assert!(
            format!("{error:#}").contains("unknown field `fallbackRoamer`"),
            "{error:#}"
        );

        let mut missing = serde_json::to_value(roaming_catalog_for_tests("RAIKOU", "ENTEI"))
            .expect("serialize roaming catalog");
        missing
            .as_object_mut()
            .expect("catalog object")
            .remove("slotCount");
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::RoamingPokemon,
                missing,
            )
            .expect_err("roaming catalog cannot infer a slot count");
        assert!(
            format!("{error:#}").contains("missing field `slotCount`"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_buena_prizes_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BuenaPrizes,
            serde_json::json!({
                "RARE_CANDY": 3
            }),
        )
        .expect("apply Buena prizes payload");

        assert_eq!(
            data.buena_prizes,
            BTreeMap::from([("RARE_CANDY".to_string(), 3)])
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_buena_prize_item_ids() {
        let mut data = GameDataSet::default();
        data.buena_prizes.insert("RARE_CANDY".to_string(), 3);
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BuenaPrizes,
                serde_json::json!({
                    "RARE_CANDY": 5
                }),
            )
            .expect_err("duplicate Buena prize item id must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate Buena prize item id 'RARE_CANDY'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_buena_prizes_at_load_time() {
        let cases = vec![
            (
                "malformed item id",
                serde_json::json!({ "RARE CANDY": 3 }),
                "Buena prize item id 'RARE CANDY' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "zero cost",
                serde_json::json!({ "RARE_CANDY": 0 }),
                "Buena prize item 'RARE_CANDY' cost must be nonzero",
            ),
            (
                "reserved fallback item id",
                serde_json::json!({ "fallback_cost": 3 }),
                "Buena prize item id 'fallback_cost' uses reserved modpack payload prefix",
            ),
        ];

        for (label, payload, expected) in cases {
            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::BuenaPrizes, payload)
                .expect_err(label);

            assert!(
                format!("{error:#}").contains(expected),
                "{label} produced unexpected error: {error:#}"
            );
        }
    }

    #[test]
    fn content_pack_payloads_merge_buena_password_categories_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BuenaPasswordCategories,
            serde_json::json!({
                "order": ["HealingItems"],
                "categories": {
                  "HealingItems": {
                    "categoryType": "BUENA_ITEM",
                    "points": 12,
                    "options": ["POTION", "ANTIDOTE", "PARLYZ_HEAL"]
                  }
                }
            }),
        )
        .expect("apply Buena password category payload");

        assert_eq!(
            data.buena_password_categories,
            BuenaPasswordCategories {
                order: vec!["HealingItems".to_string()],
                categories: BTreeMap::from([(
                    "HealingItems".to_string(),
                    BuenaPasswordCategoryDefinition {
                        category_type: "BUENA_ITEM".to_string(),
                        points: 12,
                        options: vec![
                            "POTION".to_string(),
                            "ANTIDOTE".to_string(),
                            "PARLYZ_HEAL".to_string()
                        ],
                    }
                )]),
            }
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_buena_password_category_ids() {
        let mut data = GameDataSet::default();
        data.buena_password_categories
            .order
            .push("HealingItems".to_string());
        data.buena_password_categories.categories.insert(
            "HealingItems".to_string(),
            BuenaPasswordCategoryDefinition {
                category_type: "BUENA_ITEM".to_string(),
                points: 12,
                options: vec!["POTION".to_string()],
            },
        );
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BuenaPasswordCategories,
                serde_json::json!({
                    "order": ["HealingItems"],
                    "categories": {
                      "HealingItems": {
                        "categoryType": "BUENA_ITEM",
                        "points": 10,
                        "options": ["ANTIDOTE"]
                      }
                    }
                }),
            )
            .expect_err("duplicate Buena password category id must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate Buena password category id 'HealingItems'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_buena_password_categories_at_load_time() {
        let cases = vec![
            (
                "buena password order id must be a nonempty exact pack token",
                serde_json::json!({
                    "order": ["Healing Items"],
                    "categories": {
                      "Healing Items": {
                        "categoryType": "BUENA_ITEM",
                        "points": 12,
                        "options": ["POTION"]
                      }
                    }
                }),
                "buena password order id must be a nonempty exact pack token",
            ),
            (
                "extra category not in order",
                serde_json::json!({
                    "order": ["HealingItems"],
                    "categories": {
                      "HealingItems": {
                        "categoryType": "BUENA_ITEM",
                        "points": 12,
                        "options": ["POTION"]
                      },
                      "ExtraItems": {
                        "categoryType": "BUENA_ITEM",
                        "points": 12,
                        "options": ["ANTIDOTE"]
                      }
                    }
                }),
                "buena password category id \"ExtraItems\" missing from order",
            ),
            (
                "unknown category type",
                serde_json::json!({
                    "order": ["HealingItems"],
                    "categories": {
                      "HealingItems": {
                        "categoryType": "BUENA_BERRY",
                        "points": 12,
                        "options": ["POTION"]
                      }
                    }
                }),
                "unknown buena password categoryType \"BUENA_BERRY\"",
            ),
            (
                "zero points",
                serde_json::json!({
                    "order": ["HealingItems"],
                    "categories": {
                      "HealingItems": {
                        "categoryType": "BUENA_ITEM",
                        "points": 0,
                        "options": ["POTION"]
                      }
                    }
                }),
                "buena password category points must be nonzero",
            ),
            (
                "malformed option",
                serde_json::json!({
                    "order": ["HealingItems"],
                    "categories": {
                      "HealingItems": {
                        "categoryType": "BUENA_ITEM",
                        "points": 12,
                        "options": ["SUPER POTION"]
                      }
                    }
                }),
                "Buena password category 'HealingItems' option 0 'SUPER POTION' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "reserved legacy category id",
                serde_json::json!({
                    "order": ["legacyCategory"],
                    "categories": {
                      "legacyCategory": {
                        "categoryType": "BUENA_ITEM",
                        "points": 12,
                        "options": ["POTION"]
                      }
                    }
                }),
                "buena password order id must be a nonempty exact pack token",
            ),
            (
                "empty category table",
                serde_json::json!({
                    "order": [],
                    "categories": {}
                }),
                "buena password order must not be empty",
            ),
        ];

        for (label, payload, expected) in cases {
            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::BuenaPasswordCategories, payload)
                .expect_err(label);

            assert!(
                format!("{error:#}").contains(expected),
                "{label} produced unexpected error: {error:#}"
            );
        }
    }

    #[test]
    fn content_pack_payloads_merge_kurt_apricorn_recipes_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::KurtApricornRecipes,
            serde_json::json!({
                "RED_APRICORN": "LEVEL_BALL"
            }),
        )
        .expect("apply Kurt apricorn recipe payload");

        assert_eq!(
            data.kurt_apricorn_recipes,
            BTreeMap::from([("RED_APRICORN".to_string(), "LEVEL_BALL".to_string())])
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_kurt_apricorn_recipe_ids() {
        let mut data = GameDataSet::default();
        data.kurt_apricorn_recipes
            .insert("RED_APRICORN".to_string(), "LEVEL_BALL".to_string());
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::KurtApricornRecipes,
                serde_json::json!({
                    "RED_APRICORN": "FRIEND_BALL"
                }),
            )
            .expect_err("duplicate Kurt apricorn recipe must not be accepted");

        assert!(
            format!("{error:#}")
                .contains("duplicate Kurt apricorn recipe for apricorn 'RED_APRICORN'"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_kurt_apricorn_recipes_at_load_time() {
        let cases = vec![
            (
                "malformed apricorn id",
                serde_json::json!({ "RED APRICORN": "LEVEL_BALL" }),
                "Kurt apricorn recipe apricorn id 'RED APRICORN' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "malformed ball id",
                serde_json::json!({ "RED_APRICORN": "LEVEL BALL" }),
                "Kurt apricorn recipe ball id 'LEVEL BALL' must be exact ASCII alphanumeric or underscore",
            ),
            (
                "reserved fallback apricorn id",
                serde_json::json!({ "fallback_apricorn": "LEVEL_BALL" }),
                "Kurt apricorn recipe apricorn id 'fallback_apricorn' uses reserved modpack payload prefix",
            ),
            (
                "reserved fallback ball id",
                serde_json::json!({ "RED_APRICORN": "fallback_ball" }),
                "Kurt apricorn recipe ball id 'fallback_ball' uses reserved modpack payload prefix",
            ),
        ];

        for (label, payload, expected) in cases {
            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::KurtApricornRecipes, payload)
                .expect_err(label);

            assert!(
                format!("{error:#}").contains(expected),
                "{label} produced unexpected error: {error:#}"
            );
        }
    }

    #[test]
    fn content_pack_payloads_merge_shuckie_gift_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::ShuckieGift,
            serde_json::json!({
                "species": "SHUCKLE",
                "level": 15,
                "heldItem": "BERRY",
                "nickname": "SHUCKIE",
                "originalTrainerName": "MANIA",
                "originalTrainerId": 518,
                "gotTodayEngineFlag": "ENGINE_GOT_SHUCKIE_TODAY"
            }),
        )
        .expect("apply Shuckie gift payload");

        assert_eq!(
            data.shuckie_gift,
            Some(ShuckieGiftDefinition {
                species: "SHUCKLE".to_string(),
                level: 15,
                held_item: "BERRY".to_string(),
                nickname: "SHUCKIE".to_string(),
                original_trainer_name: "MANIA".to_string(),
                original_trainer_id: 518,
                got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
            })
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_shuckie_gift_definitions() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "species": "SHUCKLE",
            "level": 15,
            "heldItem": "BERRY",
            "nickname": "SHUCKIE",
            "originalTrainerName": "MANIA",
            "originalTrainerId": 518,
            "gotTodayEngineFlag": "ENGINE_GOT_SHUCKIE_TODAY"
        });
        data.apply_content_pack_payload(ContentPackCategory::ShuckieGift, payload.clone())
            .expect("initial Shuckie gift should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::ShuckieGift, payload)
            .expect_err("duplicate Shuckie gift must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Shuckie gift definition"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_shuckie_gift_definitions() {
        for (field, value, expected) in [
            (
                "species",
                serde_json::json!("SHUCKLE "),
                "shuckie species must be a nonempty exact pack token",
            ),
            (
                "species",
                serde_json::json!("legacyShuckie"),
                "shuckie species must be a nonempty exact pack token",
            ),
            (
                "level",
                serde_json::json!(0),
                "shuckie level must be 1..100",
            ),
            (
                "heldItem",
                serde_json::json!("BERRY JUICE"),
                "shuckie heldItem must be a nonempty exact pack token",
            ),
            (
                "heldItem",
                serde_json::json!("fallbackBerry"),
                "shuckie heldItem must be a nonempty exact pack token",
            ),
            (
                "nickname",
                serde_json::json!(""),
                "shuckie nickname must be nonempty exact text",
            ),
            (
                "originalTrainerName",
                serde_json::json!(" MANIA"),
                "shuckie originalTrainerName must be nonempty exact text",
            ),
            (
                "gotTodayEngineFlag",
                serde_json::json!("ENGINE GOT SHUCKIE TODAY"),
                "shuckie gotTodayEngineFlag must be a nonempty exact pack token",
            ),
        ] {
            let mut payload = serde_json::json!({
                "species": "SHUCKLE",
                "level": 15,
                "heldItem": "BERRY",
                "nickname": "SHUCKIE",
                "originalTrainerName": "MANIA",
                "originalTrainerId": 518,
                "gotTodayEngineFlag": "ENGINE_GOT_SHUCKIE_TODAY"
            });
            payload[field] = value;

            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::ShuckieGift, payload)
                .expect_err("invalid Shuckie gift must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
    }

    #[test]
    fn content_pack_payloads_merge_dratini_move_sets_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::DratiniMoveSets,
            serde_json::json!({
                "0": ["WRAP", "THUNDER_WAVE", "TWISTER", "EXTREMESPEED"],
                "1": ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"]
            }),
        )
        .expect("apply Dratini move sets payload");

        assert_eq!(
            data.dratini_move_sets,
            BTreeMap::from([
                (
                    0,
                    vec![
                        "WRAP".to_string(),
                        "THUNDER_WAVE".to_string(),
                        "TWISTER".to_string(),
                        "EXTREMESPEED".to_string()
                    ],
                ),
                (
                    1,
                    vec![
                        "WRAP".to_string(),
                        "LEER".to_string(),
                        "THUNDER_WAVE".to_string(),
                        "TWISTER".to_string()
                    ],
                ),
            ])
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_dratini_move_set_modes() {
        let mut data = GameDataSet::default();
        data.dratini_move_sets
            .insert(0, vec!["WRAP".to_string(), "THUNDER_WAVE".to_string()]);
        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::DratiniMoveSets,
                serde_json::json!({
                    "0": ["LEER", "TWISTER"]
                }),
            )
            .expect_err("duplicate Dratini move set mode must not be accepted");

        assert!(
            format!("{error:#}").contains("duplicate Dratini move set mode 0"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_dratini_move_sets() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::DratiniMoveSets,
                serde_json::json!({
                    "0": []
                }),
            )
            .expect_err("empty Dratini move sets must fail during pack load");
        assert!(
            format!("{error:#}").contains("must not be empty"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::DratiniMoveSets,
                serde_json::json!({
                    "0": ["THUNDER WAVE"]
                }),
            )
            .expect_err("malformed Dratini move ids must fail during pack load");
        assert!(
            format!("{error:#}").contains("Dratini move set move id"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::DratiniMoveSets,
                serde_json::json!({
                    "0": ["legacyMove"]
                }),
            )
            .expect_err("reserved Dratini move ids must fail during pack load");
        assert!(
            format!("{error:#}").contains(
                "Dratini move set move id 'legacyMove' uses reserved modpack payload prefix"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_bug_contest_config_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BugContestConfig,
            serde_json::json!({
                "parkBalls": 20,
                "timerMinutes": 20,
                "timerSeconds": 0,
                "selectedContestantCount": 2,
                "contestantFlags": [
                    "EVENT_BUG_CATCHING_CONTESTANT_1A",
                    "EVENT_BUG_CATCHING_CONTESTANT_2A"
                ],
                "encounters": serde_json::to_value(bug_contest_encounters_for_tests())
                    .expect("serialize Bug Contest encounters")
            }),
        )
        .expect("apply Bug-Catching Contest config payload");

        assert_eq!(
            data.bug_contest_config,
            Some(BugContestConfig {
                park_balls: 20,
                timer_minutes: 20,
                timer_seconds: 0,
                selected_contestant_count: 2,
                contestant_flags: vec![
                    "EVENT_BUG_CATCHING_CONTESTANT_1A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_2A".to_string()
                ],
                encounters: bug_contest_encounters_for_tests(),
            })
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_bug_contest_config() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "parkBalls": 20,
            "timerMinutes": 20,
            "timerSeconds": 0,
            "selectedContestantCount": 1,
            "contestantFlags": ["EVENT_BUG_CATCHING_CONTESTANT_1A"],
            "encounters": serde_json::to_value(bug_contest_encounters_for_tests())
                .expect("serialize Bug Contest encounters")
        });
        data.apply_content_pack_payload(ContentPackCategory::BugContestConfig, payload.clone())
            .expect("initial Bug-Catching Contest config should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BugContestConfig, payload)
            .expect_err("duplicate Bug-Catching Contest config must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Bug-Catching Contest config"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_bug_contest_config() {
        for (field, value, expected) in [
            (
                "parkBalls",
                serde_json::json!(0),
                "parkBalls must be nonzero",
            ),
            (
                "timerSeconds",
                serde_json::json!(60),
                "timerSeconds must be 0..59",
            ),
            (
                "selectedContestantCount",
                serde_json::json!(0),
                "selectedContestantCount must be nonzero",
            ),
        ] {
            let mut payload = serde_json::json!({
                "parkBalls": 20,
                "timerMinutes": 20,
                "timerSeconds": 0,
                "selectedContestantCount": 1,
                "contestantFlags": ["EVENT_BUG_CATCHING_CONTESTANT_1A"],
                "encounters": serde_json::to_value(bug_contest_encounters_for_tests())
                    .expect("serialize Bug Contest encounters")
            });
            payload[field] = value;

            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::BugContestConfig, payload)
                .expect_err("invalid Bug-Catching Contest config must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }

        for (contestant_flags, expected) in [
            (
                serde_json::json!([]),
                "selectedContestantCount 1 exceeds contestant flag count 0",
            ),
            (
                serde_json::json!(["EVENT_BUG_CATCHING_CONTESTANT_1A "]),
                "bug contest contestantFlags[0] must be a nonempty exact pack token",
            ),
            (
                serde_json::json!([
                    "EVENT_BUG_CATCHING_CONTESTANT_1A",
                    "EVENT_BUG_CATCHING_CONTESTANT_1A"
                ]),
                "bug contest contestantFlags[1] duplicates",
            ),
        ] {
            let error = GameDataSet::default()
                .apply_content_pack_payload(
                    ContentPackCategory::BugContestConfig,
                    serde_json::json!({
                        "parkBalls": 20,
                        "timerMinutes": 20,
                        "timerSeconds": 0,
                        "selectedContestantCount": 1,
                        "contestantFlags": contestant_flags,
                        "encounters": serde_json::to_value(bug_contest_encounters_for_tests())
                            .expect("serialize Bug Contest encounters")
                    }),
                )
                .expect_err("invalid Bug-Catching Contest contestant flags must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
    }

    #[test]
    fn content_pack_payloads_merge_battle_tower_rules_as_exact_pack_data() {
        let mut data = GameDataSet::default();
        let rules = test_battle_tower_rules();

        data.apply_content_pack_payload(
            ContentPackCategory::BattleTowerRules,
            serde_json::to_value(&rules).expect("serialize Battle Tower rules fixture"),
        )
        .expect("apply Battle Tower rules payload");

        assert_eq!(data.battle_tower_rules, Some(rules));
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_battle_tower_rules() {
        let mut data = GameDataSet::default();
        let payload = serde_json::to_value(test_battle_tower_rules())
            .expect("serialize Battle Tower rules fixture");
        data.apply_content_pack_payload(ContentPackCategory::BattleTowerRules, payload.clone())
            .expect("initial Battle Tower rules should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BattleTowerRules, payload)
            .expect_err("duplicate Battle Tower rules must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Battle Tower rules"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_battle_tower_rules() {
        for (field, value, expected) in [
            (
                "requiredPartyCount",
                serde_json::json!(0),
                "requiredPartyCount must be nonzero",
            ),
            (
                "challengeStreakLength",
                serde_json::json!(0),
                "challengeStreakLength must be nonzero",
            ),
            (
                "levelGroupSize",
                serde_json::json!(0),
                "levelGroupSize must be nonzero",
            ),
            (
                "minimumLevelGroup",
                serde_json::json!(0),
                "level group range must be nonzero and ordered",
            ),
            (
                "partyCountFailureText",
                serde_json::json!(" OnlyThreeMonMayBeEnteredText"),
                "partyCountFailureText must be a nonempty exact pack token",
            ),
            (
                "duplicateSpeciesFailureText",
                serde_json::json!("The Mon Must All Be Different Kinds Text"),
                "duplicateSpeciesFailureText must be a nonempty exact pack token",
            ),
            (
                "duplicateHeldItemFailureText",
                serde_json::json!("TheMonMustNotHoldTheSameItemsText\n"),
                "duplicateHeldItemFailureText must be a nonempty exact pack token",
            ),
            (
                "eggFailureText",
                serde_json::json!(""),
                "eggFailureText must be a nonempty exact pack token",
            ),
        ] {
            let mut payload = serde_json::to_value(test_battle_tower_rules())
                .expect("serialize Battle Tower rules fixture");
            payload[field] = value;

            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::BattleTowerRules, payload)
                .expect_err("invalid Battle Tower rules must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }

        let mut payload = serde_json::to_value(test_battle_tower_rules())
            .expect("serialize Battle Tower rules fixture");
        payload["bannedSpecies"] = serde_json::json!({ "MEW TWO": {} });
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::BattleTowerRules, payload)
            .expect_err("invalid Battle Tower banned species key must be rejected");

        assert!(
            format!("{error:#}")
                .contains("battle tower bannedSpecies key must be a nonempty exact pack token"),
            "{error:#}"
        );

        let mut payload = serde_json::to_value(test_battle_tower_rules())
            .expect("serialize Battle Tower rules fixture");
        payload["bannedSpecies"] = serde_json::json!({ "legacyMewtwo": {} });
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::BattleTowerRules, payload)
            .expect_err("reserved Battle Tower banned species keys must be rejected");

        assert!(
            format!("{error:#}")
                .contains("battle tower bannedSpecies key must be a nonempty exact pack token"),
            "{error:#}"
        );

        let mut payload = serde_json::to_value(test_battle_tower_rules())
            .expect("serialize Battle Tower rules fixture");
        payload["bannedSpecies"] = serde_json::json!({ "MEWTWO": {} });
        payload["partyCountFailureText"] = serde_json::json!("fallbackPartyText");
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::BattleTowerRules, payload)
            .expect_err("reserved Battle Tower failure text ids must be rejected");

        assert!(
            format!("{error:#}")
                .contains("battle tower partyCountFailureText must be a nonempty exact pack token"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_oak_ratings_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::OakRatings,
            serde_json::json!([
                {
                    "caughtCountLimit": 9,
                    "fanfare": "SFX_DEX_FANFARE_LESS_THAN_20",
                    "textLabel": "OakRating01"
                }
            ]),
        )
        .expect("apply Oak ratings payload");

        assert_eq!(
            data.oak_ratings,
            vec![OakRatingEntry {
                caught_count_limit: 9,
                fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                text_label: "OakRating01".to_string(),
            }]
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_oak_rating_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!([
            {
                "caughtCountLimit": 9,
                "fanfare": "SFX_DEX_FANFARE_LESS_THAN_20",
                "textLabel": "OakRating01"
            }
        ]);
        data.apply_content_pack_payload(ContentPackCategory::OakRatings, payload.clone())
            .expect("initial Oak rating table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::OakRatings, payload)
            .expect_err("duplicate Oak rating table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Oak rating table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_oak_rating_tables() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OakRatings,
                serde_json::json!([
                    {
                        "caughtCountLimit": 9,
                        "fanfare": "SFX DEX_FANFARE",
                        "textLabel": "OakRating01"
                    }
                ]),
            )
            .expect_err("malformed Oak rating fanfare ids must fail during pack load");
        assert!(
            format!("{error:#}").contains("oak rating fanfare must be a nonempty exact pack token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OakRatings,
                serde_json::json!([
                    {
                        "caughtCountLimit": 9,
                        "fanfare": "fallbackFanfare",
                        "textLabel": "OakRating01"
                    }
                ]),
            )
            .expect_err("reserved Oak rating fanfare ids must fail during pack load");
        assert!(
            format!("{error:#}").contains("oak rating fanfare must be a nonempty exact pack token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OakRatings,
                serde_json::json!([
                    {
                        "caughtCountLimit": 9,
                        "fanfare": "SFX_DEX_FANFARE",
                        "textLabel": "legacyOakText"
                    }
                ]),
            )
            .expect_err("reserved Oak rating text labels must fail during pack load");
        assert!(
            format!("{error:#}")
                .contains("oak rating textLabel must be a nonempty exact pack token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OakRatings,
                serde_json::json!([
                    {
                        "caughtCountLimit": 9,
                        "fanfare": "SFX_DEX_FANFARE",
                        "textLabel": "OakRating01"
                    },
                    {
                        "caughtCountLimit": 9,
                        "fanfare": "SFX_DEX_FANFARE_2",
                        "textLabel": "OakRating02"
                    }
                ]),
            )
            .expect_err("unordered Oak rating limits must fail during pack load");
        assert!(
            format!("{error:#}").contains("caught_count_limit must increase"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_odd_egg_definitions_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::OddEggDefinitions,
            serde_json::json!([
                {
                    "species": "CLEFFA",
                    "moves": ["POUND", "CHARM", "DIZZY_PUNCH"],
                    "originalTrainerId": 768,
                    "dvs": [2, 10, 10, 10],
                    "probability": 100,
                    "level": 5,
                    "experience": 125,
                    "hatchCycles": 20,
                    "nickname": "EGG",
                    "originalTrainerName": "ODD"
                }
            ]),
        )
        .expect("apply Odd Egg definitions payload");

        assert_eq!(
            data.odd_egg_definitions,
            vec![OddEggDefinition {
                species: "CLEFFA".to_string(),
                moves: vec![
                    "POUND".to_string(),
                    "CHARM".to_string(),
                    "DIZZY_PUNCH".to_string()
                ],
                original_trainer_id: 768,
                dvs: [2, 10, 10, 10],
                probability: 100,
                level: 5,
                experience: 125,
                hatch_cycles: 20,
                nickname: "EGG".to_string(),
                original_trainer_name: "ODD".to_string(),
            }]
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_odd_egg_definitions_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!([
            {
                "species": "CLEFFA",
                "moves": ["POUND", "CHARM", "DIZZY_PUNCH"],
                "originalTrainerId": 768,
                "dvs": [2, 10, 10, 10],
                "probability": 100,
                "level": 5,
                "experience": 125,
                "hatchCycles": 20,
                "nickname": "EGG",
                "originalTrainerName": "ODD"
            }
        ]);
        data.apply_content_pack_payload(ContentPackCategory::OddEggDefinitions, payload.clone())
            .expect("initial Odd Egg definitions table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::OddEggDefinitions, payload)
            .expect_err("duplicate Odd Egg definitions table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Odd Egg definitions table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_odd_egg_definitions() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OddEggDefinitions,
                serde_json::json!([
                    {
                        "species": "CLEFFA",
                        "moves": ["POUND"],
                        "originalTrainerId": 768,
                        "dvs": [2, 10, 10, 10],
                        "probability": 99,
                        "level": 5,
                        "experience": 125,
                        "hatchCycles": 20,
                        "nickname": "EGG",
                        "originalTrainerName": "ODD"
                    }
                ]),
            )
            .expect_err("invalid Odd Egg probability totals must fail during pack load");
        assert!(
            format!("{error:#}").contains("probabilities must total 100"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OddEggDefinitions,
                serde_json::json!([
                    {
                        "species": "CLEFFA",
                        "moves": ["PO UND"],
                        "originalTrainerId": 768,
                        "dvs": [2, 10, 10, 10],
                        "probability": 100,
                        "level": 5,
                        "experience": 125,
                        "hatchCycles": 20,
                        "nickname": "EGG",
                        "originalTrainerName": "ODD"
                    }
                ]),
            )
            .expect_err("malformed Odd Egg move ids must fail during pack load");
        assert!(
            format!("{error:#}").contains("odd egg moves[0] must be a nonempty exact pack token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OddEggDefinitions,
                serde_json::json!([
                    {
                        "species": "fallbackCleffa",
                        "moves": ["POUND"],
                        "originalTrainerId": 768,
                        "dvs": [2, 10, 10, 10],
                        "probability": 100,
                        "level": 5,
                        "experience": 125,
                        "hatchCycles": 20,
                        "nickname": "EGG",
                        "originalTrainerName": "ODD"
                    }
                ]),
            )
            .expect_err("reserved Odd Egg species ids must fail during pack load");
        assert!(
            format!("{error:#}").contains("odd egg species must be a nonempty exact pack token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OddEggDefinitions,
                serde_json::json!([
                    {
                        "species": "CLEFFA",
                        "moves": ["legacyPunch"],
                        "originalTrainerId": 768,
                        "dvs": [2, 10, 10, 10],
                        "probability": 100,
                        "level": 5,
                        "experience": 125,
                        "hatchCycles": 20,
                        "nickname": "EGG",
                        "originalTrainerName": "ODD"
                    }
                ]),
            )
            .expect_err("reserved Odd Egg move ids must fail during pack load");
        assert!(
            format!("{error:#}").contains("odd egg moves[0] must be a nonempty exact pack token"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OddEggDefinitions,
                serde_json::json!([
                    {
                        "species": "CLEFFA",
                        "moves": ["POUND"],
                        "originalTrainerId": 768,
                        "dvs": [2, 10, 10, 10],
                        "probability": 100,
                        "level": 0,
                        "experience": 125,
                        "hatchCycles": 20,
                        "nickname": "EGG",
                        "originalTrainerName": "ODD"
                    }
                ]),
            )
            .expect_err("invalid Odd Egg levels must fail during pack load");
        assert!(
            format!("{error:#}").contains("odd egg level must be 1..100"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::OddEggDefinitions,
                serde_json::json!([
                    {
                        "species": "CLEFFA",
                        "moves": ["POUND"],
                        "originalTrainerId": 768,
                        "dvs": [2, 10, 10, 10],
                        "probability": 100,
                        "level": 5,
                        "experience": 125,
                        "hatchCycles": 20,
                        "nickname": " EGG",
                        "originalTrainerName": "ODD"
                    }
                ]),
            )
            .expect_err("padded Odd Egg nicknames must fail during pack load");
        assert!(
            format!("{error:#}").contains("odd egg nickname must be nonempty exact text"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_magikarp_lengths_as_exact_pack_data() {
        let mut data = GameDataSet::default();
        let lengths = magikarp_lengths_for_tests();

        data.apply_content_pack_payload(
            ContentPackCategory::MagikarpLengths,
            serde_json::to_value(&lengths).expect("serialize exact Magikarp table"),
        )
        .expect("apply Magikarp length table payload");

        assert_eq!(data.magikarp_lengths, lengths);
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_magikarp_length_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::to_value(magikarp_lengths_for_tests())
            .expect("serialize exact Magikarp table");
        data.apply_content_pack_payload(ContentPackCategory::MagikarpLengths, payload.clone())
            .expect("initial Magikarp length table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::MagikarpLengths, payload)
            .expect_err("duplicate Magikarp length table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate Magikarp length table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_magikarp_length_tables() {
        let mut zero_divisor = serde_json::to_value(magikarp_lengths_for_tests())
            .expect("serialize exact Magikarp table");
        zero_divisor[0]["divisor"] = serde_json::json!(0);
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MagikarpLengths,
                zero_divisor,
            )
            .expect_err("zero Magikarp length divisors must fail during pack load");
        assert!(
            format!("{error:#}").contains("divisor must be nonzero"),
            "{error:#}"
        );

        let mut oversized_divisor = serde_json::to_value(magikarp_lengths_for_tests())
            .expect("serialize exact Magikarp table");
        oversized_divisor[0]["divisor"] = serde_json::json!(256);
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::MagikarpLengths, oversized_divisor)
            .expect_err("Magikarp divisors larger than one byte must fail during pack load");
        assert!(
            format!("{error:#}").contains("divisor must fit one source byte"),
            "{error:#}"
        );

        let mut unordered = magikarp_lengths_for_tests();
        unordered.swap(0, 1);
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MagikarpLengths,
                serde_json::to_value(unordered).expect("serialize unordered table"),
            )
            .expect_err("unordered Magikarp length thresholds must fail during pack load");
        assert!(
            format!("{error:#}").contains("threshold must increase"),
            "{error:#}"
        );

        let short = serde_json::to_value(&magikarp_lengths_for_tests()[..13])
            .expect("serialize short Magikarp table");
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::MagikarpLengths, short)
            .expect_err("Magikarp table must contain exactly fourteen source rows");
        assert!(
            format!("{error:#}").contains("exactly 14 source rows"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_happiness_data_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::HappinessData,
            serde_json::json!({
                "changes": {
                    "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 }
                },
                "services": {
                    "DaisysGrooming": [
                        { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                    ]
                }
            }),
        )
        .expect("apply happiness data payload");

        assert_eq!(
            data.happiness_data,
            Some(HappinessData {
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
            })
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_happiness_data_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "changes": {
                "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 }
            },
            "services": {
                "DaisysGrooming": [
                    { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                ]
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::HappinessData, payload.clone())
            .expect("initial happiness data table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::HappinessData, payload)
            .expect_err("duplicate happiness data table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate happiness data table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_happiness_data_table() {
        for (payload, expected) in [
            (
                serde_json::json!({
                    "changes": {},
                    "services": {
                        "DaisysGrooming": [
                            { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                        ]
                    }
                }),
                "happiness changes must not be empty",
            ),
            (
                serde_json::json!({
                    "changes": {
                        "18": { "code": "HAPPINESS GROOMING", "low": 3, "mid": 3, "high": 1 }
                    },
                    "services": {
                        "DaisysGrooming": [
                            { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                        ]
                    }
                }),
                "happiness change code must be a nonempty exact pack token",
            ),
            (
                serde_json::json!({
                    "changes": {
                        "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 },
                        "19": { "code": "HAPPINESS_GROOMING", "low": 5, "mid": 5, "high": 2 }
                    },
                    "services": {
                        "DaisysGrooming": [
                            { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                        ]
                    }
                }),
                "duplicates code name HAPPINESS_GROOMING",
            ),
            (
                serde_json::json!({
                    "changes": {
                        "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 }
                    },
                    "services": {}
                }),
                "happiness services must not be empty",
            ),
            (
                serde_json::json!({
                    "changes": {
                        "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 }
                    },
                    "services": {
                        "Daisys Grooming": [
                            { "rollWeight": 255, "scriptValue": 2, "changeCode": 18 }
                        ]
                    }
                }),
                "happiness service routine must be a nonempty exact pack token",
            ),
            (
                serde_json::json!({
                    "changes": {
                        "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 }
                    },
                    "services": {
                        "DaisysGrooming": []
                    }
                }),
                "happiness service DaisysGrooming must declare outcomes",
            ),
            (
                serde_json::json!({
                    "changes": {
                        "18": { "code": "HAPPINESS_GROOMING", "low": 3, "mid": 3, "high": 1 }
                    },
                    "services": {
                        "DaisysGrooming": [
                            { "rollWeight": 255, "scriptValue": 2, "changeCode": 19 }
                        ]
                    }
                }),
                "references unknown change code 19",
            ),
        ] {
            let error = GameDataSet::default()
                .apply_content_pack_payload(ContentPackCategory::HappinessData, payload)
                .expect_err("invalid happiness data table must be rejected");

            assert!(format!("{error:#}").contains(expected), "{error:#}");
        }
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_flee_mons_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "buckets": {
                "always": ["RAIKOU"],
                "often": ["ENTEI"],
                "sometimes": ["SUICUNE"]
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::FleeMons, payload.clone())
            .expect("initial flee mons table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::FleeMons, payload)
            .expect_err("duplicate flee mons table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate flee mons table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_flee_mon_bucket_ids() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::FleeMons,
                serde_json::json!({
                    "buckets": {
                        "Always": ["RAIKOU"]
                    }
                }),
            )
            .expect_err("malformed flee mon bucket ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("flee mons bucket id"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_empty_flee_mon_buckets() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::FleeMons,
                serde_json::json!({
                    "buckets": {
                        "always": []
                    }
                }),
            )
            .expect_err("empty flee mon buckets must fail during pack load");

        assert!(
            format!("{error:#}").contains("must not be empty"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_flee_mon_species_ids() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::FleeMons,
                serde_json::json!({
                    "buckets": {
                        "always": ["RAI KOU"]
                    }
                }),
            )
            .expect_err("malformed flee mon species ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("flee mons species id"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FleeMons,
                serde_json::json!({
                    "buckets": {
                        "always": ["fallbackRaikou"]
                    }
                }),
            )
            .expect_err("reserved flee mon species ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("flee mons species id must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FleeMons,
                serde_json::json!({
                    "flee_mons": {
                        "buckets": {
                            "always": ["RAIKOU"]
                        }
                    },
                    "fallback_buckets": {}
                }),
            )
            .expect_err("flee mons payload must be the compiler-emitted table");
        assert!(
            format!("{error:#}").contains("unknown field `fallback_buckets`, expected `buckets`"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_move_names_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!(["POUND", "KARATE_CHOP"]);
        data.apply_content_pack_payload(ContentPackCategory::MoveNames, payload.clone())
            .expect("initial move names table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::MoveNames, payload)
            .expect_err("duplicate move names table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate move names table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_move_names_table_at_load_time() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::MoveNames, serde_json::json!([]))
            .expect_err("empty move names payload must not be a silent no-op");

        assert!(
            format!("{error:#}").contains("move names payload must contain at least one entry"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MoveNames,
                serde_json::json!(["POUND", ""]),
            )
            .expect_err("move names must be exact token ids");

        assert!(
            format!("{error:#}").contains("move name '' must be exact, non-empty, and untrimmed"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::MoveNames,
                serde_json::json!({
                    "moves": ["POUND"],
                    "fallback_move": "TACKLE"
                }),
            )
            .expect_err("move names payload must be the compiler-emitted token array");

        assert!(
            format!("{error:#}").contains("parse move names payload"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_battle_animation_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!(["BattleAnim_Pound", "BattleAnim_KarateChop"]);
        data.apply_content_pack_payload(ContentPackCategory::BattleAnimationTable, payload.clone())
            .expect("initial battle animation table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BattleAnimationTable, payload)
            .expect_err("duplicate battle animation table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle animation table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_battle_animation_values_without_trimming() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimationTable,
                serde_json::json!([]),
            )
            .expect_err("empty battle animation table payload must not be a silent no-op");

        assert!(
            format!("{error:#}")
                .contains("battle animation table payload must contain at least one entry"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimations,
                serde_json::json!({
                    "BattleAnim_Pound": [" anim_wait 1"]
                }),
            )
            .expect_err("battle animation commands must not be trimmed");

        assert!(
            format!("{error:#}").contains(
                "battle animation command ' anim_wait 1' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimations,
                serde_json::json!({
                    "BattleAnim Pound": ["anim_wait 1"]
                }),
            )
            .expect_err("battle animation labels must be token ids");

        assert!(
            format!("{error:#}").contains(
                "battle animation 'BattleAnim Pound' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimations,
                serde_json::json!({
                    "animations": {
                        "BattleAnim_Pound": ["anim_wait 1"]
                    },
                    "fallback_animation": "BattleAnim_Default"
                }),
            )
            .expect_err("battle animation payload must be the compiler-emitted command map");

        assert!(
            format!("{error:#}").contains("parse battle animation payload"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimationTable,
                serde_json::json!(["BattleAnim_Pound", "BattleAnim_KarateChop\u{0007}"]),
            )
            .expect_err("battle animation table entries must not contain control characters");

        assert!(
            format!("{error:#}").contains(
                "battle animation table entry 'BattleAnim_KarateChop\u{0007}' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimationTable,
                serde_json::json!(["BattleAnim_Pound", "BattleAnim KarateChop"]),
            )
            .expect_err("battle animation table entries must reject internal spaces");

        assert!(
            format!("{error:#}").contains(
                "battle animation table entry 'BattleAnim KarateChop' must be exact ASCII alphanumeric or underscore"
            ),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimationTable,
                serde_json::json!({
                    "animations": ["BattleAnim_Pound"],
                    "fallback_animation": "BattleAnim_Default"
                }),
            )
            .expect_err("battle animation table payload must be the compiler-emitted token array");

        assert!(
            format!("{error:#}").contains("parse battle animation table payload"),
            "{error:#}"
        );

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimations,
                serde_json::json!({
                    "BattleAnim_Empty": []
                }),
            )
            .expect_err("battle animation command lists must not be empty");

        assert!(
            format!("{error:#}").contains(
                "battle animation 'BattleAnim_Empty' must declare at least one battle animation command"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_animation_bundles() {
        let mut data = GameDataSet::default();
        let battle_payload = complete_battle_anim_bundle_payload();
        data.apply_content_pack_payload(
            ContentPackCategory::BattleAnimBundle,
            battle_payload.clone(),
        )
        .expect("initial battle animation bundle should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BattleAnimBundle, battle_payload)
            .expect_err("duplicate battle animation bundle must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate battle animation bundle"),
            "{error:#}"
        );

        let sprite_payload = complete_sprite_anim_bundle_payload();
        data.apply_content_pack_payload(
            ContentPackCategory::SpriteAnimBundle,
            sprite_payload.clone(),
        )
        .expect("initial sprite animation bundle should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::SpriteAnimBundle, sprite_payload)
            .expect_err("duplicate sprite animation bundle must not overwrite");
        assert!(
            format!("{error:#}").contains("duplicate sprite animation bundle"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_animation_bundles_at_load_time() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::BattleAnimBundle,
                serde_json::json!({ "objects": { "BattleAnim_Pound": {} } }),
            )
            .expect_err("battle animation bundle must include all required sections");
        assert!(format!("{error:#}").contains("MissingSection"), "{error:#}");

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::SpriteAnimBundle,
                serde_json::json!({ "oam_sets": {} }),
            )
            .expect_err("sprite animation bundle sections must be nonempty objects");
        assert!(format!("{error:#}").contains("MissingSection"), "{error:#}");

        let mut battle_payload = complete_battle_anim_bundle_payload();
        battle_payload["fallback_objects"] = serde_json::json!({});
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::BattleAnimBundle, battle_payload)
            .expect_err("battle animation bundles must reject unknown fallback sections");
        assert!(format!("{error:#}").contains("UnknownSection"), "{error:#}");

        let mut sprite_payload = complete_sprite_anim_bundle_payload();
        sprite_payload["fallback_framesets"] = serde_json::json!({});
        let error = GameDataSet::default()
            .apply_content_pack_payload(ContentPackCategory::SpriteAnimBundle, sprite_payload)
            .expect_err("sprite animation bundles must reject unknown fallback sections");
        assert!(format!("{error:#}").contains("UnknownSection"), "{error:#}");
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_tileset_payloads() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "johto": {
                "collision": {
                    "0": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"]
                },
                "palette_map": [0, 1, 2, 3]
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::Tilesets, payload.clone())
            .expect("initial tileset payload should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::Tilesets, payload)
            .expect_err("duplicate tileset payload must not append");

        assert!(
            format!("{error:#}").contains("duplicate tileset id 'johto'"),
            "{error:#}"
        );

        let array_error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Tilesets,
                serde_json::json!([0, 1, 2, 3]),
            )
            .expect_err("tilesets must not accept anonymous palette arrays")
            .to_string();
        assert!(
            array_error.contains("parse object-map payload"),
            "{array_error}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_tileset_collision_without_trimming() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Tilesets,
                serde_json::json!({
                    "johto": {
                        "collision": {
                            " 10": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"]
                        },
                        "palette_map": [0]
                    }
                }),
            )
            .expect_err("tileset metatile ids must not be trimmed");
        assert!(
            format!("{error:#}")
                .contains("tileset metatile id key ' 10' must be exact, non-empty, and untrimmed"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Tilesets,
                serde_json::json!({
                    "johto": {
                        "collision": {
                            "10": ["FLOOR", " FLOOR", "FLOOR", "FLOOR"]
                        },
                        "palette_map": [0]
                    }
                }),
            )
            .expect_err("tileset collision tokens must not be trimmed");
        assert!(
            format!("{error:#}").contains(
                "tileset collision token ' FLOOR' must be exact, non-empty, and untrimmed"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Tilesets,
                serde_json::json!({
                    "johto": {
                        "collision": {
                            "10": ["FLOOR", "UNKNOWN_COLLISION", "FLOOR", "FLOOR"]
                        },
                        "palette_map": [0]
                    }
                }),
            )
            .expect_err("tileset collision tokens must be known");
        assert!(
            format!("{error:#}")
                .contains("resolve collision token 'UNKNOWN_COLLISION' in tileset 'johto:10'"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Tilesets,
                serde_json::json!({
                    "johto": {
                        "collision": {
                            "0": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"],
                            "2": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"]
                        },
                        "palette_map": [0]
                    }
                }),
            )
            .expect_err("tileset collision ids must be dense");
        assert!(
            format!("{error:#}")
                .contains("tileset 'johto' collision map must explicitly declare metatile id 1"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_encounter_slot_tables_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::EncounterSlotTables,
            serde_json::json!({
                "tables": {
                    "grass": [
                        { "threshold": 30, "slot": 0 },
                        { "threshold": 100, "slot": 1 }
                    ],
                    "water": [
                        { "threshold": 100, "slot": 0 }
                    ]
                }
            }),
        )
        .expect("apply encounter slot tables payload");

        assert_eq!(
            data.encounter_slot_tables,
            EncounterSlotTables::for_crystal(
                vec![
                    crystal_core::world::encounters::EncounterSlotChance {
                        threshold: 30,
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
            )
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_encounter_slot_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "tables": {
                "grass": [
                    { "threshold": 100, "slot": 0 }
                ],
                "water": [
                    { "threshold": 100, "slot": 0 }
                ]
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::EncounterSlotTables, payload.clone())
            .expect("initial encounter slot table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::EncounterSlotTables, payload)
            .expect_err("duplicate encounter slot table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate encounter slot table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_encounter_slot_tables() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EncounterSlotTables,
                serde_json::json!({
                    "tables": {
                        "grass": [
                            { "threshold": 50, "slot": 0 },
                            { "threshold": 40, "slot": 1 }
                        ],
                        "water": [
                            { "threshold": 100, "slot": 0 }
                        ]
                    }
                }),
            )
            .expect_err("unordered encounter slot thresholds must fail during pack load");

        assert!(
            format!("{error:#}").contains("UnorderedThreshold"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_missing_required_encounter_slot_tables() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EncounterSlotTables,
                serde_json::json!({
                    "tables": {
                        "grass": [
                            { "threshold": 100, "slot": 0 }
                        ]
                    }
                }),
            )
            .expect_err("missing water encounter slot table must fail during pack load");

        assert!(
            format!("{error:#}").contains("MissingTable { surface: Water }"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_encounter_music_modifiers_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::EncounterMusicModifiers,
            serde_json::json!({
                "modifiers": {
                    "MUSIC_POKEMON_MARCH": { "numerator": 2, "denominator": 1 },
                    "MUSIC_POKEMON_LULLABY": { "numerator": 1, "denominator": 2 }
                }
            }),
        )
        .expect("apply encounter music modifiers payload");

        assert_eq!(
            data.encounter_music_modifiers,
            EncounterMusicModifiers {
                modifiers: BTreeMap::from([
                    (
                        "MUSIC_POKEMON_MARCH".to_string(),
                        EncounterMusicModifier {
                            numerator: 2,
                            denominator: 1,
                        },
                    ),
                    (
                        "MUSIC_POKEMON_LULLABY".to_string(),
                        EncounterMusicModifier {
                            numerator: 1,
                            denominator: 2,
                        },
                    ),
                ]),
            }
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_encounter_music_modifier_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "modifiers": {
                "MUSIC_POKEMON_MARCH": { "numerator": 2, "denominator": 1 }
            }
        });
        data.apply_content_pack_payload(
            ContentPackCategory::EncounterMusicModifiers,
            payload.clone(),
        )
        .expect("initial encounter music modifier table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::EncounterMusicModifiers, payload)
            .expect_err("duplicate encounter music modifier table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate encounter music modifier table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_encounter_music_modifiers() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::EncounterMusicModifiers,
                serde_json::json!({
                    "modifiers": {
                        "MUSIC POKEMON_MARCH": { "numerator": 2, "denominator": 1 }
                    }
                }),
            )
            .expect_err("malformed encounter music ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("encounter token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::EncounterMusicModifiers,
                serde_json::json!({
                    "modifiers": {
                        "legacyMusic": { "numerator": 2, "denominator": 1 }
                    }
                }),
            )
            .expect_err("reserved encounter music ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("encounter token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::EncounterMusicModifiers,
                serde_json::json!({
                    "modifiers": {
                        "MUSIC_POKEMON_MARCH": { "numerator": 2, "denominator": 0 }
                    }
                }),
            )
            .expect_err("zero encounter music modifier denominators must fail during pack load");

        assert!(
            format!("{error:#}").contains("denominator must be nonzero"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_battle_stat_multipliers_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::BattleStatMultipliers,
            serde_json::json!({
                "stat": [
                    { "numerator": 25, "denominator": 100 },
                    { "numerator": 28, "denominator": 100 },
                    { "numerator": 33, "denominator": 100 },
                    { "numerator": 40, "denominator": 100 },
                    { "numerator": 50, "denominator": 100 },
                    { "numerator": 66, "denominator": 100 },
                    { "numerator": 1, "denominator": 1 },
                    { "numerator": 15, "denominator": 10 },
                    { "numerator": 2, "denominator": 1 },
                    { "numerator": 25, "denominator": 10 },
                    { "numerator": 3, "denominator": 1 },
                    { "numerator": 35, "denominator": 10 },
                    { "numerator": 4, "denominator": 1 }
                ],
                "accuracy": [
                    { "numerator": 33, "denominator": 100 },
                    { "numerator": 36, "denominator": 100 },
                    { "numerator": 43, "denominator": 100 },
                    { "numerator": 50, "denominator": 100 },
                    { "numerator": 60, "denominator": 100 },
                    { "numerator": 75, "denominator": 100 },
                    { "numerator": 1, "denominator": 1 },
                    { "numerator": 133, "denominator": 100 },
                    { "numerator": 166, "denominator": 100 },
                    { "numerator": 2, "denominator": 1 },
                    { "numerator": 233, "denominator": 100 },
                    { "numerator": 133, "denominator": 50 },
                    { "numerator": 3, "denominator": 1 }
                ]
            }),
        )
        .expect("apply battle stat multipliers payload");

        assert_eq!(data.battle_stat_multipliers.stat.len(), 13);
        assert_eq!(data.battle_stat_multipliers.accuracy.len(), 13);
        assert_eq!(data.battle_stat_multipliers.stat[0].numerator, 25);
        assert_eq!(data.battle_stat_multipliers.accuracy[8].numerator, 166);
        assert_eq!(data.battle_stat_multipliers.accuracy[11].denominator, 50);
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_battle_stat_multiplier_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "stat": [
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 }
            ],
            "accuracy": [
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 },
                { "numerator": 1, "denominator": 1 }
            ]
        });
        data.apply_content_pack_payload(
            ContentPackCategory::BattleStatMultipliers,
            payload.clone(),
        )
        .expect("initial battle stat multiplier table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BattleStatMultipliers, payload)
            .expect_err("duplicate battle stat multiplier table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle stat multiplier table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_battle_stat_multiplier_tables() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleStatMultipliers,
                serde_json::json!({
                    "stat": [
                        { "numerator": 1, "denominator": 1 }
                    ],
                    "accuracy": [
                        { "numerator": 1, "denominator": 0 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 },
                        { "numerator": 1, "denominator": 1 }
                    ]
                }),
            )
            .expect_err("invalid battle stat multiplier tables must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("InvalidDenominator { table: Stat, stage: 0, denominator: 0 }"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::BattleStatMultipliers,
                serde_json::json!({
                    "stat": [],
                    "accuracy": []
                }),
            )
            .expect_err("empty battle stat multiplier tables must not be accepted as defaults");

        assert!(
            format!("{error:#}").contains("battle stat multiplier tables"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_capture_wobble_probabilities_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::CaptureWobbleProbabilities,
            serde_json::json!([
                { "catch_rate": 1, "chance": 63 },
                { "catch_rate": 255, "chance": 255 }
            ]),
        )
        .expect("apply capture wobble probabilities payload");

        assert_eq!(
            data.capture_wobble_probabilities,
            vec![
                CaptureWobbleProbability {
                    catch_rate: 1,
                    chance: 63,
                },
                CaptureWobbleProbability {
                    catch_rate: 255,
                    chance: 255,
                },
            ]
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_capture_wobble_probability_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!([
            { "catch_rate": 1, "chance": 63 },
            { "catch_rate": 255, "chance": 255 }
        ]);
        data.apply_content_pack_payload(
            ContentPackCategory::CaptureWobbleProbabilities,
            payload.clone(),
        )
        .expect("initial capture wobble probability table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::CaptureWobbleProbabilities, payload)
            .expect_err("duplicate capture wobble probability table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate capture wobble probability table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_capture_wobble_probability_tables() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::CaptureWobbleProbabilities,
                serde_json::json!([
                    { "catch_rate": 0, "chance": 0 },
                    { "catch_rate": 255, "chance": 255 }
                ]),
            )
            .expect_err("zero catch rate capture wobble entries must fail during pack load");
        assert!(
            format!("{error:#}").contains("catch_rate must be positive"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::CaptureWobbleProbabilities,
                serde_json::json!([
                    { "catch_rate": 10, "chance": 20 },
                    { "catch_rate": 9, "chance": 30 },
                    { "catch_rate": 255, "chance": 255 }
                ]),
            )
            .expect_err("unordered capture wobble tables must fail during pack load");
        assert!(
            format!("{error:#}").contains("catch_rate values must be nondecreasing"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::CaptureWobbleProbabilities,
                serde_json::json!([
                    { "catch_rate": 1, "chance": 63 }
                ]),
            )
            .expect_err("incomplete capture wobble tables must fail during pack load");
        assert!(
            format!("{error:#}").contains("must end at catch_rate 255"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_capture_rules_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "fast_ball_species": ["MAGNEMITE"],
            "heavy_ball_modifiers": {},
            "ball_rules": {},
            "guaranteed_capture_balls": [],
            "status_bonus": {}
        });
        data.apply_content_pack_payload(ContentPackCategory::CaptureRules, payload.clone())
            .expect("initial capture rules table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::CaptureRules, payload)
            .expect_err("duplicate capture rules table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate capture rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_require_complete_capture_rules_table() {
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": [],
                    "ball_rules": {},
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("capture rules must not default omitted pack fields");

        assert!(
            format!("{error:#}").contains("missing field `heavy_ball_modifiers`"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_capture_species_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": ["MAGNEMITE "],
                    "heavy_ball_modifiers": {},
                    "ball_rules": {},
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("malformed capture species ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("fast ball species \"MAGNEMITE \" is not exact"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": ["FALLBACK_MAGNEMITE"],
                    "heavy_ball_modifiers": {},
                    "ball_rules": {},
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("reserved capture species ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("fast ball species \"FALLBACK_MAGNEMITE\" is not exact"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_capture_ball_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": [],
                    "heavy_ball_modifiers": {},
                    "ball_rules": {
                        "POKE BALL": {
                            "multiplier_numerator": 1,
                            "multiplier_denominator": 1,
                            "battle_type": "",
                            "skip_hp_calc": false,
                            "use_heavy_ball_weight_modifier": false,
                            "use_level_ball_multiplier": false,
                            "require_same_species": false,
                            "require_same_gender": false,
                            "require_fast_species": false
                        }
                    },
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("malformed capture ball ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("invalid capture ball rule for 'POKE BALL'")
                && format!("{error:#}").contains("ball id must be an exact nonempty id"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": [],
                    "heavy_ball_modifiers": {},
                    "ball_rules": {
                        "LEGACY_BALL": {
                            "multiplier_numerator": 1,
                            "multiplier_denominator": 1,
                            "battle_type": "",
                            "skip_hp_calc": false,
                            "use_heavy_ball_weight_modifier": false,
                            "use_level_ball_multiplier": false,
                            "require_same_species": false,
                            "require_same_gender": false,
                            "require_fast_species": false
                        }
                    },
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("reserved capture ball ids must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "invalid capture ball rule for 'LEGACY_BALL': ball id must be an exact nonempty id"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_capture_battle_type_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": [],
                    "heavy_ball_modifiers": {},
                    "ball_rules": {
                        "POKE_BALL": {
                            "multiplier_numerator": 1,
                            "multiplier_denominator": 1,
                            "battle_type": "wild battle",
                            "skip_hp_calc": false,
                            "use_heavy_ball_weight_modifier": false,
                            "use_level_ball_multiplier": false,
                            "require_same_species": false,
                            "require_same_gender": false,
                            "require_fast_species": false
                        }
                    },
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("malformed capture battle type ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("capture battle type \"wild battle\" is not exact"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_zero_denominator_capture_ball_rules() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::CaptureRules,
                serde_json::json!({
                    "fast_ball_species": [],
                    "heavy_ball_modifiers": {},
                    "ball_rules": {
                        "POKE_BALL": {
                            "multiplier_numerator": 1,
                            "multiplier_denominator": 0,
                            "battle_type": "",
                            "skip_hp_calc": false,
                            "use_heavy_ball_weight_modifier": false,
                            "use_level_ball_multiplier": false,
                            "require_same_species": false,
                            "require_same_gender": false,
                            "require_fast_species": false
                        }
                    },
                    "guaranteed_capture_balls": [],
                    "status_bonus": {}
                }),
            )
            .expect_err("zero denominator capture ball rules must fail during pack load");

        assert!(
            format!("{error:#}").contains("multiplier denominator must be nonzero"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_battle_escape_rules_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "player_speed_multiplier": 32,
            "enemy_speed_divisor": 4,
            "failed_attempt_bonus": 30,
            "rng_roll_values": 256
        });
        data.apply_content_pack_payload(ContentPackCategory::BattleEscapeRules, payload.clone())
            .expect("initial battle escape rules table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BattleEscapeRules, payload)
            .expect_err("duplicate battle escape rules table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle escape rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_battle_escape_rules() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleEscapeRules,
                serde_json::json!({
                    "player_speed_multiplier": 32,
                    "enemy_speed_divisor": 0,
                    "failed_attempt_bonus": 30,
                    "rng_roll_values": 257
                }),
            )
            .expect_err("invalid battle escape rules must fail during pack load");

        assert!(
            format!("{error:#}").contains("MissingEnemySpeedDivisor"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::BattleEscapeRules,
                serde_json::json!({
                    "player_speed_multiplier": 0,
                    "enemy_speed_divisor": 0,
                    "failed_attempt_bonus": 0,
                    "rng_roll_values": 0
                }),
            )
            .expect_err("all-zero battle escape rules must not be accepted as defaults");

        assert!(
            format!("{error:#}").contains("battle escape rules"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_type_effectiveness_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::TypeEffectiveness,
            serde_json::json!({
                "matchups": {
                    "FIRE": {
                        "GRASS": { "numerator": 2, "denominator": 1 }
                    },
                    "ELECTRIC": {
                        "GROUND": { "numerator": 0, "denominator": 1 }
                    }
                },
                "foresight_matchups": {
                    "NORMAL": {
                        "GHOST": { "numerator": 0, "denominator": 1 }
                    }
                }
            }),
        )
        .expect("apply type effectiveness payload");

        assert_eq!(
            data.type_effectiveness,
            serde_json::from_value::<TypeEffectivenessTable>(serde_json::json!({
                "matchups": {
                    "FIRE": {
                        "GRASS": { "numerator": 2, "denominator": 1 }
                    },
                    "ELECTRIC": {
                        "GROUND": { "numerator": 0, "denominator": 1 }
                    }
                },
                "foresight_matchups": {
                    "NORMAL": {
                        "GHOST": { "numerator": 0, "denominator": 1 }
                    }
                }
            }))
            .expect("type effectiveness fixture should parse")
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_type_effectiveness_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "matchups": {
                "FIRE": {
                    "GRASS": { "numerator": 2, "denominator": 1 }
                }
            },
            "foresight_matchups": {
                "NORMAL": {
                    "GHOST": { "numerator": 0, "denominator": 1 }
                }
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::TypeEffectiveness, payload.clone())
            .expect("initial type effectiveness table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::TypeEffectiveness, payload)
            .expect_err("duplicate type effectiveness table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate type effectiveness table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_type_effectiveness_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::TypeEffectiveness,
                serde_json::json!({
                    "matchups": {
                        "FIRE ": {
                            "GRASS": { "numerator": 2, "denominator": 1 }
                        }
                    },
                    "foresight_matchups": {}
                }),
            )
            .expect_err("malformed type effectiveness attacker ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("type effectiveness foresight_matchups must be explicit"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::TypeEffectiveness,
                serde_json::json!({
                    "matchups": {
                        "FIRE": {
                            "GRA SS": { "numerator": 2, "denominator": 1 }
                        }
                    },
                    "foresight_matchups": {}
                }),
            )
            .expect_err("malformed type effectiveness defender ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("type effectiveness foresight_matchups must be explicit"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_zero_denominator_type_effectiveness() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::TypeEffectiveness,
                serde_json::json!({
                    "matchups": {
                        "FIRE": {
                            "GRASS": { "numerator": 2, "denominator": 0 }
                        }
                    },
                    "foresight_matchups": {}
                }),
            )
            .expect_err("zero denominator type effectiveness must fail during pack load");

        assert!(
            format!("{error:#}").contains("denominator must be nonzero"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_type_categories_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::TypeCategories,
            serde_json::json!({
                "physical": ["NORMAL", "FIGHTING", "BIRD"],
                "special": ["FIRE", "WATER", "PSYCHIC_TYPE"]
            }),
        )
        .expect("apply type categories payload");

        assert_eq!(
            data.type_categories,
            TypeCategories {
                physical: vec![
                    "NORMAL".to_string(),
                    "FIGHTING".to_string(),
                    "BIRD".to_string(),
                ],
                special: vec![
                    "FIRE".to_string(),
                    "WATER".to_string(),
                    "PSYCHIC_TYPE".to_string(),
                ],
            }
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_type_category_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "physical": ["NORMAL"],
            "special": ["FIRE"]
        });
        data.apply_content_pack_payload(ContentPackCategory::TypeCategories, payload.clone())
            .expect("initial type category table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::TypeCategories, payload)
            .expect_err("duplicate type category table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate type category table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_type_category_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::TypeCategories,
                serde_json::json!({
                    "physical": ["NORMAL "],
                    "special": ["FIRE"]
                }),
            )
            .expect_err("malformed type category tokens must fail during pack load");

        assert!(
            format!("{error:#}").contains("physical type category \"NORMAL \" is not exact"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_overlapping_type_categories() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::TypeCategories,
                serde_json::json!({
                    "physical": ["FIRE"],
                    "special": ["FIRE"]
                }),
            )
            .expect_err("overlapping type categories must fail during pack load");

        assert!(
            format!("{error:#}").contains("cannot be both physical and special"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_merge_move_priorities_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::MovePriorities,
            serde_json::json!({
                "base_priority": 1,
                "effect_priorities": { "PROTECT": 3, "PRIORITY_HIT": 2 },
                "move_priorities": [
                    { "move": "VITAL_THROW", "priority": 0 }
                ]
            }),
        )
        .expect("apply move priorities payload");

        assert_eq!(
            data.move_priorities,
            MovePriorityTable {
                base_priority: 1,
                effect_priorities: [("PROTECT".to_string(), 3), ("PRIORITY_HIT".to_string(), 2),]
                    .into_iter()
                    .collect(),
                move_priorities: vec![crystal_core::battle::turn::MovePriorityOverride {
                    r#move: "VITAL_THROW".to_string(),
                    priority: 0,
                }],
            }
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_move_priority_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "base_priority": 1,
            "effect_priorities": {
                "PROTECT": 3
            },
            "move_priorities": [
                { "move": "VITAL_THROW", "priority": 0 }
            ]
        });
        data.apply_content_pack_payload(ContentPackCategory::MovePriorities, payload.clone())
            .expect("initial move priority table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::MovePriorities, payload)
            .expect_err("duplicate move priority table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate move priority table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_move_priority_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MovePriorities,
                serde_json::json!({
                    "base_priority": 1,
                    "effect_priorities": { "PRIORITY-HIT": 2 },
                    "move_priorities": []
                }),
            )
            .expect_err("malformed move priority tokens must fail during pack load");

        assert!(
            format!("{error:#}").contains("move priority effect id"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_negative_move_priority_values() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::MovePriorities,
                serde_json::json!({
                    "base_priority": 1,
                    "effect_priorities": { "PROTECT": -1 },
                    "move_priorities": []
                }),
            )
            .expect_err("negative move priority values must fail during pack load");

        assert!(
            format!("{error:#}").contains("has negative priority"),
            "{error:#}"
        );
    }

    #[test]
    fn verifier_rejects_move_priority_overrides_for_missing_moves() {
        let mut data = GameDataSet {
            moves: [("TACKLE".to_string(), test_move("TACKLE"))]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        data.move_priorities = MovePriorityTable {
            base_priority: 1,
            effect_priorities: [("NORMAL_HIT".to_string(), 1)].into_iter().collect(),
            move_priorities: vec![crystal_core::battle::turn::MovePriorityOverride {
                r#move: "EXTREME_SPEED".to_string(),
                priority: 2,
            }],
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_move_priority"
                && diagnostic.subject == "move_priorities:move_priorities"
        }));
    }

    #[test]
    fn content_pack_payloads_merge_weather_modifiers_as_exact_pack_data() {
        let mut data = GameDataSet::default();

        data.apply_content_pack_payload(
            ContentPackCategory::WeatherModifiers,
            serde_json::json!({
                "type_modifiers": {
                    "WEATHER_RAIN": {
                        "WATER": { "numerator": 3, "denominator": 2 }
                    },
                    "WEATHER_SUN": {
                        "FIRE": { "numerator": 3, "denominator": 2 }
                    }
                },
                "move_effect_modifiers": {
                    "WEATHER_RAIN": {
                        "SOLARBEAM": { "numerator": 1, "denominator": 2 }
                    }
                }
            }),
        )
        .expect("apply weather modifiers payload");

        assert_eq!(
            data.weather_modifiers,
            serde_json::from_value::<WeatherModifiers>(serde_json::json!({
                "type_modifiers": {
                    "WEATHER_RAIN": {
                        "WATER": { "numerator": 3, "denominator": 2 }
                    },
                    "WEATHER_SUN": {
                        "FIRE": { "numerator": 3, "denominator": 2 }
                    }
                },
                "move_effect_modifiers": {
                    "WEATHER_RAIN": {
                        "SOLARBEAM": { "numerator": 1, "denominator": 2 }
                    }
                }
            }))
            .expect("weather modifier fixture should parse")
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_weather_modifier_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
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
        });
        data.apply_content_pack_payload(ContentPackCategory::WeatherModifiers, payload.clone())
            .expect("initial weather modifier table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::WeatherModifiers, payload)
            .expect_err("duplicate weather modifier table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate weather modifier table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_weather_modifier_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::WeatherModifiers,
                serde_json::json!({
                    "type_modifiers": {
                        "WEATHER_RAIN": {
                            "WATER ": { "numerator": 3, "denominator": 2 }
                        }
                    },
                    "move_effect_modifiers": {}
                }),
            )
            .expect_err("malformed weather modifier tokens must fail during pack load");

        assert!(
            format!("{error:#}").contains("weather move_effect_modifiers must be explicit"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_zero_denominator_weather_modifiers() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::WeatherModifiers,
                serde_json::json!({
                    "type_modifiers": {
                        "WEATHER_RAIN": {
                            "WATER": { "numerator": 3, "denominator": 0 }
                        }
                    },
                    "move_effect_modifiers": {}
                }),
            )
            .expect_err("zero denominator weather modifiers must fail during pack load");

        assert!(
            format!("{error:#}").contains("denominator must be nonzero"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_battle_reward_rules_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "max_level": 100,
            "wild_exp_divisor": 7,
            "trainer_exp_numerator": 3,
            "trainer_exp_denominator": 2,
            "mom_money_increment": 2300,
            "mom_random_items": [{
                "trigger": 0, "cost": 600, "kind": "item",
                "target": "SUPER_POTION", "decoration_flag": null
            }],
            "mom_progression_items": [{
                "trigger": 900, "cost": 600, "kind": "item",
                "target": "SUPER_POTION", "decoration_flag": null
            }]
        });
        data.apply_content_pack_payload(ContentPackCategory::BattleRewardRules, payload.clone())
            .expect("initial battle reward rules table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::BattleRewardRules, payload)
            .expect_err("duplicate battle reward rules table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate battle reward rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_battle_reward_rules() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::BattleRewardRules,
                serde_json::json!({
                    "max_level": 100,
                    "wild_exp_divisor": 0,
                    "trainer_exp_numerator": 3,
                    "trainer_exp_denominator": 2,
                    "mom_money_increment": 2300,
                    "mom_random_items": [{
                        "trigger": 0, "cost": 600, "kind": "item",
                        "target": "SUPER_POTION", "decoration_flag": null
                    }],
                    "mom_progression_items": [{
                        "trigger": 900, "cost": 600, "kind": "item",
                        "target": "SUPER_POTION", "decoration_flag": null
                    }]
                }),
            )
            .expect_err("invalid battle reward rules must fail during pack load");

        assert!(
            format!("{error:#}").contains("InvalidWildExpDivisor"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::BattleRewardRules,
                serde_json::json!({
                    "max_level": 0,
                    "wild_exp_divisor": 0,
                    "trainer_exp_numerator": 0,
                    "trainer_exp_denominator": 0,
                    "mom_money_increment": 0,
                    "mom_random_items": [],
                    "mom_progression_items": []
                }),
            )
            .expect_err("all-zero battle reward rules must not be accepted as defaults");

        assert!(
            format!("{error:#}").contains("battle reward rules"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_step_event_rules_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "poison_step_interval": 4,
            "egg_step_trigger": 1,
            "hatched_egg_happiness": 120,
            "poison_status": "PSN",
            "egg_nickname": "EGG",
            "happiness_step_counter_mask": 255,
            "happiness_step_counter_target": 0
        });
        data.apply_content_pack_payload(ContentPackCategory::StepEventRules, payload.clone())
            .expect("initial step event rules table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::StepEventRules, payload)
            .expect_err("duplicate step event rules table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate step event rules table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_step_event_tokens() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::StepEventRules,
                serde_json::json!({
                    "poison_step_interval": 4,
                    "egg_step_trigger": 1,
                    "hatched_egg_happiness": 120,
                    "poison_status": "PSN ",
                    "egg_nickname": "EGG",
                    "happiness_step_counter_mask": 255,
                    "happiness_step_counter_target": 0
                }),
            )
            .expect_err("malformed step event tokens must fail during pack load");

        assert!(
            format!("{error:#}").contains("invalid step event rules: InvalidPoisonStatus"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StepEventRules,
                serde_json::json!({
                    "poison_step_interval": 4,
                    "egg_step_trigger": 1,
                    "hatched_egg_happiness": 120,
                    "poison_status": "PSN",
                    "egg_nickname": "legacyEgg",
                    "happiness_step_counter_mask": 255,
                    "happiness_step_counter_target": 0
                }),
            )
            .expect_err("reserved step event egg nickname ids must fail during pack load");

        assert!(
            format!("{error:#}").contains("invalid step event rules: InvalidEggNickname"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_step_event_counters() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::StepEventRules,
                serde_json::json!({
                    "poison_step_interval": 4,
                    "egg_step_trigger": 1,
                    "hatched_egg_happiness": 120,
                    "poison_status": "PSN",
                    "egg_nickname": "EGG",
                    "happiness_step_counter_mask": 7,
                    "happiness_step_counter_target": 8
                }),
            )
            .expect_err("invalid step event counters must fail during pack load");

        assert!(
            format!("{error:#}").contains("HappinessTargetOutsideMask"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::StepEventRules,
                serde_json::json!({
                    "poison_step_interval": 0,
                    "egg_step_trigger": 0,
                    "hatched_egg_happiness": 0,
                    "poison_status": "",
                    "egg_nickname": "",
                    "happiness_step_counter_mask": 0,
                    "happiness_step_counter_target": 0
                }),
            )
            .expect_err("default step event rules must not be accepted as pack data");

        assert!(
            format!("{error:#}").contains("invalid step event rules"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_fishing_table() {
        let mut data = GameDataSet::default();
        let payload = serde_json::json!({
            "groups": {},
            "time_groups": {},
            "swarm_rules": {},
            "rod_items": {
                "OLD_ROD": "OLD_ROD"
            }
        });
        data.apply_content_pack_payload(ContentPackCategory::Fishing, payload.clone())
            .expect("initial fishing table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::Fishing, payload)
            .expect_err("duplicate fishing table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate fishing table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_fishing_rod_items() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {
                        "OLD ROD": "OLD_ROD"
                    }
                }),
            )
            .expect_err("malformed fishing rod item ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("fishing rod item token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {
                        "legacyRod": "OLD_ROD"
                    }
                }),
            )
            .expect_err("reserved fishing rod item ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("fishing rod item token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {},
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {
                        "OLD_ROD": "UNKNOWN_ROD"
                    }
                }),
            )
            .expect_err("unknown fishing rods must fail during pack load");

        assert!(
            format!("{error:#}").contains("must be a known fishing rod"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_fishing_slots() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_SHORE": {
                            "source_index": 1,
                            "bite_threshold": 255,
                            "rod_tables": {
                                "OLD_ROD": {
                                    "slots": [
                                        { "threshold": 100, "species": "MAGIKARP", "level": 10, "time_group": null },
                                        { "threshold": 90, "species": "TENTACOOL", "level": 10, "time_group": null }
                                    ]
                                }
                            }
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            )
            .expect_err("unordered fishing slots must fail during pack load");

        assert!(
            format!("{error:#}").contains("thresholds must be nondecreasing"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_SHORE": {
                            "source_index": 1,
                            "bite_threshold": 255,
                            "rod_tables": {
                                "OLD_ROD": {
                                    "slots": [
                                        { "threshold": 255, "species": null, "level": 0, "time_group": null }
                                    ]
                                }
                            }
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            )
            .expect_err("fishing slots without species or time group must fail during pack load");

        assert!(
            format!("{error:#}").contains("must define species or time group"),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_SHORE": {
                            "source_index": 1,
                            "bite_threshold": 255,
                            "rod_tables": {
                                "OLD_ROD": {
                                    "slots": [
                                        { "threshold": 255, "species": "fallbackFish", "level": 10, "time_group": null }
                                    ]
                                }
                            }
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {},
                    "rod_items": {}
                }),
            )
            .expect_err("reserved fishing slot species ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("fishing token must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_fishing_swarm_rules() {
        let mut data = GameDataSet::default();

        let error = data
            .apply_content_pack_payload(
                ContentPackCategory::Fishing,
                serde_json::json!({
                    "groups": {
                        "FISHGROUP_BASE": {
                            "source_index": 1,
                            "bite_threshold": 255,
                            "rod_tables": {}
                        }
                    },
                    "time_groups": {},
                    "swarm_rules": {
                        "QWILFISH_SWARM": {
                            "daily_flag_bit": 8,
                            "swarm": 1,
                            "base_group": "FISHGROUP_BASE",
                            "swarm_group": "FISHGROUP_SWARM"
                        }
                    },
                    "rod_items": {}
                }),
            )
            .expect_err("invalid fishing swarm rules must fail during pack load");

        assert!(
            format!("{error:#}").contains("daily flag bit must be below 8"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_duplicate_field_moves_table() {
        let mut data = GameDataSet::default();
        let catalog = test_field_move_catalog();
        let payload = serde_json::to_value(catalog).expect("field move fixture should serialize");
        data.apply_content_pack_payload(ContentPackCategory::FieldMoves, payload.clone())
            .expect("initial field moves table should load");

        let error = data
            .apply_content_pack_payload(ContentPackCategory::FieldMoves, payload)
            .expect_err("duplicate field moves table must not overwrite");

        assert!(
            format!("{error:#}").contains("duplicate field moves table"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_malformed_field_move_tokens() {
        let mut catalog = test_field_move_catalog();
        catalog.fly.move_id = "FL Y".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldMoves,
                serde_json::to_value(catalog).expect("field move fixture should serialize"),
            )
            .expect_err("malformed field move ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("field_moves.fly.move_id must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut catalog = test_field_move_catalog();
        catalog.fly.move_id = "legacyFly".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldMoves,
                serde_json::to_value(catalog).expect("field move fixture should serialize"),
            )
            .expect_err("reserved field move ids must fail during pack load");

        assert!(
            format!("{error:#}")
                .contains("field_moves.fly.move_id must be exact ASCII alphanumeric/underscore"),
            "{error:#}"
        );

        let mut catalog = test_field_move_catalog();
        catalog.itemfinder.item_id = "fallbackItemfinder".to_string();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldMoves,
                serde_json::to_value(catalog).expect("field move fixture should serialize"),
            )
            .expect_err("reserved field item ids must fail during pack load");

        assert!(
            format!("{error:#}").contains(
                "field_moves.itemfinder.item_id must be exact ASCII alphanumeric/underscore"
            ),
            "{error:#}"
        );

        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldMoves,
                serde_json::to_value(FieldMoveCatalog::default())
                    .expect("default field move fixture should serialize"),
            )
            .expect_err("default field move catalog must not be accepted as pack data");

        assert!(
            format!("{error:#}").contains("field_moves.cut.move_id"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_field_move_badges() {
        let mut catalog = test_field_move_catalog();
        catalog.fly.badge.index = 8;
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldMoves,
                serde_json::to_value(catalog).expect("field move fixture should serialize"),
            )
            .expect_err("invalid field move badges must fail during pack load");

        assert!(
            format!("{error:#}").contains("badge.index must be 0..7"),
            "{error:#}"
        );
    }

    #[test]
    fn content_pack_payloads_reject_invalid_field_move_replacements() {
        let mut catalog = test_field_move_catalog();
        catalog.cut.replacements.clear();
        let error = GameDataSet::default()
            .apply_content_pack_payload(
                ContentPackCategory::FieldMoves,
                serde_json::to_value(catalog).expect("field move fixture should serialize"),
            )
            .expect_err("missing field move replacements must fail during pack load");

        assert!(
            format!("{error:#}").contains("replacements must not be empty"),
            "{error:#}"
        );
    }

    fn test_field_move_catalog() -> FieldMoveCatalog {
        let badge = crystal_core::systems::field_moves::FieldMoveBadgeRequirement {
            region: "johto".to_string(),
            index: 0,
        };
        let replacement = crystal_core::systems::field_moves::FieldMoveReplacement {
            replacement_block_id: 2,
            variant: "CUT_TREE".to_string(),
        };
        let replacements =
            BTreeMap::from([("johto".to_string(), BTreeMap::from([(1, replacement)]))]);
        FieldMoveCatalog {
            cut: FieldMoveBlockRule {
                move_id: "CUT".to_string(),
                badge: badge.clone(),
                target_collisions: vec![1],
                replacements: replacements.clone(),
            },
            whirlpool: FieldMoveBlockRule {
                move_id: "WHIRLPOOL".to_string(),
                badge: badge.clone(),
                target_collisions: vec![2],
                replacements,
            },
            strength: FieldMoveFlagRule {
                move_id: "STRENGTH".to_string(),
                badge: badge.clone(),
                engine_flag: "ENGINE_STRENGTH_ACTIVE".to_string(),
            },
            flash: FieldMoveFlagRule {
                move_id: "FLASH".to_string(),
                badge: badge.clone(),
                engine_flag: "ENGINE_FLASH_ACTIVE".to_string(),
            },
            surf: FieldMoveTravelRule {
                move_id: "SURF".to_string(),
                badge: badge.clone(),
                blocked_collisions: vec![],
                target_collisions: vec![],
            },
            waterfall: FieldMoveTravelRule {
                move_id: "WATERFALL".to_string(),
                badge: badge.clone(),
                blocked_collisions: vec![],
                target_collisions: vec![3],
            },
            fly: FieldMoveRule {
                move_id: "FLY".to_string(),
                badge,
            },
            dig: FieldMoveMoveRule {
                move_id: "DIG".to_string(),
                target_collisions: vec![],
            },
            teleport: FieldMoveMoveRule {
                move_id: "TELEPORT".to_string(),
                target_collisions: vec![],
            },
            headbutt: FieldMoveMoveRule {
                move_id: "HEADBUTT".to_string(),
                target_collisions: vec![0x15, 0x1d],
            },
            rock_smash: FieldMoveMoveRule {
                move_id: "ROCK_SMASH".to_string(),
                target_collisions: vec![],
            },
            sweet_scent: FieldMoveMoveRule {
                move_id: "SWEET_SCENT".to_string(),
                target_collisions: vec![],
            },
            escape_rope: FieldEscapeItemRule {
                item_id: "ESCAPE_ROPE".to_string(),
                escape_rope_mode: "ESCAPE_ROPE".to_string(),
            },
            repel: crystal_core::systems::field_moves::FieldRepelItemRule {},
            bicycle: FieldItemRule {
                item_id: "BICYCLE".to_string(),
            },
            itemfinder: FieldItemRule {
                item_id: "ITEMFINDER".to_string(),
            },
            squirtbottle: FieldItemRule {
                item_id: "SQUIRTBOTTLE".to_string(),
            },
            card_key: crystal_core::systems::field_moves::FieldStoryKeyRule {
                item_id: "CARD_KEY".to_string(),
                map_name: "RadioTower3F".to_string(),
                required_facing: Some(Direction::Up),
                target_tile: TilePosition::new(14, 2),
                target_script: "CardKeySlotScript".to_string(),
            },
            basement_key: crystal_core::systems::field_moves::FieldStoryKeyRule {
                item_id: "BASEMENT_KEY".to_string(),
                map_name: "GoldenrodUnderground".to_string(),
                required_facing: None,
                target_tile: TilePosition::new(18, 6),
                target_script: "BasementDoorScript".to_string(),
            },
            coin_case: FieldItemRule {
                item_id: "COIN_CASE".to_string(),
            },
            blue_card: FieldItemRule {
                item_id: "BLUE_CARD".to_string(),
            },
            town_map: FieldItemRule {
                item_id: "TOWN_MAP".to_string(),
            },
            pokegear: FieldItemRule {
                item_id: "POKEGEAR".to_string(),
            },
        }
    }
