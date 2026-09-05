fn webmcp_test_app() -> App {
    WEBMCP_BRIDGE.with_borrow_mut(|bridge| *bridge = WebMcpBridge::default());
    let mut shell = core_modular_title_shell_for_test();
    shell.intro_screen = None;
    shell.title_menu = None;
    open_visible_name_choice(&mut shell).unwrap();
    for _ in 0..40 {
        tick_visible_player_name_choice(&mut shell).unwrap();
    }
    mark_runtime_snapshot_dirty(&mut shell);
    let mut app = integrated_shell_test_app(shell);
    // MinimalPlugins omits the production InputPlugin edge reset.
    app.add_systems(First, |mut keys: ResMut<ButtonInput<KeyCode>>| keys.clear());
    app.add_systems(Update, apply_webmcp_input.before(apply_keyboard_input));
    app.add_systems(PostUpdate, finish_webmcp_request);
    app.update();
    app
}

fn finish_webmcp_test_action(app: &mut App, id: u32) -> serde_json::Value {
    for _ in 0..100 {
        app.update();
        if let Some(result) = crystal_webmcp_poll(id) {
            return serde_json::from_str(&result).unwrap();
        }
    }
    panic!("WebMCP action did not complete");
}

#[test]
fn webmcp_observes_real_name_choices_and_uses_normal_joypad_input() {
    let mut app = webmcp_test_app();
    let read = crystal_webmcp_request(r#"{"kind":"observe"}"#).unwrap();
    let before = finish_webmcp_test_action(&mut app, read);
    assert_eq!(before["status"]["screen"], "naming");
    assert_eq!(before["observe"]["menus"][0]["selected"], 0);
    let action = crystal_webmcp_request(r#"{"kind":"press","button":"down","frames":1}"#).unwrap();
    let after = finish_webmcp_test_action(&mut app, action);
    assert_eq!(after["observe"]["menus"][0]["selected"], 1);
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ArrowDown)
    );
    assert!(after["frame"].as_u64().unwrap() > before["frame"].as_u64().unwrap());
}

#[test]
fn webmcp_cancel_releases_held_input_and_reports_partial_action() {
    let mut app = webmcp_test_app();
    let id = crystal_webmcp_request(r#"{"kind":"press","button":"down","frames":60}"#).unwrap();
    app.update();
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ArrowDown)
    );
    assert!(crystal_webmcp_request(r#"{"kind":"observe"}"#).is_err());
    crystal_webmcp_cancel(id);
    let result = finish_webmcp_test_action(&mut app, id);
    assert!(result["error"].as_str().unwrap().contains("canceled"));
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ArrowDown)
    );
    assert!(crystal_webmcp_request(r#"{"kind":"observe"}"#).is_ok());
}

#[test]
fn webmcp_rejects_non_game_actions_and_unbounded_input() {
    WEBMCP_BRIDGE.with_borrow_mut(|bridge| *bridge = WebMcpBridge::default());
    for input in [
        r#"{"kind":"wait"}"#,
        r#"{"kind":"press","button":"warp","frames":1}"#,
        r#"{"kind":"press","button":"a","frames":0}"#,
        r#"{"kind":"press","button":"a","frames":61}"#,
        r#"{"kind":"observe","save":"x"}"#,
    ] {
        assert!(crystal_webmcp_request(input).is_err(), "accepted {input}");
    }
}

#[test]
fn webmcp_multiplayer_uses_the_existing_challenge_key() {
    let mut app = webmcp_test_app();
    let id = crystal_webmcp_request(r#"{"kind":"multiplayer","interaction":"trade"}"#).unwrap();
    app.update();
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::KeyV)
    );
    let result = finish_webmcp_test_action(&mut app, id);
    assert!(result.get("error").is_none());
    assert!(
        !app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::KeyV)
    );
}

#[test]
fn webmcp_battle_entry_is_observable_before_the_action_menu_opens() {
    let asset_root = AssetRoot::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap(),
    );
    let runtime = workspace_desktop_runtime(&asset_root);
    let mut shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: 14,
            map_name: "Route36".into(),
            tile_x: 20,
            tile_y: 8,
        },
        BevyShellConfig::default(),
    )
    .unwrap();
    shell
        .shell
        .add_party_pokemon(
            "CYNDAQUIL",
            10,
            None,
            None,
            "WEBMCP_TEST",
            1,
            Dv::from_non_hp(10, 10, 10, 10),
        )
        .unwrap();
    settle_visible_shell_smoke_until_idle(&mut shell).unwrap();
    shell
        .shell
        .start_scripted_wild_battle("Route36", "WateredWeirdTreeScript", 12)
        .unwrap();
    prepare_visible_battle_entry(&mut shell).unwrap();
    let observation =
        webmcp_observation(&shell, None).expect("battle introduction remains readable");
    assert_eq!(observation["status"]["screen"], "battle");
    assert!(
        observation["observe"]["menus"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(observation["status"]["party"][0].get("dvs").is_none());
}

#[test]
fn webmcp_does_not_release_a_button_already_held_by_the_human() {
    let mut app = webmcp_test_app();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ArrowDown);
    let id = crystal_webmcp_request(r#"{"kind":"press","button":"down","frames":1}"#).unwrap();
    let result = finish_webmcp_test_action(&mut app, id);
    assert!(result.get("error").is_some());
    assert!(
        app.world()
            .resource::<ButtonInput<KeyCode>>()
            .pressed(KeyCode::ArrowDown)
    );
}
