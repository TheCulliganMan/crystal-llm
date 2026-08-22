    #[test]
    fn runtime_special_palette_and_snorlax_apply_pack_declared_effects() {
        let root = temp_repository_root("special-palette-snorlax-time");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        for routine in [
            "SetPlayerPalette",
            "SnorlaxAwake",
            "SetDayOfWeek",
            "InitialSetDSTFlag",
            "InitialClearDSTFlag",
            "UpdateTime",
            "SampleKenjiBreakCountdown",
            "CheckLuckyNumberShowFlag",
            "ResetLuckyNumberShowFlag",
            "PrintTodaysLuckyNumber",
            "CheckForLuckyNumberWinners",
            "PlaceMoneyTopRight",
            "DisplayMoneyAndCoinBalance",
            "DisplayCoinCaseBalance",
            "GSHealings",
            "StubbedTrainerRankings_Healings",
            "Reset",
            "HoOhChamber",
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
        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "160".to_string());

        let palette = session
            .apply_special_routine(&runtime, "SetPlayerPalette")
            .expect("set player palette");

        assert_eq!(
            palette.outcome.effect,
            SpecialRoutineEffect::SetPlayerPalette {
                raw_value: 160,
                palette_id: 2,
                changed: true
            }
        );
        assert_eq!(session.state.player_palette_id, 2);
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("2")
        );

        session.state.script_runtime.current_music = Some("MUSIC_POKE_FLUTE_CHANNEL".to_string());
        session.state.overworld = OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(72, 18),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let snorlax = session
            .apply_special_routine(&runtime, "SnorlaxAwake")
            .expect("snorlax awake");

        assert_eq!(
            snorlax.outcome.effect,
            SpecialRoutineEffect::SnorlaxAwake {
                music: Some("MUSIC_POKE_FLUTE_CHANNEL".to_string()),
                tile: Some((72, 18)),
                awake: true
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("1")
        );

        session.state.time.current_day = 5;
        session
            .state
            .script_runtime
            .variables
            .insert("wTempDayOfWeek".to_string(), "0".to_string());
        let day = session
            .apply_special_routine(&runtime, "SetDayOfWeek")
            .expect("set day of week");

        assert_eq!(
            day.outcome.effect,
            SpecialRoutineEffect::SetDayOfWeek { day: 0 }
        );
        assert_eq!(session.state.time.day_of_week, 0);

        let dst = session
            .apply_special_routine(&runtime, "InitialSetDSTFlag")
            .expect("set dst");

        assert_eq!(dst.outcome.effect, SpecialRoutineEffect::InitialSetDstFlag);
        assert!(session.state.time.dst);

        session.state.time.start_time = ClockTime::new(2, 9, 30, 15);
        session.state.time.registers.rtc_day_lo = 3;
        session.state.time.registers.rtc_hours = 8;
        session.state.time.registers.rtc_minutes = 45;
        session.state.time.registers.rtc_seconds = 50;
        let time = session
            .apply_special_routine(&runtime, "UpdateTime")
            .expect("update time");

        assert_eq!(
            time.outcome.effect,
            SpecialRoutineEffect::UpdateTime {
                hour: 18,
                minute: 16,
                second: 5,
                day_of_week: 5,
                time_of_day: TimeOfDay::Night
            }
        );

        session.state.random_state =
            crystal_core::random::CrystalRandomState { add: 0xff, sub: 0 };
        session.divider =
            crystal_core::random::RuntimeDividerSource::replay([0, 200]);
        let kenji = session
            .apply_special_routine(&runtime, "SampleKenjiBreakCountdown")
            .expect("kenji countdown");

        assert_eq!(
            kenji.outcome.effect,
            SpecialRoutineEffect::SampleKenjiBreakCountdown {
                value: 3,
                random_state_after: crystal_core::random::CrystalRandomState {
                    add: 0xff,
                    sub: 56
                }
            }
        );
        assert_eq!(session.state.kenji_break_timer, 3);
        session.state.lucky_number_show_flag = true;
        session.state.time.current_day = 4;
        session.state.time.day_of_week = 4;
        session.state.random_state = crystal_core::random::CrystalRandomState::default();
        let lucky_flag = session
            .apply_special_routine(&runtime, "CheckLuckyNumberShowFlag")
            .expect("check lucky flag");

        assert_eq!(
            lucky_flag.outcome.effect,
            SpecialRoutineEffect::CheckLuckyNumberShowFlag { flag: true }
        );

        session.divider =
            crystal_core::random::RuntimeDividerSource::replay([0, 255, 0, 255]);
        let lucky_reset = session
            .apply_special_routine(&runtime, "ResetLuckyNumberShowFlag")
            .expect("reset lucky flag");

        assert_eq!(
            lucky_reset.outcome.effect,
            SpecialRoutineEffect::ResetLuckyNumberShowFlag {
                lucky_number: 258,
                lucky_number_day: 4,
                random_state_after: crystal_core::random::CrystalRandomState { add: 2, sub: 2 }
            }
        );
        assert!(!session.state.lucky_number_show_flag);

        let lucky_print = session
            .apply_special_routine(&runtime, "PrintTodaysLuckyNumber")
            .expect("print lucky number");

        assert_eq!(
            lucky_print.outcome.effect,
            SpecialRoutineEffect::PrintTodaysLuckyNumber {
                lucky_number: 258,
                formatted: "00258".to_string()
            }
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("00258")
        );

        let mut winner = wounded_runtime_pokemon("CHIKORITA");
        winner.original_trainer_id = 1_358;
        session
            .state
            .storage
            .register_capture_in_box(0, winner)
            .expect("store lucky winner");
        session.state.sync_party_from_storage();
        let lucky_winner = session
            .apply_special_routine(&runtime, "CheckForLuckyNumberWinners")
            .expect("check lucky winners");

        assert_eq!(
            lucky_winner.outcome.effect,
            SpecialRoutineEffect::CheckForLuckyNumberWinners {
                lucky_number: 258,
                tier: 3,
                source: Some(
                    crystal_core::systems::special_routines::LuckyNumberWinnerSource::Party
                ),
                species: Some("CHIKORITA".to_string()),
                text_label: Some("LuckyNumberMatchPartyText".to_string())
            }
        );
        session.state.money = 54_321;
        session.state.coins = 987;
        let place_money = session
            .apply_special_routine(&runtime, "PlaceMoneyTopRight")
            .expect("place money");

        assert_eq!(
            place_money.outcome.effect,
            SpecialRoutineEffect::PlaceMoneyTopRight {
                money: 54_321,
                formatted: "054321".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.money_events.len(), 1);

        let balance = session
            .apply_special_routine(&runtime, "DisplayMoneyAndCoinBalance")
            .expect("display money and coins");

        assert_eq!(
            balance.outcome.effect,
            SpecialRoutineEffect::DisplayMoneyAndCoinBalance {
                money: 54_321,
                coins: 987,
                formatted_money: "054321".to_string(),
                formatted_coins: "0987".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.money_events.len(), 2);
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_2")
                .map(String::as_str),
            Some("0987")
        );
        let coin_case = session
            .apply_special_routine(&runtime, "DisplayCoinCaseBalance")
            .expect("display coin case");

        assert_eq!(
            coin_case.outcome.effect,
            SpecialRoutineEffect::DisplayCoinCaseBalance {
                coins: 987,
                formatted_coins: "0987".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.money_events.len(), 3);
        session.state.gs_healings = 9;
        let healings = session
            .apply_special_routine(&runtime, "GSHealings")
            .expect("gs healings");

        assert_eq!(
            healings.outcome.effect,
            SpecialRoutineEffect::GsHealings { healings: 9 }
        );

        session.state.trainer_rankings_healings = 11;
        let rankings = session
            .apply_special_routine(&runtime, "StubbedTrainerRankings_Healings")
            .expect("trainer rankings healings");

        assert_eq!(
            rankings.outcome.effect,
            SpecialRoutineEffect::TrainerRankingsHealings { healings: 11 }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("old".to_string(), "value".to_string());
        let reset = session
            .apply_special_routine(&runtime, "Reset")
            .expect("reset");

        assert_eq!(
            reset.outcome.effect,
            SpecialRoutineEffect::Reset {
                value: "$0".to_string()
            }
        );
        assert!(session.state.script_runtime.reset_requested);
        assert_eq!(session.state.script_runtime.variables.len(), 1);

        let mut ho_oh = wounded_runtime_pokemon("HO_OH");
        ho_oh.original_trainer_id = 1234;
        session
            .state
            .storage
            .register_capture_in_box(0, ho_oh)
            .expect("store ho-oh");
        session.state.sync_party_from_storage();
        for flag in [
            "EVENT_UNLEASHED_SUICUNE",
            "EVENT_UNLEASHED_RAIKOU",
            "EVENT_UNLEASHED_ENTEI",
        ] {
            session
                .state
                .flags
                .set_event_flag(flag, true)
                .expect("set beast flag");
        }
        let chamber = session
            .apply_special_routine(&runtime, "HoOhChamber")
            .expect("ho-oh chamber");

        assert_eq!(
            chamber.outcome.effect,
            SpecialRoutineEffect::HoOhChamber {
                has_ho_oh: true,
                suicune_unleashed: true,
                raikou_unleashed: true,
                entei_unleashed: true,
                open: true
            }
        );
        assert_ne!(palette.state_checksum, snorlax.state_checksum);
        assert_ne!(snorlax.state_checksum, day.state_checksum);
        assert_ne!(day.state_checksum, dst.state_checksum);
        assert_ne!(time.state_checksum, kenji.state_checksum);
        assert_ne!(kenji.state_checksum, lucky_flag.state_checksum);
        assert_ne!(lucky_reset.state_checksum, lucky_print.state_checksum);
        assert_ne!(lucky_print.state_checksum, lucky_winner.state_checksum);
        assert_ne!(lucky_winner.state_checksum, place_money.state_checksum);
        assert_ne!(place_money.state_checksum, balance.state_checksum);
        assert_ne!(balance.state_checksum, coin_case.state_checksum);
        assert_ne!(coin_case.state_checksum, healings.state_checksum);
        assert_ne!(healings.state_checksum, rankings.state_checksum);
        assert_ne!(rankings.state_checksum, reset.state_checksum);
        assert_ne!(reset.state_checksum, chamber.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_heal_party_skips_unknown_move_pp_but_heals_known_state() {
        let root = temp_repository_root("special-heal-party-move-reject");
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
        assert!(runtime.has_special_routine("HealParty"));
        assert!(!runtime.has_special_routine("healparty"));
        assert!(runtime.special_routine_ids().contains("HealParty"));
        assert!(runtime.require_special_routine("HealParty").is_ok());
        assert!(runtime.require_special_routine("healparty").is_err());
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let mut pokemon = wounded_runtime_pokemon("CHIKORITA");
        pokemon.moves[0].name = "tackle".to_string();
        session
            .state
            .storage
            .register_capture_in_box(0, pokemon)
            .expect("store");
        session.state.sync_party_from_storage();
        let special = session
            .apply_special_routine(&runtime, "HealParty")
            .expect("unknown move metadata does not block HP/status healing");
        assert_eq!(
            special.outcome.effect,
            SpecialRoutineEffect::HealParty {
                healed_slots: vec![0]
            }
        );
        let healed = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("healed party Pokemon");
        assert_eq!(healed.hp, healed.max_hp);
        assert_eq!(healed.status, None);
        assert_eq!(healed.moves[0].current_pp, 1);
        let mut shell = RuntimeGameShell {
            asset_root: asset_root.clone(),
            runtime: runtime.clone(),
            session,
            last_frame: None,
            linked_menu_results: Vec::new(),
            runtime_command_sequence: 0,
            runtime_commands: Vec::new(),
            runtime_results: Vec::new(),
            retain_runtime_journal: true,
        };
        assert!(shell.has_special_routine("HealParty"));
        assert!(!shell.has_special_routine("healparty"));
        assert!(shell.special_routine_ids().contains("HealParty"));
        assert!(shell.require_special_routine("HealParty").is_ok());
        assert!(shell.require_special_routine("healparty").is_err());
        let shell_error = shell
            .apply_special_routine("healparty")
            .expect_err("shell rejects routine case mismatch before mutation");
        assert!(
            shell_error
                .to_string()
                .contains("compiled game pack missing exact special routine healparty"),
            "{shell_error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_town_map_reports_current_landmark_from_definitive_pack_data() {
        let root = temp_repository_root("town-map");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut town_map = town_map_item();
        town_map.effect = "MOD_TOWN_MAP".to_string();
        data.items.insert("TOWN_MAP".to_string(), town_map);
        add_runtime_landmark(&mut data);
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
            .add_item(&runtime.data.items["TOWN_MAP"], 1)
            .expect("add town map");

        let use_item = session
            .use_bag_town_map_in_field(&runtime, "TOWN_MAP")
            .expect("use town map");

        assert_eq!(use_item.item_use.item_id, "TOWN_MAP");
        assert!(!use_item.item_use.consumed);
        assert_eq!(use_item.map_name, "RuntimeMap");
        assert_eq!(use_item.map_constant, "RUNTIME_MAP");
        assert_eq!(use_item.environment, "ROUTE");
        assert_eq!(use_item.landmark.constant, "LANDMARK_RUNTIME_TOWN");
        assert_eq!(use_item.landmark.label, "RUNTIME_TOWN");
        assert_eq!(use_item.landmark.name, "RUNTIME TOWN");
        assert_eq!(use_item.landmark.x, 12);
        assert_eq!(use_item.landmark.y, 24);
        assert_eq!(use_item.landmark.region, "JOHTO");
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["TOWN_MAP"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_town_map_rejects_wrong_effect_or_missing_landmark_without_mutation() {
        let root = temp_repository_root("town-map-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad_town_map = runtime_item("BAD_TOWN_MAP", item_pocket("KEY_ITEM"));
        bad_town_map.effect = "NONE".to_string();
        bad_town_map.field_menu = "ITEMMENU_CURRENT".to_string();
        bad_town_map.field_usable = true;
        data.items.insert("BAD_TOWN_MAP".to_string(), bad_town_map);
        data.items.insert("TOWN_MAP".to_string(), town_map_item());
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
            .add_item(&runtime.data.items["BAD_TOWN_MAP"], 1)
            .expect("add bad town map");
        session
            .state
            .bag
            .add_item(&runtime.data.items["TOWN_MAP"], 1)
            .expect("add town map");

        let before_bad = session.state.clone();
        let bad = session
            .use_bag_town_map_in_field(&runtime, "BAD_TOWN_MAP")
            .expect_err("wrong effect rejected");
        let bad = error_debug(bad);
        assert!(bad.contains("InvalidFieldItemId"), "{bad}");
        assert_eq!(session.state, before_bad);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_town_map_rejects_missing_landmark_constant_without_mutation() {
        let root = temp_repository_root("town-map-missing-landmark");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert("TOWN_MAP".to_string(), town_map_item());
        data.pokegear_landmarks.map_to_landmark.insert(
            "RuntimeMap".to_string(),
            "LANDMARK_RUNTIME_TOWN".to_string(),
        );
        let error = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect_err("missing landmark constant is rejected by pack verification");
        let error = error_debug(error);
        assert!(
            error.contains("unknown_pokegear_landmark_constant")
                && error.contains("LANDMARK_RUNTIME_TOWN"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_coin_case_reports_coin_balance_from_definitive_item_rule() {
        let root = temp_repository_root("coin-case");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut coin_case = coin_case_item();
        coin_case.effect = "MOD_COIN_CASE".to_string();
        data.items.insert("COIN_CASE".to_string(), coin_case);
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
            .add_item(&runtime.data.items["COIN_CASE"], 1)
            .expect("add coin case");
        session.state.coins = 321;

        let use_item = session
            .use_bag_coin_case_in_field(&runtime, "COIN_CASE")
            .expect("use coin case");

        assert_eq!(use_item.item_use.item_id, "COIN_CASE");
        assert_eq!(use_item.balance_label, "COIN");
        assert_eq!(use_item.balance, 321);
        assert!(!use_item.item_use.consumed);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["COIN_CASE"]),
            1
        );
        assert_eq!(session.state.coins, 321);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_blue_card_reports_exact_saved_wram_balance() {
        let root = temp_repository_root("blue-card");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut blue_card = blue_card_item();
        blue_card.effect = "MOD_BLUE_CARD".to_string();
        data.items.insert("BLUE_CARD".to_string(), blue_card);
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
            .add_item(&runtime.data.items["BLUE_CARD"], 1)
            .expect("add blue card");
        session.state.blue_card_balance = 12;

        let use_item = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect("use blue card");

        assert_eq!(use_item.item_use.item_id, "BLUE_CARD");
        assert_eq!(use_item.balance_label, "POINT");
        assert_eq!(use_item.balance, 12);
        assert!(!use_item.item_use.consumed);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["BLUE_CARD"]),
            1
        );
        assert_eq!(session.state.blue_card_balance, 12);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_blue_card_missing_balance_reports_initial_zero() {
        let root = temp_repository_root("blue-card-zero");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert("BLUE_CARD".to_string(), blue_card_item());
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
            .add_item(&runtime.data.items["BLUE_CARD"], 1)
            .expect("add blue card");

        let use_item = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect("use blue card with initial balance");

        assert_eq!(use_item.balance_label, "POINT");
        assert_eq!(use_item.balance, 0);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_balance_key_items_reject_wrong_effect_and_invalid_blue_card_without_mutation() {
        let root = temp_repository_root("balance-key-item-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad_coin_case = runtime_item("BAD_COIN_CASE", item_pocket("KEY_ITEM"));
        bad_coin_case.effect = "NONE".to_string();
        bad_coin_case.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_coin_case.field_usable = true;
        let mut bad_blue_card = runtime_item("BAD_BLUE_CARD", item_pocket("KEY_ITEM"));
        bad_blue_card.effect = "NONE".to_string();
        bad_blue_card.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_blue_card.field_usable = true;
        data.items
            .insert("BAD_COIN_CASE".to_string(), bad_coin_case);
        data.items
            .insert("BAD_BLUE_CARD".to_string(), bad_blue_card);
        data.items.insert("BLUE_CARD".to_string(), blue_card_item());
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
            .add_item(&runtime.data.items["BAD_COIN_CASE"], 1)
            .expect("add bad coin case");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_BLUE_CARD"], 1)
            .expect("add bad blue card");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BLUE_CARD"], 1)
            .expect("add blue card");

        let before_bad_coin = session.state.clone();
        let bad_coin = session
            .use_bag_coin_case_in_field(&runtime, "BAD_COIN_CASE")
            .expect_err("wrong coin case effect rejected");
        let bad_coin = error_debug(bad_coin);
        assert!(bad_coin.contains("InvalidFieldItemId"), "{bad_coin}");
        assert_eq!(session.state, before_bad_coin);

        let before_bad_blue = session.state.clone();
        let bad_blue = session
            .use_bag_blue_card_in_field(&runtime, "BAD_BLUE_CARD")
            .expect_err("wrong blue card effect rejected");
        let bad_blue = error_debug(bad_blue);
        assert!(bad_blue.contains("InvalidFieldItemId"), "{bad_blue}");
        assert_eq!(session.state, before_bad_blue);

        session.state.blue_card_balance = 31;
        let before_out_of_range = session.state.clone();
        let out_of_range = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect_err("out-of-range blue card balance rejected");
        let out_of_range = error_debug(out_of_range);
        assert!(
            out_of_range.contains("BlueCardBalanceOutOfRange"),
            "{out_of_range}"
        );
        assert_eq!(session.state, before_out_of_range);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_squirtbottle_runs_facing_sudowoodo_object_script_from_pack() {
        let root = temp_repository_root("squirtbottle");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut squirtbottle = squirtbottle_item();
        squirtbottle.effect = "MOD_SQUIRTBOTTLE".to_string();
        data.items.insert("SQUIRTBOTTLE".to_string(), squirtbottle);
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut weird_tree = runtime_object("RUNTIME_WEIRD_TREE", "-1");
        weird_tree.x = 0;
        weird_tree.y = 1;
        weird_tree.spritemovedata = "SPRITEMOVEDATA_SUDOWOODO".to_string();
        weird_tree.script = "ModdedWateredTreeScript".to_string();
        map.objects = vec![weird_tree];
        map.scripts.insert(
            "ModdedWateredTreeScript".to_string(),
            serde_json::Value::Array(Vec::new()),
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
            .add_item(&runtime.data.items["SQUIRTBOTTLE"], 1)
            .expect("add squirtbottle");

        let use_item = session
            .use_bag_squirtbottle_in_field(&runtime, "SQUIRTBOTTLE")
            .expect("use squirtbottle");

        assert_eq!(use_item.item_use.item_id, "SQUIRTBOTTLE");
        assert!(!use_item.item_use.consumed);
        assert_eq!(use_item.target_tile, TilePosition::new(0, 1));
        assert_eq!(
            use_item.target_object_identifier.as_deref(),
            Some("RUNTIME_WEIRD_TREE")
        );
        assert_eq!(use_item.target_movement, "SPRITEMOVEDATA_SUDOWOODO");
        assert_eq!(
            use_item.target_script.as_deref(),
            Some("ModdedWateredTreeScript")
        );
        assert_eq!(
            session.state.script_runtime.next_script.as_ref().map(|location| location.script.as_str()),
            Some("ModdedWateredTreeScript")
        );
        assert_eq!(
            session.state.script_runtime.last_talked_object.as_deref(),
            Some("RUNTIME_WEIRD_TREE")
        );
        assert_eq!(
            session.overworld.last_talked_object_identifier.as_deref(),
            Some("RUNTIME_WEIRD_TREE")
        );
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["SQUIRTBOTTLE"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_squirtbottle_records_nothing_path_without_target_script() {
        let root = temp_repository_root("squirtbottle-nothing");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items
            .insert("SQUIRTBOTTLE".to_string(), squirtbottle_item());
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut npc = runtime_object("RUNTIME_NPC", "-1");
        npc.x = 0;
        npc.y = 1;
        npc.spritemovedata = "SPRITEMOVEDATA_STANDING_DOWN".to_string();
        npc.script = "RuntimeNpcScript".to_string();
        map.objects = vec![npc];
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
            .add_item(&runtime.data.items["SQUIRTBOTTLE"], 1)
            .expect("add squirtbottle");

        let use_item = session
            .use_bag_squirtbottle_in_field(&runtime, "SQUIRTBOTTLE")
            .expect("use squirtbottle");

        assert_eq!(use_item.target_script, None);
        assert_eq!(
            use_item.target_object_identifier.as_deref(),
            Some("RUNTIME_NPC")
        );
        assert_eq!(use_item.target_movement, "SPRITEMOVEDATA_STANDING_DOWN");
        assert_eq!(session.state.script_runtime.next_script, None);
        assert_eq!(session.state.script_runtime.last_talked_object, None);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_squirtbottle_rejects_wrong_effect_and_missing_target_script_without_mutation() {
        let root = temp_repository_root("squirtbottle-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad = runtime_item("BAD_SQUIRTBOTTLE", item_pocket("KEY_ITEM"));
        bad.effect = "NONE".to_string();
        bad.field_menu = "ITEMMENU_CLOSE".to_string();
        bad.field_usable = true;
        data.items.insert("BAD_SQUIRTBOTTLE".to_string(), bad);
        data.items
            .insert("SQUIRTBOTTLE".to_string(), squirtbottle_item());
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut weird_tree = runtime_object("RUNTIME_WEIRD_TREE", "-1");
        weird_tree.x = 0;
        weird_tree.y = 1;
        weird_tree.spritemovedata = "SPRITEMOVEDATA_SUDOWOODO".to_string();
        weird_tree.script = "MissingWateredTreeScript".to_string();
        map.objects = vec![weird_tree];
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
            .add_item(&runtime.data.items["BAD_SQUIRTBOTTLE"], 1)
            .expect("add bad squirtbottle");
        session
            .state
            .bag
            .add_item(&runtime.data.items["SQUIRTBOTTLE"], 1)
            .expect("add squirtbottle");

        let before_bad_effect = session.state.clone();
        let bad_effect = session
            .use_bag_squirtbottle_in_field(&runtime, "BAD_SQUIRTBOTTLE")
            .expect_err("wrong effect rejected");
        let bad_effect = error_debug(bad_effect);
        assert!(bad_effect.contains("InvalidFieldItemId"), "{bad_effect}");
        assert_eq!(session.state, before_bad_effect);

        let before_missing_script = session.state.clone();
        let missing_script = session
            .use_bag_squirtbottle_in_field(&runtime, "SQUIRTBOTTLE")
            .expect_err("missing target script rejected");
        let missing_script = error_debug(missing_script);
        assert!(
            missing_script.contains("MissingWateredTreeScript"),
            "{missing_script}"
        );
        assert_eq!(session.state, before_missing_script);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_heals_active_party_pokemon_from_exact_pack_effect() {
        let root = temp_repository_root("battle-item-heal");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        let healed_hp = player.max_hp.min(11 + 20);
        player.hp = 11;
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 2)
            .expect("add potion");
        let before_checksum = game_state_checksum(&session.state).expect("checksum");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "POTION")
            .expect("use battle potion");

        assert_eq!(item_use.item_use.item_id, "POTION");
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.battle_item.hp_before, 11);
        assert_eq!(item_use.battle_item.hp_after, healed_hp);
        assert_ne!(item_use.state_checksum, before_checksum);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("lead")
                .hp,
            healed_hp
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_x_item_raises_active_party_stat_stage_from_pack_data() {
        let root = temp_repository_root("battle-item-x-attack");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut x_attack = runtime_item("X_ATTACK", item_pocket("ITEM"));
        x_attack.effect = "X_ITEM".to_string();
        x_attack.battle_stat_boost_stat = Some("ATTACK".to_string());
        x_attack.battle_stat_boost_stages = Some(1);
        x_attack.field_menu = "ITEMMENU_NOUSE".to_string();
        x_attack.field_usable = false;
        x_attack.battle_menu = "ITEMMENU_CLOSE".to_string();
        x_attack.battle_usable = true;
        x_attack.consumable = true;
        data.items.insert("X_ATTACK".to_string(), x_attack);
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
            .add_item(&runtime.data.items["X_ATTACK"], 1)
            .expect("add X Attack");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "X_ATTACK")
            .expect("use X Attack");

        assert_eq!(item_use.item_use.item_id, "X_ATTACK");
        assert_eq!(
            item_use.battle_item.battle_stat_stage_changes,
            vec![crystal_core::systems::battle_items::BattleItemStageChange {
                stat: "ATTACK".to_string(),
                stage_before: 0,
                stage_after: 1,
            }]
        );
        let lead = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("lead");
        assert_eq!(lead.stat_boosts[&crystal_core::models::Stat::Attack], 1);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["X_ATTACK"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_x_item_rejects_capped_stat_without_consumption() {
        let root = temp_repository_root("battle-item-x-attack-capped");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut x_attack = runtime_item("X_ATTACK", item_pocket("ITEM"));
        x_attack.effect = "X_ITEM".to_string();
        x_attack.battle_stat_boost_stat = Some("ATTACK".to_string());
        x_attack.battle_stat_boost_stages = Some(1);
        x_attack.field_menu = "ITEMMENU_NOUSE".to_string();
        x_attack.field_usable = false;
        x_attack.battle_menu = "ITEMMENU_CLOSE".to_string();
        x_attack.battle_usable = true;
        x_attack.consumable = true;
        data.items.insert("X_ATTACK".to_string(), x_attack);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player
            .stat_boosts
            .insert(crystal_core::models::Stat::Attack, 6);
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["X_ATTACK"], 1)
            .expect("add X Attack");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "X_ATTACK")
            .expect_err("capped stat rejects X Attack");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["X_ATTACK"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_full_restore_heals_and_clears_status() {
        let root = temp_repository_root("battle-item-full-restore");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut full_restore = runtime_item("FULL_RESTORE", item_pocket("ITEM"));
        full_restore.effect = "MOD_FULL_RESTORE".to_string();
        full_restore.parameter = -1;
        full_restore.status_heals = vec![
            "POISON".to_string(),
            "BURN".to_string(),
            "FREEZE".to_string(),
            "SLEEP".to_string(),
            "PARALYSIS".to_string(),
        ];
        full_restore.field_menu = "ITEMMENU_PARTY".to_string();
        full_restore.field_usable = true;
        full_restore.battle_menu = "ITEMMENU_PARTY".to_string();
        full_restore.battle_usable = true;
        full_restore.consumable = true;
        data.items.insert("FULL_RESTORE".to_string(), full_restore);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        let full_hp = player.max_hp;
        player.hp = 11;
        player.status = Some("POISON".to_string());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["FULL_RESTORE"], 1)
            .expect("add full restore");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "FULL_RESTORE")
            .expect("use full restore");

        assert_eq!(item_use.item_use.item_id, "FULL_RESTORE");
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.battle_item.hp_before, 11);
        assert_eq!(item_use.battle_item.hp_after, full_hp);
        assert_eq!(
            item_use.battle_item.status_before,
            Some("POISON".to_string())
        );
        assert_eq!(item_use.battle_item.status_after, None);
        let lead = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("lead");
        assert_eq!(lead.hp, full_hp);
        assert_eq!(lead.status, None);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["FULL_RESTORE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_status_heal_uses_exact_modpack_statuses() {
        let root = temp_repository_root("battle-item-status-heal");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut antidote = runtime_item("ANTIDOTE", item_pocket("ITEM"));
        antidote.effect = "STATUS_HEAL".to_string();
        antidote.status_heals = vec!["POISON".to_string()];
        antidote.field_menu = "ITEMMENU_PARTY".to_string();
        antidote.field_usable = true;
        antidote.battle_menu = "ITEMMENU_PARTY".to_string();
        antidote.battle_usable = true;
        antidote.consumable = true;
        data.items.insert("ANTIDOTE".to_string(), antidote);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.status = Some("POISON".to_string());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ANTIDOTE"], 1)
            .expect("add antidote");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "ANTIDOTE")
            .expect("use antidote");

        assert_eq!(item_use.item_use.item_id, "ANTIDOTE");
        assert_eq!(
            item_use.battle_item.status_before,
            Some("POISON".to_string())
        );
        assert_eq!(item_use.battle_item.status_after, None);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("lead")
                .status,
            None
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["ANTIDOTE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_status_heal_rejects_mismatched_status_without_consumption() {
        let root = temp_repository_root("battle-item-status-mismatch");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut antidote = runtime_item("ANTIDOTE", item_pocket("ITEM"));
        antidote.effect = "STATUS_HEAL".to_string();
        antidote.status_heals = vec!["POISON".to_string()];
        antidote.field_menu = "ITEMMENU_PARTY".to_string();
        antidote.field_usable = true;
        antidote.battle_menu = "ITEMMENU_PARTY".to_string();
        antidote.battle_usable = true;
        antidote.consumable = true;
        data.items.insert("ANTIDOTE".to_string(), antidote);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.status = Some("BURN".to_string());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ANTIDOTE"], 1)
            .expect("add antidote");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "ANTIDOTE")
            .expect_err("antidote does not heal burn");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["ANTIDOTE"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_revives_explicit_party_target_from_pack_percent() {
        let root = temp_repository_root("battle-item-revive");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut revive = runtime_item("REVIVE", item_pocket("ITEM"));
        revive.effect = "REVIVE".to_string();
        revive.revive_hp_percent = Some(50);
        revive.field_menu = "ITEMMENU_PARTY".to_string();
        revive.field_usable = true;
        revive.battle_menu = "ITEMMENU_PARTY".to_string();
        revive.battle_usable = true;
        revive.consumable = true;
        data.items.insert("REVIVE".to_string(), revive);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fainted = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        let revive_hp = fainted.max_hp / 2;
        fainted.hp = 0;
        let mut active = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        active.hp = 22;
        session
            .state
            .storage
            .register_capture_in_box(0, fainted)
            .expect("register fainted");
        session
            .state
            .storage
            .register_capture_in_box(0, active)
            .expect("register active");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REVIVE"], 1)
            .expect("add revive");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        assert_eq!(session.state.battle_active_party_index, Some(1));

        let item_use = session
            .use_bag_item_on_battle_party_pokemon(&runtime, "REVIVE", 0)
            .expect("use revive");

        assert_eq!(item_use.item_use.item_id, "REVIVE");
        assert_eq!(item_use.battle_item.hp_before, 0);
        assert_eq!(item_use.battle_item.hp_after, revive_hp);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("revived")
                .hp,
            revive_hp
        );
        assert_eq!(session.state.bag.quantity(&runtime.data.items["REVIVE"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_revive_rejects_non_fainted_target_without_consumption() {
        let root = temp_repository_root("battle-item-revive-healthy");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut revive = runtime_item("REVIVE", item_pocket("ITEM"));
        revive.effect = "REVIVE".to_string();
        revive.revive_hp_percent = Some(50);
        revive.field_menu = "ITEMMENU_PARTY".to_string();
        revive.field_usable = true;
        revive.battle_menu = "ITEMMENU_PARTY".to_string();
        revive.battle_usable = true;
        revive.consumable = true;
        data.items.insert("REVIVE".to_string(), revive);
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
            .add_item(&runtime.data.items["REVIVE"], 1)
            .expect("add revive");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_battle_party_pokemon(&runtime, "REVIVE", 0)
            .expect_err("revive cannot target healthy Pokemon");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["REVIVE"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_heals_party_pokemon_from_exact_pack_effect() {
        let root = temp_repository_root("field-item-potion");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        let healed_hp = player.max_hp.min(11 + 20);
        player.hp = 11;
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 1)
            .expect("add potion");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "POTION", 0)
            .expect("use field potion");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "POTION");
        assert_eq!(item_use.item_effect.hp_before, 11);
        assert_eq!(item_use.item_effect.hp_after, healed_hp);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .hp,
            healed_hp
        );
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_status_heal_uses_exact_modpack_statuses() {
        let root = temp_repository_root("field-item-status");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut antidote = runtime_item("ANTIDOTE", item_pocket("ITEM"));
        antidote.effect = "STATUS_HEAL".to_string();
        antidote.status_heals = vec!["POISON".to_string()];
        antidote.field_menu = "ITEMMENU_PARTY".to_string();
        antidote.field_usable = true;
        antidote.battle_menu = "ITEMMENU_PARTY".to_string();
        antidote.battle_usable = true;
        antidote.consumable = true;
        data.items.insert("ANTIDOTE".to_string(), antidote);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.status = Some("POISON".to_string());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ANTIDOTE"], 1)
            .expect("add antidote");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "ANTIDOTE", 0)
            .expect("use field antidote");

        assert_eq!(
            item_use.item_effect.status_before,
            Some("POISON".to_string())
        );
        assert_eq!(item_use.item_effect.status_after, None);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .status,
            None
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["ANTIDOTE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_revives_explicit_party_target_from_pack_percent() {
        let root = temp_repository_root("field-item-revive");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut revive = runtime_item("REVIVE", item_pocket("ITEM"));
        revive.effect = "REVIVE".to_string();
        revive.revive_hp_percent = Some(50);
        revive.field_menu = "ITEMMENU_PARTY".to_string();
        revive.field_usable = true;
        revive.battle_menu = "ITEMMENU_PARTY".to_string();
        revive.battle_usable = true;
        revive.consumable = true;
        data.items.insert("REVIVE".to_string(), revive);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fainted = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        let revive_hp = fainted.max_hp / 2;
        fainted.hp = 0;
        session
            .state
            .storage
            .register_capture_in_box(0, fainted)
            .expect("register fainted");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REVIVE"], 1)
            .expect("add revive");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "REVIVE", 0)
            .expect("use field revive");

        assert_eq!(item_use.item_effect.hp_before, 0);
        assert_eq!(item_use.item_effect.hp_after, revive_hp);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("revived")
                .hp,
            revive_hp
        );
        assert_eq!(session.state.bag.quantity(&runtime.data.items["REVIVE"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rejects_full_hp_without_consumption() {
        let root = temp_repository_root("field-item-full-hp");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
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
            .add_item(&runtime.data.items["POTION"], 1)
            .expect("add potion");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "POTION", 0)
            .expect_err("full HP has no target change");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_vitamin_raises_party_stat_exp_from_pack_data() {
        let root = temp_repository_root("field-item-vitamin");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut protein = runtime_item("PROTEIN", item_pocket("ITEM"));
        protein.effect = "VITAMIN".to_string();
        protein.vitamin_stat = Some("ATTACK".to_string());
        protein.vitamin_stat_exp = Some(2560);
        protein.vitamin_max_stat_exp = Some(25600);
        protein.field_menu = "ITEMMENU_PARTY".to_string();
        protein.field_usable = true;
        protein.battle_menu = "ITEMMENU_NOUSE".to_string();
        protein.battle_usable = false;
        protein.consumable = true;
        data.items.insert("PROTEIN".to_string(), protein);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 30, Dv::default());
        player.attack_exp = 0;
        let attack_before = player.attack;
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PROTEIN"], 1)
            .expect("add protein");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "PROTEIN", 0)
            .expect("use field protein");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "PROTEIN");
        assert_eq!(item_use.item_effect.stat_changes.len(), 1);
        assert_eq!(item_use.item_effect.stat_changes[0].stat, "ATTACK");
        assert_eq!(item_use.item_effect.stat_changes[0].stat_exp_before, 0);
        assert_eq!(item_use.item_effect.stat_changes[0].stat_exp_after, 2560);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.attack_exp, 2560);
        assert!(pokemon.attack >= attack_before);
        assert_eq!(
            item_use.item_effect.stat_changes[0].stat_after,
            pokemon.attack
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["PROTEIN"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_vitamin_rejects_maxed_stat_exp_without_consumption() {
        let root = temp_repository_root("field-item-vitamin-maxed");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut protein = runtime_item("PROTEIN", item_pocket("ITEM"));
        protein.effect = "VITAMIN".to_string();
        protein.vitamin_stat = Some("ATTACK".to_string());
        protein.vitamin_stat_exp = Some(2560);
        protein.vitamin_max_stat_exp = Some(25600);
        protein.field_menu = "ITEMMENU_PARTY".to_string();
        protein.field_usable = true;
        protein.battle_menu = "ITEMMENU_NOUSE".to_string();
        protein.battle_usable = false;
        protein.consumable = true;
        data.items.insert("PROTEIN".to_string(), protein);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 30, Dv::default());
        player.attack_exp = 25600;
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PROTEIN"], 1)
            .expect("add protein");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "PROTEIN", 0)
            .expect_err("maxed vitamin target rejects item");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["PROTEIN"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rare_candy_levels_party_pokemon_from_pack_data() {
        let root = temp_repository_root("field-item-rare-candy");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut rare_candy = runtime_item("RARE_CANDY", item_pocket("ITEM"));
        rare_candy.effect = "MOD_CANDY".to_string();
        rare_candy.rare_candy_level_gain = Some(1);
        rare_candy.field_menu = "ITEMMENU_PARTY".to_string();
        rare_candy.field_usable = true;
        rare_candy.battle_menu = "ITEMMENU_NOUSE".to_string();
        rare_candy.battle_usable = false;
        rare_candy.consumable = true;
        data.items.insert("RARE_CANDY".to_string(), rare_candy);
        data.moves
            .insert("GROWL".to_string(), runtime_move_named("GROWL", 40));
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(10, "GROWL".to_string()),
            ],
        );
        data.evolutions
            .0
            .insert("CHIKORITA".to_string(), Vec::new());
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 9, Dv::default());
        player.species.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        player.experience =
            calculate_experience(&runtime.data.growth_rates, "GROWTH_MEDIUM_FAST", 9).unwrap();
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["RARE_CANDY"], 1)
            .expect("add rare candy");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "RARE_CANDY", 0)
            .expect("use rare candy");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "RARE_CANDY");
        assert_eq!(item_use.item_effect.level_before, 9);
        assert_eq!(item_use.item_effect.level_after, 10);
        assert_eq!(
            item_use.item_effect.learned_moves,
            vec!["GROWL".to_string()]
        );
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.level, 10);
        assert_eq!(
            pokemon.experience,
            calculate_experience(&runtime.data.growth_rates, "GROWTH_MEDIUM_FAST", 10).unwrap()
        );
        assert_eq!(pokemon.moves[1].name, "GROWL");
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["RARE_CANDY"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rare_candy_rejects_max_level_without_consumption() {
        let root = temp_repository_root("field-item-rare-candy-maxed");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut rare_candy = runtime_item("RARE_CANDY", item_pocket("ITEM"));
        rare_candy.effect = "RARE_CANDY".to_string();
        rare_candy.rare_candy_level_gain = Some(1);
        rare_candy.field_menu = "ITEMMENU_PARTY".to_string();
        rare_candy.field_usable = true;
        rare_candy.battle_menu = "ITEMMENU_NOUSE".to_string();
        rare_candy.battle_usable = false;
        rare_candy.consumable = true;
        data.items.insert("RARE_CANDY".to_string(), rare_candy);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 100, Dv::default());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["RARE_CANDY"], 1)
            .expect("add rare candy");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "RARE_CANDY", 0)
            .expect_err("max level rejects rare candy");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["RARE_CANDY"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_evolution_stone_evolves_party_pokemon_from_pack_tables() {
        let root = temp_repository_root("field-item-evo-stone");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut thunderstone = runtime_item("THUNDERSTONE", item_pocket("ITEM"));
        thunderstone.effect = "MOD_STONE".to_string();
        thunderstone.field_menu = "ITEMMENU_PARTY".to_string();
        thunderstone.field_usable = true;
        thunderstone.battle_menu = "ITEMMENU_NOUSE".to_string();
        thunderstone.battle_usable = false;
        thunderstone.consumable = true;
        data.items.insert("THUNDERSTONE".to_string(), thunderstone);
        data.pokemon.insert(
            "PIKACHU".to_string(),
            PokemonSpecies::new_for_tests("PIKACHU", BaseStats::new(35, 55, 30, 90, 50, 50)),
        );
        data.pokemon.insert(
            "RAICHU".to_string(),
            PokemonSpecies::new_for_tests("RAICHU", BaseStats::new(60, 90, 55, 100, 90, 80)),
        );
        add_runtime_species_presentation(&mut data, "PIKACHU");
        add_runtime_species_presentation(&mut data, "RAICHU");
        data.moves.insert(
            "THUNDERBOLT".to_string(),
            runtime_move_named("THUNDERBOLT", 15),
        );
        data.learnsets.insert(
            "RAICHU".to_string(),
            vec![LearnsetEntry(20, "THUNDERBOLT".to_string())],
        );
        data.evolutions.0.insert(
            "PIKACHU".to_string(),
            vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
        );
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player =
            Pokemon::new_for_tests(runtime.data.pokemon["PIKACHU"].clone(), 20, Dv::default());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["THUNDERSTONE"], 1)
            .expect("add thunderstone");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "THUNDERSTONE", 0)
            .expect("use thunderstone");

        assert_eq!(item_use.item_effect.item_id, "THUNDERSTONE");
        assert_eq!(
            item_use.item_effect.evolution_target,
            Some("RAICHU".to_string())
        );
        assert_eq!(
            item_use.item_effect.learned_moves,
            vec!["THUNDERBOLT".to_string()]
        );
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.species.id, "RAICHU");
        assert_eq!(pokemon.moves[0].name, "THUNDERBOLT");
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["THUNDERSTONE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_evolution_stone_rejects_wrong_stone_without_consumption() {
        let root = temp_repository_root("field-item-evo-stone-wrong");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut fire_stone = runtime_item("FIRE_STONE", item_pocket("ITEM"));
        fire_stone.effect = "MOD_STONE".to_string();
        fire_stone.field_menu = "ITEMMENU_PARTY".to_string();
        fire_stone.field_usable = true;
        fire_stone.battle_menu = "ITEMMENU_NOUSE".to_string();
        fire_stone.battle_usable = false;
        fire_stone.consumable = true;
        data.items.insert("FIRE_STONE".to_string(), fire_stone);
        data.pokemon.insert(
            "PIKACHU".to_string(),
            PokemonSpecies::new_for_tests("PIKACHU", BaseStats::new(35, 55, 30, 90, 50, 50)),
        );
        data.pokemon.insert(
            "RAICHU".to_string(),
            PokemonSpecies::new_for_tests("RAICHU", BaseStats::new(60, 90, 55, 100, 90, 80)),
        );
        data.pokemon.insert(
            "VULPIX".to_string(),
            PokemonSpecies::new_for_tests("VULPIX", BaseStats::new(38, 41, 40, 65, 50, 65)),
        );
        data.pokemon.insert(
            "NINETALES".to_string(),
            PokemonSpecies::new_for_tests("NINETALES", BaseStats::new(73, 76, 75, 100, 81, 100)),
        );
        add_runtime_species_presentation(&mut data, "PIKACHU");
        add_runtime_species_presentation(&mut data, "RAICHU");
        add_runtime_species_presentation(&mut data, "VULPIX");
        add_runtime_species_presentation(&mut data, "NINETALES");
        data.learnsets.insert("RAICHU".to_string(), Vec::new());
        data.learnsets.insert("NINETALES".to_string(), Vec::new());
        data.evolutions.0.insert(
            "PIKACHU".to_string(),
            vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
        );
        data.evolutions.0.insert(
            "VULPIX".to_string(),
            vec![EvolutionEntry::item("NINETALES", "FIRE_STONE")],
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player =
            Pokemon::new_for_tests(runtime.data.pokemon["PIKACHU"].clone(), 20, Dv::default());
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["FIRE_STONE"], 1)
            .expect("add fire stone");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "FIRE_STONE", 0)
            .expect_err("wrong stone has no target change");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["FIRE_STONE"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_sacred_ash_revives_whole_party_from_pack_percent() {
        let root = temp_repository_root("field-item-sacred-ash");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut sacred_ash = runtime_item("MOD_ASH", item_pocket("ITEM"));
        sacred_ash.effect = "MOD_ASH".to_string();
        sacred_ash.party_revive_hp_percent = Some(100);
        sacred_ash.field_menu = "ITEMMENU_CLOSE".to_string();
        sacred_ash.field_usable = true;
        sacred_ash.battle_menu = "ITEMMENU_NOUSE".to_string();
        sacred_ash.battle_usable = false;
        sacred_ash.consumable = true;
        data.items.insert("MOD_ASH".to_string(), sacred_ash);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fainted = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        let revived_hp = fainted.max_hp;
        fainted.hp = 0;
        let mut healthy = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        healthy.hp = 12;
        session
            .state
            .storage
            .register_capture_in_box(0, fainted)
            .expect("register fainted");
        session
            .state
            .storage
            .register_capture_in_box(0, healthy)
            .expect("register healthy");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["MOD_ASH"], 1)
            .expect("add Sacred Ash");

        let item_use = session
            .use_bag_item_on_whole_party(&runtime, "MOD_ASH")
            .expect("use Sacred Ash");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "MOD_ASH");
        assert_eq!(item_use.item_effect.revive_changes.len(), 1);
        assert_eq!(item_use.item_effect.revive_changes[0].party_index, 0);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("slot 0")
                .hp,
            revived_hp
        );
        assert_eq!(
            session.state.storage.party.pokemon[1]
                .as_ref()
                .expect("slot 1")
                .hp,
            12
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["MOD_ASH"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_sacred_ash_rejects_no_fainted_party_without_consumption() {
        let root = temp_repository_root("field-item-sacred-ash-no-target");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut sacred_ash = runtime_item("MOD_ASH", item_pocket("ITEM"));
        sacred_ash.effect = "MOD_ASH".to_string();
        sacred_ash.party_revive_hp_percent = Some(100);
        sacred_ash.field_menu = "ITEMMENU_CLOSE".to_string();
        sacred_ash.field_usable = true;
        sacred_ash.battle_menu = "ITEMMENU_NOUSE".to_string();
        sacred_ash.battle_usable = false;
        sacred_ash.consumable = true;
        data.items.insert("MOD_ASH".to_string(), sacred_ash);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let healthy = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture_in_box(0, healthy)
            .expect("register healthy");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["MOD_ASH"], 1)
            .expect("add Sacred Ash");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_whole_party(&runtime, "MOD_ASH")
            .expect_err("no fainted target rejects Sacred Ash");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["MOD_ASH"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_restores_selected_move_pp_from_compiled_moves() {
        let root = temp_repository_root("field-item-ether");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("GROWL".to_string(), runtime_move_named("GROWL", 40));
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 20,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
        ];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");

        let item_use = session
            .use_bag_item_on_party_move(&runtime, "ETHER", 0, Some(0))
            .expect("use field ether");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.pp_changes.len(), 1);
        assert_eq!(item_use.item_effect.pp_changes[0].move_id, "TACKLE");
        assert_eq!(item_use.item_effect.pp_changes[0].pp_before, 20);
        assert_eq!(item_use.item_effect.pp_changes[0].pp_after, 30);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.moves[0].current_pp, 30);
        assert_eq!(pokemon.moves[1].current_pp, 1);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rejects_full_pp_without_consumption() {
        let root = temp_repository_root("field-item-ether-full");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_move(&runtime, "ETHER", 0, Some(0))
            .expect_err("full PP has no target change");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tm_teaches_explicit_move_and_consumes_tm_flag() {
        let root = temp_repository_root("field-item-tm-headbutt");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("HEADBUTT".to_string(), runtime_move_named("HEADBUTT", 15));
        let mut tm = runtime_item("TM_HEADBUTT", item_pocket("TM_HM"));
        tm.field_menu = "ITEMMENU_PARTY".to_string();
        tm.field_usable = true;
        tm.consumable = true;
        tm.tmhm_index = Some(1);
        tm.tmhm_move = Some("HEADBUTT".to_string());
        data.items.insert("TM_HEADBUTT".to_string(), tm);
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.species.tmhm_learnset = vec!["HEADBUTT".to_string()];
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["TM_HEADBUTT"], 1)
            .expect("add TM");

        let item_use = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "TM_HEADBUTT", 0, None)
            .expect("teach TM");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.learned_move.learned_move, "HEADBUTT");
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .moves
                .last()
                .expect("learned")
                .current_pp,
            15
        );
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["TM_HEADBUTT"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_hm_teaches_explicit_move_without_consuming_hm_flag() {
        let root = temp_repository_root("field-item-hm-cut");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("CUT".to_string(), runtime_move_named("CUT", 30));
        sync_runtime_move_tables(&mut data);
        let mut hm = runtime_item("HM_CUT", item_pocket("TM_HM"));
        hm.field_menu = "ITEMMENU_PARTY".to_string();
        hm.field_usable = true;
        hm.consumable = false;
        hm.tmhm_index = Some(50);
        hm.tmhm_move = Some("CUT".to_string());
        data.items.insert("HM_CUT".to_string(), hm);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.species.tmhm_learnset = vec!["CUT".to_string()];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["HM_CUT"], 1)
            .expect("add HM");

        let item_use = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "HM_CUT", 0, None)
            .expect("teach HM");

        assert!(!item_use.item_use.consumed);
        assert_eq!(item_use.learned_move.learned_move, "CUT");
        assert_eq!(session.state.bag.quantity(&runtime.data.items["HM_CUT"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_cut_field_move_replaces_block_and_persists_override() {
        let root = temp_repository_root("field-move-cut");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "5b": ["FLOOR", "CUT_TREE", "CUT_TREE", "CUT_TREE"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[
                ("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"]),
                ("5b", &["FLOOR", "CUT_TREE", "CUT_TREE", "CUT_TREE"]),
            ]),
        );
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x5b, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        add_runtime_deferred_field_move_global_scripts(&mut data);
        let mut connected_map = runtime_map();
        connected_map.id = "ConnectedMap".to_string();
        connected_map.attributes.map_constant = None;
        connected_map.attributes.blocks_label = Some("ConnectedMap_Blocks".to_string());
        connected_map.attributes.map_scripts_label = Some("ConnectedMap_MapScripts".to_string());
        connected_map.attributes.map_events_label = Some("ConnectedMap_MapEvents".to_string());
        connected_map.blocks = vec![0x00, 0x00];
        data.map_attributes.insert(
            "ConnectedMap".to_string(),
            connected_map.attributes.clone(),
        );
        data.maps
            .insert("ConnectedMap".to_string(), connected_map);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "CUT".to_string(),
            current_pp: 30,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[1] = true;

        let field_move = session
            .use_cut_field_move(&runtime, 0, 0, 0)
            .expect("use cut");

        assert_eq!(field_move.outcome.move_id, "CUT");
        assert_eq!(field_move.outcome.previous_block_id, 0x5b);
        assert_eq!(field_move.outcome.replacement_block_id, 0x3c);
        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x5b));
        assert_eq!(
            session.state.script_runtime.pending_block_field_move,
            Some(field_move.outcome.clone())
        );
        assert!(session.state.map_block_overrides.get("RuntimeMap").is_none());

        runtime
            .data
            .apply_script_runtime_command_in_session(
                &mut session.state,
                &mut session.overworld,
                "RuntimeMap",
                "Script_Cut",
                3,
                ScriptRuntimeInputs::default(),
            )
            .expect("execute source CutDownTreeOrGrass callasm");

        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x3c));
        assert!(session.state.script_runtime.pending_block_field_move.is_none());
        assert_eq!(
            session
                .state
                .map_block_overrides
                .get("RuntimeMap")
                .and_then(|overrides| overrides.get(&(0, 0)))
                .copied(),
            Some(0x3c)
        );
        session
            .state
            .map_block_overrides
            .entry("ConnectedMap".to_string())
            .or_default()
            .insert((1, 0), 0x5b);
        let mut rendered_maps = runtime
            .map_catalog_snapshot(&session.overworld.map, &session.state)
            .into_iter();
        let rendered_map = rendered_maps
            .clone()
            .find(|map| map.map_name == "RuntimeMap")
            .expect("active render map snapshot");
        assert_eq!(rendered_map.blocks, vec![0x3c, 0x00]);
        let connected_map = rendered_maps
            .find(|map| map.map_name == "ConnectedMap")
            .expect("connected render map snapshot");
        assert_eq!(
            connected_map.blocks,
            vec![0x00, 0x5b],
            "saved block overrides must remain visible when a neighboring map is composited"
        );

        let resumed =
            RuntimeOverworldSession::from_state(&runtime, &asset_root, session.state.clone())
                .expect("resume with cut override");
        assert_eq!(resumed.overworld.map.metatile_at(0, 0), Some(0x3c));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_cut_field_move_rejects_missing_badge_without_mutation() {
        let root = temp_repository_root("field-move-cut-no-badge");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "5b": ["FLOOR", "CUT_TREE", "CUT_TREE", "CUT_TREE"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[
                ("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"]),
                ("5b", &["FLOOR", "CUT_TREE", "CUT_TREE", "CUT_TREE"]),
            ]),
        );
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x5b, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "CUT".to_string(),
            current_pp: 30,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        let before = session.state.clone();

        let error = session
            .use_cut_field_move(&runtime, 0, 0, 0)
            .expect_err("missing badge rejects cut");
        let error = error_debug(error);

        assert!(
            error.contains("field move CUT requires johto badge index 1"),
            "{error}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x5b));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_whirlpool_field_move_replaces_block() {
        let root = temp_repository_root("field-move-whirlpool");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "07": ["FLOOR", "WHIRLPOOL", "WHIRLPOOL", "WHIRLPOOL"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[
                ("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"]),
                ("07", &["FLOOR", "WHIRLPOOL", "WHIRLPOOL", "WHIRLPOOL"]),
            ]),
        );
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x07, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        add_runtime_deferred_field_move_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "WHIRLPOOL".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[6] = true;

        let field_move = session
            .use_whirlpool_field_move(&runtime, 0, 0, 0)
            .expect("use whirlpool");

        assert_eq!(field_move.outcome.move_id, "WHIRLPOOL");
        assert_eq!(field_move.outcome.replacement_block_id, 0x36);
        assert_eq!(field_move.outcome.variant, "whirlpool");
        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x07));
        assert_eq!(
            session.state.script_runtime.pending_block_field_move,
            Some(field_move.outcome.clone())
        );

        runtime
            .data
            .apply_script_runtime_command_in_session(
                &mut session.state,
                &mut session.overworld,
                "RuntimeMap",
                "Script_UsedWhirlpool",
                3,
                ScriptRuntimeInputs::default(),
            )
            .expect("execute source DisappearWhirlpool callasm");

        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x36));
        assert!(session.state.script_runtime.pending_block_field_move.is_none());
        assert_eq!(
            session
                .state
                .map_block_overrides
                .get("RuntimeMap")
                .and_then(|overrides| overrides.get(&(0, 0)))
                .copied(),
            Some(0x36)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_strength_and_flash_commit_flags_at_exact_source_callasm_boundaries() {
        let root = temp_repository_root("field-move-flags");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_deferred_field_move_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut strength_user = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        strength_user.moves = vec![LearnedMove {
            name: "STRENGTH".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        let strength_species = strength_user.species.id.clone();
        let mut flash_user = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        flash_user.moves = vec![LearnedMove {
            name: "FLASH".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, strength_user)
            .expect("register strength user");
        session
            .state
            .storage
            .register_capture_in_box(0, flash_user)
            .expect("register flash user");
        session.state.sync_party_from_storage();
        session.state.badges.johto[2] = true;
        session.state.badges.johto[0] = true;

        let strength = session
            .queue_strength_from_menu(&runtime, 0)
            .expect("queue Script_StrengthFromMenu");

        assert_eq!(
            session
                .state
                .flags
                .is_engine_flag_set("ENGINE_STRENGTH_ACTIVE"),
            Ok(false),
            "party-menu dispatch must not run SetStrengthFlag before the source script"
        );
        assert_eq!(strength.next_script, "Script_StrengthFromMenu");

        runtime
            .data
            .apply_script_runtime_command_in_session(
                &mut session.state,
                &mut session.overworld,
                "RuntimeMap",
                "Script_UsedStrength",
                0,
                ScriptRuntimeInputs::default(),
            )
            .expect("execute source SetStrengthFlag callasm");
        assert_eq!(
            session
                .state
                .flags
                .is_engine_flag_set("ENGINE_STRENGTH_ACTIVE"),
            Ok(true)
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .memory
                .get("wStrengthSpecies"),
            Some(&strength_species)
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_1"),
            Some(&strength_species)
        );
        let flash = session
            .use_flash_field_move(&runtime, 1)
            .expect("use flash");

        assert_eq!(flash.outcome.engine_flag, "STATUSFLAGS_FLASH");
        assert_eq!(
            session
                .state
                .flags
                .is_engine_flag_set("ENGINE_STRENGTH_ACTIVE"),
            Ok(true)
        );
        assert_eq!(
            session.state.flags.is_engine_flag_set("STATUSFLAGS_FLASH"),
            Ok(false)
        );
        assert_eq!(
            session.state.script_runtime.pending_flash_field_move,
            Some(flash.outcome.clone())
        );

        runtime
            .data
            .apply_script_runtime_command_in_session(
                &mut session.state,
                &mut session.overworld,
                "RuntimeMap",
                "Script_UseFlash",
                3,
                ScriptRuntimeInputs::default(),
            )
            .expect("execute source BlindingFlash callasm");

        assert_eq!(
            session.state.flags.is_engine_flag_set("STATUSFLAGS_FLASH"),
            Ok(true)
        );
        assert!(session.state.script_runtime.pending_flash_field_move.is_none());
        assert_ne!(strength.state_checksum, flash.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_surf_field_move_enters_water_and_updates_saved_overworld() {
        let root = temp_repository_root("field-move-surf");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": ["FLOOR", "WATER", "FLOOR", "FLOOR"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_deferred_field_move_global_scripts(&mut data);
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "WATER", "FLOOR", "FLOOR"])]),
        );
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x00, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime, 0)
            .expect("runtime game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SURF".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        shell
            .session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session.state.sync_party_from_storage();
        shell.session.state.badges.johto[3] = true;
        shell.session.overworld.player.tile = TilePosition::new(0, 0);
        shell.session.overworld.player.facing = Direction::Right;

        shell.session.overworld.tileset.metatiles[0].collision[0] =
            permissions::RIGHT_WALL;
        assert!(
            shell
                .contextual_surf_direction_is_blocked()
                .expect("evaluate exact CheckDirection mask"),
            "contextual TrySurfOW must not prompt across the current tile's RIGHT_WALL"
        );
        shell.session.overworld.tileset.metatiles[0].collision[0] = permissions::FLOOR;
        assert!(
            !shell
                .contextual_surf_direction_is_blocked()
                .expect("evaluate clear Surf direction"),
            "ordinary floor-to-water Surf direction must remain available"
        );

        shell
            .session
            .state
            .flags
            .set_engine_flag("ENGINE_ALWAYS_ON_BIKE", true)
            .expect("set exact always-on-bike flag");
        shell.session.overworld.player.mode = MovementMode::Bike;
        let before = (shell.session.state.clone(), shell.session.overworld.clone());
        let error = shell
            .use_surf_field_move(0)
            .expect_err("always-on-bike state must reject Surf atomically");
        assert!(
            format!("{error:#}").contains("ENGINE_ALWAYS_ON_BIKE"),
            "{error:#}"
        );
        assert_eq!((shell.session.state.clone(), shell.session.overworld.clone()), before);
        shell
            .session
            .state
            .flags
            .set_engine_flag("ENGINE_ALWAYS_ON_BIKE", false)
            .expect("clear exact always-on-bike flag");
        shell.session.overworld.player.mode = MovementMode::Normal;

        let surf = shell.use_surf_field_move(0).expect("use surf");

        assert_eq!(surf.outcome.from_tile, TilePosition::new(0, 0));
        assert_eq!(surf.outcome.to_tile, TilePosition::new(1, 0));
        assert_eq!(shell.session.overworld.player.mode, MovementMode::Normal);
        assert_eq!(shell.session.overworld.player.tile, TilePosition::new(0, 0));
        assert_eq!(
            shell.session.state.script_runtime.pending_surf_field_move,
            Some(surf.outcome.clone())
        );

        for command_index in [3, 4, 5, 9] {
            shell
                .step_compiled_script_command(
                    "RuntimeMap",
                    "UsedSurfScript",
                    command_index,
                    ScriptRuntimeInputs::default(),
                    ScriptPhoneInputs::default(),
                )
                .unwrap_or_else(|error| {
                    panic!("execute UsedSurfScript command {command_index}: {error:#}")
                });
        }

        assert_eq!(shell.session.overworld.player.mode, MovementMode::Surf);
        assert_eq!(shell.session.overworld.player.tile, TilePosition::new(1, 0));
        assert!(shell.session.state.script_runtime.pending_surf_field_move.is_none());
        assert_eq!(
            shell.session.state.overworld,
            OverworldMemory::from_snapshot(&shell.session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_surf_field_move_rejects_occupied_target_without_moving() {
        let root = temp_repository_root("field-move-surf-occupied");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": ["FLOOR", "WATER", "FLOOR", "FLOOR"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "WATER", "FLOOR", "FLOOR"])]),
        );
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x00, 0x00];
        module.objects = vec![ObjectEvent {
            x: 1,
            y: 0,
            object_identifier: Some("SURF_BLOCKER".to_string()),
            ..runtime_object("SURF_BLOCKER", "-1")
        }];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SURF".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[3] = true;
        session.overworld.player.tile = TilePosition::new(0, 0);
        session.overworld.player.facing = Direction::Right;
        let before = session.state.clone();

        let error = session
            .use_surf_field_move(&runtime, 0)
            .expect_err("occupied target rejects surf");
        let error = error_debug(error);

        assert!(error.contains("occupied tile"));
        assert_eq!(session.state, before);
        assert_eq!(session.overworld.player.mode, MovementMode::Normal);
        assert_eq!(session.overworld.player.tile, TilePosition::new(0, 0));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_waterfall_field_move_climbs_and_updates_saved_overworld() {
        let root = temp_repository_root("field-move-waterfall");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": ["FLOOR", "FLOOR", "FLOOR", "FLOOR"],
  "08": ["WATER", "WATER", "WATER", "WATER"],
  "09": ["WATERFALL", "WATERFALL", "WATERFALL", "WATERFALL"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_deferred_field_move_global_scripts(&mut data);
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[
                ("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"]),
                ("08", &["WATER", "WATER", "WATER", "WATER"]),
                ("09", &["WATERFALL", "WATERFALL", "WATERFALL", "WATERFALL"]),
            ]),
        );
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.attributes.width = 1;
        module.attributes.height = 4;
        module.blocks = vec![0x00, 0x09, 0x09, 0x08];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "WATERFALL".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[7] = true;
        session.overworld.player.tile = TilePosition::new(0, 5);
        session.overworld.player.facing = Direction::Up;
        session.overworld.player.mode = MovementMode::Surf;

        let waterfall = session
            .use_waterfall_field_move(&runtime, 0)
            .expect("use waterfall");

        assert_eq!(
            waterfall.outcome.steps,
            4,
            "ASM and TypeScript move once, then test whether the destination remains WATERFALL"
        );
        assert_eq!(waterfall.outcome.from_tile, TilePosition::new(0, 5));
        assert_eq!(waterfall.outcome.to_tile, TilePosition::new(0, 1));
        assert_eq!(session.overworld.player.mode, MovementMode::Surf);
        assert_eq!(session.overworld.player.tile, TilePosition::new(0, 5));
        assert_eq!(
            session.state.script_runtime.pending_waterfall_field_move,
            Some(waterfall.outcome.clone())
        );

        for step_index in 0..waterfall.outcome.steps {
            runtime
                .data
                .apply_script_movement_in_session(
                    &mut session.state,
                    &mut session.overworld,
                    "RuntimeMap",
                    ".loop@Script_UsedWaterfall",
                    0,
                )
                .unwrap_or_else(|error| panic!("execute Waterfall step {step_index}: {error:#}"));
            runtime
                .data
                .apply_script_runtime_command_in_session(
                    &mut session.state,
                    &mut session.overworld,
                    "RuntimeMap",
                    ".loop@Script_UsedWaterfall",
                    1,
                    ScriptRuntimeInputs::default(),
                )
                .unwrap_or_else(|error| {
                    panic!("execute Waterfall continuation {step_index}: {error:#}")
                });
            let expected_value = if step_index + 1 == waterfall.outcome.steps {
                "1"
            } else {
                "0"
            };
            assert_eq!(
                session.state.script_runtime.script_value.as_deref(),
                Some(expected_value)
            );
        }

        assert_eq!(session.overworld.player.tile, TilePosition::new(0, 1));
        assert!(
            session
                .state
                .script_runtime
                .pending_waterfall_field_move
                .is_none()
        );
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fly_field_move_transitions_to_exact_spawn_and_updates_saved_overworld() {
        let root = temp_repository_root("field-move-fly");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_fly_destination(&mut data);
        data.field_moves.fly.badge = field_move_badge(0);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "FLY".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[0] = true;
        session
            .state
            .flags
            .set_engine_flag("ENGINE_FLYPOINT_NEW_BARK", true)
            .expect("set flypoint flag");
        session.overworld.player.mode = MovementMode::Bike;
        session.state.overworld = OverworldMemory::from_snapshot(&session.overworld.snapshot());
        let source_snapshot = session.overworld.snapshot();

        let fly = session
            .use_fly_field_move(&runtime, &asset_root, 0, 14, "ENGINE_FLYPOINT_NEW_BARK")
            .expect("use fly");

        assert_eq!(fly.actor_party_index, 0);
        assert_eq!(fly.actor_species, "CHIKORITA");
        assert_eq!(fly.flypoint_flag, "ENGINE_FLYPOINT_NEW_BARK");
        assert_eq!(fly.source_map, "RuntimeMap");
        assert_eq!(fly.destination_spawn_identifier, 14);
        assert_eq!(fly.destination_map, "FlyMap");
        assert_eq!(fly.destination_tile, TilePosition::new(1, 1));
        assert_eq!(session.overworld.snapshot(), source_snapshot);
        let pending = session
            .state
            .script_runtime
            .pending_field_travel
            .as_ref()
            .expect("FLY travel remains pending through the departure script");
        assert_eq!(pending.move_id, "FLY");
        assert_eq!(pending.source_map, "RuntimeMap");
        assert_eq!(pending.destination_map, "FlyMap");

        let committed = session
            .commit_pending_field_travel(&runtime)
            .expect("commit FLY at the source warp boundary");
        assert_eq!(committed.move_id, "FLY");
        assert_eq!(session.overworld.map.name, "FlyMap");
        assert_eq!(session.overworld.player.tile, TilePosition::new(1, 1));
        assert_eq!(session.overworld.player.facing, Direction::Down);
        assert_eq!(session.overworld.player.mode, MovementMode::Normal);
        assert_eq!(session.state.last_spawn_identifier, Some(14));
        assert!(session.state.script_runtime.pending_field_travel.is_none());
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fly_field_move_rejects_unset_flypoint_without_transition() {
        let root = temp_repository_root("field-move-fly-unset-flag");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_fly_destination(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "FLY".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[5] = true;
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_fly_field_move(&runtime, &asset_root, 0, 14, "ENGINE_FLYPOINT_NEW_BARK")
            .expect_err("unset flypoint rejects fly");
        let error = error_debug(error);

        assert!(error.contains("destination flag"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fly_field_move_rejects_non_overworld_environment_without_transition() {
        let root = temp_repository_root("field-move-fly-cave");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_fly_destination(&mut data);
        data.runtime_map_metadata.insert(
            "RUNTIME_MAP".to_string(),
            runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "CAVE"),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "FLY".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[5] = true;
        session
            .state
            .flags
            .set_engine_flag("ENGINE_FLYPOINT_NEW_BARK", true)
            .expect("set flypoint flag");
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_fly_field_move(&runtime, &asset_root, 0, 14, "ENGINE_FLYPOINT_NEW_BARK")
            .expect_err("cave rejects fly");
        let error = error_debug(error);

        assert!(error.contains("environment CAVE"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_teleport_field_move_transitions_to_saved_spawn_without_fallback() {
        let root = temp_repository_root("field-move-teleport");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_teleport_destination(&mut data);
        data.field_moves.teleport.move_id = "DIG".to_string();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "DIG".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.last_spawn_identifier = Some(21);
        let source_snapshot = session.overworld.snapshot();

        let teleport = session
            .use_teleport_field_move(&runtime, &asset_root, 0)
            .expect("use teleport");

        assert_eq!(teleport.actor_party_index, 0);
        assert_eq!(teleport.actor_species, "CHIKORITA");
        assert_eq!(teleport.source_map, "RuntimeMap");
        assert_eq!(teleport.destination_spawn_identifier, 21);
        assert_eq!(teleport.destination_map, "TeleportMap");
        assert_eq!(teleport.destination_tile, TilePosition::new(1, 1));
        assert_eq!(session.overworld.snapshot(), source_snapshot);
        let pending = session
            .state
            .script_runtime
            .pending_field_travel
            .as_ref()
            .expect("TELEPORT travel remains pending through the departure script");
        assert_eq!(pending.move_id, "DIG");
        assert_eq!(pending.source_map, "RuntimeMap");
        assert_eq!(pending.destination_map, "TeleportMap");

        let committed = session
            .commit_pending_field_travel(&runtime)
            .expect("commit TELEPORT at the source warp boundary");
        assert_eq!(committed.move_id, "DIG");
        assert_eq!(session.overworld.map.name, "TeleportMap");
        assert_eq!(session.overworld.player.tile, TilePosition::new(1, 1));
        assert_eq!(session.state.last_spawn_identifier, Some(21));
        assert!(session.state.script_runtime.pending_field_travel.is_none());
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_teleport_field_move_rejects_missing_saved_spawn_without_transition() {
        let root = temp_repository_root("field-move-teleport-missing-spawn");
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
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TELEPORT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.last_spawn_identifier = None;
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_teleport_field_move(&runtime, &asset_root, 0)
            .expect_err("missing saved spawn rejects teleport");
        let error = error_debug(error);

        assert!(error.contains("TELEPORT field move has no saved spawn identifier"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_teleport_field_move_rejects_invalid_environment_without_transition() {
        let root = temp_repository_root("field-move-teleport-cave");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_teleport_destination(&mut data);
        data.runtime_map_metadata.insert(
            "RUNTIME_MAP".to_string(),
            runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "CAVE"),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TELEPORT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.last_spawn_identifier = Some(21);
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_teleport_field_move(&runtime, &asset_root, 0)
            .expect_err("cave rejects teleport");
        let error = error_debug(error);

        assert!(error.contains("environment CAVE"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_headbutt_menu_defers_exact_rng_until_tree_mon_encounter_callasm() {
        let root = temp_repository_root("field-move-headbutt");
        write_headbutt_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "FLOOR", "HEADBUTT_TREE", "FLOOR"])]),
        );
        add_runtime_field_encounters(&mut data);
        add_runtime_headbutt_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0).expect("game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        shell
            .session_mut()
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session_mut().state.sync_party_from_storage();
        shell.session_mut().overworld.player.facing = Direction::Down;
        shell.session_mut().divider = RuntimeDividerSource::replay(
            [255, 0, 53, 0]
                .into_iter()
                .chain(std::iter::repeat_n(0, 32)),
        );
        let state_before_dispatch = shell.session().state().clone();
        let retained_before = shell.retained_runtime_commands().len();

        let dispatch = shell
            .queue_headbutt_script(0, true)
            .expect("queue HeadbuttFromMenuScript");

        assert_eq!(dispatch.next_script, "HeadbuttFromMenuScript");
        assert_eq!(shell.session().state().random_state, state_before_dispatch.random_state);
        assert_eq!(shell.session().state().battle, BattleMemory::Inactive);
        assert_eq!(
            shell.session().state().script_runtime.memory.get("wCurPartyMon"),
            Some(&"0".to_string())
        );
        let frame = &shell.retained_runtime_commands()[retained_before];
        let recorded = crystal_assets::decode_runtime_mutation_command_frame(
            frame,
            &state_before_dispatch,
        )
        .expect("decode journaled Headbutt menu dispatch");
        assert_eq!(
            recorded,
            RuntimeMutationCommand::QueueHeadbuttScript(RuntimeHeadbuttScriptCommand {
                party_index: 0,
                from_menu: true,
            })
        );

        let resolved = shell
            .step_compiled_script_command(
                "RuntimeMap",
                "HeadbuttScript",
                4,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute TreeMonEncounter after the tree animation");
        let RuntimeMutationResult::TreeMonEncounterResolved(outcome) = resolved.mutation.result
        else {
            panic!("TreeMonEncounter returned the wrong mutation");
        };
        assert_eq!(outcome.roll.kind, FieldEncounterKind::Headbutt);
        assert_eq!(outcome.roll.target_tile_x, 0);
        assert_eq!(outcome.roll.target_tile_y, 1);
        assert_eq!(outcome.roll.score, Some(0));
        assert_eq!(outcome.roll.chance_roll, 0);
        assert_eq!(outcome.roll.entry_roll, Some(54));
        let encounter = outcome
            .roll
            .resolved
            .as_ref()
            .expect("Headbutt roll resolves the common encounter");
        assert_eq!(encounter.encounter.species, "CHIKORITA");
        assert_eq!(encounter.encounter.level, 12);
        assert_eq!(shell.session().state().battle, BattleMemory::Inactive);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_contextual_headbutt_queues_headbutt_script_without_the_menu_wrapper() {
        let root = temp_repository_root("field-move-contextual-headbutt");
        write_headbutt_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "FLOOR", "HEADBUTT_TREE", "FLOOR"])]),
        );
        add_runtime_field_encounters(&mut data);
        add_runtime_headbutt_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        shell
            .session_mut()
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session_mut().state.sync_party_from_storage();
        shell.session_mut().overworld.player.facing = Direction::Down;
        let state_before_dispatch = shell.session().state().clone();
        let retained_before = shell.retained_runtime_commands().len();

        let dispatch = shell
            .queue_headbutt_script(0, false)
            .expect("queue contextual HeadbuttScript");

        assert_eq!(dispatch.next_script, "HeadbuttScript");
        let recorded = crystal_assets::decode_runtime_mutation_command_frame(
            &shell.retained_runtime_commands()[retained_before],
            &state_before_dispatch,
        )
        .expect("decode journaled contextual Headbutt dispatch");
        assert_eq!(
            recorded,
            RuntimeMutationCommand::QueueHeadbuttScript(RuntimeHeadbuttScriptCommand {
                party_index: 0,
                from_menu: false,
            })
        );
        assert_eq!(
            shell
                .session()
                .state()
                .script_runtime
                .next_script
                .as_ref()
                .map(|location| location.script.as_str()),
            Some("HeadbuttScript")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_rock_smash_party_menu_queues_the_common_script_and_overwrites_last_talked() {
        let root = temp_repository_root("field-move-rock-smash");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_field_encounters(&mut data);
        add_runtime_rock_smash_global_scripts(&mut data);
        let mut rock = runtime_object("RUNTIME_SMASHABLE_ROCK", "-1");
        rock.x = 0;
        rock.y = 1;
        rock.spritemovedata = "SPRITEMOVEDATA_SMASHABLE_ROCK".to_string();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .objects
            .push(rock);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0).expect("game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "ROCK_SMASH".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        shell
            .session_mut()
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session_mut().state.sync_party_from_storage();
        shell.session_mut().state.script_runtime.last_talked_object =
            Some("STALE_OBJECT".to_string());
        shell.session_mut().overworld.last_talked_object_identifier =
            Some("STALE_OBJECT".to_string());
        let replay_base = shell.session().clone();
        let retained_before = shell.retained_runtime_commands().len();

        let dispatch = shell
            .queue_rock_smash_from_menu(0)
            .expect("queue RockSmashFromMenuScript");

        assert_eq!(
            dispatch.next_script,
            "RockSmashFromMenuScript"
        );
        assert_eq!(
            dispatch.last_talked_object.as_deref(),
            Some("RUNTIME_SMASHABLE_ROCK")
        );
        assert_eq!(
            shell
                .session()
                .state()
                .script_runtime
                .last_talked_object
                .as_deref(),
            Some("RUNTIME_SMASHABLE_ROCK")
        );
        assert_eq!(
            shell
                .session()
                .overworld
                .last_talked_object_identifier
                .as_deref(),
            Some("RUNTIME_SMASHABLE_ROCK")
        );
        assert_eq!(
            shell
                .session()
                .state()
                .script_runtime
                .memory
                .get("wCurPartyMon"),
            Some(&"0".to_string())
        );
        assert_eq!(shell.session().state().battle, BattleMemory::Inactive);
        // The common RockSmashScript owns applymovementlasttalked/disappear;
        // menu dispatch itself must not eagerly hide the target.
        assert!(
            shell
                .session()
                .overworld
                .visible_object_at(TilePosition::new(0, 1))
                .is_some()
        );
        let frame = &shell.retained_runtime_commands()[retained_before];
        let recorded = crystal_assets::decode_runtime_mutation_command_frame(
            frame,
            replay_base.state(),
        )
        .expect("decode journaled Rock Smash menu dispatch");
        assert_eq!(
            recorded,
            RuntimeMutationCommand::QueueRockSmashFromMenu(RuntimeFieldPartyCommand {
                party_index: 0,
            })
        );
        let mut remote = replay_base;
        let replayed = remote
            .apply_runtime_command_frame(&runtime, frame)
            .expect("remote applies the same menu dispatch frame");
        assert_eq!(remote.state(), shell.session().state());
        assert_eq!(remote.overworld, shell.session().overworld);
        assert_eq!(replayed.state_checksum, dispatch.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tree_mon_encounter_rejects_missing_field_encounters_without_rng_change() {
        let root = temp_repository_root("field-move-headbutt-missing-table");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "FLOOR", "HEADBUTT_TREE", "FLOOR"])]),
        );
        add_runtime_headbutt_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.overworld.player.facing = Direction::Down;
        session
            .queue_headbutt_script(&runtime, 0, true)
            .expect("menu dispatch succeeds before TreeMonEncounter");
        let before = session.state.clone();
        let command = RuntimeScriptCommandRef::new("RuntimeMap", "HeadbuttScript", 4);
        let mut divider = RuntimeDividerSource::replay([]);

        let error = runtime
            .data
            .resolve_tree_mon_encounter(
                &mut session.state,
                &session.overworld,
                &command,
                &mut divider,
            )
            .expect_err("missing field encounters reject TreeMonEncounter");
        let error = error_debug(error);

        assert!(error.contains("missing field encounters"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tree_mon_encounter_rejects_present_map_missing_table_without_rng_change() {
        let root = temp_repository_root("field-move-headbutt-present-map-missing-table");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "FLOOR", "HEADBUTT_TREE", "FLOOR"])]),
        );
        add_runtime_field_encounters(&mut data);
        add_runtime_headbutt_global_scripts(&mut data);
        data.field_encounters
            .get_mut("RuntimeMap")
            .expect("RuntimeMap field encounters")
            .tables
            .remove(FieldEncounterKind::Headbutt.as_key());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.overworld.player.facing = Direction::Down;
        session
            .queue_headbutt_script(&runtime, 0, true)
            .expect("menu dispatch succeeds before TreeMonEncounter");
        let before = session.state.clone();
        let command = RuntimeScriptCommandRef::new("RuntimeMap", "HeadbuttScript", 4);
        let mut divider = RuntimeDividerSource::replay([]);

        let error = runtime
            .data
            .resolve_tree_mon_encounter(
                &mut session.state,
                &session.overworld,
                &command,
                &mut divider,
            )
            .expect_err("present map missing Headbutt table");
        let error = error_debug(error);

        assert!(
            error.contains("Headbutt field encounter table")
                && error.contains("RuntimeMap")
                && error.contains("missing from the modpack"),
            "{error}"
        );
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tree_mon_encounter_rejects_empty_selected_bucket_without_rng_change() {
        let root = temp_repository_root("field-move-headbutt-empty-rare");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "FLOOR", "HEADBUTT_TREE", "FLOOR"])]),
        );
        add_runtime_field_encounters(&mut data);
        add_runtime_headbutt_global_scripts(&mut data);
        data.field_encounters
            .get_mut("RuntimeMap")
            .expect("RuntimeMap field encounters")
            .table_mut(FieldEncounterKind::Headbutt)
            .expect("headbutt table")
            .rare
            .clear();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.overworld.player.facing = Direction::Down;
        session
            .queue_headbutt_script(&runtime, 0, true)
            .expect("menu dispatch succeeds before TreeMonEncounter");
        let before = session.state.clone();
        let command = RuntimeScriptCommandRef::new("RuntimeMap", "HeadbuttScript", 4);
        let mut divider = RuntimeDividerSource::replay(
            [255, 0, 53, 0]
                .into_iter()
                .chain(std::iter::repeat_n(0, 32)),
        );

        let error = runtime
            .data
            .resolve_tree_mon_encounter(
                &mut session.state,
                &session.overworld,
                &command,
                &mut divider,
            )
            .expect_err("empty selected rare bucket");
        let error = error_debug(error);

        assert!(
            error.contains("Headbutt field encounter table")
                && error.contains("RuntimeMap")
                && error.contains("no entries")
                && error.contains("rare"),
            "{error}"
        );
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_sweet_scent_menu_defers_rng_and_battle_until_exact_script_boundaries() {
        let root = temp_repository_root("field-move-sweet-scent");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_grass_encounter();
        add_runtime_sweet_scent_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SWEET_SCENT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        shell.session_mut()
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session_mut().state.sync_party_from_storage();
        shell.session_mut().state.random_state = crystal_core::random::CrystalRandomState::default();
        shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([
            0, 0, // roaming selector
            0, 0, // slot
            0, 0, // level
            0, 0, // held item
            0, 0, // attack/defense
            0, 0, // speed/special
        ]);

        let random_before = shell.session().state().random_state;
        let dispatch = shell
            .queue_sweet_scent_from_menu(0)
            .expect("queue Sweet Scent script");
        assert_eq!(dispatch.next_script, ".SweetScent@SweetScentFromMenu");
        assert_eq!(shell.session().state().random_state, random_before);
        assert_eq!(shell.session().state().battle, BattleMemory::Inactive);

        let resolved = shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                5,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute exact SweetScentEncounter callasm");
        let RuntimeMutationResult::SweetScentEncounterResolved(outcome) = resolved.mutation.result
        else {
            panic!("SweetScentEncounter returned the wrong mutation");
        };
        let wild_encounter = outcome
            .wild_encounter
            .as_ref()
            .expect("grass Sweet Scent encounter");
        assert_eq!(wild_encounter.map_name, "RuntimeMap");
        assert_eq!(wild_encounter.surface, EncounterSurface::Grass);
        assert_eq!(wild_encounter.threshold, 255);
        assert_eq!(wild_encounter.encounter_roll, 0);
        assert_eq!(wild_encounter.slot_percent_roll, Some(1));
        assert_eq!(wild_encounter.level_roll, None);
        let resolved = wild_encounter
            .resolved
            .clone()
            .expect("resolved");
        assert_eq!(resolved.encounter.species, "CHIKORITA");
        assert_eq!(resolved.level, 14);
        assert_eq!(shell.session().state().battle, BattleMemory::Inactive);
        assert_eq!(shell.session().state().script_runtime.script_value.as_deref(), Some("1"));
        shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                9,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute Sweet Scent randomwildmon");
        let started = shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                10,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute Sweet Scent startbattle");
        let RuntimeMutationResult::ScriptedWildBattleStarted(start) = started.mutation.result else {
            panic!("Sweet Scent startbattle returned the wrong mutation");
        };
        assert_eq!(start.species, "CHIKORITA");
        assert_eq!(start.level, 14);
        assert!(matches!(
            shell.session().state().battle,
            BattleMemory::StaticWild { .. }
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_sweet_scent_exact_callasm_returns_false_on_missing_surface_without_rng() {
        let root = temp_repository_root("field-move-sweet-scent-missing-surface");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_grass_encounter();
        add_runtime_sweet_scent_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SWEET_SCENT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        shell.session_mut()
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session_mut().state.sync_party_from_storage();
        for metatile in &mut shell.session_mut().overworld.tileset.metatiles {
            metatile.collision = [
                crystal_core::world::collision::permissions::FLOOR;
                4
            ];
        }
        shell.session_mut().divider = crystal_core::random::RuntimeDividerSource::replay([]);
        let before = shell.session().state().clone();
        shell.queue_sweet_scent_from_menu(0).expect("queue Sweet Scent script");
        let resolved = shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                5,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("Sweet Scent on a non-encounter tile resolves false");
        let RuntimeMutationResult::SweetScentEncounterResolved(outcome) = resolved.mutation.result
        else {
            panic!("SweetScentEncounter returned the wrong mutation");
        };
        assert_eq!(outcome.wild_encounter, None);
        assert_eq!(shell.session().state().random_state, before.random_state);
        assert_eq!(shell.session().state().script_runtime.script_value.as_deref(), Some("0"));
        assert_eq!(shell.session().state().battle, BattleMemory::Inactive);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_sweet_scent_preserves_roaming_battle_identity_through_startbattle() {
        let root = temp_repository_root("field-move-sweet-scent-roaming");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_grass_encounter();
        add_runtime_sweet_scent_global_scripts(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SWEET_SCENT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        shell
            .session_mut()
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        shell.session_mut().state.sync_party_from_storage();
        shell.session_mut().state.roaming_pokemon[0] =
            crystal_core::state::RoamingPokemonState {
                species: Some("CHIKORITA".to_string()),
                level: 40,
                map_group: 1,
                map_number: 1,
                hp: 1,
                dvs_be: [0, 0],
            };
        shell.session_mut().state.random_state =
            crystal_core::random::CrystalRandomState { add: 0xff, sub: 0 };
        shell.session_mut().divider = RuntimeDividerSource::replay(
            [0, 255]
                .into_iter()
                .chain(std::iter::repeat_n(0, 32)),
        );

        shell
            .queue_sweet_scent_from_menu(0)
            .expect("queue Sweet Scent script");
        let resolved = shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                5,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute SweetScentEncounter");
        let RuntimeMutationResult::SweetScentEncounterResolved(outcome) = resolved.mutation.result
        else {
            panic!("SweetScentEncounter returned the wrong mutation");
        };
        assert_eq!(
            outcome
                .wild_encounter
                .as_ref()
                .expect("roaming encounter roll")
                .roaming_slot,
            Some(0)
        );
        shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                9,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute randomwildmon");
        shell
            .step_compiled_script_command(
                "RuntimeMap",
                ".SweetScent@SweetScentFromMenu",
                10,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("execute startbattle");

        assert!(matches!(
            &shell.session().state().battle,
            BattleMemory::StaticWild {
                battle_type,
                battle_music,
                roaming_slot: Some(0),
                ..
            } if battle_type == "BATTLETYPE_ROAMING"
                && battle_music == "MUSIC_SUICUNE_BATTLE"
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tm_rejects_cannot_learn_without_consumption() {
        let root = temp_repository_root("field-item-tm-cannot-learn");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("HEADBUTT".to_string(), runtime_move_named("HEADBUTT", 15));
        let mut tm = runtime_item("TM_HEADBUTT", item_pocket("TM_HM"));
        tm.field_menu = "ITEMMENU_PARTY".to_string();
        tm.field_usable = true;
        tm.consumable = true;
        tm.tmhm_index = Some(1);
        tm.tmhm_move = Some("HEADBUTT".to_string());
        data.items.insert("TM_HEADBUTT".to_string(), tm);
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        session
            .state
            .storage
            .register_capture_in_box(0, Pokemon::new_for_tests(runtime_species(), 8, Dv::default()))
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["TM_HEADBUTT"], 1)
            .expect("add TM");
        let before = session.state.clone();

        let error = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "TM_HEADBUTT", 0, None)
            .expect_err("cannot learn");

        let error = error_debug(error);
        assert!(
            error.contains("species 'CHIKORITA' cannot learn 'HEADBUTT' from TM/HM"),
            "{error}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["TM_HEADBUTT"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tm_replaces_selected_full_move_slot() {
        let root = temp_repository_root("field-item-tm-replace");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        for (name, pp) in [
            ("HEADBUTT", 15),
            ("GROWL", 40),
            ("TAIL_WHIP", 30),
            ("LEER", 30),
        ] {
            data.moves
                .insert(name.to_string(), runtime_move_named(name, pp));
        }
        let mut tm = runtime_item("TM_HEADBUTT", item_pocket("TM_HM"));
        tm.field_menu = "ITEMMENU_PARTY".to_string();
        tm.field_usable = true;
        tm.consumable = true;
        tm.tmhm_index = Some(1);
        tm.tmhm_move = Some("HEADBUTT".to_string());
        data.items.insert("TM_HEADBUTT".to_string(), tm);
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.species.tmhm_learnset = vec!["HEADBUTT".to_string()];
        player.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 40,
                pp_ups: 0,
            },
            LearnedMove {
                name: "TAIL_WHIP".to_string(),
                current_pp: 30,
                pp_ups: 0,
            },
            LearnedMove {
                name: "LEER".to_string(),
                current_pp: 30,
                pp_ups: 0,
            },
        ];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["TM_HEADBUTT"], 1)
            .expect("add TM");

        let item_use = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "TM_HEADBUTT", 0, Some(2))
            .expect("replace move");

        assert_eq!(item_use.learned_move.replaced_slot, Some(2));
        assert_eq!(
            item_use.learned_move.replaced_move.as_deref(),
            Some("TAIL_WHIP")
        );
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .moves[2]
                .name,
            "HEADBUTT"
        );
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["TM_HEADBUTT"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_pp_up_raises_selected_move_pp_stage() {
        let root = temp_repository_root("field-item-pp-up");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut pp_up = runtime_item("PP_UP", item_pocket("ITEM"));
        pp_up.effect = "MOD_PP_UP".to_string();
        pp_up.pp_up_stages = Some(1);
        pp_up.field_menu = "ITEMMENU_PARTY".to_string();
        pp_up.field_usable = true;
        pp_up.battle_menu = "ITEMMENU_NOUSE".to_string();
        pp_up.battle_usable = false;
        pp_up.consumable = true;
        data.items.insert("PP_UP".to_string(), pp_up);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PP_UP"], 1)
            .expect("add PP Up");

        let item_use = session
            .use_bag_item_on_party_move(&runtime, "PP_UP", 0, Some(0))
            .expect("use PP Up");

        assert_eq!(item_use.item_use.item_id, "PP_UP");
        assert_eq!(item_use.item_effect.pp_changes[0].pp_before, 20);
        assert_eq!(item_use.item_effect.pp_changes[0].pp_after, 27);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.moves[0].pp_ups, 1);
        assert_eq!(pokemon.moves[0].current_pp, 27);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["PP_UP"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_pp_up_rejects_maxed_move_without_consumption() {
        let root = temp_repository_root("field-item-pp-up-maxed");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut pp_up = runtime_item("PP_UP", item_pocket("ITEM"));
        pp_up.effect = "MOD_PP_UP".to_string();
        pp_up.pp_up_stages = Some(1);
        pp_up.field_menu = "ITEMMENU_PARTY".to_string();
        pp_up.field_usable = true;
        pp_up.battle_menu = "ITEMMENU_NOUSE".to_string();
        pp_up.battle_usable = false;
        pp_up.consumable = true;
        data.items.insert("PP_UP".to_string(), pp_up);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 56,
            pp_ups: 3,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PP_UP"], 1)
            .expect("add PP Up");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_move(&runtime, "PP_UP", 0, Some(0))
            .expect_err("maxed move rejects PP Up");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["PP_UP"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_restores_selected_move_pp_from_compiled_moves() {
        let root = temp_repository_root("battle-item-ether");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("GROWL".to_string(), runtime_move_named("GROWL", 40));
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        sync_runtime_move_tables(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 20,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
        ];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_battle_party_move(&runtime, "ETHER", 0, Some(0))
            .expect("use ether");

        assert_eq!(item_use.item_use.item_id, "ETHER");
        assert_eq!(item_use.battle_item.pp_changes.len(), 1);
        assert_eq!(item_use.battle_item.pp_changes[0].move_id, "TACKLE");
        assert_eq!(item_use.battle_item.pp_changes[0].pp_before, 20);
        assert_eq!(item_use.battle_item.pp_changes[0].pp_after, 30);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.moves[0].current_pp, 30);
        assert_eq!(pokemon.moves[1].current_pp, 1);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_rejects_full_pp_without_consumption() {
        let root = temp_repository_root("battle-item-ether-full");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture_in_box(0, player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_battle_party_move(&runtime, "ETHER", 0, Some(0))
            .expect_err("full PP has no target change");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }
