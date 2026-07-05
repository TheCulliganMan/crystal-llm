use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::world::map::{METATILE_WIDTH, OverworldMapData};

pub const CHANGE_BLOCK_COORD_STRIDE: u16 = METATILE_WIDTH as u16;
pub const SCRIPT_BLOCK_CHANGE_COMMANDS: &[&str] = &["changeblock"];

pub fn is_known_script_block_change_command(command: &str) -> bool {
    SCRIPT_BLOCK_CHANGE_COMMANDS.contains(&command)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptBlockChange {
    pub x: u16,
    pub y: u16,
    pub block_id: u16,
    #[serde(deserialize_with = "required_script_block_label_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptBlockChange {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptBlockChange {
            x: u16,
            y: u16,
            block_id: u16,
            #[serde(deserialize_with = "required_script_block_label_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptBlockChange::deserialize(deserializer)?;
        let change = Self {
            x: raw.x,
            y: raw.y,
            block_id: raw.block_id,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        validate_script_block_change_shape(&change).map_err(D::Error::custom)?;
        Ok(change)
    }
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
#[serde(deny_unknown_fields)]
pub enum ScriptBlockError {
    InvalidSourceScript {
        source_script: String,
    },
    UnalignedCoordinates {
        source_script: String,
        command_index: usize,
        x: u16,
        y: u16,
    },
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
    InvalidSourceScript {
        source_script: String,
        command_index: usize,
    },
    UnalignedCoordinates {
        source_script: String,
        command_index: usize,
        x: u16,
        y: u16,
    },
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

fn validate_script_block_change_shape(change: &ScriptBlockChange) -> Result<(), String> {
    if script_block_change_metatile(change.x, change.y).is_none() {
        return Err(format!(
            "script block change at ({}, {}) is not aligned to stride {}",
            change.x, change.y, CHANGE_BLOCK_COORD_STRIDE
        ));
    }
    Ok(())
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
        if !is_exact_script_block_label_token(&change.source_script) {
            issues.push(ScriptBlockChangeIssue::InvalidSourceScript {
                source_script: change.source_script.clone(),
                command_index: change.command_index,
            });
        }
        let Some((metatile_x, metatile_y)) = script_block_change_metatile(change.x, change.y)
        else {
            issues.push(ScriptBlockChangeIssue::UnalignedCoordinates {
                source_script: change.source_script.clone(),
                command_index: change.command_index,
                x: change.x,
                y: change.y,
            });
            continue;
        };
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
    if !is_exact_script_block_label_token(&change.source_script) {
        return Err(ScriptBlockError::InvalidSourceScript {
            source_script: change.source_script,
        });
    }
    let (metatile_x, metatile_y) =
        script_block_change_metatile(change.x, change.y).ok_or_else(|| {
            ScriptBlockError::UnalignedCoordinates {
                source_script: change.source_script.clone(),
                command_index: change.command_index,
                x: change.x,
                y: change.y,
            }
        })?;
    let metatile_x = i16::try_from(metatile_x).map_err(|_| ScriptBlockError::OutOfBounds {
        map_name: map.name.clone(),
        x: change.x,
        y: change.y,
        width: map.width,
        height: map.height,
    })?;
    let metatile_y = i16::try_from(metatile_y).map_err(|_| ScriptBlockError::OutOfBounds {
        map_name: map.name.clone(),
        x: change.x,
        y: change.y,
        width: map.width,
        height: map.height,
    })?;
    let index = map.metatile_index(metatile_x, metatile_y).ok_or_else(|| {
        ScriptBlockError::OutOfBounds {
            map_name: map.name.clone(),
            x: change.x,
            y: change.y,
            width: map.width,
            height: map.height,
        }
    })?;
    let previous_block_id = map.metatile_ids[index];
    map.metatile_ids[index] = change.block_id;
    Ok(ScriptBlockChangeOutcome {
        map_name: map.name.clone(),
        x: change.x,
        y: change.y,
        metatile_x: u16::try_from(metatile_x).map_err(|_| ScriptBlockError::OutOfBounds {
            map_name: map.name.clone(),
            x: change.x,
            y: change.y,
            width: map.width,
            height: map.height,
        })?,
        metatile_y: u16::try_from(metatile_y).map_err(|_| ScriptBlockError::OutOfBounds {
            map_name: map.name.clone(),
            x: change.x,
            y: change.y,
            width: map.width,
            height: map.height,
        })?,
        previous_block_id,
        block_id: change.block_id,
        source_script: change.source_script,
        command_index: change.command_index,
    })
}

fn script_block_change_metatile(x: u16, y: u16) -> Option<(u16, u16)> {
    if x % CHANGE_BLOCK_COORD_STRIDE != 0 || y % CHANGE_BLOCK_COORD_STRIDE != 0 {
        return None;
    }
    Some((x / CHANGE_BLOCK_COORD_STRIDE, y / CHANGE_BLOCK_COORD_STRIDE))
}

fn is_exact_script_block_label_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_graphic())
        && !has_reserved_pack_prefix(value)
}

fn required_script_block_label_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_script_block_label_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script block label must be exact visible ASCII, found {value:?}"
        )))
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
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
    fn exported_script_block_command_set_is_exact() {
        assert!(SCRIPT_BLOCK_CHANGE_COMMANDS.contains(&"changeblock"));
        assert!(is_known_script_block_change_command("changeblock"));
        assert!(!is_known_script_block_change_command("ChangeBlock"));
        assert!(!is_known_script_block_change_command(
            "fallback_changeblock"
        ));
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
    fn rejects_coordinates_that_overflow_metatile_lookup_without_mutating_map() {
        let mut map = map();
        let original = map.metatile_ids.clone();
        let error = apply_script_block_change(&mut map, change(u16::MAX - 1, 0, 0x2e))
            .expect_err("overflowing metatile coordinate is an error");

        assert_eq!(
            error,
            ScriptBlockError::OutOfBounds {
                map_name: "RuinsOfAlphKabutoChamber".to_string(),
                x: u16::MAX - 1,
                y: 0,
                width: 3,
                height: 2,
            }
        );
        assert_eq!(map.metatile_ids, original);
    }

    #[test]
    fn rejects_unaligned_coordinates_without_flooring_or_mutating_map() {
        let mut map = map();
        let original = map.metatile_ids.clone();
        let error = apply_script_block_change(&mut map, change(3, 2, 0x2e))
            .expect_err("odd changeblock coordinate is malformed");

        assert_eq!(
            error,
            ScriptBlockError::UnalignedCoordinates {
                source_script: "DoorScript".to_string(),
                command_index: 7,
                x: 3,
                y: 2,
            }
        );
        assert_eq!(map.metatile_ids, original);
    }

    #[test]
    fn rejects_invalid_source_script_without_mutating_map() {
        let mut map = map();
        let original = map.metatile_ids.clone();
        let mut change = change(2, 2, 0x2e);
        change.source_script = "legacy_script".to_string();

        assert_eq!(
            script_block_change_issues(&[change.clone()], 3, 2, 6),
            vec![ScriptBlockChangeIssue::InvalidSourceScript {
                source_script: "legacy_script".to_string(),
                command_index: 7,
            }]
        );
        assert_eq!(
            apply_script_block_change(&mut map, change),
            Err(ScriptBlockError::InvalidSourceScript {
                source_script: "legacy_script".to_string(),
            })
        );
        assert_eq!(map.metatile_ids, original);
    }

    #[test]
    fn script_block_change_issues_validate_bounds_and_exact_block_count() {
        let changes = vec![change(6, 0, 0x2e), change(0, 2, 0x2f), change(3, 2, 0x30)];

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
                ScriptBlockChangeIssue::UnalignedCoordinates {
                    source_script: "DoorScript".to_string(),
                    command_index: 7,
                    x: 3,
                    y: 2,
                },
            ]
        );

        assert!(
            script_block_change_issues(&changes, 3, 2, 0)
                .iter()
                .all(|issue| matches!(
                    issue,
                    ScriptBlockChangeIssue::OutOfBounds { .. }
                        | ScriptBlockChangeIssue::UnalignedCoordinates { .. }
                ))
        );
    }

    #[test]
    fn script_block_error_json_rejects_unknown_fallback_fields() {
        let command_error = serde_json::from_value::<ScriptBlockChange>(serde_json::json!({
            "x": 2,
            "y": 2,
            "block_id": 46,
            "source_script": "fallback_script",
            "command_index": 7
        }))
        .expect_err("script block changes must reject fallback source labels")
        .to_string();
        assert!(
            command_error.contains("script block label"),
            "{command_error}"
        );

        let error = serde_json::from_value::<ScriptBlockError>(serde_json::json!({
            "OutOfBounds": {
                "map_name": "RUINS_OF_ALPH",
                "x": 6,
                "y": 0,
                "width": 3,
                "height": 2,
                "fallback_block_id": 0
            }
        }))
        .expect_err("script block errors must not accept fallback block ids")
        .to_string();
        assert!(
            error.contains("unknown field `fallback_block_id`"),
            "{error}"
        );
    }
}
