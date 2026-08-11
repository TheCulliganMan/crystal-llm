    #[test]
    fn runtime_battle_item_rejects_missing_payload_without_consumption() {
        let root = temp_repository_root("battle-item-unsupported");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut bad_potion = runtime_item("BAD_POTION", item_pocket("ITEM"));
        bad_potion.effect = "MOD_UNDECLARED".to_string();
        bad_potion.parameter = 0;
        bad_potion.field_menu = "ITEMMENU_PARTY".to_string();
        bad_potion.field_usable = true;
        bad_potion.battle_menu = "ITEMMENU_PARTY".to_string();
        bad_potion.battle_usable = true;
        bad_potion.consumable = true;
        data.items.insert("BAD_POTION".to_string(), bad_potion);
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
            .add_item(&runtime.data.items["BAD_POTION"], 1)
            .expect("add item");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "BAD_POTION")
            .expect_err("payload-less battle item rejected");

        assert!(
            format!("{error:?}").contains("declares no battle item payload"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["BAD_POTION"]),
            1
        );
        assert!(session.state.script_runtime.item_use_events.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_rejects_full_hp_without_consumption() {
        let root = temp_repository_root("battle-item-full-hp");
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
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "POTION")
            .expect_err("full HP target has no effect");

        assert!(
            format!("{error:?}").contains("would not change the target"),
            "{error:?}"
        );
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert!(session.state.script_runtime.item_use_events.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_script_item_economy_and_shop_commands() {
        let root = temp_repository_root("script-items-economy-shop");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.price = 300;
        let mut poke_ball = runtime_ball_item("POKE_BALL");
        poke_ball.price = 200;
        data.items.insert("POTION".to_string(), potion);
        data.items.insert("POKE_BALL".to_string(), poke_ball);
        data.marts.0.insert(
            "RuntimeMart".to_string(),
            vec!["POTION".to_string(), "POKE_BALL".to_string()],
        );
        data.currency_constants
            .0
            .insert("RUNTIME_PRICE".to_string(), 500);
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.script_item_grants
            .push(crystal_core::systems::script_items::ScriptItemGrant {
                command: "verbosegiveitem".to_string(),
                item_id: "POTION".to_string(),
                quantity: 2,
                source_script: "RuntimeItemScript".to_string(),
                command_index: 1,
                verbose: true,
            });
        map.script_item_checks
            .push(crystal_core::systems::script_items::ScriptItemAccess {
                command: "checkitem".to_string(),
                item_id: "POTION".to_string(),
                source_script: "RuntimeItemScript".to_string(),
                command_index: 2,
            });
        map.script_item_takes
            .push(crystal_core::systems::script_items::ScriptItemAccess {
                command: "takeitem".to_string(),
                item_id: "POTION".to_string(),
                source_script: "RuntimeItemScript".to_string(),
                command_index: 3,
            });
        map.script_economy_commands
            .push(crystal_core::systems::economy::ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["RUNTIME_PRICE".to_string()],
                source_script: "RuntimeEconomyScript".to_string(),
                command_index: 4,
            });
        map.script_economy_commands
            .push(crystal_core::systems::economy::ScriptEconomyCommand {
                command: "takemoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["RUNTIME_PRICE".to_string()],
                source_script: "RuntimeEconomyScript".to_string(),
                command_index: 5,
            });
        map.script_shop_commands
            .push(crystal_core::systems::shop::ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "RuntimeMart".to_string(),
                source_script: "RuntimeShopScript".to_string(),
                command_index: 6,
            });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session.state.money = 1_000;
        let shop_command_row = RuntimeScriptShopCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "pokemart".to_string(),
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "RuntimeMart".to_string(),
            source_script: "RuntimeShopScript".to_string(),
            command_index: 6,
        };
        let wrong_shop_command_row = RuntimeScriptShopCommandKey {
            mart_id: "runtimemart".to_string(),
            ..shop_command_row.clone()
        };
        assert!(runtime.has_script_shop_command(&shop_command_row));
        assert!(!runtime.has_script_shop_command(&wrong_shop_command_row));
        assert!(
            runtime
                .script_shop_command_keys()
                .contains(&shop_command_row)
        );
        assert!(
            runtime
                .require_script_shop_command(&shop_command_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_shop_command(&wrong_shop_command_row)
                .is_err()
        );
        let item_grant_row = RuntimeScriptItemGrantKey {
            map_name: "RuntimeMap".to_string(),
            command: "verbosegiveitem".to_string(),
            item_id: "POTION".to_string(),
            quantity: 2,
            source_script: "RuntimeItemScript".to_string(),
            command_index: 1,
            verbose: true,
        };
        let wrong_item_grant_row = RuntimeScriptItemGrantKey {
            item_id: "potion".to_string(),
            ..item_grant_row.clone()
        };
        assert!(runtime.has_script_item_grant(&item_grant_row));
        assert!(!runtime.has_script_item_grant(&wrong_item_grant_row));
        assert!(runtime.script_item_grant_keys().contains(&item_grant_row));
        assert!(runtime.require_script_item_grant(&item_grant_row).is_ok());
        assert!(
            runtime
                .require_script_item_grant(&wrong_item_grant_row)
                .is_err()
        );
        let item_check_row = RuntimeScriptItemAccessKey {
            map_name: "RuntimeMap".to_string(),
            command: "checkitem".to_string(),
            item_id: "POTION".to_string(),
            source_script: "RuntimeItemScript".to_string(),
            command_index: 2,
        };
        let item_take_row = RuntimeScriptItemAccessKey {
            command: "takeitem".to_string(),
            command_index: 3,
            ..item_check_row.clone()
        };
        let wrong_item_access_row = RuntimeScriptItemAccessKey {
            command: "CheckItem".to_string(),
            ..item_check_row.clone()
        };
        assert!(runtime.has_script_item_access(&item_check_row));
        assert!(runtime.has_script_item_access(&item_take_row));
        assert!(!runtime.has_script_item_access(&wrong_item_access_row));
        assert!(runtime.script_item_access_keys().contains(&item_check_row));
        assert!(runtime.script_item_access_keys().contains(&item_take_row));
        assert!(runtime.require_script_item_access(&item_check_row).is_ok());
        assert!(
            runtime
                .require_script_item_access(&wrong_item_access_row)
                .is_err()
        );
        let economy_command_row = RuntimeScriptEconomyCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "checkmoney".to_string(),
            account: Some("YOUR_MONEY".to_string()),
            amount_tokens: vec!["RUNTIME_PRICE".to_string()],
            source_script: "RuntimeEconomyScript".to_string(),
            command_index: 4,
        };
        let wrong_economy_command_row = RuntimeScriptEconomyCommandKey {
            amount_tokens: vec!["runtime_price".to_string()],
            ..economy_command_row.clone()
        };
        assert!(runtime.has_script_economy_command(&economy_command_row));
        assert!(!runtime.has_script_economy_command(&wrong_economy_command_row));
        assert!(
            runtime
                .script_economy_command_keys()
                .contains(&economy_command_row)
        );
        assert!(
            runtime
                .require_script_economy_command(&economy_command_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_economy_command(&wrong_economy_command_row)
                .is_err()
        );

        let grant = session
            .grant_script_item(&runtime, "RuntimeMap", "RuntimeItemScript", 1)
            .expect("grant script item");
        let check = session
            .check_script_item(&runtime, "RuntimeMap", "RuntimeItemScript", 2)
            .expect("check script item");
        let take = session
            .take_script_item(&runtime, "RuntimeMap", "RuntimeItemScript", 3)
            .expect("take script item");
        let money_check = session
            .apply_script_economy_command(&runtime, "RuntimeMap", "RuntimeEconomyScript", 4)
            .expect("check money");
        let money_take = session
            .apply_script_economy_command(&runtime, "RuntimeMap", "RuntimeEconomyScript", 5)
            .expect("take money");
        let shop = session
            .open_script_shop(&runtime, "RuntimeMap", "RuntimeShopScript", 6)
            .expect("open shop");
        let buy = session
            .buy_shop_item(&runtime, "POKE_BALL", 1)
            .expect("buy ball");

        assert!(matches!(
            grant.outcome,
            ScriptItemGrantOutcome::Granted { .. }
        ));
        assert!(check.outcome.held);
        assert!(take.outcome.removed);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert!(matches!(
            money_check.outcome,
            ScriptEconomyOutcome::Check {
                script_value,
                ..
            } if script_value == "0"
        ));
        assert!(matches!(
            money_take.outcome,
            ScriptEconomyOutcome::MoneyChanged { balance: 500, .. }
        ));
        assert_eq!(shop.outcome.inventory, vec!["POTION", "POKE_BALL"]);
        assert!(buy.outcome.success);
        assert_eq!(session.state.money, 300);
        let sell = session
            .sell_shop_item(&runtime, "POKE_BALL", 1)
            .expect("sell ball");
        assert!(sell.outcome.success);
        assert_eq!(session.state.money, 400);
        assert_ne!(grant.state_checksum, sell.state_checksum);

        let wrong_index = session
            .open_script_shop(&runtime, "RuntimeMap", "RuntimeShopScript", 7)
            .expect_err("script shop command indexes are exact");
        assert!(
            format!("{wrong_index:#}")
                .contains("has no script shop command at RuntimeShopScript:7")
        );
        let wrong_item = session
            .buy_shop_item(&runtime, "poke_ball", 1)
            .expect_err("active shop item ids are exact");
        let wrong_item = error_debug(wrong_item);
        assert!(wrong_item.contains("poke_ball"), "{wrong_item}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_field_pickups_and_phone_commands() {
        let root = temp_repository_root("field-pickups-phone");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        data.items.insert(
            "BERRY".to_string(),
            runtime_item("BERRY", item_pocket("ITEM")),
        );
        data.fruit_trees
            .0
            .insert("FRUITTREE_RUNTIME".to_string(), "BERRY".to_string());
        data.phone_contacts.0.insert(
            "PHONE_MOM".to_string(),
            crystal_core::systems::phone::PhoneContactRecord {
                contact_id: "PHONE_MOM".to_string(),
                trainer_class: None,
                trainer_label: None,
                lines: vec!["Mom".to_string()],
                primary_label: "MomPhoneScript".to_string(),
                map_constant: None,
                callee_time_mask: 0xff,
                callee_script: None,
                caller_time_mask: 0xff,
                caller_script: None,
            },
        );
        data.phone_contacts.0.insert(
            "PHONE_JOEY".to_string(),
            crystal_core::systems::phone::PhoneContactRecord {
                contact_id: "PHONE_JOEY".to_string(),
                trainer_class: Some("YOUNGSTER".to_string()),
                trainer_label: Some("JOEY".to_string()),
                lines: vec!["Top percentage.".to_string()],
                primary_label: "JoeyPhoneScript".to_string(),
                map_constant: Some("RUNTIME_MAP".to_string()),
                callee_time_mask: 0xff,
                callee_script: None,
                caller_time_mask: 0xff,
                caller_script: None,
            },
        );
        data.permanent_phone_numbers = [(
            "PHONE_MOM".to_string(),
            crystal_core::systems::phone::PermanentPhoneNumberRule::default(),
        )]
        .into_iter()
        .collect();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.objects.push(ObjectEvent {
            sprite: "SPRITE_BALL_CUT_FRUIT".to_string(),
            x: 1,
            y: 0,
            spritemovedata: "SPRITEMOVEDATA_STILL".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_ITEMBALL".to_string(),
            radius: 0,
            script: "RuntimeItemBallScript".to_string(),
            label: None,
            event_flag: "EVENT_RUNTIME_POTION".to_string(),
            object_identifier: Some("RUNTIME_POTION_BALL".to_string()),
            sightline_direction_override: None,
        });
        map.script_field_pickups
            .push(crystal_core::systems::field_items::ScriptFieldPickup {
                command: "itemball".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_RUNTIME_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "RuntimeItemBallScript".to_string(),
                command_index: 0,
            });
        map.script_field_pickups
            .push(crystal_core::systems::field_items::ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some("FRUITTREE_RUNTIME".to_string()),
                source_script: "RuntimeFruitTreeScript".to_string(),
                command_index: 1,
            });
        map.script_phone_commands
            .push(crystal_core::systems::phone::ScriptPhoneCommand {
                command: "checkcellnum".to_string(),
                contact_id: "PHONE_JOEY".to_string(),
                source_script: "RuntimePhoneScript".to_string(),
                command_index: 2,
            });
        map.script_phone_commands
            .push(crystal_core::systems::phone::ScriptPhoneCommand {
                command: "askforphonenumber".to_string(),
                contact_id: "PHONE_JOEY".to_string(),
                source_script: "RuntimePhoneScript".to_string(),
                command_index: 3,
            });
        map.scripts.insert(
            "RuntimeItemBallScript".to_string(),
            serde_json::json!([
                {"command": "itemball", "args": ["POTION"]}
            ]),
        );
        map.scripts.insert(
            "RuntimeFruitTreeScript".to_string(),
            serde_json::json!([
                {"command": "fruittree", "args": ["FRUITTREE_RUNTIME"]}
            ]),
        );
        map.scripts.insert(
            "RuntimePhoneScript".to_string(),
            serde_json::json!([
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimePhoneText"]},
                {"command": "checkcellnum", "args": ["PHONE_JOEY"]},
                {"command": "askforphonenumber", "args": ["PHONE_JOEY"]}
            ]),
        );
        map.script_text_bodies.insert(
            "RuntimePhoneText".to_string(),
            ScriptTextBody {
                label: "RuntimePhoneText".to_string(),
                commands: Vec::new(),
            },
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
        let field_pickup_row = RuntimeScriptFieldPickupKey {
            map_name: "RuntimeMap".to_string(),
            command: "itemball".to_string(),
            item_id: Some("POTION".to_string()),
            quantity: 1,
            event_flag: Some("EVENT_RUNTIME_POTION".to_string()),
            fruit_tree_id: None,
            source_script: "RuntimeItemBallScript".to_string(),
            command_index: 0,
        };
        let wrong_field_pickup_row = RuntimeScriptFieldPickupKey {
            event_flag: Some("event_runtime_potion".to_string()),
            ..field_pickup_row.clone()
        };
        assert!(runtime.has_script_field_pickup(&field_pickup_row));
        assert!(!runtime.has_script_field_pickup(&wrong_field_pickup_row));
        assert!(
            runtime
                .script_field_pickup_keys()
                .contains(&field_pickup_row)
        );
        assert!(
            runtime
                .require_script_field_pickup(&field_pickup_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_field_pickup(&wrong_field_pickup_row)
                .is_err()
        );
        let phone_command_row = RuntimeScriptPhoneCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "checkcellnum".to_string(),
            contact_id: "PHONE_JOEY".to_string(),
            source_script: "RuntimePhoneScript".to_string(),
            command_index: 2,
        };
        let wrong_phone_command_row = RuntimeScriptPhoneCommandKey {
            contact_id: "phone_joey".to_string(),
            ..phone_command_row.clone()
        };
        assert!(runtime.has_script_phone_command(&phone_command_row));
        assert!(!runtime.has_script_phone_command(&wrong_phone_command_row));
        assert!(
            runtime
                .script_phone_command_keys()
                .contains(&phone_command_row)
        );
        assert!(
            runtime
                .require_script_phone_command(&phone_command_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_phone_command(&wrong_phone_command_row)
                .is_err()
        );
        assert!(
            session
                .overworld
                .visible_object_at(TilePosition::new(1, 0))
                .is_some()
        );

        let permanent = session
            .initialize_permanent_phone_numbers(&runtime)
            .expect("permanent phones initialize");
        let check_before = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                2,
                ScriptPhoneInputs::default(),
            )
            .expect("check phone before registration");
        let mut dispatch_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
            .expect("dispatch phone game shell");
        let dispatched_phone = dispatch_shell
            .apply_compiled_script_command(
                "RuntimeMap",
                "RuntimePhoneScript",
                2,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs { accepted: None },
            )
            .expect("compiled phone command dispatch");
        let RuntimeMutationResult::ScriptPhoneApplied(dispatched_phone_outcome) =
            dispatched_phone.result
        else {
            panic!("compiled phone command must dispatch as script phone");
        };
        assert!(matches!(
            dispatched_phone_outcome,
            ScriptPhoneOutcome::CheckCellNum {
                registered: false,
                script_value,
                ..
            } if script_value == "0"
        ));
        let mut prompt_shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
            .expect("phone prompt game shell");
        let prompted = prompt_shell
            .resolve_phone_prompt_and_run_compiled_script(
                "RuntimePhoneScript",
                3,
                ScriptRuntimeInputs::default(),
                true,
                4,
            )
            .expect("compiled phone prompt resolves and runs");
        assert!(matches!(
            prompted.step.mutation.result,
            RuntimeMutationResult::ScriptPhoneApplied(ScriptPhoneOutcome::AskForPhoneNumber {
                result: crystal_core::systems::phone::PhoneRegistrationResult::Registered,
                script_value,
                ..
            }) if script_value == "0"
        ));
        assert!(prompted.run.steps.is_empty());
        assert_eq!(prompted.run.next_cursor, None);
        let ask = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                3,
                ScriptPhoneInputs {
                    accepted: Some(true),
                },
            )
            .expect("register phone");
        let check_after = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                2,
                ScriptPhoneInputs::default(),
            )
            .expect("check phone after registration");
        let pickup = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "RuntimeItemBallScript", 0)
            .expect("pickup itemball");
        let fruit = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "RuntimeFruitTreeScript", 1)
            .expect("pickup fruit");

        assert_eq!(permanent.inserted, vec!["PHONE_MOM".to_string()]);
        assert!(matches!(
            check_before.outcome,
            ScriptPhoneOutcome::CheckCellNum {
                registered: false,
                script_value,
                ..
            } if script_value == "0"
        ));
        assert!(matches!(
            ask.outcome,
            ScriptPhoneOutcome::AskForPhoneNumber {
                result: crystal_core::systems::phone::PhoneRegistrationResult::Registered,
                script_value,
                ..
            } if script_value == "0"
        ));
        assert!(matches!(
            check_after.outcome,
            ScriptPhoneOutcome::CheckCellNum {
                registered: true,
                script_value,
                ..
            } if script_value == "1"
        ));
        assert!(matches!(
            pickup.outcome,
            FieldItemPickupOutcome::Collected {
                item_id,
                event_flag,
                ..
            } if item_id == "POTION" && event_flag == "EVENT_RUNTIME_POTION"
        ));
        assert!(
            session
                .overworld
                .visible_object_at(TilePosition::new(1, 0))
                .is_none()
        );
        assert!(matches!(
            fruit.outcome,
            FieldItemPickupOutcome::Collected {
                item_id,
                event_flag,
                ..
            } if item_id == "BERRY" && event_flag == "FRUITTREE_RUNTIME_COLLECTED"
        ));
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["BERRY"]), 1);
        assert_ne!(permanent.state_checksum, fruit.state_checksum);

        let wrong_phone = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                4,
                ScriptPhoneInputs::default(),
            )
            .expect_err("phone command indexes are exact");
        assert!(
            format!("{wrong_phone:#}")
                .contains("has no script phone command at RuntimePhoneScript:4")
        );
        let wrong_fruit = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "runtimefruittreescript", 1)
            .expect_err("field pickup script ids are exact");
        assert!(
            format!("{wrong_fruit:#}")
                .contains("has no script field pickup at runtimefruittreescript:1")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_flags_scenes_and_block_changes() {
        let root = temp_repository_root("flags-scenes-blocks");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.scenes = MapSceneTable {
            scenes: vec![
                MapScene {
                    scene_id: "SCENE_RUNTIME_START".to_string(),
                    script_name: Some("RuntimeStartScene".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_RUNTIME_DONE".to_string(),
                    script_name: None,
                },
            ],
        };
        map.objects.push(ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x: 1,
            y: 0,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "RuntimeFlagScript".to_string(),
            label: None,
            event_flag: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
            object_identifier: Some("RUNTIME_HIDE_OBJECT".to_string()),
            sightline_direction_override: None,
        });
        map.script_flag_commands
            .push(crystal_core::systems::script_flags::ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
                source_script: "RuntimeFlagScript".to_string(),
                command_index: 0,
            });
        map.script_flag_commands
            .push(crystal_core::systems::script_flags::ScriptFlagCommand {
                command: "checkevent".to_string(),
                flag_id: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
                source_script: "RuntimeFlagScript".to_string(),
                command_index: 1,
            });
        map.script_scene_commands
            .push(crystal_core::systems::script_scenes::ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some("SCENE_RUNTIME_DONE".to_string()),
                source_script: "RuntimeSceneScript".to_string(),
                command_index: 2,
            });
        map.script_scene_commands
            .push(crystal_core::systems::script_scenes::ScriptSceneCommand {
                command: "checkscene".to_string(),
                map_id: None,
                scene_id: None,
                source_script: "RuntimeSceneScript".to_string(),
                command_index: 3,
            });
        map.script_scene_commands
            .push(crystal_core::systems::script_scenes::ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("RUNTIME_MAP".to_string()),
                scene_id: Some("SCENE_RUNTIME_START".to_string()),
                source_script: "RuntimeSceneScript".to_string(),
                command_index: 4,
            });
        map.script_block_changes
            .push(crystal_core::systems::script_blocks::ScriptBlockChange {
                x: 2,
                y: 0,
                block_id: 7,
                source_script: "RuntimeBlockScript".to_string(),
                command_index: 5,
            });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let flag_command_row = RuntimeScriptFlagCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "setevent".to_string(),
            flag_id: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
            source_script: "RuntimeFlagScript".to_string(),
            command_index: 0,
        };
        let wrong_flag_command_row = RuntimeScriptFlagCommandKey {
            flag_id: "event_runtime_hide_object".to_string(),
            ..flag_command_row.clone()
        };
        assert!(runtime.has_script_flag_command(&flag_command_row));
        assert!(!runtime.has_script_flag_command(&wrong_flag_command_row));
        assert!(
            runtime
                .script_flag_command_keys()
                .contains(&flag_command_row)
        );
        assert!(
            runtime
                .require_script_flag_command(&flag_command_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_flag_command(&wrong_flag_command_row)
                .is_err()
        );
        let scene_command_row = RuntimeScriptSceneCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "setscene".to_string(),
            map_id: None,
            scene_id: Some("SCENE_RUNTIME_DONE".to_string()),
            source_script: "RuntimeSceneScript".to_string(),
            command_index: 2,
        };
        let wrong_scene_command_row = RuntimeScriptSceneCommandKey {
            scene_id: Some("scene_runtime_done".to_string()),
            ..scene_command_row.clone()
        };
        assert!(runtime.has_script_scene_command(&scene_command_row));
        assert!(!runtime.has_script_scene_command(&wrong_scene_command_row));
        assert!(
            runtime
                .script_scene_command_keys()
                .contains(&scene_command_row)
        );
        assert!(
            runtime
                .require_script_scene_command(&scene_command_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_scene_command(&wrong_scene_command_row)
                .is_err()
        );
        let block_change_row = RuntimeScriptBlockChangeKey {
            map_name: "RuntimeMap".to_string(),
            x: 2,
            y: 0,
            block_id: 7,
            source_script: "RuntimeBlockScript".to_string(),
            command_index: 5,
        };
        let wrong_block_change_row = RuntimeScriptBlockChangeKey {
            block_id: 8,
            ..block_change_row.clone()
        };
        assert!(runtime.has_script_block_change(&block_change_row));
        assert!(!runtime.has_script_block_change(&wrong_block_change_row));
        assert!(
            runtime
                .script_block_change_keys()
                .contains(&block_change_row)
        );
        assert!(
            runtime
                .require_script_block_change(&block_change_row)
                .is_ok()
        );
        assert!(
            runtime
                .require_script_block_change(&wrong_block_change_row)
                .is_err()
        );
        assert!(
            session
                .overworld
                .visible_object_at(TilePosition::new(1, 0))
                .is_some()
        );

        let flag = session
            .apply_script_flag_mutation(&runtime, "RuntimeMap", "RuntimeFlagScript", 0)
            .expect("set flag");
        let check_flag = session
            .check_script_flag(&runtime, "RuntimeMap", "RuntimeFlagScript", 1)
            .expect("check flag");
        let set_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 2)
            .expect("set scene");
        let check_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 3)
            .expect("check scene");
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("1")
        );
        let set_map_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 4)
            .expect("set map scene");
        let block = session
            .apply_script_block_change(&runtime, "RuntimeMap", "RuntimeBlockScript", 5)
            .expect("change block");

        assert!(flag.outcome.value);
        assert!(check_flag.outcome.set);
        assert!(
            session
                .overworld
                .visible_object_at(TilePosition::new(1, 0))
                .is_none()
        );
        assert_eq!(set_scene.outcome.scene_id, "SCENE_RUNTIME_DONE");
        assert_eq!(check_scene.outcome.scene_id, "SCENE_RUNTIME_DONE");
        assert_eq!(set_map_scene.outcome.scene_id, "SCENE_RUNTIME_START");
        assert_eq!(session.overworld.map.metatile_at(1, 0), Some(7));
        assert_eq!(
            session
                .state
                .map_block_overrides
                .get("RuntimeMap")
                .and_then(|overrides| overrides.get(&(1, 0)))
                .copied(),
            Some(7)
        );
        assert_ne!(flag.state_checksum, block.state_checksum);

        let saved_state = session.state.clone();
        let resumed = RuntimeOverworldSession::from_state(&runtime, &asset_root, saved_state)
            .expect("resume with block overrides");
        assert_eq!(resumed.overworld.map.metatile_at(1, 0), Some(7));

        let wrong_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 9)
            .expect_err("scene command indexes are exact");
        let wrong_scene = error_debug(wrong_scene);
        assert!(wrong_scene.contains("RuntimeSceneScript"));
        assert!(wrong_scene.contains("9"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_rejects_map_music_missing_from_runtime_catalog() {
        let root = temp_repository_root("overworld-missing-music");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .music = Some("MUSIC_ROUTE_30".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");

        let error = runtime
            .start_overworld_session(&asset_root, 0)
            .expect_err("missing map music asset must fail")
            .to_string();

        assert!(
            error.contains("saved maps.RuntimeMap.attributes.music MUSIC_ROUTE_30 is missing from compiled pack audio"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_save_resume_uses_saved_position_without_spawn_fallback() {
        let root = temp_repository_root("overworld-resume");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
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
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        let moved = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("move right");
        let save_path = root.join("slot.crystalsave");
        runtime
            .save_game(&save_path, session.state.clone())
            .expect("save moved state");
        let loaded = runtime.load_save(&save_path).expect("load moved state");

        let mut resumed = runtime
            .resume_overworld_session(&asset_root, loaded)
            .expect("resume saved overworld");

        assert_eq!(resumed.snapshot(), moved.snapshot);
        assert_eq!(resumed.state.frame_counter, 2);
        assert_eq!(
            resumed.state.overworld.snapshot_identity(),
            Some((
                "RuntimeMap",
                TilePosition::new(1, 0),
                Direction::Right,
                crystal_core::world::movement::MovementMode::Normal
            ))
        );
        let held = resumed
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("held right after resume");
        assert_eq!(held.pressed_mask, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_resume_rejects_inactive_state() {
        let root = temp_repository_root("overworld-inactive");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");

        let error = runtime
            .resume_overworld_session(&asset_root, GameState::default())
            .expect_err("inactive state must not fall back to spawn")
            .to_string();

        assert!(error.contains("inactive GameState"), "{error}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_rejects_conflicting_direction_buttons() {
        let root = temp_repository_root("overworld-conflict");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let error = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Left, GameButton::Right])
            .expect_err("conflicting directions must fail");

        let error = error_debug(error);
        assert!(error.contains("conflicting direction buttons"), "{error}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_helpers_bind_save_to_compiled_pack_identity() {
        let root = temp_repository_root("save");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let mut state = GameState::default();
        state.frame_counter = 77;

        runtime
            .save_game(&save_path, state.clone())
            .expect("write runtime save");
        let saved = read_save_game_for_modpack(
            &save_path,
            runtime.modpack(),
            &runtime.pack_identity().content_hash,
        )
        .expect("read save metadata for exact runtime pack");
        let summary = runtime
            .load_save_summary(&save_path)
            .expect("load exact runtime save summary");
        let checkpoint = runtime
            .load_save_checkpoint(&save_path, 7)
            .expect("load exact runtime save checkpoint");
        let slots = runtime
            .list_save_slots(&root)
            .expect("list exact runtime save slots");
        let session_checkpoint = runtime
            .session_save_checkpoint_for_state(
                LinkSessionIdentity::new(
                    "runtime-session",
                    runtime.modpack().clone(),
                    runtime.pack_identity().content_hash.clone(),
                )
                .expect("session"),
                &state,
                7,
            )
            .expect("session save checkpoint");
        let loaded = runtime.load_save(&save_path).expect("load runtime save");

        assert_eq!(
            saved.metadata().pack_content_hash(),
            runtime.pack_identity().content_hash.as_str()
        );
        assert_eq!(summary.modpack(), runtime.modpack());
        assert_eq!(
            summary.pack_content_hash(),
            runtime.pack_identity().content_hash.as_str()
        );
        assert_eq!(summary.saved_frame(), 77);
        assert_eq!(summary.state_frame(), 77);
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].slot_id(), "slot");
        assert_eq!(slots[0].path(), save_path);
        assert_eq!(slots[0].summary(), &summary);
        assert_eq!(checkpoint.summary(), &summary);
        assert_eq!(checkpoint.checksum().player_id(), 7);
        assert_eq!(checkpoint.checksum().frame(), 77);
        assert_eq!(session_checkpoint.checkpoint(), &checkpoint);
        assert_eq!(session_checkpoint.session().session_id(), "runtime-session");
        assert_eq!(loaded, state);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_currency_above_compiled_pack_caps() {
        let root = temp_repository_root("save-currency-caps");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                runtime_data_with_currency_caps(500, 50),
                report(),
            ),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.money = 501;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("money above pack cap must not save");
        let error = error_debug(error);
        assert!(
            error.contains("SavedMoneyExceedsLimit { amount: 501, limit: 500 }"),
            "{error}"
        );

        let mut state = GameState::default();
        state.moms_money = 501;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("mom money above pack cap must not save");
        let error = error_debug(error);
        assert!(
            error.contains("SavedMomsMoneyExceedsLimit { amount: 501, limit: 500 }"),
            "{error}"
        );

        let mut state = GameState::default();
        state.coins = 51;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("coins above pack cap must not save");
        let error = error_debug(error);
        assert!(
            error.contains("SavedCoinsExceedsLimit { amount: 51, limit: 50 }"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.active_repel_item = Some("SUPER_REPEL".to_string());
        state.repel_steps_remaining = 10;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active repel item must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved active_repel_item SUPER_REPEL: MissingSavedActiveRepelItem"));

        let mut state = GameState::default();
        state.active_repel_item = Some("POTION".to_string());
        state.repel_steps_remaining = 10;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active repel item must be configured as repel");
        let error = error_debug(error);
        assert!(
            error.contains("saved active_repel_item POTION: MissingSavedActiveRepelItem"),
            "{error}"
        );

        let mut repel_data = minimal_runtime_data();
        let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
        repel.repel_steps = Some(100);
        repel_data.items.insert("REPEL".to_string(), repel);
        let repel_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(repel_data, report()),
            identity(),
        )
        .expect("repel runtime");
        let mut state = GameState::default();
        state.active_repel_item = Some("REPEL".to_string());
        state.repel_steps_remaining = 101;
        let error = repel_runtime
            .save_game(&save_path, state)
            .expect_err("saved repel steps must not exceed compiled item duration");
        let error = error_debug(error);
        assert!(
            error.contains(
                "SavedRepelStepsExceedCompiledDuration { item_id: \"REPEL\", steps_remaining: 101, compiled_steps: 100 }"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.pending_special_battle_type = Some("BATTLETYPE_STALE".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending special battle type must be declared by compiled pack");
        let error = error_debug(error);
        assert!(error.contains(
            "saved pending_special_battle_type BATTLETYPE_STALE is not declared by compiled scripted battles or special routines"
        ));

        let mut state = GameState::default();
        state.script_runtime.current_music = Some("MUSIC_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved current music must exist in compiled pack audio");
        let error = error_debug(error);
        assert!(error.contains(
            "save field script_runtime.current_music references missing Music audio id 'MUSIC_MISSING'"
        ));

        let mut state = GameState::default();
        state
            .flags
            .event_flags
            .insert("EVENT_STALE".to_string(), true);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved event flags must be declared by compiled pack");
        let error = error_debug(error);
        assert!(error.contains(
            "saved flags.event_flags EVENT_STALE is missing from compiled pack event flags"
        ));

        let mut state = GameState::default();
        state
            .flags
            .engine_flags
            .insert("ENGINE_STALE".to_string(), true);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved engine flags must be declared by compiled pack");
        let error = error_debug(error);
        assert!(error.contains(
            "saved flags.engine_flags ENGINE_STALE is missing from compiled pack engine flags"
        ));

        let mut state = GameState::default();
        state
            .bug_contest
            .selected_contestant_flags
            .push("EVENT_STALE".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved Bug Contest contestant flags must be declared by compiled pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved bug_contest.selected_contestant_flags EVENT_STALE is missing from compiled pack event flags"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.last_spawn_identifier = Some(99);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("last spawn identifier must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved last_spawn_identifier 99 is missing from compiled pack runtime spawn points"
            ),
            "{error}"
        );

        let mut spawn_mismatch_data = minimal_runtime_data();
        let mut spawn = spawn_mismatch_data
            .runtime_spawn_points
            .get("0")
            .expect("minimal spawn point")
            .clone();
        spawn.identifier = 1;
        spawn_mismatch_data
            .runtime_spawn_points
            .insert("0".to_string(), spawn);
        let error = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(spawn_mismatch_data, report()),
            identity(),
        )
        .expect_err("spawn identifier mismatch fails pack verification");
        let error = error_debug(error);
        assert!(
            error.contains("runtime_spawn_point_identifier_mismatch"),
            "{error}"
        );

        let mut state = GameState::default();
        state.dig_warp_map_name = Some("MissingMap".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("dig warp map must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains("saved dig_warp_map_name MissingMap is missing"),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .map_block_overrides
            .insert("MissingMap".to_string(), BTreeMap::new());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("block override map must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains("saved map_block_overrides MissingMap is missing"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_validates_registered_key_item_against_compiled_pack() {
        let root = temp_repository_root("save-registered-key-item");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.registered_key_item = Some("BICYCLE".to_string());
        state.bag.key_items.insert("BICYCLE".to_string(), 1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("registered key item must exist in compiled pack");
        let error = error_debug(error);
        assert!(
            error.contains("saved bag.key_items item BICYCLE is missing from compiled pack items"),
            "{error}"
        );

        let mut data = minimal_runtime_data();
        data.items.insert(
            "BICYCLE".to_string(),
            runtime_item("BICYCLE", item_pocket("KEY_ITEM")),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime with registered key item");
        let mut state = GameState::default();
        state.registered_key_item = Some("BICYCLE".to_string());
        state.bag.key_items.insert("BICYCLE".to_string(), 1);
        runtime
            .save_game(&save_path, state)
            .expect("compiled carried registered key item saves");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_register_key_item_is_an_authoritative_mutation() {
        let root = temp_repository_root("register-key-item-mutation");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert(
            "BICYCLE".to_string(),
            runtime_item("BICYCLE", item_pocket("KEY_ITEM")),
        );
        data.items.insert(
            "OLD_ROD".to_string(),
            runtime_item("OLD_ROD", item_pocket("KEY_ITEM")),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime with key items");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");
        shell
            .session_mut()
            .state
            .bag
            .key_items
            .insert("BICYCLE".to_string(), 1);
        shell
            .session_mut()
            .state
            .bag
            .key_items
            .insert("OLD_ROD".to_string(), 1);
        shell.session_mut().state.registered_key_item = Some("OLD_ROD".to_string());

        let registered = shell
            .register_key_item("BICYCLE")
            .expect("carried declared key item registers through runtime mutation");

        assert_eq!(registered.outcome.item_id, "BICYCLE");
        assert_eq!(
            registered.outcome.previous_item_id,
            Some("OLD_ROD".to_string())
        );
        assert_eq!(
            shell.session().state().registered_key_item.as_deref(),
            Some("BICYCLE")
        );
        assert_eq!(
            registered.state_checksum,
            game_state_checksum(shell.session().state()).expect("checksum after registration")
        );

        let unknown = shell
            .register_key_item("bicycle")
            .expect_err("runtime command must not coerce item identifiers");
        let unknown = error_debug(unknown);
        assert!(unknown.contains("registered_key_item bicycle is missing"));
        assert_eq!(
            shell.session().state().registered_key_item.as_deref(),
            Some("BICYCLE")
        );

        shell.session_mut().state.bag.key_items.remove("OLD_ROD");
        let not_carried = shell
            .register_key_item("OLD_ROD")
            .expect_err("declared key item must be carried before registration");
        let not_carried = error_debug(not_carried);
        assert!(not_carried.contains("cannot register key item OLD_ROD because it is not carried"));
        assert_eq!(
            shell.session().state().registered_key_item.as_deref(),
            Some("BICYCLE")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_accepts_flags_declared_by_compiled_pack() {
        let root = temp_repository_root("save-declared-flags");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.story_event_script_constants
            .global
            .insert("EVENT_RUNTIME_WILD_DONE".to_string(), 2);
        data.initialize_events
            .engine_flags
            .push("ENGINE_RUNTIME_WILD_DONE".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let mut state = GameState::default();
        state
            .flags
            .event_flags
            .insert("EVENT_RUNTIME_CONTESTANT".to_string(), true);
        state
            .flags
            .event_flags
            .insert("EVENT_RUNTIME_WILD_DONE".to_string(), true);
        state
            .flags
            .engine_flags
            .insert("ENGINE_GOT_SHUCKIE_TODAY".to_string(), true);
        state
            .flags
            .engine_flags
            .insert("ENGINE_RUNTIME_WILD_DONE".to_string(), true);
        state
            .bug_contest
            .selected_contestant_flags
            .push("EVENT_RUNTIME_CONTESTANT".to_string());

        runtime
            .save_game(&save_path, state)
            .expect("save state with compiled declared flags");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_accepts_pending_special_battle_type_declared_by_compiled_pack() {
        let root = temp_repository_root("save-pending-special-battle-type");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.special_routines
            .insert("TrainerHouse".to_string(), SpecialRoutineRule::default());
        data.special_routines.insert(
            "CelebiShrineEvent".to_string(),
            SpecialRoutineRule::default(),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        assert!(
            runtime
                .pending_special_battle_type_ids()
                .contains("BATTLETYPE_TRAINER_HOUSE")
        );
        assert!(
            runtime
                .pending_special_battle_type_ids()
                .contains("BATTLETYPE_CELEBI")
        );
        assert!(runtime.has_pending_special_battle_type("BATTLETYPE_TRAINER_HOUSE"));
        assert!(runtime.has_pending_special_battle_type("BATTLETYPE_CELEBI"));
        assert!(
            runtime
                .require_pending_special_battle_type("BATTLETYPE_TRAINER_HOUSE")
                .is_ok()
        );
        assert!(
            runtime
                .require_pending_special_battle_type("BATTLETYPE_CELEBI")
                .is_ok()
        );

        let mut trainer_house = GameState::default();
        trainer_house.pending_special_battle_type = Some("BATTLETYPE_TRAINER_HOUSE".to_string());
        runtime
            .save_game(&save_path, trainer_house)
            .expect("save Trainer House special battle type");

        let mut celebi = GameState::default();
        celebi.pending_special_battle_type = Some("BATTLETYPE_CELEBI".to_string());
        runtime
            .save_game(&save_path, celebi)
            .expect("save Celebi special battle type");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_fishing_state_not_declared_by_compiled_pack() {
        let root = temp_repository_root("save-fishing-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.fishing.rod_state = FishingRodState::Waiting;
        state.fishing.rod_index = Some(1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active fishing rod must exist in compiled fishing tables");
        let error = error_debug(error);
        assert!(error.contains(
            "saved fishing.rod_index 1 resolves to GOOD_ROD, which is missing from compiled fishing rod tables"
        ));

        let mut data = minimal_runtime_data_with_fishing();
        data.fishing.swarm_rules.insert(
            "RUNTIME_SWARM".to_string(),
            FishingSwarmRule {
                daily_flag_bit: 2,
                swarm: 2,
                base_group: "FISHGROUP_RUNTIME".to_string(),
                swarm_group: "FISHGROUP_RUNTIME".to_string(),
            },
        );
        let swarm_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("swarm runtime");

        let mut stale_flag = GameState::default();
        stale_flag.fishing.daily_flags1 = 1 << 4;
        let error = swarm_runtime
            .save_game(&save_path, stale_flag)
            .expect_err("saved fishing daily flag bits must be pack-declared");
        let error = error_debug(error);
        assert!(error.contains(
            "saved fishing.daily_flags1 bit 4 is missing from compiled fishing swarm rules"
        ));

        let mut stale_swarm = GameState::default();
        stale_swarm.fishing.swarm_flag = 3;
        let error = swarm_runtime
            .save_game(&save_path, stale_swarm)
            .expect_err("saved fishing swarm ids must be pack-declared");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved fishing.swarm_flag 3 is missing from compiled fishing swarm rules"
            )
        );

        let mut stale_swarm_map = GameState::default();
        stale_swarm_map.swarms.active.insert(
            "SWARM_YANMA".to_string(),
            SwarmMapTarget {
                map_id: "MISSING_ROUTE".to_string(),
                map_group: Some(3),
                map_number: Some(18),
            },
        );
        let error = swarm_runtime
            .save_game(&save_path, stale_swarm_map)
            .expect_err("saved active swarm map constant must exist");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved swarms.active SWARM_YANMA references missing runtime map MISSING_ROUTE"
            ),
            "{error}"
        );

        let mut stale_swarm_group = GameState::default();
        stale_swarm_group.swarms.active.insert(
            "SWARM_YANMA".to_string(),
            SwarmMapTarget {
                map_id: "RUNTIME_MAP".to_string(),
                map_group: Some(9),
                map_number: Some(9),
            },
        );
        let error = swarm_runtime
            .save_game(&save_path, stale_swarm_group)
            .expect_err("saved active swarm group/number must match compiled metadata");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved swarms.active SWARM_YANMA map RUNTIME_MAP has group/number Some(9)/Some(9), expected 1/1"
            ),
            "{error}"
        );

        let mut valid_swarm = GameState::default();
        valid_swarm.swarms.active.insert(
            "SWARM_YANMA".to_string(),
            SwarmMapTarget {
                map_id: "RUNTIME_MAP".to_string(),
                map_group: Some(1),
                map_number: Some(1),
            },
        );
        swarm_runtime
            .save_game(&save_path, valid_swarm)
            .expect("valid active swarm state saves");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_invalid_overworld_and_map_override_references() {
        let root = temp_repository_root("save-overworld-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.overworld = OverworldMemory::Active {
            map_name: "MissingMap".to_string(),
            tile: TilePosition::new(0, 0),
            facing: Direction::Down,
            mode: crystal_core::world::movement::MovementMode::Normal,
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active overworld map must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved overworld.active.map_name MissingMap is missing"));

        let mut state = GameState::default();
        state.overworld = OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(4, 0),
            facing: Direction::Down,
            mode: crystal_core::world::movement::MovementMode::Normal,
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active overworld tile must fit map dimensions");
        let error = error_debug(error);
        assert!(error.contains(
            "saved overworld.active tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds"
        ));

        let mut state = GameState::default();
        state.overworld = OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(1, 0),
            facing: Direction::Down,
            mode: crystal_core::world::movement::MovementMode::Normal,
        };
        runtime
            .save_game(&save_path, state)
            .expect("odd active overworld tile is a valid exact runtime tile");

        let mut state = GameState::default();
        state.dig_warp_map_name = Some("RuntimeMap".to_string());
        state.dig_warp_index = Some(99);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("dig warp index must exist on map");
        let error = error_debug(error);
        assert!(error.contains("saved dig_warp_index 99 is missing from compiled map RuntimeMap"));

        let mut bad_dig_data = minimal_runtime_data();
        bad_dig_data
            .maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .events
            .warps[0]
            .x = 4;
        let bad_dig_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(bad_dig_data, report()),
            identity(),
        )
        .expect("bad dig runtime");
        let mut state = GameState::default();
        state.dig_warp_map_name = Some("RuntimeMap".to_string());
        state.dig_warp_index = Some(4);
        let error = bad_dig_runtime
            .save_game(&save_path, state)
            .expect_err("dig warp destination tile must fit map runtime bounds");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved dig_warp destination RuntimeMap warp 4 runtime tile (4, 0) is invalid"
            ),
            "{error}"
        );
        assert!(
            error.contains(
                "runtime player tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds 4x2"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .map_block_overrides
            .entry("RuntimeMap".to_string())
            .or_default()
            .insert((2, 0), 1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("block override coordinate must fit map");
        let error = error_debug(error);
        assert!(
            error.contains("saved map_block_overrides RuntimeMap coordinate (2, 0) is outside")
        );

        let mut state = GameState::default();
        state
            .map_block_overrides
            .entry("RuntimeMap".to_string())
            .or_default()
            .insert((0, 0), 0xffff);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("block override id must exist in compiled tileset collision data");
        let error = error_debug(error);
        assert!(error.contains(
            "saved map_block_overrides RuntimeMap coordinate (0, 0) block 0xffff is missing"
        ));

        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "RuntimeMap".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "MissingObject".to_string(),
                    OverworldObjectMemory {
                        x: 0,
                        y: 0,
                        tile: Some(TilePosition::new(0, 0)),
                        facing: None,
                    },
                )]),
                ..OverworldObjectMapMemory::default()
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("object override id must exist on map");
        let error = error_debug(error);
        assert!(error.contains("saved map_object_overrides.objects MissingObject is missing"));

        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "RuntimeMap".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "RuntimeNpc".to_string(),
                    OverworldObjectMemory {
                        x: 10,
                        y: 0,
                        tile: Some(TilePosition::new(10, 0)),
                        facing: None,
                    },
                )]),
                ..OverworldObjectMapMemory::default()
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("object override coordinate must fit map");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved map_object_overrides.objects RuntimeMap:RuntimeNpc raw coordinate (10, 0) resolves to runtime tile (10, 0) outside compiled runtime tile bounds"
            ),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_invalid_scene_references() {
        let root = temp_repository_root("save-scene-pack-references");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.maps.get_mut("RuntimeMap").expect("runtime map").scenes = MapSceneTable {
            scenes: vec![
                MapScene {
                    scene_id: "SCENE_RUNTIME_START".to_string(),
                    script_name: Some("RuntimeStartScene".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_RUNTIME_DONE".to_string(),
                    script_name: Some("RuntimeDoneScene".to_string()),
                },
            ],
        };
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.scenes.current_map_name = "MissingMap".to_string();
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("current scene map must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved scenes.current_map_name MissingMap is missing"));

        let mut state = GameState::default();
        state.scenes.map_scenes.insert(
            "RuntimeMap".to_string(),
            "SCENE_RUNTIME_MISSING".to_string(),
        );
        state
            .scenes
            .map_scene_indices
            .insert("RuntimeMap".to_string(), 0);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved scene name must exist in map scene table");
        let error = error_debug(error);
        assert!(
            error.contains("saved scenes.map_scenes RuntimeMap:SCENE_RUNTIME_MISSING is missing")
        );

        let mut state = GameState::default();
        state
            .scenes
            .map_scenes
            .insert("RuntimeMap".to_string(), "SCENE_RUNTIME_DONE".to_string());
        state
            .scenes
            .map_scene_indices
            .insert("RuntimeMap".to_string(), 0);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved scene index must match compiled scene position");
        let error = error_debug(error);
        assert!(error.contains("SCENE_RUNTIME_DONE index 0 does not match compiled scene index 1"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_bag_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-bag-pack-references");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        data.items
            .insert("POKE_BALL".to_string(), runtime_ball_item("POKE_BALL"));
        data.items.insert(
            "BASEMENT_KEY".to_string(),
            runtime_item("BASEMENT_KEY", item_pocket("KEY_ITEM")),
        );
        data.items
            .insert("TM01".to_string(), runtime_tmhm_item("TM01", 1, "TACKLE"));
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data.clone(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.bag.items.insert("MISSING_ITEM".to_string(), 1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved bag item must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved bag.items item MISSING_ITEM is missing"));

        let mut state = GameState::default();
        state.bag.items.insert("POKE_BALL".to_string(), 1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved bag item pocket must match compiled item pocket");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved bag.items item POKE_BALL is in compiled pocket BALL, expected ITEM"
            )
        );

        let mut state = GameState::default();
        state.bag.pc_items.insert("MISSING_PC_ITEM".to_string(), 1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved PC item must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved bag.pc_items item MISSING_PC_ITEM is missing"));

        let mut state = GameState::default();
        state.bag.pc_items.insert("POKE_BALL".to_string(), 1);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved PC item pocket must match compiled item pocket");
        let error = error_debug(error);
        assert!(error.contains(
            "saved bag.pc_items item POKE_BALL is in compiled pocket BALL, expected ITEM"
        ));

        let mut custom_data = data.clone();
        custom_data.items.insert(
            "BATTLE_PASS".to_string(),
            runtime_item("BATTLE_PASS", item_pocket("BATTLE_PASS")),
        );
        let custom_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(custom_data, report()),
            identity(),
        )
        .expect("custom runtime");
        let mut state = GameState::default();
        state.bag.tm_hm = vec![0, 0];
        state.bag.custom_pockets.insert(
            "BATTLE_PASS".to_string(),
            BTreeMap::from([("BATTLE_PASS".to_string(), 1)]),
        );
        custom_runtime
            .save_game(&save_path, state)
            .expect("saved custom pocket item must match compiled custom pocket");

        let mut state = GameState::default();
        state.bag.tm_hm = vec![0, 0];
        state.bag.custom_pockets.insert(
            "WRONG_POCKET".to_string(),
            BTreeMap::from([("BATTLE_PASS".to_string(), 1)]),
        );
        let error = custom_runtime
            .save_game(&save_path, state)
            .expect_err("saved custom pocket must match compiled item pocket");
        let error = error_debug(error);
        assert!(error.contains(
            "saved bag.custom_pockets.WRONG_POCKET item BATTLE_PASS is in compiled pocket BATTLE_PASS, expected WRONG_POCKET"
        ));

        let mut mismatched_data = data.clone();
        let mut mismatched = runtime_item("DIFFERENT_SCRIPT_NAME", item_pocket("ITEM"));
        mismatched.name = "Potion".to_string();
        mismatched_data
            .items
            .insert("POTION".to_string(), mismatched);
        let mismatched_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(mismatched_data, report()),
            identity(),
        )
        .expect("mismatched runtime");
        let mut state = GameState::default();
        state.bag.items.insert("POTION".to_string(), 1);
        let error = mismatched_runtime
            .save_game(&save_path, state)
            .expect_err("saved bag item id must match compiled item script_name");
        let error = error_debug(error);
        assert!(error.contains(
            "saved bag.items item POTION does not match compiled item script_name DIFFERENT_SCRIPT_NAME"
        ));

        let mut state = GameState::default();
        state.bag.tm_hm = vec![0];
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved TM/HM vector cannot be shorter than compiled indexes");
        let error = error_debug(error);
        assert!(
            error.contains("saved bag.tm_hm has 1 slots, fewer than compiled TM/HM max index 1")
        );

        let mut state = GameState::default();
        state.bag.tm_hm = vec![0, 1, 0];
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved TM/HM vector cannot extend past compiled indexes");
        let error = error_debug(error);
        assert!(error.contains("saved bag.tm_hm has 3 slots, compiled TM/HM max index is 1"));

        let mut duplicate_data = data;
        duplicate_data.items.insert(
            "TM01_DUPLICATE".to_string(),
            runtime_tmhm_item("TM01_DUPLICATE", 1, "TACKLE"),
        );
        let duplicate_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(duplicate_data, report()),
            identity(),
        )
        .expect("duplicate runtime");
        let mut state = GameState::default();
        state.bag.tm_hm = vec![0, 1];
        let error = duplicate_runtime
            .save_game(&save_path, state)
            .expect_err("saved TM/HM indexes must resolve uniquely");
        let error = error_debug(error);
        assert!(error.contains(
            "saved bag.tm_hm[1] matches 2 compiled TM/HM items; tmhm_index must be unique"
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_snapshot_rejects_tmhm_indexes_missing_from_saved_flags() {
        let root = temp_repository_root("snapshot-tmhm-missing-flag");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items
            .insert("TM01".to_string(), runtime_tmhm_item("TM01", 1, "TACKLE"));
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let state = GameState::default();

        let error = runtime
            .bag_snapshot(&state)
            .expect_err("snapshot must not default missing TM/HM flags")
            .to_string();

        assert!(error.contains("saved TM/HM flags missing index 1 required by compiled item TM01"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_pokedex_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-pokedex-pack-references");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.pokedex.seen_species.insert("CYNDAQUIL".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved seen species must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved pokedex.seen_species CYNDAQUIL is missing"));

        let mut state = GameState::default();
        state.pokedex.caught_species.insert("CYNDAQUIL".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved caught species must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved pokedex.caught_species CYNDAQUIL is missing"));

        let mut state = GameState::default();
        state.pokedex.caught_species.insert("CHIKORITA".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved caught species must also be saved as seen");
        let error = error_debug(error);
        assert!(error.contains(
            "saved pokedex.caught_species CHIKORITA is not present in saved pokedex.seen_species"
        ));

        let mut data = minimal_runtime_data();
        let mut mismatched_species = runtime_species();
        mismatched_species.id = "CYNDAQUIL".to_string();
        data.pokemon
            .insert("CHIKORITA".to_string(), mismatched_species);
        let mismatched_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime with mismatched species id");
        let mut state = GameState::default();
        state.pokedex.seen_species.insert("CHIKORITA".to_string());
        let error = mismatched_runtime
            .save_game(&save_path, state)
            .expect_err("saved species id must match compiled species payload id");
        let error = error_debug(error);
        assert!(error.contains(
            "saved pokedex.seen_species CHIKORITA does not match compiled species id CYNDAQUIL"
        ));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_active_battle_pokemon_missing_from_compiled_pack() {
        let root = temp_repository_root("save-battle-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let mut species =
            PokemonSpecies::new_for_tests("CYNDAQUIL", BaseStats::new(39, 52, 43, 65, 60, 50));
        species.int_id = 155;
        let pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
        let player_pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());

        let state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                origin_map_name: "RuntimeMap".to_string(),
                species: "CYNDAQUIL".to_string(),
                level: 5,
                source_script: "RuntimeScript".to_string(),
                startbattle_command_index: 4,
                resume_command_index: 5,
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            storage: active_player_storage(player_pokemon),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active battle species must exist in pack");
        let error = error_debug(error);

        assert!(
            error.contains("saved battle.static_wild.species CYNDAQUIL is missing"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_wild_battle_missing_from_compiled_encounters() {
        let root = temp_repository_root("save-wild-battle-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                minimal_runtime_data_with_grass_encounter(),
                report(),
            ),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());

        let state = GameState {
            battle: BattleMemory::Wild {
                roaming_slot: None,
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                map_name: "RuntimeMap".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("wild battle species and level must exist in compiled encounters");
        let error = error_debug(error);

        assert!(error.contains(
            "saved battle.wild RuntimeMap encounter CHIKORITA:5 is missing from compiled wild encounter sources"
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_active_battle_script_and_text_missing_from_compiled_pack() {
        let root = temp_repository_root("save-battle-script-pack-references");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.asm_text.insert(
            "RuntimeSeenText".to_string(),
            "Runtime seen text".to_string(),
        );
        data.asm_text
            .insert("RuntimeWinText".to_string(), "Runtime win text".to_string());
        data.asm_text.insert(
            "RuntimeLossText".to_string(),
            "Runtime loss text".to_string(),
        );
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.scripts.insert(
            "RuntimeWildScript".to_string(),
            serde_json::Value::Array(Vec::new()),
        );
        map.scripts.insert(
            "RuntimeTrainerScript".to_string(),
            serde_json::Value::Array(Vec::new()),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());

        let state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                origin_map_name: "RuntimeMap".to_string(),
                species: "CHIKORITA".to_string(),
                level: 5,
                source_script: "MissingScript".to_string(),
                startbattle_command_index: 4,
                resume_command_index: 5,
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("static battle source script must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved battle.static_wild.source_script MissingScript is missing"));

        let state = GameState {
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                origin_map_name: "RuntimeMap".to_string(),
                species: "CHIKORITA".to_string(),
                level: 5,
                source_script: "RuntimeWildScript".to_string(),
                startbattle_command_index: 4,
                resume_command_index: 5,
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("static battle request must match compiled scripted wild battle");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved battle.static_wild RuntimeMap/RuntimeWildScript:4->5 request BATTLETYPE_NORMAL:CHIKORITA:5 is missing from compiled wild battle origins"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "MissingSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle text must match compiled request");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.seen_text MissingSeenText does not match compiled trainer battle RuntimeTrainerScript seen_text RuntimeSeenText"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_STALE_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle event flag must match compiled request");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.event_flag EVENT_STALE_RIVAL does not match compiled trainer battle RuntimeTrainerScript event_flag EVENT_BEAT_RUNTIME_RIVAL"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_ROUTE_29".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle encounter music must match compiled trainer");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.encounter_music MUSIC_ROUTE_29 does not match compiled pack trainer RIVAL1 encounter music MUSIC_RIVAL_ENCOUNTER"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 2,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle AI flags must match compiled trainer");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.ai_move_flags 2 does not match compiled pack trainer RIVAL1 ai_move_flags 1"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_SMART".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle AI layers must match compiled trainer");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.ai_layers [\"AI_SMART\"] do not match compiled pack trainer RIVAL1 ai_layers [\"AI_BASIC\"]"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "OLD_RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle name must match compiled trainer");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.trainer_name OLD_RIVAL@ does not match compiled pack trainer RIVAL1 name RIVAL@"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 50,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle reward must match compiled trainer");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.reward 50 does not match compiled pack trainer RIVAL1 base_reward 100"
            ),
            "{error}"
        );

        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_RIVAL_ENCOUNTER.mid"),
        );
        let mut trainer_party_data = minimal_runtime_data_with_scripted_battles();
        trainer_party_data.asm_text.insert(
            "RuntimeSeenText".to_string(),
            "Runtime seen text".to_string(),
        );
        trainer_party_data
            .asm_text
            .insert("RuntimeWinText".to_string(), "Runtime win text".to_string());
        trainer_party_data.asm_text.insert(
            "RuntimeLossText".to_string(),
            "Runtime loss text".to_string(),
        );
        trainer_party_data.audio.push(
            ModpackAudioAsset::music(
                "MUSIC_RIVAL_ENCOUNTER",
                "content-packs/test/music/MUSIC_RIVAL_ENCOUNTER.mid",
            )
            .expect("trainer music asset"),
        );
        trainer_party_data
            .maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeTrainerScript".to_string(),
                serde_json::Value::Array(Vec::new()),
            );
        let trainer_party_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(trainer_party_data, report()),
            identity(),
        )
        .expect("trainer party runtime");
        let stale_trainer_pokemon = Pokemon::new_for_tests(runtime_species(), 6, Dv::default());
        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: stale_trainer_pokemon.clone(),
                enemy_party: vec![stale_trainer_pokemon],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = trainer_party_runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle party must match compiled trainer roster");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved battle.trainer.enemy_party[0] level 6 does not match compiled trainer RIVAL1 level 5"
            ),
            "{error}"
        );

        let state = GameState {
            battle: BattleMemory::Trainer {
                battle_type: "BATTLETYPE_TRAINER".to_string(),
                trainer_class: "RIVAL1".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_name: "RIVAL@".to_string(),
                event_flag: "EVENT_BEAT_RUNTIME_RIVAL".to_string(),
                seen_text: "RuntimeSeenText".to_string(),
                win_text: "RuntimeWinText".to_string(),
                loss_text: "RuntimeLossText".to_string(),
                callback: String::new(),
                source_script: "RuntimeTrainerScript".to_string(),
                enemy_pokemon: pokemon.clone(),
                enemy_party: vec![pokemon.clone()],
                reward: 100,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                ai_layers: vec!["AI_BASIC".to_string()],
            },
            storage: active_player_storage(pokemon.clone()),
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("trainer battle encounter music must exist in pack audio");
        let error = error_debug(error);
        assert!(
            error.contains(
                "save field battle.trainer.encounter_music references missing Music audio id 'MUSIC_RIVAL_ENCOUNTER'"
            ),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_storage_pokemon_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-storage-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let mut species =
            PokemonSpecies::new_for_tests("CYNDAQUIL", BaseStats::new(39, 52, 43, 65, 60, 50));
        species.int_id = 155;

        let mut state = GameState::default();
        state.storage.party.pokemon[0] =
            Some(Pokemon::new_for_tests(species.clone(), 5, Dv::default()));
        state.party = crystal_core::state::PartyState::from_storage(&state.storage);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("party Pokemon species must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved storage.party[0].species CYNDAQUIL is missing"));

        let mut state = GameState::default();
        let mut stored = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        stored.item = Some("MISSING_ITEM".to_string());
        let mut box0 = crystal_core::models::PcBox::new(0);
        box0.set_slot(0, Some(stored));
        state.storage.pc_boxes.push(box0);
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("stored Pokemon held item must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved storage.pc_boxes[0][0].item MISSING_ITEM is missing"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_link_session_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-link-session-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.link_session.link_mode = 1;
        state.link_session.active_room = Some("TradeCenter".to_string());
        state.link_session.serial_connection_status =
            crystal_core::state::LinkSerialConnectionStatus::UsingExternalClock;
        state.link_session.battle_random = Some(crystal_core::random::LinkBattleRandomState {
            seeds: [0; 10],
            count: 0,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active link room must exist in pack special routines");
        let error = error_debug(error);
        assert!(
            error.contains("saved link_session.active_room TradeCenter is missing from compiled pack special routines")
        );

        let mut data = minimal_runtime_data();
        data.special_routines
            .insert("TradeCenter".to_string(), SpecialRoutineRule::default());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut state = GameState::default();
        state.link_session.link_mode = 1;
        state.link_session.active_room = Some("TradeCenter".to_string());
        state.link_session.serial_connection_status =
            crystal_core::state::LinkSerialConnectionStatus::UsingExternalClock;
        state.link_session.battle_random = Some(crystal_core::random::LinkBattleRandomState {
            seeds: [0; 10],
            count: 0,
        });
        runtime
            .save_game(&save_path, state)
            .expect("active link room is backed by compiled pack special routine");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_magikarp_record_without_compiled_length_table() {
        let root = temp_repository_root("save-magikarp-record-pack-references");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.magikarp_lengths.clear();
        let error = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect_err("runtime pack must declare Magikarp length table");
        let error = error_debug(error);
        assert!(
            error.contains("missing_runtime_magikarp_lengths"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_special_saved_pokemon_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-special-pokemon-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let mut species =
            PokemonSpecies::new_for_tests("CYNDAQUIL", BaseStats::new(39, 52, 43, 65, 60, 50));
        species.int_id = 155;

        let mut state = GameState::default();
        state.bug_contest.caught_species = Some("CYNDAQUIL".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("bug contest caught species must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved bug_contest.caught_species CYNDAQUIL is missing"));

        let mut state = GameState::default();
        state.day_care.man.pokemon =
            Some(Pokemon::new_for_tests(species.clone(), 5, Dv::default()));
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("day care Pokemon species must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved day_care.man.pokemon.species CYNDAQUIL is missing"));

        let mut state = GameState::default();
        state.roaming_pokemon[0] = crystal_core::state::RoamingPokemonState {
            species: Some("CYNDAQUIL".to_string()),
            level: 40,
            map_group: 1,
            map_number: 1,
            hp: 1,
            dvs_be: [0, 0],
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("roaming Pokemon species must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved roaming_pokemon[0].species CYNDAQUIL is missing"));

        let mut state = GameState::default();
        state.roaming_pokemon[0] = crystal_core::state::RoamingPokemonState {
            species: Some("CHIKORITA".to_string()),
            level: 41,
            map_group: 1,
            map_number: 1,
            hp: 1,
            dvs_be: [0, 0],
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved roaming level must match roaming definitions");
        let error = error_debug(error);
        assert!(error.contains(
            "saved roaming_pokemon[0] CHIKORITA level 41 does not match catalog init slot"
        ));

        let mut state = GameState::default();
        state.roaming_pokemon[0] = crystal_core::state::RoamingPokemonState {
            species: Some("CHIKORITA".to_string()),
            level: 40,
            map_group: 1,
            map_number: 99,
            hp: 1,
            dvs_be: [0, 0],
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("roaming Pokemon location must exist in pack map metadata");
        let error = error_debug(error);
        assert!(error.contains(
            "saved roaming_pokemon[0] location group 1 map 99 is missing from compiled runtime map metadata"
        ));

        let mut state = GameState::default();
        state.roaming_pokemon[0] = crystal_core::state::RoamingPokemonState {
            species: Some("CHIKORITA".to_string()),
            level: 40,
            map_group: 1,
            map_number: 1,
            hp: u8::MAX,
            dvs_be: [0, 0],
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved roaming HP must fit the exact species, level, and DVs");
        let error = error_debug(error);
        assert!(
            error.contains("saved roaming_pokemon[0] hp 255 exceeds CHIKORITA level 40 max HP"),
            "{error}"
        );

        let mut state = GameState::default();
        state.mystery_gift.stored_item = Some("MISSING_ITEM".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("mystery gift item must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved mystery_gift.stored_item MISSING_ITEM is missing"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_battle_tower_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-battle-tower-pack-references");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.battle_tower.challenge_state = 1;
        state.battle_tower.reward_item = "MISSING_ITEM".to_string();
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("Battle Tower reward item must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved battle_tower.reward_item MISSING_ITEM is missing"));

        let mut state = GameState::default();
        state.battle_tower.reward_item = "MISSING_ITEM".to_string();
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("inactive Battle Tower reward item override must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved battle_tower.reward_item MISSING_ITEM is missing"));

        let mut data = minimal_runtime_data();
        data.battle_tower_rules = None;
        data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        let missing_rules = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data.clone(), report()),
            identity(),
        )
        .expect_err("runtime packs without Battle Tower rules fail verification");
        let error = error_debug(missing_rules);
        assert!(
            error.contains("missing_runtime_battle_tower_rules"),
            "{error}"
        );

        data.battle_tower_rules = Some(crystal_core::systems::special_routines::BattleTowerRules {
            banned_species: BTreeMap::new(),
            required_party_count: 3,
            challenge_streak_length: 7,
            minimum_level_group: 10,
            maximum_level_group: 100,
            level_group_size: 10,
            party_count_failure_text: "Need three.".to_string(),
            duplicate_species_failure_text: "No duplicates.".to_string(),
            duplicate_held_item_failure_text: "No duplicate items.".to_string(),
            egg_failure_text: "No eggs.".to_string(),
            trainers: Vec::new(),
            mon_groups: Vec::new(),
        });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");

        let mut state = GameState::default();
        state.battle_tower.level_group = 9;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved Battle Tower level group must fit pack rules");
        let error = error_debug(error);
        assert!(error.contains(
            "saved battle_tower.level_group 9 is outside compiled Battle Tower range 10..=100"
        ));

        let mut state = GameState::default();
        state.battle_tower.record_streaks = vec![0; 8];
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved Battle Tower records must fit pack streak length");
        let error = error_debug(error);
        assert!(error.contains(
            "saved battle_tower.record_streaks has 8 entries, compiled Battle Tower challenge_streak_length is 7"
        ));

        let mut state = GameState::default();
        state.battle_tower.loaded_trainer_id = Some("MISSING_TRAINER".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("loaded Battle Tower trainer must exist in pack");
        let error = error_debug(error);
        assert!(error.contains("saved battle_tower.loaded_trainer_id MISSING_TRAINER is missing"));

        let mut trainer_mismatch_data = verified_runtime_bootstrap_data();
        trainer_mismatch_data
            .trainers
            .trainers
            .get_mut("TRAINER_RUNTIME")
            .expect("runtime trainer")
            .trainer_id = "TRAINER_OTHER".to_string();
        let error = trainer_mismatch_data
            .validate_saved_trainer_reference("battle_tower.loaded_trainer_id", "TRAINER_RUNTIME")
            .expect_err("saved trainer id must match compiled trainer payload")
            .to_string();
        assert!(error.contains(
            "saved battle_tower.loaded_trainer_id TRAINER_RUNTIME does not match compiled trainer id TRAINER_OTHER"
        ));

        let mut state = GameState::default();
        state.battle_tower.last_sprite_constant = Some("SPRITE_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("loaded Battle Tower sprite must exist in pack");
        let error = error_debug(error);
        assert!(
            error.contains("saved battle_tower.last_sprite_constant SPRITE_MISSING is missing")
        );

        let mut state = GameState::default();
        state.battle_tower.selected_party_indexes = vec![0];
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("selected Battle Tower party slot must contain Pokemon");
        let error = error_debug(error);
        assert!(error.contains("saved battle_tower.selected_party_indexes slot 0 has no party"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_buena_password_references_missing_from_compiled_pack() {
        let root = temp_repository_root("save-buena-password-pack-references");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.buena_prizes.clear();
        let missing_prizes = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect_err("runtime packs without Buena prizes fail verification");
        let error = error_debug(missing_prizes);
        assert!(error.contains("missing_runtime_buena_prizes"), "{error}");
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state.buenas_password.generated = true;
        state.buenas_password.category_index = 99;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("generated Buena password category must exist in pack");
        let error = error_debug(error);
        assert!(error.contains(
            "saved buenas_password.category_index 99 is outside compiled Buena password categories"
        ));

        let mut data = minimal_runtime_data();
        data.buena_password_categories
            .order
            .push("RuntimeWords".to_string());
        data.buena_password_categories.categories.insert(
            "RuntimeWords".to_string(),
            BuenaPasswordCategoryDefinition {
                category_type: "BUENA_STRING".to_string(),
                points: 1,
                options: vec!["PASSWORD".to_string()],
            },
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");

        let mut state = GameState::default();
        state.buenas_password.generated = true;
        state.buenas_password.category_index = 1;
        state.buenas_password.option_index = 1;
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("generated Buena password option must exist in pack");
        let error = error_debug(error);
        assert!(error.contains(
            "saved buenas_password.option_index 1 is outside compiled Buena password category RuntimeWords options"
        ));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_rejects_script_runtime_references_missing_from_compiled_pack() {
        std::thread::Builder::new()
            .name("save-runtime-reference-validation".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(runtime_save_rejects_script_runtime_references_missing_from_compiled_pack_impl)
            .expect("spawn save validation test thread")
            .join()
            .expect("save validation test thread should not panic");
    }

    fn runtime_save_rejects_script_runtime_references_missing_from_compiled_pack_impl() {
        let root = temp_repository_root("save-script-runtime-pack-references");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_NONE.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/sfx/SFX_TACKLE.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/sfx/SFX_ITEM.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/cries/CRY_CHIKORITA.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = verified_runtime_bootstrap_data();
        data.pokemon_cries
            .get_mut("CHIKORITA")
            .expect("runtime cry metadata")
            .cry = "CRY_CHIKORITA".to_string();
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::music("MUSIC_NONE", "content-packs/test/music/MUSIC_NONE.mid")
                .expect("music none asset"),
            ModpackAudioAsset::sound_effect("SFX_TACKLE", "content-packs/test/sfx/SFX_TACKLE.mid")
                .expect("sfx asset"),
            ModpackAudioAsset::sound_effect("SFX_ITEM", "content-packs/test/sfx/SFX_ITEM.mid")
                .expect("item sfx asset"),
            ModpackAudioAsset::cry(
                "CRY_CHIKORITA",
                "content-packs/test/cries/CRY_CHIKORITA.mid",
            )
            .expect("cry asset"),
        ];
        let runtime_audio = data.audio.clone();
        data.asm_text.insert(
            "OtherRuntimeText".to_string(),
            "OtherRuntimeText".to_string(),
        );
        for routine in [
            "FadeOutToWhite",
            "ClearTilemap",
            "PlaceMoneyTopRight",
            "DisplayMoneyAndCoinBalance",
            "DisplayCoinCaseBalance",
            "FadeOutMusic",
            "WaitSFX",
            "PlayCurMonCry",
            "GetMysteryGiftItem",
        ] {
            data.special_routines
                .insert(routine.to_string(), SpecialRoutineRule::default());
        }
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeScript".to_string(),
                serde_json::json!([
                    { "command": "noop" },
                    { "command": "special", "args": ["RuntimeSpecial"] },
                    { "command": "noop" }
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert("OtherQueuedScript".to_string(), serde_json::json!([]));
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimePayloadScript".to_string(),
                serde_json::json!([
                    { "command": "elevfloor", "args": ["FLOOR_1F", "4", "RuntimeMap"] },
                    { "command": "stonetable", "args": ["5", "RUNTIME_BOULDER", "RuntimeScript"] },
                    { "command": "describedecoration", "args": ["DECODESC_LEFT_DOLL"] },
                    { "command": "writevar", "args": ["VAR_BLUECARDBALANCE"] },
                    { "command": "getnum", "args": ["STRING_BUFFER_3"] },
                    { "command": "pokemart", "args": ["MARTTYPE_STANDARD", "RUNTIME_MART"] },
                    { "command": "musicfadeout", "args": ["MUSIC_ROUTE_29", "16"] },
                    { "command": "waitsfx" },
                    { "command": "warpfacing", "args": ["RUNTIME_MAP", "2", "1", "RIGHT"] },
                    { "command": "newloadmap", "args": ["MAPSETUP_TRAIN"] },
                    { "command": "reanchormap", "args": ["MAPSETUP_TRAIN"] },
                    { "command": "refreshmap" },
                    { "command": "writetext", "args": ["RuntimeText"] },
                    { "command": "jumptext", "args": ["RuntimeText"] },
                    { "command": "yesorno" },
                    { "command": "waitbutton" },
                    { "command": "endcallback" },
                    { "command": "pause", "args": ["15"] },
                    { "command": "earthquake", "args": ["72"] },
                    { "command": "showemote", "args": ["EMOTE_SHOCK", "RuntimeObject", "16"] },
                    { "command": "writecmdqueue", "args": ["RuntimeScript"] },
                    { "command": "cmdqueue", "args": ["BANK_1", "RuntimeScript"] },
                    { "command": "conditional_event", "args": ["EVENT_RUNTIME", "RuntimeScript"] },
                    { "command": "catchtutorial", "args": ["BATTLETYPE_TUTORIAL"] }
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_runtime_commands
            .push(ScriptRuntimeCommand {
                command: "catchtutorial".to_string(),
                args: vec!["BATTLETYPE_TUTORIAL".to_string()],
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 22,
            });
        let report = report_for(&data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report.clone()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::FadeMusic,
                audio_id: Some("MUSIC_NONE".to_string()),
                fade_frames: Some(2),
                source_script: "FadeOutMusic".to_string(),
                command_index: 0,
            });
        runtime
            .save_game(&save_path, state)
            .expect("special routine audio event with exact runtime payload");

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::FadeMusic,
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: Some(2),
                source_script: "FadeOutMusic".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("special routine audio event must use exact routine audio id");
        let error = error_debug(error);
        assert!(
            error.contains(
                "saved script_runtime.audio_events[0].source_script FadeOutMusic special audio event audio_id MUSIC_ROUTE_29 does not match MUSIC_NONE"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect,
                audio_id: Some("SFX_TACKLE".to_string()),
                fade_frames: None,
                source_script: "WaitSFX".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("special WaitSFX event must not carry audio id");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.audio_events[0].source_script WaitSFX special audio event has unexpected audio_id SFX_TACKLE"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::SoundEffect,
                audio_id: Some("SFX_ITEM".to_string()),
                fade_frames: None,
                source_script: "GetMysteryGiftItem".to_string(),
                command_index: 1,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("special audio event command index must match runtime origin");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.audio_events[0].source_script GetMysteryGiftItem:1 special audio event must use command_index 0"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.current_music = Some("SFX_TACKLE".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved current music must reference a compiled music asset");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.current_music SFX_TACKLE is compiled as SoundEffect, expected Music"
        ));

        let mut state = GameState::default();
        state.script_runtime.last_special_routine = Some("MissingRoutine".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("last special routine must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains("saved script_runtime.last_special_routine MissingRoutine is missing")
        );

        let mut state = GameState::default();
        state.script_runtime.active_menu = Some("MissingMenu".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active menu must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains("saved script_runtime.active_menu MissingMenu is missing"));

        let mut state = GameState::default();
        state.script_runtime.active_menu = Some("RuntimeMenu".to_string());
        let error = runtime
            .active_menu_snapshot(&state)
            .expect_err("active script menu snapshots require a current map source")
            .to_string();
        assert!(
            error.contains(
                "active runtime menu 'RuntimeMenu' requires an active overworld map or special routine"
            ),
            "{error}"
        );

        let mut other_map = runtime_map();
        other_map.id = "OtherMap".to_string();
        other_map.attributes.map_constant = Some("OTHER_MAP".to_string());
        other_map.attributes.blocks_label = Some("OtherMap_Blocks".to_string());
        other_map.attributes.map_scripts_label = Some("OtherMap_MapScripts".to_string());
        other_map.attributes.map_events_label = Some("OtherMap_MapEvents".to_string());
        other_map.script_menu_definitions.clear();
        other_map.script_vertical_menus.clear();
        let mut other_map_data = minimal_runtime_data_with_music();
        other_map_data.map_blocks.insert(
            "OtherMap_Blocks".to_string(),
            other_map_data
                .map_blocks
                .get("RuntimeMap_Blocks")
                .cloned()
                .expect("runtime map blocks label"),
        );
        other_map_data
            .map_scripts
            .insert("OtherMap_MapScripts".to_string(), serde_json::json!([]));
        other_map_data.asm_text.insert(
            "OtherMap_MapEvents".to_string(),
            "OtherMap events".to_string(),
        );
        other_map_data.runtime_map_metadata.insert(
            "OTHER_MAP".to_string(),
            runtime_map_metadata("OTHER_MAP", "OtherMap", 1, 2, "ROUTE"),
        );
        other_map_data
            .map_attributes
            .insert("OtherMap".to_string(), other_map.attributes.clone());
        other_map_data
            .maps
            .insert("OtherMap".to_string(), other_map);
        let other_map_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(other_map_data, report),
            identity(),
        )
        .expect("runtime with second map");
        let mut state = GameState::default();
        state.overworld = OverworldMemory::Active {
            map_name: "OtherMap".to_string(),
            tile: TilePosition::new(0, 0),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        state.script_runtime.active_menu = Some("RuntimeMenu".to_string());
        let error = other_map_runtime
            .active_menu_snapshot(&state)
            .expect_err("active menu must resolve from the active map")
            .to_string();
        assert!(
            error.contains(
                "active runtime menu 'RuntimeMenu' is not declared by current compiled map OtherMap"
            ),
            "{error}"
        );
        let other_map_ui = other_map_runtime
            .ui_snapshot(&state, None)
            .expect("other map UI snapshot");
        assert!(
            other_map_ui.elevators.is_empty(),
            "active-map UI snapshot must not expose elevators from other maps"
        );
        assert!(
            other_map_ui.gift_pokemon.is_empty(),
            "active-map UI snapshot must not expose gift Pokemon from other maps"
        );
        let inactive_ui = other_map_runtime
            .ui_snapshot(&GameState::default(), None)
            .expect("inactive UI snapshot");
        assert!(inactive_ui.elevators.is_empty());
        assert!(inactive_ui.gift_pokemon.is_empty());

        let mut state = GameState::default();
        state.script_runtime.active_pokemon_picture = Some("CYNDAQUIL".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("active Pokemon picture species must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains("saved script_runtime.active_pokemon_picture CYNDAQUIL is missing"));

        let mut state = GameState::default();
        state.script_runtime.last_talked_object = Some("MISSING_OBJECT".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("last talked object requires active map context");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.last_talked_object MISSING_OBJECT requires an active overworld map"
        ));

        let mut state = GameState::default();
        state.overworld = OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(0, 0),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        state.script_runtime.last_talked_object = Some("MISSING_OBJECT".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("last talked object must exist in active compiled map");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.last_talked_object MISSING_OBJECT is missing from compiled map RuntimeMap objects"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .phone_numbers
            .insert("PHONE_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved phone contact must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains("saved script_runtime.phone_numbers PHONE_MISSING is missing"));

        let mut phone_mismatch_data = verified_runtime_bootstrap_data();
        phone_mismatch_data.audio = runtime_audio.clone();
        phone_mismatch_data
            .pokemon_cries
            .get_mut("CHIKORITA")
            .expect("runtime cry metadata")
            .cry = "CRY_CHIKORITA".to_string();
        phone_mismatch_data
            .phone_contacts
            .0
            .get_mut("PHONE_RUNTIME")
            .expect("runtime phone contact")
            .contact_id = "PHONE_OTHER".to_string();
        let phone_mismatch_report = report_for(&phone_mismatch_data);
        let phone_mismatch_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(phone_mismatch_data, phone_mismatch_report),
            identity(),
        )
        .expect("phone mismatch runtime");
        let mut state = GameState::default();
        state
            .script_runtime
            .phone_numbers
            .insert("PHONE_RUNTIME".to_string());
        let error = phone_mismatch_runtime
            .save_game(&save_path, state)
            .expect_err("saved phone contact must match compiled contact payload");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.phone_numbers PHONE_RUNTIME does not match compiled phone contact id PHONE_OTHER"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .special_phone_calls
            .push("SPECIALCALL_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved special phone call must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.special_phone_calls[0] SPECIALCALL_MISSING is missing"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .completed_trades
            .push("NPC_TRADE_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("completed NPC trade must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains("saved script_runtime.completed_trades[0] NPC_TRADE_MISSING is missing")
        );

        let mut state = GameState::default();
        let divergent_species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(99, 49, 65, 45, 49, 65));
        state.storage.party.pokemon[0] =
            Some(Pokemon::new_for_tests(divergent_species, 5, Dv::default()));
        state.sync_party_from_storage();
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved Pokemon species payload must match compiled pack species");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved storage.party[0].species CHIKORITA does not match compiled pack species data"
        ));

        let mut move_mismatch_data = verified_runtime_bootstrap_data();
        move_mismatch_data.audio = runtime_audio.clone();
        move_mismatch_data
            .pokemon_cries
            .get_mut("CHIKORITA")
            .expect("runtime cry metadata")
            .cry = "CRY_CHIKORITA".to_string();
        move_mismatch_data
            .moves
            .insert("TACKLE".to_string(), runtime_move_named("GROWL", 40));
        let move_mismatch_report = report_for(&move_mismatch_data);
        let move_mismatch_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(move_mismatch_data, move_mismatch_report),
            identity(),
        )
        .expect("move mismatch runtime");
        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        pokemon.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 1,
            pp_ups: 0,
        });
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();
        let error = move_mismatch_runtime
            .save_game(&save_path, state)
            .expect_err("saved move must match compiled move payload");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved storage.party[0].moves[0] TACKLE does not match compiled move name GROWL"
        ));

        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        pokemon.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 43,
            pp_ups: 1,
        });
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved move PP must not exceed compiled pack max PP");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved storage.party[0].moves[0] TACKLE current_pp 43 exceeds compiled max PP 42"
        ));

        let mut state = GameState::default();
        state.pending_move_learn = Some(crystal_core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "CHIKORITA".to_string(),
            level: 5,
            learned_move: LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending move learn must target a saved party Pokemon");
        let error = format!("{error:#}");
        assert!(
            error.contains("pending_move_learn.party_index 0 references an empty saved party slot")
        );

        let mut pending_species_mismatch_data = verified_runtime_bootstrap_data();
        pending_species_mismatch_data.audio = runtime_audio.clone();
        pending_species_mismatch_data
            .pokemon_cries
            .get_mut("CHIKORITA")
            .expect("runtime cry metadata")
            .cry = "CRY_CHIKORITA".to_string();
        pending_species_mismatch_data.pokemon.insert(
            "BAYLEEF".to_string(),
            PokemonSpecies::new_for_tests("BAYLEEF", BaseStats::new(60, 62, 80, 60, 63, 80)),
        );
        pending_species_mismatch_data.pokemon_cries.insert(
            "BAYLEEF".to_string(),
            pending_species_mismatch_data
                .pokemon_cries
                .get("CHIKORITA")
                .expect("chikorita cry metadata")
                .clone(),
        );
        pending_species_mismatch_data.menu_icons.insert(
            "BAYLEEF".to_string(),
            pending_species_mismatch_data
                .menu_icons
                .get("CHIKORITA")
                .expect("chikorita menu icon")
                .clone(),
        );
        let mut bayleef_pokedex = pending_species_mismatch_data
            .pokedex_entries
            .get("CHIKORITA")
            .expect("chikorita pokedex entry")
            .clone();
        bayleef_pokedex.species = "BAYLEEF".to_string();
        pending_species_mismatch_data
            .pokedex_entries
            .insert("BAYLEEF".to_string(), bayleef_pokedex);
        pending_species_mismatch_data.pokemon_frontpic_anim.insert(
            "BAYLEEF".to_string(),
            pending_species_mismatch_data
                .pokemon_frontpic_anim
                .get("CHIKORITA")
                .expect("chikorita frontpic anim")
                .clone(),
        );
        let pending_species_mismatch_report = report_for(&pending_species_mismatch_data);
        let pending_species_mismatch_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                pending_species_mismatch_data,
                pending_species_mismatch_report,
            ),
            identity(),
        )
        .expect("pending species mismatch runtime");
        let mut state = GameState::default();
        state.storage.party.pokemon[0] =
            Some(Pokemon::new_for_tests(runtime_species(), 5, Dv::default()));
        state.sync_party_from_storage();
        state.pending_move_learn = Some(crystal_core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "BAYLEEF".to_string(),
            level: 5,
            learned_move: LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        let error = pending_species_mismatch_runtime
            .save_game(&save_path, state)
            .expect_err("pending move learn species must match the saved party slot");
        let error = format!("{error:#}");
        assert!(error.contains(
            "pending_move_learn.species_id BAYLEEF does not match saved storage.party[0].species CHIKORITA"
        ));

        let mut state = GameState::default();
        state.storage.party.pokemon[0] =
            Some(Pokemon::new_for_tests(runtime_species(), 5, Dv::default()));
        state.sync_party_from_storage();
        state.pending_move_learn = Some(crystal_core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "CHIKORITA".to_string(),
            level: 6,
            learned_move: LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending move learn level must match the saved party slot");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "pending_move_learn.level 6 does not match saved storage.party[0].level 5"
            )
        );

        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        pokemon.moves.push(LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 1,
            pp_ups: 0,
        });
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();
        state.pending_move_learn = Some(crystal_core::state::PendingMoveLearn {
            party_index: 0,
            species_id: "CHIKORITA".to_string(),
            level: 5,
            learned_move: LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
            defer_level_evolution: false,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending move learn must not queue a move already known by that Pokemon");
        let error = format!("{error:#}");
        assert!(error.contains(
            "pending_move_learn.learned_move.name TACKLE is already known by saved storage.party[0]"
        ));

        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        pokemon.status = Some("UNDECLARED_STATUS".to_string());
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved Pokemon status must be declared by the compiled pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved storage.party[0].status UNDECLARED_STATUS is missing from compiled pack status declarations"
        ));

        let mut status_data = verified_runtime_bootstrap_data();
        status_data.audio = runtime_audio.clone();
        status_data
            .pokemon_cries
            .get_mut("CHIKORITA")
            .expect("runtime cry metadata")
            .cry = "CRY_CHIKORITA".to_string();
        status_data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        status_data
            .items
            .get_mut("POTION")
            .expect("runtime item")
            .status_heals = vec!["MOD_STATUS".to_string()];
        let status_report = report_for(&status_data);
        let status_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(status_data, status_report),
            identity(),
        )
        .expect("status runtime");
        let mut state = GameState::default();
        let mut pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        pokemon.status = Some("MOD_STATUS".to_string());
        state.storage.party.pokemon[0] = Some(pokemon);
        state.sync_party_from_storage();
        status_runtime
            .save_game(&save_path, state)
            .expect("saved Pokemon status declared by item status_heals");

        let mut state = GameState::default();
        state
            .script_runtime
            .catch_tutorials
            .push("BATTLETYPE_TUTORIAL".to_string());
        runtime
            .save_game(&save_path, state)
            .expect("saved catch tutorial battle type declared by pack catchtutorial command");

        let mut state = GameState::default();
        state
            .script_runtime
            .catch_tutorials
            .push("BATTLETYPE_STALE".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved catch tutorial battle type must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.catch_tutorials[0] BATTLETYPE_STALE is missing from compiled pack catchtutorial commands"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .variable_sprites
            .insert("SPRITE_MON".to_string(), "SPRITE_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved variable sprite replacement must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.variable_sprites key SPRITE_MON is missing from compiled pack variable sprites"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.blackout_mod = Some("MISSING_MAP_CONSTANT".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved blackout map constant must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.blackout_mod MISSING_MAP_CONSTANT is missing from compiled pack map constants"
        ));

        let mut metadata_mismatch_data = verified_runtime_bootstrap_data();
        metadata_mismatch_data.audio = runtime_audio.clone();
        metadata_mismatch_data
            .pokemon_cries
            .get_mut("CHIKORITA")
            .expect("runtime cry metadata")
            .cry = "CRY_CHIKORITA".to_string();
        let mut metadata = metadata_mismatch_data
            .runtime_map_metadata
            .get("RUNTIME_MAP")
            .expect("minimal runtime map metadata")
            .clone();
        metadata.constant = "OTHER_RUNTIME_MAP".to_string();
        metadata_mismatch_data
            .runtime_map_metadata
            .insert("RUNTIME_MAP".to_string(), metadata);
        let metadata_mismatch_report = report_for(&metadata_mismatch_data);
        let error = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                metadata_mismatch_data,
                metadata_mismatch_report,
            ),
            identity(),
        )
        .expect_err("runtime map metadata constants must match before runtime load");
        let error = format!("{error:#}");
        assert!(error.contains(
            "runtime map metadata key 'RUNTIME_MAP' does not match record constant 'OTHER_RUNTIME_MAP'"
        ));

        let mut state = GameState::default();
        state.script_runtime.battle_tower_text = Some("MissingTowerText".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved Battle Tower text must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains("saved script_runtime.battle_tower_text MissingTowerText is missing")
        );

        let mut state = GameState::default();
        state.script_runtime.next_script = Some(ScriptLocation {
            origin_map_name: "RuntimeMap".to_string(),
            script: "MissingScript".to_string(),
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("next script must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.next_script.script MissingScript is missing from compiled pack scripts"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .effects
            .push(crystal_core::state::ScriptRuntimeEffect {
                command: "special".to_string(),
                args: vec!["RuntimeSpecial".to_string()],
                source_script: "MissingScript".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("script runtime effect source script must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error
                .contains("saved script_runtime.effects[0].source_script MissingScript is missing")
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .effects
            .push(crystal_core::state::ScriptRuntimeEffect {
                command: "special".to_string(),
                args: vec!["RuntimeSpecial".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 3,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("script runtime effect command index must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.effects[0].source_script RuntimeScript:3 is outside compiled script command count 3"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .effects
            .push(crystal_core::state::ScriptRuntimeEffect {
                command: "special".to_string(),
                args: vec!["RuntimeSpecial".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("script runtime effect command must match compiled command at saved index");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.effects[0].source_script RuntimeScript:0 command special does not match compiled command noop"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .effects
            .push(crystal_core::state::ScriptRuntimeEffect {
                command: "special".to_string(),
                args: vec!["MissingRoutine".to_string()],
                source_script: "RuntimeScript".to_string(),
                command_index: 1,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("script runtime effect args must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.effects[0].source_script RuntimeScript:1 args ["MissingRoutine"] do not match compiled args ["RuntimeSpecial"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .call_stack
            .push(crystal_core::state::ScriptReturnFrame {
                origin_map_name: "RuntimeMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                next_command_index: 4,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("script runtime return frame must resume inside compiled script");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.call_stack[0].source_script RuntimeScript:4 is outside compiled script command count 3"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .command_queue
            .push(crystal_core::state::ScriptRuntimeQueuedCommand {
                origin_map_name: "RuntimeMap".to_string(),
                command: "writecmdqueue".to_string(),
                target: "MissingQueuedScript".to_string(),
                bank: None,
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("queued command target must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.command_queue[0].target MissingQueuedScript is missing"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .command_queue
            .push(crystal_core::state::ScriptRuntimeQueuedCommand {
                origin_map_name: "RuntimeMap".to_string(),
                command: "writecmdqueue".to_string(),
                target: "OtherQueuedScript".to_string(),
                bank: None,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 20,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved queued command target must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.command_queue[0].source_script RuntimePayloadScript:20 args ["OtherQueuedScript"] do not match compiled args ["RuntimeScript"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .command_queue
            .push(crystal_core::state::ScriptRuntimeQueuedCommand {
                origin_map_name: "RuntimeMap".to_string(),
                command: "cmdqueue".to_string(),
                target: "RuntimeScript".to_string(),
                bank: Some("BANK_2".to_string()),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 21,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved queued command bank must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.command_queue[0].source_script RuntimePayloadScript:21 args ["BANK_2", "RuntimeScript"] do not match compiled args ["BANK_1", "RuntimeScript"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .command_queue
            .push(crystal_core::state::ScriptRuntimeQueuedCommand {
                origin_map_name: "RuntimeMap".to_string(),
                command: "conditional_event".to_string(),
                target: "RuntimeScript".to_string(),
                bank: Some("EVENT_STALE".to_string()),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 22,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("conditional_event data cannot be persisted as a queued command");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.command_queue[0].source_script RuntimePayloadScript:22 args ["EVENT_STALE", "RuntimeScript"] do not match compiled args ["EVENT_RUNTIME", "RuntimeScript"]"#
        ), "{error}");

        let mut state = GameState::default();
        state
            .script_runtime
            .checked_mail_targets
            .push("MissingMailScript".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("checked mail target must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.checked_mail_targets[0] MissingMailScript is missing"
            )
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .given_mail_targets
            .push("MissingGivenMailScript".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("given mail target must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.given_mail_targets[0] MissingGivenMailScript is missing"
        ));

        let mut state = GameState::default();
        state.script_runtime.elevator_floors.push(
            crystal_core::state::ScriptRuntimeElevatorFloor {
                floor: "FLOOR_2F".to_string(),
                warp: 4,
                target_map: "RuntimeMap".to_string(),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 0,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved elevator floor payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                r#"saved script_runtime.elevator_floors[0].source_script RuntimePayloadScript:0 args ["FLOOR_2F", "4", "RUNTIME_MAP"] do not match compiled args ["FLOOR_1F", "4", "RuntimeMap"]"#
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.stone_table_entries.push(
            crystal_core::state::ScriptRuntimeStoneTableEntry {
                warp: 5,
                object_event: "STALE_BOULDER".to_string(),
                script: "RuntimeScript".to_string(),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 1,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved stone table payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.stone_table_entries[0].source_script RuntimePayloadScript:1 args ["5", "STALE_BOULDER", "RuntimeScript"] do not match compiled args ["5", "RUNTIME_BOULDER", "RuntimeScript"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.decoration_descriptions.push(
            crystal_core::state::ScriptRuntimeDecorationDescription {
                decoration: "DECODESC_POSTER".to_string(),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 2,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved decoration description payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.decoration_descriptions[0].source_script RuntimePayloadScript:2 args ["DECODESC_POSTER"] do not match compiled args ["DECODESC_LEFT_DOLL"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.variable_writes.push(
            crystal_core::state::ScriptRuntimeVariableWrite {
                target: "VAR_STALE_TARGET".to_string(),
                value: "12".to_string(),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 3,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved variable write target must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.variable_writes[0].source_script RuntimePayloadScript:3 args ["VAR_STALE_TARGET"] do not match compiled args ["VAR_BLUECARDBALANCE"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.numeric_buffer_writes.push(
            crystal_core::state::ScriptRuntimeNumericBufferWrite {
                target_buffer: "STRING_BUFFER_4".to_string(),
                value: "37".to_string(),
                width: 3,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 4,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved numeric buffer target must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.numeric_buffer_writes[0].source_script RuntimePayloadScript:4 args ["STRING_BUFFER_4"] do not match compiled args ["STRING_BUFFER_3"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .shop_events
            .push(crystal_core::state::ScriptShopRuntimeEvent {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "STALE_MART".to_string(),
                inventory: Vec::new(),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 5,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved shop event payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.shop_events[0].source_script RuntimePayloadScript:5 args ["MARTTYPE_STANDARD", "STALE_MART"] do not match compiled args ["MARTTYPE_STANDARD", "RUNTIME_MART"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_shop = Some(crystal_core::state::ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "STALE_MART".to_string(),
            inventory: Vec::new(),
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 5,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending shop payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_shop.source_script RuntimePayloadScript:5 args ["MARTTYPE_STANDARD", "STALE_MART"] do not match compiled args ["MARTTYPE_STANDARD", "RUNTIME_MART"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_music_fade = Some(crystal_core::state::ScriptMusicFade {
            audio_id: "MUSIC_ROUTE_29".to_string(),
            fade_frames: 8,
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 6,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending music fade payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_music_fade.source_script RuntimePayloadScript:6 args ["MUSIC_ROUTE_29", "8"] do not match compiled args ["MUSIC_ROUTE_29", "16"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "musicfadeout".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::FadeMusic,
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: Some(8),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 6,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved audio event payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.audio_events[0].source_script RuntimePayloadScript:6 args ["MUSIC_ROUTE_29", "8"] do not match compiled args ["MUSIC_ROUTE_29", "16"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "cry".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::Cry,
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved cry audio event must reference a compiled cry asset");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.audio_events[0].audio_id MUSIC_ROUTE_29 is compiled as Music, expected Cry"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .audio_events
            .push(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "waitsfx".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect,
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 7,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("waitsfx audio events cannot carry saved audio payload");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.audio_events[0].source_script RuntimePayloadScript:7 command waitsfx has unexpected audio_id"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(crystal_core::state::ScriptWarpRequest {
            target_map: "RuntimeMap".to_string(),
            tile: TilePosition::new(1, 1),
            facing: Some(Direction::Down),
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 8,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending script warp payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_script_warp.source_script RuntimePayloadScript:8 args ["RUNTIME_MAP", "1", "1", "DOWN"] do not match compiled args ["RUNTIME_MAP", "2", "1", "RIGHT"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .map_events
            .push(crystal_core::state::ScriptMapRuntimeEvent {
                command: "warpfacing".to_string(),
                kind: crystal_core::state::ScriptMapRuntimeKind::Warp,
                target_map: Some("RuntimeMap".to_string()),
                tile: Some(TilePosition::new(2, 1)),
                facing: Some(Direction::Right),
                map_setup: None,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 8,
            });
        runtime
            .save_game(&save_path, state)
            .expect("saved map runtime warp event must match canonical compiled payload");

        let mut state = GameState::default();
        state
            .script_runtime
            .map_events
            .push(crystal_core::state::ScriptMapRuntimeEvent {
                command: "warpfacing".to_string(),
                kind: crystal_core::state::ScriptMapRuntimeKind::Warp,
                target_map: Some("RuntimeMap".to_string()),
                tile: Some(TilePosition::new(4, 0)),
                facing: Some(Direction::Right),
                map_setup: None,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 8,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved map runtime warp event destination tile must fit target map");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.map_events[0] destination RuntimeMap runtime tile (4, 0) is invalid"
            ),
            "{error}"
        );
        assert!(
            error.contains(
                "runtime player tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds 4x2"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.pending_map_load = Some(crystal_core::state::ScriptMapLoadRequest {
            command: "newloadmap".to_string(),
            map_setup: Some("MAPSETUP_STALE".to_string()),
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 9,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending map load payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_map_load.source_script RuntimePayloadScript:9 args ["MAPSETUP_STALE"] do not match compiled args ["MAPSETUP_TRAIN"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_map_refresh =
            Some(crystal_core::state::ScriptMapRefreshRequest {
                command: "reanchormap".to_string(),
                map_setup: Some("MAPSETUP_STALE".to_string()),
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 10,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending map refresh payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_map_refresh.source_script RuntimePayloadScript:10 args ["MAPSETUP_STALE"] do not match compiled args ["MAPSETUP_TRAIN"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .map_events
            .push(crystal_core::state::ScriptMapRuntimeEvent {
                command: "refreshmap".to_string(),
                kind: crystal_core::state::ScriptMapRuntimeKind::RefreshMap,
                target_map: Some("RuntimeMap".to_string()),
                tile: None,
                facing: None,
                map_setup: None,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 11,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("no-payload map events cannot carry saved map payload");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.map_events[0].source_script RuntimePayloadScript:11 command refreshmap has unexpected map payload"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .text_events
            .push(crystal_core::state::ScriptTextRuntimeEvent {
                command: "writetext".to_string(),
                kind: crystal_core::state::ScriptTextRuntimeKind::Write,
                text_label: Some("OtherRuntimeText".to_string()),
                face_player: false,
                closes_text: false,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 12,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved text event label must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.text_events[0].source_script RuntimePayloadScript:12 args ["OtherRuntimeText"] do not match compiled args ["RuntimeText"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .text_events
            .push(crystal_core::state::ScriptTextRuntimeEvent {
                command: "jumptext".to_string(),
                kind: crystal_core::state::ScriptTextRuntimeKind::Write,
                text_label: Some("RuntimeText".to_string()),
                face_player: true,
                closes_text: true,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 13,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved text event flags must match generated command behavior");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.text_events[0].source_script RuntimePayloadScript:13 command jumptext has face_player true, expected false"
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_text_label = Some("OtherRuntimeText".to_string());
        state.script_runtime.pending_text_wait = Some(crystal_core::state::ScriptTextWait {
            command: "jumptext".to_string(),
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 13,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending text wait payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_text_wait.source_script RuntimePayloadScript:13 args ["OtherRuntimeText"] do not match compiled args ["RuntimeText"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_yes_no = Some(crystal_core::state::ScriptYesNoPrompt {
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 15,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved yes/no prompt must point at a compiled yesorno command");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.pending_yes_no.source_script RuntimePayloadScript:15 command yesorno does not match compiled command waitbutton"
        ));

        let mut state = GameState::default();
        state.script_runtime.script_ended = Some(crystal_core::state::ScriptEndState {
            callback: false,
            just_battled_guard: false,
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 16,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved script end flags must match compiled end command");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.script_ended.source_script RuntimePayloadScript:16 command end does not match compiled command endcallback"
        ));

        let mut state = GameState::default();
        state.script_runtime.script_ended = Some(crystal_core::state::ScriptEndState {
            callback: true,
            just_battled_guard: true,
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 16,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved script end cannot be both callback and guarded");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.script_ended.source_script RuntimePayloadScript:16 cannot be both callback and just_battled_guard"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .control_events
            .push(crystal_core::state::ScriptControlRuntimeEvent {
                kind: crystal_core::state::ScriptControlRuntimeKind::Continue,
                target_script: Some("RuntimeScript".to_string()),
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("continued control events cannot carry a target script");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.control_events[0].source_script RuntimeScript:0 continued control event has unexpected target_script"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .control_events
            .push(crystal_core::state::ScriptControlRuntimeEvent {
                kind: crystal_core::state::ScriptControlRuntimeKind::Jump,
                target_script: None,
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("jump control events must carry a target script");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.control_events[0].source_script RuntimeScript:0 control event is missing target_script"
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_screen_fade = Some(crystal_core::state::ScriptScreenFade {
            color: crystal_core::state::ScriptFadeColor::Black,
            direction: crystal_core::state::ScriptFadeDirection::Out,
            frames: 8,
            source_script: "FadeOutToWhite".to_string(),
            command_index: 0,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved pending screen fade color must match special routine");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.pending_screen_fade.source_script FadeOutToWhite color Black does not match White"
        ));

        let mut state = GameState::default();
        state.script_runtime.graphics_events.push(
            crystal_core::state::ScriptGraphicsRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptGraphicsRuntimeKind::ScreenFade,
                color: Some(crystal_core::state::ScriptFadeColor::White),
                direction: Some(crystal_core::state::ScriptFadeDirection::Out),
                frames: Some(4),
                source_script: "FadeOutToWhite".to_string(),
                command_index: 0,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved graphics screen fade frames must match special routine");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.graphics_events[0].source_script FadeOutToWhite frames 4 does not match 8"
        ));

        let mut state = GameState::default();
        state.script_runtime.graphics_events.push(
            crystal_core::state::ScriptGraphicsRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptGraphicsRuntimeKind::ClearTilemap,
                color: Some(crystal_core::state::ScriptFadeColor::White),
                direction: None,
                frames: None,
                source_script: "ClearTilemap".to_string(),
                command_index: 0,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("non-fade graphics events cannot carry fade payload");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.graphics_events[0].source_script ClearTilemap:0 graphics event has unexpected fade payload"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .money_events
            .push(crystal_core::state::ScriptMoneyRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptMoneyRuntimeKind::DisplayMoneyAndCoinBalance,
                money: 100,
                coins: Some(7),
                source_script: "PlaceMoneyTopRight".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved money event kind must match special routine");
        let error = format!("{error:#}");
        assert!(error.contains(
            "saved script_runtime.money_events[0].source_script PlaceMoneyTopRight kind DisplayMoneyAndCoinBalance does not match PlaceMoneyTopRight"
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .money_events
            .push(crystal_core::state::ScriptMoneyRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptMoneyRuntimeKind::PlaceMoneyTopRight,
                money: 100,
                coins: Some(7),
                source_script: "PlaceMoneyTopRight".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("place-money events cannot carry coins");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.money_events[0].source_script PlaceMoneyTopRight:0 money event has unexpected coins"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .money_events
            .push(crystal_core::state::ScriptMoneyRuntimeEvent {
                command: "special".to_string(),
                kind: crystal_core::state::ScriptMoneyRuntimeKind::DisplayCoinCaseBalance,
                money: 100,
                coins: Some(7),
                source_script: "DisplayCoinCaseBalance".to_string(),
                command_index: 0,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("coin-case money display stores zero money");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.money_events[0].source_script DisplayCoinCaseBalance:0 money 100 does not match 0"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .pending_delays
            .push(crystal_core::state::ScriptRuntimeDelay {
                command: "pause".to_string(),
                frames: 12,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 17,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved delay frames must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_delays[0].source_script RuntimePayloadScript:17 args ["12"] do not match compiled args ["15"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_earthquakes.push(
            crystal_core::state::ScriptRuntimeEarthquake {
                parameter: 72,
                shake_frames: 71,
                sleep_frames: 8,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 18,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved earthquake shake frames must be derived from parameter");
        let error = format!("{error:#}");
        assert!(
            error.contains("pending_earthquakes[0].shake_frames 71 must equal parameter 72"),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.pending_earthquakes.push(
            crystal_core::state::ScriptRuntimeEarthquake {
                parameter: 70,
                shake_frames: 70,
                sleep_frames: 6,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 18,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved earthquake parameter must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_earthquakes[0].source_script RuntimePayloadScript:18 args ["70"] do not match compiled args ["72"]"#
        ));

        let mut state = GameState::default();
        state
            .script_runtime
            .pending_emotes
            .push(crystal_core::state::ScriptRuntimeEmote {
                emote: "EMOTE_SHOCK".to_string(),
                object: "RuntimeObject".to_string(),
                duration: 8,
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 19,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved emote payload must match compiled command args");
        let error = format!("{error:#}");
        assert!(error.contains(
            r#"saved script_runtime.pending_emotes[0].source_script RuntimePayloadScript:19 args ["EMOTE_SHOCK", "RuntimeObject", "8"] do not match compiled args ["EMOTE_SHOCK", "RuntimeObject", "16"]"#
        ));

        let mut state = GameState::default();
        state.script_runtime.elevator_floors.push(
            crystal_core::state::ScriptRuntimeElevatorFloor {
                floor: "1F".to_string(),
                warp: 0,
                target_map: "MissingMap".to_string(),
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            },
        );
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("script runtime elevator target map must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains("saved script_runtime.elevator_floors[0].target_map MissingMap"));

        let mut state = GameState::default();
        state.script_runtime.current_music = Some("MUSIC_MISSING".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("current music must exist in pack audio");
        let error = format!("{error:#}");
        assert!(error.contains(
            "save field script_runtime.current_music references missing Music audio id 'MUSIC_MISSING'"
        ));

        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(crystal_core::state::ScriptWarpRequest {
            target_map: "MissingMap".to_string(),
            tile: TilePosition::new(1, 1),
            facing: Some(Direction::Down),
            source_script: "RuntimeScript".to_string(),
            command_index: 1,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending script warp target map must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.pending_script_warp.target_map MissingMap is missing"
            )
        );

        let mut state = GameState::default();
        state.script_runtime.pending_script_warp = Some(crystal_core::state::ScriptWarpRequest {
            target_map: "RuntimeMap".to_string(),
            tile: TilePosition::new(4, 0),
            facing: Some(Direction::Right),
            source_script: "RuntimePayloadScript".to_string(),
            command_index: 8,
        });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending script warp destination tile must fit target map");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.pending_script_warp destination RuntimeMap runtime tile (4, 0) is invalid"
            ),
            "{error}"
        );
        assert!(
            error.contains(
                "runtime player tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds 4x2"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.overworld = OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(4, 0),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("saved active overworld tile must fit map runtime bounds");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved overworld.active tile (4, 0) is outside compiled map RuntimeMap runtime tile bounds 4x2"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state.script_runtime.pending_text_label = Some("MissingText".to_string());
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("pending text label must exist in pack");
        let error = format!("{error:#}");
        assert!(error.contains("saved script_runtime.pending_text_label MissingText is missing"));

        let mut state = GameState::default();
        state
            .script_runtime
            .shop_events
            .push(crystal_core::state::ScriptShopRuntimeEvent {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "RUNTIME_MART".to_string(),
                inventory: vec!["MISSING_ITEM".to_string()],
                source_script: "RuntimePayloadScript".to_string(),
                command_index: 5,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("shop runtime inventory must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.shop_events[0].inventory[0] MISSING_ITEM is missing"
            ),
            "{error}"
        );

        let mut state = GameState::default();
        state
            .script_runtime
            .item_use_events
            .push(crystal_core::state::ItemUseRuntimeEvent {
                item_id: "MISSING_ITEM".to_string(),
                context: "field".to_string(),
                consumed: true,
            });
        let error = runtime
            .save_game(&save_path, state)
            .expect_err("item-use runtime item must exist in pack");
        let error = format!("{error:#}");
        assert!(
            error.contains(
                "saved script_runtime.item_use_events[0].item_id MISSING_ITEM is missing"
            )
        );
        let _ = std::fs::remove_dir_all(root);
    }
