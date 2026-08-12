    use super::*;
    use crystal_core::map::MapConnection;
    use crystal_core::models::{
        BaseStats, Item, MAX_BOX_MONS, PcBox, ability, egg_group, growth_rate,
        item_pocket, pokemon_type,
    };
    use crystal_core::random::{CrystalRandomState, Random};
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

    fn divider_trace_for_sub_values(values: impl IntoIterator<Item = u8>) -> RuntimeDividerTrace {
        let mut previous_sub = 0_u8;
        let mut samples = Vec::new();
        for value in values {
            samples.push(0);
            samples.push(previous_sub.wrapping_sub(value));
            previous_sub = value;
        }
        RuntimeDividerTrace::new(samples)
    }

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
    fn standard_scripts_have_no_label_specific_runtime_dispatch() {
        let pack = AssetRoot::new(repository_root_for_tests())
            .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
            .expect("load regenerated compiled game pack");
        let catalog =
            compiled_standard_script_catalog(pack.data()).expect("standard-script catalog");
        let pointers = catalog
            .get("StdScripts")
            .and_then(Value::as_array)
            .expect("standard-script pointer table");
        let formerly_label_specific_sources = concat!(
            include_str!("../runtime_commands.rs"),
            include_str!("../game_data.rs"),
            include_str!("../../../crystal-bevy/src/lib.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/battle_results.rs"),
        );

        for pointer in pointers {
            let label = pointer
                .get("args")
                .and_then(Value::as_array)
                .and_then(|args| args.first())
                .and_then(Value::as_str)
                .expect("standard-script pointer label");
            assert!(
                !formerly_label_specific_sources.contains(&format!("\"{label}\"")),
                "{label} has been reintroduced into label-specific runtime dispatch"
            );
        }
        for removed_symbol in [
            "ApplyStandardScript",
            "StandardScriptApplied",
            "apply_standard_script",
            "is_runtime_standard_script_target",
            "standard_receive_item",
            "pokecenter_greeting",
            "pokecenter_take",
            "pokecenter_return",
            "town_map_intro",
        ] {
            assert!(
                !formerly_label_specific_sources.contains(removed_symbol),
                "removed StandardScript compatibility path {removed_symbol} was reintroduced"
            );
        }
    }

    #[test]
    fn compiled_standard_script_catalog_requires_exact_pointer_bodies() {
        let pack = AssetRoot::new(repository_root_for_tests())
            .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
            .expect("load regenerated compiled game pack");
        let data = pack.data().clone();
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
        let mut common_interpreter_labels = Vec::new();
        for entry in catalog
            .get("StdScripts")
            .and_then(Value::as_array)
            .expect("pointer table")
        {
            let label = entry
                .get("args")
                .and_then(Value::as_array)
                .and_then(|args| args.first())
                .and_then(Value::as_str)
                .expect("standard-script pointer label");
            let body = catalog
                .get(label)
                .and_then(Value::as_array)
                .expect("standard-script body");
            match standard_script_execution_path(label, body)
                .unwrap_or_else(|error| panic!("{label} must be executable: {error}"))
            {
                StandardScriptExecutionPath::CommonInterpreter => {
                    common_interpreter_labels.push(label)
                }
            }
        }
        assert_eq!(common_interpreter_labels.len(), 52);

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

        let mut unsupported = data.clone();
        let catalog = unsupported
            .story_events
            .iter_mut()
            .find_map(|payload| payload.get_mut("StandardScripts"))
            .and_then(Value::as_object_mut)
            .expect("mutable standard-script catalog");
        catalog.insert(
            "DifficultBookshelfScript".to_string(),
            serde_json::json!([{ "command": "legacy_typed_handler", "args": [] }]),
        );
        let error = validate_compiled_standard_script_catalog(&unsupported)
            .expect_err("pointer without an executable body or typed handler must fail")
            .to_string();
        assert!(
            error.contains("DifficultBookshelfScript has no executable runtime path"),
            "{error}"
        );
        let mut diagnostics = Vec::new();
        verify_standard_script_catalog(&unsupported, &mut diagnostics);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "invalid_runtime_standard_scripts");
        assert_eq!(diagnostics[0].subject, "story_events:StandardScripts");
        assert!(
            diagnostics[0]
                .message
                .contains("DifficultBookshelfScript has no executable runtime path"),
            "{}",
            diagnostics[0].message
        );

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
    fn every_standard_script_pointer_is_a_common_interpreter_source() {
        let pack = AssetRoot::new(repository_root_for_tests())
            .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
            .expect("load regenerated compiled game pack");
        let data = pack.data();
        let catalog = compiled_standard_script_catalog(data).expect("standard-script catalog");
        let pointers = catalog
            .get("StdScripts")
            .and_then(Value::as_array)
            .expect("standard-script pointer table");

        for pointer in pointers {
            let label = pointer
                .get("args")
                .and_then(Value::as_array)
                .and_then(|args| args.first())
                .and_then(Value::as_str)
                .expect("standard-script pointer label");
            let body = catalog
                .get(label)
                .and_then(Value::as_array)
                .expect("standard-script command body");
            assert_eq!(
                standard_script_execution_path(label, body)
                    .unwrap_or_else(|error| panic!("classify {label}: {error}")),
                StandardScriptExecutionPath::CommonInterpreter,
                "{label} still bypasses the compiled command interpreter"
            );
            assert_eq!(
                data.compiled_script_body(label).and_then(Value::as_array),
                Some(body),
                "{label} is not addressable by the compiled command interpreter"
            );
        }
    }

    #[test]
    fn bug_contest_runtime_accepts_only_the_canonical_battle_type() {
        assert!(battle_type_guarantees_escape("BATTLETYPE_CONTEST"));
        for alias in ["CONTEST", "BATTLETYPE_BUG_CONTEST", "BATTLETYPE_PARK"] {
            assert!(
                !battle_type_guarantees_escape(alias),
                "noncanonical battle type {alias} must not get contest escape behavior"
            );
        }

        let production_source = concat!(
            include_str!("../game_data.rs"),
            include_str!("../mutation_protocol.rs"),
        );
        for alias in ["CONTEST", "BATTLETYPE_BUG_CONTEST", "BATTLETYPE_PARK"] {
            assert!(
                !production_source.contains(&format!("\"{alias}\"")),
                "runtime production paths must not accept {alias}"
            );
        }
    }

    #[test]
    fn native_vendor_runtime_file_compilation_covers_every_renderer_dependency() {
        let root = AssetRoot::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../..")
                .canonicalize()
                .expect("repository root"),
        );
        let files = compile_runtime_files(&root).expect("compile native runtime files");
        validate_compiled_runtime_files(&files).expect("complete native runtime file bundle");
        for &key in REQUIRED_VENDOR_RUNTIME_FILE_KEYS {
            assert!(
                files.get(key).is_some_and(|bytes| !bytes.is_empty()),
                "compiled native runtime files must include {key}"
            );
        }
    }

    #[test]
    fn native_vendor_runtime_file_inventory_matches_every_production_source_read() {
        fn call_string_literals(source: &str, needle: &str) -> Vec<String> {
            let mut literals = Vec::new();
            let mut remaining = source;
            while let Some(offset) = remaining.find(needle) {
                remaining = &remaining[offset + needle.len()..];
                let end = remaining
                    .find('"')
                    .unwrap_or_else(|| panic!("unterminated string literal after {needle}"));
                literals.push(remaining[..end].to_string());
                remaining = &remaining[end + 1..];
            }
            literals
        }

        fn source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
            let start = source
                .find(start)
                .unwrap_or_else(|| panic!("production source is missing {start}"));
            let remaining = &source[start..];
            let end = remaining
                .find(end)
                .unwrap_or_else(|| panic!("production source is missing {end}"));
            &remaining[..end]
        }

        fn quoted_string_literals(source: &str) -> Vec<String> {
            source
                .split('"')
                .enumerate()
                .filter_map(|(index, value)| (index % 2 == 1).then(|| value.to_string()))
                .collect()
        }

        let production_source = concat!(
            include_str!("../../../crystal-bevy/src/main.rs"),
            include_str!("../../../crystal-bevy/src/lib.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/deterministic_session.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/field_travel.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/trainer_card.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/title_menu.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/credits.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/script_callbacks.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/economy.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/battle_messages.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/battle_results.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/battle_entry.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/menu_rendering.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/render_mod.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/overworld_rendering.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/start_menu.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/bitmap_font.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/graphics_assets.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/field_pack.rs"),
            include_str!("../../../crystal-bevy/src/bevy_shell/intro_renderer.rs"),
        );
        let vendor_roots = call_string_literals(production_source, "resolve_vendor(\"");
        assert_eq!(
            production_source.matches("resolve_vendor(").count(),
            vendor_roots.len(),
            "every production vendor resolver must use an auditable literal path"
        );

        let mut source_dependencies = std::collections::BTreeSet::new();
        for relative in &vendor_roots {
            if !matches!(
                relative.as_str(),
                "gfx/card_flip" | "gfx/slots" | "gfx/unown_puzzle"
            ) {
                source_dependencies.insert(format!("vendor/pokecrystal/{relative}"));
            }
        }

        let card_flip = source_between(
            production_source,
            "fn load_card_flip_render_sources(",
            "fn render_visible_card_flip_frame(",
        );
        for file in call_string_literals(card_flip, "root.join(\"") {
            source_dependencies.insert(format!("vendor/pokecrystal/gfx/card_flip/{file}"));
        }

        let slots = source_between(
            production_source,
            "fn load_slot_machine_render_sources(",
            "fn render_visible_slot_machine_frame(",
        );
        for file in call_string_literals(slots, "root.join(\"") {
            source_dependencies.insert(format!("vendor/pokecrystal/gfx/slots/{file}"));
        }

        let unown = source_between(
            production_source,
            "fn load_unown_puzzle_render_sources(",
            "fn render_visible_unown_puzzle_frame(",
        );
        for file in call_string_literals(unown, "root.join(\"") {
            source_dependencies.insert(format!("vendor/pokecrystal/gfx/unown_puzzle/{file}"));
        }
        let puzzle_ids = source_between(
            unown,
            "for puzzle_id in [",
            "] {",
        );
        for puzzle_id in quoted_string_literals(puzzle_ids) {
            source_dependencies.insert(format!(
                "vendor/pokecrystal/gfx/unown_puzzle/{puzzle_id}.png"
            ));
        }

        let verified_dependencies = REQUIRED_VENDOR_RUNTIME_FILE_KEYS
            .iter()
            .map(|key| (*key).to_string())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            source_dependencies, verified_dependencies,
            "pack verification must require exactly every production vendor filesystem read"
        );
    }

    #[test]
    fn regenerated_core_pack_embeds_exact_native_vendor_dependencies() {
        let root = AssetRoot::new(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../..")
                .canonicalize()
                .expect("repository root"),
        );
        let pack = root
            .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
            .expect("load regenerated core pack");
        validate_compiled_runtime_files(pack.runtime_files())
            .expect("regenerated pack native runtime file bundle");
        for &key in REQUIRED_VENDOR_RUNTIME_FILE_KEYS {
            assert!(
                pack.runtime_files()
                    .get(key)
                    .is_some_and(|bytes| !bytes.is_empty()),
                "regenerated core pack must include {key}"
            );
        }
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
                        { "command": "add_stdscript", "args": ["DifficultBookshelfScript"] }
                    ],
                    "DifficultBookshelfScript": [
                        { "command": "farjumptext", "args": ["DifficultBookshelfText"] }
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
        data.roaming_pokemon = roaming_catalog_for_tests("RATTATA", "RATTATA");
        data.roaming_pokemon.init_writes[0].level = 5;
        data.roaming_pokemon.init_writes[1].level = 5;
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
            encounters: bug_contest_encounters_for_tests(),
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
        data.magikarp_lengths = magikarp_lengths_for_tests();
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
                roaming_slot: None,
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
        let mut player = crystal_core::models::Pokemon::new_for_tests(
            species(),
            5,
            crystal_core::models::Dv::default(),
        );
        let enemy = player.clone();
        player.hp = 0;
        let mut state = GameState {
            last_spawn_identifier: Some(2),
            money: 100,
            battle_pay_day_money: 50,
            script_runtime: ScriptRuntimeMemory {
                blackout_mod: Some("DARK_CAVE".to_string()),
                ..ScriptRuntimeMemory::default()
            },
            battle: BattleMemory::StaticWild {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
                origin_map_name: "Route30".to_string(),
                species: enemy.species.id.clone(),
                level: enemy.level,
                source_script: "RockSmashScript".to_string(),
                startbattle_command_index: 12,
                resume_command_index: 13,
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        state.sync_party_from_storage();
        state.script_runtime.next_script = Some(crystal_core::state::ScriptLocation {
            origin_map_name: "Route30".to_string(),
            script: "RockSmashScript".to_string(),
        });
        state.script_runtime.deferred_scripts.push(crystal_core::state::ScriptLocation {
            origin_map_name: "Route30".to_string(),
            script: "RockSmashScript".to_string(),
        });
        state.script_runtime.call_stack.push(crystal_core::state::ScriptReturnFrame {
            origin_map_name: "Route30".to_string(),
            source_script: "RockSmashScript".to_string(),
            next_command_index: 13,
        });
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

        let mut draw_state = state.clone();
        crystal_core::battle::start::deactivate_battle_after_draw(&mut draw_state);
        let mut draw_session = session.clone();
        let draw_before = (draw_state.clone(), draw_session.clone());
        let error = data
            .apply_runtime_mutation_command(
                &mut draw_state,
                &mut draw_session,
                RuntimeMutationCommand::ResolveBlackoutToLastSpawn,
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("DRAW terminal cannot be consumed as a whiteout");
        assert!(error.to_string().contains("terminal result 0x02"), "{error:#}");
        assert_eq!((draw_state, draw_session), draw_before);

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
        assert_eq!(state.battle_result, 1);
        assert!(state.pending_static_wild_terminal.is_none());
        assert_eq!(state.money, 50, "loss skips Pay Day before whiteout halves money");
        assert!(state.script_runtime.next_script.is_none());
        assert!(state.script_runtime.deferred_scripts.is_empty());
        assert!(state.script_runtime.call_stack.is_empty());
        assert!(state.script_runtime.command_queue.is_empty());
        assert!(state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .all(|pokemon| pokemon.hp == pokemon.max_hp));
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
    fn active_battle_escape_item_uses_draw_result_and_skips_pay_day() {
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
                roaming_slot: None,
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            money: 100,
            battle_pay_day_money: 50,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        state
            .bag
            .add_item(&data.items["POKE_DOLL"], 1)
            .expect("add escape item");

        let outcome = data
            .use_bag_item_to_escape_active_wild_battle(&mut state, "POKE_DOLL")
            .expect("Poke Doll exits the wild battle");

        assert!(outcome.escaped);
        assert_eq!(state.bag.quantity(&data.items["POKE_DOLL"]), 0);
        assert_eq!(state.script_runtime.item_use_events.len(), 1);
        assert!(matches!(state.battle, BattleMemory::Inactive));
        assert_eq!(state.battle_result, 2);
        assert_eq!(state.money, 100, "DRAW skips CheckPayDay");
        assert_eq!(state.battle_pay_day_money, 0);
    }

    #[test]
    fn active_wild_battle_run_uses_draw_result_and_skips_pay_day() {
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
                roaming_slot: None,
                enemy_pokemon: enemy.clone(),
                enemy_party: vec![enemy],
            },
            battle_active_party_index: Some(0),
            battle_active_enemy_party_index: Some(0),
            battle_escape_attempts: u8::MAX,
            money: 100,
            battle_pay_day_money: 50,
            rng_seed: 7,
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        let outcome = data
            .resolve_active_wild_battle_run(&mut state)
            .expect("manual RUN succeeds at the saturated attempt count");

        assert!(outcome.escaped);
        assert!(matches!(state.battle, BattleMemory::Inactive));
        assert_eq!(state.battle_result, 2);
        assert_eq!(state.money, 100, "DRAW skips CheckPayDay");
        assert_eq!(state.battle_pay_day_money, 0);
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
                roaming_slot: None,
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

        let mut divider = crystal_core::random::ReplayDivider::new([]);
        let error = data
            .claim_active_wild_battle_rewards(&mut state, TimeOfDay::Day, &mut divider)
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
                roaming_slot: None,
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
    fn active_wild_capture_rejects_full_current_box_without_routing_to_an_empty_box() {
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
                roaming_slot: None,
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
        state.current_pc_box = 0;
        let mut current_box = PcBox::new(state.current_pc_box);
        for slot in 0..MAX_BOX_MONS {
            current_box.set_slot(slot, Some(pokemon.clone()));
        }
        state.storage.pc_boxes.push(current_box);
        state.storage.pc_boxes.push(PcBox::new(1));
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
        assert_eq!(state.storage.pc_boxes[1].filled_slots(), 0);
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
                roaming_slot: None,
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
            .complete_active_wild_capture(
                &mut state,
                &outcome,
                None,
                &mut crystal_core::random::ReplayDivider::new([]),
            )
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
                roaming_slot: None,
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
            .complete_active_wild_capture(
                &mut party_state,
                &outcome,
                Some("SPARKY"),
                &mut crystal_core::random::ReplayDivider::new([]),
            )
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
            .complete_active_wild_capture(
                &mut pc_state,
                &outcome,
                Some("BOXMON"),
                &mut crystal_core::random::ReplayDivider::new([]),
            )
            .expect("complete named PC capture");
        let stored = pc_completion.stored.as_ref().unwrap();
        assert_eq!(stored.pokemon.nickname, "BOXMON");
        assert_eq!(pc_state.storage.pc_boxes[0].pokemon[0].as_ref().unwrap().nickname, "BOXMON");
        assert_eq!(pc_state.storage.pc_boxes[0].nicknames[0], "BOXMON");
    }

    #[test]
    fn move_pokemon_without_mail_inserts_at_the_asm_box_cursor() {
        fn named_pokemon(name: &str) -> crystal_core::models::Pokemon {
            let mut pokemon = crystal_core::models::Pokemon::new_for_tests(
                species(),
                20,
                crystal_core::models::Dv::default(),
            );
            pokemon.nickname = name.to_string();
            pokemon
        }

        let data = GameDataSet::default();
        let mut state = GameState::default();
        let mut source = PcBox::new(0);
        assert!(source.add_pokemon(named_pokemon("A")));
        assert!(source.add_pokemon(named_pokemon("B")));
        let mut target = PcBox::new(1);
        assert!(target.add_pokemon(named_pokemon("C")));
        assert!(target.add_pokemon(named_pokemon("D")));
        state.storage.pc_boxes = vec![source, target];
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "PcMoveTest".to_string(),
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

        let applied = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::MovePcPokemonWithoutMail(RuntimePcMoveCommand {
                    source_box: 0,
                    source_slot: 0,
                    target_box: 1,
                    target_slot: 1,
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("move inserts before the selected destination");
        let RuntimeMutationResult::PcPokemonMoved(outcome) = applied.result else {
            panic!("expected PC Pokemon move result");
        };
        assert_eq!(outcome.target_slot, 1);
        assert_eq!(
            state.storage.pc_boxes[0]
                .pokemon
                .iter()
                .flatten()
                .map(|pokemon| pokemon.nickname.as_str())
                .collect::<Vec<_>>(),
            vec!["B"]
        );
        assert_eq!(
            state.storage.pc_boxes[1]
                .pokemon
                .iter()
                .flatten()
                .map(|pokemon| pokemon.nickname.as_str())
                .collect::<Vec<_>>(),
            vec!["C", "A", "D"]
        );
        state.storage.validate_metadata().expect("compact moved boxes");
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
    fn deferred_level_evolution_queues_same_level_target_move_when_moves_are_full() {
        let mut data = GameDataSet::default();
        let mut dragonair = species();
        dragonair.id = "DRAGONAIR".to_string();
        dragonair.int_id = 148;
        let mut dragonite = species();
        dragonite.id = "DRAGONITE".to_string();
        dragonite.int_id = 149;
        data.pokemon
            .insert(dragonair.id.clone(), dragonair.clone());
        data.pokemon
            .insert(dragonite.id.clone(), dragonite.clone());
        for move_id in ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER", "WING_ATTACK"] {
            data.moves.insert(move_id.to_string(), test_move(move_id));
        }
        data.learnsets.insert(dragonair.id.clone(), Vec::new());
        data.learnsets.insert(
            dragonite.id.clone(),
            vec![crystal_core::systems::learnsets::LearnsetEntry(
                55,
                "WING_ATTACK".to_string(),
            )],
        );
        data.evolutions.0.insert(
            dragonair.id.clone(),
            vec![crystal_core::systems::evolution::EvolutionEntry::level(
                dragonite.id.clone(),
                55,
            )],
        );
        data.evolutions
            .0
            .insert(dragonite.id.clone(), Vec::new());
        let mut pokemon = crystal_core::models::Pokemon::new_for_tests(
            dragonair,
            55,
            crystal_core::models::Dv::default(),
        );
        pokemon.moves = ["WRAP", "LEER", "THUNDER_WAVE", "TWISTER"]
            .into_iter()
            .map(|move_id| crystal_core::models::LearnedMove {
                name: move_id.to_string(),
                current_pp: data.moves[move_id].pp,
                pp_ups: 0,
            })
            .collect();
        let moves_before = pokemon.moves.clone();
        let mut state = GameState::default();
        state.storage.party.pokemon[0] = Some(pokemon);

        let report = data
            .resolve_deferred_level_evolution(&mut state, 0, TimeOfDay::Day)
            .expect("resolve Dragonair evolution");

        let evolved = state.storage.party.pokemon[0].as_ref().expect("Dragonite");
        assert_eq!(evolved.species.id, "DRAGONITE");
        assert_eq!(evolved.moves, moves_before);
        assert_eq!(report.pending_move_learns.len(), 1);
        let pending = state.pending_move_learn.as_ref().expect("Wing Attack prompt");
        assert_eq!(pending.party_index, 0);
        assert_eq!(pending.species_id, "DRAGONITE");
        assert_eq!(pending.level, 55);
        assert_eq!(pending.learned_move.name, "WING_ATTACK");
        assert_eq!(pending.learned_move.current_pp, 35);
        assert!(!pending.defer_level_evolution);
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
            evolution_cancel_snapshot: None,
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
            },
            ScriptedTrainerBattle {
                source_script: "BadTrainerIdScript".to_string(),
                loadtrainer_command_index: 9,
                startbattle_command_index: 10,
                request: TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER JOEY", ""),
            },
            ScriptedTrainerBattle {
                source_script: "BadTrainerClassScript".to_string(),
                loadtrainer_command_index: 11,
                startbattle_command_index: 12,
                request: TrainerBattleRequest::new("YOUNG STER", "YOUNGSTER_JOEY", ""),
            },
        ];
        module.scripted_wild_battles = vec![
            ScriptedWildBattle {
                source_script: "WildCaseScript".to_string(),
                loadwildmon_command_index: 5,
                startbattle_command_index: 6,
                request: StaticWildBattleRequest::new(known_species_id.to_lowercase(), 10),
            },
            ScriptedWildBattle {
                source_script: "WildZeroScript".to_string(),
                loadwildmon_command_index: 7,
                startbattle_command_index: 8,
                request: StaticWildBattleRequest::new(known_species_id, 0),
            },
            ScriptedWildBattle {
                source_script: "WildMalformedScript".to_string(),
                loadwildmon_command_index: 13,
                startbattle_command_index: 14,
                request: StaticWildBattleRequest::new("HO OT", 10),
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
            },
            ScriptedTrainerBattle {
                source_script: "BattleScript".to_string(),
                loadtrainer_command_index: 3,
                startbattle_command_index: 2,
                request: TrainerBattleRequest::new("YOUNGSTER", "YOUNGSTER_JOEY", ""),
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
    fn rock_mon_encounter_runtime_command_replays_exactly_and_is_atomic() {
        let standard_scripts: Value = serde_json::from_str(
            &std::fs::read_to_string(
                repository_root_for_tests()
                    .join("apps/web/assets/data/story_events/StandardScripts.json"),
            )
            .expect("read StandardScripts export"),
        )
        .expect("parse StandardScripts export");
        let mut data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route40", "ROUTE_40", None)]),
            story_events: vec![standard_scripts],
            ..GameDataSet::default()
        };
        data.materialize_global_scripts()
            .expect("materialize exact global Rock Smash scripts");
        let mut state = GameState {
            random_state: CrystalRandomState {
                add: 0x12,
                sub: 0x34,
            },
            ..GameState::default()
        };
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "Route40".to_string(),
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
        let command = RuntimeScriptCommandRef::new("Route40", "RockSmashScript", 8);

        let no_table = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ResolveRockMonEncounter(
                    RuntimeRockMonEncounterCommand {
                        command: command.clone(),
                        divider_trace: RuntimeDividerTrace::new([]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("a map absent from RockMonMaps is an exact zero-read miss");
        let RuntimeMutationResult::RockMonEncounterResolved(no_table) = no_table.result else {
            panic!("expected RockMonEncounter result");
        };
        assert_eq!(no_table.chance_roll, None);
        assert_eq!(state.random_state, CrystalRandomState { add: 0x12, sub: 0x34 });
        assert_eq!(state.script_runtime.memory.get("wTempWildMonSpecies"), Some(&"0".to_string()));
        assert_eq!(state.script_runtime.memory.get("wCurPartyLevel"), Some(&"0".to_string()));

        let before_no_table_tail = state.clone();
        let no_table_tail = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ResolveRockMonEncounter(
                    RuntimeRockMonEncounterCommand {
                        command: command.clone(),
                        divider_trace: RuntimeDividerTrace::new([99]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("a zero-read miss must reject an injected divider tail");
        assert!(no_table_tail.to_string().contains("1 unconsumed samples after 0 reads"));
        assert_eq!(state, before_no_table_tail);

        data.field_encounters.insert(
            "Route40".to_string(),
            FieldEncounterData::for_crystal(
                "Route40",
                None,
                Some(FieldEncounterTable {
                    common: vec![
                        FieldEncounterEntry {
                            weight: 90,
                            species: "KRABBY".to_string(),
                            level: 15,
                            sleep_turns_by_time: BTreeMap::new(),
                        },
                        FieldEncounterEntry {
                            weight: 10,
                            species: "SHUCKLE".to_string(),
                            level: 15,
                            sleep_turns_by_time: BTreeMap::new(),
                        },
                    ],
                    rare: Vec::new(),
                }),
            ),
        );
        state.random_state = CrystalRandomState::default();

        let before_short = state.clone();
        let short = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ResolveRockMonEncounter(
                    RuntimeRockMonEncounterCommand {
                        command: command.clone(),
                        divider_trace: RuntimeDividerTrace::new([255, 0, 89]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("a truncated conditional entry draw must reject atomically");
        assert!(short.to_string().contains("divider replay exhausted after 3 samples"));
        assert_eq!(state, before_short);

        let before_miss_tail = state.clone();
        let miss_tail = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ResolveRockMonEncounter(
                    RuntimeRockMonEncounterCommand {
                        command: command.clone(),
                        divider_trace: RuntimeDividerTrace::new([3, 0, 77]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("a chance miss must reject an unused entry-roll byte");
        assert!(miss_tail.to_string().contains("1 unconsumed samples after 2 reads"));
        assert_eq!(state, before_miss_tail);

        let hit = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ResolveRockMonEncounter(
                    RuntimeRockMonEncounterCommand {
                        command,
                        divider_trace: RuntimeDividerTrace::new([255, 0, 89, 0]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("exact conditional RockMonEncounter trace applies once");
        let RuntimeMutationResult::RockMonEncounterResolved(hit) = hit.result else {
            panic!("expected RockMonEncounter result");
        };
        assert_eq!(hit.chance_roll, Some(0));
        assert_eq!(hit.entry_roll, Some(90));
        assert_eq!(
            state.script_runtime.memory.get("wTempWildMonSpecies"),
            Some(&"SHUCKLE".to_string())
        );
        assert_eq!(
            state.script_runtime.memory.get("wCurPartyLevel"),
            Some(&"15".to_string())
        );
        assert_eq!(state.random_state, CrystalRandomState { add: 90, sub: 255 });
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
            roaming_slot: None,
            resolved: Some(ResolvedWildEncounter {
                level: 5,
                encounter: WildEncounter {
                    level: 5,
                    species: "MISSINGNO".to_string(),
                },
                slot: 0,
            }),
            repelled_by: None,
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
    fn fishing_and_headbutt_starts_preserve_pret_battle_types() {
        let mut data = GameDataSet {
            maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
            ..GameDataSet::default()
        };
        add_complete_runtime_pack_fixture(&mut data);
        let landmark = data
            .pokegear_landmarks
            .landmarks
            .first_mut()
            .expect("test landmark");
        landmark.region = "JOHTO".to_string();
        data.audio.push(
            ModpackAudioAsset::pcm(
                "MUSIC_JOHTO_WILD_BATTLE",
                "content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE.pcm",
                ModpackAudioKind::Music,
                test_pcm_format(),
            )
            .expect("wild music fixture"),
        );
        data.audio.push(
            ModpackAudioAsset::pcm(
                "MUSIC_JOHTO_WILD_BATTLE_NIGHT",
                "content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE_NIGHT.pcm",
                ModpackAudioKind::Music,
                test_pcm_format(),
            )
            .expect("night wild music fixture"),
        );
        let mut caterpie = data.pokemon["NEW_MON"].clone();
        caterpie.id = "CATERPIE".to_string();
        caterpie.int_id = 10;
        data.pokemon.insert(caterpie.id.clone(), caterpie);
        data.learnsets.insert(
            "CATERPIE".to_string(),
            data.learnsets["NEW_MON"].clone(),
        );
        data.field_encounters.insert(
            "Route29".to_string(),
            FieldEncounterData::for_crystal(
                "Route29",
                Some(FieldEncounterTable {
                    common: vec![FieldEncounterEntry {
                        weight: 100,
                        species: "CATERPIE".to_string(),
                        level: 5,
                        sleep_turns_by_time: BTreeMap::from([(TimeOfDay::Night, 7)]),
                    }],
                    rare: vec![FieldEncounterEntry {
                        weight: 100,
                        species: "CATERPIE".to_string(),
                        level: 5,
                        sleep_turns_by_time: BTreeMap::from([(TimeOfDay::Night, 7)]),
                    }],
                }),
                None,
            ),
        );

        let mut fishing_state = GameState::default();
        let fishing = data
            .start_fishing_battle(
                &mut fishing_state,
                "Route29",
                TilePosition::new(0, 0),
                WildEncounter {
                    level: 5,
                    species: "NEW_MON".to_string(),
                },
                TimeOfDay::Day,
                0,
                0,
            )
            .expect("fishing battle");
        assert_eq!(fishing.battle_type, "BATTLETYPE_FISH");
        assert!(matches!(
            fishing_state.battle,
            BattleMemory::Wild { ref battle_type, .. } if battle_type == "BATTLETYPE_FISH"
        ));

        let headbutt_roll = crystal_core::world::encounters::FieldEncounterRoll {
            map_name: "Route29".to_string(),
            kind: crystal_core::world::encounters::FieldEncounterKind::Headbutt,
            target_tile_x: 0,
            target_tile_y: 0,
            score: Some(1),
            chance_roll: 0,
            entry_roll: Some(0),
            resolved: Some(ResolvedWildEncounter {
                level: 5,
                encounter: WildEncounter {
                    level: 5,
                    species: "CATERPIE".to_string(),
                },
                slot: 0,
            }),
        };
        let mut tree_state = GameState::default();
        tree_state.time.time_of_day = TimeOfDay::Night;
        let tree = data
            .start_field_encounter_battle(&mut tree_state, &headbutt_roll)
            .expect("headbutt battle")
            .expect("resolved headbutt encounter");
        assert_eq!(tree.battle_type, "BATTLETYPE_TREE");
        assert_eq!(tree.enemy_pokemon.status.as_deref(), Some("SLEEP"));
        assert_eq!(tree.enemy_pokemon.sleep_turns, 7);
        assert!(matches!(
            tree_state.battle,
            BattleMemory::Wild {
                ref battle_type,
                ref enemy_pokemon,
                ..
            } if battle_type == "BATTLETYPE_TREE"
                && enemy_pokemon.status.as_deref() == Some("SLEEP")
                && enemy_pokemon.sleep_turns == 7
        ));

        let mut day_tree_state = GameState::default();
        day_tree_state.time.time_of_day = TimeOfDay::Day;
        let day_tree = data
            .start_field_encounter_battle(&mut day_tree_state, &headbutt_roll)
            .expect("day headbutt battle")
            .expect("resolved day headbutt encounter");
        assert_eq!(day_tree.enemy_pokemon.status, None);
        assert_eq!(day_tree.enemy_pokemon.sleep_turns, 0);
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

        let mut divider = ReplayDivider::new([]);
        let mut rng = CrystalRandom::new(state.random_state, &mut divider);
        let error = data
            .check_wild_encounter_after_step(&mut state, &session, &mut rng)
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
    fn new_game_spawn_projects_all_roaming_slots_to_catalog_inactive_before_partial_init() {
        let mut module = test_map_module("Route29", "ROUTE_29", None);
        module.blocks = vec![1];
        let spawn = test_runtime_spawn_point(0, "Route29");
        let catalog = roaming_catalog_for_tests("RAIKOU", "ENTEI");
        let mut raikou = species();
        raikou.id = "RAIKOU".to_string();
        let mut entei = species();
        entei.id = "ENTEI".to_string();
        let data = GameDataSet {
            pokemon: [
                ("RAIKOU".to_string(), raikou),
                ("ENTEI".to_string(), entei),
            ]
            .into_iter()
            .collect(),
            maps: map_payload(vec![module]),
            tilesets: [("johto".to_string(), test_tileset_definition())]
                .into_iter()
                .collect(),
            runtime_map_metadata: [(
                "ROUTE_29".to_string(),
                test_runtime_map_metadata("ROUTE_29", "Route29"),
            )]
            .into_iter()
            .collect(),
            roaming_pokemon: catalog.clone(),
            special_routines: special_routine_rules(["InitRoamMons"]),
            ..GameDataSet::default()
        };

        let (mut state, _) = data
            .start_overworld_session_from_spawn(&spawn, &BTreeSet::new())
            .expect("start exact new-game spawn");
        assert!(state.roaming_pokemon.iter().all(|roaming| {
            roaming.species.is_none()
                && roaming.map_group == catalog.inactive_map.map_group
                && roaming.map_number == catalog.inactive_map.map_number
                && roaming.level == 0
                && roaming.hp == 0
                && roaming.dvs_be == [0, 0]
        }));

        data.apply_special_routine(&mut state, "InitRoamMons", &BTreeSet::new())
            .expect("apply partial source InitRoamMons writes");
        assert_eq!(state.roaming_pokemon[0].species.as_deref(), Some("RAIKOU"));
        assert_eq!(state.roaming_pokemon[1].species.as_deref(), Some("ENTEI"));
        assert_eq!(state.roaming_pokemon[2].species, None);
        assert_eq!(
            (
                state.roaming_pokemon[2].map_group,
                state.roaming_pokemon[2].map_number,
            ),
            (
                catalog.inactive_map.map_group,
                catalog.inactive_map.map_number,
            )
        );
    }

    #[test]
    fn saved_roaming_battle_requires_selected_slot_map_and_normal_battles_never_refind_roamers() {
        let dvs = Dv::from_non_hp(1, 2, 3, 4);
        let enemy = Pokemon::new_for_tests(species(), 40, dvs);
        let mut other_metadata = test_runtime_map_metadata("OTHER_MAP", "OtherMap");
        other_metadata.map_id = 2;
        let data = GameDataSet {
            roaming_pokemon: roaming_catalog_for_tests("NEW_MON", "NEW_MON"),
            runtime_map_metadata: [("OTHER_MAP".to_string(), other_metadata)]
                .into_iter()
                .collect(),
            ..GameDataSet::default()
        };
        let roaming = crystal_core::state::RoamingPokemonState {
            species: Some("NEW_MON".to_string()),
            level: 40,
            map_group: 1,
            map_number: 1,
            hp: enemy.hp as u8,
            dvs_be: [0x12, 0x34],
        };

        let roaming_error = data
            .validate_saved_roaming_battle_origin_references("OtherMap", 0, &roaming, &enemy)
            .expect_err("selected roaming slot must occupy the exact saved battle map")
            .to_string();
        assert!(
            roaming_error.contains(
                "saved roaming battle slot 0 location 1/1 does not match battle map OtherMap location 1/2"
            ),
            "{roaming_error}"
        );

        let normal_error = data
            .validate_saved_wild_battle_origin_references(
                "BATTLETYPE_NORMAL",
                "OtherMap",
                &enemy,
            )
            .expect_err("normal wild origins must not refind a matching roaming species and level")
            .to_string();
        assert!(
            normal_error.contains(
                "saved battle.wild OtherMap encounter NEW_MON:40 is missing from compiled wild encounter sources"
            ),
            "{normal_error}"
        );
    }

    #[test]
    fn runtime_field_encounter_commands_reject_unused_payload_fields() {
        let headbutt_error = serde_json::from_value::<RuntimeHeadbuttFieldEncounterCommand>(
            serde_json::json!({
                "party_index": 0,
                "player_id": 12345,
                "rng_seed_after": 7,
                "surface": "grass"
            }),
        )
        .expect_err("HEADBUTT command must not carry a surface payload");
        assert!(
            headbutt_error.to_string().contains("unknown field `surface`"),
            "{headbutt_error}"
        );

        let sweet_scent_error = serde_json::from_value::<RuntimeSweetScentFieldMoveCommand>(
            serde_json::json!({
                "party_index": 0,
                "divider_trace": { "samples": [] },
                "player_id": 12345
            }),
        )
        .expect_err("SWEET_SCENT command must not carry player_id");
        assert!(
            sweet_scent_error
                .to_string()
                .contains("unknown field `player_id`"),
            "{sweet_scent_error}"
        );

        let missing_trace = serde_json::from_value::<RuntimeSweetScentFieldMoveCommand>(
            serde_json::json!({ "party_index": 0 }),
        )
        .expect_err("Sweet Scent commands must declare their exact divider trace");
        assert!(
            missing_trace.to_string().contains("missing field `divider_trace`"),
            "{missing_trace}"
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
    fn runtime_bug_contest_commands_have_no_rank_authority_and_use_exact_rng_payloads() {
        let judging_trace = RuntimeDividerTrace::new([1, 2]);
        let judging = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::Judge,
            divider_trace: Some(judging_trace.clone()),
        };
        assert_eq!(
            runtime_bug_contest_divider_trace(&judging).expect("judge divider trace"),
            Some(&judging_trace)
        );
        assert_eq!(runtime_bug_contest_action_name(judging.action), "judge");

        let injected_error = serde_json::from_value::<RuntimeBugContestCommand>(
            serde_json::json!({
                "action": "judge",
                "rank": 3,
                "divider_trace": { "samples": [1, 2] }
            }),
        )
        .expect_err("Bug Contest commands must not expose a fabricated rank authority");
        assert!(
            injected_error.to_string().contains("unknown field `rank`"),
            "{injected_error}"
        );

        let selecting_trace = RuntimeDividerTrace::new([3, 4]);
        let selecting = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::SelectContestants,
            divider_trace: Some(selecting_trace.clone()),
        };
        assert_eq!(
            runtime_bug_contest_divider_trace(&selecting).expect("select divider trace"),
            Some(&selecting_trace)
        );

        let missing_divider_trace = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::SelectContestants,
            divider_trace: None,
        };
        let missing_trace_error = runtime_bug_contest_divider_trace(&missing_divider_trace)
            .expect_err("select contestants must carry the authoritative divider trace");
        assert!(
            format!("{missing_trace_error:#}")
                .contains("Bug Contest select_contestants command requires divider_trace"),
            "{missing_trace_error:#}"
        );

        let unused_divider_trace = RuntimeBugContestCommand {
            action: RuntimeBugContestAction::GiveParkBalls,
            divider_trace: Some(RuntimeDividerTrace::new([1, 2])),
        };
        let unused_trace_error = runtime_bug_contest_divider_trace(&unused_divider_trace)
            .expect_err("non-RNG bug contest actions must reject divider traces");
        assert!(
            format!("{unused_trace_error:#}")
                .contains("Bug Contest give_park_balls command must not declare divider_trace"),
            "{unused_trace_error:#}"
        );
    }

    #[test]
    fn bug_contest_select_contestants_requires_an_atomic_exact_divider_trace() {
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
                encounters: bug_contest_encounters_for_tests(),
            }),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        let before = state.clone();
        let audio_ids = BTreeSet::new();
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

        let exhausted = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand {
                    action: RuntimeBugContestAction::SelectContestants,
                    divider_trace: Some(RuntimeDividerTrace::new([0])),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exhausted Bug Contest divider trace must reject");

        assert!(
            exhausted
                .to_string()
                .contains("divider replay exhausted after 1 samples"),
            "{exhausted}"
        );
        assert_eq!(state, before);

        let mut trace_with_tail = divider_trace_for_sub_values([0]);
        trace_with_tail.samples.push(77);
        let unused_tail = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand {
                    action: RuntimeBugContestAction::SelectContestants,
                    divider_trace: Some(trace_with_tail),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("unused Bug Contest divider tail must reject");
        assert!(
            unused_tail
                .to_string()
                .contains("divider trace has 1 unconsumed samples after 2 reads"),
            "{unused_tail}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand {
                    action: RuntimeBugContestAction::SelectContestants,
                    divider_trace: Some(divider_trace_for_sub_values([0])),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("exact Bug Contest divider trace applies once");
        let RuntimeMutationResult::BugContestUsed(special) = outcome.result else {
            panic!("expected Bug Contest result");
        };
        assert_eq!(
            special.effect,
            SpecialRoutineEffect::SelectRandomBugContestContestants {
                flags: vec!["EVENT_BUG_CATCHING_CONTESTANT_1A".to_string()],
                random_state_after: CrystalRandomState::default(),
            }
        );
        assert_eq!(state.random_state, CrystalRandomState::default());
        assert_eq!(
            state.bug_contest.selected_contestant_flags,
            vec!["EVENT_BUG_CATCHING_CONTESTANT_1A".to_string()]
        );
    }

    #[test]
    fn runtime_shuckie_commands_use_exact_party_payloads() {
        let trace = RuntimeDividerTrace::new([1, 2, 3, 4]);
        let give = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Give,
            party_index: None,
            divider_trace: Some(trace.clone()),
        };
        assert_eq!(runtime_shuckie_party_slot(&give).expect("give"), None);
        assert_eq!(
            runtime_shuckie_divider_trace(&give).expect("give divider trace"),
            Some(&trace)
        );

        let give_with_slot = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Give,
            party_index: Some(0),
            divider_trace: Some(trace.clone()),
        };
        let give_error = runtime_shuckie_party_slot(&give_with_slot)
            .expect_err("GiveShuckle must not receive ignored party slot state");
        assert!(
            format!("{give_error:#}").contains("Shuckie give command must not declare party_index"),
            "{give_error:#}"
        );
        let missing_divider_trace = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Give,
            party_index: None,
            divider_trace: None,
        };
        let missing_rng_error = runtime_shuckie_divider_trace(&missing_divider_trace)
            .expect_err("GiveShuckle must declare divider boundary");
        assert!(
            format!("{missing_rng_error:#}")
                .contains("Shuckie give command requires divider_trace"),
            "{missing_rng_error:#}"
        );

        let return_selected = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Return,
            party_index: Some(2),
            divider_trace: None,
        };
        assert_eq!(
            runtime_shuckie_party_slot(&return_selected).expect("return selected"),
            Some(2)
        );
        assert_eq!(
            runtime_shuckie_divider_trace(&return_selected).expect("return selected divider"),
            None
        );

        let return_cancelled = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Return,
            party_index: None,
            divider_trace: None,
        };
        assert_eq!(
            runtime_shuckie_party_slot(&return_cancelled).expect("return cancelled"),
            None
        );
        let return_with_rng = RuntimeShuckieCommand {
            action: RuntimeShuckieAction::Return,
            party_index: Some(2),
            divider_trace: Some(trace),
        };
        let unused_rng_error = runtime_shuckie_divider_trace(&return_with_rng)
            .expect_err("ReturnShuckie must not accept unused RNG state");
        assert!(
            format!("{unused_rng_error:#}")
                .contains("Shuckie return command must not declare divider_trace"),
            "{unused_rng_error:#}"
        );
    }

    #[test]
    fn generic_special_routine_command_enforces_rng_boundary() {
        assert!(runtime_special_routine_requires_divider_trace(
            "SampleKenjiBreakCountdown"
        ));
        assert!(!runtime_special_routine_requires_legacy_seed_boundary(
            "SampleKenjiBreakCountdown"
        ));
        assert!(!runtime_special_routine_requires_divider_trace(
            "HealParty"
        ));

        let data = GameDataSet {
            special_routines: special_routine_rules(["SampleKenjiBreakCountdown", "HealParty"]),
            ..GameDataSet::default()
        };
        let mut state = GameState {
            random_state: CrystalRandomState { add: 0xff, sub: 0 },
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

        let wrong_command = data
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
            .expect_err("exact RNG special must reject the legacy seed command");
        assert!(
            wrong_command
                .to_string()
                .contains("requires an authoritative divider trace command"),
            "{wrong_command}"
        );
        assert_eq!(state, before);

        let exhausted = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                    RuntimeRandomSpecialRoutineCommand {
                    routine: "SampleKenjiBreakCountdown".to_string(),
                        divider_trace: RuntimeDividerTrace::new([0]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exact RNG special must reject an exhausted divider trace");
        assert!(
            exhausted
                .to_string()
                .contains("divider replay exhausted after 1 samples"),
            "{exhausted}"
        );
        assert_eq!(state, before);

        let unused_tail = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                    RuntimeRandomSpecialRoutineCommand {
                        routine: "SampleKenjiBreakCountdown".to_string(),
                        divider_trace: RuntimeDividerTrace::new([0, 200, 77]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exact RNG special must reject an unused divider tail");
        assert!(
            unused_tail
                .to_string()
                .contains("divider trace has 1 unconsumed samples after 2 reads"),
            "{unused_tail}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                    RuntimeRandomSpecialRoutineCommand {
                        routine: "SampleKenjiBreakCountdown".to_string(),
                        divider_trace: RuntimeDividerTrace::new([0, 200]),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("generic RNG special applies with an exact divider trace");
        let RuntimeMutationResult::SpecialRoutineApplied(special) = outcome.result else {
            panic!("expected generic special result");
        };
        let SpecialRoutineEffect::SampleKenjiBreakCountdown {
            value,
            random_state_after,
        } = special.effect
        else {
            panic!("expected Kenji countdown effect");
        };
        assert_eq!(
            random_state_after,
            CrystalRandomState {
                add: 0xff,
                sub: 56
            }
        );
        assert_eq!(state.random_state, random_state_after);
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
    fn generic_unown_puzzle_requires_an_atomic_exact_divider_trace() {
        let data = GameDataSet {
            special_routines: special_routine_rules(["UnownPuzzle"]),
            ..GameDataSet::default()
        };
        let mut state = GameState::default();
        state
            .script_runtime
            .variables
            .insert("_value".to_string(), "KABUTO".to_string());
        let before = state.clone();
        let audio_ids = BTreeSet::new();
        let mut session = OverworldSession::with_events_and_objects(
            OverworldMapData {
                name: "RuntimeUnownPuzzleMap".to_string(),
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

        let mut exact_trace = divider_trace_for_sub_values(0_u8..16);
        exact_trace.samples.pop();
        let exhausted = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                    RuntimeRandomSpecialRoutineCommand {
                        routine: "UnownPuzzle".to_string(),
                        divider_trace: exact_trace,
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("truncated Unown puzzle divider trace must reject");
        assert!(
            exhausted
                .to_string()
                .contains("divider replay exhausted after 31 samples"),
            "{exhausted}"
        );
        assert_eq!(state, before);

        let mut trace_with_tail = divider_trace_for_sub_values(0_u8..16);
        trace_with_tail.samples.push(77);
        let unused_tail = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                    RuntimeRandomSpecialRoutineCommand {
                        routine: "UnownPuzzle".to_string(),
                        divider_trace: trace_with_tail,
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("unused Unown puzzle divider tail must reject");
        assert!(
            unused_tail
                .to_string()
                .contains("divider trace has 1 unconsumed samples after 32 reads"),
            "{unused_tail}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                    RuntimeRandomSpecialRoutineCommand {
                        routine: "UnownPuzzle".to_string(),
                        divider_trace: divider_trace_for_sub_values(0_u8..16),
                    },
                ),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("exact Unown puzzle divider trace applies once");
        let RuntimeMutationResult::SpecialRoutineApplied(special) = outcome.result else {
            panic!("expected generic special result");
        };
        let SpecialRoutineEffect::UnownPuzzle {
            puzzle_id,
            layout,
            random_state_after,
            ..
        } = special.effect
        else {
            panic!("expected Unown puzzle effect");
        };
        assert_eq!(puzzle_id, "KABUTO");
        assert_eq!(
            layout,
            vec![
                vec![1, 2, 3, 4, 5, 6],
                vec![7, 0, 0, 0, 0, 8],
                vec![9, 0, 0, 0, 0, 10],
                vec![11, 0, 0, 0, 0, 12],
                vec![13, 0, 0, 0, 0, 14],
                vec![15, 0, 0, 0, 0, 16],
            ]
        );
        assert_eq!(
            random_state_after,
            CrystalRandomState { add: 0, sub: 15 }
        );
        assert_eq!(state.random_state, random_state_after);
    }

    #[test]
    fn shuckie_give_requires_an_atomic_exact_divider_trace() {
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
                    divider_trace: Some(RuntimeDividerTrace::new([])),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exhausted Shuckie divider trace must reject");
        assert!(
            error
                .to_string()
                .contains("divider replay exhausted after 0 samples"),
            "{error}"
        );
        assert_eq!(state, before);

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand {
                    action: RuntimeShuckieAction::Give,
                    party_index: None,
                    divider_trace: Some(RuntimeDividerTrace::new([0; 5])),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("unused Shuckie divider tail must reject");
        assert!(
            error
                .to_string()
                .contains("use Shuckie give divider trace has 1 unconsumed samples after 4 reads"),
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
                    divider_trace: Some(RuntimeDividerTrace::new([0; 4])),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("Shuckie gift command applies with exact divider trace");
        let RuntimeMutationResult::ShuckieUsed(special) = outcome.result else {
            panic!("expected Shuckie result");
        };
        assert_eq!(
            special.effect,
            SpecialRoutineEffect::GiveShuckle {
                stored: true,
                random_state_after: CrystalRandomState::default(),
            }
        );
        assert_eq!(state.storage.party.filled_slots(), 1);
        assert_eq!(state.random_state, CrystalRandomState::default());
    }

    #[test]
    fn runtime_odd_egg_command_requires_exact_rng_boundary() {
        let missing_rng_seed =
            serde_json::from_value::<RuntimeOddEggCommand>(serde_json::json!({}))
                .expect_err("Odd Egg command must declare the divider trace");
        assert!(
            missing_rng_seed
                .to_string()
                .contains("missing field `divider_trace`"),
            "{missing_rng_seed}"
        );

        let command = serde_json::from_value::<RuntimeOddEggCommand>(serde_json::json!({
            "divider_trace": { "samples": [1, 2] }
        }))
        .expect("Odd Egg command accepts exact divider boundary");
        assert_eq!(command.divider_trace.samples, vec![1, 2]);
    }

    #[test]
    fn odd_egg_command_requires_an_atomic_exact_divider_trace() {
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
        let audio_ids = BTreeSet::new();
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
                RuntimeMutationCommand::GiveOddEgg(RuntimeOddEggCommand {
                    divider_trace: RuntimeDividerTrace::new([]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exhausted Odd Egg divider trace must reject");

        assert!(
            error
                .to_string()
                .contains("divider replay exhausted after 0 samples"),
            "{error}"
        );
        assert_eq!(state, before);

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::GiveOddEgg(RuntimeOddEggCommand {
                    divider_trace: RuntimeDividerTrace::new([0, 0, 1]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("unused Odd Egg divider tail must reject");
        assert!(
            error
                .to_string()
                .contains("give Odd Egg divider trace has 1 unconsumed samples after 2 reads"),
            "{error}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::GiveOddEgg(RuntimeOddEggCommand {
                    divider_trace: RuntimeDividerTrace::new([0, 0]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("Odd Egg command applies with exact divider trace");
        let RuntimeMutationResult::OddEggGiven(special) = outcome.result else {
            panic!("expected Odd Egg result");
        };
        assert!(matches!(
            special.effect,
            SpecialRoutineEffect::GiveOddEgg {
                table_index: 0,
                party_slot: 0,
                random_state_after: CrystalRandomState { add: 0, sub: 0 },
                ..
            }
        ));
        assert_eq!(state.storage.party.filled_slots(), 1);
    }

    #[test]
    fn runtime_buena_password_command_requires_exact_rng_boundary() {
        let missing_divider_trace =
            serde_json::from_value::<RuntimeBuenaPasswordCommand>(serde_json::json!({
                "guess": "TODAY"
            }))
            .expect_err("Buena password command must declare the divider trace");
        assert!(
            missing_divider_trace
                .to_string()
                .contains("missing field `divider_trace`"),
            "{missing_divider_trace}"
        );
    }

    #[test]
    fn runtime_phone_random_special_command_requires_exact_rng_boundary() {
        let missing_divider_trace =
            serde_json::from_value::<RuntimePhoneCallerCommand>(serde_json::json!({
                "special": "random_phone_wild_mon",
                "contact_id": "PHONE_BIRDKEEPER_VANCE"
            }))
            .expect_err("phone random special command must declare its divider trace");
        assert!(
            missing_divider_trace
                .to_string()
                .contains("missing field `divider_trace`"),
            "{missing_divider_trace}"
        );
    }

    #[test]
    fn phone_random_special_command_requires_exact_consumed_divider_trace() {
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
        let mut state = GameState::default();
        let before = state.clone();
        let audio_ids = BTreeSet::new();
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

        let exhausted = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand {
                    special: RuntimePhoneRandomSpecial::RandomPhoneWildMon,
                    contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                    divider_trace: RuntimeDividerTrace::new([0]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exhausted phone random divider trace must reject");
        assert!(
            exhausted
                .to_string()
                .contains("divider replay exhausted after 1 samples"),
            "{exhausted}"
        );
        assert_eq!(state, before);

        let unused_tail = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand {
                    special: RuntimePhoneRandomSpecial::RandomPhoneWildMon,
                    contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                    divider_trace: RuntimeDividerTrace::new([0, 255, 17]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("unused phone random divider tail must reject");
        assert!(
            unused_tail
                .to_string()
                .contains("divider trace has 1 unconsumed samples after 2 reads"),
            "{unused_tail}"
        );
        assert_eq!(state, before);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand {
                    special: RuntimePhoneRandomSpecial::RandomPhoneWildMon,
                    contact_id: "PHONE_BIRDKEEPER_VANCE".to_string(),
                    divider_trace: RuntimeDividerTrace::new([0, 255]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("phone random special command applies with exact divider trace");
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
                random_state_after: CrystalRandomState { add: 0, sub: 1 },
            }
        );
        assert_eq!(state.random_state, CrystalRandomState { add: 0, sub: 1 });
    }

    #[test]
    fn buena_password_command_uses_an_atomic_exact_divider_trace() {
        let mut data = GameDataSet::default();
        data.special_routines = special_routine_rules(["BuenasPassword"]);
        let mut order = vec!["DailyWord".to_string()];
        let mut categories = BTreeMap::from([(
            "DailyWord".to_string(),
            BuenaPasswordCategoryDefinition {
                category_type: "BUENA_STRING".to_string(),
                points: 10,
                options: vec![
                    "TODAY".to_string(),
                    "TOMORROW".to_string(),
                    "YESTERDAY".to_string(),
                ],
            },
        )]);
        for index in 1..11 {
            let category_id = format!("DailyWord{index}");
            order.push(category_id.clone());
            categories.insert(
                category_id,
                BuenaPasswordCategoryDefinition {
                    category_type: "BUENA_STRING".to_string(),
                    points: 1,
                    options: vec![
                        format!("A{index}"),
                        format!("B{index}"),
                        format!("C{index}"),
                    ],
                },
            );
        }
        data.buena_password_categories = BuenaPasswordCategories {
            order,
            categories,
        };
        let mut state = GameState {
            rng_seed: 1,
            ..GameState::default()
        };
        let audio_ids = BTreeSet::new();
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
                    divider_trace: RuntimeDividerTrace::new([]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("exhausted Buena password divider trace must reject");
        assert!(
            error
                .to_string()
                .contains("divider replay exhausted after 0 samples"),
            "{error}"
        );
        assert_eq!(state, before_stale);

        let error = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBuenaPassword(RuntimeBuenaPasswordCommand {
                    guess: Some("TODAY".to_string()),
                    divider_trace: RuntimeDividerTrace::new([0; 5]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect_err("unused Buena password divider tail must reject");
        assert!(
            error
                .to_string()
                .contains("use Buena password divider trace has 1 unconsumed samples after 4 reads"),
            "{error}"
        );
        assert_eq!(state, before_stale);

        let outcome = data
            .apply_runtime_mutation_command(
                &mut state,
                &mut session,
                RuntimeMutationCommand::UseBuenaPassword(RuntimeBuenaPasswordCommand {
                    guess: Some("TODAY".to_string()),
                    divider_trace: RuntimeDividerTrace::new([0; 4]),
                }),
                &audio_ids,
                &audio_ids,
                &audio_ids,
            )
            .expect("Buena password command applies with exact divider trace");
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
                random_state_after: CrystalRandomState::default(),
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

    #[test]
    fn script_battle_result_accumulator_masks_only_capture_flags_and_keeps_win_loss_codes() {
        for (raw_result, expected) in [(0x00, "0"), (0x01, "1"), (0x40, "0"), (0x81, "1")] {
            let mut state = GameState {
                battle_result: raw_result,
                ..GameState::default()
            };

            set_script_battle_result_accumulator(&mut state);

            assert_eq!(state.script_runtime.script_value.as_deref(), Some(expected));
            assert_eq!(
                state
                    .script_runtime
                    .variables
                    .get("_value")
                    .map(String::as_str),
                Some(expected)
            );
        }
    }
