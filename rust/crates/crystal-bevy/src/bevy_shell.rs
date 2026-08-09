use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(not(test))]
use std::time::Duration;

use anyhow::{Context, Result};
#[cfg(test)]
use bevy::audio::{AudioBundle, PlaybackSettings};
use bevy::audio::{AudioPlugin, AudioSource};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
#[cfg(feature = "location-tester")]
use bevy::render::view::screenshot::ScreenshotManager;
#[cfg(feature = "location-tester")]
use bevy::window::PrimaryWindow;
use bevy::window::{PresentMode, WindowResolution};
use chrono::{Datelike, Local as ChronoLocal, Timelike};
use crystal_assets::{
    RuntimeBadgeRegion, RuntimeBugContestAction, RuntimeCurrencyAccount, RuntimeDayCareAction,
    RuntimeDayCareCaretaker, RuntimeGameCornerService, RuntimeGraphicsSpecial,
    RuntimeHappinessServiceRoutine, RuntimeLinkBattleResult, RuntimeMysteryGiftAction,
    RuntimePartyCheckSpecial, RuntimePhoneRandomSpecial, RuntimeShuckieAction,
    RuntimeStoryGateSpecial,
};

use crate::assets::{
    ModpackAudioKind, ModpackAudioPlaybackMode, RuntimeMutationOutcome, RuntimeMutationResult,
    RuntimePendingScriptRequestKind, RuntimeScriptEventQueue, RuntimeScriptRuntimeFlag,
    RuntimeScriptRuntimeFlagValue, RuntimeScriptRuntimeMemoryEntry,
    RuntimeScriptRuntimeMemoryValue, RuntimeScriptRuntimeQueue, RuntimeScriptRuntimeRecordQueue,
};
use crate::audio::{AudioKind, AudioProgramSource};
use crate::core::battle::turn::{BattleAction, active_battle_combat_state};
use crate::core::input::GameButton;
use crate::core::models::Dv;
use crate::core::multiplayer::{
    BattleActionFrame, DeterministicInputJournal, DeterministicInputJournalFrame,
    DeterministicReplayBundle, LinkMessage, LockstepFrame, MenuChoiceFrame, MenuChoiceResultFrame,
    PlayerInputFrame, SaveResumeReplayBundle, SessionRuntimeCommandFrame,
    SessionRuntimeCommandResultFrame, SessionSaveCheckpointFrame, StateChecksum,
    StateChecksumFrame, encode_link_message_bytes,
};
use crate::core::random::Random;
use crate::core::state::{
    BattleScene, BattleStyle, FrameType, MenuAccount, PLAYER_GENDER_FEMALE, PLAYER_GENDER_MALE,
    PrintOption, ScriptFadeColor, ScriptFadeDirection, Sound, TextSpeed,
};
use crate::core::systems::battle_items::{
    BattleItemError, BattleItemOutcome, ITEM_EFFECT_BEHAVIOR_EVOLUTION_STONE,
    ITEM_EFFECT_BEHAVIOR_RARE_CANDY,
};
use crate::core::systems::battle_rewards::BattleRewardError;
use crate::core::systems::evolution::EvolutionReport;
use crate::core::systems::field_items::FieldItemPickupOutcome;
use crate::core::systems::field_moves::FieldMoveError;
use crate::core::systems::phone::ScriptPhoneInputs;
use crate::core::systems::script_control::ScriptControlAction;
use crate::core::systems::script_runtime::ScriptRuntimeInputs;
use crate::core::systems::special_routines::SpecialRoutineEffect;
use crate::core::systems::time::{ClockTime, DAY_HOUR, GameDate, MORN_HOUR, NITE_HOUR};
use crate::core::systems::tmhm::TmHmLearnError;
use crate::core::timing::{Frame, GB_FRAME_DURATION_SECONDS};
use crate::core::world::encounters::EncounterSurface;
use crate::core::world::fishing::FishingError;
use crate::core::world::map::{Direction, METATILE_WIDTH, TilePosition};
use crate::core::world::movement::{
    LedgeJumpOutcome, MovementMode, StepOptions, StepOutcome, checked_move_by_stride,
};
use crate::core::world::session::{
    background_event_tile_position_checked, coord_event_tile_position_checked,
    object_event_initial_facing, object_tile_position_checked, warp_tile_position_checked,
};
use crate::{
    CrystalRuntime, RuntimeBagItemSnapshot, RuntimeBattleKind, RuntimeCompiledScriptCursor,
    RuntimeElevatorSnapshot, RuntimeFlyDestinationKey, RuntimeGameShell,
    RuntimeGiftPokemonSnapshot, RuntimeLinkSessionDescriptor, RuntimeMapCatalogSnapshot,
    RuntimePendingScriptRequest, RuntimeResolvedAudioPlaybackKind, RuntimeRtcSample,
    RuntimeShellSnapshot, RuntimeTilesetKey, assets::AssetRoot,
};

mod intro_renderer;

const GAME_TICK_SECONDS: f32 = GB_FRAME_DURATION_SECONDS as f32;
const VIEWPORT_TILES_X: i16 = 20;
const VIEWPORT_TILES_Y: i16 = 18;
// The Game Boy Color LCD is exactly 20 by 18 tiles.  Render source tiles at a
// uniform 4x integer scale so every game surface occupies the 640 by 576
// window; using 3x left an exposed, non-Game-Boy backing area around screens.
const TILE_SIZE: f32 = 32.0;
const PLAYFIELD_LEFT: f32 = -320.0;
const PLAYFIELD_TOP: f32 = 288.0;
const PLAYFIELD_WIDTH: f32 = VIEWPORT_TILES_X as f32 * TILE_SIZE;
const PLAYFIELD_HEIGHT: f32 = VIEWPORT_TILES_Y as f32 * TILE_SIZE;
const EVENT_LOG_LIMIT: usize = 192;
const RECENT_OVERWORLD_INPUT_LIMIT: usize = 2048;
const WALK_FRAME_HOLD_TICKS: u8 = 8;
const OVERWORLD_TURN_HOLD_TICKS: u8 = 4;
// Crystal advances a walking tile over several VBlanks.  The core session
// operates on completed tiles, so the real-time host must gate held movement
// instead of applying a full tile at every 60 Hz input sample.
const OVERWORLD_STEP_REPEAT_TICKS: u8 = 8;
// A stalled host frame must not turn into an unbounded burst of gameplay
// ticks.  The original LCD is 60 Hz, but replaying hundreds of overdue ticks
// after a texture compile makes the desktop shell spiral at 100% CPU and
// leaves the user watching a one-FPS transition. One authoritative tick per
// host update preserves input/text cadence while dropping stale gameplay work.
// Never fast-forward several Game Boy frames in one host update.  Catch-up
// made a stalled macOS frame instantly consume dialogue, fades, and scripted
// input boundaries; dropping excess elapsed time keeps visible pacing stable.
const MAX_RUNTIME_CATCH_UP_TICKS: u32 = 1;
// Presentation sequences must preserve their 60 Hz wall-clock duration even
// when a composed frame briefly costs more than one VBlank. Twelve frames is
// a bounded 200ms recovery window: it prevents an eight-frame fade from being
// stretched into seconds of black while still rejecting the unbounded bursts
// that made title/credits instantly disappear after a real stall.
const MAX_VISIBLE_SEQUENCE_CATCH_UP_FRAMES: usize = 12;
const LOCAL_PLAYER_ID: u64 = 1;
const DEFAULT_MALE_PLAYER_NAME: &str = "CHRIS";
const DEFAULT_FEMALE_PLAYER_NAME: &str = "KRIS";
const SCENE_MENU_VISIBLE_ROWS: usize = 6;
const SCENE_DIALOG_TEXT_CHARS: usize = 18;
const BATTLE_MAIN_MENU_LABELS: [&str; 4] = ["FIGHT", "<PKMN>", "PACK", "RUN"];
const BATTLE_MAIN_MENU_LEFT_TILE: f32 = 8.0;
const BATTLE_MAIN_MENU_TOP_TILE: f32 = 12.0;
const BATTLE_MAIN_MENU_WIDTH_TILES: f32 = 12.0;
const BATTLE_MAIN_MENU_HEIGHT_TILES: f32 = 6.0;
const BATTLE_MAIN_MENU_ORIGIN_TILE_X: f32 = BATTLE_MAIN_MENU_LEFT_TILE + 1.0;
const BATTLE_MAIN_MENU_ORIGIN_TILE_Y: f32 = BATTLE_MAIN_MENU_TOP_TILE + 1.0;
const BATTLE_MAIN_MENU_COLUMN_SPACING_TILES: f32 = 6.0;
const BATTLE_MAIN_MENU_ROW_SPACING_TILES: f32 = 2.0;
const BATTLE_MOVE_SELECTION_LEFT_TILE: f32 = 4.0;
const BATTLE_MOVE_SELECTION_TOP_TILE: f32 = 12.0;
const BATTLE_MOVE_SELECTION_WIDTH_TILES: f32 = 16.0;
const BATTLE_MOVE_SELECTION_HEIGHT_TILES: f32 = 6.0;
const BATTLE_MOVE_INFO_LEFT_TILE: f32 = 0.0;
const BATTLE_MOVE_INFO_TOP_TILE: f32 = 8.0;
const BATTLE_MOVE_INFO_WIDTH_TILES: f32 = 11.0;
const BATTLE_MOVE_INFO_HEIGHT_TILES: f32 = 5.0;
const BATTLE_TEXT_BOX_LEFT_TILE: f32 = 0.0;
const BATTLE_TEXT_BOX_TOP_TILE: f32 = 12.0;
const BATTLE_TEXT_BOX_WIDTH_TILES: f32 = 20.0;
const BATTLE_TEXT_BOX_HEIGHT_TILES: f32 = 6.0;
const FIELD_TEXT_BOX_LEFT_TILE: f32 = 0.0;
const FIELD_TEXT_BOX_TOP_TILE: f32 = 12.0;
const FIELD_TEXT_BOX_WIDTH_TILES: f32 = 20.0;
const FIELD_TEXT_BOX_HEIGHT_TILES: f32 = 6.0;
const FIELD_TEXT_BOX_TEXT_LEFT_TILE: f32 = FIELD_TEXT_BOX_LEFT_TILE + 1.0;
// A 20x6 textbox has four interior rows (13..=16). Starting at row 14 puts
// the fourth line on the bottom frame tile, which visibly leaks text below
// the window on Retina-scaled desktop output.
const FIELD_TEXT_BOX_TEXT_TOP_TILE: f32 = FIELD_TEXT_BOX_TOP_TILE + 1.0;
// ASM `YesNoBox` in home/menu.asm: menu_coords 14, 7, 19, 11.
const FIELD_YES_NO_LEFT_TILE: f32 = 14.0;
const FIELD_YES_NO_TOP_TILE: f32 = 7.0;
const FIELD_YES_NO_WIDTH_TILES: f32 = 6.0;
const FIELD_YES_NO_HEIGHT_TILES: f32 = 5.0;
const FIELD_TEXT_BOX_ROW_SPACING_TILES: f32 = 1.0;
const FIELD_TEXT_BOX_VISIBLE_ROWS: usize = 4;
const START_MENU_LEFT_TILE: f32 = 9.0;
const START_MENU_TOP_TILE: f32 = 0.0;
const START_MENU_RIGHT_TILE: f32 = 19.0;
const START_MENU_MIN_HEIGHT_TILES: f32 = 3.0;
const START_MENU_CURSOR_TILE_X: f32 = START_MENU_LEFT_TILE + 1.0;
const START_MENU_LABEL_TILE_X: f32 = START_MENU_LEFT_TILE + 2.0;
const START_MENU_FIRST_ROW_TILE_Y: f32 = START_MENU_TOP_TILE + 1.0;
const OPTIONS_MENU_LEFT_TILE: f32 = 0.0;
const OPTIONS_MENU_TOP_TILE: f32 = 0.0;
const OPTIONS_MENU_WIDTH_TILES: usize = 20;
const OPTIONS_MENU_HEIGHT_TILES: usize = 18;
const OPTIONS_MENU_CURSOR_TILE_X: f32 = 1.0;
const OPTIONS_MENU_LABEL_TILE_X: f32 = 2.0;
const OPTIONS_MENU_VALUE_TILE_X: f32 = 11.0;
const OPTIONS_MENU_FRAME_VALUE_TILE_X: f32 = 16.0;
const OPTIONS_MENU_FIRST_ROW_TILE_Y: f32 = 2.0;
const OPTIONS_MENU_ROW_SPACING_TILES: f32 = 2.0;
const BATTLE_SUBMENU_ORIGIN_TILE_X: f32 = BATTLE_TEXT_BOX_LEFT_TILE + 1.0;
const BATTLE_SUBMENU_ORIGIN_TILE_Y: f32 = BATTLE_TEXT_BOX_TOP_TILE + 1.0;
const BATTLE_SUBMENU_ROW_SPACING_TILES: f32 = 1.0;
const BATTLE_SUBMENU_COLUMN_SPACING_TILES: f32 = 9.0;
const BATTLE_MOVE_MENU_ORIGIN_TILE_X: f32 = 6.0;
const BATTLE_MOVE_MENU_ORIGIN_TILE_Y: f32 = 13.0;
const BATTLE_MOVE_MENU_ROW_SPACING_TILES: f32 = 1.0;
const BATTLE_HUD_HP_BAR_LENGTH_PX: u16 = 48;
const BATTLE_HUD_HP_BAR_LENGTH_TILES: f32 = 6.0;
const BATTLE_HUD_SCALE: f32 = TILE_SIZE / SOURCE_TILE_SIZE as f32;
const VISIBLE_NAME_ENTRY_MAX_LENGTH: usize = 7;
const SOURCE_TILE_SIZE: usize = 8;
const BITMAP_FONT_GLYPH_SIZE: f32 = TILE_SIZE;
const BITMAP_FONT_ADVANCE: f32 = TILE_SIZE;
const BITMAP_FONT_TILE_SIZE: usize = 8;
const RENDER_METATILE_WIDTH: i16 = 4;
const RENDER_TILES_PER_RUNTIME_TILE: i16 = RENDER_METATILE_WIDTH / METATILE_WIDTH;
const METATILE_TILE_COUNT: usize = RENDER_METATILE_WIDTH as usize * RENDER_METATILE_WIDTH as usize;
const START_MENU_SURFACE_ID: &str = "shell:start-menu";
const ENGINE_POKEDEX_FLAG: &str = "ENGINE_POKEDEX";
const ENGINE_POKEGEAR_FLAG: &str = "ENGINE_POKEGEAR";
const FIELD_PACK_POCKETS: &[FieldPackPocket] = &[
    FieldPackPocket::Items,
    FieldPackPocket::Balls,
    FieldPackPocket::KeyItems,
    FieldPackPocket::TmHm,
];
const OPTIONS_MENU_ITEMS: &[OptionsMenuItem] = &[
    OptionsMenuItem::TextSpeed,
    OptionsMenuItem::BattleScene,
    OptionsMenuItem::BattleStyle,
    OptionsMenuItem::Sound,
    OptionsMenuItem::Print,
    OptionsMenuItem::MenuAccount,
    OptionsMenuItem::Frame,
    OptionsMenuItem::Cancel,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BevyShellStart {
    #[cfg(any(test, feature = "location-tester"))]
    NewGame {
        spawn_identifier: u16,
    },
    #[cfg(any(test, feature = "location-tester"))]
    NewGameAtRuntimeTile {
        spawn_identifier: u16,
        map_name: String,
        tile_x: i16,
        tile_y: i16,
    },
    LoadSave {
        save_path: PathBuf,
    },
    Title {
        spawn_identifier: u16,
        save_path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BevyShellConfig {
    pub quick_save_path: Option<PathBuf>,
    pub smoke_player_name: Option<String>,
    /// `None` keeps the optional voxel feature's normal enabled behavior;
    /// location tools can force either side of a 2D/2.5D comparison.
    pub voxel_view_enabled: Option<bool>,
    pub window_title: Option<String>,
    #[cfg(feature = "location-tester")]
    pub render_test_screenshot: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellStartMenuSmoke {
    pub initial_map: String,
    pub initial_tile_x: i16,
    pub initial_tile_y: i16,
    pub start_menu_entries: Vec<String>,
    pub party_entries: Vec<String>,
    pub pack_entries: Vec<String>,
    pub trainer_entries: Vec<String>,
    pub save_entries: Vec<String>,
    pub saved_frame: u64,
    pub save_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellTitleSmoke {
    pub title_entries: Vec<String>,
    pub selected: String,
    pub map: String,
    pub tile_x: i16,
    pub tile_y: i16,
    pub state_hash: StateChecksum,
    pub saved_frame: Option<u64>,
    pub save_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellTitleNameInputSmoke {
    pub title_entries: Vec<String>,
    pub initial_name_entries: Vec<String>,
    pub typed_name_entries: Vec<String>,
    pub selected: String,
    pub trainer_name: String,
    pub map: String,
    pub tile_x: i16,
    pub tile_y: i16,
    pub state_hash: StateChecksum,
    pub saved_frame: Option<u64>,
    pub save_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellPartySmoke {
    pub initial_entries: Vec<String>,
    pub action_entries: Vec<String>,
    pub summary_entries: Vec<String>,
    pub switch_entries: Vec<String>,
    pub final_entries: Vec<String>,
    pub lead_before: String,
    pub lead_after: String,
    pub state_hash: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellBattleSmokeRef {
    pub map_name: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellBattleSmoke {
    pub wild_species: String,
    pub wild_level: u8,
    pub action_entries: Vec<String>,
    pub switch_entries: Vec<String>,
    pub pack_entries: Vec<String>,
    pub ball_entries: Vec<String>,
    pub move_entries: Vec<String>,
    pub after_entries: Vec<String>,
    pub active_battle_after: bool,
    pub state_hash: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellTrainerBattleSmoke {
    pub trainer_class: String,
    pub trainer_id: String,
    pub trainer_name: String,
    pub initial_entries: Vec<String>,
    pub first_move_entries: Vec<String>,
    pub shift_prompt_entries: Vec<String>,
    pub shift_prompt_count: usize,
    pub kept_current_after_shift_prompt: bool,
    pub switched_after_shift_prompt: bool,
    pub turns: usize,
    pub trainer_defeated: bool,
    pub final_entries: Vec<String>,
    pub active_battle_after: bool,
    pub state_hash: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellOverworldSmoke {
    pub start_map: String,
    pub start_tile_x: i16,
    pub start_tile_y: i16,
    pub start_scene: Option<String>,
    pub final_map: String,
    pub final_tile_x: i16,
    pub final_tile_y: i16,
    pub final_scene: Option<String>,
    pub frames: usize,
    pub interactions: usize,
    pub coord_events: usize,
    pub trainer_sight_events: usize,
    pub warps: usize,
    pub connections: usize,
    pub wild_battles: usize,
    pub last_movement: Option<String>,
    pub frame_events: Vec<String>,
    pub active_music: Option<String>,
    pub audio_events: Vec<String>,
    pub pending_audio: usize,
    pub final_party_species: Vec<String>,
    pub final_bag_items: Vec<RuntimeBagItemSnapshot>,
    pub state_hash: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellSmokePokemon {
    pub species_id: String,
    pub level: u8,
    pub held_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleShellSmokeItem {
    pub item_id: String,
    pub quantity: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTrainerCardPage {
    Info,
    JohtoBadges,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleBalanceOverlay {
    MoneyTopRight { money: u32 },
    MoneyAndCoins { money: u32, coins: u16 },
    CoinsTopRight { coins: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleMomBankPhase {
    InitializeQuestion,
    AccessQuestion,
    Menu,
    Withdraw,
    Deposit,
    ChangeQuestion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleBlackoutPhase {
    AwaitText,
    FadeOut,
    WhiteHold { frames_remaining: u8 },
    FadeIn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleWalkWarpPhase {
    FadeOut,
    FadeIn,
    ScriptFadeIn,
    MapReloadFadeIn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingOverworldStepBoundary {
    Arrival,
    CoordEvent,
    TrainerSight,
    WildBattle,
    PoisonBlackout,
    StepEvent(crate::core::systems::step_events::StepEventResult),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleScriptMovementPhase {
    Sound {
        audio_id: String,
    },
    Move {
        from: TilePosition,
        to: TilePosition,
        direction: Direction,
        duration: u8,
        jump: bool,
        update_facing: bool,
        standing_frame: bool,
    },
    Hold {
        duration: u16,
    },
    TreeShake {
        duration: u16,
    },
    Visibility {
        hidden: bool,
    },
    Stationary {
        duration: u16,
        effect: VisibleStationaryMovementEffect,
    },
    ScreenShake {
        parameter: u16,
    },
    Turn {
        direction: Direction,
        duration: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleStationaryMovementEffect {
    TeleportSpin,
    TeleportRise,
    TeleportWait,
    TeleportDescent,
    SkyfallWait,
    SkyfallFall,
    SkyfallTop,
    DigSpin,
    RockSmash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleFieldTravelAnimation {
    DigOut,
    DigReturn,
    TeleportFrom,
    TeleportTo,
    Pitfall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleStrengthNoticePhase {
    UseText,
    CryPause,
    MoveText,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleScriptMovement {
    object_id: String,
    phases: VecDeque<VisibleScriptMovementPhase>,
    pending_programs: VecDeque<VisibleScriptMovementProgram>,
    hold_frames_remaining: u16,
    active_jump_duration: Option<u8>,
    active_uses_standing_frame: bool,
    active_tree_shake_duration: Option<u16>,
    active_stationary_effect: Option<VisibleStationaryMovementEffect>,
    active_stationary_duration: u16,
    stationary_y_offset: i16,
    stationary_initial_facing: Direction,
    follower_object_id: Option<String>,
    follower_queued_step: Option<VisibleFollowerStep>,
    follower_active_jump_duration: Option<u8>,
    follower_active_uses_standing_frame: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleFollowerStep {
    direction: Direction,
    stride: u8,
    duration: u8,
    jump: bool,
    standing_frame: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleScriptMovementProgram {
    object_id: String,
    previous_tile: TilePosition,
    previous_facing: Direction,
    previous_hidden: bool,
    phases: VecDeque<VisibleScriptMovementPhase>,
    follower_object_id: Option<String>,
    follower_queued_step: Option<VisibleFollowerStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMomBank {
    phase: VisibleMomBankPhase,
    menu_index: usize,
    yes_no_index: usize,
    amount: u32,
    digit: u8,
    messages: VecDeque<String>,
    close_after_messages: bool,
}

#[derive(Resource)]
struct BevyRuntimeShell {
    /// Presentation-time Game Boy frame. Unlike the semantic save checksum,
    /// this advances while the player stands still so LCD animations continue.
    lcd_animation_frame: u64,
    ambient_tileset_animation_active: bool,
    ambient_tileset_animation_schedule: Vec<(u64, u64)>,
    battle_lcd_animation_active: bool,
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    shell: RuntimeGameShell,
    latest_rtc_sample: Option<RuntimeRtcSample>,
    intro_screen: Option<VisibleIntroScreen>,
    title_menu: Option<TitleMenu>,
    visible_continue_screen: Option<VisibleContinueScreen>,
    credits_screen: Option<VisibleCreditsScreen>,
    last_error: Option<String>,
    last_action_status: Option<String>,
    last_audio_events: Vec<String>,
    pending_audio: Vec<BevyAudioCommand>,
    audio_source_cache: HashMap<BevyAudioCacheKey, Handle<AudioSource>>,
    trainer_items_used: BTreeSet<String>,
    pending_music_stop: bool,
    transient_audio_playing: bool,
    active_music: Option<String>,
    faded_music: Option<String>,
    last_battle_cry_key: Option<String>,
    pending_battle_cries_after_messages: VecDeque<(String, String, String)>,
    battle_enemy_send_out_pending: bool,
    battle_player_send_out_pending: bool,
    battle_enemy_hp_at_player_send_out: Option<u16>,
    pending_battle_scenes_after_message: VecDeque<(String, Box<RuntimeShellSnapshot>)>,
    pending_enemy_response_after_capture: Option<(String, String)>,
    pending_plain_battle_map_reload: bool,
    last_overworld_input: Option<VisibleOverworldInputRecord>,
    // Authoritative movement commits at a tile boundary. Retain its previous
    // tile so presentation can cross that boundary over real LCD frames.
    player_walk_from: Option<TilePosition>,
    player_walk_frame_ticks: u8,
    player_walk_total_ticks: u8,
    player_walk_stride: bool,
    player_walk_mirror_stride: bool,
    object_walk_frame_ticks: u8,
    object_walk_total_ticks: u8,
    object_walk_frame_ticks_by_id: BTreeMap<String, u8>,
    object_walk_total_ticks_by_id: BTreeMap<String, u8>,
    object_walk_stride: bool,
    // Core commits autonomous object tiles atomically. Retain their prior
    // tiles while the shell presents Crystal's eight-frame walk.
    object_walk_from: BTreeMap<String, TilePosition>,
    object_walk_phases: BTreeMap<String, u8>,
    object_walk_directions: BTreeMap<String, Direction>,
    trainer_walk_from: Option<(String, TilePosition)>,
    pending_overworld_step_boundary: Option<PendingOverworldStepBoundary>,
    pending_overworld_warp_scene: Option<Arc<RuntimeShellSnapshot>>,
    visible_script_movement: Option<VisibleScriptMovement>,
    visible_script_movement_scene: Option<Arc<RuntimeShellSnapshot>>,
    overworld_direction_repeat_ticks: u8,
    overworld_held_direction: Option<GameButton>,
    overworld_held_directions: VecDeque<GameButton>,
    overworld_buffered_direction: Option<GameButton>,
    ui_held_direction: Option<GameButton>,
    ui_direction_repeat_ticks: u8,
    recent_overworld_inputs: VecDeque<VisibleOverworldInputRecord>,
    deterministic_session_start: StateChecksum,
    deterministic_session_checkpoint: Option<SessionSaveCheckpointFrame>,
    deterministic_input_frames: VecDeque<PlayerInputFrame>,
    deterministic_battle_actions: VecDeque<BattleActionFrame>,
    deterministic_menu_results: VecDeque<MenuChoiceResultFrame>,
    last_runtime_action: Option<VisibleRuntimeActionRecord>,
    quick_save_path: Option<PathBuf>,
    active_script_cursor: Option<ActiveScriptCursor>,
    pending_map_callbacks: Vec<String>,
    map_callback_return_cursor: Option<RuntimeCompiledScriptCursor>,
    map_reload_return_cursor: Option<RuntimeCompiledScriptCursor>,
    pending_scene_script: Option<String>,
    script_command_cursor: usize,
    start_menu_cursor: Option<MenuCursor>,
    menu_cursor: Option<MenuCursor>,
    sell_cursor: Option<MenuCursor>,
    shop_top_cursor: Option<MenuCursor>,
    shop_quantity: Option<VisibleShopQuantity>,
    shop_notice: Option<String>,
    shop_welcome_seen: bool,
    shop_return_to_top_after_notice: bool,
    shop_close_after_notice: bool,
    elevator_cursor: Option<MenuCursor>,
    gift_pokemon_cursor: Option<MenuCursor>,
    yes_no_cursor: Option<MenuCursor>,
    pending_phone_prompt: Option<PendingPhonePrompt>,
    pending_day_of_week: Option<PendingDayOfWeekPrompt>,
    pending_trainer_sight: Option<PendingTrainerSight>,
    previous_map_sign_landmark: Option<String>,
    visible_map_name_sign: Option<VisibleMapNameSign>,
    pending_delete_save: Option<VisibleDeleteSaveScreen>,
    pending_clock_reset: Option<VisibleClockResetScreen>,
    pending_mystery_gift: Option<VisibleMysteryGiftScreen>,
    pending_time_set: Option<VisibleTimeSetScreen>,
    pending_oak_intro: Option<VisibleOakIntroSequence>,
    pending_gender_selection: Option<VisibleGenderSelection>,
    screen_fade: Option<VisibleScreenFade>,
    visible_blackout_phase: Option<VisibleBlackoutPhase>,
    visible_walk_warp_phase: Option<VisibleWalkWarpPhase>,
    field_text_reveal: Option<VisibleFieldTextReveal>,
    selected_player_gender: Option<VisiblePlayerGender>,
    pending_name_input: Option<PendingNameInput>,
    pending_name_choice: Option<VisibleNameChoice>,
    pending_standard_capture: Option<PendingStandardCapture>,
    party_menu_open: bool,
    party_summary_open: bool,
    party_summary_page: u8,
    party_cursor: usize,
    party_action_cursor: Option<MenuCursor>,
    party_give_take_cursor: Option<MenuCursor>,
    party_mail_take_stage: Option<u8>,
    party_held_item_give_target: Option<usize>,
    held_item_swap_prompt: bool,
    pending_contextual_field_move: Option<PartyFieldMove>,
    pending_script_party_selection: Option<PendingScriptPartySelection>,
    kurt_apricorn_cursor: Option<MenuCursor>,
    kurt_apricorn_quantity: Option<u16>,
    buena_prize_cursor: Option<MenuCursor>,
    visible_unown_puzzle: Option<VisibleUnownPuzzle>,
    visible_slot_machine: Option<VisibleSlotMachine>,
    visible_card_flip: Option<VisibleCardFlip>,
    visible_heal_machine: Option<VisibleHealMachine>,
    visible_magnet_train: Option<VisibleMagnetTrain>,
    visible_unown_words: Option<String>,
    visible_diploma: Option<u8>,
    visible_battle_transition: Option<VisibleBattleTransition>,
    visible_capture_animation: Option<VisibleCaptureAnimation>,
    visible_move_animations: VecDeque<VisibleMoveAnimation>,
    visible_send_out_animation: Option<VisibleSendOutAnimation>,
    visible_trainer_exit_animation: Option<VisibleTrainerExitAnimation>,
    visible_frontpic_animation: Option<VisibleFrontpicAnimation>,
    visible_fishing_animation: Option<VisibleFishingAnimation>,
    heal_music_active: bool,
    party_move_reorder_open: bool,
    party_move_reorder_origin: Option<usize>,
    party_switch_cursor: Option<MenuCursor>,
    party_hp_transfer_source: Option<usize>,
    party_hp_transfer_move: Option<PartyFieldMove>,
    pokedex_menu_open: bool,
    pokedex_detail_open: bool,
    pokedex_detail_page: usize,
    pokedex_scripted_entry: bool,
    pokedex_cursor: usize,
    pokegear_menu_open: bool,
    pokegear_cursor: usize,
    pokegear_phone_cursor: usize,
    pokegear_phone_status: Option<String>,
    pokegear_page: PokegearPage,
    pokegear_radio_station: Option<String>,
    pokegear_radio_segment: usize,
    pokegear_radio_index: usize,
    active_pokegear_radio: Option<(String, String)>,
    trainer_card_open: bool,
    trainer_card_page: VisibleTrainerCardPage,
    trainer_card_colon_visible: bool,
    trainer_card_colon_ticks: u8,
    trainer_card_badge_frame: u8,
    trainer_card_badge_ticks: u8,
    options_menu_open: bool,
    options_cursor: usize,
    save_menu_open: bool,
    save_flow: Option<VisibleSaveFlow>,
    special_boundary: Option<SpecialBoundaryDisplay>,
    special_boundary_queue: VecDeque<SpecialBoundaryDisplay>,
    pending_special_cry: Option<String>,
    pending_special_sound: Option<String>,
    visible_balance_overlay: Option<VisibleBalanceOverlay>,
    visible_mom_bank: Option<VisibleMomBank>,
    visible_overworld_emote: Option<VisibleOverworldEmote>,
    visible_earthquake: Option<VisibleEarthquake>,
    visible_ledge_jump: Option<VisibleLedgeJump>,
    visible_grass_rustle: Option<VisibleGrassRustle>,
    visible_strength_boulder_dust: Option<VisibleStrengthBoulderDust>,
    visible_script_delay_frames: Option<u16>,
    poison_flash_frames_remaining: u8,
    field_pack_pocket: Option<FieldPackPocket>,
    last_field_pack_pocket: FieldPackPocket,
    field_pack_cursor_positions: [usize; 4],
    field_pack_action_cursor: Option<MenuCursor>,
    field_pack_target_mode: Option<FieldPackTargetMode>,
    tmhm_teach_prompt_cursor: Option<MenuCursor>,
    pending_tmhm_teach_prompt_after_boot: bool,
    tmhm_decision_prompt_cursor: Option<MenuCursor>,
    tmhm_decision: Option<VisibleTmHmDecision>,
    tmhm_forget_menu_open: bool,
    move_learn_decision_cursor: Option<MenuCursor>,
    move_learn_decision: Option<VisibleTmHmDecision>,
    move_learn_forget_menu_open: bool,
    battle_pack_target_mode: Option<BattlePackTargetMode>,
    pack_toss: Option<VisiblePackToss>,
    battle_messages: VecDeque<String>,
    battle_text_reveal: Option<VisibleBattleTextReveal>,
    battle_fanfare_messages: VecDeque<String>,
    battle_evolution_cries: VecDeque<(String, String)>,
    battle_evolution_cancellations: VecDeque<VisibleEvolutionCancellation>,
    field_evolution_cancellation: Option<VisibleEvolutionCancellation>,
    battle_sounds_after_messages: VecDeque<(String, String)>,
    battle_entry_messages_remaining: usize,
    battle_message_scene: Option<Box<RuntimeShellSnapshot>>,
    battle_message_scenes: VecDeque<Box<RuntimeShellSnapshot>>,
    battle_hp_tween: Option<VisibleBattleHpTween>,
    battle_exp_tween: Option<VisibleBattleExpTween>,
    pending_battle_exp_tweens: VecDeque<VisibleBattleExpTween>,
    battle_level_stats: VecDeque<VisibleBattleLevelStats>,
    bag_cursor: Option<MenuCursor>,
    key_item_cursor: Option<MenuCursor>,
    ball_cursor: Option<MenuCursor>,
    tmhm_cursor: Option<MenuCursor>,
    custom_item_cursor: Option<MenuCursor>,
    storage_cursor: Option<MenuCursor>,
    pc_item_cursor: Option<MenuCursor>,
    pc_item_action: Option<VisiblePlayerPcAction>,
    pc_item_quantity: Option<VisiblePcItemQuantity>,
    pc_hub_session_open: bool,
    pc_hub_cursor: Option<MenuCursor>,
    hall_of_fame_pc_index: Option<usize>,
    player_pc_action_cursor: Option<MenuCursor>,
    mailbox_cursor: Option<MenuCursor>,
    mailbox_action_cursor: Option<MenuCursor>,
    mailbox_attach_index: Option<usize>,
    pc_confirmation: Option<VisiblePcConfirmation>,
    bill_pc_session_open: bool,
    bill_pc_action_cursor: Option<MenuCursor>,
    bill_pc_box_cursor: Option<MenuCursor>,
    bill_pc_move_open: bool,
    bill_pc_move_source: Option<(usize, usize)>,
    bill_pc_pokemon_action_cursor: Option<MenuCursor>,
    bill_pc_box_summary: Option<VisiblePcBoxSummary>,
    pending_pc_release: Option<VisiblePcReleasePrompt>,
    pc_notice: Option<String>,
    field_notice: Option<String>,
    field_notice_queue: VecDeque<String>,
    pending_sweet_scent_nothing_notice: bool,
    pending_item_notification: Option<String>,
    field_notice_scene: Option<Arc<RuntimeShellSnapshot>>,
    pending_field_travel_arrival: bool,
    pending_field_travel_delay_frames: Option<u16>,
    visible_field_travel_animation: Option<VisibleFieldTravelAnimation>,
    pending_field_notice_sound: Option<String>,
    pending_field_notice_cry: Option<String>,
    visible_strength_notice_phase: Option<VisibleStrengthNoticePhase>,
    pending_field_battle_entry: bool,
    pending_field_notice_effect_frames: Option<u8>,
    visible_sweet_scent_delay: bool,
    visible_cut_animation: Option<VisibleCutAnimation>,
    visible_whirlpool_animation: Option<VisibleWhirlpoolAnimation>,
    visible_headbutt_animation: Option<VisibleHeadbuttAnimation>,
    visible_flash_animation: Option<VisibleFlashAnimation>,
    visible_fly_animation: Option<VisibleFlyAnimation>,
    visible_waterfall_animation: Option<VisibleWaterfallAnimation>,
    pending_surf_start_from: Option<TilePosition>,
    fly_cursor: Option<MenuCursor>,
    battle_action_cursor: Option<MenuCursor>,
    battle_move_cursor: Option<MenuCursor>,
    battle_move_swap_origin: Option<usize>,
    battle_shift_prompt_cursor: Option<MenuCursor>,
    battle_faint_prompt_cursor: Option<MenuCursor>,
    battle_switch_cursor: Option<MenuCursor>,
    battle_party_action_cursor: Option<MenuCursor>,
    battle_party_summary_open: bool,
    pending_battle_move_switch_slot: Option<usize>,
    party_move_cursor: Option<MenuCursor>,
    snapshot_revision: u64,
    cached_snapshot: Option<(u64, Arc<RuntimeShellSnapshot>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleSaveFlowStage {
    Prompt,
    OverwritePrompt,
    Saving,
    Saved,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleShopQuantity {
    item_id: String,
    selling: bool,
    quantity: u16,
    max_quantity: u16,
    unit_price: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePackToss {
    item_id: String,
    quantity: u16,
    max_quantity: u16,
    confirming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePcReleasePrompt {
    box_index: usize,
    box_slot: usize,
    nickname: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePcBoxSummary {
    box_index: usize,
    box_slot: usize,
    page: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTmHmDecision {
    ForgetMove,
    StopLearning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleSaveFlow {
    stage: VisibleSaveFlowStage,
    save_exists: bool,
    yes_no_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMapNameSign {
    landmark: String,
    label: String,
    frames_remaining: u8,
}

const SAVE_TEXT_WOULD_YOU_LIKE: &str = "_WouldYouLikeToSaveTheGameText";
const SAVE_TEXT_ALREADY_EXISTS: &str = "_AlreadyASaveFileText";
const SAVE_TEXT_SAVING: &str = "_SavingDontTurnOffThePowerText";
const SAVE_TEXT_SAVED: &str = "_SavedTheGameText";
const SAVE_TEXT_CORRUPTED: &str = "_SaveFileCorruptedText";

#[cfg(not(test))]
struct NativeAudioBackend {
    output: Option<NativeAudioOutput>,
    music_sink: Option<rodio::Sink>,
    transient_sinks: Vec<rodio::Sink>,
}

#[cfg(not(test))]
struct NativeAudioOutput {
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
}

#[cfg(not(test))]
impl NativeAudioBackend {
    fn new() -> Self {
        Self {
            output: None,
            music_sink: None,
            transient_sinks: Vec::new(),
        }
    }

    fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }

    fn stop_transient(&mut self) {
        for sink in self.transient_sinks.drain(..) {
            sink.stop();
        }
    }

    fn transient_finished(&mut self) -> bool {
        self.transient_sinks.retain(|sink| !sink.empty());
        self.transient_sinks.is_empty()
    }

    fn play(
        &mut self,
        command: &BevyAudioCommand,
        bytes: Arc<[u8]>,
        pcm_loop: Option<(usize, usize)>,
    ) -> Result<()> {
        use rodio::Source as _;

        // Stop the previous channel before decoding/starting the replacement.
        // Starting first leaves a real overlap window on the native backend,
        // which is especially audible when a queued map transition contains
        // several events in one Bevy update.
        if matches!(command.kind, ModpackAudioKind::Music) {
            self.stop_music();
        } else {
            self.stop_transient();
        }
        self.transient_sinks.retain(|sink| !sink.empty());
        if self.output.is_none() {
            let (stream, handle) =
                rodio::OutputStream::try_default().context("open native audio output stream")?;
            self.output = Some(NativeAudioOutput {
                _stream: stream,
                handle,
            });
        }
        let output = self
            .output
            .as_ref()
            .expect("native audio output initialized");
        let sink = rodio::Sink::try_new(&output.handle).context("create native audio sink")?;
        let decoder = rodio::Decoder::new(std::io::Cursor::new(bytes))
            .context("decode generated WAV for native audio playback")?;
        if let Some((loop_start_sample, loop_end_sample)) = pcm_loop {
            let channels = decoder.channels();
            let sample_rate = decoder.sample_rate();
            let samples = decoder.collect::<Vec<i16>>();
            sink.append(PcmLoopSource::new(
                samples,
                channels,
                sample_rate,
                loop_start_sample,
                loop_end_sample,
            )?);
        } else if native_audio_repeats_without_pcm_loop(command) {
            sink.append(decoder.repeat_infinite());
        } else {
            sink.append(decoder);
        }
        sink.play();
        if matches!(command.kind, ModpackAudioKind::Music) {
            self.music_sink = Some(sink);
        } else {
            self.transient_sinks.push(sink);
        }
        Ok(())
    }
}

fn native_audio_repeats_without_pcm_loop(command: &BevyAudioCommand) -> bool {
    // A rendered PCM asset is finite unless the exporter supplied exact loop
    // sample bounds.  Repeating the whole file restarts one-shot score cues
    // such as the Crystal opening and makes them play forever.
    command.looped && matches!(command.mode, ModpackAudioPlaybackMode::SequencedMidi)
}

#[cfg(not(test))]
struct PcmLoopSource {
    samples: Vec<i16>,
    position: usize,
    loop_start: usize,
    loop_end: usize,
    channels: u16,
    sample_rate: u32,
}

#[cfg(not(test))]
impl PcmLoopSource {
    fn new(
        samples: Vec<i16>,
        channels: u16,
        sample_rate: u32,
        loop_start_sample: usize,
        loop_end_sample: usize,
    ) -> Result<Self> {
        let channels_usize = usize::from(channels);
        if channels_usize == 0 || samples.len() % channels_usize != 0 {
            anyhow::bail!("decoded PCM loop source has an invalid channel layout");
        }
        let frame_count = samples.len() / channels_usize;
        if loop_start_sample >= loop_end_sample || loop_end_sample > frame_count {
            anyhow::bail!(
                "decoded PCM loop range [{loop_start_sample}, {loop_end_sample}) is outside {frame_count} frames"
            );
        }
        Ok(Self {
            samples,
            position: 0,
            loop_start: loop_start_sample * channels_usize,
            loop_end: loop_end_sample * channels_usize,
            channels,
            sample_rate,
        })
    }
}

#[cfg(not(test))]
impl Iterator for PcmLoopSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.loop_end {
            self.position = self.loop_start;
        }
        let sample = self.samples.get(self.position).copied();
        self.position += 1;
        sample
    }
}

#[cfg(not(test))]
impl rodio::Source for PcmLoopSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleOverworldInputRecord {
    frame: u64,
    input_mask: u8,
    pressed_mask: u8,
    player_moved: bool,
    state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleRuntimeActionRecord {
    action: String,
    frame: u64,
    state_hash: u32,
}

fn deterministic_input_frame_from_post_tick_checksum(
    state_checksum: &StateChecksum,
) -> Result<u64> {
    state_checksum.frame().checked_sub(1).with_context(|| {
        format!(
            "visible runtime cannot derive deterministic input frame before post-tick frame {}",
            state_checksum.frame()
        )
    })
}

fn record_visible_overworld_input(
    runtime_shell: &mut BevyRuntimeShell,
    input: VisibleOverworldInputRecord,
) -> Result<()> {
    let deterministic_frame = PlayerInputFrame::new(
        LOCAL_PLAYER_ID,
        Frame(input.frame),
        input.input_mask,
    )
    .with_context(|| {
        format!(
            "visible runtime produced invalid deterministic joypad mask {:#010b} at frame {}",
            input.input_mask, input.frame
        )
    })?;
    runtime_shell.last_overworld_input = Some(input.clone());
    runtime_shell.recent_overworld_inputs.push_back(input);
    while runtime_shell.recent_overworld_inputs.len() > RECENT_OVERWORLD_INPUT_LIMIT {
        runtime_shell.recent_overworld_inputs.pop_front();
    }
    runtime_shell
        .deterministic_input_frames
        .push_back(deterministic_frame);
    Ok(())
}

fn record_visible_runtime_action(
    runtime_shell: &mut BevyRuntimeShell,
    action: impl Into<String>,
) -> Result<()> {
    let state_checksum = runtime_shell.shell.state_checksum()?;
    runtime_shell.last_runtime_action = Some(VisibleRuntimeActionRecord {
        action: action.into(),
        frame: state_checksum.frame(),
        state_hash: state_checksum.hash(),
    });
    Ok(())
}

fn set_visible_runtime_action_from_checksum(
    runtime_shell: &mut BevyRuntimeShell,
    action: impl Into<String>,
    state_checksum: &StateChecksum,
) {
    runtime_shell.last_runtime_action = Some(VisibleRuntimeActionRecord {
        action: action.into(),
        frame: state_checksum.frame(),
        state_hash: state_checksum.hash(),
    });
}

fn record_visible_runtime_error(runtime_shell: &mut BevyRuntimeShell, error: &anyhow::Error) {
    let action = format!("runtime:error:{error}");
    if let Ok(snapshot) = runtime_shell.shell.snapshot() {
        set_visible_runtime_action_from_checksum(runtime_shell, action, &snapshot.state_checksum);
    } else {
        runtime_shell.last_runtime_action = Some(VisibleRuntimeActionRecord {
            action,
            frame: 0,
            state_hash: 0,
        });
    }
}

fn record_visible_runtime_system_error(runtime_shell: &mut BevyRuntimeShell, error: anyhow::Error) {
    record_visible_runtime_error(runtime_shell, &error);
    runtime_shell.last_error = Some(format!("{error:#?}"));
}

fn record_visible_battle_action_frame(
    runtime_shell: &mut BevyRuntimeShell,
    action: BattleAction,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let state_hash = format!("{:08x}", snapshot.state_checksum.hash());
    let action_frame = BattleActionFrame::new(
        LOCAL_PLAYER_ID,
        snapshot.state_checksum.frame(),
        action,
        state_hash,
    )
    .context("visible runtime produced invalid deterministic battle action")?;
    runtime_shell
        .deterministic_battle_actions
        .push_back(action_frame);
    Ok(())
}

fn record_visible_battle_item_action_frame(
    runtime_shell: &mut BevyRuntimeShell,
    item_id: &str,
) -> Result<()> {
    record_visible_battle_action_frame(
        runtime_shell,
        BattleAction::Item {
            item_id: item_id.to_string(),
        },
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TitleMenu {
    spawn_identifier: u16,
    save_path: Option<PathBuf>,
    cursor: MenuCursor,
    phase: VisibleTitlePhase,
    frame: u32,
    main_menu_frame: u32,
    scx: u8,
    title_timer: u16,
    clock_reset_trigger: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleContinueScreen {
    save_path: PathBuf,
    player_name: String,
    badge_count: usize,
    pokedex_count: Option<usize>,
    hours: u16,
    minutes: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleIntroScreen {
    jumptable_index: usize,
    scene_frame_counter: u8,
    next_scene_frame_counter: Option<u8>,
    scene_delay_frames: u8,
    scene_timer: u8,
    scroll_x: u8,
    scroll_y: u8,
    global_anim_x_offset: u8,
    sprite_count: u8,
    sprites: Vec<VisibleIntroSprite>,
    palette_effect: VisibleIntroPaletteEffect,
    finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleIntroPaletteEffect {
    None,
    UnownFade { palette_idx: u8, timer: u8 },
    AppearUnown { palette_set_idx: u8, revealed: u8 },
    Scene24Fade { fade_index: u8 },
    CrystalWordFade { fade_level: u8, timer: u8 },
    ClearBg,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleIntroSprite {
    x: i16,
    y: i16,
    oam_attr: u8,
    gfx_name: String,
    jumptable_index: u8,
    frame_timer: u8,
    frameset_step: i16,
    start_delay: u8,
    x_offset: i16,
    y_offset: i16,
    var1: u8,
    var2: u8,
    frameset_name: String,
    object_name: String,
    anim_function: String,
    current_oam_set: Option<String>,
    attr_flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTimeSetPhase {
    WakeDialogue,
    SetHour,
    HourConfirm,
    SetMinute,
    MinuteConfirm,
    FinalReaction,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleTimeSetScreen {
    phase: VisibleTimeSetPhase,
    next: VisibleTimeSetNext,
    wake_index: usize,
    hour: u8,
    minute: u8,
    visible_chars: usize,
    text_timer: u8,
    yes_no_index: usize,
    reaction_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTimeSetNext {
    OakIntro,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTimeSetDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleOakIntroMode {
    Intro,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleOakIntroPhase {
    FadeIn,
    WipeIn,
    Text,
    TextOne,
    Cry,
    TextTwo,
    FadeOut,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleOakFadeDirection {
    In,
    Out,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleOakIntroSequence {
    mode: VisibleOakIntroMode,
    scene_index: usize,
    scene_state: String,
    scene_phase: VisibleOakIntroPhase,
    current_sprite: Option<String>,
    wooper_cry_queued: bool,
    scene_fade_out_steps: u8,
    fade_active: bool,
    fade_direction: VisibleOakFadeDirection,
    fade_total_frames: u16,
    fade_elapsed: u16,
    fade_alpha: u8,
    wipe_active: bool,
    wipe_window_x: u16,
    text_queue: Vec<String>,
    current_text: String,
    visible_chars: usize,
    text_timer: u8,
    waiting_for_input: bool,
    blink_timer: u8,
    finished: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTitlePhase {
    Entrance,
    Timer,
    PressStart,
    MainMenu,
    Timeout,
    Exiting,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleCreditsScreen {
    allow_skip: bool,
    resume_game_timer_on_exit: bool,
    frame: u32,
    consumed_bytes: u16,
    awaiting_exit: bool,
    scene_index: u8,
    timer: u8,
    script_index: usize,
    jumptable_index: u8,
    lines: Vec<VisibleCreditsLine>,
    border_frame_counter: Option<u8>,
    border_frame_top: Option<VisibleCreditsBorderFrame>,
    border_frame_bottom: Option<VisibleCreditsBorderFrame>,
    border_frame_pending: Option<VisibleCreditsBorderFrame>,
    border_frame_pending_blank: bool,
    border_mon_index: u8,
    ly_override: u8,
    show_the_end: bool,
    script_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleCreditsBorderFrame {
    mon_index: u8,
    frame_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleCreditsLine {
    token: String,
    text: String,
    tiles: Vec<Vec<u16>>,
    line_index: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleDeleteSaveScreen {
    selected_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMysteryGiftScreen {
    message: String,
    awaiting_exchange: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleClockResetScreen {
    phase: VisibleClockResetPhase,
    confirm_selection: usize,
    day: u8,
    hour: u8,
    minute: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleClockResetPhase {
    Confirm,
    SetDay,
    SetHour,
    SetMinute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum VisibleCreditsOp {
    String { token: String, line_index: u8 },
    Wait(u8),
    Wait2(u8),
    Scene(u8),
    Clear,
    Music,
    TheEnd,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitleMenuOption {
    Continue,
    NewGame,
    Options,
    MysteryGift,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleGenderSelection {
    selected_index: usize,
    confirmed: bool,
    confirm_countdown: u8,
    fade_counter: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisiblePlayerGender {
    Boy,
    Girl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveScriptCursor {
    origin_map_name: String,
    source_script: String,
    next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MenuCursor {
    surface_id: String,
    option_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingPhonePrompt {
    source_script: String,
    command_index: usize,
    contact_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingDayOfWeekPrompt {
    origin_map_name: String,
    source_script: String,
    command_index: usize,
    selected_day: u8,
    confirming: bool,
    yes_no_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingNameInput {
    label: String,
    value: String,
    max_length: usize,
    cursor_column: usize,
    cursor_row: usize,
    case: NameInputCase,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingStandardCapture {
    outcome: crate::core::battle::capture::CaptureOutcome,
    scripted_static_wild: Option<VisibleStaticWildOrigin>,
    default_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PendingScriptPartySelection {
    BillsGrandfather,
    ReturnShuckie,
    CheckMagikarpLength,
    PhotoStudio,
    PokeSeer,
    NameRater,
    OlderHaircutBrother,
    YoungerHaircutBrother,
    DaisysGrooming,
    DayCareDeposit {
        caretaker: String,
    },
    MoveTutor {
        move_id: String,
        party_index: Option<usize>,
    },
    MoveDeletion {
        party_index: Option<usize>,
    },
    CheckPokeMail {
        origin_map_name: String,
        source_script: String,
        command_index: usize,
    },
    NpcTrade {
        origin_map_name: String,
        source_script: String,
        command_index: usize,
        trade_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleNameChoice {
    options: Vec<String>,
    selected: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum NameInputCase {
    Upper,
    Lower,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SpecialBoundaryDisplay {
    label: String,
    details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleUnownPuzzle {
    puzzle_id: String,
    layout: [[u8; 6]; 6],
    holding_piece: Option<u8>,
    cursor_x: usize,
    cursor_y: usize,
    moves: u16,
    solved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleSlotMachine {
    bet: u8,
    coins: u16,
    payout: u16,
    windows: [[String; 3]; 3],
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleCardFlipPhase {
    AskPlay,
    ChooseCard,
    PlaceBet,
    PlayAgain,
    NotEnoughCoins,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleCardFlip {
    phase: VisibleCardFlipPhase,
    yes_no_index: usize,
    which_card: usize,
    bet_x: usize,
    bet_y: usize,
    round: usize,
    face_card: Option<(String, u8)>,
    coins: u16,
    payout: u16,
    deck: Vec<String>,
    revealed: Vec<bool>,
    message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleOverworldEmote {
    emote: String,
    object: String,
    frames_remaining: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleCutAnimation {
    target_tile: TilePosition,
    facing: Direction,
    variant: String,
    frame: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleWhirlpoolAnimation {
    target_tile: TilePosition,
    frame: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleHeadbuttAnimation {
    target_tile: TilePosition,
    facing: Direction,
    frame: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleFlashAnimation {
    frame: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleWaterfallAnimation {
    from_tile: TilePosition,
    to_tile: TilePosition,
    steps: u16,
    frame: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleFlyAnimationPhase {
    From,
    To,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleFlyAnimation {
    phase: VisibleFlyAnimationPhase,
    frame: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleHealMachine {
    kind: u8,
    party_count: u8,
    frame: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleMagnetTrain {
    direction: i16,
    hold_position: i16,
    final_position: i16,
    position: i16,
    offset: i16,
    wait_counter: u16,
    phase: u8,
    arrival_sfx_played: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleBattleTransition {
    frame: u16,
    stronger_enemy: bool,
    cave_environment: bool,
    trainer_battle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleCaptureAnimation {
    trigger_message: String,
    ball_id: String,
    animation_shakes: u8,
    blocked: bool,
    caught: bool,
    started: bool,
    complete: bool,
    sprites_cleared: bool,
    frame: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMoveAnimation {
    trigger_message: String,
    move_id: String,
    animation_label: String,
    player_move: bool,
    started: bool,
    waiting_for_hp: bool,
    frame: u16,
    total_frames: u16,
    sound_events: Vec<(u16, String)>,
    next_sound_event: usize,
    cry_events: Vec<(u16, u8)>,
    next_cry_event: usize,
    object_events: Vec<VisibleMoveObjectEvent>,
    bg_events: Vec<VisibleMoveBgEvent>,
    actor_species_override: Option<String>,
    actor_shiny_override: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMoveObjectEvent {
    frame: u16,
    command: VisibleMoveObjectCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleMoveObjectCommand {
    Spawn {
        object_id: String,
        x: i16,
        y: i16,
        param: u8,
    },
    Clear,
    Increment {
        slot: u8,
    },
    Set {
        slot: u8,
        value: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMoveBgEvent {
    frame: u16,
    effect_id: String,
    duration: u16,
    target: String,
    param: u8,
    incremented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleBattlerArtOverride {
    Unchanged,
    Pokemon,
    Transform,
    Substitute,
    Minimize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleSendOutAnimation {
    side: crate::core::battle::turn::BattleSide,
    frame: u8,
    shiny: bool,
}

impl VisibleSendOutAnimation {
    const NORMAL_FRAMES: u8 = 36;
    const SHINY_FRAMES: u8 = 64;

    fn total_frames(&self) -> u8 {
        Self::NORMAL_FRAMES + if self.shiny { Self::SHINY_FRAMES } else { 0 }
    }

    fn battler_scale(&self) -> f32 {
        if self.frame < 4 { 0.0 } else { 1.0 }
    }

    fn battler_clip_tiles(&self) -> Option<u8> {
        if self.frame < 4 {
            return None;
        }
        match (self.side, self.frame - 4) {
            (crate::core::battle::turn::BattleSide::Player, 0) => Some(2),
            (crate::core::battle::turn::BattleSide::Player, 1) => Some(4),
            (crate::core::battle::turn::BattleSide::Enemy, 0) => Some(3),
            (crate::core::battle::turn::BattleSide::Enemy, 1) => Some(5),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleTrainerExitAnimation {
    side: crate::core::battle::turn::BattleSide,
    frame: u8,
    send_out_after: bool,
}

impl VisibleTrainerExitAnimation {
    fn total_frames(&self) -> u8 {
        match self.side {
            crate::core::battle::turn::BattleSide::Player => 27,
            crate::core::battle::turn::BattleSide::Enemy => 24,
        }
    }

    fn x_offset(&self) -> f32 {
        let steps = self.frame / 3 + 1;
        let pixels = f32::from(steps) * TILE_SIZE;
        match self.side {
            crate::core::battle::turn::BattleSide::Player => -pixels,
            crate::core::battle::turn::BattleSide::Enemy => pixels,
        }
    }
}

impl VisibleCaptureAnimation {
    fn throw_active(&self) -> bool {
        self.started && !self.complete
    }

    fn ball_visible(&self) -> bool {
        self.throw_active() || (self.complete && self.caught && !self.sprites_cleared)
    }

    fn retained_objects_visible(&self) -> bool {
        self.throw_active() || (self.complete && !self.sprites_cleared)
    }

    fn shake_setup_frame(&self) -> u16 {
        // Ordinary branches enter .Shake at 68. Master Ball waits another
        // 24 frames before its 64-frame sparkle branch, entering at 140.
        // .Shake then spends 160 frames before the first 48-frame check loop.
        if self.ball_id.eq_ignore_ascii_case("MASTER_BALL") {
            300
        } else {
            228
        }
    }

    fn first_shake_check_frame(&self) -> u16 {
        self.shake_setup_frame().saturating_add(48)
    }

    fn total_frames(&self) -> u16 {
        if self.blocked {
            52
        } else if self.caught {
            self.shake_setup_frame() + 48 * u16::from(self.animation_shakes.max(1))
        } else {
            self.shake_setup_frame() + 48 * (u16::from(self.animation_shakes) + 1) + 34
        }
    }

    fn enemy_hidden(&self) -> bool {
        if (!self.started && !self.complete) || self.blocked || self.frame < 76 {
            return false;
        }
        self.caught || self.frame + 32 < self.total_frames()
    }

    fn enemy_clip_tiles(&self) -> Option<u8> {
        if (!self.started && !self.complete) || self.blocked || self.frame < 68 {
            return None;
        }
        if self.frame < 76 {
            return match self.frame - 68 {
                0 => Some(7),
                1 => Some(5),
                _ => Some(3),
            };
        }
        if self.caught {
            return None;
        }
        let expansion_start = self.total_frames().saturating_sub(32);
        if self.frame < expansion_start {
            return None;
        }
        match self.frame - expansion_start {
            0 => Some(3),
            1 => Some(5),
            _ => Some(7),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingTrainerSight {
    interaction: crate::core::world::session::OverworldInteraction,
    object_id: String,
    direction: Direction,
    steps_remaining: u16,
    frames_until_step: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleEarthquake {
    intensity: u16,
    frames_remaining: u16,
    phase: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleLedgeJump {
    from: TilePosition,
    to: TilePosition,
    frame: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleGrassRustle {
    tile: TilePosition,
    frames_remaining: u8,
    age: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleStrengthBoulderDust {
    object_id: String,
    direction: Direction,
    frames_remaining: u8,
    age: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleBattleHpTween {
    player_hp: u16,
    player_target_hp: u16,
    player_max_hp: u16,
    player_pixels: u16,
    player_target_pixels: u16,
    player_frames_until_step: u8,
    enemy_pixels: u16,
    enemy_target_pixels: u16,
    enemy_frames_until_step: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBattleExpTween {
    trigger_message: String,
    started: bool,
    pixels: u16,
    level: u8,
    target_pixels: u16,
    remaining_targets: VecDeque<u16>,
    frames_until_step: u8,
    steps_in_segment: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBattleLevelStats {
    trigger_message: String,
    triggered: bool,
    active: bool,
    frames_before_input: u8,
    attack: u16,
    defense: u16,
    speed: u16,
    special_attack: u16,
    special_defense: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleEvolutionCancellation {
    party_index: usize,
    trigger_message: String,
    evolved_message: String,
    pending_move_messages: Vec<String>,
    report: EvolutionReport,
}

#[derive(Component)]
struct MainCameraMarker;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PokegearPage {
    Clock,
    Map,
    Phone,
    Radio,
}

const VISIBLE_POKEGEAR_RADIO_FREQUENCIES: [(f32, &str); 9] = [
    (4.50, "PKMNTalkAndPokedexShow"),
    (7.50, "PokemonMusic"),
    (8.50, "LuckyChannel"),
    (10.50, "BuenasPassword"),
    (13.50, "RuinsOfAlphRadio"),
    (16.50, "PlacesAndPeople"),
    (18.50, "LetsAllSing"),
    (20.00, "PokeFluteRadio"),
    (20.50, "EvolutionRadio"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartMenuOption {
    Pokedex,
    Pokemon,
    Pack,
    Pokegear,
    TrainerCard,
    Save,
    QuitContest,
    Options,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisiblePcHubAction {
    BillsPc,
    PlayerPc,
    OakPc,
    HallOfFame,
    TurnOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleBillPcAction {
    Withdraw,
    Deposit,
    ChangeBox,
    MoveWithoutMail,
    SeeYa,
}

const VISIBLE_BILL_PC_ACTIONS: [VisibleBillPcAction; 5] = [
    VisibleBillPcAction::Withdraw,
    VisibleBillPcAction::Deposit,
    VisibleBillPcAction::ChangeBox,
    VisibleBillPcAction::MoveWithoutMail,
    VisibleBillPcAction::SeeYa,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisiblePlayerPcAction {
    WithdrawItem,
    DepositItem,
    TossItem,
    MailBox,
    Decoration,
    LogOff,
    TurnOff,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisiblePcConfirmation {
    TossItem {
        item_id: String,
        quantity: u16,
    },
    PutMailInPack(usize),
    NpcTrade(PendingScriptPartySelection),
    ScriptPartyIntro(PendingScriptPartySelection),
    MoveDeletion {
        party_index: usize,
        move_index: usize,
    },
    MoveTutorForget {
        move_id: String,
        party_index: usize,
    },
    MoveTutorStop {
        move_id: String,
        party_index: usize,
    },
    BuenaPrize {
        item_id: String,
    },
    DayCareWithdraw {
        caretaker: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePcItemQuantity {
    action: VisiblePlayerPcAction,
    item_id: String,
    quantity: u16,
    maximum: u16,
}

const VISIBLE_PLAYER_PC_ACTIONS: [VisiblePlayerPcAction; 5] = [
    VisiblePlayerPcAction::WithdrawItem,
    VisiblePlayerPcAction::DepositItem,
    VisiblePlayerPcAction::TossItem,
    VisiblePlayerPcAction::MailBox,
    VisiblePlayerPcAction::LogOff,
];

const VISIBLE_PLAYERS_HOUSE_PC_ACTIONS: [VisiblePlayerPcAction; 6] = [
    VisiblePlayerPcAction::WithdrawItem,
    VisiblePlayerPcAction::DepositItem,
    VisiblePlayerPcAction::TossItem,
    VisiblePlayerPcAction::MailBox,
    VisiblePlayerPcAction::Decoration,
    VisiblePlayerPcAction::TurnOff,
];

const VISIBLE_MAILBOX_ACTIONS: [&str; 4] = ["READ MAIL", "PUT IN PACK", "ATTACH MAIL", "CANCEL"];

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum FieldPackPocket {
    Items,
    Balls,
    KeyItems,
    TmHm,
    Custom(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FieldPackAction {
    Use,
    Give,
    Toss,
    Select,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FieldPackTargetMode {
    PartyPokemon,
    PartyMove,
    TmHmPokemon,
    HeldItem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BattlePackTargetMode {
    PartyPokemon,
    PartyMove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleBattleAction {
    Fight,
    Pokemon,
    Pack,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptionsMenuItem {
    TextSpeed,
    BattleScene,
    BattleStyle,
    Sound,
    Print,
    MenuAccount,
    Frame,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PartyAction {
    Summary,
    Switch,
    Move,
    Item,
    Cancel,
    FieldMove(PartyFieldMove),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PartyFieldMove {
    Surf,
    Cut,
    Fly,
    Strength,
    Flash,
    Waterfall,
    Dig,
    Teleport,
    Headbutt,
    Whirlpool,
    RockSmash,
    SweetScent,
    Softboiled,
    MilkDrink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BevyAudioCommand {
    audio_id: String,
    kind: ModpackAudioKind,
    mode: ModpackAudioPlaybackMode,
    looped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BevyAudioCacheKey {
    audio_id: String,
    kind: &'static str,
    mode: &'static str,
    looped: bool,
}

impl BevyAudioCacheKey {
    fn from_command(command: &BevyAudioCommand) -> Self {
        Self {
            audio_id: command.audio_id.clone(),
            kind: match command.kind {
                ModpackAudioKind::Music => "music",
                ModpackAudioKind::SoundEffect => "sound_effect",
                ModpackAudioKind::Cry => "cry",
            },
            mode: match command.mode {
                ModpackAudioPlaybackMode::SequencedMidi => "sequenced_midi",
                ModpackAudioPlaybackMode::RawPcm => "raw_pcm",
            },
            looped: command.looped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BevyAudioAction {
    Play(BevyAudioCommand),
    FadeMusic { audio_id: String, fade_frames: u16 },
    WaitForSoundEffect,
}

fn enqueue_bevy_audio_command(queue: &mut Vec<BevyAudioCommand>, command: BevyAudioCommand) {
    if matches!(command.kind, ModpackAudioKind::Music) {
        clear_pending_music_commands(queue);
    }
    queue.push(command);
}

fn clear_pending_music_commands(queue: &mut Vec<BevyAudioCommand>) {
    queue.retain(|pending| !matches!(pending.kind, ModpackAudioKind::Music));
}

fn coalesce_pending_transient_audio(pending: Vec<BevyAudioCommand>) -> Vec<BevyAudioCommand> {
    let last_music = pending
        .iter()
        .rposition(|command| matches!(command.kind, ModpackAudioKind::Music));
    let last_transient = pending
        .iter()
        .rposition(|command| !matches!(command.kind, ModpackAudioKind::Music));
    pending
        .into_iter()
        .enumerate()
        .filter(|(index, _command)| {
            (Some(*index) == last_music) || (Some(*index) == last_transient)
        })
        .map(|(_, command)| command)
        .collect()
}

fn pcm_loop_range_for_bevy_audio_command(
    runtime_audio: &crate::RuntimeAudioCatalog,
    command: &BevyAudioCommand,
) -> Result<Option<(usize, usize)>> {
    if !command.looped {
        return Ok(None);
    }
    let program = match command.kind {
        ModpackAudioKind::Music => runtime_audio.require_music(&command.audio_id)?,
        ModpackAudioKind::SoundEffect => runtime_audio.require_sound_effect(&command.audio_id)?,
        ModpackAudioKind::Cry => runtime_audio.require_cry(&command.audio_id)?,
    };
    match &program.source {
        AudioProgramSource::Pcm {
            loop_start_sample,
            loop_end_sample,
            ..
        }
        | AudioProgramSource::PcmGzip {
            loop_start_sample,
            loop_end_sample,
            ..
        } => match (*loop_start_sample, *loop_end_sample) {
            (Some(start), Some(end)) => Ok(Some((start, end))),
            (None, None) => Ok(None),
            _ => anyhow::bail!("verified PCM audio source has unpaired loop metadata"),
        },
        AudioProgramSource::Midi(_) => Ok(None),
    }
}

#[derive(Resource)]
struct RuntimeTickTimer {
    step_seconds: f64,
    accumulated_seconds: f64,
    finished_vblanks: u32,
    finished_ticks: u32,
    presentation_ticks: u32,
}

impl RuntimeTickTimer {
    fn new(step_seconds: f64) -> Self {
        Self {
            step_seconds,
            accumulated_seconds: 0.0,
            finished_vblanks: 0,
            finished_ticks: 0,
            presentation_ticks: 0,
        }
    }

    fn tick(&mut self, delta_seconds: f64) {
        if self.step_seconds <= 0.0 {
            self.finished_vblanks = self.finished_vblanks.saturating_add(1);
            self.finished_ticks = self.finished_ticks.saturating_add(1);
            return;
        }
        self.accumulated_seconds += delta_seconds.max(0.0);
        let ticks = (self.accumulated_seconds / self.step_seconds).floor() as u32;
        if ticks > 0 {
            // GameTimer runs on every elapsed VBlank. Gameplay/input catch-up
            // remains deliberately bounded so a host stall cannot fast-
            // forward movement or consume buffered joypad commands.
            self.finished_vblanks = self.finished_vblanks.saturating_add(ticks);
            self.finished_ticks = self
                .finished_ticks
                .saturating_add(ticks.min(MAX_RUNTIME_CATCH_UP_TICKS));
            self.accumulated_seconds -= self.step_seconds * f64::from(ticks);
        }
    }

    fn take_ticks(&mut self) -> u32 {
        std::mem::take(&mut self.finished_ticks)
    }

    fn take_vblanks(&mut self) -> u32 {
        std::mem::take(&mut self.finished_vblanks)
    }

    fn has_tick(&self) -> bool {
        self.finished_ticks > 0
    }

    fn stage_presentation_ticks(&mut self, ticks: u32) {
        // This is a same-update handoff from the authoritative frame system
        // to the modal/hotkey system. Overwrite instead of accumulating so a
        // presentation early return can never queue input work for a later
        // host update.
        self.presentation_ticks = ticks;
    }

    fn take_presentation_ticks(&mut self) -> u32 {
        std::mem::take(&mut self.presentation_ticks)
    }
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
enum NativeRtcSource {
    SystemLocal,
    Fixed(RuntimeRtcSample),
}

impl NativeRtcSource {
    fn system_local() -> Self {
        Self::SystemLocal
    }

    #[cfg(test)]
    fn fixed(sample: RuntimeRtcSample) -> Self {
        Self::Fixed(sample)
    }

    fn sample(self) -> RuntimeRtcSample {
        match self {
            Self::SystemLocal => {
                let now = ChronoLocal::now();
                RuntimeRtcSample {
                    date: GameDate::new(now.year(), now.month() as u8, now.day() as u8),
                    hour: now.hour() as u8,
                    minute: now.minute() as u8,
                    second: now.second() as u8,
                }
            }
            Self::Fixed(sample) => sample,
        }
    }
}

fn required_native_rtc_sample(runtime_shell: &BevyRuntimeShell) -> Result<RuntimeRtcSample> {
    runtime_shell
        .latest_rtc_sample
        .context("native RTC sample is required before changing the in-game clock")
}

#[derive(Resource)]
struct VisibleSequenceTickClock {
    accumulated_seconds: f32,
    step_seconds: f32,
}

impl VisibleSequenceTickClock {
    fn realtime() -> Self {
        Self {
            accumulated_seconds: 0.0,
            step_seconds: GAME_TICK_SECONDS,
        }
    }

    fn deterministic_test() -> Self {
        Self {
            accumulated_seconds: 0.0,
            step_seconds: 0.0,
        }
    }

    fn consume_frames(&mut self, delta_seconds: f32) -> usize {
        if self.step_seconds <= 0.0 {
            return 1;
        }
        self.accumulated_seconds += delta_seconds.max(0.0);
        let frames = (self.accumulated_seconds / self.step_seconds).floor() as usize;
        if frames == 0 {
            return 0;
        }
        // Visible title/Oak/credits animation is presentation, not a replay
        // catch-up loop. Keep a small bounded catch-up so a 20-30 Hz host
        // still displays the original 60 Hz sequence duration, while a long
        // stall cannot fast-forward through an entire fade or page of text.
        let frames = frames.min(MAX_VISIBLE_SEQUENCE_CATCH_UP_FRAMES);
        self.accumulated_seconds -= self.step_seconds * frames as f32;
        frames
    }
}

#[derive(Resource, Default)]
struct RenderedViewport {
    map_name: Option<String>,
    tile: Option<TilePosition>,
    map_texture: Option<Handle<Image>>,
    map_priority_texture: Option<Handle<Image>>,
    /// Previous fully-settled viewport.  The live composite moves by up to
    /// one render tile while the camera catches up to a committed step; this
    /// stable copy fills the edge that movement exposes instead of allowing
    /// the window clear colour to show through.
    map_backing_texture: Option<Handle<Image>>,
    map_backing_priority_texture: Option<Handle<Image>>,
    /// Read-only presentation metadata for an optional overworld renderer.
    /// These cells describe the same 20x18 source-tile viewport as
    /// `map_texture`; they never participate in collision or movement.
    visual_tiles: Vec<crystal_render_api::VisualTile>,
    viewport_origin: Option<(i16, i16)>,
    /// The viewport origin shown immediately before a committed walking step.
    /// Retaining it lets the renderer scroll the replacement texture over the
    /// same LCD frames as the player sprite instead of snapping the camera.
    walk_viewport_origin: Option<(i16, i16)>,
    map_visual_key: Option<u64>,
    world_key: Option<u64>,
    position_key: Option<u64>,
    appearance_key: Option<u64>,
    state_hash: Option<u32>,
    /// Stable visual identity for the dialog layer.  Runtime checksums also
    /// change while scripts wait/advance, but the dialog pixels often do not;
    /// retaining this key avoids despawning and recreating every glyph entity
    /// on those bookkeeping-only frames.
    dialog_key: Option<u64>,
    shell_render_key: Option<u64>,
    snapshot_revision: Option<u64>,
    title_active: bool,
    /// Direction used to construct the currently retained player frames.
    /// A transform can be interpolated in place, but its texture must be
    /// rebuilt when the authoritative facing changes.
    player_sprite_facing: Option<Direction>,
    player_sprite_mode: Option<MovementMode>,
    player_sprite_walking: Option<bool>,
    object_sprite_walking: Option<bool>,
}

#[derive(Resource, Default)]
struct RenderedTilesetArt {
    cache: HashMap<TilesetArtKey, TilesetArt>,
    errors: HashMap<TilesetArtKey, String>,
    sprite_cache: HashMap<SpriteArtKey, SpriteArt>,
    sprite_errors: HashMap<SpriteArtKey, String>,
    emote_cache: HashMap<String, SpriteFrame>,
    emote_errors: HashMap<String, String>,
    ledge_shadow: Option<SpriteFrame>,
    ledge_shadow_error: Option<String>,
    grass_rustle_cache: HashMap<String, [SpriteFrame; 2]>,
    grass_rustle_errors: HashMap<String, String>,
    boulder_dust_cache: HashMap<String, [SpriteFrame; 2]>,
    boulder_dust_errors: HashMap<String, String>,
    field_move_tile_cache: HashMap<(String, String, u8), SpriteFrame>,
    heal_machine_ball_cache: Option<[SpriteFrame; 4]>,
    heal_machine_lamp_cache: Option<[SpriteFrame; 4]>,
    heal_machine_ball_error: Option<String>,
    magnet_train_base_cache: Option<Vec<u8>>,
    magnet_train_base_error: Option<String>,
    diploma_base_cache: Option<Vec<u8>>,
    diploma_base_error: Option<String>,
    slot_machine_sources: Option<SlotMachineRenderSources>,
    slot_machine_source_error: Option<String>,
    card_flip_sources: Option<CardFlipRenderSources>,
    card_flip_source_error: Option<String>,
    unown_puzzle_sources: Option<UnownPuzzleRenderSources>,
    unown_puzzle_source_error: Option<String>,
    map_name_sign_cache: HashMap<String, Vec<SpriteFrame>>,
    map_name_sign_errors: HashMap<String, String>,
    party_icon_cache: HashMap<String, [SpriteFrame; 2]>,
    party_icon_errors: HashMap<String, String>,
    party_icon_overlay_cache: HashMap<String, SpriteFrame>,
    party_icon_overlay_errors: HashMap<String, String>,
    move_description_cache: Option<HashMap<String, String>>,
    move_description_error: Option<String>,
    battle_hud_border_cache: Option<HashMap<u8, SpriteFrame>>,
    battle_hud_border_error: Option<String>,
    battle_exp_bar_cache: Option<HashMap<u8, SpriteFrame>>,
    battle_exp_bar_error: Option<String>,
    battle_hp_bar_cache: Option<HashMap<(u8, u8), SpriteFrame>>,
    battle_hp_bar_error: Option<String>,
    battle_party_ball_cache: Option<[SpriteFrame; 4]>,
    battle_party_ball_error: Option<String>,
    battle_send_out_poof_cache: Option<[SpriteFrame; 4]>,
    battle_send_out_poof_error: Option<String>,
    battle_anim_bundle_cache: Option<serde_json::Value>,
    battle_anim_bundle_error: Option<String>,
    battle_anim_object_cache: HashMap<String, BattleAnimRenderedFrame>,
    battle_anim_object_errors: HashMap<String, String>,
    battle_battler_overlay_cache: HashMap<(AssetId<Image>, [u8; 3]), SpriteFrame>,
    fishing_rod_cache: Option<[SpriteFrame; 3]>,
    fishing_rod_error: Option<String>,
    fishing_player_cache: HashMap<String, SpriteFrame>,
    fishing_player_errors: HashMap<String, String>,
    battle_substitute_cache: Option<[SpriteFrame; 2]>,
    battle_substitute_error: Option<String>,
    battle_minimize_cache: Option<SpriteFrame>,
    battle_minimize_error: Option<String>,
    title_cache: HashMap<TitleArtKey, SpriteFrame>,
    title_errors: HashMap<TitleArtKey, String>,
    // The title menu is redrawn as its cursor/clock changes, but its source
    // font and frame PNGs never change. Keep the decoded sources resident so
    // an animated title does not hit the filesystem and PNG decoder every
    // host frame.
    title_menu_font_source: Option<image::RgbaImage>,
    title_menu_frame_source: Option<image::RgbaImage>,
    title_screen_cache: HashMap<TitleScreenArtKey, SpriteFrame>,
    title_screen_errors: HashMap<TitleScreenArtKey, String>,
    /// Immutable credits art decoded once for the whole sequence. Credits
    /// advance at LCD cadence; reopening PNGs and palette files for every
    /// animation frame caused visible stalls even after the output texture
    /// itself became retained.
    credits_sources: Option<CreditsRenderSources>,
    credits_source_error: Option<String>,
    /// One GPU texture for every complete 160x144 title/full-screen LCD.
    /// Intro, title, new-game setup, and credits all commit into this same
    /// allocation so a screen handoff can never expose the window clear color.
    /// The historical field name is retained because tests and the standalone
    /// intro compositor inspect it directly.
    intro_presented_surface: Option<SpriteFrame>,
    /// The ECS sprite presenting `intro_presented_surface`, while a full-screen
    /// scene is active. The image allocation remains resident after the entity
    /// is removed on a successfully staged overworld frame, ready for the next
    /// title/credits sequence without mutating any cached source image.
    presented_fullscreen_entity: Option<Entity>,
    /// A cold title/full-screen -> field handoff keeps the presenter through
    /// one extraction containing both newly spawned map layers. The next
    /// update retires it only after those layers are query-visible.
    presented_fullscreen_release_pending: bool,
    intro_scene_errors: HashMap<IntroSceneArtKey, String>,
    intro_source_cache: HashMap<String, image::RgbaImage>,
    intro_palette_cache: HashMap<String, Vec<Palette>>,
    intro_sprite_bundle_cache: Option<serde_json::Value>,
    pokemon_cache: HashMap<PokemonArtKey, SpriteFrame>,
    pokemon_errors: HashMap<PokemonArtKey, String>,
    pokepic_cache: HashMap<String, SpriteFrame>,
    pokepic_errors: HashMap<String, String>,
    intro_cache: HashMap<IntroArtKey, SpriteFrame>,
    intro_errors: HashMap<IntroArtKey, String>,
    oak_intro_cache: HashMap<OakIntroArtKey, SpriteFrame>,
    oak_intro_errors: HashMap<OakIntroArtKey, String>,
    oak_intro_cache_order: VecDeque<OakIntroArtKey>,
    name_entry_cache: HashMap<NameEntryArtKey, SpriteFrame>,
    name_entry_errors: HashMap<NameEntryArtKey, String>,
    name_entry_cache_order: VecDeque<NameEntryArtKey>,
    gender_cache: HashMap<GenderArtKey, SpriteFrame>,
    gender_errors: HashMap<GenderArtKey, String>,
    time_set_cache: HashMap<TimeSetArtKey, SpriteFrame>,
    time_set_errors: HashMap<TimeSetArtKey, String>,
    time_set_cache_order: VecDeque<TimeSetArtKey>,
    trainer_card_cache: HashMap<TrainerCardArtKey, SpriteFrame>,
    trainer_card_errors: HashMap<TrainerCardArtKey, String>,
    trainer_card_cache_order: VecDeque<TrainerCardArtKey>,
    window_frame_cache: HashMap<u8, WindowFrameArt>,
    window_frame_errors: HashMap<u8, String>,
    selected_window_frame_id: u8,
    font_cache: Option<BitmapFontArt>,
    font_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TilesetArtKey {
    tileset_id: String,
    time_of_day: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpriteArtKey {
    sprite_id: String,
    palette_id: u8,
    time_of_day: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TitleArtKey {
    asset_id: String,
    palette_id: u8,
    transparent_zero: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TitleScreenArtKey {
    scx: u8,
    frame: u32,
    show_version_window: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IntroSceneArtKey {
    scene_index: usize,
    scene_frame_counter: u8,
    scene_timer: u8,
    scroll_x: u8,
    scroll_y: u8,
    global_anim_x_offset: u8,
    sprite_hash: u64,
    palette_effect: VisibleIntroPaletteEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PokemonArtKey {
    species_id: String,
    side: PokemonSpriteSide,
    shiny: bool,
    frame: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleFrontpicAnimation {
    species_id: String,
    speed: u16,
    pointer: usize,
    repeat: u16,
    wait: u16,
    frame: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleFishingPhase {
    Cast,
    Hook,
    Pause,
    AwaitText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleFishingAnimation {
    phase: VisibleFishingPhase,
    frame: u8,
    facing_up: bool,
    bite: bool,
    starts_battle: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct IntroArtKey {
    asset_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OakIntroArtKey {
    mode: VisibleOakIntroMode,
    scene_state: String,
    scene_phase: VisibleOakIntroPhase,
    current_sprite: Option<String>,
    player_gender: u8,
    current_text: String,
    visible_chars: usize,
    waiting_for_input: bool,
    blink_visible: bool,
    wipe_active: bool,
    wipe_window_x: u16,
    fade_active: bool,
    fade_alpha: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct NameEntryArtKey {
    label: String,
    value: String,
    cursor_column: usize,
    cursor_row: usize,
    case: NameInputCase,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GenderArtKey {
    selected_index: usize,
    confirmed: bool,
    fade_counter: u8,
}

#[derive(Debug, Clone, Copy)]
struct VisibleScreenFade {
    color: ScriptFadeColor,
    direction: ScriptFadeDirection,
    total_frames: u16,
    elapsed_frames: u16,
    accumulated_seconds: f32,
    alpha: u8,
    terminal_frame_presented: bool,
}

/// The Game Boy prints field dialogue progressively.  This is kept in the
/// presentation shell rather than the persistent game state: it controls how
/// an already-emitted ASM text page is revealed, not which script command has
/// executed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleFieldTextReveal {
    /// Identity of the complete script text, including page boundaries.
    text: String,
    /// `para`/`next` split a field message into pages. The source script does
    /// not advance until the player acknowledges the fully printed page.
    page_index: usize,
    visible_chars: usize,
    frames_until_next_char: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBattleTextReveal {
    text: String,
    page_index: usize,
    visible_chars: usize,
    frames_until_next_char: u8,
}

fn visible_text_frames_per_char(speed: TextSpeed) -> u8 {
    match speed {
        TextSpeed::Fast => 1,
        // `PrintLetterDelay` consumes the wOptions low bits directly:
        // TEXT_DELAY_FAST=$01, TEXT_DELAY_MED=$03, TEXT_DELAY_SLOW=$05.
        TextSpeed::Mid => 3,
        TextSpeed::Slow => 5,
    }
}

impl VisibleScreenFade {
    fn new(color: ScriptFadeColor, direction: ScriptFadeDirection, total_frames: u16) -> Self {
        Self {
            color,
            direction,
            total_frames: total_frames.max(1),
            elapsed_frames: 0,
            accumulated_seconds: 0.0,
            alpha: match direction {
                ScriptFadeDirection::Out => 0,
                ScriptFadeDirection::In => 255,
            },
            terminal_frame_presented: false,
        }
    }

    fn advance(&mut self, delta_seconds: f32) {
        self.accumulated_seconds += delta_seconds.max(0.0);
        let frames = (self.accumulated_seconds / GAME_TICK_SECONDS).floor() as u16;
        if frames == 0 {
            return;
        }
        self.accumulated_seconds -= GAME_TICK_SECONDS * f32::from(frames);
        self.elapsed_frames = self
            .elapsed_frames
            .saturating_add(frames)
            .min(self.total_frames);
        let progress = u32::from(self.elapsed_frames);
        let total = u32::from(self.total_frames);
        self.alpha = match self.direction {
            ScriptFadeDirection::Out => ((255 * progress) / total) as u8,
            ScriptFadeDirection::In => ((255 * (total - progress)) / total) as u8,
        };
        if self.elapsed_frames >= self.total_frames {
            self.alpha = match self.direction {
                ScriptFadeDirection::Out => 255,
                ScriptFadeDirection::In => 0,
            };
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TimeSetArtKey {
    phase: VisibleTimeSetPhase,
    hour: u8,
    minute: u8,
    visible_dialog: String,
    yes_no_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TrainerCardArtKey {
    page: VisibleTrainerCardPage,
    badge_frame: u8,
    player_name: String,
    player_id: u16,
    player_gender: u8,
    money: u32,
    has_pokedex: bool,
    pokedex_owned: usize,
    game_time_hours: u16,
    game_time_minutes: u8,
    colon_visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PokemonSpriteSide {
    Front,
    Back,
}

struct TilesetArt {
    metatile_layout: Vec<u8>,
    tile_handles: Vec<Handle<Image>>,
    priority_tile_handles: Vec<Handle<Image>>,
    animated_tiles: HashMap<usize, TilesetAnimatedTile>,
}

struct TilesetAnimatedTile {
    frames: Vec<Handle<Image>>,
    frame_ticks: u64,
    phase_offset: u64,
    requires_forest_restless: bool,
    cave_water_composite: bool,
    advance_on_phase_offset: bool,
    additional_schedule: Vec<(u64, u64)>,
}

#[derive(Clone)]
struct SpriteFrame {
    handle: Handle<Image>,
    size: Vec2,
}

#[derive(Clone, Copy)]
enum PresentedFullscreenFrameSource {
    /// The source frame was created solely for this presentation commit. Move
    /// its image into the retained surface (or remove it after copying).
    Transient,
    /// The source handle belongs to an art cache and must remain immutable.
    Cached,
}

const PRESENTED_FULLSCREEN_BASE_Z: f32 = 0.9;

/// Commit one complete native LCD image into the stable title/full-screen
/// texture. All supported screens are exactly 160x144 RGBA, so replacing the
/// asset value under the existing handle is an atomic whole-frame update from
/// Bevy's render extraction point of view. Cached source handles are cloned,
/// never mutated; transient frame assets are consumed immediately.
fn present_fullscreen_frame(
    rendered_art: &mut RenderedTilesetArt,
    frame: &SpriteFrame,
    source: PresentedFullscreenFrameSource,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let expected_width = (VIEWPORT_TILES_X as usize * SOURCE_TILE_SIZE) as u32;
    let expected_height = (VIEWPORT_TILES_Y as usize * SOURCE_TILE_SIZE) as u32;
    let source_id = frame.handle.id();

    if rendered_art
        .intro_presented_surface
        .as_ref()
        .is_some_and(|surface| surface.handle.id() == source_id)
    {
        return Ok(rendered_art
            .intro_presented_surface
            .as_ref()
            .expect("presented surface checked above")
            .clone());
    }

    let mut next_image = match source {
        PresentedFullscreenFrameSource::Transient => images
            .remove(source_id)
            .context("transient full-screen frame image is unavailable")?,
        PresentedFullscreenFrameSource::Cached => images
            .get(source_id)
            .context("cached full-screen frame image is unavailable")?
            .clone(),
    };
    let size = next_image.texture_descriptor.size;
    if size.width != expected_width
        || size.height != expected_height
        || size.depth_or_array_layers != 1
        || next_image.texture_descriptor.dimension != TextureDimension::D2
        || next_image.texture_descriptor.format != TextureFormat::Rgba8UnormSrgb
        || next_image.data.len() != expected_width as usize * expected_height as usize * 4
    {
        anyhow::bail!(
            "full-screen LCD frame must be {}x{} RGBA8, got {}x{} {:?} with {} bytes",
            expected_width,
            expected_height,
            size.width,
            size.height,
            next_image.texture_descriptor.format,
            next_image.data.len(),
        );
    }
    next_image.sampler = ImageSampler::nearest();

    if let Some(surface) = rendered_art.intro_presented_surface.as_ref() {
        if let Some(image) = images.get_mut(&surface.handle) {
            *image = next_image;
        } else {
            // A retained strong handle should keep this asset alive. Reinsert
            // defensively under the same id so the visible sprite never has to
            // swap handles if an external asset operation removed it.
            images.insert(surface.handle.id(), next_image);
        }
        return Ok(surface.clone());
    }

    let surface = SpriteFrame {
        handle: images.add(next_image),
        size: Vec2::new(expected_width as f32, expected_height as f32),
    };
    rendered_art.intro_presented_surface = Some(surface.clone());
    Ok(surface)
}

fn ensure_presented_fullscreen_entity(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    frame: &SpriteFrame,
    z: f32,
) {
    if let Some(entity) = rendered_art.presented_fullscreen_entity {
        // Some field-owned full-screen surfaces (notably naming) must cover
        // overworld actors, while title options must remain above the title
        // LCD. Move the one retained presenter instead of spawning a second
        // surface for either layering mode.
        commands
            .entity(entity)
            .insert(Transform::from_xyz(0.0, 0.0, z));
        return;
    }
    let entity = commands
        .spawn((
            SpriteBundle {
                texture: frame.handle.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, z),
                ..default()
            },
            TitleScreenMarker,
            VisibleIntroSurface,
        ))
        .id();
    rendered_art.presented_fullscreen_entity = Some(entity);
}

fn commit_presented_fullscreen_frame(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    frame: &SpriteFrame,
    source: PresentedFullscreenFrameSource,
    z: f32,
    images: &mut Assets<Image>,
) -> Result<SpriteFrame> {
    let frame = present_fullscreen_frame(rendered_art, frame, source, images)?;
    ensure_presented_fullscreen_entity(commands, rendered_art, &frame, z);
    rendered_art.presented_fullscreen_release_pending = false;
    Ok(frame)
}

/// Stage a solid LCD backdrop in the same retained allocation used by raster
/// full-screen scenes. The transient asset is consumed immediately, so menu
/// cursor updates neither recreate a visible ECS surface nor accumulate image
/// assets while their independently retained glyphs are rebuilt above it.
fn commit_presented_fullscreen_solid(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
    rgba: [u8; 4],
    z: f32,
    images: &mut Assets<Image>,
) -> Result<()> {
    let width = VIEWPORT_TILES_X as usize * SOURCE_TILE_SIZE;
    let height = VIEWPORT_TILES_Y as usize * SOURCE_TILE_SIZE;
    let mut data = vec![0_u8; width * height * 4];
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&rgba);
    }
    let mut image = Image::new(
        Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let frame = SpriteFrame {
        handle: images.add(image),
        size: Vec2::new(width as f32, height as f32),
    };
    commit_presented_fullscreen_frame(
        commands,
        rendered_art,
        &frame,
        PresentedFullscreenFrameSource::Transient,
        z,
        images,
    )?;
    Ok(())
}

const DYNAMIC_FULLSCREEN_FRAME_CACHE_LIMIT: usize = 16;

/// Bound caches whose keys contain live user/runtime values (typed names,
/// clock values, money, animation cursors). The shared presenter owns a copy
/// of the visible pixels, so evicting an old source frame cannot invalidate
/// the LCD currently on screen.
fn retain_bounded_fullscreen_art_key<K>(
    cache: &mut HashMap<K, SpriteFrame>,
    errors: &mut HashMap<K, String>,
    order: &mut VecDeque<K>,
    key: K,
    images: &mut Assets<Image>,
) where
    K: Clone + Eq + std::hash::Hash,
{
    order.push_back(key);
    while order.len() > DYNAMIC_FULLSCREEN_FRAME_CACHE_LIMIT {
        let Some(evicted_key) = order.pop_front() else {
            break;
        };
        if let Some(frame) = cache.remove(&evicted_key) {
            images.remove(frame.handle.id());
        }
        errors.remove(&evicted_key);
    }
}

fn remove_presented_fullscreen_entity(
    commands: &mut Commands,
    rendered_art: &mut RenderedTilesetArt,
) {
    if let Some(entity) = rendered_art.presented_fullscreen_entity.take() {
        commands.entity(entity).despawn();
    }
    rendered_art.presented_fullscreen_release_pending = false;
}

#[derive(Clone)]
struct BattleAnimRenderedFrame {
    sprite: SpriteFrame,
    offset_x: i16,
    offset_y: i16,
}

struct SpriteArt {
    down: OverworldDirectionArt,
    up: OverworldDirectionArt,
    left: OverworldDirectionArt,
    right: OverworldDirectionArt,
}

#[derive(Clone)]
struct OverworldDirectionArt {
    standing: SpriteFrame,
    walking: Option<SpriteFrame>,
}

impl OverworldDirectionArt {
    fn frame(&self, walking: bool) -> SpriteFrame {
        if walking {
            if let Some(frame) = &self.walking {
                return frame.clone();
            }
        }
        self.standing.clone()
    }
}

struct BitmapFontArt {
    glyphs: HashMap<char, SpriteFrame>,
}

struct WindowFrameArt {
    top_left: SpriteFrame,
    top_edge: SpriteFrame,
    top_right: SpriteFrame,
    side_edge: SpriteFrame,
    bottom_left: SpriteFrame,
    bottom_right: SpriteFrame,
}

struct CreditsFontTiles {
    levels: BTreeMap<u16, Vec<u8>>,
}

/// Decoded, palette-indexed credits inputs. The compositor still assembles a
/// fresh LCD byte buffer for each semantic frame, but all filesystem and PNG
/// work is paid once when the sequence first becomes visible.
struct CreditsRenderSources {
    palette_sets: Vec<[Palette; 3]>,
    mon_frames: Vec<Vec<u8>>,
    border_tiles: Vec<Vec<u8>>,
    font: CreditsFontTiles,
    copyright_tiles: Vec<Vec<u8>>,
    the_end_levels: Vec<u8>,
}

struct SlotMachineRenderSources {
    base: Vec<u8>,
    symbols: image::RgbaImage,
    palettes: Vec<Palette>,
}

struct CardFlipRenderSources {
    base: Vec<u8>,
    light_on: image::RgbaImage,
    palettes: Vec<Palette>,
}

struct UnownPuzzleRenderSources {
    pieces: HashMap<String, Vec<image::RgbaImage>>,
    cursor: image::RgbaImage,
    start_cancel: image::RgbaImage,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BattleHpSide {
    Enemy,
    Player,
}

#[derive(Component)]
struct PlayfieldTile;

#[derive(Component)]
struct PlayfieldPriorityTile;

/// Opaque copy of the last settled base viewport, retained behind the live
/// map while its exact-size 640x576 composite scrolls by one tile.
#[derive(Component)]
struct PlayfieldMapBackingBase;

/// Cropped copies of the last settled priority viewport.  Separate axes let
/// an unusual diagonal origin change expose both edges without drawing stale
/// priority pixels over the transparent parts of the new viewport.
#[derive(Component, Clone, Copy)]
enum PlayfieldMapBackingPriorityAxis {
    X,
    Y,
}

#[derive(Component)]
struct PlayerMarker;

/// The player has one retained entity; walking animation swaps its texture
/// instead of forcing `render_playfield` to rebuild the complete map.
#[derive(Component)]
struct PlayerSpriteFrames {
    standing: Handle<Image>,
    walking: Option<Handle<Image>>,
    mirror_walking: bool,
}

#[derive(Component)]
struct PlayerFacingMarker;

#[derive(Component)]
struct LedgeShadowMarker;

#[derive(Component)]
struct GrassRustleMarker;

#[derive(Component)]
struct BoulderDustMarker;

#[derive(Component)]
struct MapNameSignMarker;

#[derive(Component)]
struct ObjectMarker;

/// Stable identity for an overworld object sprite.  Keeping this separate
/// from the marker lets the renderer move an NPC in place when only its tile
/// changed, instead of despawning and recreating every visible sprite.
#[derive(Component)]
struct VisibleObjectSprite {
    /// Original visible-object index, stable even when ASM leaves the object
    /// identifier blank.  Identifiers are useful for runtime lookups, but
    /// cannot be the renderer's identity because anonymous objects are valid.
    object_index: usize,
    object_identifier: Option<String>,
    above_priority: bool,
    standing: Handle<Image>,
    walking: Option<Handle<Image>>,
    mirror_walking: bool,
    animated: bool,
}

#[derive(Component)]
struct EventMarker;

#[derive(Component)]
struct FieldPromptMarker;

#[derive(Component)]
struct FieldCommandMarker;

#[derive(Component)]
struct FieldCommandWindowFrameMarker;

#[derive(Component)]
struct SceneDialogMarker;

/// Stable identity for a bitmap glyph in the retained dialog layer.  Dialog
/// text advances one character at a time; retaining these entities avoids a
/// command-buffer despawn/spawn storm on every character.
#[derive(Component)]
struct DialogGlyphMarker {
    key: u64,
}

#[derive(Component)]
struct PokemonPictureMarker;

#[derive(Component)]
struct TitleScreenMarker;

/// The title/full-screen sequence is one continuously updated LCD surface.
/// Keeping this entity alive across intro, menus, setup, and credits prevents
/// the renderer from presenting the clear color between command-buffer flushes
/// on macOS. The historical marker name is retained for test compatibility.
#[derive(Component)]
struct VisibleIntroSurface;

#[derive(Component)]
struct ScreenFadeOverlay;

#[derive(Component)]
struct PoisonFlashOverlay;

#[derive(Component)]
struct BattleBattlerMarker;

#[derive(Component)]
struct BattleHudMarker;

#[derive(Component)]
struct BattleCommandMarker;

#[derive(Component)]
struct FixedBattleCanvasMarker;

#[derive(Component)]
struct BattleWindowFrameMarker;

#[derive(Component)]
struct SceneDialogWindowFrameMarker;

/// The field text-box paper is static across typewriter updates.  It must be
/// retained with its border; retaining only the frame tiles exposes the map
/// through the dialog after the first character advances.
#[derive(Component)]
struct SceneDialogTextBoxBackgroundMarker;

#[derive(Component)]
struct MusicAudioMarker;

#[derive(Component)]
struct TransientAudioMarker;

pub fn run_bevy_shell(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
) -> Result<()> {
    #[cfg(feature = "voxel-view")]
    let voxel_view_enabled = config.voxel_view_enabled.unwrap_or(false);
    let window_title = config
        .window_title
        .clone()
        .unwrap_or_else(|| "Pokemon Crystal Rust".to_string());
    #[cfg(feature = "location-tester")]
    let render_test_screenshot = config.render_test_screenshot.clone();
    let runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;

    let mut app = App::new();
    #[cfg(not(test))]
    app.insert_non_send_resource(NativeAudioBackend::new());
    app.insert_resource(ClearColor(Color::rgb(0.05, 0.07, 0.06)))
        .insert_resource(runtime_shell)
        .insert_resource(NativeRtcSource::system_local())
        .insert_resource(RuntimeTickTimer::new(f64::from(GAME_TICK_SECONDS)))
        .insert_resource(VisibleSequenceTickClock::realtime())
        .init_resource::<Assets<AudioSource>>()
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .insert_resource(HudMode::Status)
        // The shell uses the native rodio backend below as its sole audio
        // mixer. Bevy's AudioPlugin would initialize a second output/mixer
        // and can leave two independent streams alive during map/title
        // transitions, which is both wasteful and audible as overlapping
        // music on some macOS backends. AudioSource remains an asset cache;
        // playback is deliberately owned by the native backend.
        .add_plugins(
            DefaultPlugins
                .build()
                .disable::<AudioPlugin>()
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: window_title,
                        resolution: WindowResolution::new(640.0, 576.0),
                        // AutoVsync is allowed to select an uncapped mode on some
                        // macOS backends. FIFO is the explicit blocking/vsync path;
                        // it prevents an idle shell from burning a CPU core while
                        // the runtime still advances at the fixed Game Boy cadence.
                        present_mode: PresentMode::Fifo,
                        // The game is a fixed 160x144 LCD presentation at 4x.  A
                        // resizable host window exposes non-Game-Boy space and makes
                        // pixel scale inconsistent between screens.
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_plugins(crystal_render_api::VisualWorldRenderPlugin)
        .add_systems(Startup, setup_shell_view)
        .add_systems(Update, apply_keyboard_input)
        .add_systems(Update, apply_runtime_hotkeys.after(apply_keyboard_input))
        .add_systems(
            Update,
            drain_unused_runtime_ticks.after(apply_runtime_hotkeys),
        )
        .add_systems(
            Update,
            sync_visible_player_sprite.after(drain_unused_runtime_ticks),
        )
        .add_systems(Update, sync_visible_ledge_jump.after(render_playfield))
        .add_systems(
            Update,
            (
                sync_visible_script_jump.after(render_playfield),
                sync_visible_script_tree_shake.after(render_playfield),
                sync_visible_stationary_movement_effect.after(render_playfield),
            ),
        )
        .add_systems(
            Update,
            sync_visible_object_sprites.after(drain_unused_runtime_ticks),
        )
        .add_systems(
            Update,
            tick_visible_screen_fade
                .after(drain_unused_runtime_ticks)
                .before(render_playfield),
        )
        .add_systems(
            Update,
            sync_visible_earthquake_camera.after(drain_unused_runtime_ticks),
        )
        .add_systems(
            Update,
            drain_runtime_audio_events.after(apply_runtime_hotkeys),
        )
        .add_systems(
            Update,
            tick_visible_title_screen.after(drain_runtime_audio_events),
        )
        .add_systems(
            Update,
            sync_runtime_title_music.after(tick_visible_title_screen),
        )
        .add_systems(
            Update,
            sync_runtime_battle_music.after(sync_runtime_title_music),
        )
        .add_systems(
            Update,
            sync_runtime_current_music.after(sync_runtime_battle_music),
        )
        .add_systems(
            Update,
            queue_battle_intro_cry.after(sync_runtime_current_music),
        )
        .add_systems(Update, play_pending_audio.after(queue_battle_intro_cry))
        .add_systems(
            Update,
            render_playfield
                .after(play_pending_audio)
                .in_set(crystal_render_api::WorldRenderSet::ClassicWorld),
        )
        .add_systems(
            Update,
            apply_visible_battle_screen_offset.after(render_playfield),
        )
        .add_systems(Update, render_screen_fade_overlay.after(render_playfield))
        .add_systems(Update, render_poison_flash_overlay.after(render_playfield))
        .add_systems(Update, refresh_status_text.after(render_playfield))
        .add_systems(Update, refresh_dialog_text.after(refresh_status_text))
        .add_systems(Update, refresh_battle_text.after(refresh_dialog_text))
        .add_systems(Update, refresh_shell_panels.after(refresh_battle_text))
        .add_systems(
            Update,
            publish_visual_world_frame
                .after(sync_visible_player_sprite)
                .after(sync_visible_object_sprites)
                .after(sync_visible_ledge_jump)
                .after(sync_visible_script_jump)
                .after(sync_visible_script_tree_shake)
                .after(sync_visible_stationary_movement_effect)
                .after(apply_visible_battle_screen_offset)
                .in_set(crystal_render_api::WorldRenderSet::PresentationExtract),
        );
    #[cfg(feature = "voxel-view")]
    app.insert_resource(crystal_voxel_view::VoxelViewSettings {
        enabled: voxel_view_enabled,
        allow_f3_toggle: true,
    })
    .add_plugins(crystal_voxel_view::VoxelViewPlugin)
    .add_systems(
        Startup,
        configure_voxel_composite_camera.after(setup_shell_view),
    )
    .add_systems(
        Update,
        sync_voxel_classic_world_layers.after(crystal_render_api::WorldRenderSet::RenderSync),
    );
    #[cfg(feature = "location-tester")]
    if let Some(path) = render_test_screenshot.clone() {
        app.insert_resource(RenderTestScreenshot {
            path,
            frame: 0,
            requested: false,
            requested_at: None,
        })
        .add_systems(Update, capture_render_test_screenshot);
    }
    app.run();

    #[cfg(feature = "location-tester")]
    if let Some(path) = render_test_screenshot.as_deref() {
        validate_render_test_screenshot(path)?;
    }

    Ok(())
}

#[cfg(feature = "voxel-view")]
fn configure_voxel_composite_camera(
    mut commands: Commands,
    mut cameras: Query<(Entity, &mut Camera), With<MainCameraMarker>>,
) {
    for (entity, mut camera) in &mut cameras {
        camera.order = 1;
        camera.clear_color = bevy::render::camera::ClearColorConfig::None;
        commands
            .entity(entity)
            .insert(bevy::render::view::RenderLayers::layer(0));
    }
}

#[cfg(feature = "voxel-view")]
fn sync_voxel_classic_world_layers(
    status: Res<crystal_voxel_view::VoxelViewStatus>,
    mut commands: Commands,
    classic_world: Query<
        Entity,
        Or<(
            With<PlayfieldTile>,
            With<PlayfieldMapBackingBase>,
            With<PlayfieldMapBackingPriorityAxis>,
            With<PlayerMarker>,
            With<ObjectMarker>,
            With<LedgeShadowMarker>,
        )>,
    >,
) {
    for entity in &classic_world {
        if status.active {
            // The voxel plugin's first camera draws this faithful world as a
            // coverage layer. Its 3D camera then overlays authored geometry,
            // and the layer-0 camera composites unchanged UI and fades.
            commands.entity(entity).insert(
                bevy::render::view::RenderLayers::layer(
                    crystal_voxel_view::CLASSIC_FALLBACK_RENDER_LAYER,
                ),
            );
        } else {
            commands
                .entity(entity)
                .remove::<bevy::render::view::RenderLayers>();
        }
    }
}

#[cfg(feature = "location-tester")]
#[derive(Resource)]
struct RenderTestScreenshot {
    path: PathBuf,
    frame: u32,
    requested: bool,
    requested_at: Option<u32>,
}

#[cfg(feature = "location-tester")]
fn capture_render_test_screenshot(
    mut capture: ResMut<RenderTestScreenshot>,
    voxel_status: Res<crystal_voxel_view::VoxelViewStatus>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut screenshots: ResMut<ScreenshotManager>,
    mut exit: EventWriter<AppExit>,
) {
    capture.frame = capture.frame.saturating_add(1);
    // Give the retained classic renderer, extracted visual frame, voxel mesh,
    // and GPU render target several presented frames to settle. Capturing the
    // first frame in which status flips active can still read the preceding
    // classic frame from the window swapchain.
    let presentation_settled = voxel_status.active_frames >= 30
        || voxel_status.fallback_reason.as_deref() == Some("disabled");
    if !capture.requested && capture.frame >= 90 && presentation_settled {
        println!(
            "2.5D renderer status: {}{}",
            if voxel_status.active {
                "active"
            } else {
                "inactive"
            },
            voxel_status
                .fallback_reason
                .as_deref()
                .map(|reason| format!(" ({reason})"))
                .unwrap_or_default()
        );
        let Ok(window) = primary_window.get_single() else {
            return;
        };
        if screenshots
            .save_screenshot_to_disk(window, &capture.path)
            .is_ok()
        {
            capture.requested = true;
            capture.requested_at = Some(capture.frame);
        }
    }
    if capture
        .requested_at
        .is_some_and(|requested_at| capture.frame >= requested_at.saturating_add(60))
    {
        exit.send(AppExit::Success);
    }
}

#[cfg(feature = "location-tester")]
fn validate_render_test_screenshot(path: &Path) -> Result<()> {
    let screenshot = image::open(path)
        .with_context(|| format!("read Bevy render-test screenshot {}", path.display()))?
        .into_rgba8();
    let (width, height) = screenshot.dimensions();
    anyhow::ensure!(
        width >= 640 && height >= 576,
        "Bevy render-test screenshot {} is only {width}x{height}",
        path.display()
    );

    let mut colors = std::collections::BTreeSet::new();
    let mut min_luma = u32::MAX;
    let mut max_luma = 0_u32;
    for pixel in screenshot.pixels() {
        let [red, green, blue, alpha] = pixel.0;
        if alpha == 0 {
            continue;
        }
        colors.insert([red, green, blue]);
        let luma = 299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue);
        min_luma = min_luma.min(luma);
        max_luma = max_luma.max(luma);
    }
    anyhow::ensure!(
        colors.len() >= 4 && max_luma.saturating_sub(min_luma) >= 32_000,
        "Bevy render-test screenshot {} is a blank/uniform frame ({} colors, luma range {})",
        path.display(),
        colors.len(),
        max_luma.saturating_sub(min_luma) as f32 / 1000.0
    );
    println!(
        "verified visible Bevy screenshot: {} ({width}x{height}, {} colors, luma range {:.1})",
        path.display(),
        colors.len(),
        max_luma.saturating_sub(min_luma) as f32 / 1000.0
    );
    Ok(())
}

pub fn smoke_visible_shell_start_menu(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
    party: &[VisibleShellSmokePokemon],
    bag_items: &[VisibleShellSmokeItem],
) -> Result<VisibleShellStartMenuSmoke> {
    let smoke_player_name = config.smoke_player_name.clone();
    let mut runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, smoke_player_name.as_deref())?;
    for (index, pokemon) in party.iter().enumerate() {
        runtime_shell.shell.add_party_pokemon(
            &pokemon.species_id,
            pokemon.level,
            pokemon.held_item_id.clone(),
            None,
            "BEVY_SMOKE",
            u16::try_from(index + 1).context("visible shell smoke party index overflow")?,
            Dv::from_non_hp(10, 10, 10, 10),
        )?;
    }
    for item in bag_items {
        runtime_shell
            .shell
            .add_bag_item(&item.item_id, item.quantity)?;
    }
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    let initial = runtime_shell.shell.snapshot()?;
    press_visible_start_button(&mut runtime_shell)?;
    let start_menu_entries = visible_start_menu_entries(&runtime_shell)?;
    select_visible_start_menu_option_exact(&mut runtime_shell, StartMenuOption::Pokemon)?;
    select_visible_start_menu_option(&mut runtime_shell)?;
    let party_snapshot = runtime_shell.shell.snapshot()?;
    let party_entries = visible_party_menu_entries(&party_snapshot, &runtime_shell);
    close_visible_party_menu(&mut runtime_shell);
    press_visible_start_button(&mut runtime_shell)?;
    select_visible_start_menu_option_exact(&mut runtime_shell, StartMenuOption::Pack)?;
    select_visible_start_menu_option(&mut runtime_shell)?;
    let pack_snapshot = runtime_shell.shell.snapshot()?;
    let pack_entries = visible_field_pack_entries(&pack_snapshot, &runtime_shell);
    close_visible_field_pack_without_log(&mut runtime_shell);
    press_visible_start_button(&mut runtime_shell)?;
    select_visible_start_menu_option_exact(&mut runtime_shell, StartMenuOption::TrainerCard)?;
    select_visible_start_menu_option(&mut runtime_shell)?;
    let trainer_snapshot = runtime_shell.shell.snapshot()?;
    let trainer_entries = visible_trainer_card_entries(&trainer_snapshot, &runtime_shell);
    close_visible_trainer_card(&mut runtime_shell);
    press_visible_start_button(&mut runtime_shell)?;
    select_visible_start_menu_option_exact(&mut runtime_shell, StartMenuOption::Save)?;
    select_visible_start_menu_option(&mut runtime_shell)?;
    let save_snapshot = runtime_shell.shell.snapshot()?;
    let save_entries = visible_scene_dialog_entries(&save_snapshot, &runtime_shell)?;
    confirm_visible_save_menu(&mut runtime_shell)?;
    let save_path = runtime_shell
        .quick_save_path
        .clone()
        .context("visible shell smoke missing quick-save path")?;
    let summary = runtime_shell
        .shell
        .runtime()
        .load_save_summary(&save_path)?;
    Ok(VisibleShellStartMenuSmoke {
        initial_map: initial.overworld.map_name,
        initial_tile_x: initial.overworld.tile.x,
        initial_tile_y: initial.overworld.tile.y,
        start_menu_entries,
        party_entries,
        pack_entries,
        trainer_entries,
        save_entries,
        saved_frame: summary.saved_frame(),
        save_path,
    })
}

pub fn smoke_visible_shell_title(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    save_path: Option<PathBuf>,
    continue_save: bool,
) -> Result<VisibleShellTitleSmoke> {
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: save_path.clone(),
        },
        BevyShellConfig {
            quick_save_path: save_path.clone(),
            ..Default::default()
        },
    )?;
    if runtime_shell.intro_screen.is_some() {
        skip_visible_intro_screen(&mut runtime_shell, GameButton::Start)?;
    }
    advance_visible_title_to_main_menu(&mut runtime_shell)?;
    let title = runtime_shell
        .title_menu
        .clone()
        .context("visible shell title smoke did not open title menu")?;
    if continue_save {
        let options = visible_title_menu_options(&runtime_shell, &title);
        let Some(index) = options
            .iter()
            .position(|option| matches!(option, TitleMenuOption::Continue))
        else {
            anyhow::bail!("visible shell title Continue requested without a configured save path");
        };
        if let Some(title) = runtime_shell.title_menu.as_mut() {
            title.cursor.option_index = index;
        }
    } else {
        let options = visible_title_menu_options(&runtime_shell, &title);
        let Some(index) = options
            .iter()
            .position(|option| matches!(option, TitleMenuOption::NewGame))
        else {
            anyhow::bail!("visible shell title New Game option missing");
        };
        if let Some(title) = runtime_shell.title_menu.as_mut() {
            title.cursor.option_index = index;
        }
    }
    let title = runtime_shell
        .title_menu
        .clone()
        .context("visible shell title menu closed before capture")?;
    let title_entries = visible_title_menu_entries(&runtime_shell, &title)?;
    let selected = runtime_shell
        .title_menu
        .as_ref()
        .map(|title| {
            selected_visible_title_menu_option(&runtime_shell, title).map(|option| match option {
                TitleMenuOption::Continue => "CONTINUE".to_string(),
                TitleMenuOption::NewGame => "NEW_GAME".to_string(),
                TitleMenuOption::Options => "OPTIONS".to_string(),
                TitleMenuOption::MysteryGift => "MYSTERY_GIFT".to_string(),
            })
        })
        .context("visible shell title menu closed before selection")??;
    select_visible_title_menu_option(&mut runtime_shell)?;
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let new_game_identity_pending = runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.pending_name_input.is_some();
    if !continue_save && !new_game_identity_pending {
        if let Some(path) = save_path.as_ref() {
            runtime_shell.shell.save(path)?;
        }
    }
    let should_read_save_summary = continue_save || !new_game_identity_pending;
    let saved_frame = save_path
        .as_ref()
        .filter(|_| should_read_save_summary)
        .map(|path| {
            runtime_shell
                .shell
                .runtime()
                .load_save_summary(path)
                .map(|summary| summary.saved_frame())
        })
        .transpose()?;
    Ok(VisibleShellTitleSmoke {
        title_entries,
        selected,
        map: snapshot.overworld.map_name,
        tile_x: snapshot.overworld.tile.x,
        tile_y: snapshot.overworld.tile.y,
        state_hash: snapshot.state_checksum,
        saved_frame,
        save_path,
    })
}

pub fn smoke_visible_shell_title_name_input(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    spawn_identifier: u16,
    save_path: Option<PathBuf>,
    player_name: &str,
) -> Result<VisibleShellTitleNameInputSmoke> {
    let mut runtime_shell = initialize_bevy_runtime_shell(
        asset_root,
        runtime,
        BevyShellStart::Title {
            spawn_identifier,
            save_path: save_path.clone(),
        },
        BevyShellConfig {
            quick_save_path: save_path.clone(),
            ..Default::default()
        },
    )?;
    if runtime_shell.intro_screen.is_some() {
        skip_visible_intro_screen(&mut runtime_shell, GameButton::Start)?;
    }
    advance_visible_title_to_main_menu(&mut runtime_shell)?;
    let title = runtime_shell
        .title_menu
        .clone()
        .context("visible title name-input smoke did not open title menu")?;
    let options = visible_title_menu_options(&runtime_shell, &title);
    let Some(index) = options
        .iter()
        .position(|option| matches!(option, TitleMenuOption::NewGame))
    else {
        anyhow::bail!("visible title New Game option missing");
    };
    if let Some(title) = runtime_shell.title_menu.as_mut() {
        title.cursor.option_index = index;
    }
    let title = runtime_shell
        .title_menu
        .clone()
        .context("visible title menu closed before capture")?;
    let title_entries = visible_title_menu_entries(&runtime_shell, &title)?;
    let selected =
        selected_visible_title_menu_option(&runtime_shell, &title).map(|option| match option {
            TitleMenuOption::Continue => "CONTINUE".to_string(),
            TitleMenuOption::NewGame => "NEW_GAME".to_string(),
            TitleMenuOption::Options => "OPTIONS".to_string(),
            TitleMenuOption::MysteryGift => "MYSTERY_GIFT".to_string(),
        })?;
    select_visible_title_menu_option(&mut runtime_shell)?;
    complete_visible_smoke_gender_if_needed(&mut runtime_shell)?;
    complete_visible_smoke_time_set_if_needed(&mut runtime_shell)?;
    complete_visible_smoke_oak_intro_if_needed(&mut runtime_shell)?;
    if runtime_shell.pending_name_choice.is_some() {
        confirm_visible_name_choice(&mut runtime_shell)?;
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let initial_name_entries = visible_scene_dialog_entries(&snapshot, &runtime_shell)?;
    for ch in player_name.chars() {
        apply_visible_name_input_smoke_char(&mut runtime_shell, ch)?;
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let typed_name_entries = visible_scene_dialog_entries(&snapshot, &runtime_shell)?;
    apply_visible_name_input_smoke_key(&mut runtime_shell, KeyCode::Enter);
    apply_visible_name_input_smoke_key(&mut runtime_shell, KeyCode::KeyZ);
    complete_visible_smoke_oak_intro_if_needed(&mut runtime_shell)?;
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(path) = save_path.as_ref() {
        runtime_shell.shell.save(path)?;
    }
    let saved_frame = save_path
        .as_ref()
        .map(|path| {
            runtime_shell
                .shell
                .runtime()
                .load_save_summary(path)
                .map(|summary| summary.saved_frame())
        })
        .transpose()?;
    Ok(VisibleShellTitleNameInputSmoke {
        title_entries,
        initial_name_entries,
        typed_name_entries,
        selected,
        trainer_name: snapshot.trainer.player_name,
        map: snapshot.overworld.map_name,
        tile_x: snapshot.overworld.tile.x,
        tile_y: snapshot.overworld.tile.y,
        state_hash: snapshot.state_checksum,
        saved_frame,
        save_path,
    })
}

pub fn smoke_visible_shell_party(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
    party: &[VisibleShellSmokePokemon],
) -> Result<VisibleShellPartySmoke> {
    if party.len() < 2 {
        anyhow::bail!("visible shell party smoke requires at least two Pokemon");
    }
    let smoke_player_name = config.smoke_player_name.clone();
    let mut runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, smoke_player_name.as_deref())?;
    for (index, pokemon) in party.iter().enumerate() {
        runtime_shell.shell.add_party_pokemon(
            &pokemon.species_id,
            pokemon.level,
            pokemon.held_item_id.clone(),
            None,
            "BEVY_PARTY_SMOKE",
            u16::try_from(index + 1).context("visible shell party smoke party index overflow")?,
            Dv::from_non_hp(10, 10, 10, 10),
        )?;
    }
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    open_visible_party_menu(&mut runtime_shell)?;
    let initial_snapshot = runtime_shell.shell.snapshot()?;
    let lead_before = initial_snapshot
        .party
        .slots
        .first()
        .map(|slot| slot.pokemon.species.id.clone())
        .context("visible shell party smoke missing lead Pokemon")?;
    let initial_entries = visible_party_menu_entries(&initial_snapshot, &runtime_shell);

    open_visible_party_action_menu(&mut runtime_shell)?;
    let action_snapshot = runtime_shell.shell.snapshot()?;
    let action_entries = visible_party_menu_entries(&action_snapshot, &runtime_shell);
    execute_visible_party_action(&mut runtime_shell)?;
    let summary_snapshot = runtime_shell.shell.snapshot()?;
    let summary_entries = visible_party_menu_entries(&summary_snapshot, &runtime_shell);
    close_visible_party_summary(&mut runtime_shell);

    open_visible_party_action_menu(&mut runtime_shell)?;
    move_visible_party_action_cursor(&mut runtime_shell, 1)?;
    execute_visible_party_action(&mut runtime_shell)?;
    let switch_snapshot = runtime_shell.shell.snapshot()?;
    let switch_entries = visible_party_menu_entries(&switch_snapshot, &runtime_shell);
    confirm_visible_party_switch_target(&mut runtime_shell)?;
    let final_snapshot = runtime_shell.shell.snapshot()?;
    let lead_after = final_snapshot
        .party
        .slots
        .first()
        .map(|slot| slot.pokemon.species.id.clone())
        .context("visible shell party smoke missing final lead Pokemon")?;
    let final_entries = visible_party_menu_entries(&final_snapshot, &runtime_shell);
    Ok(VisibleShellPartySmoke {
        initial_entries,
        action_entries,
        summary_entries,
        switch_entries,
        final_entries,
        lead_before,
        lead_after,
        state_hash: final_snapshot.state_checksum,
    })
}

pub fn smoke_visible_shell_wild_battle(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
    battle_ref: VisibleShellBattleSmokeRef,
    party: &[VisibleShellSmokePokemon],
    bag_items: &[VisibleShellSmokeItem],
) -> Result<VisibleShellBattleSmoke> {
    if party.is_empty() {
        anyhow::bail!("visible shell battle smoke requires at least one Pokemon");
    }
    let smoke_player_name = config.smoke_player_name.clone();
    let mut runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, smoke_player_name.as_deref())?;
    for (index, pokemon) in party.iter().enumerate() {
        runtime_shell.shell.add_party_pokemon(
            &pokemon.species_id,
            pokemon.level,
            pokemon.held_item_id.clone(),
            None,
            "BEVY_BATTLE_SMOKE",
            u16::try_from(index + 1).context("visible shell battle smoke party index overflow")?,
            Dv::from_non_hp(10, 10, 10, 10),
        )?;
    }
    for item in bag_items {
        runtime_shell
            .shell
            .add_bag_item(&item.item_id, item.quantity)?;
    }
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    let start = runtime_shell.shell.start_scripted_wild_battle(
        &battle_ref.map_name,
        &battle_ref.source_script,
        battle_ref.command_index,
    )?;
    prepare_visible_battle_entry(&mut runtime_shell)?;
    sync_visible_battle_action_cursor(&mut runtime_shell);
    let action_snapshot = runtime_shell.shell.snapshot()?;
    let action_battle = action_snapshot
        .battle
        .as_ref()
        .context("visible shell battle smoke did not start a battle")?;
    let action_entries =
        visible_battle_command_menu_entries(&action_snapshot, &runtime_shell, action_battle)?;

    let mut switch_entries = Vec::new();
    if !action_battle.commands.switch_party_indices.is_empty() {
        let actions = visible_battle_action_ids(&action_snapshot, action_battle);
        if let Some(index) = actions
            .iter()
            .position(|action| *action == VisibleBattleAction::Pokemon)
        {
            runtime_shell.battle_action_cursor = Some(MenuCursor {
                surface_id: "battle:actions".to_string(),
                option_index: index,
            });
            press_visible_battle_a_button(&mut runtime_shell)?;
            let switch_snapshot = runtime_shell.shell.snapshot()?;
            if let Some(battle) = switch_snapshot.battle.as_ref() {
                switch_entries =
                    visible_battle_command_menu_entries(&switch_snapshot, &runtime_shell, battle)?;
            }
            press_visible_battle_b_button(&mut runtime_shell)?;
        }
    }

    let mut pack_entries = Vec::new();
    let mut ball_entries = Vec::new();
    let pack_snapshot = runtime_shell.shell.snapshot()?;
    if let Some(battle) = pack_snapshot.battle.as_ref() {
        let actions = visible_battle_action_ids(&pack_snapshot, battle);
        if let Some(index) = actions
            .iter()
            .position(|action| *action == VisibleBattleAction::Pack)
        {
            runtime_shell.battle_action_cursor = Some(MenuCursor {
                surface_id: "battle:actions".to_string(),
                option_index: index,
            });
            press_visible_battle_a_button(&mut runtime_shell)?;
            let item_snapshot = runtime_shell.shell.snapshot()?;
            if let Some(battle) = item_snapshot.battle.as_ref() {
                pack_entries =
                    visible_battle_command_menu_entries(&item_snapshot, &runtime_shell, battle)?;
                if !carried_ball_item_ids(&item_snapshot).is_empty() {
                    ball_entries = pack_entries.clone();
                }
            }
            press_visible_battle_b_button(&mut runtime_shell)?;
        }
    }

    select_visible_battle_action(&mut runtime_shell, VisibleBattleAction::Fight)?;
    press_visible_battle_a_button(&mut runtime_shell)?;
    let move_snapshot = runtime_shell.shell.snapshot()?;
    let move_battle = move_snapshot
        .battle
        .as_ref()
        .context("visible shell battle smoke lost battle before move selection")?;
    let move_entries =
        visible_battle_command_menu_entries(&move_snapshot, &runtime_shell, move_battle)?;
    press_visible_battle_a_button(&mut runtime_shell)?;
    finish_visible_wild_battle_with_first_move(&mut runtime_shell)?;
    let final_snapshot = runtime_shell.shell.snapshot()?;
    let after_entries = final_snapshot
        .battle
        .as_ref()
        .map(|battle| visible_battle_command_menu_entries(&final_snapshot, &runtime_shell, battle))
        .transpose()?
        .unwrap_or_default();
    Ok(VisibleShellBattleSmoke {
        wild_species: start.species,
        wild_level: start.level,
        action_entries,
        switch_entries,
        pack_entries,
        ball_entries,
        move_entries,
        after_entries,
        active_battle_after: final_snapshot.battle.is_some(),
        state_hash: final_snapshot.state_checksum,
    })
}

pub fn smoke_visible_shell_trainer_battle(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
    battle_ref: VisibleShellBattleSmokeRef,
    party: &[VisibleShellSmokePokemon],
) -> Result<VisibleShellTrainerBattleSmoke> {
    if party.is_empty() {
        anyhow::bail!("visible shell trainer battle smoke requires at least one Pokemon");
    }
    let smoke_player_name = config.smoke_player_name.clone();
    let mut runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, smoke_player_name.as_deref())?;
    for (index, pokemon) in party.iter().enumerate() {
        runtime_shell.shell.add_party_pokemon(
            &pokemon.species_id,
            pokemon.level,
            pokemon.held_item_id.clone(),
            None,
            "BEVY_TRAINER_BATTLE_SMOKE",
            u16::try_from(index + 1)
                .context("visible shell trainer battle smoke party index overflow")?,
            Dv::from_non_hp(10, 10, 10, 10),
        )?;
    }
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    runtime_shell.shell.start_scripted_trainer_battle(
        &battle_ref.map_name,
        &battle_ref.source_script,
        battle_ref.command_index,
    )?;
    prepare_visible_battle_entry(&mut runtime_shell)?;
    sync_visible_battle_action_cursor(&mut runtime_shell);
    let initial_snapshot = runtime_shell.shell.snapshot()?;
    let initial_battle = initial_snapshot
        .battle
        .as_ref()
        .context("visible shell trainer battle smoke did not start a battle")?;
    let (trainer_class, trainer_id, trainer_name) = match &initial_battle.kind {
        RuntimeBattleKind::Trainer {
            trainer_class,
            trainer_id,
            trainer_name,
            ..
        } => (
            trainer_class.clone(),
            trainer_id.clone(),
            trainer_name.clone(),
        ),
        _ => anyhow::bail!("visible shell trainer battle smoke did not start a trainer battle"),
    };
    let initial_entries =
        visible_battle_command_menu_entries(&initial_snapshot, &runtime_shell, initial_battle)?;
    let mut first_move_entries = Vec::new();
    let mut shift_prompt_entries = Vec::new();
    let mut shift_prompt_count = 0usize;
    let mut kept_current_after_shift_prompt = false;
    let switched_after_shift_prompt = false;
    let mut turns = 0usize;
    let mut interaction_steps = 0usize;
    const MAX_VISIBLE_TRAINER_BATTLE_TURNS: usize = 128;
    while turns < MAX_VISIBLE_TRAINER_BATTLE_TURNS {
        interaction_steps += 1;
        if interaction_steps > MAX_VISIBLE_TRAINER_BATTLE_TURNS * 4 {
            anyhow::bail!(
                "visible shell trainer battle smoke exceeded {} menu interactions before completing {turns} turns",
                MAX_VISIBLE_TRAINER_BATTLE_TURNS * 4
            );
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        if snapshot.battle.is_none() {
            break;
        }
        if let Some(battle) = snapshot.battle.as_ref() {
            if runtime_shell.battle_shift_prompt_cursor.is_some()
                && trainer_shift_switch_pending(&snapshot, battle)
            {
                shift_prompt_count += 1;
                if shift_prompt_entries.is_empty() {
                    shift_prompt_entries =
                        visible_battle_command_menu_entries(&snapshot, &runtime_shell, battle)?;
                }
                runtime_shell.battle_shift_prompt_cursor = Some(MenuCursor {
                    surface_id: "battle:shift-prompt".to_string(),
                    option_index: 1,
                });
                press_visible_battle_a_button(&mut runtime_shell)?;
                kept_current_after_shift_prompt = true;
                continue;
            }
        }
        if snapshot
            .battle
            .as_ref()
            .is_some_and(|battle| battle.enemy_pokemon.hp == 0)
            && !visible_active_battle_player_fainted(&snapshot)
        {
            press_visible_battle_a_button(&mut runtime_shell)?;
            continue;
        }
        if visible_active_battle_player_fainted(&snapshot) {
            // Exercise the normal ASM replacement flow rather than assuming
            // a smoke lead can solo every member of a full trainer party.
            press_visible_battle_a_button(&mut runtime_shell)?;
            if runtime_shell.battle_switch_cursor.is_some() {
                let replacement_snapshot = runtime_shell.shell.snapshot()?;
                let active_party_index = replacement_snapshot
                    .battle
                    .as_ref()
                    .and_then(|battle| battle.active_player_party_index);
                let replacement_index = replacement_snapshot
                    .party
                    .slots
                    .iter()
                    .position(|slot| slot.pokemon.hp > 0 && Some(slot.index) != active_party_index)
                    .context("fainted battle smoke has no healthy replacement")?;
                runtime_shell.battle_switch_cursor = Some(MenuCursor {
                    surface_id: "battle:switch".to_string(),
                    option_index: replacement_index,
                });
                press_visible_battle_a_button(&mut runtime_shell)?;
            }
            continue;
        }
        select_visible_battle_action(&mut runtime_shell, VisibleBattleAction::Fight)?;
        press_visible_battle_a_button(&mut runtime_shell)?;
        let move_snapshot = runtime_shell.shell.snapshot()?;
        let move_battle = move_snapshot
            .battle
            .as_ref()
            .context("visible shell trainer battle ended before move selection")?;
        if first_move_entries.is_empty() {
            first_move_entries =
                visible_battle_command_menu_entries(&move_snapshot, &runtime_shell, move_battle)?;
        }
        if let Some(move_index) = move_snapshot
            .party
            .slots
            .iter()
            .find(|slot| slot.is_active_battle_pokemon)
            .and_then(|slot| {
                let healing_move = (u32::from(slot.pokemon.hp) * 3
                    < u32::from(slot.pokemon.max_hp))
                .then(|| {
                    slot.pokemon
                        .moves
                        .iter()
                        .enumerate()
                        .find(|(_, learned)| {
                            learned.current_pp > 0
                                && move_snapshot.moves.iter().any(|known| {
                                    known.move_id == learned.name
                                        && matches!(
                                            known.effect.as_str(),
                                            "HEAL" | "MOONLIGHT" | "MORNING_SUN" | "SYNTHESIS"
                                        )
                                })
                        })
                        .map(|(index, _)| index)
                })
                .flatten();
                if healing_move.is_some() {
                    return healing_move;
                }
                slot.pokemon
                    .moves
                    .iter()
                    .enumerate()
                    .filter(|(_, learned)| learned.current_pp > 0)
                    .max_by_key(|(_, learned)| {
                        move_snapshot
                            .moves
                            .iter()
                            .find(|known| known.move_id == learned.name)
                            // A repeated Future Sight setup is legal but does
                            // not exercise ordinary visible damage progress.
                            // Prefer an immediately resolving damaging move
                            // for this end-to-end shell smoke.
                            .map_or(0, |known| {
                                if known.effect == "FUTURE_SIGHT" {
                                    0
                                } else {
                                    known.power
                                }
                            })
                    })
                    .map(|(index, _)| index)
            })
        {
            runtime_shell.battle_move_cursor = Some(MenuCursor {
                surface_id: "battle:moves".to_string(),
                option_index: move_index,
            });
        }
        press_visible_battle_a_button(&mut runtime_shell)?;
        turns += 1;
        if runtime_shell.shell.snapshot()?.battle.is_none() {
            break;
        }
    }
    let final_snapshot = runtime_shell.shell.snapshot()?;
    let active_battle_after = final_snapshot.battle.is_some();
    let trainer_defeated = !active_battle_after;
    if !trainer_defeated {
        anyhow::bail!(
            "visible shell trainer battle smoke did not defeat trainer after {MAX_VISIBLE_TRAINER_BATTLE_TURNS} turns: battle={:?} active_player={:?} action_cursor={:?} move_cursor={:?} switch_cursor={:?} queued_messages={} status={:?}",
            final_snapshot.battle.as_ref().map(|battle| (
                battle.active_player_party_index,
                battle.active_enemy_party_index,
                battle.enemy_pokemon.hp,
                battle.rewarded_enemy_party_indices.clone(),
            )),
            final_snapshot
                .party
                .slots
                .iter()
                .find(|slot| slot.is_active_battle_pokemon)
                .map(|slot| (slot.pokemon.hp, slot.pokemon.moves.clone())),
            runtime_shell.battle_action_cursor,
            runtime_shell.battle_move_cursor,
            runtime_shell.battle_switch_cursor,
            runtime_shell.battle_messages.len(),
            runtime_shell.last_action_status,
        );
    }
    let final_entries = final_snapshot
        .battle
        .as_ref()
        .map(|battle| visible_battle_command_menu_entries(&final_snapshot, &runtime_shell, battle))
        .transpose()?
        .unwrap_or_default();
    Ok(VisibleShellTrainerBattleSmoke {
        trainer_class,
        trainer_id,
        trainer_name,
        initial_entries,
        first_move_entries,
        shift_prompt_entries,
        shift_prompt_count,
        kept_current_after_shift_prompt,
        switched_after_shift_prompt,
        turns,
        trainer_defeated,
        final_entries,
        active_battle_after,
        state_hash: final_snapshot.state_checksum,
    })
}

pub fn smoke_visible_shell_overworld(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
    input_frames: &[Vec<GameButton>],
    save_path: Option<&PathBuf>,
) -> Result<VisibleShellOverworldSmoke> {
    if input_frames.is_empty() {
        anyhow::bail!("visible shell overworld smoke requires at least one input frame");
    }
    let smoke_player_name = config.smoke_player_name.clone();
    let mut runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;
    complete_visible_smoke_player_name_if_needed(&mut runtime_shell, smoke_player_name.as_deref())?;
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    let start_snapshot = runtime_shell.shell.snapshot()?;
    let start_map = start_snapshot.overworld.map_name.clone();
    let start_tile_x = start_snapshot.overworld.tile.x;
    let start_tile_y = start_snapshot.overworld.tile.y;
    let start_scene = current_visible_scene_label(&runtime_shell)?;
    let mut interactions = 0usize;
    let mut coord_events = 0usize;
    let mut trainer_sight_events = 0usize;
    let mut warps = 0usize;
    let mut connections = 0usize;
    let mut wild_battles = 0usize;
    let mut last_movement = None;
    let mut frame_events = Vec::new();

    for (index, buttons) in input_frames.iter().enumerate() {
        let outcome = apply_visible_shell_smoke_frame(&mut runtime_shell, buttons)
            .with_context(|| format!("advance visible overworld input frame {}", index + 1))?;
        if outcome.interaction {
            interactions += 1;
        }
        if let Some(frame) = outcome.frame {
            if let Some(movement) = frame.movement.as_ref() {
                last_movement = Some(format!("{movement:?}"));
            }
            let movement = frame
                .movement
                .as_ref()
                .map(|movement| format!("{movement:?}"))
                .unwrap_or_else(|| "none".to_string());
            frame_events.push(format!(
                "{}:{:?}:{}@({},{}):movement={}:interaction={}:coord={}:trainer_sight={}:warp={}:connection={}:wild={}",
                index + 1,
                buttons,
                frame.snapshot.map_name,
                frame.snapshot.tile.x,
                frame.snapshot.tile.y,
                movement,
                frame.interaction.is_some(),
                frame.coord_event.is_some(),
                frame.trainer_sight.is_some(),
                frame.warp.is_some(),
                frame.connection.is_some(),
                frame.wild_battle.is_some()
            ));
            if frame.interaction.is_some() {
                interactions += 1;
                execute_last_interaction_script(&mut runtime_shell)?;
            }
            if frame.coord_event.is_some() {
                coord_events += 1;
                execute_last_coord_event_script(&mut runtime_shell)?;
            }
            if frame.trainer_sight.is_some() {
                trainer_sight_events += 1;
                execute_last_trainer_sight_script(&mut runtime_shell)?;
            }
            if frame.warp.is_some() {
                warps += 1;
                settle_visible_overworld_frame_arrival(&mut runtime_shell)?;
            }
            if frame.connection.is_some() {
                connections += 1;
                settle_visible_overworld_frame_arrival(&mut runtime_shell)?;
            }
            if frame.wild_battle.is_some() {
                wild_battles += 1;
                prepare_visible_battle_entry(&mut runtime_shell)?;
                settle_visible_battle_after_action(&mut runtime_shell)?;
                sync_visible_battle_action_cursor(&mut runtime_shell);
                if runtime_shell.shell.snapshot()?.battle.is_some() {
                    finish_visible_overworld_random_battle(&mut runtime_shell)?;
                }
            }
        }
        settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    }

    let final_snapshot = runtime_shell.shell.snapshot()?;
    let final_scene = current_visible_scene_label(&runtime_shell)?;
    if let Some(save_path) = save_path {
        runtime_shell.shell.save(save_path)?;
    }
    let active_music = runtime_shell.active_music.clone();
    let audio_events = runtime_shell.last_audio_events.clone();
    let pending_audio = runtime_shell.pending_audio.len();
    let final_party_species = final_snapshot
        .party
        .slots
        .iter()
        .map(|slot| slot.pokemon.species.id.clone())
        .collect();
    let final_bag_items = final_snapshot.bag.items.clone();
    Ok(VisibleShellOverworldSmoke {
        start_map,
        start_tile_x,
        start_tile_y,
        start_scene,
        final_map: final_snapshot.overworld.map_name,
        final_tile_x: final_snapshot.overworld.tile.x,
        final_tile_y: final_snapshot.overworld.tile.y,
        final_scene,
        frames: input_frames.len(),
        interactions,
        coord_events,
        trainer_sight_events,
        warps,
        connections,
        wild_battles,
        last_movement,
        active_music,
        audio_events,
        pending_audio,
        final_party_species,
        final_bag_items,
        frame_events,
        state_hash: final_snapshot.state_checksum,
    })
}

fn current_visible_scene_label(runtime_shell: &BevyRuntimeShell) -> Result<Option<String>> {
    runtime_shell
        .shell
        .current_scene_script()
        .map(|scene| scene.map(|scene| scene.scene_id))
}

struct VisibleShellSmokeFrameOutcome {
    frame: Option<crate::RuntimeOverworldFrame>,
    interaction: bool,
}

fn apply_visible_shell_smoke_frame(
    runtime_shell: &mut BevyRuntimeShell,
    buttons: &[GameButton],
) -> Result<VisibleShellSmokeFrameOutcome> {
    let mut overworld_buttons = Vec::new();
    let mut interaction = false;
    for button in buttons.iter().copied() {
        match button {
            GameButton::Start if has_visible_shell_start_action(runtime_shell) => {
                press_visible_start_button(runtime_shell)?;
            }
            GameButton::A if has_visible_shell_a_action(runtime_shell)? => {
                if runtime_shell
                    .shell
                    .last_frame()
                    .and_then(|frame| frame.interaction.as_ref())
                    .is_some()
                    || runtime_shell
                        .shell
                        .current_overworld_interaction_checked()?
                        .is_some()
                {
                    interaction = true;
                }
                press_visible_a_button(runtime_shell)?;
            }
            GameButton::B if has_visible_shell_b_action(runtime_shell) => {
                press_visible_b_button(runtime_shell)?;
            }
            GameButton::Select if has_visible_shell_select_action(runtime_shell) => {
                press_visible_select_button(runtime_shell)?;
            }
            GameButton::Up if has_visible_shell_direction_action(runtime_shell) => {
                move_visible_primary_cursor_up(runtime_shell)?;
            }
            GameButton::Down if has_visible_shell_direction_action(runtime_shell) => {
                move_visible_primary_cursor_down(runtime_shell)?;
            }
            GameButton::Left if has_visible_shell_direction_action(runtime_shell) => {
                move_visible_primary_cursor_left(runtime_shell)?;
            }
            GameButton::Right if has_visible_shell_direction_action(runtime_shell) => {
                move_visible_primary_cursor_right(runtime_shell)?;
            }
            button => overworld_buttons.push(button),
        }
    }
    let frame = if overworld_buttons.is_empty() {
        None
    } else {
        Some(runtime_shell.shell.tick(overworld_buttons)?.clone())
    };
    Ok(VisibleShellSmokeFrameOutcome { frame, interaction })
}

fn settle_visible_shell_smoke_until_idle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MAX_IDLE_SETTLE_STEPS: usize = 1024;
    for _ in 0..MAX_IDLE_SETTLE_STEPS {
        // Smoke execution has no Bevy `Time` system, but ASM delays, emotes,
        // and earthquakes still consume frames. Advance their presentation
        // clocks exactly one frame per settle iteration so script commands
        // following `showemote`/`pause` can become reachable.
        if let Some(frames) = runtime_shell.visible_script_delay_frames.as_mut() {
            *frames = frames.saturating_sub(1);
        }
        if runtime_shell.visible_heal_machine.is_some() {
            advance_visible_heal_machine(runtime_shell)?;
            continue;
        }
        if runtime_shell.visible_battle_transition.is_some() {
            advance_visible_battle_transition(runtime_shell);
            continue;
        }
        let field_travel_delay_finished =
            if let Some(frames) = runtime_shell.pending_field_travel_delay_frames.as_mut() {
                *frames = frames.saturating_sub(1);
                *frames == 0
            } else {
                false
            };
        if field_travel_delay_finished {
            runtime_shell.pending_field_travel_delay_frames = None;
            runtime_shell.field_notice = None;
            runtime_shell.field_notice_queue.clear();
            runtime_shell.pending_sweet_scent_nothing_notice = false;
            runtime_shell.pending_field_travel_arrival = false;
            if runtime_shell.visible_field_travel_animation
                == Some(VisibleFieldTravelAnimation::TeleportFrom)
            {
                queue_visible_shell_sound_effect(runtime_shell, "SFX_WARP_TO")?;
                begin_visible_teleport_travel_animation(runtime_shell, false)?;
                mark_runtime_snapshot_dirty(runtime_shell);
                continue;
            }
            runtime_shell.field_notice_scene = None;
            settle_visible_overworld_travel(runtime_shell)?;
            mark_runtime_snapshot_dirty(runtime_shell);
            continue;
        }
        if let Some(emote) = runtime_shell.visible_overworld_emote.as_mut() {
            emote.frames_remaining = emote.frames_remaining.saturating_sub(1);
        }
        if let Some(earthquake) = runtime_shell.visible_earthquake.as_mut() {
            earthquake.frames_remaining = earthquake.frames_remaining.saturating_sub(1);
            earthquake.phase = earthquake.phase.wrapping_add(1) % 4;
        }
        if runtime_shell.pending_trainer_sight.is_some() {
            advance_visible_trainer_sight_cutscene(runtime_shell)?;
            if runtime_shell.pending_trainer_sight.is_some() {
                continue;
            }
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        if runtime_shell.pending_gender_selection.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_time_set.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_oak_intro.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_name_input.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_name_choice.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_day_of_week.is_some() {
            // Smoke sessions choose the default weekday and then accept the
            // ASM confirmation prompt, exactly as two consecutive A presses.
            confirm_visible_day_of_week(runtime_shell)?;
            if runtime_shell.pending_day_of_week.is_some() {
                confirm_visible_day_of_week(runtime_shell)?;
            }
            continue;
        }
        if !has_visible_auto_script_action(runtime_shell, &snapshot) {
            return Ok(());
        }
        if advance_visible_next_pending_script_request(runtime_shell, &snapshot)? {
            continue;
        }
        if snapshot.ui.pending_text_wait.is_some() {
            advance_visible_pending_text_wait(runtime_shell)?;
            continue;
        }
        if snapshot.ui.pending_yes_no.is_some() {
            accept_visible_pending_yes_no(runtime_shell)?;
            continue;
        }
        if snapshot.ui.menu.is_some()
            || snapshot.ui.window_open
            || (snapshot.ui.text_window_open && runtime_shell.active_script_cursor.is_none())
            || snapshot.ui.active_pokemon_picture.is_some()
            || snapshot.pending_shop.is_some()
        {
            close_active_runtime_surface(runtime_shell)?;
            continue;
        }
        if runtime_shell.special_boundary.is_some() {
            close_visible_special_boundary(runtime_shell)?;
            continue;
        }
        if snapshot.script_events.script_ended.is_some() {
            take_visible_script_end_state(runtime_shell)?;
            continue_visible_script_after_prompt(runtime_shell)?;
            continue;
        }
        if !snapshot.script_events.audio_events.is_empty() {
            drain_visible_audio_events(runtime_shell)?;
            continue_visible_script_after_prompt(runtime_shell)?;
            continue;
        }
        if has_visible_pending_non_audio_script_events(&snapshot) {
            drain_visible_non_audio_script_events(runtime_shell)?;
            continue_visible_script_after_prompt(runtime_shell)?;
            continue;
        }
        if visible_auto_runtime_flag(&snapshot).is_some() {
            consume_visible_runtime_flag(runtime_shell)?;
            continue;
        }
        if runtime_shell.active_script_cursor.is_some() {
            execute_visible_active_script_step(runtime_shell)?;
            continue;
        }
        if !snapshot.script_events.command_queue.is_empty() {
            execute_next_visible_queued_script_command(runtime_shell)?;
            continue;
        }
        if snapshot.script_events.next_script.is_some() {
            take_visible_next_script(runtime_shell)?;
            continue;
        }
        if !snapshot.script_events.deferred_scripts.is_empty() {
            take_visible_deferred_script(runtime_shell)?;
            continue;
        }
        anyhow::bail!("visible shell smoke could not settle active runtime surface");
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let active_cursor_command = runtime_shell
        .active_script_cursor
        .as_ref()
        .and_then(|cursor| {
            runtime_shell
                .shell
                .runtime()
                .compiled_script_commands(&cursor.source_script)
                .ok()
                .and_then(|commands| commands.get(cursor.next_command_index).cloned())
        });
    anyhow::bail!(
        "visible shell smoke exceeded idle settle limit {MAX_IDLE_SETTLE_STEPS}: cursor={:?} cursor_command={active_cursor_command:?} special_boundary={:?} next_script={:?} command_queue={} deferred={} ended={:?} pending_text_label={:?} pending_text_wait={:?} text_window={} window={} runtime_flag={:?} non_audio_events={} recent_audio={:?}",
        runtime_shell.active_script_cursor,
        runtime_shell.special_boundary,
        snapshot.script_events.next_script,
        snapshot.script_events.command_queue.len(),
        snapshot.script_events.deferred_scripts.len(),
        snapshot.script_events.script_ended,
        snapshot.script_events.pending_text_label,
        snapshot.ui.pending_text_wait,
        snapshot.ui.text_window_open,
        snapshot.ui.window_open,
        visible_auto_runtime_flag(&snapshot),
        has_visible_pending_non_audio_script_events(&snapshot),
        runtime_shell.last_audio_events
    )
}

fn initialize_bevy_runtime_shell(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
) -> Result<BevyRuntimeShell> {
    #[cfg(any(test, feature = "location-tester"))]
    let runtime_tile_start = matches!(&start, BevyShellStart::NewGameAtRuntimeTile { .. });
    let asset_root = if runtime.has_runtime_files() {
        runtime.materialize_runtime_files()?
    } else {
        asset_root
    };
    let initial_arrival_reason = match &start {
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGame { .. } => Some("new_game"),
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGameAtRuntimeTile { .. } => None,
        BevyShellStart::LoadSave { .. } => None,
        BevyShellStart::Title { .. } => None,
    };
    let restore_loaded_visible_state = matches!(&start, BevyShellStart::LoadSave { .. });
    #[cfg(any(test, feature = "location-tester"))]
    let initial_player_name_prompt = matches!(&start, BevyShellStart::NewGame { .. });
    #[cfg(not(any(test, feature = "location-tester")))]
    let initial_player_name_prompt = false;
    let title_menu = match &start {
        BevyShellStart::Title {
            spawn_identifier,
            save_path,
        } => Some(TitleMenu {
            spawn_identifier: *spawn_identifier,
            save_path: save_path.clone(),
            cursor: MenuCursor {
                surface_id: "title".to_string(),
                option_index: 0,
            },
            phase: VisibleTitlePhase::Entrance,
            frame: 0,
            main_menu_frame: 0,
            scx: VISIBLE_TITLE_ENTRANCE_START_SCX,
            title_timer: 0,
            clock_reset_trigger: false,
        }),
        BevyShellStart::LoadSave { .. } => None,
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGame { .. } => None,
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGameAtRuntimeTile { .. } => None,
    };
    let intro_screen = matches!(&start, BevyShellStart::Title { .. }).then(VisibleIntroScreen::new);
    let mut shell = match start {
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGame { spawn_identifier } => {
            RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), spawn_identifier)?
        }
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier,
            map_name,
            tile_x,
            tile_y,
        } => RuntimeGameShell::new_game_at_runtime_tile(
            asset_root.clone(),
            runtime.clone(),
            spawn_identifier,
            &map_name,
            tile_x,
            tile_y,
        )?,
        BevyShellStart::LoadSave { save_path } => {
            RuntimeGameShell::resume_from_save(asset_root.clone(), runtime.clone(), save_path)?
        }
        BevyShellStart::Title {
            spawn_identifier,
            save_path: _,
        } => RuntimeGameShell::new_game(asset_root.clone(), runtime.clone(), spawn_identifier)?,
    };
    if title_menu.is_some() || initial_player_name_prompt {
        // MainMenu clears GAME_TIMER_COUNTING_F. The bit is armed only when
        // the new-game introduction reaches FinishContinue.
        shell
            .session_mut()
            .state_mut()
            .set_game_timer_counting(false);
    }
    // The interactive Bevy shell does not need to retain a serialized
    // command/result frame for every 60 Hz joypad sample.  Keeping the
    // authoritative state/checksum while disabling that diagnostic journal
    // avoids per-frame allocation and state-frame encoding.
    shell.set_runtime_journal_enabled(false);

    let initial_snapshot = shell.snapshot()?;
    let deterministic_session_start = initial_snapshot.state_checksum;
    let deterministic_session_checkpoint = if initial_snapshot.trainer.player_name.is_empty() {
        None
    } else {
        Some(visible_deterministic_session_checkpoint(
            &shell,
            deterministic_session_start.clone(),
        )?)
    };
    let mut runtime_shell = BevyRuntimeShell {
        lcd_animation_frame: 0,
        ambient_tileset_animation_active: false,
        ambient_tileset_animation_schedule: Vec::new(),
        battle_lcd_animation_active: false,
        asset_root,
        runtime,
        shell,
        latest_rtc_sample: None,
        intro_screen,
        title_menu,
        visible_continue_screen: None,
        credits_screen: None,
        last_error: None,
        last_action_status: None,
        last_audio_events: Vec::new(),
        pending_audio: Vec::new(),
        audio_source_cache: HashMap::new(),
        trainer_items_used: BTreeSet::new(),
        pending_music_stop: false,
        transient_audio_playing: false,
        active_music: None,
        faded_music: None,
        last_battle_cry_key: None,
        pending_battle_cries_after_messages: VecDeque::new(),
        battle_enemy_send_out_pending: false,
        battle_player_send_out_pending: false,
        battle_enemy_hp_at_player_send_out: None,
        pending_battle_scenes_after_message: VecDeque::new(),
        pending_enemy_response_after_capture: None,
        pending_plain_battle_map_reload: false,
        last_overworld_input: None,
        player_walk_from: None,
        player_walk_frame_ticks: 0,
        player_walk_total_ticks: WALK_FRAME_HOLD_TICKS,
        player_walk_stride: false,
        player_walk_mirror_stride: false,
        object_walk_frame_ticks: 0,
        object_walk_total_ticks: WALK_FRAME_HOLD_TICKS,
        object_walk_frame_ticks_by_id: BTreeMap::new(),
        object_walk_total_ticks_by_id: BTreeMap::new(),
        object_walk_stride: false,
        object_walk_from: BTreeMap::new(),
        object_walk_phases: BTreeMap::new(),
        object_walk_directions: BTreeMap::new(),
        trainer_walk_from: None,
        pending_overworld_step_boundary: None,
        pending_overworld_warp_scene: None,
        visible_script_movement: None,
        visible_script_movement_scene: None,
        overworld_direction_repeat_ticks: 0,
        overworld_held_direction: None,
        overworld_held_directions: VecDeque::new(),
        overworld_buffered_direction: None,
        ui_held_direction: None,
        ui_direction_repeat_ticks: 0,
        recent_overworld_inputs: VecDeque::new(),
        deterministic_session_start,
        deterministic_session_checkpoint,
        deterministic_input_frames: VecDeque::new(),
        deterministic_battle_actions: VecDeque::new(),
        deterministic_menu_results: VecDeque::new(),
        last_runtime_action: None,
        quick_save_path: config.quick_save_path,
        active_script_cursor: None,
        pending_map_callbacks: Vec::new(),
        map_callback_return_cursor: None,
        map_reload_return_cursor: None,
        pending_scene_script: None,
        script_command_cursor: 0,
        start_menu_cursor: None,
        menu_cursor: None,
        sell_cursor: None,
        shop_top_cursor: Some(MenuCursor {
            surface_id: "shop:top".to_string(),
            option_index: 0,
        }),
        shop_quantity: None,
        shop_notice: None,
        shop_welcome_seen: false,
        shop_return_to_top_after_notice: false,
        shop_close_after_notice: false,
        elevator_cursor: None,
        gift_pokemon_cursor: None,
        yes_no_cursor: None,
        pending_phone_prompt: None,
        pending_day_of_week: None,
        pending_trainer_sight: None,
        previous_map_sign_landmark: if matches!(
            initial_snapshot.overworld.map_name.as_str(),
            "Route35NationalParkGate" | "Route36NationalParkGate"
        ) {
            Some("__MAP_NAME_SIGN_SENTINEL__".to_string())
        } else {
            initial_snapshot
                .presentation
                .pokegear_landmarks
                .map_to_landmark
                .get(&initial_snapshot.overworld.map_name)
                .cloned()
        },
        visible_map_name_sign: None,
        pending_delete_save: None,
        pending_clock_reset: None,
        pending_mystery_gift: None,
        pending_time_set: None,
        pending_oak_intro: None,
        pending_gender_selection: None,
        screen_fade: None,
        visible_blackout_phase: None,
        visible_walk_warp_phase: None,
        field_text_reveal: None,
        selected_player_gender: None,
        pending_name_input: None,
        pending_name_choice: None,
        pending_standard_capture: None,
        party_menu_open: false,
        party_summary_open: false,
        party_summary_page: 1,
        party_cursor: 0,
        party_action_cursor: None,
        party_give_take_cursor: None,
        party_mail_take_stage: None,
        party_held_item_give_target: None,
        held_item_swap_prompt: false,
        pending_contextual_field_move: None,
        pending_script_party_selection: None,
        kurt_apricorn_cursor: None,
        kurt_apricorn_quantity: None,
        buena_prize_cursor: None,
        visible_unown_puzzle: None,
        visible_slot_machine: None,
        visible_card_flip: None,
        visible_heal_machine: None,
        visible_magnet_train: None,
        visible_unown_words: None,
        visible_diploma: None,
        visible_battle_transition: None,
        visible_capture_animation: None,
        visible_move_animations: VecDeque::new(),
        visible_send_out_animation: None,
        visible_trainer_exit_animation: None,
        visible_frontpic_animation: None,
        visible_fishing_animation: None,
        heal_music_active: false,
        party_move_reorder_open: false,
        party_move_reorder_origin: None,
        party_switch_cursor: None,
        party_hp_transfer_source: None,
        party_hp_transfer_move: None,
        pokedex_menu_open: false,
        pokedex_detail_open: false,
        pokedex_detail_page: 0,
        pokedex_scripted_entry: false,
        pokedex_cursor: 0,
        pokegear_menu_open: false,
        pokegear_cursor: 0,
        pokegear_phone_cursor: 0,
        pokegear_phone_status: None,
        pokegear_page: PokegearPage::Clock,
        pokegear_radio_station: None,
        pokegear_radio_segment: 0,
        pokegear_radio_index: 0,
        active_pokegear_radio: None,
        trainer_card_open: false,
        trainer_card_page: VisibleTrainerCardPage::Info,
        trainer_card_colon_visible: false,
        trainer_card_colon_ticks: 0,
        trainer_card_badge_frame: 0,
        trainer_card_badge_ticks: 0,
        options_menu_open: false,
        options_cursor: 0,
        save_menu_open: false,
        save_flow: None,
        special_boundary: None,
        special_boundary_queue: VecDeque::new(),
        pending_special_cry: None,
        pending_special_sound: None,
        visible_balance_overlay: None,
        visible_mom_bank: None,
        visible_overworld_emote: None,
        visible_earthquake: None,
        visible_ledge_jump: None,
        visible_grass_rustle: None,
        visible_strength_boulder_dust: None,
        visible_script_delay_frames: None,
        poison_flash_frames_remaining: 0,
        field_pack_pocket: None,
        last_field_pack_pocket: FieldPackPocket::Items,
        field_pack_cursor_positions: [0; 4],
        field_pack_action_cursor: None,
        field_pack_target_mode: None,
        tmhm_teach_prompt_cursor: None,
        pending_tmhm_teach_prompt_after_boot: false,
        tmhm_decision_prompt_cursor: None,
        tmhm_decision: None,
        tmhm_forget_menu_open: false,
        move_learn_decision_cursor: None,
        move_learn_decision: None,
        move_learn_forget_menu_open: false,
        battle_pack_target_mode: None,
        pack_toss: None,
        battle_messages: VecDeque::new(),
        battle_text_reveal: None,
        battle_fanfare_messages: VecDeque::new(),
        battle_evolution_cries: VecDeque::new(),
        battle_evolution_cancellations: VecDeque::new(),
        field_evolution_cancellation: None,
        battle_sounds_after_messages: VecDeque::new(),
        battle_entry_messages_remaining: 0,
        battle_message_scene: None,
        battle_message_scenes: VecDeque::new(),
        battle_exp_tween: None,
        pending_battle_exp_tweens: VecDeque::new(),
        battle_level_stats: VecDeque::new(),
        battle_hp_tween: None,
        bag_cursor: None,
        key_item_cursor: None,
        ball_cursor: None,
        tmhm_cursor: None,
        custom_item_cursor: None,
        storage_cursor: None,
        pc_item_cursor: None,
        pc_item_action: None,
        pc_item_quantity: None,
        pc_hub_session_open: false,
        pc_hub_cursor: None,
        hall_of_fame_pc_index: None,
        player_pc_action_cursor: None,
        mailbox_cursor: None,
        mailbox_action_cursor: None,
        mailbox_attach_index: None,
        pc_confirmation: None,
        bill_pc_session_open: false,
        bill_pc_action_cursor: None,
        bill_pc_box_cursor: None,
        bill_pc_move_open: false,
        bill_pc_move_source: None,
        bill_pc_pokemon_action_cursor: None,
        bill_pc_box_summary: None,
        pending_pc_release: None,
        pc_notice: None,
        field_notice: None,
        field_notice_queue: VecDeque::new(),
        pending_sweet_scent_nothing_notice: false,
        pending_item_notification: None,
        field_notice_scene: None,
        pending_field_travel_arrival: false,
        pending_field_travel_delay_frames: None,
        visible_field_travel_animation: None,
        pending_field_notice_sound: None,
        pending_field_notice_cry: None,
        visible_strength_notice_phase: None,
        pending_field_battle_entry: false,
        pending_field_notice_effect_frames: None,
        visible_sweet_scent_delay: false,
        visible_cut_animation: None,
        visible_whirlpool_animation: None,
        visible_headbutt_animation: None,
        visible_flash_animation: None,
        visible_fly_animation: None,
        visible_waterfall_animation: None,
        pending_surf_start_from: None,
        fly_cursor: None,
        battle_action_cursor: None,
        battle_move_cursor: None,
        battle_move_swap_origin: None,
        battle_shift_prompt_cursor: None,
        battle_faint_prompt_cursor: None,
        battle_switch_cursor: None,
        battle_party_action_cursor: None,
        battle_party_summary_open: false,
        pending_battle_move_switch_slot: None,
        party_move_cursor: None,
        snapshot_revision: 0,
        cached_snapshot: None,
    };
    if initial_player_name_prompt {
        open_visible_name_choice(&mut runtime_shell)?;
    }
    if runtime_shell.pending_name_input.is_none() && runtime_shell.pending_name_choice.is_none() {
        if restore_loaded_visible_state {
            restore_visible_loaded_runtime_state(&mut runtime_shell, "load_save")?;
        } else if let Some(reason) = initial_arrival_reason {
            settle_visible_overworld_arrival(&mut runtime_shell, reason)?;
        }
    }
    #[cfg(any(test, feature = "location-tester"))]
    if runtime_tile_start {
        // Runtime-tile starts still execute the map's authoritative callback.
        // Finish its terminal EndCallback/control records before exposing the
        // first test joypad frame, just as an ordinary arrival does.
        continue_visible_script_after_prompt(&mut runtime_shell)?;
        let snapshot = runtime_shell.shell.snapshot()?;
        if !snapshot.script_events.audio_events.is_empty() {
            drain_visible_audio_events(&mut runtime_shell)?;
        }
        if has_visible_pending_non_audio_script_events(&snapshot) {
            drain_visible_non_audio_script_events(&mut runtime_shell)?;
        }
        close_visible_noninteractive_runtime_surfaces_until_idle(&mut runtime_shell)?;
    }
    // Initialization may execute callbacks through direct shell mutations;
    // never carry an intermediate routing snapshot into the first joypad
    // frame.
    mark_runtime_snapshot_dirty(&mut runtime_shell);
    Ok(runtime_shell)
}

include!("bevy_shell/deterministic_session.rs");
include!("bevy_shell/field_travel.rs");
include!("bevy_shell/trainer_card.rs");
include!("bevy_shell/title_menu.rs");
include!("bevy_shell/credits.rs");
include!("bevy_shell/script_callbacks.rs");
include!("bevy_shell/economy.rs");
include!("bevy_shell/battle_messages.rs");
include!("bevy_shell/battle_results.rs");
include!("bevy_shell/battle_entry.rs");
include!("bevy_shell/menu_rendering.rs");
include!("bevy_shell/render_mod.rs");
include!("bevy_shell/overworld_rendering.rs");
include!("bevy_shell/start_menu.rs");
include!("bevy_shell/bitmap_font.rs");
include!("bevy_shell/graphics_assets.rs");
include!("bevy_shell/field_pack.rs");

#[cfg(test)]
#[path = "bevy_shell/tests.rs"]
mod tests;
