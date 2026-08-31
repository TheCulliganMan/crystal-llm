#[test]
fn all_unown_puzzles_complete_the_authored_chamber_scripts() {
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repository_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load Rust-built game pack");

    let cases = [
        (
            "RuinsOfAlphKabutoChamber",
            "RuinsOfAlphKabutoChamberPuzzle",
            "UNOWNPUZZLE_KABUTO",
            "KABUTO",
            "EVENT_SOLVED_KABUTO_PUZZLE",
            "ENGINE_UNLOCKED_UNOWNS_A_TO_K",
        ),
        (
            "RuinsOfAlphOmanyteChamber",
            "RuinsOfAlphOmanyteChamberPuzzle",
            "UNOWNPUZZLE_OMANYTE",
            "OMANYTE",
            "EVENT_SOLVED_OMANYTE_PUZZLE",
            "ENGINE_UNLOCKED_UNOWNS_L_TO_R",
        ),
        (
            "RuinsOfAlphAerodactylChamber",
            "RuinsOfAlphAerodactylChamberPuzzle",
            "UNOWNPUZZLE_AERODACTYL",
            "AERODACTYL",
            "EVENT_SOLVED_AERODACTYL_PUZZLE",
            "ENGINE_UNLOCKED_UNOWNS_S_TO_W",
        ),
        (
            "RuinsOfAlphHoOhChamber",
            "RuinsOfAlphHoOhChamberPuzzle",
            "UNOWNPUZZLE_HO_OH",
            "HOOH",
            "EVENT_SOLVED_HO_OH_PUZZLE",
            "ENGINE_UNLOCKED_UNOWNS_X_TO_Z",
        ),
    ];

    for (map_name, script, script_value, puzzle_id, solved_event, unlocked_flag) in cases {
        let mut shell = RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime.clone(),
            0,
            map_name,
            4,
            4,
        )
        .expect("start at puzzle");
        let opened = shell
            .run_compiled_script_until_boundary(
                RuntimeCompiledScriptCursor {
                    origin_map_name: map_name.to_string(),
                    source_script: script.to_string(),
                    command_index: 0,
                },
                16,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("open authored puzzle script");
        assert_eq!(
            opened.boundary,
            Some(RuntimeCompiledScriptBoundary::PendingMapRefresh(
                ScriptMapRefreshRequest {
                    command: "reanchormap".to_string(),
                    map_setup: None,
                    source_script: script.to_string(),
                    command_index: 0,
                }
            )),
            "{map_name} must begin with its authored map refresh"
        );
        shell
            .take_pending_script_request(RuntimePendingScriptRequestKind::MapRefresh)
            .expect("take puzzle map refresh");

        // Start one move from completion, then exercise the same pickup/place
        // special calls used by the visible puzzle controls.
        let target_with_first_piece_misplaced =
            "1,0,0,0,0,0;0,0,2,3,4,0;0,5,6,7,8,0;0,9,10,11,12,0;0,13,14,15,16,0;0,0,0,0,0,0";
        let scripts = &mut shell.session_mut().state.script_runtime;
        scripts.variables.insert(
            format!("unown_layout_{puzzle_id}"),
            target_with_first_piece_misplaced.to_string(),
        );
        scripts
            .variables
            .insert("_value".to_string(), script_value.to_string());
        scripts.script_value = Some(script_value.to_string());
        scripts
            .variables
            .insert("unown_action".to_string(), "pickup".to_string());
        scripts
            .variables
            .insert("unown_x".to_string(), "0".to_string());
        scripts
            .variables
            .insert("unown_y".to_string(), "0".to_string());

        // Pick up piece one and place it into its missing slot. This confirms
        // both action paths and that the solved layout drives `iftrue`.
        let pickup = shell
            .apply_declared_special_routine("UnownPuzzle")
            .expect("pick up final puzzle piece");
        assert!(matches!(
            pickup.outcome.effect,
            SpecialRoutineEffect::UnownPuzzle {
                puzzle_id: ref id,
                holding_piece: Some(1),
                solved: false,
                ..
            } if id == puzzle_id
        ));
        assert_eq!(
            shell.session().state.script_runtime.script_value.as_deref(),
            Some("0"),
            "{map_name} remains unsolved while the final piece is held"
        );

        let scripts = &mut shell.session_mut().state.script_runtime;
        scripts
            .variables
            .insert("_value".to_string(), script_value.to_string());
        scripts.script_value = Some(script_value.to_string());
        scripts
            .variables
            .insert("unown_action".to_string(), "place".to_string());
        scripts
            .variables
            .insert("unown_x".to_string(), "1".to_string());
        scripts
            .variables
            .insert("unown_y".to_string(), "1".to_string());
        let placed = shell
            .apply_declared_special_routine("UnownPuzzle")
            .expect("place final puzzle piece");
        assert!(matches!(
            placed.outcome.effect,
            SpecialRoutineEffect::UnownPuzzle {
                puzzle_id: ref id,
                holding_piece: None,
                solved: true,
                ..
            } if id == puzzle_id
        ));
        assert_eq!(
            shell.session().state.script_runtime.script_value.as_deref(),
            Some("1"),
            "{map_name} solved placement must drive iftrue"
        );

        if shell.session().state.script_runtime.script_ended.is_some() {
            shell
                .take_script_end_state()
                .expect("consume completed puzzle interaction");
        }
        shell.session_mut().state.script_runtime.active_menu = None;
        let completed = shell
            .run_compiled_script_until_boundary(
                RuntimeCompiledScriptCursor {
                    origin_map_name: map_name.to_string(),
                    source_script: script.to_string(),
                    command_index: 4,
                },
                32,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("resume puzzle completion script");
        assert!(matches!(
            completed.boundary,
            Some(RuntimeCompiledScriptBoundary::Earthquake(_))
        ));
        shell
            .session_mut()
            .state
            .script_runtime
            .pending_earthquakes
            .clear();
        let emote = shell
            .run_compiled_script_until_boundary(
                completed
                    .next_cursor
                    .expect("resume after first earthquake"),
                16,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("run puzzle doorway opening");
        assert!(matches!(
            emote.boundary,
            Some(RuntimeCompiledScriptBoundary::Emote(_))
        ));
        shell
            .session_mut()
            .state
            .script_runtime
            .pending_emotes
            .clear();
        let opened_door = shell
            .run_compiled_script_until_boundary(
                emote.next_cursor.expect("resume after puzzle emote"),
                16,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )
            .expect("open puzzle doorway");
        assert!(matches!(
            opened_door.boundary,
            Some(RuntimeCompiledScriptBoundary::PendingMapRefresh(_))
        ));
        assert_eq!(
            shell.session().overworld.map.metatile_at(1, 1),
            Some(0x18),
            "{map_name} must open the left half of its inner-chamber doorway"
        );
        assert_eq!(
            shell.session().overworld.map.metatile_at(2, 1),
            Some(0x19),
            "{map_name} must open the right half of its inner-chamber doorway"
        );
        let inner_chamber_warps = runtime
            .data()
            .map_module(map_name)
            .expect("compiled puzzle chamber")
            .events
            .warps
            .iter()
            .filter(|warp| {
                matches!((warp.x, warp.y), (3, 3) | (4, 3))
                    && warp.target_map_constant == "RUINS_OF_ALPH_INNER_CHAMBER"
            })
            .count();
        assert_eq!(
            inner_chamber_warps, 2,
            "{map_name} opened door must link to the inner chamber"
        );
        assert_eq!(
            runtime
                .data()
                .map_name_for_constant("RUINS_OF_ALPH_INNER_CHAMBER")
                .expect("resolve inner chamber map constant"),
            "RuinsOfAlphInnerChamber",
            "the chamber door target must resolve to the loaded inner-chamber map"
        );
        assert_eq!(
            shell.session().state.flags.is_event_flag_set(solved_event),
            Ok(true),
            "{map_name} completion event"
        );
        assert_eq!(
            shell
                .session()
                .state
                .flags
                .is_engine_flag_set(unlocked_flag),
            Ok(true),
            "{map_name} Unown letter set"
        );
    }
}
