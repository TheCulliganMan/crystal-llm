#[test]
fn chikorita_battle_art_loads_real_runtime_assets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();

    let front = load_pokemon_frame(
        &asset_root,
        "CHIKORITA",
        PokemonSpriteSide::Front,
        false,
        &mut images,
    )
    .expect("load Chikorita front art");
    let back = load_pokemon_frame(
        &asset_root,
        "CHIKORITA",
        PokemonSpriteSide::Back,
        false,
        &mut images,
    )
    .expect("load Chikorita back art");

    assert_eq!(front.size, Vec2::new(40.0, 40.0));
    assert_eq!(back.size, Vec2::new(48.0, 48.0));
    assert_eq!(images.len(), 2);
}
#[test]
fn wooper_frontpic_retains_black_outline_like_typescript_renderer() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();
    let frame = load_pokemon_frame(
        &asset_root,
        "WOOPER",
        PokemonSpriteSide::Front,
        false,
        &mut images,
    )
    .expect("load Wooper front art");
    let image = images.get(&frame.handle).expect("Wooper image");
    let black_outline_pixels = image
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] == 255 && pixel[0] < 24 && pixel[1] < 24 && pixel[2] < 24)
        .count();
    let transparent_background_pixels = image
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] == 0)
        .count();
    assert!(
        black_outline_pixels > 0,
        "Wooper frontpic lost its black outline"
    );
    assert!(
        transparent_background_pixels > 0,
        "Wooper frontpic background was not keyed transparent"
    );
}

#[test]
fn emote_art_keys_its_corner_color_transparent_like_typescript_renderer() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();
    let frame = emote_frame_for_art(&mut rendered_art, &asset_root, "EMOTE_SHOCK", &mut images)
        .expect("load shock emote art");
    let image = images.get(&frame.handle).expect("shock emote image");
    let transparent_pixels = image
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] == 0)
        .count();
    let visible_pixels = image
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .count();

    assert!(
        transparent_pixels > 0,
        "emote background was not keyed transparent"
    );
    assert!(
        visible_pixels > 0,
        "emote artwork was keyed away with its background"
    );
    assert_eq!(frame.size, Vec2::splat(64.0));
}

#[test]
fn object_palette_zero_uses_compiled_sprite_default() {
    let defaults = BTreeMap::from([(String::from("SPRITE_ELM"), 5_i64)]);
    assert_eq!(
        resolve_visible_object_palette("SPRITE_ELM", 0, &defaults),
        5
    );
    assert_eq!(
        resolve_visible_object_palette("SPRITE_ELM", 3, &defaults),
        3
    );
}

#[test]
fn pokemon_colorkey_preserves_enclosed_palette_zero_pixels() {
    let mut source = image::RgbaImage::from_pixel(5, 5, image::Rgba([255, 255, 255, 255]));
    for row in 1..4 {
        for col in 1..4 {
            source.put_pixel(col, row, image::Rgba([0, 0, 0, 255]));
        }
    }
    source.put_pixel(2, 2, image::Rgba([255, 255, 255, 255]));
    let palette = [[255, 255, 255], [132, 165, 206], [140, 66, 115], [0, 0, 0]];
    let mut target = vec![0_u8; 5 * 5 * 4];
    copy_pokemon_frame_rgba(&source, 5, 5, &palette, &palette, &mut target);
    assert_eq!(
        target[3], 0,
        "border-connected background must be transparent"
    );
    let center = (2 * 5 + 2) * 4;
    assert_eq!(
        target[center + 3],
        255,
        "enclosed palette zero must remain opaque"
    );
    let outline = (1 * 5 + 1) * 4;
    assert_eq!(&target[outline..outline + 4], &[0, 0, 0, 255]);
}

#[test]
fn colorized_pokemon_png_uses_the_species_palette_indices() {
    let source_palette = [[255, 255, 255], [173, 189, 99], [24, 165, 0], [0, 0, 0]];
    let target_palette = [[255, 255, 255], [10, 20, 30], [40, 50, 60], [0, 0, 0]];
    let mut source = image::RgbaImage::from_pixel(3, 3, image::Rgba([255, 255, 255, 255]));
    source.put_pixel(1, 1, image::Rgba([24, 165, 0, 255]));
    let mut target = vec![0_u8; 3 * 3 * 4];

    copy_pokemon_frame_rgba(
        &source,
        3,
        3,
        &source_palette,
        &target_palette,
        &mut target,
    );

    let center = (1 * 3 + 1) * 4;
    assert_eq!(&target[center..center + 4], &[40, 50, 60, 255]);
}

#[test]
fn battle_player_backpic_keys_palette_zero_transparent() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();
    let frame = load_oak_intro_frame(
        &asset_root,
        "battle-player:chris_back",
        &mut images,
    )
    .expect("load Chris battle backpic");
    let image = images.get(&frame.handle).expect("Chris battle backpic image");

    assert!(
        image.data.chunks_exact(4).any(|pixel| pixel[3] == 0),
        "battle backpic background must reveal the battle canvas"
    );
    assert!(
        image.data.chunks_exact(4).any(|pixel| pixel[3] == 255),
        "battle backpic artwork must remain visible"
    );
}

#[test]
fn retained_frame_pair_rejects_either_frame_outside_session_range() {
    validate_retained_frame_pair("test", 10, 12, 10, 12).expect("boundary frames are valid");
    for (first, second) in [(9, 11), (13, 11), (11, 9), (11, 13)] {
        let error = validate_retained_frame_pair("test", first, second, 10, 12)
            .expect_err("out-of-range retained frame must reject");
        assert!(
            error
                .to_string()
                .contains("outside session frame range 10..=12"),
            "{error:#}"
        );
    }
}

#[test]
fn retained_menu_choice_order_is_strictly_increasing() {
    validate_retained_menu_choice_order(None, 10).expect("first choice frame is valid");
    validate_retained_menu_choice_order(Some(10), 11).expect("later choice frame is valid");
    for current in [9, 10] {
        let error = validate_retained_menu_choice_order(Some(10), current)
            .expect_err("non-increasing menu choice frame must reject");
        assert!(
            error
                .to_string()
                .contains("not strictly after previous choice frame 10"),
            "{error:#}"
        );
    }
}

#[test]
fn retained_input_frames_fill_missing_frames_with_zero_masks() {
    let masks = BTreeMap::from([(10, 0x10), (12, 0x80)]);
    let inputs = retained_input_frames_from_masks(10, 13, &masks).expect("retained input frames");

    assert_eq!(
        inputs
            .iter()
            .map(|input| (input.frame(), input.joypad_mask()))
            .collect::<Vec<_>>(),
        vec![(10, 0x10), (11, 0x00), (12, 0x80)]
    );

    let lockstep =
        retained_lockstep_frames_from_masks(10, 13, &masks).expect("retained lockstep frames");
    assert_eq!(lockstep.len(), 3);
    assert_eq!(lockstep[1].joypad_mask_for(LOCAL_PLAYER_ID), Some(0));
}

#[test]
fn deterministic_input_frame_uses_pre_tick_frame_from_post_tick_checksum() {
    assert_eq!(
        deterministic_input_frame_from_post_tick_checksum(&StateChecksum::new(145, 0xaabb_ccdd))
            .expect("input frame"),
        144
    );
    let error =
        deterministic_input_frame_from_post_tick_checksum(&StateChecksum::new(0, 0xaabb_ccdd))
            .expect_err("post-tick frame zero cannot produce input frame");
    assert!(
        error.to_string().contains("before post-tick frame 0"),
        "{error:#}"
    );
}

#[test]
fn retained_battle_action_order_is_strictly_increasing() {
    validate_retained_battle_action_order(None, 10).expect("first action frame is valid");
    validate_retained_battle_action_order(Some(10), 11).expect("later action frame is valid");

    for turn in [9, 10] {
        let error = validate_retained_battle_action_order(Some(10), turn)
            .expect_err("stale or duplicate battle action frame must reject");
        assert!(
            error
                .to_string()
                .contains("not strictly after previous turn 10"),
            "{error:#}"
        );
    }
}

#[test]
fn retained_state_hash_stream_rejects_terminal_before_start() {
    let start = StateChecksumFrame::new(LOCAL_PLAYER_ID, Frame(10), 0x1111_1111);
    let current = StateChecksumFrame::new(LOCAL_PLAYER_ID, Frame(9), 0x2222_2222);

    let error = validate_retained_state_hash_stream(&start, &current)
        .expect_err("terminal state hash before start must reject");
    assert!(
        error
            .to_string()
            .contains("current frame 9 is before start frame 10"),
        "{error:#}"
    );
}

#[test]
fn runtime_tile_to_metatile_u16_rejects_negative_coordinates() {
    let error =
        runtime_tile_to_metatile_u16(-1, 0, "test").expect_err("negative runtime x must reject");
    assert!(
        error
            .to_string()
            .contains("outside unsigned map coordinates"),
        "{error:#}"
    );
}

#[test]
fn script_text_renderer_resolves_runtime_named_buffers() {
    let body = ScriptTextBody {
        label: "RuntimeBufferText".to_string(),
        commands: vec![
            ScriptTextBodyCommand {
                command: "text".to_string(),
                args: vec!["Caught".to_string()],
                command_index: 0,
            },
            ScriptTextBodyCommand {
                command: "text_ram".to_string(),
                args: vec!["STRING_BUFFER_3".to_string()],
                command_index: 1,
            },
            ScriptTextBodyCommand {
                command: "text_decimal".to_string(),
                args: vec![
                    "STRING_BUFFER_4".to_string(),
                    "1".to_string(),
                    "3".to_string(),
                ],
                command_index: 2,
            },
        ],
    };
    let named_buffers = BTreeMap::from([
        ("STRING_BUFFER_3".to_string(), "CHIKORITA".to_string()),
        ("STRING_BUFFER_4".to_string(), "152".to_string()),
    ]);

    assert_eq!(
        render_script_text_body(&body, &named_buffers),
        "Caught\nCHIKORITA\n152"
    );
}

#[test]
fn asm_text_renderer_resolves_exact_text_ram_operand_names() {
    let named_buffers = BTreeMap::from([
        ("wSeerNickname".to_string(), "EMBER".to_string()),
        ("wStringBuffer2 + 1".to_string(), "12".to_string()),
    ]);

    assert_eq!(
        render_visible_asm_text_pages(
            "Hm… I see you met\n<RAM:wSeerNickname> at level <DECIMAL:wStringBuffer2 + 1, 1, 3>!",
            &named_buffers,
            "CHRIS",
            "RIVAL",
            0,
        ),
        vec!["Hm… I see you met\nEMBER at level 12!"]
    );
}

#[test]
fn asm_text_renderer_resolves_source_wram_name_for_canonical_script_buffer() {
    let named_buffers =
        BTreeMap::from([("STRING_BUFFER_3".to_string(), "CYNDAQUIL".to_string())]);

    assert_eq!(
        render_visible_asm_text_pages(
            "<PLAYER> received\n<RAM:wStringBuffer3>!",
            &named_buffers,
            "CHRIS",
            "RIVAL",
            0,
        ),
        vec!["CHRIS received\nCYNDAQUIL!"]
    );
}

#[test]
fn missing_script_text_buffers_render_blank_instead_of_host_diagnostics() {
    let body = ScriptTextBody {
        label: "MissingRuntimeBufferText".to_string(),
        commands: vec![ScriptTextBodyCommand {
            command: "text_ram".to_string(),
            args: vec!["STRING_BUFFER_3".to_string()],
            command_index: 0,
        }],
    };

    assert_eq!(render_script_text_body(&body, &BTreeMap::new()), "");
    assert_eq!(
        render_visible_script_text_pages(&body, &BTreeMap::new(), "CHRIS", "RIVAL", 0),
        vec![String::new()]
    );
}

#[test]
fn flattened_asm_text_resolves_runtime_buffers_before_glyph_rendering() {
    let named_buffers = BTreeMap::from([(
        "STRING_BUFFER_4".to_string(),
        "#GEAR".to_string(),
    )]);

    assert_eq!(
        render_visible_asm_text_pages(
            "<PLAYER> received\n<STRING_BUFFER_4>.",
            &named_buffers,
            "KRIS",
            "RIVAL",
            0,
        ),
        vec!["KRIS received\nPOKéGEAR.".to_string()],
        "the flattened ASM text path must not print a buffer token as player-visible glyphs",
    );
}

#[test]
fn visible_script_text_keeps_asm_paragraphs_as_player_advanced_pages() {
    let body = ScriptTextBody {
        label: "MomGivesPokegearText".to_string(),
        commands: vec![
            ScriptTextBodyCommand {
                command: "text".to_string(),
                args: vec!["#MON GEAR, or".to_string()],
                command_index: 0,
            },
            ScriptTextBodyCommand {
                command: "line".to_string(),
                args: vec!["just #GEAR.".to_string()],
                command_index: 1,
            },
            ScriptTextBodyCommand {
                command: "para".to_string(),
                args: vec!["It's essential if".to_string()],
                command_index: 2,
            },
            ScriptTextBodyCommand {
                command: "line".to_string(),
                args: vec!["you want to be a".to_string()],
                command_index: 3,
            },
            ScriptTextBodyCommand {
                command: "cont".to_string(),
                args: vec!["good trainer.".to_string()],
                command_index: 4,
            },
            ScriptTextBodyCommand {
                command: "done".to_string(),
                args: Vec::new(),
                command_index: 5,
            },
        ],
    };

    assert_eq!(
        render_visible_script_text_pages(&body, &BTreeMap::new(), "CHRIS", "RIVAL", 0),
        vec![
            "POKéMON GEAR, or\njust POKéGEAR.",
            "It's essential if\nyou want to be a",
            "you want to be a\ngood trainer.",
        ],
        "ASM field pages contain two text baselines; their intervening tile row is layout, not text"
    );
}

#[test]
fn cont_scroll_keeps_the_carried_line_without_printing_it_twice() {
    let previous = "It's essential if\nyou want to be a";
    let next = "you want to be a\ngood trainer.";

    assert_eq!(
        visible_field_page_initial_chars(previous, next),
        "you want to be a\n".chars().count(),
        "ASM <CONT> scrolls the existing bottom line to the top before printing only the new line"
    );
    assert_eq!(
        &next["you want to be a\n".len()..],
        "good trainer.",
        "the newly printed suffix must not contain the carried words"
    );
}

#[test]
fn text_chunks_and_ram_buffers_share_the_current_asm_cursor() {
    let body = ScriptTextBody {
        label: "Text_PlayerGotFive".to_string(),
        commands: vec![
            ScriptTextBodyCommand {
                command: "text".to_string(),
                args: vec!["<PLAYER> got five".to_string()],
                command_index: 0,
            },
            ScriptTextBodyCommand {
                command: "line".to_string(),
                args: vec!["@".to_string()],
                command_index: 1,
            },
            ScriptTextBodyCommand {
                command: "text_ram".to_string(),
                args: vec!["wStringBuffer4".to_string()],
                command_index: 2,
            },
            ScriptTextBodyCommand {
                command: "text".to_string(),
                args: vec!["!@".to_string()],
                command_index: 3,
            },
            ScriptTextBodyCommand {
                command: "text_end".to_string(),
                args: Vec::new(),
                command_index: 4,
            },
        ],
    };
    let named_buffers = BTreeMap::from([("wStringBuffer4".to_string(), "RARE CANDY".to_string())]);

    assert_eq!(
        render_visible_script_text_pages(&body, &named_buffers, "CHRIS", "RIVAL", 0),
        vec!["CHRIS got five\nRARE CANDY!".to_string()],
        "ASM @ terminates a source string and text/text_ram continue at the same cursor"
    );
}

#[test]
fn low_pause_and_today_follow_their_asm_cursor_semantics() {
    let body = ScriptTextBody {
        label: "OpcodeParityText".to_string(),
        commands: vec![
            ScriptTextBodyCommand {
                command: "text".to_string(),
                args: vec!["<PLAYER> used the@".to_string()],
                command_index: 0,
            },
            ScriptTextBodyCommand {
                command: "text_low".to_string(),
                args: Vec::new(),
                command_index: 1,
            },
            ScriptTextBodyCommand {
                command: "text_ram".to_string(),
                args: vec!["wStringBuffer2".to_string()],
                command_index: 2,
            },
            ScriptTextBodyCommand {
                command: "text_pause".to_string(),
                args: Vec::new(),
                command_index: 3,
            },
            ScriptTextBodyCommand {
                command: "text".to_string(),
                args: vec!["\" on @\"".to_string()],
                command_index: 4,
            },
            ScriptTextBodyCommand {
                command: "text_today".to_string(),
                args: Vec::new(),
                command_index: 5,
            },
            ScriptTextBodyCommand {
                command: "done".to_string(),
                args: Vec::new(),
                command_index: 6,
            },
        ],
    };
    let named_buffers = BTreeMap::from([("wStringBuffer2".to_string(), "BICYCLE".to_string())]);

    assert_eq!(
        render_visible_script_text_pages(&body, &named_buffers, "CHRIS", "RIVAL", 2),
        vec!["CHRIS used the\nBICYCLE on TUE".to_string()],
        "TX_LOW moves to the bottom baseline, TX_PAUSE prints no glyph, and TX_DAY appends the weekday at the current cursor"
    );
}

#[test]
fn every_exported_script_text_body_fits_the_asm_two_baseline_machine() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &AssetRoot::new(repo_root),
        "content-packs/core-modular.crystalpack",
    )
    .expect("load canonical compiled pack");
    let bodies = runtime.script_text_body_keys();
    assert!(
        bodies.len() > 1_000,
        "audit must cover the complete text corpus"
    );

    let mut audited_pages = 0usize;
    let mut audited_continuations = 0usize;
    for key in bodies {
        let has_far_text = key
            .commands
            .iter()
            .any(|command| command.command == "text_far");
        let named_buffers = key
            .commands
            .iter()
            .filter(|command| matches!(command.command.as_str(), "text_ram" | "text_decimal"))
            .filter_map(|command| command.args.first())
            .map(|buffer| (buffer.clone(), "BUFFER".to_string()))
            .collect::<BTreeMap<_, _>>();
        let body = ScriptTextBody {
            label: key.label.clone(),
            commands: key
                .commands
                .into_iter()
                .map(|command| ScriptTextBodyCommand {
                    command: command.command,
                    args: command.args,
                    command_index: command.command_index,
                })
                .collect(),
        };
        let pages = render_visible_script_text_pages(&body, &named_buffers, "CHRIS", "RIVAL", 0);
        let continuation_page_indexes = body
            .commands
            .iter()
            .enumerate()
            .filter(|(_, command)| command.command == "cont")
            .map(|(command_index, _)| {
                let prefix = ScriptTextBody {
                    label: body.label.clone(),
                    commands: body.commands[..=command_index].to_vec(),
                };
                render_visible_script_text_pages(&prefix, &named_buffers, "CHRIS", "RIVAL", 0).len()
                    - 1
            })
            .collect::<std::collections::BTreeSet<_>>();
        audited_pages += pages.len();
        for (page_index, page) in pages.iter().enumerate() {
            assert!(
                page.lines().count() <= 2,
                "{}:{} page {page_index} exceeds ASM's two printable baselines: {page:?}",
                key.map_name,
                key.label,
            );
            if page_index == 0 {
                continue;
            }
            if continuation_page_indexes.contains(&page_index) {
                let carried_chars = visible_field_page_initial_chars(&pages[page_index - 1], page);
                audited_continuations += 1;
                assert!(
                    carried_chars > 0,
                    "{}:{} authored <CONT> page {page_index} did not retain its previous baseline: {page:?}",
                    key.map_name,
                    key.label,
                );
                assert!(
                    carried_chars < page.chars().count(),
                    "{}:{} continuation must add new text after its carried line: {page:?}",
                    key.map_name,
                    key.label,
                );
            }
        }
        if !has_far_text {
            assert_eq!(
                continuation_page_indexes.len(),
                body.commands
                    .iter()
                    .filter(|command| command.command == "cont")
                    .count(),
                "{}:{} every authored <CONT> must map to one rendered continuation page",
                key.map_name,
                key.label,
            );
        }
    }
    assert!(
        audited_pages > 5_000,
        "audit did not traverse the full page corpus"
    );
    assert!(
        audited_continuations > 1_000,
        "audit did not exercise the exported <CONT> corpus"
    );
}

#[test]
fn inspect_route29_fruit_tree_runtime_flow() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(
        &asset_root,
        "content-packs/core-modular.crystalpack",
    )
    .expect("load compiled pack");
    let mut shell =
        RuntimeGameShell::new_game_at_runtime_tile(asset_root, runtime, 1, "Route29", 0, 0)
            .expect("start Route 29 shell");
    let run = shell
        .run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: "Route29".to_string(),
                source_script: "Route29FruitTree".to_string(),
                command_index: 0,
            },
            32,
            ScriptRuntimeInputs::default(),
            ScriptPhoneInputs::default(),
        )
        .expect("run Route 29 fruit tree");
    eprintln!("fruit_tree_run={run:#?}");
    eprintln!(
        "fruit_tree_snapshot={:#?}",
        shell.snapshot().expect("snapshot")
    );
}

#[test]
fn field_dialogue_prompt_waits_for_the_current_page_to_finish_printing() {
    let reveal = VisibleFieldTextReveal {
        // The stored value is the identity of every page, not the text of
        // this one page.  A multi-page dialogue must therefore be judged
        // against `current_page`, never this value.
        text: "first page\u{1e}second page".to_string(),
        page_index: 0,
        visible_chars: 3,
        frames_until_next_char: 0,
    };

    assert!(!visible_field_text_reveal_is_complete(
        &reveal,
        "first page"
    ));

    let finished = VisibleFieldTextReveal {
        visible_chars: "first page".chars().count(),
        ..reveal
    };
    assert!(visible_field_text_reveal_is_complete(
        &finished,
        "first page"
    ));
}

#[test]
fn visible_script_text_normalizes_asm_quotes_and_control_tokens() {
    assert_eq!(
        normalize_visible_script_text("\"Oh, <PLAYER>! Your #MON\"", "CHRIS"),
        "\"Oh, CHRIS! Your POKéMON\""
    );
    assert_eq!(render_script_text_args(&["\"Hello\"".to_string()]), "Hello");
    assert_eq!(bitmap_font_char_map().get(&'é'), Some(&0xea));
}

#[test]
fn visible_script_text_uses_saved_rival_name_and_current_weekday() {
    assert_eq!(
        normalize_visible_script_text_with_context(
            "Meet <RIVAL> on <TODAY>.",
            "CHRIS",
            "SILVER",
            3,
        ),
        "Meet SILVER on WED."
    );
}

#[test]
fn bitmap_font_normalization_and_glyph_map_match_typescript() {
    assert_eq!(
        normalize_bitmap_font_text(
            "<TRAINER> <ROCKET> <PKMN> <POKE> <PC> <TM> <PK><MN><DOT><PO><KE><LV><ID><……>#"
        ),
        "\u{e103} \u{e104} \u{e105}\u{e106} POKé \u{e101} \u{e102} \u{e105}\u{e106}\u{e107}\u{e108}\u{e109}\u{e10a}\u{e10b}……POKé"
    );

    let glyphs = bitmap_font_char_map();
    for (glyph, tile) in [
        ('Ä', 0xc0),
        ('Ö', 0xc1),
        ('Ü', 0xc2),
        ('ä', 0xc3),
        ('ö', 0xc4),
        ('ü', 0xc5),
        ('☎', 0x62),
        ('▲', 0x61),
        ('…', 0x75),
        ('—', 0x7a),
        ('–', 0x7a),
        ('\u{e100}', 0x4a),
        ('\u{e101}', 0x5b),
        ('\u{e102}', 0x5c),
        ('\u{e103}', 0x5d),
        ('\u{e104}', 0x5e),
        ('\u{e105}', 0xe1),
        ('\u{e106}', 0xe2),
        ('\u{e107}', 0xf2),
        ('\u{e108}', 0x70),
        ('\u{e109}', 0x71),
        ('\u{e10a}', 0x6e),
        ('\u{e10b}', 0x73),
    ] {
        assert_eq!(glyphs.get(&glyph), Some(&tile), "glyph {glyph:?}");
    }
}

fn route36_overworld_shell_for_battle_render_regression() -> BevyRuntimeShell {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "Route36".to_string(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Route36 overworld shell");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "BATTLE_RENDER_REGRESSION",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add battle party Pokemon");
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)
        .expect("settle arrival scripts before battle render regression");
    runtime_shell
}

fn route36_battle_shell_for_render_regression() -> BevyRuntimeShell {
    let mut runtime_shell = route36_overworld_shell_for_battle_render_regression();
    runtime_shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .expect("start Sudowoodo battle");
    prepare_visible_battle_entry(&mut runtime_shell).expect("prepare visible battle entry");
    runtime_shell
}

fn battle_render_regression_app(runtime_shell: BevyRuntimeShell) -> App {
    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);
    app
}

fn finish_current_battle_message_for_regression(runtime_shell: &mut BevyRuntimeShell) {
    let message = runtime_shell
        .battle_messages
        .front()
        .expect("battle message to finish")
        .clone();
    let pages = battle_message_pages(&message);
    let page_index = pages
        .len()
        .checked_sub(1)
        .expect("at least one message page");
    runtime_shell.battle_text_reveal = Some(VisibleBattleTextReveal {
        text: message,
        page_index,
        visible_chars: pages[page_index].chars().count(),
        frames_until_next_char: 0,
    });
}
