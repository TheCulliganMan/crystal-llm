use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::map::{BackgroundEvent, CoordEvent, MapConnection, MapEvents, ObjectEvent, WarpEvent};
use crate::multiplayer::{OverworldPresence, PresenceEntityType, fnv1a32};
use crate::random::Random;
use crate::state::EventFlagMemory;

use super::collision::{
    Terrain, TilesetCollision, describe_collision, permissions, sample_collision,
};
use super::encounters::{
    EncounterError, EncounterSurface, ResolvedWildEncounter, TimeOfDay, WildEncounterData,
    encounter_threshold, passes_encounter_roll, select_wild_encounter,
};
use super::map::{Direction, OverworldMapData, TilePosition};
use super::movement::{
    LedgeJumpOutcome, MovementMode, OccupiedTile, PlayerMovementState, StepOptions, StepOutcome,
    attempt_ledge_jump_with_occupied_tiles, attempt_step_with_occupied_tiles, move_by_stride,
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
#[serde(deny_unknown_fields)]
pub struct OverworldLedgeJumpResult {
    pub outcome: LedgeJumpOutcome,
    pub warp: Option<WarpTrigger>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
}

impl Default for EncounterCheckOptions {
    fn default() -> Self {
        Self {
            time: TimeOfDay::Day,
            music_token: None,
            has_cleanse_tag: false,
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
    pub rng_seed_after: u32,
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
    ) -> OverworldPresence {
        OverworldPresence {
            user_id: user_id.into(),
            player_name: player_name.into(),
            entity_type: PresenceEntityType::Player,
            map_name: self.map.name.clone(),
            tile: self.player.tile,
            direction: self.player.facing,
            updated_at_ms,
        }
    }

    pub fn step(&mut self, direction: Direction, options: StepOptions) -> StepOutcome {
        let occupied_tiles = self.occupied_tiles();
        let outcome = attempt_step_with_occupied_tiles(
            &mut self.player,
            direction,
            &self.map,
            &self.tileset,
            options,
            &occupied_tiles,
        );
        self.frame += 1;
        outcome
    }

    pub fn ledge_jump(&mut self, direction: Direction, options: StepOptions) -> LedgeJumpOutcome {
        let occupied_tiles = self.occupied_tiles();
        let outcome = attempt_ledge_jump_with_occupied_tiles(
            &mut self.player,
            direction,
            &self.map,
            &self.tileset,
            options,
            &occupied_tiles,
        );
        self.frame += 1;
        outcome
    }

    pub fn occupied_tiles(&self) -> Vec<OccupiedTile> {
        self.objects
            .iter()
            .filter(|object| self.is_object_visible(object))
            .map(|object| OccupiedTile {
                tile: object_tile_position(object),
                object_identifier: object.object_identifier.clone(),
            })
            .collect()
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

    pub fn check_interaction(&self, stride_tiles: i16) -> Option<OverworldInteraction> {
        let stride = stride_tiles.max(1);
        let facing_tile = move_by_stride(self.player.tile, self.player.facing, stride);
        if facing_tile.x < 0 || facing_tile.y < 0 {
            return None;
        }

        if let Some((object_index, object)) = self.visible_object_at(facing_tile) {
            return Some(self.object_interaction(object_index, object, facing_tile));
        }

        let adjusted_tile = self.counter_adjusted_tile(facing_tile, stride);
        if adjusted_tile != facing_tile {
            if let Some((object_index, object)) = self.visible_object_at(adjusted_tile) {
                return Some(self.object_interaction(object_index, object, adjusted_tile));
            }
        }

        self.background_event_at(adjusted_tile, stride)
            .map(|event| OverworldInteraction {
                map_name: self.map.name.clone(),
                player_tile: self.player.tile,
                facing: self.player.facing,
                target_tile: adjusted_tile,
                script: event.script.clone(),
                target: OverworldInteractionTarget::Background {
                    event_type: event.event_type.clone(),
                },
            })
    }

    pub fn check_coord_event(
        &self,
        current_scene: Option<&str>,
        stride_tiles: i16,
    ) -> Option<CoordEventTrigger> {
        let stride = stride_tiles.max(1);
        let offset = stride - 1;
        self.map_events
            .coord_events
            .iter()
            .find(|event| {
                let scene_matches = if event.scene_id.is_empty() {
                    true
                } else {
                    current_scene
                        .map(|scene| scene == event.scene_id)
                        .unwrap_or(false)
                };
                scene_matches
                    && coord_event_tile_position(event, stride, offset) == self.player.tile
            })
            .map(|event| CoordEventTrigger {
                map_name: self.map.name.clone(),
                tile: self.player.tile,
                scene_id: event.scene_id.clone(),
                script_name: event.script_name.clone(),
            })
    }

    pub fn visible_object_at(&self, tile: TilePosition) -> Option<(u16, &ObjectEvent)> {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, object)| self.is_object_visible(object))
            .find(|(_, object)| object_tile_position(object) == tile)
            .map(|(index, object)| ((index + 1) as u16, object))
    }

    pub fn background_event_at(
        &self,
        tile: TilePosition,
        stride_tiles: i16,
    ) -> Option<&BackgroundEvent> {
        let stride = stride_tiles.max(1);
        let offset = stride - 1;
        self.map_events.bg_events.iter().find(|event| {
            let event_tile = background_event_tile_position(event, stride, offset);
            event_tile == tile
        })
    }

    pub fn counter_adjusted_tile(&self, tile: TilePosition, stride_tiles: i16) -> TilePosition {
        let stride = stride_tiles.max(1);
        let delta_x = tile.x - self.player.tile.x;
        let delta_y = tile.y - self.player.tile.y;
        if delta_x == 0 && delta_y == 0 {
            return tile;
        }

        let mut candidates = Vec::new();
        if delta_x == 0 && delta_y != 0 {
            let front_y = self.player.tile.y + delta_y;
            for offset in 0..=stride {
                candidates.push(TilePosition::new(self.player.tile.x - offset, front_y));
            }
        } else if delta_y == 0 && delta_x != 0 {
            let front_x = self.player.tile.x + delta_x;
            for offset in 0..=stride {
                candidates.push(TilePosition::new(front_x, self.player.tile.y - offset));
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
            .map(|counter| TilePosition::new(counter.x + delta_x, counter.y + delta_y))
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

    pub fn check_warp(&self) -> Option<WarpTrigger> {
        self.map_events
            .warps
            .iter()
            .find(|warp| warp_tile_position(warp) == self.player.tile)
            .cloned()
            .map(|warp| WarpTrigger {
                map_name: self.map.name.clone(),
                tile: self.player.tile,
                warp,
            })
    }

    pub fn check_connection(&self) -> Option<ConnectionTrigger> {
        connection_for_tile(&self.map, self.player.tile)
    }

    pub fn step_and_check_warp(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> OverworldStepResult {
        let outcome = self.step(direction, options);
        let warp = self.check_warp();
        OverworldStepResult { outcome, warp }
    }

    pub fn ledge_jump_and_check_warp(
        &mut self,
        direction: Direction,
        options: StepOptions,
    ) -> OverworldLedgeJumpResult {
        let outcome = self.ledge_jump(direction, options);
        let warp = self.check_warp();
        OverworldLedgeJumpResult { outcome, warp }
    }

    pub fn check_wild_encounter(
        &self,
        encounters: &WildEncounterData,
        rng: &mut Random,
        options: EncounterCheckOptions,
    ) -> Result<Option<WildEncounterRoll>, EncounterError> {
        let Some(surface) = encounter_surface_for_player_tile(self) else {
            return Ok(None);
        };
        let threshold = encounter_threshold(
            encounters,
            surface,
            options.time,
            options.music_token.as_deref(),
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
                rng_seed_after: rng.seed(),
            }));
        }

        let slot_percent_roll = next_percent_roll(rng);
        let level_roll = rng.randrange(256) as u8;
        let resolved = select_wild_encounter(
            encounters,
            surface,
            options.time,
            slot_percent_roll,
            level_roll,
        )?;
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
            rng_seed_after: rng.seed(),
        }))
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
    let sample = sample_collision(&session.map, &session.tileset, session.player.tile)?;
    if sample.permission == permissions::TALL_GRASS {
        return Some(EncounterSurface::Grass);
    }
    let attributes = describe_collision(sample.permission);
    if session.player.mode == MovementMode::Surf && attributes.terrain == Terrain::Water {
        return Some(EncounterSurface::Water);
    }
    None
}

fn next_percent_roll(rng: &mut Random) -> u8 {
    loop {
        let value = rng.randrange(256) as u8;
        if value < 100 {
            return value + 1;
        }
    }
}

pub fn warp_tile_position(warp: &WarpEvent) -> TilePosition {
    TilePosition::new(warp.x as i16, warp.y as i16)
}

pub fn connection_for_tile(
    map: &OverworldMapData,
    tile: TilePosition,
) -> Option<ConnectionTrigger> {
    let (width, height) = map.tile_bounds();
    let width = width as i16;
    let height = height as i16;
    map.connections()
        .iter()
        .find(|connection| match connection.direction.as_str() {
            "north" => tile.y < 0,
            "south" => tile.y >= height,
            "west" => tile.x < 0,
            "east" => tile.x >= width,
            _ => false,
        })
        .cloned()
        .map(|connection| ConnectionTrigger {
            map_name: map.name.clone(),
            tile,
            connection,
        })
}

impl WarpTransition {
    pub fn apply_to(
        &self,
        map: OverworldMapData,
        map_events: MapEvents,
        objects: Vec<ObjectEvent>,
        tileset: TilesetCollision,
        frame: u64,
    ) -> OverworldSession {
        OverworldSession {
            frame,
            map,
            map_events,
            objects,
            object_facings: BTreeMap::new(),
            following: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            tileset,
            player: PlayerMovementState::new(self.destination.tile),
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
    ) -> OverworldSession {
        OverworldSession {
            frame,
            map,
            map_events,
            objects,
            object_facings: BTreeMap::new(),
            following: None,
            last_talked_object_identifier: None,
            player_hidden: false,
            hidden_event_flags: BTreeSet::new(),
            hidden_object_identifiers: BTreeSet::new(),
            tileset,
            player: PlayerMovementState::new(self.destination.tile),
        }
    }
}

pub fn object_tile_position(object: &ObjectEvent) -> TilePosition {
    TilePosition::new(object.x as i16, object.y as i16)
}

pub fn background_event_tile_position(
    event: &BackgroundEvent,
    stride_tiles: i16,
    offset_tiles: i16,
) -> TilePosition {
    TilePosition::new(
        event.x as i16 * stride_tiles + offset_tiles,
        event.y as i16 * stride_tiles + offset_tiles,
    )
}

pub fn coord_event_tile_position(
    event: &CoordEvent,
    stride_tiles: i16,
    offset_tiles: i16,
) -> TilePosition {
    TilePosition::new(
        event.x as i16 * stride_tiles + offset_tiles,
        event.y as i16 * stride_tiles + offset_tiles,
    )
}

pub fn is_counter_permission(permission: u8) -> bool {
    permission == permissions::COUNTER || permission == permissions::COUNTER_98
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{BackgroundEvent, CoordEvent, MapAttributes, ObjectEvent};
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
            session.presence("u1", "Chris", 123).map_name,
            "test".to_string()
        );
    }

    #[test]
    fn session_blocks_steps_into_visible_objects() {
        let mut session = OverworldSession::with_events_and_objects(
            map(),
            MapEvents::default(),
            vec![object("ROUTE29_TEACHER1", 2, 0, "-1")],
            tileset(),
            TilePosition::new(0, 0),
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
                at: TilePosition::new(2, 0),
                facing: Direction::Right,
                object_identifier: Some("ROUTE29_TEACHER1".to_string()),
            }
        );
        assert_eq!(session.snapshot().tile, TilePosition::new(0, 0));
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
            vec![object("ROUTE29_POKE_BALL", 2, 0, "EVENT_ROUTE_29_POTION")],
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
            vec![object("ROUTE29_POKE_BALL", 2, 0, "EVENT_ROUTE_29_POTION")],
            tileset(),
            TilePosition::new(0, 0),
        )
        .with_event_flag_memory(&flags);

        assert_eq!(session.occupied_tiles().len(), 1);
    }

    #[test]
    fn interaction_targets_visible_object_on_facing_tile() {
        let mut teacher = object("ROUTE29_TEACHER1", 0, 2, "-1");
        teacher.script = "Route29TeacherScript".to_string();
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(1, 2, vec![0, 0]),
            MapEvents::default(),
            vec![teacher],
            tileset(),
            TilePosition::new(0, 0),
        );

        let interaction = session.check_interaction(2).expect("object interaction");

        assert_eq!(
            interaction,
            OverworldInteraction {
                map_name: "test".to_string(),
                player_tile: TilePosition::new(0, 0),
                facing: Direction::Down,
                target_tile: TilePosition::new(0, 2),
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
    fn interaction_uses_lowest_visible_object_slot_on_shared_tile() {
        let mut first = object("FIRST_OBJECT", 0, 2, "-1");
        first.script = "FirstScript".to_string();
        let mut second = object("SECOND_OBJECT", 0, 2, "-1");
        second.script = "SecondScript".to_string();
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(1, 2, vec![0, 0]),
            MapEvents::default(),
            vec![first, second],
            tileset(),
            TilePosition::new(0, 0),
        );

        let interaction = session.check_interaction(2).expect("object interaction");

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
    fn interaction_does_not_hide_objects_with_case_changed_event_flags() {
        let hidden_flags = BTreeSet::from(["event_route_29_potion".to_string()]);
        let mut item = object("ROUTE29_POKE_BALL", 0, 2, "EVENT_ROUTE_29_POTION");
        item.object_type = "OBJECTTYPE_ITEMBALL".to_string();
        item.script = "Route29Potion".to_string();
        let session = OverworldSession::with_events_and_objects(
            map_with_blocks(1, 2, vec![0, 0]),
            MapEvents::default(),
            vec![item],
            tileset(),
            TilePosition::new(0, 0),
        )
        .with_hidden_event_flags(hidden_flags);

        let interaction = session.check_interaction(2).expect("exact flag visible");

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
        let mut clerk = object("MART_CLERK", 4, 0, "-1");
        clerk.script = "MartClerkScript".to_string();
        let mut session = OverworldSession::with_events_and_objects(
            map_with_blocks(3, 1, vec![0, 1, 0]),
            MapEvents::default(),
            vec![clerk],
            counter_tileset(),
            TilePosition::new(0, 0),
        );
        session.player.facing = Direction::Right;

        let interaction = session.check_interaction(2).expect("counter object");

        assert_eq!(interaction.target_tile, TilePosition::new(4, 0));
        assert_eq!(interaction.script, "MartClerkScript");
    }

    #[test]
    fn interaction_targets_background_events_by_stride_adjusted_tile() {
        let events = MapEvents {
            bg_events: vec![background_event(1, 0, "BGEVENT_READ", "SignpostScript")],
            ..MapEvents::default()
        };
        let mut session = OverworldSession::with_events(
            map_with_blocks(3, 2, vec![0, 0, 0, 0, 0, 0]),
            events,
            tileset(),
            TilePosition::new(1, 1),
        );
        session.player.facing = Direction::Right;

        let interaction = session.check_interaction(2).expect("background event");

        assert_eq!(
            interaction,
            OverworldInteraction {
                map_name: "test".to_string(),
                player_tile: TilePosition::new(1, 1),
                facing: Direction::Right,
                target_tile: TilePosition::new(3, 1),
                script: "SignpostScript".to_string(),
                target: OverworldInteractionTarget::Background {
                    event_type: "BGEVENT_READ".to_string(),
                },
            }
        );
    }

    #[test]
    fn coord_event_triggers_for_matching_scene_and_stride_adjusted_tile() {
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
            TilePosition::new(3, 1),
        );

        assert_eq!(
            session.check_coord_event(Some("SCENE_ROUTE29_CATCHING_TUTORIAL"), 2),
            Some(CoordEventTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(3, 1),
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
            TilePosition::new(3, 1),
        );

        assert_eq!(
            session.check_coord_event(Some("scene_route29_catching_tutorial"), 2),
            None
        );
        assert_eq!(session.check_coord_event(None, 2), None);
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
            TilePosition::new(1, 3),
        );

        assert_eq!(
            session.check_coord_event(None, 2),
            Some(CoordEventTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(1, 3),
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
            TilePosition::new(1, 3),
        );

        assert_eq!(
            session
                .check_coord_event(Some("ANY_SCENE"), 2)
                .expect("coord event")
                .script_name,
            "FirstScript"
        );
    }

    #[test]
    fn session_rolls_deterministic_grass_encounter_on_grass_tile() {
        let session = OverworldSession::new(map(), grass_tileset(), TilePosition::new(0, 0));
        let mut rng = Random::new(1);

        let roll = session
            .check_wild_encounter(
                &encounter_data(),
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
    fn session_skips_wild_encounter_check_on_floor_tile() {
        let session = OverworldSession::new(map(), tileset(), TilePosition::new(0, 0));
        let mut rng = Random::new(1);

        let roll = session
            .check_wild_encounter(
                &encounter_data(),
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
            x: 3,
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
            OverworldSession::with_events(map(), events, tileset(), TilePosition::new(1, 1));

        assert_eq!(session.check_warp(), None);

        let result = session.step_and_check_warp(
            Direction::Right,
            StepOptions {
                force_step_after_turn: true,
                ..StepOptions::default()
            },
        );

        assert_eq!(
            result.outcome,
            StepOutcome::Moved {
                from: TilePosition::new(1, 1),
                to: TilePosition::new(3, 1),
                speed_multiplier: 1,
            }
        );
        assert_eq!(
            result.warp,
            Some(WarpTrigger {
                map_name: "test".to_string(),
                tile: TilePosition::new(3, 1),
                warp,
            })
        );
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
                tile: TilePosition::new(5, 7),
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
        );

        assert_eq!(session.frame, 42);
        assert_eq!(session.player.tile, TilePosition::new(5, 7));
        assert_eq!(session.objects, destination_objects);
    }

    #[test]
    fn session_reports_connection_trigger_when_player_crosses_declared_boundary() {
        let session =
            OverworldSession::new(map_with_connections(), tileset(), TilePosition::new(4, 1));

        let trigger = session.check_connection().expect("east connection trigger");

        assert_eq!(trigger.map_name, "test");
        assert_eq!(trigger.tile, TilePosition::new(4, 1));
        assert_eq!(trigger.connection.target_map, "next");
        assert_eq!(trigger.connection.direction, "east");
    }

    #[test]
    fn connection_transition_builds_destination_session_without_loading_fallbacks() {
        let transition = ConnectionTransition {
            trigger: ConnectionTrigger {
                map_name: "source".to_string(),
                tile: TilePosition::new(4, 1),
                connection: MapConnection {
                    direction: "east".to_string(),
                    target_map: "target".to_string(),
                    offset: 0,
                },
            },
            destination: ConnectionDestination {
                map_name: "target".to_string(),
                tile: TilePosition::new(1, 1),
            },
        };

        let destination_objects = vec![object("CONNECTION_NPC", 1, 1, "-1")];
        let session = transition.apply_to(
            map(),
            MapEvents::default(),
            destination_objects.clone(),
            tileset(),
            77,
        );

        assert_eq!(session.frame, 77);
        assert_eq!(session.player.tile, TilePosition::new(1, 1));
        assert_eq!(session.objects, destination_objects);
    }
}
