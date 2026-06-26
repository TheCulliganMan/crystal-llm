use serde::{Deserialize, Serialize};

use crate::world::map::OverworldMapData;

pub const CHANGE_BLOCK_COORD_STRIDE: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptBlockChange {
    pub x: u16,
    pub y: u16,
    pub block_id: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptBlockChangeOutcome {
    pub map_name: String,
    pub x: u16,
    pub y: u16,
    pub metatile_x: u16,
    pub metatile_y: u16,
    pub previous_block_id: u16,
    pub block_id: u16,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptBlockError {
    OutOfBounds {
        map_name: String,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScriptBlockChangeIssue {
    OutOfBounds {
        source_script: String,
        command_index: usize,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    },
    MapSizeMismatch {
        source_script: String,
        command_index: usize,
        actual_blocks: usize,
        expected_blocks: usize,
    },
}

pub fn script_block_change_issues(
    changes: &[ScriptBlockChange],
    width: u16,
    height: u16,
    block_count: usize,
) -> Vec<ScriptBlockChangeIssue> {
    let expected_blocks = width as usize * height as usize;
    let mut issues = Vec::new();
    for change in changes {
        let metatile_x = change.x / CHANGE_BLOCK_COORD_STRIDE;
        let metatile_y = change.y / CHANGE_BLOCK_COORD_STRIDE;
        if metatile_x >= width || metatile_y >= height {
            issues.push(ScriptBlockChangeIssue::OutOfBounds {
                source_script: change.source_script.clone(),
                command_index: change.command_index,
                x: change.x,
                y: change.y,
                width,
                height,
            });
        }
        if block_count != 0 && block_count != expected_blocks {
            issues.push(ScriptBlockChangeIssue::MapSizeMismatch {
                source_script: change.source_script.clone(),
                command_index: change.command_index,
                actual_blocks: block_count,
                expected_blocks,
            });
        }
    }
    issues
}

pub fn apply_script_block_change(
    map: &mut OverworldMapData,
    change: ScriptBlockChange,
) -> Result<ScriptBlockChangeOutcome, ScriptBlockError> {
    let metatile_x = change.x / CHANGE_BLOCK_COORD_STRIDE;
    let metatile_y = change.y / CHANGE_BLOCK_COORD_STRIDE;
    let index = map
        .metatile_index(metatile_x as i16, metatile_y as i16)
        .ok_or_else(|| ScriptBlockError::OutOfBounds {
            map_name: map.name.clone(),
            x: change.x,
            y: change.y,
            width: map.width,
            height: map.height,
        })?;
    let previous_block_id = map.metatile_ids[index];
    map.metatile_ids[index] = change.block_id;
    Ok(ScriptBlockChangeOutcome {
        map_name: map.name.clone(),
        x: change.x,
        y: change.y,
        metatile_x,
        metatile_y,
        previous_block_id,
        block_id: change.block_id,
        source_script: change.source_script,
        command_index: change.command_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapAttributes;

    fn map() -> OverworldMapData {
        OverworldMapData::from_attributes(
            "RuinsOfAlphKabutoChamber",
            &MapAttributes {
                tileset_name: "ruins".to_string(),
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

    fn change(x: u16, y: u16, block_id: u16) -> ScriptBlockChange {
        ScriptBlockChange {
            x,
            y,
            block_id,
            source_script: "DoorScript".to_string(),
            command_index: 7,
        }
    }

    #[test]
    fn changes_exact_in_bounds_block() {
        let mut map = map();
        let outcome =
            apply_script_block_change(&mut map, change(2, 2, 0x2e)).expect("change block");

        assert_eq!(outcome.previous_block_id, 5);
        assert_eq!((outcome.metatile_x, outcome.metatile_y), (1, 1));
        assert_eq!(outcome.block_id, 0x2e);
        assert_eq!(map.metatile_at(1, 1), Some(0x2e));
        assert_eq!(map.metatile_ids, vec![1, 2, 3, 4, 0x2e, 6]);
    }

    #[test]
    fn rejects_out_of_bounds_without_resizing_map() {
        let mut map = map();
        let original = map.metatile_ids.clone();
        let error = apply_script_block_change(&mut map, change(6, 0, 0x2e))
            .expect_err("out of bounds block is an error");

        assert_eq!(
            error,
            ScriptBlockError::OutOfBounds {
                map_name: "RuinsOfAlphKabutoChamber".to_string(),
                x: 6,
                y: 0,
                width: 3,
                height: 2,
            }
        );
        assert_eq!(map.metatile_ids, original);
    }

    #[test]
    fn script_block_change_issues_validate_bounds_and_exact_block_count() {
        let changes = vec![change(6, 0, 0x2e), change(0, 2, 0x2f)];

        assert_eq!(
            script_block_change_issues(&changes, 3, 2, 5),
            vec![
                ScriptBlockChangeIssue::OutOfBounds {
                    source_script: "DoorScript".to_string(),
                    command_index: 7,
                    x: 6,
                    y: 0,
                    width: 3,
                    height: 2,
                },
                ScriptBlockChangeIssue::MapSizeMismatch {
                    source_script: "DoorScript".to_string(),
                    command_index: 7,
                    actual_blocks: 5,
                    expected_blocks: 6,
                },
                ScriptBlockChangeIssue::MapSizeMismatch {
                    source_script: "DoorScript".to_string(),
                    command_index: 7,
                    actual_blocks: 5,
                    expected_blocks: 6,
                },
            ]
        );

        assert!(
            script_block_change_issues(&changes, 3, 2, 0)
                .iter()
                .all(|issue| matches!(issue, ScriptBlockChangeIssue::OutOfBounds { .. }))
        );
    }
}
