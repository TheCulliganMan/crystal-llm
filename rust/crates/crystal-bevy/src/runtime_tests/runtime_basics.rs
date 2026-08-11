    use super::*;
    use crystal_assets::SpecialRoutineRule;
    use crystal_assets::modpack::{
        MapModule, ModpackAudioAsset, ModpackCompileReport, VerificationError, VerificationSeverity,
    };
    use crystal_assets::{
        ModpackPcmAudioFormat, PokegearLandmark, PokemonCryMetadata, ScriptedTrainerBattle,
        ScriptedWildBattle,
    };
    use crystal_core::battle::capture::CaptureWobbleProbability;
    use crystal_core::battle::start::{
        StaticWildBattleRequest, TrainerBattleRequest, TrainerBattleStartStatus,
    };
    use crystal_core::battle::stats::{BattleStatMultiplier, BattleStatMultiplierTables};
    use crystal_core::map::{
        BackgroundEvent, CoordEvent, MapAttributes, MapEvents, MapScene, MapSceneTable,
        ObjectEvent, WarpEvent,
    };
    use crystal_core::models::{
        BaseStats, CaptureStorageLocation, Dv, Item, ItemPocket, LearnedMove, Move, Pokemon,
        PokemonSpecies, Trainer, TrainerPartyPokemon, growth_rate, item_pocket, pokemon_type,
    };
    use crystal_core::state::{
        FishingRodState, PLAYER_GENDER_FEMALE, PLAYER_GENDER_MALE, ScriptGraphicsRuntimeKind,
        SwarmMapTarget,
    };
    use crystal_core::systems::evolution::EvolutionEntry;
    use crystal_core::systems::experience::calculate_experience;
    use crystal_core::systems::field_items::ScriptFieldPickup;
    use crystal_core::systems::field_moves::{
        FieldEscapeItemRule, FieldItemRule, FieldMoveBadgeRequirement, FieldMoveBlockRule,
        FieldMoveCatalog, FieldMoveFlagRule, FieldMoveMoveRule, FieldMoveReplacement,
        FieldMoveRule, FieldMoveTravelRule, FieldRepelItemRule,
    };
    use crystal_core::systems::gift_pokemon::GiftPokemonScript;
    use crystal_core::systems::learnsets::LearnsetEntry;
    use crystal_core::systems::script_audio::{ScriptAudioCommand, ScriptAudioCue};
    use crystal_core::systems::script_control::{ScriptControlAction, ScriptControlCommand};
    use crystal_core::systems::script_flags::ScriptFlagCommand;
    use crystal_core::systems::script_objects::{
        ScriptMovement, ScriptMovementStep, ScriptObjectCommand,
    };
    use crystal_core::systems::script_runtime::{
        ScriptRuntimeCommand, ScriptRuntimeInputs, ScriptRuntimeOutcome,
    };
    use crystal_core::systems::script_swarms::ScriptSwarmCommand;
    use crystal_core::systems::script_text::{ScriptTextAction, ScriptTextBody, ScriptTextCommand};
    use crystal_core::systems::script_variables::{ScriptVariableCommand, ScriptVariableOutcome};
    use crystal_core::systems::script_warps::{ScriptMapAction, ScriptMapCommand};
    use crystal_core::systems::special_routines::{
        BuenaPasswordCategoryDefinition, SpecialRoutineEffect,
    };
    use crystal_core::systems::step_events::{
        PoisonDamageResult, StepEventCounters, StepEventRules,
    };
    use crystal_core::world::encounters::{
        EncounterSlotChance, EncounterSlotTables, FieldEncounterData, FieldEncounterEntry,
        FieldEncounterTable,
    };
    use crystal_core::world::encounters::{WildEncounter, WildEncounterData, WildEncounterTable};
    use crystal_core::world::fishing::{
        FishingCatalog, FishingGroup, FishingSlot, FishingSwarmRule, ROD_GOOD, RodTable,
    };

    fn error_debug(error: impl std::fmt::Debug + std::fmt::Display) -> String {
        format!("{error:#}")
    }

    fn magikarp_lengths_for_tests(
    ) -> Vec<crystal_core::systems::special_routines::MagikarpLengthEntry> {
        [
            (110, 1),
            (310, 2),
            (710, 4),
            (2710, 20),
            (7710, 50),
            (17710, 100),
            (32710, 150),
            (47710, 150),
            (57710, 100),
            (62710, 50),
            (64710, 20),
            (65210, 5),
            (65410, 2),
            (65510, 1),
        ]
        .into_iter()
        .map(|(threshold, divisor)| {
            crystal_core::systems::special_routines::MagikarpLengthEntry {
                threshold,
                divisor,
            }
        })
        .collect()
    }

    #[test]
    fn exhausted_moves_expose_crystal_struggle_slot() {
        let species =
            PokemonSpecies::new_for_tests("TESTMON", BaseStats::new(40, 40, 40, 40, 40, 40));
        let mut pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
        pokemon.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 0,
            pp_ups: 0,
        }];
        assert!(
            available_move_slots(&pokemon).is_empty(),
            "a zero-PP move is not selectable; RuntimeBattleOptions exposes STRUGGLE separately"
        );
    }
    use crystal_core::world::movement::MovementMode;

    fn temp_repository_root(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "crystal-bevy-runtime-{}-{unique}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("apps/web/assets/data"))
            .expect("create runtime data root");
        root
    }

    fn write_midi(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create midi parent");
        }
        std::fs::write(
            path,
            b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60MTrk\x00\x00\x00\x0c\x00\x90\x3c\x40\x60\x80\x3c\x40\x00\xff\x2f\x00",
        )
            .expect("write midi fixture");
    }

    fn write_pcm(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create pcm parent");
        }
        std::fs::write(path, [0_u8, 0, 0xff, 0x7f]).expect("write pcm fixture");
    }

    fn write_floor_tileset(root: &std::path::Path, tileset_name: &str) {
        write_tileset(
            root,
            tileset_name,
            r#"{
  "0": [0, 0, 0, 0]
}"#,
        );
    }

    fn write_fishing_tileset(root: &std::path::Path, tileset_name: &str) {
        write_tileset(
            root,
            tileset_name,
            r#"{
  "0": [0, 0, 41, 0]
}"#,
        );
    }

    fn write_headbutt_tileset(root: &std::path::Path, tileset_name: &str) {
        write_tileset(
            root,
            tileset_name,
            r#"{
  "0": [21, 21, 21, 21]
}"#,
        );
    }

    fn write_grass_tileset(root: &std::path::Path, tileset_name: &str) {
        write_tileset(
            root,
            tileset_name,
            r#"{
  "0": [0, 0, 0, 0],
  "1": [24, 24, 24, 24]
}"#,
        );
    }

    fn write_tileset(root: &std::path::Path, tileset_name: &str, payload: &str) {
        let path = root
            .join("apps/web/assets/data/tilesets")
            .join(format!("{tileset_name}.json"));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create tileset parent");
        }
        std::fs::write(path, payload).expect("write tileset fixture");
    }

    fn test_tileset(entries: &[(&str, &[&str])]) -> TilesetDefinition {
        let max_id = entries
            .iter()
            .map(|(id, _)| usize::from_str_radix(id, 16).expect("hex metatile id"))
            .max()
            .unwrap_or(0);
        let mut collision = (0..=max_id)
            .map(|id| {
                (
                    format!("{id:02x}"),
                    vec![
                        "FLOOR".to_string(),
                        "FLOOR".to_string(),
                        "FLOOR".to_string(),
                        "FLOOR".to_string(),
                    ],
                )
            })
            .collect::<BTreeMap<_, _>>();
        for (id, quadrants) in entries {
            collision.insert(
                (*id).to_string(),
                quadrants.iter().map(|token| (*token).to_string()).collect(),
            );
        }
        TilesetDefinition {
            collision,
            palette_map: vec![0; max_id + 1],
        }
    }

    fn report() -> ModpackCompileReport {
        report_for(&minimal_runtime_data())
    }

    fn report_for(data: &GameDataSet) -> ModpackCompileReport {
        ModpackCompileReport {
            manifests: vec!["core-modular".to_string()],
            maps: data.maps.len(),
            pokemon: data.pokemon.len(),
            moves: data.moves.len(),
            items: data.items.len(),
            ..ModpackCompileReport::default()
        }
    }

    fn identity() -> SaveModpackIdentity {
        SaveModpackIdentity::new(
            "core-modular",
            "1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd",
        )
        .expect("identity")
    }

    fn static_wild_request(species: &str, level: u8) -> StaticWildBattleRequest {
        let mut request = StaticWildBattleRequest::new(species, level);
        request.battle_music = "MUSIC_JOHTO_WILD_BATTLE".to_string();
        request
    }

    fn load_minimal_compiled_runtime(
        name: &str,
    ) -> (std::path::PathBuf, AssetRoot, CrystalRuntime) {
        load_minimal_compiled_runtime_with_runtime_files(name, BTreeMap::new())
    }

    fn load_minimal_compiled_runtime_with_runtime_files(
        name: &str,
        runtime_files: BTreeMap<String, Vec<u8>>,
    ) -> (std::path::PathBuf, AssetRoot, CrystalRuntime) {
        let root = temp_repository_root(name);
        write_midi(&root.join("apps/web/assets/data/content-packs/test/music/MUSIC_NONE.mid"));
        write_midi(&root.join("apps/web/assets/data/content-packs/test/music/MUSIC_ROUTE_29.mid"));
        write_midi(&root.join("apps/web/assets/data/content-packs/test/music/MUSIC_ROUTE_30.mid"));
        write_midi(
            &root.join("apps/web/assets/data/content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE.mid"),
        );
        write_midi(&root.join(
            "apps/web/assets/data/content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE_NIGHT.mid",
        ));
        write_midi(
            &root.join("apps/web/assets/data/content-packs/test/music/MUSIC_KANTO_WILD_BATTLE.mid"),
        );
        write_midi(&root.join("apps/web/assets/data/content-packs/test/sfx/SFX_TACKLE.mid"));
        write_midi(
            &root.join("apps/web/assets/data/content-packs/test/sfx/SFX_TITLE_SCREEN_ENTRANCE.mid"),
        );
        write_midi(&root.join("apps/web/assets/data/content-packs/test/cries/CRY_CHIKORITA.mid"));
        write_midi(&root.join("apps/web/assets/data/content-packs/test/cries/CRY_WOOPER.mid"));
        write_floor_tileset(&root, "johto");
        let data_root = root.join("apps/web/assets/data");
        let data = minimal_runtime_data_with_oak_intro();
        let pack = CompiledGamePack::new_unchecked_for_tests(data.clone(), report_for(&data))
            .with_runtime_files_for_tests(runtime_files);
        crystal_assets::write_compiled_game_pack_for_tests(
            data_root.join("runtime.crystalpack"),
            &pack,
        )
        .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect("load runtime");
        (root, asset_root, runtime)
    }

    fn complete_vendor_runtime_files() -> BTreeMap<String, Vec<u8>> {
        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        crystal_assets::REQUIRED_VENDOR_RUNTIME_FILE_KEYS
            .iter()
            .map(|&key| {
                let path = repository_root.join(key);
                let bytes = std::fs::read(&path).unwrap_or_else(|error| {
                    panic!("read required vendor asset {}: {error}", path.display())
                });
                (key.to_string(), bytes)
            })
            .collect()
    }

    fn runtime_species() -> PokemonSpecies {
        let mut species =
            PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65));
        species.int_id = 1;
        species
    }

    fn runtime_wooper_species() -> PokemonSpecies {
        let mut species =
            PokemonSpecies::new_for_tests("WOOPER", BaseStats::new(55, 45, 45, 15, 25, 25));
        species.int_id = 2;
        species
    }

    fn active_player_storage(pokemon: Pokemon) -> crystal_core::models::PokemonStorage {
        let mut storage = crystal_core::models::PokemonStorage::default();
        storage.party.pokemon[0] = Some(pokemon);
        storage
    }

    fn runtime_move() -> Move {
        runtime_move_named("TACKLE", 35)
    }

    fn runtime_move_named(name: &str, pp: u8) -> Move {
        Move {
            name: name.to_string(),
            move_type: pokemon_type("NORMAL"),
            power: 40,
            accuracy: 100,
            pp,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    fn sync_runtime_move_tables(data: &mut GameDataSet) {
        let move_ids: Vec<String> = data.moves.keys().cloned().collect();
        data.move_names = move_ids.clone();
        data.battle_animations
            .entry("ANIM_NULL".to_string())
            .or_insert_with(|| vec!["BATTLE_ANIM_END".to_string()]);
        for move_id in &move_ids {
            data.battle_animations
                .entry(move_id.clone())
                .or_insert_with(|| vec!["BATTLE_ANIM_END".to_string()]);
        }
        data.battle_animation_table = std::iter::once("ANIM_NULL".to_string())
            .chain(move_ids)
            .collect();
    }

    fn growth_rate_catalog_for_tests() -> crystal_core::systems::experience::GrowthRateCatalog {
        [
            ("GROWTH_MEDIUM_FAST", 1, 1, 0, 0, 0),
            ("GROWTH_SLIGHTLY_FAST", 3, 4, 10, 0, 30),
            ("GROWTH_SLIGHTLY_SLOW", 3, 4, 20, 0, 70),
            ("GROWTH_MEDIUM_SLOW", 6, 5, -15, 100, 140),
            ("GROWTH_FAST", 4, 5, 0, 0, 0),
            ("GROWTH_SLOW", 5, 4, 0, 0, 0),
        ]
        .into_iter()
        .map(
            |(id, numerator, denominator, quadratic, linear, constant)| {
                (
                    id.to_string(),
                    crystal_core::systems::experience::GrowthRateCurve {
                        id: id.to_string(),
                        numerator,
                        denominator,
                        quadratic,
                        linear,
                        constant,
                    },
                )
            },
        )
        .collect()
    }

    fn runtime_item(id: &str, pocket: ItemPocket) -> Item {
        Item {
            name: id.to_string(),
            description: String::new(),
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
            pocket,
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn runtime_ball_item(id: &str) -> Item {
        let mut item = runtime_item(id, item_pocket("BALL"));
        item.effect = "POKE_BALL".to_string();
        item.consumable = true;
        item.battle_menu = "ITEMMENU_CLOSE".to_string();
        item.battle_usable = true;
        item
    }

    fn runtime_tmhm_item(id: &str, tmhm_index: usize, move_id: &str) -> Item {
        let mut item = runtime_item(id, item_pocket("TM_HM"));
        item.tmhm_index = Some(tmhm_index);
        item.tmhm_move = Some(move_id.to_string());
        item
    }

    fn runtime_map() -> MapModule {
        MapModule {
            id: "RuntimeMap".to_string(),
            attributes: MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 0,
                width: 2,
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
                map_constant: Some("RUNTIME_MAP".to_string()),
                map_group_constant: None,
                blocks_label: Some("RuntimeMap_Blocks".to_string()),
                map_scripts_label: Some("RuntimeMap_MapScripts".to_string()),
                map_events_label: Some("RuntimeMap_MapEvents".to_string()),
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
            map_script_section_commands: vec![crystal_core::map::MapScriptSectionCommand {
                command: "callback".to_string(),
                args: vec![
                    "MAPCALLBACK_NEWMAP".to_string(),
                    "RuntimeScript".to_string(),
                ],
                command_index: 0,
            }],
            map_event_section_commands: vec![crystal_core::map::MapEventSectionCommand {
                command: "warp_event".to_string(),
                args: vec![
                    "1".to_string(),
                    "0".to_string(),
                    "RUNTIME_MAP".to_string(),
                    "4".to_string(),
                ],
                command_index: 0,
            }],
            scenes: MapSceneTable::default(),
            events: MapEvents {
                warps: vec![WarpEvent {
                    index: 4,
                    x: 1,
                    y: 0,
                    target_map_constant: "RUNTIME_MAP".to_string(),
                    target_map: "RUNTIME_MAP".to_string(),
                    target_warp_id: 4,
                }],
                ..MapEvents::default()
            },
            objects: Vec::new(),
            blocks: vec![0, 0],
        }
    }

    fn runtime_map_metadata(
        constant: &str,
        name: &str,
        group_id: u16,
        map_id: u16,
        environment: &str,
    ) -> crystal_assets::RuntimeMapMetadata {
        crystal_assets::RuntimeMapMetadata {
            constant: constant.to_string(),
            name: name.to_string(),
            group_name: "RUNTIME".to_string(),
            group_id,
            map_id,
            width: 2,
            height: 1,
            environment: environment.to_string(),
            phone_service: 0,
        }
    }

    fn runtime_object(object_identifier: &str, event_flag: &str) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x: 1,
            y: 1,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "RuntimeWildScript".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(object_identifier.to_string()),
            sightline_direction_override: None,
        }
    }

    fn minimal_runtime_data() -> GameDataSet {
        let mut data = GameDataSet {
            pokemon: [("CHIKORITA".to_string(), runtime_species())]
                .into_iter()
                .collect(),
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
            moves: [("TACKLE".to_string(), runtime_move())]
                .into_iter()
                .collect(),
            audio: vec![
                ModpackAudioAsset::music("MUSIC_NONE", "content-packs/test/music/MUSIC_NONE.mid")
                    .expect("music none asset"),
                ModpackAudioAsset::music(
                    "MUSIC_ROUTE_29",
                    "content-packs/test/music/MUSIC_ROUTE_29.mid",
                )
                .expect("route 29 music asset"),
                ModpackAudioAsset::music(
                    "MUSIC_JOHTO_WILD_BATTLE",
                    "content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE.mid",
                )
                .expect("johto wild battle music asset"),
                ModpackAudioAsset::music(
                    "MUSIC_JOHTO_WILD_BATTLE_NIGHT",
                    "content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE_NIGHT.mid",
                )
                .expect("johto night wild battle music asset"),
                ModpackAudioAsset::music(
                    "MUSIC_KANTO_WILD_BATTLE",
                    "content-packs/test/music/MUSIC_KANTO_WILD_BATTLE.mid",
                )
                .expect("kanto wild battle music asset"),
                ModpackAudioAsset::sound_effect(
                    "SFX_TACKLE",
                    "content-packs/test/sfx/SFX_TACKLE.mid",
                )
                .expect("sfx asset"),
                ModpackAudioAsset::sound_effect(
                    "SFX_TITLE_SCREEN_ENTRANCE",
                    "content-packs/test/sfx/SFX_TITLE_SCREEN_ENTRANCE.mid",
                )
                .expect("title entrance sfx asset"),
                ModpackAudioAsset::cry(
                    "CRY_CHIKORITA",
                    "content-packs/test/cries/CRY_CHIKORITA.mid",
                )
                .expect("cry asset"),
            ],
            growth_rates: growth_rate_catalog_for_tests(),
            evolutions: crystal_core::systems::evolution::EvolutionTable(
                [("CHIKORITA".to_string(), Vec::new())]
                    .into_iter()
                    .collect(),
            ),
            maps: [("RuntimeMap".to_string(), runtime_map())]
                .into_iter()
                .collect(),
            map_attributes: [("RuntimeMap".to_string(), runtime_map().attributes.clone())]
                .into_iter()
                .collect(),
            runtime_spawn_points: [(
                "0".to_string(),
                RuntimeSpawnPoint {
                    identifier: 0,
                    map_constant: "RUNTIME_MAP".to_string(),
                    map_name: "RuntimeMap".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 0,
                    tile_y: 0,
                    group_name: "RUNTIME".to_string(),
                    metatile_x: 0,
                    metatile_y: 0,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]
            .into_iter()
            .collect(),
            runtime_map_metadata: [(
                "RUNTIME_MAP".to_string(),
                runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "ROUTE"),
            )]
            .into_iter()
            .collect(),
            runtime_title_screen: RuntimeTitleScreen {
                new_game_spawn_identifier: Some(0),
                title_music: Some("MUSIC_NONE".to_string()),
            },
            currency_constants: crystal_core::systems::economy::CurrencyCatalog(
                [
                    ("MAX_MONEY".to_string(), 999_999),
                    ("MAX_COINS".to_string(), 9_999),
                ]
                .into_iter()
                .collect(),
            ),
            encounter_slot_tables: EncounterSlotTables {
                tables: [
                    (
                        EncounterSurface::Grass.as_key().to_string(),
                        vec![EncounterSlotChance {
                            threshold: 100,
                            slot: 0,
                        }],
                    ),
                    (
                        EncounterSurface::Water.as_key().to_string(),
                        vec![EncounterSlotChance {
                            threshold: 100,
                            slot: 0,
                        }],
                    ),
                ]
                .into_iter()
                .collect(),
            },
            battle_stat_multipliers: BattleStatMultiplierTables {
                stat: vec![
                    BattleStatMultiplier {
                        numerator: 1,
                        denominator: 1,
                    };
                    13
                ],
                accuracy: vec![
                    BattleStatMultiplier {
                        numerator: 1,
                        denominator: 1,
                    };
                    13
                ],
            },
            capture_wobble_probabilities: vec![CaptureWobbleProbability {
                catch_rate: 255,
                chance: 255,
            }],
            capture_rules: minimal_capture_rules(),
            battle_escape_rules: minimal_battle_escape_rules(),
            oak_ratings: vec![crystal_core::systems::special_routines::OakRatingEntry {
                caught_count_limit: 1,
                fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                text_label: "OakRating01".to_string(),
            }],
            move_priorities: crystal_core::battle::turn::MovePriorityTable {
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
            },
            type_categories: crystal_core::battle::damage::TypeCategories {
                physical: vec!["NORMAL".to_string(), "FIGHTING".to_string()],
                special: vec!["FIRE".to_string(), "WATER".to_string()],
            },
            type_effectiveness: crystal_core::battle::damage::TypeEffectivenessTable {
                matchups: ["NORMAL", "FIGHTING", "FIRE", "WATER"]
                    .into_iter()
                    .map(|attacker| {
                        (
                            attacker.to_string(),
                            ["NORMAL", "FIGHTING", "FIRE", "WATER"]
                                .into_iter()
                                .map(|defender| {
                                    (
                                        defender.to_string(),
                                        crystal_core::battle::damage::TypeMultiplier::one(),
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect(),
                foresight_matchups: [(
                    "NORMAL".to_string(),
                    [(
                        "FIGHTING".to_string(),
                        crystal_core::battle::damage::TypeMultiplier::zero(),
                    )]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .collect(),
            },
            weather_modifiers: crystal_core::battle::damage::WeatherModifiers {
                type_modifiers: [(
                    "WEATHER_RAIN".to_string(),
                    [(
                        "WATER".to_string(),
                        crystal_core::battle::damage::TypeMultiplier {
                            numerator: 3,
                            denominator: 2,
                        },
                    )]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .collect(),
                move_effect_modifiers: [(
                    "WEATHER_RAIN".to_string(),
                    [(
                        "SOLARBEAM".to_string(),
                        crystal_core::battle::damage::TypeMultiplier {
                            numerator: 1,
                            denominator: 2,
                        },
                    )]
                    .into_iter()
                    .collect(),
                )]
                .into_iter()
                .collect(),
            },
            battle_reward_rules: minimal_battle_reward_rules(),
            step_event_rules: minimal_step_event_rules(),
            field_moves: minimal_field_move_catalog(),
            ..GameDataSet::default()
        };
        populate_minimal_runtime_presence_catalogs(&mut data);
        data
    }

    fn populate_minimal_runtime_presence_catalogs(data: &mut GameDataSet) {
        data.story_events = vec![serde_json::json!({
            "StandardScripts": {
                "StdScripts": [
                    {"command": "add_stdscript", "args": ["PokecenterSignScript"]}
                ],
                "PokecenterSignScript": [
                    {"command": "farjumptext", "args": ["PokecenterSignText"]}
                ]
            }
        })];
        data.asm_text
            .entry("PokecenterSignText".to_string())
            .or_insert_with(|| "A POKéMON CENTER heals tired POKéMON.".to_string());
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![crystal_core::systems::learnsets::LearnsetEntry(
                1,
                "TACKLE".to_string(),
            )],
        );
        data.items
            .entry("POKE_BALL".to_string())
            .or_insert_with(|| runtime_ball_item("POKE_BALL"));
        data.items
            .entry("BLU_APRICORN".to_string())
            .or_insert_with(|| runtime_item("BLU_APRICORN", item_pocket("ITEM")));
        data.items
            .entry("COIN_CASE".to_string())
            .or_insert_with(|| {
                let mut coin_case = runtime_item("COIN_CASE", item_pocket("KEY_ITEM"));
                coin_case.effect = "COIN_CASE".to_string();
                coin_case.field_menu = "ITEMMENU_CLOSE".to_string();
                coin_case.field_usable = true;
                coin_case
            });
        data.items
            .entry("TM_TACKLE".to_string())
            .or_insert_with(|| runtime_item("TM_TACKLE", item_pocket("TM_HM")));
        data.marts
            .0
            .entry("MART_RUNTIME".to_string())
            .or_insert_with(|| vec!["POKE_BALL".to_string()]);
        data.fishing
            .groups
            .entry("FISHGROUP_RUNTIME".to_string())
            .or_insert_with(|| crystal_core::world::fishing::FishingGroup {
                bite_threshold: 255,
                rod_tables: [(
                    crystal_core::world::fishing::ROD_OLD.to_string(),
                    crystal_core::world::fishing::RodTable {
                        slots: vec![crystal_core::world::fishing::FishingSlot {
                            threshold: 255,
                            species: Some("CHIKORITA".to_string()),
                            level: 5,
                            time_group: None,
                        }],
                    },
                )]
                .into_iter()
                .collect(),
            });
        data.fishing
            .rod_items
            .entry("OLD_ROD".to_string())
            .or_insert_with(|| crystal_core::world::fishing::ROD_OLD.to_string());
        data.fruit_trees
            .0
            .entry("FRUITTREE_RUNTIME".to_string())
            .or_insert_with(|| "BLU_APRICORN".to_string());
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .objects
            .push(runtime_object("RuntimeNpc", "EVENT_RUNTIME_NPC"));
        data.map_scripts
            .entry("RuntimeMap_MapScripts".to_string())
            .or_insert_with(|| serde_json::json!({ "RuntimeScript": [] }));
        data.map_scripts
            .entry("RuntimeMap_MapEvents".to_string())
            .or_insert_with(|| {
                serde_json::json!([
                    {"command":"def_warp_events","args":[]},
                    {"command":"warp_event","args":["0","0","RUNTIME_MAP","4"]},
                    {"command":"warp_event","args":["0","0","RUNTIME_MAP","4"]},
                    {"command":"warp_event","args":["0","0","RUNTIME_MAP","4"]},
                    {"command":"warp_event","args":["1","0","RUNTIME_MAP","4"]},
                    {"command":"def_coord_events","args":[]},
                    {"command":"def_bg_events","args":[]},
                    {"command":"def_object_events","args":[]}
                ])
            });
        data.map_blocks
            .entry("RuntimeMap_Blocks".to_string())
            .or_insert_with(|| "00 00".to_string());
        data.npcs
            .entry("RuntimeMap".to_string())
            .or_insert_with(|| serde_json::json!({ "objects": ["RuntimeNpc"] }));
        data.pc_strings
            .entry("PC_RUNTIME".to_string())
            .or_insert_with(|| "Runtime PC".to_string());
        data.menu_icons
            .entry("CHIKORITA".to_string())
            .or_insert_with(|| "ICON_CHIKORITA".to_string());
        data.pokedex_entries
            .entry("CHIKORITA".to_string())
            .or_insert_with(|| crystal_core::models::RuntimePokedexEntry {
                species: "CHIKORITA".to_string(),
                classification: "Leaf".to_string(),
                height_digits: 9,
                weight_digits: 64,
                pages: vec!["A sweet leaf Pokemon.".to_string()],
            });
        data.pokemon_frontpic_anim
            .entry("CHIKORITA".to_string())
            .or_insert_with(|| crystal_core::models::FrontpicAnimProgram {
                commands: vec![crystal_core::models::FrontpicAnimCommand {
                    kind: "frame".to_string(),
                    frame: Some(0),
                    duration: Some(8),
                    ..crystal_core::models::FrontpicAnimCommand::default()
                }],
            });
        data.asm_text
            .entry("RuntimeText".to_string())
            .or_insert_with(|| "RuntimeText".to_string());
        sync_runtime_move_tables(data);
        data.battle_anim_bundle = serde_json::json!({
            "objects": { "BattleAnim_Tackle": {} },
            "framesets": { "BattleAnim_TackleFrames": {} },
            "oam_sets": { "BattleAnim_TackleOam": {} },
            "gfx_table": { "BattleAnim_TackleGfx": {} },
            "gfx_sources": { "BattleAnim_TackleGfx": {} }
        })
        .to_string();
        data.sprite_anim_bundle = serde_json::json!({
            "oam_sets": { "SpriteAnimFrame": {} },
            "framesets": { "SpriteAnimFrameSet": {} },
            "objects": { "SpriteAnimObject": {} }
        })
        .to_string();
        data.trainers
            .trainers
            .entry("TRAINER_RUNTIME".to_string())
            .or_insert_with(|| crystal_core::models::Trainer {
                name: "Runtime".to_string(),
                trainer_id: "TRAINER_RUNTIME".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                party: vec![crystal_core::models::TrainerPartyPokemon {
                    species: "CHIKORITA".to_string(),
                    level: 5,
                    item: None,
                    moves: Vec::new(),
                    dvs: Dv::default(),
                }],
                win_quote: "Win".to_string(),
                lose_quote: "Lose".to_string(),
                items: Vec::new(),
                base_reward: 1,
                ai_move_flags: 0,
                ai_item_switch_flags: 0,
                encounter_music: "MUSIC_NONE".to_string(),
                ai_layers: Vec::new(),
            });
        data.tilesets
            .entry("johto".to_string())
            .or_insert_with(|| TilesetDefinition {
                collision: [(
                    "00".to_string(),
                    vec![
                        "FLOOR".to_string(),
                        "FLOOR".to_string(),
                        "FLOOR".to_string(),
                        "FLOOR".to_string(),
                    ],
                )]
                .into_iter()
                .collect(),
                palette_map: vec![0],
            });
        data.sprite_palette_defaults
            .entry("SPRITE_MON".to_string())
            .or_insert(0);
        data.pokegear_town_map_palette_map
            .entry("RuntimeMap".to_string())
            .or_insert_with(|| vec!["PAL_RUNTIME".to_string()]);
        if data.pokegear_landmarks.landmarks.is_empty() {
            data.pokegear_landmarks.landmarks = vec![crystal_core::models::PokegearLandmark {
                id: 1,
                constant: "LANDMARK_RUNTIME".to_string(),
                label: "RuntimeLandmark".to_string(),
                name: "Runtime".to_string(),
                x: 1,
                y: 1,
                region: "JOHTO".to_string(),
            }];
        }
        data.pokegear_landmarks
            .map_to_landmark
            .entry("RuntimeMap".to_string())
            .or_insert_with(|| "LANDMARK_RUNTIME".to_string());
        data.phone_contacts
            .0
            .entry("PHONE_RUNTIME".to_string())
            .or_insert_with(|| crystal_core::systems::phone::PhoneContactRecord {
                contact_id: "PHONE_RUNTIME".to_string(),
                trainer_class: None,
                trainer_label: None,
                lines: vec!["RuntimePhone".to_string()],
                primary_label: "RuntimePhone".to_string(),
                map_constant: Some("RUNTIME_MAP".to_string()),
                callee_time_mask: 0xff,
                callee_script: Some("RuntimePhoneScript".to_string()),
                caller_time_mask: 0xff,
                caller_script: Some("RuntimePhoneScript".to_string()),
            });
        data.permanent_phone_numbers
            .entry("PHONE_RUNTIME".to_string())
            .or_insert_with(crystal_core::systems::phone::PermanentPhoneNumberRule::default);
        data.special_phone_calls
            .entry("RuntimePhoneScript".to_string())
            .or_insert_with(crystal_assets::SpecialPhoneCallRule::default);
        if data.phone_scripts.is_empty() {
            data.phone_scripts = vec![serde_json::json!({ "RuntimePhoneScript": [] })];
        }
        data.flee_mons
            .buckets
            .entry("always".to_string())
            .or_insert_with(|| vec!["CHIKORITA".to_string()]);
        if data.buena_password_categories.order.is_empty() {
            data.buena_password_categories.order = vec!["BUENA_RUNTIME".to_string()];
        }
        data.buena_password_categories
            .categories
            .entry("BUENA_RUNTIME".to_string())
            .or_insert_with(|| {
                crystal_core::systems::special_routines::BuenaPasswordCategoryDefinition {
                    category_type:
                        crystal_core::systems::special_routines::BUENA_PASSWORD_CATEGORY_MON
                            .to_string(),
                    points: 1,
                    options: vec!["CHIKORITA".to_string()],
                }
            });
        if data.roaming_pokemon.is_empty() {
            use crystal_core::systems::special_routines::{
                RoamingMapLocation, RoamingPokemonCatalog, RoamingPokemonInitWrite,
                RoamingPokemonRoute,
            };

            data.roaming_pokemon = RoamingPokemonCatalog {
                slot_count: 3,
                inactive_map: RoamingMapLocation {
                    map_group: 0xfe,
                    map_number: 0xfd,
                },
                init_writes: vec![
                    RoamingPokemonInitWrite {
                        slot: 0,
                        species: "CHIKORITA".to_string(),
                        level: 40,
                        map_group: 1,
                        map_number: 1,
                        hp: 0,
                    },
                    RoamingPokemonInitWrite {
                        slot: 1,
                        species: "CHIKORITA".to_string(),
                        level: 40,
                        map_group: 1,
                        map_number: 2,
                        hp: 0,
                    },
                ],
                routes: (0_u8..16)
                    .map(|index| RoamingPokemonRoute {
                        map_group: 1,
                        map_number: index + 1,
                        connections: vec![RoamingMapLocation {
                            map_group: 1,
                            map_number: (index + 1) % 16 + 1,
                        }],
                    })
                    .collect(),
                jump_mask: 15,
            };
        }
        data.buena_prizes
            .entry("POKE_BALL".to_string())
            .or_insert(1);
        data.kurt_apricorn_recipes
            .entry("BLU_APRICORN".to_string())
            .or_insert_with(|| "POKE_BALL".to_string());
        if data.shuckie_gift.is_none() {
            data.shuckie_gift = Some(
                crystal_core::systems::special_routines::ShuckieGiftDefinition {
                    species: "CHIKORITA".to_string(),
                    level: 15,
                    held_item: "POKE_BALL".to_string(),
                    nickname: "SHUCKIE".to_string(),
                    original_trainer_name: "MANIA".to_string(),
                    original_trainer_id: 1,
                    got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
                },
            );
        }
        data.dratini_move_sets
            .entry(0)
            .or_insert_with(|| vec!["TACKLE".to_string()]);
        if data.bug_contest_config.is_none() {
            data.bug_contest_config =
                Some(crystal_core::systems::special_routines::BugContestConfig {
                    park_balls: 20,
                    timer_minutes: 20,
                    timer_seconds: 0,
                    selected_contestant_count: 1,
                    contestant_flags: vec!["EVENT_RUNTIME_CONTESTANT".to_string()],
                    encounters: {
                        let mut encounters = (0..10)
                            .map(|_| crystal_core::systems::special_routines::BugContestEncounterEntry {
                                weight: 10,
                                species: "CHIKORITA".to_string(),
                                min_level: 5,
                                max_level: 5,
                            })
                            .collect::<Vec<_>>();
                        encounters.push(crystal_core::systems::special_routines::BugContestEncounterEntry {
                            weight: u8::MAX,
                            species: "CHIKORITA".to_string(),
                            min_level: 5,
                            max_level: 5,
                        });
                        encounters
                    },
                });
        }
        if data.battle_tower_rules.is_none() {
            data.battle_tower_rules =
                Some(crystal_core::systems::special_routines::BattleTowerRules {
                    banned_species: BTreeMap::new(),
                    required_party_count: 3,
                    challenge_streak_length: 7,
                    minimum_level_group: 10,
                    maximum_level_group: 100,
                    level_group_size: 10,
                    party_count_failure_text: "BattleTowerNeedThreeText".to_string(),
                    duplicate_species_failure_text: "BattleTowerDuplicateSpeciesText".to_string(),
                    duplicate_held_item_failure_text: "BattleTowerDuplicateHeldItemText"
                        .to_string(),
                    egg_failure_text: "BattleTowerEggText".to_string(),
                    trainers: vec![
                        crystal_core::systems::special_routines::BattleTowerTrainerDefinition {
                            index: 0,
                            trainer_class: "YOUNGSTER".to_string(),
                            name: "RUNTIME@".to_string(),
                            sprite_constant: "SPRITE_YOUNGSTER".to_string(),
                        },
                    ],
                    mon_groups: vec![vec![
                        crystal_core::systems::special_routines::BattleTowerMonDefinition {
                            species: "CHIKORITA".to_string(),
                            moves: vec!["TACKLE".to_string()],
                            level: 10,
                            nickname: "CHIKORITA".to_string(),
                            ..Default::default()
                        },
                    ]],
                });
        }
        if !data
            .initialize_events
            .event_flags
            .iter()
            .any(|flag| flag == "EVENT_RUNTIME_CONTESTANT")
        {
            data.initialize_events
                .event_flags
                .push("EVENT_RUNTIME_CONTESTANT".to_string());
        }
        if !data
            .initialize_events
            .engine_flags
            .iter()
            .any(|flag| flag == "ENGINE_GOT_SHUCKIE_TODAY")
        {
            data.initialize_events
                .engine_flags
                .push("ENGINE_GOT_SHUCKIE_TODAY".to_string());
        }
        if data.odd_egg_definitions.is_empty() {
            data.odd_egg_definitions =
                vec![crystal_core::systems::special_routines::OddEggDefinition {
                    species: "CHIKORITA".to_string(),
                    moves: vec!["TACKLE".to_string()],
                    original_trainer_id: 1,
                    dvs: [0; 4],
                    probability: 100,
                    level: 5,
                    experience: 0,
                    hatch_cycles: 1,
                    nickname: "EGG".to_string(),
                    original_trainer_name: "DAYCARE".to_string(),
                }];
        }
        if data.magikarp_lengths.is_empty() {
            data.magikarp_lengths = magikarp_lengths_for_tests();
        }
        if data.happiness_data.is_none() {
            data.happiness_data = Some(crystal_core::systems::special_routines::HappinessData {
                changes: [(
                    9,
                    crystal_core::systems::special_routines::HappinessChangeEntry {
                        code: "HAPPINESS_RUNTIME_BOOTSTRAP".to_string(),
                        low: 1,
                        mid: 1,
                        high: 1,
                    },
                )]
                .into_iter()
                .collect(),
                services: [(
                    "RuntimeBootstrapHappiness".to_string(),
                    vec![
                        crystal_core::systems::special_routines::HappinessServiceOutcome {
                            roll_weight: 255,
                            script_value: 2,
                            change_code: 9,
                        },
                    ],
                )]
                .into_iter()
                .collect(),
            });
        }
        data.story_event_script_constants
            .global
            .entry("EVENT_RUNTIME".to_string())
            .or_insert(1);
    }

    fn verified_runtime_bootstrap_data() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.pokemon_cries.insert(
            "CHIKORITA".to_string(),
            crystal_assets::PokemonCryMetadata {
                cry: "CRY_NIDORAN_M".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        data.trainers
            .trainers
            .get_mut("TRAINER_RUNTIME")
            .expect("runtime trainer")
            .encounter_music = "MUSIC_ROUTE_29".to_string();
        data
    }

    fn minimal_step_event_rules() -> StepEventRules {
        StepEventRules {
            poison_step_interval: 4,
            egg_step_trigger: 0x80,
            hatched_egg_happiness: 0x78,
            poison_status: "POISON".to_string(),
            egg_nickname: "EGG".to_string(),
            happiness_step_counter_mask: 1,
            happiness_step_counter_target: 0,
        }
    }

    fn minimal_capture_ball_rule() -> crystal_core::battle::capture::CaptureBallRule {
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
        }
    }

    fn minimal_capture_rules() -> crystal_core::battle::capture::CaptureRules {
        crystal_core::battle::capture::CaptureRules {
            fast_ball_species: BTreeSet::new(),
            heavy_ball_modifiers: BTreeMap::new(),
            ball_rules: [
                ("MASTER_BALL".to_string(), minimal_capture_ball_rule()),
                ("POKE_BALL".to_string(), minimal_capture_ball_rule()),
            ]
            .into_iter()
            .collect(),
            guaranteed_capture_balls: ["MASTER_BALL".to_string()].into_iter().collect(),
            status_bonus: [("SLEEP".to_string(), 10), ("FREEZE".to_string(), 10)]
                .into_iter()
                .collect(),
        }
    }

    fn minimal_battle_reward_rules() -> BattleRewardRules {
        BattleRewardRules {
            max_level: 100,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
        }
    }

    fn minimal_battle_escape_rules() -> crystal_core::systems::battle_escape::BattleEscapeRules {
        crystal_core::systems::battle_escape::BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        }
    }

    fn field_move_badge(index: usize) -> FieldMoveBadgeRequirement {
        FieldMoveBadgeRequirement {
            region: "johto".to_string(),
            index,
        }
    }

    fn field_move_replacement(replacement_block_id: u16, variant: &str) -> FieldMoveReplacement {
        FieldMoveReplacement {
            replacement_block_id,
            variant: variant.to_string(),
        }
    }

    fn field_move_replacements(
        tileset: &str,
        block_id: u16,
        replacement_block_id: u16,
        variant: &str,
    ) -> BTreeMap<String, BTreeMap<u16, FieldMoveReplacement>> {
        [(
            tileset.to_string(),
            [(
                block_id,
                field_move_replacement(replacement_block_id, variant),
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect()
    }

    fn minimal_field_move_catalog() -> FieldMoveCatalog {
        FieldMoveCatalog {
            cut: FieldMoveBlockRule {
                move_id: "CUT".to_string(),
                badge: field_move_badge(1),
                target_collisions: vec![0x12, 0x1a, 0x18, 0x14, 0x1c],
                replacements: field_move_replacements("johto", 0x5b, 0x3c, "tree"),
            },
            whirlpool: FieldMoveBlockRule {
                move_id: "WHIRLPOOL".to_string(),
                badge: field_move_badge(6),
                target_collisions: vec![0x24, 0x2c],
                replacements: field_move_replacements("johto", 0x07, 0x36, "whirlpool"),
            },
            strength: FieldMoveFlagRule {
                move_id: "STRENGTH".to_string(),
                badge: field_move_badge(2),
                engine_flag: "ENGINE_STRENGTH_ACTIVE".to_string(),
            },
            flash: FieldMoveFlagRule {
                move_id: "FLASH".to_string(),
                badge: field_move_badge(0),
                engine_flag: "STATUSFLAGS_FLASH".to_string(),
            },
            surf: FieldMoveTravelRule {
                move_id: "SURF".to_string(),
                badge: field_move_badge(3),
                blocked_collisions: vec![0x24, 0x2c, 0x33, 0x30, 0x31, 0x32],
                target_collisions: Vec::new(),
            },
            waterfall: FieldMoveTravelRule {
                move_id: "WATERFALL".to_string(),
                badge: field_move_badge(7),
                blocked_collisions: Vec::new(),
                target_collisions: vec![0x33, 0x30, 0x31, 0x32, 0x3b],
            },
            fly: FieldMoveRule {
                move_id: "FLY".to_string(),
                badge: field_move_badge(5),
            },
            dig: FieldMoveMoveRule {
                move_id: "DIG".to_string(),
                target_collisions: Vec::new(),
            },
            teleport: FieldMoveMoveRule {
                move_id: "TELEPORT".to_string(),
                target_collisions: Vec::new(),
            },
            headbutt: FieldMoveMoveRule {
                move_id: "HEADBUTT".to_string(),
                target_collisions: vec![0x15, 0x1d],
            },
            rock_smash: FieldMoveMoveRule {
                move_id: "ROCK_SMASH".to_string(),
                target_collisions: Vec::new(),
            },
            sweet_scent: FieldMoveMoveRule {
                move_id: "SWEET_SCENT".to_string(),
                target_collisions: Vec::new(),
            },
            escape_rope: FieldEscapeItemRule {
                item_id: "ESCAPE_ROPE".to_string(),
                escape_rope_mode: "DIG_WARP".to_string(),
            },
            repel: FieldRepelItemRule {},
            bicycle: FieldItemRule {
                item_id: "BICYCLE".to_string(),
            },
            itemfinder: FieldItemRule {
                item_id: "ITEMFINDER".to_string(),
            },
            squirtbottle: FieldItemRule {
                item_id: "SQUIRTBOTTLE".to_string(),
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

    fn runtime_data_with_currency_caps(max_money: u32, max_coins: u32) -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.currency_constants = crystal_core::systems::economy::CurrencyCatalog(
            [
                ("MAX_MONEY".to_string(), max_money),
                ("MAX_COINS".to_string(), max_coins),
            ]
            .into_iter()
            .collect(),
        );
        data
    }

    fn add_runtime_fly_destination(data: &mut GameDataSet) {
        let mut fly_map = runtime_map();
        fly_map.id = "FlyMap".to_string();
        fly_map.attributes.map_constant = Some("FLY_MAP".to_string());
        fly_map.attributes.environment = Some("town".to_string());
        fly_map.attributes.location = Some("johto".to_string());
        data.map_attributes
            .insert("FlyMap".to_string(), fly_map.attributes.clone());
        data.maps.insert("FlyMap".to_string(), fly_map);
        data.runtime_map_metadata.insert(
            "FLY_MAP".to_string(),
            runtime_map_metadata("FLY_MAP", "FlyMap", 2, 2, "TOWN"),
        );
        data.runtime_spawn_points.insert(
            "14".to_string(),
            crystal_core::systems::special_routines::runtime_spawn_point_from_runtime_tile(
                14,
                "FLY_MAP".to_string(),
                "FlyMap".to_string(),
                2,
                2,
                "FLY".to_string(),
                TilePosition::new(1, 1),
            )
            .expect("fly spawn point"),
        );
        data.fly_destinations.insert(
            "ENGINE_FLYPOINT_FLY_MAP".to_string(),
            crystal_assets::FlyDestination {
                flypoint_flag: "ENGINE_FLYPOINT_FLY_MAP".to_string(),
                destination_spawn_identifier: 14,
                label: "LANDMARK_FLY_MAP".to_string(),
            },
        );
    }

    fn add_runtime_teleport_destination(data: &mut GameDataSet) {
        let mut teleport_map = runtime_map();
        teleport_map.id = "TeleportMap".to_string();
        teleport_map.attributes.map_constant = Some("TELEPORT_MAP".to_string());
        teleport_map.attributes.environment = Some("town".to_string());
        data.map_attributes
            .insert("TeleportMap".to_string(), teleport_map.attributes.clone());
        data.maps.insert("TeleportMap".to_string(), teleport_map);
        data.runtime_map_metadata.insert(
            "TELEPORT_MAP".to_string(),
            runtime_map_metadata("TELEPORT_MAP", "TeleportMap", 2, 3, "TOWN"),
        );
        data.runtime_spawn_points.insert(
            "21".to_string(),
            crystal_core::systems::special_routines::runtime_spawn_point_from_runtime_tile(
                21,
                "TELEPORT_MAP".to_string(),
                "TeleportMap".to_string(),
                2,
                3,
                "TELEPORT".to_string(),
                TilePosition::new(1, 1),
            )
            .expect("teleport spawn point"),
        );
    }

    fn add_runtime_field_encounters(data: &mut GameDataSet) {
        data.field_encounters.insert(
            "RuntimeMap".to_string(),
            FieldEncounterData {
                map_name: "RuntimeMap".to_string(),
                tables: [
                    (
                        FieldEncounterKind::Headbutt.as_key().to_string(),
                        FieldEncounterTable {
                            common: vec![FieldEncounterEntry {
                                weight: 100,
                                species: "CHIKORITA".to_string(),
                                level: 10,
                                sleep_turns_by_time: Default::default(),
                            }],
                            rare: vec![FieldEncounterEntry {
                                weight: 100,
                                species: "CHIKORITA".to_string(),
                                level: 12,
                                sleep_turns_by_time: Default::default(),
                            }],
                        },
                    ),
                    (
                        FieldEncounterKind::RockSmash.as_key().to_string(),
                        FieldEncounterTable {
                            common: vec![FieldEncounterEntry {
                                weight: 100,
                                species: "CHIKORITA".to_string(),
                                level: 15,
                                sleep_turns_by_time: Default::default(),
                            }],
                            rare: Vec::new(),
                        },
                    ),
                ]
                .into_iter()
                .collect(),
            },
        );
    }

    fn add_runtime_rock_smash_global_scripts(data: &mut GameDataSet) {
        data.special_routines.insert(
            "WarpToSpawnPoint".to_string(),
            SpecialRoutineRule::default(),
        );
        data.story_event_script_constants
            .global
            .insert("CHIKORITA".to_string(), 152);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../apps/web/assets/data/story_events/StandardScripts.json");
        let exported: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(path).expect("read canonical StandardScripts export"),
        )
        .expect("parse canonical StandardScripts export");
        let exported = exported
            .get("StandardScripts")
            .and_then(serde_json::Value::as_object)
            .expect("StandardScripts definitions");
        let definitions = data
            .story_events
            .iter_mut()
            .find_map(|catalog| catalog.get_mut("StandardScripts"))
            .and_then(serde_json::Value::as_object_mut)
            .expect("minimal StandardScripts catalog");
        for label in [
            "RockMonEncounter",
            ".no_battle@RockMonEncounter",
            "RockSmashScript",
            ".done@RockSmashScript",
            "RockSmashFromMenuScript",
            "MovementData_RockSmash",
            "UseRockSmashText",
        ] {
            definitions.insert(
                label.to_string(),
                exported.get(label).unwrap_or_else(|| panic!("{label} export")).clone(),
            );
        }
        definitions
            .get_mut("StdScripts")
            .and_then(serde_json::Value::as_array_mut)
            .expect("minimal StdScripts roots")
            .extend([
                serde_json::json!({"command": "add_stdscript", "args": ["RockSmashScript"]}),
                serde_json::json!({"command": "add_stdscript", "args": ["RockSmashFromMenuScript"]}),
            ]);
        data.materialize_global_scripts()
            .expect("materialize exact Rock Smash common scripts");
    }

    fn minimal_runtime_data_with_oak_intro() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.pokemon
            .insert("WOOPER".to_string(), runtime_wooper_species());
        data.pokemon_cries.insert(
            "WOOPER".to_string(),
            PokemonCryMetadata {
                cry: "CRY_WOOPER".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        data.audio.push(
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_30",
                "content-packs/test/music/MUSIC_ROUTE_30.mid",
            )
            .expect("route 30 music asset"),
        );
        data.audio.push(
            ModpackAudioAsset::cry("CRY_WOOPER", "content-packs/test/cries/CRY_WOOPER.mid")
                .expect("wooper cry asset"),
        );
        data.evolutions.0.insert("WOOPER".to_string(), Vec::new());
        data.learnsets.insert(
            "WOOPER".to_string(),
            vec![crystal_core::systems::learnsets::LearnsetEntry(
                1,
                "TACKLE".to_string(),
            )],
        );
        data.menu_icons
            .insert("WOOPER".to_string(), "ICON_WOOPER".to_string());
        data.pokedex_entries.insert(
            "WOOPER".to_string(),
            crystal_core::models::RuntimePokedexEntry {
                species: "WOOPER".to_string(),
                classification: "Water Fish".to_string(),
                height_digits: 4,
                weight_digits: 85,
                pages: vec!["A damp test Pokemon.".to_string()],
            },
        );
        data.pokemon_frontpic_anim.insert(
            "WOOPER".to_string(),
            crystal_core::models::FrontpicAnimProgram {
                commands: vec![crystal_core::models::FrontpicAnimCommand {
                    kind: "frame".to_string(),
                    frame: Some(0),
                    duration: Some(8),
                    ..crystal_core::models::FrontpicAnimCommand::default()
                }],
            },
        );
        data
    }

    fn minimal_runtime_data_with_music() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .music = Some("MUSIC_ROUTE_29".to_string());
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_text_bodies
            .insert(
                "RuntimeGreetingText".to_string(),
                ScriptTextBody {
                    label: "RuntimeGreetingText".to_string(),
                    commands: Vec::new(),
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_text_bodies
            .insert(
                "RuntimeGreetingText".to_string(),
                ScriptTextBody {
                    label: "RuntimeGreetingText".to_string(),
                    commands: Vec::new(),
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_menu_definitions
            .insert(
                "RuntimeMenu".to_string(),
                ScriptMenuDefinition {
                    label: "RuntimeMenu".to_string(),
                    commands: vec![
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "menu_coords".to_string(),
                            args: vec![
                                "0".to_string(),
                                "0".to_string(),
                                "10".to_string(),
                                "8".to_string(),
                            ],
                            command_index: 0,
                        },
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "db".to_string(),
                            args: vec!["2".to_string(), "1".to_string(), "0".to_string()],
                            command_index: 1,
                        },
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "dw".to_string(),
                            args: vec!["RuntimeMenuItems".to_string()],
                            command_index: 2,
                        },
                    ],
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_menu_definitions
            .insert(
                "RuntimeMenuItems".to_string(),
                ScriptMenuDefinition {
                    label: "RuntimeMenuItems".to_string(),
                    commands: vec![
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "db".to_string(),
                            args: vec!["\"First@\"".to_string()],
                            command_index: 0,
                        },
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "db".to_string(),
                            args: vec!["\"Second@\"".to_string()],
                            command_index: 1,
                        },
                    ],
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_vertical_menus
            .insert(
                "RuntimeScript:4".to_string(),
                crystal_assets::ScriptVerticalMenuDefinition {
                    source_script: "RuntimeScript".to_string(),
                    loadmenu_command_index: 3,
                    verticalmenu_command_index: 4,
                    header_label: "RuntimeMenu".to_string(),
                    data_label: Some("RuntimeMenuItems".to_string()),
                    options: vec!["First".to_string(), "Second".to_string()],
                    two_dimensional: false,
                    rows: None,
                    columns: None,
                    spacing: None,
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeScript".to_string(),
                serde_json::json!([
                    {"command": "opentext", "args": []},
                    {"command": "writetext", "args": ["RuntimeText"]},
                    {"command": "waitbutton", "args": []},
                    {"command": "loadmenu", "args": ["RuntimeMenu"]},
                    {"command": "verticalmenu", "args": []},
                    {"command": "elevator", "args": ["RuntimeElevatorData"]},
                    {"command": "yesorno", "args": []}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeShopScript".to_string(),
                serde_json::json!([
                    {"command": "pokemart", "args": ["MARTTYPE_STANDARD", "MART_RUNTIME"]}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_shop_commands
            .push(crystal_core::systems::shop::ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MART_RUNTIME".to_string(),
                source_script: "RuntimeShopScript".to_string(),
                command_index: 0,
            });
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeElevatorData".to_string(),
                serde_json::json!([
                    {"command": "elevfloor", "args": ["FLOOR_2F", "4", "RuntimeMap"]}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeShopScript".to_string(),
                serde_json::json!([
                    {"command": "pokemart", "args": ["MARTTYPE_STANDARD", "MART_RUNTIME"]}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_shop_commands
            .push(crystal_core::systems::shop::ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MART_RUNTIME".to_string(),
                source_script: "RuntimeShopScript".to_string(),
                command_index: 0,
            });
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_elevators
            .insert(
                "RuntimeScript:5".to_string(),
                crystal_assets::ScriptElevatorDefinition {
                    source_script: "RuntimeScript".to_string(),
                    elevator_command_index: 5,
                    data_label: "RuntimeElevatorData".to_string(),
                    floors: vec![ScriptRuntimeElevatorFloor {
                        floor: "FLOOR_2F".to_string(),
                        warp: 4,
                        target_map: "RuntimeMap".to_string(),
                        source_script: "RuntimeElevatorData".to_string(),
                        command_index: 0,
                    }],
                },
            );
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::music(
                "MUSIC_JOHTO_WILD_BATTLE",
                "content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::music(
                "MUSIC_JOHTO_WILD_BATTLE_NIGHT",
                "content-packs/test/music/MUSIC_JOHTO_WILD_BATTLE_NIGHT.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::music(
                "MUSIC_KANTO_WILD_BATTLE",
                "content-packs/test/music/MUSIC_KANTO_WILD_BATTLE.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::sound_effect("SFX_TACKLE", "content-packs/test/sfx/SFX_TACKLE.mid")
                .expect("sfx asset"),
            ModpackAudioAsset::cry(
                "CRY_NIDORAN_M",
                "content-packs/test/cries/CRY_NIDORAN_M.mid",
            )
            .expect("cry asset"),
            ModpackAudioAsset::cry(
                "CRY_CHIKORITA",
                "content-packs/test/cries/CRY_CHIKORITA.mid",
            )
            .expect("cry asset"),
        ];
        data
    }

    fn minimal_runtime_data_with_script_audio_and_map_commands() -> GameDataSet {
        let mut data = minimal_runtime_data_with_music();
        data.pokemon_cries.insert(
            "CHIKORITA".to_string(),
            PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.script_audio_commands = vec![
            ScriptAudioCommand {
                command: "playmusic".to_string(),
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 0,
            },
            ScriptAudioCommand {
                command: "playsound".to_string(),
                audio_id: Some("SFX_TACKLE".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 1,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("CHIKORITA".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 2,
            },
            ScriptAudioCommand {
                command: "PlayMusic".to_string(),
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 3,
            },
        ];
        map.script_map_commands = vec![
            ScriptMapCommand {
                command: "warpfacing".to_string(),
                target_map: Some("RuntimeMap".to_string()),
                x: Some(1),
                y: Some(0),
                facing: Some("RIGHT".to_string()),
                map_setup: None,
                source_script: "RuntimeWarpScript".to_string(),
                command_index: 0,
            },
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("NONE".to_string()),
                x: Some(0),
                y: Some(0),
                facing: None,
                map_setup: None,
                source_script: "RuntimeWarpScript".to_string(),
                command_index: 1,
            },
        ];
        data
    }

    fn minimal_runtime_data_with_text_variable_and_control_commands() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.currency_constants
            .0
            .insert("RUNTIME_PRICE".to_string(), 500);
        data.story_event_script_constants
            .global
            .insert("RUNTIME_BADGES".to_string(), 8);
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.script_text_bodies.insert(
            "RuntimeGreetingText".to_string(),
            ScriptTextBody {
                label: "RuntimeGreetingText".to_string(),
                commands: Vec::new(),
            },
        );
        map.script_text_commands = vec![
            ScriptTextCommand {
                command: "opentext".to_string(),
                text_label: None,
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("RuntimeGreetingText".to_string()),
                source_script: "RuntimeScript".to_string(),
                command_index: 1,
            },
            ScriptTextCommand {
                command: "yesorno".to_string(),
                text_label: None,
                source_script: "RuntimeScript".to_string(),
                command_index: 2,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("runtimegreetingtext".to_string()),
                source_script: "RuntimeScript".to_string(),
                command_index: 3,
            },
            ScriptTextCommand {
                command: "opentext".to_string(),
                text_label: None,
                source_script: "RuntimeAcceptedScript".to_string(),
                command_index: 0,
            },
        ];
        map.script_variable_commands = vec![
            ScriptVariableCommand {
                command: "loadvar".to_string(),
                target: Some("VAR_CALLERID".to_string()),
                value_tokens: vec!["PHONE_BIRDKEEPER_VANCE".to_string()],
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 0,
            },
            ScriptVariableCommand {
                command: "readvar".to_string(),
                target: Some("VAR_CALLERID".to_string()),
                value_tokens: Vec::new(),
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 1,
            },
            ScriptVariableCommand {
                command: "checktime".to_string(),
                target: None,
                value_tokens: vec!["NITE".to_string()],
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 2,
            },
            ScriptVariableCommand {
                command: "setval".to_string(),
                target: None,
                value_tokens: vec!["8".to_string()],
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 3,
            },
        ];
        map.script_control_commands = vec![
            ScriptControlCommand {
                command: "iftrue".to_string(),
                compare_value: None,
                target_label: Some(".Accepted".to_string()),
                resolved_target_script: Some("RuntimeAcceptedScript".to_string()),
                source_script: "RuntimeControlScript".to_string(),
                command_index: 0,
            },
            ScriptControlCommand {
                command: "ifgreater".to_string(),
                compare_value: Some("RUNTIME_BADGES - 1".to_string()),
                target_label: Some(".Enough".to_string()),
                resolved_target_script: Some("RuntimeEnoughScript".to_string()),
                source_script: "RuntimeControlScript".to_string(),
                command_index: 1,
            },
            ScriptControlCommand {
                command: "jumpstd".to_string(),
                compare_value: None,
                target_label: Some("PokecenterSignScript".to_string()),
                resolved_target_script: None,
                source_script: "RuntimeControlScript".to_string(),
                command_index: 2,
            },
        ];
        // Standard scripts are global ASM labels. Production pack loading
        // materializes them into every map before script lookup, so this
        // focused fixture must exercise that same exported-data path.
        data.materialize_global_scripts()
            .expect("materialize standard scripts");
        data
    }

    fn minimal_runtime_data_with_object_and_movement_commands() -> GameDataSet {
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut npc = runtime_object("RUNTIME_NPC", "EVENT_RUNTIME_NPC_HIDDEN");
        npc.x = 1;
        npc.y = 0;
        let mut guide = runtime_object("RUNTIME_GUIDE", "-1");
        guide.x = 0;
        guide.y = 0;
        map.objects = vec![npc, guide];
        map.scripts.insert(
            "RuntimeObjectScript".to_string(),
            serde_json::json!([
                {"command": "moveobject", "args": ["RUNTIME_NPC", "0", "0"]},
                {"command": "turnobject", "args": ["RUNTIME_NPC", "LEFT"]},
                {"command": "disappear", "args": ["RUNTIME_NPC"]},
                {"command": "appear", "args": ["RUNTIME_NPC"]},
                {"command": "applymovement", "args": ["RUNTIME_NPC", "RuntimeNpcMovement"]},
                {"command": "follow", "args": ["RUNTIME_GUIDE", "PLAYER"]},
                {"command": "stopfollow", "args": []},
                {"command": "applymovement", "args": ["RUNTIME_NPC", "runtimenpcmovement"]},
                {"command": "showemote", "args": ["EMOTE_SHOCK", "RUNTIME_NPC", "15"]}
            ]),
        );
        map.script_object_commands = vec![
            ScriptObjectCommand {
                command: "moveobject".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: Some(0),
                y: Some(0),
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 0,
            },
            ScriptObjectCommand {
                command: "turnobject".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: Some("LEFT".to_string()),
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 1,
            },
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 2,
            },
            ScriptObjectCommand {
                command: "appear".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 3,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("RuntimeNpcMovement".to_string()),
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 4,
            },
            ScriptObjectCommand {
                command: "follow".to_string(),
                object_id: Some("RUNTIME_GUIDE".to_string()),
                target_object_id: Some("PLAYER".to_string()),
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 5,
            },
            ScriptObjectCommand {
                command: "stopfollow".to_string(),
                object_id: None,
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 6,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("runtimenpcmovement".to_string()),
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 7,
            },
            ScriptObjectCommand {
                command: "showemote".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: Some("EMOTE_SHOCK".to_string()),
                duration: Some(15),
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 8,
            },
        ];
        map.script_movements = vec![ScriptMovement {
            label: "RuntimeNpcMovement".to_string(),
            source_script: Some("RuntimeObjectScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "teleport_from".to_string(),
                    direction: None,
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "turn_head".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 2,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 3,
                },
            ],
        }];
        data
    }

    fn minimal_runtime_data_with_runtime_commands() -> GameDataSet {
        let mut data = minimal_runtime_data_with_object_and_movement_commands();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.scripts.insert(
            "RuntimeCommandScript".to_string(),
            serde_json::json!([
                {"command": "special", "args": ["FadeOutMusic"]},
                {"command": "pause", "args": ["15"]},
                {"command": "random", "args": ["10"]},
                {"command": "checkver", "args": []},
                {"command": "writevar", "args": ["VAR_BLUECARDBALANCE"]},
                {"command": "getnum", "args": ["STRING_BUFFER_3"]},
                {"command": "setlasttalked", "args": ["RUNTIME_NPC"]},
                {"command": "setlasttalked", "args": ["runtime_npc"]}
            ]),
        );
        map.scripts.insert(
            "FadeOutMusic".to_string(),
            serde_json::json!([{"command": "musicfadeout", "args": ["MUSIC_NONE", "2"]}]),
        );
        map.script_runtime_commands = vec![
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec!["FadeOutMusic".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 0,
            },
            ScriptRuntimeCommand {
                command: "pause".to_string(),
                args: vec!["15".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 1,
            },
            ScriptRuntimeCommand {
                command: "random".to_string(),
                args: vec!["10".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 2,
            },
            ScriptRuntimeCommand {
                command: "checkver".to_string(),
                args: Vec::new(),
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 3,
            },
            ScriptRuntimeCommand {
                command: "writevar".to_string(),
                args: vec!["VAR_BLUECARDBALANCE".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 4,
            },
            ScriptRuntimeCommand {
                command: "getnum".to_string(),
                args: vec!["STRING_BUFFER_3".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 5,
            },
            ScriptRuntimeCommand {
                command: "setlasttalked".to_string(),
                args: vec!["RUNTIME_NPC".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 6,
            },
            ScriptRuntimeCommand {
                command: "setlasttalked".to_string(),
                args: vec!["runtime_npc".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 7,
            },
        ];
        data
    }

    fn minimal_runtime_data_with_swarm_commands() -> GameDataSet {
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.scripts.insert(
            "RuntimeSwarmScript".to_string(),
            serde_json::json!([
                {"command": "swarm", "args": ["SWARM_YANMA", "RUNTIME_MAP"]},
                {"command": "Swarm", "args": ["SWARM_DUNSPARCE", "RUNTIME_MAP"]}
            ]),
        );
        map.script_swarm_commands = vec![
            ScriptSwarmCommand {
                command: "swarm".to_string(),
                swarm_token: "SWARM_YANMA".to_string(),
                map_id: "RUNTIME_MAP".to_string(),
                source_script: "RuntimeSwarmScript".to_string(),
                command_index: 0,
            },
            ScriptSwarmCommand {
                command: "Swarm".to_string(),
                swarm_token: "SWARM_DUNSPARCE".to_string(),
                map_id: "RUNTIME_MAP".to_string(),
                source_script: "RuntimeSwarmScript".to_string(),
                command_index: 1,
            },
        ];
        data
    }

    fn minimal_runtime_data_with_coord_event() -> GameDataSet {
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.attributes.width = 3;
        map.attributes.height = 2;
        map.blocks = vec![0; 6];
        map.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_RUNTIME_ACTIVE".to_string(),
                script_name: Some("RuntimeSceneScript".to_string()),
            }],
        };
        map.events.coord_events = vec![CoordEvent {
            x: 2,
            y: 1,
            scene_id: "SCENE_RUNTIME_ACTIVE".to_string(),
            script_name: "RuntimeCoordScript".to_string(),
        }];
        data.runtime_spawn_points.insert(
            "0".to_string(),
            crystal_core::systems::special_routines::runtime_spawn_point_from_runtime_tile(
                0,
                "RUNTIME_MAP".to_string(),
                "RuntimeMap".to_string(),
                1,
                1,
                "RUNTIME".to_string(),
                TilePosition::new(1, 1),
            )
            .expect("runtime spawn point"),
        );
        data
    }

    fn minimal_runtime_data_with_grass_encounter() -> GameDataSet {
        let mut data = minimal_runtime_data_with_music();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.blocks = vec![1, 0];
        map.events.warps.clear();
        map.map_event_section_commands.clear();
        data.map_blocks
            .insert("RuntimeMap_Blocks".to_string(), "01 00".to_string());
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[
                ("00", &["FLOOR", "FLOOR", "FLOOR", "FLOOR"]),
                (
                    "01",
                    &["TALL_GRASS", "TALL_GRASS", "TALL_GRASS", "TALL_GRASS"],
                ),
            ]),
        );
        let encounter = WildEncounter {
            level: 14,
            species: "CHIKORITA".to_string(),
        };
        let grass_slots = vec![encounter.clone(); 7];
        data.wild_encounters.insert(
            "RuntimeMap".to_string(),
            WildEncounterData {
                map_name: "RuntimeMap".to_string(),
                grass_rates: Some(
                    [
                        ("morning".to_string(), 255),
                        ("day".to_string(), 255),
                        ("night".to_string(), 255),
                    ]
                    .into_iter()
                    .collect(),
                ),
                water_rate: None,
                grass: Some(WildEncounterTable {
                    morning: grass_slots.clone(),
                    day: grass_slots.clone(),
                    night: grass_slots,
                }),
                water: None,
            },
        );
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(1, "TACKLE".to_string())],
        );
        data
    }

    fn minimal_runtime_data_with_fishing() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.tilesets.insert(
            "johto".to_string(),
            test_tileset(&[("00", &["FLOOR", "FLOOR", "WATER", "FLOOR"])]),
        );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .fishing_group = Some("FISHGROUP_RUNTIME".to_string());
        data.fishing = FishingCatalog {
            groups: [(
                "FISHGROUP_RUNTIME".to_string(),
                FishingGroup {
                    bite_threshold: 255,
                    rod_tables: [(
                        ROD_GOOD.to_string(),
                        RodTable {
                            slots: vec![FishingSlot {
                                threshold: 255,
                                species: Some("CHIKORITA".to_string()),
                                level: 9,
                                time_group: None,
                            }],
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
            time_groups: BTreeMap::new(),
            swarm_rules: BTreeMap::new(),
            rod_items: [("GOOD_ROD".to_string(), ROD_GOOD.to_string())]
                .into_iter()
                .collect(),
        };
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(1, "TACKLE".to_string())],
        );
        data
    }

    fn minimal_runtime_data_with_scripted_battles() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.items
            .insert("MASTER_BALL".to_string(), runtime_ball_item("MASTER_BALL"));
        data.items.insert(
            "BERRY".to_string(),
            runtime_item("BERRY", item_pocket("ITEM")),
        );
        let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
        repel.repel_steps = Some(100);
        data.items.insert("REPEL".to_string(), repel);
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(1, "TACKLE".to_string())],
        );
        data.trainers
            .insert(Trainer {
                name: "RIVAL@".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_class: "RIVAL1".to_string(),
                party: vec![TrainerPartyPokemon {
                    species: "CHIKORITA".to_string(),
                    level: 5,
                    item: None,
                    moves: Vec::new(),
                    dvs: Dv::from_non_hp(0, 0, 0, 0),
                }],
                win_quote: "RivalWinText".to_string(),
                lose_quote: "RivalLossText".to_string(),
                items: Vec::new(),
                base_reward: 100,
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_layers: vec!["AI_BASIC".to_string()],
            })
            .expect("trainer inserts");
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.objects.push(runtime_object(
            "RUNTIME_STATIC_MON",
            "EVENT_RUNTIME_STATIC_MON_HIDDEN",
        ));
        map.scripts.insert(
            "RuntimeWildScript".to_string(),
            serde_json::json!([
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeWildText"]},
                {"command": "closetext", "args": []},
                {"command": "loadwildmon", "args": ["CHIKORITA", "6"]},
                {"command": "startbattle", "args": []},
                {"command": "reloadmapafterbattle", "args": []},
                {"command": "setevent", "args": ["EVENT_RUNTIME_WILD_DONE"]},
                {"command": "setflag", "args": ["ENGINE_RUNTIME_WILD_DONE"]},
                {"command": "disappear", "args": ["RUNTIME_STATIC_MON"]},
                {"command": "end", "args": []},
            ]),
        );
        map.scripts.insert(
            "RuntimeTrainerScript".to_string(),
            serde_json::json!([
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeSeenText"]},
                {"command": "closetext", "args": []},
                {"command": "jump", "args": ["RuntimeTrainerScript"]},
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeWinText"]},
                {"command": "closetext", "args": []},
                {"command": "loadtrainer", "args": ["RIVAL1", "RIVAL1"]},
                {"command": "startbattle", "args": []},
                {"command": "reloadmapafterbattle", "args": []},
                {"command": "setevent", "args": ["EVENT_RUNTIME_TRAINER_POST"]},
                {"command": "setflag", "args": ["ENGINE_RUNTIME_TRAINER_POST"]},
                {"command": "end", "args": []},
            ]),
        );
        map.scripts.insert(
            "RuntimeGiftScript".to_string(),
            serde_json::json!([
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeGiftText"]},
                {"command": "closetext", "args": []},
                {"command": "verbosegiveitem", "args": ["BERRY"]},
                {"command": "waitbutton", "args": []},
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeGiftName"]},
                {"command": "closetext", "args": []},
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeGiftText"]},
                {"command": "closetext", "args": []},
                {"command": "waitbutton", "args": []},
                {"command": "givepoke", "args": ["CHIKORITA", "7", "BERRY", "RuntimeGiftName"]},
            ]),
        );
        map.scripts.insert(
            "RuntimeEggScript".to_string(),
            serde_json::json!([
                {"command": "opentext", "args": []},
                {"command": "writetext", "args": ["RuntimeGiftText"]},
                {"command": "closetext", "args": []},
                {"command": "giveegg", "args": ["CHIKORITA", "EGG_LEVEL"]},
            ]),
        );
        map.script_runtime_commands.push(ScriptRuntimeCommand {
            command: "givepoke".to_string(),
            args: vec![
                "CHIKORITA".to_string(),
                "7".to_string(),
                "BERRY".to_string(),
                "RuntimeGiftName".to_string(),
            ],
            source_script: "RuntimeGiftScript".to_string(),
            command_index: 12,
        });
        map.scripted_wild_battles.push(ScriptedWildBattle {
            source_script: "RuntimeWildScript".to_string(),
            loadwildmon_command_index: 3,
            startbattle_command_index: 4,
            request: StaticWildBattleRequest {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE_NIGHT".to_string(),
                species: "CHIKORITA".to_string(),
                level: 6,
                source_script: "RuntimeWildScript".to_string(),
            },
        });
        map.scripted_trainer_battles.push(ScriptedTrainerBattle {
            source_script: "RuntimeTrainerScript".to_string(),
            loadtrainer_command_index: 7,
            startbattle_command_index: 8,
            request: {
                let mut request =
                    TrainerBattleRequest::new("RIVAL1", "RIVAL1", "EVENT_BEAT_RUNTIME_RIVAL");
                request.seen_text = "RuntimeSeenText".to_string();
                request.win_text = "RuntimeWinText".to_string();
                request.loss_text = "RuntimeLossText".to_string();
                request.source_script = "RuntimeTrainerScript".to_string();
                request
            },
        });
        map.script_map_commands.extend([
            ScriptMapCommand {
                command: "reloadmapafterbattle".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: None,
                source_script: "RuntimeWildScript".to_string(),
                command_index: 5,
            },
            ScriptMapCommand {
                command: "reloadmapafterbattle".to_string(),
                target_map: None,
                x: None,
                y: None,
                facing: None,
                map_setup: None,
                source_script: "RuntimeTrainerScript".to_string(),
                command_index: 9,
            },
        ]);
        map.script_flag_commands.extend([
            ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: "EVENT_RUNTIME_WILD_DONE".to_string(),
                source_script: "RuntimeWildScript".to_string(),
                command_index: 6,
            },
            ScriptFlagCommand {
                command: "setflag".to_string(),
                flag_id: "ENGINE_RUNTIME_WILD_DONE".to_string(),
                source_script: "RuntimeWildScript".to_string(),
                command_index: 7,
            },
            ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: "EVENT_RUNTIME_TRAINER_POST".to_string(),
                source_script: "RuntimeTrainerScript".to_string(),
                command_index: 10,
            },
            ScriptFlagCommand {
                command: "setflag".to_string(),
                flag_id: "ENGINE_RUNTIME_TRAINER_POST".to_string(),
                source_script: "RuntimeTrainerScript".to_string(),
                command_index: 11,
            },
        ]);
        map.script_object_commands.push(ScriptObjectCommand {
            command: "disappear".to_string(),
            object_id: Some("RUNTIME_STATIC_MON".to_string()),
            target_object_id: None,
            x: None,
            y: None,
            direction: None,
            movement: None,
            emote: None,
            duration: None,
            source_script: "RuntimeWildScript".to_string(),
            command_index: 8,
        });
        map.gift_pokemon_scripts.push(GiftPokemonScript {
            species_id: "CHIKORITA".to_string(),
            level_token: "7".to_string(),
            level: 7,
            held_item_id: Some("BERRY".to_string()),
            nickname_label: Some("RuntimeGiftName".to_string()),
            ot_label: Some("PLAYER".to_string()),
            source_script: "RuntimeGiftScript".to_string(),
            command_index: 12,
            egg: false,
        });
        map.gift_pokemon_scripts.push(GiftPokemonScript {
            species_id: "CHIKORITA".to_string(),
            level_token: "EGG_LEVEL".to_string(),
            level: 5,
            held_item_id: None,
            nickname_label: None,
            ot_label: None,
            source_script: "RuntimeEggScript".to_string(),
            command_index: 3,
            egg: true,
        });
        data
    }

    fn minimal_runtime_data_with_battle_rewards() -> GameDataSet {
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves.insert(
            "RAZOR_LEAF".to_string(),
            runtime_move_named("RAZOR_LEAF", 25),
        );
        let mut chikorita = runtime_species();
        chikorita.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        chikorita.base_exp = 64;
        data.pokemon.insert("CHIKORITA".to_string(), chikorita);
        let mut bayleef =
            PokemonSpecies::new_for_tests("BAYLEEF", BaseStats::new(60, 62, 80, 60, 63, 80));
        bayleef.int_id = 2;
        bayleef.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        bayleef.base_exp = 141;
        data.pokemon.insert("BAYLEEF".to_string(), bayleef);
        add_runtime_species_presentation(&mut data, "BAYLEEF");
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(16, "RAZOR_LEAF".to_string()),
            ],
        );
        data.learnsets.insert("BAYLEEF".to_string(), Vec::new());
        data.evolutions.0.insert(
            "CHIKORITA".to_string(),
            vec![EvolutionEntry::level("BAYLEEF", 16)],
        );
        data.evolutions.0.insert("BAYLEEF".to_string(), Vec::new());
        sync_runtime_move_tables(&mut data);
        data
    }

    #[test]
    fn runtime_bootstrap_loads_compiled_pack_and_declared_midi_assets() {
        let root = temp_repository_root("loads");
        let data_root = root.join("apps/web/assets/data");
        write_midi(&data_root.join("content-packs/test/music/MUSIC_ROUTE_29.mid"));
        write_midi(&data_root.join("content-packs/test/sfx/SFX_ITEM.mid"));
        write_midi(&data_root.join("content-packs/test/cries/CRY_NIDORAN_M.mid"));
        let mut data = verified_runtime_bootstrap_data();
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::sound_effect("SFX_ITEM", "content-packs/test/sfx/SFX_ITEM.mid")
                .expect("sfx asset"),
            ModpackAudioAsset::cry(
                "CRY_NIDORAN_M",
                "content-packs/test/cries/CRY_NIDORAN_M.mid",
            )
            .expect("cry asset"),
        ];
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report());
        crystal_assets::write_compiled_game_pack_for_tests(
            data_root.join("runtime.crystalpack"),
            &pack,
        )
        .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);

        let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect("load runtime");

        assert_eq!(runtime.modpack.id(), "core-modular");
        assert_eq!(runtime.modpack.hash().len(), 64);
        assert!(
            runtime
                .audio
                .program(AudioKind::Music, "MUSIC_ROUTE_29")
                .is_some()
        );
        assert!(
            runtime
                .audio
                .program(AudioKind::SoundEffect, "SFX_ITEM")
                .is_some()
        );
        assert!(
            runtime
                .audio
                .program(AudioKind::Cry, "CRY_NIDORAN_M")
                .is_some()
        );
        let summary = runtime.boot_summary();
        assert_eq!(summary.modpack_id, "core-modular");
        assert_eq!(
            summary.pack_content_hash,
            runtime.pack_identity().content_hash
        );
        assert_eq!(summary.pack_content_hash.len(), 64);
        assert_eq!(summary.pokemon_species, 1);
        assert_eq!(summary.moves, 1);
        assert_eq!(summary.maps, 1);
        assert_eq!(summary.music_tracks, 1);
        assert_eq!(summary.sound_effects, 1);
        assert_eq!(summary.cries, 1);
        assert_eq!(runtime.audio.manifest().music.len(), 1);
        assert_eq!(runtime.audio.manifest().sound_effects.len(), 1);
        assert_eq!(runtime.audio.manifest().cries.len(), 1);
        assert!(
            runtime
                .audio
                .manifest()
                .music
                .contains_key("MUSIC_ROUTE_29")
        );
        assert!(
            runtime
                .audio
                .manifest()
                .sound_effects
                .contains_key("SFX_ITEM")
        );
        assert!(runtime.audio.manifest().cries.contains_key("CRY_NIDORAN_M"));
        assert_eq!(
            runtime
                .audio
                .playback()
                .music
                .get("MUSIC_ROUTE_29")
                .expect("music playback plan")
                .loop_policy,
            ModpackAudioLoopPolicy::Loop
        );
        assert_eq!(
            runtime
                .audio
                .playback()
                .sound_effects
                .get("SFX_ITEM")
                .expect("sfx playback plan")
                .loop_policy,
            ModpackAudioLoopPolicy::Once
        );
        assert_eq!(
            runtime
                .audio
                .playback()
                .cries
                .get("CRY_NIDORAN_M")
                .expect("cry playback plan")
                .mode,
            ModpackAudioPlaybackMode::SequencedMidi
        );
        let resolved_music = runtime
            .audio
            .resolve_audio_event(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "playmusic".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::Music,
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 1,
            })
            .expect("resolve exact music playback");
        let RuntimeResolvedAudioPlaybackKind::Play { audio_id, playback } = resolved_music.kind
        else {
            panic!("expected resolved music playback");
        };
        assert_eq!(audio_id, "MUSIC_ROUTE_29");
        assert_eq!(playback.loop_policy, ModpackAudioLoopPolicy::Loop);
        let invalid_wait = runtime
            .audio
            .resolve_audio_event(crystal_core::state::ScriptAudioRuntimeEvent {
                command: "waitsfx".to_string(),
                kind: crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect,
                audio_id: Some("SFX_ITEM".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 2,
            })
            .expect_err("wait events must not infer an audio asset")
            .to_string();
        assert!(
            invalid_wait.contains("must not carry audio_id or fade_frames"),
            "{invalid_wait}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_file_bytes_partition_materialization_cache_identity() {
        let key = "data/runtime-cache-probe.bin".to_string();
        let mut runtime_files_a = complete_vendor_runtime_files();
        runtime_files_a.insert(key.clone(), b"pack-a".to_vec());
        let mut runtime_files_b = complete_vendor_runtime_files();
        runtime_files_b.insert(key, b"pack-b".to_vec());
        let (root_a, _, runtime_a) = load_minimal_compiled_runtime_with_runtime_files(
            "runtime-file-cache-a",
            runtime_files_a,
        );
        let (root_b, _, runtime_b) = load_minimal_compiled_runtime_with_runtime_files(
            "runtime-file-cache-b",
            runtime_files_b,
        );
        let expected_mount_a = std::env::temp_dir().join(format!(
            "crystal-pack-assets-{}-{}",
            std::process::id(),
            runtime_a.pack_identity().content_hash
        ));
        let expected_mount_b = std::env::temp_dir().join(format!(
            "crystal-pack-assets-{}-{}",
            std::process::id(),
            runtime_b.pack_identity().content_hash
        ));
        let _ = std::fs::remove_dir_all(&expected_mount_a);
        let _ = std::fs::remove_dir_all(&expected_mount_b);

        let mounted_a = runtime_a
            .materialize_runtime_files()
            .expect("materialize pack A runtime files");
        assert_eq!(
            std::fs::read(
                mounted_a
                    .runtime_assets()
                    .join("data/runtime-cache-probe.bin")
            )
            .expect("read pack A cache byte probe"),
            b"pack-a"
        );
        let mounted_b = runtime_b
            .materialize_runtime_files()
            .expect("materialize pack B runtime files");
        assert_eq!(
            std::fs::read(
                mounted_b
                    .runtime_assets()
                    .join("data/runtime-cache-probe.bin")
            )
            .expect("read pack B cache byte probe"),
            b"pack-b"
        );
        assert_ne!(
            runtime_a.pack_identity().content_hash,
            runtime_b.pack_identity().content_hash
        );
        assert_ne!(mounted_a.repository_root, mounted_b.repository_root);

        let _ = std::fs::remove_dir_all(expected_mount_a);
        let _ = std::fs::remove_dir_all(expected_mount_b);
        let _ = std::fs::remove_dir_all(root_a);
        let _ = std::fs::remove_dir_all(root_b);
    }

    #[test]
    fn runtime_file_bundle_materializes_exact_vendor_dependency_closure_from_empty_root() {
        let runtime_files = complete_vendor_runtime_files();
        let (root, asset_root, runtime) = load_minimal_compiled_runtime_with_runtime_files(
            "runtime-file-vendor-closure",
            runtime_files.clone(),
        );
        assert!(
            !asset_root.vendor_pokecrystal().exists(),
            "runtime fixture root must not contain a repository vendor checkout"
        );
        assert_eq!(
            runtime
                .runtime_files
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            crystal_assets::REQUIRED_VENDOR_RUNTIME_FILE_KEYS
        );
        let expected_mount = std::env::temp_dir().join(format!(
            "crystal-pack-assets-{}-{}",
            std::process::id(),
            runtime.pack_identity().content_hash
        ));
        let _ = std::fs::remove_dir_all(&expected_mount);

        let mounted = runtime
            .materialize_runtime_files()
            .expect("materialize complete vendor runtime bundle");
        for &key in crystal_assets::REQUIRED_VENDOR_RUNTIME_FILE_KEYS {
            assert_eq!(
                std::fs::read(mounted.repository_root.join(key)).unwrap_or_else(|error| {
                    panic!("read materialized vendor asset {key}: {error}")
                }),
                runtime_files[key],
                "materialized vendor asset bytes must come from the compiled pack: {key}"
            );
        }

        let _ = std::fs::remove_dir_all(expected_mount);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_file_materialization_rejects_path_aliases_before_writing() {
        let (root, _, runtime) = load_minimal_compiled_runtime("runtime-file-invalid-paths");
        let absolute_escape = root.join("absolute-escape.bin");
        let cases = [
            (
                "data/./current-alias.bin".to_string(),
                "must not include current-directory components",
            ),
            (
                "../parent-escape.bin".to_string(),
                "must not traverse parent directories",
            ),
            (
                r"data\..\parent-escape.bin".to_string(),
                "must not traverse parent directories",
            ),
            (
                "data//double-alias.bin".to_string(),
                "must not contain empty path components",
            ),
            (
                r"data\\double-alias.bin".to_string(),
                "must not contain empty path components",
            ),
            (
                "data/trailing-alias.bin/".to_string(),
                "must not contain empty path components",
            ),
            (
                r"data\trailing-alias.bin\".to_string(),
                "must not contain empty path components",
            ),
            (r"C:\absolute-escape.bin".to_string(), "must be relative"),
            (
                r"\\server\share\escape.bin".to_string(),
                "must be relative",
            ),
            (
                r"data\portable-separator.bin".to_string(),
                "must use forward-slash separators",
            ),
            (
                absolute_escape.to_string_lossy().into_owned(),
                "must be relative",
            ),
        ];

        for (key, expected) in cases {
            let mut invalid_runtime = runtime.clone();
            invalid_runtime.runtime_files =
                BTreeMap::from([(key.clone(), b"must-not-write".to_vec())]);
            let mount = std::env::temp_dir().join(format!(
                "crystal-pack-assets-{}-{}",
                std::process::id(),
                invalid_runtime.pack_identity().content_hash
            ));
            let _ = std::fs::remove_dir_all(&mount);

            let error = invalid_runtime
                .materialize_runtime_files()
                .expect_err("aliased runtime-file key must be rejected")
                .to_string();
            assert!(error.contains(expected) && error.contains(&key), "{error}");
            assert!(
                !mount.exists(),
                "runtime-file validation must finish before creating the mount"
            );
            assert!(
                !absolute_escape.exists(),
                "absolute runtime-file key must never be written"
            );
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn release_audio_programs_do_not_reference_external_pcm_files() {
        for (path, source) in [
            (
                "crystal-audio/src/lib.rs",
                include_str!("../../../crystal-audio/src/lib.rs"),
            ),
            ("crystal-bevy/src/lib.rs", include_str!("../lib.rs")),
            (
                "crystal-bevy/src/bevy_shell.rs",
                include_str!("../bevy_shell.rs"),
            ),
            (
                "crystal-bevy/src/bevy_shell/graphics_assets.rs",
                include_str!("../bevy_shell/graphics_assets.rs"),
            ),
        ] {
            assert!(
                !source.contains("PcmFile"),
                "{path} must not retain an external-file audio source"
            );
        }
        assert!(
            !include_str!("../lib.rs").contains("from_game_data_with_external_audio_root"),
            "the zero-caller external audio-root constructor must stay removed"
        );
    }

    #[test]
    fn regenerated_pack_keeps_pcm_lazy_until_playback() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .expect("repository root");
        let pack_path = root.join("content-packs/core-modular.crystalpack");
        let loaded = crystal_assets::read_loaded_verified_compiled_game_pack(&pack_path)
            .expect("load regenerated core pack");
        let runtime = CrystalRuntime::from_loaded_compiled_pack(&AssetRoot::new(root), loaded)
            .expect("load regenerated runtime");
        let program = runtime
            .audio()
            .program(AudioKind::Music, "MUSIC_TITLE")
            .expect("title music program");
        match &program.source {
            AudioProgramSource::PcmGzip {
                format, byte_len, ..
            } => {
                assert_eq!(format.sample_rate_hz, 22_050);
                assert_eq!(format.channels, 1);
                assert!(*byte_len > 0);
            }
            other => panic!("title music was expanded eagerly: {other:?}"),
        }
    }

    #[test]
    fn runtime_player_gender_is_authoritative_state() {
        let root = temp_repository_root("player-gender");
        let asset_root = AssetRoot::new(&root);
        let data = minimal_runtime_data();
        let report = report_for(&data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report),
            identity(),
        )
        .expect("runtime");
        let mut shell = RuntimeGameShell::new_game(asset_root, runtime, 0).expect("game shell");

        let initial = shell.snapshot().expect("initial snapshot");
        assert_eq!(initial.trainer.player_gender, PLAYER_GENDER_MALE);
        let gender_set = shell
            .set_player_gender(PLAYER_GENDER_FEMALE)
            .expect("set player gender");
        assert_eq!(gender_set.player_gender_before, PLAYER_GENDER_MALE);
        assert_eq!(gender_set.player_gender_after, PLAYER_GENDER_FEMALE);
        assert_eq!(
            shell
                .snapshot()
                .expect("snapshot after player gender")
                .trainer
                .player_gender,
            PLAYER_GENDER_FEMALE
        );
        assert!(shell.set_player_gender(2).is_err());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn visible_title_new_game_does_not_eagerly_load_stale_continue_save() {
        let (root, asset_root, runtime) = load_minimal_compiled_runtime("title-stale-save");
        let stale_save = root.join("target/crystal-bevy/saves/core-modular.crystalsave");
        std::fs::create_dir_all(stale_save.parent().expect("save parent"))
            .expect("create save dir");
        std::fs::write(&stale_save, b"not a crystal runtime save").expect("write stale save");

        let smoke = smoke_visible_shell_title(asset_root, runtime, 0, Some(stale_save), false)
            .expect("title new game must boot without loading stale Continue save");

        assert_eq!(smoke.selected, "NEW_GAME");
        assert_eq!(smoke.map, "RuntimeMap");
        assert_eq!(smoke.tile_x, 0);
        assert_eq!(smoke.tile_y, 0);
        assert_eq!(smoke.saved_frame, None);
        assert_eq!(
            smoke.title_entries,
            vec![
                " CONTINUE".to_string(),
                ">NEW GAME".to_string(),
                " OPTION".to_string()
            ]
        );
        let (reject_root, reject_asset_root, reject_runtime) =
            load_minimal_compiled_runtime("title-stale-save-reject");
        let reject_stale_save =
            reject_root.join("target/crystal-bevy/saves/core-modular.crystalsave");
        std::fs::create_dir_all(reject_stale_save.parent().expect("save parent"))
            .expect("create save dir");
        std::fs::write(&reject_stale_save, b"not a crystal runtime save")
            .expect("write stale save");
        let error = smoke_visible_shell_title(
            reject_asset_root,
            reject_runtime,
            0,
            Some(reject_stale_save),
            true,
        )
        .expect_err("title Continue must reject an invalid configured save");
        assert!(
            error.to_string().contains("title Continue rejected"),
            "{error:#}"
        );
        let _ = std::fs::remove_dir_all(reject_root);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn visible_title_recovers_backup_only_save_before_offering_continue() {
        let (root, asset_root, runtime) = load_minimal_compiled_runtime("title-backup-save");
        let save_path = root.join("target/crystal-bevy/saves/core-modular.crystalsave");
        let mut shell = RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), 0)
            .expect("game shell");
        shell.save(&save_path).expect("write primary save");
        shell.save(&save_path).expect("rotate primary save to backup");
        let backup_path = PathBuf::from(format!("{}.bak", save_path.display()));
        assert!(backup_path.exists(), "second save must create a backup");
        std::fs::remove_file(&save_path).expect("remove primary save");

        let smoke = smoke_visible_shell_title(
            asset_root,
            runtime,
            0,
            Some(save_path.clone()),
            true,
        )
        .expect("title Continue must recover the validated backup save");

        assert_eq!(smoke.selected, "CONTINUE");
        assert_eq!(
            smoke.title_entries,
            vec![
                ">CONTINUE".to_string(),
                " NEW GAME".to_string(),
                " OPTION".to_string()
            ]
        );
        assert!(
            save_path.exists(),
            "reading the valid backup must restore the primary save"
        );
        assert_eq!(smoke.saved_frame, Some(0));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn visible_title_new_game_name_input_is_controlled_before_spawn() {
        let (root, asset_root, runtime) = load_minimal_compiled_runtime("title-name-input");

        let smoke = smoke_visible_shell_title_name_input(asset_root, runtime, 0, None, "AB")
            .expect("visible title name input smoke must type and confirm player name");

        assert_eq!(smoke.selected, "NEW_GAME");
        assert_eq!(
            smoke.title_entries,
            vec![">NEW GAME".to_string(), " OPTION".to_string()]
        );
        assert_eq!(
            smoke.initial_name_entries.first().map(String::as_str),
            Some("NAME ENTRY")
        );
        assert!(
            smoke
                .initial_name_entries
                .iter()
                .any(|entry| entry == "YOUR NAME?"),
            "{:?}",
            smoke.initial_name_entries
        );
        assert!(
            smoke
                .initial_name_entries
                .iter()
                .any(|entry| entry == "A B C D E F G H I"),
            "{:?}",
            smoke.initial_name_entries
        );
        assert!(
            smoke
                .initial_name_entries
                .iter()
                .any(|entry| entry == "lower  DEL   END "),
            "{:?}",
            smoke.initial_name_entries
        );
        assert!(
            smoke
                .typed_name_entries
                .iter()
                .any(|entry| entry == "NAME AB_"),
            "{:?}",
            smoke.typed_name_entries
        );
        assert_eq!(smoke.trainer_name, "AB");
        assert_eq!(smoke.map, "RuntimeMap");
        assert_eq!(smoke.tile_x, 0);
        assert_eq!(smoke.tile_y, 0);
        assert_ne!(smoke.state_hash.hash(), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn visible_overworld_smoke_replays_same_inputs_deterministically() {
        let input_frames = vec![
            vec![GameButton::Right],
            vec![GameButton::Right],
            vec![GameButton::Down],
            vec![GameButton::Left],
            vec![GameButton::Up],
        ];
        let (first_root, first_asset_root, first_runtime) =
            load_minimal_compiled_runtime("visible-overworld-deterministic-a");
        let first = smoke_visible_shell_overworld(
            first_asset_root,
            first_runtime,
            BevyShellStart::NewGame {
                spawn_identifier: 0,
            },
            BevyShellConfig {
                smoke_player_name: Some("AB".to_string()),
                ..Default::default()
            },
            &input_frames,
            None,
        )
        .expect("first visible overworld smoke");
        let (second_root, second_asset_root, second_runtime) =
            load_minimal_compiled_runtime("visible-overworld-deterministic-b");
        let second = smoke_visible_shell_overworld(
            second_asset_root,
            second_runtime,
            BevyShellStart::NewGame {
                spawn_identifier: 0,
            },
            BevyShellConfig {
                smoke_player_name: Some("AB".to_string()),
                ..Default::default()
            },
            &input_frames,
            None,
        )
        .expect("second visible overworld smoke");

        assert_eq!(first, second);
        assert_eq!(first.frames, input_frames.len());
        assert_eq!(first.start_map, "RuntimeMap");
        assert_eq!(first.final_map, "RuntimeMap");
        assert_eq!(first.state_hash.frame(), input_frames.len() as u64);
        assert_ne!(first.state_hash.hash(), 0);
        let _ = std::fs::remove_dir_all(second_root);
        let _ = std::fs::remove_dir_all(first_root);
    }

    #[test]
    fn runtime_game_shell_ticks_snapshots_and_saves_against_exact_pack() {
        std::thread::Builder::new()
            .name("runtime-game-shell-large-stack".to_string())
            .stack_size(32 * 1024 * 1024)
            .spawn(runtime_game_shell_ticks_snapshots_and_saves_against_exact_pack_impl)
            .expect("spawn large-stack runtime shell test")
            .join()
            .expect("runtime shell test thread");
    }

    fn runtime_game_shell_ticks_snapshots_and_saves_against_exact_pack_impl() {
        let root = temp_repository_root("shell");
        let asset_root = AssetRoot::new(root.clone());
        let mut data = verified_runtime_bootstrap_data();
        add_runtime_fly_destination(&mut data);
        data.trainers
            .insert(Trainer {
                name: "RIVAL@".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_class: "RIVAL1".to_string(),
                party: vec![TrainerPartyPokemon {
                    species: "CHIKORITA".to_string(),
                    level: 5,
                    item: None,
                    moves: Vec::new(),
                    dvs: Dv::from_non_hp(0, 0, 0, 0),
                }],
                win_quote: "RivalWinText".to_string(),
                lose_quote: "RivalLossText".to_string(),
                items: Vec::new(),
                base_reward: 100,
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_layers: vec!["AI_BASIC".to_string()],
            })
            .expect("trainer inserts");
        data.items
            .insert("MASTER_BALL".to_string(), runtime_ball_item("MASTER_BALL"));
        data.items.insert(
            "BERRY".to_string(),
            runtime_item("BERRY", item_pocket("ITEM")),
        );
        let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
        repel.repel_steps = Some(100);
        data.items.insert("REPEL".to_string(), repel);
        let encounter = WildEncounter {
            level: 14,
            species: "CHIKORITA".to_string(),
        };
        let grass_slots = vec![encounter.clone(); 7];
        data.wild_encounters.insert(
            "RuntimeMap".to_string(),
            WildEncounterData {
                map_name: "RuntimeMap".to_string(),
                grass_rates: Some(
                    [
                        ("morning".to_string(), 255),
                        ("day".to_string(), 255),
                        ("night".to_string(), 255),
                    ]
                    .into_iter()
                    .collect(),
                ),
                water_rate: None,
                grass: Some(WildEncounterTable {
                    morning: grass_slots.clone(),
                    day: grass_slots.clone(),
                    night: grass_slots,
                }),
                water: None,
            },
        );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .music = Some("MUSIC_ROUTE_29".to_string());
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_text_bodies
            .insert(
                "RuntimeGreetingText".to_string(),
                ScriptTextBody {
                    label: "RuntimeGreetingText".to_string(),
                    commands: Vec::new(),
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_menu_definitions
            .insert(
                "RuntimeMenu".to_string(),
                ScriptMenuDefinition {
                    label: "RuntimeMenu".to_string(),
                    commands: vec![
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "menu_coords".to_string(),
                            args: vec![
                                "0".to_string(),
                                "0".to_string(),
                                "10".to_string(),
                                "8".to_string(),
                            ],
                            command_index: 0,
                        },
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "db".to_string(),
                            args: vec!["2".to_string(), "1".to_string(), "0".to_string()],
                            command_index: 1,
                        },
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "dw".to_string(),
                            args: vec!["RuntimeMenuItems".to_string()],
                            command_index: 2,
                        },
                    ],
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_menu_definitions
            .insert(
                "RuntimeMenuItems".to_string(),
                ScriptMenuDefinition {
                    label: "RuntimeMenuItems".to_string(),
                    commands: vec![
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "db".to_string(),
                            args: vec!["\"First@\"".to_string()],
                            command_index: 0,
                        },
                        crystal_core::systems::script_text::ScriptMenuCommand {
                            command: "db".to_string(),
                            args: vec!["\"Second@\"".to_string()],
                            command_index: 1,
                        },
                    ],
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_vertical_menus
            .insert(
                "RuntimeScript:4".to_string(),
                crystal_assets::ScriptVerticalMenuDefinition {
                    source_script: "RuntimeScript".to_string(),
                    loadmenu_command_index: 3,
                    verticalmenu_command_index: 4,
                    header_label: "RuntimeMenu".to_string(),
                    data_label: Some("RuntimeMenuItems".to_string()),
                    options: vec!["First".to_string(), "Second".to_string()],
                    two_dimensional: false,
                    rows: None,
                    columns: None,
                    spacing: None,
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeScript".to_string(),
                serde_json::json!([
                    {"command": "opentext", "args": []},
                    {"command": "writetext", "args": ["RuntimeText"]},
                    {"command": "waitbutton", "args": []},
                    {"command": "loadmenu", "args": ["RuntimeMenu"]},
                    {"command": "verticalmenu", "args": []},
                    {"command": "elevator", "args": ["RuntimeElevatorData"]}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeElevatorData".to_string(),
                serde_json::json!([
                    {"command": "elevfloor", "args": ["FLOOR_2F", "4", "RuntimeMap"]}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .scripts
            .insert(
                "RuntimeShopScript".to_string(),
                serde_json::json!([
                    {"command": "pokemart", "args": ["MARTTYPE_STANDARD", "MART_RUNTIME"]}
                ]),
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_shop_commands
            .push(crystal_core::systems::shop::ScriptShopCommand {
                command: "pokemart".to_string(),
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "MART_RUNTIME".to_string(),
                source_script: "RuntimeShopScript".to_string(),
                command_index: 0,
            });
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .script_elevators
            .insert(
                "RuntimeScript:5".to_string(),
                crystal_assets::ScriptElevatorDefinition {
                    source_script: "RuntimeScript".to_string(),
                    elevator_command_index: 5,
                    data_label: "RuntimeElevatorData".to_string(),
                    floors: vec![ScriptRuntimeElevatorFloor {
                        floor: "FLOOR_2F".to_string(),
                        warp: 4,
                        target_map: "RuntimeMap".to_string(),
                        source_script: "RuntimeElevatorData".to_string(),
                        command_index: 0,
                    }],
                },
            );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .gift_pokemon_scripts
            .push(GiftPokemonScript {
                species_id: "CHIKORITA".to_string(),
                level_token: "7".to_string(),
                level: 7,
                held_item_id: Some("BERRY".to_string()),
                nickname_label: Some("RuntimeGiftName".to_string()),
                ot_label: None,
                source_script: "RuntimeGiftScript".to_string(),
                command_index: 12,
                egg: false,
            });
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::sound_effect("SFX_TACKLE", "content-packs/test/sfx/SFX_TACKLE.mid")
                .expect("sfx asset"),
            ModpackAudioAsset::cry(
                "CRY_NIDORAN_M",
                "content-packs/test/cries/CRY_NIDORAN_M.mid",
            )
            .expect("cry asset"),
        ];
        let report = report_for(&data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report.clone()),
            identity(),
        )
        .expect("runtime");
        let mut shell =
            RuntimeGameShell::new_game(asset_root.clone(), runtime, 0).expect("game shell");

        let initial = shell.snapshot().expect("initial snapshot");
        assert_eq!(initial.boot.modpack_id, "core-modular");
        assert_eq!(initial.overworld.map_name, "RuntimeMap");
        assert_eq!(initial.phase, RuntimeShellPhase::Overworld);
        assert_eq!(initial.progression.last_spawn_identifier, Some(0));
        assert_eq!(initial.trainer.player_gender, PLAYER_GENDER_MALE);
        assert!(shell.runtime().has_item("MASTER_BALL"));
        assert!(!shell.runtime().has_item("master_ball"));
        assert!(shell.runtime().item_ids().contains("MASTER_BALL"));
        assert!(shell.runtime().require_item("MASTER_BALL").is_ok());
        assert!(shell.runtime().require_item("master_ball").is_err());
        assert!(shell.runtime().has_move("TACKLE"));
        assert!(!shell.runtime().has_move("tackle"));
        assert!(shell.runtime().move_ids().contains("TACKLE"));
        assert!(shell.runtime().require_move("TACKLE").is_ok());
        assert!(shell.runtime().require_move("tackle").is_err());
        let move_battle_data = RuntimeMoveBattleDataKey {
            move_id: "TACKLE".to_string(),
            name: "TACKLE".to_string(),
            move_type: "NORMAL".to_string(),
            power: 40,
            accuracy: 100,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        };
        let wrong_move_battle_pp = RuntimeMoveBattleDataKey {
            pp: 34,
            ..move_battle_data.clone()
        };
        let wrong_move_battle_effect = RuntimeMoveBattleDataKey {
            effect: "normal_hit".to_string(),
            ..move_battle_data.clone()
        };
        assert!(shell.runtime().has_move_battle_data(&move_battle_data));
        assert!(!shell.runtime().has_move_battle_data(&wrong_move_battle_pp));
        assert!(
            !shell
                .runtime()
                .has_move_battle_data(&wrong_move_battle_effect)
        );
        assert!(
            shell
                .runtime()
                .move_battle_data_keys()
                .contains(&move_battle_data)
        );
        assert!(
            shell
                .runtime()
                .require_move_battle_data(&move_battle_data)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_move_battle_data(&wrong_move_battle_pp)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_move_battle_data(&wrong_move_battle_effect)
                .is_err()
        );
        assert!(shell.runtime().has_species("CHIKORITA"));
        assert!(!shell.runtime().has_species("chikorita"));
        assert!(shell.runtime().species_ids().contains("CHIKORITA"));
        assert!(shell.runtime().require_species("CHIKORITA").is_ok());
        assert!(shell.runtime().require_species("chikorita").is_err());
        let fixture_species = runtime_species();
        let species_battle_data = RuntimeSpeciesBattleDataKey {
            species_id: fixture_species.id.clone(),
            int_id: fixture_species.int_id,
            base_hp: fixture_species.base_stats.hp,
            base_attack: fixture_species.base_stats.attack,
            base_defense: fixture_species.base_stats.defense,
            base_speed: fixture_species.base_stats.speed,
            base_special_attack: fixture_species.base_stats.special_attack,
            base_special_defense: fixture_species.base_stats.special_defense,
            type1: fixture_species.type1.clone(),
            type2: fixture_species.type2.clone(),
            catch_rate: fixture_species.catch_rate,
            base_exp: fixture_species.base_exp,
            item1: fixture_species.item1.clone(),
            item2: fixture_species.item2.clone(),
            gender_ratio: fixture_species.gender_ratio,
            step_cycles_to_hatch: fixture_species.step_cycles_to_hatch,
            growth_rate: fixture_species.growth_rate.clone(),
            egg_group1: fixture_species.egg_group1.clone(),
            egg_group2: fixture_species.egg_group2.clone(),
            tmhm_learnset: fixture_species.tmhm_learnset.clone(),
            ability: fixture_species.ability.clone(),
            weight: fixture_species.weight,
        };
        let wrong_species_growth = RuntimeSpeciesBattleDataKey {
            growth_rate: "GROWTH_MEDIUM_FAST".to_string(),
            ..species_battle_data.clone()
        };
        let wrong_species_catch_rate = RuntimeSpeciesBattleDataKey {
            catch_rate: 44,
            ..species_battle_data.clone()
        };
        assert!(
            shell
                .runtime()
                .has_species_battle_data(&species_battle_data)
        );
        assert!(
            !shell
                .runtime()
                .has_species_battle_data(&wrong_species_growth)
        );
        assert!(
            !shell
                .runtime()
                .has_species_battle_data(&wrong_species_catch_rate)
        );
        assert!(
            shell
                .runtime()
                .species_battle_data_keys()
                .contains(&species_battle_data)
        );
        assert!(
            shell
                .runtime()
                .require_species_battle_data(&species_battle_data)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_species_battle_data(&wrong_species_growth)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_species_battle_data(&wrong_species_catch_rate)
                .is_err()
        );
        assert!(shell.runtime().has_map("RuntimeMap"));
        assert!(!shell.runtime().has_map("runtimemap"));
        assert!(shell.runtime().map_ids().contains("RuntimeMap"));
        assert!(shell.runtime().require_map("RuntimeMap").is_ok());
        assert!(shell.runtime().require_map("runtimemap").is_err());
        let runtime_map_metadata = RuntimeMapMetadataKey {
            map_name: "RuntimeMap".to_string(),
            map_id: "RuntimeMap".to_string(),
            tileset_name: "johto".to_string(),
            border_block: 0,
            width: 2,
            height: 1,
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: Some("route".to_string()),
            location: Some("johto".to_string()),
            music: Some("MUSIC_ROUTE_29".to_string()),
            palette: None,
            fishing_group: None,
            map_constant: Some("RUNTIME_MAP".to_string()),
            map_group_constant: None,
            metadata_constant: Some("RUNTIME_MAP".to_string()),
            metadata_group_name: Some("RUNTIME".to_string()),
            metadata_group_id: Some(1),
            metadata_map_id: Some(1),
            metadata_environment: Some("ROUTE".to_string()),
        };
        let wrong_runtime_map_tileset = RuntimeMapMetadataKey {
            tileset_name: "JOHTO".to_string(),
            ..runtime_map_metadata.clone()
        };
        let wrong_runtime_map_environment = RuntimeMapMetadataKey {
            metadata_environment: Some("route".to_string()),
            ..runtime_map_metadata.clone()
        };
        assert!(shell.runtime().has_map_metadata(&runtime_map_metadata));
        assert!(!shell.runtime().has_map_metadata(&wrong_runtime_map_tileset));
        assert!(
            !shell
                .runtime()
                .has_map_metadata(&wrong_runtime_map_environment)
        );
        assert!(
            shell
                .runtime()
                .map_metadata_keys()
                .contains(&runtime_map_metadata)
        );
        assert!(
            shell
                .runtime()
                .require_map_metadata(&runtime_map_metadata)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_map_metadata(&wrong_runtime_map_tileset)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_map_metadata(&wrong_runtime_map_environment)
                .is_err()
        );
        assert!(shell.runtime().has_trainer("RIVAL1"));
        assert!(!shell.runtime().has_trainer("rival1"));
        assert!(shell.runtime().trainer_ids().contains("RIVAL1"));
        assert!(shell.runtime().require_trainer("RIVAL1").is_ok());
        assert!(shell.runtime().require_trainer("rival1").is_err());
        let trainer_battle_data = RuntimeTrainerBattleDataKey {
            trainer_id: "RIVAL1".to_string(),
            name: "RIVAL@".to_string(),
            trainer_class: "RIVAL1".to_string(),
            win_quote: "RivalWinText".to_string(),
            lose_quote: "RivalLossText".to_string(),
            items: Vec::new(),
            base_reward: 100,
            ai_move_flags: 1,
            ai_item_switch_flags: 0,
            encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
            ai_layers: vec!["AI_BASIC".to_string()],
        };
        let wrong_trainer_ai = RuntimeTrainerBattleDataKey {
            ai_move_flags: 2,
            ..trainer_battle_data.clone()
        };
        let trainer_party_pokemon = RuntimeTrainerPartyPokemonKey {
            trainer_id: "RIVAL1".to_string(),
            party_index: 0,
            species: "CHIKORITA".to_string(),
            level: 5,
            item: None,
            move_names: Vec::new(),
            move_pp: Vec::new(),
            move_pp_ups: Vec::new(),
            dv_attack: 0,
            dv_defense: 0,
            dv_speed: 0,
            dv_special: 0,
            dv_hp: 0,
        };
        let wrong_trainer_party_level = RuntimeTrainerPartyPokemonKey {
            level: 6,
            ..trainer_party_pokemon.clone()
        };
        assert!(
            shell
                .runtime()
                .has_trainer_battle_data(&trainer_battle_data)
        );
        assert!(!shell.runtime().has_trainer_battle_data(&wrong_trainer_ai));
        assert!(
            shell
                .runtime()
                .trainer_battle_data_keys()
                .contains(&trainer_battle_data)
        );
        assert!(
            shell
                .runtime()
                .require_trainer_battle_data(&trainer_battle_data)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_trainer_battle_data(&wrong_trainer_ai)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .has_trainer_party_pokemon(&trainer_party_pokemon)
        );
        assert!(
            !shell
                .runtime()
                .has_trainer_party_pokemon(&wrong_trainer_party_level)
        );
        assert!(
            shell
                .runtime()
                .trainer_party_pokemon_keys()
                .contains(&trainer_party_pokemon)
        );
        assert!(
            shell
                .runtime()
                .require_trainer_party_pokemon(&trainer_party_pokemon)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_trainer_party_pokemon(&wrong_trainer_party_level)
                .is_err()
        );
        assert!(shell.runtime().has_text("RuntimeText"));
        assert!(!shell.runtime().has_text("runtimetext"));
        assert!(shell.runtime().text_ids().contains("RuntimeText"));
        assert!(shell.runtime().require_text("RuntimeText").is_ok());
        assert!(shell.runtime().require_text("runtimetext").is_err());
        let script_text_body = RuntimeScriptTextBodyKey {
            map_name: "RuntimeMap".to_string(),
            body_key: "RuntimeGreetingText".to_string(),
            label: "RuntimeGreetingText".to_string(),
            commands: Vec::new(),
        };
        let wrong_script_text_body_label = RuntimeScriptTextBodyKey {
            label: "runtimegreetingtext".to_string(),
            ..script_text_body.clone()
        };
        assert!(shell.runtime().has_script_text_body(&script_text_body));
        assert!(
            !shell
                .runtime()
                .has_script_text_body(&wrong_script_text_body_label)
        );
        assert!(
            shell
                .runtime()
                .script_text_body_keys()
                .contains(&script_text_body)
        );
        assert!(
            shell
                .runtime()
                .require_script_text_body(&script_text_body)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_text_body(&wrong_script_text_body_label)
                .is_err()
        );
        assert!(shell.runtime().has_menu("RuntimeMenu"));
        assert!(!shell.runtime().has_menu("runtimemenu"));
        assert!(shell.runtime().menu_ids().contains("RuntimeMenu"));
        assert!(shell.runtime().require_menu("RuntimeMenu").is_ok());
        assert!(shell.runtime().require_menu("runtimemenu").is_err());
        assert_eq!(
            parse_menu_coords(&[
                "0".to_string(),
                "TEXTBOX_Y".to_string(),
                "SCREEN_WIDTH - 1".to_string(),
                "SCREEN_HEIGHT - 1".to_string(),
            ])
            .expect("exact menu coordinate expression"),
            [0, 12, 19, 17]
        );
        assert_eq!(
            parse_menu_coords(&[
                "SCREEN_LEFT".to_string(),
                "TEXTBOX_Y - %1".to_string(),
                "SCREEN_WIDTH - $1".to_string(),
                "SCREEN_HEIGHT - +1".to_string(),
            ])
            .expect("exact ASM numeric menu coordinate expression"),
            [0, 11, 19, 17]
        );
        assert_eq!(
            parse_menu_coords(&[
                "$0".to_string(),
                "%10".to_string(),
                "$13".to_string(),
                "%10001".to_string(),
            ])
            .expect("exact standalone ASM numeric menu coordinates"),
            [0, 2, 19, 17]
        );
        assert!(
            parse_menu_coords(&[
                "0".to_string(),
                "TEXTBOX_Y".to_string(),
                "SCREEN_WIDTH  - 1".to_string(),
                "SCREEN_HEIGHT".to_string(),
            ])
            .expect_err("repeated spaces must not be normalized")
            .to_string()
            .contains("menu coordinate right must be an exact i16")
        );
        assert!(
            parse_menu_coords(&[
                "0".to_string(),
                " TEXTBOX_Y".to_string(),
                "SCREEN_WIDTH - 1".to_string(),
                "SCREEN_HEIGHT - 1".to_string(),
            ])
            .expect_err("leading spaces must not be normalized")
            .to_string()
            .contains("menu coordinate top must be an exact i16")
        );
        let script_menu_definition = RuntimeScriptMenuDefinitionKey {
            map_name: "RuntimeMap".to_string(),
            menu_key: "RuntimeMenu".to_string(),
            label: "RuntimeMenu".to_string(),
            commands: vec![
                RuntimeScriptMenuCommandKey {
                    command: "menu_coords".to_string(),
                    args: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "10".to_string(),
                        "8".to_string(),
                    ],
                    command_index: 0,
                },
                RuntimeScriptMenuCommandKey {
                    command: "db".to_string(),
                    args: vec!["2".to_string(), "1".to_string(), "0".to_string()],
                    command_index: 1,
                },
                RuntimeScriptMenuCommandKey {
                    command: "dw".to_string(),
                    args: vec!["RuntimeMenuItems".to_string()],
                    command_index: 2,
                },
            ],
        };
        let wrong_script_menu_command = RuntimeScriptMenuDefinitionKey {
            commands: vec![
                RuntimeScriptMenuCommandKey {
                    command: "menu_coords".to_string(),
                    args: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "10".to_string(),
                        "8".to_string(),
                    ],
                    command_index: 0,
                },
                RuntimeScriptMenuCommandKey {
                    command: "DB".to_string(),
                    args: vec!["2".to_string(), "1".to_string(), "0".to_string()],
                    command_index: 1,
                },
                RuntimeScriptMenuCommandKey {
                    command: "dw".to_string(),
                    args: vec!["RuntimeMenuItems".to_string()],
                    command_index: 2,
                },
            ],
            ..script_menu_definition.clone()
        };
        assert!(
            shell
                .runtime()
                .has_script_menu_definition(&script_menu_definition)
        );
        assert!(
            !shell
                .runtime()
                .has_script_menu_definition(&wrong_script_menu_command)
        );
        assert!(
            shell
                .runtime()
                .script_menu_definition_keys()
                .contains(&script_menu_definition)
        );
        assert!(
            shell
                .runtime()
                .require_script_menu_definition(&script_menu_definition)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_menu_definition(&wrong_script_menu_command)
                .is_err()
        );
        assert!(shell.runtime().has_phone_contact("PHONE_RUNTIME"));
        assert!(!shell.runtime().has_phone_contact("phone_runtime"));
        assert!(
            shell
                .runtime()
                .phone_contact_ids()
                .contains("PHONE_RUNTIME")
        );
        assert!(
            shell
                .runtime()
                .require_phone_contact("PHONE_RUNTIME")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_phone_contact("phone_runtime")
                .is_err()
        );
        assert!(shell.runtime().has_special_phone_call("RuntimePhoneScript"));
        assert!(!shell.runtime().has_special_phone_call("runtimephonescript"));
        assert!(
            shell
                .runtime()
                .special_phone_call_ids()
                .contains("RuntimePhoneScript")
        );
        assert!(
            shell
                .runtime()
                .require_special_phone_call("RuntimePhoneScript")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_special_phone_call("runtimephonescript")
                .is_err()
        );
        assert!(!shell.runtime().has_npc_trade("NPC_TRADE_RUNTIME"));
        assert!(shell.runtime().npc_trade_ids().is_empty());
        assert!(
            shell
                .runtime()
                .require_npc_trade("NPC_TRADE_RUNTIME")
                .is_err()
        );
        assert!(shell.runtime().has_sprite("SPRITE_MON"));
        assert!(!shell.runtime().has_sprite("sprite_mon"));
        assert!(shell.runtime().sprite_ids().contains("SPRITE_MON"));
        assert!(shell.runtime().require_sprite("SPRITE_MON").is_ok());
        assert!(shell.runtime().require_sprite("sprite_mon").is_err());
        assert!(shell.runtime().has_map_constant("RUNTIME_MAP"));
        assert!(!shell.runtime().has_map_constant("runtime_map"));
        assert!(shell.runtime().map_constants().contains("RUNTIME_MAP"));
        assert!(shell.runtime().require_map_constant("RUNTIME_MAP").is_ok());
        assert!(shell.runtime().require_map_constant("runtime_map").is_err());
        assert!(shell.runtime().has_event_flag("EVENT_RUNTIME_CONTESTANT"));
        assert!(shell.runtime().has_event_flag("EVENT_RUNTIME"));
        assert!(!shell.runtime().has_event_flag("event_runtime"));
        assert!(
            shell
                .runtime()
                .event_flag_ids()
                .contains("EVENT_RUNTIME_CONTESTANT")
        );
        assert!(shell.runtime().event_flag_ids().contains("EVENT_RUNTIME"));
        assert!(
            shell
                .runtime()
                .require_event_flag("EVENT_RUNTIME_CONTESTANT")
                .is_ok()
        );
        assert!(shell.runtime().require_event_flag("event_runtime").is_err());
        assert!(shell.runtime().has_engine_flag("ENGINE_GOT_SHUCKIE_TODAY"));
        assert!(!shell.runtime().has_engine_flag("engine_got_shuckie_today"));
        assert!(
            shell
                .runtime()
                .engine_flag_ids()
                .contains("ENGINE_GOT_SHUCKIE_TODAY")
        );
        assert!(
            shell
                .runtime()
                .require_engine_flag("ENGINE_GOT_SHUCKIE_TODAY")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_engine_flag("engine_got_shuckie_today")
                .is_err()
        );
        assert!(shell.runtime().has_spawn_identifier(0));
        assert!(!shell.runtime().has_spawn_identifier(99));
        assert!(shell.runtime().spawn_identifiers().contains(&0));
        assert!(shell.runtime().require_spawn_identifier(0).is_ok());
        assert!(shell.runtime().require_spawn_identifier(99).is_err());
        assert!(shell.runtime().has_tileset("johto"));
        assert!(!shell.runtime().has_tileset("JOHTO"));
        assert!(shell.runtime().tileset_ids().contains("johto"));
        assert!(shell.runtime().require_tileset("johto").is_ok());
        assert!(shell.runtime().require_tileset("JOHTO").is_err());
        let tileset_row = RuntimeTilesetKey {
            tileset_id: "johto".to_string(),
            collision: [(
                "00".to_string(),
                vec![
                    "FLOOR".to_string(),
                    "FLOOR".to_string(),
                    "FLOOR".to_string(),
                    "FLOOR".to_string(),
                ],
            )]
            .into_iter()
            .collect(),
            palette_map: vec![0],
        };
        let wrong_tileset_collision = RuntimeTilesetKey {
            collision: [(
                "00".to_string(),
                vec![
                    "floor".to_string(),
                    "FLOOR".to_string(),
                    "FLOOR".to_string(),
                    "FLOOR".to_string(),
                ],
            )]
            .into_iter()
            .collect(),
            ..tileset_row.clone()
        };
        let wrong_tileset_palette = RuntimeTilesetKey {
            palette_map: vec![1],
            ..tileset_row.clone()
        };
        assert!(shell.runtime().has_tileset_row(&tileset_row));
        assert!(!shell.runtime().has_tileset_row(&wrong_tileset_collision));
        assert!(!shell.runtime().has_tileset_row(&wrong_tileset_palette));
        assert!(shell.runtime().tileset_keys().contains(&tileset_row));
        assert!(shell.runtime().require_tileset_row(&tileset_row).is_ok());
        assert!(
            shell
                .runtime()
                .require_tileset_row(&wrong_tileset_collision)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_tileset_row(&wrong_tileset_palette)
                .is_err()
        );
        assert!(shell.runtime().has_landmark("LANDMARK_RUNTIME"));
        assert!(!shell.runtime().has_landmark("landmark_runtime"));
        assert!(shell.runtime().landmark_ids().contains("LANDMARK_RUNTIME"));
        assert!(shell.runtime().require_landmark("LANDMARK_RUNTIME").is_ok());
        assert!(
            shell
                .runtime()
                .require_landmark("landmark_runtime")
                .is_err()
        );
        let pc_string_row = RuntimePcStringKey {
            string_id: "PC_RUNTIME".to_string(),
            text: "Runtime PC".to_string(),
        };
        let wrong_pc_string_text = RuntimePcStringKey {
            text: "runtime pc".to_string(),
            ..pc_string_row.clone()
        };
        let menu_icon_row = RuntimeMenuIconKey {
            species_id: "CHIKORITA".to_string(),
            icon_id: "ICON_CHIKORITA".to_string(),
        };
        let wrong_menu_icon = RuntimeMenuIconKey {
            icon_id: "icon_chikorita".to_string(),
            ..menu_icon_row.clone()
        };
        let pokedex_entry_row = RuntimePokedexEntryKey {
            species_id: "CHIKORITA".to_string(),
            species: "CHIKORITA".to_string(),
            classification: "Leaf".to_string(),
            height_digits: 9,
            weight_digits: 64,
            pages: vec!["A sweet leaf Pokemon.".to_string()],
        };
        let wrong_pokedex_page = RuntimePokedexEntryKey {
            pages: vec!["A sweet leaf pokemon.".to_string()],
            ..pokedex_entry_row.clone()
        };
        let pokegear_landmark_row = RuntimePokegearLandmarkKey {
            landmark_id: 1,
            constant: "LANDMARK_RUNTIME".to_string(),
            label: "RuntimeLandmark".to_string(),
            name: "Runtime".to_string(),
            x: 1,
            y: 1,
            region: "JOHTO".to_string(),
        };
        let wrong_pokegear_landmark_x = RuntimePokegearLandmarkKey {
            x: 2,
            ..pokegear_landmark_row.clone()
        };
        let pokegear_map_landmark_row = RuntimePokegearMapLandmarkKey {
            map_name: "RuntimeMap".to_string(),
            landmark_constant: "LANDMARK_RUNTIME".to_string(),
        };
        let wrong_pokegear_map_landmark = RuntimePokegearMapLandmarkKey {
            landmark_constant: "landmark_runtime".to_string(),
            ..pokegear_map_landmark_row.clone()
        };
        assert!(shell.runtime().has_pc_string(&pc_string_row));
        assert!(!shell.runtime().has_pc_string(&wrong_pc_string_text));
        assert!(shell.runtime().pc_string_keys().contains(&pc_string_row));
        assert!(shell.runtime().require_pc_string(&pc_string_row).is_ok());
        assert!(
            shell
                .runtime()
                .require_pc_string(&wrong_pc_string_text)
                .is_err()
        );
        assert!(shell.runtime().has_menu_icon(&menu_icon_row));
        assert!(!shell.runtime().has_menu_icon(&wrong_menu_icon));
        assert!(shell.runtime().menu_icon_keys().contains(&menu_icon_row));
        assert!(shell.runtime().require_menu_icon(&menu_icon_row).is_ok());
        assert!(shell.runtime().require_menu_icon(&wrong_menu_icon).is_err());
        assert!(shell.runtime().has_pokedex_entry(&pokedex_entry_row));
        assert!(!shell.runtime().has_pokedex_entry(&wrong_pokedex_page));
        assert!(
            shell
                .runtime()
                .pokedex_entry_keys()
                .contains(&pokedex_entry_row)
        );
        assert!(
            shell
                .runtime()
                .require_pokedex_entry(&pokedex_entry_row)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_pokedex_entry(&wrong_pokedex_page)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .has_pokegear_landmark(&pokegear_landmark_row)
        );
        assert!(
            !shell
                .runtime()
                .has_pokegear_landmark(&wrong_pokegear_landmark_x)
        );
        assert!(
            shell
                .runtime()
                .pokegear_landmark_keys()
                .contains(&pokegear_landmark_row)
        );
        assert!(
            shell
                .runtime()
                .require_pokegear_landmark(&pokegear_landmark_row)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_pokegear_landmark(&wrong_pokegear_landmark_x)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .has_pokegear_map_landmark(&pokegear_map_landmark_row)
        );
        assert!(
            !shell
                .runtime()
                .has_pokegear_map_landmark(&wrong_pokegear_map_landmark)
        );
        assert!(
            shell
                .runtime()
                .pokegear_map_landmark_keys()
                .contains(&pokegear_map_landmark_row)
        );
        assert!(
            shell
                .runtime()
                .require_pokegear_map_landmark(&pokegear_map_landmark_row)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_pokegear_map_landmark(&wrong_pokegear_map_landmark)
                .is_err()
        );
        assert!(shell.runtime().has_fishing_rod(ROD_OLD));
        assert!(!shell.runtime().has_fishing_rod(ROD_GOOD));
        assert!(shell.runtime().fishing_rod_ids().contains(ROD_OLD));
        assert!(shell.runtime().require_fishing_rod(ROD_OLD).is_ok());
        assert!(shell.runtime().require_fishing_rod(ROD_GOOD).is_err());
        assert!(shell.runtime().has_map_group("RUNTIME"));
        assert!(!shell.runtime().has_map_group("group_runtime"));
        assert!(shell.runtime().map_group_ids().contains("RUNTIME"));
        assert!(shell.runtime().require_map_group("RUNTIME").is_ok());
        assert!(shell.runtime().require_map_group("group_runtime").is_err());
        assert!(shell.runtime().has_encounter_group("FISHGROUP_RUNTIME"));
        assert!(!shell.runtime().has_encounter_group("fishgroup_runtime"));
        assert!(
            shell
                .runtime()
                .encounter_group_ids()
                .contains("FISHGROUP_RUNTIME")
        );
        assert!(
            shell
                .runtime()
                .require_encounter_group("FISHGROUP_RUNTIME")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_encounter_group("fishgroup_runtime")
                .is_err()
        );
        assert!(shell.runtime().has_mart("MART_RUNTIME"));
        assert!(!shell.runtime().has_mart("mart_runtime"));
        assert!(shell.runtime().mart_ids().contains("MART_RUNTIME"));
        assert!(shell.runtime().require_mart("MART_RUNTIME").is_ok());
        assert!(shell.runtime().require_mart("mart_runtime").is_err());
        let mart_row = RuntimeMartKey {
            mart_id: "MART_RUNTIME".to_string(),
            item_ids: vec!["POKE_BALL".to_string()],
        };
        let wrong_mart_item = RuntimeMartKey {
            item_ids: vec!["poke_ball".to_string()],
            ..mart_row.clone()
        };
        assert!(shell.runtime().has_mart_row(&mart_row));
        assert!(!shell.runtime().has_mart_row(&wrong_mart_item));
        assert!(shell.runtime().mart_keys().contains(&mart_row));
        assert!(shell.runtime().require_mart_row(&mart_row).is_ok());
        assert!(shell.runtime().require_mart_row(&wrong_mart_item).is_err());
        assert!(shell.runtime().has_fruit_tree("FRUITTREE_RUNTIME"));
        assert!(!shell.runtime().has_fruit_tree("fruittree_runtime"));
        assert!(
            shell
                .runtime()
                .fruit_tree_ids()
                .contains("FRUITTREE_RUNTIME")
        );
        assert!(
            shell
                .runtime()
                .require_fruit_tree("FRUITTREE_RUNTIME")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_fruit_tree("fruittree_runtime")
                .is_err()
        );
        let fruit_tree_row = RuntimeFruitTreeKey {
            fruit_tree_id: "FRUITTREE_RUNTIME".to_string(),
            item_id: "BLU_APRICORN".to_string(),
        };
        let wrong_fruit_tree_item = RuntimeFruitTreeKey {
            item_id: "blu_apricorn".to_string(),
            ..fruit_tree_row.clone()
        };
        assert!(shell.runtime().has_fruit_tree_row(&fruit_tree_row));
        assert!(!shell.runtime().has_fruit_tree_row(&wrong_fruit_tree_item));
        assert!(shell.runtime().fruit_tree_keys().contains(&fruit_tree_row));
        assert!(
            shell
                .runtime()
                .require_fruit_tree_row(&fruit_tree_row)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_fruit_tree_row(&wrong_fruit_tree_item)
                .is_err()
        );
        assert!(shell.runtime().has_field_move_rule("cut"));
        assert!(!shell.runtime().has_field_move_rule("CUT"));
        assert!(shell.runtime().field_move_rule_ids().contains("cut"));
        assert!(shell.runtime().require_field_move_rule("cut").is_ok());
        assert!(shell.runtime().require_field_move_rule("CUT").is_err());
        let cut_rule_row = RuntimeFieldMoveRuleKey {
            rule_id: "cut".to_string(),
            rule_kind: "block".to_string(),
            move_id: Some("CUT".to_string()),
            item_id: None,
            badge_region: Some("johto".to_string()),
            badge_index: Some(1),
            engine_flag: None,
            escape_rope_mode: None,
            target_collisions: vec![0x12, 0x1a, 0x18, 0x14, 0x1c],
            blocked_collisions: Vec::new(),
            replacements: [(
                "johto".to_string(),
                [(
                    0x5b,
                    RuntimeFieldMoveReplacementKey {
                        replacement_block_id: 0x3c,
                        variant: "tree".to_string(),
                    },
                )]
                .into_iter()
                .collect(),
            )]
            .into_iter()
            .collect(),
        };
        let wrong_cut_badge = RuntimeFieldMoveRuleKey {
            badge_index: Some(2),
            ..cut_rule_row.clone()
        };
        let wrong_cut_variant = RuntimeFieldMoveRuleKey {
            replacements: [(
                "johto".to_string(),
                [(
                    0x5b,
                    RuntimeFieldMoveReplacementKey {
                        replacement_block_id: 0x3c,
                        variant: "TREE".to_string(),
                    },
                )]
                .into_iter()
                .collect(),
            )]
            .into_iter()
            .collect(),
            ..cut_rule_row.clone()
        };
        assert!(shell.runtime().has_field_move_rule_row(&cut_rule_row));
        assert!(!shell.runtime().has_field_move_rule_row(&wrong_cut_badge));
        assert!(!shell.runtime().has_field_move_rule_row(&wrong_cut_variant));
        assert!(
            shell
                .runtime()
                .field_move_rule_keys()
                .contains(&cut_rule_row)
        );
        assert!(
            shell
                .runtime()
                .require_field_move_rule_row(&cut_rule_row)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_field_move_rule_row(&wrong_cut_badge)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_field_move_rule_row(&wrong_cut_variant)
                .is_err()
        );
        assert!(shell.runtime().has_field_move_move("CUT"));
        assert!(!shell.runtime().has_field_move_move("cut"));
        assert!(shell.runtime().field_move_move_ids().contains("CUT"));
        assert!(shell.runtime().require_field_move_move("CUT").is_ok());
        assert!(shell.runtime().require_field_move_move("cut").is_err());
        assert!(shell.runtime().has_field_move_item("ESCAPE_ROPE"));
        assert!(!shell.runtime().has_field_move_item("escape_rope"));
        assert!(
            shell
                .runtime()
                .field_move_item_ids()
                .contains("ESCAPE_ROPE")
        );
        assert!(
            shell
                .runtime()
                .require_field_move_item("ESCAPE_ROPE")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_field_move_item("escape_rope")
                .is_err()
        );
        let fly_destination_row = RuntimeFlyDestinationKey {
            flypoint_flag: "ENGINE_FLYPOINT_FLY_MAP".to_string(),
            destination_spawn_identifier: 14,
            label: "LANDMARK_FLY_MAP".to_string(),
        };
        let wrong_fly_destination_row = RuntimeFlyDestinationKey {
            destination_spawn_identifier: 15,
            ..fly_destination_row.clone()
        };
        assert!(
            shell
                .runtime()
                .has_fly_destination("ENGINE_FLYPOINT_FLY_MAP")
        );
        assert!(
            !shell
                .runtime()
                .has_fly_destination("engine_flypoint_fly_map")
        );
        assert!(
            shell
                .runtime()
                .fly_destination_ids()
                .contains("ENGINE_FLYPOINT_FLY_MAP")
        );
        assert!(
            shell
                .runtime()
                .require_fly_destination("ENGINE_FLYPOINT_FLY_MAP")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_fly_destination("engine_flypoint_fly_map")
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .has_fly_destination_row(&fly_destination_row)
        );
        assert!(
            !shell
                .runtime()
                .has_fly_destination_row(&wrong_fly_destination_row)
        );
        assert!(
            shell
                .runtime()
                .fly_destination_keys()
                .contains(&fly_destination_row)
        );
        assert!(
            shell
                .runtime()
                .require_fly_destination_row(&fly_destination_row)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_fly_destination_row(&wrong_fly_destination_row)
                .is_err()
        );
        assert!(shell.runtime().has_flee_mon_bucket("always"));
        assert!(!shell.runtime().has_flee_mon_bucket("ALWAYS"));
        assert!(shell.runtime().flee_mon_bucket_ids().contains("always"));
        assert!(shell.runtime().require_flee_mon_bucket("always").is_ok());
        assert!(shell.runtime().require_flee_mon_bucket("ALWAYS").is_err());
        assert!(shell.runtime().has_buena_password_category("BUENA_RUNTIME"));
        assert!(!shell.runtime().has_buena_password_category("buena_runtime"));
        assert!(
            shell
                .runtime()
                .buena_password_category_ids()
                .contains("BUENA_RUNTIME")
        );
        assert!(
            shell
                .runtime()
                .require_buena_password_category("BUENA_RUNTIME")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_buena_password_category("buena_runtime")
                .is_err()
        );
        assert!(shell.runtime().has_roaming_species("CHIKORITA"));
        assert!(!shell.runtime().has_roaming_species("chikorita"));
        assert!(shell.runtime().roaming_species_ids().contains("CHIKORITA"));
        assert!(shell.runtime().require_roaming_species("CHIKORITA").is_ok());
        assert!(
            shell
                .runtime()
                .require_roaming_species("chikorita")
                .is_err()
        );
        assert!(shell.runtime().has_buena_prize_item("POKE_BALL"));
        assert!(!shell.runtime().has_buena_prize_item("poke_ball"));
        assert!(shell.runtime().buena_prize_item_ids().contains("POKE_BALL"));
        assert!(
            shell
                .runtime()
                .require_buena_prize_item("POKE_BALL")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_buena_prize_item("poke_ball")
                .is_err()
        );
        assert!(shell.runtime().has_kurt_apricorn_item("BLU_APRICORN"));
        assert!(!shell.runtime().has_kurt_apricorn_item("blu_apricorn"));
        assert!(
            shell
                .runtime()
                .kurt_apricorn_item_ids()
                .contains("BLU_APRICORN")
        );
        assert!(
            shell
                .runtime()
                .require_kurt_apricorn_item("BLU_APRICORN")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_kurt_apricorn_item("blu_apricorn")
                .is_err()
        );
        assert!(shell.runtime().has_dratini_move_set(0));
        assert!(!shell.runtime().has_dratini_move_set(1));
        assert!(shell.runtime().dratini_move_set_ids().contains(&0));
        assert!(shell.runtime().require_dratini_move_set(0).is_ok());
        assert!(shell.runtime().require_dratini_move_set(1).is_err());
        assert!(shell.runtime().has_special_feature("bug_contest"));
        assert!(!shell.runtime().has_special_feature("BugContest"));
        assert!(
            shell
                .runtime()
                .special_feature_ids()
                .contains("bug_contest")
        );
        assert!(
            shell
                .runtime()
                .require_special_feature("bug_contest")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_special_feature("BugContest")
                .is_err()
        );
        assert!(shell.runtime().has_oak_rating_text("OakRating01"));
        assert!(!shell.runtime().has_oak_rating_text("oakrating01"));
        assert!(
            shell
                .runtime()
                .oak_rating_text_ids()
                .contains("OakRating01")
        );
        assert!(
            shell
                .runtime()
                .require_oak_rating_text("OakRating01")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_oak_rating_text("oakrating01")
                .is_err()
        );
        assert!(shell.runtime().has_odd_egg_species("CHIKORITA"));
        assert!(!shell.runtime().has_odd_egg_species("chikorita"));
        assert!(shell.runtime().odd_egg_species_ids().contains("CHIKORITA"));
        assert!(shell.runtime().require_odd_egg_species("CHIKORITA").is_ok());
        assert!(
            shell
                .runtime()
                .require_odd_egg_species("chikorita")
                .is_err()
        );
        assert!(shell.runtime().has_magikarp_length_threshold(110));
        assert!(!shell.runtime().has_magikarp_length_threshold(2));
        assert!(shell.runtime().magikarp_length_thresholds().contains(&110));
        assert!(shell.runtime().require_magikarp_length_threshold(110).is_ok());
        assert!(
            shell
                .runtime()
                .require_magikarp_length_threshold(2)
                .is_err()
        );
        assert!(shell.runtime().has_happiness_change(9));
        assert!(!shell.runtime().has_happiness_change(1));
        assert!(shell.runtime().happiness_change_ids().contains(&9));
        assert!(shell.runtime().require_happiness_change(9).is_ok());
        assert!(shell.runtime().require_happiness_change(1).is_err());
        assert!(
            shell
                .runtime()
                .has_happiness_service("RuntimeBootstrapHappiness")
        );
        assert!(!shell.runtime().has_happiness_service("haircut"));
        assert!(
            shell
                .runtime()
                .happiness_service_ids()
                .contains("RuntimeBootstrapHappiness")
        );
        assert!(
            shell
                .runtime()
                .require_happiness_service("RuntimeBootstrapHappiness")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_happiness_service("haircut")
                .is_err()
        );
        assert!(shell.runtime().has_pokemon_status("POISON"));
        assert!(shell.runtime().has_pokemon_status("SLEEP"));
        assert!(shell.runtime().has_pokemon_status("POKERUS"));
        assert!(!shell.runtime().has_pokemon_status("poison"));
        assert!(shell.runtime().pokemon_status_ids().contains("POISON"));
        assert!(shell.runtime().pokemon_status_ids().contains("SLEEP"));
        assert!(shell.runtime().pokemon_status_ids().contains("POKERUS"));
        assert!(shell.runtime().require_pokemon_status("POISON").is_ok());
        assert!(shell.runtime().require_pokemon_status("poison").is_err());
        assert!(!shell.runtime().has_fishing_daily_flag_bit(2));
        assert!(shell.runtime().fishing_daily_flag_bits().is_empty());
        assert!(shell.runtime().require_fishing_daily_flag_bit(2).is_err());
        assert!(!shell.runtime().has_fishing_swarm_flag(1));
        assert!(shell.runtime().fishing_swarm_flags().is_empty());
        assert!(shell.runtime().require_fishing_swarm_flag(1).is_err());
        assert!(
            !shell
                .runtime()
                .has_pending_special_battle_type("BATTLETYPE_NORMAL")
        );
        assert!(shell.runtime().pending_special_battle_type_ids().is_empty());
        assert!(
            shell
                .runtime()
                .require_pending_special_battle_type("BATTLETYPE_NORMAL")
                .is_err()
        );
        let missing_wild_encounter = RuntimeWildEncounterOriginKey {
            map_name: "RuntimeMap".to_string(),
            species: "CHIKORITA".to_string(),
            level: 2,
        };
        let runtime_wild_encounter = RuntimeWildEncounterOriginKey {
            map_name: "RuntimeMap".to_string(),
            species: "CHIKORITA".to_string(),
            level: 14,
        };
        assert!(
            shell
                .runtime()
                .wild_encounter_origin_keys()
                .contains(&runtime_wild_encounter)
        );
        assert!(
            shell
                .runtime()
                .has_wild_encounter_origin(&runtime_wild_encounter)
        );
        assert!(
            shell
                .runtime()
                .require_wild_encounter_origin(&runtime_wild_encounter)
                .is_ok()
        );
        assert!(
            !shell
                .runtime()
                .has_wild_encounter_origin(&missing_wild_encounter)
        );
        assert!(
            shell
                .runtime()
                .require_wild_encounter_origin(&missing_wild_encounter)
                .is_err()
        );
        assert!(shell.runtime().has_script_label("RuntimeScript"));
        assert!(!shell.runtime().has_script_label("runtimescript"));
        assert!(shell.runtime().script_label_ids().contains("RuntimeScript"));
        assert!(
            shell
                .runtime()
                .require_script_label("RuntimeScript")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_label("runtimescript")
                .is_err()
        );
        let runtime_script_command = RuntimeScriptCommandKey {
            script_label: "RuntimeScript".to_string(),
            command_index: 4,
        };
        let missing_script_command = RuntimeScriptCommandKey {
            script_label: "RuntimeScript".to_string(),
            command_index: 99,
        };
        assert!(shell.runtime().has_script_command(&runtime_script_command));
        assert!(!shell.runtime().has_script_command(&missing_script_command));
        assert!(
            shell
                .runtime()
                .script_command_keys()
                .contains(&runtime_script_command)
        );
        assert!(
            shell
                .runtime()
                .require_script_command(&runtime_script_command)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_command(&missing_script_command)
                .is_err()
        );
        let runtime_script_payload = RuntimeScriptCommandPayloadKey {
            script_label: "RuntimeScript".to_string(),
            command_index: 4,
            command: "verticalmenu".to_string(),
            args: Vec::new(),
        };
        let wrong_script_payload = RuntimeScriptCommandPayloadKey {
            script_label: "RuntimeScript".to_string(),
            command_index: 4,
            command: "VerticalMenu".to_string(),
            args: Vec::new(),
        };
        assert!(
            shell
                .runtime()
                .has_script_command_payload(&runtime_script_payload)
        );
        assert!(
            !shell
                .runtime()
                .has_script_command_payload(&wrong_script_payload)
        );
        assert!(
            shell
                .runtime()
                .script_command_payload_keys()
                .contains(&runtime_script_payload)
        );
        assert!(
            shell
                .runtime()
                .require_script_command_payload(&runtime_script_payload)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_command_payload(&wrong_script_payload)
                .is_err()
        );
        let runtime_script_return = RuntimeScriptReturnKey {
            script_label: "RuntimeScript".to_string(),
            next_command_index: 6,
        };
        let missing_script_return = RuntimeScriptReturnKey {
            script_label: "RuntimeScript".to_string(),
            next_command_index: 99,
        };
        assert!(shell.runtime().has_script_return(&runtime_script_return));
        assert!(!shell.runtime().has_script_return(&missing_script_return));
        assert!(
            shell
                .runtime()
                .script_return_keys()
                .contains(&runtime_script_return)
        );
        assert!(
            shell
                .runtime()
                .require_script_return(&runtime_script_return)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_return(&missing_script_return)
                .is_err()
        );
        let runtime_vertical_menu = RuntimeScriptVerticalMenuKey {
            map_name: "RuntimeMap".to_string(),
            menu_key: "RuntimeScript:4".to_string(),
            source_script: "RuntimeScript".to_string(),
            loadmenu_command_index: 3,
            verticalmenu_command_index: 4,
            header_label: "RuntimeMenu".to_string(),
            data_label: Some("RuntimeMenuItems".to_string()),
            options: vec!["First".to_string(), "Second".to_string()],
        };
        let wrong_vertical_menu_option = RuntimeScriptVerticalMenuKey {
            options: vec!["First".to_string(), "second".to_string()],
            ..runtime_vertical_menu.clone()
        };
        assert!(
            shell
                .runtime()
                .has_script_vertical_menu(&runtime_vertical_menu)
        );
        assert!(
            !shell
                .runtime()
                .has_script_vertical_menu(&wrong_vertical_menu_option)
        );
        assert!(
            shell
                .runtime()
                .script_vertical_menu_keys()
                .contains(&runtime_vertical_menu)
        );
        assert!(
            shell
                .runtime()
                .require_script_vertical_menu(&runtime_vertical_menu)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_vertical_menu(&wrong_vertical_menu_option)
                .is_err()
        );
        let runtime_elevator = RuntimeScriptElevatorKey {
            map_name: "RuntimeMap".to_string(),
            elevator_key: "RuntimeScript:5".to_string(),
            source_script: "RuntimeScript".to_string(),
            elevator_command_index: 5,
            data_label: "RuntimeElevatorData".to_string(),
            floors: vec![RuntimeScriptElevatorFloorKey {
                floor: "FLOOR_2F".to_string(),
                warp: 4,
                target_map: "RuntimeMap".to_string(),
                source_script: "RuntimeElevatorData".to_string(),
                command_index: 0,
            }],
        };
        let wrong_elevator_floor = RuntimeScriptElevatorKey {
            floors: vec![RuntimeScriptElevatorFloorKey {
                floor: "FLOOR_2F".to_string(),
                warp: 4,
                target_map: "runtimemap".to_string(),
                source_script: "RuntimeElevatorData".to_string(),
                command_index: 0,
            }],
            ..runtime_elevator.clone()
        };
        assert!(shell.runtime().has_script_elevator(&runtime_elevator));
        assert!(!shell.runtime().has_script_elevator(&wrong_elevator_floor));
        assert!(
            shell
                .runtime()
                .script_elevator_keys()
                .contains(&runtime_elevator)
        );
        assert!(
            shell
                .runtime()
                .require_script_elevator(&runtime_elevator)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_script_elevator(&wrong_elevator_floor)
                .is_err()
        );
        let runtime_gift = RuntimeGiftPokemonKey {
            map_name: "RuntimeMap".to_string(),
            species_id: "CHIKORITA".to_string(),
            level_token: "7".to_string(),
            level: 7,
            held_item_id: Some("BERRY".to_string()),
            nickname_label: Some("RuntimeGiftName".to_string()),
            ot_label: None,
            source_script: "RuntimeGiftScript".to_string(),
            command_index: 12,
            egg: false,
        };
        let wrong_runtime_gift_level = RuntimeGiftPokemonKey {
            level: 8,
            ..runtime_gift.clone()
        };
        assert!(shell.runtime().has_gift_pokemon(&runtime_gift));
        assert!(!shell.runtime().has_gift_pokemon(&wrong_runtime_gift_level));
        assert!(shell.runtime().gift_pokemon_keys().contains(&runtime_gift));
        assert!(shell.runtime().require_gift_pokemon(&runtime_gift).is_ok());
        assert!(
            shell
                .runtime()
                .require_gift_pokemon(&wrong_runtime_gift_level)
                .is_err()
        );
        let map_script_section_command = RuntimeMapScriptSectionCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "callback".to_string(),
            args: vec![
                "MAPCALLBACK_NEWMAP".to_string(),
                "RuntimeScript".to_string(),
            ],
            command_index: 0,
        };
        let wrong_map_script_section_command = RuntimeMapScriptSectionCommandKey {
            command: "Callback".to_string(),
            ..map_script_section_command.clone()
        };
        assert!(
            shell
                .runtime()
                .has_map_script_section_command(&map_script_section_command)
        );
        assert!(
            !shell
                .runtime()
                .has_map_script_section_command(&wrong_map_script_section_command)
        );
        assert!(
            shell
                .runtime()
                .map_script_section_command_keys()
                .contains(&map_script_section_command)
        );
        assert!(
            shell
                .runtime()
                .require_map_script_section_command(&map_script_section_command)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_map_script_section_command(&wrong_map_script_section_command)
                .is_err()
        );
        let map_event_section_command = RuntimeMapEventSectionCommandKey {
            map_name: "RuntimeMap".to_string(),
            command: "warp_event".to_string(),
            args: vec![
                "1".to_string(),
                "0".to_string(),
                "RUNTIME_MAP".to_string(),
                "4".to_string(),
            ],
            command_index: 0,
        };
        let wrong_map_event_section_command = RuntimeMapEventSectionCommandKey {
            args: vec![
                "1".to_string(),
                "0".to_string(),
                "RuntimeMap".to_string(),
                "4".to_string(),
            ],
            ..map_event_section_command.clone()
        };
        assert!(
            shell
                .runtime()
                .has_map_event_section_command(&map_event_section_command)
        );
        assert!(
            !shell
                .runtime()
                .has_map_event_section_command(&wrong_map_event_section_command)
        );
        assert!(
            shell
                .runtime()
                .map_event_section_command_keys()
                .contains(&map_event_section_command)
        );
        assert!(
            shell
                .runtime()
                .require_map_event_section_command(&map_event_section_command)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_map_event_section_command(&wrong_map_event_section_command)
                .is_err()
        );
        let runtime_warp = RuntimeWarpKey {
            map_name: "RuntimeMap".to_string(),
            warp_index: 4,
        };
        let missing_warp = RuntimeWarpKey {
            map_name: "RuntimeMap".to_string(),
            warp_index: 99,
        };
        assert!(shell.runtime().has_warp(&runtime_warp));
        assert!(!shell.runtime().has_warp(&missing_warp));
        assert!(shell.runtime().warp_keys().contains(&runtime_warp));
        assert!(shell.runtime().require_warp(&runtime_warp).is_ok());
        assert!(shell.runtime().require_warp(&missing_warp).is_err());
        let runtime_map_object = RuntimeMapObjectKey {
            map_name: "RuntimeMap".to_string(),
            object_id: "RuntimeNpc".to_string(),
        };
        let missing_map_object = RuntimeMapObjectKey {
            map_name: "RuntimeMap".to_string(),
            object_id: "RUNTIME_NPC".to_string(),
        };
        assert!(shell.runtime().has_map_object(&runtime_map_object));
        assert!(
            shell
                .runtime()
                .map_object_keys()
                .contains(&runtime_map_object)
        );
        assert!(
            shell
                .runtime()
                .require_map_object(&runtime_map_object)
                .is_ok()
        );
        assert!(!shell.runtime().has_map_object(&missing_map_object));
        assert!(
            shell
                .runtime()
                .require_map_object(&missing_map_object)
                .is_err()
        );
        let missing_map_scene = RuntimeMapSceneKey {
            map_name: "RuntimeMap".to_string(),
            scene_id: "SCENE_RUNTIME_ACTIVE".to_string(),
        };
        assert!(shell.runtime().map_scene_keys().is_empty());
        assert!(!shell.runtime().has_map_scene(&missing_map_scene));
        assert!(
            shell
                .runtime()
                .require_map_scene(&missing_map_scene)
                .is_err()
        );
        assert!(shell.runtime().has_currency_constant("MAX_MONEY"));
        assert!(!shell.runtime().has_currency_constant("max_money"));
        assert!(
            shell
                .runtime()
                .currency_constant_ids()
                .contains("MAX_MONEY")
        );
        assert!(
            shell
                .runtime()
                .require_currency_constant("MAX_MONEY")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_currency_constant("max_money")
                .is_err()
        );
        assert!(shell.runtime().has_capture_ball_rule("POKE_BALL"));
        assert!(!shell.runtime().has_capture_ball_rule("poke_ball"));
        assert!(
            shell
                .runtime()
                .capture_ball_rule_ids()
                .contains("POKE_BALL")
        );
        assert!(
            shell
                .runtime()
                .require_capture_ball_rule("POKE_BALL")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_capture_ball_rule("poke_ball")
                .is_err()
        );
        let capture_ball_rule = RuntimeCaptureBallRuleKey {
            ball_id: "POKE_BALL".to_string(),
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
        let wrong_capture_ball_rule = RuntimeCaptureBallRuleKey {
            multiplier_numerator: 2,
            ..capture_ball_rule.clone()
        };
        assert!(
            shell
                .runtime()
                .has_capture_ball_rule_key(&capture_ball_rule)
        );
        assert!(
            !shell
                .runtime()
                .has_capture_ball_rule_key(&wrong_capture_ball_rule)
        );
        assert!(
            shell
                .runtime()
                .capture_ball_rule_keys()
                .contains(&capture_ball_rule)
        );
        assert!(
            shell
                .runtime()
                .require_capture_ball_rule_key(&capture_ball_rule)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_capture_ball_rule_key(&wrong_capture_ball_rule)
                .is_err()
        );
        let item_battle_use = RuntimeItemBattleUseKey {
            item_id: "POKE_BALL".to_string(),
            effect: "POKE_BALL".to_string(),
            battle_menu: "ITEMMENU_CLOSE".to_string(),
            battle_usable: true,
            battle_stat_boost_stat: None,
            battle_stat_boost_stages: None,
            battle_escape_mode: None,
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
        };
        let wrong_item_battle_use = RuntimeItemBattleUseKey {
            battle_usable: false,
            ..item_battle_use.clone()
        };
        let wrong_item_battle_effect = RuntimeItemBattleUseKey {
            effect: "NONE".to_string(),
            ..item_battle_use.clone()
        };
        assert!(shell.runtime().has_item_battle_use(&item_battle_use));
        assert!(!shell.runtime().has_item_battle_use(&wrong_item_battle_use));
        assert!(
            !shell
                .runtime()
                .has_item_battle_use(&wrong_item_battle_effect)
        );
        assert!(
            shell
                .runtime()
                .item_battle_use_keys()
                .contains(&item_battle_use)
        );
        assert!(
            shell
                .runtime()
                .require_item_battle_use(&item_battle_use)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_item_battle_use(&wrong_item_battle_use)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_item_battle_use(&wrong_item_battle_effect)
                .is_err()
        );
        let item_field_use = RuntimeItemFieldUseKey {
            item_id: "POKE_BALL".to_string(),
            effect: "POKE_BALL".to_string(),
            field_menu: String::new(),
            field_usable: true,
            consumable: true,
            repel_steps: None,
            escape_rope_mode: None,
            tmhm_index: None,
            tmhm_move: None,
        };
        let wrong_item_field_use = RuntimeItemFieldUseKey {
            consumable: false,
            ..item_field_use.clone()
        };
        let wrong_item_field_effect = RuntimeItemFieldUseKey {
            effect: "NONE".to_string(),
            ..item_field_use.clone()
        };
        assert!(shell.runtime().has_item_field_use(&item_field_use));
        assert!(!shell.runtime().has_item_field_use(&wrong_item_field_use));
        assert!(!shell.runtime().has_item_field_use(&wrong_item_field_effect));
        assert!(
            shell
                .runtime()
                .item_field_use_keys()
                .contains(&item_field_use)
        );
        assert!(
            shell
                .runtime()
                .require_item_field_use(&item_field_use)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_item_field_use(&wrong_item_field_use)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_item_field_use(&wrong_item_field_effect)
                .is_err()
        );
        assert!(shell.runtime().has_guaranteed_capture_ball("MASTER_BALL"));
        assert!(!shell.runtime().has_guaranteed_capture_ball("master_ball"));
        assert!(
            shell
                .runtime()
                .guaranteed_capture_ball_ids()
                .contains("MASTER_BALL")
        );
        assert!(
            shell
                .runtime()
                .require_guaranteed_capture_ball("MASTER_BALL")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_guaranteed_capture_ball("master_ball")
                .is_err()
        );
        assert!(shell.runtime().has_capture_status_bonus("SLEEP"));
        assert!(!shell.runtime().has_capture_status_bonus("sleep"));
        assert!(shell.runtime().capture_status_bonus_ids().contains("SLEEP"));
        assert!(
            shell
                .runtime()
                .require_capture_status_bonus("SLEEP")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_capture_status_bonus("sleep")
                .is_err()
        );
        let capture_status_bonus = RuntimeCaptureStatusBonusKey {
            status: "SLEEP".to_string(),
            bonus: 10,
        };
        let wrong_capture_status_bonus = RuntimeCaptureStatusBonusKey {
            status: "SLEEP".to_string(),
            bonus: 9,
        };
        assert!(
            shell
                .runtime()
                .has_capture_status_bonus_key(&capture_status_bonus)
        );
        assert!(
            !shell
                .runtime()
                .has_capture_status_bonus_key(&wrong_capture_status_bonus)
        );
        assert!(
            shell
                .runtime()
                .capture_status_bonus_keys()
                .contains(&capture_status_bonus)
        );
        assert!(
            shell
                .runtime()
                .require_capture_status_bonus_key(&capture_status_bonus)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_capture_status_bonus_key(&wrong_capture_status_bonus)
                .is_err()
        );
        let capture_wobble_probability = RuntimeCaptureWobbleProbabilityKey {
            catch_rate: 255,
            chance: 255,
        };
        let wrong_capture_wobble_probability = RuntimeCaptureWobbleProbabilityKey {
            catch_rate: 255,
            chance: 254,
        };
        let missing_capture_wobble_probability = RuntimeCaptureWobbleProbabilityKey {
            catch_rate: 254,
            chance: 255,
        };
        assert!(
            shell
                .runtime()
                .has_capture_wobble_probability(&capture_wobble_probability)
        );
        assert!(
            !shell
                .runtime()
                .has_capture_wobble_probability(&wrong_capture_wobble_probability)
        );
        assert!(
            !shell
                .runtime()
                .has_capture_wobble_probability(&missing_capture_wobble_probability)
        );
        assert!(
            shell
                .runtime()
                .capture_wobble_probability_keys()
                .contains(&capture_wobble_probability)
        );
        assert!(
            shell
                .runtime()
                .require_capture_wobble_probability(&capture_wobble_probability)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_capture_wobble_probability(&wrong_capture_wobble_probability)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_capture_wobble_probability(&missing_capture_wobble_probability)
                .is_err()
        );
        assert!(shell.runtime().fast_ball_species_ids().is_empty());
        assert!(!shell.runtime().has_fast_ball_species("CHIKORITA"));
        assert!(
            shell
                .runtime()
                .require_fast_ball_species("CHIKORITA")
                .is_err()
        );
        assert!(shell.runtime().heavy_ball_species_ids().is_empty());
        assert!(!shell.runtime().has_heavy_ball_species("CHIKORITA"));
        assert!(
            shell
                .runtime()
                .require_heavy_ball_species("CHIKORITA")
                .is_err()
        );
        let missing_heavy_ball_modifier = RuntimeHeavyBallModifierKey {
            species_id: "CHIKORITA".to_string(),
            modifier: 40,
        };
        assert!(shell.runtime().heavy_ball_modifier_keys().is_empty());
        assert!(
            !shell
                .runtime()
                .has_heavy_ball_modifier(&missing_heavy_ball_modifier)
        );
        assert!(
            shell
                .runtime()
                .require_heavy_ball_modifier(&missing_heavy_ball_modifier)
                .is_err()
        );
        assert!(shell.runtime().has_move_priority_effect("NORMAL_HIT"));
        assert!(!shell.runtime().has_move_priority_effect("normal_hit"));
        assert!(
            shell
                .runtime()
                .move_priority_effect_ids()
                .contains("NORMAL_HIT")
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_effect("NORMAL_HIT")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_effect("normal_hit")
                .is_err()
        );
        let move_priority_effect = RuntimeMovePriorityEffectKey {
            effect_id: "NORMAL_HIT".to_string(),
            priority: 1,
        };
        let wrong_move_priority_effect = RuntimeMovePriorityEffectKey {
            effect_id: "NORMAL_HIT".to_string(),
            priority: 0,
        };
        assert!(
            shell
                .runtime()
                .has_move_priority_effect_key(&move_priority_effect)
        );
        assert!(
            !shell
                .runtime()
                .has_move_priority_effect_key(&wrong_move_priority_effect)
        );
        assert!(
            shell
                .runtime()
                .move_priority_effect_keys()
                .contains(&move_priority_effect)
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_effect_key(&move_priority_effect)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_effect_key(&wrong_move_priority_effect)
                .is_err()
        );
        assert!(shell.runtime().has_move_priority_move("VITAL_THROW"));
        assert!(!shell.runtime().has_move_priority_move("vital_throw"));
        assert!(
            shell
                .runtime()
                .move_priority_move_ids()
                .contains("VITAL_THROW")
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_move("VITAL_THROW")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_move("vital_throw")
                .is_err()
        );
        let move_priority_move = RuntimeMovePriorityMoveKey {
            move_id: "VITAL_THROW".to_string(),
            priority: 0,
        };
        let wrong_move_priority_move = RuntimeMovePriorityMoveKey {
            move_id: "VITAL_THROW".to_string(),
            priority: 1,
        };
        assert!(
            shell
                .runtime()
                .has_move_priority_move_key(&move_priority_move)
        );
        assert!(
            !shell
                .runtime()
                .has_move_priority_move_key(&wrong_move_priority_move)
        );
        assert!(
            shell
                .runtime()
                .move_priority_move_keys()
                .contains(&move_priority_move)
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_move_key(&move_priority_move)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_move_priority_move_key(&wrong_move_priority_move)
                .is_err()
        );
        let battle_stat_multiplier = RuntimeBattleStatMultiplierKey {
            table: "stat".to_string(),
            stage: 0,
            numerator: 1,
            denominator: 1,
        };
        let wrong_battle_stat_multiplier = RuntimeBattleStatMultiplierKey {
            denominator: 2,
            ..battle_stat_multiplier.clone()
        };
        let accuracy_multiplier = RuntimeBattleStatMultiplierKey {
            table: "accuracy".to_string(),
            stage: 6,
            numerator: 1,
            denominator: 1,
        };
        let missing_multiplier_table = RuntimeBattleStatMultiplierKey {
            table: "evasion".to_string(),
            stage: 0,
            numerator: 1,
            denominator: 1,
        };
        assert!(
            shell
                .runtime()
                .has_battle_stat_multiplier(&battle_stat_multiplier)
        );
        assert!(
            !shell
                .runtime()
                .has_battle_stat_multiplier(&wrong_battle_stat_multiplier)
        );
        assert!(
            shell
                .runtime()
                .has_battle_stat_multiplier(&accuracy_multiplier)
        );
        assert!(
            !shell
                .runtime()
                .has_battle_stat_multiplier(&missing_multiplier_table)
        );
        assert!(
            shell
                .runtime()
                .battle_stat_multiplier_keys()
                .contains(&battle_stat_multiplier)
        );
        assert!(
            shell
                .runtime()
                .battle_stat_multiplier_keys()
                .contains(&accuracy_multiplier)
        );
        assert!(
            shell
                .runtime()
                .require_battle_stat_multiplier(&battle_stat_multiplier)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_battle_stat_multiplier(&wrong_battle_stat_multiplier)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_battle_stat_multiplier(&missing_multiplier_table)
                .is_err()
        );
        let reward_max_level = RuntimeBattleRewardRuleKey {
            field: "max_level".to_string(),
            value: 100,
        };
        let wrong_reward_max_level = RuntimeBattleRewardRuleKey {
            field: "max_level".to_string(),
            value: 99,
        };
        let missing_reward_field = RuntimeBattleRewardRuleKey {
            field: "money_divisor".to_string(),
            value: 1,
        };
        assert!(shell.runtime().has_battle_reward_rule(&reward_max_level));
        assert!(
            !shell
                .runtime()
                .has_battle_reward_rule(&wrong_reward_max_level)
        );
        assert!(
            !shell
                .runtime()
                .has_battle_reward_rule(&missing_reward_field)
        );
        assert!(
            shell
                .runtime()
                .battle_reward_rule_keys()
                .contains(&reward_max_level)
        );
        assert!(
            shell
                .runtime()
                .require_battle_reward_rule(&reward_max_level)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_battle_reward_rule(&wrong_reward_max_level)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_battle_reward_rule(&missing_reward_field)
                .is_err()
        );
        let escape_roll_values = RuntimeBattleEscapeRuleKey {
            field: "rng_roll_values".to_string(),
            value: 256,
        };
        let wrong_escape_roll_values = RuntimeBattleEscapeRuleKey {
            field: "rng_roll_values".to_string(),
            value: 255,
        };
        let missing_escape_field = RuntimeBattleEscapeRuleKey {
            field: "trainer_escape_bonus".to_string(),
            value: 1,
        };
        assert!(shell.runtime().has_battle_escape_rule(&escape_roll_values));
        assert!(
            !shell
                .runtime()
                .has_battle_escape_rule(&wrong_escape_roll_values)
        );
        assert!(
            !shell
                .runtime()
                .has_battle_escape_rule(&missing_escape_field)
        );
        assert!(
            shell
                .runtime()
                .battle_escape_rule_keys()
                .contains(&escape_roll_values)
        );
        assert!(
            shell
                .runtime()
                .require_battle_escape_rule(&escape_roll_values)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_battle_escape_rule(&wrong_escape_roll_values)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_battle_escape_rule(&missing_escape_field)
                .is_err()
        );
        assert!(shell.runtime().has_physical_type("NORMAL"));
        assert!(!shell.runtime().has_physical_type("normal"));
        assert!(shell.runtime().physical_type_ids().contains("NORMAL"));
        assert!(shell.runtime().require_physical_type("NORMAL").is_ok());
        assert!(shell.runtime().require_physical_type("normal").is_err());
        assert!(shell.runtime().has_special_type("WATER"));
        assert!(!shell.runtime().has_special_type("water"));
        assert!(shell.runtime().special_type_ids().contains("WATER"));
        assert!(shell.runtime().require_special_type("WATER").is_ok());
        assert!(shell.runtime().require_special_type("water").is_err());
        assert!(shell.runtime().has_weather("WEATHER_RAIN"));
        assert!(!shell.runtime().has_weather("weather_rain"));
        assert!(shell.runtime().weather_ids().contains("WEATHER_RAIN"));
        assert!(shell.runtime().require_weather("WEATHER_RAIN").is_ok());
        assert!(shell.runtime().require_weather("weather_rain").is_err());
        let type_effectiveness = RuntimeTypeEffectivenessKey {
            attacking_type: "NORMAL".to_string(),
            defending_type: "FIGHTING".to_string(),
        };
        let wrong_type_effectiveness = RuntimeTypeEffectivenessKey {
            attacking_type: "normal".to_string(),
            defending_type: "FIGHTING".to_string(),
        };
        assert!(shell.runtime().has_type_effectiveness(&type_effectiveness));
        assert!(
            !shell
                .runtime()
                .has_type_effectiveness(&wrong_type_effectiveness)
        );
        assert!(
            shell
                .runtime()
                .type_effectiveness_keys()
                .contains(&type_effectiveness)
        );
        assert!(
            shell
                .runtime()
                .require_type_effectiveness(&type_effectiveness)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_type_effectiveness(&wrong_type_effectiveness)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .has_foresight_type_effectiveness(&type_effectiveness)
        );
        assert!(
            !shell
                .runtime()
                .has_foresight_type_effectiveness(&wrong_type_effectiveness)
        );
        assert!(
            shell
                .runtime()
                .foresight_type_effectiveness_keys()
                .contains(&type_effectiveness)
        );
        assert!(
            shell
                .runtime()
                .require_foresight_type_effectiveness(&type_effectiveness)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_foresight_type_effectiveness(&wrong_type_effectiveness)
                .is_err()
        );
        let weather_type_modifier = RuntimeWeatherTypeModifierKey {
            weather: "WEATHER_RAIN".to_string(),
            type_id: "WATER".to_string(),
        };
        let wrong_weather_type_modifier = RuntimeWeatherTypeModifierKey {
            weather: "weather_rain".to_string(),
            type_id: "WATER".to_string(),
        };
        assert!(
            shell
                .runtime()
                .has_weather_type_modifier(&weather_type_modifier)
        );
        assert!(
            !shell
                .runtime()
                .has_weather_type_modifier(&wrong_weather_type_modifier)
        );
        assert!(
            shell
                .runtime()
                .weather_type_modifier_keys()
                .contains(&weather_type_modifier)
        );
        assert!(
            shell
                .runtime()
                .require_weather_type_modifier(&weather_type_modifier)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_weather_type_modifier(&wrong_weather_type_modifier)
                .is_err()
        );
        let weather_effect_modifier = RuntimeWeatherMoveEffectModifierKey {
            weather: "WEATHER_RAIN".to_string(),
            effect_id: "SOLARBEAM".to_string(),
        };
        let wrong_weather_effect_modifier = RuntimeWeatherMoveEffectModifierKey {
            weather: "WEATHER_RAIN".to_string(),
            effect_id: "solarbeam".to_string(),
        };
        assert!(
            shell
                .runtime()
                .has_weather_move_effect_modifier(&weather_effect_modifier)
        );
        assert!(
            !shell
                .runtime()
                .has_weather_move_effect_modifier(&wrong_weather_effect_modifier)
        );
        assert!(
            shell
                .runtime()
                .weather_move_effect_modifier_keys()
                .contains(&weather_effect_modifier)
        );
        assert!(
            shell
                .runtime()
                .require_weather_move_effect_modifier(&weather_effect_modifier)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_weather_move_effect_modifier(&wrong_weather_effect_modifier)
                .is_err()
        );
        let audio_keys = shell.runtime().audio_asset_keys();
        let music_audio_asset = audio_keys
            .iter()
            .find(|key| key.audio_id == "MUSIC_ROUTE_29")
            .expect("music audio asset")
            .clone();
        let sfx_audio_asset = audio_keys
            .iter()
            .find(|key| key.audio_id == "SFX_TACKLE")
            .expect("sfx audio asset")
            .clone();
        let cry_audio_asset = audio_keys
            .iter()
            .find(|key| key.audio_id == "CRY_NIDORAN_M")
            .expect("cry audio asset")
            .clone();
        let wrong_audio_asset_source = RuntimeAudioAssetKey {
            source: "pcm".to_string(),
            ..music_audio_asset.clone()
        };
        let wrong_audio_asset_path = RuntimeAudioAssetKey {
            path: "content-packs/test/music/music_route_29.mid".to_string(),
            ..music_audio_asset.clone()
        };
        let wrong_audio_asset_kind = RuntimeAudioAssetKey {
            kind: "Cry".to_string(),
            ..cry_audio_asset.clone()
        };
        assert!(shell.runtime().has_audio_asset(&music_audio_asset));
        assert!(shell.runtime().has_audio_asset(&sfx_audio_asset));
        assert!(shell.runtime().has_audio_asset(&cry_audio_asset));
        assert!(!shell.runtime().has_audio_asset(&wrong_audio_asset_source));
        assert!(!shell.runtime().has_audio_asset(&wrong_audio_asset_path));
        assert!(!shell.runtime().has_audio_asset(&wrong_audio_asset_kind));
        assert!(
            shell
                .runtime()
                .audio_asset_keys()
                .contains(&music_audio_asset)
        );
        assert!(
            shell
                .runtime()
                .require_audio_asset(&music_audio_asset)
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .require_audio_asset(&wrong_audio_asset_source)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_audio_asset(&wrong_audio_asset_path)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_audio_asset(&wrong_audio_asset_kind)
                .is_err()
        );
        let pokemon_cry = RuntimePokemonCryKey {
            species_id: "CHIKORITA".to_string(),
            cry_id: "CRY_NIDORAN_M".to_string(),
            pitch: 0,
            length: 0,
        };
        let wrong_pokemon_cry_pitch = RuntimePokemonCryKey {
            pitch: 1,
            ..pokemon_cry.clone()
        };
        let wrong_pokemon_cry_id = RuntimePokemonCryKey {
            cry_id: "cry_nidoran_m".to_string(),
            ..pokemon_cry.clone()
        };
        assert!(shell.runtime().has_pokemon_cry(&pokemon_cry));
        assert!(!shell.runtime().has_pokemon_cry(&wrong_pokemon_cry_pitch));
        assert!(!shell.runtime().has_pokemon_cry(&wrong_pokemon_cry_id));
        assert!(shell.runtime().pokemon_cry_keys().contains(&pokemon_cry));
        assert!(shell.runtime().require_pokemon_cry(&pokemon_cry).is_ok());
        assert!(
            shell
                .runtime()
                .require_pokemon_cry(&wrong_pokemon_cry_pitch)
                .is_err()
        );
        assert!(
            shell
                .runtime()
                .require_pokemon_cry(&wrong_pokemon_cry_id)
                .is_err()
        );
        assert!(shell.runtime().has_music("MUSIC_ROUTE_29"));
        assert!(!shell.runtime().has_music("music_route_29"));
        assert!(shell.runtime().music_ids().contains("MUSIC_ROUTE_29"));
        assert!(shell.runtime().require_music("MUSIC_ROUTE_29").is_ok());
        assert!(shell.runtime().require_music("music_route_29").is_err());
        assert!(shell.runtime().has_sound_effect("SFX_TACKLE"));
        assert!(!shell.runtime().has_sound_effect("sfx_tackle"));
        assert!(shell.runtime().sound_effect_ids().contains("SFX_TACKLE"));
        assert!(shell.runtime().require_sound_effect("SFX_TACKLE").is_ok());
        assert!(shell.runtime().require_sound_effect("sfx_tackle").is_err());
        assert!(shell.runtime().has_cry("CRY_NIDORAN_M"));
        assert!(!shell.runtime().has_cry("cry_nidoran_m"));
        assert!(shell.runtime().cry_ids().contains("CRY_NIDORAN_M"));
        assert!(shell.runtime().require_cry("CRY_NIDORAN_M").is_ok());
        assert!(shell.runtime().require_cry("cry_nidoran_m").is_err());
        assert!(shell.has_item("MASTER_BALL"));
        assert!(shell.item_ids().contains("MASTER_BALL"));
        assert!(shell.require_item("MASTER_BALL").is_ok());
        assert!(shell.has_move("TACKLE"));
        assert!(shell.move_ids().contains("TACKLE"));
        assert!(shell.require_move("TACKLE").is_ok());
        assert!(shell.has_move_battle_data(&move_battle_data));
        assert!(shell.move_battle_data_keys().contains(&move_battle_data));
        assert!(shell.require_move_battle_data(&move_battle_data).is_ok());
        assert!(shell.has_species("CHIKORITA"));
        assert!(shell.species_ids().contains("CHIKORITA"));
        assert!(shell.require_species("CHIKORITA").is_ok());
        assert!(shell.has_species_battle_data(&species_battle_data));
        assert!(
            shell
                .species_battle_data_keys()
                .contains(&species_battle_data)
        );
        assert!(
            shell
                .require_species_battle_data(&species_battle_data)
                .is_ok()
        );
        assert!(shell.has_map("RuntimeMap"));
        assert!(shell.map_ids().contains("RuntimeMap"));
        assert!(shell.require_map("RuntimeMap").is_ok());
        assert!(shell.has_map_metadata(&runtime_map_metadata));
        assert!(shell.map_metadata_keys().contains(&runtime_map_metadata));
        assert!(shell.require_map_metadata(&runtime_map_metadata).is_ok());
        assert!(shell.has_trainer("RIVAL1"));
        assert!(shell.trainer_ids().contains("RIVAL1"));
        assert!(shell.require_trainer("RIVAL1").is_ok());
        assert!(shell.has_trainer_battle_data(&trainer_battle_data));
        assert!(
            shell
                .trainer_battle_data_keys()
                .contains(&trainer_battle_data)
        );
        assert!(
            shell
                .require_trainer_battle_data(&trainer_battle_data)
                .is_ok()
        );
        assert!(shell.has_trainer_party_pokemon(&trainer_party_pokemon));
        assert!(
            shell
                .trainer_party_pokemon_keys()
                .contains(&trainer_party_pokemon)
        );
        assert!(
            shell
                .require_trainer_party_pokemon(&trainer_party_pokemon)
                .is_ok()
        );
        assert!(shell.has_text("RuntimeText"));
        assert!(shell.text_ids().contains("RuntimeText"));
        assert!(shell.require_text("RuntimeText").is_ok());
        assert!(shell.has_script_text_body(&script_text_body));
        assert!(shell.script_text_body_keys().contains(&script_text_body));
        assert!(shell.require_script_text_body(&script_text_body).is_ok());
        assert!(shell.has_menu("RuntimeMenu"));
        assert!(shell.menu_ids().contains("RuntimeMenu"));
        assert!(shell.require_menu("RuntimeMenu").is_ok());
        assert!(shell.has_script_menu_definition(&script_menu_definition));
        assert!(
            shell
                .script_menu_definition_keys()
                .contains(&script_menu_definition)
        );
        assert!(
            shell
                .require_script_menu_definition(&script_menu_definition)
                .is_ok()
        );
        assert!(shell.has_phone_contact("PHONE_RUNTIME"));
        assert!(shell.phone_contact_ids().contains("PHONE_RUNTIME"));
        assert!(shell.require_phone_contact("PHONE_RUNTIME").is_ok());
        assert!(shell.has_special_phone_call("RuntimePhoneScript"));
        assert!(
            shell
                .special_phone_call_ids()
                .contains("RuntimePhoneScript")
        );
        assert!(
            shell
                .require_special_phone_call("RuntimePhoneScript")
                .is_ok()
        );
        assert!(!shell.has_npc_trade("NPC_TRADE_RUNTIME"));
        assert!(shell.npc_trade_ids().is_empty());
        assert!(shell.require_npc_trade("NPC_TRADE_RUNTIME").is_err());
        assert!(shell.has_sprite("SPRITE_MON"));
        assert!(shell.sprite_ids().contains("SPRITE_MON"));
        assert!(shell.require_sprite("SPRITE_MON").is_ok());
        assert!(shell.has_map_constant("RUNTIME_MAP"));
        assert!(shell.map_constants().contains("RUNTIME_MAP"));
        assert!(shell.require_map_constant("RUNTIME_MAP").is_ok());
        assert!(shell.has_event_flag("EVENT_RUNTIME_CONTESTANT"));
        assert!(shell.event_flag_ids().contains("EVENT_RUNTIME_CONTESTANT"));
        assert!(shell.require_event_flag("EVENT_RUNTIME_CONTESTANT").is_ok());
        assert!(shell.has_engine_flag("ENGINE_GOT_SHUCKIE_TODAY"));
        assert!(shell.engine_flag_ids().contains("ENGINE_GOT_SHUCKIE_TODAY"));
        assert!(
            shell
                .require_engine_flag("ENGINE_GOT_SHUCKIE_TODAY")
                .is_ok()
        );
        assert!(shell.has_spawn_identifier(0));
        assert!(shell.spawn_identifiers().contains(&0));
        assert!(shell.require_spawn_identifier(0).is_ok());
        assert!(shell.has_tileset("johto"));
        assert!(shell.tileset_ids().contains("johto"));
        assert!(shell.require_tileset("johto").is_ok());
        assert!(shell.has_tileset_row(&tileset_row));
        assert!(shell.tileset_keys().contains(&tileset_row));
        assert!(shell.require_tileset_row(&tileset_row).is_ok());
        assert!(shell.has_landmark("LANDMARK_RUNTIME"));
        assert!(shell.landmark_ids().contains("LANDMARK_RUNTIME"));
        assert!(shell.require_landmark("LANDMARK_RUNTIME").is_ok());
        assert!(shell.has_pc_string(&pc_string_row));
        assert!(shell.pc_string_keys().contains(&pc_string_row));
        assert!(shell.require_pc_string(&pc_string_row).is_ok());
        assert!(shell.has_menu_icon(&menu_icon_row));
        assert!(shell.menu_icon_keys().contains(&menu_icon_row));
        assert!(shell.require_menu_icon(&menu_icon_row).is_ok());
        assert!(shell.has_pokedex_entry(&pokedex_entry_row));
        assert!(shell.pokedex_entry_keys().contains(&pokedex_entry_row));
        assert!(shell.require_pokedex_entry(&pokedex_entry_row).is_ok());
        assert!(shell.has_pokegear_landmark(&pokegear_landmark_row));
        assert!(
            shell
                .pokegear_landmark_keys()
                .contains(&pokegear_landmark_row)
        );
        assert!(
            shell
                .require_pokegear_landmark(&pokegear_landmark_row)
                .is_ok()
        );
        assert!(shell.has_pokegear_map_landmark(&pokegear_map_landmark_row));
        assert!(
            shell
                .pokegear_map_landmark_keys()
                .contains(&pokegear_map_landmark_row)
        );
        assert!(
            shell
                .require_pokegear_map_landmark(&pokegear_map_landmark_row)
                .is_ok()
        );
        assert!(shell.has_fishing_rod(ROD_OLD));
        assert!(!shell.has_fishing_rod(ROD_GOOD));
        assert!(shell.fishing_rod_ids().contains(ROD_OLD));
        assert!(shell.require_fishing_rod(ROD_OLD).is_ok());
        assert!(shell.require_fishing_rod(ROD_GOOD).is_err());
        assert!(shell.has_map_group("RUNTIME"));
        assert!(shell.map_group_ids().contains("RUNTIME"));
        assert!(shell.require_map_group("RUNTIME").is_ok());
        assert!(shell.has_encounter_group("FISHGROUP_RUNTIME"));
        assert!(shell.encounter_group_ids().contains("FISHGROUP_RUNTIME"));
        assert!(shell.require_encounter_group("FISHGROUP_RUNTIME").is_ok());
        assert!(shell.has_mart("MART_RUNTIME"));
        assert!(shell.mart_ids().contains("MART_RUNTIME"));
        assert!(shell.require_mart("MART_RUNTIME").is_ok());
        assert!(shell.has_mart_row(&mart_row));
        assert!(shell.mart_keys().contains(&mart_row));
        assert!(shell.require_mart_row(&mart_row).is_ok());
        assert!(shell.has_fruit_tree("FRUITTREE_RUNTIME"));
        assert!(shell.fruit_tree_ids().contains("FRUITTREE_RUNTIME"));
        assert!(shell.require_fruit_tree("FRUITTREE_RUNTIME").is_ok());
        assert!(shell.has_fruit_tree_row(&fruit_tree_row));
        assert!(shell.fruit_tree_keys().contains(&fruit_tree_row));
        assert!(shell.require_fruit_tree_row(&fruit_tree_row).is_ok());
        assert!(shell.has_field_move_rule("cut"));
        assert!(shell.field_move_rule_ids().contains("cut"));
        assert!(shell.require_field_move_rule("cut").is_ok());
        assert!(shell.has_field_move_rule_row(&cut_rule_row));
        assert!(shell.field_move_rule_keys().contains(&cut_rule_row));
        assert!(shell.require_field_move_rule_row(&cut_rule_row).is_ok());
        assert!(shell.has_field_move_move("CUT"));
        assert!(shell.field_move_move_ids().contains("CUT"));
        assert!(shell.require_field_move_move("CUT").is_ok());
        assert!(shell.has_field_move_item("ESCAPE_ROPE"));
        assert!(shell.field_move_item_ids().contains("ESCAPE_ROPE"));
        assert!(shell.require_field_move_item("ESCAPE_ROPE").is_ok());
        let fly_destination_row = RuntimeFlyDestinationKey {
            flypoint_flag: "ENGINE_FLYPOINT_FLY_MAP".to_string(),
            destination_spawn_identifier: 14,
            label: "LANDMARK_FLY_MAP".to_string(),
        };
        assert!(shell.has_fly_destination("ENGINE_FLYPOINT_FLY_MAP"));
        assert!(
            shell
                .fly_destination_ids()
                .contains("ENGINE_FLYPOINT_FLY_MAP")
        );
        assert!(
            shell
                .require_fly_destination("ENGINE_FLYPOINT_FLY_MAP")
                .is_ok()
        );
        assert!(shell.has_fly_destination_row(&fly_destination_row));
        assert!(shell.fly_destination_keys().contains(&fly_destination_row));
        assert!(
            shell
                .require_fly_destination_row(&fly_destination_row)
                .is_ok()
        );
        assert!(shell.has_flee_mon_bucket("always"));
        assert!(shell.flee_mon_bucket_ids().contains("always"));
        assert!(shell.require_flee_mon_bucket("always").is_ok());
        assert!(shell.has_buena_password_category("BUENA_RUNTIME"));
        assert!(
            shell
                .buena_password_category_ids()
                .contains("BUENA_RUNTIME")
        );
        assert!(
            shell
                .require_buena_password_category("BUENA_RUNTIME")
                .is_ok()
        );
        assert!(shell.has_roaming_species("CHIKORITA"));
        assert!(shell.roaming_species_ids().contains("CHIKORITA"));
        assert!(shell.require_roaming_species("CHIKORITA").is_ok());
        assert!(shell.has_buena_prize_item("POKE_BALL"));
        assert!(shell.buena_prize_item_ids().contains("POKE_BALL"));
        assert!(shell.require_buena_prize_item("POKE_BALL").is_ok());
        assert!(shell.has_kurt_apricorn_item("BLU_APRICORN"));
        assert!(shell.kurt_apricorn_item_ids().contains("BLU_APRICORN"));
        assert!(shell.require_kurt_apricorn_item("BLU_APRICORN").is_ok());
        assert!(shell.has_dratini_move_set(0));
        assert!(shell.dratini_move_set_ids().contains(&0));
        assert!(shell.require_dratini_move_set(0).is_ok());
        assert!(shell.has_special_feature("bug_contest"));
        assert!(shell.special_feature_ids().contains("bug_contest"));
        assert!(shell.require_special_feature("bug_contest").is_ok());
        assert!(shell.has_oak_rating_text("OakRating01"));
        assert!(shell.oak_rating_text_ids().contains("OakRating01"));
        assert!(shell.require_oak_rating_text("OakRating01").is_ok());
        assert!(shell.has_odd_egg_species("CHIKORITA"));
        assert!(shell.odd_egg_species_ids().contains("CHIKORITA"));
        assert!(shell.require_odd_egg_species("CHIKORITA").is_ok());
        assert!(shell.has_magikarp_length_threshold(110));
        assert!(shell.magikarp_length_thresholds().contains(&110));
        assert!(shell.require_magikarp_length_threshold(110).is_ok());
        assert!(shell.has_happiness_change(9));
        assert!(!shell.has_happiness_change(1));
        assert!(shell.happiness_change_ids().contains(&9));
        assert!(shell.require_happiness_change(9).is_ok());
        assert!(shell.require_happiness_change(1).is_err());
        assert!(shell.has_happiness_service("RuntimeBootstrapHappiness"));
        assert!(!shell.has_happiness_service("haircut"));
        assert!(
            shell
                .happiness_service_ids()
                .contains("RuntimeBootstrapHappiness")
        );
        assert!(
            shell
                .require_happiness_service("RuntimeBootstrapHappiness")
                .is_ok()
        );
        assert!(shell.require_happiness_service("haircut").is_err());
        assert!(shell.has_pokemon_status("POISON"));
        assert!(shell.pokemon_status_ids().contains("POISON"));
        assert!(shell.require_pokemon_status("POISON").is_ok());
        assert!(!shell.has_fishing_daily_flag_bit(2));
        assert!(shell.fishing_daily_flag_bits().is_empty());
        assert!(shell.require_fishing_daily_flag_bit(2).is_err());
        assert!(!shell.has_fishing_swarm_flag(1));
        assert!(shell.fishing_swarm_flags().is_empty());
        assert!(shell.require_fishing_swarm_flag(1).is_err());
        assert!(!shell.has_pending_special_battle_type("BATTLETYPE_NORMAL"));
        assert!(shell.pending_special_battle_type_ids().is_empty());
        assert!(
            shell
                .require_pending_special_battle_type("BATTLETYPE_NORMAL")
                .is_err()
        );
        assert!(
            shell
                .wild_encounter_origin_keys()
                .contains(&runtime_wild_encounter)
        );
        assert!(shell.has_wild_encounter_origin(&runtime_wild_encounter));
        assert!(
            shell
                .require_wild_encounter_origin(&runtime_wild_encounter)
                .is_ok()
        );
        assert!(!shell.has_wild_encounter_origin(&missing_wild_encounter));
        assert!(
            shell
                .require_wild_encounter_origin(&missing_wild_encounter)
                .is_err()
        );
        assert!(shell.has_script_label("RuntimeScript"));
        assert!(shell.script_label_ids().contains("RuntimeScript"));
        assert!(shell.require_script_label("RuntimeScript").is_ok());
        assert!(shell.has_script_command(&runtime_script_command));
        assert!(
            shell
                .script_command_keys()
                .contains(&runtime_script_command)
        );
        assert!(
            shell
                .require_script_command(&runtime_script_command)
                .is_ok()
        );
        assert!(shell.has_script_command_payload(&runtime_script_payload));
        assert!(
            shell
                .script_command_payload_keys()
                .contains(&runtime_script_payload)
        );
        assert!(
            shell
                .require_script_command_payload(&runtime_script_payload)
                .is_ok()
        );
        assert!(shell.has_script_return(&runtime_script_return));
        assert!(shell.script_return_keys().contains(&runtime_script_return));
        assert!(shell.require_script_return(&runtime_script_return).is_ok());
        assert!(shell.has_script_vertical_menu(&runtime_vertical_menu));
        assert!(
            shell
                .script_vertical_menu_keys()
                .contains(&runtime_vertical_menu)
        );
        assert!(
            shell
                .require_script_vertical_menu(&runtime_vertical_menu)
                .is_ok()
        );
        assert!(shell.has_script_elevator(&runtime_elevator));
        assert!(shell.script_elevator_keys().contains(&runtime_elevator));
        assert!(shell.require_script_elevator(&runtime_elevator).is_ok());
        assert!(shell.has_gift_pokemon(&runtime_gift));
        assert!(shell.gift_pokemon_keys().contains(&runtime_gift));
        assert!(shell.require_gift_pokemon(&runtime_gift).is_ok());
        assert!(shell.has_map_script_section_command(&map_script_section_command));
        assert!(
            shell
                .map_script_section_command_keys()
                .contains(&map_script_section_command)
        );
        assert!(
            shell
                .require_map_script_section_command(&map_script_section_command)
                .is_ok()
        );
        assert!(shell.has_map_event_section_command(&map_event_section_command));
        assert!(
            shell
                .map_event_section_command_keys()
                .contains(&map_event_section_command)
        );
        assert!(
            shell
                .require_map_event_section_command(&map_event_section_command)
                .is_ok()
        );
        assert!(shell.has_warp(&runtime_warp));
        assert!(shell.warp_keys().contains(&runtime_warp));
        assert!(shell.require_warp(&runtime_warp).is_ok());
        assert!(shell.has_map_object(&runtime_map_object));
        assert!(shell.map_object_keys().contains(&runtime_map_object));
        assert!(shell.require_map_object(&runtime_map_object).is_ok());
        assert!(!shell.has_map_object(&missing_map_object));
        assert!(shell.require_map_object(&missing_map_object).is_err());
        assert!(shell.map_scene_keys().is_empty());
        assert!(!shell.has_map_scene(&missing_map_scene));
        assert!(shell.require_map_scene(&missing_map_scene).is_err());
        assert!(shell.has_currency_constant("MAX_MONEY"));
        assert!(shell.currency_constant_ids().contains("MAX_MONEY"));
        assert!(shell.require_currency_constant("MAX_MONEY").is_ok());
        assert!(shell.has_capture_ball_rule("POKE_BALL"));
        assert!(shell.capture_ball_rule_ids().contains("POKE_BALL"));
        assert!(shell.require_capture_ball_rule("POKE_BALL").is_ok());
        assert!(shell.has_capture_ball_rule_key(&capture_ball_rule));
        assert!(shell.capture_ball_rule_keys().contains(&capture_ball_rule));
        assert!(
            shell
                .require_capture_ball_rule_key(&capture_ball_rule)
                .is_ok()
        );
        assert!(shell.has_item_battle_use(&item_battle_use));
        assert!(shell.item_battle_use_keys().contains(&item_battle_use));
        assert!(shell.require_item_battle_use(&item_battle_use).is_ok());
        assert!(shell.has_item_field_use(&item_field_use));
        assert!(shell.item_field_use_keys().contains(&item_field_use));
        assert!(shell.require_item_field_use(&item_field_use).is_ok());
        assert!(shell.has_guaranteed_capture_ball("MASTER_BALL"));
        assert!(shell.guaranteed_capture_ball_ids().contains("MASTER_BALL"));
        assert!(shell.require_guaranteed_capture_ball("MASTER_BALL").is_ok());
        assert!(shell.has_capture_status_bonus("SLEEP"));
        assert!(shell.capture_status_bonus_ids().contains("SLEEP"));
        assert!(shell.require_capture_status_bonus("SLEEP").is_ok());
        assert!(shell.has_capture_status_bonus_key(&capture_status_bonus));
        assert!(
            shell
                .capture_status_bonus_keys()
                .contains(&capture_status_bonus)
        );
        assert!(
            shell
                .require_capture_status_bonus_key(&capture_status_bonus)
                .is_ok()
        );
        assert!(shell.has_capture_wobble_probability(&capture_wobble_probability));
        assert!(
            shell
                .capture_wobble_probability_keys()
                .contains(&capture_wobble_probability)
        );
        assert!(
            shell
                .require_capture_wobble_probability(&capture_wobble_probability)
                .is_ok()
        );
        assert!(shell.fast_ball_species_ids().is_empty());
        assert!(shell.require_fast_ball_species("CHIKORITA").is_err());
        assert!(shell.heavy_ball_species_ids().is_empty());
        assert!(shell.require_heavy_ball_species("CHIKORITA").is_err());
        assert!(shell.heavy_ball_modifier_keys().is_empty());
        assert!(
            shell
                .require_heavy_ball_modifier(&missing_heavy_ball_modifier)
                .is_err()
        );
        assert!(shell.has_move_priority_effect("NORMAL_HIT"));
        assert!(shell.move_priority_effect_ids().contains("NORMAL_HIT"));
        assert!(shell.require_move_priority_effect("NORMAL_HIT").is_ok());
        assert!(shell.has_move_priority_effect_key(&move_priority_effect));
        assert!(
            shell
                .move_priority_effect_keys()
                .contains(&move_priority_effect)
        );
        assert!(
            shell
                .require_move_priority_effect_key(&move_priority_effect)
                .is_ok()
        );
        assert!(shell.has_move_priority_move("VITAL_THROW"));
        assert!(shell.move_priority_move_ids().contains("VITAL_THROW"));
        assert!(shell.require_move_priority_move("VITAL_THROW").is_ok());
        assert!(shell.has_move_priority_move_key(&move_priority_move));
        assert!(
            shell
                .move_priority_move_keys()
                .contains(&move_priority_move)
        );
        assert!(
            shell
                .require_move_priority_move_key(&move_priority_move)
                .is_ok()
        );
        assert!(shell.has_battle_stat_multiplier(&battle_stat_multiplier));
        assert!(
            shell
                .battle_stat_multiplier_keys()
                .contains(&battle_stat_multiplier)
        );
        assert!(
            shell
                .battle_stat_multiplier_keys()
                .contains(&accuracy_multiplier)
        );
        assert!(
            shell
                .require_battle_stat_multiplier(&battle_stat_multiplier)
                .is_ok()
        );
        assert!(shell.has_battle_reward_rule(&reward_max_level));
        assert!(shell.battle_reward_rule_keys().contains(&reward_max_level));
        assert!(shell.require_battle_reward_rule(&reward_max_level).is_ok());
        assert!(shell.has_battle_escape_rule(&escape_roll_values));
        assert!(
            shell
                .battle_escape_rule_keys()
                .contains(&escape_roll_values)
        );
        assert!(
            shell
                .require_battle_escape_rule(&escape_roll_values)
                .is_ok()
        );
        assert!(shell.has_physical_type("NORMAL"));
        assert!(shell.physical_type_ids().contains("NORMAL"));
        assert!(shell.require_physical_type("NORMAL").is_ok());
        assert!(shell.has_special_type("WATER"));
        assert!(shell.special_type_ids().contains("WATER"));
        assert!(shell.require_special_type("WATER").is_ok());
        assert!(shell.has_weather("WEATHER_RAIN"));
        assert!(shell.weather_ids().contains("WEATHER_RAIN"));
        assert!(shell.require_weather("WEATHER_RAIN").is_ok());
        assert!(shell.has_type_effectiveness(&type_effectiveness));
        assert!(
            shell
                .type_effectiveness_keys()
                .contains(&type_effectiveness)
        );
        assert!(
            shell
                .require_type_effectiveness(&type_effectiveness)
                .is_ok()
        );
        assert!(shell.has_foresight_type_effectiveness(&type_effectiveness));
        assert!(
            shell
                .foresight_type_effectiveness_keys()
                .contains(&type_effectiveness)
        );
        assert!(
            shell
                .require_foresight_type_effectiveness(&type_effectiveness)
                .is_ok()
        );
        assert!(shell.has_weather_type_modifier(&weather_type_modifier));
        assert!(
            shell
                .weather_type_modifier_keys()
                .contains(&weather_type_modifier)
        );
        assert!(
            shell
                .require_weather_type_modifier(&weather_type_modifier)
                .is_ok()
        );
        assert!(shell.has_weather_move_effect_modifier(&weather_effect_modifier));
        assert!(
            shell
                .weather_move_effect_modifier_keys()
                .contains(&weather_effect_modifier)
        );
        assert!(
            shell
                .require_weather_move_effect_modifier(&weather_effect_modifier)
                .is_ok()
        );
        assert!(shell.has_audio_asset(&music_audio_asset));
        assert!(shell.has_audio_asset(&sfx_audio_asset));
        assert!(shell.has_audio_asset(&cry_audio_asset));
        assert!(shell.audio_asset_keys().contains(&music_audio_asset));
        assert!(shell.require_audio_asset(&music_audio_asset).is_ok());
        assert!(shell.has_pokemon_cry(&pokemon_cry));
        assert!(shell.pokemon_cry_keys().contains(&pokemon_cry));
        assert!(shell.require_pokemon_cry(&pokemon_cry).is_ok());
        assert!(shell.has_music("MUSIC_ROUTE_29"));
        assert!(shell.music_ids().contains("MUSIC_ROUTE_29"));
        assert!(shell.require_music("MUSIC_ROUTE_29").is_ok());
        assert!(shell.has_sound_effect("SFX_TACKLE"));
        assert!(shell.sound_effect_ids().contains("SFX_TACKLE"));
        assert!(shell.require_sound_effect("SFX_TACKLE").is_ok());
        assert!(shell.has_cry("CRY_NIDORAN_M"));
        assert!(shell.cry_ids().contains("CRY_NIDORAN_M"));
        assert!(shell.require_cry("CRY_NIDORAN_M").is_ok());
        let link_descriptor = shell
            .link_session_descriptor("runtime-session", 7, "P7")
            .expect("runtime link session descriptor");
        assert_eq!(link_descriptor.session.session_id(), "runtime-session");
        assert_eq!(link_descriptor.session.modpack().id(), "core-modular");
        assert_eq!(
            link_descriptor.session.modpack().hash(),
            shell.runtime().modpack().hash()
        );
        assert_eq!(
            link_descriptor.session.pack_content_hash(),
            shell.runtime().pack_identity().content_hash.as_str()
        );
        assert_eq!(link_descriptor.local_player.id(), 7);
        assert_eq!(link_descriptor.local_player.display_name(), "P7");
        assert_eq!(link_descriptor.hello.session(), &link_descriptor.session);
        assert_eq!(
            link_descriptor.hello.player(),
            &link_descriptor.local_player
        );
        assert_eq!(link_descriptor.checksum.player_id(), 7);
        assert_eq!(
            link_descriptor.checksum.frame(),
            initial.state_checksum.frame()
        );
        assert_eq!(link_descriptor.checksum.checksum(), initial.state_checksum);
        assert_eq!(
            link_descriptor.save_checkpoint.session(),
            &link_descriptor.session
        );
        assert_eq!(
            link_descriptor.save_checkpoint.checkpoint().checksum(),
            &link_descriptor.checksum
        );
        assert_eq!(
            link_descriptor
                .save_checkpoint
                .checkpoint()
                .summary()
                .pack_content_hash(),
            shell.runtime().pack_identity().content_hash.as_str()
        );
        let wrong_pack_summary = serde_json::from_value::<SaveGameSummary>(serde_json::json!({
            "format_version": crystal_core::save::SAVE_FORMAT_VERSION,
            "modpack": {
                "id": "other-pack",
                "hash": shell.runtime().modpack().hash()
            },
            "pack_content_hash": shell.runtime().pack_identity().content_hash.as_str(),
            "created_frame": link_descriptor.checksum.frame(),
            "saved_frame": link_descriptor.checksum.frame(),
            "state_frame": link_descriptor.checksum.frame(),
            "state_hash": link_descriptor.checksum.hash()
        }))
        .expect("wrong-pack summary shape");
        let unchecked_wrong_pack_checkpoint = SessionSaveCheckpointFrame::new_unchecked_for_tests(
            link_descriptor.session.clone(),
            SaveCheckpointFrame::new_unchecked_for_tests(
                wrong_pack_summary,
                link_descriptor.checksum.clone(),
            ),
        );
        let mut invalid_descriptor = link_descriptor.clone();
        invalid_descriptor.save_checkpoint = unchecked_wrong_pack_checkpoint;
        let descriptor_error = shell
            .validate_link_session_descriptor(&invalid_descriptor)
            .expect_err("runtime link descriptors must revalidate save checkpoint pack identity");
        let descriptor_error_chain = format!("{descriptor_error:#}");
        assert!(
            descriptor_error_chain.contains("runtime link save checkpoint is invalid")
                && descriptor_error_chain.contains("save summary modpack id other-pack"),
            "{descriptor_error_chain}"
        );
        let (transport, peer_transport) =
            crystal_net::MemoryLinkTransport::pair_for_session(link_descriptor.session.clone())
                .expect("memory link transport");
        let mut endpoint = shell
            .link_endpoint(transport, &link_descriptor)
            .expect("runtime link endpoint");
        let peer_hello = LinkHello::from_session(
            link_descriptor.session.clone(),
            PlayerIdentity::new(8, "P8").expect("peer player"),
        )
        .expect("peer hello");
        let mut peer_endpoint = crystal_net::LinkEndpoint::new(peer_transport, peer_hello.clone())
            .expect("peer endpoint");
        endpoint.send_hello().expect("send runtime link hello");
        peer_endpoint.send_hello().expect("send peer link hello");
        assert_eq!(
            endpoint.poll().expect("host hello poll"),
            vec![crystal_net::LinkEndpointEvent::PeerHello(
                peer_hello.clone()
            )]
        );
        shell
            .send_link_save_checkpoint(&mut endpoint, &link_descriptor)
            .expect("send link save checkpoint");
        assert_eq!(
            peer_endpoint.poll().expect("peer bootstrap poll"),
            vec![
                crystal_net::LinkEndpointEvent::PeerHello(link_descriptor.hello.clone()),
                crystal_net::LinkEndpointEvent::PeerSaveCheckpoint {
                    player_id: 7,
                    checkpoint: link_descriptor.save_checkpoint.checkpoint().clone()
                }
            ]
        );
        assert_eq!(
            peer_endpoint.peer_checkpoints().get(&7),
            Some(link_descriptor.save_checkpoint.checkpoint())
        );
        assert!(peer_endpoint.is_ready());
        assert!(peer_endpoint.is_ready_for_gameplay());
        shell
            .require_link_checkpoints(&peer_endpoint, [7, 8])
            .expect("peer has host checkpoint");
        let terminal_checksum = StateChecksumFrame::new(
            7,
            crystal_core::timing::Frame(initial.state_checksum.frame() + 1),
            0xbbcc_ddee,
        );
        let input_journal = shell
            .local_input_journal(
                &link_descriptor,
                terminal_checksum.clone(),
                [(initial.state_checksum.frame(), B_PAD_RIGHT)],
            )
            .expect("runtime local input journal");
        assert_eq!(input_journal.journal.session(), &link_descriptor.session);
        assert_eq!(
            input_journal.journal.start_checksum(),
            &link_descriptor.checksum
        );
        assert_eq!(
            input_journal.journal.terminal_checksum(),
            &terminal_checksum
        );
        assert_eq!(input_journal.journal.players(), &BTreeSet::from([7]));
        assert_eq!(input_journal.journal.frames().len(), 1);
        assert_eq!(
            input_journal
                .fingerprint_hex()
                .expect("journal fingerprint")
                .len(),
            8
        );
        let input_journal_message = shell
            .input_journal_message(input_journal.clone())
            .expect("input journal message");
        let LinkMessage::InputJournal(input_journal_frame) = input_journal_message else {
            panic!("expected input journal message");
        };
        assert_eq!(
            input_journal_frame.fingerprint(),
            input_journal
                .fingerprint_hex()
                .expect("input journal fingerprint")
        );
        assert_eq!(input_journal_frame.journal(), &input_journal.journal);
        let replay_descriptor = shell
            .link_session_descriptor("runtime-replay", 1, "P1")
            .expect("runtime replay descriptor");
        let mut recorded_journal_shell = shell.clone();
        recorded_journal_shell.clear_retained_runtime_commands();
        let recorded_journal = recorded_journal_shell
            .record_local_input_journal(&replay_descriptor, vec![vec![GameButton::Right]])
            .expect("record local runtime input journal");
        assert_eq!(
            recorded_journal.journal.start_checksum(),
            &replay_descriptor.checksum
        );
        assert_eq!(recorded_journal.journal.players(), &BTreeSet::from([1]));
        assert_eq!(recorded_journal.journal.frames().len(), 1);
        assert_eq!(
            recorded_journal.journal.frames()[0].frame(),
            initial.state_checksum.frame()
        );
        assert_eq!(
            recorded_journal.journal.frames()[0].joypad_mask_for(1),
            Some(B_PAD_RIGHT)
        );
        assert_eq!(
            recorded_journal.journal.terminal_checksum(),
            &recorded_journal_shell
                .state_checksum_frame(1)
                .expect("recorded terminal checksum")
        );
        let replay_commands = recorded_journal_shell
            .retained_runtime_commands()
            .iter()
            .cloned()
            .map(|command| {
                SessionRuntimeCommandFrame::new(
                    replay_descriptor.session.clone(),
                    command,
                )
                .expect("bind replay command")
            })
            .collect();
        let replay_results = recorded_journal_shell
            .retained_runtime_results()
            .iter()
            .cloned()
            .map(|result| {
                SessionRuntimeCommandResultFrame::new(
                    replay_descriptor.session.clone(),
                    result,
                )
                .expect("bind replay result")
            })
            .collect();
        let save_resume_message = recorded_journal_shell
            .save_resume_replay_message(
                &replay_descriptor,
                recorded_journal.clone(),
                replay_commands,
                replay_results,
                Vec::new(),
            )
            .expect("save exact command-authoritative replay message");
        let LinkMessage::SaveResumeReplay(save_resume_replay) = save_resume_message else {
            panic!("expected save resume replay message");
        };
        assert_eq!(
            save_resume_replay.checkpoint(),
            &replay_descriptor.save_checkpoint
        );
        let mut replayed_journal_shell = shell.clone();
        replayed_journal_shell.clear_retained_runtime_commands();
        replayed_journal_shell
            .validate_local_input_journal_start(&replay_descriptor, &recorded_journal.journal)
            .expect("preflight local input journal");
        let replayed_journal = replayed_journal_shell
            .apply_deterministic_replay_bundle(
                &replay_descriptor,
                save_resume_replay.replay(),
            )
            .expect("apply exact runtime command replay");
        assert_eq!(
            replayed_journal.terminal_checksum,
            recorded_journal.terminal_checksum
        );
        assert_eq!(
            replayed_journal_shell
                .state_checksum_frame(1)
                .expect("replayed terminal checksum"),
            recorded_journal.terminal_checksum
        );
        assert!(
            replayed_journal_shell
                .validate_local_input_journal_start(
                    &replay_descriptor,
                    &recorded_journal.journal,
                )
                .is_err()
        );
        assert_eq!(
            initial.audio.current_music.as_deref(),
            Some("MUSIC_ROUTE_29")
        );
        assert!(
            shell
                .runtime()
                .audio()
                .require_music("MUSIC_ROUTE_29")
                .is_ok()
        );
        assert!(
            shell
                .runtime()
                .audio()
                .require_sound_effect("SFX_TACKLE")
                .is_ok()
        );
        assert!(shell.runtime().audio().require_cry("CRY_NIDORAN_M").is_ok());
        assert!(
            shell
                .runtime()
                .audio()
                .require_sound_effect("sfx_tackle")
                .is_err()
        );
        let music_manifest = initial
            .audio_catalog
            .manifest
            .music
            .get("MUSIC_ROUTE_29")
            .expect("compiled music manifest entry");
        assert_eq!(
            music_manifest.path,
            "content-packs/test/music/MUSIC_ROUTE_29.mid"
        );
        assert_eq!(music_manifest.source, ModpackAudioSource::Midi);
        let music_playback = initial
            .audio_catalog
            .playback
            .music
            .get("MUSIC_ROUTE_29")
            .expect("compiled music playback entry");
        assert_eq!(music_playback.mode, ModpackAudioPlaybackMode::SequencedMidi);
        assert_eq!(music_playback.loop_policy, ModpackAudioLoopPolicy::Loop);
        let music_program = initial
            .audio_catalog
            .music
            .get("MUSIC_ROUTE_29")
            .expect("loaded music program");
        assert!(music_program.cache_key.contains("MUSIC_ROUTE_29.mid"));
        assert_eq!(
            music_program.source,
            RuntimeAudioProgramSourceSnapshot::Midi {
                byte_len: music_manifest.byte_len,
            }
        );
        let sfx_manifest = initial
            .audio_catalog
            .manifest
            .sound_effects
            .get("SFX_TACKLE")
            .expect("compiled sound effect manifest entry");
        assert_eq!(sfx_manifest.source, ModpackAudioSource::Midi);
        assert_eq!(
            initial
                .audio_catalog
                .sound_effects
                .get("SFX_TACKLE")
                .expect("loaded sound effect program")
                .source,
            RuntimeAudioProgramSourceSnapshot::Midi {
                byte_len: sfx_manifest.byte_len,
            }
        );
        let cry_manifest = initial
            .audio_catalog
            .manifest
            .cries
            .get("CRY_NIDORAN_M")
            .expect("compiled cry manifest entry");
        assert_eq!(cry_manifest.source, ModpackAudioSource::Midi);
        assert_eq!(
            initial
                .audio_catalog
                .cries
                .get("CRY_NIDORAN_M")
                .expect("loaded cry program")
                .source,
            RuntimeAudioProgramSourceSnapshot::Midi {
                byte_len: cry_manifest.byte_len,
            }
        );
        assert!(initial.menu.is_none());
        assert!(initial.ui.menu.is_none());
        assert_eq!(initial.ui.gift_pokemon.len(), 1);
        assert_eq!(initial.ui.gift_pokemon[0].map_name, "RuntimeMap");
        assert_eq!(
            initial.ui.gift_pokemon[0].source_script,
            "RuntimeGiftScript"
        );
        assert_eq!(initial.ui.gift_pokemon[0].command_index, 12);
        assert_eq!(initial.ui.gift_pokemon[0].species_id, "CHIKORITA");
        assert_eq!(
            initial.ui.gift_pokemon[0].nickname_label.as_deref(),
            Some("RuntimeGiftName")
        );
        assert!(!initial.ui.gift_pokemon[0].egg);
        assert!(initial.ui.text.is_none());
        assert!(!initial.ui.window_open);
        assert!(!initial.ui.text_window_open);
        assert!(initial.ui.coords.is_none());
        assert!(initial.ui.active_pokemon_picture.is_none());
        assert!(initial.battle.is_none());
        assert!(initial.party.slots.is_empty());
        assert!(initial.bag.items.is_empty());
        assert!(initial.bag.balls.is_empty());
        assert!(initial.bag.key_items.is_empty());
        assert!(initial.bag.tm_hm.is_empty());
        assert!(initial.bag.pc_items.is_empty());
        assert!(initial.bag.custom_pockets.is_empty());
        let catalog_berry = shell
            .runtime()
            .data
            .items
            .get("BERRY")
            .expect("compiled item catalog exposes exact BERRY payload");
        assert_eq!(catalog_berry.effect, "NONE");
        assert_eq!(catalog_berry.held_effect, "HELD_NONE");
        assert_eq!(catalog_berry.pocket, "ITEM");
        assert!(catalog_berry.field_usable);
        let catalog_tackle = initial
            .moves
            .iter()
            .find(|move_data| move_data.move_id == "TACKLE")
            .expect("compiled move catalog exposes exact TACKLE payload");
        assert_eq!(catalog_tackle.name, "TACKLE");
        assert_eq!(catalog_tackle.move_type, "NORMAL");
        assert_eq!(catalog_tackle.effect, "NORMAL_HIT");
        assert_eq!(catalog_tackle.power, 40);
        assert_eq!(catalog_tackle.accuracy, 100);
        assert_eq!(catalog_tackle.pp, 35);
        let catalog_chikorita = initial
            .pokemon
            .iter()
            .find(|species| species.species_id == "CHIKORITA")
            .expect("compiled Pokemon catalog exposes exact CHIKORITA payload");
        assert_eq!(
            catalog_chikorita.base_stats,
            BaseStats::new(45, 49, 65, 45, 49, 65)
        );
        assert_eq!(catalog_chikorita.type1, "NORMAL");
        assert_eq!(catalog_chikorita.type2, "NORMAL");
        assert_eq!(catalog_chikorita.growth_rate, "GROWTH_MEDIUM_SLOW");
        assert_eq!(catalog_chikorita.egg_group1, "EGG_MONSTER");
        assert_eq!(catalog_chikorita.ability, "NONE");
        let catalog_trainer = initial
            .trainers
            .iter()
            .find(|trainer| trainer.trainer_id == "RIVAL1")
            .expect("compiled trainer catalog exposes exact RIVAL1 payload");
        assert_eq!(catalog_trainer.name, "RIVAL@");
        assert_eq!(catalog_trainer.trainer_class, "RIVAL1");
        assert_eq!(catalog_trainer.party.len(), 1);
        assert_eq!(catalog_trainer.party[0].species, "CHIKORITA");
        assert_eq!(catalog_trainer.party[0].level, 5);
        assert_eq!(catalog_trainer.base_reward, 100);
        assert_eq!(catalog_trainer.ai_move_flags, 1);
        assert_eq!(catalog_trainer.encounter_music, "MUSIC_RIVAL_ENCOUNTER");
        assert_eq!(catalog_trainer.ai_layers, vec!["AI_BASIC".to_string()]);
        let catalog_map = initial
            .maps
            .iter()
            .find(|map| map.map_name == "RuntimeMap")
            .expect("compiled map catalog exposes exact RuntimeMap payload");
        assert_eq!(catalog_map.id, "RuntimeMap");
        assert_eq!(catalog_map.attributes.tileset_name, "johto");
        assert_eq!(catalog_map.attributes.width, 2);
        assert_eq!(catalog_map.attributes.height, 1);
        assert_eq!(catalog_map.blocks, vec![0, 0]);
        let map_metadata = catalog_map.metadata.as_ref().expect("runtime map metadata");
        assert_eq!(map_metadata.constant, "RUNTIME_MAP");
        assert_eq!(map_metadata.environment, "ROUTE");
        assert_eq!(catalog_map.events.warps.len(), 1);
        assert_eq!(catalog_map.events.warps[0].target_map, "RUNTIME_MAP");
        assert_eq!(
            catalog_map.objects[0].object_identifier.as_deref(),
            Some("RuntimeNpc")
        );
        let catalog_tileset = initial
            .tilesets
            .iter()
            .find(|tileset| tileset.tileset_id == "johto")
            .expect("compiled tileset catalog exposes exact johto payload");
        assert_eq!(
            catalog_tileset
                .collision
                .get("00")
                .expect("metatile 0 collision"),
            &vec![
                "FLOOR".to_string(),
                "FLOOR".to_string(),
                "FLOOR".to_string(),
                "FLOOR".to_string(),
            ]
        );
        assert_eq!(
            initial
                .encounters
                .slot_tables
                .tables
                .get(EncounterSurface::Grass.as_key())
                .expect("grass encounter slot table")[0],
            EncounterSlotChance {
                threshold: 100,
                slot: 0,
            }
        );
        assert!(initial.encounters.wild.contains_key("RuntimeMap"));
        assert!(initial.encounters.field.is_empty());
        assert!(
            initial
                .encounters
                .fishing
                .groups
                .contains_key("FISHGROUP_RUNTIME")
        );
        assert!(
            initial
                .battle_rules
                .capture_rules
                .guaranteed_capture_balls
                .contains("MASTER_BALL")
        );
        assert_eq!(
            initial.battle_rules.capture_wobble_probabilities[0].catch_rate,
            255
        );
        assert_eq!(initial.battle_rules.reward_rules.max_level, 100);
        assert_eq!(
            initial.battle_rules.escape_rules.player_speed_multiplier,
            32
        );
        assert_eq!(initial.battle_rules.move_priorities.base_priority, 1);
        assert_eq!(
            initial
                .battle_rules
                .move_priorities
                .effect_priorities
                .get("NORMAL_HIT"),
            Some(&1)
        );
        assert!(
            initial
                .battle_rules
                .type_categories
                .physical
                .contains(&"NORMAL".to_string())
        );
        assert!(
            initial
                .battle_rules
                .weather_modifiers
                .type_modifiers
                .contains_key("WEATHER_RAIN")
        );
        assert_eq!(
            initial.world_rules.currency.0.get("MAX_MONEY"),
            Some(&999_999)
        );
        assert!(initial.world_rules.marts.0.contains_key("MART_RUNTIME"));
        assert!(
            initial
                .world_rules
                .fruit_trees
                .0
                .contains_key("FRUITTREE_RUNTIME")
        );
        assert_eq!(initial.world_rules.field_moves.cut.move_id, "CUT");
        assert_eq!(initial.world_rules.field_moves.fly.badge.index, 5);
        assert_eq!(
            initial.world_rules.field_moves.escape_rope.escape_rope_mode,
            "DIG_WARP"
        );
        assert_eq!(
            initial.presentation.pc_strings.get("PC_RUNTIME"),
            Some(&"Runtime PC".to_string())
        );
        assert_eq!(
            initial.presentation.menu_icons.get("CHIKORITA"),
            Some(&"ICON_CHIKORITA".to_string())
        );
        assert_eq!(
            initial
                .presentation
                .pokedex_entries
                .get("CHIKORITA")
                .expect("compiled pokedex entry")
                .classification,
            "Leaf"
        );
        assert_eq!(initial.presentation.move_names, vec!["TACKLE".to_string()]);
        assert_eq!(
            initial.presentation.battle_animation_table,
            vec!["ANIM_NULL".to_string(), "TACKLE".to_string()]
        );
        assert_eq!(
            initial.presentation.battle_animations.get("TACKLE"),
            Some(&vec!["BATTLE_ANIM_END".to_string()])
        );
        assert!(
            initial
                .presentation
                .battle_anim_bundle
                .contains("BattleAnim_Tackle")
        );
        assert!(
            initial
                .presentation
                .sprite_anim_bundle
                .contains("SpriteAnimFrame")
        );
        assert_eq!(
            initial
                .presentation
                .sprite_palette_defaults
                .get("SPRITE_MON"),
            Some(&0)
        );
        assert_eq!(
            initial
                .presentation
                .pokegear_town_map_palette_map
                .get("RuntimeMap"),
            Some(&vec!["PAL_RUNTIME".to_string()])
        );
        assert_eq!(
            initial.presentation.pokegear_landmarks.landmarks[0].constant,
            "LANDMARK_RUNTIME"
        );
        assert_eq!(
            initial
                .presentation
                .pokemon_cries
                .get("CHIKORITA")
                .expect("compiled cry metadata")
                .cry,
            "CRY_NIDORAN_M"
        );
        assert_eq!(
            initial
                .presentation
                .pokemon_frontpic_anim
                .get("CHIKORITA")
                .expect("compiled frontpic animation")
                .commands[0]
                .duration,
            Some(8)
        );
        let phone_contact = initial
            .special
            .phone_contacts
            .0
            .get("PHONE_RUNTIME")
            .expect("compiled phone contact");
        assert_eq!(phone_contact.primary_label, "RuntimePhone");
        assert_eq!(
            phone_contact.caller_script.as_deref(),
            Some("RuntimePhoneScript")
        );
        assert!(
            initial
                .special
                .permanent_phone_numbers
                .contains_key("PHONE_RUNTIME")
        );
        assert!(
            initial
                .special
                .special_phone_calls
                .contains_key("RuntimePhoneScript")
        );
        assert!(
            initial
                .special
                .flee_mons
                .buckets
                .get("always")
                .expect("flee mon bucket")
                .contains(&"CHIKORITA".to_string())
        );
        assert_eq!(
            initial.special.buena_password_categories.order,
            vec!["BUENA_RUNTIME".to_string()]
        );
        assert_eq!(
            initial
                .special
                .roaming_pokemon
                .init_write(0)
                .map(|write| write.level),
            Some(40)
        );
        assert_eq!(initial.special.buena_prizes.get("POKE_BALL"), Some(&1));
        assert_eq!(
            initial.special.kurt_apricorn_recipes.get("BLU_APRICORN"),
            Some(&"POKE_BALL".to_string())
        );
        assert_eq!(
            initial
                .special
                .shuckie_gift
                .as_ref()
                .expect("compiled Shuckie gift")
                .nickname,
            "SHUCKIE"
        );
        assert_eq!(
            initial.special.dratini_move_sets.get(&0),
            Some(&vec!["TACKLE".to_string()])
        );
        assert_eq!(
            initial
                .special
                .bug_contest_config
                .as_ref()
                .expect("compiled bug contest config")
                .park_balls,
            20
        );
        assert_eq!(
            initial
                .special
                .battle_tower_rules
                .as_ref()
                .expect("compiled battle tower rules")
                .required_party_count,
            3
        );
        assert_eq!(initial.special.oak_ratings[0].text_label, "OakRating01");
        assert_eq!(initial.special.odd_egg_definitions[0].species, "CHIKORITA");
        assert_eq!(initial.special.magikarp_lengths[0].threshold, 110);
        assert!(initial.special.happiness_data.is_some());
        assert!(
            initial
                .story
                .initialize_events
                .event_flags
                .contains(&"EVENT_RUNTIME_CONTESTANT".to_string())
        );
        assert!(
            initial
                .story
                .initialize_events
                .engine_flags
                .contains(&"ENGINE_GOT_SHUCKIE_TODAY".to_string())
        );
        assert_eq!(
            initial
                .story
                .story_event_script_constants
                .global
                .get("EVENT_RUNTIME"),
            Some(&1)
        );
        assert_eq!(
            initial.playability,
            crystal_assets::PlayabilityRules::default()
        );
        assert_eq!(initial.storage.current_pc_box, 0);
        assert_eq!(initial.storage.party_count, 0);
        assert!(initial.storage.boxes.is_empty());
        assert_eq!(initial.trainer.player_name, "");
        assert_eq!(initial.trainer.money, 0);
        assert_eq!(initial.trainer.coins, 0);
        assert_eq!(
            initial.trainer.options,
            crystal_core::state::Options::default()
        );
        assert_eq!(initial.progression.pokedex_seen, 0);
        assert_eq!(initial.progression.pokedex_owned, 0);
        assert_eq!(
            initial.progression.badges,
            crystal_core::state::Badges::default()
        );
        assert_eq!(initial.progression.repel_steps_remaining, 0);
        assert!(initial.progression.active_repel_item.is_none());
        assert!(initial.script_events.text_events.is_empty());
        assert!(initial.script_events.map_events.is_empty());
        assert!(initial.script_events.graphics_events.is_empty());
        assert!(initial.script_events.money_events.is_empty());
        assert!(initial.script_events.control_events.is_empty());
        assert!(initial.script_events.script_value.is_none());
        assert!(initial.script_events.variables.is_empty());
        assert!(initial.script_events.named_buffers.is_empty());
        assert!(initial.script_events.phone_numbers.is_empty());
        assert!(initial.script_events.variable_writes.is_empty());
        assert!(initial.script_events.special_phone_calls.is_empty());
        assert!(!initial.script_events.window_open);
        assert!(initial.script_events.menu_coords.is_none());
        assert!(initial.script_events.active_pokemon_picture.is_none());
        assert!(initial.script_events.pending_text_label.is_none());
        assert!(!initial.script_events.reset_requested);
        assert!(!initial.script_events.map_music_requested);
        assert!(!initial.script_events.waiting_for_sound_effect);
        assert!(initial.pending_shop.is_none());
        assert!(shell.last_frame().is_none());
        let party_pokemon = Pokemon::new_for_tests(runtime_species(), 5, Dv::default());
        let pc_pokemon = Pokemon::new_for_tests(runtime_species(), 6, Dv::default());
        shell.session_mut().state.storage.party.pokemon[0] = Some(party_pokemon.clone());
        let mut pc_box = PcBox::new(0);
        pc_box.set_slot(3, Some(pc_pokemon.clone()));
        shell.session_mut().state.storage.pc_boxes.push(pc_box);
        shell.session_mut().state.current_pc_box = 0;
        let party_state =
            crystal_core::state::PartyState::from_storage(&shell.session().state.storage);
        shell.session_mut().state.party = party_state;
        let item = shell.runtime().data.items["BLU_APRICORN"].clone();
        let ball = shell.runtime().data.items["POKE_BALL"].clone();
        shell
            .session_mut()
            .state
            .bag
            .add_item(&item, 2)
            .expect("add item pocket fixture");
        shell
            .session_mut()
            .state
            .bag
            .add_item(&ball, 1)
            .expect("add ball pocket fixture");
        shell.session_mut().state.money = 3000;
        shell.session_mut().state.coins = 12;
        shell.session_mut().state.badges.johto[0] = true;
        shell
            .session_mut()
            .state
            .pokedex
            .record_caught_pokemon(&party_pokemon);
        shell.session_mut().state.repel_steps_remaining = 100;
        shell.session_mut().state.active_repel_item = Some("REPEL".to_string());
        shell.session_mut().state.script_runtime.text_window_open = true;
        shell.session_mut().state.script_runtime.pending_text_label =
            Some("RuntimeText".to_string());
        let inventory = shell.snapshot().expect("inventory snapshot");
        assert_eq!(inventory.phase, RuntimeShellPhase::Text);
        assert_eq!(inventory.trainer.money, 3000);
        assert_eq!(inventory.trainer.coins, 12);
        assert_eq!(inventory.progression.pokedex_seen, 1);
        assert_eq!(inventory.progression.pokedex_owned, 1);
        assert!(inventory.progression.badges.johto[0]);
        assert_eq!(inventory.progression.repel_steps_remaining, 100);
        assert_eq!(
            inventory.progression.active_repel_item.as_deref(),
            Some("REPEL")
        );
        assert!(inventory.script_events.text_window_open);
        assert_eq!(
            inventory.script_events.pending_text_label.as_deref(),
            Some("RuntimeText")
        );
        let ui_text = inventory.ui.text.as_ref().expect("runtime UI text");
        assert_eq!(ui_text.label, "RuntimeText");
        assert_eq!(ui_text.source, RuntimeTextSource::AsmText);
        assert_eq!(ui_text.asm_text.as_deref(), Some("RuntimeText"));
        assert!(ui_text.body.is_none());
        assert_eq!(ui_text.queued_text_events, 0);
        assert!(inventory.ui.text_window_open);
        let mut bad_ui_shell = shell.clone();
        bad_ui_shell
            .session_mut()
            .state
            .script_runtime
            .pending_text_label = Some("MissingRuntimeText".to_string());
        let bad_ui_error = bad_ui_shell
            .snapshot()
            .expect_err("runtime UI text labels are exact");
        let bad_ui_error = format!("{bad_ui_error:#}");
        assert!(
            bad_ui_error.contains("MissingRuntimeText")
                && bad_ui_error.contains("saved script_runtime.pending_text_label"),
            "{bad_ui_error}"
        );
        let runtime_text_event = ScriptTextRuntimeEvent {
            command: "writetext".to_string(),
            kind: ScriptTextRuntimeKind::Write,
            text_label: Some("RuntimeText".to_string()),
            face_player: false,
            closes_text: false,
            source_script: "RuntimeShopScript".to_string(),
            command_index: 0,
        };
        shell
            .session_mut()
            .state
            .script_runtime
            .text_events
            .push(runtime_text_event.clone());
        let drained_text = shell
            .drain_script_event_queue(RuntimeScriptEventQueue::Text)
            .expect("drain text event queue");
        assert_eq!(
            drained_text,
            RuntimeScriptEventDrainResult::Text(vec![runtime_text_event])
        );
        assert!(
            shell
                .snapshot()
                .expect("post text-drain snapshot")
                .script_events
                .text_events
                .is_empty()
        );
        let runtime_audio_event = crystal_core::state::ScriptAudioRuntimeEvent {
            command: "playsound".to_string(),
            kind: crystal_core::state::ScriptAudioRuntimeKind::SoundEffect,
            audio_id: Some("SFX_TACKLE".to_string()),
            fade_frames: None,
            source_script: "RuntimeAudioScript".to_string(),
            command_index: 1,
        };
        shell
            .session_mut()
            .state
            .script_runtime
            .audio_events
            .push(runtime_audio_event.clone());
        let drained_audio = shell.drain_audio_events().expect("drain audio event queue");
        assert_eq!(drained_audio.events, vec![runtime_audio_event]);
        assert!(
            shell
                .snapshot()
                .expect("post audio-drain snapshot")
                .script_events
                .audio_events
                .is_empty()
        );
        let resolved_audio_event = crystal_core::state::ScriptAudioRuntimeEvent {
            command: "playsound".to_string(),
            kind: crystal_core::state::ScriptAudioRuntimeKind::SoundEffect,
            audio_id: Some("SFX_TACKLE".to_string()),
            fade_frames: None,
            source_script: "RuntimeAudioScript".to_string(),
            command_index: 2,
        };
        shell
            .session_mut()
            .state
            .script_runtime
            .audio_events
            .push(resolved_audio_event.clone());
        let resolved_audio = shell
            .drain_resolved_audio_events()
            .expect("drain resolved audio event queue");
        assert_eq!(resolved_audio.events.len(), 1);
        assert_eq!(resolved_audio.events[0].event, resolved_audio_event);
        let RuntimeResolvedAudioPlaybackKind::Play { audio_id, playback } =
            &resolved_audio.events[0].kind
        else {
            panic!("expected resolved SFX playback");
        };
        assert_eq!(audio_id, "SFX_TACKLE");
        assert_eq!(playback.id, "SFX_TACKLE");
        assert_eq!(playback.loop_policy, ModpackAudioLoopPolicy::Once);
        let taken_text_label = shell
            .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)
            .expect("take pending text label");
        assert_eq!(
            taken_text_label,
            RuntimePendingScriptRequest::TextLabel("RuntimeText".to_string())
        );
        assert!(
            shell
                .snapshot()
                .expect("post text-label take snapshot")
                .script_events
                .pending_text_label
                .is_none()
        );
        let text_wait = ScriptTextWait {
            command: "waitbutton".to_string(),
            source_script: "RuntimeScript".to_string(),
            command_index: 2,
        };
        shell.session_mut().state.script_runtime.pending_text_wait = Some(text_wait.clone());
        let advanced_text_wait = shell
            .advance_pending_text_wait()
            .expect("advance pending text wait");
        assert_eq!(advanced_text_wait.wait, text_wait);
        assert!(
            shell
                .snapshot()
                .expect("post text-wait advance snapshot")
                .script_events
                .pending_text_wait
                .is_none()
        );
        shell.session_mut().state.script_runtime.text_window_open = true;
        shell.session_mut().state.script_runtime.pending_text_label =
            Some("RuntimeText".to_string());
        let jumptext_wait = ScriptTextWait {
            command: "jumptext".to_string(),
            source_script: "RuntimeScript".to_string(),
            command_index: 4,
        };
        shell.session_mut().state.script_runtime.pending_text_wait = Some(jumptext_wait.clone());
        let advanced_jumptext_wait = shell
            .advance_pending_text_wait()
            .expect("advance jumptext wait");
        assert_eq!(advanced_jumptext_wait.wait, jumptext_wait);
        let jumptext_snapshot = shell.snapshot().expect("post jumptext wait snapshot");
        assert!(!jumptext_snapshot.script_events.text_window_open);
        assert!(jumptext_snapshot.script_events.pending_text_label.is_none());
        assert!(jumptext_snapshot.script_events.pending_text_wait.is_none());
        assert!(
            !shell
                .snapshot()
                .expect("post text-window-close snapshot")
                .script_events
                .text_window_open
        );
        let runtime_delay = ScriptRuntimeDelay {
            command: "pause".to_string(),
            frames: 15,
            source_script: "RuntimeScript".to_string(),
            command_index: 1,
        };
        shell
            .session_mut()
            .state
            .script_runtime
            .pending_delays
            .push(runtime_delay.clone());
        let drained_delays = shell
            .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingDelay)
            .expect("drain pending delays");
        assert_eq!(
            drained_delays,
            RuntimeScriptRuntimeQueueDrainResult::PendingDelay(vec![runtime_delay])
        );
        assert!(
            shell
                .snapshot()
                .expect("post delay-drain snapshot")
                .script_events
                .pending_delays
                .is_empty()
        );
        shell.session_mut().state.script_runtime.reset_requested = true;
        assert!(
            shell
                .snapshot()
                .expect("pre reset-consume snapshot")
                .script_events
                .reset_requested
        );
        let consumed_reset = shell
            .consume_script_runtime_flag(RuntimeScriptRuntimeFlag::ResetRequested)
            .expect("consume reset request");
        assert_eq!(
            consumed_reset,
            RuntimeScriptRuntimeFlagValue::ResetRequested
        );
        assert!(
            !shell
                .snapshot()
                .expect("post reset-consume snapshot")
                .script_events
                .reset_requested
        );
        assert!(
            shell
                .snapshot()
                .expect("post variable-write-drain snapshot")
                .script_events
                .variable_writes
                .is_empty()
        );
        shell.session_mut().state.script_runtime.script_value = Some("12".to_string());
        assert_eq!(
            shell
                .snapshot()
                .expect("script value snapshot")
                .script_events
                .script_value
                .as_deref(),
            Some("12")
        );
        let taken_script_value = shell
            .take_script_runtime_memory_value(RuntimeScriptRuntimeMemoryValue::ScriptValue)
            .expect("take script value");
        assert_eq!(
            taken_script_value,
            RuntimeScriptRuntimeMemoryValueTaken::ScriptValue("12".to_string())
        );
        shell
            .session_mut()
            .state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), "9".to_string());
        let removed_variable = shell
            .remove_script_runtime_memory_entry(
                RuntimeScriptRuntimeMemoryEntry::Variable,
                "VAR_BLUECARDBALANCE",
            )
            .expect("remove script variable");
        assert_eq!(
            removed_variable,
            RuntimeScriptRuntimeMemoryEntryRemoved::Variable {
                key: "VAR_BLUECARDBALANCE".to_string(),
                value: "9".to_string()
            }
        );
        assert!(
            shell
                .snapshot()
                .expect("post variable-remove snapshot")
                .script_events
                .variables
                .is_empty()
        );
        assert_eq!(inventory.party.slots.len(), 1);
        assert_eq!(inventory.party.slots[0].index, 0);
        assert_eq!(inventory.party.slots[0].pokemon, party_pokemon);
        assert!(!inventory.party.slots[0].is_active_battle_pokemon);
        assert_eq!(inventory.storage.party_count, 1);
        assert_eq!(inventory.storage.current_pc_box, 0);
        assert_eq!(inventory.storage.boxes.len(), 1);
        assert_eq!(inventory.storage.boxes[0].index, 0);
        assert_eq!(inventory.storage.boxes[0].name, "BOX 01");
        assert_eq!(inventory.storage.boxes[0].count, 1);
        assert_eq!(inventory.storage.boxes[0].slots.len(), 1);
        assert_eq!(inventory.storage.boxes[0].slots[0].index, 3);
        assert_eq!(inventory.storage.boxes[0].slots[0].pokemon, pc_pokemon);
        assert_eq!(
            inventory.bag.items,
            vec![RuntimeBagItemSnapshot {
                item_id: "BLU_APRICORN".to_string(),
                quantity: 2,
            }]
        );
        assert_eq!(
            inventory.bag.balls,
            vec![RuntimeBagItemSnapshot {
                item_id: "POKE_BALL".to_string(),
                quantity: 1,
            }]
        );

        let frame = shell.tick([GameButton::Right]).expect("tick").clone();
        assert_eq!(frame.input_mask & B_PAD_RIGHT, B_PAD_RIGHT);
        assert_eq!(shell.last_frame(), Some(&frame));
        let after_tick = shell.snapshot().expect("after tick snapshot");
        assert_eq!(after_tick.overworld.frame, frame.snapshot.frame);
        assert_eq!(after_tick.state_checksum, frame.state_checksum);
        let framed_sequence = shell
            .runtime_command_sequence
            .checked_add(1)
            .expect("runtime command sequence");
        let framed_command = shell
            .runtime_command_frame(
                1,
                framed_sequence,
                RuntimeMutationCommand::ApplyOverworldInput(RuntimeOverworldInputCommand {
                    buttons: vec![GameButton::Down],
                    divider_trace: RuntimeDividerTrace::new([]),
                }),
            )
            .expect("runtime command frame");
        shell
            .require_runtime_command_expected_state(&framed_command)
            .expect("expected shell state");
        let framed_outcome = shell
            .apply_runtime_command_frame(&framed_command)
            .expect("apply runtime command frame");
        let RuntimeMutationResult::OverworldInputApplied(_) = framed_outcome.result else {
            panic!("runtime command frame returned non-overworld result");
        };
        assert_eq!(
            shell
                .last_frame()
                .expect("shell frame from framed command")
                .input_mask
                & B_PAD_DOWN,
            B_PAD_DOWN
        );
        let result_frame = shell
            .runtime_mutation_result_frame(framed_command, &framed_outcome)
            .expect("runtime result frame");
        assert_eq!(result_frame.request().sequence(), framed_sequence);
        let after_frame = shell.snapshot().expect("after frame snapshot");
        assert_eq!(after_frame.state_checksum, framed_outcome.state_checksum);

        let save_path = root.join("saves/shell-slot.crystalsave");
        shell.save(&save_path).expect("save shell");
        let resumed_runtime = shell.runtime().clone();
        let resumed = RuntimeGameShell::resume_from_save(asset_root, resumed_runtime, &save_path)
            .expect("resume shell");
        assert_eq!(
            resumed.snapshot().expect("resumed snapshot").state_checksum,
            after_frame.state_checksum
        );
        shell.session_mut().state.script_runtime.text_window_open = true;
        shell.session_mut().state.script_runtime.active_menu = Some("RuntimeMenu".to_string());
        shell.session_mut().state.script_runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "RuntimeScript".to_string(),
            command_index: 2,
        });
        let modal_error = shell
            .apply_runtime_mutation_command(RuntimeMutationCommand::CloseRuntimeWindow)
            .expect_err("invalid modal state must block retained runtime command before mutation")
            .to_string();
        assert!(
            modal_error
                .contains("cannot apply runtime mutation command from invalid script modal state"),
            "{modal_error}"
        );
        assert_eq!(
            shell.session().state.script_runtime.active_menu.as_deref(),
            Some("RuntimeMenu")
        );
        shell.session_mut().state.script_runtime.text_window_open = false;
        shell.session_mut().state.script_runtime.active_menu = None;
        shell.session_mut().state.script_runtime.pending_yes_no = None;
        shell.session_mut().state.script_runtime.pending_yes_no = Some(ScriptYesNoPrompt {
            source_script: "RuntimeScript".to_string(),
            command_index: 2,
        });
        let modal_error = shell
            .open_vertical_menu("RuntimeMap", "RuntimeMenu", "RuntimeScript", 3, 4)
            .expect_err("invalid modal state must block menu open before mutation")
            .to_string();
        assert!(
            modal_error.contains("cannot open vertical menu from invalid script modal state"),
            "{modal_error}"
        );
        assert!(shell.session().state.script_runtime.active_menu.is_none());
        shell.session_mut().state.script_runtime.pending_yes_no = None;
        let opened_menu = shell
            .open_vertical_menu("RuntimeMap", "RuntimeScript:4", "RuntimeScript", 3, 4)
            .expect("open vertical menu");
        assert_eq!(opened_menu.menu_id, "RuntimeMenu");
        assert_eq!(
            shell.session().state.script_runtime.active_menu.as_deref(),
            Some("RuntimeMenu")
        );
        assert!(shell.session().state.script_runtime.window_open);
        shell.session_mut().state.script_runtime.menu_coords = Some([0, 0, 10, 8]);
        assert_eq!(
            shell
                .snapshot()
                .expect("script event menu snapshot")
                .script_events
                .active_menu
                .as_deref(),
            Some("RuntimeMenu")
        );
        let menu_snapshot = shell.snapshot().expect("menu snapshot").menu.expect("menu");
        assert_eq!(menu_snapshot.menu_id, "RuntimeMenu");
        assert_eq!(
            menu_snapshot.source,
            RuntimeMenuSource::ScriptDefinition {
                map_name: "RuntimeMap".to_string()
            }
        );
        assert!(menu_snapshot.window_open);
        assert_eq!(menu_snapshot.coords, Some([0, 0, 10, 8]));
        assert_eq!(menu_snapshot.layout.declared_coords, Some([0, 0, 10, 8]));
        assert_eq!(menu_snapshot.layout.data_commands.len(), 2);
        assert_eq!(menu_snapshot.layout.data_commands[0].command, "db");
        assert_eq!(
            menu_snapshot.layout.data_commands[0].args,
            vec!["2".to_string(), "1".to_string(), "0".to_string()]
        );
        assert_eq!(menu_snapshot.layout.data_commands[1].command, "dw");
        assert_eq!(
            menu_snapshot.layout.data_commands[1].args,
            vec!["RuntimeMenuItems".to_string()]
        );
        assert_eq!(menu_snapshot.layout.vertical_menus.len(), 1);
        assert_eq!(
            menu_snapshot.layout.vertical_menus[0].options,
            vec!["First".to_string(), "Second".to_string()]
        );
        assert_eq!(
            menu_snapshot.layout.vertical_menus[0].data_label,
            Some("RuntimeMenuItems".to_string())
        );
        assert_eq!(
            menu_snapshot
                .definition
                .expect("runtime menu definition")
                .commands[0]
                .command,
            "menu_coords"
        );
        assert_eq!(
            shell.snapshot().expect("menu snapshot").phase,
            RuntimeShellPhase::Menu
        );
        let selection = shell
            .select_vertical_menu_option("RuntimeMenu", "RuntimeScript", 4, 1, "Second")
            .expect("select vertical menu option");
        assert_eq!(selection.option, "Second");
        assert_eq!(selection.option_index, 1);
        assert_eq!(selection.script_value, "2");
        assert_eq!(
            shell.session().state.script_runtime.script_value.as_deref(),
            Some("2")
        );
        assert_eq!(
            shell
                .session()
                .state
                .script_runtime
                .memory
                .get("wScriptVar")
                .map(String::as_str),
            Some("2")
        );
        let ui_snapshot = shell.snapshot().expect("ui snapshot").ui;
        assert_eq!(ui_snapshot.elevators.len(), 1);
        assert_eq!(ui_snapshot.elevators[0].map_name, "RuntimeMap");
        assert_eq!(ui_snapshot.elevators[0].data_label, "RuntimeElevatorData");
        assert_eq!(ui_snapshot.elevators[0].floors.len(), 1);
        assert_eq!(ui_snapshot.elevators[0].floors[0].floor, "FLOOR_2F");
        assert_eq!(ui_snapshot.elevators[0].floors[0].warp, 4);
        assert_eq!(ui_snapshot.elevators[0].floors[0].target_map, "RuntimeMap");
        let elevator = shell
            .select_elevator_floor(
                "RuntimeMap",
                "RuntimeElevatorData",
                "RuntimeScript",
                5,
                0,
                "FLOOR_2F",
                4,
                "RuntimeMap",
            )
            .expect("select elevator floor");
        assert_eq!(elevator.script_value, "1");
        assert_eq!(elevator.destination_tile.x, 1);
        assert_eq!(elevator.destination_tile.y, 0);
        let pending_warp = shell
            .session()
            .state
            .script_runtime
            .pending_script_warp
            .as_ref()
            .expect("pending elevator warp");
        assert_eq!(pending_warp.target_map, "RuntimeMap");
        assert_eq!(pending_warp.tile.x, 1);
        assert_eq!(pending_warp.tile.y, 0);
        assert_eq!(
            shell
                .session()
                .state
                .script_runtime
                .memory
                .get("wScriptVar")
                .map(String::as_str),
            Some("1")
        );
        shell.session_mut().state.script_runtime.pending_shop = Some(ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "MART_RUNTIME".to_string(),
            inventory: vec!["POKE_BALL".to_string()],
            source_script: "RuntimeShopScript".to_string(),
            command_index: 0,
        });
        let modal_error = shell
            .close_active_menu()
            .expect_err("invalid modal state must block menu close before mutation")
            .to_string();
        assert!(
            modal_error.contains("cannot close active menu from invalid script modal state"),
            "{modal_error}"
        );
        assert_eq!(
            shell.session().state.script_runtime.active_menu.as_deref(),
            Some("RuntimeMenu")
        );
        shell.session_mut().state.script_runtime.pending_shop = None;
        let closed_menu = shell.close_active_menu().expect("close active menu");
        assert_eq!(closed_menu.menu, "RuntimeMenu");
        let cleared_menu_coords = shell.clear_menu_coords().expect("clear menu coords");
        assert_eq!(cleared_menu_coords.coords, [0, 0, 10, 8]);
        shell.close_runtime_window().expect("close runtime window");
        assert_eq!(
            shell.snapshot().expect("post-menu snapshot").phase,
            RuntimeShellPhase::Overworld
        );
        assert!(
            !shell
                .snapshot()
                .expect("post-menu presentation snapshot")
                .script_events
                .window_open
        );
        assert!(
            shell
                .snapshot()
                .expect("post-menu coords snapshot")
                .script_events
                .menu_coords
                .is_none()
        );
        shell
            .session_mut()
            .state
            .script_runtime
            .active_pokemon_picture = Some("CHIKORITA".to_string());
        assert_eq!(
            shell
                .snapshot()
                .expect("pokemon picture snapshot")
                .script_events
                .active_pokemon_picture
                .as_deref(),
            Some("CHIKORITA")
        );
        let closed_picture = shell
            .close_active_pokemon_picture()
            .expect("close active pokemon picture");
        assert_eq!(closed_picture.species_id, "CHIKORITA");
        assert!(
            shell
                .snapshot()
                .expect("post-picture snapshot")
                .script_events
                .active_pokemon_picture
                .is_none()
        );
        shell.session_mut().state.script_runtime.pending_shop = Some(ScriptShopRequest {
            mart_type: "MARTTYPE_STANDARD".to_string(),
            mart_id: "MART_RUNTIME".to_string(),
            inventory: vec!["POKE_BALL".to_string()],
            source_script: "RuntimeShopScript".to_string(),
            command_index: 0,
        });
        assert_eq!(
            shell.snapshot().expect("shop snapshot").phase,
            RuntimeShellPhase::Shop
        );
        let closed_shop = shell.close_script_shop().expect("close script shop");
        assert_eq!(closed_shop.shop.mart_id, "MART_RUNTIME");
        assert_eq!(
            shell.snapshot().expect("post-shop snapshot").phase,
            RuntimeShellPhase::Overworld
        );
        let enemy = Pokemon::new_for_tests(runtime_species(), 14, Dv::default());
        shell.session_mut().state.battle = BattleMemory::Wild {
            roaming_slot: None,
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "RuntimeMap".to_string(),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        };
        shell.session_mut().state.battle_active_party_index = Some(0);
        shell.session_mut().state.battle_active_enemy_party_index = Some(0);
        shell.session_mut().state.battle_escape_attempts = 2;
        shell
            .session_mut()
            .state
            .battle_player_stat_drop_guard_turns = 3;
        let battle_snapshot = shell
            .snapshot()
            .expect("battle snapshot")
            .battle
            .expect("active battle");
        assert_eq!(battle_snapshot.phase(), RuntimeShellPhase::WildBattle);
        assert_eq!(
            battle_snapshot.kind,
            RuntimeBattleKind::Wild {
                map_name: "RuntimeMap".to_string(),
                battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            }
        );
        assert_eq!(battle_snapshot.battle_music, "MUSIC_JOHTO_WILD_BATTLE");
        assert_eq!(battle_snapshot.enemy_pokemon, enemy);
        assert_eq!(battle_snapshot.active_player_party_index, Some(0));
        assert_eq!(battle_snapshot.active_enemy_party_index, Some(0));
        assert!(battle_snapshot.rewarded_enemy_party_indices.is_empty());
        assert_eq!(battle_snapshot.escape_attempts, 2);
        assert_eq!(battle_snapshot.player_stat_drop_guard_turns, 3);
        let _ = std::fs::remove_dir_all(root);
    }
