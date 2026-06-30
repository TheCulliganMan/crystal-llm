use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use bevy::audio::{AudioBundle, AudioSource, PlaybackSettings};
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use crystal_assets::{
    RuntimeBadgeRegion, RuntimeBugContestAction, RuntimeCurrencyAccount, RuntimeDayCareAction,
    RuntimeDayCareCaretaker, RuntimeGameCornerService, RuntimeGraphicsSpecial,
    RuntimeHappinessServiceRoutine, RuntimeLinkBattleResult, RuntimeMysteryGiftAction,
    RuntimeNoopSpecial, RuntimePartyCheckSpecial, RuntimePhoneRandomSpecial, RuntimeShuckieAction,
    RuntimeStoryGateSpecial,
};

use crate::assets::{
    ModpackAudioKind, ModpackAudioPlaybackMode, RuntimeMutationOutcome, RuntimeMutationResult,
    RuntimePendingScriptRequestKind, RuntimeScriptEventQueue, RuntimeScriptRuntimeFlag,
    RuntimeScriptRuntimeMemoryEntry, RuntimeScriptRuntimeMemoryValue, RuntimeScriptRuntimeQueue,
    RuntimeScriptRuntimeRecordQueue,
};
use crate::audio::AudioProgramSource;
use crate::core::battle::turn::BattleAction;
use crate::core::input::GameButton;
use crate::core::models::Dv;
use crate::core::multiplayer::encode_link_message_bytes;
use crate::core::state::BattleStyle;
use crate::core::systems::battle_items::{
    ITEM_EFFECT_BEHAVIOR_EVOLUTION_STONE, ITEM_EFFECT_BEHAVIOR_RARE_CANDY,
};
use crate::core::systems::phone::ScriptPhoneInputs;
use crate::core::systems::script_control::ScriptControlAction;
use crate::core::systems::script_runtime::ScriptRuntimeInputs;
use crate::core::systems::time::{ClockTime, GameDate};
use crate::core::world::encounters::EncounterSurface;
use crate::core::world::map::TilePosition;
use crate::{
    CrystalRuntime, RuntimeBagItemSnapshot, RuntimeBattleCommandSnapshot, RuntimeBattleKind,
    RuntimeGameShell, RuntimeLinkSessionDescriptor, RuntimeResolvedAudioPlaybackKind,
    RuntimeShellSnapshot, assets::AssetRoot,
};

const GAME_TICK_SECONDS: f32 = 1.0 / 30.0;
const VIEWPORT_TILES_X: i16 = 20;
const VIEWPORT_TILES_Y: i16 = 18;
const TILE_SIZE: f32 = 24.0;
const PLAYFIELD_LEFT: f32 = -240.0;
const PLAYFIELD_TOP: f32 = 216.0;
const EVENT_LOG_LIMIT: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BevyShellStart {
    NewGame { spawn_identifier: u16 },
    LoadSave { save_path: PathBuf },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BevyShellConfig {
    pub quick_save_path: Option<PathBuf>,
}

#[derive(Resource)]
struct BevyRuntimeShell {
    shell: RuntimeGameShell,
    last_error: Option<String>,
    last_audio_events: Vec<String>,
    pending_audio: Vec<BevyAudioCommand>,
    last_battle_cry_key: Option<String>,
    quick_save_path: Option<PathBuf>,
    interaction_script_cursor: Option<ScriptCursor>,
    coord_event_script_cursor: Option<ScriptCursor>,
    active_script_cursor: Option<ActiveScriptCursor>,
    pending_map_callbacks: Vec<String>,
    pending_scene_script: Option<String>,
    script_command_cursor: usize,
    menu_cursor: Option<MenuCursor>,
    sell_cursor: Option<MenuCursor>,
    party_cursor: usize,
    bag_cursor: Option<MenuCursor>,
    ball_cursor: Option<MenuCursor>,
    tmhm_cursor: Option<MenuCursor>,
    storage_cursor: Option<MenuCursor>,
    pc_item_cursor: Option<MenuCursor>,
    fly_cursor: Option<MenuCursor>,
    battle_action_cursor: Option<MenuCursor>,
    battle_move_cursor: Option<MenuCursor>,
    battle_switch_cursor: Option<MenuCursor>,
    party_move_cursor: Option<MenuCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptCursor {
    source_script: String,
    frame: u64,
    next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveScriptCursor {
    source_script: String,
    next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MenuCursor {
    surface_id: String,
    option_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BevyAudioCommand {
    audio_id: String,
    kind: ModpackAudioKind,
    mode: ModpackAudioPlaybackMode,
    looped: bool,
}

#[derive(Resource)]
struct RuntimeTickTimer(Timer);

#[derive(Resource, Default)]
struct RenderedViewport {
    map_name: Option<String>,
    tile: Option<TilePosition>,
    state_hash: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
enum HudMode {
    Status,
    Party,
    Bag,
    Battle,
    Ui,
    Progress,
    Storage,
    Map,
    Scripts,
    Audio,
    Special,
}

impl HudMode {
    const fn next(self) -> Self {
        match self {
            Self::Status => Self::Party,
            Self::Party => Self::Bag,
            Self::Bag => Self::Battle,
            Self::Battle => Self::Ui,
            Self::Ui => Self::Progress,
            Self::Progress => Self::Storage,
            Self::Storage => Self::Map,
            Self::Map => Self::Scripts,
            Self::Scripts => Self::Audio,
            Self::Audio => Self::Special,
            Self::Special => Self::Status,
        }
    }
}

#[derive(Component)]
struct StatusText;

#[derive(Component)]
struct DialogText;

#[derive(Component)]
struct BattleText;

#[derive(Component)]
struct DialogPanel;

#[derive(Component)]
struct BattlePanel;

#[derive(Component)]
struct PlayfieldTile;

#[derive(Component)]
struct PlayerMarker;

#[derive(Component)]
struct ObjectMarker;

#[derive(Component)]
struct EventMarker;

pub fn run_bevy_shell(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
) -> Result<()> {
    let shell = match start {
        BevyShellStart::NewGame { spawn_identifier } => {
            RuntimeGameShell::new_game(asset_root, runtime, spawn_identifier)?
        }
        BevyShellStart::LoadSave { save_path } => {
            RuntimeGameShell::resume_from_save(asset_root, runtime, save_path)?
        }
    };

    App::new()
        .insert_resource(ClearColor(Color::rgb(0.05, 0.07, 0.06)))
        .insert_resource(BevyRuntimeShell {
            shell,
            last_error: None,
            last_audio_events: Vec::new(),
            pending_audio: Vec::new(),
            last_battle_cry_key: None,
            quick_save_path: config.quick_save_path,
            interaction_script_cursor: None,
            coord_event_script_cursor: None,
            active_script_cursor: None,
            pending_map_callbacks: Vec::new(),
            pending_scene_script: None,
            script_command_cursor: 0,
            menu_cursor: None,
            sell_cursor: None,
            party_cursor: 0,
            bag_cursor: None,
            ball_cursor: None,
            tmhm_cursor: None,
            storage_cursor: None,
            pc_item_cursor: None,
            fly_cursor: None,
            battle_action_cursor: None,
            battle_move_cursor: None,
            battle_switch_cursor: None,
            party_move_cursor: None,
        })
        .insert_resource(RuntimeTickTimer(Timer::from_seconds(
            GAME_TICK_SECONDS,
            TimerMode::Repeating,
        )))
        .insert_resource(RenderedViewport::default())
        .insert_resource(HudMode::Status)
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Pokemon Crystal Rust".to_string(),
                resolution: WindowResolution::new(640.0, 576.0),
                present_mode: PresentMode::AutoVsync,
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_shell_view)
        .add_systems(
            Update,
            (
                apply_keyboard_input,
                apply_runtime_hotkeys,
                apply_hud_hotkeys,
                drain_runtime_audio_events,
                queue_battle_intro_cry,
                play_pending_audio,
                render_playfield,
                refresh_status_text,
                refresh_dialog_text,
                refresh_battle_text,
            ),
        )
        .run();

    Ok(())
}

fn setup_shell_view(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.06, 0.08, 0.10, 0.90),
                custom_size: Some(Vec2::new(612.0, 116.0)),
                ..default()
            },
            transform: Transform::from_xyz(0.0, -222.0, 5.0),
            ..default()
        },
        DialogPanel,
    ));
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgba(0.07, 0.09, 0.12, 0.82),
                custom_size: Some(Vec2::new(302.0, 128.0)),
                ..default()
            },
            transform: Transform::from_xyz(165.0, 216.0, 4.0),
            ..default()
        },
        BattlePanel,
    ));
    commands.spawn((
        TextBundle::from_section(
            "Loading runtime...",
            TextStyle {
                font_size: 18.0,
                color: Color::rgb(0.88, 0.94, 0.86),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(18.0),
            top: Val::Px(18.0),
            max_width: Val::Px(604.0),
            ..default()
        }),
        StatusText,
    ));
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 20.0,
                color: Color::rgb(0.97, 0.97, 0.90),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            left: Val::Px(22.0),
            bottom: Val::Px(20.0),
            max_width: Val::Px(596.0),
            ..default()
        }),
        DialogText,
    ));
    commands.spawn((
        TextBundle::from_section(
            "",
            TextStyle {
                font_size: 18.0,
                color: Color::rgb(0.94, 0.97, 0.99),
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(18.0),
            max_width: Val::Px(290.0),
            ..default()
        }),
        BattleText,
    ));
}

fn apply_keyboard_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut timer: ResMut<RuntimeTickTimer>,
) {
    timer.0.tick(time.delta());
    if !timer.0.finished() {
        return;
    }

    let shell_consumes_a = has_visible_shell_a_action(&runtime_shell);
    let shell_consumes_b = has_visible_shell_b_action(&runtime_shell);
    let shell_consumes_direction = has_visible_shell_direction_action(&runtime_shell);
    let mut buttons = Vec::new();
    for (key, button, shell_consumes_button) in [
        (KeyCode::ArrowUp, GameButton::Up, shell_consumes_direction),
        (
            KeyCode::ArrowDown,
            GameButton::Down,
            shell_consumes_direction,
        ),
        (
            KeyCode::ArrowLeft,
            GameButton::Left,
            shell_consumes_direction,
        ),
        (
            KeyCode::ArrowRight,
            GameButton::Right,
            shell_consumes_direction,
        ),
        (KeyCode::KeyZ, GameButton::A, shell_consumes_a),
        (KeyCode::KeyX, GameButton::B, shell_consumes_b),
        (KeyCode::Enter, GameButton::Start, false),
        (KeyCode::ShiftRight, GameButton::Select, false),
    ] {
        if keys.pressed(key) && !shell_consumes_button {
            buttons.push(button);
        }
    }
    let input_active = !buttons.is_empty();

    let mut tick_ok = false;
    let mut execute_coord_event_script = false;
    let mut execute_interaction_script = false;
    match runtime_shell.shell.tick(buttons) {
        Ok(frame) => {
            tick_ok = true;
            execute_coord_event_script = frame.coord_event.is_some();
            execute_interaction_script =
                keys.just_pressed(KeyCode::KeyZ) && frame.interaction.is_some();
            let frame_activity = if input_active {
                summarize_frame_activity(frame)
            } else {
                None
            };
            runtime_shell.last_error = None;
            if let Some(activity) = frame_activity {
                runtime_shell.last_audio_events.push(activity);
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
        }
        Err(error) => runtime_shell.last_error = Some(error.to_string()),
    }
    sync_visible_battle_action_cursor(&mut runtime_shell);
    if !tick_ok {
        return;
    }
    if execute_coord_event_script {
        run_bevy_action(&mut runtime_shell, execute_last_coord_event_script);
    } else if execute_interaction_script {
        run_bevy_action(&mut runtime_shell, execute_last_interaction_script);
    } else {
        match advance_visible_script_until_player_boundary(&mut runtime_shell) {
            Ok(()) => {}
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }
}

fn apply_hud_hotkeys(keys: Res<ButtonInput<KeyCode>>, mut mode: ResMut<HudMode>) {
    let shift_pressed = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::Tab) {
        *mode = mode.next();
    }
    if keys.just_pressed(KeyCode::F6) && !shift_pressed {
        *mode = HudMode::Status;
    }
    if keys.just_pressed(KeyCode::F7) && !shift_pressed {
        *mode = HudMode::Party;
    }
    if keys.just_pressed(KeyCode::F8) && !shift_pressed {
        *mode = HudMode::Bag;
    }
    if keys.just_pressed(KeyCode::F9) && !shift_pressed {
        *mode = HudMode::Battle;
    }
    if keys.just_pressed(KeyCode::F10) && !shift_pressed {
        *mode = HudMode::Ui;
    }
    if keys.just_pressed(KeyCode::F11) && !shift_pressed {
        *mode = HudMode::Progress;
    }
    if keys.just_pressed(KeyCode::F12) && !shift_pressed {
        *mode = HudMode::Map;
    }
    if keys.just_pressed(KeyCode::F6) && shift_pressed {
        *mode = HudMode::Storage;
    }
    if keys.just_pressed(KeyCode::F7) && shift_pressed {
        *mode = HudMode::Scripts;
    }
    if keys.just_pressed(KeyCode::F8) && shift_pressed {
        *mode = HudMode::Audio;
    }
    if keys.just_pressed(KeyCode::F9) && shift_pressed {
        *mode = HudMode::Special;
    }
}

fn apply_runtime_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
) {
    let shift_pressed = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt_pressed = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let ctrl_pressed = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if keys.just_pressed(KeyCode::Comma) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, queue_selected_music_preview);
    }
    if keys.just_pressed(KeyCode::KeyS) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, queue_selected_sound_effect_preview);
    }
    if keys.just_pressed(KeyCode::KeyY) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, queue_selected_cry_preview);
    }

    if keys.just_pressed(KeyCode::ArrowUp) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_primary_cursor_up);
    }

    if keys.just_pressed(KeyCode::ArrowDown) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_primary_cursor_down);
    }

    if keys.just_pressed(KeyCode::ArrowLeft) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_primary_cursor_left);
    }

    if keys.just_pressed(KeyCode::ArrowRight) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_primary_cursor_right);
    }

    if keys.just_pressed(KeyCode::ArrowUp) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_sell_cursor_up);
    }

    if keys.just_pressed(KeyCode::ArrowDown) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_sell_cursor_down);
    }

    if keys.just_pressed(KeyCode::PageUp) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_party_cursor_up);
    }

    if keys.just_pressed(KeyCode::PageDown) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_party_cursor_down);
    }

    if keys.just_pressed(KeyCode::PageUp) && !shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_bag_cursor_up);
    }

    if keys.just_pressed(KeyCode::PageDown) && !shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_bag_cursor_down);
    }

    if keys.just_pressed(KeyCode::PageUp) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_ball_cursor_up);
    }

    if keys.just_pressed(KeyCode::PageDown) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_ball_cursor_down);
    }

    if keys.just_pressed(KeyCode::PageUp) && !shift_pressed && !alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_tmhm_cursor_up);
    }

    if keys.just_pressed(KeyCode::PageDown) && !shift_pressed && !alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_tmhm_cursor_down);
    }

    if keys.just_pressed(KeyCode::PageUp) && !shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_storage_cursor_up);
    }

    if keys.just_pressed(KeyCode::PageDown) && !shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_storage_cursor_down);
    }

    if keys.just_pressed(KeyCode::PageUp) && shift_pressed && !alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_pc_item_cursor_up);
    }

    if keys.just_pressed(KeyCode::PageDown) && shift_pressed && !alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_pc_item_cursor_down);
    }

    if keys.just_pressed(KeyCode::Home) && !shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_fly_cursor_up);
    }

    if keys.just_pressed(KeyCode::End) && !shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_fly_cursor_down);
    }

    if keys.just_pressed(KeyCode::BracketLeft) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_battle_move_cursor_up);
    }

    if keys.just_pressed(KeyCode::BracketRight) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_battle_move_cursor_down);
    }

    if keys.just_pressed(KeyCode::BracketLeft) && !shift_pressed && !alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_party_move_cursor_up);
    }

    if keys.just_pressed(KeyCode::BracketRight) && !shift_pressed && !alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, move_visible_party_move_cursor_down);
    }

    if keys.just_pressed(KeyCode::Space) && !shift_pressed {
        run_bevy_action(&mut runtime_shell, advance_visible_pending_text_wait);
    }

    if keys.just_pressed(KeyCode::KeyZ) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        if has_visible_shell_a_action(&runtime_shell) {
            run_bevy_action(&mut runtime_shell, press_visible_a_button);
        }
    }

    if keys.just_pressed(KeyCode::KeyX) && !shift_pressed && !alt_pressed && !ctrl_pressed {
        if has_visible_shell_b_action(&runtime_shell) {
            run_bevy_action(&mut runtime_shell, press_visible_b_button);
        }
    }

    if keys.just_pressed(KeyCode::KeyY) && !shift_pressed {
        run_bevy_action(&mut runtime_shell, accept_visible_pending_yes_no);
    }

    if keys.just_pressed(KeyCode::KeyN) && !shift_pressed {
        let has_yes_no = runtime_shell
            .shell
            .snapshot()
            .map(|snapshot| snapshot.ui.pending_yes_no.is_some())
            .unwrap_or(false);
        if has_yes_no {
            run_bevy_action(&mut runtime_shell, decline_visible_pending_yes_no);
        } else {
            match use_visible_whirlpool(&mut runtime_shell) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => runtime_shell.last_error = Some(error.to_string()),
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape) {
        run_bevy_action(&mut runtime_shell, press_visible_b_button);
    }

    if keys.just_pressed(KeyCode::KeyP) && !shift_pressed && !alt_pressed {
        match use_visible_repel(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyV) && !shift_pressed {
        match use_visible_bicycle(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyT) && !shift_pressed {
        match use_visible_town_map(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyE) && !shift_pressed {
        match use_visible_escape_rope(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyF) && !shift_pressed && !alt_pressed {
        match use_visible_fishing_rod(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyF) && alt_pressed && !shift_pressed {
        run_bevy_action(&mut runtime_shell, use_visible_fly);
    }

    if keys.just_pressed(KeyCode::KeyI) && !shift_pressed && !alt_pressed {
        match use_visible_itemfinder(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyQ) && !shift_pressed {
        match use_visible_squirtbottle(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyG) && !shift_pressed {
        match use_visible_coin_case(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyU) && !shift_pressed {
        match use_visible_blue_card(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyA) && !shift_pressed {
        match use_visible_surf(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyK) && !shift_pressed && !alt_pressed {
        match use_visible_strength(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyJ) && !shift_pressed {
        match use_visible_flash(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyW) && !shift_pressed {
        match use_visible_waterfall(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyD) && !shift_pressed {
        match use_visible_dig(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyH) && !shift_pressed {
        match use_visible_headbutt(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    for (key, option_index) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ] {
        if keys.just_pressed(key) && !shift_pressed && !alt_pressed && !ctrl_pressed {
            run_bevy_action_with_index(
                &mut runtime_shell,
                option_index,
                select_visible_primary_action,
            );
        }
    }

    for (key, option_index) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
        (KeyCode::Digit7, 6),
        (KeyCode::Digit8, 7),
        (KeyCode::Digit9, 8),
    ] {
        if keys.just_pressed(key) && ctrl_pressed && !shift_pressed && !alt_pressed {
            match select_visible_linked_menu_option(&mut runtime_shell, option_index) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => runtime_shell.last_error = Some(error.to_string()),
            }
        }
    }

    for (key, party_index) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
    ] {
        if keys.just_pressed(key) && alt_pressed && !shift_pressed {
            match switch_visible_battle_pokemon_to(&mut runtime_shell, party_index) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => runtime_shell.last_error = Some(error.to_string()),
            }
        }
    }

    for (key, ball_index) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
        (KeyCode::Digit5, 4),
        (KeyCode::Digit6, 5),
    ] {
        if keys.just_pressed(key) && alt_pressed && shift_pressed {
            match throw_visible_battle_ball_at(&mut runtime_shell, ball_index) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => runtime_shell.last_error = Some(error.to_string()),
            }
        }
    }

    for (key, slot) in [
        (KeyCode::F1, 0),
        (KeyCode::F2, 1),
        (KeyCode::F3, 2),
        (KeyCode::F4, 3),
    ] {
        if keys.just_pressed(key) && !shift_pressed {
            match resolve_visible_battle_move(&mut runtime_shell, slot) {
                Ok(()) => runtime_shell.last_error = None,
                Err(error) => runtime_shell.last_error = Some(error.to_string()),
            }
        }
    }

    if keys.just_pressed(KeyCode::F4) && shift_pressed {
        run_bevy_action(&mut runtime_shell, advance_visible_trainer_battle);
    }
    if keys.just_pressed(KeyCode::F1) && shift_pressed && !ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_move_deletion);
    }
    if keys.just_pressed(KeyCode::F2) && shift_pressed && !ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_name_rater);
    }
    if keys.just_pressed(KeyCode::F3) && shift_pressed && !ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_move_tutor);
    }
    if keys.just_pressed(KeyCode::F3) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, check_visible_pokerus);
    }
    if keys.just_pressed(KeyCode::F4) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_poke_seer);
    }
    if keys.just_pressed(KeyCode::F1) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, give_visible_shuckie);
    }
    if keys.just_pressed(KeyCode::F2) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, return_visible_shuckie);
    }
    if keys.just_pressed(KeyCode::F3) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, give_visible_odd_egg);
    }

    if keys.just_pressed(KeyCode::KeyR) && !shift_pressed {
        match run_or_rock_smash(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyB) && !shift_pressed {
        match throw_visible_battle_ball(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyS) && !shift_pressed {
        match switch_visible_battle_pokemon(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyC) && !shift_pressed {
        let in_battle = runtime_shell
            .shell
            .snapshot()
            .map(|snapshot| snapshot.battle.is_some())
            .unwrap_or(false);
        let result = if in_battle {
            claim_visible_battle_rewards(&mut runtime_shell)
        } else {
            use_visible_cut(&mut runtime_shell)
        };
        match result {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyM) && !shift_pressed {
        match buy_selected_shop_item(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyL) && !shift_pressed && !alt_pressed {
        match sell_selected_bag_item(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyO) && !shift_pressed {
        match close_shop_or_teleport(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::Comma) && !shift_pressed && !alt_pressed {
        match use_visible_sweet_scent(&mut runtime_shell, EncounterSurface::Grass) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::Period) && !shift_pressed {
        match use_visible_sweet_scent(&mut runtime_shell, EncounterSurface::Water) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::KeyZ) && shift_pressed {
        match execute_next_visible_queued_script_command(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::Enter) && shift_pressed {
        match take_visible_next_script(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::Space) && shift_pressed {
        run_bevy_action(&mut runtime_shell, advance_visible_text_label);
    }

    if keys.just_pressed(KeyCode::PageDown) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_pending_map_load);
    }

    if keys.just_pressed(KeyCode::PageUp) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_pending_map_refresh);
    }

    if keys.just_pressed(KeyCode::Home) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_pending_music_fade);
    }

    if keys.just_pressed(KeyCode::End) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_pending_screen_fade);
    }

    if keys.just_pressed(KeyCode::Home) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, deposit_visible_bag_item_to_pc);
    }

    if keys.just_pressed(KeyCode::End) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, withdraw_visible_pc_item_to_bag);
    }

    if keys.just_pressed(KeyCode::Insert) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_pending_shop_request);
    }

    if keys.just_pressed(KeyCode::Insert) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, open_selected_script_shop);
    }

    if keys.just_pressed(KeyCode::KeyX) && keys.pressed(KeyCode::ShiftLeft) && !alt_pressed {
        run_bevy_action(&mut runtime_shell, clear_visible_menu_coords);
    }

    if keys.just_pressed(KeyCode::KeyE) && shift_pressed {
        run_bevy_action(&mut runtime_shell, select_visible_elevator_floor);
    }

    if keys.just_pressed(KeyCode::KeyA) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_audio_events);
    }

    if keys.just_pressed(KeyCode::KeyM) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_map_events);
    }

    if keys.just_pressed(KeyCode::KeyT) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_text_events);
    }

    if keys.just_pressed(KeyCode::Backspace) && shift_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_misc_script_events);
    }

    if keys.just_pressed(KeyCode::KeyD) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_delays);
    }

    if keys.just_pressed(KeyCode::Backslash) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_misc_runtime_queues);
    }

    if keys.just_pressed(KeyCode::PageDown) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, bug_contest_give_park_balls);
    }

    if keys.just_pressed(KeyCode::PageUp) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, bug_contest_select_contestants);
    }

    if keys.just_pressed(KeyCode::Home) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, bug_contest_drop_off_mons);
    }

    if keys.just_pressed(KeyCode::End) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, bug_contest_return_mons);
    }

    if keys.just_pressed(KeyCode::Insert) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, bug_contest_check_party_full);
    }

    if keys.just_pressed(KeyCode::Backslash) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, judge_visible_bug_contest_rank);
    }

    if keys.just_pressed(KeyCode::KeyA) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, use_visible_kurt_apricorn);
    }

    if keys.just_pressed(KeyCode::KeyR) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, drain_visible_runtime_records);
    }

    if keys.just_pressed(KeyCode::KeyF) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, consume_visible_runtime_flag);
    }

    if keys.just_pressed(KeyCode::Semicolon) && shift_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_script_value);
    }

    if keys.just_pressed(KeyCode::Quote) && shift_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_last_special_routine);
    }

    if keys.just_pressed(KeyCode::Slash) && shift_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_last_talked_object);
    }

    if keys.just_pressed(KeyCode::Digit7) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, remove_selected_runtime_variable);
    }

    if keys.just_pressed(KeyCode::Digit8) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, remove_selected_runtime_memory);
    }

    if keys.just_pressed(KeyCode::Digit9) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, remove_selected_named_buffer);
    }
    if keys.just_pressed(KeyCode::Digit1) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, set_visible_clock_morning);
    }
    if keys.just_pressed(KeyCode::Digit2) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, set_visible_clock_day);
    }
    if keys.just_pressed(KeyCode::Digit3) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, set_visible_clock_night);
    }
    if keys.just_pressed(KeyCode::Digit4) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, set_visible_manual_clock_evening);
    }
    if keys.just_pressed(KeyCode::Digit5) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_battle_escape_item);
    }
    if keys.just_pressed(KeyCode::Digit6) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_guard_spec);
    }

    if keys.just_pressed(KeyCode::KeyG) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, grant_selected_gift_pokemon);
    }

    if keys.just_pressed(KeyCode::KeyG) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, grant_selected_script_item);
    }

    if keys.just_pressed(KeyCode::KeyW) && shift_pressed && !ctrl_pressed {
        run_bevy_action(
            &mut runtime_shell,
            start_or_complete_visible_scripted_wild_battle,
        );
    }

    if keys.just_pressed(KeyCode::KeyG) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, give_visible_dratini);
    }

    if keys.just_pressed(KeyCode::KeyH) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_visible_bills_grandfather);
    }

    if keys.just_pressed(KeyCode::KeyW) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, check_visible_magikarp_length);
    }

    if keys.just_pressed(KeyCode::KeyO) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, show_visible_prof_oaks_pc_boot);
    }

    if keys.just_pressed(KeyCode::KeyP) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, show_visible_magikarp_house_sign);
    }

    if keys.just_pressed(KeyCode::KeyB) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_battle_tower_reset);
    }

    if keys.just_pressed(KeyCode::KeyB) && shift_pressed && !ctrl_pressed {
        run_bevy_action(
            &mut runtime_shell,
            start_or_complete_visible_scripted_trainer_battle,
        );
    }

    if keys.just_pressed(KeyCode::KeyB) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, award_visible_badge);
    }

    if keys.just_pressed(KeyCode::KeyI) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_party_item);
    }

    if keys.just_pressed(KeyCode::KeyI) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_rare_candy);
    }

    if keys.just_pressed(KeyCode::KeyI) && alt_pressed && !shift_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_party_item_on_second_slot);
    }

    if keys.just_pressed(KeyCode::KeyI) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, give_selected_held_item);
    }

    if keys.just_pressed(KeyCode::KeyO) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_whole_party_item);
    }

    if keys.just_pressed(KeyCode::KeyO) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, record_selected_pokedex_caught);
    }

    if keys.just_pressed(KeyCode::KeyM) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, add_visible_money);
    }

    if keys.just_pressed(KeyCode::KeyM) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, take_visible_money);
    }

    if keys.just_pressed(KeyCode::KeyC) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, add_visible_coins);
    }

    if keys.just_pressed(KeyCode::KeyT) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, toggle_visible_battle_style);
    }

    if keys.just_pressed(KeyCode::KeyS) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, swap_visible_selected_party_pokemon);
    }

    if keys.just_pressed(KeyCode::KeyP) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_pp_item);
    }

    if keys.just_pressed(KeyCode::KeyP) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, record_selected_pokedex_seen);
    }

    if keys.just_pressed(KeyCode::KeyP) && alt_pressed && !shift_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_pp_item_on_second_slot);
    }

    if keys.just_pressed(KeyCode::KeyK) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, teach_selected_tmhm);
    }

    if keys.just_pressed(KeyCode::KeyK) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_evolution_item);
    }

    if keys.just_pressed(KeyCode::KeyK) && alt_pressed && !shift_pressed {
        run_bevy_action(&mut runtime_shell, teach_selected_tmhm_on_second_slot);
    }

    if keys.just_pressed(KeyCode::KeyK) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, take_selected_held_item);
    }

    if keys.just_pressed(KeyCode::KeyS) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_active_battle_item);
    }

    if keys.just_pressed(KeyCode::KeyL) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_battle_party_item);
    }

    if keys.just_pressed(KeyCode::KeyL) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, record_visible_link_win);
    }

    if keys.just_pressed(KeyCode::KeyL) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, record_visible_link_loss);
    }

    if keys.just_pressed(KeyCode::KeyD) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, record_visible_link_draw);
    }

    if keys.just_pressed(KeyCode::KeyL) && alt_pressed && !shift_pressed {
        run_bevy_action(
            &mut runtime_shell,
            use_selected_battle_party_item_on_second_slot,
        );
    }

    if keys.just_pressed(KeyCode::Comma) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, use_selected_battle_party_move_item);
    }

    if keys.just_pressed(KeyCode::Comma) && alt_pressed && !shift_pressed {
        run_bevy_action(
            &mut runtime_shell,
            use_selected_battle_party_move_item_on_second_slot,
        );
    }

    if keys.just_pressed(KeyCode::Period) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_time_money_special);
    }

    if keys.just_pressed(KeyCode::KeyN) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, initialize_visible_phone_numbers);
    }

    if keys.just_pressed(KeyCode::KeyV) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_phone_command);
    }

    if keys.just_pressed(KeyCode::KeyC) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_swarm_command);
    }

    if keys.just_pressed(KeyCode::KeyC) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, check_selected_script_item);
    }

    if keys.just_pressed(KeyCode::KeyT) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, take_selected_script_item);
    }

    if keys.just_pressed(KeyCode::KeyM) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_economy_command);
    }

    if keys.just_pressed(KeyCode::KeyF) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, pickup_selected_script_field_item);
    }

    if keys.just_pressed(KeyCode::KeyF) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, set_visible_player_palette);
    }

    if keys.just_pressed(KeyCode::KeyT) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, set_visible_day_of_week);
    }

    if keys.just_pressed(KeyCode::KeyE) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, update_visible_time);
    }

    if keys.just_pressed(KeyCode::PageDown) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, warp_visible_to_spawn_point);
    }

    if keys.just_pressed(KeyCode::Home) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, fade_visible_music_special);
    }

    if keys.just_pressed(KeyCode::End) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, wait_visible_sfx_special);
    }

    if keys.just_pressed(KeyCode::KeyP) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, play_visible_map_music_special);
    }

    if keys.just_pressed(KeyCode::KeyK) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, restart_visible_map_music_special);
    }

    if keys.just_pressed(KeyCode::KeyR) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_flag_mutation);
    }

    if keys.just_pressed(KeyCode::KeyR) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, init_visible_roam_mons);
    }

    if keys.just_pressed(KeyCode::KeyQ) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, check_selected_script_flag);
    }

    if keys.just_pressed(KeyCode::KeyS) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_scene_command);
    }

    if keys.just_pressed(KeyCode::KeyD) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_block_change);
    }

    if keys.just_pressed(KeyCode::KeyA) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_audio_command);
    }

    if keys.just_pressed(KeyCode::KeyX) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_text_command);
    }

    if keys.just_pressed(KeyCode::KeyA) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_older_haircut);
    }

    if keys.just_pressed(KeyCode::KeyD) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_younger_haircut);
    }

    if keys.just_pressed(KeyCode::KeyX) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_daisy_grooming);
    }

    if keys.just_pressed(KeyCode::KeyC) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, check_visible_mystery_gift);
    }

    if keys.just_pressed(KeyCode::KeyV) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, claim_visible_mystery_gift_item);
    }

    if keys.just_pressed(KeyCode::KeyN) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, unlock_visible_mystery_gift);
    }

    if keys.just_pressed(KeyCode::KeyZ) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_story_gate_special);
    }

    if keys.just_pressed(KeyCode::KeyU) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_day_care_status_special);
    }

    if keys.just_pressed(KeyCode::KeyJ) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_noop_special);
    }

    if keys.just_pressed(KeyCode::Digit1) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_graphics_special);
    }

    if keys.just_pressed(KeyCode::Digit2) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_party_check_special);
    }

    if keys.just_pressed(KeyCode::Digit3) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_phone_random_special);
    }

    if keys.just_pressed(KeyCode::Digit4) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, check_selected_item_in_pc_or_bag_special);
    }

    if keys.just_pressed(KeyCode::Digit5) && shift_pressed && alt_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, activate_visible_fishing_swarm_special);
    }

    if keys.just_pressed(KeyCode::KeyV) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_variable_command);
    }

    if keys.just_pressed(KeyCode::KeyV) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, use_visible_buena_password);
    }

    if keys.just_pressed(KeyCode::Period) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, use_visible_buena_prize);
    }

    if keys.just_pressed(KeyCode::KeyL) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_control_command);
    }

    if keys.just_pressed(KeyCode::KeyO) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_object_mutation);
    }

    if keys.just_pressed(KeyCode::KeyY) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_trade_command);
    }

    if keys.just_pressed(KeyCode::KeyY) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, deposit_visible_day_care_man);
    }

    if keys.just_pressed(KeyCode::KeyY) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, deposit_visible_day_care_lady);
    }

    if keys.just_pressed(KeyCode::KeyU) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, withdraw_visible_day_care_man);
    }

    if keys.just_pressed(KeyCode::KeyU) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, withdraw_visible_day_care_lady);
    }

    if keys.just_pressed(KeyCode::KeyJ) && shift_pressed && ctrl_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, inspect_visible_day_care_man);
    }

    if keys.just_pressed(KeyCode::KeyJ) && shift_pressed && ctrl_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, inspect_visible_day_care_lady);
    }

    if keys.just_pressed(KeyCode::KeyH) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_catch_tutorial_command);
    }

    if keys.just_pressed(KeyCode::KeyU) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, execute_last_interaction_script);
    }

    if keys.just_pressed(KeyCode::Digit0) && shift_pressed {
        run_bevy_action(&mut runtime_shell, execute_last_coord_event_script);
    }

    if keys.just_pressed(KeyCode::KeyJ) && shift_pressed && !alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, execute_visible_pending_script_warp);
    }

    if keys.just_pressed(KeyCode::KeyJ) && shift_pressed && alt_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_script_map_command);
    }

    if keys.just_pressed(KeyCode::Minus) && shift_pressed {
        reset_script_cursors(&mut runtime_shell);
    }

    if keys.just_pressed(KeyCode::BracketRight) && shift_pressed && alt_pressed {
        shift_visible_script_command_cursor(&mut runtime_shell, 1);
    }

    if keys.just_pressed(KeyCode::BracketLeft) && shift_pressed && alt_pressed {
        shift_visible_script_command_cursor(&mut runtime_shell, -1);
    }

    if keys.just_pressed(KeyCode::Delete) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, prepare_visible_local_link_descriptor);
    }

    if keys.just_pressed(KeyCode::Delete) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, release_visible_current_box_pokemon);
    }

    if keys.just_pressed(KeyCode::Backspace) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, resolve_visible_blackout);
    }

    if keys.just_pressed(KeyCode::F5) && ctrl_pressed && !shift_pressed {
        match quick_load(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::F5) && !shift_pressed && !ctrl_pressed {
        match quick_save(&mut runtime_shell) {
            Ok(()) => runtime_shell.last_error = None,
            Err(error) => runtime_shell.last_error = Some(error.to_string()),
        }
    }

    if keys.just_pressed(KeyCode::F5) && shift_pressed && !ctrl_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_heal_party);
    }
    if keys.just_pressed(KeyCode::F5) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, full_heal_visible_whole_party);
    }
    if keys.just_pressed(KeyCode::F6) && shift_pressed && ctrl_pressed {
        run_bevy_action(&mut runtime_shell, full_heal_visible_party_lead);
    }
    if keys.just_pressed(KeyCode::F10) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_pokemon_center_pc);
    }
    if keys.just_pressed(KeyCode::F11) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_players_house_pc);
    }
    if keys.just_pressed(KeyCode::F12) && shift_pressed && !alt_pressed {
        run_bevy_action(&mut runtime_shell, apply_visible_overworld_town_map);
    }
    if keys.just_pressed(KeyCode::F10) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, switch_visible_next_pc_box);
    }
    if keys.just_pressed(KeyCode::F11) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, deposit_visible_party_pokemon);
    }
    if keys.just_pressed(KeyCode::F12) && shift_pressed && alt_pressed {
        run_bevy_action(&mut runtime_shell, withdraw_visible_pc_pokemon);
    }
    if keys.just_pressed(KeyCode::Backquote) && shift_pressed {
        run_bevy_action(&mut runtime_shell, apply_selected_service_menu_special);
    }
}

fn run_bevy_action(
    runtime_shell: &mut BevyRuntimeShell,
    action: fn(&mut BevyRuntimeShell) -> Result<()>,
) {
    match action(runtime_shell)
        .and_then(|()| advance_visible_script_until_player_boundary(runtime_shell))
    {
        Ok(()) => runtime_shell.last_error = None,
        Err(error) => runtime_shell.last_error = Some(error.to_string()),
    }
    sync_visible_battle_action_cursor(runtime_shell);
}

fn run_bevy_action_with_index(
    runtime_shell: &mut BevyRuntimeShell,
    index: usize,
    action: fn(&mut BevyRuntimeShell, usize) -> Result<()>,
) {
    match action(runtime_shell, index)
        .and_then(|()| advance_visible_script_until_player_boundary(runtime_shell))
    {
        Ok(()) => runtime_shell.last_error = None,
        Err(error) => runtime_shell.last_error = Some(error.to_string()),
    }
    sync_visible_battle_action_cursor(runtime_shell);
}

fn advance_visible_script_until_player_boundary(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MAX_AUTO_SCRIPT_STEPS: usize = 256;
    let mut advanced = 0usize;
    loop {
        let snapshot = runtime_shell.shell.snapshot()?;
        if visible_player_boundary(&snapshot)
            || !has_visible_auto_script_action(runtime_shell, &snapshot)
        {
            return Ok(());
        }
        if advanced >= MAX_AUTO_SCRIPT_STEPS {
            anyhow::bail!(
                "visible script auto-advance exceeded {MAX_AUTO_SCRIPT_STEPS} steps before reaching a player boundary"
            );
        }
        press_visible_a_button(runtime_shell)?;
        advanced += 1;
    }
}

fn visible_player_boundary(snapshot: &RuntimeShellSnapshot) -> bool {
    snapshot.ui.pending_yes_no.is_some()
        || snapshot.ui.pending_text_wait.is_some()
        || snapshot.pending_shop.is_some()
        || snapshot.ui.text_window_open
        || snapshot.ui.active_pokemon_picture.is_some()
        || !snapshot.ui.elevators.is_empty()
        || !snapshot.ui.gift_pokemon.is_empty()
        || snapshot.ui.menu.is_some()
        || snapshot.battle.is_some()
}

fn has_visible_auto_script_action(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> bool {
    snapshot.script_events.pending_text_label.is_some()
        || snapshot.script_events.pending_map_load.is_some()
        || snapshot.script_events.pending_map_refresh.is_some()
        || snapshot.script_events.pending_music_fade.is_some()
        || snapshot.script_events.pending_screen_fade.is_some()
        || !snapshot.script_events.pending_delays.is_empty()
        || !snapshot.script_events.pending_earthquakes.is_empty()
        || !snapshot.script_events.pending_emotes.is_empty()
        || snapshot.script_events.pending_script_warp.is_some()
        || !snapshot.script_events.command_queue.is_empty()
        || snapshot.script_events.next_script.is_some()
        || !snapshot.script_events.deferred_scripts.is_empty()
        || snapshot.script_events.script_ended.is_some()
        || snapshot.script_events.blackout_mod.is_some()
        || visible_auto_runtime_flag(snapshot).is_some()
        || runtime_shell.active_script_cursor.is_some()
}

fn select_visible_primary_action(
    runtime_shell: &mut BevyRuntimeShell,
    option_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_some() {
        return select_visible_menu_option(runtime_shell, option_index);
    }
    if snapshot.pending_shop.is_some() {
        return buy_visible_shop_item_at(runtime_shell, option_index);
    }
    if let Some(battle) = snapshot.battle {
        if battle.commands.player_move_slots.contains(&option_index) {
            return resolve_visible_battle_move(runtime_shell, option_index);
        }
        anyhow::bail!("battle move slot {option_index} is not available");
    }
    select_visible_menu_option(runtime_shell, option_index)
}

fn press_visible_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.pending_yes_no.is_some() {
        return accept_visible_pending_yes_no(runtime_shell);
    }
    if snapshot.ui.pending_text_wait.is_some() {
        return advance_visible_pending_text_wait(runtime_shell);
    }
    if snapshot.pending_shop.is_some() {
        return buy_visible_shop_cursor_item(runtime_shell);
    }
    if advance_visible_next_pending_script_request(runtime_shell, &snapshot)? {
        return Ok(());
    }
    if snapshot.ui.text_window_open {
        return close_visible_text_window(runtime_shell);
    }
    if snapshot.ui.active_pokemon_picture.is_some() {
        return close_visible_pokemon_picture(runtime_shell);
    }
    if !snapshot.script_events.command_queue.is_empty() {
        return execute_next_visible_queued_script_command(runtime_shell);
    }
    if snapshot.script_events.next_script.is_some() {
        return take_visible_next_script(runtime_shell);
    }
    if !snapshot.script_events.deferred_scripts.is_empty() {
        return take_visible_deferred_script(runtime_shell);
    }
    if snapshot.script_events.script_ended.is_some() {
        return take_visible_script_end_state(runtime_shell);
    }
    if snapshot.script_events.blackout_mod.is_some() {
        return resolve_visible_blackout(runtime_shell);
    }
    if let Some(flag) = visible_auto_runtime_flag(&snapshot) {
        return consume_visible_runtime_flag_kind(runtime_shell, flag);
    }
    if !snapshot.ui.elevators.is_empty() {
        return select_visible_elevator_floor(runtime_shell);
    }
    if !snapshot.ui.gift_pokemon.is_empty() {
        return grant_selected_gift_pokemon(runtime_shell);
    }
    if runtime_shell.active_script_cursor.is_some() {
        return execute_visible_active_script_step(runtime_shell);
    }
    if snapshot.ui.menu.is_some() {
        return select_visible_menu_cursor_option(runtime_shell);
    }
    if snapshot.battle.is_some() {
        return press_visible_battle_a_button(runtime_shell);
    }
    Ok(())
}

fn has_visible_shell_a_action(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| {
            snapshot.ui.pending_yes_no.is_some()
                || snapshot.ui.pending_text_wait.is_some()
                || snapshot.pending_shop.is_some()
                || snapshot.script_events.pending_text_label.is_some()
                || snapshot.script_events.pending_map_load.is_some()
                || snapshot.script_events.pending_map_refresh.is_some()
                || snapshot.script_events.pending_music_fade.is_some()
                || snapshot.script_events.pending_screen_fade.is_some()
                || !snapshot.script_events.pending_delays.is_empty()
                || !snapshot.script_events.pending_earthquakes.is_empty()
                || !snapshot.script_events.pending_emotes.is_empty()
                || snapshot.ui.text_window_open
                || snapshot.ui.active_pokemon_picture.is_some()
                || snapshot.script_events.pending_script_warp.is_some()
                || !snapshot.script_events.command_queue.is_empty()
                || snapshot.script_events.next_script.is_some()
                || !snapshot.script_events.deferred_scripts.is_empty()
                || snapshot.script_events.script_ended.is_some()
                || snapshot.script_events.blackout_mod.is_some()
                || visible_auto_runtime_flag(&snapshot).is_some()
                || !snapshot.ui.elevators.is_empty()
                || !snapshot.ui.gift_pokemon.is_empty()
                || runtime_shell.active_script_cursor.is_some()
                || snapshot.ui.menu.is_some()
                || snapshot.battle.is_some()
        })
        .unwrap_or(false)
}

fn press_visible_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.pending_yes_no.is_some() {
        return decline_visible_pending_yes_no(runtime_shell);
    }
    if snapshot.ui.pending_text_wait.is_some() {
        return advance_visible_pending_text_wait(runtime_shell);
    }
    if snapshot.pending_shop.is_some() {
        return close_visible_shop(runtime_shell);
    }
    if snapshot.ui.text_window_open
        || snapshot.ui.window_open
        || snapshot.ui.menu.is_some()
        || snapshot.ui.active_pokemon_picture.is_some()
    {
        return close_active_runtime_surface(runtime_shell);
    }
    if snapshot.battle.is_some()
        && (runtime_shell.ball_cursor.is_some() || runtime_shell.bag_cursor.is_some())
    {
        reset_visible_battle_item_cursors(runtime_shell);
        runtime_shell
            .last_audio_events
            .push("closed battle item cursor".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if snapshot.battle.is_some()
        && (runtime_shell.battle_move_cursor.is_some()
            || runtime_shell.battle_switch_cursor.is_some())
        && !snapshot
            .battle
            .as_ref()
            .is_some_and(|battle| battle.commands.can_run)
    {
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_switch_cursor = None;
        runtime_shell
            .last_audio_events
            .push("reset battle action cursor".to_string());
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if snapshot
        .battle
        .as_ref()
        .is_some_and(|battle| battle.commands.can_run)
    {
        return press_visible_battle_b_button(runtime_shell);
    }
    Ok(())
}

fn has_visible_shell_b_action(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| {
            snapshot.ui.pending_yes_no.is_some()
                || snapshot.ui.pending_text_wait.is_some()
                || snapshot.pending_shop.is_some()
                || snapshot.ui.text_window_open
                || snapshot.ui.window_open
                || snapshot.ui.menu.is_some()
                || snapshot.ui.active_pokemon_picture.is_some()
                || (snapshot.battle.is_some()
                    && (runtime_shell.ball_cursor.is_some() || runtime_shell.bag_cursor.is_some()))
                || (snapshot.battle.is_some()
                    && (runtime_shell.battle_move_cursor.is_some()
                        || runtime_shell.battle_switch_cursor.is_some()))
                || snapshot
                    .battle
                    .as_ref()
                    .is_some_and(|battle| battle.commands.can_run)
        })
        .unwrap_or(false)
}

fn has_visible_shell_direction_action(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| {
            snapshot.pending_shop.is_some()
                || snapshot.ui.menu.is_some()
                || snapshot.battle.is_some()
                || !snapshot.ui.elevators.is_empty()
                || !snapshot.ui.gift_pokemon.is_empty()
        })
        .unwrap_or(false)
}

fn advance_visible_pending_text_wait(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let advance = runtime_shell.shell.advance_pending_text_wait()?;
    runtime_shell
        .last_audio_events
        .push(format!("advanced text wait {:?}", advance.state_checksum));
    Ok(())
}

fn accept_visible_pending_yes_no(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    resolve_visible_pending_yes_no(runtime_shell, true)
}

fn decline_visible_pending_yes_no(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    resolve_visible_pending_yes_no(runtime_shell, false)
}

fn resolve_visible_pending_yes_no(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    let resolution = runtime_shell.shell.resolve_pending_yes_no(accepted)?;
    runtime_shell.last_audio_events.push(format!(
        "yes/no accepted={} script_value={} checksum={:?}",
        resolution.accepted, resolution.script_value, resolution.state_checksum
    ));
    Ok(())
}

fn advance_visible_next_pending_script_request(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<bool> {
    if snapshot.script_events.pending_text_label.is_some() {
        advance_visible_text_label(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_script_warp.is_some() {
        execute_visible_pending_script_warp(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_map_load.is_some() {
        take_visible_pending_map_load(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_map_refresh.is_some() {
        take_visible_pending_map_refresh(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_music_fade.is_some() {
        take_visible_pending_music_fade(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.script_events.pending_screen_fade.is_some() {
        take_visible_pending_screen_fade(runtime_shell)?;
        return Ok(true);
    }
    if !snapshot.script_events.pending_delays.is_empty() {
        drain_visible_delays(runtime_shell)?;
        return Ok(true);
    }
    if !snapshot.script_events.pending_earthquakes.is_empty() {
        drain_visible_earthquakes(runtime_shell)?;
        return Ok(true);
    }
    if !snapshot.script_events.pending_emotes.is_empty() {
        drain_visible_emotes(runtime_shell)?;
        return Ok(true);
    }
    if snapshot.pending_shop.is_some() {
        take_visible_pending_shop_request(runtime_shell)?;
        return Ok(true);
    }
    Ok(false)
}

fn move_visible_primary_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_primary_cursor(runtime_shell, -1)
}

fn move_visible_primary_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_primary_cursor(runtime_shell, 1)
}

fn move_visible_primary_cursor_left(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_some() {
        return move_visible_battle_action_cursor(runtime_shell, -1);
    }
    move_visible_primary_cursor(runtime_shell, -1)
}

fn move_visible_primary_cursor_right(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_some() {
        return move_visible_battle_action_cursor(runtime_shell, 1);
    }
    move_visible_primary_cursor(runtime_shell, 1)
}

fn move_visible_primary_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(battle) = &snapshot.battle {
        if battle.commands.can_use_items {
            if runtime_shell.ball_cursor.is_some() {
                return move_visible_ball_cursor(runtime_shell, delta);
            }
            if runtime_shell.bag_cursor.is_some() {
                if selected_visible_battle_bag_menu(runtime_shell)?.as_deref()
                    == Some("ITEMMENU_PARTY")
                {
                    if selected_visible_battle_bag_item_targets_move(runtime_shell)? {
                        return move_visible_party_move_cursor(runtime_shell, delta);
                    }
                    return move_visible_party_cursor(runtime_shell, delta);
                }
                return move_visible_battle_bag_cursor(runtime_shell, delta);
            }
        }
        match selected_visible_battle_action_id_readonly(runtime_shell, &snapshot, battle)? {
            "Fight" => return move_visible_battle_move_cursor(runtime_shell, delta),
            "Pokemon" => return move_visible_battle_switch_cursor(runtime_shell, delta),
            _ => return move_visible_battle_action_cursor(runtime_shell, delta),
        }
    }
    if snapshot.pending_shop.is_some() || snapshot.ui.menu.is_some() {
        return move_visible_menu_cursor(runtime_shell, delta);
    }
    if let Some(option_count) = visible_script_surface_option_count(&snapshot) {
        shift_visible_script_command_cursor_bounded(runtime_shell, delta, option_count);
    }
    Ok(())
}

fn visible_script_surface_option_count(snapshot: &RuntimeShellSnapshot) -> Option<usize> {
    if !snapshot.ui.elevators.is_empty() {
        let mut option_count = snapshot.ui.elevators.len();
        for elevator in &snapshot.ui.elevators {
            if !elevator.floors.is_empty() {
                option_count = lcm_usize(option_count, elevator.floors.len());
            }
        }
        return Some(option_count.max(1));
    }
    if !snapshot.ui.gift_pokemon.is_empty() {
        return Some(snapshot.ui.gift_pokemon.len());
    }
    None
}

fn move_visible_sell_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_sell_cursor(runtime_shell, -1)
}

fn move_visible_sell_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_sell_cursor(runtime_shell, 1)
}

fn move_visible_party_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_cursor(runtime_shell, -1)
}

fn move_visible_party_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_cursor(runtime_shell, 1)
}

fn move_visible_bag_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_bag_cursor(runtime_shell, -1)
}

fn move_visible_bag_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_bag_cursor(runtime_shell, 1)
}

fn move_visible_ball_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_ball_cursor(runtime_shell, -1)
}

fn move_visible_ball_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_ball_cursor(runtime_shell, 1)
}

fn move_visible_tmhm_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_tmhm_cursor(runtime_shell, -1)
}

fn move_visible_tmhm_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_tmhm_cursor(runtime_shell, 1)
}

fn move_visible_storage_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_storage_cursor(runtime_shell, -1)
}

fn move_visible_storage_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_storage_cursor(runtime_shell, 1)
}

fn move_visible_pc_item_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_pc_item_cursor(runtime_shell, -1)
}

fn move_visible_pc_item_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_pc_item_cursor(runtime_shell, 1)
}

fn move_visible_fly_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_fly_cursor(runtime_shell, -1)
}

fn move_visible_fly_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_fly_cursor(runtime_shell, 1)
}

fn move_visible_battle_move_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_battle_move_cursor(runtime_shell, -1)
}

fn move_visible_battle_move_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_battle_move_cursor(runtime_shell, 1)
}

fn move_visible_battle_switch_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    move_visible_cursor_slot(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch".to_string(),
        battle.commands.switch_party_indices.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_battle_action_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        anyhow::bail!("no active battle");
    };
    let actions = visible_battle_action_ids(&snapshot, battle);
    if actions.is_empty() {
        anyhow::bail!("active battle has no available player action");
    }
    move_visible_cursor_slot(
        &mut runtime_shell.battle_action_cursor,
        "battle:actions".to_string(),
        actions.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_party_move_cursor_up(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_move_cursor(runtime_shell, -1)
}

fn move_visible_party_move_cursor_down(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    move_visible_party_move_cursor(runtime_shell, 1)
}

fn move_visible_party_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        anyhow::bail!("party is empty");
    }
    let current = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len() - 1);
    let next = if delta.is_negative() {
        current
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(snapshot.party.slots.len() - 1)
    } else {
        (current + delta as usize) % snapshot.party.slots.len()
    };
    runtime_shell.party_cursor = next;
    runtime_shell
        .last_audio_events
        .push(format!("party cursor {}->{}", current + 1, next + 1));
    Ok(())
}

fn selected_party_index(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.party.slots.is_empty() {
        anyhow::bail!("party is empty");
    }
    if runtime_shell.party_cursor >= snapshot.party.slots.len() {
        runtime_shell.party_cursor = 0;
    }
    Ok(snapshot.party.slots[runtime_shell.party_cursor].index)
}

fn selected_party_move_slot(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if slot.pokemon.moves.is_empty() {
        anyhow::bail!("selected party index {party_index} has no moves");
    }
    Ok(visible_cursor_index(
        &mut runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(party_index),
        slot.pokemon.moves.len(),
    ))
}

fn selected_pokedex_species_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot.party.slots.is_empty() {
        let party_index = selected_party_index(runtime_shell)?;
        return snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.index == party_index)
            .map(|slot| slot.pokemon.species.id.clone())
            .with_context(|| format!("selected party index {party_index} is not in the party"));
    }
    if snapshot.pokemon.is_empty() {
        anyhow::bail!("compiled pack has no Pokemon species");
    }
    let selected_index = runtime_shell.script_command_cursor % snapshot.pokemon.len();
    Ok(snapshot.pokemon[selected_index].species_id.clone())
}

fn move_visible_bag_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    move_visible_cursor_slot(
        &mut runtime_shell.bag_cursor,
        "bag:items".to_string(),
        snapshot
            .bag
            .items
            .iter()
            .filter(|item| item.quantity > 0)
            .count(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_battle_bag_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = carried_battle_usable_item_ids(&snapshot);
    if item_ids.is_empty() {
        anyhow::bail!("bag item pocket has no carried battle-usable item");
    }
    move_visible_cursor_slot(
        &mut runtime_shell.bag_cursor,
        "battle:bag-items".to_string(),
        item_ids.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_ball_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    move_visible_cursor_slot(
        &mut runtime_shell.ball_cursor,
        "bag:balls".to_string(),
        snapshot
            .bag
            .balls
            .iter()
            .filter(|item| item.quantity > 0)
            .count(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_tmhm_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    move_visible_cursor_slot(
        &mut runtime_shell.tmhm_cursor,
        "bag:tmhm".to_string(),
        snapshot
            .bag
            .tm_hm
            .iter()
            .count(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_storage_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_box = current_storage_box(&snapshot)?;
    move_visible_cursor_slot(
        &mut runtime_shell.storage_cursor,
        storage_cursor_surface_id(snapshot.storage.current_pc_box),
        current_box.slots.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_pc_item_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    move_visible_cursor_slot(
        &mut runtime_shell.pc_item_cursor,
        "pc:items".to_string(),
        snapshot
            .bag
            .pc_items
            .iter()
            .filter(|item| item.quantity > 0)
            .count(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_fly_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    move_visible_cursor_slot(
        &mut runtime_shell.fly_cursor,
        "fly:destinations".to_string(),
        active_fly_destination_count(&snapshot),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_battle_move_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    move_visible_cursor_slot(
        &mut runtime_shell.battle_move_cursor,
        "battle:moves".to_string(),
        battle.commands.player_move_slots.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_party_move_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    move_visible_cursor_slot(
        &mut runtime_shell.party_move_cursor,
        party_move_cursor_surface_id(party_index),
        slot.pokemon.moves.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn selected_current_box_slot_index(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_box = current_storage_box(&snapshot)?;
    if current_box.slots.is_empty() {
        anyhow::bail!(
            "current PC box {} has no Pokemon",
            snapshot.storage.current_pc_box
        );
    }
    let surface_id = storage_cursor_surface_id(snapshot.storage.current_pc_box);
    let slot_offset = visible_cursor_index(
        &mut runtime_shell.storage_cursor,
        &surface_id,
        current_box.slots.len(),
    );
    Ok(current_box.slots[slot_offset].index)
}

fn current_storage_box(snapshot: &RuntimeShellSnapshot) -> Result<&crate::RuntimePcBoxSnapshot> {
    snapshot
        .storage
        .boxes
        .iter()
        .find(|pc_box| pc_box.index == snapshot.storage.current_pc_box)
        .with_context(|| {
            format!(
                "current PC box {} is missing from storage snapshot",
                snapshot.storage.current_pc_box
            )
        })
}

fn storage_cursor_surface_id(box_index: usize) -> String {
    format!("pc:box:{box_index}")
}

fn party_move_cursor_surface_id(party_index: usize) -> String {
    format!("party:{party_index}:moves")
}

fn move_visible_sell_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_none() {
        return Ok(());
    }
    let sellable = sellable_carried_item_ids(&snapshot);
    move_visible_cursor_slot(
        &mut runtime_shell.sell_cursor,
        "sell:bag".to_string(),
        sellable.len(),
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_menu_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(shop) = &snapshot.pending_shop {
        return move_visible_cursor_for_surface(
            runtime_shell,
            shop_cursor_surface_id(shop),
            shop.inventory.len(),
            delta,
        );
    }
    if snapshot.ui.menu.is_none() {
        return Ok(());
    }
    let menu_target = active_menu_target(&snapshot, &runtime_shell.menu_cursor)?;
    move_visible_cursor_for_surface(
        runtime_shell,
        menu_target.surface_id,
        menu_target.option_count,
        delta,
    )
}

fn move_visible_cursor_for_surface(
    runtime_shell: &mut BevyRuntimeShell,
    surface_id: String,
    option_count: usize,
    delta: isize,
) -> Result<()> {
    move_visible_cursor_slot(
        &mut runtime_shell.menu_cursor,
        surface_id,
        option_count,
        delta,
        &mut runtime_shell.last_audio_events,
    )
}

fn move_visible_cursor_slot(
    cursor_slot: &mut Option<MenuCursor>,
    surface_id: String,
    option_count: usize,
    delta: isize,
    event_log: &mut Vec<String>,
) -> Result<()> {
    if option_count == 0 {
        anyhow::bail!("{surface_id} has no selectable options");
    }
    let current = visible_cursor_index(cursor_slot, &surface_id, option_count);
    let next = if delta.is_negative() {
        current
            .checked_sub(delta.unsigned_abs())
            .unwrap_or(option_count - 1)
    } else {
        (current + delta as usize) % option_count
    };
    *cursor_slot = Some(MenuCursor {
        surface_id,
        option_index: next,
    });
    event_log.push(format!("cursor {}->{}", current + 1, next + 1));
    Ok(())
}

fn select_visible_menu_cursor_option(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let menu_target = active_menu_target(&snapshot, &runtime_shell.menu_cursor)?;
    let option_index = visible_cursor_index(
        &mut runtime_shell.menu_cursor,
        &menu_target.surface_id,
        menu_target.option_count,
    );
    select_visible_menu_option(runtime_shell, option_index)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveMenuTarget {
    surface_id: String,
    option_count: usize,
}

fn active_menu_target(
    snapshot: &RuntimeShellSnapshot,
    cursor: &Option<MenuCursor>,
) -> Result<ActiveMenuTarget> {
    let Some(menu) = &snapshot.ui.menu else {
        anyhow::bail!("no active menu");
    };
    if let Some(cursor) = cursor {
        if let Some(vertical) = menu
            .layout
            .vertical_menus
            .iter()
            .find(|vertical| vertical_menu_surface_id(menu, vertical) == cursor.surface_id)
        {
            if vertical.options.is_empty() {
                anyhow::bail!("menu {} vertical menu has no options", menu.menu_id);
            }
            return Ok(ActiveMenuTarget {
                surface_id: cursor.surface_id.clone(),
                option_count: vertical.options.len(),
            });
        }
    }
    let vertical = first_selectable_vertical_menu(menu)?;
    Ok(ActiveMenuTarget {
        surface_id: vertical_menu_surface_id(menu, vertical),
        option_count: vertical.options.len(),
    })
}

fn first_selectable_vertical_menu<'a>(
    menu: &'a crate::RuntimeMenuSnapshot,
) -> Result<&'a crate::RuntimeVerticalMenuSnapshot> {
    menu.layout
        .vertical_menus
        .iter()
        .find(|vertical| !vertical.options.is_empty())
        .with_context(|| format!("menu {} has no selectable options", menu.menu_id))
}

fn selected_vertical_menu<'a>(
    menu: &'a crate::RuntimeMenuSnapshot,
    cursor: &Option<MenuCursor>,
) -> Result<&'a crate::RuntimeVerticalMenuSnapshot> {
    if let Some(cursor) = cursor {
        if let Some(vertical) = menu
            .layout
            .vertical_menus
            .iter()
            .find(|vertical| vertical_menu_surface_id(menu, vertical) == cursor.surface_id)
        {
            return Ok(vertical);
        }
    }
    first_selectable_vertical_menu(menu)
}

fn vertical_menu_surface_id(
    menu: &crate::RuntimeMenuSnapshot,
    vertical: &crate::RuntimeVerticalMenuSnapshot,
) -> String {
    format!(
        "{}:{}:{}",
        menu.menu_id, vertical.source_script, vertical.verticalmenu_command_index
    )
}

fn visible_cursor_index(
    cursor_slot: &mut Option<MenuCursor>,
    surface_id: &str,
    option_count: usize,
) -> usize {
    match cursor_slot {
        Some(cursor) if cursor.surface_id == surface_id && cursor.option_index < option_count => {
            cursor.option_index
        }
        _ => {
            *cursor_slot = Some(MenuCursor {
                surface_id: surface_id.to_string(),
                option_index: 0,
            });
            0
        }
    }
}

fn visible_local_link_descriptor(
    runtime_shell: &mut BevyRuntimeShell,
    session_id: String,
) -> Result<RuntimeLinkSessionDescriptor> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.trainer.player_id == 0 {
        anyhow::bail!("trainer player_id 0 is not valid for link identity");
    }
    runtime_shell.shell.link_session_descriptor(
        session_id,
        u64::from(snapshot.trainer.player_id),
        snapshot.trainer.player_name,
    )
}

fn explicit_script_runtime_inputs(
    runtime_shell: &BevyRuntimeShell,
    command: &str,
    args: &[String],
    command_index: usize,
) -> Result<ScriptRuntimeInputs> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let random_value = if command == "random" {
        let bound_token = args
            .first()
            .with_context(|| "compiled random command is missing bound argument")?;
        let bound = bound_token
            .parse::<u32>()
            .with_context(|| format!("compiled random bound '{bound_token}' is not a u32"))?;
        if bound == 0 {
            anyhow::bail!("compiled random command has zero bound");
        }
        Some((snapshot.state_checksum.hash() ^ command_index as u32) % bound)
    } else {
        None
    };
    let game_version = if command == "checkver" {
        Some(runtime_shell.shell.runtime().modpack().id().to_string())
    } else {
        None
    };
    Ok(ScriptRuntimeInputs {
        random_value,
        game_version,
    })
}

fn explicit_compiled_script_runtime_inputs(
    runtime_shell: &BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> Result<ScriptRuntimeInputs> {
    if let Some(command) = runtime_shell
        .shell
        .script_runtime_command_keys()
        .into_iter()
        .find(|command| {
            command.source_script == source_script && command.command_index == command_index
        })
    {
        explicit_script_runtime_inputs(
            runtime_shell,
            &command.command,
            &command.args,
            command.command_index,
        )
    } else {
        Ok(ScriptRuntimeInputs::default())
    }
}

fn explicit_compiled_script_phone_inputs(
    runtime_shell: &BevyRuntimeShell,
    source_script: &str,
    command_index: usize,
) -> ScriptPhoneInputs {
    runtime_shell
        .shell
        .script_phone_command_keys()
        .into_iter()
        .find(|command| {
            command.source_script == source_script && command.command_index == command_index
        })
        .map(|command| ScriptPhoneInputs {
            accepted: (command.command == "askforphonenumber").then_some(true),
        })
        .unwrap_or(ScriptPhoneInputs { accepted: None })
}

fn close_active_runtime_surface(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.text_window_open {
        return close_visible_text_window(runtime_shell);
    }
    if snapshot.ui.window_open {
        let close = runtime_shell.shell.close_runtime_window()?;
        runtime_shell
            .last_audio_events
            .push(format!("closed runtime window {:?}", close.state_checksum));
        return Ok(());
    }
    if snapshot.ui.menu.is_some() {
        let close = runtime_shell.shell.close_active_menu()?;
        reset_visible_selection_cursors(runtime_shell);
        runtime_shell.last_audio_events.push(format!(
            "closed menu {} {:?}",
            close.menu, close.state_checksum
        ));
        return Ok(());
    }
    if snapshot.ui.active_pokemon_picture.is_some() {
        return close_visible_pokemon_picture(runtime_shell);
    }
    if snapshot.pending_shop.is_some() {
        return close_visible_shop(runtime_shell);
    }
    anyhow::bail!("no active runtime surface to close")
}

fn close_visible_text_window(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let close = runtime_shell.shell.close_text_window()?;
    runtime_shell
        .last_audio_events
        .push(format!("closed text window {:?}", close.state_checksum));
    Ok(())
}

fn close_visible_pokemon_picture(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let close = runtime_shell.shell.close_active_pokemon_picture()?;
    runtime_shell.last_audio_events.push(format!(
        "closed pokemon picture {} {:?}",
        close.species_id, close.state_checksum
    ));
    Ok(())
}

fn select_visible_menu_option(
    runtime_shell: &mut BevyRuntimeShell,
    option_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(menu) = snapshot.ui.menu else {
        anyhow::bail!("no active menu");
    };
    let vertical = selected_vertical_menu(&menu, &runtime_shell.menu_cursor)?;
    let option = vertical
        .options
        .get(option_index)
        .with_context(|| format!("menu {} has no option index {}", menu.menu_id, option_index))?;
    let selection = runtime_shell.shell.select_vertical_menu_option(
        menu.menu_id.clone(),
        vertical.source_script.clone(),
        vertical.verticalmenu_command_index,
        option_index,
        option.clone(),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "selected menu option {}={} script_value={} checksum={:?}",
        selection.option_index, selection.option, selection.script_value, selection.state_checksum
    ));
    Ok(())
}

fn select_visible_linked_menu_option(
    runtime_shell: &mut BevyRuntimeShell,
    option_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(menu) = snapshot.ui.menu else {
        anyhow::bail!("no active menu");
    };
    let vertical = selected_vertical_menu(&menu, &runtime_shell.menu_cursor)?;
    let option = vertical.options.get(option_index).with_context(|| {
        format!(
            "linked menu {} has no option index {}",
            menu.menu_id, option_index
        )
    })?;
    let session_id = format!(
        "bevy-local-menu-{}-{}",
        snapshot.state_checksum.frame(),
        option_index
    );
    let descriptor = visible_local_link_descriptor(runtime_shell, session_id.clone())?;
    let choice = runtime_shell.shell.select_linked_vertical_menu_option(
        &descriptor,
        menu.menu_id.clone(),
        vertical.source_script.clone(),
        vertical.verticalmenu_command_index,
        option_index,
        option.clone(),
    )?;
    let result = runtime_shell
        .shell
        .record_linked_menu_choice_result(&descriptor, &choice)?;
    runtime_shell.last_audio_events.push(format!(
        "linked menu session={} player={} option {}={} script_value={} choice_frame={} result_frame={} checksum={:?}",
        session_id,
        descriptor.local_player.id(),
        choice.selection.option_index,
        choice.selection.option,
        choice.selection.script_value,
        choice.frame.frame(),
        result.checksum().frame(),
        choice.selection.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn advance_visible_text_label(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let label = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::TextLabel)?;
    runtime_shell
        .last_audio_events
        .push(format!("advanced text label {:?}", label));
    Ok(())
}

fn take_visible_pending_map_load(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapLoad)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending map load {:?}", request));
    arm_visible_current_scene_script(runtime_shell, "map_load")?;
    arm_visible_current_map_callbacks(runtime_shell, "map_load")?;
    Ok(())
}

fn take_visible_pending_map_refresh(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MapRefresh)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending map refresh {:?}", request));
    runtime_shell.pending_scene_script = None;
    arm_visible_current_map_callbacks(runtime_shell, "map_refresh")?;
    Ok(())
}

fn arm_visible_current_scene_script(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    let Some(scene) = runtime_shell.shell.current_scene_script()? else {
        runtime_shell.pending_scene_script = None;
        runtime_shell
            .last_audio_events
            .push(format!("scene script none reason={reason}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    runtime_shell.pending_scene_script = scene.script_name.clone();
    runtime_shell.last_audio_events.push(format!(
        "scene armed map={} scene={} script={:?} reason={}",
        scene.map_name, scene.scene_id, scene.script_name, reason
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn arm_visible_current_map_callbacks(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let map_name = snapshot.overworld.map_name;
    let mut callbacks = runtime_shell
        .shell
        .map_script_section_command_keys()
        .into_iter()
        .filter(|key| key.map_name == map_name && key.command == "callback")
        .collect::<Vec<_>>();
    callbacks.sort_by_key(|key| key.command_index);
    let mut scripts = Vec::with_capacity(callbacks.len());
    for callback in callbacks {
        if callback.args.len() != 2 {
            anyhow::bail!(
                "compiled map callback {}:{} has {} args, expected 2",
                callback.map_name,
                callback.command_index,
                callback.args.len()
            );
        }
        scripts.push(callback.args[1].clone());
    }
    runtime_shell.pending_map_callbacks = scripts;
    if runtime_shell.pending_map_callbacks.is_empty() {
        runtime_shell
            .last_audio_events
            .push(format!("map callbacks none map={map_name} reason={reason}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return take_visible_pending_scene_script(runtime_shell);
    }
    runtime_shell.last_audio_events.push(format!(
        "map callbacks armed map={} reason={} count={}",
        map_name,
        reason,
        runtime_shell.pending_map_callbacks.len()
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    take_next_visible_map_callback(runtime_shell)
}

fn take_next_visible_map_callback(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    if runtime_shell.pending_map_callbacks.is_empty() {
        return take_visible_pending_scene_script(runtime_shell);
    }
    let script = runtime_shell.pending_map_callbacks.remove(0);
    runtime_shell.last_audio_events.push(format!(
        "map callback script={} remaining={}",
        script,
        runtime_shell.pending_map_callbacks.len()
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    reset_visible_selection_cursors(runtime_shell);
    if !has_visible_compiled_script_command(runtime_shell, &script, 0) {
        runtime_shell
            .last_audio_events
            .push(format!("script complete={script}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return take_next_visible_map_callback(runtime_shell);
    }
    start_visible_script_entry(runtime_shell, &script)
}

fn take_visible_pending_scene_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(script) = runtime_shell.pending_scene_script.take() else {
        return Ok(());
    };
    runtime_shell
        .last_audio_events
        .push(format!("scene script={script}"));
    trim_event_log(&mut runtime_shell.last_audio_events);
    reset_visible_selection_cursors(runtime_shell);
    start_visible_script_entry(runtime_shell, &script)
}

fn take_visible_pending_music_fade(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::MusicFade)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending music fade {:?}", request));
    Ok(())
}

fn take_visible_pending_screen_fade(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::ScreenFade)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending screen fade {:?}", request));
    Ok(())
}

fn take_visible_pending_shop_request(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let request = runtime_shell
        .shell
        .take_pending_script_request(RuntimePendingScriptRequestKind::Shop)?;
    runtime_shell
        .last_audio_events
        .push(format!("took pending shop {:?}", request));
    Ok(())
}

fn clear_visible_menu_coords(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let clear = runtime_shell.shell.clear_menu_coords()?;
    runtime_shell.last_audio_events.push(format!(
        "cleared menu coords {:?} checksum={:?}",
        clear.coords, clear.state_checksum
    ));
    Ok(())
}

fn select_visible_elevator_floor(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.elevators.is_empty() {
        anyhow::bail!("no compiled elevator is visible");
    }
    let elevator_index = runtime_shell.script_command_cursor % snapshot.ui.elevators.len();
    let elevator = &snapshot.ui.elevators[elevator_index];
    if elevator.floors.is_empty() {
        anyhow::bail!("elevator {} has no floors", elevator.data_label);
    }
    let floor_index = runtime_shell.script_command_cursor % elevator.floors.len();
    let floor = &elevator.floors[floor_index];
    let selection = runtime_shell.shell.select_elevator_floor(
        elevator.data_label.clone(),
        elevator.source_script.clone(),
        elevator.elevator_command_index,
        floor.floor_index,
        floor.floor.clone(),
        floor.warp,
        floor.target_map.clone(),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "selected elevator {}/{} floor={}/{} {} target={} script_value={} checksum={:?}",
        elevator_index + 1,
        snapshot.ui.elevators.len(),
        floor_index + 1,
        elevator.floors.len(),
        selection.floor,
        selection.target_map,
        selection.script_value,
        selection.state_checksum
    ));
    Ok(())
}

fn drain_visible_audio_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drain = runtime_shell.shell.drain_resolved_audio_events()?;
    runtime_shell.last_audio_events.push(format!(
        "drained resolved audio events={} checksum={:?}",
        drain.events.len(),
        drain.state_checksum
    ));
    for event in drain.events {
        runtime_shell.last_audio_events.push(format!(
            "audio event {:?} resolved={:?}",
            event.event, event.kind
        ));
    }
    Ok(())
}

fn drain_visible_map_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_event_queue(RuntimeScriptEventQueue::Map)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained map events {:?}", drained));
    Ok(())
}

fn drain_visible_text_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_event_queue(RuntimeScriptEventQueue::Text)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained text events {:?}", drained));
    Ok(())
}

fn drain_visible_misc_script_events(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    for queue in [
        RuntimeScriptEventQueue::Graphics,
        RuntimeScriptEventQueue::Money,
        RuntimeScriptEventQueue::Control,
        RuntimeScriptEventQueue::Shop,
        RuntimeScriptEventQueue::ItemUse,
    ] {
        let drained = runtime_shell.shell.drain_script_event_queue(queue)?;
        runtime_shell
            .last_audio_events
            .push(format!("drained script event {:?}: {:?}", queue, drained));
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn drain_visible_delays(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingDelay)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained runtime delays {:?}", drained));
    Ok(())
}

fn drain_visible_earthquakes(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingEarthquake)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained runtime earthquakes {:?}", drained));
    Ok(())
}

fn drain_visible_emotes(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let drained = runtime_shell
        .shell
        .drain_script_runtime_queue(RuntimeScriptRuntimeQueue::PendingEmote)?;
    runtime_shell
        .last_audio_events
        .push(format!("drained runtime emotes {:?}", drained));
    Ok(())
}

fn drain_visible_misc_runtime_queues(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    for queue in [
        RuntimeScriptRuntimeQueue::PendingEarthquake,
        RuntimeScriptRuntimeQueue::PendingEmote,
        RuntimeScriptRuntimeQueue::Command,
        RuntimeScriptRuntimeQueue::Stack,
        RuntimeScriptRuntimeQueue::CallStack,
        RuntimeScriptRuntimeQueue::DeferredScript,
    ] {
        let drained = runtime_shell.shell.drain_script_runtime_queue(queue)?;
        runtime_shell
            .last_audio_events
            .push(format!("drained runtime queue {:?}: {:?}", queue, drained));
    }
    let linked_menu_results = runtime_shell.shell.drain_linked_menu_results();
    if !linked_menu_results.is_empty() {
        runtime_shell.last_audio_events.push(format!(
            "drained linked menu results {linked_menu_results:?}"
        ));
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn drain_visible_runtime_records(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    for queue in [
        RuntimeScriptRuntimeRecordQueue::VariableWrite,
        RuntimeScriptRuntimeRecordQueue::Effect,
        RuntimeScriptRuntimeRecordQueue::AsmDirective,
        RuntimeScriptRuntimeRecordQueue::NumericBufferWrite,
        RuntimeScriptRuntimeRecordQueue::ElevatorFloor,
        RuntimeScriptRuntimeRecordQueue::StoneTableEntry,
        RuntimeScriptRuntimeRecordQueue::DecorationDescription,
        RuntimeScriptRuntimeRecordQueue::SpecialPhoneCall,
        RuntimeScriptRuntimeRecordQueue::CompletedTrade,
        RuntimeScriptRuntimeRecordQueue::CatchTutorial,
        RuntimeScriptRuntimeRecordQueue::CheckedMailTarget,
        RuntimeScriptRuntimeRecordQueue::GivenMailTarget,
    ] {
        let drained = runtime_shell
            .shell
            .drain_script_runtime_record_queue(queue)?;
        runtime_shell
            .last_audio_events
            .push(format!("drained runtime record {:?}: {:?}", queue, drained));
    }
    Ok(())
}

fn consume_visible_runtime_flag(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let scripts = &snapshot.script_events;
    let flag = if scripts.map_music_restart_disabled {
        RuntimeScriptRuntimeFlag::MapMusicRestartDisabled
    } else if scripts.map_music_requested {
        RuntimeScriptRuntimeFlag::MapMusicRequested
    } else if scripts.waiting_for_sound_effect {
        RuntimeScriptRuntimeFlag::WaitingForSoundEffect
    } else if scripts.warp_check_requested {
        RuntimeScriptRuntimeFlag::WarpCheckRequested
    } else if scripts.item_notify_queued {
        RuntimeScriptRuntimeFlag::ItemNotifyQueued
    } else if scripts.warp_sound_queued {
        RuntimeScriptRuntimeFlag::WarpSoundQueued
    } else if scripts.teleport_from_queued {
        RuntimeScriptRuntimeFlag::TeleportFromQueued
    } else if scripts.hall_of_fame_requested {
        RuntimeScriptRuntimeFlag::HallOfFameRequested
    } else if scripts.credits_requested {
        RuntimeScriptRuntimeFlag::CreditsRequested
    } else if scripts.reset_requested {
        RuntimeScriptRuntimeFlag::ResetRequested
    } else if scripts.menu_2d_requested {
        RuntimeScriptRuntimeFlag::Menu2dRequested
    } else if scripts.version_check_requested {
        RuntimeScriptRuntimeFlag::VersionCheckRequested
    } else if scripts.blackout_mod.is_some() {
        RuntimeScriptRuntimeFlag::BlackoutMod
    } else if scripts.battle_tower_text.is_some() {
        RuntimeScriptRuntimeFlag::BattleTowerText
    } else {
        anyhow::bail!("no active script runtime flag");
    };
    consume_visible_runtime_flag_kind(runtime_shell, flag)
}

fn visible_auto_runtime_flag(snapshot: &RuntimeShellSnapshot) -> Option<RuntimeScriptRuntimeFlag> {
    let scripts = &snapshot.script_events;
    if scripts.map_music_restart_disabled {
        Some(RuntimeScriptRuntimeFlag::MapMusicRestartDisabled)
    } else if scripts.map_music_requested {
        Some(RuntimeScriptRuntimeFlag::MapMusicRequested)
    } else if scripts.waiting_for_sound_effect {
        Some(RuntimeScriptRuntimeFlag::WaitingForSoundEffect)
    } else if scripts.warp_check_requested {
        Some(RuntimeScriptRuntimeFlag::WarpCheckRequested)
    } else if scripts.item_notify_queued {
        Some(RuntimeScriptRuntimeFlag::ItemNotifyQueued)
    } else if scripts.warp_sound_queued {
        Some(RuntimeScriptRuntimeFlag::WarpSoundQueued)
    } else if scripts.teleport_from_queued {
        Some(RuntimeScriptRuntimeFlag::TeleportFromQueued)
    } else if scripts.hall_of_fame_requested {
        Some(RuntimeScriptRuntimeFlag::HallOfFameRequested)
    } else if scripts.credits_requested {
        Some(RuntimeScriptRuntimeFlag::CreditsRequested)
    } else if scripts.reset_requested {
        Some(RuntimeScriptRuntimeFlag::ResetRequested)
    } else if scripts.menu_2d_requested {
        Some(RuntimeScriptRuntimeFlag::Menu2dRequested)
    } else if scripts.version_check_requested {
        Some(RuntimeScriptRuntimeFlag::VersionCheckRequested)
    } else if scripts.battle_tower_text.is_some() {
        Some(RuntimeScriptRuntimeFlag::BattleTowerText)
    } else {
        None
    }
}

fn consume_visible_runtime_flag_kind(
    runtime_shell: &mut BevyRuntimeShell,
    flag: RuntimeScriptRuntimeFlag,
) -> Result<()> {
    let consumed = runtime_shell.shell.consume_script_runtime_flag(flag)?;
    runtime_shell
        .last_audio_events
        .push(format!("consumed runtime flag {:?}", consumed));
    Ok(())
}

fn take_visible_script_value(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    take_visible_runtime_memory_value(runtime_shell, RuntimeScriptRuntimeMemoryValue::ScriptValue)
}

fn take_visible_last_special_routine(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    take_visible_runtime_memory_value(
        runtime_shell,
        RuntimeScriptRuntimeMemoryValue::LastSpecialRoutine,
    )
}

fn take_visible_last_talked_object(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    take_visible_runtime_memory_value(
        runtime_shell,
        RuntimeScriptRuntimeMemoryValue::LastTalkedObject,
    )
}

fn take_visible_runtime_memory_value(
    runtime_shell: &mut BevyRuntimeShell,
    value: RuntimeScriptRuntimeMemoryValue,
) -> Result<()> {
    let taken = runtime_shell
        .shell
        .take_script_runtime_memory_value(value)?;
    runtime_shell
        .last_audio_events
        .push(format!("took runtime memory {:?}: {:?}", value, taken));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn remove_selected_runtime_variable(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (_, _, key) = selected_btree_key(
        runtime_shell,
        "script runtime variable",
        &snapshot.script_events.variables,
    )?;
    remove_visible_runtime_memory_entry(
        runtime_shell,
        RuntimeScriptRuntimeMemoryEntry::Variable,
        key,
    )
}

fn remove_selected_runtime_memory(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (_, _, key) = selected_btree_key(
        runtime_shell,
        "script runtime memory entry",
        &snapshot.script_events.memory,
    )?;
    remove_visible_runtime_memory_entry(runtime_shell, RuntimeScriptRuntimeMemoryEntry::Memory, key)
}

fn remove_selected_named_buffer(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (_, _, key) = selected_btree_key(
        runtime_shell,
        "script runtime named buffer",
        &snapshot.script_events.named_buffers,
    )?;
    remove_visible_runtime_memory_entry(
        runtime_shell,
        RuntimeScriptRuntimeMemoryEntry::NamedBuffer,
        key,
    )
}

fn remove_visible_runtime_memory_entry(
    runtime_shell: &mut BevyRuntimeShell,
    entry: RuntimeScriptRuntimeMemoryEntry,
    key: String,
) -> Result<()> {
    let removed = runtime_shell
        .shell
        .remove_script_runtime_memory_entry(entry, key)?;
    runtime_shell
        .last_audio_events
        .push(format!("removed runtime memory {:?}: {:?}", entry, removed));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn set_visible_clock_morning(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    update_visible_clock(runtime_shell, 6, 0, 0, "morning")
}

fn set_visible_clock_day(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    update_visible_clock(runtime_shell, 12, 0, 0, "day")
}

fn set_visible_clock_night(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    update_visible_clock(runtime_shell, 22, 0, 0, "night")
}

fn set_visible_manual_clock_evening(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let update = runtime_shell.shell.set_manual_clock_time(
        GameDate::new(2000, 1, 1),
        6,
        0,
        0,
        ClockTime::new(0, 20, 30, 0),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "manual clock evening tod={:?} day={} game={}:{} checksum={:?}",
        update.time_of_day,
        update.day_of_week,
        update.game_time_hours,
        update.game_time_minutes,
        update.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn update_visible_clock(
    runtime_shell: &mut BevyRuntimeShell,
    hour: u8,
    minute: u8,
    second: u8,
    label: &str,
) -> Result<()> {
    let update = runtime_shell.shell.update_clock_from_datetime(
        GameDate::new(2000, 1, 2),
        hour,
        minute,
        second,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "clock {label} tod={:?} day={} game={}:{} checksum={:?}",
        update.time_of_day,
        update.day_of_week,
        update.game_time_hours,
        update.game_time_minutes,
        update.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn initialize_visible_phone_numbers(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let initialized = runtime_shell.shell.initialize_permanent_phone_numbers()?;
    runtime_shell.last_audio_events.push(format!(
        "initialized permanent phone numbers={} checksum={:?}",
        initialized.inserted.len(),
        initialized.state_checksum
    ));
    Ok(())
}

fn apply_selected_phone_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, command) = selected_script_command_key(
        runtime_shell,
        "phone",
        runtime_shell
            .shell
            .script_phone_command_keys()
            .into_iter()
            .filter(|command| command.map_name == current_map)
            .collect(),
    )?;
    let inputs = ScriptPhoneInputs {
        accepted: (command.command == "askforphonenumber").then_some(true),
    };
    let applied = runtime_shell.shell.apply_script_phone_command(
        &command.map_name,
        &command.source_script,
        command.command_index,
        inputs,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "phone command {}/{} {} contact={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        command.command,
        command.contact_id,
        applied.outcome,
        applied.state_checksum
    ));
    Ok(())
}

fn apply_selected_swarm_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, command) = selected_script_command_key(
        runtime_shell,
        "swarm",
        runtime_shell
            .shell
            .script_swarm_command_keys()
            .into_iter()
            .filter(|command| command.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_swarm_command(
        &command.map_name,
        &command.source_script,
        command.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "swarm command {}/{} {} token={} map_id={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        command.command,
        command.swarm_token,
        command.map_id,
        applied.outcome,
        applied.state_checksum
    ));
    Ok(())
}

fn shift_visible_script_command_cursor(runtime_shell: &mut BevyRuntimeShell, delta: isize) {
    let before = runtime_shell.script_command_cursor;
    runtime_shell.script_command_cursor = if delta.is_negative() {
        before.saturating_sub((-delta) as usize)
    } else {
        before.saturating_add(delta as usize)
    };
    runtime_shell.last_audio_events.push(format!(
        "script command cursor {}->{}",
        before, runtime_shell.script_command_cursor
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.last_error = None;
}

fn shift_visible_script_command_cursor_bounded(
    runtime_shell: &mut BevyRuntimeShell,
    delta: isize,
    option_count: usize,
) {
    if option_count == 0 {
        return;
    }
    let before = runtime_shell.script_command_cursor % option_count;
    let next = if delta.is_negative() {
        before
            .checked_sub(delta.unsigned_abs() % option_count)
            .unwrap_or_else(|| option_count - ((delta.unsigned_abs() - before) % option_count))
            % option_count
    } else {
        (before + (delta as usize % option_count)) % option_count
    };
    runtime_shell.script_command_cursor = next;
    runtime_shell.last_audio_events.push(format!(
        "script command cursor {}->{}",
        before, runtime_shell.script_command_cursor
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.last_error = None;
}

fn gcd_usize(mut left: usize, mut right: usize) -> usize {
    while right != 0 {
        let next = left % right;
        left = right;
        right = next;
    }
    left
}

fn lcm_usize(left: usize, right: usize) -> usize {
    if left == 0 || right == 0 {
        0
    } else {
        (left / gcd_usize(left, right)).saturating_mul(right)
    }
}

fn selected_script_command_key<T: Clone>(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    keys: Vec<T>,
) -> Result<(usize, usize, T)> {
    if keys.is_empty() {
        anyhow::bail!("current map has no compiled {family} command");
    }
    let selected_index = runtime_shell.script_command_cursor % keys.len();
    Ok((selected_index, keys.len(), keys[selected_index].clone()))
}

fn selected_btree_key<T>(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    keys: &std::collections::BTreeMap<String, T>,
) -> Result<(usize, usize, String)> {
    if keys.is_empty() {
        anyhow::bail!("compiled pack declares no {family}");
    }
    let selected_index = runtime_shell.script_command_cursor % keys.len();
    let key = keys
        .keys()
        .nth(selected_index)
        .with_context(|| {
            format!(
                "compiled {family} cursor selected index {selected_index} outside {} keys",
                keys.len()
            )
        })?
        .clone();
    Ok((selected_index, keys.len(), key))
}

fn selected_declared_special<T: Copy>(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    declared: &std::collections::BTreeMap<String, crystal_assets::SpecialRoutineRule>,
    candidates: &[T],
    routine: fn(T) -> &'static str,
) -> Result<(usize, usize, T)> {
    let visible = candidates
        .iter()
        .copied()
        .filter(|candidate| declared.contains_key(routine(*candidate)))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        anyhow::bail!("compiled pack declares no Bevy-visible {family} special");
    }
    let selected_index = runtime_shell.script_command_cursor % visible.len();
    Ok((selected_index, visible.len(), visible[selected_index]))
}

fn selected_declared_special_routine(
    runtime_shell: &BevyRuntimeShell,
    family: &str,
    declared: &std::collections::BTreeMap<String, crystal_assets::SpecialRoutineRule>,
    candidates: &[&'static str],
) -> Result<(usize, usize, &'static str)> {
    let visible = candidates
        .iter()
        .copied()
        .filter(|routine| declared.contains_key(*routine))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        anyhow::bail!("compiled pack declares no Bevy-visible {family} special");
    }
    let selected_index = runtime_shell.script_command_cursor % visible.len();
    Ok((selected_index, visible.len(), visible[selected_index]))
}

fn grant_selected_script_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script item grant",
        runtime_shell
            .shell
            .script_item_grant_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let granted = runtime_shell.shell.grant_script_item(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script item grant {}/{} command={} item={} quantity={} verbose={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        key.quantity,
        key.verbose,
        granted.outcome,
        granted.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn check_selected_script_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (selected_index, selected_len, key) =
        selected_script_item_access_key(runtime_shell, "checkitem")?;
    let checked = runtime_shell.shell.check_script_item(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script item check {}/{} command={} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        checked.outcome,
        checked.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn take_selected_script_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (selected_index, selected_len, key) =
        selected_script_item_access_key(runtime_shell, "takeitem")?;
    let taken = runtime_shell.shell.take_script_item(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script item take {}/{} command={} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        taken.outcome,
        taken.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn selected_script_item_access_key(
    runtime_shell: &mut BevyRuntimeShell,
    command: &str,
) -> Result<(usize, usize, crate::RuntimeScriptItemAccessKey)> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    selected_script_command_key(
        runtime_shell,
        command,
        runtime_shell
            .shell
            .script_item_access_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map && key.command == command)
            .collect(),
    )
}

fn apply_selected_script_economy_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script economy",
        runtime_shell
            .shell
            .script_economy_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_economy_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script economy {}/{} command={} account={:?} amount_tokens={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command, key.account, key.amount_tokens, applied.outcome, applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn pickup_selected_script_field_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script field pickup",
        runtime_shell
            .shell
            .script_field_pickup_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let pickup = runtime_shell.shell.pickup_script_field_item(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script field pickup {}/{} command={} item={:?} quantity={} event={:?} fruit_tree={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.item_id,
        key.quantity,
        key.event_flag,
        key.fruit_tree_id,
        pickup.outcome,
        pickup.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_flag_mutation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script flag mutation",
        runtime_shell
            .shell
            .script_flag_command_keys()
            .into_iter()
            .filter(|key| {
                key.map_name == current_map
                    && matches!(
                        key.command.as_str(),
                        "setevent"
                            | "clearevent"
                            | "set_flag"
                            | "clear_flag"
                            | "setflag"
                            | "clearflag"
                            | "setengineflag"
                    )
            })
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_flag_mutation(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script flag mutation {}/{} command={} flag={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.flag_id,
        applied.outcome,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn check_selected_script_flag(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script flag check",
        runtime_shell
            .shell
            .script_flag_command_keys()
            .into_iter()
            .filter(|key| {
                key.map_name == current_map
                    && matches!(
                        key.command.as_str(),
                        "checkevent" | "checkflag" | "check_flag"
                    )
            })
            .collect(),
    )?;
    let checked = runtime_shell.shell.check_script_flag(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script flag check {}/{} command={} flag={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.flag_id,
        checked.outcome,
        checked.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_scene_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script scene",
        runtime_shell
            .shell
            .script_scene_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_scene_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script scene {}/{} command={} map={:?} scene={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.map_id,
        key.scene_id,
        applied.outcome,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_block_change(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script block change",
        runtime_shell
            .shell
            .script_block_change_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_block_change(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script block change {}/{} x={} y={} block={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.x,
        key.y,
        key.block_id,
        applied.outcome,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_audio_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script audio",
        runtime_shell
            .shell
            .script_audio_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_audio_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script audio {}/{} command={} audio={:?} fade={:?} cue={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.audio_id,
        key.fade_frames,
        applied.cue,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_text_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script text",
        runtime_shell
            .shell
            .script_text_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_text_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script text {}/{} command={} label={:?} action={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.text_label,
        applied.action,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_variable_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script variable",
        runtime_shell
            .shell
            .script_variable_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_variable_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script variable {}/{} command={} target={:?} values={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.target,
        key.value_tokens,
        applied.outcome,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_control_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script control",
        runtime_shell
            .shell
            .script_control_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_control_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script control {}/{} command={} compare={:?} target={:?} resolved={:?} action={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.compare_value,
        key.target_label,
        key.resolved_target_script,
        applied.action,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_object_mutation(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script object",
        runtime_shell
            .shell
            .script_object_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_object_mutation(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script object {}/{} command={} object={:?} target={:?} xy=({:?},{:?}) dir={:?} movement={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.object_id,
        key.target_object_id,
        key.x,
        key.y,
        key.direction,
        key.movement,
        applied.outcome,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_script_map_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script map",
        runtime_shell
            .shell
            .script_map_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_map_command(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script map {}/{} command={} target={:?} xy=({:?},{:?}) facing={:?} setup={:?} action={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.target_map,
        key.x,
        key.y,
        key.facing,
        key.map_setup,
        applied.action,
        applied.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_selected_script_shop(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, key) = selected_script_command_key(
        runtime_shell,
        "script shop",
        runtime_shell
            .shell
            .script_shop_command_keys()
            .into_iter()
            .filter(|key| key.map_name == current_map)
            .collect(),
    )?;
    let opened = runtime_shell.shell.open_script_shop(
        &key.map_name,
        &key.source_script,
        key.command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script shop {}/{} command={} mart_type={} mart_id={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        key.command,
        key.mart_type,
        key.mart_id,
        opened.outcome,
        opened.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn apply_selected_runtime_command_named(
    runtime_shell: &mut BevyRuntimeShell,
    command_name: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let current_map = snapshot.overworld.map_name.clone();
    let (selected_index, selected_len, command) = selected_script_command_key(
        runtime_shell,
        command_name,
        runtime_shell
            .shell
            .script_runtime_command_keys()
            .into_iter()
            .filter(|command| command.map_name == current_map && command.command == command_name)
            .collect(),
    )?;
    let applied = runtime_shell.shell.apply_script_runtime_command(
        &command.map_name,
        &command.source_script,
        command.command_index,
        explicit_script_runtime_inputs(
            runtime_shell,
            &command.command,
            &command.args,
            command.command_index,
        )?,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "runtime command {}/{} {} args={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        command.command,
        command.args,
        applied.outcome,
        applied.state_checksum
    ));
    Ok(())
}

fn apply_selected_trade_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_selected_runtime_command_named(runtime_shell, "trade")
}

fn apply_selected_catch_tutorial_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_selected_runtime_command_named(runtime_shell, "catchtutorial")
}

fn deposit_visible_day_care_man(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::Deposit,
    )
}

fn deposit_visible_day_care_lady(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Lady,
        RuntimeDayCareAction::Deposit,
    )
}

fn withdraw_visible_day_care_man(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::Withdraw,
    )
}

fn withdraw_visible_day_care_lady(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Lady,
        RuntimeDayCareAction::Withdraw,
    )
}

fn inspect_visible_day_care_man(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Man,
        RuntimeDayCareAction::Inspect,
    )
}

fn inspect_visible_day_care_lady(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_day_care(
        runtime_shell,
        RuntimeDayCareCaretaker::Lady,
        RuntimeDayCareAction::Inspect,
    )
}

fn use_visible_day_care(
    runtime_shell: &mut BevyRuntimeShell,
    caretaker: RuntimeDayCareCaretaker,
    action: RuntimeDayCareAction,
) -> Result<()> {
    let party_index = if action == RuntimeDayCareAction::Deposit {
        Some(selected_party_index(runtime_shell)?)
    } else {
        None
    };
    let used = runtime_shell
        .shell
        .use_day_care(caretaker, action, party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "day care caretaker={:?} action={:?} party_index={:?} outcome={:?} checksum={:?}",
        caretaker, action, party_index, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn bug_contest_give_park_balls(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::GiveParkBalls, None)
}

fn bug_contest_select_contestants(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(
        runtime_shell,
        RuntimeBugContestAction::SelectContestants,
        None,
    )
}

fn bug_contest_drop_off_mons(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::DropOffMons, None)
}

fn bug_contest_return_mons(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::ReturnMons, None)
}

fn bug_contest_check_party_full(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::CheckPartyFull, None)
}

fn judge_visible_bug_contest_rank(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let rank = ((runtime_shell.script_command_cursor % 3) + 1) as u8;
    use_visible_bug_contest(runtime_shell, RuntimeBugContestAction::Judge, Some(rank))
}

fn use_visible_bug_contest(
    runtime_shell: &mut BevyRuntimeShell,
    action: RuntimeBugContestAction,
    rank: Option<u8>,
) -> Result<()> {
    let used = runtime_shell.shell.use_bug_contest(action, rank)?;
    runtime_shell.last_audio_events.push(format!(
        "bug contest action={:?} rank={:?} outcome={:?} checksum={:?}",
        action, rank, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn use_visible_kurt_apricorn(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let apricorn_id = selected_bag_item_id(runtime_shell)?;
    if !snapshot
        .special
        .kurt_apricorn_recipes
        .contains_key(&apricorn_id)
    {
        anyhow::bail!("selected item {apricorn_id} is not a compiled Kurt apricorn");
    }
    let quantity = snapshot
        .bag
        .items
        .iter()
        .find(|item| item.item_id == apricorn_id)
        .map(|item| item.quantity)
        .with_context(|| format!("selected apricorn {apricorn_id} is not carried"))?;
    let used = runtime_shell
        .shell
        .use_kurt_apricorn(apricorn_id.clone(), quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "kurt apricorn item={} quantity={} outcome={:?} checksum={:?}",
        apricorn_id, quantity, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn use_visible_buena_password(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.use_current_buena_password_guess()?;
    runtime_shell.last_audio_events.push(format!(
        "buena password outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn use_visible_buena_prize(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let balance = snapshot.trainer.blue_card_balance;
    let affordable = snapshot
        .special
        .buena_prizes
        .iter()
        .filter(|(_, cost)| u16::from(**cost) <= balance)
        .map(|(item_id, cost)| (item_id.clone(), *cost))
        .collect::<Vec<_>>();
    if affordable.is_empty() {
        anyhow::bail!("Blue Card balance cannot afford any compiled Buena prize");
    }
    let selected_index = runtime_shell.script_command_cursor % affordable.len();
    let (item_id, cost) = affordable[selected_index].clone();
    let used = runtime_shell.shell.use_buena_prize(item_id.clone(), 1)?;
    runtime_shell.last_audio_events.push(format!(
        "buena prize {}/{} item={} cost={} balance={} outcome={:?} checksum={:?}",
        selected_index + 1,
        affordable.len(),
        item_id,
        cost,
        balance,
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn give_visible_shuckie(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell
        .shell
        .use_shuckie(RuntimeShuckieAction::Give, None)?;
    runtime_shell.last_audio_events.push(format!(
        "shuckie give outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn return_visible_shuckie(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let used = runtime_shell
        .shell
        .use_shuckie(RuntimeShuckieAction::Return, Some(party_index))?;
    runtime_shell.last_audio_events.push(format!(
        "shuckie return party_index={} outcome={:?} checksum={:?}",
        party_index, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn give_visible_odd_egg(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.give_odd_egg()?;
    runtime_shell.last_audio_events.push(format!(
        "odd egg outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn give_visible_dratini(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.special.dratini_move_sets.is_empty() {
        anyhow::bail!("compiled pack declares no Dratini move sets");
    }
    let mode = *snapshot
        .special
        .dratini_move_sets
        .keys()
        .nth(runtime_shell.script_command_cursor % snapshot.special.dratini_move_sets.len())
        .context("selected Dratini move set missing from compiled pack")?;
    let used = runtime_shell.shell.give_dratini(mode)?;
    runtime_shell.last_audio_events.push(format!(
        "dratini mode={} outcome={:?} checksum={:?}",
        mode, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn use_visible_bills_grandfather(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let used = runtime_shell
        .shell
        .use_bills_grandfather(Some(party_index), None)?;
    runtime_shell.last_audio_events.push(format!(
        "bill grandfather party_index={} outcome={:?} checksum={:?}",
        party_index, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn init_visible_roam_mons(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.init_roam_mons()?;
    runtime_shell.last_audio_events.push(format!(
        "roamers outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn check_visible_magikarp_length(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let used = runtime_shell.shell.check_magikarp_length(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "magikarp length party_index={} outcome={:?} checksum={:?}",
        party_index, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn show_visible_prof_oaks_pc_boot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.show_prof_oaks_pc_boot()?;
    runtime_shell.last_audio_events.push(format!(
        "prof oak pc outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn show_visible_magikarp_house_sign(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.show_magikarp_house_sign()?;
    runtime_shell.last_audio_events.push(format!(
        "magikarp sign outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn apply_visible_battle_tower_reset(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.apply_battle_tower_action(
        "BATTLETOWERACTION_RESETDATA".to_string(),
        None,
        None,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "battle tower reset outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn apply_visible_older_haircut(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_happiness_service(
        runtime_shell,
        RuntimeHappinessServiceRoutine::OlderHaircutBrother,
    )
}

fn apply_visible_younger_haircut(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_happiness_service(
        runtime_shell,
        RuntimeHappinessServiceRoutine::YoungerHaircutBrother,
    )
}

fn apply_visible_daisy_grooming(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_happiness_service(
        runtime_shell,
        RuntimeHappinessServiceRoutine::DaisysGrooming,
    )
}

fn apply_visible_happiness_service(
    runtime_shell: &mut BevyRuntimeShell,
    routine: RuntimeHappinessServiceRoutine,
) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let rng_roll = (runtime_shell.script_command_cursor % 256) as u8;
    let used = runtime_shell
        .shell
        .apply_happiness_service(routine, party_index, rng_roll)?;
    runtime_shell.last_audio_events.push(format!(
        "happiness routine={:?} party_index={} roll={} outcome={:?} checksum={:?}",
        routine, party_index, rng_roll, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn check_visible_mystery_gift(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_mystery_gift(runtime_shell, RuntimeMysteryGiftAction::Check)
}

fn claim_visible_mystery_gift_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_mystery_gift(runtime_shell, RuntimeMysteryGiftAction::ClaimItem)
}

fn unlock_visible_mystery_gift(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    use_visible_mystery_gift(runtime_shell, RuntimeMysteryGiftAction::Unlock)
}

fn use_visible_mystery_gift(
    runtime_shell: &mut BevyRuntimeShell,
    action: RuntimeMysteryGiftAction,
) -> Result<()> {
    let used = runtime_shell.shell.use_mystery_gift(action)?;
    runtime_shell.last_audio_events.push(format!(
        "mystery gift action={:?} outcome={:?} checksum={:?}",
        action, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn set_visible_player_palette(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let palette_id = (runtime_shell.script_command_cursor % 8) as u8;
    let used = runtime_shell
        .shell
        .set_player_palette(0x80 | (palette_id << 4))?;
    runtime_shell.last_audio_events.push(format!(
        "player palette selected={} outcome={:?} checksum={:?}",
        palette_id, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn set_visible_day_of_week(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.set_day_of_week()?;
    runtime_shell.last_audio_events.push(format!(
        "day of week outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn update_visible_time(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let used = runtime_shell.shell.update_time()?;
    runtime_shell.last_audio_events.push(format!(
        "time update outcome={:?} checksum={:?}",
        used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn grant_selected_gift_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.gift_pokemon.is_empty() {
        anyhow::bail!("no compiled gift Pokemon is visible");
    }
    let selected_index = runtime_shell.script_command_cursor % snapshot.ui.gift_pokemon.len();
    let gift = &snapshot.ui.gift_pokemon[selected_index];
    let granted = runtime_shell.shell.grant_compiled_gift_pokemon_command(
        &gift.source_script,
        gift.command_index,
        snapshot.trainer.player_name.clone(),
        snapshot.trainer.player_id,
        gift_dvs_from_checksum(snapshot.state_checksum.hash()),
        false,
        None,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "gift pokemon {}/{} species={} level={} outcome={:?} checksum={:?}",
        selected_index + 1,
        snapshot.ui.gift_pokemon.len(),
        gift.species_id,
        gift.level,
        granted.outcome,
        granted.state_checksum
    ));
    Ok(())
}

fn gift_dvs_from_checksum(hash: u32) -> Dv {
    Dv::from_non_hp(
        ((hash >> 24) & 0x0f) as u8,
        ((hash >> 16) & 0x0f) as u8,
        ((hash >> 8) & 0x0f) as u8,
        (hash & 0x0f) as u8,
    )
}

fn first_party_index(snapshot: &RuntimeShellSnapshot) -> Result<usize> {
    snapshot
        .party
        .slots
        .first()
        .map(|slot| slot.index)
        .with_context(|| "party is empty")
}

fn second_party_index(snapshot: &RuntimeShellSnapshot) -> Result<usize> {
    snapshot
        .party
        .slots
        .get(1)
        .map(|slot| slot.index)
        .with_context(|| "party has no second slot")
}

fn selected_carried_normal_item_matching(
    runtime_shell: &mut BevyRuntimeShell,
    predicate: impl Fn(&crate::RuntimeItemCatalogSnapshot) -> bool,
    empty_message: &str,
) -> Result<String> {
    let item_id = selected_bag_item_id(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?;
    if !predicate(item) {
        anyhow::bail!("{empty_message}: selected item {item_id} is not valid");
    }
    Ok(item_id)
}

fn carried_battle_usable_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    snapshot
        .bag
        .items
        .iter()
        .filter(|bag_item| bag_item.quantity > 0)
        .filter(|bag_item| {
            snapshot
                .items
                .iter()
                .any(|item| item.item_id == bag_item.item_id && item.battle_usable)
        })
        .map(|item| item.item_id.clone())
        .collect()
}

fn selected_battle_bag_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = carried_battle_usable_item_ids(&snapshot);
    if item_ids.is_empty() {
        anyhow::bail!("bag item pocket has no carried battle-usable item");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.bag_cursor,
        "battle:bag-items",
        item_ids.len(),
    );
    Ok(item_ids[index].clone())
}

fn selected_battle_ball_id(runtime_shell: &mut BevyRuntimeShell) -> Result<(usize, String)> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let ball_ids = carried_ball_item_ids(&snapshot);
    if ball_ids.is_empty() {
        anyhow::bail!("bag has no carried ball");
    }
    let index = visible_cursor_index(&mut runtime_shell.ball_cursor, "bag:balls", ball_ids.len());
    Ok((index, ball_ids[index].clone()))
}

fn carried_ball_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    snapshot
        .bag
        .balls
        .iter()
        .filter(|ball| ball.quantity > 0)
        .map(|ball| ball.item_id.clone())
        .collect()
}

fn selected_bag_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = snapshot
        .bag
        .items
        .iter()
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    if item_ids.is_empty() {
        anyhow::bail!("bag item pocket has no carried item");
    }
    let index = visible_cursor_index(&mut runtime_shell.bag_cursor, "bag:items", item_ids.len());
    Ok(item_ids[index].clone())
}

fn selected_pc_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_ids = snapshot
        .bag
        .pc_items
        .iter()
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    if item_ids.is_empty() {
        anyhow::bail!("PC item storage has no item");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.pc_item_cursor,
        "pc:items",
        item_ids.len(),
    );
    Ok(item_ids[index].clone())
}

fn selected_bag_or_pc_item_id(runtime_shell: &mut BevyRuntimeShell) -> Result<String> {
    if runtime_shell.ball_cursor.is_some() {
        if let Ok((_, item_id)) = selected_battle_ball_id(runtime_shell) {
            return Ok(item_id);
        }
    }
    if runtime_shell.tmhm_cursor.is_some() {
        if let Ok((item_id, _)) = selected_tmhm(runtime_shell) {
            return Ok(item_id);
        }
    }
    if runtime_shell.pc_item_cursor.is_some() {
        if let Ok(item_id) = selected_pc_item_id(runtime_shell) {
            return Ok(item_id);
        }
    }
    if let Ok(item_id) = selected_bag_item_id(runtime_shell) {
        return Ok(item_id);
    }
    if let Ok((_, item_id)) = selected_battle_ball_id(runtime_shell) {
        return Ok(item_id);
    }
    if let Ok((item_id, _)) = selected_tmhm(runtime_shell) {
        return Ok(item_id);
    }
    selected_pc_item_id(runtime_shell)
}

fn selected_tmhm(runtime_shell: &mut BevyRuntimeShell) -> Result<(String, Option<String>)> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let tmhms = snapshot
        .bag
        .tm_hm
        .iter()
        .map(|tmhm| (tmhm.item_id.clone(), tmhm.move_id.clone()))
        .collect::<Vec<_>>();
    if tmhms.is_empty() {
        anyhow::bail!("bag has no carried TM/HM");
    }
    let index = visible_cursor_index(&mut runtime_shell.tmhm_cursor, "bag:tmhm", tmhms.len());
    Ok(tmhms[index].clone())
}

fn selected_party_special_item_id(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    behavior_id: &str,
    error_message: &str,
) -> Result<String> {
    let item_id = selected_bag_item_id(runtime_shell)?;
    let matches_behavior = snapshot
        .item_effect_plans
        .iter()
        .any(|plan| plan.item_id == item_id && plan.behavior_id == behavior_id);
    if !matches_behavior {
        anyhow::bail!("{error_message}: selected item {item_id} is not valid");
    }
    Ok(item_id)
}

fn use_selected_party_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_party_item_on(runtime_shell, party_index)
}

fn use_selected_party_item_on_second_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = second_party_index(&snapshot)?;
    use_selected_party_item_on(runtime_shell, party_index)
}

fn use_selected_party_item_on(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| {
            item.revive_hp_percent.is_some()
                || !item.status_heals.is_empty()
                || item.confusion_heal == Some(true)
                || item.vitamin_stat.is_some()
                || item.rare_candy_level_gain.is_some()
        },
        "bag has no carried party item matching compiled party-use effect fields",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_item_on_party_pokemon(&item_id, party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "party item item={} party_index={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.item_effect, used.state_checksum
    ));
    Ok(())
}

fn use_selected_whole_party_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.party_revive_hp_percent.is_some(),
        "bag has no carried whole-party item",
    )?;
    let used = runtime_shell.shell.use_bag_item_on_whole_party(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "whole party item item={} item_use={:?} effect={:?} checksum={:?}",
        item_id, used.item_use, used.item_effect, used.state_checksum
    ));
    Ok(())
}

fn use_selected_pp_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_pp_item_on(runtime_shell, party_index)
}

fn use_selected_pp_item_on_second_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = second_party_index(&snapshot)?;
    use_selected_pp_item_on(runtime_shell, party_index)
}

fn use_selected_pp_item_on(runtime_shell: &mut BevyRuntimeShell, party_index: usize) -> Result<()> {
    let move_slot = selected_party_move_slot(runtime_shell, party_index)?;
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.pp_restore_points.is_some() || item.pp_up_stages.is_some(),
        "bag has no carried PP item",
    )?;
    let used =
        runtime_shell
            .shell
            .use_bag_item_on_party_move(&item_id, party_index, Some(move_slot))?;
    runtime_shell.last_audio_events.push(format!(
        "party move item item={} party_index={} move_slot={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, move_slot, used.item_use, used.item_effect, used.state_checksum
    ));
    Ok(())
}

fn use_selected_rare_candy(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let item_id = selected_party_special_item_id(
        runtime_shell,
        &snapshot,
        ITEM_EFFECT_BEHAVIOR_RARE_CANDY,
        "selected item is not a rare-candy party item",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_item_on_party_pokemon(&item_id, party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "rare candy item={} party_index={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.item_effect, used.state_checksum
    ));
    Ok(())
}

fn use_selected_evolution_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let item_id = selected_party_special_item_id(
        runtime_shell,
        &snapshot,
        ITEM_EFFECT_BEHAVIOR_EVOLUTION_STONE,
        "selected item is not an evolution party item",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_item_on_party_pokemon(&item_id, party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "evolution item item={} party_index={} item_use={:?} effect={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.item_effect, used.state_checksum
    ));
    Ok(())
}

fn swap_visible_selected_party_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let lead = snapshot
        .party
        .slots
        .first()
        .with_context(|| "party has no lead Pokemon")?;
    let selected_index = selected_party_index(runtime_shell)?;
    let swap_index = if selected_index == lead.index {
        snapshot
            .party
            .slots
            .get(1)
            .map(|slot| slot.index)
            .with_context(|| "party has no second Pokemon to swap with selected lead")?
    } else {
        selected_index
    };
    let swapped = runtime_shell
        .shell
        .swap_party_pokemon(lead.index, swap_index)?;
    runtime_shell.last_audio_events.push(format!(
        "party swap {}<->{} first_after={} second_after={} checksum={:?}",
        swapped.first_party_index,
        swapped.second_party_index,
        swapped.first_species_after,
        swapped.second_species_after,
        swapped.state_checksum
    ));
    Ok(())
}

fn give_selected_held_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let item_id = selected_bag_item_id(runtime_shell)?;
    let transfer = runtime_shell
        .shell
        .give_bag_item_to_party_pokemon(&item_id, party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "held item give item={} party_index={} bag_after={} checksum={:?}",
        transfer.item_id,
        transfer.party_index,
        transfer.bag_quantity_after,
        transfer.state_checksum
    ));
    Ok(())
}

fn take_selected_held_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let transfer = runtime_shell
        .shell
        .take_held_item_from_party_pokemon(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "held item take item={} party_index={} bag_after={} checksum={:?}",
        transfer.item_id,
        transfer.party_index,
        transfer.bag_quantity_after,
        transfer.state_checksum
    ));
    Ok(())
}

fn award_visible_badge(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let badge_slots = [
        (RuntimeBadgeRegion::Johto, 0usize),
        (RuntimeBadgeRegion::Johto, 1),
        (RuntimeBadgeRegion::Johto, 2),
        (RuntimeBadgeRegion::Johto, 3),
        (RuntimeBadgeRegion::Johto, 4),
        (RuntimeBadgeRegion::Johto, 5),
        (RuntimeBadgeRegion::Johto, 6),
        (RuntimeBadgeRegion::Johto, 7),
        (RuntimeBadgeRegion::Kanto, 0),
        (RuntimeBadgeRegion::Kanto, 1),
        (RuntimeBadgeRegion::Kanto, 2),
        (RuntimeBadgeRegion::Kanto, 3),
        (RuntimeBadgeRegion::Kanto, 4),
        (RuntimeBadgeRegion::Kanto, 5),
        (RuntimeBadgeRegion::Kanto, 6),
        (RuntimeBadgeRegion::Kanto, 7),
    ];
    let selected_index = runtime_shell.script_command_cursor % badge_slots.len();
    let (region, index) = badge_slots[selected_index];
    let award = runtime_shell.shell.award_badge(region, index)?;
    runtime_shell.last_audio_events.push(format!(
        "badge award {}/{} region={:?} index={} already={} total={} checksum={:?}",
        selected_index + 1,
        badge_slots.len(),
        award.region,
        award.index,
        award.already_awarded,
        award.awarded_count_after,
        award.state_checksum
    ));
    Ok(())
}

fn record_selected_pokedex_seen(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let species_id = selected_pokedex_species_id(runtime_shell)?;
    let record = runtime_shell.shell.record_pokedex_seen(&species_id)?;
    runtime_shell.last_audio_events.push(format!(
        "pokedex seen species={} already_seen={} already_caught={} seen={} caught={} checksum={:?}",
        record.species_id,
        record.already_seen,
        record.already_caught,
        record.seen_count_after,
        record.caught_count_after,
        record.state_checksum
    ));
    Ok(())
}

fn record_selected_pokedex_caught(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let species_id = selected_pokedex_species_id(runtime_shell)?;
    let record = runtime_shell.shell.record_pokedex_caught(&species_id)?;
    runtime_shell.last_audio_events.push(format!(
        "pokedex caught species={} already_seen={} already_caught={} seen={} caught={} checksum={:?}",
        record.species_id,
        record.already_seen,
        record.already_caught,
        record.seen_count_after,
        record.caught_count_after,
        record.state_checksum
    ));
    Ok(())
}

fn add_visible_money(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_currency_delta(runtime_shell, RuntimeCurrencyAccount::Money, 1_000, true)
}

fn take_visible_money(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_currency_delta(runtime_shell, RuntimeCurrencyAccount::Money, 100, false)
}

fn add_visible_coins(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    apply_visible_currency_delta(runtime_shell, RuntimeCurrencyAccount::Coins, 100, true)
}

fn apply_visible_currency_delta(
    runtime_shell: &mut BevyRuntimeShell,
    account: RuntimeCurrencyAccount,
    amount: u32,
    add: bool,
) -> Result<()> {
    let mutation = if add {
        runtime_shell.shell.add_currency(account, amount)?
    } else {
        runtime_shell.shell.take_currency(account, amount)?
    };
    runtime_shell.last_audio_events.push(format!(
        "currency {} account={:?} amount={} before={} after={} cap={} checksum={:?}",
        if add { "add" } else { "take" },
        mutation.account,
        mutation.amount,
        mutation.value_before,
        mutation.value_after,
        mutation.cap,
        mutation.state_checksum
    ));
    Ok(())
}

fn record_visible_link_win(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_link_result(runtime_shell, RuntimeLinkBattleResult::Win)
}

fn record_visible_link_loss(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_link_result(runtime_shell, RuntimeLinkBattleResult::Loss)
}

fn record_visible_link_draw(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_link_result(runtime_shell, RuntimeLinkBattleResult::Draw)
}

fn record_visible_link_result(
    runtime_shell: &mut BevyRuntimeShell,
    result: RuntimeLinkBattleResult,
) -> Result<()> {
    let record = runtime_shell.shell.record_link_battle_result(result)?;
    runtime_shell.last_audio_events.push(format!(
        "link result={:?} wins={} losses={} draws={} checksum={:?}",
        record.result,
        record.wins_after,
        record.losses_after,
        record.draws_after,
        record.state_checksum
    ));
    Ok(())
}

fn toggle_visible_battle_style(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let mut options = snapshot.trainer.options.clone();
    options.battle_style = match options.battle_style {
        BattleStyle::Shift => BattleStyle::Set,
        BattleStyle::Set => BattleStyle::Shift,
    };
    let result = runtime_shell.shell.set_options(options)?;
    runtime_shell.last_audio_events.push(format!(
        "options battle_style {:?}->{:?} checksum={:?}",
        result.options_before.battle_style,
        result.options_after.battle_style,
        result.state_checksum
    ));
    Ok(())
}

fn teach_selected_tmhm(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    teach_selected_tmhm_on(runtime_shell, party_index)
}

fn teach_selected_tmhm_on_second_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = second_party_index(&snapshot)?;
    teach_selected_tmhm_on(runtime_shell, party_index)
}

fn teach_selected_tmhm_on(runtime_shell: &mut BevyRuntimeShell, party_index: usize) -> Result<()> {
    let (item_id, move_id) = selected_tmhm(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let move_count = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?
        .pokemon
        .moves
        .len();
    let replace_slot = if move_count >= 4 {
        Some(selected_party_move_slot(runtime_shell, party_index)?)
    } else {
        None
    };
    let taught =
        runtime_shell
            .shell
            .use_bag_tmhm_on_party_pokemon(&item_id, party_index, replace_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "tmhm item={} move={:?} party_index={} replace_slot={:?} item_use={:?} checksum={:?}",
        item_id, move_id, party_index, replace_slot, taught.item_use, taught.state_checksum
    ));
    Ok(())
}

fn use_selected_active_battle_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| {
            item.battle_usable
                && (item.battle_stat_boost_stat.is_some()
                    || item.battle_focus_energy == Some(true)
                    || item.battle_stat_drop_guard == Some(true)
                    || item.revive_hp_percent.is_some()
                    || !item.status_heals.is_empty()
                    || item.confusion_heal == Some(true))
        },
        "bag has no carried active battle item",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_item_on_active_battle_pokemon(&item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "active battle item item={} item_use={:?} battle_item={:?} checksum={:?}",
        item_id, used.item_use, used.battle_item, used.state_checksum
    ));
    Ok(())
}

fn use_selected_visible_battle_bag_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?;
    let battle_usable = item.battle_usable;
    let battle_escape_mode = item.battle_escape_mode.clone();
    let battle_stat_drop_guard = item.battle_stat_drop_guard;
    let battle_menu = item.battle_menu.clone();
    if !battle_usable {
        anyhow::bail!("selected bag item {item_id} is not battle usable");
    }
    if battle_escape_mode.is_some() {
        return use_selected_battle_escape_item(runtime_shell);
    }
    if battle_stat_drop_guard == Some(true) {
        return use_selected_guard_spec(runtime_shell);
    }
    let targets_move = item_targets_party_move_fields(
        item.pp_restore_scope.as_deref(),
        item.pp_restore_points,
        item.pp_up_stages,
    );
    match battle_menu.as_str() {
        "ITEMMENU_CLOSE" => use_selected_active_battle_item(runtime_shell),
        "ITEMMENU_PARTY" if targets_move => use_selected_battle_party_move_item(runtime_shell),
        "ITEMMENU_PARTY" => use_selected_battle_party_item(runtime_shell),
        other => anyhow::bail!("selected bag item {item_id} has unsupported battle menu {other}"),
    }
}

fn selected_visible_battle_bag_menu(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<Option<String>> {
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return Ok(None);
    }
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?;
    if item.battle_usable {
        Ok(Some(item.battle_menu.clone()))
    } else {
        Ok(None)
    }
}

fn selected_visible_battle_bag_item_targets_move(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<bool> {
    let item_id = selected_battle_bag_item_id(runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return Ok(false);
    }
    let item = snapshot
        .items
        .iter()
        .find(|item| item.item_id == item_id)
        .with_context(|| format!("selected bag item {item_id} is missing from item catalog"))?;
    Ok(item.battle_usable
        && item_targets_party_move_fields(
            item.pp_restore_scope.as_deref(),
            item.pp_restore_points,
            item.pp_up_stages,
        ))
}

fn item_targets_party_move_fields(
    pp_restore_scope: Option<&str>,
    pp_restore_points: Option<u8>,
    pp_up_stages: Option<u8>,
) -> bool {
    pp_up_stages.is_some() || (pp_restore_scope == Some("MOVE") && pp_restore_points.is_some())
}

fn use_selected_battle_party_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_battle_party_item_on(runtime_shell, party_index)
}

fn use_selected_battle_party_item_on_second_slot(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let party_index = second_party_index(&snapshot)?;
    use_selected_battle_party_item_on(runtime_shell, party_index)
}

fn use_selected_battle_party_item_on(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| {
            item.battle_usable
                && (item.battle_stat_boost_stat.is_some()
                    || item.battle_focus_energy == Some(true)
                    || item.battle_stat_drop_guard == Some(true)
                    || item.revive_hp_percent.is_some()
                    || !item.status_heals.is_empty()
                    || item.confusion_heal == Some(true))
                || item.revive_hp_percent.is_some()
                || !item.status_heals.is_empty()
                || item.confusion_heal == Some(true)
                || item.vitamin_stat.is_some()
                || item.rare_candy_level_gain.is_some()
        },
        "bag has no carried battle party item",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_item_on_battle_party_pokemon(&item_id, party_index)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle party item item={} party_index={} item_use={:?} battle_item={:?} checksum={:?}",
        item_id, party_index, used.item_use, used.battle_item, used.state_checksum
    ));
    Ok(())
}

fn use_selected_battle_party_move_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let party_index = selected_party_index(runtime_shell)?;
    use_selected_battle_party_move_item_on(runtime_shell, party_index)
}

fn use_selected_battle_party_move_item_on_second_slot(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let party_index = second_party_index(&snapshot)?;
    use_selected_battle_party_move_item_on(runtime_shell, party_index)
}

fn use_selected_battle_party_move_item_on(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let move_slot = selected_party_move_slot(runtime_shell, party_index)?;
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.pp_restore_points.is_some() || item.pp_up_stages.is_some(),
        "bag has no carried battle PP item",
    )?;
    let used = runtime_shell.shell.use_bag_item_on_battle_party_move(
        &item_id,
        party_index,
        Some(move_slot),
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle party move item item={} party_index={} move_slot={} item_use={:?} battle_item={:?} checksum={:?}",
        item_id, party_index, move_slot, used.item_use, used.battle_item, used.state_checksum
    ));
    Ok(())
}

fn use_selected_battle_escape_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.battle_escape_mode.is_some(),
        "bag has no carried battle escape item",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_item_to_escape_active_wild_battle(&item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle escape item item={} item_use={:?} mode={:?} escaped={} checksum={:?}",
        item_id, used.item_use, used.battle_escape_mode, used.escaped, used.state_checksum
    ));
    Ok(())
}

fn use_selected_guard_spec(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        anyhow::bail!("no active battle");
    }
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.battle_stat_drop_guard == Some(true),
        "bag has no carried Guard Spec item",
    )?;
    let used = runtime_shell
        .shell
        .use_bag_guard_spec_in_active_battle(&item_id)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "guard spec item={} item_use={:?} guard_turns {}->{} checksum={:?}",
        item_id,
        used.item_use,
        used.stat_drop_guard_turns_before,
        used.stat_drop_guard_turns_after,
        used.state_checksum
    ));
    Ok(())
}

fn start_or_complete_visible_scripted_wild_battle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(battle) = snapshot.battle {
        let RuntimeBattleKind::StaticWild { source_script, .. } = battle.kind else {
            anyhow::bail!("active battle is not a scripted wild battle");
        };
        complete_visible_scripted_wild_battle(
            runtime_shell,
            &snapshot.overworld.map_name,
            &source_script,
        )?;
        return Ok(());
    }
    let key = runtime_shell
        .shell
        .scripted_wild_battle_keys()
        .into_iter()
        .find(|key| key.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "map {} has no compiled scripted wild battle",
                snapshot.overworld.map_name
            )
        })?;
    let start = runtime_shell.shell.start_scripted_wild_battle(
        &key.map_name,
        &key.source_script,
        key.startbattle_command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "scripted wild start source={} species={} level={} start={:?}",
        key.source_script, key.species, key.level, start
    ));
    Ok(())
}

fn start_or_complete_visible_scripted_trainer_battle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(battle) = snapshot.battle {
        let RuntimeBattleKind::Trainer { source_script, .. } = battle.kind else {
            anyhow::bail!("active battle is not a scripted trainer battle");
        };
        complete_visible_scripted_trainer_battle(
            runtime_shell,
            &snapshot.overworld.map_name,
            &source_script,
            true,
            false,
        )?;
        return Ok(());
    }
    let key = runtime_shell
        .shell
        .scripted_trainer_battle_keys()
        .into_iter()
        .find(|key| key.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "map {} has no compiled scripted trainer battle",
                snapshot.overworld.map_name
            )
        })?;
    let start = runtime_shell.shell.start_scripted_trainer_battle(
        &key.map_name,
        &key.source_script,
        key.startbattle_command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "scripted trainer start source={} trainer={}:{} start={:?}",
        key.source_script, key.trainer_class, key.trainer_id, start
    ));
    Ok(())
}

fn complete_visible_scripted_wild_battle(
    runtime_shell: &mut BevyRuntimeShell,
    map_name: &str,
    source_script: &str,
) -> Result<()> {
    let key = runtime_shell
        .shell
        .scripted_wild_battle_keys()
        .into_iter()
        .find(|key| key.map_name == map_name && key.source_script == source_script)
        .with_context(|| {
            format!(
                "compiled scripted wild battle key missing for {} on {}",
                source_script, map_name
            )
        })?;
    let completion = runtime_shell.shell.complete_scripted_wild_battle(
        &key.map_name,
        &key.source_script,
        key.startbattle_command_index,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "scripted wild complete source={} reload={} effects={:?} checksum={:?}",
        key.source_script,
        key.reload_map_after_battle,
        completion.effects,
        completion.state_checksum
    ));
    if key.reload_map_after_battle {
        arm_visible_current_map_callbacks(runtime_shell, "after_wild_battle")?;
    }
    Ok(())
}

fn complete_visible_scripted_trainer_battle(
    runtime_shell: &mut BevyRuntimeShell,
    map_name: &str,
    source_script: &str,
    won: bool,
    can_lose: bool,
) -> Result<()> {
    let key = runtime_shell
        .shell
        .scripted_trainer_battle_keys()
        .into_iter()
        .find(|key| key.map_name == map_name && key.source_script == source_script)
        .with_context(|| {
            format!(
                "compiled scripted trainer battle key missing for {} on {}",
                source_script, map_name
            )
        })?;
    let completion = runtime_shell.shell.complete_scripted_trainer_battle(
        &key.map_name,
        &key.source_script,
        key.startbattle_command_index,
        won,
        can_lose,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "scripted trainer complete source={} reload={} effects={:?} checksum={:?}",
        key.source_script,
        key.reload_map_after_battle,
        completion.effects,
        completion.state_checksum
    ));
    if key.reload_map_after_battle {
        arm_visible_current_map_callbacks(runtime_shell, "after_trainer_battle")?;
    }
    Ok(())
}

fn resolve_visible_battle_move(runtime_shell: &mut BevyRuntimeShell, slot: usize) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if !battle.commands.player_move_slots.contains(&slot) {
        anyhow::bail!("player move slot {slot} is not available");
    }
    let enemy_slot = selected_enemy_battle_move_slot(&snapshot, &battle.commands)?;
    let turn = runtime_shell.shell.resolve_active_battle_turn(
        BattleAction::Move { slot },
        BattleAction::Move { slot: enemy_slot },
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle move player_slot={} enemy_slot={} outcome={:?} checksum={:?}",
        slot, enemy_slot, turn.outcome, turn.state_checksum
    ));
    Ok(())
}

fn visible_battle_action_ids(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Vec<&'static str> {
    let mut actions = Vec::new();
    if !battle.commands.player_move_slots.is_empty() {
        actions.push("Fight");
    }
    if !battle.commands.switch_party_indices.is_empty() {
        actions.push("Pokemon");
    }
    if battle.commands.can_use_items && !carried_battle_usable_item_ids(snapshot).is_empty() {
        actions.push("Pack");
    }
    if battle.commands.can_use_items && snapshot.bag.balls.iter().any(|ball| ball.quantity > 0) {
        actions.push("Ball");
    }
    if battle.commands.can_run {
        actions.push("Run");
    }
    actions
}

fn sync_visible_battle_action_cursor(runtime_shell: &mut BevyRuntimeShell) {
    let Ok(snapshot) = runtime_shell.shell.snapshot() else {
        return;
    };
    let Some(battle) = snapshot.battle.as_ref() else {
        runtime_shell.battle_action_cursor = None;
        runtime_shell.battle_move_cursor = None;
        runtime_shell.battle_switch_cursor = None;
        runtime_shell.party_move_cursor = None;
        if runtime_shell
            .bag_cursor
            .as_ref()
            .is_some_and(|cursor| cursor.surface_id == "battle:bag-items")
        {
            runtime_shell.bag_cursor = None;
        }
        return;
    };
    let actions = visible_battle_action_ids(&snapshot, battle);
    if actions.is_empty() {
        runtime_shell.battle_action_cursor = None;
        return;
    }
    visible_cursor_index(
        &mut runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    );
}

fn selected_visible_battle_action_id(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<&'static str> {
    let actions = visible_battle_action_ids(snapshot, battle);
    if actions.is_empty() {
        anyhow::bail!("active battle has no available player action");
    }
    let index = visible_cursor_index(
        &mut runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    );
    Ok(actions[index])
}

fn selected_visible_battle_action_id_readonly(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<&'static str> {
    let actions = visible_battle_action_ids(snapshot, battle);
    if actions.is_empty() {
        anyhow::bail!("active battle has no available player action");
    }
    let index = readonly_cursor_index(
        &runtime_shell.battle_action_cursor,
        "battle:actions",
        actions.len(),
    )
    .unwrap_or(0);
    Ok(actions[index])
}

fn open_visible_battle_pack(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        anyhow::bail!("no active battle");
    };
    if !battle.commands.can_use_items {
        anyhow::bail!("active battle does not allow item use");
    }
    let item_ids = carried_battle_usable_item_ids(&snapshot);
    if item_ids.is_empty() {
        anyhow::bail!("bag item pocket has no carried battle-usable item");
    }
    visible_cursor_index(
        &mut runtime_shell.bag_cursor,
        "battle:bag-items",
        item_ids.len(),
    );
    runtime_shell.ball_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened battle pack items={} selected={}",
        item_ids.len(),
        item_ids[readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "battle:bag-items",
            item_ids.len()
        )
        .unwrap_or(0)]
    ));
    Ok(())
}

fn open_visible_battle_ball_pocket(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        anyhow::bail!("no active battle");
    };
    if !battle.commands.can_use_items {
        anyhow::bail!("active battle does not allow item use");
    }
    let ball_count = snapshot
        .bag
        .balls
        .iter()
        .filter(|ball| ball.quantity > 0)
        .count();
    if ball_count == 0 {
        anyhow::bail!("bag has no carried ball");
    }
    visible_cursor_index(&mut runtime_shell.ball_cursor, "bag:balls", ball_count);
    runtime_shell.bag_cursor = None;
    runtime_shell
        .last_audio_events
        .push(format!("opened battle ball pocket balls={ball_count}"));
    Ok(())
}

fn press_visible_battle_a_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.as_ref() else {
        anyhow::bail!("no active battle");
    };
    if battle.commands.can_use_items {
        if runtime_shell.ball_cursor.is_some() {
            return throw_visible_battle_ball(runtime_shell);
        }
        if runtime_shell.bag_cursor.is_some() {
            return use_selected_visible_battle_bag_item(runtime_shell);
        }
    }
    if battle.enemy_pokemon.hp == 0 {
        return match battle.kind {
            RuntimeBattleKind::Trainer { .. } => {
                let Some(enemy_index) = battle.active_enemy_party_index else {
                    anyhow::bail!("active trainer battle has no active enemy party index");
                };
                if battle.rewarded_enemy_party_indices.contains(&enemy_index) {
                    advance_visible_trainer_battle(runtime_shell)
                } else {
                    claim_visible_battle_rewards(runtime_shell)
                }
            }
            RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => {
                claim_visible_battle_rewards(runtime_shell)
            }
        };
    }
    match selected_visible_battle_action_id(runtime_shell, &snapshot, &battle)? {
        "Fight" => resolve_visible_selected_battle_move(runtime_shell),
        "Pokemon" => switch_visible_battle_pokemon(runtime_shell),
        "Pack" => open_visible_battle_pack(runtime_shell),
        "Ball" => open_visible_battle_ball_pocket(runtime_shell),
        "Run" => press_visible_battle_b_button(runtime_shell),
        action => anyhow::bail!("unsupported battle action {action}"),
    }
}

fn press_visible_battle_b_button(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if !battle.commands.can_run {
        anyhow::bail!("active battle does not allow running");
    }
    let escape = runtime_shell.shell.attempt_escape_active_wild_battle()?;
    runtime_shell.last_audio_events.push(format!(
        "battle escape {:?} checksum={:?}",
        escape.outcome, escape.state_checksum
    ));
    Ok(())
}

fn resolve_visible_selected_battle_move(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let slot = selected_battle_move_slot(runtime_shell)?;
    resolve_visible_battle_move(runtime_shell, slot)
}

fn selected_battle_move_slot(runtime_shell: &mut BevyRuntimeShell) -> Result<usize> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if battle.commands.player_move_slots.is_empty() {
        anyhow::bail!("active battle has no available player moves");
    }
    let cursor_index = visible_cursor_index(
        &mut runtime_shell.battle_move_cursor,
        "battle:moves",
        battle.commands.player_move_slots.len(),
    );
    Ok(battle.commands.player_move_slots[cursor_index])
}

fn selected_enemy_battle_move_slot(
    snapshot: &RuntimeShellSnapshot,
    commands: &RuntimeBattleCommandSnapshot,
) -> Result<usize> {
    if commands.enemy_move_slots.is_empty() {
        anyhow::bail!("active battle has no available enemy move");
    }
    let selected_index = deterministic_enemy_move_index(snapshot, commands.enemy_move_slots.len());
    Ok(commands.enemy_move_slots[selected_index])
}

fn deterministic_enemy_move_index(snapshot: &RuntimeShellSnapshot, option_count: usize) -> usize {
    if option_count == 0 {
        return 0;
    }
    let mixed = snapshot
        .progression
        .rng_seed
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223)
        .wrapping_add(snapshot.state_checksum.hash())
        .wrapping_add(snapshot.state_checksum.frame());
    mixed as usize % option_count
}

fn use_visible_repel(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.repel_steps.is_some(),
        "selected item is not a repel",
    )?;
    let item_use = runtime_shell.shell.use_bag_repel_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field repel item={} steps={} consumed={} checksum={:?}",
        item_id, item_use.repel_steps_after, item_use.item_use.consumed, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_bicycle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "bicycle")?;
    let item_use = runtime_shell.shell.use_bag_bicycle_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field bicycle item={} mode={:?}->{:?} checksum={:?}",
        item_id, item_use.mode_before, item_use.mode_after, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_town_map(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "town_map")?;
    let item_use = runtime_shell.shell.use_bag_town_map_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field town_map item={} landmark={:?} checksum={:?}",
        item_id, item_use.landmark, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_escape_rope(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "escape_rope")?;
    let item_use = runtime_shell.shell.use_bag_escape_rope_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field escape_rope item={} destination={} warp={} checksum={:?}",
        item_id, item_use.destination_map, item_use.destination_warp_index, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_fishing_rod(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let rods = runtime_shell.shell.fishing_rod_ids();
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| rods.contains(&item.item_id),
        "selected item is not a fishing rod",
    )?;
    let item_use = runtime_shell.shell.use_bag_fishing_rod_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field fishing item={} rod={} group={:?} bite={:?} battle={:?} checksum={:?}",
        item_id,
        item_use.rod,
        item_use.cast.session.group,
        item_use.cast.bite,
        item_use.cast.wild_battle,
        item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_itemfinder(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "itemfinder")?;
    let item_use = runtime_shell.shell.use_bag_itemfinder_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field itemfinder item={} found={:?} cues={} consumed={} checksum={:?}",
        item_id,
        item_use.found,
        item_use.itemfinder_sound_cues,
        item_use.item_use.consumed,
        item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_squirtbottle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "squirtbottle")?;
    let item_use = runtime_shell
        .shell
        .use_bag_squirtbottle_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field squirtbottle item={} target={:?} movement={} script={:?} checksum={:?}",
        item_id,
        item_use.target_object_identifier,
        item_use.target_movement,
        item_use.target_script,
        item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_coin_case(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "coin_case")?;
    let item_use = runtime_shell.shell.use_bag_coin_case_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field coin_case item={} {}={} checksum={:?}",
        item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_blue_card(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "blue_card")?;
    let item_use = runtime_shell.shell.use_bag_blue_card_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field blue_card item={} {}={} checksum={:?}",
        item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_surf(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "surf",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_surf_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field surf party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_cut(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "cut",
        runtime_shell.party_cursor,
    )?;
    let (metatile_x, metatile_y) = facing_tile_u16(&snapshot)?;
    let field_move = runtime_shell
        .shell
        .use_cut_field_move(party_index, metatile_x, metatile_y)?;
    runtime_shell.last_audio_events.push(format!(
        "field cut party_index={} target=({}, {}) outcome={:?} checksum={:?}",
        party_index, metatile_x, metatile_y, field_move.outcome, field_move.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn use_visible_whirlpool(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "whirlpool",
        runtime_shell.party_cursor,
    )?;
    let (metatile_x, metatile_y) = facing_tile_u16(&snapshot)?;
    let field_move =
        runtime_shell
            .shell
            .use_whirlpool_field_move(party_index, metatile_x, metatile_y)?;
    runtime_shell.last_audio_events.push(format!(
        "field whirlpool party_index={} target=({}, {}) outcome={:?} checksum={:?}",
        party_index, metatile_x, metatile_y, field_move.outcome, field_move.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn use_visible_strength(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "strength",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_strength_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field strength party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_flash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "flash",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_flash_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field flash party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_waterfall(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "waterfall",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_waterfall_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field waterfall party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_fly(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "fly",
        runtime_shell.party_cursor,
    )?;
    let destinations = active_fly_destination_flags(&snapshot);
    if destinations.is_empty() {
        anyhow::bail!("no active FLYPOINT engine flags");
    }
    let selected_index =
        visible_cursor_index(&mut runtime_shell.fly_cursor, "fly:destinations", destinations.len());
    let flag = &destinations[selected_index];
    let field_move = runtime_shell.shell.use_fly_field_move(
        party_index,
        0,
        flag,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "field fly destination {}/{} flag={} party_index={} spawn={} map={} tile=({}, {}) checksum={:?}",
        selected_index + 1,
        destinations.len(),
        field_move.flypoint_flag,
        party_index,
        field_move.destination_spawn_identifier,
        field_move.destination_map,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    Ok(())
}

fn active_fly_destination_count(snapshot: &RuntimeShellSnapshot) -> usize {
    active_fly_destination_flags(snapshot).len()
}

fn active_fly_destination_flags(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    let mut destinations = snapshot
        .progression
        .active_engine_flags
        .iter()
        .filter(|flag| flag.contains("FLYPOINT"))
        .cloned()
        .collect::<Vec<_>>();
    destinations.sort();
    destinations
}

fn use_visible_dig(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "dig",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_dig_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field dig party_index={} destination={} warp={} tile=({}, {}) checksum={:?}",
        party_index,
        field_move.destination_map,
        field_move.destination_warp_index,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_teleport(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "teleport",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_teleport_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field teleport party_index={} destination={} spawn={} tile=({}, {}) checksum={:?}",
        party_index,
        field_move.destination_map,
        field_move.destination_spawn_identifier,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_headbutt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "headbutt",
        runtime_shell.party_cursor,
    )?;
    let player_id = snapshot.trainer.player_id;
    let field_move = runtime_shell
        .shell
        .use_headbutt_field_move(party_index, player_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field headbutt party_index={} encounter={:?} battle={:?} checksum={:?}",
        party_index, field_move.field_encounter, field_move.wild_battle, field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_rock_smash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "rock_smash",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell.shell.use_rock_smash_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field rock_smash party_index={} encounter={:?} battle={:?} checksum={:?}",
        party_index, field_move.field_encounter, field_move.wild_battle, field_move.state_checksum
    ));
    Ok(())
}

fn use_visible_sweet_scent(
    runtime_shell: &mut BevyRuntimeShell,
    surface: EncounterSurface,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "sweet_scent",
        runtime_shell.party_cursor,
    )?;
    let field_move = runtime_shell
        .shell
        .use_sweet_scent_field_move(party_index, surface)?;
    runtime_shell.last_audio_events.push(format!(
        "field sweet_scent party_index={} surface={:?} encounter={:?} battle={:?} checksum={:?}",
        party_index,
        surface,
        field_move.wild_encounter,
        field_move.wild_battle,
        field_move.state_checksum
    ));
    Ok(())
}

fn carried_field_rule_item(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
) -> Result<String> {
    let Some(item_id) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.item_id)
    else {
        anyhow::bail!("compiled pack has no field item rule {rule_id}");
    };
    if carried_item_ids(snapshot).any(|carried| carried == item_id) {
        Ok(item_id)
    } else {
        anyhow::bail!("bag does not carry field item {item_id} for rule {rule_id}")
    }
}

fn carried_item_ids(snapshot: &RuntimeShellSnapshot) -> impl Iterator<Item = &str> {
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.as_str())
}

fn sellable_carried_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect()
}

fn party_index_for_field_move_rule(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
    party_cursor: usize,
) -> Result<usize> {
    let Some(move_id) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.move_id)
    else {
        anyhow::bail!("compiled pack has no field move rule {rule_id}");
    };
    if let Some(selected) = snapshot.party.slots.get(party_cursor) {
        if selected
            .pokemon
            .moves
            .iter()
            .any(|learned| learned.name == move_id)
        {
            return Ok(selected.index);
        }
    }
    snapshot
        .party
        .slots
        .iter()
        .find(|slot| {
            slot.pokemon
                .moves
                .iter()
                .any(|learned| learned.name == move_id)
        })
        .map(|slot| slot.index)
        .with_context(|| {
            format!("party has no Pokemon with field move {move_id} for rule {rule_id}")
        })
}

fn facing_tile_u16(snapshot: &RuntimeShellSnapshot) -> Result<(u16, u16)> {
    let (dx, dy) = snapshot.overworld.facing.delta();
    let x = snapshot.overworld.tile.x + dx;
    let y = snapshot.overworld.tile.y + dy;
    if x < 0 || y < 0 {
        anyhow::bail!(
            "facing tile ({x}, {y}) is outside unsigned map coordinates for field block move"
        );
    }
    Ok((x as u16, y as u16))
}

fn switch_visible_battle_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if battle.commands.switch_party_indices.is_empty() {
        anyhow::bail!("active battle has no available party switches");
    }
    let selected_index = visible_cursor_index(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle.commands.switch_party_indices.len(),
    );
    let party_index = battle.commands.switch_party_indices[selected_index];
    switch_visible_battle_pokemon_to(runtime_shell, party_index)
}

fn switch_visible_battle_pokemon_to(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if !battle.commands.switch_party_indices.contains(&party_index) {
        anyhow::bail!("party index {party_index} is not an available battle switch");
    }
    let enemy_slot = selected_enemy_battle_move_slot(&snapshot, &battle.commands)?;
    let turn = runtime_shell.shell.resolve_active_battle_turn(
        BattleAction::Switch { party_index },
        BattleAction::Move { slot: enemy_slot },
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle switch party_index={} enemy_slot={} outcome={:?} checksum={:?}",
        party_index, enemy_slot, turn.outcome, turn.state_checksum
    ));
    Ok(())
}

fn throw_visible_battle_ball(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (ball_index, ball_id) = selected_battle_ball_id(runtime_shell)?;
    throw_visible_battle_ball_id(runtime_shell, ball_index, ball_id)
}

fn throw_visible_battle_ball_at(
    runtime_shell: &mut BevyRuntimeShell,
    ball_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    if !battle.commands.can_use_items {
        anyhow::bail!("active battle does not allow item use");
    }
    let ball = snapshot
        .bag
        .balls
        .get(ball_index)
        .with_context(|| format!("bag has no ball at index {}", ball_index + 1))?;
    let ball_id = ball.item_id.clone();
    throw_visible_battle_ball_id(runtime_shell, ball_index, ball_id)
}

fn throw_visible_battle_ball_id(
    runtime_shell: &mut BevyRuntimeShell,
    ball_index: usize,
    ball_id: String,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let scripted_static_wild = snapshot.battle.and_then(|battle| {
        if let crate::RuntimeBattleKind::StaticWild { source_script, .. } = battle.kind {
            Some((snapshot.overworld.map_name, source_script))
        } else {
            None
        }
    });
    let capture = runtime_shell.shell.throw_ball_at_active_battle(&ball_id)?;
    runtime_shell.last_audio_events.push(format!(
        "threw ball_index={} ball={} outcome={:?} checksum={:?}",
        ball_index + 1,
        ball_id,
        capture.outcome,
        capture.state_checksum
    ));
    if let Some(outcome) = capture.outcome.as_ref().filter(|outcome| outcome.caught) {
        let completion = runtime_shell.shell.complete_active_wild_capture(outcome)?;
        runtime_shell.last_audio_events.push(format!(
            "capture complete stored={:?} checksum={:?}",
            completion.stored, completion.state_checksum
        ));
        if let Some((map_name, source_script)) = scripted_static_wild {
            complete_visible_scripted_wild_battle(runtime_shell, &map_name, &source_script)?;
        }
    }
    reset_visible_battle_action_cursors(runtime_shell);
    Ok(())
}

fn claim_visible_battle_rewards(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    let map_name = snapshot.overworld.map_name.clone();
    let message = match battle.kind {
        crate::RuntimeBattleKind::Wild { .. } => {
            let rewards = runtime_shell.shell.claim_active_wild_battle_rewards()?;
            format!(
                "claimed wild rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            )
        }
        crate::RuntimeBattleKind::StaticWild { source_script, .. } => {
            let rewards = runtime_shell.shell.claim_active_wild_battle_rewards()?;
            let message = format!(
                "claimed wild rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            );
            runtime_shell.last_audio_events.push(message);
            complete_visible_scripted_wild_battle(runtime_shell, &map_name, &source_script)?;
            reset_visible_battle_action_cursors(runtime_shell);
            return Ok(());
        }
        crate::RuntimeBattleKind::Trainer { .. } => {
            let rewards = runtime_shell.shell.claim_active_trainer_battle_rewards()?;
            format!(
                "claimed trainer rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            )
        }
    };
    runtime_shell.last_audio_events.push(message);
    reset_visible_battle_action_cursors(runtime_shell);
    Ok(())
}

fn advance_visible_trainer_battle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle else {
        anyhow::bail!("no active battle");
    };
    let crate::RuntimeBattleKind::Trainer { source_script, .. } = battle.kind else {
        anyhow::bail!("active battle is not a trainer battle");
    };
    let map_name = snapshot.overworld.map_name.clone();
    let advance = runtime_shell.shell.advance_active_trainer_battle()?;
    runtime_shell.last_audio_events.push(format!(
        "advanced trainer battle defeated={} next_enemy={:?} checksum={:?}",
        advance.trainer_defeated, advance.next_enemy, advance.state_checksum
    ));
    if advance.trainer_defeated {
        complete_visible_scripted_trainer_battle(
            runtime_shell,
            &map_name,
            &source_script,
            true,
            false,
        )?;
    }
    reset_visible_battle_action_cursors(runtime_shell);
    Ok(())
}

fn buy_selected_shop_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    buy_visible_shop_cursor_item(runtime_shell)
}

fn buy_visible_shop_cursor_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(shop) = snapshot.pending_shop else {
        anyhow::bail!("no active shop");
    };
    if shop.inventory.is_empty() {
        anyhow::bail!("shop {} has no compiled inventory", shop.mart_id);
    }
    let surface_id = shop_cursor_surface_id(&shop);
    let selected_index = visible_cursor_index(
        &mut runtime_shell.menu_cursor,
        &surface_id,
        shop.inventory.len(),
    );
    buy_visible_shop_item_from_snapshot(runtime_shell, &shop, selected_index)
}

fn buy_visible_shop_item_at(
    runtime_shell: &mut BevyRuntimeShell,
    selected_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(shop) = snapshot.pending_shop else {
        anyhow::bail!("no active shop");
    };
    buy_visible_shop_item_from_snapshot(runtime_shell, &shop, selected_index)
}

fn buy_visible_shop_item_from_snapshot(
    runtime_shell: &mut BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    selected_index: usize,
) -> Result<()> {
    if shop.inventory.is_empty() {
        anyhow::bail!("shop {} has no compiled inventory", shop.mart_id);
    }
    let item_id = shop
        .inventory
        .get(selected_index)
        .cloned()
        .with_context(|| format!("shop {} has no item index {}", shop.mart_id, selected_index))?;
    let transaction = runtime_shell.shell.buy_shop_item(&item_id, 1)?;
    runtime_shell.last_audio_events.push(format!(
        "shop buy {}/{} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        shop.inventory.len(),
        item_id,
        transaction.outcome,
        transaction.state_checksum
    ));
    Ok(())
}

fn shop_cursor_surface_id(shop: &crate::core::state::ScriptShopRequest) -> String {
    format!(
        "shop:{}:{}:{}",
        shop.source_script, shop.command_index, shop.mart_id
    )
}

fn sell_selected_bag_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_none() {
        anyhow::bail!("no active shop");
    }
    let sellable = sellable_carried_item_ids(&snapshot);
    if sellable.is_empty() {
        anyhow::bail!("bag has no normal or ball pocket item to sell");
    }
    let selected_index =
        visible_cursor_index(&mut runtime_shell.sell_cursor, "sell:bag", sellable.len());
    let item_id = sellable[selected_index].clone();
    let transaction = runtime_shell.shell.sell_shop_item(&item_id, 1)?;
    runtime_shell.last_audio_events.push(format!(
        "shop sell {}/{} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        sellable.len(),
        item_id,
        transaction.outcome,
        transaction.state_checksum
    ));
    Ok(())
}

fn close_visible_shop(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let close = runtime_shell.shell.close_script_shop()?;
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "shop closed mart={} checksum={:?}",
        close.shop.mart_id, close.state_checksum
    ));
    Ok(())
}

fn close_shop_or_teleport(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_some() {
        close_visible_shop(runtime_shell)
    } else {
        use_visible_teleport(runtime_shell)
    }
}

fn run_or_rock_smash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_some() {
        let escape = runtime_shell.shell.attempt_escape_active_wild_battle()?;
        runtime_shell.last_audio_events.push(format!(
            "battle escape {:?} checksum={:?}",
            escape.outcome, escape.state_checksum
        ));
        Ok(())
    } else {
        use_visible_rock_smash(runtime_shell)
    }
}

fn execute_next_visible_queued_script_command(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let executed = runtime_shell.shell.execute_next_queued_script_command()?;
    let target = executed.queued.target.clone();
    runtime_shell.last_audio_events.push(format!(
        "script queued command={} target={} bank={:?} source={}:{} checksum={:?}",
        executed.queued.command,
        target,
        executed.queued.bank,
        executed.queued.source_script,
        executed.queued.command_index,
        executed.state_checksum
    ));
    start_visible_script_entry(runtime_shell, &target)?;
    Ok(())
}

fn take_visible_next_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let next = runtime_shell.shell.take_next_script()?;
    let script = next.script.clone();
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "script next={} checksum={:?}",
        next.script, next.state_checksum
    ));
    start_visible_script_entry(runtime_shell, &script)?;
    Ok(())
}

fn take_visible_deferred_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let deferred = runtime_shell.shell.pop_deferred_script()?;
    let script = deferred.script.clone();
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "script deferred={} checksum={:?}",
        deferred.script, deferred.state_checksum
    ));
    start_visible_script_entry(runtime_shell, &script)?;
    Ok(())
}

fn take_visible_script_end_state(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    consume_visible_script_end_state(runtime_shell)
}

fn consume_visible_script_end_state(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let end = runtime_shell.shell.take_script_end_state()?;
    let is_callback = end.end.callback;
    runtime_shell.last_audio_events.push(format!(
        "script end state source={}:{} callback={} just_battled={} checksum={:?}",
        end.end.source_script,
        end.end.command_index,
        end.end.callback,
        end.end.just_battled_guard,
        end.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    if is_callback {
        take_next_visible_map_callback(runtime_shell)?;
    }
    Ok(())
}

fn apply_visible_script_entry_command(
    runtime_shell: &mut BevyRuntimeShell,
    script: &str,
) -> Result<()> {
    let stepped = runtime_shell.shell.apply_compiled_script_command(
        script,
        0,
        explicit_compiled_script_runtime_inputs(runtime_shell, script, 0)?,
        explicit_compiled_script_phone_inputs(runtime_shell, script, 0),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script step={} command=0 result={} checksum={:?}",
        script,
        stepped.result.result_tag(),
        stepped.state_checksum
    ));
    arm_visible_script_cursor_after_outcome(runtime_shell, script, 1, &stepped);
    Ok(())
}

fn start_visible_script_entry(runtime_shell: &mut BevyRuntimeShell, script: &str) -> Result<()> {
    if has_visible_compiled_script_command(runtime_shell, script, 0) {
        apply_visible_script_entry_command(runtime_shell, script)
    } else {
        runtime_shell.active_script_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("script complete={script}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
        Ok(())
    }
}

fn arm_visible_active_script_cursor(
    runtime_shell: &mut BevyRuntimeShell,
    script: &str,
    next_command_index: usize,
) {
    if has_visible_compiled_script_command(runtime_shell, script, next_command_index) {
        runtime_shell.active_script_cursor = Some(ActiveScriptCursor {
            source_script: script.to_string(),
            next_command_index,
        });
    } else {
        runtime_shell.active_script_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("script complete={script}"));
        trim_event_log(&mut runtime_shell.last_audio_events);
    }
}

fn arm_visible_script_cursor_after_outcome(
    runtime_shell: &mut BevyRuntimeShell,
    script: &str,
    next_command_index: usize,
    outcome: &RuntimeMutationOutcome,
) {
    match &outcome.result {
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::End {
            callback,
            ..
        }) => {
            runtime_shell.active_script_cursor = None;
            if let Err(error) = consume_visible_script_end_state(runtime_shell) {
                runtime_shell.last_error = Some(error.to_string());
                return;
            }
            if *callback {
                return;
            }
            if visible_script_call_stack_has_return(runtime_shell) {
                match resume_visible_script_return(runtime_shell) {
                    Ok(()) => {}
                    Err(error) => {
                        runtime_shell.active_script_cursor = None;
                        runtime_shell.last_error = Some(error.to_string());
                    }
                }
            } else {
                runtime_shell.active_script_cursor = None;
                runtime_shell
                    .last_audio_events
                    .push(format!("script end={script}"));
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
        }
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::Jump {
            target_script,
            call,
            deferred,
            standard,
            ..
        }) => {
            runtime_shell.last_audio_events.push(format!(
                "script branch={} target={} call={} deferred={} standard={}",
                script, target_script, call, deferred, standard
            ));
            trim_event_log(&mut runtime_shell.last_audio_events);
            runtime_shell.active_script_cursor = None;
        }
        RuntimeMutationResult::ScriptControlApplied(ScriptControlAction::Continue { .. }) => {
            arm_visible_active_script_cursor(runtime_shell, script, next_command_index);
        }
        _ => arm_visible_active_script_cursor(runtime_shell, script, next_command_index),
    }
}

fn visible_script_call_stack_has_return(runtime_shell: &BevyRuntimeShell) -> bool {
    runtime_shell
        .shell
        .snapshot()
        .map(|snapshot| !snapshot.script_events.call_stack.is_empty())
        .unwrap_or(false)
}

fn resume_visible_script_return(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let returned = runtime_shell.shell.pop_script_call_stack()?;
    runtime_shell.last_audio_events.push(format!(
        "script return={} command={} checksum={:?}",
        returned.frame.source_script, returned.frame.next_command_index, returned.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    arm_visible_active_script_cursor(
        runtime_shell,
        &returned.frame.source_script,
        returned.frame.next_command_index,
    );
    Ok(())
}

fn execute_visible_active_script_step(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(cursor) = runtime_shell.active_script_cursor.clone() else {
        anyhow::bail!("no active script cursor");
    };
    if !has_visible_compiled_script_command(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index,
    ) {
        runtime_shell.active_script_cursor = None;
        runtime_shell
            .last_audio_events
            .push(format!("script complete={}", cursor.source_script));
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let stepped = runtime_shell.shell.apply_compiled_script_command(
        &cursor.source_script,
        cursor.next_command_index,
        explicit_compiled_script_runtime_inputs(
            runtime_shell,
            &cursor.source_script,
            cursor.next_command_index,
        )?,
        explicit_compiled_script_phone_inputs(
            runtime_shell,
            &cursor.source_script,
            cursor.next_command_index,
        ),
    )?;
    runtime_shell.last_audio_events.push(format!(
        "script step={} command={} result={} checksum={:?}",
        cursor.source_script,
        cursor.next_command_index,
        stepped.result.result_tag(),
        stepped.state_checksum
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    arm_visible_script_cursor_after_outcome(
        runtime_shell,
        &cursor.source_script,
        cursor.next_command_index + 1,
        &stepped,
    );
    Ok(())
}

fn has_visible_compiled_script_command(
    runtime_shell: &BevyRuntimeShell,
    script: &str,
    command_index: usize,
) -> bool {
    runtime_shell
        .shell
        .runtime()
        .compiled_script_command_name(script, command_index)
        .is_ok()
}

fn execute_last_interaction_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (frame_id, interaction) = runtime_shell
        .shell
        .last_frame()
        .and_then(|frame| {
            frame
                .interaction
                .clone()
                .map(|interaction| (frame.snapshot.frame, interaction))
        })
        .with_context(|| "no object or background interaction has been recorded")?;
    let (command_index, command) = execute_cursor_script_step(
        runtime_shell,
        ScriptCursorKind::Interaction,
        &interaction.script,
        frame_id,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "executed interaction script {} command={} target={:?} result={} checksum={:?}",
        interaction.script,
        command_index,
        interaction.target,
        command.result.result_tag(),
        command.state_checksum
    ));
    arm_visible_script_cursor_after_outcome(
        runtime_shell,
        &interaction.script,
        command_index + 1,
        &command,
    );
    Ok(())
}

fn execute_last_coord_event_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let (frame_id, coord_event) = runtime_shell
        .shell
        .last_frame()
        .and_then(|frame| {
            frame
                .coord_event
                .clone()
                .map(|coord_event| (frame.snapshot.frame, coord_event))
        })
        .with_context(|| "no coord event has been recorded")?;
    let (command_index, command) = execute_cursor_script_step(
        runtime_shell,
        ScriptCursorKind::CoordEvent,
        &coord_event.script_name,
        frame_id,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "executed coord event script {} command={} tile=({}, {}) scene={} result={} checksum={:?}",
        coord_event.script_name,
        command_index,
        coord_event.tile.x,
        coord_event.tile.y,
        coord_event.scene_id,
        command.result.result_tag(),
        command.state_checksum
    ));
    arm_visible_script_cursor_after_outcome(
        runtime_shell,
        &coord_event.script_name,
        command_index + 1,
        &command,
    );
    Ok(())
}

fn execute_visible_pending_script_warp(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let warp = runtime_shell.shell.execute_pending_script_warp()?;
    reset_visible_navigation_state(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "script warp target={} tile=({}, {}) facing={:?} checksum={:?}",
        warp.target_map, warp.tile.x, warp.tile.y, warp.facing, warp.state_checksum
    ));
    Ok(())
}

fn reset_script_cursors(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.interaction_script_cursor = None;
    runtime_shell.coord_event_script_cursor = None;
    runtime_shell.active_script_cursor = None;
    runtime_shell
        .last_audio_events
        .push("reset script cursors".to_string());
    trim_event_log(&mut runtime_shell.last_audio_events);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptCursorKind {
    Interaction,
    CoordEvent,
}

fn execute_cursor_script_step(
    runtime_shell: &mut BevyRuntimeShell,
    kind: ScriptCursorKind,
    source_script: &str,
    frame: u64,
) -> Result<(usize, RuntimeMutationOutcome)> {
    let command_index = {
        let cursor = match kind {
            ScriptCursorKind::Interaction => &mut runtime_shell.interaction_script_cursor,
            ScriptCursorKind::CoordEvent => &mut runtime_shell.coord_event_script_cursor,
        };
        let reset_cursor = match cursor.as_ref() {
            Some(cursor) => cursor.source_script != source_script || cursor.frame != frame,
            None => true,
        };
        if reset_cursor {
            *cursor = Some(ScriptCursor {
                source_script: source_script.to_string(),
                frame,
                next_command_index: 0,
            });
        }
        cursor
            .as_ref()
            .context("script cursor was not initialized")?
            .next_command_index
    };
    let command = runtime_shell.shell.apply_compiled_script_command(
        source_script,
        command_index,
        explicit_compiled_script_runtime_inputs(runtime_shell, source_script, command_index)?,
        explicit_compiled_script_phone_inputs(runtime_shell, source_script, command_index),
    )?;
    let cursor = match kind {
        ScriptCursorKind::Interaction => &mut runtime_shell.interaction_script_cursor,
        ScriptCursorKind::CoordEvent => &mut runtime_shell.coord_event_script_cursor,
    };
    cursor
        .as_mut()
        .context("script cursor was not initialized for advancement")?
        .next_command_index = command_index + 1;
    Ok((command_index, command))
}

fn quick_save(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(path) = runtime_shell.quick_save_path.clone() else {
        anyhow::bail!("F5 quick-save requires --save-path <path>");
    };
    runtime_shell.shell.save(&path)?;
    let summary = runtime_shell.shell.runtime().load_save_summary(&path)?;
    runtime_shell.last_audio_events.push(format!(
        "saved {} frame={} pack_hash={}",
        path.display(),
        summary.saved_frame(),
        summary.pack_content_hash()
    ));
    Ok(())
}

fn quick_load(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(path) = runtime_shell.quick_save_path.clone() else {
        anyhow::bail!("Ctrl+F5 quick-load requires --save-path <path>");
    };
    runtime_shell.shell.load(&path)?;
    let summary = runtime_shell.shell.runtime().load_save_summary(&path)?;
    reset_visible_navigation_state(runtime_shell);
    runtime_shell.pending_audio.clear();
    runtime_shell.last_audio_events.push(format!(
        "loaded {} frame={} pack_hash={}",
        path.display(),
        summary.saved_frame(),
        summary.pack_content_hash()
    ));
    Ok(())
}

fn reset_visible_navigation_state(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.interaction_script_cursor = None;
    runtime_shell.coord_event_script_cursor = None;
    runtime_shell.active_script_cursor = None;
    runtime_shell.script_command_cursor = 0;
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_battle_cry_key = None;
}

fn reset_visible_selection_cursors(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.menu_cursor = None;
    runtime_shell.sell_cursor = None;
    runtime_shell.party_cursor = 0;
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.tmhm_cursor = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.fly_cursor = None;
    runtime_shell.battle_action_cursor = None;
    runtime_shell.battle_move_cursor = None;
    runtime_shell.battle_switch_cursor = None;
    runtime_shell.party_move_cursor = None;
}

fn reset_visible_battle_action_cursors(runtime_shell: &mut BevyRuntimeShell) {
    reset_visible_battle_item_cursors(runtime_shell);
    runtime_shell.battle_action_cursor = None;
    runtime_shell.battle_move_cursor = None;
    runtime_shell.battle_switch_cursor = None;
}

fn reset_visible_battle_item_cursors(runtime_shell: &mut BevyRuntimeShell) {
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.party_move_cursor = None;
}

fn prepare_visible_local_link_descriptor(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let session_id = format!("bevy-local-{}", snapshot.state_checksum.frame());
    let descriptor = visible_local_link_descriptor(runtime_shell, session_id.clone())?;
    let journal = runtime_shell.shell.local_input_journal(
        &descriptor,
        descriptor.checksum.clone(),
        std::iter::empty(),
    )?;
    let journal_bytes = journal.journal.canonical_bytes()?;
    let journal_frame_count = journal.journal.frames().len();
    let journal_message = runtime_shell.shell.input_journal_message(journal.clone())?;
    let journal_message_bytes = encode_link_message_bytes(&journal_message)?;
    let save_resume_message = runtime_shell.shell.save_resume_replay_message(
        &descriptor,
        journal,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )?;
    let save_resume_message_bytes = encode_link_message_bytes(&save_resume_message)?;
    runtime_shell.last_audio_events.push(format!(
        "link descriptor session={} player={} checksum_frame={} checksum_hash={:#010x} checkpoint_frame={} journal_frames={} journal_bytes={} journal_msg_bytes={} save_resume_msg_bytes={}",
        session_id,
        descriptor.local_player.id(),
        descriptor.checksum.frame(),
        descriptor.checksum.hash(),
        descriptor.save_checkpoint.checkpoint().summary().state_frame(),
        journal_frame_count,
        journal_bytes.len(),
        journal_message_bytes.len(),
        save_resume_message_bytes.len()
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn switch_visible_next_pc_box(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let next_box = (snapshot.storage.current_pc_box + 1) % crate::core::models::MAX_PC_BOXES;
    let switched = runtime_shell.shell.switch_current_pc_box(next_box)?;
    runtime_shell.storage_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "pc box switch {}->{} checksum={:?}",
        switched.box_index_before, switched.box_index_after, switched.state_checksum
    ));
    Ok(())
}

fn deposit_visible_party_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if slot.is_active_battle_pokemon {
        anyhow::bail!(
            "selected party index {party_index} is active in battle and cannot be deposited"
        );
    }
    let deposit = runtime_shell
        .shell
        .deposit_party_pokemon_to_current_box(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "pc deposit party_index={} pokemon={} box={} slot={} checksum={:?}",
        deposit.party_index,
        deposit.pokemon.species.id,
        deposit.box_index,
        deposit.box_slot,
        deposit.state_checksum
    ));
    runtime_shell.party_cursor = 0;
    runtime_shell.storage_cursor = None;
    Ok(())
}

fn withdraw_visible_pc_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let box_slot = selected_current_box_slot_index(runtime_shell)?;
    let withdraw = runtime_shell
        .shell
        .withdraw_current_box_pokemon_to_party(box_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "pc withdraw box={} slot={} pokemon={} party_index={} checksum={:?}",
        withdraw.box_index,
        withdraw.box_slot,
        withdraw.pokemon.species.id,
        withdraw.party_index,
        withdraw.state_checksum
    ));
    runtime_shell.party_cursor = 0;
    runtime_shell.storage_cursor = None;
    Ok(())
}

fn deposit_visible_bag_item_to_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_bag_item_id(runtime_shell)?;
    let transfer = runtime_shell.shell.deposit_bag_item_to_pc(&item_id, 1)?;
    runtime_shell.last_audio_events.push(format!(
        "pc item deposit item={} quantity={} bag_after={} pc_after={} checksum={:?}",
        transfer.item_id,
        transfer.quantity,
        transfer.bag_quantity_after,
        transfer.pc_quantity_after,
        transfer.state_checksum
    ));
    runtime_shell.bag_cursor = None;
    runtime_shell.pc_item_cursor = None;
    Ok(())
}

fn withdraw_visible_pc_item_to_bag(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let item_id = selected_pc_item_id(runtime_shell)?;
    let transfer = runtime_shell.shell.withdraw_pc_item_to_bag(&item_id, 1)?;
    runtime_shell.last_audio_events.push(format!(
        "pc item withdraw item={} quantity={} bag_after={} pc_after={} checksum={:?}",
        transfer.item_id,
        transfer.quantity,
        transfer.bag_quantity_after,
        transfer.pc_quantity_after,
        transfer.state_checksum
    ));
    runtime_shell.bag_cursor = None;
    runtime_shell.pc_item_cursor = None;
    Ok(())
}

fn release_visible_current_box_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let box_slot = selected_current_box_slot_index(runtime_shell)?;
    let released = runtime_shell.shell.release_current_box_pokemon(box_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "pc release box={} slot={} pokemon={} checksum={:?}",
        released.box_index, released.box_slot, released.pokemon.species.id, released.state_checksum
    ));
    runtime_shell.storage_cursor = None;
    Ok(())
}

fn apply_visible_heal_party(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.heal_party_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special heal outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn warp_visible_to_spawn_point(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.warp_to_spawn_point()?;
    reset_visible_navigation_state(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "spawn warp outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn fade_visible_music_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.fade_out_music_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special music fade outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn wait_visible_sfx_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.wait_sfx_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special wait sfx outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn play_visible_map_music_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.play_map_music_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special play map music outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn restart_visible_map_music_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.restart_map_music_special()?;
    runtime_shell.last_audio_events.push(format!(
        "special restart map music outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn full_heal_visible_party_lead(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let recovered = runtime_shell.shell.full_heal_party_pokemon(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "party recovery slot={} species={} hp {}->{} status {:?}->{:?} pp_moves={} checksum={:?}",
        recovered.party_index,
        recovered.species_id,
        recovered.hp_before,
        recovered.hp_after,
        recovered.status_before,
        recovered.status_after,
        recovered.pp_restored.len(),
        recovered.state_checksum
    ));
    Ok(())
}

fn full_heal_visible_whole_party(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let recovered = runtime_shell.shell.full_heal_whole_party()?;
    let checksum = recovered.last().map(|entry| &entry.state_checksum);
    runtime_shell.last_audio_events.push(format!(
        "whole party recovery slots={} checksum={:?}",
        recovered.len(),
        checksum
    ));
    Ok(())
}

fn resolve_visible_blackout(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let recovered = runtime_shell.shell.resolve_blackout_to_last_spawn()?;
    reset_visible_navigation_state(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "blackout recovery spawn={} map={} tile=({}, {}) healed={} checksum={:?}",
        recovered.spawn_identifier,
        recovered.map_name,
        recovered.tile.x,
        recovered.tile.y,
        recovered.healed.len(),
        recovered.state_checksum
    ));
    Ok(())
}

fn apply_visible_pokemon_center_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.open_pokemon_center_pc_special()?;
    runtime_shell.last_audio_events.push(format!(
        "pokemon center pc outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_players_house_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.open_players_house_pc_special()?;
    runtime_shell.last_audio_events.push(format!(
        "players house pc outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_overworld_town_map(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.open_overworld_town_map_special()?;
    runtime_shell.last_audio_events.push(format!(
        "overworld town map outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_move_deletion(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let move_count = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?
        .pokemon
        .moves
        .len();
    if move_count <= 1 {
        anyhow::bail!("selected party index {party_index} has no deletable move");
    }
    let move_slot = selected_party_move_slot(runtime_shell, party_index)?;
    if move_slot >= move_count {
        anyhow::bail!("selected move slot {move_slot} is not deletable");
    }
    let special = runtime_shell
        .shell
        .delete_party_move_special(party_index, move_slot)?;
    runtime_shell.last_audio_events.push(format!(
        "move deletion party_index={} move_slot={} outcome={:?} checksum={:?}",
        party_index, move_slot, special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_name_rater(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    let special = runtime_shell
        .shell
        .rate_party_nickname_special(slot.index, slot.pokemon.nickname.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "name rater outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_move_tutor(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = selected_party_index(runtime_shell)?;
    let slot = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .with_context(|| format!("selected party index {party_index} is not in the party"))?;
    if slot.pokemon.moves.len() >= 4 {
        anyhow::bail!("selected party index {party_index} has no open move slot");
    }
    let known_moves = slot
        .pokemon
        .moves
        .iter()
        .map(|learned| learned.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let teachable_moves = snapshot
        .moves
        .iter()
        .filter(|candidate| !known_moves.contains(candidate.move_id.as_str()))
        .collect::<Vec<_>>();
    if teachable_moves.is_empty() {
        anyhow::bail!(
            "compiled move catalog has no teachable move for selected party index {party_index}"
        );
    }
    let selected_index = runtime_shell.script_command_cursor % teachable_moves.len();
    let move_id = teachable_moves[selected_index].move_id.clone();
    let special = runtime_shell
        .shell
        .teach_party_move_special(slot.index, move_id.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "move tutor {}/{} move={} party_index={} outcome={:?} checksum={:?}",
        selected_index + 1,
        teachable_moves.len(),
        move_id,
        slot.index,
        special.outcome.effect,
        special.state_checksum
    ));
    Ok(())
}

fn check_visible_pokerus(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let special = runtime_shell.shell.check_pokerus_special()?;
    runtime_shell.last_audio_events.push(format!(
        "pokerus outcome={:?} checksum={:?}",
        special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_visible_poke_seer(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let party_index = selected_party_index(runtime_shell)?;
    let special = runtime_shell.shell.see_party_pokemon_special(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "poke seer party_index={} outcome={:?} checksum={:?}",
        party_index, special.outcome.effect, special.state_checksum
    ));
    Ok(())
}

fn apply_selected_service_menu_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, routine) = selected_declared_special_routine(
        runtime_shell,
        "service menu",
        &snapshot.special.special_routines,
        &[
            "BankOfMom",
            "SlotMachine",
            "CardFlip",
            "DisplayLinkRecord",
            "TrainerHouse",
            "PhotoStudio",
            "Menu_ChallengeExplanationCancel",
        ],
    )?;
    let special = match routine {
        "BankOfMom" => runtime_shell.shell.open_bank_of_mom_special()?,
        "SlotMachine" => runtime_shell
            .shell
            .open_game_corner_special(RuntimeGameCornerService::SlotMachine)?,
        "CardFlip" => runtime_shell
            .shell
            .open_game_corner_special(RuntimeGameCornerService::CardFlip)?,
        "DisplayLinkRecord" => runtime_shell.shell.open_display_link_record_special()?,
        "TrainerHouse" => runtime_shell.shell.open_trainer_house_special()?,
        "PhotoStudio" => {
            let party_index = selected_party_index(runtime_shell)?;
            runtime_shell.shell.open_photo_studio_special(party_index)?
        }
        "Menu_ChallengeExplanationCancel" => runtime_shell
            .shell
            .cancel_battle_tower_challenge_explanation_special()?,
        _ => unreachable!("selected service routine comes from the static candidate list"),
    };
    runtime_shell.last_audio_events.push(format!(
        "service menu {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        routine,
        special.outcome.effect,
        special.state_checksum
    ));
    Ok(())
}

fn apply_selected_time_money_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, routine) = selected_declared_special_routine(
        runtime_shell,
        "time/money",
        &snapshot.special.special_routines,
        &[
            "SetDayOfWeek",
            "InitialSetDSTFlag",
            "InitialClearDSTFlag",
            "UpdateTime",
            "UnusedCheckUnusedTwoDayTimer",
            "SampleKenjiBreakCountdown",
            "CheckLuckyNumberShowFlag",
            "ResetLuckyNumberShowFlag",
            "CheckForLuckyNumberWinners",
            "PlaceMoneyTopRight",
            "DisplayMoneyAndCoinBalance",
            "DisplayCoinCaseBalance",
            "PrintTodaysLuckyNumber",
            "GSHealings",
            "StubbedTrainerRankings_Healings",
            "Reset",
            "HoOhChamber",
        ],
    )?;
    let used = runtime_shell
        .shell
        .apply_declared_special_routine(routine)?;
    runtime_shell.last_audio_events.push(format!(
        "time/money {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        routine,
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn apply_selected_story_gate_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "story gate",
        &snapshot.special.special_routines,
        &[
            RuntimeStoryGateSpecial::CheckCaughtCelebi,
            RuntimeStoryGateSpecial::CelebiShrineEvent,
            RuntimeStoryGateSpecial::SnorlaxAwake,
            RuntimeStoryGateSpecial::CheckForBattleTowerRules,
        ],
        |special| special.routine(),
    )?;
    let used = runtime_shell.shell.apply_story_gate_special(special)?;
    runtime_shell.last_audio_events.push(format!(
        "story gate {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn apply_selected_graphics_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "graphics",
        &snapshot.special.special_routines,
        &[
            RuntimeGraphicsSpecial::ClearBgPalettesBufferScreen,
            RuntimeGraphicsSpecial::ClearBgPalettes,
            RuntimeGraphicsSpecial::UpdateTimePals,
            RuntimeGraphicsSpecial::ClearTilemap,
            RuntimeGraphicsSpecial::LoadMapPalettes,
            RuntimeGraphicsSpecial::RefreshSprites,
            RuntimeGraphicsSpecial::UpdateSprites,
            RuntimeGraphicsSpecial::ReloadSpritesNoPalettes,
            RuntimeGraphicsSpecial::FadeOutToWhite,
            RuntimeGraphicsSpecial::FadeInFromWhite,
            RuntimeGraphicsSpecial::FadeOutToBlack,
            RuntimeGraphicsSpecial::FadeInFromBlack,
            RuntimeGraphicsSpecial::GameboyCheck,
            RuntimeGraphicsSpecial::CheckMobileAdapterStatus,
            RuntimeGraphicsSpecial::BattleTowerFade,
            RuntimeGraphicsSpecial::UpdatePlayerSprite,
            RuntimeGraphicsSpecial::HealMachineAnim,
            RuntimeGraphicsSpecial::SurfStartStep,
            RuntimeGraphicsSpecial::LoadUsedSpritesGfx,
            RuntimeGraphicsSpecial::ToggleMaptileDecorations,
            RuntimeGraphicsSpecial::ToggleDecorationsVisibility,
            RuntimeGraphicsSpecial::MagnetTrain,
            RuntimeGraphicsSpecial::Diploma,
            RuntimeGraphicsSpecial::PrintDiploma,
            RuntimeGraphicsSpecial::UnownPuzzle,
            RuntimeGraphicsSpecial::OmanyteChamber,
            RuntimeGraphicsSpecial::DisplayUnownWords,
        ],
        |special| special.routine(),
    )?;
    let used = runtime_shell.shell.apply_graphics_special(special)?;
    runtime_shell.last_audio_events.push(format!(
        "graphics {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn apply_selected_party_check_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "party check",
        &snapshot.special.special_routines,
        &[
            RuntimePartyCheckSpecial::CheckFirstMonIsEgg,
            RuntimePartyCheckSpecial::GetFirstPokemonHappiness,
            RuntimePartyCheckSpecial::FindPartyMonThatSpecies,
            RuntimePartyCheckSpecial::FindPartyMonAboveLevel,
            RuntimePartyCheckSpecial::FindPartyMonAtLeastThatHappy,
            RuntimePartyCheckSpecial::FindPartyMonThatSpeciesYourTrainerId,
            RuntimePartyCheckSpecial::MonCheck,
            RuntimePartyCheckSpecial::BeastsCheck,
            RuntimePartyCheckSpecial::GameCornerPrizeMonCheckDex,
            RuntimePartyCheckSpecial::UnusedSetSeenMon,
        ],
        |special| special.routine(),
    )?;
    let species_id = if special.requires_species() {
        Some(selected_pokedex_species_id(runtime_shell)?)
    } else {
        None
    };
    let threshold = if special.requires_threshold() {
        Some(((runtime_shell.script_command_cursor % 100) + 1) as u8)
    } else {
        None
    };
    let used =
        runtime_shell
            .shell
            .apply_party_check_special(special, species_id.clone(), threshold)?;
    runtime_shell.last_audio_events.push(format!(
        "party check {}/{} routine={} species={:?} threshold={:?} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        species_id,
        threshold,
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn apply_selected_phone_random_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "phone random",
        &snapshot.special.special_routines,
        &[
            RuntimePhoneRandomSpecial::RandomUnseenWildMon,
            RuntimePhoneRandomSpecial::RandomPhoneWildMon,
            RuntimePhoneRandomSpecial::RandomPhoneMon,
        ],
        |special| special.routine(),
    )?;
    let (contact_index, contact_len, contact_id) = selected_btree_key(
        runtime_shell,
        "phone contacts",
        &snapshot.special.phone_contacts.0,
    )?;
    let used = runtime_shell
        .shell
        .apply_phone_random_special(special, contact_id.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "phone random {}/{} routine={} contact={}/{} {} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        contact_index + 1,
        contact_len,
        contact_id,
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn check_selected_item_in_pc_or_bag_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot
        .special
        .special_routines
        .contains_key("UnusedFindItemInPCOrBag")
    {
        anyhow::bail!("compiled pack declares no PC/bag item check special");
    }
    let item_id = selected_bag_or_pc_item_id(runtime_shell)?;
    let used = runtime_shell
        .shell
        .check_item_in_pc_or_bag_special(item_id.clone())?;
    runtime_shell.last_audio_events.push(format!(
        "pc/bag item check item={} outcome={:?} checksum={:?}",
        item_id, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn activate_visible_fishing_swarm_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if !snapshot
        .special
        .special_routines
        .contains_key("ActivateFishingSwarm")
    {
        anyhow::bail!("compiled pack declares no fishing swarm special");
    }
    let value = ((runtime_shell.script_command_cursor % 255) + 1) as u8;
    let used = runtime_shell.shell.activate_fishing_swarm_special(value)?;
    runtime_shell.last_audio_events.push(format!(
        "fishing swarm value={} outcome={:?} checksum={:?}",
        value, used.outcome.effect, used.state_checksum
    ));
    Ok(())
}

fn apply_selected_day_care_status_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, routine) = selected_declared_special_routine(
        runtime_shell,
        "Day Care status",
        &snapshot.special.special_routines,
        &["DayCareManOutside", "DayCareMon1", "DayCareMon2"],
    )?;
    let special = match routine {
        "DayCareManOutside" => runtime_shell.shell.check_day_care_man_outside_special()?,
        "DayCareMon1" => runtime_shell
            .shell
            .check_day_care_resident_special(RuntimeDayCareCaretaker::Man)?,
        "DayCareMon2" => runtime_shell
            .shell
            .check_day_care_resident_special(RuntimeDayCareCaretaker::Lady)?,
        _ => unreachable!("selected Day Care routine comes from the static candidate list"),
    };
    runtime_shell.last_audio_events.push(format!(
        "day care status {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        routine,
        special.outcome.effect,
        special.state_checksum
    ));
    Ok(())
}

fn apply_selected_noop_special(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, special) = selected_declared_special(
        runtime_shell,
        "no-op",
        &snapshot.special.special_routines,
        &[
            RuntimeNoopSpecial::UnusedDummy,
            RuntimeNoopSpecial::UnusedBattleTowerDummy1,
            RuntimeNoopSpecial::UnusedBattleTowerDummy2,
        ],
        |special| special.routine(),
    )?;
    let used = runtime_shell.shell.apply_noop_special(special)?;
    runtime_shell.last_audio_events.push(format!(
        "noop {}/{} routine={} outcome={:?} checksum={:?}",
        selected_index + 1,
        selected_len,
        special.routine(),
        used.outcome.effect,
        used.state_checksum
    ));
    Ok(())
}

fn drain_runtime_audio_events(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    match runtime_shell.shell.drain_resolved_audio_events() {
        Ok(drain) => {
            let mut pending_audio = Vec::new();
            let events: Vec<String> = drain
                .events
                .into_iter()
                .map(|event| {
                    if let Some(command) = bevy_audio_command(&event.kind) {
                        pending_audio.push(command);
                    }
                    format!("{event:?}")
                })
                .collect();
            runtime_shell.last_audio_events.extend(events);
            trim_event_log(&mut runtime_shell.last_audio_events);
            runtime_shell.pending_audio.extend(pending_audio);
        }
        Err(error) => {
            runtime_shell.last_error = Some(error.to_string());
        }
    }
}

fn queue_battle_intro_cry(mut runtime_shell: ResMut<BevyRuntimeShell>) {
    let Ok(snapshot) = runtime_shell.shell.snapshot() else {
        return;
    };
    let Some(battle) = snapshot.battle.as_ref() else {
        runtime_shell.last_battle_cry_key = None;
        return;
    };
    let species_id = &battle.enemy_pokemon.species.id;
    let key = format!(
        "{:?}|{}|{}|{}|{:?}",
        battle.kind,
        battle.battle_type,
        species_id,
        battle.enemy_pokemon.level,
        battle.active_enemy_party_index
    );
    if runtime_shell.last_battle_cry_key.as_deref() == Some(key.as_str()) {
        return;
    }

    let Some(cry) = snapshot.presentation.pokemon_cries.get(species_id) else {
        runtime_shell.last_error = Some(format!(
            "battle species {species_id} has no pack cry metadata"
        ));
        runtime_shell.last_battle_cry_key = Some(key);
        return;
    };
    let Some(playback) = snapshot.audio_catalog.playback.cries.get(&cry.cry) else {
        runtime_shell.last_error = Some(format!(
            "battle species {species_id} cry {} has no pack playback entry",
            cry.cry
        ));
        runtime_shell.last_battle_cry_key = Some(key);
        return;
    };

    runtime_shell.pending_audio.push(BevyAudioCommand {
        audio_id: cry.cry.clone(),
        kind: ModpackAudioKind::Cry,
        mode: playback.mode,
        looped: matches!(
            playback.loop_policy,
            crate::assets::ModpackAudioLoopPolicy::Loop
        ),
    });
    runtime_shell.last_audio_events.push(format!(
        "queued battle cry {} species={species_id}",
        cry.cry
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    runtime_shell.last_battle_cry_key = Some(key);
}

fn queue_selected_music_preview(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_selected_audio_preview(runtime_shell, ModpackAudioKind::Music)
}

fn queue_selected_sound_effect_preview(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_selected_audio_preview(runtime_shell, ModpackAudioKind::SoundEffect)
}

fn queue_selected_cry_preview(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    queue_selected_audio_preview(runtime_shell, ModpackAudioKind::Cry)
}

fn queue_selected_audio_preview(
    runtime_shell: &mut BevyRuntimeShell,
    kind: ModpackAudioKind,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (selected_index, selected_len, audio_id, playback) = match kind {
        ModpackAudioKind::Music => {
            let (selected_index, selected_len, audio_id) = selected_btree_key(
                runtime_shell,
                "music playback entries",
                &snapshot.audio_catalog.playback.music,
            )?;
            let playback = snapshot
                .audio_catalog
                .playback
                .music
                .get(&audio_id)
                .with_context(|| format!("selected music playback entry {audio_id} is missing"))?
                .clone();
            (selected_index, selected_len, audio_id, playback)
        }
        ModpackAudioKind::SoundEffect => {
            let (selected_index, selected_len, audio_id) = selected_btree_key(
                runtime_shell,
                "sound-effect playback entries",
                &snapshot.audio_catalog.playback.sound_effects,
            )?;
            let playback = snapshot
                .audio_catalog
                .playback
                .sound_effects
                .get(&audio_id)
                .with_context(|| {
                    format!("selected sound-effect playback entry {audio_id} is missing")
                })?
                .clone();
            (selected_index, selected_len, audio_id, playback)
        }
        ModpackAudioKind::Cry => {
            let (selected_index, selected_len, audio_id) = selected_btree_key(
                runtime_shell,
                "cry playback entries",
                &snapshot.audio_catalog.playback.cries,
            )?;
            let playback = snapshot
                .audio_catalog
                .playback
                .cries
                .get(&audio_id)
                .with_context(|| format!("selected cry playback entry {audio_id} is missing"))?
                .clone();
            (selected_index, selected_len, audio_id, playback)
        }
    };
    runtime_shell.pending_audio.push(BevyAudioCommand {
        audio_id: audio_id.clone(),
        kind,
        mode: playback.mode,
        looped: matches!(
            playback.loop_policy,
            crate::assets::ModpackAudioLoopPolicy::Loop
        ),
    });
    runtime_shell.last_audio_events.push(format!(
        "queued {:?} preview {}/{} {} mode={:?}",
        kind,
        selected_index + 1,
        selected_len,
        audio_id,
        playback.mode
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn trim_event_log(events: &mut Vec<String>) {
    let remove_count = events.len().saturating_sub(EVENT_LOG_LIMIT);
    if remove_count > 0 {
        events.drain(0..remove_count);
    }
}

fn summarize_frame_activity(frame: &crate::RuntimeOverworldFrame) -> Option<String> {
    if let Some(wild_battle) = &frame.wild_battle {
        return Some(format!(
            "wild battle {:?} checksum={:?}",
            wild_battle, frame.state_checksum
        ));
    }
    if let Some(wild_encounter) = &frame.wild_encounter {
        return Some(format!(
            "wild encounter {:?} checksum={:?}",
            wild_encounter, frame.state_checksum
        ));
    }
    if let Some(interaction) = &frame.interaction {
        return Some(format!(
            "interaction {:?} checksum={:?}",
            interaction, frame.state_checksum
        ));
    }
    if let Some(warp) = &frame.warp {
        return Some(format!(
            "warp {:?} checksum={:?}",
            warp, frame.state_checksum
        ));
    }
    if let Some(connection) = &frame.connection {
        return Some(format!(
            "connection {:?} checksum={:?}",
            connection, frame.state_checksum
        ));
    }
    if let Some(coord_event) = &frame.coord_event {
        return Some(format!(
            "coord event {:?} checksum={:?}",
            coord_event, frame.state_checksum
        ));
    }
    frame.movement.as_ref().map(|movement| {
        format!(
            "movement {:?} tile=({}, {}) checksum={:?}",
            movement, frame.snapshot.tile.x, frame.snapshot.tile.y, frame.state_checksum
        )
    })
}

fn render_playfield(
    mut commands: Commands,
    runtime_shell: Res<BevyRuntimeShell>,
    mut rendered: ResMut<RenderedViewport>,
    tiles: Query<Entity, With<PlayfieldTile>>,
    players: Query<Entity, With<PlayerMarker>>,
    objects: Query<Entity, With<ObjectMarker>>,
    events: Query<Entity, With<EventMarker>>,
) {
    let Ok(snapshot) = runtime_shell.shell.snapshot() else {
        return;
    };
    let state_hash = snapshot.state_checksum.hash();
    if rendered.map_name.as_ref() == Some(&snapshot.overworld.map_name)
        && rendered.tile == Some(snapshot.overworld.tile)
        && rendered.state_hash == Some(state_hash)
    {
        return;
    }

    for entity in tiles.iter() {
        commands.entity(entity).despawn();
    }
    for entity in players.iter() {
        commands.entity(entity).despawn();
    }
    for entity in objects.iter() {
        commands.entity(entity).despawn();
    }
    for entity in events.iter() {
        commands.entity(entity).despawn();
    }

    let Some(map) = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
    else {
        return;
    };

    let start_x = snapshot.overworld.tile.x - VIEWPORT_TILES_X / 2;
    let start_y = snapshot.overworld.tile.y - VIEWPORT_TILES_Y / 2;
    let width = map.attributes.width as i16;
    let height = map.attributes.height as i16;
    let Some(tileset) = snapshot
        .tilesets
        .iter()
        .find(|tileset| tileset.tileset_id == map.attributes.tileset_name)
    else {
        return;
    };

    for y in 0..VIEWPORT_TILES_Y {
        for x in 0..VIEWPORT_TILES_X {
            let map_x = start_x + x;
            let map_y = start_y + y;
            let block = if map_x >= 0 && map_y >= 0 && map_x < width && map_y < height {
                let index = (map_y as usize * map.attributes.width as usize) + map_x as usize;
                map.blocks
                    .get(index)
                    .copied()
                    .unwrap_or(map.attributes.border_block as u16)
            } else {
                map.attributes.border_block as u16
            };
            let Some(color) = metatile_color(block, tileset.palette_map.get(block as usize).copied())
            else {
                continue;
            };
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color,
                        custom_size: Some(Vec2::splat(TILE_SIZE - 1.0)),
                        ..default()
                    },
                    transform: Transform::from_xyz(
                        PLAYFIELD_LEFT + x as f32 * TILE_SIZE,
                        PLAYFIELD_TOP - y as f32 * TILE_SIZE,
                        0.0,
                    ),
                    ..default()
                },
                PlayfieldTile,
            ));
        }
    }

    for (index, object) in snapshot.visible_objects.iter().enumerate() {
        let view_x = object.x as i16 - start_x;
        let view_y = object.y as i16 - start_y;
        if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
            continue;
        }

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: object_marker_color(index, object.pal),
                    custom_size: Some(Vec2::new(TILE_SIZE * 0.58, TILE_SIZE * 0.58)),
                    ..default()
                },
                transform: Transform::from_xyz(
                    PLAYFIELD_LEFT + view_x as f32 * TILE_SIZE,
                    PLAYFIELD_TOP - view_y as f32 * TILE_SIZE,
                    1.5,
                ),
                ..default()
            },
            ObjectMarker,
        ));
    }

    for warp in &map.events.warps {
        spawn_event_marker(
            &mut commands,
            start_x,
            start_y,
            warp.x,
            warp.y,
            Color::rgb(0.18, 0.42, 0.96),
            TILE_SIZE * 0.34,
            1.1,
        );
    }
    for bg in &map.events.bg_events {
        spawn_event_marker(
            &mut commands,
            start_x,
            start_y,
            bg.x,
            bg.y,
            Color::rgb(0.92, 0.92, 0.86),
            TILE_SIZE * 0.26,
            1.2,
        );
    }
    for coord in &map.events.coord_events {
        spawn_event_marker(
            &mut commands,
            start_x,
            start_y,
            coord.x,
            coord.y,
            Color::rgb(0.74, 0.42, 0.94),
            TILE_SIZE * 0.42,
            1.3,
        );
    }

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.95, 0.18, 0.16),
                custom_size: Some(Vec2::new(TILE_SIZE * 0.72, TILE_SIZE * 0.72)),
                ..default()
            },
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + (VIEWPORT_TILES_X / 2) as f32 * TILE_SIZE,
                PLAYFIELD_TOP - (VIEWPORT_TILES_Y / 2) as f32 * TILE_SIZE,
                2.0,
            ),
            ..default()
        },
        PlayerMarker,
    ));

    rendered.map_name = Some(snapshot.overworld.map_name);
    rendered.tile = Some(snapshot.overworld.tile);
    rendered.state_hash = Some(state_hash);
}

fn metatile_color(block: u16, palette: Option<u8>) -> Option<Color> {
    let palette = palette?;
    let shade = match block % 4 {
        0 => 0.00_f32,
        1 => 0.06_f32,
        2 => -0.04_f32,
        _ => 0.10_f32,
    };
    let (red, green, blue) = match palette % 8 {
        0 => (0.34_f32, 0.54_f32, 0.30_f32),
        1 => (0.28_f32, 0.42_f32, 0.62_f32),
        2 => (0.62_f32, 0.50_f32, 0.32_f32),
        3 => (0.42_f32, 0.44_f32, 0.46_f32),
        4 => (0.62_f32, 0.34_f32, 0.48_f32),
        5 => (0.28_f32, 0.58_f32, 0.58_f32),
        6 => (0.66_f32, 0.62_f32, 0.36_f32),
        _ => (0.48_f32, 0.38_f32, 0.64_f32),
    };
    Some(Color::rgb(
        (red + shade).clamp(0.0, 1.0),
        (green + shade).clamp(0.0, 1.0),
        (blue + shade).clamp(0.0, 1.0),
    ))
}

fn object_marker_color(index: usize, palette: u8) -> Color {
    match (palette as usize + index) % 6 {
        0 => Color::rgb(0.98, 0.86, 0.24),
        1 => Color::rgb(0.20, 0.78, 0.86),
        2 => Color::rgb(0.93, 0.42, 0.72),
        3 => Color::rgb(0.94, 0.60, 0.20),
        4 => Color::rgb(0.58, 0.82, 0.34),
        _ => Color::rgb(0.78, 0.68, 0.95),
    }
}

fn spawn_event_marker(
    commands: &mut Commands,
    start_x: i16,
    start_y: i16,
    tile_x: u16,
    tile_y: u16,
    color: Color,
    size: f32,
    z: f32,
) {
    let view_x = tile_x as i16 - start_x;
    let view_y = tile_y as i16 - start_y;
    if !(0..VIEWPORT_TILES_X).contains(&view_x) || !(0..VIEWPORT_TILES_Y).contains(&view_y) {
        return;
    }

    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color,
                custom_size: Some(Vec2::new(size, size)),
                ..default()
            },
            transform: Transform::from_xyz(
                PLAYFIELD_LEFT + view_x as f32 * TILE_SIZE,
                PLAYFIELD_TOP - view_y as f32 * TILE_SIZE,
                z,
            ),
            ..default()
        },
        EventMarker,
    ));
}

fn bevy_audio_command(kind: &RuntimeResolvedAudioPlaybackKind) -> Option<BevyAudioCommand> {
    match kind {
        RuntimeResolvedAudioPlaybackKind::Play { audio_id, playback } => Some(BevyAudioCommand {
            audio_id: audio_id.clone(),
            kind: playback.kind,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        }),
        RuntimeResolvedAudioPlaybackKind::FadeMusic {
            audio_id, playback, ..
        } => Some(BevyAudioCommand {
            audio_id: audio_id.clone(),
            kind: playback.kind,
            mode: playback.mode,
            looped: matches!(
                playback.loop_policy,
                crate::assets::ModpackAudioLoopPolicy::Loop
            ),
        }),
        RuntimeResolvedAudioPlaybackKind::WaitForSoundEffect => None,
    }
}

fn play_pending_audio(
    mut commands: Commands,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let pending = std::mem::take(&mut runtime_shell.pending_audio);
    for command in pending {
        let source = match command.kind {
            ModpackAudioKind::Music => runtime_shell
                .shell
                .runtime()
                .audio()
                .require_music(&command.audio_id),
            ModpackAudioKind::SoundEffect => runtime_shell
                .shell
                .runtime()
                .audio()
                .require_sound_effect(&command.audio_id),
            ModpackAudioKind::Cry => runtime_shell
                .shell
                .runtime()
                .audio()
                .require_cry(&command.audio_id),
        }
        .map(|program| program.source.clone());
        let Ok(source) = source else {
            runtime_shell.last_error = Some(format!("audio program {} missing", command.audio_id));
            continue;
        };
        let wav = match source {
            AudioProgramSource::Pcm { bytes, format } => {
                if command.mode != ModpackAudioPlaybackMode::RawPcm {
                    runtime_shell.last_audio_events.push(format!(
                        "bevy audio {} declared PCM but queued as {:?}",
                        command.audio_id, command.mode
                    ));
                }
                pcm_to_wav(
                    &bytes,
                    format.sample_rate_hz,
                    format.channels,
                    format.bits_per_sample,
                )
            }
            AudioProgramSource::Midi(bytes) => {
                if command.mode != ModpackAudioPlaybackMode::SequencedMidi {
                    runtime_shell.last_audio_events.push(format!(
                        "bevy audio {} declared MIDI but queued as {:?}",
                        command.audio_id, command.mode
                    ));
                }
                midi_to_wav(&command.audio_id, &bytes, command.kind, command.looped)
            }
        };
        let wav = match wav {
            Ok(wav) => wav,
            Err(error) => {
                runtime_shell.last_error =
                    Some(format!("audio program {} failed: {error:#}", command.audio_id));
                continue;
            }
        };
        let handle = audio_sources.add(AudioSource { bytes: wav.into() });
        let settings = if command.looped {
            PlaybackSettings::LOOP
        } else {
            PlaybackSettings::DESPAWN
        };
        commands.spawn(AudioBundle {
            source: handle,
            settings,
        });
    }
}

fn midi_to_wav(
    audio_id: &str,
    bytes: &[u8],
    kind: ModpackAudioKind,
    looped: bool,
) -> Result<Vec<u8>> {
    let program = parse_standard_midi(bytes)
        .with_context(|| format!("audio program {audio_id} is not valid Standard MIDI"))?;
    let sample_rate_hz = 44_100;
    let pcm = render_standard_midi_to_pcm(&program, sample_rate_hz, kind, looped)
        .with_context(|| format!("audio program {audio_id} could not be rendered from MIDI"))?;
    pcm_to_wav(&pcm, sample_rate_hz, 1, 16)
}

#[derive(Debug, Clone, PartialEq)]
struct StandardMidiProgram {
    ticks_per_quarter: u16,
    tempo_changes: Vec<MidiTempoChange>,
    notes: Vec<MidiNote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MidiTempoChange {
    tick: u64,
    micros_per_quarter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MidiNote {
    start_tick: u64,
    end_tick: u64,
    note: u8,
    velocity: u8,
}

fn parse_standard_midi(bytes: &[u8]) -> Result<StandardMidiProgram> {
    let mut cursor = MidiCursor::new(bytes);
    cursor.read_tag(b"MThd")?;
    let header_len = cursor.read_u32()? as usize;
    if header_len != 6 {
        anyhow::bail!("MIDI header length {header_len} is not 6");
    }
    let format = cursor.read_u16()?;
    let track_count = cursor.read_u16()?;
    let division = cursor.read_u16()?;
    if format > 1 {
        anyhow::bail!("MIDI format {format} is not supported");
    }
    if format == 0 && track_count != 1 {
        anyhow::bail!("MIDI format 0 must contain exactly one track");
    }
    if track_count == 0 {
        anyhow::bail!("MIDI has no tracks");
    }
    if division & 0x8000 != 0 {
        anyhow::bail!("SMPTE MIDI timing is not supported");
    }
    if division == 0 {
        anyhow::bail!("MIDI ticks_per_quarter must be positive");
    }

    let mut tempo_changes = vec![MidiTempoChange {
        tick: 0,
        micros_per_quarter: 500_000,
    }];
    let mut notes = Vec::new();
    for _ in 0..track_count {
        cursor.read_tag(b"MTrk")?;
        let track_len = cursor.read_u32()? as usize;
        let track_end = cursor
            .position()
            .checked_add(track_len)
            .context("MIDI track length overflow")?;
        if track_end > bytes.len() {
            anyhow::bail!("MIDI track length exceeds file length");
        }
        parse_midi_track(
            bytes,
            cursor.position(),
            track_end,
            &mut tempo_changes,
            &mut notes,
        )?;
        cursor.set_position(track_end)?;
    }
    if notes.is_empty() {
        anyhow::bail!("MIDI contains no note events");
    }
    tempo_changes.sort_by_key(|tempo| tempo.tick);
    tempo_changes.dedup_by_key(|tempo| tempo.tick);
    notes.sort_by_key(|note| (note.start_tick, note.note, note.end_tick));
    Ok(StandardMidiProgram {
        ticks_per_quarter: division,
        tempo_changes,
        notes,
    })
}

fn parse_midi_track(
    bytes: &[u8],
    start: usize,
    end: usize,
    tempo_changes: &mut Vec<MidiTempoChange>,
    notes: &mut Vec<MidiNote>,
) -> Result<()> {
    let mut cursor = MidiCursor::new(&bytes[start..end]);
    let mut absolute_tick = 0u64;
    let mut running_status: Option<u8> = None;
    let mut active: BTreeMap<(u8, u8), Vec<(u64, u8)>> = BTreeMap::new();

    while cursor.position() < cursor.len() {
        absolute_tick = absolute_tick
            .checked_add(cursor.read_var_len()? as u64)
            .context("MIDI absolute tick overflow")?;
        let first = cursor.read_u8()?;
        let status = if first & 0x80 != 0 {
            first
        } else {
            running_status.with_context(|| "MIDI running status used before status byte")?
        };
        let first_data = if first & 0x80 == 0 { Some(first) } else { None };

        match status {
            0xff => {
                running_status = None;
                let meta_type = cursor.read_u8()?;
                let len = cursor.read_var_len()? as usize;
                if meta_type == 0x51 {
                    if len != 3 {
                        anyhow::bail!("MIDI tempo meta event length {len} is not 3");
                    }
                    let b0 = u32::from(cursor.read_u8()?);
                    let b1 = u32::from(cursor.read_u8()?);
                    let b2 = u32::from(cursor.read_u8()?);
                    tempo_changes.push(MidiTempoChange {
                        tick: absolute_tick,
                        micros_per_quarter: (b0 << 16) | (b1 << 8) | b2,
                    });
                } else {
                    cursor.skip(len)?;
                    if meta_type == 0x2f {
                        break;
                    }
                }
            }
            0xf0 | 0xf7 => {
                running_status = None;
                let len = cursor.read_var_len()? as usize;
                cursor.skip(len)?;
            }
            0x80..=0xef => {
                running_status = Some(status);
                let channel = status & 0x0f;
                let command = status & 0xf0;
                let data_len = midi_channel_data_len(command)?;
                let data1 = match first_data {
                    Some(value) => value,
                    None => cursor.read_u8()?,
                };
                let data2 = if data_len == 2 {
                    Some(cursor.read_u8()?)
                } else {
                    None
                };
                match (command, data2) {
                    (0x80, Some(velocity)) => {
                        close_midi_note(&mut active, notes, channel, data1, absolute_tick, velocity);
                    }
                    (0x90, Some(0)) => {
                        close_midi_note(&mut active, notes, channel, data1, absolute_tick, 0);
                    }
                    (0x90, Some(velocity)) => {
                        active
                            .entry((channel, data1))
                            .or_default()
                            .push((absolute_tick, velocity));
                    }
                    _ => {}
                }
            }
            _ => anyhow::bail!("invalid MIDI status byte {status:#04x}"),
        }
    }
    Ok(())
}

fn midi_channel_data_len(command: u8) -> Result<usize> {
    match command {
        0xc0 | 0xd0 => Ok(1),
        0x80 | 0x90 | 0xa0 | 0xb0 | 0xe0 => Ok(2),
        _ => anyhow::bail!("invalid MIDI channel command {command:#04x}"),
    }
}

fn close_midi_note(
    active: &mut BTreeMap<(u8, u8), Vec<(u64, u8)>>,
    notes: &mut Vec<MidiNote>,
    channel: u8,
    note: u8,
    end_tick: u64,
    _release_velocity: u8,
) {
    let Some(stack) = active.get_mut(&(channel, note)) else {
        return;
    };
    let Some((start_tick, velocity)) = stack.pop() else {
        return;
    };
    if stack.is_empty() {
        active.remove(&(channel, note));
    }
    if end_tick > start_tick {
        notes.push(MidiNote {
            start_tick,
            end_tick,
            note,
            velocity,
        });
    }
}

fn render_standard_midi_to_pcm(
    program: &StandardMidiProgram,
    sample_rate_hz: u32,
    kind: ModpackAudioKind,
    looped: bool,
) -> Result<Vec<u8>> {
    let last_tick = program
        .notes
        .iter()
        .map(|note| note.end_tick)
        .max()
        .context("MIDI contains no notes")?;
    let natural_seconds = midi_tick_to_seconds(program, last_tick)?;
    let max_seconds = match kind {
        ModpackAudioKind::Music if looped => 12.0,
        ModpackAudioKind::Music => 8.0,
        ModpackAudioKind::SoundEffect | ModpackAudioKind::Cry => 3.0,
    };
    let duration_seconds = natural_seconds.min(max_seconds).max(0.05);
    let sample_count = (duration_seconds * f64::from(sample_rate_hz)).ceil() as usize;
    let mut mix = vec![0.0f32; sample_count];
    for note in &program.notes {
        let start = (midi_tick_to_seconds(program, note.start_tick)? * f64::from(sample_rate_hz))
            as usize;
        let end = (midi_tick_to_seconds(program, note.end_tick)? * f64::from(sample_rate_hz))
            .ceil() as usize;
        if start >= sample_count {
            continue;
        }
        let end = end.min(sample_count).max(start + 1);
        let frequency = 440.0f32 * 2.0f32.powf((f32::from(note.note) - 69.0) / 12.0);
        let gain = (f32::from(note.velocity) / 127.0) * 0.18;
        for (offset, sample) in mix[start..end].iter_mut().enumerate() {
            let t = offset as f32 / sample_rate_hz as f32;
            let phase = (t * frequency).fract();
            let wave = if phase < 0.5 { 1.0 } else { -1.0 };
            let release = 1.0 - (offset as f32 / (end - start) as f32).powf(4.0) * 0.18;
            *sample += wave * gain * release;
        }
    }
    let mut pcm = Vec::with_capacity(sample_count * 2);
    for sample in mix {
        let scaled = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        pcm.extend_from_slice(&scaled.to_le_bytes());
    }
    Ok(pcm)
}

fn midi_tick_to_seconds(program: &StandardMidiProgram, tick: u64) -> Result<f64> {
    let mut seconds = 0.0f64;
    let mut last_tick = 0u64;
    let mut tempo = 500_000u32;
    for change in &program.tempo_changes {
        if change.tick > tick {
            break;
        }
        let delta = change.tick.saturating_sub(last_tick);
        seconds += midi_delta_seconds(delta, tempo, program.ticks_per_quarter)?;
        last_tick = change.tick;
        tempo = change.micros_per_quarter;
    }
    seconds += midi_delta_seconds(tick.saturating_sub(last_tick), tempo, program.ticks_per_quarter)?;
    Ok(seconds)
}

fn midi_delta_seconds(
    delta_ticks: u64,
    micros_per_quarter: u32,
    ticks_per_quarter: u16,
) -> Result<f64> {
    if ticks_per_quarter == 0 {
        anyhow::bail!("MIDI ticks_per_quarter must be positive");
    }
    Ok(delta_ticks as f64 * f64::from(micros_per_quarter)
        / f64::from(ticks_per_quarter)
        / 1_000_000.0)
}

struct MidiCursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> MidiCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn position(&self) -> usize {
        self.position
    }

    fn set_position(&mut self, position: usize) -> Result<()> {
        if position > self.bytes.len() {
            anyhow::bail!("MIDI cursor position exceeds byte length");
        }
        self.position = position;
        Ok(())
    }

    fn read_tag(&mut self, expected: &[u8; 4]) -> Result<()> {
        let actual = self.read_exact(4)?;
        if actual != expected {
            anyhow::bail!("MIDI chunk tag is not {}", String::from_utf8_lossy(expected));
        }
        Ok(())
    }

    fn read_u8(&mut self) -> Result<u8> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_var_len(&mut self) -> Result<u32> {
        let mut value = 0u32;
        for _ in 0..4 {
            let byte = self.read_u8()?;
            value = (value << 7) | u32::from(byte & 0x7f);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        anyhow::bail!("MIDI variable-length integer exceeds 4 bytes")
    }

    fn skip(&mut self, len: usize) -> Result<()> {
        self.read_exact(len).map(|_| ())
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(len)
            .context("MIDI cursor overflow")?;
        if end > self.bytes.len() {
            anyhow::bail!("MIDI ended inside chunk");
        }
        let slice = &self.bytes[self.position..end];
        self.position = end;
        Ok(slice)
    }
}

fn pcm_to_wav(
    bytes: &[u8],
    sample_rate_hz: u32,
    channels: u8,
    bits_per_sample: u8,
) -> Result<Vec<u8>> {
    if sample_rate_hz == 0 {
        anyhow::bail!("PCM sample_rate_hz must be positive");
    }
    if channels == 0 {
        anyhow::bail!("PCM channels must be positive");
    }
    if bits_per_sample != 8 && bits_per_sample != 16 {
        anyhow::bail!("PCM bits_per_sample must be 8 or 16");
    }
    let channels_u16 = channels as u16;
    let bytes_per_sample = u16::from(bits_per_sample) / 8;
    let block_align = channels_u16
        .checked_mul(bytes_per_sample)
        .context("PCM block_align overflow")?;
    let byte_rate = sample_rate_hz
        .checked_mul(u32::from(block_align))
        .context("PCM byte_rate overflow")?;
    let data_len = u32::try_from(bytes.len()).context("PCM data exceeds WAV length field")?;
    if bytes.len() % usize::from(block_align) != 0 {
        anyhow::bail!("PCM byte length is not aligned to frame size");
    }
    let riff_len = 36u32
        .checked_add(data_len)
        .context("PCM RIFF length overflow")?;
    let mut wav = Vec::with_capacity(44 + bytes.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_len.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&channels_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate_hz.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&u16::from(bits_per_sample).to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    wav.extend_from_slice(bytes);
    Ok(wav)
}

fn refresh_status_text(
    runtime_shell: Res<BevyRuntimeShell>,
    mode: Res<HudMode>,
    mut query: Query<&mut Text, With<StatusText>>,
) {
    let mut text = query.single_mut();
    text.sections[0].value = match runtime_shell.shell.snapshot() {
        Ok(snapshot) => format_snapshot(&snapshot, &runtime_shell, *mode),
        Err(error) => format!("snapshot error: {error}"),
    };
}

fn refresh_dialog_text(
    runtime_shell: Res<BevyRuntimeShell>,
    mut query: Query<&mut Text, With<DialogText>>,
) {
    let mut text = query.single_mut();
    text.sections[0].value = match runtime_shell.shell.snapshot() {
        Ok(snapshot) => format_dialog_overlay(&snapshot, &runtime_shell),
        Err(_) => String::new(),
    };
}

fn refresh_battle_text(
    runtime_shell: Res<BevyRuntimeShell>,
    mut query: Query<&mut Text, With<BattleText>>,
) {
    let mut text = query.single_mut();
    text.sections[0].value = match runtime_shell.shell.snapshot() {
        Ok(snapshot) => format_battle_overlay(&snapshot, &runtime_shell),
        Err(_) => String::new(),
    };
}

fn format_dialog_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let mut lines = Vec::new();
    if let Some(text) = &snapshot.ui.text {
        lines.push(format!("{}:", text.label));
        if let Some(asm_text) = &text.asm_text {
            lines.push(asm_text.clone());
        } else if let Some(body) = &text.body {
            for command in body.commands.iter().take(4) {
                lines.push(format!("{} {}", command.command, command.args.join(" ")));
            }
        }
    }
    if let Some(prompt) = &snapshot.ui.pending_yes_no {
        lines.push(format!(
            "yes/no {}:{}",
            prompt.source_script, prompt.command_index
        ));
    }
    if let Some(wait) = &snapshot.ui.pending_text_wait {
        lines.push(format!(
            "wait {}:{} {}",
            wait.source_script, wait.command_index, wait.command
        ));
    }
    if let Some(menu) = &snapshot.ui.menu {
        lines.push(format!("menu {}", menu.menu_id));
        for vertical in &menu.layout.vertical_menus {
            let surface_id = vertical_menu_surface_id(menu, vertical);
            let cursor_index = runtime_shell
                .menu_cursor
                .as_ref()
                .filter(|cursor| cursor.surface_id == surface_id)
                .map(|cursor| cursor.option_index)
                .unwrap_or(0);
            let options = vertical
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    if index == cursor_index {
                        format!("> {option}")
                    } else {
                        format!("  {option}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" / ");
            lines.push(options);
        }
    }
    if let Some(shop) = &snapshot.pending_shop {
        let surface_id = shop_cursor_surface_id(shop);
        let cursor_index = runtime_shell
            .menu_cursor
            .as_ref()
            .filter(|cursor| cursor.surface_id == surface_id)
            .map(|cursor| cursor.option_index)
            .unwrap_or(0);
        lines.push(format!("shop {} {}", shop.mart_type, shop.mart_id));
        let inventory = shop
            .inventory
            .iter()
            .enumerate()
            .map(|(index, item)| {
                if index == cursor_index {
                    format!("> {item}")
                } else {
                    format!("  {item}")
                }
            })
            .collect::<Vec<_>>()
            .join(" / ");
        lines.push(inventory);
        let sellable = sellable_carried_item_ids(snapshot);
        if !sellable.is_empty() {
            let sell_cursor = runtime_shell
                .sell_cursor
                .as_ref()
                .filter(|cursor| cursor.surface_id == "sell:bag")
                .map(|cursor| cursor.option_index)
                .unwrap_or(0);
            let sell_options = sellable
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    if index == sell_cursor {
                        format!("> {item}")
                    } else {
                        format!("  {item}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" / ");
            lines.push(format!("sell {sell_options}"));
        }
    }
    if lines.is_empty() {
        append_field_overlay(snapshot, runtime_shell, &mut lines);
    }
    lines.join("\n")
}

fn append_field_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "{}  tile=({}, {}) facing={:?}",
        snapshot.overworld.map_name,
        snapshot.overworld.tile.x,
        snapshot.overworld.tile.y,
        snapshot.overworld.facing
    ));
    let selected_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    if let Some(lead) = snapshot.party.slots.first() {
        lines.push(format!(
            "lead {} L{} HP {}/{} status={:?}",
            lead.pokemon.species.id,
            lead.pokemon.level,
            lead.pokemon.hp,
            lead.pokemon.max_hp,
            lead.pokemon.status
        ));
    }
    if selected_slot > 0 {
        if let Some(selected) = snapshot.party.slots.get(selected_slot) {
            lines.push(format!(
                "selected {} {} L{} HP {}/{} status={:?}",
                selected.index,
                selected.pokemon.species.id,
                selected.pokemon.level,
                selected.pokemon.hp,
                selected.pokemon.max_hp,
                selected.pokemon.status
            ));
        }
    }
    append_party_roster_overlay(snapshot, selected_slot, lines);
    append_fly_destination_overlay(snapshot, runtime_shell, lines);
    if snapshot.progression.repel_steps_remaining > 0 {
        lines.push(format!(
            "repel {} steps via {:?}",
            snapshot.progression.repel_steps_remaining, snapshot.progression.active_repel_item
        ));
    }
    let has_wild = snapshot
        .encounters
        .wild
        .contains_key(&snapshot.overworld.map_name);
    let has_field = snapshot
        .encounters
        .field
        .contains_key(&snapshot.overworld.map_name);
    lines.push(format!(
        "music={:?} encounters wild={} field={}",
        snapshot.audio.current_music, has_wild, has_field
    ));
    lines.push(format!(
        "bag items={} balls={} key={} tm_hm={}",
        snapshot.bag.items.len(),
        snapshot.bag.balls.len(),
        snapshot.bag.key_items.len(),
        snapshot.bag.tm_hm.len()
    ));
    lines.push(format!(
        "money={} coins={} time={:?}",
        snapshot.trainer.money, snapshot.trainer.coins, snapshot.progression.time
    ));
    append_runtime_request_overlay(snapshot, lines);
    if let Some(front) = front_context_line(snapshot) {
        lines.push(front);
    }
}

fn append_fly_destination_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let destinations = active_fly_destination_flags(snapshot);
    if destinations.is_empty() {
        return;
    }
    let selected_index = readonly_cursor_index(
        &runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    )
    .unwrap_or(0);
    let flag = &destinations[selected_index];
    lines.push(format!(
        "fly {}/{} flag={}",
        selected_index + 1,
        destinations.len(),
        flag
    ));
}

fn append_party_roster_overlay(
    snapshot: &RuntimeShellSnapshot,
    selected_slot: usize,
    lines: &mut Vec<String>,
) {
    if snapshot.party.slots.len() <= 1 {
        return;
    }
    let summary = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(slot_offset, slot)| {
            let marker = if slot_offset == selected_slot {
                ">"
            } else {
                ""
            };
            format!(
                "{}{}:{} {}/{}",
                marker, slot.index, slot.pokemon.species.id, slot.pokemon.hp, slot.pokemon.max_hp
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    lines.push(format!("party {summary}"));
}

fn append_runtime_request_overlay(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    let scripts = &snapshot.script_events;
    if scripts.hall_of_fame_requested {
        lines.push("request=hall_of_fame".to_string());
    }
    if scripts.credits_requested {
        lines.push("request=credits".to_string());
    }
    if scripts.reset_requested {
        lines.push("request=reset".to_string());
    }
    if let Some(blackout_mod) = &scripts.blackout_mod {
        lines.push(format!("blackout_mod={blackout_mod}"));
    }
    if let Some(text) = &scripts.battle_tower_text {
        lines.push(format!("battle_tower_text={text}"));
    }
    if let Some(picture) = &snapshot.ui.active_pokemon_picture {
        lines.push(format!("pokemon_picture={picture}"));
    }
    if !snapshot.ui.gift_pokemon.is_empty() {
        let selected = runtime_shell.script_command_cursor % snapshot.ui.gift_pokemon.len();
        let gift = &snapshot.ui.gift_pokemon[selected];
        lines.push(format!(
            "gift_pokemon_selected={}/{} {} Lv{}",
            selected + 1,
            snapshot.ui.gift_pokemon.len(),
            gift.species_id,
            gift.level
        ));
    }
    if !snapshot.ui.elevators.is_empty() {
        let selected = runtime_shell.script_command_cursor % snapshot.ui.elevators.len();
        lines.push(format!(
            "elevator_selected={}/{} {}",
            selected + 1,
            snapshot.ui.elevators.len(),
            snapshot.ui.elevators[selected].data_label
        ));
    }
    if !scripts.completed_trades.is_empty() {
        lines.push(format!("completed_trades={:?}", scripts.completed_trades));
    }
    if !scripts.catch_tutorials.is_empty() {
        lines.push(format!("catch_tutorials={:?}", scripts.catch_tutorials));
    }
}

fn front_context_line(snapshot: &RuntimeShellSnapshot) -> Option<String> {
    let (dx, dy) = snapshot.overworld.facing.delta();
    let front_x = snapshot.overworld.tile.x + dx;
    let front_y = snapshot.overworld.tile.y + dy;
    if let Some(object) = snapshot
        .visible_objects
        .iter()
        .find(|object| tile_matches(object.x, object.y, front_x, front_y))
    {
        return Some(format!(
            "front object {:?} sprite={} script={}",
            object.object_identifier, object.sprite, object.script
        ));
    }
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)?;
    if let Some(bg) = map
        .events
        .bg_events
        .iter()
        .find(|event| tile_matches(event.x, event.y, front_x, front_y))
    {
        return Some(format!("front {} script={}", bg.event_type, bg.script));
    }
    if let Some(warp) = map.events.warps.iter().find(|warp| {
        tile_matches(
            warp.x,
            warp.y,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y,
        )
    }) {
        return Some(format!(
            "standing warp {} -> {}#{}",
            warp.index, warp.target_map, warp.target_warp_id
        ));
    }
    None
}

fn format_battle_overlay(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let Some(battle) = &snapshot.battle else {
        return String::new();
    };
    let active_player = battle
        .active_player_party_index
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index));
    let mut lines = vec![
        format!("{:?} {}", battle.kind, battle.battle_type),
        format!(
            "Enemy {} L{} HP {}/{}",
            battle.enemy_pokemon.species.id,
            battle.enemy_pokemon.level,
            battle.enemy_pokemon.hp,
            battle.enemy_pokemon.max_hp
        ),
    ];
    if let Some(slot) = active_player {
        lines.push(format!(
            "Player {} L{} HP {}/{}",
            slot.pokemon.species.id, slot.pokemon.level, slot.pokemon.hp, slot.pokemon.max_hp
        ));
        let selected_move_slot = readonly_cursor_index(
            &runtime_shell.battle_move_cursor,
            "battle:moves",
            battle.commands.player_move_slots.len(),
        )
        .and_then(|cursor_index| battle.commands.player_move_slots.get(cursor_index).copied());
        let moves = slot
            .pokemon
            .moves
            .iter()
            .enumerate()
            .map(|(index, learned)| {
                let marker = if Some(index) == selected_move_slot {
                    ">"
                } else {
                    ""
                };
                format!(
                    "{}{} {} pp={} up={}",
                    marker,
                    index + 1,
                    learned.name,
                    learned.current_pp,
                    learned.pp_ups
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        if !moves.is_empty() {
            lines.push(moves);
        }
    }
    let selected_party_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    if let Some(selected) = snapshot.party.slots.get(selected_party_slot) {
        lines.push(format!(
            "Selected party {} {} HP {}/{}",
            selected.index,
            selected.pokemon.species.id,
            selected.pokemon.hp,
            selected.pokemon.max_hp
        ));
    }
    let player_party = snapshot
        .party
        .slots
        .iter()
        .enumerate()
        .map(|(slot_offset, slot)| {
            format!(
                "{}{}{}:{} {}/{}",
                if slot_offset == selected_party_slot {
                    ">"
                } else {
                    ""
                },
                if slot.is_active_battle_pokemon {
                    "*"
                } else {
                    ""
                },
                slot.index,
                slot.pokemon.species.id,
                slot.pokemon.hp,
                slot.pokemon.max_hp
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    if !player_party.is_empty() {
        lines.push(format!("Player party {player_party}"));
    }
    if !battle.enemy_party.is_empty() {
        let enemy_party = battle
            .enemy_party
            .iter()
            .enumerate()
            .map(|(index, pokemon)| {
                let active = Some(index) == battle.active_enemy_party_index;
                let rewarded = battle.rewarded_enemy_party_indices.contains(&index);
                format!(
                    "{}{}:{} L{} {}/{}{}",
                    if active { "*" } else { "" },
                    index,
                    pokemon.species.id,
                    pokemon.level,
                    pokemon.hp,
                    pokemon.max_hp,
                    if rewarded { " done" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("Enemy party {enemy_party}"));
    }
    lines.push(format!(
        "run={} escape_attempts={} guard={} switch={:?} items={} balls={}",
        battle.commands.can_run,
        battle.escape_attempts,
        battle.player_stat_drop_guard_turns,
        battle.commands.switch_party_indices,
        battle.commands.can_use_items,
        snapshot.bag.balls.len()
    ));
    append_battle_cursor_context(snapshot, runtime_shell, &mut lines);
    lines.push(
        "controls arrows=battle action/item cursor | Z/A=select | X/B=cancel/run | 1-4 direct move"
            .to_string(),
    );
    if !snapshot.bag.balls.is_empty() {
        let balls = snapshot
            .bag
            .balls
            .iter()
            .enumerate()
            .map(|(index, ball)| format!("{}:{}x{}", index + 1, ball.item_id, ball.quantity))
            .collect::<Vec<_>>()
            .join(" | ");
        lines.push(format!("Balls {balls}"));
    }
    lines.push(format!(
        "move_slots player={:?} enemy={:?}",
        battle.commands.player_move_slots, battle.commands.enemy_move_slots
    ));
    lines.join("\n")
}

fn append_battle_cursor_context(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let battle = snapshot.battle.as_ref();
    let actions = battle
        .map(|battle| visible_battle_action_ids(snapshot, battle))
        .unwrap_or_default();
    let selected_action = if actions.is_empty() {
        "-"
    } else {
        readonly_cursor_index(
            &runtime_shell.battle_action_cursor,
            "battle:actions",
            actions.len(),
        )
        .and_then(|index| actions.get(index).copied())
        .unwrap_or(actions[0])
    };
    let battle_items = carried_battle_usable_item_ids(snapshot);
    let selected_item = if battle_items.is_empty() {
        "-"
    } else {
        readonly_cursor_index(
            &runtime_shell.bag_cursor,
            "battle:bag-items",
            battle_items.len(),
        )
        .and_then(|index| battle_items.get(index))
        .map(|item| item.as_str())
        .unwrap_or(battle_items[0].as_str())
    };
    let ball_items = carried_ball_item_ids(snapshot);
    let selected_ball_index =
        readonly_cursor_index(&runtime_shell.ball_cursor, "bag:balls", ball_items.len());
    let selected_ball = selected_ball_index
        .and_then(|index| ball_items.get(index))
        .map(|ball| ball.as_str())
        .unwrap_or("-");
    let selected_move = battle
        .and_then(|battle| {
            readonly_cursor_index(
                &runtime_shell.battle_move_cursor,
                "battle:moves",
                battle.commands.player_move_slots.len(),
            )
            .and_then(|cursor_index| battle.commands.player_move_slots.get(cursor_index).copied())
        })
        .map(|slot| (slot + 1).to_string())
        .unwrap_or_else(|| "-".to_string());
    let selected_party_move = selected_party_move_name(snapshot, runtime_shell);
    lines.push(format!(
        "selected action={} move={} party_move={} item={} ball={}",
        selected_action, selected_move, selected_party_move, selected_item, selected_ball
    ));
}

fn format_snapshot(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    mode: HudMode,
) -> String {
    let mut lines = vec![
        format!(
            "Pokemon Crystal Rust  pack={} hash={}",
            snapshot.boot.modpack_id, snapshot.boot.pack_content_hash
        ),
        format!(
            "frame={} phase={:?} checksum={:?}",
            snapshot.overworld.frame, snapshot.phase, snapshot.state_checksum
        ),
        format!(
            "map={} tile=({}, {}) facing={:?} mode={:?}",
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y,
            snapshot.overworld.facing,
            snapshot.overworld.mode
        ),
        format!(
            "player={} money={} coins={} badges={:?}",
            snapshot.trainer.player_name,
            snapshot.trainer.money,
            snapshot.trainer.coins,
            snapshot.progression.badges
        ),
        format!(
            "party={} bag={} balls={} key={} tm_hm={}",
            snapshot.party.slots.len(),
            snapshot.bag.items.len(),
            snapshot.bag.balls.len(),
            snapshot.bag.key_items.len(),
            snapshot.bag.tm_hm.len()
        ),
        "input: arrows move, Z=A, X=B, Enter=Start, Right Shift=Select".to_string(),
        "runtime: Z=accept/text/script/menu/shop buy, X=no/close, Up/Down=menu/shop cursor"
            .to_string(),
        "runtime: Shift+Up/Down=sell cursor, [/]=battle move cursor, Ctrl+[/]=party move cursor, L=sell selected, Space=text, Y/N=yes/no"
            .to_string(),
        "runtime: Esc=close, 1-9=menu/shop/battle, Ctrl+1-9=linked menu"
            .to_string(),
        "runtime: Shift+Z=execute queued script, Shift+Enter=take next script".to_string(),
        "runtime: Shift+Space=text label, Shift+PageDown=map load, Shift+PageUp=map refresh"
            .to_string(),
        "runtime: Shift+Home=music fade, Shift+End=screen fade, Ctrl+Shift+PgUp/PgDn=PC item cursor, Alt+Shift+Home/End=item PC deposit selected/withdraw selected"
            .to_string(),
        "runtime: Shift+Insert=take shop, Alt+Shift+Insert=open shop, Shift+E=elevator, Shift+X=clear menu coords"
            .to_string(),
        "save: F5=quick save, Ctrl+F5=quick load, both require --save-path <slot.crystalsave>"
            .to_string(),
        "runtime scripts: Shift+U=step interaction, Shift+0=step coord-event, Shift+J=script warp, Alt+Shift+J=map command"
            .to_string(),
        "runtime scripts: Shift+-=reset cursors, Alt+Shift+[/]=command cursor"
            .to_string(),
        "runtime drains: Shift+A=audio, Shift+M=map, Shift+T=text, Shift+Backspace=events, Shift+D=delay, Shift+\\=queues, Shift+R=records, Shift+F=flag"
            .to_string(),
        "audio: Ctrl+Alt+Shift+,/S/Y=selected music/SFX/cry preview".to_string(),
        "runtime link: Shift+Delete=local descriptor/checkpoint/input journal, Ctrl+1-9=local linked menu result".to_string(),
        "progress: Ctrl+Shift+B=award selected badge, Ctrl+Shift+P=Pokedex seen, Ctrl+Shift+O=Pokedex caught"
            .to_string(),
        "records: Ctrl+Shift+M=money +1000, Ctrl+Alt+Shift+M=money -100, Ctrl+Shift+C=coins +100, Ctrl+Shift+L=link win, Ctrl+Alt+Shift+L=link loss, Ctrl+Shift+D=link draw"
            .to_string(),
        "options: Ctrl+Shift+T=toggle battle style".to_string(),
        "runtime memory: Shift+;=script value, Shift+'=last special, Shift+/=last talked"
            .to_string(),
        "runtime memory: Shift+7=remove variable, Shift+8=remove memory, Shift+9=remove buffer"
            .to_string(),
        "clock: Shift+1=morning, Shift+2=day, Shift+3=night, Shift+4=manual evening"
            .to_string(),
        "battle items: Shift+5=escape item, Shift+6=Guard Spec, Shift+<=battle PP"
            .to_string(),
        "scripted: Shift+G=gift, Shift+W=wild start/complete, Shift+B=trainer start/complete"
            .to_string(),
        "phone/swarm: Shift+N=init contacts, Shift+V=map phone command, Shift+C=map swarm command"
            .to_string(),
        "runtime actions: Shift+Y=map trade command, Shift+H=map catch tutorial, Ctrl+Shift+Y/U/J=daycare deposit/withdraw/inspect man"
            .to_string(),
        "runtime actions: Ctrl+Alt+Shift+Y/U/J=daycare deposit/withdraw/inspect lady".to_string(),
        "bug contest: Ctrl+Shift+PgDn/PgUp/Home/End/Ins/\\=balls/contestants/dropoff/return/check/judge"
            .to_string(),
        "services: Ctrl+Shift+A=Kurt selected apricorn, Ctrl+Shift+V=Buena password, Ctrl+Shift+.=Buena selected prize"
            .to_string(),
        "runtime actions: Alt+Shift+G=script item grant, Alt+Shift+C=check item, Alt+Shift+T=take item, Alt+Shift+M=economy, Alt+Shift+F=field pickup"
            .to_string(),
        "runtime actions: Alt+Shift+R=flag mutation, Alt+Shift+Q=flag check, Alt+Shift+S=scene, Alt+Shift+D=block, Alt+Shift+A=audio, Alt+Shift+X=text"
            .to_string(),
        "runtime actions: Alt+Shift+V=variable, Alt+Shift+L=control, Alt+Shift+O=object"
            .to_string(),
        "items: PgUp/PgDn=party cursor, Alt+PgUp/PgDn=bag cursor, Shift+PgUp/PgDn=ball cursor, Ctrl+PgUp/PgDn=TM/HM cursor, Shift+I=party item selected"
            .to_string(),
        "items: Shift+O=whole party, Shift+P=PP selected move, Ctrl+Shift+I=give selected held, Alt+I=slot 2, Shift+K=TM/HM selected with selected replacement, Ctrl+Shift+K=take held, Alt+K=TM/HM slot 2, Alt+Shift+I/K=rare candy/evolution item".to_string(),
        "items: Shift+S=active battle item, Ctrl+Shift+S=swap selected with lead, Shift+L=battle item active, Alt+L=slot 2, Shift+<=battle PP selected move, Alt+<=slot 2"
            .to_string(),
        "field: P=selected repel, V=bike, T=map, E=rope, F=selected rod, I=itemfinder, field moves prefer selected party cursor".to_string(),
        "field: Q=squirtbottle, G=coin case, U=blue card".to_string(),
        "moves: A=surf, C=cut, K=strength, J=flash, W=waterfall, D=dig, H=headbutt".to_string(),
        "moves: N=whirlpool, R=rock smash, O=teleport, Ctrl+Alt+Home/End=fly destination, Alt+F=fly, <=sweet scent grass, >=sweet scent water"
            .to_string(),
        "shop: M=buy selected, L=sell selected normal item, O=close shop".to_string(),
        "battle/save: Z=selected move, F1-F4=direct move, S=switch selected party cursor, Alt+1-6=switch, B=selected ball, Alt+Shift+1-6=ball".to_string(),
        "battle/save: R=run, C=rewards, F5=save, Ctrl+F5=load".to_string(),
        "battle/save: Shift+F4=next trainer mon, Shift+F5=heal special, Ctrl+Shift+F5/F6=full heal party/selected, Ctrl+Shift+Backspace=blackout".to_string(),
        "services: Shift+F1=delete selected move, Shift+F2=name rater selected, Shift+F3=tutor selected, Ctrl+Shift+F3/F4=Pokerus/Poke Seer"
            .to_string(),
        "services: Ctrl+Alt+Shift+F1/F2/F3=Give Shuckie/Return Shuckie/Odd Egg".to_string(),
        "services: Ctrl+Alt+Shift+G/H/R/W=Dratini/Bill/Roamers/Magikarp length".to_string(),
        "services: Ctrl+Alt+Shift+O/P/B=Oak PC/Magikarp sign/Battle Tower reset".to_string(),
        "services: Ctrl+Alt+Shift+A/D/X=Older haircut/Younger haircut/Daisy grooming"
            .to_string(),
        "services: Ctrl+Alt+Shift+C/V/N=Mystery Gift check/claim/unlock".to_string(),
        "services: Ctrl+Alt+Shift+Z/U/J=story gate/Day Care status/no-op special".to_string(),
        "services: Ctrl+Alt+Shift+1/2/3/4/5=graphics/party/phone/item/fishing special"
            .to_string(),
        "services: Ctrl+Alt+Shift+F/T/E=palette/day/time update".to_string(),
        "services: Ctrl+Alt+Shift+PgDn/Home/End/P/K=spawn/music fade/wait/play/restart"
            .to_string(),
        "services: Shift+`=selected money/game corner/link/photo/tower routine".to_string(),
        "services: Shift+>=selected time/lucky/money/reset/chamber routine".to_string(),
        "pc/map: Shift+F10=Pokemon Center PC, Shift+F11=player PC, Shift+F12=town map, Ctrl+Alt+PgUp/PgDn=box cursor, Alt+Shift+F10/F11/F12=next box/deposit selected/withdraw selected, Alt+Shift+Delete=release selected"
            .to_string(),
        "hud: Tab=cycle, F6=status, F7=party, F8=bag, F9=battle, F10=ui, F11=progress, F12=map"
            .to_string(),
        "hud: Shift+F6=storage, Shift+F7=scripts, Shift+F8=audio, Shift+F9=special"
            .to_string(),
        format!("hud_mode={mode:?}"),
    ];

    match mode {
        HudMode::Status => format_status_details(snapshot, &mut lines),
        HudMode::Party => format_party_details(snapshot, runtime_shell, &mut lines),
        HudMode::Bag => format_bag_details(snapshot, runtime_shell, &mut lines),
        HudMode::Battle => format_battle_details(snapshot, runtime_shell, &mut lines),
        HudMode::Ui => format_ui_details(snapshot, &mut lines),
        HudMode::Progress => format_progress_details(snapshot, &mut lines),
        HudMode::Storage => format_storage_details(snapshot, runtime_shell, &mut lines),
        HudMode::Map => format_map_details(snapshot, &mut lines),
        HudMode::Scripts => format_script_details(snapshot, runtime_shell, &mut lines),
        HudMode::Audio => format_audio_details(snapshot, runtime_shell, &mut lines),
        HudMode::Special => format_special_details(snapshot, &mut lines),
    }
    if !runtime_shell.last_audio_events.is_empty() {
        lines.push(format!(
            "audio events={}",
            runtime_shell.last_audio_events.join(" | ")
        ));
    }
    if let Some(error) = &runtime_shell.last_error {
        lines.push(format!("error={error}"));
    }

    lines.join("\n")
}

fn format_status_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    let current_map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name);
    lines.push(format!(
        "pokedex seen={} owned={} repel={} last_spawn={:?} music={:?}",
        snapshot.progression.pokedex_seen,
        snapshot.progression.pokedex_owned,
        snapshot.progression.repel_steps_remaining,
        snapshot.progression.last_spawn_identifier,
        snapshot.audio.current_music
    ));
    lines.push(format!(
        "objects={} visible_objects={} warps={} coord_events={} bg_events={} scenes={}",
        current_map.map(|map| map.objects.len()).unwrap_or(0),
        snapshot.visible_objects.len(),
        current_map.map(|map| map.events.warps.len()).unwrap_or(0),
        current_map
            .map(|map| map.events.coord_events.len())
            .unwrap_or(0),
        current_map
            .map(|map| map.events.bg_events.len())
            .unwrap_or(0),
        current_map.map(|map| map.scenes.scenes.len()).unwrap_or(0)
    ));
    if let Some(metadata) = current_map.and_then(|map| map.metadata.as_ref()) {
        lines.push(format!(
            "map_meta constant={} group={} id={}x{} env={} phone={}",
            metadata.constant,
            metadata.group_name,
            metadata.width,
            metadata.height,
            metadata.environment,
            metadata.phone_service
        ));
    }
    if let Some(map) = current_map {
        append_nearby_map_context(snapshot, map, &snapshot.visible_objects, lines);
    }
    if !snapshot.audio.queued_events.is_empty() {
        lines.push(format!("queued_audio={:?}", snapshot.audio.queued_events));
    }
}

fn append_nearby_map_context(
    snapshot: &RuntimeShellSnapshot,
    map: &crate::RuntimeMapCatalogSnapshot,
    visible_objects: &[crate::core::map::ObjectEvent],
    lines: &mut Vec<String>,
) {
    let (dx, dy) = snapshot.overworld.facing.delta();
    let front_x = snapshot.overworld.tile.x + dx;
    let front_y = snapshot.overworld.tile.y + dy;
    lines.push(format!(
        "front tile=({}, {}) facing={:?}",
        front_x, front_y, snapshot.overworld.facing
    ));

    for object in visible_objects
        .iter()
        .filter(|object| tile_matches(object.x, object.y, front_x, front_y))
        .take(4)
    {
        lines.push(format!(
            "front_object id={:?} sprite={} script={} flag={} type={}",
            object.object_identifier,
            object.sprite,
            object.script,
            object.event_flag,
            object.object_type
        ));
    }
    for bg in map
        .events
        .bg_events
        .iter()
        .filter(|event| tile_matches(event.x, event.y, front_x, front_y))
        .take(4)
    {
        lines.push(format!(
            "front_bg type={} script={} tile=({}, {})",
            bg.event_type, bg.script, bg.x, bg.y
        ));
    }
    for warp in map
        .events
        .warps
        .iter()
        .filter(|warp| {
            tile_matches(
                warp.x,
                warp.y,
                snapshot.overworld.tile.x,
                snapshot.overworld.tile.y,
            )
        })
        .take(4)
    {
        lines.push(format!(
            "standing_warp index={} target={} target_warp={}",
            warp.index, warp.target_map, warp.target_warp_id
        ));
    }
    for coord in map
        .events
        .coord_events
        .iter()
        .filter(|coord| {
            tile_matches(
                coord.x,
                coord.y,
                snapshot.overworld.tile.x,
                snapshot.overworld.tile.y,
            )
        })
        .take(4)
    {
        lines.push(format!(
            "standing_coord scene={} script={}",
            coord.scene_id, coord.script_name
        ));
    }
}

fn tile_matches(event_x: u16, event_y: u16, tile_x: i16, tile_y: i16) -> bool {
    tile_x >= 0 && tile_y >= 0 && event_x == tile_x as u16 && event_y == tile_y as u16
}

fn format_party_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    if snapshot.party.slots.is_empty() {
        lines.push("party empty".to_string());
        return;
    }
    let selected_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let selected_party_index = snapshot.party.slots[selected_slot].index;
    let selected_move_slot = readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(selected_party_index),
        snapshot.party.slots[selected_slot].pokemon.moves.len(),
    );
    lines.push(format!(
        "party_cursor={} move_cursor={}",
        selected_slot + 1,
        selected_move_slot
            .map(|slot| (slot + 1).to_string())
            .unwrap_or_else(|| "-".to_string())
    ));
    for (slot_offset, slot) in snapshot.party.slots.iter().enumerate() {
        let moves = slot
            .pokemon
            .moves
            .iter()
            .enumerate()
            .map(|(move_index, learned)| {
                let move_marker =
                    if slot_offset == selected_slot && Some(move_index) == selected_move_slot {
                        ">"
                    } else {
                        ""
                    };
                format!("{}{}({}pp)", move_marker, learned.name, learned.current_pp)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let marker = if slot_offset == selected_slot {
            ">"
        } else if slot.is_active_battle_pokemon {
            "*"
        } else {
            " "
        };
        lines.push(format!(
            "{}{} {} lvl={} hp={}/{} status={:?} item={:?} moves=[{}]",
            marker,
            slot.index,
            slot.pokemon.species.id,
            slot.pokemon.level,
            slot.pokemon.hp,
            slot.pokemon.max_hp,
            slot.pokemon.status,
            slot.pokemon.item,
            moves
        ));
    }
}

fn format_bag_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let selected_item = readonly_cursor_index(
        &runtime_shell.bag_cursor,
        "bag:items",
        carried_item_count(&snapshot.bag.items),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.items, index));
    let selected_ball = readonly_cursor_index(
        &runtime_shell.ball_cursor,
        "bag:balls",
        carried_item_count(&snapshot.bag.balls),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.balls, index));
    let selected_pc_item = readonly_cursor_index(
        &runtime_shell.pc_item_cursor,
        "pc:items",
        carried_item_count(&snapshot.bag.pc_items),
    )
    .and_then(|index| carried_item_offset(&snapshot.bag.pc_items, index));
    append_item_section(
        "items",
        &snapshot.bag.items,
        selected_item,
        lines,
    );
    append_item_section(
        "balls",
        &snapshot.bag.balls,
        selected_ball,
        lines,
    );
    append_item_section("key_items", &snapshot.bag.key_items, None, lines);
    if !snapshot.bag.tm_hm.is_empty() {
        lines.push("tm_hm:".to_string());
        let selected_tmhm = readonly_cursor_index(
            &runtime_shell.tmhm_cursor,
            "bag:tmhm",
            snapshot.bag.tm_hm.len(),
        );
        for (index, tm) in snapshot.bag.tm_hm.iter().enumerate() {
            let marker = if Some(index) == selected_tmhm {
                ">"
            } else {
                " "
            };
            lines.push(format!("{marker} {tm:?}"));
        }
    }
    append_item_section(
        "pc_items",
        &snapshot.bag.pc_items,
        selected_pc_item,
        lines,
    );
}

fn carried_item_count(items: &[RuntimeBagItemSnapshot]) -> usize {
    items.iter().filter(|item| item.quantity > 0).count()
}

fn carried_item_offset(items: &[RuntimeBagItemSnapshot], carried_index: usize) -> Option<usize> {
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.quantity > 0)
        .nth(carried_index)
        .map(|(offset, _)| offset)
}

fn append_item_section(
    label: &str,
    items: &[crate::RuntimeBagItemSnapshot],
    selected_index: Option<usize>,
    lines: &mut Vec<String>,
) {
    if items.is_empty() {
        return;
    }
    lines.push(format!("{label}:"));
    for (index, item) in items.iter().take(12).enumerate() {
        let marker = if Some(index) == selected_index {
            ">"
        } else {
            " "
        };
        lines.push(format!("{marker} {} x{}", item.item_id, item.quantity));
    }
    if items.len() > 12 {
        lines.push(format!("  ... {} more", items.len() - 12));
    }
}

fn readonly_cursor_index(
    cursor: &Option<MenuCursor>,
    surface_id: &str,
    option_count: usize,
) -> Option<usize> {
    if option_count == 0 {
        return None;
    }
    cursor
        .as_ref()
        .filter(|cursor| cursor.surface_id == surface_id)
        .map(|cursor| cursor.option_index.min(option_count - 1))
}

fn selected_party_move_name(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
) -> String {
    let selected_slot = runtime_shell
        .party_cursor
        .min(snapshot.party.slots.len().saturating_sub(1));
    let Some(slot) = snapshot.party.slots.get(selected_slot) else {
        return "-".to_string();
    };
    readonly_cursor_index(
        &runtime_shell.party_move_cursor,
        &party_move_cursor_surface_id(slot.index),
        slot.pokemon.moves.len(),
    )
    .and_then(|move_slot| slot.pokemon.moves.get(move_slot))
    .map(|learned| learned.name.clone())
    .unwrap_or_else(|| "-".to_string())
}

fn format_battle_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let Some(battle) = &snapshot.battle else {
        lines.push("battle none".to_string());
        return;
    };
    lines.push(format!(
        "battle {:?} type={} enemy={} lvl={} hp={}/{}",
        battle.kind,
        battle.battle_type,
        battle.enemy_pokemon.species.id,
        battle.enemy_pokemon.level,
        battle.enemy_pokemon.hp,
        battle.enemy_pokemon.max_hp
    ));
    lines.push(format!(
        "active_player={:?} active_enemy={:?} rewarded={:?}",
        battle.active_player_party_index,
        battle.active_enemy_party_index,
        battle.rewarded_enemy_party_indices
    ));
    lines.push(format!(
        "commands player_moves={:?} enemy_moves={:?} switches={:?} items={} balls={} run={} escapes={}",
        battle.commands.player_move_slots,
        battle.commands.enemy_move_slots,
        battle.commands.switch_party_indices,
        battle.commands.can_use_items,
        snapshot.bag.balls.len(),
        battle.commands.can_run,
        battle.escape_attempts
    ));
    append_battle_cursor_context(snapshot, runtime_shell, lines);
    if !battle.enemy_party.is_empty() {
        lines.push(format!("enemy_party_count={}", battle.enemy_party.len()));
    }
    if !snapshot.bag.balls.is_empty() {
        for (index, ball) in snapshot.bag.balls.iter().take(8).enumerate() {
            lines.push(format!(
                "ball {} item={} quantity={}",
                index + 1,
                ball.item_id,
                ball.quantity
            ));
        }
    }
}

fn format_ui_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    lines.push(format!(
        "window={} text_window={} coords={:?} picture={:?}",
        snapshot.ui.window_open,
        snapshot.ui.text_window_open,
        snapshot.ui.coords,
        snapshot.ui.active_pokemon_picture
    ));
    if !snapshot.linked_menu_results.is_empty() {
        lines.push(format!(
            "linked_menu_results={}",
            snapshot.linked_menu_results.len()
        ));
        for result in snapshot.linked_menu_results.iter().take(4) {
            lines.push(format!("  linked_result {result:?}"));
        }
    }
    if let Some(shop) = &snapshot.pending_shop {
        lines.push(format!("shop={shop:?}"));
    }
    if let Some(text) = &snapshot.ui.text {
        lines.push(format!(
            "text label={} source={:?} asm={:?} queued={}",
            text.label, text.source, text.asm_text, text.queued_text_events
        ));
    }
    if let Some(menu) = &snapshot.ui.menu {
        lines.push(format!(
            "menu={} source={:?} coords={:?} vertical_menus={}",
            menu.menu_id,
            menu.source,
            menu.coords,
            menu.layout.vertical_menus.len()
        ));
        for vertical in &menu.layout.vertical_menus {
            lines.push(format!(
                "  {} command={} options={:?}",
                vertical.source_script, vertical.verticalmenu_command_index, vertical.options
            ));
        }
    }
    if let Some(prompt) = &snapshot.ui.pending_yes_no {
        lines.push(format!("yes_no={prompt:?}"));
    }
    if let Some(wait) = &snapshot.ui.pending_text_wait {
        lines.push(format!("text_wait={wait:?}"));
    }
    if !snapshot.presentation.pc_strings.is_empty() {
        lines.push(format!(
            "pc_strings={}",
            snapshot.presentation.pc_strings.len()
        ));
        for (key, text) in snapshot.presentation.pc_strings.iter().take(6) {
            lines.push(format!("  {key}={text}"));
        }
    }
    if !snapshot.ui.elevators.is_empty() {
        lines.push(format!("elevators={:?}", snapshot.ui.elevators));
    }
    if !snapshot.ui.gift_pokemon.is_empty() {
        lines.push(format!("gift_pokemon={:?}", snapshot.ui.gift_pokemon));
    }
}

fn format_progress_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    lines.push(format!(
        "trainer={} id={} money={} moms_money={} coins={} pc_box={} event_flags={} engine_flags={}",
        snapshot.trainer.player_name,
        snapshot.trainer.player_id,
        snapshot.trainer.money,
        snapshot.trainer.moms_money,
        snapshot.trainer.coins,
        snapshot.trainer.current_pc_box,
        snapshot.progression.active_event_flags.len(),
        snapshot.progression.active_engine_flags.len()
    ));
    lines.push(format!(
        "pokedex seen={} owned={} badges={:?}",
        snapshot.progression.pokedex_seen,
        snapshot.progression.pokedex_owned,
        snapshot.progression.badges
    ));
    lines.push(format!(
        "link wins={} losses={} draws={} repel={} active_repel={:?} last_spawn={:?}",
        snapshot.progression.link_wins,
        snapshot.progression.link_losses,
        snapshot.progression.link_draws,
        snapshot.progression.repel_steps_remaining,
        snapshot.progression.active_repel_item,
        snapshot.progression.last_spawn_identifier
    ));
    lines.push(format!("time={:?}", snapshot.progression.time));
    lines.push(format!(
        "multiplayer frame={} state_hash={:#010x} rng_seed={:#010x} linked_menu_results={}",
        snapshot.state_checksum.frame(),
        snapshot.state_checksum.hash(),
        snapshot.progression.rng_seed,
        snapshot.linked_menu_results.len()
    ));
    for flag in snapshot.progression.active_engine_flags.iter().take(8) {
        lines.push(format!("engine_flag {flag}"));
    }
    for flag in snapshot.progression.active_event_flags.iter().take(8) {
        lines.push(format!("event_flag {flag}"));
    }
}

fn format_storage_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "storage current_box={} party_count={} boxes={}",
        snapshot.storage.current_pc_box,
        snapshot.storage.party_count,
        snapshot.storage.boxes.len()
    ));
    for box_snapshot in snapshot.storage.boxes.iter().take(8) {
        let selected_slot = if box_snapshot.index == snapshot.storage.current_pc_box {
            readonly_cursor_index(
                &runtime_shell.storage_cursor,
                &storage_cursor_surface_id(box_snapshot.index),
                box_snapshot.slots.len(),
            )
        } else {
            None
        };
        lines.push(format!(
            "box {} {} count={}",
            box_snapshot.index, box_snapshot.name, box_snapshot.count
        ));
        for (slot_offset, slot) in box_snapshot.slots.iter().take(4).enumerate() {
            let marker = if Some(slot_offset) == selected_slot {
                ">"
            } else {
                " "
            };
            lines.push(format!(
                "{marker} {} {} lvl={} hp={}/{} item={:?}",
                slot.index,
                slot.pokemon.species.id,
                slot.pokemon.level,
                slot.pokemon.hp,
                slot.pokemon.max_hp,
                slot.pokemon.item
            ));
        }
        if box_snapshot.slots.len() > 4 {
            lines.push(format!("  ... {} more", box_snapshot.slots.len() - 4));
        }
    }
    if snapshot.storage.boxes.len() > 8 {
        lines.push(format!(
            "... {} more boxes",
            snapshot.storage.boxes.len() - 8
        ));
    }
}

fn format_map_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    let Some(map) = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
    else {
        lines.push(format!(
            "map {} missing from catalog",
            snapshot.overworld.map_name
        ));
        return;
    };
    lines.push(format!(
        "map={} id={} size={}x{} tileset={} border={} blocks={} music={:?}",
        map.map_name,
        map.id,
        map.attributes.width,
        map.attributes.height,
        map.attributes.tileset_name,
        map.attributes.border_block,
        map.blocks.len(),
        map.attributes.music
    ));
    if let Some(metadata) = &map.metadata {
        lines.push(format!(
            "meta constant={} display={} group={} ids={}:{} env={} phone={}",
            metadata.constant,
            metadata.name,
            metadata.group_name,
            metadata.group_id,
            metadata.map_id,
            metadata.environment,
            metadata.phone_service
        ));
    }
    for spawn in snapshot
        .spawn_points
        .iter()
        .filter(|spawn| spawn.map_name == map.map_name)
        .take(8)
    {
        lines.push(format!(
            "spawn {} map_constant={} tile=({}, {}) group={}:{}",
            spawn.identifier,
            spawn.map_constant,
            spawn.tile_x,
            spawn.tile_y,
            spawn.group_name,
            spawn.group_id
        ));
    }
    lines.push(format!(
        "events warps={} coord={} bg={} objects={} visible_objects={} scenes={} connections={}",
        map.events.warps.len(),
        map.events.coord_events.len(),
        map.events.bg_events.len(),
        map.objects.len(),
        snapshot.visible_objects.len(),
        map.scenes.scenes.len(),
        map.attributes.connections.len()
    ));
    for warp in map.events.warps.iter().take(8) {
        lines.push(format!(
            "warp {} tile=({}, {}) target={} target_warp={}",
            warp.index, warp.x, warp.y, warp.target_map, warp.target_warp_id
        ));
    }
    for object in snapshot.visible_objects.iter().take(8) {
        lines.push(format!(
            "visible_object {:?} sprite={} tile=({}, {}) script={} flag={}",
            object.object_identifier,
            object.sprite,
            object.x,
            object.y,
            object.script,
            object.event_flag
        ));
    }
    let active_flypoints = active_fly_destination_flags(snapshot)
        .into_iter()
        .take(8)
        .collect::<Vec<_>>();
    if !active_flypoints.is_empty() {
        lines.push(format!("active_flypoints={active_flypoints:?}"));
    }
    if let Some(wild) = snapshot.encounters.wild.get(&map.map_name) {
        lines.push(format!(
            "wild grass_rates={:?} water_rate={:?}",
            wild.grass_rates, wild.water_rate
        ));
    }
    if let Some(field) = snapshot.encounters.field.get(&map.map_name) {
        lines.push(format!("field encounter tables={}", field.tables.len()));
    }
}

fn format_script_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let scripts = &snapshot.script_events;
    lines.push(format!(
        "script_value={:?} command_cursor={} variables={} memory={} buffers={} phones={} last_special={:?} last_talked={:?}",
        scripts.script_value,
        runtime_shell.script_command_cursor,
        scripts.variables.len(),
        scripts.memory.len(),
        scripts.named_buffers.len(),
        scripts.phone_numbers.len(),
        scripts.last_special_routine,
        scripts.last_talked_object
    ));
    lines.push(format!(
        "queues delays={} emotes={} commands={} call_stack={} deferred={} ended={} variable_writes={} effects={} asm={} text={} audio={} graphics={}",
        scripts.pending_delays.len(),
        scripts.pending_emotes.len(),
        scripts.command_queue.len(),
        scripts.call_stack.len(),
        scripts.deferred_scripts.len(),
        scripts.script_ended.is_some(),
        scripts.variable_writes.len(),
        scripts.effects.len(),
        scripts.asm_directives.len(),
        scripts.text_events.len(),
        scripts.audio_events.len(),
        scripts.graphics_events.len()
    ));
    lines.push(format!(
        "pending warp={:?} load={:?} refresh={:?} yes_no={:?} text_wait={:?}",
        scripts.pending_script_warp,
        scripts.pending_map_load,
        scripts.pending_map_refresh,
        snapshot.ui.pending_yes_no,
        snapshot.ui.pending_text_wait
    ));
    for (key, value) in scripts.variables.iter().take(8) {
        lines.push(format!("var {key}={value}"));
    }
    for command in scripts.command_queue.iter().take(8) {
        lines.push(format!("command {command:?}"));
    }
    for effect in scripts.effects.iter().take(8) {
        lines.push(format!("effect {effect:?}"));
    }
    append_current_map_script_command_summary(snapshot, runtime_shell, lines);
}

fn append_current_map_script_command_summary(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    let current_map = snapshot.overworld.map_name.as_str();
    let map_commands = runtime_shell
        .shell
        .script_map_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let shop_commands = runtime_shell
        .shell
        .script_shop_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let flag_commands = runtime_shell
        .shell
        .script_flag_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let scene_commands = runtime_shell
        .shell
        .script_scene_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let object_commands = runtime_shell
        .shell
        .script_object_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let variable_commands = runtime_shell
        .shell
        .script_variable_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let control_commands = runtime_shell
        .shell
        .script_control_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let item_grants = runtime_shell
        .shell
        .script_item_grant_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let item_access = runtime_shell
        .shell
        .script_item_access_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let field_pickups = runtime_shell
        .shell
        .script_field_pickup_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let economy_commands = runtime_shell
        .shell
        .script_economy_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let audio_commands = runtime_shell
        .shell
        .script_audio_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let text_commands = runtime_shell
        .shell
        .script_text_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let phone_commands = runtime_shell
        .shell
        .script_phone_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let swarm_commands = runtime_shell
        .shell
        .script_swarm_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();
    let runtime_commands = runtime_shell
        .shell
        .script_runtime_command_keys()
        .into_iter()
        .filter(|key| key.map_name == current_map)
        .collect::<Vec<_>>();

    lines.push(format!(
        "current_map_commands map={} cursor={} script_map={} shop={} flags={} scenes={} objects={} vars={} control={} grants={} access={} pickups={} economy={} audio={} text={} phone={} swarm={} runtime={}",
        current_map,
        runtime_shell.script_command_cursor,
        map_commands.len(),
        shop_commands.len(),
        flag_commands.len(),
        scene_commands.len(),
        object_commands.len(),
        variable_commands.len(),
        control_commands.len(),
        item_grants.len(),
        item_access.len(),
        field_pickups.len(),
        economy_commands.len(),
        audio_commands.len(),
        text_commands.len(),
        phone_commands.len(),
        swarm_commands.len(),
        runtime_commands.len()
    ));

    for command in map_commands.iter().take(3) {
        lines.push(format!(
            "  mapcmd {}:{} command={} target={:?} xy=({:?},{:?})",
            command.source_script,
            command.command_index,
            command.command,
            command.target_map,
            command.x,
            command.y
        ));
    }
    for command in shop_commands.iter().take(3) {
        lines.push(format!(
            "  shopcmd {}:{} command={} mart={}:{}",
            command.source_script,
            command.command_index,
            command.command,
            command.mart_type,
            command.mart_id
        ));
    }
    for command in flag_commands.iter().take(3) {
        lines.push(format!(
            "  flagcmd {}:{} command={} flag={}",
            command.source_script, command.command_index, command.command, command.flag_id
        ));
    }
    for command in scene_commands.iter().take(3) {
        lines.push(format!(
            "  scenecmd {}:{} command={} map={:?} scene={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.map_id,
            command.scene_id
        ));
    }
    for command in object_commands.iter().take(3) {
        lines.push(format!(
            "  objectcmd {}:{} command={} object={:?} target={:?} xy=({:?},{:?})",
            command.source_script,
            command.command_index,
            command.command,
            command.object_id,
            command.target_object_id,
            command.x,
            command.y
        ));
    }
    for command in variable_commands.iter().take(3) {
        lines.push(format!(
            "  varcmd {}:{} command={} target={:?} values={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.target,
            command.value_tokens
        ));
    }
    for command in control_commands.iter().take(3) {
        lines.push(format!(
            "  controlcmd {}:{} command={} target={:?} resolved={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.target_label,
            command.resolved_target_script
        ));
    }
    for command in item_grants.iter().take(3) {
        lines.push(format!(
            "  grantcmd {}:{} command={} item={} quantity={}",
            command.source_script,
            command.command_index,
            command.command,
            command.item_id,
            command.quantity
        ));
    }
    for command in item_access.iter().take(3) {
        lines.push(format!(
            "  itemcmd {}:{} command={} item={}",
            command.source_script, command.command_index, command.command, command.item_id
        ));
    }
    for command in field_pickups.iter().take(3) {
        lines.push(format!(
            "  pickupcmd {}:{} command={} item={:?} fruit={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.item_id,
            command.fruit_tree_id
        ));
    }
    for command in economy_commands.iter().take(3) {
        lines.push(format!(
            "  economycmd {}:{} command={} account={:?} amount={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.account,
            command.amount_tokens
        ));
    }
    for command in audio_commands.iter().take(3) {
        lines.push(format!(
            "  audiocmd {}:{} command={} audio={:?} fade={:?}",
            command.source_script,
            command.command_index,
            command.command,
            command.audio_id,
            command.fade_frames
        ));
    }
    for command in text_commands.iter().take(3) {
        lines.push(format!(
            "  textcmd {}:{} command={} label={:?}",
            command.source_script, command.command_index, command.command, command.text_label
        ));
    }
    for command in phone_commands.iter().take(3) {
        lines.push(format!(
            "  phonecmd {}:{} command={} contact={}",
            command.source_script, command.command_index, command.command, command.contact_id
        ));
    }
    for command in swarm_commands.iter().take(3) {
        lines.push(format!(
            "  swarmcmd {}:{} command={} token={} map_id={}",
            command.source_script,
            command.command_index,
            command.command,
            command.swarm_token,
            command.map_id
        ));
    }
    for command in runtime_commands.iter().take(3) {
        lines.push(format!(
            "  runtimecmd {}:{} command={} args={:?}",
            command.source_script, command.command_index, command.command, command.args
        ));
    }
}

fn format_audio_details(
    snapshot: &RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    lines: &mut Vec<String>,
) {
    lines.push(format!(
        "audio current_music={:?} queued={} catalog music={} sfx={} cries={}",
        snapshot.audio.current_music,
        snapshot.audio.queued_events.len(),
        snapshot.audio_catalog.music.len(),
        snapshot.audio_catalog.sound_effects.len(),
        snapshot.audio_catalog.cries.len()
    ));
    lines.push(format!(
        "resolved_events={} pending_audio={}",
        runtime_shell.last_audio_events.len(),
        runtime_shell.pending_audio.len()
    ));
    for event in runtime_shell.last_audio_events.iter().take(8) {
        lines.push(format!("event {event}"));
    }
    for (music, _) in snapshot.audio_catalog.music.iter().take(8) {
        lines.push(format!("music {music}"));
    }
    for (effect, _) in snapshot.audio_catalog.sound_effects.iter().take(8) {
        lines.push(format!("sfx {effect}"));
    }
    for (cry, _) in snapshot.audio_catalog.cries.iter().take(8) {
        lines.push(format!("cry {cry}"));
    }
}

fn format_special_details(snapshot: &RuntimeShellSnapshot, lines: &mut Vec<String>) {
    let special = &snapshot.special;
    lines.push(format!(
        "special routines={} contacts={} permanent_phones={} calls={} trades={}",
        special.special_routines.len(),
        special.phone_contacts.0.len(),
        special.permanent_phone_numbers.len(),
        special.special_phone_calls.len(),
        special.npc_trades.len()
    ));
    lines.push(format!(
        "features shuckie={} bug_contest={} battle_tower={} happiness={} buena_categories={} prizes={} roaming={}",
        special.shuckie_gift.is_some(),
        special.bug_contest_config.is_some(),
        special.battle_tower_rules.is_some(),
        special.happiness_data.is_some(),
        special.buena_password_categories.categories.len(),
        special.buena_prizes.len(),
        special.roaming_pokemon.len()
    ));
    lines.push(format!(
        "services heal={} pc={} delete={} name_rater={} tutor={} menu={} time={} story={} daycare={} noop={}",
        special.special_routines.contains_key("HealParty"),
        special.special_routines.contains_key("PokemonCenterPC"),
        special.special_routines.contains_key("MoveDeletion"),
        special.special_routines.contains_key("NameRater"),
        special.special_routines.contains_key("MoveTutor"),
        [
            "BankOfMom",
            "SlotMachine",
            "CardFlip",
            "DisplayLinkRecord",
            "TrainerHouse",
            "PhotoStudio",
            "Menu_ChallengeExplanationCancel",
        ]
            .into_iter()
            .filter(|routine| special.special_routines.contains_key(*routine))
            .count(),
        [
            "SetDayOfWeek",
            "InitialSetDSTFlag",
            "InitialClearDSTFlag",
            "UpdateTime",
            "SampleKenjiBreakCountdown",
            "CheckLuckyNumberShowFlag",
            "ResetLuckyNumberShowFlag",
            "PrintTodaysLuckyNumber",
            "CheckForLuckyNumberWinners",
            "PlaceMoneyTopRight",
            "DisplayMoneyAndCoinBalance",
            "DisplayCoinCaseBalance",
            "GSHealings",
            "StubbedTrainerRankings_Healings",
            "Reset",
            "HoOhChamber",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        [
            "CheckCaughtCelebi",
            "CelebiShrineEvent",
            "SnorlaxAwake",
            "CheckForBattleTowerRules",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        ["DayCareManOutside", "DayCareMon1", "DayCareMon2"]
            .into_iter()
            .filter(|routine| special.special_routines.contains_key(*routine))
            .count(),
        [
            "UnusedDummySpecial",
            "UnusedBattleTowerDummySpecial1",
            "UnusedBattleTowerDummySpecial2",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count()
    ));
    lines.push(format!(
        "special groups graphics={} party={} phone={} item_check={} fishing={}",
        [
            "ClearBGPalettesBufferScreen",
            "ClearBGPalettes",
            "UpdateTimePals",
            "ClearTilemap",
            "LoadMapPalettes",
            "RefreshSprites",
            "UpdateSprites",
            "ReloadSpritesNoPalettes",
            "FadeOutToWhite",
            "FadeInFromWhite",
            "FadeOutToBlack",
            "FadeInFromBlack",
            "GameboyCheck",
            "CheckMobileAdapterStatusSpecial",
            "BattleTowerFade",
            "UpdatePlayerSprite",
            "HealMachineAnim",
            "SurfStartStep",
            "LoadUsedSpritesGFX",
            "ToggleMaptileDecorations",
            "ToggleDecorationsVisibility",
            "MagnetTrain",
            "Diploma",
            "PrintDiploma",
            "UnownPuzzle",
            "OmanyteChamber",
            "DisplayUnownWords",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        [
            "CheckFirstMonIsEgg",
            "GetFirstPokemonHappiness",
            "FindPartyMonThatSpecies",
            "FindPartyMonAboveLevel",
            "FindPartyMonAtLeastThatHappy",
            "FindPartyMonThatSpeciesYourTrainerID",
            "MonCheck",
            "BeastsCheck",
            "GameCornerPrizeMonCheckDex",
            "UnusedSetSeenMon",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        [
            "RandomUnseenWildMon",
            "RandomPhoneWildMon",
            "RandomPhoneMon",
        ]
        .into_iter()
        .filter(|routine| special.special_routines.contains_key(*routine))
        .count(),
        special
            .special_routines
            .contains_key("UnusedFindItemInPCOrBag"),
        special
            .special_routines
            .contains_key("ActivateFishingSwarm")
    ));
    lines.push(format!(
        "kurt={} oak_ratings={} odd_eggs={} magikarp_lengths={} flee_mons={:?}",
        special.kurt_apricorn_recipes.len(),
        special.oak_ratings.len(),
        special.odd_egg_definitions.len(),
        special.magikarp_lengths.len(),
        special.flee_mons
    ));
    for routine in special.special_routines.keys().take(8) {
        lines.push(format!("routine {routine}"));
    }
    for (key, text) in snapshot.presentation.pc_strings.iter().take(8) {
        lines.push(format!("pc_string {key}={text}"));
    }
    for trade in special.npc_trades.keys().take(8) {
        lines.push(format!("npc_trade {trade}"));
    }
    for species in special.roaming_pokemon.keys().take(8) {
        lines.push(format!("roaming {species}"));
    }
}
