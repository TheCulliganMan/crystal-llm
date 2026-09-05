#[test]
fn elms_lab_heal_script_retains_its_authored_machine_animation() {
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
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name: "ElmsLab".to_string(),
            tile_x: 4,
            tile_y: 6,
        },
        BevyShellConfig::default(),
    )
    .expect("initialize Elm's Lab shell");
    runtime_shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            5,
            None,
            None,
            "CHRIS",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .expect("add starter");
    if runtime_shell.shell.script_events_snapshot().script_ended.is_some() {
        runtime_shell
            .shell
            .take_script_end_state()
            .expect("clear map initialization script end");
    }
    arm_visible_active_script_cursor(&mut runtime_shell, "ElmsLabHealingMachine_HealParty", 0);

    continue_visible_script_after_prompt(&mut runtime_shell).expect("start Elm heal script");

    let animation = runtime_shell
        .visible_heal_machine
        .as_ref()
        .expect("Elm heal script must retain HealMachineAnim for rendering");
    assert_eq!(animation.kind, 1, "Elm's Lab must use HEALMACHINE_ELMS_LAB");
    assert_eq!(animation.party_count, 1);
    assert_eq!(animation.frame, 0);
    assert_eq!(
        runtime_shell
            .active_script_cursor
            .as_ref()
            .map(|cursor| (cursor.source_script.as_str(), cursor.next_command_index)),
        Some(("ElmsLabHealingMachine_HealParty", 5)),
        "the script must stop at HealMachineAnim until its retained animation completes"
    );

    let mut app = integrated_shell_test_app(runtime_shell);
    #[cfg(feature = "fullscreen-scaling")]
    {
        app.world_mut().spawn((Window { resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0), ..default() }, bevy::window::PrimaryWindow));
        app.add_systems(Startup, setup_fullscreen_scene).add_systems(PostUpdate,
            (sync_fullscreen_scaling, sync_fullscreen_scene_layout, sync_fullscreen_world_layout).chain());
    }
    app.update();
    app.update();
    let (ball_handle, lamp_handle) = {
        let rendered = app.world().resource::<RenderedTilesetArt>();
        (
            rendered
                .heal_machine_ball_cache
                .as_ref()
                .expect("heal-machine ball art must load")[0]
                .handle
                .clone(),
            rendered
                .heal_machine_lamp_cache
                .as_ref()
                .expect("heal-machine lamp art must load")[0]
                .handle
                .clone(),
        )
    };
    let world = app.world_mut();
    let mut sprites = world.query_filtered::<&Handle<Image>, With<FieldCommandMarker>>();
    let rendered_heal_sprites = sprites
        .iter(world)
        .filter(|handle| **handle == ball_handle || **handle == lamp_handle)
        .count();
    assert_eq!(
        rendered_heal_sprites, 3,
        "Elm's one-Pokemon heal must render one ball and both machine lamps"
    );
    let mut positioned = world.query_filtered::<(&Handle<Image>, &Transform), With<FieldCommandMarker>>();
    for (handle, transform) in positioned.iter(world) {
        let position = transform.translation.truncate();
        if *handle == ball_handle {
            let (x, y) = battle_hud_tile_origin(40.0 / 8.0, 54.0 / 8.0);
            assert_eq!(position, Vec2::new(x, y), "dbsprite x/y and pixel offsets must match the source macro");
        } else if *handle == lamp_handle {
            let expected = [42.0, 46.0].map(|x| { let (x, y) = battle_hud_tile_origin(x / 8.0, 48.0 / 8.0); Vec2::new(x, y) });
            assert!(expected.contains(&position), "lamp offsets must match the source macro");
        }
    }
    #[cfg(feature = "fullscreen-scaling")]
    {
        let world_root = world.query_filtered::<Entity, With<FullscreenWorldRoot>>().single(world);
        let mut sprites = world.query_filtered::<(&Handle<Image>, Option<&Parent>), With<FieldCommandMarker>>();
        for (handle, parent) in sprites.iter(world) {
            if *handle == ball_handle || *handle == lamp_handle {
                assert_eq!(parent.map(Parent::get), Some(world_root),
                    "healing balls and lamps must follow the machine's world scale and position");
            }
        }
    }

}
