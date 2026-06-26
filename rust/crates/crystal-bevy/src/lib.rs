use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use crystal_assets::modpack::{
    CompiledGamePack, GameDataSet, LoadedCompiledGamePack, ModpackAudioKind, ModpackAudioSource,
};
use crystal_assets::{AssetRoot, RuntimeMapMetadata, RuntimeSpawnPoint};
use crystal_audio::{AudioKind, AudioProgram, AudioProgramSource};
use crystal_core::battle::capture::{
    complete_captured_pokemon, throw_ball_from_bag, validate_capture_ball_item,
    CaptureAttemptContext, CaptureOutcome, StoredCapture,
};
use crystal_core::battle::start::{
    complete_trainer_battle, StaticWildBattleStart, TrainerBattleCompletion,
    TrainerBattleStartStatus, WildBattleStart,
};
use crystal_core::battle::turn::{
    resolve_battle_turn_with_items, BattleAction, BattleCombatState, BattleTurnInput,
    BattleTurnOutcome,
};
use crystal_core::input::{
    GameButton, JoypadState, B_PAD_A, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP,
};
use crystal_core::models::{Dv, Item, Move, Pokemon, PokemonSpecies};
use crystal_core::multiplayer::{game_state_checksum, PlayerId, StateChecksum, StateChecksumFrame};
use crystal_core::random::Random;
use crystal_core::save::{
    read_save_game_for_modpack, write_save_game_for_modpack, SaveModpackIdentity,
};
use crystal_core::state::{
    BattleMemory, GameState, OverworldFollowMemory, OverworldMemory, OverworldObjectMapMemory,
    OverworldObjectMemory,
};
use crystal_core::systems::battle_escape::{attempt_wild_battle_escape, BattleEscapeAttempt};
use crystal_core::systems::battle_items::{
    apply_active_battle_item_effect, apply_battle_pp_item_effect,
    apply_evolution_stone_item_effect, apply_party_wide_item_effect, apply_rare_candy_item_effect,
    validate_battle_escape_item, validate_battle_stat_drop_guard_item, BattleItemError,
    BattleItemOutcome, PartyItemOutcome,
};
use crystal_core::systems::battle_rewards::{
    apply_trainer_battle_rewards, apply_wild_battle_rewards, BattleRewardOutcome, BattleRewardRules,
};
use crystal_core::systems::economy::{apply_script_economy_command, ScriptEconomyOutcome};
use crystal_core::systems::evolution::EvolutionTable;
use crystal_core::systems::field_items::{pickup_script_field_item, FieldItemPickupOutcome};
use crystal_core::systems::field_moves::{
    apply_cut_field_move, apply_flash_field_move, apply_strength_field_move, apply_surf_field_move,
    apply_waterfall_field_move, apply_whirlpool_field_move, validate_bicycle_item,
    validate_blue_card_item, validate_coin_case_item, validate_dig_field_move,
    validate_field_escape_item, validate_fly_field_move, validate_itemfinder_item,
    validate_repel_item, validate_squirtbottle_item, validate_teleport_field_move,
    validate_town_map_item, FieldMoveBlockOutcome, FieldMoveFlagOutcome, FieldMoveTravelOutcome,
};
use crystal_core::systems::gift_pokemon::{
    give_gift_pokemon, GiftPokemonOutcome, GiftPokemonRequest,
};
use crystal_core::systems::item_use::{
    use_bag_item, ItemUseContext, ItemUseOutcome, ItemUseRequest,
};
use crystal_core::systems::learnsets::SpeciesLearnsets;
use crystal_core::systems::phone::{
    apply_script_phone_command, initialize_permanent_phone_numbers, ScriptPhoneInputs,
    ScriptPhoneOutcome,
};
use crystal_core::systems::script_audio::{apply_script_audio_command, ScriptAudioCue};
use crystal_core::systems::script_blocks::{apply_script_block_change, ScriptBlockChangeOutcome};
use crystal_core::systems::script_control::{apply_script_control_command, ScriptControlAction};
use crystal_core::systems::script_flags::{
    apply_script_flag_mutation, check_script_flag, ScriptFlagCheckOutcome,
    ScriptFlagMutationOutcome,
};
use crystal_core::systems::script_items::{
    check_script_item, grant_script_item, take_script_item, ScriptItemCheckOutcome,
    ScriptItemGrantOutcome, ScriptItemTakeOutcome,
};
use crystal_core::systems::script_objects::{
    apply_script_movement, apply_script_object_mutation, ScriptMovementOutcome,
    ScriptObjectMutationOutcome,
};
use crystal_core::systems::script_runtime::{
    apply_script_runtime_command, ScriptRuntimeInputs, ScriptRuntimeOutcome,
};
use crystal_core::systems::script_scenes::{apply_script_scene_command, ScriptSceneOutcome};
use crystal_core::systems::script_text::{apply_script_text_command, ScriptTextAction};
use crystal_core::systems::script_variables::{
    apply_script_variable_command, ScriptVariableOutcome,
};
use crystal_core::systems::script_warps::{apply_script_map_command, ScriptMapAction};
use crystal_core::systems::scripted_battles::{
    apply_scripted_battle_effects_to_session, ScriptedBattleEffects, ScriptedBattleEffectsOutcome,
};
use crystal_core::systems::shop::{
    apply_script_shop_command, buy_item, sell_item, ScriptShopOutcome, ShopResult,
};
use crystal_core::systems::special_routines::{
    apply_special_routine_with_context, RuntimeSpawnPointRef, SpecialRoutineContext,
    SpecialRoutineOutcome,
};
use crystal_core::systems::step_events::{process_step, StepEventResult};
use crystal_core::systems::time::{ClockTime, GameDate};
use crystal_core::systems::tmhm::{teach_tmhm_move, TmHmLearnOutcome};
use crystal_core::world::collision::{permissions, sample_collision};
use crystal_core::world::encounters::{
    require_encounter_table_for_surface, select_headbutt_encounter, select_rock_smash_encounter,
    select_sweet_scent_encounter, EncounterSurface, FieldEncounterKind, FieldEncounterRoll,
    ResolvedWildEncounter, TimeOfDay,
};
use crystal_core::world::fishing::{
    do_fishing, fishing_battle_trigger, fishing_bite, fishing_rod_for_item_id, FishingSession,
    ROD_GOOD, ROD_OLD, ROD_SUPER,
};
use crystal_core::world::map::{Direction, OverworldMapData, TilePosition};
use crystal_core::world::movement::{move_by_stride, MovementMode, StepOptions, StepOutcome};
use crystal_core::world::session::{
    ConnectionTransition, CoordEventTrigger, EncounterCheckOptions, OverworldFollowState,
    OverworldInteraction, OverworldSession, OverworldSnapshot, WarpTransition, WildEncounterRoll,
};

pub use crystal_assets as assets;
pub use crystal_audio as audio;
pub use crystal_core as core;
pub use crystal_net as net;

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
    music: BTreeMap<String, AudioProgram>,
    sound_effects: BTreeMap<String, AudioProgram>,
    cries: BTreeMap<String, AudioProgram>,
}

impl RuntimeAudioCatalog {
    pub fn is_empty(&self) -> bool {
        self.music.is_empty() && self.sound_effects.is_empty() && self.cries.is_empty()
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
    data: GameDataSet,
    audio: RuntimeAudioCatalog,
    viewport: GameViewport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBootSummary {
    pub modpack_id: String,
    pub modpack_hash: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOverworldFrame {
    pub snapshot: OverworldSnapshot,
    pub input_mask: u8,
    pub pressed_mask: u8,
    pub movement: Option<StepOutcome>,
    pub step_events: Option<StepEventResult>,
    pub coord_event: Option<CoordEventTrigger>,
    pub interaction: Option<OverworldInteraction>,
    pub warp: Option<WarpTransition>,
    pub connection: Option<ConnectionTransition>,
    pub wild_encounter: Option<WildEncounterRoll>,
    pub wild_battle: Option<WildBattleStart>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeScriptedBattleCompletion {
    pub continued_after_battle: bool,
    pub effects: Option<ScriptedBattleEffectsOutcome>,
    pub trainer_prize_money: Option<u32>,
    pub money_after: Option<u32>,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCaptureCompletion {
    pub stored: Option<StoredCapture>,
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
pub struct RuntimeBattleEscape {
    pub outcome: BattleEscapeAttempt,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeBattleCommand {
    Turn(RuntimeBattleTurn),
    Escape(RuntimeBattleEscape),
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
    pub game_time_hours: u8,
    pub game_time_minutes: u8,
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
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSweetScentFieldMoveUse {
    pub actor_party_index: usize,
    pub actor_species: String,
    pub wild_encounter: WildEncounterRoll,
    pub wild_battle: WildBattleStart,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemUse {
    pub outcome: ItemUseOutcome,
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
pub struct RuntimeItemfinderHiddenItem {
    pub map_name: String,
    pub tile: TilePosition,
    pub source_script: String,
    pub event_flag: String,
    pub item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemfinderUse {
    pub item_use: ItemUseOutcome,
    pub player_tile: TilePosition,
    pub found: Option<RuntimeItemfinderHiddenItem>,
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
pub struct RuntimeTownMapLandmark {
    pub id: u16,
    pub constant: String,
    pub label: String,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub region: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTownMapUse {
    pub item_use: ItemUseOutcome,
    pub map_name: String,
    pub map_constant: String,
    pub environment: String,
    pub landmark: RuntimeTownMapLandmark,
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

fn apply_party_special_item_effect(
    pokemon: &mut Pokemon,
    item: &Item,
    species: &BTreeMap<String, PokemonSpecies>,
    moves: &BTreeMap<String, Move>,
    learnsets: &SpeciesLearnsets,
    growth_rates: &crystal_core::systems::experience::GrowthRateCatalog,
    reward_rules: &BattleRewardRules,
    evolutions: &EvolutionTable,
    time_of_day: TimeOfDay,
    consumed: bool,
) -> std::result::Result<BattleItemOutcome, BattleItemError> {
    if item.rare_candy_level_gain.is_some() {
        apply_rare_candy_item_effect(
            pokemon,
            item,
            species,
            moves,
            learnsets,
            growth_rates,
            reward_rules,
            evolutions,
            time_of_day,
            consumed,
        )
    } else if evolutions.contains_item_evolution(&item.script_name) {
        apply_evolution_stone_item_effect(
            pokemon,
            item,
            species,
            moves,
            learnsets,
            evolutions,
            time_of_day,
            consumed,
        )
    } else {
        Err(BattleItemError::MissingBattleItemPayload {
            item_id: item.script_name.clone(),
        })
    }
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
pub struct RuntimeScriptShop {
    pub outcome: ScriptShopOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShopTransaction {
    pub outcome: ShopResult,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RuntimeInputError {
    #[error("input frame has conflicting direction buttons: mask {mask:#010b}")]
    ConflictingDirections { mask: u8 },
}

impl CrystalRuntime {
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
        let modpack_id = runtime_modpack_id(loaded.pack())?;
        let modpack = SaveModpackIdentity::from_compiled_pack_bytes(modpack_id, loaded.bytes())
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
        let expected_id = runtime_modpack_id(&pack)?;
        if modpack.id() != expected_id {
            anyhow::bail!(
                "compiled game pack identity '{}' does not match report manifest id '{}'",
                modpack.id(),
                expected_id
            );
        }
        reject_pack_with_error_diagnostics(&pack)?;
        reject_pack_without_runtime_game_data(&pack)?;
        let (_, data, _) = pack.into_parts();
        let audio = RuntimeAudioCatalog::from_game_data(asset_root, &data)?;
        Ok(Self {
            modpack,
            data,
            audio,
            viewport: GameViewport::default(),
        })
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

    pub fn viewport(&self) -> &GameViewport {
        &self.viewport
    }

    pub fn save_game(&self, path: impl AsRef<Path>, state: GameState) -> Result<()> {
        write_save_game_for_modpack(path, state, &self.modpack)
            .context("write Crystal runtime save")
    }

    pub fn load_save(&self, path: impl AsRef<Path>) -> Result<GameState> {
        let save = read_save_game_for_modpack(path, &self.modpack)
            .context("read Crystal runtime save for compiled modpack identity")?;
        Ok(save.into_state())
    }

    pub fn boot_summary(&self) -> RuntimeBootSummary {
        RuntimeBootSummary {
            modpack_id: self.modpack.id().to_string(),
            modpack_hash: self.modpack.hash().to_string(),
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

    pub fn start_overworld_session(
        &self,
        asset_root: &AssetRoot,
        spawn_identifier: u16,
    ) -> Result<RuntimeOverworldSession> {
        let spawn = self.spawn_point(spawn_identifier)?;
        RuntimeOverworldSession::new(self, asset_root, spawn)
    }

    pub fn resume_overworld_session(
        &self,
        asset_root: &AssetRoot,
        state: GameState,
    ) -> Result<RuntimeOverworldSession> {
        RuntimeOverworldSession::from_state(self, asset_root, state)
    }

    fn spawn_point(&self, spawn_identifier: u16) -> Result<&RuntimeSpawnPoint> {
        self.data
            .runtime_spawn_points
            .get(&spawn_identifier.to_string())
            .with_context(|| format!("compiled game pack missing spawn point {spawn_identifier}"))
    }

    fn map_module(&self, map_name: &str) -> Result<&crystal_assets::modpack::MapModule> {
        self.data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))
    }

    fn map_name_for_constant(&self, map_constant: &str) -> Result<String> {
        self.data
            .maps
            .iter()
            .find(|(_, module)| module.attributes.map_constant.as_deref() == Some(map_constant))
            .map(|(map_name, _)| map_name.clone())
            .with_context(|| format!("compiled game pack missing map constant {map_constant}"))
    }

    fn overworld_session_for(
        &self,
        asset_root: &AssetRoot,
        map_name: &str,
        player_tile: TilePosition,
        frame: u64,
    ) -> Result<OverworldSession> {
        let module = self.map_module(map_name)?;
        let tileset = asset_root
            .load_tileset_collision(&module.attributes.tileset_name)
            .with_context(|| {
                format!(
                    "load tileset collision {} for {map_name}",
                    module.attributes.tileset_name
                )
            })?;
        let map =
            OverworldMapData::from_attributes(map_name, &module.attributes, module.blocks.clone());
        let mut session = OverworldSession::with_events_and_objects(
            map,
            module.events.clone(),
            module.objects.clone(),
            tileset,
            player_tile,
        );
        session.frame = frame;
        Ok(session)
    }

    fn resolve_warp_transition(
        &self,
        trigger: &crystal_core::world::session::WarpTrigger,
    ) -> Result<WarpTransition> {
        self.data.resolve_warp_transition(trigger)
    }

    fn resolve_connection_transition(
        &self,
        trigger: &crystal_core::world::session::ConnectionTrigger,
    ) -> Result<ConnectionTransition> {
        self.data.resolve_connection_transition(trigger)
    }

    fn map_music(&self, map_name: &str) -> Result<Option<String>> {
        let module = self
            .data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?;
        let Some(music) = &module.attributes.music else {
            return Ok(None);
        };
        if !self.audio.contains_music(music) {
            anyhow::bail!("map {map_name} references missing runtime music asset {music}");
        }
        Ok(Some(music.clone()))
    }

    fn map_ids(&self) -> BTreeSet<String> {
        self.data.maps.keys().cloned().collect()
    }

    fn cry_by_species(&self) -> BTreeMap<String, String> {
        self.data
            .pokemon_cries
            .iter()
            .map(|(species_id, cry)| (species_id.clone(), cry.cry.clone()))
            .collect()
    }

    fn script_text_labels(&self, module: &crystal_assets::modpack::MapModule) -> BTreeSet<String> {
        module.script_text_bodies.keys().cloned().collect()
    }

    fn script_numeric_constants(&self) -> BTreeMap<String, i32> {
        let mut constants = BTreeMap::new();
        for (constant, value) in &self.data.currency_constants.0 {
            if let Ok(value) = i32::try_from(*value) {
                constants.insert(constant.clone(), value);
            }
        }
        for (constant, value) in &self.data.story_event_script_constants.global {
            if let Ok(value) = i32::try_from(*value) {
                constants.insert(constant.clone(), value);
            }
        }
        for constants_by_map in self.data.story_event_script_constants.maps.values() {
            for (constant, value) in constants_by_map {
                if let Ok(value) = i32::try_from(*value) {
                    constants.insert(constant.clone(), value);
                }
            }
        }
        constants
    }

    fn sync_current_map_music(&self, state: &mut GameState, map_name: &str) -> Result<()> {
        state.script_runtime.current_music = self.map_music(map_name)?;
        state.script_runtime.pending_music_fade = None;
        Ok(())
    }

    fn sync_current_map_scene(&self, state: &mut GameState, map_name: &str) -> Result<()> {
        let module = self.map_module(map_name)?;
        if module.scenes.scenes.is_empty() {
            return Ok(());
        }
        state
            .scenes
            .enter_map(map_name, &module.scenes)
            .map_err(|error| anyhow::anyhow!("enter scene context for {map_name}: {error:?}"))?;
        Ok(())
    }

    fn runtime_map_metadata_for_name(&self, map_name: &str) -> Result<&RuntimeMapMetadata> {
        self.data
            .runtime_map_metadata
            .values()
            .find(|metadata| metadata.name == map_name)
            .with_context(|| {
                format!("compiled game pack missing runtime metadata for map {map_name}")
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SavedDigWarpDestination {
    map_name: String,
    warp_index: u16,
    tile: TilePosition,
}

impl RuntimeOverworldSession {
    fn new(
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        spawn: &RuntimeSpawnPoint,
    ) -> Result<Self> {
        let overworld = runtime.overworld_session_for(
            asset_root,
            &spawn.map_name,
            TilePosition::new(spawn.tile_x, spawn.tile_y),
            0,
        )?;
        let mut state = GameState::default();
        state.overworld = OverworldMemory::from_snapshot(&overworld.snapshot());
        state.last_spawn_identifier = Some(spawn.identifier);
        runtime.sync_current_map_music(&mut state, &overworld.map.name)?;
        runtime.sync_current_map_scene(&mut state, &overworld.map.name)?;
        Ok(Self {
            state,
            overworld,
            joypad: JoypadState::new(),
        })
    }

    fn from_state(
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        mut state: GameState,
    ) -> Result<Self> {
        let (map_name, tile, facing, mode) = state
            .overworld
            .snapshot_identity()
            .with_context(|| "cannot resume overworld session from inactive GameState")?;
        let map_name = map_name.to_string();
        let mut overworld =
            runtime.overworld_session_for(asset_root, &map_name, tile, state.frame_counter)?;
        overworld.player.facing = facing;
        overworld.player.mode = mode;
        apply_state_block_overrides(&mut overworld, &state)?;
        apply_state_object_overrides(&mut overworld, &state)?;
        runtime.sync_current_map_scene(&mut state, &map_name)?;
        if let Some(music) = state.script_runtime.current_music.as_deref() {
            if !runtime.audio.contains_music(music) {
                anyhow::bail!("saved state references missing runtime music asset {music}");
            }
        }
        Ok(Self {
            joypad: JoypadState::from_previous_mask(state.joypad.h_joy_down),
            state,
            overworld,
        })
    }

    pub fn apply_buttons(
        &mut self,
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        buttons: impl IntoIterator<Item = GameButton>,
    ) -> Result<RuntimeOverworldFrame> {
        let update = self.joypad.update(buttons, 0xff);
        self.state.joypad.h_joy_pressed = update.h_joy_pressed;
        self.state.joypad.h_joy_down = update.h_joy_down;
        self.state.joypad.h_joy_last = update.h_joy_down;
        self.state.joypad.h_joypad_pressed = update.h_joy_pressed;
        self.state.joypad.h_joypad_down = update.h_joy_down;
        self.state.joypad.h_joypad_sum |= update.h_joy_down;

        let direction = direction_from_mask(update.h_joy_down)?;
        let mut movement = None;
        let mut step_events = None;
        let mut coord_event = None;
        let mut warp = None;
        let mut connection = None;
        let mut interaction = None;
        let mut wild_encounter = None;
        let mut wild_battle = None;

        if let Some(direction) = direction {
            let result = self
                .overworld
                .step_and_check_warp(direction, StepOptions::default());
            let moved = matches!(result.outcome, StepOutcome::Moved { .. });
            movement = Some(result.outcome);
            if let Some(trigger) = result.warp {
                let transition = runtime.resolve_warp_transition(&trigger)?;
                update_dig_warp_memory_for_transition(runtime, &mut self.state, &transition)?;
                let destination = &transition.destination;
                self.overworld = runtime.overworld_session_for(
                    asset_root,
                    &destination.map_name,
                    destination.tile,
                    self.overworld.frame,
                )?;
                apply_state_block_overrides(&mut self.overworld, &self.state)?;
                apply_state_object_overrides(&mut self.overworld, &self.state)?;
                runtime.sync_current_map_music(&mut self.state, &destination.map_name)?;
                runtime.sync_current_map_scene(&mut self.state, &destination.map_name)?;
                warp = Some(transition);
            } else if let Some(trigger) = self.overworld.check_connection() {
                let transition = runtime.resolve_connection_transition(&trigger)?;
                let destination = &transition.destination;
                self.overworld = runtime.overworld_session_for(
                    asset_root,
                    &destination.map_name,
                    destination.tile,
                    self.overworld.frame,
                )?;
                apply_state_block_overrides(&mut self.overworld, &self.state)?;
                apply_state_object_overrides(&mut self.overworld, &self.state)?;
                runtime.sync_current_map_music(&mut self.state, &destination.map_name)?;
                runtime.sync_current_map_scene(&mut self.state, &destination.map_name)?;
                connection = Some(transition);
            } else if moved {
                step_events = Some(process_step(
                    &runtime.data.step_event_rules,
                    &mut self.state.step_events,
                    &mut self.state.storage.party,
                ));
                decrement_active_repel(&mut self.state);
                self.state.sync_party_from_storage();
                coord_event = self.check_coord_event_after_step();
                wild_encounter = self.check_wild_encounter_after_step(runtime)?;
                if let Some(encounter) = wild_encounter
                    .clone()
                    .filter(|roll| roll.resolved.is_some())
                {
                    wild_battle = Some(self.start_wild_battle(runtime, encounter)?);
                }
            }
        } else {
            self.overworld.frame += 1;
        }

        if update.h_joy_pressed & B_PAD_A != 0 {
            interaction = self
                .overworld
                .check_interaction(StepOptions::default().stride_tiles);
        }

        self.state.frame_counter = self.overworld.frame;
        let snapshot = self.overworld.snapshot();
        self.state.overworld = OverworldMemory::from_snapshot(&snapshot);
        let state_checksum =
            game_state_checksum(&self.state).context("checksum authoritative GameState")?;
        Ok(RuntimeOverworldFrame {
            snapshot,
            input_mask: update.h_joy_down,
            pressed_mask: update.h_joy_pressed,
            movement,
            step_events,
            coord_event,
            interaction,
            warp,
            connection,
            wild_encounter,
            wild_battle,
            state_checksum,
        })
    }

    pub fn snapshot(&self) -> OverworldSnapshot {
        self.overworld.snapshot()
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn overworld(&self) -> &OverworldSession {
        &self.overworld
    }

    pub fn state_checksum_frame(&self, player_id: PlayerId) -> Result<StateChecksumFrame> {
        StateChecksumFrame::from_game_state(player_id, &self.state)
            .context("checksum authoritative GameState for player")
    }

    fn runtime_time_update(&self, context: &'static str) -> Result<RuntimeTimeUpdate> {
        let state_checksum = game_state_checksum(&self.state).context(context)?;
        Ok(RuntimeTimeUpdate {
            time_of_day: self.state.time.time_of_day,
            day_of_week: self.state.time.day_of_week,
            game_time_hours: self.state.time.game_time_hours,
            game_time_minutes: self.state.time.game_time_minutes,
            state_checksum,
        })
    }

    pub fn start_scripted_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<StaticWildBattleStart> {
        self.require_current_map(map_name)?;
        let module = runtime
            .data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?;
        let battle = module
            .scripted_wild_battles
            .iter()
            .find(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == startbattle_command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no scripted wild battle at {source_script}:{startbattle_command_index}"
                )
            })?;
        for flag in &battle.pre_battle_event_flags {
            self.state
                .flags
                .set_event_flag(flag, true)
                .with_context(|| format!("set pre-battle event flag {flag}"))?;
        }
        let mut rng = Random::new(self.state.rng_seed);
        let start = runtime
            .data
            .static_wild_battle_start(battle.request.clone(), &mut rng)
            .with_context(|| {
                format!(
                    "start scripted wild battle at {map_name}/{source_script}:{startbattle_command_index}"
                )
        })?;
        self.state.rng_seed = rng.seed();
        self.state.battle = BattleMemory::from(&start);
        self.state.pokedex.record_seen_pokemon(&start.enemy_pokemon);
        self.state.battle_active_party_index = first_available_party_index(&self.state);
        self.state.battle_active_enemy_party_index = Some(0);
        self.state.battle_rewarded_enemy_party_indices.clear();
        self.state.battle_escape_attempts = 0;
        self.state.battle_player_stat_drop_guard_turns = 0;
        Ok(start)
    }

    pub fn start_scripted_trainer_battle(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<TrainerBattleStartStatus> {
        self.require_current_map(map_name)?;
        let module = runtime
            .data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?;
        let battle = module
            .scripted_trainer_battles
            .iter()
            .find(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == startbattle_command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no scripted trainer battle at {source_script}:{startbattle_command_index}"
                )
            })?;
        let start = runtime
            .data
            .trainer_battle_start(&self.state, battle.request.clone())
            .with_context(|| {
                format!(
                    "start scripted trainer battle at {map_name}/{source_script}:{startbattle_command_index}"
                )
        })?;
        if let TrainerBattleStartStatus::Started(started) = &start {
            self.state.battle = BattleMemory::from(started);
            self.state
                .pokedex
                .record_seen_pokemon(&started.enemy_pokemon);
            self.state.battle_active_party_index = first_available_party_index(&self.state);
            self.state.battle_active_enemy_party_index = Some(0);
            self.state.battle_rewarded_enemy_party_indices.clear();
            self.state.battle_escape_attempts = 0;
            self.state.battle_player_stat_drop_guard_turns = 0;
        }
        Ok(start)
    }

    pub fn complete_scripted_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        startbattle_command_index: usize,
    ) -> Result<RuntimeScriptedBattleCompletion> {
        self.require_current_map(map_name)?;
        let module = runtime
            .data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?;
        let battle = module
            .scripted_wild_battles
            .iter()
            .find(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == startbattle_command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no scripted wild battle at {source_script}:{startbattle_command_index}"
                )
            })?;
        let effects = ScriptedBattleEffects {
            event_flags: battle.post_battle_event_flags.clone(),
            script_flags: battle.post_battle_script_flags.clone(),
            disappear_object_ids: battle.disappear_object_ids.clone(),
        };
        let outcome = apply_scripted_battle_effects_to_session(
            &mut self.state,
            &mut self.overworld,
            &effects,
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "complete scripted wild battle at {map_name}/{source_script}:{startbattle_command_index}: {error:?}"
            )
        })?;
        self.state.battle = BattleMemory::Inactive;
        self.state.battle_active_party_index = None;
        self.state.battle_active_enemy_party_index = None;
        self.state.battle_rewarded_enemy_party_indices.clear();
        self.state.battle_escape_attempts = 0;
        self.state.battle_player_stat_drop_guard_turns = 0;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum scripted wild battle completion")?;
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle: true,
            effects: Some(outcome),
            trainer_prize_money: None,
            money_after: None,
            state_checksum,
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
        self.require_current_map(map_name)?;
        let module = runtime
            .data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?;
        let battle = module
            .scripted_trainer_battles
            .iter()
            .find(|battle| {
                battle.source_script == source_script
                    && battle.startbattle_command_index == startbattle_command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no scripted trainer battle at {source_script}:{startbattle_command_index}"
                )
            })?;
        let completion = TrainerBattleCompletion {
            trainer_id: battle.request.trainer_id.clone(),
            trainer_class: battle.request.trainer_class.clone(),
            event_flag: battle.request.event_flag.clone(),
            won,
            can_lose,
        };
        let completion_outcome = complete_trainer_battle(
            &mut self.state,
            &runtime.data.currency_constants,
            &completion,
        )
            .with_context(|| {
                format!(
                    "complete scripted trainer battle at {map_name}/{source_script}:{startbattle_command_index}"
                )
        })?;
        let continued_after_battle = completion_outcome.continued_after_battle;
        if continued_after_battle {
            self.state.battle_active_party_index = None;
            self.state.battle_active_enemy_party_index = None;
            self.state.battle_rewarded_enemy_party_indices.clear();
            self.state.battle_escape_attempts = 0;
            self.state.battle_player_stat_drop_guard_turns = 0;
        }
        let effects = if continued_after_battle {
            let effects = ScriptedBattleEffects {
                event_flags: battle.post_battle_event_flags.clone(),
                script_flags: battle.post_battle_script_flags.clone(),
                disappear_object_ids: Vec::new(),
            };
            Some(
                apply_scripted_battle_effects_to_session(
                    &mut self.state,
                    &mut self.overworld,
                    &effects,
                )
                .map_err(|error| {
                    anyhow::anyhow!(
                        "apply scripted trainer post-battle effects at {map_name}/{source_script}:{startbattle_command_index}: {error:?}"
                    )
                })?,
            )
        } else {
            None
        };
        let state_checksum = game_state_checksum(&self.state)
            .context("checksum scripted trainer battle completion")?;
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle,
            effects,
            trainer_prize_money: Some(completion_outcome.prize_money),
            money_after: Some(completion_outcome.money_after),
            state_checksum,
        })
    }

    pub fn throw_ball_at_active_battle(
        &mut self,
        runtime: &CrystalRuntime,
        ball_id: &str,
    ) -> Result<RuntimeCaptureAttempt> {
        let ball = runtime
            .data
            .items
            .get(ball_id)
            .with_context(|| format!("compiled game pack missing ball item {ball_id}"))?;
        validate_capture_ball_item(&runtime.data.capture_rules, ball).with_context(|| {
            format!("battle capture item {ball_id} is not declared by exact capture rules")
        })?;
        if !ball.battle_usable {
            anyhow::bail!("battle capture item {ball_id} is not usable in battle");
        }
        let active_index = require_active_party_index(&self.state)?;
        let player = self.state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let (enemy, context) = match &self.state.battle {
            BattleMemory::Wild {
                battle_type,
                enemy_pokemon,
                ..
            }
            | BattleMemory::StaticWild {
                battle_type,
                enemy_pokemon,
                ..
            } => {
                let mut context = CaptureAttemptContext::wild(ball_id);
                context.battle_type = battle_type.clone();
                (enemy_pokemon.clone(), context)
            }
            BattleMemory::Trainer {
                battle_type,
                enemy_pokemon,
                ..
            } => {
                let mut context = CaptureAttemptContext::wild(ball_id);
                context.battle_type = battle_type.clone();
                context.trainer_battle = true;
                (enemy_pokemon.clone(), context)
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot throw a ball without an active battle");
            }
        };
        let mut rng = Random::new(self.state.rng_seed);
        let outcome = throw_ball_from_bag(
            &mut self.state.bag,
            ball,
            &player,
            &enemy,
            context,
            &runtime.data.capture_rules,
            &runtime.data.capture_wobble_probabilities,
            &mut rng,
        )
        .map_err(|error| anyhow::anyhow!("throw ball {ball_id}: {error}"))?;
        self.state.rng_seed = rng.seed();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum capture attempt")?;
        Ok(RuntimeCaptureAttempt {
            outcome,
            state_checksum,
        })
    }

    pub fn complete_active_wild_capture(
        &mut self,
        outcome: &CaptureOutcome,
    ) -> Result<RuntimeCaptureCompletion> {
        let enemy_pokemon = match &self.state.battle {
            BattleMemory::Wild { enemy_pokemon, .. }
            | BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
            BattleMemory::Trainer { trainer_id, .. } => {
                anyhow::bail!("cannot capture during trainer battle {trainer_id}");
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot complete capture without an active wild battle");
            }
        };
        let stored = complete_captured_pokemon(
            outcome,
            &mut self.state.storage,
            &mut self.state.pokedex,
            enemy_pokemon,
        )
        .map_err(|error| anyhow::anyhow!("complete captured Pokemon: {error}"))?;
        if stored.is_some() {
            self.state.battle_result |= 1 << 6;
            self.state.battle = BattleMemory::Inactive;
            self.state.battle_active_party_index = None;
            self.state.battle_active_enemy_party_index = None;
            self.state.battle_rewarded_enemy_party_indices.clear();
            self.state.battle_escape_attempts = 0;
            self.state.battle_player_stat_drop_guard_turns = 0;
        }
        self.state.sync_party_from_storage();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum capture completion")?;
        Ok(RuntimeCaptureCompletion {
            stored,
            state_checksum,
        })
    }

    pub fn resolve_active_battle_turn(
        &mut self,
        runtime: &CrystalRuntime,
        player_action: BattleAction,
        enemy_action: BattleAction,
    ) -> Result<RuntimeBattleTurn> {
        let active_index = match player_action {
            BattleAction::Switch { party_index } => {
                validate_active_party_index(&self.state, party_index)?;
                self.state.battle_active_party_index = Some(party_index);
                party_index
            }
            _ => require_active_party_index(&self.state)?,
        };
        let player = self.state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let enemy = match &self.state.battle {
            BattleMemory::Wild { enemy_pokemon, .. }
            | BattleMemory::StaticWild { enemy_pokemon, .. }
            | BattleMemory::Trainer { enemy_pokemon, .. } => enemy_pokemon.clone(),
            BattleMemory::Inactive => {
                anyhow::bail!("cannot resolve battle turn without an active battle");
            }
        };
        let mut rng = Random::new(self.state.rng_seed);
        let combat = BattleCombatState::new(player, enemy, self.state.rng_seed);
        let outcome = resolve_battle_turn_with_items(
            combat,
            BattleTurnInput {
                player: player_action,
                enemy: enemy_action,
            },
            &runtime.data.moves,
            &runtime.data.items,
            &runtime.data.move_priorities,
            &runtime.data.battle_stat_multipliers,
            &runtime.data.type_categories,
            &runtime.data.type_effectiveness,
            &runtime.data.weather_modifiers,
            &mut rng,
        )
        .map_err(|error| anyhow::anyhow!("resolve active battle turn: {error:?}"))?;
        self.state.rng_seed = outcome.state.rng_seed_after;
        self.state.storage.party.pokemon[active_index] = Some(outcome.state.player.clone());
        self.state.sync_party_from_storage();
        update_active_battle_enemy(&mut self.state, outcome.state.enemy.clone())?;
        let state_checksum = game_state_checksum(&self.state).context("checksum battle turn")?;
        Ok(RuntimeBattleTurn {
            outcome,
            state_checksum,
        })
    }

    pub fn resolve_active_battle_command(
        &mut self,
        runtime: &CrystalRuntime,
        player_action: BattleAction,
        enemy_action: BattleAction,
    ) -> Result<RuntimeBattleCommand> {
        match player_action {
            BattleAction::Run => self
                .attempt_escape_active_wild_battle(runtime)
                .map(RuntimeBattleCommand::Escape),
            action => self
                .resolve_active_battle_turn(runtime, action, enemy_action)
                .map(RuntimeBattleCommand::Turn),
        }
    }

    pub fn attempt_escape_active_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeBattleEscape> {
        let active_index = require_active_party_index(&self.state)?;
        let player = self.state.storage.party.pokemon[active_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("active battle party index {active_index} has no Pokemon"))?;
        let enemy = match &self.state.battle {
            BattleMemory::Wild { enemy_pokemon, .. }
            | BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
            BattleMemory::Trainer { trainer_id, .. } => {
                anyhow::bail!("cannot escape from trainer battle {trainer_id}");
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot escape without an active wild battle");
            }
        };
        let mut rng = Random::new(self.state.rng_seed);
        let outcome = attempt_wild_battle_escape(
            &player,
            &enemy,
            &runtime.data.battle_stat_multipliers,
            &runtime.data.battle_escape_rules,
            self.state.battle_escape_attempts,
            &mut rng,
        )
        .map_err(|error| anyhow::anyhow!("attempt wild battle escape: {error:?}"))?;
        self.state.rng_seed = outcome.rng_seed_after;
        self.state.battle_escape_attempts = outcome.attempts_after;
        if outcome.escaped {
            self.state.battle = BattleMemory::Inactive;
            self.state.battle_active_party_index = None;
            self.state.battle_active_enemy_party_index = None;
            self.state.battle_rewarded_enemy_party_indices.clear();
            self.state.battle_escape_attempts = 0;
            self.state.battle_player_stat_drop_guard_turns = 0;
        }
        let state_checksum = game_state_checksum(&self.state).context("checksum battle escape")?;
        Ok(RuntimeBattleEscape {
            outcome,
            state_checksum,
        })
    }

    pub fn use_bag_item_to_escape_active_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBattleEscapeItemUse> {
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let battle_escape_mode = validate_battle_escape_item(item)
            .map_err(|error| anyhow::anyhow!("validate battle escape item {item_id}: {error:?}"))?;
        match &self.state.battle {
            BattleMemory::Wild { .. } | BattleMemory::StaticWild { .. } => {}
            BattleMemory::Trainer { trainer_id, .. } => {
                anyhow::bail!(
                    "cannot use battle escape item {item_id} in trainer battle {trainer_id}"
                );
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot use battle escape item {item_id} without an active battle");
            }
        }

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Battle,
            },
        )
        .map_err(|error| anyhow::anyhow!("use battle escape item {item_id}: {error:?}"))?;
        self.state.battle = BattleMemory::Inactive;
        self.state.battle_active_party_index = None;
        self.state.battle_active_enemy_party_index = None;
        self.state.battle_rewarded_enemy_party_indices.clear();
        self.state.battle_escape_attempts = 0;
        self.state.battle_player_stat_drop_guard_turns = 0;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum battle escape item use")?;
        Ok(RuntimeBattleEscapeItemUse {
            item_use,
            battle_escape_mode: battle_escape_mode.to_string(),
            escaped: true,
            state_checksum,
        })
    }

    pub fn use_bag_guard_spec_in_active_battle(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBattleStateItemUse> {
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let turns = validate_battle_stat_drop_guard_item(item).map_err(|error| {
            anyhow::anyhow!("validate battle stat drop guard item {item_id}: {error:?}")
        })?;
        match &self.state.battle {
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {}
            BattleMemory::Inactive => {
                anyhow::bail!("cannot use battle state item {item_id} without an active battle");
            }
        }

        let before = self.state.battle_player_stat_drop_guard_turns;
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Battle,
            },
        )
        .map_err(|error| anyhow::anyhow!("use battle state item {item_id}: {error:?}"))?;
        self.state.battle_player_stat_drop_guard_turns = turns;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum battle state item use")?;
        Ok(RuntimeBattleStateItemUse {
            item_use,
            stat_drop_guard_turns_before: before,
            stat_drop_guard_turns_after: self.state.battle_player_stat_drop_guard_turns,
            state_checksum,
        })
    }

    pub fn advance_active_trainer_battle(&mut self) -> Result<RuntimeTrainerBattleAdvance> {
        let current_enemy_index = require_active_enemy_party_index(&self.state)?;
        if !self
            .state
            .battle_rewarded_enemy_party_indices
            .contains(&current_enemy_index)
        {
            anyhow::bail!(
                "trainer battle enemy party index {current_enemy_index} rewards have not been claimed"
            );
        }
        let BattleMemory::Trainer {
            enemy_pokemon,
            enemy_party,
            ..
        } = &mut self.state.battle
        else {
            anyhow::bail!("cannot advance trainer battle without an active trainer battle");
        };
        if enemy_pokemon.hp != 0 {
            anyhow::bail!("cannot advance trainer battle before active enemy fainted");
        }
        if current_enemy_index >= enemy_party.len() {
            anyhow::bail!(
                "active enemy party index {current_enemy_index} is outside trainer party"
            );
        }
        enemy_party[current_enemy_index] = enemy_pokemon.clone();
        let next = enemy_party
            .iter()
            .enumerate()
            .skip(current_enemy_index + 1)
            .find_map(|(index, pokemon)| (pokemon.hp > 0).then_some((index, pokemon.clone())));
        let (next_enemy, trainer_defeated) = if let Some((index, pokemon)) = next {
            *enemy_pokemon = pokemon.clone();
            self.state.battle_active_enemy_party_index = Some(index);
            self.state.pokedex.record_seen_pokemon(&pokemon);
            (Some(pokemon), false)
        } else {
            (None, true)
        };
        let state_checksum =
            game_state_checksum(&self.state).context("checksum trainer battle advance")?;
        Ok(RuntimeTrainerBattleAdvance {
            next_enemy,
            trainer_defeated,
            state_checksum,
        })
    }

    pub fn claim_active_trainer_battle_rewards(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeBattleRewards> {
        let enemy = match &self.state.battle {
            BattleMemory::Trainer { enemy_pokemon, .. } => enemy_pokemon.clone(),
            BattleMemory::Wild { .. } | BattleMemory::StaticWild { .. } => {
                anyhow::bail!("trainer battle rewards require an active trainer battle");
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot claim trainer battle rewards without an active battle");
            }
        };
        let enemy_index = require_active_enemy_party_index(&self.state)?;
        if self
            .state
            .battle_rewarded_enemy_party_indices
            .contains(&enemy_index)
        {
            anyhow::bail!("trainer battle enemy party index {enemy_index} rewards already claimed");
        }
        let active_index = require_active_party_index(&self.state)?;
        let player = self.state.storage.party.pokemon[active_index]
            .as_mut()
            .with_context(|| "player party Pokemon disappeared during trainer rewards")?;
        let outcome = apply_trainer_battle_rewards(
            &runtime.data.battle_reward_rules,
            player,
            &enemy,
            &runtime.data.pokemon,
            &runtime.data.moves,
            &runtime.data.learnsets,
            &runtime.data.growth_rates,
            &runtime.data.evolutions,
            self.state.time.time_of_day,
        )
        .map_err(|error| anyhow::anyhow!("claim trainer battle rewards: {error:?}"))?;
        self.state.sync_party_from_storage();
        update_active_battle_enemy(&mut self.state, enemy)?;
        self.state
            .battle_rewarded_enemy_party_indices
            .insert(enemy_index);
        let state_checksum =
            game_state_checksum(&self.state).context("checksum trainer battle rewards")?;
        Ok(RuntimeBattleRewards {
            outcome,
            state_checksum,
        })
    }

    pub fn claim_active_wild_battle_rewards(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimeBattleRewards> {
        let enemy = match &self.state.battle {
            BattleMemory::Wild { enemy_pokemon, .. }
            | BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
            BattleMemory::Trainer { trainer_id, .. } => {
                anyhow::bail!(
                    "trainer battle {trainer_id} rewards require trainer-completion sequencing"
                );
            }
            BattleMemory::Inactive => {
                anyhow::bail!("cannot claim battle rewards without an active wild battle");
            }
        };
        let active_index = require_active_party_index(&self.state)?;
        let player = self.state.storage.party.pokemon[active_index]
            .as_mut()
            .with_context(|| "player party Pokemon disappeared during rewards")?;
        let outcome = apply_wild_battle_rewards(
            &runtime.data.battle_reward_rules,
            player,
            &enemy,
            &runtime.data.pokemon,
            &runtime.data.moves,
            &runtime.data.learnsets,
            &runtime.data.growth_rates,
            &runtime.data.evolutions,
            self.state.time.time_of_day,
        )
        .map_err(|error| anyhow::anyhow!("claim wild battle rewards: {error:?}"))?;
        self.state.battle = BattleMemory::Inactive;
        self.state.battle_active_party_index = None;
        self.state.battle_active_enemy_party_index = None;
        self.state.battle_rewarded_enemy_party_indices.clear();
        self.state.battle_escape_attempts = 0;
        self.state.battle_player_stat_drop_guard_turns = 0;
        self.state.sync_party_from_storage();
        let state_checksum = game_state_checksum(&self.state).context("checksum battle rewards")?;
        Ok(RuntimeBattleRewards {
            outcome,
            state_checksum,
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
        nickname: Option<String>,
    ) -> Result<RuntimeGiftPokemonGrant> {
        self.require_current_map(map_name)?;
        let module = runtime
            .data
            .maps
            .get(map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?;
        let gift = module
            .gift_pokemon_scripts
            .iter()
            .find(|gift| gift.source_script == source_script && gift.command_index == command_index)
            .with_context(|| {
                format!(
                    "map {map_name} has no gift Pokemon script at {source_script}:{command_index}"
                )
            })?;
        if gift.nickname_label.is_some() && nickname.is_none() {
            let nickname_label = gift
                .nickname_label
                .as_deref()
                .expect("nickname label was checked as present");
            anyhow::bail!(
                "gift Pokemon at {map_name}/{source_script}:{command_index} requires resolved nickname label {}",
                nickname_label
            );
        }
        if gift.nickname_label.is_none() && nickname.is_some() {
            anyhow::bail!(
                "gift Pokemon at {map_name}/{source_script}:{command_index} does not declare a nickname label"
            );
        }
        let outcome = give_gift_pokemon(
            &mut self.state.storage,
            &runtime.data.pokemon,
            &runtime.data.learnsets,
            &runtime.data.moves,
            &runtime.data.growth_rates,
            &runtime.data.items,
            GiftPokemonRequest {
                species_id: gift.species_id.clone(),
                level: gift.level,
                held_item_id: gift.held_item_id.clone(),
                nickname,
                original_trainer_name: original_trainer_name.into(),
                original_trainer_id,
                source_script: gift.source_script.clone(),
                command_index: gift.command_index,
                egg: gift.egg,
                dvs,
            },
        )
        .map_err(|error| {
            anyhow::anyhow!(
                "grant gift Pokemon at {map_name}/{source_script}:{command_index}: {error:?}"
            )
        })?;
        if !gift.egg {
            self.state.pokedex.record_caught_pokemon(&outcome.pokemon);
        }
        self.state.sync_party_from_storage();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum gift Pokemon grant")?;
        Ok(RuntimeGiftPokemonGrant {
            outcome,
            state_checksum,
        })
    }

    pub fn use_bag_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        context: ItemUseContext,
    ) -> Result<RuntimeItemUse> {
        let outcome = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context,
            },
        )
        .map_err(|error| anyhow::anyhow!("use bag item {item_id}: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum bag item use")?;
        Ok(RuntimeItemUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_bag_repel_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeRepelItemUse> {
        match &self.state.battle {
            BattleMemory::Inactive => {}
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use field repel item {item_id} during an active battle");
            }
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let steps = validate_repel_item(&runtime.data.field_moves, item)
            .map_err(|error| anyhow::anyhow!("validate field repel item {item_id}: {error:?}"))?;

        let repel_steps_before = self.state.repel_steps_remaining;
        let active_repel_item_before = self.state.active_repel_item.clone();
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field repel item {item_id}: {error:?}"))?;
        self.state.repel_steps_remaining = steps;
        self.state.active_repel_item = Some(item_id.to_string());
        let state_checksum = game_state_checksum(&self.state).context("checksum repel item use")?;
        Ok(RuntimeRepelItemUse {
            item_use,
            repel_steps_before,
            repel_steps_after: self.state.repel_steps_remaining,
            active_repel_item_before,
            active_repel_item_after: self.state.active_repel_item.clone(),
            state_checksum,
        })
    }

    pub fn use_bag_bicycle_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBicycleItemUse> {
        self.require_no_active_battle("field bicycle item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_bicycle_item(&runtime.data.field_moves, item)
            .map_err(|error| anyhow::anyhow!("validate field bicycle item {item_id}: {error:?}"))?;
        if !item.field_usable {
            anyhow::bail!("field bicycle item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field bicycle item {item_id} is not in the bag");
        }
        let map_name = self.overworld.map.name.clone();
        let metadata = runtime.runtime_map_metadata_for_name(&map_name)?;
        if !is_bicycle_environment(&metadata.environment) {
            anyhow::bail!(
                "cannot use field bicycle item {item_id} in environment {}",
                metadata.environment
            );
        }
        let sample = sample_collision(
            &self.overworld.map,
            &self.overworld.tileset,
            self.overworld.player.tile,
        )
        .with_context(|| {
            format!(
                "field bicycle item {item_id} cannot sample current tile {},{}",
                self.overworld.player.tile.x, self.overworld.player.tile.y
            )
        })?;
        if sample.permission & 0x0f != permissions::FLOOR {
            anyhow::bail!(
                "cannot use field bicycle item {item_id} on permission {:#04x}",
                sample.permission
            );
        }
        let mode_before = self.overworld.player.mode;
        let always_on_bike = self
            .state
            .flags
            .is_engine_flag_set("ENGINE_ALWAYS_ON_BIKE")
            .context("check ENGINE_ALWAYS_ON_BIKE")?;
        let mode_after = match mode_before {
            MovementMode::Normal => MovementMode::Bike,
            MovementMode::Bike if always_on_bike => {
                anyhow::bail!("cannot get off bicycle while ENGINE_ALWAYS_ON_BIKE is set");
            }
            MovementMode::Bike => MovementMode::Normal,
            MovementMode::Skate | MovementMode::Surf => {
                anyhow::bail!("cannot toggle bicycle from movement mode {mode_before:?}");
            }
        };
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field bicycle item {item_id}: {error:?}"))?;
        self.overworld.player.mode = mode_after;
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        self.state.frame_counter = self.overworld.frame;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field bicycle item use")?;
        Ok(RuntimeBicycleItemUse {
            item_use,
            map_name,
            permission: sample.permission,
            mode_before,
            mode_after,
            state_checksum,
        })
    }

    pub fn use_bag_itemfinder_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeItemfinderUse> {
        self.require_no_active_battle("field itemfinder item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_itemfinder_item(&runtime.data.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field itemfinder item {item_id}: {error:?}")
        })?;
        if !item.field_usable {
            anyhow::bail!("field itemfinder item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field itemfinder item {item_id} is not in the bag");
        }
        let map_name = self.overworld.map.name.clone();
        let map_module = runtime
            .data
            .maps
            .get(&map_name)
            .with_context(|| format!("compiled game pack missing current map {map_name}"))?;
        let found = find_itemfinder_hidden_item(
            map_module,
            &self.state,
            self.overworld.player.tile,
            &map_name,
        )?;
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field itemfinder item {item_id}: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field itemfinder item use")?;
        Ok(RuntimeItemfinderUse {
            item_use,
            player_tile: self.overworld.player.tile,
            itemfinder_sound_cues: if found.is_some() { 8 } else { 0 },
            found,
            state_checksum,
        })
    }

    pub fn use_bag_squirtbottle_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeSquirtBottleUse> {
        self.require_no_active_battle("field squirtbottle item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_squirtbottle_item(&runtime.data.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field squirtbottle item {item_id}: {error:?}")
        })?;
        if !item.field_usable {
            anyhow::bail!("field squirtbottle item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field squirtbottle item {item_id} is not in the bag");
        }
        let map_name = self.overworld.map.name.clone();
        let module = runtime.map_module(&map_name)?;
        let target = squirtbottle_target(&self.overworld, module)?;
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field squirtbottle item {item_id}: {error:?}"))?;
        if let Some(script) = target.target_script.as_ref() {
            self.state.script_runtime.next_script = Some(script.clone());
            self.state.script_runtime.last_talked_object = target.target_object_identifier.clone();
            self.overworld.last_talked_object_identifier = target.target_object_identifier.clone();
            self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        }
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field squirtbottle item use")?;
        Ok(RuntimeSquirtBottleUse {
            item_use,
            player_tile: self.overworld.player.tile,
            target_tile: target.target_tile,
            target_object_identifier: target.target_object_identifier,
            target_movement: target.target_movement,
            target_script: target.target_script,
            state_checksum,
        })
    }

    pub fn use_bag_coin_case_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeKeyItemBalanceUse> {
        self.require_no_active_battle("field coin case item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_coin_case_item(&runtime.data.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field coin case item {item_id}: {error:?}")
        })?;
        if !item.field_usable {
            anyhow::bail!("field coin case item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field coin case item {item_id} is not in the bag");
        }
        let balance = u32::from(self.state.coins);
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field coin case item {item_id}: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field coin case item use")?;
        Ok(RuntimeKeyItemBalanceUse {
            item_use,
            balance_label: "COIN".to_string(),
            balance,
            state_checksum,
        })
    }

    pub fn use_bag_blue_card_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeKeyItemBalanceUse> {
        self.require_no_active_battle("field blue card item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_blue_card_item(&runtime.data.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field blue card item {item_id}: {error:?}")
        })?;
        if !item.field_usable {
            anyhow::bail!("field blue card item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field blue card item {item_id} is not in the bag");
        }
        let balance = u32::from(blue_card_balance(&self.state)?);
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field blue card item {item_id}: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field blue card item use")?;
        Ok(RuntimeKeyItemBalanceUse {
            item_use,
            balance_label: "POINT".to_string(),
            balance,
            state_checksum,
        })
    }

    pub fn use_bag_town_map_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeTownMapUse> {
        self.require_no_active_battle("field town map item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_town_map_item(&runtime.data.field_moves, item).map_err(|error| {
            anyhow::anyhow!("validate field town map item {item_id}: {error:?}")
        })?;
        if !item.field_usable {
            anyhow::bail!("field town map item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field town map item {item_id} is not in the bag");
        }
        let map_name = self.overworld.map.name.clone();
        let metadata = runtime.runtime_map_metadata_for_name(&map_name)?;
        let landmark = town_map_landmark_for_map(&runtime.data, &map_name)?;
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field town map item {item_id}: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field town map item use")?;
        Ok(RuntimeTownMapUse {
            item_use,
            map_name,
            map_constant: metadata.constant.clone(),
            environment: metadata.environment.clone(),
            landmark,
            state_checksum,
        })
    }

    pub fn use_bag_escape_rope_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        item_id: &str,
    ) -> Result<RuntimeEscapeRopeUse> {
        match &self.state.battle {
            BattleMemory::Inactive => {}
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use field escape item {item_id} during an active battle");
            }
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        validate_field_escape_item(&runtime.data.field_moves, item)
            .map_err(|error| anyhow::anyhow!("use field escape item {item_id}: {error:?}"))?;

        let current_map = self.overworld.map.name.clone();
        let current_metadata = runtime.runtime_map_metadata_for_name(&current_map)?;
        if !is_escape_rope_environment(&current_metadata.environment) {
            anyhow::bail!(
                "cannot use field escape item {item_id} in environment {}",
                current_metadata.environment
            );
        }
        let destination =
            self.saved_dig_warp_destination(runtime, &format!("field escape item {item_id}"))?;

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field escape item {item_id}: {error:?}"))?;
        self.overworld = runtime.overworld_session_for(
            asset_root,
            &destination.map_name,
            destination.tile,
            self.overworld.frame,
        )?;
        apply_state_block_overrides(&mut self.overworld, &self.state)?;
        apply_state_object_overrides(&mut self.overworld, &self.state)?;
        runtime.sync_current_map_music(&mut self.state, &destination.map_name)?;
        runtime.sync_current_map_scene(&mut self.state, &destination.map_name)?;
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        self.state.frame_counter = self.overworld.frame;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum escape rope item use")?;
        Ok(RuntimeEscapeRopeUse {
            item_use,
            source_map: current_map,
            destination_map: destination.map_name,
            destination_warp_index: destination.warp_index,
            destination_tile: destination.tile,
            state_checksum,
        })
    }

    pub fn use_cut_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        self.require_no_active_battle("CUT field move")?;
        let map_name = self.overworld.map.name.clone();
        self.require_current_map(&map_name)?;
        let module = runtime.map_module(&map_name)?;
        let storage = self.state.storage.clone();
        let outcome = apply_cut_field_move(
            &runtime.data.field_moves,
            &mut self.state,
            &storage,
            &mut self.overworld.map,
            &self.overworld.tileset,
            &module.attributes.tileset_name,
            party_index,
            metatile_x,
            metatile_y,
        )
        .map_err(|error| anyhow::anyhow!("use CUT field move: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum CUT field move")?;
        Ok(RuntimeFieldMoveBlockUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_whirlpool_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        metatile_x: u16,
        metatile_y: u16,
    ) -> Result<RuntimeFieldMoveBlockUse> {
        self.require_no_active_battle("WHIRLPOOL field move")?;
        let map_name = self.overworld.map.name.clone();
        self.require_current_map(&map_name)?;
        let module = runtime.map_module(&map_name)?;
        let storage = self.state.storage.clone();
        let outcome = apply_whirlpool_field_move(
            &runtime.data.field_moves,
            &mut self.state,
            &storage,
            &mut self.overworld.map,
            &self.overworld.tileset,
            &module.attributes.tileset_name,
            party_index,
            metatile_x,
            metatile_y,
        )
        .map_err(|error| anyhow::anyhow!("use WHIRLPOOL field move: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum WHIRLPOOL field move")?;
        Ok(RuntimeFieldMoveBlockUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_strength_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveFlagUse> {
        self.require_no_active_battle("STRENGTH field move")?;
        let storage = self.state.storage.clone();
        let outcome = apply_strength_field_move(
            &runtime.data.field_moves,
            &mut self.state,
            &storage,
            party_index,
        )
        .map_err(|error| anyhow::anyhow!("use STRENGTH field move: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum STRENGTH field move")?;
        Ok(RuntimeFieldMoveFlagUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_flash_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveFlagUse> {
        self.require_no_active_battle("FLASH field move")?;
        let storage = self.state.storage.clone();
        let outcome = apply_flash_field_move(
            &runtime.data.field_moves,
            &mut self.state,
            &storage,
            party_index,
        )
        .map_err(|error| anyhow::anyhow!("use FLASH field move: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum FLASH field move")?;
        Ok(RuntimeFieldMoveFlagUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_surf_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveTravelUse> {
        self.require_no_active_battle("SURF field move")?;
        let target = move_by_stride(self.overworld.player.tile, self.overworld.player.facing, 2);
        if let Some((_, object)) = self.overworld.visible_object_at(target) {
            anyhow::bail!(
                "cannot use SURF field move onto occupied tile {target:?} by {:?}",
                object.object_identifier
            );
        }
        let storage = self.state.storage.clone();
        let state_snapshot = self.state.clone();
        let outcome = apply_surf_field_move(
            &runtime.data.field_moves,
            &state_snapshot,
            &storage,
            &self.overworld.map,
            &self.overworld.tileset,
            &mut self.overworld.player,
            party_index,
        )
        .map_err(|error| anyhow::anyhow!("use SURF field move: {error:?}"))?;
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        let state_checksum =
            game_state_checksum(&self.state).context("checksum SURF field move")?;
        Ok(RuntimeFieldMoveTravelUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_waterfall_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldMoveTravelUse> {
        self.require_no_active_battle("WATERFALL field move")?;
        let storage = self.state.storage.clone();
        let state_snapshot = self.state.clone();
        let outcome = apply_waterfall_field_move(
            &runtime.data.field_moves,
            &state_snapshot,
            &storage,
            &self.overworld.map,
            &self.overworld.tileset,
            &mut self.overworld.player,
            party_index,
        )
        .map_err(|error| anyhow::anyhow!("use WATERFALL field move: {error:?}"))?;
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        let state_checksum =
            game_state_checksum(&self.state).context("checksum WATERFALL field move")?;
        Ok(RuntimeFieldMoveTravelUse {
            outcome,
            state_checksum,
        })
    }

    pub fn use_fly_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        party_index: usize,
        destination_spawn_identifier: u16,
        flypoint_flag: &str,
    ) -> Result<RuntimeFlyFieldMoveUse> {
        self.require_no_active_battle("FLY field move")?;
        let source_map = self.overworld.map.name.clone();
        let source_metadata = runtime.runtime_map_metadata_for_name(&source_map)?;
        if !is_fly_source_environment(&source_metadata.environment) {
            anyhow::bail!(
                "cannot use FLY field move in environment {}",
                source_metadata.environment
            );
        }
        let fly_rule = validate_fly_field_move(
            &runtime.data.field_moves,
            &self.state,
            &self.state.storage,
            party_index,
        )
        .map_err(|error| anyhow::anyhow!("use FLY field move: {error:?}"))?;
        if !self
            .state
            .flags
            .is_engine_flag_set(flypoint_flag)
            .with_context(|| format!("check FLY destination flag {flypoint_flag}"))?
        {
            anyhow::bail!("FLY destination flag {flypoint_flag} is not set");
        }
        let destination_spawn = runtime.spawn_point(destination_spawn_identifier)?;
        let destination_map = destination_spawn.map_name.clone();
        let destination_tile =
            TilePosition::new(destination_spawn.tile_x, destination_spawn.tile_y);
        self.overworld = runtime.overworld_session_for(
            asset_root,
            &destination_map,
            destination_tile,
            self.overworld.frame,
        )?;
        apply_state_block_overrides(&mut self.overworld, &self.state)?;
        apply_state_object_overrides(&mut self.overworld, &self.state)?;
        runtime.sync_current_map_music(&mut self.state, &destination_map)?;
        runtime.sync_current_map_scene(&mut self.state, &destination_map)?;
        self.state.last_spawn_identifier = Some(destination_spawn_identifier);
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        self.state.frame_counter = self.overworld.frame;
        let state_checksum = game_state_checksum(&self.state).context("checksum FLY field move")?;
        Ok(RuntimeFlyFieldMoveUse {
            actor_party_index: fly_rule.actor_party_index,
            actor_species: fly_rule.actor_species,
            flypoint_flag: flypoint_flag.to_string(),
            source_map,
            destination_spawn_identifier,
            destination_map,
            destination_tile,
            state_checksum,
        })
    }

    pub fn use_dig_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        party_index: usize,
    ) -> Result<RuntimeDigFieldMoveUse> {
        self.require_no_active_battle("DIG field move")?;
        let source_map = self.overworld.map.name.clone();
        let source_metadata = runtime.runtime_map_metadata_for_name(&source_map)?;
        if !is_dig_field_move_environment(&source_metadata.environment) {
            anyhow::bail!(
                "cannot use DIG field move in environment {}",
                source_metadata.environment
            );
        }
        let dig_rule =
            validate_dig_field_move(&runtime.data.field_moves, &self.state.storage, party_index)
                .map_err(|error| anyhow::anyhow!("use DIG field move: {error:?}"))?;
        let destination = self.saved_dig_warp_destination(runtime, "DIG field move")?;
        self.overworld = runtime.overworld_session_for(
            asset_root,
            &destination.map_name,
            destination.tile,
            self.overworld.frame,
        )?;
        apply_state_block_overrides(&mut self.overworld, &self.state)?;
        apply_state_object_overrides(&mut self.overworld, &self.state)?;
        runtime.sync_current_map_music(&mut self.state, &destination.map_name)?;
        runtime.sync_current_map_scene(&mut self.state, &destination.map_name)?;
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        self.state.frame_counter = self.overworld.frame;
        let state_checksum = game_state_checksum(&self.state).context("checksum DIG field move")?;
        Ok(RuntimeDigFieldMoveUse {
            actor_party_index: dig_rule.actor_party_index,
            actor_species: dig_rule.actor_species,
            source_map,
            destination_map: destination.map_name,
            destination_warp_index: destination.warp_index,
            destination_tile: destination.tile,
            state_checksum,
        })
    }

    pub fn use_teleport_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        party_index: usize,
    ) -> Result<RuntimeTeleportFieldMoveUse> {
        self.require_no_active_battle("TELEPORT field move")?;
        let source_map = self.overworld.map.name.clone();
        let source_metadata = runtime.runtime_map_metadata_for_name(&source_map)?;
        if !is_teleport_source_environment(&source_metadata.environment) {
            anyhow::bail!(
                "cannot use TELEPORT field move in environment {}",
                source_metadata.environment
            );
        }
        let teleport_rule = validate_teleport_field_move(
            &runtime.data.field_moves,
            &self.state.storage,
            party_index,
        )
        .map_err(|error| anyhow::anyhow!("use TELEPORT field move: {error:?}"))?;
        let destination_spawn_identifier = self
            .state
            .last_spawn_identifier
            .with_context(|| "TELEPORT field move has no saved spawn identifier")?;
        let destination_spawn = runtime.spawn_point(destination_spawn_identifier)?;
        let destination_map = destination_spawn.map_name.clone();
        let destination_tile =
            TilePosition::new(destination_spawn.tile_x, destination_spawn.tile_y);
        self.overworld = runtime.overworld_session_for(
            asset_root,
            &destination_map,
            destination_tile,
            self.overworld.frame,
        )?;
        apply_state_block_overrides(&mut self.overworld, &self.state)?;
        apply_state_object_overrides(&mut self.overworld, &self.state)?;
        runtime.sync_current_map_music(&mut self.state, &destination_map)?;
        runtime.sync_current_map_scene(&mut self.state, &destination_map)?;
        self.state.last_spawn_identifier = Some(destination_spawn_identifier);
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        self.state.frame_counter = self.overworld.frame;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum TELEPORT field move")?;
        Ok(RuntimeTeleportFieldMoveUse {
            actor_party_index: teleport_rule.actor_party_index,
            actor_species: teleport_rule.actor_species,
            source_map,
            destination_spawn_identifier,
            destination_map,
            destination_tile,
            state_checksum,
        })
    }

    pub fn use_headbutt_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        player_id: u16,
    ) -> Result<RuntimeFieldEncounterMoveUse> {
        self.require_no_active_battle("HEADBUTT field move")?;
        self.require_party_pokemon_knows_move(party_index, "HEADBUTT")?;
        let map_name = self.overworld.map.name.clone();
        let target = move_by_stride(self.overworld.player.tile, self.overworld.player.facing, 2);
        let encounters = runtime
            .data
            .field_encounters
            .get(&map_name)
            .with_context(|| {
                format!("compiled game pack missing field encounters for {map_name}")
            })?;
        let mut rng = Random::new(self.state.rng_seed);
        let chance_roll = rng.randrange(10) as u8;
        let entry_roll = rng.randrange(100) as u8;
        let field_encounter = select_headbutt_encounter(
            encounters,
            target.x,
            target.y,
            player_id,
            chance_roll,
            entry_roll,
        )?;
        self.state.rng_seed = rng.seed();
        let wild_battle = self.start_field_encounter_battle(runtime, &field_encounter)?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum HEADBUTT field move")?;
        Ok(RuntimeFieldEncounterMoveUse {
            field_encounter,
            wild_battle,
            state_checksum,
        })
    }

    pub fn use_rock_smash_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
    ) -> Result<RuntimeFieldEncounterMoveUse> {
        self.require_no_active_battle("ROCK_SMASH field move")?;
        self.require_party_pokemon_knows_move(party_index, "ROCK_SMASH")?;
        let map_name = self.overworld.map.name.clone();
        let target = move_by_stride(self.overworld.player.tile, self.overworld.player.facing, 2);
        let encounters = runtime
            .data
            .field_encounters
            .get(&map_name)
            .with_context(|| {
                format!("compiled game pack missing field encounters for {map_name}")
            })?;
        let mut rng = Random::new(self.state.rng_seed);
        let chance_roll = rng.randrange(10) as u8;
        let entry_roll = rng.randrange(100) as u8;
        let field_encounter =
            select_rock_smash_encounter(encounters, target.x, target.y, chance_roll, entry_roll)?;
        self.state.rng_seed = rng.seed();
        let wild_battle = self.start_field_encounter_battle(runtime, &field_encounter)?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum ROCK_SMASH field move")?;
        Ok(RuntimeFieldEncounterMoveUse {
            field_encounter,
            wild_battle,
            state_checksum,
        })
    }

    pub fn use_sweet_scent_field_move(
        &mut self,
        runtime: &CrystalRuntime,
        party_index: usize,
        surface: EncounterSurface,
    ) -> Result<RuntimeSweetScentFieldMoveUse> {
        self.require_no_active_battle("SWEET_SCENT field move")?;
        let actor_species = self
            .require_party_pokemon_knows_move(party_index, "SWEET_SCENT")?
            .species
            .id
            .clone();
        let map_name = self.overworld.map.name.clone();
        let encounters = runtime
            .data
            .wild_encounters
            .get(&map_name)
            .with_context(|| {
                format!("compiled game pack missing wild encounters for {map_name}")
            })?;
        let time = self.state.time.time_of_day;
        require_encounter_table_for_surface(encounters, surface, time)
            .with_context(|| format!("validate SWEET_SCENT encounters on {map_name}"))?;
        let mut rng = Random::new(self.state.rng_seed);
        let slot_percent_roll = rng.randrange(100) as u8 + 1;
        let level_roll = rng.randrange(256) as u8;
        let rng_seed_after_selection = rng.seed();
        self.state.rng_seed = rng_seed_after_selection;
        let mut wild_encounter = select_sweet_scent_encounter(
            encounters,
            &runtime.data.encounter_slot_tables,
            surface,
            time,
            self.overworld.player.tile,
            slot_percent_roll,
            level_roll,
        )
        .with_context(|| format!("select SWEET_SCENT encounter on {map_name}"))?;
        wild_encounter.rng_seed_after = rng_seed_after_selection;
        let wild_battle = self.start_wild_battle(runtime, wild_encounter.clone())?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum SWEET_SCENT field move")?;
        Ok(RuntimeSweetScentFieldMoveUse {
            actor_party_index: party_index,
            actor_species,
            wild_encounter,
            wild_battle,
            state_checksum,
        })
    }

    pub fn use_bag_item_on_party_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimePartyItemUse> {
        match &self.state.battle {
            BattleMemory::Inactive => {}
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use field item {item_id} during an active battle");
            }
        }
        if party_index >= self.state.storage.party.pokemon.len() {
            anyhow::bail!("item party index {party_index} is outside the party");
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let mut preview = self.state.storage.party.pokemon[party_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("item party index {party_index} has no Pokemon"))?;
        if item.rare_candy_level_gain.is_some()
            || runtime
                .data
                .evolutions
                .contains_item_evolution(&item.script_name)
        {
            apply_party_special_item_effect(
                &mut preview,
                item,
                &runtime.data.pokemon,
                &runtime.data.moves,
                &runtime.data.learnsets,
                &runtime.data.growth_rates,
                &runtime.data.battle_reward_rules,
                &runtime.data.evolutions,
                self.state.time.time_of_day,
                false,
            )
            .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        } else {
            apply_active_battle_item_effect(&mut preview, item, false)
                .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        }

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        let pokemon = self.state.storage.party.pokemon[party_index]
            .as_mut()
            .with_context(|| format!("item party index {party_index} has no Pokemon"))?;
        let item_effect = if item.rare_candy_level_gain.is_some()
            || runtime
                .data
                .evolutions
                .contains_item_evolution(&item.script_name)
        {
            apply_party_special_item_effect(
                pokemon,
                item,
                &runtime.data.pokemon,
                &runtime.data.moves,
                &runtime.data.learnsets,
                &runtime.data.growth_rates,
                &runtime.data.battle_reward_rules,
                &runtime.data.evolutions,
                self.state.time.time_of_day,
                item_use.consumed,
            )
        } else {
            apply_active_battle_item_effect(pokemon, item, item_use.consumed)
        }
        .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        self.state.sync_party_from_storage();
        let state_checksum = game_state_checksum(&self.state).context("checksum party item use")?;
        Ok(RuntimePartyItemUse {
            item_use,
            item_effect,
            state_checksum,
        })
    }

    pub fn use_bag_item_on_whole_party(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeWholePartyItemUse> {
        match &self.state.battle {
            BattleMemory::Inactive => {}
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use field item {item_id} during an active battle");
            }
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let mut preview = self.state.storage.party.clone();
        apply_party_wide_item_effect(&mut preview, item, false)
            .map_err(|error| anyhow::anyhow!("use whole-party item {item_id}: {error:?}"))?;

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use whole-party item {item_id}: {error:?}"))?;
        let item_effect =
            apply_party_wide_item_effect(&mut self.state.storage.party, item, item_use.consumed)
                .map_err(|error| anyhow::anyhow!("use whole-party item {item_id}: {error:?}"))?;
        self.state.sync_party_from_storage();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum whole-party item use")?;
        Ok(RuntimeWholePartyItemUse {
            item_use,
            item_effect,
            state_checksum,
        })
    }

    pub fn use_bag_item_on_party_move(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<RuntimePartyItemUse> {
        match &self.state.battle {
            BattleMemory::Inactive => {}
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use field item {item_id} during an active battle");
            }
        }
        if party_index >= self.state.storage.party.pokemon.len() {
            anyhow::bail!("item party index {party_index} is outside the party");
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let mut preview = self.state.storage.party.pokemon[party_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("item party index {party_index} has no Pokemon"))?;
        apply_battle_pp_item_effect(&mut preview, item, &runtime.data.moves, move_slot, false)
            .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        let pokemon = self.state.storage.party.pokemon[party_index]
            .as_mut()
            .with_context(|| format!("item party index {party_index} has no Pokemon"))?;
        let item_effect = apply_battle_pp_item_effect(
            pokemon,
            item,
            &runtime.data.moves,
            move_slot,
            item_use.consumed,
        )
        .map_err(|error| anyhow::anyhow!("use party item {item_id}: {error:?}"))?;
        self.state.sync_party_from_storage();
        let state_checksum = game_state_checksum(&self.state).context("checksum party item use")?;
        Ok(RuntimePartyItemUse {
            item_use,
            item_effect,
            state_checksum,
        })
    }

    pub fn use_bag_tmhm_on_party_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
        replace_slot: Option<usize>,
    ) -> Result<RuntimeTmHmItemUse> {
        match &self.state.battle {
            BattleMemory::Inactive => {}
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use field TM/HM {item_id} during an active battle");
            }
        }
        if party_index >= self.state.storage.party.pokemon.len() {
            anyhow::bail!("TM/HM party index {party_index} is outside the party");
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let mut preview = self.state.storage.party.pokemon[party_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("TM/HM party index {party_index} has no Pokemon"))?;
        teach_tmhm_move(&mut preview, item, &runtime.data.moves, replace_slot, false)
            .map_err(|error| anyhow::anyhow!("use TM/HM {item_id}: {error:?}"))?;

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use TM/HM {item_id}: {error:?}"))?;
        let pokemon = self.state.storage.party.pokemon[party_index]
            .as_mut()
            .with_context(|| format!("TM/HM party index {party_index} has no Pokemon"))?;
        let learned_move = teach_tmhm_move(
            pokemon,
            item,
            &runtime.data.moves,
            replace_slot,
            item_use.consumed,
        )
        .map_err(|error| anyhow::anyhow!("use TM/HM {item_id}: {error:?}"))?;
        self.state.sync_party_from_storage();
        let state_checksum = game_state_checksum(&self.state).context("checksum TM/HM item use")?;
        Ok(RuntimeTmHmItemUse {
            item_use,
            learned_move,
            state_checksum,
        })
    }

    pub fn use_bag_item_on_active_battle_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeBattleItemUse> {
        let active_index = require_active_party_index(&self.state)?;
        self.use_bag_item_on_battle_party_pokemon(runtime, item_id, active_index)
    }

    pub fn use_bag_item_on_battle_party_pokemon(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
    ) -> Result<RuntimeBattleItemUse> {
        match &self.state.battle {
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {}
            BattleMemory::Inactive => {
                anyhow::bail!("cannot use battle item {item_id} without an active battle");
            }
        }
        if party_index >= self.state.storage.party.pokemon.len() {
            anyhow::bail!("battle item party index {party_index} is outside the party");
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let mut preview = self.state.storage.party.pokemon[party_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("battle item party index {party_index} has no Pokemon"))?;
        apply_active_battle_item_effect(&mut preview, item, false)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Battle,
            },
        )
        .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        let pokemon = self.state.storage.party.pokemon[party_index]
            .as_mut()
            .with_context(|| format!("battle item party index {party_index} has no Pokemon"))?;
        let battle_item = apply_active_battle_item_effect(pokemon, item, item_use.consumed)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        self.state.sync_party_from_storage();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum battle item use")?;
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum,
        })
    }

    pub fn use_bag_item_on_battle_party_move(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        party_index: usize,
        move_slot: Option<usize>,
    ) -> Result<RuntimeBattleItemUse> {
        match &self.state.battle {
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {}
            BattleMemory::Inactive => {
                anyhow::bail!("cannot use battle item {item_id} without an active battle");
            }
        }
        if party_index >= self.state.storage.party.pokemon.len() {
            anyhow::bail!("battle item party index {party_index} is outside the party");
        }
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let mut preview = self.state.storage.party.pokemon[party_index]
            .as_ref()
            .cloned()
            .with_context(|| format!("battle item party index {party_index} has no Pokemon"))?;
        apply_battle_pp_item_effect(&mut preview, item, &runtime.data.moves, move_slot, false)
            .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;

        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Battle,
            },
        )
        .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        let pokemon = self.state.storage.party.pokemon[party_index]
            .as_mut()
            .with_context(|| format!("battle item party index {party_index} has no Pokemon"))?;
        let battle_item = apply_battle_pp_item_effect(
            pokemon,
            item,
            &runtime.data.moves,
            move_slot,
            item_use.consumed,
        )
        .map_err(|error| anyhow::anyhow!("use battle item {item_id}: {error:?}"))?;
        self.state.sync_party_from_storage();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum battle item use")?;
        Ok(RuntimeBattleItemUse {
            item_use,
            battle_item,
            state_checksum,
        })
    }

    pub fn update_clock_from_datetime(
        &mut self,
        date: GameDate,
        hour: u8,
        minute: u8,
        second: u8,
    ) -> Result<RuntimeTimeUpdate> {
        self.state
            .time
            .update_from_datetime(date, hour, minute, second);
        self.runtime_time_update("checksum clock update")
    }

    pub fn set_manual_clock_time(
        &mut self,
        now_date: GameDate,
        now_hour: u8,
        now_minute: u8,
        now_second: u8,
        target: ClockTime,
    ) -> Result<RuntimeTimeUpdate> {
        self.state
            .time
            .set_manual_time(now_date, now_hour, now_minute, now_second, target);
        self.runtime_time_update("checksum manual clock set")
    }

    pub fn cast_fishing_rod(
        &mut self,
        runtime: &CrystalRuntime,
        rod: &str,
    ) -> Result<RuntimeFishingCast> {
        validate_fishing_rod_id(rod)
            .with_context(|| format!("validate fishing rod {rod} before cast"))?;
        self.require_no_active_battle("fishing rod")?;
        let map_name = self.overworld.map.name.clone();
        let group = runtime
            .data
            .maps
            .get(&map_name)
            .with_context(|| format!("compiled game pack missing map module {map_name}"))?
            .attributes
            .fishing_group
            .clone();
        let mut rng = Random::new(self.state.rng_seed);
        let bite_roll = rng.randrange(256) as u8;
        let slot_roll = rng.randrange(256) as u8;
        self.state.rng_seed = rng.seed();
        let time_of_day = self.state.time.time_of_day;
        let mut session = do_fishing(
            &mut self.state,
            &runtime.data.fishing,
            group.as_deref(),
            rod,
            time_of_day,
            bite_roll,
            slot_roll,
        )
        .map_err(|error| anyhow::anyhow!("cast fishing rod {rod} on {map_name}: {error:?}"))?;
        let bite_frame = session
            .start_frame
            .saturating_add(session.cast_frames)
            .saturating_add(session.bite_delay_frames);
        let bite = fishing_bite(&mut self.state, &mut session, Some(bite_frame));
        let mut wild_battle = None;
        if bite == Some(true) {
            fishing_battle_trigger(&mut self.state);
            if let Some(encounter) = session.outcome.encounter.clone() {
                wild_battle = Some(self.start_fishing_battle(
                    runtime,
                    encounter,
                    time_of_day,
                    bite_roll,
                    slot_roll,
                )?);
            }
        }
        let state_checksum = game_state_checksum(&self.state).context("checksum fishing cast")?;
        Ok(RuntimeFishingCast {
            session,
            bite,
            wild_battle,
            state_checksum,
        })
    }

    pub fn use_bag_fishing_rod_in_field(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
    ) -> Result<RuntimeFishingRodItemUse> {
        self.require_no_active_battle("field fishing rod item")?;
        let item = runtime
            .data
            .items
            .get(item_id)
            .with_context(|| format!("compiled game pack missing item {item_id}"))?;
        let rod = fishing_rod_for_item_id(&runtime.data.fishing, item_id).with_context(|| {
            format!(
                "field fishing rod item {item_id} is not declared by exact fishing rod item rules"
            )
        })?;
        if !item.field_usable {
            anyhow::bail!("field fishing rod item {item_id} is not usable in the field");
        }
        if !self.state.bag.has_item(item) {
            anyhow::bail!("field fishing rod item {item_id} is not in the bag");
        }

        let cast = self.cast_fishing_rod(runtime, rod)?;
        let item_use = use_bag_item(
            &mut self.state,
            &runtime.data.items,
            ItemUseRequest {
                item_id: item_id.to_string(),
                context: ItemUseContext::Field,
            },
        )
        .map_err(|error| anyhow::anyhow!("use field fishing rod item {item_id}: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum field fishing rod item use")?;
        Ok(RuntimeFishingRodItemUse {
            item_use,
            rod: rod.to_string(),
            cast,
            state_checksum,
        })
    }

    pub fn grant_script_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemGrant> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let grant = module
            .script_item_grants
            .iter()
            .find(|grant| {
                grant.source_script == source_script && grant.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script item grant at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = grant_script_item(&mut self.state, &runtime.data.items, grant)
            .map_err(|error| anyhow::anyhow!("grant script item: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum item grant")?;
        Ok(RuntimeScriptItemGrant {
            outcome,
            state_checksum,
        })
    }

    pub fn check_script_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemCheck> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let access = module
            .script_item_checks
            .iter()
            .find(|access| {
                access.source_script == source_script && access.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script item check at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = check_script_item(&self.state, &runtime.data.items, access)
            .map_err(|error| anyhow::anyhow!("check script item: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum item check")?;
        Ok(RuntimeScriptItemCheck {
            outcome,
            state_checksum,
        })
    }

    pub fn take_script_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptItemTake> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let access = module
            .script_item_takes
            .iter()
            .find(|access| {
                access.source_script == source_script && access.command_index == command_index
            })
            .with_context(|| {
                format!("map {map_name} has no script item take at {source_script}:{command_index}")
            })?
            .clone();
        let outcome = take_script_item(&mut self.state, &runtime.data.items, access)
            .map_err(|error| anyhow::anyhow!("take script item: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum item take")?;
        Ok(RuntimeScriptItemTake {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_special_routine(
        &mut self,
        runtime: &CrystalRuntime,
        routine: &str,
    ) -> Result<RuntimeSpecialRoutineUse> {
        if !runtime.data.special_routines.contains(routine) {
            anyhow::bail!("compiled game pack missing exact special routine {routine}");
        }
        if routine == "FadeOutMusic" && !runtime.audio.contains_music("MUSIC_NONE") {
            anyhow::bail!("special routine FadeOutMusic requires compiled music asset MUSIC_NONE");
        }
        let cry_by_species = runtime.cry_by_species();
        let runtime_spawn_points = runtime
            .data
            .runtime_spawn_points
            .iter()
            .map(|(key, spawn)| {
                (
                    key.clone(),
                    RuntimeSpawnPointRef {
                        identifier: spawn.identifier,
                        map_constant: spawn.map_constant.clone(),
                        map_name: spawn.map_name.clone(),
                        group_id: spawn.group_id,
                        map_id: spawn.map_id,
                        tile_x: spawn.tile_x,
                        tile_y: spawn.tile_y,
                        group_name: spawn.group_name.clone(),
                        metatile_x: spawn.metatile_x,
                        metatile_y: spawn.metatile_y,
                        subtile_x: spawn.subtile_x,
                        subtile_y: spawn.subtile_y,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let outcome = apply_special_routine_with_context(
            &mut self.state,
            SpecialRoutineContext {
                move_catalog: &runtime.data.moves,
                cry_by_species: &cry_by_species,
                species_catalog: &runtime.data.pokemon,
                learnsets: &runtime.data.learnsets,
                growth_rates: &runtime.data.growth_rates,
                item_catalog: &runtime.data.items,
                runtime_spawn_points: &runtime_spawn_points,
                roaming_pokemon: &runtime.data.roaming_pokemon,
                buena_password_categories: &runtime.data.buena_password_categories,
                buena_prizes: &runtime.data.buena_prizes,
                kurt_apricorn_recipes: &runtime.data.kurt_apricorn_recipes,
                shuckie_gift: runtime.data.shuckie_gift.as_ref(),
                dratini_move_sets: &runtime.data.dratini_move_sets,
                bug_contest_config: runtime.data.bug_contest_config.as_ref(),
                battle_tower_rules: runtime.data.battle_tower_rules.as_ref(),
                magikarp_lengths: &runtime.data.magikarp_lengths,
                happiness_data: runtime.data.happiness_data.as_ref(),
                trainer_catalog: &runtime.data.trainers,
                odd_egg_definitions: &runtime.data.odd_egg_definitions,
                oak_ratings: &runtime.data.oak_ratings,
            },
            routine,
        )
        .map_err(|error| anyhow::anyhow!("apply special routine {routine}: {error}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum special routine")?;
        Ok(RuntimeSpecialRoutineUse {
            outcome,
            state_checksum,
        })
    }

    pub fn pickup_script_field_item(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFieldPickup> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let pickup = module
            .script_field_pickups
            .iter()
            .find(|pickup| {
                pickup.source_script == source_script && pickup.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script field pickup at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = pickup_script_field_item(
            &mut self.state,
            &runtime.data.items,
            &runtime.data.fruit_trees,
            pickup,
        )
        .map_err(|error| anyhow::anyhow!("pickup script field item: {error:?}"))?;
        self.overworld.sync_event_flag_memory(&self.state.flags);
        let state_checksum = game_state_checksum(&self.state).context("checksum field pickup")?;
        Ok(RuntimeFieldPickup {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_economy_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptEconomy> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_economy_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script economy command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_economy_command(
            &mut self.state,
            command,
            &runtime.data.currency_constants,
        )
        .map_err(|error| anyhow::anyhow!("apply script economy command: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum economy command")?;
        Ok(RuntimeScriptEconomy {
            outcome,
            state_checksum,
        })
    }

    pub fn initialize_permanent_phone_numbers(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<RuntimePermanentPhoneNumbers> {
        let inserted = initialize_permanent_phone_numbers(
            &mut self.state,
            &runtime.data.phone_contacts,
            &runtime.data.permanent_phone_numbers,
        )
        .map_err(|error| anyhow::anyhow!("initialize permanent phone numbers: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum permanent phone numbers")?;
        Ok(RuntimePermanentPhoneNumbers {
            inserted,
            state_checksum,
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
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_phone_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script phone command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_phone_command(
            &mut self.state,
            command,
            &runtime.data.phone_contacts,
            &runtime.data.permanent_phone_numbers,
            inputs,
        )
        .map_err(|error| anyhow::anyhow!("apply script phone command: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum phone command")?;
        Ok(RuntimePhoneCommand {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_flag_mutation(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFlagMutation> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_flag_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script flag mutation at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_flag_mutation(&mut self.state, command)
            .map_err(|error| anyhow::anyhow!("apply script flag mutation: {error:?}"))?;
        self.overworld.sync_event_flag_memory(&self.state.flags);
        let state_checksum = game_state_checksum(&self.state).context("checksum flag mutation")?;
        Ok(RuntimeFlagMutation {
            outcome,
            state_checksum,
        })
    }

    pub fn check_script_flag(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeFlagCheck> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_flag_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script flag check at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = check_script_flag(&self.state, command)
            .map_err(|error| anyhow::anyhow!("check script flag: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum flag check")?;
        Ok(RuntimeFlagCheck {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_scene_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeSceneCommand> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_scene_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script scene command at {source_script}:{command_index}"
                )
            })?
            .clone();
        if !module.scenes.scenes.is_empty() {
            self.state
                .scenes
                .enter_map(map_name, &module.scenes)
                .map_err(|error| {
                    anyhow::anyhow!("enter scene context for {map_name}: {error:?}")
                })?;
        }
        let (target_map_name, scene_table) = if let Some(target_map_id) = command.map_id.as_deref()
        {
            let target_map_name =
                runtime
                    .map_name_for_constant(target_map_id)
                    .with_context(|| {
                        format!("script scene command references missing map id {target_map_id}")
                    })?;
            let target_module = runtime.map_module(&target_map_name)?;
            (Some(target_map_name), &target_module.scenes)
        } else {
            (None, &module.scenes)
        };
        let outcome = apply_script_scene_command(
            &mut self.state,
            map_name,
            target_map_name.as_deref(),
            scene_table,
            command,
        )
        .map_err(|error| anyhow::anyhow!("apply script scene command: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum scene command")?;
        Ok(RuntimeSceneCommand {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_block_change(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeBlockChange> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let change = module
            .script_block_changes
            .iter()
            .find(|change| {
                change.source_script == source_script && change.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script block change at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_block_change(&mut self.overworld.map, change)
            .map_err(|error| anyhow::anyhow!("apply script block change: {error:?}"))?;
        self.state
            .map_block_overrides
            .entry(outcome.map_name.clone())
            .or_default()
            .insert((outcome.metatile_x, outcome.metatile_y), outcome.block_id);
        let state_checksum = game_state_checksum(&self.state).context("checksum block change")?;
        Ok(RuntimeBlockChange {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_audio_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptAudio> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_audio_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script audio command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let cry_by_species = runtime.cry_by_species();
        let cue = apply_script_audio_command(
            &mut self.state,
            command,
            &runtime.audio.music_ids(),
            &runtime.audio.sound_effect_ids(),
            &runtime.audio.cry_ids(),
            &runtime.data.pokemon,
            &cry_by_species,
        )
        .map_err(|error| anyhow::anyhow!("apply script audio command: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum audio command")?;
        Ok(RuntimeScriptAudio {
            cue,
            state_checksum,
        })
    }

    pub fn apply_script_map_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptMapCommand> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_map_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script map command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let action = apply_script_map_command(&mut self.state, command, &runtime.map_ids())
            .map_err(|error| anyhow::anyhow!("apply script map command: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum map command")?;
        Ok(RuntimeScriptMapCommand {
            action,
            state_checksum,
        })
    }

    pub fn execute_pending_script_warp(
        &mut self,
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
    ) -> Result<RuntimeScriptWarp> {
        let request = self
            .state
            .script_runtime
            .pending_script_warp
            .clone()
            .with_context(|| "cannot execute script warp without a pending script warp")?;
        let frame = self.overworld.frame;
        self.overworld =
            runtime.overworld_session_for(asset_root, &request.target_map, request.tile, frame)?;
        if let Some(facing) = request.facing {
            self.overworld.player.facing = facing;
        }
        apply_state_block_overrides(&mut self.overworld, &self.state)?;
        apply_state_object_overrides(&mut self.overworld, &self.state)?;
        runtime.sync_current_map_music(&mut self.state, &request.target_map)?;
        runtime.sync_current_map_scene(&mut self.state, &request.target_map)?;
        self.state.script_runtime.pending_script_warp = None;
        self.state.overworld = OverworldMemory::from_snapshot(&self.overworld.snapshot());
        self.state.frame_counter = self.overworld.frame;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum pending script warp")?;
        Ok(RuntimeScriptWarp {
            target_map: request.target_map,
            tile: request.tile,
            facing: request.facing,
            state_checksum,
        })
    }

    pub fn apply_script_text_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptText> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_text_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script text command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let text_labels = runtime.script_text_labels(module);
        let action = apply_script_text_command(&mut self.state, command, &text_labels)
            .map_err(|error| anyhow::anyhow!("apply script text command: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum text command")?;
        Ok(RuntimeScriptText {
            action,
            state_checksum,
        })
    }

    pub fn apply_script_variable_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptVariable> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_variable_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script variable command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let time_of_day = self.state.time.time_of_day;
        let outcome = apply_script_variable_command(&mut self.state, command, Some(time_of_day))
            .map_err(|error| anyhow::anyhow!("apply script variable command: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum variable command")?;
        Ok(RuntimeScriptVariable {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_control_command(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptControl> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_control_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script control command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let constants = runtime.script_numeric_constants();
        let action = apply_script_control_command(&mut self.state, command, &constants)
            .map_err(|error| anyhow::anyhow!("apply script control command: {error:?}"))?;
        let state_checksum =
            game_state_checksum(&self.state).context("checksum control command")?;
        Ok(RuntimeScriptControl {
            action,
            state_checksum,
        })
    }

    pub fn apply_script_object_mutation(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptObjectMutation> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script object command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_object_mutation(&mut self.state, &mut self.overworld, &command)
            .map_err(|error| anyhow::anyhow!("apply script object mutation: {error:?}"))?;
        sync_state_object_overrides(&mut self.state, &self.overworld);
        let state_checksum =
            game_state_checksum(&self.state).context("checksum object mutation")?;
        Ok(RuntimeScriptObjectMutation {
            outcome,
            state_checksum,
        })
    }

    pub fn apply_script_movement(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptMovement> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_object_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script movement command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let movement_label = command.movement.as_deref().with_context(|| {
            format!(
                "script movement command at {source_script}:{command_index} has no movement label"
            )
        })?;
        let movement = module
            .script_movements
            .iter()
            .find(|movement| {
                movement.label == movement_label
                    && movement.source_script.as_deref().is_none_or(|movement_source| {
                        movement_source == command.source_script.as_str()
                    })
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no exact movement {movement_label} for {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_movement(&mut self.overworld, &command, &movement)
            .map_err(|error| anyhow::anyhow!("apply script movement: {error:?}"))?;
        sync_state_object_overrides(&mut self.state, &self.overworld);
        let state_checksum = game_state_checksum(&self.state).context("checksum movement")?;
        Ok(RuntimeScriptMovement {
            outcome,
            state_checksum,
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
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_runtime_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script runtime command at {source_script}:{command_index}"
                )
            })?
            .clone();
        if command.command == "setlasttalked" {
            let object_id = command
                .args
                .first()
                .with_context(|| "setlasttalked command missing object id")?;
            self.require_runtime_object_reference(object_id)?;
        }
        let outcome = apply_script_runtime_command(&mut self.state, command.clone(), inputs)
            .map_err(|error| anyhow::anyhow!("apply script runtime command: {error:?}"))?;
        if command.command == "setlasttalked" {
            self.overworld.last_talked_object_identifier = command.args.first().cloned();
            sync_state_object_overrides(&mut self.state, &self.overworld);
        }
        let state_checksum =
            game_state_checksum(&self.state).context("checksum runtime command")?;
        Ok(RuntimeScriptRuntimeCommand {
            outcome,
            state_checksum,
        })
    }

    pub fn open_script_shop(
        &mut self,
        runtime: &CrystalRuntime,
        map_name: &str,
        source_script: &str,
        command_index: usize,
    ) -> Result<RuntimeScriptShop> {
        self.require_current_map(map_name)?;
        let module = runtime.map_module(map_name)?;
        let command = module
            .script_shop_commands
            .iter()
            .find(|command| {
                command.source_script == source_script && command.command_index == command_index
            })
            .with_context(|| {
                format!(
                    "map {map_name} has no script shop command at {source_script}:{command_index}"
                )
            })?
            .clone();
        let outcome = apply_script_shop_command(
            &mut self.state,
            &runtime.data.marts,
            &runtime.data.items,
            command,
        )
        .map_err(|error| anyhow::anyhow!("open script shop: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum script shop")?;
        Ok(RuntimeScriptShop {
            outcome,
            state_checksum,
        })
    }

    pub fn buy_shop_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeShopTransaction> {
        self.require_active_shop_item(item_id)?;
        let outcome = buy_item(&mut self.state, &runtime.data.items, item_id, quantity)
            .map_err(|error| anyhow::anyhow!("buy shop item {item_id}: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum shop purchase")?;
        Ok(RuntimeShopTransaction {
            outcome,
            state_checksum,
        })
    }

    pub fn sell_shop_item(
        &mut self,
        runtime: &CrystalRuntime,
        item_id: &str,
        quantity: u16,
    ) -> Result<RuntimeShopTransaction> {
        let outcome = sell_item(
            &mut self.state,
            &runtime.data.items,
            &runtime.data.currency_constants,
            item_id,
            quantity,
        )
        .map_err(|error| anyhow::anyhow!("sell shop item {item_id}: {error:?}"))?;
        let state_checksum = game_state_checksum(&self.state).context("checksum shop sale")?;
        Ok(RuntimeShopTransaction {
            outcome,
            state_checksum,
        })
    }

    fn require_current_map(&self, map_name: &str) -> Result<()> {
        if self.overworld.map.name != map_name {
            anyhow::bail!(
                "scripted battle map mismatch: session is on {}, request was for {map_name}",
                self.overworld.map.name
            );
        }
        Ok(())
    }

    fn require_no_active_battle(&self, context: &str) -> Result<()> {
        match &self.state.battle {
            BattleMemory::Inactive => Ok(()),
            BattleMemory::Wild { .. }
            | BattleMemory::StaticWild { .. }
            | BattleMemory::Trainer { .. } => {
                anyhow::bail!("cannot use {context} during an active battle")
            }
        }
    }

    fn require_party_pokemon_knows_move(
        &self,
        party_index: usize,
        move_id: &str,
    ) -> Result<&Pokemon> {
        let pokemon = self
            .state
            .storage
            .party
            .pokemon
            .get(party_index)
            .with_context(|| format!("field move party index {party_index} is outside the party"))?
            .as_ref()
            .with_context(|| format!("field move party index {party_index} has no Pokemon"))?;
        if !pokemon.moves.iter().any(|known| known.name == move_id) {
            anyhow::bail!("party Pokemon at index {party_index} does not know {move_id}");
        }
        Ok(pokemon)
    }

    fn saved_dig_warp_destination(
        &self,
        runtime: &CrystalRuntime,
        context: &str,
    ) -> Result<SavedDigWarpDestination> {
        let map_name = self
            .state
            .dig_warp_map_name
            .clone()
            .with_context(|| format!("{context} has no saved dig warp map"))?;
        let warp_index = self
            .state
            .dig_warp_index
            .with_context(|| format!("{context} has no saved dig warp index"))?;
        let module = runtime.map_module(&map_name)?;
        let warp = module
            .events
            .warps
            .iter()
            .find(|warp| warp.index == warp_index)
            .cloned()
            .with_context(|| {
                format!("{context} saved dig warp index {warp_index} missing on {map_name}")
            })?;
        let tile = crystal_core::world::session::warp_tile_position(&warp);
        Ok(SavedDigWarpDestination {
            map_name,
            warp_index,
            tile,
        })
    }

    fn require_active_shop_item(&self, item_id: &str) -> Result<()> {
        let shop = self
            .state
            .script_runtime
            .pending_shop
            .as_ref()
            .with_context(|| "cannot buy item without an active script shop")?;
        if !shop.inventory.iter().any(|id| id == item_id) {
            anyhow::bail!(
                "active script shop {} does not sell exact item id {item_id}",
                shop.mart_id
            );
        }
        Ok(())
    }

    fn require_runtime_object_reference(&self, object_id: &str) -> Result<()> {
        if object_id == "PLAYER" {
            return Ok(());
        }
        if self
            .overworld
            .objects
            .iter()
            .any(|object| object.object_identifier.as_deref() == Some(object_id))
        {
            return Ok(());
        }
        anyhow::bail!(
            "runtime command references missing exact object id {object_id} on {}",
            self.overworld.map.name
        );
    }

    fn check_coord_event_after_step(&self) -> Option<CoordEventTrigger> {
        let current_scene = self
            .state
            .scenes
            .map_scenes
            .get(&self.overworld.map.name)
            .map(String::as_str);
        self.overworld
            .check_coord_event(current_scene, StepOptions::default().stride_tiles)
    }

    fn check_wild_encounter_after_step(
        &mut self,
        runtime: &CrystalRuntime,
    ) -> Result<Option<WildEncounterRoll>> {
        let Some(encounters) = runtime.data.wild_encounters.get(&self.overworld.map.name) else {
            return Ok(None);
        };
        let mut rng = Random::new(self.state.rng_seed);
        let roll = self
            .overworld
            .check_wild_encounter(
                encounters,
                &runtime.data.encounter_slot_tables,
                &runtime.data.encounter_music_modifiers,
                &mut rng,
                EncounterCheckOptions {
                    time: self.state.time.time_of_day,
                    music_token: self.state.script_runtime.current_music.clone(),
                    has_cleanse_tag: false,
                },
            )
            .with_context(|| format!("check wild encounters on {}", self.overworld.map.name))?;
        self.state.rng_seed = rng.seed();
        Ok(roll.map(|roll| apply_repel_to_encounter_roll(&self.state, roll)))
    }

    fn start_wild_battle(
        &mut self,
        runtime: &CrystalRuntime,
        encounter: WildEncounterRoll,
    ) -> Result<WildBattleStart> {
        let mut rng = Random::new(self.state.rng_seed);
        let battle = runtime
            .data
            .wild_battle_start(encounter, &mut rng)
            .context("start wild battle from resolved encounter")?;
        self.state.rng_seed = rng.seed();
        self.state.battle = BattleMemory::from(&battle);
        self.state
            .pokedex
            .record_seen_pokemon(&battle.enemy_pokemon);
        self.state.battle_active_party_index = first_available_party_index(&self.state);
        self.state.battle_active_enemy_party_index = Some(0);
        self.state.battle_rewarded_enemy_party_indices.clear();
        self.state.battle_escape_attempts = 0;
        self.state.battle_player_stat_drop_guard_turns = 0;
        Ok(battle)
    }

    fn start_field_encounter_battle(
        &mut self,
        runtime: &CrystalRuntime,
        field_encounter: &FieldEncounterRoll,
    ) -> Result<Option<WildBattleStart>> {
        let Some(resolved) = field_encounter.resolved.clone() else {
            return Ok(None);
        };
        let surface = match field_encounter.kind {
            FieldEncounterKind::Headbutt => EncounterSurface::Grass,
            FieldEncounterKind::RockSmash => EncounterSurface::Rock,
        };
        let encounter = WildEncounterRoll {
            map_name: field_encounter.map_name.clone(),
            tile: TilePosition::new(field_encounter.target_tile_x, field_encounter.target_tile_y),
            surface,
            time: self.state.time.time_of_day,
            threshold: 255,
            encounter_roll: field_encounter.chance_roll,
            slot_percent_roll: field_encounter.entry_roll,
            level_roll: None,
            resolved: Some(resolved),
            repelled_by: None,
            rng_seed_after: self.state.rng_seed,
        };
        self.start_wild_battle(runtime, encounter).map(Some)
    }

    fn start_fishing_battle(
        &mut self,
        runtime: &CrystalRuntime,
        encounter: crystal_core::world::encounters::WildEncounter,
        time: TimeOfDay,
        bite_roll: u8,
        slot_roll: u8,
    ) -> Result<WildBattleStart> {
        let roll = WildEncounterRoll {
            map_name: self.overworld.map.name.clone(),
            tile: self.overworld.player.tile,
            surface: EncounterSurface::Water,
            time,
            threshold: 0,
            encounter_roll: bite_roll,
            slot_percent_roll: Some(slot_roll),
            level_roll: None,
            resolved: Some(ResolvedWildEncounter {
                level: encounter.level,
                encounter,
                slot: 0,
            }),
            repelled_by: None,
            rng_seed_after: self.state.rng_seed,
        };
        self.start_wild_battle(runtime, roll)
    }
}

fn direction_from_mask(mask: u8) -> Result<Option<Direction>, RuntimeInputError> {
    let directions = [
        (B_PAD_DOWN, Direction::Down),
        (B_PAD_UP, Direction::Up),
        (B_PAD_LEFT, Direction::Left),
        (B_PAD_RIGHT, Direction::Right),
    ];
    let mut pressed = directions
        .into_iter()
        .filter(|(bit, _)| mask & *bit != 0)
        .map(|(_, direction)| direction);
    let first = pressed.next();
    if pressed.next().is_some() {
        return Err(RuntimeInputError::ConflictingDirections { mask });
    }
    Ok(first)
}

fn decrement_active_repel(state: &mut GameState) {
    if state.repel_steps_remaining == 0 {
        state.active_repel_item = None;
        return;
    }
    state.repel_steps_remaining -= 1;
    if state.repel_steps_remaining == 0 {
        state.active_repel_item = None;
    }
}

fn apply_repel_to_encounter_roll(
    state: &GameState,
    mut roll: WildEncounterRoll,
) -> WildEncounterRoll {
    if state.repel_steps_remaining == 0 {
        return roll;
    }
    let Some(item_id) = state.active_repel_item.as_ref() else {
        return roll;
    };
    let Some(resolved) = roll.resolved.as_ref() else {
        return roll;
    };
    let Some(lead_level) = leading_party_level(state) else {
        return roll;
    };
    if lead_level > resolved.level {
        roll.resolved = None;
        roll.repelled_by = Some(item_id.clone());
    }
    roll
}

fn leading_party_level(state: &GameState) -> Option<u8> {
    state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .find(|pokemon| pokemon.hp > 0)
        .map(|pokemon| pokemon.level)
}

fn update_dig_warp_memory_for_transition(
    runtime: &CrystalRuntime,
    state: &mut GameState,
    transition: &WarpTransition,
) -> Result<()> {
    let source_metadata = runtime.runtime_map_metadata_for_name(&transition.trigger.map_name)?;
    let destination_metadata =
        runtime.runtime_map_metadata_for_name(&transition.destination.map_name)?;
    if is_dig_warp_source_environment(&source_metadata.environment)
        && is_dig_warp_destination_environment(&destination_metadata.environment)
        && !is_dig_previous_map_blacklisted(&transition.trigger.map_name)
    {
        state.dig_warp_map_name = Some(transition.trigger.map_name.clone());
        state.dig_warp_index = Some(transition.trigger.warp.index);
    } else {
        state.dig_warp_map_name = None;
        state.dig_warp_index = None;
    }
    Ok(())
}

fn is_dig_warp_source_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN")
}

fn is_dig_warp_destination_environment(environment: &str) -> bool {
    matches!(environment, "INDOOR" | "CAVE" | "DUNGEON" | "GATE")
}

fn is_escape_rope_environment(environment: &str) -> bool {
    matches!(environment, "CAVE" | "DUNGEON")
}

fn is_bicycle_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN" | "CAVE" | "GATE")
}

fn find_itemfinder_hidden_item(
    map_module: &crystal_assets::modpack::MapModule,
    state: &GameState,
    player_tile: TilePosition,
    map_name: &str,
) -> Result<Option<RuntimeItemfinderHiddenItem>> {
    for event in &map_module.events.bg_events {
        if event.event_type != "BGEVENT_ITEM" {
            continue;
        }
        if !event_in_itemfinder_range(event.x, event.y, player_tile) {
            continue;
        }
        let pickup = map_module
            .script_field_pickups
            .iter()
            .find(|pickup| pickup.command == "hiddenitem" && pickup.source_script == event.script)
            .with_context(|| {
                format!(
                    "itemfinder bg event {} at {},{} has no exact hiddenitem pickup",
                    event.script, event.x, event.y
                )
            })?;
        let event_flag = pickup.event_flag.as_ref().with_context(|| {
            format!(
                "itemfinder hiddenitem {} on {map_name} is missing event_flag",
                pickup.source_script
            )
        })?;
        if state
            .flags
            .is_event_flag_set(event_flag)
            .with_context(|| format!("check itemfinder event flag {event_flag}"))?
        {
            continue;
        }
        let item_id = pickup.item_id.as_ref().with_context(|| {
            format!(
                "itemfinder hiddenitem {} on {map_name} is missing item_id",
                pickup.source_script
            )
        })?;
        return Ok(Some(RuntimeItemfinderHiddenItem {
            map_name: map_name.to_string(),
            tile: TilePosition::new(event.x as i16, event.y as i16),
            source_script: pickup.source_script.clone(),
            event_flag: event_flag.clone(),
            item_id: item_id.clone(),
        }));
    }
    Ok(None)
}

fn event_in_itemfinder_range(event_x: u16, event_y: u16, player_tile: TilePosition) -> bool {
    const SCREEN_WIDTH_TILES: i16 = 20;
    const SCREEN_HEIGHT_TILES: i16 = 18;
    let x_margin = SCREEN_WIDTH_TILES / 4;
    let y_margin = SCREEN_HEIGHT_TILES / 4;
    let half_width = SCREEN_WIDTH_TILES / 2;
    let half_height = SCREEN_HEIGHT_TILES / 2;
    let dx = player_tile.x + x_margin - event_x as i16;
    if dx < 0 || dx >= half_width {
        return false;
    }
    let dy = player_tile.y + y_margin - event_y as i16;
    if dy < 0 || dy >= half_height {
        return false;
    }
    true
}

fn blue_card_balance(state: &GameState) -> Result<u8> {
    let Some(value) = state.script_runtime.variables.get("VAR_BLUECARDBALANCE") else {
        return Ok(0);
    };
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        anyhow::bail!("blue card balance VAR_BLUECARDBALANCE has invalid exact integer {value}");
    }
    let parsed: u8 = value.parse().with_context(|| {
        format!("blue card balance VAR_BLUECARDBALANCE has invalid exact integer {value}")
    })?;
    if parsed > 30 {
        anyhow::bail!("blue card balance VAR_BLUECARDBALANCE is outside 0..=30: {parsed}");
    }
    Ok(parsed)
}

fn town_map_landmark_for_map(data: &GameDataSet, map_name: &str) -> Result<RuntimeTownMapLandmark> {
    let landmark_constant = data
        .pokegear_landmarks
        .map_to_landmark
        .get(map_name)
        .with_context(|| format!("town map missing exact landmark mapping for map {map_name}"))?;
    let landmark = data
        .pokegear_landmarks
        .landmarks
        .iter()
        .find(|landmark| landmark.constant == *landmark_constant)
        .with_context(|| {
            format!(
                "town map landmark mapping for map {map_name} points to missing landmark {landmark_constant}"
            )
        })?;
    Ok(RuntimeTownMapLandmark {
        id: landmark.id,
        constant: landmark.constant.clone(),
        label: landmark.label.clone(),
        name: landmark.name.clone(),
        x: landmark.x,
        y: landmark.y,
        region: landmark.region.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SquirtBottleTarget {
    target_tile: TilePosition,
    target_object_identifier: Option<String>,
    target_movement: String,
    target_script: Option<String>,
}

fn squirtbottle_target(
    overworld: &OverworldSession,
    map_module: &crystal_assets::modpack::MapModule,
) -> Result<SquirtBottleTarget> {
    let target_tile = move_by_stride(
        overworld.player.tile,
        overworld.player.facing,
        StepOptions::default().stride_tiles,
    );
    let Some((_, object)) = overworld.visible_object_at(target_tile) else {
        return Ok(SquirtBottleTarget {
            target_tile,
            target_object_identifier: None,
            target_movement: String::new(),
            target_script: None,
        });
    };
    if object.spritemovedata != "SPRITEMOVEDATA_SUDOWOODO" {
        return Ok(SquirtBottleTarget {
            target_tile,
            target_object_identifier: object.object_identifier.clone(),
            target_movement: object.spritemovedata.clone(),
            target_script: None,
        });
    }
    if !map_module.scripts.contains_key(&object.script) {
        anyhow::bail!(
            "field squirtbottle target {:?} references missing exact script {}",
            object.object_identifier,
            object.script
        );
    }
    Ok(SquirtBottleTarget {
        target_tile,
        target_object_identifier: object.object_identifier.clone(),
        target_movement: object.spritemovedata.clone(),
        target_script: Some(object.script.clone()),
    })
}

fn is_dig_field_move_environment(environment: &str) -> bool {
    matches!(environment, "CAVE" | "DUNGEON")
}

fn is_fly_source_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN")
}

fn is_teleport_source_environment(environment: &str) -> bool {
    matches!(environment, "ROUTE" | "TOWN")
}

fn validate_fishing_rod_id(rod: &str) -> Result<&str> {
    match rod {
        ROD_OLD | ROD_GOOD | ROD_SUPER => Ok(rod),
        _ => anyhow::bail!("unknown fishing rod '{rod}'"),
    }
}

fn is_dig_previous_map_blacklisted(map_name: &str) -> bool {
    matches!(map_name, "MountMoonSquare" | "TinTowerRoof")
}

fn update_active_battle_enemy(
    state: &mut GameState,
    enemy_pokemon: crystal_core::models::Pokemon,
) -> Result<()> {
    let active_enemy_index = require_active_enemy_party_index(state)?;
    match &mut state.battle {
        BattleMemory::Wild {
            enemy_pokemon: active,
            enemy_party,
            ..
        }
        | BattleMemory::StaticWild {
            enemy_pokemon: active,
            enemy_party,
            ..
        }
        | BattleMemory::Trainer {
            enemy_pokemon: active,
            enemy_party,
            ..
        } => {
            *active = enemy_pokemon.clone();
            let party_entry = enemy_party.get_mut(active_enemy_index).with_context(|| {
                format!(
                    "active enemy party index {active_enemy_index} is outside battle enemy party"
                )
            })?;
            *party_entry = enemy_pokemon;
            Ok(())
        }
        BattleMemory::Inactive => anyhow::bail!("cannot update inactive battle enemy"),
    }
}

fn first_available_party_index(state: &GameState) -> Option<usize> {
    state
        .storage
        .party
        .pokemon
        .iter()
        .enumerate()
        .find_map(|(index, pokemon)| {
            let pokemon = pokemon.as_ref()?;
            (pokemon.hp > 0).then_some(index)
        })
}

fn require_active_party_index(state: &GameState) -> Result<usize> {
    let index = state
        .battle_active_party_index
        .with_context(|| "active battle party index is not set")?;
    validate_active_party_index(state, index)?;
    Ok(index)
}

fn require_active_enemy_party_index(state: &GameState) -> Result<usize> {
    state
        .battle_active_enemy_party_index
        .with_context(|| "active enemy party index is not set")
}

fn validate_active_party_index(state: &GameState, index: usize) -> Result<()> {
    if index >= state.storage.party.pokemon.len() {
        anyhow::bail!("active battle party index {index} is outside the party");
    }
    let pokemon = state.storage.party.pokemon[index]
        .as_ref()
        .with_context(|| format!("active battle party index {index} has no Pokemon"))?;
    if pokemon.hp == 0 {
        anyhow::bail!("active battle party index {index} is fainted");
    }
    Ok(())
}

fn apply_state_block_overrides(overworld: &mut OverworldSession, state: &GameState) -> Result<()> {
    let Some(overrides) = state.map_block_overrides.get(&overworld.map.name) else {
        return Ok(());
    };
    for ((metatile_x, metatile_y), block_id) in overrides {
        let x = i16::try_from(*metatile_x).with_context(|| {
            format!(
                "saved block override x coordinate {} is out of range for map {}",
                metatile_x, overworld.map.name
            )
        })?;
        let y = i16::try_from(*metatile_y).with_context(|| {
            format!(
                "saved block override y coordinate {} is out of range for map {}",
                metatile_y, overworld.map.name
            )
        })?;
        let index = overworld.map.metatile_index(x, y).with_context(|| {
            format!(
                "saved block override ({metatile_x}, {metatile_y}) is outside map {}",
                overworld.map.name
            )
        })?;
        overworld.map.metatile_ids[index] = *block_id;
    }
    Ok(())
}

fn apply_state_object_overrides(overworld: &mut OverworldSession, state: &GameState) -> Result<()> {
    let Some(memory) = state.map_object_overrides.get(&overworld.map.name) else {
        return Ok(());
    };
    for (object_id, object_memory) in &memory.objects {
        let object = overworld
            .objects
            .iter_mut()
            .find(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .with_context(|| {
                format!(
                    "saved object override references missing object {object_id} on map {}",
                    overworld.map.name
                )
            })?;
        object.x = object_memory.x;
        object.y = object_memory.y;
        if let Some(facing) = object_memory.facing {
            overworld.object_facings.insert(object_id.clone(), facing);
        }
    }
    overworld.hidden_object_identifiers = memory.hidden_object_identifiers.clone();
    overworld.player_hidden = memory.player_hidden;
    overworld.last_talked_object_identifier = memory.last_talked_object_identifier.clone();
    overworld.following = memory
        .following
        .as_ref()
        .map(|following| OverworldFollowState {
            leader_object_id: following.leader_object_id.clone(),
            follower_object_id: following.follower_object_id.clone(),
        });
    Ok(())
}

fn sync_state_object_overrides(state: &mut GameState, overworld: &OverworldSession) {
    let objects = overworld
        .objects
        .iter()
        .filter_map(|object| {
            let object_id = object.object_identifier.as_ref()?;
            Some((
                object_id.clone(),
                OverworldObjectMemory {
                    x: object.x,
                    y: object.y,
                    facing: overworld.object_facings.get(object_id).copied(),
                },
            ))
        })
        .collect();
    state.map_object_overrides.insert(
        overworld.map.name.clone(),
        OverworldObjectMapMemory {
            objects,
            hidden_object_identifiers: overworld.hidden_object_identifiers.clone(),
            following: overworld
                .following
                .as_ref()
                .map(|following| OverworldFollowMemory {
                    leader_object_id: following.leader_object_id.clone(),
                    follower_object_id: following.follower_object_id.clone(),
                }),
            last_talked_object_identifier: overworld.last_talked_object_identifier.clone(),
            player_hidden: overworld.player_hidden,
        },
    );
    state.overworld = OverworldMemory::from_snapshot(&overworld.snapshot());
}

impl RuntimeAudioCatalog {
    pub fn from_game_data(asset_root: &AssetRoot, data: &GameDataSet) -> Result<Self> {
        let mut catalog = Self {
            music: BTreeMap::new(),
            sound_effects: BTreeMap::new(),
            cries: BTreeMap::new(),
        };

        for asset in &data.audio {
            asset.validate()?;
            let path = asset_root
                .resolve_data_path(&asset.path)
                .with_context(|| format!("resolve runtime audio asset {}", asset.path))?;
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read runtime audio asset {}", path.display()))?;
            let source = match asset.source {
                ModpackAudioSource::Midi => {
                    if !bytes.starts_with(b"MThd") {
                        anyhow::bail!(
                            "runtime audio asset {} is not a MIDI file",
                            path.display()
                        );
                    }
                    AudioProgramSource::Midi(bytes)
                }
                ModpackAudioSource::Pcm => {
                    if bytes.is_empty() {
                        anyhow::bail!("runtime PCM audio asset {} is empty", path.display());
                    }
                    let sample_rate_hz = asset.sample_rate_hz.ok_or_else(|| {
                        anyhow::anyhow!("PCM audio asset '{}' must declare sample_rate_hz", asset.id)
                    })?;
                    let channels = asset.channels.ok_or_else(|| {
                        anyhow::anyhow!("PCM audio asset '{}' must declare channels", asset.id)
                    })?;
                    AudioProgramSource::Pcm {
                        sample_rate_hz,
                        channels,
                        bytes,
                    }
                }
            };
            let program = AudioProgram {
                cache_key: format!(
                    "{}:{}:{}",
                    audio_kind_name(asset.kind),
                    asset.id,
                    path.display()
                ),
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
                    audio_kind_name(asset.kind),
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

fn reject_pack_with_error_diagnostics(pack: &CompiledGamePack) -> Result<()> {
    if !pack.report().has_errors() {
        return Ok(());
    }
    let summary = pack
        .report()
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity == crystal_assets::modpack::VerificationSeverity::Error
        })
        .take(8)
        .map(|diagnostic| {
            format!(
                "{:?} {} [{}]: {}",
                diagnostic.severity, diagnostic.subject, diagnostic.code, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    anyhow::bail!("compiled game pack has verification errors: {summary}")
}

fn reject_pack_without_runtime_game_data(pack: &CompiledGamePack) -> Result<()> {
    let data = pack.data();
    if data.pokemon.is_empty() {
        anyhow::bail!("compiled game pack has no Pokemon species data");
    }
    if data.moves.is_empty() {
        anyhow::bail!("compiled game pack has no move data");
    }
    if data.maps.is_empty() {
        anyhow::bail!("compiled game pack has no map modules");
    }
    for (map_name, module) in &data.maps {
        if module.attributes.width == 0 || module.attributes.height == 0 {
            anyhow::bail!("compiled game pack map '{map_name}' has empty dimensions");
        }
        let expected_blocks = module.attributes.width as usize * module.attributes.height as usize;
        if module.blocks.len() != expected_blocks {
            anyhow::bail!(
                "compiled game pack map '{map_name}' has {} blocks but dimensions require {expected_blocks}",
                module.blocks.len()
            );
        }
    }
    Ok(())
}

fn runtime_modpack_id(pack: &CompiledGamePack) -> Result<String> {
    if pack.report().manifests.is_empty() {
        anyhow::bail!("compiled game pack report must include at least one manifest id");
    }
    Ok(pack.report().manifests.join("+"))
}

fn audio_kind_name(kind: ModpackAudioKind) -> &'static str {
    match kind {
        ModpackAudioKind::Music => "music",
        ModpackAudioKind::SoundEffect => "sound_effect",
        ModpackAudioKind::Cry => "cry",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crystal_assets::modpack::{
        MapModule, ModpackAudioAsset, ModpackCompileReport, VerificationError, VerificationSeverity,
    };
    use crystal_assets::{
        PokegearLandmark, PokemonCryMetadata, ScriptedTrainerBattle, ScriptedWildBattle,
    };
    use crystal_core::battle::capture::CaptureWobbleProbability;
    use crystal_core::battle::start::{
        StaticWildBattleRequest, TrainerBattleRequest, TrainerBattleStartStatus,
    };
    use crystal_core::battle::stats::{BattleStatMultiplier, BattleStatMultiplierTables};
    use crystal_core::map::{
        BackgroundEvent, CoordEvent, MapAttributes, MapEvents, MapScene, MapSceneTable,
        ObjectEvent, WarpEvent,
    };
    use crystal_core::models::{
        growth_rate, item_pocket, pokemon_type, BaseStats, CaptureStorageLocation, Dv, Item,
        ItemPocket, LearnedMove, Move, Pokemon, PokemonSpecies, Trainer, TrainerPartyPokemon,
    };
    use crystal_core::state::{FishingRodState, ScriptGraphicsRuntimeKind};
    use crystal_core::systems::evolution::EvolutionEntry;
    use crystal_core::systems::experience::calculate_experience;
    use crystal_core::systems::field_items::ScriptFieldPickup;
    use crystal_core::systems::field_moves::{
        FieldEscapeItemRule, FieldItemRule, FieldMoveBadgeRequirement, FieldMoveBlockRule,
        FieldMoveCatalog, FieldMoveFlagRule, FieldMoveMoveRule, FieldMoveReplacement,
        FieldMoveRule, FieldMoveTravelRule, FieldRepelItemRule,
    };
    use crystal_core::systems::gift_pokemon::GiftPokemonScript;
    use crystal_core::systems::learnsets::LearnsetEntry;
    use crystal_core::systems::script_audio::{ScriptAudioCommand, ScriptAudioCue};
    use crystal_core::systems::script_control::{ScriptControlAction, ScriptControlCommand};
    use crystal_core::systems::script_objects::{
        ScriptMovement, ScriptMovementStep, ScriptObjectCommand,
    };
    use crystal_core::systems::script_runtime::{
        ScriptRuntimeCommand, ScriptRuntimeInputs, ScriptRuntimeOutcome,
    };
    use crystal_core::systems::script_text::{ScriptTextAction, ScriptTextBody, ScriptTextCommand};
    use crystal_core::systems::script_variables::{ScriptVariableCommand, ScriptVariableOutcome};
    use crystal_core::systems::script_warps::{ScriptMapAction, ScriptMapCommand};
    use crystal_core::systems::special_routines::SpecialRoutineEffect;
    use crystal_core::systems::step_events::{
        PoisonDamageResult, StepEventCounters, StepEventRules,
    };
    use crystal_core::world::encounters::{
        EncounterSlotChance, EncounterSlotTables, FieldEncounterData, FieldEncounterEntry,
        FieldEncounterTable,
    };
    use crystal_core::world::encounters::{WildEncounter, WildEncounterData, WildEncounterTable};
    use crystal_core::world::fishing::{
        FishingCatalog, FishingGroup, FishingRodItemRule, FishingSlot, RodTable, ROD_GOOD,
    };
    use crystal_core::world::movement::MovementMode;

    fn temp_repository_root(name: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "crystal-bevy-runtime-{}-{unique}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("apps/web/assets/data"))
            .expect("create runtime data root");
        root
    }

    fn write_midi(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create midi parent");
        }
        std::fs::write(path, b"MThd\x00\x00\x00\x06\x00\x00\x00\x01\x00\x60")
            .expect("write midi fixture");
    }

    fn write_pcm(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create pcm parent");
        }
        std::fs::write(path, [0_u8, 0, 0xff, 0x7f]).expect("write pcm fixture");
    }

    fn write_floor_tileset(root: &std::path::Path, tileset_name: &str) {
        write_tileset(
            root,
            tileset_name,
            r#"{
  "0": [0, 0, 0, 0]
}"#,
        );
    }

    fn write_grass_tileset(root: &std::path::Path, tileset_name: &str) {
        write_tileset(
            root,
            tileset_name,
            r#"{
  "0": [0, 0, 0, 0],
  "1": [24, 24, 24, 24]
}"#,
        );
    }

    fn write_tileset(root: &std::path::Path, tileset_name: &str, payload: &str) {
        let path = root
            .join("apps/web/assets/data/tilesets")
            .join(format!("{tileset_name}.json"));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create tileset parent");
        }
        std::fs::write(path, payload).expect("write tileset fixture");
    }

    fn report() -> ModpackCompileReport {
        ModpackCompileReport {
            manifests: vec!["core-modular".to_string()],
            ..ModpackCompileReport::default()
        }
    }

    fn identity() -> SaveModpackIdentity {
        SaveModpackIdentity::new("core-modular", "1234abcd").expect("identity")
    }

    fn runtime_species() -> PokemonSpecies {
        PokemonSpecies::new_for_tests("CHIKORITA", BaseStats::new(45, 49, 65, 45, 49, 65))
    }

    fn runtime_move() -> Move {
        runtime_move_named("TACKLE", 35)
    }

    fn runtime_move_named(name: &str, pp: u8) -> Move {
        Move {
            name: name.to_string(),
            move_type: pokemon_type("NORMAL"),
            power: 40,
            accuracy: 100,
            pp,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    fn growth_rate_catalog_for_tests() -> crystal_core::systems::experience::GrowthRateCatalog {
        [
            ("GROWTH_MEDIUM_FAST", 1, 1, 0, 0, 0),
            ("GROWTH_SLIGHTLY_FAST", 3, 4, 10, 0, 30),
            ("GROWTH_SLIGHTLY_SLOW", 3, 4, 20, 0, 70),
            ("GROWTH_MEDIUM_SLOW", 6, 5, -15, 100, 140),
            ("GROWTH_FAST", 4, 5, 0, 0, 0),
            ("GROWTH_SLOW", 5, 4, 0, 0, 0),
        ]
        .into_iter()
        .map(
            |(id, numerator, denominator, quadratic, linear, constant)| {
                (
                    id.to_string(),
                    crystal_core::systems::experience::GrowthRateCurve {
                        id: id.to_string(),
                        numerator,
                        denominator,
                        quadratic,
                        linear,
                        constant,
                    },
                )
            },
        )
        .collect()
    }

    fn runtime_item(id: &str, pocket: ItemPocket) -> Item {
        Item {
            name: id.to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
            status_heals: Vec::new(),
            revive_hp_percent: None,
            party_revive_hp_percent: None,
            pp_restore_scope: None,
            pp_restore_points: None,
            pp_up_stages: None,
            vitamin_stat: None,
            vitamin_stat_exp: None,
            vitamin_max_stat_exp: None,
            rare_candy_level_gain: None,
            battle_stat_boost_stat: None,
            battle_stat_boost_stages: None,
            battle_escape_mode: None,
            battle_focus_energy: None,
            battle_stat_drop_guard: None,
            battle_stat_drop_guard_turns: None,
            confusion_heal: None,
            repel_steps: None,
            escape_rope_mode: None,
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket,
            field_menu: String::new(),
            field_usable: true,
            battle_menu: String::new(),
            battle_usable: true,
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
            tmhm_move: None,
        }
    }

    fn runtime_ball_item(id: &str) -> Item {
        let mut item = runtime_item(id, item_pocket("BALL"));
        item.effect = "POKE_BALL".to_string();
        item.consumable = true;
        item.battle_menu = "ITEMMENU_CLOSE".to_string();
        item.battle_usable = true;
        item
    }

    fn runtime_map() -> MapModule {
        MapModule {
            id: "RuntimeMap".to_string(),
            attributes: MapAttributes {
                tileset_name: "johto".to_string(),
                border_block: 0,
                width: 2,
                height: 1,
                connections: Vec::new(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: Some("route".to_string()),
                location: Some("johto".to_string()),
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: Some("RUNTIME_MAP".to_string()),
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
            scripts: BTreeMap::new(),
            trainer_scripts: BTreeMap::new(),
            scripted_trainer_battles: Vec::new(),
            scripted_wild_battles: Vec::new(),
            script_item_grants: Vec::new(),
            script_item_checks: Vec::new(),
            script_item_takes: Vec::new(),
            script_economy_commands: Vec::new(),
            gift_pokemon_scripts: Vec::new(),
            script_flag_commands: Vec::new(),
            script_scene_commands: Vec::new(),
            script_audio_commands: Vec::new(),
            script_block_changes: Vec::new(),
            script_object_commands: Vec::new(),
            script_movements: Vec::new(),
            script_map_commands: Vec::new(),
            script_text_commands: Vec::new(),
            script_text_bodies: BTreeMap::new(),
            script_menu_definitions: BTreeMap::new(),
            script_variable_commands: Vec::new(),
            script_control_commands: Vec::new(),
            script_field_pickups: Vec::new(),
            script_shop_commands: Vec::new(),
            script_phone_commands: Vec::new(),
            script_runtime_commands: Vec::new(),
            map_script_section_commands: Vec::new(),
            map_event_section_commands: Vec::new(),
            scenes: MapSceneTable::default(),
            events: MapEvents::default(),
            objects: Vec::new(),
            blocks: vec![0, 0],
        }
    }

    fn runtime_map_metadata(
        constant: &str,
        name: &str,
        group_id: u16,
        map_id: u16,
        environment: &str,
    ) -> RuntimeMapMetadata {
        RuntimeMapMetadata {
            constant: constant.to_string(),
            name: name.to_string(),
            group_name: "RUNTIME".to_string(),
            group_id,
            map_id,
            width: 2,
            height: 1,
            environment: environment.to_string(),
            phone_service: 0,
        }
    }

    fn runtime_object(object_identifier: &str, event_flag: &str) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x: 1,
            y: 1,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "RuntimeWildScript".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(object_identifier.to_string()),
            sightline_direction_override: None,
        }
    }

    fn minimal_runtime_data() -> GameDataSet {
        GameDataSet {
            pokemon: [("CHIKORITA".to_string(), runtime_species())]
                .into_iter()
                .collect(),
            moves: [("TACKLE".to_string(), runtime_move())]
                .into_iter()
                .collect(),
            growth_rates: growth_rate_catalog_for_tests(),
            evolutions: crystal_core::systems::evolution::EvolutionTable(
                [("CHIKORITA".to_string(), Vec::new())]
                    .into_iter()
                    .collect(),
            ),
            maps: [("RuntimeMap".to_string(), runtime_map())]
                .into_iter()
                .collect(),
            map_attributes: [("RuntimeMap".to_string(), runtime_map().attributes.clone())]
                .into_iter()
                .collect(),
            runtime_spawn_points: [(
                "0".to_string(),
                RuntimeSpawnPoint {
                    identifier: 0,
                    map_constant: "RUNTIME_MAP".to_string(),
                    map_name: "RuntimeMap".to_string(),
                    group_id: 1,
                    map_id: 1,
                    tile_x: 0,
                    tile_y: 0,
                    group_name: "RUNTIME".to_string(),
                    metatile_x: 0,
                    metatile_y: 0,
                    subtile_x: 0,
                    subtile_y: 0,
                },
            )]
            .into_iter()
            .collect(),
            runtime_map_metadata: [(
                "RUNTIME_MAP".to_string(),
                runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "ROUTE"),
            )]
            .into_iter()
            .collect(),
            currency_constants: crystal_core::systems::economy::CurrencyCatalog(
                [
                    ("MAX_MONEY".to_string(), 999_999),
                    ("MAX_COINS".to_string(), 9_999),
                ]
                .into_iter()
                .collect(),
            ),
            encounter_slot_tables: EncounterSlotTables {
                grass: vec![EncounterSlotChance {
                    threshold: 100,
                    slot: 0,
                }],
                water: vec![EncounterSlotChance {
                    threshold: 100,
                    slot: 0,
                }],
            },
            battle_stat_multipliers: BattleStatMultiplierTables {
                stat: vec![
                    BattleStatMultiplier {
                        numerator: 1,
                        denominator: 1,
                    };
                    13
                ],
                accuracy: vec![
                    BattleStatMultiplier {
                        numerator: 1,
                        denominator: 1,
                    };
                    13
                ],
            },
            capture_wobble_probabilities: vec![CaptureWobbleProbability {
                catch_rate: 255,
                chance: 255,
            }],
            capture_rules: minimal_capture_rules(),
            battle_escape_rules: minimal_battle_escape_rules(),
            oak_ratings: vec![crystal_core::systems::special_routines::OakRatingEntry {
                caught_count_limit: 1,
                fanfare: "SFX_DEX_FANFARE_LESS_THAN_20".to_string(),
                text_label: "OakRating01".to_string(),
            }],
            move_priorities: crystal_core::battle::turn::MovePriorityTable {
                base_priority: 1,
                effect_priorities: vec![
                    crystal_core::battle::turn::MoveEffectPriority {
                        move_effect: "PRIORITY_HIT".to_string(),
                        priority: 2,
                    },
                    crystal_core::battle::turn::MoveEffectPriority {
                        move_effect: "NORMAL_HIT".to_string(),
                        priority: 1,
                    },
                ],
                move_priorities: vec![crystal_core::battle::turn::MovePriorityOverride {
                    r#move: "VITAL_THROW".to_string(),
                    priority: 0,
                }],
            },
            type_categories: crystal_core::battle::damage::TypeCategories {
                physical: vec!["NORMAL".to_string(), "FIGHTING".to_string()],
                special: vec!["FIRE".to_string(), "WATER".to_string()],
            },
            type_effectiveness: crystal_core::battle::damage::TypeEffectivenessTable {
                matchups: ["NORMAL", "FIGHTING", "FIRE", "WATER"]
                    .into_iter()
                    .flat_map(|attacker| {
                        ["NORMAL", "FIGHTING", "FIRE", "WATER"]
                            .into_iter()
                            .map(move |defender| {
                                crystal_core::battle::damage::TypeEffectivenessEntry {
                                    attacker: pokemon_type(attacker),
                                    defender: pokemon_type(defender),
                                    multiplier: crystal_core::battle::damage::TypeMultiplier::one(),
                                }
                            })
                    })
                    .collect(),
                foresight_matchups: vec![crystal_core::battle::damage::TypeEffectivenessEntry {
                    attacker: pokemon_type("NORMAL"),
                    defender: pokemon_type("FIGHTING"),
                    multiplier: crystal_core::battle::damage::TypeMultiplier::zero(),
                }],
            },
            weather_modifiers: crystal_core::battle::damage::WeatherModifiers {
                type_modifiers: vec![crystal_core::battle::damage::WeatherTypeModifier {
                    weather: "WEATHER_RAIN".to_string(),
                    move_type: pokemon_type("WATER"),
                    multiplier: crystal_core::battle::damage::TypeMultiplier {
                        numerator: 3,
                        denominator: 2,
                    },
                }],
                move_effect_modifiers: vec![
                    crystal_core::battle::damage::WeatherMoveEffectModifier {
                        weather: "WEATHER_RAIN".to_string(),
                        move_effect: "SOLARBEAM".to_string(),
                        multiplier: crystal_core::battle::damage::TypeMultiplier {
                            numerator: 1,
                            denominator: 2,
                        },
                    },
                ],
            },
            battle_reward_rules: minimal_battle_reward_rules(),
            step_event_rules: minimal_step_event_rules(),
            field_moves: minimal_field_move_catalog(),
            ..GameDataSet::default()
        }
    }

    fn verified_runtime_bootstrap_data() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![crystal_core::systems::learnsets::LearnsetEntry(
                1,
                "TACKLE".to_string(),
            )],
        );
        data.items
            .insert("POKE_BALL".to_string(), runtime_ball_item("POKE_BALL"));
        data.items.insert(
            "BLU_APRICORN".to_string(),
            runtime_item("BLU_APRICORN", item_pocket("ITEM")),
        );
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .objects
            .push(runtime_object("RuntimeNpc", "EVENT_RUNTIME_NPC"));
        data.map_scripts.insert(
            "RuntimeMap".to_string(),
            serde_json::json!({ "RuntimeScript": [] }),
        );
        data.marts
            .0
            .insert("MART_RUNTIME".to_string(), vec!["POKE_BALL".to_string()]);
        data.fishing.groups.insert(
            "FISHGROUP_RUNTIME".to_string(),
            crystal_core::world::fishing::FishingGroup {
                bite_threshold: 255,
                rod_tables: [(
                    crystal_core::world::fishing::ROD_OLD.to_string(),
                    crystal_core::world::fishing::RodTable {
                        slots: vec![crystal_core::world::fishing::FishingSlot {
                            threshold: 255,
                            species: Some("CHIKORITA".to_string()),
                            level: 5,
                            time_group: None,
                        }],
                    },
                )]
                .into_iter()
                .collect(),
            },
        );
        data.fishing.rod_items = vec![crystal_core::world::fishing::FishingRodItemRule {
            item_id: "OLD_ROD".to_string(),
            rod: crystal_core::world::fishing::ROD_OLD.to_string(),
        }];
        data.fruit_trees
            .0
            .insert("FRUITTREE_RUNTIME".to_string(), "BLU_APRICORN".to_string());
        data.pc_strings
            .insert("PC_RUNTIME".to_string(), "Runtime PC".to_string());
        data.menu_icons
            .insert("CHIKORITA".to_string(), "ICON_CHIKORITA".to_string());
        data.pokedex_entries.insert(
            "CHIKORITA".to_string(),
            crystal_core::models::RuntimePokedexEntry {
                species: "CHIKORITA".to_string(),
                classification: "Leaf".to_string(),
                height_digits: 9,
                weight_digits: 64,
                pages: vec!["A sweet leaf Pokemon.".to_string()],
            },
        );
        data.pokemon_frontpic_anim.insert(
            "CHIKORITA".to_string(),
            crystal_core::models::FrontpicAnimProgram {
                commands: Vec::new(),
            },
        );
        data.move_names = vec!["TACKLE".to_string()];
        data.asm_text
            .insert("RuntimeText".to_string(), "Runtime text.".to_string());
        data.battle_animations
            .insert("TACKLE".to_string(), vec!["BATTLE_ANIM_END".to_string()]);
        data.battle_animation_table = vec!["TACKLE".to_string()];
        data.battle_anim_bundle = serde_json::json!({ "animations": ["TACKLE"] }).to_string();
        data.sprite_anim_bundle = serde_json::json!({ "sprites": ["SPRITE_MON"] }).to_string();
        data.sprite_palette_defaults
            .insert("SPRITE_MON".to_string(), 0);
        data.pokegear_town_map_palette_map
            .insert("RuntimeMap".to_string(), vec!["PAL_RUNTIME".to_string()]);
        data.pokegear_landmarks.landmarks = vec![crystal_core::models::PokegearLandmark {
            id: 1,
            constant: "LANDMARK_RUNTIME".to_string(),
            label: "RuntimeLandmark".to_string(),
            name: "Runtime".to_string(),
            x: 1,
            y: 1,
            region: "johto".to_string(),
        }];
        data.pokegear_landmarks
            .map_to_landmark
            .insert("RuntimeMap".to_string(), "LANDMARK_RUNTIME".to_string());
        data.pokemon_cries.insert(
            "CHIKORITA".to_string(),
            crystal_assets::PokemonCryMetadata {
                cry: "CRY_NIDORAN_M".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        data.trainers.trainers.insert(
            "TRAINER_RUNTIME".to_string(),
            crystal_core::models::Trainer {
                name: "Runtime".to_string(),
                trainer_id: "TRAINER_RUNTIME".to_string(),
                trainer_class: "YOUNGSTER".to_string(),
                party: vec![crystal_core::models::TrainerPartyPokemon {
                    species: "CHIKORITA".to_string(),
                    level: 5,
                    item: None,
                    moves: Vec::new(),
                    dvs: Dv::default(),
                }],
                win_quote: "Win".to_string(),
                lose_quote: "Lose".to_string(),
                items: Vec::new(),
                base_reward: 1,
                ai_move_flags: 0,
                ai_item_switch_flags: 0,
                encounter_music: "MUSIC_ROUTE_29".to_string(),
                ai_layers: Vec::new(),
            },
        );
        data.phone_contacts.0.insert(
            "PHONE_RUNTIME".to_string(),
            crystal_core::systems::phone::PhoneContactRecord {
                contact_id: "PHONE_RUNTIME".to_string(),
                trainer_class: None,
                trainer_label: None,
                lines: vec!["Hello.".to_string()],
                primary_label: "RuntimePhone".to_string(),
                map_constant: Some("RUNTIME_MAP".to_string()),
                callee_time_mask: 0xff,
                callee_script: Some("RuntimePhoneScript".to_string()),
                caller_time_mask: 0xff,
                caller_script: Some("RuntimePhoneScript".to_string()),
            },
        );
        data.permanent_phone_numbers = vec!["PHONE_RUNTIME".to_string()];
        data.special_phone_calls = ["RuntimePhoneScript".to_string()].into_iter().collect();
        data.phone_scripts = vec![serde_json::json!({ "RuntimePhoneScript": [] })];
        data.flee_mons.always = vec!["CHIKORITA".to_string()];
        data.buena_password_categories = vec![
            crystal_core::systems::special_routines::BuenaPasswordCategoryDefinition {
                id: "BUENA_RUNTIME".to_string(),
                category_type: crystal_core::systems::special_routines::BUENA_PASSWORD_CATEGORY_MON
                    .to_string(),
                points: 1,
                options: vec!["CHIKORITA".to_string()],
            },
        ];
        data.roaming_pokemon = vec![
            crystal_core::systems::special_routines::RoamingPokemonDefinition {
                species: "CHIKORITA".to_string(),
                level: 40,
                map_group: 1,
                map_number: 1,
            },
        ];
        data.buena_prizes = vec![
            crystal_core::systems::special_routines::BuenaPrizeDefinition {
                item_id: "POKE_BALL".to_string(),
                cost: 1,
            },
        ];
        data.kurt_apricorn_recipes = vec![
            crystal_core::systems::special_routines::KurtApricornRecipe {
                apricorn: "BLU_APRICORN".to_string(),
                ball: "POKE_BALL".to_string(),
            },
        ];
        data.shuckie_gift = Some(
            crystal_core::systems::special_routines::ShuckieGiftDefinition {
                species: "CHIKORITA".to_string(),
                level: 15,
                held_item: "POKE_BALL".to_string(),
                nickname: "SHUCKIE".to_string(),
                original_trainer_name: "MANIA".to_string(),
                original_trainer_id: 1,
                got_today_engine_flag: "ENGINE_GOT_SHUCKIE_TODAY".to_string(),
            },
        );
        data.dratini_move_sets = vec![
            crystal_core::systems::special_routines::DratiniMoveSetDefinition {
                mode: 0,
                moves: vec!["TACKLE".to_string()],
            },
        ];
        data.bug_contest_config = Some(crystal_core::systems::special_routines::BugContestConfig {
            park_balls: 20,
            timer_minutes: 20,
            timer_seconds: 0,
            selected_contestant_count: 1,
            contestant_flags: vec!["EVENT_RUNTIME_CONTESTANT".to_string()],
        });
        data.battle_tower_rules = Some(crystal_core::systems::special_routines::BattleTowerRules {
            banned_species: Vec::new(),
            required_party_count: 3,
            challenge_streak_length: 7,
            minimum_level_group: 10,
            maximum_level_group: 100,
            level_group_size: 10,
            party_count_failure_text: "Need three.".to_string(),
            duplicate_species_failure_text: "No duplicates.".to_string(),
            duplicate_held_item_failure_text: "No duplicate items.".to_string(),
            egg_failure_text: "No eggs.".to_string(),
        });
        data.odd_egg_definitions =
            vec![crystal_core::systems::special_routines::OddEggDefinition {
                species: "CHIKORITA".to_string(),
                moves: vec!["TACKLE".to_string()],
                original_trainer_id: 1,
                dvs: [0; 4],
                probability: 100,
                level: 5,
                experience: 0,
                hatch_cycles: 1,
                nickname: "EGG".to_string(),
                original_trainer_name: "DAYCARE".to_string(),
            }];
        data.magikarp_lengths = vec![
            crystal_core::systems::special_routines::MagikarpLengthEntry {
                threshold: 1,
                divisor: 1,
            },
        ];
        data.happiness_data = Some(crystal_core::systems::special_routines::HappinessData {
            changes: Vec::new(),
            services: Vec::new(),
        });
        data.initialize_events.event_flags = vec!["EVENT_RUNTIME_CONTESTANT".to_string()];
        data.initialize_events.engine_flags = vec!["ENGINE_GOT_SHUCKIE_TODAY".to_string()];
        data.story_event_script_constants
            .global
            .insert("EVENT_RUNTIME".to_string(), 1);
        data.tilesets = vec![serde_json::json!({ "id": "johto" })];
        data
    }

    fn minimal_step_event_rules() -> StepEventRules {
        StepEventRules {
            poison_step_interval: 4,
            egg_step_trigger: 0x80,
            hatched_egg_happiness: 0x78,
            poison_status: "POISON".to_string(),
            egg_nickname: "EGG".to_string(),
            happiness_step_counter_mask: 1,
            happiness_step_counter_target: 0,
        }
    }

    fn minimal_capture_ball_rule() -> crystal_core::battle::capture::CaptureBallRule {
        crystal_core::battle::capture::CaptureBallRule {
            multiplier_numerator: 1,
            multiplier_denominator: 1,
            battle_type: String::new(),
            skip_hp_calc: false,
            use_heavy_ball_weight_modifier: false,
            use_level_ball_multiplier: false,
            require_same_species: false,
            require_same_gender: false,
            require_fast_species: false,
        }
    }

    fn minimal_capture_rules() -> crystal_core::battle::capture::CaptureRules {
        crystal_core::battle::capture::CaptureRules {
            fast_ball_species: BTreeSet::new(),
            heavy_ball_modifiers: BTreeMap::new(),
            ball_rules: [
                ("MASTER_BALL".to_string(), minimal_capture_ball_rule()),
                ("POKE_BALL".to_string(), minimal_capture_ball_rule()),
            ]
            .into_iter()
            .collect(),
            guaranteed_capture_balls: ["MASTER_BALL".to_string()].into_iter().collect(),
            status_bonus: [("SLEEP".to_string(), 10), ("FREEZE".to_string(), 10)]
                .into_iter()
                .collect(),
        }
    }

    fn minimal_battle_reward_rules() -> BattleRewardRules {
        BattleRewardRules {
            max_level: 100,
            wild_exp_divisor: 7,
            trainer_exp_numerator: 3,
            trainer_exp_denominator: 2,
        }
    }

    fn minimal_battle_escape_rules() -> crystal_core::systems::battle_escape::BattleEscapeRules {
        crystal_core::systems::battle_escape::BattleEscapeRules {
            player_speed_multiplier: 32,
            enemy_speed_divisor: 4,
            failed_attempt_bonus: 30,
            rng_roll_values: 256,
        }
    }

    fn field_move_badge(index: usize) -> FieldMoveBadgeRequirement {
        FieldMoveBadgeRequirement {
            region: "johto".to_string(),
            index,
        }
    }

    fn field_move_replacement(
        tileset: &str,
        block_id: u16,
        replacement_block_id: u16,
        variant: &str,
    ) -> FieldMoveReplacement {
        FieldMoveReplacement {
            tileset: tileset.to_string(),
            block_id,
            replacement_block_id,
            variant: variant.to_string(),
        }
    }

    fn minimal_field_move_catalog() -> FieldMoveCatalog {
        FieldMoveCatalog {
            cut: FieldMoveBlockRule {
                move_id: "CUT".to_string(),
                badge: field_move_badge(1),
                target_collisions: vec![0x12, 0x1a, 0x18, 0x14, 0x1c],
                replacements: vec![field_move_replacement("johto", 0x5b, 0x3c, "tree")],
            },
            whirlpool: FieldMoveBlockRule {
                move_id: "WHIRLPOOL".to_string(),
                badge: field_move_badge(6),
                target_collisions: vec![0x24, 0x2c],
                replacements: vec![field_move_replacement("johto", 0x07, 0x36, "whirlpool")],
            },
            strength: FieldMoveFlagRule {
                move_id: "STRENGTH".to_string(),
                badge: field_move_badge(2),
                engine_flag: "ENGINE_STRENGTH_ACTIVE".to_string(),
            },
            flash: FieldMoveFlagRule {
                move_id: "FLASH".to_string(),
                badge: field_move_badge(0),
                engine_flag: "STATUSFLAGS_FLASH".to_string(),
            },
            surf: FieldMoveTravelRule {
                move_id: "SURF".to_string(),
                badge: field_move_badge(3),
                blocked_collisions: vec![0x24, 0x2c, 0x33, 0x30, 0x31, 0x32],
                target_collisions: Vec::new(),
            },
            waterfall: FieldMoveTravelRule {
                move_id: "WATERFALL".to_string(),
                badge: field_move_badge(7),
                blocked_collisions: Vec::new(),
                target_collisions: vec![0x33, 0x30, 0x31, 0x32, 0x3b],
            },
            fly: FieldMoveRule {
                move_id: "FLY".to_string(),
                badge: field_move_badge(5),
            },
            dig: FieldMoveMoveRule {
                move_id: "DIG".to_string(),
            },
            teleport: FieldMoveMoveRule {
                move_id: "TELEPORT".to_string(),
            },
            escape_rope: FieldEscapeItemRule {
                item_id: "ESCAPE_ROPE".to_string(),
                escape_rope_mode: "DIG_WARP".to_string(),
            },
            repel: FieldRepelItemRule {},
            bicycle: FieldItemRule {
                item_id: "BICYCLE".to_string(),
            },
            itemfinder: FieldItemRule {
                item_id: "ITEMFINDER".to_string(),
            },
            squirtbottle: FieldItemRule {
                item_id: "SQUIRTBOTTLE".to_string(),
            },
            coin_case: FieldItemRule {
                item_id: "COIN_CASE".to_string(),
            },
            blue_card: FieldItemRule {
                item_id: "BLUE_CARD".to_string(),
            },
            town_map: FieldItemRule {
                item_id: "TOWN_MAP".to_string(),
            },
        }
    }

    fn add_runtime_fly_destination(data: &mut GameDataSet) {
        let mut fly_map = runtime_map();
        fly_map.id = "FlyMap".to_string();
        fly_map.attributes.map_constant = Some("FLY_MAP".to_string());
        fly_map.attributes.environment = Some("town".to_string());
        fly_map.attributes.location = Some("johto".to_string());
        data.map_attributes
            .insert("FlyMap".to_string(), fly_map.attributes.clone());
        data.maps.insert("FlyMap".to_string(), fly_map);
        data.runtime_map_metadata.insert(
            "FLY_MAP".to_string(),
            runtime_map_metadata("FLY_MAP", "FlyMap", 2, 2, "TOWN"),
        );
        data.runtime_spawn_points.insert(
            "14".to_string(),
            RuntimeSpawnPoint {
                identifier: 14,
                map_constant: "FLY_MAP".to_string(),
                map_name: "FlyMap".to_string(),
                group_id: 2,
                map_id: 2,
                tile_x: 0,
                tile_y: 0,
                group_name: "FLY".to_string(),
                metatile_x: 0,
                metatile_y: 0,
                subtile_x: 0,
                subtile_y: 0,
            },
        );
    }

    fn add_runtime_teleport_destination(data: &mut GameDataSet) {
        let mut teleport_map = runtime_map();
        teleport_map.id = "TeleportMap".to_string();
        teleport_map.attributes.map_constant = Some("TELEPORT_MAP".to_string());
        teleport_map.attributes.environment = Some("town".to_string());
        data.map_attributes
            .insert("TeleportMap".to_string(), teleport_map.attributes.clone());
        data.maps.insert("TeleportMap".to_string(), teleport_map);
        data.runtime_map_metadata.insert(
            "TELEPORT_MAP".to_string(),
            runtime_map_metadata("TELEPORT_MAP", "TeleportMap", 2, 3, "TOWN"),
        );
        data.runtime_spawn_points.insert(
            "21".to_string(),
            RuntimeSpawnPoint {
                identifier: 21,
                map_constant: "TELEPORT_MAP".to_string(),
                map_name: "TeleportMap".to_string(),
                group_id: 2,
                map_id: 3,
                tile_x: 1,
                tile_y: 0,
                group_name: "TELEPORT".to_string(),
                metatile_x: 0,
                metatile_y: 0,
                subtile_x: 1,
                subtile_y: 0,
            },
        );
    }

    fn add_runtime_field_encounters(data: &mut GameDataSet) {
        data.field_encounters.insert(
            "RuntimeMap".to_string(),
            FieldEncounterData {
                map_name: "RuntimeMap".to_string(),
                headbutt: Some(FieldEncounterTable {
                    common: vec![FieldEncounterEntry {
                        weight: 100,
                        species: "CHIKORITA".to_string(),
                        level: 10,
                    }],
                    rare: vec![FieldEncounterEntry {
                        weight: 100,
                        species: "CHIKORITA".to_string(),
                        level: 12,
                    }],
                }),
                rock_smash: Some(FieldEncounterTable {
                    common: vec![FieldEncounterEntry {
                        weight: 100,
                        species: "CHIKORITA".to_string(),
                        level: 15,
                    }],
                    rare: Vec::new(),
                }),
            },
        );
    }

    fn minimal_runtime_data_with_music() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .music = Some("MUSIC_ROUTE_29".to_string());
        data.audio = vec![ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "content-packs/test/music/MUSIC_ROUTE_29.mid",
        )
        .expect("music asset")];
        data
    }

    fn minimal_runtime_data_with_script_audio_and_map_commands() -> GameDataSet {
        let mut data = minimal_runtime_data_with_music();
        data.audio.push(
            ModpackAudioAsset::sound_effect("SFX_TACKLE", "content-packs/test/sfx/SFX_TACKLE.mid")
                .expect("sfx asset"),
        );
        data.audio.push(
            ModpackAudioAsset::cry(
                "CRY_CHIKORITA",
                "content-packs/test/cries/CRY_CHIKORITA.mid",
            )
            .expect("cry asset"),
        );
        data.pokemon_cries.insert(
            "CHIKORITA".to_string(),
            PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.script_audio_commands = vec![
            ScriptAudioCommand {
                command: "playmusic".to_string(),
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 0,
            },
            ScriptAudioCommand {
                command: "playsound".to_string(),
                audio_id: Some("SFX_TACKLE".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 1,
            },
            ScriptAudioCommand {
                command: "cry".to_string(),
                audio_id: Some("CHIKORITA".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 2,
            },
            ScriptAudioCommand {
                command: "PlayMusic".to_string(),
                audio_id: Some("MUSIC_ROUTE_29".to_string()),
                fade_frames: None,
                source_script: "RuntimeAudioScript".to_string(),
                command_index: 3,
            },
        ];
        map.script_map_commands = vec![
            ScriptMapCommand {
                command: "warpfacing".to_string(),
                target_map: Some("RuntimeMap".to_string()),
                x: Some(1),
                y: Some(0),
                facing: Some("RIGHT".to_string()),
                map_setup: None,
                source_script: "RuntimeWarpScript".to_string(),
                command_index: 0,
            },
            ScriptMapCommand {
                command: "warp".to_string(),
                target_map: Some("NONE".to_string()),
                x: Some(0),
                y: Some(0),
                facing: None,
                map_setup: None,
                source_script: "RuntimeWarpScript".to_string(),
                command_index: 1,
            },
        ];
        data
    }

    fn minimal_runtime_data_with_text_variable_and_control_commands() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.currency_constants
            .0
            .insert("RUNTIME_PRICE".to_string(), 500);
        data.story_event_script_constants
            .global
            .insert("RUNTIME_BADGES".to_string(), 8);
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.script_text_bodies.insert(
            "RuntimeGreetingText".to_string(),
            ScriptTextBody {
                label: "RuntimeGreetingText".to_string(),
                commands: Vec::new(),
            },
        );
        map.script_text_commands = vec![
            ScriptTextCommand {
                command: "opentext".to_string(),
                text_label: None,
                source_script: "RuntimeScript".to_string(),
                command_index: 0,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("RuntimeGreetingText".to_string()),
                source_script: "RuntimeScript".to_string(),
                command_index: 1,
            },
            ScriptTextCommand {
                command: "yesorno".to_string(),
                text_label: None,
                source_script: "RuntimeScript".to_string(),
                command_index: 2,
            },
            ScriptTextCommand {
                command: "writetext".to_string(),
                text_label: Some("runtimegreetingtext".to_string()),
                source_script: "RuntimeScript".to_string(),
                command_index: 3,
            },
        ];
        map.script_variable_commands = vec![
            ScriptVariableCommand {
                command: "loadvar".to_string(),
                target: Some("VAR_CALLERID".to_string()),
                value_tokens: vec!["PHONE_BIRDKEEPER_VANCE".to_string()],
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 0,
            },
            ScriptVariableCommand {
                command: "readvar".to_string(),
                target: Some("VAR_CALLERID".to_string()),
                value_tokens: Vec::new(),
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 1,
            },
            ScriptVariableCommand {
                command: "checktime".to_string(),
                target: None,
                value_tokens: vec!["NITE".to_string()],
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 2,
            },
            ScriptVariableCommand {
                command: "setval".to_string(),
                target: None,
                value_tokens: vec!["8".to_string()],
                source_script: "RuntimeVariableScript".to_string(),
                command_index: 3,
            },
        ];
        map.script_control_commands = vec![
            ScriptControlCommand {
                command: "iftrue".to_string(),
                compare_value: None,
                target_label: Some(".Accepted".to_string()),
                resolved_target_script: Some("RuntimeAcceptedScript".to_string()),
                source_script: "RuntimeControlScript".to_string(),
                command_index: 0,
            },
            ScriptControlCommand {
                command: "ifgreater".to_string(),
                compare_value: Some("RUNTIME_BADGES - 1".to_string()),
                target_label: Some(".Enough".to_string()),
                resolved_target_script: Some("RuntimeEnoughScript".to_string()),
                source_script: "RuntimeControlScript".to_string(),
                command_index: 1,
            },
            ScriptControlCommand {
                command: "jumpstd".to_string(),
                compare_value: None,
                target_label: Some("PokecenterSignScript".to_string()),
                resolved_target_script: None,
                source_script: "RuntimeControlScript".to_string(),
                command_index: 2,
            },
        ];
        data
    }

    fn minimal_runtime_data_with_object_and_movement_commands() -> GameDataSet {
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut npc = runtime_object("RUNTIME_NPC", "EVENT_RUNTIME_NPC_HIDDEN");
        npc.x = 1;
        npc.y = 0;
        let mut guide = runtime_object("RUNTIME_GUIDE", "-1");
        guide.x = 0;
        guide.y = 0;
        map.objects = vec![npc, guide];
        map.script_object_commands = vec![
            ScriptObjectCommand {
                command: "moveobject".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: Some(0),
                y: Some(0),
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 0,
            },
            ScriptObjectCommand {
                command: "turnobject".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: Some("LEFT".to_string()),
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 1,
            },
            ScriptObjectCommand {
                command: "disappear".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 2,
            },
            ScriptObjectCommand {
                command: "appear".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 3,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("RuntimeNpcMovement".to_string()),
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 4,
            },
            ScriptObjectCommand {
                command: "follow".to_string(),
                object_id: Some("RUNTIME_GUIDE".to_string()),
                target_object_id: Some("PLAYER".to_string()),
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 5,
            },
            ScriptObjectCommand {
                command: "stopfollow".to_string(),
                object_id: None,
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: None,
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 6,
            },
            ScriptObjectCommand {
                command: "applymovement".to_string(),
                object_id: Some("RUNTIME_NPC".to_string()),
                target_object_id: None,
                x: None,
                y: None,
                direction: None,
                movement: Some("runtimenpcmovement".to_string()),
                emote: None,
                duration: None,
                source_script: "RuntimeObjectScript".to_string(),
                command_index: 7,
            },
        ];
        map.script_movements = vec![ScriptMovement {
            label: "RuntimeNpcMovement".to_string(),
            source_script: Some("RuntimeObjectScript".to_string()),
            steps: vec![
                ScriptMovementStep {
                    command: "step".to_string(),
                    direction: Some("RIGHT".to_string()),
                    duration: None,
                    index: 0,
                },
                ScriptMovementStep {
                    command: "turn_head".to_string(),
                    direction: Some("UP".to_string()),
                    duration: None,
                    index: 1,
                },
                ScriptMovementStep {
                    command: "step_end".to_string(),
                    direction: None,
                    duration: None,
                    index: 2,
                },
            ],
        }];
        data
    }

    fn minimal_runtime_data_with_runtime_commands() -> GameDataSet {
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.objects = vec![runtime_object("RUNTIME_NPC", "-1")];
        map.script_runtime_commands = vec![
            ScriptRuntimeCommand {
                command: "special".to_string(),
                args: vec!["FadeOutMusic".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 0,
            },
            ScriptRuntimeCommand {
                command: "pause".to_string(),
                args: vec!["15".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 1,
            },
            ScriptRuntimeCommand {
                command: "random".to_string(),
                args: vec!["10".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 2,
            },
            ScriptRuntimeCommand {
                command: "checkver".to_string(),
                args: Vec::new(),
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 3,
            },
            ScriptRuntimeCommand {
                command: "writevar".to_string(),
                args: vec!["VAR_BLUECARDBALANCE".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 4,
            },
            ScriptRuntimeCommand {
                command: "getnum".to_string(),
                args: vec!["STRING_BUFFER_3".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 5,
            },
            ScriptRuntimeCommand {
                command: "setlasttalked".to_string(),
                args: vec!["RUNTIME_NPC".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 6,
            },
            ScriptRuntimeCommand {
                command: "setlasttalked".to_string(),
                args: vec!["runtime_npc".to_string()],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 7,
            },
            ScriptRuntimeCommand {
                command: "showemote".to_string(),
                args: vec![
                    "EMOTE_SHOCK".to_string(),
                    "RUNTIME_NPC".to_string(),
                    "15".to_string(),
                ],
                source_script: "RuntimeCommandScript".to_string(),
                command_index: 8,
            },
        ];
        data
    }

    fn minimal_runtime_data_with_coord_event() -> GameDataSet {
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.attributes.width = 3;
        map.attributes.height = 2;
        map.blocks = vec![0; 6];
        map.scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_RUNTIME_ACTIVE".to_string(),
                script_name: Some("RuntimeSceneScript".to_string()),
            }],
        };
        map.events.coord_events = vec![CoordEvent {
            x: 1,
            y: 0,
            scene_id: "SCENE_RUNTIME_ACTIVE".to_string(),
            script_name: "RuntimeCoordScript".to_string(),
        }];
        let spawn = data
            .runtime_spawn_points
            .get_mut("0")
            .expect("runtime spawn point");
        spawn.tile_x = 1;
        spawn.tile_y = 1;
        data
    }

    fn minimal_runtime_data_with_grass_encounter() -> GameDataSet {
        let mut data = minimal_runtime_data_with_music();
        data.maps.get_mut("RuntimeMap").expect("runtime map").blocks = vec![0, 1];
        let encounter = WildEncounter {
            level: 2,
            species: "CHIKORITA".to_string(),
        };
        let grass_slots = vec![encounter.clone(); 7];
        data.wild_encounters.insert(
            "RuntimeMap".to_string(),
            WildEncounterData {
                map_name: "RuntimeMap".to_string(),
                grass_rates: Some(
                    [
                        ("morning".to_string(), 100),
                        ("day".to_string(), 100),
                        ("night".to_string(), 100),
                    ]
                    .into_iter()
                    .collect(),
                ),
                water_rate: None,
                grass: Some(WildEncounterTable {
                    morning: grass_slots.clone(),
                    day: grass_slots.clone(),
                    night: grass_slots,
                }),
                water: None,
            },
        );
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(1, "TACKLE".to_string())],
        );
        data
    }

    fn minimal_runtime_data_with_fishing() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .fishing_group = Some("FISHGROUP_RUNTIME".to_string());
        data.fishing = FishingCatalog {
            groups: [(
                "FISHGROUP_RUNTIME".to_string(),
                FishingGroup {
                    bite_threshold: 255,
                    rod_tables: [(
                        ROD_GOOD.to_string(),
                        RodTable {
                            slots: vec![FishingSlot {
                                threshold: 255,
                                species: Some("CHIKORITA".to_string()),
                                level: 9,
                                time_group: None,
                            }],
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            )]
            .into_iter()
            .collect(),
            time_groups: Vec::new(),
            swarm_rules: Vec::new(),
            rod_items: vec![FishingRodItemRule {
                item_id: "GOOD_ROD".to_string(),
                rod: ROD_GOOD.to_string(),
            }],
        };
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(1, "TACKLE".to_string())],
        );
        data
    }

    fn minimal_runtime_data_with_scripted_battles() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.items
            .insert("MASTER_BALL".to_string(), runtime_ball_item("MASTER_BALL"));
        data.items.insert(
            "BERRY".to_string(),
            runtime_item("BERRY", item_pocket("ITEM")),
        );
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![LearnsetEntry(1, "TACKLE".to_string())],
        );
        data.trainers
            .insert(Trainer {
                name: "RIVAL@".to_string(),
                trainer_id: "RIVAL1".to_string(),
                trainer_class: "RIVAL1".to_string(),
                party: vec![TrainerPartyPokemon {
                    species: "CHIKORITA".to_string(),
                    level: 5,
                    item: None,
                    moves: Vec::new(),
                    dvs: Dv::from_non_hp(0, 0, 0, 0),
                }],
                win_quote: "RivalWinText".to_string(),
                lose_quote: "RivalLossText".to_string(),
                items: Vec::new(),
                base_reward: 100,
                ai_move_flags: 1,
                ai_item_switch_flags: 0,
                encounter_music: "MUSIC_RIVAL_ENCOUNTER".to_string(),
                ai_layers: vec!["AI_BASIC".to_string()],
            })
            .expect("trainer inserts");
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.objects.push(runtime_object(
            "RUNTIME_STATIC_MON",
            "EVENT_RUNTIME_STATIC_MON_HIDDEN",
        ));
        map.scripted_wild_battles.push(ScriptedWildBattle {
            source_script: "RuntimeWildScript".to_string(),
            loadwildmon_command_index: 3,
            startbattle_command_index: 4,
            request: StaticWildBattleRequest {
                battle_type: "BATTLETYPE_NORMAL".to_string(),
                species: "CHIKORITA".to_string(),
                level: 6,
                source_script: "RuntimeWildScript".to_string(),
            },
            reload_map_after_battle: true,
            pre_battle_event_flags: vec!["EVENT_RUNTIME_WILD_READY".to_string()],
            post_battle_event_flags: vec!["EVENT_RUNTIME_WILD_DONE".to_string()],
            post_battle_script_flags: vec!["ENGINE_RUNTIME_WILD_DONE".to_string()],
            disappear_object_ids: vec!["RUNTIME_STATIC_MON".to_string()],
        });
        map.scripted_trainer_battles.push(ScriptedTrainerBattle {
            source_script: "RuntimeTrainerScript".to_string(),
            loadtrainer_command_index: 7,
            startbattle_command_index: 8,
            request: {
                let mut request =
                    TrainerBattleRequest::new("RIVAL1", "RIVAL1", "EVENT_BEAT_RUNTIME_RIVAL");
                request.seen_text = "RuntimeSeenText".to_string();
                request.win_text = "RuntimeWinText".to_string();
                request.loss_text = "RuntimeLossText".to_string();
                request.source_script = "RuntimeTrainerScript".to_string();
                request
            },
            reload_map_after_battle: false,
            post_battle_event_flags: vec!["EVENT_RUNTIME_TRAINER_POST".to_string()],
            post_battle_script_flags: vec!["ENGINE_RUNTIME_TRAINER_POST".to_string()],
        });
        map.gift_pokemon_scripts.push(GiftPokemonScript {
            species_id: "CHIKORITA".to_string(),
            level_token: "7".to_string(),
            level: 7,
            held_item_id: Some("BERRY".to_string()),
            nickname_label: Some("RuntimeGiftName".to_string()),
            ot_label: None,
            source_script: "RuntimeGiftScript".to_string(),
            command_index: 12,
            egg: false,
        });
        map.gift_pokemon_scripts.push(GiftPokemonScript {
            species_id: "CHIKORITA".to_string(),
            level_token: "EGG_LEVEL".to_string(),
            level: 5,
            held_item_id: None,
            nickname_label: None,
            ot_label: None,
            source_script: "RuntimeEggScript".to_string(),
            command_index: 3,
            egg: true,
        });
        data
    }

    fn minimal_runtime_data_with_battle_rewards() -> GameDataSet {
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves.insert(
            "RAZOR_LEAF".to_string(),
            runtime_move_named("RAZOR_LEAF", 25),
        );
        let mut chikorita = runtime_species();
        chikorita.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        chikorita.base_exp = 64;
        data.pokemon.insert("CHIKORITA".to_string(), chikorita);
        let mut bayleef =
            PokemonSpecies::new_for_tests("BAYLEEF", BaseStats::new(60, 62, 80, 60, 63, 80));
        bayleef.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        bayleef.base_exp = 141;
        data.pokemon.insert("BAYLEEF".to_string(), bayleef);
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(16, "RAZOR_LEAF".to_string()),
            ],
        );
        data.learnsets.insert("BAYLEEF".to_string(), Vec::new());
        data.evolutions.0.insert(
            "CHIKORITA".to_string(),
            vec![EvolutionEntry::level("BAYLEEF", 16)],
        );
        data.evolutions.0.insert("BAYLEEF".to_string(), Vec::new());
        data
    }

    #[test]
    fn runtime_bootstrap_loads_compiled_pack_and_declared_midi_assets() {
        let root = temp_repository_root("loads");
        let data_root = root.join("apps/web/assets/data");
        write_midi(&data_root.join("content-packs/test/music/MUSIC_ROUTE_29.mid"));
        write_midi(&data_root.join("content-packs/test/sfx/SFX_TACKLE.mid"));
        write_midi(&data_root.join("content-packs/test/cries/CRY_NIDORAN_M.mid"));
        let mut data = verified_runtime_bootstrap_data();
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::sound_effect("SFX_TACKLE", "content-packs/test/sfx/SFX_TACKLE.mid")
                .expect("sfx asset"),
            ModpackAudioAsset::cry(
                "CRY_NIDORAN_M",
                "content-packs/test/cries/CRY_NIDORAN_M.mid",
            )
            .expect("cry asset"),
        ];
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report());
        crystal_assets::write_compiled_game_pack_for_tests(
            data_root.join("runtime.crystalpack"),
            &pack,
        )
        .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);

        let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect("load runtime");

        assert_eq!(runtime.modpack.id(), "core-modular");
        assert_eq!(runtime.modpack.hash().len(), 8);
        assert!(runtime
            .audio
            .program(AudioKind::Music, "MUSIC_ROUTE_29")
            .is_some());
        assert!(runtime
            .audio
            .program(AudioKind::SoundEffect, "SFX_TACKLE")
            .is_some());
        assert!(runtime
            .audio
            .program(AudioKind::Cry, "CRY_NIDORAN_M")
            .is_some());
        let summary = runtime.boot_summary();
        assert_eq!(summary.modpack_id, "core-modular");
        assert_eq!(summary.pokemon_species, 1);
        assert_eq!(summary.moves, 1);
        assert_eq!(summary.maps, 1);
        assert_eq!(summary.music_tracks, 1);
        assert_eq!(summary.sound_effects, 1);
        assert_eq!(summary.cries, 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_loads_declared_pcm_cries() {
        let root = temp_repository_root("pcm-cry");
        let data_root = root.join("apps/web/assets/data");
        write_midi(&data_root.join("content-packs/test/music/MUSIC_ROUTE_29.mid"));
        write_pcm(&data_root.join("content-packs/test/cries/CRY_NIDORAN_M.pcm"));
        let mut data = verified_runtime_bootstrap_data();
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
            ModpackAudioAsset::cry_pcm(
                "CRY_NIDORAN_M",
                "content-packs/test/cries/CRY_NIDORAN_M.pcm",
                22050,
                1,
            )
            .expect("pcm cry asset"),
        ];
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report());
        crystal_assets::write_compiled_game_pack_for_tests(
            data_root.join("runtime.crystalpack"),
            &pack,
        )
        .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);

        let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect("load runtime with PCM cry");
        let cry = runtime
            .audio
            .program(AudioKind::Cry, "CRY_NIDORAN_M")
            .expect("pcm cry program");

        assert_eq!(
            cry.source,
            AudioProgramSource::Pcm {
                sample_rate_hz: 22050,
                channels: 1,
                bytes: vec![0_u8, 0, 0xff, 0x7f],
            }
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_starts_from_declared_spawn_and_steps_from_joypad() {
        let root = temp_repository_root("overworld");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        assert_eq!(
            session.state.overworld,
            OverworldMemory::Active {
                map_name: "RuntimeMap".to_string(),
                tile: TilePosition::new(0, 0),
                facing: Direction::Down,
                mode: crystal_core::world::movement::MovementMode::Normal,
            }
        );

        let first = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        assert_eq!(first.snapshot.tile, TilePosition::new(0, 0));
        assert_eq!(first.snapshot.facing, Direction::Right);
        assert!(matches!(
            first.movement,
            Some(StepOutcome::Turned {
                facing: Direction::Right
            })
        ));
        assert_eq!(first.step_events, None);

        let second = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("move right");
        assert_eq!(second.snapshot.tile, TilePosition::new(2, 0));
        assert_eq!(second.step_events, Some(StepEventResult::default()));
        assert_eq!(second.snapshot.frame, 2);
        assert_eq!(session.state.frame_counter, 2);
        assert_eq!(
            session.state.overworld.snapshot_identity(),
            Some((
                "RuntimeMap",
                TilePosition::new(2, 0),
                Direction::Right,
                crystal_core::world::movement::MovementMode::Normal
            ))
        );
        assert_eq!(second.input_mask, B_PAD_RIGHT);
        assert_eq!(second.pressed_mask, 0);
        assert_eq!(second.state_checksum.frame(), 2);
        assert_eq!(
            session
                .state_checksum_frame(7)
                .expect("checksum frame")
                .checksum(),
            second.state_checksum
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_processes_step_events_on_actual_moves_with_exact_statuses() {
        let root = temp_repository_root("step-events");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut poisoned = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        poisoned.hp = 3;
        poisoned.status = Some(runtime.data.step_event_rules.poison_status.clone());
        session
            .state
            .storage
            .register_capture(poisoned)
            .expect("register poisoned Pokemon");
        session.state.sync_party_from_storage();
        session.state.step_events = StepEventCounters {
            poison_step_count: 3,
            ..StepEventCounters::default()
        };

        let turn = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        assert_eq!(turn.step_events, None);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("poisoned")
                .hp,
            3
        );

        let step = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("move right");
        assert_eq!(
            step.step_events.expect("step events").poison_result,
            Some(PoisonDamageResult {
                damaged_names: vec!["CHIKORITA".to_string()],
                fainted_names: Vec::new(),
            })
        );
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("poisoned")
                .hp,
            2
        );
        assert_eq!(session.state.step_events.poison_step_count, 0);

        session.overworld.player.tile = TilePosition::new(0, 0);
        session.overworld.player.facing = Direction::Right;
        session.state.step_events.poison_step_count = 3;
        session.state.storage.party.pokemon[0]
            .as_mut()
            .expect("poisoned")
            .status = Some("poison".to_string());
        let case_changed = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("move with case changed status");
        assert_eq!(
            case_changed.step_events.expect("step events").poison_result,
            None
        );
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("poisoned")
                .hp,
            2
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_sets_declared_map_music_from_modpack_asset() {
        let root = temp_repository_root("overworld-music");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_music(), report()),
            identity(),
        )
        .expect("runtime");

        let session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        assert_eq!(
            session.state.script_runtime.current_music.as_deref(),
            Some("MUSIC_ROUTE_29")
        );
        let save_path = root.join("slot.crystalsave");
        runtime
            .save_game(&save_path, session.state.clone())
            .expect("save music state");
        let loaded = runtime.load_save(&save_path).expect("load music state");
        let resumed = runtime
            .resume_overworld_session(&asset_root, loaded)
            .expect("resume music state");
        assert_eq!(
            resumed.state.script_runtime.current_music.as_deref(),
            Some("MUSIC_ROUTE_29")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_clock_updates_are_authoritative_and_saveable() {
        let root = temp_repository_root("runtime-clock");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        assert_eq!(session.state.time.time_of_day, TimeOfDay::Night);
        let manual = session
            .set_manual_clock_time(
                GameDate::new(2000, 1, 1),
                6,
                0,
                0,
                ClockTime::new(0, 20, 30, 15),
            )
            .expect("set manual time");
        assert_eq!(manual.time_of_day, TimeOfDay::Night);
        assert_eq!(manual.game_time_hours, 20);
        assert_eq!(manual.game_time_minutes, 30);

        let day = session
            .update_clock_from_datetime(GameDate::new(2000, 1, 2), 22, 45, 0)
            .expect("update clock");
        assert_eq!(day.time_of_day, TimeOfDay::Day);
        assert_eq!(day.game_time_hours, 13);
        assert_eq!(day.game_time_minutes, 15);

        let save_path = root.join("clock.crystalsave");
        runtime
            .save_game(&save_path, session.state.clone())
            .expect("save clock state");
        let loaded = runtime.load_save(&save_path).expect("load clock state");
        assert_eq!(loaded.time, session.state.time);
        assert_eq!(loaded.time.time_of_day, TimeOfDay::Day);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_triggers_exact_coord_event_for_current_scene() {
        let root = temp_repository_root("coord-event");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_coord_event(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        assert_eq!(
            session.state.scenes.map_scenes.get("RuntimeMap"),
            Some(&"SCENE_RUNTIME_ACTIVE".to_string())
        );

        let turn = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        assert_eq!(turn.snapshot.tile, TilePosition::new(1, 1));
        assert_eq!(turn.coord_event, None);

        let step = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("step onto coord event");
        let coord = step.coord_event.expect("coord event");
        assert_eq!(coord.map_name, "RuntimeMap");
        assert_eq!(coord.tile, TilePosition::new(3, 1));
        assert_eq!(coord.scene_id, "SCENE_RUNTIME_ACTIVE");
        assert_eq!(coord.script_name, "RuntimeCoordScript");

        session
            .state
            .scenes
            .map_scenes
            .insert("RuntimeMap".to_string(), "scene_runtime_active".to_string());
        session.overworld.player.tile = TilePosition::new(1, 1);
        session.overworld.player.facing = Direction::Right;
        let case_changed = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("case changed scene step");
        assert_eq!(case_changed.coord_event, None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_script_audio_commands_from_exact_modpack_entries() {
        let root = temp_repository_root("script-audio");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/sfx/SFX_TACKLE.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/cries/CRY_CHIKORITA.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                minimal_runtime_data_with_script_audio_and_map_commands(),
                report(),
            ),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let music = session
            .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 0)
            .expect("play music");
        assert!(matches!(
            music.cue,
            ScriptAudioCue::Play {
                audio_id,
                ..
            } if audio_id == "MUSIC_ROUTE_29"
        ));
        assert_eq!(
            session.state.script_runtime.current_music.as_deref(),
            Some("MUSIC_ROUTE_29")
        );

        let sfx = session
            .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 1)
            .expect("play sfx");
        assert!(matches!(
            sfx.cue,
            ScriptAudioCue::Play {
                audio_id,
                ..
            } if audio_id == "SFX_TACKLE"
        ));

        let cry = session
            .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 2)
            .expect("play cry");
        assert!(matches!(
            cry.cue,
            ScriptAudioCue::Play {
                audio_id,
                ..
            } if audio_id == "CRY_CHIKORITA"
        ));
        assert_eq!(session.state.script_runtime.audio_events.len(), 3);
        assert_ne!(music.state_checksum, cry.state_checksum);

        let missing_exact_case = session
            .apply_script_audio_command(&runtime, "RuntimeMap", "runtimeaudioscript", 0)
            .expect_err("script labels are exact");
        assert!(missing_exact_case
            .to_string()
            .contains("has no script audio command"));

        let wrong_command_case = session
            .apply_script_audio_command(&runtime, "RuntimeMap", "RuntimeAudioScript", 3)
            .expect_err("command names are exact");
        assert!(wrong_command_case.to_string().contains("PlayMusic"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_script_map_commands_and_executes_pending_warp() {
        let root = temp_repository_root("script-map");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/sfx/SFX_TACKLE.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/cries/CRY_CHIKORITA.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                minimal_runtime_data_with_script_audio_and_map_commands(),
                report(),
            ),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let map_command = session
            .apply_script_map_command(&runtime, "RuntimeMap", "RuntimeWarpScript", 0)
            .expect("apply warpfacing");
        assert!(matches!(
            map_command.action,
            ScriptMapAction::Warp {
                target_map,
                tile,
                facing: Some(Direction::Right),
                ..
            } if target_map == "RuntimeMap" && tile == TilePosition::new(1, 0)
        ));
        assert_eq!(
            session
                .state
                .script_runtime
                .pending_script_warp
                .as_ref()
                .map(|request| (&request.target_map, request.tile, request.facing)),
            Some((
                &"RuntimeMap".to_string(),
                TilePosition::new(1, 0),
                Some(Direction::Right)
            ))
        );

        let warp = session
            .execute_pending_script_warp(&runtime, &asset_root)
            .expect("execute pending warp");
        assert_eq!(warp.target_map, "RuntimeMap");
        assert_eq!(warp.tile, TilePosition::new(1, 0));
        assert_eq!(warp.facing, Some(Direction::Right));
        assert_eq!(session.snapshot().tile, TilePosition::new(1, 0));
        assert_eq!(session.snapshot().facing, Direction::Right);
        assert_eq!(session.state.script_runtime.pending_script_warp, None);
        assert_eq!(
            session.state.overworld.snapshot_identity(),
            Some((
                "RuntimeMap",
                TilePosition::new(1, 0),
                Direction::Right,
                crystal_core::world::movement::MovementMode::Normal
            ))
        );

        let no_warp = session
            .apply_script_map_command(&runtime, "RuntimeMap", "RuntimeWarpScript", 1)
            .expect("apply no-warp sentinel");
        assert!(matches!(no_warp.action, ScriptMapAction::NoWarp { .. }));
        assert_eq!(session.state.script_runtime.pending_script_warp, None);

        let missing_index = session
            .apply_script_map_command(&runtime, "RuntimeMap", "RuntimeWarpScript", 9)
            .expect_err("command index is exact");
        assert!(missing_index
            .to_string()
            .contains("has no script map command"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_script_text_variable_and_control_commands_exactly() {
        let root = temp_repository_root("script-text-variable-control");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                minimal_runtime_data_with_text_variable_and_control_commands(),
                report(),
            ),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let open = session
            .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 0)
            .expect("open text");
        assert!(matches!(open.action, ScriptTextAction::Open { .. }));
        let write = session
            .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 1)
            .expect("write text");
        assert!(matches!(
            write.action,
            ScriptTextAction::Write {
                text_label,
                ..
            } if text_label == "RuntimeGreetingText"
        ));
        let yes_no = session
            .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 2)
            .expect("yes no");
        assert!(matches!(yes_no.action, ScriptTextAction::YesNo { .. }));
        assert!(session.state.script_runtime.text_window_open);
        assert_eq!(
            session.state.script_runtime.pending_text_label.as_deref(),
            Some("RuntimeGreetingText")
        );
        assert!(session.state.script_runtime.pending_yes_no.is_some());

        let bad_text_case = session
            .apply_script_text_command(&runtime, "RuntimeMap", "RuntimeScript", 3)
            .expect_err("compiled text labels are exact");
        assert!(bad_text_case.to_string().contains("runtimegreetingtext"));

        let load_var = session
            .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 0)
            .expect("load var");
        assert!(matches!(
            load_var.outcome,
            ScriptVariableOutcome::LoadVariable {
                variable,
                value,
                ..
            } if variable == "VAR_CALLERID" && value == "PHONE_BIRDKEEPER_VANCE"
        ));
        let read_var = session
            .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 1)
            .expect("read var");
        assert!(matches!(
            read_var.outcome,
            ScriptVariableOutcome::SetAccumulator {
                value,
                ..
            } if value == "PHONE_BIRDKEEPER_VANCE"
        ));
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("PHONE_BIRDKEEPER_VANCE")
        );
        let check_time = session
            .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 2)
            .expect("check time");
        assert!(matches!(
            check_time.outcome,
            ScriptVariableOutcome::SetAccumulator {
                value,
                ..
            } if value == "TRUE"
        ));

        let branch = session
            .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 0)
            .expect("iftrue");
        assert!(matches!(
            branch.action,
            ScriptControlAction::Jump {
                target_script,
                ..
            } if target_script == "RuntimeAcceptedScript"
        ));
        assert_eq!(
            session.state.script_runtime.next_script.as_deref(),
            Some("RuntimeAcceptedScript")
        );

        let clock = session
            .update_clock_from_datetime(GameDate::new(2000, 1, 1), 12, 0, 0)
            .expect("update clock to day");
        assert_eq!(clock.time_of_day, TimeOfDay::Day);
        let day_check = session
            .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 2)
            .expect("check day time against night command");
        assert!(matches!(
            day_check.outcome,
            ScriptVariableOutcome::SetAccumulator {
                value,
                ..
            } if value == "FALSE"
        ));

        session
            .apply_script_variable_command(&runtime, "RuntimeMap", "RuntimeVariableScript", 3)
            .expect("set numeric accumulator");
        let numeric_branch = session
            .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 1)
            .expect("numeric branch");
        assert!(matches!(
            numeric_branch.action,
            ScriptControlAction::Jump {
                target_script,
                ..
            } if target_script == "RuntimeEnoughScript"
        ));

        let jumpstd = session
            .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 2)
            .expect("jumpstd");
        assert!(matches!(
            jumpstd.action,
            ScriptControlAction::Jump {
                target_script,
                standard: true,
                ..
            } if target_script == "PokecenterSignScript"
        ));
        assert_ne!(open.state_checksum, jumpstd.state_checksum);

        let missing_exact_script = session
            .apply_script_variable_command(&runtime, "RuntimeMap", "runtimevariablescript", 0)
            .expect_err("script labels are exact");
        assert!(missing_exact_script
            .to_string()
            .contains("has no script variable command"));
        let missing_control_index = session
            .apply_script_control_command(&runtime, "RuntimeMap", "RuntimeControlScript", 9)
            .expect_err("control command indexes are exact");
        assert!(missing_control_index
            .to_string()
            .contains("has no script control command"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_script_object_and_movement_commands_with_persistent_state() {
        let root = temp_repository_root("script-object-movement");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(
                minimal_runtime_data_with_object_and_movement_commands(),
                report(),
            ),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let moved = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 0)
            .expect("moveobject");
        assert_eq!(moved.outcome.object_id, "RUNTIME_NPC");
        assert_eq!((moved.outcome.x, moved.outcome.y), (Some(0), Some(0)));
        let turned = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 1)
            .expect("turnobject");
        assert_eq!(turned.outcome.object_id, "RUNTIME_NPC");
        assert_eq!(
            session.overworld.object_facings.get("RUNTIME_NPC"),
            Some(&Direction::Left)
        );

        let hidden = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 2)
            .expect("disappear");
        assert_eq!(
            hidden.outcome.event_flag.as_deref(),
            Some("EVENT_RUNTIME_NPC_HIDDEN")
        );
        assert!(session.state.flags.event_flags["EVENT_RUNTIME_NPC_HIDDEN"]);
        let shown = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 3)
            .expect("appear");
        assert_eq!(
            shown.outcome.event_flag.as_deref(),
            Some("EVENT_RUNTIME_NPC_HIDDEN")
        );
        assert!(!session.state.flags.event_flags["EVENT_RUNTIME_NPC_HIDDEN"]);

        let movement = session
            .apply_script_movement(&runtime, "RuntimeMap", "RuntimeObjectScript", 4)
            .expect("applymovement");
        assert_eq!(movement.outcome.previous_tile, TilePosition::new(0, 0));
        assert_eq!(movement.outcome.tile, TilePosition::new(1, 0));
        assert_eq!(movement.outcome.facing, Direction::Up);
        assert_eq!(movement.outcome.steps_applied, 2);
        assert_eq!(
            session
                .state
                .map_object_overrides
                .get("RuntimeMap")
                .and_then(|memory| memory.objects.get("RUNTIME_NPC"))
                .map(|object| (object.x, object.y, object.facing)),
            Some((1, 0, Some(Direction::Up)))
        );

        let follow = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 5)
            .expect("follow");
        assert_eq!(follow.outcome.object_id, "RUNTIME_GUIDE");
        assert_eq!(
            session
                .state
                .map_object_overrides
                .get("RuntimeMap")
                .and_then(|memory| memory.following.as_ref())
                .map(|following| {
                    (
                        following.leader_object_id.as_str(),
                        following.follower_object_id.as_str(),
                    )
                }),
            Some(("RUNTIME_GUIDE", "PLAYER"))
        );

        let saved_state = session.state.clone();
        let resumed = RuntimeOverworldSession::from_state(&runtime, &asset_root, saved_state)
            .expect("resume object overrides");
        let resumed_npc = resumed
            .overworld
            .objects
            .iter()
            .find(|object| object.object_identifier.as_deref() == Some("RUNTIME_NPC"))
            .expect("resumed npc");
        assert_eq!((resumed_npc.x, resumed_npc.y), (1, 0));
        assert_eq!(
            resumed.overworld.object_facings.get("RUNTIME_NPC"),
            Some(&Direction::Up)
        );
        assert_eq!(
            resumed.overworld.following,
            Some(OverworldFollowState {
                leader_object_id: "RUNTIME_GUIDE".to_string(),
                follower_object_id: "PLAYER".to_string(),
            })
        );

        let stop = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 6)
            .expect("stopfollow");
        assert_eq!(stop.outcome.object_id, "FOLLOW");
        assert_eq!(
            session
                .state
                .map_object_overrides
                .get("RuntimeMap")
                .and_then(|memory| memory.following.as_ref()),
            None
        );
        assert_ne!(moved.state_checksum, stop.state_checksum);

        let bad_movement_case = session
            .apply_script_movement(&runtime, "RuntimeMap", "RuntimeObjectScript", 7)
            .expect_err("movement labels are exact");
        assert!(bad_movement_case
            .to_string()
            .contains("has no exact movement runtimenpcmovement"));
        let missing_command = session
            .apply_script_object_mutation(&runtime, "RuntimeMap", "RuntimeObjectScript", 99)
            .expect_err("object command indexes are exact");
        assert!(missing_command
            .to_string()
            .contains("has no script object command"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_script_runtime_commands_with_explicit_inputs() {
        let root = temp_repository_root("script-runtime");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_runtime_commands(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let special = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                0,
                ScriptRuntimeInputs::default(),
            )
            .expect("special");
        assert!(matches!(
            special.outcome,
            ScriptRuntimeOutcome::EffectRecorded {
                command,
                ..
            } if command == "special"
        ));
        assert_eq!(
            session.state.script_runtime.last_special_routine.as_deref(),
            Some("FadeOutMusic")
        );

        session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                1,
                ScriptRuntimeInputs::default(),
            )
            .expect("pause");
        assert_eq!(session.state.script_runtime.pending_delays[0].frames, 15);

        let random = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                2,
                ScriptRuntimeInputs {
                    random_value: Some(7),
                    game_version: None,
                },
            )
            .expect("random");
        assert!(matches!(
            random.outcome,
            ScriptRuntimeOutcome::ScriptValueSet {
                value,
                ..
            } if value == "7"
        ));
        let random_error = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                2,
                ScriptRuntimeInputs::default(),
            )
            .expect_err("random requires explicit deterministic input");
        assert!(random_error.to_string().contains("MissingRandomInput"));

        let version = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                3,
                ScriptRuntimeInputs {
                    random_value: None,
                    game_version: Some("CRYSTAL".to_string()),
                },
            )
            .expect("checkver");
        assert!(matches!(
            version.outcome,
            ScriptRuntimeOutcome::ScriptValueSet {
                value,
                ..
            } if value == "CRYSTAL"
        ));

        session.state.script_runtime.script_value = Some("12".to_string());
        session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                4,
                ScriptRuntimeInputs::default(),
            )
            .expect("writevar");
        assert_eq!(
            session
                .state
                .script_runtime
                .variables
                .get("VAR_BLUECARDBALANCE")
                .map(String::as_str),
            Some("12")
        );
        session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                5,
                ScriptRuntimeInputs::default(),
            )
            .expect("getnum");
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("12")
        );

        let last_talked = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                6,
                ScriptRuntimeInputs::default(),
            )
            .expect("setlasttalked");
        assert_eq!(
            session.overworld.last_talked_object_identifier.as_deref(),
            Some("RUNTIME_NPC")
        );
        assert_eq!(
            session
                .state
                .map_object_overrides
                .get("RuntimeMap")
                .and_then(|memory| memory.last_talked_object_identifier.as_deref()),
            Some("RUNTIME_NPC")
        );

        session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                8,
                ScriptRuntimeInputs::default(),
            )
            .expect("showemote");
        assert_eq!(session.state.script_runtime.pending_emotes.len(), 1);
        assert_eq!(
            session.state.script_runtime.pending_emotes[0].emote,
            "EMOTE_SHOCK"
        );
        assert_ne!(special.state_checksum, last_talked.state_checksum);

        let bad_last_talked = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                7,
                ScriptRuntimeInputs::default(),
            )
            .expect_err("object ids are exact");
        assert!(bad_last_talked
            .to_string()
            .contains("missing exact object id runtime_npc"));
        let missing_index = session
            .apply_script_runtime_command(
                &runtime,
                "RuntimeMap",
                "RuntimeCommandScript",
                99,
                ScriptRuntimeInputs::default(),
            )
            .expect_err("runtime command indexes are exact");
        assert!(missing_index
            .to_string()
            .contains("has no script runtime command"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_checks_wild_encounters_after_successful_grass_step() {
        let root = temp_repository_root("overworld-encounter");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_grass_encounter(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let turn = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        assert_eq!(turn.wild_encounter, None);
        assert_eq!(session.state.rng_seed, 1);

        let step = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("step into grass");
        let roll = step
            .wild_encounter
            .clone()
            .expect("100 percent grass should produce a roll");

        assert_eq!(roll.map_name, "RuntimeMap");
        assert_eq!(roll.time, session.state.time.time_of_day);
        assert_eq!(
            roll.resolved.clone().expect("resolved").encounter.species,
            "CHIKORITA"
        );
        let battle = step.wild_battle.expect("resolved encounter starts battle");
        assert_eq!(battle.enemy_pokemon.species.id, "CHIKORITA");
        assert_eq!(battle.enemy_pokemon.moves[0].name, "TACKLE");
        assert_eq!(battle.enemy_pokemon.original_trainer_name, "WILD");
        assert_eq!(battle.encounter, roll);
        assert_eq!(
            session.state.battle,
            BattleMemory::Wild {
                battle_type: battle.battle_type.clone(),
                map_name: "RuntimeMap".to_string(),
                enemy_pokemon: battle.enemy_pokemon.clone(),
                enemy_party: battle.enemy_party.clone(),
            }
        );
        assert_ne!(session.state.rng_seed, 1);
        assert_eq!(battle.rng_seed_after, session.state.rng_seed);
        let saved_battle = session.state.battle.clone();
        let save_path = root.join("battle.crystalsave");
        runtime
            .save_game(&save_path, session.state.clone())
            .expect("save active battle");
        let loaded = runtime.load_save(&save_path).expect("load active battle");
        assert_eq!(loaded.battle, saved_battle);
        let resumed = runtime
            .resume_overworld_session(&asset_root, loaded)
            .expect("resume active battle");
        assert_eq!(resumed.state.battle, saved_battle);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_repel_item_blocks_lower_level_wild_encounter_after_real_step() {
        let root = temp_repository_root("overworld-repel-lower");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_grass_encounter();
        let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
        repel.effect = "MOD_REPEL".to_string();
        repel.field_menu = "ITEMMENU_CLOSE".to_string();
        repel.field_usable = true;
        repel.consumable = true;
        repel.repel_steps = Some(100);
        data.items.insert("REPEL".to_string(), repel);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REPEL"], 1)
            .expect("add Repel");

        let item_use = session
            .use_bag_repel_in_field(&runtime, "REPEL")
            .expect("use Repel");
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.repel_steps_before, 0);
        assert_eq!(item_use.repel_steps_after, 100);
        assert_eq!(item_use.active_repel_item_after, Some("REPEL".to_string()));
        assert!(!session.state.bag.has_item(&runtime.data.items["REPEL"]));

        let turn = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        assert_eq!(turn.wild_encounter, None);
        assert_eq!(session.state.repel_steps_remaining, 100);

        let step = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("step into grass");
        let roll = step
            .wild_encounter
            .clone()
            .expect("repelled grass still records the roll");
        assert_eq!(session.state.repel_steps_remaining, 99);
        assert_eq!(session.state.active_repel_item, Some("REPEL".to_string()));
        assert_eq!(roll.repelled_by, Some("REPEL".to_string()));
        assert_eq!(roll.resolved, None);
        assert_eq!(step.wild_battle, None);
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_ne!(session.state.rng_seed, 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_repel_does_not_block_same_or_higher_level_wild_encounter() {
        let root = temp_repository_root("overworld-repel-higher");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_grass_encounter();
        let encounter = WildEncounter {
            level: 8,
            species: "CHIKORITA".to_string(),
        };
        let grass_slots = vec![encounter; 7];
        let wild = data
            .wild_encounters
            .get_mut("RuntimeMap")
            .expect("runtime wild encounters");
        wild.grass = Some(WildEncounterTable {
            morning: grass_slots.clone(),
            day: grass_slots.clone(),
            night: grass_slots,
        });
        let mut repel = runtime_item("REPEL", item_pocket("ITEM"));
        repel.effect = "REPEL".to_string();
        repel.field_menu = "ITEMMENU_CLOSE".to_string();
        repel.field_usable = true;
        repel.consumable = true;
        repel.repel_steps = Some(100);
        data.items.insert("REPEL".to_string(), repel);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REPEL"], 1)
            .expect("add Repel");
        session
            .use_bag_repel_in_field(&runtime, "REPEL")
            .expect("use Repel");
        session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");

        let step = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("step into grass");
        let roll = step.wild_encounter.clone().expect("encounter roll");

        assert_eq!(session.state.repel_steps_remaining, 99);
        assert_eq!(roll.repelled_by, None);
        assert_eq!(roll.resolved.clone().expect("resolved").encounter.level, 8);
        assert!(step.wild_battle.is_some());
        assert!(matches!(session.state.battle, BattleMemory::Wild { .. }));
        let _ = std::fs::remove_dir_all(root);
    }

    fn runtime_data_with_escape_rope_maps() -> GameDataSet {
        let mut data = minimal_runtime_data_with_music();
        let source_warp = WarpEvent {
            index: 1,
            x: 2,
            y: 0,
            target_map_constant: "RUNTIME_CAVE".to_string(),
            target_map: "RuntimeCave".to_string(),
            target_warp_id: 1,
        };
        let cave_warp = WarpEvent {
            index: 1,
            x: 0,
            y: 0,
            target_map_constant: "RUNTIME_MAP".to_string(),
            target_map: "RuntimeMap".to_string(),
            target_warp_id: 1,
        };
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .events
            .warps = vec![source_warp.clone()];
        let mut cave = runtime_map();
        cave.id = "RuntimeCave".to_string();
        cave.attributes.environment = Some("cave".to_string());
        cave.attributes.map_constant = Some("RUNTIME_CAVE".to_string());
        cave.events.warps = vec![cave_warp];
        data.maps.insert("RuntimeCave".to_string(), cave.clone());
        data.map_attributes.insert(
            "RuntimeMap".to_string(),
            data.maps
                .get("RuntimeMap")
                .expect("runtime map")
                .attributes
                .clone(),
        );
        data.map_attributes
            .insert("RuntimeCave".to_string(), cave.attributes.clone());
        data.runtime_map_metadata.insert(
            "RUNTIME_MAP".to_string(),
            runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "ROUTE"),
        );
        data.runtime_map_metadata.insert(
            "RUNTIME_CAVE".to_string(),
            runtime_map_metadata("RUNTIME_CAVE", "RuntimeCave", 1, 2, "CAVE"),
        );
        data.runtime_spawn_points.insert(
            "1".to_string(),
            RuntimeSpawnPoint {
                identifier: 1,
                map_constant: "RUNTIME_CAVE".to_string(),
                map_name: "RuntimeCave".to_string(),
                group_id: 1,
                map_id: 2,
                tile_x: 0,
                tile_y: 0,
                group_name: "RUNTIME".to_string(),
                metatile_x: 0,
                metatile_y: 0,
                subtile_x: 0,
                subtile_y: 0,
            },
        );
        let mut escape_rope = runtime_item("ESCAPE_ROPE", item_pocket("ITEM"));
        escape_rope.effect = "ESCAPE_ROPE".to_string();
        escape_rope.field_menu = "ITEMMENU_CLOSE".to_string();
        escape_rope.field_usable = true;
        escape_rope.consumable = true;
        escape_rope.escape_rope_mode = Some("DIG_WARP".to_string());
        data.items.insert("ESCAPE_ROPE".to_string(), escape_rope);
        data
    }

    #[test]
    fn runtime_escape_rope_uses_saved_dig_warp_without_fallback_destination() {
        let root = temp_repository_root("escape-rope");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = runtime_data_with_escape_rope_maps();
        data.field_moves.escape_rope.escape_rope_mode = "MOD_WARP".to_string();
        {
            let escape_rope = data.items.get_mut("ESCAPE_ROPE").expect("escape rope");
            escape_rope.effect = "MOD_ESCAPE".to_string();
            escape_rope.escape_rope_mode = Some("MOD_WARP".to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        session
            .state
            .bag
            .add_item(&runtime.data.items["ESCAPE_ROPE"], 1)
            .expect("add Escape Rope");

        session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        let warp_step = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("step onto warp");
        assert_eq!(
            warp_step
                .warp
                .expect("warp transition")
                .destination
                .map_name,
            "RuntimeCave"
        );
        assert_eq!(session.overworld.map.name, "RuntimeCave");
        assert_eq!(
            session.state.dig_warp_map_name,
            Some("RuntimeMap".to_string())
        );
        assert_eq!(session.state.dig_warp_index, Some(1));

        let escape = session
            .use_bag_escape_rope_in_field(&runtime, &asset_root, "ESCAPE_ROPE")
            .expect("Escape Rope uses saved dig warp");

        assert!(escape.item_use.consumed);
        assert_eq!(escape.source_map, "RuntimeCave");
        assert_eq!(escape.destination_map, "RuntimeMap");
        assert_eq!(escape.destination_warp_index, 1);
        assert_eq!(escape.destination_tile, TilePosition::new(2, 0));
        assert_eq!(session.overworld.map.name, "RuntimeMap");
        assert_eq!(session.overworld.player.tile, TilePosition::new(2, 0));
        assert!(!session
            .state
            .bag
            .has_item(&runtime.data.items["ESCAPE_ROPE"]));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_escape_rope_rejects_missing_dig_warp_without_consumption() {
        let root = temp_repository_root("escape-rope-missing-dig");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = runtime_data_with_escape_rope_maps();
        data.field_moves.dig.move_id = "TELEPORT".to_string();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 1)
            .expect("overworld session");
        session
            .state
            .bag
            .add_item(&runtime.data.items["ESCAPE_ROPE"], 1)
            .expect("add Escape Rope");
        let before = session.state.clone();

        let error = session
            .use_bag_escape_rope_in_field(&runtime, &asset_root, "ESCAPE_ROPE")
            .expect_err("missing dig warp rejected");

        assert!(error.to_string().contains("has no saved dig warp map"));
        assert_eq!(session.state, before);
        assert!(session
            .state
            .bag
            .has_item(&runtime.data.items["ESCAPE_ROPE"]));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_dig_field_move_uses_saved_dig_warp_without_fallback_destination() {
        let root = temp_repository_root("field-move-dig");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = runtime_data_with_escape_rope_maps();
        data.field_moves.dig.move_id = "TELEPORT".to_string();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TELEPORT".to_string(),
            current_pp: 10,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("step onto warp");
        assert_eq!(session.overworld.map.name, "RuntimeCave");
        assert_eq!(
            session.state.dig_warp_map_name,
            Some("RuntimeMap".to_string())
        );
        assert_eq!(session.state.dig_warp_index, Some(1));

        let dig = session
            .use_dig_field_move(&runtime, &asset_root, 0)
            .expect("DIG uses saved dig warp");

        assert_eq!(dig.actor_party_index, 0);
        assert_eq!(dig.actor_species, "CHIKORITA");
        assert_eq!(dig.source_map, "RuntimeCave");
        assert_eq!(dig.destination_map, "RuntimeMap");
        assert_eq!(dig.destination_warp_index, 1);
        assert_eq!(dig.destination_tile, TilePosition::new(2, 0));
        assert_eq!(session.overworld.map.name, "RuntimeMap");
        assert_eq!(session.overworld.player.tile, TilePosition::new(2, 0));
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_dig_field_move_rejects_missing_dig_warp_without_mutation() {
        let root = temp_repository_root("field-move-dig-missing-warp");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(runtime_data_with_escape_rope_maps(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 1)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "DIG".to_string(),
            current_pp: 10,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_dig_field_move(&runtime, &asset_root, 0)
            .expect_err("missing dig warp rejected");

        assert!(error
            .to_string()
            .contains("DIG field move has no saved dig warp map"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_casts_fishing_rod_from_current_map_compiled_group() {
        let root = temp_repository_root("fishing-battle");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_fishing(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let cast = session
            .cast_fishing_rod(&runtime, ROD_GOOD)
            .expect("cast good rod");

        assert_eq!(cast.session.group.as_deref(), Some("FISHGROUP_RUNTIME"));
        assert_eq!(cast.bite, Some(true));
        assert_eq!(session.state.fishing.rod_state, FishingRodState::Battle);
        assert_eq!(session.state.fishing.rod_index, Some(1));
        let battle = cast.wild_battle.expect("fishing starts battle");
        assert_eq!(battle.enemy_pokemon.species.id, "CHIKORITA");
        assert_eq!(battle.enemy_pokemon.level, 9);
        assert_eq!(battle.encounter.surface, EncounterSurface::Water);
        assert_eq!(session.state.battle, BattleMemory::from(&battle));
        assert!(session.state.pokedex.has_seen("CHIKORITA"));
        assert_eq!(session.state.battle_active_party_index, None);

        let before_bad_rod_state = session.state.clone();
        let before_bad_rod_snapshot = session.overworld.snapshot();
        let bad_rod = session
            .cast_fishing_rod(&runtime, "good_rod")
            .expect_err("rod ids are exact");
        assert!(bad_rod
            .to_string()
            .contains("validate fishing rod good_rod before cast"));
        assert_eq!(session.state, before_bad_rod_state);
        assert_eq!(session.overworld.snapshot(), before_bad_rod_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_uses_bag_fishing_rod_item_from_definitive_item_id() {
        let root = temp_repository_root("fishing-bag-item");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_fishing();
        let mut good_rod = runtime_item("GOOD_ROD", item_pocket("KEY_ITEM"));
        good_rod.effect = "MOD_GOOD_ROD".to_string();
        good_rod.field_menu = "ITEMMENU_CLOSE".to_string();
        good_rod.field_usable = true;
        data.items.insert("GOOD_ROD".to_string(), good_rod);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        session
            .state
            .bag
            .add_item(&runtime.data.items["GOOD_ROD"], 1)
            .expect("add good rod");

        let use_rod = session
            .use_bag_fishing_rod_in_field(&runtime, "GOOD_ROD")
            .expect("use good rod item");

        assert_eq!(use_rod.item_use.item_id, "GOOD_ROD");
        assert!(!use_rod.item_use.consumed);
        assert_eq!(use_rod.rod, ROD_GOOD);
        assert_eq!(
            use_rod.cast.session.group.as_deref(),
            Some("FISHGROUP_RUNTIME")
        );
        assert_eq!(use_rod.cast.bite, Some(true));
        assert_eq!(session.state.fishing.rod_state, FishingRodState::Battle);
        assert_eq!(session.state.fishing.rod_index, Some(1));
        assert_eq!(
            session.state.battle,
            BattleMemory::from(&use_rod.cast.wild_battle.expect("battle"))
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["GOOD_ROD"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        assert_eq!(
            session.state.script_runtime.item_use_events[0].item_id,
            "GOOD_ROD"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bag_fishing_rod_rejects_missing_or_case_changed_item_id_without_mutation() {
        let root = temp_repository_root("fishing-bag-item-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_fishing();
        let mut missing_rod = runtime_item("GOOD_ROD", item_pocket("KEY_ITEM"));
        missing_rod.effect = "MOD_GOOD_ROD".to_string();
        missing_rod.field_menu = "ITEMMENU_CLOSE".to_string();
        missing_rod.field_usable = true;
        let mut bad_case_rod = runtime_item("BAD_CASE_ROD", item_pocket("KEY_ITEM"));
        bad_case_rod.effect = "good_rod".to_string();
        bad_case_rod.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_case_rod.field_usable = true;
        data.items.insert("GOOD_ROD".to_string(), missing_rod);
        data.items.insert("BAD_CASE_ROD".to_string(), bad_case_rod);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let before_missing = session.state.clone();
        let missing = session
            .use_bag_fishing_rod_in_field(&runtime, "GOOD_ROD")
            .expect_err("missing rod rejects before cast");
        assert!(missing.to_string().contains("not in the bag"));
        assert_eq!(session.state, before_missing);

        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_CASE_ROD"], 1)
            .expect("add bad case rod");
        let before_bad_case = session.state.clone();
        let bad_case = session
            .use_bag_fishing_rod_in_field(&runtime, "BAD_CASE_ROD")
            .expect_err("case changed rod item id rejects");
        assert!(bad_case
            .to_string()
            .contains("not declared by exact fishing rod item rules"));
        assert_eq!(session.state, before_bad_case);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fishing_without_map_group_does_not_fabricate_encounters() {
        let root = temp_repository_root("fishing-no-group");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let cast = session
            .cast_fishing_rod(&runtime, ROD_GOOD)
            .expect("cast good rod without group");

        assert_eq!(cast.session.group, None);
        assert_eq!(cast.bite, Some(false));
        assert_eq!(cast.wild_battle, None);
        assert_eq!(session.state.fishing.result, 0);
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_starts_scripted_wild_battle_from_exact_map_script_command() {
        let root = temp_repository_root("scripted-wild-battle");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let start = session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        assert_eq!(start.species, "CHIKORITA");
        assert_eq!(start.level, 6);
        assert_eq!(
            session
                .state
                .flags
                .is_event_flag_set("EVENT_RUNTIME_WILD_READY"),
            Ok(true)
        );
        assert_eq!(session.state.battle, BattleMemory::from(&start));
        assert!(session.state.pokedex.has_seen("CHIKORITA"));
        assert_eq!(session.state.battle_active_party_index, None);
        let completion = session
            .complete_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle completes");
        assert!(completion.continued_after_battle);
        let effects = completion.effects.expect("wild battle effects apply");
        assert_eq!(
            effects.event_flags_set,
            vec!["EVENT_RUNTIME_WILD_DONE".to_string()]
        );
        assert_eq!(
            effects.script_flags_set,
            vec!["ENGINE_RUNTIME_WILD_DONE".to_string()]
        );
        assert_eq!(
            effects.disappeared_objects[0].object_identifier,
            "RUNTIME_STATIC_MON"
        );
        assert_eq!(
            session
                .state
                .flags
                .is_event_flag_set("EVENT_RUNTIME_STATIC_MON_HIDDEN"),
            Ok(true)
        );
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);

        let error = session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "runtimewildscript", 4)
            .expect_err("script names are exact");
        assert!(error
            .to_string()
            .contains("has no scripted wild battle at runtimewildscript:4"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_resolves_active_battle_turn_into_authoritative_state() {
        let root = temp_repository_root("battle-turn");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = runtime
            .data
            .static_wild_battle_start(
                StaticWildBattleRequest::new("CHIKORITA", 8),
                &mut Random::new(7),
            )
            .expect("player pokemon")
            .enemy_pokemon;
        player.original_trainer_name = "PLAYER".to_string();
        session
            .state
            .storage
            .register_capture(player.clone())
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        assert_eq!(session.state.battle_active_party_index, Some(0));
        assert!(session.state.pokedex.has_seen("CHIKORITA"));
        let enemy_before = match &session.state.battle {
            BattleMemory::StaticWild { enemy_pokemon, .. } => enemy_pokemon.clone(),
            other => panic!("expected static wild battle, got {other:?}"),
        };
        let before_checksum = game_state_checksum(&session.state).expect("checksum before turn");

        let turn = session
            .resolve_active_battle_turn(
                &runtime,
                BattleAction::Move { slot: 0 },
                BattleAction::Move { slot: 0 },
            )
            .expect("resolve turn");

        assert_eq!(turn.outcome.state.turn, 1);
        assert_eq!(turn.outcome.state.rng_seed_after, session.state.rng_seed);
        assert!(turn.outcome.events.iter().any(|event| matches!(
            event,
            crystal_core::battle::turn::BattleEvent::Damage { .. }
        )));
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .moves[0]
                .current_pp,
            34
        );
        let enemy_after = match &session.state.battle {
            BattleMemory::StaticWild {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                assert_eq!(enemy_party[0], *enemy_pokemon);
                enemy_pokemon.clone()
            }
            other => panic!("expected static wild battle, got {other:?}"),
        };
        assert!(enemy_after.hp < enemy_before.hp);
        assert_ne!(turn.state_checksum, before_checksum);

        runtime
            .save_game(root.join("battle-turn.crystalsave"), session.state.clone())
            .expect("save battle turn");
        let loaded = runtime
            .load_save(root.join("battle-turn.crystalsave"))
            .expect("load battle turn");
        assert_eq!(loaded.battle, session.state.battle);
        assert_eq!(loaded.battle_active_party_index, Some(0));
        assert_eq!(
            loaded.storage.party.pokemon,
            session.state.storage.party.pokemon
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_turn_uses_compiled_item_payload() {
        let root = temp_repository_root("battle-turn-item");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.parameter = 20;
        potion.consumable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = runtime
            .data
            .static_wild_battle_start(
                StaticWildBattleRequest::new("CHIKORITA", 8),
                &mut Random::new(7),
            )
            .expect("player pokemon")
            .enemy_pokemon;
        player.max_hp = 40;
        player.hp = 10;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        let hp_before = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player")
            .hp;

        let turn = session
            .resolve_active_battle_turn(
                &runtime,
                BattleAction::Item {
                    item_id: "POTION".to_string(),
                },
                BattleAction::Move { slot: 0 },
            )
            .expect("item turn resolves");

        assert_eq!(hp_before, 10);
        assert!(turn.outcome.events.iter().any(|event| matches!(
            event,
            crystal_core::battle::turn::BattleEvent::BattleItemEffect {
                side: crystal_core::battle::turn::BattleSide::Player,
                outcome
            } if outcome.item_id == "POTION"
                && outcome.hp_before == 10
                && outcome.hp_after == 30
                && !outcome.consumed
        )));
        assert!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .hp
                > hp_before
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_switch_updates_authoritative_active_party_slot() {
        let root = temp_repository_root("battle-turn-switch");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        for level in [8, 9] {
            let mut player = runtime
                .data
                .static_wild_battle_start(
                    StaticWildBattleRequest::new("CHIKORITA", level),
                    &mut Random::new(level as u32),
                )
                .expect("player pokemon")
                .enemy_pokemon;
            player.original_trainer_name = "PLAYER".to_string();
            session
                .state
                .storage
                .register_capture(player)
                .expect("register player");
        }
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        assert_eq!(session.state.battle_active_party_index, Some(0));
        let slot0_before = session.state.storage.party.pokemon[0].clone();
        let slot1_before_pp = session.state.storage.party.pokemon[1]
            .as_ref()
            .expect("second party mon")
            .moves[0]
            .current_pp;

        let turn = session
            .resolve_active_battle_turn(
                &runtime,
                BattleAction::Switch { party_index: 1 },
                BattleAction::Move { slot: 0 },
            )
            .expect("switch resolves");

        assert_eq!(session.state.battle_active_party_index, Some(1));
        assert_eq!(session.state.storage.party.pokemon[0], slot0_before);
        assert!(turn.outcome.events.iter().any(|event| matches!(
            event,
            crystal_core::battle::turn::BattleEvent::MoveSelected {
                side: crystal_core::battle::turn::BattleSide::Enemy,
                ..
            }
        )));
        assert_eq!(
            session.state.storage.party.pokemon[1]
                .as_ref()
                .expect("second party mon")
                .moves[0]
                .current_pp,
            slot1_before_pp
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_turn_rejects_missing_exact_move_data_without_state_mutation() {
        let root = temp_repository_root("battle-turn-missing-move");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves.push(crystal_core::models::LearnedMove {
            name: "tackle".to_string(),
            current_pp: 35,
            pp_ups: 0,
        });
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.battle = BattleMemory::StaticWild {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            species: "CHIKORITA".to_string(),
            level: 6,
            source_script: "RuntimeWildScript".to_string(),
            enemy_pokemon: {
                let mut enemy = Pokemon::new_for_tests(runtime_species(), 6, Dv::default());
                enemy.moves.push(crystal_core::models::LearnedMove {
                    name: "TACKLE".to_string(),
                    current_pp: 35,
                    pp_ups: 0,
                });
                enemy
            },
            enemy_party: Vec::new(),
        };
        session.state.battle_active_party_index = Some(0);
        let before = session.state.clone();
        let error = session
            .resolve_active_battle_turn(
                &runtime,
                BattleAction::Move { slot: 0 },
                BattleAction::Move { slot: 0 },
            )
            .expect_err("missing move data rejected");
        assert!(error.to_string().contains("MissingMoveData"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_turn_rejects_invalid_active_party_index_without_state_mutation() {
        let root = temp_repository_root("battle-turn-invalid-active");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        let before = session.state.clone();

        let error = session
            .resolve_active_battle_turn(
                &runtime,
                BattleAction::Switch { party_index: 2 },
                BattleAction::Move { slot: 0 },
            )
            .expect_err("empty party slot rejected");

        assert!(error
            .to_string()
            .contains("active battle party index 2 has no Pokemon"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_turn_rejects_unknown_item_and_run_actions_without_state_mutation() {
        let root = temp_repository_root("battle-turn-rejected-actions");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        let before_item = session.state.clone();

        let item_error = session
            .resolve_active_battle_turn(
                &runtime,
                BattleAction::Item {
                    item_id: "POTION".to_string(),
                },
                BattleAction::Move { slot: 0 },
            )
            .expect_err("unknown battle turn item rejects");

        assert!(item_error.to_string().contains("UnknownItem"));
        assert_eq!(session.state, before_item);
        let run_error = session
            .resolve_active_battle_turn(&runtime, BattleAction::Run, BattleAction::Move { slot: 0 })
            .expect_err("run uses explicit escape runtime, not turn noop");
        assert!(run_error.to_string().contains("UnsupportedRunAction"));
        assert_eq!(session.state, before_item);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_command_routes_run_through_wild_escape() {
        let root = temp_repository_root("battle-command-run");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fast_species = runtime_species();
        fast_species.base_stats.speed = 999;
        let player = Pokemon::new_for_tests(fast_species, 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        session.state.battle_escape_attempts = 2;

        let command = session
            .resolve_active_battle_command(
                &runtime,
                BattleAction::Run,
                BattleAction::Move { slot: 0 },
            )
            .expect("run command resolves");

        let RuntimeBattleCommand::Escape(escape) = command else {
            panic!("run command should route to escape");
        };
        assert!(escape.outcome.escaped);
        assert_eq!(escape.outcome.attempts_before, 2);
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_escape_attempts, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_command_routes_non_run_to_turn_resolution() {
        let root = temp_repository_root("battle-command-turn");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = runtime
            .data
            .static_wild_battle_start(
                StaticWildBattleRequest::new("CHIKORITA", 8),
                &mut Random::new(7),
            )
            .expect("player pokemon")
            .enemy_pokemon;
        player.original_trainer_name = "PLAYER".to_string();
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");

        let command = session
            .resolve_active_battle_command(
                &runtime,
                BattleAction::Move { slot: 0 },
                BattleAction::Move { slot: 0 },
            )
            .expect("move command resolves");

        let RuntimeBattleCommand::Turn(turn) = command else {
            panic!("move command should route to turn resolution");
        };
        assert_eq!(turn.outcome.state.turn, 1);
        assert!(turn.outcome.events.iter().any(|event| matches!(
            event,
            crystal_core::battle::turn::BattleEvent::Damage { .. }
        )));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_attempts_wild_escape_with_saved_attempt_counter() {
        let root = temp_repository_root("battle-escape-failure");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut slow_species = runtime_species();
        slow_species.base_stats.speed = 1;
        let player = Pokemon::new_for_tests(slow_species, 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        match &mut session.state.battle {
            BattleMemory::StaticWild {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                enemy_pokemon.species.base_stats.speed = 255;
                enemy_party[0].species.base_stats.speed = 255;
            }
            other => panic!("expected static wild battle, got {other:?}"),
        }
        let before = game_state_checksum(&session.state).expect("checksum before escape");

        let escape = session
            .attempt_escape_active_wild_battle(&runtime)
            .expect("escape attempt resolves");

        assert!(!escape.outcome.escaped);
        assert_eq!(escape.outcome.attempts_before, 0);
        assert_eq!(escape.outcome.attempts_after, 1);
        assert_eq!(escape.outcome.roll, Some(196));
        assert_eq!(session.state.battle_escape_attempts, 1);
        assert!(matches!(
            session.state.battle,
            BattleMemory::StaticWild { .. }
        ));
        assert_ne!(escape.state_checksum, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_successful_wild_escape_clears_active_battle_state() {
        let root = temp_repository_root("battle-escape-success");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fast_species = runtime_species();
        fast_species.base_stats.speed = 999;
        let player = Pokemon::new_for_tests(fast_species, 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        session.state.battle_escape_attempts = 3;

        let escape = session
            .attempt_escape_active_wild_battle(&runtime)
            .expect("escape attempt resolves");

        assert!(escape.outcome.escaped);
        assert_eq!(escape.outcome.roll, None);
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);
        assert_eq!(session.state.battle_active_enemy_party_index, None);
        assert!(session.state.battle_rewarded_enemy_party_indices.is_empty());
        assert_eq!(session.state.battle_escape_attempts, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_poke_doll_consumes_item_and_clears_active_wild_battle_state() {
        let root = temp_repository_root("battle-poke-doll");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut poke_doll = runtime_item("POKE_DOLL", item_pocket("ITEM"));
        poke_doll.effect = "MOD_DOLL".to_string();
        poke_doll.battle_menu = "ITEMMENU_CLOSE".to_string();
        poke_doll.battle_usable = true;
        poke_doll.consumable = true;
        poke_doll.battle_escape_mode = Some("WILD_BATTLE".to_string());
        data.items.insert("POKE_DOLL".to_string(), poke_doll);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POKE_DOLL"], 1)
            .expect("add Poke Doll");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        session.state.battle_escape_attempts = 2;
        session.state.battle_player_stat_drop_guard_turns = 5;

        let item_escape = session
            .use_bag_item_to_escape_active_wild_battle(&runtime, "POKE_DOLL")
            .expect("Poke Doll escapes wild battle");

        assert!(item_escape.item_use.consumed);
        assert_eq!(item_escape.item_use.item_id, "POKE_DOLL");
        assert_eq!(item_escape.battle_escape_mode, "WILD_BATTLE");
        assert!(item_escape.escaped);
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);
        assert_eq!(session.state.battle_active_enemy_party_index, None);
        assert!(session.state.battle_rewarded_enemy_party_indices.is_empty());
        assert_eq!(session.state.battle_escape_attempts, 0);
        assert_eq!(session.state.battle_player_stat_drop_guard_turns, 0);
        assert!(!session.state.bag.has_item(&runtime.data.items["POKE_DOLL"]));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_rejects_poke_doll_in_trainer_battle_without_consumption() {
        let root = temp_repository_root("battle-poke-doll-trainer");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut poke_doll = runtime_item("POKE_DOLL", item_pocket("ITEM"));
        poke_doll.effect = "POKE_DOLL".to_string();
        poke_doll.battle_menu = "ITEMMENU_CLOSE".to_string();
        poke_doll.battle_usable = true;
        poke_doll.consumable = true;
        poke_doll.battle_escape_mode = Some("WILD_BATTLE".to_string());
        data.items.insert("POKE_DOLL".to_string(), poke_doll);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POKE_DOLL"], 1)
            .expect("add Poke Doll");
        session
            .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
            .expect("start trainer battle");
        let before = session.state.clone();

        let error = session
            .use_bag_item_to_escape_active_wild_battle(&runtime, "POKE_DOLL")
            .expect_err("Poke Doll cannot escape trainer battle");

        assert!(error
            .to_string()
            .contains("cannot use battle escape item POKE_DOLL in trainer battle"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_guard_spec_consumes_item_and_sets_stat_drop_guard_turns() {
        let root = temp_repository_root("battle-guard-spec");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut guard_spec = runtime_item("GUARD_SPEC", item_pocket("ITEM"));
        guard_spec.effect = "MOD_GUARD".to_string();
        guard_spec.battle_menu = "ITEMMENU_CLOSE".to_string();
        guard_spec.battle_usable = true;
        guard_spec.consumable = true;
        guard_spec.battle_stat_drop_guard = Some(true);
        guard_spec.battle_stat_drop_guard_turns = Some(5);
        data.items.insert("GUARD_SPEC".to_string(), guard_spec);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["GUARD_SPEC"], 1)
            .expect("add Guard Spec");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");

        let guard = session
            .use_bag_guard_spec_in_active_battle(&runtime, "GUARD_SPEC")
            .expect("Guard Spec applies");

        assert!(guard.item_use.consumed);
        assert_eq!(guard.item_use.item_id, "GUARD_SPEC");
        assert_eq!(guard.stat_drop_guard_turns_before, 0);
        assert_eq!(guard.stat_drop_guard_turns_after, 5);
        assert_eq!(session.state.battle_player_stat_drop_guard_turns, 5);
        assert!(!session
            .state
            .bag
            .has_item(&runtime.data.items["GUARD_SPEC"]));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_rejects_escape_from_trainer_battle_without_state_mutation() {
        let root = temp_repository_root("battle-escape-trainer");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
            .expect("start trainer battle");
        let before = session.state.clone();

        let error = session
            .attempt_escape_active_wild_battle(&runtime)
            .expect_err("trainer battles cannot be escaped");

        assert!(error
            .to_string()
            .contains("cannot escape from trainer battle"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_claims_wild_battle_rewards_into_authoritative_party_state() {
        let root = temp_repository_root("battle-rewards");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_battle_rewards(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = runtime
            .data
            .static_wild_battle_start(
                StaticWildBattleRequest::new("CHIKORITA", 15),
                &mut Random::new(7),
            )
            .expect("player pokemon")
            .enemy_pokemon;
        player.original_trainer_name = "PLAYER".to_string();
        player.experience =
            calculate_experience(&runtime.data.growth_rates, "GROWTH_MEDIUM_FAST", 16).unwrap() - 1;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        assert_eq!(session.state.battle_active_party_index, Some(0));
        match &mut session.state.battle {
            BattleMemory::StaticWild {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                enemy_pokemon.hp = 0;
                enemy_party[0].hp = 0;
            }
            other => panic!("expected static wild battle, got {other:?}"),
        }

        let rewards = session
            .claim_active_wild_battle_rewards(&runtime)
            .expect("claim battle rewards");

        assert_eq!(rewards.outcome.level_before, 15);
        assert_eq!(rewards.outcome.level_after, 16);
        assert_eq!(
            rewards.outcome.learned_moves,
            vec!["RAZOR_LEAF".to_string()]
        );
        assert_eq!(
            rewards.outcome.evolution.target_species,
            Some("BAYLEEF".to_string())
        );
        let lead = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("lead");
        assert_eq!(lead.species.id, "BAYLEEF");
        assert!(lead.moves.iter().any(|known| known.name == "RAZOR_LEAF"));
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);
        assert_eq!(
            session.state.party.pokemon[0]
                .as_ref()
                .map(|entry| entry.species.as_str()),
            Some("BAYLEEF")
        );
        runtime
            .save_game(
                root.join("battle-rewards.crystalsave"),
                session.state.clone(),
            )
            .expect("save battle rewards");
        let loaded = runtime
            .load_save(root.join("battle-rewards.crystalsave"))
            .expect("load battle rewards");
        assert_eq!(
            loaded.storage.party.pokemon,
            session.state.storage.party.pokemon
        );
        assert_eq!(loaded.battle, BattleMemory::Inactive);
        assert_eq!(loaded.battle_active_party_index, None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_rewards_reject_non_fainted_enemy() {
        let root = temp_repository_root("battle-rewards-not-fainted");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_battle_rewards(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = runtime
            .data
            .static_wild_battle_start(
                StaticWildBattleRequest::new("CHIKORITA", 15),
                &mut Random::new(7),
            )
            .expect("player pokemon")
            .enemy_pokemon;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("start battle");
        assert_eq!(session.state.battle_active_party_index, Some(0));
        let before = session.state.clone();

        let error = session
            .claim_active_wild_battle_rewards(&runtime)
            .expect_err("enemy must be fainted");

        assert!(error.to_string().contains("DefeatedPokemonNotFainted"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_capture_completion_stores_active_wild_pokemon_in_authoritative_state() {
        let root = temp_repository_root("scripted-wild-capture");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.items
            .get_mut("MASTER_BALL")
            .expect("master ball")
            .effect = "MOD_MASTER_BALL".to_string();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
        session
            .state
            .storage
            .register_capture(player)
            .expect("player party Pokemon");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["MASTER_BALL"], 1)
            .expect("add master ball");
        let start = session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        assert_eq!(session.state.battle_active_party_index, Some(0));
        assert!(session.state.pokedex.has_seen("CHIKORITA"));
        let attempt = session
            .throw_ball_at_active_battle(&runtime, "MASTER_BALL")
            .expect("throw master ball");
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["MASTER_BALL"]),
            0
        );
        let capture = attempt.outcome.expect("ball was available");
        assert!(capture.caught);

        let captured = session
            .complete_active_wild_capture(&capture)
            .expect("captured Pokemon stores");

        let stored = captured.stored.expect("successful capture stores");
        assert_eq!(stored.location, CaptureStorageLocation::Party { slot: 1 });
        assert_eq!(stored.pokemon.species.id, "CHIKORITA");
        assert_eq!(stored.pokemon.level, start.level);
        assert_eq!(session.state.battle_result & (1 << 6), 1 << 6);
        assert_eq!(
            session.state.storage.party.pokemon[1]
                .as_ref()
                .expect("stored party mon")
                .species
                .id,
            "CHIKORITA"
        );
        assert_eq!(
            session.state.party.pokemon[1],
            Some(crystal_core::state::PartyPokemonRef {
                species: "CHIKORITA".to_string(),
                level: 6,
            })
        );
        assert!(session.state.pokedex.has_caught("CHIKORITA"));
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_capture_item_rejects_undeclared_ball_rule_without_mutation() {
        let root = temp_repository_root("scripted-wild-capture-bad-ball");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut bad_ball = runtime_ball_item("BAD_BALL");
        bad_ball.effect = "MOD_BALL".to_string();
        data.items.insert("BAD_BALL".to_string(), bad_ball);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
        session
            .state
            .storage
            .register_capture(player)
            .expect("player party Pokemon");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_BALL"], 1)
            .expect("add bad ball");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before_state = session.state.clone();

        let error = session
            .throw_ball_at_active_battle(&runtime, "BAD_BALL")
            .expect_err("undeclared capture ball is rejected");

        assert!(
            error
                .to_string()
                .contains("battle capture item BAD_BALL is not declared by exact capture rules"),
            "{error}"
        );
        assert_eq!(session.state, before_state);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["BAD_BALL"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_grants_scripted_gift_pokemon_into_authoritative_storage() {
        let root = temp_repository_root("scripted-gift-pokemon");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let grant = session
            .grant_scripted_gift_pokemon(
                &runtime,
                "RuntimeMap",
                "RuntimeGiftScript",
                12,
                "PLAYER",
                1234,
                Dv::from_non_hp(10, 10, 10, 10),
                Some("Leafy".to_string()),
            )
            .expect("gift Pokemon grants");

        assert_eq!(
            grant.outcome.location,
            CaptureStorageLocation::Party { slot: 0 }
        );
        assert_eq!(grant.outcome.pokemon.species.id, "CHIKORITA");
        assert_eq!(grant.outcome.pokemon.level, 7);
        assert_eq!(grant.outcome.pokemon.item.as_deref(), Some("BERRY"));
        assert_eq!(grant.outcome.pokemon.nickname, "Leafy");
        assert_eq!(grant.outcome.pokemon.original_trainer_name, "PLAYER");
        assert_eq!(
            session.state.party.pokemon[0],
            Some(crystal_core::state::PartyPokemonRef {
                species: "CHIKORITA".to_string(),
                level: 7,
            })
        );
        assert!(session.state.pokedex.has_caught("CHIKORITA"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_rejects_unresolved_gift_labels_and_uses_resolved_gift_levels() {
        let root = temp_repository_root("scripted-gift-rejections");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let nickname_error = session
            .grant_scripted_gift_pokemon(
                &runtime,
                "RuntimeMap",
                "RuntimeGiftScript",
                12,
                "PLAYER",
                1234,
                Dv::default(),
                None,
            )
            .expect_err("nickname label must be resolved by caller");
        assert!(nickname_error
            .to_string()
            .contains("requires resolved nickname label RuntimeGiftName"));

        let egg = session
            .grant_scripted_gift_pokemon(
                &runtime,
                "RuntimeMap",
                "RuntimeEggScript",
                3,
                "PLAYER",
                1234,
                Dv::default(),
                None,
            )
            .expect("compiled egg level is already resolved");
        assert_eq!(egg.outcome.pokemon.level, 5);
        assert_eq!(egg.outcome.pokemon.nickname, "EGG");
        assert_eq!(egg.outcome.pokemon.hp, 0);
        assert_eq!(session.state.storage.party.filled_slots(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_starts_scripted_trainer_battle_from_exact_map_script_command() {
        let root = temp_repository_root("scripted-trainer-battle");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
        session
            .state
            .storage
            .register_capture(player)
            .expect("player party Pokemon");
        session.state.sync_party_from_storage();

        let start = session
            .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
            .expect("scripted trainer battle resolves");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should not already be defeated");
        };

        assert_eq!(start.trainer_id, "RIVAL1");
        assert_eq!(start.trainer_class, "RIVAL1");
        assert_eq!(start.enemy_pokemon.species.id, "CHIKORITA");
        assert_eq!(session.state.battle, BattleMemory::from(&start));
        assert!(session.state.pokedex.has_seen("CHIKORITA"));
        assert_eq!(session.state.battle_active_party_index, Some(0));
        assert_eq!(session.state.battle_active_enemy_party_index, Some(0));
        session
            .state
            .bag
            .add_item(&runtime.data.items["MASTER_BALL"], 1)
            .expect("add master ball");
        let blocked = session
            .throw_ball_at_active_battle(&runtime, "MASTER_BALL")
            .expect("trainer battle ball throw resolves as blocked")
            .outcome
            .expect("ball was available");
        assert!(blocked.blocked);
        assert!(!blocked.caught);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["MASTER_BALL"]),
            0
        );
        match &mut session.state.battle {
            BattleMemory::Trainer {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                enemy_pokemon.hp = 0;
                enemy_party[0].hp = 0;
            }
            other => panic!("expected trainer battle, got {other:?}"),
        }
        let trainer_rewards = session
            .claim_active_trainer_battle_rewards(&runtime)
            .expect("claim trainer rewards");
        assert_eq!(
            trainer_rewards.outcome.experience_awarded,
            crystal_core::systems::battle_rewards::trainer_experience_award(
                &runtime.data.battle_reward_rules,
                match &session.state.battle {
                    BattleMemory::Trainer { enemy_pokemon, .. } => enemy_pokemon,
                    other => panic!("expected trainer battle, got {other:?}"),
                }
            )
            .expect("trainer experience")
        );
        let completion = session
            .complete_scripted_trainer_battle(
                &runtime,
                "RuntimeMap",
                "RuntimeTrainerScript",
                8,
                true,
                false,
            )
            .expect("scripted trainer battle completes");
        assert!(completion.continued_after_battle);
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);
        assert_eq!(session.state.battle_active_enemy_party_index, None);
        assert_eq!(completion.trainer_prize_money, Some(500));
        assert_eq!(completion.money_after, Some(500));
        assert_eq!(session.state.money, 500);
        assert_eq!(
            session
                .state
                .flags
                .is_event_flag_set("EVENT_BEAT_RUNTIME_RIVAL"),
            Ok(true)
        );
        let effects = completion.effects.expect("trainer effects apply");
        assert_eq!(
            effects.event_flags_set,
            vec!["EVENT_RUNTIME_TRAINER_POST".to_string()]
        );
        assert_eq!(
            effects.script_flags_set,
            vec!["ENGINE_RUNTIME_TRAINER_POST".to_string()]
        );

        let error = session
            .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 9)
            .expect_err("command indexes are exact");
        assert!(error
            .to_string()
            .contains("has no scripted trainer battle at RuntimeTrainerScript:9"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_trainer_battle_advances_through_exact_compiled_party() {
        let root = temp_repository_root("trainer-battle-party-advance");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let trainer = data
            .trainers
            .trainers
            .get_mut("RIVAL1")
            .expect("trainer exists");
        trainer.party.push(TrainerPartyPokemon {
            species: "CHIKORITA".to_string(),
            level: 6,
            item: None,
            moves: Vec::new(),
            dvs: Dv::from_non_hp(1, 1, 1, 1),
        });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
        session
            .state
            .storage
            .register_capture(player)
            .expect("player party Pokemon");
        session.state.sync_party_from_storage();
        let start = session
            .start_scripted_trainer_battle(&runtime, "RuntimeMap", "RuntimeTrainerScript", 8)
            .expect("scripted trainer battle resolves");
        let TrainerBattleStartStatus::Started(start) = start else {
            panic!("trainer should start");
        };
        assert_eq!(start.enemy_party.len(), 2);
        assert_eq!(session.state.battle_active_enemy_party_index, Some(0));
        match &mut session.state.battle {
            BattleMemory::Trainer {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                enemy_pokemon.hp = 0;
                enemy_party[0].hp = 0;
            }
            other => panic!("expected trainer battle, got {other:?}"),
        }
        let before_unclaimed_advance = session.state.clone();
        let unclaimed = session
            .advance_active_trainer_battle()
            .expect_err("cannot advance before reward claim");
        assert!(unclaimed
            .to_string()
            .contains("rewards have not been claimed"));
        assert_eq!(session.state, before_unclaimed_advance);
        let first_rewards = session
            .claim_active_trainer_battle_rewards(&runtime)
            .expect("first trainer rewards");
        assert_eq!(
            first_rewards.outcome.experience_awarded,
            crystal_core::systems::battle_rewards::trainer_experience_award(
                &runtime.data.battle_reward_rules,
                match &session.state.battle {
                    BattleMemory::Trainer { enemy_party, .. } => &enemy_party[0],
                    other => panic!("expected trainer battle, got {other:?}"),
                }
            )
            .expect("trainer experience")
        );
        let duplicate_rewards = session
            .claim_active_trainer_battle_rewards(&runtime)
            .expect_err("trainer rewards cannot be claimed twice");
        assert!(duplicate_rewards
            .to_string()
            .contains("rewards already claimed"));

        let advance = session
            .advance_active_trainer_battle()
            .expect("advance to next trainer Pokemon");

        let next = advance.next_enemy.expect("next trainer Pokemon");
        assert!(!advance.trainer_defeated);
        assert_eq!(next.level, 6);
        assert_eq!(session.state.battle_active_enemy_party_index, Some(1));
        match &session.state.battle {
            BattleMemory::Trainer {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                assert_eq!(enemy_pokemon.level, 6);
                assert_eq!(enemy_party[0].hp, 0);
                assert_eq!(enemy_party[1], *enemy_pokemon);
            }
            other => panic!("expected trainer battle, got {other:?}"),
        }
        match &mut session.state.battle {
            BattleMemory::Trainer {
                enemy_pokemon,
                enemy_party,
                ..
            } => {
                enemy_pokemon.hp = 0;
                enemy_party[1].hp = 0;
            }
            other => panic!("expected trainer battle, got {other:?}"),
        }
        let second_rewards = session
            .claim_active_trainer_battle_rewards(&runtime)
            .expect("second trainer rewards");
        assert_eq!(
            second_rewards.outcome.experience_awarded,
            crystal_core::systems::battle_rewards::trainer_experience_award(
                &runtime.data.battle_reward_rules,
                match &session.state.battle {
                    BattleMemory::Trainer { enemy_party, .. } => &enemy_party[1],
                    other => panic!("expected trainer battle, got {other:?}"),
                }
            )
            .expect("trainer experience")
        );

        let defeated = session
            .advance_active_trainer_battle()
            .expect("last trainer Pokemon defeated");

        assert_eq!(defeated.next_enemy, None);
        assert!(defeated.trainer_defeated);
        assert_eq!(session.state.battle_active_enemy_party_index, Some(1));
        let completion = session
            .complete_scripted_trainer_battle(
                &runtime,
                "RuntimeMap",
                "RuntimeTrainerScript",
                8,
                true,
                false,
            )
            .expect("trainer completion clears battle");
        assert!(completion.continued_after_battle);
        assert_eq!(completion.trainer_prize_money, Some(600));
        assert_eq!(completion.money_after, Some(600));
        assert_eq!(session.state.battle, BattleMemory::Inactive);
        assert_eq!(session.state.battle_active_party_index, None);
        assert_eq!(session.state.battle_active_enemy_party_index, None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_uses_exact_pack_item_effects_and_checksums_state() {
        let root = temp_repository_root("item-use");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        let mut itemfinder = runtime_item("ITEMFINDER", item_pocket("KEY_ITEM"));
        itemfinder.effect = "ITEMFINDER".to_string();
        itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
        itemfinder.field_usable = true;
        data.items.insert("POTION".to_string(), potion);
        data.items.insert("ITEMFINDER".to_string(), itemfinder);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 2)
            .expect("add potion");
        session
            .state
            .bag
            .add_item(&runtime.data.items["ITEMFINDER"], 1)
            .expect("add itemfinder");

        let first = session
            .use_bag_item(&runtime, "POTION", ItemUseContext::Battle)
            .expect("use potion");
        let second = session
            .use_bag_item(&runtime, "ITEMFINDER", ItemUseContext::Field)
            .expect("use itemfinder");

        assert_eq!(first.outcome.item_id, "POTION");
        assert!(first.outcome.consumed);
        assert_ne!(first.state_checksum, second.state_checksum);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["ITEMFINDER"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 2);
        assert_eq!(
            session.state.script_runtime.item_use_events[1].item_id,
            "ITEMFINDER"
        );

        let error = session
            .use_bag_item(&runtime, "itemfinder", ItemUseContext::Field)
            .expect_err("case changed item id rejected");
        assert!(error.to_string().contains("UnknownItem"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bicycle_item_toggles_bike_mode_from_definitive_effect() {
        let root = temp_repository_root("bicycle-item");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
        bicycle.effect = "MOD_BICYCLE".to_string();
        bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
        bicycle.field_usable = true;
        data.items.insert("BICYCLE".to_string(), bicycle);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BICYCLE"], 1)
            .expect("add bicycle");

        let on = session
            .use_bag_bicycle_in_field(&runtime, "BICYCLE")
            .expect("use bicycle on");

        assert_eq!(on.item_use.item_id, "BICYCLE");
        assert!(!on.item_use.consumed);
        assert_eq!(on.mode_before, MovementMode::Normal);
        assert_eq!(on.mode_after, MovementMode::Bike);
        assert_eq!(on.permission, permissions::FLOOR);
        assert_eq!(session.overworld.player.mode, MovementMode::Bike);
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);

        let off = session
            .use_bag_bicycle_in_field(&runtime, "BICYCLE")
            .expect("use bicycle off");

        assert_eq!(off.mode_before, MovementMode::Bike);
        assert_eq!(off.mode_after, MovementMode::Normal);
        assert_eq!(session.overworld.player.mode, MovementMode::Normal);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["BICYCLE"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 2);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bicycle_item_rejects_invalid_contexts_without_mutation() {
        let root = temp_repository_root("bicycle-item-reject");
        write_tileset(
            &root,
            "johto",
            r#"{
  "0": [7, 7, 7, 7]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
        bicycle.effect = "BICYCLE".to_string();
        bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
        bicycle.field_usable = true;
        let mut bad_bicycle = runtime_item("BAD_BICYCLE", item_pocket("KEY_ITEM"));
        bad_bicycle.effect = "NONE".to_string();
        bad_bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_bicycle.field_usable = true;
        data.items.insert("BICYCLE".to_string(), bicycle);
        data.items.insert("BAD_BICYCLE".to_string(), bad_bicycle);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BICYCLE"], 1)
            .expect("add bicycle");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_BICYCLE"], 1)
            .expect("add bad bicycle");

        let before_wall = session.state.clone();
        let wall = session
            .use_bag_bicycle_in_field(&runtime, "BICYCLE")
            .expect_err("wall permission rejects bicycle");
        assert!(wall.to_string().contains("permission 0x07"), "{wall}");
        assert_eq!(session.state, before_wall);
        assert_eq!(session.overworld.player.mode, MovementMode::Normal);

        let before_bad_effect = session.state.clone();
        let bad_effect = session
            .use_bag_bicycle_in_field(&runtime, "BAD_BICYCLE")
            .expect_err("wrong effect rejects bicycle");
        assert!(
            bad_effect.to_string().contains("InvalidFieldItemId"),
            "{bad_effect}"
        );
        assert_eq!(session.state, before_bad_effect);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bicycle_item_rejects_disallowed_environment_and_always_on_dismount() {
        let root = temp_repository_root("bicycle-item-env");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
        bicycle.effect = "BICYCLE".to_string();
        bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
        bicycle.field_usable = true;
        data.items.insert("BICYCLE".to_string(), bicycle);
        data.runtime_map_metadata.insert(
            "RUNTIME_MAP".to_string(),
            runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "INDOOR"),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BICYCLE"], 1)
            .expect("add bicycle");
        let before_indoor = session.state.clone();
        let indoor = session
            .use_bag_bicycle_in_field(&runtime, "BICYCLE")
            .expect_err("indoor rejects bicycle");
        assert!(indoor.to_string().contains("environment INDOOR"));
        assert_eq!(session.state, before_indoor);

        let mut route_data = minimal_runtime_data();
        let mut route_bicycle = runtime_item("BICYCLE", item_pocket("KEY_ITEM"));
        route_bicycle.effect = "BICYCLE".to_string();
        route_bicycle.field_menu = "ITEMMENU_CLOSE".to_string();
        route_bicycle.field_usable = true;
        route_data
            .items
            .insert("BICYCLE".to_string(), route_bicycle);
        let route_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(route_data, report()),
            identity(),
        )
        .expect("route runtime");
        let mut route_session = route_runtime
            .start_overworld_session(&asset_root, 0)
            .expect("route session starts");
        route_session
            .state
            .bag
            .add_item(&route_runtime.data.items["BICYCLE"], 1)
            .expect("add route bicycle");
        route_session
            .state
            .flags
            .set_engine_flag("ENGINE_ALWAYS_ON_BIKE", true)
            .expect("set always on bike");
        route_session.overworld.player.mode = MovementMode::Bike;
        route_session.state.overworld =
            OverworldMemory::from_snapshot(&route_session.overworld.snapshot());
        let before_always_on = route_session.state.clone();
        let always_on = route_session
            .use_bag_bicycle_in_field(&route_runtime, "BICYCLE")
            .expect_err("always-on bike rejects dismount");
        assert!(always_on.to_string().contains("ENGINE_ALWAYS_ON_BIKE"));
        assert_eq!(route_session.state, before_always_on);
        let _ = std::fs::remove_dir_all(root);
    }

    fn itemfinder_item() -> Item {
        let mut itemfinder = runtime_item("ITEMFINDER", item_pocket("KEY_ITEM"));
        itemfinder.effect = "ITEMFINDER".to_string();
        itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
        itemfinder.field_usable = true;
        itemfinder
    }

    fn hidden_item_event(script: &str, x: u16, y: u16) -> BackgroundEvent {
        BackgroundEvent {
            x,
            y,
            event_type: "BGEVENT_ITEM".to_string(),
            script: script.to_string(),
        }
    }

    fn hidden_item_pickup(script: &str, item_id: &str, event_flag: &str) -> ScriptFieldPickup {
        ScriptFieldPickup {
            command: "hiddenitem".to_string(),
            item_id: Some(item_id.to_string()),
            quantity: 1,
            event_flag: Some(event_flag.to_string()),
            fruit_tree_id: None,
            source_script: script.to_string(),
            command_index: 0,
        }
    }

    #[test]
    fn runtime_itemfinder_reports_uncollected_hidden_item_from_definitive_pack_events() {
        let root = temp_repository_root("itemfinder");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut itemfinder_item = itemfinder_item();
        itemfinder_item.effect = "MOD_ITEMFINDER".to_string();
        data.items.insert("ITEMFINDER".to_string(), itemfinder_item);
        data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.events.bg_events = vec![hidden_item_event("HiddenPotion", 4, 4)];
        map.script_field_pickups = vec![hidden_item_pickup(
            "HiddenPotion",
            "POTION",
            "EVENT_HIDDEN_POTION",
        )];
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["ITEMFINDER"], 1)
            .expect("add itemfinder");

        let itemfinder = session
            .use_bag_itemfinder_in_field(&runtime, "ITEMFINDER")
            .expect("use itemfinder");

        assert_eq!(itemfinder.item_use.item_id, "ITEMFINDER");
        assert!(!itemfinder.item_use.consumed);
        assert_eq!(itemfinder.itemfinder_sound_cues, 8);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["ITEMFINDER"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let found = itemfinder.found.expect("hidden item found");
        assert_eq!(found.map_name, "RuntimeMap");
        assert_eq!(found.tile, TilePosition::new(4, 4));
        assert_eq!(found.source_script, "HiddenPotion");
        assert_eq!(found.event_flag, "EVENT_HIDDEN_POTION");
        assert_eq!(found.item_id, "POTION");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_itemfinder_ignores_collected_or_out_of_range_hidden_items() {
        let root = temp_repository_root("itemfinder-empty");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items
            .insert("ITEMFINDER".to_string(), itemfinder_item());
        data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.events.bg_events = vec![
            hidden_item_event("HiddenPotion", 4, 4),
            hidden_item_event("FarHiddenPotion", 20, 20),
        ];
        map.script_field_pickups = vec![
            hidden_item_pickup("HiddenPotion", "POTION", "EVENT_HIDDEN_POTION"),
            hidden_item_pickup("FarHiddenPotion", "POTION", "EVENT_FAR_HIDDEN_POTION"),
        ];
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["ITEMFINDER"], 1)
            .expect("add itemfinder");
        session
            .state
            .flags
            .set_event_flag("EVENT_HIDDEN_POTION", true)
            .expect("collect hidden item");

        let itemfinder = session
            .use_bag_itemfinder_in_field(&runtime, "ITEMFINDER")
            .expect("use itemfinder");

        assert_eq!(itemfinder.found, None);
        assert_eq!(itemfinder.itemfinder_sound_cues, 0);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_itemfinder_rejects_wrong_effect_without_mutation() {
        let root = temp_repository_root("itemfinder-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad_itemfinder = runtime_item("BAD_ITEMFINDER", item_pocket("KEY_ITEM"));
        bad_itemfinder.effect = "NONE".to_string();
        bad_itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_itemfinder.field_usable = true;
        data.items
            .insert("BAD_ITEMFINDER".to_string(), bad_itemfinder);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_ITEMFINDER"], 1)
            .expect("add bad itemfinder");
        let before = session.state.clone();

        let error = session
            .use_bag_itemfinder_in_field(&runtime, "BAD_ITEMFINDER")
            .expect_err("wrong effect rejected");

        assert!(error.to_string().contains("InvalidFieldItemId"), "{error}");
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    fn squirtbottle_item() -> Item {
        let mut squirtbottle = runtime_item("SQUIRTBOTTLE", item_pocket("KEY_ITEM"));
        squirtbottle.effect = "SQUIRTBOTTLE".to_string();
        squirtbottle.field_menu = "ITEMMENU_CLOSE".to_string();
        squirtbottle.field_usable = true;
        squirtbottle
    }

    fn coin_case_item() -> Item {
        let mut coin_case = runtime_item("COIN_CASE", item_pocket("KEY_ITEM"));
        coin_case.effect = "COIN_CASE".to_string();
        coin_case.field_menu = "ITEMMENU_CLOSE".to_string();
        coin_case.field_usable = true;
        coin_case
    }

    fn blue_card_item() -> Item {
        let mut blue_card = runtime_item("BLUE_CARD", item_pocket("KEY_ITEM"));
        blue_card.effect = "BLUE_CARD".to_string();
        blue_card.field_menu = "ITEMMENU_CLOSE".to_string();
        blue_card.field_usable = true;
        blue_card
    }

    fn town_map_item() -> Item {
        let mut town_map = runtime_item("TOWN_MAP", item_pocket("KEY_ITEM"));
        town_map.effect = "TOWN_MAP".to_string();
        town_map.field_menu = "ITEMMENU_CURRENT".to_string();
        town_map.field_usable = true;
        town_map
    }

    fn add_runtime_landmark(data: &mut GameDataSet) {
        data.pokegear_landmarks.landmarks.push(PokegearLandmark {
            id: 1,
            constant: "LANDMARK_RUNTIME_TOWN".to_string(),
            label: "RUNTIME_TOWN".to_string(),
            name: "RUNTIME TOWN".to_string(),
            x: 12,
            y: 24,
            region: "JOHTO".to_string(),
        });
        data.pokegear_landmarks.map_to_landmark.insert(
            "RuntimeMap".to_string(),
            "LANDMARK_RUNTIME_TOWN".to_string(),
        );
    }

    fn wounded_runtime_pokemon(species_id: &str) -> Pokemon {
        let mut species = runtime_species();
        species.id = species_id.to_string();
        let mut pokemon = Pokemon::new_for_tests(species, 5, Dv::default());
        pokemon.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 1,
            pp_ups: 1,
        }];
        pokemon.hp = 1;
        pokemon.status = Some(minimal_step_event_rules().poison_status);
        pokemon.sleep_turns = 2;
        pokemon.confusion_turns = 3;
        pokemon.focus_energy = true;
        pokemon
    }

    #[test]
    fn runtime_special_heal_party_requires_pack_declared_routine_and_restores_party() {
        let root = temp_repository_root("special-heal-party");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.special_routines.insert("HealParty".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .storage
            .register_capture(wounded_runtime_pokemon("CHIKORITA"))
            .expect("store first");
        session
            .state
            .storage
            .register_capture(wounded_runtime_pokemon("CYNDAQUIL"))
            .expect("store second");
        session.state.sync_party_from_storage();

        let special = session
            .apply_special_routine(&runtime, "HealParty")
            .expect("heal party");

        assert_eq!(special.outcome.routine, "HealParty");
        assert_eq!(
            special.outcome.effect,
            SpecialRoutineEffect::HealParty {
                healed_slots: vec![0, 1]
            }
        );
        assert_eq!(
            session.state.script_runtime.last_special_routine.as_deref(),
            Some("HealParty")
        );
        for slot in 0..2 {
            let pokemon = session.state.storage.party.pokemon[slot]
                .as_ref()
                .expect("party pokemon");
            assert_eq!(pokemon.hp, pokemon.max_hp);
            assert_eq!(pokemon.status, None);
            assert_eq!(pokemon.sleep_turns, 0);
            assert_eq!(pokemon.confusion_turns, 0);
            assert!(!pokemon.focus_energy);
            assert_eq!(pokemon.moves[0].current_pp, 42);
            assert_eq!(
                session.state.party.pokemon[slot]
                    .as_ref()
                    .expect("projected party")
                    .species
                    .as_str(),
                pokemon.species.id
            );
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_routine_rejects_missing_or_unsupported_exact_routine_without_mutation() {
        let root = temp_repository_root("special-routine-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.special_routines.insert("FadeOutMusic".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .storage
            .register_capture(wounded_runtime_pokemon("CHIKORITA"))
            .expect("store");
        session.state.sync_party_from_storage();

        let before_missing = session.state.clone();
        let missing = session
            .apply_special_routine(&runtime, "HealParty")
            .expect_err("undeclared routine rejected");
        assert!(
            missing
                .to_string()
                .contains("missing exact special routine HealParty"),
            "{missing}"
        );
        assert_eq!(session.state, before_missing);

        let before_case = session.state.clone();
        let case_changed = session
            .apply_special_routine(&runtime, "fadeoutmusic")
            .expect_err("case changed routine rejected before execution");
        assert!(
            case_changed
                .to_string()
                .contains("missing exact special routine fadeoutmusic"),
            "{case_changed}"
        );
        assert_eq!(session.state, before_case);

        let before_undeclared_audio = session.state.clone();
        let undeclared_audio = session
            .apply_special_routine(&runtime, "FadeOutMusic")
            .expect_err("FadeOutMusic requires declared MUSIC_NONE");
        assert!(
            undeclared_audio
                .to_string()
                .contains("requires compiled music asset MUSIC_NONE"),
            "{undeclared_audio}"
        );
        assert_eq!(session.state, before_undeclared_audio);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_audio_routines_apply_exact_pack_declared_effects() {
        let root = temp_repository_root("special-audio-routines");
        write_floor_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_NONE.mid"),
        );
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/cries/CRY_CHIKORITA.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.special_routines.insert("FadeOutMusic".to_string());
        data.special_routines.insert("WaitSFX".to_string());
        data.special_routines.insert("PlayMapMusic".to_string());
        data.special_routines.insert("RestartMapMusic".to_string());
        data.special_routines.insert("PlayCurMonCry".to_string());
        data.special_routines.insert("PlaySlowCry".to_string());
        data.audio.push(
            ModpackAudioAsset::music("MUSIC_NONE", "content-packs/test/music/MUSIC_NONE.mid")
                .expect("music none asset"),
        );
        data.audio.push(
            ModpackAudioAsset::cry(
                "CRY_CHIKORITA",
                "content-packs/test/cries/CRY_CHIKORITA.mid",
            )
            .expect("cry asset"),
        );
        data.pokemon_cries.insert(
            "CHIKORITA".to_string(),
            PokemonCryMetadata {
                cry: "CRY_CHIKORITA".to_string(),
                pitch: 0,
                length: 0,
            },
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session.state.script_runtime.current_music = Some("MUSIC_ROUTE_30".to_string());
        session.state.script_runtime.map_music_restart_disabled = true;

        let fade = session
            .apply_special_routine(&runtime, "FadeOutMusic")
            .expect("fade out music");

        assert_eq!(
            fade.outcome.effect,
            SpecialRoutineEffect::FadeOutMusic {
                audio_id: "MUSIC_NONE".to_string(),
                fade_frames: 2
            }
        );
        let pending_fade = session
            .state
            .script_runtime
            .pending_music_fade
            .as_ref()
            .expect("pending fade");
        assert_eq!(pending_fade.audio_id, "MUSIC_NONE");
        assert_eq!(pending_fade.fade_frames, 2);
        assert_eq!(pending_fade.source_script, "FadeOutMusic");
        assert_eq!(
            session.state.script_runtime.current_music.as_deref(),
            Some("MUSIC_ROUTE_30")
        );
        assert_eq!(session.state.script_runtime.audio_events.len(), 1);
        assert_eq!(
            session.state.script_runtime.audio_events[0]
                .audio_id
                .as_deref(),
            Some("MUSIC_NONE")
        );
        assert_ne!(fade.state_checksum.hash(), 0);

        let wait_sfx = session
            .apply_special_routine(&runtime, "WaitSFX")
            .expect("wait sfx");

        assert_eq!(wait_sfx.outcome.effect, SpecialRoutineEffect::WaitSfx);
        assert!(session.state.script_runtime.waiting_for_sound_effect);
        assert_eq!(session.state.script_runtime.audio_events.len(), 2);
        assert_eq!(
            session.state.script_runtime.audio_events[1].kind,
            crystal_core::state::ScriptAudioRuntimeKind::WaitForSoundEffect
        );
        assert_eq!(session.state.script_runtime.audio_events[1].audio_id, None);
        assert_ne!(wait_sfx.state_checksum, fade.state_checksum);

        session.state.script_runtime.map_music_restart_disabled = true;
        let play_map_music = session
            .apply_special_routine(&runtime, "PlayMapMusic")
            .expect("play map music");

        assert_eq!(
            play_map_music.outcome.effect,
            SpecialRoutineEffect::PlayMapMusic
        );
        assert!(session.state.script_runtime.map_music_requested);
        assert!(!session.state.script_runtime.map_music_restart_disabled);
        assert_eq!(
            session.state.script_runtime.last_special_routine.as_deref(),
            Some("PlayMapMusic")
        );
        assert_ne!(play_map_music.state_checksum, wait_sfx.state_checksum);

        session.state.script_runtime.map_music_requested = false;
        session.state.script_runtime.map_music_restart_disabled = true;
        let restart = session
            .apply_special_routine(&runtime, "RestartMapMusic")
            .expect("restart map music");

        assert_eq!(
            restart.outcome.effect,
            SpecialRoutineEffect::RestartMapMusic
        );
        assert!(session.state.script_runtime.map_music_requested);
        assert!(!session.state.script_runtime.map_music_restart_disabled);
        assert_eq!(
            session.state.script_runtime.last_special_routine.as_deref(),
            Some("RestartMapMusic")
        );
        assert_ne!(restart.state_checksum, play_map_music.state_checksum);

        session
            .state
            .script_runtime
            .variables
            .insert("wCurPartySpecies".to_string(), "CHIKORITA".to_string());
        let cur_cry = session
            .apply_special_routine(&runtime, "PlayCurMonCry")
            .expect("play current cry");

        assert_eq!(
            cur_cry.outcome.effect,
            SpecialRoutineEffect::PlayCurMonCry {
                species: "CHIKORITA".to_string(),
                audio_id: "CRY_CHIKORITA".to_string()
            }
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .audio_events
                .last()
                .expect("cry event")
                .kind,
            crystal_core::state::ScriptAudioRuntimeKind::Cry
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .audio_events
                .last()
                .expect("cry event")
                .audio_id
                .as_deref(),
            Some("CRY_CHIKORITA")
        );
        assert_ne!(cur_cry.state_checksum, restart.state_checksum);

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "CHIKORITA".to_string());
        let slow_cry = session
            .apply_special_routine(&runtime, "PlaySlowCry")
            .expect("play slow cry");

        assert_eq!(
            slow_cry.outcome.effect,
            SpecialRoutineEffect::PlaySlowCry {
                species: "CHIKORITA".to_string(),
                audio_id: "CRY_CHIKORITA".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.audio_events.len(), 4);
        assert_ne!(slow_cry.state_checksum, cur_cry.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_screen_fades_apply_exact_pack_declared_effects() {
        let root = temp_repository_root("special-screen-fades");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        for routine in [
            "FadeOutToWhite",
            "FadeInFromWhite",
            "FadeOutToBlack",
            "FadeInFromBlack",
        ] {
            data.special_routines.insert(routine.to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");

        let fade_out_white = session
            .apply_special_routine(&runtime, "FadeOutToWhite")
            .expect("fade out white");

        assert_eq!(
            fade_out_white.outcome.effect,
            SpecialRoutineEffect::ScreenFade {
                color: crystal_core::state::ScriptFadeColor::White,
                direction: crystal_core::state::ScriptFadeDirection::Out,
                frames: 8
            }
        );
        assert_eq!(session.state.script_runtime.graphics_events.len(), 1);
        assert_eq!(
            session
                .state
                .script_runtime
                .pending_screen_fade
                .as_ref()
                .expect("pending fade")
                .source_script,
            "FadeOutToWhite"
        );

        let fade_in_black = session
            .apply_special_routine(&runtime, "FadeInFromBlack")
            .expect("fade in black");

        assert_eq!(
            fade_in_black.outcome.effect,
            SpecialRoutineEffect::ScreenFade {
                color: crystal_core::state::ScriptFadeColor::Black,
                direction: crystal_core::state::ScriptFadeDirection::In,
                frames: 8
            }
        );
        assert_eq!(session.state.script_runtime.graphics_events.len(), 2);
        assert_eq!(
            session
                .state
                .script_runtime
                .pending_screen_fade
                .as_ref()
                .expect("pending fade")
                .source_script,
            "FadeInFromBlack"
        );
        assert_ne!(fade_out_white.state_checksum, fade_in_black.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_graphics_and_hardware_commands_apply_exact_pack_declared_effects() {
        let root = temp_repository_root("special-graphics-commands");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        for routine in [
            "GameboyCheck",
            "CheckMobileAdapterStatusSpecial",
            "ClearBGPalettesBufferScreen",
            "ClearBGPalettes",
            "UpdateTimePals",
            "ClearTilemap",
            "LoadMapPalettes",
            "RefreshSprites",
            "UpdateSprites",
            "ReloadSpritesNoPalettes",
        ] {
            data.special_routines.insert(routine.to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");

        let gameboy = session
            .apply_special_routine(&runtime, "GameboyCheck")
            .expect("gameboy check");

        assert_eq!(
            gameboy.outcome.effect,
            SpecialRoutineEffect::GameboyCheck {
                token: "GBCHECK_CGB".to_string()
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("GBCHECK_CGB")
        );

        let mobile = session
            .apply_special_routine(&runtime, "CheckMobileAdapterStatusSpecial")
            .expect("mobile adapter status");

        assert_eq!(
            mobile.outcome.effect,
            SpecialRoutineEffect::MobileAdapterStatus {
                value: "0".to_string()
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("0")
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .variables
                .get("_value")
                .map(String::as_str),
            Some("0")
        );

        let clear = session
            .apply_special_routine(&runtime, "ClearBGPalettes")
            .expect("clear palettes");

        assert_eq!(
            clear.outcome.effect,
            SpecialRoutineEffect::GraphicsCommand {
                kind: crystal_core::state::ScriptGraphicsRuntimeKind::ClearBgPalettes
            }
        );
        assert_eq!(session.state.script_runtime.graphics_events.len(), 1);
        assert_eq!(
            session.state.script_runtime.graphics_events[0].kind,
            crystal_core::state::ScriptGraphicsRuntimeKind::ClearBgPalettes
        );
        assert_eq!(session.state.script_runtime.graphics_events[0].color, None);

        let reload = session
            .apply_special_routine(&runtime, "ReloadSpritesNoPalettes")
            .expect("reload sprites");

        assert_eq!(
            reload.outcome.effect,
            SpecialRoutineEffect::GraphicsCommand {
                kind: crystal_core::state::ScriptGraphicsRuntimeKind::ReloadSpritesNoPalettes
            }
        );
        assert_eq!(session.state.script_runtime.graphics_events.len(), 2);
        assert_eq!(
            session.state.script_runtime.graphics_events[1].kind,
            crystal_core::state::ScriptGraphicsRuntimeKind::ReloadSpritesNoPalettes
        );
        assert_eq!(
            session.state.script_runtime.last_special_routine.as_deref(),
            Some("ReloadSpritesNoPalettes")
        );
        assert_ne!(gameboy.state_checksum, mobile.state_checksum);
        assert_ne!(mobile.state_checksum, clear.state_checksum);
        assert_ne!(clear.state_checksum, reload.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_party_checks_apply_exact_pack_declared_effects() {
        let root = temp_repository_root("special-party-checks");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.special_routines
            .insert("GetFirstPokemonHappiness".to_string());
        data.special_routines
            .insert("CheckFirstMonIsEgg".to_string());
        data.special_routines
            .insert("FindPartyMonThatSpecies".to_string());
        data.special_routines
            .insert("FindPartyMonThatSpeciesYourTrainerID".to_string());
        data.special_routines
            .insert("FindPartyMonAboveLevel".to_string());
        data.special_routines
            .insert("FindPartyMonAtLeastThatHappy".to_string());
        data.special_routines.insert("MonCheck".to_string());
        data.special_routines.insert("BeastsCheck".to_string());
        data.special_routines
            .insert("GameCornerPrizeMonCheckDex".to_string());
        data.special_routines.insert("UnusedSetSeenMon".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let mut egg = wounded_runtime_pokemon("EGG");
        egg.nickname = "EGG".to_string();
        egg.happiness = 1;
        let mut chikorita = wounded_runtime_pokemon("CHIKORITA");
        chikorita.nickname = "Leafy".to_string();
        chikorita.level = 31;
        chikorita.happiness = 201;
        chikorita.original_trainer_name = "KRIS".to_string();
        chikorita.original_trainer_id = 0x2222;
        session.state.player_name = "KRIS".to_string();
        session.state.player_id = 0x2222;
        session
            .state
            .storage
            .register_capture(egg)
            .expect("store egg");
        session
            .state
            .storage
            .register_capture(chikorita)
            .expect("store mon");
        session.state.sync_party_from_storage();

        let egg_check = session
            .apply_special_routine(&runtime, "CheckFirstMonIsEgg")
            .expect("egg check");

        assert_eq!(
            egg_check.outcome.effect,
            SpecialRoutineEffect::CheckFirstMonIsEgg {
                species: "EGG".to_string(),
                nickname: "EGG".to_string(),
                is_egg: true
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("1")
        );

        let happiness = session
            .apply_special_routine(&runtime, "GetFirstPokemonHappiness")
            .expect("happiness");

        assert_eq!(
            happiness.outcome.effect,
            SpecialRoutineEffect::FirstPokemonHappiness {
                party_slot: 1,
                species: "CHIKORITA".to_string(),
                nickname: "Leafy".to_string(),
                happiness: 201
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("201")
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("Leafy")
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "CHIKORITA".to_string());
        let found = session
            .apply_special_routine(&runtime, "FindPartyMonThatSpecies")
            .expect("find party mon");

        assert_eq!(
            found.outcome.effect,
            SpecialRoutineEffect::FindPartyMonThatSpecies {
                species: "CHIKORITA".to_string(),
                found: true
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("1")
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "30".to_string());
        let above_level = session
            .apply_special_routine(&runtime, "FindPartyMonAboveLevel")
            .expect("find above level");

        assert_eq!(
            above_level.outcome.effect,
            SpecialRoutineEffect::FindPartyMonAboveLevel {
                level: 30,
                found: true,
                species: Some("CHIKORITA".to_string())
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "202".to_string());
        let happy = session
            .apply_special_routine(&runtime, "FindPartyMonAtLeastThatHappy")
            .expect("find happy");

        assert_eq!(
            happy.outcome.effect,
            SpecialRoutineEffect::FindPartyMonAtLeastThatHappy {
                happiness: 202,
                found: false,
                species: None
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "CHIKORITA".to_string());
        let ot_found = session
            .apply_special_routine(&runtime, "FindPartyMonThatSpeciesYourTrainerID")
            .expect("find ot species");

        assert_eq!(
            ot_found.outcome.effect,
            SpecialRoutineEffect::FindPartyMonThatSpeciesYourTrainerId {
                species: "CHIKORITA".to_string(),
                player_name: "KRIS".to_string(),
                player_id: 0x2222,
                found: true
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "CHIKORITA".to_string());
        let mon_check = session
            .apply_special_routine(&runtime, "MonCheck")
            .expect("mon check");

        assert_eq!(
            mon_check.outcome.effect,
            SpecialRoutineEffect::MonCheck {
                species: "CHIKORITA".to_string(),
                player_name: "KRIS".to_string(),
                player_id: 0x2222,
                owned: true
            }
        );

        for species in ["RAIKOU", "ENTEI", "SUICUNE"] {
            let mut beast = wounded_runtime_pokemon(species);
            beast.original_trainer_name = "KRIS".to_string();
            beast.original_trainer_id = 0x2222;
            session
                .state
                .storage
                .register_capture(beast)
                .expect("store beast");
        }
        session.state.sync_party_from_storage();
        let beasts = session
            .apply_special_routine(&runtime, "BeastsCheck")
            .expect("beasts check");

        assert_eq!(
            beasts.outcome.effect,
            SpecialRoutineEffect::BeastsCheck {
                player_name: "KRIS".to_string(),
                player_id: 0x2222,
                missing_species: None,
                owned_all: true
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "CHIKORITA".to_string());
        let prize_dex = session
            .apply_special_routine(&runtime, "GameCornerPrizeMonCheckDex")
            .expect("game corner prize dex");

        assert_eq!(
            prize_dex.outcome.effect,
            SpecialRoutineEffect::GameCornerPrizeMonCheckDex {
                species: "CHIKORITA".to_string(),
                species_int_id: 0,
                already_caught: false,
                recorded_caught: true
            }
        );
        assert!(session.state.pokedex.has_seen("CHIKORITA"));
        assert!(session.state.pokedex.has_caught("CHIKORITA"));

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "CHIKORITA".to_string());
        let set_seen = session
            .apply_special_routine(&runtime, "UnusedSetSeenMon")
            .expect("unused set seen mon");

        assert_eq!(
            set_seen.outcome.effect,
            SpecialRoutineEffect::UnusedSetSeenMon {
                species: "CHIKORITA".to_string(),
                species_int_id: 0,
                newly_seen: false
            }
        );
        assert_ne!(egg_check.state_checksum, happiness.state_checksum);
        assert_ne!(happiness.state_checksum, found.state_checksum);
        assert_ne!(found.state_checksum, above_level.state_checksum);
        assert_ne!(above_level.state_checksum, happy.state_checksum);
        assert_ne!(happy.state_checksum, ot_found.state_checksum);
        assert_ne!(ot_found.state_checksum, mon_check.state_checksum);
        assert_ne!(mon_check.state_checksum, beasts.state_checksum);
        assert_ne!(beasts.state_checksum, prize_dex.state_checksum);
        assert_ne!(prize_dex.state_checksum, set_seen.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_pc_display_and_move_deletion_apply_pack_declared_effects() {
        let root = temp_repository_root("special-pc-display-move-deletion");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        for routine in [
            "PokemonCenterPC",
            "PlayersHousePC",
            "ProfOaksPCBoot",
            "OverworldTownMap",
            "UnownPrinter",
            "MapRadio",
            "NameRival",
            "MoveDeletion",
        ] {
            data.special_routines.insert(routine.to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let mut pokemon = wounded_runtime_pokemon("CHIKORITA");
        pokemon.moves.push(LearnedMove {
            name: "GROWL".to_string(),
            current_pp: 1,
            pp_ups: 0,
        });
        session
            .state
            .storage
            .register_capture(pokemon)
            .expect("store party mon");
        session.state.sync_party_from_storage();
        session.state.current_pc_box = 2;

        let pc = session
            .apply_special_routine(&runtime, "PokemonCenterPC")
            .expect("pokemon center pc");

        assert_eq!(
            pc.outcome.effect,
            SpecialRoutineEffect::PokemonCenterPc {
                party_count: 1,
                current_pc_box: 2
            }
        );
        assert_eq!(
            session.state.script_runtime.active_menu.as_deref(),
            Some("PokemonCenterPC")
        );

        let player_pc = session
            .apply_special_routine(&runtime, "PlayersHousePC")
            .expect("players house pc");

        assert_eq!(
            player_pc.outcome.effect,
            SpecialRoutineEffect::PlayersHousePc { party_count: 1 }
        );

        let oak = session
            .apply_special_routine(&runtime, "ProfOaksPCBoot")
            .expect("oak pc");

        assert_eq!(
            oak.outcome.effect,
            SpecialRoutineEffect::ProfOaksPcBoot {
                seen_count: 0,
                caught_count: 0,
                rating_label: "OakRating01".to_string()
            }
        );

        let town_map = session
            .apply_special_routine(&runtime, "OverworldTownMap")
            .expect("town map");

        assert_eq!(
            town_map.outcome.effect,
            SpecialRoutineEffect::OverworldTownMap {
                map_name: Some("RuntimeMap".to_string())
            }
        );

        let printer = session
            .apply_special_routine(&runtime, "UnownPrinter")
            .expect("unown printer");

        assert_eq!(
            printer.outcome.effect,
            SpecialRoutineEffect::UnownPrinter { unlocked: true }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "MAPRADIO_UNOWN".to_string());
        let radio = session
            .apply_special_routine(&runtime, "MapRadio")
            .expect("map radio");

        assert_eq!(
            radio.outcome.effect,
            SpecialRoutineEffect::MapRadio {
                station: "MAPRADIO_UNOWN".to_string()
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_rival_name".to_string(), "SILVER".to_string());
        let rival = session
            .apply_special_routine(&runtime, "NameRival")
            .expect("name rival");

        assert_eq!(
            rival.outcome.effect,
            SpecialRoutineEffect::NameRival {
                rival_name: "SILVER".to_string()
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_party_slot".to_string(), "0".to_string());
        session
            .state
            .script_runtime
            .variables
            .insert("_move_slot".to_string(), "1".to_string());
        let deletion = session
            .apply_special_routine(&runtime, "MoveDeletion")
            .expect("move deletion");

        assert_eq!(
            deletion.outcome.effect,
            SpecialRoutineEffect::MoveDeletion {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                deleted_move: "GROWL".to_string(),
                remaining_moves: 1
            }
        );
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("party mon")
                .moves
                .iter()
                .map(|move_slot| move_slot.name.as_str())
                .collect::<Vec<_>>(),
            vec!["TACKLE"]
        );
        assert_ne!(pc.state_checksum, player_pc.state_checksum);
        assert_ne!(player_pc.state_checksum, oak.state_checksum);
        assert_ne!(oak.state_checksum, town_map.state_checksum);
        assert_ne!(town_map.state_checksum, printer.state_checksum);
        assert_ne!(printer.state_checksum, radio.state_checksum);
        assert_ne!(radio.state_checksum, rival.state_checksum);
        assert_ne!(rival.state_checksum, deletion.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_visual_commands_apply_pack_declared_effects() {
        let root = temp_repository_root("special-visual-commands");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let cases = [
            (
                "BattleTowerFade",
                ScriptGraphicsRuntimeKind::BattleTowerFade,
            ),
            (
                "UpdatePlayerSprite",
                ScriptGraphicsRuntimeKind::UpdatePlayerSprite,
            ),
            (
                "HealMachineAnim",
                ScriptGraphicsRuntimeKind::HealMachineAnim,
            ),
            ("SurfStartStep", ScriptGraphicsRuntimeKind::SurfStartStep),
            (
                "LoadUsedSpritesGFX",
                ScriptGraphicsRuntimeKind::LoadUsedSpritesGfx,
            ),
            (
                "ToggleMaptileDecorations",
                ScriptGraphicsRuntimeKind::ToggleMaptileDecorations,
            ),
            (
                "ToggleDecorationsVisibility",
                ScriptGraphicsRuntimeKind::ToggleDecorationsVisibility,
            ),
            ("MagnetTrain", ScriptGraphicsRuntimeKind::MagnetTrain),
            ("Diploma", ScriptGraphicsRuntimeKind::Diploma),
            ("PrintDiploma", ScriptGraphicsRuntimeKind::PrintDiploma),
            ("UnownPuzzle", ScriptGraphicsRuntimeKind::UnownPuzzle),
            ("OmanyteChamber", ScriptGraphicsRuntimeKind::OmanyteChamber),
            (
                "DisplayUnownWords",
                ScriptGraphicsRuntimeKind::DisplayUnownWords,
            ),
        ];
        for (routine, _) in cases {
            data.special_routines.insert(routine.to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let mut previous_checksum = None;

        for (routine, kind) in cases {
            let use_result = session
                .apply_special_routine(&runtime, routine)
                .expect("visual special");

            assert_eq!(
                use_result.outcome.effect,
                SpecialRoutineEffect::RuntimeVisualCommand { kind }
            );
            assert_eq!(
                session.state.script_runtime.last_special_routine.as_deref(),
                Some(routine)
            );
            assert_eq!(
                session
                    .state
                    .script_runtime
                    .graphics_events
                    .last()
                    .expect("graphics event")
                    .kind,
                kind
            );
            if let Some(previous_checksum) = previous_checksum {
                assert_ne!(previous_checksum, use_result.state_checksum);
            }
            previous_checksum = Some(use_result.state_checksum);
        }
        assert_eq!(
            session.state.script_runtime.graphics_events.len(),
            cases.len()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_service_routines_apply_pack_declared_effects() {
        use crystal_core::systems::special_routines::{
            HappinessChangeEntry, HappinessData, HappinessServiceOutcome, HappinessServiceTable,
        };

        let root = temp_repository_root("special-service-routines");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.moves
            .insert("EMBER".to_string(), runtime_move_named("EMBER", 25));
        for routine in [
            "CheckPokerus",
            "OlderHaircutBrother",
            "YoungerHaircutBrother",
            "DaisysGrooming",
            "NameRater",
            "PokeSeer",
            "MoveTutor",
        ] {
            data.special_routines.insert(routine.to_string());
        }
        data.happiness_data = Some(HappinessData {
            changes: vec![
                HappinessChangeEntry {
                    code: "HAPPINESS_OLDERCUT1".to_string(),
                    change_code: 9,
                    low: 1,
                    mid: 1,
                    high: 1,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_YOUNGCUT2".to_string(),
                    change_code: 13,
                    low: 3,
                    mid: 3,
                    high: 1,
                },
                HappinessChangeEntry {
                    code: "HAPPINESS_GROOMING".to_string(),
                    change_code: 18,
                    low: 3,
                    mid: 3,
                    high: 1,
                },
            ],
            services: vec![
                HappinessServiceTable {
                    routine: "OlderHaircutBrother".to_string(),
                    outcomes: vec![HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 2,
                        change_code: 9,
                    }],
                },
                HappinessServiceTable {
                    routine: "YoungerHaircutBrother".to_string(),
                    outcomes: vec![
                        HappinessServiceOutcome {
                            roll_weight: 76,
                            script_value: 2,
                            change_code: 9,
                        },
                        HappinessServiceOutcome {
                            roll_weight: 255,
                            script_value: 3,
                            change_code: 13,
                        },
                    ],
                },
                HappinessServiceTable {
                    routine: "DaisysGrooming".to_string(),
                    outcomes: vec![HappinessServiceOutcome {
                        roll_weight: 255,
                        script_value: 2,
                        change_code: 18,
                    }],
                },
            ],
        });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let mut pokemon = wounded_runtime_pokemon("CHIKORITA");
        pokemon.nickname = "Leafy".to_string();
        pokemon.happiness = 70;
        pokemon.status = Some("POKERUS".to_string());
        pokemon.original_trainer_name = "KRIS".to_string();
        pokemon.original_trainer_id = 0x2222;
        session
            .state
            .storage
            .register_capture(pokemon)
            .expect("store party mon");
        session.state.sync_party_from_storage();

        let pokerus = session
            .apply_special_routine(&runtime, "CheckPokerus")
            .expect("check pokerus");

        assert_eq!(
            pokerus.outcome.effect,
            SpecialRoutineEffect::CheckPokerus {
                found: true,
                newly_discovered: true
            }
        );
        assert_eq!(
            session
                .state
                .flags
                .is_engine_flag_set("ENGINE_CAUGHT_POKERUS"),
            Ok(true)
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_party_slot".to_string(), "0".to_string());
        session
            .state
            .script_runtime
            .variables
            .insert("_rng_roll".to_string(), "0".to_string());
        let older = session
            .apply_special_routine(&runtime, "OlderHaircutBrother")
            .expect("older haircut");

        assert_eq!(
            older.outcome.effect,
            SpecialRoutineEffect::HappinessService {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                old_happiness: 70,
                new_happiness: 71,
                script_value: 2,
                change_code: 9,
                rng_seed_after: 1
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_rng_roll".to_string(), "80".to_string());
        let younger = session
            .apply_special_routine(&runtime, "YoungerHaircutBrother")
            .expect("younger haircut");

        assert_eq!(
            younger.outcome.effect,
            SpecialRoutineEffect::HappinessService {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                old_happiness: 71,
                new_happiness: 74,
                script_value: 3,
                change_code: 13,
                rng_seed_after: 1
            }
        );

        let daisy = session
            .apply_special_routine(&runtime, "DaisysGrooming")
            .expect("daisy grooming");

        assert_eq!(
            daisy.outcome.effect,
            SpecialRoutineEffect::HappinessService {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                old_happiness: 74,
                new_happiness: 77,
                script_value: 2,
                change_code: 18,
                rng_seed_after: 1
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_selected_nickname".to_string(), "Chiko".to_string());
        let rename = session
            .apply_special_routine(&runtime, "NameRater")
            .expect("name rater");

        assert_eq!(
            rename.outcome.effect,
            SpecialRoutineEffect::NameRater {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                old_nickname: "Leafy".to_string(),
                new_nickname: "Chiko".to_string()
            }
        );

        let seer = session
            .apply_special_routine(&runtime, "PokeSeer")
            .expect("poke seer");

        assert_eq!(
            seer.outcome.effect,
            SpecialRoutineEffect::PokeSeer {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                nickname: "Chiko".to_string(),
                original_trainer_name: "KRIS".to_string(),
                original_trainer_id: 0x2222
            }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_move".to_string(), "EMBER".to_string());
        let tutor = session
            .apply_special_routine(&runtime, "MoveTutor")
            .expect("move tutor");

        assert_eq!(
            tutor.outcome.effect,
            SpecialRoutineEffect::MoveTutor {
                party_slot: 0,
                species: "CHIKORITA".to_string(),
                move_name: "EMBER".to_string(),
                learned: true
            }
        );
        assert_ne!(pokerus.state_checksum, older.state_checksum);
        assert_ne!(older.state_checksum, younger.state_checksum);
        assert_ne!(younger.state_checksum, daisy.state_checksum);
        assert_ne!(daisy.state_checksum, rename.state_checksum);
        assert_ne!(rename.state_checksum, seer.state_checksum);
        assert_ne!(seer.state_checksum, tutor.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_service_menu_requests_apply_pack_declared_effects() {
        let root = temp_repository_root("special-service-menu-requests");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let active_routines = [
            "BankOfMom",
            "SlotMachine",
            "CardFlip",
            "DisplayLinkRecord",
            "TrainerHouse",
            "PhotoStudio",
            "Menu_ChallengeExplanationCancel",
        ];
        let inactive_routines = [
            "UnusedMemoryGame",
            "UnusedCheckUnusedTwoDayTimer",
            "UnusedFindItemInPCOrBag",
            "UnusedDummySpecial",
        ];
        for routine in active_routines.into_iter().chain(inactive_routines) {
            data.special_routines.insert(routine.to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session.state.money = 1200;
        session.state.moms_money = 345;
        session.state.coins = 99;
        session.state.link_battle_stats.wins = 7;
        session.state.link_battle_stats.losses = 3;
        session.state.link_battle_stats.draws = 1;
        session
            .state
            .storage
            .register_capture(wounded_runtime_pokemon("CHIKORITA"))
            .expect("store photo mon");
        session.state.sync_party_from_storage();
        let mut previous_checksum = None;

        let expectations = [
            (
                "BankOfMom",
                SpecialRoutineEffect::BankOfMom {
                    money: 1200,
                    moms_money: 345,
                },
            ),
            (
                "SlotMachine",
                SpecialRoutineEffect::SlotMachine { coins: 99 },
            ),
            ("CardFlip", SpecialRoutineEffect::CardFlip { coins: 99 }),
            (
                "DisplayLinkRecord",
                SpecialRoutineEffect::DisplayLinkRecord {
                    wins: 7,
                    losses: 3,
                    draws: 1,
                },
            ),
            (
                "TrainerHouse",
                SpecialRoutineEffect::TrainerHouse {
                    wins: 7,
                    losses: 3,
                    draws: 1,
                },
            ),
            (
                "PhotoStudio",
                SpecialRoutineEffect::PhotoStudio {
                    party_slot: Some(0),
                    species: Some("CHIKORITA".to_string()),
                },
            ),
            (
                "Menu_ChallengeExplanationCancel",
                SpecialRoutineEffect::BattleTowerChallengeExplanationCancel,
            ),
        ];

        for (routine, expected_effect) in expectations {
            let use_result = session
                .apply_special_routine(&runtime, routine)
                .expect("service special");

            assert_eq!(use_result.outcome.effect, expected_effect);
            assert_eq!(
                session.state.script_runtime.active_menu.as_deref(),
                Some(routine)
            );
            if let Some(previous_checksum) = previous_checksum {
                assert_ne!(previous_checksum, use_result.state_checksum);
            }
            previous_checksum = Some(use_result.state_checksum);
        }
        for routine in inactive_routines {
            let before = session.state.clone();
            let error = session
                .apply_special_routine(&runtime, routine)
                .expect_err("inactive declared service must reject");
            assert!(error
                .to_string()
                .contains("is inactive in the definitive modpack scripts"));
            assert_eq!(session.state, before);
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_battle_result_and_fishing_swarm_apply_pack_declared_effects() {
        let root = temp_repository_root("special-battle-result-fishing-swarm");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.special_routines
            .insert("CheckCaughtCelebi".to_string());
        data.special_routines
            .insert("ActivateFishingSwarm".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session.state.battle_result = 1 << 6;

        let caught = session
            .apply_special_routine(&runtime, "CheckCaughtCelebi")
            .expect("caught celebi");

        assert_eq!(
            caught.outcome.effect,
            SpecialRoutineEffect::CheckCaughtCelebi { caught: true }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("1")
        );

        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "258".to_string());
        let swarm = session
            .apply_special_routine(&runtime, "ActivateFishingSwarm")
            .expect("activate fishing swarm");

        assert_eq!(
            swarm.outcome.effect,
            SpecialRoutineEffect::ActivateFishingSwarm { value: 2 }
        );
        assert_eq!(session.state.fishing.swarm_flag, 2);
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("258")
        );
        assert_ne!(caught.state_checksum, swarm.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_palette_and_snorlax_apply_pack_declared_effects() {
        let root = temp_repository_root("special-palette-snorlax-time");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        for routine in [
            "SetPlayerPalette",
            "SnorlaxAwake",
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
        ] {
            data.special_routines.insert(routine.to_string());
        }
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .script_runtime
            .variables
            .insert("_value".to_string(), "160".to_string());

        let palette = session
            .apply_special_routine(&runtime, "SetPlayerPalette")
            .expect("set player palette");

        assert_eq!(
            palette.outcome.effect,
            SpecialRoutineEffect::SetPlayerPalette {
                raw_value: 160,
                palette_id: 2,
                changed: true
            }
        );
        assert_eq!(session.state.player_palette_id, 2);
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("2")
        );

        session.state.script_runtime.current_music = Some("MUSIC_POKE_FLUTE_CHANNEL".to_string());
        session.state.overworld = OverworldMemory::Active {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(36, 9),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let snorlax = session
            .apply_special_routine(&runtime, "SnorlaxAwake")
            .expect("snorlax awake");

        assert_eq!(
            snorlax.outcome.effect,
            SpecialRoutineEffect::SnorlaxAwake {
                music: Some("MUSIC_POKE_FLUTE_CHANNEL".to_string()),
                tile: Some((36, 9)),
                awake: true
            }
        );
        assert_eq!(
            session.state.script_runtime.script_value.as_deref(),
            Some("1")
        );

        session.state.time.current_day = 5;
        let day = session
            .apply_special_routine(&runtime, "SetDayOfWeek")
            .expect("set day of week");

        assert_eq!(
            day.outcome.effect,
            SpecialRoutineEffect::SetDayOfWeek { day: 0 }
        );
        assert_eq!(session.state.time.day_of_week, 0);

        let dst = session
            .apply_special_routine(&runtime, "InitialSetDSTFlag")
            .expect("set dst");

        assert_eq!(dst.outcome.effect, SpecialRoutineEffect::InitialSetDstFlag);
        assert!(session.state.time.dst);

        session.state.time.start_time = ClockTime::new(2, 9, 30, 15);
        session.state.time.registers.rtc_day_lo = 3;
        session.state.time.registers.rtc_hours = 8;
        session.state.time.registers.rtc_minutes = 45;
        session.state.time.registers.rtc_seconds = 50;
        let time = session
            .apply_special_routine(&runtime, "UpdateTime")
            .expect("update time");

        assert_eq!(
            time.outcome.effect,
            SpecialRoutineEffect::UpdateTime {
                hour: 18,
                minute: 16,
                second: 5,
                day_of_week: 5,
                time_of_day: TimeOfDay::Night
            }
        );

        session.state.rng_seed = 1;
        let kenji = session
            .apply_special_routine(&runtime, "SampleKenjiBreakCountdown")
            .expect("kenji countdown");

        assert_eq!(
            kenji.outcome.effect,
            SpecialRoutineEffect::SampleKenjiBreakCountdown {
                value: 4,
                rng_seed_after: 58_598
            }
        );
        assert_eq!(session.state.kenji_break_timer, 4);
        session.state.lucky_number_show_flag = true;
        session.state.time.current_day = 4;
        session.state.rng_seed = 1;
        let lucky_flag = session
            .apply_special_routine(&runtime, "CheckLuckyNumberShowFlag")
            .expect("check lucky flag");

        assert_eq!(
            lucky_flag.outcome.effect,
            SpecialRoutineEffect::CheckLuckyNumberShowFlag { flag: true }
        );

        let lucky_reset = session
            .apply_special_routine(&runtime, "ResetLuckyNumberShowFlag")
            .expect("reset lucky flag");

        assert_eq!(
            lucky_reset.outcome.effect,
            SpecialRoutineEffect::ResetLuckyNumberShowFlag {
                lucky_number: 16_523,
                lucky_number_day: 4,
                rng_seed_after: 127_215
            }
        );
        assert!(!session.state.lucky_number_show_flag);

        let lucky_print = session
            .apply_special_routine(&runtime, "PrintTodaysLuckyNumber")
            .expect("print lucky number");

        assert_eq!(
            lucky_print.outcome.effect,
            SpecialRoutineEffect::PrintTodaysLuckyNumber {
                lucky_number: 16_523,
                formatted: "16523".to_string()
            }
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_3")
                .map(String::as_str),
            Some("16523")
        );

        let mut winner = wounded_runtime_pokemon("CHIKORITA");
        winner.original_trainer_id = 41_523;
        session
            .state
            .storage
            .register_capture(winner)
            .expect("store lucky winner");
        session.state.sync_party_from_storage();
        let lucky_winner = session
            .apply_special_routine(&runtime, "CheckForLuckyNumberWinners")
            .expect("check lucky winners");

        assert_eq!(
            lucky_winner.outcome.effect,
            SpecialRoutineEffect::CheckForLuckyNumberWinners {
                lucky_number: 16_523,
                tier: 2,
                source: Some(
                    crystal_core::systems::special_routines::LuckyNumberWinnerSource::Party
                ),
                species: Some("CHIKORITA".to_string()),
                text_label: Some("LuckyNumberMatchPartyText".to_string())
            }
        );
        session.state.money = 54_321;
        session.state.coins = 987;
        let place_money = session
            .apply_special_routine(&runtime, "PlaceMoneyTopRight")
            .expect("place money");

        assert_eq!(
            place_money.outcome.effect,
            SpecialRoutineEffect::PlaceMoneyTopRight {
                money: 54_321,
                formatted: "054321".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.money_events.len(), 1);

        let balance = session
            .apply_special_routine(&runtime, "DisplayMoneyAndCoinBalance")
            .expect("display money and coins");

        assert_eq!(
            balance.outcome.effect,
            SpecialRoutineEffect::DisplayMoneyAndCoinBalance {
                money: 54_321,
                coins: 987,
                formatted_money: "054321".to_string(),
                formatted_coins: "0987".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.money_events.len(), 2);
        assert_eq!(
            session
                .state
                .script_runtime
                .named_buffers
                .get("STRING_BUFFER_2")
                .map(String::as_str),
            Some("0987")
        );
        let coin_case = session
            .apply_special_routine(&runtime, "DisplayCoinCaseBalance")
            .expect("display coin case");

        assert_eq!(
            coin_case.outcome.effect,
            SpecialRoutineEffect::DisplayCoinCaseBalance {
                coins: 987,
                formatted_coins: "0987".to_string()
            }
        );
        assert_eq!(session.state.script_runtime.money_events.len(), 3);
        session.state.gs_healings = 9;
        let healings = session
            .apply_special_routine(&runtime, "GSHealings")
            .expect("gs healings");

        assert_eq!(
            healings.outcome.effect,
            SpecialRoutineEffect::GsHealings { healings: 9 }
        );

        session.state.trainer_rankings_healings = 11;
        let rankings = session
            .apply_special_routine(&runtime, "StubbedTrainerRankings_Healings")
            .expect("trainer rankings healings");

        assert_eq!(
            rankings.outcome.effect,
            SpecialRoutineEffect::TrainerRankingsHealings { healings: 11 }
        );

        session
            .state
            .script_runtime
            .variables
            .insert("old".to_string(), "value".to_string());
        let reset = session
            .apply_special_routine(&runtime, "Reset")
            .expect("reset");

        assert_eq!(
            reset.outcome.effect,
            SpecialRoutineEffect::Reset {
                value: "$0".to_string()
            }
        );
        assert!(session.state.script_runtime.reset_requested);
        assert_eq!(session.state.script_runtime.variables.len(), 1);

        let mut ho_oh = wounded_runtime_pokemon("HO_OH");
        ho_oh.original_trainer_id = 1234;
        session
            .state
            .storage
            .register_capture(ho_oh)
            .expect("store ho-oh");
        session.state.sync_party_from_storage();
        for flag in [
            "EVENT_UNLEASHED_SUICUNE",
            "EVENT_UNLEASHED_RAIKOU",
            "EVENT_UNLEASHED_ENTEI",
        ] {
            session
                .state
                .flags
                .set_event_flag(flag, true)
                .expect("set beast flag");
        }
        let chamber = session
            .apply_special_routine(&runtime, "HoOhChamber")
            .expect("ho-oh chamber");

        assert_eq!(
            chamber.outcome.effect,
            SpecialRoutineEffect::HoOhChamber {
                has_ho_oh: true,
                suicune_unleashed: true,
                raikou_unleashed: true,
                entei_unleashed: true,
                open: true
            }
        );
        assert_ne!(palette.state_checksum, snorlax.state_checksum);
        assert_ne!(snorlax.state_checksum, day.state_checksum);
        assert_ne!(day.state_checksum, dst.state_checksum);
        assert_ne!(time.state_checksum, kenji.state_checksum);
        assert_ne!(kenji.state_checksum, lucky_flag.state_checksum);
        assert_ne!(lucky_reset.state_checksum, lucky_print.state_checksum);
        assert_ne!(lucky_print.state_checksum, lucky_winner.state_checksum);
        assert_ne!(lucky_winner.state_checksum, place_money.state_checksum);
        assert_ne!(place_money.state_checksum, balance.state_checksum);
        assert_ne!(balance.state_checksum, coin_case.state_checksum);
        assert_ne!(coin_case.state_checksum, healings.state_checksum);
        assert_ne!(healings.state_checksum, rankings.state_checksum);
        assert_ne!(rankings.state_checksum, reset.state_checksum);
        assert_ne!(reset.state_checksum, chamber.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_special_heal_party_rejects_unknown_move_without_mutation() {
        let root = temp_repository_root("special-heal-party-move-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.special_routines.insert("HealParty".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        let mut pokemon = wounded_runtime_pokemon("CHIKORITA");
        pokemon.moves[0].name = "tackle".to_string();
        session
            .state
            .storage
            .register_capture(pokemon)
            .expect("store");
        session.state.sync_party_from_storage();
        let before = session.state.clone();

        let error = session
            .apply_special_routine(&runtime, "HealParty")
            .expect_err("unknown exact move rejected");

        assert!(error.to_string().contains("unknown move tackle"), "{error}");
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_town_map_reports_current_landmark_from_definitive_pack_data() {
        let root = temp_repository_root("town-map");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut town_map = town_map_item();
        town_map.effect = "MOD_TOWN_MAP".to_string();
        data.items.insert("TOWN_MAP".to_string(), town_map);
        add_runtime_landmark(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["TOWN_MAP"], 1)
            .expect("add town map");

        let use_item = session
            .use_bag_town_map_in_field(&runtime, "TOWN_MAP")
            .expect("use town map");

        assert_eq!(use_item.item_use.item_id, "TOWN_MAP");
        assert!(!use_item.item_use.consumed);
        assert_eq!(use_item.map_name, "RuntimeMap");
        assert_eq!(use_item.map_constant, "RUNTIME_MAP");
        assert_eq!(use_item.environment, "ROUTE");
        assert_eq!(use_item.landmark.constant, "LANDMARK_RUNTIME_TOWN");
        assert_eq!(use_item.landmark.label, "RUNTIME_TOWN");
        assert_eq!(use_item.landmark.name, "RUNTIME TOWN");
        assert_eq!(use_item.landmark.x, 12);
        assert_eq!(use_item.landmark.y, 24);
        assert_eq!(use_item.landmark.region, "JOHTO");
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["TOWN_MAP"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_town_map_rejects_wrong_effect_or_missing_landmark_without_mutation() {
        let root = temp_repository_root("town-map-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad_town_map = runtime_item("BAD_TOWN_MAP", item_pocket("KEY_ITEM"));
        bad_town_map.effect = "NONE".to_string();
        bad_town_map.field_menu = "ITEMMENU_CURRENT".to_string();
        bad_town_map.field_usable = true;
        data.items.insert("BAD_TOWN_MAP".to_string(), bad_town_map);
        data.items.insert("TOWN_MAP".to_string(), town_map_item());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_TOWN_MAP"], 1)
            .expect("add bad town map");
        session
            .state
            .bag
            .add_item(&runtime.data.items["TOWN_MAP"], 1)
            .expect("add town map");

        let before_bad = session.state.clone();
        let bad = session
            .use_bag_town_map_in_field(&runtime, "BAD_TOWN_MAP")
            .expect_err("wrong effect rejected");
        assert!(bad.to_string().contains("InvalidFieldItemId"), "{bad}");
        assert_eq!(session.state, before_bad);

        let before_missing = session.state.clone();
        let missing = session
            .use_bag_town_map_in_field(&runtime, "TOWN_MAP")
            .expect_err("missing landmark rejected");
        assert!(
            missing
                .to_string()
                .contains("missing exact landmark mapping for map RuntimeMap"),
            "{missing}"
        );
        assert_eq!(session.state, before_missing);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_town_map_rejects_missing_landmark_constant_without_mutation() {
        let root = temp_repository_root("town-map-missing-landmark");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert("TOWN_MAP".to_string(), town_map_item());
        data.pokegear_landmarks.map_to_landmark.insert(
            "RuntimeMap".to_string(),
            "LANDMARK_RUNTIME_TOWN".to_string(),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["TOWN_MAP"], 1)
            .expect("add town map");

        let before = session.state.clone();
        let error = session
            .use_bag_town_map_in_field(&runtime, "TOWN_MAP")
            .expect_err("missing landmark constant rejected");
        assert!(
            error
                .to_string()
                .contains("points to missing landmark LANDMARK_RUNTIME_TOWN"),
            "{error}"
        );
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_coin_case_reports_coin_balance_from_definitive_item_rule() {
        let root = temp_repository_root("coin-case");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut coin_case = coin_case_item();
        coin_case.effect = "MOD_COIN_CASE".to_string();
        data.items.insert("COIN_CASE".to_string(), coin_case);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["COIN_CASE"], 1)
            .expect("add coin case");
        session.state.coins = 321;

        let use_item = session
            .use_bag_coin_case_in_field(&runtime, "COIN_CASE")
            .expect("use coin case");

        assert_eq!(use_item.item_use.item_id, "COIN_CASE");
        assert_eq!(use_item.balance_label, "COIN");
        assert_eq!(use_item.balance, 321);
        assert!(!use_item.item_use.consumed);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["COIN_CASE"]),
            1
        );
        assert_eq!(session.state.coins, 321);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_blue_card_reports_exact_script_runtime_balance() {
        let root = temp_repository_root("blue-card");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut blue_card = blue_card_item();
        blue_card.effect = "MOD_BLUE_CARD".to_string();
        data.items.insert("BLUE_CARD".to_string(), blue_card);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BLUE_CARD"], 1)
            .expect("add blue card");
        session
            .state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), "12".to_string());

        let use_item = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect("use blue card");

        assert_eq!(use_item.item_use.item_id, "BLUE_CARD");
        assert_eq!(use_item.balance_label, "POINT");
        assert_eq!(use_item.balance, 12);
        assert!(!use_item.item_use.consumed);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["BLUE_CARD"]),
            1
        );
        assert_eq!(
            session
                .state
                .script_runtime
                .variables
                .get("VAR_BLUECARDBALANCE")
                .map(String::as_str),
            Some("12")
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_blue_card_missing_balance_reports_initial_zero() {
        let root = temp_repository_root("blue-card-zero");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert("BLUE_CARD".to_string(), blue_card_item());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BLUE_CARD"], 1)
            .expect("add blue card");

        let use_item = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect("use blue card with initial balance");

        assert_eq!(use_item.balance_label, "POINT");
        assert_eq!(use_item.balance, 0);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_balance_key_items_reject_wrong_effect_and_invalid_blue_card_without_mutation() {
        let root = temp_repository_root("balance-key-item-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad_coin_case = runtime_item("BAD_COIN_CASE", item_pocket("KEY_ITEM"));
        bad_coin_case.effect = "NONE".to_string();
        bad_coin_case.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_coin_case.field_usable = true;
        let mut bad_blue_card = runtime_item("BAD_BLUE_CARD", item_pocket("KEY_ITEM"));
        bad_blue_card.effect = "NONE".to_string();
        bad_blue_card.field_menu = "ITEMMENU_CLOSE".to_string();
        bad_blue_card.field_usable = true;
        data.items
            .insert("BAD_COIN_CASE".to_string(), bad_coin_case);
        data.items
            .insert("BAD_BLUE_CARD".to_string(), bad_blue_card);
        data.items.insert("BLUE_CARD".to_string(), blue_card_item());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_COIN_CASE"], 1)
            .expect("add bad coin case");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_BLUE_CARD"], 1)
            .expect("add bad blue card");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BLUE_CARD"], 1)
            .expect("add blue card");

        let before_bad_coin = session.state.clone();
        let bad_coin = session
            .use_bag_coin_case_in_field(&runtime, "BAD_COIN_CASE")
            .expect_err("wrong coin case effect rejected");
        assert!(
            bad_coin.to_string().contains("InvalidFieldItemId"),
            "{bad_coin}"
        );
        assert_eq!(session.state, before_bad_coin);

        let before_bad_blue = session.state.clone();
        let bad_blue = session
            .use_bag_blue_card_in_field(&runtime, "BAD_BLUE_CARD")
            .expect_err("wrong blue card effect rejected");
        assert!(
            bad_blue.to_string().contains("InvalidFieldItemId"),
            "{bad_blue}"
        );
        assert_eq!(session.state, before_bad_blue);

        session
            .state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), "31".to_string());
        let before_out_of_range = session.state.clone();
        let out_of_range = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect_err("out-of-range blue card balance rejected");
        assert!(
            out_of_range.to_string().contains("outside 0..=30"),
            "{out_of_range}"
        );
        assert_eq!(session.state, before_out_of_range);

        session
            .state
            .script_runtime
            .variables
            .insert("VAR_BLUECARDBALANCE".to_string(), " 12".to_string());
        let before_invalid = session.state.clone();
        let invalid = session
            .use_bag_blue_card_in_field(&runtime, "BLUE_CARD")
            .expect_err("non-exact blue card balance rejected");
        assert!(
            invalid.to_string().contains("invalid exact integer  12"),
            "{invalid}"
        );
        assert_eq!(session.state, before_invalid);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_squirtbottle_runs_facing_sudowoodo_object_script_from_pack() {
        let root = temp_repository_root("squirtbottle");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut squirtbottle = squirtbottle_item();
        squirtbottle.effect = "MOD_SQUIRTBOTTLE".to_string();
        data.items.insert("SQUIRTBOTTLE".to_string(), squirtbottle);
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut weird_tree = runtime_object("RUNTIME_WEIRD_TREE", "-1");
        weird_tree.x = 0;
        weird_tree.y = 2;
        weird_tree.spritemovedata = "SPRITEMOVEDATA_SUDOWOODO".to_string();
        weird_tree.script = "ModdedWateredTreeScript".to_string();
        map.objects = vec![weird_tree];
        map.scripts.insert(
            "ModdedWateredTreeScript".to_string(),
            serde_json::Value::Array(Vec::new()),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["SQUIRTBOTTLE"], 1)
            .expect("add squirtbottle");

        let use_item = session
            .use_bag_squirtbottle_in_field(&runtime, "SQUIRTBOTTLE")
            .expect("use squirtbottle");

        assert_eq!(use_item.item_use.item_id, "SQUIRTBOTTLE");
        assert!(!use_item.item_use.consumed);
        assert_eq!(use_item.target_tile, TilePosition::new(0, 2));
        assert_eq!(
            use_item.target_object_identifier.as_deref(),
            Some("RUNTIME_WEIRD_TREE")
        );
        assert_eq!(use_item.target_movement, "SPRITEMOVEDATA_SUDOWOODO");
        assert_eq!(
            use_item.target_script.as_deref(),
            Some("ModdedWateredTreeScript")
        );
        assert_eq!(
            session.state.script_runtime.next_script.as_deref(),
            Some("ModdedWateredTreeScript")
        );
        assert_eq!(
            session.state.script_runtime.last_talked_object.as_deref(),
            Some("RUNTIME_WEIRD_TREE")
        );
        assert_eq!(
            session.overworld.last_talked_object_identifier.as_deref(),
            Some("RUNTIME_WEIRD_TREE")
        );
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["SQUIRTBOTTLE"]),
            1
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_squirtbottle_records_nothing_path_without_target_script() {
        let root = temp_repository_root("squirtbottle-nothing");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items
            .insert("SQUIRTBOTTLE".to_string(), squirtbottle_item());
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut npc = runtime_object("RUNTIME_NPC", "-1");
        npc.x = 0;
        npc.y = 2;
        npc.spritemovedata = "SPRITEMOVEDATA_STANDING_DOWN".to_string();
        npc.script = "RuntimeNpcScript".to_string();
        map.objects = vec![npc];
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["SQUIRTBOTTLE"], 1)
            .expect("add squirtbottle");

        let use_item = session
            .use_bag_squirtbottle_in_field(&runtime, "SQUIRTBOTTLE")
            .expect("use squirtbottle");

        assert_eq!(use_item.target_script, None);
        assert_eq!(
            use_item.target_object_identifier.as_deref(),
            Some("RUNTIME_NPC")
        );
        assert_eq!(use_item.target_movement, "SPRITEMOVEDATA_STANDING_DOWN");
        assert_eq!(session.state.script_runtime.next_script, None);
        assert_eq!(session.state.script_runtime.last_talked_object, None);
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_squirtbottle_rejects_wrong_effect_and_missing_target_script_without_mutation() {
        let root = temp_repository_root("squirtbottle-reject");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut bad = runtime_item("BAD_SQUIRTBOTTLE", item_pocket("KEY_ITEM"));
        bad.effect = "NONE".to_string();
        bad.field_menu = "ITEMMENU_CLOSE".to_string();
        bad.field_usable = true;
        data.items.insert("BAD_SQUIRTBOTTLE".to_string(), bad);
        data.items
            .insert("SQUIRTBOTTLE".to_string(), squirtbottle_item());
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        let mut weird_tree = runtime_object("RUNTIME_WEIRD_TREE", "-1");
        weird_tree.x = 0;
        weird_tree.y = 2;
        weird_tree.spritemovedata = "SPRITEMOVEDATA_SUDOWOODO".to_string();
        weird_tree.script = "MissingWateredTreeScript".to_string();
        map.objects = vec![weird_tree];
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_SQUIRTBOTTLE"], 1)
            .expect("add bad squirtbottle");
        session
            .state
            .bag
            .add_item(&runtime.data.items["SQUIRTBOTTLE"], 1)
            .expect("add squirtbottle");

        let before_bad_effect = session.state.clone();
        let bad_effect = session
            .use_bag_squirtbottle_in_field(&runtime, "BAD_SQUIRTBOTTLE")
            .expect_err("wrong effect rejected");
        assert!(
            bad_effect.to_string().contains("InvalidFieldItemId"),
            "{bad_effect}"
        );
        assert_eq!(session.state, before_bad_effect);

        let before_missing_script = session.state.clone();
        let missing_script = session
            .use_bag_squirtbottle_in_field(&runtime, "SQUIRTBOTTLE")
            .expect_err("missing target script rejected");
        assert!(
            missing_script
                .to_string()
                .contains("MissingWateredTreeScript"),
            "{missing_script}"
        );
        assert_eq!(session.state, before_missing_script);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_heals_active_party_pokemon_from_exact_pack_effect() {
        let root = temp_repository_root("battle-item-heal");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.hp = 11;
        player.max_hp = 35;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 2)
            .expect("add potion");
        let before_checksum = game_state_checksum(&session.state).expect("checksum");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "POTION")
            .expect("use battle potion");

        assert_eq!(item_use.item_use.item_id, "POTION");
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.battle_item.hp_before, 11);
        assert_eq!(item_use.battle_item.hp_after, 31);
        assert_ne!(item_use.state_checksum, before_checksum);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("lead")
                .hp,
            31
        );
        assert_eq!(session.state.script_runtime.item_use_events.len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_x_item_raises_active_party_stat_stage_from_pack_data() {
        let root = temp_repository_root("battle-item-x-attack");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut x_attack = runtime_item("X_ATTACK", item_pocket("ITEM"));
        x_attack.effect = "X_ITEM".to_string();
        x_attack.battle_stat_boost_stat = Some("ATTACK".to_string());
        x_attack.battle_stat_boost_stages = Some(1);
        x_attack.field_menu = "ITEMMENU_NOUSE".to_string();
        x_attack.field_usable = false;
        x_attack.battle_menu = "ITEMMENU_CLOSE".to_string();
        x_attack.battle_usable = true;
        x_attack.consumable = true;
        data.items.insert("X_ATTACK".to_string(), x_attack);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["X_ATTACK"], 1)
            .expect("add X Attack");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "X_ATTACK")
            .expect("use X Attack");

        assert_eq!(item_use.item_use.item_id, "X_ATTACK");
        assert_eq!(
            item_use.battle_item.battle_stat_stage_changes,
            vec![crystal_core::systems::battle_items::BattleItemStageChange {
                stat: "ATTACK".to_string(),
                stage_before: 0,
                stage_after: 1,
            }]
        );
        let lead = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("lead");
        assert_eq!(lead.stat_boosts[&crystal_core::models::Stat::Attack], 1);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["X_ATTACK"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_x_item_rejects_capped_stat_without_consumption() {
        let root = temp_repository_root("battle-item-x-attack-capped");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut x_attack = runtime_item("X_ATTACK", item_pocket("ITEM"));
        x_attack.effect = "X_ITEM".to_string();
        x_attack.battle_stat_boost_stat = Some("ATTACK".to_string());
        x_attack.battle_stat_boost_stages = Some(1);
        x_attack.field_menu = "ITEMMENU_NOUSE".to_string();
        x_attack.field_usable = false;
        x_attack.battle_menu = "ITEMMENU_CLOSE".to_string();
        x_attack.battle_usable = true;
        x_attack.consumable = true;
        data.items.insert("X_ATTACK".to_string(), x_attack);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player
            .stat_boosts
            .insert(crystal_core::models::Stat::Attack, 6);
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["X_ATTACK"], 1)
            .expect("add X Attack");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "X_ATTACK")
            .expect_err("capped stat rejects X Attack");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["X_ATTACK"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_full_restore_heals_and_clears_status() {
        let root = temp_repository_root("battle-item-full-restore");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut full_restore = runtime_item("FULL_RESTORE", item_pocket("ITEM"));
        full_restore.effect = "MOD_FULL_RESTORE".to_string();
        full_restore.parameter = -1;
        full_restore.status_heals = vec![
            "POISON".to_string(),
            "BURN".to_string(),
            "FREEZE".to_string(),
            "SLEEP".to_string(),
            "PARALYSIS".to_string(),
        ];
        full_restore.field_menu = "ITEMMENU_PARTY".to_string();
        full_restore.field_usable = true;
        full_restore.battle_menu = "ITEMMENU_PARTY".to_string();
        full_restore.battle_usable = true;
        full_restore.consumable = true;
        data.items.insert("FULL_RESTORE".to_string(), full_restore);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.hp = 11;
        player.max_hp = 35;
        player.status = Some("POISON".to_string());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["FULL_RESTORE"], 1)
            .expect("add full restore");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "FULL_RESTORE")
            .expect("use full restore");

        assert_eq!(item_use.item_use.item_id, "FULL_RESTORE");
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.battle_item.hp_before, 11);
        assert_eq!(item_use.battle_item.hp_after, 35);
        assert_eq!(
            item_use.battle_item.status_before,
            Some("POISON".to_string())
        );
        assert_eq!(item_use.battle_item.status_after, None);
        let lead = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("lead");
        assert_eq!(lead.hp, 35);
        assert_eq!(lead.status, None);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["FULL_RESTORE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_status_heal_uses_exact_modpack_statuses() {
        let root = temp_repository_root("battle-item-status-heal");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut antidote = runtime_item("ANTIDOTE", item_pocket("ITEM"));
        antidote.effect = "STATUS_HEAL".to_string();
        antidote.status_heals = vec!["POISON".to_string()];
        antidote.field_menu = "ITEMMENU_PARTY".to_string();
        antidote.field_usable = true;
        antidote.battle_menu = "ITEMMENU_PARTY".to_string();
        antidote.battle_usable = true;
        antidote.consumable = true;
        data.items.insert("ANTIDOTE".to_string(), antidote);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.status = Some("POISON".to_string());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ANTIDOTE"], 1)
            .expect("add antidote");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "ANTIDOTE")
            .expect("use antidote");

        assert_eq!(item_use.item_use.item_id, "ANTIDOTE");
        assert_eq!(
            item_use.battle_item.status_before,
            Some("POISON".to_string())
        );
        assert_eq!(item_use.battle_item.status_after, None);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("lead")
                .status,
            None
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["ANTIDOTE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_status_heal_rejects_mismatched_status_without_consumption() {
        let root = temp_repository_root("battle-item-status-mismatch");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut antidote = runtime_item("ANTIDOTE", item_pocket("ITEM"));
        antidote.effect = "STATUS_HEAL".to_string();
        antidote.status_heals = vec!["POISON".to_string()];
        antidote.field_menu = "ITEMMENU_PARTY".to_string();
        antidote.field_usable = true;
        antidote.battle_menu = "ITEMMENU_PARTY".to_string();
        antidote.battle_usable = true;
        antidote.consumable = true;
        data.items.insert("ANTIDOTE".to_string(), antidote);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.status = Some("BURN".to_string());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ANTIDOTE"], 1)
            .expect("add antidote");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "ANTIDOTE")
            .expect_err("antidote does not heal burn");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["ANTIDOTE"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_revives_explicit_party_target_from_pack_percent() {
        let root = temp_repository_root("battle-item-revive");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut revive = runtime_item("REVIVE", item_pocket("ITEM"));
        revive.effect = "REVIVE".to_string();
        revive.revive_hp_percent = Some(50);
        revive.field_menu = "ITEMMENU_PARTY".to_string();
        revive.field_usable = true;
        revive.battle_menu = "ITEMMENU_PARTY".to_string();
        revive.battle_usable = true;
        revive.consumable = true;
        data.items.insert("REVIVE".to_string(), revive);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fainted = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        fainted.hp = 0;
        fainted.max_hp = 35;
        let mut active = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        active.hp = 22;
        active.max_hp = 35;
        session
            .state
            .storage
            .register_capture(fainted)
            .expect("register fainted");
        session
            .state
            .storage
            .register_capture(active)
            .expect("register active");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REVIVE"], 1)
            .expect("add revive");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        assert_eq!(session.state.battle_active_party_index, Some(1));

        let item_use = session
            .use_bag_item_on_battle_party_pokemon(&runtime, "REVIVE", 0)
            .expect("use revive");

        assert_eq!(item_use.item_use.item_id, "REVIVE");
        assert_eq!(item_use.battle_item.hp_before, 0);
        assert_eq!(item_use.battle_item.hp_after, 17);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("revived")
                .hp,
            17
        );
        assert_eq!(session.state.bag.quantity(&runtime.data.items["REVIVE"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_revive_rejects_non_fainted_target_without_consumption() {
        let root = temp_repository_root("battle-item-revive-healthy");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut revive = runtime_item("REVIVE", item_pocket("ITEM"));
        revive.effect = "REVIVE".to_string();
        revive.revive_hp_percent = Some(50);
        revive.field_menu = "ITEMMENU_PARTY".to_string();
        revive.field_usable = true;
        revive.battle_menu = "ITEMMENU_PARTY".to_string();
        revive.battle_usable = true;
        revive.consumable = true;
        data.items.insert("REVIVE".to_string(), revive);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REVIVE"], 1)
            .expect("add revive");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_battle_party_pokemon(&runtime, "REVIVE", 0)
            .expect_err("revive cannot target healthy Pokemon");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["REVIVE"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_heals_party_pokemon_from_exact_pack_effect() {
        let root = temp_repository_root("field-item-potion");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.hp = 11;
        player.max_hp = 35;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 1)
            .expect("add potion");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "POTION", 0)
            .expect("use field potion");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "POTION");
        assert_eq!(item_use.item_effect.hp_before, 11);
        assert_eq!(item_use.item_effect.hp_after, 31);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .hp,
            31
        );
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_status_heal_uses_exact_modpack_statuses() {
        let root = temp_repository_root("field-item-status");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut antidote = runtime_item("ANTIDOTE", item_pocket("ITEM"));
        antidote.effect = "STATUS_HEAL".to_string();
        antidote.status_heals = vec!["POISON".to_string()];
        antidote.field_menu = "ITEMMENU_PARTY".to_string();
        antidote.field_usable = true;
        antidote.battle_menu = "ITEMMENU_PARTY".to_string();
        antidote.battle_usable = true;
        antidote.consumable = true;
        data.items.insert("ANTIDOTE".to_string(), antidote);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.status = Some("POISON".to_string());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ANTIDOTE"], 1)
            .expect("add antidote");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "ANTIDOTE", 0)
            .expect("use field antidote");

        assert_eq!(
            item_use.item_effect.status_before,
            Some("POISON".to_string())
        );
        assert_eq!(item_use.item_effect.status_after, None);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .status,
            None
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["ANTIDOTE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_revives_explicit_party_target_from_pack_percent() {
        let root = temp_repository_root("field-item-revive");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut revive = runtime_item("REVIVE", item_pocket("ITEM"));
        revive.effect = "REVIVE".to_string();
        revive.revive_hp_percent = Some(50);
        revive.field_menu = "ITEMMENU_PARTY".to_string();
        revive.field_usable = true;
        revive.battle_menu = "ITEMMENU_PARTY".to_string();
        revive.battle_usable = true;
        revive.consumable = true;
        data.items.insert("REVIVE".to_string(), revive);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fainted = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        fainted.hp = 0;
        fainted.max_hp = 35;
        session
            .state
            .storage
            .register_capture(fainted)
            .expect("register fainted");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["REVIVE"], 1)
            .expect("add revive");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "REVIVE", 0)
            .expect("use field revive");

        assert_eq!(item_use.item_effect.hp_before, 0);
        assert_eq!(item_use.item_effect.hp_after, 17);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("revived")
                .hp,
            17
        );
        assert_eq!(session.state.bag.quantity(&runtime.data.items["REVIVE"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rejects_full_hp_without_consumption() {
        let root = temp_repository_root("field-item-full-hp");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 1)
            .expect("add potion");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "POTION", 0)
            .expect_err("full HP has no target change");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_vitamin_raises_party_stat_exp_from_pack_data() {
        let root = temp_repository_root("field-item-vitamin");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut protein = runtime_item("PROTEIN", item_pocket("ITEM"));
        protein.effect = "VITAMIN".to_string();
        protein.vitamin_stat = Some("ATTACK".to_string());
        protein.vitamin_stat_exp = Some(2560);
        protein.vitamin_max_stat_exp = Some(25600);
        protein.field_menu = "ITEMMENU_PARTY".to_string();
        protein.field_usable = true;
        protein.battle_menu = "ITEMMENU_NOUSE".to_string();
        protein.battle_usable = false;
        protein.consumable = true;
        data.items.insert("PROTEIN".to_string(), protein);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 30, Dv::default());
        player.attack_exp = 0;
        let attack_before = player.attack;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PROTEIN"], 1)
            .expect("add protein");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "PROTEIN", 0)
            .expect("use field protein");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "PROTEIN");
        assert_eq!(item_use.item_effect.stat_changes.len(), 1);
        assert_eq!(item_use.item_effect.stat_changes[0].stat, "ATTACK");
        assert_eq!(item_use.item_effect.stat_changes[0].stat_exp_before, 0);
        assert_eq!(item_use.item_effect.stat_changes[0].stat_exp_after, 2560);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.attack_exp, 2560);
        assert!(pokemon.attack >= attack_before);
        assert_eq!(
            item_use.item_effect.stat_changes[0].stat_after,
            pokemon.attack
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["PROTEIN"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_vitamin_rejects_maxed_stat_exp_without_consumption() {
        let root = temp_repository_root("field-item-vitamin-maxed");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut protein = runtime_item("PROTEIN", item_pocket("ITEM"));
        protein.effect = "VITAMIN".to_string();
        protein.vitamin_stat = Some("ATTACK".to_string());
        protein.vitamin_stat_exp = Some(2560);
        protein.vitamin_max_stat_exp = Some(25600);
        protein.field_menu = "ITEMMENU_PARTY".to_string();
        protein.field_usable = true;
        protein.battle_menu = "ITEMMENU_NOUSE".to_string();
        protein.battle_usable = false;
        protein.consumable = true;
        data.items.insert("PROTEIN".to_string(), protein);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 30, Dv::default());
        player.attack_exp = 25600;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PROTEIN"], 1)
            .expect("add protein");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "PROTEIN", 0)
            .expect_err("maxed vitamin target rejects item");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["PROTEIN"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rare_candy_levels_party_pokemon_from_pack_data() {
        let root = temp_repository_root("field-item-rare-candy");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut rare_candy = runtime_item("RARE_CANDY", item_pocket("ITEM"));
        rare_candy.effect = "MOD_CANDY".to_string();
        rare_candy.rare_candy_level_gain = Some(1);
        rare_candy.field_menu = "ITEMMENU_PARTY".to_string();
        rare_candy.field_usable = true;
        rare_candy.battle_menu = "ITEMMENU_NOUSE".to_string();
        rare_candy.battle_usable = false;
        rare_candy.consumable = true;
        data.items.insert("RARE_CANDY".to_string(), rare_candy);
        data.moves
            .insert("GROWL".to_string(), runtime_move_named("GROWL", 40));
        data.learnsets.insert(
            "CHIKORITA".to_string(),
            vec![
                LearnsetEntry(1, "TACKLE".to_string()),
                LearnsetEntry(10, "GROWL".to_string()),
            ],
        );
        data.evolutions
            .0
            .insert("CHIKORITA".to_string(), Vec::new());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 9, Dv::default());
        player.species.growth_rate = growth_rate("GROWTH_MEDIUM_FAST");
        player.experience =
            calculate_experience(&runtime.data.growth_rates, "GROWTH_MEDIUM_FAST", 9).unwrap();
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["RARE_CANDY"], 1)
            .expect("add rare candy");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "RARE_CANDY", 0)
            .expect("use rare candy");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "RARE_CANDY");
        assert_eq!(item_use.item_effect.level_before, 9);
        assert_eq!(item_use.item_effect.level_after, 10);
        assert_eq!(
            item_use.item_effect.learned_moves,
            vec!["GROWL".to_string()]
        );
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.level, 10);
        assert_eq!(
            pokemon.experience,
            calculate_experience(&runtime.data.growth_rates, "GROWTH_MEDIUM_FAST", 10).unwrap()
        );
        assert_eq!(pokemon.moves[1].name, "GROWL");
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["RARE_CANDY"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rare_candy_rejects_max_level_without_consumption() {
        let root = temp_repository_root("field-item-rare-candy-maxed");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut rare_candy = runtime_item("RARE_CANDY", item_pocket("ITEM"));
        rare_candy.effect = "RARE_CANDY".to_string();
        rare_candy.rare_candy_level_gain = Some(1);
        rare_candy.field_menu = "ITEMMENU_PARTY".to_string();
        rare_candy.field_usable = true;
        rare_candy.battle_menu = "ITEMMENU_NOUSE".to_string();
        rare_candy.battle_usable = false;
        rare_candy.consumable = true;
        data.items.insert("RARE_CANDY".to_string(), rare_candy);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 100, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["RARE_CANDY"], 1)
            .expect("add rare candy");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "RARE_CANDY", 0)
            .expect_err("max level rejects rare candy");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["RARE_CANDY"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_evolution_stone_evolves_party_pokemon_from_pack_tables() {
        let root = temp_repository_root("field-item-evo-stone");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut thunderstone = runtime_item("THUNDERSTONE", item_pocket("ITEM"));
        thunderstone.effect = "MOD_STONE".to_string();
        thunderstone.field_menu = "ITEMMENU_PARTY".to_string();
        thunderstone.field_usable = true;
        thunderstone.battle_menu = "ITEMMENU_NOUSE".to_string();
        thunderstone.battle_usable = false;
        thunderstone.consumable = true;
        data.items.insert("THUNDERSTONE".to_string(), thunderstone);
        data.pokemon.insert(
            "PIKACHU".to_string(),
            PokemonSpecies::new_for_tests("PIKACHU", BaseStats::new(35, 55, 30, 90, 50, 50)),
        );
        data.pokemon.insert(
            "RAICHU".to_string(),
            PokemonSpecies::new_for_tests("RAICHU", BaseStats::new(60, 90, 55, 100, 90, 80)),
        );
        data.moves.insert(
            "THUNDERBOLT".to_string(),
            runtime_move_named("THUNDERBOLT", 15),
        );
        data.learnsets.insert(
            "RAICHU".to_string(),
            vec![LearnsetEntry(20, "THUNDERBOLT".to_string())],
        );
        data.evolutions.0.insert(
            "PIKACHU".to_string(),
            vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player =
            Pokemon::new_for_tests(runtime.data.pokemon["PIKACHU"].clone(), 20, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["THUNDERSTONE"], 1)
            .expect("add thunderstone");

        let item_use = session
            .use_bag_item_on_party_pokemon(&runtime, "THUNDERSTONE", 0)
            .expect("use thunderstone");

        assert_eq!(item_use.item_effect.item_id, "THUNDERSTONE");
        assert_eq!(
            item_use.item_effect.evolution_target,
            Some("RAICHU".to_string())
        );
        assert_eq!(
            item_use.item_effect.learned_moves,
            vec!["THUNDERBOLT".to_string()]
        );
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.species.id, "RAICHU");
        assert_eq!(pokemon.moves[0].name, "THUNDERBOLT");
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["THUNDERSTONE"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_evolution_stone_rejects_wrong_stone_without_consumption() {
        let root = temp_repository_root("field-item-evo-stone-wrong");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut fire_stone = runtime_item("FIRE_STONE", item_pocket("ITEM"));
        fire_stone.effect = "MOD_STONE".to_string();
        fire_stone.field_menu = "ITEMMENU_PARTY".to_string();
        fire_stone.field_usable = true;
        fire_stone.battle_menu = "ITEMMENU_NOUSE".to_string();
        fire_stone.battle_usable = false;
        fire_stone.consumable = true;
        data.items.insert("FIRE_STONE".to_string(), fire_stone);
        data.pokemon.insert(
            "PIKACHU".to_string(),
            PokemonSpecies::new_for_tests("PIKACHU", BaseStats::new(35, 55, 30, 90, 50, 50)),
        );
        data.pokemon.insert(
            "RAICHU".to_string(),
            PokemonSpecies::new_for_tests("RAICHU", BaseStats::new(60, 90, 55, 100, 90, 80)),
        );
        data.learnsets.insert("RAICHU".to_string(), Vec::new());
        data.evolutions.0.insert(
            "PIKACHU".to_string(),
            vec![EvolutionEntry::item("RAICHU", "THUNDERSTONE")],
        );
        data.evolutions.0.insert(
            "VULPIX".to_string(),
            vec![EvolutionEntry::item("NINETALES", "FIRE_STONE")],
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player =
            Pokemon::new_for_tests(runtime.data.pokemon["PIKACHU"].clone(), 20, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["FIRE_STONE"], 1)
            .expect("add fire stone");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_pokemon(&runtime, "FIRE_STONE", 0)
            .expect_err("wrong stone has no target change");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["FIRE_STONE"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_sacred_ash_revives_whole_party_from_pack_percent() {
        let root = temp_repository_root("field-item-sacred-ash");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut sacred_ash = runtime_item("MOD_ASH", item_pocket("ITEM"));
        sacred_ash.effect = "MOD_ASH".to_string();
        sacred_ash.party_revive_hp_percent = Some(100);
        sacred_ash.field_menu = "ITEMMENU_CLOSE".to_string();
        sacred_ash.field_usable = true;
        sacred_ash.battle_menu = "ITEMMENU_NOUSE".to_string();
        sacred_ash.battle_usable = false;
        sacred_ash.consumable = true;
        data.items.insert("MOD_ASH".to_string(), sacred_ash);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut fainted = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        fainted.hp = 0;
        fainted.max_hp = 35;
        let mut healthy = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        healthy.hp = 12;
        healthy.max_hp = 40;
        session
            .state
            .storage
            .register_capture(fainted)
            .expect("register fainted");
        session
            .state
            .storage
            .register_capture(healthy)
            .expect("register healthy");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["MOD_ASH"], 1)
            .expect("add Sacred Ash");

        let item_use = session
            .use_bag_item_on_whole_party(&runtime, "MOD_ASH")
            .expect("use Sacred Ash");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.item_id, "MOD_ASH");
        assert_eq!(item_use.item_effect.revive_changes.len(), 1);
        assert_eq!(item_use.item_effect.revive_changes[0].party_index, 0);
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("slot 0")
                .hp,
            35
        );
        assert_eq!(
            session.state.storage.party.pokemon[1]
                .as_ref()
                .expect("slot 1")
                .hp,
            12
        );
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["MOD_ASH"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_sacred_ash_rejects_no_fainted_party_without_consumption() {
        let root = temp_repository_root("field-item-sacred-ash-no-target");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut sacred_ash = runtime_item("MOD_ASH", item_pocket("ITEM"));
        sacred_ash.effect = "MOD_ASH".to_string();
        sacred_ash.party_revive_hp_percent = Some(100);
        sacred_ash.field_menu = "ITEMMENU_CLOSE".to_string();
        sacred_ash.field_usable = true;
        sacred_ash.battle_menu = "ITEMMENU_NOUSE".to_string();
        sacred_ash.battle_usable = false;
        sacred_ash.consumable = true;
        data.items.insert("MOD_ASH".to_string(), sacred_ash);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let healthy = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(healthy)
            .expect("register healthy");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["MOD_ASH"], 1)
            .expect("add Sacred Ash");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_whole_party(&runtime, "MOD_ASH")
            .expect_err("no fainted target rejects Sacred Ash");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(
            session.state.bag.quantity(&runtime.data.items["MOD_ASH"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_restores_selected_move_pp_from_compiled_moves() {
        let root = temp_repository_root("field-item-ether");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("GROWL".to_string(), runtime_move_named("GROWL", 40));
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 20,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
        ];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");

        let item_use = session
            .use_bag_item_on_party_move(&runtime, "ETHER", 0, Some(0))
            .expect("use field ether");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert_eq!(item_use.item_effect.pp_changes.len(), 1);
        assert_eq!(item_use.item_effect.pp_changes[0].move_id, "TACKLE");
        assert_eq!(item_use.item_effect.pp_changes[0].pp_before, 20);
        assert_eq!(item_use.item_effect.pp_changes[0].pp_after, 30);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.moves[0].current_pp, 30);
        assert_eq!(pokemon.moves[1].current_pp, 1);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_rejects_full_pp_without_consumption() {
        let root = temp_repository_root("field-item-ether-full");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_move(&runtime, "ETHER", 0, Some(0))
            .expect_err("full PP has no target change");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tm_teaches_explicit_move_and_consumes_tm_flag() {
        let root = temp_repository_root("field-item-tm-headbutt");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("HEADBUTT".to_string(), runtime_move_named("HEADBUTT", 15));
        let mut tm = runtime_item("TM_HEADBUTT", item_pocket("TM_HM"));
        tm.field_menu = "ITEMMENU_PARTY".to_string();
        tm.field_usable = true;
        tm.consumable = true;
        tm.tmhm_index = Some(1);
        tm.tmhm_move = Some("HEADBUTT".to_string());
        data.items.insert("TM_HEADBUTT".to_string(), tm);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.species.tmhm_learnset = vec!["HEADBUTT".to_string()];
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["TM_HEADBUTT"], 1)
            .expect("add TM");

        let item_use = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "TM_HEADBUTT", 0, None)
            .expect("teach TM");

        assert_eq!(item_use.item_use.context, ItemUseContext::Field);
        assert!(item_use.item_use.consumed);
        assert_eq!(item_use.learned_move.learned_move, "HEADBUTT");
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .moves
                .last()
                .expect("learned")
                .current_pp,
            15
        );
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["TM_HEADBUTT"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_hm_teaches_explicit_move_without_consuming_hm_flag() {
        let root = temp_repository_root("field-item-hm-cut");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("CUT".to_string(), runtime_move_named("CUT", 30));
        let mut hm = runtime_item("HM_CUT", item_pocket("TM_HM"));
        hm.field_menu = "ITEMMENU_PARTY".to_string();
        hm.field_usable = true;
        hm.consumable = false;
        hm.tmhm_index = Some(50);
        hm.tmhm_move = Some("CUT".to_string());
        data.items.insert("HM_CUT".to_string(), hm);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.species.tmhm_learnset = vec!["CUT".to_string()];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["HM_CUT"], 1)
            .expect("add HM");

        let item_use = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "HM_CUT", 0, None)
            .expect("teach HM");

        assert!(!item_use.item_use.consumed);
        assert_eq!(item_use.learned_move.learned_move, "CUT");
        assert_eq!(session.state.bag.quantity(&runtime.data.items["HM_CUT"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_cut_field_move_replaces_block_and_persists_override() {
        let root = temp_repository_root("field-move-cut");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "5b": ["CUT_TREE", "CUT_TREE", "CUT_TREE", "CUT_TREE"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x5b, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "CUT".to_string(),
            current_pp: 30,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[1] = true;

        let field_move = session
            .use_cut_field_move(&runtime, 0, 0, 0)
            .expect("use cut");

        assert_eq!(field_move.outcome.move_id, "CUT");
        assert_eq!(field_move.outcome.previous_block_id, 0x5b);
        assert_eq!(field_move.outcome.replacement_block_id, 0x3c);
        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x3c));
        assert_eq!(
            session
                .state
                .map_block_overrides
                .get("RuntimeMap")
                .and_then(|overrides| overrides.get(&(0, 0)))
                .copied(),
            Some(0x3c)
        );

        let resumed =
            RuntimeOverworldSession::from_state(&runtime, &asset_root, session.state.clone())
                .expect("resume with cut override");
        assert_eq!(resumed.overworld.map.metatile_at(0, 0), Some(0x3c));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_cut_field_move_rejects_missing_badge_without_mutation() {
        let root = temp_repository_root("field-move-cut-no-badge");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "5b": ["CUT_TREE", "CUT_TREE", "CUT_TREE", "CUT_TREE"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x5b, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "CUT".to_string(),
            current_pp: 30,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        let before = session.state.clone();

        let error = session
            .use_cut_field_move(&runtime, 0, 0, 0)
            .expect_err("missing badge rejects cut");

        assert!(error.to_string().contains("MissingBadge"));
        assert_eq!(session.state, before);
        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x5b));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_whirlpool_field_move_replaces_block() {
        let root = temp_repository_root("field-move-whirlpool");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "07": ["WHIRLPOOL", "WHIRLPOOL", "WHIRLPOOL", "WHIRLPOOL"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x07, 0x00];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "WHIRLPOOL".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[6] = true;

        let field_move = session
            .use_whirlpool_field_move(&runtime, 0, 0, 0)
            .expect("use whirlpool");

        assert_eq!(field_move.outcome.move_id, "WHIRLPOOL");
        assert_eq!(field_move.outcome.replacement_block_id, 0x36);
        assert_eq!(field_move.outcome.variant, "whirlpool");
        assert_eq!(session.overworld.map.metatile_at(0, 0), Some(0x36));
        assert_eq!(
            session
                .state
                .map_block_overrides
                .get("RuntimeMap")
                .and_then(|overrides| overrides.get(&(0, 0)))
                .copied(),
            Some(0x36)
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_strength_and_flash_set_exact_engine_flags() {
        let root = temp_repository_root("field-move-flags");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut strength_user = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        strength_user.moves = vec![LearnedMove {
            name: "STRENGTH".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        let mut flash_user = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        flash_user.moves = vec![LearnedMove {
            name: "FLASH".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(strength_user)
            .expect("register strength user");
        session
            .state
            .storage
            .register_capture(flash_user)
            .expect("register flash user");
        session.state.sync_party_from_storage();
        session.state.badges.johto[2] = true;
        session.state.badges.johto[0] = true;

        let strength = session
            .use_strength_field_move(&runtime, 0)
            .expect("use strength");
        let flash = session
            .use_flash_field_move(&runtime, 1)
            .expect("use flash");

        assert_eq!(strength.outcome.engine_flag, "ENGINE_STRENGTH_ACTIVE");
        assert_eq!(flash.outcome.engine_flag, "STATUSFLAGS_FLASH");
        assert_eq!(
            session
                .state
                .flags
                .is_engine_flag_set("ENGINE_STRENGTH_ACTIVE"),
            Ok(true)
        );
        assert_eq!(
            session.state.flags.is_engine_flag_set("STATUSFLAGS_FLASH"),
            Ok(true)
        );
        assert_ne!(strength.state_checksum, flash.state_checksum);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_surf_field_move_enters_water_and_updates_saved_overworld() {
        let root = temp_repository_root("field-move-surf");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "08": ["WATER", "WATER", "WATER", "WATER"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x00, 0x08];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SURF".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[3] = true;
        session.overworld.player.facing = Direction::Right;

        let surf = session.use_surf_field_move(&runtime, 0).expect("use surf");

        assert_eq!(surf.outcome.from_tile, TilePosition::new(0, 0));
        assert_eq!(surf.outcome.to_tile, TilePosition::new(2, 0));
        assert_eq!(session.overworld.player.mode, MovementMode::Surf);
        assert_eq!(session.overworld.player.tile, TilePosition::new(2, 0));
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_surf_field_move_rejects_occupied_target_without_moving() {
        let root = temp_repository_root("field-move-surf-occupied");
        write_tileset(
            &root,
            "johto",
            r#"{
  "00": [0, 0, 0, 0],
  "08": ["WATER", "WATER", "WATER", "WATER"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.blocks = vec![0x00, 0x08];
        module.objects = vec![ObjectEvent {
            x: 2,
            y: 0,
            object_identifier: Some("SURF_BLOCKER".to_string()),
            ..runtime_object("SURF_BLOCKER", "-1")
        }];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SURF".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[3] = true;
        session.overworld.player.facing = Direction::Right;
        let before = session.state.clone();

        let error = session
            .use_surf_field_move(&runtime, 0)
            .expect_err("occupied target rejects surf");

        assert!(error.to_string().contains("occupied tile"));
        assert_eq!(session.state, before);
        assert_eq!(session.overworld.player.mode, MovementMode::Normal);
        assert_eq!(session.overworld.player.tile, TilePosition::new(0, 0));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_waterfall_field_move_climbs_and_updates_saved_overworld() {
        let root = temp_repository_root("field-move-waterfall");
        write_tileset(
            &root,
            "johto",
            r#"{
  "08": ["WATER", "WATER", "WATER", "WATER"],
  "09": ["WATERFALL", "WATERFALL", "WATERFALL", "WATERFALL"]
}"#,
        );
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let module = data.maps.get_mut("RuntimeMap").expect("runtime map");
        module.attributes.width = 1;
        module.attributes.height = 4;
        module.blocks = vec![0x08, 0x09, 0x09, 0x08];
        data.map_attributes
            .insert("RuntimeMap".to_string(), module.attributes.clone());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "WATERFALL".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[7] = true;
        session.overworld.player.tile = TilePosition::new(1, 7);
        session.overworld.player.facing = Direction::Up;
        session.overworld.player.mode = MovementMode::Surf;

        let waterfall = session
            .use_waterfall_field_move(&runtime, 0)
            .expect("use waterfall");

        assert_eq!(waterfall.outcome.steps, 3);
        assert_eq!(waterfall.outcome.from_tile, TilePosition::new(1, 7));
        assert_eq!(waterfall.outcome.to_tile, TilePosition::new(1, 1));
        assert_eq!(session.overworld.player.mode, MovementMode::Surf);
        assert_eq!(session.overworld.player.tile, TilePosition::new(1, 1));
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fly_field_move_transitions_to_exact_spawn_and_updates_saved_overworld() {
        let root = temp_repository_root("field-move-fly");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_fly_destination(&mut data);
        data.field_moves.fly.badge = field_move_badge(0);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "FLY".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[0] = true;
        session
            .state
            .flags
            .set_engine_flag("ENGINE_FLYPOINT_NEW_BARK", true)
            .expect("set flypoint flag");

        let fly = session
            .use_fly_field_move(&runtime, &asset_root, 0, 14, "ENGINE_FLYPOINT_NEW_BARK")
            .expect("use fly");

        assert_eq!(fly.actor_party_index, 0);
        assert_eq!(fly.actor_species, "CHIKORITA");
        assert_eq!(fly.flypoint_flag, "ENGINE_FLYPOINT_NEW_BARK");
        assert_eq!(fly.source_map, "RuntimeMap");
        assert_eq!(fly.destination_spawn_identifier, 14);
        assert_eq!(fly.destination_map, "FlyMap");
        assert_eq!(fly.destination_tile, TilePosition::new(0, 0));
        assert_eq!(session.overworld.map.name, "FlyMap");
        assert_eq!(session.overworld.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.overworld.player.facing, Direction::Down);
        assert_eq!(session.overworld.player.mode, MovementMode::Normal);
        assert_eq!(session.state.last_spawn_identifier, Some(14));
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fly_field_move_rejects_unset_flypoint_without_transition() {
        let root = temp_repository_root("field-move-fly-unset-flag");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_fly_destination(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "FLY".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[5] = true;
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_fly_field_move(&runtime, &asset_root, 0, 14, "ENGINE_FLYPOINT_NEW_BARK")
            .expect_err("unset flypoint rejects fly");

        assert!(error.to_string().contains("destination flag"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_fly_field_move_rejects_non_overworld_environment_without_transition() {
        let root = temp_repository_root("field-move-fly-cave");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_fly_destination(&mut data);
        data.runtime_map_metadata.insert(
            "RUNTIME_MAP".to_string(),
            runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "CAVE"),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "FLY".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.badges.johto[5] = true;
        session
            .state
            .flags
            .set_engine_flag("ENGINE_FLYPOINT_NEW_BARK", true)
            .expect("set flypoint flag");
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_fly_field_move(&runtime, &asset_root, 0, 14, "ENGINE_FLYPOINT_NEW_BARK")
            .expect_err("cave rejects fly");

        assert!(error.to_string().contains("environment CAVE"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_teleport_field_move_transitions_to_saved_spawn_without_fallback() {
        let root = temp_repository_root("field-move-teleport");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_teleport_destination(&mut data);
        data.field_moves.teleport.move_id = "DIG".to_string();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "DIG".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.last_spawn_identifier = Some(21);

        let teleport = session
            .use_teleport_field_move(&runtime, &asset_root, 0)
            .expect("use teleport");

        assert_eq!(teleport.actor_party_index, 0);
        assert_eq!(teleport.actor_species, "CHIKORITA");
        assert_eq!(teleport.source_map, "RuntimeMap");
        assert_eq!(teleport.destination_spawn_identifier, 21);
        assert_eq!(teleport.destination_map, "TeleportMap");
        assert_eq!(teleport.destination_tile, TilePosition::new(1, 0));
        assert_eq!(session.overworld.map.name, "TeleportMap");
        assert_eq!(session.overworld.player.tile, TilePosition::new(1, 0));
        assert_eq!(session.state.last_spawn_identifier, Some(21));
        assert_eq!(
            session.state.overworld,
            OverworldMemory::from_snapshot(&session.overworld.snapshot())
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_teleport_field_move_rejects_missing_saved_spawn_without_transition() {
        let root = temp_repository_root("field-move-teleport-missing-spawn");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TELEPORT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.last_spawn_identifier = None;
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_teleport_field_move(&runtime, &asset_root, 0)
            .expect_err("missing saved spawn rejects teleport");

        assert!(error
            .to_string()
            .contains("TELEPORT field move has no saved spawn identifier"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_teleport_field_move_rejects_invalid_environment_without_transition() {
        let root = temp_repository_root("field-move-teleport-cave");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_teleport_destination(&mut data);
        data.runtime_map_metadata.insert(
            "RUNTIME_MAP".to_string(),
            runtime_map_metadata("RUNTIME_MAP", "RuntimeMap", 1, 1, "CAVE"),
        );
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TELEPORT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.last_spawn_identifier = Some(21);
        let before_state = session.state.clone();
        let before_snapshot = session.overworld.snapshot();

        let error = session
            .use_teleport_field_move(&runtime, &asset_root, 0)
            .expect_err("cave rejects teleport");

        assert!(error.to_string().contains("environment CAVE"));
        assert_eq!(session.state, before_state);
        assert_eq!(session.overworld.snapshot(), before_snapshot);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_headbutt_field_move_uses_exact_field_encounter_table() {
        let root = temp_repository_root("field-move-headbutt");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_field_encounters(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.overworld.player.facing = Direction::Down;
        session.state.rng_seed = 1;

        let use_result = session
            .use_headbutt_field_move(&runtime, 0, 0)
            .expect("use headbutt");

        assert_eq!(
            use_result.field_encounter.kind,
            FieldEncounterKind::Headbutt
        );
        assert_eq!(use_result.field_encounter.score, Some(0));
        assert_eq!(use_result.field_encounter.chance_roll, 2);
        assert_eq!(use_result.field_encounter.entry_roll, Some(54));
        let battle = use_result.wild_battle.expect("headbutt battle");
        assert_eq!(battle.enemy_pokemon.species.id, "CHIKORITA");
        assert_eq!(battle.enemy_pokemon.level, 12);
        assert!(matches!(session.state.battle, BattleMemory::Wild { .. }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_rock_smash_field_move_uses_exact_field_encounter_table() {
        let root = temp_repository_root("field-move-rock-smash");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_field_encounters(&mut data);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "ROCK_SMASH".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.rng_seed = 1;

        let use_result = session
            .use_rock_smash_field_move(&runtime, 0)
            .expect("use rock smash");

        assert_eq!(
            use_result.field_encounter.kind,
            FieldEncounterKind::RockSmash
        );
        assert_eq!(use_result.field_encounter.chance_roll, 2);
        assert_eq!(use_result.field_encounter.entry_roll, Some(54));
        let battle = use_result.wild_battle.expect("rock smash battle");
        assert_eq!(battle.enemy_pokemon.species.id, "CHIKORITA");
        assert_eq!(battle.enemy_pokemon.level, 15);
        assert!(matches!(session.state.battle, BattleMemory::Wild { .. }));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_headbutt_field_move_rejects_missing_field_encounters_without_rng_change() {
        let root = temp_repository_root("field-move-headbutt-missing-table");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let data = minimal_runtime_data_with_scripted_battles();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.rng_seed = 1;
        let before = session.state.clone();

        let error = session
            .use_headbutt_field_move(&runtime, 0, 0)
            .expect_err("missing field encounters reject headbutt");

        assert!(error.to_string().contains("missing field encounters"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_headbutt_field_move_rejects_present_map_missing_table_without_rng_change() {
        let root = temp_repository_root("field-move-headbutt-present-map-missing-table");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_field_encounters(&mut data);
        data.field_encounters
            .get_mut("RuntimeMap")
            .expect("RuntimeMap field encounters")
            .headbutt = None;
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.rng_seed = 1;
        let before = session.state.clone();

        let error = session
            .use_headbutt_field_move(&runtime, 0, 0)
            .expect_err("present map missing headbutt table");

        assert!(error
            .to_string()
            .contains("Headbutt field encounter table for map 'RuntimeMap' is missing"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_headbutt_field_move_rejects_empty_selected_bucket_without_rng_change() {
        let root = temp_repository_root("field-move-headbutt-empty-rare");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        add_runtime_field_encounters(&mut data);
        data.field_encounters
            .get_mut("RuntimeMap")
            .expect("RuntimeMap field encounters")
            .headbutt
            .as_mut()
            .expect("headbutt table")
            .rare
            .clear();
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "HEADBUTT".to_string(),
            current_pp: 15,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.overworld.player.facing = Direction::Down;
        session.state.rng_seed = 1;
        let before = session.state.clone();

        let error = session
            .use_headbutt_field_move(&runtime, 0, 0)
            .expect_err("empty selected rare bucket");

        assert!(error.to_string().contains(
            "Headbutt field encounter table for map 'RuntimeMap' has no entries in bucket 'rare'"
        ));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_sweet_scent_field_move_starts_exact_surface_wild_battle() {
        let root = temp_repository_root("field-move-sweet-scent");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_grass_encounter(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SWEET_SCENT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.rng_seed = 1;

        let use_result = session
            .use_sweet_scent_field_move(&runtime, 0, EncounterSurface::Grass)
            .expect("use sweet scent");

        assert_eq!(use_result.actor_party_index, 0);
        assert_eq!(use_result.actor_species, "CHIKORITA");
        assert_eq!(use_result.wild_encounter.map_name, "RuntimeMap");
        assert_eq!(use_result.wild_encounter.surface, EncounterSurface::Grass);
        assert_eq!(use_result.wild_encounter.threshold, 255);
        assert_eq!(use_result.wild_encounter.encounter_roll, 0);
        assert_eq!(use_result.wild_encounter.slot_percent_roll, Some(26));
        assert_eq!(use_result.wild_encounter.level_roll, Some(139));
        let resolved = use_result
            .wild_encounter
            .resolved
            .clone()
            .expect("resolved");
        assert_eq!(resolved.encounter.species, "CHIKORITA");
        assert_eq!(resolved.level, 3);
        assert_eq!(use_result.wild_battle.enemy_pokemon.species.id, "CHIKORITA");
        assert_eq!(use_result.wild_battle.enemy_pokemon.level, 3);
        assert_eq!(use_result.wild_battle.encounter, use_result.wild_encounter);
        assert!(matches!(session.state.battle, BattleMemory::Wild { .. }));
        assert_eq!(
            use_result.wild_battle.rng_seed_after,
            session.state.rng_seed
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_sweet_scent_field_move_rejects_missing_surface_without_rng_change() {
        let root = temp_repository_root("field-move-sweet-scent-missing-surface");
        write_grass_tileset(&root, "johto");
        write_midi(
            &root
                .join("apps/web/assets/data")
                .join("content-packs/test/music/MUSIC_ROUTE_29.mid"),
        );
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data_with_grass_encounter(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "SWEET_SCENT".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session.state.rng_seed = 1;
        let before = session.state.clone();

        let error = session
            .use_sweet_scent_field_move(&runtime, 0, EncounterSurface::Water)
            .expect_err("missing water table rejects sweet scent");

        assert!(error
            .to_string()
            .contains("validate SWEET_SCENT encounters on RuntimeMap"));
        assert_eq!(session.state, before);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tm_rejects_cannot_learn_without_consumption() {
        let root = temp_repository_root("field-item-tm-cannot-learn");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("HEADBUTT".to_string(), runtime_move_named("HEADBUTT", 15));
        let mut tm = runtime_item("TM_HEADBUTT", item_pocket("TM_HM"));
        tm.field_menu = "ITEMMENU_PARTY".to_string();
        tm.field_usable = true;
        tm.consumable = true;
        tm.tmhm_index = Some(1);
        tm.tmhm_move = Some("HEADBUTT".to_string());
        data.items.insert("TM_HEADBUTT".to_string(), tm);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        session
            .state
            .storage
            .register_capture(Pokemon::new_for_tests(runtime_species(), 8, Dv::default()))
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["TM_HEADBUTT"], 1)
            .expect("add TM");
        let before = session.state.clone();

        let error = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "TM_HEADBUTT", 0, None)
            .expect_err("cannot learn");

        assert!(error.to_string().contains("CannotLearn"));
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["TM_HEADBUTT"]),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_tm_replaces_selected_full_move_slot() {
        let root = temp_repository_root("field-item-tm-replace");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        for (name, pp) in [
            ("HEADBUTT", 15),
            ("GROWL", 40),
            ("TAIL_WHIP", 30),
            ("LEER", 30),
        ] {
            data.moves
                .insert(name.to_string(), runtime_move_named(name, pp));
        }
        let mut tm = runtime_item("TM_HEADBUTT", item_pocket("TM_HM"));
        tm.field_menu = "ITEMMENU_PARTY".to_string();
        tm.field_usable = true;
        tm.consumable = true;
        tm.tmhm_index = Some(1);
        tm.tmhm_move = Some("HEADBUTT".to_string());
        data.items.insert("TM_HEADBUTT".to_string(), tm);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.species.tmhm_learnset = vec!["HEADBUTT".to_string()];
        player.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 35,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 40,
                pp_ups: 0,
            },
            LearnedMove {
                name: "TAIL_WHIP".to_string(),
                current_pp: 30,
                pp_ups: 0,
            },
            LearnedMove {
                name: "LEER".to_string(),
                current_pp: 30,
                pp_ups: 0,
            },
        ];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["TM_HEADBUTT"], 1)
            .expect("add TM");

        let item_use = session
            .use_bag_tmhm_on_party_pokemon(&runtime, "TM_HEADBUTT", 0, Some(2))
            .expect("replace move");

        assert_eq!(item_use.learned_move.replaced_slot, Some(2));
        assert_eq!(
            item_use.learned_move.replaced_move.as_deref(),
            Some("TAIL_WHIP")
        );
        assert_eq!(
            session.state.storage.party.pokemon[0]
                .as_ref()
                .expect("player")
                .moves[2]
                .name,
            "HEADBUTT"
        );
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["TM_HEADBUTT"]),
            0
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_pp_up_raises_selected_move_pp_stage() {
        let root = temp_repository_root("field-item-pp-up");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut pp_up = runtime_item("PP_UP", item_pocket("ITEM"));
        pp_up.effect = "MOD_PP_UP".to_string();
        pp_up.pp_up_stages = Some(1);
        pp_up.field_menu = "ITEMMENU_PARTY".to_string();
        pp_up.field_usable = true;
        pp_up.battle_menu = "ITEMMENU_NOUSE".to_string();
        pp_up.battle_usable = false;
        pp_up.consumable = true;
        data.items.insert("PP_UP".to_string(), pp_up);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 20,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PP_UP"], 1)
            .expect("add PP Up");

        let item_use = session
            .use_bag_item_on_party_move(&runtime, "PP_UP", 0, Some(0))
            .expect("use PP Up");

        assert_eq!(item_use.item_use.item_id, "PP_UP");
        assert_eq!(item_use.item_effect.pp_changes[0].pp_before, 20);
        assert_eq!(item_use.item_effect.pp_changes[0].pp_after, 27);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.moves[0].pp_ups, 1);
        assert_eq!(pokemon.moves[0].current_pp, 27);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["PP_UP"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_field_item_pp_up_rejects_maxed_move_without_consumption() {
        let root = temp_repository_root("field-item-pp-up-maxed");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut pp_up = runtime_item("PP_UP", item_pocket("ITEM"));
        pp_up.effect = "MOD_PP_UP".to_string();
        pp_up.pp_up_stages = Some(1);
        pp_up.field_menu = "ITEMMENU_PARTY".to_string();
        pp_up.field_usable = true;
        pp_up.battle_menu = "ITEMMENU_NOUSE".to_string();
        pp_up.battle_usable = false;
        pp_up.consumable = true;
        data.items.insert("PP_UP".to_string(), pp_up);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 56,
            pp_ups: 3,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["PP_UP"], 1)
            .expect("add PP Up");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_party_move(&runtime, "PP_UP", 0, Some(0))
            .expect_err("maxed move rejects PP Up");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["PP_UP"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_restores_selected_move_pp_from_compiled_moves() {
        let root = temp_repository_root("battle-item-ether");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        data.moves
            .insert("GROWL".to_string(), runtime_move_named("GROWL", 40));
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![
            LearnedMove {
                name: "TACKLE".to_string(),
                current_pp: 20,
                pp_ups: 0,
            },
            LearnedMove {
                name: "GROWL".to_string(),
                current_pp: 1,
                pp_ups: 0,
            },
        ];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");

        let item_use = session
            .use_bag_item_on_battle_party_move(&runtime, "ETHER", 0, Some(0))
            .expect("use ether");

        assert_eq!(item_use.item_use.item_id, "ETHER");
        assert_eq!(item_use.battle_item.pp_changes.len(), 1);
        assert_eq!(item_use.battle_item.pp_changes[0].move_id, "TACKLE");
        assert_eq!(item_use.battle_item.pp_changes[0].pp_before, 20);
        assert_eq!(item_use.battle_item.pp_changes[0].pp_after, 30);
        let pokemon = session.state.storage.party.pokemon[0]
            .as_ref()
            .expect("player");
        assert_eq!(pokemon.moves[0].current_pp, 30);
        assert_eq!(pokemon.moves[1].current_pp, 1);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_rejects_full_pp_without_consumption() {
        let root = temp_repository_root("battle-item-ether-full");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut ether = runtime_item("ETHER", item_pocket("ITEM"));
        ether.effect = "MOD_RESTORE_PP".to_string();
        ether.pp_restore_scope = Some("MOVE".to_string());
        ether.pp_restore_points = Some(10);
        ether.field_menu = "ITEMMENU_PARTY".to_string();
        ether.field_usable = true;
        ether.battle_menu = "ITEMMENU_PARTY".to_string();
        ether.battle_usable = true;
        ether.consumable = true;
        data.items.insert("ETHER".to_string(), ether);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.moves = vec![LearnedMove {
            name: "TACKLE".to_string(),
            current_pp: 35,
            pp_ups: 0,
        }];
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["ETHER"], 1)
            .expect("add ether");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_battle_party_move(&runtime, "ETHER", 0, Some(0))
            .expect_err("full PP has no target change");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["ETHER"]), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_rejects_missing_payload_without_consumption() {
        let root = temp_repository_root("battle-item-unsupported");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut bad_potion = runtime_item("BAD_POTION", item_pocket("ITEM"));
        bad_potion.effect = "MOD_UNDECLARED".to_string();
        bad_potion.parameter = 0;
        bad_potion.field_menu = "ITEMMENU_PARTY".to_string();
        bad_potion.field_usable = true;
        bad_potion.battle_menu = "ITEMMENU_PARTY".to_string();
        bad_potion.battle_usable = true;
        bad_potion.consumable = true;
        data.items.insert("BAD_POTION".to_string(), bad_potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let mut player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        player.hp = 11;
        player.max_hp = 35;
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["BAD_POTION"], 1)
            .expect("add item");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "BAD_POTION")
            .expect_err("payload-less battle item rejected");

        assert!(error.to_string().contains("MissingBattleItemPayload"));
        assert_eq!(session.state, before);
        assert_eq!(
            session
                .state
                .bag
                .quantity(&runtime.data.items["BAD_POTION"]),
            1
        );
        assert!(session.state.script_runtime.item_use_events.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_battle_item_rejects_full_hp_without_consumption() {
        let root = temp_repository_root("battle-item-full-hp");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data_with_scripted_battles();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.effect = "RESTORE_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.field_usable = true;
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_usable = true;
        potion.consumable = true;
        data.items.insert("POTION".to_string(), potion);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        let player = Pokemon::new_for_tests(runtime_species(), 8, Dv::default());
        session
            .state
            .storage
            .register_capture(player)
            .expect("register player");
        session.state.sync_party_from_storage();
        session
            .state
            .bag
            .add_item(&runtime.data.items["POTION"], 1)
            .expect("add potion");
        session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "RuntimeWildScript", 4)
            .expect("scripted wild battle starts");
        let before = session.state.clone();

        let error = session
            .use_bag_item_on_active_battle_pokemon(&runtime, "POTION")
            .expect_err("full HP target has no effect");

        assert!(error.to_string().contains("NoTargetChange"));
        assert_eq!(session.state, before);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert!(session.state.script_runtime.item_use_events.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_script_item_economy_and_shop_commands() {
        let root = temp_repository_root("script-items-economy-shop");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut potion = runtime_item("POTION", item_pocket("ITEM"));
        potion.price = 300;
        let mut poke_ball = runtime_ball_item("POKE_BALL");
        poke_ball.price = 200;
        data.items.insert("POTION".to_string(), potion);
        data.items.insert("POKE_BALL".to_string(), poke_ball);
        data.marts.0.insert(
            "RuntimeMart".to_string(),
            vec!["POTION".to_string(), "POKE_BALL".to_string()],
        );
        data.currency_constants
            .0
            .insert("RUNTIME_PRICE".to_string(), 500);
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.script_item_grants
            .push(crystal_core::systems::script_items::ScriptItemGrant {
                item_id: "POTION".to_string(),
                quantity: 2,
                source_script: "RuntimeItemScript".to_string(),
                command_index: 1,
                verbose: true,
            });
        map.script_item_checks
            .push(crystal_core::systems::script_items::ScriptItemAccess {
                item_id: "POTION".to_string(),
                source_script: "RuntimeItemScript".to_string(),
                command_index: 2,
            });
        map.script_item_takes
            .push(crystal_core::systems::script_items::ScriptItemAccess {
                item_id: "POTION".to_string(),
                source_script: "RuntimeItemScript".to_string(),
                command_index: 3,
            });
        map.script_economy_commands
            .push(crystal_core::systems::economy::ScriptEconomyCommand {
                command: "checkmoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["RUNTIME_PRICE".to_string()],
                source_script: "RuntimeEconomyScript".to_string(),
                command_index: 4,
            });
        map.script_economy_commands
            .push(crystal_core::systems::economy::ScriptEconomyCommand {
                command: "takemoney".to_string(),
                account: Some("YOUR_MONEY".to_string()),
                amount_tokens: vec!["RUNTIME_PRICE".to_string()],
                source_script: "RuntimeEconomyScript".to_string(),
                command_index: 5,
            });
        map.script_shop_commands
            .push(crystal_core::systems::shop::ScriptShopCommand {
                mart_type: "MARTTYPE_STANDARD".to_string(),
                mart_id: "RuntimeMart".to_string(),
                source_script: "RuntimeShopScript".to_string(),
                command_index: 6,
            });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        session.state.money = 1_000;

        let grant = session
            .grant_script_item(&runtime, "RuntimeMap", "RuntimeItemScript", 1)
            .expect("grant script item");
        let check = session
            .check_script_item(&runtime, "RuntimeMap", "RuntimeItemScript", 2)
            .expect("check script item");
        let take = session
            .take_script_item(&runtime, "RuntimeMap", "RuntimeItemScript", 3)
            .expect("take script item");
        let money_check = session
            .apply_script_economy_command(&runtime, "RuntimeMap", "RuntimeEconomyScript", 4)
            .expect("check money");
        let money_take = session
            .apply_script_economy_command(&runtime, "RuntimeMap", "RuntimeEconomyScript", 5)
            .expect("take money");
        let shop = session
            .open_script_shop(&runtime, "RuntimeMap", "RuntimeShopScript", 6)
            .expect("open shop");
        let buy = session
            .buy_shop_item(&runtime, "POKE_BALL", 1)
            .expect("buy ball");

        assert!(matches!(
            grant.outcome,
            ScriptItemGrantOutcome::Granted { .. }
        ));
        assert!(check.outcome.held);
        assert!(take.outcome.removed);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert!(matches!(
            money_check.outcome,
            ScriptEconomyOutcome::Check {
                script_value,
                ..
            } if script_value == "HAVE_MORE"
        ));
        assert!(matches!(
            money_take.outcome,
            ScriptEconomyOutcome::MoneyChanged { balance: 500, .. }
        ));
        assert_eq!(shop.outcome.inventory, vec!["POTION", "POKE_BALL"]);
        assert!(buy.outcome.success);
        assert_eq!(session.state.money, 300);
        let sell = session
            .sell_shop_item(&runtime, "POKE_BALL", 1)
            .expect("sell ball");
        assert!(sell.outcome.success);
        assert_eq!(session.state.money, 400);
        assert_ne!(grant.state_checksum, sell.state_checksum);

        let wrong_index = session
            .open_script_shop(&runtime, "RuntimeMap", "RuntimeShopScript", 7)
            .expect_err("script shop command indexes are exact");
        assert!(wrong_index
            .to_string()
            .contains("has no script shop command at RuntimeShopScript:7"));
        let wrong_item = session
            .buy_shop_item(&runtime, "poke_ball", 1)
            .expect_err("active shop item ids are exact");
        assert!(wrong_item
            .to_string()
            .contains("does not sell exact item id poke_ball"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_field_pickups_and_phone_commands() {
        let root = temp_repository_root("field-pickups-phone");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.items.insert(
            "POTION".to_string(),
            runtime_item("POTION", item_pocket("ITEM")),
        );
        data.items.insert(
            "BERRY".to_string(),
            runtime_item("BERRY", item_pocket("ITEM")),
        );
        data.fruit_trees
            .0
            .insert("FRUITTREE_RUNTIME".to_string(), "BERRY".to_string());
        data.phone_contacts.0.insert(
            "PHONE_MOM".to_string(),
            crystal_core::systems::phone::PhoneContactRecord {
                contact_id: "PHONE_MOM".to_string(),
                trainer_class: None,
                trainer_label: None,
                lines: vec!["Mom".to_string()],
                primary_label: "MomPhoneScript".to_string(),
                map_constant: None,
                callee_time_mask: 0xff,
                callee_script: None,
                caller_time_mask: 0xff,
                caller_script: None,
            },
        );
        data.phone_contacts.0.insert(
            "PHONE_JOEY".to_string(),
            crystal_core::systems::phone::PhoneContactRecord {
                contact_id: "PHONE_JOEY".to_string(),
                trainer_class: Some("YOUNGSTER".to_string()),
                trainer_label: Some("JOEY".to_string()),
                lines: vec!["Top percentage.".to_string()],
                primary_label: "JoeyPhoneScript".to_string(),
                map_constant: Some("RUNTIME_MAP".to_string()),
                callee_time_mask: 0xff,
                callee_script: None,
                caller_time_mask: 0xff,
                caller_script: None,
            },
        );
        data.permanent_phone_numbers = vec!["PHONE_MOM".to_string()];
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.objects.push(ObjectEvent {
            sprite: "SPRITE_BALL_CUT_FRUIT".to_string(),
            x: 1,
            y: 0,
            spritemovedata: "SPRITEMOVEDATA_STILL".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_ITEMBALL".to_string(),
            radius: 0,
            script: "RuntimeItemBallScript".to_string(),
            label: None,
            event_flag: "EVENT_RUNTIME_POTION".to_string(),
            object_identifier: Some("RUNTIME_POTION_BALL".to_string()),
            sightline_direction_override: None,
        });
        map.script_field_pickups
            .push(crystal_core::systems::field_items::ScriptFieldPickup {
                command: "itemball".to_string(),
                item_id: Some("POTION".to_string()),
                quantity: 1,
                event_flag: Some("EVENT_RUNTIME_POTION".to_string()),
                fruit_tree_id: None,
                source_script: "RuntimeItemBallScript".to_string(),
                command_index: 0,
            });
        map.script_field_pickups
            .push(crystal_core::systems::field_items::ScriptFieldPickup {
                command: "fruittree".to_string(),
                item_id: None,
                quantity: 1,
                event_flag: None,
                fruit_tree_id: Some("FRUITTREE_RUNTIME".to_string()),
                source_script: "RuntimeFruitTreeScript".to_string(),
                command_index: 1,
            });
        map.script_phone_commands
            .push(crystal_core::systems::phone::ScriptPhoneCommand {
                command: "checkcellnum".to_string(),
                contact_id: "PHONE_JOEY".to_string(),
                source_script: "RuntimePhoneScript".to_string(),
                command_index: 2,
            });
        map.script_phone_commands
            .push(crystal_core::systems::phone::ScriptPhoneCommand {
                command: "askforphonenumber".to_string(),
                contact_id: "PHONE_JOEY".to_string(),
                source_script: "RuntimePhoneScript".to_string(),
                command_index: 3,
            });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        assert!(session
            .overworld
            .visible_object_at(TilePosition::new(1, 0))
            .is_some());

        let permanent = session
            .initialize_permanent_phone_numbers(&runtime)
            .expect("permanent phones initialize");
        let check_before = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                2,
                ScriptPhoneInputs::default(),
            )
            .expect("check phone before registration");
        let ask = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                3,
                ScriptPhoneInputs {
                    accepted: Some(true),
                },
            )
            .expect("register phone");
        let check_after = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                2,
                ScriptPhoneInputs::default(),
            )
            .expect("check phone after registration");
        let pickup = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "RuntimeItemBallScript", 0)
            .expect("pickup itemball");
        let fruit = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "RuntimeFruitTreeScript", 1)
            .expect("pickup fruit");

        assert_eq!(permanent.inserted, vec!["PHONE_MOM".to_string()]);
        assert!(matches!(
            check_before.outcome,
            ScriptPhoneOutcome::CheckCellNum {
                registered: false,
                script_value,
                ..
            } if script_value == "0"
        ));
        assert!(matches!(
            ask.outcome,
            ScriptPhoneOutcome::AskForPhoneNumber {
                result: crystal_core::systems::phone::PhoneRegistrationResult::Registered,
                script_value,
                ..
            } if script_value == "0"
        ));
        assert!(matches!(
            check_after.outcome,
            ScriptPhoneOutcome::CheckCellNum {
                registered: true,
                script_value,
                ..
            } if script_value == "1"
        ));
        assert!(matches!(
            pickup.outcome,
            FieldItemPickupOutcome::Collected {
                item_id,
                event_flag,
                ..
            } if item_id == "POTION" && event_flag == "EVENT_RUNTIME_POTION"
        ));
        assert!(session
            .overworld
            .visible_object_at(TilePosition::new(1, 0))
            .is_none());
        assert!(matches!(
            fruit.outcome,
            FieldItemPickupOutcome::Collected {
                item_id,
                event_flag,
                ..
            } if item_id == "BERRY" && event_flag == "FRUITTREE_RUNTIME_COLLECTED"
        ));
        assert_eq!(session.state.bag.quantity(&runtime.data.items["POTION"]), 1);
        assert_eq!(session.state.bag.quantity(&runtime.data.items["BERRY"]), 1);
        assert_ne!(permanent.state_checksum, fruit.state_checksum);

        let wrong_phone = session
            .apply_script_phone_command(
                &runtime,
                "RuntimeMap",
                "RuntimePhoneScript",
                4,
                ScriptPhoneInputs::default(),
            )
            .expect_err("phone command indexes are exact");
        assert!(wrong_phone
            .to_string()
            .contains("has no script phone command at RuntimePhoneScript:4"));
        let wrong_fruit = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "runtimefruittreescript", 1)
            .expect_err("field pickup script ids are exact");
        assert!(wrong_fruit
            .to_string()
            .contains("has no script field pickup at runtimefruittreescript:1"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_flags_scenes_and_block_changes() {
        let root = temp_repository_root("flags-scenes-blocks");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let map = data.maps.get_mut("RuntimeMap").expect("runtime map");
        map.scenes = MapSceneTable {
            scenes: vec![
                MapScene {
                    scene_id: "SCENE_RUNTIME_START".to_string(),
                    script_name: Some("RuntimeStartScene".to_string()),
                },
                MapScene {
                    scene_id: "SCENE_RUNTIME_DONE".to_string(),
                    script_name: None,
                },
            ],
        };
        map.objects.push(ObjectEvent {
            sprite: "SPRITE_MON".to_string(),
            x: 1,
            y: 0,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "RuntimeFlagScript".to_string(),
            label: None,
            event_flag: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
            object_identifier: Some("RUNTIME_HIDE_OBJECT".to_string()),
            sightline_direction_override: None,
        });
        map.script_flag_commands
            .push(crystal_core::systems::script_flags::ScriptFlagCommand {
                command: "setevent".to_string(),
                flag_id: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
                source_script: "RuntimeFlagScript".to_string(),
                command_index: 0,
            });
        map.script_flag_commands
            .push(crystal_core::systems::script_flags::ScriptFlagCommand {
                command: "checkevent".to_string(),
                flag_id: "EVENT_RUNTIME_HIDE_OBJECT".to_string(),
                source_script: "RuntimeFlagScript".to_string(),
                command_index: 1,
            });
        map.script_scene_commands
            .push(crystal_core::systems::script_scenes::ScriptSceneCommand {
                command: "setscene".to_string(),
                map_id: None,
                scene_id: Some("SCENE_RUNTIME_DONE".to_string()),
                source_script: "RuntimeSceneScript".to_string(),
                command_index: 2,
            });
        map.script_scene_commands
            .push(crystal_core::systems::script_scenes::ScriptSceneCommand {
                command: "checkscene".to_string(),
                map_id: None,
                scene_id: None,
                source_script: "RuntimeSceneScript".to_string(),
                command_index: 3,
            });
        map.script_scene_commands
            .push(crystal_core::systems::script_scenes::ScriptSceneCommand {
                command: "setmapscene".to_string(),
                map_id: Some("RUNTIME_MAP".to_string()),
                scene_id: Some("0".to_string()),
                source_script: "RuntimeSceneScript".to_string(),
                command_index: 4,
            });
        map.script_block_changes
            .push(crystal_core::systems::script_blocks::ScriptBlockChange {
                x: 2,
                y: 0,
                block_id: 7,
                source_script: "RuntimeBlockScript".to_string(),
                command_index: 5,
            });
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        assert!(session
            .overworld
            .visible_object_at(TilePosition::new(1, 0))
            .is_some());

        let flag = session
            .apply_script_flag_mutation(&runtime, "RuntimeMap", "RuntimeFlagScript", 0)
            .expect("set flag");
        let check_flag = session
            .check_script_flag(&runtime, "RuntimeMap", "RuntimeFlagScript", 1)
            .expect("check flag");
        let set_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 2)
            .expect("set scene");
        let check_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 3)
            .expect("check scene");
        let set_map_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 4)
            .expect("set map scene");
        let block = session
            .apply_script_block_change(&runtime, "RuntimeMap", "RuntimeBlockScript", 5)
            .expect("change block");

        assert!(flag.outcome.value);
        assert!(check_flag.outcome.set);
        assert!(session
            .overworld
            .visible_object_at(TilePosition::new(1, 0))
            .is_none());
        assert_eq!(set_scene.outcome.scene_id, "SCENE_RUNTIME_DONE");
        assert_eq!(check_scene.outcome.scene_id, "SCENE_RUNTIME_DONE");
        assert_eq!(set_map_scene.outcome.scene_id, "SCENE_RUNTIME_START");
        assert_eq!(session.overworld.map.metatile_at(1, 0), Some(7));
        assert_eq!(
            session
                .state
                .map_block_overrides
                .get("RuntimeMap")
                .and_then(|overrides| overrides.get(&(1, 0)))
                .copied(),
            Some(7)
        );
        assert_ne!(flag.state_checksum, block.state_checksum);

        let saved_state = session.state.clone();
        let resumed = RuntimeOverworldSession::from_state(&runtime, &asset_root, saved_state)
            .expect("resume with block overrides");
        assert_eq!(resumed.overworld.map.metatile_at(1, 0), Some(7));

        let wrong_scene = session
            .apply_script_scene_command(&runtime, "RuntimeMap", "RuntimeSceneScript", 9)
            .expect_err("scene command indexes are exact");
        assert!(wrong_scene
            .to_string()
            .contains("has no script scene command at RuntimeSceneScript:9"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_rejects_map_music_missing_from_runtime_catalog() {
        let root = temp_repository_root("overworld-missing-music");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .music = Some("MUSIC_ROUTE_29".to_string());
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(data, report()),
            identity(),
        )
        .expect("runtime");

        let error = runtime
            .start_overworld_session(&asset_root, 0)
            .expect_err("missing map music asset must fail")
            .to_string();

        assert!(
            error.contains("missing runtime music asset MUSIC_ROUTE_29"),
            "{error}"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_save_resume_uses_saved_position_without_spawn_fallback() {
        let root = temp_repository_root("overworld-resume");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");
        session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("turn right");
        let moved = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("move right");
        let save_path = root.join("slot.crystalsave");
        runtime
            .save_game(&save_path, session.state.clone())
            .expect("save moved state");
        let loaded = runtime.load_save(&save_path).expect("load moved state");

        let mut resumed = runtime
            .resume_overworld_session(&asset_root, loaded)
            .expect("resume saved overworld");

        assert_eq!(resumed.snapshot(), moved.snapshot);
        assert_eq!(resumed.state.frame_counter, 2);
        assert_eq!(
            resumed.state.overworld.snapshot_identity(),
            Some((
                "RuntimeMap",
                TilePosition::new(2, 0),
                Direction::Right,
                crystal_core::world::movement::MovementMode::Normal
            ))
        );
        let held = resumed
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("held right after resume");
        assert_eq!(held.pressed_mask, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_resume_rejects_inactive_state() {
        let root = temp_repository_root("overworld-inactive");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");

        let error = runtime
            .resume_overworld_session(&asset_root, GameState::default())
            .expect_err("inactive state must not fall back to spawn")
            .to_string();

        assert!(error.contains("inactive GameState"), "{error}");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_overworld_rejects_conflicting_direction_buttons() {
        let root = temp_repository_root("overworld-conflict");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

        let error = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Left, GameButton::Right])
            .expect_err("conflicting directions must fail");

        assert!(error.to_string().contains("conflicting direction buttons"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_helpers_bind_save_to_compiled_pack_identity() {
        let root = temp_repository_root("save");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("runtime");
        let save_path = root.join("slot.crystalsave");
        let mut state = GameState::default();
        state.frame_counter = 77;

        runtime
            .save_game(&save_path, state.clone())
            .expect("write runtime save");
        let loaded = runtime.load_save(&save_path).expect("load runtime save");

        assert_eq!(loaded, state);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_save_loader_rejects_different_compiled_pack_identity() {
        let root = temp_repository_root("save-mismatch");
        let asset_root = AssetRoot::new(&root);
        let first_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("first runtime");
        let second_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report()),
            SaveModpackIdentity::new("core-modular", "ffffffff").expect("identity"),
        )
        .expect("second runtime");
        let save_path = root.join("slot.crystalsave");

        first_runtime
            .save_game(&save_path, GameState::default())
            .expect("write runtime save");
        let error = second_runtime
            .load_save(&save_path)
            .expect_err("runtime must reject saves from another pack")
            .to_string();

        assert!(error.contains("read Crystal runtime save for compiled modpack identity"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_json_pack_paths() {
        let root = temp_repository_root("json");
        std::fs::write(
            root.join("apps/web/assets/data/runtime.json"),
            br#"{"not":"a runtime pack"}"#,
        )
        .expect("write json fixture");
        let asset_root = AssetRoot::new(&root);

        let error = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.json")
            .expect_err("runtime must require .crystalpack")
            .to_string();

        assert!(error.contains("must use .crystalpack"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_compiled_pack_without_manifest_identity() {
        let root = temp_repository_root("missing-identity");
        let data_root = root.join("apps/web/assets/data");
        let pack = CompiledGamePack::new_unchecked_for_tests(GameDataSet::default(), ModpackCompileReport::default());
        crystal_assets::write_compiled_game_pack_for_tests(
            data_root.join("runtime.crystalpack"),
            &pack,
        )
        .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);

        let error = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect_err("runtime pack must declare a manifest identity")
            .to_string();

        assert!(error.contains("must include at least one manifest id"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_cannot_load_unverified_loaded_pack_publicly() {
        let root = temp_repository_root("loaded-unverified");
        let data_root = root.join("apps/web/assets/data");
        let pack = CompiledGamePack::new_unchecked_for_tests(GameDataSet::default(), report());
        crystal_assets::write_compiled_game_pack_for_tests(
            data_root.join("runtime.crystalpack"),
            &pack,
        )
        .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);
        let error = asset_root
            .load_loaded_verified_compiled_game_pack("runtime.crystalpack")
            .expect_err("public loaded pack access must reject unverified packs")
            .to_string();

        assert!(error.contains("compiled game pack is not verified for runtime"));
        assert!(error.contains("missing_runtime_pokemon"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_mp3_audio_declarations() {
        let root = temp_repository_root("mp3");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.audio = vec![ModpackAudioAsset {
            id: "MUSIC_ROUTE_29".to_string(),
            path: "content-packs/test/music/MUSIC_ROUTE_29.mp3".to_string(),
            kind: ModpackAudioKind::Music,
            source: ModpackAudioSource::Midi,
            sample_rate_hz: None,
            channels: None,
        }];
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report());

        let error = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect_err("runtime must reject mp3 audio")
            .to_string();

        assert!(error.contains("must use a .mid file"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_missing_declared_midi_assets() {
        let root = temp_repository_root("missing-midi");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        data.audio = vec![ModpackAudioAsset::music(
            "MUSIC_ROUTE_29",
            "content-packs/test/music/MUSIC_ROUTE_29.mid",
        )
        .expect("music asset")];
        let pack = CompiledGamePack::new_unchecked_for_tests(data, report());

        let error = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect_err("runtime must not synthesize missing audio")
            .to_string();

        assert!(error.contains("read runtime audio asset"));
        assert!(error.contains("content-packs/test/music/MUSIC_ROUTE_29.mid"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_empty_clean_game_data() {
        let root = temp_repository_root("empty-game");
        let asset_root = AssetRoot::new(&root);
        let pack = CompiledGamePack::new_unchecked_for_tests(GameDataSet::default(), report());

        let error = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect_err("runtime must not boot a clean report with no game data")
            .to_string();

        assert!(error.contains("no Pokemon species data"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_allows_warning_diagnostics() {
        let root = temp_repository_root("verify-warning");
        let asset_root = AssetRoot::new(&root);
        let report = ModpackCompileReport {
            manifests: vec!["core-modular".to_string()],
            diagnostics: vec![VerificationError {
                severity: VerificationSeverity::Warning,
                code: "warning_pack".to_string(),
                subject: "runtime".to_string(),
                message: "pack has an unresolved warning".to_string(),
            }],
            ..ModpackCompileReport::default()
        };
        let pack = CompiledGamePack::new_unchecked_for_tests(minimal_runtime_data(), report);

        CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect("runtime should boot warning-only compiled packs");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_pack_with_verification_errors() {
        let root = temp_repository_root("verify");
        let asset_root = AssetRoot::new(&root);
        let report = ModpackCompileReport {
            manifests: vec!["core-modular".to_string()],
            diagnostics: vec![
                VerificationError {
                    severity: VerificationSeverity::Warning,
                    code: "warning_pack".to_string(),
                    subject: "runtime".to_string(),
                    message: "pack has an unresolved warning".to_string(),
                },
                VerificationError {
                    severity: VerificationSeverity::Error,
                    code: "bad_pack".to_string(),
                    subject: "runtime".to_string(),
                    message: "pack failed verification".to_string(),
                },
            ],
            ..ModpackCompileReport::default()
        };
        let pack = CompiledGamePack::new_unchecked_for_tests(GameDataSet::default(), report);

        let error = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect_err("runtime must reject diagnostic-bearing compiled packs")
            .to_string();

        assert!(error.contains("compiled game pack has verification errors"));
        assert!(!error.contains("warning_pack"));
        assert!(error.contains("bad_pack"));
        let _ = std::fs::remove_dir_all(root);
    }
}
