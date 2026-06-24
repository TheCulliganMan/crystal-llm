use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::map::MapSceneTable;
use crate::models::{Bag, PokedexState, Pokemon, PokemonStorage};
use crate::timing::Frame;
use crate::world::map::{Direction, TilePosition};
use crate::world::movement::MovementMode;
use crate::world::session::OverworldSnapshot;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameState {
    pub options: Options,
    pub party: PartyState,
    pub storage: PokemonStorage,
    pub bag: Bag,
    pub money: u32,
    pub moms_money: u32,
    pub coins: u16,
    pub pokedex: PokedexState,
    pub link_battle_stats: LinkBattleStats,
    pub badges: Badges,
    pub overworld: OverworldMemory,
    pub battle: BattleMemory,
    pub map_block_overrides: BTreeMap<String, BTreeMap<(u16, u16), u16>>,
    pub joypad: JoypadMemory,
    pub fishing: FishingMemory,
    pub scenes: SceneMemory,
    pub flags: EventFlagMemory,
    pub script_runtime: ScriptRuntimeMemory,
    pub frame_counter: u64,
    pub rng_seed: u32,
    pub has_seen_intro: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            options: Options::default(),
            party: PartyState::default(),
            storage: PokemonStorage::default(),
            bag: Bag::default(),
            money: 0,
            moms_money: 0,
            coins: 0,
            pokedex: PokedexState::default(),
            link_battle_stats: LinkBattleStats::default(),
            badges: Badges::default(),
            overworld: OverworldMemory::default(),
            battle: BattleMemory::default(),
            map_block_overrides: BTreeMap::new(),
            joypad: JoypadMemory::default(),
            fishing: FishingMemory::default(),
            scenes: SceneMemory::default(),
            flags: EventFlagMemory::default(),
            script_runtime: ScriptRuntimeMemory::default(),
            frame_counter: 0,
            rng_seed: 1,
            has_seen_intro: false,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleMemory {
    #[default]
    Inactive,
    Wild {
        battle_type: String,
        map_name: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
    },
    StaticWild {
        battle_type: String,
        species: String,
        level: u8,
        source_script: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
    },
    Trainer {
        battle_type: String,
        trainer_class: String,
        trainer_id: String,
        trainer_name: String,
        event_flag: String,
        seen_text: String,
        win_text: String,
        loss_text: String,
        callback: String,
        source_script: String,
        enemy_pokemon: Pokemon,
        enemy_party: Vec<Pokemon>,
        reward: u32,
        encounter_music: String,
        ai_move_flags: u32,
        ai_item_switch_flags: u32,
        ai_layers: Vec<String>,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverworldMemory {
    #[default]
    Inactive,
    Active {
        map_name: String,
        tile: TilePosition,
        facing: Direction,
        mode: MovementMode,
    },
}

impl OverworldMemory {
    pub fn from_snapshot(snapshot: &OverworldSnapshot) -> Self {
        Self::Active {
            map_name: snapshot.map_name.clone(),
            tile: snapshot.tile,
            facing: snapshot.facing,
            mode: snapshot.mode,
        }
    }

    pub fn snapshot_identity(&self) -> Option<(&str, TilePosition, Direction, MovementMode)> {
        match self {
            Self::Inactive => None,
            Self::Active {
                map_name,
                tile,
                facing,
                mode,
            } => Some((map_name, *tile, *facing, *mode)),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeMemory {
    pub script_value: Option<String>,
    pub variables: BTreeMap<String, String>,
    pub variable_writes: Vec<ScriptRuntimeVariableWrite>,
    pub memory: BTreeMap<String, String>,
    pub effects: Vec<ScriptRuntimeEffect>,
    pub named_buffers: BTreeMap<String, String>,
    pub asm_directives: Vec<ScriptRuntimeAsmDirective>,
    pub numeric_buffer_writes: Vec<ScriptRuntimeNumericBufferWrite>,
    pub elevator_floors: Vec<ScriptRuntimeElevatorFloor>,
    pub stone_table_entries: Vec<ScriptRuntimeStoneTableEntry>,
    pub decoration_descriptions: Vec<ScriptRuntimeDecorationDescription>,
    pub variable_sprites: BTreeMap<String, String>,
    pub phone_numbers: BTreeSet<String>,
    pub special_phone_calls: Vec<String>,
    pub pending_delays: Vec<ScriptRuntimeDelay>,
    pub pending_earthquakes: Vec<ScriptRuntimeEarthquake>,
    pub pending_emotes: Vec<ScriptRuntimeEmote>,
    pub command_queue: Vec<ScriptRuntimeQueuedCommand>,
    pub stack: Vec<String>,
    pub last_special_routine: Option<String>,
    pub last_talked_object: Option<String>,
    pub active_menu: Option<String>,
    pub active_pokemon_picture: Option<String>,
    pub menu_coords: Option<[i16; 4]>,
    pub map_music_restart_disabled: bool,
    pub map_music_requested: bool,
    pub window_open: bool,
    pub item_notify_queued: bool,
    pub warp_sound_queued: bool,
    pub teleport_from_queued: bool,
    pub hall_of_fame_requested: bool,
    pub credits_requested: bool,
    pub menu_2d_requested: bool,
    pub version_check_requested: bool,
    pub blackout_mod: Option<String>,
    pub battle_tower_text: Option<String>,
    pub completed_trades: Vec<String>,
    pub catch_tutorials: Vec<String>,
    pub checked_mail_targets: Vec<String>,
    pub given_mail_targets: Vec<String>,
    pub audio_events: Vec<ScriptAudioRuntimeEvent>,
    pub current_music: Option<String>,
    pub pending_music_fade: Option<ScriptMusicFade>,
    pub waiting_for_sound_effect: bool,
    pub map_events: Vec<ScriptMapRuntimeEvent>,
    pub pending_script_warp: Option<ScriptWarpRequest>,
    pub pending_map_load: Option<ScriptMapLoadRequest>,
    pub pending_map_refresh: Option<ScriptMapRefreshRequest>,
    pub warp_check_requested: bool,
    pub text_events: Vec<ScriptTextRuntimeEvent>,
    pub text_window_open: bool,
    pub pending_text_label: Option<String>,
    pub pending_text_wait: Option<ScriptTextWait>,
    pub pending_yes_no: Option<ScriptYesNoPrompt>,
    pub control_events: Vec<ScriptControlRuntimeEvent>,
    pub next_script: Option<String>,
    pub call_stack: Vec<ScriptReturnFrame>,
    pub deferred_scripts: Vec<String>,
    pub script_ended: Option<ScriptEndState>,
    pub shop_events: Vec<ScriptShopRuntimeEvent>,
    pub pending_shop: Option<ScriptShopRequest>,
    pub item_use_events: Vec<ItemUseRuntimeEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemUseRuntimeEvent {
    pub item_id: String,
    pub effect: String,
    pub held_effect: String,
    pub parameter: i16,
    pub property: String,
    pub field_menu: String,
    pub battle_menu: String,
    pub context: String,
    pub consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEffect {
    pub command: String,
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeVariableWrite {
    pub target: String,
    pub value: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeAsmDirective {
    pub command: String,
    pub args: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeNumericBufferWrite {
    pub target_buffer: String,
    pub value: String,
    pub width: u8,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeElevatorFloor {
    pub floor: String,
    pub warp: u16,
    pub target_map: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeStoneTableEntry {
    pub warp: u16,
    pub object_event: String,
    pub script: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeDecorationDescription {
    pub decoration: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptAudioRuntimeEvent {
    pub command: String,
    pub kind: ScriptAudioRuntimeKind,
    pub audio_id: Option<String>,
    pub fade_frames: Option<u16>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptAudioRuntimeKind {
    Music,
    SoundEffect,
    Cry,
    FadeMusic,
    WaitForSoundEffect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMusicFade {
    pub audio_id: String,
    pub fade_frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapRuntimeEvent {
    pub command: String,
    pub kind: ScriptMapRuntimeKind,
    pub target_map: Option<String>,
    pub tile: Option<TilePosition>,
    pub facing: Option<Direction>,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptMapRuntimeKind {
    NoWarp,
    Warp,
    WarpCheck,
    LoadMap,
    RefreshMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptWarpRequest {
    pub target_map: String,
    pub tile: TilePosition,
    pub facing: Option<Direction>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapLoadRequest {
    pub command: String,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptMapRefreshRequest {
    pub command: String,
    pub map_setup: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextRuntimeEvent {
    pub command: String,
    pub kind: ScriptTextRuntimeKind,
    pub text_label: Option<String>,
    pub face_player: bool,
    pub closes_text: bool,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptTextRuntimeKind {
    Open,
    Close,
    WaitButton,
    YesNo,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptTextWait {
    pub command: String,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptYesNoPrompt {
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptControlRuntimeEvent {
    pub kind: ScriptControlRuntimeKind,
    pub target_script: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptControlRuntimeKind {
    Continue,
    Jump,
    Call,
    Defer,
    StandardJump,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptReturnFrame {
    pub source_script: String,
    pub next_command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptEndState {
    pub callback: bool,
    pub just_battled_guard: bool,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopRuntimeEvent {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptShopRequest {
    pub mart_type: String,
    pub mart_id: String,
    pub inventory: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeDelay {
    pub command: String,
    pub frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEarthquake {
    pub parameter: u16,
    pub shake_frames: u16,
    pub sleep_frames: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeEmote {
    pub emote: String,
    pub object: String,
    pub duration: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptRuntimeQueuedCommand {
    pub command: String,
    pub target: String,
    pub bank: Option<String>,
    pub source_script: String,
    pub command_index: usize,
}

impl GameState {
    pub fn frame(&self) -> Frame {
        Frame(self.frame_counter)
    }

    pub fn advance_frame(&mut self) -> Frame {
        self.frame_counter += 1;
        self.frame()
    }

    pub fn sync_party_from_storage(&mut self) {
        self.party = PartyState::from_storage(&self.storage);
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventFlagMemory {
    pub event_flags: BTreeMap<String, bool>,
    pub engine_flags: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum EventFlagError {
    #[error("empty flag name")]
    EmptyFlagName,
}

impl EventFlagMemory {
    pub fn set_event_flag(&mut self, flag_name: &str, value: bool) -> Result<(), EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        self.event_flags.insert(flag_name.to_string(), value);
        Ok(())
    }

    pub fn clear_event_flag(&mut self, flag_name: &str) -> Result<(), EventFlagError> {
        self.set_event_flag(flag_name, false)
    }

    pub fn is_event_flag_set(&self, flag_name: &str) -> Result<bool, EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        Ok(self.event_flags.get(flag_name).copied().unwrap_or(false))
    }

    pub fn set_engine_flag(&mut self, flag_name: &str, value: bool) -> Result<(), EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        self.engine_flags.insert(flag_name.to_string(), value);
        Ok(())
    }

    pub fn clear_engine_flag(&mut self, flag_name: &str) -> Result<(), EventFlagError> {
        self.set_engine_flag(flag_name, false)
    }

    pub fn is_engine_flag_set(&self, flag_name: &str) -> Result<bool, EventFlagError> {
        let flag_name = validate_flag_name(flag_name)?;
        Ok(self.engine_flags.get(flag_name).copied().unwrap_or(false))
    }

    pub fn set_script_flag(&mut self, flag_name: &str, value: bool) -> Result<(), EventFlagError> {
        if is_engine_flag_name(flag_name) {
            self.set_engine_flag(flag_name, value)
        } else {
            self.set_event_flag(flag_name, value)
        }
    }

    pub fn clear_script_flag(&mut self, flag_name: &str) -> Result<(), EventFlagError> {
        self.set_script_flag(flag_name, false)
    }

    pub fn is_script_flag_set(&self, flag_name: &str) -> Result<bool, EventFlagError> {
        if is_engine_flag_name(flag_name) {
            self.is_engine_flag_set(flag_name)
        } else {
            self.is_event_flag_set(flag_name)
        }
    }

    pub fn active_event_flags(&self) -> impl Iterator<Item = &String> {
        self.event_flags
            .iter()
            .filter_map(|(flag, value)| value.then_some(flag))
    }
}

pub fn is_engine_flag_name(flag_name: &str) -> bool {
    flag_name.starts_with("ENGINE_") || flag_name.starts_with("STATUSFLAGS_")
}

fn validate_flag_name(flag_name: &str) -> Result<&str, EventFlagError> {
    if flag_name.is_empty() {
        Err(EventFlagError::EmptyFlagName)
    } else {
        Ok(flag_name)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneMemory {
    pub current_map_name: String,
    pub scene_name: String,
    pub map_scenes: BTreeMap<String, String>,
    pub map_scene_indices: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneStatus {
    pub map_name: String,
    pub scene_name: String,
    pub scene_index: usize,
    pub script_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneError {
    MissingSceneTable {
        map_name: String,
    },
    UnknownScene {
        map_name: String,
        scene_name: String,
    },
    EmptySceneTable {
        map_name: String,
    },
}

impl SceneMemory {
    pub fn enter_map(
        &mut self,
        map_name: impl Into<String>,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let map_name = map_name.into();
        self.current_map_name = map_name.clone();
        let status = self.ensure_map_scene_initialized(&map_name, table)?;
        self.scene_name = status.scene_name.clone();
        Ok(status)
    }

    pub fn ensure_map_scene_initialized(
        &mut self,
        map_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        if table.scenes.is_empty() {
            return Err(SceneError::EmptySceneTable {
                map_name: map_name.to_string(),
            });
        }
        if let Some(scene_name) = self.map_scenes.get(map_name).cloned() {
            return self.scene_status(map_name, &scene_name, table);
        }
        let scene = table
            .scenes
            .first()
            .ok_or_else(|| SceneError::EmptySceneTable {
                map_name: map_name.to_string(),
            })?;
        self.set_map_scene(map_name, &scene.scene_id, table)
    }

    pub fn set_current_scene(
        &mut self,
        scene_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let map_name = self.current_map_name.clone();
        if map_name.is_empty() {
            return Err(SceneError::MissingSceneTable {
                map_name: String::new(),
            });
        }
        self.set_map_scene(&map_name, scene_name, table)
    }

    pub fn set_map_scene(
        &mut self,
        map_name: &str,
        scene_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let status = self.scene_status(map_name, scene_name, table)?;
        self.map_scenes
            .insert(map_name.to_string(), scene_name.to_string());
        self.map_scene_indices
            .insert(map_name.to_string(), status.scene_index);
        if self.current_map_name == map_name {
            self.scene_name = scene_name.to_string();
        }
        Ok(status)
    }

    pub fn check_scene(
        &mut self,
        map_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let status = self.ensure_map_scene_initialized(map_name, table)?;
        if self.current_map_name == map_name {
            self.scene_name = status.scene_name.clone();
        }
        Ok(status)
    }

    fn scene_status(
        &self,
        map_name: &str,
        scene_name: &str,
        table: &MapSceneTable,
    ) -> Result<SceneStatus, SceneError> {
        let (scene_index, scene) = table
            .scenes
            .iter()
            .enumerate()
            .find(|(_, scene)| scene.scene_id == scene_name)
            .ok_or_else(|| SceneError::UnknownScene {
                map_name: map_name.to_string(),
                scene_name: scene_name.to_string(),
            })?;
        Ok(SceneStatus {
            map_name: map_name.to_string(),
            scene_name: scene.scene_id.clone(),
            scene_index,
            script_name: scene.script_name.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    pub text_speed: TextSpeed,
    pub battle_scene: BattleScene,
    pub battle_style: BattleStyle,
    pub sound: Sound,
    pub menu_account: MenuAccount,
    pub print_option: PrintOption,
    pub frame: FrameType,
    pub no_text_scroll: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            text_speed: TextSpeed::Fast,
            battle_scene: BattleScene::On,
            battle_style: BattleStyle::Shift,
            sound: Sound::Stereo,
            menu_account: MenuAccount::On,
            print_option: PrintOption::Normal,
            frame: FrameType::Frame1,
            no_text_scroll: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSpeed {
    Fast,
    Mid,
    Slow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleScene {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattleStyle {
    Shift,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    Mono,
    Stereo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuAccount {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrintOption {
    Normal,
    Lightest,
    Lighter,
    Darker,
    Darkest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameType {
    Frame1,
    Frame2,
    Frame3,
    Frame4,
    Frame5,
    Frame6,
    Frame7,
    Frame8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyState {
    pub pokemon: [Option<PartyPokemonRef>; 6],
}

impl Default for PartyState {
    fn default() -> Self {
        Self {
            pokemon: [const { None }; 6],
        }
    }
}

impl PartyState {
    pub fn from_storage(storage: &PokemonStorage) -> Self {
        let mut state = Self::default();
        for (index, pokemon) in storage.party.pokemon.iter().enumerate() {
            if let Some(pokemon) = pokemon {
                state.pokemon[index] = Some(PartyPokemonRef {
                    species: pokemon.species.id.clone(),
                    level: pokemon.level,
                });
            }
        }
        state
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartyPokemonRef {
    pub species: String,
    pub level: u8,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkBattleStats {
    pub wins: u16,
    pub losses: u16,
    pub draws: u16,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FishingRodState {
    #[default]
    Idle,
    Waiting,
    Bite,
    Battle,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FishingMemory {
    pub rod_state: FishingRodState,
    pub rod_index: Option<u8>,
    pub bites_remaining: u8,
    pub result: u8,
    pub daily_flags1: u8,
    pub swarm_flag: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Badges {
    pub johto: [bool; 8],
    pub kanto: [bool; 8],
}

impl Default for Badges {
    fn default() -> Self {
        Self {
            johto: [false; 8],
            kanto: [false; 8],
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoypadMemory {
    pub h_joypad_released: u8,
    pub h_joypad_pressed: u8,
    pub h_joypad_down: u8,
    pub h_joypad_sum: u8,
    pub h_joy_released: u8,
    pub h_joy_pressed: u8,
    pub h_joy_down: u8,
    pub h_joy_last: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameCommand {
    Joypad { mask: u8 },
    OpenMenu,
    CloseMenu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GameEvent {
    FrameAdvanced { frame: u64 },
    JoypadChanged { pressed: u8, down: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_matches_typescript_defaults_that_affect_gameplay() {
        let state = GameState::default();
        assert_eq!(state.options.text_speed, TextSpeed::Fast);
        assert_eq!(state.options.battle_scene, BattleScene::On);
        assert_eq!(state.options.battle_style, BattleStyle::Shift);
        assert_eq!(state.options.sound, Sound::Stereo);
        assert_eq!(state.options.menu_account, MenuAccount::On);
        assert_eq!(state.party.pokemon, [const { None }; 6]);
        assert_eq!(state.storage, PokemonStorage::default());
        assert_eq!(state.bag, Bag::default());
        assert_eq!(state.pokedex, PokedexState::default());
        assert_eq!(state.link_battle_stats, LinkBattleStats::default());
        assert_eq!(state.badges.johto, [false; 8]);
        assert_eq!(state.badges.kanto, [false; 8]);
        assert_eq!(state.overworld, OverworldMemory::Inactive);
        assert_eq!(state.scenes, SceneMemory::default());
        assert_eq!(state.flags, EventFlagMemory::default());
        assert_eq!(state.frame_counter, 0);
        assert_eq!(state.rng_seed, 1);
        assert!(!state.has_seen_intro);
    }

    #[test]
    fn state_serializes_for_saves_and_multiplayer_hash_inputs() {
        let state = GameState::default();
        let json = serde_json::to_string(&state).expect("serialize game state");
        assert!(json.contains(r#""text_speed":"fast""#));
        assert_eq!(
            serde_json::from_str::<GameState>(&json).expect("deserialize game state"),
            state
        );
    }

    #[test]
    fn party_state_projects_full_authoritative_storage() {
        let mut state = GameState::default();
        let mut species = crate::models::PokemonSpecies::new_for_tests(
            "CHIKORITA",
            crate::models::BaseStats::new(45, 49, 49, 45, 65, 65),
        );
        species.int_id = 152;
        let pokemon = Pokemon::new_for_tests(species, 6, crate::models::Dv::default());
        state
            .storage
            .register_capture(pokemon)
            .expect("capture registers");

        state.sync_party_from_storage();

        assert_eq!(
            state.party.pokemon[0],
            Some(PartyPokemonRef {
                species: "CHIKORITA".to_string(),
                level: 6,
            })
        );
    }

    #[test]
    fn state_json_rejects_unknown_saved_runtime_fields_without_legacy_fallbacks() {
        let mut state_json = serde_json::to_value(GameState::default()).expect("state json");
        state_json
            .as_object_mut()
            .expect("state object")
            .insert("legacy_sram".to_string(), serde_json::json!({}));
        let state_error = serde_json::from_value::<GameState>(state_json)
            .expect_err("game state must not accept legacy save fields")
            .to_string();
        assert!(
            state_error.contains("unknown field `legacy_sram`"),
            "{state_error}"
        );

        let mut runtime_json =
            serde_json::to_value(ScriptRuntimeMemory::default()).expect("runtime json");
        runtime_json
            .as_object_mut()
            .expect("runtime object")
            .insert(
                "fallback_script".to_string(),
                serde_json::json!("MomScript"),
            );
        let runtime_error = serde_json::from_value::<ScriptRuntimeMemory>(runtime_json)
            .expect_err("script runtime memory must not accept fallback script fields")
            .to_string();
        assert!(
            runtime_error.contains("unknown field `fallback_script`"),
            "{runtime_error}"
        );
    }

    #[test]
    fn frame_advancement_is_explicit() {
        let mut state = GameState::default();
        assert_eq!(state.frame(), Frame(0));
        assert_eq!(state.advance_frame(), Frame(1));
        assert_eq!(state.frame_counter, 1);
    }

    #[test]
    fn overworld_memory_serializes_exact_active_position_for_saves_and_sync() {
        let memory = OverworldMemory::Active {
            map_name: "PlayersHouse2F".to_string(),
            tile: TilePosition::new(3, 3),
            facing: Direction::Down,
            mode: MovementMode::Normal,
        };
        let json = serde_json::to_string(&memory).expect("serialize overworld memory");

        assert_eq!(
            json,
            r#"{"active":{"map_name":"PlayersHouse2F","tile":{"x":3,"y":3},"facing":"down","mode":"normal"}}"#
        );
        assert_eq!(
            serde_json::from_str::<OverworldMemory>(&json).expect("deserialize overworld memory"),
            memory
        );
    }

    fn scene_table() -> MapSceneTable {
        MapSceneTable {
            scenes: vec![
                crate::map::MapScene {
                    scene_id: "SCENE_ELMSLAB_MEET_ELM".to_string(),
                    script_name: Some("ElmsLabMeetElmScene".to_string()),
                },
                crate::map::MapScene {
                    scene_id: "SCENE_ELMSLAB_NOOP".to_string(),
                    script_name: None,
                },
            ],
        }
    }

    #[test]
    fn scene_memory_initializes_from_explicit_pack_order() {
        let mut memory = SceneMemory::default();
        let status = memory
            .enter_map("ElmsLab", &scene_table())
            .expect("initialize scene");

        assert_eq!(status.scene_name, "SCENE_ELMSLAB_MEET_ELM");
        assert_eq!(status.scene_index, 0);
        assert_eq!(status.script_name, Some("ElmsLabMeetElmScene".to_string()));
        assert_eq!(
            memory.map_scenes["ElmsLab"],
            "SCENE_ELMSLAB_MEET_ELM".to_string()
        );
        assert_eq!(memory.map_scene_indices["ElmsLab"], 0);
        assert_eq!(memory.scene_name, "SCENE_ELMSLAB_MEET_ELM");
    }

    #[test]
    fn scene_memory_sets_exact_scene_without_case_coercion() {
        let mut memory = SceneMemory::default();
        memory
            .enter_map("ElmsLab", &scene_table())
            .expect("initialize scene");
        let status = memory
            .set_current_scene("SCENE_ELMSLAB_NOOP", &scene_table())
            .expect("set scene");

        assert_eq!(status.scene_index, 1);
        assert_eq!(status.script_name, None);
        assert_eq!(memory.scene_name, "SCENE_ELMSLAB_NOOP");
        assert_eq!(memory.map_scene_indices["ElmsLab"], 1);
        assert_eq!(
            memory.set_current_scene("scene_elmslab_noop", &scene_table()),
            Err(SceneError::UnknownScene {
                map_name: "ElmsLab".to_string(),
                scene_name: "scene_elmslab_noop".to_string(),
            })
        );
    }

    #[test]
    fn scene_memory_rejects_empty_scene_tables() {
        let mut memory = SceneMemory::default();
        assert_eq!(
            memory.enter_map("Route29", &MapSceneTable::default()),
            Err(SceneError::EmptySceneTable {
                map_name: "Route29".to_string(),
            })
        );
    }

    #[test]
    fn event_flags_are_exact_strings_without_case_coercion() {
        let mut flags = EventFlagMemory::default();
        flags
            .set_event_flag("EVENT_ROUTE_29_POTION", true)
            .expect("set flag");

        assert_eq!(flags.is_event_flag_set("EVENT_ROUTE_29_POTION"), Ok(true));
        assert_eq!(flags.is_event_flag_set("event_route_29_potion"), Ok(false));
        assert_eq!(
            flags.active_event_flags().cloned().collect::<Vec<_>>(),
            vec!["EVENT_ROUTE_29_POTION".to_string()]
        );
    }

    #[test]
    fn script_flags_route_engine_prefixes_to_engine_store() {
        let mut flags = EventFlagMemory::default();
        flags
            .set_script_flag("ENGINE_ZEPHYRBADGE", true)
            .expect("set engine flag");
        flags
            .set_script_flag("EVENT_BEAT_YOUNGSTER_JOEY", true)
            .expect("set event flag");

        assert_eq!(flags.is_engine_flag_set("ENGINE_ZEPHYRBADGE"), Ok(true));
        assert_eq!(flags.is_event_flag_set("ENGINE_ZEPHYRBADGE"), Ok(false));
        assert_eq!(
            flags.is_event_flag_set("EVENT_BEAT_YOUNGSTER_JOEY"),
            Ok(true)
        );
        flags
            .clear_script_flag("ENGINE_ZEPHYRBADGE")
            .expect("clear engine flag");
        assert_eq!(flags.is_engine_flag_set("ENGINE_ZEPHYRBADGE"), Ok(false));
    }

    #[test]
    fn empty_flag_names_are_errors() {
        let mut flags = EventFlagMemory::default();
        assert_eq!(
            flags.set_event_flag("", true),
            Err(EventFlagError::EmptyFlagName)
        );
        assert_eq!(
            flags.is_script_flag_set(""),
            Err(EventFlagError::EmptyFlagName)
        );
    }
}
