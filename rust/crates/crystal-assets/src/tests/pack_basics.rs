    use super::*;
    use crystal_core::map::MapConnection;
    use crystal_core::models::{
        BaseStats, Item, MAX_BOX_MONS, MAX_PC_BOXES, PcBox, ability, egg_group, growth_rate,
        item_pocket, pokemon_type,
    };
    use crystal_core::random::Random;
    use crystal_core::state::{GameState, ScriptRuntimeMemory};
    use crystal_core::systems::economy::{
        AmountComparison, MoneyAccount, check_coins, check_money, take_money,
    };
    use crystal_core::systems::field_items::{
        FieldItemPickup, FieldItemPickupOutcome, FieldItemSource, pickup_field_item,
        pickup_script_field_item,
    };
    use crystal_core::systems::gift_pokemon::{GiftPokemonRequest, give_gift_pokemon};
    use crystal_core::systems::phone::PhoneContactRecord;
    use crystal_core::systems::script_blocks::apply_script_block_change;
    use crystal_core::systems::script_flags::{apply_script_flag_mutation, check_script_flag};
    use crystal_core::systems::script_items::{
        ScriptItemGrantOutcome, check_script_item, grant_script_item, take_script_item,
    };

    fn test_battle_tower_trainers() -> Vec<BattleTowerTrainerDefinition> {
        vec![BattleTowerTrainerDefinition {
            index: 0,
            trainer_class: "YOUNGSTER".to_string(),
            name: "TEST@".to_string(),
            sprite_constant: "SPRITE_YOUNGSTER".to_string(),
        }]
    }

    fn test_battle_tower_mon_groups() -> Vec<Vec<BattleTowerMonDefinition>> {
        vec![vec![BattleTowerMonDefinition {
            species: "RATTATA".to_string(),
            moves: vec!["TACKLE".to_string()],
            level: 10,
            nickname: "RATTATA".to_string(),
            ..BattleTowerMonDefinition::default()
        }]]
    }

    fn test_battle_tower_rules() -> BattleTowerRules {
        BattleTowerRules {
            banned_species: ["MEWTWO", "MEW", "LUGIA", "HO_OH", "CELEBI"]
                .into_iter()
                .map(|species| (species.to_string(), BattleTowerBannedSpeciesRule::default()))
                .collect(),
            required_party_count: 3,
            challenge_streak_length: 7,
            minimum_level_group: 1,
            maximum_level_group: 10,
            level_group_size: 10,
            party_count_failure_text: "OnlyThreeMonMayBeEnteredText".to_string(),
            duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
            duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
            egg_failure_text: "YouCantTakeAnEggText".to_string(),
            trainers: test_battle_tower_trainers(),
            mon_groups: test_battle_tower_mon_groups(),
        }
    }

    #[test]
    fn standard_text_scripts_open_the_runtime_dialogue_surface() {
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load exported standard scripts");
        for (script, expected_text) in [
            ("DifficultBookshelfScript", "DifficultBookshelfText"),
            ("PictureBookshelfScript", "PictureBookshelfText"),
            ("MagazineBookshelfScript", "MagazineBookshelfText"),
            ("TeamRocketOathScript", "TeamRocketOathText"),
            ("IncenseBurnerScript", "IncenseBurnerText"),
            ("MerchandiseShelfScript", "MerchandiseShelfText"),
            ("WindowScript", "WindowText"),
            ("HomepageScript", "HomepageText"),
            ("TrashCanScript", "TrashCanText"),
            ("PokecenterSignScript", "PokecenterSignText"),
            ("MartSignScript", "MartSignText"),
        ] {
            let body = data
                .compiled_standard_script_body(script)
                .expect("compiled terminal standard-script body");
            let mut state = GameState::default();
            let result = apply_standard_script(&mut state, &BTreeMap::new(), script, body)
                .expect("standard text script");

            assert_eq!(result, expected_text);
            assert!(state.script_runtime.text_window_open);
            assert_eq!(
                state
                    .script_runtime
                    .text_events
                    .last()
                    .and_then(|event| event.text_label.as_deref()),
                Some(expected_text)
            );
            assert_eq!(
                state
                    .script_runtime
                    .pending_text_wait
                    .as_ref()
                    .map(|wait| wait.command.as_str()),
                Some("farjumptext")
            );
        }

        let production_source = concat!(
            include_str!("../lib.rs"),
            include_str!("../content_pack.rs"),
            include_str!("../map_modules.rs"),
            include_str!("../runtime_pack.rs"),
            include_str!("../verification.rs"),
            include_str!("../runtime_commands.rs"),
            include_str!("../game_data.rs"),
            include_str!("../mutation_protocol.rs"),
            include_str!("../merge.rs"),
            include_str!("../script_parsing.rs"),
        )
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("production source");
        for removed_dispatch_label in [
            "DifficultBookshelfScript\" =>",
            "PictureBookshelfScript\" =>",
            "MagazineBookshelfScript\" =>",
            "TeamRocketOathScript\" =>",
            "IncenseBurnerScript\" =>",
            "MerchandiseShelfScript\" =>",
            "WindowScript\" =>",
            "HomepageScript\" =>",
            "TrashCanScript\" =>",
            "PokecenterSignScript\" =>",
            "MartSignScript\" =>",
        ] {
            assert!(!production_source.contains(removed_dispatch_label));
        }
    }

    #[test]
    fn compiled_standard_script_catalog_requires_exact_pointer_bodies() {
        let data = AssetRoot::new(repository_root_for_tests())
            .load_base_game_data()
            .expect("load exported standard scripts");
        validate_compiled_standard_script_catalog(&data).expect("valid standard-script catalog");
        let catalog = compiled_standard_script_catalog(&data).expect("standard-script catalog");
        assert_eq!(
            catalog
                .get("StdScripts")
                .and_then(Value::as_array)
                .expect("pointer table")
                .len(),
            52
        );

        let mut missing = data.clone();
        missing.story_events.retain(|payload| {
            !payload
                .as_object()
                .is_some_and(|payload| payload.contains_key("StandardScripts"))
        });
        let error = validate_compiled_standard_script_catalog(&missing)
            .expect_err("missing catalog must fail")
            .to_string();
        assert!(error.contains("missing the StandardScripts"), "{error}");

        let mut stale = data;
        let catalog = stale
            .story_events
            .iter_mut()
            .find_map(|payload| payload.get_mut("StandardScripts"))
            .and_then(Value::as_object_mut)
            .expect("mutable standard-script catalog");
        catalog.remove("PokecenterSignScript");
        let error = validate_compiled_standard_script_catalog(&stale)
            .expect_err("pointer without a body must fail")
            .to_string();
        assert!(
            error.contains("PokecenterSignScript has no command body"),
            "{error}"
        );
    }

    #[test]
    fn progression_standard_scripts_apply_rocket_and_egg_state() {
        let mut state = GameState::default();
        apply_standard_script(&mut state, &BTreeMap::new(), "ReceiveTogepiEggScript", &[])
            .expect("Togepi egg receive script");
        assert!(
            state
                .script_runtime
                .audio_events
                .iter()
                .any(|event| { event.audio_id.as_deref() == Some("SFX_GET_EGG") })
        );

        apply_standard_script(&mut state, &BTreeMap::new(), "RadioTowerRocketsScript", &[])
            .expect("Radio Tower Rocket script");
        assert_eq!(
            state
                .flags
                .engine_flags
                .get("ENGINE_ROCKETS_IN_RADIO_TOWER"),
            Some(&true)
        );
        assert_eq!(
            state
                .flags
                .event_flags
                .get("EVENT_RADIO_TOWER_ROCKET_TAKEOVER"),
            Some(&false)
        );
        assert!(
            state
                .script_runtime
                .special_phone_calls
                .iter()
                .any(|call| call == "SPECIALCALL_WEIRDBROADCAST")
        );
    }

    #[test]
    fn phone_standard_scripts_resolve_caller_specific_text_label() {
        let mut state = GameState::default();
        state.script_runtime.variables.insert(
            "VAR_CALLERID".to_string(),
            "PHONE_SCHOOLBOY_JACK".to_string(),
        );
        apply_standard_script(&mut state, &BTreeMap::new(), "AskNumber1MScript", &[])
            .expect("phone ask-number script");
        assert_eq!(
            state
                .script_runtime
                .text_events
                .last()
                .and_then(|event| event.text_label.as_deref()),
            Some("JackAskNumber1Text")
        );
    }

    #[test]
    fn happiness_standard_script_uses_asm_thresholds() {
        for (happiness, expected) in [
            (0, "HappinessText1"),
            (49, "HappinessText1"),
            (50, "HappinessText2"),
            (149, "HappinessText2"),
            (150, "HappinessText3"),
        ] {
            let mut state = GameState::default();
            let mut pokemon = Pokemon::new_for_tests(species(), 5, Dv::default());
            pokemon.happiness = happiness;
            state.storage.party.pokemon[0] = Some(pokemon);
            apply_standard_script(&mut state, &BTreeMap::new(), "HappinessCheckScript", &[])
                .expect("happiness standard script");
            assert_eq!(
                state
                    .script_runtime
                    .text_events
                    .last()
                    .and_then(|event| event.text_label.as_deref()),
                Some(expected)
            );
        }
    }

    #[test]
    fn happiness_standard_script_skips_leading_eggs_like_asm() {
        let mut state = GameState::default();
        let mut egg = Pokemon::new_for_tests(species(), 5, Dv::default());
        egg.is_egg = true;
        egg.happiness = 0;
        let mut pokemon = Pokemon::new_for_tests(species(), 5, Dv::default());
        pokemon.nickname = "BUDDY".to_string();
        pokemon.happiness = 150;
        let expected_species = pokemon.species.id.clone();
        state.storage.party.pokemon[0] = Some(egg);
        state.storage.party.pokemon[1] = Some(pokemon);

        apply_standard_script(&mut state, &BTreeMap::new(), "HappinessCheckScript", &[])
            .expect("happiness standard script");

        assert_eq!(state.script_runtime.script_value.as_deref(), Some("150"));
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("BUDDY")
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("wCurPartySpecies"),
            Some(&expected_species)
        );
        assert_eq!(
            state
                .script_runtime
                .text_events
                .last()
                .and_then(|event| event.text_label.as_deref()),
            Some("HappinessText3")
        );
    }

    #[test]
    fn happiness_standard_script_does_not_treat_egg_nickname_as_species() {
        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(species(), 5, Dv::default());
        pokemon.nickname = "EGG".to_string();
        pokemon.happiness = 150;
        state.storage.party.pokemon[0] = Some(pokemon);

        apply_standard_script(&mut state, &BTreeMap::new(), "HappinessCheckScript", &[])
            .expect("happiness standard script");

        assert_eq!(state.script_runtime.script_value.as_deref(), Some("150"));
        assert_eq!(
            state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("EGG")
        );
    }

    #[test]
    fn game_corner_vendor_reaches_coin_case_branch_and_prompt() {
        let mut no_case = GameState::default();
        apply_standard_script(
            &mut no_case,
            &BTreeMap::new(),
            "GameCornerCoinVendorScript",
            &[],
        )
        .expect("coin vendor without case");
        assert_eq!(
            no_case
                .script_runtime
                .text_events
                .last()
                .and_then(|event| event.text_label.as_deref()),
            Some("CoinVendor_NoCoinCaseText")
        );
        assert!(no_case.script_runtime.pending_yes_no.is_none());

        let mut with_case = GameState::default();
        with_case.bag.key_items.insert("COIN_CASE".to_string(), 1);
        apply_standard_script(
            &mut with_case,
            &BTreeMap::new(),
            "GameCornerCoinVendorScript",
            &[],
        )
        .expect("coin vendor with case");
        assert_eq!(
            with_case
                .script_runtime
                .pending_yes_no
                .as_ref()
                .map(|prompt| prompt.command_index),
            Some(2)
        );
    }

    #[test]
    fn game_corner_purchase_matches_money_and_coin_caps() {
        let mut purchase = GameState::default();
        purchase.money = 1_000;
        assert_eq!(
            resolve_coin_vendor_purchase(&mut purchase).expect("purchase"),
            "CoinVendor_Buy50CoinsText"
        );
        assert_eq!(purchase.money, 0);
        assert_eq!(purchase.coins, 50);

        let mut poor = GameState::default();
        poor.money = 999;
        assert_eq!(
            resolve_coin_vendor_purchase(&mut poor).expect("poor purchase"),
            "CoinVendor_NotEnoughMoneyText"
        );
        assert_eq!(poor.money, 999);

        let mut full = GameState::default();
        full.money = 10_000;
        full.coins = 9_950;
        assert_eq!(
            resolve_coin_vendor_purchase(&mut full).expect("full coin case"),
            "CoinVendor_CoinCaseFullText"
        );
        assert_eq!(full.money, 10_000);
    }

    #[test]
    fn regenerated_core_pack_embeds_native_presentation_files() {
        let root = AssetRoot::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../..")
                .canonicalize()
                .expect("repository root"),
        );
        let pack = root
            .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
            .expect("load regenerated core pack");
        assert!(!pack.runtime_files().is_empty());
        assert!(
            pack.runtime_files()
                .keys()
                .any(|path| path.ends_with(".png"))
        );
        assert!(
            pack.runtime_files()
                .keys()
                .any(|path| path.ends_with(".2bpp"))
        );
        assert!(
            pack.runtime_files()
                .keys()
                .any(|path| path.ends_with(".gbcpal"))
        );
    }

    #[test]
    fn regenerated_core_pack_uses_playable_downmixed_pcm_audio() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let pack =
            read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
                .expect("load regenerated repository core pack");
        let title = pack
            .data()
            .audio
            .iter()
            .find(|audio| audio.id == "MUSIC_TITLE")
            .expect("compiled title music metadata");
        let format = title.pcm_format.as_ref().expect("title PCM format");
        assert_eq!(format.sample_rate_hz, 22_050);
        assert_eq!(format.channels, 1);
        assert_eq!(format.bits_per_sample, 16);
        assert!(pack.compiled_audio().contains_key("MUSIC_TITLE"));
    }
    use crystal_core::systems::script_objects::{
        apply_script_movement, apply_script_object_mutation,
    };

    #[test]
    fn compiled_mail_text_normalizes_asm_terminators() {
        assert_eq!(
            strip_compiled_mail_text("\"DARK CAVE leads\""),
            "DARK CAVE leads"
        );
        assert_eq!(
            strip_compiled_mail_text("\"to another road@\""),
            "to another road"
        );
    }
    use crystal_core::systems::script_scenes::apply_script_scene_command;
    use crystal_core::systems::scripted_battles::{
        ScriptedBattleEffects, apply_scripted_battle_effects_to_session,
    };
    use crystal_core::systems::special_routines::{
        BUENA_PASSWORD_CATEGORY_ITEM, BUENA_PASSWORD_CATEGORY_MON, BUENA_PASSWORD_CATEGORY_MOVE,
    };
    use crystal_core::world::collision::{
        MetatileCollision, PlayerTraversalState, TilesetCollision, can_enter_tile, permissions,
        sample_collision,
    };
    use crystal_core::world::encounters::EncounterMusicModifier;
    use crystal_core::world::encounters::{
        EncounterSurface, FieldEncounterData, FieldEncounterEntry, FieldEncounterTable, TimeOfDay,
        WildEncounter, WildEncounterTable, table_for_surface,
    };
    use crystal_core::world::map::{Direction, OverworldMapData, TilePosition};
    use crystal_core::world::movement::{StepOptions, StepOutcome};
    use crystal_core::world::session::{EncounterCheckOptions, OverworldSession};

    fn npc_trade_rules<const N: usize>(ids: [&str; N]) -> BTreeMap<String, NpcTradeRule> {
        ids.into_iter()
            .map(|id| (id.to_string(), NpcTradeRule::default()))
            .collect()
    }

    fn special_routine_rules<const N: usize>(
        ids: [&str; N],
    ) -> BTreeMap<String, SpecialRoutineRule> {
        ids.into_iter()
            .map(|id| (id.to_string(), SpecialRoutineRule::default()))
            .collect()
    }

    fn item_payload(items: Vec<Item>) -> BTreeMap<String, Item> {
        items
            .into_iter()
            .map(|item| (item.script_name.clone(), item))
            .collect()
    }

    fn move_payload(moves: Vec<Move>) -> BTreeMap<String, Move> {
        moves
            .into_iter()
            .map(|move_data| (move_data.name.clone(), move_data))
            .collect()
    }

    fn pokemon_payload(species: Vec<PokemonSpecies>) -> BTreeMap<String, PokemonSpecies> {
        species
            .into_iter()
            .map(|species| (species.id.clone(), species))
            .collect()
    }

    fn wild_encounter_payload(
        encounters: Vec<WildEncounterData>,
    ) -> BTreeMap<String, WildEncounterData> {
        encounters
            .into_iter()
            .map(|encounter| (encounter.map_name.clone(), encounter))
            .collect()
    }

    fn field_encounter_payload(
        encounters: Vec<FieldEncounterData>,
    ) -> BTreeMap<String, FieldEncounterData> {
        encounters
            .into_iter()
            .map(|encounter| (encounter.map_name.clone(), encounter))
            .collect()
    }

    fn map_payload(maps: Vec<MapModule>) -> BTreeMap<String, MapModule> {
        maps.into_iter().map(|map| (map.id.clone(), map)).collect()
    }

    fn growth_rate_payload(
        curves: Vec<crystal_core::systems::experience::GrowthRateCurve>,
    ) -> BTreeMap<String, crystal_core::systems::experience::GrowthRateCurve> {
        curves
            .into_iter()
            .map(|curve| (curve.id.clone(), curve))
            .collect()
    }

    fn test_item(id: &str) -> Item {
        Item {
            name: id.to_string(),
            description: "A test item.".to_string(),
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
            pocket: item_pocket("ITEM"),
            field_menu: "ITEMMENU_NOUSE".to_string(),
            field_usable: false,
            battle_menu: "ITEMMENU_NOUSE".to_string(),
            battle_usable: false,
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn test_phone_contact(contact_id: &str) -> PhoneContactRecord {
        PhoneContactRecord {
            contact_id: contact_id.to_string(),
            trainer_class: Some("TRAINER_NONE".to_string()),
            trainer_label: Some(format!("PHONECONTACT_{contact_id}")),
            lines: vec![format!("{contact_id}:")],
            primary_label: contact_id.to_string(),
            map_constant: None,
            callee_time_mask: 7,
            callee_script: None,
            caller_time_mask: 0,
            caller_script: None,
        }
    }

    fn test_runtime_spawn_point(identifier: u16, map_name: &str) -> RuntimeSpawnPoint {
        RuntimeSpawnPoint {
            identifier,
            map_constant: "ROUTE_29".to_string(),
            map_name: map_name.to_string(),
            group_id: 1,
            map_id: 1,
            tile_x: 0,
            tile_y: 0,
            group_name: "GROUP_ROUTE_29".to_string(),
            metatile_x: 0,
            metatile_y: 0,
            subtile_x: 0,
            subtile_y: 0,
        }
    }

    fn test_runtime_map_metadata(constant: &str, name: &str) -> RuntimeMapMetadata {
        RuntimeMapMetadata {
            constant: constant.to_string(),
            name: name.to_string(),
            group_name: "GROUP_ROUTE_29".to_string(),
            group_id: 1,
            map_id: 1,
            width: 10,
            height: 9,
            environment: "TOWN".to_string(),
            phone_service: 1,
        }
    }

    fn species() -> PokemonSpecies {
        PokemonSpecies {
            id: "NEW_MON".to_string(),
            int_id: 252,
            base_stats: BaseStats::new(40, 50, 40, 60, 70, 50),
            type1: pokemon_type("ELECTRIC"),
            type2: pokemon_type("ELECTRIC"),
            catch_rate: 45,
            base_exp: 80,
            item1: None,
            item2: None,
            gender_ratio: 127,
            unknown1: 0,
            step_cycles_to_hatch: 20,
            unknown2: 0,
            growth_rate: growth_rate("GROWTH_MEDIUM_FAST"),
            egg_group1: egg_group("EGG_GROUND"),
            egg_group2: egg_group("EGG_GROUND"),
            tmhm_learnset: vec!["THUNDERBOLT".to_string()],
            ability: ability("NONE"),
            pic_size: 0,
            front_pic: 0,
            back_pic: 0,
            weight: 120,
        }
    }

    fn test_move(name: &str) -> Move {
        Move {
            name: name.to_string(),
            move_type: pokemon_type("NORMAL"),
            power: 40,
            accuracy: 100,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    fn test_battle_stat_multipliers() -> BattleStatMultiplierTables {
        let identity = crystal_core::battle::stats::BattleStatMultiplier {
            numerator: 1,
            denominator: 1,
        };
        BattleStatMultiplierTables {
            stat: vec![identity; 13],
            accuracy: vec![identity; 13],
        }
    }

    fn test_weather_modifiers() -> WeatherModifiers {
        serde_json::from_value(serde_json::json!({
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
        .expect("weather modifier fixture should parse")
    }

    fn test_type_effectiveness() -> TypeEffectivenessTable {
        let types = ["NORMAL", "FIGHTING", "FIRE", "WATER"];
        let matchups = types
            .iter()
            .map(|attacker| {
                (
                    (*attacker).to_string(),
                    types
                        .iter()
                        .map(|defender| {
                            (
                                (*defender).to_string(),
                                crystal_core::battle::damage::TypeMultiplier::one(),
                            )
                        })
                        .collect(),
                )
            })
            .collect();
        let foresight_matchups = [(
            "NORMAL".to_string(),
            [(
                "FIGHTING".to_string(),
                crystal_core::battle::damage::TypeMultiplier::zero(),
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();
        TypeEffectivenessTable {
            matchups,
            foresight_matchups,
        }
    }

    fn test_type_categories() -> TypeCategories {
        TypeCategories {
            physical: vec!["NORMAL".to_string(), "FIGHTING".to_string()],
            special: vec!["FIRE".to_string(), "WATER".to_string()],
        }
    }

    fn test_move_priorities() -> MovePriorityTable {
        MovePriorityTable {
            base_priority: 1,
            effect_priorities: [
                ("PRIORITY_HIT".to_string(), 2),
                ("NORMAL_HIT".to_string(), 1),
            ]
            .into_iter()
            .collect(),
            move_priorities: vec![crystal_core::battle::turn::MovePriorityOverride {
                r#move: "VITAL_THROW".to_string(),
                priority: 0,
            }],
        }
    }

    fn test_buena_password_categories() -> BuenaPasswordCategories {
        BuenaPasswordCategories {
            order: vec!["HealingItems".to_string()],
            categories: BTreeMap::from([(
                "HealingItems".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_ITEM".to_string(),
                    points: 12,
                    options: vec!["POTION".to_string()],
                },
            )]),
        }
    }

    fn test_battle_escape_rules() -> BattleEscapeRules {
        BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        }
    }

    fn test_battle_reward_rules() -> BattleRewardRules {
        BattleRewardRules {
            max_level: 100,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
        }
    }

    fn test_step_event_rules() -> StepEventRules {
        StepEventRules {
            poison_step_interval: 4,
            egg_step_trigger: 1,
            hatched_egg_happiness: 120,
            poison_status: "PSN".to_string(),
            egg_nickname: "EGG".to_string(),
            happiness_step_counter_mask: 255,
            happiness_step_counter_target: 0,
        }
    }

    fn add_test_growth_rates(data: &mut GameDataSet) {
        data.growth_rates.insert(
            "GROWTH_MEDIUM_FAST".to_string(),
            crystal_core::systems::experience::GrowthRateCurve {
                id: "GROWTH_MEDIUM_FAST".to_string(),
                numerator: 1,
                denominator: 1,
                quadratic: 0,
                linear: 0,
                constant: 0,
            },
        );
    }

    fn add_runtime_species_and_move(data: &mut GameDataSet) {
        let mut known_species = species();
        known_species.tmhm_learnset.clear();
        let species_id = known_species.id.clone();
        data.pokemon.insert(species_id.clone(), known_species);
        let mut rattata = species();
        rattata.id = "RATTATA".to_string();
        rattata.tmhm_learnset.clear();
        data.pokemon.insert(rattata.id.clone(), rattata);
        add_test_growth_rates(data);
        data.moves.insert("TACKLE".to_string(), test_move("TACKLE"));
        for move_id in [
            "VITAL_THROW",
            "SOLARBEAM",
            "CUT",
            "WHIRLPOOL",
            "STRENGTH",
            "FLASH",
            "SURF",
            "WATERFALL",
            "FLY",
            "DIG",
            "TELEPORT",
            "HEADBUTT",
            "ROCK_SMASH",
            "SWEET_SCENT",
        ] {
            data.moves.entry(move_id.to_string()).or_insert_with(|| {
                let mut move_data = test_move(move_id);
                if move_id == "SOLARBEAM" {
                    move_data.effect = "SOLARBEAM".to_string();
                }
                move_data
            });
        }
        data.battle_stat_multipliers = test_battle_stat_multipliers();
        data.battle_escape_rules = test_battle_escape_rules();
        data.move_priorities = test_move_priorities();
        data.type_categories = test_type_categories();
        data.type_effectiveness = test_type_effectiveness();
        data.weather_modifiers = test_weather_modifiers();
        data.learnsets.insert(species_id.clone(), Vec::new());
        data.evolutions.0.insert(species_id.clone(), Vec::new());
        data.menu_icons
            .insert(species_id.clone(), "ICON_PIKACHU".to_string());
        data.pokedex_entries.insert(
            species_id.clone(),
            RuntimePokedexEntry {
                species: species_id.clone(),
                classification: "SPARK".to_string(),
                height_digits: 4,
                weight_digits: 60,
                pages: vec!["Stores static in its fur.".to_string()],
            },
        );
        data.pokemon_frontpic_anim.insert(
            species_id.clone(),
            FrontpicAnimProgram {
                commands: vec![FrontpicAnimCommand {
                    kind: "endanim".to_string(),
                    ..FrontpicAnimCommand::default()
                }],
            },
        );
        data.pokemon_cries.insert(
            species_id.clone(),
            PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        data.learnsets.entry("RATTATA".to_string()).or_default();
        data.evolutions.0.entry("RATTATA".to_string()).or_default();
        data.menu_icons
            .insert("RATTATA".to_string(), "ICON_PIKACHU".to_string());
        data.pokedex_entries.insert(
            "RATTATA".to_string(),
            RuntimePokedexEntry {
                species: "RATTATA".to_string(),
                classification: "MOUSE".to_string(),
                height_digits: 3,
                weight_digits: 35,
                pages: vec!["A test rodent.".to_string()],
            },
        );
        data.pokemon_frontpic_anim.insert(
            "RATTATA".to_string(),
            FrontpicAnimProgram {
                commands: vec![FrontpicAnimCommand {
                    kind: "endanim".to_string(),
                    ..FrontpicAnimCommand::default()
                }],
            },
        );
        data.pokemon_cries.insert(
            "RATTATA".to_string(),
            PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: 0,
                length: 0,
            },
        );
    }

    fn test_pcm_format() -> ModpackPcmAudioFormat {
        ModpackPcmAudioFormat {
            sample_rate_hz: 8000,
            channels: 1,
            bits_per_sample: 8,
        }
    }

    fn test_poke_ball() -> Item {
        let mut item = test_item("POKE_BALL");
        item.pocket = item_pocket("BALL");
        item.battle_menu = "ITEMMENU_CURRENT".to_string();
        item.battle_usable = true;
        item.battle_capture_ball = Some(true);
        item.consumable = true;
        item
    }

    fn test_tm_item() -> Item {
        let mut item = test_item("TM01");
        item.pocket = item_pocket("TM_HM");
        item.tmhm_index = Some(1);
        item.tmhm_move = Some("TACKLE".to_string());
        item
    }

    fn add_complete_runtime_pack_fixture(data: &mut GameDataSet) {
        add_runtime_species_and_move(data);
        if !data.story_events.iter().any(|payload| {
            payload
                .as_object()
                .is_some_and(|payload| payload.contains_key("StandardScripts"))
        }) {
            data.story_events.push(serde_json::json!({
                "StandardScripts": {
                    "StdScripts": [
                        { "command": "add_stdscript", "args": ["TestStandardScript"] }
                    ],
                    "TestStandardScript": [
                        { "command": "end", "args": [] }
                    ]
                }
            }));
        }
        data.items
            .entry("POKE_BALL".to_string())
            .or_insert_with(test_poke_ball);
        data.items
            .entry("TM01".to_string())
            .or_insert_with(test_tm_item);
        data.items
            .entry("POTION".to_string())
            .or_insert_with(|| test_item("POTION"));
        data.items.entry("OLD_ROD".to_string()).or_insert_with(|| {
            let mut item = test_item("OLD_ROD");
            item.field_menu = "ITEMMENU_CLOSE".to_string();
            item.field_usable = true;
            item
        });
        let mut escape_rope = test_item("ESCAPE_ROPE");
        escape_rope.effect = "ESCAPE_ROPE".to_string();
        escape_rope.escape_rope_mode = Some("ESCAPE_ROPE".to_string());
        escape_rope.field_menu = "ITEMMENU_CURRENT".to_string();
        escape_rope.field_usable = true;
        data.items.insert("ESCAPE_ROPE".to_string(), escape_rope);
        let mut repel = test_item("REPEL");
        repel.effect = "REPEL".to_string();
        repel.repel_steps = Some(100);
        repel.field_menu = "ITEMMENU_CURRENT".to_string();
        repel.field_usable = true;
        data.items.insert("REPEL".to_string(), repel);
        for (item_id, effect) in [
            ("BICYCLE", "BICYCLE"),
            ("ITEMFINDER", "ITEMFINDER"),
            ("SQUIRTBOTTLE", "SQUIRTBOTTLE"),
            ("COIN_CASE", "COIN_CASE"),
            ("BLUE_CARD", "BLUE_CARD"),
            ("TOWN_MAP", "TOWN_MAP"),
            ("POKEGEAR", "POKEGEAR"),
        ] {
            let mut item = test_item(item_id);
            item.effect = effect.to_string();
            item.pocket = item_pocket("KEY_ITEM");
            item.field_menu = "ITEMMENU_CLOSE".to_string();
            item.field_usable = true;
            data.items.insert(item_id.to_string(), item);
        }
        data.capture_rules.ball_rules.insert(
            "POKE_BALL".to_string(),
            CaptureBallRule {
                multiplier_numerator: 1,
                multiplier_denominator: 1,
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                skip_hp_calc: false,
                use_heavy_ball_weight_modifier: false,
                use_level_ball_multiplier: false,
                require_same_species: false,
                require_same_gender: false,
                require_fast_species: false,
            },
        );
        data.capture_wobble_probabilities = vec![
            CaptureWobbleProbability {
                catch_rate: 1,
                chance: 0,
            },
            CaptureWobbleProbability {
                catch_rate: u8::MAX,
                chance: u8::MAX,
            },
        ];
        data.battle_reward_rules = test_battle_reward_rules();
        data.battle_escape_rules = test_battle_escape_rules();
        data.move_priorities
            .effect_priorities
            .insert("SOLARBEAM".to_string(), 1);
        data.marts
            .0
            .insert("MART_TEST".to_string(), vec!["POTION".to_string()]);
        data.currency_constants
            .0
            .insert("MAX_MONEY".to_string(), 999_999);
        data.currency_constants
            .0
            .insert("MAX_COINS".to_string(), 9_999);
        data.step_event_rules = test_step_event_rules();
        if data.fishing.groups.is_empty() && data.fishing.rod_items.is_empty() {
            data.fishing = serde_json::from_value(serde_json::json!({
                "groups": {
                    "test": {
                        "bite_threshold": 128,
                        "rod_tables": {
                            "OLD_ROD": {
                                "slots": [
                                    { "threshold": 255, "species": "RATTATA", "level": 5, "time_group": null }
                                ]
                            }
                        }
                    }
                },
                "time_groups": {},
                "swarm_rules": {},
                "rod_items": { "OLD_ROD": "OLD_ROD" }
            }))
            .expect("complete fishing fixture should parse");
        }
        data.fruit_trees
            .0
            .insert("FRUITTREE_TEST".to_string(), "POTION".to_string());
        if data.field_moves == FieldMoveCatalog::default() {
            data.field_moves = test_field_move_catalog();
        }
        data.runtime_title_screen = RuntimeTitleScreen {
            new_game_spawn_identifier: Some(1),
            title_music: Some("MUSIC_TITLE".to_string()),
        };
        data.trainers
            .trainers
            .entry("YOUNGSTER_JOEY".to_string())
            .or_insert_with(|| test_trainer("YOUNGSTER_JOEY", "MUSIC_TITLE"));
        let first_map = data.maps.keys().next().cloned();
        if let Some(first_map) = first_map {
            if let Some(module) = data.maps.get_mut(&first_map) {
                module
                    .scripts
                    .entry("ObjectScript".to_string())
                    .or_insert_with(|| serde_json::json!([]));
                if module.objects.is_empty() {
                    module.objects.push(test_object("TEST_OBJECT", "", 0, 0));
                }
            }
            data.runtime_spawn_points.insert(
                "1".to_string(),
                RuntimeSpawnPoint {
                    identifier: 1,
                    map_constant: data
                        .maps
                        .get(&first_map)
                        .and_then(|module| module.attributes.map_constant.clone())
                        .unwrap_or_else(|| "START_MAP".to_string()),
                    map_name: first_map.clone(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 0,
                    tile_y: 0,
                    group_name: "GROUP_TEST".to_string(),
                    metatile_x: 0,
                    metatile_y: 0,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            );
        }
        for (map_name, module) in &data.maps {
            data.map_attributes
                .insert(map_name.clone(), module.attributes.clone());
            let constant = module
                .attributes
                .map_constant
                .clone()
                .unwrap_or_else(|| map_name.to_string());
            data.runtime_map_metadata.insert(
                constant.clone(),
                RuntimeMapMetadata {
                    constant,
                    name: map_name.clone(),
                    group_name: "GROUP_TEST".to_string(),
                    group_id: 1,
                    map_id: 1,
                    width: module.attributes.width,
                    height: module.attributes.height,
                    environment: "ROUTE".to_string(),
                    phone_service: 1,
                },
            );
        }
        data.audio.push(
            ModpackAudioAsset::pcm(
                "MUSIC_TITLE",
                "content-packs/test/music/MUSIC_TITLE.pcm",
                ModpackAudioKind::Music,
                test_pcm_format(),
            )
            .expect("music PCM fixture"),
        );
        data.audio.push(
            ModpackAudioAsset::pcm(
                "SFX_ITEM",
                "content-packs/test/sfx/SFX_ITEM.pcm",
                ModpackAudioKind::SoundEffect,
                test_pcm_format(),
            )
            .expect("sfx PCM fixture"),
        );
        data.audio.push(
            ModpackAudioAsset::pcm(
                "CRY_CHIKORITA",
                "content-packs/test/cries/CRY_CHIKORITA.pcm",
                ModpackAudioKind::Cry,
                test_pcm_format(),
            )
            .expect("cry PCM fixture"),
        );
        data.tilesets.entry("johto".to_string()).or_insert_with(|| {
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
            tileset
        });
        data.pc_strings
            .insert("PLAYER_PC".to_string(), "Player's PC".to_string());
        data.move_names = data.moves.keys().cloned().collect();
        data.asm_text
            .insert("OakRating01".to_string(), "Good work!".to_string());
        data.battle_animations.insert(
            "BattleAnim_Pound".to_string(),
            vec!["anim_wait 1".to_string()],
        );
        data.battle_animation_table = std::iter::once("BattleAnim_Pound".to_string())
            .chain(data.moves.keys().map(|_| "BattleAnim_Pound".to_string()))
            .collect();
        data.battle_anim_bundle = serde_json::to_string(&complete_battle_anim_bundle_payload())
            .expect("battle animation bundle fixture");
        data.sprite_anim_bundle = serde_json::to_string(&complete_sprite_anim_bundle_payload())
            .expect("sprite animation bundle fixture");
        data.sprite_palette_defaults
            .insert("SPRITE_MON".to_string(), 0);
        data.pokegear_town_map_palette_map
            .insert("johto".to_string(), vec!["PAL_ROUTE".to_string()]);
        data.pokegear_landmarks.landmarks.push(PokegearLandmark {
            id: 1,
            constant: "LANDMARK_START".to_string(),
            label: "Start".to_string(),
            name: "Start".to_string(),
            x: 0,
            y: 0,
            region: "johto".to_string(),
        });
        if let Some(first_map) = data.maps.keys().next().cloned() {
            data.pokegear_landmarks
                .map_to_landmark
                .insert(first_map, "LANDMARK_START".to_string());
        }
        data.phone_contacts.0.insert(
            "TEST_CONTACT".to_string(),
            test_phone_contact("TEST_CONTACT"),
        );
        data.permanent_phone_numbers.insert(
            "TEST_CONTACT".to_string(),
            PermanentPhoneNumberRule::default(),
        );
        data.special_phone_calls
            .insert("TEST_CALL".to_string(), SpecialPhoneCallRule::default());
        data.phone_scripts
            .push(serde_json::json!({"id": "TEST_PHONE"}));
        data.flee_mons
            .buckets
            .insert("test".to_string(), vec!["RATTATA".to_string()]);
        data.buena_password_categories = test_buena_password_categories();
        data.roaming_pokemon.insert(
            "RATTATA".to_string(),
            RoamingPokemonDefinition {
                level: 5,
                map_group: 1,
                map_number: 1,
            },
        );
        data.buena_prizes.insert("POTION".to_string(), 1);
        data.kurt_apricorn_recipes
            .insert("POTION".to_string(), "POKE_BALL".to_string());
        data.shuckie_gift = Some(ShuckieGiftDefinition {
            species: "RATTATA".to_string(),
            level: 5,
            held_item: "POTION".to_string(),
            nickname: "SHUCKIE".to_string(),
            original_trainer_name: "MANIA".to_string(),
            original_trainer_id: 518,
            got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
        });
        data.dratini_move_sets.insert(1, vec!["TACKLE".to_string()]);
        data.initialize_events
            .event_flags
            .push("EVENT_BUG_CONTESTANT_1".to_string());
        data.initialize_events
            .engine_flags
            .push("ENGINE_GOT_SHUCKIE_TODAY".to_string());
        data.bug_contest_config = Some(BugContestConfig {
            park_balls: 20,
            timer_minutes: 20,
            timer_seconds: 0,
            selected_contestant_count: 1,
            contestant_flags: vec!["EVENT_BUG_CONTESTANT_1".to_string()],
        });
        data.battle_tower_rules = Some(BattleTowerRules {
            banned_species: BTreeMap::new(),
            required_party_count: 3,
            challenge_streak_length: 7,
            minimum_level_group: 1,
            maximum_level_group: 10,
            level_group_size: 10,
            party_count_failure_text: "OnlyThreeMonMayBeEnteredText".to_string(),
            duplicate_species_failure_text: "TheMonMustAllBeDifferentKindsText".to_string(),
            duplicate_held_item_failure_text: "TheMonMustNotHoldTheSameItemsText".to_string(),
            egg_failure_text: "YouCantTakeAnEggText".to_string(),
            trainers: test_battle_tower_trainers(),
            mon_groups: test_battle_tower_mon_groups(),
        });
        data.oak_ratings.push(OakRatingEntry {
            caught_count_limit: data.pokemon.len(),
            fanfare: "SFX_ITEM".to_string(),
            text_label: "OakRating01".to_string(),
        });
        data.odd_egg_definitions.push(OddEggDefinition {
            species: "RATTATA".to_string(),
            moves: vec!["TACKLE".to_string()],
            original_trainer_id: 768,
            dvs: [2, 10, 10, 10],
            probability: 100,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        });
        data.magikarp_lengths.push(MagikarpLengthEntry {
            threshold: 100,
            divisor: 1,
        });
        data.happiness_data = Some(
            serde_json::from_value(serde_json::json!({
                "changes": { "1": { "code": "GAIN_LEVEL", "low": 5, "mid": 3, "high": 2 } },
                "services": {
                    "HaircutBrother": [
                        { "rollWeight": 1, "scriptValue": 0, "changeCode": 1 }
                    ]
                }
            }))
            .expect("happiness fixture should parse"),
        );
        data.story_event_script_constants
            .global
            .insert("EVENT_CHAMPION_DEFEATED".to_string(), 1);
    }

    fn verify_complete_test_game_data(
        data: &GameDataSet,
        rules: &PlayabilityRules,
    ) -> ModpackCompileReport {
        let mut data = data.clone();
        add_complete_runtime_pack_fixture(&mut data);
        let root = repository_root_for_tests();
        write_complete_runtime_audio_fixture(&root);
        verify_game_data(&AssetRoot::new(root), &data, rules)
    }

    fn write_complete_runtime_audio_fixture(root: &Path) {
        for path in [
            "content-packs/test/music/MUSIC_TITLE.pcm",
            "content-packs/test/sfx/SFX_ITEM.pcm",
            "content-packs/test/cries/CRY_CHIKORITA.pcm",
        ] {
            let path = root.join("apps/web/assets/data").join(path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create complete runtime audio fixture dir");
            }
            std::fs::write(path, [0_u8]).expect("write complete runtime audio fixture");
        }
    }

    fn add_wild_encounter_marker(data: &mut GameDataSet) {
        data.wild_encounters.insert(
            "Route29".to_string(),
            WildEncounterData {
                map_name: "Route29".to_string(),
                grass_rates: Some([("day".to_string(), 30)].into_iter().collect()),
                water_rate: None,
                grass: None,
                water: None,
            },
        );
    }

    fn add_test_trainer(data: &mut GameDataSet, encounter_music: &str) {
        data.trainers.trainers.insert(
            "YOUNGSTER_JOEY".to_string(),
            test_trainer("YOUNGSTER_JOEY", encounter_music),
        );
    }

    fn test_trainer(trainer_id: &str, encounter_music: &str) -> Trainer {
        Trainer {
            name: "Joey".to_string(),
            trainer_id: trainer_id.to_string(),
            trainer_class: "YOUNGSTER".to_string(),
            party: vec![TrainerPartyPokemon {
                species: "RATTATA".to_string(),
                level: 4,
                item: None,
                moves: vec![crystal_core::models::LearnedMove {
                    name: "TACKLE".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                }],
                dvs: crystal_core::models::Dv::default(),
            }],
            win_quote: "I won!".to_string(),
            lose_quote: "I lost!".to_string(),
            items: Vec::new(),
            base_reward: 4,
            ai_move_flags: 0,
            ai_item_switch_flags: 0,
            encounter_music: encounter_music.to_string(),
            ai_layers: Vec::new(),
        }
    }

    #[test]
    fn active_wild_battle_escape_rejects_stale_rng_seed_before_mutation() {
        let mut data = GameDataSet::default();
        data.battle_escape_rules = test_battle_escape_rules();
        data.battle_stat_multipliers = test_battle_stat_multipliers();
        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let enemy = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            rng_seed: 7,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        let before = state.clone();
        let mut preview = state.clone();
        data.resolve_active_wild_battle_run(&mut preview)
            .expect("preview escape");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeBattleEscapeMap".to_string(),
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
                RuntimeMutationCommand::AttemptEscapeActiveWildBattle(RuntimeBattleEscapeCommand {
                    rng_seed_after: 0,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale escape rng seed must reject");

        assert!(
            error
                .to_string()
                .contains("attempt active wild battle escape rng_seed_after 0 does not match")
        );
        assert_eq!(state, before);
    }

    #[test]
    fn runtime_blackout_recovery_consumes_blackout_marker() {
        let mut data = GameDataSet::default();
        data.moves.insert("TACKLE".to_string(), test_move("TACKLE"));
        data.runtime_spawn_points.insert(
            "2".to_string(),
            test_runtime_spawn_point(2, "PlayersHouse2F"),
        );
        data.runtime_map_metadata.insert(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "PlayersHouse2F"),
        );
        data.special_routines = special_routine_rules(["WarpToSpawnPoint"]);
        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            5,
            crystal_core::models::Dv::default(),
        );
        let mut state = GameState {
            last_spawn_identifier: Some(2),
            script_runtime: ScriptRuntimeMemory {
                blackout_mod: Some("DARK_CAVE".to_string()),
                ..ScriptRuntimeMemory::default()
            },
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route30".to_string(),
                enemy_pokemon: player.clone(),
                enemy_party: vec![player.clone()],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        state.sync_party_from_storage();
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "Route30".to_string(),
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
                RuntimeMutationCommand::ResolveBlackoutToLastSpawn,
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("resolve blackout");

        let RuntimeMutationResult::BlackoutResolved(recovery) = outcome.result else {
            panic!("expected blackout recovery result");
        };
        assert_eq!(recovery.spawn_identifier, Some(2));
        assert_eq!(recovery.map_name, "PlayersHouse2F");
        assert_eq!(state.script_runtime.blackout_mod, None);
        assert!(matches!(state.battle, BattleMemory::Inactive));
        assert_eq!(
            state
                .script_runtime
                .pending_script_warp
                .as_ref()
                .map(|warp| (warp.target_map.as_str(), warp.tile)),
            Some(("PlayersHouse2F", TilePosition::new(0, 0)))
        );
        assert_eq!(
            state.overworld,
            OverworldMemory::Active {
                map_name: "PlayersHouse2F".to_string(),
                tile: TilePosition::new(0, 0),
                facing: Direction::Down,
                mode: MovementMode::Normal
            }
        );
    }

    #[test]
    fn active_battle_escape_item_is_atomic_when_pay_day_claim_rejects() {
        let mut data = GameDataSet::default();
        let mut escape_item = test_item("POKE_DOLL");
        escape_item.battle_menu = "ITEMMENU_CURRENT".to_string();
        escape_item.battle_usable = true;
        escape_item.battle_escape_mode = Some("WILD_BATTLE".to_string());
        escape_item.consumable = true;
        data.items
            .insert(escape_item.script_name.clone(), escape_item);

        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let enemy = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            battle_pay_day_money: 50,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        state
            .bag
            .add_item(&data.items["POKE_DOLL"], 1)
            .expect("add escape item");

        let error = data
            .use_bag_item_to_escape_active_wild_battle(&mut state, "POKE_DOLL")
            .expect_err("missing MAX_MONEY must reject after staged escape item use");

        assert!(
            format!("{error:#}").contains("currency constants missing MAX_MONEY"),
            "{error:#}"
        );
        assert_eq!(state.bag.quantity(&data.items["POKE_DOLL"]), 1);
        assert!(state.script_runtime.item_use_events.is_empty());
        assert!(matches!(state.battle, BattleMemory::Wild { .. }));
        assert_eq!(state.battle_pay_day_money, 50);
    }

    #[test]
    fn active_wild_battle_run_is_atomic_when_pay_day_claim_rejects() {
        let mut data = GameDataSet::default();
        data.battle_escape_rules = BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: u16::MAX,
            rng_roll_values: 256,
        };
        data.battle_stat_multipliers = test_battle_stat_multipliers();
        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let enemy = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            battle_escape_attempts: u8::MAX,
            battle_pay_day_money: 50,
            rng_seed: 7,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        let before = state.clone();

        let error = data
            .resolve_active_wild_battle_run(&mut state)
            .expect_err("missing MAX_MONEY must reject after staged successful escape");

        assert!(
            format!("{error:#}").contains("currency constants missing MAX_MONEY"),
            "{error:#}"
        );
        assert_eq!(state, before);
    }

    #[test]
    fn active_wild_battle_reward_claim_is_atomic_when_pay_day_claim_rejects() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        add_test_growth_rates(&mut data);
        data.battle_reward_rules = test_battle_reward_rules();
        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let mut enemy = crystal_core::models::Pokemon::new_for_tests(
            species(),
            5,
            crystal_core::models::Dv::default(),
        );
        enemy.hp = 0;
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            battle_pay_day_money: 50,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        let before = state.clone();

        let error = data
            .claim_active_wild_battle_rewards(&mut state, TimeOfDay::Day)
            .expect_err("missing MAX_MONEY must reject after staged reward claim");

        assert!(
            format!("{error:#}").contains("currency constants missing MAX_MONEY"),
            "{error:#}"
        );
        assert_eq!(state, before);
    }

    #[test]
    fn active_battle_ball_throw_rejects_stale_rng_seed_before_mutation() {
        let mut data = GameDataSet::default();
        let mut ball = test_item("POKE_BALL");
        ball.pocket = item_pocket(ITEM_POCKET_BALL);
        ball.battle_menu = "ITEMMENU_CURRENT".to_string();
        ball.battle_usable = true;
        data.items.insert(ball.script_name.clone(), ball.clone());
        data.capture_rules = CaptureRules {
            fast_ball_species: BTreeSet::new(),
            heavy_ball_modifiers: BTreeMap::new(),
            ball_rules: [(
                "POKE_BALL".to_string(),
                CaptureBallRule {
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
        };
        data.capture_wobble_probabilities = vec![CaptureWobbleProbability {
            catch_rate: u8::MAX,
            chance: u8::MAX,
        }];
        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let mut enemy_species = species();
        enemy_species.catch_rate = 45;
        let enemy = crystal_core::models::Pokemon::new_for_tests(
            enemy_species,
            20,
            crystal_core::models::Dv::default(),
        );
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            rng_seed: 11,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        state
            .bag
            .add_item(&ball, 1)
            .expect("add ball to bag for capture test");
        let before = state.clone();
        let mut preview = state.clone();
        data.throw_ball_at_active_battle(&mut preview, "POKE_BALL")
            .expect("preview ball throw");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeBallThrowMap".to_string(),
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
                RuntimeMutationCommand::ThrowBallAtActiveBattle(RuntimeBattleItemCommand {
                    item_id: "POKE_BALL".to_string(),
                    rng_seed_after: 0,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale ball throw rng seed must reject");

        assert!(
            error
                .to_string()
                .contains("throw ball at active battle rng_seed_after 0 does not match")
        );
        assert_eq!(state, before);
    }

    #[test]
    fn active_wild_capture_rejects_full_storage_without_consuming_ball_or_rng() {
        let mut data = GameDataSet::default();
        let mut ball = test_item("POKE_BALL");
        ball.pocket = item_pocket(ITEM_POCKET_BALL);
        ball.battle_menu = "ITEMMENU_CURRENT".to_string();
        ball.battle_usable = true;
        data.items.insert(ball.script_name.clone(), ball.clone());
        let pokemon = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let enemy = pokemon.clone();
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            rng_seed: 11,
            ..GameState::default()
        };
        for slot in &mut state.storage.party.pokemon {
            *slot = Some(pokemon.clone());
        }
        for box_index in 0..MAX_PC_BOXES {
            let mut pc_box = PcBox::new(box_index);
            for slot in 0..MAX_BOX_MONS {
                pc_box.set_slot(slot, Some(pokemon.clone()));
            }
            state.storage.pc_boxes.push(pc_box);
        }
        state
            .bag
            .add_item(&ball, 1)
            .expect("add ball to full-storage capture test");
        let rng_seed_before = state.rng_seed;

        let outcome = data
            .throw_ball_at_active_battle(&mut state, "POKE_BALL")
            .expect("full storage is a visible blocked capture outcome");

        assert!(outcome.blocked);
        assert!(outcome.storage_full);
        assert!(!outcome.caught);
        assert_eq!(outcome.animation_shakes, 0);
        assert_eq!(state.rng_seed, rng_seed_before);
        assert_eq!(state.bag.quantity(&ball), 1);
        assert!(matches!(state.battle, BattleMemory::Wild { .. }));
    }

    #[test]
    fn active_wild_capture_completion_is_atomic_when_pay_day_claim_rejects() {
        let data = GameDataSet::default();
        let player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let enemy = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let mut state = GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            battle_pay_day_money: 50,
            rng_seed: 11,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        let before = state.clone();
        let outcome = CaptureOutcome {
            caught: true,
            blocked: false,
            storage_full: false,
            wobble_count: 4,
            animation_shakes: 3,
            final_catch_rate: u8::MAX,
            rng_seed_after: state.rng_seed,
            ball_id: None,
        };

        let error = data
            .complete_active_wild_capture(&mut state, &outcome, None)
            .expect_err("missing MAX_MONEY must reject after staged capture completion");

        assert!(
            format!("{error:#}").contains("currency constants missing MAX_MONEY"),
            "{error:#}"
        );
        assert_eq!(state, before);
    }

    #[test]
    fn active_wild_capture_applies_chosen_nickname_to_party_and_pc_destinations() {
        let data = GameDataSet::default();
        let pokemon = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        let outcome = CaptureOutcome {
            caught: true,
            blocked: false,
            storage_full: false,
            wobble_count: 4,
            animation_shakes: 4,
            final_catch_rate: u8::MAX,
            rng_seed_after: 11,
            ball_id: Some("POKE_BALL".to_string()),
        };
        let make_state = || GameState {
            battle: BattleMemory::Wild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "Route29".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            rng_seed: 11,
            ..GameState::default()
        };

        let mut party_state = make_state();
        party_state.storage.party.pokemon[0] = Some(pokemon.clone());
        let party_completion = data
            .complete_active_wild_capture(&mut party_state, &outcome, Some("SPARKY"))
            .expect("complete named party capture");
        assert_eq!(party_completion.stored.as_ref().unwrap().pokemon.nickname, "SPARKY");
        assert_eq!(
            party_state.storage.party.pokemon[1]
                .as_ref()
                .unwrap()
                .nickname,
            "SPARKY"
        );

        let mut pc_state = make_state();
        for slot in &mut pc_state.storage.party.pokemon {
            *slot = Some(pokemon.clone());
        }
        let pc_completion = data
            .complete_active_wild_capture(&mut pc_state, &outcome, Some("BOXMON"))
            .expect("complete named PC capture");
        let stored = pc_completion.stored.as_ref().unwrap();
        assert_eq!(stored.pokemon.nickname, "BOXMON");
        assert_eq!(pc_state.storage.pc_boxes[0].pokemon[0].as_ref().unwrap().nickname, "BOXMON");
        assert_eq!(pc_state.storage.pc_boxes[0].nicknames[0], "BOXMON");
    }

    #[test]
    fn deferred_level_evolution_rejects_existing_pending_move_learn_before_mutation() {
        let mut data = GameDataSet::default();
        let mut mon = species();
        mon.id = "NEW_MON".to_string();
        mon.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        let mut evolved = species();
        evolved.id = "NEW_MON_EVOLVED".to_string();
        evolved.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        data.pokemon.insert(mon.id.clone(), mon.clone());
        data.pokemon.insert(evolved.id.clone(), evolved);
        data.learnsets.insert(mon.id.clone(), Vec::new());
        data.learnsets
            .insert("NEW_MON_EVOLVED".to_string(), Vec::new());
        data.evolutions.0.insert(
            mon.id.clone(),
            vec![crystal_core::systems::evolution::EvolutionEntry::level(
                "NEW_MON_EVOLVED",
                16,
            )],
        );
        data.evolutions
            .0
            .insert("NEW_MON_EVOLVED".to_string(), Vec::new());
        let mut state = crystal_core::state::GameState::default();
        state.storage.party.pokemon[0] = Some(crystal_core::models::Pokemon::new_for_tests(
            mon,
            16,
            crystal_core::models::Dv::default(),
        ));
        state.pending_move_learn = Some(crystal_core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "NEW_MON".to_string(),
            level: 16,
            learned_move: crystal_core::models::LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            defer_level_evolution: true,
        });
        let before = state.clone();

        let error = data
            .resolve_deferred_level_evolution(
                &mut state,
                0,
                crystal_core::world::encounters::TimeOfDay::Day,
            )
            .expect_err("existing pending move learn must block deferred evolution");

        let error = format!("{error:#}");
        assert!(error.contains(
            "pending move learn already exists before resolving deferred level evolution for party index 0"
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn party_item_pending_move_learn_guard_runs_before_item_mutation() {
        let data = GameDataSet::default();
        let mut state = crystal_core::state::GameState::default();
        state.pending_move_learn = Some(crystal_core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "NEW_MON".to_string(),
            level: 16,
            learned_move: crystal_core::models::LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            defer_level_evolution: true,
        });
        let empty_effect = crystal_core::systems::battle_items::BattleItemOutcome {
            item_id: "RARE_CANDY".to_string(),
            hp_before: 10,
            hp_after: 10,
            level_before: 16,
            level_after: 16,
            experience_before: 0,
            experience_after: 0,
            status_before: None,
            status_after: None,
            confusion_turns_before: 0,
            confusion_turns_after: 0,
            focus_energy_before: false,
            focus_energy_after: false,
            pp_changes: Vec::new(),
            stat_changes: Vec::new(),
            battle_stat_stage_changes: Vec::new(),
            learned_moves: Vec::new(),
            pending_move_learns: Vec::new(),
            deferred_level_evolution: false,
            evolution_target: None,
            consumed: false,
        };
        data.require_no_existing_pending_move_learn_for_item_effect(&state, 0, &empty_effect)
            .expect("item effects without pending move learn can continue");

        let mut pending_effect = empty_effect.clone();
        pending_effect.pending_move_learns = vec![crystal_core::models::LearnedMove {
            name: "RAZOR_LEAF".to_string(),
            current_pp: 25,
            pp_ups: 0,
        }];
        let mut clear_state = state.clone();
        clear_state.pending_move_learn = None;
        data.require_no_existing_pending_move_learn_for_item_effect(
            &clear_state,
            0,
            &pending_effect,
        )
        .expect("new pending move learn can be queued when no prompt is active");

        let error = data
            .require_no_existing_pending_move_learn_for_item_effect(&state, 0, &pending_effect)
            .expect_err("existing pending move learn must block before item mutation");
        let error = format!("{error:#}");
        assert!(error.contains("pending move learn already exists for party index 0"));
    }

    #[test]
    fn verifier_rejects_missing_battle_escape_rules_without_formula_fallback() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.battle_escape_rules = BattleEscapeRules::default();

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.has_errors());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_battle_escape_rules"
                && diagnostic.subject == "battle_escape_rules"
        }));
    }

    #[test]
    fn verifier_requires_trainer_encounter_music_declared_by_pack() {
        let mut data = GameDataSet::default();
        add_test_trainer(&mut data, "");

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_trainer_encounter_music"
                && diagnostic.subject == "YOUNGSTER_JOEY"
        }));
    }

    #[test]
    fn verifier_requires_trainer_encounter_music_reference_exact_music_asset() {
        let mut data = GameDataSet::default();
        add_test_trainer(&mut data, "MUSIC_YOUNGSTER_ENCOUNTER");
        data.audio.push(ModpackAudioAsset {
            id: "SFX_TACKLE".to_string(),
            path: "content-packs/test/sfx/SFX_TACKLE.mid".to_string(),
            kind: ModpackAudioKind::SoundEffect,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        });

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_trainer_encounter_music"
                && diagnostic.subject == "YOUNGSTER_JOEY"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_trainer_encounter_music_id_before_lookup() {
        let mut data = GameDataSet::default();
        add_test_trainer(&mut data, "MUSIC YOUNGSTER ENCOUNTER");

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_trainer_encounter_music"
                && diagnostic.subject == "YOUNGSTER_JOEY"
                && diagnostic.message.contains("MUSIC YOUNGSTER ENCOUNTER")
        }));
    }

    #[test]
    fn verifier_rejects_scripted_battle_requests_without_runtime_fallbacks() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        add_test_trainer(&mut data, "MUSIC_YOUNGSTER_ENCOUNTER");
        data.audio.push(ModpackAudioAsset {
            id: "MUSIC_YOUNGSTER_ENCOUNTER".to_string(),
            path: "content-packs/test/music/MUSIC_YOUNGSTER_ENCOUNTER.mid".to_string(),
            kind: ModpackAudioKind::Music,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        });
        let known_species_id = data.pokemon.keys().next().expect("runtime species").clone();
        let mut module = test_map_module("Start", "START_MAP", None);
        module.trainer_scripts.insert(
            "TrainerScript".to_string(),
            TrainerBattleRequest::new("youngster", "YOUNGSTER_JOEY", "EVENT_BEAT_JOEY"),
        );
        module.scripted_trainer_battles = vec![
            ScriptedTrainerBattle {
                source_script: "LoadTrainerScript".to_string(),
                loadtrainer_command_index: 3,
                startbattle_command_index: 4,
                request: TrainerBattleRequest::new("YOUNGSTER", "youngster_joey", ""),
                reload_map_after_battle: false,
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
            },
            ScriptedTrainerBattle {
                source_script: "BadTrainerIdScript".to_string(),
                loadtrainer_command_index: 9,
                startbattle_command_index: 10,
                request: TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER JOEY", ""),
                reload_map_after_battle: false,
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
            },
            ScriptedTrainerBattle {
                source_script: "BadTrainerClassScript".to_string(),
                loadtrainer_command_index: 11,
                startbattle_command_index: 12,
                request: TrainerBattleRequest::new("YOUNG STER", "YOUNGSTER_JOEY", ""),
                reload_map_after_battle: false,
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
            },
        ];
        module.scripted_wild_battles = vec![
            ScriptedWildBattle {
                source_script: "WildCaseScript".to_string(),
                loadwildmon_command_index: 5,
                startbattle_command_index: 6,
                request: StaticWildBattleRequest::new(known_species_id.to_lowercase(), 10),
                reload_map_after_battle: false,
                pre_battle_event_flags: Vec::new(),
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
                disappear_object_ids: Vec::new(),
            },
            ScriptedWildBattle {
                source_script: "WildZeroScript".to_string(),
                loadwildmon_command_index: 7,
                startbattle_command_index: 8,
                request: StaticWildBattleRequest::new(known_species_id, 0),
                reload_map_after_battle: false,
                pre_battle_event_flags: Vec::new(),
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
                disappear_object_ids: Vec::new(),
            },
            ScriptedWildBattle {
                source_script: "WildMalformedScript".to_string(),
                loadwildmon_command_index: 13,
                startbattle_command_index: 14,
                request: StaticWildBattleRequest::new("HO OT", 10),
                reload_map_after_battle: false,
                pre_battle_event_flags: Vec::new(),
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
                disappear_object_ids: Vec::new(),
            },
        ];
        data.maps.insert("Start".to_string(), module);

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "scripted_trainer_class_mismatch"
                && diagnostic.subject == "Start:TrainerScript"
                && diagnostic.message.contains("youngster")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_scripted_trainer"
                && diagnostic.subject == "Start:LoadTrainerScript:3"
                && diagnostic.message.contains("youngster_joey")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_scripted_wild_species"
                && diagnostic.subject == "Start:WildCaseScript:5"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_scripted_wild_level"
                && diagnostic.subject == "Start:WildZeroScript:7"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_scripted_trainer_id"
                && diagnostic.subject == "Start:BadTrainerIdScript:9"
                && diagnostic.message.contains("YOUNGSTER JOEY")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_scripted_trainer_class"
                && diagnostic.subject == "Start:BadTrainerClassScript:11"
                && diagnostic.message.contains("YOUNG STER")
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_scripted_wild_species"
                && diagnostic.subject == "Start:WildMalformedScript:13"
                && diagnostic.message.contains("HO OT")
        }));
        for subject in [
            "Start:BadTrainerIdScript:9",
            "Start:BadTrainerClassScript:11",
            "Start:WildMalformedScript:13",
        ] {
            assert!(!report.diagnostics.iter().any(|diagnostic| {
                diagnostic.subject == subject
                    && (diagnostic.code == "unknown_scripted_trainer"
                        || diagnostic.code == "unknown_scripted_wild_species"
                        || diagnostic.code == "scripted_trainer_class_mismatch")
            }));
        }
    }

    #[test]
    fn verifier_rejects_duplicate_scripted_battle_start_positions() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.scripted_trainer_battles = vec![
            ScriptedTrainerBattle {
                source_script: "BattleScript".to_string(),
                loadtrainer_command_index: 1,
                startbattle_command_index: 2,
                request: TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", ""),
                reload_map_after_battle: false,
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
            },
            ScriptedTrainerBattle {
                source_script: "BattleScript".to_string(),
                loadtrainer_command_index: 3,
                startbattle_command_index: 2,
                request: TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", ""),
                reload_map_after_battle: false,
                post_battle_event_flags: Vec::new(),
                post_battle_script_flags: Vec::new(),
            },
        ];
        let mut data = GameDataSet {
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
                && diagnostic.subject == "Start:scripted_trainer_battle_start:BattleScript:2"
        }));
    }

    #[test]
    fn verifier_rejects_trainer_objects_without_exact_battle_requests() {
        let mut module = test_map_module("Start", "START_MAP", None);
        let mut trainer = test_object("START_TRAINER", "-1", 1, 1);
        trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        trainer.script = "StartTrainerScript".to_string();
        module.objects = vec![trainer];
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
            diagnostic.code == "trainer_object_missing_battle_request"
                && diagnostic.subject == "Start:START_TRAINER"
                && diagnostic.message.contains("StartTrainerScript")
        }));
    }

    #[test]
    fn verifier_rejects_trainer_battle_requests_without_exact_objects() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.trainer_scripts.insert(
            "StartTrainerScript".to_string(),
            TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", "EVENT_BEAT_JOEY"),
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

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "trainer_battle_request_missing_object"
                && diagnostic.subject == "Start:trainer_script:StartTrainerScript"
                && diagnostic.message.contains("StartTrainerScript")
        }));
    }

    #[test]
    fn verifier_rejects_duplicate_trainer_object_scripts() {
        let mut module = test_map_module("Start", "START_MAP", None);
        let mut first_trainer = test_object("START_TRAINER_1", "-1", 1, 1);
        first_trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        first_trainer.script = "StartTrainerScript".to_string();
        let mut second_trainer = test_object("START_TRAINER_2", "-1", 2, 1);
        second_trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        second_trainer.script = "StartTrainerScript".to_string();
        module.objects = vec![first_trainer, second_trainer];
        module.trainer_scripts.insert(
            "StartTrainerScript".to_string(),
            TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", "EVENT_BEAT_JOEY"),
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

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "trainer_duplicate_object_script"
                && diagnostic.subject == "Start:StartTrainerScript"
                && diagnostic.message.contains("2 OBJECTTYPE_TRAINER objects")
        }));
    }

    #[test]
    fn verifier_rejects_trainer_object_event_flags_that_mismatch_battle_request() {
        let mut module = test_map_module("Start", "START_MAP", None);
        let mut trainer = test_object("START_TRAINER", "EVENT_HIDE_START_TRAINER", 1, 1);
        trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        trainer.script = "StartTrainerScript".to_string();
        module.objects = vec![trainer];
        module.trainer_scripts.insert(
            "StartTrainerScript".to_string(),
            TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", "EVENT_BEAT_START_TRAINER"),
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

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "trainer_object_event_flag_mismatch"
                && diagnostic.subject == "Start:START_TRAINER"
                && diagnostic.message.contains("EVENT_HIDE_START_TRAINER")
                && diagnostic.message.contains("EVENT_BEAT_START_TRAINER")
        }));
    }

    #[test]
    fn verifier_rejects_scripted_battle_effects_that_cannot_apply_at_runtime() {
        let mut module = test_map_module("Start", "START_MAP", None);
        module.objects = vec![test_object("VISIBLE_NPC", "-1", 1, 1)];
        module.scripted_wild_battles = vec![ScriptedWildBattle {
            source_script: "WildScript".to_string(),
            loadwildmon_command_index: 1,
            startbattle_command_index: 2,
            request: StaticWildBattleRequest::new("CHIKORITA", 5),
            reload_map_after_battle: false,
            pre_battle_event_flags: vec!["EVENT PRE".to_string()],
            post_battle_event_flags: vec!["0".to_string()],
            post_battle_script_flags: vec!["legacy_script_flag".to_string()],
            disappear_object_ids: vec![
                "MISSING_NPC".to_string(),
                "VISIBLE_NPC".to_string(),
                "BAD NPC".to_string(),
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

        for code in [
            "invalid_scripted_battle_effect_flag",
            "unknown_scripted_battle_disappear_object",
            "unhideable_scripted_battle_disappear_object",
            "invalid_scripted_battle_disappear_object",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == code && diagnostic.subject == "Start:WildScript:2"
                }),
                "missing {code}: {:?}",
                report.diagnostics
            );
        }
    }

    #[test]
    fn verifier_requires_fly_field_move_from_exact_modpack_rule() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.field_moves.fly = FieldMoveRule {
            move_id: "fly".to_string(),
            badge: crystal_core::systems::field_moves::FieldMoveBadgeRequirement {
                region: "johto".to_string(),
                index: 5,
            },
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_move_id" && diagnostic.subject == "field_moves:fly"
        }));
    }

    #[test]
    fn verifier_rejects_field_move_replacements_that_do_not_change_blocks() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.tilesets = [("johto".to_string(), test_tileset_definition())]
            .into_iter()
            .collect();
        data.field_moves.cut = crystal_core::systems::field_moves::FieldMoveBlockRule {
            move_id: "CUT".to_string(),
            badge: crystal_core::systems::field_moves::FieldMoveBadgeRequirement {
                region: "johto".to_string(),
                index: 1,
            },
            target_collisions: vec![0x12],
            replacements: [(
                "johto".to_string(),
                [(
                    0x03,
                    crystal_core::systems::field_moves::FieldMoveReplacement {
                        replacement_block_id: 0x03,
                        variant: "tree".to_string(),
                    },
                )]
                .into_iter()
                .collect(),
            )]
            .into_iter()
            .collect(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_field_move_replacement_block"
                && diagnostic.subject == "field_moves:cut:replacements:johto:3"
        }));

        data.field_moves.cut.replacements = [(
            "johto".to_string(),
            [(
                0x03,
                crystal_core::systems::field_moves::FieldMoveReplacement {
                    replacement_block_id: 0x63,
                    variant: "tree".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect();
        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_move_replacement_target_block"
                && diagnostic.subject == "field_moves:cut:replacements:johto:3"
        }));
    }

    #[test]
    fn escape_rope_session_use_is_atomic_when_destination_rejects() {
        let mut escape_rope = test_item("ESCAPE_ROPE");
        escape_rope.field_usable = true;
        escape_rope.consumable = true;
        escape_rope.escape_rope_mode = Some("ESCAPE_ROPE".to_string());

        let mut source = test_map_module("SourceCave", "SOURCE_CAVE", None);
        source.attributes.environment = Some("cave".to_string());
        let mut destination = test_map_module("EscapeDest", "ESCAPE_DEST", None);
        destination.events.warps = vec![WarpEvent {
            index: 1,
            x: 5,
            y: 5,
            target_map_constant: "SOURCE_CAVE".to_string(),
            target_map: "SourceCave".to_string(),
            target_warp_id: 1,
        }];

        let mut source_metadata = test_runtime_map_metadata("SOURCE_CAVE", "SourceCave");
        source_metadata.environment = "CAVE".to_string();
        let mut destination_metadata = test_runtime_map_metadata("ESCAPE_DEST", "EscapeDest");
        destination_metadata.environment = "ROUTE".to_string();
        let data = GameDataSet {
            maps: map_payload(vec![source, destination]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            runtime_map_metadata: BTreeMap::from([
                ("SOURCE_CAVE".to_string(), source_metadata),
                ("ESCAPE_DEST".to_string(), destination_metadata),
            ]),
            items: item_payload(vec![escape_rope]),
            field_moves: test_field_move_catalog(),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            dig_warp_map_name: Some("EscapeDest".to_string()),
            dig_warp_index: Some(1),
            ..GameState::default()
        };
        state
            .bag
            .add_item(&data.items["ESCAPE_ROPE"], 1)
            .expect("add escape rope");
        let mut overworld = data
            .overworld_session_for_traversal(
                "SourceCave",
                TilePosition { x: 0, y: 0 },
                17,
                PlayerTraversalState::Walk,
            )
            .expect("source session");
        let music_ids = BTreeSet::new();

        let error = data
            .use_bag_escape_rope_in_session(&mut state, &mut overworld, "ESCAPE_ROPE", &music_ids)
            .expect_err("out-of-bounds destination must reject after staged item use");

        assert!(
            format!("{error:#}")
                .contains("runtime player tile (5, 5) is outside compiled map EscapeDest"),
            "{error:#}"
        );
        assert_eq!(state.bag.quantity(&data.items["ESCAPE_ROPE"]), 1);
        assert!(state.script_runtime.item_use_events.is_empty());
        assert_eq!(overworld.map.name, "SourceCave");
        assert_eq!(overworld.player.tile, TilePosition { x: 0, y: 0 });
        assert_eq!(overworld.frame, 17);
    }

    #[test]
    fn verifier_allows_target_collision_blocks_without_field_move_replacement_rows() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        let mut tileset = test_tileset_definition();
        tileset.collision.insert(
            "3".to_string(),
            vec![
                "CUT_TREE".to_string(),
                "CUT_TREE".to_string(),
                "CUT_TREE".to_string(),
                "CUT_TREE".to_string(),
            ],
        );
        data.tilesets = [("johto".to_string(), tileset)].into_iter().collect();
        let mut module = test_map_module("IlexForest", "ILEX_FOREST", None);
        module.blocks = vec![3];
        data.maps = [("IlexForest".to_string(), module)].into_iter().collect();
        data.field_moves.cut = crystal_core::systems::field_moves::FieldMoveBlockRule {
            move_id: "CUT".to_string(),
            badge: crystal_core::systems::field_moves::FieldMoveBadgeRequirement {
                region: "johto".to_string(),
                index: 1,
            },
            target_collisions: vec![0x12],
            replacements: [(
                "johto".to_string(),
                [(
                    4,
                    crystal_core::systems::field_moves::FieldMoveReplacement {
                        replacement_block_id: 1,
                        variant: "tree".to_string(),
                    },
                )]
                .into_iter()
                .collect(),
            )]
            .into_iter()
            .collect(),
        };

        let report = verify_complete_test_game_data(&data, &PlayabilityRules::default());

        assert!(
            !report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing_field_move_runtime_replacement")
        );
    }

    #[test]
    fn verifier_requires_escape_rope_rule_match_exact_item_payload() {
        let mut data = GameDataSet::default();
        let mut item = test_item("ESCAPE_ROPE");
        item.effect = "ESCAPE_ROPE".to_string();
        item.escape_rope_mode = Some("DIG_WARP".to_string());
        data.items.insert("ESCAPE_ROPE".to_string(), item);
        data.field_moves.escape_rope = crystal_core::systems::field_moves::FieldEscapeItemRule {
            item_id: "MOD_ESCAPE_ROPE".to_string(),
            escape_rope_mode: "MOD_WARP".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_escape_item_rule"
                && diagnostic.subject == "field_moves:escape_rope"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_escape_rope_rule_without_unknown_fallback() {
        let mut data = GameDataSet::default();
        let mut item = test_item("ESCAPE_ROPE");
        item.effect = "ESCAPE_ROPE".to_string();
        item.escape_rope_mode = Some("DIG_WARP".to_string());
        data.items.insert("ESCAPE_ROPE".to_string(), item);
        data.field_moves.escape_rope = crystal_core::systems::field_moves::FieldEscapeItemRule {
            item_id: "ESCAPE ROPE".to_string(),
            escape_rope_mode: "DIG WARP".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_field_escape_item_id"
                && diagnostic.subject == "field_moves:escape_rope"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_field_escape_item_mode"
                && diagnostic.subject == "field_moves:escape_rope"
        }));
        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_escape_item_rule"
                && diagnostic.subject == "field_moves:escape_rope"
        }));
    }

    #[test]
    fn verifier_requires_repel_rule_match_exact_item_payload() {
        let mut data = GameDataSet::default();
        let mut item = test_item("REPEL");
        item.effect = "REPEL".to_string();
        data.items.insert("REPEL".to_string(), item);
        data.field_moves.repel = crystal_core::systems::field_moves::FieldRepelItemRule {};
        data.field_moves.bicycle = FieldItemRule {
            item_id: "REPEL".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_field_repel_item_payload"
                && diagnostic.subject == "field_moves:repel"
        }));
    }

    #[test]
    fn verifier_requires_bicycle_rule_match_exact_field_item_payload() {
        let mut data = GameDataSet::default();
        let mut item = test_item("BICYCLE");
        item.effect = "BICYCLE".to_string();
        item.field_menu = "ITEMMENU_CLOSE".to_string();
        data.items.insert("BICYCLE".to_string(), item);
        data.field_moves.bicycle = FieldItemRule {
            item_id: "MOD_BICYCLE".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_item_id"
                && diagnostic.subject == "field_moves:bicycle"
        }));
    }

    #[test]
    fn verifier_requires_field_key_item_rules_match_exact_item_payloads() {
        let mut data = GameDataSet::default();
        for (item_id, effect) in [
            ("ITEMFINDER", "ITEMFINDER"),
            ("SQUIRTBOTTLE", "SQUIRTBOTTLE"),
            ("COIN_CASE", "COIN_CASE"),
            ("BLUE_CARD", "BLUE_CARD"),
            ("TOWN_MAP", "TOWN_MAP"),
            ("POKEGEAR", "POKEGEAR"),
        ] {
            let mut item = test_item(item_id);
            item.effect = effect.to_string();
            item.field_menu = "ITEMMENU_CLOSE".to_string();
            data.items.insert(item_id.to_string(), item);
        }
        data.field_moves.itemfinder = FieldItemRule {
            item_id: "MOD_ITEMFINDER".to_string(),
        };
        data.field_moves.squirtbottle = FieldItemRule {
            item_id: "MOD_SQUIRTBOTTLE".to_string(),
        };
        data.field_moves.coin_case = FieldItemRule {
            item_id: "MOD_COIN_CASE".to_string(),
        };
        data.field_moves.blue_card = FieldItemRule {
            item_id: "MOD_BLUE_CARD".to_string(),
        };
        data.field_moves.town_map = FieldItemRule {
            item_id: "MOD_TOWN_MAP".to_string(),
        };
        data.field_moves.pokegear = FieldItemRule {
            item_id: "MOD_POKEGEAR".to_string(),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for subject in [
            "field_moves:itemfinder",
            "field_moves:squirtbottle",
            "field_moves:coin_case",
            "field_moves:blue_card",
            "field_moves:town_map",
            "field_moves:pokegear",
        ] {
            assert!(report.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "unknown_field_item_id" && diagnostic.subject == subject
            }));
        }
    }

    #[test]
    fn runtime_field_pokegear_item_uses_exact_pack_rule_without_literal_fallback() {
        let mut data = GameDataSet::default();
        data.field_moves.pokegear = FieldItemRule {
            item_id: "MOD_POKEGEAR".to_string(),
        };
        let mut bad = test_item("POKEGEAR");
        bad.effect = "POKEGEAR".to_string();
        bad.field_menu = "ITEMMENU_CLOSE".to_string();
        bad.field_usable = true;
        let mut exact = test_item("MOD_POKEGEAR");
        exact.effect = "POKEGEAR".to_string();
        exact.field_menu = "ITEMMENU_CLOSE".to_string();
        exact.field_usable = true;
        data.items.insert("POKEGEAR".to_string(), bad);
        data.items.insert("MOD_POKEGEAR".to_string(), exact);

        let item = data
            .field_pokegear_item("MOD_POKEGEAR")
            .expect("pack-defined Pokegear item accepted");
        assert_eq!(item.script_name, "MOD_POKEGEAR");

        let error = data
            .field_pokegear_item("POKEGEAR")
            .expect_err("literal POKEGEAR rejected after pack override");
        assert!(error.to_string().contains("InvalidFieldItemId"), "{error}");
    }

    #[test]
    fn runtime_field_box_item_uses_exact_pack_rule_without_literal_fallback() {
        let mut data = GameDataSet::default();
        let mut literal = test_item("NORMAL_BOX");
        literal.effect = "NORMAL_BOX".to_string();
        literal.field_menu = "ITEMMENU_CURRENT".to_string();
        literal.field_usable = true;
        literal.consumable = true;
        let mut exact = test_item("MOD_BOX");
        exact.effect = "MOD_BOX_EFFECT".to_string();
        exact.field_menu = "ITEMMENU_CURRENT".to_string();
        exact.field_usable = true;
        exact.consumable = true;
        data.items.insert("NORMAL_BOX".to_string(), literal);
        data.items.insert("MOD_BOX".to_string(), exact);
        data.field_box_items.insert(
            "MOD_BOX".to_string(),
            FieldBoxItemRule {
                item_id: "MOD_BOX".to_string(),
                effect: "MOD_BOX_EFFECT".to_string(),
                decoration_flag: "EVENT_MOD_BOX_DECORATION".to_string(),
            },
        );
        let mut state = GameState::default();
        state
            .bag
            .add_item(&data.items["MOD_BOX"], 1)
            .expect("add mod box");
        state
            .bag
            .add_item(&data.items["NORMAL_BOX"], 1)
            .expect("add literal box");

        let outcome = data
            .use_bag_box_in_field(&mut state, "MOD_BOX")
            .expect("pack-defined field box item works");
        assert_eq!(outcome.decoration_flag, "EVENT_MOD_BOX_DECORATION");
        assert!(!outcome.already_owned);
        assert_eq!(
            state.flags.event_flags.get("EVENT_MOD_BOX_DECORATION"),
            Some(&true)
        );

        let error = data
            .use_bag_box_in_field(&mut state, "NORMAL_BOX")
            .expect_err("literal NORMAL_BOX rejected without a pack rule");
        assert!(
            format!("{error:#}").contains("not defined by the pack"),
            "{error:#}"
        );
    }

    #[test]
    fn verifier_requires_field_box_items_to_match_exact_pack_rules() {
        let mut good_item = test_item("MOD_BOX");
        good_item.effect = "MOD_BOX_EFFECT".to_string();
        good_item.field_menu = "ITEMMENU_CURRENT".to_string();
        good_item.field_usable = true;
        let mut wrong_effect = test_item("WRONG_EFFECT_BOX");
        wrong_effect.effect = "OTHER_BOX_EFFECT".to_string();
        wrong_effect.field_menu = "ITEMMENU_CURRENT".to_string();
        wrong_effect.field_usable = true;
        let mut wrong_menu = test_item("WRONG_MENU_BOX");
        wrong_menu.effect = "WRONG_MENU_EFFECT".to_string();
        wrong_menu.field_menu = "ITEMMENU_CLOSE".to_string();
        wrong_menu.field_usable = true;
        let data = GameDataSet {
            items: [
                ("MOD_BOX".to_string(), good_item),
                ("WRONG_EFFECT_BOX".to_string(), wrong_effect),
                ("WRONG_MENU_BOX".to_string(), wrong_menu),
            ]
            .into_iter()
            .collect(),
            field_box_items: [
                (
                    "MOD_BOX".to_string(),
                    FieldBoxItemRule {
                        item_id: "MOD_BOX".to_string(),
                        effect: "MOD_BOX_EFFECT".to_string(),
                        decoration_flag: "EVENT_MOD_BOX_DECORATION".to_string(),
                    },
                ),
                (
                    "WRONG_EFFECT_BOX".to_string(),
                    FieldBoxItemRule {
                        item_id: "WRONG_EFFECT_BOX".to_string(),
                        effect: "WRONG_EFFECT_RULE".to_string(),
                        decoration_flag: "EVENT_WRONG_EFFECT_BOX".to_string(),
                    },
                ),
                (
                    "WRONG_MENU_BOX".to_string(),
                    FieldBoxItemRule {
                        item_id: "WRONG_MENU_BOX".to_string(),
                        effect: "WRONG_MENU_EFFECT".to_string(),
                        decoration_flag: "EVENT_WRONG_MENU_BOX".to_string(),
                    },
                ),
                (
                    "missing box".to_string(),
                    FieldBoxItemRule {
                        item_id: "missing box".to_string(),
                        effect: "MISSING_BOX_EFFECT".to_string(),
                        decoration_flag: "EVENT_MISSING_BOX".to_string(),
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

        assert!(!report.diagnostics.iter().any(|diagnostic| {
            diagnostic.subject == "field_box_items:MOD_BOX"
                && diagnostic.severity == VerificationSeverity::Error
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "mismatched_field_box_item_effect"
                && diagnostic.subject == "field_box_items:WRONG_EFFECT_BOX"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_field_box_item_menu"
                && diagnostic.subject == "field_box_items:WRONG_MENU_BOX"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_field_box_item_rule_id"
                && diagnostic.subject == "field_box_items:missing box"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_field_box_item"
                && diagnostic.subject == "field_box_items:missing box"
        }));
    }

    #[test]
    fn verifier_requires_title_screen_spawn_declared_by_pack() {
        let mut data = GameDataSet {
            runtime_title_screen: RuntimeTitleScreen {
                new_game_spawn_identifier: None,
                title_music: Some("MUSIC_TITLE".to_string()),
            },
            ..GameDataSet::default()
        };
        data.audio.push(ModpackAudioAsset {
            id: "MUSIC_TITLE".to_string(),
            path: "content-packs/test/music/MUSIC_TITLE.mid".to_string(),
            kind: ModpackAudioKind::Music,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        });

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_runtime_title_spawn_identifier"
                && diagnostic.subject == "runtime_title_screen"
        }));
    }

    #[test]
    fn verifier_requires_title_screen_music_declared_by_pack() {
        let data = GameDataSet {
            runtime_title_screen: RuntimeTitleScreen {
                new_game_spawn_identifier: Some(0),
                title_music: None,
            },
            runtime_spawn_points: BTreeMap::from([(
                "0".to_string(),
                RuntimeSpawnPoint {
                    identifier: 0,
                    map_constant: "NEW_BARK_TOWN".to_string(),
                    map_name: "NewBarkTown".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 4,
                    tile_y: 6,
                    group_name: "GROUP_NEW_BARK".to_string(),
                    metatile_x: 2,
                    metatile_y: 3,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]),
            ..GameDataSet::default()
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_runtime_title_music_id"
                && diagnostic.subject == "runtime_title_screen"
        }));
    }

    #[test]
    fn verifier_rejects_title_screen_spawn_missing_from_spawn_table() {
        let mut data = GameDataSet {
            runtime_title_screen: RuntimeTitleScreen {
                new_game_spawn_identifier: Some(0),
                title_music: Some("MUSIC_TITLE".to_string()),
            },
            ..GameDataSet::default()
        };
        data.audio.push(ModpackAudioAsset {
            id: "MUSIC_TITLE".to_string(),
            path: "content-packs/test/music/MUSIC_TITLE.mid".to_string(),
            kind: ModpackAudioKind::Music,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        });

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_runtime_title_spawn_identifier"
                && diagnostic.subject == "runtime_title_screen"
        }));
    }

    #[test]
    fn verifier_requires_encounter_music_modifiers_declared_by_pack() {
        let mut data = GameDataSet::default();
        add_wild_encounter_marker(&mut data);

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "missing_encounter_music_modifiers"
                && diagnostic.subject == "encounter_music_modifiers"
        }));
    }

    #[test]
    fn verifier_requires_encounter_music_modifiers_reference_exact_music_assets() {
        let mut data = GameDataSet::default();
        add_wild_encounter_marker(&mut data);
        data.audio.push(ModpackAudioAsset {
            id: "MUSIC_POKEMON_MARCH".to_string(),
            path: "content-packs/test/music/MUSIC_POKEMON_MARCH.mid".to_string(),
            kind: ModpackAudioKind::Music,
            source: ModpackAudioSource::Midi,
            pcm_format: None,
            pcm_frame_count: None,
            payload_hash: None,
            loop_start_sample: None,
            loop_end_sample: None,
        });
        data.encounter_music_modifiers = EncounterMusicModifiers {
            modifiers: BTreeMap::from([
                (
                    "MUSIC_POKEMON_MARCH".to_string(),
                    EncounterMusicModifier {
                        numerator: 2,
                        denominator: 1,
                    },
                ),
                (
                    "MUSIC POKEMON MARCH".to_string(),
                    EncounterMusicModifier {
                        numerator: 1,
                        denominator: 1,
                    },
                ),
                (
                    "SFX_TACKLE".to_string(),
                    EncounterMusicModifier {
                        numerator: 1,
                        denominator: 0,
                    },
                ),
            ]),
        };

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unknown_encounter_music_modifier_id"
                && diagnostic.subject == "encounter_music_modifiers:SFX_TACKLE"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_encounter_music_modifier_id"
                && diagnostic.subject == "encounter_music_modifiers:MUSIC POKEMON MARCH"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "invalid_encounter_music_modifier_ratio"
                && diagnostic.subject == "encounter_music_modifiers:SFX_TACKLE"
        }));
    }

    #[test]
    fn verifier_rejects_invalid_battle_escape_rules_from_pack() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.battle_escape_rules.player_speed_multiplier = 0;
        data.battle_escape_rules.enemy_speed_divisor = 0;
        data.battle_escape_rules.rng_roll_values = u16::from(u8::MAX) + 2;

        let report = verify_game_data(
            &AssetRoot::new(repository_root_for_tests()),
            &data,
            &PlayabilityRules::default(),
        );

        for subject in [
            "battle_escape_rules:player_speed_multiplier",
            "battle_escape_rules:enemy_speed_divisor",
            "battle_escape_rules:rng_roll_values",
        ] {
            assert!(
                report.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "invalid_battle_escape_rule" && diagnostic.subject == subject
                }),
                "missing invalid battle escape diagnostic for {subject}"
            );
        }
    }

    fn test_map_module(id: &str, map_constant: &str, connection_target: Option<&str>) -> MapModule {
        MapModule {
            id: id.to_string(),
            attributes: MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 0,
                width: 1,
                height: 1,
                connections: connection_target
                    .map(|target| {
                        vec![MapConnection {
                            direction: "east".to_string(),
                            target_map: target.to_string(),
                            offset: 0,
                        }]
                    })
                    .unwrap_or_default(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: Some("route".to_string()),
                location: Some("johto".to_string()),
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: Some(map_constant.to_string()),
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
            blocks: vec![1],
        }
    }

    fn test_tileset_definition() -> TilesetDefinition {
        TilesetDefinition {
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
            palette_map: vec![0],
        }
    }

    #[test]
    fn field_encounter_battle_rejects_target_outside_runtime_map_bounds() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let field_encounter = crystal_core::world::encounters::FieldEncounterRoll {
            map_name: "Route29".to_string(),
            kind: crystal_core::world::encounters::FieldEncounterKind::Headbutt,
            target_tile_x: 2,
            target_tile_y: 0,
            chance_roll: 0,
            entry_roll: Some(0),
            score: Some(0),
            resolved: Some(ResolvedWildEncounter {
                level: 5,
                encounter: WildEncounter {
                    level: 5,
                    species: "PIDGEY".to_string(),
                },
                slot: 0,
            }),
        };

        let error = data
            .start_field_encounter_battle(&mut state, &field_encounter)
            .expect_err("field encounter battle target must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "field encounter target tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn field_encounter_battle_rejects_target_outside_explicit_runtime_bounds() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let field_encounter = crystal_core::world::encounters::FieldEncounterRoll {
            map_name: "Route29".to_string(),
            kind: crystal_core::world::encounters::FieldEncounterKind::Headbutt,
            target_tile_x: 4,
            target_tile_y: 0,
            chance_roll: 0,
            entry_roll: Some(0),
            score: Some(0),
            resolved: Some(ResolvedWildEncounter {
                level: 5,
                encounter: WildEncounter {
                    level: 5,
                    species: "PIDGEY".to_string(),
                },
                slot: 0,
            }),
        };

        let error = data
            .start_field_encounter_battle(&mut state, &field_encounter)
            .expect_err("field encounter battle target must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "field encounter target tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn itemfinder_rejects_player_tile_outside_runtime_map_bounds() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };

        let error = data
            .find_itemfinder_hidden_item(&GameState::default(), "Route29", TilePosition::new(4, 0))
            .expect_err("Itemfinder must reject runtime player tiles outside map bounds");

        assert!(
            format!("{error:#}").contains(
                "itemfinder player tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn headbutt_roll_rejects_target_outside_runtime_map_bounds_before_rng() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(0x1234_5678);

        let error = data
            .roll_headbutt_encounter("Route29", TilePosition::new(2, 0), 0, &mut rng)
            .expect_err("HEADBUTT target must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "HEADBUTT encounter tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
        assert_eq!(rng.seed(), 0x1234_5678);
    }

    #[test]
    fn headbutt_roll_rejects_target_outside_explicit_runtime_map_bounds_before_rng() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(0x1234_5678);

        let error = data
            .roll_headbutt_encounter("Route29", TilePosition::new(4, 0), 0, &mut rng)
            .expect_err("HEADBUTT target must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "HEADBUTT encounter tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
        assert_eq!(rng.seed(), 0x1234_5678);
    }

    #[test]
    fn rock_smash_roll_rejects_target_outside_runtime_map_bounds_before_rng() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(0x1234_5678);

        let error = data
            .roll_rock_smash_encounter("Route29", TilePosition::new(2, 0), &mut rng)
            .expect_err("ROCK_SMASH target must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "ROCK_SMASH encounter tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
        assert_eq!(rng.seed(), 0x1234_5678);
    }

    #[test]
    fn rock_smash_roll_rejects_target_outside_explicit_runtime_map_bounds_before_rng() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(0x1234_5678);

        let error = data
            .roll_rock_smash_encounter("Route29", TilePosition::new(4, 0), &mut rng)
            .expect_err("ROCK_SMASH target must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "ROCK_SMASH encounter tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
        assert_eq!(rng.seed(), 0x1234_5678);
    }

    fn resolved_test_wild_encounter_roll(map_name: &str, tile: TilePosition) -> WildEncounterRoll {
        WildEncounterRoll {
            map_name: map_name.to_string(),
            tile,
            surface: EncounterSurface::Grass,
            time: TimeOfDay::Day,
            threshold: 255,
            encounter_roll: 0,
            slot_percent_roll: Some(0),
            level_roll: None,
            resolved: Some(ResolvedWildEncounter {
                level: 5,
                encounter: WildEncounter {
                    level: 5,
                    species: "MISSINGNO".to_string(),
                },
                slot: 0,
            }),
            repelled_by: None,
            rng_seed_after: 0,
        }
    }

    #[test]
    fn wild_battle_start_rejects_origin_outside_runtime_map_bounds_before_species_lookup() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(1);

        let error = data
            .wild_battle_start(
                resolved_test_wild_encounter_roll("Route29", TilePosition::new(2, 0)),
                &mut rng,
            )
            .expect_err("wild battle origin must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "wild battle encounter roll tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn wild_battle_start_rejects_origin_outside_explicit_runtime_bounds_before_species_lookup() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(1);

        let error = data
            .wild_battle_start(
                resolved_test_wild_encounter_roll("Route29", TilePosition::new(4, 0)),
                &mut rng,
            )
            .expect_err("wild battle origin must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "wild battle encounter roll tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn start_wild_battle_rejects_invalid_origin_before_rng_or_battle_mutation() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            rng_seed: 0x1234_5678,
            ..GameState::default()
        };

        let error = data
            .start_wild_battle(
                &mut state,
                resolved_test_wild_encounter_roll("Route29", TilePosition::new(2, 0)),
            )
            .expect_err("invalid wild battle origin must fail before mutation");

        assert!(
            format!("{error:#}").contains(
                "wild battle encounter roll tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
        assert_eq!(state.rng_seed, 0x1234_5678);
        assert_eq!(state.battle, BattleMemory::Inactive);
    }

    #[test]
    fn fishing_battle_rejects_origin_outside_runtime_map_bounds() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();

        let error = data
            .start_fishing_battle(
                &mut state,
                "Route29",
                TilePosition::new(2, 0),
                WildEncounter {
                    level: 5,
                    species: "MAGIKARP".to_string(),
                },
                TimeOfDay::Day,
                0,
                0,
            )
            .expect_err("fishing battle origin must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "fishing battle tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn fishing_battle_rejects_origin_outside_explicit_runtime_map_bounds() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();

        let error = data
            .start_fishing_battle(
                &mut state,
                "Route29",
                TilePosition::new(4, 0),
                WildEncounter {
                    level: 5,
                    species: "MAGIKARP".to_string(),
                },
                TimeOfDay::Day,
                0,
                0,
            )
            .expect_err("fishing battle origin must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "fishing battle tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
    }

    #[test]
    fn sweet_scent_roll_rejects_origin_outside_runtime_map_bounds_before_rng() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(0x1234_5678);

        let error = data
            .roll_sweet_scent_encounter(
                "Route29",
                EncounterSurface::Grass,
                TimeOfDay::Day,
                TilePosition::new(2, 0),
                &mut rng,
            )
            .expect_err("SWEET_SCENT origin must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "SWEET_SCENT encounter tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
        assert_eq!(rng.seed(), 0x1234_5678);
    }

    #[test]
    fn sweet_scent_roll_rejects_origin_outside_explicit_runtime_map_bounds_before_rng() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.attributes.width = 2;
        module.attributes.height = 2;
        module.blocks = vec![1, 1, 1, 1];
        let data = GameDataSet {
            maps: map_payload(vec![module]),
            ..GameDataSet::default()
        };
        let mut rng = Random::new(0x1234_5678);

        let error = data
            .roll_sweet_scent_encounter(
                "Route29",
                EncounterSurface::Grass,
                TimeOfDay::Day,
                TilePosition::new(4, 0),
                &mut rng,
            )
            .expect_err("SWEET_SCENT origin must fit compiled runtime map bounds");

        assert!(
            format!("{error:#}").contains(
                "SWEET_SCENT encounter tile (4, 0) is outside compiled map Route29 runtime tile bounds 4x4"
            ),
            "{error:#}"
        );
        assert_eq!(rng.seed(), 0x1234_5678);
    }

    #[test]
    fn wild_encounter_after_step_rejects_out_of_bounds_session_tile_before_rng_commit() {
        let data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            rng_seed: 0x1234_5678,
            ..GameState::default()
        };
        let mut session = data
            .overworld_session("Route29", TilePosition::new(0, 0), 0)
            .expect("valid session");
        session.player.tile = TilePosition::new(4, 0);

        let error = data
            .check_wild_encounter_after_step(&mut state, &session)
            .expect_err("wild encounter check must reject session tiles outside map bounds");

        assert!(
            format!("{error:#}").contains(
                "wild encounter check tile (4, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
        assert_eq!(state.rng_seed, 0x1234_5678);
    }

    #[test]
    fn runtime_field_encounter_commands_reject_unused_payload_fields() {
        let headbutt_error =
            reject_field_encounter_surface("HEADBUTT", Some(EncounterSurface::Grass))
                .expect_err("HEADBUTT command must not carry a surface payload");
        assert!(
            format!("{headbutt_error:#}")
                .contains("HEADBUTT field move command must not declare surface"),
            "{headbutt_error:#}"
        );

        let sweet_scent_error = reject_field_encounter_player_id("SWEET_SCENT", Some(12345))
            .expect_err("SWEET_SCENT command must not carry player_id");
        assert!(
            format!("{sweet_scent_error:#}")
                .contains("SWEET_SCENT field move command must not declare player_id"),
            "{sweet_scent_error:#}"
        );

        let missing_rng_seed =
            serde_json::from_value::<RuntimeFieldEncounterCommand>(serde_json::json!({
                "party_index": 0,
                "player_id": 12345,
                "surface": null
            }))
            .expect_err("field encounter commands must declare the post-mutation RNG seed");
        assert!(
            missing_rng_seed
                .to_string()
                .contains("missing field `rng_seed_after`"),
            "{missing_rng_seed}"
        );
    }

    #[test]
    fn runtime_day_care_commands_use_exact_action_payloads() {
        let deposit = RuntimeDayCareCommand {
            caretaker: RuntimeDayCareCaretaker::Man,
            action: RuntimeDayCareAction::Deposit,
            party_index: Some(0),
        };
        assert_eq!(
            runtime_day_care_party_slot(&deposit).expect("deposit slot"),
            Some(0)
        );
        assert_eq!(runtime_day_care_action_name(deposit.action), "deposit");

        let missing_slot = RuntimeDayCareCommand {
            caretaker: RuntimeDayCareCaretaker::Man,
            action: RuntimeDayCareAction::Deposit,
            party_index: None,
        };
        let missing_error = runtime_day_care_party_slot(&missing_slot)
            .expect_err("deposit must carry the party slot consumed by DayCareMan");
        assert!(
            format!("{missing_error:#}").contains("Day Care deposit command requires party_index"),
            "{missing_error:#}"
        );

        for action in [
            RuntimeDayCareAction::Withdraw,
            RuntimeDayCareAction::Inspect,
        ] {
            let command = RuntimeDayCareCommand {
                caretaker: RuntimeDayCareCaretaker::Lady,
                action,
                party_index: Some(0),
            };
            let error = runtime_day_care_party_slot(&command)
                .expect_err("non-deposit Day Care actions must not carry an ignored party slot");
            assert!(
                format!("{error:#}").contains(&format!(
                    "Day Care {} command must not declare party_index",
                    runtime_day_care_action_name(action)
                )),
                "{error:#}"
            );
        }
    }

    #[test]
    fn runtime_bug_contest_commands_compute_rank_and_use_exact_rng_payloads() {
        let judging = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::Judge,
            rank: None,
            rng_seed_after: Some(42),
        };
        assert_eq!(
            runtime_bug_contest_rank(&judging).expect("computed judge rank"),
            None
        );
        assert_eq!(runtime_bug_contest_rng_seed_after(&judging).expect("judge RNG seed"), Some(42));
        assert_eq!(runtime_bug_contest_action_name(judging.action), "judge");

        let injected_rank = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::Judge,
            rank: Some(3),
            rng_seed_after: Some(42),
        };
        let injected_error = runtime_bug_contest_rank(&injected_rank)
            .expect_err("judging must not accept a fabricated rank");
        assert!(
            format!("{injected_error:#}").contains("Bug Contest judge command must not declare rank"),
            "{injected_error:#}"
        );

        for action in [
            RuntimeBugContestAction::GiveParkBalls,
            RuntimeBugContestAction::SelectContestants,
            RuntimeBugContestAction::DropOffMons,
            RuntimeBugContestAction::ReturnMons,
            RuntimeBugContestAction::CheckPartyFull,
        ] {
            let command = RuntimeBugContestCommand {
                action,
                rank: Some(1),
                rng_seed_after: None,
            };
            let error = runtime_bug_contest_rank(&command)
                .expect_err("non-judging bug contest actions must not carry ignored rank");
            assert!(
                format!("{error:#}").contains(&format!(
                    "Bug Contest {} command must not declare rank",
                    runtime_bug_contest_action_name(action)
                )),
                "{error:#}"
            );
        }

        let selecting = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::SelectContestants,
            rank: None,
            rng_seed_after: Some(42),
        };
        assert_eq!(
            runtime_bug_contest_rng_seed_after(&selecting).expect("select RNG seed"),
            Some(42)
        );

        let missing_rng_seed = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::SelectContestants,
            rank: None,
            rng_seed_after: None,
        };
        let missing_rng_error = runtime_bug_contest_rng_seed_after(&missing_rng_seed)
            .expect_err("select contestants must carry the post-routine RNG seed");
        assert!(
            format!("{missing_rng_error:#}")
                .contains("Bug Contest select_contestants command requires rng_seed_after"),
            "{missing_rng_error:#}"
        );

        let unused_rng_seed = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::GiveParkBalls,
            rank: None,
            rng_seed_after: Some(42),
        };
        let unused_rng_error = runtime_bug_contest_rng_seed_after(&unused_rng_seed)
            .expect_err("non-RNG bug contest actions must reject rng_seed_after");
        assert!(
            format!("{unused_rng_error:#}")
                .contains("Bug Contest give_park_balls command must not declare rng_seed_after"),
            "{unused_rng_error:#}"
        );
    }

    #[test]
    fn bug_contest_select_contestants_rejects_stale_rng_seed_before_mutation() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["SelectRandomBugContestContestants"]),
            bug_contest_config: Some(BugContestConfig {
                park_balls: 20,
                timer_minutes: 20,
                timer_seconds: 0,
                selected_contestant_count: 1,
                contestant_flags: vec![
                    "EVENT_BUG_CATCHING_CONTESTANT_1A".to_string(),
                    "EVENT_BUG_CATCHING_CONTESTANT_2A".to_string(),
                ],
            }),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let before = state.clone();
        let audio_ids = BTreeSet::new();
        let mut preview = state.clone();
        data.apply_special_routine(
            &mut preview,
            "SelectRandomBugContestContestants",
            &audio_ids,
        )
        .expect("preview Bug Contest contestant selection");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeBugContestMap".to_string(),
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

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand {
                    action: RuntimeBugContestAction::SelectContestants,
                    rank: None,
                    rng_seed_after: Some(0),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale Bug Contest contestant RNG seed must reject");

        assert!(
            error
                .to_string()
                .contains("use Bug Contest select_contestants rng_seed_after 0 does not match"),
            "{error}"
        );
        assert_eq!(state, before);
    }

    #[test]
    fn runtime_shuckie_commands_use_exact_party_payloads() {
        let give = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Give,
            party_index: None,
            rng_seed_after: Some(42),
        };
        assert_eq!(runtime_shuckie_party_slot(&give).expect("give"), None);
        assert_eq!(
            runtime_shuckie_rng_seed_after(&give).expect("give RNG seed"),
            Some(42)
        );

        let give_with_slot = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Give,
            party_index: Some(0),
            rng_seed_after: Some(42),
        };
        let give_error = runtime_shuckie_party_slot(&give_with_slot)
            .expect_err("GiveShuckle must not receive ignored party slot state");
        assert!(
            format!("{give_error:#}").contains("Shuckie give command must not declare party_index"),
            "{give_error:#}"
        );
        let missing_rng_seed = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Give,
            party_index: None,
            rng_seed_after: None,
        };
        let missing_rng_error = runtime_shuckie_rng_seed_after(&missing_rng_seed)
            .expect_err("GiveShuckle must declare RNG boundary");
        assert!(
            format!("{missing_rng_error:#}")
                .contains("Shuckie give command requires rng_seed_after"),
            "{missing_rng_error:#}"
        );

        let return_selected = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Return,
            party_index: Some(2),
            rng_seed_after: None,
        };
        assert_eq!(
            runtime_shuckie_party_slot(&return_selected).expect("return selected"),
            Some(2)
        );
        assert_eq!(
            runtime_shuckie_rng_seed_after(&return_selected).expect("return selected RNG"),
            None
        );

        let return_cancelled = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Return,
            party_index: None,
            rng_seed_after: None,
        };
        assert_eq!(
            runtime_shuckie_party_slot(&return_cancelled).expect("return cancelled"),
            None
        );
        let return_with_rng = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Return,
            party_index: Some(2),
            rng_seed_after: Some(42),
        };
        let unused_rng_error = runtime_shuckie_rng_seed_after(&return_with_rng)
            .expect_err("ReturnShuckie must not accept unused RNG state");
        assert!(
            format!("{unused_rng_error:#}")
                .contains("Shuckie return command must not declare rng_seed_after"),
            "{unused_rng_error:#}"
        );
    }

    #[test]
    fn generic_special_routine_command_enforces_rng_boundary() {
        assert!(runtime_special_routine_requires_rng_boundary(
            "SampleKenjiBreakCountdown"
        ));
        assert!(!runtime_special_routine_requires_rng_boundary("HealParty"));

        let data = GameDataSet {
            special_routines: special_routine_rules(["SampleKenjiBreakCountdown", "HealParty"]),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let before = state.clone();
        let audio_ids = BTreeSet::new();
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeGenericSpecialMap".to_string(),
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

        let missing = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplySpecialRoutine {
                    routine: "SampleKenjiBreakCountdown".to_string(),
                    rng_seed_after: None,
                },
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("generic RNG special must declare rng_seed_after");
        assert!(
            missing.to_string().contains(
                "special routine SampleKenjiBreakCountdown command requires rng_seed_after"
            ),
            "{missing}"
        );
        assert_eq!(state, before);

        let stale = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplySpecialRoutine {
                    routine: "SampleKenjiBreakCountdown".to_string(),
                    rng_seed_after: Some(0),
                },
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("generic RNG special must reject stale rng_seed_after");
        assert!(
            stale.to_string().contains(
                "apply special routine SampleKenjiBreakCountdown rng_seed_after 0 does not match"
            ),
            "{stale}"
        );
        assert_eq!(state, before);

        let mut preview = state.clone();
        data.apply_special_routine(&mut preview, "SampleKenjiBreakCountdown", &audio_ids)
            .expect("preview Kenji countdown");
        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplySpecialRoutine {
                    routine: "SampleKenjiBreakCountdown".to_string(),
                    rng_seed_after: Some(preview.rng_seed),
                },
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("generic RNG special applies with exact rng_seed_after");
        let RuntimeMutationResult::SpecialRoutineApplied(special) = outcome.result else {
            panic!("expected generic special result");
        };
        let SpecialRoutineEffect::SampleKenjiBreakCountdown {
            value,
            rng_seed_after,
        } = special.effect
        else {
            panic!("expected Kenji countdown effect");
        };
        assert_eq!(rng_seed_after, preview.rng_seed);
        assert_eq!(state.rng_seed, preview.rng_seed);
        assert_eq!(state.kenji_break_timer, value);

        let current_rng_seed = state.rng_seed;
        let unused = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplySpecialRoutine {
                    routine: "HealParty".to_string(),
                    rng_seed_after: Some(current_rng_seed),
                },
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("non-RNG generic special must reject unused rng_seed_after");
        assert!(
            unused
                .to_string()
                .contains("special routine HealParty command must not declare rng_seed_after"),
            "{unused}"
        );
    }

    #[test]
    fn shuckie_give_rejects_stale_rng_seed_before_mutation() {
        let mut data = GameDataSet {
            special_routines: special_routine_rules(["GiveShuckle"]),
            growth_rates: [(
                "GROWTH_MEDIUM_FAST".to_string(),
                crystal_core::systems::experience::GrowthRateCurve {
                    id: "GROWTH_MEDIUM_FAST".to_string(),
                    numerator: 1,
                    denominator: 1,
                    quadratic: 0,
                    linear: 0,
                    constant: 0,
                },
            )]
            .into_iter()
            .collect(),
            items: [("BERRY".to_string(), test_item("BERRY"))]
                .into_iter()
                .collect(),
            shuckie_gift: Some(ShuckieGiftDefinition {
                species: "NEW_MON".to_string(),
                level: 15,
                held_item: "BERRY".to_string(),
                nickname: "SHUCKIE".to_string(),
                original_trainer_name: "MANIA".to_string(),
                original_trainer_id: 518,
                got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
            }),
            ..GameDataSet::default()
        };
        add_runtime_species_and_move(&mut data);
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let before = state.clone();
        let audio_ids = BTreeSet::new();
        let mut preview = state.clone();
        data.apply_special_routine(&mut preview, "GiveShuckle", &audio_ids)
            .expect("preview Shuckie gift");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeShuckieMap".to_string(),
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

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand {
                    action: RuntimeShuckieAction::Give,
                    party_index: None,
                    rng_seed_after: Some(0),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale Shuckie gift rng seed must reject");
        assert!(
            error
                .to_string()
                .contains("use Shuckie give rng_seed_after 0 does not match"),
            "{error}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand {
                    action: RuntimeShuckieAction::Give,
                    party_index: None,
                    rng_seed_after: Some(preview.rng_seed),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("Shuckie gift command applies with exact RNG seed");
        let RuntimeMutationResult::ShuckieUsed(special) = outcome.result else {
            panic!("expected Shuckie result");
        };
        assert_eq!(
            special.effect,
            SpecialRoutineEffect::GiveShuckle {
                stored: true,
                rng_seed_after: preview.rng_seed,
            }
        );
        assert_eq!(state.storage.party.filled_slots(), 1);
        assert_eq!(state.rng_seed, preview.rng_seed);
    }

    #[test]
    fn runtime_odd_egg_command_requires_exact_rng_boundary() {
        let missing_rng_seed =
            serde_json::from_value::<RuntimeOddEggCommand>(serde_json::json!({}))
                .expect_err("Odd Egg command must declare the post-mutation RNG seed");
        assert!(
            missing_rng_seed
                .to_string()
                .contains("missing field `rng_seed_after`"),
            "{missing_rng_seed}"
        );

        let command = serde_json::from_value::<RuntimeOddEggCommand>(serde_json::json!({
            "rng_seed_after": 12345
        }))
        .expect("Odd Egg command accepts exact RNG boundary");
        assert_eq!(command.rng_seed_after, 12345);
    }

    #[test]
    fn odd_egg_command_rejects_stale_rng_seed_before_mutation() {
        let mut data = GameDataSet::default();
        add_runtime_species_and_move(&mut data);
        data.special_routines = special_routine_rules(["GiveOddEgg"]);
        data.odd_egg_definitions = vec![OddEggDefinition {
            species: "NEW_MON".to_string(),
            moves: vec!["TACKLE".to_string()],
            original_trainer_id: 2048,
            dvs: [0, 0, 0, 0],
            probability: 100,
            level: 5,
            experience: 125,
            hatch_cycles: 20,
            nickname: "EGG".to_string(),
            original_trainer_name: "ODD".to_string(),
        }];
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let before = state.clone();
        let mut preview = state.clone();
        let audio_ids = BTreeSet::new();
        data.apply_special_routine(&mut preview, "GiveOddEgg", &audio_ids)
            .expect("preview Odd Egg");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeOddEggMap".to_string(),
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

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::GiveOddEgg(RuntimeOddEggCommand { rng_seed_after: 0 }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale Odd Egg rng seed must reject");

        assert!(
            error
                .to_string()
                .contains("give Odd Egg rng_seed_after 0 does not match"),
            "{error}"
        );
        assert_eq!(state, before);
    }

    #[test]
    fn runtime_buena_password_command_requires_exact_rng_boundary() {
        let missing_rng_seed =
            serde_json::from_value::<RuntimeBuenaPasswordCommand>(serde_json::json!({
                "guess": "TODAY"
            }))
            .expect_err("Buena password command must declare the post-mutation RNG seed");
        assert!(
            missing_rng_seed
                .to_string()
                .contains("missing field `rng_seed_after`"),
            "{missing_rng_seed}"
        );
    }

    #[test]
    fn runtime_phone_random_special_command_requires_exact_rng_boundary() {
        let missing_rng_seed =
            serde_json::from_value::<RuntimePhoneCallerCommand>(serde_json::json!({
                "special": "random_phone_wild_mon",
                "contact_id": "PHONE_BIRDKEEPER_VANCE"
            }))
            .expect_err("phone random special command must declare the post-mutation RNG seed");
        assert!(
            missing_rng_seed
                .to_string()
                .contains("missing field `rng_seed_after`"),
            "{missing_rng_seed}"
        );
    }

    #[test]
    fn phone_random_special_command_rejects_stale_rng_seed_before_mutation() {
        let mut rattata = species();
        rattata.id = "RATTATA".to_string();
        rattata.int_id = 19;
        let data = GameDataSet {
            special_routines: special_routine_rules(["RandomPhoneWildMon"]),
            pokemon: BTreeMap::from([("RATTATA".to_string(), rattata)]),
            phone_contacts: PhoneContactCatalog(BTreeMap::from([(
                "PHONE_BIRDKEEPER_VANCE".to_string(),
                PhoneContactRecord {
                    contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                    trainer_class: None,
                    trainer_label: None,
                    lines: vec!["Vance:".to_string()],
                    primary_label: "Vance".to_string(),
                    map_constant: Some("ROUTE_44".to_string()),
                    callee_time_mask: 0,
                    callee_script: None,
                    caller_time_mask: 0,
                    caller_script: None,
                },
            )])),
            wild_encounters: BTreeMap::from([(
                "ROUTE_44".to_string(),
                WildEncounterData {
                    map_name: "ROUTE_44".to_string(),
                    grass_rates: Some(BTreeMap::from([
                        ("morning".to_string(), 30),
                        ("day".to_string(), 30),
                        ("night".to_string(), 30),
                    ])),
                    water_rate: None,
                    grass: Some(WildEncounterTable {
                        morning: vec![
                            WildEncounter {
                                level: 20,
                                species: "RATTATA".to_string(),
                            };
                            4
                        ],
                        day: vec![
                            WildEncounter {
                                level: 20,
                                species: "RATTATA".to_string(),
                            };
                            4
                        ],
                        night: vec![
                            WildEncounter {
                                level: 20,
                                species: "RATTATA".to_string(),
                            };
                            4
                        ],
                    }),
                    water: None,
                },
            )]),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let before = state.clone();
        let audio_ids = BTreeSet::new();
        let mut preview = state.clone();
        preview.script_runtime.variables.insert(
            "VAR_CALLERID".to_string(),
            "PHONE_BIRDKEEPER_VANCE".to_string(),
        );
        data.apply_special_routine(&mut preview, "RandomPhoneWildMon", &audio_ids)
            .expect("preview phone random special");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimePhoneMap".to_string(),
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

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand {
                    special: RuntimePhoneRandomSpecial::RandomPhoneWildMon,
                    contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                    rng_seed_after: 0,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale phone random special rng seed must reject");
        assert!(
            error.to_string().contains(
                "apply phone random special RandomPhoneWildMon rng_seed_after 0 does not match"
            ),
            "{error}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand {
                    special: RuntimePhoneRandomSpecial::RandomPhoneWildMon,
                    contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                    rng_seed_after: preview.rng_seed,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("phone random special command applies with exact RNG seed");
        let RuntimeMutationResult::PhoneRandomSpecialApplied(special) = outcome.result else {
            panic!("expected phone random special result");
        };
        assert_eq!(
            special.effect,
            SpecialRoutineEffect::RandomPhoneWildMon {
                contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                map_name: "ROUTE_44".to_string(),
                time_of_day: TimeOfDay::Night,
                species: "RATTATA".to_string(),
                rng_seed_after: preview.rng_seed,
            }
        );
    }

    #[test]
    fn buena_password_command_uses_exact_guess_and_rejects_stale_rng_seed_before_mutation() {
        let mut data = GameDataSet::default();
        data.special_routines = special_routine_rules(["BuenasPassword"]);
        data.buena_password_categories = BuenaPasswordCategories {
            order: vec!["DailyWord".to_string()],
            categories: BTreeMap::from([(
                "DailyWord".to_string(),
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_STRING".to_string(),
                    points: 10,
                    options: vec!["TODAY".to_string()],
                },
            )]),
        };
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let audio_ids = BTreeSet::new();
        let mut preview = state.clone();
        preview
            .script_runtime
            .variables
            .insert("BUENA_PASSWORD".to_string(), "TODAY".to_string());
        data.apply_special_routine(&mut preview, "BuenasPassword", &audio_ids)
            .expect("preview Buena password");
        assert_ne!(preview.rng_seed, 0);
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeBuenaMap".to_string(),
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
        let before_stale = state.clone();

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBuenaPassword(RuntimeBuenaPasswordCommand {
                    guess: Some("TODAY".to_string()),
                    rng_seed_after: 0,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("stale Buena password rng seed must reject");
        assert!(
            error
                .to_string()
                .contains("use Buena password rng_seed_after 0 does not match"),
            "{error}"
        );
        assert_eq!(state, before_stale);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBuenaPassword(RuntimeBuenaPasswordCommand {
                    guess: Some("TODAY".to_string()),
                    rng_seed_after: preview.rng_seed,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("Buena password command applies with exact RNG seed");
        let RuntimeMutationResult::BuenaPasswordUsed(special) = outcome.result else {
            panic!("expected Buena password result");
        };
        assert_eq!(
            special.effect,
            SpecialRoutineEffect::BuenasPassword {
                category: "DailyWord".to_string(),
                category_type: "BUENA_STRING".to_string(),
                correct: "TODAY".to_string(),
                guess: Some("TODAY".to_string()),
                matched: true,
                rng_seed_after: preview.rng_seed,
            }
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("BUENA_PASSWORD")
                .map(String::as_str),
            Some("TODAY")
        );
        assert!(!state.script_runtime.variables.contains_key("_buena_guess"));
    }
