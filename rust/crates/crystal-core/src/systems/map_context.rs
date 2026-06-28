use serde::{Deserialize, Serialize};

use crate::map::MapSceneTable;
use crate::state::{
    GameState, OverworldFollowMemory, OverworldMemory, OverworldObjectMapMemory,
    OverworldObjectMemory, SceneError,
};
use crate::world::session::{OverworldFollowState, OverworldSession, OverworldSnapshot};

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
    #[error("saved object override references missing object {object_id} on map {map_name}")]
    MissingObjectOverride { map_name: String, object_id: String },
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
        overworld.map.metatile_ids[index] = *block_id;
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
    for (object_id, object_memory) in &memory.objects {
        let object = overworld
            .objects
            .iter_mut()
            .find(|object| object.object_identifier.as_deref() == Some(object_id.as_str()))
            .ok_or_else(|| MapContextError::MissingObjectOverride {
                map_name: overworld.map.name.clone(),
                object_id: object_id.clone(),
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

pub fn sync_state_object_overrides(state: &mut GameState, overworld: &OverworldSession) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapScene;

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
                script_name: "SceneScript".to_string(),
                script_index: 0,
                command_index: 1,
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
                script_name: "SceneScript".to_string(),
                script_index: 0,
                command_index: 1,
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
}
