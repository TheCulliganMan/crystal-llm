use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::map::MapSceneTable;
use crate::state::{
    GameState, OverworldFollowMemory, OverworldMemory, OverworldObjectMapMemory,
    OverworldObjectMemory, SceneError,
};
use crate::world::map::TilePosition;
use crate::world::session::{
    OverworldFollowState, OverworldSession, OverworldSnapshot,
    raw_event_tile_to_runtime_tile_checked,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapContextOutcome {
    pub map_name: String,
    pub current_music: Option<String>,
    pub scene_id: Option<String>,
    pub scene_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum SpawnMemoryUpdate {
    Preserve,
    Set(u16),
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MapContextError {
    #[error("map context scene error: {0:?}")]
    Scene(#[from] SceneError),
    #[error("saved block override x coordinate {x} is out of range for map {map_name}")]
    BlockOverrideXOutOfRange { map_name: String, x: u32 },
    #[error("saved block override y coordinate {y} is out of range for map {map_name}")]
    BlockOverrideYOutOfRange { map_name: String, y: u32 },
    #[error("saved block override ({x}, {y}) is outside map {map_name}")]
    BlockOverrideOutsideMap { map_name: String, x: u32, y: u32 },
    #[error("compiled map {map_name} runtime tile bounds overflow supported coordinates")]
    MapBoundsOverflow { map_name: String },
    #[error("saved object override references missing object {object_id} on map {map_name}")]
    MissingObjectOverride { map_name: String, object_id: String },
    #[error(
        "saved object override {object_id} on map {map_name} has out-of-range raw event coordinate ({x}, {y})"
    )]
    ObjectOverrideCoordinatesOutOfRange {
        map_name: String,
        object_id: String,
        x: u16,
        y: u16,
    },
    #[error(
        "saved object override {object_id} on map {map_name} raw event coordinate resolves outside runtime tile bounds {width}x{height} at runtime tile ({x}, {y})"
    )]
    ObjectOverrideOutsideMap {
        map_name: String,
        object_id: String,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    },
}

pub fn apply_map_context(
    state: &mut GameState,
    map_name: &str,
    current_music: Option<String>,
    scenes: &MapSceneTable,
) -> Result<MapContextOutcome, MapContextError> {
    apply_map_music_context(state, current_music.clone());
    let scene_status = apply_map_scene_context(state, map_name, scenes)?;

    Ok(MapContextOutcome {
        map_name: map_name.to_string(),
        current_music,
        scene_id: scene_status
            .as_ref()
            .map(|status| status.scene_name.clone()),
        scene_index: scene_status.as_ref().map(|status| status.scene_index),
    })
}

pub fn apply_map_music_context(state: &mut GameState, current_music: Option<String>) {
    state.script_runtime.current_music = current_music;
    state.script_runtime.pending_music_fade = None;
}

pub fn apply_map_scene_context(
    state: &mut GameState,
    map_name: &str,
    scenes: &MapSceneTable,
) -> Result<Option<crate::state::SceneStatus>, MapContextError> {
    if scenes.scenes.is_empty() {
        return Ok(None);
    }
    Ok(Some(state.scenes.enter_map(map_name, scenes)?))
}

pub fn commit_overworld_snapshot(
    state: &mut GameState,
    snapshot: &OverworldSnapshot,
    spawn_update: SpawnMemoryUpdate,
) {
    state.overworld = crate::state::OverworldMemory::from_snapshot(snapshot);
    state.frame_counter = snapshot.frame;
    match spawn_update {
        SpawnMemoryUpdate::Preserve => {}
        SpawnMemoryUpdate::Set(identifier) => {
            state.last_spawn_identifier = Some(identifier);
        }
        SpawnMemoryUpdate::Clear => {
            state.last_spawn_identifier = None;
        }
    }
}

pub fn apply_state_block_overrides(
    overworld: &mut OverworldSession,
    state: &GameState,
) -> Result<(), MapContextError> {
    let Some(overrides) = state.map_block_overrides.get(&overworld.map.name) else {
        return Ok(());
    };
    let mut validated_overrides = Vec::with_capacity(overrides.len());
    for ((metatile_x, metatile_y), block_id) in overrides {
        let x =
            i16::try_from(*metatile_x).map_err(|_| MapContextError::BlockOverrideXOutOfRange {
                map_name: overworld.map.name.clone(),
                x: u32::from(*metatile_x),
            })?;
        let y =
            i16::try_from(*metatile_y).map_err(|_| MapContextError::BlockOverrideYOutOfRange {
                map_name: overworld.map.name.clone(),
                y: u32::from(*metatile_y),
            })?;
        let index = overworld.map.metatile_index(x, y).ok_or_else(|| {
            MapContextError::BlockOverrideOutsideMap {
                map_name: overworld.map.name.clone(),
                x: u32::from(*metatile_x),
                y: u32::from(*metatile_y),
            }
        })?;
        validated_overrides.push((index, *block_id));
    }
    for (index, block_id) in validated_overrides {
        overworld.map.metatile_ids[index] = block_id;
    }
    Ok(())
}

pub fn apply_state_object_overrides(
    overworld: &mut OverworldSession,
    state: &GameState,
) -> Result<(), MapContextError> {
    let Some(memory) = state.map_object_overrides.get(&overworld.map.name) else {
        return Ok(());
    };
    let (width, height) =
        overworld
            .map
            .checked_tile_bounds()
            .ok_or_else(|| MapContextError::MapBoundsOverflow {
                map_name: overworld.map.name.clone(),
            })?;
    for (object_id, object_memory) in &memory.objects {
        overworld
            .objects
            .iter()
            .find(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .ok_or_else(|| MapContextError::MissingObjectOverride {
                map_name: overworld.map.name.clone(),
                object_id: object_id.clone(),
            })?;
        saved_object_raw_tile_to_runtime_tile(
            &overworld.map.name,
            object_id,
            object_memory.x,
            object_memory.y,
            width,
            height,
        )?;
    }
    for (object_id, object_memory) in &memory.objects {
        let object = overworld
            .objects
            .iter_mut()
            .find(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .expect("validated object override target must remain present");
        object.x = object_memory.x;
        object.y = object_memory.y;
        if let Some(tile) = object_memory.tile {
            overworld
                .object_runtime_tiles
                .insert(object_id.clone(), tile);
        } else {
            let tile = saved_object_raw_tile_to_runtime_tile(
                &overworld.map.name,
                object_id,
                object_memory.x,
                object_memory.y,
                width,
                height,
            )?;
            overworld
                .object_runtime_tiles
                .insert(object_id.clone(), tile);
        }
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

pub fn sync_state_object_overrides(
    state: &mut GameState,
    overworld: &OverworldSession,
) -> Result<(), MapContextError> {
    let (width, height) =
        overworld
            .map
            .checked_tile_bounds()
            .ok_or_else(|| MapContextError::MapBoundsOverflow {
                map_name: overworld.map.name.clone(),
            })?;
    let mut objects = BTreeMap::new();
    for object in &overworld.objects {
        let Some(object_id) = object.object_identifier.as_ref() else {
            continue;
        };
        let raw_tile = saved_object_raw_tile_to_runtime_tile(
            &overworld.map.name,
            object_id,
            object.x,
            object.y,
            width,
            height,
        )?;
        let runtime_tile = overworld
            .object_runtime_tiles
            .get(object_id)
            .copied()
            .unwrap_or(raw_tile);
        objects.insert(
            object_id.clone(),
            OverworldObjectMemory {
                x: object.x,
                y: object.y,
                tile: Some(runtime_tile),
                facing: overworld.object_facings.get(object_id).copied(),
            },
        );
    }
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
    Ok(())
}

fn saved_object_raw_tile_to_runtime_tile(
    map_name: &str,
    object_id: &str,
    raw_x: u16,
    raw_y: u16,
    runtime_width: u16,
    runtime_height: u16,
) -> Result<TilePosition, MapContextError> {
    let tile = raw_event_tile_to_runtime_tile_checked(raw_x, raw_y).ok_or_else(|| {
        MapContextError::ObjectOverrideCoordinatesOutOfRange {
            map_name: map_name.to_string(),
            object_id: object_id.to_string(),
            x: raw_x,
            y: raw_y,
        }
    })?;
    if tile.x < 0
        || tile.y < 0
        || i32::from(tile.x) >= i32::from(runtime_width)
        || i32::from(tile.y) >= i32::from(runtime_height)
    {
        return Err(MapContextError::ObjectOverrideOutsideMap {
            map_name: map_name.to_string(),
            object_id: object_id.to_string(),
            x: tile.x,
            y: tile.y,
            width: runtime_width,
            height: runtime_height,
        });
    }
    Ok(tile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    use crate::map::{MapAttributes, MapScene, ObjectEvent};
    use crate::world::collision::{MetatileCollision, TilesetCollision, permissions};
    use crate::world::map::{Direction, OverworldMapData, TilePosition};
    use crate::world::session::object_tile_position_checked;

    fn test_map() -> OverworldMapData {
        OverworldMapData::from_attributes(
            "Route29",
            &MapAttributes {
                tileset_name: "Overworld".to_string(),
                border_block: 0,
                width: 3,
                height: 2,
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
            vec![1, 2, 3, 4, 5, 6],
        )
    }

    fn overflowing_test_map() -> OverworldMapData {
        let mut map = test_map();
        map.width = u16::MAX;
        map
    }

    fn test_tileset() -> TilesetCollision {
        TilesetCollision {
            metatiles: vec![MetatileCollision {
                collision: [permissions::FLOOR; 4],
            }],
        }
    }

    fn test_object(object_id: &str, x: u16, y: u16) -> ObjectEvent {
        ObjectEvent {
            sprite: "SPRITE_YOUNGSTER".to_string(),
            x,
            y,
            spritemovedata: "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
            move_range_x: 0,
            move_range_y: 0,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: "YoungsterScript".to_string(),
            label: None,
            event_flag: "EVENT_YOUNGSTER".to_string(),
            object_identifier: Some(object_id.to_string()),
            sightline_direction_override: None,
        }
    }

    #[test]
    fn map_context_sets_music_clears_fade_and_enters_declared_scene() {
        let mut state = GameState::default();
        state.script_runtime.pending_music_fade = Some(crate::state::ScriptMusicFade {
            audio_id: "MUSIC_OLD".to_string(),
            fade_frames: 8,
            source_script: "MapContextTest".to_string(),
            command_index: 1,
        });
        let scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_START".to_string(),
                script_name: Some("SceneScript".to_string()),
            }],
        };

        let outcome = apply_map_context(
            &mut state,
            "Route29",
            Some("MUSIC_ROUTE_29".to_string()),
            &scenes,
        )
        .expect("map context");

        assert_eq!(outcome.current_music.as_deref(), Some("MUSIC_ROUTE_29"));
        assert_eq!(outcome.scene_id.as_deref(), Some("SCENE_START"));
        assert_eq!(outcome.scene_index, Some(0));
        assert_eq!(
            state.script_runtime.current_music.as_deref(),
            Some("MUSIC_ROUTE_29")
        );
        assert_eq!(state.script_runtime.pending_music_fade, None);
        assert_eq!(state.scenes.current_map_name, "Route29");
        assert_eq!(state.scenes.scene_name, "SCENE_START");
    }

    #[test]
    fn map_context_allows_maps_without_scenes() {
        let mut state = GameState::default();

        let outcome = apply_map_context(&mut state, "Route29", None, &MapSceneTable::default())
            .expect("scene-less map context");

        assert_eq!(outcome.scene_id, None);
        assert_eq!(state.script_runtime.current_music, None);
        assert_eq!(state.scenes.current_map_name, "");
    }

    #[test]
    fn scene_context_does_not_change_current_music_or_pending_fade() {
        let mut state = GameState::default();
        state.script_runtime.current_music = Some("MUSIC_CURRENT".to_string());
        state.script_runtime.pending_music_fade = Some(crate::state::ScriptMusicFade {
            audio_id: "MUSIC_NEXT".to_string(),
            fade_frames: 4,
            source_script: "MapContextTest".to_string(),
            command_index: 1,
        });
        let scenes = MapSceneTable {
            scenes: vec![MapScene {
                scene_id: "SCENE_START".to_string(),
                script_name: Some("SceneScript".to_string()),
            }],
        };

        let status =
            apply_map_scene_context(&mut state, "Route29", &scenes).expect("scene context");

        assert_eq!(status.unwrap().scene_name, "SCENE_START");
        assert_eq!(
            state.script_runtime.current_music.as_deref(),
            Some("MUSIC_CURRENT")
        );
        assert_eq!(
            state
                .script_runtime
                .pending_music_fade
                .as_ref()
                .map(|fade| fade.audio_id.as_str()),
            Some("MUSIC_NEXT")
        );
    }

    #[test]
    fn commit_overworld_snapshot_updates_frame_map_and_explicit_spawn_memory() {
        let mut state = GameState::default();
        state.last_spawn_identifier = Some(21);
        let snapshot = OverworldSnapshot {
            frame: 42,
            map_name: "Route29".to_string(),
            tile: crate::world::map::TilePosition::new(4, 5),
            facing: crate::world::map::Direction::Left,
            mode: crate::world::movement::MovementMode::Normal,
        };

        commit_overworld_snapshot(&mut state, &snapshot, SpawnMemoryUpdate::Preserve);
        assert_eq!(state.frame_counter, 42);
        assert_eq!(state.last_spawn_identifier, Some(21));
        assert_eq!(state.overworld.snapshot_identity().unwrap().0, "Route29");

        commit_overworld_snapshot(&mut state, &snapshot, SpawnMemoryUpdate::Set(14));
        assert_eq!(state.last_spawn_identifier, Some(14));

        commit_overworld_snapshot(&mut state, &snapshot, SpawnMemoryUpdate::Clear);
        assert_eq!(state.last_spawn_identifier, None);
    }

    #[test]
    fn saved_overworld_overrides_keep_block_and_event_coordinate_spaces_distinct() {
        let mut state = GameState::default();
        state
            .map_block_overrides
            .insert("Route29".to_string(), BTreeMap::from([((1, 0), 0x22)]));
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "YOUNGSTER".to_string(),
                    OverworldObjectMemory {
                        x: 1,
                        y: 0,
                        tile: Some(TilePosition::new(1, 0)),
                        facing: Some(Direction::Left),
                    },
                )]),
                hidden_object_identifiers: BTreeSet::from(["HIDDEN_NPC".to_string()]),
                following: None,
                last_talked_object_identifier: Some("YOUNGSTER".to_string()),
                player_hidden: true,
            },
        );
        let mut overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("YOUNGSTER", 4, 1)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        apply_state_block_overrides(&mut overworld, &state).expect("block overrides apply");
        apply_state_object_overrides(&mut overworld, &state).expect("object overrides apply");

        assert_eq!(overworld.map.metatile_at(1, 0), Some(0x22));
        let object = overworld
            .objects
            .iter()
            .find(|object| object.object_identifier.as_deref() == Some("YOUNGSTER"))
            .expect("object remains present");
        assert_eq!(
            object_tile_position_checked(object).expect("valid object coordinate"),
            TilePosition::new(1, 0)
        );
        assert_eq!(
            overworld.object_runtime_tiles.get("YOUNGSTER").copied(),
            Some(TilePosition::new(1, 0))
        );
        assert_eq!(
            overworld.object_facings.get("YOUNGSTER"),
            Some(&Direction::Left)
        );
        assert!(overworld.hidden_object_identifiers.contains("HIDDEN_NPC"));
        assert_eq!(
            overworld.last_talked_object_identifier.as_deref(),
            Some("YOUNGSTER")
        );
        assert!(overworld.player_hidden);
    }

    #[test]
    fn state_block_overrides_validate_all_coordinates_before_mutating_blocks() {
        let mut state = GameState::default();
        state.map_block_overrides.insert(
            "Route29".to_string(),
            BTreeMap::from([((1, 0), 0x22), ((3, 0), 0x33)]),
        );
        let mut overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            Vec::new(),
            test_tileset(),
            TilePosition::new(0, 0),
        );
        let original_blocks = overworld.map.metatile_ids.clone();

        assert_eq!(
            apply_state_block_overrides(&mut overworld, &state),
            Err(MapContextError::BlockOverrideOutsideMap {
                map_name: "Route29".to_string(),
                x: 3,
                y: 0,
            })
        );
        assert_eq!(overworld.map.metatile_ids, original_blocks);
    }

    #[test]
    fn state_object_overrides_reject_coordinates_that_overflow_runtime_tiles() {
        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "YOUNGSTER".to_string(),
                    OverworldObjectMemory {
                        x: 40_000,
                        y: 0,
                        tile: Some(TilePosition::new(0, 0)),
                        facing: None,
                    },
                )]),
                hidden_object_identifiers: BTreeSet::new(),
                following: None,
                last_talked_object_identifier: None,
                player_hidden: false,
            },
        );
        let mut overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("YOUNGSTER", 4, 1)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        assert_eq!(
            apply_state_object_overrides(&mut overworld, &state),
            Err(MapContextError::ObjectOverrideCoordinatesOutOfRange {
                map_name: "Route29".to_string(),
                object_id: "YOUNGSTER".to_string(),
                x: 40_000,
                y: 0,
            })
        );
    }

    #[test]
    fn state_object_overrides_validate_all_coordinates_before_mutating_objects() {
        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([
                    (
                        "FIRST".to_string(),
                        OverworldObjectMemory {
                            x: 2,
                            y: 0,
                            tile: Some(TilePosition::new(2, 0)),
                            facing: Some(Direction::Right),
                        },
                    ),
                    (
                        "SECOND".to_string(),
                        OverworldObjectMemory {
                            x: 40_000,
                            y: 0,
                            tile: None,
                            facing: Some(Direction::Left),
                        },
                    ),
                ]),
                hidden_object_identifiers: BTreeSet::from(["HIDDEN_NPC".to_string()]),
                following: None,
                last_talked_object_identifier: Some("FIRST".to_string()),
                player_hidden: true,
            },
        );
        let mut overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("FIRST", 0, 0), test_object("SECOND", 1, 0)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        assert_eq!(
            apply_state_object_overrides(&mut overworld, &state),
            Err(MapContextError::ObjectOverrideCoordinatesOutOfRange {
                map_name: "Route29".to_string(),
                object_id: "SECOND".to_string(),
                x: 40_000,
                y: 0,
            })
        );
        assert_eq!((overworld.objects[0].x, overworld.objects[0].y), (0, 0));
        assert_eq!((overworld.objects[1].x, overworld.objects[1].y), (1, 0));
        assert!(overworld.object_facings.is_empty());
        assert!(overworld.hidden_object_identifiers.is_empty());
        assert_eq!(overworld.last_talked_object_identifier, None);
        assert!(!overworld.player_hidden);
    }

    #[test]
    fn state_object_overrides_reject_coordinates_outside_runtime_map_bounds() {
        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "YOUNGSTER".to_string(),
                    OverworldObjectMemory {
                        x: 6,
                        y: 0,
                        tile: Some(TilePosition::new(6, 0)),
                        facing: None,
                    },
                )]),
                hidden_object_identifiers: BTreeSet::new(),
                following: None,
                last_talked_object_identifier: None,
                player_hidden: false,
            },
        );
        let mut overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("YOUNGSTER", 4, 1)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        assert_eq!(
            apply_state_object_overrides(&mut overworld, &state),
            Err(MapContextError::ObjectOverrideOutsideMap {
                map_name: "Route29".to_string(),
                object_id: "YOUNGSTER".to_string(),
                x: 6,
                y: 0,
                width: 6,
                height: 4,
            })
        );
    }

    #[test]
    fn sync_state_object_overrides_persists_raw_event_coordinates_and_runtime_snapshot() {
        let mut state = GameState::default();
        let mut overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("YOUNGSTER", 1, 1)],
            test_tileset(),
            TilePosition::new(2, 0),
        );
        overworld.objects[0].x = 2;
        overworld.objects[0].y = 1;
        overworld
            .object_facings
            .insert("YOUNGSTER".to_string(), Direction::Right);
        overworld
            .hidden_object_identifiers
            .insert("CUT_TREE".to_string());
        overworld.last_talked_object_identifier = Some("YOUNGSTER".to_string());
        overworld.player_hidden = true;

        assert_eq!(
            object_tile_position_checked(&overworld.objects[0]),
            Some(TilePosition::new(4, 2))
        );
        sync_state_object_overrides(&mut state, &overworld).expect("object overrides sync");

        let memory = state
            .map_object_overrides
            .get("Route29")
            .expect("route object memory");
        assert_eq!(
            memory.objects.get("YOUNGSTER"),
            Some(&OverworldObjectMemory {
                x: 2,
                y: 1,
                tile: Some(TilePosition::new(2, 1)),
                facing: Some(Direction::Right),
            })
        );
        assert!(memory.hidden_object_identifiers.contains("CUT_TREE"));
        assert_eq!(
            memory.last_talked_object_identifier.as_deref(),
            Some("YOUNGSTER")
        );
        assert!(memory.player_hidden);
        assert_eq!(
            state.overworld.snapshot_identity(),
            Some((
                "Route29",
                TilePosition::new(2, 0),
                Direction::Down,
                crate::world::movement::MovementMode::Normal,
            ))
        );
    }

    #[test]
    fn sync_state_object_overrides_rejects_unsaveable_object_coordinates_without_state_changes() {
        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "OLD".to_string(),
                    OverworldObjectMemory {
                        x: 1,
                        y: 1,
                        tile: Some(TilePosition::new(1, 1)),
                        facing: Some(Direction::Down),
                    },
                )]),
                hidden_object_identifiers: BTreeSet::from(["OLD_HIDDEN".to_string()]),
                following: None,
                last_talked_object_identifier: Some("OLD".to_string()),
                player_hidden: true,
            },
        );
        let before_overrides = state.map_object_overrides.clone();
        let before_overworld = state.overworld.clone();
        let overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("BAD", 40_000, 0)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        assert_eq!(
            sync_state_object_overrides(&mut state, &overworld),
            Err(MapContextError::ObjectOverrideCoordinatesOutOfRange {
                map_name: "Route29".to_string(),
                object_id: "BAD".to_string(),
                x: 40_000,
                y: 0,
            })
        );
        assert_eq!(state.map_object_overrides, before_overrides);
        assert_eq!(state.overworld, before_overworld);
    }

    #[test]
    fn sync_state_object_overrides_rejects_out_of_bounds_object_coordinates_without_state_changes()
    {
        let mut state = GameState::default();
        let before_overworld = state.overworld.clone();
        let overworld = OverworldSession::with_events_and_objects(
            test_map(),
            Default::default(),
            vec![test_object("OUTSIDE", 6, 0)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        assert_eq!(
            sync_state_object_overrides(&mut state, &overworld),
            Err(MapContextError::ObjectOverrideOutsideMap {
                map_name: "Route29".to_string(),
                object_id: "OUTSIDE".to_string(),
                x: 6,
                y: 0,
                width: 6,
                height: 4,
            })
        );
        assert!(!state.map_object_overrides.contains_key("Route29"));
        assert_eq!(state.overworld, before_overworld);
    }

    #[test]
    fn object_overrides_reject_overflowing_map_bounds_without_mutating_session() {
        let mut state = GameState::default();
        state.map_object_overrides.insert(
            "Route29".to_string(),
            OverworldObjectMapMemory {
                objects: BTreeMap::from([(
                    "YOUNGSTER".to_string(),
                    OverworldObjectMemory {
                        x: 1,
                        y: 1,
                        tile: Some(TilePosition::new(1, 1)),
                        facing: Some(Direction::Down),
                    },
                )]),
                hidden_object_identifiers: BTreeSet::new(),
                following: None,
                last_talked_object_identifier: None,
                player_hidden: false,
            },
        );
        let mut overworld = OverworldSession::with_events_and_objects(
            overflowing_test_map(),
            Default::default(),
            vec![test_object("YOUNGSTER", 0, 0)],
            test_tileset(),
            TilePosition::new(0, 0),
        );
        let before_objects = overworld.objects.clone();
        let before_facings = overworld.object_facings.clone();

        assert_eq!(
            apply_state_object_overrides(&mut overworld, &state),
            Err(MapContextError::MapBoundsOverflow {
                map_name: "Route29".to_string(),
            })
        );
        assert_eq!(overworld.objects, before_objects);
        assert_eq!(overworld.object_facings, before_facings);
    }

    #[test]
    fn sync_state_object_overrides_rejects_overflowing_map_bounds_without_state_changes() {
        let mut state = GameState::default();
        let before_overrides = state.map_object_overrides.clone();
        let before_overworld = state.overworld.clone();
        let overworld = OverworldSession::with_events_and_objects(
            overflowing_test_map(),
            Default::default(),
            vec![test_object("YOUNGSTER", 0, 0)],
            test_tileset(),
            TilePosition::new(0, 0),
        );

        assert_eq!(
            sync_state_object_overrides(&mut state, &overworld),
            Err(MapContextError::MapBoundsOverflow {
                map_name: "Route29".to_string(),
            })
        );
        assert_eq!(state.map_object_overrides, before_overrides);
        assert_eq!(state.overworld, before_overworld);
    }
}
