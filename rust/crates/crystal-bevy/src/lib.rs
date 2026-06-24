use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use crystal_assets::modpack::{
    CompiledGamePack, GameDataSet, LoadedCompiledGamePack, ModpackAudioKind,
};
use crystal_assets::{AssetRoot, RuntimeSpawnPoint};
use crystal_audio::{AudioKind, AudioProgram, AudioProgramSource};
use crystal_core::battle::capture::{
    CaptureAttemptContext, CaptureOutcome, StoredCapture, complete_captured_pokemon,
    throw_ball_from_bag,
};
use crystal_core::battle::start::{
    StaticWildBattleStart, TrainerBattleCompletion, TrainerBattleStartStatus, WildBattleStart,
    complete_trainer_battle,
};
use crystal_core::input::{
    B_PAD_A, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP, GameButton, JoypadState,
};
use crystal_core::models::Dv;
use crystal_core::multiplayer::{PlayerId, StateChecksum, StateChecksumFrame, game_state_checksum};
use crystal_core::random::Random;
use crystal_core::save::{
    SaveGame, SaveModpackIdentity, assert_save_matches_modpack, read_save_game, write_save_game,
};
use crystal_core::state::{BattleMemory, GameState, OverworldMemory};
use crystal_core::systems::economy::{ScriptEconomyOutcome, apply_script_economy_command};
use crystal_core::systems::field_items::{FieldItemPickupOutcome, pickup_script_field_item};
use crystal_core::systems::gift_pokemon::{
    GiftPokemonOutcome, GiftPokemonRequest, give_gift_pokemon,
};
use crystal_core::systems::item_use::{
    ItemUseContext, ItemUseOutcome, ItemUseRequest, use_bag_item,
};
use crystal_core::systems::phone::{
    ScriptPhoneInputs, ScriptPhoneOutcome, apply_script_phone_command,
    initialize_permanent_phone_numbers,
};
use crystal_core::systems::script_blocks::{ScriptBlockChangeOutcome, apply_script_block_change};
use crystal_core::systems::script_flags::{
    ScriptFlagCheckOutcome, ScriptFlagMutationOutcome, apply_script_flag_mutation,
    check_script_flag,
};
use crystal_core::systems::script_scenes::{ScriptSceneOutcome, apply_script_scene_command};
use crystal_core::systems::script_items::{
    ScriptItemCheckOutcome, ScriptItemGrantOutcome, ScriptItemTakeOutcome, check_script_item,
    grant_script_item, take_script_item,
};
use crystal_core::systems::scripted_battles::{
    ScriptedBattleEffects, ScriptedBattleEffectsOutcome, apply_scripted_battle_effects_to_session,
};
use crystal_core::systems::shop::{
    ScriptShopOutcome, ShopResult, apply_script_shop_command, buy_item, sell_item,
};
use crystal_core::world::map::{Direction, OverworldMapData, TilePosition};
use crystal_core::world::movement::{StepOptions, StepOutcome};
use crystal_core::world::session::{
    ConnectionTransition, EncounterCheckOptions, OverworldInteraction, OverworldSession,
    OverworldSnapshot, WarpTransition, WildEncounterRoll,
};

pub use crystal_assets as assets;
pub use crystal_audio as audio;
pub use crystal_core as core;
pub use crystal_net as net;

pub const CORE_RUNTIME_PACK_PATH: &str = "content-packs/core-modular.crystalpack";

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
    pub music: BTreeMap<String, AudioProgram>,
    pub sound_effects: BTreeMap<String, AudioProgram>,
    pub cries: BTreeMap<String, AudioProgram>,
}

impl RuntimeAudioCatalog {
    pub fn is_empty(&self) -> bool {
        self.music.is_empty() && self.sound_effects.is_empty() && self.cries.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrystalRuntime {
    pub modpack: SaveModpackIdentity,
    pub data: GameDataSet,
    pub audio: RuntimeAudioCatalog,
    pub viewport: GameViewport,
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
    pub state: GameState,
    pub overworld: OverworldSession,
    joypad: JoypadState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeOverworldFrame {
    pub snapshot: OverworldSnapshot,
    pub input_mask: u8,
    pub pressed_mask: u8,
    pub movement: Option<StepOutcome>,
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
pub struct RuntimeGiftPokemonGrant {
    pub outcome: GiftPokemonOutcome,
    pub state_checksum: StateChecksum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeItemUse {
    pub outcome: ItemUseOutcome,
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
    pub fn load_core(asset_root: &AssetRoot) -> Result<Self> {
        Self::load_from_compiled_pack(asset_root, CORE_RUNTIME_PACK_PATH)
    }

    pub fn load_from_compiled_pack(
        asset_root: &AssetRoot,
        compiled_pack_path: impl AsRef<Path>,
    ) -> Result<Self> {
        let loaded = asset_root.load_loaded_compiled_game_pack(compiled_pack_path)?;
        Self::from_loaded_compiled_pack(asset_root, loaded)
    }

    pub fn from_loaded_compiled_pack(
        asset_root: &AssetRoot,
        loaded: LoadedCompiledGamePack,
    ) -> Result<Self> {
        let modpack_id = runtime_modpack_id(&loaded.pack)?;
        let modpack = SaveModpackIdentity::from_compiled_pack_bytes(modpack_id, &loaded.bytes)
            .context("compute compiled game pack save identity")?;
        Self::from_compiled_pack(asset_root, loaded.pack, modpack)
    }

    pub fn from_compiled_pack(
        asset_root: &AssetRoot,
        pack: CompiledGamePack,
        modpack: SaveModpackIdentity,
    ) -> Result<Self> {
        modpack.validate()?;
        let expected_id = runtime_modpack_id(&pack)?;
        if modpack.id != expected_id {
            anyhow::bail!(
                "compiled game pack identity '{}' does not match report manifest id '{}'",
                modpack.id,
                expected_id
            );
        }
        reject_pack_with_error_diagnostics(&pack)?;
        reject_pack_without_runtime_game_data(&pack)?;
        let audio = RuntimeAudioCatalog::from_game_data(asset_root, &pack.data)?;
        Ok(Self {
            modpack,
            data: pack.data,
            audio,
            viewport: GameViewport::default(),
        })
    }

    pub fn save_game(&self, path: impl AsRef<Path>, state: GameState) -> Result<()> {
        let save = SaveGame::new(state, self.modpack.clone());
        write_save_game(path, &save).context("write Crystal runtime save")
    }

    pub fn load_save(&self, path: impl AsRef<Path>) -> Result<GameState> {
        let save = read_save_game(path).context("read Crystal runtime save")?;
        assert_save_matches_modpack(&save, &self.modpack)
            .context("validate save compiled modpack identity")?;
        Ok(save.state)
    }

    pub fn boot_summary(&self) -> RuntimeBootSummary {
        RuntimeBootSummary {
            modpack_id: self.modpack.id.clone(),
            modpack_hash: self.modpack.hash.clone(),
            pokemon_species: self.data.pokemon.len(),
            moves: self.data.moves.len(),
            maps: self.data.maps.len(),
            items: self.data.items.len(),
            wild_encounter_tables: self.data.wild_encounters.len(),
            music_tracks: self.audio.music.len(),
            sound_effects: self.audio.sound_effects.len(),
            cries: self.audio.cries.len(),
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
        if !self.audio.music.contains_key(music) {
            anyhow::bail!("map {map_name} references missing runtime music asset {music}");
        }
        Ok(Some(music.clone()))
    }

    fn sync_current_map_music(&self, state: &mut GameState, map_name: &str) -> Result<()> {
        state.script_runtime.current_music = self.map_music(map_name)?;
        state.script_runtime.pending_music_fade = None;
        Ok(())
    }
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
        runtime.sync_current_map_music(&mut state, &overworld.map.name)?;
        Ok(Self {
            state,
            overworld,
            joypad: JoypadState::new(),
        })
    }

    fn from_state(
        runtime: &CrystalRuntime,
        asset_root: &AssetRoot,
        state: GameState,
    ) -> Result<Self> {
        let (map_name, tile, facing, mode) = state
            .overworld
            .snapshot_identity()
            .with_context(|| "cannot resume overworld session from inactive GameState")?;
        let mut overworld =
            runtime.overworld_session_for(asset_root, map_name, tile, state.frame_counter)?;
        overworld.player.facing = facing;
        overworld.player.mode = mode;
        apply_state_block_overrides(&mut overworld, &state);
        if let Some(music) = state.script_runtime.current_music.as_deref() {
            if !runtime.audio.music.contains_key(music) {
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
                let destination = &transition.destination;
                self.overworld = runtime.overworld_session_for(
                    asset_root,
                    &destination.map_name,
                    destination.tile,
                    self.overworld.frame,
                )?;
                apply_state_block_overrides(&mut self.overworld, &self.state);
                runtime.sync_current_map_music(&mut self.state, &destination.map_name)?;
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
                apply_state_block_overrides(&mut self.overworld, &self.state);
                runtime.sync_current_map_music(&mut self.state, &destination.map_name)?;
                connection = Some(transition);
            } else if moved {
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

    pub fn state_checksum_frame(&self, player_id: PlayerId) -> Result<StateChecksumFrame> {
        StateChecksumFrame::from_game_state(player_id, &self.state)
            .context("checksum authoritative GameState for player")
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
        let state_checksum =
            game_state_checksum(&self.state).context("checksum scripted wild battle completion")?;
        Ok(RuntimeScriptedBattleCompletion {
            continued_after_battle: true,
            effects: Some(outcome),
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
        let continued_after_battle = complete_trainer_battle(&mut self.state, &completion)
            .with_context(|| {
                format!(
                    "complete scripted trainer battle at {map_name}/{source_script}:{startbattle_command_index}"
                )
            })?;
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
        let player = self
            .state
            .storage
            .party
            .pokemon
            .iter()
            .flatten()
            .next()
            .cloned()
            .with_context(|| "cannot throw a ball without a player party Pokemon")?;
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
            self.state.battle = BattleMemory::Inactive;
        }
        self.state.sync_party_from_storage();
        let state_checksum =
            game_state_checksum(&self.state).context("checksum capture completion")?;
        Ok(RuntimeCaptureCompletion {
            stored,
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
            anyhow::bail!(
                "gift Pokemon at {map_name}/{source_script}:{command_index} requires resolved nickname label {}",
                gift.nickname_label.as_deref().unwrap_or_default()
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
                .with_context(|| format!("enter scene context for {map_name}"))?;
        }
        let (target_map_name, scene_table) = if let Some(target_map_id) = command.map_id.as_deref()
        {
            let target_map_name = runtime
                .data
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
        let outcome = sell_item(&mut self.state, &runtime.data.items, item_id, quantity)
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
                &mut rng,
                EncounterCheckOptions {
                    time: crystal_core::world::encounters::TimeOfDay::Day,
                    music_token: self.state.script_runtime.current_music.clone(),
                    has_cleanse_tag: false,
                },
            )
            .with_context(|| format!("check wild encounters on {}", self.overworld.map.name))?;
        self.state.rng_seed = rng.seed();
        Ok(roll)
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
        Ok(battle)
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

fn apply_state_block_overrides(overworld: &mut OverworldSession, state: &GameState) {
    let Some(overrides) = state.map_block_overrides.get(&overworld.map.name) else {
        return;
    };
    for ((metatile_x, metatile_y), block_id) in overrides {
        let (Ok(x), Ok(y)) = (i16::try_from(*metatile_x), i16::try_from(*metatile_y)) else {
            continue;
        };
        if let Some(index) = overworld.map.metatile_index(x, y) {
            overworld.map.metatile_ids[index] = *block_id;
        }
    }
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
                .with_context(|| format!("resolve runtime MIDI asset {}", asset.path))?;
            let bytes = std::fs::read(&path)
                .with_context(|| format!("read runtime MIDI asset {}", path.display()))?;
            if !bytes.starts_with(b"MThd") {
                anyhow::bail!("runtime audio asset {} is not a MIDI file", path.display());
            }
            let program = AudioProgram {
                cache_key: format!(
                    "{}:{}:{}",
                    audio_kind_name(asset.kind),
                    asset.id,
                    path.display()
                ),
                source: AudioProgramSource::Midi(bytes),
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
}

fn reject_pack_with_error_diagnostics(pack: &CompiledGamePack) -> Result<()> {
    if !pack.report.has_errors() {
        return Ok(());
    }
    let summary = pack
        .report
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
    let data = &pack.data;
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
    if pack.report.manifests.is_empty() {
        anyhow::bail!("compiled game pack report must include at least one manifest id");
    }
    Ok(pack.report.manifests.join("+"))
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
    use crystal_assets::{ScriptedTrainerBattle, ScriptedWildBattle};
    use crystal_core::battle::start::{
        StaticWildBattleRequest, TrainerBattleRequest, TrainerBattleStartStatus,
    };
    use crystal_core::map::{MapAttributes, MapEvents, MapSceneTable, ObjectEvent};
    use crystal_core::models::{
        BaseStats, CaptureStorageLocation, Dv, Item, ItemPocket, Move, Pokemon, PokemonSpecies,
        PokemonType, Trainer, TrainerPartyPokemon,
    };
    use crystal_core::systems::gift_pokemon::GiftPokemonScript;
    use crystal_core::systems::learnsets::LearnsetEntry;
    use crystal_core::world::encounters::{WildEncounter, WildEncounterData, WildEncounterTable};

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
        Move {
            name: "TACKLE".to_string(),
            move_type: PokemonType::Normal,
            power: 40,
            accuracy: 100,
            pp: 35,
            effect: "NORMAL_HIT".to_string(),
            effect_chance: 0,
            stat: None,
            amount: None,
        }
    }

    fn runtime_item(id: &str, pocket: ItemPocket) -> Item {
        Item {
            name: id.to_string(),
            description: String::new(),
            effect: "NONE".to_string(),
            price: 0,
            held_effect: "HELD_NONE".to_string(),
            parameter: 0,
            property: String::new(),
            pocket,
            field_menu: String::new(),
            battle_menu: String::new(),
            script_name: id.to_string(),
            consumable: false,
            tmhm_index: None,
        }
    }

    fn runtime_ball_item(id: &str) -> Item {
        let mut item = runtime_item(id, ItemPocket::Ball);
        item.consumable = true;
        item.battle_menu = "ITEMMENU_CLOSE".to_string();
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
            maps: [("RuntimeMap".to_string(), runtime_map())]
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
            ..GameDataSet::default()
        }
    }

    fn minimal_runtime_data_with_music() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.maps
            .get_mut("RuntimeMap")
            .expect("runtime map")
            .attributes
            .music = Some("MUSIC_ROUTE_29".to_string());
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
        ];
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

    fn minimal_runtime_data_with_scripted_battles() -> GameDataSet {
        let mut data = minimal_runtime_data();
        data.items
            .insert("MASTER_BALL".to_string(), runtime_ball_item("MASTER_BALL"));
        data.items
            .insert("BERRY".to_string(), runtime_item("BERRY", ItemPocket::Item));
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

    #[test]
    fn runtime_bootstrap_loads_compiled_pack_and_declared_midi_assets() {
        let root = temp_repository_root("loads");
        let data_root = root.join("apps/web/assets/data");
        write_midi(&data_root.join("content-packs/test/music/MUSIC_ROUTE_29.mid"));
        write_midi(&data_root.join("content-packs/test/sfx/SFX_TACKLE.mid"));
        write_midi(&data_root.join("content-packs/test/cries/CRY_NIDORAN_M.mid"));
        let mut data = minimal_runtime_data();
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
        let pack = CompiledGamePack::new(data, report());
        crystal_assets::write_compiled_game_pack(data_root.join("runtime.crystalpack"), &pack)
            .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);

        let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect("load runtime");

        assert_eq!(runtime.modpack.id, "core-modular");
        assert_eq!(runtime.modpack.hash.len(), 8);
        assert!(
            runtime
                .audio
                .program(AudioKind::Music, "MUSIC_ROUTE_29")
                .is_some()
        );
        assert!(
            runtime
                .audio
                .program(AudioKind::SoundEffect, "SFX_TACKLE")
                .is_some()
        );
        assert!(
            runtime
                .audio
                .program(AudioKind::Cry, "CRY_NIDORAN_M")
                .is_some()
        );
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
    fn runtime_overworld_starts_from_declared_spawn_and_steps_from_joypad() {
        let root = temp_repository_root("overworld");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new(minimal_runtime_data(), report()),
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

        let second = session
            .apply_buttons(&runtime, &asset_root, [GameButton::Right])
            .expect("move right");
        assert_eq!(second.snapshot.tile, TilePosition::new(2, 0));
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
        assert_eq!(second.state_checksum.frame, 2);
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
            CompiledGamePack::new(minimal_runtime_data_with_music(), report()),
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
            CompiledGamePack::new(minimal_runtime_data_with_grass_encounter(), report()),
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
    fn runtime_starts_scripted_wild_battle_from_exact_map_script_command() {
        let root = temp_repository_root("scripted-wild-battle");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new(minimal_runtime_data_with_scripted_battles(), report()),
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

        let error = session
            .start_scripted_wild_battle(&runtime, "RuntimeMap", "runtimewildscript", 4)
            .expect_err("script names are exact");
        assert!(
            error
                .to_string()
                .contains("has no scripted wild battle at runtimewildscript:4")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_capture_completion_stores_active_wild_pokemon_in_authoritative_state() {
        let root = temp_repository_root("scripted-wild-capture");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new(minimal_runtime_data_with_scripted_battles(), report()),
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
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_grants_scripted_gift_pokemon_into_authoritative_storage() {
        let root = temp_repository_root("scripted-gift-pokemon");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new(minimal_runtime_data_with_scripted_battles(), report()),
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
            CompiledGamePack::new(minimal_runtime_data_with_scripted_battles(), report()),
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
        assert!(
            nickname_error
                .to_string()
                .contains("requires resolved nickname label RuntimeGiftName")
        );

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
            CompiledGamePack::new(minimal_runtime_data_with_scripted_battles(), report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("overworld session");

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
        let player = Pokemon::new_for_tests(runtime_species(), 10, Dv::from_non_hp(10, 10, 10, 10));
        session
            .state
            .storage
            .register_capture(player)
            .expect("player party Pokemon");
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
        assert!(
            error
                .to_string()
                .contains("has no scripted trainer battle at RuntimeTrainerScript:9")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_uses_exact_pack_item_effects_and_checksums_state() {
        let root = temp_repository_root("item-use");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut potion = runtime_item("POTION", ItemPocket::Item);
        potion.effect = "HEAL_HP".to_string();
        potion.parameter = 20;
        potion.field_menu = "ITEMMENU_PARTY".to_string();
        potion.battle_menu = "ITEMMENU_PARTY".to_string();
        potion.consumable = true;
        let mut itemfinder = runtime_item("ITEMFINDER", ItemPocket::KeyItem);
        itemfinder.effect = "ITEMFINDER".to_string();
        itemfinder.field_menu = "ITEMMENU_CLOSE".to_string();
        data.items.insert("POTION".to_string(), potion);
        data.items.insert("ITEMFINDER".to_string(), itemfinder);
        let runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new(data, report()),
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

        assert_eq!(first.outcome.effect, "HEAL_HP");
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
            session.state.script_runtime.item_use_events[1].effect,
            "ITEMFINDER"
        );

        let error = session
            .use_bag_item(&runtime, "itemfinder", ItemUseContext::Field)
            .expect_err("case changed item id rejected");
        assert!(error.to_string().contains("UnknownItem"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_applies_exact_script_item_economy_and_shop_commands() {
        let root = temp_repository_root("script-items-economy-shop");
        write_floor_tileset(&root, "johto");
        let asset_root = AssetRoot::new(&root);
        let mut data = minimal_runtime_data();
        let mut potion = runtime_item("POTION", ItemPocket::Item);
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
            CompiledGamePack::new(data, report()),
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
        assert!(
            wrong_index
                .to_string()
                .contains("has no script shop command at RuntimeShopScript:7")
        );
        let wrong_item = session
            .buy_shop_item(&runtime, "poke_ball", 1)
            .expect_err("active shop item ids are exact");
        assert!(
            wrong_item
                .to_string()
                .contains("does not sell exact item id poke_ball")
        );
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
            runtime_item("POTION", ItemPocket::Item),
        );
        data.items
            .insert("BERRY".to_string(), runtime_item("BERRY", ItemPocket::Item));
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
            CompiledGamePack::new(data, report()),
            identity(),
        )
        .expect("runtime");
        let mut session = runtime
            .start_overworld_session(&asset_root, 0)
            .expect("session starts");
        assert!(
            session
                .overworld
                .visible_object_at(TilePosition::new(1, 0))
                .is_some()
        );

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
        assert!(
            session
                .overworld
                .visible_object_at(TilePosition::new(1, 0))
                .is_none()
        );
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
        assert!(
            wrong_phone
                .to_string()
                .contains("has no script phone command at RuntimePhoneScript:4")
        );
        let wrong_fruit = session
            .pickup_script_field_item(&runtime, "RuntimeMap", "runtimefruittreescript", 1)
            .expect_err("field pickup script ids are exact");
        assert!(
            wrong_fruit
                .to_string()
                .contains("has no script field pickup at runtimefruittreescript:1")
        );
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
            CompiledGamePack::new(data, report()),
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
            CompiledGamePack::new(minimal_runtime_data(), report()),
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
            CompiledGamePack::new(minimal_runtime_data(), report()),
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
            CompiledGamePack::new(minimal_runtime_data(), report()),
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
            CompiledGamePack::new(minimal_runtime_data(), report()),
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
            CompiledGamePack::new(minimal_runtime_data(), report()),
            identity(),
        )
        .expect("first runtime");
        let second_runtime = CrystalRuntime::from_compiled_pack(
            &asset_root,
            CompiledGamePack::new(minimal_runtime_data(), report()),
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

        assert!(error.contains("validate save compiled modpack identity"));
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
        let pack = CompiledGamePack::new(GameDataSet::default(), ModpackCompileReport::default());
        crystal_assets::write_compiled_game_pack(data_root.join("runtime.crystalpack"), &pack)
            .expect("write compiled runtime pack");
        let asset_root = AssetRoot::new(&root);

        let error = CrystalRuntime::load_from_compiled_pack(&asset_root, "runtime.crystalpack")
            .expect_err("runtime pack must declare a manifest identity")
            .to_string();

        assert!(error.contains("must include at least one manifest id"));
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
        }];
        let pack = CompiledGamePack::new(data, report());

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
        data.audio = vec![
            ModpackAudioAsset::music(
                "MUSIC_ROUTE_29",
                "content-packs/test/music/MUSIC_ROUTE_29.mid",
            )
            .expect("music asset"),
        ];
        let pack = CompiledGamePack::new(data, report());

        let error = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect_err("runtime must not synthesize missing audio")
            .to_string();

        assert!(error.contains("read runtime MIDI asset"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_bootstrap_rejects_empty_clean_game_data() {
        let root = temp_repository_root("empty-game");
        let asset_root = AssetRoot::new(&root);
        let pack = CompiledGamePack::new(GameDataSet::default(), report());

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
        let pack = CompiledGamePack::new(minimal_runtime_data(), report);

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
        let pack = CompiledGamePack::new(GameDataSet::default(), report);

        let error = CrystalRuntime::from_compiled_pack(&asset_root, pack, identity())
            .expect_err("runtime must reject diagnostic-bearing compiled packs")
            .to_string();

        assert!(error.contains("compiled game pack has verification errors"));
        assert!(!error.contains("warning_pack"));
        assert!(error.contains("bad_pack"));
        let _ = std::fs::remove_dir_all(root);
    }
}
