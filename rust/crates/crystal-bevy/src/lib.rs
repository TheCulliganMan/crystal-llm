use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use anyhow::{Context, Result};
use crystal_assets::modpack::{
    CompiledGamePack, CompiledGamePackIdentity, GameDataSet, LoadedCompiledGamePack,
    ModpackAudioKind, ModpackAudioManifest, ModpackAudioManifestEntry, ModpackAudioPlaybackEntry,
    ModpackAudioPlaybackPlan, ModpackAudioSource,
};
use crystal_assets::{
    ActiveBattleCommandOutcome, AssetRoot, BlackoutRecoveryOutcome, OverworldInputFrame,
    PartyRecoveryOutcome, PokemonCryMetadata, RuntimeBadgeCommand, RuntimeBadgeRegion,
    RuntimeBagItemDeltaCommand, RuntimeBattleEnemyActionCommand, RuntimeBattleEscapeCommand,
    RuntimeBattleItemCommand, RuntimeBattleTowerActionCommand, RuntimeBattleTowerBattleCommand,
    RuntimeBattleTowerMobileFlag, RuntimeBattleTowerOpponentCommand, RuntimeBattleTurnCommand,
    RuntimeBillsGrandfatherCommand, RuntimeBuenaPasswordCommand, RuntimeBuenaPrizeCommand,
    RuntimeBugContestAction, RuntimeBugContestCommand, RuntimeCableClubGenderCommand,
    RuntimeCableClubRequest, RuntimeCaptureCompletionCommand, RuntimeClockUpdateCommand,
    RuntimeCurrencyAccount, RuntimeCurrencyDeltaCommand, RuntimeDayCareAction,
    RuntimeDayCareCaretaker, RuntimeDayCareCommand, RuntimeDividerTrace,
    RuntimeElevatorFloorSelectionCommand, RuntimeFieldBlockMoveCommand, RuntimeFieldPartyCommand,
    RuntimeFishingSwarmCommand, RuntimeFlyCommand, RuntimeGameCornerCommand,
    RuntimeGameCornerService, RuntimeGameLogicPauseCommand, RuntimeGameTimerAdvanceCommand,
    RuntimeGameTimerCountingCommand, RuntimeGameTimerOutcome, RuntimeGiftPokemonCommand,
    RuntimeGiveDratiniCommand, RuntimeGraphicsSpecial, RuntimeHappinessServiceCommand,
    RuntimeHappinessServiceRoutine, RuntimeHeadbuttFieldEncounterCommand, RuntimeHeldItemCommand,
    RuntimeItemCommand, RuntimeKurtApricornCommand, RuntimeLinkBattleRecordCommand,
    RuntimeLinkBattleResult, RuntimeLinkFriendReadyCommand, RuntimeLinkRoomSelectionCommand,
    RuntimeLinkRoomSpecial, RuntimeLinkTimeoutCommand, RuntimeMagikarpLengthCommand,
    RuntimeMailboxPartyCommand, RuntimeMailboxSlotCommand, RuntimeManualClockCommand,
    RuntimeMapRadioCommand, RuntimeMobileHandshakeCommand, RuntimeMobileSelectThreeMonsCommand,
    RuntimeMoveDeletionCommand, RuntimeMoveLearnReplacementCommand, RuntimeMoveTutorCommand,
    RuntimeMutationCommand, RuntimeMutationOutcome, RuntimeMutationResult,
    RuntimeMysteryGiftAction, RuntimeNameRivalCommand, RuntimeOddEggCommand, RuntimeOptionsCommand,
    RuntimeOverworldInputCommand, RuntimePartyCheckCommand, RuntimePartyCheckSpecial,
    RuntimePartyHpTransferCommand, RuntimePartyHpTransferOutcome, RuntimePartyItemCommand,
    RuntimePartyMoveItemCommand, RuntimePartyMoveSwapCommand, RuntimePartyNicknameCommand,
    RuntimePartyPokemonCommand, RuntimePartyRecoverySetupCommand, RuntimePartyRecoverySetupOutcome,
    RuntimePartySlotCommand, RuntimePartySwapCommand, RuntimePcBagItemCheckCommand,
    RuntimePcBoxCommand, RuntimePcDepositCommand, RuntimePcItemCommand, RuntimePcMoveCommand,
    RuntimePcReleaseCommand, RuntimePcWithdrawCommand, RuntimePendingScriptRequest,
    RuntimePendingScriptRequestCommand, RuntimePendingScriptRequestKind,
    RuntimePendingYesNoResolutionCommand, RuntimePhoneCallerCommand, RuntimePhoneRandomSpecial,
    RuntimePlayerGenderCommand, RuntimePlayerPaletteCommand, RuntimePokedexCommand,
    RuntimeRandomSpecialRoutineCommand, RuntimeRegisteredKeyItemCommand,
    RuntimeRegisteredKeyItemOutcome, RuntimeRememberPasswordCommand,
    RuntimeRockMonEncounterCommand, RuntimeScriptCommandRef, RuntimeScriptEventDrainCommand,
    RuntimeScriptEventDrainResult, RuntimeScriptEventQueue, RuntimeScriptRuntimeFlag,
    RuntimeScriptRuntimeFlagCommand, RuntimeScriptRuntimeFlagValue,
    RuntimeScriptRuntimeMemoryEntry, RuntimeScriptRuntimeMemoryEntryCommand,
    RuntimeScriptRuntimeMemoryEntryRemoved, RuntimeScriptRuntimeMemoryValue,
    RuntimeScriptRuntimeMemoryValueCommand, RuntimeScriptRuntimeMemoryValueTaken,
    RuntimeScriptRuntimeQueue, RuntimeScriptRuntimeQueueDrainCommand,
    RuntimeScriptRuntimeQueueDrainResult, RuntimeScriptRuntimeRecordQueue,
    RuntimeScriptRuntimeRecordQueueDrainCommand, RuntimeScriptRuntimeRecordQueueDrainResult,
    RuntimeScriptedWildBattleCompletionCommand, RuntimeScriptedWildBattleStartCommand,
    RuntimeScriptedWildBattleTerminal, RuntimeShopTransactionCommand, RuntimeShuckieAction,
    RuntimeShuckieCommand, RuntimeSpawnPoint, RuntimeSpecialCryCommand,
    RuntimeStaticWildBattleOrigin, RuntimeStoryGateSpecial, RuntimeSweetScentFieldMoveCommand,
    RuntimeTmHmCommand, RuntimeTrainerBattleCompletionCommand, RuntimeTrainerIdentityCommand,
    RuntimeVerticalMenuOpenCommand, RuntimeVerticalMenuSelectionCommand, TilesetDefinition,
    decode_runtime_mutation_command_frame, decode_runtime_mutation_command_payload,
    runtime_mutation_command_frame,
    runtime_mutation_result_frame as assets_runtime_mutation_result_frame,
    runtime_special_routine_requires_divider_trace,
    runtime_special_routine_requires_legacy_seed_boundary, validate_compiled_audio_payload,
    validate_compiled_runtime_files,
};
use crystal_audio::{AudioKind, AudioPcmFormat, AudioProgram, AudioProgramSource};
use crystal_core::battle::capture::{CaptureCompletion, CaptureOutcome, StoredCapture};
use crystal_core::battle::capture::{CaptureRules, CaptureWobbleProbability};
use crystal_core::battle::damage::{TypeCategories, TypeEffectivenessTable, WeatherModifiers};
use crystal_core::battle::start::{
    StaticWildBattleStart, TrainerBattleStartStatus, WildBattleStart,
};
use crystal_core::battle::stats::BattleStatMultiplierTables;
use crystal_core::battle::turn::{BattleAction, BattleTurnOutcome, MovePriorityTable};
use crystal_core::input::{
    B_PAD_A, B_PAD_B, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_SELECT, B_PAD_START, B_PAD_UP,
    GameButton, JoypadState,
};
use crystal_core::models::{
    Dv, FrontpicAnimProgram, ITEM_POCKET_TM_HM, Item, LearnedMove, Move, PcBox, PokegearLandmark,
    PokegearLandmarksPayload, Pokemon, PokemonSpecies, RuntimePokedexEntry, Stat, Trainer,
};
use crystal_core::multiplayer::{
    DeterministicInputJournal, DeterministicInputJournalFrame, DeterministicReplayBundle,
    LinkHello, LinkMessage, LinkSessionIdentity, LockstepFrame, MenuChoiceFrame,
    MenuChoiceResultFrame, PlayerId, PlayerIdentity, RuntimeCommandFrame,
    RuntimeCommandResultFrame, SaveCheckpointFrame, SaveResumeReplayBundle,
    SessionRuntimeCommandFrame, SessionRuntimeCommandResultFrame, SessionSaveCheckpointFrame,
    StateChecksum, StateChecksumFrame, game_state_checksum, validate_link_session_identity,
};
use crystal_core::random::{Random, RecordingDivider, RuntimeDividerSource};
use crystal_core::save::{
    SaveGameSummary, SaveModpackIdentity, SaveSlotSummary, list_save_game_summaries_for_modpack,
    read_save_game_for_modpack, read_save_game_summary_for_modpack, write_save_game_for_modpack,
};
use crystal_core::state::{
    Badges, BattleMemory, GameState, ItemUseRuntimeEvent, LinkSerialConnectionStatus, Options,
    OverworldMemory, SavedTrainerMetadata, ScriptControlRuntimeEvent, ScriptEndState,
    ScriptGraphicsRuntimeEvent, ScriptLocation, ScriptMapLoadRequest, ScriptMapRefreshRequest,
    ScriptMapRuntimeEvent, ScriptMoneyRuntimeEvent, ScriptMusicFade, ScriptReturnFrame,
    ScriptRuntimeAsmDirective, ScriptRuntimeDecorationDescription, ScriptRuntimeDelay,
    ScriptRuntimeEarthquake, ScriptRuntimeEffect, ScriptRuntimeElevatorFloor, ScriptRuntimeEmote,
    ScriptRuntimeNumericBufferWrite, ScriptRuntimeQueuedCommand, ScriptRuntimeStoneTableEntry,
    ScriptRuntimeVariableWrite, ScriptScreenFade, ScriptShopRequest, ScriptShopRuntimeEvent,
    ScriptTextRuntimeEvent, ScriptTextWait, ScriptWarpRequest, ScriptYesNoPrompt,
    is_engine_flag_name, saved_decoration_description_command_payload, saved_delay_command_payload,
    saved_earthquake_command_payload, saved_emote_command_payload, saved_map_load_command_payload,
    saved_map_refresh_command_payload, saved_music_fade_command_payload,
    saved_numeric_buffer_write_command_payload, saved_queued_command_args,
    saved_shop_event_command_payload, saved_shop_request_command_payload,
    saved_stone_table_entry_command_payload, saved_variable_write_command_payload,
    validate_saved_trainer_metadata,
};
use crystal_core::systems::battle_escape::{BattleEscapeAttempt, BattleEscapeRules};
use crystal_core::systems::battle_items::{
    BattleItemEffectPlan, BattleItemOutcome, PartyItemOutcome, active_battle_item_effect_plan,
    battle_pp_item_effect_plan, party_special_item_effect_plan, party_wide_item_effect_plan,
};
use crystal_core::systems::battle_rewards::{
    BattleRewardOutcome, BattleRewardRules, PendingMoveLearnResolution,
};
use crystal_core::systems::economy::ScriptEconomyOutcome;
use crystal_core::systems::evolution::EvolutionReport;
use crystal_core::systems::field_items::{
    FieldItemPickupOutcome, ItemfinderHiddenItem as CoreItemfinderHiddenItem,
};
use crystal_core::systems::field_moves::{
    FieldEscapeItemRule, FieldItemRule, FieldMoveBadgeRequirement, FieldMoveBlockOutcome,
    FieldMoveBlockRule, FieldMoveCatalog, FieldMoveFlagOutcome, FieldMoveFlagRule,
    FieldMoveMoveRule, FieldMoveReplacement, FieldMoveRule, FieldMoveTravelOutcome,
    FieldMoveTravelRule,
};
use crystal_core::systems::gift_pokemon::{GiftPokemonOutcome, GiftPokemonScript};
use crystal_core::systems::item_use::{ItemUseContext, ItemUseOutcome};
use crystal_core::systems::map_context::{SpawnMemoryUpdate, commit_overworld_snapshot};
use crystal_core::systems::phone::{ScriptPhoneInputs, ScriptPhoneOutcome};
use crystal_core::systems::script_audio::ScriptAudioCue;
use crystal_core::systems::script_blocks::ScriptBlockChangeOutcome;
use crystal_core::systems::script_control::ScriptControlAction;
use crystal_core::systems::script_flags::{ScriptFlagCheckOutcome, ScriptFlagMutationOutcome};
use crystal_core::systems::script_items::{
    ScriptItemCheckOutcome, ScriptItemGrantOutcome, ScriptItemTakeOutcome,
};
use crystal_core::systems::script_objects::{ScriptMovementOutcome, ScriptObjectMutationOutcome};
use crystal_core::systems::script_runtime::{
    ScriptRuntimeInputs, ScriptRuntimeOutcome, commit_interaction_script_dispatch,
    parse_menu_coord_token,
};
use crystal_core::systems::script_scenes::ScriptSceneOutcome;
use crystal_core::systems::script_swarms::ScriptSwarmOutcome;
use crystal_core::systems::script_text::{ScriptMenuDefinition, ScriptTextAction, ScriptTextBody};
use crystal_core::systems::script_variables::ScriptVariableOutcome;
use crystal_core::systems::script_warps::ScriptMapAction;
use crystal_core::systems::shop::{ScriptShopOutcome, ShopResult};
use crystal_core::systems::special_routines::{
    SpecialRoutineOutcome, saved_special_battle_type_builtin_routines,
};
use crystal_core::systems::step_events::StepEventResult;
use crystal_core::systems::time::{ClockTime, GameDate};
use crystal_core::systems::tmhm::TmHmLearnOutcome;
use crystal_core::world::encounters::{
    EncounterSlotTables, EncounterSurface, FieldEncounterData, FieldEncounterRoll, TimeOfDay,
    WildEncounterData,
};
use crystal_core::world::fishing::FishingSession;
use crystal_core::world::map::{Direction, TilePosition};
use crystal_core::world::movement::{LedgeJumpOutcome, MovementMode, StepOutcome};
use crystal_core::world::session::{
    ConnectionTransition, CoordEventTrigger, OverworldInteraction, OverworldInteractionTarget,
    OverworldSession, OverworldSnapshot, WarpTransition, WildEncounterRoll,
};

pub use crystal_assets as assets;
pub use crystal_audio as audio;
pub use crystal_core as core;
pub use crystal_net as net;

#[cfg(target_arch = "wasm32")]
static BROWSER_RUNTIME_FILES: OnceLock<BTreeMap<String, Vec<u8>>> = OnceLock::new();

pub(crate) fn read_runtime_asset(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
    let path = path.as_ref();
    #[cfg(target_arch = "wasm32")]
    if let Some(files) = BROWSER_RUNTIME_FILES.get() {
        let key = browser_runtime_asset_key(path);
        return files.get(&key).cloned().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("embedded runtime asset {key}"),
            )
        });
    }
    std::fs::read(path)
}

pub(crate) fn runtime_asset_exists(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();
    #[cfg(target_arch = "wasm32")]
    if let Some(files) = BROWSER_RUNTIME_FILES.get() {
        return files.contains_key(&browser_runtime_asset_key(path));
    }
    path.is_file()
}

#[cfg(target_arch = "wasm32")]
fn browser_runtime_asset_key(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    text.split("apps/web/assets/")
        .nth(1)
        .map(str::to_owned)
        .or_else(|| {
            text.split("vendor/")
                .nth(1)
                .map(|value| format!("vendor/{value}"))
        })
        .unwrap_or_else(|| text.trim_start_matches("./").to_owned())
}

pub(crate) fn read_runtime_asset_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
    String::from_utf8(read_runtime_asset(path)?)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

pub(crate) fn open_runtime_image(
    path: impl AsRef<Path>,
) -> image::ImageResult<image::DynamicImage> {
    let path = path.as_ref();
    #[cfg(target_arch = "wasm32")]
    {
        let bytes = read_runtime_asset(path).map_err(image::ImageError::IoError)?;
        return image::load_from_memory(&bytes);
    }
    #[cfg(not(target_arch = "wasm32"))]
    image::open(path)
}

#[cfg(feature = "bevy-shell")]
pub mod bevy_shell;
#[cfg(feature = "bevy-shell")]
pub use bevy_shell::{
    BevyShellConfig, BevyShellStart, VisibleShellBattleSmoke, VisibleShellBattleSmokeRef,
    VisibleShellOverworldSmoke, VisibleShellPartySmoke, VisibleShellSmokeItem,
    VisibleShellSmokePokemon, VisibleShellStartMenuSmoke, VisibleShellTitleNameInputSmoke,
    VisibleShellTitleSmoke, VisibleShellTrainerBattleSmoke, run_bevy_shell,
    smoke_visible_shell_overworld, smoke_visible_shell_party, smoke_visible_shell_start_menu,
    smoke_visible_shell_title, smoke_visible_shell_title_name_input,
    smoke_visible_shell_trainer_battle, smoke_visible_shell_wild_battle,
};

const BATTLE_MOVE_SLOTS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameViewport {
    pub width: u32,
    pub height: u32,
    pub scale: u32,
}

impl Default for GameViewport {
    fn default() -> Self {
        Self {
            width: 160,
            height: 144,
            scale: 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAudioCatalog {
    manifest: ModpackAudioManifest,
    playback: ModpackAudioPlaybackPlan,
    music: BTreeMap<String, AudioProgram>,
    sound_effects: BTreeMap<String, AudioProgram>,
    cries: BTreeMap<String, AudioProgram>,
}

impl RuntimeAudioCatalog {
    pub fn is_empty(&self) -> bool {
        self.music.is_empty() && self.sound_effects.is_empty() && self.cries.is_empty()
    }

    pub fn manifest(&self) -> &ModpackAudioManifest {
        &self.manifest
    }

    pub fn playback(&self) -> &ModpackAudioPlaybackPlan {
        &self.playback
    }

    pub fn music(&self) -> &BTreeMap<String, AudioProgram> {
        &self.music
    }

    pub fn sound_effects(&self) -> &BTreeMap<String, AudioProgram> {
        &self.sound_effects
    }

    pub fn cries(&self) -> &BTreeMap<String, AudioProgram> {
        &self.cries
    }

    pub fn music_count(&self) -> usize {
        self.music.len()
    }

    pub fn sound_effect_count(&self) -> usize {
        self.sound_effects.len()
    }

    pub fn cry_count(&self) -> usize {
        self.cries.len()
    }

    pub fn contains_music(&self, id: &str) -> bool {
        self.music.contains_key(id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrystalRuntime {
    modpack: SaveModpackIdentity,
    pack_identity: CompiledGamePackIdentity,
    data: GameDataSet,
    runtime_files: BTreeMap<String, Vec<u8>>,
    audio: RuntimeAudioCatalog,
    viewport: GameViewport,
    /// Immutable pack map catalogs shared by presentation snapshots. Runtime
    /// snapshots clone only the active/overridden map instead of deep-cloning
    /// every map, scene, event, object, and block table each movement frame.
    map_catalog: Vec<Arc<RuntimeMapCatalogSnapshot>>,
    catalog_cache: Arc<OnceLock<RuntimeStaticCatalogCache>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeStaticCatalogCache {
    audio: Arc<RuntimeAudioCatalogSnapshot>,
    items: Arc<Vec<RuntimeItemCatalogSnapshot>>,
    item_effect_plans: Arc<Vec<RuntimeItemEffectPlanKey>>,
    moves: Arc<Vec<RuntimeMoveCatalogSnapshot>>,
    pokemon: Arc<Vec<RuntimePokemonCatalogSnapshot>>,
    trainers: Arc<Vec<RuntimeTrainerCatalogSnapshot>>,
    spawn_points: Arc<Vec<RuntimeSpawnPoint>>,
    tilesets: Arc<Vec<RuntimeTilesetCatalogSnapshot>>,
    encounters: Arc<RuntimeEncounterCatalogSnapshot>,
    battle_rules: Arc<RuntimeBattleRuleCatalogSnapshot>,
    world_rules: Arc<RuntimeWorldRuleCatalogSnapshot>,
    presentation: Arc<RuntimePresentationCatalogSnapshot>,
    special: Arc<RuntimeSpecialCatalogSnapshot>,
    story: Arc<RuntimeStoryCatalogSnapshot>,
    playability: Arc<crystal_assets::PlayabilityRules>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBootSummary {
    pub modpack_id: String,
    pub modpack_hash: String,
    pub pack_content_hash: String,
    pub pokemon_species: usize,
    pub moves: usize,
    pub maps: usize,
    pub items: usize,
    pub wild_encounter_tables: usize,
    pub music_tracks: usize,
    pub sound_effects: usize,
    pub cries: usize,
    pub viewport: GameViewport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOverworldSession {
    state: GameState,
    overworld: OverworldSession,
    joypad: JoypadState,
    divider: RuntimeDividerSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeGameShell {
    asset_root: AssetRoot,
    runtime: CrystalRuntime,
    session: RuntimeOverworldSession,
    last_frame: Option<RuntimeOverworldFrame>,
    linked_menu_results: Vec<MenuChoiceResultFrame>,
    runtime_command_sequence: u64,
    runtime_commands: Vec<RuntimeCommandFrame>,
    runtime_results: Vec<RuntimeCommandResultFrame>,
    /// Retained command frames are useful for deterministic link/replay
    /// diagnostics, but serializing every idle input frame is not gameplay.
    /// Bevy disables this for its live shell; the public runtime keeps the
    /// historical default enabled for replay callers.
    retain_runtime_journal: bool,
}

struct RecordedRuntimeMutation {
    command: RuntimeMutationCommand,
    state: GameState,
    overworld: OverworldSession,
    outcome: RuntimeMutationOutcome,
    divider_after: Option<RuntimeDividerSource>,
}

const RUNTIME_LOCAL_PLAYER_ID: PlayerId = 1;

fn reject_unexpected_gift_pokemon_inputs(
    source_script: &str,
    command_index: usize,
    command: &str,
    inputs: &ScriptRuntimeInputs,
) -> Result<()> {
    if inputs.gift_original_trainer_name.is_some()
        || inputs.gift_original_trainer_id.is_some()
        || inputs.gift_dvs.is_some()
        || inputs.gift_rng_seed_after.is_some()
        || inputs.gift_nickname_accepted.is_some()
        || inputs.gift_nickname.is_some()
    {
        anyhow::bail!(
            "compiled script command {}:{} '{}' must not declare gift Pokemon input fields",
            source_script,
            command_index,
            command
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShellSnapshot {
    pub boot: RuntimeBootSummary,
    pub overworld: OverworldSnapshot,
    pub overworld_player_hidden: bool,
    pub visible_objects: Vec<crystal_core::map::ObjectEvent>,
    pub visible_object_slots: Vec<usize>,
    pub visible_object_runtime_tiles: BTreeMap<String, TilePosition>,
    pub visible_object_facings: BTreeMap<String, Direction>,
    pub state_checksum: StateChecksum,
    /// Checksum of the render-relevant game state with the monotonically
    /// advancing frame counter removed.  The authoritative checksum must
    /// change every frame; using it to decide whether to rebuild the viewport
    /// made idle gameplay recreate every tile and NPC at 60 Hz.
    pub visual_state_hash: u32,
    pub phase: RuntimeShellPhase,
    pub trainer: RuntimeTrainerSnapshot,
    pub progression: RuntimeProgressionSnapshot,
    pub roaming_pokemon: [crystal_core::state::RoamingPokemonState; 3],
    pub day_care: crystal_core::state::DayCareState,
    pub bug_contest: crystal_core::state::BugContestState,
    pub magikarp_record: crystal_core::state::MagikarpRecordState,
    pub buenas_password: crystal_core::state::BuenasPasswordState,
    pub mystery_gift: RuntimeMysteryGiftSnapshot,
    pub link_session: RuntimeLinkSessionSnapshot,
    pub battle_tower: crystal_core::state::BattleTowerState,
    pub mobile_link: crystal_core::state::MobileLinkState,
    pub audio: RuntimeShellAudioState,
    pub audio_catalog: Arc<RuntimeAudioCatalogSnapshot>,
    pub menu: Option<RuntimeMenuSnapshot>,
    pub ui: RuntimeUiSnapshot,
    pub battle: Option<RuntimeBattleSnapshot>,
    pub pending_move_learn: Option<RuntimePendingMoveLearnSnapshot>,
    pub party: RuntimePartySnapshot,
    pub storage: RuntimeStorageSnapshot,
    pub mailbox: Vec<crystal_core::state::MailboxMail>,
    pub bag: RuntimeBagSnapshot,
    pub items: Arc<Vec<RuntimeItemCatalogSnapshot>>,
    pub item_effect_plans: Arc<Vec<RuntimeItemEffectPlanKey>>,
    pub moves: Arc<Vec<RuntimeMoveCatalogSnapshot>>,
    pub pokemon: Arc<Vec<RuntimePokemonCatalogSnapshot>>,
    pub trainers: Arc<Vec<RuntimeTrainerCatalogSnapshot>>,
    pub maps: Vec<Arc<RuntimeMapCatalogSnapshot>>,
    pub spawn_points: Arc<Vec<RuntimeSpawnPoint>>,
    pub tilesets: Arc<Vec<RuntimeTilesetCatalogSnapshot>>,
    pub encounters: Arc<RuntimeEncounterCatalogSnapshot>,
    pub battle_rules: Arc<RuntimeBattleRuleCatalogSnapshot>,
    pub world_rules: Arc<RuntimeWorldRuleCatalogSnapshot>,
    pub presentation: Arc<RuntimePresentationCatalogSnapshot>,
    pub special: Arc<RuntimeSpecialCatalogSnapshot>,
    pub story: Arc<RuntimeStoryCatalogSnapshot>,
    pub playability: Arc<crystal_assets::PlayabilityRules>,
    pub script_events: RuntimeScriptEventsSnapshot,
    pub pending_shop: Option<ScriptShopRequest>,
    pub linked_menu_results: Vec<MenuChoiceResultFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMysteryGiftSnapshot {
    pub unlocked: bool,
    pub stored_item: Option<String>,
    pub backup_item: Option<String>,
    pub trainer_house_flag: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLinkSessionSnapshot {
    pub link_mode: u8,
    pub player_link_action: u8,
    pub chosen_cable_club_room: u8,
    pub other_player_link_mode: u8,
    pub friend_ready: bool,
    pub last_result: bool,
    pub failed_link_to_past: bool,
    pub quick_save_requested: bool,
    pub active_room: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLinkSessionDescriptor {
    pub session: LinkSessionIdentity,
    pub local_player: PlayerIdentity,
    pub hello: LinkHello,
    pub checksum: StateChecksumFrame,
    pub save_checkpoint: SessionSaveCheckpointFrame,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInputJournal {
    pub journal: DeterministicInputJournal,
    pub terminal_checksum: StateChecksumFrame,
}

impl RuntimeInputJournal {
    pub fn fingerprint(&self) -> Result<u32> {
        self.journal
            .fingerprint()
            .context("fingerprint runtime input journal")
    }

    pub fn fingerprint_hex(&self) -> Result<String> {
        self.journal
            .fingerprint_hex()
            .context("fingerprint runtime input journal")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShellAudioState {
    pub current_music: Option<String>,
    pub queued_events: Vec<crystal_core::state::ScriptAudioRuntimeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAudioEventDrain {
    pub events: Vec<crystal_core::state::ScriptAudioRuntimeEvent>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResolvedAudioEventDrain {
    pub events: Vec<RuntimeResolvedAudioPlayback>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeResolvedAudioPlayback {
    pub event: crystal_core::state::ScriptAudioRuntimeEvent,
    pub kind: RuntimeResolvedAudioPlaybackKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeResolvedAudioPlaybackKind {
    Play {
        audio_id: String,
        playback: ModpackAudioPlaybackEntry,
    },
    FadeMusic {
        audio_id: String,
        fade_frames: u16,
        playback: ModpackAudioPlaybackEntry,
    },
    WaitForSoundEffect,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAudioCatalogSnapshot {
    pub manifest: ModpackAudioManifest,
    pub playback: ModpackAudioPlaybackPlan,
    pub music: BTreeMap<String, RuntimeAudioProgramSnapshot>,
    pub sound_effects: BTreeMap<String, RuntimeAudioProgramSnapshot>,
    pub cries: BTreeMap<String, RuntimeAudioProgramSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeAudioProgramSnapshot {
    pub cache_key: String,
    pub source: RuntimeAudioProgramSourceSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeAudioProgramSourceSnapshot {
    Midi {
        byte_len: usize,
    },
    Pcm {
        byte_len: usize,
        format: AudioPcmFormat,
        loop_start_sample: Option<usize>,
        loop_end_sample: Option<usize>,
    },
    PcmGzip {
        byte_len: usize,
        format: AudioPcmFormat,
        loop_start_sample: Option<usize>,
        loop_end_sample: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTrainerSnapshot {
    pub player_name: String,
    pub player_id: u16,
    pub player_gender: u8,
    pub player_palette_id: u8,
    pub money: u32,
    pub moms_money: u32,
    pub coins: u16,
    pub blue_card_balance: u16,
    pub current_pc_box: usize,
    pub options: Options,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeProgressionSnapshot {
    pub rng_seed: u32,
    pub badges: Badges,
    pub pokedex_seen: usize,
    pub pokedex_owned: usize,
    pub pokedex_seen_species: BTreeSet<String>,
    pub pokedex_caught_species: BTreeSet<String>,
    pub link_wins: u16,
    pub link_losses: u16,
    pub link_draws: u16,
    pub pending_special_battle_type: Option<String>,
    pub repel_steps_remaining: u16,
    pub active_repel_item: Option<String>,
    pub registered_key_item: Option<String>,
    pub last_spawn_identifier: Option<u16>,
    pub hall_of_fame: crystal_core::state::HallOfFameState,
    pub time: crystal_core::systems::time::TimeState,
    pub active_event_flags: BTreeSet<String>,
    pub active_engine_flags: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptEventsSnapshot {
    pub script_value: Option<String>,
    pub variables: BTreeMap<String, String>,
    pub memory: BTreeMap<String, String>,
    pub named_buffers: BTreeMap<String, String>,
    pub variable_sprites: BTreeMap<String, String>,
    pub phone_numbers: BTreeSet<String>,
    pub last_special_routine: Option<String>,
    pub last_talked_object: Option<String>,
    pub active_menu: Option<String>,
    pub pending_delays: Vec<ScriptRuntimeDelay>,
    pub pending_earthquakes: Vec<ScriptRuntimeEarthquake>,
    pub pending_emotes: Vec<ScriptRuntimeEmote>,
    pub command_queue: Vec<ScriptRuntimeQueuedCommand>,
    pub call_stack: Vec<ScriptReturnFrame>,
    pub variable_writes: Vec<ScriptRuntimeVariableWrite>,
    pub effects: Vec<ScriptRuntimeEffect>,
    pub asm_directives: Vec<ScriptRuntimeAsmDirective>,
    pub numeric_buffer_writes: Vec<ScriptRuntimeNumericBufferWrite>,
    pub elevator_floors: Vec<ScriptRuntimeElevatorFloor>,
    pub stone_table_entries: Vec<ScriptRuntimeStoneTableEntry>,
    pub decoration_descriptions: Vec<ScriptRuntimeDecorationDescription>,
    pub special_phone_calls: Vec<String>,
    pub audio_events: Vec<crystal_core::state::ScriptAudioRuntimeEvent>,
    pub pending_music_fade: Option<ScriptMusicFade>,
    pub waiting_for_sound_effect: bool,
    pub map_music_restart_disabled: bool,
    pub map_music_requested: bool,
    pub graphics_events: Vec<ScriptGraphicsRuntimeEvent>,
    pub pending_screen_fade: Option<ScriptScreenFade>,
    pub money_events: Vec<ScriptMoneyRuntimeEvent>,
    pub map_events: Vec<ScriptMapRuntimeEvent>,
    pub pending_script_warp: Option<ScriptWarpRequest>,
    pub pending_map_load: Option<ScriptMapLoadRequest>,
    pub pending_map_refresh: Option<ScriptMapRefreshRequest>,
    pub warp_check_requested: bool,
    pub text_events: Vec<ScriptTextRuntimeEvent>,
    pub window_open: bool,
    pub menu_coords: Option<[i16; 4]>,
    pub active_pokemon_picture: Option<String>,
    pub text_window_open: bool,
    pub pending_text_label: Option<String>,
    pub pending_text_wait: Option<ScriptTextWait>,
    pub pending_yes_no: Option<ScriptYesNoPrompt>,
    pub control_events: Vec<ScriptControlRuntimeEvent>,
    pub next_script: Option<ScriptLocation>,
    pub deferred_scripts: Vec<ScriptLocation>,
    pub script_ended: Option<ScriptEndState>,
    pub player_input_locked: bool,
    pub all_input_locked: bool,
    pub script_stop_requested: bool,
    pub item_notify_queued: bool,
    pub warp_sound_queued: bool,
    pub teleport_from_queued: bool,
    pub hall_of_fame_requested: bool,
    pub credits_requested: bool,
    pub reset_requested: bool,
    pub menu_2d_requested: bool,
    pub version_check_requested: bool,
    pub blackout_mod: Option<String>,
    pub battle_tower_text: Option<String>,
    pub completed_trades: Vec<String>,
    pub catch_tutorials: Vec<String>,
    pub checked_mail_targets: Vec<String>,
    pub given_mail_targets: Vec<String>,
    pub shop_events: Vec<ScriptShopRuntimeEvent>,
    pub item_use_events: Vec<ItemUseRuntimeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenuClose {
    pub menu: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeWindowClose {
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTextWindowClose {
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTextWaitAdvance {
    pub wait: ScriptTextWait,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeYesNoResolution {
    pub prompt: ScriptYesNoPrompt,
    pub accepted: bool,
    pub script_value: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVerticalMenuOpen {
    pub map_name: String,
    pub menu_key: String,
    pub menu_id: String,
    pub source_script: String,
    pub loadmenu_command_index: usize,
    pub verticalmenu_command_index: usize,
    pub options: Vec<String>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVerticalMenuOptionSelection {
    pub menu_id: String,
    pub source_script: String,
    pub verticalmenu_command_index: usize,
    pub option_index: usize,
    pub option: String,
    pub script_value: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeElevatorFloorSelection {
    pub map_name: String,
    pub data_label: String,
    pub source_script: String,
    pub elevator_command_index: usize,
    pub floor_index: usize,
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
    pub destination_tile: crystal_core::world::map::TilePosition,
    pub script_value: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLinkedMenuChoice {
    pub frame: MenuChoiceFrame,
    pub selection: RuntimeVerticalMenuOptionSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenuCoordsClear {
    pub coords: [i16; 4],
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokemonPictureClose {
    pub species_id: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShopClose {
    pub shop: ScriptShopRequest,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenuSnapshot {
    pub menu_id: String,
    pub source: RuntimeMenuSource,
    pub definition: Option<ScriptMenuDefinition>,
    pub layout: RuntimeMenuLayoutSnapshot,
    pub window_open: bool,
    pub coords: Option<[i16; 4]>,
    pub menu_2d_requested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenuLayoutSnapshot {
    pub declared_coords: Option<[i16; 4]>,
    pub data_commands: Vec<RuntimeMenuDataCommandSnapshot>,
    pub vertical_menus: Vec<RuntimeVerticalMenuSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenuDataCommandSnapshot {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeVerticalMenuSnapshot {
    pub source_script: String,
    pub loadmenu_command_index: usize,
    pub verticalmenu_command_index: usize,
    pub header_label: String,
    pub data_label: Option<String>,
    pub options: Vec<String>,
    pub two_dimensional: bool,
    pub rows: Option<usize>,
    pub columns: Option<usize>,
    pub spacing: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeElevatorSnapshot {
    pub map_name: String,
    pub source_script: String,
    pub elevator_command_index: usize,
    pub data_label: String,
    pub floors: Vec<RuntimeElevatorFloorSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeElevatorFloorSnapshot {
    pub floor_index: usize,
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGiftPokemonSnapshot {
    pub map_name: String,
    pub source_script: String,
    pub command_index: usize,
    pub species_id: String,
    pub level: u8,
    pub level_token: String,
    pub held_item_id: Option<String>,
    pub nickname_label: Option<String>,
    pub ot_label: Option<String>,
    pub egg: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeUiSnapshot {
    pub menu: Option<RuntimeMenuSnapshot>,
    pub elevators: Vec<RuntimeElevatorSnapshot>,
    pub gift_pokemon: Vec<RuntimeGiftPokemonSnapshot>,
    pub text: Option<RuntimeTextSnapshot>,
    pub window_open: bool,
    pub text_window_open: bool,
    pub coords: Option<[i16; 4]>,
    pub active_pokemon_picture: Option<String>,
    pub pending_yes_no: Option<ScriptYesNoPrompt>,
    pub pending_text_wait: Option<ScriptTextWait>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTextSnapshot {
    pub label: String,
    pub source: RuntimeTextSource,
    pub asm_text: Option<String>,
    pub body: Option<ScriptTextBody>,
    pub queued_text_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeTextSource {
    AsmText,
    ScriptBody { map_name: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMenuSource {
    ScriptDefinition { map_name: String },
    SpecialRoutine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleSnapshot {
    pub kind: RuntimeBattleKind,
    pub battle_music: String,
    pub battle_type: String,
    pub enemy_pokemon: Pokemon,
    pub enemy_party: Vec<Pokemon>,
    pub active_player_party_index: Option<usize>,
    pub active_enemy_party_index: Option<usize>,
    pub player_transformed_species: Option<String>,
    pub enemy_transformed_species: Option<String>,
    pub player_substitute_hp: u16,
    pub enemy_substitute_hp: u16,
    pub player_semi_invulnerable: bool,
    pub enemy_semi_invulnerable: bool,
    pub player_moves: Vec<LearnedMove>,
    pub enemy_moves: Vec<LearnedMove>,
    pub player_last_move: Option<String>,
    pub player_used_moves: Vec<String>,
    pub enemy_last_move: Option<String>,
    pub enemy_toxic_turns: u8,
    pub enemy_turns_taken: u8,
    pub enemy_switch_locked: bool,
    pub player_cannot_escape: bool,
    pub player_wrapped: bool,
    pub enemy_wrapped: bool,
    pub rewarded_enemy_party_indices: Vec<usize>,
    pub escape_attempts: u8,
    pub player_stat_drop_guard_turns: u8,
    pub pay_day_money: u32,
    pub amulet_coin_active: bool,
    pub trainer_items_used: BTreeSet<String>,
    pub player_disabled_move: Option<String>,
    pub commands: RuntimeBattleCommandSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptedTrainerBattleKey {
    pub map_name: String,
    pub source_script: String,
    pub loadtrainer_command_index: usize,
    pub startbattle_command_index: usize,
    pub battle_type: String,
    pub trainer_class: String,
    pub trainer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeWildEncounterOriginKey {
    pub map_name: String,
    pub species: String,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptCommandKey {
    pub script_label: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptCommandPayloadKey {
    pub script_label: String,
    pub command_index: usize,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptReturnKey {
    pub script_label: String,
    pub next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeWarpKey {
    pub map_name: String,
    pub warp_index: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMapObjectKey {
    pub map_name: String,
    pub object_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMapSceneKey {
    pub map_name: String,
    pub scene_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMapMetadataKey {
    pub map_name: String,
    pub map_id: String,
    pub tileset_name: String,
    pub border_block: u8,
    pub width: u16,
    pub height: u16,
    pub time_of_day: Option<String>,
    pub phone_service: u8,
    pub phone_flag: bool,
    pub environment: Option<String>,
    pub location: Option<String>,
    pub music: Option<String>,
    pub palette: Option<String>,
    pub fishing_group: Option<String>,
    pub map_constant: Option<String>,
    pub map_group_constant: Option<String>,
    pub metadata_constant: Option<String>,
    pub metadata_group_name: Option<String>,
    pub metadata_group_id: Option<u16>,
    pub metadata_map_id: Option<u16>,
    pub metadata_environment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeTilesetKey {
    pub tileset_id: String,
    pub collision: BTreeMap<String, Vec<String>>,
    pub palette_map: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeAudioAssetKey {
    pub audio_id: String,
    pub kind: String,
    pub source: String,
    pub path: String,
    pub byte_len: usize,
    pub payload_hash: String,
    pub pcm_sample_rate_hz: Option<u32>,
    pub pcm_channels: Option<u8>,
    pub pcm_bits_per_sample: Option<u8>,
    pub pcm_frame_count: Option<usize>,
    pub cache_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMartKey {
    pub mart_id: String,
    pub item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFruitTreeKey {
    pub fruit_tree_id: String,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFieldMoveReplacementKey {
    pub replacement_block_id: u16,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFieldMoveRuleKey {
    pub rule_id: String,
    pub rule_kind: String,
    pub move_id: Option<String>,
    pub item_id: Option<String>,
    pub badge_region: Option<String>,
    pub badge_index: Option<usize>,
    pub engine_flag: Option<String>,
    pub escape_rope_mode: Option<String>,
    pub target_collisions: Vec<u8>,
    pub blocked_collisions: Vec<u8>,
    pub replacements: BTreeMap<String, BTreeMap<u16, RuntimeFieldMoveReplacementKey>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeFlyDestinationKey {
    pub flypoint_flag: String,
    pub destination_spawn_identifier: u16,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimePokemonCryKey {
    pub species_id: String,
    pub cry_id: String,
    pub pitch: i16,
    pub length: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimePcStringKey {
    pub string_id: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMenuIconKey {
    pub species_id: String,
    pub icon_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimePokedexEntryKey {
    pub species_id: String,
    pub species: String,
    pub classification: String,
    pub height_digits: u16,
    pub weight_digits: u16,
    pub pages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimePokegearLandmarkKey {
    pub landmark_id: u16,
    pub constant: String,
    pub label: String,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimePokegearMapLandmarkKey {
    pub map_name: String,
    pub landmark_constant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptVerticalMenuKey {
    pub map_name: String,
    pub menu_key: String,
    pub source_script: String,
    pub loadmenu_command_index: usize,
    pub verticalmenu_command_index: usize,
    pub header_label: String,
    pub data_label: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptTextBodyCommandKey {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptTextBodyKey {
    pub map_name: String,
    pub body_key: String,
    pub label: String,
    pub commands: Vec<RuntimeScriptTextBodyCommandKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptMenuCommandKey {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptMenuDefinitionKey {
    pub map_name: String,
    pub menu_key: String,
    pub label: String,
    pub commands: Vec<RuntimeScriptMenuCommandKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptElevatorFloorKey {
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptElevatorKey {
    pub map_name: String,
    pub elevator_key: String,
    pub source_script: String,
    pub elevator_command_index: usize,
    pub data_label: String,
    pub floors: Vec<RuntimeScriptElevatorFloorKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeGiftPokemonKey {
    pub map_name: String,
    pub species_id: String,
    pub level_token: String,
    pub level: u8,
    pub held_item_id: Option<String>,
    pub nickname_label: Option<String>,
    pub ot_label: Option<String>,
    pub source_script: String,
    pub command_index: usize,
    pub egg: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptObjectCommandKey {
    pub map_name: String,
    pub command: String,
    pub object_id: Option<String>,
    pub target_object_id: Option<String>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub direction: Option<String>,
    pub movement: Option<String>,
    pub emote: Option<String>,
    pub duration: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptMovementStepKey {
    pub command: String,
    pub direction: Option<String>,
    pub duration: Option<u16>,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptMovementKey {
    pub map_name: String,
    pub label: String,
    pub source_script: Option<String>,
    pub steps: Vec<RuntimeScriptMovementStepKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMapScriptSectionCommandKey {
    pub map_name: String,
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMapEventSectionCommandKey {
    pub map_name: String,
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptMapCommandKey {
    pub map_name: String,
    pub command: String,
    pub target_map: Option<String>,
    pub x: Option<u16>,
    pub y: Option<u16>,
    pub facing: Option<String>,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptVariableCommandKey {
    pub map_name: String,
    pub command: String,
    pub target: Option<String>,
    pub value_tokens: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptControlCommandKey {
    pub map_name: String,
    pub command: String,
    pub compare_value: Option<String>,
    pub target_label: Option<String>,
    pub resolved_target_script: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptFieldPickupKey {
    pub map_name: String,
    pub command: String,
    pub item_id: Option<String>,
    pub quantity: u16,
    pub event_flag: Option<String>,
    pub fruit_tree_id: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptShopCommandKey {
    pub map_name: String,
    pub command: String,
    pub mart_type: String,
    pub mart_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptPhoneCommandKey {
    pub map_name: String,
    pub command: String,
    pub contact_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptSwarmCommandKey {
    pub map_name: String,
    pub command: String,
    pub swarm_token: String,
    pub map_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptRuntimeCommandKey {
    pub map_name: String,
    pub command: String,
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptItemGrantKey {
    pub map_name: String,
    pub command: String,
    pub item_id: String,
    pub quantity: u16,
    pub source_script: String,
    pub command_index: usize,
    pub verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptItemAccessKey {
    pub map_name: String,
    pub command: String,
    pub item_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptEconomyCommandKey {
    pub map_name: String,
    pub command: String,
    pub account: Option<String>,
    pub amount_tokens: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptFlagCommandKey {
    pub map_name: String,
    pub command: String,
    pub flag_id: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptSceneCommandKey {
    pub map_name: String,
    pub command: String,
    pub map_id: Option<String>,
    pub scene_id: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptBlockChangeKey {
    pub map_name: String,
    pub x: u16,
    pub y: u16,
    pub block_id: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptAudioCommandKey {
    pub map_name: String,
    pub command: String,
    pub audio_id: Option<String>,
    pub fade_frames: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeScriptTextCommandKey {
    pub map_name: String,
    pub command: String,
    pub text_label: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeCaptureBallRuleKey {
    pub ball_id: String,
    pub multiplier_numerator: u16,
    pub multiplier_denominator: u16,
    pub battle_type: String,
    pub skip_hp_calc: bool,
    pub use_heavy_ball_weight_modifier: bool,
    pub use_level_ball_multiplier: bool,
    pub require_same_species: bool,
    pub require_same_gender: bool,
    pub require_fast_species: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeHeavyBallModifierKey {
    pub species_id: String,
    pub modifier: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeCaptureStatusBonusKey {
    pub status: String,
    pub bonus: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeCaptureWobbleProbabilityKey {
    pub catch_rate: u8,
    pub chance: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeItemBattleUseKey {
    pub item_id: String,
    pub effect: String,
    pub battle_menu: String,
    pub battle_usable: bool,
    pub battle_stat_boost_stat: Option<String>,
    pub battle_stat_boost_stages: Option<u8>,
    pub battle_escape_mode: Option<String>,
    pub battle_focus_energy: Option<bool>,
    pub battle_stat_drop_guard: Option<bool>,
    pub battle_stat_drop_guard_turns: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeItemEffectPlanKey {
    pub item_id: String,
    pub effect_id: String,
    pub behavior_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeItemFieldUseKey {
    pub item_id: String,
    pub effect: String,
    pub field_menu: String,
    pub field_usable: bool,
    pub consumable: bool,
    pub repel_steps: Option<u16>,
    pub escape_rope_mode: Option<String>,
    pub tmhm_index: Option<usize>,
    pub tmhm_move: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMoveBattleDataKey {
    pub move_id: String,
    pub name: String,
    pub move_type: String,
    pub power: u16,
    pub accuracy: u8,
    pub pp: u8,
    pub effect: String,
    pub effect_chance: u8,
    pub stat: Option<Stat>,
    pub amount: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeSpeciesBattleDataKey {
    pub species_id: String,
    pub int_id: u16,
    pub base_hp: u16,
    pub base_attack: u16,
    pub base_defense: u16,
    pub base_speed: u16,
    pub base_special_attack: u16,
    pub base_special_defense: u16,
    pub type1: String,
    pub type2: String,
    pub catch_rate: u8,
    pub base_exp: u16,
    pub item1: Option<String>,
    pub item2: Option<String>,
    pub gender_ratio: u8,
    pub step_cycles_to_hatch: u8,
    pub growth_rate: String,
    pub egg_group1: String,
    pub egg_group2: String,
    pub tmhm_learnset: Vec<String>,
    pub ability: String,
    pub weight: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeSpeciesLearnsetKey {
    pub species_id: String,
    pub level: u8,
    pub move_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeSpeciesEvolutionKey {
    pub source_species_id: String,
    pub method: String,
    pub target_species_id: String,
    pub level: Option<u8>,
    pub item: Option<String>,
    pub held_item: Option<String>,
    pub happiness: Option<String>,
    pub stat_ratio: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeTrainerBattleDataKey {
    pub trainer_id: String,
    pub name: String,
    pub trainer_class: String,
    pub win_quote: String,
    pub lose_quote: String,
    pub items: Vec<Option<String>>,
    pub base_reward: u32,
    pub ai_move_flags: u32,
    pub ai_item_switch_flags: u32,
    pub encounter_music: String,
    pub ai_layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeTrainerPartyPokemonKey {
    pub trainer_id: String,
    pub party_index: usize,
    pub species: String,
    pub level: u8,
    pub item: Option<String>,
    pub move_names: Vec<String>,
    pub move_pp: Vec<u8>,
    pub move_pp_ups: Vec<u8>,
    pub dv_attack: u8,
    pub dv_defense: u8,
    pub dv_speed: u8,
    pub dv_special: u8,
    pub dv_hp: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMovePriorityEffectKey {
    pub effect_id: String,
    pub priority: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeMovePriorityMoveKey {
    pub move_id: String,
    pub priority: i8,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeBattleStatMultiplierKey {
    pub table: String,
    pub stage: i8,
    pub numerator: i32,
    pub denominator: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeBattleRewardRuleKey {
    pub field: String,
    pub value: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeBattleEscapeRuleKey {
    pub field: String,
    pub value: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeTypeEffectivenessKey {
    pub attacking_type: String,
    pub defending_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeWeatherTypeModifierKey {
    pub weather: String,
    pub type_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RuntimeWeatherMoveEffectModifierKey {
    pub weather: String,
    pub effect_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleCommandSnapshot {
    pub player_move_slots: Vec<usize>,
    pub player_forced_struggle: bool,
    /// The source battle loop bypasses command selection while an existing
    /// multi-turn move or recharge state owns the player's next action.
    pub player_turn_automatic: bool,
    /// Bide still exposes the four-command menu, but choosing FIGHT bypasses
    /// move selection and resumes the retained Bide move immediately.
    pub player_fight_automatic: bool,
    pub enemy_move_slots: Vec<usize>,
    pub switch_party_indices: Vec<usize>,
    pub can_use_items: bool,
    pub can_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeBattleKind {
    Wild {
        map_name: String,
        battle_music: String,
    },
    StaticWild {
        origin_map_name: String,
        species: String,
        level: u8,
        source_script: String,
        startbattle_command_index: usize,
        resume_command_index: usize,
        battle_music: String,
    },
    Trainer {
        trainer_class: String,
        trainer_id: String,
        trainer_name: String,
        event_flag: String,
        seen_text: String,
        win_text: String,
        loss_text: String,
        callback: String,
        source_script: String,
        reward: u32,
        encounter_music: String,
        ai_move_flags: u32,
        ai_item_switch_flags: u32,
        ai_layers: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePendingMoveLearnSnapshot {
    pub party_index: usize,
    pub species_id: String,
    pub level: u8,
    pub learned_move: LearnedMove,
    pub defer_level_evolution: bool,
}

impl RuntimePendingMoveLearnSnapshot {
    fn from_state(state: &GameState) -> Option<Self> {
        let pending = state.pending_move_learn.as_ref()?;
        Some(Self {
            party_index: pending.party_index,
            species_id: pending.species_id.clone(),
            level: pending.level,
            learned_move: pending.learned_move.clone(),
            defer_level_evolution: pending.defer_level_evolution,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePendingMoveLearnResolution {
    pub resolution: PendingMoveLearnResolution,
    pub deferred_evolution: Option<EvolutionReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartySnapshot {
    pub slots: Vec<RuntimePartySlotSnapshot>,
    pub active_battle_slot: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartySlotSnapshot {
    pub index: usize,
    pub pokemon: Pokemon,
    pub is_active_battle_pokemon: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageSnapshot {
    pub current_pc_box: usize,
    pub party_count: usize,
    pub boxes: Vec<RuntimePcBoxSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePcBoxSnapshot {
    pub index: usize,
    pub name: String,
    pub count: usize,
    pub slots: Vec<RuntimePcBoxSlotSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePcBoxSlotSnapshot {
    pub index: usize,
    pub pokemon: Pokemon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageBoxSwitch {
    pub box_index_before: usize,
    pub box_index_after: usize,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageDeposit {
    pub party_index: usize,
    pub box_index: usize,
    pub box_slot: usize,
    pub pokemon: Pokemon,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageWithdraw {
    pub box_index: usize,
    pub box_slot: usize,
    pub party_index: usize,
    pub pokemon: Pokemon,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageRelease {
    pub box_index: usize,
    pub box_slot: usize,
    pub pokemon: Pokemon,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStorageMove {
    pub source_box: usize,
    pub source_slot: usize,
    pub target_box: usize,
    pub target_slot: usize,
    pub swapped: bool,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePcItemTransfer {
    pub item_id: String,
    pub quantity: u16,
    pub bag_quantity_after: u16,
    pub pc_quantity_after: u16,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeHeldItemTransfer {
    pub party_index: usize,
    pub item_id: String,
    pub bag_quantity_after: u16,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMailTransfer {
    pub party_index: Option<usize>,
    pub mailbox_index: Option<usize>,
    pub item_id: String,
    pub mail: crystal_core::models::pokemon::MailData,
    pub mailbox_count_after: usize,
    pub bag_quantity_after: u16,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBagItemMutation {
    pub item_id: String,
    pub quantity: u16,
    pub added: bool,
    pub quantity_before: u16,
    pub quantity_after: u16,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBadgeAward {
    pub region: RuntimeBadgeRegion,
    pub index: usize,
    pub already_awarded: bool,
    pub awarded_count_after: usize,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokedexRecord {
    pub species_id: String,
    pub already_seen: bool,
    pub already_caught: bool,
    pub seen_count_after: usize,
    pub caught_count_after: usize,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCurrencyMutation {
    pub account: RuntimeCurrencyAccount,
    pub amount: u32,
    pub value_before: u32,
    pub value_after: u32,
    pub cap: u32,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLinkBattleRecord {
    pub result: RuntimeLinkBattleResult,
    pub wins_after: u16,
    pub losses_after: u16,
    pub draws_after: u16,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOptionsSet {
    pub options_before: Options,
    pub options_after: Options,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTrainerIdentitySet {
    pub player_name_before: String,
    pub player_id_before: u16,
    pub player_name_after: String,
    pub player_id_after: u16,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlayerGenderSet {
    pub player_gender_before: u8,
    pub player_gender_after: u8,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyNicknameSet {
    pub party_index: usize,
    pub species_id: String,
    pub nickname_before: String,
    pub nickname_after: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyRecoveryStateSet {
    pub outcome: RuntimePartyRecoverySetupOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyHpTransfer {
    pub outcome: RuntimePartyHpTransferOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyRecovery {
    pub party_index: usize,
    pub species_id: String,
    pub hp_before: u16,
    pub hp_after: u16,
    pub status_before: Option<String>,
    pub status_after: Option<String>,
    pub pp_restored: Vec<(String, u8, u8)>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBlackoutRecovery {
    pub spawn_identifier: Option<u16>,
    pub map_name: String,
    pub tile: TilePosition,
    pub healed: Vec<RuntimePartyRecovery>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartySwap {
    pub first_party_index: usize,
    pub second_party_index: usize,
    pub first_species_after: String,
    pub second_species_after: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyMoveSwap {
    pub party_index: usize,
    pub first_move_index: usize,
    pub second_move_index: usize,
    pub first_move_after: String,
    pub second_move_after: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBagSnapshot {
    pub items: Vec<RuntimeBagItemSnapshot>,
    pub balls: Vec<RuntimeBagItemSnapshot>,
    pub key_items: Vec<RuntimeBagItemSnapshot>,
    pub tm_hm: Vec<RuntimeTmHmSnapshot>,
    pub pc_items: Vec<RuntimeBagItemSnapshot>,
    pub custom_pockets: BTreeMap<String, Vec<RuntimeBagItemSnapshot>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBagItemSnapshot {
    pub item_id: String,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemCatalogSnapshot {
    pub item_id: String,
    pub name: String,
    pub description: String,
    pub effect: String,
    pub status_heals: Vec<String>,
    pub revive_hp_percent: Option<u8>,
    pub party_revive_hp_percent: Option<u8>,
    pub pp_restore_scope: Option<String>,
    pub pp_restore_points: Option<u8>,
    pub pp_up_stages: Option<u8>,
    pub vitamin_stat: Option<String>,
    pub vitamin_stat_exp: Option<u16>,
    pub vitamin_max_stat_exp: Option<u16>,
    pub rare_candy_level_gain: Option<u8>,
    pub party_special_effect: bool,
    pub battle_stat_boost_stat: Option<String>,
    pub battle_stat_boost_stages: Option<u8>,
    pub battle_escape_mode: Option<String>,
    pub battle_focus_energy: Option<bool>,
    pub battle_stat_drop_guard: Option<bool>,
    pub battle_stat_drop_guard_turns: Option<u8>,
    pub confusion_heal: Option<bool>,
    pub repel_steps: Option<u16>,
    pub escape_rope_mode: Option<String>,
    pub price: u16,
    pub held_effect: String,
    pub parameter: i16,
    pub property: String,
    pub pocket: String,
    pub field_menu: String,
    pub field_usable: bool,
    pub battle_menu: String,
    pub battle_usable: bool,
    pub script_name: String,
    pub consumable: bool,
    pub tmhm_index: Option<usize>,
    pub tmhm_move: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMoveCatalogSnapshot {
    pub move_id: String,
    pub name: String,
    pub move_type: String,
    pub power: u16,
    pub accuracy: u8,
    pub pp: u8,
    pub effect: String,
    pub effect_chance: u8,
    pub stat: Option<Stat>,
    pub amount: Option<i8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokemonCatalogSnapshot {
    pub species_id: String,
    pub int_id: u16,
    pub base_stats: crystal_core::models::BaseStats,
    pub type1: String,
    pub type2: String,
    pub catch_rate: u8,
    pub base_exp: u16,
    pub item1: Option<String>,
    pub item2: Option<String>,
    pub gender_ratio: u8,
    pub unknown1: u8,
    pub step_cycles_to_hatch: u8,
    pub unknown2: u8,
    pub growth_rate: String,
    pub egg_group1: String,
    pub egg_group2: String,
    pub tmhm_learnset: Vec<String>,
    pub ability: String,
    pub pic_size: u8,
    pub front_pic: u16,
    pub back_pic: u16,
    pub weight: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTrainerCatalogSnapshot {
    pub trainer_id: String,
    pub name: String,
    pub trainer_class: String,
    pub party: Vec<RuntimeTrainerPartyPokemonSnapshot>,
    pub win_quote: String,
    pub lose_quote: String,
    pub items: Vec<Option<String>>,
    pub base_reward: u32,
    pub ai_move_flags: u32,
    pub ai_item_switch_flags: u32,
    pub encounter_music: String,
    pub ai_layers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTrainerPartyPokemonSnapshot {
    pub species: String,
    pub level: u8,
    pub item: Option<String>,
    pub moves: Vec<RuntimeLearnedMoveSnapshot>,
    pub dvs: Dv,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLearnedMoveSnapshot {
    pub name: String,
    pub current_pp: u8,
    pub pp_ups: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMapCatalogSnapshot {
    pub map_name: String,
    pub id: String,
    pub attributes: crystal_core::map::MapAttributes,
    pub metadata: Option<RuntimeMapMetadataSnapshot>,
    pub scenes: crystal_core::map::MapSceneTable,
    pub events: crystal_core::map::MapEvents,
    pub objects: Vec<crystal_core::map::ObjectEvent>,
    pub blocks: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMapMetadataSnapshot {
    pub constant: String,
    pub name: String,
    pub group_name: String,
    pub group_id: u16,
    pub map_id: u16,
    pub width: u16,
    pub height: u16,
    pub environment: String,
    pub phone_service: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCurrentSceneScript {
    pub map_name: String,
    pub scene_id: String,
    pub script_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTilesetCatalogSnapshot {
    pub tileset_id: String,
    pub collision: BTreeMap<String, Vec<String>>,
    pub palette_map: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEncounterCatalogSnapshot {
    pub wild: BTreeMap<String, WildEncounterData>,
    pub field: BTreeMap<String, FieldEncounterData>,
    pub slot_tables: EncounterSlotTables,
    pub fishing: crystal_core::world::fishing::FishingCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleRuleCatalogSnapshot {
    pub capture_rules: CaptureRules,
    pub capture_wobble_probabilities: Vec<CaptureWobbleProbability>,
    pub stat_multipliers: BattleStatMultiplierTables,
    pub move_priorities: MovePriorityTable,
    pub type_categories: TypeCategories,
    pub type_effectiveness: TypeEffectivenessTable,
    pub weather_modifiers: WeatherModifiers,
    pub reward_rules: BattleRewardRules,
    pub escape_rules: BattleEscapeRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeWorldRuleCatalogSnapshot {
    pub marts: crystal_core::systems::shop::MartCatalog,
    pub currency: crystal_core::systems::economy::CurrencyCatalog,
    pub fruit_trees: crystal_core::systems::field_items::FruitTreeCatalog,
    pub field_moves: crystal_core::systems::field_moves::FieldMoveCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePresentationCatalogSnapshot {
    pub pc_strings: BTreeMap<String, String>,
    pub menu_icons: BTreeMap<String, String>,
    pub pokedex_entries: BTreeMap<String, RuntimePokedexEntry>,
    pub pokemon_frontpic_anim: BTreeMap<String, FrontpicAnimProgram>,
    pub asm_text: BTreeMap<String, String>,
    pub move_names: Vec<String>,
    pub battle_animations: BTreeMap<String, Vec<String>>,
    pub battle_animation_table: Vec<String>,
    pub battle_anim_bundle: String,
    pub sprite_anim_bundle: String,
    pub sprite_palette_defaults: BTreeMap<String, i64>,
    pub pokegear_town_map_palette_map: BTreeMap<String, Vec<String>>,
    pub pokegear_landmarks: PokegearLandmarksPayload,
    pub pokemon_cries: BTreeMap<String, PokemonCryMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSpecialCatalogSnapshot {
    pub phone_contacts: crystal_core::systems::phone::PhoneContactCatalog,
    pub permanent_phone_numbers:
        BTreeMap<String, crystal_core::systems::phone::PermanentPhoneNumberRule>,
    pub special_phone_calls: BTreeMap<String, crystal_assets::SpecialPhoneCallRule>,
    pub npc_trades: BTreeMap<String, crystal_assets::NpcTradeRule>,
    pub special_routines: BTreeMap<String, crystal_assets::SpecialRoutineRule>,
    pub flee_mons: crystal_core::systems::flee_mons::FleeMonTables,
    pub buena_password_categories: crystal_core::systems::special_routines::BuenaPasswordCategories,
    pub roaming_pokemon: crystal_core::systems::special_routines::RoamingPokemonCatalog,
    pub buena_prizes: crystal_core::systems::special_routines::BuenaPrizeDefinitions,
    pub kurt_apricorn_recipes: crystal_core::systems::special_routines::KurtApricornRecipes,
    pub shuckie_gift: Option<crystal_core::systems::special_routines::ShuckieGiftDefinition>,
    pub dratini_move_sets: crystal_core::systems::special_routines::DratiniMoveSets,
    pub bug_contest_config: Option<crystal_core::systems::special_routines::BugContestConfig>,
    pub battle_tower_rules: Option<crystal_core::systems::special_routines::BattleTowerRules>,
    pub oak_ratings: Vec<crystal_core::systems::special_routines::OakRatingEntry>,
    pub odd_egg_definitions: Vec<crystal_core::systems::special_routines::OddEggDefinition>,
    pub magikarp_lengths: Vec<crystal_core::systems::special_routines::MagikarpLengthEntry>,
    pub happiness_data: Option<crystal_core::systems::special_routines::HappinessData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStoryCatalogSnapshot {
    pub initialize_events: crystal_core::systems::script_runtime::InitializeEventsConfig,
    pub story_event_script_constants:
        crystal_core::systems::script_runtime::StoryEventScriptConstants,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTmHmSnapshot {
    pub item_id: String,
    pub tmhm_index: usize,
    pub move_id: Option<String>,
    pub quantity: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeShellPhase {
    Overworld,
    WildBattle,
    StaticWildBattle,
    TrainerBattle,
    Text,
    YesNo,
    Menu,
    Shop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOverworldFrame {
    pub snapshot: OverworldSnapshot,
    pub input_mask: u8,
    pub pressed_mask: u8,
    pub autonomous_objects_changed: bool,
    pub movement: Option<StepOutcome>,
    pub ledge_jump: Option<LedgeJumpOutcome>,
    pub grass_rustle: Option<crystal_assets::OverworldGrassRustle>,
    pub phone_call: Option<crystal_assets::IncomingPhoneCall>,
    pub step_events: Option<StepEventResult>,
    pub coord_event: Option<CoordEventTrigger>,
    pub trainer_sight: Option<OverworldInteraction>,
    pub interaction: Option<OverworldInteraction>,
    pub warp: Option<WarpTransition>,
    pub connection: Option<ConnectionTransition>,
    pub wild_encounter: Option<WildEncounterRoll>,
    pub wild_battle: Option<WildBattleStart>,
    pub state_checksum: StateChecksum,
}

/// A deterministic RTC sample supplied by the host or replay/oracle adapter.
/// Applying it immediately before the joypad frame preserves Crystal's
/// ordering: day-boundary resets are committed before movement and scripts
/// observe the new date/time in that same frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeRtcSample {
    pub date: GameDate,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl RuntimeOverworldFrame {
    fn from_input_frame(frame: OverworldInputFrame, state_checksum: StateChecksum) -> Self {
        Self {
            snapshot: frame.snapshot,
            input_mask: frame.input_mask,
            pressed_mask: frame.pressed_mask,
            autonomous_objects_changed: frame.autonomous_objects_changed,
            movement: frame.movement,
            ledge_jump: frame.ledge_jump,
            grass_rustle: frame.grass_rustle,
            phone_call: frame.phone_call,
            step_events: frame.step_events,
            coord_event: frame.coord_event,
            trainer_sight: frame.trainer_sight,
            interaction: frame.interaction,
            warp: frame.warp,
            connection: frame.connection,
            wild_encounter: frame.wild_encounter,
            wild_battle: frame.wild_battle,
            state_checksum,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInteractionScriptDispatch {
    pub next_script: String,
    pub last_talked_object: Option<String>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompiledScriptCursor {
    pub origin_map_name: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompiledScriptStep {
    pub origin_map_name: String,
    pub source_script: String,
    pub command_index: usize,
    pub command: String,
    pub mutation: RuntimeMutationOutcome,
    pub next_cursor: Option<RuntimeCompiledScriptCursor>,
    pub boundary: Option<RuntimeCompiledScriptBoundary>,
    pub next_script: Option<String>,
    pub ended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeCompiledScriptBoundary {
    DayOfWeekPrompt,
    TextLabel(String),
    TextWait(ScriptTextWait),
    YesNo(ScriptYesNoPrompt),
    ActiveMenu(String),
    PendingShop(ScriptShopRequest),
    PendingScriptWarp(ScriptWarpRequest),
    PendingMapLoad(ScriptMapLoadRequest),
    PendingMapRefresh(ScriptMapRefreshRequest),
    Delay(ScriptRuntimeDelay),
    Earthquake(ScriptRuntimeEarthquake),
    Emote(ScriptRuntimeEmote),
    ScriptMovement,
    WaitForSoundEffect,
    ActiveBattle(RuntimeShellPhase),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompiledScriptRun {
    pub steps: Vec<RuntimeCompiledScriptStep>,
    pub next_cursor: Option<RuntimeCompiledScriptCursor>,
    pub boundary: Option<RuntimeCompiledScriptBoundary>,
    pub ended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeQueuedCompiledScriptRun {
    pub queued: RuntimeQueuedScriptCommand,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePendingCompiledScriptRun {
    pub next_script: RuntimeNextScript,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStandardScriptRun {
    pub next_script: RuntimeNextScript,
    pub result: String,
    pub state_checksum: StateChecksum,
    pub boundary: Option<RuntimeCompiledScriptBoundary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDeferredCompiledScriptRun {
    pub deferred_script: RuntimeDeferredScript,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTextWaitCompiledScriptRun {
    pub wait: RuntimeTextWaitAdvance,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeYesNoCompiledScriptRun {
    pub resolution: RuntimeYesNoResolution,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePhonePromptCompiledScriptRun {
    pub step: RuntimeCompiledScriptStep,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeMenuSelectionCompiledScriptRun {
    pub selection: RuntimeVerticalMenuOptionSelection,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeElevatorFloorCompiledScriptRun {
    pub selection: RuntimeElevatorFloorSelection,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGiftPokemonCompiledScriptRun {
    pub grant: RuntimeGiftPokemonGrant,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptWarpCompiledScriptRun {
    pub warp: RuntimeScriptWarp,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptedWildBattleCompiledScriptRun {
    pub completion: RuntimeScriptedBattleCompletion,
    pub run: RuntimeCompiledScriptRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptedTrainerBattleCompiledScriptRun {
    pub completion: RuntimeScriptedBattleCompletion,
    pub run: RuntimeCompiledScriptRun,
}

fn buttons_from_joypad_mask(mask: u8) -> Vec<GameButton> {
    [
        (B_PAD_A, GameButton::A),
        (B_PAD_B, GameButton::B),
        (B_PAD_START, GameButton::Start),
        (B_PAD_SELECT, GameButton::Select),
        (B_PAD_RIGHT, GameButton::Right),
        (B_PAD_LEFT, GameButton::Left),
        (B_PAD_UP, GameButton::Up),
        (B_PAD_DOWN, GameButton::Down),
    ]
    .into_iter()
    .filter_map(|(bit, button)| (mask & bit != 0).then_some(button))
    .collect()
}

fn compiled_script_boundary(state: &GameState) -> Option<RuntimeCompiledScriptBoundary> {
    if let Some(wait) = &state.script_runtime.pending_text_wait {
        return Some(RuntimeCompiledScriptBoundary::TextWait(wait.clone()));
    }
    if let Some(label) = &state.script_runtime.pending_text_label {
        return Some(RuntimeCompiledScriptBoundary::TextLabel(label.clone()));
    }
    if let Some(prompt) = &state.script_runtime.pending_yes_no {
        return Some(RuntimeCompiledScriptBoundary::YesNo(prompt.clone()));
    }
    if let Some(menu) = &state.script_runtime.active_menu {
        return Some(RuntimeCompiledScriptBoundary::ActiveMenu(menu.clone()));
    }
    if let Some(shop) = &state.script_runtime.pending_shop {
        return Some(RuntimeCompiledScriptBoundary::PendingShop(shop.clone()));
    }
    if let Some(warp) = &state.script_runtime.pending_script_warp {
        return Some(RuntimeCompiledScriptBoundary::PendingScriptWarp(
            warp.clone(),
        ));
    }
    if let Some(load) = &state.script_runtime.pending_map_load {
        return Some(RuntimeCompiledScriptBoundary::PendingMapLoad(load.clone()));
    }
    if let Some(refresh) = &state.script_runtime.pending_map_refresh {
        return Some(RuntimeCompiledScriptBoundary::PendingMapRefresh(
            refresh.clone(),
        ));
    }
    if let Some(delay) = state.script_runtime.pending_delays.first() {
        return Some(RuntimeCompiledScriptBoundary::Delay(delay.clone()));
    }
    if let Some(earthquake) = state.script_runtime.pending_earthquakes.first() {
        return Some(RuntimeCompiledScriptBoundary::Earthquake(
            earthquake.clone(),
        ));
    }
    if let Some(emote) = state.script_runtime.pending_emotes.first() {
        return Some(RuntimeCompiledScriptBoundary::Emote(emote.clone()));
    }
    if state.script_runtime.waiting_for_sound_effect {
        return Some(RuntimeCompiledScriptBoundary::WaitForSoundEffect);
    }
    match &state.battle {
        BattleMemory::Inactive => None,
        BattleMemory::Wild { .. } => Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::WildBattle,
        )),
        BattleMemory::StaticWild { .. } => Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::StaticWildBattle,
        )),
        BattleMemory::Trainer { .. } => Some(RuntimeCompiledScriptBoundary::ActiveBattle(
            RuntimeShellPhase::TrainerBattle,
        )),
    }
}

fn empty_compiled_script_run() -> RuntimeCompiledScriptRun {
    RuntimeCompiledScriptRun {
        steps: Vec::new(),
        next_cursor: None,
        boundary: None,
        ended: false,
    }
}

impl RuntimeGameShell {
    fn new_game(
        asset_root: AssetRoot,
        runtime: CrystalRuntime,
        spawn_identifier: u16,
    ) -> Result<Self> {
        let mut session = runtime
            .start_overworld_session(&asset_root, spawn_identifier)
            .with_context(|| format!("start runtime game shell at spawn {spawn_identifier}"))?;
        // A headless RuntimeGameShell begins at FinishContinue's playable
        // overworld boundary. The visible title/new-game flow clears this
        // again while its pre-overworld sequence is active.
        session.state.set_game_timer_counting(true);
        Ok(Self {
            asset_root,
            runtime,
            session,
            last_frame: None,
            linked_menu_results: Vec::new(),
            runtime_command_sequence: 0,
            runtime_commands: Vec::new(),
            runtime_results: Vec::new(),
            retain_runtime_journal: true,
        })
    }

    #[cfg(any(test, feature = "location-tester"))]
    fn new_game_at_runtime_tile(
        asset_root: AssetRoot,
        runtime: CrystalRuntime,
        spawn_identifier: u16,
        map_name: impl AsRef<str>,
        tile_x: i16,
        tile_y: i16,
    ) -> Result<Self> {
        let map_name = map_name.as_ref();
        let mut session = runtime
            .start_overworld_session_at_runtime_tile(&asset_root, map_name, tile_x, tile_y)
            .with_context(|| {
                format!("start runtime game shell at {map_name} runtime tile ({tile_x}, {tile_y})")
            })?;
        session.state.last_spawn_identifier = Some(spawn_identifier);
        session.state.set_game_timer_counting(true);
        Ok(Self {
            asset_root,
            runtime,
            session,
            last_frame: None,
            linked_menu_results: Vec::new(),
            runtime_command_sequence: 0,
            runtime_commands: Vec::new(),
            runtime_results: Vec::new(),
            retain_runtime_journal: true,
        })
    }

    pub fn resume_from_save(
        asset_root: AssetRoot,
        runtime: CrystalRuntime,
        save_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let state = runtime.load_save(save_path)?;
        let mut session = runtime
            .resume_overworld_session(&asset_root, state)
            .context("resume runtime game shell from save")?;
        // FinishContinueFunction sets GAME_TIMER_COUNTING_F after loading;
        // the WRAM control byte itself is not SRAM-backed.
        session.state.set_game_timer_counting(true);
        session.state.set_game_logic_paused(false);
        Ok(Self {
            asset_root,
            runtime,
            session,
            last_frame: None,
            linked_menu_results: Vec::new(),
            runtime_command_sequence: 0,
            runtime_commands: Vec::new(),
            runtime_results: Vec::new(),
            retain_runtime_journal: true,
        })
    }

    /// Disable retained command/result serialization for a real-time host.
    /// Gameplay state and the per-frame checksum remain authoritative; only
    /// the optional replay journal is omitted.
    pub fn set_runtime_journal_enabled(&mut self, enabled: bool) {
        self.retain_runtime_journal = enabled;
    }

    pub fn tick(
        &mut self,
        buttons: impl IntoIterator<Item = GameButton>,
    ) -> Result<&RuntimeOverworldFrame> {
        self.advance_game_timer_vblank()?;
        self.tick_after_vblank(buttons)
    }

    fn tick_after_vblank(
        &mut self,
        buttons: impl IntoIterator<Item = GameButton>,
    ) -> Result<&RuntimeOverworldFrame> {
        let buttons = buttons.into_iter().collect::<Vec<_>>();
        if !self.retain_runtime_journal {
            let frame = self
                .session
                .apply_overworld_input_live(&self.runtime, buttons)?;
            self.last_frame = Some(frame);
            return self
                .last_frame
                .as_ref()
                .context("runtime shell did not store the live frame it just produced");
        }
        let recorded = self.session.stage_overworld_input(
            &self.runtime,
            buttons,
            self.retain_runtime_journal,
        )?;
        let mutation = self
            .apply_recorded_runtime_mutation(recorded)
            .context("advance runtime game shell")?;
        let RuntimeMutationResult::OverworldInputApplied(frame) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-overworld-input result");
        };
        self.session.joypad = JoypadState::from_previous_mask(frame.input_mask);
        self.last_frame = Some(RuntimeOverworldFrame::from_input_frame(
            frame,
            mutation.state_checksum,
        ));
        self.last_frame
            .as_ref()
            .context("runtime shell did not store the frame it just produced")
    }

    /// Advance one authoritative gameplay frame using an injected RTC sample.
    /// This is the deterministic entry point for replay/oracle adapters; the
    /// ordinary `tick` path remains available for hosts that update the clock
    /// separately.
    pub fn tick_with_rtc(
        &mut self,
        buttons: impl IntoIterator<Item = GameButton>,
        rtc: RuntimeRtcSample,
    ) -> Result<&RuntimeOverworldFrame> {
        self.advance_game_timer_vblank()?;
        self.update_clock_from_datetime(rtc.date, rtc.hour, rtc.minute, rtc.second)?;
        self.tick_after_vblank(buttons)
    }

    pub(crate) fn tick_with_rtc_after_vblank(
        &mut self,
        buttons: impl IntoIterator<Item = GameButton>,
        rtc: RuntimeRtcSample,
    ) -> Result<&RuntimeOverworldFrame> {
        self.update_clock_from_datetime(rtc.date, rtc.hour, rtc.minute, rtc.second)?;
        self.tick_after_vblank(buttons)
    }

    pub fn state_checksum_frame(&self, player_id: PlayerId) -> Result<StateChecksumFrame> {
        self.runtime
            .validate_save_state_for_runtime_pack(self.session.state())
            .context("validate runtime game shell state before checksum")?;
        self.session.state_checksum_frame(player_id)
    }

    pub fn link_session_descriptor(
        &self,
        session_id: impl Into<String>,
        player_id: PlayerId,
        display_name: impl Into<String>,
    ) -> Result<RuntimeLinkSessionDescriptor> {
        let session = LinkSessionIdentity::new(
            session_id,
            self.runtime.modpack().clone(),
            self.runtime.pack_identity().content_hash.clone(),
        )
        .context("build runtime link session identity")?;
        let local_player = PlayerIdentity::new(player_id, display_name)
            .context("build runtime link player identity")?;
        let hello = LinkHello::from_session(session.clone(), local_player.clone())
            .context("build runtime link hello")?;
        let checksum = self.state_checksum_frame(player_id)?;
        let save_checkpoint = self.runtime.session_save_checkpoint_for_state(
            session.clone(),
            self.session.state(),
            player_id,
        )?;
        Ok(RuntimeLinkSessionDescriptor {
            session,
            local_player,
            hello,
            checksum,
            save_checkpoint,
        })
    }

    pub fn validate_link_session_descriptor(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
    ) -> Result<()> {
        validate_link_session_identity(&descriptor.session, descriptor.hello.session())
            .context("runtime link hello session does not match descriptor session")?;
        if descriptor.hello.player() != &descriptor.local_player {
            anyhow::bail!("runtime link hello player does not match descriptor local player");
        }
        if descriptor.checksum.player_id() != descriptor.local_player.id() {
            anyhow::bail!(
                "runtime link checksum player {} does not match local player {}",
                descriptor.checksum.player_id(),
                descriptor.local_player.id()
            );
        }
        if descriptor.save_checkpoint.session() != &descriptor.session {
            anyhow::bail!("runtime link save checkpoint session does not match descriptor session");
        }
        descriptor
            .save_checkpoint
            .validate()
            .context("runtime link save checkpoint is invalid")?;
        let checkpoint = descriptor.save_checkpoint.checkpoint();
        if checkpoint.summary().state_frame() != descriptor.checksum.frame()
            || checkpoint.checksum().frame() != descriptor.checksum.frame()
            || checkpoint.summary().state_hash() != descriptor.checksum.hash()
            || checkpoint.checksum().hash() != descriptor.checksum.hash()
        {
            anyhow::bail!(
                "runtime link save checkpoint frame/hash does not match descriptor checksum: summary {} {:#010x}, checkpoint {} {:#010x}, descriptor {} {:#010x}",
                checkpoint.summary().state_frame(),
                checkpoint.summary().state_hash(),
                checkpoint.checksum().frame(),
                checkpoint.checksum().hash(),
                descriptor.checksum.frame(),
                descriptor.checksum.hash()
            );
        }
        Ok(())
    }

    pub fn link_endpoint<T: crystal_net::LinkTransport>(
        &self,
        transport: T,
        descriptor: &RuntimeLinkSessionDescriptor,
    ) -> Result<crystal_net::LinkEndpoint<T>> {
        self.validate_link_session_descriptor(descriptor)
            .context("validate runtime link descriptor before endpoint creation")?;
        crystal_net::LinkEndpoint::new(transport, descriptor.hello.clone())
            .context("build runtime link endpoint")
    }

    pub fn send_link_save_checkpoint<T: crystal_net::LinkTransport>(
        &self,
        endpoint: &mut crystal_net::LinkEndpoint<T>,
        descriptor: &RuntimeLinkSessionDescriptor,
    ) -> Result<()> {
        self.validate_link_session_descriptor(descriptor)
            .context("validate runtime link descriptor before save checkpoint send")?;
        endpoint
            .send(LinkMessage::SessionSaveCheckpoint(
                descriptor.save_checkpoint.clone(),
            ))
            .context("send runtime link save checkpoint")
    }

    pub fn send_link_bootstrap<T: crystal_net::LinkTransport>(
        &self,
        endpoint: &mut crystal_net::LinkEndpoint<T>,
        descriptor: &RuntimeLinkSessionDescriptor,
    ) -> Result<()> {
        self.validate_link_session_descriptor(descriptor)
            .context("validate runtime link descriptor before bootstrap send")?;
        endpoint.send_hello().context("send runtime link hello")?;
        self.send_link_save_checkpoint(endpoint, descriptor)
    }

    pub fn require_link_checkpoints<T: crystal_net::LinkTransport>(
        &self,
        endpoint: &crystal_net::LinkEndpoint<T>,
        players: impl IntoIterator<Item = PlayerId>,
    ) -> Result<()> {
        endpoint
            .require_checkpoints_for_players(players)
            .context("require runtime link peer save checkpoints")
    }

    pub fn input_journal_from_lockstep_frames(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        players: impl IntoIterator<Item = PlayerId>,
        terminal_checksum: StateChecksumFrame,
        frames: Vec<LockstepFrame>,
    ) -> Result<RuntimeInputJournal> {
        self.validate_link_session_descriptor(descriptor)
            .context("validate runtime link descriptor before journal build")?;
        let journal = DeterministicInputJournal::new(
            descriptor.session.clone(),
            players,
            descriptor.checksum.clone(),
            terminal_checksum.clone(),
            frames,
        )
        .context("build deterministic runtime input journal")?;
        Ok(RuntimeInputJournal {
            journal,
            terminal_checksum,
        })
    }

    pub fn local_input_journal(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        terminal_checksum: StateChecksumFrame,
        inputs: impl IntoIterator<Item = (u64, u8)>,
    ) -> Result<RuntimeInputJournal> {
        let player_id = descriptor.local_player.id();
        let mut frames = Vec::new();
        for (frame, joypad_mask) in inputs {
            frames.push(
                LockstepFrame::new(frame, BTreeMap::from([(player_id, joypad_mask)]))
                    .context("build local runtime lockstep input frame")?,
            );
        }
        self.input_journal_from_lockstep_frames(descriptor, [player_id], terminal_checksum, frames)
    }

    pub fn record_local_input_journal(
        &mut self,
        descriptor: &RuntimeLinkSessionDescriptor,
        inputs: impl IntoIterator<Item = Vec<GameButton>>,
    ) -> Result<RuntimeInputJournal> {
        let player_id = descriptor.local_player.id();
        let mut next_frame = descriptor.checksum.frame();
        let mut frames = Vec::new();
        for buttons in inputs {
            let applied = self.tick(buttons)?;
            frames.push(
                LockstepFrame::new(
                    next_frame,
                    BTreeMap::from([(player_id, applied.input_mask)]),
                )
                .context("record runtime lockstep input frame")?,
            );
            next_frame = next_frame.checked_add(1).with_context(|| {
                format!("runtime input journal frame cursor overflowed at frame {next_frame}")
            })?;
        }
        let terminal_checksum = self.state_checksum_frame(player_id)?;
        self.input_journal_from_lockstep_frames(descriptor, [player_id], terminal_checksum, frames)
    }

    pub fn apply_deterministic_replay_bundle(
        &mut self,
        descriptor: &RuntimeLinkSessionDescriptor,
        bundle: &DeterministicReplayBundle,
    ) -> Result<RuntimeInputJournal> {
        validate_deterministic_replay_runtime_authority(bundle, descriptor.local_player.id())?;
        let journal = bundle.input_journal().journal();
        self.validate_local_input_journal_start(descriptor, &journal)?;
        let player_id = descriptor.local_player.id();
        let previous = self.clone();
        let original_divider = self.session.divider.clone();
        let replay = (|| {
            // No command in a deterministic bundle may sample the host DIV.
            // Trace-bearing commands construct their own ReplayDivider; an
            // accidental legacy read therefore fails closed here.
            self.session.divider = RuntimeDividerSource::replay([]);
            for (command, expected_result) in bundle
                .runtime_commands()
                .iter()
                .zip(bundle.runtime_results())
            {
                let request = command.command();
                let result_index = self.runtime_results.len();
                self.apply_runtime_command_frame(request).with_context(|| {
                    format!(
                        "apply deterministic runtime command sequence {}",
                        request.sequence()
                    )
                })?;
                let actual_result = self.runtime_results.get(result_index).with_context(|| {
                    format!(
                        "runtime command sequence {} did not retain its generated result",
                        request.sequence()
                    )
                })?;
                if actual_result != expected_result.result() {
                    anyhow::bail!(
                        "generated result for runtime command sequence {} does not match the deterministic bundle",
                        request.sequence()
                    );
                }
            }
            let terminal_checksum = self.state_checksum_frame(player_id)?;
            if &terminal_checksum != bundle.terminal_checksum() {
                anyhow::bail!(
                    "deterministic runtime replay terminal checksum does not match the bundle"
                );
            }
            self.session.divider = original_divider;
            Ok(RuntimeInputJournal {
                journal: journal.clone(),
                terminal_checksum,
            })
        })();
        if replay.is_err() {
            *self = previous;
        }
        replay
    }

    pub fn validate_local_input_journal_start(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        journal: &DeterministicInputJournal,
    ) -> Result<()> {
        self.validate_link_session_descriptor(descriptor)
            .context("validate runtime link descriptor before journal start validation")?;
        journal
            .validate()
            .context("validate deterministic runtime input journal")?;
        validate_link_session_identity(&descriptor.session, journal.session())
            .context("validate runtime input journal session")?;
        if journal.start_checksum() != &descriptor.checksum {
            anyhow::bail!("runtime input journal start checksum does not match descriptor");
        }
        let current_checksum = self.state_checksum_frame(descriptor.local_player.id())?;
        if current_checksum != descriptor.checksum {
            anyhow::bail!(
                "runtime input journal start checksum frame/hash {} {:#010x} does not match current state {} {:#010x}",
                descriptor.checksum.frame(),
                descriptor.checksum.hash(),
                current_checksum.frame(),
                current_checksum.hash()
            );
        }
        Ok(())
    }

    pub fn input_journal_message(&self, journal: RuntimeInputJournal) -> Result<LinkMessage> {
        Ok(LinkMessage::InputJournal(
            DeterministicInputJournalFrame::new(journal.journal)
                .context("build runtime input journal frame")?,
        ))
    }

    pub fn save_resume_replay_bundle(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        journal: RuntimeInputJournal,
        runtime_commands: Vec<SessionRuntimeCommandFrame>,
        runtime_results: Vec<SessionRuntimeCommandResultFrame>,
        menu_results: Vec<MenuChoiceResultFrame>,
    ) -> Result<SaveResumeReplayBundle> {
        self.validate_link_session_descriptor(descriptor)
            .context("validate runtime link descriptor before save-resume replay")?;
        let journal_frame = DeterministicInputJournalFrame::new(journal.journal)
            .context("build runtime save-resume input journal frame")?;
        let replay = DeterministicReplayBundle::new(
            journal_frame,
            runtime_commands,
            runtime_results,
            menu_results,
            journal.terminal_checksum,
        )
        .context("build runtime deterministic replay bundle")?;
        validate_deterministic_replay_runtime_authority(&replay, descriptor.local_player.id())
            .context("validate runtime replay command authority before send")?;
        SaveResumeReplayBundle::new(descriptor.save_checkpoint.clone(), replay)
            .context("build runtime save-resume replay bundle")
    }

    pub fn save_resume_replay_message(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        journal: RuntimeInputJournal,
        runtime_commands: Vec<SessionRuntimeCommandFrame>,
        runtime_results: Vec<SessionRuntimeCommandResultFrame>,
        menu_results: Vec<MenuChoiceResultFrame>,
    ) -> Result<LinkMessage> {
        Ok(LinkMessage::SaveResumeReplay(
            self.save_resume_replay_bundle(
                descriptor,
                journal,
                runtime_commands,
                runtime_results,
                menu_results,
            )?,
        ))
    }

    pub fn linked_menu_results(&self) -> &[MenuChoiceResultFrame] {
        &self.linked_menu_results
    }

    pub fn drain_linked_menu_results(&mut self) -> Vec<MenuChoiceResultFrame> {
        std::mem::take(&mut self.linked_menu_results)
    }

    pub fn retained_runtime_commands(&self) -> &[RuntimeCommandFrame] {
        &self.runtime_commands
    }

    pub fn retained_runtime_results(&self) -> &[RuntimeCommandResultFrame] {
        &self.runtime_results
    }

    pub fn clear_retained_runtime_commands(&mut self) {
        self.runtime_command_sequence = 0;
        self.runtime_commands.clear();
        self.runtime_results.clear();
    }

    fn require_valid_script_modal_state(&self, action: &str) -> Result<()> {
        self.session
            .state
            .script_runtime
            .validate()
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("cannot {action} from invalid script modal state"))
    }

    pub fn runtime_command_frame(
        &self,
        player_id: PlayerId,
        sequence: u64,
        command: RuntimeMutationCommand,
    ) -> Result<RuntimeCommandFrame> {
        self.session
            .runtime_command_frame(player_id, sequence, command)
    }

    pub fn require_runtime_command_expected_state(
        &self,
        request: &RuntimeCommandFrame,
    ) -> Result<()> {
        self.session.require_runtime_command_expected_state(request)
    }

    pub fn runtime_mutation_result_frame(
        &self,
        request: RuntimeCommandFrame,
        outcome: &RuntimeMutationOutcome,
    ) -> Result<RuntimeCommandResultFrame> {
        self.session.runtime_mutation_result_frame(request, outcome)
    }

    pub fn apply_runtime_mutation_command(
        &mut self,
        command: RuntimeMutationCommand,
    ) -> Result<RuntimeMutationOutcome> {
        self.require_valid_script_modal_state("apply runtime mutation command")?;
        if !self.retain_runtime_journal {
            if let RuntimeMutationCommand::AdvanceGameTimerVBlanks(command) = command.clone() {
                let outcome = self
                    .runtime
                    .data
                    .advance_game_timer_vblanks_fast(
                        &mut self.session.state,
                        &mut self.session.overworld,
                        command.vblanks,
                        &self.runtime.audio.music_ids(),
                        &self.runtime.audio.sound_effect_ids(),
                        &self.runtime.audio.cry_ids(),
                    )
                    .context("advance runtime game timer VBlank")?;
                self.record_runtime_mutation_outcome(&outcome);
                return Ok(outcome);
            }
        }
        let sequence = self
            .runtime_command_sequence
            .checked_add(1)
            .context("runtime command sequence overflow")?;
        let request = self
            .runtime_command_frame(RUNTIME_LOCAL_PLAYER_ID, sequence, command.clone())
            .context("build retained runtime command frame")?;
        // Local input uses the single-execution recorded path. Remote input
        // replays an explicit divider trace and must roll back state if result
        // framing/checksum validation fails after the asset-layer transaction.
        let transactional = !matches!(command, RuntimeMutationCommand::AdvanceGameTimerVBlanks(_));
        let previous_session = transactional.then(|| self.session.clone());
        let outcome = match self
            .session
            .apply_runtime_mutation_command(&self.runtime, command)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(previous_session) = previous_session {
                    self.session = previous_session;
                }
                return Err(error);
            }
        };
        let result = match self
            .runtime_mutation_result_frame(request.clone(), &outcome)
            .context("build retained runtime command result frame")
        {
            Ok(result) => result,
            Err(error) => {
                if let Some(previous_session) = previous_session {
                    self.session = previous_session;
                }
                return Err(error);
            }
        };
        if let Err(error) = self.require_valid_script_modal_state("finish runtime mutation command")
        {
            if let Some(previous_session) = previous_session {
                self.session = previous_session;
            }
            return Err(error);
        }
        self.runtime_command_sequence = sequence;
        self.runtime_commands.push(request);
        self.runtime_results.push(result);
        self.record_runtime_mutation_outcome(&outcome);
        Ok(outcome)
    }

    fn apply_recorded_runtime_mutation(
        &mut self,
        recorded: RecordedRuntimeMutation,
    ) -> Result<RuntimeMutationOutcome> {
        self.require_valid_script_modal_state("apply recorded runtime mutation")?;
        if !self.retain_runtime_journal {
            let outcome = self.session.commit_recorded_mutation(recorded);
            self.record_runtime_mutation_outcome(&outcome);
            return Ok(outcome);
        }
        let sequence = self
            .runtime_command_sequence
            .checked_add(1)
            .context("runtime command sequence overflow")?;
        // The command is formed only after the single staged execution has
        // captured its exact DIV reads, but its expected checksum is still
        // computed against the untouched pre-mutation session.
        let request = self
            .runtime_command_frame(RUNTIME_LOCAL_PLAYER_ID, sequence, recorded.command.clone())
            .context("build recorded runtime command frame")?;
        let previous_session = self.session.clone();
        let outcome = self.session.commit_recorded_mutation(recorded);
        let result = match self
            .runtime_mutation_result_frame(request.clone(), &outcome)
            .context("build recorded runtime command result frame")
        {
            Ok(result) => result,
            Err(error) => {
                self.session = previous_session;
                return Err(error);
            }
        };
        if let Err(error) =
            self.require_valid_script_modal_state("finish recorded runtime mutation")
        {
            self.session = previous_session;
            return Err(error);
        }
        self.runtime_command_sequence = sequence;
        self.runtime_commands.push(request);
        self.runtime_results.push(result);
        self.record_runtime_mutation_outcome(&outcome);
        Ok(outcome)
    }

    fn apply_special_routine_runtime_mutation(
        &mut self,
        routine: &str,
    ) -> Result<RuntimeMutationOutcome> {
        if runtime_special_routine_requires_divider_trace(routine) {
            let recorded = self
                .session
                .stage_random_special_routine(&self.runtime, routine)?;
            return self.apply_recorded_runtime_mutation(recorded);
        }
        let rng_seed_after = preview_generic_special_routine_rng_seed_after(
            self.runtime.data(),
            &self.session.state,
            &self.runtime.music_ids(),
            routine,
        )?;
        self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplySpecialRoutine {
            routine: routine.to_string(),
            rng_seed_after,
        })
    }

    pub fn apply_compiled_script_command(
        &mut self,
        origin_map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeMutationOutcome> {
        let map_name = origin_map_name.to_string();
        let command = self
            .runtime
            .compiled_script_command_name(source_script, command_index)?;
        let command_ref = RuntimeScriptCommandRef::new(&map_name, source_script, command_index);
        let is_gift_pokemon_command =
            self.runtime
                .has_gift_pokemon_command_at(&map_name, source_script, command_index);
        if !is_gift_pokemon_command {
            reject_unexpected_gift_pokemon_inputs(source_script, command_index, &command, &inputs)?;
        }
        if self
            .runtime
            .data()
            .is_exact_rock_mon_encounter_command(&command_ref)?
        {
            let recorded = self
                .session
                .stage_rock_mon_encounter(&self.runtime, command_ref)?;
            return self.apply_recorded_runtime_mutation(recorded);
        }
        let is_scripted_wild_battle_start = self.runtime.has_scripted_wild_battle_start_command_at(
            &map_name,
            source_script,
            command_index,
        ) || self
            .runtime
            .data()
            .is_exact_rock_smash_dynamic_start_command(&command_ref)?;
        if is_scripted_wild_battle_start {
            let recorded = self
                .session
                .stage_scripted_wild_battle_start(&self.runtime, command_ref)?;
            return self.apply_recorded_runtime_mutation(recorded);
        }
        let mutation = if self.runtime.has_scripted_trainer_battle_start_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            if command == "startbattle" {
                self.runtime.data().require_scripted_trainer_battle_setup(
                    self.session.state(),
                    &map_name,
                    source_script,
                    command_index,
                )?;
            }
            RuntimeMutationCommand::StartScriptedTrainerBattle(command_ref)
        } else if is_gift_pokemon_command {
            RuntimeMutationCommand::GrantScriptedGiftPokemon(RuntimeGiftPokemonCommand {
                command: command_ref,
                original_trainer_name: inputs.gift_original_trainer_name.clone().with_context(
                    || {
                        format!(
                            "compiled gift Pokemon command {}:{} requires gift_original_trainer_name input",
                            source_script, command_index
                        )
                    },
                )?,
                original_trainer_id: inputs.gift_original_trainer_id.with_context(|| {
                    format!(
                        "compiled gift Pokemon command {}:{} requires gift_original_trainer_id input",
                        source_script, command_index
                    )
                })?,
                dvs: inputs.gift_dvs.with_context(|| {
                    format!(
                        "compiled gift Pokemon command {}:{} requires gift_dvs input",
                        source_script, command_index
                    )
                })?,
                rng_seed_after: inputs.gift_rng_seed_after.with_context(|| {
                    format!(
                        "compiled gift Pokemon command {}:{} requires gift_rng_seed_after input",
                        source_script, command_index
                    )
                })?,
                nickname_accepted: inputs.gift_nickname_accepted.with_context(|| {
                    format!(
                        "compiled gift Pokemon command {}:{} requires gift_nickname_accepted input",
                        source_script, command_index
                    )
                })?,
                nickname: inputs.gift_nickname.clone(),
            })
        } else if self.runtime.has_script_item_grant_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::GrantScriptItem(command_ref)
        } else if self.runtime.has_script_item_check_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::CheckScriptItem(command_ref)
        } else if self.runtime.has_script_item_take_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::TakeScriptItem(command_ref)
        } else if self.runtime.has_script_field_pickup_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::PickupScriptFieldItem(command_ref)
        } else if self.runtime.has_script_economy_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::ApplyScriptEconomy(command_ref)
        } else if self
            .runtime
            .has_script_flag_command_at(&map_name, source_script, command_index)
        {
            match command.as_str() {
                "checkevent" | "checkflag" | "check_flag" => {
                    RuntimeMutationCommand::CheckScriptFlag(command_ref)
                }
                _ => RuntimeMutationCommand::ApplyScriptFlagMutation(command_ref),
            }
        } else if self
            .runtime
            .has_script_scene_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptScene(command_ref)
        } else if self.runtime.has_script_block_change_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::ApplyScriptBlockChange(command_ref)
        } else if self
            .runtime
            .has_script_audio_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptAudio(command_ref)
        } else if self
            .runtime
            .has_script_map_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptMap(command_ref)
        } else if self
            .runtime
            .has_script_text_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptText(command_ref)
        } else if self.runtime.has_script_variable_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::ApplyScriptVariableNow(command_ref)
        } else if self
            .runtime
            .has_script_swarm_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptSwarm(command_ref)
        } else if self
            .runtime
            .has_script_phone_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptPhone {
                command: command_ref,
                inputs: phone_inputs,
            }
        } else if self.runtime.has_script_control_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::ApplyScriptControl(command_ref)
        } else if self.runtime.has_script_movement_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            RuntimeMutationCommand::ApplyScriptMovement(command_ref)
        } else if self
            .runtime
            .has_script_object_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::ApplyScriptObjectMutation(command_ref)
        } else if self.runtime.has_script_runtime_command_at(
            &map_name,
            source_script,
            command_index,
        ) {
            if command == "special" {
                let routine = self
                    .runtime
                    .script_runtime_command_at(&map_name, source_script, command_index)
                    .filter(|key| key.command == "special")
                    .and_then(|key| key.args.first().cloned())
                    .with_context(|| {
                        format!(
                            "compiled script command {}:{} special is missing routine id",
                            source_script, command_index
                        )
                    })?;
                if runtime_special_routine_requires_divider_trace(&routine) {
                    return self.apply_special_routine_runtime_mutation(&routine);
                }
                let rng_seed_after = preview_generic_special_routine_rng_seed_after(
                    self.runtime.data(),
                    &self.session.state,
                    &self.runtime.music_ids(),
                    &routine,
                )?;
                RuntimeMutationCommand::ApplySpecialRoutine {
                    routine,
                    rng_seed_after,
                }
            } else {
                RuntimeMutationCommand::ApplyScriptRuntime {
                    command: command_ref,
                    inputs,
                }
            }
        } else if self
            .runtime
            .has_script_shop_command_at(&map_name, source_script, command_index)
        {
            RuntimeMutationCommand::OpenScriptShop(command_ref)
        } else {
            anyhow::bail!(
                "compiled script command {}:{} '{}' has no Rust runtime mutation",
                source_script,
                command_index,
                command
            );
        };
        self.apply_runtime_mutation_command(mutation)
            .with_context(|| {
                format!("apply compiled script command {source_script}:{command_index} '{command}'")
            })
    }

    pub fn step_compiled_script_command(
        &mut self,
        origin_map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeCompiledScriptStep> {
        if !self.runtime.data().maps.contains_key(origin_map_name) {
            anyhow::bail!(
                "compiled script cursor origin map {origin_map_name} is missing from the pack"
            );
        }
        let command = self
            .runtime
            .compiled_script_command_name(source_script, command_index)?;
        let command_count = self.runtime.compiled_script_commands(source_script)?.len();
        let mutation = self.apply_compiled_script_command(
            origin_map_name,
            source_script,
            command_index,
            inputs,
            phone_inputs,
        )?;
        let mut next_script = self
            .session
            .state()
            .script_runtime
            .next_script
            .as_ref()
            .filter(|next| next.script != source_script)
            .cloned();
        let mut ended = self.session.state().script_runtime.script_ended.is_some();
        if let RuntimeMutationResult::ScriptControlApplied(action) = &mutation.result {
            match action {
                ScriptControlAction::Jump {
                    target_script,
                    deferred,
                    ..
                } => {
                    if !deferred {
                        next_script = Some(ScriptLocation {
                            origin_map_name: origin_map_name.to_string(),
                            script: target_script.clone(),
                        });
                    }
                    ended = false;
                }
                ScriptControlAction::End { .. } => {
                    next_script = None;
                    ended = true;
                }
                ScriptControlAction::Continue { .. } => {}
            }
        }
        let next_cursor = if let Some(next_script) = &next_script {
            Some(RuntimeCompiledScriptCursor {
                origin_map_name: next_script.origin_map_name.clone(),
                source_script: next_script.script.clone(),
                command_index: 0,
            })
        } else if ended || command_index + 1 >= command_count {
            None
        } else {
            Some(RuntimeCompiledScriptCursor {
                origin_map_name: origin_map_name.to_string(),
                source_script: source_script.to_string(),
                command_index: command_index + 1,
            })
        };
        let boundary = compiled_script_boundary(self.session.state()).or_else(|| {
            matches!(
                mutation.result,
                RuntimeMutationResult::ScriptMovementApplied(_)
            )
            .then_some(RuntimeCompiledScriptBoundary::ScriptMovement)
        });

        Ok(RuntimeCompiledScriptStep {
            origin_map_name: origin_map_name.to_string(),
            source_script: source_script.to_string(),
            command_index,
            command,
            mutation,
            next_cursor,
            boundary,
            next_script: next_script.map(|next| next.script),
            ended,
        })
    }

    pub fn compiled_script_runtime_inputs(
        &self,
        origin_map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<ScriptRuntimeInputs> {
        let command_name = self
            .runtime
            .compiled_script_command_name(source_script, command_index)?;
        let map_name = origin_map_name;
        let runtime_command =
            self.runtime
                .script_runtime_command_at(&map_name, source_script, command_index);
        let (random_value, rng_seed_after) = if command_name == "random" {
            let command = runtime_command.as_ref().with_context(|| {
                format!(
                    "compiled random command {}:{} is missing runtime command metadata",
                    source_script, command_index
                )
            })?;
            let bound_token = command
                .args
                .first()
                .with_context(|| "compiled random command is missing bound argument")?;
            let bound = bound_token
                .parse::<u32>()
                .with_context(|| format!("compiled random bound '{bound_token}' is not a u32"))?;
            if bound == 0 {
                anyhow::bail!("compiled random command has zero bound");
            }
            let mut rng = Random::new_crystal(self.session.state().rng_seed);
            let value = rng.randrange(bound);
            (Some(value), Some(rng.seed()))
        } else {
            (None, None)
        };
        let game_version = if command_name == "checkver" {
            Some("0".to_string())
        } else {
            None
        };
        let (
            gift_original_trainer_name,
            gift_original_trainer_id,
            gift_dvs,
            gift_rng_seed_after,
            gift_nickname_accepted,
            gift_nickname,
        ) = if self
            .runtime
            .has_gift_pokemon_command_at(&map_name, source_script, command_index)
        {
            let state = self.session.state();
            if state.player_name.is_empty() {
                anyhow::bail!(
                    "compiled gift Pokemon command {}:{} requires player identity before gift inputs can be generated",
                    source_script,
                    command_index
                );
            }
            let mut rng = Random::new_crystal(state.rng_seed);
            let dvs = Dv::from_non_hp(
                rng.randrange(16) as u8,
                rng.randrange(16) as u8,
                rng.randrange(16) as u8,
                rng.randrange(16) as u8,
            );
            (
                Some(state.player_name.clone()),
                Some(state.player_id),
                Some(dvs),
                Some(rng.seed()),
                Some(false),
                None,
            )
        } else {
            (None, None, None, None, None, None)
        };
        Ok(ScriptRuntimeInputs {
            random_value,
            rng_seed_after,
            game_version,
            gift_original_trainer_name,
            gift_original_trainer_id,
            gift_dvs,
            gift_rng_seed_after,
            gift_nickname_accepted,
            gift_nickname,
            ..ScriptRuntimeInputs::default()
        })
    }

    pub fn run_compiled_script_until_boundary(
        &mut self,
        start: RuntimeCompiledScriptCursor,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeCompiledScriptRun> {
        if max_steps == 0 {
            anyhow::bail!("compiled script runner requires max_steps greater than zero");
        }
        let mut cursor = Some(start);
        let mut steps = Vec::new();
        while let Some(current) = cursor.take() {
            let follows_pending_next_script = !steps.is_empty()
                && self
                    .session
                    .state()
                    .script_runtime
                    .next_script
                    .as_ref()
                    .is_some_and(|next| {
                        next.origin_map_name == current.origin_map_name
                            && next.script == current.source_script
                    });
            if follows_pending_next_script {
                self.apply_runtime_mutation_command(RuntimeMutationCommand::TakeNextScript)
                    .with_context(|| {
                        format!(
                            "consume followed script transition {}:{}",
                            current.origin_map_name, current.source_script
                        )
                    })?;
            }
            if steps.len() >= max_steps {
                anyhow::bail!("compiled script runner exceeded max_steps {max_steps}");
            }
            let day_of_week_prompt = self
                .runtime
                .compiled_script_commands(&current.source_script)?
                .get(current.command_index)
                .is_some_and(|command| {
                    command.get("command").and_then(serde_json::Value::as_str) == Some("special")
                        && command
                            .get("args")
                            .and_then(serde_json::Value::as_array)
                            .and_then(|args| args.first())
                            .and_then(serde_json::Value::as_str)
                            == Some("SetDayOfWeek")
                });
            if day_of_week_prompt {
                return Ok(RuntimeCompiledScriptRun {
                    steps,
                    next_cursor: Some(current),
                    boundary: Some(RuntimeCompiledScriptBoundary::DayOfWeekPrompt),
                    ended: false,
                });
            }
            let step_inputs = if steps.is_empty() && inputs != ScriptRuntimeInputs::default() {
                inputs.clone()
            } else {
                self.compiled_script_runtime_inputs(
                    &current.origin_map_name,
                    &current.source_script,
                    current.command_index,
                )?
            };
            let step = self
                .step_compiled_script_command(
                    &current.origin_map_name,
                    &current.source_script,
                    current.command_index,
                    step_inputs,
                    phone_inputs.clone(),
                )
                .with_context(|| {
                    format!(
                        "run compiled script {}:{}:{}",
                        current.origin_map_name, current.source_script, current.command_index
                    )
                })?;
            let boundary = step.boundary.clone();
            let ended = step.ended;
            cursor = step.next_cursor.clone();
            steps.push(step);
            if ended && !self.session.state().script_runtime.call_stack.is_empty() {
                // ScriptEvents returns from an `scall`/`farscall` frame when
                // the called command stream ends. A composed run must do the
                // same instead of exposing the callee's end as the caller's
                // terminal boundary.
                self.take_script_end_state()
                    .context("consume called script end before returning")?;
                let returned = self
                    .pop_script_call_stack()
                    .context("resume compiled script call frame")?;
                cursor = Some(RuntimeCompiledScriptCursor {
                    origin_map_name: returned.frame.origin_map_name,
                    source_script: returned.frame.source_script,
                    command_index: returned.frame.next_command_index,
                });
                continue;
            }
            // `writetext`/`farwritetext` publish a label for presentation but
            // do not pause ScriptEvents. Keep executing until an actual ASM
            // input or modal command establishes the boundary. Treating the
            // label itself as a pause makes the shell resume the same text a
            // second time before `promptbutton`/`yesorno`, and can strand the
            // eventual wait behind a textbox the shell already closed.
            if compiled_script_boundary_stops_run(
                &steps.last().expect("pushed script step").command,
                &boundary,
            ) || ended
            {
                return Ok(RuntimeCompiledScriptRun {
                    steps,
                    next_cursor: cursor,
                    boundary,
                    ended,
                });
            }
        }
        Ok(RuntimeCompiledScriptRun {
            steps,
            next_cursor: None,
            boundary: None,
            ended: false,
        })
    }

    pub fn run_next_queued_script_until_boundary(
        &mut self,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeQueuedCompiledScriptRun> {
        let queued = self.execute_next_queued_script_command()?;
        let run = self.run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: queued.queued.origin_map_name.clone(),
                source_script: queued.queued.target.clone(),
                command_index: 0,
            },
            max_steps,
            inputs,
            phone_inputs,
        )?;
        Ok(RuntimeQueuedCompiledScriptRun { queued, run })
    }

    pub fn run_pending_next_script_until_boundary(
        &mut self,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimePendingCompiledScriptRun> {
        let next_script = self.take_next_script()?;
        let run = self.run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: next_script.origin_map_name.clone(),
                source_script: next_script.script.clone(),
                command_index: 0,
            },
            max_steps,
            inputs,
            phone_inputs,
        )?;
        Ok(RuntimePendingCompiledScriptRun { next_script, run })
    }

    pub fn run_next_deferred_script_until_boundary(
        &mut self,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeDeferredCompiledScriptRun> {
        let deferred_script = self.pop_deferred_script()?;
        let run = self.run_compiled_script_until_boundary(
            RuntimeCompiledScriptCursor {
                origin_map_name: deferred_script.origin_map_name.clone(),
                source_script: deferred_script.script.clone(),
                command_index: 0,
            },
            max_steps,
            inputs,
            phone_inputs,
        )?;
        Ok(RuntimeDeferredCompiledScriptRun {
            deferred_script,
            run,
        })
    }

    pub fn advance_text_wait_and_run_compiled_script(
        &mut self,
        next_cursor: Option<RuntimeCompiledScriptCursor>,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeTextWaitCompiledScriptRun> {
        let wait = self.advance_pending_text_wait()?;
        let run = if let Some(cursor) = next_cursor {
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeTextWaitCompiledScriptRun { wait, run })
    }

    pub fn resolve_yes_no_and_run_compiled_script(
        &mut self,
        accepted: bool,
        next_cursor: Option<RuntimeCompiledScriptCursor>,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeYesNoCompiledScriptRun> {
        let resolution = self.resolve_pending_yes_no(accepted)?;
        let run = if let Some(cursor) = next_cursor {
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeYesNoCompiledScriptRun { resolution, run })
    }

    pub fn resolve_phone_prompt_and_run_compiled_script(
        &mut self,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
        accepted: bool,
        max_steps: usize,
    ) -> Result<RuntimePhonePromptCompiledScriptRun> {
        let origin_map_name = self.session.overworld.map.name.clone();
        let step = self.step_compiled_script_command(
            &origin_map_name,
            source_script,
            command_index,
            inputs,
            ScriptPhoneInputs {
                accepted: Some(accepted),
            },
        )?;
        let RuntimeMutationResult::ScriptPhoneApplied(_) = &step.mutation.result else {
            anyhow::bail!(
                "compiled script command {source_script}:{command_index} did not resolve a phone prompt"
            );
        };
        let run = if let Some(cursor) = step.next_cursor.clone() {
            self.run_compiled_script_until_boundary(
                cursor,
                max_steps,
                ScriptRuntimeInputs::default(),
                ScriptPhoneInputs::default(),
            )?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimePhonePromptCompiledScriptRun { step, run })
    }

    pub fn select_vertical_menu_option_and_run_compiled_script(
        &mut self,
        menu_id: impl Into<String>,
        source_script: impl Into<String>,
        verticalmenu_command_index: usize,
        option_index: usize,
        option: impl Into<String>,
        next_cursor: Option<RuntimeCompiledScriptCursor>,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeMenuSelectionCompiledScriptRun> {
        let selection = self.select_vertical_menu_option(
            menu_id,
            source_script,
            verticalmenu_command_index,
            option_index,
            option,
        )?;
        let run = if let Some(cursor) = next_cursor {
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeMenuSelectionCompiledScriptRun { selection, run })
    }

    pub fn select_elevator_floor_and_run_compiled_script(
        &mut self,
        map_name: impl Into<String>,
        data_label: impl Into<String>,
        source_script: impl Into<String>,
        elevator_command_index: usize,
        floor_index: usize,
        floor: impl Into<String>,
        warp: u16,
        target_map: impl Into<String>,
        next_cursor: Option<RuntimeCompiledScriptCursor>,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeElevatorFloorCompiledScriptRun> {
        let selection = self.select_elevator_floor(
            map_name,
            data_label,
            source_script,
            elevator_command_index,
            floor_index,
            floor,
            warp,
            target_map,
        )?;
        let run = if let Some(cursor) = next_cursor {
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeElevatorFloorCompiledScriptRun { selection, run })
    }

    pub fn transition_script_warp_and_run_compiled_script(
        &mut self,
        next_cursor: Option<RuntimeCompiledScriptCursor>,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeScriptWarpCompiledScriptRun> {
        let warp = self.execute_pending_script_warp()?;
        let run = if let Some(cursor) = next_cursor {
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeScriptWarpCompiledScriptRun { warp, run })
    }

    pub fn complete_scripted_wild_battle_and_run_compiled_script(
        &mut self,
        origin: RuntimeStaticWildBattleOrigin,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeScriptedWildBattleCompiledScriptRun> {
        let cursor = RuntimeCompiledScriptCursor {
            origin_map_name: origin.map_name.clone(),
            source_script: origin.source_script.clone(),
            command_index: origin.resume_command_index,
        };
        let completion = self.complete_scripted_wild_battle(origin)?;
        let run =
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?;
        Ok(RuntimeScriptedWildBattleCompiledScriptRun { completion, run })
    }

    pub fn complete_scripted_trainer_battle_and_run_compiled_script(
        &mut self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
        won: bool,
        can_lose: bool,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeScriptedTrainerBattleCompiledScriptRun> {
        let trainer_callback =
            self.snapshot()?
                .battle
                .as_ref()
                .and_then(|battle| match &battle.kind {
                    RuntimeBattleKind::Trainer { callback, .. } if !callback.is_empty() => {
                        Some(callback.clone())
                    }
                    _ => None,
                });
        let completion = self.complete_scripted_trainer_battle(
            map_name,
            source_script,
            startbattle_command_index,
            won,
            can_lose,
        )?;
        let run = if completion.continued_after_battle {
            let (resume_script, command_index) = if let Some(callback) = trainer_callback {
                (callback, 0)
            } else {
                (
                    source_script.to_string(),
                    startbattle_command_index
                        .checked_add(1)
                        .context("scripted trainer startbattle command index overflow")?,
                )
            };
            self.run_compiled_script_until_boundary(
                RuntimeCompiledScriptCursor {
                    origin_map_name: map_name.to_string(),
                    source_script: resume_script,
                    command_index,
                },
                max_steps,
                inputs,
                phone_inputs,
            )?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeScriptedTrainerBattleCompiledScriptRun { completion, run })
    }

    pub fn grant_compiled_gift_pokemon_command(
        &mut self,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        rng_seed_after: u32,
        nickname_accepted: bool,
        nickname: Option<String>,
    ) -> Result<RuntimeGiftPokemonGrant> {
        let map_name = self.runtime.script_owner_map(source_script)?;
        if !self
            .runtime
            .has_gift_pokemon_command_at(&map_name, source_script, command_index)
        {
            let command = self
                .runtime
                .compiled_script_command_name(source_script, command_index)?;
            anyhow::bail!(
                "compiled script command {}:{} '{}' is not a gift Pokemon command",
                source_script,
                command_index,
                command
            );
        }
        self.grant_scripted_gift_pokemon(
            &map_name,
            source_script,
            command_index,
            original_trainer_name,
            original_trainer_id,
            dvs,
            rng_seed_after,
            nickname_accepted,
            nickname,
        )
    }

    pub fn grant_compiled_gift_pokemon_command_and_run_compiled_script(
        &mut self,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        rng_seed_after: u32,
        nickname_accepted: bool,
        nickname: Option<String>,
        next_cursor: Option<RuntimeCompiledScriptCursor>,
        max_steps: usize,
        inputs: ScriptRuntimeInputs,
        phone_inputs: ScriptPhoneInputs,
    ) -> Result<RuntimeGiftPokemonCompiledScriptRun> {
        let grant = self.grant_compiled_gift_pokemon_command(
            source_script,
            command_index,
            original_trainer_name,
            original_trainer_id,
            dvs,
            rng_seed_after,
            nickname_accepted,
            nickname,
        )?;
        let run = if let Some(cursor) = next_cursor {
            self.run_compiled_script_until_boundary(cursor, max_steps, inputs, phone_inputs)?
        } else {
            empty_compiled_script_run()
        };
        Ok(RuntimeGiftPokemonCompiledScriptRun { grant, run })
    }

    pub fn apply_runtime_command_frame(
        &mut self,
        request: &RuntimeCommandFrame,
    ) -> Result<RuntimeMutationOutcome> {
        if request.player_id() != RUNTIME_LOCAL_PLAYER_ID {
            anyhow::bail!(
                "runtime command player {} does not match local player {}",
                request.player_id(),
                RUNTIME_LOCAL_PLAYER_ID
            );
        }
        let expected_sequence = self
            .runtime_command_sequence
            .checked_add(1)
            .context("runtime command sequence overflow")?;
        if request.sequence() != expected_sequence {
            anyhow::bail!(
                "runtime command sequence {} does not match next retained sequence {}",
                request.sequence(),
                expected_sequence
            );
        }
        self.require_valid_script_modal_state("apply runtime command frame")?;
        let previous_session = self.session.clone();
        let outcome = match self
            .session
            .apply_runtime_command_frame(&self.runtime, request)
        {
            Ok(outcome) => outcome,
            Err(error) => {
                self.session = previous_session;
                return Err(error);
            }
        };
        let result = match self
            .runtime_mutation_result_frame(request.clone(), &outcome)
            .context("build retained runtime command frame result")
        {
            Ok(result) => result,
            Err(error) => {
                self.session = previous_session;
                return Err(error);
            }
        };
        if let Err(error) = self.require_valid_script_modal_state("finish runtime command frame") {
            self.session = previous_session;
            return Err(error);
        }
        self.runtime_command_sequence = request.sequence();
        self.runtime_commands.push(request.clone());
        self.runtime_results.push(result);
        self.record_runtime_mutation_outcome(&outcome);
        Ok(outcome)
    }

    pub fn close_active_menu(&mut self) -> Result<RuntimeMenuClose> {
        self.require_valid_script_modal_state("close active menu")?;
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CloseActiveMenu)?;
        let RuntimeMutationResult::ActiveMenuClosed(menu) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-menu-close result");
        };
        Ok(RuntimeMenuClose {
            menu,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn close_runtime_window(&mut self) -> Result<RuntimeWindowClose> {
        self.require_valid_script_modal_state("close runtime window")?;
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CloseRuntimeWindow)?;
        let RuntimeMutationResult::RuntimeWindowClosed = mutation.result else {
            anyhow::bail!("runtime mutation returned non-runtime-window-close result");
        };
        Ok(RuntimeWindowClose {
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn close_text_window(&mut self) -> Result<RuntimeTextWindowClose> {
        self.require_valid_script_modal_state("close text window")?;
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CloseTextWindow)?;
        let RuntimeMutationResult::TextWindowClosed = mutation.result else {
            anyhow::bail!("runtime mutation returned non-text-window-close result");
        };
        Ok(RuntimeTextWindowClose {
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn advance_pending_text_wait(&mut self) -> Result<RuntimeTextWaitAdvance> {
        self.require_valid_script_modal_state("advance pending text wait")?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::TakePendingScriptRequest(RuntimePendingScriptRequestCommand {
                kind: RuntimePendingScriptRequestKind::TextWait,
            }),
        )?;
        let RuntimeMutationResult::PendingScriptRequestTaken(
            RuntimePendingScriptRequest::TextWait(wait),
        ) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-text-wait result");
        };
        Ok(RuntimeTextWaitAdvance {
            wait,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn resolve_pending_yes_no(&mut self, accepted: bool) -> Result<RuntimeYesNoResolution> {
        self.require_valid_script_modal_state("resolve pending yes/no")?;
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ResolvePendingYesNo(
                RuntimePendingYesNoResolutionCommand { accepted },
            ))?;
        let RuntimeMutationResult::PendingYesNoResolved(resolution) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-yes-no-resolution result");
        };
        Ok(RuntimeYesNoResolution {
            prompt: resolution.prompt,
            accepted: resolution.accepted,
            script_value: resolution.script_value,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_vertical_menu(
        &mut self,
        map_name: impl Into<String>,
        menu_key: impl Into<String>,
        source_script: impl Into<String>,
        loadmenu_command_index: usize,
        verticalmenu_command_index: usize,
    ) -> Result<RuntimeVerticalMenuOpen> {
        self.require_valid_script_modal_state("open vertical menu")?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::OpenVerticalMenu(RuntimeVerticalMenuOpenCommand {
                map_name: map_name.into(),
                menu_key: menu_key.into(),
                source_script: source_script.into(),
                loadmenu_command_index,
                verticalmenu_command_index,
            }),
        )?;
        let RuntimeMutationResult::VerticalMenuOpened(opened) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-vertical-menu-open result");
        };
        Ok(RuntimeVerticalMenuOpen {
            map_name: opened.map_name,
            menu_key: opened.menu_key,
            menu_id: opened.menu_id,
            source_script: opened.source_script,
            loadmenu_command_index: opened.loadmenu_command_index,
            verticalmenu_command_index: opened.verticalmenu_command_index,
            options: opened.options,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn select_vertical_menu_option(
        &mut self,
        menu_id: impl Into<String>,
        source_script: impl Into<String>,
        verticalmenu_command_index: usize,
        option_index: usize,
        option: impl Into<String>,
    ) -> Result<RuntimeVerticalMenuOptionSelection> {
        self.require_valid_script_modal_state("select vertical menu option")?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SelectVerticalMenuOption(RuntimeVerticalMenuSelectionCommand {
                menu_id: menu_id.into(),
                source_script: source_script.into(),
                verticalmenu_command_index,
                option_index,
                option: option.into(),
            }),
        )?;
        let RuntimeMutationResult::VerticalMenuOptionSelected(selection) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-vertical-menu-selection result");
        };
        Ok(RuntimeVerticalMenuOptionSelection {
            menu_id: selection.menu_id,
            source_script: selection.source_script,
            verticalmenu_command_index: selection.verticalmenu_command_index,
            option_index: selection.option_index,
            option: selection.option,
            script_value: selection.script_value,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn select_linked_vertical_menu_option(
        &mut self,
        descriptor: &RuntimeLinkSessionDescriptor,
        menu_id: impl Into<String>,
        source_script: impl Into<String>,
        verticalmenu_command_index: usize,
        option_index: usize,
        option: impl Into<String>,
    ) -> Result<RuntimeLinkedMenuChoice> {
        let menu_id = menu_id.into();
        let frame = self.linked_menu_choice_frame(
            descriptor,
            menu_id.clone(),
            verticalmenu_command_index,
            option_index,
        )?;
        let selection = self.select_vertical_menu_option(
            menu_id,
            source_script,
            verticalmenu_command_index,
            option_index,
            option,
        )?;
        Ok(RuntimeLinkedMenuChoice { frame, selection })
    }

    pub fn linked_menu_choice_frame(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        menu_id: impl Into<String>,
        verticalmenu_command_index: usize,
        option_index: usize,
    ) -> Result<MenuChoiceFrame> {
        MenuChoiceFrame::new(
            descriptor.local_player.id(),
            crystal_core::timing::Frame(self.session.state().frame_counter),
            menu_id.into(),
            option_index,
            verticalmenu_command_index,
        )
        .context("build runtime linked menu choice frame")
    }

    pub fn send_linked_vertical_menu_option<T: crystal_net::LinkTransport>(
        &mut self,
        endpoint: &mut crystal_net::LinkEndpoint<T>,
        descriptor: &RuntimeLinkSessionDescriptor,
        menu_id: impl Into<String>,
        source_script: impl Into<String>,
        verticalmenu_command_index: usize,
        option_index: usize,
        option: impl Into<String>,
    ) -> Result<RuntimeLinkedMenuChoice> {
        let menu_id = menu_id.into();
        let frame = self.linked_menu_choice_frame(
            descriptor,
            menu_id.clone(),
            verticalmenu_command_index,
            option_index,
        )?;
        endpoint
            .send(LinkMessage::MenuChoice(frame.clone()))
            .context("send runtime linked menu choice")?;
        let selection = self.select_vertical_menu_option(
            menu_id,
            source_script,
            verticalmenu_command_index,
            option_index,
            option,
        )?;
        Ok(RuntimeLinkedMenuChoice { frame, selection })
    }

    pub fn linked_menu_choice_result_frame(
        &self,
        descriptor: &RuntimeLinkSessionDescriptor,
        choice: &RuntimeLinkedMenuChoice,
    ) -> Result<MenuChoiceResultFrame> {
        MenuChoiceResultFrame::new(
            choice.frame.clone(),
            StateChecksumFrame::new(
                descriptor.local_player.id(),
                crystal_core::timing::Frame(choice.selection.state_checksum.frame()),
                choice.selection.state_checksum.hash(),
            ),
            choice.selection.script_value.clone(),
        )
        .context("build runtime linked menu choice result frame")
    }

    pub fn record_linked_menu_choice_result(
        &mut self,
        descriptor: &RuntimeLinkSessionDescriptor,
        choice: &RuntimeLinkedMenuChoice,
    ) -> Result<MenuChoiceResultFrame> {
        let result = self.linked_menu_choice_result_frame(descriptor, choice)?;
        self.linked_menu_results.push(result.clone());
        Ok(result)
    }

    pub fn send_linked_menu_choice_result<T: crystal_net::LinkTransport>(
        &mut self,
        endpoint: &mut crystal_net::LinkEndpoint<T>,
        descriptor: &RuntimeLinkSessionDescriptor,
        choice: &RuntimeLinkedMenuChoice,
    ) -> Result<MenuChoiceResultFrame> {
        let result = self.linked_menu_choice_result_frame(descriptor, choice)?;
        endpoint
            .send(LinkMessage::MenuChoiceResult(result.clone()))
            .context("send runtime linked menu choice result")?;
        self.linked_menu_results.push(result.clone());
        Ok(result)
    }

    pub fn select_elevator_floor(
        &mut self,
        map_name: impl Into<String>,
        data_label: impl Into<String>,
        source_script: impl Into<String>,
        elevator_command_index: usize,
        floor_index: usize,
        floor: impl Into<String>,
        warp: u16,
        target_map: impl Into<String>,
    ) -> Result<RuntimeElevatorFloorSelection> {
        self.require_valid_script_modal_state("select elevator floor")?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SelectElevatorFloor(RuntimeElevatorFloorSelectionCommand {
                map_name: map_name.into(),
                data_label: data_label.into(),
                source_script: source_script.into(),
                elevator_command_index,
                floor_index,
                floor: floor.into(),
                warp,
                target_map: target_map.into(),
            }),
        )?;
        let RuntimeMutationResult::ElevatorFloorSelected(selection) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-elevator-floor-selection result");
        };
        Ok(RuntimeElevatorFloorSelection {
            map_name: selection.map_name,
            data_label: selection.data_label,
            source_script: selection.source_script,
            elevator_command_index: selection.elevator_command_index,
            floor_index: selection.floor_index,
            floor: selection.floor,
            warp: selection.warp,
            target_map: selection.target_map,
            destination_tile: selection.destination_tile,
            script_value: selection.script_value,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn clear_menu_coords(&mut self) -> Result<RuntimeMenuCoordsClear> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ClearMenuCoords)?;
        let RuntimeMutationResult::MenuCoordsCleared(coords) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-menu-coords-clear result");
        };
        Ok(RuntimeMenuCoordsClear {
            coords,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn close_active_pokemon_picture(&mut self) -> Result<RuntimePokemonPictureClose> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CloseActivePokemonPicture)?;
        let RuntimeMutationResult::ActivePokemonPictureClosed(species_id) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-pokemon-picture-close result");
        };
        Ok(RuntimePokemonPictureClose {
            species_id,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn close_script_shop(&mut self) -> Result<RuntimeShopClose> {
        self.require_valid_script_modal_state("close script shop")?;
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CloseScriptShop)?;
        let RuntimeMutationResult::ScriptShopClosed(shop) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-shop-close result");
        };
        Ok(RuntimeShopClose {
            shop,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn drain_script_event_queue(
        &mut self,
        queue: RuntimeScriptEventQueue,
    ) -> Result<RuntimeScriptEventDrainResult> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::DrainScriptEventQueue(RuntimeScriptEventDrainCommand { queue }),
        )?;
        let RuntimeMutationResult::ScriptEventQueueDrained(drained) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-event-drain result");
        };
        Ok(drained)
    }

    pub fn drain_audio_events(&mut self) -> Result<RuntimeAudioEventDrain> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::DrainScriptEventQueue(RuntimeScriptEventDrainCommand {
                queue: RuntimeScriptEventQueue::Audio,
            }),
        )?;
        let RuntimeMutationResult::ScriptEventQueueDrained(RuntimeScriptEventDrainResult::Audio(
            events,
        )) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-audio-event-drain result");
        };
        Ok(RuntimeAudioEventDrain {
            events,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn drain_resolved_audio_events(&mut self) -> Result<RuntimeResolvedAudioEventDrain> {
        let drain = self.drain_audio_events()?;
        let events = self
            .runtime
            .audio()
            .resolve_audio_events(drain.events)
            .context("resolve drained runtime audio events")?;
        Ok(RuntimeResolvedAudioEventDrain {
            events,
            state_checksum: drain.state_checksum,
        })
    }

    pub fn drain_script_runtime_queue(
        &mut self,
        queue: RuntimeScriptRuntimeQueue,
    ) -> Result<RuntimeScriptRuntimeQueueDrainResult> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::DrainScriptRuntimeQueue(
                RuntimeScriptRuntimeQueueDrainCommand { queue },
            ))?;
        let RuntimeMutationResult::ScriptRuntimeQueueDrained(drained) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-runtime-queue-drain result");
        };
        Ok(drained)
    }

    pub fn drain_script_runtime_record_queue(
        &mut self,
        queue: RuntimeScriptRuntimeRecordQueue,
    ) -> Result<RuntimeScriptRuntimeRecordQueueDrainResult> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::DrainScriptRuntimeRecordQueue(
                RuntimeScriptRuntimeRecordQueueDrainCommand { queue },
            ),
        )?;
        let RuntimeMutationResult::ScriptRuntimeRecordQueueDrained(drained) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-script-runtime-record-queue-drain result");
        };
        Ok(drained)
    }

    pub fn take_pending_script_request(
        &mut self,
        kind: RuntimePendingScriptRequestKind,
    ) -> Result<RuntimePendingScriptRequest> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::TakePendingScriptRequest(
                RuntimePendingScriptRequestCommand { kind },
            ))?;
        let RuntimeMutationResult::PendingScriptRequestTaken(request) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-pending-script-request result");
        };
        Ok(request)
    }

    pub fn consume_script_runtime_flag(
        &mut self,
        flag: RuntimeScriptRuntimeFlag,
    ) -> Result<RuntimeScriptRuntimeFlagValue> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ConsumeScriptRuntimeFlag(
                RuntimeScriptRuntimeFlagCommand { flag },
            ))?;
        let RuntimeMutationResult::ScriptRuntimeFlagConsumed(value) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-runtime-flag result");
        };
        Ok(value)
    }

    pub fn take_script_runtime_memory_value(
        &mut self,
        value: RuntimeScriptRuntimeMemoryValue,
    ) -> Result<RuntimeScriptRuntimeMemoryValueTaken> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::TakeScriptRuntimeMemoryValue(
                RuntimeScriptRuntimeMemoryValueCommand { value },
            ),
        )?;
        let RuntimeMutationResult::ScriptRuntimeMemoryValueTaken(value) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-runtime-memory-value result");
        };
        Ok(value)
    }

    pub fn remove_script_runtime_memory_entry(
        &mut self,
        entry: RuntimeScriptRuntimeMemoryEntry,
        key: impl Into<String>,
    ) -> Result<RuntimeScriptRuntimeMemoryEntryRemoved> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::RemoveScriptRuntimeMemoryEntry(
                RuntimeScriptRuntimeMemoryEntryCommand {
                    entry,
                    key: key.into(),
                },
            ),
        )?;
        let RuntimeMutationResult::ScriptRuntimeMemoryEntryRemoved(removed) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-script-runtime-memory-entry result");
        };
        Ok(removed)
    }

    pub fn special_routine_ids(&self) -> BTreeSet<String> {
        self.runtime.special_routine_ids()
    }

    pub fn item_ids(&self) -> BTreeSet<String> {
        self.runtime.item_ids()
    }

    pub fn move_ids(&self) -> BTreeSet<String> {
        self.runtime.move_ids()
    }

    pub fn species_ids(&self) -> BTreeSet<String> {
        self.runtime.species_ids()
    }

    pub fn map_ids(&self) -> BTreeSet<String> {
        self.runtime.map_ids()
    }

    pub fn trainer_ids(&self) -> BTreeSet<String> {
        self.runtime.trainer_ids()
    }

    pub fn text_ids(&self) -> BTreeSet<String> {
        self.runtime.text_ids()
    }

    pub fn menu_ids(&self) -> BTreeSet<String> {
        self.runtime.menu_ids()
    }

    pub fn phone_contact_ids(&self) -> BTreeSet<String> {
        self.runtime.phone_contact_ids()
    }

    pub fn special_phone_call_ids(&self) -> BTreeSet<String> {
        self.runtime.special_phone_call_ids()
    }

    pub fn npc_trade_ids(&self) -> BTreeSet<String> {
        self.runtime.npc_trade_ids()
    }

    pub fn sprite_ids(&self) -> BTreeSet<String> {
        self.runtime.sprite_ids()
    }

    pub fn map_constants(&self) -> BTreeSet<String> {
        self.runtime.map_constants()
    }

    pub fn event_flag_ids(&self) -> BTreeSet<String> {
        self.runtime.event_flag_ids()
    }

    pub fn engine_flag_ids(&self) -> BTreeSet<String> {
        self.runtime.engine_flag_ids()
    }

    pub fn spawn_identifiers(&self) -> BTreeSet<u16> {
        self.runtime.spawn_identifiers()
    }

    pub fn tileset_ids(&self) -> BTreeSet<String> {
        self.runtime.tileset_ids()
    }

    pub fn tileset_keys(&self) -> BTreeSet<RuntimeTilesetKey> {
        self.runtime.tileset_keys()
    }

    pub fn pc_string_keys(&self) -> BTreeSet<RuntimePcStringKey> {
        self.runtime.pc_string_keys()
    }

    pub fn menu_icon_keys(&self) -> BTreeSet<RuntimeMenuIconKey> {
        self.runtime.menu_icon_keys()
    }

    pub fn pokedex_entry_keys(&self) -> BTreeSet<RuntimePokedexEntryKey> {
        self.runtime.pokedex_entry_keys()
    }

    pub fn landmark_ids(&self) -> BTreeSet<String> {
        self.runtime.landmark_ids()
    }

    pub fn pokegear_landmark_keys(&self) -> BTreeSet<RuntimePokegearLandmarkKey> {
        self.runtime.pokegear_landmark_keys()
    }

    pub fn pokegear_map_landmark_keys(&self) -> BTreeSet<RuntimePokegearMapLandmarkKey> {
        self.runtime.pokegear_map_landmark_keys()
    }

    pub fn fishing_rod_ids(&self) -> BTreeSet<String> {
        self.runtime.fishing_rod_ids()
    }

    pub fn map_group_ids(&self) -> BTreeSet<String> {
        self.runtime.map_group_ids()
    }

    pub fn encounter_group_ids(&self) -> BTreeSet<String> {
        self.runtime.encounter_group_ids()
    }

    pub fn mart_ids(&self) -> BTreeSet<String> {
        self.runtime.mart_ids()
    }

    pub fn mart_keys(&self) -> BTreeSet<RuntimeMartKey> {
        self.runtime.mart_keys()
    }

    pub fn fruit_tree_ids(&self) -> BTreeSet<String> {
        self.runtime.fruit_tree_ids()
    }

    pub fn fruit_tree_keys(&self) -> BTreeSet<RuntimeFruitTreeKey> {
        self.runtime.fruit_tree_keys()
    }

    pub fn field_move_rule_ids(&self) -> BTreeSet<String> {
        self.runtime.field_move_rule_ids()
    }

    pub fn field_move_rule_keys(&self) -> BTreeSet<RuntimeFieldMoveRuleKey> {
        self.runtime.field_move_rule_keys()
    }

    pub fn fly_destination_ids(&self) -> BTreeSet<String> {
        self.runtime.fly_destination_ids()
    }

    pub fn fly_destination_keys(&self) -> BTreeSet<RuntimeFlyDestinationKey> {
        self.runtime.fly_destination_keys()
    }

    pub fn field_move_move_ids(&self) -> BTreeSet<String> {
        self.runtime.field_move_move_ids()
    }

    pub fn field_move_item_ids(&self) -> BTreeSet<String> {
        self.runtime.field_move_item_ids()
    }

    pub fn flee_mon_bucket_ids(&self) -> BTreeSet<String> {
        self.runtime.flee_mon_bucket_ids()
    }

    pub fn buena_password_category_ids(&self) -> BTreeSet<String> {
        self.runtime.buena_password_category_ids()
    }

    pub fn roaming_species_ids(&self) -> BTreeSet<String> {
        self.runtime.roaming_species_ids()
    }

    pub fn buena_prize_item_ids(&self) -> BTreeSet<String> {
        self.runtime.buena_prize_item_ids()
    }

    pub fn kurt_apricorn_item_ids(&self) -> BTreeSet<String> {
        self.runtime.kurt_apricorn_item_ids()
    }

    pub fn dratini_move_set_ids(&self) -> BTreeSet<u8> {
        self.runtime.dratini_move_set_ids()
    }

    pub fn special_feature_ids(&self) -> BTreeSet<String> {
        self.runtime.special_feature_ids()
    }

    pub fn oak_rating_text_ids(&self) -> BTreeSet<String> {
        self.runtime.oak_rating_text_ids()
    }

    pub fn odd_egg_species_ids(&self) -> BTreeSet<String> {
        self.runtime.odd_egg_species_ids()
    }

    pub fn magikarp_length_thresholds(&self) -> BTreeSet<u16> {
        self.runtime.magikarp_length_thresholds()
    }

    pub fn happiness_change_ids(&self) -> BTreeSet<u8> {
        self.runtime.happiness_change_ids()
    }

    pub fn happiness_service_ids(&self) -> BTreeSet<String> {
        self.runtime.happiness_service_ids()
    }

    pub fn pokemon_status_ids(&self) -> BTreeSet<String> {
        self.runtime.pokemon_status_ids()
    }

    pub fn fishing_daily_flag_bits(&self) -> BTreeSet<u32> {
        self.runtime.fishing_daily_flag_bits()
    }

    pub fn fishing_swarm_flags(&self) -> BTreeSet<u8> {
        self.runtime.fishing_swarm_flags()
    }

    pub fn pending_special_battle_type_ids(&self) -> BTreeSet<String> {
        self.runtime.pending_special_battle_type_ids()
    }

    pub fn scripted_trainer_battle_keys(&self) -> BTreeSet<RuntimeScriptedTrainerBattleKey> {
        self.runtime.scripted_trainer_battle_keys()
    }

    pub fn wild_encounter_origin_keys(&self) -> BTreeSet<RuntimeWildEncounterOriginKey> {
        self.runtime.wild_encounter_origin_keys()
    }

    pub fn script_label_ids(&self) -> BTreeSet<String> {
        self.runtime.script_label_ids()
    }

    pub fn script_command_keys(&self) -> BTreeSet<RuntimeScriptCommandKey> {
        self.runtime.script_command_keys()
    }

    pub fn script_command_payload_keys(&self) -> BTreeSet<RuntimeScriptCommandPayloadKey> {
        self.runtime.script_command_payload_keys()
    }

    pub fn script_return_keys(&self) -> BTreeSet<RuntimeScriptReturnKey> {
        self.runtime.script_return_keys()
    }

    pub fn script_vertical_menu_keys(&self) -> BTreeSet<RuntimeScriptVerticalMenuKey> {
        self.runtime.script_vertical_menu_keys()
    }

    pub fn script_text_body_keys(&self) -> BTreeSet<RuntimeScriptTextBodyKey> {
        self.runtime.script_text_body_keys()
    }

    pub fn script_menu_definition_keys(&self) -> BTreeSet<RuntimeScriptMenuDefinitionKey> {
        self.runtime.script_menu_definition_keys()
    }

    pub fn script_elevator_keys(&self) -> BTreeSet<RuntimeScriptElevatorKey> {
        self.runtime.script_elevator_keys()
    }

    pub fn gift_pokemon_keys(&self) -> BTreeSet<RuntimeGiftPokemonKey> {
        self.runtime.gift_pokemon_keys()
    }

    pub fn script_object_command_keys(&self) -> BTreeSet<RuntimeScriptObjectCommandKey> {
        self.runtime.script_object_command_keys()
    }

    pub fn script_movement_keys(&self) -> BTreeSet<RuntimeScriptMovementKey> {
        self.runtime.script_movement_keys()
    }

    pub fn map_script_section_command_keys(&self) -> BTreeSet<RuntimeMapScriptSectionCommandKey> {
        self.runtime.map_script_section_command_keys()
    }

    pub fn map_event_section_command_keys(&self) -> BTreeSet<RuntimeMapEventSectionCommandKey> {
        self.runtime.map_event_section_command_keys()
    }

    pub fn script_map_command_keys(&self) -> BTreeSet<RuntimeScriptMapCommandKey> {
        self.runtime.script_map_command_keys()
    }

    pub fn script_variable_command_keys(&self) -> BTreeSet<RuntimeScriptVariableCommandKey> {
        self.runtime.script_variable_command_keys()
    }

    pub fn script_control_command_keys(&self) -> BTreeSet<RuntimeScriptControlCommandKey> {
        self.runtime.script_control_command_keys()
    }

    pub fn script_swarm_command_keys(&self) -> BTreeSet<RuntimeScriptSwarmCommandKey> {
        self.runtime.script_swarm_command_keys()
    }

    pub fn script_field_pickup_keys(&self) -> BTreeSet<RuntimeScriptFieldPickupKey> {
        self.runtime.script_field_pickup_keys()
    }

    pub fn script_shop_command_keys(&self) -> BTreeSet<RuntimeScriptShopCommandKey> {
        self.runtime.script_shop_command_keys()
    }

    pub fn script_phone_command_keys(&self) -> BTreeSet<RuntimeScriptPhoneCommandKey> {
        self.runtime.script_phone_command_keys()
    }

    pub fn script_runtime_command_keys(&self) -> BTreeSet<RuntimeScriptRuntimeCommandKey> {
        self.runtime.script_runtime_command_keys()
    }

    pub fn script_runtime_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Option<RuntimeScriptRuntimeCommandKey> {
        self.runtime
            .script_runtime_command_at(map_name, source_script, command_index)
    }

    pub fn script_item_grant_keys(&self) -> BTreeSet<RuntimeScriptItemGrantKey> {
        self.runtime.script_item_grant_keys()
    }

    pub fn script_item_access_keys(&self) -> BTreeSet<RuntimeScriptItemAccessKey> {
        self.runtime.script_item_access_keys()
    }

    pub fn script_economy_command_keys(&self) -> BTreeSet<RuntimeScriptEconomyCommandKey> {
        self.runtime.script_economy_command_keys()
    }

    pub fn script_flag_command_keys(&self) -> BTreeSet<RuntimeScriptFlagCommandKey> {
        self.runtime.script_flag_command_keys()
    }

    pub fn script_scene_command_keys(&self) -> BTreeSet<RuntimeScriptSceneCommandKey> {
        self.runtime.script_scene_command_keys()
    }

    pub fn script_block_change_keys(&self) -> BTreeSet<RuntimeScriptBlockChangeKey> {
        self.runtime.script_block_change_keys()
    }

    pub fn script_audio_command_keys(&self) -> BTreeSet<RuntimeScriptAudioCommandKey> {
        self.runtime.script_audio_command_keys()
    }

    pub fn script_text_command_keys(&self) -> BTreeSet<RuntimeScriptTextCommandKey> {
        self.runtime.script_text_command_keys()
    }

    pub fn warp_keys(&self) -> BTreeSet<RuntimeWarpKey> {
        self.runtime.warp_keys()
    }

    pub fn map_object_keys(&self) -> BTreeSet<RuntimeMapObjectKey> {
        self.runtime.map_object_keys()
    }

    pub fn map_scene_keys(&self) -> BTreeSet<RuntimeMapSceneKey> {
        self.runtime.map_scene_keys()
    }

    pub fn map_metadata_keys(&self) -> BTreeSet<RuntimeMapMetadataKey> {
        self.runtime.map_metadata_keys()
    }

    pub fn currency_constant_ids(&self) -> BTreeSet<String> {
        self.runtime.currency_constant_ids()
    }

    pub fn capture_ball_rule_ids(&self) -> BTreeSet<String> {
        self.runtime.capture_ball_rule_ids()
    }

    pub fn guaranteed_capture_ball_ids(&self) -> BTreeSet<String> {
        self.runtime.guaranteed_capture_ball_ids()
    }

    pub fn capture_status_bonus_ids(&self) -> BTreeSet<String> {
        self.runtime.capture_status_bonus_ids()
    }

    pub fn fast_ball_species_ids(&self) -> BTreeSet<String> {
        self.runtime.fast_ball_species_ids()
    }

    pub fn heavy_ball_species_ids(&self) -> BTreeSet<String> {
        self.runtime.heavy_ball_species_ids()
    }

    pub fn move_priority_effect_ids(&self) -> BTreeSet<String> {
        self.runtime.move_priority_effect_ids()
    }

    pub fn move_priority_move_ids(&self) -> BTreeSet<String> {
        self.runtime.move_priority_move_ids()
    }

    pub fn capture_ball_rule_keys(&self) -> BTreeSet<RuntimeCaptureBallRuleKey> {
        self.runtime.capture_ball_rule_keys()
    }

    pub fn heavy_ball_modifier_keys(&self) -> BTreeSet<RuntimeHeavyBallModifierKey> {
        self.runtime.heavy_ball_modifier_keys()
    }

    pub fn capture_status_bonus_keys(&self) -> BTreeSet<RuntimeCaptureStatusBonusKey> {
        self.runtime.capture_status_bonus_keys()
    }

    pub fn capture_wobble_probability_keys(&self) -> BTreeSet<RuntimeCaptureWobbleProbabilityKey> {
        self.runtime.capture_wobble_probability_keys()
    }

    pub fn item_battle_use_keys(&self) -> BTreeSet<RuntimeItemBattleUseKey> {
        self.runtime.item_battle_use_keys()
    }

    pub fn item_effect_plan_keys(&self) -> BTreeSet<RuntimeItemEffectPlanKey> {
        self.runtime.item_effect_plan_keys()
    }

    pub fn item_field_use_keys(&self) -> BTreeSet<RuntimeItemFieldUseKey> {
        self.runtime.item_field_use_keys()
    }

    pub fn move_battle_data_keys(&self) -> BTreeSet<RuntimeMoveBattleDataKey> {
        self.runtime.move_battle_data_keys()
    }

    pub fn species_battle_data_keys(&self) -> BTreeSet<RuntimeSpeciesBattleDataKey> {
        self.runtime.species_battle_data_keys()
    }

    pub fn species_learnset_keys(&self) -> BTreeSet<RuntimeSpeciesLearnsetKey> {
        self.runtime.species_learnset_keys()
    }

    pub fn species_evolution_keys(&self) -> BTreeSet<RuntimeSpeciesEvolutionKey> {
        self.runtime.species_evolution_keys()
    }

    pub fn trainer_battle_data_keys(&self) -> BTreeSet<RuntimeTrainerBattleDataKey> {
        self.runtime.trainer_battle_data_keys()
    }

    pub fn trainer_party_pokemon_keys(&self) -> BTreeSet<RuntimeTrainerPartyPokemonKey> {
        self.runtime.trainer_party_pokemon_keys()
    }

    pub fn move_priority_effect_keys(&self) -> BTreeSet<RuntimeMovePriorityEffectKey> {
        self.runtime.move_priority_effect_keys()
    }

    pub fn move_priority_move_keys(&self) -> BTreeSet<RuntimeMovePriorityMoveKey> {
        self.runtime.move_priority_move_keys()
    }

    pub fn battle_stat_multiplier_keys(&self) -> BTreeSet<RuntimeBattleStatMultiplierKey> {
        self.runtime.battle_stat_multiplier_keys()
    }

    pub fn battle_reward_rule_keys(&self) -> BTreeSet<RuntimeBattleRewardRuleKey> {
        self.runtime.battle_reward_rule_keys()
    }

    pub fn battle_escape_rule_keys(&self) -> BTreeSet<RuntimeBattleEscapeRuleKey> {
        self.runtime.battle_escape_rule_keys()
    }

    pub fn physical_type_ids(&self) -> BTreeSet<String> {
        self.runtime.physical_type_ids()
    }

    pub fn special_type_ids(&self) -> BTreeSet<String> {
        self.runtime.special_type_ids()
    }

    pub fn weather_ids(&self) -> BTreeSet<String> {
        self.runtime.weather_ids()
    }

    pub fn type_effectiveness_keys(&self) -> BTreeSet<RuntimeTypeEffectivenessKey> {
        self.runtime.type_effectiveness_keys()
    }

    pub fn foresight_type_effectiveness_keys(&self) -> BTreeSet<RuntimeTypeEffectivenessKey> {
        self.runtime.foresight_type_effectiveness_keys()
    }

    pub fn weather_type_modifier_keys(&self) -> BTreeSet<RuntimeWeatherTypeModifierKey> {
        self.runtime.weather_type_modifier_keys()
    }

    pub fn weather_move_effect_modifier_keys(
        &self,
    ) -> BTreeSet<RuntimeWeatherMoveEffectModifierKey> {
        self.runtime.weather_move_effect_modifier_keys()
    }

    pub fn music_ids(&self) -> BTreeSet<String> {
        self.runtime.music_ids()
    }

    pub fn sound_effect_ids(&self) -> BTreeSet<String> {
        self.runtime.sound_effect_ids()
    }

    pub fn cry_ids(&self) -> BTreeSet<String> {
        self.runtime.cry_ids()
    }

    pub fn pokemon_cry_keys(&self) -> BTreeSet<RuntimePokemonCryKey> {
        self.runtime.pokemon_cry_keys()
    }

    pub fn audio_asset_keys(&self) -> BTreeSet<RuntimeAudioAssetKey> {
        self.runtime.audio_asset_keys()
    }

    pub fn has_special_routine(&self, routine: &str) -> bool {
        self.runtime.has_special_routine(routine)
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.runtime.has_item(item_id)
    }

    pub fn has_move(&self, move_id: &str) -> bool {
        self.runtime.has_move(move_id)
    }

    pub fn has_species(&self, species_id: &str) -> bool {
        self.runtime.has_species(species_id)
    }

    pub fn has_map(&self, map_name: &str) -> bool {
        self.runtime.has_map(map_name)
    }

    pub fn has_trainer(&self, trainer_id: &str) -> bool {
        self.runtime.has_trainer(trainer_id)
    }

    pub fn has_text(&self, text_label: &str) -> bool {
        self.runtime.has_text(text_label)
    }

    pub fn has_menu(&self, menu: &str) -> bool {
        self.runtime.has_menu(menu)
    }

    pub fn has_phone_contact(&self, contact_id: &str) -> bool {
        self.runtime.has_phone_contact(contact_id)
    }

    pub fn has_special_phone_call(&self, call_id: &str) -> bool {
        self.runtime.has_special_phone_call(call_id)
    }

    pub fn has_npc_trade(&self, trade_id: &str) -> bool {
        self.runtime.has_npc_trade(trade_id)
    }

    pub fn has_sprite(&self, sprite_id: &str) -> bool {
        self.runtime.has_sprite(sprite_id)
    }

    pub fn has_map_constant(&self, map_constant: &str) -> bool {
        self.runtime.has_map_constant(map_constant)
    }

    pub fn has_event_flag(&self, flag: &str) -> bool {
        self.runtime.has_event_flag(flag)
    }

    pub fn has_engine_flag(&self, flag: &str) -> bool {
        self.runtime.has_engine_flag(flag)
    }

    pub fn has_spawn_identifier(&self, spawn_identifier: u16) -> bool {
        self.runtime.has_spawn_identifier(spawn_identifier)
    }

    pub fn has_tileset(&self, tileset_id: &str) -> bool {
        self.runtime.has_tileset(tileset_id)
    }

    pub fn has_tileset_row(&self, key: &RuntimeTilesetKey) -> bool {
        self.runtime.has_tileset_row(key)
    }

    pub fn has_pc_string(&self, key: &RuntimePcStringKey) -> bool {
        self.runtime.has_pc_string(key)
    }

    pub fn has_menu_icon(&self, key: &RuntimeMenuIconKey) -> bool {
        self.runtime.has_menu_icon(key)
    }

    pub fn has_pokedex_entry(&self, key: &RuntimePokedexEntryKey) -> bool {
        self.runtime.has_pokedex_entry(key)
    }

    pub fn has_landmark(&self, landmark_id: &str) -> bool {
        self.runtime.has_landmark(landmark_id)
    }

    pub fn has_pokegear_landmark(&self, key: &RuntimePokegearLandmarkKey) -> bool {
        self.runtime.has_pokegear_landmark(key)
    }

    pub fn has_pokegear_map_landmark(&self, key: &RuntimePokegearMapLandmarkKey) -> bool {
        self.runtime.has_pokegear_map_landmark(key)
    }

    pub fn has_fishing_rod(&self, rod: &str) -> bool {
        self.runtime.has_fishing_rod(rod)
    }

    pub fn has_map_group(&self, group_id: &str) -> bool {
        self.runtime.has_map_group(group_id)
    }

    pub fn has_encounter_group(&self, group_id: &str) -> bool {
        self.runtime.has_encounter_group(group_id)
    }

    pub fn has_mart(&self, mart_id: &str) -> bool {
        self.runtime.has_mart(mart_id)
    }

    pub fn has_mart_row(&self, key: &RuntimeMartKey) -> bool {
        self.runtime.has_mart_row(key)
    }

    pub fn has_fruit_tree(&self, fruit_tree_id: &str) -> bool {
        self.runtime.has_fruit_tree(fruit_tree_id)
    }

    pub fn has_fruit_tree_row(&self, key: &RuntimeFruitTreeKey) -> bool {
        self.runtime.has_fruit_tree_row(key)
    }

    pub fn has_field_move_rule(&self, rule_id: &str) -> bool {
        self.runtime.has_field_move_rule(rule_id)
    }

    pub fn has_field_move_rule_row(&self, key: &RuntimeFieldMoveRuleKey) -> bool {
        self.runtime.has_field_move_rule_row(key)
    }

    pub fn has_fly_destination(&self, flypoint_flag: &str) -> bool {
        self.runtime.has_fly_destination(flypoint_flag)
    }

    pub fn has_fly_destination_row(&self, key: &RuntimeFlyDestinationKey) -> bool {
        self.runtime.has_fly_destination_row(key)
    }

    pub fn has_field_move_move(&self, move_id: &str) -> bool {
        self.runtime.has_field_move_move(move_id)
    }

    pub fn has_field_move_item(&self, item_id: &str) -> bool {
        self.runtime.has_field_move_item(item_id)
    }

    pub fn has_flee_mon_bucket(&self, bucket_id: &str) -> bool {
        self.runtime.has_flee_mon_bucket(bucket_id)
    }

    pub fn has_buena_password_category(&self, category_id: &str) -> bool {
        self.runtime.has_buena_password_category(category_id)
    }

    pub fn has_roaming_species(&self, species_id: &str) -> bool {
        self.runtime.has_roaming_species(species_id)
    }

    pub fn has_buena_prize_item(&self, item_id: &str) -> bool {
        self.runtime.has_buena_prize_item(item_id)
    }

    pub fn has_kurt_apricorn_item(&self, item_id: &str) -> bool {
        self.runtime.has_kurt_apricorn_item(item_id)
    }

    pub fn has_dratini_move_set(&self, answer: u8) -> bool {
        self.runtime.has_dratini_move_set(answer)
    }

    pub fn has_special_feature(&self, feature_id: &str) -> bool {
        self.runtime.has_special_feature(feature_id)
    }

    pub fn has_oak_rating_text(&self, text_id: &str) -> bool {
        self.runtime.has_oak_rating_text(text_id)
    }

    pub fn has_odd_egg_species(&self, species_id: &str) -> bool {
        self.runtime.has_odd_egg_species(species_id)
    }

    pub fn has_magikarp_length_threshold(&self, threshold: u16) -> bool {
        self.runtime.has_magikarp_length_threshold(threshold)
    }

    pub fn has_happiness_change(&self, change_id: u8) -> bool {
        self.runtime.has_happiness_change(change_id)
    }

    pub fn has_happiness_service(&self, service_id: &str) -> bool {
        self.runtime.has_happiness_service(service_id)
    }

    pub fn has_pokemon_status(&self, status: &str) -> bool {
        self.runtime.has_pokemon_status(status)
    }

    pub fn has_fishing_daily_flag_bit(&self, bit: u32) -> bool {
        self.runtime.has_fishing_daily_flag_bit(bit)
    }

    pub fn has_fishing_swarm_flag(&self, swarm_flag: u8) -> bool {
        self.runtime.has_fishing_swarm_flag(swarm_flag)
    }

    pub fn has_pending_special_battle_type(&self, battle_type: &str) -> bool {
        self.runtime.has_pending_special_battle_type(battle_type)
    }

    pub fn has_wild_encounter_origin(&self, key: &RuntimeWildEncounterOriginKey) -> bool {
        self.runtime.has_wild_encounter_origin(key)
    }

    pub fn has_script_label(&self, script_label: &str) -> bool {
        self.runtime.has_script_label(script_label)
    }

    pub fn has_script_command(&self, key: &RuntimeScriptCommandKey) -> bool {
        self.runtime.has_script_command(key)
    }

    pub fn has_script_command_payload(&self, key: &RuntimeScriptCommandPayloadKey) -> bool {
        self.runtime.has_script_command_payload(key)
    }

    pub fn has_script_return(&self, key: &RuntimeScriptReturnKey) -> bool {
        self.runtime.has_script_return(key)
    }

    pub fn has_script_vertical_menu(&self, key: &RuntimeScriptVerticalMenuKey) -> bool {
        self.runtime.has_script_vertical_menu(key)
    }

    pub fn has_script_text_body(&self, key: &RuntimeScriptTextBodyKey) -> bool {
        self.runtime.has_script_text_body(key)
    }

    pub fn has_script_menu_definition(&self, key: &RuntimeScriptMenuDefinitionKey) -> bool {
        self.runtime.has_script_menu_definition(key)
    }

    pub fn has_script_elevator(&self, key: &RuntimeScriptElevatorKey) -> bool {
        self.runtime.has_script_elevator(key)
    }

    pub fn has_script_elevator_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.runtime
            .has_script_elevator_command_at(map_name, source_script, command_index)
    }

    pub fn has_gift_pokemon(&self, key: &RuntimeGiftPokemonKey) -> bool {
        self.runtime.has_gift_pokemon(key)
    }

    pub fn has_gift_pokemon_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.runtime
            .has_gift_pokemon_command_at(map_name, source_script, command_index)
    }

    pub fn has_script_phone_prompt_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.runtime
            .has_script_phone_prompt_command_at(map_name, source_script, command_index)
    }

    pub fn has_script_object_command(&self, key: &RuntimeScriptObjectCommandKey) -> bool {
        self.runtime.has_script_object_command(key)
    }

    pub fn has_script_movement(&self, key: &RuntimeScriptMovementKey) -> bool {
        self.runtime.has_script_movement(key)
    }

    pub fn has_map_script_section_command(&self, key: &RuntimeMapScriptSectionCommandKey) -> bool {
        self.runtime.has_map_script_section_command(key)
    }

    pub fn has_map_event_section_command(&self, key: &RuntimeMapEventSectionCommandKey) -> bool {
        self.runtime.has_map_event_section_command(key)
    }

    pub fn has_script_map_command(&self, key: &RuntimeScriptMapCommandKey) -> bool {
        self.runtime.has_script_map_command(key)
    }

    pub fn has_script_variable_command(&self, key: &RuntimeScriptVariableCommandKey) -> bool {
        self.runtime.has_script_variable_command(key)
    }

    pub fn has_script_control_command(&self, key: &RuntimeScriptControlCommandKey) -> bool {
        self.runtime.has_script_control_command(key)
    }

    pub fn has_script_swarm_command(&self, key: &RuntimeScriptSwarmCommandKey) -> bool {
        self.runtime.has_script_swarm_command(key)
    }

    pub fn has_script_field_pickup(&self, key: &RuntimeScriptFieldPickupKey) -> bool {
        self.runtime.has_script_field_pickup(key)
    }

    pub fn has_script_shop_command(&self, key: &RuntimeScriptShopCommandKey) -> bool {
        self.runtime.has_script_shop_command(key)
    }

    pub fn has_script_phone_command(&self, key: &RuntimeScriptPhoneCommandKey) -> bool {
        self.runtime.has_script_phone_command(key)
    }

    pub fn has_script_runtime_command(&self, key: &RuntimeScriptRuntimeCommandKey) -> bool {
        self.runtime.has_script_runtime_command(key)
    }

    pub fn has_script_item_grant(&self, key: &RuntimeScriptItemGrantKey) -> bool {
        self.runtime.has_script_item_grant(key)
    }

    pub fn has_script_item_access(&self, key: &RuntimeScriptItemAccessKey) -> bool {
        self.runtime.has_script_item_access(key)
    }

    pub fn has_script_economy_command(&self, key: &RuntimeScriptEconomyCommandKey) -> bool {
        self.runtime.has_script_economy_command(key)
    }

    pub fn has_script_flag_command(&self, key: &RuntimeScriptFlagCommandKey) -> bool {
        self.runtime.has_script_flag_command(key)
    }

    pub fn has_script_scene_command(&self, key: &RuntimeScriptSceneCommandKey) -> bool {
        self.runtime.has_script_scene_command(key)
    }

    pub fn has_script_block_change(&self, key: &RuntimeScriptBlockChangeKey) -> bool {
        self.runtime.has_script_block_change(key)
    }

    pub fn has_script_audio_command(&self, key: &RuntimeScriptAudioCommandKey) -> bool {
        self.runtime.has_script_audio_command(key)
    }

    pub fn has_script_text_command(&self, key: &RuntimeScriptTextCommandKey) -> bool {
        self.runtime.has_script_text_command(key)
    }

    pub fn has_warp(&self, key: &RuntimeWarpKey) -> bool {
        self.runtime.has_warp(key)
    }

    pub fn has_map_object(&self, key: &RuntimeMapObjectKey) -> bool {
        self.runtime.has_map_object(key)
    }

    pub fn has_map_scene(&self, key: &RuntimeMapSceneKey) -> bool {
        self.runtime.has_map_scene(key)
    }

    pub fn has_map_metadata(&self, key: &RuntimeMapMetadataKey) -> bool {
        self.runtime.has_map_metadata(key)
    }

    pub fn has_currency_constant(&self, id: &str) -> bool {
        self.runtime.has_currency_constant(id)
    }

    pub fn has_capture_ball_rule(&self, id: &str) -> bool {
        self.runtime.has_capture_ball_rule(id)
    }

    pub fn has_guaranteed_capture_ball(&self, id: &str) -> bool {
        self.runtime.has_guaranteed_capture_ball(id)
    }

    pub fn has_capture_status_bonus(&self, status: &str) -> bool {
        self.runtime.has_capture_status_bonus(status)
    }

    pub fn has_fast_ball_species(&self, species_id: &str) -> bool {
        self.runtime.has_fast_ball_species(species_id)
    }

    pub fn has_heavy_ball_species(&self, species_id: &str) -> bool {
        self.runtime.has_heavy_ball_species(species_id)
    }

    pub fn has_move_priority_effect(&self, effect_id: &str) -> bool {
        self.runtime.has_move_priority_effect(effect_id)
    }

    pub fn has_move_priority_move(&self, move_id: &str) -> bool {
        self.runtime.has_move_priority_move(move_id)
    }

    pub fn has_capture_ball_rule_key(&self, key: &RuntimeCaptureBallRuleKey) -> bool {
        self.runtime.has_capture_ball_rule_key(key)
    }

    pub fn has_heavy_ball_modifier(&self, key: &RuntimeHeavyBallModifierKey) -> bool {
        self.runtime.has_heavy_ball_modifier(key)
    }

    pub fn has_capture_status_bonus_key(&self, key: &RuntimeCaptureStatusBonusKey) -> bool {
        self.runtime.has_capture_status_bonus_key(key)
    }

    pub fn has_capture_wobble_probability(&self, key: &RuntimeCaptureWobbleProbabilityKey) -> bool {
        self.runtime.has_capture_wobble_probability(key)
    }

    pub fn has_item_battle_use(&self, key: &RuntimeItemBattleUseKey) -> bool {
        self.runtime.has_item_battle_use(key)
    }

    pub fn has_item_effect_plan(&self, key: &RuntimeItemEffectPlanKey) -> bool {
        self.runtime.has_item_effect_plan(key)
    }

    pub fn has_item_field_use(&self, key: &RuntimeItemFieldUseKey) -> bool {
        self.runtime.has_item_field_use(key)
    }

    pub fn has_move_battle_data(&self, key: &RuntimeMoveBattleDataKey) -> bool {
        self.runtime.has_move_battle_data(key)
    }

    pub fn has_species_battle_data(&self, key: &RuntimeSpeciesBattleDataKey) -> bool {
        self.runtime.has_species_battle_data(key)
    }

    pub fn has_trainer_battle_data(&self, key: &RuntimeTrainerBattleDataKey) -> bool {
        self.runtime.has_trainer_battle_data(key)
    }

    pub fn has_trainer_party_pokemon(&self, key: &RuntimeTrainerPartyPokemonKey) -> bool {
        self.runtime.has_trainer_party_pokemon(key)
    }

    pub fn has_move_priority_effect_key(&self, key: &RuntimeMovePriorityEffectKey) -> bool {
        self.runtime.has_move_priority_effect_key(key)
    }

    pub fn has_move_priority_move_key(&self, key: &RuntimeMovePriorityMoveKey) -> bool {
        self.runtime.has_move_priority_move_key(key)
    }

    pub fn has_battle_stat_multiplier(&self, key: &RuntimeBattleStatMultiplierKey) -> bool {
        self.runtime.has_battle_stat_multiplier(key)
    }

    pub fn has_battle_reward_rule(&self, key: &RuntimeBattleRewardRuleKey) -> bool {
        self.runtime.has_battle_reward_rule(key)
    }

    pub fn has_battle_escape_rule(&self, key: &RuntimeBattleEscapeRuleKey) -> bool {
        self.runtime.has_battle_escape_rule(key)
    }

    pub fn has_physical_type(&self, type_id: &str) -> bool {
        self.runtime.has_physical_type(type_id)
    }

    pub fn has_special_type(&self, type_id: &str) -> bool {
        self.runtime.has_special_type(type_id)
    }

    pub fn has_weather(&self, weather_id: &str) -> bool {
        self.runtime.has_weather(weather_id)
    }

    pub fn has_type_effectiveness(&self, key: &RuntimeTypeEffectivenessKey) -> bool {
        self.runtime.has_type_effectiveness(key)
    }

    pub fn has_foresight_type_effectiveness(&self, key: &RuntimeTypeEffectivenessKey) -> bool {
        self.runtime.has_foresight_type_effectiveness(key)
    }

    pub fn has_weather_type_modifier(&self, key: &RuntimeWeatherTypeModifierKey) -> bool {
        self.runtime.has_weather_type_modifier(key)
    }

    pub fn has_weather_move_effect_modifier(
        &self,
        key: &RuntimeWeatherMoveEffectModifierKey,
    ) -> bool {
        self.runtime.has_weather_move_effect_modifier(key)
    }

    pub fn has_audio_asset(&self, key: &RuntimeAudioAssetKey) -> bool {
        self.runtime.has_audio_asset(key)
    }

    pub fn has_pokemon_cry(&self, key: &RuntimePokemonCryKey) -> bool {
        self.runtime.has_pokemon_cry(key)
    }

    pub fn has_music(&self, music_id: &str) -> bool {
        self.runtime.has_music(music_id)
    }

    pub fn has_sound_effect(&self, sound_effect_id: &str) -> bool {
        self.runtime.has_sound_effect(sound_effect_id)
    }

    pub fn has_cry(&self, cry_id: &str) -> bool {
        self.runtime.has_cry(cry_id)
    }

    pub fn require_special_routine(&self, routine: &str) -> Result<()> {
        self.runtime.require_special_routine(routine)
    }

    pub fn require_item(&self, item_id: &str) -> Result<()> {
        self.runtime.require_item(item_id)
    }

    pub fn require_move(&self, move_id: &str) -> Result<()> {
        self.runtime.require_move(move_id)
    }

    pub fn require_species(&self, species_id: &str) -> Result<()> {
        self.runtime.require_species(species_id)
    }

    pub fn require_map(&self, map_name: &str) -> Result<()> {
        self.runtime.require_map(map_name)
    }

    pub fn require_trainer(&self, trainer_id: &str) -> Result<()> {
        self.runtime.require_trainer(trainer_id)
    }

    pub fn require_text(&self, text_label: &str) -> Result<()> {
        self.runtime.require_text(text_label)
    }

    pub fn require_menu(&self, menu: &str) -> Result<()> {
        self.runtime.require_menu(menu)
    }

    pub fn require_phone_contact(&self, contact_id: &str) -> Result<()> {
        self.runtime.require_phone_contact(contact_id)
    }

    pub fn require_special_phone_call(&self, call_id: &str) -> Result<()> {
        self.runtime.require_special_phone_call(call_id)
    }

    pub fn require_npc_trade(&self, trade_id: &str) -> Result<()> {
        self.runtime.require_npc_trade(trade_id)
    }

    pub fn require_sprite(&self, sprite_id: &str) -> Result<()> {
        self.runtime.require_sprite(sprite_id)
    }

    pub fn require_map_constant(&self, map_constant: &str) -> Result<()> {
        self.runtime.require_map_constant(map_constant)
    }

    pub fn require_event_flag(&self, flag: &str) -> Result<()> {
        self.runtime.require_event_flag(flag)
    }

    pub fn require_engine_flag(&self, flag: &str) -> Result<()> {
        self.runtime.require_engine_flag(flag)
    }

    pub fn require_spawn_identifier(&self, spawn_identifier: u16) -> Result<()> {
        self.runtime.require_spawn_identifier(spawn_identifier)
    }

    pub fn require_tileset(&self, tileset_id: &str) -> Result<()> {
        self.runtime.require_tileset(tileset_id)
    }

    pub fn require_tileset_row(&self, key: &RuntimeTilesetKey) -> Result<()> {
        self.runtime.require_tileset_row(key)
    }

    pub fn require_pc_string(&self, key: &RuntimePcStringKey) -> Result<()> {
        self.runtime.require_pc_string(key)
    }

    pub fn require_menu_icon(&self, key: &RuntimeMenuIconKey) -> Result<()> {
        self.runtime.require_menu_icon(key)
    }

    pub fn require_pokedex_entry(&self, key: &RuntimePokedexEntryKey) -> Result<()> {
        self.runtime.require_pokedex_entry(key)
    }

    pub fn require_landmark(&self, landmark_id: &str) -> Result<()> {
        self.runtime.require_landmark(landmark_id)
    }

    pub fn require_pokegear_landmark(&self, key: &RuntimePokegearLandmarkKey) -> Result<()> {
        self.runtime.require_pokegear_landmark(key)
    }

    pub fn require_pokegear_map_landmark(&self, key: &RuntimePokegearMapLandmarkKey) -> Result<()> {
        self.runtime.require_pokegear_map_landmark(key)
    }

    pub fn require_fishing_rod(&self, rod: &str) -> Result<()> {
        self.runtime.require_fishing_rod(rod)
    }

    pub fn require_map_group(&self, group_id: &str) -> Result<()> {
        self.runtime.require_map_group(group_id)
    }

    pub fn require_encounter_group(&self, group_id: &str) -> Result<()> {
        self.runtime.require_encounter_group(group_id)
    }

    pub fn require_mart(&self, mart_id: &str) -> Result<()> {
        self.runtime.require_mart(mart_id)
    }

    pub fn require_mart_row(&self, key: &RuntimeMartKey) -> Result<()> {
        self.runtime.require_mart_row(key)
    }

    pub fn require_fruit_tree(&self, fruit_tree_id: &str) -> Result<()> {
        self.runtime.require_fruit_tree(fruit_tree_id)
    }

    pub fn require_fruit_tree_row(&self, key: &RuntimeFruitTreeKey) -> Result<()> {
        self.runtime.require_fruit_tree_row(key)
    }

    pub fn require_field_move_rule(&self, rule_id: &str) -> Result<()> {
        self.runtime.require_field_move_rule(rule_id)
    }

    pub fn require_field_move_rule_row(&self, key: &RuntimeFieldMoveRuleKey) -> Result<()> {
        self.runtime.require_field_move_rule_row(key)
    }

    pub fn require_fly_destination(&self, flypoint_flag: &str) -> Result<()> {
        self.runtime.require_fly_destination(flypoint_flag)
    }

    pub fn require_fly_destination_row(&self, key: &RuntimeFlyDestinationKey) -> Result<()> {
        self.runtime.require_fly_destination_row(key)
    }

    pub fn require_field_move_move(&self, move_id: &str) -> Result<()> {
        self.runtime.require_field_move_move(move_id)
    }

    pub fn require_field_move_item(&self, item_id: &str) -> Result<()> {
        self.runtime.require_field_move_item(item_id)
    }

    pub fn require_flee_mon_bucket(&self, bucket_id: &str) -> Result<()> {
        self.runtime.require_flee_mon_bucket(bucket_id)
    }

    pub fn require_buena_password_category(&self, category_id: &str) -> Result<()> {
        self.runtime.require_buena_password_category(category_id)
    }

    pub fn require_roaming_species(&self, species_id: &str) -> Result<()> {
        self.runtime.require_roaming_species(species_id)
    }

    pub fn require_buena_prize_item(&self, item_id: &str) -> Result<()> {
        self.runtime.require_buena_prize_item(item_id)
    }

    pub fn require_kurt_apricorn_item(&self, item_id: &str) -> Result<()> {
        self.runtime.require_kurt_apricorn_item(item_id)
    }

    pub fn require_dratini_move_set(&self, answer: u8) -> Result<()> {
        self.runtime.require_dratini_move_set(answer)
    }

    pub fn require_special_feature(&self, feature_id: &str) -> Result<()> {
        self.runtime.require_special_feature(feature_id)
    }

    pub fn require_oak_rating_text(&self, text_id: &str) -> Result<()> {
        self.runtime.require_oak_rating_text(text_id)
    }

    pub fn require_odd_egg_species(&self, species_id: &str) -> Result<()> {
        self.runtime.require_odd_egg_species(species_id)
    }

    pub fn require_magikarp_length_threshold(&self, threshold: u16) -> Result<()> {
        self.runtime.require_magikarp_length_threshold(threshold)
    }

    pub fn require_happiness_change(&self, change_id: u8) -> Result<()> {
        self.runtime.require_happiness_change(change_id)
    }

    pub fn require_happiness_service(&self, service_id: &str) -> Result<()> {
        self.runtime.require_happiness_service(service_id)
    }

    pub fn require_pokemon_status(&self, status: &str) -> Result<()> {
        self.runtime.require_pokemon_status(status)
    }

    pub fn require_fishing_daily_flag_bit(&self, bit: u32) -> Result<()> {
        self.runtime.require_fishing_daily_flag_bit(bit)
    }

    pub fn require_fishing_swarm_flag(&self, swarm_flag: u8) -> Result<()> {
        self.runtime.require_fishing_swarm_flag(swarm_flag)
    }

    pub fn require_pending_special_battle_type(&self, battle_type: &str) -> Result<()> {
        self.runtime
            .require_pending_special_battle_type(battle_type)
    }

    pub fn require_wild_encounter_origin(&self, key: &RuntimeWildEncounterOriginKey) -> Result<()> {
        self.runtime.require_wild_encounter_origin(key)
    }

    pub fn require_script_label(&self, script_label: &str) -> Result<()> {
        self.runtime.require_script_label(script_label)
    }

    pub fn require_script_command(&self, key: &RuntimeScriptCommandKey) -> Result<()> {
        self.runtime.require_script_command(key)
    }

    pub fn require_script_command_payload(
        &self,
        key: &RuntimeScriptCommandPayloadKey,
    ) -> Result<()> {
        self.runtime.require_script_command_payload(key)
    }

    pub fn require_script_return(&self, key: &RuntimeScriptReturnKey) -> Result<()> {
        self.runtime.require_script_return(key)
    }

    pub fn require_script_vertical_menu(&self, key: &RuntimeScriptVerticalMenuKey) -> Result<()> {
        self.runtime.require_script_vertical_menu(key)
    }

    pub fn require_script_text_body(&self, key: &RuntimeScriptTextBodyKey) -> Result<()> {
        self.runtime.require_script_text_body(key)
    }

    pub fn require_script_menu_definition(
        &self,
        key: &RuntimeScriptMenuDefinitionKey,
    ) -> Result<()> {
        self.runtime.require_script_menu_definition(key)
    }

    pub fn require_script_elevator(&self, key: &RuntimeScriptElevatorKey) -> Result<()> {
        self.runtime.require_script_elevator(key)
    }

    pub fn require_gift_pokemon(&self, key: &RuntimeGiftPokemonKey) -> Result<()> {
        self.runtime.require_gift_pokemon(key)
    }

    pub fn require_script_object_command(&self, key: &RuntimeScriptObjectCommandKey) -> Result<()> {
        self.runtime.require_script_object_command(key)
    }

    pub fn require_script_movement(&self, key: &RuntimeScriptMovementKey) -> Result<()> {
        self.runtime.require_script_movement(key)
    }

    pub fn require_map_script_section_command(
        &self,
        key: &RuntimeMapScriptSectionCommandKey,
    ) -> Result<()> {
        self.runtime.require_map_script_section_command(key)
    }

    pub fn require_map_event_section_command(
        &self,
        key: &RuntimeMapEventSectionCommandKey,
    ) -> Result<()> {
        self.runtime.require_map_event_section_command(key)
    }

    pub fn require_script_map_command(&self, key: &RuntimeScriptMapCommandKey) -> Result<()> {
        self.runtime.require_script_map_command(key)
    }

    pub fn require_script_variable_command(
        &self,
        key: &RuntimeScriptVariableCommandKey,
    ) -> Result<()> {
        self.runtime.require_script_variable_command(key)
    }

    pub fn require_script_control_command(
        &self,
        key: &RuntimeScriptControlCommandKey,
    ) -> Result<()> {
        self.runtime.require_script_control_command(key)
    }

    pub fn require_script_swarm_command(&self, key: &RuntimeScriptSwarmCommandKey) -> Result<()> {
        self.runtime.require_script_swarm_command(key)
    }

    pub fn require_script_field_pickup(&self, key: &RuntimeScriptFieldPickupKey) -> Result<()> {
        self.runtime.require_script_field_pickup(key)
    }

    pub fn require_script_shop_command(&self, key: &RuntimeScriptShopCommandKey) -> Result<()> {
        self.runtime.require_script_shop_command(key)
    }

    pub fn require_script_phone_command(&self, key: &RuntimeScriptPhoneCommandKey) -> Result<()> {
        self.runtime.require_script_phone_command(key)
    }

    pub fn require_script_runtime_command(
        &self,
        key: &RuntimeScriptRuntimeCommandKey,
    ) -> Result<()> {
        self.runtime.require_script_runtime_command(key)
    }

    pub fn require_script_item_grant(&self, key: &RuntimeScriptItemGrantKey) -> Result<()> {
        self.runtime.require_script_item_grant(key)
    }

    pub fn require_script_item_access(&self, key: &RuntimeScriptItemAccessKey) -> Result<()> {
        self.runtime.require_script_item_access(key)
    }

    pub fn require_script_economy_command(
        &self,
        key: &RuntimeScriptEconomyCommandKey,
    ) -> Result<()> {
        self.runtime.require_script_economy_command(key)
    }

    pub fn require_script_flag_command(&self, key: &RuntimeScriptFlagCommandKey) -> Result<()> {
        self.runtime.require_script_flag_command(key)
    }

    pub fn require_script_scene_command(&self, key: &RuntimeScriptSceneCommandKey) -> Result<()> {
        self.runtime.require_script_scene_command(key)
    }

    pub fn require_script_block_change(&self, key: &RuntimeScriptBlockChangeKey) -> Result<()> {
        self.runtime.require_script_block_change(key)
    }

    pub fn require_script_audio_command(&self, key: &RuntimeScriptAudioCommandKey) -> Result<()> {
        self.runtime.require_script_audio_command(key)
    }

    pub fn require_script_text_command(&self, key: &RuntimeScriptTextCommandKey) -> Result<()> {
        self.runtime.require_script_text_command(key)
    }

    pub fn require_warp(&self, key: &RuntimeWarpKey) -> Result<()> {
        self.runtime.require_warp(key)
    }

    pub fn require_map_object(&self, key: &RuntimeMapObjectKey) -> Result<()> {
        self.runtime.require_map_object(key)
    }

    pub fn require_map_scene(&self, key: &RuntimeMapSceneKey) -> Result<()> {
        self.runtime.require_map_scene(key)
    }

    pub fn require_map_metadata(&self, key: &RuntimeMapMetadataKey) -> Result<()> {
        self.runtime.require_map_metadata(key)
    }

    pub fn require_currency_constant(&self, id: &str) -> Result<()> {
        self.runtime.require_currency_constant(id)
    }

    pub fn require_capture_ball_rule(&self, id: &str) -> Result<()> {
        self.runtime.require_capture_ball_rule(id)
    }

    pub fn require_guaranteed_capture_ball(&self, id: &str) -> Result<()> {
        self.runtime.require_guaranteed_capture_ball(id)
    }

    pub fn require_capture_status_bonus(&self, status: &str) -> Result<()> {
        self.runtime.require_capture_status_bonus(status)
    }

    pub fn require_fast_ball_species(&self, species_id: &str) -> Result<()> {
        self.runtime.require_fast_ball_species(species_id)
    }

    pub fn require_heavy_ball_species(&self, species_id: &str) -> Result<()> {
        self.runtime.require_heavy_ball_species(species_id)
    }

    pub fn require_move_priority_effect(&self, effect_id: &str) -> Result<()> {
        self.runtime.require_move_priority_effect(effect_id)
    }

    pub fn require_move_priority_move(&self, move_id: &str) -> Result<()> {
        self.runtime.require_move_priority_move(move_id)
    }

    pub fn require_capture_ball_rule_key(&self, key: &RuntimeCaptureBallRuleKey) -> Result<()> {
        self.runtime.require_capture_ball_rule_key(key)
    }

    pub fn require_heavy_ball_modifier(&self, key: &RuntimeHeavyBallModifierKey) -> Result<()> {
        self.runtime.require_heavy_ball_modifier(key)
    }

    pub fn require_capture_status_bonus_key(
        &self,
        key: &RuntimeCaptureStatusBonusKey,
    ) -> Result<()> {
        self.runtime.require_capture_status_bonus_key(key)
    }

    pub fn require_capture_wobble_probability(
        &self,
        key: &RuntimeCaptureWobbleProbabilityKey,
    ) -> Result<()> {
        self.runtime.require_capture_wobble_probability(key)
    }

    pub fn require_item_battle_use(&self, key: &RuntimeItemBattleUseKey) -> Result<()> {
        self.runtime.require_item_battle_use(key)
    }

    pub fn require_item_effect_plan(&self, key: &RuntimeItemEffectPlanKey) -> Result<()> {
        self.runtime.require_item_effect_plan(key)
    }

    pub fn require_item_field_use(&self, key: &RuntimeItemFieldUseKey) -> Result<()> {
        self.runtime.require_item_field_use(key)
    }

    pub fn require_move_battle_data(&self, key: &RuntimeMoveBattleDataKey) -> Result<()> {
        self.runtime.require_move_battle_data(key)
    }

    pub fn require_species_battle_data(&self, key: &RuntimeSpeciesBattleDataKey) -> Result<()> {
        self.runtime.require_species_battle_data(key)
    }

    pub fn require_trainer_battle_data(&self, key: &RuntimeTrainerBattleDataKey) -> Result<()> {
        self.runtime.require_trainer_battle_data(key)
    }

    pub fn require_trainer_party_pokemon(&self, key: &RuntimeTrainerPartyPokemonKey) -> Result<()> {
        self.runtime.require_trainer_party_pokemon(key)
    }

    pub fn require_move_priority_effect_key(
        &self,
        key: &RuntimeMovePriorityEffectKey,
    ) -> Result<()> {
        self.runtime.require_move_priority_effect_key(key)
    }

    pub fn require_move_priority_move_key(&self, key: &RuntimeMovePriorityMoveKey) -> Result<()> {
        self.runtime.require_move_priority_move_key(key)
    }

    pub fn require_battle_stat_multiplier(
        &self,
        key: &RuntimeBattleStatMultiplierKey,
    ) -> Result<()> {
        self.runtime.require_battle_stat_multiplier(key)
    }

    pub fn require_battle_reward_rule(&self, key: &RuntimeBattleRewardRuleKey) -> Result<()> {
        self.runtime.require_battle_reward_rule(key)
    }

    pub fn require_battle_escape_rule(&self, key: &RuntimeBattleEscapeRuleKey) -> Result<()> {
        self.runtime.require_battle_escape_rule(key)
    }

    pub fn require_physical_type(&self, type_id: &str) -> Result<()> {
        self.runtime.require_physical_type(type_id)
    }

    pub fn require_special_type(&self, type_id: &str) -> Result<()> {
        self.runtime.require_special_type(type_id)
    }

    pub fn require_weather(&self, weather_id: &str) -> Result<()> {
        self.runtime.require_weather(weather_id)
    }

    pub fn require_type_effectiveness(&self, key: &RuntimeTypeEffectivenessKey) -> Result<()> {
        self.runtime.require_type_effectiveness(key)
    }

    pub fn require_foresight_type_effectiveness(
        &self,
        key: &RuntimeTypeEffectivenessKey,
    ) -> Result<()> {
        self.runtime.require_foresight_type_effectiveness(key)
    }

    pub fn require_weather_type_modifier(&self, key: &RuntimeWeatherTypeModifierKey) -> Result<()> {
        self.runtime.require_weather_type_modifier(key)
    }

    pub fn require_weather_move_effect_modifier(
        &self,
        key: &RuntimeWeatherMoveEffectModifierKey,
    ) -> Result<()> {
        self.runtime.require_weather_move_effect_modifier(key)
    }

    pub fn require_audio_asset(&self, key: &RuntimeAudioAssetKey) -> Result<()> {
        self.runtime.require_audio_asset(key)
    }

    pub fn require_pokemon_cry(&self, key: &RuntimePokemonCryKey) -> Result<()> {
        self.runtime.require_pokemon_cry(key)
    }

    pub fn require_music(&self, music_id: &str) -> Result<()> {
        self.runtime.require_music(music_id)
    }

    pub fn require_sound_effect(&self, sound_effect_id: &str) -> Result<()> {
        self.runtime.require_sound_effect(sound_effect_id)
    }

    pub fn require_cry(&self, cry_id: &str) -> Result<()> {
        self.runtime.require_cry(cry_id)
    }

    #[cfg(test)]
    pub fn apply_special_routine(&mut self, routine: &str) -> Result<RuntimeSpecialRoutineUse> {
        self.runtime.require_special_routine(routine)?;
        let mutation = self.apply_special_routine_runtime_mutation(routine)?;
        let RuntimeMutationResult::SpecialRoutineApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-routine result");
        };
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_day_care(
        &mut self,
        caretaker: RuntimeDayCareCaretaker,
        action: RuntimeDayCareAction,
        party_index: Option<usize>,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::UseDayCare(
            RuntimeDayCareCommand {
                caretaker,
                action,
                party_index,
            },
        ))?;
        let RuntimeMutationResult::DayCareUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-day-care result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Day Care use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_day_care_man_outside_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckDayCareManOutsideSpecial,
        )?;
        let RuntimeMutationResult::DayCareManOutsideChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-day-care-man-outside result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Day Care man outside check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_day_care_resident_special(
        &mut self,
        caretaker: RuntimeDayCareCaretaker,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckDayCareResidentSpecial(caretaker),
        )?;
        let RuntimeMutationResult::DayCareResidentChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-day-care-resident result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Day Care resident check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bug_contest(
        &mut self,
        action: RuntimeBugContestAction,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = match action {
            RuntimeBugContestAction::SelectContestants | RuntimeBugContestAction::Judge => {
                let recorded = self
                    .session
                    .stage_random_bug_contest(&self.runtime, action)?;
                self.apply_recorded_runtime_mutation(recorded)?
            }
            RuntimeBugContestAction::GiveParkBalls
            | RuntimeBugContestAction::DropOffMons
            | RuntimeBugContestAction::ReturnMons
            | RuntimeBugContestAction::CheckPartyFull => self.apply_runtime_mutation_command(
                RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand {
                    action,
                    divider_trace: None,
                }),
            )?,
        };
        let RuntimeMutationResult::BugContestUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-bug-contest result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Bug Contest use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_kurt_apricorn(
        &mut self,
        apricorn_id: String,
        quantity: u16,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseKurtApricorn(RuntimeKurtApricornCommand {
                apricorn_id,
                quantity,
            }),
        )?;
        let RuntimeMutationResult::KurtApricornUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Kurt-apricorn result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Kurt apricorn use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_buena_password(
        &mut self,
        guess: Option<String>,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let recorded = self.session.stage_buena_password(&self.runtime, guess)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::BuenaPasswordUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Buena-password result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Buena password use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_current_buena_password_guess(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        self.use_buena_password(None)?;
        let guess = self
            .session
            .state
            .script_runtime
            .variables
            .get("_buena_password")
            .cloned()
            .context("BuenasPassword did not produce _buena_password")?;
        self.use_buena_password(Some(guess))
    }

    pub fn use_buena_prize(
        &mut self,
        item_id: String,
        quantity: u16,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBuenaPrize(RuntimeBuenaPrizeCommand { item_id, quantity }),
        )?;
        let RuntimeMutationResult::BuenaPrizeUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Buena-prize result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Buena prize use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_shuckie(
        &mut self,
        action: RuntimeShuckieAction,
        party_index: Option<usize>,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = if matches!(action, RuntimeShuckieAction::Give) {
            if party_index.is_some() {
                anyhow::bail!("Shuckie give must not select a party index");
            }
            let recorded = self.session.stage_shuckie_give(&self.runtime)?;
            self.apply_recorded_runtime_mutation(recorded)?
        } else {
            self.apply_runtime_mutation_command(RuntimeMutationCommand::UseShuckie(
                RuntimeShuckieCommand {
                    action,
                    party_index,
                    divider_trace: None,
                },
            ))?
        };
        let RuntimeMutationResult::ShuckieUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Shuckie result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Shuckie use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn give_odd_egg(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let recorded = self.session.stage_odd_egg(&self.runtime)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::OddEggGiven(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Odd-Egg result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Odd Egg gift")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn give_dratini(&mut self, mode: u8) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::GiveDratini(
            RuntimeGiveDratiniCommand { mode },
        ))?;
        let RuntimeMutationResult::DratiniGiven(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Dratini result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Dratini gift")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bills_grandfather(
        &mut self,
        party_index: Option<usize>,
        species_id: Option<String>,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBillsGrandfather(RuntimeBillsGrandfatherCommand {
                party_index,
                species_id,
            }),
        )?;
        let RuntimeMutationResult::BillsGrandfatherUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Bills-Grandfather result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Bill's Grandfather use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn init_roam_mons(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::InitRoamMons)?;
        let RuntimeMutationResult::RoamersInitialized(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-roamer-init result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after roamer init")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_magikarp_length(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CheckMagikarpLength(
                RuntimeMagikarpLengthCommand { party_index },
            ))?;
        let RuntimeMutationResult::MagikarpLengthChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Magikarp-length result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Magikarp length check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn show_prof_oaks_pc_boot(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ShowProfOaksPcBoot)?;
        let RuntimeMutationResult::ProfOaksPcBootShown(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Prof-Oak-PC result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Prof Oak PC boot")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn show_magikarp_house_sign(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ShowMagikarpHouseSign)?;
        let RuntimeMutationResult::MagikarpHouseSignShown(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Magikarp-house-sign result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Magikarp house sign")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_battle_tower_action(
        &mut self,
        action: String,
        level_group: Option<u8>,
        selected_reward: Option<String>,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyBattleTowerAction(RuntimeBattleTowerActionCommand {
                action,
                level_group,
                selected_reward,
            }),
        )?;
        let RuntimeMutationResult::BattleTowerActionApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-action result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower action")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_battle_tower_room_menu_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::OpenBattleTowerRoomMenuSpecial,
        )?;
        let RuntimeMutationResult::BattleTowerRoomMenuOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-room-menu result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower room menu")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn start_battle_tower_battle_special(
        &mut self,
        battle_result: u8,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::StartBattleTowerBattleSpecial(
                RuntimeBattleTowerBattleCommand { battle_result },
            ),
        )?;
        let RuntimeMutationResult::BattleTowerBattleStarted(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-battle result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower battle")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn load_battle_tower_opponent_special(
        &mut self,
        trainer_id: String,
        sprite_constant: String,
        target_object: String,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let recorded = self.session.stage_battle_tower_opponent(
            &self.runtime,
            trainer_id,
            sprite_constant,
            target_object,
        )?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::BattleTowerOpponentLoaded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-opponent result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower opponent load")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn show_battle_tower_mobile_error_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ShowBattleTowerMobileErrorSpecial,
        )?;
        let RuntimeMutationResult::BattleTowerMobileErrorShown(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-mobile-error result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower mobile error")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn ask_remember_password_special(
        &mut self,
        remember: bool,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::AskRememberPasswordSpecial(RuntimeRememberPasswordCommand {
                remember,
            }),
        )?;
        let RuntimeMutationResult::RememberPasswordAsked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-remember-password result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after remember-password prompt")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_battle_tower_leaderboard_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::OpenBattleTowerLeaderboardSpecial,
        )?;
        let RuntimeMutationResult::BattleTowerLeaderboardOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-leaderboard result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower leaderboard")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_mobile_handshake_special(
        &mut self,
        command: RuntimeMobileHandshakeCommand,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyMobileHandshakeSpecial(command),
        )?;
        let RuntimeMutationResult::MobileHandshakeApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-mobile-handshake result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after mobile handshake")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn end_mobile_session_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::EndMobileSessionSpecial)?;
        let RuntimeMutationResult::MobileSessionEnded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-mobile-session-end result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after mobile session end")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_battle_tower_mobile_flag_special(
        &mut self,
        flag: RuntimeBattleTowerMobileFlag,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SetBattleTowerMobileFlagSpecial(flag),
        )?;
        let RuntimeMutationResult::BattleTowerMobileFlagSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-mobile-flag result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower mobile flag")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn select_three_mobile_mons_special(
        &mut self,
        party_indexes: [usize; 3],
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SelectThreeMobileMonsSpecial(
                RuntimeMobileSelectThreeMonsCommand { party_indexes },
            ),
        )?;
        let RuntimeMutationResult::MobileThreeMonsSelected(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-mobile-three-mons result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after mobile three-mon selection")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_happiness_service(
        &mut self,
        routine: RuntimeHappinessServiceRoutine,
        party_index: usize,
        rng_roll: u8,
        rng_seed_after: u32,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyHappinessService(RuntimeHappinessServiceCommand {
                routine,
                party_index,
                rng_roll,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::HappinessServiceApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-happiness-service result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after happiness service")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_mystery_gift(
        &mut self,
        action: RuntimeMysteryGiftAction,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::UseMysteryGift(action))?;
        let RuntimeMutationResult::MysteryGiftUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Mystery-Gift result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Mystery Gift use")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn warp_to_spawn_point(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::WarpToSpawnPoint)?;
        let RuntimeMutationResult::SpawnPointWarped(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-spawn-warp result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after spawn warp")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn heal_party_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::HealPartySpecial)?;
        let RuntimeMutationResult::PartyHealedBySpecial(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-heal result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after special party heal")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn fade_out_music_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::FadeOutMusicSpecial)?;
        let RuntimeMutationResult::MusicFadedOutBySpecial(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-music-fade result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after special music fade")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn wait_sfx_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::WaitSfxSpecial)?;
        let RuntimeMutationResult::SoundEffectWaitQueued(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-sfx-wait result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after special sfx wait")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn play_map_music_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::PlayMapMusicSpecial)?;
        let RuntimeMutationResult::MapMusicPlayedBySpecial(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-map-music result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after special map music")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn restart_map_music_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::RestartMapMusicSpecial)?;
        let RuntimeMutationResult::MapMusicRestartedBySpecial(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-map-music-restart result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after special map music restart")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn play_cur_mon_cry(&mut self, species_id: String) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::PlayCurMonCry(RuntimeSpecialCryCommand { species_id }),
        )?;
        let RuntimeMutationResult::CurrentMonCryPlayed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-current-mon-cry result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after current-mon cry")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn play_slow_cry(&mut self, species_id: String) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::PlaySlowCry(
            RuntimeSpecialCryCommand { species_id },
        ))?;
        let RuntimeMutationResult::SlowCryPlayed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-slow-cry result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after slow cry")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_pokemon_center_pc_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::OpenPokemonCenterPcSpecial)?;
        let RuntimeMutationResult::PokemonCenterPcOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Pokemon-Center-PC result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Pokemon Center PC")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_players_house_pc_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::OpenPlayersHousePcSpecial)?;
        let RuntimeMutationResult::PlayersHousePcOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Players-House-PC result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after player's house PC")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_overworld_town_map_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::OpenOverworldTownMapSpecial)?;
        let RuntimeMutationResult::OverworldTownMapOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-overworld-town-map result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after overworld town map")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_unown_printer_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::OpenUnownPrinterSpecial)?;
        let RuntimeMutationResult::UnownPrinterOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Unown-Printer result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Unown Printer")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_map_radio_special(&mut self, station: String) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::OpenMapRadioSpecial(RuntimeMapRadioCommand { station }),
        )?;
        let RuntimeMutationResult::MapRadioOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Map-Radio result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Map Radio")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn name_rival_special(&mut self, rival_name: String) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::NameRivalSpecial(RuntimeNameRivalCommand { rival_name }),
        )?;
        let RuntimeMutationResult::RivalNamed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-rival-name result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after rival name")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn delete_party_move_special(
        &mut self,
        party_index: usize,
        move_index: usize,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::DeletePartyMoveSpecial(RuntimeMoveDeletionCommand {
                party_index,
                move_index,
            }),
        )?;
        let RuntimeMutationResult::PartyMoveDeletedBySpecial(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-move-deletion result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after move deletion")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_pokerus_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CheckPokerusSpecial)?;
        let RuntimeMutationResult::PokerusChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Pokerus-check result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Pokerus check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn rate_party_nickname_special(
        &mut self,
        party_index: usize,
        nickname: String,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::RatePartyNicknameSpecial(RuntimePartyNicknameCommand {
                party_index,
                nickname,
            }),
        )?;
        let RuntimeMutationResult::PartyNicknameRated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-name-rater result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after name rater")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn see_party_pokemon_special(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SeePartyPokemonSpecial(RuntimePartySlotCommand { party_index }),
        )?;
        let RuntimeMutationResult::PartyPokemonSeenBySeer(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Poke-Seer result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Poke Seer")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn teach_party_move_special(
        &mut self,
        party_index: usize,
        move_id: String,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::TeachPartyMoveSpecial(RuntimeMoveTutorCommand {
                party_index,
                move_id,
            }),
        )?;
        let RuntimeMutationResult::PartyMoveTaughtBySpecial(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-move-tutor result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after move tutor")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_bank_of_mom_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::OpenBankOfMomSpecial)?;
        let RuntimeMutationResult::BankOfMomOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Bank-of-Mom result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Bank of Mom")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_game_corner_special(
        &mut self,
        service: RuntimeGameCornerService,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = match service {
            RuntimeGameCornerService::CardFlip => {
                let recorded = self.session.stage_card_flip(&self.runtime)?;
                self.apply_recorded_runtime_mutation(recorded)?
            }
            RuntimeGameCornerService::SlotMachine => self.apply_runtime_mutation_command(
                RuntimeMutationCommand::OpenGameCornerSpecial(RuntimeGameCornerCommand {
                    service,
                    divider_trace: None,
                }),
            )?,
        };
        let RuntimeMutationResult::GameCornerOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Game-Corner result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Game Corner service")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_display_link_record_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::OpenDisplayLinkRecordSpecial)?;
        let RuntimeMutationResult::DisplayLinkRecordOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-display-link-record result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after display link record")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_trainer_house_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::OpenTrainerHouseSpecial)?;
        let RuntimeMutationResult::TrainerHouseOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Trainer-House result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Trainer House")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_photo_studio_special(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::OpenPhotoStudioSpecial(RuntimePartySlotCommand { party_index }),
        )?;
        let RuntimeMutationResult::PhotoStudioOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Photo-Studio result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Photo Studio")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn cancel_battle_tower_challenge_explanation_special(
        &mut self,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CancelBattleTowerChallengeExplanationSpecial,
        )?;
        let RuntimeMutationResult::BattleTowerChallengeExplanationCancelled(outcome) =
            mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-Battle-Tower-explanation result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Battle Tower explanation cancel")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_declared_special_routine(
        &mut self,
        routine: &str,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_special_routine_runtime_mutation(routine)?;
        let RuntimeMutationResult::SpecialRoutineApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-routine result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .with_context(|| format!("validate runtime state after special routine {routine}"))?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_graphics_special(
        &mut self,
        special: RuntimeGraphicsSpecial,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyGraphicsSpecial(special),
        )?;
        let RuntimeMutationResult::GraphicsSpecialApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-graphics-special result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after graphics special")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_party_check_special(
        &mut self,
        special: RuntimePartyCheckSpecial,
        species_id: Option<String>,
        threshold: Option<u8>,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyPartyCheckSpecial(RuntimePartyCheckCommand {
                special,
                species_id,
                threshold,
            }),
        )?;
        let RuntimeMutationResult::PartyCheckSpecialApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-check-special result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party check special")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_phone_random_special(
        &mut self,
        special: RuntimePhoneRandomSpecial,
        contact_id: String,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let recorded =
            self.session
                .stage_phone_random_special(&self.runtime, special, contact_id)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::PhoneRandomSpecialApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-phone-random-special result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after phone random special")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_item_in_pc_or_bag_special(
        &mut self,
        item_id: String,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckItemInPcOrBagSpecial(RuntimePcBagItemCheckCommand {
                item_id,
            }),
        )?;
        let RuntimeMutationResult::ItemInPcOrBagChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-bag-item-check result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC/bag item check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_another_usable_party_mon_special(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckAnotherUsablePartyMonSpecial(RuntimePartySlotCommand {
                party_index,
            }),
        )?;
        let RuntimeMutationResult::AnotherUsablePartyMonChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-another-usable-party-mon result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after usable party mon check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn activate_fishing_swarm_special(
        &mut self,
        value: u8,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ActivateFishingSwarmSpecial(RuntimeFishingSwarmCommand {
                value,
            }),
        )?;
        let RuntimeMutationResult::FishingSwarmActivated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-fishing-swarm result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after fishing swarm activation")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_story_gate_special(
        &mut self,
        special: RuntimeStoryGateSpecial,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyStoryGateSpecial(special),
        )?;
        let RuntimeMutationResult::StoryGateSpecialApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-story-gate-special result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after story gate special")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_player_palette(&mut self, raw_value: u8) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SetPlayerPalette(RuntimePlayerPaletteCommand { raw_value }),
        )?;
        let RuntimeMutationResult::PlayerPaletteSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-player-palette result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after player palette set")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_day_of_week(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::SetDayOfWeek)?;
        let RuntimeMutationResult::DayOfWeekSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-day-of-week result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after day-of-week set")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn update_time(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::UpdateTime)?;
        let RuntimeMutationResult::TimeUpdated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-time-update result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after time update")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_runtime_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
    ) -> Result<RuntimeScriptRuntimeCommand> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptRuntime {
                command: RuntimeScriptCommandRef::new(map_name, source_script, command_index),
                inputs,
            })?;
        let RuntimeMutationResult::ScriptRuntimeApplied(_, outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-runtime result");
        };
        Ok(RuntimeScriptRuntimeCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn execute_next_queued_script_command(&mut self) -> Result<RuntimeQueuedScriptCommand> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ExecuteNextQueuedScriptCommand,
        )?;
        let RuntimeMutationResult::QueuedScriptCommandExecuted(queued) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-queued-script-command result");
        };
        Ok(RuntimeQueuedScriptCommand {
            queued,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_next_script(&mut self) -> Result<RuntimeNextScript> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::TakeNextScript)?;
        let RuntimeMutationResult::NextScriptTaken(location) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-next-script result");
        };
        Ok(RuntimeNextScript {
            origin_map_name: location.origin_map_name,
            script: location.script,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn pop_script_call_stack(&mut self) -> Result<RuntimeScriptReturnResume> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::PopScriptCallStack)?;
        let RuntimeMutationResult::ScriptCallStackPopped(frame) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-call-stack-pop result");
        };
        Ok(RuntimeScriptReturnResume {
            frame,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn pop_deferred_script(&mut self) -> Result<RuntimeDeferredScript> {
        self.session.pop_deferred_script(&self.runtime)
    }

    pub fn take_script_end_state(&mut self) -> Result<RuntimeScriptEnd> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::TakeScriptEndState)?;
        let RuntimeMutationResult::ScriptEndStateTaken(end) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-end-state-take result");
        };
        Ok(RuntimeScriptEnd {
            end,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_swarm_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptSwarm> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptSwarm(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptSwarmApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-swarm result");
        };
        Ok(RuntimeScriptSwarm {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn grant_script_item(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemGrant> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::GrantScriptItem(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptItemGranted(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-item-grant result");
        };
        Ok(RuntimeScriptItemGrant {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_script_item(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemCheck> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CheckScriptItem(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptItemChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-item-check result");
        };
        Ok(RuntimeScriptItemCheck {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_script_item(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemTake> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::TakeScriptItem(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptItemTaken(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-item-take result");
        };
        Ok(RuntimeScriptItemTake {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn pickup_script_field_item(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFieldPickup> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::PickupScriptFieldItem(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptFieldItemPickedUp(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-field-pickup result");
        };
        Ok(RuntimeFieldPickup {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_economy_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptEconomy> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptEconomy(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptEconomyApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-economy result");
        };
        Ok(RuntimeScriptEconomy {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn initialize_permanent_phone_numbers(&mut self) -> Result<RuntimePermanentPhoneNumbers> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::InitializePermanentPhoneNumbers,
        )?;
        let RuntimeMutationResult::PermanentPhoneNumbersInitialized(inserted) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-permanent-phone-number result");
        };
        Ok(RuntimePermanentPhoneNumbers {
            inserted,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_phone_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptPhoneInputs,
    ) -> Result<RuntimePhoneCommand> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptPhone {
                command: RuntimeScriptCommandRef::new(map_name, source_script, command_index),
                inputs,
            })?;
        let RuntimeMutationResult::ScriptPhoneApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-phone result");
        };
        Ok(RuntimePhoneCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn grant_scripted_gift_pokemon(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        rng_seed_after: u32,
        nickname_accepted: bool,
        nickname: Option<String>,
    ) -> Result<RuntimeGiftPokemonGrant> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::GrantScriptedGiftPokemon(RuntimeGiftPokemonCommand {
                command: RuntimeScriptCommandRef::new(map_name, source_script, command_index),
                original_trainer_name: original_trainer_name.into(),
                original_trainer_id,
                dvs,
                rng_seed_after,
                nickname_accepted,
                nickname,
            }),
        )?;
        let RuntimeMutationResult::ScriptedGiftPokemonGranted(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-gift-pokemon result");
        };
        Ok(RuntimeGiftPokemonGrant {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn add_party_pokemon(
        &mut self,
        species_id: &str,
        level: u8,
        held_item_id: Option<String>,
        nickname: Option<String>,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
    ) -> Result<RuntimeGiftPokemonGrant> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::AddPartyPokemon(RuntimePartyPokemonCommand {
                species_id: species_id.to_string(),
                level,
                held_item_id,
                nickname,
                original_trainer_name: original_trainer_name.into(),
                original_trainer_id,
                dvs,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonAdded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-pokemon-add result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party Pokemon add")?;
        Ok(RuntimeGiftPokemonGrant {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_flag_mutation(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFlagMutation> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptFlagMutation(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptFlagMutated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-flag-mutation result");
        };
        Ok(RuntimeFlagMutation {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_script_flag(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFlagCheck> {
        self.session
            .check_script_flag(&self.runtime, map_name, source_script, command_index)
    }

    pub fn apply_script_scene_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeSceneCommand> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptScene(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptSceneApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-scene result");
        };
        Ok(RuntimeSceneCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_block_change(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeBlockChange> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptBlockChange(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptBlockChanged(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-block-change result");
        };
        Ok(RuntimeBlockChange {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_audio_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptAudio> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptAudio(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptAudioApplied(cue) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-audio result");
        };
        Ok(RuntimeScriptAudio {
            cue,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_map_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptMapCommand> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptMap(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptMapApplied(action) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-map result");
        };
        Ok(RuntimeScriptMapCommand {
            action,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn execute_pending_script_warp(&mut self) -> Result<RuntimeScriptWarp> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::TransitionPendingScriptWarp)?;
        let RuntimeMutationResult::PendingScriptWarpTransitioned(request) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-warp result");
        };
        Ok(RuntimeScriptWarp {
            target_map: request.target_map,
            tile: request.tile,
            facing: request.facing,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_text_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptText> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptText(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptTextApplied(action) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-text result");
        };
        Ok(RuntimeScriptText {
            action,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_variable_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptVariable> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptVariableNow(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptVariableApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-variable result");
        };
        Ok(RuntimeScriptVariable {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_script_runtime_variable(
        &mut self,
        key: &str,
        value: impl Into<String>,
    ) -> Result<()> {
        self.session
            .state
            .script_runtime
            .variables
            .insert(key.to_string(), value.into());
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .with_context(|| format!("validate script runtime variable {key}"))?;
        Ok(())
    }

    pub fn set_script_runtime_accumulator(&mut self, value: impl Into<String>) -> Result<()> {
        let value = value.into();
        self.session.state.script_runtime.script_value = Some(value.clone());
        self.session
            .state
            .script_runtime
            .memory
            .insert("wScriptVar".to_string(), value);
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate script runtime accumulator")?;
        Ok(())
    }

    pub fn apply_script_control_command(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptControl> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptControl(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptControlApplied(action) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-control result");
        };
        Ok(RuntimeScriptControl {
            action,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_object_mutation(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptObjectMutation> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ApplyScriptObjectMutation(RuntimeScriptCommandRef::new(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptObjectMutated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-object result");
        };
        Ok(RuntimeScriptObjectMutation {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_movement(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptMovement> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ApplyScriptMovement(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptMovementApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-movement result");
        };
        Ok(RuntimeScriptMovement {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn update_clock_from_datetime(
        &mut self,
        date: GameDate,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<RuntimeTimeUpdate> {
        let recorded =
            self.session
                .stage_clock_update(&self.runtime, date, hour, minute, second)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ClockUpdated = mutation.result else {
            anyhow::bail!("runtime mutation returned non-clock-update result");
        };
        Ok(RuntimeTimeUpdate {
            time_of_day: self.session.state.time.time_of_day,
            day_of_week: self.session.state.time.day_of_week,
            hour: self.session.state.time.registers.hours,
            minute: self.session.state.time.registers.minutes,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn advance_game_timer_vblank(&mut self) -> Result<RuntimeGameTimerOutcome> {
        self.advance_game_timer_vblanks(1)
    }

    pub fn advance_game_timer_vblanks(&mut self, vblanks: u32) -> Result<RuntimeGameTimerOutcome> {
        if vblanks == 0 {
            anyhow::bail!("game timer advance requires a nonzero VBlank count");
        }
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::AdvanceGameTimerVBlanks(
                RuntimeGameTimerAdvanceCommand { vblanks },
            ))?;
        let RuntimeMutationResult::GameTimerVBlanksAdvanced(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-game-timer-vblanks result");
        };
        Ok(outcome)
    }

    pub fn set_game_timer_counting(&mut self, counting: bool) -> Result<RuntimeGameTimerOutcome> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::SetGameTimerCounting(
                RuntimeGameTimerCountingCommand { counting },
            ))?;
        let RuntimeMutationResult::GameTimerCountingSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-game-timer-counting result");
        };
        Ok(outcome)
    }

    pub fn set_game_logic_paused(&mut self, paused: bool) -> Result<RuntimeGameTimerOutcome> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SetGameLogicPaused(RuntimeGameLogicPauseCommand { paused }),
        )?;
        let RuntimeMutationResult::GameLogicPauseSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-game-logic-pause result");
        };
        Ok(outcome)
    }

    pub fn set_manual_clock_time(
        &mut self,
        now_date: GameDate,
        now_hour: u8,
        now_minute: u8,
        now_second: u8,
        target: ClockTime,
    ) -> Result<RuntimeTimeUpdate> {
        let recorded = self.session.stage_manual_clock_update(
            &self.runtime,
            now_date,
            now_hour,
            now_minute,
            now_second,
            target,
        )?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ManualClockSet = mutation.result else {
            anyhow::bail!("runtime mutation returned non-manual-clock result");
        };
        Ok(RuntimeTimeUpdate {
            time_of_day: self.session.state.time.time_of_day,
            day_of_week: self.session.state.time.day_of_week,
            hour: self.session.state.time.registers.hours,
            minute: self.session.state.time.registers.minutes,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_script_shop(
        &mut self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptShop> {
        self.require_valid_script_modal_state("open script shop")?;
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::OpenScriptShop(
                RuntimeScriptCommandRef::new(map_name, source_script, command_index),
            ))?;
        let RuntimeMutationResult::ScriptShopOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-shop result");
        };
        Ok(RuntimeScriptShop {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn buy_shop_item(
        &mut self,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeShopTransaction> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::BuyShopItem(
            RuntimeShopTransactionCommand {
                item_id: item_id.to_string(),
                quantity,
            },
        ))?;
        let RuntimeMutationResult::ShopItemBought(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-shop-purchase result");
        };
        Ok(RuntimeShopTransaction {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn sell_shop_item(
        &mut self,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeShopTransaction> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SellShopItem(RuntimeShopTransactionCommand {
                item_id: item_id.to_string(),
                quantity,
            }),
        )?;
        let RuntimeMutationResult::ShopItemSold(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-shop-sale result");
        };
        Ok(RuntimeShopTransaction {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item(
        &mut self,
        item_id: &str,
        context: ItemUseContext,
    ) -> Result<RuntimeItemUse> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::UseBagItem {
            item_id: item_id.to_string(),
            context,
        })?;
        let RuntimeMutationResult::BagItemUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-item-use result");
        };
        Ok(RuntimeItemUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn register_key_item(&mut self, item_id: &str) -> Result<RuntimeRegisteredKeyItem> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::RegisterKeyItem(RuntimeRegisteredKeyItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::KeyItemRegistered(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-key-item-registration result");
        };
        Ok(RuntimeRegisteredKeyItem {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_repel_in_field(&mut self, item_id: &str) -> Result<RuntimeRepelItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagRepelInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldRepelUsed(repel) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-repel result");
        };
        Ok(RuntimeRepelItemUse {
            item_use: repel.item_use,
            repel_steps_before: repel.repel_steps_before,
            repel_steps_after: repel.repel_steps_after,
            active_repel_item_before: repel.active_repel_item_before,
            active_repel_item_after: repel.active_repel_item_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_bicycle_in_field(&mut self, item_id: &str) -> Result<RuntimeBicycleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagBicycleInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldBicycleUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-bicycle result");
        };
        Ok(RuntimeBicycleItemUse {
            item_use: outcome.item_use,
            map_name: outcome.map_name,
            permission: outcome.permission,
            mode_before: outcome.mode_before,
            mode_after: outcome.mode_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_itemfinder_in_field(&mut self, item_id: &str) -> Result<RuntimeItemfinderUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemfinderInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldItemfinderUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-itemfinder result");
        };
        Ok(RuntimeItemfinderUse {
            item_use: outcome.item_use,
            player_tile: outcome.player_tile,
            itemfinder_sound_cues: outcome.itemfinder_sound_cues,
            found: outcome.found,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_squirtbottle_in_field(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeSquirtBottleUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagSquirtbottleInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldSquirtbottleUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-squirtbottle result");
        };
        Ok(RuntimeSquirtBottleUse {
            item_use: outcome.item_use,
            player_tile: outcome.player_tile,
            target_tile: outcome.target_tile,
            target_object_identifier: outcome.target_object_identifier,
            target_movement: outcome.target_movement,
            target_script: outcome.target_script,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_coin_case_in_field(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeKeyItemBalanceUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagCoinCaseInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldCoinCaseUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-coin-case result");
        };
        Ok(RuntimeKeyItemBalanceUse {
            item_use: outcome.item_use,
            balance_label: outcome.balance_label,
            balance: outcome.balance,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_blue_card_in_field(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeKeyItemBalanceUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagBlueCardInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldBlueCardUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-blue-card result");
        };
        Ok(RuntimeKeyItemBalanceUse {
            item_use: outcome.item_use,
            balance_label: outcome.balance_label,
            balance: outcome.balance,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_town_map_in_field(&mut self, item_id: &str) -> Result<RuntimeTownMapUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagTownMapInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldTownMapUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-town-map result");
        };
        Ok(RuntimeTownMapUse {
            item_use: outcome.item_use,
            map_name: outcome.map_name,
            map_constant: outcome.map_constant,
            environment: outcome.environment,
            landmark: outcome.landmark,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn is_bag_pokegear_item(&self, item_id: &str) -> bool {
        self.runtime.data.field_moves.pokegear.item_id == item_id
    }

    pub fn is_bag_box_item(&self, item_id: &str) -> bool {
        self.runtime.data.field_box_items.contains_key(item_id)
    }

    pub fn use_bag_pokegear_in_field(&mut self, item_id: &str) -> Result<RuntimePokegearUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagPokegearInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldPokegearUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-pokegear result");
        };
        Ok(RuntimePokegearUse {
            item_use: outcome.item_use,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_box_in_field(&mut self, item_id: &str) -> Result<RuntimeBoxItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagBoxInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldBoxUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-box result");
        };
        Ok(RuntimeBoxItemUse {
            item_use: outcome.item_use,
            decoration_flag: outcome.decoration_flag,
            already_owned: outcome.already_owned,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_escape_rope_in_field(&mut self, item_id: &str) -> Result<RuntimeEscapeRopeUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagEscapeRopeInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldEscapeRopeUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-escape-rope result");
        };
        Ok(RuntimeEscapeRopeUse {
            item_use: outcome.item_use,
            source_map: outcome.source_map,
            destination_map: outcome.destination_map,
            destination_warp_index: outcome.destination_warp_index,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn cast_fishing_rod(&mut self, rod: &str) -> Result<RuntimeFishingCast> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CastFishingRod {
                rod: rod.to_string(),
            })?;
        let RuntimeMutationResult::FishingRodCast(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-fishing-cast result");
        };
        Ok(RuntimeFishingCast {
            session: outcome.session,
            bite: outcome.bite,
            wild_battle: outcome.wild_battle,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_fishing_rod_in_field(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeFishingRodItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagFishingRodInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::BagFishingRodUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-fishing-rod-item result");
        };
        Ok(RuntimeFishingRodItemUse {
            item_use: outcome.item_use,
            rod: outcome.rod,
            cast: RuntimeFishingCast {
                session: outcome.cast.session,
                bite: outcome.cast.bite,
                wild_battle: outcome.cast.wild_battle,
                state_checksum: outcome.cast_state_checksum,
            },
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_cut_field_move(
        &mut self,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseCutFieldMove(RuntimeFieldBlockMoveCommand {
                party_index,
                metatile_x,
                metatile_y,
            }),
        )?;
        let RuntimeMutationResult::CutFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-CUT result");
        };
        Ok(RuntimeFieldMoveBlockUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_cut_field_move_in_front(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let (metatile_x, metatile_y) = self
            .runtime
            .data()
            .field_block_target_metatile_in_front(&self.session.overworld)?;
        self.use_cut_field_move(party_index, metatile_x, metatile_y)
    }

    pub fn use_whirlpool_field_move(
        &mut self,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseWhirlpoolFieldMove(RuntimeFieldBlockMoveCommand {
                party_index,
                metatile_x,
                metatile_y,
            }),
        )?;
        let RuntimeMutationResult::WhirlpoolFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-WHIRLPOOL result");
        };
        Ok(RuntimeFieldMoveBlockUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_whirlpool_field_move_in_front(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let (metatile_x, metatile_y) = self
            .runtime
            .data()
            .field_block_target_metatile_in_front(&self.session.overworld)?;
        self.use_whirlpool_field_move(party_index, metatile_x, metatile_y)
    }

    pub fn use_strength_field_move(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveFlagUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseStrengthFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::StrengthFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-STRENGTH result");
        };
        Ok(RuntimeFieldMoveFlagUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_flash_field_move(&mut self, party_index: usize) -> Result<RuntimeFieldMoveFlagUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseFlashFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::FlashFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-FLASH result");
        };
        Ok(RuntimeFieldMoveFlagUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_surf_field_move(&mut self, party_index: usize) -> Result<RuntimeFieldMoveTravelUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseSurfFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::SurfFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-SURF result");
        };
        Ok(RuntimeFieldMoveTravelUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_waterfall_field_move(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveTravelUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseWaterfallFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::WaterfallFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-WATERFALL result");
        };
        Ok(RuntimeFieldMoveTravelUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_fly_field_move(
        &mut self,
        party_index: usize,
        destination_spawn_identifier: u16,
        flypoint_flag: &str,
    ) -> Result<RuntimeFlyFieldMoveUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseFlyFieldMove(RuntimeFlyCommand {
                party_index,
                destination_spawn_identifier,
                flypoint_flag: flypoint_flag.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FlyFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-FLY result");
        };
        Ok(RuntimeFlyFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            flypoint_flag: outcome.flypoint_flag,
            source_map: outcome.source_map,
            destination_spawn_identifier: outcome.destination_spawn_identifier,
            destination_map: outcome.destination_map,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_dig_field_move(&mut self, party_index: usize) -> Result<RuntimeDigFieldMoveUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseDigFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::DigFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-DIG result");
        };
        Ok(RuntimeDigFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            source_map: outcome.source_map,
            destination_map: outcome.destination_map,
            destination_warp_index: outcome.destination_warp_index,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_teleport_field_move(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeTeleportFieldMoveUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseTeleportFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::TeleportFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-TELEPORT result");
        };
        Ok(RuntimeTeleportFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            source_map: outcome.source_map,
            destination_spawn_identifier: outcome.destination_spawn_identifier,
            destination_map: outcome.destination_map,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_headbutt_field_move(
        &mut self,
        party_index: usize,
        player_id: u16,
    ) -> Result<RuntimeFieldEncounterMoveUse> {
        let rng_seed_after = preview_headbutt_field_move_rng_seed_after(
            self.runtime.data(),
            &self.session.state,
            &self.session.overworld,
            party_index,
            player_id,
        )?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseHeadbuttFieldMove(RuntimeHeadbuttFieldEncounterCommand {
                party_index,
                player_id,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::HeadbuttFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-HEADBUTT result");
        };
        Ok(RuntimeFieldEncounterMoveUse {
            field_encounter: outcome.field_encounter,
            wild_battle: outcome.wild_battle,
            removed_object_identifier: outcome.removed_object_identifier,
            removed_event_flag: outcome.removed_event_flag,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn queue_rock_smash_from_menu(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeInteractionScriptDispatch> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::QueueRockSmashFromMenu(
                RuntimeFieldPartyCommand { party_index },
            ))?;
        let RuntimeMutationResult::RockSmashFromMenuQueued(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-RockSmashFromMenu result");
        };
        Ok(RuntimeInteractionScriptDispatch {
            next_script: outcome.next_script,
            last_talked_object: Some(outcome.object_identifier),
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn current_encounter_surface(&self) -> Option<EncounterSurface> {
        self.current_encounter_surface_checked()
            .expect("current encounter surface requires verified map metadata and collision")
    }

    pub fn current_encounter_surface_checked(&self) -> Result<Option<EncounterSurface>> {
        let environment = &self
            .runtime
            .data()
            .runtime_map_metadata_for_name(&self.session.overworld.map.name)?
            .environment;
        let land_encounters_on_any_land =
            environment.eq_ignore_ascii_case("cave") || environment.eq_ignore_ascii_case("dungeon");
        self.session
            .overworld
            .current_encounter_surface_checked_with_land_encounters(land_encounters_on_any_land)
            .map_err(|error| anyhow::anyhow!("current encounter surface: {error}"))
    }

    pub fn use_sweet_scent_field_move(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeSweetScentFieldMoveUse> {
        let recorded = self
            .session
            .stage_sweet_scent_field_move(&self.runtime, party_index)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::SweetScentFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-SWEET_SCENT result");
        };
        Ok(RuntimeSweetScentFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            wild_encounter: outcome.wild_encounter,
            wild_battle: outcome.wild_battle,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn start_scripted_wild_battle(
        &mut self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<StaticWildBattleStart> {
        let recorded = self.session.stage_scripted_wild_battle_start(
            &self.runtime,
            RuntimeScriptCommandRef::new(map_name, source_script, startbattle_command_index),
        )?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ScriptedWildBattleStarted(start) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-wild-battle-start result");
        };
        Ok(start)
    }

    pub fn start_scripted_trainer_battle(
        &mut self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<TrainerBattleStartStatus> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::StartScriptedTrainerBattle(RuntimeScriptCommandRef::new(
                map_name,
                source_script,
                startbattle_command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptedTrainerBattleStarted(start) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-trainer-battle-start result");
        };
        Ok(start)
    }

    pub fn complete_scripted_wild_battle(
        &mut self,
        origin: RuntimeStaticWildBattleOrigin,
    ) -> Result<RuntimeScriptedBattleCompletion> {
        let recorded = self
            .session
            .stage_scripted_wild_battle_completion(&self.runtime, origin)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ScriptedWildBattleCompleted = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-wild-battle-completion result");
        };
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle: true,
            trainer_prize_money: None,
            money_after: None,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn complete_scripted_trainer_battle(
        &mut self,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
        won: bool,
        can_lose: bool,
    ) -> Result<RuntimeScriptedBattleCompletion> {
        let recorded = self.session.stage_scripted_trainer_battle_completion(
            &self.runtime,
            map_name,
            source_script,
            startbattle_command_index,
            won,
            can_lose,
        )?;
        let completion_mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ScriptedTrainerBattleCompleted(completion_outcome) =
            completion_mutation.result
        else {
            anyhow::bail!(
                "runtime mutation returned non-scripted-trainer-battle-completion result"
            );
        };
        let continued_after_battle = completion_outcome.continued_after_battle;
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle,
            trainer_prize_money: Some(completion_outcome.prize_money),
            money_after: Some(completion_outcome.money_after),
            state_checksum: completion_mutation.state_checksum,
        })
    }

    pub fn dispatch_interaction_script(
        &mut self,
        interaction: &OverworldInteraction,
    ) -> Result<RuntimeInteractionScriptDispatch> {
        self.session
            .dispatch_interaction_script(&self.runtime, interaction)
    }

    pub fn dispatch_coord_event_script(
        &mut self,
        coord_event: &CoordEventTrigger,
    ) -> Result<RuntimeInteractionScriptDispatch> {
        self.session
            .dispatch_coord_event_script(&self.runtime, coord_event)
    }

    pub fn resolve_active_battle_command(
        &mut self,
        player_action: BattleAction,
        enemy_action: BattleAction,
        rng_seed_after: u32,
    ) -> Result<RuntimeBattleCommand> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ResolveActiveBattleCommand(RuntimeBattleTurnCommand {
                player_action,
                enemy_action,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::ActiveBattleCommandResolved(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-command result");
        };
        Ok(RuntimeBattleCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn preview_active_battle_command_rng_seed_after(
        &self,
        player_action: BattleAction,
        enemy_action: BattleAction,
    ) -> Result<u32> {
        let mut preview = self.session.state.clone();
        self.runtime
            .data()
            .resolve_active_battle_command(&mut preview, player_action, enemy_action)
            .context("preview active battle command rng boundary")?;
        Ok(preview.rng_seed)
    }

    pub fn resolve_active_battle_turn(
        &mut self,
        player_action: BattleAction,
        enemy_action: BattleAction,
        rng_seed_after: u32,
    ) -> Result<RuntimeBattleTurn> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ResolveActiveBattleTurn(RuntimeBattleTurnCommand {
                player_action,
                enemy_action,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::ActiveBattleTurnResolved(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-turn result");
        };
        Ok(RuntimeBattleTurn {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn preview_active_battle_turn_rng_seed_after(
        &self,
        player_action: BattleAction,
        enemy_action: BattleAction,
    ) -> Result<u32> {
        let mut preview = self.session.state.clone();
        self.runtime
            .data()
            .resolve_active_battle_turn(&mut preview, player_action, enemy_action)
            .context("preview active battle turn rng boundary")?;
        Ok(preview.rng_seed)
    }

    pub fn resolve_active_battle_enemy_action(
        &mut self,
        enemy_action: BattleAction,
        rng_seed_after: u32,
    ) -> Result<RuntimeBattleTurn> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ResolveActiveBattleEnemyAction(
                RuntimeBattleEnemyActionCommand {
                    enemy_action,
                    rng_seed_after,
                },
            ),
        )?;
        let RuntimeMutationResult::ActiveBattleEnemyActionResolved(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-enemy-battle-action result");
        };
        Ok(RuntimeBattleTurn {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn attempt_escape_active_wild_battle(&mut self) -> Result<RuntimeBattleEscape> {
        let rng_seed_after = preview_active_wild_battle_escape_rng_seed_after(
            self.runtime.data(),
            &self.session.state,
        )?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::AttemptEscapeActiveWildBattle(RuntimeBattleEscapeCommand {
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::ActiveWildBattleEscapeAttempted(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-battle-escape result");
        };
        Ok(RuntimeBattleEscape {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn throw_ball_at_active_battle(&mut self, ball_id: &str) -> Result<RuntimeCaptureAttempt> {
        let rng_seed_after =
            preview_throw_ball_rng_seed_after(self.runtime.data(), &self.session.state, ball_id)?;
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ThrowBallAtActiveBattle(RuntimeBattleItemCommand {
                item_id: ball_id.to_string(),
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::BallThrown(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-capture result");
        };
        Ok(RuntimeCaptureAttempt {
            outcome: Some(outcome),
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn complete_active_wild_capture(
        &mut self,
        outcome: &CaptureOutcome,
        nickname: Option<String>,
    ) -> Result<RuntimeCaptureCompletion> {
        let recorded =
            self.session
                .stage_active_wild_capture_completion(&self.runtime, outcome, nickname)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ActiveWildCaptureCompleted(CaptureCompletion {
            stored,
            contest_pokemon,
        }) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-capture-completion result");
        };
        Ok(RuntimeCaptureCompletion {
            stored,
            contest_pokemon,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn resolve_bug_contest_caught_mon(
        &mut self,
        keep_new: bool,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ResolveBugContestCaughtMon { keep_new },
        )?;
        let RuntimeMutationResult::SpecialRoutineApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Bug Contest decision result");
        };
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_to_escape_active_wild_battle(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeBattleEscapeItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemToEscapeActiveWildBattle(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::ActiveWildBattleEscapeItemUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-escape-item result");
        };
        Ok(RuntimeBattleEscapeItemUse {
            item_use: outcome.item_use,
            battle_escape_mode: outcome.battle_escape_mode,
            escaped: outcome.escaped,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_guard_spec_in_active_battle(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeBattleStateItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagGuardSpecInActiveBattle(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::ActiveBattleGuardSpecUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-state-item result");
        };
        Ok(RuntimeBattleStateItemUse {
            item_use: outcome.item_use,
            stat_drop_guard_turns_before: outcome.stat_drop_guard_turns_before,
            stat_drop_guard_turns_after: outcome.stat_drop_guard_turns_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn switch_active_battle_party(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeBattlePartySwitch> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::SwitchActiveBattleParty(
                RuntimePartySlotCommand { party_index },
            ))?;
        let RuntimeMutationResult::ActiveBattlePartySwitched(party_index) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-active-battle-party-switch result");
        };
        Ok(RuntimeBattlePartySwitch {
            party_index,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_party_pokemon(
        &mut self,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimePartyItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemOnPartyPokemon(RuntimePartyItemCommand {
                item_id: item_id.to_string(),
                party_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonItemUsed(item_use, item_effect) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-party-item result");
        };
        Ok(RuntimePartyItemUse {
            item_use,
            item_effect,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_whole_party(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeWholePartyItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemOnWholeParty(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::WholePartyItemUsed(item_use, item_effect) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-whole-party-item result");
        };
        Ok(RuntimeWholePartyItemUse {
            item_use,
            item_effect,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_party_move(
        &mut self,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<RuntimePartyItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemOnPartyMove(RuntimePartyMoveItemCommand {
                item_id: item_id.to_string(),
                party_index,
                move_slot,
            }),
        )?;
        let RuntimeMutationResult::PartyMoveItemUsed(item_use, item_effect) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-party-move-item result");
        };
        Ok(RuntimePartyItemUse {
            item_use,
            item_effect,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_active_battle_pokemon(
        &mut self,
        item_id: &str,
    ) -> Result<RuntimeBattleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemOnActiveBattlePokemon(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::ActiveBattlePokemonItemUsed(item_use, battle_item) =
            mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-active-battle-item result");
        };
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_tmhm_on_party_pokemon(
        &mut self,
        item_id: &str,
        party_index: usize,
        replace_slot: Option<usize>,
    ) -> Result<RuntimeTmHmItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagTmHmOnPartyPokemon(RuntimeTmHmCommand {
                item_id: item_id.to_string(),
                party_index,
                replace_slot,
            }),
        )?;
        let RuntimeMutationResult::TmHmItemUsed(item_use, learned_move) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-TM/HM result");
        };
        Ok(RuntimeTmHmItemUse {
            item_use,
            learned_move,
            state_checksum: mutation.state_checksum,
        })
    }

    pub(crate) fn preview_tmhm_on_party_pokemon(
        &self,
        item_id: &str,
        party_index: usize,
        replace_slot: Option<usize>,
    ) -> Result<TmHmLearnOutcome> {
        let mut pokemon = self
            .session
            .state
            .storage
            .party
            .pokemon
            .get(party_index)
            .and_then(|pokemon| pokemon.clone())
            .with_context(|| format!("party index {party_index} has no Pokemon"))?;
        self.runtime
            .data
            .teach_tmhm_move(&mut pokemon, item_id, replace_slot, false)
    }

    pub(crate) fn preview_party_item_on_pokemon(
        &self,
        item_id: &str,
        party_index: usize,
    ) -> Result<BattleItemOutcome> {
        let mut pokemon = self
            .session
            .state
            .storage
            .party
            .pokemon
            .get(party_index)
            .and_then(|pokemon| pokemon.clone())
            .with_context(|| format!("party index {party_index} has no Pokemon"))?;
        self.runtime.data.apply_party_pokemon_item_effect(
            &mut pokemon,
            item_id,
            self.session.state.time.time_of_day,
            false,
        )
    }

    pub fn use_bag_item_on_battle_party_pokemon(
        &mut self,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimeBattleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemOnBattlePartyPokemon(RuntimePartyItemCommand {
                item_id: item_id.to_string(),
                party_index,
            }),
        )?;
        let RuntimeMutationResult::BattlePartyPokemonItemUsed(item_use, battle_item) =
            mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-battle-party-item result");
        };
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_battle_party_move(
        &mut self,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<RuntimeBattleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::UseBagItemOnBattlePartyMove(RuntimePartyMoveItemCommand {
                item_id: item_id.to_string(),
                party_index,
                move_slot,
            }),
        )?;
        let RuntimeMutationResult::BattlePartyMoveItemUsed(item_use, battle_item) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-battle-party-move-item result");
        };
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn advance_active_trainer_battle(&mut self) -> Result<RuntimeTrainerBattleAdvance> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::AdvanceActiveTrainerBattle)?;
        let RuntimeMutationResult::ActiveTrainerBattleAdvanced(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-trainer-battle-advance result");
        };
        Ok(RuntimeTrainerBattleAdvance {
            next_enemy: outcome.next_enemy,
            trainer_defeated: outcome.trainer_defeated,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn claim_active_trainer_battle_rewards(&mut self) -> Result<RuntimeBattleRewards> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ClaimActiveTrainerBattleRewardsNow,
        )?;
        let RuntimeMutationResult::ActiveTrainerBattleRewardsClaimed(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-trainer-rewards result");
        };
        Ok(RuntimeBattleRewards {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn claim_active_wild_battle_rewards(&mut self) -> Result<RuntimeBattleRewards> {
        let recorded = self.session.stage_wild_battle_rewards(&self.runtime)?;
        let mutation = self.apply_recorded_runtime_mutation(recorded)?;
        let RuntimeMutationResult::ActiveWildBattleRewardsClaimed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-wild-rewards result");
        };
        Ok(RuntimeBattleRewards {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn switch_current_pc_box(&mut self, box_index: usize) -> Result<RuntimeStorageBoxSwitch> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SwitchCurrentPcBox(RuntimePcBoxCommand { box_index }),
        )?;
        let RuntimeMutationResult::CurrentPcBoxSwitched(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-box-switch result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC box switch")?;
        Ok(RuntimeStorageBoxSwitch {
            box_index_before: outcome.box_index_before,
            box_index_after: outcome.box_index_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn deposit_party_pokemon_to_current_box(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeStorageDeposit> {
        // BillsPC.PartyToBox pauses wGameLogicPaused across its save boundary.
        // The Rust storage mutation is atomic, so bracket that boundary in
        // the deterministic command journal and always restore the prior
        // control byte, including on a rejected deposit.
        let paused_before = self.session.state.game_logic_paused;
        self.set_game_logic_paused(true)?;
        let deposit_result = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::DepositPartyPokemonToCurrentBox(RuntimePcDepositCommand {
                party_index,
            }),
        );
        let restore_result = self.set_game_logic_paused(paused_before);
        let mutation = match (deposit_result, restore_result) {
            (Ok(mutation), Ok(_)) => mutation,
            (Err(error), Ok(_)) => return Err(error),
            (Ok(_), Err(error)) => {
                return Err(error).context("resume game logic after PC party deposit");
            }
            (Err(deposit_error), Err(restore_error)) => {
                return Err(deposit_error).context(format!(
                    "also failed to resume game logic after PC party deposit: {restore_error:#}"
                ));
            }
        };
        let RuntimeMutationResult::PartyPokemonDeposited(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-deposit result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party deposit")?;
        Ok(RuntimeStorageDeposit {
            party_index: outcome.party_index,
            box_index: outcome.box_index,
            box_slot: outcome.box_slot,
            pokemon: outcome.pokemon,
            state_checksum: game_state_checksum(&self.session.state)
                .context("checksum runtime state after resuming PC party deposit")?,
        })
    }

    pub fn withdraw_current_box_pokemon_to_party(
        &mut self,
        box_slot: usize,
    ) -> Result<RuntimeStorageWithdraw> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::WithdrawCurrentBoxPokemonToParty(RuntimePcWithdrawCommand {
                box_slot,
            }),
        )?;
        let RuntimeMutationResult::PcPokemonWithdrawn(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-withdraw result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC withdraw")?;
        Ok(RuntimeStorageWithdraw {
            box_index: outcome.box_index,
            box_slot: outcome.box_slot,
            party_index: outcome.party_index,
            pokemon: outcome.pokemon,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn release_current_box_pokemon(
        &mut self,
        box_slot: usize,
    ) -> Result<RuntimeStorageRelease> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::ReleaseCurrentBoxPokemon(RuntimePcReleaseCommand { box_slot }),
        )?;
        let RuntimeMutationResult::PcPokemonReleased(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-release result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC release")?;
        Ok(RuntimeStorageRelease {
            box_index: outcome.box_index,
            box_slot: outcome.box_slot,
            pokemon: outcome.pokemon,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn move_pc_pokemon_without_mail(
        &mut self,
        source_box: usize,
        source_slot: usize,
        target_box: usize,
        target_slot: usize,
    ) -> Result<RuntimeStorageMove> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::MovePcPokemonWithoutMail(RuntimePcMoveCommand {
                source_box,
                source_slot,
                target_box,
                target_slot,
            }),
        )?;
        let RuntimeMutationResult::PcPokemonMoved(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-move result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC move")?;
        Ok(RuntimeStorageMove {
            source_box: outcome.source_box,
            source_slot: outcome.source_slot,
            target_box: outcome.target_box,
            target_slot: outcome.target_slot,
            swapped: outcome.swapped,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn deposit_bag_item_to_pc(
        &mut self,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimePcItemTransfer> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::DepositBagItemToPc(RuntimePcItemCommand {
                item_id: item_id.to_string(),
                quantity,
            }),
        )?;
        let RuntimeMutationResult::BagItemDepositedToPc(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-item-deposit result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC item deposit")?;
        Ok(RuntimePcItemTransfer {
            item_id: outcome.item_id,
            quantity: outcome.quantity,
            bag_quantity_after: outcome.bag_quantity_after,
            pc_quantity_after: outcome.pc_quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn withdraw_pc_item_to_bag(
        &mut self,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimePcItemTransfer> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::WithdrawPcItemToBag(RuntimePcItemCommand {
                item_id: item_id.to_string(),
                quantity,
            }),
        )?;
        let RuntimeMutationResult::PcItemWithdrawnToBag(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-item-withdraw result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC item withdraw")?;
        Ok(RuntimePcItemTransfer {
            item_id: outcome.item_id,
            quantity: outcome.quantity,
            bag_quantity_after: outcome.bag_quantity_after,
            pc_quantity_after: outcome.pc_quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn toss_pc_item(&mut self, item_id: &str, quantity: u16) -> Result<RuntimePcItemTransfer> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::TossPcItem(
            RuntimePcItemCommand {
                item_id: item_id.to_string(),
                quantity,
            },
        ))?;
        let RuntimeMutationResult::PcItemTossed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-PC-item-toss result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after PC item toss")?;
        Ok(RuntimePcItemTransfer {
            item_id: outcome.item_id,
            quantity: outcome.quantity,
            bag_quantity_after: outcome.bag_quantity_after,
            pc_quantity_after: outcome.pc_quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn give_bag_item_to_party_pokemon(
        &mut self,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimeHeldItemTransfer> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::GiveBagItemToPartyPokemon(RuntimeHeldItemCommand {
                item_id: item_id.to_string(),
                party_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonHeldItemGiven(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-held-item-give result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after held item give")?;
        Ok(RuntimeHeldItemTransfer {
            party_index: outcome.party_index,
            item_id: outcome.item_id,
            bag_quantity_after: outcome.bag_quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_held_item_from_party_pokemon(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeHeldItemTransfer> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::TakeHeldItemFromPartyPokemon(RuntimePartySlotCommand {
                party_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonHeldItemTaken(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-held-item-take result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after held item take")?;
        Ok(RuntimeHeldItemTransfer {
            party_index: outcome.party_index,
            item_id: outcome.item_id,
            bag_quantity_after: outcome.bag_quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn send_party_mail_to_mailbox(
        &mut self,
        party_index: usize,
    ) -> Result<RuntimeMailTransfer> {
        self.apply_mail_mutation(RuntimeMutationCommand::SendPartyMailToMailbox(
            RuntimePartySlotCommand { party_index },
        ))
    }

    pub fn discard_party_mail_to_bag(&mut self, party_index: usize) -> Result<RuntimeMailTransfer> {
        self.apply_mail_mutation(RuntimeMutationCommand::DiscardPartyMailToBag(
            RuntimePartySlotCommand { party_index },
        ))
    }

    pub fn delete_mailbox_mail(&mut self, mailbox_index: usize) -> Result<RuntimeMailTransfer> {
        self.apply_mail_mutation(RuntimeMutationCommand::DeleteMailboxMail(
            RuntimeMailboxSlotCommand { mailbox_index },
        ))
    }

    pub fn move_mailbox_mail_to_bag(
        &mut self,
        mailbox_index: usize,
    ) -> Result<RuntimeMailTransfer> {
        self.apply_mail_mutation(RuntimeMutationCommand::MoveMailboxMailToBag(
            RuntimeMailboxSlotCommand { mailbox_index },
        ))
    }

    pub fn attach_mailbox_mail_to_party(
        &mut self,
        mailbox_index: usize,
        party_index: usize,
    ) -> Result<RuntimeMailTransfer> {
        self.apply_mail_mutation(RuntimeMutationCommand::AttachMailboxMailToParty(
            RuntimeMailboxPartyCommand {
                mailbox_index,
                party_index,
            },
        ))
    }

    fn apply_mail_mutation(
        &mut self,
        command: RuntimeMutationCommand,
    ) -> Result<RuntimeMailTransfer> {
        let mutation = self.apply_runtime_mutation_command(command)?;
        let outcome = match mutation.result {
            RuntimeMutationResult::PartyMailSentToMailbox(outcome)
            | RuntimeMutationResult::PartyMailDiscardedToBag(outcome)
            | RuntimeMutationResult::MailboxMailDeleted(outcome) => outcome,
            RuntimeMutationResult::MailboxMailMovedToBag(outcome)
            | RuntimeMutationResult::MailboxMailAttachedToParty(outcome) => outcome,
            _ => anyhow::bail!("runtime mutation returned non-Mail-transfer result"),
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Mail transfer")?;
        Ok(RuntimeMailTransfer {
            party_index: outcome.party_index,
            mailbox_index: outcome.mailbox_index,
            item_id: outcome.item_id,
            mail: outcome.mail,
            mailbox_count_after: outcome.mailbox_count_after,
            bag_quantity_after: outcome.bag_quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn award_badge(
        &mut self,
        region: RuntimeBadgeRegion,
        index: usize,
    ) -> Result<RuntimeBadgeAward> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::AwardBadge(
            RuntimeBadgeCommand { region, index },
        ))?;
        let RuntimeMutationResult::BadgeAwarded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-badge-award result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after badge award")?;
        Ok(RuntimeBadgeAward {
            region: outcome.region,
            index: outcome.index,
            already_awarded: outcome.already_awarded,
            awarded_count_after: outcome.awarded_count_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn record_pokedex_seen(&mut self, species_id: &str) -> Result<RuntimePokedexRecord> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::RecordPokedexSeen(RuntimePokedexCommand {
                species_id: species_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::PokedexSeenRecorded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Pokedex-seen result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Pokedex seen record")?;
        Ok(RuntimePokedexRecord {
            species_id: outcome.species_id,
            already_seen: outcome.already_seen,
            already_caught: outcome.already_caught,
            seen_count_after: outcome.seen_count_after,
            caught_count_after: outcome.caught_count_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn record_pokedex_caught(&mut self, species_id: &str) -> Result<RuntimePokedexRecord> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::RecordPokedexCaught(RuntimePokedexCommand {
                species_id: species_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::PokedexCaughtRecorded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Pokedex-caught result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Pokedex caught record")?;
        Ok(RuntimePokedexRecord {
            species_id: outcome.species_id,
            already_seen: outcome.already_seen,
            already_caught: outcome.already_caught,
            seen_count_after: outcome.seen_count_after,
            caught_count_after: outcome.caught_count_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn add_currency(
        &mut self,
        account: RuntimeCurrencyAccount,
        amount: u32,
    ) -> Result<RuntimeCurrencyMutation> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::AddCurrency(
            RuntimeCurrencyDeltaCommand { account, amount },
        ))?;
        let RuntimeMutationResult::CurrencyAdded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-currency-add result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after currency add")?;
        Ok(RuntimeCurrencyMutation {
            account: outcome.account,
            amount: outcome.amount,
            value_before: outcome.value_before,
            value_after: outcome.value_after,
            cap: outcome.cap,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_currency(
        &mut self,
        account: RuntimeCurrencyAccount,
        amount: u32,
    ) -> Result<RuntimeCurrencyMutation> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::TakeCurrency(RuntimeCurrencyDeltaCommand { account, amount }),
        )?;
        let RuntimeMutationResult::CurrencyTaken(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-currency-take result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after currency take")?;
        Ok(RuntimeCurrencyMutation {
            account: outcome.account,
            amount: outcome.amount,
            value_before: outcome.value_before,
            value_after: outcome.value_after,
            cap: outcome.cap,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn add_bag_item(&mut self, item_id: &str, quantity: u16) -> Result<RuntimeBagItemMutation> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::AddBagItem(
            RuntimeBagItemDeltaCommand {
                item_id: item_id.to_string(),
                quantity,
            },
        ))?;
        let RuntimeMutationResult::BagItemAdded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-bag-item-add result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after bag item add")?;
        Ok(RuntimeBagItemMutation {
            item_id: outcome.item_id,
            quantity: outcome.quantity,
            added: outcome.added,
            quantity_before: outcome.quantity_before,
            quantity_after: outcome.quantity_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn remove_bag_item(
        &mut self,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeBagItemMutation> {
        let item = self
            .runtime
            .data
            .items
            .get(item_id)
            .cloned()
            .with_context(|| format!("unknown bag item {item_id}"))?;
        let quantity_before = self.session.state.bag.quantity(&item);
        self.session
            .state
            .bag
            .remove_item(&item, quantity)
            .map_err(anyhow::Error::msg)?;
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after bag item remove")?;
        let item_id = item.script_name.clone();
        Ok(RuntimeBagItemMutation {
            item_id,
            quantity,
            added: false,
            quantity_before,
            quantity_after: self.session.state.bag.quantity(&item),
            state_checksum: game_state_checksum(&self.session.state)?,
        })
    }

    pub fn record_link_battle_result(
        &mut self,
        result: RuntimeLinkBattleResult,
    ) -> Result<RuntimeLinkBattleRecord> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::RecordLinkBattleResult(
                RuntimeLinkBattleRecordCommand { result },
            ))?;
        let RuntimeMutationResult::LinkBattleResultRecorded(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-link-battle-record result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after link battle record")?;
        Ok(RuntimeLinkBattleRecord {
            result: outcome.result,
            wins_after: outcome.wins_after,
            losses_after: outcome.losses_after,
            draws_after: outcome.draws_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_cable_club_request(
        &mut self,
        request: RuntimeCableClubRequest,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::SetCableClubRequest(request))?;
        let RuntimeMutationResult::CableClubRequestSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-cable-club-request result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Cable Club request")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn wait_for_linked_friend_special(
        &mut self,
        ready: bool,
        serial_connection_status: LinkSerialConnectionStatus,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::WaitForLinkedFriendSpecial(RuntimeLinkFriendReadyCommand {
                ready,
                serial_connection_status,
            }),
        )?;
        let RuntimeMutationResult::LinkedFriendWaitedFor(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-linked-friend-wait result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after linked friend wait")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_link_timeout_receptionist_special(
        &mut self,
        timeout: bool,
        other_player_link_mode: u8,
        serial_connection_status: LinkSerialConnectionStatus,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckLinkTimeoutReceptionistSpecial(
                RuntimeLinkTimeoutCommand {
                    timeout,
                    other_player_link_mode,
                    serial_connection_status,
                },
            ),
        )?;
        let RuntimeMutationResult::LinkTimeoutReceptionistChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-link-timeout-result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after link timeout receptionist check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_both_selected_same_room_special(
        &mut self,
        other_player_room: u8,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckBothSelectedSameRoomSpecial(
                RuntimeLinkRoomSelectionCommand { other_player_room },
            ),
        )?;
        let RuntimeMutationResult::BothSelectedSameRoomChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-same-room-result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after same-room check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn close_link_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::CloseLinkSpecial)?;
        let RuntimeMutationResult::LinkClosed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-link-close result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after link close")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn wait_for_other_player_to_exit_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::WaitForOtherPlayerToExitSpecial,
        )?;
        let RuntimeMutationResult::OtherPlayerExitWaitedFor(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-other-player-exit result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after other player exit wait")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn failed_link_to_past_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::FailedLinkToPastSpecial)?;
        let RuntimeMutationResult::LinkToPastFailed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-failed-link-to-past result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after failed link to past")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_link_room_special(
        &mut self,
        room: RuntimeLinkRoomSpecial,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::OpenLinkRoomSpecial(room))?;
        let RuntimeMutationResult::LinkRoomOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-link-room result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after link room")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_time_capsule_compatibility_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CheckTimeCapsuleCompatibilitySpecial,
        )?;
        let RuntimeMutationResult::TimeCapsuleCompatibilityChecked(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-time-capsule-compatibility result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Time Capsule compatibility check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn try_quick_save_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::TryQuickSaveSpecial)?;
        let RuntimeMutationResult::QuickSaveTried(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-quick-save result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after quick save special")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn ask_mobile_or_cable_special(&mut self) -> Result<RuntimeSpecialRoutineUse> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::AskMobileOrCableSpecial)?;
        let RuntimeMutationResult::MobileOrCableAsked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-mobile-or-cable result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after mobile/cable prompt")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn cable_club_check_which_chris_special(
        &mut self,
        gender: String,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::CableClubCheckWhichChrisSpecial(
                RuntimeCableClubGenderCommand { gender },
            ),
        )?;
        let RuntimeMutationResult::CableClubChrisChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Cable-Club-Chris result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after Cable Club Chris check")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_options(&mut self, options: Options) -> Result<RuntimeOptionsSet> {
        let mutation = self.apply_runtime_mutation_command(RuntimeMutationCommand::SetOptions(
            RuntimeOptionsCommand { options },
        ))?;
        let RuntimeMutationResult::OptionsSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-options-set result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after options set")?;
        Ok(RuntimeOptionsSet {
            options_before: outcome.options_before,
            options_after: outcome.options_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_trainer_identity(
        &mut self,
        player_name: impl Into<String>,
        player_id: u16,
    ) -> Result<RuntimeTrainerIdentitySet> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SetTrainerIdentity(RuntimeTrainerIdentityCommand {
                player_name: player_name.into(),
                player_id,
            }),
        )?;
        let RuntimeMutationResult::TrainerIdentitySet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-trainer-identity result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after trainer identity set")?;
        Ok(RuntimeTrainerIdentitySet {
            player_name_before: outcome.player_name_before,
            player_id_before: outcome.player_id_before,
            player_name_after: outcome.player_name_after,
            player_id_after: outcome.player_id_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_player_gender(&mut self, player_gender: u8) -> Result<RuntimePlayerGenderSet> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SetPlayerGender(RuntimePlayerGenderCommand { player_gender }),
        )?;
        let RuntimeMutationResult::PlayerGenderSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-player-gender result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after player gender set")?;
        Ok(RuntimePlayerGenderSet {
            player_gender_before: outcome.player_gender_before,
            player_gender_after: outcome.player_gender_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn rename_party_pokemon(
        &mut self,
        party_index: usize,
        nickname: impl Into<String>,
    ) -> Result<RuntimePartyNicknameSet> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::RenamePartyPokemon(RuntimePartyNicknameCommand {
                party_index,
                nickname: nickname.into(),
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonRenamed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-nickname result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party nickname set")?;
        Ok(RuntimePartyNicknameSet {
            party_index: outcome.party_index,
            species_id: outcome.species_id,
            nickname_before: outcome.nickname_before,
            nickname_after: outcome.nickname_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn set_party_pokemon_recovery_state(
        &mut self,
        party_index: usize,
        hp: u16,
        status: Option<String>,
        first_move_pp: Option<u8>,
    ) -> Result<RuntimePartyRecoveryStateSet> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SetPartyPokemonRecoveryState(
                RuntimePartyRecoverySetupCommand {
                    party_index,
                    hp,
                    status,
                    first_move_pp,
                },
            ),
        )?;
        let RuntimeMutationResult::PartyPokemonRecoveryStateSet(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-recovery-setup result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party recovery setup")?;
        Ok(RuntimePartyRecoveryStateSet {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn full_heal_party_pokemon(&mut self, party_index: usize) -> Result<RuntimePartyRecovery> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::FullHealPartyPokemon(RuntimePartySlotCommand { party_index }),
        )?;
        let RuntimeMutationResult::PartyPokemonFullHealed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-recovery result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party Pokemon recovery")?;
        Ok(runtime_party_recovery(outcome, mutation.state_checksum))
    }

    pub fn transfer_party_pokemon_hp(
        &mut self,
        source_party_index: usize,
        target_party_index: usize,
    ) -> Result<RuntimePartyHpTransfer> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::TransferPartyPokemonHp(RuntimePartyHpTransferCommand {
                source_party_index,
                target_party_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonHpTransferred(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-HP-transfer result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party HP transfer")?;
        Ok(RuntimePartyHpTransfer {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn full_heal_whole_party(&mut self) -> Result<Vec<RuntimePartyRecovery>> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::FullHealWholeParty)?;
        let RuntimeMutationResult::WholePartyFullHealed(outcomes) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-whole-party-recovery result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after whole-party recovery")?;
        Ok(outcomes
            .into_iter()
            .map(|outcome| runtime_party_recovery(outcome, mutation.state_checksum.clone()))
            .collect())
    }

    pub fn replace_pending_move_learn(
        &mut self,
        move_slot: usize,
    ) -> Result<RuntimePendingMoveLearnResolution> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::ReplacePendingMoveLearn(
                RuntimeMoveLearnReplacementCommand { move_slot },
            ))?;
        let RuntimeMutationResult::PendingMoveLearnReplaced(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-pending-move-learn-replacement result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after pending move learn replacement")?;
        Ok(RuntimePendingMoveLearnResolution {
            resolution: outcome.resolution,
            deferred_evolution: outcome.deferred_evolution,
        })
    }

    pub fn decline_pending_move_learn(&mut self) -> Result<RuntimePendingMoveLearnResolution> {
        let mutation =
            self.apply_runtime_mutation_command(RuntimeMutationCommand::DeclinePendingMoveLearn)?;
        let RuntimeMutationResult::PendingMoveLearnDeclined(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-pending-move-learn-decline result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after pending move learn decline")?;
        Ok(RuntimePendingMoveLearnResolution {
            resolution: outcome.resolution,
            deferred_evolution: outcome.deferred_evolution,
        })
    }

    pub fn resolve_blackout_to_last_spawn(&mut self) -> Result<RuntimeBlackoutRecovery> {
        let mutation = self
            .apply_runtime_mutation_command(RuntimeMutationCommand::ResolveBlackoutToLastSpawn)?;
        let RuntimeMutationResult::BlackoutResolved(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-blackout-recovery result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after blackout recovery")?;
        let state = self.session.state.clone();
        self.session = self
            .runtime
            .resume_overworld_session(&self.asset_root, state)
            .context("resume overworld after blackout recovery")?;
        self.last_frame = None;
        Ok(runtime_blackout_recovery(outcome, mutation.state_checksum))
    }

    pub fn swap_party_pokemon(
        &mut self,
        first_party_index: usize,
        second_party_index: usize,
    ) -> Result<RuntimePartySwap> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SwapPartyPokemon(RuntimePartySwapCommand {
                first_party_index,
                second_party_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonSwapped(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-swap result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party swap")?;
        Ok(RuntimePartySwap {
            first_party_index: outcome.first_party_index,
            second_party_index: outcome.second_party_index,
            first_species_after: outcome.first_species_after,
            second_species_after: outcome.second_species_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn swap_party_pokemon_moves(
        &mut self,
        party_index: usize,
        first_move_index: usize,
        second_move_index: usize,
    ) -> Result<RuntimePartyMoveSwap> {
        let mutation = self.apply_runtime_mutation_command(
            RuntimeMutationCommand::SwapPartyPokemonMoves(RuntimePartyMoveSwapCommand {
                party_index,
                first_move_index,
                second_move_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonMovesSwapped(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-party-move-swap result");
        };
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after party move swap")?;
        Ok(RuntimePartyMoveSwap {
            party_index: outcome.party_index,
            first_move_index: outcome.first_move_index,
            second_move_index: outcome.second_move_index,
            first_move_after: outcome.first_move_after,
            second_move_after: outcome.second_move_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let paused_before = self.session.state.game_logic_paused;
        self.set_game_logic_paused(true)?;
        let save_result = self
            .runtime
            .save_game(path, self.session.state.clone())
            .context("save runtime game shell state");
        let restore_result = self.set_game_logic_paused(paused_before);
        match (save_result, restore_result) {
            (Ok(()), Ok(_)) => Ok(()),
            (Err(error), Ok(_)) => Err(error),
            (Ok(()), Err(error)) => Err(error).context("resume game logic after save"),
            (Err(save_error), Err(restore_error)) => Err(save_error).context(format!(
                "also failed to resume game logic after save: {restore_error:#}"
            )),
        }
    }

    pub fn load(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let mut state = self.runtime.load_save(path)?;
        state.set_game_timer_counting(true);
        state.set_game_logic_paused(false);
        self.session = self
            .runtime
            .resume_overworld_session(&self.asset_root, state)
            .context("load runtime game shell state")?;
        self.last_frame = None;
        self.linked_menu_results.clear();
        self.clear_retained_runtime_commands();
        Ok(())
    }

    pub fn snapshot(&self) -> Result<RuntimeShellSnapshot> {
        self.snapshot_with_integrity(true)
    }

    /// Build the read-only shell view used by the real-time renderer.
    ///
    /// Save validation and deterministic whole-state checksums belong at
    /// persistence, replay, and network boundaries. Running both for every
    /// LCD update cloned and walked the complete game state twice and made a
    /// presentation-only text change capable of missing multiple frames.
    pub fn presentation_snapshot(&self) -> Result<RuntimeShellSnapshot> {
        self.snapshot_with_integrity(false)
    }

    fn snapshot_with_integrity(&self, integrity: bool) -> Result<RuntimeShellSnapshot> {
        let (state_checksum, visual_state_hash) = if integrity {
            self.runtime
                .validate_save_state_for_runtime_pack(&self.session.state)
                .context("validate runtime game shell state before snapshot")?;
            let state_checksum = game_state_checksum(&self.session.state)
                .context("checksum runtime game shell state")?;
            let mut visual_state = self.session.state.clone();
            visual_state.frame_counter = 0;
            let visual_state_hash = game_state_checksum(&visual_state)
                .context("checksum render-relevant runtime game state")?
                .hash();
            (state_checksum, visual_state_hash)
        } else {
            // Presentation keys are maintained by BevyRuntimeShell's explicit
            // revision and render-key fields. Keep only the authoritative
            // frame number for UI animation code; no state checksum is
            // computed on this path.
            (StateChecksum::new(self.session.state.frame_counter, 0), 0)
        };
        let menu = self.runtime.active_menu_snapshot(&self.session.state)?;
        let ui = self
            .runtime
            .ui_snapshot(&self.session.state, menu.clone())?;
        let catalogs = self.runtime.static_catalog_cache();
        Ok(RuntimeShellSnapshot {
            boot: self.runtime.boot_summary(),
            overworld: self.session.snapshot(),
            overworld_player_hidden: self.session.overworld.player_hidden,
            visible_objects: self
                .session
                .overworld
                .objects
                .iter()
                .filter(|object| self.session.overworld.is_object_visible(object))
                .cloned()
                .collect(),
            visible_object_slots: self
                .session
                .overworld
                .objects
                .iter()
                .enumerate()
                .filter(|(_, object)| self.session.overworld.is_object_visible(object))
                .map(|(index, _)| index)
                .collect(),
            visible_object_runtime_tiles: self.session.overworld.object_runtime_tiles.clone(),
            visible_object_facings: self.session.overworld.object_facings.clone(),
            state_checksum,
            visual_state_hash,
            phase: RuntimeShellPhase::from_state(&self.session.state),
            trainer: RuntimeTrainerSnapshot::from_state(&self.session.state),
            progression: RuntimeProgressionSnapshot::from_state(&self.session.state),
            roaming_pokemon: self.session.state.roaming_pokemon.clone(),
            day_care: self.session.state.day_care.clone(),
            bug_contest: self.session.state.bug_contest.clone(),
            magikarp_record: self.session.state.magikarp_record.clone(),
            buenas_password: self.session.state.buenas_password.clone(),
            mystery_gift: RuntimeMysteryGiftSnapshot {
                unlocked: self.session.state.mystery_gift_unlocked,
                stored_item: self.session.state.mystery_gift.stored_item.clone(),
                backup_item: self.session.state.mystery_gift.backup_item.clone(),
                trainer_house_flag: self.session.state.mystery_gift.trainer_house_flag,
            },
            link_session: RuntimeLinkSessionSnapshot::from_state(&self.session.state),
            battle_tower: self.session.state.battle_tower.clone(),
            mobile_link: self.session.state.mobile_link.clone(),
            audio: RuntimeShellAudioState::from_state(&self.session.state),
            audio_catalog: Arc::clone(&catalogs.audio),
            menu,
            ui,
            battle: RuntimeBattleSnapshot::from_state(&self.session.state)?,
            pending_move_learn: RuntimePendingMoveLearnSnapshot::from_state(&self.session.state),
            party: RuntimePartySnapshot::from_state(&self.session.state),
            storage: RuntimeStorageSnapshot::from_state(&self.session.state),
            mailbox: self.session.state.mailbox.clone(),
            bag: self.runtime.bag_snapshot(&self.session.state)?,
            items: Arc::clone(&catalogs.items),
            item_effect_plans: Arc::clone(&catalogs.item_effect_plans),
            moves: Arc::clone(&catalogs.moves),
            pokemon: Arc::clone(&catalogs.pokemon),
            trainers: Arc::clone(&catalogs.trainers),
            maps: self
                .runtime
                .map_catalog_snapshot(&self.session.overworld.map, &self.session.state),
            spawn_points: Arc::clone(&catalogs.spawn_points),
            tilesets: Arc::clone(&catalogs.tilesets),
            encounters: Arc::clone(&catalogs.encounters),
            battle_rules: Arc::clone(&catalogs.battle_rules),
            world_rules: Arc::clone(&catalogs.world_rules),
            presentation: Arc::clone(&catalogs.presentation),
            special: Arc::clone(&catalogs.special),
            story: Arc::clone(&catalogs.story),
            playability: Arc::clone(&catalogs.playability),
            script_events: RuntimeScriptEventsSnapshot::from_state(&self.session.state),
            pending_shop: self.session.state.script_runtime.pending_shop.clone(),
            linked_menu_results: self.linked_menu_results.clone(),
        })
    }

    pub fn text_snapshot(&self, label: &str) -> Result<RuntimeTextSnapshot> {
        self.runtime
            .text_snapshot_for_label(&self.session.state, label)
            .with_context(|| format!("resolve runtime game shell text '{label}'"))
    }

    pub fn state_checksum(&self) -> Result<StateChecksum> {
        game_state_checksum(&self.session.state).context("checksum runtime game shell state")
    }

    pub fn set_script_flag_for_smoke(&mut self, flag_id: &str) -> Result<StateChecksum> {
        if is_engine_flag_name(flag_id) {
            self.runtime.require_engine_flag(flag_id)?;
        } else {
            self.runtime.require_event_flag(flag_id)?;
        }
        self.session
            .state
            .flags
            .set_script_flag(flag_id, true)
            .with_context(|| format!("set smoke script flag {flag_id}"))?;
        self.state_checksum()
    }

    pub fn set_blue_card_balance_for_smoke(&mut self, balance: u8) -> Result<StateChecksum> {
        self.session.state.blue_card_balance = balance;
        self.runtime
            .validate_save_state_for_runtime_pack(&self.session.state)
            .context("validate runtime state after smoke Blue Card balance seed")?;
        self.state_checksum()
    }

    pub fn current_map_name(&self) -> &str {
        &self.session.overworld.map.name
    }

    pub fn script_events_snapshot(&self) -> RuntimeScriptEventsSnapshot {
        RuntimeScriptEventsSnapshot::from_state(&self.session.state)
    }

    /// Cheap predicate for the host frame loop.  Unlike `snapshot()` this
    /// does not clone the semantic state or calculate a checksum, so an idle
    /// overworld frame can stay on the fast path.
    pub fn has_pending_script_work(&self) -> bool {
        self.pending_script_work_reason().is_some()
    }

    /// Identify the first authoritative script condition that still owns a
    /// frame. This is intentionally allocation-free so input diagnostics can
    /// report a precise capture reason without cloning a runtime snapshot.
    pub fn pending_script_work_reason(&self) -> Option<&'static str> {
        let script = &self.session.state.script_runtime;
        Some(if script.pending_text_label.is_some() {
            "pending_text_label"
        } else if script.pending_text_wait.is_some() {
            "pending_text_wait"
        } else if script.pending_yes_no.is_some() {
            "pending_yes_no"
        } else if script.pending_shop.is_some() {
            "pending_shop"
        } else if script.active_pokemon_picture.is_some() {
            "active_pokemon_picture"
        } else if script.window_open {
            "window_open"
        } else if script.text_window_open {
            "text_window_open"
        } else if script.pending_map_load.is_some() {
            "pending_map_load"
        } else if script.pending_map_refresh.is_some() {
            "pending_map_refresh"
        } else if script.pending_music_fade.is_some() {
            "pending_music_fade"
        } else if script.pending_screen_fade.is_some() {
            "pending_screen_fade"
        } else if !script.pending_delays.is_empty() {
            "pending_delays"
        } else if !script.pending_earthquakes.is_empty() {
            "pending_earthquakes"
        } else if !script.pending_emotes.is_empty() {
            "pending_emotes"
        } else if script.pending_script_warp.is_some() {
            "pending_script_warp"
        } else if !script.command_queue.is_empty() {
            "command_queue"
        } else if script.next_script.is_some() {
            "next_script"
        } else if !script.deferred_scripts.is_empty() {
            "deferred_scripts"
        } else if script.script_ended.is_some() {
            "script_ended"
        } else if !script.audio_events.is_empty() {
            "audio_events"
        } else if !script.graphics_events.is_empty() {
            "graphics_events"
        } else if !script.money_events.is_empty() {
            "money_events"
        } else if !script.map_events.is_empty() {
            "map_events"
        }
        // Text events are retained execution history, not pending work.
        else if !script.control_events.is_empty() {
            "control_events"
        } else if !script.shop_events.is_empty() {
            "shop_events"
        } else if !script.item_use_events.is_empty() {
            "item_use_events"
        } else if script.active_menu.is_some() {
            "active_menu"
        } else if script.waiting_for_sound_effect {
            "waiting_for_sound_effect"
        } else if script.player_input_locked {
            "player_input_locked"
        } else if script.all_input_locked {
            "all_input_locked"
        } else if script.script_stop_requested {
            "script_stop_requested"
        } else if script.warp_check_requested {
            "warp_check_requested"
        } else {
            return None;
        })
    }

    /// Avoid entering the transactional audio-queue mutation path when the
    /// game has not emitted a sound event.
    pub fn has_pending_audio_events(&self) -> bool {
        !self.session.state.script_runtime.audio_events.is_empty()
    }

    pub fn current_music_id(&self) -> Option<&str> {
        self.session.state.script_runtime.current_music.as_deref()
    }

    pub fn has_pending_music_fade(&self) -> bool {
        self.session
            .state
            .script_runtime
            .pending_music_fade
            .is_some()
    }

    pub fn has_active_bug_contest_timer(&self) -> bool {
        self.session.state.bug_contest.timer_active
    }

    pub fn audio_state(&self) -> RuntimeShellAudioState {
        RuntimeShellAudioState::from_state(&self.session.state)
    }

    pub fn has_active_battle(&self) -> bool {
        self.session.state.battle_active_party_index.is_some()
    }

    pub fn runtime(&self) -> &CrystalRuntime {
        &self.runtime
    }

    pub fn facing_tile_collision_permission(&self) -> anyhow::Result<Option<u8>> {
        let target = crystal_core::world::movement::checked_move_by_stride(
            self.session.overworld.player.tile,
            self.session.overworld.player.facing,
            crystal_core::world::movement::StepOptions::default().stride_tiles,
        )
        .with_context(|| {
            format!(
                "facing tile overflows runtime coordinates from ({}, {}) facing {:?}",
                self.session.overworld.player.tile.x,
                self.session.overworld.player.tile.y,
                self.session.overworld.player.facing
            )
        })?;
        Ok(crystal_core::world::collision::sample_collision(
            &self.session.overworld.map,
            &self.session.overworld.tileset,
            target,
        )
        .map(|sample| sample.permission))
    }

    pub fn current_overworld_interaction(&self) -> Option<OverworldInteraction> {
        self.current_overworld_interaction_checked()
            .expect("current overworld interaction coordinates must be valid")
    }

    pub fn current_overworld_interaction_checked(&self) -> Result<Option<OverworldInteraction>> {
        let candidate = self
            .session
            .overworld
            .check_interaction_checked(
                crystal_core::world::movement::StepOptions::default().stride_tiles,
            )
            .with_context(|| {
                format!(
                    "check current overworld interaction on {}",
                    self.session.overworld.map.name
                )
            })?;
        candidate
            .as_ref()
            .map(|candidate| {
                self.runtime
                    .data
                    .resolve_overworld_interaction(&self.session.state, candidate)
            })
            .transpose()
            .map(Option::flatten)
    }

    pub fn current_scene_script(&self) -> Result<Option<RuntimeCurrentSceneScript>> {
        let map_name = self.session.snapshot().map_name;
        let Some(module) = self.runtime.data.maps.get(&map_name) else {
            anyhow::bail!("current map {map_name} missing from runtime map catalog");
        };
        if module.scenes.scenes.is_empty() {
            return Ok(None);
        }
        let scene_memory = &self.session.state.scenes;
        let scene_id = scene_memory.map_scenes.get(&map_name);
        let Some(scene_id) = scene_id else {
            return Ok(None);
        };
        let scene = module
            .scenes
            .scenes
            .iter()
            .find(|scene| scene.scene_id == *scene_id)
            .with_context(|| {
                format!("current scene {map_name}:{scene_id} missing from compiled map")
            })?;
        Ok(Some(RuntimeCurrentSceneScript {
            map_name,
            scene_id: scene.scene_id.clone(),
            script_name: scene.script_name.clone(),
        }))
    }

    pub fn session(&self) -> &RuntimeOverworldSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut RuntimeOverworldSession {
        &mut self.session
    }

    pub fn last_frame(&self) -> Option<&RuntimeOverworldFrame> {
        self.last_frame.as_ref()
    }

    pub fn asset_root(&self) -> &AssetRoot {
        &self.asset_root
    }

    fn record_runtime_mutation_outcome(&mut self, outcome: &RuntimeMutationOutcome) {
        if let RuntimeMutationResult::OverworldInputApplied(frame) = &outcome.result {
            self.last_frame = Some(RuntimeOverworldFrame::from_input_frame(
                frame.clone(),
                outcome.state_checksum.clone(),
            ));
        }
    }
}

fn compiled_script_boundary_stops_run(
    command: &str,
    boundary: &Option<RuntimeCompiledScriptBoundary>,
) -> bool {
    boundary.is_some()
        && !matches!(boundary, Some(RuntimeCompiledScriptBoundary::TextLabel(_)))
        // ASM Script_loadmenu prepares the header and returns. The actual
        // input boundary is Script_verticalmenu/Script__2dmenu.
        && !(command == "loadmenu"
            && matches!(boundary, Some(RuntimeCompiledScriptBoundary::ActiveMenu(_))))
}

impl RuntimeShellAudioState {
    fn from_state(state: &GameState) -> Self {
        Self {
            current_music: state.script_runtime.current_music.clone(),
            queued_events: state.script_runtime.audio_events.clone(),
        }
    }
}

impl RuntimeTrainerSnapshot {
    fn from_state(state: &GameState) -> Self {
        Self {
            player_name: state.player_name.clone(),
            player_id: state.player_id,
            player_gender: state.player_gender,
            player_palette_id: state.player_palette_id,
            money: state.money,
            moms_money: state.moms_money,
            coins: state.coins,
            blue_card_balance: u16::from(state.blue_card_balance),
            current_pc_box: state.current_pc_box,
            options: state.options.clone(),
        }
    }
}

impl RuntimeProgressionSnapshot {
    fn from_state(state: &GameState) -> Self {
        Self {
            rng_seed: state.rng_seed,
            badges: state.badges.clone(),
            pokedex_seen: state.pokedex.seen_count(),
            pokedex_owned: state.pokedex.caught_count(),
            pokedex_seen_species: state.pokedex.seen_species.clone(),
            pokedex_caught_species: state.pokedex.caught_species.clone(),
            link_wins: state.link_battle_stats.wins,
            link_losses: state.link_battle_stats.losses,
            link_draws: state.link_battle_stats.draws,
            pending_special_battle_type: state.pending_special_battle_type.clone(),
            repel_steps_remaining: state.repel_steps_remaining,
            active_repel_item: state.active_repel_item.clone(),
            registered_key_item: state.registered_key_item.clone(),
            last_spawn_identifier: state.last_spawn_identifier,
            hall_of_fame: state.hall_of_fame.clone(),
            time: state.time.clone(),
            active_event_flags: state.flags.active_event_flags().cloned().collect(),
            active_engine_flags: state
                .flags
                .engine_flags
                .iter()
                .filter_map(|(flag, set)| set.then(|| flag.clone()))
                .collect(),
        }
    }
}

impl RuntimeLinkSessionSnapshot {
    fn from_state(state: &GameState) -> Self {
        let link = &state.link_session;
        Self {
            link_mode: link.link_mode,
            player_link_action: link.player_link_action,
            chosen_cable_club_room: link.chosen_cable_club_room,
            other_player_link_mode: link.other_player_link_mode,
            friend_ready: link.friend_ready,
            last_result: link.last_result,
            failed_link_to_past: link.failed_link_to_past,
            quick_save_requested: link.quick_save_requested,
            active_room: link.active_room.clone(),
        }
    }
}

impl RuntimeScriptEventsSnapshot {
    fn from_state(state: &GameState) -> Self {
        let runtime = &state.script_runtime;
        Self {
            script_value: runtime.script_value.clone(),
            variables: runtime.variables.clone(),
            memory: runtime.memory.clone(),
            named_buffers: runtime.named_buffers.clone(),
            variable_sprites: runtime.variable_sprites.clone(),
            phone_numbers: runtime.phone_numbers.clone(),
            last_special_routine: runtime.last_special_routine.clone(),
            last_talked_object: runtime.last_talked_object.clone(),
            active_menu: runtime.active_menu.clone(),
            pending_delays: runtime.pending_delays.clone(),
            pending_earthquakes: runtime.pending_earthquakes.clone(),
            pending_emotes: runtime.pending_emotes.clone(),
            command_queue: runtime.command_queue.clone(),
            call_stack: runtime.call_stack.clone(),
            variable_writes: runtime.variable_writes.clone(),
            effects: runtime.effects.clone(),
            asm_directives: runtime.asm_directives.clone(),
            numeric_buffer_writes: runtime.numeric_buffer_writes.clone(),
            elevator_floors: runtime.elevator_floors.clone(),
            stone_table_entries: runtime.stone_table_entries.clone(),
            decoration_descriptions: runtime.decoration_descriptions.clone(),
            special_phone_calls: runtime.special_phone_calls.clone(),
            audio_events: runtime.audio_events.clone(),
            pending_music_fade: runtime.pending_music_fade.clone(),
            waiting_for_sound_effect: runtime.waiting_for_sound_effect,
            map_music_restart_disabled: runtime.map_music_restart_disabled,
            map_music_requested: runtime.map_music_requested,
            graphics_events: runtime.graphics_events.clone(),
            pending_screen_fade: runtime.pending_screen_fade.clone(),
            money_events: runtime.money_events.clone(),
            map_events: runtime.map_events.clone(),
            pending_script_warp: runtime.pending_script_warp.clone(),
            pending_map_load: runtime.pending_map_load.clone(),
            pending_map_refresh: runtime.pending_map_refresh.clone(),
            warp_check_requested: runtime.warp_check_requested,
            text_events: runtime.text_events.clone(),
            window_open: runtime.window_open,
            menu_coords: runtime.menu_coords,
            active_pokemon_picture: runtime.active_pokemon_picture.clone(),
            text_window_open: runtime.text_window_open,
            pending_text_label: runtime.pending_text_label.clone(),
            pending_text_wait: runtime.pending_text_wait.clone(),
            pending_yes_no: runtime.pending_yes_no.clone(),
            control_events: runtime.control_events.clone(),
            next_script: runtime.next_script.clone(),
            deferred_scripts: runtime.deferred_scripts.clone(),
            script_ended: runtime.script_ended.clone(),
            player_input_locked: runtime.player_input_locked,
            all_input_locked: runtime.all_input_locked,
            script_stop_requested: runtime.script_stop_requested,
            item_notify_queued: runtime.item_notify_queued,
            warp_sound_queued: runtime.warp_sound_queued,
            teleport_from_queued: runtime.teleport_from_queued,
            hall_of_fame_requested: runtime.hall_of_fame_requested,
            credits_requested: runtime.credits_requested,
            reset_requested: runtime.reset_requested,
            menu_2d_requested: runtime.menu_2d_requested,
            version_check_requested: runtime.version_check_requested,
            blackout_mod: runtime.blackout_mod.clone(),
            battle_tower_text: runtime.battle_tower_text.clone(),
            completed_trades: runtime.completed_trades.clone(),
            catch_tutorials: runtime.catch_tutorials.clone(),
            checked_mail_targets: runtime.checked_mail_targets.clone(),
            given_mail_targets: runtime.given_mail_targets.clone(),
            shop_events: runtime.shop_events.clone(),
            item_use_events: runtime.item_use_events.clone(),
        }
    }
}

impl RuntimeMenuSnapshot {
    fn from_state(
        state: &GameState,
        menu_id: String,
        source: RuntimeMenuSource,
        definition: Option<ScriptMenuDefinition>,
        vertical_menus: Vec<RuntimeVerticalMenuSnapshot>,
    ) -> Result<Self> {
        let layout =
            RuntimeMenuLayoutSnapshot::from_definition(definition.as_ref(), vertical_menus)
                .with_context(|| format!("parse runtime menu layout for {menu_id}"))?;
        Ok(Self {
            menu_id,
            source,
            definition,
            layout,
            window_open: state.script_runtime.window_open,
            coords: state.script_runtime.menu_coords,
            menu_2d_requested: state.script_runtime.menu_2d_requested,
        })
    }
}

impl RuntimeMenuLayoutSnapshot {
    fn from_definition(
        definition: Option<&ScriptMenuDefinition>,
        vertical_menus: Vec<RuntimeVerticalMenuSnapshot>,
    ) -> Result<Self> {
        let Some(definition) = definition else {
            return Ok(Self {
                declared_coords: None,
                data_commands: Vec::new(),
                vertical_menus,
            });
        };
        let mut declared_coords = None;
        let mut data_commands = Vec::new();
        for command in &definition.commands {
            match command.command.as_str() {
                "menu_coords" => {
                    let coords = parse_menu_coords(&command.args).with_context(|| {
                        format!("parse menu_coords command {}", command.command_index)
                    })?;
                    declared_coords = Some(coords);
                }
                "db" | "dw" => data_commands.push(RuntimeMenuDataCommandSnapshot {
                    command: command.command.clone(),
                    args: command.args.clone(),
                    command_index: command.command_index,
                }),
                _ => {}
            }
        }
        Ok(Self {
            declared_coords,
            data_commands,
            vertical_menus,
        })
    }
}

impl RuntimeVerticalMenuSnapshot {
    fn from_definition(definition: &crystal_assets::ScriptVerticalMenuDefinition) -> Self {
        Self {
            source_script: definition.source_script.clone(),
            loadmenu_command_index: definition.loadmenu_command_index,
            verticalmenu_command_index: definition.verticalmenu_command_index,
            header_label: definition.header_label.clone(),
            data_label: definition.data_label.clone(),
            options: definition.options.clone(),
            two_dimensional: definition.two_dimensional,
            rows: definition.rows,
            columns: definition.columns,
            spacing: definition.spacing,
        }
    }
}

impl RuntimeElevatorSnapshot {
    fn from_definition(
        map_name: &str,
        definition: &crystal_assets::ScriptElevatorDefinition,
    ) -> Self {
        Self {
            map_name: map_name.to_string(),
            source_script: definition.source_script.clone(),
            elevator_command_index: definition.elevator_command_index,
            data_label: definition.data_label.clone(),
            floors: definition
                .floors
                .iter()
                .enumerate()
                .map(|(floor_index, floor)| RuntimeElevatorFloorSnapshot {
                    floor_index,
                    floor: floor.floor.clone(),
                    warp: floor.warp,
                    target_map: floor.target_map.clone(),
                    source_script: floor.source_script.clone(),
                    command_index: floor.command_index,
                })
                .collect(),
        }
    }
}

impl RuntimeGiftPokemonSnapshot {
    fn from_script(map_name: &str, gift: &GiftPokemonScript) -> Self {
        Self {
            map_name: map_name.to_string(),
            source_script: gift.source_script.clone(),
            command_index: gift.command_index,
            species_id: gift.species_id.clone(),
            level: gift.level,
            level_token: gift.level_token.clone(),
            held_item_id: gift.held_item_id.clone(),
            nickname_label: gift.nickname_label.clone(),
            ot_label: gift.ot_label.clone(),
            egg: gift.egg,
        }
    }
}

fn parse_menu_coords(args: &[String]) -> Result<[i16; 4]> {
    if args.len() != 4 {
        anyhow::bail!("menu_coords requires 4 operands, got {}", args.len());
    }
    Ok([
        parse_menu_coord(&args[0], "left")?,
        parse_menu_coord(&args[1], "top")?,
        parse_menu_coord(&args[2], "right")?,
        parse_menu_coord(&args[3], "bottom")?,
    ])
}

fn parse_menu_coord(value: &str, name: &str) -> Result<i16> {
    parse_menu_coord_expression(value)
        .with_context(|| format!("menu coordinate {name} must be an exact i16, got {value:?}"))
}

fn parse_menu_coord_expression(value: &str) -> Result<i16> {
    parse_menu_coord_token("menu_coords", value).map_err(anyhow::Error::new)
}

impl RuntimeUiSnapshot {
    fn from_state(
        state: &GameState,
        menu: Option<RuntimeMenuSnapshot>,
        elevators: Vec<RuntimeElevatorSnapshot>,
        gift_pokemon: Vec<RuntimeGiftPokemonSnapshot>,
        text: Option<RuntimeTextSnapshot>,
    ) -> Self {
        let runtime = &state.script_runtime;
        Self {
            menu,
            elevators,
            gift_pokemon,
            text,
            window_open: runtime.window_open,
            text_window_open: runtime.text_window_open,
            coords: runtime.menu_coords,
            active_pokemon_picture: runtime.active_pokemon_picture.clone(),
            pending_yes_no: runtime.pending_yes_no.clone(),
            pending_text_wait: runtime.pending_text_wait.clone(),
        }
    }
}

impl RuntimeBattleSnapshot {
    pub fn phase(&self) -> RuntimeShellPhase {
        match &self.kind {
            RuntimeBattleKind::Wild { .. } => RuntimeShellPhase::WildBattle,
            RuntimeBattleKind::StaticWild { .. } => RuntimeShellPhase::StaticWildBattle,
            RuntimeBattleKind::Trainer { .. } => RuntimeShellPhase::TrainerBattle,
        }
    }

    fn from_state(state: &GameState) -> Result<Option<Self>> {
        match &state.battle {
            BattleMemory::Inactive => Ok(None),
            BattleMemory::Wild {
                battle_type,
                battle_music,
                map_name,
                enemy_pokemon,
                enemy_party,
                ..
            } => Self::from_parts(
                state,
                RuntimeBattleKind::Wild {
                    map_name: map_name.clone(),
                    battle_music: battle_music.clone(),
                },
                battle_type,
                enemy_pokemon,
                enemy_party,
            )
            .map(Some),
            BattleMemory::StaticWild {
                battle_type,
                battle_music,
                origin_map_name,
                species,
                level,
                source_script,
                startbattle_command_index,
                resume_command_index,
                enemy_pokemon,
                enemy_party,
            } => Self::from_parts(
                state,
                RuntimeBattleKind::StaticWild {
                    origin_map_name: origin_map_name.clone(),
                    species: species.clone(),
                    level: *level,
                    source_script: source_script.clone(),
                    startbattle_command_index: *startbattle_command_index,
                    resume_command_index: *resume_command_index,
                    battle_music: battle_music.clone(),
                },
                battle_type,
                enemy_pokemon,
                enemy_party,
            )
            .map(Some),
            BattleMemory::Trainer {
                battle_type,
                trainer_class,
                trainer_id,
                trainer_name,
                event_flag,
                seen_text,
                win_text,
                loss_text,
                callback,
                source_script,
                enemy_pokemon,
                enemy_party,
                reward,
                encounter_music,
                ai_move_flags,
                ai_item_switch_flags,
                ai_layers,
            } => Self::from_parts(
                state,
                RuntimeBattleKind::Trainer {
                    trainer_class: trainer_class.clone(),
                    trainer_id: trainer_id.clone(),
                    trainer_name: trainer_name.clone(),
                    event_flag: event_flag.clone(),
                    seen_text: seen_text.clone(),
                    win_text: win_text.clone(),
                    loss_text: loss_text.clone(),
                    callback: callback.clone(),
                    source_script: source_script.clone(),
                    reward: *reward,
                    encounter_music: encounter_music.clone(),
                    ai_move_flags: *ai_move_flags,
                    ai_item_switch_flags: *ai_item_switch_flags,
                    ai_layers: ai_layers.clone(),
                },
                battle_type,
                enemy_pokemon,
                enemy_party,
            )
            .map(Some),
        }
    }

    fn from_parts(
        state: &GameState,
        kind: RuntimeBattleKind,
        battle_type: &str,
        enemy_pokemon: &Pokemon,
        enemy_party: &[Pokemon],
    ) -> Result<Self> {
        let commands = RuntimeBattleCommandSnapshot::from_state(state, &kind, enemy_pokemon)?;
        let battle_music = battle_music_from_kind(&kind);
        let combat = state.script_runtime.active_battle_combat.as_ref();
        let active_player = state
            .battle_active_party_index
            .and_then(|index| state.storage.party.pokemon.get(index))
            .and_then(Option::as_ref)
            .with_context(|| "active battle snapshot is missing its player Pokemon")?;
        let player_moves = combat
            .map(|combat| {
                combat
                    .player_transform
                    .as_ref()
                    .map(|transform| transform.moves.clone())
                    .unwrap_or_else(|| combat.player.moves.clone())
            })
            .unwrap_or_else(|| active_player.moves.clone());
        let enemy_moves = combat
            .map(|combat| {
                combat
                    .enemy_transform
                    .as_ref()
                    .map(|transform| transform.moves.clone())
                    .unwrap_or_else(|| combat.enemy.moves.clone())
            })
            .unwrap_or_else(|| enemy_pokemon.moves.clone());
        Ok(Self {
            kind,
            battle_music,
            battle_type: battle_type.to_string(),
            enemy_pokemon: enemy_pokemon.clone(),
            enemy_party: enemy_party.to_vec(),
            active_player_party_index: state.battle_active_party_index,
            active_enemy_party_index: state.battle_active_enemy_party_index,
            player_transformed_species: combat
                .and_then(|combat| combat.player_transform.as_ref())
                .map(|transform| transform.species.id.clone()),
            enemy_transformed_species: combat
                .and_then(|combat| combat.enemy_transform.as_ref())
                .map(|transform| transform.species.id.clone()),
            player_substitute_hp: combat.map_or(0, |combat| combat.player_substitute_hp),
            enemy_substitute_hp: combat.map_or(0, |combat| combat.enemy_substitute_hp),
            player_semi_invulnerable: combat
                .is_some_and(|combat| combat.player_airborne_move.is_some()),
            enemy_semi_invulnerable: combat
                .is_some_and(|combat| combat.enemy_airborne_move.is_some()),
            player_moves,
            enemy_moves,
            player_last_move: combat.and_then(|combat| combat.player_last_move.clone()),
            player_used_moves: combat
                .map(|combat| combat.player_used_moves.clone())
                .unwrap_or_default(),
            enemy_last_move: combat.and_then(|combat| combat.enemy_last_move.clone()),
            enemy_toxic_turns: combat.map_or(0, |combat| combat.enemy_toxic_turns),
            enemy_turns_taken: combat.map_or(0, |combat| combat.enemy_turns_taken),
            enemy_switch_locked: combat.is_some_and(|combat| {
                combat.enemy_recharge_move.is_some()
                    || combat.enemy_airborne_move.is_some()
                    || combat.enemy_charging_move.is_some()
                    || combat.enemy.rampage_turns > 0
                    || combat.enemy_bide_turns > 0
                    || combat.enemy_rollout_turns > 0
            }),
            player_cannot_escape: combat.is_some_and(|combat| combat.player_escape_trap.is_some()),
            player_wrapped: combat.is_some_and(|combat| combat.player_trap.is_some()),
            enemy_wrapped: combat.is_some_and(|combat| combat.enemy_trap.is_some()),
            rewarded_enemy_party_indices: state
                .battle_rewarded_enemy_party_indices
                .iter()
                .copied()
                .collect(),
            escape_attempts: state.battle_escape_attempts,
            player_stat_drop_guard_turns: state.battle_player_stat_drop_guard_turns,
            pay_day_money: state.battle_pay_day_money,
            amulet_coin_active: state.battle_amulet_coin_active,
            trainer_items_used: state
                .script_runtime
                .active_battle_combat
                .as_ref()
                .map(|combat| combat.trainer_items_used.clone())
                .unwrap_or_default(),
            player_disabled_move: state
                .script_runtime
                .active_battle_combat
                .as_ref()
                .and_then(|combat| combat.player_disable.as_ref())
                .filter(|disable| disable.turns_remaining > 0)
                .map(|disable| disable.move_name.clone()),
            commands,
        })
    }
}

fn battle_music_from_kind(kind: &RuntimeBattleKind) -> String {
    match kind {
        RuntimeBattleKind::Wild { battle_music, .. }
        | RuntimeBattleKind::StaticWild { battle_music, .. } => battle_music.clone(),
        RuntimeBattleKind::Trainer {
            encounter_music, ..
        } => encounter_music.clone(),
    }
}

impl RuntimeBattleCommandSnapshot {
    fn from_state(
        state: &GameState,
        kind: &RuntimeBattleKind,
        enemy_pokemon: &Pokemon,
    ) -> Result<Self> {
        let player_disable = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .and_then(|combat| combat.player_disable.as_ref())
            .filter(|disable| disable.turns_remaining > 0)
            .map(|disable| disable.move_name.as_str());
        let mut player_forced_struggle = false;
        let player_turn_automatic = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .is_some_and(|combat| {
                combat.player.hp > 0
                    && (combat.player_recharge_move.is_some()
                        || combat.player_airborne_move.is_some()
                        || combat.player_charging_move.is_some()
                        || combat.player.rampage_turns > 0
                        || combat.player_rollout_turns > 0)
            });
        let player_fight_automatic = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .is_some_and(|combat| combat.player.hp > 0 && combat.player_bide_turns > 0);
        let player_move_slots = state
            .battle_active_party_index
            .map(|index| {
                let pokemon = state
                    .storage
                    .party
                    .pokemon
                    .get(index)
                    .with_context(|| {
                        format!("active battle party index {index} is outside saved party")
                    })?
                    .as_ref()
                    .with_context(|| {
                        format!("active battle party index {index} points to an empty party slot")
                    })?;
                let moves = state
                    .script_runtime
                    .active_battle_combat
                    .as_ref()
                    .map(|combat| {
                        combat
                            .player_transform
                            .as_ref()
                            .map(|transform| transform.moves.as_slice())
                            .unwrap_or(combat.player.moves.as_slice())
                    })
                    .unwrap_or(pokemon.moves.as_slice());
                player_forced_struggle = !moves.iter().take(BATTLE_MOVE_SLOTS).any(|learned| {
                    learned.current_pp > 0 && player_disable != Some(learned.name.as_str())
                });
                Ok::<Vec<usize>, anyhow::Error>((0..moves.len().min(BATTLE_MOVE_SLOTS)).collect())
            })
            .transpose()?
            .with_context(|| "active battle snapshot is missing active player party index")?;
        let enemy_moves = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .map(|combat| {
                combat
                    .enemy_transform
                    .as_ref()
                    .map(|transform| transform.moves.as_slice())
                    .unwrap_or(combat.enemy.moves.as_slice())
            })
            .unwrap_or(enemy_pokemon.moves.as_slice());
        let enemy_disable = state
            .script_runtime
            .active_battle_combat
            .as_ref()
            .and_then(|combat| combat.enemy_disable.as_ref())
            .filter(|disable| disable.turns_remaining > 0)
            .map(|disable| disable.move_name.as_str());
        let enemy_move_slots = available_learned_move_slots(enemy_moves, enemy_disable);
        let switch_party_indices = state
            .storage
            .party
            .pokemon
            .iter()
            .enumerate()
            .filter_map(|(index, pokemon)| {
                let pokemon = pokemon.as_ref()?;
                (Some(index) != state.battle_active_party_index && pokemon.hp > 0).then_some(index)
            })
            .collect();
        Ok(Self {
            player_move_slots,
            player_forced_struggle,
            player_turn_automatic,
            player_fight_automatic,
            enemy_move_slots,
            switch_party_indices,
            can_use_items: state.battle_active_party_index.is_some(),
            can_run: matches!(
                kind,
                RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. }
            ),
        })
    }
}

fn available_move_slots(pokemon: &Pokemon) -> Vec<usize> {
    available_learned_move_slots(&pokemon.moves, None)
}

fn available_learned_move_slots(moves: &[LearnedMove], disabled_move: Option<&str>) -> Vec<usize> {
    moves
        .iter()
        .take(BATTLE_MOVE_SLOTS)
        .enumerate()
        .filter_map(|(slot, learned)| {
            (learned.current_pp > 0 && disabled_move != Some(learned.name.as_str())).then_some(slot)
        })
        .collect()
}

pub(crate) fn validate_deterministic_replay_runtime_authority(
    bundle: &DeterministicReplayBundle,
    player_id: PlayerId,
) -> Result<()> {
    bundle
        .validate()
        .context("validate deterministic replay framing")?;
    let journal = bundle.input_journal().journal();
    let mut covered_input_frames = BTreeSet::new();
    for command in bundle.runtime_commands() {
        let request = command.command();
        let decoded =
            decode_runtime_mutation_command_payload(request.payload()).with_context(|| {
                format!(
                    "decode deterministic runtime command sequence {}",
                    request.sequence()
                )
            })?;
        if let RuntimeMutationCommand::ApplyOverworldInput(input) = decoded {
            let frame = request.expected_state().frame();
            let journal_frame = journal
                .frames()
                .iter()
                .find(|candidate| candidate.frame() == frame)
                .with_context(|| {
                    format!(
                        "ApplyOverworldInput sequence {} has no input-journal frame {frame}",
                        request.sequence()
                    )
                })?;
            let expected_mask = journal_frame.joypad_mask_for(player_id).with_context(|| {
                format!("runtime input journal frame {frame} is missing local player {player_id}")
            })?;
            let actual_mask = JoypadState::compute_mask(input.buttons);
            if actual_mask != expected_mask {
                anyhow::bail!(
                    "ApplyOverworldInput sequence {} mask {actual_mask:#04x} does not match journal frame {frame} mask {expected_mask:#04x}",
                    request.sequence()
                );
            }
            if !covered_input_frames.insert(frame) {
                anyhow::bail!(
                    "input-journal frame {frame} is covered by more than one ApplyOverworldInput command"
                );
            }
        }
    }
    for frame in journal.frames() {
        if !covered_input_frames.contains(&frame.frame()) {
            anyhow::bail!(
                "input-journal frame {} has no ApplyOverworldInput command",
                frame.frame()
            );
        }
    }
    Ok(())
}

fn runtime_party_recovery(
    outcome: PartyRecoveryOutcome,
    state_checksum: StateChecksum,
) -> RuntimePartyRecovery {
    RuntimePartyRecovery {
        party_index: outcome.party_index,
        species_id: outcome.species_id,
        hp_before: outcome.hp_before,
        hp_after: outcome.hp_after,
        status_before: outcome.status_before,
        status_after: outcome.status_after,
        pp_restored: outcome.pp_restored,
        state_checksum,
    }
}

fn runtime_blackout_recovery(
    outcome: BlackoutRecoveryOutcome,
    state_checksum: StateChecksum,
) -> RuntimeBlackoutRecovery {
    RuntimeBlackoutRecovery {
        spawn_identifier: outcome.spawn_identifier,
        map_name: outcome.map_name,
        tile: outcome.tile,
        healed: outcome
            .healed
            .into_iter()
            .map(|healed| runtime_party_recovery(healed, state_checksum.clone()))
            .collect(),
        state_checksum,
    }
}

impl RuntimePartySnapshot {
    fn from_state(state: &GameState) -> Self {
        Self {
            slots: state
                .storage
                .party
                .pokemon
                .iter()
                .enumerate()
                .filter_map(|(index, pokemon)| {
                    pokemon.clone().map(|pokemon| RuntimePartySlotSnapshot {
                        index,
                        pokemon,
                        is_active_battle_pokemon: state.battle_active_party_index == Some(index),
                    })
                })
                .collect(),
            active_battle_slot: state.battle_active_party_index,
        }
    }
}

impl RuntimeStorageSnapshot {
    fn from_state(state: &GameState) -> Self {
        Self {
            current_pc_box: state.current_pc_box,
            party_count: state.storage.party.filled_slots(),
            boxes: state
                .storage
                .pc_boxes
                .iter()
                .enumerate()
                .map(RuntimePcBoxSnapshot::from_box)
                .collect(),
        }
    }
}

impl RuntimePcBoxSnapshot {
    fn from_box((index, pc_box): (usize, &PcBox)) -> Self {
        Self {
            index,
            name: pc_box.name.clone(),
            count: pc_box.count,
            slots: pc_box
                .pokemon
                .iter()
                .enumerate()
                .filter_map(|(slot, pokemon)| {
                    pokemon.clone().map(|pokemon| RuntimePcBoxSlotSnapshot {
                        index: slot,
                        pokemon,
                    })
                })
                .collect(),
        }
    }
}

impl RuntimeBagSnapshot {
    fn inventory(inventory: &BTreeMap<String, u16>) -> Vec<RuntimeBagItemSnapshot> {
        inventory
            .iter()
            .map(|(item_id, quantity)| RuntimeBagItemSnapshot {
                item_id: item_id.clone(),
                quantity: *quantity,
            })
            .collect()
    }

    fn tm_hm(
        items: &BTreeMap<String, Item>,
        quantities: &[u8],
    ) -> Result<Vec<RuntimeTmHmSnapshot>> {
        let mut entries = Vec::new();
        for (item_id, item) in items {
            if item.pocket != ITEM_POCKET_TM_HM {
                continue;
            }
            let Some(index) = item.tmhm_index else {
                continue;
            };
            let quantity = quantities.get(index).copied().with_context(|| {
                format!(
                    "saved TM/HM flags missing index {index} required by compiled item {item_id}"
                )
            })?;
            if quantity > 0 {
                entries.push(RuntimeTmHmSnapshot {
                    item_id: item_id.clone(),
                    tmhm_index: index,
                    move_id: item.tmhm_move.clone(),
                    quantity: u16::from(quantity),
                });
            }
        }
        entries.sort_by(|left, right| {
            left.tmhm_index
                .cmp(&right.tmhm_index)
                .then_with(|| left.item_id.cmp(&right.item_id))
        });
        Ok(entries)
    }
}

impl RuntimeItemCatalogSnapshot {
    fn from_item(
        (item_id, item): (&String, &Item),
        evolutions: &crystal_core::systems::evolution::EvolutionTable,
    ) -> Self {
        Self {
            item_id: item_id.clone(),
            name: item.name.clone(),
            description: item.description.clone(),
            effect: item.effect.clone(),
            status_heals: item.status_heals.clone(),
            revive_hp_percent: item.revive_hp_percent,
            party_revive_hp_percent: item.party_revive_hp_percent,
            pp_restore_scope: item.pp_restore_scope.clone(),
            pp_restore_points: item.pp_restore_points,
            pp_up_stages: item.pp_up_stages,
            vitamin_stat: item.vitamin_stat.clone(),
            vitamin_stat_exp: item.vitamin_stat_exp,
            vitamin_max_stat_exp: item.vitamin_max_stat_exp,
            rare_candy_level_gain: item.rare_candy_level_gain,
            party_special_effect: party_special_item_effect_plan(item, evolutions).is_some(),
            battle_stat_boost_stat: item.battle_stat_boost_stat.clone(),
            battle_stat_boost_stages: item.battle_stat_boost_stages,
            battle_escape_mode: item.battle_escape_mode.clone(),
            battle_focus_energy: item.battle_focus_energy,
            battle_stat_drop_guard: item.battle_stat_drop_guard,
            battle_stat_drop_guard_turns: item.battle_stat_drop_guard_turns,
            confusion_heal: item.confusion_heal,
            repel_steps: item.repel_steps,
            escape_rope_mode: item.escape_rope_mode.clone(),
            price: item.price,
            held_effect: item.held_effect.clone(),
            parameter: item.parameter,
            property: item.property.clone(),
            pocket: item.pocket.clone(),
            field_menu: item.field_menu.clone(),
            field_usable: item.field_usable,
            battle_menu: item.battle_menu.clone(),
            battle_usable: item.battle_usable,
            script_name: item.script_name.clone(),
            consumable: item.consumable,
            tmhm_index: item.tmhm_index,
            tmhm_move: item.tmhm_move.clone(),
        }
    }
}

impl RuntimeItemEffectPlanKey {
    fn from_plan(plan: BattleItemEffectPlan) -> Self {
        Self {
            item_id: plan.item_id,
            effect_id: plan.effect_id,
            behavior_id: plan.behavior_id,
        }
    }
}

impl RuntimeMoveCatalogSnapshot {
    fn from_move((move_id, move_data): (&String, &Move)) -> Self {
        Self {
            move_id: move_id.clone(),
            name: move_data.name.clone(),
            move_type: move_data.move_type.clone(),
            power: move_data.power,
            accuracy: move_data.accuracy,
            pp: move_data.pp,
            effect: move_data.effect.clone(),
            effect_chance: move_data.effect_chance,
            stat: move_data.stat.clone(),
            amount: move_data.amount,
        }
    }
}

impl RuntimePokemonCatalogSnapshot {
    fn from_species((species_id, species): (&String, &PokemonSpecies)) -> Self {
        Self {
            species_id: species_id.clone(),
            int_id: species.int_id,
            base_stats: species.base_stats,
            type1: species.type1.clone(),
            type2: species.type2.clone(),
            catch_rate: species.catch_rate,
            base_exp: species.base_exp,
            item1: species.item1.clone(),
            item2: species.item2.clone(),
            gender_ratio: species.gender_ratio,
            unknown1: species.unknown1,
            step_cycles_to_hatch: species.step_cycles_to_hatch,
            unknown2: species.unknown2,
            growth_rate: species.growth_rate.clone(),
            egg_group1: species.egg_group1.clone(),
            egg_group2: species.egg_group2.clone(),
            tmhm_learnset: species.tmhm_learnset.clone(),
            ability: species.ability.clone(),
            pic_size: species.pic_size,
            front_pic: species.front_pic,
            back_pic: species.back_pic,
            weight: species.weight,
        }
    }
}

impl RuntimeTrainerCatalogSnapshot {
    fn from_trainer(trainer: &Trainer) -> Self {
        Self {
            trainer_id: trainer.trainer_id.clone(),
            name: trainer.name.clone(),
            trainer_class: trainer.trainer_class.clone(),
            party: trainer
                .party
                .iter()
                .map(RuntimeTrainerPartyPokemonSnapshot::from_party_pokemon)
                .collect(),
            win_quote: trainer.win_quote.clone(),
            lose_quote: trainer.lose_quote.clone(),
            items: trainer.items.clone(),
            base_reward: trainer.base_reward,
            ai_move_flags: trainer.ai_move_flags,
            ai_item_switch_flags: trainer.ai_item_switch_flags,
            encounter_music: trainer.encounter_music.clone(),
            ai_layers: trainer.ai_layers.clone(),
        }
    }
}

impl RuntimeTrainerPartyPokemonSnapshot {
    fn from_party_pokemon(pokemon: &crystal_core::models::TrainerPartyPokemon) -> Self {
        Self {
            species: pokemon.species.clone(),
            level: pokemon.level,
            item: pokemon.item.clone(),
            moves: pokemon
                .moves
                .iter()
                .map(RuntimeLearnedMoveSnapshot::from_learned_move)
                .collect(),
            dvs: pokemon.dvs,
        }
    }
}

impl RuntimeLearnedMoveSnapshot {
    fn from_learned_move(move_data: &crystal_core::models::LearnedMove) -> Self {
        Self {
            name: move_data.name.clone(),
            current_pp: move_data.current_pp,
            pp_ups: move_data.pp_ups,
        }
    }
}

impl RuntimeMapCatalogSnapshot {
    fn from_module(
        map_name: &str,
        module: &crystal_assets::MapModule,
        metadata: Option<&crystal_assets::RuntimeMapMetadata>,
    ) -> Self {
        Self {
            map_name: map_name.to_string(),
            id: module.id.clone(),
            attributes: module.attributes.clone(),
            metadata: metadata.map(RuntimeMapMetadataSnapshot::from_metadata),
            scenes: module.scenes.clone(),
            events: module.events.clone(),
            objects: module.objects.clone(),
            blocks: module.blocks.clone(),
        }
    }
}

impl RuntimeMapMetadataSnapshot {
    fn from_metadata(metadata: &crystal_assets::RuntimeMapMetadata) -> Self {
        Self {
            constant: metadata.constant.clone(),
            name: metadata.name.clone(),
            group_name: metadata.group_name.clone(),
            group_id: metadata.group_id,
            map_id: metadata.map_id,
            width: metadata.width,
            height: metadata.height,
            environment: metadata.environment.clone(),
            phone_service: metadata.phone_service,
        }
    }
}

impl RuntimeTilesetCatalogSnapshot {
    fn from_tileset((tileset_id, tileset): (&String, &TilesetDefinition)) -> Self {
        Self {
            tileset_id: tileset_id.clone(),
            collision: tileset.collision.clone(),
            palette_map: tileset.palette_map.clone(),
        }
    }
}

impl RuntimeEncounterCatalogSnapshot {
    fn from_data(data: &GameDataSet) -> Self {
        Self {
            wild: data.wild_encounters.clone(),
            field: data.field_encounters.clone(),
            slot_tables: data.encounter_slot_tables.clone(),
            fishing: data.fishing.clone(),
        }
    }
}

impl RuntimeBattleRuleCatalogSnapshot {
    fn from_data(data: &GameDataSet) -> Self {
        Self {
            capture_rules: data.capture_rules.clone(),
            capture_wobble_probabilities: data.capture_wobble_probabilities.clone(),
            stat_multipliers: data.battle_stat_multipliers.clone(),
            move_priorities: data.move_priorities.clone(),
            type_categories: data.type_categories.clone(),
            type_effectiveness: data.type_effectiveness.clone(),
            weather_modifiers: data.weather_modifiers.clone(),
            reward_rules: data.battle_reward_rules.clone(),
            escape_rules: data.battle_escape_rules.clone(),
        }
    }
}

impl RuntimeWorldRuleCatalogSnapshot {
    fn from_data(data: &GameDataSet) -> Self {
        Self {
            marts: data.marts.clone(),
            currency: data.currency_constants.clone(),
            fruit_trees: data.fruit_trees.clone(),
            field_moves: data.field_moves.clone(),
        }
    }
}

impl RuntimePresentationCatalogSnapshot {
    fn from_data(data: &GameDataSet) -> Self {
        Self {
            pc_strings: data.pc_strings.clone(),
            menu_icons: data.menu_icons.clone(),
            pokedex_entries: data.pokedex_entries.clone(),
            pokemon_frontpic_anim: data.pokemon_frontpic_anim.clone(),
            asm_text: data.asm_text.clone(),
            move_names: data.move_names.clone(),
            battle_animations: data.battle_animations.clone(),
            battle_animation_table: data.battle_animation_table.clone(),
            battle_anim_bundle: data.battle_anim_bundle.clone(),
            sprite_anim_bundle: data.sprite_anim_bundle.clone(),
            sprite_palette_defaults: data.sprite_palette_defaults.clone(),
            pokegear_town_map_palette_map: data.pokegear_town_map_palette_map.clone(),
            pokegear_landmarks: data.pokegear_landmarks.clone(),
            pokemon_cries: data.pokemon_cries.clone(),
        }
    }
}

impl RuntimeSpecialCatalogSnapshot {
    fn from_data(data: &GameDataSet) -> Self {
        Self {
            phone_contacts: data.phone_contacts.clone(),
            permanent_phone_numbers: data.permanent_phone_numbers.clone(),
            special_phone_calls: data.special_phone_calls.clone(),
            npc_trades: data.npc_trades.clone(),
            special_routines: data.special_routines.clone(),
            flee_mons: data.flee_mons.clone(),
            buena_password_categories: data.buena_password_categories.clone(),
            roaming_pokemon: data.roaming_pokemon.clone(),
            buena_prizes: data.buena_prizes.clone(),
            kurt_apricorn_recipes: data.kurt_apricorn_recipes.clone(),
            shuckie_gift: data.shuckie_gift.clone(),
            dratini_move_sets: data.dratini_move_sets.clone(),
            bug_contest_config: data.bug_contest_config.clone(),
            battle_tower_rules: data.battle_tower_rules.clone(),
            oak_ratings: data.oak_ratings.clone(),
            odd_egg_definitions: data.odd_egg_definitions.clone(),
            magikarp_lengths: data.magikarp_lengths.clone(),
            happiness_data: data.happiness_data.clone(),
        }
    }
}

impl RuntimeStoryCatalogSnapshot {
    fn from_data(data: &GameDataSet) -> Self {
        Self {
            initialize_events: data.initialize_events.clone(),
            story_event_script_constants: data.story_event_script_constants.clone(),
        }
    }
}

impl RuntimeAudioCatalogSnapshot {
    fn from_catalog(catalog: &RuntimeAudioCatalog) -> Self {
        Self {
            manifest: catalog.manifest.clone(),
            playback: catalog.playback.clone(),
            music: catalog
                .music
                .iter()
                .map(|(id, program)| {
                    (
                        id.clone(),
                        RuntimeAudioProgramSnapshot::from_program(program),
                    )
                })
                .collect(),
            sound_effects: catalog
                .sound_effects
                .iter()
                .map(|(id, program)| {
                    (
                        id.clone(),
                        RuntimeAudioProgramSnapshot::from_program(program),
                    )
                })
                .collect(),
            cries: catalog
                .cries
                .iter()
                .map(|(id, program)| {
                    (
                        id.clone(),
                        RuntimeAudioProgramSnapshot::from_program(program),
                    )
                })
                .collect(),
        }
    }
}

impl RuntimeAudioProgramSnapshot {
    fn from_program(program: &AudioProgram) -> Self {
        Self {
            cache_key: program.cache_key.clone(),
            source: RuntimeAudioProgramSourceSnapshot::from_source(&program.source),
        }
    }
}

impl RuntimeAudioProgramSourceSnapshot {
    fn from_source(source: &AudioProgramSource) -> Self {
        match source {
            AudioProgramSource::Midi(bytes) => Self::Midi {
                byte_len: bytes.len(),
            },
            AudioProgramSource::Pcm {
                bytes,
                format,
                loop_start_sample,
                loop_end_sample,
            } => Self::Pcm {
                byte_len: bytes.len(),
                format: format.clone(),
                loop_start_sample: *loop_start_sample,
                loop_end_sample: *loop_end_sample,
            },
            AudioProgramSource::PcmGzip {
                bytes,
                format,
                loop_start_sample,
                loop_end_sample,
                ..
            } => Self::PcmGzip {
                byte_len: bytes.len(),
                format: format.clone(),
                loop_start_sample: *loop_start_sample,
                loop_end_sample: *loop_end_sample,
            },
        }
    }
}

impl RuntimeShellPhase {
    fn from_state(state: &GameState) -> Self {
        if state.script_runtime.pending_yes_no.is_some() {
            Self::YesNo
        } else if state.script_runtime.pending_text_wait.is_some()
            || state.script_runtime.pending_text_label.is_some()
            || state.script_runtime.text_window_open
        {
            Self::Text
        } else if state.script_runtime.pending_shop.is_some() {
            Self::Shop
        } else if state.script_runtime.active_menu.is_some() {
            Self::Menu
        } else {
            match state.battle {
                BattleMemory::Inactive => Self::Overworld,
                BattleMemory::Wild { .. } => Self::WildBattle,
                BattleMemory::StaticWild { .. } => Self::StaticWildBattle,
                BattleMemory::Trainer { .. } => Self::TrainerBattle,
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptedBattleCompletion {
    pub continued_after_battle: bool,
    pub trainer_prize_money: Option<u32>,
    pub money_after: Option<u32>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCaptureCompletion {
    pub stored: Option<StoredCapture>,
    pub contest_pokemon: Option<crystal_core::models::Pokemon>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCaptureAttempt {
    pub outcome: Option<CaptureOutcome>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleTurn {
    pub outcome: BattleTurnOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleCommand {
    pub outcome: ActiveBattleCommandOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattlePartySwitch {
    pub party_index: usize,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleEscape {
    pub outcome: BattleEscapeAttempt,
    pub state_checksum: StateChecksum,
}

fn preview_active_wild_battle_escape_rng_seed_after(
    data: &GameDataSet,
    state: &GameState,
) -> Result<u32> {
    let mut preview = state.clone();
    data.resolve_active_wild_battle_run(&mut preview)
        .context("preview active wild battle escape")?;
    Ok(preview.rng_seed)
}

fn preview_throw_ball_rng_seed_after(
    data: &GameDataSet,
    state: &GameState,
    ball_id: &str,
) -> Result<u32> {
    let mut preview = state.clone();
    data.throw_ball_at_active_battle(&mut preview, ball_id)
        .with_context(|| format!("preview throw ball {ball_id}"))?;
    Ok(preview.rng_seed)
}

fn preview_headbutt_field_move_rng_seed_after(
    data: &GameDataSet,
    state: &GameState,
    overworld: &OverworldSession,
    party_index: usize,
    player_id: u16,
) -> Result<u32> {
    let mut preview = state.clone();
    data.use_headbutt_field_move(&mut preview, overworld, party_index, player_id)
        .context("preview HEADBUTT field move")?;
    Ok(preview.rng_seed)
}

fn preview_odd_egg_rng_seed_after(
    data: &GameDataSet,
    state: &GameState,
    music_ids: &BTreeSet<String>,
) -> Result<u32> {
    let mut preview = state.clone();
    data.apply_special_routine(&mut preview, "GiveOddEgg", music_ids)
        .context("preview GiveOddEgg")?;
    Ok(preview.rng_seed)
}

fn preview_generic_special_routine_rng_seed_after(
    data: &GameDataSet,
    state: &GameState,
    music_ids: &BTreeSet<String>,
    routine: &str,
) -> Result<Option<u32>> {
    if runtime_special_routine_requires_divider_trace(routine) {
        anyhow::bail!(
            "special routine {routine} must execute once with an authoritative divider source"
        );
    }
    if !runtime_special_routine_requires_legacy_seed_boundary(routine) {
        return Ok(None);
    }
    let mut preview = state.clone();
    data.apply_special_routine(&mut preview, routine, music_ids)
        .with_context(|| format!("preview special routine {routine}"))?;
    Ok(Some(preview.rng_seed))
}

fn preview_buena_password_rng_seed_after(
    data: &GameDataSet,
    state: &GameState,
    music_ids: &BTreeSet<String>,
    guess: Option<&str>,
) -> Result<u32> {
    let mut preview = state.clone();
    match guess {
        Some(guess) => {
            preview
                .script_runtime
                .variables
                .insert("BUENA_PASSWORD".to_string(), guess.to_string());
        }
        None => {
            preview.script_runtime.variables.remove("BUENA_PASSWORD");
        }
    }
    data.apply_special_routine(&mut preview, "BuenasPassword", music_ids)
        .context("preview BuenasPassword")?;
    Ok(preview.rng_seed)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleEscapeItemUse {
    pub item_use: ItemUseOutcome,
    pub battle_escape_mode: String,
    pub escaped: bool,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleStateItemUse {
    pub item_use: ItemUseOutcome,
    pub stat_drop_guard_turns_before: u8,
    pub stat_drop_guard_turns_after: u8,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleRewards {
    pub outcome: BattleRewardOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTrainerBattleAdvance {
    pub next_enemy: Option<crystal_core::models::Pokemon>,
    pub trainer_defeated: bool,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFishingCast {
    pub session: FishingSession,
    pub bite: Option<bool>,
    pub wild_battle: Option<WildBattleStart>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFishingRodItemUse {
    pub item_use: ItemUseOutcome,
    pub rod: String,
    pub cast: RuntimeFishingCast,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTimeUpdate {
    pub time_of_day: TimeOfDay,
    pub day_of_week: u8,
    pub hour: u8,
    pub minute: u8,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGiftPokemonGrant {
    pub outcome: GiftPokemonOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFieldMoveBlockUse {
    pub outcome: FieldMoveBlockOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFieldMoveFlagUse {
    pub outcome: FieldMoveFlagOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFieldMoveTravelUse {
    pub outcome: FieldMoveTravelOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFlyFieldMoveUse {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub flypoint_flag: String,
    pub source_map: String,
    pub destination_spawn_identifier: u16,
    pub destination_map: String,
    pub destination_tile: TilePosition,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTeleportFieldMoveUse {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub source_map: String,
    pub destination_spawn_identifier: u16,
    pub destination_map: String,
    pub destination_tile: TilePosition,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFieldEncounterMoveUse {
    pub field_encounter: FieldEncounterRoll,
    pub wild_battle: Option<WildBattleStart>,
    pub removed_object_identifier: Option<String>,
    pub removed_event_flag: Option<String>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSweetScentFieldMoveUse {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub wild_encounter: Option<WildEncounterRoll>,
    pub wild_battle: Option<WildBattleStart>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemUse {
    pub outcome: ItemUseOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRegisteredKeyItem {
    pub outcome: RuntimeRegisteredKeyItemOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBattleItemUse {
    pub item_use: ItemUseOutcome,
    pub battle_item: BattleItemOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePartyItemUse {
    pub item_use: ItemUseOutcome,
    pub item_effect: BattleItemOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeWholePartyItemUse {
    pub item_use: ItemUseOutcome,
    pub item_effect: PartyItemOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTmHmItemUse {
    pub item_use: ItemUseOutcome,
    pub learned_move: TmHmLearnOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRepelItemUse {
    pub item_use: ItemUseOutcome,
    pub repel_steps_before: u16,
    pub repel_steps_after: u16,
    pub active_repel_item_before: Option<String>,
    pub active_repel_item_after: Option<String>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBicycleItemUse {
    pub item_use: ItemUseOutcome,
    pub map_name: String,
    pub permission: u8,
    pub mode_before: MovementMode,
    pub mode_after: MovementMode,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemfinderUse {
    pub item_use: ItemUseOutcome,
    pub player_tile: TilePosition,
    pub found: Option<CoreItemfinderHiddenItem>,
    pub itemfinder_sound_cues: usize,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSquirtBottleUse {
    pub item_use: ItemUseOutcome,
    pub player_tile: TilePosition,
    pub target_tile: TilePosition,
    pub target_object_identifier: Option<String>,
    pub target_movement: String,
    pub target_script: Option<String>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeKeyItemBalanceUse {
    pub item_use: ItemUseOutcome,
    pub balance_label: String,
    pub balance: u32,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTownMapUse {
    pub item_use: ItemUseOutcome,
    pub map_name: String,
    pub map_constant: String,
    pub environment: String,
    pub landmark: PokegearLandmark,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePokegearUse {
    pub item_use: ItemUseOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBoxItemUse {
    pub item_use: ItemUseOutcome,
    pub decoration_flag: String,
    pub already_owned: bool,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEscapeRopeUse {
    pub item_use: ItemUseOutcome,
    pub source_map: String,
    pub destination_map: String,
    pub destination_warp_index: u16,
    pub destination_tile: TilePosition,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDigFieldMoveUse {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub source_map: String,
    pub destination_map: String,
    pub destination_warp_index: u16,
    pub destination_tile: TilePosition,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptItemGrant {
    pub outcome: ScriptItemGrantOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptItemCheck {
    pub outcome: ScriptItemCheckOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptItemTake {
    pub outcome: ScriptItemTakeOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSpecialRoutineUse {
    pub outcome: SpecialRoutineOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFieldPickup {
    pub outcome: FieldItemPickupOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptEconomy {
    pub outcome: ScriptEconomyOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePhoneCommand {
    pub outcome: ScriptPhoneOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePermanentPhoneNumbers {
    pub inserted: Vec<String>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFlagMutation {
    pub outcome: ScriptFlagMutationOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFlagCheck {
    pub outcome: ScriptFlagCheckOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSceneCommand {
    pub outcome: ScriptSceneOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBlockChange {
    pub outcome: ScriptBlockChangeOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptAudio {
    pub cue: ScriptAudioCue,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptMapCommand {
    pub action: ScriptMapAction,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptWarp {
    pub target_map: String,
    pub tile: TilePosition,
    pub facing: Option<Direction>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptText {
    pub action: ScriptTextAction,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptVariable {
    pub outcome: ScriptVariableOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptSwarm {
    pub outcome: ScriptSwarmOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptControl {
    pub action: ScriptControlAction,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptObjectMutation {
    pub outcome: ScriptObjectMutationOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptMovement {
    pub outcome: ScriptMovementOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptRuntimeCommand {
    pub outcome: ScriptRuntimeOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeQueuedScriptCommand {
    pub queued: ScriptRuntimeQueuedCommand,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNextScript {
    pub origin_map_name: String,
    pub script: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptReturnResume {
    pub frame: ScriptReturnFrame,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDeferredScript {
    pub origin_map_name: String,
    pub script: String,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptEnd {
    pub end: ScriptEndState,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptShop {
    pub outcome: ScriptShopOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShopTransaction {
    pub outcome: ShopResult,
    pub state_checksum: StateChecksum,
}

impl CrystalRuntime {
    pub fn growth_rates(&self) -> &crystal_core::systems::experience::GrowthRateCatalog {
        &self.data.growth_rates
    }

    pub fn load_from_compiled_pack(
        asset_root: &AssetRoot,
        compiled_pack_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let loaded = asset_root.load_loaded_verified_compiled_game_pack(compiled_pack_path)?;
        Self::from_loaded_compiled_pack(asset_root, loaded)
    }

    pub fn from_loaded_compiled_pack(
        asset_root: &AssetRoot,
        loaded: LoadedCompiledGamePack,
    ) -> Result<Self> {
        crystal_assets::verify_compiled_game_pack_for_runtime(loaded.pack())?;
        let modpack = loaded
            .save_modpack_identity()
            .context("compute compiled game pack save identity")?;
        let (_, _, pack) = loaded.into_parts();
        Self::from_compiled_pack(asset_root, pack, modpack)
    }

    fn from_compiled_pack(
        asset_root: &AssetRoot,
        pack: CompiledGamePack,
        modpack: SaveModpackIdentity,
    ) -> Result<Self> {
        modpack.validate()?;
        let expected_id = pack.runtime_modpack_id()?;
        if modpack.id() != expected_id {
            anyhow::bail!(
                "compiled game pack identity '{}' does not match report manifest id '{}'",
                modpack.id(),
                expected_id
            );
        }
        crystal_assets::verify_compiled_game_pack_for_runtime(&pack)?;
        let pack_identity = pack.identity()?;
        let (
            _,
            data,
            compiled_audio,
            audio_manifest,
            audio_compression,
            runtime_files,
            _,
            stored_identity,
        ) = pack.into_parts();
        if pack_identity != stored_identity {
            anyhow::bail!("compiled game pack stored identity changed during runtime load");
        }
        let audio_manifest = if audio_compression.is_none()
            && audio_manifest.music.is_empty()
            && audio_manifest.sound_effects.is_empty()
            && audio_manifest.cries.is_empty()
        {
            ModpackAudioManifest::from_assets(&data.audio, &compiled_audio)?
        } else {
            audio_manifest
        };
        let audio_playback = ModpackAudioPlaybackPlan::from_manifest(&audio_manifest)?;
        // Every gameplay audio payload is compiled into the pack.  The
        // repository asset root remains relevant to build-time loading, but
        // is deliberately not consulted by the runtime so a release binary
        // plus pack is sufficient to play.
        let _ = asset_root;
        let audio = RuntimeAudioCatalog::from_game_data_owned(
            &data,
            compiled_audio,
            audio_manifest,
            audio_playback,
            audio_compression.as_deref(),
        )?;
        let map_catalog = Self::base_map_catalog_snapshot(&data);
        let runtime = Self {
            modpack,
            pack_identity,
            data,
            runtime_files,
            audio,
            viewport: GameViewport::default(),
            map_catalog,
            catalog_cache: Arc::new(OnceLock::new()),
        };
        // Build once while loading the pack. Every later presentation
        // snapshot shares these immutable catalogs in O(1).
        let _ = runtime
            .catalog_cache
            .set(runtime.build_static_catalog_cache());
        Ok(runtime)
    }

    pub fn modpack(&self) -> &SaveModpackIdentity {
        &self.modpack
    }

    pub fn data(&self) -> &GameDataSet {
        &self.data
    }

    pub fn audio(&self) -> &RuntimeAudioCatalog {
        &self.audio
    }

    pub fn runtime_file(&self, relative_path: &str) -> Option<&[u8]> {
        self.runtime_files.get(relative_path).map(Vec::as_slice)
    }

    pub fn has_runtime_files(&self) -> bool {
        !self.runtime_files.is_empty()
    }

    #[cfg(target_arch = "wasm32")]
    pub fn install_browser_runtime_files(&self) -> Result<()> {
        BROWSER_RUNTIME_FILES
            .set(self.runtime_files.clone())
            .map_err(|_| anyhow::anyhow!("browser runtime files were already installed"))
    }

    /// Materialize the embedded non-audio presentation bundle into an
    /// isolated runtime asset root.  The Bevy renderer still consumes the
    /// existing path-based loaders, so mounting the pack here lets those
    /// loaders work on a clean machine without the repository checkout.
    pub fn materialize_runtime_files(&self) -> Result<AssetRoot> {
        let mount = std::env::temp_dir().join(format!(
            "crystal-pack-assets-{}-{}",
            std::process::id(),
            self.pack_identity.content_hash
        ));
        validate_compiled_runtime_files(&self.runtime_files)?;
        let materialization_plan = self
            .runtime_files
            .iter()
            .map(|(relative, bytes)| {
                let path = if let Some(vendor_relative) = relative.strip_prefix("vendor/") {
                    mount.join("vendor").join(vendor_relative)
                } else {
                    mount.join("apps/web/assets").join(relative)
                };
                (path, bytes)
            })
            .collect::<Vec<_>>();
        let complete_marker = mount.join(".crystal-pack-assets-complete");
        if complete_marker.is_file() {
            return Ok(AssetRoot::new(mount));
        }
        for (path, bytes) in materialization_plan {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("create embedded runtime asset mount {}", parent.display())
                })?;
            }
            std::fs::write(&path, bytes).with_context(|| {
                format!("materialize embedded runtime asset {}", path.display())
            })?;
        }
        std::fs::write(&complete_marker, self.pack_identity.content_hash.as_bytes()).with_context(
            || format!("finalize embedded runtime asset mount {}", mount.display()),
        )?;
        Ok(AssetRoot::new(mount))
    }

    pub fn title_music_id(&self) -> Result<&str> {
        self.data
            .runtime_title_screen
            .title_music
            .as_deref()
            .context("compiled pack title screen missing title music")
    }

    pub fn title_new_game_spawn_identifier(&self) -> Result<u16> {
        self.data
            .runtime_title_screen
            .new_game_spawn_identifier
            .context("compiled pack title screen missing new-game spawn identifier")
    }

    pub fn special_routine_ids(&self) -> BTreeSet<String> {
        self.data.special_routines.keys().cloned().collect()
    }

    pub fn item_ids(&self) -> BTreeSet<String> {
        self.data.items.keys().cloned().collect()
    }

    pub fn move_ids(&self) -> BTreeSet<String> {
        self.data.moves.keys().cloned().collect()
    }

    pub fn species_ids(&self) -> BTreeSet<String> {
        self.data.pokemon.keys().cloned().collect()
    }

    pub fn map_ids(&self) -> BTreeSet<String> {
        self.data.maps.keys().cloned().collect()
    }

    pub fn trainer_ids(&self) -> BTreeSet<String> {
        self.data.trainers.trainers.keys().cloned().collect()
    }

    pub fn text_ids(&self) -> BTreeSet<String> {
        self.data
            .asm_text
            .keys()
            .cloned()
            .chain(
                self.data
                    .maps
                    .values()
                    .flat_map(|module| module.script_text_bodies.keys().cloned()),
            )
            .collect()
    }

    pub fn menu_ids(&self) -> BTreeSet<String> {
        self.data
            .special_routines
            .keys()
            .cloned()
            .chain(
                self.data
                    .maps
                    .values()
                    .flat_map(|module| module.script_menu_definitions.keys().cloned()),
            )
            .collect()
    }

    pub fn phone_contact_ids(&self) -> BTreeSet<String> {
        self.data.phone_contacts.0.keys().cloned().collect()
    }

    pub fn special_phone_call_ids(&self) -> BTreeSet<String> {
        self.data.special_phone_calls.keys().cloned().collect()
    }

    pub fn npc_trade_ids(&self) -> BTreeSet<String> {
        self.data.npc_trades.keys().cloned().collect()
    }

    pub fn sprite_ids(&self) -> BTreeSet<String> {
        self.data.sprite_palette_defaults.keys().cloned().collect()
    }

    pub fn map_constants(&self) -> BTreeSet<String> {
        self.data.runtime_map_metadata.keys().cloned().collect()
    }

    pub fn event_flag_ids(&self) -> BTreeSet<String> {
        let mut ids: BTreeSet<String> = self
            .data
            .initialize_events
            .event_flags
            .iter()
            .cloned()
            .collect();
        ids.extend(
            self.data
                .story_event_script_constants
                .global
                .keys()
                .cloned(),
        );
        ids.extend(
            self.data
                .story_event_script_constants
                .maps
                .values()
                .flat_map(|constants| constants.keys().cloned()),
        );
        if let Some(config) = &self.data.bug_contest_config {
            ids.extend(config.contestant_flags.iter().cloned());
        }
        for module in self.data.maps.values() {
            ids.extend(
                module
                    .script_flag_commands
                    .iter()
                    .filter(|command| !crystal_core::state::is_engine_flag_name(&command.flag_id))
                    .map(|command| command.flag_id.clone()),
            );
        }
        ids
    }

    pub fn engine_flag_ids(&self) -> BTreeSet<String> {
        let mut ids: BTreeSet<String> = self
            .data
            .initialize_events
            .engine_flags
            .iter()
            .cloned()
            .collect();
        ids.extend(
            self.data
                .story_event_script_constants
                .global
                .keys()
                .cloned(),
        );
        ids.extend(
            self.data
                .story_event_script_constants
                .maps
                .values()
                .flat_map(|constants| constants.keys().cloned()),
        );
        for module in self.data.maps.values() {
            ids.extend(
                module
                    .script_flag_commands
                    .iter()
                    .filter(|command| crystal_core::state::is_engine_flag_name(&command.flag_id))
                    .map(|command| command.flag_id.clone()),
            );
        }
        ids
    }

    pub fn spawn_identifiers(&self) -> BTreeSet<u16> {
        self.data
            .runtime_spawn_points
            .values()
            .map(|spawn| spawn.identifier)
            .collect()
    }

    pub fn tileset_ids(&self) -> BTreeSet<String> {
        self.data.tilesets.keys().cloned().collect()
    }

    pub fn tileset_keys(&self) -> BTreeSet<RuntimeTilesetKey> {
        self.data
            .tilesets
            .iter()
            .map(|(tileset_id, tileset)| RuntimeTilesetKey {
                tileset_id: tileset_id.clone(),
                collision: tileset.collision.clone(),
                palette_map: tileset.palette_map.clone(),
            })
            .collect()
    }

    pub fn pc_string_keys(&self) -> BTreeSet<RuntimePcStringKey> {
        self.data
            .pc_strings
            .iter()
            .map(|(string_id, text)| RuntimePcStringKey {
                string_id: string_id.clone(),
                text: text.clone(),
            })
            .collect()
    }

    pub fn menu_icon_keys(&self) -> BTreeSet<RuntimeMenuIconKey> {
        self.data
            .menu_icons
            .iter()
            .map(|(species_id, icon_id)| RuntimeMenuIconKey {
                species_id: species_id.clone(),
                icon_id: icon_id.clone(),
            })
            .collect()
    }

    pub fn pokedex_entry_keys(&self) -> BTreeSet<RuntimePokedexEntryKey> {
        self.data
            .pokedex_entries
            .iter()
            .map(|(species_id, entry)| RuntimePokedexEntryKey {
                species_id: species_id.clone(),
                species: entry.species.clone(),
                classification: entry.classification.clone(),
                height_digits: entry.height_digits,
                weight_digits: entry.weight_digits,
                pages: entry.pages.clone(),
            })
            .collect()
    }

    pub fn landmark_ids(&self) -> BTreeSet<String> {
        self.data
            .pokegear_landmarks
            .landmarks
            .iter()
            .map(|landmark| landmark.constant.clone())
            .collect()
    }

    pub fn pokegear_landmark_keys(&self) -> BTreeSet<RuntimePokegearLandmarkKey> {
        self.data
            .pokegear_landmarks
            .landmarks
            .iter()
            .map(|landmark| RuntimePokegearLandmarkKey {
                landmark_id: landmark.id,
                constant: landmark.constant.clone(),
                label: landmark.label.clone(),
                name: landmark.name.clone(),
                x: landmark.x,
                y: landmark.y,
                region: landmark.region.clone(),
            })
            .collect()
    }

    pub fn pokegear_map_landmark_keys(&self) -> BTreeSet<RuntimePokegearMapLandmarkKey> {
        self.data
            .pokegear_landmarks
            .map_to_landmark
            .iter()
            .map(
                |(map_name, landmark_constant)| RuntimePokegearMapLandmarkKey {
                    map_name: map_name.clone(),
                    landmark_constant: landmark_constant.clone(),
                },
            )
            .collect()
    }

    pub fn fishing_rod_ids(&self) -> BTreeSet<String> {
        self.data
            .fishing
            .groups
            .values()
            .flat_map(|group| group.rod_tables.keys().cloned())
            .collect()
    }

    pub fn map_group_ids(&self) -> BTreeSet<String> {
        self.data
            .runtime_map_metadata
            .values()
            .map(|metadata| metadata.group_name.clone())
            .collect()
    }

    pub fn encounter_group_ids(&self) -> BTreeSet<String> {
        self.data.fishing.groups.keys().cloned().collect()
    }

    pub fn mart_ids(&self) -> BTreeSet<String> {
        self.data.marts.0.keys().cloned().collect()
    }

    pub fn mart_keys(&self) -> BTreeSet<RuntimeMartKey> {
        self.data
            .marts
            .0
            .iter()
            .map(|(mart_id, item_ids)| RuntimeMartKey {
                mart_id: mart_id.clone(),
                item_ids: item_ids.clone(),
            })
            .collect()
    }

    pub fn fruit_tree_ids(&self) -> BTreeSet<String> {
        self.data.fruit_trees.0.keys().cloned().collect()
    }

    pub fn fruit_tree_keys(&self) -> BTreeSet<RuntimeFruitTreeKey> {
        self.data
            .fruit_trees
            .0
            .iter()
            .map(|(fruit_tree_id, item_id)| RuntimeFruitTreeKey {
                fruit_tree_id: fruit_tree_id.clone(),
                item_id: item_id.clone(),
            })
            .collect()
    }

    pub fn field_move_rule_ids(&self) -> BTreeSet<String> {
        [
            "cut",
            "whirlpool",
            "strength",
            "flash",
            "surf",
            "waterfall",
            "fly",
            "dig",
            "teleport",
            "escape_rope",
            "repel",
            "bicycle",
            "itemfinder",
            "squirtbottle",
            "coin_case",
            "blue_card",
            "town_map",
            "pokegear",
        ]
        .into_iter()
        .map(str::to_string)
        .collect()
    }

    pub fn field_move_rule_keys(&self) -> BTreeSet<RuntimeFieldMoveRuleKey> {
        field_move_rule_keys(&self.data.field_moves)
    }

    pub fn fly_destination_ids(&self) -> BTreeSet<String> {
        self.data.fly_destinations.keys().cloned().collect()
    }

    pub fn fly_destination_keys(&self) -> BTreeSet<RuntimeFlyDestinationKey> {
        fly_destination_keys(&self.data.fly_destinations)
    }

    pub fn field_move_move_ids(&self) -> BTreeSet<String> {
        field_move_move_ids(&self.data.field_moves)
    }

    pub fn field_move_item_ids(&self) -> BTreeSet<String> {
        field_move_item_ids(&self.data.field_moves)
    }

    pub fn field_box_item_ids(&self) -> BTreeSet<String> {
        self.data.field_box_items.keys().cloned().collect()
    }

    pub fn flee_mon_bucket_ids(&self) -> BTreeSet<String> {
        self.data.flee_mons.buckets.keys().cloned().collect()
    }

    pub fn buena_password_category_ids(&self) -> BTreeSet<String> {
        self.data
            .buena_password_categories
            .categories
            .keys()
            .cloned()
            .collect()
    }

    pub fn roaming_species_ids(&self) -> BTreeSet<String> {
        self.data
            .roaming_pokemon
            .init_writes
            .iter()
            .map(|write| write.species.clone())
            .collect()
    }

    pub fn buena_prize_item_ids(&self) -> BTreeSet<String> {
        self.data.buena_prizes.keys().cloned().collect()
    }

    pub fn kurt_apricorn_item_ids(&self) -> BTreeSet<String> {
        self.data.kurt_apricorn_recipes.keys().cloned().collect()
    }

    pub fn dratini_move_set_ids(&self) -> BTreeSet<u8> {
        self.data.dratini_move_sets.keys().copied().collect()
    }

    pub fn special_feature_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        if self.data.shuckie_gift.is_some() {
            ids.insert("shuckie_gift".to_string());
        }
        if self.data.bug_contest_config.is_some() {
            ids.insert("bug_contest".to_string());
        }
        if self.data.battle_tower_rules.is_some() {
            ids.insert("battle_tower".to_string());
        }
        if !self.data.oak_ratings.is_empty() {
            ids.insert("oak_ratings".to_string());
        }
        if !self.data.odd_egg_definitions.is_empty() {
            ids.insert("odd_egg".to_string());
        }
        if !self.data.magikarp_lengths.is_empty() {
            ids.insert("magikarp_lengths".to_string());
        }
        if self.data.happiness_data.is_some() {
            ids.insert("happiness".to_string());
        }
        ids
    }

    pub fn oak_rating_text_ids(&self) -> BTreeSet<String> {
        self.data
            .oak_ratings
            .iter()
            .map(|rating| rating.text_label.clone())
            .collect()
    }

    pub fn odd_egg_species_ids(&self) -> BTreeSet<String> {
        self.data
            .odd_egg_definitions
            .iter()
            .map(|definition| definition.species.clone())
            .collect()
    }

    pub fn magikarp_length_thresholds(&self) -> BTreeSet<u16> {
        self.data
            .magikarp_lengths
            .iter()
            .map(|entry| entry.threshold)
            .collect()
    }

    pub fn happiness_change_ids(&self) -> BTreeSet<u8> {
        self.data
            .happiness_data
            .as_ref()
            .map(|data| data.changes.keys().copied().collect())
            .unwrap_or_default()
    }

    pub fn happiness_service_ids(&self) -> BTreeSet<String> {
        self.data
            .happiness_data
            .as_ref()
            .map(|data| data.services.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn pokemon_status_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::from([
            self.data.step_event_rules.poison_status.clone(),
            "POKERUS".to_string(),
        ]);
        ids.extend(self.data.capture_rules.status_bonus.keys().cloned());
        ids.extend(
            self.data
                .items
                .values()
                .flat_map(|item| item.status_heals.iter().cloned()),
        );
        ids.retain(|status| !status.is_empty());
        ids
    }

    pub fn fishing_daily_flag_bits(&self) -> BTreeSet<u32> {
        self.data
            .fishing
            .swarm_rules
            .values()
            .map(|rule| u32::from(rule.daily_flag_bit))
            .collect()
    }

    pub fn fishing_swarm_flags(&self) -> BTreeSet<u8> {
        self.data
            .fishing
            .swarm_rules
            .values()
            .map(|rule| rule.swarm)
            .collect()
    }

    pub fn pending_special_battle_type_ids(&self) -> BTreeSet<String> {
        let mut ids = BTreeSet::new();
        for module in self.data.maps.values() {
            ids.extend(
                module
                    .scripted_trainer_battles
                    .iter()
                    .filter(|battle| !battle.request.battle_type.is_empty())
                    .map(|battle| battle.request.battle_type.clone()),
            );
            ids.extend(
                module
                    .scripted_wild_battles
                    .iter()
                    .filter(|battle| !battle.request.battle_type.is_empty())
                    .map(|battle| battle.request.battle_type.clone()),
            );
        }
        ids.extend(
            saved_special_battle_type_builtin_routines()
                .iter()
                .filter(|(_, routine)| self.data.special_routines.contains_key(*routine))
                .map(|(battle_type, _)| (*battle_type).to_string()),
        );
        ids
    }

    pub fn scripted_trainer_battle_keys(&self) -> BTreeSet<RuntimeScriptedTrainerBattleKey> {
        let mut keys = BTreeSet::new();
        for (map_name, module) in &self.data.maps {
            keys.extend(module.scripted_trainer_battles.iter().map(|battle| {
                RuntimeScriptedTrainerBattleKey {
                    map_name: map_name.clone(),
                    source_script: battle.source_script.clone(),
                    loadtrainer_command_index: battle.loadtrainer_command_index,
                    startbattle_command_index: battle.startbattle_command_index,
                    battle_type: battle.request.battle_type.clone(),
                    trainer_class: battle.request.trainer_class.clone(),
                    trainer_id: battle.request.trainer_id.clone(),
                }
            }));
            for (source_script, request) in &module.trainer_scripts {
                let Some(command_index) = module
                    .scripts
                    .get(source_script)
                    .and_then(serde_json::Value::as_array)
                    .and_then(|commands| {
                        commands.iter().position(|command| {
                            command.get("command").and_then(serde_json::Value::as_str)
                                == Some("trainer")
                        })
                    })
                else {
                    continue;
                };
                keys.insert(RuntimeScriptedTrainerBattleKey {
                    map_name: map_name.clone(),
                    source_script: source_script.clone(),
                    loadtrainer_command_index: command_index,
                    startbattle_command_index: command_index,
                    battle_type: request.battle_type.clone(),
                    trainer_class: request.trainer_class.clone(),
                    trainer_id: request.trainer_id.clone(),
                });
            }
        }
        keys
    }

    pub fn wild_encounter_origin_keys(&self) -> BTreeSet<RuntimeWildEncounterOriginKey> {
        let mut keys = BTreeSet::new();
        for (map_name, encounters) in &self.data.wild_encounters {
            collect_wild_encounter_keys(map_name, encounters, &mut keys);
        }
        for (map_name, encounters) in &self.data.field_encounters {
            collect_field_encounter_keys(map_name, encounters, &mut keys);
        }
        for (map_name, module) in &self.data.maps {
            let Some(group_name) = module.attributes.fishing_group.as_deref() else {
                continue;
            };
            let Some(group) = self.data.fishing.groups.get(group_name) else {
                continue;
            };
            collect_fishing_encounter_keys(
                map_name,
                group,
                &self.data.fishing.time_groups,
                &mut keys,
            );
        }
        keys
    }

    pub fn script_label_ids(&self) -> BTreeSet<String> {
        self.data
            .maps
            .values()
            .flat_map(|module| module.scripts.keys().cloned())
            .collect()
    }

    pub fn script_command_keys(&self) -> BTreeSet<RuntimeScriptCommandKey> {
        let mut keys = BTreeSet::new();
        for (script_label, body) in self
            .data
            .maps
            .values()
            .flat_map(|module| module.scripts.iter())
        {
            if let Some(commands) = body.as_array() {
                keys.extend(commands.iter().enumerate().map(|(command_index, _)| {
                    RuntimeScriptCommandKey {
                        script_label: script_label.clone(),
                        command_index,
                    }
                }));
            }
        }
        keys
    }

    pub fn script_command_payload_keys(&self) -> BTreeSet<RuntimeScriptCommandPayloadKey> {
        let mut keys = BTreeSet::new();
        for (script_label, body) in self
            .data
            .maps
            .values()
            .flat_map(|module| module.scripts.iter())
        {
            if let Some(commands) = body.as_array() {
                keys.extend(
                    commands
                        .iter()
                        .enumerate()
                        .filter_map(|(command_index, command)| {
                            let command_name = command.get("command")?.as_str()?;
                            let args = command
                                .get("args")
                                .and_then(|args| args.as_array())
                                .map(|args| {
                                    args.iter()
                                        .filter_map(|arg| arg.as_str().map(str::to_string))
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            Some(RuntimeScriptCommandPayloadKey {
                                script_label: script_label.clone(),
                                command_index,
                                command: command_name.to_string(),
                                args,
                            })
                        }),
                );
            }
        }
        keys
    }

    pub fn script_return_keys(&self) -> BTreeSet<RuntimeScriptReturnKey> {
        let mut keys = BTreeSet::new();
        for (script_label, body) in self
            .data
            .maps
            .values()
            .flat_map(|module| module.scripts.iter())
        {
            if let Some(commands) = body.as_array() {
                keys.extend((0..=commands.len()).map(|next_command_index| {
                    RuntimeScriptReturnKey {
                        script_label: script_label.clone(),
                        next_command_index,
                    }
                }));
            }
        }
        keys
    }

    pub fn script_vertical_menu_keys(&self) -> BTreeSet<RuntimeScriptVerticalMenuKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_vertical_menus
                    .iter()
                    .map(move |(menu_key, menu)| RuntimeScriptVerticalMenuKey {
                        map_name: map_name.clone(),
                        menu_key: menu_key.clone(),
                        source_script: menu.source_script.clone(),
                        loadmenu_command_index: menu.loadmenu_command_index,
                        verticalmenu_command_index: menu.verticalmenu_command_index,
                        header_label: menu.header_label.clone(),
                        data_label: menu.data_label.clone(),
                        options: menu.options.clone(),
                    })
            })
            .collect()
    }

    pub fn script_text_body_keys(&self) -> BTreeSet<RuntimeScriptTextBodyKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_text_bodies
                    .iter()
                    .map(move |(body_key, body)| RuntimeScriptTextBodyKey {
                        map_name: map_name.clone(),
                        body_key: body_key.clone(),
                        label: body.label.clone(),
                        commands: body
                            .commands
                            .iter()
                            .map(|command| RuntimeScriptTextBodyCommandKey {
                                command: command.command.clone(),
                                args: command.args.clone(),
                                command_index: command.command_index,
                            })
                            .collect(),
                    })
            })
            .collect()
    }

    pub fn script_menu_definition_keys(&self) -> BTreeSet<RuntimeScriptMenuDefinitionKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_menu_definitions
                    .iter()
                    .map(
                        move |(menu_key, definition)| RuntimeScriptMenuDefinitionKey {
                            map_name: map_name.clone(),
                            menu_key: menu_key.clone(),
                            label: definition.label.clone(),
                            commands: definition
                                .commands
                                .iter()
                                .map(|command| RuntimeScriptMenuCommandKey {
                                    command: command.command.clone(),
                                    args: command.args.clone(),
                                    command_index: command.command_index,
                                })
                                .collect(),
                        },
                    )
            })
            .collect()
    }

    pub fn script_elevator_keys(&self) -> BTreeSet<RuntimeScriptElevatorKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_elevators
                    .iter()
                    .map(move |(elevator_key, elevator)| RuntimeScriptElevatorKey {
                        map_name: map_name.clone(),
                        elevator_key: elevator_key.clone(),
                        source_script: elevator.source_script.clone(),
                        elevator_command_index: elevator.elevator_command_index,
                        data_label: elevator.data_label.clone(),
                        floors: elevator
                            .floors
                            .iter()
                            .map(|floor| RuntimeScriptElevatorFloorKey {
                                floor: floor.floor.clone(),
                                warp: floor.warp,
                                target_map: floor.target_map.clone(),
                                source_script: floor.source_script.clone(),
                                command_index: floor.command_index,
                            })
                            .collect(),
                    })
            })
            .collect()
    }

    pub fn gift_pokemon_keys(&self) -> BTreeSet<RuntimeGiftPokemonKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .gift_pokemon_scripts
                    .iter()
                    .map(move |gift| RuntimeGiftPokemonKey {
                        map_name: map_name.clone(),
                        species_id: gift.species_id.clone(),
                        level_token: gift.level_token.clone(),
                        level: gift.level,
                        held_item_id: gift.held_item_id.clone(),
                        nickname_label: gift.nickname_label.clone(),
                        ot_label: gift.ot_label.clone(),
                        source_script: gift.source_script.clone(),
                        command_index: gift.command_index,
                        egg: gift.egg,
                    })
            })
            .collect()
    }

    pub fn script_object_command_keys(&self) -> BTreeSet<RuntimeScriptObjectCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_object_commands.iter().map(move |command| {
                    RuntimeScriptObjectCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        object_id: command.object_id.clone(),
                        target_object_id: command.target_object_id.clone(),
                        x: command.x,
                        y: command.y,
                        direction: command.direction.clone(),
                        movement: command.movement.clone(),
                        emote: command.emote.clone(),
                        duration: command.duration,
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_movement_keys(&self) -> BTreeSet<RuntimeScriptMovementKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_movements
                    .iter()
                    .map(move |movement| RuntimeScriptMovementKey {
                        map_name: map_name.clone(),
                        label: movement.label.clone(),
                        source_script: movement.source_script.clone(),
                        steps: movement
                            .steps
                            .iter()
                            .map(|step| RuntimeScriptMovementStepKey {
                                command: step.command.clone(),
                                direction: step.direction.clone(),
                                duration: step.duration,
                                index: step.index,
                            })
                            .collect(),
                    })
            })
            .collect()
    }

    pub fn map_script_section_command_keys(&self) -> BTreeSet<RuntimeMapScriptSectionCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .map_script_section_commands
                    .iter()
                    .map(move |command| RuntimeMapScriptSectionCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        args: command.args.clone(),
                        command_index: command.command_index,
                    })
            })
            .collect()
    }

    pub fn map_event_section_command_keys(&self) -> BTreeSet<RuntimeMapEventSectionCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .map_event_section_commands
                    .iter()
                    .map(move |command| RuntimeMapEventSectionCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        args: command.args.clone(),
                        command_index: command.command_index,
                    })
            })
            .collect()
    }

    pub fn script_map_command_keys(&self) -> BTreeSet<RuntimeScriptMapCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_map_commands
                    .iter()
                    .map(move |command| RuntimeScriptMapCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        target_map: command.target_map.clone(),
                        x: command.x,
                        y: command.y,
                        facing: command.facing.clone(),
                        map_setup: command.map_setup.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    })
            })
            .collect()
    }

    pub fn script_variable_command_keys(&self) -> BTreeSet<RuntimeScriptVariableCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_variable_commands.iter().map(move |command| {
                    RuntimeScriptVariableCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        target: command.target.clone(),
                        value_tokens: command.value_tokens.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_control_command_keys(&self) -> BTreeSet<RuntimeScriptControlCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_control_commands.iter().map(move |command| {
                    RuntimeScriptControlCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        compare_value: command.compare_value.clone(),
                        target_label: command.target_label.clone(),
                        resolved_target_script: command.resolved_target_script.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_swarm_command_keys(&self) -> BTreeSet<RuntimeScriptSwarmCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_swarm_commands.iter().map(move |command| {
                    RuntimeScriptSwarmCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        swarm_token: command.swarm_token.clone(),
                        map_id: command.map_id.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_field_pickup_keys(&self) -> BTreeSet<RuntimeScriptFieldPickupKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_field_pickups
                    .iter()
                    .map(move |pickup| RuntimeScriptFieldPickupKey {
                        map_name: map_name.clone(),
                        command: pickup.command.clone(),
                        item_id: pickup.item_id.clone(),
                        quantity: pickup.quantity,
                        event_flag: pickup.event_flag.clone(),
                        fruit_tree_id: pickup.fruit_tree_id.clone(),
                        source_script: pickup.source_script.clone(),
                        command_index: pickup.command_index,
                    })
            })
            .collect()
    }

    pub fn script_shop_command_keys(&self) -> BTreeSet<RuntimeScriptShopCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_shop_commands
                    .iter()
                    .map(move |command| RuntimeScriptShopCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        mart_type: command.mart_type.clone(),
                        mart_id: command.mart_id.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    })
            })
            .collect()
    }

    pub fn script_phone_command_keys(&self) -> BTreeSet<RuntimeScriptPhoneCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_phone_commands.iter().map(move |command| {
                    RuntimeScriptPhoneCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        contact_id: command.contact_id.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_runtime_command_keys(&self) -> BTreeSet<RuntimeScriptRuntimeCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_runtime_commands.iter().map(move |command| {
                    RuntimeScriptRuntimeCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        args: command.args.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_runtime_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Option<RuntimeScriptRuntimeCommandKey> {
        self.data
            .script_runtime_command(map_name, source_script, command_index)
            .ok()
            .map(|command| RuntimeScriptRuntimeCommandKey {
                map_name: map_name.to_string(),
                command: command.command.clone(),
                args: command.args.clone(),
                source_script: command.source_script.clone(),
                command_index: command.command_index,
            })
    }

    pub fn script_item_grant_keys(&self) -> BTreeSet<RuntimeScriptItemGrantKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_item_grants
                    .iter()
                    .map(move |grant| RuntimeScriptItemGrantKey {
                        map_name: map_name.clone(),
                        command: grant.command.clone(),
                        item_id: grant.item_id.clone(),
                        quantity: grant.quantity,
                        source_script: grant.source_script.clone(),
                        command_index: grant.command_index,
                        verbose: grant.verbose,
                    })
            })
            .collect()
    }

    pub fn script_item_access_keys(&self) -> BTreeSet<RuntimeScriptItemAccessKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                let check_map_name = map_name.clone();
                let take_map_name = map_name.clone();
                let checks = module.script_item_checks.iter().map(move |access| {
                    RuntimeScriptItemAccessKey {
                        map_name: check_map_name.clone(),
                        command: access.command.clone(),
                        item_id: access.item_id.clone(),
                        source_script: access.source_script.clone(),
                        command_index: access.command_index,
                    }
                });
                let takes =
                    module
                        .script_item_takes
                        .iter()
                        .map(move |access| RuntimeScriptItemAccessKey {
                            map_name: take_map_name.clone(),
                            command: access.command.clone(),
                            item_id: access.item_id.clone(),
                            source_script: access.source_script.clone(),
                            command_index: access.command_index,
                        });
                checks.chain(takes)
            })
            .collect()
    }

    pub fn script_economy_command_keys(&self) -> BTreeSet<RuntimeScriptEconomyCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_economy_commands.iter().map(move |command| {
                    RuntimeScriptEconomyCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        account: command.account.clone(),
                        amount_tokens: command.amount_tokens.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_flag_command_keys(&self) -> BTreeSet<RuntimeScriptFlagCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_flag_commands
                    .iter()
                    .map(move |command| RuntimeScriptFlagCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        flag_id: command.flag_id.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    })
            })
            .collect()
    }

    pub fn script_scene_command_keys(&self) -> BTreeSet<RuntimeScriptSceneCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_scene_commands.iter().map(move |command| {
                    RuntimeScriptSceneCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        map_id: command.map_id.clone(),
                        scene_id: command.scene_id.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_block_change_keys(&self) -> BTreeSet<RuntimeScriptBlockChangeKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_block_changes
                    .iter()
                    .map(move |change| RuntimeScriptBlockChangeKey {
                        map_name: map_name.clone(),
                        x: change.x,
                        y: change.y,
                        block_id: change.block_id,
                        source_script: change.source_script.clone(),
                        command_index: change.command_index,
                    })
            })
            .collect()
    }

    pub fn script_audio_command_keys(&self) -> BTreeSet<RuntimeScriptAudioCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.script_audio_commands.iter().map(move |command| {
                    RuntimeScriptAudioCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        audio_id: command.audio_id.clone(),
                        fade_frames: command.fade_frames,
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    }
                })
            })
            .collect()
    }

    pub fn script_text_command_keys(&self) -> BTreeSet<RuntimeScriptTextCommandKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .script_text_commands
                    .iter()
                    .map(move |command| RuntimeScriptTextCommandKey {
                        map_name: map_name.clone(),
                        command: command.command.clone(),
                        text_label: command.text_label.clone(),
                        source_script: command.source_script.clone(),
                        command_index: command.command_index,
                    })
            })
            .collect()
    }

    pub fn warp_keys(&self) -> BTreeSet<RuntimeWarpKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.events.warps.iter().map(move |warp| RuntimeWarpKey {
                    map_name: map_name.clone(),
                    warp_index: warp.index,
                })
            })
            .collect()
    }

    pub fn map_object_keys(&self) -> BTreeSet<RuntimeMapObjectKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module.objects.iter().filter_map(move |object| {
                    object
                        .object_identifier
                        .as_ref()
                        .map(|object_id| RuntimeMapObjectKey {
                            map_name: map_name.clone(),
                            object_id: object_id.clone(),
                        })
                })
            })
            .collect()
    }

    pub fn map_scene_keys(&self) -> BTreeSet<RuntimeMapSceneKey> {
        self.data
            .maps
            .iter()
            .flat_map(|(map_name, module)| {
                module
                    .scenes
                    .scenes
                    .iter()
                    .map(move |scene| RuntimeMapSceneKey {
                        map_name: map_name.clone(),
                        scene_id: scene.scene_id.clone(),
                    })
            })
            .collect()
    }

    pub fn map_metadata_keys(&self) -> BTreeSet<RuntimeMapMetadataKey> {
        self.data
            .maps
            .iter()
            .map(|(map_name, module)| {
                let metadata = module
                    .attributes
                    .map_constant
                    .as_ref()
                    .and_then(|constant| self.data.runtime_map_metadata.get(constant));
                RuntimeMapMetadataKey {
                    map_name: map_name.clone(),
                    map_id: module.id.clone(),
                    tileset_name: module.attributes.tileset_name.clone(),
                    border_block: module.attributes.border_block,
                    width: module.attributes.width,
                    height: module.attributes.height,
                    time_of_day: module.attributes.time_of_day.clone(),
                    phone_service: module.attributes.phone_service,
                    phone_flag: module.attributes.phone_flag,
                    environment: module.attributes.environment.clone(),
                    location: module.attributes.location.clone(),
                    music: module.attributes.music.clone(),
                    palette: module.attributes.palette.clone(),
                    fishing_group: module.attributes.fishing_group.clone(),
                    map_constant: module.attributes.map_constant.clone(),
                    map_group_constant: module.attributes.map_group_constant.clone(),
                    metadata_constant: metadata.map(|entry| entry.constant.clone()),
                    metadata_group_name: metadata.map(|entry| entry.group_name.clone()),
                    metadata_group_id: metadata.map(|entry| entry.group_id),
                    metadata_map_id: metadata.map(|entry| entry.map_id),
                    metadata_environment: metadata.map(|entry| entry.environment.clone()),
                }
            })
            .collect()
    }

    pub fn currency_constant_ids(&self) -> BTreeSet<String> {
        self.data.currency_constants.0.keys().cloned().collect()
    }

    pub fn capture_ball_rule_ids(&self) -> BTreeSet<String> {
        self.data.capture_rules.ball_rules.keys().cloned().collect()
    }

    pub fn guaranteed_capture_ball_ids(&self) -> BTreeSet<String> {
        self.data
            .capture_rules
            .guaranteed_capture_balls
            .iter()
            .cloned()
            .collect()
    }

    pub fn capture_status_bonus_ids(&self) -> BTreeSet<String> {
        self.data
            .capture_rules
            .status_bonus
            .keys()
            .cloned()
            .collect()
    }

    pub fn fast_ball_species_ids(&self) -> BTreeSet<String> {
        self.data
            .capture_rules
            .fast_ball_species
            .iter()
            .cloned()
            .collect()
    }

    pub fn heavy_ball_species_ids(&self) -> BTreeSet<String> {
        self.data
            .capture_rules
            .heavy_ball_modifiers
            .keys()
            .cloned()
            .collect()
    }

    pub fn move_priority_effect_ids(&self) -> BTreeSet<String> {
        self.data
            .move_priorities
            .effect_priorities
            .keys()
            .cloned()
            .collect()
    }

    pub fn move_priority_move_ids(&self) -> BTreeSet<String> {
        self.data
            .move_priorities
            .move_priorities
            .iter()
            .map(|priority| priority.r#move.clone())
            .collect()
    }

    pub fn capture_ball_rule_keys(&self) -> BTreeSet<RuntimeCaptureBallRuleKey> {
        self.data
            .capture_rules
            .ball_rules
            .iter()
            .map(|(ball_id, rule)| RuntimeCaptureBallRuleKey {
                ball_id: ball_id.clone(),
                multiplier_numerator: rule.multiplier_numerator,
                multiplier_denominator: rule.multiplier_denominator,
                battle_type: rule.battle_type.clone(),
                skip_hp_calc: rule.skip_hp_calc,
                use_heavy_ball_weight_modifier: rule.use_heavy_ball_weight_modifier,
                use_level_ball_multiplier: rule.use_level_ball_multiplier,
                require_same_species: rule.require_same_species,
                require_same_gender: rule.require_same_gender,
                require_fast_species: rule.require_fast_species,
            })
            .collect()
    }

    pub fn heavy_ball_modifier_keys(&self) -> BTreeSet<RuntimeHeavyBallModifierKey> {
        self.data
            .capture_rules
            .heavy_ball_modifiers
            .iter()
            .map(|(species_id, modifier)| RuntimeHeavyBallModifierKey {
                species_id: species_id.clone(),
                modifier: *modifier,
            })
            .collect()
    }

    pub fn capture_status_bonus_keys(&self) -> BTreeSet<RuntimeCaptureStatusBonusKey> {
        self.data
            .capture_rules
            .status_bonus
            .iter()
            .map(|(status, bonus)| RuntimeCaptureStatusBonusKey {
                status: status.clone(),
                bonus: *bonus,
            })
            .collect()
    }

    pub fn capture_wobble_probability_keys(&self) -> BTreeSet<RuntimeCaptureWobbleProbabilityKey> {
        self.data
            .capture_wobble_probabilities
            .iter()
            .map(|probability| RuntimeCaptureWobbleProbabilityKey {
                catch_rate: probability.catch_rate,
                chance: probability.chance,
            })
            .collect()
    }

    pub fn item_battle_use_keys(&self) -> BTreeSet<RuntimeItemBattleUseKey> {
        self.data
            .items
            .iter()
            .map(|(item_id, item)| RuntimeItemBattleUseKey {
                item_id: item_id.clone(),
                effect: item.effect.clone(),
                battle_menu: item.battle_menu.clone(),
                battle_usable: item.battle_usable,
                battle_stat_boost_stat: item.battle_stat_boost_stat.clone(),
                battle_stat_boost_stages: item.battle_stat_boost_stages,
                battle_escape_mode: item.battle_escape_mode.clone(),
                battle_focus_energy: item.battle_focus_energy,
                battle_stat_drop_guard: item.battle_stat_drop_guard,
                battle_stat_drop_guard_turns: item.battle_stat_drop_guard_turns,
            })
            .collect()
    }

    pub fn item_effect_plan_keys(&self) -> BTreeSet<RuntimeItemEffectPlanKey> {
        self.data
            .items
            .values()
            .flat_map(|item| {
                [
                    active_battle_item_effect_plan(item),
                    battle_pp_item_effect_plan(item),
                    party_wide_item_effect_plan(item),
                    party_special_item_effect_plan(item, &self.data.evolutions),
                ]
            })
            .flatten()
            .map(RuntimeItemEffectPlanKey::from_plan)
            .collect()
    }

    pub fn item_field_use_keys(&self) -> BTreeSet<RuntimeItemFieldUseKey> {
        self.data
            .items
            .iter()
            .map(|(item_id, item)| RuntimeItemFieldUseKey {
                item_id: item_id.clone(),
                effect: item.effect.clone(),
                field_menu: item.field_menu.clone(),
                field_usable: item.field_usable,
                consumable: item.consumable,
                repel_steps: item.repel_steps,
                escape_rope_mode: item.escape_rope_mode.clone(),
                tmhm_index: item.tmhm_index,
                tmhm_move: item.tmhm_move.clone(),
            })
            .collect()
    }

    pub fn move_battle_data_keys(&self) -> BTreeSet<RuntimeMoveBattleDataKey> {
        self.data
            .moves
            .iter()
            .map(|(move_id, move_data)| RuntimeMoveBattleDataKey {
                move_id: move_id.clone(),
                name: move_data.name.clone(),
                move_type: move_data.move_type.clone(),
                power: move_data.power,
                accuracy: move_data.accuracy,
                pp: move_data.pp,
                effect: move_data.effect.clone(),
                effect_chance: move_data.effect_chance,
                stat: move_data.stat.clone(),
                amount: move_data.amount,
            })
            .collect()
    }

    pub fn species_battle_data_keys(&self) -> BTreeSet<RuntimeSpeciesBattleDataKey> {
        self.data
            .pokemon
            .iter()
            .map(|(species_id, species)| RuntimeSpeciesBattleDataKey {
                species_id: species_id.clone(),
                int_id: species.int_id,
                base_hp: species.base_stats.hp,
                base_attack: species.base_stats.attack,
                base_defense: species.base_stats.defense,
                base_speed: species.base_stats.speed,
                base_special_attack: species.base_stats.special_attack,
                base_special_defense: species.base_stats.special_defense,
                type1: species.type1.clone(),
                type2: species.type2.clone(),
                catch_rate: species.catch_rate,
                base_exp: species.base_exp,
                item1: species.item1.clone(),
                item2: species.item2.clone(),
                gender_ratio: species.gender_ratio,
                step_cycles_to_hatch: species.step_cycles_to_hatch,
                growth_rate: species.growth_rate.clone(),
                egg_group1: species.egg_group1.clone(),
                egg_group2: species.egg_group2.clone(),
                tmhm_learnset: species.tmhm_learnset.clone(),
                ability: species.ability.clone(),
                weight: species.weight,
            })
            .collect()
    }

    pub fn species_learnset_keys(&self) -> BTreeSet<RuntimeSpeciesLearnsetKey> {
        self.data
            .learnsets
            .iter()
            .flat_map(|(species_id, entries)| {
                entries.iter().map(move |entry| RuntimeSpeciesLearnsetKey {
                    species_id: species_id.clone(),
                    level: entry.0,
                    move_id: entry.1.clone(),
                })
            })
            .collect()
    }

    pub fn species_evolution_keys(&self) -> BTreeSet<RuntimeSpeciesEvolutionKey> {
        self.data
            .evolutions
            .0
            .iter()
            .flat_map(|(source_species_id, entries)| {
                entries.iter().map(move |entry| RuntimeSpeciesEvolutionKey {
                    source_species_id: source_species_id.clone(),
                    method: entry.method.clone(),
                    target_species_id: entry.species.clone(),
                    level: entry.level,
                    item: entry.item.clone(),
                    held_item: entry.held_item.clone(),
                    happiness: entry.happiness.clone(),
                    stat_ratio: entry.stat_ratio.clone(),
                })
            })
            .collect()
    }

    pub fn trainer_battle_data_keys(&self) -> BTreeSet<RuntimeTrainerBattleDataKey> {
        self.data
            .trainers
            .trainers
            .values()
            .map(|trainer| RuntimeTrainerBattleDataKey {
                trainer_id: trainer.trainer_id.clone(),
                name: trainer.name.clone(),
                trainer_class: trainer.trainer_class.clone(),
                win_quote: trainer.win_quote.clone(),
                lose_quote: trainer.lose_quote.clone(),
                items: trainer.items.clone(),
                base_reward: trainer.base_reward,
                ai_move_flags: trainer.ai_move_flags,
                ai_item_switch_flags: trainer.ai_item_switch_flags,
                encounter_music: trainer.encounter_music.clone(),
                ai_layers: trainer.ai_layers.clone(),
            })
            .collect()
    }

    pub fn trainer_party_pokemon_keys(&self) -> BTreeSet<RuntimeTrainerPartyPokemonKey> {
        self.data
            .trainers
            .trainers
            .values()
            .flat_map(|trainer| {
                trainer
                    .party
                    .iter()
                    .enumerate()
                    .map(
                        move |(party_index, pokemon)| RuntimeTrainerPartyPokemonKey {
                            trainer_id: trainer.trainer_id.clone(),
                            party_index,
                            species: pokemon.species.clone(),
                            level: pokemon.level,
                            item: pokemon.item.clone(),
                            move_names: pokemon
                                .moves
                                .iter()
                                .map(|move_data| move_data.name.clone())
                                .collect(),
                            move_pp: pokemon
                                .moves
                                .iter()
                                .map(|move_data| move_data.current_pp)
                                .collect(),
                            move_pp_ups: pokemon
                                .moves
                                .iter()
                                .map(|move_data| move_data.pp_ups)
                                .collect(),
                            dv_attack: pokemon.dvs.attack,
                            dv_defense: pokemon.dvs.defense,
                            dv_speed: pokemon.dvs.speed,
                            dv_special: pokemon.dvs.special,
                            dv_hp: pokemon.dvs.hp,
                        },
                    )
            })
            .collect()
    }

    pub fn move_priority_effect_keys(&self) -> BTreeSet<RuntimeMovePriorityEffectKey> {
        self.data
            .move_priorities
            .effect_priorities
            .iter()
            .map(|(effect_id, priority)| RuntimeMovePriorityEffectKey {
                effect_id: effect_id.clone(),
                priority: *priority,
            })
            .collect()
    }

    pub fn move_priority_move_keys(&self) -> BTreeSet<RuntimeMovePriorityMoveKey> {
        self.data
            .move_priorities
            .move_priorities
            .iter()
            .map(|priority| RuntimeMovePriorityMoveKey {
                move_id: priority.r#move.clone(),
                priority: priority.priority,
            })
            .collect()
    }

    pub fn battle_stat_multiplier_keys(&self) -> BTreeSet<RuntimeBattleStatMultiplierKey> {
        let mut keys = BTreeSet::new();
        keys.extend(
            self.data
                .battle_stat_multipliers
                .stat
                .iter()
                .enumerate()
                .map(|(index, multiplier)| RuntimeBattleStatMultiplierKey {
                    table: "stat".to_string(),
                    stage: index as i8 - 6,
                    numerator: multiplier.numerator,
                    denominator: multiplier.denominator,
                }),
        );
        keys.extend(
            self.data
                .battle_stat_multipliers
                .accuracy
                .iter()
                .enumerate()
                .map(|(index, multiplier)| RuntimeBattleStatMultiplierKey {
                    table: "accuracy".to_string(),
                    stage: index as i8 - 6,
                    numerator: multiplier.numerator,
                    denominator: multiplier.denominator,
                }),
        );
        keys
    }

    pub fn battle_reward_rule_keys(&self) -> BTreeSet<RuntimeBattleRewardRuleKey> {
        [
            RuntimeBattleRewardRuleKey {
                field: "max_level".to_string(),
                value: i32::from(self.data.battle_reward_rules.max_level),
            },
            RuntimeBattleRewardRuleKey {
                field: "wild_exp_divisor".to_string(),
                value: self.data.battle_reward_rules.wild_exp_divisor,
            },
            RuntimeBattleRewardRuleKey {
                field: "trainer_exp_numerator".to_string(),
                value: self.data.battle_reward_rules.trainer_exp_numerator,
            },
            RuntimeBattleRewardRuleKey {
                field: "trainer_exp_denominator".to_string(),
                value: self.data.battle_reward_rules.trainer_exp_denominator,
            },
        ]
        .into_iter()
        .collect()
    }

    pub fn battle_escape_rule_keys(&self) -> BTreeSet<RuntimeBattleEscapeRuleKey> {
        [
            RuntimeBattleEscapeRuleKey {
                field: "player_speed_multiplier".to_string(),
                value: self.data.battle_escape_rules.player_speed_multiplier,
            },
            RuntimeBattleEscapeRuleKey {
                field: "enemy_speed_divisor".to_string(),
                value: self.data.battle_escape_rules.enemy_speed_divisor,
            },
            RuntimeBattleEscapeRuleKey {
                field: "failed_attempt_bonus".to_string(),
                value: self.data.battle_escape_rules.failed_attempt_bonus,
            },
            RuntimeBattleEscapeRuleKey {
                field: "rng_roll_values".to_string(),
                value: self.data.battle_escape_rules.rng_roll_values,
            },
        ]
        .into_iter()
        .collect()
    }

    pub fn physical_type_ids(&self) -> BTreeSet<String> {
        self.data.type_categories.physical.iter().cloned().collect()
    }

    pub fn special_type_ids(&self) -> BTreeSet<String> {
        self.data.type_categories.special.iter().cloned().collect()
    }

    pub fn weather_ids(&self) -> BTreeSet<String> {
        let mut ids: BTreeSet<String> = self
            .data
            .weather_modifiers
            .type_modifiers
            .keys()
            .cloned()
            .collect();
        ids.extend(
            self.data
                .weather_modifiers
                .move_effect_modifiers
                .keys()
                .cloned(),
        );
        ids
    }

    pub fn type_effectiveness_keys(&self) -> BTreeSet<RuntimeTypeEffectivenessKey> {
        self.data
            .type_effectiveness
            .matchups
            .iter()
            .flat_map(|(attacking_type, defenders)| {
                defenders
                    .keys()
                    .map(move |defending_type| RuntimeTypeEffectivenessKey {
                        attacking_type: attacking_type.clone(),
                        defending_type: defending_type.clone(),
                    })
            })
            .collect()
    }

    pub fn foresight_type_effectiveness_keys(&self) -> BTreeSet<RuntimeTypeEffectivenessKey> {
        self.data
            .type_effectiveness
            .foresight_matchups
            .iter()
            .flat_map(|(attacking_type, defenders)| {
                defenders
                    .keys()
                    .map(move |defending_type| RuntimeTypeEffectivenessKey {
                        attacking_type: attacking_type.clone(),
                        defending_type: defending_type.clone(),
                    })
            })
            .collect()
    }

    pub fn weather_type_modifier_keys(&self) -> BTreeSet<RuntimeWeatherTypeModifierKey> {
        self.data
            .weather_modifiers
            .type_modifiers
            .iter()
            .flat_map(|(weather, modifiers)| {
                modifiers
                    .keys()
                    .map(move |type_id| RuntimeWeatherTypeModifierKey {
                        weather: weather.clone(),
                        type_id: type_id.clone(),
                    })
            })
            .collect()
    }

    pub fn weather_move_effect_modifier_keys(
        &self,
    ) -> BTreeSet<RuntimeWeatherMoveEffectModifierKey> {
        self.data
            .weather_modifiers
            .move_effect_modifiers
            .iter()
            .flat_map(|(weather, modifiers)| {
                modifiers
                    .keys()
                    .map(move |effect_id| RuntimeWeatherMoveEffectModifierKey {
                        weather: weather.clone(),
                        effect_id: effect_id.clone(),
                    })
            })
            .collect()
    }

    pub fn music_ids(&self) -> BTreeSet<String> {
        self.audio.music_ids()
    }

    pub fn sound_effect_ids(&self) -> BTreeSet<String> {
        self.audio.sound_effect_ids()
    }

    pub fn cry_ids(&self) -> BTreeSet<String> {
        self.audio.cry_ids()
    }

    pub fn pokemon_cry_keys(&self) -> BTreeSet<RuntimePokemonCryKey> {
        self.data
            .pokemon_cries
            .iter()
            .map(|(species_id, cry)| RuntimePokemonCryKey {
                species_id: species_id.clone(),
                cry_id: cry.cry.clone(),
                pitch: cry.pitch,
                length: cry.length,
            })
            .collect()
    }

    pub fn audio_asset_keys(&self) -> BTreeSet<RuntimeAudioAssetKey> {
        self.audio.audio_asset_keys()
    }

    pub fn has_special_routine(&self, routine: &str) -> bool {
        self.data.special_routines.contains_key(routine)
    }

    pub fn has_item(&self, item_id: &str) -> bool {
        self.data.items.contains_key(item_id)
    }

    pub fn has_move(&self, move_id: &str) -> bool {
        self.data.moves.contains_key(move_id)
    }

    pub fn has_species(&self, species_id: &str) -> bool {
        self.data.pokemon.contains_key(species_id)
    }

    pub fn has_map(&self, map_name: &str) -> bool {
        self.data.maps.contains_key(map_name)
    }

    pub fn has_trainer(&self, trainer_id: &str) -> bool {
        self.data.trainers.trainers.contains_key(trainer_id)
    }

    pub fn has_text(&self, text_label: &str) -> bool {
        self.data.saved_text_exists(text_label)
    }

    pub fn has_menu(&self, menu: &str) -> bool {
        self.data.saved_menu_exists(menu)
    }

    pub fn has_phone_contact(&self, contact_id: &str) -> bool {
        self.data.phone_contacts.0.contains_key(contact_id)
    }

    pub fn has_special_phone_call(&self, call_id: &str) -> bool {
        self.data.saved_special_phone_call_exists(call_id)
    }

    pub fn has_npc_trade(&self, trade_id: &str) -> bool {
        self.data.saved_npc_trade_exists(trade_id)
    }

    pub fn has_sprite(&self, sprite_id: &str) -> bool {
        self.data.saved_sprite_exists(sprite_id)
    }

    pub fn has_map_constant(&self, map_constant: &str) -> bool {
        self.data.saved_map_constant(map_constant).is_some()
    }

    pub fn has_event_flag(&self, flag: &str) -> bool {
        self.data.saved_event_flag_exists(flag)
    }

    pub fn has_engine_flag(&self, flag: &str) -> bool {
        self.data.saved_engine_flag_exists(flag)
    }

    pub fn has_spawn_identifier(&self, spawn_identifier: u16) -> bool {
        self.data.saved_spawn_identifier(spawn_identifier).is_some()
    }

    pub fn has_tileset(&self, tileset_id: &str) -> bool {
        self.data.saved_tileset_exists(tileset_id)
    }

    pub fn has_tileset_row(&self, key: &RuntimeTilesetKey) -> bool {
        self.tileset_keys().contains(key)
    }

    pub fn has_pc_string(&self, key: &RuntimePcStringKey) -> bool {
        self.pc_string_keys().contains(key)
    }

    pub fn has_menu_icon(&self, key: &RuntimeMenuIconKey) -> bool {
        self.menu_icon_keys().contains(key)
    }

    pub fn has_pokedex_entry(&self, key: &RuntimePokedexEntryKey) -> bool {
        self.pokedex_entry_keys().contains(key)
    }

    pub fn has_landmark(&self, landmark_id: &str) -> bool {
        self.data
            .pokegear_landmarks
            .landmarks
            .iter()
            .any(|landmark| landmark.constant == landmark_id)
    }

    pub fn has_pokegear_landmark(&self, key: &RuntimePokegearLandmarkKey) -> bool {
        self.pokegear_landmark_keys().contains(key)
    }

    pub fn has_pokegear_map_landmark(&self, key: &RuntimePokegearMapLandmarkKey) -> bool {
        self.pokegear_map_landmark_keys().contains(key)
    }

    pub fn has_fishing_rod(&self, rod: &str) -> bool {
        self.data.saved_fishing_rod_exists(rod)
    }

    pub fn has_map_group(&self, group_id: &str) -> bool {
        self.data
            .runtime_map_metadata
            .values()
            .any(|metadata| metadata.group_name == group_id)
    }

    pub fn has_encounter_group(&self, group_id: &str) -> bool {
        self.data.fishing.groups.contains_key(group_id)
    }

    pub fn has_mart(&self, mart_id: &str) -> bool {
        self.data.marts.0.contains_key(mart_id)
    }

    pub fn has_mart_row(&self, key: &RuntimeMartKey) -> bool {
        self.mart_keys().contains(key)
    }

    pub fn has_fruit_tree(&self, fruit_tree_id: &str) -> bool {
        self.data.fruit_trees.0.contains_key(fruit_tree_id)
    }

    pub fn has_fruit_tree_row(&self, key: &RuntimeFruitTreeKey) -> bool {
        self.fruit_tree_keys().contains(key)
    }

    pub fn has_field_move_rule(&self, rule_id: &str) -> bool {
        self.field_move_rule_ids().contains(rule_id)
    }

    pub fn has_field_move_rule_row(&self, key: &RuntimeFieldMoveRuleKey) -> bool {
        self.field_move_rule_keys().contains(key)
    }

    pub fn has_fly_destination(&self, flypoint_flag: &str) -> bool {
        self.data.fly_destinations.contains_key(flypoint_flag)
    }

    pub fn has_fly_destination_row(&self, key: &RuntimeFlyDestinationKey) -> bool {
        self.fly_destination_keys().contains(key)
    }

    pub fn has_field_move_move(&self, move_id: &str) -> bool {
        self.field_move_move_ids().contains(move_id)
    }

    pub fn has_field_move_item(&self, item_id: &str) -> bool {
        self.field_move_item_ids().contains(item_id)
    }

    pub fn has_field_box_item(&self, item_id: &str) -> bool {
        self.data.field_box_items.contains_key(item_id)
    }

    pub fn has_flee_mon_bucket(&self, bucket_id: &str) -> bool {
        self.data.flee_mons.buckets.contains_key(bucket_id)
    }

    pub fn has_buena_password_category(&self, category_id: &str) -> bool {
        self.data
            .buena_password_categories
            .categories
            .contains_key(category_id)
    }

    pub fn has_roaming_species(&self, species_id: &str) -> bool {
        self.data
            .roaming_pokemon
            .init_writes
            .iter()
            .any(|write| write.species == species_id)
    }

    pub fn has_buena_prize_item(&self, item_id: &str) -> bool {
        self.data.buena_prizes.contains_key(item_id)
    }

    pub fn has_kurt_apricorn_item(&self, item_id: &str) -> bool {
        self.data.kurt_apricorn_recipes.contains_key(item_id)
    }

    pub fn has_dratini_move_set(&self, answer: u8) -> bool {
        self.data.dratini_move_sets.contains_key(&answer)
    }

    pub fn has_special_feature(&self, feature_id: &str) -> bool {
        self.special_feature_ids().contains(feature_id)
    }

    pub fn has_oak_rating_text(&self, text_id: &str) -> bool {
        self.data
            .oak_ratings
            .iter()
            .any(|rating| rating.text_label == text_id)
    }

    pub fn has_odd_egg_species(&self, species_id: &str) -> bool {
        self.data
            .odd_egg_definitions
            .iter()
            .any(|definition| definition.species == species_id)
    }

    pub fn has_magikarp_length_threshold(&self, threshold: u16) -> bool {
        self.data
            .magikarp_lengths
            .iter()
            .any(|entry| entry.threshold == threshold)
    }

    pub fn has_happiness_change(&self, change_id: u8) -> bool {
        self.data
            .happiness_data
            .as_ref()
            .is_some_and(|data| data.changes.contains_key(&change_id))
    }

    pub fn has_happiness_service(&self, service_id: &str) -> bool {
        self.data
            .happiness_data
            .as_ref()
            .is_some_and(|data| data.services.contains_key(service_id))
    }

    pub fn has_pokemon_status(&self, status: &str) -> bool {
        self.data.saved_pokemon_status_exists(status)
    }

    pub fn has_fishing_daily_flag_bit(&self, bit: u32) -> bool {
        self.data.saved_fishing_daily_flag_bit_exists(bit)
    }

    pub fn has_fishing_swarm_flag(&self, swarm_flag: u8) -> bool {
        self.data.saved_fishing_swarm_flag_exists(swarm_flag)
    }

    pub fn has_pending_special_battle_type(&self, battle_type: &str) -> bool {
        self.data
            .saved_pending_special_battle_type_exists(battle_type)
    }

    pub fn has_wild_encounter_origin(&self, key: &RuntimeWildEncounterOriginKey) -> bool {
        self.data
            .saved_wild_encounter_exists(&key.map_name, &key.species, key.level)
    }

    pub fn has_script_label(&self, script_label: &str) -> bool {
        self.data.compiled_script_body(script_label).is_some()
    }

    pub fn script_owner_map(&self, script_label: &str) -> Result<String> {
        self.data
            .maps
            .iter()
            .find(|(_, module)| module.scripts.contains_key(script_label))
            .map(|(map_name, _)| map_name.clone())
            .with_context(|| format!("compiled game pack missing script label {script_label}"))
    }

    pub fn compiled_script_command_name(
        &self,
        script_label: &str,
        command_index: usize,
    ) -> Result<String> {
        let body = self
            .data
            .compiled_script_body(script_label)
            .with_context(|| format!("compiled game pack missing script label {script_label}"))?;
        let command = body
            .as_array()
            .and_then(|commands| commands.get(command_index))
            .with_context(|| {
                format!("compiled script {script_label} missing command {command_index}")
            })?;
        command
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .with_context(|| {
                format!(
                    "compiled script {script_label} command {command_index} missing command name"
                )
            })
    }

    pub fn compiled_script_commands(&self, script_label: &str) -> Result<Vec<serde_json::Value>> {
        let body = self
            .data
            .compiled_script_body(script_label)
            .with_context(|| format!("compiled game pack missing script label {script_label}"))?;
        body.as_array()
            .cloned()
            .with_context(|| format!("compiled script {script_label} is not a command array"))
    }

    pub fn has_script_command(&self, key: &RuntimeScriptCommandKey) -> bool {
        self.data
            .compiled_script_body(&key.script_label)
            .and_then(|body| body.as_array())
            .is_some_and(|commands| key.command_index < commands.len())
    }

    pub fn has_script_command_payload(&self, key: &RuntimeScriptCommandPayloadKey) -> bool {
        self.data
            .validate_saved_script_command_payload_reference(
                "runtime.script_command_payload",
                &key.script_label,
                key.command_index,
                &key.command,
                &key.args,
            )
            .is_ok()
    }

    pub fn has_script_return(&self, key: &RuntimeScriptReturnKey) -> bool {
        self.data
            .validate_saved_script_return_reference(
                "runtime.script_return",
                &key.script_label,
                key.next_command_index,
            )
            .is_ok()
    }

    pub fn has_script_vertical_menu(&self, key: &RuntimeScriptVerticalMenuKey) -> bool {
        self.script_vertical_menu_keys().contains(key)
    }

    pub fn has_script_text_body(&self, key: &RuntimeScriptTextBodyKey) -> bool {
        self.script_text_body_keys().contains(key)
    }

    pub fn has_script_menu_definition(&self, key: &RuntimeScriptMenuDefinitionKey) -> bool {
        self.script_menu_definition_keys().contains(key)
    }

    pub fn has_script_elevator(&self, key: &RuntimeScriptElevatorKey) -> bool {
        self.script_elevator_keys().contains(key)
    }

    pub fn has_script_elevator_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_elevators.values().any(|elevator| {
                elevator.source_script == source_script
                    && elevator.elevator_command_index == command_index
            })
        })
    }

    pub fn has_gift_pokemon(&self, key: &RuntimeGiftPokemonKey) -> bool {
        self.gift_pokemon_keys().contains(key)
    }

    pub fn has_gift_pokemon_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.gift_pokemon_keys().into_iter().any(|key| {
            key.map_name == map_name
                && key.source_script == source_script
                && key.command_index == command_index
        })
    }

    pub fn has_script_phone_prompt_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_phone_commands.iter().any(|command| {
                command.command == "askforphonenumber"
                    && command.source_script == source_script
                    && command.command_index == command_index
            })
        })
    }

    pub fn has_scripted_wild_battle_start_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.scripted_wild_battles.iter().any(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == command_index
            })
        })
    }

    pub fn has_scripted_trainer_battle_start_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.scripted_trainer_battle_keys().into_iter().any(|key| {
            key.map_name == map_name
                && key.source_script == source_script
                && key.startbattle_command_index == command_index
        })
    }

    pub fn has_script_object_command(&self, key: &RuntimeScriptObjectCommandKey) -> bool {
        self.script_object_command_keys().contains(key)
    }

    pub fn has_script_movement(&self, key: &RuntimeScriptMovementKey) -> bool {
        self.script_movement_keys().contains(key)
    }

    pub fn has_map_script_section_command(&self, key: &RuntimeMapScriptSectionCommandKey) -> bool {
        self.map_script_section_command_keys().contains(key)
    }

    pub fn has_map_event_section_command(&self, key: &RuntimeMapEventSectionCommandKey) -> bool {
        self.map_event_section_command_keys().contains(key)
    }

    pub fn has_script_map_command(&self, key: &RuntimeScriptMapCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_map_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.target_map == key.target_map
                        && command.x == key.x
                        && command.y == key.y
                        && command.facing == key.facing
                        && command.map_setup == key.map_setup
                })
    }

    pub fn has_script_variable_command(&self, key: &RuntimeScriptVariableCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_variable_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.target == key.target
                        && command.value_tokens == key.value_tokens
                })
    }

    pub fn has_script_control_command(&self, key: &RuntimeScriptControlCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_control_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.compare_value == key.compare_value
                        && command.target_label == key.target_label
                        && command.resolved_target_script == key.resolved_target_script
                })
    }

    pub fn has_script_swarm_command(&self, key: &RuntimeScriptSwarmCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_swarm_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.swarm_token == key.swarm_token
                        && command.map_id == key.map_id
                })
    }

    pub fn has_script_field_pickup(&self, key: &RuntimeScriptFieldPickupKey) -> bool {
        self.script_field_pickup_keys().contains(key)
    }

    pub fn has_script_shop_command(&self, key: &RuntimeScriptShopCommandKey) -> bool {
        self.script_shop_command_keys().contains(key)
    }

    pub fn has_script_phone_command(&self, key: &RuntimeScriptPhoneCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_phone_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command && command.contact_id == key.contact_id
                })
    }

    pub fn has_script_runtime_command(&self, key: &RuntimeScriptRuntimeCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_runtime_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| command.command == key.command && command.args == key.args)
    }

    pub fn has_script_item_grant(&self, key: &RuntimeScriptItemGrantKey) -> bool {
        self.script_item_grant_keys().contains(key)
    }

    pub fn has_script_item_access(&self, key: &RuntimeScriptItemAccessKey) -> bool {
        self.script_item_access_keys().contains(key)
    }

    pub fn has_script_economy_command(&self, key: &RuntimeScriptEconomyCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_economy_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.account == key.account
                        && command.amount_tokens == key.amount_tokens
                })
    }

    pub fn has_script_flag_command(&self, key: &RuntimeScriptFlagCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_flag_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command && command.flag_id == key.flag_id
                })
    }

    pub fn has_script_scene_command(&self, key: &RuntimeScriptSceneCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_scene_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.map_id == key.map_id
                        && command.scene_id == key.scene_id
                })
    }

    pub fn has_script_block_change(&self, key: &RuntimeScriptBlockChangeKey) -> bool {
        self.script_block_change_keys().contains(key)
    }

    pub fn has_script_audio_command(&self, key: &RuntimeScriptAudioCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_audio_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command
                        && command.audio_id == key.audio_id
                        && command.fade_frames == key.fade_frames
                })
    }

    pub fn has_script_text_command(&self, key: &RuntimeScriptTextCommandKey) -> bool {
        self.data.maps.contains_key(&key.map_name)
            && self
                .data
                .script_text_command(&key.map_name, &key.source_script, key.command_index)
                .is_ok_and(|command| {
                    command.command == key.command && command.text_label == key.text_label
                })
    }

    pub fn has_script_object_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_object_commands.iter().any(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
        })
    }

    pub fn has_script_movement_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_object_commands.iter().any(|command| {
                command.source_script == source_script
                    && command.command_index == command_index
                    && command.movement.is_some()
            })
        })
    }

    pub fn has_script_map_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_map_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_variable_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_variable_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_control_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_control_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_swarm_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_swarm_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_phone_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_phone_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_field_pickup_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_field_pickups.iter().any(|pickup| {
                pickup.source_script == source_script && pickup.command_index == command_index
            })
        })
    }

    pub fn has_script_shop_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_shop_commands.iter().any(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
        })
    }

    pub fn has_script_runtime_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_runtime_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_item_grant_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_item_grants.iter().any(|grant| {
                grant.source_script == source_script && grant.command_index == command_index
            })
        })
    }

    pub fn has_script_item_check_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_item_checks.iter().any(|access| {
                access.command == "checkitem"
                    && access.source_script == source_script
                    && access.command_index == command_index
            })
        })
    }

    pub fn has_script_item_take_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_item_takes.iter().any(|access| {
                access.command == "takeitem"
                    && access.source_script == source_script
                    && access.command_index == command_index
            })
        })
    }

    pub fn has_script_economy_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_economy_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_flag_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_flag_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_scene_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_scene_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_block_change_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data.maps.get(map_name).is_some_and(|module| {
            module.script_block_changes.iter().any(|change| {
                change.source_script == source_script && change.command_index == command_index
            })
        })
    }

    pub fn has_script_audio_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_audio_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_script_text_command_at(
        &self,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> bool {
        self.data
            .script_text_command(map_name, source_script, command_index)
            .is_ok()
    }

    pub fn has_warp(&self, key: &RuntimeWarpKey) -> bool {
        self.data.saved_warp_exists(&key.map_name, key.warp_index)
    }

    pub fn has_map_object(&self, key: &RuntimeMapObjectKey) -> bool {
        self.data.map_declares_object(&key.map_name, &key.object_id)
    }

    pub fn has_map_scene(&self, key: &RuntimeMapSceneKey) -> bool {
        self.data
            .saved_scene_index(&key.map_name, &key.scene_id)
            .is_some()
    }

    pub fn has_map_metadata(&self, key: &RuntimeMapMetadataKey) -> bool {
        self.map_metadata_keys().contains(key)
    }

    pub fn has_currency_constant(&self, id: &str) -> bool {
        self.data.currency_constants.0.contains_key(id)
    }

    pub fn has_capture_ball_rule(&self, id: &str) -> bool {
        self.data.capture_rules.ball_rules.contains_key(id)
    }

    pub fn has_guaranteed_capture_ball(&self, id: &str) -> bool {
        self.data
            .capture_rules
            .guaranteed_capture_balls
            .contains(id)
    }

    pub fn has_capture_status_bonus(&self, status: &str) -> bool {
        self.data.capture_rules.status_bonus.contains_key(status)
    }

    pub fn has_fast_ball_species(&self, species_id: &str) -> bool {
        self.data
            .capture_rules
            .fast_ball_species
            .contains(species_id)
    }

    pub fn has_heavy_ball_species(&self, species_id: &str) -> bool {
        self.data
            .capture_rules
            .heavy_ball_modifiers
            .contains_key(species_id)
    }

    pub fn has_move_priority_effect(&self, effect_id: &str) -> bool {
        self.data
            .move_priorities
            .effect_priorities
            .contains_key(effect_id)
    }

    pub fn has_move_priority_move(&self, move_id: &str) -> bool {
        self.data
            .move_priorities
            .move_priorities
            .iter()
            .any(|priority| priority.r#move == move_id)
    }

    pub fn has_capture_ball_rule_key(&self, key: &RuntimeCaptureBallRuleKey) -> bool {
        self.data
            .capture_rules
            .ball_rules
            .get(&key.ball_id)
            .is_some_and(|rule| {
                rule.multiplier_numerator == key.multiplier_numerator
                    && rule.multiplier_denominator == key.multiplier_denominator
                    && rule.battle_type == key.battle_type
                    && rule.skip_hp_calc == key.skip_hp_calc
                    && rule.use_heavy_ball_weight_modifier == key.use_heavy_ball_weight_modifier
                    && rule.use_level_ball_multiplier == key.use_level_ball_multiplier
                    && rule.require_same_species == key.require_same_species
                    && rule.require_same_gender == key.require_same_gender
                    && rule.require_fast_species == key.require_fast_species
            })
    }

    pub fn has_heavy_ball_modifier(&self, key: &RuntimeHeavyBallModifierKey) -> bool {
        self.data
            .capture_rules
            .heavy_ball_modifiers
            .get(&key.species_id)
            .is_some_and(|modifier| *modifier == key.modifier)
    }

    pub fn has_capture_status_bonus_key(&self, key: &RuntimeCaptureStatusBonusKey) -> bool {
        self.data
            .capture_rules
            .status_bonus
            .get(&key.status)
            .is_some_and(|bonus| *bonus == key.bonus)
    }

    pub fn has_capture_wobble_probability(&self, key: &RuntimeCaptureWobbleProbabilityKey) -> bool {
        self.data
            .capture_wobble_probabilities
            .iter()
            .any(|probability| {
                probability.catch_rate == key.catch_rate && probability.chance == key.chance
            })
    }

    pub fn has_item_battle_use(&self, key: &RuntimeItemBattleUseKey) -> bool {
        self.data.items.get(&key.item_id).is_some_and(|item| {
            item.effect == key.effect
                && item.battle_menu == key.battle_menu
                && item.battle_usable == key.battle_usable
                && item.battle_stat_boost_stat == key.battle_stat_boost_stat
                && item.battle_stat_boost_stages == key.battle_stat_boost_stages
                && item.battle_escape_mode == key.battle_escape_mode
                && item.battle_focus_energy == key.battle_focus_energy
                && item.battle_stat_drop_guard == key.battle_stat_drop_guard
                && item.battle_stat_drop_guard_turns == key.battle_stat_drop_guard_turns
        })
    }

    pub fn has_item_effect_plan(&self, key: &RuntimeItemEffectPlanKey) -> bool {
        self.item_effect_plan_keys().contains(key)
    }

    pub fn has_item_field_use(&self, key: &RuntimeItemFieldUseKey) -> bool {
        self.data.items.get(&key.item_id).is_some_and(|item| {
            item.effect == key.effect
                && item.field_menu == key.field_menu
                && item.field_usable == key.field_usable
                && item.consumable == key.consumable
                && item.repel_steps == key.repel_steps
                && item.escape_rope_mode == key.escape_rope_mode
                && item.tmhm_index == key.tmhm_index
                && item.tmhm_move == key.tmhm_move
        })
    }

    pub fn has_move_battle_data(&self, key: &RuntimeMoveBattleDataKey) -> bool {
        self.data.moves.get(&key.move_id).is_some_and(|move_data| {
            move_data.name == key.name
                && move_data.move_type == key.move_type
                && move_data.power == key.power
                && move_data.accuracy == key.accuracy
                && move_data.pp == key.pp
                && move_data.effect == key.effect
                && move_data.effect_chance == key.effect_chance
                && move_data.stat == key.stat
                && move_data.amount == key.amount
        })
    }

    pub fn has_species_battle_data(&self, key: &RuntimeSpeciesBattleDataKey) -> bool {
        self.data
            .pokemon
            .get(&key.species_id)
            .is_some_and(|species| {
                species.int_id == key.int_id
                    && species.base_stats.hp == key.base_hp
                    && species.base_stats.attack == key.base_attack
                    && species.base_stats.defense == key.base_defense
                    && species.base_stats.speed == key.base_speed
                    && species.base_stats.special_attack == key.base_special_attack
                    && species.base_stats.special_defense == key.base_special_defense
                    && species.type1 == key.type1
                    && species.type2 == key.type2
                    && species.catch_rate == key.catch_rate
                    && species.base_exp == key.base_exp
                    && species.item1 == key.item1
                    && species.item2 == key.item2
                    && species.gender_ratio == key.gender_ratio
                    && species.step_cycles_to_hatch == key.step_cycles_to_hatch
                    && species.growth_rate == key.growth_rate
                    && species.egg_group1 == key.egg_group1
                    && species.egg_group2 == key.egg_group2
                    && species.tmhm_learnset == key.tmhm_learnset
                    && species.ability == key.ability
                    && species.weight == key.weight
            })
    }

    pub fn has_trainer_battle_data(&self, key: &RuntimeTrainerBattleDataKey) -> bool {
        self.data
            .trainers
            .trainers
            .get(&key.trainer_id)
            .is_some_and(|trainer| {
                trainer.name == key.name
                    && trainer.trainer_class == key.trainer_class
                    && trainer.win_quote == key.win_quote
                    && trainer.lose_quote == key.lose_quote
                    && trainer.items == key.items
                    && trainer.base_reward == key.base_reward
                    && trainer.ai_move_flags == key.ai_move_flags
                    && trainer.ai_item_switch_flags == key.ai_item_switch_flags
                    && trainer.encounter_music == key.encounter_music
                    && trainer.ai_layers == key.ai_layers
            })
    }

    pub fn has_trainer_party_pokemon(&self, key: &RuntimeTrainerPartyPokemonKey) -> bool {
        self.data
            .trainers
            .trainers
            .get(&key.trainer_id)
            .and_then(|trainer| trainer.party.get(key.party_index))
            .is_some_and(|pokemon| {
                pokemon.species == key.species
                    && pokemon.level == key.level
                    && pokemon.item == key.item
                    && pokemon
                        .moves
                        .iter()
                        .map(|move_data| &move_data.name)
                        .eq(key.move_names.iter())
                    && pokemon
                        .moves
                        .iter()
                        .map(|move_data| move_data.current_pp)
                        .eq(key.move_pp.iter().copied())
                    && pokemon
                        .moves
                        .iter()
                        .map(|move_data| move_data.pp_ups)
                        .eq(key.move_pp_ups.iter().copied())
                    && pokemon.dvs.attack == key.dv_attack
                    && pokemon.dvs.defense == key.dv_defense
                    && pokemon.dvs.speed == key.dv_speed
                    && pokemon.dvs.special == key.dv_special
                    && pokemon.dvs.hp == key.dv_hp
            })
    }

    pub fn has_move_priority_effect_key(&self, key: &RuntimeMovePriorityEffectKey) -> bool {
        self.data
            .move_priorities
            .effect_priorities
            .get(&key.effect_id)
            .is_some_and(|priority| *priority == key.priority)
    }

    pub fn has_move_priority_move_key(&self, key: &RuntimeMovePriorityMoveKey) -> bool {
        self.data
            .move_priorities
            .move_priorities
            .iter()
            .any(|priority| priority.r#move == key.move_id && priority.priority == key.priority)
    }

    pub fn has_battle_stat_multiplier(&self, key: &RuntimeBattleStatMultiplierKey) -> bool {
        let index = key.stage + 6;
        if !(0..=12).contains(&index) {
            return false;
        }
        let table = match key.table.as_str() {
            "stat" => &self.data.battle_stat_multipliers.stat,
            "accuracy" => &self.data.battle_stat_multipliers.accuracy,
            _ => return false,
        };
        table.get(index as usize).is_some_and(|multiplier| {
            multiplier.numerator == key.numerator && multiplier.denominator == key.denominator
        })
    }

    pub fn has_battle_reward_rule(&self, key: &RuntimeBattleRewardRuleKey) -> bool {
        match key.field.as_str() {
            "max_level" => i32::from(self.data.battle_reward_rules.max_level) == key.value,
            "wild_exp_divisor" => self.data.battle_reward_rules.wild_exp_divisor == key.value,
            "trainer_exp_numerator" => {
                self.data.battle_reward_rules.trainer_exp_numerator == key.value
            }
            "trainer_exp_denominator" => {
                self.data.battle_reward_rules.trainer_exp_denominator == key.value
            }
            _ => false,
        }
    }

    pub fn has_battle_escape_rule(&self, key: &RuntimeBattleEscapeRuleKey) -> bool {
        match key.field.as_str() {
            "player_speed_multiplier" => {
                self.data.battle_escape_rules.player_speed_multiplier == key.value
            }
            "enemy_speed_divisor" => self.data.battle_escape_rules.enemy_speed_divisor == key.value,
            "failed_attempt_bonus" => {
                self.data.battle_escape_rules.failed_attempt_bonus == key.value
            }
            "rng_roll_values" => self.data.battle_escape_rules.rng_roll_values == key.value,
            _ => false,
        }
    }

    pub fn has_physical_type(&self, type_id: &str) -> bool {
        self.data
            .type_categories
            .physical
            .iter()
            .any(|known| known == type_id)
    }

    pub fn has_special_type(&self, type_id: &str) -> bool {
        self.data
            .type_categories
            .special
            .iter()
            .any(|known| known == type_id)
    }

    pub fn has_weather(&self, weather_id: &str) -> bool {
        self.data
            .weather_modifiers
            .type_modifiers
            .contains_key(weather_id)
            || self
                .data
                .weather_modifiers
                .move_effect_modifiers
                .contains_key(weather_id)
    }

    pub fn has_type_effectiveness(&self, key: &RuntimeTypeEffectivenessKey) -> bool {
        self.data
            .type_effectiveness
            .matchups
            .get(&key.attacking_type)
            .is_some_and(|defenders| defenders.contains_key(&key.defending_type))
    }

    pub fn has_foresight_type_effectiveness(&self, key: &RuntimeTypeEffectivenessKey) -> bool {
        self.data
            .type_effectiveness
            .foresight_matchups
            .get(&key.attacking_type)
            .is_some_and(|defenders| defenders.contains_key(&key.defending_type))
    }

    pub fn has_weather_type_modifier(&self, key: &RuntimeWeatherTypeModifierKey) -> bool {
        self.data
            .weather_modifiers
            .type_modifiers
            .get(&key.weather)
            .is_some_and(|modifiers| modifiers.contains_key(&key.type_id))
    }

    pub fn has_weather_move_effect_modifier(
        &self,
        key: &RuntimeWeatherMoveEffectModifierKey,
    ) -> bool {
        self.data
            .weather_modifiers
            .move_effect_modifiers
            .get(&key.weather)
            .is_some_and(|modifiers| modifiers.contains_key(&key.effect_id))
    }

    pub fn has_audio_asset(&self, key: &RuntimeAudioAssetKey) -> bool {
        self.audio.has_audio_asset(key)
    }

    pub fn has_pokemon_cry(&self, key: &RuntimePokemonCryKey) -> bool {
        self.pokemon_cry_keys().contains(key)
    }

    pub fn has_music(&self, music_id: &str) -> bool {
        self.audio.music.contains_key(music_id)
    }

    pub fn has_sound_effect(&self, sound_effect_id: &str) -> bool {
        self.audio.sound_effects.contains_key(sound_effect_id)
    }

    pub fn has_cry(&self, cry_id: &str) -> bool {
        self.audio.cries.contains_key(cry_id)
    }

    pub fn require_special_routine(&self, routine: &str) -> Result<()> {
        self.data.require_special_routine(routine)
    }

    pub fn require_item(&self, item_id: &str) -> Result<()> {
        require_runtime_catalog_id("item", item_id, self.has_item(item_id))
    }

    pub fn require_move(&self, move_id: &str) -> Result<()> {
        require_runtime_catalog_id("move", move_id, self.has_move(move_id))
    }

    pub fn require_species(&self, species_id: &str) -> Result<()> {
        require_runtime_catalog_id("Pokemon species", species_id, self.has_species(species_id))
    }

    pub fn require_map(&self, map_name: &str) -> Result<()> {
        require_runtime_catalog_id("map", map_name, self.has_map(map_name))
    }

    pub fn require_trainer(&self, trainer_id: &str) -> Result<()> {
        require_runtime_catalog_id("trainer", trainer_id, self.has_trainer(trainer_id))
    }

    pub fn require_text(&self, text_label: &str) -> Result<()> {
        self.data
            .validate_saved_text_reference("runtime.text", text_label)
    }

    pub fn require_menu(&self, menu: &str) -> Result<()> {
        self.data
            .validate_saved_menu_reference("runtime.menu", menu)
    }

    pub fn require_phone_contact(&self, contact_id: &str) -> Result<()> {
        self.data
            .validate_saved_phone_contact_reference("runtime.phone_contact", contact_id)
    }

    pub fn require_special_phone_call(&self, call_id: &str) -> Result<()> {
        self.data
            .validate_saved_special_phone_call_reference("runtime.special_phone_call", call_id)
    }

    pub fn require_npc_trade(&self, trade_id: &str) -> Result<()> {
        self.data
            .validate_saved_npc_trade_reference("runtime.npc_trade", trade_id)
    }

    pub fn require_sprite(&self, sprite_id: &str) -> Result<()> {
        self.data
            .validate_saved_sprite_reference("runtime.sprite", sprite_id)
    }

    pub fn require_map_constant(&self, map_constant: &str) -> Result<()> {
        self.data
            .validate_saved_map_constant_reference("runtime.map_constant", map_constant)
    }

    pub fn require_event_flag(&self, flag: &str) -> Result<()> {
        self.data
            .validate_saved_event_flag_reference("runtime.event_flag", flag)
    }

    pub fn require_engine_flag(&self, flag: &str) -> Result<()> {
        self.data
            .validate_saved_engine_flag_reference("runtime.engine_flag", flag)
    }

    pub fn require_spawn_identifier(&self, spawn_identifier: u16) -> Result<()> {
        self.data
            .validate_saved_spawn_reference("runtime.spawn_identifier", spawn_identifier)
    }

    pub fn require_tileset(&self, tileset_id: &str) -> Result<()> {
        require_runtime_catalog_id("tileset", tileset_id, self.has_tileset(tileset_id))
    }

    pub fn require_tileset_row(&self, key: &RuntimeTilesetKey) -> Result<()> {
        if self.has_tileset_row(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact tileset row {}",
                key.tileset_id
            )
        }
    }

    pub fn require_pc_string(&self, key: &RuntimePcStringKey) -> Result<()> {
        if self.has_pc_string(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact PC string row {}",
                key.string_id
            )
        }
    }

    pub fn require_menu_icon(&self, key: &RuntimeMenuIconKey) -> Result<()> {
        if self.has_menu_icon(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact menu icon row {}",
                key.species_id
            )
        }
    }

    pub fn require_pokedex_entry(&self, key: &RuntimePokedexEntryKey) -> Result<()> {
        if self.has_pokedex_entry(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact Pokedex entry row {}",
                key.species_id
            )
        }
    }

    pub fn require_landmark(&self, landmark_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "Pokegear landmark",
            landmark_id,
            self.has_landmark(landmark_id),
        )
    }

    pub fn require_pokegear_landmark(&self, key: &RuntimePokegearLandmarkKey) -> Result<()> {
        if self.has_pokegear_landmark(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact Pokegear landmark row {}",
                key.constant
            )
        }
    }

    pub fn require_pokegear_map_landmark(&self, key: &RuntimePokegearMapLandmarkKey) -> Result<()> {
        if self.has_pokegear_map_landmark(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact Pokegear map landmark row {}",
                key.map_name
            )
        }
    }

    pub fn require_fishing_rod(&self, rod: &str) -> Result<()> {
        require_runtime_catalog_id("fishing rod", rod, self.has_fishing_rod(rod))
    }

    pub fn require_map_group(&self, group_id: &str) -> Result<()> {
        require_runtime_catalog_id("map group", group_id, self.has_map_group(group_id))
    }

    pub fn require_encounter_group(&self, group_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "encounter group",
            group_id,
            self.has_encounter_group(group_id),
        )
    }

    pub fn require_mart(&self, mart_id: &str) -> Result<()> {
        require_runtime_catalog_id("mart", mart_id, self.has_mart(mart_id))
    }

    pub fn require_mart_row(&self, key: &RuntimeMartKey) -> Result<()> {
        if self.has_mart_row(key) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact mart row {}", key.mart_id)
        }
    }

    pub fn require_fruit_tree(&self, fruit_tree_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "fruit tree",
            fruit_tree_id,
            self.has_fruit_tree(fruit_tree_id),
        )
    }

    pub fn require_fruit_tree_row(&self, key: &RuntimeFruitTreeKey) -> Result<()> {
        if self.has_fruit_tree_row(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact fruit tree row {}",
                key.fruit_tree_id
            )
        }
    }

    pub fn require_field_move_rule(&self, rule_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "field move rule",
            rule_id,
            self.has_field_move_rule(rule_id),
        )
    }

    pub fn require_field_move_rule_row(&self, key: &RuntimeFieldMoveRuleKey) -> Result<()> {
        if self.has_field_move_rule_row(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact field move rule row {}",
                key.rule_id
            )
        }
    }

    pub fn require_fly_destination(&self, flypoint_flag: &str) -> Result<()> {
        require_runtime_catalog_id(
            "fly destination",
            flypoint_flag,
            self.has_fly_destination(flypoint_flag),
        )
    }

    pub fn require_fly_destination_row(&self, key: &RuntimeFlyDestinationKey) -> Result<()> {
        if self.has_fly_destination_row(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact fly destination row {}",
                key.flypoint_flag
            )
        }
    }

    pub fn require_field_move_move(&self, move_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "field move move",
            move_id,
            self.has_field_move_move(move_id),
        )
    }

    pub fn require_field_move_item(&self, item_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "field move item",
            item_id,
            self.has_field_move_item(item_id),
        )
    }

    pub fn require_flee_mon_bucket(&self, bucket_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "flee mon bucket",
            bucket_id,
            self.has_flee_mon_bucket(bucket_id),
        )
    }

    pub fn require_buena_password_category(&self, category_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "Buena password category",
            category_id,
            self.has_buena_password_category(category_id),
        )
    }

    pub fn require_roaming_species(&self, species_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "roaming Pokemon species",
            species_id,
            self.has_roaming_species(species_id),
        )
    }

    pub fn require_buena_prize_item(&self, item_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "Buena prize item",
            item_id,
            self.has_buena_prize_item(item_id),
        )
    }

    pub fn require_kurt_apricorn_item(&self, item_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "Kurt apricorn item",
            item_id,
            self.has_kurt_apricorn_item(item_id),
        )
    }

    pub fn require_dratini_move_set(&self, answer: u8) -> Result<()> {
        if self.has_dratini_move_set(answer) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact Dratini move set id {answer}")
        }
    }

    pub fn require_special_feature(&self, feature_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "special feature",
            feature_id,
            self.has_special_feature(feature_id),
        )
    }

    pub fn require_oak_rating_text(&self, text_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "Oak rating text",
            text_id,
            self.has_oak_rating_text(text_id),
        )
    }

    pub fn require_odd_egg_species(&self, species_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "Odd Egg species",
            species_id,
            self.has_odd_egg_species(species_id),
        )
    }

    pub fn require_magikarp_length_threshold(&self, threshold: u16) -> Result<()> {
        if self.has_magikarp_length_threshold(threshold) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact Magikarp length threshold {threshold}")
        }
    }

    pub fn require_happiness_change(&self, change_id: u8) -> Result<()> {
        if self.has_happiness_change(change_id) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact happiness change id {change_id}")
        }
    }

    pub fn require_happiness_service(&self, service_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "happiness service",
            service_id,
            self.has_happiness_service(service_id),
        )
    }

    pub fn require_pokemon_status(&self, status: &str) -> Result<()> {
        self.data
            .validate_saved_pokemon_status_reference("runtime.pokemon_status", status)
    }

    pub fn require_fishing_daily_flag_bit(&self, bit: u32) -> Result<()> {
        if self.has_fishing_daily_flag_bit(bit) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact fishing daily flag bit {bit}")
        }
    }

    pub fn require_fishing_swarm_flag(&self, swarm_flag: u8) -> Result<()> {
        if self.has_fishing_swarm_flag(swarm_flag) {
            Ok(())
        } else {
            anyhow::bail!("compiled game pack missing exact fishing swarm flag {swarm_flag}")
        }
    }

    pub fn require_pending_special_battle_type(&self, battle_type: &str) -> Result<()> {
        self.data
            .validate_saved_pending_special_battle_type(Some(battle_type))
    }

    pub fn require_wild_encounter_origin(&self, key: &RuntimeWildEncounterOriginKey) -> Result<()> {
        if self.has_wild_encounter_origin(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact wild encounter origin {}:{}:{}",
                key.map_name,
                key.species,
                key.level
            )
        }
    }

    pub fn require_script_label(&self, script_label: &str) -> Result<()> {
        self.data
            .validate_saved_script_label_reference("runtime.script_label", script_label)
    }

    pub fn require_script_command(&self, key: &RuntimeScriptCommandKey) -> Result<()> {
        self.data.validate_saved_script_command_reference(
            "runtime.script_command",
            &key.script_label,
            key.command_index,
        )
    }

    pub fn require_script_command_payload(
        &self,
        key: &RuntimeScriptCommandPayloadKey,
    ) -> Result<()> {
        self.data.validate_saved_script_command_payload_reference(
            "runtime.script_command_payload",
            &key.script_label,
            key.command_index,
            &key.command,
            &key.args,
        )
    }

    pub fn require_script_return(&self, key: &RuntimeScriptReturnKey) -> Result<()> {
        self.data.validate_saved_script_return_reference(
            "runtime.script_return",
            &key.script_label,
            key.next_command_index,
        )
    }

    pub fn require_script_vertical_menu(&self, key: &RuntimeScriptVerticalMenuKey) -> Result<()> {
        if self.has_script_vertical_menu(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script vertical menu row {}:{}",
                key.map_name,
                key.menu_key
            )
        }
    }

    pub fn require_script_text_body(&self, key: &RuntimeScriptTextBodyKey) -> Result<()> {
        if self.has_script_text_body(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script text body row {}:{}",
                key.map_name,
                key.body_key
            )
        }
    }

    pub fn require_script_menu_definition(
        &self,
        key: &RuntimeScriptMenuDefinitionKey,
    ) -> Result<()> {
        if self.has_script_menu_definition(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script menu definition row {}:{}",
                key.map_name,
                key.menu_key
            )
        }
    }

    pub fn require_script_elevator(&self, key: &RuntimeScriptElevatorKey) -> Result<()> {
        if self.has_script_elevator(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script elevator row {}:{}",
                key.map_name,
                key.elevator_key
            )
        }
    }

    pub fn require_gift_pokemon(&self, key: &RuntimeGiftPokemonKey) -> Result<()> {
        if self.has_gift_pokemon(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact gift Pokemon row {}:{}",
                key.map_name,
                key.source_script
            )
        }
    }

    pub fn require_script_object_command(&self, key: &RuntimeScriptObjectCommandKey) -> Result<()> {
        if self.has_script_object_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script object command row {}:{}",
                key.map_name,
                key.command_index
            )
        }
    }

    pub fn require_script_movement(&self, key: &RuntimeScriptMovementKey) -> Result<()> {
        if self.has_script_movement(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script movement row {}:{}",
                key.map_name,
                key.label
            )
        }
    }

    pub fn require_map_script_section_command(
        &self,
        key: &RuntimeMapScriptSectionCommandKey,
    ) -> Result<()> {
        if self.has_map_script_section_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact map script section command row {}:{}",
                key.map_name,
                key.command_index
            )
        }
    }

    pub fn require_map_event_section_command(
        &self,
        key: &RuntimeMapEventSectionCommandKey,
    ) -> Result<()> {
        if self.has_map_event_section_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact map event section command row {}:{}",
                key.map_name,
                key.command_index
            )
        }
    }

    pub fn require_script_map_command(&self, key: &RuntimeScriptMapCommandKey) -> Result<()> {
        if self.has_script_map_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script map command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_variable_command(
        &self,
        key: &RuntimeScriptVariableCommandKey,
    ) -> Result<()> {
        if self.has_script_variable_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script variable command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_control_command(
        &self,
        key: &RuntimeScriptControlCommandKey,
    ) -> Result<()> {
        if self.has_script_control_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script control command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_swarm_command(&self, key: &RuntimeScriptSwarmCommandKey) -> Result<()> {
        if self.has_script_swarm_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script swarm command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_field_pickup(&self, key: &RuntimeScriptFieldPickupKey) -> Result<()> {
        if self.has_script_field_pickup(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script field pickup row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_shop_command(&self, key: &RuntimeScriptShopCommandKey) -> Result<()> {
        if self.has_script_shop_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script shop command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_phone_command(&self, key: &RuntimeScriptPhoneCommandKey) -> Result<()> {
        if self.has_script_phone_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script phone command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_runtime_command(
        &self,
        key: &RuntimeScriptRuntimeCommandKey,
    ) -> Result<()> {
        if self.has_script_runtime_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script runtime command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_item_grant(&self, key: &RuntimeScriptItemGrantKey) -> Result<()> {
        if self.has_script_item_grant(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script item grant row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_item_access(&self, key: &RuntimeScriptItemAccessKey) -> Result<()> {
        if self.has_script_item_access(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script item {} row {}:{}:{}",
                key.command,
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_economy_command(
        &self,
        key: &RuntimeScriptEconomyCommandKey,
    ) -> Result<()> {
        if self.has_script_economy_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script economy command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_flag_command(&self, key: &RuntimeScriptFlagCommandKey) -> Result<()> {
        if self.has_script_flag_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script flag command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_scene_command(&self, key: &RuntimeScriptSceneCommandKey) -> Result<()> {
        if self.has_script_scene_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script scene command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_block_change(&self, key: &RuntimeScriptBlockChangeKey) -> Result<()> {
        if self.has_script_block_change(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script block change row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_audio_command(&self, key: &RuntimeScriptAudioCommandKey) -> Result<()> {
        if self.has_script_audio_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script audio command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_script_text_command(&self, key: &RuntimeScriptTextCommandKey) -> Result<()> {
        if self.has_script_text_command(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact script text command row {}:{}:{}",
                key.map_name,
                key.source_script,
                key.command_index
            )
        }
    }

    pub fn require_warp(&self, key: &RuntimeWarpKey) -> Result<()> {
        self.data
            .validate_saved_warp_reference("runtime.warp", &key.map_name, key.warp_index)
            .map(|_| ())
    }

    pub fn require_map_object(&self, key: &RuntimeMapObjectKey) -> Result<()> {
        self.data
            .validate_saved_map_object_reference(
                &key.map_name,
                "runtime.map_object",
                &key.object_id,
            )
            .map(|_| ())
    }

    pub fn require_map_scene(&self, key: &RuntimeMapSceneKey) -> Result<()> {
        if self.has_map_scene(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact map scene {}:{}",
                key.map_name,
                key.scene_id
            )
        }
    }

    pub fn require_map_metadata(&self, key: &RuntimeMapMetadataKey) -> Result<()> {
        if self.has_map_metadata(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact map metadata row {}",
                key.map_name
            )
        }
    }

    pub fn require_currency_constant(&self, id: &str) -> Result<()> {
        require_runtime_catalog_id("currency constant", id, self.has_currency_constant(id))
    }

    pub fn require_capture_ball_rule(&self, id: &str) -> Result<()> {
        require_runtime_catalog_id("capture ball rule", id, self.has_capture_ball_rule(id))
    }

    pub fn require_guaranteed_capture_ball(&self, id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "guaranteed capture ball",
            id,
            self.has_guaranteed_capture_ball(id),
        )
    }

    pub fn require_capture_status_bonus(&self, status: &str) -> Result<()> {
        require_runtime_catalog_id(
            "capture status bonus",
            status,
            self.has_capture_status_bonus(status),
        )
    }

    pub fn require_fast_ball_species(&self, species_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "fast ball species",
            species_id,
            self.has_fast_ball_species(species_id),
        )
    }

    pub fn require_heavy_ball_species(&self, species_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "heavy ball species",
            species_id,
            self.has_heavy_ball_species(species_id),
        )
    }

    pub fn require_move_priority_effect(&self, effect_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "move priority effect",
            effect_id,
            self.has_move_priority_effect(effect_id),
        )
    }

    pub fn require_move_priority_move(&self, move_id: &str) -> Result<()> {
        require_runtime_catalog_id(
            "move priority move",
            move_id,
            self.has_move_priority_move(move_id),
        )
    }

    pub fn require_capture_ball_rule_key(&self, key: &RuntimeCaptureBallRuleKey) -> Result<()> {
        if self.has_capture_ball_rule_key(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact capture ball rule row {}",
                key.ball_id
            )
        }
    }

    pub fn require_heavy_ball_modifier(&self, key: &RuntimeHeavyBallModifierKey) -> Result<()> {
        if self.has_heavy_ball_modifier(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact heavy ball modifier row {}:{}",
                key.species_id,
                key.modifier
            )
        }
    }

    pub fn require_capture_status_bonus_key(
        &self,
        key: &RuntimeCaptureStatusBonusKey,
    ) -> Result<()> {
        if self.has_capture_status_bonus_key(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact capture status bonus row {}:{}",
                key.status,
                key.bonus
            )
        }
    }

    pub fn require_capture_wobble_probability(
        &self,
        key: &RuntimeCaptureWobbleProbabilityKey,
    ) -> Result<()> {
        if self.has_capture_wobble_probability(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact capture wobble probability row {}:{}",
                key.catch_rate,
                key.chance
            )
        }
    }

    pub fn require_item_battle_use(&self, key: &RuntimeItemBattleUseKey) -> Result<()> {
        if self.has_item_battle_use(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact item battle-use row {}",
                key.item_id
            )
        }
    }

    pub fn require_item_effect_plan(&self, key: &RuntimeItemEffectPlanKey) -> Result<()> {
        if self.has_item_effect_plan(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact item effect plan row {}:{}:{}",
                key.item_id,
                key.effect_id,
                key.behavior_id
            )
        }
    }

    pub fn require_item_field_use(&self, key: &RuntimeItemFieldUseKey) -> Result<()> {
        if self.has_item_field_use(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact item field-use row {}",
                key.item_id
            )
        }
    }

    pub fn require_move_battle_data(&self, key: &RuntimeMoveBattleDataKey) -> Result<()> {
        if self.has_move_battle_data(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact move battle data row {}",
                key.move_id
            )
        }
    }

    pub fn require_species_battle_data(&self, key: &RuntimeSpeciesBattleDataKey) -> Result<()> {
        if self.has_species_battle_data(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact Pokemon species battle data row {}",
                key.species_id
            )
        }
    }

    pub fn require_trainer_battle_data(&self, key: &RuntimeTrainerBattleDataKey) -> Result<()> {
        if self.has_trainer_battle_data(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact trainer battle data row {}",
                key.trainer_id
            )
        }
    }

    pub fn require_trainer_party_pokemon(&self, key: &RuntimeTrainerPartyPokemonKey) -> Result<()> {
        if self.has_trainer_party_pokemon(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact trainer party row {}:{}",
                key.trainer_id,
                key.party_index
            )
        }
    }

    pub fn require_move_priority_effect_key(
        &self,
        key: &RuntimeMovePriorityEffectKey,
    ) -> Result<()> {
        if self.has_move_priority_effect_key(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact move priority effect row {}:{}",
                key.effect_id,
                key.priority
            )
        }
    }

    pub fn require_move_priority_move_key(&self, key: &RuntimeMovePriorityMoveKey) -> Result<()> {
        if self.has_move_priority_move_key(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact move priority move row {}:{}",
                key.move_id,
                key.priority
            )
        }
    }

    pub fn require_battle_stat_multiplier(
        &self,
        key: &RuntimeBattleStatMultiplierKey,
    ) -> Result<()> {
        if self.has_battle_stat_multiplier(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact battle stat multiplier row {}:{}:{}/{}",
                key.table,
                key.stage,
                key.numerator,
                key.denominator
            )
        }
    }

    pub fn require_battle_reward_rule(&self, key: &RuntimeBattleRewardRuleKey) -> Result<()> {
        if self.has_battle_reward_rule(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact battle reward rule row {}:{}",
                key.field,
                key.value
            )
        }
    }

    pub fn require_battle_escape_rule(&self, key: &RuntimeBattleEscapeRuleKey) -> Result<()> {
        if self.has_battle_escape_rule(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact battle escape rule row {}:{}",
                key.field,
                key.value
            )
        }
    }

    pub fn require_physical_type(&self, type_id: &str) -> Result<()> {
        require_runtime_catalog_id("physical type", type_id, self.has_physical_type(type_id))
    }

    pub fn require_special_type(&self, type_id: &str) -> Result<()> {
        require_runtime_catalog_id("special type", type_id, self.has_special_type(type_id))
    }

    pub fn require_weather(&self, weather_id: &str) -> Result<()> {
        require_runtime_catalog_id("weather", weather_id, self.has_weather(weather_id))
    }

    pub fn require_type_effectiveness(&self, key: &RuntimeTypeEffectivenessKey) -> Result<()> {
        if self.has_type_effectiveness(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact type effectiveness matchup {}:{}",
                key.attacking_type,
                key.defending_type
            )
        }
    }

    pub fn require_foresight_type_effectiveness(
        &self,
        key: &RuntimeTypeEffectivenessKey,
    ) -> Result<()> {
        if self.has_foresight_type_effectiveness(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact foresight type effectiveness matchup {}:{}",
                key.attacking_type,
                key.defending_type
            )
        }
    }

    pub fn require_audio_asset(&self, key: &RuntimeAudioAssetKey) -> Result<()> {
        if self.has_audio_asset(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact audio asset row {}:{}",
                key.kind,
                key.audio_id
            )
        }
    }

    pub fn require_pokemon_cry(&self, key: &RuntimePokemonCryKey) -> Result<()> {
        if self.has_pokemon_cry(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact Pokemon cry row {}:{}",
                key.species_id,
                key.cry_id
            )
        }
    }

    pub fn require_weather_type_modifier(&self, key: &RuntimeWeatherTypeModifierKey) -> Result<()> {
        if self.has_weather_type_modifier(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact weather type modifier {}:{}",
                key.weather,
                key.type_id
            )
        }
    }

    pub fn require_weather_move_effect_modifier(
        &self,
        key: &RuntimeWeatherMoveEffectModifierKey,
    ) -> Result<()> {
        if self.has_weather_move_effect_modifier(key) {
            Ok(())
        } else {
            anyhow::bail!(
                "compiled game pack missing exact weather move effect modifier {}:{}",
                key.weather,
                key.effect_id
            )
        }
    }

    pub fn require_music(&self, music_id: &str) -> Result<()> {
        self.audio.require_music(music_id).map(|_| ())
    }

    pub fn require_sound_effect(&self, sound_effect_id: &str) -> Result<()> {
        self.audio.require_sound_effect(sound_effect_id).map(|_| ())
    }

    pub fn require_cry(&self, cry_id: &str) -> Result<()> {
        self.audio.require_cry(cry_id).map(|_| ())
    }

    pub fn pack_identity(&self) -> &CompiledGamePackIdentity {
        &self.pack_identity
    }

    pub fn viewport(&self) -> &GameViewport {
        &self.viewport
    }

    pub fn save_game(&self, path: impl AsRef<Path>, state: GameState) -> Result<()> {
        self.validate_save_state_for_runtime_pack(&state)?;
        write_save_game_for_modpack(path, state, &self.modpack, &self.pack_identity.content_hash)
            .context("write Crystal runtime save")
    }

    pub fn load_save(&self, path: impl AsRef<Path>) -> Result<GameState> {
        let save =
            read_save_game_for_modpack(path, &self.modpack, &self.pack_identity.content_hash)
                .context("read Crystal runtime save for compiled modpack identity")?;
        let state = save.into_state();
        self.validate_save_state_for_runtime_pack(&state)?;
        Ok(state)
    }

    pub fn load_save_summary(&self, path: impl AsRef<Path>) -> Result<SaveGameSummary> {
        read_save_game_summary_for_modpack(path, &self.modpack, &self.pack_identity.content_hash)
            .context("read Crystal runtime save summary for compiled modpack identity")
    }

    pub fn list_save_slots(&self, directory: impl AsRef<Path>) -> Result<Vec<SaveSlotSummary>> {
        list_save_game_summaries_for_modpack(
            directory,
            &self.modpack,
            &self.pack_identity.content_hash,
        )
        .context("list Crystal runtime save slots for compiled modpack identity")
    }

    pub fn save_summary_for_state(&self, state: &GameState) -> Result<SaveGameSummary> {
        self.validate_save_state_for_runtime_pack(state)?;
        SaveGameSummary::new(
            self.modpack.clone(),
            self.pack_identity.content_hash.clone(),
            state,
        )
        .context("build Crystal runtime save summary")
    }

    pub fn save_checkpoint_for_state(
        &self,
        state: &GameState,
        player_id: PlayerId,
    ) -> Result<SaveCheckpointFrame> {
        let summary = self.save_summary_for_state(state)?;
        let checksum = StateChecksumFrame::from_game_state(player_id, state)
            .context("checksum Crystal runtime save checkpoint state")?;
        SaveCheckpointFrame::new(summary, checksum).context("build Crystal runtime save checkpoint")
    }

    pub fn session_save_checkpoint_for_state(
        &self,
        session: LinkSessionIdentity,
        state: &GameState,
        player_id: PlayerId,
    ) -> Result<SessionSaveCheckpointFrame> {
        let checkpoint = self.save_checkpoint_for_state(state, player_id)?;
        SessionSaveCheckpointFrame::new(session, checkpoint)
            .context("build session-bound Crystal runtime save checkpoint")
    }

    pub fn load_save_checkpoint(
        &self,
        path: impl AsRef<Path>,
        player_id: PlayerId,
    ) -> Result<SaveCheckpointFrame> {
        let state = self.load_save(path)?;
        self.save_checkpoint_for_state(&state, player_id)
    }

    fn active_menu_snapshot(&self, state: &GameState) -> Result<Option<RuntimeMenuSnapshot>> {
        let Some(menu_id) = state.script_runtime.active_menu.clone() else {
            return Ok(None);
        };
        if let OverworldMemory::Active { map_name, .. } = &state.overworld
            && let Some(module) = self.data.maps.get(map_name)
            && let Some(definition) = module.script_menu_definitions.get(&menu_id)
        {
            let vertical_menus = module
                .script_vertical_menus
                .values()
                .filter(|menu| menu.header_label == menu_id)
                .map(RuntimeVerticalMenuSnapshot::from_definition)
                .collect();
            return RuntimeMenuSnapshot::from_state(
                state,
                menu_id,
                RuntimeMenuSource::ScriptDefinition {
                    map_name: map_name.clone(),
                },
                Some(definition.clone()),
                vertical_menus,
            )
            .map(Some);
        }
        if self.data.special_routines.contains_key(&menu_id) {
            return RuntimeMenuSnapshot::from_state(
                state,
                menu_id,
                RuntimeMenuSource::SpecialRoutine,
                None,
                Vec::new(),
            )
            .map(Some);
        }
        match &state.overworld {
            OverworldMemory::Active { map_name, .. } => anyhow::bail!(
                "active runtime menu '{menu_id}' is not declared by current compiled map {map_name}"
            ),
            OverworldMemory::Inactive => anyhow::bail!(
                "active runtime menu '{menu_id}' requires an active overworld map or special routine"
            ),
        }
    }

    fn ui_snapshot(
        &self,
        state: &GameState,
        menu: Option<RuntimeMenuSnapshot>,
    ) -> Result<RuntimeUiSnapshot> {
        let text = self.active_text_snapshot(state)?;
        let elevators = self.elevator_snapshots(state);
        let gift_pokemon = self.gift_pokemon_snapshots(state);
        Ok(RuntimeUiSnapshot::from_state(
            state,
            menu,
            elevators,
            gift_pokemon,
            text,
        ))
    }

    fn elevator_snapshots(&self, state: &GameState) -> Vec<RuntimeElevatorSnapshot> {
        let OverworldMemory::Active { map_name, .. } = &state.overworld else {
            return Vec::new();
        };
        self.data
            .maps
            .get(map_name)
            .into_iter()
            .flat_map(|module| module.script_elevators.values())
            .map(|definition| RuntimeElevatorSnapshot::from_definition(map_name, definition))
            .collect()
    }

    fn gift_pokemon_snapshots(&self, state: &GameState) -> Vec<RuntimeGiftPokemonSnapshot> {
        let OverworldMemory::Active { map_name, .. } = &state.overworld else {
            return Vec::new();
        };
        self.data
            .maps
            .get(map_name)
            .into_iter()
            .flat_map(|module| module.gift_pokemon_scripts.iter())
            .map(|gift| RuntimeGiftPokemonSnapshot::from_script(map_name, gift))
            .collect()
    }

    fn active_text_snapshot(&self, state: &GameState) -> Result<Option<RuntimeTextSnapshot>> {
        if let Some(label) = state.script_runtime.pending_text_label.as_deref() {
            return self
                .text_snapshot_for_label(state, label)
                .map(Some)
                .with_context(|| format!("resolve runtime text snapshot for '{label}'"));
        }
        if !state.script_runtime.text_window_open {
            return Ok(None);
        }
        // Text events are retained as diagnostics, so the most recent Write
        // in the entire history is not necessarily the text owned by this
        // window. An Open after that Write starts an empty window; rendering
        // past it is the source of the bogus pre-text seen between Mom's
        // canonical lines (and after changing maps). Wait/YesNo retain the
        // current Write, while Open/Close explicitly delimit it.
        for event in state.script_runtime.text_events.iter().rev() {
            match event.kind {
                crate::core::state::ScriptTextRuntimeKind::Write => {
                    let label = event.text_label.as_deref().with_context(|| {
                        format!("runtime Write event {} has no text label", event.command)
                    })?;
                    return self.text_snapshot_for_label(state, label).map(Some);
                }
                crate::core::state::ScriptTextRuntimeKind::Open
                | crate::core::state::ScriptTextRuntimeKind::Close => return Ok(None),
                crate::core::state::ScriptTextRuntimeKind::WaitButton
                | crate::core::state::ScriptTextRuntimeKind::YesNo => {}
            }
        }
        Ok(None)
    }

    fn text_snapshot_for_label(
        &self,
        state: &GameState,
        label: &str,
    ) -> Result<RuntimeTextSnapshot> {
        if let OverworldMemory::Active { map_name, .. } = &state.overworld
            && let Some(module) = self.data.maps.get(map_name)
            && let Some(body) = module.script_text_bodies.get(label)
        {
            return Ok(RuntimeTextSnapshot {
                label: label.to_string(),
                source: RuntimeTextSource::ScriptBody {
                    map_name: map_name.clone(),
                },
                asm_text: None,
                body: Some(body.clone()),
                queued_text_events: state.script_runtime.text_events.len(),
            });
        }
        if let Some(module) = &self.data.global_scripts
            && let Some(body) = module.script_text_bodies.get(label)
        {
            return Ok(RuntimeTextSnapshot {
                label: label.to_string(),
                source: RuntimeTextSource::ScriptBody {
                    map_name: "GlobalScripts".to_string(),
                },
                asm_text: None,
                body: Some(body.clone()),
                queued_text_events: state.script_runtime.text_events.len(),
            });
        }
        if let Some(text) = self.data.asm_text.get(label) {
            return Ok(RuntimeTextSnapshot {
                label: label.to_string(),
                source: RuntimeTextSource::AsmText,
                asm_text: Some(text.clone()),
                body: None,
                queued_text_events: state.script_runtime.text_events.len(),
            });
        }
        if matches!(state.overworld, OverworldMemory::Inactive) {
            anyhow::bail!(
                "runtime UI script text label '{label}' requires an active overworld map"
            );
        }
        self.data
            .validate_saved_text_reference("runtime_ui.text.label", label)
            .with_context(|| format!("validate runtime UI text label '{label}'"))?;
        match &state.overworld {
            OverworldMemory::Active { map_name, .. } => anyhow::bail!(
                "runtime UI script text label '{label}' is not declared by current compiled map {map_name}"
            ),
            OverworldMemory::Inactive => anyhow::bail!(
                "runtime UI script text label '{label}' requires an active overworld map"
            ),
        }
    }

    fn bag_snapshot(&self, state: &GameState) -> Result<RuntimeBagSnapshot> {
        Ok(RuntimeBagSnapshot {
            items: RuntimeBagSnapshot::inventory(&state.bag.items),
            balls: RuntimeBagSnapshot::inventory(&state.bag.balls),
            key_items: RuntimeBagSnapshot::inventory(&state.bag.key_items),
            tm_hm: RuntimeBagSnapshot::tm_hm(&self.data.items, &state.bag.tm_hm)?,
            pc_items: RuntimeBagSnapshot::inventory(&state.bag.pc_items),
            custom_pockets: state
                .bag
                .custom_pockets
                .iter()
                .map(|(pocket_id, inventory)| {
                    (pocket_id.clone(), RuntimeBagSnapshot::inventory(inventory))
                })
                .collect(),
        })
    }

    fn item_catalog_snapshot(&self) -> Vec<RuntimeItemCatalogSnapshot> {
        self.data
            .items
            .iter()
            .map(|entry| RuntimeItemCatalogSnapshot::from_item(entry, &self.data.evolutions))
            .collect()
    }

    fn move_catalog_snapshot(&self) -> Vec<RuntimeMoveCatalogSnapshot> {
        self.data
            .moves
            .iter()
            .map(RuntimeMoveCatalogSnapshot::from_move)
            .collect()
    }

    fn pokemon_catalog_snapshot(&self) -> Vec<RuntimePokemonCatalogSnapshot> {
        let mut pokemon = self
            .data
            .pokemon
            .iter()
            .map(RuntimePokemonCatalogSnapshot::from_species)
            .collect::<Vec<_>>();
        pokemon.sort_by_key(|species| species.int_id);
        pokemon
    }

    fn trainer_catalog_snapshot(&self) -> Vec<RuntimeTrainerCatalogSnapshot> {
        self.data
            .trainers
            .trainers
            .values()
            .map(RuntimeTrainerCatalogSnapshot::from_trainer)
            .collect()
    }

    fn base_map_catalog_snapshot(data: &GameDataSet) -> Vec<Arc<RuntimeMapCatalogSnapshot>> {
        data.maps
            .iter()
            .map(|(map_name, module)| {
                let metadata = module
                    .attributes
                    .map_constant
                    .as_deref()
                    .and_then(|constant| data.runtime_map_metadata.get(constant));
                Arc::new(RuntimeMapCatalogSnapshot::from_module(
                    map_name, module, metadata,
                ))
            })
            .collect()
    }

    fn build_static_catalog_cache(&self) -> RuntimeStaticCatalogCache {
        RuntimeStaticCatalogCache {
            audio: Arc::new(self.audio_catalog_snapshot()),
            items: Arc::new(self.item_catalog_snapshot()),
            item_effect_plans: Arc::new(self.item_effect_plan_keys().into_iter().collect()),
            moves: Arc::new(self.move_catalog_snapshot()),
            pokemon: Arc::new(self.pokemon_catalog_snapshot()),
            trainers: Arc::new(self.trainer_catalog_snapshot()),
            spawn_points: Arc::new(self.data.runtime_spawn_points.values().cloned().collect()),
            tilesets: Arc::new(self.tileset_catalog_snapshot()),
            encounters: Arc::new(self.encounter_catalog_snapshot()),
            battle_rules: Arc::new(self.battle_rule_catalog_snapshot()),
            world_rules: Arc::new(self.world_rule_catalog_snapshot()),
            presentation: Arc::new(self.presentation_catalog_snapshot()),
            special: Arc::new(self.special_catalog_snapshot()),
            story: Arc::new(self.story_catalog_snapshot()),
            playability: Arc::new(self.playability_rules_snapshot()),
        }
    }

    fn static_catalog_cache(&self) -> &RuntimeStaticCatalogCache {
        self.catalog_cache
            .get_or_init(|| self.build_static_catalog_cache())
    }

    fn map_catalog_snapshot(
        &self,
        active_map: &crystal_core::world::map::OverworldMapData,
        state: &GameState,
    ) -> Vec<Arc<RuntimeMapCatalogSnapshot>> {
        self.map_catalog
            .iter()
            .map(|base| {
                let map_name = base.map_name.as_str();
                if active_map.name == map_name {
                    // The active OverworldSession carries callback/field-move
                    // block writes.  Rendering immutable pack blocks here
                    // erased those authoritative mutations, including the
                    // default Town Map in the player's upstairs bedroom.
                    let mut snapshot = (**base).clone();
                    snapshot.blocks.clone_from(&active_map.metatile_ids);
                    Arc::new(snapshot)
                } else if let Some(overrides) = state.map_block_overrides.get(map_name) {
                    // A connection can expose a neighboring map before it
                    // becomes the active session. Keep block writes from an
                    // earlier visit visible at that seam. `snapshot()` has
                    // already validated these coordinates against this map.
                    let mut snapshot = (**base).clone();
                    for ((x, y), block_id) in overrides {
                        let index = usize::from(*y) * usize::from(snapshot.attributes.width)
                            + usize::from(*x);
                        snapshot.blocks[index] = *block_id;
                    }
                    Arc::new(snapshot)
                } else {
                    Arc::clone(base)
                }
            })
            .collect()
    }

    fn tileset_catalog_snapshot(&self) -> Vec<RuntimeTilesetCatalogSnapshot> {
        self.data
            .tilesets
            .iter()
            .map(RuntimeTilesetCatalogSnapshot::from_tileset)
            .collect()
    }

    fn encounter_catalog_snapshot(&self) -> RuntimeEncounterCatalogSnapshot {
        RuntimeEncounterCatalogSnapshot::from_data(&self.data)
    }

    fn battle_rule_catalog_snapshot(&self) -> RuntimeBattleRuleCatalogSnapshot {
        RuntimeBattleRuleCatalogSnapshot::from_data(&self.data)
    }

    fn world_rule_catalog_snapshot(&self) -> RuntimeWorldRuleCatalogSnapshot {
        RuntimeWorldRuleCatalogSnapshot::from_data(&self.data)
    }

    fn presentation_catalog_snapshot(&self) -> RuntimePresentationCatalogSnapshot {
        RuntimePresentationCatalogSnapshot::from_data(&self.data)
    }

    fn special_catalog_snapshot(&self) -> RuntimeSpecialCatalogSnapshot {
        RuntimeSpecialCatalogSnapshot::from_data(&self.data)
    }

    fn story_catalog_snapshot(&self) -> RuntimeStoryCatalogSnapshot {
        RuntimeStoryCatalogSnapshot::from_data(&self.data)
    }

    fn audio_catalog_snapshot(&self) -> RuntimeAudioCatalogSnapshot {
        RuntimeAudioCatalogSnapshot::from_catalog(&self.audio)
    }

    fn playability_rules_snapshot(&self) -> crystal_assets::PlayabilityRules {
        self.data.playability.clone()
    }

    fn validate_save_state_for_runtime_pack(&self, state: &GameState) -> Result<()> {
        self.data.validate_save_currency(state)?;
        validate_save_references_for_runtime_pack(state, &self.data)
            .context("validate Crystal runtime save references against compiled pack")
    }

    pub fn boot_summary(&self) -> RuntimeBootSummary {
        RuntimeBootSummary {
            modpack_id: self.modpack.id().to_string(),
            modpack_hash: self.modpack.hash().to_string(),
            pack_content_hash: self.pack_identity.content_hash.clone(),
            pokemon_species: self.data.pokemon.len(),
            moves: self.data.moves.len(),
            maps: self.data.maps.len(),
            items: self.data.items.len(),
            wild_encounter_tables: self.data.wild_encounters.len(),
            music_tracks: self.audio.music_count(),
            sound_effects: self.audio.sound_effect_count(),
            cries: self.audio.cry_count(),
            viewport: self.viewport,
        }
    }

    fn start_overworld_session(
        &self,
        asset_root: &AssetRoot,
        spawn_identifier: u16,
    ) -> Result<RuntimeOverworldSession> {
        let spawn = self.data.runtime_spawn_point(spawn_identifier)?;
        RuntimeOverworldSession::new(self, asset_root, spawn)
    }

    #[cfg(any(test, feature = "location-tester"))]
    fn start_overworld_session_at_runtime_tile(
        &self,
        asset_root: &AssetRoot,
        map_name: &str,
        tile_x: i16,
        tile_y: i16,
    ) -> Result<RuntimeOverworldSession> {
        RuntimeOverworldSession::new_at_runtime_tile(
            self,
            asset_root,
            map_name,
            TilePosition::new(tile_x, tile_y),
        )
    }

    pub fn resume_overworld_session(
        &self,
        asset_root: &AssetRoot,
        state: GameState,
    ) -> Result<RuntimeOverworldSession> {
        RuntimeOverworldSession::from_state(self, asset_root, state)
    }
}

fn require_runtime_catalog_id(kind: &str, id: &str, exists: bool) -> Result<()> {
    if exists {
        Ok(())
    } else {
        anyhow::bail!("compiled game pack missing exact {kind} id {id}")
    }
}

fn field_move_move_ids(field_moves: &FieldMoveCatalog) -> BTreeSet<String> {
    [
        field_moves.cut.move_id.as_str(),
        field_moves.whirlpool.move_id.as_str(),
        field_moves.strength.move_id.as_str(),
        field_moves.flash.move_id.as_str(),
        field_moves.surf.move_id.as_str(),
        field_moves.waterfall.move_id.as_str(),
        field_moves.fly.move_id.as_str(),
        field_moves.dig.move_id.as_str(),
        field_moves.teleport.move_id.as_str(),
        field_moves.headbutt.move_id.as_str(),
        field_moves.rock_smash.move_id.as_str(),
        field_moves.sweet_scent.move_id.as_str(),
    ]
    .into_iter()
    .filter(|move_id| !move_id.is_empty())
    .map(str::to_string)
    .collect()
}

fn field_move_item_ids(field_moves: &FieldMoveCatalog) -> BTreeSet<String> {
    [
        field_moves.escape_rope.item_id.as_str(),
        field_moves.bicycle.item_id.as_str(),
        field_moves.itemfinder.item_id.as_str(),
        field_moves.squirtbottle.item_id.as_str(),
        field_moves.coin_case.item_id.as_str(),
        field_moves.blue_card.item_id.as_str(),
        field_moves.town_map.item_id.as_str(),
        field_moves.pokegear.item_id.as_str(),
    ]
    .into_iter()
    .filter(|item_id| !item_id.is_empty())
    .map(str::to_string)
    .collect()
}

fn fly_destination_keys(
    destinations: &BTreeMap<String, crystal_assets::FlyDestination>,
) -> BTreeSet<RuntimeFlyDestinationKey> {
    destinations
        .values()
        .map(|destination| RuntimeFlyDestinationKey {
            flypoint_flag: destination.flypoint_flag.clone(),
            destination_spawn_identifier: destination.destination_spawn_identifier,
            label: destination.label.clone(),
        })
        .collect()
}

fn field_move_rule_keys(field_moves: &FieldMoveCatalog) -> BTreeSet<RuntimeFieldMoveRuleKey> {
    [
        field_move_block_rule_key("cut", &field_moves.cut),
        field_move_block_rule_key("whirlpool", &field_moves.whirlpool),
        field_move_flag_rule_key("strength", &field_moves.strength),
        field_move_flag_rule_key("flash", &field_moves.flash),
        field_move_travel_rule_key("surf", &field_moves.surf),
        field_move_travel_rule_key("waterfall", &field_moves.waterfall),
        field_move_badged_rule_key("fly", &field_moves.fly),
        field_move_move_rule_key("dig", &field_moves.dig),
        field_move_move_rule_key("teleport", &field_moves.teleport),
        field_move_move_rule_key("headbutt", &field_moves.headbutt),
        field_move_move_rule_key("rock_smash", &field_moves.rock_smash),
        field_move_move_rule_key("sweet_scent", &field_moves.sweet_scent),
        field_escape_item_rule_key("escape_rope", &field_moves.escape_rope),
        RuntimeFieldMoveRuleKey {
            rule_id: "repel".to_string(),
            rule_kind: "repel_item".to_string(),
            move_id: None,
            item_id: None,
            badge_region: None,
            badge_index: None,
            engine_flag: None,
            escape_rope_mode: None,
            target_collisions: Vec::new(),
            blocked_collisions: Vec::new(),
            replacements: BTreeMap::new(),
        },
        field_item_rule_key("bicycle", &field_moves.bicycle),
        field_item_rule_key("itemfinder", &field_moves.itemfinder),
        field_item_rule_key("squirtbottle", &field_moves.squirtbottle),
        field_item_rule_key("coin_case", &field_moves.coin_case),
        field_item_rule_key("blue_card", &field_moves.blue_card),
        field_item_rule_key("town_map", &field_moves.town_map),
        field_item_rule_key("pokegear", &field_moves.pokegear),
    ]
    .into_iter()
    .collect()
}

fn field_move_badge_parts(badge: &FieldMoveBadgeRequirement) -> (Option<String>, Option<usize>) {
    (Some(badge.region.clone()), Some(badge.index))
}

fn field_move_replacement_keys(
    replacements: &BTreeMap<String, BTreeMap<u16, FieldMoveReplacement>>,
) -> BTreeMap<String, BTreeMap<u16, RuntimeFieldMoveReplacementKey>> {
    replacements
        .iter()
        .map(|(tileset_id, replacements)| {
            (
                tileset_id.clone(),
                replacements
                    .iter()
                    .map(|(block_id, replacement)| {
                        (
                            *block_id,
                            RuntimeFieldMoveReplacementKey {
                                replacement_block_id: replacement.replacement_block_id,
                                variant: replacement.variant.clone(),
                            },
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

fn field_move_block_rule_key(rule_id: &str, rule: &FieldMoveBlockRule) -> RuntimeFieldMoveRuleKey {
    let (badge_region, badge_index) = field_move_badge_parts(&rule.badge);
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "block".to_string(),
        move_id: Some(rule.move_id.clone()),
        item_id: None,
        badge_region,
        badge_index,
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: rule.target_collisions.clone(),
        blocked_collisions: Vec::new(),
        replacements: field_move_replacement_keys(&rule.replacements),
    }
}

fn field_move_flag_rule_key(rule_id: &str, rule: &FieldMoveFlagRule) -> RuntimeFieldMoveRuleKey {
    let (badge_region, badge_index) = field_move_badge_parts(&rule.badge);
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "flag".to_string(),
        move_id: Some(rule.move_id.clone()),
        item_id: None,
        badge_region,
        badge_index,
        engine_flag: Some(rule.engine_flag.clone()),
        escape_rope_mode: None,
        target_collisions: Vec::new(),
        blocked_collisions: Vec::new(),
        replacements: BTreeMap::new(),
    }
}

fn field_move_travel_rule_key(
    rule_id: &str,
    rule: &FieldMoveTravelRule,
) -> RuntimeFieldMoveRuleKey {
    let (badge_region, badge_index) = field_move_badge_parts(&rule.badge);
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "travel".to_string(),
        move_id: Some(rule.move_id.clone()),
        item_id: None,
        badge_region,
        badge_index,
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: rule.target_collisions.clone(),
        blocked_collisions: rule.blocked_collisions.clone(),
        replacements: BTreeMap::new(),
    }
}

fn field_move_badged_rule_key(rule_id: &str, rule: &FieldMoveRule) -> RuntimeFieldMoveRuleKey {
    let (badge_region, badge_index) = field_move_badge_parts(&rule.badge);
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "badged_move".to_string(),
        move_id: Some(rule.move_id.clone()),
        item_id: None,
        badge_region,
        badge_index,
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: Vec::new(),
        blocked_collisions: Vec::new(),
        replacements: BTreeMap::new(),
    }
}

fn field_move_move_rule_key(rule_id: &str, rule: &FieldMoveMoveRule) -> RuntimeFieldMoveRuleKey {
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "move".to_string(),
        move_id: Some(rule.move_id.clone()),
        item_id: None,
        badge_region: None,
        badge_index: None,
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: rule.target_collisions.clone(),
        blocked_collisions: Vec::new(),
        replacements: BTreeMap::new(),
    }
}

fn field_escape_item_rule_key(
    rule_id: &str,
    rule: &FieldEscapeItemRule,
) -> RuntimeFieldMoveRuleKey {
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "escape_item".to_string(),
        move_id: None,
        item_id: Some(rule.item_id.clone()),
        badge_region: None,
        badge_index: None,
        engine_flag: None,
        escape_rope_mode: Some(rule.escape_rope_mode.clone()),
        target_collisions: Vec::new(),
        blocked_collisions: Vec::new(),
        replacements: BTreeMap::new(),
    }
}

fn field_item_rule_key(rule_id: &str, rule: &FieldItemRule) -> RuntimeFieldMoveRuleKey {
    RuntimeFieldMoveRuleKey {
        rule_id: rule_id.to_string(),
        rule_kind: "item".to_string(),
        move_id: None,
        item_id: Some(rule.item_id.clone()),
        badge_region: None,
        badge_index: None,
        engine_flag: None,
        escape_rope_mode: None,
        target_collisions: Vec::new(),
        blocked_collisions: Vec::new(),
        replacements: BTreeMap::new(),
    }
}

fn collect_wild_encounter_keys(
    map_name: &str,
    encounters: &WildEncounterData,
    keys: &mut BTreeSet<RuntimeWildEncounterOriginKey>,
) {
    if let Some(table) = &encounters.grass {
        for encounter in table
            .morning
            .iter()
            .chain(table.day.iter())
            .chain(table.night.iter())
        {
            keys.insert(RuntimeWildEncounterOriginKey {
                map_name: map_name.to_string(),
                species: encounter.species.clone(),
                level: encounter.level,
            });
        }
    }
    if let Some(table) = &encounters.water {
        for encounter in table
            .morning
            .iter()
            .chain(table.day.iter())
            .chain(table.night.iter())
        {
            keys.insert(RuntimeWildEncounterOriginKey {
                map_name: map_name.to_string(),
                species: encounter.species.clone(),
                level: encounter.level,
            });
        }
    }
}

fn collect_field_encounter_keys(
    map_name: &str,
    encounters: &FieldEncounterData,
    keys: &mut BTreeSet<RuntimeWildEncounterOriginKey>,
) {
    for table in encounters.tables.values() {
        for encounter in table.common.iter().chain(table.rare.iter()) {
            keys.insert(RuntimeWildEncounterOriginKey {
                map_name: map_name.to_string(),
                species: encounter.species.clone(),
                level: encounter.level,
            });
        }
    }
}

fn collect_fishing_encounter_keys(
    map_name: &str,
    group: &crystal_core::world::fishing::FishingGroup,
    time_groups: &BTreeMap<String, crystal_core::world::fishing::TimeFishEntry>,
    keys: &mut BTreeSet<RuntimeWildEncounterOriginKey>,
) {
    for slot in group
        .rod_tables
        .values()
        .flat_map(|table| table.slots.iter())
    {
        if let Some(species) = &slot.species {
            keys.insert(RuntimeWildEncounterOriginKey {
                map_name: map_name.to_string(),
                species: species.clone(),
                level: slot.level,
            });
        }
        if let Some(time_group) = slot
            .time_group
            .as_ref()
            .and_then(|time_group| time_groups.get(time_group))
        {
            keys.insert(RuntimeWildEncounterOriginKey {
                map_name: map_name.to_string(),
                species: time_group.day_species.clone(),
                level: time_group.day_level,
            });
            keys.insert(RuntimeWildEncounterOriginKey {
                map_name: map_name.to_string(),
                species: time_group.night_species.clone(),
                level: time_group.night_level,
            });
        }
    }
}

impl RuntimeOverworldSession {
    fn new(
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        spawn: &RuntimeSpawnPoint,
    ) -> Result<Self> {
        let (state, overworld) = runtime
            .data
            .start_overworld_session_from_spawn(spawn, &runtime.audio.music_ids())?;
        let mut state = state;
        state.last_spawn_identifier = Some(spawn.identifier);
        Ok(Self {
            state,
            overworld,
            joypad: JoypadState::new(),
            divider: RuntimeDividerSource::live(),
        })
    }

    #[cfg(any(test, feature = "location-tester"))]
    fn new_at_runtime_tile(
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        map_name: &str,
        tile: TilePosition,
    ) -> Result<Self> {
        let (state, overworld) = runtime.data.start_overworld_session_at_runtime_tile(
            map_name,
            tile,
            &runtime.audio.music_ids(),
        )?;
        Ok(Self {
            state,
            overworld,
            joypad: JoypadState::new(),
            divider: RuntimeDividerSource::live(),
        })
    }

    fn from_state(
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        state: GameState,
    ) -> Result<Self> {
        let (state, overworld) = runtime
            .data
            .resume_overworld_session_from_state(state, &runtime.audio.music_ids())?;
        Ok(Self {
            joypad: JoypadState::from_previous_mask(state.joypad.h_joy_down),
            state,
            overworld,
            divider: RuntimeDividerSource::live(),
        })
    }

    fn stage_overworld_input(
        &mut self,
        runtime: &CrystalRuntime,
        buttons: Vec<GameButton>,
        checksum: bool,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut divider_after = self.divider.clone();
        let mut recording = RecordingDivider::new(&mut divider_after);
        let frame = runtime.data.apply_overworld_input(
            &mut state,
            &mut overworld,
            buttons.clone(),
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        drop(recording);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::OverworldInputApplied(frame),
            state_checksum: if checksum {
                game_state_checksum(&state)?
            } else {
                StateChecksum::new(state.frame_counter, 0)
            },
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::ApplyOverworldInput(RuntimeOverworldInputCommand {
                buttons,
                divider_trace,
            }),
            state,
            overworld,
            outcome,
            divider_after: Some(divider_after),
        })
    }

    /// Apply a real-time host frame without constructing a second outer
    /// transactional copy or a replay checksum. `GameDataSet` still owns the
    /// single atomic gameplay transaction; the shell no longer duplicates
    /// that already-staged state solely to serialize a disabled journal.
    fn apply_overworld_input_live(
        &mut self,
        runtime: &CrystalRuntime,
        buttons: Vec<GameButton>,
    ) -> Result<RuntimeOverworldFrame> {
        let frame = runtime.data.apply_overworld_input(
            &mut self.state,
            &mut self.overworld,
            buttons,
            &runtime.music_ids(),
            &mut self.divider,
        )?;
        self.joypad = JoypadState::from_previous_mask(frame.input_mask);
        Ok(RuntimeOverworldFrame::from_input_frame(
            frame,
            StateChecksum::new(self.state.frame_counter, 0),
        ))
    }

    fn stage_sweet_scent_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        overworld.set_time_of_day(state.time.time_of_day);
        let mut divider_after = self.divider.clone();
        let mut recording = RecordingDivider::new(&mut divider_after);
        let result = {
            let mut rng =
                crystal_core::random::CrystalRandom::new(state.random_state, &mut recording);
            runtime.data.use_sweet_scent_field_move(
                &mut state,
                &overworld,
                party_index,
                &mut rng,
            )?
        };
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        drop(recording);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::SweetScentFieldMoveUsed(result),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::UseSweetScentFieldMove(
                RuntimeSweetScentFieldMoveCommand {
                    party_index,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: Some(divider_after),
        })
    }

    fn stage_random_special_routine(
        &mut self,
        runtime: &CrystalRuntime,
        routine: &str,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut divider_after = self.divider.clone();
        let mut recording = RecordingDivider::new(&mut divider_after);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            routine,
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        drop(recording);
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let mutation_outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::SpecialRoutineApplied(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::ApplyRandomSpecialRoutine(
                RuntimeRandomSpecialRoutineCommand {
                    routine: routine.to_string(),
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome: mutation_outcome,
            divider_after: Some(divider_after),
        })
    }

    fn stage_rock_mon_encounter(
        &mut self,
        runtime: &CrystalRuntime,
        command: RuntimeScriptCommandRef,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let current_map = overworld.map.name.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let resolved = runtime.data.resolve_rock_mon_encounter(
            &mut state,
            &current_map,
            &command,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::RockMonEncounterResolved(resolved),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::ResolveRockMonEncounter(
                RuntimeRockMonEncounterCommand {
                    command,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_scripted_wild_battle_start(
        &mut self,
        runtime: &CrystalRuntime,
        command: RuntimeScriptCommandRef,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let start = runtime.data.start_scripted_wild_battle_in_session(
            &mut state,
            &overworld,
            &command.map_name,
            &command.source_script,
            command.command_index,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ScriptedWildBattleStarted(start),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::StartScriptedWildBattle(
                RuntimeScriptedWildBattleStartCommand {
                    command,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_active_wild_capture_completion(
        &mut self,
        runtime: &CrystalRuntime,
        capture_outcome: &CaptureOutcome,
        nickname: Option<String>,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let completion = runtime.data.complete_active_wild_capture(
            &mut state,
            capture_outcome,
            nickname.as_deref(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ActiveWildCaptureCompleted(completion),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::CompleteActiveWildCapture(
                RuntimeCaptureCompletionCommand {
                    outcome: capture_outcome.clone(),
                    nickname,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_phone_random_special(
        &mut self,
        runtime: &CrystalRuntime,
        special: RuntimePhoneRandomSpecial,
        contact_id: String,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        state
            .script_runtime
            .variables
            .insert("VAR_CALLERID".to_string(), contact_id.clone());
        let mut recording = RecordingDivider::new(&mut self.divider);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            special.routine(),
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let mutation_outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::PhoneRandomSpecialApplied(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::ApplyPhoneRandomSpecial(RuntimePhoneCallerCommand {
                special,
                contact_id,
                divider_trace,
            }),
            state,
            overworld,
            outcome: mutation_outcome,
            divider_after: None,
        })
    }

    fn stage_random_bug_contest(
        &mut self,
        runtime: &CrystalRuntime,
        action: RuntimeBugContestAction,
    ) -> Result<RecordedRuntimeMutation> {
        let routine = match action {
            RuntimeBugContestAction::SelectContestants => "SelectRandomBugContestContestants",
            RuntimeBugContestAction::Judge => "BugContestJudging",
            _ => anyhow::bail!("Bug Contest exact RNG staging requested for {action:?}"),
        };
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        state.script_runtime.variables.remove("_bug_contest_rank");
        let mut divider_after = self.divider.clone();
        let mut recording = RecordingDivider::new(&mut divider_after);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            routine,
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        drop(recording);
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let mutation_outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::BugContestUsed(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::UseBugContest(RuntimeBugContestCommand {
                action,
                divider_trace: Some(divider_trace),
            }),
            state,
            overworld,
            outcome: mutation_outcome,
            divider_after: Some(divider_after),
        })
    }

    fn stage_buena_password(
        &mut self,
        runtime: &CrystalRuntime,
        guess: Option<String>,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        match guess.as_deref() {
            Some(guess) => {
                state
                    .script_runtime
                    .variables
                    .insert("BUENA_PASSWORD".to_string(), guess.to_string());
            }
            None => {
                state.script_runtime.variables.remove("BUENA_PASSWORD");
            }
        }
        let mut recording = RecordingDivider::new(&mut self.divider);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            "BuenasPassword",
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::BuenaPasswordUsed(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::UseBuenaPassword(RuntimeBuenaPasswordCommand {
                guess,
                divider_trace,
            }),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_shuckie_give(&mut self, runtime: &CrystalRuntime) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            "GiveShuckle",
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ShuckieUsed(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::UseShuckie(RuntimeShuckieCommand {
                action: RuntimeShuckieAction::Give,
                party_index: None,
                divider_trace: Some(divider_trace),
            }),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_odd_egg(&mut self, runtime: &CrystalRuntime) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            "GiveOddEgg",
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::OddEggGiven(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::GiveOddEgg(RuntimeOddEggCommand { divider_trace }),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_battle_tower_opponent(
        &mut self,
        runtime: &CrystalRuntime,
        trainer_id: String,
        sprite_constant: String,
        target_object: String,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        state
            .script_runtime
            .variables
            .insert("_battle_tower_trainer_id".to_string(), trainer_id.clone());
        state.script_runtime.variables.insert(
            "_battle_tower_sprite_constant".to_string(),
            sprite_constant.clone(),
        );
        state.script_runtime.variables.insert(
            "_battle_tower_target_object".to_string(),
            target_object.clone(),
        );
        let mut recording = RecordingDivider::new(&mut self.divider);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            "LoadOpponentTrainerAndPokemonWithOTSprite",
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::BattleTowerOpponentLoaded(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::LoadBattleTowerOpponentSpecial(
                RuntimeBattleTowerOpponentCommand {
                    trainer_id,
                    sprite_constant,
                    target_object,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_card_flip(&mut self, runtime: &CrystalRuntime) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let outcome = runtime.data.apply_random_special_routine(
            &mut state,
            "CardFlip",
            &runtime.music_ids(),
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::GameCornerOpened(outcome),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::OpenGameCornerSpecial(RuntimeGameCornerCommand {
                service: RuntimeGameCornerService::CardFlip,
                divider_trace: Some(divider_trace),
            }),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_scripted_wild_battle_completion(
        &mut self,
        runtime: &CrystalRuntime,
        origin: RuntimeStaticWildBattleOrigin,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let current_map = overworld.map.name.clone();
        let terminal = runtime
            .data
            .scripted_wild_battle_terminal(&state, &origin)?;
        let mut recording = RecordingDivider::new(&mut self.divider);
        runtime.data.complete_scripted_wild_battle(
            &mut state,
            &mut overworld,
            &current_map,
            &origin,
            terminal,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ScriptedWildBattleCompleted,
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::CompleteScriptedWildBattle(
                RuntimeScriptedWildBattleCompletionCommand {
                    origin,
                    terminal,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_scripted_trainer_battle_completion(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        won: bool,
        can_lose: bool,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let current_map = self.overworld.map.name.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        let completion = runtime.data.complete_scripted_trainer_battle(
            &mut state,
            &current_map,
            map_name,
            source_script,
            command_index,
            won,
            can_lose,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ScriptedTrainerBattleCompleted(completion),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::CompleteScriptedTrainerBattle(
                RuntimeTrainerBattleCompletionCommand {
                    command: Self::script_command_ref(map_name, source_script, command_index),
                    won,
                    can_lose,
                    divider_trace,
                },
            ),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_wild_battle_rewards(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let time_of_day = state.time.time_of_day;
        let mut recording = RecordingDivider::new(&mut self.divider);
        let rewards = runtime.data.claim_active_wild_battle_rewards(
            &mut state,
            time_of_day,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ActiveWildBattleRewardsClaimed(rewards),
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::ClaimActiveWildBattleRewardsNow(divider_trace),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_clock_update(
        &mut self,
        runtime: &CrystalRuntime,
        date: GameDate,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        runtime.data.update_clock_from_datetime(
            &mut state,
            date,
            hour,
            minute,
            second,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ClockUpdated,
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::UpdateClockFromDatetime(RuntimeClockUpdateCommand {
                date,
                hour,
                minute,
                second,
                divider_trace,
            }),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn stage_manual_clock_update(
        &mut self,
        runtime: &CrystalRuntime,
        now_date: GameDate,
        now_hour: u8,
        now_minute: u8,
        now_second: u8,
        target: ClockTime,
    ) -> Result<RecordedRuntimeMutation> {
        let mut state = self.state.clone();
        let mut overworld = self.overworld.clone();
        let mut recording = RecordingDivider::new(&mut self.divider);
        runtime.data.set_manual_clock_time(
            &mut state,
            now_date,
            now_hour,
            now_minute,
            now_second,
            target,
            &mut recording,
        )?;
        let divider_trace = RuntimeDividerTrace::new(recording.samples().iter().copied());
        overworld.set_time_of_day(state.time.time_of_day);
        overworld.sync_event_flag_memory(&state.flags);
        let outcome = RuntimeMutationOutcome {
            result: RuntimeMutationResult::ManualClockSet,
            state_checksum: game_state_checksum(&state)?,
        };
        Ok(RecordedRuntimeMutation {
            command: RuntimeMutationCommand::SetManualClockTime(RuntimeManualClockCommand {
                now_date,
                now_hour,
                now_minute,
                now_second,
                target,
                divider_trace,
            }),
            state,
            overworld,
            outcome,
            divider_after: None,
        })
    }

    fn commit_recorded_mutation(
        &mut self,
        recorded: RecordedRuntimeMutation,
    ) -> RuntimeMutationOutcome {
        self.state = recorded.state;
        self.overworld = recorded.overworld;
        if let Some(divider_after) = recorded.divider_after {
            self.divider = divider_after;
        }
        recorded.outcome
    }

    pub fn apply_buttons(
        &mut self,
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        buttons: impl IntoIterator<Item = GameButton>,
    ) -> Result<RuntimeOverworldFrame> {
        let recorded = self.stage_overworld_input(runtime, buttons.into_iter().collect(), true)?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::OverworldInputApplied(frame) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-overworld-input result");
        };
        self.joypad = JoypadState::from_previous_mask(frame.input_mask);
        Ok(RuntimeOverworldFrame::from_input_frame(
            frame,
            mutation.state_checksum,
        ))
    }

    pub fn dispatch_interaction_script(
        &mut self,
        runtime: &CrystalRuntime,
        interaction: &OverworldInteraction,
    ) -> Result<RuntimeInteractionScriptDispatch> {
        let interaction = runtime
            .data
            .resolve_overworld_interaction_dispatch(&self.state, interaction)?
            .with_context(|| {
                format!(
                    "background interaction {}:{} is no longer eligible",
                    interaction.map_name, interaction.script
                )
            })?;
        if interaction.map_name != self.overworld.map.name {
            anyhow::bail!(
                "interaction script {} belongs to map {} but active overworld map is {}",
                interaction.script,
                interaction.map_name,
                self.overworld.map.name
            );
        }
        runtime.require_map(&interaction.map_name)?;
        if !runtime.has_script_label(&interaction.script)
            && !crate::core::world::collision::is_standard_interaction_script(&interaction.script)
        {
            runtime.require_script_label(&interaction.script)?;
        }
        let last_talked_object = match &interaction.target {
            OverworldInteractionTarget::Object {
                object_identifier, ..
            } => object_identifier.as_deref(),
            OverworldInteractionTarget::Background { .. }
            | OverworldInteractionTarget::Collision { .. } => None,
        };
        let dispatch = commit_interaction_script_dispatch(
            &mut self.state,
            &mut self.overworld.last_talked_object_identifier,
            &interaction.map_name,
            &interaction.script,
            last_talked_object,
        )
        .with_context(|| {
            format!(
                "dispatch interaction script {} on {}",
                interaction.script, interaction.map_name
            )
        })?;
        if let OverworldInteractionTarget::Object {
            object_identifier: Some(object_id),
            ..
        } = &interaction.target
        {
            let facing = match interaction.facing {
                Direction::Up => Direction::Down,
                Direction::Down => Direction::Up,
                Direction::Left => Direction::Right,
                Direction::Right => Direction::Left,
            };
            self.overworld
                .object_facings
                .insert(object_id.clone(), facing);
        }
        commit_overworld_snapshot(
            &mut self.state,
            &self.overworld.snapshot(),
            SpawnMemoryUpdate::Preserve,
        );
        self.overworld.set_time_of_day(self.state.time.time_of_day);
        Ok(RuntimeInteractionScriptDispatch {
            next_script: dispatch.next_script,
            last_talked_object: dispatch.last_talked_object,
            state_checksum: game_state_checksum(&self.state)?,
        })
    }

    pub fn dispatch_coord_event_script(
        &mut self,
        runtime: &CrystalRuntime,
        coord_event: &CoordEventTrigger,
    ) -> Result<RuntimeInteractionScriptDispatch> {
        if coord_event.map_name != self.overworld.map.name {
            anyhow::bail!(
                "coord event script {} belongs to map {} but active overworld map is {}",
                coord_event.script_name,
                coord_event.map_name,
                self.overworld.map.name
            );
        }
        runtime.require_map(&coord_event.map_name)?;
        runtime.require_script_label(&coord_event.script_name)?;
        let dispatch = commit_interaction_script_dispatch(
            &mut self.state,
            &mut self.overworld.last_talked_object_identifier,
            &coord_event.map_name,
            &coord_event.script_name,
            None,
        )
        .with_context(|| {
            format!(
                "dispatch coord event script {} on {} at ({}, {})",
                coord_event.script_name,
                coord_event.map_name,
                coord_event.tile.x,
                coord_event.tile.y
            )
        })?;
        commit_overworld_snapshot(
            &mut self.state,
            &self.overworld.snapshot(),
            SpawnMemoryUpdate::Preserve,
        );
        self.overworld.set_time_of_day(self.state.time.time_of_day);
        Ok(RuntimeInteractionScriptDispatch {
            next_script: dispatch.next_script,
            last_talked_object: dispatch.last_talked_object,
            state_checksum: game_state_checksum(&self.state)?,
        })
    }

    pub fn snapshot(&self) -> OverworldSnapshot {
        self.overworld.snapshot()
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut GameState {
        &mut self.state
    }

    pub fn overworld(&self) -> &OverworldSession {
        &self.overworld
    }

    pub fn state_checksum_frame(&self, player_id: PlayerId) -> Result<StateChecksumFrame> {
        StateChecksumFrame::from_game_state(player_id, &self.state)
            .context("checksum authoritative GameState for player")
    }

    pub fn runtime_command_frame(
        &self,
        player_id: PlayerId,
        sequence: u64,
        command: RuntimeMutationCommand,
    ) -> Result<RuntimeCommandFrame> {
        runtime_mutation_command_frame(player_id, sequence, &command, &self.state)
    }

    pub fn require_runtime_command_expected_state(
        &self,
        request: &RuntimeCommandFrame,
    ) -> Result<()> {
        decode_runtime_mutation_command_frame(request, &self.state).map(|_| ())
    }

    pub fn runtime_mutation_result_frame(
        &self,
        request: RuntimeCommandFrame,
        outcome: &RuntimeMutationOutcome,
    ) -> Result<RuntimeCommandResultFrame> {
        assets_runtime_mutation_result_frame(request, outcome, &self.state)
    }

    pub fn apply_runtime_mutation_command(
        &mut self,
        runtime: &CrystalRuntime,
        command: RuntimeMutationCommand,
    ) -> Result<RuntimeMutationOutcome> {
        runtime
            .data
            .apply_runtime_mutation_command(
                &mut self.state,
                &mut self.overworld,
                command,
                &runtime.audio.music_ids(),
                &runtime.audio.sound_effect_ids(),
                &runtime.audio.cry_ids(),
            )
            .context("apply runtime mutation command")
    }

    pub fn apply_runtime_command_frame(
        &mut self,
        runtime: &CrystalRuntime,
        request: &RuntimeCommandFrame,
    ) -> Result<RuntimeMutationOutcome> {
        let command = decode_runtime_mutation_command_frame(request, &self.state)?;
        self.apply_runtime_mutation_command(runtime, command)
    }

    fn script_command_ref(
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> RuntimeScriptCommandRef {
        RuntimeScriptCommandRef::new(map_name, source_script, command_index)
    }

    fn runtime_time_update(&self, context: &'static str) -> Result<RuntimeTimeUpdate> {
        let state_checksum = game_state_checksum(&self.state).context(context)?;
        Ok(self.runtime_time_update_with_checksum(state_checksum))
    }

    fn runtime_time_update_with_checksum(
        &self,
        state_checksum: StateChecksum,
    ) -> RuntimeTimeUpdate {
        RuntimeTimeUpdate {
            time_of_day: self.state.time.time_of_day,
            day_of_week: self.state.time.day_of_week,
            hour: self.state.time.registers.hours,
            minute: self.state.time.registers.minutes,
            state_checksum,
        }
    }

    pub fn start_scripted_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<StaticWildBattleStart> {
        let recorded = self.stage_scripted_wild_battle_start(
            runtime,
            Self::script_command_ref(map_name, source_script, startbattle_command_index),
        )?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ScriptedWildBattleStarted(start) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-wild-battle-start result");
        };
        Ok(start)
    }

    pub fn start_scripted_trainer_battle(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<TrainerBattleStartStatus> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::StartScriptedTrainerBattle(Self::script_command_ref(
                map_name,
                source_script,
                startbattle_command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptedTrainerBattleStarted(start) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-trainer-battle-start result");
        };
        Ok(start)
    }

    pub fn complete_scripted_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        origin: RuntimeStaticWildBattleOrigin,
    ) -> Result<RuntimeScriptedBattleCompletion> {
        let recorded = self.stage_scripted_wild_battle_completion(runtime, origin)?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ScriptedWildBattleCompleted = mutation.result else {
            anyhow::bail!("runtime mutation returned non-scripted-wild-battle-completion result");
        };
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle: true,
            trainer_prize_money: None,
            money_after: None,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn complete_scripted_trainer_battle(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
        won: bool,
        can_lose: bool,
    ) -> Result<RuntimeScriptedBattleCompletion> {
        let recorded = self.stage_scripted_trainer_battle_completion(
            runtime,
            map_name,
            source_script,
            startbattle_command_index,
            won,
            can_lose,
        )?;
        let completion_mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ScriptedTrainerBattleCompleted(completion_outcome) =
            completion_mutation.result
        else {
            anyhow::bail!(
                "runtime mutation returned non-scripted-trainer-battle-completion result"
            );
        };
        let continued_after_battle = completion_outcome.continued_after_battle;
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle,
            trainer_prize_money: Some(completion_outcome.prize_money),
            money_after: Some(completion_outcome.money_after),
            state_checksum: completion_mutation.state_checksum,
        })
    }

    pub fn throw_ball_at_active_battle(
        &mut self,
        runtime: &CrystalRuntime,
        ball_id: &str,
    ) -> Result<RuntimeCaptureAttempt> {
        let rng_seed_after =
            preview_throw_ball_rng_seed_after(runtime.data(), &self.state, ball_id)?;
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ThrowBallAtActiveBattle(RuntimeBattleItemCommand {
                item_id: ball_id.to_string(),
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::BallThrown(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-capture result");
        };
        Ok(RuntimeCaptureAttempt {
            outcome: Some(outcome),
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn complete_active_wild_capture(
        &mut self,
        runtime: &CrystalRuntime,
        outcome: &CaptureOutcome,
        nickname: Option<String>,
    ) -> Result<RuntimeCaptureCompletion> {
        let recorded = self.stage_active_wild_capture_completion(runtime, outcome, nickname)?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ActiveWildCaptureCompleted(CaptureCompletion {
            stored,
            contest_pokemon,
        }) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-capture-completion result");
        };
        Ok(RuntimeCaptureCompletion {
            stored,
            contest_pokemon,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn resolve_bug_contest_caught_mon(
        &mut self,
        runtime: &CrystalRuntime,
        keep_new: bool,
    ) -> Result<RuntimeSpecialRoutineUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ResolveBugContestCaughtMon { keep_new },
        )?;
        let RuntimeMutationResult::SpecialRoutineApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-Bug Contest decision result");
        };
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn resolve_active_battle_turn(
        &mut self,
        runtime: &CrystalRuntime,
        player_action: BattleAction,
        enemy_action: BattleAction,
        rng_seed_after: u32,
    ) -> Result<RuntimeBattleTurn> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ResolveActiveBattleTurn(RuntimeBattleTurnCommand {
                player_action,
                enemy_action,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::ActiveBattleTurnResolved(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-turn result");
        };
        Ok(RuntimeBattleTurn {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn resolve_active_battle_command(
        &mut self,
        runtime: &CrystalRuntime,
        player_action: BattleAction,
        enemy_action: BattleAction,
        rng_seed_after: u32,
    ) -> Result<RuntimeBattleCommand> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ResolveActiveBattleCommand(RuntimeBattleTurnCommand {
                player_action,
                enemy_action,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::ActiveBattleCommandResolved(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-command result");
        };
        Ok(RuntimeBattleCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn resolve_active_battle_enemy_action(
        &mut self,
        runtime: &CrystalRuntime,
        enemy_action: BattleAction,
        rng_seed_after: u32,
    ) -> Result<RuntimeBattleTurn> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ResolveActiveBattleEnemyAction(
                RuntimeBattleEnemyActionCommand {
                    enemy_action,
                    rng_seed_after,
                },
            ),
        )?;
        let RuntimeMutationResult::ActiveBattleEnemyActionResolved(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-enemy-battle-action result");
        };
        Ok(RuntimeBattleTurn {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn attempt_escape_active_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeBattleEscape> {
        let rng_seed_after =
            preview_active_wild_battle_escape_rng_seed_after(runtime.data(), &self.state)?;
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::AttemptEscapeActiveWildBattle(RuntimeBattleEscapeCommand {
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::ActiveWildBattleEscapeAttempted(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-battle-escape result");
        };
        Ok(RuntimeBattleEscape {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_to_escape_active_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBattleEscapeItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemToEscapeActiveWildBattle(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::ActiveWildBattleEscapeItemUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-escape-item result");
        };
        Ok(RuntimeBattleEscapeItemUse {
            item_use: outcome.item_use,
            battle_escape_mode: outcome.battle_escape_mode,
            escaped: outcome.escaped,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_guard_spec_in_active_battle(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBattleStateItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagGuardSpecInActiveBattle(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::ActiveBattleGuardSpecUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-battle-state-item result");
        };
        Ok(RuntimeBattleStateItemUse {
            item_use: outcome.item_use,
            stat_drop_guard_turns_before: outcome.stat_drop_guard_turns_before,
            stat_drop_guard_turns_after: outcome.stat_drop_guard_turns_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn switch_active_battle_party(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeBattlePartySwitch> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::SwitchActiveBattleParty(RuntimePartySlotCommand {
                party_index,
            }),
        )?;
        let RuntimeMutationResult::ActiveBattlePartySwitched(party_index) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-active-battle-party-switch result");
        };
        Ok(RuntimeBattlePartySwitch {
            party_index,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn advance_active_trainer_battle(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeTrainerBattleAdvance> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::AdvanceActiveTrainerBattle,
        )?;
        let RuntimeMutationResult::ActiveTrainerBattleAdvanced(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-trainer-battle-advance result");
        };
        Ok(RuntimeTrainerBattleAdvance {
            next_enemy: outcome.next_enemy,
            trainer_defeated: outcome.trainer_defeated,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn claim_active_trainer_battle_rewards(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeBattleRewards> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ClaimActiveTrainerBattleRewardsNow,
        )?;
        let RuntimeMutationResult::ActiveTrainerBattleRewardsClaimed(outcome) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-trainer-rewards result");
        };
        Ok(RuntimeBattleRewards {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn claim_active_wild_battle_rewards(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeBattleRewards> {
        let recorded = self.stage_wild_battle_rewards(runtime)?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ActiveWildBattleRewardsClaimed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-wild-rewards result");
        };
        Ok(RuntimeBattleRewards {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn grant_scripted_gift_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        original_trainer_name: impl Into<String>,
        original_trainer_id: u16,
        dvs: Dv,
        rng_seed_after: u32,
        nickname_accepted: bool,
        nickname: Option<String>,
    ) -> Result<RuntimeGiftPokemonGrant> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::GrantScriptedGiftPokemon(RuntimeGiftPokemonCommand {
                command: RuntimeScriptCommandRef::new(map_name, source_script, command_index),
                original_trainer_name: original_trainer_name.into(),
                original_trainer_id,
                dvs,
                rng_seed_after,
                nickname_accepted,
                nickname,
            }),
        )?;
        let RuntimeMutationResult::ScriptedGiftPokemonGranted(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-gift-Pokemon result");
        };
        Ok(RuntimeGiftPokemonGrant {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        context: ItemUseContext,
    ) -> Result<RuntimeItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItem {
                item_id: item_id.to_string(),
                context,
            },
        )?;
        let RuntimeMutationResult::BagItemUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-item-use result");
        };
        Ok(RuntimeItemUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn register_key_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeRegisteredKeyItem> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::RegisterKeyItem(RuntimeRegisteredKeyItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::KeyItemRegistered(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-key-item-registration result");
        };
        Ok(RuntimeRegisteredKeyItem {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_repel_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeRepelItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagRepelInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldRepelUsed(repel) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-repel result");
        };
        Ok(RuntimeRepelItemUse {
            item_use: repel.item_use,
            repel_steps_before: repel.repel_steps_before,
            repel_steps_after: repel.repel_steps_after,
            active_repel_item_before: repel.active_repel_item_before,
            active_repel_item_after: repel.active_repel_item_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_bicycle_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBicycleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagBicycleInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldBicycleUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-bicycle result");
        };
        Ok(RuntimeBicycleItemUse {
            item_use: outcome.item_use,
            map_name: outcome.map_name,
            permission: outcome.permission,
            mode_before: outcome.mode_before,
            mode_after: outcome.mode_after,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_itemfinder_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeItemfinderUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemfinderInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldItemfinderUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-itemfinder result");
        };
        Ok(RuntimeItemfinderUse {
            item_use: outcome.item_use,
            player_tile: outcome.player_tile,
            itemfinder_sound_cues: outcome.itemfinder_sound_cues,
            found: outcome.found,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_squirtbottle_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeSquirtBottleUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagSquirtbottleInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldSquirtbottleUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-squirtbottle result");
        };
        Ok(RuntimeSquirtBottleUse {
            item_use: outcome.item_use,
            player_tile: outcome.player_tile,
            target_tile: outcome.target_tile,
            target_object_identifier: outcome.target_object_identifier,
            target_movement: outcome.target_movement,
            target_script: outcome.target_script,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_coin_case_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeKeyItemBalanceUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagCoinCaseInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldCoinCaseUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-coin-case result");
        };
        Ok(RuntimeKeyItemBalanceUse {
            item_use: outcome.item_use,
            balance_label: outcome.balance_label,
            balance: outcome.balance,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_blue_card_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeKeyItemBalanceUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagBlueCardInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldBlueCardUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-blue-card result");
        };
        Ok(RuntimeKeyItemBalanceUse {
            item_use: outcome.item_use,
            balance_label: outcome.balance_label,
            balance: outcome.balance,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_town_map_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeTownMapUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagTownMapInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldTownMapUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-town-map result");
        };
        Ok(RuntimeTownMapUse {
            item_use: outcome.item_use,
            map_name: outcome.map_name,
            map_constant: outcome.map_constant,
            environment: outcome.environment,
            landmark: outcome.landmark,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_pokegear_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimePokegearUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagPokegearInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldPokegearUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-pokegear result");
        };
        Ok(RuntimePokegearUse {
            item_use: outcome.item_use,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_box_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBoxItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagBoxInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldBoxUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-box result");
        };
        Ok(RuntimeBoxItemUse {
            item_use: outcome.item_use,
            decoration_flag: outcome.decoration_flag,
            already_owned: outcome.already_owned,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_escape_rope_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        item_id: &str,
    ) -> Result<RuntimeEscapeRopeUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagEscapeRopeInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FieldEscapeRopeUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-escape-rope result");
        };
        Ok(RuntimeEscapeRopeUse {
            item_use: outcome.item_use,
            source_map: outcome.source_map,
            destination_map: outcome.destination_map,
            destination_warp_index: outcome.destination_warp_index,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_cut_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseCutFieldMove(RuntimeFieldBlockMoveCommand {
                party_index,
                metatile_x,
                metatile_y,
            }),
        )?;
        let RuntimeMutationResult::CutFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-CUT result");
        };
        Ok(RuntimeFieldMoveBlockUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_cut_field_move_in_front(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let (metatile_x, metatile_y) = runtime
            .data()
            .field_block_target_metatile_in_front(&self.overworld)?;
        self.use_cut_field_move(runtime, party_index, metatile_x, metatile_y)
    }

    pub fn use_whirlpool_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseWhirlpoolFieldMove(RuntimeFieldBlockMoveCommand {
                party_index,
                metatile_x,
                metatile_y,
            }),
        )?;
        let RuntimeMutationResult::WhirlpoolFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-WHIRLPOOL result");
        };
        Ok(RuntimeFieldMoveBlockUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_whirlpool_field_move_in_front(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        let (metatile_x, metatile_y) = runtime
            .data()
            .field_block_target_metatile_in_front(&self.overworld)?;
        self.use_whirlpool_field_move(runtime, party_index, metatile_x, metatile_y)
    }

    pub fn use_strength_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveFlagUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseStrengthFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::StrengthFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-STRENGTH result");
        };
        Ok(RuntimeFieldMoveFlagUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_flash_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveFlagUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseFlashFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::FlashFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-FLASH result");
        };
        Ok(RuntimeFieldMoveFlagUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_surf_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveTravelUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseSurfFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::SurfFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-SURF result");
        };
        Ok(RuntimeFieldMoveTravelUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_waterfall_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveTravelUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseWaterfallFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::WaterfallFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-WATERFALL result");
        };
        Ok(RuntimeFieldMoveTravelUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_fly_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        party_index: usize,
        destination_spawn_identifier: u16,
        flypoint_flag: &str,
    ) -> Result<RuntimeFlyFieldMoveUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseFlyFieldMove(RuntimeFlyCommand {
                party_index,
                destination_spawn_identifier,
                flypoint_flag: flypoint_flag.to_string(),
            }),
        )?;
        let RuntimeMutationResult::FlyFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-FLY result");
        };
        Ok(RuntimeFlyFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            flypoint_flag: outcome.flypoint_flag,
            source_map: outcome.source_map,
            destination_spawn_identifier: outcome.destination_spawn_identifier,
            destination_map: outcome.destination_map,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_dig_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        party_index: usize,
    ) -> Result<RuntimeDigFieldMoveUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseDigFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::DigFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-DIG result");
        };
        Ok(RuntimeDigFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            source_map: outcome.source_map,
            destination_map: outcome.destination_map,
            destination_warp_index: outcome.destination_warp_index,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_teleport_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
        party_index: usize,
    ) -> Result<RuntimeTeleportFieldMoveUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseTeleportFieldMove(RuntimeFieldPartyCommand { party_index }),
        )?;
        let RuntimeMutationResult::TeleportFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-TELEPORT result");
        };
        Ok(RuntimeTeleportFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            source_map: outcome.source_map,
            destination_spawn_identifier: outcome.destination_spawn_identifier,
            destination_map: outcome.destination_map,
            destination_tile: outcome.destination_tile,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_headbutt_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        player_id: u16,
    ) -> Result<RuntimeFieldEncounterMoveUse> {
        let rng_seed_after = preview_headbutt_field_move_rng_seed_after(
            runtime.data(),
            &self.state,
            &self.overworld,
            party_index,
            player_id,
        )?;
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseHeadbuttFieldMove(RuntimeHeadbuttFieldEncounterCommand {
                party_index,
                player_id,
                rng_seed_after,
            }),
        )?;
        let RuntimeMutationResult::HeadbuttFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-HEADBUTT result");
        };
        Ok(RuntimeFieldEncounterMoveUse {
            field_encounter: outcome.field_encounter,
            wild_battle: outcome.wild_battle,
            removed_object_identifier: outcome.removed_object_identifier,
            removed_event_flag: outcome.removed_event_flag,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn queue_rock_smash_from_menu(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeInteractionScriptDispatch> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::QueueRockSmashFromMenu(RuntimeFieldPartyCommand {
                party_index,
            }),
        )?;
        let RuntimeMutationResult::RockSmashFromMenuQueued(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-RockSmashFromMenu result");
        };
        Ok(RuntimeInteractionScriptDispatch {
            next_script: outcome.next_script,
            last_talked_object: Some(outcome.object_identifier),
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_sweet_scent_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeSweetScentFieldMoveUse> {
        let recorded = self.stage_sweet_scent_field_move(runtime, party_index)?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::SweetScentFieldMoveUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-SWEET_SCENT result");
        };
        Ok(RuntimeSweetScentFieldMoveUse {
            actor_party_index: outcome.actor_party_index,
            actor_species: outcome.actor_species,
            wild_encounter: outcome.wild_encounter,
            wild_battle: outcome.wild_battle,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_party_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimePartyItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemOnPartyPokemon(RuntimePartyItemCommand {
                item_id: item_id.to_string(),
                party_index,
            }),
        )?;
        let RuntimeMutationResult::PartyPokemonItemUsed(item_use, item_effect) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-party-item result");
        };
        Ok(RuntimePartyItemUse {
            item_use,
            item_effect,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_whole_party(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeWholePartyItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemOnWholeParty(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::WholePartyItemUsed(item_use, item_effect) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-whole-party-item result");
        };
        Ok(RuntimeWholePartyItemUse {
            item_use,
            item_effect,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_party_move(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<RuntimePartyItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemOnPartyMove(RuntimePartyMoveItemCommand {
                item_id: item_id.to_string(),
                party_index,
                move_slot,
            }),
        )?;
        let RuntimeMutationResult::PartyMoveItemUsed(item_use, item_effect) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-party-move-item result");
        };
        Ok(RuntimePartyItemUse {
            item_use,
            item_effect,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_tmhm_on_party_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
        replace_slot: Option<usize>,
    ) -> Result<RuntimeTmHmItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagTmHmOnPartyPokemon(RuntimeTmHmCommand {
                item_id: item_id.to_string(),
                party_index,
                replace_slot,
            }),
        )?;
        let RuntimeMutationResult::TmHmItemUsed(item_use, learned_move) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-TM/HM result");
        };
        Ok(RuntimeTmHmItemUse {
            item_use,
            learned_move,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_active_battle_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBattleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemOnActiveBattlePokemon(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::ActiveBattlePokemonItemUsed(item_use, battle_item) =
            mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-active-battle-item result");
        };
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_battle_party_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimeBattleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemOnBattlePartyPokemon(RuntimePartyItemCommand {
                item_id: item_id.to_string(),
                party_index,
            }),
        )?;
        let RuntimeMutationResult::BattlePartyPokemonItemUsed(item_use, battle_item) =
            mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-battle-party-item result");
        };
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_item_on_battle_party_move(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<RuntimeBattleItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagItemOnBattlePartyMove(RuntimePartyMoveItemCommand {
                item_id: item_id.to_string(),
                party_index,
                move_slot,
            }),
        )?;
        let RuntimeMutationResult::BattlePartyMoveItemUsed(item_use, battle_item) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-battle-party-move-item result");
        };
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn update_clock_from_datetime(
        &mut self,
        runtime: &CrystalRuntime,
        date: GameDate,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<RuntimeTimeUpdate> {
        let recorded = self.stage_clock_update(runtime, date, hour, minute, second)?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ClockUpdated = mutation.result else {
            anyhow::bail!("runtime mutation returned non-clock-update result");
        };
        Ok(self.runtime_time_update_with_checksum(mutation.state_checksum))
    }

    pub fn set_manual_clock_time(
        &mut self,
        runtime: &CrystalRuntime,
        now_date: GameDate,
        now_hour: u8,
        now_minute: u8,
        now_second: u8,
        target: ClockTime,
    ) -> Result<RuntimeTimeUpdate> {
        let recorded = self.stage_manual_clock_update(
            runtime, now_date, now_hour, now_minute, now_second, target,
        )?;
        let mutation = self.commit_recorded_mutation(recorded);
        let RuntimeMutationResult::ManualClockSet = mutation.result else {
            anyhow::bail!("runtime mutation returned non-manual-clock result");
        };
        Ok(self.runtime_time_update_with_checksum(mutation.state_checksum))
    }

    pub fn cast_fishing_rod(
        &mut self,
        runtime: &CrystalRuntime,
        rod: &str,
    ) -> Result<RuntimeFishingCast> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::CastFishingRod {
                rod: rod.to_string(),
            },
        )?;
        let RuntimeMutationResult::FishingRodCast(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-fishing-cast result");
        };
        Ok(RuntimeFishingCast {
            session: outcome.session,
            bite: outcome.bite,
            wild_battle: outcome.wild_battle,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn use_bag_fishing_rod_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeFishingRodItemUse> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::UseBagFishingRodInField(RuntimeItemCommand {
                item_id: item_id.to_string(),
            }),
        )?;
        let RuntimeMutationResult::BagFishingRodUsed(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-fishing-rod-item result");
        };
        Ok(RuntimeFishingRodItemUse {
            item_use: outcome.item_use,
            rod: outcome.rod,
            cast: RuntimeFishingCast {
                session: outcome.cast.session,
                bite: outcome.cast.bite,
                wild_battle: outcome.cast.wild_battle,
                state_checksum: outcome.cast_state_checksum,
            },
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn grant_script_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemGrant> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::GrantScriptItem(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptItemGranted(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-item-grant result");
        };
        Ok(RuntimeScriptItemGrant {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_script_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemCheck> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::CheckScriptItem(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptItemChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-item-check result");
        };
        Ok(RuntimeScriptItemCheck {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_script_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemTake> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::TakeScriptItem(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptItemTaken(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-item-take result");
        };
        Ok(RuntimeScriptItemTake {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    #[cfg(test)]
    pub fn apply_special_routine(
        &mut self,
        runtime: &CrystalRuntime,
        routine: &str,
    ) -> Result<RuntimeSpecialRoutineUse> {
        if runtime_special_routine_requires_divider_trace(routine) {
            let recorded = self.stage_random_special_routine(runtime, routine)?;
            let mutation = self.commit_recorded_mutation(recorded);
            let RuntimeMutationResult::SpecialRoutineApplied(outcome) = mutation.result else {
                anyhow::bail!("runtime mutation returned non-special-routine result");
            };
            return Ok(RuntimeSpecialRoutineUse {
                outcome,
                state_checksum: mutation.state_checksum,
            });
        }
        let rng_seed_after = preview_generic_special_routine_rng_seed_after(
            runtime.data(),
            &self.state,
            &runtime.music_ids(),
            routine,
        )?;
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplySpecialRoutine {
                routine: routine.to_string(),
                rng_seed_after,
            },
        )?;
        let RuntimeMutationResult::SpecialRoutineApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-special-routine result");
        };
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn pickup_script_field_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFieldPickup> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::PickupScriptFieldItem(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptFieldItemPickedUp(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-field-pickup result");
        };
        Ok(RuntimeFieldPickup {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_economy_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptEconomy> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptEconomy(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptEconomyApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-economy result");
        };
        Ok(RuntimeScriptEconomy {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn initialize_permanent_phone_numbers(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimePermanentPhoneNumbers> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::InitializePermanentPhoneNumbers,
        )?;
        let RuntimeMutationResult::PermanentPhoneNumbersInitialized(inserted) = mutation.result
        else {
            anyhow::bail!("runtime mutation returned non-permanent-phone-number result");
        };
        Ok(RuntimePermanentPhoneNumbers {
            inserted,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_phone_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptPhoneInputs,
    ) -> Result<RuntimePhoneCommand> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptPhone {
                command: Self::script_command_ref(map_name, source_script, command_index),
                inputs,
            },
        )?;
        let RuntimeMutationResult::ScriptPhoneApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-phone result");
        };
        Ok(RuntimePhoneCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_flag_mutation(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFlagMutation> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptFlagMutation(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptFlagMutated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-flag-mutation result");
        };
        Ok(RuntimeFlagMutation {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn check_script_flag(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFlagCheck> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::CheckScriptFlag(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptFlagChecked(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-flag-check result");
        };
        Ok(RuntimeFlagCheck {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_scene_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeSceneCommand> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptScene(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptSceneApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-scene result");
        };
        Ok(RuntimeSceneCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_block_change(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeBlockChange> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptBlockChange(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptBlockChanged(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-block-change result");
        };
        Ok(RuntimeBlockChange {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_audio_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptAudio> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptAudio(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptAudioApplied(cue) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-audio result");
        };
        Ok(RuntimeScriptAudio {
            cue,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_map_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptMapCommand> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptMap(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptMapApplied(action) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-map result");
        };
        Ok(RuntimeScriptMapCommand {
            action,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn execute_pending_script_warp(
        &mut self,
        runtime: &CrystalRuntime,
        _asset_root: &AssetRoot,
    ) -> Result<RuntimeScriptWarp> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::TransitionPendingScriptWarp,
        )?;
        let RuntimeMutationResult::PendingScriptWarpTransitioned(request) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-warp result");
        };
        Ok(RuntimeScriptWarp {
            target_map: request.target_map,
            tile: request.tile,
            facing: request.facing,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_text_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptText> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptText(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptTextApplied(action) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-text result");
        };
        Ok(RuntimeScriptText {
            action,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_variable_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptVariable> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptVariableNow(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptVariableApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-variable result");
        };
        Ok(RuntimeScriptVariable {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_swarm_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptSwarm> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptSwarm(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptSwarmApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-swarm result");
        };
        Ok(RuntimeScriptSwarm {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_control_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptControl> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptControl(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptControlApplied(action) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-control result");
        };
        Ok(RuntimeScriptControl {
            action,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_object_mutation(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptObjectMutation> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptObjectMutation(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptObjectMutated(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-object result");
        };
        Ok(RuntimeScriptObjectMutation {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_movement(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptMovement> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptMovement(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptMovementApplied(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-movement result");
        };
        Ok(RuntimeScriptMovement {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn apply_script_runtime_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
        inputs: ScriptRuntimeInputs,
    ) -> Result<RuntimeScriptRuntimeCommand> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ApplyScriptRuntime {
                command: Self::script_command_ref(map_name, source_script, command_index),
                inputs,
            },
        )?;
        let RuntimeMutationResult::ScriptRuntimeApplied(_, outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-runtime result");
        };
        Ok(RuntimeScriptRuntimeCommand {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn execute_next_queued_script_command(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeQueuedScriptCommand> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::ExecuteNextQueuedScriptCommand,
        )?;
        let RuntimeMutationResult::QueuedScriptCommandExecuted(queued) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-queued-script-command result");
        };
        Ok(RuntimeQueuedScriptCommand {
            queued,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_next_script(&mut self, runtime: &CrystalRuntime) -> Result<RuntimeNextScript> {
        let mutation =
            self.apply_runtime_mutation_command(runtime, RuntimeMutationCommand::TakeNextScript)?;
        let RuntimeMutationResult::NextScriptTaken(location) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-next-script result");
        };
        Ok(RuntimeNextScript {
            origin_map_name: location.origin_map_name,
            script: location.script,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn pop_script_call_stack(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeScriptReturnResume> {
        let mutation = self
            .apply_runtime_mutation_command(runtime, RuntimeMutationCommand::PopScriptCallStack)?;
        let RuntimeMutationResult::ScriptCallStackPopped(frame) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-call-stack-pop result");
        };
        Ok(RuntimeScriptReturnResume {
            frame,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn pop_deferred_script(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeDeferredScript> {
        let mutation = self
            .apply_runtime_mutation_command(runtime, RuntimeMutationCommand::PopDeferredScript)?;
        let RuntimeMutationResult::DeferredScriptPopped(location) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-deferred-script-pop result");
        };
        Ok(RuntimeDeferredScript {
            origin_map_name: location.origin_map_name,
            script: location.script,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn take_script_end_state(&mut self, runtime: &CrystalRuntime) -> Result<RuntimeScriptEnd> {
        let mutation = self
            .apply_runtime_mutation_command(runtime, RuntimeMutationCommand::TakeScriptEndState)?;
        let RuntimeMutationResult::ScriptEndStateTaken(end) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-end-state-take result");
        };
        Ok(RuntimeScriptEnd {
            end,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn open_script_shop(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptShop> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::OpenScriptShop(Self::script_command_ref(
                map_name,
                source_script,
                command_index,
            )),
        )?;
        let RuntimeMutationResult::ScriptShopOpened(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-script-shop result");
        };
        Ok(RuntimeScriptShop {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn buy_shop_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeShopTransaction> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::BuyShopItem(RuntimeShopTransactionCommand {
                item_id: item_id.to_string(),
                quantity,
            }),
        )?;
        let RuntimeMutationResult::ShopItemBought(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-shop-purchase result");
        };
        Ok(RuntimeShopTransaction {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }

    pub fn sell_shop_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeShopTransaction> {
        let mutation = self.apply_runtime_mutation_command(
            runtime,
            RuntimeMutationCommand::SellShopItem(RuntimeShopTransactionCommand {
                item_id: item_id.to_string(),
                quantity,
            }),
        )?;
        let RuntimeMutationResult::ShopItemSold(outcome) = mutation.result else {
            anyhow::bail!("runtime mutation returned non-shop-sale result");
        };
        Ok(RuntimeShopTransaction {
            outcome,
            state_checksum: mutation.state_checksum,
        })
    }
}

impl RuntimeAudioCatalog {
    fn from_game_data_owned(
        data: &GameDataSet,
        compiled_audio: BTreeMap<String, Vec<u8>>,
        manifest: ModpackAudioManifest,
        playback: ModpackAudioPlaybackPlan,
        audio_compression: Option<&str>,
    ) -> Result<Self> {
        Self::from_game_data_inner(data, compiled_audio, manifest, playback, audio_compression)
    }

    fn from_game_data_inner(
        data: &GameDataSet,
        mut compiled_audio: BTreeMap<String, Vec<u8>>,
        manifest: ModpackAudioManifest,
        playback: ModpackAudioPlaybackPlan,
        audio_compression: Option<&str>,
    ) -> Result<Self> {
        let declared_audio = data.audio_ids();
        for audio_id in compiled_audio.keys() {
            if !declared_audio.contains(audio_id.as_str()) {
                anyhow::bail!(
                    "runtime embedded audio payload {} is not declared by compiled pack data",
                    audio_id
                );
            }
        }

        if audio_compression.is_none() && manifest != data.audio_manifest(&compiled_audio)? {
            anyhow::bail!(
                "runtime audio manifest does not match embedded definitive audio payloads"
            );
        }
        playback
            .validate_for_manifest(&manifest)
            .context("validate runtime audio playback plan")?;
        let mut catalog = Self {
            manifest: manifest.clone(),
            playback,
            music: BTreeMap::new(),
            sound_effects: BTreeMap::new(),
            cries: BTreeMap::new(),
        };

        for asset in &data.audio {
            asset.validate()?;
            let source = match (asset.source, compiled_audio.remove(&asset.id)) {
                (ModpackAudioSource::Midi, Some(bytes)) => {
                    validate_compiled_audio_payload(asset, &bytes)?;
                    AudioProgramSource::Midi(bytes)
                }
                (ModpackAudioSource::Midi, None) => anyhow::bail!(
                    "compiled game pack missing embedded MIDI audio payload {}",
                    asset.id
                ),
                (ModpackAudioSource::Pcm, Some(bytes)) if audio_compression == Some("gzip") => {
                    let format = asset.pcm_format.as_ref().with_context(|| {
                        format!(
                            "compiled PCM audio asset '{}' missing validated pcm_format",
                            asset.id
                        )
                    })?;
                    let entry = manifest
                        .music
                        .get(&asset.id)
                        .or_else(|| manifest.sound_effects.get(&asset.id))
                        .or_else(|| manifest.cries.get(&asset.id))
                        .with_context(|| format!("audio manifest missing {}", asset.id))?;
                    AudioProgramSource::PcmGzip {
                        bytes,
                        format: AudioPcmFormat {
                            sample_rate_hz: format.sample_rate_hz,
                            channels: format.channels,
                            bits_per_sample: format.bits_per_sample,
                        },
                        byte_len: entry.byte_len,
                        payload_hash: entry.payload_hash.clone(),
                        loop_start_sample: asset.loop_start_sample,
                        loop_end_sample: asset.loop_end_sample,
                    }
                }
                (ModpackAudioSource::Pcm, Some(bytes)) => {
                    validate_compiled_audio_payload(asset, &bytes)?;
                    let format = asset.pcm_format.as_ref().with_context(|| {
                        format!(
                            "compiled PCM audio asset '{}' missing validated pcm_format",
                            asset.id
                        )
                    })?;
                    AudioProgramSource::Pcm {
                        bytes,
                        format: AudioPcmFormat {
                            sample_rate_hz: format.sample_rate_hz,
                            channels: format.channels,
                            bits_per_sample: format.bits_per_sample,
                        },
                        loop_start_sample: asset.loop_start_sample,
                        loop_end_sample: asset.loop_end_sample,
                    }
                }
                (ModpackAudioSource::Pcm, None) => anyhow::bail!(
                    "compiled game pack missing embedded PCM audio payload {}",
                    asset.id
                ),
            };
            let program = AudioProgram {
                cache_key: format!("{}:{}:{}", asset.kind.runtime_name(), asset.id, asset.path),
                source,
            };
            let previous = match asset.kind {
                ModpackAudioKind::Music => catalog.music.insert(asset.id.clone(), program),
                ModpackAudioKind::SoundEffect => {
                    catalog.sound_effects.insert(asset.id.clone(), program)
                }
                ModpackAudioKind::Cry => catalog.cries.insert(asset.id.clone(), program),
            };
            if previous.is_some() {
                anyhow::bail!(
                    "duplicate runtime {} audio id '{}'",
                    asset.kind.runtime_name(),
                    asset.id
                );
            }
        }
        if !compiled_audio.is_empty() {
            anyhow::bail!("runtime compiled audio contains undeclared payloads");
        }
        Ok(catalog)
    }

    pub fn from_game_data(
        data: &GameDataSet,
        compiled_audio: &BTreeMap<String, Vec<u8>>,
        manifest: ModpackAudioManifest,
        playback: ModpackAudioPlaybackPlan,
    ) -> Result<Self> {
        let declared_audio = data.audio_ids();
        for audio_id in compiled_audio.keys() {
            if !declared_audio.contains(audio_id.as_str()) {
                anyhow::bail!(
                    "runtime embedded audio payload {} is not declared by compiled pack data",
                    audio_id
                );
            }
        }

        if manifest != data.audio_manifest(compiled_audio)? {
            anyhow::bail!(
                "runtime audio manifest does not match embedded definitive audio payloads"
            );
        }
        playback
            .validate_for_manifest(&manifest)
            .context("validate runtime audio playback plan")?;
        let mut catalog = Self {
            manifest,
            playback,
            music: BTreeMap::new(),
            sound_effects: BTreeMap::new(),
            cries: BTreeMap::new(),
        };

        for asset in &data.audio {
            asset.validate()?;
            let bytes = compiled_audio
                .get(&asset.id)
                .with_context(|| {
                    format!(
                        "compiled game pack missing embedded audio payload {}",
                        asset.id
                    )
                })?
                .clone();
            validate_compiled_audio_payload(asset, &bytes)?;
            let source = match asset.source {
                ModpackAudioSource::Midi => AudioProgramSource::Midi(bytes),
                ModpackAudioSource::Pcm => {
                    let format = asset.pcm_format.as_ref().with_context(|| {
                        format!(
                            "compiled PCM audio asset '{}' missing validated pcm_format",
                            asset.id
                        )
                    })?;
                    AudioProgramSource::Pcm {
                        bytes,
                        format: AudioPcmFormat {
                            sample_rate_hz: format.sample_rate_hz,
                            channels: format.channels,
                            bits_per_sample: format.bits_per_sample,
                        },
                        loop_start_sample: asset.loop_start_sample,
                        loop_end_sample: asset.loop_end_sample,
                    }
                }
            };
            let program = AudioProgram {
                cache_key: format!("{}:{}:{}", asset.kind.runtime_name(), asset.id, asset.path),
                source,
            };
            let previous = match asset.kind {
                ModpackAudioKind::Music => catalog.music.insert(asset.id.clone(), program),
                ModpackAudioKind::SoundEffect => {
                    catalog.sound_effects.insert(asset.id.clone(), program)
                }
                ModpackAudioKind::Cry => catalog.cries.insert(asset.id.clone(), program),
            };
            if previous.is_some() {
                anyhow::bail!(
                    "duplicate runtime {} audio id '{}'",
                    asset.kind.runtime_name(),
                    asset.id
                );
            }
        }

        Ok(catalog)
    }

    pub fn program(&self, kind: AudioKind, id: &str) -> Option<&AudioProgram> {
        match kind {
            AudioKind::Music => self.music.get(id),
            AudioKind::SoundEffect => self.sound_effects.get(id),
            AudioKind::Cry => self.cries.get(id),
        }
    }

    pub fn playback_entry(&self, kind: AudioKind, id: &str) -> Option<&ModpackAudioPlaybackEntry> {
        match kind {
            AudioKind::Music => self.playback.music.get(id),
            AudioKind::SoundEffect => self.playback.sound_effects.get(id),
            AudioKind::Cry => self.playback.cries.get(id),
        }
    }

    pub fn require_program(&self, kind: AudioKind, id: &str) -> Result<&AudioProgram> {
        let kind_name = match kind {
            AudioKind::Music => "music",
            AudioKind::SoundEffect => "sound_effect",
            AudioKind::Cry => "cry",
        };
        self.program(kind, id)
            .with_context(|| format!("runtime audio catalog missing {kind_name} id {id}"))
    }

    pub fn require_playback_entry(
        &self,
        kind: AudioKind,
        id: &str,
    ) -> Result<&ModpackAudioPlaybackEntry> {
        let kind_name = match kind {
            AudioKind::Music => "music",
            AudioKind::SoundEffect => "sound_effect",
            AudioKind::Cry => "cry",
        };
        self.playback_entry(kind, id)
            .with_context(|| format!("runtime audio playback plan missing {kind_name} id {id}"))
    }

    pub fn resolve_audio_event(
        &self,
        event: crystal_core::state::ScriptAudioRuntimeEvent,
    ) -> Result<RuntimeResolvedAudioPlayback> {
        let kind = match event.kind {
            crystal_core::state::ScriptAudioRuntimeKind::Music => {
                let audio_id = required_audio_event_id(&event)?;
                self.require_music(&audio_id)?;
                RuntimeResolvedAudioPlaybackKind::Play {
                    playback: self
                        .require_playback_entry(AudioKind::Music, &audio_id)?
                        .clone(),
                    audio_id,
                }
            }
            crystal_core::state::ScriptAudioRuntimeKind::SoundEffect => {
                let audio_id = required_audio_event_id(&event)?;
                self.require_sound_effect(&audio_id)?;
                RuntimeResolvedAudioPlaybackKind::Play {
                    playback: self
                        .require_playback_entry(AudioKind::SoundEffect, &audio_id)?
                        .clone(),
                    audio_id,
                }
            }
            crystal_core::state::ScriptAudioRuntimeKind::Cry => {
                let audio_id = required_audio_event_id(&event)?;
                self.require_cry(&audio_id)?;
                RuntimeResolvedAudioPlaybackKind::Play {
                    playback: self
                        .require_playback_entry(AudioKind::Cry, &audio_id)?
                        .clone(),
                    audio_id,
                }
            }
            crystal_core::state::ScriptAudioRuntimeKind::FadeMusic => {
                let audio_id = required_audio_event_id(&event)?;
                let fade_frames = event.fade_frames.with_context(|| {
                    format!(
                        "runtime audio fade event {}:{} is missing fade_frames",
                        event.source_script, event.command_index
                    )
                })?;
                self.require_music(&audio_id)?;
                RuntimeResolvedAudioPlaybackKind::FadeMusic {
                    playback: self
                        .require_playback_entry(AudioKind::Music, &audio_id)?
                        .clone(),
                    audio_id,
                    fade_frames,
                }
            }
            crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect => {
                if event.audio_id.is_some() || event.fade_frames.is_some() {
                    anyhow::bail!(
                        "runtime audio wait event {}:{} must not carry audio_id or fade_frames",
                        event.source_script,
                        event.command_index
                    );
                }
                RuntimeResolvedAudioPlaybackKind::WaitForSoundEffect
            }
        };
        Ok(RuntimeResolvedAudioPlayback { event, kind })
    }

    pub fn resolve_audio_events(
        &self,
        events: impl IntoIterator<Item = crystal_core::state::ScriptAudioRuntimeEvent>,
    ) -> Result<Vec<RuntimeResolvedAudioPlayback>> {
        events
            .into_iter()
            .map(|event| self.resolve_audio_event(event))
            .collect()
    }

    pub fn require_music(&self, id: &str) -> Result<&AudioProgram> {
        self.require_program(AudioKind::Music, id)
    }

    pub fn require_sound_effect(&self, id: &str) -> Result<&AudioProgram> {
        self.require_program(AudioKind::SoundEffect, id)
    }

    pub fn require_cry(&self, id: &str) -> Result<&AudioProgram> {
        self.require_program(AudioKind::Cry, id)
    }

    pub fn audio_asset_keys(&self) -> BTreeSet<RuntimeAudioAssetKey> {
        let mut keys = BTreeSet::new();
        keys.extend(self.manifest.music.iter().filter_map(|(id, entry)| {
            self.music
                .get(id)
                .map(|program| runtime_audio_asset_key(entry, program))
        }));
        keys.extend(
            self.manifest
                .sound_effects
                .iter()
                .filter_map(|(id, entry)| {
                    self.sound_effects
                        .get(id)
                        .map(|program| runtime_audio_asset_key(entry, program))
                }),
        );
        keys.extend(self.manifest.cries.iter().filter_map(|(id, entry)| {
            self.cries
                .get(id)
                .map(|program| runtime_audio_asset_key(entry, program))
        }));
        keys
    }

    pub fn has_audio_asset(&self, key: &RuntimeAudioAssetKey) -> bool {
        self.audio_asset_keys().contains(key)
    }

    pub fn music_ids(&self) -> BTreeSet<String> {
        self.music.keys().cloned().collect()
    }

    pub fn sound_effect_ids(&self) -> BTreeSet<String> {
        self.sound_effects.keys().cloned().collect()
    }

    pub fn cry_ids(&self) -> BTreeSet<String> {
        self.cries.keys().cloned().collect()
    }
}

fn required_audio_event_id(event: &crystal_core::state::ScriptAudioRuntimeEvent) -> Result<String> {
    event.audio_id.clone().with_context(|| {
        format!(
            "runtime audio event {}:{} command {} is missing audio_id",
            event.source_script, event.command_index, event.command
        )
    })
}

fn runtime_audio_asset_key(
    entry: &ModpackAudioManifestEntry,
    program: &AudioProgram,
) -> RuntimeAudioAssetKey {
    let (pcm_sample_rate_hz, pcm_channels, pcm_bits_per_sample) = entry
        .pcm_format
        .as_ref()
        .map(|format| {
            (
                Some(format.sample_rate_hz),
                Some(format.channels),
                Some(format.bits_per_sample),
            )
        })
        .unwrap_or((None, None, None));
    let source = match entry.source {
        ModpackAudioSource::Midi => "midi",
        ModpackAudioSource::Pcm => "pcm",
    };
    RuntimeAudioAssetKey {
        audio_id: entry.id.clone(),
        kind: entry.kind.runtime_name().to_string(),
        source: source.to_string(),
        path: entry.path.clone(),
        byte_len: entry.byte_len,
        payload_hash: entry.payload_hash.clone(),
        pcm_sample_rate_hz,
        pcm_channels,
        pcm_bits_per_sample,
        pcm_frame_count: entry.pcm_frame_count,
        cache_key: program.cache_key.clone(),
    }
}

fn validate_save_references_for_runtime_pack(state: &GameState, data: &GameDataSet) -> Result<()> {
    data.validate_saved_bag_references(&state.bag)?;
    data.validate_saved_pokedex_references(&state.pokedex)?;
    data.validate_saved_storage_references(&state.storage)?;
    data.validate_saved_bug_contest_references(&state.bug_contest)?;
    data.validate_saved_day_care_references(&state.day_care)?;
    data.validate_saved_roaming_references(&state.roaming_pokemon, &state.roaming_map_history)?;
    data.validate_saved_mystery_gift_references(&state.mystery_gift)?;
    data.validate_saved_magikarp_record_references(&state.magikarp_record)?;
    data.validate_saved_blue_card_references(state)?;
    data.validate_saved_buena_password_references(&state.buenas_password)?;
    data.validate_saved_battle_tower_references(&state.battle_tower, &state.storage.party)?;
    data.validate_saved_link_session_references(&state.link_session)?;
    data.validate_saved_fishing_references(&state.fishing)?;
    data.validate_saved_swarm_references(&state.swarms)?;
    data.validate_saved_pending_special_battle_type(state.pending_special_battle_type.as_deref())?;
    validate_saved_script_runtime_references(data, state)?;
    data.validate_saved_overworld_references(&state.overworld)?;
    validate_saved_overworld_walkable_for_runtime_pack(&state.overworld, data)?;
    data.validate_saved_scene_references(&state.scenes)?;
    data.validate_saved_flag_references(&state.flags)?;
    if let Some(item_id) = &state.active_repel_item {
        data.validate_saved_active_repel_item(item_id, state.repel_steps_remaining)?;
    }
    if let Some(item_id) = &state.registered_key_item {
        data.validate_saved_item_reference("registered_key_item", item_id)?;
    }
    for pending in state
        .pending_move_learn
        .iter()
        .chain(state.pending_move_learn_queue.iter())
    {
        data.validate_saved_species_reference(
            "pending_move_learn.species_id",
            &pending.species_id,
        )?;
        data.validate_saved_move_reference(
            "pending_move_learn.learned_move.name",
            &pending.learned_move.name,
        )?;
        let pokemon = state
            .storage
            .party
            .pokemon
            .get(pending.party_index)
            .with_context(|| {
                format!(
                    "pending_move_learn.party_index {} is outside saved party range",
                    pending.party_index
                )
            })?
            .as_ref()
            .with_context(|| {
                format!(
                    "pending_move_learn.party_index {} references an empty saved party slot",
                    pending.party_index
                )
            })?;
        if pokemon.species.id != pending.species_id {
            anyhow::bail!(
                "pending_move_learn.species_id {} does not match saved storage.party[{}].species {}",
                pending.species_id,
                pending.party_index,
                pokemon.species.id
            );
        }
        if pokemon.level != pending.level {
            anyhow::bail!(
                "pending_move_learn.level {} does not match saved storage.party[{}].level {}",
                pending.level,
                pending.party_index,
                pokemon.level
            );
        }
        if pokemon
            .moves
            .iter()
            .any(|known| known.name == pending.learned_move.name)
        {
            anyhow::bail!(
                "pending_move_learn.learned_move.name {} is already known by saved storage.party[{}]",
                pending.learned_move.name,
                pending.party_index
            );
        }
    }
    if let Some(spawn_identifier) = state.last_spawn_identifier {
        data.validate_saved_spawn_reference("last_spawn_identifier", spawn_identifier)?;
    }
    if let Some(map_name) = &state.dig_warp_map_name {
        let _ = data.validate_saved_map_reference("dig_warp_map_name", map_name)?;
        if let Some(warp_index) = state.dig_warp_index {
            data.validate_saved_warp_reference("dig_warp_index", map_name, warp_index)?;
            validate_saved_dig_warp_destination(data, state)?;
        }
    }
    for (map_name, overrides) in &state.map_block_overrides {
        data.validate_saved_block_overrides(map_name, overrides)?;
    }
    for (map_name, memory) in &state.map_object_overrides {
        data.validate_saved_object_overrides(map_name, memory)?;
    }
    if let Some(terminal) = &state.pending_static_wild_terminal {
        data.validate_saved_static_wild_battle_origin_references(
            &terminal.battle_type,
            &terminal.species,
            terminal.level,
            &terminal.origin_map_name,
            &terminal.source_script,
            terminal.startbattle_command_index,
            terminal.resume_command_index,
        )?;
    }
    validate_saved_battle_references(data, state)
}

fn validate_saved_dig_warp_destination(data: &GameDataSet, state: &GameState) -> Result<()> {
    let destination = data
        .saved_dig_warp_destination(state, "saved dig_warp")
        .context("saved dig_warp destination is invalid")?;
    data.overworld_session(&destination.map_name, destination.tile, 0)
        .with_context(|| {
            format!(
                "saved dig_warp destination {} warp {} runtime tile ({}, {}) is invalid",
                destination.map_name,
                destination.warp_index,
                destination.tile.x,
                destination.tile.y
            )
        })?;
    Ok(())
}

fn validate_saved_overworld_walkable_for_runtime_pack(
    overworld: &OverworldMemory,
    data: &GameDataSet,
) -> Result<()> {
    let OverworldMemory::Active {
        map_name,
        tile,
        mode,
        ..
    } = overworld
    else {
        return Ok(());
    };
    data.overworld_session_for_traversal(map_name, *tile, 0, mode.traversal_state())
        .with_context(|| {
            format!(
                "saved overworld.active tile ({}, {}) is invalid on compiled map {map_name} for {:?}",
                tile.x, tile.y, mode
            )
        })?;
    Ok(())
}

fn validate_saved_script_runtime_references(data: &GameDataSet, state: &GameState) -> Result<()> {
    let runtime = &state.script_runtime;
    if let Some(script) = &runtime.next_script {
        if !data.maps.contains_key(&script.origin_map_name) {
            anyhow::bail!(
                "script_runtime.next_script has unknown origin map {}",
                script.origin_map_name
            );
        }
        data.validate_saved_script_label_reference(
            "script_runtime.next_script.script",
            &script.script,
        )?;
    }
    for (index, script) in runtime.deferred_scripts.iter().enumerate() {
        if !data.maps.contains_key(&script.origin_map_name) {
            anyhow::bail!(
                "script_runtime.deferred_scripts[{index}] has unknown origin map {}",
                script.origin_map_name
            );
        }
        data.validate_saved_script_label_reference(
            &format!("script_runtime.deferred_scripts[{index}].script"),
            &script.script,
        )?;
    }
    for (index, script) in runtime.stack.iter().enumerate() {
        data.validate_saved_script_label_reference(
            &format!("script_runtime.stack[{index}]"),
            script,
        )?;
    }
    for (index, frame) in runtime.call_stack.iter().enumerate() {
        if !data.maps.contains_key(&frame.origin_map_name) {
            anyhow::bail!(
                "script_runtime.call_stack[{index}] has unknown origin map {}",
                frame.origin_map_name
            );
        }
        data.validate_saved_script_return_reference(
            &format!("script_runtime.call_stack[{index}].source_script"),
            &frame.source_script,
            frame.next_command_index,
        )?;
    }
    if let Some(end) = &runtime.script_ended {
        data.validate_saved_script_end_command(end)?;
    }
    if let Some(routine) = &runtime.last_special_routine {
        data.validate_saved_special_routine_reference(
            "script_runtime.last_special_routine",
            routine,
        )?;
    }
    if let Some(menu) = &runtime.active_menu {
        data.validate_saved_menu_reference("script_runtime.active_menu", menu)?;
    }
    if let Some(species) = &runtime.active_pokemon_picture {
        data.validate_saved_species_reference("script_runtime.active_pokemon_picture", species)?;
    }
    if let Some(object_id) = &runtime.last_talked_object {
        data.validate_saved_last_talked_object_reference(state, object_id)?;
    }
    for (sprite, replacement) in &runtime.variable_sprites {
        data.validate_saved_variable_sprite_reference(
            "script_runtime.variable_sprites key",
            sprite,
        )?;
        data.validate_saved_sprite_reference(
            &format!("script_runtime.variable_sprites[{sprite}]"),
            replacement,
        )?;
    }
    if let Some(map_constant) = &runtime.blackout_mod {
        data.validate_saved_map_constant_reference("script_runtime.blackout_mod", map_constant)?;
    }
    if let Some(text_label) = &runtime.battle_tower_text {
        data.validate_saved_text_reference("script_runtime.battle_tower_text", text_label)?;
    }
    for contact_id in &runtime.phone_numbers {
        data.validate_saved_phone_contact_reference("script_runtime.phone_numbers", contact_id)?;
    }
    for (index, call_id) in runtime.special_phone_calls.iter().enumerate() {
        data.validate_saved_special_phone_call_reference(
            &format!("script_runtime.special_phone_calls[{index}]"),
            call_id,
        )?;
    }
    for (index, trade_id) in runtime.completed_trades.iter().enumerate() {
        data.validate_saved_npc_trade_reference(
            &format!("script_runtime.completed_trades[{index}]"),
            trade_id,
        )?;
    }
    for (index, battle_type) in runtime.catch_tutorials.iter().enumerate() {
        if !data.saved_catch_tutorial_battle_type_exists(battle_type) {
            anyhow::bail!(
                "saved script_runtime.catch_tutorials[{index}] {battle_type} is missing from compiled pack catchtutorial commands"
            );
        }
    }
    for (index, effect) in runtime.effects.iter().enumerate() {
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.effects[{index}].source_script"),
            &effect.source_script,
            effect.command_index,
            &effect.command,
            &effect.args,
        )?;
    }
    for (index, write) in runtime.variable_writes.iter().enumerate() {
        let (command, args) = saved_variable_write_command_payload(write);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.variable_writes[{index}].source_script"),
            &write.source_script,
            write.command_index,
            command,
            &args,
        )?;
    }
    for (index, directive) in runtime.asm_directives.iter().enumerate() {
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.asm_directives[{index}].source_script"),
            &directive.source_script,
            directive.command_index,
            &directive.command,
            &directive.args,
        )?;
    }
    for (index, write) in runtime.numeric_buffer_writes.iter().enumerate() {
        let (command, args) = saved_numeric_buffer_write_command_payload(write);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.numeric_buffer_writes[{index}].source_script"),
            &write.source_script,
            write.command_index,
            command,
            &args,
        )?;
    }
    for (index, floor) in runtime.elevator_floors.iter().enumerate() {
        data.validate_saved_map_reference(
            &format!("script_runtime.elevator_floors[{index}].target_map"),
            &floor.target_map,
        )?;
        data.validate_saved_elevator_floor_reference(
            &format!("script_runtime.elevator_floors[{index}].source_script"),
            floor,
        )?;
    }
    for (index, entry) in runtime.stone_table_entries.iter().enumerate() {
        data.validate_saved_script_label_reference(
            &format!("script_runtime.stone_table_entries[{index}].script"),
            &entry.script,
        )?;
        let (command, args) = saved_stone_table_entry_command_payload(entry);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.stone_table_entries[{index}].source_script"),
            &entry.source_script,
            entry.command_index,
            command,
            &args,
        )?;
    }
    for (index, description) in runtime.decoration_descriptions.iter().enumerate() {
        let (command, args) = saved_decoration_description_command_payload(description);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.decoration_descriptions[{index}].source_script"),
            &description.source_script,
            description.command_index,
            command,
            &args,
        )?;
    }
    for (index, delay) in runtime.pending_delays.iter().enumerate() {
        let (command, args) = saved_delay_command_payload(delay);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.pending_delays[{index}].source_script"),
            &delay.source_script,
            delay.command_index,
            command,
            &args,
        )?;
    }
    for (index, earthquake) in runtime.pending_earthquakes.iter().enumerate() {
        let (command, args) = saved_earthquake_command_payload(earthquake);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.pending_earthquakes[{index}].source_script"),
            &earthquake.source_script,
            earthquake.command_index,
            command,
            &args,
        )?;
    }
    for (index, emote) in runtime.pending_emotes.iter().enumerate() {
        let (command, args) = saved_emote_command_payload(emote);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.pending_emotes[{index}].source_script"),
            &emote.source_script,
            emote.command_index,
            command,
            &args,
        )?;
    }
    for (index, command) in runtime.command_queue.iter().enumerate() {
        if !data.maps.contains_key(&command.origin_map_name) {
            anyhow::bail!(
                "script_runtime.command_queue[{index}] has unknown origin map {}",
                command.origin_map_name
            );
        }
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.command_queue[{index}].source_script"),
            &command.source_script,
            command.command_index,
            &command.command,
            &saved_queued_command_args(command),
        )?;
    }
    for (index, target) in runtime.checked_mail_targets.iter().enumerate() {
        data.validate_saved_script_label_reference(
            &format!("script_runtime.checked_mail_targets[{index}]"),
            target,
        )?;
    }
    for (index, target) in runtime.given_mail_targets.iter().enumerate() {
        data.validate_saved_script_label_reference(
            &format!("script_runtime.given_mail_targets[{index}]"),
            target,
        )?;
    }
    if let Some(music) = &runtime.current_music {
        data.validate_saved_audio_reference(
            "script_runtime.current_music",
            music,
            ModpackAudioKind::Music,
        )?;
    }
    if let Some(fade) = &runtime.pending_music_fade {
        data.validate_saved_audio_reference(
            "script_runtime.pending_music_fade.audio_id",
            &fade.audio_id,
            ModpackAudioKind::Music,
        )?;
        let (command, args) = saved_music_fade_command_payload(fade);
        data.validate_saved_script_command_payload_reference(
            "script_runtime.pending_music_fade.source_script",
            &fade.source_script,
            fade.command_index,
            command,
            &args,
        )?;
    }
    for (index, event) in runtime.audio_events.iter().enumerate() {
        if let Some(audio_id) = &event.audio_id {
            let expected_kind = match event.kind {
                crystal_core::state::ScriptAudioRuntimeKind::Music
                | crystal_core::state::ScriptAudioRuntimeKind::FadeMusic => ModpackAudioKind::Music,
                crystal_core::state::ScriptAudioRuntimeKind::SoundEffect => {
                    ModpackAudioKind::SoundEffect
                }
                crystal_core::state::ScriptAudioRuntimeKind::Cry => ModpackAudioKind::Cry,
                crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect => {
                    ModpackAudioKind::SoundEffect
                }
            };
            if event.kind != crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect {
                data.validate_saved_audio_reference(
                    &format!("script_runtime.audio_events[{index}].audio_id"),
                    audio_id,
                    expected_kind,
                )?;
            }
        }
        if data
            .compiled_standard_script_body(&event.source_script)
            .is_err()
        {
            data.validate_saved_audio_runtime_event_command(
                &format!("script_runtime.audio_events[{index}].source_script"),
                event,
            )?;
        }
    }
    for (index, event) in runtime.graphics_events.iter().enumerate() {
        data.validate_saved_graphics_runtime_event(
            &format!("script_runtime.graphics_events[{index}].source_script"),
            event,
        )?;
    }
    if let Some(fade) = &runtime.pending_screen_fade {
        data.validate_saved_screen_fade("script_runtime.pending_screen_fade.source_script", fade)?;
    }
    for (index, event) in runtime.money_events.iter().enumerate() {
        data.validate_saved_money_runtime_event(
            &format!("script_runtime.money_events[{index}].source_script"),
            event,
        )?;
    }
    for (index, event) in runtime.map_events.iter().enumerate() {
        if let Some(map_name) = &event.target_map {
            let _ = data.validate_saved_map_reference(
                &format!("script_runtime.map_events[{index}].target_map"),
                map_name,
            )?;
        }
        validate_saved_map_runtime_event_destination(data, index, event)?;
        data.validate_saved_map_runtime_event_command(
            &format!("script_runtime.map_events[{index}].source_script"),
            event,
        )?;
    }
    if let Some(warp) = &runtime.pending_script_warp {
        let _ = data.validate_saved_map_reference(
            "script_runtime.pending_script_warp.target_map",
            &warp.target_map,
        )?;
        validate_saved_pending_script_warp_destination(data, warp)?;
        if data.saved_special_routine_exists(&warp.source_script) {
            data.validate_saved_special_routine_reference(
                "script_runtime.pending_script_warp.source_script",
                &warp.source_script,
            )?;
        } else if !data.saved_elevator_pending_warp_exists(warp) {
            data.validate_saved_script_warp_reference(
                "script_runtime.pending_script_warp.source_script",
                warp,
            )?;
        }
    }
    if let Some(load) = &runtime.pending_map_load {
        let (command, args) = saved_map_load_command_payload(load);
        data.validate_saved_script_command_payload_reference(
            "script_runtime.pending_map_load.source_script",
            &load.source_script,
            load.command_index,
            command,
            &args,
        )?;
    }
    if let Some(refresh) = &runtime.pending_map_refresh {
        let (command, args) = saved_map_refresh_command_payload(refresh);
        data.validate_saved_script_command_payload_reference(
            "script_runtime.pending_map_refresh.source_script",
            &refresh.source_script,
            refresh.command_index,
            command,
            &args,
        )?;
    }
    if let Some(text_label) = &runtime.pending_text_label {
        data.validate_saved_text_reference("script_runtime.pending_text_label", text_label)?;
    }
    for (index, event) in runtime.text_events.iter().enumerate() {
        if let Some(text_label) = &event.text_label {
            data.validate_saved_text_reference(
                &format!("script_runtime.text_events[{index}].text_label"),
                text_label,
            )?;
        }
        if data
            .compiled_standard_script_body(&event.source_script)
            .is_err()
        {
            data.validate_saved_text_runtime_event_command(
                &format!("script_runtime.text_events[{index}].source_script"),
                event,
            )?;
        }
    }
    if let Some(wait) = &runtime.pending_text_wait {
        if data
            .compiled_standard_script_body(&wait.source_script)
            .is_err()
        {
            data.validate_saved_pending_text_wait_command(runtime, wait)?;
        }
    }
    if let Some(prompt) = &runtime.pending_yes_no {
        if data
            .compiled_standard_script_body(&prompt.source_script)
            .is_err()
        {
            data.validate_saved_script_command_payload_reference(
                "script_runtime.pending_yes_no.source_script",
                &prompt.source_script,
                prompt.command_index,
                "yesorno",
                &[],
            )?;
        }
    }
    for (index, event) in runtime.control_events.iter().enumerate() {
        data.validate_saved_control_runtime_event_command(
            &format!("script_runtime.control_events[{index}].source_script"),
            event,
        )?;
        if let Some(target_script) = &event.target_script {
            data.validate_saved_script_label_reference(
                &format!("script_runtime.control_events[{index}].target_script"),
                target_script,
            )?;
        }
    }
    for (index, event) in runtime.shop_events.iter().enumerate() {
        let (command, args) = saved_shop_event_command_payload(event);
        data.validate_saved_script_command_payload_reference(
            &format!("script_runtime.shop_events[{index}].source_script"),
            &event.source_script,
            event.command_index,
            command,
            &args,
        )?;
        for (item_index, item_id) in event.inventory.iter().enumerate() {
            data.validate_saved_item_reference(
                &format!("script_runtime.shop_events[{index}].inventory[{item_index}]"),
                item_id,
            )?;
        }
    }
    if let Some(shop) = &runtime.pending_shop {
        let (command, args) = saved_shop_request_command_payload(shop);
        data.validate_saved_script_command_payload_reference(
            "script_runtime.pending_shop.source_script",
            &shop.source_script,
            shop.command_index,
            command,
            &args,
        )?;
        for (item_index, item_id) in shop.inventory.iter().enumerate() {
            data.validate_saved_item_reference(
                &format!("script_runtime.pending_shop.inventory[{item_index}]"),
                item_id,
            )?;
        }
    }
    for (index, event) in runtime.item_use_events.iter().enumerate() {
        data.validate_saved_item_reference(
            &format!("script_runtime.item_use_events[{index}].item_id"),
            &event.item_id,
        )?;
    }
    Ok(())
}

fn validate_saved_map_runtime_event_destination(
    data: &GameDataSet,
    index: usize,
    event: &ScriptMapRuntimeEvent,
) -> Result<()> {
    if event.kind != crystal_core::state::ScriptMapRuntimeKind::Warp {
        return Ok(());
    }
    let Some(target_map) = event.target_map.as_deref() else {
        return Ok(());
    };
    let Some(tile) = event.tile else {
        return Ok(());
    };
    data.overworld_session(target_map, tile, 0)
        .with_context(|| {
            format!(
                "saved script_runtime.map_events[{index}] destination {target_map} runtime tile ({}, {}) is invalid",
                tile.x, tile.y
            )
        })?;
    Ok(())
}

fn validate_saved_pending_script_warp_destination(
    data: &GameDataSet,
    warp: &ScriptWarpRequest,
) -> Result<()> {
    data.overworld_session(&warp.target_map, warp.tile, 0)
        .with_context(|| {
            format!(
                "saved script_runtime.pending_script_warp destination {} runtime tile ({}, {}) is invalid",
                warp.target_map, warp.tile.x, warp.tile.y
            )
        })?;
    Ok(())
}

fn validate_saved_battle_references(data: &GameDataSet, state: &GameState) -> Result<()> {
    match &state.battle {
        BattleMemory::Inactive => Ok(()),
        BattleMemory::Wild {
            battle_type,
            map_name,
            roaming_slot,
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            let _ = data.validate_saved_map_reference("battle.wild.map_name", map_name)?;
            data.validate_saved_pokemon_reference("battle.wild.enemy_pokemon", enemy_pokemon)?;
            data.validate_saved_pokemon_party_references("battle.wild.enemy_party", enemy_party)?;
            if battle_type == "BATTLETYPE_ROAMING" {
                let slot = roaming_slot.context("saved roaming battle is missing roaming_slot")?;
                let roaming = state
                    .roaming_pokemon
                    .get(usize::from(slot))
                    .with_context(|| format!("saved roaming battle slot {slot} is invalid"))?;
                data.validate_saved_roaming_battle_origin_references(
                    map_name,
                    slot,
                    roaming,
                    enemy_pokemon,
                )
            } else {
                data.validate_saved_wild_battle_origin_references(
                    battle_type,
                    map_name,
                    enemy_pokemon,
                )
            }
        }
        BattleMemory::StaticWild {
            battle_type,
            origin_map_name,
            species,
            level,
            source_script,
            startbattle_command_index,
            resume_command_index,
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            data.validate_saved_species_reference("battle.static_wild.species", species)?;
            data.validate_saved_script_label_reference(
                "battle.static_wild.source_script",
                source_script,
            )?;
            data.validate_saved_static_wild_battle_origin_references(
                battle_type,
                species,
                *level,
                origin_map_name,
                source_script,
                *startbattle_command_index,
                *resume_command_index,
            )?;
            data.validate_saved_pokemon_reference(
                "battle.static_wild.enemy_pokemon",
                enemy_pokemon,
            )?;
            data.validate_saved_pokemon_party_references(
                "battle.static_wild.enemy_party",
                enemy_party,
            )
        }
        BattleMemory::Trainer {
            battle_type,
            trainer_id,
            trainer_class,
            trainer_name,
            event_flag,
            seen_text,
            win_text,
            loss_text,
            callback,
            source_script,
            encounter_music,
            ai_move_flags,
            ai_item_switch_flags,
            ai_layers,
            reward,
            enemy_pokemon,
            enemy_party,
            ..
        } => {
            let canonical_battle_tower_trainer =
                data.battle_tower_rules.as_ref().is_some_and(|rules| {
                    trainer_id
                        .strip_prefix("BATTLE_TOWER_")
                        .and_then(|index| index.parse::<usize>().ok())
                        .is_some_and(|index| {
                            rules.trainers.iter().any(|trainer| trainer.index == index)
                        })
                });
            if canonical_battle_tower_trainer {
                data.validate_saved_audio_reference(
                    "battle.trainer.encounter_music",
                    encounter_music,
                    ModpackAudioKind::Music,
                )?;
                data.validate_saved_pokemon_reference(
                    "battle.trainer.enemy_pokemon",
                    enemy_pokemon,
                )?;
                data.validate_saved_pokemon_party_references(
                    "battle.trainer.enemy_party",
                    enemy_party,
                )?;
                if enemy_party.first() != Some(enemy_pokemon) {
                    anyhow::bail!(
                        "saved Battle Tower enemy party head does not match enemy_pokemon"
                    );
                }
                return Ok(());
            }
            let trainer =
                data.validate_saved_trainer_reference("battle.trainer.trainer_id", trainer_id)?;
            validate_saved_trainer_metadata(
                trainer,
                SavedTrainerMetadata {
                    trainer_class,
                    trainer_name,
                    ai_move_flags: *ai_move_flags,
                    ai_item_switch_flags: *ai_item_switch_flags,
                    ai_layers,
                    reward: *reward,
                    encounter_music,
                },
            )
            .map_err(|error| anyhow::anyhow!("{error}"))?;
            data.validate_saved_trainer_battle_origin_references(
                trainer,
                battle_type,
                trainer_class,
                event_flag,
                seen_text,
                win_text,
                loss_text,
                callback,
                source_script,
            )?;
            data.validate_saved_audio_reference(
                "battle.trainer.encounter_music",
                encounter_music,
                ModpackAudioKind::Music,
            )?;
            data.validate_saved_pokemon_reference("battle.trainer.enemy_pokemon", enemy_pokemon)?;
            data.validate_saved_pokemon_party_references(
                "battle.trainer.enemy_party",
                enemy_party,
            )?;
            data.validate_saved_trainer_enemy_party(trainer, enemy_party, enemy_pokemon)
        }
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
