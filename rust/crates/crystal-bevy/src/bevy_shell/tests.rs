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
fn overworld_screen_shake_uses_the_vertical_source_counter_sequence() {
    let mut earthquake = super::VisibleEarthquake::from_script(84, 20, 20);

    assert_eq!(
        super::visible_earthquake_camera_offset(Some(earthquake)),
        (0.0, 8.0),
        "remaining counter 19 applies -2 to SCY, which projects to +8 in Bevy's upward Y axis"
    );
    earthquake.advance(1);
    assert_eq!(
        super::visible_earthquake_camera_offset(Some(earthquake)),
        (0.0, -8.0),
        "remaining counter 18 applies +2 to SCY"
    );
    earthquake.advance(18);
    assert_eq!(earthquake.shake_frames_remaining, 1);
    assert_eq!(
        super::visible_earthquake_camera_offset(Some(earthquake)),
        (0.0, 0.0),
        "StepFunction_ScreenShake restores the baseline before deleting on its final update"
    );
    earthquake.advance(1);
    assert_eq!(earthquake.shake_frames_remaining, 0);
    assert_eq!(
        super::visible_earthquake_camera_offset(Some(earthquake)),
        (0.0, 0.0),
        "the generated step_sleep phase must not continue shaking"
    );
}

#[test]
fn poison_step_uses_four_opaque_bg_palette_frames_then_one_restored_frame() {
    let poison = [230.0 / 255.0, 173.0 / 255.0, 1.0, 1.0];
    for frames_remaining in [5, 4, 3, 2] {
        assert_eq!(
            super::visible_poison_bg_palette_rgba(frames_remaining),
            Some(poison),
            "LoadPoisonBGPals keeps every CGB background color at the poison color"
        );
    }
    assert_eq!(
        super::visible_poison_bg_palette_rgba(1),
        None,
        "the trailing DelayFrame runs after _UpdateTimePals restores the map palette"
    );
    assert_eq!(super::visible_poison_bg_palette_rgba(0), None);
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
