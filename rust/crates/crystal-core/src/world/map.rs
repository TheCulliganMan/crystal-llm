use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::map::{MapAttributes, MapConnection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Direction {
    Down,
    Up,
    Left,
    Right,
}

impl Direction {
    pub const fn delta(self) -> (i16, i16) {
        match self {
            Self::Down => (0, 1),
            Self::Up => (0, -1),
            Self::Left => (-1, 0),
            Self::Right => (1, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TilePosition {
    pub x: i16,
    pub y: i16,
}

impl TilePosition {
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    pub const fn moved(self, direction: Direction) -> Self {
        let (dx, dy) = direction.delta();
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OverworldMapData {
    pub name: String,
    pub width: u16,
    pub height: u16,
    pub border_block: u16,
    pub connections: Vec<MapConnection>,
    pub metatile_ids: Vec<u16>,
}

impl<'de> Deserialize<'de> for OverworldMapData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawOverworldMapData {
            name: String,
            width: u16,
            height: u16,
            border_block: u16,
            connections: Vec<MapConnection>,
            metatile_ids: Vec<u16>,
        }

        let raw = RawOverworldMapData::deserialize(deserializer)?;
        let map = Self {
            name: raw.name,
            width: raw.width,
            height: raw.height,
            border_block: raw.border_block,
            connections: raw.connections,
            metatile_ids: raw.metatile_ids,
        };
        map.validate().map_err(D::Error::custom)?;
        Ok(map)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeMapMetadata {
    pub constant: String,
    pub name: String,
    pub group_name: String,
    pub group_id: u16,
    pub map_id: u16,
    pub width: u16,
    pub height: u16,
    pub environment: String,
    pub phone_service: u8,
}

impl<'de> Deserialize<'de> for RuntimeMapMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct RawRuntimeMapMetadata {
            #[serde(deserialize_with = "required_metadata_token")]
            constant: String,
            #[serde(deserialize_with = "required_metadata_token")]
            name: String,
            #[serde(deserialize_with = "required_metadata_token")]
            group_name: String,
            group_id: u16,
            map_id: u16,
            width: u16,
            height: u16,
            #[serde(deserialize_with = "required_metadata_token")]
            environment: String,
            phone_service: u8,
        }

        let raw = RawRuntimeMapMetadata::deserialize(deserializer)?;
        let metadata = Self {
            constant: raw.constant,
            name: raw.name,
            group_name: raw.group_name,
            group_id: raw.group_id,
            map_id: raw.map_id,
            width: raw.width,
            height: raw.height,
            environment: raw.environment,
            phone_service: raw.phone_service,
        };
        metadata.validate().map_err(D::Error::custom)?;
        Ok(metadata)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeMapMetadataIssue {
    ConstantMismatch {
        key: String,
        record_constant: String,
    },
    NameMismatch {
        key: String,
        constant: String,
        metadata_name: String,
        map_name: String,
    },
    UnknownMapConstant {
        key: String,
        constant: String,
    },
    InvalidMetadata {
        key: String,
    },
}

pub fn runtime_map_metadata_issues(
    metadata: &BTreeMap<String, RuntimeMapMetadata>,
    map_names: &BTreeMap<String, String>,
) -> Vec<RuntimeMapMetadataIssue> {
    let mut issues = Vec::new();

    for (key, metadata) in metadata {
        let invalid_metadata = !is_exact_nonempty_metadata_token(key)
            || !is_exact_nonempty_metadata_token(&metadata.constant)
            || !is_exact_nonempty_metadata_token(&metadata.name)
            || !is_exact_nonempty_metadata_token(&metadata.group_name)
            || !is_exact_nonempty_metadata_token(&metadata.environment);
        if invalid_metadata {
            issues.push(RuntimeMapMetadataIssue::InvalidMetadata { key: key.clone() });
        }
        if key != &metadata.constant {
            issues.push(RuntimeMapMetadataIssue::ConstantMismatch {
                key: key.clone(),
                record_constant: metadata.constant.clone(),
            });
        }
        if is_exact_nonempty_metadata_token(&metadata.constant) {
            match map_names.get(&metadata.constant) {
                Some(map_name) if map_name == &metadata.name => {}
                Some(map_name) => issues.push(RuntimeMapMetadataIssue::NameMismatch {
                    key: key.clone(),
                    constant: metadata.constant.clone(),
                    metadata_name: metadata.name.clone(),
                    map_name: map_name.clone(),
                }),
                None => issues.push(RuntimeMapMetadataIssue::UnknownMapConstant {
                    key: key.clone(),
                    constant: metadata.constant.clone(),
                }),
            }
        }
    }

    issues
}

fn is_exact_nonempty_metadata_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !has_reserved_pack_prefix(value)
}

fn required_metadata_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_metadata_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "runtime map metadata token must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

impl OverworldMapData {
    pub fn validate(&self) -> Result<(), String> {
        if !is_exact_nonempty_metadata_token(&self.name) {
            return Err(format!("overworld map name {:?} is not exact", self.name));
        }
        if self.width == 0 {
            return Err(format!("overworld map {} has width 0", self.name));
        }
        if self.height == 0 {
            return Err(format!("overworld map {} has height 0", self.name));
        }
        if self.border_block > u8::MAX as u16 {
            return Err(format!(
                "overworld map {} border_block {} exceeds u8 block id range",
                self.name, self.border_block
            ));
        }
        let expected = self.width as usize * self.height as usize;
        if self.metatile_ids.len() != expected {
            return Err(format!(
                "overworld map {} has {} metatiles, expected {expected}",
                self.name,
                self.metatile_ids.len()
            ));
        }
        Ok(())
    }

    pub fn from_attributes(
        name: impl Into<String>,
        attributes: &MapAttributes,
        metatile_ids: Vec<u16>,
    ) -> Self {
        Self {
            name: name.into(),
            width: attributes.width,
            height: attributes.height,
            border_block: attributes.border_block as u16,
            connections: attributes.connections.clone(),
            metatile_ids,
        }
    }

    pub fn connections(&self) -> &[MapConnection] {
        &self.connections
    }

    pub fn contains_metatile(&self, x: i16, y: i16) -> bool {
        x >= 0 && y >= 0 && (x as u16) < self.width && (y as u16) < self.height
    }

    pub fn metatile_index(&self, x: i16, y: i16) -> Option<usize> {
        self.contains_metatile(x, y)
            .then_some(y as usize * self.width as usize + x as usize)
    }

    pub fn metatile_at(&self, x: i16, y: i16) -> Option<u16> {
        self.metatile_index(x, y)
            .and_then(|index| self.metatile_ids.get(index).copied())
    }

    pub fn tile_bounds(&self) -> (u16, u16) {
        (
            self.width * METATILE_WIDTH as u16,
            self.height * METATILE_WIDTH as u16,
        )
    }
}

impl RuntimeMapMetadata {
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 {
            return Err(format!(
                "runtime map metadata {} has width 0",
                self.constant
            ));
        }
        if self.height == 0 {
            return Err(format!(
                "runtime map metadata {} has height 0",
                self.constant
            ));
        }
        Ok(())
    }
}

pub const METATILE_WIDTH: i16 = 2;
pub const METATILE_SIZE_PX: u16 = 16;

pub fn determine_quadrant_index(tile_x: i16, tile_y: i16) -> Option<usize> {
    if tile_x < 0 || tile_y < 0 {
        return None;
    }
    let half = METATILE_WIDTH / 2;
    let x_half = (tile_x % METATILE_WIDTH) / half;
    let y_half = (tile_y % METATILE_WIDTH) / half;
    Some((y_half * 2 + x_half) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::MapConnection;

    fn attributes(width: u16, height: u16, border_block: u8) -> MapAttributes {
        MapAttributes {
            tileset_name: "test".to_string(),
            border_block,
            width,
            height,
            connections: Vec::<MapConnection>::new(),
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
    fn map_data_preserves_declared_dimensions_and_block_payload_exactly() {
        let padded =
            OverworldMapData::from_attributes("test", &attributes(3, 2, 7), vec![1, 2, 3, 4]);
        assert_eq!((padded.width, padded.height), (3, 2));
        assert_eq!(padded.metatile_ids, vec![1, 2, 3, 4]);
        assert_eq!(padded.metatile_at(1, 1), None);

        let truncated =
            OverworldMapData::from_attributes("test", &attributes(2, 2, 7), vec![1, 2, 3, 4, 5]);
        assert_eq!((truncated.width, truncated.height), (2, 2));
        assert_eq!(truncated.metatile_ids, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn runtime_map_metadata_issues_validate_exact_pack_records() {
        let metadata = [
            (
                "ROUTE_29".to_string(),
                RuntimeMapMetadata {
                    constant: "ROUTE_29_ALIAS".to_string(),
                    name: "Route29".to_string(),
                    group_name: String::new(),
                    group_id: 1,
                    map_id: 1,
                    width: 10,
                    height: 9,
                    environment: "TOWN".to_string(),
                    phone_service: 1,
                },
            ),
            (
                "ROUTE 30".to_string(),
                RuntimeMapMetadata {
                    constant: "ROUTE 30".to_string(),
                    name: "Route 30".to_string(),
                    group_name: "GROUP ROUTE_30".to_string(),
                    group_id: 1,
                    map_id: 3,
                    width: 10,
                    height: 9,
                    environment: "ROUTE".to_string(),
                    phone_service: 1,
                },
            ),
            (
                "NEW_BARK_TOWN".to_string(),
                RuntimeMapMetadata {
                    constant: "NEW_BARK_TOWN".to_string(),
                    name: "WrongName".to_string(),
                    group_name: "GROUP_NEW_BARK".to_string(),
                    group_id: 1,
                    map_id: 2,
                    width: 10,
                    height: 9,
                    environment: "TOWN".to_string(),
                    phone_service: 1,
                },
            ),
        ]
        .into_iter()
        .collect();
        let map_names = [("NEW_BARK_TOWN".to_string(), "NewBarkTown".to_string())]
            .into_iter()
            .collect();

        assert_eq!(
            runtime_map_metadata_issues(&metadata, &map_names),
            vec![
                RuntimeMapMetadataIssue::NameMismatch {
                    key: "NEW_BARK_TOWN".to_string(),
                    constant: "NEW_BARK_TOWN".to_string(),
                    metadata_name: "WrongName".to_string(),
                    map_name: "NewBarkTown".to_string(),
                },
                RuntimeMapMetadataIssue::InvalidMetadata {
                    key: "ROUTE 30".to_string(),
                },
                RuntimeMapMetadataIssue::InvalidMetadata {
                    key: "ROUTE_29".to_string(),
                },
                RuntimeMapMetadataIssue::ConstantMismatch {
                    key: "ROUTE_29".to_string(),
                    record_constant: "ROUTE_29_ALIAS".to_string(),
                },
                RuntimeMapMetadataIssue::UnknownMapConstant {
                    key: "ROUTE_29".to_string(),
                    constant: "ROUTE_29_ALIAS".to_string(),
                },
            ],
        );
    }

    #[test]
    fn runtime_map_metadata_rejects_reserved_pack_prefixes() {
        for (field, value) in [
            ("constant", serde_json::json!("fallback_route_29")),
            ("name", serde_json::json!("legacy_route_29")),
            ("groupName", serde_json::json!("fallback_group")),
            ("environment", serde_json::json!("legacy_route")),
        ] {
            let mut metadata = valid_runtime_map_metadata_json();
            metadata[field] = value;

            let error = serde_json::from_value::<RuntimeMapMetadata>(metadata)
                .expect_err("reserved runtime map metadata tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("runtime map metadata token must be"),
                "{field} produced unexpected error: {error}"
            );
        }

        let metadata = [(
            "fallback_route_29".to_string(),
            RuntimeMapMetadata {
                constant: "fallback_route_29".to_string(),
                name: "legacy_route_29".to_string(),
                group_name: "GROUP_NEW_BARK".to_string(),
                group_id: 1,
                map_id: 2,
                width: 10,
                height: 9,
                environment: "TOWN".to_string(),
                phone_service: 1,
            },
        )]
        .into_iter()
        .collect();
        let map_names = [(
            "fallback_route_29".to_string(),
            "legacy_route_29".to_string(),
        )]
        .into_iter()
        .collect();

        assert_eq!(
            runtime_map_metadata_issues(&metadata, &map_names),
            vec![RuntimeMapMetadataIssue::InvalidMetadata {
                key: "fallback_route_29".to_string(),
            }],
        );
    }

    fn valid_runtime_map_metadata_json() -> serde_json::Value {
        serde_json::json!({
            "constant": "ROUTE_29",
            "name": "Route29",
            "groupName": "GROUP_NEW_BARK",
            "groupId": 1,
            "mapId": 2,
            "width": 10,
            "height": 9,
            "environment": "TOWN",
            "phoneService": 1
        })
    }

    #[test]
    fn tile_positions_move_by_direction() {
        let pos = TilePosition::new(5, 5);
        assert_eq!(pos.moved(Direction::Up), TilePosition::new(5, 4));
        assert_eq!(pos.moved(Direction::Right), TilePosition::new(6, 5));
    }

    #[test]
    fn tile_position_json_rejects_alternate_coordinate_fields() {
        let error = serde_json::from_value::<TilePosition>(serde_json::json!({
            "x": 5,
            "y": 6,
            "tileX": 5
        }))
        .expect_err("tile positions must use exact coordinate fields")
        .to_string();

        assert!(error.contains("unknown field `tileX`"), "{error}");
    }

    #[test]
    fn direction_json_rejects_legacy_alias_payloads() {
        let error = serde_json::from_value::<Direction>(serde_json::json!({
            "down": {
                "legacy_direction": "DOWN"
            }
        }))
        .expect_err("directions must not accept legacy object payloads")
        .to_string();
        assert!(
            error.contains("invalid type") || error.contains("unknown variant"),
            "{error}"
        );
    }

    #[test]
    fn quadrant_index_matches_two_by_two_metatile_layout() {
        assert_eq!(determine_quadrant_index(0, 0), Some(0));
        assert_eq!(determine_quadrant_index(1, 0), Some(1));
        assert_eq!(determine_quadrant_index(0, 1), Some(2));
        assert_eq!(determine_quadrant_index(1, 1), Some(3));
        assert_eq!(determine_quadrant_index(-1, 0), None);
    }
}
