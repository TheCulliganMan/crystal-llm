#[test]
fn field_move_play_refusals_do_not_swallow_unsupported_pack_coverage() {
    assert!(party_field_move_error_is_play_refusal(&anyhow::anyhow!(
        FieldMoveError::MissingBadge {
            move_id: "SURF".to_string(),
            region: "johto".to_string(),
            badge_index: 5,
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
fn viewport_tile_composite_preserves_scaled_tile_grid() {
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
    assert_eq!(image.texture_descriptor.size.width, 640);
    assert_eq!(image.texture_descriptor.size.height, 576);
    assert_eq!(&image.data[0..4], &[0xff, 0x00, 0x00, 0xff]);
    let last = (576 * 640 - 1) * 4;
    assert_eq!(&image.data[last..last + 4], &[0xff, 0x00, 0x00, 0xff]);
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
    assert_eq!(FIELD_TEXT_BOX_TEXT_TOP_TILE, 13.0);
    assert_eq!(FIELD_TEXT_BOX_VISIBLE_ROWS, 4);
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
fn field_dialogue_wraps_pokegear_text_inside_the_four_row_box() {
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
fn player_walk_stride_alternates_on_consecutive_steps() {
    let first = next_player_walk_stride(0, false);
    let second = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, first);
    let third = next_player_walk_stride(WALK_FRAME_HOLD_TICKS, second);

    assert!(first, "the first step starts on the walking frame");
    assert!(!second, "the second consecutive step returns to standing");
    assert!(
        third,
        "the third consecutive step resumes the walking frame"
    );
}

#[test]
fn player_walk_interpolates_each_lcd_frame_between_committed_tiles() {
    for (from, to) in [
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 3, y: 2 }),
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 1, y: 2 }),
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 2, y: 1 }),
        (TilePosition { x: 2, y: 2 }, TilePosition { x: 2, y: 3 }),
    ] {
        let positions = (0..=WALK_FRAME_HOLD_TICKS)
            .rev()
            .map(|remaining| {
                visible_player_playfield_position(to, Some(from), remaining, 0, 0)
                    .expect("walk position")
            })
            .collect::<Vec<_>>();
        let final_position =
            visible_player_playfield_position(to, None, 0, 0, 0).expect("final walk position");
        assert_eq!(
            positions[0],
            visible_player_playfield_position(from, None, 0, 0, 0).expect("initial tile")
        );
        assert_eq!(positions.last().copied(), Some(final_position));

        let dx = (final_position.0 - positions[0].0) / f32::from(WALK_FRAME_HOLD_TICKS);
        let dy = (final_position.1 - positions[0].1) / f32::from(WALK_FRAME_HOLD_TICKS);
        for (frame, position) in positions.iter().enumerate() {
            assert_eq!(position.0, positions[0].0 + dx * frame as f32);
            assert_eq!(position.1, positions[0].1 + dy * frame as f32);
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

    assert_eq!(initial, Vec2::new(TILE_SIZE * 2.0, 0.0));
    assert_eq!(middle, Vec2::new(TILE_SIZE, 0.0));
    assert_eq!(final_offset, Vec2::ZERO);
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
    for key in [KeyCode::ArrowRight, KeyCode::ArrowLeft] {
        press_key_for_runtime_hotkey_app(&mut app, key);
        let mut saw_walk = false;
        for _ in 0..=usize::from(WALK_FRAME_HOLD_TICKS) + 2 {
            let (camera_offset, walk_frames) = {
                let world = app.world();
                (
                    visible_overworld_camera_offset(
                        world.resource::<RenderedViewport>(),
                        world.resource::<BevyRuntimeShell>(),
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
        assert!(saw_walk, "{key:?} must execute a visible walking step");
    }
    assert!(
        most_positive >= TILE_SIZE && most_negative <= -TILE_SIZE,
        "fixture must cover both ±one-tile camera extremes, got {most_negative}..={most_positive}"
    );
}

#[test]
fn reversing_during_a_walk_refreshes_facing_without_replacing_the_lcd() {
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

    // Reverse before the prior eight LCD walking frames end. This is the
    // old moonwalking path: the semantic facing changed but the retained
    // sprite kept the right-facing texture.
    app.world_mut().resource_mut::<HeldArrowRightTestFrames>().0 = 0;
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::ArrowRight);
        keys.clear_just_pressed(KeyCode::ArrowRight);
        keys.press(KeyCode::ArrowLeft);
    }
    for _ in 0..2 {
        app.update();
        if app
            .world()
            .resource::<BevyRuntimeShell>()
            .shell
            .snapshot()
            .expect("snapshot while reversing")
            .overworld
            .facing
            == Direction::Left
        {
            break;
        }
    }

    let shell = app.world().resource::<BevyRuntimeShell>();
    let snapshot = shell.shell.snapshot().expect("snapshot after turn");
    assert_eq!(snapshot.overworld.facing, Direction::Left);
    assert!(
        shell.player_walk_frame_ticks > 0,
        "the reversal must happen inside the retained walking interval"
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
            tile_x: 9,
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
