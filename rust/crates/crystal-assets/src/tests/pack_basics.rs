use super::*;
use crystal_core::map::MapConnection;
use crystal_core::models::{
    BaseStats, Item, MAX_BOX_MONS, PcBox, ability, egg_group, growth_rate, item_pocket,
    pokemon_type,
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

fn presentation_span() -> RuntimePresentationSourceSpan {
    RuntimePresentationSourceSpan {
        file: "engine/menus/title.asm".to_string(),
        start_line: 1,
        end_line: 1,
    }
}

fn complete_runtime_title_program() -> RuntimePresentationProgram {
    RuntimePresentationProgram {
        schema_version: 1,
        entrypoints: [
            "boot",
            "intro",
            "title",
            "main_menu",
            "continue",
            "new_game",
            "delete_save",
            "reset_clock",
        ]
        .into_iter()
        .map(|entrypoint| (entrypoint.to_string(), "Program".to_string()))
        .collect(),
        blocks: [(
            "Program".to_string(),
            RuntimePresentationBlock {
                source_span: presentation_span(),
                operations: vec![RuntimePresentationOperation {
                    op: "return".to_string(),
                    source_span: presentation_span(),
                    fields: Default::default(),
                }],
            },
        )]
        .into_iter()
        .collect(),
        audio: vec![RuntimePresentationAudio {
            id: "MUSIC_TITLE".to_string(),
            kind: "music".to_string(),
            source_span: presentation_span(),
        }],
        ..RuntimePresentationProgram::default()
    }
}

#[test]
fn runtime_presentation_interpreter_follows_exported_jumps() {
    let program = RuntimePresentationProgram {
        schema_version: 1,
        entrypoints: [("title".to_string(), "TitleStart".to_string())]
            .into_iter()
            .collect(),
        blocks: [
            (
                "TitleStart".to_string(),
                RuntimePresentationBlock {
                    source_span: presentation_span(),
                    operations: vec![RuntimePresentationOperation {
                        op: "jump".to_string(),
                        source_span: presentation_span(),
                        fields: [("target".to_string(), serde_json::json!("TitleLoop"))]
                            .into_iter()
                            .collect(),
                    }],
                },
            ),
            (
                "TitleLoop".to_string(),
                RuntimePresentationBlock {
                    source_span: presentation_span(),
                    operations: vec![RuntimePresentationOperation {
                        op: "wait_frame".to_string(),
                        source_span: presentation_span(),
                        fields: Default::default(),
                    }],
                },
            ),
        ]
        .into_iter()
        .collect(),
        ..RuntimePresentationProgram::default()
    };

    let mut interpreter = RuntimePresentationInterpreter::new(&program, "title")
        .expect("title entrypoint should resolve");
    assert_eq!(interpreter.block, "TitleStart");
    assert_eq!(
        interpreter.step(&program).expect("jump should execute"),
        RuntimePresentationStep::Jump {
            from: "TitleStart".to_string(),
            to: "TitleLoop".to_string(),
        }
    );
    assert_eq!(interpreter.block, "TitleLoop");
    assert_eq!(interpreter.operation_index, 0);
    assert!(matches!(
        interpreter.step(&program).expect("operation should execute"),
        RuntimePresentationStep::Operation(operation) if operation.op == "wait_frame"
    ));
    assert_eq!(
        interpreter.step(&program).expect("block should finish"),
        RuntimePresentationStep::BlockComplete {
            block: "TitleLoop".to_string(),
        }
    );
}

#[test]
fn runtime_presentation_interpreter_rejects_missing_targets() {
    let mut program = RuntimePresentationProgram {
        entrypoints: [("title".to_string(), "Missing".to_string())]
            .into_iter()
            .collect(),
        ..RuntimePresentationProgram::default()
    };
    let error = RuntimePresentationInterpreter::new(&program, "title")
        .expect_err("missing entrypoint block must fail");
    assert!(error.to_string().contains("targets missing block Missing"));

    program.blocks.insert(
        "Present".to_string(),
        RuntimePresentationBlock {
            source_span: presentation_span(),
            operations: Vec::new(),
        },
    );
    program
        .entrypoints
        .insert("title".to_string(), "Present".to_string());
    let mut interpreter = RuntimePresentationInterpreter::new(&program, "title")
        .expect("present entrypoint block should resolve");
    let error = interpreter
        .jump(&program, "Missing")
        .expect_err("missing jump target must fail");
    assert!(error.to_string().contains("targets missing block Missing"));
}

#[test]
fn runtime_subprogram_interpreter_jumps_to_exported_source_labels() {
    let phase = RuntimePresentationPhase {
        id: "title_screen".to_string(),
        source_span: presentation_span(),
        labels: [("TitleScreenMain".to_string(), 1)].into_iter().collect(),
        operations: vec![
            RuntimePresentationOperation {
                op: "return".to_string(),
                source_span: presentation_span(),
                fields: Default::default(),
            },
            RuntimePresentationOperation {
                op: "sample_input".to_string(),
                source_span: presentation_span(),
                fields: Default::default(),
            },
        ],
    };
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "start_title_screen".to_string(),
            phases: vec![phase],
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };
    let mut interpreter = RuntimePresentationSubprogramInterpreter::new(
        &program,
        "start_title_screen",
        "title_screen",
    )
    .expect("title subprogram should resolve");

    interpreter
        .jump_to_label(&program, "TitleScreenMain")
        .expect("source label should resolve");
    assert_eq!(interpreter.operation_index, 1);
    assert_eq!(
        interpreter.current_label.as_deref(),
        Some("TitleScreenMain")
    );
    assert_eq!(
        interpreter
            .step(&program)
            .expect("source operation should execute")
            .expect("source operation should exist")
            .op,
        "sample_input"
    );
    assert!(
        interpreter
            .jump_to_label(&program, "Missing")
            .expect_err("missing source label must fail")
            .to_string()
            .contains("label Missing is missing")
    );
}

#[test]
fn runtime_phase_machine_executes_exported_title_branches_and_memory() {
    let operation = |value: serde_json::Value| {
        serde_json::from_value::<RuntimePresentationOperation>(value)
            .expect("typed presentation operation")
    };
    let span = serde_json::to_value(presentation_span()).expect("source span");
    let operations = vec![
        operation(serde_json::json!({
            "op": "decrement_memory_word_unless_zero",
            "target": "wTitleScreenTimer",
            "zero_target": ".end@TitleScreenMain",
            "source_span": span,
        })),
        operation(serde_json::json!({
            "op": "sample_input",
            "result": "hJoyDown",
            "source_span": span,
        })),
        operation(serde_json::json!({
            "op": "input_chord_branch",
            "sample": "hJoyDown",
            "mask": 9,
            "predicate": "masked_nonzero",
            "target": ".incave@TitleScreenMain",
            "source_span": span,
        })),
        operation(serde_json::json!({ "op": "return", "source_span": span })),
        operation(serde_json::json!({
            "op": "select_title_option",
            "target": "wTitleScreenSelectedOption",
            "options": [{ "source": ".incave@TitleScreenMain", "value": 0 }],
            "source_span": span,
        })),
        operation(serde_json::json!({
            "op": "set_memory_bit",
            "target": "wJumptableIndex",
            "bit": 7,
            "source_span": span,
        })),
        operation(serde_json::json!({ "op": "return", "source_span": span })),
        operation(serde_json::json!({
            "op": "fade_audio",
            "audio": "MUSIC_NONE",
            "frames": 64,
            "fade_register": { "target": "wMusicFade", "value": 8 },
            "source_span": span,
        })),
        operation(serde_json::json!({ "op": "return", "source_span": span })),
    ];
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "start_title_screen".to_string(),
            phases: vec![RuntimePresentationPhase {
                id: "title_screen".to_string(),
                source_span: presentation_span(),
                labels: [
                    ("TitleScreenMain".to_string(), 0),
                    (".incave@TitleScreenMain".to_string(), 4),
                    (".end@TitleScreenMain".to_string(), 7),
                ]
                .into_iter()
                .collect(),
                operations,
            }],
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };
    let mut machine =
        RuntimePresentationPhaseMachine::new(&program, "start_title_screen", "title_screen")
            .expect("title phase machine");
    assert!(
        machine
            .run_from_label(&program, "TitleScreenMain", 0)
            .expect_err("uninitialized source memory must fail closed")
            .to_string()
            .contains("wTitleScreenTimer was read before initialization")
    );
    machine.memory.insert("wTitleScreenTimer".to_string(), 2);
    machine.memory.insert("wJumptableIndex".to_string(), 0);

    machine
        .run_from_label(&program, "TitleScreenMain", 0x01)
        .expect("A input branch should execute");
    assert_eq!(machine.memory["wTitleScreenTimer"], 1);
    assert_eq!(machine.memory["wTitleScreenSelectedOption"], 0);
    assert_eq!(machine.memory["wJumptableIndex"], 0x80);

    machine.memory.insert("wTitleScreenTimer".to_string(), 0);
    let fade = machine
        .run_from_label(&program, "TitleScreenMain", 0)
        .expect("zero timer branch should execute");
    assert_eq!(machine.memory["wMusicFade"], 64);
    assert!(
        fade.effects
            .iter()
            .any(|operation| operation.op == "fade_audio")
    );
}

#[test]
fn runtime_phase_machine_rejects_unimplemented_source_operations() {
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "start_title_screen".to_string(),
            phases: vec![RuntimePresentationPhase {
                id: "title_screen".to_string(),
                source_span: presentation_span(),
                labels: [("TitleScreenMain".to_string(), 0)].into_iter().collect(),
                operations: vec![RuntimePresentationOperation {
                    op: "invented_source_operation".to_string(),
                    source_span: presentation_span(),
                    fields: Default::default(),
                }],
            }],
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };
    let mut machine =
        RuntimePresentationPhaseMachine::new(&program, "start_title_screen", "title_screen")
            .expect("title phase machine");

    assert!(
        machine
            .run_from_label(&program, "TitleScreenMain", 0)
            .expect_err("unimplemented source operation must fail closed")
            .to_string()
            .contains("cannot execute source operation invented_source_operation")
    );
}

#[test]
fn runtime_phase_machine_executes_title_crystal_oam_motion() {
    let operation = |value: serde_json::Value| {
        serde_json::from_value::<RuntimePresentationOperation>(value)
            .expect("typed presentation operation")
    };
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "start_title_screen".to_string(),
            phases: vec![RuntimePresentationPhase {
                id: "title_screen".to_string(),
                source_span: presentation_span(),
                labels: [("TitleScreenEntrance".to_string(), 0)]
                    .into_iter()
                    .collect(),
                operations: vec![
                    operation(serde_json::json!({
                        "op": "animate_title_crystal",
                        "target": "wShadowOAMSprite00YCoord",
                        "stop_at": 22,
                        "y_delta": 2,
                        "source_span": presentation_span(),
                    })),
                    operation(serde_json::json!({
                        "op": "return",
                        "source_span": presentation_span(),
                    })),
                ],
            }],
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };
    let mut machine =
        RuntimePresentationPhaseMachine::new(&program, "start_title_screen", "title_screen")
            .expect("title phase machine");
    machine
        .memory
        .insert("wShadowOAMSprite00YCoord".to_string(), 222);

    for _ in 0..28 {
        machine
            .run_from_label(&program, "TitleScreenEntrance", 0)
            .expect("title crystal animation step");
    }
    assert_eq!(machine.memory["wShadowOAMSprite00YCoord"], 22);
    machine
        .run_from_label(&program, "TitleScreenEntrance", 0)
        .expect("title crystal stop step");
    assert_eq!(machine.memory["wShadowOAMSprite00YCoord"], 22);
}

#[test]
fn title_presentation_parameters_are_read_from_certified_operations() {
    let operation =
        |op: &str, target: &str, field: &str, value: u16| RuntimePresentationOperation {
            op: op.to_string(),
            source_span: presentation_span(),
            fields: [
                ("target".to_string(), serde_json::json!(target)),
                (field.to_string(), serde_json::json!(value)),
            ]
            .into_iter()
            .collect(),
        };
    let input_mask =
        |target: &str, mask: u8| operation("input_chord_branch", target, "mask", u16::from(mask));
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "start_title_screen".to_string(),
            source_entry: ".TitleScreen".to_string(),
            accepted_call_forms: vec!["farcall".to_string()],
            phases: vec![RuntimePresentationPhase {
                id: "title_screen".to_string(),
                source_span: presentation_span(),
                labels: [
                    (".delete_save_data@TitleScreenMain".to_string(), 4),
                    (".check_start@TitleScreenMain".to_string(), 5),
                    (".reset_clock@TitleScreenMain".to_string(), 6),
                    (".incave@TitleScreenMain".to_string(), 7),
                ]
                .into_iter()
                .collect(),
                operations: vec![
                    RuntimePresentationOperation {
                        op: "initialize_title_crystal_oam".to_string(),
                        source_span: presentation_span(),
                        fields: [
                            (
                                "target".to_string(),
                                serde_json::json!("wShadowOAMSprite00"),
                            ),
                            ("initial_y".to_string(), serde_json::json!(-34)),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    RuntimePresentationOperation {
                        op: "animate_title_crystal".to_string(),
                        source_span: presentation_span(),
                        fields: [
                            (
                                "target".to_string(),
                                serde_json::json!("wShadowOAMSprite00YCoord"),
                            ),
                            ("stop_at".to_string(), serde_json::json!(22)),
                            ("y_delta".to_string(), serde_json::json!(2)),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    RuntimePresentationOperation {
                        op: "draw_indexed_title_suicune_frame".to_string(),
                        source_span: presentation_span(),
                        fields: [
                            ("frames".to_string(), serde_json::json!([128, 136, 0, 8])),
                            (
                                "selector".to_string(),
                                serde_json::json!({
                                    "mask": 24,
                                    "shift_left": 1,
                                    "swap_nibbles": true
                                }),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    operation("write_memory_byte", "hSCX", "value", 112),
                    operation("subtract_memory_byte", "hSCX", "delta", 4),
                    operation("write_memory_word", "wTitleScreenTimer", "value", 4_416),
                    operation("fade_audio", "", "frames", 64),
                    input_mask(".delete_save_data@TitleScreenMain", 0x46),
                    input_mask(".check_start@TitleScreenMain", 0x86),
                    input_mask(".reset_clock@TitleScreenMain", 0x60),
                    input_mask(".incave@TitleScreenMain", 0x09),
                ],
            }],
            source_span: presentation_span(),
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };

    assert_eq!(
        RuntimeTitlePresentationParameters::from_program(&program)
            .expect("source-derived title parameters should resolve"),
        RuntimeTitlePresentationParameters {
            entrance_start_scx: 112,
            entrance_scroll_step: 4,
            timeout_frames: 4_416,
            timeout_fade_frames: 64,
            crystal_oam_target: "wShadowOAMSprite00YCoord".to_string(),
            crystal_initial_y: 222,
            suicune_frames: vec![128, 136, 0, 8],
            suicune_selector_mask: 24,
            suicune_selector_shift_left: 1,
            suicune_selector_swap_nibbles: true,
            delete_save_mask: 0x46,
            clock_reset_arm_mask: 0x86,
            clock_reset_finish_mask: 0x60,
            start_mask: 0x09,
        }
    );
}

#[test]
fn title_main_menu_definition_is_read_from_certified_asm_tables() {
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "main_menu".to_string(),
            phases: vec![RuntimePresentationPhase {
                id: "main_menu".to_string(),
                source_span: presentation_span(),
                operations: vec![
                    RuntimePresentationOperation {
                        op: "select_main_menu_variant".to_string(),
                        source_span: presentation_span(),
                        fields: [(
                            "variants".to_string(),
                            serde_json::json!([
                                { "id": "new_game", "value": 0 },
                                { "id": "continue", "value": 1 },
                                { "id": "mystery", "value": 6 }
                            ]),
                        )]
                        .into_iter()
                        .collect(),
                    },
                    RuntimePresentationOperation {
                        op: "load_menu".to_string(),
                        source_span: presentation_span(),
                        fields: [
                            (
                                "coordinates".to_string(),
                                serde_json::json!({
                                    "left": 0, "top": 0, "right": 16, "bottom": 7
                                }),
                            ),
                            ("default_option".to_string(), serde_json::json!(1)),
                            (
                                "item_sets".to_string(),
                                serde_json::json!([
                                    [1, 2],
                                    [0, 1, 2],
                                    [1, 2],
                                    [1, 2],
                                    [1, 2],
                                    [1, 2],
                                    [0, 1, 2]
                                ]),
                            ),
                            (
                                "strings".to_string(),
                                serde_json::json!(["CONTINUE", "NEW GAME", "OPTION"]),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    RuntimePresentationOperation {
                        op: "dispatch_table".to_string(),
                        source_span: presentation_span(),
                        fields: [
                            (
                                "dispatcher".to_string(),
                                serde_json::json!("MainMenu selection"),
                            ),
                            (
                                "entries".to_string(),
                                serde_json::json!([
                                    "MainMenu_Continue",
                                    "MainMenu_NewGame",
                                    "MainMenu_Option"
                                ]),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    },
                ],
                ..RuntimePresentationPhase::default()
            }],
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };

    let definition = RuntimeTitleMainMenuDefinition::from_program(&program)
        .expect("source-derived main menu should resolve");
    assert_eq!(
        &definition.variants[..2],
        vec![
            vec![
                RuntimeTitleMainMenuItem {
                    label: "NEW GAME".to_string(),
                    dispatch_target: "MainMenu_NewGame".to_string(),
                },
                RuntimeTitleMainMenuItem {
                    label: "OPTION".to_string(),
                    dispatch_target: "MainMenu_Option".to_string(),
                },
            ],
            vec![
                RuntimeTitleMainMenuItem {
                    label: "CONTINUE".to_string(),
                    dispatch_target: "MainMenu_Continue".to_string(),
                },
                RuntimeTitleMainMenuItem {
                    label: "NEW GAME".to_string(),
                    dispatch_target: "MainMenu_NewGame".to_string(),
                },
                RuntimeTitleMainMenuItem {
                    label: "OPTION".to_string(),
                    dispatch_target: "MainMenu_Option".to_string(),
                },
            ],
        ]
    );
    assert_eq!(definition.new_game_variant, 0);
    assert_eq!(definition.continue_variant, 1);
    assert_eq!(definition.mystery_variant, 6);
    assert_eq!(
        (
            definition.left,
            definition.top,
            definition.right,
            definition.bottom,
            definition.default_option,
        ),
        (0, 0, 16, 7, 1)
    );
}

#[test]
fn gender_menu_definition_is_read_from_certified_asm_tables() {
    let program = RuntimePresentationProgram {
        subprograms: vec![RuntimePresentationSubprogram {
            id: "player_profile_setup".to_string(),
            phases: vec![RuntimePresentationPhase {
                id: "gender_selection".to_string(),
                source_span: presentation_span(),
                operations: vec![
                    RuntimePresentationOperation {
                        op: "load_menu".to_string(),
                        source_span: presentation_span(),
                        fields: [
                            ("items".to_string(), serde_json::json!(["Boy", "Girl"])),
                            (
                                "coordinates".to_string(),
                                serde_json::json!({
                                    "left": 6, "top": 4, "right": 12, "bottom": 9
                                }),
                            ),
                            ("default_option".to_string(), serde_json::json!(1)),
                        ]
                        .into_iter()
                        .collect(),
                    },
                    RuntimePresentationOperation {
                        op: "select_player_gender".to_string(),
                        source_span: presentation_span(),
                        fields: [(
                            "domain".to_string(),
                            serde_json::json!([
                                { "cursor": 1, "value": 0, "label": "Boy" },
                                { "cursor": 2, "value": 1, "label": "Girl" }
                            ]),
                        )]
                        .into_iter()
                        .collect(),
                    },
                    RuntimePresentationOperation {
                        op: "wait_frames".to_string(),
                        source_span: presentation_span(),
                        fields: [("frames".to_string(), serde_json::json!(10))]
                            .into_iter()
                            .collect(),
                    },
                ],
                ..RuntimePresentationPhase::default()
            }],
            ..RuntimePresentationSubprogram::default()
        }],
        ..RuntimePresentationProgram::default()
    };

    assert_eq!(
        RuntimeGenderMenuDefinition::from_program(&program)
            .expect("source-derived gender menu should resolve"),
        RuntimeGenderMenuDefinition {
            items: vec!["Boy".to_string(), "Girl".to_string()],
            values: vec![0, 1],
            left: 6,
            top: 4,
            right: 12,
            bottom: 9,
            default_option: 1,
            confirm_delay_frames: 10,
        }
    );
}

fn test_battle_tower_trainers() -> Vec<BattleTowerTrainerDefinition> {
    vec![BattleTowerTrainerDefinition {
        index: 0,
        trainer_class: "YOUNGSTER".to_string(),
        name: "TEST@".to_string(),
        sprite_constant: "SPRITE_YOUNGSTER".to_string(),
        female: false,
    }]
}

fn test_battle_tower_mon_groups() -> Vec<Vec<BattleTowerMonDefinition>> {
    vec![vec![BattleTowerMonDefinition {
        species: "RATTATA".to_string(),
        moves: vec![
            "TACKLE".to_string(),
            "NO_MOVE".to_string(),
            "NO_MOVE".to_string(),
            "NO_MOVE".to_string(),
        ],
        stat_exp: vec![0; 5],
        dvs: vec![8; 4],
        pp: vec![35, 0, 0, 0],
        pokerus: vec![0; 3],
        level: 10,
        status: vec![0; 2],
        stats: vec![30, 30, 18, 15, 22, 13, 13],
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
        reward_candidates: vec!["HP_UP".to_string(), "LUCKY_PUNCH".to_string()],
        excluded_reward_items: vec!["LUCKY_PUNCH".to_string()],
        reward_quantity: 5,
        reward_failure_sentinel: "POTION".to_string(),
        reward_item_values: [
            ("POTION".to_string(), 0x12),
            ("HP_UP".to_string(), 0x1a),
            ("LUCKY_PUNCH".to_string(), 0x1e),
        ]
        .into_iter()
        .collect(),
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
    let catalog = compiled_standard_script_catalog(pack.data()).expect("standard-script catalog");
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
            StandardScriptExecutionPath::CommonInterpreter => common_interpreter_labels.push(label),
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

    let mut stale = data.clone();
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
fn compiled_overworld_event_catalog_requires_player_event_pointer_bodies() {
    let pack = AssetRoot::new(repository_root_for_tests())
        .load_verified_compiled_game_pack("content-packs/core-modular.crystalpack")
        .expect("load regenerated compiled game pack");
    let data = pack.data().clone();
    validate_compiled_overworld_event_catalog(&data).expect("valid overworld player-event catalog");
    let catalog = compiled_overworld_event_catalog(&data).expect("overworld event catalog");
    assert_eq!(
        catalog
            .get("ChangeDirectionScript")
            .and_then(Value::as_array)
            .and_then(|body| body.first())
            .and_then(|command| command.get("command"))
            .and_then(Value::as_str),
        Some("deactivatefacing")
    );

    let mut missing = data.clone();
    missing.story_events.retain(|payload| {
        !payload
            .as_object()
            .is_some_and(|payload| payload.contains_key("OverworldEvents"))
    });
    let error = validate_compiled_overworld_event_catalog(&missing)
        .expect_err("missing catalog must fail")
        .to_string();
    assert!(error.contains("missing the OverworldEvents"), "{error}");

    let mut stale = data.clone();
    let catalog = stale
        .story_events
        .iter_mut()
        .find_map(|payload| payload.get_mut("OverworldEvents"))
        .and_then(Value::as_object_mut)
        .expect("mutable overworld event catalog");
    catalog.remove("ChangeDirectionScript");
    let error = validate_compiled_overworld_event_catalog(&stale)
        .expect_err("pointer without a body must fail")
        .to_string();
    assert!(
        error.contains("ChangeDirectionScript has no command body"),
        "{error}"
    );

    let mut drifted_pointers = data.clone();
    let pointer_table = drifted_pointers
        .story_events
        .iter_mut()
        .find_map(|payload| payload.get_mut("OverworldEvents"))
        .and_then(Value::as_object_mut)
        .and_then(|catalog| catalog.get_mut("PlayerEventScriptPointers"))
        .and_then(Value::as_array_mut)
        .expect("mutable PlayerEventScriptPointers");
    pointer_table
        .iter_mut()
        .filter(|entry| entry.get("command").and_then(Value::as_str) == Some("dba"))
        .nth(5)
        .expect("PLAYEREVENT_WARP pointer")["args"] = serde_json::json!(["FallIntoMapScript"]);
    let error = validate_compiled_overworld_event_catalog(&drifted_pointers)
        .expect_err("player-event pointer index drift must fail")
        .to_string();
    assert!(
        error.contains("pointer 5 requires WarpToNewMapScript"),
        "{error}"
    );

    let mut drifted_interpreted = data.clone();
    let warp_body = drifted_interpreted
        .story_events
        .iter_mut()
        .find_map(|payload| payload.get_mut("OverworldEvents"))
        .and_then(Value::as_object_mut)
        .and_then(|catalog| catalog.get_mut("WarpToNewMapScript"))
        .and_then(Value::as_array_mut)
        .expect("mutable WarpToNewMapScript body");
    warp_body[1]["args"] = serde_json::json!(["MAPSETUP_FALL"]);
    let error = validate_compiled_overworld_event_catalog(&drifted_interpreted)
        .expect_err("interpreter-owned player-event source drift must fail")
        .to_string();
    assert!(
        error.contains("common interpreter certificate failed for WarpToNewMapScript"),
        "{error}"
    );

    let mut drifted_pitfall_target = data.clone();
    let landing_body = drifted_pitfall_target
        .story_events
        .iter_mut()
        .find_map(|payload| payload.get_mut("OverworldEvents"))
        .and_then(Value::as_object_mut)
        .and_then(|catalog| catalog.get_mut("LandAfterPitfallScript"))
        .and_then(Value::as_array_mut)
        .expect("mutable LandAfterPitfallScript body");
    landing_body[0]["args"] = serde_json::json!(["15"]);
    let error = validate_compiled_overworld_event_catalog(&drifted_pitfall_target)
        .expect_err("called player-event source drift must fail")
        .to_string();
    assert!(
        error.contains("FallIntoMapScript target LandAfterPitfallScript"),
        "{error}"
    );

    let mut drifted_direction_target = data.clone();
    let enable_wild_body = drifted_direction_target
        .story_events
        .iter_mut()
        .find_map(|payload| payload.get_mut("OverworldEvents"))
        .and_then(Value::as_object_mut)
        .and_then(|catalog| catalog.get_mut("EnableWildEncounters"))
        .and_then(Value::as_array_mut)
        .expect("mutable EnableWildEncounters body");
    enable_wild_body[1]["args"] = serde_json::json!(["PLAYEREVENTS_WARPS_AND_CONNECTIONS", "[hl]"]);
    let error = validate_compiled_overworld_event_catalog(&drifted_direction_target)
        .expect_err("called ChangeDirectionScript CPU source drift must fail")
        .to_string();
    assert!(
        error.contains("ChangeDirectionScript target EnableWildEncounters"),
        "{error}"
    );

    let mut drifted = data;
    let seen_body = drifted
        .story_events
        .iter_mut()
        .find_map(|payload| payload.get_mut("OverworldEvents"))
        .and_then(Value::as_object_mut)
        .and_then(|catalog| catalog.get_mut("SeenByTrainerScript"))
        .and_then(Value::as_array_mut)
        .expect("mutable SeenByTrainerScript body");
    seen_body[0]["command"] = Value::String("legacy_trainer_fallback".to_string());
    let error = validate_compiled_overworld_event_catalog(&drifted)
        .expect_err("typed player-event source drift must fail")
        .to_string();
    assert!(
        error.contains("typed consumer certificate failed for SeenByTrainerScript"),
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
    let puzzle_ids = source_between(unown, "for puzzle_id in [", "] {");
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
fn regenerated_core_pack_uses_canonical_pcm_audio() {
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
    assert_eq!(format.channels, 2);
    assert_eq!(format.bits_per_sample, 16);
    assert!(pack.compiled_audio().contains_key("MUSIC_TITLE"));
}
use crystal_core::systems::script_objects::{apply_script_movement, apply_script_object_mutation};

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
};
use crystal_core::world::encounters::EncounterMusicModifier;
use crystal_core::world::encounters::{
    EncounterSurface, FieldEncounterData, FieldEncounterEntry, FieldEncounterTable, TimeOfDay,
    WildEncounter, WildEncounterTable, table_for_surface,
};
use crystal_core::world::map::{Direction, OverworldMapData, TilePosition};
use crystal_core::world::movement::{StepOptions, StepOutcome};
use crystal_core::world::session::OverworldSession;

fn npc_trade_rules<const N: usize>(ids: [&str; N]) -> BTreeMap<String, NpcTradeRule> {
    ids.into_iter()
        .map(|id| (id.to_string(), NpcTradeRule::default()))
        .collect()
}

fn special_routine_rules<const N: usize>(ids: [&str; N]) -> BTreeMap<String, SpecialRoutineRule> {
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
        callee_script: Some("TestPhoneCalleeScript".to_string()),
        caller_time_mask: 7,
        caller_script: Some("TestPhoneCallerScript".to_string()),
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
        source_index: 1,
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
    for (source_index, move_data) in data.moves.values_mut().enumerate() {
        move_data.source_index = u8::try_from(source_index + 1).expect("test move source index");
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
        sample_rate_hz: 22_050,
        channels: 2,
        bits_per_sample: 16,
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
    let original_first_map = data.maps.keys().next().cloned();
    add_runtime_species_and_move(data);
    if data.decorations == DecorationCatalog::default() {
        data.decorations =
            read_json_file(&repository_root_for_tests().join(
                "apps/web/assets/data/content-packs/core-modular/decorations/decorations.json",
            ))
            .expect("load canonical decoration catalog fixture");
    }
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
                "GlobalScriptRoots": [],
                "DifficultBookshelfScript": [
                    { "command": "farjumptext", "args": ["DifficultBookshelfText"] }
                ]
            }
        }));
    }
    if !data.story_events.iter().any(|payload| {
        payload
            .as_object()
            .is_some_and(|payload| payload.contains_key("OverworldEvents"))
    }) {
        data.story_events.push(serde_json::json!({
            "OverworldEvents": {
                "PlayerEventScriptPointers": [
                    { "command": "dba", "args": ["InvalidEventScript"] },
                    { "command": "dba", "args": ["SeenByTrainerScript"] },
                    { "command": "dba", "args": ["TalkToTrainerScript"] },
                    { "command": "dba", "args": ["FindItemInBallScript"] },
                    { "command": "dba", "args": ["EdgeWarpScript"] },
                    { "command": "dba", "args": ["WarpToNewMapScript"] },
                    { "command": "dba", "args": ["FallIntoMapScript"] },
                    { "command": "dba", "args": ["OverworldWhiteoutScript"] },
                    { "command": "dba", "args": ["HatchEggScript"] },
                    { "command": "dba", "args": ["ChangeDirectionScript"] },
                    { "command": "dba", "args": ["InvalidEventScript"] }
                ],
                "InvalidEventScript": [
                    { "command": "end", "args": [] }
                ],
                "SeenByTrainerScript": [
                    { "command": "loadtemptrainer", "args": [] },
                    { "command": "encountermusic", "args": [] },
                    { "command": "showemote", "args": ["EMOTE_SHOCK", "LAST_TALKED", "30"] },
                    { "command": "callasm", "args": ["TrainerWalkToPlayer"] },
                    { "command": "applymovementlasttalked", "args": ["wMovementBuffer"] },
                    { "command": "writeobjectxy", "args": ["LAST_TALKED"] },
                    { "command": "faceobject", "args": ["PLAYER", "LAST_TALKED"] },
                    { "command": "sjump", "args": ["StartBattleWithMapTrainerScript"] }
                ],
                "TalkToTrainerScript": [
                    { "command": "faceplayer", "args": [] },
                    { "command": "trainerflagaction", "args": ["CHECK_FLAG"] },
                    { "command": "iftrue", "args": ["AlreadyBeatenTrainerScript"] },
                    { "command": "loadtemptrainer", "args": [] },
                    { "command": "encountermusic", "args": [] },
                    { "command": "sjump", "args": ["StartBattleWithMapTrainerScript"] }
                ],
                "FindItemInBallScript": [
                    { "command": "callasm", "args": [".TryReceiveItem"] },
                    { "command": "iffalse", "args": [".no_room"] },
                    { "command": "disappear", "args": ["LAST_TALKED"] },
                    { "command": "opentext", "args": [] },
                    { "command": "writetext", "args": [".FoundItemText"] },
                    { "command": "playsound", "args": ["SFX_ITEM"] },
                    { "command": "pause", "args": ["60"] },
                    { "command": "itemnotify", "args": [] },
                    { "command": "closetext", "args": [] },
                    { "command": "end", "args": [] }
                ],
                "EdgeWarpScript": [
                    { "command": "reloadend", "args": ["MAPSETUP_CONNECTION"] }
                ],
                "WarpToNewMapScript": [
                    { "command": "warpsound", "args": [] },
                    { "command": "newloadmap", "args": ["MAPSETUP_DOOR"] },
                    { "command": "end", "args": [] }
                ],
                "FallIntoMapScript": [
                    { "command": "newloadmap", "args": ["MAPSETUP_FALL"] },
                    { "command": "playsound", "args": ["SFX_KINESIS"] },
                    { "command": "applymovement", "args": ["PLAYER", ".SkyfallMovement"] },
                    { "command": "playsound", "args": ["SFX_STRENGTH"] },
                    { "command": "scall", "args": ["LandAfterPitfallScript"] },
                    { "command": "end", "args": [] }
                ],
                ".SkyfallMovement@FallIntoMapScript": [
                    { "command": "skyfall", "args": [] },
                    { "command": "step_end", "args": [] }
                ],
                "LandAfterPitfallScript": [
                    { "command": "earthquake", "args": ["16"] },
                    { "command": "end", "args": [] }
                ],
                "OverworldWhiteoutScript": [
                    { "command": "reanchormap", "args": [] },
                    { "command": "callasm", "args": ["OverworldBGMap"] },
                    { "command": "sjump", "args": ["Script_Whiteout"] }
                ],
                "HatchEggScript": [
                    { "command": "callasm", "args": ["OverworldHatchEgg"] },
                    { "command": "end", "args": [] }
                ],
                "ChangeDirectionScript": [
                    { "command": "deactivatefacing", "args": ["3"] },
                    { "command": "callasm", "args": ["EnableWildEncounters"] },
                    { "command": "end", "args": [] }
                ],
                "EnableWildEncounters": [
                    { "command": "ld", "args": ["hl", "wEnabledPlayerEvents"] },
                    { "command": "set", "args": ["PLAYEREVENTS_WILD_ENCOUNTERS", "[hl]"] },
                    { "command": "ret", "args": [] }
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
    data.items
        .entry("SUPER_POTION".to_string())
        .or_insert_with(|| test_item("SUPER_POTION"));
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
        ("CARD_KEY", "CARD_KEY"),
        ("BASEMENT_KEY", "BASEMENT_KEY"),
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
    data.currency_constants
        .0
        .insert("START_MONEY".to_string(), 3_000);
    data.currency_constants
        .0
        .insert("MOM_MONEY".to_string(), 2_300);
    data.step_event_rules = test_step_event_rules();
    if data.fishing.groups.is_empty() && data.fishing.rod_items.is_empty() {
        data.fishing = serde_json::from_value(serde_json::json!({
                "groups": {
                    "test": {
                        "source_index": 1,
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
    let installed_default_field_moves = data.field_moves == FieldMoveCatalog::default();
    if installed_default_field_moves {
        data.field_moves = test_field_move_catalog();
        for (map_name, map_constant, target_script) in [
            ("RadioTower3F", "RADIO_TOWER_3F", "CardKeySlotScript"),
            (
                "GoldenrodUnderground",
                "GOLDENROD_UNDERGROUND",
                "BasementDoorScript",
            ),
        ] {
            let mut module = test_map_module(map_name, map_constant, None);
            module.attributes.width = 10;
            module.attributes.height = 10;
            module.blocks = vec![0; 100];
            module
                .scripts
                .insert(target_script.to_string(), serde_json::json!([]));
            data.maps.insert(map_name.to_string(), module);
        }
    }
    data.runtime_title_screen = RuntimeTitleScreen {
        title_music: Some("MUSIC_TITLE".to_string()),
        program: complete_runtime_title_program(),
    };
    data.story_event_script_constants
        .global
        .insert("SPAWN_HOME".to_string(), 1);
    data.trainers
        .trainers
        .entry("YOUNGSTER_JOEY".to_string())
        .or_insert_with(|| test_trainer("YOUNGSTER_JOEY", "MUSIC_TITLE"));
    data.trainer_class_names
        .entry("YOUNGSTER".to_string())
        .or_insert_with(|| "YOUNGSTER".to_string());
    let first_map = original_first_map.clone();
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
        let map_id = constant
            .strip_prefix("ROAMING_TEST_")
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(1);
        data.runtime_map_metadata.insert(
            constant.clone(),
            RuntimeMapMetadata {
                constant,
                name: map_name.clone(),
                group_name: "GROUP_TEST".to_string(),
                group_id: 1,
                map_id,
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
        ModpackAudioAsset::sound_effect("SFX_ITEM", "content-packs/test/sfx/SFX_ITEM.pcm", 0x01)
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
    if let Some(first_map) = original_first_map {
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
    data.special_phone_calls.insert(
        "TEST_CALL".to_string(),
        SpecialPhoneCallRule {
            value: 1,
            condition: "SpecialCallWhereverYouAre".to_string(),
            contact_id: "TEST_CONTACT".to_string(),
            caller_script: "TestPhoneCallerScript".to_string(),
        },
    );
    let phone_roots = BTreeMap::from([
        (
            "LoadPhoneScriptBank".to_string(),
            serde_json::json!([{"command": "pause", "args": ["1"]}]),
        ),
        (
            "LoadOutOfAreaScript".to_string(),
            serde_json::json!([{"command": "pause", "args": ["1"]}]),
        ),
        (
            "PhoneScript_JustTalkToThem".to_string(),
            serde_json::json!([{"command": "pause", "args": ["1"]}]),
        ),
        (
            "PhoneOutOfAreaScript".to_string(),
            serde_json::json!([{"command": "pause", "args": ["1"]}]),
        ),
        (
            "TestPhoneCalleeScript".to_string(),
            serde_json::json!([{"command": "pause", "args": ["1"]}]),
        ),
        (
            "TestPhoneCallerScript".to_string(),
            serde_json::json!([{"command": "pause", "args": ["1"]}]),
        ),
    ]);
    let parsed_phone_roots = parse_script_runtime_commands("GlobalScripts", &phone_roots)
        .expect("parse complete phone-root fixture");
    let global_scripts = data
        .global_scripts
        .get_or_insert_with(GlobalScriptModule::default);
    for (script, body) in phone_roots {
        global_scripts.scripts.entry(script).or_insert(body);
    }
    for command in parsed_phone_roots {
        if !global_scripts
            .script_runtime_commands
            .iter()
            .any(|existing| {
                existing.source_script == command.source_script
                    && existing.command_index == command.command_index
            })
        {
            global_scripts.script_runtime_commands.push(command);
        }
    }
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
        reward_candidates: vec!["HP_UP".to_string(), "LUCKY_PUNCH".to_string()],
        excluded_reward_items: vec!["LUCKY_PUNCH".to_string()],
        reward_quantity: 5,
        reward_failure_sentinel: "POTION".to_string(),
        reward_item_values: [
            ("POTION".to_string(), 0x12),
            ("HP_UP".to_string(), 0x1a),
            ("LUCKY_PUNCH".to_string(), 0x1e),
        ]
        .into_iter()
        .collect(),
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

fn add_roaming_verification_maps(data: &mut GameDataSet) {
    for map_id in 2_u16..=16 {
        let map_name = format!("RoamingTest{map_id}");
        let map_constant = format!("ROAMING_TEST_{map_id}");
        data.maps.insert(
            map_name.clone(),
            test_map_module(&map_name, &map_constant, None),
        );
    }
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
        std::fs::write(path, [0_u8; 4]).expect("write complete runtime audio fixture");
    }
}

fn add_wild_encounter_marker(data: &mut GameDataSet) {
    data.wild_encounters.insert(
        "Route29".to_string(),
        WildEncounterData {
            map_name: "Route29".to_string(),
            grass_rates: Some([("day".to_string(), 30)].into_iter().collect()),
            water_rate: None,
            swarm_overrides: BTreeMap::new(),
            zones: Vec::new(),
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

struct TrainerAiTestRandom {
    values: std::collections::VecDeque<u8>,
    calls: usize,
}

impl TrainerAiTestRandom {
    fn new(values: impl IntoIterator<Item = u8>) -> Self {
        Self {
            values: values.into_iter().collect(),
            calls: 0,
        }
    }
}

impl crystal_core::random::BattleRandomSource for TrainerAiTestRandom {
    fn battle_random_byte(&mut self) -> u8 {
        self.calls += 1;
        self.values
            .pop_front()
            .expect("trainer AI test RNG exhausted")
    }
}

fn trainer_post_order_test_data_and_combat() -> (GameDataSet, BattleCombatState) {
    let mut data = GameDataSet::default();
    data.moves.insert("TACKLE".to_string(), test_move("TACKLE"));
    let neutral = crystal_core::battle::damage::TypeMultiplier {
        numerator: 1,
        denominator: 1,
    };
    data.type_effectiveness.matchups.insert(
        "ELECTRIC".to_string(),
        [("ELECTRIC".to_string(), neutral)].into_iter().collect(),
    );
    data.type_effectiveness.matchups.insert(
        "NORMAL".to_string(),
        [("ELECTRIC".to_string(), neutral)].into_iter().collect(),
    );
    let mut trainer = test_trainer("YOUNGSTER_JOEY", "MUSIC_NONE");
    trainer.items = vec![Some("POTION".to_string()), None];
    data.trainers
        .trainers
        .insert(trainer.trainer_id.clone(), trainer);
    let mut player = Pokemon::new_for_tests(species(), 5, Dv::default());
    player.moves = vec![LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 35,
        pp_ups: 0,
    }];
    let mut enemy = Pokemon::new_for_tests(species(), 5, Dv::default());
    enemy.moves = player.moves.clone();
    enemy.hp = (enemy.max_hp / 4).max(1);
    let combat = BattleCombatState::new(player.clone(), enemy.clone())
        .with_parties(vec![player], vec![enemy])
        .with_party_indices(0, 0);
    (data, combat)
}

#[test]
fn trainer_post_order_item_uses_exact_inventory_slot_once() {
    let (data, combat) = trainer_post_order_test_data_and_combat();
    let mut used = BTreeSet::new();
    let mut rng = TrainerAiTestRandom::new([]);
    let action = data
        .select_trainer_post_order_action(
            &combat,
            "YOUNGSTER_JOEY",
            "BATTLETYPE_NORMAL",
            0,
            &mut used,
            2,
            &mut rng,
        )
        .expect("trainer item selection");
    assert_eq!(
        action,
        BattleAction::TrainerItem {
            selected_move_slot: 2,
            item_id: "POTION".to_string(),
        }
    );
    assert_eq!(
        used,
        BTreeSet::from(["YOUNGSTER_JOEY:POTION:0".to_string()])
    );
    assert_eq!(rng.calls, 0);
}

#[test]
fn trainer_post_order_move_lock_suppresses_switch_and_item_without_rng() {
    let (data, mut combat) = trainer_post_order_test_data_and_combat();
    combat.enemy_recharge_move = Some("TACKLE".to_string());
    let mut used = BTreeSet::new();
    let mut rng = TrainerAiTestRandom::new([]);
    let action = data
        .select_trainer_post_order_action(
            &combat,
            "YOUNGSTER_JOEY",
            "BATTLETYPE_NORMAL",
            1,
            &mut used,
            1,
            &mut rng,
        )
        .expect("locked trainer action");
    assert_eq!(action, BattleAction::Move { slot: 1 });
    assert!(used.is_empty());
    assert_eq!(rng.calls, 0);
}

#[test]
fn trainer_post_order_perish_switch_uses_source_tier_and_one_battle_byte() {
    let (data, mut combat) = trainer_post_order_test_data_and_combat();
    combat
        .enemy_party
        .push(Pokemon::new_for_tests(species(), 5, Dv::default()));
    combat.enemy_party[0].hp = combat.enemy_party[0].max_hp;
    combat.enemy_party[0].perish_song_turns = 1;
    let mut used = BTreeSet::new();
    let mut rng = TrainerAiTestRandom::new([255]);
    let action = data
        .select_trainer_post_order_action(
            &combat,
            "YOUNGSTER_JOEY",
            "BATTLETYPE_NORMAL",
            1,
            &mut used,
            3,
            &mut rng,
        )
        .expect("trainer perish switch");
    assert_eq!(
        action,
        BattleAction::TrainerSwitch {
            selected_move_slot: 3,
            party_index: 1,
        }
    );
    assert_eq!(rng.calls, 1);
    assert!(used.is_empty());
}

#[test]
fn battle_tower_post_order_forbids_trainer_items() {
    let (mut data, combat) = trainer_post_order_test_data_and_combat();
    let mut falkner = test_trainer("FALKNER_1", "MUSIC_NONE");
    falkner.trainer_class = "FALKNER".to_string();
    falkner.ai_item_switch_flags = 0;
    data.trainers
        .trainers
        .insert(falkner.trainer_id.clone(), falkner);
    let mut used = BTreeSet::new();
    let mut rng = TrainerAiTestRandom::new([]);
    let action = data
        .select_trainer_post_order_action(
            &combat,
            "YOUNGSTER_JOEY",
            "BATTLETYPE_BATTLE_TOWER",
            u32::MAX,
            &mut used,
            0,
            &mut rng,
        )
        .expect("Battle Tower trainer action");
    assert_eq!(action, BattleAction::Move { slot: 0 });
    assert!(used.is_empty());
    assert_eq!(rng.calls, 0);
}

#[test]
fn active_wild_battle_escape_rejects_truncated_divider_trace_before_mutation() {
    let mut data = GameDataSet::default();
    data.battle_escape_rules = test_battle_escape_rules();
    data.battle_stat_multipliers = test_battle_stat_multipliers();
    data.roaming_pokemon = roaming_catalog_for_tests("NEW_MON", "NEW_MON");
    data.runtime_map_metadata.insert(
        "ROUTE_29".to_string(),
        test_runtime_map_metadata("ROUTE_29", "Route29"),
    );
    let player = crystal_core::models::Pokemon::new_for_tests(
        species(),
        20,
        crystal_core::models::Dv::default(),
    );
    let mut fast_species = species();
    fast_species.base_stats.speed = 255;
    let enemy = crystal_core::models::Pokemon::new_for_tests(
        fast_species,
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
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(player);
    let before = state.clone();
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
                divider_trace: RuntimeDividerTrace::new([]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("truncated escape divider trace must reject");

    assert!(error.to_string().contains("divider replay exhausted"));
    assert_eq!(state, before);

    let mut smoke_ball = test_item("SMOKE_BALL");
    smoke_ball.held_effect = "HELD_ESCAPE".to_string();
    data.items.insert("SMOKE_BALL".to_string(), smoke_ball);
    state.storage.party.pokemon[0]
        .as_mut()
        .expect("active Pokemon")
        .item = Some("SMOKE_BALL".to_string());
    let outcome = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::AttemptEscapeActiveWildBattle(RuntimeBattleEscapeCommand {
                divider_trace: divider_trace_for_sub_values([1]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect("held escape effect consumes only the battle-end roaming gate DIV");
    let RuntimeMutationResult::ActiveWildBattleEscapeAttempted(escape) = outcome.result else {
        panic!("expected escape mutation result");
    };
    assert!(escape.escaped);
    assert_eq!(escape.roll, None);
    assert!(matches!(state.battle, BattleMemory::Inactive));
}

#[test]
fn runtime_blackout_recovery_uses_authoritative_saved_spawn() {
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
    data.maps = map_payload(vec![test_map_module("PlayersHouse2F", "ROUTE_29", None)]);
    data.tilesets = BTreeMap::from([("johto".to_string(), test_tileset_definition())]);
    data.pokegear_landmarks = map_name_sign_landmarks_for_tests(["PlayersHouse2F"]);
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
        battle: BattleMemory::StaticWild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            roaming_slot: None,
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
    state
        .script_runtime
        .deferred_scripts
        .push(crystal_core::state::ScriptLocation {
            origin_map_name: "Route30".to_string(),
            script: "RockSmashScript".to_string(),
        });
    state
        .script_runtime
        .call_stack
        .push(crystal_core::state::ScriptReturnFrame {
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
    assert!(
        error.to_string().contains("terminal result 0x02"),
        "{error:#}"
    );
    assert_eq!((draw_state, draw_session), draw_before);

    crystal_core::battle::start::deactivate_battle_after_loss(&mut state);
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
    assert!(matches!(state.battle, BattleMemory::Inactive));
    assert_eq!(state.battle_result, 1);
    assert!(state.pending_static_wild_terminal.is_none());
    assert_eq!(
        state.money, 50,
        "loss skips Pay Day before whiteout halves money"
    );
    assert!(state.script_runtime.next_script.is_none());
    assert!(state.script_runtime.deferred_scripts.is_empty());
    assert!(state.script_runtime.call_stack.is_empty());
    assert!(state.script_runtime.command_queue.is_empty());
    assert!(
        state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .all(|pokemon| pokemon.hp == pokemon.max_hp)
    );
    assert!(state.script_runtime.pending_script_warp.is_none());
    assert_eq!(session.map.name, "PlayersHouse2F");
    assert_eq!(session.player.tile, TilePosition::new(0, 0));
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
    data.roaming_pokemon = roaming_catalog_for_tests("NEW_MON", "NEW_MON");
    data.runtime_map_metadata.insert(
        "ROUTE_29".to_string(),
        test_runtime_map_metadata("ROUTE_29", "Route29"),
    );
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
    data.roaming_pokemon = roaming_catalog_for_tests("NEW_MON", "NEW_MON");
    data.runtime_map_metadata.insert(
        "ROUTE_29".to_string(),
        test_runtime_map_metadata("ROUTE_29", "Route29"),
    );
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
fn faint_prompt_escape_uses_first_party_raw_speed_instead_of_fainted_active_speed() {
    let mut data = GameDataSet::default();
    data.roaming_pokemon = roaming_catalog_for_tests("NEW_MON", "NEW_MON");
    data.runtime_map_metadata.insert(
        "ROUTE_29".to_string(),
        test_runtime_map_metadata("ROUTE_29", "Route29"),
    );
    data.battle_escape_rules = test_battle_escape_rules();
    data.battle_stat_multipliers = test_battle_stat_multipliers();

    let species_with_speed = |id: &str, speed| {
        let mut species = species();
        species.id = id.to_string();
        species.base_stats = BaseStats::new(40, 50, 40, speed, 70, 50);
        species
    };
    let first_party =
        Pokemon::new_for_tests(species_with_speed("SLOW_FIRST", 5), 20, Dv::default());
    let mut fainted_active =
        Pokemon::new_for_tests(species_with_speed("FAST_ACTIVE", 120), 20, Dv::default());
    fainted_active.hp = 0;
    let enemy = Pokemon::new_for_tests(species_with_speed("MID_ENEMY", 60), 20, Dv::default());
    assert!(first_party.speed < enemy.speed);
    assert!(enemy.speed < fainted_active.speed);
    let combat = BattleCombatState::new(fainted_active.clone(), enemy.clone()).with_parties(
        vec![first_party.clone(), fainted_active.clone()],
        vec![enemy.clone()],
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
        battle_active_party_index: Some(1),
        battle_active_enemy_party_index: Some(0),
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(first_party);
    state.storage.party.pokemon[1] = Some(fainted_active);
    state.script_runtime.active_battle_combat = Some(combat);
    let trace = divider_trace_for_sub_values([u8::MAX]);
    let mut divider = ReplayDivider::new(trace.samples);

    let outcome = data
        .resolve_active_wild_battle_run_with_divider(&mut state, &mut divider)
        .expect("faint-prompt escape resolves from the source speed pointers");

    assert!(!outcome.escaped);
    assert_eq!(outcome.roll, Some(u8::MAX));
    assert_eq!(outcome.attempts_after, 1);
    assert_eq!(divider.remaining(), 0);
    assert!(matches!(state.battle, BattleMemory::Wild { .. }));
}

#[test]
fn roaming_wild_battle_run_saves_hp_and_runs_exact_route_update() {
    let mut data = GameDataSet {
        roaming_pokemon: roaming_catalog_for_tests("NEW_MON", "NEW_MON"),
        runtime_map_metadata: [(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "Route29"),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    data.battle_escape_rules = test_battle_escape_rules();
    data.battle_stat_multipliers = test_battle_stat_multipliers();
    let mut player = Pokemon::new_for_tests(species(), 20, Dv::default());
    player.item = Some("SMOKE_BALL".to_string());
    let mut smoke_ball = test_item("SMOKE_BALL");
    smoke_ball.held_effect = "HELD_ESCAPE".to_string();
    data.items.insert("SMOKE_BALL".to_string(), smoke_ball);
    let mut enemy = Pokemon::new_for_tests(species(), 40, Dv::default());
    enemy.hp = 7;
    let inactive = crystal_core::state::RoamingPokemonState {
        map_group: data.roaming_pokemon.inactive_map.map_group,
        map_number: data.roaming_pokemon.inactive_map.map_number,
        ..crystal_core::state::RoamingPokemonState::default()
    };
    let mut state = GameState {
        battle: BattleMemory::Wild {
            battle_type: "BATTLETYPE_ROAMING".to_string(),
            battle_music: "MUSIC_SUICUNE_BATTLE".to_string(),
            map_name: "Route29".to_string(),
            roaming_slot: Some(0),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy],
        },
        battle_active_party_index: Some(0),
        battle_active_enemy_party_index: Some(0),
        roaming_pokemon: [
            crystal_core::state::RoamingPokemonState {
                species: Some("NEW_MON".to_string()),
                level: 40,
                map_group: 1,
                map_number: 1,
                hp: 20,
                dvs_be: [0, 0],
            },
            inactive.clone(),
            inactive,
        ],
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(player);
    let trace = divider_trace_for_sub_values([0, 1]);
    let mut divider = ReplayDivider::new(trace.samples);

    let outcome = data
        .resolve_active_wild_battle_run_with_divider(&mut state, &mut divider)
        .expect("held item guarantees escape before exact roaming route update");

    assert!(outcome.escaped);
    assert_eq!(divider.remaining(), 0);
    assert_eq!(state.roaming_pokemon[0].hp, 7);
    assert_eq!(
        (
            state.roaming_pokemon[0].map_group,
            state.roaming_pokemon[0].map_number,
        ),
        (1, 2)
    );
    assert_eq!(
        state.roaming_map_history,
        crystal_core::state::RoamingMapHistory {
            current_map_group: 1,
            current_map_number: 1,
            ..crystal_core::state::RoamingMapHistory::default()
        }
    );
}

#[test]
fn roaming_escape_item_records_the_exact_route_update_rng() {
    let mut data = GameDataSet {
        roaming_pokemon: roaming_catalog_for_tests("NEW_MON", "NEW_MON"),
        runtime_map_metadata: [(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "Route29"),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    let mut escape_item = test_item("POKE_DOLL");
    escape_item.battle_menu = "ITEMMENU_CURRENT".to_string();
    escape_item.battle_usable = true;
    escape_item.battle_escape_mode = Some("WILD_BATTLE".to_string());
    escape_item.consumable = true;
    data.items.insert("POKE_DOLL".to_string(), escape_item);
    let mut enemy = Pokemon::new_for_tests(species(), 40, Dv::default());
    enemy.hp = 9;
    let inactive = crystal_core::state::RoamingPokemonState {
        map_group: data.roaming_pokemon.inactive_map.map_group,
        map_number: data.roaming_pokemon.inactive_map.map_number,
        ..crystal_core::state::RoamingPokemonState::default()
    };
    let mut state = GameState {
        battle: BattleMemory::Wild {
            battle_type: "BATTLETYPE_ROAMING".to_string(),
            battle_music: "MUSIC_SUICUNE_BATTLE".to_string(),
            map_name: "Route29".to_string(),
            roaming_slot: Some(0),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy],
        },
        battle_active_party_index: Some(0),
        battle_active_enemy_party_index: Some(0),
        roaming_pokemon: [
            crystal_core::state::RoamingPokemonState {
                species: Some("NEW_MON".to_string()),
                level: 40,
                map_group: 1,
                map_number: 1,
                hp: 20,
                dvs_be: [0, 0],
            },
            inactive.clone(),
            inactive,
        ],
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(Pokemon::new_for_tests(species(), 20, Dv::default()));
    state
        .bag
        .add_item(&data.items["POKE_DOLL"], 1)
        .expect("add escape item");
    let trace = divider_trace_for_sub_values([0, 1]);
    let mut divider = ReplayDivider::new(trace.samples);

    data.use_bag_item_to_escape_active_wild_battle_with_divider(
        &mut state,
        "POKE_DOLL",
        &mut divider,
    )
    .expect("escape item runs exact roaming battle-end handler");

    assert_eq!(divider.remaining(), 0);
    assert_eq!(state.roaming_pokemon[0].hp, 9);
    assert_eq!(state.roaming_pokemon[0].map_number, 2);
    assert_eq!(state.roaming_map_history.current_map_number, 1);
}

#[test]
fn captured_roamer_is_cleared_without_route_rng() {
    let data = GameDataSet {
        roaming_pokemon: roaming_catalog_for_tests("NEW_MON", "NEW_MON"),
        runtime_map_metadata: [(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "Route29"),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    let enemy = Pokemon::new_for_tests(species(), 40, Dv::default());
    let inactive = crystal_core::state::RoamingPokemonState {
        map_group: data.roaming_pokemon.inactive_map.map_group,
        map_number: data.roaming_pokemon.inactive_map.map_number,
        ..crystal_core::state::RoamingPokemonState::default()
    };
    let mut state = GameState {
        battle: BattleMemory::Wild {
            battle_type: "BATTLETYPE_ROAMING".to_string(),
            battle_music: "MUSIC_SUICUNE_BATTLE".to_string(),
            map_name: "Route29".to_string(),
            roaming_slot: Some(0),
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy],
        },
        battle_active_party_index: Some(0),
        battle_active_enemy_party_index: Some(0),
        roaming_pokemon: [
            crystal_core::state::RoamingPokemonState {
                species: Some("NEW_MON".to_string()),
                level: 40,
                map_group: 1,
                map_number: 1,
                hp: 20,
                dvs_be: [0, 0],
            },
            inactive.clone(),
            inactive,
        ],
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(Pokemon::new_for_tests(species(), 20, Dv::default()));
    let outcome = CaptureOutcome {
        caught: true,
        blocked: false,
        storage_full: false,
        wobble_count: 4,
        animation_shakes: 4,
        final_catch_rate: u8::MAX,
        ball_id: Some("MASTER_BALL".to_string()),
    };
    let mut divider = ReplayDivider::new([]);

    data.complete_active_wild_capture(&mut state, &outcome, None, &mut divider)
        .expect("captured roaming Pokemon clears its slot");

    assert_eq!(divider.remaining(), 0);
    assert_eq!(state.roaming_pokemon[0].species, None);
    assert_eq!(state.roaming_pokemon[0].hp, 0);
    assert_eq!(
        (
            state.roaming_pokemon[0].map_group,
            state.roaming_pokemon[0].map_number,
        ),
        (
            data.roaming_pokemon.inactive_map.map_group,
            data.roaming_pokemon.inactive_map.map_number,
        )
    );
}

#[test]
fn nonroaming_battle_end_uses_one_in_sixteen_gate_before_route_update() {
    let mut data = GameDataSet {
        roaming_pokemon: roaming_catalog_for_tests("NEW_MON", "NEW_MON"),
        runtime_map_metadata: [(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "Route29"),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    data.battle_escape_rules = test_battle_escape_rules();
    data.battle_stat_multipliers = test_battle_stat_multipliers();
    let mut smoke_ball = test_item("SMOKE_BALL");
    smoke_ball.held_effect = "HELD_ESCAPE".to_string();
    data.items.insert("SMOKE_BALL".to_string(), smoke_ball);
    let make_state = || {
        let mut player = Pokemon::new_for_tests(species(), 20, Dv::default());
        player.item = Some("SMOKE_BALL".to_string());
        let enemy = Pokemon::new_for_tests(species(), 20, Dv::default());
        let inactive = crystal_core::state::RoamingPokemonState {
            map_group: data.roaming_pokemon.inactive_map.map_group,
            map_number: data.roaming_pokemon.inactive_map.map_number,
            ..crystal_core::state::RoamingPokemonState::default()
        };
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
            roaming_pokemon: [
                crystal_core::state::RoamingPokemonState {
                    species: Some("NEW_MON".to_string()),
                    level: 40,
                    map_group: 1,
                    map_number: 1,
                    hp: 20,
                    dvs_be: [0, 0],
                },
                inactive.clone(),
                inactive,
            ],
            ..GameState::default()
        };
        state.storage.party.pokemon[0] = Some(player);
        state
    };

    let mut miss = make_state();
    let miss_trace = divider_trace_for_sub_values([1]);
    let mut miss_divider = ReplayDivider::new(miss_trace.samples);
    data.resolve_active_wild_battle_run_with_divider(&mut miss, &mut miss_divider)
        .expect("nonzero low-nibble gate skips route update");
    assert_eq!(miss_divider.remaining(), 0);
    assert_eq!(miss.roaming_pokemon[0].map_number, 1);
    assert_eq!(
        miss.roaming_map_history,
        crystal_core::state::RoamingMapHistory::default()
    );

    let mut hit = make_state();
    let hit_trace = divider_trace_for_sub_values([0, 0, 1]);
    let mut hit_divider = ReplayDivider::new(hit_trace.samples);
    data.resolve_active_wild_battle_run_with_divider(&mut hit, &mut hit_divider)
        .expect("zero low-nibble gate runs exact route update");
    assert_eq!(hit_divider.remaining(), 0);
    assert_eq!(hit.roaming_pokemon[0].map_number, 2);
    assert_eq!(hit.roaming_map_history.current_map_number, 1);
}

#[test]
fn nonroaming_link_battle_end_uses_link_gate_then_divider_for_routes() {
    let mut data = GameDataSet {
        roaming_pokemon: roaming_catalog_for_tests("NEW_MON", "NEW_MON"),
        runtime_map_metadata: [(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "Route29"),
        )]
        .into_iter()
        .collect(),
        ..GameDataSet::default()
    };
    data.battle_escape_rules = test_battle_escape_rules();
    data.battle_stat_multipliers = test_battle_stat_multipliers();
    let enemy = Pokemon::new_for_tests(species(), 20, Dv::default());
    let inactive = crystal_core::state::RoamingPokemonState {
        map_group: data.roaming_pokemon.inactive_map.map_group,
        map_number: data.roaming_pokemon.inactive_map.map_number,
        ..crystal_core::state::RoamingPokemonState::default()
    };
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
        roaming_pokemon: [
            crystal_core::state::RoamingPokemonState {
                species: Some("NEW_MON".to_string()),
                level: 40,
                map_group: 1,
                map_number: 1,
                hp: 20,
                dvs_be: [0, 0],
            },
            inactive.clone(),
            inactive,
        ],
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(Pokemon::new_for_tests(species(), 20, Dv::default()));
    state.link_session.link_mode = 1;
    state.link_session.battle_random = Some(crystal_core::random::LinkBattleRandomState {
        seeds: [1, 2, 3, 4, 5, 6, 7, 8, 0x10, 10],
        count: 8,
    });
    let trace = divider_trace_for_sub_values([0, 1]);
    let mut divider = ReplayDivider::new(trace.samples);

    data.resolve_active_wild_battle_run_with_divider(&mut state, &mut divider)
        .expect("link gate uses link RNG and route update uses ordinary RNG");

    assert_eq!(divider.remaining(), 0);
    assert_eq!(state.roaming_pokemon[0].map_number, 2);
    assert_eq!(
        state
            .link_session
            .battle_random
            .as_ref()
            .expect("link random remains active")
            .count,
        0
    );
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
fn active_battle_ball_throw_rejects_truncated_divider_trace_before_mutation() {
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
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(player);
    state
        .bag
        .add_item(&ball, 1)
        .expect("add ball to bag for capture test");
    let before = state.clone();
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
                divider_trace: RuntimeDividerTrace::new([]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("truncated ball throw divider trace must reject");

    assert!(error.to_string().contains("divider replay exhausted"));
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
    state.storage.pc_boxes[0] = current_box;
    state
        .bag
        .add_item(&ball, 1)
        .expect("add ball to full-storage capture test");

    let outcome = data
        .throw_ball_at_active_battle(&mut state, "POKE_BALL")
        .expect("full storage is a visible blocked capture outcome");

    assert!(outcome.blocked);
    assert!(outcome.storage_full);
    assert!(!outcome.caught);
    assert_eq!(outcome.animation_shakes, 0);
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
    let mut data = GameDataSet::default();
    data.roaming_pokemon = roaming_catalog_for_tests("NEW_MON", "NEW_MON");
    data.runtime_map_metadata.insert(
        "ROUTE_29".to_string(),
        test_runtime_map_metadata("ROUTE_29", "Route29"),
    );
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
        ..GameState::default()
    };

    let mut party_state = make_state();
    party_state.storage.party.pokemon[0] = Some(pokemon.clone());
    let party_completion = data
        .complete_active_wild_capture(
            &mut party_state,
            &outcome,
            Some("SPARKY"),
            &mut crystal_core::random::ReplayDivider::new([0, 255]),
        )
        .expect("complete named party capture");
    assert_eq!(
        party_completion.stored.as_ref().unwrap().pokemon.nickname,
        "SPARKY"
    );
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
            &mut crystal_core::random::ReplayDivider::new([0, 255]),
        )
        .expect("complete named PC capture");
    let stored = pc_completion.stored.as_ref().unwrap();
    assert_eq!(stored.pokemon.nickname, "BOXMON");
    assert_eq!(
        pc_state.storage.pc_boxes[0].pokemon[0]
            .as_ref()
            .unwrap()
            .nickname,
        "BOXMON"
    );
    assert_eq!(pc_state.storage.pc_boxes[0].nicknames[0], "BOXMON");
}

#[test]
fn catching_a_transformed_wild_pokemon_materializes_ditto() {
    let mut data = GameDataSet::default();
    data.roaming_pokemon = roaming_catalog_for_tests("NEW_MON", "NEW_MON");
    data.runtime_map_metadata.insert(
        "ROUTE_29".to_string(),
        test_runtime_map_metadata("ROUTE_29", "Route29"),
    );
    data.growth_rates = crystal_core::systems::experience::crystal_growth_rate_catalog_for_tests();
    let mut ditto_species = species();
    ditto_species.id = "DITTO".to_string();
    ditto_species.int_id = 132;
    ditto_species.base_stats = BaseStats::new(48, 48, 48, 48, 48, 48);
    data.pokemon.insert("DITTO".to_string(), ditto_species);
    data.learnsets.insert("DITTO".to_string(), Vec::new());

    let player = crystal_core::models::Pokemon::new_for_tests(
        species(),
        20,
        crystal_core::models::Dv::from_non_hp(1, 2, 3, 4),
    );
    let mut enemy = crystal_core::models::Pokemon::new_for_tests(
        species(),
        12,
        crystal_core::models::Dv::from_non_hp(9, 8, 7, 6),
    );
    enemy.hp = 7;
    enemy.status = Some("POISON".to_string());
    let mut combat =
        crystal_core::battle::turn::BattleCombatState::new(player.clone(), enemy.clone());
    combat.enemy_transform = Some(crystal_core::battle::turn::BattleTransformState {
        species: player.species.clone(),
        dvs: player.dvs,
        moves: player.moves.clone(),
        stat_boosts: player.stat_boosts.clone(),
        attack: player.attack,
        defense: player.defense,
        speed: player.speed,
        special_attack: player.special_attack,
        special_defense: player.special_defense,
    });
    let mut state = GameState {
        battle: BattleMemory::Wild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            battle_music: "MUSIC_JOHTO_WILD_BATTLE".to_string(),
            map_name: "Route29".to_string(),
            roaming_slot: None,
            enemy_pokemon: enemy.clone(),
            enemy_party: vec![enemy.clone()],
        },
        battle_active_party_index: Some(0),
        battle_active_enemy_party_index: Some(0),
        ..GameState::default()
    };
    state.storage.party.pokemon[0] = Some(player);
    state.script_runtime.active_battle_combat = Some(combat);
    let outcome = CaptureOutcome {
        caught: true,
        blocked: false,
        storage_full: false,
        wobble_count: 4,
        animation_shakes: 4,
        final_catch_rate: u8::MAX,
        ball_id: Some("POKE_BALL".to_string()),
    };

    let completion = data
        .complete_active_wild_capture(
            &mut state,
            &outcome,
            None,
            &mut crystal_core::random::ReplayDivider::new([0, 255]),
        )
        .expect("complete transformed capture");
    let caught = &completion
        .stored
        .expect("stored transformed capture")
        .pokemon;

    assert_eq!(caught.species.id, "DITTO");
    assert_eq!(caught.level, enemy.level);
    assert_eq!(caught.dvs, enemy.dvs);
    assert_eq!(caught.hp, enemy.hp);
    assert_eq!(caught.status, enemy.status);
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
                source: RuntimePokemonStorageLocation::Box {
                    box_index: 0,
                    slot: 0,
                },
                target: RuntimePokemonStorageLocation::Box {
                    box_index: 1,
                    slot: 1,
                },
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect("move inserts before the selected destination");
    let RuntimeMutationResult::PcPokemonMoved(outcome) = applied.result else {
        panic!("expected PC Pokemon move result");
    };
    assert_eq!(
        outcome.target,
        RuntimePokemonStorageLocation::Box {
            box_index: 1,
            slot: 1
        }
    );
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
    let applied = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::MovePcPokemonWithoutMail(RuntimePcMoveCommand {
                source: RuntimePokemonStorageLocation::Box {
                    box_index: 1,
                    slot: 0,
                },
                target: RuntimePokemonStorageLocation::Box {
                    box_index: 1,
                    slot: 2,
                },
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect("same-box insertion adjusts the destination after removal");
    let RuntimeMutationResult::PcPokemonMoved(outcome) = applied.result else {
        panic!("expected same-box PC Pokemon move result");
    };
    assert_eq!(
        outcome.target,
        RuntimePokemonStorageLocation::Box {
            box_index: 1,
            slot: 1
        }
    );
    assert_eq!(
        state.storage.pc_boxes[1]
            .pokemon
            .iter()
            .flatten()
            .map(|pokemon| pokemon.nickname.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "C", "D"]
    );
    state
        .storage
        .validate_metadata()
        .expect("compact moved boxes");
}

#[test]
fn move_pokemon_without_mail_supports_every_asm_party_box_path() {
    fn named_pokemon(name: &str) -> crystal_core::models::Pokemon {
        let mut pokemon = crystal_core::models::Pokemon::new_for_tests(
            species(),
            20,
            crystal_core::models::Dv::default(),
        );
        pokemon.nickname = name.to_string();
        pokemon
    }

    let mut data = GameDataSet::default();
    data.moves.insert("TACKLE".to_string(), test_move("TACKLE"));
    let mut state = GameState::default();
    let mut first = named_pokemon("PARTY_A");
    first.moves.push(crystal_core::models::LearnedMove {
        name: "TACKLE".to_string(),
        current_pp: 1,
        pp_ups: 1,
    });
    assert!(state.storage.party.add_pokemon(first));
    assert!(state.storage.party.add_pokemon(named_pokemon("PARTY_B")));
    let mut pc_box = PcBox::new(0);
    assert!(pc_box.add_pokemon(named_pokemon("BOX_A")));
    state.storage.pc_boxes = vec![pc_box];
    state.sync_party_from_storage();
    let mut session = OverworldSession::with_events_and_objects(
        OverworldMapData {
            name: "PcPartyMoveTest".to_string(),
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
    let mut apply_move = |state: &mut GameState, source, target| {
        data.apply_runtime_mutation_command(
            state,
            &mut session,
            RuntimeMutationCommand::MovePcPokemonWithoutMail(RuntimePcMoveCommand {
                source,
                target,
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
    };

    apply_move(
        &mut state,
        RuntimePokemonStorageLocation::Party { slot: 0 },
        RuntimePokemonStorageLocation::Box {
            box_index: 0,
            slot: 1,
        },
    )
    .expect("party to box");
    assert_eq!(
        state.storage.party.pokemon[0].as_ref().unwrap().nickname,
        "PARTY_B"
    );
    let deposited = state.storage.pc_boxes[0].pokemon[1].as_ref().unwrap();
    assert_eq!(deposited.nickname, "PARTY_A");
    assert_eq!(deposited.moves[0].current_pp, 42);

    apply_move(
        &mut state,
        RuntimePokemonStorageLocation::Box {
            box_index: 0,
            slot: 0,
        },
        RuntimePokemonStorageLocation::Party { slot: 1 },
    )
    .expect("box to party");
    apply_move(
        &mut state,
        RuntimePokemonStorageLocation::Party { slot: 0 },
        RuntimePokemonStorageLocation::Party { slot: 2 },
    )
    .expect("party to party");
    assert_eq!(
        state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .map(|pokemon| pokemon.nickname.as_str())
            .collect::<Vec<_>>(),
        vec!["BOX_A", "PARTY_B"]
    );
    state
        .storage
        .validate_metadata()
        .expect("compact party and box");
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
    data.pokemon.insert(dragonair.id.clone(), dragonair.clone());
    data.pokemon.insert(dragonite.id.clone(), dragonite.clone());
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
    data.evolutions.0.insert(dragonite.id.clone(), Vec::new());
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
    let pending = state
        .pending_move_learn
        .as_ref()
        .expect("Wing Attack prompt");
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
    data.require_no_existing_pending_move_learn_for_item_effect(&clear_state, 0, &pending_effect)
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
        path: "content-packs/test/sfx/SFX_TACKLE.pcm".to_string(),
        kind: ModpackAudioKind::SoundEffect,
        source: ModpackAudioSource::Pcm,
        sfx_priority: Some(0x41),
        pcm_format: None,
        pcm_frame_count: None,
        payload_hash: None,
        loop_start_sample: None,
        loop_end_sample: None,
        midi_program: None,
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
        path: "content-packs/test/music/MUSIC_YOUNGSTER_ENCOUNTER.pcm".to_string(),
        kind: ModpackAudioKind::Music,
        source: ModpackAudioSource::Pcm,
        sfx_priority: None,
        pcm_format: None,
        pcm_frame_count: None,
        payload_hash: None,
        loop_start_sample: None,
        loop_end_sample: None,
        midi_program: None,
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
fn verifier_requires_exact_new_game_money_constants() {
    let mut data = GameDataSet {
        battle_reward_rules: test_battle_reward_rules(),
        ..GameDataSet::default()
    };
    data.currency_constants
        .0
        .insert("MOM_MONEY".to_string(), 2_301);

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_start_money_constant"
            && diagnostic.subject == "currency_constants:START_MONEY"
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "mismatched_mom_money_constant"
            && diagnostic.subject == "currency_constants:MOM_MONEY"
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
fn escape_rope_transition_commit_is_atomic_when_destination_rejects() {
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

    let prepared = data
        .use_bag_escape_rope_in_session(&mut state, &mut overworld, "ESCAPE_ROPE", &music_ids)
        .expect("source item use prepares the later warp boundary");

    assert!(prepared.item_use.consumed);
    assert_eq!(state.bag.quantity(&data.items["ESCAPE_ROPE"]), 0);
    assert_eq!(state.script_runtime.item_use_events.len(), 1);
    assert!(state.script_runtime.pending_field_travel.is_some());
    assert_eq!(overworld.map.name, "SourceCave");
    assert_eq!(overworld.player.tile, TilePosition { x: 0, y: 0 });
    assert_eq!(overworld.frame, 17);
    let state_before_commit = state.clone();
    let overworld_before_commit = overworld.clone();

    let error = data
        .commit_pending_field_travel(&mut state, &mut overworld, &music_ids)
        .expect_err("out-of-bounds destination must reject at the warp boundary");

    assert!(
        format!("{error:#}")
            .contains("runtime player tile (5, 5) is outside compiled map EscapeDest"),
        "{error:#}"
    );
    assert_eq!(state, state_before_commit);
    assert_eq!(overworld, overworld_before_commit);
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
        diagnostic.code == "unknown_field_item_id" && diagnostic.subject == "field_moves:bicycle"
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
fn verifier_rejects_story_key_rules_that_diverge_from_compiled_map_content() {
    let mut data = GameDataSet {
        field_moves: test_field_move_catalog(),
        ..GameDataSet::default()
    };
    let mut card_key = test_item("CARD_KEY");
    card_key.effect = "WRONG_EFFECT".to_string();
    data.items.insert("CARD_KEY".to_string(), card_key);

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "mismatched_story_key_effect"
            && diagnostic.subject == "field_moves:card_key"
    }));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_story_key_map" && diagnostic.subject == "field_moves:card_key"
    }));

    data.items.get_mut("CARD_KEY").expect("card key").effect = "CARD_KEY".to_string();
    let mut radio_tower = test_map_module("RadioTower3F", "RADIO_TOWER_3F", None);
    radio_tower.attributes.width = 10;
    radio_tower.attributes.height = 10;
    data.maps.insert("RadioTower3F".to_string(), radio_tower);

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_story_key_script"
            && diagnostic.subject == "field_moves:card_key"
    }));

    data.maps
        .get_mut("RadioTower3F")
        .expect("radio tower")
        .scripts
        .insert("CardKeySlotScript".to_string(), serde_json::json!([]));
    data.field_moves.card_key.target_tile = TilePosition::new(20, 2);

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "story_key_target_out_of_bounds"
            && diagnostic.subject == "field_moves:card_key"
    }));
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
fn verifier_requires_source_home_spawn_constant() {
    let legacy_error = serde_json::from_value::<RuntimeTitleScreen>(serde_json::json!({
        "new_game_spawn_identifier": 0,
        "title_music": "MUSIC_TITLE"
    }))
    .expect_err("legacy caller-authored title spawn must be rejected");
    assert!(
        legacy_error
            .to_string()
            .contains("unknown field `new_game_spawn_identifier`"),
        "{legacy_error}"
    );

    let mut data = GameDataSet {
        runtime_title_screen: RuntimeTitleScreen {
            title_music: Some("MUSIC_TITLE".to_string()),
            program: RuntimePresentationProgram::default(),
        },
        ..GameDataSet::default()
    };
    data.audio.push(ModpackAudioAsset {
        id: "MUSIC_TITLE".to_string(),
        path: "content-packs/test/music/MUSIC_TITLE.pcm".to_string(),
        kind: ModpackAudioKind::Music,
        source: ModpackAudioSource::Pcm,
        sfx_priority: None,
        pcm_format: None,
        pcm_frame_count: None,
        payload_hash: None,
        loop_start_sample: None,
        loop_end_sample: None,
        midi_program: None,
    });

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "missing_runtime_title_spawn_identifier"
            && diagnostic.subject == "SPAWN_HOME"
    }));
}

#[test]
fn verifier_requires_title_screen_music_declared_by_pack() {
    let data = GameDataSet {
        runtime_title_screen: RuntimeTitleScreen {
            title_music: None,
            program: RuntimePresentationProgram::default(),
        },
        story_event_script_constants: StoryEventScriptConstants {
            global: BTreeMap::from([("SPAWN_HOME".to_string(), 0)]),
            maps: BTreeMap::new(),
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
fn verifier_rejects_source_home_spawn_missing_from_spawn_table() {
    let mut data = GameDataSet {
        runtime_title_screen: RuntimeTitleScreen {
            title_music: Some("MUSIC_TITLE".to_string()),
            program: RuntimePresentationProgram::default(),
        },
        story_event_script_constants: StoryEventScriptConstants {
            global: BTreeMap::from([("SPAWN_HOME".to_string(), 0)]),
            maps: BTreeMap::new(),
        },
        ..GameDataSet::default()
    };
    data.audio.push(ModpackAudioAsset {
        id: "MUSIC_TITLE".to_string(),
        path: "content-packs/test/music/MUSIC_TITLE.pcm".to_string(),
        kind: ModpackAudioKind::Music,
        source: ModpackAudioSource::Pcm,
        sfx_priority: None,
        pcm_format: None,
        pcm_frame_count: None,
        payload_hash: None,
        loop_start_sample: None,
        loop_end_sample: None,
        midi_program: None,
    });

    let report = verify_game_data(
        &AssetRoot::new(repository_root_for_tests()),
        &data,
        &PlayabilityRules::default(),
    );

    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "unknown_runtime_title_spawn_identifier"
            && diagnostic.subject == "SPAWN_HOME"
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
        path: "content-packs/test/music/MUSIC_POKEMON_MARCH.pcm".to_string(),
        kind: ModpackAudioKind::Music,
        source: ModpackAudioSource::Pcm,
        sfx_priority: None,
        pcm_format: None,
        pcm_frame_count: None,
        payload_hash: None,
        loop_start_sample: None,
        loop_end_sample: None,
        midi_program: None,
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
    let mut data = AssetRoot::new(repository_root_for_tests())
        .load_base_game_data()
        .expect("load exact global Rock Smash scripts");
    data.field_encounters.remove("Route40");
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
            RuntimeMutationCommand::ResolveRockMonEncounter(RuntimeRockMonEncounterCommand {
                command: command.clone(),
                divider_trace: RuntimeDividerTrace::new([]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect("a map absent from RockMonMaps is an exact zero-read miss");
    let RuntimeMutationResult::RockMonEncounterResolved(no_table) = no_table.result else {
        panic!("expected RockMonEncounter result");
    };
    assert_eq!(no_table.chance_roll, None);
    assert_eq!(
        state.random_state,
        CrystalRandomState {
            add: 0x12,
            sub: 0x34
        }
    );
    assert_eq!(
        state.script_runtime.memory.get("wTempWildMonSpecies"),
        Some(&"0".to_string())
    );
    assert_eq!(
        state.script_runtime.memory.get("wCurPartyLevel"),
        Some(&"0".to_string())
    );

    let before_no_table_tail = state.clone();
    let no_table_tail = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ResolveRockMonEncounter(RuntimeRockMonEncounterCommand {
                command: command.clone(),
                divider_trace: RuntimeDividerTrace::new([99]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("a zero-read miss must reject an injected divider tail");
    assert!(
        no_table_tail
            .to_string()
            .contains("1 unconsumed samples after 0 reads")
    );
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
            RuntimeMutationCommand::ResolveRockMonEncounter(RuntimeRockMonEncounterCommand {
                command: command.clone(),
                divider_trace: RuntimeDividerTrace::new([255, 0, 89]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("a truncated conditional entry draw must reject atomically");
    assert!(
        short
            .to_string()
            .contains("divider replay exhausted after 3 samples")
    );
    assert_eq!(state, before_short);

    let before_miss_tail = state.clone();
    let miss_tail = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ResolveRockMonEncounter(RuntimeRockMonEncounterCommand {
                command: command.clone(),
                divider_trace: RuntimeDividerTrace::new([3, 0, 77]),
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("a chance miss must reject an unused entry-roll byte");
    assert!(
        miss_tail
            .to_string()
            .contains("1 unconsumed samples after 2 reads")
    );
    assert_eq!(state, before_miss_tail);

    let hit = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ResolveRockMonEncounter(RuntimeRockMonEncounterCommand {
                command,
                divider_trace: RuntimeDividerTrace::new([255, 0, 89, 0]),
            }),
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

#[test]
fn wild_battle_start_rejects_origin_outside_runtime_map_bounds_before_species_lookup() {
    let data = GameDataSet {
        maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
        ..GameDataSet::default()
    };
    let error = data
        .validate_runtime_map_tile(
            "wild battle encounter roll",
            "Route29",
            TilePosition::new(2, 0),
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
    let error = data
        .validate_runtime_map_tile(
            "wild battle encounter roll",
            "Route29",
            TilePosition::new(4, 0),
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
    let state = GameState::default();

    let error = data
        .validate_runtime_map_tile(
            "wild battle encounter roll",
            "Route29",
            TilePosition::new(2, 0),
        )
        .expect_err("invalid wild battle origin must fail before mutation");

    assert!(
            format!("{error:#}").contains(
                "wild battle encounter roll tile (2, 0) is outside compiled map Route29 runtime tile bounds 2x2"
            ),
            "{error:#}"
        );
    assert_eq!(state.battle, BattleMemory::Inactive);
}

#[test]
fn fishing_battle_rejects_origin_outside_runtime_map_bounds() {
    let data = GameDataSet {
        maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
        ..GameDataSet::default()
    };
    let mut state = GameState::default();

    let mut divider = ReplayDivider::new([]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);
    let error = data
        .start_fishing_battle_with_rng(
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
            &mut rng,
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

    let mut divider = ReplayDivider::new([]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);
    let error = data
        .start_fishing_battle_with_rng(
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
            &mut rng,
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
fn fishing_start_preserves_pret_battle_type() {
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
    let mut fishing_state = GameState::default();
    let mut fishing_divider = ReplayDivider::new([0; 6]);
    let mut fishing_rng = CrystalRandom::new(fishing_state.random_state, &mut fishing_divider);
    let fishing = data
        .start_fishing_battle_with_rng(
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
            &mut fishing_rng,
        )
        .expect("fishing battle");
    assert_eq!(fishing.battle_type, "BATTLETYPE_FISH");
    assert!(matches!(
        fishing_state.battle,
        BattleMemory::Wild { ref battle_type, .. } if battle_type == "BATTLETYPE_FISH"
    ));
}

#[test]
fn wild_encounter_after_step_rejects_out_of_bounds_session_tile_before_rng_commit() {
    let data = GameDataSet {
        maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
        tilesets: BTreeMap::from([("johto".to_string(), test_tileset_definition())]),
        ..GameDataSet::default()
    };
    let mut state = GameState::default();
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
}

#[test]
fn missing_wild_table_consumes_the_asm_zero_rate_random_call() {
    let mut tileset = test_tileset_definition();
    *tileset.collision.get_mut("1").expect("fixture metatile") = vec!["TALL_GRASS".to_string(); 4];
    let data = GameDataSet {
        maps: map_payload(vec![test_map_module("Route29", "ROUTE_29", None)]),
        tilesets: BTreeMap::from([("johto".to_string(), tileset)]),
        runtime_map_metadata: BTreeMap::from([(
            "ROUTE_29".to_string(),
            test_runtime_map_metadata("ROUTE_29", "Route29"),
        )]),
        ..GameDataSet::default()
    };
    let session = data
        .overworld_session("Route29", TilePosition::new(0, 0), 0)
        .expect("grass session");
    let mut state = GameState::default();
    state.random_state = CrystalRandomState { add: 0xff, sub: 0 };
    let mut divider = ReplayDivider::new([0, 0]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    let roll = data
        .check_wild_encounter_after_step(&mut state, &session, &mut rng)
        .expect("zero-rate check")
        .expect("grass reaches TryWildEncounter");

    assert_eq!(roll.threshold, 0);
    assert!(roll.resolved.is_none());
    assert_eq!(divider.remaining(), 0);
}

#[test]
fn dungeon_environment_allows_encounters_on_ordinary_tower_floor() {
    let data = AssetRoot::new(repository_root_for_tests())
        .load_base_game_data()
        .expect("load base game data");
    let metadata = data
        .runtime_map_metadata_for_name("TinTower7F")
        .expect("Tin Tower metadata");
    let runtime_width = i16::try_from(metadata.width * 2).expect("map width");
    let runtime_height = i16::try_from(metadata.height * 2).expect("map height");
    let session = (0..runtime_height)
        .flat_map(|y| (0..runtime_width).map(move |x| TilePosition::new(x, y)))
        .find_map(|tile| {
            let session = data.overworld_session("TinTower7F", tile, 0).ok()?;
            (session
                .current_encounter_surface_checked_with_land_encounters(false)
                .is_ok_and(|surface| surface.is_none())
                && session
                    .current_encounter_surface_checked_with_land_encounters(true)
                    .is_ok_and(|surface| {
                        surface == Some(crystal_core::world::encounters::EncounterSurface::Grass)
                    }))
            .then_some(session)
        })
        .expect("Tin Tower ordinary non-ice floor tile");
    let mut state = GameState::default();
    let mut divider = ReplayDivider::new([0; 64]);
    let mut rng = CrystalRandom::new(state.random_state, &mut divider);

    let encounter = data
        .check_wild_encounter_after_step(&mut state, &session, &mut rng)
        .expect("check Tin Tower encounter");

    assert!(
        encounter.is_some(),
        "DUNGEON floor must reach its land table"
    );
    let battle = data
        .start_resolved_wild_encounter_after_step(&mut state, &session, &encounter, &mut rng)
        .expect("start Tin Tower encounter");
    assert!(battle.is_some());
}

#[test]
fn active_yanma_swarm_replaces_route_35_grass_table() {
    let data = AssetRoot::new(repository_root_for_tests())
        .load_base_game_data()
        .expect("load base game data");
    assert_eq!(
        data.require_wild_encounters_for_map("DarkCaveVioletEntrance")
            .expect("Dark Cave encounters")
            .swarm_overrides
            .get("SWARM_DUNSPARCE")
            .expect("exported Dunsparce swarm")
            .engine_flag,
        "ENGINE_DUNSPARCE_SWARM"
    );
    assert_eq!(
        data.require_wild_encounters_for_map("Route35")
            .expect("Route 35 encounters")
            .swarm_overrides
            .get("SWARM_YANMA")
            .expect("exported Yanma swarm")
            .engine_flag,
        "ENGINE_YANMA_SWARM"
    );
    let metadata = data
        .runtime_map_metadata_for_name("Route35")
        .expect("Route 35 metadata");
    let runtime_width = i16::try_from(metadata.width * 2).expect("map width");
    let runtime_height = i16::try_from(metadata.height * 2).expect("map height");
    let session = (0..runtime_height)
        .flat_map(|y| (0..runtime_width).map(move |x| TilePosition::new(x, y)))
        .find_map(|tile| {
            let session = data.overworld_session("Route35", tile, 0).ok()?;
            session
                .current_encounter_surface_checked()
                .is_ok_and(|surface| {
                    surface == Some(crystal_core::world::encounters::EncounterSurface::Grass)
                })
                .then_some(session)
        })
        .expect("Route 35 grass tile");
    let target = crystal_core::state::SwarmMapTarget {
        map_id: "ROUTE_35".to_string(),
        map_group: Some(metadata.group_id),
        map_number: Some(metadata.map_id),
    };
    let resolve_slot_two = |state: &mut GameState| {
        // Rate 0 succeeds, roamer selector 0 misses, and percent 61 selects
        // grass slot 2: PSYDUCK normally at this fixture's time, YANMA during
        // the source swarm.
        let mut divider = ReplayDivider::new([0, 0, 0, 0, 0, 196]);
        let mut rng = CrystalRandom::new(state.random_state, &mut divider);
        let roll = data
            .check_wild_encounter_after_step(state, &session, &mut rng)
            .expect("check Route 35 encounter")
            .expect("Route 35 grass encounter check");
        assert_eq!(divider.remaining(), 0);
        roll.resolved.expect("encounter resolves")
    };

    let mut flag_only = GameState::default();
    flag_only
        .flags
        .set_engine_flag("ENGINE_YANMA_SWARM", true)
        .expect("set Yanma swarm flag");
    assert_eq!(
        resolve_slot_two(&mut flag_only).encounter.species,
        "PSYDUCK",
        "the engine flag alone does not satisfy the stored swarm map check"
    );

    let mut target_only = GameState::default();
    target_only
        .swarms
        .active
        .insert("SWARM_YANMA".to_string(), target.clone());
    assert_eq!(
        resolve_slot_two(&mut target_only).encounter.species,
        "PSYDUCK",
        "the stored swarm map alone does not satisfy the engine flag check"
    );

    let mut active = flag_only;
    active
        .swarms
        .active
        .insert("SWARM_YANMA".to_string(), target);
    let encounter = resolve_slot_two(&mut active);

    assert_eq!(encounter.slot, 2);
    assert_eq!(encounter.encounter.species, "YANMA");
    assert_eq!(encounter.level, 12);
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
        pokemon: [("RAIKOU".to_string(), raikou), ("ENTEI".to_string(), entei)]
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
        currency_constants: CurrencyCatalog(BTreeMap::from([
            ("START_MONEY".to_string(), 3_000),
            ("MOM_MONEY".to_string(), 2_300),
        ])),
        pokegear_landmarks: map_name_sign_landmarks_for_tests(["Route29"]),
        ..GameDataSet::default()
    };

    let (mut state, _) = data
        .start_overworld_session_from_spawn(&spawn, &BTreeSet::new())
        .expect("start exact new-game spawn");
    assert_eq!(state.map_name_sign.current_landmark, 2);
    assert_eq!(state.map_name_sign.previous_landmark, 2);
    assert_eq!(state.map_name_sign.flags, 0);
    assert_eq!(state.map_name_sign.timer, 0);
    assert_eq!(state.money, 3_000);
    assert_eq!(state.mom_item_trigger_balance, 2_300);
    assert_eq!(
        state.script_runtime.variables.get("_rival_name"),
        Some(&"???".to_string())
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_moms_name")
            .map(String::as_str),
        Some("MOM")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_reds_name")
            .map(String::as_str),
        Some("RED")
    );
    assert_eq!(
        state
            .script_runtime
            .variables
            .get("_greens_name")
            .map(String::as_str),
        Some("GREEN")
    );
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wDecoBed")
            .map(String::as_str),
        Some("DECO_FEATHERY_BED")
    );
    assert_eq!(
        state
            .script_runtime
            .memory
            .get("wDecoPoster")
            .map(String::as_str),
        Some("DECO_TOWN_MAP")
    );
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
        .validate_saved_wild_battle_origin_references("BATTLETYPE_NORMAL", "OtherMap", &enemy)
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
    let headbutt_error = serde_json::from_value::<RuntimeFieldPartyCommand>(serde_json::json!({
        "party_index": 0,
        "surface": "grass"
    }))
    .expect_err("HEADBUTT command must not carry a surface payload");
    assert!(
        headbutt_error
            .to_string()
            .contains("unknown field `surface`"),
        "{headbutt_error}"
    );

    let sweet_scent_error =
        serde_json::from_value::<RuntimeSweetScentEncounterCommand>(serde_json::json!({
            "command": {
                "map_name": "RuntimeMap",
                "source_script": ".SweetScent@SweetScentFromMenu",
                "command_index": 5
            },
            "divider_trace": { "samples": [] },
            "party_index": 0
        }))
        .expect_err("SweetScentEncounter must not carry the menu-time party index");
    assert!(
        sweet_scent_error
            .to_string()
            .contains("unknown field `party_index`"),
        "{sweet_scent_error}"
    );

    let missing_trace =
        serde_json::from_value::<RuntimeSweetScentEncounterCommand>(serde_json::json!({
            "command": {
                "map_name": "RuntimeMap",
                "source_script": ".SweetScent@SweetScentFromMenu",
                "command_index": 5
            }
        }))
        .expect_err("Sweet Scent commands must declare their exact divider trace");
    assert!(
        missing_trace
            .to_string()
            .contains("missing field `divider_trace`"),
        "{missing_trace}"
    );
}

#[test]
fn runtime_day_care_commands_use_exact_action_payloads() {
    let missing_trace = serde_json::from_value::<RuntimeDayCareCommand>(serde_json::json!({
        "caretaker": "man",
        "action": "inspect",
        "party_index": null
    }))
    .expect_err("Day Care commands must carry an exact divider trace");
    assert!(
        missing_trace
            .to_string()
            .contains("missing field `divider_trace`"),
        "{missing_trace}"
    );

    let deposit = RuntimeDayCareCommand {
        caretaker: RuntimeDayCareCaretaker::Man,
        action: RuntimeDayCareAction::Deposit,
        party_index: Some(0),
        divider_trace: RuntimeDividerTrace::new([]),
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
        divider_trace: RuntimeDividerTrace::new([]),
    };
    let missing_error = runtime_day_care_party_slot(&missing_slot)
        .expect_err("deposit must carry the party slot consumed by DayCareMan");
    assert!(
        format!("{missing_error:#}").contains("Day Care deposit command requires party_index"),
        "{missing_error:#}"
    );

    for action in [
        RuntimeDayCareAction::Open,
        RuntimeDayCareAction::Withdraw,
        RuntimeDayCareAction::Inspect,
    ] {
        let command = RuntimeDayCareCommand {
            caretaker: RuntimeDayCareCaretaker::Lady,
            action,
            party_index: Some(0),
            divider_trace: RuntimeDividerTrace::new([]),
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
fn saved_day_care_state_uses_the_source_byte_and_no_host_step_counters() {
    let encoded = serde_json::to_value(GameState::default()).expect("serialize game state");

    let mut oversized_countdown = encoded.clone();
    oversized_countdown["day_care"]["steps_until_next_egg"] = serde_json::json!(256);
    let error = serde_json::from_value::<GameState>(oversized_countdown)
        .expect_err("wStepsToEgg must reject values above one byte");
    assert!(error.to_string().contains("invalid value"), "{error}");

    for (scope, field, value) in [
        ("day_care", "steps_since_last_egg", serde_json::json!(1)),
        ("day_care", "last_interaction", serde_json::json!(null)),
        ("man", "initial_experience", serde_json::json!(0)),
        ("man", "initial_level", serde_json::json!(5)),
        ("man", "steps", serde_json::json!(1)),
    ] {
        let mut stale = encoded.clone();
        if scope == "day_care" {
            stale["day_care"][field] = value;
        } else {
            stale["day_care"][scope][field] = value;
        }
        let error = serde_json::from_value::<GameState>(stale)
            .expect_err("non-source Day Care counters must reject");
        assert!(
            error
                .to_string()
                .contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}

#[test]
fn saved_bug_contest_state_rejects_parallel_host_mirrors() {
    let encoded = serde_json::to_value(GameState::default()).expect("serialize game state");
    for (field, value) in [
        ("last_rank", serde_json::json!(1)),
        ("last_result", serde_json::json!(0)),
        ("caught_species", serde_json::json!("SCYTHER")),
        ("caught_level", serde_json::json!(14)),
    ] {
        let mut stale = encoded.clone();
        stale["bug_contest"][field] = value;
        let error = serde_json::from_value::<GameState>(stale)
            .expect_err("Bug Contest results belong to the live script result boundary");
        assert!(
            error
                .to_string()
                .contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}

#[test]
fn saved_link_session_rejects_parallel_result_mirrors() {
    let encoded = serde_json::to_value(GameState::default()).expect("serialize game state");
    for (field, value) in [
        ("last_result", serde_json::json!(true)),
        ("failed_link_to_past", serde_json::json!(true)),
        ("quick_save_requested", serde_json::json!(true)),
        ("active_room", serde_json::json!("Colosseum")),
        ("friend_ready", serde_json::json!(true)),
    ] {
        let mut stale = encoded.clone();
        stale["link_session"][field] = value;
        let error = serde_json::from_value::<GameState>(stale)
            .expect_err("link results belong to source registers, not persistent mirrors");
        assert!(
            error
                .to_string()
                .contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}

#[test]
fn saved_link_rng_is_required_only_for_colosseum_mode() {
    for mode in [1, 2] {
        let mut state = GameState::default();
        state.link_session.link_mode = mode;
        let decoded: GameState = serde_json::from_value(
            serde_json::to_value(&state).expect("serialize non-battle link mode"),
        )
        .expect("Time Capsule and Trade modes do not consume Colosseum RNG");
        assert_eq!(decoded.link_session.link_mode, mode);
        assert!(decoded.link_session.battle_random.is_none());
    }

    let mut colosseum = GameState::default();
    colosseum.link_session.link_mode = 3;
    let error = serde_json::from_value::<GameState>(
        serde_json::to_value(&colosseum).expect("serialize Colosseum without RNG"),
    )
    .expect_err("Colosseum mode requires its synchronized RNG stream");
    assert!(
        error.to_string().contains(
            "active Colosseum session requires persisted link_session.battle_random seeds and count"
        ),
        "{error}"
    );
}

#[test]
fn saved_battle_tower_state_rejects_immediate_result_mirrors() {
    let encoded = serde_json::to_value(GameState::default()).expect("serialize game state");
    for (field, value) in [
        (
            "last_rule_failure",
            serde_json::json!("YouCantTakeAnEggText"),
        ),
        (
            "last_sprite_constant",
            serde_json::json!("SPRITE_GENTLEMAN"),
        ),
    ] {
        let mut stale = encoded.clone();
        stale["battle_tower"][field] = value;
        let error = serde_json::from_value::<GameState>(stale)
            .expect_err("Battle Tower immediate results must not become saved state");
        assert!(
            error
                .to_string()
                .contains(&format!("unknown field `{field}`")),
            "{error}"
        );
    }
}

#[test]
fn saved_script_runtime_rejects_last_special_execution_history() {
    let mut state = GameState::default();
    state.script_runtime.last_special_routine = Some("HealParty".to_string());
    let encoded = serde_json::to_value(state).expect("serialize game state");
    assert!(
        encoded["script_runtime"]
            .get("last_special_routine")
            .is_none(),
        "last special execution history is not cartridge save state"
    );

    let mut stale = encoded;
    stale["script_runtime"]["last_special_routine"] = serde_json::json!("HealParty");
    let error = serde_json::from_value::<GameState>(stale)
        .expect_err("stale execution history must not re-enter saved state");
    assert!(
        error
            .to_string()
            .contains("unknown field `last_special_routine`"),
        "{error}"
    );
}

#[test]
fn runtime_bug_contest_commands_have_no_rank_authority_and_use_exact_rng_payloads() {
    let judging_trace = RuntimeDividerTrace::new([1, 2]);
    let judging = RuntimeBugContestCommand::Judge {
        divider_trace: judging_trace.clone(),
    };
    assert_eq!(judging.divider_trace(), Some(&judging_trace));
    assert_eq!(runtime_bug_contest_action_name(judging.action()), "judge");

    let injected_error = serde_json::from_value::<RuntimeBugContestCommand>(serde_json::json!({
        "action": "judge",
        "rank": 3,
        "divider_trace": { "samples": [1, 2] }
    }))
    .expect_err("Bug Contest commands must not expose a fabricated rank authority");
    assert!(
        injected_error.to_string().contains("unknown field `rank`"),
        "{injected_error}"
    );

    let selecting_trace = RuntimeDividerTrace::new([3, 4]);
    let selecting = RuntimeBugContestCommand::SelectContestants {
        divider_trace: selecting_trace.clone(),
    };
    assert_eq!(selecting.divider_trace(), Some(&selecting_trace));

    let missing_trace_error = serde_json::from_value::<RuntimeBugContestCommand>(
        serde_json::json!({ "action": "select_contestants" }),
    )
    .expect_err("select contestants must carry the authoritative divider trace");
    assert!(
        missing_trace_error
            .to_string()
            .contains("missing field `divider_trace`"),
        "{missing_trace_error}"
    );

    let unused_trace_error =
        serde_json::from_value::<RuntimeBugContestCommand>(serde_json::json!({
            "action": "give_park_balls",
            "divider_trace": { "samples": [1, 2] }
        }))
        .expect_err("non-RNG bug contest actions must reject divider traces");
    assert!(
        unused_trace_error
            .to_string()
            .contains("unknown field `divider_trace`"),
        "{unused_trace_error}"
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
            RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand::SelectContestants {
                divider_trace: RuntimeDividerTrace::new([0]),
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
            RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand::SelectContestants {
                divider_trace: trace_with_tail,
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
            RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand::SelectContestants {
                divider_trace: divider_trace_for_sub_values([0]),
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
    let give = RuntimeShuckieCommand::Give {
        divider_trace: trace.clone(),
    };
    let RuntimeShuckieCommand::Give { divider_trace } = &give else {
        panic!("expected give command");
    };
    assert_eq!(divider_trace, &trace);

    let give_error = serde_json::from_value::<RuntimeShuckieCommand>(serde_json::json!({
        "action": "give",
        "party_index": 0,
        "divider_trace": { "samples": [1, 2, 3, 4] }
    }))
    .expect_err("GiveShuckle must not receive ignored party slot state");
    assert!(
        give_error
            .to_string()
            .contains("unknown field `party_index`"),
        "{give_error}"
    );
    let missing_rng_error =
        serde_json::from_value::<RuntimeShuckieCommand>(serde_json::json!({ "action": "give" }))
            .expect_err("GiveShuckle must declare divider boundary");
    assert!(
        missing_rng_error
            .to_string()
            .contains("missing field `divider_trace`"),
        "{missing_rng_error}"
    );

    let return_selected = RuntimeShuckieCommand::Return {
        party_index: Some(2),
    };
    let RuntimeShuckieCommand::Return { party_index } = return_selected else {
        panic!("expected return command");
    };
    assert_eq!(party_index, Some(2));

    let return_cancelled = RuntimeShuckieCommand::Return { party_index: None };
    let RuntimeShuckieCommand::Return { party_index } = return_cancelled else {
        panic!("expected cancelled return command");
    };
    assert_eq!(party_index, None);
    let unused_rng_error = serde_json::from_value::<RuntimeShuckieCommand>(serde_json::json!({
        "action": "return",
        "party_index": 2,
        "divider_trace": { "samples": [1, 2, 3, 4] }
    }))
    .expect_err("ReturnShuckie must not accept unused RNG state");
    assert!(
        unused_rng_error
            .to_string()
            .contains("unknown field `divider_trace`"),
        "{unused_rng_error}"
    );
}

#[test]
fn generic_special_routine_command_enforces_exact_divider_boundary() {
    assert!(runtime_special_routine_requires_divider_trace(
        "SampleKenjiBreakCountdown"
    ));
    assert!(!runtime_special_routine_requires_divider_trace("HealParty"));

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
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "SampleKenjiBreakCountdown".to_string(),
                divider_trace: RuntimeDividerTrace::new([0]),
            }),
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
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "SampleKenjiBreakCountdown".to_string(),
                divider_trace: RuntimeDividerTrace::new([0, 200, 77]),
            }),
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
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "SampleKenjiBreakCountdown".to_string(),
                divider_trace: RuntimeDividerTrace::new([0, 200]),
            }),
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
        CrystalRandomState { add: 0xff, sub: 56 }
    );
    assert_eq!(state.random_state, random_state_after);
    assert_eq!(state.kenji_break_timer, value);

    let legacy_field = serde_json::from_value::<RuntimeMutationCommand>(serde_json::json!({
        "kind": "apply_special_routine",
        "payload": {
            "routine": "HealParty",
            "rng_seed_after": 1
        }
    }))
    .expect_err("generic specials must reject the removed rng_seed_after field");
    assert!(
        legacy_field
            .to_string()
            .contains("unknown field `rng_seed_after`"),
        "{legacy_field}"
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
        .insert("_value".to_string(), "UNOWNPUZZLE_KABUTO".to_string());
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

    let mut alias_state = state.clone();
    alias_state
        .script_runtime
        .variables
        .insert("_value".to_string(), "KABUTO".to_string());
    let alias_before = alias_state.clone();
    let alias_error = data
        .apply_runtime_mutation_command(
            &mut alias_state,
            &mut session,
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "UnownPuzzle".to_string(),
                divider_trace: RuntimeDividerTrace {
                    samples: Vec::new(),
                },
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("bare host puzzle aliases must reject at the pack boundary");
    assert!(
        alias_error
            .to_string()
            .contains("unknown puzzle id 'KABUTO'"),
        "{alias_error}"
    );
    assert_eq!(alias_state, alias_before);

    let mut unscoped_state = state.clone();
    unscoped_state.script_runtime.variables.insert(
        "unown_layout".to_string(),
        "1,2,3,4,5,6;7,0,0,0,0,8;9,0,0,0,0,10;11,0,0,0,0,12;13,0,0,0,0,14;15,0,0,0,0,16"
            .to_string(),
    );
    unscoped_state
        .script_runtime
        .variables
        .insert("unown_action".to_string(), "noop".to_string());
    let unscoped_before = unscoped_state.clone();
    let unscoped_error = data
        .apply_runtime_mutation_command(
            &mut unscoped_state,
            &mut session,
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "UnownPuzzle".to_string(),
                divider_trace: RuntimeDividerTrace {
                    samples: Vec::new(),
                },
            }),
            &audio_ids,
            &audio_ids,
            &audio_ids,
        )
        .expect_err("unscoped host puzzle state must reject at the pack boundary");
    assert!(
        unscoped_error
            .to_string()
            .contains("KABUTO has no active layout"),
        "{unscoped_error}"
    );
    assert_eq!(unscoped_state, unscoped_before);

    let mut exact_trace = divider_trace_for_sub_values(0_u8..16);
    exact_trace.samples.pop();
    let exhausted = data
        .apply_runtime_mutation_command(
            &mut state,
            &mut session,
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "UnownPuzzle".to_string(),
                divider_trace: exact_trace,
            }),
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
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "UnownPuzzle".to_string(),
                divider_trace: trace_with_tail,
            }),
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
            RuntimeMutationCommand::ApplyRandomSpecialRoutine(RuntimeRandomSpecialRoutineCommand {
                routine: "UnownPuzzle".to_string(),
                divider_trace: divider_trace_for_sub_values(0_u8..16),
            }),
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
    assert_eq!(random_state_after, CrystalRandomState { add: 0, sub: 15 });
    assert_eq!(state.random_state, random_state_after);
    assert!(
        !state
            .script_runtime
            .variables
            .contains_key("unown_moves_KABUTO")
    );
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
    let mut state = GameState::default();
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
            RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand::Give {
                divider_trace: RuntimeDividerTrace::new([]),
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
            RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand::Give {
                divider_trace: RuntimeDividerTrace::new([0; 5]),
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
            RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand::Give {
                divider_trace: RuntimeDividerTrace::new([0; 4]),
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
    let missing_divider_trace =
        serde_json::from_value::<RuntimeOddEggCommand>(serde_json::json!({}))
            .expect_err("Odd Egg command must declare the divider trace");
    assert!(
        missing_divider_trace
            .to_string()
            .contains("missing field `divider_trace`"),
        "{missing_divider_trace}"
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
    let mut state = GameState::default();
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
                swarm_overrides: BTreeMap::new(),
                zones: Vec::new(),
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
    data.buena_password_categories = BuenaPasswordCategories { order, categories };
    let mut state = GameState::default();
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
            options: vec![
                "TODAY".to_string(),
                "TOMORROW".to_string(),
                "YESTERDAY".to_string(),
            ],
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
        None
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
