use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use crate::input::{B_PAD_A, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP};
use crate::map::{BackgroundEvent, CoordEvent, MapConnection, MapEvents, ObjectEvent, WarpEvent};
use crate::multiplayer::{
    MultiplayerMessageError, OverworldPresence, PlayerInputFrame, PresenceEntityType, fnv1a32,
};
use crate::random::Random;
use crate::state::{EventFlagMemory, GameState, GameStateFrameError};

use super::collision::{
    Terrain, TilesetCollision, can_jump_ledge, describe_collision, permissions, sample_collision,
};
use super::encounters::{
    EncounterError, EncounterMusicModifiers, EncounterSlotTables, EncounterSurface,
    ResolvedWildEncounter, TimeOfDay, WildEncounterData, encounter_threshold,
    passes_encounter_roll, select_wild_encounter,
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
    pub object_facings: BTreeMap<String, Direction>,
    pub following: Option<OverworldFollowState>,
    pub last_talked_object_identifier: Option<String>,
    pub player_hidden: bool,
    pub hidden_event_flags: BTreeSet<String>,
    pub hidden_object_identifiers: BTreeSet<String>,
    pub tileset: TilesetCollision,
    pub player: PlayerMovementState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldFollowState {
    pub leader_object_id: String,
    pub follower_object_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpTrigger {
    pub map_name: String,
    pub tile: TilePosition,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldInputEncounterResult {
    pub input: OverworldInputResult,
    pub wild_encounter: Option<WildEncounterRoll>,
    pub expired_repel_item: Option<String>,
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum OverworldInputTickError {
    #[error(transparent)]
    Input(#[from] OverworldInputError),
    #[error(transparent)]
    Encounter(#[from] EncounterError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverworldObjectCoordinateError {
    #[error("object '{object_id}' has out-of-range runtime coordinates ({x}, {y})")]
    OutOfRange { object_id: String, x: u16, y: u16 },
}

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

fn game_state_frame_error_to_overworld_input(
    error: GameStateFrameError,
) -> OverworldInputTickError {
    match error {
        GameStateFrameError::ConflictingJoypadDirections { mask } => {
            OverworldInputTickError::Input(OverworldInputError::ConflictingDirections { mask })
        }
        GameStateFrameError::FrameCursorOverflow { frame } => {
            OverworldInputTickError::Input(OverworldInputError::FrameMismatch {
                session_frame: frame,
                input_frame: frame,
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldLedgeJumpResult {
    pub outcome: LedgeJumpOutcome,
    pub warp: Option<WarpTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Default for EncounterCheckOptions {
    fn default() -> Self {
        Self {
            time: TimeOfDay::Day,
            music_token: None,
            has_cleanse_tag: false,
            active_repel_item: None,
            lead_party_level: None,
        }
    }
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
    pub resolved: Option<ResolvedWildEncounter>,
    pub repelled_by: Option<String>,
    pub rng_seed_after: u32,
}

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
            object_facings,
            following: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            tileset,
            player: PlayerMovementState::new(player_tile),
        }
    }

    pub fn with_hidden_event_flags(mut self, hidden_event_flags: BTreeSet<String>) -> Self {
        self.hidden_event_flags = hidden_event_flags;
        self
    }

    pub fn with_event_flag_memory(mut self, flags: &EventFlagMemory) -> Self {
        self.hidden_event_flags = flags.active_event_flags().cloned().collect();
        self
    }

    pub fn sync_event_flag_memory(&mut self, flags: &EventFlagMemory) {
        self.hidden_event_flags = flags.active_event_flags().cloned().collect();
    }

    pub fn current_encounter_surface_checked(
        &self,
    ) -> Result<Option<EncounterSurface>, EncounterError> {
        encounter_surface_for_player_tile_checked(self)
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
        if let StepOutcome::Moved { from, to, .. } = outcome {
            self.update_follow_after_entity_move("PLAYER", from, to);
        }
        self.frame += 1;
        Ok(outcome)
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
        if let LedgeJumpOutcome::Jumped { from, to, .. } = outcome {
            self.update_follow_after_entity_move("PLAYER", from, to);
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
        self.set_follow_entity_tile(&following.follower_object_id, from);
        if let Some(direction) = direction_between_tiles(from, to) {
            self.set_follow_entity_facing(&following.follower_object_id, direction);
        }
    }

    fn validate_follow_after_entity_move(
        &self,
        moved_object_id: &str,
        from: TilePosition,
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
        Ok(occupied)
    }

    pub fn is_object_visible(&self, object: &ObjectEvent) -> bool {
        if object
            .object_identifier
            .as_ref()
            .is_some_and(|object_id| self.hidden_object_identifiers.contains(object_id))
        {
            return false;
        }
        object.event_flag == "-1" || !self.hidden_event_flags.contains(&object.event_flag)
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

        if let Some((object_index, object)) = self.visible_object_at_checked(facing_tile)? {
            return Ok(Some(self.object_interaction(
                object_index,
                object,
                facing_tile,
            )));
        }

        let adjusted_tile = self.counter_adjusted_tile(facing_tile);
        if adjusted_tile != facing_tile {
            if let Some((object_index, object)) = self.visible_object_at_checked(adjusted_tile)? {
                return Ok(Some(self.object_interaction(
                    object_index,
                    object,
                    adjusted_tile,
                )));
            }
        }

        Ok(self
            .background_event_at_checked(adjusted_tile)?
            .map(|event| OverworldInteraction {
                map_name: self.map.name.clone(),
                player_tile: self.player.tile,
                facing: self.player.facing,
                target_tile: adjusted_tile,
                script: event.script.clone(),
                target: OverworldInteractionTarget::Background {
                    event_type: event.event_type.clone(),
                },
            }))
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
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, object)| self.is_object_visible(object))
            .filter(|(_, object)| object.object_type == "OBJECTTYPE_SCRIPT" && object.radius > 0)
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
                if !tile_is_in_sightline(object_tile, direction, self.player.tile, object.radius)
                    || !self.has_clear_sightline_checked(
                        object_tile,
                        direction,
                        self.player.tile,
                    )?
                {
                    return Ok(None);
                }
                Ok(Some(self.object_interaction(
                    (index + 1) as u16,
                    object,
                    object_tile,
                )))
            })
    }

    fn has_clear_sightline_checked(
        &self,
        object_tile: TilePosition,
        direction: Direction,
        player_tile: TilePosition,
    ) -> Result<bool, OverworldCoordinateError> {
        let stride = StepOptions::default().stride_tiles;
        let mut cursor =
            checked_move_by_stride(object_tile, direction, stride).ok_or_else(|| {
                OverworldCoordinateError::RuntimeTileOverflow {
                    x: object_tile.x,
                    y: object_tile.y,
                    facing: direction,
                }
            })?;
        while cursor != player_tile {
            if self.visible_object_at_checked(cursor)?.is_some() {
                return Ok(false);
            }
            if sample_collision(&self.map, &self.tileset, cursor)
                .map(|sample| describe_collision(sample.permission).terrain == Terrain::Wall)
                .unwrap_or(true)
            {
                return Ok(false);
            }
            cursor = checked_move_by_stride(cursor, direction, stride).ok_or_else(|| {
                OverworldCoordinateError::RuntimeTileOverflow {
                    x: cursor.x,
                    y: cursor.y,
                    facing: direction,
                }
            })?;
        }
        Ok(true)
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

    pub fn check_warp_checked(&self) -> Result<Option<WarpTrigger>, OverworldEventCoordinateError> {
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
        let warp = staged.check_warp_checked()?;
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

    pub fn apply_input_frame_with_encounter(
        &mut self,
        input: &PlayerInputFrame,
        step_options: StepOptions,
        current_scene: Option<&str>,
        encounters: &WildEncounterData,
        slot_tables: &EncounterSlotTables,
        music_modifiers: &EncounterMusicModifiers,
        rng: &mut Random,
        encounter_options: EncounterCheckOptions,
    ) -> Result<OverworldInputEncounterResult, OverworldInputTickError> {
        if input.frame() != self.frame {
            return Err(OverworldInputTickError::Input(
                OverworldInputError::FrameMismatch {
                    session_frame: self.frame,
                    input_frame: input.frame(),
                },
            ));
        }
        self.apply_joypad_mask_with_encounter(
            input.joypad_mask(),
            step_options,
            current_scene,
            encounters,
            slot_tables,
            music_modifiers,
            rng,
            encounter_options,
        )
    }

    pub fn apply_input_frame_with_state_encounter(
        &mut self,
        state: &mut GameState,
        input: &PlayerInputFrame,
        step_options: StepOptions,
        current_scene: Option<&str>,
        encounters: &WildEncounterData,
        slot_tables: &EncounterSlotTables,
        music_modifiers: &EncounterMusicModifiers,
        rng: &mut Random,
        lead_party_level: Option<u8>,
        mut encounter_options: EncounterCheckOptions,
    ) -> Result<OverworldInputEncounterResult, OverworldInputTickError> {
        let mut staged_state = state.clone();
        staged_state
            .apply_joypad_mask(input.joypad_mask())
            .map_err(game_state_frame_error_to_overworld_input)?;
        encounter_options.active_repel_item = if staged_state.repel_steps_remaining > 0 {
            staged_state.active_repel_item.clone()
        } else {
            None
        };
        encounter_options.lead_party_level =
            lead_party_level.or_else(|| leading_usable_party_level(&staged_state));
        let mut result = self.apply_input_frame_with_encounter(
            input,
            step_options,
            current_scene,
            encounters,
            slot_tables,
            music_modifiers,
            rng,
            encounter_options,
        )?;
        if result.input.action.moves_player() {
            result.expired_repel_item = staged_state.tick_repel_step_after_movement();
        }
        *state = staged_state;
        Ok(result)
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

    pub fn apply_joypad_mask_with_encounter(
        &mut self,
        joypad_mask: u8,
        step_options: StepOptions,
        current_scene: Option<&str>,
        encounters: &WildEncounterData,
        slot_tables: &EncounterSlotTables,
        music_modifiers: &EncounterMusicModifiers,
        rng: &mut Random,
        encounter_options: EncounterCheckOptions,
    ) -> Result<OverworldInputEncounterResult, OverworldInputTickError> {
        let mut staged = self.clone();
        let mut staged_rng = *rng;
        let input = staged
            .apply_joypad_mask(joypad_mask, step_options, current_scene)
            .map_err(OverworldInputTickError::Input)?;
        let wild_encounter = if input.action.moves_player() && input.trainer_sight.is_none() {
            staged
                .check_wild_encounter(
                    encounters,
                    slot_tables,
                    music_modifiers,
                    &mut staged_rng,
                    encounter_options,
                )
                .map_err(OverworldInputTickError::Encounter)?
        } else {
            None
        };
        *self = staged;
        *rng = staged_rng;
        Ok(OverworldInputEncounterResult {
            input,
            wild_encounter,
            expired_repel_item: None,
        })
    }

    fn can_jump_ledge_from_input(
        &self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<bool, OverworldCoordinateError> {
        let stride = require_runtime_stride(options.stride_tiles)?;
        self.require_checked_tile_bounds()?;
        let ledge =
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
            ledge,
            direction,
            stride,
        ))
    }

    pub fn ledge_jump_and_check_warp_checked(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> Result<OverworldLedgeJumpResult, OverworldCoordinateError> {
        let mut staged = self.clone();
        let outcome = staged.ledge_jump_checked(direction, options)?;
        let warp = staged.check_warp_checked()?;
        *self = staged;
        Ok(OverworldLedgeJumpResult { outcome, warp })
    }

    pub fn check_wild_encounter(
        &self,
        encounters: &WildEncounterData,
        slot_tables: &EncounterSlotTables,
        music_modifiers: &EncounterMusicModifiers,
        rng: &mut Random,
        options: EncounterCheckOptions,
    ) -> Result<Option<WildEncounterRoll>, EncounterError> {
        let Some(surface) = encounter_surface_for_player_tile_checked(self)? else {
            return Ok(None);
        };
        let threshold = encounter_threshold(
            encounters,
            surface,
            options.time,
            options.music_token.as_deref(),
            music_modifiers,
            options.has_cleanse_tag,
        )?;
        let encounter_roll = rng.randrange(256) as u8;
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
                resolved: None,
                repelled_by: None,
                rng_seed_after: rng.seed(),
            }));
        }

        let slot_percent_roll = next_percent_roll(rng);
        let level_roll = rng.randrange(256) as u8;
        let resolved = select_wild_encounter(
            encounters,
            slot_tables,
            surface,
            options.time,
            slot_percent_roll,
            level_roll,
        )?;
        let (resolved, repelled_by) = apply_repel_to_wild_encounter(resolved, &options)?;
        Ok(Some(WildEncounterRoll {
            map_name: self.map.name.clone(),
            tile: self.player.tile,
            surface,
            time: options.time,
            threshold,
            encounter_roll,
            slot_percent_roll: Some(slot_percent_roll),
            level_roll: Some(level_roll),
            resolved,
            repelled_by,
            rng_seed_after: rng.seed(),
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
    if encounter.level <= lead_level {
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
        "SPRITEMOVEDATA_WALK_LEFT_RIGHT" | "SPRITEMOVEDATA_STANDING_LEFT" => Some(Direction::Left),
        "SPRITEMOVEDATA_WALK_UP_DOWN" | "SPRITEMOVEDATA_STANDING_RIGHT" => Some(Direction::Right),
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

fn encounter_surface_for_player_tile(session: &OverworldSession) -> Option<EncounterSurface> {
    encounter_surface_for_player_tile_checked(session)
        .expect("encounter surface query requires valid runtime coordinate and collision state")
}

fn encounter_surface_for_player_tile_checked(
    session: &OverworldSession,
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
    if sample.permission == permissions::TALL_GRASS {
        return Ok(Some(EncounterSurface::Grass));
    }
    let attributes = describe_collision(sample.permission);
    if session.player.mode == MovementMode::Surf && attributes.terrain == Terrain::Water {
        return Ok(Some(EncounterSurface::Water));
    }
    Ok(None)
}

fn next_percent_roll(rng: &mut Random) -> u8 {
    loop {
        let value = rng.randrange(256) as u8;
        if value < 100 {
            return value + 1;
        }
    }
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
            object_facings: BTreeMap::new(),
            following: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            tileset,
            player: PlayerMovementState::new(self.destination.tile).with_mode(mode),
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
            object_facings: BTreeMap::new(),
            following: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            tileset,
            player: PlayerMovementState::new(self.destination.tile).with_mode(mode),
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

    fn ledge_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![
                MetatileCollision {
                    collision: [permissions::FLOOR; 4],
                },
                MetatileCollision {
                    collision: [
                        permissions::FLOOR,
                        permissions::HOP_DOWN,
                        permissions::HOP_DOWN,
                        permissions::WALL,
                    ],
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
            x: 26,
            y: 1,
            target_map_constant: "ROUTE_29_ROUTE_46_GATE".to_string(),
            target_map: "Route29Route46Gate".to_string(),
            target_warp_id: 3,
        };
        let events = MapEvents {
            warps: vec![warp.clone()],
            ..MapEvents::default()
        };
        let map = map_with_blocks(27, 2, vec![0; 54]);
        let mut session =
            OverworldSession::with_events(map, events, tileset(), TilePosition::new(50, 2));

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
                to: TilePosition::new(52, 2),
                speed_multiplier: 1,
            }
        );
        assert_eq!(
            result.warp,
            Some(WarpTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(52, 2),
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
                to: TilePosition::new(2, 0),
                speed_multiplier: 1,
            }
        );

        let snapshot = session.snapshot();
        assert_eq!(snapshot.frame, 1);
        assert_eq!(snapshot.tile, TilePosition::new(2, 0));
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
                    to: TilePosition::new(2, 0),
                    speed_multiplier: 1,
                },
                warp: None,
            })
        );
        assert_eq!(result.snapshot.tile, TilePosition::new(2, 0));
    }

    #[test]
    fn input_frame_with_encounter_rolls_after_player_moves() {
        let mut session = OverworldSession::new(
            map_with_blocks(2, 1, vec![0, 0]),
            grass_tileset(),
            TilePosition::new(0, 0),
        );
        let input = PlayerInputFrame::new(1, Frame(0), B_PAD_RIGHT).expect("input frame");
        let mut rng = Random::new(1);

        let result = session
            .apply_input_frame_with_encounter(
                &input,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect("field input encounter");

        assert_eq!(result.input.snapshot.tile, TilePosition::new(2, 0));
        let roll = result.wild_encounter.expect("wild encounter roll");
        assert_eq!(roll.tile, TilePosition::new(2, 0));
        assert_eq!(roll.encounter_roll, 64);
        assert_eq!(roll.slot_percent_roll, Some(88));
        assert_eq!(roll.rng_seed_after, rng.seed());
    }

    #[test]
    fn input_frame_with_state_encounter_ticks_repel_after_movement() {
        let mut session = OverworldSession::new(
            map_with_blocks(2, 1, vec![0, 0]),
            grass_tileset(),
            TilePosition::new(0, 0),
        );
        let mut state = GameState {
            repel_steps_remaining: 1,
            active_repel_item: Some("REPEL".to_string()),
            ..GameState::default()
        };
        let input = PlayerInputFrame::new(1, Frame(0), B_PAD_RIGHT).expect("input frame");
        let mut rng = Random::new(1);

        let result = session
            .apply_input_frame_with_state_encounter(
                &mut state,
                &input,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                Some(3),
                EncounterCheckOptions::default(),
            )
            .expect("field input encounter");

        assert_eq!(result.expired_repel_item, Some("REPEL".to_string()));
        assert_eq!(state.joypad.h_joypad_pressed, B_PAD_RIGHT);
        assert_eq!(state.joypad.h_joypad_down, B_PAD_RIGHT);
        assert_eq!(state.repel_steps_remaining, 0);
        assert_eq!(state.active_repel_item, None);
        assert_eq!(
            result.wild_encounter.expect("encounter roll").repelled_by,
            None
        );
    }

    #[test]
    fn input_frame_with_encounter_does_not_consume_rng_when_only_turning() {
        let mut session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        let input = PlayerInputFrame::new(1, Frame(0), B_PAD_RIGHT).expect("input frame");
        let mut rng = Random::new(1);

        let result = session
            .apply_input_frame_with_encounter(
                &input,
                StepOptions::default(),
                None,
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect("field input encounter");

        assert_eq!(result.wild_encounter, None);
        assert_eq!(rng.seed(), 1);
        assert_eq!(
            result.input.action,
            OverworldInputAction::Step(OverworldStepResult {
                outcome: StepOutcome::Turned {
                    facing: Direction::Right,
                },
                warp: None,
            })
        );
    }

    #[test]
    fn input_frame_jumps_ledge_before_regular_step() {
        let mut session = OverworldSession::new(
            map_with_blocks(1, 3, vec![0, 1, 0]),
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
                    over: TilePosition::new(0, 3),
                    to: TilePosition::new(0, 5),
                    speed_multiplier: 1,
                },
                warp: None,
            })
        );
        assert_eq!(result.snapshot.tile, TilePosition::new(0, 5));
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
            vec![object("ROUTE29_TEACHER1", 2, 1, "-1")],
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
                at: TilePosition::new(4, 2),
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
        assert_eq!(session.snapshot().tile, TilePosition::new(2, 0));
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
        let mut teacher = object("ROUTE29_TEACHER1", 1, 2, "-1");
        teacher.script = "Route29TeacherScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 3, vec![0; 6]),
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
                target_tile: TilePosition::new(2, 4),
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
            TilePosition::new(i16::MAX - 1, 0),
        );
        session.player.facing = Direction::Right;

        let error = session
            .check_interaction_checked(StepOptions::default().stride_tiles)
            .expect_err("checked interaction must reject overflowing runtime front tiles");

        assert_eq!(
            error,
            OverworldInputError::Coordinate(OverworldCoordinateError::RuntimeTileOverflow {
                x: i16::MAX - 1,
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
            .step_checked(Direction::Right, StepOptions::default())
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
        let mut first = object("FIRST_OBJECT", 1, 2, "-1");
        first.script = "FirstScript".to_string();
        let mut second = object("SECOND_OBJECT", 1, 2, "-1");
        second.script = "SecondScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 3, vec![0; 6]),
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
        let mut item = object("ROUTE29_POKE_BALL", 1, 2, "EVENT_ROUTE_29_POTION");
        item.object_type = "OBJECTTYPE_ITEMBALL".to_string();
        item.script = "Route29Potion".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(2, 3, vec![0; 6]),
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
            map_with_blocks(3, 1, vec![0, 1, 0]),
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

        assert_eq!(interaction.target_tile, TilePosition::new(4, 0));
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
    fn interaction_targets_background_events_by_scaled_event_tile() {
        let events = MapEvents {
            bg_events: vec![background_event(2, 1, "BGEVENT_READ", "SignpostScript")],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(3, 2, vec![0, 0, 0, 0, 0, 0]),
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
                target_tile: TilePosition::new(4, 2),
                script: "SignpostScript".to_string(),
                target: OverworldInteractionTarget::Background {
                    event_type: "BGEVENT_READ".to_string(),
                },
            }
        );
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
    fn trainer_sight_uses_scaled_object_tile_radius() {
        let mut trainer = object("ROUTE29_YOUNGSTER", 1, 0, "-1");
        trainer.radius = 2;
        trainer.sightline_direction_override = Some("DOWN".to_string());
        trainer.script = "Route29YoungsterScript".to_string();
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![trainer],
            tileset(),
            TilePosition::new(2, 4),
        );

        let sight = session
            .check_trainer_sight_checked()
            .expect("checked trainer sight")
            .expect("trainer sight");

        assert_eq!(
            sight,
            OverworldInteraction {
                map_name: "test".to_string(),
                player_tile: TilePosition::new(2, 4),
                facing: Direction::Down,
                target_tile: TilePosition::new(2, 0),
                script: "Route29YoungsterScript".to_string(),
                target: OverworldInteractionTarget::Object {
                    object_index: 1,
                    object_identifier: Some("ROUTE29_YOUNGSTER".to_string()),
                    object_type: "OBJECTTYPE_SCRIPT".to_string(),
                },
            }
        );
    }

    #[test]
    fn trainer_sight_checks_scaled_intermediate_tiles_for_blockers() {
        let mut trainer = object("ROUTE29_YOUNGSTER", 1, 0, "-1");
        trainer.radius = 2;
        trainer.sightline_direction_override = Some("DOWN".to_string());
        let blocker = object("ROUTE29_TEACHER", 1, 1, "-1");
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 4, vec![0; 12]),
            MapEvents::default(),
            vec![trainer, blocker],
            tileset(),
            TilePosition::new(2, 4),
        );

        assert_eq!(
            session
                .check_trainer_sight_checked()
                .expect("checked trainer sight"),
            None
        );
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
    fn coord_event_triggers_for_matching_scene_and_scaled_event_tile() {
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
    fn session_rolls_deterministic_grass_encounter_on_grass_tile() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        let mut rng = Random::new(1);

        let roll = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect("encounter roll")
            .expect("grass encounter check");

        assert_eq!(roll.map_name, "test");
        assert_eq!(roll.surface, EncounterSurface::Grass);
        assert_eq!(roll.threshold, 255);
        assert_eq!(roll.encounter_roll, 64);
        assert_eq!(roll.slot_percent_roll, Some(88));
        assert_eq!(
            roll.resolved.expect("resolved encounter").encounter.species,
            "PIDGEY"
        );
        assert_eq!(roll.rng_seed_after, rng.seed());
    }

    #[test]
    fn session_wild_encounter_accepts_odd_runtime_player_tile() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(1, 0));
        let mut rng = Random::new(1);

        let roll = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect("encounter roll")
            .expect("grass encounter check");

        assert_eq!(roll.map_name, "test");
        assert_eq!(roll.surface, EncounterSurface::Grass);
        assert_eq!(roll.rng_seed_after, rng.seed());
    }

    #[test]
    fn session_wild_encounter_rejects_out_of_bounds_player_tile_before_rng() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(4, 0));
        let mut rng = Random::new(1);

        let error = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect_err("encounter checks must reject runtime tiles outside the map");

        assert_eq!(
            error,
            EncounterError::RuntimeTileOutOfBounds {
                map_name: "test".to_string(),
                x: 4,
                y: 0,
                width: 4,
                height: 2,
            }
        );
        assert_eq!(rng.seed(), 1);
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
    fn session_wild_encounter_rejects_missing_collision_before_rng() {
        let session = OverworldSession::new(
            map(),
            TilesetCollision {
                metatiles: Vec::new(),
            },
            TilePosition::new(0, 0),
        );
        let mut rng = Random::new(1);

        let error = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect_err("encounter checks must reject missing collision data");

        assert_eq!(
            error,
            EncounterError::MissingRuntimeCollision {
                map_name: "test".to_string(),
                x: 0,
                y: 0,
            }
        );
        assert_eq!(rng.seed(), 1);
    }

    #[test]
    fn session_repel_suppresses_resolved_weaker_wild_encounter() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        let mut rng = Random::new(1);

        let roll = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions {
                    active_repel_item: Some("REPEL".to_string()),
                    lead_party_level: Some(3),
                    ..EncounterCheckOptions::default()
                },
            )
            .expect("encounter roll")
            .expect("grass encounter check");

        assert_eq!(roll.resolved, None);
        assert_eq!(roll.repelled_by, Some("REPEL".to_string()));
        assert_eq!(roll.encounter_roll, 64);
        assert_eq!(roll.slot_percent_roll, Some(88));
        assert_eq!(roll.rng_seed_after, rng.seed());
    }

    #[test]
    fn input_with_encounter_error_does_not_commit_session_or_rng() {
        let mut session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        session.player.facing = Direction::Right;
        let mut rng = Random::new(1);
        let encounters = WildEncounterData {
            map_name: "test".to_string(),
            grass_rates: None,
            water_rate: None,
            grass: None,
            water: None,
        };

        let error = session
            .apply_joypad_mask_with_encounter(
                B_PAD_RIGHT,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
                None,
                &encounters,
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect_err("missing encounter rate must reject input frame");

        assert!(matches!(
            error,
            OverworldInputTickError::Encounter(EncounterError::MissingEncounterRate {
                map_name,
                surface: EncounterSurface::Grass,
            }) if map_name == "test"
        ));
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
        assert_eq!(rng.seed(), 1);
    }

    #[test]
    fn session_active_repel_requires_explicit_lead_party_level() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        let mut rng = Random::new(1);

        let error = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions {
                    active_repel_item: Some("REPEL".to_string()),
                    ..EncounterCheckOptions::default()
                },
            )
            .expect_err("active repel must not infer lead level");

        assert_eq!(
            error,
            EncounterError::ActiveRepelMissingLeadLevel {
                item_id: "REPEL".to_string(),
            }
        );
    }

    #[test]
    fn session_skips_wild_encounter_check_on_floor_tile() {
        let session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));
        let mut rng = Random::new(1);

        let roll = session
            .check_wild_encounter(
                &encounter_data(),
                &encounter_slot_tables(),
                &encounter_music_modifiers(),
                &mut rng,
                EncounterCheckOptions::default(),
            )
            .expect("encounter roll");

        assert_eq!(roll, None);
        assert_eq!(rng.seed(), 1);
    }

    #[test]
    fn session_reports_warp_trigger_from_map_events() {
        let warp = WarpEvent {
            index: 1,
            x: 1,
            y: 1,
            target_map_constant: "TARGET_MAP".to_string(),
            target_map: "TargetMap".to_string(),
            target_warp_id: 2,
        };
        let events = MapEvents {
            warps: vec![warp.clone()],
            ..MapEvents::default()
        };
        let mut session =
            OverworldSession::with_events(map(), events, tileset(), TilePosition::new(0, 2));

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
                to: TilePosition::new(2, 2),
                speed_multiplier: 1,
            }
        );
        assert_eq!(
            result.warp,
            Some(WarpTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(2, 2),
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
    fn checked_ledge_warp_rejects_invalid_warp_coordinates_without_committing_jump() {
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

        let error = session
            .ledge_jump_and_check_warp_checked(
                Direction::Down,
                StepOptions {
                    force_step_after_turn: true,
                    ..StepOptions::default()
                },
            )
            .expect_err("invalid warp coordinate must reject checked ledge jump");

        assert_eq!(
            error,
            OverworldCoordinateError::Event(OverworldEventCoordinateError::WarpOutOfRange {
                index: 8,
                x: u16::MAX,
                y: 1,
            })
        );
        assert_eq!(session.player.tile, TilePosition::new(0, 0));
        assert_eq!(session.frame, 0);
    }

    #[test]
    fn warp_transition_builds_destination_session_without_loading_fallbacks() {
        let trigger = WarpTrigger {
            map_name: "source".to_string(),
            tile: TilePosition::new(1, 1),
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
