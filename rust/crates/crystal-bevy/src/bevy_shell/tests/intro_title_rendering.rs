#[test]
fn intro_trigonometry_matches_the_asm_fixed_point_wave() {
    assert_eq!(visible_intro_sine(0x00, 0x20), 0);
    assert_eq!(visible_intro_sine(0x10, 0x20), 0x20);
    assert_eq!(visible_intro_sine(0x30, 0x20), -0x20);
    assert_eq!(visible_intro_cosine(0x00, 0x20), 0x20);
    assert_eq!(visible_intro_cosine(0x20, 0x20), -0x20);
    // The ASM's truncating 8.8 multiply differs from rounded floating
    // point math at this intermediate pulse position.
    assert_eq!(visible_intro_sine(0x08, 0x18), 16);
}

#[test]
fn intro_scanline_scroll_moves_only_the_background_rows() {
    const BACKING_WIDTH: usize = 32 * SOURCE_TILE_SIZE;
    let mut intro = VisibleIntroScreen::new();
    intro.lcdc_pointer = 67;
    intro.ly_overrides[0] = 1;
    let mut target = vec![0_u8; BACKING_WIDTH * BACKING_WIDTH * 4];
    for x in 0..BACKING_WIDTH {
        target[x * 4] = x as u8;
        target[(BACKING_WIDTH + x) * 4] = x as u8;
    }
    apply_visible_intro_scanline_scroll(&intro, &mut target)
        .expect("apply source scanline scroll");
    assert_eq!(target[0], 1);
    assert_eq!(target[(BACKING_WIDTH - 1) * 4], 0);
    assert_eq!(target[BACKING_WIDTH * 4], 0);
}

#[test]
fn intro_cleanup_waits_on_the_source_white_lcd_surface() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    skip_visible_intro_screen(&mut runtime_shell, GameButton::Start)
        .expect("begin CrystalIntro cleanup");
    let intro = runtime_shell
        .intro_screen
        .as_ref()
        .expect("intro remains active for WaitBGMap");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();
    let frame = intro_scene_frame_for_art(
        &mut rendered_art,
        &asset_root,
        intro,
        &mut images,
    )
    .expect("render CrystalIntro cleanup wait");
    let data = &images.get(&frame.handle).expect("cleanup image").data;

    assert!(
        data.chunks_exact(4).all(|pixel| pixel == [u8::MAX; 4]),
        "ClearBGPalettes must expose an opaque white LCD throughout WaitBGMap"
    );
}

#[test]
fn intro_grass_vram_request_changes_tiles_from_exported_resource_table() {
    let runtime_shell = core_modular_title_shell_for_test();
    let program = runtime_shell.runtime.title_presentation_program();
    let mut intro = runtime_shell.intro_screen.clone().expect("intro screen");
    intro.jumptable_index = 8;
    intro.attrmap_palette_overrides =
        visible_intro_attrmap_fills(&intro, program).expect("source attrmap fills");
    intro.jumptable_index = 9;
    apply_visible_intro_background_binding(&mut intro, program).expect("scene 10 background");

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    intro.tile_override = visible_intro_indexed_tile_override(&intro, program, 0)
        .expect("first grass request");
    let first = intro_scene_frame_for_art(
        &mut rendered_art,
        &asset_root,
        &intro,
        &mut images,
    )
    .expect("render first grass frame");
    let first_data = images
        .get(&first.handle)
        .expect("first grass image")
        .data
        .clone();

    intro.tile_override = visible_intro_indexed_tile_override(&intro, program, 8)
        .expect("third grass request");
    let third = intro_scene_frame_for_art(
        &mut rendered_art,
        &asset_root,
        &intro,
        &mut images,
    )
    .expect("render third grass frame");
    let third_data = &images.get(&third.handle).expect("third grass image").data;
    assert_ne!(first_data, *third_data);
}

#[test]
fn intro_scene16_tilemap_xor_changes_only_the_exported_tile_domain() {
    let runtime_shell = core_modular_title_shell_for_test();
    let program = runtime_shell.runtime.title_presentation_program();
    let mut intro = runtime_shell.intro_screen.clone().expect("intro screen");
    intro.jumptable_index = 15;
    apply_visible_intro_background_binding(&mut intro, program).expect("scene 16 background");

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    let first = intro_scene_frame_for_art(
        &mut rendered_art,
        &asset_root,
        &intro,
        &mut images,
    )
    .expect("render first colored Suicune frame");
    let first_data = images
        .get(&first.handle)
        .expect("first colored Suicune image")
        .data
        .clone();

    intro.tilemap_xor_mask = 8;
    let second = intro_scene_frame_for_art(
        &mut rendered_art,
        &asset_root,
        &intro,
        &mut images,
    )
    .expect("render XOR-swapped colored Suicune frame");
    let second_data = &images
        .get(&second.handle)
        .expect("second colored Suicune image")
        .data;
    assert_ne!(first_data, *second_data);
}

#[test]
fn intro_framesets_preserve_asm_durations() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = load_intro_sprite_anim_bundle(
        runtime_shell
            .shell
            .runtime()
            .data()
            .sprite_anim_bundle
            .as_str(),
    )
    .expect("load pack-owned sprite animation bundle");
    let unown = visible_intro_frameset_steps(&bundle, "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1")
        .expect("Unown frameset");
    assert_eq!(
        unown.iter().map(|step| step.duration).collect::<Vec<_>>(),
        vec![3, 3, 7, 0]
    );
    let pichu = visible_intro_frameset_steps(&bundle, "SPRITE_ANIM_FRAMESET_INTRO_PICHU")
        .expect("Pichu frameset");
    assert_eq!(
        pichu.iter().map(|step| step.duration).collect::<Vec<_>>(),
        vec![32, 7, 7, 0]
    );
    assert_eq!(pichu.last().map(|step| step.command.as_str()), Some("end"));
    let unown_f = visible_intro_frameset_steps(&bundle, "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2")
        .expect("Unown F frameset");
    assert_eq!(
        unown_f.iter().map(|step| step.duration).collect::<Vec<_>>(),
        vec![3, 3, 3, 7, 7, 0]
    );
}

#[test]
fn visible_intro_framesets_match_every_pack_owned_asm_step() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = load_intro_sprite_anim_bundle(
        runtime_shell
            .shell
            .runtime()
            .data()
            .sprite_anim_bundle
            .as_str(),
    )
    .expect("load pack-owned sprite animation bundle");

    for object in bundle.objects.values() {
        let Some(frameset_name) = object
            .get("frameset")
            .and_then(serde_json::Value::as_str)
            .filter(|name| name.starts_with("SPRITE_ANIM_FRAMESET_INTRO_"))
        else {
            continue;
        };
        let exported = bundle
            .framesets
            .get(frameset_name)
            .unwrap_or_else(|| panic!("pack object references missing frameset {frameset_name}"));
        let consumed = visible_intro_frameset_steps(&bundle, frameset_name)
            .unwrap_or_else(|_| panic!("visible intro omits pack frameset {frameset_name}"));
        let exported_steps = exported
            .steps
            .iter()
            .map(|step| {
                (
                    step.oam_set.as_deref(),
                    step.duration,
                    step.attr_flags,
                    step.command.as_str(),
                )
            })
            .collect::<Vec<_>>();
        let consumed_steps = consumed
            .iter()
            .map(|step| {
                (
                    step.oam_set.as_deref(),
                    step.duration,
                    step.attr_flags,
                    step.command.as_str(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            consumed_steps, exported_steps,
            "visible intro must consume pack data for {frameset_name}"
        );
    }
}

#[test]
fn visible_intro_animation_timing_is_driven_by_the_pack_frameset() {
    let runtime_shell = core_modular_title_shell_for_test();
    let mut bundle = load_intro_sprite_anim_bundle(
        runtime_shell
            .shell
            .runtime()
            .data()
            .sprite_anim_bundle
            .as_str(),
    )
    .expect("load pack-owned sprite animation bundle");
    bundle
        .framesets
        .get_mut("SPRITE_ANIM_FRAMESET_INTRO_SUICUNE")
        .expect("Suicune frameset")
        .steps[0]
        .duration = 9;

    let mut intro = VisibleIntroScreen::new();
    spawn_visible_intro_sprite(
        &mut intro,
        &bundle,
        "SPRITE_ANIM_OBJ_INTRO_SUICUNE",
        10 * 8,
        9 * 8,
    )
    .expect("spawn Suicune intro sprite");
    update_visible_intro_sprite_animations(&mut intro, &bundle)
        .expect("advance pack-owned Suicune frameset");

    assert_eq!(intro.sprites[0].frame_timer, 8);
    assert_eq!(
        intro.sprites[0].current_oam_set.as_deref(),
        Some("SPRITE_ANIM_OAMSET_INTRO_SUICUNE_1")
    );
}

#[test]
fn visible_intro_sprite_initialization_uses_the_pack_object_definition() {
    let runtime_shell = core_modular_title_shell_for_test();
    let program = runtime_shell
        .runtime
        .data()
        .runtime_title_screen
        .program
        .clone();
    let mut bundle = load_intro_sprite_anim_bundle(
        runtime_shell
            .shell
            .runtime()
            .data()
            .sprite_anim_bundle
            .as_str(),
    )
    .expect("load pack-owned sprite animation bundle");
    let object = bundle
        .objects
        .get_mut("SPRITE_ANIM_OBJ_INTRO_SUICUNE")
        .expect("Suicune object")
        .as_object_mut()
        .expect("typed Suicune object");
    object.insert(
        "frameset".to_string(),
        serde_json::Value::String("SPRITE_ANIM_FRAMESET_INTRO_PICHU".to_string()),
    );
    object.insert(
        "function".to_string(),
        serde_json::Value::String("SPRITE_ANIM_FUNC_SOURCE_TEST".to_string()),
    );

    let mut intro = VisibleIntroScreen::new();
    let sprite = spawn_visible_intro_sprite(
        &mut intro,
        &bundle,
        "SPRITE_ANIM_OBJ_INTRO_SUICUNE",
        10 * 8,
        9 * 8,
    )
    .expect("spawn pack-owned Suicune object");

    assert_eq!(sprite.frameset_name, "SPRITE_ANIM_FRAMESET_INTRO_PICHU");
    assert_eq!(sprite.anim_function, "SPRITE_ANIM_FUNC_SOURCE_TEST");
    let error = apply_visible_intro_sprite_pipeline(&mut intro, &bundle, &program)
        .expect_err("unknown pack animation functions must fail closed")
        .to_string();
    assert!(error.contains("SPRITE_ANIM_FUNC_SOURCE_TEST"), "{error}");
}

#[test]
fn intro_scene_thirteen_uses_the_asm_suicune_initial_position() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    let intro = runtime_shell
        .intro_screen
        .as_mut()
        .expect("title startup has an intro screen");
    intro.jumptable_index = 12;

    assert!(step_visible_intro_scene(&mut runtime_shell).expect("initialize IntroScene13"));

    let intro = runtime_shell
        .intro_screen
        .as_ref()
        .expect("IntroScene13 remains active");
    assert_eq!(intro.sprites.len(), 1);
    assert_eq!(
        (intro.sprites[0].x, intro.sprites[0].y),
        (88, 108),
        "IntroScene13 must use the initial x/y written by engine/movie/intro.asm"
    );
}

#[test]
fn intro_sprite_initial_registers_are_driven_by_the_presentation_program() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .expect("title startup has the sprite bundle");
    let mut program = runtime_shell
        .runtime
        .data()
        .runtime_title_screen
        .program
        .clone();
    let sprite_program = program
        .subprograms
        .iter_mut()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .expect("crystal intro subprogram")
        .sprite_programs
        .iter_mut()
        .find(|sprite_program| {
            sprite_program
                .pointer("/allocation_source_span/start_line")
                .and_then(serde_json::Value::as_u64)
                == Some(673)
        })
        .expect("IntroScene13 Suicune program");
    sprite_program["initial_memory"]["xcoord"] = serde_json::json!(77);
    sprite_program["initial_memory"]["ycoord"] = serde_json::json!(66);
    sprite_program["initial_memory"]["tile_id"] = serde_json::json!(55);
    sprite_program["initial_memory"]["var2"] = serde_json::json!(9);

    let mut intro = VisibleIntroScreen::new();
    intro.jumptable_index = 12;
    spawn_visible_intro_sprite_program_group(&mut intro, bundle, &program)
        .expect("spawn mutated pack-owned sprite program");

    assert_eq!((intro.sprites[0].x, intro.sprites[0].y), (77, 66));
    assert_eq!(intro.sprites[0].tile_id, 55);
    assert_eq!(intro.sprites[0].var2, 9);
}

#[test]
fn intro_sprite_graphics_are_driven_by_the_allocation_vram_binding() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .expect("title startup has the sprite bundle");
    let program = &runtime_shell.runtime.data().runtime_title_screen.program;

    let mut jump_scene = VisibleIntroScreen::new();
    jump_scene.jumptable_index = 14;
    spawn_visible_intro_sprite_program_group(&mut jump_scene, bundle, program)
        .expect("spawn Scene15 grass streak");
    assert_eq!(jump_scene.sprites[1].gfx_name, "grass4");

    let mut back_scene = VisibleIntroScreen::new();
    back_scene.jumptable_index = 18;
    spawn_visible_intro_sprite_program_group(&mut back_scene, bundle, program)
        .expect("spawn Scene19 grass streak");
    assert_eq!(back_scene.sprites[0].gfx_name, "grass4");

    let mut mutated_program = program.clone();
    let binding = mutated_program
        .subprograms
        .iter_mut()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .expect("crystal intro subprogram")
        .sprite_programs
        .iter_mut()
        .find(|sprite_program| {
            sprite_program
                .pointer("/allocation_source_span/start_line")
                .and_then(serde_json::Value::as_u64)
                == Some(787)
        })
        .expect("Scene15 streak program");
    binding["graphic_binding"]["resource"] =
        serde_json::json!("gfx/intro/pulse.2bpp.lz");
    binding["graphic_binding"]["tile_base"] = serde_json::json!(7);
    let mut source_probe = VisibleIntroScreen::new();
    source_probe.jumptable_index = 14;
    spawn_visible_intro_sprite_program_group(&mut source_probe, bundle, &mutated_program)
        .expect("spawn mutated source-bound sprite");
    assert_eq!(source_probe.sprites[1].gfx_name, "pulse");
    assert_eq!(source_probe.sprites[1].gfx_tile_base, 7);
}

#[test]
fn intro_background_resources_are_driven_by_exported_vram_bindings() {
    let mut program = core_modular_title_shell_for_test()
        .runtime
        .data()
        .runtime_title_screen
        .program
        .clone();
    let binding = program
        .subprograms
        .iter_mut()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .expect("crystal intro subprogram")
        .phases
        .iter_mut()
        .find(|phase| phase.id == "scene_dispatch")
        .expect("scene dispatch phase")
        .operations
        .iter_mut()
        .find(|operation| {
            operation.op == "intro_background_binding"
                && operation
                    .fields
                    .get("dispatcher_entry")
                    .and_then(serde_json::Value::as_u64)
                    == Some(14)
        })
        .expect("IntroScene15 background binding");
    binding.fields.insert(
        "tilemap_resource".to_string(),
        serde_json::json!("gfx/intro/unown_a.tilemap.lz"),
    );
    binding.fields.insert(
        "palette_resource".to_string(),
        serde_json::json!("gfx/intro/unowns.pal"),
    );

    let mut intro = VisibleIntroScreen::new();
    intro.jumptable_index = 14;
    apply_visible_intro_background_binding(&mut intro, &program)
        .expect("apply mutated pack background binding");
    let background = intro
        .background_binding
        .expect("source-derived background binding");
    assert_eq!(background.tilemap_resource, "gfx/intro/unown_a.tilemap.lz");
    assert_eq!(background.palette_resource, "gfx/intro/unowns.pal");
    assert!(
        background
            .tile_bindings
            .iter()
            .any(|tiles| tiles.resource == "gfx/intro/suicune_jump.2bpp.lz")
    );
}

#[test]
fn intro_scene_nineteen_preserves_the_asm_departing_suicune_tile_base() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    let intro = runtime_shell
        .intro_screen
        .as_mut()
        .expect("title startup has an intro screen");
    intro.jumptable_index = 18;

    assert!(step_visible_intro_scene(&mut runtime_shell).expect("initialize IntroScene19"));

    let sprite = &runtime_shell
        .intro_screen
        .as_ref()
        .expect("IntroScene19 remains active")
        .sprites[0];
    assert_eq!(sprite.tile_id, 0x7f);
}

#[test]
fn intro_unown_f_callback_uses_the_wslotsdelay_frame_counter_alias() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    let intro = runtime_shell
        .intro_screen
        .as_mut()
        .expect("title startup has an intro screen");
    intro.jumptable_index = 14;
    assert!(step_visible_intro_scene(&mut runtime_shell).expect("initialize IntroScene15"));

    let intro = runtime_shell
        .intro_screen
        .as_mut()
        .expect("IntroScene15 remains active");
    intro.scene_frame_counter = 0x40;
    apply_visible_intro_sprite_pipeline_for_shell(&mut runtime_shell)
        .expect("run the exported intro sprite callbacks");

    let unown_f = runtime_shell
        .intro_screen
        .as_ref()
        .expect("IntroScene15 remains active")
        .sprites
        .iter()
        .find(|sprite| sprite.object_name == "SPRITE_ANIM_OBJ_INTRO_UNOWN_F")
        .expect("Unown F sprite");
    assert_eq!(
        unown_f.frameset_name,
        "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2"
    );
    assert_eq!(unown_f.frameset_step, 0);
}

#[test]
fn intro_callback_frameset_reinitialization_is_driven_by_exported_instructions() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .expect("title startup has the sprite bundle");
    let mut program = runtime_shell
        .runtime
        .data()
        .runtime_title_screen
        .program
        .clone();
    let sprite_programs = &mut program
        .subprograms
        .iter_mut()
        .find(|subprogram| subprogram.id == "crystal_intro")
        .expect("crystal intro subprogram")
        .sprite_programs;
    let mut mutated = 0;
    for callback in sprite_programs.iter_mut().filter_map(|sprite_program| {
        let callback = sprite_program.get_mut("callback")?;
        (callback.get("symbol")?.as_str()? == "SPRITE_ANIM_FUNC_INTRO_SUICUNE").then_some(callback)
    }) {
        let instruction = callback["instructions"]
            .as_array_mut()
            .expect("callback instruction list")
            .iter_mut()
            .find(|instruction| {
                instruction["opcode"] == "ld"
                    && instruction["args"]
                        == serde_json::json!(["a", "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2"])
            })
            .expect("Suicune frameset load instruction");
        instruction["args"][1] = serde_json::json!("SPRITE_ANIM_FRAMESET_INTRO_PICHU");
        mutated += 1;
    }
    assert!(mutated > 0, "Suicune callback program");

    let mut intro = VisibleIntroScreen::new();
    intro.scene_timer = 1;
    let sprite = spawn_visible_intro_sprite(
        &mut intro,
        bundle,
        "SPRITE_ANIM_OBJ_INTRO_SUICUNE",
        10 * 8,
        9 * 8,
    )
    .expect("spawn Suicune intro sprite");
    sprite.frameset_step = 2;
    sprite.frame_timer = 4;

    apply_visible_intro_sprite_pipeline(&mut intro, bundle, &program)
        .expect("execute mutated exported callback instructions");

    assert_eq!(
        intro.sprites[0].frameset_name,
        "SPRITE_ANIM_FRAMESET_INTRO_PICHU"
    );
    assert_eq!(intro.sprites[0].frameset_step, 0);
}

#[test]
fn intro_suicune_away_callback_wraps_its_byte_sized_y_coordinate() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .expect("title startup has the sprite bundle");
    let program = &runtime_shell.runtime.data().runtime_title_screen.program;
    let mut intro = VisibleIntroScreen::new();
    spawn_visible_intro_sprite(
        &mut intro,
        bundle,
        "SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY",
        0,
        0xf0,
    )
    .expect("spawn departing Suicune");

    apply_visible_intro_sprite_pipeline(&mut intro, bundle, program)
        .expect("run departing Suicune callback");

    assert_eq!(intro.sprites[0].y, 0);
}

#[test]
fn intro_callback_interpreter_executes_source_branches_stack_and_math() {
    let runtime_shell = core_modular_title_shell_for_test();
    let bundle = runtime_shell
        .intro_sprite_bundle
        .as_ref()
        .expect("title startup has the sprite bundle");
    let program = &runtime_shell.runtime.data().runtime_title_screen.program;

    let mut pichu = VisibleIntroScreen::new();
    let sprite =
        spawn_visible_intro_sprite(&mut pichu, bundle, "SPRITE_ANIM_OBJ_INTRO_PICHU", 0, 0)
            .expect("spawn Pichu");
    sprite.var1 = 20;
    sprite.y_offset = 7;
    apply_visible_intro_sprite_pipeline(&mut pichu, bundle, program)
        .expect("execute Pichu callback");
    assert_eq!(pichu.sprites[0].var1, 20);
    assert_eq!(pichu.sprites[0].y_offset, 7);

    let mut suicune = VisibleIntroScreen::new();
    let sprite =
        spawn_visible_intro_sprite(&mut suicune, bundle, "SPRITE_ANIM_OBJ_INTRO_SUICUNE", 0, 0)
            .expect("spawn Suicune");
    sprite.y_offset = 9;
    apply_visible_intro_sprite_pipeline(&mut suicune, bundle, program)
        .expect("execute inactive Suicune callback");
    assert_eq!(suicune.sprites[0].y_offset, 9);

    let mut unown = VisibleIntroScreen::new();
    let sprite =
        spawn_visible_intro_sprite(&mut unown, bundle, "SPRITE_ANIM_OBJ_INTRO_UNOWN", 0, 0)
            .expect("spawn Unown");
    sprite.var1 = 0x10;
    sprite.jumptable_index = 5;
    apply_visible_intro_sprite_pipeline(&mut unown, bundle, program)
        .expect("execute Unown callback");
    assert_eq!(unown.sprites[0].jumptable_index, 8);
    assert_eq!(unown.sprites[0].y_offset, 5);
    assert_eq!(unown.sprites[0].x_offset, 0);
}

#[test]
fn title_art_loads_real_runtime_assets() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();

    let logo =
        load_title_frame(&asset_root, "logo", 1, true, &mut images).expect("load title logo");
    let crystal =
        load_title_frame(&asset_root, "crystal", 1, true, &mut images).expect("load title crystal");
    let suicune =
        load_title_frame(&asset_root, "suicune", 8, true, &mut images).expect("load title suicune");
    let copyright = load_title_frame(&asset_root, "copyright", 1, true, &mut images)
        .expect("load title copyright");

    assert_eq!(logo.size, Vec2::new(160.0, 64.0));
    assert_eq!(crystal.size, Vec2::new(48.0, 80.0));
    assert_eq!(suicune.size, Vec2::new(128.0, 128.0));
    assert_eq!(copyright.size, Vec2::new(232.0, 8.0));
    assert_eq!(images.len(), 4);
}

#[test]
fn title_art_rejects_an_out_of_range_palette_instead_of_using_the_first_bank() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut images = Assets::<Image>::default();

    let error = match load_title_frame(&asset_root, "logo", u8::MAX, true, &mut images) {
        Ok(_) => panic!("an invalid title palette must fail closed"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("title asset logo references palette 255"));
    assert_eq!(images.len(), 0);
}

#[test]
fn title_suicune_bg_tiles_keep_palette_color_zero_opaque() {
    let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
    let palette = [[11, 22, 33], [44, 55, 66], [77, 88, 99], [111, 122, 133]];
    let mut target = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];

    blit_native_title_tile(
        &source,
        0,
        &palette,
        false,
        0,
        0,
        NativeTitleScroll::None,
        &mut target,
        None,
    );

    assert_eq!(
        &target[..4],
        &[11, 22, 33, 255],
        "ASM draws Suicune through the opaque BG layer; color zero must not punch holes in its head"
    );
}

#[test]
fn intro_suicune_bg_tiles_preserve_exported_alpha_like_typescript() {
    let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
    let palette = [[13, 24, 35], [44, 55, 66], [77, 88, 99], [111, 122, 133]];
    let mut target = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE * 4];

    blit_intro_source_tile(
        &source,
        8,
        0,
        &palette,
        false,
        false,
        false,
        0,
        0,
        &mut target,
    );

    assert_eq!(
        &target[..4],
        &[0, 0, 0, 0],
        "the TypeScript compositor leaves exported alpha transparent instead of painting a palette-zero tile rectangle"
    );
}

#[test]
fn native_title_screen_frame_uses_title_palettes_and_window_layer() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let title = runtime_shell.title_menu.clone().expect("title menu");
    let mut images = Assets::<Image>::default();

    let entrance_frame = load_title_screen_frame(&runtime_shell.asset_root, &title, &mut images)
        .expect("render title entrance frame");
    let entrance_data = images
        .get(&entrance_frame.handle)
        .expect("title entrance image")
        .data
        .clone();
    assert_eq!(entrance_frame.size, Vec2::new(160.0, 144.0));
    assert_opaque_nonblack_lcd_pixels(&entrance_data, "title entrance");
    assert!(
        entrance_data
            .chunks_exact(4)
            .any(|pixel| pixel[3] == 255 && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)),
        "native title frame must contain real palette-colored title art"
    );

    let mut main_title = title.clone();
    main_title
        .presentation_machine
        .memory
        .insert("wJumptableIndex".to_string(), 0x82);
    main_title
        .presentation_machine
        .memory
        .insert("wTitleScreenSelectedOption".to_string(), 0);
    main_title
        .presentation_machine
        .memory
        .insert("hSCX".to_string(), 0);
    let main_frame = load_title_screen_frame(&runtime_shell.asset_root, &main_title, &mut images)
        .expect("render title main frame");
    let main_data = images
        .get(&main_frame.handle)
        .expect("title main image")
        .data
        .clone();
    assert_ne!(
        entrance_data, main_data,
        "main title frame must include the version window layer absent during entrance"
    );
}

#[test]
fn title_teardown_renders_the_source_cleared_palette_surface() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    finish_and_drain_visible_intro_for_test(&mut runtime_shell, "test")
        .expect("finish intro");
    advance_visible_title_to_press_start(&mut runtime_shell);
    press_visible_title_confirm_button(&mut runtime_shell, GameButton::Start)
        .expect("start title teardown");
    let title = runtime_shell.title_menu.as_ref().expect("title menu");
    assert!(matches!(title.source_phase(), VisibleTitlePhase::Teardown));

    let mut images = Assets::<Image>::default();
    let frame = load_title_screen_frame(&runtime_shell.asset_root, title, &mut images)
        .expect("render title teardown");
    let image = images.get(&frame.handle).expect("teardown image");
    assert!(
        image
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]),
        "ClearPalettes must expose a fully white LCD surface throughout the timed title teardown"
    );
}

#[test]
fn native_title_layers_use_asm_scy_and_wy_coordinates() {
    let entrance_scroll = NativeTitleScroll::EntranceInterlaced(112);
    assert_eq!(entrance_scroll.at_scanline(0), 112);
    assert_eq!(entrance_scroll.at_scanline(1), 144);
    assert_eq!(entrance_scroll.at_scanline(80), 0);

    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let mut title = runtime_shell.title_menu.take().expect("title menu");
    title
        .presentation_machine
        .memory
        .insert("wJumptableIndex".to_string(), 2);
    title
        .presentation_machine
        .values
        .insert("title_suicune_frame".to_string(), 0);
    title
        .presentation_machine
        .memory
        .insert("hSCX".to_string(), 0);

    let logo = image::RgbaImage::from_pixel(160, 64, image::Rgba([0, 0, 0, 255]));
    let suicune = image::RgbaImage::from_pixel(128, 128, image::Rgba([0, 0, 0, 255]));
    let palette_bank = (0_u8..9)
        .map(|palette| {
            [
                [palette, 0, 0],
                [palette, 1, 0],
                [palette, 2, 0],
                [palette, 3, 0],
            ]
        })
        .collect::<Vec<Palette>>();
    let mut target = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];
    let mut priority_map = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];

    draw_native_title_background(
        &logo,
        &suicune,
        &palette_bank,
        &title,
        &mut target,
        &mut priority_map,
    )
    .expect("draw title BG");

    let logo_top =
        (TITLE_LOGO_ASM_Y_TILE * SOURCE_TILE_SIZE - TITLE_BG_SCY) * TITLE_SCREEN_WIDTH * 4;
    assert_eq!(
        &target[logo_top..logo_top + 4],
        &[2, 3, 0, 255],
        "ASM hlcoord 0,3 must appear at y=16 after SCY=8"
    );
    let suicune_top = ((TITLE_SUICUNE_ASM_Y_TILE * SOURCE_TILE_SIZE - TITLE_BG_SCY)
        * TITLE_SCREEN_WIDTH
        + 6 * SOURCE_TILE_SIZE)
        * 4;
    assert_eq!(
        &target[suicune_top..suicune_top + 4],
        &[0, 3, 0, 255],
        "ASM hlcoord 6,12 must place Suicune at visible y=88 after SCY=8"
    );

    target.fill(0);
    priority_map.fill(0);
    draw_native_title_version_window(&logo, &palette_bank, &mut target, &mut priority_map)
        .expect("draw title window");
    let copyright_top = TITLE_VERSION_WINDOW_Y * TITLE_SCREEN_WIDTH * 4
        + TITLE_VERSION_TEXT_START_COLUMN * SOURCE_TILE_SIZE * 4;
    assert_eq!(
        &target[copyright_top..copyright_top + 4],
        &[7, 3, 0, 255],
        "ASM WY=$88 must place the copyright window on the bottom scanline row"
    );
    assert_eq!(
        &target[TITLE_VERSION_TEXT_START_COLUMN * SOURCE_TILE_SIZE * 4
            ..TITLE_VERSION_TEXT_START_COLUMN * SOURCE_TILE_SIZE * 4 + 4],
        &[0, 0, 0, 0],
        "copyright must not be drawn at the top of the title"
    );
}

#[test]
fn native_title_preserves_every_suicune_pixel_including_the_head() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let mut title = runtime_shell.title_menu.take().expect("title menu");
    title
        .presentation_machine
        .memory
        .insert("wJumptableIndex".to_string(), 2);
    title
        .presentation_machine
        .values
        .insert("title_suicune_frame".to_string(), 0);
    title
        .presentation_machine
        .memory
        .insert("hSCX".to_string(), 0);
    let mut images = Assets::<Image>::default();
    let source = image::open(
        runtime_shell
            .asset_root
            .runtime_assets()
            .join("gfx/title/suicune.png"),
    )
    .expect("load Suicune source")
    .to_rgba8();
    let palette =
        load_title_palette_bank(&runtime_shell.asset_root).expect("load title palettes")[0];

    let origin_x = 6 * SOURCE_TILE_SIZE;
    let origin_y = TITLE_SUICUNE_ASM_Y_TILE * SOURCE_TILE_SIZE - TITLE_BG_SCY;
    for (scene, scx, label) in [
        (2, 0, "settled"),
        (0, 112, "entrance"),
    ] {
        title
            .presentation_machine
            .memory
            .insert("wJumptableIndex".to_string(), scene);
        title
            .presentation_machine
            .memory
            .insert("hSCX".to_string(), scx);
        let frame = load_title_screen_frame(&runtime_shell.asset_root, &title, &mut images)
            .expect("render title frame");
        let actual = &images.get(&frame.handle).expect("title image").data;
        for y in 0..6 * SOURCE_TILE_SIZE {
            for x in 0..8 * SOURCE_TILE_SIZE {
                let source_pixel = source.get_pixel(x as u32, y as u32);
                let expected = palette[palette_index_from_gray(source_pixel[0])];
                let offset = ((origin_y + y) * TITLE_SCREEN_WIDTH + origin_x + x) * 4;
                assert_eq!(
                    &actual[offset..offset + 4],
                    &[expected[0], expected[1], expected[2], 255],
                    "{label} Suicune frame-0 pixel ({x},{y}) changed during title composition"
                );
            }
        }
    }
}

// This is only a source-image invariant. Presentation tests below separately
// prove that Bevy attaches these pixels to a visible, correctly scaled entity.
fn assert_opaque_nonblack_lcd_pixels(data: &[u8], screen: &str) {
    assert_eq!(
        data.len(),
        160 * 144 * 4,
        "{screen} must compose one native 160x144 LCD"
    );
    assert!(
        data.chunks_exact(4).all(|pixel| pixel[3] == 255),
        "{screen} must be opaque so no previous scene can bleed through"
    );
    assert!(
        data.chunks_exact(4)
            .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0),
        "{screen} must not present an all-black frame"
    );
}

#[test]
fn native_title_crystal_pixels_respect_bg_window_priority() {
    let mut crystal = image::RgbaImage::new(1, 1);
    crystal.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
    let palette: Palette = [[0, 0, 0], [80, 80, 80], [160, 160, 160], [240, 16, 32]];
    let mut target = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT * 4];
    target[0..4].copy_from_slice(&[1, 2, 3, 255]);
    let mut priority_map = vec![0_u8; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    priority_map[0] = 2;

    blit_native_title_image_with_priority(
        &crystal,
        &palette,
        true,
        0,
        0,
        0,
        &priority_map,
        &mut target,
    );
    assert_eq!(
        &target[0..4],
        &[1, 2, 3, 255],
        "Title crystal OAM priority must not draw over non-zero BG/WIN pixels"
    );

    priority_map[0] = 0;
    blit_native_title_image_with_priority(
        &crystal,
        &palette,
        true,
        0,
        0,
        0,
        &priority_map,
        &mut target,
    );
    assert_eq!(
        &target[0..4],
        &[240, 16, 32, 255],
        "Title crystal pixels should draw over BG/WIN color index zero"
    );
}

#[test]
fn native_title_suicune_uses_the_source_preincrement_frame_counter() {
    let index = |counter| native_title_suicune_frame_index(counter, 0x18, 1, true);
    assert_eq!(index(0), 0);
    assert_eq!(index(1), 0);
    assert_eq!(index(8), 1);
    assert_eq!(index(9), 1);
    assert_eq!(index(17), 2);
    assert_eq!(index(25), 3);
    assert_eq!(index(33), 0);
}

#[test]
fn visible_title_screen_spawns_only_native_frame_without_status_text_overlay() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    let mut title = runtime_shell.title_menu.clone().expect("title menu");
    title
        .presentation_machine
        .memory
        .insert("wJumptableIndex".to_string(), 2);
    title
        .presentation_machine
        .memory
        .insert("hSCX".to_string(), 0);
    runtime_shell.title_menu = Some(title);

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, render_playfield);

    app.update();

    let runtime_shell = app.world().resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    let mut query = app
        .world_mut()
        .query_filtered::<&Sprite, With<TitleScreenMarker>>();
    let sizes = query
        .iter(app.world())
        .filter_map(|sprite| sprite.custom_size)
        .collect::<Vec<_>>();
    assert_eq!(
        sizes,
        vec![Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)],
        "title screen should render only the native 160x144 title frame scaled to the playfield, with no Rust-only status text overlay"
    );
}

#[test]
fn title_main_menu_frame_uses_source_window_layout_without_title_overlay() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    finish_and_drain_visible_intro_for_test(&mut runtime_shell, "test")
        .expect("finish intro");
    advance_visible_title_to_press_start(&mut runtime_shell);
    open_visible_title_main_menu(&mut runtime_shell).expect("open main menu");
    let title = runtime_shell.title_menu.clone().expect("title menu");
    assert_eq!(visible_title_main_menu_item_tile_y(&title, 0), 2);
    assert_eq!(visible_title_main_menu_item_tile_y(&title, 1), 4);
    assert_eq!(visible_title_main_menu_item_tile_y(&title, 2), 6);
    let mut images = Assets::<Image>::default();
    let mut rendered_art = RenderedTilesetArt::default();

    let frame =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
            .expect("render title main menu frame");
    assert!(rendered_art.title_menu_font_source.is_some());
    assert!(rendered_art.title_menu_frame_source.is_some());
    let image = images.get(&frame.handle).expect("main menu image");
    assert_eq!(
        image.texture_descriptor.size.width,
        (20 * SOURCE_TILE_SIZE) as u32
    );
    assert_eq!(
        image.texture_descriptor.size.height,
        (18 * SOURCE_TILE_SIZE) as u32
    );
    assert_eq!(
        &image.data[0..4],
        &[255, 255, 255, 255],
        "MainMenu draws over a white background, not the animated title art"
    );
    assert!(
        image
            .data
            .chunks_exact(4)
            .any(|pixel| { pixel[3] == 255 && (pixel[0] < 64 || pixel[1] < 64 || pixel[2] < 64) }),
        "main menu frame should include window borders and bitmap glyph pixels"
    );
    let time_box_sample =
        ((TITLE_MAIN_MENU_TIME_BOX_Y * SOURCE_TILE_SIZE + 1) * TITLE_SCREEN_WIDTH + 1) * 4;
    assert_eq!(
        &image.data[time_box_sample..time_box_sample + 4],
        &[255, 255, 255, 255],
        "without a continue save, the TypeScript main menu does not draw the time box"
    );
}

#[test]
fn title_main_menu_static_cursor_does_not_move_between_redraws() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    finish_and_drain_visible_intro_for_test(&mut runtime_shell, "test")
        .expect("finish intro");
    advance_visible_title_to_press_start(&mut runtime_shell);
    open_visible_title_main_menu(&mut runtime_shell).expect("open main menu");
    let mut title = runtime_shell.title_menu.clone().expect("title menu");
    let mut images = Assets::<Image>::default();
    let mut rendered_art = RenderedTilesetArt::default();

    let still =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
            .expect("render unbobbed main menu");
    let still_data = images
        .get(&still.handle)
        .expect("unbobbed image")
        .data
        .clone();
    title
        .presentation_machine
        .values
        .insert("title_suicune_frame".to_string(), 24);
    let redrawn =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
            .expect("redraw static main menu");
    let redrawn_data = images
        .get(&redrawn.handle)
        .expect("redrawn image")
        .data
        .clone();

    assert_eq!(
        still_data, redrawn_data,
        "ASM STATICMENU_CURSOR must remain on its exact menu row across redraws"
    );
}

#[test]
fn title_main_menu_draws_immediately_without_a_host_fade() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    finish_and_drain_visible_intro_for_test(&mut runtime_shell, "test")
        .expect("finish intro");
    advance_visible_title_to_press_start(&mut runtime_shell);
    open_visible_title_main_menu(&mut runtime_shell).expect("open main menu");
    let title = runtime_shell.title_menu.clone().expect("title menu");
    let mut images = Assets::<Image>::default();
    let mut rendered_art = RenderedTilesetArt::default();

    let frame =
        load_visible_title_main_menu_frame(&runtime_shell, &title, &mut rendered_art, &mut images)
            .expect("render source main menu frame");
    let image = images.get(&frame.handle).expect("main menu image");
    assert_eq!(
        &image.data[0..4],
        &[255, 255, 255, 255],
        "ASM MainMenu draws its white cleared tilemap immediately"
    );
}

#[test]
fn title_scene_spawns_real_art_entities_from_compiled_pack() {
    fn spawn_title_scene_once(
        mut commands: Commands,
        mut runtime_shell: ResMut<BevyRuntimeShell>,
        mut rendered_art: ResMut<RenderedTilesetArt>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let title = runtime_shell
            .title_menu
            .clone()
            .expect("title menu is active");
        spawn_title_screen(
            &mut commands,
            &mut runtime_shell,
            &title,
            &mut rendered_art,
            &mut images,
        )
        .expect("spawn real title screen art");
    }

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
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize title shell");

    let mut app = App::new();
    app.insert_resource(runtime_shell)
        .insert_resource(RenderedTilesetArt::default())
        .init_resource::<Assets<Image>>()
        .add_systems(Update, spawn_title_scene_once);
    app.update();

    let world = app.world();
    let runtime_shell = world.resource::<BevyRuntimeShell>();
    assert_eq!(runtime_shell.last_error, None);
    let rendered_art = world.resource::<RenderedTilesetArt>();
    assert_eq!(rendered_art.title_screen_cache.len(), 1);
    assert!(rendered_art.title_screen_errors.is_empty());
    assert_eq!(rendered_art.font_error, None);
    assert!(
        !world.resource::<Assets<Image>>().is_empty(),
        "title scene should include the composed native title frame"
    );

    let world = app.world_mut();
    let mut title_entities = world.query_filtered::<Entity, With<TitleScreenMarker>>();
    assert_eq!(
        title_entities.iter(world).count(),
        1,
        "title scene should spawn one composed native title surface, with no Rust-only glyph overlay"
    );
}

#[test]
fn intro_surface_preserves_the_native_lcd_aspect_at_integer_scale() {
    assert_eq!(visible_intro_display_size(), Vec2::new(640.0, 576.0));
    assert_eq!(
        visible_intro_display_size(),
        Vec2::new(
            TITLE_SCREEN_WIDTH as f32 * (TILE_SIZE / SOURCE_TILE_SIZE as f32),
            TITLE_SCREEN_HEIGHT as f32 * (TILE_SIZE / SOURCE_TILE_SIZE as f32),
        ),
        "the 160x144 LCD must be shown at four-times integer scale, never as a square"
    );
}

#[test]
fn intro_scene_renderer_uses_real_asm_tilemap_art_not_debug_text() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let intro = VisibleIntroScreen::new();
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    let frame = intro_scene_frame_for_art(&mut rendered_art, &asset_root, &intro, &mut images)
        .expect("render first intro scene from ASM tilemap art");
    let image = images.get(&frame.handle).expect("intro image asset");

    assert_eq!(
        image.texture_descriptor.size.width,
        20 * SOURCE_TILE_SIZE as u32
    );
    assert_eq!(
        image.texture_descriptor.size.height,
        18 * SOURCE_TILE_SIZE as u32
    );
    assert!(
        image.data.chunks_exact(4).any(|pixel| pixel[3] != 0),
        "intro renderer should produce visible tile pixels"
    );
    assert!(
        rendered_art.intro_presented_surface.is_some(),
        "intro rendering must retain one LCD texture instead of allocating a frame cache"
    );
    assert!(rendered_art.intro_scene_errors.is_empty());
    assert!(
        rendered_art.font_cache.is_none(),
        "intro scene rendering must not fall back to bitmap debug text"
    );

    // The opening frame is intentionally black while the first palette
    // fades in.  A settled background scene must not remain black: this
    // catches a broken palette/tilemap path rather than treating alpha as
    // evidence that the player can see the intro.
    let mut settled = intro;
    settled.jumptable_index = 3;
    settled.scene_frame_counter = 0x20;
    let settled_frame =
        intro_scene_frame_for_art(&mut rendered_art, &asset_root, &settled, &mut images)
            .expect("render settled intro background scene");
    let settled_image = images
        .get(&settled_frame.handle)
        .expect("settled intro image asset");
    assert!(
        settled_image
            .data
            .chunks_exact(4)
            .any(|pixel| pixel[0] > 12 || pixel[1] > 12 || pixel[2] > 12),
        "a settled intro scene must contain lit Game Boy pixels"
    );
}

#[test]
fn intro_suicune_close_head_uses_its_asm_palette_banks() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut intro = VisibleIntroScreen::new();
    intro.jumptable_index = 17;
    intro.scroll_x = 0x60;
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    let frame = intro_scene_frame_for_art(&mut rendered_art, &asset_root, &intro, &mut images)
        .expect("render Suicune close-up head frame");
    let image = images.get(&frame.handle).expect("Suicune close-up image");
    let colors = image
        .data
        .chunks_exact(4)
        .filter(|pixel| pixel[3] != 0)
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<BTreeSet<_>>();

    // `IntroScene17` loads IntroSuicuneClosePalette. These two colors come
    // from its palette banks 2-4 and do not exist in IntroSuicunePalette.
    // Their presence proves that the colored head is not flattened into the
    // generic orange Suicune background palette.
    assert!(
        colors.contains(&[99, 165, 255]),
        "Suicune's head must contain the close-up palette's light blue"
    );
    assert!(
        colors.contains(&[156, 66, 255]),
        "Suicune's head must contain the close-up palette's purple"
    );
}

#[test]
fn representative_intro_latched_scene_boundaries_match_the_asm_rom_oracle() {
    use sha2::{Digest, Sha256};

    let mut runtime_shell = core_modular_title_shell_for_test();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let sprite_bundle = crate::read_runtime_asset_to_string(
        &asset_root
            .runtime_assets()
            .join("data/sprite_anim_bundle.json"),
    )
    .expect("read sprite animation bundle");
    for (scene_index, frame_counter, expected_hash) in [
        (
            3,
            128,
            "62963d21aba81ca626213032883eb5c676e33403473cd6371d5df72f352a8669",
        ),
        (
            7,
            94,
            "3ff9aac822a49c1e568358cb107784c9396e6fee8e719d7b614fba0c3302870f",
        ),
        (
            9,
            192,
            "bf686eed7abce51c6e5df4569ea2fa6642cc6b182a0543f4811479098fcac031",
        ),
        (
            13,
            128,
            "37db34b69f884eebf449c2385c756c7ea10f3c42d17cc79fdb9ca21361829bb1",
        ),
        (
            17,
            96,
            "6b221d34984119e40e758a9194de49468ae78696944ae652e46011782441114d",
        ),
        (
            19,
            152,
            "e7c0c06b8783a257afd42cf6fd5cbe0022895ede8737f2852c7e33a744999273",
        ),
        (
            26,
            128,
            "9796e7f10069f44626d7b1a1cede9df16d8f1aca5b86628528e3e9faa6613c8e",
        ),
    ] {
        for _ in 0..3_000 {
            let at_checkpoint = runtime_shell.intro_screen.as_ref().is_some_and(|intro| {
                intro.jumptable_index == scene_index
                    && intro.scene_frame_counter == frame_counter
            });
            if at_checkpoint {
                break;
            }
            tick_visible_intro_screen(&mut runtime_shell)
                .expect("advance CrystalIntro to ROM checkpoint");
        }
        let current = runtime_shell
            .intro_screen
            .as_ref()
            .expect("intro remains active at ROM checkpoint");
        assert_eq!(current.jumptable_index, scene_index);
        assert_eq!(current.scene_frame_counter, frame_counter);

        let mut render_intro = runtime_shell
            .intro_display_screen
            .clone()
            .expect("intro checkpoint has a VBlank-latched display state");
        let current = runtime_shell
            .intro_screen
            .as_ref()
            .expect("intro remains active while composing its LCD");
        render_intro.ly_overrides = current.ly_overrides.clone();
        render_intro.lcdc_pointer = current.lcdc_pointer;
        apply_visible_intro_background_binding(
            &mut render_intro,
            runtime_shell.runtime.title_presentation_program(),
        )
        .expect("bind intro checkpoint background");
        let mut rendered_art = RenderedTilesetArt::default();
        let mut images = Assets::<Image>::default();
        let frame = intro_scene_frame_for_art_with_bundle(
            &mut rendered_art,
            &asset_root,
            &sprite_bundle,
            &render_intro,
            &mut images,
        );
        assert!(
            frame.is_some(),
            "render IntroScene{} terminal LCD: {:?}",
            scene_index + 1,
            rendered_art.intro_scene_errors
        );
        let frame = frame.expect("checked intro checkpoint frame");
        let image = images.get(&frame.handle).expect("intro checkpoint image");
        let rgb5 = image
            .data
            .chunks_exact(4)
            .flat_map(|pixel| [pixel[0] >> 3, pixel[1] >> 3, pixel[2] >> 3])
            .collect::<Vec<_>>();
        assert_eq!(
            format!("{:x}", Sha256::digest(rgb5)),
            expected_hash,
            "IntroScene{} terminal pixels must match intro_trace.py's unmodified ROM",
            scene_index + 1
        );
    }
}

#[test]
fn intro_scene_renderer_composites_real_oam_sprites_from_bundle() {
    let program = core_modular_title_shell_for_test()
        .runtime
        .data()
        .runtime_title_screen
        .program
        .clone();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let sprite_bundle_text = crate::read_runtime_asset_to_string(
        &asset_root
            .runtime_assets()
            .join("data/sprite_anim_bundle.json"),
    )
    .expect("read runtime sprite animation bundle");
    let sprite_bundle = load_intro_sprite_anim_bundle(&sprite_bundle_text)
        .expect("load runtime sprite animation bundle");
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    let mut background_only = VisibleIntroScreen::new();
    background_only.jumptable_index = 6;
    let background_frame = intro_scene_frame_for_art(
        &mut rendered_art,
        &asset_root,
        &background_only,
        &mut images,
    )
    .expect("render background-only intro scene");
    let background_data = images
        .get(&background_frame.handle)
        .expect("background intro image")
        .data
        .clone();

    let mut with_sprite = background_only.clone();
    spawn_visible_intro_sprite_program_group(&mut with_sprite, &sprite_bundle, &program)
        .expect("spawn source-bound Suicune intro sprite");
    with_sprite.sprites[0].x = 10 * 8;
    with_sprite.sprites[0].y = 9 * 8;
    apply_visible_intro_sprite_pipeline(&mut with_sprite, &sprite_bundle, &program)
        .expect("advance Suicune through pack-owned frameset");
    let sprite_frame =
        intro_scene_frame_for_art(&mut rendered_art, &asset_root, &with_sprite, &mut images)
            .expect("render intro scene with sprite OAM");
    let sprite_data = &images
        .get(&sprite_frame.handle)
        .expect("sprite intro image")
        .data;

    assert_eq!(
        background_frame.handle, sprite_frame.handle,
        "intro animation should update the retained LCD texture instead of allocating a new image"
    );
    assert!(
        background_data
            .iter()
            .zip(sprite_data.iter())
            .any(|(left, right)| left != right),
        "intro sprite OAM should visibly alter the rendered frame"
    );
}

#[test]
fn intro_oam_tiles_clip_at_the_lcd_edge_instead_of_wrapping() {
    let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]));
    let mut target = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE * 4];
    let background_priority = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE];
    let palette = [[0, 0, 0], [10, 20, 30], [40, 50, 60], [70, 80, 90]];

    blit_intro_sprite_source_tile(
        &source,
        8,
        0,
        &palette,
        false,
        false,
        false,
        -8,
        0,
        0,
        &background_priority,
        &mut target,
    );

    let left_edge = 0;
    let wrapped_right_edge = ((32 * SOURCE_TILE_SIZE - 1) * 4) as usize;
    assert_eq!(&target[left_edge..left_edge + 4], &[0, 0, 0, 0]);
    assert_eq!(
        &target[wrapped_right_edge..wrapped_right_edge + 4],
        &[0, 0, 0, 0],
        "an offscreen OAM tile must not reappear at the opposite BG edge"
    );
}

#[test]
fn intro_oam_priority_hides_obj_pixels_behind_nonzero_bg_pixels() {
    let source = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]));
    let palette = [[0, 0, 0], [10, 20, 30], [40, 50, 60], [70, 80, 90]];
    let mut target = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE * 4];
    let mut background_priority = vec![0_u8; 32 * SOURCE_TILE_SIZE * 32 * SOURCE_TILE_SIZE];
    background_priority[0] = 1;

    blit_intro_sprite_source_tile(
        &source,
        8,
        0,
        &palette,
        true,
        false,
        false,
        0,
        0,
        0x80,
        &background_priority,
        &mut target,
    );
    assert_eq!(&target[..4], &[0, 0, 0, 0]);

    background_priority[0] = 0;
    blit_intro_sprite_source_tile(
        &source,
        8,
        0,
        &palette,
        true,
        false,
        false,
        0,
        0,
        0x80,
        &background_priority,
        &mut target,
    );
    assert_eq!(&target[..4], &[70, 80, 90, 255]);
}

#[test]
fn intro_scene_renderer_applies_asm_palette_effects() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("repository root");
    let asset_root = AssetRoot::new(repo_root);
    let mut rendered_art = RenderedTilesetArt::default();
    let mut images = Assets::<Image>::default();

    let base = VisibleIntroScreen::new();
    let base_frame = intro_scene_frame_for_art(&mut rendered_art, &asset_root, &base, &mut images)
        .expect("render base Unown intro frame");
    let base_data = images
        .get(&base_frame.handle)
        .expect("base intro image")
        .data
        .clone();
    assert_eq!(base_data.len(), 160 * 144 * 4);
    assert!(
        base_data
            .chunks_exact(4)
            .all(|pixel| pixel == [0, 0, 0, 255]),
        "IntroScene1 must begin on the source black LCD before the Unown fade"
    );

    let mut faded = base.clone();
    faded.jumptable_index = 1;
    faded.palette_effect = VisibleIntroPaletteEffect::UnownFade {
        palette_idx: 0,
        colors: [[248, 248, 248], [0, 120, 248], [0, 0, 248]],
    };
    let faded_frame =
        intro_scene_frame_for_art(&mut rendered_art, &asset_root, &faded, &mut images)
            .expect("render faded Unown intro frame");
    let faded_data = images
        .get(&faded_frame.handle)
        .expect("faded intro image")
        .data
        .clone();
    assert!(
        base_data
            .iter()
            .zip(faded_data.iter())
            .any(|(left, right)| left != right),
        "Unown palette fade should visibly alter the rendered frame"
    );

    let intro_root = asset_root.runtime_assets().join("gfx/intro");
    let suicune_palettes =
        load_intro_palette_bank(&intro_root, "suicune").expect("load Suicune intro palettes");
    let suicune_base = suicune_palettes[0];
    let faded_suicune =
        visible_intro_effective_palette(&faded, &intro_root, "suicune", 0, &suicune_base)
            .expect("resolve faded Suicune palette");
    assert_eq!(
        faded_suicune, suicune_base,
        "unownFade only overrides the Unown palette bank; Suicune changes happen in appearUnown"
    );

    let mut appear = base.clone();
    appear.palette_effect = VisibleIntroPaletteEffect::AppearUnown {
        palette_resource: "gfx/intro/unown_1.pal".to_string(),
        revealed: 3,
    };
    let unrevealed_suicune =
        visible_intro_effective_palette(&appear, &intro_root, "suicune", 1, &suicune_palettes[1])
            .expect("resolve unrevealed Suicune palette");
    let revealed_suicune =
        visible_intro_effective_palette(&appear, &intro_root, "suicune", 3, &suicune_base)
            .expect("resolve revealed Suicune palette");
    let reveal_palette = load_intro_palette_bank(&intro_root, "unown_1")
        .expect("load appearUnown source palette")[0];
    assert_eq!(
        unrevealed_suicune, suicune_palettes[1],
        "appearUnown must not rewrite palettes before the first ASM target index"
    );
    assert_eq!(
        revealed_suicune, reveal_palette,
        "appearUnown writes the same hardware palette to Suicune-backed attrs"
    );

    let mut cleared = base.clone();
    cleared.palette_effect = VisibleIntroPaletteEffect::ClearBg {
        color: [248, 248, 248],
    };
    let cleared_frame =
        intro_scene_frame_for_art(&mut rendered_art, &asset_root, &cleared, &mut images)
            .expect("render cleared intro frame");
    let cleared_data = &images
        .get(&cleared_frame.handle)
        .expect("cleared intro image")
        .data;
    assert!(
        cleared_data
            .chunks_exact(4)
            .filter(|pixel| pixel[3] != 0)
            .all(|pixel| pixel[0] == 248 && pixel[1] == 248 && pixel[2] == 248),
        "ClearBGPalettes should render every nontransparent pixel with its exported fill color"
    );
}

#[test]
fn intro_unown_fade_does_not_recolor_obj_pulses() {
    let base_palette = [[8, 16, 24], [32, 40, 48], [56, 64, 72], [80, 88, 96]];
    let mut rendered_art = RenderedTilesetArt::default();
    rendered_art
        .intro_palette_cache
        .insert("unowns:false".to_string(), vec![base_palette]);
    let mut intro = VisibleIntroScreen::new();
    intro.palette_effect = VisibleIntroPaletteEffect::UnownFade {
        palette_idx: 0,
        colors: [[248, 248, 248], [0, 120, 248], [0, 0, 248]],
    };

    let pulse_palette = visible_intro_effective_palette_cached(
        &intro,
        &rendered_art,
        "unowns",
        0,
        &base_palette,
        true,
    )
    .expect("resolve pulse OBJ palette");

    assert_eq!(
        pulse_palette, base_palette,
        "the BG-only Unown fade must not recolor the pulse OBJ palette"
    );
}

#[test]
fn intro_sprite_bundle_rejects_missing_oam_fields_instead_of_defaulting_them() {
    let malformed = serde_json::json!({
        "oam_sets": {
            "SPRITE_ANIM_OAMSET_TEST": {
                "name": "SPRITE_ANIM_OAMSET_TEST",
                "tile_offset": 0,
                "pieces": [{ "y": 0, "tile": 0, "attributes": 0 }]
            }
        },
        "framesets": {
            "Frameset_Test": {
                "name": "Frameset_Test",
                "steps": [{
                    "oam_set": "SPRITE_ANIM_OAMSET_TEST",
                    "duration": 1,
                    "attr_flags": 0,
                    "command": "frame"
                }]
            }
        },
        "objects": { "SPRITE_ANIM_OBJ_TEST": {} }
    });

    let error = load_intro_sprite_anim_bundle(&malformed.to_string())
        .expect_err("an OAM piece without X must be rejected");
    assert!(
        error
            .to_string()
            .contains("parse packed sprite animation bundle")
    );
}

#[test]
fn intro_tile_resolution_rejects_missing_source_tiles_instead_of_wrapping() {
    let error = resolve_intro_tile_index(7, 0, IntroTileIndexMode::Offset, 4)
        .expect_err("tile ids outside the source sheet must not wrap");
    assert!(error.to_string().contains("outside 4 source tiles"));
}

#[test]
fn credits_parser_reads_exported_source_script_and_strings() {
    let runtime_shell = core_modular_title_shell_for_test();
    let program = load_visible_credits_program(&runtime_shell).expect("load exported credits data");
    let constants = program.constant_indices;
    let strings = program.strings;
    let string_tiles = program.string_tiles;
    let script = program.ops;

    let staff_index = constants.get("STAFF").copied().expect("STAFF constant");
    assert_eq!(
        strings.get(staff_index).map(String::as_str),
        Some("      #MON\n  CRYSTAL VERSION\n       STAFF")
    );
    let staff_tiles = string_tiles.get(staff_index).expect("STAFF tile rows");
    assert_eq!(staff_tiles.len(), 3);
    assert_eq!(&staff_tiles[0][6..10], &[0x8f, 0x8e, 0x8a, 0xea]);
    assert_eq!(
        staff_tiles[0].len(),
        "      POKéMON".chars().count(),
        "PlaceString must expand # to POKé while preserving ASM spacing"
    );
    assert!(matches!(script.first(), Some(VisibleCreditsOp::Clear)));
    assert!(
        script
            .iter()
            .any(|op| matches!(op, VisibleCreditsOp::Music))
    );
    assert!(
        script
            .iter()
            .any(|op| matches!(op, VisibleCreditsOp::TheEnd))
    );
    assert!(matches!(script.last(), Some(VisibleCreditsOp::End)));
}

#[test]
fn credits_screen_opens_from_exported_program_and_reaches_music_opcode_by_tick() {
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
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize title shell");

    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    {
        let credits = runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen active");
        assert!(credits.lines.is_empty());
        assert_eq!(credits.timer, 0);
        assert_eq!(visible_credits_step_index(credits), 0);
        assert_eq!(runtime_shell.active_music.as_deref(), None);
    }

    tick_visible_credits_screen(&mut runtime_shell);
    {
        let credits = runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen active after first tick");
        assert_eq!(
            credits.lines.first().map(|line| line.text.as_str()),
            Some("      #MON\n  CRYSTAL VERSION\n       STAFF")
        );
        assert_eq!(
            credits.lines.first().map(|line| line.tiles.len()),
            Some(3),
            "credits rendering should carry parsed ASM tile rows"
        );
        assert!(
            credits
                .lines
                .first()
                .and_then(|line| line.tiles.first())
                .is_some_and(|tiles| tiles.windows(4).any(|run| run == [0x8f, 0x8e, 0x8a, 0xea])),
            "credits rendering should preserve PlaceString's expanded POKé tiles"
        );
        assert_eq!(credits.timer, 8);
        assert_eq!(credits.bg_map_mode, 1);
        assert_eq!(credits.bg_map_third, 1);
        assert!(
            credits.displayed_text_rows.is_empty(),
            "the first Credits VBlank transfers rows 0-5, before STAFF at rows 8-10"
        );
        assert_eq!(visible_credits_step_index(credits), 1);
        assert_eq!(runtime_shell.active_music.as_deref(), None);
    }

    for _ in 0..7 {
        tick_visible_credits_screen(&mut runtime_shell);
    }
    {
        let credits = runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen active after first jumptable pass");
        assert_eq!(
            credits.timer, 8,
            "wait counters should only decrement on the parse step, not every frame"
        );
        assert_eq!(
            credits
                .displayed_text_rows
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![8, 10, 12],
            "the middle and bottom transfers must commit STAFF with <NEXT>'s blank rows"
        );
        assert_eq!(credits.ly_override, 0xfe);
        assert_eq!(runtime_shell.active_music.as_deref(), None);
    }

    for _ in 0..140 {
        if runtime_shell
            .credits_screen
            .as_ref()
            .is_some_and(|credits| credits.music_start_delay_frames == 1)
        {
            break;
        }
        tick_visible_credits_screen(&mut runtime_shell);
    }
    assert_eq!(runtime_shell.active_music.as_deref(), Some("MUSIC_NONE"));
    assert_eq!(
        runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen during music delay")
            .music_start_delay_frames,
        1,
        "Credits .music must retain the source DelayFrame between MUSIC_NONE and MUSIC_CREDITS"
    );
    tick_visible_credits_screen(&mut runtime_shell);

    let credits = runtime_shell
        .credits_screen
        .as_ref()
        .expect("credits screen still active");
    assert_eq!(credits.timer, 10);
    assert_eq!(visible_credits_step_index(credits), 1);
    assert_eq!(runtime_shell.active_music.as_deref(), Some("MUSIC_CREDITS"));
    assert!(
        runtime_shell.pending_full_audio_reset,
        "the source MUSIC_NONE reset must survive the following credits-music request"
    );
    assert!(
        runtime_shell
            .pending_audio
            .iter()
            .any(|command| command.audio_id == "MUSIC_CREDITS"),
        "credits music should be queued from the parsed CREDITS_MUSIC opcode"
    );
}

#[test]
fn credits_bg_transfer_preserves_next_spacing_across_hardware_thirds() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    {
        let credits = runtime_shell.credits_screen.as_mut().expect("credits screen");
        credits.lines = vec![VisibleCreditsLine {
            token: "US_VERSION_STAFF".to_string(),
            text: "three rows".to_string(),
            tiles: vec![vec![1], vec![2], vec![3]],
            line_index: 2,
        }];
        credits.bg_map_mode = 1;
        credits.bg_map_third = 0;
    }

    commit_visible_credits_bg_map_third(&mut runtime_shell);
    assert!(
        runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen")
            .displayed_text_rows
            .is_empty()
    );
    commit_visible_credits_bg_map_third(&mut runtime_shell);
    assert_eq!(
        runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen")
            .displayed_text_rows
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![10]
    );
    commit_visible_credits_bg_map_third(&mut runtime_shell);
    assert_eq!(
        runtime_shell
            .credits_screen
            .as_ref()
            .expect("credits screen")
            .displayed_text_rows
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![10, 12, 14]
    );
}

#[test]
fn credits_first_top_third_lcd_matches_the_asm_rom_oracle() {
    use sha2::{Digest, Sha256};

    let mut runtime_shell = core_modular_title_shell_for_test();
    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    tick_visible_credits_screen(&mut runtime_shell);
    let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
    assert_eq!(credits.bg_map_third, 1);
    assert!(credits.displayed_text_rows.is_empty());

    let mut images = Assets::<Image>::default();
    let frame = render_visible_credits_frame(&runtime_shell.asset_root, credits, &mut images)
        .expect("render first Credits top-third transfer");
    let image = images.get(&frame.handle).expect("Credits image");
    let rgb5 = image
        .data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0] >> 3, pixel[1] >> 3, pixel[2] >> 3])
        .collect::<Vec<_>>();
    assert_eq!(
        format!("{:x}", Sha256::digest(rgb5)),
        "2c92d82fc2af02cbdcb74a5b2dab786188e7300a247c58894027dc71549f252e",
        "normalized RGB5 pixels must match credits_trace.py's first_bg_third_1 checkpoint"
    );

    tick_visible_credits_screen(&mut runtime_shell);
    let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
    assert_eq!(credits.bg_map_third, 2);
    assert_eq!(
        credits
            .displayed_text_rows
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![8, 10]
    );
    let frame = render_visible_credits_frame(&runtime_shell.asset_root, credits, &mut images)
        .expect("render first Credits middle-third transfer");
    let image = images.get(&frame.handle).expect("Credits image");
    let rgb5 = image
        .data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0] >> 3, pixel[1] >> 3, pixel[2] >> 3])
        .collect::<Vec<_>>();
    assert_eq!(
        format!("{:x}", Sha256::digest(rgb5)),
        "862bde42f6311313b3df66a7756be0a37b4ea2973255ae1e25f62dd1af6ae215",
        "the middle-third pixels must match the ROM's following LCD scan"
    );

    tick_visible_credits_screen(&mut runtime_shell);
    let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
    assert_eq!(credits.bg_map_third, 0);
    assert_eq!(
        credits
            .displayed_text_rows
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![8, 10, 12]
    );
    let frame = render_visible_credits_frame(&runtime_shell.asset_root, credits, &mut images)
        .expect("render completed first Credits transfer");
    let image = images.get(&frame.handle).expect("Credits image");
    let rgb5 = image
        .data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0] >> 3, pixel[1] >> 3, pixel[2] >> 3])
        .collect::<Vec<_>>();
    assert_eq!(
        format!("{:x}", Sha256::digest(rgb5)),
        "54d33685f05fa9338bfce703844edc0c2d058a2265b8b44eeb7d66542932c6a4",
        "the completed three-third transfer must match the ROM's following LCD scan"
    );
}

#[test]
fn credits_accelerated_exit_lcd_matches_the_asm_rom_oracle() {
    use sha2::{Digest, Sha256};

    let mut runtime_shell = core_modular_title_shell_for_test();
    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    for _ in 0..2_000 {
        press_visible_credits_b_button(&mut runtime_shell).expect("press Credits B");
        tick_visible_credits_screen(&mut runtime_shell);
        if runtime_shell
            .credits_screen
            .as_ref()
            .is_some_and(|credits| credits.awaiting_exit)
        {
            break;
        }
    }
    let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
    assert!(credits.awaiting_exit, "accelerated Credits did not terminate");
    assert_eq!(credits.consumed_bytes, 351);
    assert_eq!(credits.scene_index, 3);
    assert_eq!(credits.border_mon_index, 3);
    assert_eq!(credits.border_frame_counter, Some(2));
    assert_eq!(credits.bg_map_mode, 0);
    assert_eq!(credits.bg_map_third, 0);
    assert_eq!(credits.ly_override, 126);
    assert!(credits.displayed_show_the_end);

    let mut images = Assets::<Image>::default();
    let frame = render_visible_credits_frame(&runtime_shell.asset_root, credits, &mut images)
        .expect("render accelerated Credits exit");
    let image = images.get(&frame.handle).expect("Credits image");
    let rgb5 = image
        .data
        .chunks_exact(4)
        .flat_map(|pixel| [pixel[0] >> 3, pixel[1] >> 3, pixel[2] >> 3])
        .collect::<Vec<_>>();
    assert_eq!(
        format!("{:x}", Sha256::digest(rgb5)),
        "f4cafc5f7c7d019df0efcb3d8d483d8d602f21e0b729c6daa47fc76c2f967a94",
        "accelerated exit pixels must match credits_trace.py's awaiting_exit checkpoint"
    );
}

#[test]
fn credits_end_starts_source_post_credits_fade_and_return_does_not_stop_it() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    assert_eq!(runtime_shell.h_in_menu, 0);
    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    assert_eq!(
        runtime_shell.h_in_menu, 1,
        "Credits must reproduce the source hInMenu write"
    );
    runtime_shell.active_music = Some("MUSIC_CREDITS".to_string());
    {
        let credits = runtime_shell.credits_screen.as_mut().expect("credits screen");
        credits.program.ops = vec![VisibleCreditsOp::TheEnd, VisibleCreditsOp::End];
        credits.script_index = 0;
        credits.jumptable_index &= 0xf0;
    }

    tick_visible_credits_screen(&mut runtime_shell);
    let fade = runtime_shell.music_fade.as_ref().expect("post-credits music fade");
    assert_eq!(fade.target_music, "MUSIC_POST_CREDITS");
    assert_eq!(fade.rate, 32);
    assert!(!fade.fading_in);
    assert_eq!(runtime_shell.active_music.as_deref(), Some("MUSIC_CREDITS"));
    assert!(runtime_shell.credits_screen.as_ref().expect("credits screen").awaiting_exit);

    let (frame, step) = {
        let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
        (credits.frame, visible_credits_step_index(credits))
    };
    tick_visible_credits_screen(&mut runtime_shell);
    let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
    assert_eq!(credits.frame, frame + 1);
    assert_eq!(visible_credits_step_index(credits), (step + 1) & 0x0f);
    assert!(credits.show_the_end);

    press_visible_credits_a_button(&mut runtime_shell).expect("return from Credits");
    assert_eq!(
        runtime_shell
            .credits_screen
            .as_ref()
            .expect("Credits remains active for ClearBGPalettes")
            .exit_clear_frames_remaining,
        Some(4)
    );
    let mut images = Assets::<Image>::default();
    let clear_frame = render_visible_credits_frame(
        &runtime_shell.asset_root,
        runtime_shell
            .credits_screen
            .as_ref()
            .expect("Credits clear frame"),
        &mut images,
    )
    .expect("render Credits ClearBGPalettes frame");
    let clear_image = images
        .get(&clear_frame.handle)
        .expect("Credits ClearBGPalettes image");
    assert!(
        clear_image
            .data
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]),
        "ClearBGPalettes must make every Credits pixel white while its four-frame wait runs"
    );
    for remaining in [3, 2, 1] {
        tick_visible_credits_screen(&mut runtime_shell);
        assert_eq!(
            runtime_shell
                .credits_screen
                .as_ref()
                .expect("Credits remains active during ClearBGPalettes")
                .exit_clear_frames_remaining,
            Some(remaining)
        );
    }
    tick_visible_credits_screen(&mut runtime_shell);
    assert!(runtime_shell.credits_screen.is_none());
    let fade = runtime_shell.music_fade.as_ref().expect("fade survives Credits return");
    assert_eq!(fade.target_music, "MUSIC_POST_CREDITS");
    assert_eq!(runtime_shell.active_music.as_deref(), Some("MUSIC_CREDITS"));
    assert_eq!(
        runtime_shell.h_in_menu, 1,
        "the source Credits exit does not restore hInMenu"
    );
}

#[test]
fn credits_end_clears_wram_but_retains_the_end_pixels_until_exit() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    let the_end_index = runtime_shell
        .credits_screen
        .as_ref()
        .expect("credits screen")
        .program
        .ops
        .iter()
        .position(|op| matches!(op, VisibleCreditsOp::TheEnd))
        .expect("source CREDITS_THEEND");
    {
        let credits = runtime_shell.credits_screen.as_mut().expect("credits screen");
        assert!(matches!(
            credits.program.ops.get(the_end_index + 1),
            Some(VisibleCreditsOp::Wait(20))
        ));
        assert!(matches!(
            credits.program.ops.get(the_end_index + 2),
            Some(VisibleCreditsOp::End)
        ));
        credits.script_index = the_end_index;
        credits.jumptable_index &= 0xf0;
    }

    tick_visible_credits_screen(&mut runtime_shell);
    {
        let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
        assert!(credits.show_the_end);
        assert!(!credits.displayed_show_the_end);
        assert_eq!(credits.bg_map_third, 1);
        assert_eq!(credits.timer, 20);
        assert!(visible_credits_screen_lines(credits).iter().any(|line| line == "THE END"));
    }

    tick_visible_credits_screen(&mut runtime_shell);
    {
        let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
        assert!(credits.displayed_show_the_end);
        assert_eq!(credits.bg_map_third, 2);
    }

    {
        let credits = runtime_shell.credits_screen.as_mut().expect("credits screen");
        credits.timer = 0;
        credits.jumptable_index &= 0xf0;
    }
    tick_visible_credits_screen(&mut runtime_shell);
    let credits = runtime_shell.credits_screen.as_ref().expect("credits screen");
    assert!(credits.awaiting_exit);
    assert!(!credits.show_the_end);
    assert!(credits.displayed_show_the_end);
    assert!(
        visible_credits_screen_lines(credits)
            .iter()
            .all(|line| line != "THE END"),
        "ParseCredits clears rows 5-16 before consuming CREDITS_END"
    );
    let mut images = Assets::<Image>::default();
    let frame = render_visible_credits_frame(&runtime_shell.asset_root, credits, &mut images)
        .expect("render retained post-THE-END exit frame");
    let image = images.get(&frame.handle).expect("post-THE-END image");
    let mut colors = BTreeSet::new();
    for y in 9 * SOURCE_TILE_SIZE..11 * SOURCE_TILE_SIZE {
        for x in 6 * SOURCE_TILE_SIZE..14 * SOURCE_TILE_SIZE {
            let offset = (y * CREDITS_SCREEN_WIDTH + x) * 4;
            colors.insert(image.data[offset..offset + 4].to_vec());
        }
    }
    assert!(
        colors.len() > 1,
        "CREDITS_END disables BG-map transfer after clearing WRAM, so THE END remains in VRAM"
    );
}

#[test]
fn hall_of_fame_credits_restore_game_timer_counting_on_return() {
    let mut runtime_shell = core_modular_title_shell_for_test();
    runtime_shell.intro_screen = None;
    runtime_shell.title_menu = None;
    runtime_shell
        .shell
        .session_mut()
        .state_mut()
        .set_game_timer_counting(false);
    open_visible_credits_screen(&mut runtime_shell, true).expect("open Hall of Fame credits");
    runtime_shell
        .credits_screen
        .as_mut()
        .expect("credits screen")
        .resume_game_timer_on_exit = true;

    close_visible_credits_screen(&mut runtime_shell, "test-return")
        .expect("return from Hall of Fame credits");

    assert!(runtime_shell.shell.session().state().game_timer_counting);
    assert!(!runtime_shell.shell.session().state().game_logic_paused);
}

#[test]
fn credits_frame_renders_real_assets_and_special_font_tiles() {
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
    let spawn_identifier = runtime
        .title_new_game_spawn_identifier()
        .expect("title new-game spawn");
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root.clone(),
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: None,
        },
        BevyShellConfig {
            smoke_player_name: Some("AB".to_string()),
            ..Default::default()
        },
    )
    .expect("initialize title shell");

    open_visible_credits_screen(&mut runtime_shell, true).expect("open credits");
    tick_visible_credits_screen(&mut runtime_shell);
    let credits = runtime_shell
        .credits_screen
        .as_ref()
        .expect("credits screen active");
    let font = load_visible_credits_font_tiles(&asset_root).expect("load credits font");
    assert!(
        [0x8f, 0x8e, 0x8a, 0xea]
            .into_iter()
            .all(|tile| font.levels.contains_key(&tile)),
        "credits font must include PlacePOKEText's expanded POKé glyph tiles"
    );

    let mut images = Assets::<Image>::default();
    let frame =
        render_visible_credits_frame(&asset_root, credits, &mut images).expect("render credits");
    let image = images.get(&frame.handle).expect("credits image handle");
    assert_eq!(
        image.texture_descriptor.size.width,
        CREDITS_SCREEN_WIDTH as u32
    );
    assert_eq!(
        image.texture_descriptor.size.height,
        CREDITS_SCREEN_HEIGHT as u32
    );
    let unique_colors = image
        .data
        .chunks_exact(4)
        .filter(|rgba| rgba[3] != 0)
        .map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unique_colors,
        BTreeSet::from([[90, 115, 255], [255, 41, 41]]),
        "the first top-third frame has only the Pichu blue and red palette colors"
    );

    let mut staged_credits = credits.clone();
    staged_credits.border_frame_top = Some(VisibleCreditsBorderFrame {
        mon_index: 0,
        frame_index: 0,
    });
    staged_credits.border_frame_bottom = Some(VisibleCreditsBorderFrame {
        mon_index: 0,
        frame_index: 0,
    });
    let staged_frame = render_visible_credits_frame(&asset_root, &staged_credits, &mut images)
        .expect("render staged credits mon frame");
    let staged_image = images
        .get(&staged_frame.handle)
        .expect("staged credits image handle");
    let staged_unique_colors = staged_image
        .data
        .chunks_exact(4)
        .filter(|rgba| rgba[3] != 0)
        .map(|rgba| [rgba[0], rgba[1], rgba[2]])
        .collect::<BTreeSet<_>>();
    assert!(
        staged_unique_colors.len() >= 4,
        "staged credits frame should include tinted mon strip colors from real assets"
    );
}
