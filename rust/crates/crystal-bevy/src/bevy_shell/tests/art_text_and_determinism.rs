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
    copy_pokemon_frame_rgba(&source, 5, 5, &palette, &mut target);
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
            "POKéMON GEAR, or\n\njust POKéGEAR.",
            "It's essential if\n\nyou want to be a",
            "you want to be a\n\ngood trainer.",
        ],
        "ASM para and cont must preserve their player-acknowledged clear/scroll boundaries"
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
