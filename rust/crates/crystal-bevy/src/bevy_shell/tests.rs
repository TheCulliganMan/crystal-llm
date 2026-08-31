include!("tests/shell_basics.rs");
include!("tests/overworld_rendering.rs");
include!("tests/intro_title_rendering.rs");
include!("tests/title_flow.rs");
include!("tests/runtime_surfaces.rs");
include!("tests/new_game_flow.rs");
include!("tests/integrated_menus.rs");
include!("tests/audio_and_battle_ui.rs");
include!("tests/menu_and_input.rs");
include!("tests/story_progression.rs");
include!("tests/art_text_and_determinism.rs");
include!("tests/battle_render_regressions.rs");
include!("tests/shop_rendering.rs");
include!("tests/heal_machine_rendering.rs");
include!("tests/town_map_rendering.rs");

#[test]
fn script_earthquake_shakes_then_sleeps_for_both_low_six_bit_counters() {
    let mut earthquake = super::VisibleEarthquake::from_script(84, 20, 20);
    assert_eq!(earthquake.intensity, 2);
    assert_eq!(earthquake.frames_remaining, 40);
    assert_eq!(earthquake.shake_frames_remaining, 20);

    earthquake.advance(20);
    assert_eq!(earthquake.frames_remaining, 20);
    assert_eq!(earthquake.shake_frames_remaining, 0);

    earthquake.advance(20);
    assert_eq!(earthquake.frames_remaining, 0);

    let wrapped = super::VisibleEarthquake::from_script(0, 256, 256);
    assert_eq!(wrapped.frames_remaining, 512);
    assert_eq!(wrapped.shake_frames_remaining, 256);
}

#[test]
fn egg_hatch_wobble_uses_exact_asm_pairs_and_crack_boundaries() {
    assert_eq!(visible_egg_wobble_x(0), -2);
    assert_eq!(visible_egg_wobble_x(2), -2);
    assert_eq!(visible_egg_wobble_x(3), 2);
    assert_eq!(visible_egg_wobble_x(5), 2);
    assert_eq!(visible_egg_wobble_x(6), 0);
    assert_eq!(visible_egg_wobble_x(21), 0);
    assert_eq!(visible_egg_wobble_x(22), -2);
    assert_eq!(visible_egg_wobble_x(343), 0);
    assert_eq!(
        (0..344)
            .filter(|frame| visible_egg_crack_at(*frame))
            .collect::<Vec<_>>(),
        vec![50, 124, 222]
    );
}
