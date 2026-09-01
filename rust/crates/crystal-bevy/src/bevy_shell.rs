use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::Arc;
#[cfg(all(not(test), not(target_arch = "wasm32")))]
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use anyhow::{Context, Result};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSampler;
#[cfg(feature = "location-tester")]
use bevy::render::view::screenshot::ScreenshotManager;
#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
use bevy::render::{Render, RenderApp, RenderSet};
#[cfg(feature = "location-tester")]
use bevy::window::PrimaryWindow;
use bevy::window::{PresentMode, WindowFocused, WindowResolution};
use bevy::winit::{UpdateMode, WinitSettings};
use chrono::{Datelike, Local as ChronoLocal, Timelike};
use crystal_assets::{
    RuntimeBadgeRegion, RuntimeBugContestAction, RuntimeCurrencyAccount, RuntimeDayCareAction,
    RuntimeDayCareCaretaker, RuntimeGameCornerService, RuntimeGenderMenuDefinition,
    RuntimeGraphicsSpecial, RuntimeHappinessServiceRoutine, RuntimeLinkBattleResult,
    RuntimeMysteryGiftAction, RuntimePartyCheckSpecial, RuntimePhoneRandomSpecial,
    RuntimePresentationPhaseMachine, RuntimePresentationProgram, RuntimeShuckieAction,
    RuntimeStoryGateSpecial, RuntimeTitleMainMenuDefinition, RuntimeTitleMainMenuItem,
    RuntimeTitlePresentationParameters,
};

use crate::assets::{
    ModpackAudioKind, ModpackAudioPlaybackMode, RuntimeMutationOutcome, RuntimeMutationResult,
    RuntimePendingScriptRequestKind, RuntimeScriptEventQueue, RuntimeScriptRuntimeFlag,
    RuntimeScriptRuntimeFlagValue, RuntimeScriptRuntimeMemoryEntry,
    RuntimeScriptRuntimeMemoryValue, RuntimeScriptRuntimeQueue,
    RuntimeScriptRuntimeQueueDrainResult,
};
use crate::audio::{AudioKind, AudioPcmFormat, AudioProgramSource};
use crate::core::battle::start::LinkBattleStart;
use crate::core::battle::turn::{BattleAction, active_battle_combat_state};
use crate::core::input::GameButton;
use crate::core::models::Dv;
use crate::core::multiplayer::{
    BattleActionFrame, DeterministicInputJournal, DeterministicInputJournalFrame,
    DeterministicReplayBundle, LinkBattleRngFrame, LinkMessage, LinkPartyFrame, LockstepFrame,
    MenuChoiceFrame, MenuChoiceResultFrame, PlayerInputFrame, SaveResumeReplayBundle,
    SessionRuntimeCommandFrame, SessionRuntimeCommandResultFrame, SessionSaveCheckpointFrame,
    StateChecksum, StateChecksumFrame, TradeConfirmation, TradeOffer, TradeSyncBuffer,
    encode_link_message_bytes,
};
use crate::core::random::BattleRandomSource;
#[cfg(test)]
use crate::core::state::SlotMachineState;
use crate::core::state::{
    BattleScene, BattleStyle, CardFlipInput, CardFlipPhase, FrameType, MenuAccount,
    PLAYER_GENDER_FEMALE, PLAYER_GENDER_MALE, PrintOption, ScriptFadeColor, ScriptFadeDirection,
    SlotMachineInput, SlotMachinePhase, Sound, TextSpeed,
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
use crate::core::systems::script_warps::map_setup_callback_kinds;
use crate::core::systems::shop::format_price;
use crate::core::systems::special_routines::SpecialRoutineEffect;
use crate::core::systems::time::{ClockTime, DAY_HOUR, GameDate, MORN_HOUR, NITE_HOUR};
use crate::core::systems::tmhm::TmHmLearnError;
use crate::core::timing::{Frame, GB_FRAME_DURATION_SECONDS};
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
    RuntimeLinkSessionDescriptor, RuntimeMapCatalogSnapshot, RuntimePendingScriptRequest,
    RuntimeResolvedAudioPlaybackKind, RuntimeRtcSample, RuntimeShellSnapshot, assets::AssetRoot,
};

mod intro_renderer;

const GAME_TICK_SECONDS: f32 = GB_FRAME_DURATION_SECONDS as f32;
const VIEWPORT_TILES_X: i16 = 20;
const VIEWPORT_TILES_Y: i16 = 18;
// A normal collision step spans two 8x8 render tiles. Camera interpolation
// can therefore expose almost that much source art beyond any LCD edge. Keep
// one complete runtime-tile halo attached to the moving destination surface.
const CLASSIC_SCROLL_HALO_TILES: i16 = METATILE_WIDTH;
const CLASSIC_SCROLL_TILES_X: i16 = VIEWPORT_TILES_X + CLASSIC_SCROLL_HALO_TILES * 2;
const CLASSIC_SCROLL_TILES_Y: i16 = VIEWPORT_TILES_Y + CLASSIC_SCROLL_HALO_TILES * 2;
#[cfg(feature = "voxel-view")]
// The pitched camera needs enough real map context behind the 20x18 LCD to
// keep the far plane from cutting through structures. The optional renderer
// deliberately publishes a broad map halo so pitched views can keep drawing
// streets, trees, landmarks, and connected-map terrain far beyond the LCD
// window instead of exposing a clipped background edge.
const VISUAL_WORLD_HALO_TILES: i16 = 32;
#[cfg(not(feature = "voxel-view"))]
const VISUAL_WORLD_HALO_TILES: i16 = CLASSIC_SCROLL_HALO_TILES;
const VISUAL_WORLD_TILES_X: i16 = VIEWPORT_TILES_X + VISUAL_WORLD_HALO_TILES * 2;
const VISUAL_WORLD_TILES_Y: i16 = VIEWPORT_TILES_Y + VISUAL_WORLD_HALO_TILES * 2;
// The Game Boy Color LCD is exactly 20 by 18 tiles.  Render source tiles at a
// uniform 4x integer scale so every game surface occupies the 640 by 576
// window; using 3x left an exposed, non-Game-Boy backing area around screens.
const TILE_SIZE: f32 = 32.0;
const PLAYFIELD_LEFT: f32 = -320.0;
const PLAYFIELD_TOP: f32 = 288.0;
const PLAYFIELD_WIDTH: f32 = VIEWPORT_TILES_X as f32 * TILE_SIZE;
const PLAYFIELD_HEIGHT: f32 = VIEWPORT_TILES_Y as f32 * TILE_SIZE;
const CLASSIC_SCROLL_WIDTH: f32 = CLASSIC_SCROLL_TILES_X as f32 * TILE_SIZE;
const CLASSIC_SCROLL_HEIGHT: f32 = CLASSIC_SCROLL_TILES_Y as f32 * TILE_SIZE;
const EVENT_LOG_LIMIT: usize = 192;
const RECENT_OVERWORLD_INPUT_LIMIT: usize = 2048;
const WALK_FRAME_HOLD_TICKS: u8 = 8;
const OVERWORLD_TURN_HOLD_TICKS: u8 = 4;
// Crystal advances a walking tile over several VBlanks.  The core session
// operates on completed tiles, so the real-time host must gate held movement
// instead of applying a full tile at every 60 Hz input sample.
const OVERWORLD_STEP_REPEAT_TICKS: u8 = 8;
// Match TypeScript's five-frame accumulator: ordinary 20-60 Hz host updates
// retain the Game Boy's 60 Hz wall-clock pace, while a real stall still cannot
// turn into an unbounded burst of gameplay or buffered joypad commands.
const MAX_RUNTIME_CATCH_UP_TICKS: u32 = 5;
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
// `PrintText` begins at TEXTBOX_INNERY (TEXTBOX_Y + BORDER_WIDTH), and
// `LineChar` moves to TEXTBOX_INNERY + 2. The four-tile-high interior thus
// contains two text baselines separated by one tile row, not four lines.
const FIELD_TEXT_BOX_TEXT_TOP_TILE: f32 = FIELD_TEXT_BOX_TOP_TILE + 2.0;
// ASM `YesNoBox` in home/menu.asm: menu_coords 14, 7, 19, 11.
const FIELD_YES_NO_LEFT_TILE: f32 = 14.0;
const FIELD_YES_NO_TOP_TILE: f32 = 7.0;
const FIELD_YES_NO_WIDTH_TILES: f32 = 6.0;
// ASM `_YesNoBox` passes a 4x2 interior to `Textbox`, producing a 6x4
// outer window. TypeScript's YesNoPrompt records the same 6x4 region.
const FIELD_YES_NO_HEIGHT_TILES: f32 = 4.0;
const FIELD_TEXT_BOX_ROW_SPACING_TILES: f32 = 2.0;
const FIELD_TEXT_BOX_VISIBLE_ROWS: usize = 2;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BevyMultiplayerConfig {
    pub server_url: String,
    pub server_token: Option<String>,
    pub world_id: String,
    pub player_id: u64,
    pub display_name: String,
    pub rating: i32,
    pub rating_range: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BevyShellConfig {
    pub quick_save_path: Option<PathBuf>,
    pub smoke_player_name: Option<String>,
    /// `None` keeps the optional voxel feature's normal enabled behavior;
    /// location tools can force either side of a 2D/2.5D comparison.
    pub voxel_view_enabled: Option<bool>,
    pub window_title: Option<String>,
    pub multiplayer: Option<BevyMultiplayerConfig>,
    #[cfg(feature = "location-tester")]
    pub render_test_screenshot: Option<PathBuf>,
    /// Fixed 24-hour clock used by deterministic location screenshots.
    /// Normal play continues to use the live/new-game clock path.
    #[cfg(feature = "location-tester")]
    pub render_test_hour: Option<u8>,
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

const WHITEOUT_POST_FADE_HOLD_FRAMES: u8 = 40;

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
    TrainerSight,
    CoordEvent,
    PhoneCall,
    WildBattle,
    PoisonBlackout(crate::core::systems::step_events::StepEventResult),
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleFollowerWalk {
    object_id: String,
    from: TilePosition,
    to: TilePosition,
    direction: Direction,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleDecorationEntry {
    id: String,
    display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleDecorationMenuPhase {
    Categories {
        categories: Vec<crystal_assets::DecorationCategory>,
        cursor: MenuCursor,
    },
    Decorations {
        category: crystal_assets::DecorationCategory,
        decorations: Vec<VisibleDecorationEntry>,
        cursor: MenuCursor,
    },
    Side {
        category: crystal_assets::DecorationCategory,
        decoration_id: Option<String>,
        item_cursor_index: usize,
        cursor: MenuCursor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleDecorationMenu {
    phase: VisibleDecorationMenuPhase,
    changed: bool,
    notice_queue: VecDeque<String>,
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
    intro_sprite_bundle: Option<SpriteAnimRuntimeBundle>,
    title_menu: Option<TitleMenu>,
    /// NewGame has executed ResetWRAM but has not yet reached the first
    /// playable overworld boundary. VBlank_Normal remains authoritative
    /// throughout gender, clock, Oak, and naming presentation.
    new_game_pre_overworld: bool,
    visible_continue_screen: Option<VisibleContinueScreen>,
    credits_screen: Option<VisibleCreditsScreen>,
    last_error: Option<String>,
    last_action_status: Option<String>,
    last_audio_events: Vec<String>,
    pending_audio: Vec<BevyAudioCommand>,
    audio_source_cache: HashMap<BevyAudioCacheKey, CachedPcmAudio>,
    pending_music_stop: bool,
    pending_full_audio_reset: bool,
    transient_audio_playing: bool,
    active_transient_kind: Option<ModpackAudioKind>,
    current_sfx_priority: u8,
    active_music: Option<String>,
    faded_music: Option<String>,
    music_volume: u8,
    music_fade: Option<VisibleMusicFade>,
    last_battle_cry_key: Option<String>,
    pending_battle_cries_after_messages: VecDeque<(String, String, String)>,
    battle_enemy_send_out_pending: bool,
    battle_player_send_out_pending: bool,
    battle_enemy_hp_at_player_send_out: Option<u16>,
    pending_battle_scenes_after_message: VecDeque<(String, Box<RuntimeShellSnapshot>)>,
    pending_plain_battle_map_reload: bool,
    last_overworld_input: Option<VisibleOverworldInputRecord>,
    overworld_interaction_consumed_a: bool,
    field_text_consumed_a: bool,
    field_text_consumed_b: bool,
    // Authoritative movement commits at a tile boundary. Retain its previous
    // tile so presentation can cross that boundary over real LCD frames.
    player_walk_from: Option<TilePosition>,
    player_walk_frame_ticks: u8,
    player_walk_total_ticks: u8,
    player_walk_stride: bool,
    player_walk_mirror_stride: bool,
    player_walk_direction_phases: HashMap<Direction, u8>,
    object_walk_frame_ticks: u8,
    object_walk_total_ticks: u8,
    object_walk_frame_ticks_by_id: BTreeMap<String, u8>,
    object_walk_total_ticks_by_id: BTreeMap<String, u8>,
    object_walk_stride: bool,
    // Core commits autonomous object tiles atomically. Retain their prior
    // tiles while the shell presents Crystal's eight-frame walk.
    object_walk_from: BTreeMap<String, TilePosition>,
    // A player-led follower consumes its queued step only when the player's
    // visible stride lands. TypeScript/Crystal keep this distinct from an
    // autonomous object stride so the follower remains one full step behind.
    pending_follower_walks: VecDeque<VisibleFollowerWalk>,
    follower_visible_tile_overrides: BTreeMap<String, TilePosition>,
    object_walk_phases: BTreeMap<String, u8>,
    object_walk_direction_phases: HashMap<(String, Direction), u8>,
    trainer_walk_from: Option<(String, TilePosition)>,
    pending_overworld_step_boundary: Option<PendingOverworldStepBoundary>,
    pending_overworld_warp_scene: Option<Arc<RuntimeShellSnapshot>>,
    visible_script_movement: Option<VisibleScriptMovement>,
    visible_script_movement_scene: Option<Arc<RuntimeShellSnapshot>>,
    // OBJECT_SPRITE_Y_OFFSET survives the end of a movement program. This is
    // observable between skyfall_top/teleport_from and the map load that
    // follows them, so it cannot live only inside the active animation.
    visible_player_sprite_y_offset: i16,
    overworld_direction_repeat_ticks: u8,
    overworld_held_direction: Option<GameButton>,
    overworld_held_directions: VecDeque<GameButton>,
    overworld_buffered_direction: Option<GameButton>,
    pending_overworld_direction_press: Option<GameButton>,
    pending_ui_button_presses: VecDeque<KeyCode>,
    ui_held_direction: Option<GameButton>,
    ui_direction_repeat_ticks: u8,
    recent_overworld_inputs: VecDeque<VisibleOverworldInputRecord>,
    deterministic_session_start: StateChecksum,
    deterministic_session_checkpoint: Option<SessionSaveCheckpointFrame>,
    deterministic_input_frames: VecDeque<PlayerInputFrame>,
    deterministic_battle_actions: VecDeque<BattleActionFrame>,
    pending_link_battle_action: Option<BattleAction>,
    pending_link_battle_replacement: Option<usize>,
    deterministic_menu_results: VecDeque<MenuChoiceResultFrame>,
    last_runtime_action: Option<VisibleRuntimeActionRecord>,
    quick_save_path: Option<PathBuf>,
    active_script_cursor: Option<ActiveScriptCursor>,
    map_reload_return_cursor: Option<RuntimeCompiledScriptCursor>,
    pending_scene_script: Option<String>,
    deferred_script_warp_arrival_scripts: bool,
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
    yes_no_cursor: Option<MenuCursor>,
    pending_phone_prompt: Option<PendingPhonePrompt>,
    pending_remember_password: Option<PendingRememberPasswordPrompt>,
    pending_day_of_week: Option<PendingDayOfWeekPrompt>,
    pending_trainer_sight: Option<PendingTrainerSight>,
    pending_trainer_intro: Option<PendingTrainerIntro>,
    visible_map_name_sign: Option<VisibleMapNameSign>,
    pending_delete_save: Option<VisibleDeleteSaveScreen>,
    pending_clock_reset: Option<VisibleClockResetScreen>,
    pending_mystery_gift: Option<VisibleMysteryGiftScreen>,
    pending_time_set: Option<VisibleTimeSetScreen>,
    pending_oak_intro: Option<VisibleOakIntroSequence>,
    pending_gender_selection: Option<VisibleGenderSelection>,
    screen_fade: Option<VisibleScreenFade>,
    visible_blackout_phase: Option<VisibleBlackoutPhase>,
    pending_poison_blackout: bool,
    visible_walk_warp_phase: Option<VisibleWalkWarpPhase>,
    field_text_reveal: Option<VisibleFieldTextReveal>,
    /// Last fully printed field-text page whose glyph update was accepted by
    /// the Bevy renderer. Autonomous script fences must not replace a page
    /// until this identity has been acknowledged.
    rendered_field_text_identity: Option<(String, usize)>,
    dialogue_log_identity: Option<(String, usize)>,
    dialogue_log_events: VecDeque<String>,
    movement_log_events: VecDeque<String>,
    input_log_events: VecDeque<String>,
    selected_player_gender: Option<VisiblePlayerGender>,
    pending_name_input: Option<PendingNameInput>,
    pending_mail_input: Option<PendingMailInput>,
    pending_mail_read: Option<VisibleMailRead>,
    pending_name_choice: Option<VisibleNameChoice>,
    pending_standard_capture: Option<PendingStandardCapture>,
    visible_bug_contest_replacement: Option<VisibleBugContestReplacement>,
    pending_gift_pokemon_nickname: Option<PendingGiftPokemonNickname>,
    pending_gift_pokemon_pc_notice: bool,
    pending_egg_hatch_nickname: Option<PendingEggHatchNickname>,
    visible_field_item_notice: Option<VisibleFieldItemNotice>,
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
    pending_link_trade_party_slot: Option<Option<usize>>,
    pending_link_trade_confirmation: Option<bool>,
    pending_link_trade_save: bool,
    pending_link_room_selection: Option<u8>,
    pending_linked_friend_wait: bool,
    pending_link_room_session: bool,
    pending_npc_trade_commit: Option<PendingNpcTradeCommit>,
    pending_photo_studio_commit: Option<usize>,
    kurt_apricorn_cursor: Option<MenuCursor>,
    kurt_apricorn_quantity: Option<u16>,
    buena_prize_cursor: Option<MenuCursor>,
    visible_buena_password: Option<VisibleBuenaPassword>,
    visible_battle_tower_challenge_menu: Option<VisibleBattleTowerChallengeMenu>,
    visible_battle_tower_room_menu: Option<VisibleBattleTowerRoomMenu>,
    visible_unown_puzzle: Option<VisibleUnownPuzzle>,
    visible_unown_printer: Option<VisibleUnownPrinter>,
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
    visible_egg_hatch: Option<VisibleEggHatch>,
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
    pokegear_standalone_map: bool,
    pokegear_cursor: usize,
    pokegear_phone_cursor: usize,
    pokegear_phone_status: Option<String>,
    pokegear_phone_call: Option<VisiblePokegearPhoneCall>,
    incoming_phone_sequence: Option<VisibleIncomingPhoneSequence>,
    pokegear_page: PokegearPage,
    pokegear_radio_station: Option<String>,
    pokegear_radio_segment: usize,
    pokegear_radio_tuning_knob: u8,
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
    visible_wait_sfx_boundary: bool,
    // `WaitPlaySFX` is a play-then-wait primitive, so a source loop must
    // promote only one transient command after the preceding cue finishes.
    pending_wait_play_sfx: VecDeque<String>,
    wait_play_sfx_completion: Option<VisibleWaitPlaySfxCompletion>,
    special_boundary_queue: VecDeque<SpecialBoundaryDisplay>,
    visible_special_text_pause_frames: Option<u8>,
    visible_internal_special_delay_frames: Option<u8>,
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
    pack_item_switch_origin: Option<(FieldPackPocket, usize)>,
    last_field_pack_pocket: FieldPackPocket,
    field_pack_cursor_positions: [usize; 4],
    field_pack_action_cursor: Option<MenuCursor>,
    field_pack_target_mode: Option<FieldPackTargetMode>,
    tmhm_teach_prompt_cursor: Option<MenuCursor>,
    pending_tmhm_text_stage: Option<VisibleTmHmTextStage>,
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
    decoration_menu: Option<VisibleDecorationMenu>,
    mailbox_cursor: Option<MenuCursor>,
    mailbox_action_cursor: Option<MenuCursor>,
    mailbox_attach_index: Option<usize>,
    pc_confirmation: Option<VisiblePcConfirmation>,
    bill_pc_session_open: bool,
    bill_pc_action_cursor: Option<MenuCursor>,
    bill_pc_box_cursor: Option<MenuCursor>,
    bill_pc_box_action_cursor: Option<MenuCursor>,
    bill_pc_move_open: bool,
    bill_pc_move_party_open: bool,
    bill_pc_move_source: Option<crystal_assets::RuntimePokemonStorageLocation>,
    bill_pc_move_save: Option<VisibleBillPcMoveSave>,
    bill_pc_pokemon_action_cursor: Option<MenuCursor>,
    bill_pc_box_summary: Option<VisiblePcBoxSummary>,
    pending_pc_release: Option<VisiblePcReleasePrompt>,
    pc_release_sequence: Option<VisiblePcReleaseSequence>,
    pc_transfer_sequence: Option<VisiblePcTransferSequence>,
    pc_notice: Option<String>,
    field_notice: Option<String>,
    field_notice_queue: VecDeque<String>,
    pending_item_notification: Option<String>,
    field_notice_scene: Option<Arc<RuntimeShellSnapshot>>,
    pending_field_travel_arrival: bool,
    pending_field_travel_delay_frames: Option<u16>,
    visible_field_travel_animation: Option<VisibleFieldTravelAnimation>,
    pending_field_notice_sound: Option<String>,
    pending_field_notice_cry: Option<String>,
    pending_field_battle_entry: bool,
    pending_field_notice_effect_frames: Option<u8>,
    visible_cut_animation: Option<VisibleCutAnimation>,
    pending_whirlpool_sound_wait: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleSaveFlowOrigin {
    StartMenu,
    BillsPcMove,
    BillsPcChangeBox { box_index: usize },
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisiblePcReleasePhase {
    Released,
    Bye,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePcReleaseSequence {
    box_index: usize,
    nickname: String,
    phase: VisiblePcReleasePhase,
    frames_remaining: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisiblePcTransferKind {
    Deposit,
    Withdraw,
    BoxPrint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisiblePcTransferPhase {
    SuccessHold,
    RefusalWaitSfx,
    RefusalHold,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePcTransferSequence {
    kind: VisiblePcTransferKind,
    box_index: usize,
    phase: VisiblePcTransferPhase,
    frames_remaining: u8,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleTmHmTextStage {
    Boot,
    Contained,
    Decision(VisibleTmHmDecision),
    RestoreMovePrompt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleSaveFlow {
    stage: VisibleSaveFlowStage,
    origin: VisibleSaveFlowOrigin,
    save_exists: bool,
    yes_no_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleBillPcMoveSavePhase {
    BeforeMove,
    AfterSave,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBillPcMoveSave {
    source: crystal_assets::RuntimePokemonStorageLocation,
    target: crystal_assets::RuntimePokemonStorageLocation,
    phase: VisibleBillPcMoveSavePhase,
    frames_remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMapNameSign {
    landmark: String,
    label: String,
    frames_remaining: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMusicFade {
    target_music: String,
    rate: u8,
    count: u8,
    fading_in: bool,
}

const SAVE_TEXT_WOULD_YOU_LIKE: &str = "_WouldYouLikeToSaveTheGameText";
const SAVE_TEXT_MOVE_MON_WITHOUT_MAIL: &str = "_MoveMonWOMailSaveText";
const SAVE_TEXT_CHANGE_BOX: &str = "_ChangeBoxSaveText";
const SAVE_TEXT_ALREADY_EXISTS: &str = "_AlreadyASaveFileText";
const SAVE_TEXT_SAVING: &str = "_SavingDontTurnOffThePowerText";
const SAVE_TEXT_SAVED: &str = "_SavedTheGameText";
const SAVE_TEXT_CORRUPTED: &str = "_SaveFileCorruptedText";

#[cfg(all(not(test), not(target_arch = "wasm32")))]
struct NativeAudioBackend {
    output: Option<NativeAudioOutput>,
    music_sink: Option<rodio::Sink>,
    music_volume: f32,
    transient_sinks: Vec<rodio::Sink>,
    transient_audio_id: Option<String>,
    transient_deadline: Option<Instant>,
    preparations: HashMap<BevyAudioCacheKey, NativeAudioPreparation>,
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
struct NativeAudioPreparation {
    started_at: Instant,
    receiver: Receiver<std::result::Result<CachedPcmAudio, String>>,
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
struct NativeAudioOutput {
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
impl NativeAudioBackend {
    fn new() -> Self {
        Self {
            output: None,
            music_sink: None,
            music_volume: 1.0,
            transient_sinks: Vec::new(),
            transient_audio_id: None,
            transient_deadline: None,
            preparations: HashMap::new(),
        }
    }

    fn poll_preparation(
        &mut self,
        cache_key: BevyAudioCacheKey,
        command: BevyAudioCommand,
        source: AudioProgramSource,
    ) -> Result<Option<CachedPcmAudio>> {
        if let Some(preparation) = self.preparations.get(&cache_key) {
            return match preparation.receiver.try_recv() {
                Ok(result) => {
                    let preparation = self
                        .preparations
                        .remove(&cache_key)
                        .expect("completed audio preparation exists");
                    let elapsed = preparation.started_at.elapsed();
                    eprintln!(
                        "crystal-bevy audio prepared {:?} {} worker_ms={}",
                        command.kind,
                        command.audio_id,
                        elapsed.as_millis()
                    );
                    result.map(Some).map_err(anyhow::Error::msg)
                }
                Err(TryRecvError::Empty) => Ok(None),
                Err(TryRecvError::Disconnected) => {
                    self.preparations.remove(&cache_key);
                    anyhow::bail!(
                        "audio preparation worker disconnected for {}",
                        command.audio_id
                    )
                }
            };
        }

        let (sender, receiver) = mpsc::sync_channel(1);
        let thread_command = command.clone();
        std::thread::Builder::new()
            .name(format!("audio-prepare-{}", command.audio_id))
            .spawn(move || {
                let result = decoded_audio_program_source(&thread_command, source)
                    .map_err(|error| format!("{error:#}"));
                let _ = sender.send(result);
            })
            .with_context(|| format!("spawn audio preparation worker for {}", command.audio_id))?;
        eprintln!(
            "crystal-bevy audio queued preparation {:?} {}",
            command.kind, command.audio_id
        );
        self.preparations.insert(
            cache_key,
            NativeAudioPreparation {
                started_at: Instant::now(),
                receiver,
            },
        );
        Ok(None)
    }

    fn stop_music(&mut self) {
        if let Some(sink) = self.music_sink.take() {
            sink.stop();
        }
    }

    fn set_music_volume(&mut self, volume: u8) {
        self.music_volume = f32::from(volume.min(7)) / 7.0;
        if let Some(sink) = self.music_sink.as_ref() {
            sink.set_volume(self.music_volume);
        }
    }

    fn stop_transient(&mut self) {
        for sink in self.transient_sinks.drain(..) {
            sink.stop();
        }
        self.transient_audio_id = None;
        self.transient_deadline = None;
    }

    fn transient_finished(&mut self) -> bool {
        self.transient_sinks.retain(|sink| !sink.empty());
        if one_shot_playback_finished(
            self.transient_sinks.is_empty(),
            self.transient_deadline,
            Instant::now(),
        ) {
            if let Some(audio_id) = self.transient_audio_id.as_deref() {
                eprintln!("crystal-bevy audio completed transient {audio_id}");
            }
            self.stop_transient();
            return true;
        }
        false
    }

    fn play(
        &mut self,
        command: &BevyAudioCommand,
        audio: &CachedPcmAudio,
        sound: Sound,
    ) -> Result<()> {
        use rodio::Source as _;

        let dispatch_started_at = Instant::now();
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
        if matches!(command.kind, ModpackAudioKind::Music) {
            sink.set_volume(self.music_volume);
        }
        let samples = pcm_samples_for_sound_option(&pcm_i16_samples(audio)?, sound);
        let channels = u16::from(audio.format.channels);
        let sample_rate = audio.format.sample_rate_hz;
        let frame_count = samples.len() / usize::from(channels);
        let playback_duration =
            Duration::from_secs_f64(frame_count as f64 / f64::from(sample_rate));
        if let Some((loop_start_sample, loop_end_sample)) = audio.loop_range {
            sink.append(PcmLoopSource::new(
                Arc::clone(&samples),
                channels,
                sample_rate,
                loop_start_sample,
                loop_end_sample,
            )?);
        } else if native_audio_repeats_without_pcm_loop(command) {
            sink.append(
                PcmOneShotSource::new(Arc::clone(&samples), channels, sample_rate)?
                    .repeat_infinite(),
            );
        } else {
            sink.append(PcmOneShotSource::new(
                Arc::clone(&samples),
                channels,
                sample_rate,
            )?);
        }
        sink.play();
        if matches!(command.kind, ModpackAudioKind::Music) {
            eprintln!(
                "crystal-bevy audio started music {} frames={} sample_rate={} duration_ms={} dispatch_ms={}",
                command.audio_id,
                frame_count,
                sample_rate,
                playback_duration.as_millis(),
                dispatch_started_at.elapsed().as_millis()
            );
            self.music_sink = Some(sink);
        } else {
            let started_at = Instant::now();
            self.transient_audio_id = Some(command.audio_id.clone());
            self.transient_deadline = (!command.looped)
                .then(|| started_at.checked_add(playback_duration))
                .flatten();
            eprintln!(
                "crystal-bevy audio started transient {} frames={} sample_rate={} duration_ms={} dispatch_ms={}",
                command.audio_id,
                frame_count,
                sample_rate,
                playback_duration.as_millis(),
                dispatch_started_at.elapsed().as_millis()
            );
            self.transient_sinks.push(sink);
        }
        Ok(())
    }
}

#[cfg(all(not(test), target_arch = "wasm32"))]
struct BrowserAudioBackend {
    context: Option<web_sys::AudioContext>,
    music: Option<(web_sys::AudioBufferSourceNode, web_sys::GainNode)>,
    music_volume: f32,
    transient: Option<(web_sys::AudioBufferSourceNode, f64)>,
}

#[cfg(all(not(test), target_arch = "wasm32"))]
impl BrowserAudioBackend {
    fn new() -> Self {
        Self {
            context: None,
            music: None,
            music_volume: 1.0,
            transient: None,
        }
    }

    fn stop_music(&mut self) {
        if let Some((source, gain)) = self.music.take() {
            let _ = source.stop_with_when(0.0);
            let _ = source.disconnect();
            let _ = gain.disconnect();
        }
    }

    fn set_music_volume(&mut self, volume: u8) {
        self.music_volume = f32::from(volume.min(7)) / 7.0;
        if let Some((_, gain)) = self.music.as_ref() {
            gain.gain().set_value(self.music_volume);
        }
    }

    fn stop_transient(&mut self) {
        if let Some((source, _)) = self.transient.take() {
            let _ = source.stop_with_when(0.0);
            let _ = source.disconnect();
        }
    }

    fn transient_finished(&mut self) -> bool {
        let finished = self
            .transient
            .as_ref()
            .is_none_or(|(_, deadline)| js_sys::Date::now() >= *deadline);
        if finished {
            self.stop_transient();
        }
        finished
    }

    fn play(
        &mut self,
        command: &BevyAudioCommand,
        audio: &CachedPcmAudio,
        sound: Sound,
    ) -> Result<()> {
        if matches!(command.kind, ModpackAudioKind::Music) {
            self.stop_music();
        } else {
            self.stop_transient();
        }
        if self.context.is_none() {
            self.context = Some(
                web_sys::AudioContext::new()
                    .map_err(|error| anyhow::anyhow!("create browser AudioContext: {error:?}"))?,
            );
        }
        let context = self
            .context
            .as_ref()
            .expect("browser audio context initialized");
        context
            .resume()
            .map_err(|error| anyhow::anyhow!("resume browser AudioContext: {error:?}"))?;
        let samples = pcm_samples_for_sound_option(&pcm_i16_samples(audio)?, sound);
        let channels = usize::from(audio.format.channels);
        let frame_count = samples.len() / channels;
        let buffer = context
            .create_buffer(
                u32::from(audio.format.channels),
                u32::try_from(frame_count).context("PCM frame count exceeds WebAudio limit")?,
                audio.format.sample_rate_hz as f32,
            )
            .map_err(|error| anyhow::anyhow!("create browser PCM AudioBuffer: {error:?}"))?;
        for channel in 0..channels {
            let planar = samples
                .iter()
                .skip(channel)
                .step_by(channels)
                .map(|sample| f32::from(*sample) / 32768.0)
                .collect::<Vec<_>>();
            buffer
                .copy_to_channel(&planar, channel as i32)
                .map_err(|error| anyhow::anyhow!("copy PCM channel to WebAudio: {error:?}"))?;
        }
        let source = context
            .create_buffer_source()
            .map_err(|error| anyhow::anyhow!("create browser PCM source: {error:?}"))?;
        source.set_buffer(Some(&buffer));
        if command.looped && audio.loop_range.is_some() {
            source.set_loop(true);
            if let Some((start, end)) = audio.loop_range {
                let rate = f64::from(audio.format.sample_rate_hz);
                source.set_loop_start(start as f64 / rate);
                source.set_loop_end(end as f64 / rate);
            }
        }
        let music_gain = if matches!(command.kind, ModpackAudioKind::Music) {
            let gain = context
                .create_gain()
                .map_err(|error| anyhow::anyhow!("create browser music gain: {error:?}"))?;
            gain.gain().set_value(self.music_volume);
            source
                .connect_with_audio_node(&gain)
                .map_err(|error| anyhow::anyhow!("connect browser music source: {error:?}"))?;
            gain.connect_with_audio_node(&context.destination())
                .map_err(|error| anyhow::anyhow!("connect browser music gain: {error:?}"))?;
            Some(gain)
        } else {
            source
                .connect_with_audio_node(&context.destination())
                .map_err(|error| anyhow::anyhow!("connect browser PCM source: {error:?}"))?;
            None
        };
        source
            .start()
            .map_err(|error| anyhow::anyhow!("start browser PCM source: {error:?}"))?;
        if matches!(command.kind, ModpackAudioKind::Music) {
            self.music = Some((source, music_gain.expect("music gain was created")));
        } else {
            self.transient = Some((source, js_sys::Date::now() + buffer.duration() * 1_000.0));
        }
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn one_shot_playback_finished(
    sink_finished: bool,
    deadline: Option<Instant>,
    now: Instant,
) -> bool {
    sink_finished || deadline.is_some_and(|deadline| now >= deadline)
}

fn low_power_unfocused_game_winit_settings() -> WinitSettings {
    let game_tick = Duration::from_secs_f64(f64::from(GAME_TICK_SECONDS));
    WinitSettings {
        // Focused presentation stays synchronized to requestAnimationFrame.
        // Timer-driven redraws visibly expose the moving LCD surface edge on
        // some browser/display combinations during camera interpolation.
        // Snapshot caching keeps these display-only frames inexpensive.
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::reactive_low_power(game_tick),
    }
}

fn low_latency_game_window(title: String) -> Window {
    Window {
        title,
        resolution: WindowResolution::new(640.0, 576.0),
        // Let the backend select its best synchronized presentation mode
        // instead of forcing the roughly three-frame FIFO queue. Limit the
        // swapchain to one queued frame so input and interpolated transforms
        // reach the display on the next refresh rather than several refreshes
        // later.
        present_mode: PresentMode::AutoVsync,
        desired_maximum_frame_latency: NonZeroU32::new(1),
        // Native play keeps the fixed 4x LCD window. The browser surface is
        // resizable so Bevy receives the actual CSS/fullscreen dimensions and
        // can scale the complete presentation instead of retaining a centered
        // 640x576 surface inside a larger canvas.
        resizable: cfg!(target_arch = "wasm32"),
        #[cfg(target_arch = "wasm32")]
        canvas: Some("#crystal-canvas".to_string()),
        #[cfg(target_arch = "wasm32")]
        fit_canvas_to_parent: true,
        ..default()
    }
}

const fn classic_pixel_art_msaa() -> Msaa {
    // Every classic surface is nearest-sampled pixel art aligned to the LCD
    // coordinate system. Multisampling cannot improve those hard texel edges;
    // it only shades and resolves the complete 640x576 frame four times.
    Msaa::Off
}

fn visible_effect_frames_after_ticks(frames_remaining: u16, elapsed_ticks: u32) -> u16 {
    frames_remaining.saturating_sub(elapsed_ticks.min(u32::from(u16::MAX)) as u16)
}

fn native_audio_repeats_without_pcm_loop(command: &BevyAudioCommand) -> bool {
    // A rendered PCM asset is finite unless the exporter supplied exact loop
    // sample bounds.  Repeating the whole file restarts one-shot score cues
    // such as the Crystal opening and makes them play forever.
    let _ = command;
    false
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
struct PcmLoopSource {
    samples: Arc<[i16]>,
    position: usize,
    loop_start: usize,
    loop_end: usize,
    channels: u16,
    sample_rate: u32,
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
impl PcmLoopSource {
    fn new(
        samples: Arc<[i16]>,
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

#[cfg(all(not(test), not(target_arch = "wasm32")))]
struct PcmOneShotSource {
    samples: Arc<[i16]>,
    position: usize,
    channels: u16,
    sample_rate: u32,
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
impl PcmOneShotSource {
    fn new(samples: Arc<[i16]>, channels: u16, sample_rate: u32) -> Result<Self> {
        if channels == 0 || samples.len() % usize::from(channels) != 0 {
            anyhow::bail!("decoded PCM source has an invalid channel layout");
        }
        Ok(Self {
            samples,
            position: 0,
            channels,
            sample_rate,
        })
    }
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
impl Iterator for PcmOneShotSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.samples.get(self.position).copied();
        self.position += usize::from(sample.is_some());
        sample
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.samples.len().saturating_sub(self.position);
        (remaining, Some(remaining))
    }
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
impl ExactSizeIterator for PcmOneShotSource {}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
impl rodio::Source for PcmOneShotSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.samples.len().saturating_sub(self.position))
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let frames = self.samples.len() / usize::from(self.channels);
        Some(Duration::from_secs_f64(
            frames as f64 / f64::from(self.sample_rate),
        ))
    }
}

#[cfg(all(not(test), not(target_arch = "wasm32")))]
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

#[cfg(all(not(test), not(target_arch = "wasm32")))]
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
    bevy::log::debug!(
        target: "crystal_bevy::script_trace",
        frame = input.frame,
        input_mask = input.input_mask,
        pressed_mask = input.pressed_mask,
        player_moved = input.player_moved,
        state_hash = input.state_checksum.hash(),
        "visible overworld input"
    );
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
    let action = action.into();
    bevy::log::debug!(
        target: "crystal_bevy::script_trace",
        frame = state_checksum.frame(),
        state_hash = state_checksum.hash(),
        action = %action,
        "visible runtime action"
    );
    runtime_shell.last_runtime_action = Some(VisibleRuntimeActionRecord {
        action,
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
    let action = action.into();
    bevy::log::debug!(
        target: "crystal_bevy::script_trace",
        frame = state_checksum.frame(),
        state_hash = state_checksum.hash(),
        action = %action,
        "visible runtime action"
    );
    runtime_shell.last_runtime_action = Some(VisibleRuntimeActionRecord {
        action,
        frame: state_checksum.frame(),
        state_hash: state_checksum.hash(),
    });
}

fn record_visible_runtime_error(runtime_shell: &mut BevyRuntimeShell, error: &anyhow::Error) {
    bevy::log::error!(target: "crystal_bevy::script_trace", error = %error, "visible runtime error");
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

fn queue_visible_link_battle_action(
    runtime_shell: &mut BevyRuntimeShell,
    action: BattleAction,
) -> Result<()> {
    anyhow::ensure!(
        runtime_shell.pending_link_battle_action.is_none(),
        "a Colosseum action is already waiting for the peer"
    );
    let combat = active_battle_combat_state(runtime_shell.shell.session().state())?;
    let turn = u64::from(combat.turn).saturating_add(1);
    let snapshot = runtime_shell.shell.snapshot()?;
    let action_frame = BattleActionFrame::new(
        LOCAL_PLAYER_ID,
        turn,
        action.clone(),
        format!("{:08x}", snapshot.state_checksum.hash()),
    )
    .context("visible runtime produced invalid Colosseum action")?;
    runtime_shell
        .deterministic_battle_actions
        .push_back(action_frame);
    runtime_shell.pending_link_battle_action = Some(action);
    reset_visible_battle_action_cursors(runtime_shell);
    set_shell_action_status(runtime_shell, "WAITING FOR LINK OPPONENT");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn queue_visible_link_battle_replacement(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    anyhow::ensure!(
        runtime_shell.pending_link_battle_replacement.is_none(),
        "a Colosseum replacement is already waiting to be sent"
    );
    let combat = active_battle_combat_state(runtime_shell.shell.session().state())?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let action_frame = BattleActionFrame::new(
        LOCAL_PLAYER_ID,
        u64::from(combat.turn),
        BattleAction::Switch { party_index },
        format!("{:08x}", snapshot.state_checksum.hash()),
    )
    .context("visible runtime produced invalid Colosseum replacement")?;
    runtime_shell
        .deterministic_battle_actions
        .push_back(action_frame);
    runtime_shell.pending_link_battle_replacement = Some(party_index);
    set_shell_action_status(runtime_shell, "WAITING FOR LINK REPLACEMENT");
    mark_runtime_snapshot_dirty(runtime_shell);
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
    presentation_machine: RuntimePresentationPhaseMachine,
    main_menu: RuntimeTitleMainMenuDefinition,
    phase: VisibleTitlePhase,
    frame: u32,
    main_menu_frame: u32,
    scx: u8,
    title_timer: u16,
    entrance_start_scx: u8,
    entrance_scroll_step: u8,
    crystal_oam_target: String,
    crystal_initial_y: u8,
    suicune_frames: Vec<u8>,
    suicune_selector_mask: u8,
    suicune_selector_shift_left: u8,
    suicune_selector_swap_nibbles: bool,
    joypad_mask: u8,
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
    background_binding: Option<VisibleIntroBackgroundBinding>,
    palette_effect: VisibleIntroPaletteEffect,
    finished: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleIntroBackgroundBinding {
    dispatcher_entry: usize,
    tilemap_resource: String,
    attrmap_resource: String,
    palette_resource: String,
    tile_bindings: Vec<VisibleIntroBgTileBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleIntroBgTileBinding {
    tile_id_start: u8,
    tile_id_end: u8,
    target_vram_bank: u8,
    resource: String,
    resource_tile_start: u16,
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
    tile_id: u8,
    oam_attr: u8,
    gfx_name: String,
    gfx_tile_base: u8,
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
    FadeOut,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleGenderSelection {
    definition: RuntimeGenderMenuDefinition,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PendingRememberPasswordPrompt {
    closing_frames: Option<u8>,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingMailInput {
    item_id: String,
    party_index: usize,
    value: String,
    cursor_column: usize,
    cursor_row: usize,
    case: NameInputCase,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleMailRead {
    mail: crate::core::models::pokemon::MailData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingStandardCapture {
    outcome: crate::core::battle::capture::CaptureOutcome,
    scripted_static_wild: Option<VisibleStaticWildOrigin>,
    default_name: String,
    prompt_for_nickname: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleBugContestReplacementPhase {
    AlreadyCaughtText,
    StatsPrompt,
    CaughtText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleBugContestReplacement {
    previous: crate::core::models::Pokemon,
    candidate: crate::core::models::Pokemon,
    phase: VisibleBugContestReplacementPhase,
    scripted_static_wild: Option<VisibleStaticWildOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingGiftPokemonNickname {
    default_name: String,
    location: crate::core::models::CaptureStorageLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingEggHatchNickname {
    party_index: usize,
    default_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleFieldItemPhase {
    FoundText,
    FanfarePause { frames_remaining: u8 },
    SpecialSoundWait,
    AwaitingPrompt,
    PromptEachQueuedPage,
    PocketText,
    BagFullFoundText,
    BagFullText,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleFieldItemPresentation {
    ItemBall,
    HiddenItem { sound_id: Option<String> },
    FruitTree { sound_id: Option<String> },
    VerboseGrant { sound_id: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleFieldItemNotice {
    sound_trigger_text: String,
    pocket_text: String,
    presentation: VisibleFieldItemPresentation,
    phase: VisibleFieldItemPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum PendingScriptPartySelection {
    LinkTrade,
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
struct PendingNpcTradeCommit {
    origin_map_name: String,
    source_script: String,
    command_index: usize,
    trade_id: String,
    party_index: usize,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpecialBoundaryDisplay {
    label: String,
    details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleWaitPlaySfxCompletion {
    FieldNotice(String),
    FieldItemPocketText,
    VerboseItemPrompt,
    SpecialBoundary(SpecialBoundaryDisplay),
    FlashFieldMove,
    WhirlpoolFieldMove,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleUnownPuzzle {
    puzzle_id: String,
    layout: [[u8; 6]; 6],
    holding_piece: Option<u8>,
    cursor_x: usize,
    cursor_y: usize,
    solved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleUnownPrinter {
    selected: u8,
    letters: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBuenaPassword {
    cursor: MenuCursor,
    category_type: String,
    options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBattleTowerChallengeMenu {
    cursor: MenuCursor,
    english: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleBattleTowerRoomMenu {
    cursor: MenuCursor,
    level_groups: Vec<u8>,
    phase: VisibleBattleTowerRoomMenuPhase,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum VisibleBattleTowerRoomMenuPhase {
    PickLevel,
    ConfirmCancel { yes_no_index: usize },
    Rejection { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleSlotMachine {
    phase: VisibleSlotMachinePhase,
    animation: VisibleSlotMachineAnimation,
    yes_no_index: usize,
    bet: u8,
    coins: u16,
    payout: u16,
    offsets: [usize; 3],
    spin_ticks: [u8; 3],
    spinning: [bool; 3],
    next_reel: u8,
    actor: Option<VisibleSlotActor>,
    secondary_actor: Option<VisibleSlotActor>,
    background_y_offset: i8,
    windows: [[String; 3]; 3],
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleSlotMachinePhase {
    Betting,
    Spinning,
    Result,
    PlayAgain,
    RanOut,
    Quitting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleSlotMachineAnimation {
    None,
    Spinning {
        start_delay: u8,
        requested_stop: bool,
    },
    Stopping {
        reel: u8,
        mode: VisibleSlotStopMode,
        target: usize,
        pause: u16,
        steps: u16,
        minimum_steps: u16,
        terminal_delay: u8,
    },
    SpecialPrepare {
        mode: VisibleSlotStopMode,
        target: usize,
        start_offset: usize,
        count: u8,
    },
    SpecialWait {
        mode: VisibleSlotStopMode,
        target: usize,
        count: u8,
        frames_remaining: u8,
    },
    SlowAdvance {
        target: usize,
        steps_remaining: u8,
        frames_until_step: u8,
    },
    Golem {
        target: usize,
        remaining: u8,
        phase: VisibleSlotGolemPhase,
        phase_frame: u8,
    },
    Chansey {
        target: usize,
        remaining_eggs: u8,
        phase: VisibleSlotChanseyPhase,
        phase_frame: u8,
    },
    FlashResult {
        frames_remaining: u8,
    },
    QuitWaitBefore,
    QuitWaitAfter,
    RanOutDelay {
        frames_remaining: u8,
    },
    WaitStart {
        payout: u16,
        result_sound: Option<&'static str>,
    },
    WaitResult {
        payout: u16,
    },
    Payout {
        remaining: u16,
        frames_until_coin: u8,
        delay_counter: u16,
    },
    AwaitResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleSlotStopMode {
    Normal,
    SkipToSeven,
    Slow,
    Golem,
    Chansey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleSlotGolemPhase {
    Init,
    Fall,
    Roll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleSlotChanseyPhase {
    Walk,
    PrepareEgg,
    Egg,
    DropReel,
    CheckMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleSlotActor {
    Golem {
        x: i16,
        y_offset: i16,
        frame: u8,
        frame_tick: u8,
        flip_x: bool,
        flip_y: bool,
    },
    Chansey {
        x: i16,
        frame: u8,
        frame_tick: u8,
        finishing: bool,
    },
    Egg {
        x: i16,
        y_offset: i16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleCardFlipPhase {
    AskPlay,
    ChooseCard,
    PlaceBet,
    Result,
    PlayAgain,
    Shuffled,
    NotEnoughCoins,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleCardFlipAnimation {
    None,
    WaitStake,
    Deal {
        frame: u8,
    },
    Cycle {
        frames_until_toggle: u8,
    },
    SelectFlash {
        frame: u8,
    },
    WaitBeforeReveal,
    WaitReveal,
    WaitResult {
        payout: u16,
    },
    Payout {
        remaining: u16,
        frames_until_coin: u8,
    },
    AwaitResult,
    QuitWaitBefore,
    QuitWaitAfter,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleCardFlip {
    phase: VisibleCardFlipPhase,
    animation: VisibleCardFlipAnimation,
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
    actor_party_index: usize,
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
    player_x: i16,
    player_sprite_visible: bool,
    player_sprite_frame: u8,
    player_sprite_duration: u8,
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

    fn shake_entry_frame(&self) -> u16 {
        if self.ball_id.eq_ignore_ascii_case("MASTER_BALL") {
            156
        } else {
            68
        }
    }

    fn shake_setup_frame(&self) -> u16 {
        // Ordinary branches enter .Shake at 68. Master Ball then waits 24,
        // creates its sparkles at 92, and waits another 64, entering at 156.
        // .Shake then spends 160 frames before the first 48-frame check loop.
        if self.ball_id.eq_ignore_ascii_case("MASTER_BALL") {
            316
        } else {
            228
        }
    }

    fn master_ball_special_frame(&self) -> Option<u16> {
        self.ball_id
            .eq_ignore_ascii_case("MASTER_BALL")
            .then_some(92)
    }

    fn change_dex_sound_frame(&self) -> u16 {
        if self.ball_id.eq_ignore_ascii_case("MASTER_BALL") {
            180
        } else {
            92
        }
    }

    fn bounce_sound_frame(&self) -> u16 {
        if self.ball_id.eq_ignore_ascii_case("MASTER_BALL") {
            212
        } else {
            124
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
        let hidden_frame = self.shake_entry_frame().saturating_add(8);
        if (!self.started && !self.complete) || self.blocked || self.frame < hidden_frame {
            return false;
        }
        self.caught || self.frame + 32 < self.total_frames()
    }

    fn enemy_clip_tiles(&self) -> Option<u8> {
        let shake_entry = self.shake_entry_frame();
        let hidden_frame = shake_entry.saturating_add(8);
        if (!self.started && !self.complete) || self.blocked || self.frame < shake_entry {
            return None;
        }
        if self.frame < hidden_frame {
            return match self.frame - shake_entry {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PendingTrainerIntro {
    origin_map_name: String,
    source_script: String,
    command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct VisibleEarthquake {
    intensity: u16,
    frames_remaining: u16,
    shake_frames_remaining: u16,
    phase: u8,
}

impl VisibleEarthquake {
    fn from_script(parameter: u16, shake_frames: u16, sleep_frames: u16) -> Self {
        Self {
            intensity: 1_u16 << ((parameter >> 6) & 0x3),
            frames_remaining: shake_frames + sleep_frames,
            shake_frames_remaining: shake_frames,
            phase: 0,
        }
    }

    fn screen_shake(parameter: u16) -> Self {
        let frames = crate::core::timing::wrapping_byte_counter_ticks((parameter & 0x3f) as u8);
        Self {
            intensity: 1_u16 << ((parameter >> 6) & 0x3),
            frames_remaining: frames,
            shake_frames_remaining: frames,
            phase: 0,
        }
    }

    fn advance(&mut self, frames: u32) {
        self.frames_remaining = visible_effect_frames_after_ticks(self.frames_remaining, frames);
        self.shake_frames_remaining =
            visible_effect_frames_after_ticks(self.shake_frames_remaining, frames);
        self.phase = self.phase.wrapping_add(frames as u8) % 4;
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePokegearPhoneCall {
    contact_id: String,
    phase: VisiblePokegearPhoneCallPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisiblePokegearPhoneCallPhase {
    NoServicePrompt,
    Ringing { rings_started: u8 },
    Calling,
    FinishDelay { frames_remaining: u8 },
    AwaitHangup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleIncomingPhoneSequence {
    RingTwice {
        frames_remaining: u16,
        second_ring_started: bool,
    },
    HangUp {
        frames_remaining: u16,
    },
}

const VISIBLE_POKEGEAR_RADIO_STATIONS: [(u8, &str); 9] = [
    (16, "PKMNTalkAndPokedexShow"),
    (28, "PokemonMusic"),
    (32, "LuckyChannel"),
    (40, "BuenasPassword"),
    (52, "RuinsOfAlphRadio"),
    (64, "PlacesAndPeople"),
    (72, "LetsAllSing"),
    (78, "PokeFluteRadio"),
    (80, "EvolutionRadio"),
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
    LinkTrade,
    TossItem {
        item_id: String,
        stack_index: usize,
        quantity: u16,
    },
    PutMailInPack(usize),
    NpcTrade(PendingScriptPartySelection),
    ScriptPartyIntro(PendingScriptPartySelection),
    NameRaterRename,
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
        confirm_withdrawal: bool,
    },
    DayCareEggPickup,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisiblePcItemQuantity {
    action: VisiblePlayerPcAction,
    item_id: String,
    stack_index: usize,
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

#[derive(Debug, Clone)]
struct CachedPcmAudio {
    bytes: Arc<[u8]>,
    samples: Arc<[i16]>,
    format: AudioPcmFormat,
    loop_range: Option<(usize, usize)>,
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
                ModpackAudioPlaybackMode::RawPcm => "raw_pcm",
            },
            looped: command.looped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BevyAudioAction {
    StopMusic { audio_id: String },
    Play(BevyAudioCommand),
    FadeMusic { audio_id: String, fade_frames: u16 },
    WaitForSoundEffect,
}

fn enqueue_bevy_audio_command(queue: &mut Vec<BevyAudioCommand>, command: BevyAudioCommand) {
    queue.push(command);
}

fn clear_pending_music_commands(queue: &mut Vec<BevyAudioCommand>) {
    queue.retain(|pending| !matches!(pending.kind, ModpackAudioKind::Music));
}

fn source_ordered_pending_audio(
    pending: Vec<BevyAudioCommand>,
    active_transient_kind: Option<ModpackAudioKind>,
    current_sfx_priority: u8,
    sfx_priorities: &BTreeMap<String, u8>,
) -> Result<Vec<BevyAudioCommand>> {
    let mut accepted = Vec::with_capacity(pending.len());
    let mut transient_active = active_transient_kind.is_some();
    let mut sfx_priority_register = current_sfx_priority;
    for command in pending {
        match command.kind {
            ModpackAudioKind::Music if command.audio_id == "MUSIC_NONE" => anyhow::bail!(
                "MUSIC_NONE reached the playback queue instead of the _InitSound reset boundary"
            ),
            ModpackAudioKind::Music => accepted.push(command),
            ModpackAudioKind::Cry => {
                transient_active = true;
                accepted.push(command);
            }
            ModpackAudioKind::SoundEffect => {
                let priority = sfx_priorities
                    .get(&command.audio_id)
                    .copied()
                    .with_context(|| {
                        format!(
                            "queued sound effect {} has no ASM priority byte",
                            command.audio_id
                        )
                    })?;
                if !transient_active || priority <= sfx_priority_register {
                    transient_active = true;
                    sfx_priority_register = priority;
                    accepted.push(command);
                }
            }
        }
    }
    Ok(accepted)
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

    fn presentation_subframe(&self) -> f32 {
        if self.step_seconds <= 0.0 {
            return 0.0;
        }
        (self.accumulated_seconds / self.step_seconds).clamp(0.0, 1.0) as f32
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

    #[cfg(any(test, feature = "location-tester"))]
    fn fixed(sample: RuntimeRtcSample) -> Self {
        Self::Fixed(sample)
    }

    fn sample(self) -> RuntimeRtcSample {
        match self {
            Self::SystemLocal => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let now = ChronoLocal::now();
                    RuntimeRtcSample {
                        date: GameDate::new(now.year(), now.month() as u8, now.day() as u8),
                        hour: now.hour() as u8,
                        minute: now.minute() as u8,
                        second: now.second() as u8,
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let now = js_sys::Date::new_0();
                    RuntimeRtcSample {
                        date: GameDate::new(
                            now.get_full_year() as i32,
                            (now.get_month() + 1) as u8,
                            now.get_date() as u8,
                        ),
                        hour: now.get_hours() as u8,
                        minute: now.get_minutes() as u8,
                        second: now.get_seconds() as u8,
                    }
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
        self.accumulated_seconds -= self.step_seconds * frames as f32;
        frames.min(MAX_VISIBLE_SEQUENCE_CATCH_UP_FRAMES)
    }
}

#[derive(Resource, Default)]
struct RenderedViewport {
    map_name: Option<String>,
    tile: Option<TilePosition>,
    map_texture: Option<Handle<Image>>,
    map_priority_texture: Option<Handle<Image>>,
    /// Read-only presentation metadata for an optional overworld renderer.
    /// These cells describe the same 20x18 source-tile viewport as
    /// `map_texture`; they never participate in collision or movement.
    #[cfg(any(test, feature = "voxel-view"))]
    visual_tiles: Vec<crystal_render_api::VisualTile>,
    /// Feature-gated terrain surface extending beyond the Game Boy viewport.
    /// It is consumed only by the optional voxel renderer and never displayed
    /// or consulted by the faithful 2D path.
    #[cfg(feature = "voxel-view")]
    visual_world_texture: Option<Handle<Image>>,
    #[cfg(feature = "voxel-view")]
    visual_world_grid_size: UVec2,
    /// Whether the retained optional-renderer data was built for an active
    /// 2.5D view. A feature-enabled binary normally runs in classic mode and
    /// must not pay to compose the 84x82 terrain source on every camera step.
    #[cfg(feature = "voxel-view")]
    visual_world_enabled: bool,
    viewport_origin: Option<(i16, i16)>,
    /// The viewport origin shown immediately before a committed walking step.
    /// Retaining it lets the renderer scroll the replacement texture over the
    /// same LCD frames as the player sprite instead of snapping the camera.
    walk_viewport_origin: Option<(i16, i16)>,
    map_visual_key: Option<u64>,
    /// Exact ordered identity of the source images in the retained base,
    /// priority, and optional visual-world composites. Ambient animation
    /// schedules can tick even when none of their animated tiles are visible;
    /// this key prevents rebuilding identical multi-megabyte textures.
    tile_frame_key: Option<u64>,
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
    /// True while at least one retained object transform is interpolating
    /// from its pre-command tile. The semantic snapshot already contains the
    /// destination, so its ordinary world key cannot identify the landing.
    object_motion_active: bool,
    #[cfg(test)]
    terrain_scan_count: u64,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteAnimRuntimeBundle {
    oam_sets: BTreeMap<String, SpriteAnimOamSet>,
    framesets: BTreeMap<String, SpriteAnimFrameset>,
    objects: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteAnimOamSet {
    name: String,
    tile_offset: i16,
    pieces: Vec<SpriteAnimOamPiece>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteAnimOamPiece {
    x: i16,
    y: i16,
    tile: i16,
    attributes: u8,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteAnimFrameset {
    name: String,
    steps: Vec<SpriteAnimFrameStep>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpriteAnimFrameStep {
    oam_set: Option<String>,
    duration: u16,
    attr_flags: u8,
    command: String,
}

struct MagnetTrainBase {
    rgba: Vec<u8>,
    palette_indices: Vec<u8>,
}

struct MagnetTrainPlayerFrames {
    standing: Vec<u8>,
    walking: Vec<u8>,
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
    magnet_train_base_cache: HashMap<String, MagnetTrainBase>,
    magnet_train_player_cache: HashMap<(bool, String), MagnetTrainPlayerFrames>,
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
    battle_battler_bgp_cache: HashMap<(AssetId<Image>, u8), SpriteFrame>,
    fishing_rod_cache: Option<[SpriteFrame; 3]>,
    fishing_rod_error: Option<String>,
    fishing_player_cache: HashMap<String, SpriteFrame>,
    fishing_player_errors: HashMap<String, String>,
    egg_hatch_tile_cache: Option<[SpriteFrame; 2]>,
    egg_hatch_tile_error: Option<String>,
    battle_substitute_cache: Option<[SpriteFrame; 2]>,
    battle_substitute_error: Option<String>,
    battle_minimize_cache: Option<SpriteFrame>,
    battle_minimize_error: Option<String>,
    title_cache: HashMap<TitleArtKey, SpriteFrame>,
    title_errors: HashMap<TitleArtKey, String>,
    town_map_cache: HashMap<(String, u8), SpriteFrame>,
    town_map_errors: HashMap<(String, u8), String>,
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
    intro_sprite_bundle_cache: Option<SpriteAnimRuntimeBundle>,
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
    mail_entry_cache: HashMap<MailEntryArtKey, SpriteFrame>,
    mail_entry_errors: HashMap<MailEntryArtKey, String>,
    mail_entry_cache_order: VecDeque<MailEntryArtKey>,
    mail_read_cache: HashMap<VisibleMailRead, SpriteFrame>,
    mail_read_errors: HashMap<VisibleMailRead, String>,
    mail_read_cache_order: VecDeque<VisibleMailRead>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum VisibleEggHatchPhase {
    HuhText,
    EggHold,
    Wobble,
    Shell,
    Reveal,
    HatchText,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct VisibleEggHatch {
    party_index: usize,
    species_id: String,
    phase: VisibleEggHatchPhase,
    frame: u16,
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
struct MailEntryArtKey {
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
        let frames = take_visible_sequence_frames(&mut self.accumulated_seconds, delta_seconds);
        if frames == 0 {
            return;
        }
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

fn take_visible_sequence_frames(accumulated_seconds: &mut f32, delta_seconds: f32) -> u16 {
    *accumulated_seconds += delta_seconds.max(0.0);
    let frames = (*accumulated_seconds / GAME_TICK_SECONDS).floor() as u16;
    *accumulated_seconds -= GAME_TICK_SECONDS * f32::from(frames);
    frames.min(MAX_VISIBLE_SEQUENCE_CATCH_UP_FRAMES as u16)
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
        // LCD. Reuse the one presenter, but replace its frame as well as its
        // layer. Updating only the transform leaves furniture Town Map and
        // other later full-screen surfaces showing the preceding screen.
        commands.entity(entity).insert((
            frame.handle.clone(),
            Sprite {
                custom_size: Some(Vec2::new(PLAYFIELD_WIDTH, PLAYFIELD_HEIGHT)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, z),
        ));
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

#[derive(Clone)]
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
    actors: image::RgbaImage,
    palettes: Vec<Palette>,
}

struct CardFlipRenderSources {
    base: Vec<u8>,
    background_tiles: image::RgbaImage,
    face_tiles: image::RgbaImage,
    object_tiles: image::RgbaImage,
    font: image::RgbaImage,
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

#[derive(Component)]
struct PlayerMarker;

#[derive(Component)]
struct MultiplayerGhost {
    user_id: String,
    display_tile: Vec2,
}

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
    source_id: Arc<str>,
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

#[derive(Component)]
struct YesNoPromptMarker;

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
    let multiplayer_config = config.multiplayer.clone();
    #[cfg(feature = "voxel-view")]
    let voxel_view_enabled = config.voxel_view_enabled.unwrap_or(false);
    let window_title = config
        .window_title
        .clone()
        .unwrap_or_else(|| "Pokemon Crystal Rust".to_string());
    #[cfg(feature = "location-tester")]
    let render_test_screenshot = config.render_test_screenshot.clone();
    #[cfg(feature = "location-tester")]
    let native_rtc_source = config
        .render_test_hour
        .map(|hour| {
            NativeRtcSource::fixed(RuntimeRtcSample {
                date: GameDate::new(2000, 1, 1),
                hour,
                minute: 0,
                second: 0,
            })
        })
        .unwrap_or_else(NativeRtcSource::system_local);
    #[cfg(not(feature = "location-tester"))]
    let native_rtc_source = NativeRtcSource::system_local();
    let runtime_shell = initialize_bevy_runtime_shell(asset_root, runtime, start, config)?;
    let multiplayer_runtime = multiplayer_config
        .map(|config| MultiplayerRuntime::new(&runtime_shell, config))
        .transpose()?;

    let mut app = App::new();
    #[cfg(all(not(test), not(target_arch = "wasm32")))]
    app.insert_non_send_resource(NativeAudioBackend::new());
    #[cfg(all(not(test), target_arch = "wasm32"))]
    app.insert_non_send_resource(BrowserAudioBackend::new());
    if let Some(multiplayer_runtime) = multiplayer_runtime {
        app.insert_non_send_resource(multiplayer_runtime);
    }
    app.insert_resource(ClearColor(Color::srgb(0.05, 0.07, 0.06)))
        .insert_resource(classic_pixel_art_msaa())
        .insert_resource(runtime_shell)
        // Oak fades, script emotes, and WaitSFX are Game Boy frame loops.
        // Keep driving them while the host window is unfocused; reactive
        // low-power mode can be suspended by macOS and strand those loops.
        .insert_resource(low_power_unfocused_game_winit_settings())
        .insert_resource(native_rtc_source)
        .insert_resource(RuntimeTickTimer::new(f64::from(GAME_TICK_SECONDS)))
        .insert_resource(VisibleSequenceTickClock::realtime())
        .insert_resource(RenderedViewport::default())
        .insert_resource(RenderedTilesetArt::default())
        .insert_resource(HudMode::Status)
        .add_plugins(DefaultPlugins.build().set(WindowPlugin {
            primary_window: Some(low_latency_game_window(window_title)),
            ..default()
        }))
        .add_systems(Startup, setup_shell_view)
        .add_systems(Update, poll_multiplayer.before(apply_keyboard_input))
        .add_systems(
            Update,
            release_input_on_focus_loss.before(apply_keyboard_input),
        )
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
        .add_systems(
            Update,
            sync_multiplayer_ghosts
                .after(render_playfield)
                .after(sync_visible_player_sprite),
        )
        .add_systems(Update, render_poison_flash_overlay.after(render_playfield))
        .add_systems(Update, refresh_status_text.after(render_playfield))
        .add_systems(Update, refresh_dialog_text.after(refresh_status_text))
        .add_systems(Update, refresh_battle_text.after(refresh_dialog_text))
        .add_systems(Update, refresh_shell_panels.after(refresh_battle_text));
    #[cfg(feature = "voxel-view")]
    app.add_plugins(crystal_render_api::VisualWorldRenderPlugin)
        .add_systems(
            Update,
            publish_visual_world_frame
                .after(sync_visible_player_sprite)
                .after(sync_multiplayer_ghosts)
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
        allow_f3_toggle: !cfg!(target_arch = "wasm32"),
    })
    .add_plugins(crystal_voxel_view::VoxelViewPlugin)
    .add_systems(
        Startup,
        configure_voxel_composite_camera.after(setup_shell_view),
    )
    .add_systems(
        Update,
        sync_manual_world_view_layers.after(crystal_render_api::WorldRenderSet::RenderSync),
    );
    #[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
    app.add_systems(
        Update,
        trace_native_overworld_movement
            .after(crystal_render_api::WorldRenderSet::RenderSync)
            .after(sync_manual_world_view_layers),
    );
    #[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
    if std::env::var_os("CRYSTAL_RENDER_TRACE").is_some()
        && let Some(render_app) = app.get_sub_app_mut(RenderApp)
    {
        render_app
            .init_resource::<NativeRenderTraceState>()
            .add_systems(
                Render,
                trace_native_render_start.before(RenderSet::ManageViews),
            )
            .add_systems(
                Render,
                trace_native_render_managed
                    .after(RenderSet::ManageViews)
                    .before(RenderSet::Queue),
            )
            .add_systems(
                Render,
                trace_native_render_queued
                    .after(RenderSet::PhaseSort)
                    .before(RenderSet::Prepare),
            )
            .add_systems(
                Render,
                trace_native_render_prepared
                    .after(RenderSet::Prepare)
                    .before(RenderSet::Render),
            )
            .add_systems(
                Render,
                trace_native_render_presented
                    .after(RenderSet::Render)
                    .before(RenderSet::Cleanup),
            );
    }
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

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
#[derive(Default)]
struct NativeMovementTraceState {
    enabled: Option<bool>,
    frame: u64,
    last_at: Option<Instant>,
    last_player_position: Option<Vec3>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
#[derive(Resource, Default)]
struct NativeRenderTraceState {
    frame: u64,
    started_at: Option<Instant>,
    managed_at: Option<Instant>,
    queued_at: Option<Instant>,
    prepared_at: Option<Instant>,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
fn trace_native_render_start(mut trace: ResMut<NativeRenderTraceState>) {
    trace.frame = trace.frame.saturating_add(1);
    trace.started_at = Some(Instant::now());
    trace.managed_at = None;
    trace.queued_at = None;
    trace.prepared_at = None;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
fn trace_native_render_managed(mut trace: ResMut<NativeRenderTraceState>) {
    trace.managed_at = Some(Instant::now());
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
fn trace_native_render_queued(mut trace: ResMut<NativeRenderTraceState>) {
    trace.queued_at = Some(Instant::now());
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
fn trace_native_render_prepared(mut trace: ResMut<NativeRenderTraceState>) {
    trace.prepared_at = Some(Instant::now());
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
fn trace_native_render_presented(trace: Res<NativeRenderTraceState>) {
    let (Some(started_at), Some(managed_at), Some(queued_at), Some(prepared_at)) = (
        trace.started_at,
        trace.managed_at,
        trace.queued_at,
        trace.prepared_at,
    ) else {
        return;
    };
    let presented_at = Instant::now();
    eprintln!(
        "crystal-bevy render_trace frame={} manage_us={} queue_us={} prepare_us={} render_present_us={} total_us={}",
        trace.frame,
        managed_at.duration_since(started_at).as_micros(),
        queued_at.duration_since(managed_at).as_micros(),
        prepared_at.duration_since(queued_at).as_micros(),
        presented_at.duration_since(prepared_at).as_micros(),
        presented_at.duration_since(started_at).as_micros(),
    );
}

#[cfg(all(not(target_arch = "wasm32"), feature = "voxel-view"))]
fn trace_native_overworld_movement(
    runtime_shell: Res<BevyRuntimeShell>,
    tick_timer: Res<RuntimeTickTimer>,
    visual_frame: Res<crystal_render_api::VisualWorldFrame>,
    players: Query<&Transform, With<PlayerMarker>>,
    mut trace: Local<NativeMovementTraceState>,
) {
    let enabled = *trace
        .enabled
        .get_or_insert_with(|| std::env::var_os("CRYSTAL_MOVEMENT_TRACE").is_some());
    if !enabled {
        return;
    }

    let now = Instant::now();
    let host_us = trace
        .last_at
        .map_or(0, |last_at| now.duration_since(last_at).as_micros());
    trace.last_at = Some(now);
    trace.frame = trace.frame.saturating_add(1);

    let player_position = players
        .get_single()
        .ok()
        .map(|transform| transform.translation);
    let delta = player_position
        .zip(trace.last_player_position)
        .map_or(Vec3::ZERO, |(current, previous)| current - previous);
    trace.last_player_position = player_position;

    let snapshot = runtime_shell.shell.snapshot().ok();
    let tile = snapshot.as_ref().map(|snapshot| snapshot.overworld.tile);
    let facing = snapshot.as_ref().map(|snapshot| snapshot.overworld.facing);
    let moving = runtime_shell.player_walk_frame_ticks > 0;
    let voxel_enabled = runtime_shell.shell.snapshot().is_ok() && visual_frame.active;
    if moving || delta != Vec3::ZERO || host_us >= 25_000 {
        let position = player_position.unwrap_or(Vec3::ZERO);
        eprintln!(
            "crystal-bevy movement_trace frame={} host_us={} voxel_active={} moving={} walk_remaining={} subframe={:.4} tile={:?} facing={:?} player_x={:.3} player_y={:.3} delta_x={:.3} delta_y={:.3} terrain_revision={}",
            trace.frame,
            host_us,
            voxel_enabled,
            moving,
            runtime_shell.player_walk_frame_ticks,
            tick_timer.presentation_subframe(),
            tile,
            facing,
            position.x,
            position.y,
            delta.x,
            delta.y,
            visual_frame.terrain_revision,
        );
    }
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
fn sync_manual_world_view_layers(
    settings: Res<crystal_voxel_view::VoxelViewSettings>,
    mut commands: Commands,
    classic_world: Query<
        Entity,
        Or<(
            With<PlayfieldTile>,
            With<PlayerMarker>,
            With<MultiplayerGhost>,
            With<ObjectMarker>,
            With<LedgeShadowMarker>,
            With<GrassRustleMarker>,
        )>,
    >,
    added_classic_world: Query<
        Entity,
        Or<(
            Added<PlayfieldTile>,
            Added<PlayerMarker>,
            Added<MultiplayerGhost>,
            Added<ObjectMarker>,
            Added<LedgeShadowMarker>,
            Added<GrassRustleMarker>,
        )>,
    >,
) {
    let entities = if settings.is_changed() {
        classic_world.iter().collect::<Vec<_>>()
    } else {
        added_classic_world.iter().collect::<Vec<_>>()
    };
    for entity in entities {
        if settings.enabled {
            // Manual 2.5D selection parks the classic overworld on a layer
            // that no camera renders. Renderer readiness must never expose it;
            // only the presentation setting can return these entities to the
            // main layer. UI, dialogue, battle, and fades remain on layer 0.
            commands
                .entity(entity)
                .insert(bevy::render::view::RenderLayers::layer(
                    crystal_voxel_view::HIDDEN_CLASSIC_WORLD_RENDER_LAYER,
                ));
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
    // Give the extracted visual frame, voxel mesh, and GPU render target
    // several presented frames to settle. Capturing the first frame in which
    // status flips active can still read an earlier swapchain frame.
    let presentation_settled = voxel_status.active_frames >= 30
        || voxel_status.inactive_reason.as_deref() == Some("disabled");
    if !capture.requested && capture.frame >= 90 && presentation_settled {
        println!(
            "2.5D renderer status: {}{}",
            if voxel_status.active {
                "active"
            } else {
                "inactive"
            },
            voxel_status
                .inactive_reason
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

fn add_visible_shell_smoke_party(
    runtime_shell: &mut BevyRuntimeShell,
    party: &[VisibleShellSmokePokemon],
) -> Result<()> {
    let trainer = runtime_shell.shell.snapshot()?.trainer;
    for pokemon in party {
        runtime_shell.shell.add_party_pokemon(
            &pokemon.species_id,
            pokemon.level,
            pokemon.held_item_id.clone(),
            None,
            &trainer.player_name,
            trainer.player_id,
            Dv::from_non_hp(10, 10, 10, 10),
        )?;
    }
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
    add_visible_shell_smoke_party(&mut runtime_shell, party)?;
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
    let party_entries = visible_party_menu_entries(&party_snapshot, &runtime_shell)?;
    close_visible_party_menu(&mut runtime_shell);
    press_visible_start_button(&mut runtime_shell)?;
    select_visible_start_menu_option_exact(&mut runtime_shell, StartMenuOption::Pack)?;
    select_visible_start_menu_option(&mut runtime_shell)?;
    let pack_snapshot = runtime_shell.shell.snapshot()?;
    let pack_entries = visible_field_pack_entries(&pack_snapshot, &runtime_shell)?;
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
            .position(|option| option.dispatch_target == "MainMenu_Continue")
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
            .position(|option| option.dispatch_target == "MainMenu_NewGame")
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
            selected_visible_title_menu_option(&runtime_shell, title)
                .and_then(|option| visible_title_menu_selection_id(&option).map(str::to_string))
        })
        .context("visible shell title menu closed before selection")??;
    select_visible_title_menu_option(&mut runtime_shell)?;
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let new_game_identity_pending = runtime_shell.pending_time_set.is_some()
        || runtime_shell.pending_oak_intro.is_some()
        || runtime_shell.pending_gender_selection.is_some()
        || runtime_shell.pending_name_choice.is_some()
        || runtime_shell.pending_name_input.is_some()
        || runtime_shell.pending_mail_input.is_some();
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
        .position(|option| option.dispatch_target == "MainMenu_NewGame")
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
    let selected = visible_title_menu_selection_id(&selected_visible_title_menu_option(
        &runtime_shell,
        &title,
    )?)?
    .to_string();
    select_visible_title_menu_option(&mut runtime_shell)?;
    complete_visible_smoke_gender_if_needed(&mut runtime_shell)?;
    // A normal Bevy update samples the host RTC before the clock-setting UI
    // accepts input. This direct smoke driver intentionally bypasses the App,
    // so supply the same native source rather than granting a test-only clock
    // mutation path.
    runtime_shell.latest_rtc_sample = Some(NativeRtcSource::system_local().sample());
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
    add_visible_shell_smoke_party(&mut runtime_shell, party)?;
    settle_visible_shell_smoke_until_idle(&mut runtime_shell)?;
    open_visible_party_menu(&mut runtime_shell)?;
    let initial_snapshot = runtime_shell.shell.snapshot()?;
    let lead_before = initial_snapshot
        .party
        .slots
        .first()
        .map(|slot| slot.pokemon.species.id.clone())
        .context("visible shell party smoke missing lead Pokemon")?;
    let initial_entries = visible_party_menu_entries(&initial_snapshot, &runtime_shell)?;

    open_visible_party_action_menu(&mut runtime_shell)?;
    let action_snapshot = runtime_shell.shell.snapshot()?;
    let action_entries = visible_party_menu_entries(&action_snapshot, &runtime_shell)?;
    execute_visible_party_action(&mut runtime_shell)?;
    let summary_snapshot = runtime_shell.shell.snapshot()?;
    let summary_entries = visible_party_menu_entries(&summary_snapshot, &runtime_shell)?;
    close_visible_party_summary(&mut runtime_shell);

    open_visible_party_action_menu(&mut runtime_shell)?;
    move_visible_party_action_cursor(&mut runtime_shell, 1)?;
    execute_visible_party_action(&mut runtime_shell)?;
    let switch_snapshot = runtime_shell.shell.snapshot()?;
    let switch_entries = visible_party_menu_entries(&switch_snapshot, &runtime_shell)?;
    confirm_visible_party_switch_target(&mut runtime_shell)?;
    let final_snapshot = runtime_shell.shell.snapshot()?;
    let lead_after = final_snapshot
        .party
        .slots
        .first()
        .map(|slot| slot.pokemon.species.id.clone())
        .context("visible shell party smoke missing final lead Pokemon")?;
    let final_entries = visible_party_menu_entries(&final_snapshot, &runtime_shell)?;
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
    add_visible_shell_smoke_party(&mut runtime_shell, party)?;
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
    add_visible_shell_smoke_party(&mut runtime_shell, party)?;
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
            // apply_visible_shell_smoke_frame already dispatches and counts
            // the authoritative frame interaction. Dispatching it again here
            // restarted scripts at command zero (notably Cyndaquil after
            // reanchormap).
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
                if std::env::var_os("CRYSTAL_INPUT_TRACE").is_some() {
                    let snapshot = runtime_shell.shell.presentation_snapshot()?;
                    eprintln!(
                        "input_trace button=A map={} tile={:?} cursor={:?} text={:?} wait={:?} yes_no={} window={} picture={:?} audio={} non_audio={} current_interaction={:?}",
                        snapshot.overworld.map_name,
                        snapshot.overworld.tile,
                        runtime_shell.active_script_cursor,
                        snapshot.ui.text.as_ref().map(|text| text.label.as_str()),
                        snapshot
                            .ui
                            .pending_text_wait
                            .as_ref()
                            .map(|wait| wait.command.as_str()),
                        snapshot.ui.pending_yes_no.is_some(),
                        snapshot.ui.window_open,
                        snapshot.ui.active_pokemon_picture,
                        snapshot.script_events.audio_events.len(),
                        has_visible_pending_non_audio_script_events(&snapshot),
                        runtime_shell
                            .shell
                            .current_overworld_interaction_checked()?,
                    );
                }
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
    if frame
        .as_ref()
        .and_then(|frame| frame.interaction.as_ref())
        .is_some()
    {
        // Match the real Bevy input transaction: ordinary A interactions are
        // discovered by the authoritative overworld tick, then their compiled
        // script is dispatched after that tick. The smoke driver must not
        // route them through modal UI ownership merely to make tests pass.
        interaction = true;
        execute_last_interaction_script(runtime_shell)?;
    }
    Ok(VisibleShellSmokeFrameOutcome { frame, interaction })
}

fn settle_visible_shell_smoke_until_idle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MAX_IDLE_SETTLE_STEPS: usize = 1024;
    for _ in 0..MAX_IDLE_SETTLE_STEPS {
        if advance_visible_smoke_walk_warp_phase(runtime_shell)? {
            continue;
        }
        if runtime_shell.player_walk_frame_ticks > 0
            || runtime_shell.object_walk_frame_ticks > 0
            || !runtime_shell.object_walk_frame_ticks_by_id.is_empty()
        {
            advance_visible_walk_timers(runtime_shell, 1);
            continue;
        }
        // Smoke execution has no Bevy `Time` system, but ASM delays, emotes,
        // and earthquakes still consume frames. Advance their presentation
        // clocks exactly one frame per settle iteration so script commands
        // following `showemote`/`pause` can become reachable.
        if let Some(frames) = runtime_shell.visible_script_delay_frames.as_mut() {
            *frames = frames.saturating_sub(1);
        }
        if runtime_shell.pending_linked_friend_wait {
            break;
        }
        if let Some(frames) = runtime_shell.visible_internal_special_delay_frames.as_mut() {
            *frames = frames.saturating_sub(1);
            if *frames == 0 {
                runtime_shell.visible_internal_special_delay_frames = None;
                continue_visible_script_after_prompt(runtime_shell)?;
            }
            continue;
        }
        if runtime_shell.visible_special_text_pause_frames.is_some() {
            advance_visible_special_text_pause(runtime_shell)?;
            continue;
        }
        if let Some(prompt) = runtime_shell.pending_remember_password.as_ref() {
            if prompt.closing_frames.is_some() {
                advance_visible_remember_password_prompt(runtime_shell)?;
            } else {
                begin_closing_visible_remember_password_prompt(runtime_shell, true)?;
            }
            continue;
        }
        if runtime_shell.bill_pc_move_save.is_some() {
            advance_visible_bill_pc_move_save(runtime_shell, 1)?;
            continue;
        }
        if runtime_shell.pc_release_sequence.is_some() {
            advance_visible_pc_release_sequence(runtime_shell, 1)?;
            continue;
        }
        if runtime_shell.pc_transfer_sequence.is_some() {
            if runtime_shell
                .pc_transfer_sequence
                .as_ref()
                .is_some_and(|active| active.phase == VisiblePcTransferPhase::RefusalWaitSfx)
            {
                let pending_audio = std::mem::take(&mut runtime_shell.pending_audio);
                let transient_audio_playing = runtime_shell.transient_audio_playing;
                runtime_shell.transient_audio_playing = false;
                let advance = advance_visible_pc_transfer_sequence(runtime_shell, 1);
                runtime_shell.pending_audio = pending_audio;
                runtime_shell.transient_audio_playing = transient_audio_playing;
                advance?;
            } else {
                advance_visible_pc_transfer_sequence(runtime_shell, 1)?;
            }
            continue;
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
            earthquake.advance(1);
        }
        // The real Bevy update loop reveals field text autonomously between
        // joypad presses. Smoke sessions must advance that same presentation
        // clock or every A press is discarded against an eternally unrevealed
        // item notice or script page.
        if let Some(notice) = runtime_shell
            .field_notice
            .clone()
            .or_else(|| runtime_shell.pc_notice.clone())
        {
            if tick_visible_field_text_reveal(runtime_shell, false)? {
                mark_runtime_presentation_dirty(runtime_shell);
            }
            if !visible_field_text_reveal_is_complete_for_text(runtime_shell, &notice) {
                continue;
            }
            if runtime_shell.visible_wait_sfx_boundary {
                // Smoke sessions have no audio backend to consume queued
                // effects. Let this control-flow fence observe completion
                // while retaining the commands for the audio audit.
                let mut retained_audio = std::mem::take(&mut runtime_shell.pending_audio);
                let retained_transient_audio = runtime_shell.transient_audio_playing;
                runtime_shell.transient_audio_playing = false;
                let presentation = runtime_shell.shell.presentation_snapshot()?;
                let advanced =
                    advance_visible_wait_sfx_boundary(runtime_shell, &presentation, false)?;
                retained_audio.append(&mut runtime_shell.pending_audio);
                runtime_shell.pending_audio = retained_audio;
                runtime_shell.transient_audio_playing = retained_transient_audio;
                if advanced {
                    continue;
                }
            }
        }
        if runtime_shell.pending_trainer_sight.is_some() {
            advance_visible_trainer_sight_cutscene(runtime_shell)?;
            if runtime_shell.pending_trainer_sight.is_some() {
                continue;
            }
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        // An active battle is a player-owned boundary. Smoke settling may
        // present the script up to StartBattle, but must not resume the
        // retained post-battle cursor until a battle result is supplied.
        if snapshot.battle.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_gender_selection.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_time_set.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_oak_intro.is_some() {
            return Ok(());
        }
        if runtime_shell.pending_name_input.is_some() || runtime_shell.pending_mail_input.is_some()
        {
            return Ok(());
        }
        if runtime_shell.pending_name_choice.is_some() {
            if runtime_shell.pending_gift_pokemon_nickname.is_some() {
                finish_visible_gift_pokemon_nickname(runtime_shell, None)?;
                continue;
            }
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
        if runtime_shell.pending_scene_script.is_some() {
            take_visible_pending_scene_script(runtime_shell)?;
            continue;
        }
        if has_visible_elevator_prompt(&snapshot, runtime_shell) {
            select_visible_elevator_floor(runtime_shell)?;
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
        if snapshot.script_events.map_reentry_script.is_some() {
            take_visible_map_reentry_script(runtime_shell)?;
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

fn advance_visible_smoke_walk_warp_phase(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    let Some(phase) = runtime_shell.visible_walk_warp_phase else {
        return Ok(false);
    };
    match phase {
        VisibleWalkWarpPhase::FadeOut => {
            reset_visible_navigation_state(runtime_shell);
            queue_visible_current_music(runtime_shell)?;
            runtime_shell.visible_walk_warp_phase = Some(VisibleWalkWarpPhase::FadeIn);
        }
        VisibleWalkWarpPhase::FadeIn => {
            runtime_shell.visible_walk_warp_phase = None;
            runtime_shell.screen_fade = None;
            let pitfall = runtime_shell
                .shell
                .last_frame()
                .and_then(|frame| frame.warp.as_ref())
                .is_some_and(|warp| {
                    matches!(
                        warp.trigger.permission,
                        crate::core::world::collision::permissions::PIT
                            | crate::core::world::collision::permissions::PIT_68
                    )
                });
            if pitfall {
                begin_visible_pitfall_landing(runtime_shell)?;
            } else {
                settle_visible_overworld_arrival(runtime_shell, "walk_warp")?;
            }
        }
        VisibleWalkWarpPhase::ScriptFadeIn => {
            runtime_shell.visible_walk_warp_phase = None;
            runtime_shell.screen_fade = None;
            settle_visible_overworld_arrival(runtime_shell, "script_warp")?;
        }
        VisibleWalkWarpPhase::MapReloadFadeIn => {
            runtime_shell.visible_walk_warp_phase = None;
            runtime_shell.screen_fade = None;
            if let Some(cursor) = runtime_shell.map_reload_return_cursor.take() {
                arm_visible_active_script_cursor(
                    runtime_shell,
                    &cursor.source_script,
                    cursor.command_index,
                );
            }
            continue_visible_script_after_prompt(runtime_shell)?;
        }
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(true)
}

fn initialize_bevy_runtime_shell(
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    start: BevyShellStart,
    config: BevyShellConfig,
) -> Result<BevyRuntimeShell> {
    #[cfg(target_arch = "wasm32")]
    runtime.install_browser_runtime_files()?;
    #[cfg(any(test, feature = "location-tester"))]
    let runtime_tile_start = matches!(&start, BevyShellStart::NewGameAtRuntimeTile { .. });
    let asset_root = if cfg!(not(target_arch = "wasm32")) && runtime.has_runtime_files() {
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
    let title_parameters = matches!(&start, BevyShellStart::Title { .. })
        .then(|| {
            RuntimeTitlePresentationParameters::from_program(runtime.title_presentation_program())
        })
        .transpose()?;
    let title_main_menu = matches!(&start, BevyShellStart::Title { .. })
        .then(|| RuntimeTitleMainMenuDefinition::from_program(runtime.title_presentation_program()))
        .transpose()?;
    let title_menu = match &start {
        BevyShellStart::Title {
            spawn_identifier,
            save_path,
        } => {
            let parameters =
                title_parameters.context("title presentation parameters are missing")?;
            let main_menu = title_main_menu.context("title main menu definition is missing")?;
            let mut presentation_machine = RuntimePresentationPhaseMachine::new(
                runtime.title_presentation_program(),
                "start_title_screen",
                "title_screen",
            )?;
            presentation_machine
                .memory
                .insert("hSCX".to_string(), u16::from(parameters.entrance_start_scx));
            presentation_machine
                .memory
                .insert("wJumptableIndex".to_string(), 0);
            presentation_machine
                .memory
                .insert("wTitleScreenTimer".to_string(), 0);
            presentation_machine
                .memory
                .insert("hClockResetTrigger".to_string(), 0);
            presentation_machine
                .memory
                .insert("wMusicFade".to_string(), 0);
            presentation_machine.memory.insert(
                parameters.crystal_oam_target.clone(),
                u16::from(parameters.crystal_initial_y),
            );
            Some(TitleMenu {
                spawn_identifier: *spawn_identifier,
                save_path: save_path.clone(),
                cursor: MenuCursor {
                    surface_id: "title".to_string(),
                    option_index: 0,
                },
                presentation_machine,
                main_menu,
                phase: VisibleTitlePhase::Entrance,
                frame: 0,
                main_menu_frame: 0,
                scx: parameters.entrance_start_scx,
                title_timer: 0,
                entrance_start_scx: parameters.entrance_start_scx,
                entrance_scroll_step: parameters.entrance_scroll_step,
                crystal_oam_target: parameters.crystal_oam_target,
                crystal_initial_y: parameters.crystal_initial_y,
                suicune_frames: parameters.suicune_frames,
                suicune_selector_mask: parameters.suicune_selector_mask,
                suicune_selector_shift_left: parameters.suicune_selector_shift_left,
                suicune_selector_swap_nibbles: parameters.suicune_selector_swap_nibbles,
                joypad_mask: 0,
                clock_reset_trigger: false,
            })
        }
        BevyShellStart::LoadSave { .. } => None,
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGame { .. } => None,
        #[cfg(any(test, feature = "location-tester"))]
        BevyShellStart::NewGameAtRuntimeTile { .. } => None,
    };
    let mut intro_screen =
        matches!(&start, BevyShellStart::Title { .. }).then(VisibleIntroScreen::new);
    if let Some(intro) = intro_screen.as_mut() {
        apply_visible_intro_background_binding(
            intro,
            &runtime.data().runtime_title_screen.program,
        )?;
    }
    let intro_sprite_bundle = intro_screen
        .as_ref()
        .map(|_| load_intro_sprite_anim_bundle(runtime.data().sprite_anim_bundle.as_str()))
        .transpose()?;
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
    if let Some(save_path) = title_menu
        .as_ref()
        .and_then(|title| title.save_path.as_ref())
        && let Ok(saved_state) = shell.runtime().load_save(save_path)
    {
        // GameInit's TryLoadSaveData restores wOptions before the intro. The
        // Lucky-ID routine later reads its two SRAM fields directly during
        // ResetWRAM, so retain those alongside the title's live hRandom state.
        let title_state = shell.session_mut().state_mut();
        title_state.options = saved_state.options;
        title_state.lucky_number_day = saved_state.lucky_number_day;
        title_state.lucky_id_number = saved_state.lucky_id_number;
    }
    #[cfg(feature = "location-tester")]
    if let Some(hour) = config.render_test_hour {
        anyhow::ensure!(hour < 24, "render-test hour {hour} is outside 0..24");
        shell.update_clock_from_datetime(GameDate::new(2000, 1, 1), hour, 0, 0)?;
    }
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
        intro_sprite_bundle,
        title_menu,
        new_game_pre_overworld: false,
        visible_continue_screen: None,
        credits_screen: None,
        last_error: None,
        last_action_status: None,
        last_audio_events: Vec::new(),
        pending_audio: Vec::new(),
        audio_source_cache: HashMap::new(),
        pending_music_stop: false,
        pending_full_audio_reset: false,
        transient_audio_playing: false,
        active_transient_kind: None,
        current_sfx_priority: 0,
        active_music: None,
        faded_music: None,
        music_volume: 7,
        music_fade: None,
        last_battle_cry_key: None,
        pending_battle_cries_after_messages: VecDeque::new(),
        battle_enemy_send_out_pending: false,
        battle_player_send_out_pending: false,
        battle_enemy_hp_at_player_send_out: None,
        pending_battle_scenes_after_message: VecDeque::new(),
        pending_plain_battle_map_reload: false,
        last_overworld_input: None,
        overworld_interaction_consumed_a: false,
        field_text_consumed_a: false,
        field_text_consumed_b: false,
        player_walk_from: None,
        player_walk_frame_ticks: 0,
        player_walk_total_ticks: WALK_FRAME_HOLD_TICKS,
        player_walk_stride: false,
        player_walk_mirror_stride: false,
        player_walk_direction_phases: HashMap::new(),
        object_walk_frame_ticks: 0,
        object_walk_total_ticks: WALK_FRAME_HOLD_TICKS,
        object_walk_frame_ticks_by_id: BTreeMap::new(),
        object_walk_total_ticks_by_id: BTreeMap::new(),
        object_walk_stride: false,
        object_walk_from: BTreeMap::new(),
        pending_follower_walks: VecDeque::new(),
        follower_visible_tile_overrides: BTreeMap::new(),
        object_walk_phases: BTreeMap::new(),
        object_walk_direction_phases: HashMap::new(),
        trainer_walk_from: None,
        pending_overworld_step_boundary: None,
        pending_overworld_warp_scene: None,
        visible_script_movement: None,
        visible_script_movement_scene: None,
        visible_player_sprite_y_offset: 0,
        overworld_direction_repeat_ticks: 0,
        overworld_held_direction: None,
        overworld_held_directions: VecDeque::new(),
        overworld_buffered_direction: None,
        pending_overworld_direction_press: None,
        pending_ui_button_presses: VecDeque::new(),
        ui_held_direction: None,
        ui_direction_repeat_ticks: 0,
        recent_overworld_inputs: VecDeque::new(),
        deterministic_session_start,
        deterministic_session_checkpoint,
        deterministic_input_frames: VecDeque::new(),
        deterministic_battle_actions: VecDeque::new(),
        pending_link_battle_action: None,
        pending_link_battle_replacement: None,
        deterministic_menu_results: VecDeque::new(),
        last_runtime_action: None,
        quick_save_path: config.quick_save_path,
        active_script_cursor: None,
        map_reload_return_cursor: None,
        pending_scene_script: None,
        deferred_script_warp_arrival_scripts: false,
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
        yes_no_cursor: None,
        pending_phone_prompt: None,
        pending_remember_password: None,
        pending_day_of_week: None,
        pending_trainer_sight: None,
        pending_trainer_intro: None,
        visible_map_name_sign: None,
        pending_delete_save: None,
        pending_clock_reset: None,
        pending_mystery_gift: None,
        pending_time_set: None,
        pending_oak_intro: None,
        pending_gender_selection: None,
        screen_fade: None,
        visible_blackout_phase: None,
        pending_poison_blackout: false,
        visible_walk_warp_phase: None,
        field_text_reveal: None,
        rendered_field_text_identity: None,
        dialogue_log_identity: None,
        dialogue_log_events: VecDeque::new(),
        movement_log_events: VecDeque::new(),
        input_log_events: VecDeque::new(),
        selected_player_gender: None,
        pending_name_input: None,
        pending_mail_input: None,
        pending_mail_read: None,
        pending_name_choice: None,
        pending_standard_capture: None,
        visible_bug_contest_replacement: None,
        pending_gift_pokemon_nickname: None,
        pending_gift_pokemon_pc_notice: false,
        pending_egg_hatch_nickname: None,
        visible_field_item_notice: None,
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
        pending_link_trade_party_slot: None,
        pending_link_trade_confirmation: None,
        pending_link_trade_save: false,
        pending_link_room_selection: None,
        pending_linked_friend_wait: false,
        pending_link_room_session: false,
        pending_npc_trade_commit: None,
        pending_photo_studio_commit: None,
        kurt_apricorn_cursor: None,
        kurt_apricorn_quantity: None,
        buena_prize_cursor: None,
        visible_buena_password: None,
        visible_battle_tower_challenge_menu: None,
        visible_battle_tower_room_menu: None,
        visible_unown_puzzle: None,
        visible_unown_printer: None,
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
        visible_egg_hatch: None,
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
        pokegear_standalone_map: false,
        pokegear_cursor: 0,
        pokegear_phone_cursor: 0,
        pokegear_phone_status: None,
        pokegear_phone_call: None,
        incoming_phone_sequence: None,
        pokegear_page: PokegearPage::Clock,
        pokegear_radio_station: None,
        pokegear_radio_segment: 0,
        pokegear_radio_tuning_knob: initial_snapshot.progression.radio_tuning_knob,
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
        visible_wait_sfx_boundary: false,
        pending_wait_play_sfx: VecDeque::new(),
        wait_play_sfx_completion: None,
        special_boundary_queue: VecDeque::new(),
        visible_special_text_pause_frames: None,
        visible_internal_special_delay_frames: None,
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
        pack_item_switch_origin: None,
        last_field_pack_pocket: FieldPackPocket::Items,
        field_pack_cursor_positions: [0; 4],
        field_pack_action_cursor: None,
        field_pack_target_mode: None,
        tmhm_teach_prompt_cursor: None,
        pending_tmhm_text_stage: None,
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
        decoration_menu: None,
        mailbox_cursor: None,
        mailbox_action_cursor: None,
        mailbox_attach_index: None,
        pc_confirmation: None,
        bill_pc_session_open: false,
        bill_pc_action_cursor: None,
        bill_pc_box_cursor: None,
        bill_pc_box_action_cursor: None,
        bill_pc_move_open: false,
        bill_pc_move_party_open: false,
        bill_pc_move_source: None,
        bill_pc_move_save: None,
        bill_pc_pokemon_action_cursor: None,
        bill_pc_box_summary: None,
        pending_pc_release: None,
        pc_release_sequence: None,
        pc_transfer_sequence: None,
        pc_notice: None,
        field_notice: None,
        field_notice_queue: VecDeque::new(),
        pending_item_notification: None,
        field_notice_scene: None,
        pending_field_travel_arrival: false,
        pending_field_travel_delay_frames: None,
        visible_field_travel_animation: None,
        pending_field_notice_sound: None,
        pending_field_notice_cry: None,
        pending_field_battle_entry: false,
        pending_field_notice_effect_frames: None,
        visible_cut_animation: None,
        pending_whirlpool_sound_wait: false,
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
include!("bevy_shell/multiplayer.rs");
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
#[cfg(any(test, feature = "voxel-view"))]
include!("bevy_shell/render_mod.rs");
include!("bevy_shell/overworld_rendering.rs");
include!("bevy_shell/start_menu.rs");
include!("bevy_shell/bitmap_font.rs");
include!("bevy_shell/graphics_assets.rs");
include!("bevy_shell/field_pack.rs");

#[cfg(test)]
#[path = "bevy_shell/tests.rs"]
mod tests;
use crystal_assets::{DecorationActionOutcome, DecorationCategory, DecorationSide};
