#[test]
fn field_move_play_refusals_do_not_swallow_unsupported_pack_coverage() {
    assert!(party_field_move_error_is_play_refusal(&anyhow::anyhow!(
        FieldMoveError::MissingBadge {
            move_id: "SURF".to_string(),
            region: "johto".to_string(),
            badge_index: 5,
        }
    )));
    assert!(party_field_move_error_is_play_refusal(&anyhow::anyhow!(
        FieldMoveError::AlwaysOnBike {
            move_id: "SURF".to_string(),
        }
    )));
    assert!(!party_field_move_error_is_play_refusal(&anyhow::anyhow!(
        FieldMoveError::UnsupportedReplacement {
            move_id: "CUT".to_string(),
            tileset_name: "johto".to_string(),
            block_id: 0x5b,
        }
    )));
    assert!(!party_field_move_error_is_play_refusal(&anyhow::anyhow!(
        FieldMoveError::UnsupportedCollision {
            move_id: "WHIRLPOOL".to_string(),
            block_id: 0x30,
        }
    )));
}

#[test]
fn contextual_surf_prompt_obeys_the_source_player_state_gates() {
    assert!(source_allows_contextual_surf_prompt(
        MovementMode::Normal,
        false
    ));
    assert!(source_allows_contextual_surf_prompt(
        MovementMode::Bike,
        false
    ));
    assert!(!source_allows_contextual_surf_prompt(
        MovementMode::Bike,
        true
    ));
    assert!(!source_allows_contextual_surf_prompt(
        MovementMode::Surf,
        false
    ));
    assert!(!source_allows_contextual_surf_prompt(
        MovementMode::SurfPika,
        false
    ));
}

#[test]
fn contextual_whirlpool_prompt_requires_the_source_block_replacement() {
    let rule = crate::RuntimeFieldMoveRuleKey {
        rule_id: "whirlpool".to_string(),
        rule_kind: "block".to_string(),
        move_id: Some("WHIRLPOOL".to_string()),
        item_id: None,
        badge_region: Some("johto".to_string()),
        badge_index: Some(6),
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: vec![0x38],
        blocked_collisions: Vec::new(),
        replacements: [(
            "johto".to_string(),
            [(
                0x07,
                crate::RuntimeFieldMoveReplacementKey {
                    replacement_block_id: 0x36,
                    variant: "whirlpool".to_string(),
                },
            )]
            .into_iter()
            .collect(),
        )]
        .into_iter()
        .collect(),
    };

    assert!(source_allows_contextual_block_field_move_prompt(
        &rule,
        "johto",
        Some(0x07),
    ));
    assert!(
        !source_allows_contextual_block_field_move_prompt(&rule, "johto", Some(0x08)),
        "a target collision without an exact WhirlpoolBlockPointers replacement must fail before yesorno"
    );
    assert!(!source_allows_contextual_block_field_move_prompt(
        &rule, "kanto", Some(0x07),
    ));
    assert!(!source_allows_contextual_block_field_move_prompt(
        &rule, "johto", None,
    ));
}

#[test]
fn contextual_headbutt_prompt_tests_the_exact_facing_collision() {
    let rule = crate::RuntimeFieldMoveRuleKey {
        rule_id: "headbutt".to_string(),
        rule_kind: "move".to_string(),
        move_id: Some("HEADBUTT".to_string()),
        item_id: None,
        badge_region: None,
        badge_index: None,
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: vec![0x15, 0x1d],
        blocked_collisions: Vec::new(),
        replacements: Default::default(),
    };

    assert!(source_allows_contextual_move_only_target_prompt(&rule, 0x15));
    assert!(source_allows_contextual_move_only_target_prompt(&rule, 0x1d));
    assert!(
        !source_allows_contextual_move_only_target_prompt(&rule, 0x00),
        "a different quadrant of the same metatile must not inherit its Headbutt collision"
    );
}

#[test]
fn headbutt_dispatch_preserves_menu_vs_overworld_script_roots() {
    assert_eq!(
        source_headbutt_script_root(true),
        "HeadbuttFromMenuScript"
    );
    assert_eq!(source_headbutt_script_root(false), "HeadbuttScript");
}

#[test]
fn every_asm_text_button_wait_exposes_the_blinking_advance_cursor() {
    assert!(pending_text_wait_command_shows_prompt_arrow("promptbutton"));
    assert!(
        pending_text_wait_command_shows_prompt_arrow("waitbutton"),
        "ElmText_GotAnEmail ends in waitbutton and must not look frozen on its final `Okay...` page"
    );
}

#[test]
fn runtime_tile_playfield_position_preserves_runtime_tile_offsets() {
    assert_eq!(
        runtime_event_view_tile(TilePosition::new(1, 0), 0, 0),
        Some((2, 0))
    );
    assert_eq!(
        runtime_event_view_tile(TilePosition::new(i16::MIN, 0), 1, 0),
        None
    );
    assert_eq!(
        runtime_tile_playfield_position(TilePosition::new(1, 0), 0, 0),
        Some((
            PLAYFIELD_LEFT + TILE_SIZE * 2.5,
            PLAYFIELD_TOP - TILE_SIZE * 0.5,
        ))
    );
    assert_eq!(
        runtime_tile_playfield_position(TilePosition::new(-1, 0), 0, 0),
        None
    );
}

#[test]
fn overworld_sprites_anchor_to_their_complete_oam_footprint() {
    let base = runtime_tile_playfield_position(TilePosition::new(0, 0), 0, 0)
        .expect("visible origin tile");
    assert_eq!(
        overworld_sprite_position_from_base(base.0, base.1, Vec2::splat(TILE_SIZE * 2.0)),
        (base.0 + TILE_SIZE * 0.5, base.1 - TILE_SIZE * 0.5),
        "a 16x16 Game Boy sprite must cover its two-by-two render-tile footprint"
    );
    assert_eq!(
        overworld_sprite_position_from_base(base.0, base.1, Vec2::splat(TILE_SIZE)),
        base,
        "single-tile icon sprites remain anchored to their addressed render tile"
    );
}

#[test]
fn same_map_redraw_retains_only_the_player_not_grass_rustle_frames() {
    assert!(!should_despawn_player_facing_entity(true, true));
    assert!(
        should_despawn_player_facing_entity(true, false),
        "grass rustle and other transient player-facing OAM must be replaced each frame"
    );
    assert!(should_despawn_player_facing_entity(false, true));
}

#[test]
fn overworld_emotes_center_above_the_object_oam_footprint() {
    let base = runtime_tile_playfield_position(TilePosition::new(0, 0), 0, 0)
        .expect("visible origin tile");
    assert_eq!(
        overworld_emote_position_from_base(base.0, base.1, Vec2::splat(TILE_SIZE * 2.0)),
        (base.0 + TILE_SIZE * 0.5, base.1 + TILE_SIZE * 1.5),
        "a 16x16 emote must be centered immediately above the 16x16 object sprite"
    );
}

#[test]
fn finite_pcm_music_does_not_restart_when_its_playback_plan_requests_looping() {
    let pcm_music = BevyAudioCommand {
        audio_id: "MUSIC_CRYSTAL_OPENING".to_string(),
        kind: ModpackAudioKind::Music,
        mode: ModpackAudioPlaybackMode::RawPcm,
        looped: true,
    };
    assert!(
        !native_audio_repeats_without_pcm_loop(&pcm_music),
        "a PCM asset without explicit loop bounds must end at its exported endpoint"
    );

    let midi_music = BevyAudioCommand {
        mode: ModpackAudioPlaybackMode::SequencedMidi,
        ..pcm_music
    };
    assert!(native_audio_repeats_without_pcm_loop(&midi_music));
}

#[test]
fn map_debug_details_report_runtime_tiles_and_raw_event_coordinates() {
    let warp = crate::core::map::WarpEvent {
        index: 3,
        x: 2,
        y: 3,
        target_map_constant: "ROUTE_29".to_string(),
        target_map: "ROUTE_29".to_string(),
        target_warp_id: 1,
    };
    assert_eq!(
        format_warp_event_detail_line(&warp),
        "warp 3 runtime_tile=(2, 3) raw=(2, 3) target=ROUTE_29 target_warp=1"
    );

    let object = crate::core::map::ObjectEvent {
        sprite: "SPRITE_TEACHER".to_string(),
        x: 2,
        y: 3,
        spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
        move_range_x: 0,
        move_range_y: 0,
        hram_x: -1,
        hram_y: -1,
        pal: 0,
        object_type: "OBJECTTYPE_SCRIPT".to_string(),
        radius: 0,
        script: "TeacherScript".to_string(),
        label: None,
        event_flag: "EVENT_TEACHER".to_string(),
        object_identifier: Some("ROUTE29_TEACHER".to_string()),
        sightline_direction_override: None,
    };
    assert_eq!(
        format_visible_object_detail_line(&object),
        "visible_object Some(\"ROUTE29_TEACHER\") sprite=SPRITE_TEACHER runtime_tile=(2, 3) raw=(2, 3) script=TeacherScript flag=EVENT_TEACHER"
    );
}

#[test]
fn runtime_tile_bounds_use_checked_runtime_metatile_width() {
    assert_eq!(
        runtime_tile_bounds_i16("ROUTE_29", 20, 18).expect("map bounds"),
        (40, 36)
    );
    assert_eq!(
        render_tile_bounds_i16("ROUTE_29", 20, 18).expect("render bounds"),
        (80, 72)
    );
    assert!(
        runtime_tile_bounds_i16("OVERFLOW_WIDTH", 16_384, 1)
            .expect_err("overflowing width rejects")
            .to_string()
            .contains("width 16384 overflows supported runtime tile coordinate bounds")
    );
    assert!(
        runtime_tile_bounds_i16("OVERFLOW_HEIGHT", 1, 16_384)
            .expect_err("overflowing height rejects")
            .to_string()
            .contains("height 16384 overflows supported runtime tile coordinate bounds")
    );
}

#[test]
fn render_viewport_origin_clamps_like_typescript_camera() {
    assert_eq!(render_viewport_origin(4, 16, VIEWPORT_TILES_X), 0);
    assert_eq!(render_viewport_origin(20, 80, VIEWPORT_TILES_X), 10);
    assert_eq!(render_viewport_origin(79, 80, VIEWPORT_TILES_X), 60);
    assert_eq!(render_viewport_origin(4, 12, VIEWPORT_TILES_Y), 0);
    assert_eq!(render_viewport_origin(18, 72, VIEWPORT_TILES_Y), 9);
    assert_eq!(render_viewport_origin(71, 72, VIEWPORT_TILES_Y), 54);
}

#[test]
fn johto_tileset_art_loads_real_runtime_assets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let palette_map_path = asset_root
        .runtime_assets()
        .join("data/tilesets/johto_palette_map.json");
    let palette_map: Vec<u8> =
        serde_json::from_slice(&std::fs::read(&palette_map_path).expect("read johto palette map"))
            .expect("parse johto palette map");
    let mut images = Assets::<Image>::default();
    let art = load_tileset_art(&asset_root, "johto", "day", &palette_map, &mut images)
        .expect("load johto tileset art");

    assert!(art.metatile_layout.len() >= METATILE_TILE_COUNT);
    assert!(art.tile_handles.len() > 0x40);
    assert!(art.tile_handle(0, 0, 0).is_some());
    assert!(art.tile_handle(0, 1, 0).is_some());
    assert!(art.tile_handle(0, 3, 3).is_some());
}

#[test]
fn battle_tower_outside_applies_the_cianwood_olivine_roof_tiles() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let runtime_assets = repo_root.join("vendor/pokecrystal");
    let mut source = image::open(runtime_assets.join("gfx/tilesets/battle_tower_outside.png"))
        .expect("Battle Tower outside tileset")
        .to_rgba8();
    let before = source.clone();
    let roof = image::open(runtime_assets.join("gfx/tilesets/roofs/olivine.png"))
        .expect("Olivine roof")
        .to_rgba8();

    apply_battle_tower_outside_roof(&runtime_assets, &mut source)
        .expect("apply Battle Tower map-group roof");

    for tile in 0..9_u32 {
        let source_tile = 0x0a + tile;
        let source_x = (source_tile % 16) * SOURCE_TILE_SIZE as u32;
        let source_y = (source_tile / 16) * SOURCE_TILE_SIZE as u32;
        let roof_x = (tile % 3) * SOURCE_TILE_SIZE as u32;
        let roof_y = (tile / 3) * SOURCE_TILE_SIZE as u32;
        for y in 0..SOURCE_TILE_SIZE as u32 {
            for x in 0..SOURCE_TILE_SIZE as u32 {
                assert_eq!(
                    source.get_pixel(source_x + x, source_y + y),
                    roof.get_pixel(roof_x + x, roof_y + y)
                );
            }
        }
    }
    assert_ne!(
        source, before,
        "the runtime roof overwrite must change the shared base graphics"
    );
}

#[test]
fn viewport_tile_composite_preserves_native_tile_grid_for_gpu_scaling() {
    let mut images = Assets::<Image>::default();
    let tile = images.add(Image::new(
        Extent3d {
            width: SOURCE_TILE_SIZE as u32,
            height: SOURCE_TILE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0xff, 0x00, 0x00, 0xff]
            .into_iter()
            .cycle()
            .take(SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4)
            .collect(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let handles = vec![tile; (VIEWPORT_TILES_X * VIEWPORT_TILES_Y) as usize];
    let composite = compose_viewport_tiles(&handles, None, &mut images);
    let image_count = images.len();
    let reused = compose_viewport_tiles(&handles, Some(composite.clone()), &mut images);
    assert_eq!(reused, composite);
    assert_eq!(images.len(), image_count);
    let image = images.get(&composite).expect("composited viewport image");
    assert_eq!(image.texture_descriptor.size.width, 160);
    assert_eq!(image.texture_descriptor.size.height, 144);
    assert_eq!(&image.data[0..4], &[0xff, 0x00, 0x00, 0xff]);
    let last = (144 * 160 - 1) * 4;
    assert_eq!(&image.data[last..last + 4], &[0xff, 0x00, 0x00, 0xff]);
}

#[test]
fn scrolling_map_composite_carries_one_runtime_tile_beyond_every_lcd_edge() {
    let mut images = Assets::<Image>::default();
    let tile = images.add(Image::new(
        Extent3d {
            width: SOURCE_TILE_SIZE as u32,
            height: SOURCE_TILE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        vec![0xff; SOURCE_TILE_SIZE * SOURCE_TILE_SIZE * 4],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ));
    let handles = vec![tile; (CLASSIC_SCROLL_TILES_X * CLASSIC_SCROLL_TILES_Y) as usize];
    let composite = compose_tile_grid(
        &handles,
        CLASSIC_SCROLL_TILES_X as usize,
        CLASSIC_SCROLL_TILES_Y as usize,
        None,
        &mut images,
    );
    let size = images
        .get(&composite)
        .expect("scrolling map composite")
        .texture_descriptor
        .size;

    assert_eq!(size.width, 192);
    assert_eq!(size.height, 176);
    assert_eq!(CLASSIC_SCROLL_HALO_TILES, METATILE_WIDTH);
}

#[test]
fn scrolling_character_culling_carries_the_same_edge_halo_as_the_map() {
    for (x, y) in [
        (-CLASSIC_SCROLL_HALO_TILES, 0),
        (VIEWPORT_TILES_X + CLASSIC_SCROLL_HALO_TILES - 1, 0),
        (0, -CLASSIC_SCROLL_HALO_TILES),
        (0, VIEWPORT_TILES_Y + CLASSIC_SCROLL_HALO_TILES - 1),
    ] {
        assert!(
            overworld_object_in_scroll_region(x, y),
            "an edge character at ({x}, {y}) must exist before camera scrolling reveals it"
        );
    }

    assert!(!overworld_object_in_scroll_region(
        -CLASSIC_SCROLL_HALO_TILES - 1,
        0
    ));
    assert!(!overworld_object_in_scroll_region(
        VIEWPORT_TILES_X + CLASSIC_SCROLL_HALO_TILES,
        0
    ));
}

#[test]
fn bitmap_font_art_loads_runtime_menu_glyphs() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();
    let art = load_bitmap_font_art(&asset_root, &mut images).expect("load bitmap font art");

    for glyph in ['A', 'Z', '0', '9', '>', '?', ' '] {
        assert!(
            art.glyphs.contains_key(&glyph),
            "expected bitmap font glyph {glyph:?}"
        );
        assert_eq!(
            art.glyphs.get(&glyph).map(|frame| frame.size),
            Some(Vec2::splat(TILE_SIZE)),
            "dialogue glyph {glyph:?} must occupy one scaled 8x8 Game Boy tile"
        );
    }
}

#[test]
fn field_dialogue_uses_the_game_boy_text_grid() {
    assert_eq!(SCENE_DIALOG_TEXT_CHARS, 18);
    assert_eq!(FIELD_TEXT_BOX_TEXT_LEFT_TILE, 1.0);
    assert_eq!(FIELD_TEXT_BOX_TEXT_TOP_TILE, 14.0);
    assert_eq!(FIELD_TEXT_BOX_ROW_SPACING_TILES, 2.0);
    assert_eq!(FIELD_TEXT_BOX_VISIBLE_ROWS, 2);
    assert_eq!(BITMAP_FONT_ADVANCE, TILE_SIZE);
}

#[test]
fn window_frames_use_the_selected_asset_and_typescript_textbox_palette() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();
    let frame_one = load_window_frame_art(&asset_root, 1, &mut images).expect("frame 1");
    let frame_two = load_window_frame_art(&asset_root, 2, &mut images).expect("frame 2");
    let one = images
        .get(&frame_one.top_left.handle)
        .expect("frame 1 image");
    let two = images
        .get(&frame_two.top_left.handle)
        .expect("frame 2 image");
    let textbox_palette = [[255, 255, 255], [140, 156, 255], [115, 132, 255], [0, 0, 0]];

    assert_ne!(
        one.data, two.data,
        "frame selection must change the rendered border art"
    );
    for image in [one, two] {
        assert!(image.data.chunks_exact(4).all(|pixel| {
            pixel[3] == 255 && textbox_palette.iter().any(|color| pixel[0..3] == color[..])
        }));
    }
}

#[test]
fn textbox_frame_ids_cover_every_saved_option() {
    use crate::core::state::FrameType;
    assert_eq!(textbox_frame_id(FrameType::Frame1), 1);
    assert_eq!(textbox_frame_id(FrameType::Frame2), 2);
    assert_eq!(textbox_frame_id(FrameType::Frame3), 3);
    assert_eq!(textbox_frame_id(FrameType::Frame4), 4);
    assert_eq!(textbox_frame_id(FrameType::Frame5), 5);
    assert_eq!(textbox_frame_id(FrameType::Frame6), 6);
    assert_eq!(textbox_frame_id(FrameType::Frame7), 7);
    assert_eq!(textbox_frame_id(FrameType::Frame8), 8);
}

#[test]
fn field_dialog_fast_path_rejects_an_orphaned_yes_no_window() {
    let field_frame_tiles = battle_window_frame_tile_count(
        FIELD_TEXT_BOX_WIDTH_TILES as usize,
        FIELD_TEXT_BOX_HEIGHT_TILES as usize,
    );
    let yes_no_frame_tiles = battle_window_frame_tile_count(
        FIELD_YES_NO_WIDTH_TILES as usize,
        FIELD_YES_NO_HEIGHT_TILES as usize,
    );

    assert!(retained_field_dialog_structure_matches(
        1,
        field_frame_tiles,
        false,
        false,
    ));
    assert!(retained_field_dialog_structure_matches(
        1,
        field_frame_tiles + yes_no_frame_tiles,
        true,
        true,
    ));
    assert!(!retained_field_dialog_structure_matches(
        1,
        field_frame_tiles + yes_no_frame_tiles,
        false,
        false,
    ));
}

#[test]
fn field_dialogue_wraps_to_the_asm_eighteen_character_baseline() {
    assert_eq!(
        wrap_scene_dialog_line(
            "POKéMON GEAR, or just POKéGEAR. It's essential information.",
            SCENE_DIALOG_TEXT_CHARS,
        ),
        vec![
            "POKéMON GEAR, or",
            "just POKéGEAR.",
            "It's essential",
            "information.",
        ],
    );
}

#[test]
fn player_walk_foot_phase_alternates_on_consecutive_steps() {
    let first = next_player_walk_stride(0, false);
    let second = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, first);
    let third = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, second);

    assert!(first, "the first step uses the first walking foot");
    assert!(!second, "the second consecutive step uses the mirrored foot");
    assert!(
        third,
        "the third consecutive step returns to the first walking foot"
    );
}

#[test]
fn held_direction_cycles_standing_and_both_step_frames() {
    let mut stride = false;
    let mut mirror = false;
    let mut frames = Vec::new();

    for _ in 0..4 {
        let previous_stride = stride;
        stride = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, stride);
        if previous_stride && !stride {
            mirror = !mirror;
        }
        frames.push((player_walk_uses_action_frame(stride), mirror));
    }

    assert_eq!(
        frames,
        vec![(true, false), (false, true), (true, true), (false, false)],
        "held input must match TypeScript's step, standing, mirrored-step, standing cycle"
    );
}

#[test]
fn new_walk_step_does_not_consume_ticks_elapsed_before_it_started() {
    assert_eq!(visible_new_step_frames_remaining(WALK_FRAME_HOLD_TICKS, 0), 8);
    assert_eq!(visible_new_step_frames_remaining(WALK_FRAME_HOLD_TICKS, 1), 8);
    assert_eq!(visible_new_step_frames_remaining(WALK_FRAME_HOLD_TICKS, 4), 8);

    let target = TilePosition { x: 4, y: 2 };
    let from = TilePosition { x: 2, y: 2 };
    let one_tick = visible_player_playfield_position(target, Some(from), 8, 0, 0)
        .expect("one-tick TypeScript position");
    let two_ticks = visible_player_playfield_position(target, Some(from), 7, 0, 0)
        .expect("two-tick TypeScript position");
    let settled = visible_player_playfield_position(target, None, 0, 0, 0)
        .expect("settled TypeScript position");
    let origin = visible_player_playfield_position(from, None, 0, 0, 0)
        .expect("origin TypeScript position");

    assert_eq!(two_ticks.0 - one_tick.0, (settled.0 - origin.0) / 8.0);
    assert_eq!(two_ticks.1, one_tick.1);
}

#[test]
fn renderer_hitch_never_skips_visible_overworld_walk_substeps() {
    assert_eq!(visible_walk_ticks_for_host_update(0), 0);
    assert_eq!(visible_walk_ticks_for_host_update(1), 1);
    assert_eq!(visible_walk_ticks_for_host_update(4), 1);
}

#[test]
fn overworld_walk_advances_one_visible_substep_after_renderer_catch_up() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize renderer catch-up fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 16;
    for _ in 0..16 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .player_walk_frame_ticks
            == WALK_FRAME_HOLD_TICKS
        {
            break;
        }
    }
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .player_walk_frame_ticks,
        WALK_FRAME_HOLD_TICKS,
        "fixture must begin an authoritative overworld step"
    );

    app.world_mut()
        .resource_mut::<RuntimeTickTimer>()
        .finished_vblanks = 4;
    app.update();

    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .player_walk_frame_ticks,
        WALK_FRAME_HOLD_TICKS - 1,
        "four accumulated simulation ticks must not skip four rendered walk positions"
    );
}

#[test]
fn full_redraw_mid_walk_retains_the_active_camera_origin() {
    let old_origin = Some((4, 2));
    let new_origin = Some((6, 2));

    assert_eq!(
        next_walk_viewport_origin(old_origin, None, new_origin, true),
        old_origin,
        "the camera scroll begins at the previously rendered origin"
    );
    assert_eq!(
        next_walk_viewport_origin(new_origin, old_origin, new_origin, true),
        old_origin,
        "an ambient/full redraw during the step must retain the active scroll origin"
    );
    assert_eq!(
        next_walk_viewport_origin(new_origin, old_origin, new_origin, false),
        None,
        "the retained origin clears after movement lands"
    );
}

#[test]
fn forced_full_redraw_does_not_jump_the_walking_player_transform() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize mid-walk redraw fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 16;
    for _ in 0..16 {
        app.update();
        let world = app.world();
        if world
            .resource::<RenderedViewport>()
            .walk_viewport_origin
            .is_some()
            && world
                .resource::<BevyRuntimeShell>()
                .player_walk_frame_ticks
                > 1
        {
            break;
        }
    }
    assert!(
        app.world()
            .resource::<RenderedViewport>()
            .walk_viewport_origin
            .is_some(),
        "fixture must begin a camera-following walk"
    );
    let before = {
        let world = app.world_mut();
        let mut players = world.query_filtered::<&Transform, With<PlayerMarker>>();
        players
            .get_single(world)
            .expect("player before forced redraw")
            .translation
    };

    app.world_mut()
        .resource_mut::<RenderedViewport>()
        .shell_render_key = None;
    app.update();

    let after = {
        let world = app.world_mut();
        let mut players = world.query_filtered::<&Transform, With<PlayerMarker>>();
        players
            .get_single(world)
            .expect("player after forced redraw")
            .translation
    };
    let displacement = (after - before).truncate().length();
    assert!(
        displacement
            <= TILE_SIZE * f32::from(METATILE_WIDTH) / f32::from(WALK_FRAME_HOLD_TICKS) * 1.5,
        "a full redraw skipped visible movement positions: before={before:?} after={after:?} displacement={displacement}"
    );
}

#[test]
fn new_bark_full_width_back_and_forth_has_no_transform_spikes() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize full-width New Bark fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let mut maximum_displacement = 0.0_f32;
    for (key, target_x) in [
        (KeyCode::ArrowRight, 17),
        (KeyCode::ArrowLeft, 2),
        (KeyCode::ArrowRight, 17),
    ] {
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset(KeyCode::ArrowLeft);
            keys.reset(KeyCode::ArrowRight);
            keys.press(key);
        }
        let mut reached_target = false;
        for _ in 0..240 {
            let before = {
                let world = app.world_mut();
                let mut players = world.query_filtered::<&Transform, With<PlayerMarker>>();
                players
                    .get_single(world)
                    .expect("player before full-width update")
                    .translation
            };
            // Exercise the failure path aggressively: ordinary ambient tile
            // animation reaches the same full-redraw branch periodically.
            app.world_mut()
                .resource_mut::<RenderedViewport>()
                .shell_render_key = None;
            app.update();
            let after = {
                let world = app.world_mut();
                let mut players = world.query_filtered::<&Transform, With<PlayerMarker>>();
                players
                    .get_single(world)
                    .expect("player after full-width update")
                    .translation
            };
            maximum_displacement = maximum_displacement.max((after - before).truncate().length());

            let shell = app.world().resource::<BevyRuntimeShell>();
            let tile = shell
                .shell
                .snapshot()
                .expect("New Bark snapshot")
                .overworld
                .tile;
            reached_target = tile.x == target_x && shell.player_walk_frame_ticks == 0;
            if reached_target {
                break;
            }
        }
        {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset(KeyCode::ArrowLeft);
            keys.reset(KeyCode::ArrowRight);
        }
        app.update();
        assert!(reached_target, "{key:?} did not reach New Bark x={target_x}");
    }

    let maximum_normal_substep =
        TILE_SIZE * f32::from(METATILE_WIDTH) / f32::from(WALK_FRAME_HOLD_TICKS) * 1.5;
    assert!(
        maximum_displacement <= maximum_normal_substep,
        "full-width New Bark traversal contained a transform spike: maximum={maximum_displacement}, allowed={maximum_normal_substep}"
    );
}

#[test]
fn host_frames_interpolate_between_authoritative_walk_ticks() {
    let at_tick = visible_movement_progress_with_subframe(7, 8, 0.0);
    let halfway_to_next_tick = visible_movement_progress_with_subframe(7, 8, 0.5);
    let at_next_tick = visible_movement_progress_with_subframe(6, 8, 0.0);

    assert!(at_tick < halfway_to_next_tick);
    assert!(halfway_to_next_tick < at_next_tick);
    assert_eq!(halfway_to_next_tick, (at_tick + at_next_tick) * 0.5);
    assert_eq!(visible_movement_progress_with_subframe(0, 8, 0.5), 1.0);
}

#[test]
fn consecutive_high_refresh_steps_have_no_terminal_hold_or_boundary_jump() {
    let mut positions = Vec::new();
    for step in 0..2 {
        for remaining in (1..=WALK_FRAME_HOLD_TICKS).rev() {
            for subframe in [0.0, 0.5] {
                positions.push(
                    step as f32
                        + visible_movement_progress_with_subframe(
                            remaining,
                            WALK_FRAME_HOLD_TICKS,
                            subframe,
                        ),
                );
            }
        }
    }

    let expected_delta = 0.5 / f32::from(WALK_FRAME_HOLD_TICKS);
    for pair in positions.windows(2) {
        assert!(
            ((pair[1] - pair[0]) - expected_delta).abs() < f32::EPSILON,
            "held movement must advance uniformly across tile boundaries: {positions:?}",
        );
    }
}

#[test]
fn moving_objects_cycle_instead_of_holding_the_step_frame() {
    let frames = (0..4)
        .map(|phase| {
            (
                object_walk_uses_action_frame(phase),
                object_walk_uses_mirrored_action_frame(phase),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        frames,
        vec![(false, false), (true, false), (false, false), (true, true)]
    );
}

#[test]
fn follower_walk_animation_preserves_a_cycle_for_each_direction() {
    let mut direction_phases = HashMap::<Direction, u8>::new();
    let path = [
        Direction::Down,
        Direction::Right,
        Direction::Down,
        Direction::Right,
        Direction::Down,
    ];
    let phases = path
        .into_iter()
        .map(|direction| {
            let phase = next_directional_walk_phase(direction_phases.get(&direction).copied());
            direction_phases.insert(direction, phase);
            phase
        })
        .collect::<Vec<_>>();

    assert_eq!(
        phases,
        vec![1, 1, 2, 2, 3],
        "a follower returning to a direction must resume that direction's TypeScript gait"
    );
    assert_eq!(
        phases
            .iter()
            .map(|phase| object_walk_uses_action_frame(*phase))
            .collect::<Vec<_>>(),
        vec![true, true, false, false, true]
    );
}

#[test]
fn player_cornering_preserves_each_directions_own_walk_cycle() {
    let mut direction_phases = HashMap::<Direction, u8>::new();
    let path = [
        Direction::Down,
        Direction::Right,
        Direction::Down,
        Direction::Right,
        Direction::Down,
    ];
    let frames = path
        .into_iter()
        .map(|direction| {
            let phase = next_directional_walk_phase(direction_phases.get(&direction).copied());
            direction_phases.insert(direction, phase);
            (phase & 1 == 1, phase == 3)
        })
        .collect::<Vec<_>>();

    assert_eq!(
        frames,
        vec![(true, false), (true, false), (false, false), (false, false), (true, true)]
    );
}

#[test]
fn scripted_follow_waits_for_the_final_follower_step_to_land() {
    let follower_id = "FOLLOWER".to_string();
    let mut follower_timers = BTreeMap::new();
    follower_timers.insert(follower_id.clone(), WALK_FRAME_HOLD_TICKS);

    assert!(visible_actor_walk_in_flight(
        &follower_id,
        0,
        0,
        &follower_timers,
    ));
    follower_timers.insert(follower_id.clone(), 1);
    assert!(visible_actor_walk_in_flight(
        &follower_id,
        0,
        0,
        &follower_timers,
    ));
    follower_timers.remove(&follower_id);
    assert!(!visible_actor_walk_in_flight(
        &follower_id,
        0,
        0,
        &follower_timers,
    ));
}

#[test]
fn scripted_follow_drains_its_last_queued_command_without_requeueing_it() {
    let final_step = VisibleFollowerStep {
        direction: Direction::Left,
        stride: 2,
        duration: WALK_FRAME_HOLD_TICKS * 2,
        jump: true,
        standing_frame: false,
    };
    let mut queued_step = Some(final_step);

    let active_step = rotate_visible_follower_step(Some("FOLLOWER"), &mut queued_step, None);

    assert_eq!(active_step, Some(final_step));
    assert_eq!(queued_step, None);
}

#[test]
fn player_walk_interpolates_each_lcd_frame_between_committed_tiles() {
    for (from, to) in [
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 3, y: 2 }),
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 1, y: 2 }),
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 2, y: 1 }),
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 2, y: 3 }),
    ] {
        let positions = (1..=WALK_FRAME_HOLD_TICKS)
            .rev()
            .map(|remaining| {
                visible_player_playfield_position(to, Some(from), remaining, 0, 0)
                    .expect("walk position")
            })
            .collect::<Vec<_>>();
        let initial_position =
            visible_player_playfield_position(from, None, 0, 0, 0).expect("initial tile");
        let final_position =
            visible_player_playfield_position(to, None, 0, 0, 0).expect("final walk position");
        assert_eq!(positions.last().copied(), Some(final_position));

        let dx = (final_position.0 - initial_position.0) / f32::from(WALK_FRAME_HOLD_TICKS);
        let dy = (final_position.1 - initial_position.1) / f32::from(WALK_FRAME_HOLD_TICKS);
        for (frame, position) in positions.iter().enumerate() {
            let completed_frames = (frame + 1) as f32;
            assert_eq!(position.0, initial_position.0 + dx * completed_frames);
            assert_eq!(position.1, initial_position.1 + dy * completed_frames);
        }
    }
}

#[test]
fn walking_camera_scroll_is_interpolated_with_the_player() {
    let rendered = RenderedViewport {
        walk_viewport_origin: Some((10, 8)),
        viewport_origin: Some((12, 8)),
        ..default()
    };

    let initial = overworld_walk_camera_offset(&rendered, WALK_FRAME_HOLD_TICKS);
    let middle = overworld_walk_camera_offset(&rendered, WALK_FRAME_HOLD_TICKS / 2);
    let final_offset = overworld_walk_camera_offset(&rendered, 0);

    assert_eq!(initial, Vec2::new(TILE_SIZE * 1.75, 0.0));
    assert_eq!(middle, Vec2::new(TILE_SIZE * 0.75, 0.0));
    assert_eq!(final_offset, Vec2::ZERO);
}

#[test]
fn camera_following_walk_keeps_player_screen_position_exactly_stable() {
    let rendered = RenderedViewport {
        walk_viewport_origin: Some((10, 10)),
        viewport_origin: Some((12, 12)),
        ..default()
    };
    let from = TilePosition { x: 10, y: 10 };
    let to = TilePosition { x: 11, y: 11 };
    let positions = (0..=WALK_FRAME_HOLD_TICKS)
        .rev()
        .map(|remaining| {
            let player = visible_player_playfield_position(
                to,
                Some(from),
                remaining,
                12,
                12,
            )
            .expect("camera-following player position");
            let camera = overworld_walk_camera_offset(&rendered, remaining);
            Vec2::new(player.0, player.1) + camera
        })
        .collect::<Vec<_>>();

    assert!(
        positions.windows(2).all(|pair| pair[0] == pair[1]),
        "player must not drift against the viewport during a camera-following step: {positions:?}"
    );
}

#[test]
fn held_camera_steps_move_static_objects_without_a_tile_boundary_stutter() {
    let world_x = 15.0 * TILE_SIZE;
    let first_step = RenderedViewport {
        walk_viewport_origin: Some((10, 10)),
        viewport_origin: Some((12, 10)),
        ..default()
    };
    let second_step = RenderedViewport {
        walk_viewport_origin: Some((12, 10)),
        viewport_origin: Some((14, 10)),
        ..default()
    };
    let screen_x = |rendered: &RenderedViewport, remaining| {
        let viewport_x = f32::from(rendered.viewport_origin.expect("viewport origin").0);
        world_x - viewport_x * TILE_SIZE + overworld_walk_camera_offset(rendered, remaining).x
    };

    let positions = [
        screen_x(&first_step, 2),
        screen_x(&first_step, 1),
        screen_x(&second_step, WALK_FRAME_HOLD_TICKS),
        screen_x(&second_step, WALK_FRAME_HOLD_TICKS - 1),
    ];
    let expected_delta = -(2.0 * TILE_SIZE / f32::from(WALK_FRAME_HOLD_TICKS));

    for pair in positions.windows(2) {
        assert_eq!(
            pair[1] - pair[0],
            expected_delta,
            "a static NPC and the map must continue at the same speed across held-step boundaries"
        );
    }
}

#[test]
fn follow_command_steps_advance_on_the_first_visible_frame() {
    for total in [
        WALK_FRAME_HOLD_TICKS / 2,
        WALK_FRAME_HOLD_TICKS,
        WALK_FRAME_HOLD_TICKS * 2,
    ] {
        let progress = (1..=total)
            .rev()
            .map(|remaining| visible_movement_progress(remaining, total))
            .collect::<Vec<_>>();

        assert_eq!(progress[0], 1.0 / f32::from(total));
        assert_eq!(progress.last().copied(), Some(1.0));
        assert!(progress.windows(2).all(|frames| frames[0] < frames[1]));
    }
}

#[test]
fn follow_command_jump_arc_uses_the_advanced_movement_frame() {
    const OFFSETS: [i16; 16] = [
        -4, -6, -8, -10, -11, -12, -12, -12, -11, -10, -9, -8, -6, -4, 0, 0,
    ];
    let total = WALK_FRAME_HOLD_TICKS * 2;

    let first = visible_script_jump_y_offset(&OFFSETS, total, total);
    let landed = visible_script_jump_y_offset(&OFFSETS, total, 1);

    assert!(
        first > 0.0,
        "the follower jump must lift on its first moving frame"
    );
    assert_eq!(
        landed, 0.0,
        "the final moving frame must land on the map tile"
    );
}

#[test]
fn overworld_actor_depth_is_viewport_relative_bounded_and_stably_ordered() {
    let near_origin = overworld_entity_depth(TilePosition { x: 5, y: 7 }, Some(4), (0, 0));
    let tall_map_same_relative =
        overworld_entity_depth(TilePosition { x: 1005, y: 2007 }, Some(4), (1000, 2000));
    assert_eq!(
        near_origin, tall_map_same_relative,
        "absolute tall-map coordinates must not alter on-LCD actor priority"
    );

    let upper_left = overworld_entity_depth(
        TilePosition {
            x: i16::MIN,
            y: i16::MIN,
        },
        Some(0),
        (1000, 2000),
    );
    let lower_right = overworld_entity_depth(
        TilePosition {
            x: i16::MAX,
            y: i16::MAX,
        },
        Some(0),
        (1000, 2000),
    );
    assert!(upper_left < lower_right);
    assert!(
        lower_right < 2.4,
        "ordinary actors must remain below the priority tile plane on every stock map"
    );

    let same_row_left = overworld_entity_depth(TilePosition { x: 4, y: 8 }, Some(8), (0, 0));
    let same_row_right = overworld_entity_depth(TilePosition { x: 5, y: 8 }, Some(8), (0, 0));
    let next_row = overworld_entity_depth(TilePosition { x: 0, y: 9 }, Some(8), (0, 0));
    assert!(same_row_left < same_row_right && same_row_right < next_row);

    let low_slot = overworld_entity_depth(TilePosition { x: 5, y: 8 }, Some(0), (0, 0));
    let high_slot = overworld_entity_depth(TilePosition { x: 5, y: 8 }, Some(15), (0, 0));
    assert!(
        low_slot > high_slot,
        "lower source object slots must win otherwise exact OAM ties"
    );
}

#[test]
fn scripted_poke_ball_objects_use_item_ball_priority() {
    let object = crate::core::map::ObjectEvent {
        sprite: "SPRITE_POKE_BALL".to_string(),
        x: 6,
        y: 3,
        spritemovedata: "SPRITEMOVEDATA_STILL".to_string(),
        move_range_x: 0,
        move_range_y: 0,
        hram_x: -1,
        hram_y: -1,
        pal: 0,
        object_type: "OBJECTTYPE_SCRIPT".to_string(),
        radius: 0,
        script: "CyndaquilPokeBallScript".to_string(),
        label: None,
        event_flag: "EVENT_CYNDAQUIL_POKEBALL_IN_ELMS_LAB".to_string(),
        object_identifier: Some("ELMSLAB_POKE_BALL1".to_string()),
        sightline_direction_override: None,
    };

    assert!(
        object_uses_item_ball_priority(&object),
        "Elm's scripted starter balls must render above the desk priority tiles"
    );
}

#[test]
fn ambient_map_animation_retains_visible_object_entities() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize animated New Bark Town fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let initial_objects = {
        let world = app.world_mut();
        let mut objects = world.query_filtered::<Entity, With<ObjectMarker>>();
        let mut entities = objects.iter(world).collect::<Vec<_>>();
        entities.sort();
        assert!(
            !entities.is_empty(),
            "fixture must have visible map objects"
        );
        entities
    };
    let initial_surfaces = retained_map_surface_pair(app.world_mut());

    for frame in 0..64 {
        app.update();
        let world = app.world_mut();
        let mut objects = world.query_filtered::<Entity, With<ObjectMarker>>();
        let mut entities = objects.iter(world).collect::<Vec<_>>();
        entities.sort();
        assert_eq!(
            entities, initial_objects,
            "ambient redraw frame {frame} must not replace visible object entities"
        );
        assert_eq!(
            retained_map_surface_pair(world),
            initial_surfaces,
            "ambient redraw frame {frame} must retain both map surfaces"
        );
    }
}

#[test]
fn fly_animation_retains_map_objects_without_accumulating_old_effect_frames() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Fly rendering fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let stable_objects = {
        let world = app.world_mut();
        let mut objects = world.query_filtered::<Entity, With<VisibleObjectSprite>>();
        let mut entities = objects.iter(world).collect::<Vec<_>>();
        entities.sort();
        assert!(!entities.is_empty(), "fixture must have stable map objects");
        entities
    };
    app.world_mut()
        .resource_mut::<BevyRuntimeShell>()
        .visible_fly_animation = Some(VisibleFlyAnimation {
            phase: VisibleFlyAnimationPhase::From,
            frame: 0,
        });

    for frame in 0..12 {
        app.update();
        let world = app.world_mut();
        let mut objects = world.query_filtered::<Entity, With<VisibleObjectSprite>>();
        let mut entities = objects.iter(world).collect::<Vec<_>>();
        entities.sort();
        assert_eq!(
            entities, stable_objects,
            "Fly frame {frame} must retain the map's stable NPC entities"
        );

        let transient_count = world
            .query_filtered::<Entity, (With<ObjectMarker>, Without<VisibleObjectSprite>)>()
            .iter(world)
            .count();
        assert!(
            transient_count <= 4,
            "early Fly frame {frame} accumulated stale effect entities: {transient_count}"
        );
    }
}

#[test]
fn live_walk_retains_the_viewport_texture_and_updates_every_lcd_frame() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize walkable New Bark Town fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let start_tile = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot before walking")
        .overworld
        .tile;
    let viewport_surfaces = retained_map_surface_pair(app.world_mut());
    let image_count = app.world().resource::<Assets<Image>>().len();

    app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 16;
    for _ in 0..16 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        if shell.player_walk_frame_ticks == WALK_FRAME_HOLD_TICKS
            && shell
                .shell
                .snapshot()
                .expect("snapshot after walking starts")
                .overworld
                .tile
                .x
                > start_tile.x
        {
            break;
        }
    }
    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .player_walk_frame_ticks,
        WALK_FRAME_HOLD_TICKS,
        "fixture must execute an authoritative walking step"
    );
    let mut player_x_positions = Vec::new();
    let mut base_map_positions = Vec::new();
    let mut priority_map_positions = Vec::new();
    for _ in 0..WALK_FRAME_HOLD_TICKS {
        {
            let world = app.world_mut();
            let current_surfaces = retained_map_surface_pair(world);
            assert_eq!(
                current_surfaces, viewport_surfaces,
                "walking must retain both map entities and texture handles"
            );
            let base_position = world
                .entity(current_surfaces.base_entity)
                .get::<Transform>()
                .expect("retained base map transform")
                .translation
                .truncate();
            let priority_position = world
                .entity(current_surfaces.priority_entity)
                .get::<Transform>()
                .expect("retained priority map transform")
                .translation
                .truncate();
            assert_eq!(
                priority_position, base_position,
                "base and priority map surfaces must scroll together"
            );
            base_map_positions.push(base_position);
            priority_map_positions.push(priority_position);
        }
        assert_eq!(
            app.world().resource::<Assets<Image>>().len(),
            image_count,
            "walking frames must not allocate replacement viewport textures"
        );
        let x = {
            let world = app.world_mut();
            let mut players = world.query_filtered::<&Transform, With<PlayerMarker>>();
            players
                .get_single(world)
                .expect("player sprite remains retained while walking")
                .translation
                .x
        };
        player_x_positions.push(x);
        app.update();
    }
    let player_moves = player_x_positions
        .windows(2)
        .any(|frame| frame[0] != frame[1]);
    let base_map_moves = base_map_positions
        .windows(2)
        .any(|frame| frame[0] != frame[1]);
    let priority_map_moves = priority_map_positions
        .windows(2)
        .any(|frame| frame[0] != frame[1]);
    assert_eq!(
        priority_map_positions, base_map_positions,
        "base and priority map surfaces must have the same scroll transform on every LCD frame"
    );
    assert!(
        base_map_moves && priority_map_moves,
        "both retained map surfaces must visibly scroll during this camera-moving fixture: base={base_map_positions:?}, priority={priority_map_positions:?}"
    );
    assert!(
        player_moves || base_map_moves,
        "each LCD walk must visibly advance the player or retained camera: player={player_x_positions:?}, map={base_map_positions:?}"
    );

    // Repeated press/release steps used to alternate between one player and
    // no player: the full-render boundary despawned the retained sprite and
    // the replacement was not visible to the voxel/classic cameras until a
    // later input. Assert the entity invariant after every presented frame,
    // not merely while the first interpolation is active.
    for step in 0..4 {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .release(KeyCode::ArrowRight);
        app.update();
        app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 12;
        for frame in 0..12 {
            app.update();
            let world = app.world_mut();
            let walking = world
                .resource::<BevyRuntimeShell>()
                .player_walk_frame_ticks
                > 0;
            if walking {
                assert_eq!(
                    world.resource::<RenderedViewport>().player_sprite_walking,
                    Some(true),
                    "step {step} frame {frame} must never show standing art while translating"
                );
            }
            let player_count = world
                .query_filtered::<Entity, With<PlayerMarker>>()
                .iter(world)
                .count();
            assert_eq!(
                player_count, 1,
                "step {step} frame {frame} must retain exactly one player sprite"
            );
        }
    }
}

#[test]
fn positive_and_negative_full_tile_camera_scroll_never_expose_clear_color() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize camera-scroll fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();
    assert_opaque_base_surface_covers_camera(app.world_mut());

    let mut most_positive = 0.0_f32;
    let mut most_negative = 0.0_f32;
    // Move one tile right, return to the origin, then move one tile left so
    // the retained camera is genuinely exercised on both sides of its
    // starting position.
    for key in [KeyCode::ArrowRight, KeyCode::ArrowLeft, KeyCode::ArrowLeft] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        let mut saw_walk = false;
        for _ in 0..=usize::from(WALK_FRAME_HOLD_TICKS) + 2 {
            let (camera_offset, walk_frames) = {
                let world = app.world();
                (
                    visible_overworld_camera_offset(
                        world.resource::<RenderedViewport>(),
                        world.resource::<BevyRuntimeShell>(),
                        0.0,
                    ),
                    world.resource::<BevyRuntimeShell>().player_walk_frame_ticks,
                )
            };
            most_positive = most_positive.max(camera_offset.x);
            most_negative = most_negative.min(camera_offset.x);
            saw_walk |= walk_frames > 0;
            assert_opaque_base_surface_covers_camera(app.world_mut());
            if saw_walk && walk_frames == 0 {
                break;
            }
            app.update();
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset(key);
        let shell = app.world().resource::<BevyRuntimeShell>();
        assert!(
            saw_walk,
            "{key:?} must execute a visible walking step: snapshot={:?} error={:?} events={:?}",
            shell.shell.snapshot(),
            shell.last_error,
            shell.last_audio_events
        );
    }
    assert!(
        most_positive >= TILE_SIZE && most_negative <= -TILE_SIZE,
        "fixture must cover both ±one-tile camera extremes, got {most_negative}..={most_positive}"
    );
}

#[test]
fn direction_tap_between_game_ticks_rotates_player_on_next_tick() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize inter-tick tap fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let initial_tile = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("initial snapshot")
        .overworld
        .tile;
    app.world_mut().resource_mut::<RuntimeTickTimer>().step_seconds = 999.0;

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowUp);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::ArrowUp);
        keys.clear_just_pressed(KeyCode::ArrowUp);
    }
    app.update();

    {
        let snapshot = app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .expect("snapshot before next game tick");
        assert_eq!(snapshot.overworld.tile, initial_tile);
        assert_eq!(snapshot.overworld.facing, Direction::Down);
    }
    let terrain_scans_before_turn = app
        .world()
        .resource::<RenderedViewport>()
        .terrain_scan_count;
    assert!(
        terrain_scans_before_turn > 0,
        "initial rendering must scan terrain"
    );
    {
        let mut timer = app.world_mut().resource_mut::<RuntimeTickTimer>();
        timer.finished_vblanks = 1;
        timer.finished_ticks = 1;
    }
    app.update();

    let snapshot = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("snapshot after next game tick");
    assert_eq!(snapshot.overworld.tile, initial_tile, "a tap turns without walking");
    assert_eq!(snapshot.overworld.facing, Direction::Up);
    assert_eq!(
        app.world().resource::<RenderedViewport>().player_sprite_facing,
        Some(Direction::Up),
        "the latched tap must refresh the retained player sprite"
    );
    assert_eq!(
        app.world()
            .resource::<RenderedViewport>()
            .terrain_scan_count,
        terrain_scans_before_turn,
        "turning in place must update player art without rescanning unchanged terrain"
    );
}

#[test]
fn collision_lookup_uses_canonical_hex_without_heap_formatting() {
    for (block, expected) in [(0, "00"), (0x0f, "0f"), (0xff, "ff"), (0x100, "100")] {
        let mut buffer = [0; 4];
        assert_eq!(tileset_collision_key(block, &mut buffer), expected);
    }
}

#[test]
fn buffered_reversal_refreshes_facing_on_the_next_step_without_replacing_the_lcd() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "NewBarkTown".to_string(),
            tile_x: 13,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize walkable New Bark Town fixture");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    let viewport_surfaces = retained_map_surface_pair(app.world_mut());
    app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 16;
    for _ in 0..16 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .player_walk_frame_ticks
            == WALK_FRAME_HOLD_TICKS
        {
            break;
        }
    }
    let right_texture = {
        let world = app.world_mut();
        let mut players = world.query_filtered::<&Handle<Image>, With<PlayerMarker>>();
        players
            .get_single(world)
            .expect("rightward walking player")
            .clone()
    };

    // Reverse before the prior eight LCD walking frames end. TypeScript and
    // the Game Boy queue that direction until the tile-atomic step lands;
    // the next visible step must begin with matching facing/art.
    app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 0;
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::ArrowRight);
        keys.clear_just_pressed(KeyCode::ArrowRight);
        keys.press(KeyCode::ArrowLeft);
    }
    // The press edge itself owns the queued turn. Releasing before the prior
    // tile lands must not throw it away and leave stale facing at a wall.
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ArrowLeft);
    for _ in 0..=usize::from(WALK_FRAME_HOLD_TICKS) + 4 {
        app.update();
        let shell = app.world().resource::<BevyRuntimeShell>();
        if shell
            .shell
            .snapshot()
            .expect("snapshot while reversing")
            .overworld
            .facing
            == Direction::Left
            && shell.player_walk_frame_ticks > 0
        {
            break;
        }
    }

    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("snapshot after turn");
    assert_eq!(snapshot.overworld.facing, Direction::Left);
    assert!(
        shell.player_walk_frame_ticks > 0,
        "the buffered reversal must start a new retained walking interval"
    );
    let _ = shell;
    let (viewport_after_turn, left_texture) = {
        let world = app.world_mut();
        let viewport = retained_map_surface_pair(world);
        let mut players = world.query_filtered::<&Handle<Image>, With<PlayerMarker>>();
        let player = players
            .get_single(world)
            .expect("turned player sprite")
            .clone();
        (viewport, player)
    };
    assert_eq!(
        viewport_after_turn, viewport_surfaces,
        "a turn must retain both LCD/map surfaces rather than flash or rebuild them"
    );
    assert_ne!(
        left_texture, right_texture,
        "reversing during a walk must replace the stale right-facing frame"
    );
    assert_eq!(
        app.world()
            .resource::<RenderedViewport>()
            .player_sprite_facing,
        Some(Direction::Left),
        "the retained-player identity must match its authoritative facing"
    );
}

#[test]
fn map_name_transition_retains_base_and_priority_surfaces() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let runtime = workspace_desktop_runtime(&asset_root);
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "PlayersHouse2F".to_string(),
            // ASM PlayersHouse2F_MapEvents: warp_event 7, 0,
            // PLAYERS_HOUSE_1F, 3.
            tile_x: 7,
            tile_y: 1,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize immediately below the bedroom warp");
    let mut app = integrated_shell_test_app(runtime_shell);
    app.update();

    assert_eq!(
        app.world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .expect("source-map snapshot")
            .overworld
            .map_name,
        "PlayersHouse2F"
    );
    let initial_surfaces = retained_map_surface_pair(app.world_mut());
    assert_base_map_surface_is_fully_opaque(app.world(), &initial_surfaces);

    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowUp);
    let mut reached_destination_surface = false;
    for _ in 0..256 {
        app.update();
        let current_surfaces = retained_map_surface_pair(app.world_mut());
        assert_eq!(
            current_surfaces, initial_surfaces,
            "a map-name transition must update both retained textures in place"
        );
        if app
            .world()
            .resource::<RenderedViewport>()
            .map_name
            .as_deref()
            == Some("PlayersHouse1F")
        {
            reached_destination_surface = true;
            break;
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::ArrowUp);

    let snapshot = app
        .world()
        .resource::<BevyRuntimeShell>()
        .shell
        .snapshot()
        .expect("destination-map snapshot");
    assert!(
        reached_destination_surface,
        "bedroom warp did not render its destination within the frame budget: runtime={} rendered={:?}",
        snapshot.overworld.map_name,
        app.world().resource::<RenderedViewport>().map_name,
    );
    assert_eq!(snapshot.overworld.map_name, "PlayersHouse1F");
    for _ in 0..2 {
        app.update();
        assert_eq!(
            retained_map_surface_pair(app.world_mut()),
            initial_surfaces,
            "destination idle frames must retain both map surfaces"
        );
    }
}

#[test]
fn autonomous_character_movement_selects_walking_sprite_frames() {
    assert!(object_sprite_is_animated("SPRITEMOVEDATA_WALK_LEFT_RIGHT"));
    assert!(object_sprite_is_animated("SPRITEMOVEDATA_WANDER"));
    assert!(object_sprite_is_animated("SPRITEMOVEDATA_SPINCLOCKWISE"));
    assert!(!object_sprite_is_animated("SPRITEMOVEDATA_STANDING_DOWN"));
    assert!(!object_sprite_is_animated("SPRITEMOVEDATA_STILL"));
}

#[test]
fn chris_sprite_art_loads_real_runtime_assets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();
    let art = load_sprite_art(&asset_root, "chris", 0, "day", &mut images).expect("load chris art");

    for direction in [&art.down, &art.up, &art.left, &art.right] {
        assert_eq!(direction.standing.size, Vec2::splat(64.0));
        assert_eq!(
            direction.walking.as_ref().map(|frame| frame.size),
            Some(Vec2::splat(64.0)),
            "six-frame player sheets must preserve their walking frame"
        );
    }
    assert!(images.len() >= 8);
}

#[test]
fn kris_sprite_art_loads_for_female_player_selection() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();
    let art = load_sprite_art(&asset_root, "kris", 0, "day", &mut images).expect("load kris art");
    assert_eq!(art.down.standing.size, Vec2::splat(64.0));
    assert_eq!(
        art.down.walking.as_ref().map(|frame| frame.size),
        Some(Vec2::splat(64.0))
    );
}

#[test]
fn overworld_colorkey_preserves_enclosed_white_character_pixels() {
    let mut source = image::RgbaImage::from_pixel(5, 5, image::Rgba([255, 255, 255, 255]));
    for row in 1..4 {
        for col in 1..4 {
            source.put_pixel(col, row, image::Rgba([0, 0, 0, 255]));
        }
    }
    let palette = [[255, 255, 255], [170, 170, 170], [85, 85, 85], [0, 0, 0]];
    let mut target = vec![0_u8; 5 * 5 * 4];
    copy_source_sprite_rgba(&source, 5, 0, &palette, false, &mut target);

    assert_eq!(
        target[3], 0,
        "border-connected white background is transparent"
    );
    let center_alpha = target[(2 * 5 + 2) * 4 + 3];
    assert_eq!(center_alpha, 255, "enclosed white artwork remains opaque");
}
