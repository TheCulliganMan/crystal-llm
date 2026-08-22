use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use crate::input::{B_PAD_A, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP};
use crate::map::{BackgroundEvent, CoordEvent, MapConnection, MapEvents, ObjectEvent, WarpEvent};
use crate::multiplayer::{
    MultiplayerMessageError, OverworldPresence, PlayerInputFrame, PresenceEntityType, fnv1a32,
};
#[cfg(test)]
use crate::random::CrystalRandomState;
use crate::random::{CrystalRandom, DividerSource, Random};
use crate::state::{EventFlagMemory, GameState, RoamingPokemonState};
use crate::systems::special_routines::BugContestEncounterEntry;

use super::collision::{
    Terrain, TilesetCollision, can_jump_ledge, describe_collision, directional_warp_facing,
    is_direction_blocked, is_direction_blocked_leaving, is_warp_permission, permissions,
    sample_collision, standard_interaction_script,
};
use super::encounters::{
    EncounterError, EncounterMusicModifiers, EncounterSlotTables, EncounterSurface,
    ResolvedWildEncounter, TimeOfDay, WildEncounter, WildEncounterData, apply_cleanse_tag_effect,
    apply_encounter_music_effect, passes_encounter_roll, percent_to_byte, select_wild_encounter,
};
use super::map::{Direction, METATILE_WIDTH, OverworldMapData, TilePosition};
use super::movement::{
    DEFAULT_RUNTIME_TILE_STRIDE, LedgeJumpOutcome, MovementMode, OccupiedTile, PlayerMovementState,
    StepOptions, StepOutcome, attempt_ledge_jump_with_occupied_tiles,
    attempt_step_with_occupied_tiles, checked_move_by_stride,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldSnapshot {
    pub frame: u64,
    pub map_name: String,
    pub tile: TilePosition,
    pub facing: Direction,
    pub mode: MovementMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldSession {
    pub frame: u64,
    pub map: OverworldMapData,
    pub map_events: MapEvents,
    pub objects: Vec<ObjectEvent>,
    pub object_runtime_tiles: BTreeMap<String, TilePosition>,
    #[serde(default)]
    pub object_last_runtime_tiles: BTreeMap<String, TilePosition>,
    #[serde(default)]
    pub object_last_tiles_occupied_until_frame: BTreeMap<String, u64>,
    pub object_facings: BTreeMap<String, Direction>,
    /// Remaining LCD frames before an autonomous object may select its next
    /// movement function. Crystal stores this independently in each object's
    /// OBJECT_STEP_DURATION byte.
    #[serde(default)]
    pub object_step_durations: BTreeMap<String, u8>,
    /// Objects whose current duration is the visible stride itself. On
    /// landing, ContinueWalk performs the separate Random call that installs
    /// the slow idle duration.
    #[serde(default)]
    pub object_pending_random_wait: BTreeSet<String>,
    #[serde(default)]
    pub initialized_fixed_spin_objects: BTreeSet<String>,
    pub following: Option<OverworldFollowState>,
    /// The movement command the follower consumes when its leader completes
    /// the next step. Crystal's follower queue survives separate
    /// `applymovement` programs; retaining it here prevents each program from
    /// reconstructing a lossy direction-only approximation from geometry.
    #[serde(default)]
    pub following_queued_step: Option<FollowQueuedStep>,
    pub last_talked_object_identifier: Option<String>,
    pub player_hidden: bool,
    pub hidden_event_flags: BTreeSet<String>,
    pub hidden_object_identifiers: BTreeSet<String>,
    /// Explicit `appear` commands can keep a loaded object visible even when
    /// its backing event flag has changed. Ordinary setevent/clearevent only
    /// affect which objects are loaded on the next map entry.
    #[serde(default)]
    pub shown_object_identifiers: BTreeSet<String>,
    /// Fresh map sessions apply event/time visibility once; later flag
    /// mutations retain that loaded roster until an explicit object command
    /// or the next map entry.
    #[serde(default)]
    pub object_visibility_initialized: bool,
    /// The time used by Crystal's object scheduler (`hram_y`).  This lives on
    /// the session because collision, interaction, and trainer sight all
    /// query the same visible-object set.
    #[serde(default = "default_session_time_of_day")]
    pub time_of_day: TimeOfDay,
    pub tileset: TilesetCollision,
    pub player: PlayerMovementState,
    #[serde(default)]
    pub last_step_direction: Option<Direction>,
    /// While a player step is active, Crystal collision owns both the
    /// destination (`OBJECT_MAP_*`) and origin (`OBJECT_LAST_MAP_*`).
    #[serde(default)]
    pub player_last_runtime_tile: Option<TilePosition>,
    #[serde(default)]
    pub player_last_tile_occupied_until_frame: u64,
}

const fn default_session_time_of_day() -> TimeOfDay {
    TimeOfDay::Day
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldFollowState {
    pub leader_object_id: String,
    pub follower_object_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FollowQueuedStep {
    pub direction: Direction,
    pub stride: i16,
    pub duration: u8,
    pub jump: bool,
    pub standing_frame: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpTrigger {
    pub map_name: String,
    pub tile: TilePosition,
    pub permission: u8,
    pub warp: WarpEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldStepResult {
    pub outcome: StepOutcome,
    pub warp: Option<WarpTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OverworldInputAction {
    Interact(OverworldInteraction),
    Step(OverworldStepResult),
    LedgeJump(OverworldLedgeJumpResult),
    NoInteraction,
    Idle,
}

impl OverworldInputAction {
    pub const fn moves_player(&self) -> bool {
        matches!(
            self,
            Self::Step(OverworldStepResult {
                outcome: StepOutcome::Moved { .. },
                ..
            }) | Self::LedgeJump(OverworldLedgeJumpResult {
                outcome: LedgeJumpOutcome::Jumped { .. },
                ..
            })
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldInputResult {
    pub frame: u64,
    pub joypad_mask: u8,
    pub action: OverworldInputAction,
    pub coord_event: Option<CoordEventTrigger>,
    pub trainer_sight: Option<OverworldInteraction>,
    pub connection: Option<ConnectionTrigger>,
    pub snapshot: OverworldSnapshot,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OverworldInputError {
    #[error("overworld input frame {input_frame} does not match session frame {session_frame}")]
    FrameMismatch {
        session_frame: u64,
        input_frame: u64,
    },
    #[error("overworld joypad mask {mask:#010b} has conflicting direction buttons")]
    ConflictingDirections { mask: u8 },
    #[error(transparent)]
    ObjectCoordinate(#[from] OverworldObjectCoordinateError),
    #[error(transparent)]
    EventCoordinate(#[from] OverworldEventCoordinateError),
    #[error(transparent)]
    Coordinate(#[from] OverworldCoordinateError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverworldObjectCoordinateError {
    #[error("object '{object_id}' has out-of-range runtime coordinates ({x}, {y})")]
    OutOfRange { object_id: String, x: u16, y: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomousObjectAdvanceError<E> {
    Coordinate(OverworldObjectCoordinateError),
    Divider(E),
}

impl<E: std::fmt::Display> std::fmt::Display for AutonomousObjectAdvanceError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Coordinate(error) => error.fmt(formatter),
            Self::Divider(error) => {
                write!(formatter, "autonomous movement divider source: {error}")
            }
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for AutonomousObjectAdvanceError<E> {}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverworldEventCoordinateError {
    #[error("warp {index} has out-of-range runtime coordinates ({x}, {y})")]
    WarpOutOfRange { index: u16, x: u16, y: u16 },
    #[error("background event '{script}' has out-of-range runtime coordinates ({x}, {y})")]
    BackgroundOutOfRange { script: String, x: u16, y: u16 },
    #[error("coord event '{script}' has out-of-range runtime coordinates ({x}, {y})")]
    CoordOutOfRange { script: String, x: u16, y: u16 },
}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverworldCoordinateError {
    #[error(transparent)]
    Object(#[from] OverworldObjectCoordinateError),
    #[error(transparent)]
    Event(#[from] OverworldEventCoordinateError),
    #[error(
        "runtime movement stride {stride_tiles} does not match runtime stride {metatile_width}"
    )]
    InvalidRuntimeStride {
        stride_tiles: i16,
        metatile_width: i16,
    },
    #[error("runtime tile ({x}, {y}) is not aligned to metatile width {metatile_width}")]
    UnalignedRuntimeTile { x: i16, y: i16, metatile_width: i16 },
    #[error(
        "runtime tile movement from ({x}, {y}) facing {facing:?} overflows supported coordinates"
    )]
    RuntimeTileOverflow { x: i16, y: i16, facing: Direction },
    #[error("map {map_name} runtime tile bounds overflow supported coordinates")]
    MapBoundsOverflow { map_name: String },
    #[error("map {map_name} has unsupported connection direction '{direction}'")]
    UnsupportedConnectionDirection { map_name: String, direction: String },
    #[error("map {map_name} has duplicate connection direction '{direction}'")]
    DuplicateConnectionDirection { map_name: String, direction: String },
    #[error("runtime tile ({x}, {y}) matches multiple connections on map {map_name}")]
    AmbiguousConnectionBoundary { map_name: String, x: i16, y: i16 },
    #[error("follow object '{object_id}' is missing from the overworld session")]
    FollowObjectMissing { object_id: String },
    #[error("follow object '{object_id}' cannot be moved to unsaveable runtime tile ({x}, {y})")]
    FollowPositionUnsavable { object_id: String, x: i16, y: i16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldLedgeJumpResult {
    pub outcome: LedgeJumpOutcome,
    pub warp: Option<WarpTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum OverworldInteractionTarget {
    Object {
        object_index: u16,
        object_identifier: Option<String>,
        object_type: String,
    },
    Background {
        event_type: String,
    },
    Collision {
        permission: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldInteraction {
    pub map_name: String,
    pub player_tile: TilePosition,
    pub facing: Direction,
    pub target_tile: TilePosition,
    pub script: String,
    pub target: OverworldInteractionTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordEventTrigger {
    pub map_name: String,
    pub tile: TilePosition,
    pub scene_id: String,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EncounterCheckOptions {
    pub time: TimeOfDay,
    pub music_token: Option<String>,
    pub has_cleanse_tag: bool,
    pub active_repel_item: Option<String>,
    pub lead_party_level: Option<u8>,
    /// CAVE and DUNGEON environments permit encounters on ordinary land,
    /// except ice, without requiring a grass collision byte.
    #[serde(default)]
    pub land_encounters_on_any_land: bool,
}

impl Default for EncounterCheckOptions {
    fn default() -> Self {
        Self {
            time: TimeOfDay::Day,
            music_token: None,
            has_cleanse_tag: false,
            active_repel_item: None,
            lead_party_level: None,
            land_encounters_on_any_land: false,
        }
    }
}

/// Source-owned encounter tables and WRAM inputs for the exact chooser.
///
/// Unlike the legacy `EncounterCheckOptions` candidate vectors, this keeps
/// all three raw roaming slots and the current numeric map together so the
/// chooser itself proves the selected slot is on the active map.  Bug Contest
/// rows are borrowed directly from the required exported configuration.
#[derive(Debug, Clone, Copy)]
pub struct ExactEncounterContext<'a> {
    pub roaming_pokemon: &'a [RoamingPokemonState; 3],
    pub current_map: (u8, u8),
    pub bug_contest_encounters: Option<&'a [BugContestEncounterEntry]>,
    pub unlocked_unown_sets: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WildEncounterRoll {
    pub map_name: String,
    pub tile: TilePosition,
    pub surface: EncounterSurface,
    pub time: TimeOfDay,
    pub threshold: u8,
    pub encounter_roll: u8,
    pub slot_percent_roll: Option<u8>,
    pub level_roll: Option<u8>,
    /// Exact wRoamMon slot selected by CheckEncounterRoamMon, if any.
    pub roaming_slot: Option<u8>,
    pub resolved: Option<ResolvedWildEncounter>,
    pub repelled_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactEncounterError<E> {
    Encounter(EncounterError),
    Divider(E),
}

impl<E> From<EncounterError> for ExactEncounterError<E> {
    fn from(error: EncounterError) -> Self {
        Self::Encounter(error)
    }
}

impl<E: std::fmt::Display> std::fmt::Display for ExactEncounterError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encounter(error) => error.fmt(formatter),
            Self::Divider(error) => write!(formatter, "wild encounter divider source: {error}"),
        }
    }
}

impl<E: std::fmt::Debug + std::fmt::Display> std::error::Error for ExactEncounterError<E> {}

pub fn leading_usable_party_level(state: &GameState) -> Option<u8> {
    state
        .storage
        .party
        .pokemon
        .iter()
        .flatten()
        .find(|pokemon| pokemon.hp > 0)
        .map(|pokemon| pokemon.level)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpDestination {
    pub map_name: String,
    pub tile: TilePosition,
    pub warp: WarpEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpTransition {
    pub trigger: WarpTrigger,
    pub destination: WarpDestination,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionTrigger {
    pub map_name: String,
    pub tile: TilePosition,
    pub connection: MapConnection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionDestination {
    pub map_name: String,
    pub tile: TilePosition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionTransition {
    pub trigger: ConnectionTrigger,
    pub destination: ConnectionDestination,
}

impl OverworldSession {
    pub fn new(
        map: OverworldMapData,
        tileset: TilesetCollision,
        player_tile: TilePosition,
    ) -> Self {
        Self::with_events(map, MapEvents::default(), tileset, player_tile)
    }

    pub fn with_events(
        map: OverworldMapData,
        map_events: MapEvents,
        tileset: TilesetCollision,
        player_tile: TilePosition,
    ) -> Self {
        Self::with_events_and_objects(map, map_events, Vec::new(), tileset, player_tile)
    }

    pub fn with_events_and_objects(
        map: OverworldMapData,
        map_events: MapEvents,
        objects: Vec<ObjectEvent>,
        tileset: TilesetCollision,
        player_tile: TilePosition,
    ) -> Self {
        let object_facings = initial_object_facings(&objects);
        Self {
            frame: 0,
            map,
            map_events,
            objects,
            object_runtime_tiles: BTreeMap::new(),
            object_last_runtime_tiles: BTreeMap::new(),
            object_last_tiles_occupied_until_frame: BTreeMap::new(),
            object_facings,
            object_step_durations: BTreeMap::new(),
            object_pending_random_wait: BTreeSet::new(),
            initialized_fixed_spin_objects: BTreeSet::new(),
            following: None,
            following_queued_step: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            shown_object_identifiers: BTreeSet::new(),
            object_visibility_initialized: false,
            time_of_day: default_session_time_of_day(),
            tileset,
            player: PlayerMovementState::new(player_tile),
            last_step_direction: None,
            player_last_runtime_tile: None,
            player_last_tile_occupied_until_frame: 0,
        }
    }

    pub fn with_hidden_event_flags(mut self, hidden_event_flags: BTreeSet<String>) -> Self {
        self.hidden_event_flags = hidden_event_flags;
        self
    }

    pub fn with_event_flag_memory(mut self, flags: &EventFlagMemory) -> Self {
        self.hidden_event_flags = flags.active_event_flags().cloned().collect();
        self.object_visibility_initialized = true;
        self
    }

    pub fn sync_event_flag_memory(&mut self, flags: &EventFlagMemory) {
        if !self.object_visibility_initialized {
            self.hidden_event_flags = flags.active_event_flags().cloned().collect();
            self.object_visibility_initialized = true;
            return;
        }
        let visibility = self.loaded_object_visibility();
        self.hidden_event_flags = flags.active_event_flags().cloned().collect();
        self.retain_loaded_object_visibility(&visibility);
    }

    pub fn hide_loaded_objects_with_event_flag(&mut self, event_flag: &str) {
        for object_id in self.objects.iter().filter_map(|object| {
            (object.event_flag == event_flag)
                .then(|| object.object_identifier.clone())
                .flatten()
        }) {
            self.shown_object_identifiers.remove(&object_id);
            self.hidden_object_identifiers.insert(object_id);
        }
    }

    pub fn current_encounter_surface_checked(
        &self,
    ) -> Result<Option<EncounterSurface>, EncounterError> {
        encounter_surface_for_player_tile_checked(self, false)
    }

    pub fn current_encounter_surface_checked_with_land_encounters(
        &self,
        land_encounters_on_any_land: bool,
    ) -> Result<Option<EncounterSurface>, EncounterError> {
        encounter_surface_for_player_tile_checked(self, land_encounters_on_any_land)
    }

    pub fn current_encounter_surface(&self) -> Option<EncounterSurface> {
        self.current_encounter_surface_checked()
            .expect("encounter surface query requires valid runtime coordinate state")
    }

    pub fn snapshot(&self) -> OverworldSnapshot {
        OverworldSnapshot {
            frame: self.frame,
            map_name: self.map.name.clone(),
            tile: self.player.tile,
            facing: self.player.facing,
            mode: self.player.mode,
        }
    }

    pub fn state_hash(&self) -> u32 {
        let snapshot = self.snapshot();
        let payload = format!(
            "{}|{}|{}|{}|{:?}|{:?}",
            snapshot.frame,
            snapshot.map_name,
            snapshot.tile.x,
            snapshot.tile.y,
            snapshot.facing,
            snapshot.mode
        );
        fnv1a32(&payload)
    }

    pub fn presence(
        &self,
        user_id: impl Into<String>,
        player_name: impl Into<String>,
        updated_at_ms: u64,
    ) -> Result<OverworldPresence, MultiplayerMessageError> {
        OverworldPresence::new(
            user_id,
            player_name,
            PresenceEntityType::Player,
            self.map.name.clone(),
            self.player.tile,
            self.player.facing,
            updated_at_ms,
        )
    }

    pub fn step(&mut self, direction: Direction, options: StepOptions) -> StepOutcome {
        self.step_checked(direction, options)
            .expect("overworld step requires valid runtime coordinate state")
    }

    pub fn step_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<StepOutcome, OverworldCoordinateError> {
        require_runtime_stride(options.stride_tiles)?;
        self.require_checked_tile_bounds()?;
        let occupied_tiles = self.occupied_tiles_checked()?;
        self.validate_follow_after_entity_move("PLAYER", self.player.tile)?;
        let outcome = attempt_step_with_occupied_tiles(
            &mut self.player,
            direction,
            &self.map,
            &self.tileset,
            options,
            &occupied_tiles,
        );
        if let StepOutcome::Moved {
            from,
            to,
            speed_multiplier,
        } = outcome
        {
            self.update_follow_after_entity_move("PLAYER", from, to);
            self.last_step_direction = Some(direction);
            self.player_last_runtime_tile = Some(from);
            self.player_last_tile_occupied_until_frame = self
                .frame
                .saturating_add(u64::from((8 / speed_multiplier.max(1)).max(1)));
        }
        self.frame += 1;
        Ok(outcome)
    }

    /// Execute CheckTile's direct `.DoStep` path for current and directional
    /// walk collisions. These source movement functions do not enter
    /// TryStep, so they deliberately skip destination terrain, ledge, and
    /// object collision checks. Ice is not a caller: it continues through
    /// TryStep and retains ordinary collision behavior.
    pub fn forced_tile_step_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<StepOutcome, OverworldCoordinateError> {
        require_runtime_stride(options.stride_tiles)?;
        self.require_checked_tile_bounds()?;
        self.validate_follow_after_entity_move("PLAYER", self.player.tile)?;
        self.player.facing = direction;
        let from = self.player.tile;
        let Some(to) = checked_move_by_stride(from, direction, options.stride_tiles) else {
            self.frame += 1;
            return Ok(StepOutcome::RuntimeTileOverflow {
                from,
                facing: direction,
            });
        };
        self.player.tile = to;
        self.update_follow_after_entity_move("PLAYER", from, to);
        self.last_step_direction = Some(direction);
        self.player_last_runtime_tile = Some(from);
        self.player_last_tile_occupied_until_frame = self.frame.saturating_add(8);
        self.frame += 1;
        Ok(StepOutcome::Moved {
            from,
            to,
            speed_multiplier: 1,
        })
    }

    pub fn ledge_jump(&mut self, direction: Direction, options: StepOptions) -> LedgeJumpOutcome {
        self.ledge_jump_checked(direction, options)
            .expect("overworld ledge jump requires valid runtime coordinate state")
    }

    pub fn ledge_jump_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<LedgeJumpOutcome, OverworldCoordinateError> {
        require_runtime_stride(options.stride_tiles)?;
        self.require_checked_tile_bounds()?;
        let occupied_tiles = self.occupied_tiles_checked()?;
        self.validate_follow_after_entity_move("PLAYER", self.player.tile)?;
        let outcome = attempt_ledge_jump_with_occupied_tiles(
            &mut self.player,
            direction,
            &self.map,
            &self.tileset,
            options,
            &occupied_tiles,
        );
        if let LedgeJumpOutcome::Jumped {
            from,
            to,
            speed_multiplier,
            ..
        } = outcome
        {
            self.update_follow_after_entity_move("PLAYER", from, to);
            self.last_step_direction = Some(direction);
            self.player_last_runtime_tile = Some(from);
            self.player_last_tile_occupied_until_frame = self
                .frame
                .saturating_add(u64::from((16 / speed_multiplier.max(1)).max(1)));
        }
        self.frame += 1;
        Ok(outcome)
    }

    fn require_checked_tile_bounds(&self) -> Result<(), OverworldCoordinateError> {
        self.map.checked_tile_bounds().map(|_| ()).ok_or_else(|| {
            OverworldCoordinateError::MapBoundsOverflow {
                map_name: self.map.name.clone(),
            }
        })
    }

    pub fn update_follow_after_entity_move(
        &mut self,
        moved_object_id: &str,
        from: TilePosition,
        to: TilePosition,
    ) {
        let Some(following) = self.following.clone() else {
            return;
        };
        if following.leader_object_id != moved_object_id {
            return;
        }
        let current = if following.follower_object_id == "PLAYER" {
            Some(self.player.tile)
        } else {
            self.object_runtime_tiles
                .get(&following.follower_object_id)
                .copied()
                .or_else(|| {
                    self.objects
                        .iter()
                        .enumerate()
                        .find(|(_, object)| {
                            object.object_identifier.as_deref()
                                == Some(following.follower_object_id.as_str())
                        })
                        .and_then(|(index, object)| {
                            self.object_runtime_tile_checked(index, object).ok()
                        })
                })
        };
        let queued = self.following_queued_step.or_else(|| {
            current.and_then(|tile| {
                direction_between_tiles(tile, from).map(|direction| FollowQueuedStep {
                    direction,
                    stride: DEFAULT_RUNTIME_TILE_STRIDE,
                    duration: 8,
                    jump: false,
                    standing_frame: false,
                })
            })
        });
        if let (Some(current), Some(queued)) = (current, queued) {
            if let Some(tile) = checked_move_by_stride(current, queued.direction, queued.stride) {
                self.set_follow_entity_tile(&following.follower_object_id, tile);
                self.set_follow_entity_facing(&following.follower_object_id, queued.direction);
            }
        }
        if let Some(direction) = direction_between_tiles(from, to) {
            let stride = (to.x - from.x).abs().max((to.y - from.y).abs());
            self.following_queued_step = Some(FollowQueuedStep {
                direction,
                stride,
                duration: 8,
                jump: false,
                standing_frame: false,
            });
        }
    }

    fn validate_follow_after_entity_move(
        &self,
        moved_object_id: &str,
        _from: TilePosition,
    ) -> Result<(), OverworldCoordinateError> {
        let Some(following) = self.following.as_ref() else {
            return Ok(());
        };
        if following.leader_object_id != moved_object_id {
            return Ok(());
        }
        if following.follower_object_id == "PLAYER" {
            return Ok(());
        }
        self.objects
            .iter()
            .any(|object| {
                object.object_identifier.as_deref() == Some(following.follower_object_id.as_str())
            })
            .then_some(())
            .ok_or_else(|| OverworldCoordinateError::FollowObjectMissing {
                object_id: following.follower_object_id.clone(),
            })
    }

    fn set_follow_entity_tile(&mut self, object_id: &str, tile: TilePosition) {
        if object_id == "PLAYER" {
            self.player.tile = tile;
            return;
        }
        if let Some(object) = self
            .objects
            .iter_mut()
            .find(|object| object.object_identifier.as_deref() == Some(object_id))
        {
            if let Some(identifier) = object.object_identifier.clone() {
                self.object_runtime_tiles.insert(identifier, tile);
            }
        }
    }

    pub fn object_runtime_tile_checked(
        &self,
        index: usize,
        object: &ObjectEvent,
    ) -> Result<TilePosition, OverworldObjectCoordinateError> {
        if let Some(identifier) = object.object_identifier.as_ref() {
            if let Some(tile) = self.object_runtime_tiles.get(identifier) {
                return Ok(*tile);
            }
        }
        object_tile_position_checked(object)
            .ok_or_else(|| object_coordinate_out_of_range(index, object))
    }

    pub fn object_runtime_tile_by_id(
        &self,
        object_id: &str,
    ) -> Result<TilePosition, OverworldObjectCoordinateError> {
        if let Some(tile) = self.object_runtime_tiles.get(object_id) {
            return Ok(*tile);
        }
        let (index, object) = self
            .objects
            .iter()
            .enumerate()
            .find(|(_, object)| object.object_identifier.as_deref() == Some(object_id))
            .ok_or_else(|| OverworldObjectCoordinateError::OutOfRange {
                object_id: object_id.to_string(),
                x: 0,
                y: 0,
            })?;
        self.object_runtime_tile_checked(index, object)
    }

    pub fn set_object_runtime_tile(
        &mut self,
        object_id: &str,
        tile: TilePosition,
    ) -> Result<(), OverworldObjectCoordinateError> {
        if !self
            .objects
            .iter()
            .any(|object| object.object_identifier.as_deref() == Some(object_id))
        {
            return Err(OverworldObjectCoordinateError::OutOfRange {
                object_id: object_id.to_string(),
                x: 0,
                y: 0,
            });
        }
        self.object_runtime_tiles
            .insert(object_id.to_string(), tile);
        Ok(())
    }

    pub fn set_object_runtime_facing(
        &mut self,
        object_id: &str,
        direction: Direction,
    ) -> Result<(), OverworldObjectCoordinateError> {
        if !self
            .objects
            .iter()
            .any(|object| object.object_identifier.as_deref() == Some(object_id))
        {
            return Err(OverworldObjectCoordinateError::OutOfRange {
                object_id: object_id.to_string(),
                x: 0,
                y: 0,
            });
        }
        self.object_facings.insert(object_id.to_string(), direction);
        Ok(())
    }

    pub fn set_player_facing(&mut self, direction: Direction) {
        self.player.facing = direction;
    }

    fn set_follow_entity_facing(&mut self, object_id: &str, direction: Direction) {
        if object_id == "PLAYER" {
            self.player.facing = direction;
        } else {
            self.object_facings.insert(object_id.to_string(), direction);
        }
    }

    pub fn occupied_tiles(&self) -> Vec<OccupiedTile> {
        self.occupied_tiles_checked()
            .expect("visible object coordinates must be valid runtime coordinate state")
    }

    pub fn occupied_tiles_checked(
        &self,
    ) -> Result<Vec<OccupiedTile>, OverworldObjectCoordinateError> {
        let mut occupied = Vec::new();
        for (index, object) in self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| self.is_object_visible(object))
        {
            let tile = self.object_runtime_tile_checked(index, object)?;
            occupied.push(OccupiedTile {
                tile,
                object_identifier: object.object_identifier.clone(),
            });
        }
        for (object_id, tile) in &self.object_last_runtime_tiles {
            let retained = self
                .object_last_tiles_occupied_until_frame
                .get(object_id)
                .is_some_and(|until_frame| self.frame < *until_frame);
            if retained
                && self.objects.iter().any(|object| {
                    object.object_identifier.as_deref() == Some(object_id.as_str())
                        && self.is_object_visible(object)
                })
                && !occupied.iter().any(|entry| {
                    entry.tile == *tile
                        && entry.object_identifier.as_deref() == Some(object_id.as_str())
                })
            {
                occupied.push(OccupiedTile {
                    tile: *tile,
                    object_identifier: Some(object_id.clone()),
                });
            }
        }
        Ok(occupied)
    }

    pub fn push_strength_boulder_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<Option<String>, OverworldObjectCoordinateError> {
        let Some(facing_tile) =
            checked_move_by_stride(self.player.tile, direction, options.stride_tiles)
        else {
            return Ok(None);
        };
        let Some((object_slot, object)) = self.visible_object_at_checked(facing_tile)? else {
            return Ok(None);
        };
        if object.spritemovedata != "SPRITEMOVEDATA_STRENGTH_BOULDER" {
            return Ok(None);
        }
        let Some(object_id) = object.object_identifier.clone() else {
            return Ok(None);
        };
        let object_index = usize::from(object_slot.saturating_sub(1));
        let object_tile = self.object_runtime_tile_checked(object_index, object)?;
        let occupied_tiles = self
            .objects
            .iter()
            .enumerate()
            .filter(|(index, object)| *index != object_index && self.is_object_visible(object))
            .map(|(index, object)| {
                self.object_runtime_tile_checked(index, object)
                    .map(|tile| OccupiedTile {
                        tile,
                        object_identifier: object.object_identifier.clone(),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut player_probe = self.player.clone();
        if !matches!(
            attempt_step_with_occupied_tiles(
                &mut player_probe,
                direction,
                &self.map,
                &self.tileset,
                options,
                &occupied_tiles,
            ),
            StepOutcome::Moved { .. }
        ) {
            return Ok(None);
        }
        let mut boulder = PlayerMovementState::new(object_tile);
        let outcome = attempt_step_with_occupied_tiles(
            &mut boulder,
            direction,
            &self.map,
            &self.tileset,
            options,
            &occupied_tiles,
        );
        if !matches!(outcome, StepOutcome::Moved { .. }) {
            return Ok(None);
        }
        self.object_runtime_tiles
            .insert(object_id.clone(), boulder.tile);
        self.object_facings.insert(object_id.clone(), direction);
        Ok(Some(object_id))
    }

    pub fn is_object_visible(&self, object: &ObjectEvent) -> bool {
        if object
            .object_identifier
            .as_ref()
            .is_some_and(|object_id| self.hidden_object_identifiers.contains(object_id))
        {
            return false;
        }
        if object
            .object_identifier
            .as_ref()
            .is_some_and(|object_id| self.shown_object_identifiers.contains(object_id))
        {
            return true;
        }
        self.is_object_visible_without_identifier_override(object)
    }

    fn is_object_visible_without_identifier_override(&self, object: &ObjectEvent) -> bool {
        if object.event_flag != "-1" && self.hidden_event_flags.contains(&object.event_flag) {
            return false;
        }
        object_visible_at_time(object.hram_y, self.time_of_day)
    }

    pub fn set_time_of_day(&mut self, time_of_day: TimeOfDay) {
        if !self.object_visibility_initialized {
            self.time_of_day = time_of_day;
            return;
        }
        let visibility = self.loaded_object_visibility();
        self.time_of_day = time_of_day;
        self.retain_loaded_object_visibility(&visibility);
    }

    fn loaded_object_visibility(&self) -> Vec<(String, bool)> {
        self.objects
            .iter()
            .filter_map(|object| {
                object
                    .object_identifier
                    .as_ref()
                    .map(|id| (id.clone(), self.is_object_visible(object)))
            })
            .collect()
    }

    fn retain_loaded_object_visibility(&mut self, visibility: &[(String, bool)]) {
        for (object_id, was_visible) in visibility {
            let raw_visible = self
                .objects
                .iter()
                .find(|object| object.object_identifier.as_ref() == Some(object_id))
                .is_some_and(|object| self.is_object_visible_without_identifier_override(object));
            self.hidden_object_identifiers.remove(object_id);
            self.shown_object_identifiers.remove(object_id);
            if *was_visible && !raw_visible {
                self.shown_object_identifiers.insert(object_id.clone());
            } else if !*was_visible && raw_visible {
                self.hidden_object_identifiers.insert(object_id.clone());
            }
        }
    }

    pub fn forced_movement_direction(&self) -> Option<Direction> {
        let sample = sample_collision(&self.map, &self.tileset, self.player.tile)?;
        // `CheckTile` dispatches the complete HI_NYBBLE_CURRENT range and
        // indexes its four-direction table with the low two bits. Do not
        // narrow this to only the collision values that happen to be named.
        if (sample.permission & 0xf0) == (permissions::WATERFALL_RIGHT & 0xf0) {
            return match sample.permission & 0x03 {
                0 => Some(Direction::Right),
                1 => Some(Direction::Left),
                2 => Some(Direction::Up),
                _ => Some(Direction::Down),
            };
        }
        match sample.permission {
            permissions::WALK_RIGHT | permissions::WALK_RIGHT_ALT => Some(Direction::Right),
            permissions::WALK_LEFT | permissions::WALK_LEFT_ALT => Some(Direction::Left),
            permissions::WALK_UP | permissions::WALK_UP_ALT => Some(Direction::Up),
            permissions::WALK_DOWN
            | permissions::WALK_DOWN_ALT
            | permissions::DOOR
            | permissions::DOOR_79
            | permissions::STAIRCASE
            | permissions::CAVE => Some(Direction::Down),
            // Ice keeps the direction of the step that entered it.  Merely
            // facing a direction on an ice tile must not start sliding.
            permissions::ICE | permissions::ICE_2B => self.last_step_direction,
            _ => None,
        }
    }

    pub fn forced_current_direction(&self) -> Option<Direction> {
        let sample = sample_collision(&self.map, &self.tileset, self.player.tile)?;
        match sample.permission {
            permissions::CURRENT_RIGHT => Some(Direction::Right),
            permissions::CURRENT_LEFT => Some(Direction::Left),
            permissions::CURRENT_UP => Some(Direction::Up),
            permissions::CURRENT_DOWN => Some(Direction::Down),
            _ => None,
        }
    }

    /// Advance the frame-driven movement modes implemented by Crystal's
    /// `SpriteMovementData`. Script-controlled objects are left untouched;
    /// these walkers are the autonomous map-object behaviors.
    pub fn advance_autonomous_objects(&mut self) -> Result<(), OverworldObjectCoordinateError> {
        self.advance_autonomous_objects_with_rng(None)
    }

    pub fn advance_autonomous_objects_with_rng(
        &mut self,
        mut rng: Option<&mut Random>,
    ) -> Result<(), OverworldObjectCoordinateError> {
        self.advance_autonomous_objects_with_random_add(rng.as_mut().map(|rng| {
            move |_carry: bool| -> Result<u8, std::convert::Infallible> {
                Ok(rng.crystal_random_add_sub().0)
            }
        }))
        .map_err(|error| match error {
            AutonomousObjectAdvanceError::Coordinate(error) => error,
            AutonomousObjectAdvanceError::Divider(never) => match never {},
        })
    }

    pub fn advance_autonomous_objects_exact<S>(
        &mut self,
        rng: &mut CrystalRandom<&mut S>,
    ) -> Result<(), AutonomousObjectAdvanceError<S::Error>>
    where
        S: DividerSource + ?Sized,
    {
        self.advance_autonomous_objects_with_random_add(Some(|carry| {
            rng.random(carry)?;
            Ok(rng.state().add)
        }))
    }

    fn advance_autonomous_objects_with_random_add<E, F>(
        &mut self,
        mut random_add: Option<F>,
    ) -> Result<(), AutonomousObjectAdvanceError<E>>
    where
        F: FnMut(bool) -> Result<u8, E>,
    {
        let visible_indices: Vec<usize> = self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| self.is_object_visible(object))
            .filter(|(_, object)| {
                matches!(
                    object.spritemovedata.as_str(),
                    "SPRITEMOVEDATA_WALK_LEFT_RIGHT"
                        | "SPRITEMOVEDATA_WALK_UP_DOWN"
                        | "SPRITEMOVEDATA_WANDER"
                        | "SPRITEMOVEDATA_SWIM_WANDER"
                        | "SPRITEMOVEDATA_SPINCLOCKWISE"
                        | "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE"
                        | "SPRITEMOVEDATA_SPINRANDOM_SLOW"
                        | "SPRITEMOVEDATA_SPINRANDOM_FAST"
                )
            })
            .map(|(index, _)| index)
            .collect();
        // Runtime object coordinates commit atomically here, but Crystal keeps
        // OBJECT_LAST_MAP_* collision-owned through the visible stride. A
        // later object in this same scheduler batch must therefore not enter
        // a tile that an earlier object has just begun walking out of.
        let mut vacated_tiles = Vec::new();

        for index in visible_indices {
            let object = &self.objects[index];
            let Some(object_id) = object.object_identifier.as_deref() else {
                continue;
            };
            if let Some(duration) = self.object_step_durations.get_mut(object_id) {
                *duration = duration.wrapping_sub(1);
                if *duration != 0 {
                    continue;
                }
                self.object_step_durations.remove(object_id);
                if self.object_pending_random_wait.remove(object_id) {
                    let Some(random_add) = random_add.as_mut() else {
                        self.object_pending_random_wait
                            .insert(object_id.to_string());
                        self.object_step_durations.insert(object_id.to_string(), 1);
                        continue;
                    };
                    let duration_roll =
                        random_add(false).map_err(AutonomousObjectAdvanceError::Divider)?;
                    self.object_step_durations
                        .insert(object_id.to_string(), duration_roll & 0x7f);
                }
                // StepFunction_Sleep/ContinueWalk only returns control to the
                // movement function on the following object-update frame.
                continue;
            }
            let current = self
                .object_runtime_tile_checked(index, object)
                .map_err(AutonomousObjectAdvanceError::Coordinate)?;
            // Object movement uses the same one-runtime-tile stride as the
            // player.  METATILE_WIDTH is the map-art block size, not the
            // gameplay movement stride.
            let stride = DEFAULT_RUNTIME_TILE_STRIDE;
            let (min_x, max_x, min_y, max_y) = (
                object.x as i16 * stride - object.move_range_x as i16 * stride,
                object.x as i16 * stride + object.move_range_x as i16 * stride,
                object.y as i16 * stride - object.move_range_y as i16 * stride,
                object.y as i16 * stride + object.move_range_y as i16 * stride,
            );
            let movement = object.spritemovedata.as_str();
            let mut direction = *self
                .object_facings
                .get(object_id)
                .unwrap_or(&Direction::Down);
            if movement == "SPRITEMOVEDATA_SPINCLOCKWISE"
                || movement == "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE"
            {
                if self
                    .initialized_fixed_spin_objects
                    .insert(object_id.to_string())
                {
                    self.object_step_durations.insert(object_id.to_string(), 16);
                    continue;
                }
                direction = match (movement, direction) {
                    ("SPRITEMOVEDATA_SPINCLOCKWISE", Direction::Up) => Direction::Right,
                    ("SPRITEMOVEDATA_SPINCLOCKWISE", Direction::Right) => Direction::Down,
                    ("SPRITEMOVEDATA_SPINCLOCKWISE", Direction::Down) => Direction::Left,
                    ("SPRITEMOVEDATA_SPINCLOCKWISE", Direction::Left) => Direction::Up,
                    ("SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE", Direction::Up) => Direction::Left,
                    ("SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE", Direction::Left) => Direction::Down,
                    ("SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE", Direction::Down) => Direction::Right,
                    ("SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE", Direction::Right) => Direction::Up,
                    _ => Direction::Up,
                };
                self.object_facings.insert(object_id.to_string(), direction);
                self.object_step_durations.insert(object_id.to_string(), 16);
                continue;
            } else if movement == "SPRITEMOVEDATA_SPINRANDOM_SLOW"
                || movement == "SPRITEMOVEDATA_SPINRANDOM_FAST"
            {
                let Some(random_add) = random_add.as_mut() else {
                    continue;
                };
                let direction_roll =
                    random_add(false).map_err(AutonomousObjectAdvanceError::Divider)?;
                let previous_direction = direction;
                direction = match (direction_roll >> 2) & 3 {
                    0 => Direction::Down,
                    1 => Direction::Up,
                    2 => Direction::Left,
                    _ => Direction::Right,
                };
                if movement == "SPRITEMOVEDATA_SPINRANDOM_FAST"
                    && direction
                        == *self
                            .object_facings
                            .get(object_id)
                            .unwrap_or(&Direction::Down)
                {
                    direction = match direction {
                        Direction::Down => Direction::Right,
                        Direction::Up => Direction::Left,
                        Direction::Left => Direction::Up,
                        Direction::Right => Direction::Down,
                    };
                }
                self.object_facings.insert(object_id.to_string(), direction);
                let sampled_direction_byte = direction_roll & 0x0c;
                let previous_direction_byte = match previous_direction {
                    Direction::Down => 0,
                    Direction::Up => 4,
                    Direction::Left => 8,
                    Direction::Right => 12,
                };
                let duration_carry = movement == "SPRITEMOVEDATA_SPINRANDOM_FAST"
                    && sampled_direction_byte != previous_direction_byte
                    && sampled_direction_byte < previous_direction_byte;
                let duration_roll =
                    random_add(duration_carry).map_err(AutonomousObjectAdvanceError::Divider)?;
                let mask = if movement == "SPRITEMOVEDATA_SPINRANDOM_FAST" {
                    0x1f
                } else {
                    0x7f
                };
                self.object_step_durations
                    .insert(object_id.to_string(), duration_roll & mask);
                continue;
            } else if movement == "SPRITEMOVEDATA_WALK_LEFT_RIGHT" {
                let Some(random_add) = random_add.as_mut() else {
                    continue;
                };
                let roll = random_add(false).map_err(AutonomousObjectAdvanceError::Divider)?;
                direction = if roll & 1 == 0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
            } else if movement == "SPRITEMOVEDATA_WALK_UP_DOWN" {
                let Some(random_add) = random_add.as_mut() else {
                    continue;
                };
                let roll = random_add(false).map_err(AutonomousObjectAdvanceError::Divider)?;
                direction = if roll & 1 == 0 {
                    Direction::Down
                } else {
                    Direction::Up
                };
            } else {
                let Some(random_add) = random_add.as_mut() else {
                    continue;
                };
                let roll = random_add(false).map_err(AutonomousObjectAdvanceError::Divider)?;
                direction = match roll & 3 {
                    0 => Direction::Down,
                    1 => Direction::Up,
                    2 => Direction::Left,
                    _ => Direction::Right,
                };
            }
            let (dx, dy) = direction.delta();
            let target = TilePosition::new(current.x + dx * stride, current.y + dy * stride);
            let outside_range =
                target.x < min_x || target.x > max_x || target.y < min_y || target.y > max_y;
            let player_occupied = target == self.player.tile
                || (self.frame < self.player_last_tile_occupied_until_frame
                    && self.player_last_runtime_tile == Some(target));
            let occupied = self
                .objects
                .iter()
                .enumerate()
                .filter(|(other_index, other)| {
                    *other_index != index && self.is_object_visible(other)
                })
                .filter_map(|(other_index, other)| {
                    self.object_runtime_tile_checked(other_index, other).ok()
                })
                .any(|tile| tile == target)
                || self
                    .object_last_runtime_tiles
                    .iter()
                    .any(|(other_id, tile)| {
                        other_id != object_id
                            && *tile == target
                            && self
                                .object_last_tiles_occupied_until_frame
                                .get(other_id)
                                .is_some_and(|until_frame| self.frame < *until_frame)
                            && self.objects.iter().any(|other| {
                                other.object_identifier.as_deref() == Some(other_id.as_str())
                                    && self.is_object_visible(other)
                            })
                    })
                || vacated_tiles.contains(&target);
            self.object_facings.insert(object_id.to_string(), direction);
            let blocked_leaving = sample_collision(&self.map, &self.tileset, current)
                .is_some_and(|sample| is_direction_blocked_leaving(sample.permission, direction));
            let walkable = sample_collision(&self.map, &self.tileset, target)
                .map(|sample| {
                    if is_direction_blocked(sample.permission, direction) {
                        return false;
                    }
                    let terrain = describe_collision(sample.permission).terrain;
                    if movement == "SPRITEMOVEDATA_SWIM_WANDER" {
                        terrain == Terrain::Water
                    } else {
                        terrain == Terrain::Land
                    }
                })
                .unwrap_or(false);
            let moved =
                !outside_range && !player_occupied && !occupied && !blocked_leaving && walkable;
            if moved {
                self.object_runtime_tiles
                    .insert(object_id.to_string(), target);
                self.object_last_runtime_tiles
                    .insert(object_id.to_string(), current);
                self.object_last_tiles_occupied_until_frame
                    .insert(object_id.to_string(), self.frame.saturating_add(8));
                vacated_tiles.push(current);
            }
            if moved {
                self.object_step_durations.insert(object_id.to_string(), 8);
                self.object_pending_random_wait
                    .insert(object_id.to_string());
            } else if let Some(random_add) = random_add.as_mut() {
                let duration_roll =
                    random_add(false).map_err(AutonomousObjectAdvanceError::Divider)?;
                self.object_step_durations
                    .insert(object_id.to_string(), duration_roll & 0x7f);
            }
        }
        Ok(())
    }

    pub fn check_interaction_checked(
        &self,
        stride_tiles: i16,
    ) -> Result<Option<OverworldInteraction>, OverworldInputError> {
        let stride = require_runtime_stride(stride_tiles)?;
        let facing_tile = checked_move_by_stride(self.player.tile, self.player.facing, stride)
            .ok_or_else(|| {
                OverworldInputError::Coordinate(OverworldCoordinateError::RuntimeTileOverflow {
                    x: self.player.tile.x,
                    y: self.player.tile.y,
                    facing: self.player.facing,
                })
            })?;
        if facing_tile.x < 0 || facing_tile.y < 0 {
            return Ok(None);
        }

        if let Some((object_index, object)) =
            self.visible_object_at_checked(facing_tile)?
                .filter(|(_, object)| {
                    object_has_dispatchable_script(object)
                        && !self.object_is_visibly_walking(object)
                })
        {
            return Ok(Some(self.object_interaction(
                object_index,
                object,
                facing_tile,
            )));
        }

        let adjusted_tile = self.counter_adjusted_tile(facing_tile);
        if adjusted_tile != facing_tile {
            if let Some((object_index, object)) = self
                .visible_object_at_checked(adjusted_tile)?
                .filter(|(_, object)| {
                    object_has_dispatchable_script(object)
                        && !self.object_is_visibly_walking(object)
                })
            {
                return Ok(Some(self.object_interaction(
                    object_index,
                    object,
                    adjusted_tile,
                )));
            }
        }

        if let Some(event) = self
            .background_event_at_checked(adjusted_tile)?
            .filter(|event| background_event_accepts_facing(&event.event_type, self.player.facing))
        {
            return Ok(Some(OverworldInteraction {
                map_name: self.map.name.clone(),
                player_tile: self.player.tile,
                facing: self.player.facing,
                target_tile: adjusted_tile,
                script: event.script.clone(),
                target: OverworldInteractionTarget::Background {
                    event_type: event.event_type.clone(),
                },
            }));
        }

        Ok(
            sample_collision(&self.map, &self.tileset, adjusted_tile).and_then(|sample| {
                standard_interaction_script(sample.permission).map(|script| OverworldInteraction {
                    map_name: self.map.name.clone(),
                    player_tile: self.player.tile,
                    facing: self.player.facing,
                    target_tile: adjusted_tile,
                    script: script.to_string(),
                    target: OverworldInteractionTarget::Collision {
                        permission: sample.permission,
                    },
                })
            }),
        )
    }

    pub fn check_coord_event_checked(
        &self,
        current_scene: Option<&str>,
    ) -> Result<Option<CoordEventTrigger>, OverworldEventCoordinateError> {
        self.map_events
            .coord_events
            .iter()
            .try_fold(None, |matched, event| {
                if matched.is_some() {
                    return Ok(matched);
                }
                let scene_matches = if event.scene_id.is_empty() {
                    true
                } else {
                    current_scene
                        .map(|scene| scene == event.scene_id)
                        .unwrap_or(false)
                };
                if !scene_matches {
                    return Ok(None);
                }
                let Some(event_tile) = coord_event_tile_position_checked(event) else {
                    return Err(OverworldEventCoordinateError::CoordOutOfRange {
                        script: event.script_name.clone(),
                        x: event.x,
                        y: event.y,
                    });
                };
                if event_tile != self.player.tile {
                    return Ok(None);
                }
                Ok(Some(CoordEventTrigger {
                    map_name: self.map.name.clone(),
                    tile: self.player.tile,
                    scene_id: event.scene_id.clone(),
                    script_name: event.script_name.clone(),
                }))
            })
    }

    pub fn check_trainer_sight_checked(
        &self,
    ) -> Result<Option<OverworldInteraction>, OverworldCoordinateError> {
        self.check_trainer_sight_checked_with_filter(|_| true)
    }

    pub fn check_trainer_sight_checked_with_filter<F>(
        &self,
        mut eligible: F,
    ) -> Result<Option<OverworldInteraction>, OverworldCoordinateError>
    where
        F: FnMut(&ObjectEvent) -> bool,
    {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, object)| self.is_object_visible(object))
            .filter(|(_, object)| eligible(object))
            .filter(|(_, object)| {
                object.object_type == "OBJECTTYPE_TRAINER"
                    && object.radius > 0
                    && object_has_dispatchable_script(object)
                    && !self.object_is_visibly_walking(object)
            })
            .try_fold(None, |matched, (index, object)| {
                if matched.is_some() {
                    return Ok(matched);
                }
                let object_tile = self.object_runtime_tile_checked(index, object)?;
                let direction = object
                    .sightline_direction_override
                    .as_deref()
                    .and_then(direction_from_object_token)
                    .or_else(|| {
                        object
                            .object_identifier
                            .as_ref()
                            .and_then(|object_id| self.object_facings.get(object_id).copied())
                    })
                    .or_else(|| object_event_initial_facing(&object.spritemovedata));
                let Some(direction) = direction else {
                    return Ok(None);
                };
                if !tile_is_in_sightline(object_tile, direction, self.player.tile, object.radius) {
                    return Ok(None);
                }
                Ok(Some(self.object_interaction(
                    (index + 1) as u16,
                    object,
                    object_tile,
                )))
            })
    }

    pub fn visible_object_at(&self, tile: TilePosition) -> Option<(u16, &ObjectEvent)> {
        self.visible_object_at_checked(tile)
            .expect("visible object coordinates must be valid runtime coordinate state")
    }

    pub fn visible_object_at_checked(
        &self,
        tile: TilePosition,
    ) -> Result<Option<(u16, &ObjectEvent)>, OverworldObjectCoordinateError> {
        for (index, object) in self
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| self.is_object_visible(object))
        {
            let object_tile = self.object_runtime_tile_checked(index, object)?;
            if object_tile == tile {
                return Ok(Some(((index + 1) as u16, object)));
            }
        }
        Ok(None)
    }

    pub fn background_event_at_checked(
        &self,
        tile: TilePosition,
    ) -> Result<Option<&BackgroundEvent>, OverworldEventCoordinateError> {
        for event in &self.map_events.bg_events {
            let Some(event_tile) = background_event_tile_position_checked(event) else {
                return Err(OverworldEventCoordinateError::BackgroundOutOfRange {
                    script: event.script.clone(),
                    x: event.x,
                    y: event.y,
                });
            };
            if event_tile == tile {
                return Ok(Some(event));
            }
        }
        Ok(None)
    }

    pub fn counter_adjusted_tile(&self, tile: TilePosition) -> TilePosition {
        let stride = DEFAULT_RUNTIME_TILE_STRIDE;
        let Some(delta_x) = tile.x.checked_sub(self.player.tile.x) else {
            return tile;
        };
        let Some(delta_y) = tile.y.checked_sub(self.player.tile.y) else {
            return tile;
        };
        if delta_x == 0 && delta_y == 0 {
            return tile;
        }

        let mut candidates = Vec::new();
        if delta_x == 0 && delta_y != 0 {
            let Some(front_y) = self.player.tile.y.checked_add(delta_y) else {
                return tile;
            };
            for offset in 0..=stride {
                if let Some(x) = self.player.tile.x.checked_sub(offset) {
                    candidates.push(TilePosition::new(x, front_y));
                }
            }
        } else if delta_y == 0 && delta_x != 0 {
            let Some(front_x) = self.player.tile.x.checked_add(delta_x) else {
                return tile;
            };
            for offset in 0..=stride {
                if let Some(y) = self.player.tile.y.checked_sub(offset) {
                    candidates.push(TilePosition::new(front_x, y));
                }
            }
        } else {
            candidates.push(tile);
        }

        candidates
            .into_iter()
            .find(|candidate| {
                sample_collision(&self.map, &self.tileset, *candidate)
                    .map(|sample| is_counter_permission(sample.permission))
                    .unwrap_or(false)
            })
            .and_then(|counter| {
                Some(TilePosition::new(
                    counter.x.checked_add(delta_x)?,
                    counter.y.checked_add(delta_y)?,
                ))
            })
            .unwrap_or(tile)
    }

    fn object_interaction(
        &self,
        object_index: u16,
        object: &ObjectEvent,
        target_tile: TilePosition,
    ) -> OverworldInteraction {
        OverworldInteraction {
            map_name: self.map.name.clone(),
            player_tile: self.player.tile,
            facing: self.player.facing,
            target_tile,
            script: object.script.clone(),
            target: OverworldInteractionTarget::Object {
                object_index,
                object_identifier: object.object_identifier.clone(),
                object_type: object.object_type.clone(),
            },
        }
    }

    fn object_is_visibly_walking(&self, object: &ObjectEvent) -> bool {
        let Some(object_id) = object.object_identifier.as_deref() else {
            return false;
        };
        self.object_last_tiles_occupied_until_frame
            .get(object_id)
            .is_some_and(|until_frame| self.frame < *until_frame)
    }

    pub fn check_warp_checked(&self) -> Result<Option<WarpTrigger>, OverworldEventCoordinateError> {
        for warp in &self.map_events.warps {
            if warp_tile_position_checked(warp).is_none() {
                return Err(OverworldEventCoordinateError::WarpOutOfRange {
                    index: warp.index,
                    x: warp.x,
                    y: warp.y,
                });
            }
        }
        let Some(collision) = sample_collision(&self.map, &self.tileset, self.player.tile) else {
            return Ok(None);
        };
        if !is_warp_permission(collision.permission) {
            return Ok(None);
        }
        if directional_warp_facing(collision.permission)
            .is_some_and(|required| required != self.player.facing)
        {
            return Ok(None);
        }
        self.map_events
            .warps
            .iter()
            .try_fold(None, |matched, warp| {
                if matched.is_some() {
                    return Ok(matched);
                }
                let Some(warp_tile) = warp_tile_position_checked(warp) else {
                    return Err(OverworldEventCoordinateError::WarpOutOfRange {
                        index: warp.index,
                        x: warp.x,
                        y: warp.y,
                    });
                };
                if warp_tile != self.player.tile {
                    return Ok(None);
                }
                Ok(Some(WarpTrigger {
                    map_name: self.map.name.clone(),
                    tile: self.player.tile,
                    permission: collision.permission,
                    warp: warp.clone(),
                }))
            })
    }

    pub fn check_connection_checked(
        &self,
    ) -> Result<Option<ConnectionTrigger>, OverworldCoordinateError> {
        connection_for_tile_checked(&self.map, self.player.tile)
    }

    pub fn step_and_check_warp_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<OverworldStepResult, OverworldCoordinateError> {
        let mut staged = self.clone();
        let outcome = staged.step_checked(direction, options)?;
        let warp = if matches!(outcome, StepOutcome::Moved { .. }) {
            staged.check_warp_checked()?
        } else {
            None
        };
        *self = staged;
        Ok(OverworldStepResult { outcome, warp })
    }

    pub fn apply_input_frame(
        &mut self,
        input: &PlayerInputFrame,
        options: StepOptions,
        current_scene: Option<&str>,
    ) -> Result<OverworldInputResult, OverworldInputError> {
        if input.frame() != self.frame {
            return Err(OverworldInputError::FrameMismatch {
                session_frame: self.frame,
                input_frame: input.frame(),
            });
        }
        self.apply_joypad_mask(input.joypad_mask(), options, current_scene)
    }

    pub fn apply_joypad_mask(
        &mut self,
        joypad_mask: u8,
        options: StepOptions,
        current_scene: Option<&str>,
    ) -> Result<OverworldInputResult, OverworldInputError> {
        let mut staged = self.clone();
        let action = if joypad_mask & B_PAD_A != 0 {
            let interaction = staged.check_interaction_checked(options.stride_tiles)?;
            staged.frame += 1;
            interaction
                .map(OverworldInputAction::Interact)
                .unwrap_or(OverworldInputAction::NoInteraction)
        } else if let Some(direction) = direction_from_joypad_mask(joypad_mask)? {
            if staged.can_jump_ledge_from_input(direction, options)? {
                OverworldInputAction::LedgeJump(
                    staged.ledge_jump_and_check_warp_checked(direction, options)?,
                )
            } else {
                OverworldInputAction::Step(staged.step_and_check_warp_checked(direction, options)?)
            }
        } else {
            staged.frame += 1;
            OverworldInputAction::Idle
        };
        let coord_event = staged.check_coord_event_checked(current_scene)?;
        let trainer_sight = if action.moves_player() {
            staged.check_trainer_sight_checked()?
        } else {
            None
        };
        let connection = staged.check_connection_checked()?;
        let snapshot = staged.snapshot();
        *self = staged;
        Ok(OverworldInputResult {
            frame: snapshot.frame,
            joypad_mask,
            action,
            coord_event,
            trainer_sight,
            connection,
            snapshot,
        })
    }

    fn can_jump_ledge_from_input(
        &self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<bool, OverworldCoordinateError> {
        let stride = require_runtime_stride(options.stride_tiles)?;
        self.require_checked_tile_bounds()?;
        checked_move_by_stride(self.player.tile, direction, stride).ok_or_else(|| {
            OverworldCoordinateError::RuntimeTileOverflow {
                x: self.player.tile.x,
                y: self.player.tile.y,
                facing: direction,
            }
        })?;
        Ok(can_jump_ledge(
            &self.map,
            &self.tileset,
            self.player.tile,
            direction,
            stride,
        ))
    }

    pub fn can_jump_ledge_checked(
        &self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<bool, OverworldCoordinateError> {
        self.can_jump_ledge_from_input(direction, options)
    }

    pub fn ledge_jump_and_check_warp_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<OverworldLedgeJumpResult, OverworldCoordinateError> {
        let mut staged = self.clone();
        let outcome = staged.ledge_jump_checked(direction, options)?;
        let warp = if matches!(outcome, LedgeJumpOutcome::Jumped { .. }) {
            staged.check_warp_checked()?
        } else {
            None
        };
        *self = staged;
        Ok(OverworldLedgeJumpResult { outcome, warp })
    }

    /// Exact `TryWildEncounter` / `_TryWildEncounter_BugContest` chooser over
    /// the caller-owned persistent `hRandomAdd`/`hRandomSub` stream.
    ///
    /// This stops before `LoadEnemyMon`; the caller must continue with the
    /// same [`CrystalRandom`] so held-item and DV reads remain in-order.
    pub fn check_wild_encounter_exact<S>(
        &self,
        encounters: Option<&WildEncounterData>,
        slot_tables: &EncounterSlotTables,
        music_modifiers: &EncounterMusicModifiers,
        rng: &mut CrystalRandom<&mut S>,
        options: EncounterCheckOptions,
        context: ExactEncounterContext<'_>,
    ) -> Result<Option<WildEncounterRoll>, ExactEncounterError<S::Error>>
    where
        S: DividerSource + ?Sized,
    {
        let Some(surface) =
            encounter_surface_for_player_tile_checked(self, options.land_encounters_on_any_land)?
        else {
            return Ok(None);
        };
        let contest_encounter = context.bug_contest_encounters.is_some();
        let normal_encounters = if contest_encounter {
            None
        } else {
            Some(
                encounters.ok_or_else(|| EncounterError::EmptyEncounterSlots {
                    map_name: self.map.name.clone(),
                    surface,
                    time: options.time,
                })?,
            )
        };
        let uncleaned_threshold = if contest_encounter {
            let permission = sample_collision(&self.map, &self.tileset, self.player.tile)
                .ok_or_else(|| EncounterError::MissingRuntimeCollision {
                    map_name: self.map.name.clone(),
                    x: self.player.tile.x,
                    y: self.player.tile.y,
                })?
                .permission;
            let base = percent_to_byte(if matches!(permission, 0x14 | 0x1c) {
                40.0
            } else {
                20.0
            });
            apply_encounter_music_effect(base, options.music_token.as_deref(), music_modifiers)?
        } else {
            let base = crate::world::encounters::base_encounter_rate(
                normal_encounters.expect("non-Contest chooser preflight requires a normal table"),
                surface,
                options.time,
            )?;
            let base = percent_to_byte(f64::from(base));
            apply_encounter_music_effect(base, options.music_token.as_deref(), music_modifiers)?
        };
        let threshold = apply_cleanse_tag_effect(uncleaned_threshold, options.has_cleanse_tag);
        // With no Cleanse Tag, the final failed party scan executes
        // `add hl,de`, whose canonical WRAM range cannot overflow. With a
        // Cleanse Tag, `srl b` supplies bit 0 of the pre-halved rate.
        let rate_carry = options.has_cleanse_tag && uncleaned_threshold & 1 != 0;
        let rate_output = rng
            .random(rate_carry)
            .map_err(ExactEncounterError::Divider)?;
        let encounter_roll = if contest_encounter {
            rng.state().add
        } else {
            rate_output.value
        };
        if !passes_encounter_roll(threshold, encounter_roll) {
            return Ok(Some(WildEncounterRoll {
                map_name: self.map.name.clone(),
                tile: self.player.tile,
                surface,
                time: options.time,
                threshold,
                encounter_roll,
                slot_percent_roll: None,
                level_roll: None,
                roaming_slot: None,
                resolved: None,
                repelled_by: None,
            }));
        }

        if contest_encounter {
            // `TryWildEncounter_BugContest` returns with carry set on success.
            let mut first = true;
            let mut weighted_roll = loop {
                let output = rng.random(first).map_err(ExactEncounterError::Divider)?;
                first = false;
                if output.value < 200 {
                    break i16::from(output.value >> 1);
                }
            };
            for entry in context.bug_contest_encounters.unwrap_or_default() {
                weighted_roll -= i16::from(entry.weight);
                if weighted_roll < 0 {
                    let range = entry.max_level.saturating_sub(entry.min_level) + 1;
                    let level = if range == 1 {
                        entry.min_level
                    } else {
                        rng.random(false).map_err(ExactEncounterError::Divider)?;
                        entry.min_level + rng.state().add % range
                    };
                    let resolved = ResolvedWildEncounter {
                        level,
                        encounter: WildEncounter {
                            level,
                            species: entry.species.clone(),
                        },
                        slot: 0,
                    };
                    let (resolved, repelled_by) =
                        apply_repel_to_wild_encounter(Some(resolved), &options)?;
                    return Ok(Some(WildEncounterRoll {
                        map_name: self.map.name.clone(),
                        tile: self.player.tile,
                        surface,
                        time: options.time,
                        threshold,
                        encounter_roll,
                        slot_percent_roll: None,
                        level_roll: Some(level),
                        roaming_slot: None,
                        resolved,
                        repelled_by,
                    }));
                }
            }
            return Err(EncounterError::EmptyEncounterSlots {
                map_name: self.map.name.clone(),
                surface,
                time: options.time,
            }
            .into());
        }

        // CheckEncounterRoamMon is land-only and enters Random with carry
        // clear after CheckOnWater's final `and a`.
        if surface != EncounterSurface::Water {
            let roaming_roll = rng
                .random(false)
                .map_err(ExactEncounterError::Divider)?
                .value;
            if roaming_roll < 100 {
                let raw_slot = usize::from(roaming_roll & 0x03);
                if raw_slot != 0 {
                    let roaming = &context.roaming_pokemon[raw_slot - 1];
                    if roaming.map_group == context.current_map.0
                        && roaming.map_number == context.current_map.1
                        && let Some(species) = roaming.species.clone()
                    {
                        let level = roaming.level;
                        let resolved = ResolvedWildEncounter {
                            level,
                            encounter: WildEncounter { species, level },
                            slot: raw_slot - 1,
                        };
                        let (resolved, repelled_by) =
                            apply_repel_to_wild_encounter(Some(resolved), &options)?;
                        return Ok(Some(WildEncounterRoll {
                            map_name: self.map.name.clone(),
                            tile: self.player.tile,
                            surface,
                            time: options.time,
                            threshold,
                            encounter_roll,
                            slot_percent_roll: None,
                            level_roll: Some(level),
                            roaming_slot: Some((raw_slot - 1) as u8),
                            resolved,
                            repelled_by,
                        }));
                    }
                }
            }
        }

        let slot_percent_roll = loop {
            let value = rng
                .random(false)
                .map_err(ExactEncounterError::Divider)?
                .value;
            if value < 100 {
                break value + 1;
            }
        };
        let level_roll = if surface == EncounterSurface::Water {
            Some(
                rng.random(false)
                    .map_err(ExactEncounterError::Divider)?
                    .value,
            )
        } else {
            None
        };
        let resolved = select_wild_encounter(
            normal_encounters.expect("non-Contest chooser preflight requires a normal table"),
            slot_tables,
            surface,
            options.time,
            slot_percent_roll,
            level_roll.unwrap_or(0),
        )?;
        // When no Unown letters are unlocked, ChooseWildEncounter returns
        // normally after its chooser draws but before Repel or LoadEnemyMon.
        let (resolved, repelled_by) = if resolved
            .as_ref()
            .is_some_and(|resolved| resolved.encounter.species == "UNOWN")
            && context.unlocked_unown_sets == 0
        {
            (None, None)
        } else {
            apply_repel_to_wild_encounter(resolved, &options)?
        };
        Ok(Some(WildEncounterRoll {
            map_name: self.map.name.clone(),
            tile: self.player.tile,
            surface,
            time: options.time,
            threshold,
            encounter_roll,
            slot_percent_roll: Some(slot_percent_roll),
            level_roll,
            roaming_slot: None,
            resolved,
            repelled_by,
        }))
    }

    /// Exact `SweetScentEncounter` chooser. Unlike a walking encounter this
    /// performs no encounter-rate Random call and never applies Repel. The
    /// live collision decides land/water; callers cannot inject a surface.
    pub fn check_sweet_scent_encounter_exact<S>(
        &self,
        encounters: Option<&WildEncounterData>,
        slot_tables: &EncounterSlotTables,
        rng: &mut CrystalRandom<&mut S>,
        mut options: EncounterCheckOptions,
        context: ExactEncounterContext<'_>,
    ) -> Result<Option<WildEncounterRoll>, ExactEncounterError<S::Error>>
    where
        S: DividerSource + ?Sized,
    {
        let Some(surface) =
            encounter_surface_for_player_tile_checked(self, options.land_encounters_on_any_land)?
        else {
            return Ok(None);
        };
        let contest_encounter = context.bug_contest_encounters.is_some();
        let normal_encounters = if contest_encounter {
            None
        } else {
            Some(
                encounters.ok_or_else(|| EncounterError::EmptyEncounterSlots {
                    map_name: self.map.name.clone(),
                    surface,
                    time: options.time,
                })?,
            )
        };
        let threshold = if contest_encounter {
            u8::MAX
        } else {
            let rate = crate::world::encounters::base_encounter_rate(
                normal_encounters.expect("non-Contest chooser preflight requires a normal table"),
                surface,
                options.time,
            )?;
            if rate == 0 {
                return Ok(None);
            }
            rate
        };
        // Sweet Scent never calls CheckRepelEffect.
        options.active_repel_item = None;

        if contest_encounter {
            // CanEncounterWildMon returned SCF and the intervening BIT keeps C.
            let mut first = true;
            let mut weighted_roll = loop {
                let output = rng.random(first).map_err(ExactEncounterError::Divider)?;
                first = false;
                if output.value < 200 {
                    break i16::from(output.value >> 1);
                }
            };
            for entry in context.bug_contest_encounters.unwrap_or_default() {
                weighted_roll -= i16::from(entry.weight);
                if weighted_roll < 0 {
                    let range = entry.max_level.saturating_sub(entry.min_level) + 1;
                    let level = if range == 1 {
                        entry.min_level
                    } else {
                        rng.random(false).map_err(ExactEncounterError::Divider)?;
                        entry.min_level + rng.state().add % range
                    };
                    let resolved = Some(ResolvedWildEncounter {
                        level,
                        encounter: WildEncounter {
                            level,
                            species: entry.species.clone(),
                        },
                        slot: 0,
                    });
                    return Ok(Some(WildEncounterRoll {
                        map_name: self.map.name.clone(),
                        tile: self.player.tile,
                        surface,
                        time: options.time,
                        threshold,
                        encounter_roll: 0,
                        slot_percent_roll: None,
                        level_roll: Some(level),
                        roaming_slot: None,
                        resolved,
                        repelled_by: None,
                    }));
                }
            }
            return Err(EncounterError::EmptyEncounterSlots {
                map_name: self.map.name.clone(),
                surface,
                time: options.time,
            }
            .into());
        }

        if surface != EncounterSurface::Water {
            let roaming_roll = rng
                .random(false)
                .map_err(ExactEncounterError::Divider)?
                .value;
            if roaming_roll < 100 {
                let raw_slot = usize::from(roaming_roll & 0x03);
                if raw_slot != 0 {
                    let roaming = &context.roaming_pokemon[raw_slot - 1];
                    if roaming.map_group == context.current_map.0
                        && roaming.map_number == context.current_map.1
                        && let Some(species) = roaming.species.clone()
                    {
                        let level = roaming.level;
                        return Ok(Some(WildEncounterRoll {
                            map_name: self.map.name.clone(),
                            tile: self.player.tile,
                            surface,
                            time: options.time,
                            threshold,
                            encounter_roll: 0,
                            slot_percent_roll: None,
                            level_roll: Some(level),
                            roaming_slot: Some((raw_slot - 1) as u8),
                            resolved: Some(ResolvedWildEncounter {
                                level,
                                encounter: WildEncounter { species, level },
                                slot: raw_slot - 1,
                            }),
                            repelled_by: None,
                        }));
                    }
                }
            }
        }

        let slot_percent_roll = loop {
            let value = rng
                .random(false)
                .map_err(ExactEncounterError::Divider)?
                .value;
            if value < 100 {
                break value + 1;
            }
        };
        let level_roll = if surface == EncounterSurface::Water {
            Some(
                rng.random(false)
                    .map_err(ExactEncounterError::Divider)?
                    .value,
            )
        } else {
            None
        };
        let resolved = select_wild_encounter(
            normal_encounters.expect("non-Contest chooser preflight requires a normal table"),
            slot_tables,
            surface,
            options.time,
            slot_percent_roll,
            level_roll.unwrap_or(0),
        )?;
        let resolved = if resolved
            .as_ref()
            .is_some_and(|resolved| resolved.encounter.species == "UNOWN")
            && context.unlocked_unown_sets == 0
        {
            None
        } else {
            resolved
        };
        Ok(Some(WildEncounterRoll {
            map_name: self.map.name.clone(),
            tile: self.player.tile,
            surface,
            time: options.time,
            threshold,
            encounter_roll: 0,
            slot_percent_roll: Some(slot_percent_roll),
            level_roll,
            roaming_slot: None,
            resolved,
            repelled_by: None,
        }))
    }

    fn require_encounter_runtime_tile(&self) -> Result<(), EncounterError> {
        let (width, height) = self.map.checked_tile_bounds().ok_or_else(|| {
            EncounterError::RuntimeTileBoundsOverflow {
                map_name: self.map.name.clone(),
            }
        })?;
        if self.player.tile.x < 0
            || self.player.tile.y < 0
            || i32::from(self.player.tile.x) >= i32::from(width)
            || i32::from(self.player.tile.y) >= i32::from(height)
        {
            return Err(EncounterError::RuntimeTileOutOfBounds {
                map_name: self.map.name.clone(),
                x: self.player.tile.x,
                y: self.player.tile.y,
                width,
                height,
            });
        }
        Ok(())
    }
}

fn background_event_accepts_facing(event_type: &str, facing: Direction) -> bool {
    match event_type {
        "BGEVENT_UP" => facing == Direction::Up,
        "BGEVENT_DOWN" => facing == Direction::Down,
        "BGEVENT_RIGHT" => facing == Direction::Right,
        "BGEVENT_LEFT" => facing == Direction::Left,
        // BGEVENT_COPY only copies hidden-item metadata for Itemfinder and
        // never starts a script or acknowledges an A-button interaction.
        "BGEVENT_COPY" => false,
        _ => true,
    }
}

fn object_has_dispatchable_script(object: &ObjectEvent) -> bool {
    object.script != "-1" && object.script != "ObjectEvent"
}

fn apply_repel_to_wild_encounter(
    resolved: Option<ResolvedWildEncounter>,
    options: &EncounterCheckOptions,
) -> Result<(Option<ResolvedWildEncounter>, Option<String>), EncounterError> {
    let Some(item_id) = &options.active_repel_item else {
        return Ok((resolved, None));
    };
    let lead_level =
        options
            .lead_party_level
            .ok_or_else(|| EncounterError::ActiveRepelMissingLeadLevel {
                item_id: item_id.clone(),
            })?;
    let Some(encounter) = resolved else {
        return Ok((None, None));
    };
    if encounter.level < lead_level {
        return Ok((None, Some(item_id.clone())));
    }
    Ok((Some(encounter), None))
}

fn direction_between_tiles(from: TilePosition, to: TilePosition) -> Option<Direction> {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    if dx.abs() >= dy.abs() && dx != 0 {
        Some(if dx > 0 {
            Direction::Right
        } else {
            Direction::Left
        })
    } else if dy != 0 {
        Some(if dy > 0 {
            Direction::Down
        } else {
            Direction::Up
        })
    } else {
        None
    }
}

pub fn object_event_initial_facing(spritemovedata: &str) -> Option<Direction> {
    match spritemovedata {
        "SPRITEMOVEDATA_00"
        | "SPRITEMOVEDATA_STILL"
        | "SPRITEMOVEDATA_WANDER"
        | "SPRITEMOVEDATA_SPINRANDOM_SLOW"
        | "SPRITEMOVEDATA_WALK_UP_DOWN"
        | "SPRITEMOVEDATA_WALK_LEFT_RIGHT"
        | "SPRITEMOVEDATA_STANDING_DOWN"
        | "SPRITEMOVEDATA_SPINRANDOM_FAST"
        | "SPRITEMOVEDATA_PLAYER"
        | "SPRITEMOVEDATA_INDEXED_1"
        | "SPRITEMOVEDATA_INDEXED_2"
        | "SPRITEMOVEDATA_0E"
        | "SPRITEMOVEDATA_0F"
        | "SPRITEMOVEDATA_10"
        | "SPRITEMOVEDATA_11"
        | "SPRITEMOVEDATA_12"
        | "SPRITEMOVEDATA_FOLLOWING"
        | "SPRITEMOVEDATA_SCRIPTED"
        | "SPRITEMOVEDATA_BIGDOLLSYM"
        | "SPRITEMOVEDATA_POKEMON"
        | "SPRITEMOVEDATA_SUDOWOODO"
        | "SPRITEMOVEDATA_SMASHABLE_ROCK"
        | "SPRITEMOVEDATA_STRENGTH_BOULDER"
        | "SPRITEMOVEDATA_FOLLOWNOTEXACT"
        | "SPRITEMOVEDATA_SHADOW"
        | "SPRITEMOVEDATA_EMOTE"
        | "SPRITEMOVEDATA_SCREENSHAKE"
        | "SPRITEMOVEDATA_BIGDOLLASYM"
        | "SPRITEMOVEDATA_BIGDOLL"
        | "SPRITEMOVEDATA_BOULDERDUST"
        | "SPRITEMOVEDATA_GRASS"
        | "SPRITEMOVEDATA_SWIM_WANDER" => Some(Direction::Down),
        "SPRITEMOVEDATA_STANDING_UP" => Some(Direction::Up),
        "SPRITEMOVEDATA_STANDING_LEFT" => Some(Direction::Left),
        "SPRITEMOVEDATA_STANDING_RIGHT" => Some(Direction::Right),
        "SPRITEMOVEDATA_SPINCOUNTERCLOCKWISE" => Some(Direction::Left),
        "SPRITEMOVEDATA_SPINCLOCKWISE" => Some(Direction::Right),
        _ => None,
    }
}

fn initial_object_facings(objects: &[ObjectEvent]) -> BTreeMap<String, Direction> {
    objects
        .iter()
        .filter_map(|object| {
            let object_id = object.object_identifier.as_ref()?;
            let facing = object_event_initial_facing(&object.spritemovedata)?;
            Some((object_id.clone(), facing))
        })
        .collect()
}

fn encounter_surface_for_player_tile_checked(
    session: &OverworldSession,
    land_encounters_on_any_land: bool,
) -> Result<Option<EncounterSurface>, EncounterError> {
    session.require_encounter_runtime_tile()?;
    let sample =
        sample_collision(&session.map, &session.tileset, session.player.tile).ok_or_else(|| {
            EncounterError::MissingRuntimeCollision {
                map_name: session.map.name.clone(),
                x: session.player.tile.x,
                y: session.player.tile.y,
            }
        })?;
    if super::collision::is_grass_encounter_permission(sample.permission) {
        return Ok(Some(EncounterSurface::Grass));
    }
    let attributes = describe_collision(sample.permission);
    if attributes.terrain == Terrain::Water {
        return Ok(Some(EncounterSurface::Water));
    }
    if land_encounters_on_any_land
        && attributes.terrain == Terrain::Land
        && !matches!(sample.permission, permissions::ICE | permissions::ICE_2B)
    {
        return Ok(Some(EncounterSurface::Grass));
    }
    Ok(None)
}

fn direction_from_joypad_mask(mask: u8) -> Result<Option<Direction>, OverworldInputError> {
    let mut direction = None;
    for (bit, candidate) in [
        (B_PAD_RIGHT, Direction::Right),
        (B_PAD_LEFT, Direction::Left),
        (B_PAD_UP, Direction::Up),
        (B_PAD_DOWN, Direction::Down),
    ] {
        if mask & bit == 0 {
            continue;
        }
        if direction.is_some() {
            return Err(OverworldInputError::ConflictingDirections { mask });
        }
        direction = Some(candidate);
    }
    Ok(direction)
}

fn direction_from_object_token(value: &str) -> Option<Direction> {
    match value {
        "UP" | "SPRITEMOVEDATA_STANDING_UP" => Some(Direction::Up),
        "DOWN" | "SPRITEMOVEDATA_STANDING_DOWN" => Some(Direction::Down),
        "LEFT" | "SPRITEMOVEDATA_STANDING_LEFT" => Some(Direction::Left),
        "RIGHT" | "SPRITEMOVEDATA_STANDING_RIGHT" => Some(Direction::Right),
        _ => None,
    }
}

fn tile_is_in_sightline(
    object_tile: TilePosition,
    direction: Direction,
    player_tile: TilePosition,
    radius: u16,
) -> bool {
    let stride = i32::from(StepOptions::default().stride_tiles);
    let dx = i32::from(player_tile.x) - i32::from(object_tile.x);
    let dy = i32::from(player_tile.y) - i32::from(object_tile.y);
    let distance = match direction {
        Direction::Up if dx == 0 && dy < 0 => -dy,
        Direction::Down if dx == 0 && dy > 0 => dy,
        Direction::Left if dy == 0 && dx < 0 => -dx,
        Direction::Right if dy == 0 && dx > 0 => dx,
        _ => return false,
    };
    if distance > i32::from(i16::MAX) {
        return false;
    }
    let Some(max_distance) = i32::from(radius).checked_mul(stride) else {
        return false;
    };
    distance > 0 && distance % stride == 0 && distance <= max_distance
}

pub fn warp_tile_position_checked(warp: &WarpEvent) -> Option<TilePosition> {
    raw_event_tile_to_runtime_tile_checked(warp.x, warp.y)
}

pub fn connection_for_tile_checked(
    map: &OverworldMapData,
    tile: TilePosition,
) -> Result<Option<ConnectionTrigger>, OverworldCoordinateError> {
    let (width, height) =
        map.checked_tile_bounds()
            .ok_or_else(|| OverworldCoordinateError::MapBoundsOverflow {
                map_name: map.name.clone(),
            })?;
    let width = i32::from(width);
    let height = i32::from(height);
    let tile_x = i32::from(tile.x);
    let tile_y = i32::from(tile.y);
    let mut directions = BTreeSet::new();
    let mut matched = None;
    for connection in map.connections() {
        let direction = connection.direction.as_str();
        let crosses_boundary = match direction {
            "north" => tile_y < 0,
            "south" => tile_y >= height,
            "west" => tile_x < 0,
            "east" => tile_x >= width,
            other => {
                return Err(OverworldCoordinateError::UnsupportedConnectionDirection {
                    map_name: map.name.clone(),
                    direction: other.to_string(),
                });
            }
        };
        if !directions.insert(direction) {
            return Err(OverworldCoordinateError::DuplicateConnectionDirection {
                map_name: map.name.clone(),
                direction: direction.to_string(),
            });
        }
        if crosses_boundary {
            if matched.is_some() {
                return Err(OverworldCoordinateError::AmbiguousConnectionBoundary {
                    map_name: map.name.clone(),
                    x: tile.x,
                    y: tile.y,
                });
            }
            matched = Some(connection.clone());
        }
    }

    Ok(matched.map(|connection| ConnectionTrigger {
        map_name: map.name.clone(),
        tile,
        connection,
    }))
}

impl WarpTransition {
    pub fn apply_to(
        &self,
        map: OverworldMapData,
        map_events: MapEvents,
        objects: Vec<ObjectEvent>,
        tileset: TilesetCollision,
        frame: u64,
        mode: MovementMode,
    ) -> OverworldSession {
        OverworldSession {
            frame,
            map,
            map_events,
            objects,
            object_runtime_tiles: BTreeMap::new(),
            object_last_runtime_tiles: BTreeMap::new(),
            object_last_tiles_occupied_until_frame: BTreeMap::new(),
            object_facings: BTreeMap::new(),
            object_step_durations: BTreeMap::new(),
            object_pending_random_wait: BTreeSet::new(),
            initialized_fixed_spin_objects: BTreeSet::new(),
            following: None,
            following_queued_step: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            shown_object_identifiers: BTreeSet::new(),
            object_visibility_initialized: false,
            time_of_day: default_session_time_of_day(),
            tileset,
            player: PlayerMovementState::new(self.destination.tile).with_mode(mode),
            last_step_direction: None,
            player_last_runtime_tile: None,
            player_last_tile_occupied_until_frame: 0,
        }
    }
}

impl ConnectionTransition {
    pub fn apply_to(
        &self,
        map: OverworldMapData,
        map_events: MapEvents,
        objects: Vec<ObjectEvent>,
        tileset: TilesetCollision,
        frame: u64,
        mode: MovementMode,
    ) -> OverworldSession {
        OverworldSession {
            frame,
            map,
            map_events,
            objects,
            object_runtime_tiles: BTreeMap::new(),
            object_last_runtime_tiles: BTreeMap::new(),
            object_last_tiles_occupied_until_frame: BTreeMap::new(),
            object_facings: BTreeMap::new(),
            object_step_durations: BTreeMap::new(),
            object_pending_random_wait: BTreeSet::new(),
            initialized_fixed_spin_objects: BTreeSet::new(),
            following: None,
            following_queued_step: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            shown_object_identifiers: BTreeSet::new(),
            object_visibility_initialized: false,
            time_of_day: default_session_time_of_day(),
            tileset,
            player: PlayerMovementState::new(self.destination.tile).with_mode(mode),
            last_step_direction: None,
            player_last_runtime_tile: None,
            player_last_tile_occupied_until_frame: 0,
        }
    }
}

pub fn object_tile_position_checked(object: &ObjectEvent) -> Option<TilePosition> {
    raw_event_tile_to_runtime_tile_checked(object.x, object.y)
}

fn object_coordinate_out_of_range(
    index: usize,
    object: &ObjectEvent,
) -> OverworldObjectCoordinateError {
    OverworldObjectCoordinateError::OutOfRange {
        object_id: object
            .object_identifier
            .clone()
            .unwrap_or_else(|| format!("object#{}", index + 1)),
        x: object.x,
        y: object.y,
    }
}

/// Crystal stores an NPC's schedule as a bit mask in `hram_y`: morning is
/// bit 0, daytime bit 1, and night bit 2. Zero and -1 mean "any time".
fn object_visible_at_time(hram_y: i16, time_of_day: TimeOfDay) -> bool {
    if hram_y == 0 || hram_y == -1 {
        return true;
    }
    let time_mask = match time_of_day {
        TimeOfDay::Morning => 0b001,
        TimeOfDay::Day => 0b010,
        TimeOfDay::Night => 0b100,
    };
    (hram_y as u16 & time_mask) != 0
}

pub fn background_event_tile_position_checked(event: &BackgroundEvent) -> Option<TilePosition> {
    raw_event_tile_to_runtime_tile_checked(event.x, event.y)
}

pub fn coord_event_tile_position_checked(event: &CoordEvent) -> Option<TilePosition> {
    raw_event_tile_to_runtime_tile_checked(event.x, event.y)
}

pub fn raw_event_tile_to_runtime_tile_checked(x: u16, y: u16) -> Option<TilePosition> {
    Some(TilePosition::new(
        i16::try_from(x).ok()?,
        i16::try_from(y).ok()?,
    ))
}

pub fn runtime_tile_to_raw_event_tile(tile: TilePosition) -> Option<TilePosition> {
    if tile.x < 0 || tile.y < 0 {
        return None;
    }
    Some(tile)
}

fn require_runtime_stride(stride_tiles: i16) -> Result<i16, OverworldCoordinateError> {
    if stride_tiles == DEFAULT_RUNTIME_TILE_STRIDE {
        Ok(stride_tiles)
    } else {
        Err(OverworldCoordinateError::InvalidRuntimeStride {
            stride_tiles,
            metatile_width: METATILE_WIDTH,
        })
    }
}

pub fn is_counter_permission(permission: u8) -> bool {
    permission == permissions::COUNTER || permission == permissions::COUNTER_98
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{BackgroundEvent, CoordEvent, MapAttributes, ObjectEvent};
    use crate::multiplayer::PlayerInputFrame;
    use crate::timing::Frame;
    use crate::world::collision::{MetatileCollision, permissions};
    use crate::world::encounters::{WildEncounter, WildEncounterTable};

    #[test]
    fn object_schedule_mask_matches_crystal_time_of_day_bits() {
        assert!(object_visible_at_time(-1, TimeOfDay::Morning));
        assert!(object_visible_at_time(0, TimeOfDay::Night));
        assert!(object_visible_at_time(0b001, TimeOfDay::Morning));
        assert!(!object_visible_at_time(0b001, TimeOfDay::Day));
        assert!(object_visible_at_time(0b110, TimeOfDay::Night));
        assert!(!object_visible_at_time(0b110, TimeOfDay::Morning));
    }

    #[test]
    fn walking_object_facings_start_from_the_asm_table_before_autonomous_movement() {
        assert_eq!(
            object_event_initial_facing("SPRITEMOVEDATA_WALK_LEFT_RIGHT"),
            Some(Direction::Down)
        );
        assert_eq!(
            object_event_initial_facing("SPRITEMOVEDATA_WALK_UP_DOWN"),
            Some(Direction::Down)
        );
    }

    #[test]
    fn autonomous_horizontal_walker_advances_on_crystal_frame_cadence() {
        let mut walker = object("WALKER", 1, 1, "-1");
        walker.spritemovedata = "SPRITEMOVEDATA_WALK_LEFT_RIGHT".to_string();
        walker.move_range_x = 1;
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(4, 4, vec![0; 16]),
            MapEvents::default(),
            vec![walker],
            tileset(),
            TilePosition::new(6, 6),
        );
        session.frame = 16;
        let mut divider = crate::random::ReplayDivider::new([0, 0]);
        let mut rng = CrystalRandom::new(CrystalRandomState::default(), &mut divider);
        session
            .advance_autonomous_objects_exact(&mut rng)
            .expect("walker advances");
        assert_eq!(
            session.object_runtime_tile_by_id("WALKER").unwrap(),
            TilePosition::new(0, 1)
        );
        assert_eq!(divider.remaining(), 0);
    }

    #[test]
    fn autonomous_wander_uses_injected_crystal_rng_and_collision() {
        let mut wanderer = object("WANDERER", 1, 1, "-1");
        wanderer.spritemovedata = "SPRITEMOVEDATA_WANDER".to_string();
        wanderer.move_range_x = 1;
        wanderer.move_range_y = 1;
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(4, 4, vec![0; 16]),
            MapEvents::default(),
            vec![wanderer],
            tileset(),
            TilePosition::new(6, 6),
        );
        session.frame = 16;
        let mut divider = crate::random::ReplayDivider::new([0, 0xff]);
        let mut rng = CrystalRandom::new(CrystalRandomState::default(), &mut divider);
        session
            .advance_autonomous_objects_exact(&mut rng)
            .expect("wanderer advances");
        assert_ne!(
            session.object_runtime_tile_by_id("WANDERER").unwrap(),
            TilePosition::new(2, 2)
        );
        assert_eq!(divider.remaining(), 0);
    }

    #[test]
    fn exact_fast_random_spin_duration_inherits_direction_compare_carry() {
        let mut spinner = object("SPINNER", 1, 1, "-1");
        spinner.spritemovedata = "SPRITEMOVEDATA_SPINRANDOM_FAST".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(4, 4, vec![0; 16]),
            MapEvents::default(),
            vec![spinner],
            tileset(),
            TilePosition::new(6, 6),
        );
        session
            .object_facings
            .insert("SPINNER".to_string(), Direction::Right);
        let mut divider = crate::random::ReplayDivider::new([0, 0, 247, 0]);
        let mut rng = CrystalRandom::new(CrystalRandomState { add: 8, sub: 0 }, &mut divider);

        session
            .advance_autonomous_objects_exact(&mut rng)
            .expect("exact autonomous random spin");

        assert_eq!(
            session.object_facings.get("SPINNER"),
            Some(&Direction::Left)
        );
        assert_eq!(session.object_step_durations.get("SPINNER"), Some(&0));
        assert_eq!(rng.state(), CrystalRandomState { add: 0, sub: 0xff });
        assert_eq!(divider.remaining(), 0);
    }

    #[test]
    fn current_permission_exposes_forced_down_direction() {
        let current_tileset = TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::CURRENT_DOWN; 4],
            }],
        };
        let session = OverworldSession::new(
            map_with_blocks(2, 2, vec![0; 4]),
            current_tileset,
            TilePosition::new(0, 0),
        );
        assert_eq!(session.forced_current_direction(), Some(Direction::Down));

        for (permission, direction) in [
            (permissions::CURRENT_RIGHT, Direction::Right),
            (permissions::CURRENT_LEFT, Direction::Left),
            (permissions::CURRENT_UP, Direction::Up),
        ] {
            let session = OverworldSession::new(
                map_with_blocks(2, 2, vec![0; 4]),
                TilesetCollision {
                    metatiles: vec![MetatileCollision {
                        collision: [permission; 4],
                    }],
                },
                TilePosition::new(0, 0),
            );
            assert_eq!(session.forced_current_direction(), Some(direction));
        }

        let mut ice_session = OverworldSession::new(
            map_with_blocks(2, 2, vec![0; 4]),
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::ICE; 4],
                }],
            },
            TilePosition::new(0, 0),
        );
        ice_session.player.facing = Direction::Left;
        assert_eq!(ice_session.forced_movement_direction(), None);
        ice_session.last_step_direction = Some(Direction::Left);
        assert_eq!(
            ice_session.forced_movement_direction(),
            Some(Direction::Left)
        );
    }

    fn map() -> OverworldMapData {
        OverworldMapData::from_attributes(
            "test",
            &MapAttributes {
                tileset_name: "test".to_string(),
                border_block: 0,
                width: 2,
                height: 1,
                connections: Vec::new(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: None,
                location: None,
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: None,
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
            vec![0, 0],
        )
    }

    fn attributes(width: u16, height: u16) -> MapAttributes {
        MapAttributes {
            tileset_name: "test".to_string(),
            border_block: 0,
            width,
            height,
            connections: Vec::new(),
            time_of_day: None,
            phone_service: 0,
            phone_flag: false,
            environment: None,
            location: None,
            music: None,
            palette: None,
            fishing_group: None,
            map_constant: None,
            map_group_constant: None,
            blocks_label: None,
            map_scripts_label: None,
            map_events_label: None,
            connection_flags: None,
        }
    }

    #[test]
    fn overworld_session_discriminants_reject_legacy_alias_payloads() {
        let action_error =
            serde_json::from_str::<OverworldInputAction>(r#"{"legacy_idle":{"reason":"wait"}}"#)
                .expect_err("overworld actions must not accept legacy idle aliases")
                .to_string();
        assert!(
            action_error.contains("unknown variant `legacy_idle`"),
            "{action_error}"
        );

        let target_error = serde_json::from_str::<OverworldInteractionTarget>(
            r#"{"fallback_object":{"object_index":0,"object_identifier":null,"object_type":"npc"}}"#,
        )
        .expect_err("interaction targets must not accept fallback target aliases")
        .to_string();
        assert!(
            target_error.contains("unknown variant `fallback_object`"),
            "{target_error}"
        );
    }

    fn map_with_connections() -> OverworldMapData {
        OverworldMapData::from_attributes(
            "test",
            &MapAttributes {
                tileset_name: "test".to_string(),
                border_block: 0,
                width: 2,
                height: 1,
                connections: vec![MapConnection {
                    direction: "east".to_string(),
                    target_map: "next".to_string(),
                    offset: 0,
                }],
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: None,
                location: None,
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: None,
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
            vec![0, 0],
        )
    }

    fn map_with_blocks(width: u16, height: u16, blocks: Vec<u16>) -> OverworldMapData {
        OverworldMapData::from_attributes(
            "test",
            &MapAttributes {
                tileset_name: "test".to_string(),
                border_block: 0,
                width,
                height,
                connections: Vec::new(),
                time_of_day: None,
                phone_service: 0,
                phone_flag: false,
                environment: None,
                location: None,
                music: None,
                palette: None,
                fishing_group: None,
                map_constant: None,
                map_group_constant: None,
                blocks_label: None,
                map_scripts_label: None,
                map_events_label: None,
                connection_flags: None,
            },
            blocks,
        )
    }

    fn tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        }
    }

    fn grass_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::TALL_GRASS; 4],
            }],
        }
    }

    fn warp_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [
                    permissions::FLOOR,
                    permissions::WARP_PANEL,
                    permissions::FLOOR,
                    permissions::FLOOR,
                ],
            }],
        }
    }

    fn counter_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
                MetatileCollision {
                    collision: [permissions::COUNTER; 4],
                },
            ],
        }
    }

    fn interaction_tileset(permission: u8) -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
                MetatileCollision {
                    collision: [permission; 4],
                },
            ],
        }
    }

    fn ledge_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [
                        permissions::FLOOR,
                        permissions::FLOOR,
                        permissions::HOP_DOWN,
                        permissions::HOP_DOWN,
                    ],
                },
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
            ],
        }
    }

    fn encounter_data() -> WildEncounterData {
        WildEncounterData {
            map_name: "test".to_string(),
            grass_rates: Some([("day".to_string(), 100)].into_iter().collect()),
            water_rate: None,
            grass: Some(WildEncounterTable {
                morning: Vec::new(),
                day: (0..7)
                    .map(|_| WildEncounter {
                        level: 3,
                        species: "PIDGEY".to_string(),
                    })
                    .collect(),
                night: Vec::new(),
            }),
            water: None,
        }
    }

    fn encounter_slot_tables() -> EncounterSlotTables {
        EncounterSlotTables::for_crystal(
            vec![crate::world::encounters::EncounterSlotChance {
                threshold: 100,
                slot: 0,
            }],
            Vec::new(),
        )
    }

    fn encounter_music_modifiers() -> EncounterMusicModifiers {
        EncounterMusicModifiers {
            modifiers: BTreeMap::new(),
        }
    }

    fn object(identifier: &str, x: u16, y: u16, event_flag: &str) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_TEACHER".to_string(),
            x,
            y,
            spritemovedata: "SPRITEMOVEDATA_STILL".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "TestScript".to_string(),
            label: None,
            event_flag: event_flag.to_string(),
            object_identifier: Some(identifier.to_string()),
            sightline_direction_override: None,
        }
    }

    fn background_event(x: u16, y: u16, event_type: &str, script: &str) -> BackgroundEvent {
        BackgroundEvent {
            x,
            y,
            event_type: event_type.to_string(),
            script: script.to_string(),
        }
    }

    fn coord_event(x: u16, y: u16, scene_id: &str, script_name: &str) -> CoordEvent {
        CoordEvent {
            x,
            y,
            scene_id: scene_id.to_string(),
            script_name: script_name.to_string(),
        }
    }

    #[test]
    fn object_tile_position_uses_exact_runtime_event_tile() {
        assert_eq!(
            object_tile_position_checked(&object("ROUTE29_TEACHER", 2, 3, "-1"))
                .expect("valid object coordinate"),
            TilePosition::new(2, 3)
        );
    }

    #[test]
    fn map_events_share_runtime_tile_coordinate_contract() {
        let stride = StepOptions::default().stride_tiles;
        assert_eq!(stride, DEFAULT_RUNTIME_TILE_STRIDE);

        let warp = WarpEvent {
            index: 1,
            x: 2,
            y: 3,
            target_map_constant: "TARGET_MAP".to_string(),
            target_map: "TargetMap".to_string(),
            target_warp_id: 1,
        };
        let object = object("ROUTE29_TEACHER", 2, 3, "-1");
        let background = background_event(2, 3, "SIGNPOST_READ", "SignScript");
        let coord = coord_event(2, 3, "", "CoordScript");
        let expected = TilePosition::new(2, 3);

        assert_eq!(raw_event_tile_to_runtime_tile_checked(2, 3), Some(expected));
        assert_eq!(warp_tile_position_checked(&warp), Some(expected));
        assert_eq!(object_tile_position_checked(&object), Some(expected));
        assert_eq!(
            background_event_tile_position_checked(&background),
            Some(expected)
        );
        assert_eq!(coord_event_tile_position_checked(&coord), Some(expected));
        assert_eq!(runtime_tile_to_raw_event_tile(expected), Some(expected));
        assert_eq!(
            runtime_tile_to_raw_event_tile(TilePosition::new(i16::MIN, 7)),
            None
        );
        assert_eq!(raw_event_tile_to_runtime_tile_checked(40_000, 40_000), None);
    }

    #[test]
    fn default_input_stride_reaches_next_runtime_warp_tile() {
        let warp = WarpEvent {
            index: 1,
            x: 51,
            y: 2,
            target_map_constant: "ROUTE_29_ROUTE_46_GATE".to_string(),
            target_map: "Route29Route46Gate".to_string(),
            target_warp_id: 3,
        };
        let events = MapEvents {
            warps: vec![warp.clone()],
            ..MapEvents::default()
        };
        let map = map_with_blocks(52, 3, vec![0; 156]);
        let mut session =
            OverworldSession::with_events(map, events, warp_tileset(), TilePosition::new(50, 2));

        let result = session
            .step_and_check_warp_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect("checked step and warp");

        assert_eq!(
            result.outcome,
            StepOutcome::Moved {
                from: TilePosition::new(50, 2),
                to: TilePosition::new(51, 2),
                speed_multiplier: 1,
            }
        );
        assert_eq!(
            result.warp,
            Some(WarpTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(51, 2),
                permission: permissions::WARP_PANEL,
                warp,
            })
        );
    }

    #[test]
    fn session_steps_snapshot_and_presence_are_deterministic() {
        let mut session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));
        let start_hash = session.state_hash();

        assert_eq!(
            session.step(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            ),
            StepOutcome::Moved {
                from: TilePosition::new(0, 0),
                to: TilePosition::new(1, 0),
                speed_multiplier: 1,
            }
        );

        let snapshot = session.snapshot();
        assert_eq!(snapshot.frame, 1);
        assert_eq!(snapshot.tile, TilePosition::new(1, 0));
        assert_ne!(session.state_hash(), start_hash);
        assert_eq!(
            session
                .presence("u1", "Chris", 123)
                .expect("presence")
                .map_name(),
            "test"
        );
    }

    #[test]
    fn input_frame_drives_one_deterministic_runtime_tile() {
        let mut session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));
        let input = PlayerInputFrame::new(1, Frame(0), B_PAD_RIGHT).expect("input frame");

        let result = session
            .apply_input_frame(
                &input,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect("field input");

        assert_eq!(result.frame, 1);
        assert_eq!(
            result.action,
            OverworldInputAction::Step(OverworldStepResult {
                outcome: StepOutcome::Moved {
                    from: TilePosition::new(0, 0),
                    to: TilePosition::new(1, 0),
                    speed_multiplier: 1,
                },
                warp: None,
            })
        );
        assert_eq!(result.snapshot.tile, TilePosition::new(1, 0));
    }

    #[test]
    fn input_frame_jumps_ledge_before_regular_step() {
        let mut session = OverworldSession::new(
            map_with_blocks(1, 2, vec![0, 1]),
            ledge_tileset(),
            TilePosition::new(0, 1),
        );
        let input = PlayerInputFrame::new(1, Frame(0), B_PAD_DOWN).expect("input frame");

        let result = session
            .apply_input_frame(
                &input,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect("field input");

        assert_eq!(
            result.action,
            OverworldInputAction::LedgeJump(OverworldLedgeJumpResult {
                outcome: LedgeJumpOutcome::Jumped {
                    from: TilePosition::new(0, 1),
                    over: TilePosition::new(0, 2),
                    to: TilePosition::new(0, 3),
                    speed_multiplier: 1,
                },
                warp: None,
            })
        );
        assert_eq!(result.snapshot.tile, TilePosition::new(0, 3));
    }

    #[test]
    fn joypad_mask_rejects_conflicting_field_directions() {
        let mut session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));

        let error = session
            .apply_joypad_mask(B_PAD_LEFT | B_PAD_RIGHT, StepOptions::default(), None)
            .expect_err("conflicting directions must fail");

        assert_eq!(
            error,
            OverworldInputError::ConflictingDirections {
                mask: B_PAD_LEFT | B_PAD_RIGHT,
            }
        );
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn a_button_without_target_records_no_interaction_without_idle_fallback() {
        let mut session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));

        let result = session
            .apply_joypad_mask(B_PAD_A, StepOptions::default(), None)
            .expect("A button frame should apply");

        assert_eq!(result.action, OverworldInputAction::NoInteraction);
        assert_eq!(result.joypad_mask, B_PAD_A);
        assert_eq!(result.frame, 1);
    }

    #[test]
    fn session_blocks_steps_into_visible_objects() {
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 2, vec![0; 6]),
            MapEvents::default(),
            vec![object("ROUTE29_TEACHER1", 3, 2, "-1")],
            tileset(),
            TilePosition::new(2, 2),
        );

        let outcome = session.step(
            Direction::Right,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );

        assert_eq!(
            outcome,
            StepOutcome::BlockedByObject {
                at: TilePosition::new(3, 2),
                facing: Direction::Right,
                object_identifier: Some("ROUTE29_TEACHER1".to_string()),
            }
        );
        assert_eq!(session.snapshot().tile, TilePosition::new(2, 2));
        assert_eq!(session.snapshot().frame, 1);
    }

    #[test]
    fn session_uses_exact_event_flags_for_object_visibility() {
        let mut flags = EventFlagMemory::default();
        flags
            .set_event_flag("EVENT_ROUTE_29_POTION", true)
            .expect("set event flag");
        let mut session = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![object("ROUTE29_POKE_BALL", 1, 0, "EVENT_ROUTE_29_POTION")],
            tileset(),
            TilePosition::new(0, 0),
        )
        .with_event_flag_memory(&flags);

        assert!(session.occupied_tiles().is_empty());
        let outcome = session.step(
            Direction::Right,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );

        assert!(matches!(outcome, StepOutcome::Moved { .. }));
        assert_eq!(session.snapshot().tile, TilePosition::new(1, 0));
    }

    #[test]
    fn event_flag_changes_do_not_replace_objects_already_loaded_on_the_map() {
        let first = object("MOM_1", 1, 0, "EVENT_MOM_1");
        let second = object("MOM_2", 2, 0, "EVENT_MOM_2");
        let mut initial_flags = EventFlagMemory::default();
        initial_flags
            .set_event_flag("EVENT_MOM_2", true)
            .expect("hide replacement before map load");
        let mut session = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![first.clone(), second.clone()],
            tileset(),
            TilePosition::new(0, 0),
        )
        .with_event_flag_memory(&initial_flags);
        assert!(session.is_object_visible(&first));
        assert!(!session.is_object_visible(&second));

        let mut next_load_flags = initial_flags;
        next_load_flags
            .set_event_flag("EVENT_MOM_1", true)
            .expect("hide current object for next load");
        next_load_flags
            .set_event_flag("EVENT_MOM_2", false)
            .expect("show replacement on next load");
        session.sync_event_flag_memory(&next_load_flags);

        assert!(
            session.is_object_visible(&first),
            "setevent must not despawn a live object before map reload"
        );
        assert!(
            !session.is_object_visible(&second),
            "clearevent must not load a replacement object before map reload"
        );
        let reloaded = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![first.clone(), second.clone()],
            tileset(),
            TilePosition::new(0, 0),
        )
        .with_event_flag_memory(&next_load_flags);
        assert!(!reloaded.is_object_visible(&first));
        assert!(reloaded.is_object_visible(&second));

        let mut synchronized_on_entry = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![first.clone(), second.clone()],
            tileset(),
            TilePosition::new(0, 0),
        );
        synchronized_on_entry.sync_event_flag_memory(&next_load_flags);
        assert!(!synchronized_on_entry.is_object_visible(&first));
        assert!(synchronized_on_entry.is_object_visible(&second));
    }

    #[test]
    fn session_object_visibility_ignores_engine_flags() {
        let mut flags = EventFlagMemory::default();
        flags
            .set_engine_flag("EVENT_ROUTE_29_POTION", true)
            .expect("set engine flag");
        let session = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![object("ROUTE29_POKE_BALL", 1, 0, "EVENT_ROUTE_29_POTION")],
            tileset(),
            TilePosition::new(0, 0),
        )
        .with_event_flag_memory(&flags);

        assert_eq!(session.occupied_tiles().len(), 1);
    }

    #[test]
    fn session_step_rejects_out_of_range_visible_object_coordinates() {
        let session = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![object("ROUTE29_TEACHER1", 40_000, 0, "-1")],
            tileset(),
            TilePosition::new(0, 0),
        );

        let error = session
            .clone()
            .step_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect_err("invalid object coordinate must reject movement");
        assert_eq!(
            error,
            OverworldCoordinateError::Object(OverworldObjectCoordinateError::OutOfRange {
                object_id: "ROUTE29_TEACHER1".to_string(),
                x: 40_000,
                y: 0,
            })
        );

        let panic = std::panic::catch_unwind(|| {
            let mut session = session;
            session.step(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            );
        });
        assert!(panic.is_err());
    }

    #[test]
    fn interaction_targets_visible_object_on_facing_tile() {
        let mut teacher = object("ROUTE29_TEACHER1", 2, 3, "-1");
        teacher.script = "Route29TeacherScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![teacher],
            tileset(),
            TilePosition::new(2, 2),
        );
        session.player.facing = Direction::Down;

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("checked interaction")
            .expect("object interaction");

        assert_eq!(
            interaction,
            OverworldInteraction {
                map_name: "test".to_string(),
                player_tile: TilePosition::new(2, 2),
                facing: Direction::Down,
                target_tile: TilePosition::new(2, 3),
                script: "Route29TeacherScript".to_string(),
                target: OverworldInteractionTarget::Object {
                    object_index: 1,
                    object_identifier: Some("ROUTE29_TEACHER1".to_string()),
                    object_type: "OBJECTTYPE_SCRIPT".to_string(),
                },
            }
        );
    }

    #[test]
    fn raw_event_coordinates_are_exact_runtime_tiles() {
        assert_eq!(
            raw_event_tile_to_runtime_tile_checked(27, 1),
            Some(TilePosition::new(27, 1))
        );
        assert_eq!(raw_event_tile_to_runtime_tile_checked(u16::MAX, 0), None);
    }

    #[test]
    fn interaction_skips_reserved_object_event_script_without_dispatch() {
        let mut rival = object("CHERRYGROVECITY_RIVAL", 2, 3, "-1");
        rival.script = "ObjectEvent".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![rival],
            tileset(),
            TilePosition::new(2, 2),
        );
        session.player.facing = Direction::Down;

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("checked interaction");

        assert_eq!(interaction, None);
    }

    #[test]
    fn interaction_does_not_coerce_exact_runtime_coordinates_to_adjacent_events() {
        let mut teacher = object("ROUTE29_TEACHER1", 1, 3, "-1");
        teacher.script = "Route29TeacherScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 4, vec![0; 8]),
            MapEvents::default(),
            vec![teacher],
            tileset(),
            TilePosition::new(2, 3),
        );
        session.player.facing = Direction::Down;

        assert_eq!(
            session
                .check_interaction_checked(StepOptions::default().stride_tiles)
                .expect("checked interaction"),
            None
        );
    }

    #[test]
    fn checked_interaction_rejects_non_runtime_stride() {
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 4, vec![0; 8]),
            MapEvents::default(),
            Vec::new(),
            tileset(),
            TilePosition::new(2, 2),
        );

        let error = session
            .check_interaction_checked(2)
            .expect_err("checked interaction must use canonical runtime stride");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::InvalidRuntimeStride {
                stride_tiles: 2,
                metatile_width: METATILE_WIDTH,
            })
        );
    }

    #[test]
    fn checked_interaction_rejects_runtime_front_tile_overflow() {
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 4, vec![0; 8]),
            MapEvents::default(),
            Vec::new(),
            tileset(),
            TilePosition::new(i16::MAX, 0),
        );
        session.player.facing = Direction::Right;

        let error = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect_err("checked interaction must reject overflowing runtime front tiles");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::RuntimeTileOverflow {
                x: i16::MAX,
                y: 0,
                facing: Direction::Right,
            })
        );
    }

    #[test]
    fn checked_step_rejects_non_runtime_stride() {
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 3, vec![0; 9]),
            MapEvents::default(),
            Vec::new(),
            tileset(),
            TilePosition::new(2, 2),
        );
        let mut options = StepOptions::default();
        options.stride_tiles = 2;

        let error = session
            .step_checked(Direction::Right, options)
            .expect_err("checked movement must use canonical runtime stride");

        assert_eq!(
            error,
            OverworldCoordinateError::InvalidRuntimeStride {
                stride_tiles: 2,
                metatile_width: METATILE_WIDTH,
            }
        );
    }

    #[test]
    fn checked_step_accepts_odd_runtime_player_tile() {
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 3, vec![0; 9]),
            MapEvents::default(),
            Vec::new(),
            tileset(),
            TilePosition::new(1, 0),
        );

        let outcome = session
            .step_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect("odd runtime tile is a valid tile coordinate");

        assert_eq!(
            outcome,
            StepOutcome::Moved {
                from: TilePosition::new(1, 0),
                to: TilePosition::new(2, 0),
                speed_multiplier: 1,
            }
        );
        assert_eq!(session.player.tile, TilePosition::new(2, 0));
        assert_eq!(session.frame, 1);
    }

    #[test]
    fn checked_interaction_accepts_odd_runtime_player_tile() {
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 3, vec![0; 9]),
            MapEvents::default(),
            Vec::new(),
            tileset(),
            TilePosition::new(0, 1),
        );

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("odd runtime tile is a valid tile coordinate");

        assert_eq!(interaction, None);
    }

    #[test]
    fn interaction_uses_lowest_visible_object_slot_on_shared_tile() {
        let mut first = object("FIRST_OBJECT", 2, 3, "-1");
        first.script = "FirstScript".to_string();
        let mut second = object("SECOND_OBJECT", 2, 3, "-1");
        second.script = "SecondScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![first, second],
            tileset(),
            TilePosition::new(2, 2),
        );
        session.player.facing = Direction::Down;

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("checked interaction")
            .expect("object interaction");

        assert_eq!(interaction.script, "FirstScript");
        assert_eq!(
            interaction.target,
            OverworldInteractionTarget::Object {
                object_index: 1,
                object_identifier: Some("FIRST_OBJECT".to_string()),
                object_type: "OBJECTTYPE_SCRIPT".to_string(),
            }
        );
    }

    #[test]
    fn checked_visible_object_lookup_rejects_invalid_runtime_object_coordinates() {
        let invalid = object("BROKEN_OBJECT", u16::MAX, 1, "-1");
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(1, 2, vec![0, 0]),
            MapEvents::default(),
            vec![invalid],
            tileset(),
            TilePosition::new(1, 1),
        );

        let error = session
            .visible_object_at_checked(TilePosition::new(1, 2))
            .expect_err("invalid visible object coordinate must fail checked lookup");

        assert_eq!(
            error,
            OverworldObjectCoordinateError::OutOfRange {
                object_id: "BROKEN_OBJECT".to_string(),
                x: u16::MAX,
                y: 1,
            }
        );
    }

    #[test]
    fn input_interaction_rejects_invalid_object_coordinates_without_advancing_frame() {
        let invalid = object("BROKEN_OBJECT", u16::MAX, 1, "-1");
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(1, 2, vec![0, 0]),
            MapEvents::default(),
            vec![invalid],
            tileset(),
            TilePosition::new(1, 1),
        );
        session.player.facing = Direction::Down;

        let error = session
            .apply_joypad_mask(B_PAD_A, StepOptions::default(), None)
            .expect_err("invalid visible object coordinate must reject input");

        assert_eq!(
            error,
            OverworldInputError::ObjectCoordinate(OverworldObjectCoordinateError::OutOfRange {
                object_id: "BROKEN_OBJECT".to_string(),
                x: u16::MAX,
                y: 1,
            })
        );
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn input_movement_rejects_invalid_visible_trainer_coordinates_before_sight_check() {
        let mut invalid_trainer = object("BROKEN_TRAINER", u16::MAX, 1, "-1");
        invalid_trainer.radius = 2;
        invalid_trainer.sightline_direction_override = Some("DOWN".to_string());
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 2, vec![0, 0, 0, 0]),
            MapEvents::default(),
            vec![invalid_trainer],
            tileset(),
            TilePosition::new(0, 0),
        );
        session.player.facing = Direction::Right;

        let error = session
            .apply_joypad_mask(
                B_PAD_RIGHT,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect_err("invalid visible trainer coordinate must reject movement input");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::Object(
                OverworldObjectCoordinateError::OutOfRange {
                    object_id: "BROKEN_TRAINER".to_string(),
                    x: u16::MAX,
                    y: 1,
                }
            ))
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn input_movement_rejects_invalid_occupied_object_coordinates_without_moving() {
        let invalid = object("BROKEN_OBJECT", u16::MAX, 1, "-1");
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 2, vec![0, 0, 0, 0]),
            MapEvents::default(),
            vec![invalid],
            tileset(),
            TilePosition::new(0, 0),
        );
        session.player.facing = Direction::Right;

        let error = session
            .apply_joypad_mask(
                B_PAD_RIGHT,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect_err("invalid occupied object coordinate must reject movement input");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::Object(
                OverworldObjectCoordinateError::OutOfRange {
                    object_id: "BROKEN_OBJECT".to_string(),
                    x: u16::MAX,
                    y: 1,
                }
            ))
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn interaction_does_not_hide_objects_with_case_changed_event_flags() {
        let hidden_flags = BTreeSet::from(["event_route_29_potion".to_string()]);
        let mut item = object("ROUTE29_POKE_BALL", 2, 3, "EVENT_ROUTE_29_POTION");
        item.object_type = "OBJECTTYPE_ITEMBALL".to_string();
        item.script = "Route29Potion".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![item],
            tileset(),
            TilePosition::new(2, 2),
        )
        .with_hidden_event_flags(hidden_flags);
        session.player.facing = Direction::Down;

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("checked interaction")
            .expect("exact flag visible");

        assert_eq!(interaction.script, "Route29Potion");
        assert_eq!(
            interaction.target,
            OverworldInteractionTarget::Object {
                object_index: 1,
                object_identifier: Some("ROUTE29_POKE_BALL".to_string()),
                object_type: "OBJECTTYPE_ITEMBALL".to_string(),
            }
        );
    }

    #[test]
    fn interaction_extends_across_counter_collision_to_object() {
        let mut clerk = object("MART_CLERK", 2, 0, "-1");
        clerk.script = "MartClerkScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 1, vec![1, 0]),
            MapEvents::default(),
            vec![clerk],
            counter_tileset(),
            TilePosition::new(0, 0),
        );
        session.player.facing = Direction::Right;

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("checked interaction")
            .expect("counter object");

        assert_eq!(interaction.target_tile, TilePosition::new(2, 0));
        assert_eq!(interaction.script, "MartClerkScript");
    }

    #[test]
    fn counter_adjustment_rejects_overflowing_tiles() {
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 1, vec![0, 1, 0]),
            MapEvents::default(),
            Vec::new(),
            counter_tileset(),
            TilePosition::new(i16::MAX, i16::MAX),
        );
        session.player.facing = Direction::Right;

        assert_eq!(
            session.counter_adjusted_tile(TilePosition::new(i16::MIN, i16::MAX)),
            TilePosition::new(i16::MIN, i16::MAX)
        );
    }

    #[test]
    fn interaction_targets_background_events_by_raw_event_tile() {
        let events = MapEvents {
            bg_events: vec![background_event(3, 2, "BGEVENT_READ", "SignpostScript")],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(4, 3, vec![0; 12]),
            events,
            tileset(),
            TilePosition::new(2, 2),
        );
        session.player.facing = Direction::Right;

        let interaction = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect("checked interaction")
            .expect("background event");

        assert_eq!(
            interaction,
            OverworldInteraction {
                map_name: "test".to_string(),
                player_tile: TilePosition::new(2, 2),
                facing: Direction::Right,
                target_tile: TilePosition::new(3, 2),
                script: "SignpostScript".to_string(),
                target: OverworldInteractionTarget::Background {
                    event_type: "BGEVENT_READ".to_string(),
                },
            }
        );
    }

    #[test]
    fn directional_background_events_require_the_canonical_player_facing() {
        let events = MapEvents {
            bg_events: vec![
                background_event(3, 2, "BGEVENT_UP", "WrongSideComputerScript"),
                background_event(2, 1, "BGEVENT_UP", "ComputerScript"),
            ],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(4, 3, vec![0; 12]),
            events,
            tileset(),
            TilePosition::new(2, 2),
        );

        session.player.facing = Direction::Right;
        assert_eq!(
            session
                .check_interaction_checked(StepOptions::default().stride_tiles)
                .expect("wrong-facing background check"),
            None
        );

        session.player.facing = Direction::Up;
        assert_eq!(
            session
                .check_interaction_checked(StepOptions::default().stride_tiles)
                .expect("correct-facing background check")
                .map(|interaction| interaction.script),
            Some("ComputerScript".to_string())
        );
    }

    #[test]
    fn interaction_dispatches_standard_scripts_for_interactive_collision_tiles() {
        for (permission, script) in [
            (permissions::BOOKSHELF, "MagazineBookshelfScript"),
            (permissions::PC, "PCScript"),
            (permissions::RADIO, "Radio1Script"),
            (permissions::TOWN_MAP, "TownMapScript"),
            (permissions::TV, "TVScript"),
        ] {
            let mut session = OverworldSession::with_events(
                map_with_blocks(2, 1, vec![0, 1]),
                MapEvents::default(),
                interaction_tileset(permission),
                TilePosition::new(1, 0),
            );
            session.player.facing = Direction::Right;

            let interaction = session
                .check_interaction_checked(StepOptions::default().stride_tiles)
                .expect("checked interaction")
                .expect("interactive collision tile");

            assert_eq!(interaction.script, script, "permission {permission:#04x}");
            assert_eq!(interaction.target_tile, TilePosition::new(2, 0));
            assert_eq!(
                interaction.target,
                OverworldInteractionTarget::Collision { permission }
            );
        }
    }

    #[test]
    fn input_interaction_rejects_invalid_background_event_coordinates_without_advancing_frame() {
        let events = MapEvents {
            bg_events: vec![background_event(
                u16::MAX,
                1,
                "BGEVENT_READ",
                "BrokenSignpostScript",
            )],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(3, 2, vec![0, 0, 0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(2, 2),
        );
        session.player.facing = Direction::Right;

        let error = session
            .apply_joypad_mask(B_PAD_A, StepOptions::default(), None)
            .expect_err("invalid background event coordinate must reject input");

        assert_eq!(
            error,
            OverworldInputError::EventCoordinate(
                OverworldEventCoordinateError::BackgroundOutOfRange {
                    script: "BrokenSignpostScript".to_string(),
                    x: u16::MAX,
                    y: 1,
                }
            )
        );
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn trainer_sight_uses_raw_object_tile_radius() {
        let mut trainer = object("ROUTE29_YOUNGSTER", 2, 0, "-1");
        trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        trainer.radius = 2;
        trainer.sightline_direction_override = Some("DOWN".to_string());
        trainer.script = "Route29YoungsterScript".to_string();
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![trainer],
            tileset(),
            TilePosition::new(2, 2),
        );

        let sight = session
            .check_trainer_sight_checked()
            .expect("checked trainer sight")
            .expect("trainer sight");

        assert_eq!(
            sight,
            OverworldInteraction {
                map_name: "test".to_string(),
                player_tile: TilePosition::new(2, 2),
                facing: Direction::Down,
                target_tile: TilePosition::new(2, 0),
                script: "Route29YoungsterScript".to_string(),
                target: OverworldInteractionTarget::Object {
                    object_index: 1,
                    object_identifier: Some("ROUTE29_YOUNGSTER".to_string()),
                    object_type: "OBJECTTYPE_TRAINER".to_string(),
                },
            }
        );
    }

    #[test]
    fn trainer_object_type_triggers_sightline_battle() {
        let mut trainer = object("ROUTE29_YOUNGSTER", 2, 0, "-1");
        trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        trainer.radius = 2;
        trainer.sightline_direction_override = Some("DOWN".to_string());
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![trainer],
            tileset(),
            TilePosition::new(2, 2),
        );
        let sight = session
            .check_trainer_sight_checked()
            .expect("checked trainer sight")
            .expect("trainer object sight");
        assert_eq!(sight.script, "TestScript");
        assert_eq!(
            sight.target,
            OverworldInteractionTarget::Object {
                object_index: 1,
                object_identifier: Some("ROUTE29_YOUNGSTER".to_string()),
                object_type: "OBJECTTYPE_TRAINER".to_string(),
            }
        );
    }

    #[test]
    fn trainer_sight_ignores_intermediate_objects_like_crystal() {
        let mut trainer = object("ROUTE29_YOUNGSTER", 2, 0, "-1");
        trainer.object_type = "OBJECTTYPE_TRAINER".to_string();
        trainer.radius = 2;
        trainer.sightline_direction_override = Some("DOWN".to_string());
        let blocker = object("ROUTE29_TEACHER", 2, 1, "-1");
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![trainer, blocker],
            tileset(),
            TilePosition::new(2, 2),
        );

        let sight = session
            .check_trainer_sight_checked()
            .expect("checked trainer sight")
            .expect("intermediate objects do not block trainer sight");
        assert_eq!(sight.target_tile, TilePosition::new(2, 0));
    }

    #[test]
    fn trainer_sightline_rejects_extreme_coordinate_distance_without_overflow() {
        assert!(!tile_is_in_sightline(
            TilePosition::new(i16::MIN, 0),
            Direction::Right,
            TilePosition::new(i16::MAX, 0),
            u16::MAX,
        ));
    }

    #[test]
    fn coord_event_triggers_for_matching_scene_and_raw_event_tile() {
        let events = MapEvents {
            coord_events: vec![coord_event(
                2,
                0,
                "SCENE_ROUTE29_CATCHING_TUTORIAL",
                "Route29Tutorial1",
            )],
            ..MapEvents::default()
        };
        let session = OverworldSession::with_events(
            map_with_blocks(3, 2, vec![0, 0, 0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(2, 0),
        );

        assert_eq!(
            session
                .check_coord_event_checked(Some("SCENE_ROUTE29_CATCHING_TUTORIAL"))
                .expect("checked coord event"),
            Some(CoordEventTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(2, 0),
                scene_id: "SCENE_ROUTE29_CATCHING_TUTORIAL".to_string(),
                script_name: "Route29Tutorial1".to_string(),
            })
        );
    }

    #[test]
    fn coord_event_rejects_scene_case_changes_without_normalization() {
        let events = MapEvents {
            coord_events: vec![coord_event(
                1,
                0,
                "SCENE_ROUTE29_CATCHING_TUTORIAL",
                "Route29Tutorial1",
            )],
            ..MapEvents::default()
        };
        let session = OverworldSession::with_events(
            map_with_blocks(3, 2, vec![0, 0, 0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(2, 0),
        );

        assert_eq!(
            session
                .check_coord_event_checked(Some("scene_route29_catching_tutorial"))
                .expect("checked coord event"),
            None
        );
        assert_eq!(
            session
                .check_coord_event_checked(None)
                .expect("checked coord event"),
            None
        );
    }

    #[test]
    fn coord_event_with_empty_scene_id_triggers_without_scene_fallback() {
        let events = MapEvents {
            coord_events: vec![coord_event(0, 1, "", "AlwaysScript")],
            ..MapEvents::default()
        };
        let session = OverworldSession::with_events(
            map_with_blocks(2, 3, vec![0, 0, 0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(0, 1),
        );

        assert_eq!(
            session
                .check_coord_event_checked(None)
                .expect("checked coord event"),
            Some(CoordEventTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(0, 1),
                scene_id: String::new(),
                script_name: "AlwaysScript".to_string(),
            })
        );
    }

    #[test]
    fn coord_event_uses_first_matching_event_in_pack_order() {
        let events = MapEvents {
            coord_events: vec![
                coord_event(0, 1, "", "FirstScript"),
                coord_event(0, 1, "", "SecondScript"),
            ],
            ..MapEvents::default()
        };
        let session = OverworldSession::with_events(
            map_with_blocks(2, 3, vec![0, 0, 0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(0, 1),
        );

        assert_eq!(
            session
                .check_coord_event_checked(Some("ANY_SCENE"))
                .expect("checked coord event")
                .expect("coord event")
                .script_name,
            "FirstScript"
        );
    }

    #[test]
    fn input_movement_rejects_invalid_coord_event_coordinates() {
        let events = MapEvents {
            coord_events: vec![coord_event(u16::MAX, 0, "", "BrokenCoordScript")],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(2, 2, vec![0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(0, 0),
        );
        session.player.facing = Direction::Right;

        let error = session
            .apply_joypad_mask(
                B_PAD_RIGHT,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect_err("invalid coord event coordinate must reject input");

        assert_eq!(
            error,
            OverworldInputError::EventCoordinate(OverworldEventCoordinateError::CoordOutOfRange {
                script: "BrokenCoordScript".to_string(),
                x: u16::MAX,
                y: 0,
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn exact_walking_rate_random_uses_cleanse_shift_carry_and_no_tag_clear_carry() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));

        let mut cleanse_divider = crate::random::ReplayDivider::new([0, 0]);
        let mut cleanse_rng = CrystalRandom::new(
            CrystalRandomState { add: 0xff, sub: 0 },
            &mut cleanse_divider,
        );
        let roaming = std::array::from_fn(|_| RoamingPokemonState::default());
        let exact_context = ExactEncounterContext {
            roaming_pokemon: &roaming,
            current_map: (1, 1),
            bug_contest_encounters: None,
            unlocked_unown_sets: u8::MAX,
        };
        let cleanse = session
            .check_wild_encounter_exact(
                Some(&encounter_data()),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut cleanse_rng,
                EncounterCheckOptions {
                    has_cleanse_tag: true,
                    ..EncounterCheckOptions::default()
                },
                exact_context,
            )
            .expect("exact Cleanse Tag rate roll")
            .expect("grass permits an encounter check");
        assert_eq!(cleanse.threshold, 127);
        assert_eq!(cleanse.encounter_roll, 0xff);
        assert_eq!(cleanse.resolved, None);
        assert_eq!(cleanse_divider.remaining(), 0);

        let mut zero_rate = encounter_data();
        for rate in zero_rate
            .grass_rates
            .as_mut()
            .expect("grass rates")
            .values_mut()
        {
            *rate = 0;
        }
        let mut clear_divider = crate::random::ReplayDivider::new([0, 0]);
        let mut clear_rng =
            CrystalRandom::new(CrystalRandomState { add: 0xff, sub: 0 }, &mut clear_divider);
        let clear = session
            .check_wild_encounter_exact(
                Some(&zero_rate),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut clear_rng,
                EncounterCheckOptions::default(),
                exact_context,
            )
            .expect("exact no-Cleanse rate roll")
            .expect("grass permits an encounter check");
        assert_eq!(clear.threshold, 0);
        assert_eq!(clear.encounter_roll, 0);
        assert_eq!(clear.resolved, None);
        assert_eq!(clear_divider.remaining(), 0);
    }

    #[test]
    fn exact_sweet_scent_skips_rate_and_repel_and_uses_live_collision_surface() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        // selector roll 0 misses roaming; slot roll 0 selects percent 1.
        let mut divider = crate::random::ReplayDivider::new([0, 0, 0, 0]);
        let mut rng = CrystalRandom::new(CrystalRandomState::default(), &mut divider);
        let roaming = std::array::from_fn(|_| RoamingPokemonState::default());
        let roll = session
            .check_sweet_scent_encounter_exact(
                Some(&encounter_data()),
                &encounter_slot_tables(),
                &mut rng,
                EncounterCheckOptions {
                    active_repel_item: Some("REPEL".to_string()),
                    lead_party_level: Some(u8::MAX),
                    ..EncounterCheckOptions::default()
                },
                ExactEncounterContext {
                    roaming_pokemon: &roaming,
                    current_map: (1, 1),
                    bug_contest_encounters: None,
                    unlocked_unown_sets: u8::MAX,
                },
            )
            .expect("exact Sweet Scent chooser")
            .expect("Sweet Scent finds a grass encounter");
        assert_eq!(roll.surface, EncounterSurface::Grass);
        assert!(roll.resolved.is_some(), "Sweet Scent never applies Repel");
        assert_eq!(roll.repelled_by, None);
        assert_eq!(divider.remaining(), 0, "there is no rate Random call");
    }

    #[test]
    fn checked_current_encounter_surface_rejects_out_of_bounds_runtime_tile() {
        let odd_tile = OverworldSession::new(map(), grass_tileset(), TilePosition::new(1, 0));
        assert_eq!(
            odd_tile
                .current_encounter_surface_checked()
                .expect("odd runtime tile is a valid tile coordinate"),
            Some(EncounterSurface::Grass)
        );

        let outside = OverworldSession::new(map(), grass_tileset(), TilePosition::new(4, 0));
        assert_eq!(
            outside
                .current_encounter_surface_checked()
                .expect_err("surface query must reject out-of-bounds runtime tiles"),
            EncounterError::RuntimeTileOutOfBounds {
                map_name: "test".to_string(),
                x: 4,
                y: 0,
                width: 4,
                height: 2,
            }
        );
    }

    #[test]
    fn encounter_surface_uses_crystals_complete_grass_catalog() {
        let session = OverworldSession::new(
            map(),
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::LONG_GRASS; 4],
                }],
            },
            TilePosition::new(0, 0),
        );

        assert_eq!(
            session
                .current_encounter_surface_checked()
                .expect("long-grass surface"),
            Some(EncounterSurface::Grass)
        );
    }

    #[test]
    fn cave_environment_allows_non_ice_floor_encounters() {
        let floor = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));
        assert_eq!(
            floor
                .current_encounter_surface_checked_with_land_encounters(false)
                .expect("outdoor floor surface"),
            None
        );
        assert_eq!(
            floor
                .current_encounter_surface_checked_with_land_encounters(true)
                .expect("cave floor surface"),
            Some(EncounterSurface::Grass)
        );

        let ice = OverworldSession::new(
            map(),
            TilesetCollision {
                metatiles: vec![MetatileCollision {
                    collision: [permissions::ICE; 4],
                }],
            },
            TilePosition::new(0, 0),
        );
        assert_eq!(
            ice.current_encounter_surface_checked_with_land_encounters(true)
                .expect("cave ice surface"),
            None
        );
    }

    #[test]
    fn session_reports_warp_trigger_from_map_events() {
        let warp = WarpEvent {
            index: 1,
            x: 1,
            y: 2,
            target_map_constant: "TARGET_MAP".to_string(),
            target_map: "TargetMap".to_string(),
            target_warp_id: 2,
        };
        let events = MapEvents {
            warps: vec![warp.clone()],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(3, 3, vec![0; 9]),
            events,
            warp_tileset(),
            TilePosition::new(0, 2),
        );

        assert_eq!(session.check_warp_checked().expect("checked warp"), None);

        let result = session
            .step_and_check_warp_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect("checked step and warp");

        assert_eq!(
            result.outcome,
            StepOutcome::Moved {
                from: TilePosition::new(0, 2),
                to: TilePosition::new(1, 2),
                speed_multiplier: 1,
            }
        );
        assert_eq!(
            result.warp,
            Some(WarpTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(1, 2),
                permission: permissions::WARP_PANEL,
                warp,
            })
        );
    }

    #[test]
    fn input_movement_rejects_invalid_warp_coordinates() {
        let warp = WarpEvent {
            index: 7,
            x: u16::MAX,
            y: 1,
            target_map_constant: "TARGET_MAP".to_string(),
            target_map: "TargetMap".to_string(),
            target_warp_id: 2,
        };
        let events = MapEvents {
            warps: vec![warp],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(2, 2, vec![0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(0, 0),
        );
        session.player.facing = Direction::Right;

        let error = session
            .apply_joypad_mask(
                B_PAD_RIGHT,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect_err("invalid warp coordinate must reject input");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::Event(
                OverworldEventCoordinateError::WarpOutOfRange {
                    index: 7,
                    x: u16::MAX,
                    y: 1,
                }
            ))
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn input_movement_rejects_connection_bounds_overflow_without_committing_step() {
        let mut attributes = attributes(40_000, 1);
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        let map = OverworldMapData::from_attributes("oversized", &attributes, vec![0; 40_000]);
        let mut session = OverworldSession::new(map, tileset(), TilePosition::new(0, 0));
        session.player.facing = Direction::Right;

        let error = session
            .apply_joypad_mask(
                B_PAD_RIGHT,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
            )
            .expect_err("connection bounds overflow must reject input");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::MapBoundsOverflow {
                map_name: "oversized".to_string(),
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn checked_step_rejects_map_bounds_overflow_before_collision_sampling() {
        let attributes = attributes(40_000, 1);
        let map = OverworldMapData::from_attributes("oversized", &attributes, vec![0; 40_000]);
        let mut session = OverworldSession::new(map, tileset(), TilePosition::new(0, 0));

        let error = session
            .step_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect_err("oversized map bounds must reject checked step");

        assert_eq!(
            error,
            OverworldCoordinateError::MapBoundsOverflow {
                map_name: "oversized".to_string(),
            }
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn checked_step_rejects_missing_follow_object_before_moving_leader() {
        let mut session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));
        session.following = Some(OverworldFollowState {
            leader_object_id: "PLAYER".to_string(),
            follower_object_id: "MISSING_FOLLOWER".to_string(),
        });

        let error = session
            .step_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect_err("missing follower must reject checked movement");

        assert_eq!(
            error,
            OverworldCoordinateError::FollowObjectMissing {
                object_id: "MISSING_FOLLOWER".to_string(),
            }
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn checked_step_warp_rejects_invalid_warp_coordinates_without_committing_step() {
        let warp = WarpEvent {
            index: 7,
            x: u16::MAX,
            y: 1,
            target_map_constant: "TARGET_MAP".to_string(),
            target_map: "TargetMap".to_string(),
            target_warp_id: 2,
        };
        let events = MapEvents {
            warps: vec![warp],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(2, 2, vec![0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(0, 0),
        );

        let error = session
            .step_and_check_warp_checked(
                Direction::Right,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect_err("invalid warp coordinate must reject checked step");

        assert_eq!(
            error,
            OverworldCoordinateError::Event(OverworldEventCoordinateError::WarpOutOfRange {
                index: 7,
                x: u16::MAX,
                y: 1,
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn checked_ledge_warp_does_not_fire_without_jump() {
        let warp = WarpEvent {
            index: 8,
            x: u16::MAX,
            y: 1,
            target_map_constant: "TARGET_MAP".to_string(),
            target_map: "TargetMap".to_string(),
            target_warp_id: 2,
        };
        let events = MapEvents {
            warps: vec![warp],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(1, 3, vec![0, 1, 0]),
            events,
            ledge_tileset(),
            TilePosition::new(0, 0),
        );

        let result = session
            .ledge_jump_and_check_warp_checked(
                Direction::Down,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect("non-ledge input should not evaluate warp coordinates");

        assert!(matches!(result.outcome, LedgeJumpOutcome::NotLedge { .. }));
        assert_eq!(result.warp, None);
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 1);
    }

    #[test]
    fn warp_transition_builds_destination_session_without_loading_fallbacks() {
        let trigger = WarpTrigger {
            map_name: "source".to_string(),
            tile: TilePosition::new(1, 1),
            permission: permissions::DOOR,
            warp: WarpEvent {
                index: 1,
                x: 0,
                y: 0,
                target_map_constant: "TARGET_MAP".to_string(),
                target_map: "TargetMap".to_string(),
                target_warp_id: 1,
            },
        };
        let transition = WarpTransition {
            trigger,
            destination: WarpDestination {
                map_name: "target".to_string(),
                tile: TilePosition::new(4, 6),
                warp: WarpEvent {
                    index: 1,
                    x: 2,
                    y: 3,
                    target_map_constant: "SOURCE_MAP".to_string(),
                    target_map: "SourceMap".to_string(),
                    target_warp_id: 1,
                },
            },
        };

        let destination_objects = vec![object("DESTINATION_NPC", 1, 1, "-1")];
        let session = transition.apply_to(
            map(),
            MapEvents::default(),
            destination_objects.clone(),
            tileset(),
            42,
            MovementMode::Surf,
        );

        assert_eq!(session.frame, 42);
        assert_eq!(session.player.tile, TilePosition::new(4, 6));
        assert_eq!(session.player.mode, MovementMode::Surf);
        assert_eq!(session.objects, destination_objects);
    }

    #[test]
    fn session_reports_connection_trigger_when_player_crosses_declared_boundary() {
        let session =
            OverworldSession::new(map_with_connections(), tileset(), TilePosition::new(4, 2));

        let trigger = session
            .check_connection_checked()
            .expect("checked connection")
            .expect("east connection trigger");

        assert_eq!(trigger.map_name, "test");
        assert_eq!(trigger.tile, TilePosition::new(4, 2));
        assert_eq!(trigger.connection.target_map, "next");
        assert_eq!(trigger.connection.direction, "east");
    }

    #[test]
    fn wide_map_connection_trigger_checks_do_not_narrow_tile_bounds() {
        let mut attributes = attributes(20_000, 1);
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        let map = OverworldMapData::from_attributes("wide", &attributes, vec![0; 20_000]);
        let session = OverworldSession::new(map, tileset(), TilePosition::new(i16::MAX, 0));

        assert_eq!(
            session
                .check_connection_checked()
                .expect("checked connection"),
            None
        );
    }

    #[test]
    fn checked_connection_rejects_unsupported_connection_directions() {
        let mut attributes = attributes(3, 3);
        attributes.connections.push(MapConnection {
            direction: "up".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        let map = OverworldMapData::from_attributes("bad_connection", &attributes, vec![0; 9]);
        let session = OverworldSession::new(map, tileset(), TilePosition::new(0, 0));

        assert_eq!(
            session.check_connection_checked(),
            Err(OverworldCoordinateError::UnsupportedConnectionDirection {
                map_name: "bad_connection".to_string(),
                direction: "up".to_string(),
            })
        );
    }

    #[test]
    fn checked_connection_rejects_duplicate_connection_directions() {
        let mut attributes = attributes(3, 3);
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "other".to_string(),
            offset: 0,
        });
        let map =
            OverworldMapData::from_attributes("duplicate_connection", &attributes, vec![0; 9]);
        let session = OverworldSession::new(map, tileset(), TilePosition::new(0, 0));

        assert_eq!(
            session.check_connection_checked(),
            Err(OverworldCoordinateError::DuplicateConnectionDirection {
                map_name: "duplicate_connection".to_string(),
                direction: "east".to_string(),
            })
        );
    }

    #[test]
    fn checked_connection_rejects_ambiguous_corner_boundary_matches() {
        let mut attributes = attributes(3, 3);
        attributes.connections.push(MapConnection {
            direction: "north".to_string(),
            target_map: "north_map".to_string(),
            offset: 0,
        });
        attributes.connections.push(MapConnection {
            direction: "west".to_string(),
            target_map: "west_map".to_string(),
            offset: 0,
        });
        let map = OverworldMapData::from_attributes("corner_connection", &attributes, vec![0; 9]);
        let session = OverworldSession::new(map, tileset(), TilePosition::new(-2, -2));

        assert_eq!(
            session.check_connection_checked(),
            Err(OverworldCoordinateError::AmbiguousConnectionBoundary {
                map_name: "corner_connection".to_string(),
                x: -2,
                y: -2,
            })
        );
    }

    #[test]
    fn checked_connection_rejects_maps_with_overflowing_runtime_tile_bounds() {
        let mut attributes = attributes(40_000, 1);
        attributes.connections.push(MapConnection {
            direction: "east".to_string(),
            target_map: "next".to_string(),
            offset: 0,
        });
        let map = OverworldMapData::from_attributes("oversized", &attributes, vec![0; 40_000]);
        let session = OverworldSession::new(map, tileset(), TilePosition::new(0, 0));

        assert_eq!(
            session.check_connection_checked(),
            Err(OverworldCoordinateError::MapBoundsOverflow {
                map_name: "oversized".to_string(),
            })
        );
    }

    #[test]
    fn checked_connection_accepts_odd_player_runtime_tile() {
        let session =
            OverworldSession::new(map_with_connections(), tileset(), TilePosition::new(5, 2));

        assert_eq!(
            session.check_connection_checked(),
            Ok(Some(ConnectionTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(5, 2),
                connection: MapConnection {
                    direction: "east".to_string(),
                    target_map: "next".to_string(),
                    offset: 0,
                },
            }))
        );
    }

    #[test]
    fn connection_transition_builds_destination_session_without_loading_fallbacks() {
        let transition = ConnectionTransition {
            trigger: ConnectionTrigger {
                map_name: "source".to_string(),
                tile: TilePosition::new(4, 2),
                connection: MapConnection {
                    direction: "east".to_string(),
                    target_map: "target".to_string(),
                    offset: 0,
                },
            },
            destination: ConnectionDestination {
                map_name: "target".to_string(),
                tile: TilePosition::new(0, 2),
            },
        };

        let destination_objects = vec![object("CONNECTION_NPC", 1, 1, "-1")];
        let session = transition.apply_to(
            map(),
            MapEvents::default(),
            destination_objects.clone(),
            tileset(),
            77,
            MovementMode::Surf,
        );

        assert_eq!(session.frame, 77);
        assert_eq!(session.player.tile, TilePosition::new(0, 2));
        assert_eq!(session.player.mode, MovementMode::Surf);
        assert_eq!(session.objects, destination_objects);
    }
}
