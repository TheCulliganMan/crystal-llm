use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapConnection {
    #[serde(deserialize_with = "required_map_token")]
    pub direction: String,
    #[serde(deserialize_with = "required_map_token")]
    pub target_map: String,
    pub offset: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapAttributes {
    pub tileset_name: String,
    pub border_block: u8,
    pub width: u16,
    pub height: u16,
    pub connections: Vec<MapConnection>,
    pub time_of_day: Option<String>,
    pub phone_service: u8,
    pub phone_flag: bool,
    pub environment: Option<String>,
    pub location: Option<String>,
    pub music: Option<String>,
    pub palette: Option<String>,
    pub fishing_group: Option<String>,
    pub map_constant: Option<String>,
    pub map_group_constant: Option<String>,
    pub blocks_label: Option<String>,
    pub map_scripts_label: Option<String>,
    pub map_events_label: Option<String>,
    pub connection_flags: Option<String>,
}

impl<'de> Deserialize<'de> for MapAttributes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMapAttributes {
            #[serde(deserialize_with = "required_map_token")]
            tileset_name: String,
            border_block: u8,
            width: u16,
            height: u16,
            connections: Vec<MapConnection>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            time_of_day: Option<String>,
            phone_service: u8,
            phone_flag: bool,
            #[serde(deserialize_with = "required_nullable_map_token")]
            environment: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            location: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_expression_token")]
            music: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            palette: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            fishing_group: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            map_constant: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            map_group_constant: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            blocks_label: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            map_scripts_label: Option<String>,
            #[serde(deserialize_with = "required_nullable_map_token")]
            map_events_label: Option<String>,
            #[serde(deserialize_with = "required_nullable_connection_flags_token")]
            connection_flags: Option<String>,
        }

        let raw = RawMapAttributes::deserialize(deserializer)?;
        let attributes = Self {
            tileset_name: raw.tileset_name,
            border_block: raw.border_block,
            width: raw.width,
            height: raw.height,
            connections: raw.connections,
            time_of_day: raw.time_of_day,
            phone_service: raw.phone_service,
            phone_flag: raw.phone_flag,
            environment: raw.environment,
            location: raw.location,
            music: raw.music,
            palette: raw.palette,
            fishing_group: raw.fishing_group,
            map_constant: raw.map_constant,
            map_group_constant: raw.map_group_constant,
            blocks_label: raw.blocks_label,
            map_scripts_label: raw.map_scripts_label,
            map_events_label: raw.map_events_label,
            connection_flags: raw.connection_flags,
        };
        attributes.validate().map_err(D::Error::custom)?;
        Ok(attributes)
    }
}

impl MapAttributes {
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0 {
            return Err(format!("map {} has width 0", self.tileset_name));
        }
        if self.height == 0 {
            return Err(format!("map {} has height 0", self.tileset_name));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpEvent {
    pub index: u16,
    pub x: u16,
    pub y: u16,
    #[serde(deserialize_with = "required_map_token")]
    pub target_map_constant: String,
    #[serde(deserialize_with = "required_map_token")]
    pub target_map: String,
    pub target_warp_id: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordEvent {
    pub x: u16,
    pub y: u16,
    #[serde(deserialize_with = "required_empty_or_map_token")]
    pub scene_id: String,
    #[serde(deserialize_with = "required_map_token")]
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundEvent {
    pub x: u16,
    pub y: u16,
    #[serde(deserialize_with = "required_map_token")]
    pub event_type: String,
    #[serde(deserialize_with = "required_map_token")]
    pub script: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapEvents {
    pub warps: Vec<WarpEvent>,
    pub coord_events: Vec<CoordEvent>,
    pub bg_events: Vec<BackgroundEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapScene {
    #[serde(deserialize_with = "required_map_token")]
    pub scene_id: String,
    #[serde(deserialize_with = "required_nullable_map_token")]
    pub script_name: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSceneTable {
    pub scenes: Vec<MapScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapScriptSectionCommand {
    #[serde(deserialize_with = "required_map_token")]
    pub command: String,
    #[serde(deserialize_with = "required_map_token_vec")]
    pub args: Vec<String>,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for MapScriptSectionCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMapScriptSectionCommand {
            #[serde(deserialize_with = "required_map_token")]
            command: String,
            #[serde(deserialize_with = "required_map_token_vec")]
            args: Vec<String>,
            command_index: usize,
        }

        let raw = RawMapScriptSectionCommand::deserialize(deserializer)?;
        validate_map_section_command_shape(
            "map script",
            &map_script_section_command_arg_counts(),
            &raw.command,
            raw.args.len(),
        )
        .map_err(D::Error::custom)?;
        Ok(Self {
            command: raw.command,
            args: raw.args,
            command_index: raw.command_index,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapEventSectionCommand {
    #[serde(deserialize_with = "required_map_token")]
    pub command: String,
    #[serde(deserialize_with = "required_map_token_vec")]
    pub args: Vec<String>,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for MapEventSectionCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawMapEventSectionCommand {
            #[serde(deserialize_with = "required_map_token")]
            command: String,
            #[serde(deserialize_with = "required_map_token_vec")]
            args: Vec<String>,
            command_index: usize,
        }

        let raw = RawMapEventSectionCommand::deserialize(deserializer)?;
        validate_map_section_command_shape(
            "map event",
            &map_event_section_command_arg_counts(),
            &raw.command,
            raw.args.len(),
        )
        .map_err(D::Error::custom)?;
        Ok(Self {
            command: raw.command,
            args: raw.args,
            command_index: raw.command_index,
        })
    }
}

fn required_map_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_nonempty_map_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "map token must be exact ASCII alphanumeric/underscore/hyphen, found {value:?}"
        )))
    }
}

fn required_nullable_map_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_nonempty_map_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "map token must be exact ASCII alphanumeric/underscore/hyphen, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_nullable_connection_flags_token<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(value) if is_exact_connection_flags_token(&value) => Ok(Some(value)),
        Some(value) => Err(serde::de::Error::custom(format!(
            "connection flags token must be exact ASCII uppercase/underscore/pipe syntax, found {value:?}"
        ))),
        None => Ok(None),
    }
}

fn required_nullable_map_expression_token<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(value) if is_exact_map_expression_token(&value) => Ok(Some(value)),
        Some(value) => Err(serde::de::Error::custom(format!(
            "map expression token must be exact ASCII uppercase/underscore/pipe syntax, found {value:?}"
        ))),
        None => Ok(None),
    }
}

fn is_exact_connection_flags_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'|' | b' ')
        })
        && !has_reserved_pack_prefix(value)
}

fn is_exact_map_expression_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'|' | b' '))
        && !has_reserved_pack_prefix(value)
}

fn required_empty_or_map_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty() || is_exact_nonempty_map_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "map token must be empty or exact ASCII alphanumeric/underscore/hyphen, found {value:?}"
        )))
    }
}

fn required_map_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    if let Some(token) = values
        .iter()
        .find(|token| !is_exact_nonempty_map_token(token))
    {
        Err(serde::de::Error::custom(format!(
            "map token must be exact ASCII alphanumeric/underscore/hyphen, found {token:?}"
        )))
    } else {
        Ok(values)
    }
}

fn is_exact_nonempty_map_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        && !has_reserved_pack_prefix(value)
}

pub fn map_script_section_command_arg_counts() -> BTreeMap<&'static str, BTreeSet<usize>> {
    BTreeMap::from([
        ("def_scene_scripts", BTreeSet::from([0])),
        ("scene_script", BTreeSet::from([1, 2])),
        ("scene_const", BTreeSet::from([1])),
        ("def_callbacks", BTreeSet::from([0])),
        ("callback", BTreeSet::from([2])),
    ])
}

pub fn map_event_section_command_arg_counts() -> BTreeMap<&'static str, BTreeSet<usize>> {
    BTreeMap::from([
        ("db", BTreeSet::from([2])),
        ("def_warp_events", BTreeSet::from([0])),
        ("warp_event", BTreeSet::from([4])),
        ("def_coord_events", BTreeSet::from([0])),
        ("coord_event", BTreeSet::from([4])),
        ("def_bg_events", BTreeSet::from([0])),
        ("bg_event", BTreeSet::from([4])),
        ("def_object_events", BTreeSet::from([0])),
        ("object_event", BTreeSet::from([13])),
    ])
}

fn validate_map_section_command_shape(
    section: &str,
    counts: &BTreeMap<&'static str, BTreeSet<usize>>,
    command: &str,
    actual_arg_count: usize,
) -> Result<(), String> {
    let Some(expected) = counts.get(command) else {
        return Err(format!(
            "{section} command {command:?} is not a Crystal command"
        ));
    };
    if !expected.contains(&actual_arg_count) {
        return Err(format!(
            "{section} command {command} has {actual_arg_count} args, expected {expected:?}"
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MapScriptSectionCommandIssue {
    UnknownCommand,
    WrongArgCount {
        expected: BTreeSet<usize>,
        actual: usize,
    },
    InvalidArg {
        arg: String,
    },
    UnknownSceneScript {
        script: String,
    },
    UnknownCallbackScript {
        script: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MapEventSectionCommandIssue {
    UnknownCommand,
    WrongArgCount {
        expected: BTreeSet<usize>,
        actual: usize,
    },
    InvalidArg {
        arg: String,
    },
    UnknownEventScript {
        script: String,
    },
    UnknownObjectEventScript {
        script: String,
    },
}

pub fn map_script_section_command_issues(
    command: &MapScriptSectionCommand,
    script_labels: &BTreeSet<String>,
) -> Vec<MapScriptSectionCommandIssue> {
    let counts = map_script_section_command_arg_counts();
    let Some(expected) = counts.get(command.command.as_str()) else {
        return vec![MapScriptSectionCommandIssue::UnknownCommand];
    };
    if !expected.contains(&command.args.len()) {
        return vec![MapScriptSectionCommandIssue::WrongArgCount {
            expected: expected.clone(),
            actual: command.args.len(),
        }];
    }
    if let Some(arg) = invalid_section_arg(&command.args) {
        return vec![MapScriptSectionCommandIssue::InvalidArg { arg }];
    }
    match command.command.as_str() {
        "scene_script" => {
            let script = &command.args[0];
            if script_labels.contains(script) {
                Vec::new()
            } else {
                vec![MapScriptSectionCommandIssue::UnknownSceneScript {
                    script: script.clone(),
                }]
            }
        }
        "callback" => {
            let script = &command.args[1];
            if script_labels.contains(script) {
                Vec::new()
            } else {
                vec![MapScriptSectionCommandIssue::UnknownCallbackScript {
                    script: script.clone(),
                }]
            }
        }
        _ => Vec::new(),
    }
}

pub fn map_event_section_command_issues(
    command: &MapEventSectionCommand,
    script_labels: &BTreeSet<String>,
) -> Vec<MapEventSectionCommandIssue> {
    let counts = map_event_section_command_arg_counts();
    let Some(expected) = counts.get(command.command.as_str()) else {
        return vec![MapEventSectionCommandIssue::UnknownCommand];
    };
    if !expected.contains(&command.args.len()) {
        return vec![MapEventSectionCommandIssue::WrongArgCount {
            expected: expected.clone(),
            actual: command.args.len(),
        }];
    }
    if let Some(arg) = invalid_section_arg(&command.args) {
        return vec![MapEventSectionCommandIssue::InvalidArg { arg }];
    }
    match command.command.as_str() {
        "coord_event" | "bg_event" => {
            let script = &command.args[3];
            if script_labels.contains(script) {
                Vec::new()
            } else {
                vec![MapEventSectionCommandIssue::UnknownEventScript {
                    script: script.clone(),
                }]
            }
        }
        "object_event" => {
            let script = &command.args[11];
            if script == "-1" || script == "ObjectEvent" || script_labels.contains(script) {
                Vec::new()
            } else {
                vec![MapEventSectionCommandIssue::UnknownObjectEventScript {
                    script: script.clone(),
                }]
            }
        }
        _ => Vec::new(),
    }
}

fn invalid_section_arg(args: &[String]) -> Option<String> {
    args.iter()
        .find(|arg| !is_exact_nonempty_section_arg(arg))
        .cloned()
}

fn is_exact_nonempty_section_arg(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        && !has_reserved_pack_prefix(value)
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectEvent {
    #[serde(deserialize_with = "required_map_token")]
    pub sprite: String,
    pub sprite_has_facings: bool,
    pub x: u16,
    pub y: u16,
    #[serde(deserialize_with = "required_map_token")]
    pub spritemovedata: String,
    pub move_range_x: u16,
    pub move_range_y: u16,
    pub hram_x: i16,
    pub hram_y: i16,
    pub pal: u8,
    #[serde(deserialize_with = "required_map_token")]
    pub object_type: String,
    pub radius: u16,
    #[serde(deserialize_with = "required_map_token")]
    pub script: String,
    #[serde(deserialize_with = "required_nullable_map_token")]
    pub label: Option<String>,
    #[serde(deserialize_with = "required_empty_or_map_token")]
    pub event_flag: String,
    #[serde(deserialize_with = "required_nullable_map_token")]
    pub object_identifier: Option<String>,
    #[serde(deserialize_with = "required_nullable_map_token")]
    pub sightline_direction_override: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_events_default_to_empty_lists() {
        let events = MapEvents::default();
        assert!(events.warps.is_empty());
        assert!(events.coord_events.is_empty());
        assert!(events.bg_events.is_empty());
    }

    #[test]
    fn map_scene_table_defaults_to_no_scenes() {
        assert!(MapSceneTable::default().scenes.is_empty());
    }

    #[test]
    fn map_section_commands_require_explicit_args() {
        let error = serde_json::from_str::<MapScriptSectionCommand>(
            r#"{"command":"def_scene_scripts","command_index":0}"#,
        )
        .expect_err("missing args must not default to empty")
        .to_string();
        assert!(error.contains("missing field `args`"), "{error}");

        let error = serde_json::from_str::<MapEventSectionCommand>(
            r#"{"command":"def_warp_events","command_index":0}"#,
        )
        .expect_err("missing args must not default to empty")
        .to_string();
        assert!(error.contains("missing field `args`"), "{error}");
    }

    #[test]
    fn map_section_command_arg_counts_are_exact_pack_values() {
        assert_eq!(
            map_script_section_command_arg_counts(),
            BTreeMap::from([
                ("def_scene_scripts", BTreeSet::from([0])),
                ("scene_script", BTreeSet::from([1, 2])),
                ("scene_const", BTreeSet::from([1])),
                ("def_callbacks", BTreeSet::from([0])),
                ("callback", BTreeSet::from([2])),
            ])
        );
        assert_eq!(
            map_event_section_command_arg_counts(),
            BTreeMap::from([
                ("db", BTreeSet::from([2])),
                ("def_warp_events", BTreeSet::from([0])),
                ("warp_event", BTreeSet::from([4])),
                ("def_coord_events", BTreeSet::from([0])),
                ("coord_event", BTreeSet::from([4])),
                ("def_bg_events", BTreeSet::from([0])),
                ("bg_event", BTreeSet::from([4])),
                ("def_object_events", BTreeSet::from([0])),
                ("object_event", BTreeSet::from([13])),
            ])
        );
    }

    #[test]
    fn map_script_section_issues_validate_exact_shapes_and_targets() {
        let labels = BTreeSet::from(["KnownScript".to_string(), "KnownCallback".to_string()]);

        assert_eq!(
            map_script_section_command_issues(
                &MapScriptSectionCommand {
                    command: "Scene_Script".to_string(),
                    args: vec!["KnownScript".to_string()],
                    command_index: 0,
                },
                &labels,
            ),
            vec![MapScriptSectionCommandIssue::UnknownCommand]
        );
        assert_eq!(
            map_script_section_command_issues(
                &MapScriptSectionCommand {
                    command: "scene_script".to_string(),
                    args: Vec::new(),
                    command_index: 1,
                },
                &labels,
            ),
            vec![MapScriptSectionCommandIssue::WrongArgCount {
                expected: BTreeSet::from([1, 2]),
                actual: 0,
            }]
        );
        assert_eq!(
            map_script_section_command_issues(
                &MapScriptSectionCommand {
                    command: "scene_script".to_string(),
                    args: vec!["knownscript".to_string()],
                    command_index: 2,
                },
                &labels,
            ),
            vec![MapScriptSectionCommandIssue::UnknownSceneScript {
                script: "knownscript".to_string()
            }]
        );
        assert_eq!(
            map_script_section_command_issues(
                &MapScriptSectionCommand {
                    command: "scene_script".to_string(),
                    args: vec![" KnownScript".to_string()],
                    command_index: 4,
                },
                &labels,
            ),
            vec![MapScriptSectionCommandIssue::InvalidArg {
                arg: " KnownScript".to_string()
            }]
        );
        assert_eq!(
            map_script_section_command_issues(
                &MapScriptSectionCommand {
                    command: "scene_script".to_string(),
                    args: vec!["Known Script".to_string()],
                    command_index: 5,
                },
                &labels,
            ),
            vec![MapScriptSectionCommandIssue::InvalidArg {
                arg: "Known Script".to_string()
            }]
        );
        assert_eq!(
            map_script_section_command_issues(
                &MapScriptSectionCommand {
                    command: "callback".to_string(),
                    args: vec![
                        "MAPCALLBACK_OBJECTS".to_string(),
                        "KnownCallback".to_string()
                    ],
                    command_index: 3,
                },
                &labels,
            ),
            Vec::<MapScriptSectionCommandIssue>::new()
        );
    }

    #[test]
    fn map_event_section_issues_validate_exact_shapes_and_targets() {
        let labels = BTreeSet::from(["KnownSign".to_string(), "KnownObject".to_string()]);

        assert_eq!(
            map_event_section_command_issues(
                &MapEventSectionCommand {
                    command: "bg_event".to_string(),
                    args: vec!["1".to_string(), "2".to_string()],
                    command_index: 0,
                },
                &labels,
            ),
            vec![MapEventSectionCommandIssue::WrongArgCount {
                expected: BTreeSet::from([4]),
                actual: 2,
            }]
        );
        assert_eq!(
            map_event_section_command_issues(
                &MapEventSectionCommand {
                    command: "bg_event".to_string(),
                    args: vec![
                        "1".to_string(),
                        "2".to_string(),
                        "BGEVENT_READ".to_string(),
                        "knownsign".to_string(),
                    ],
                    command_index: 1,
                },
                &labels,
            ),
            vec![MapEventSectionCommandIssue::UnknownEventScript {
                script: "knownsign".to_string()
            }]
        );
        assert_eq!(
            map_event_section_command_issues(
                &MapEventSectionCommand {
                    command: "bg_event".to_string(),
                    args: vec![
                        "1".to_string(),
                        "2".to_string(),
                        "BGEVENT_READ".to_string(),
                        " KnownSign".to_string(),
                    ],
                    command_index: 4,
                },
                &labels,
            ),
            vec![MapEventSectionCommandIssue::InvalidArg {
                arg: " KnownSign".to_string()
            }]
        );
        assert_eq!(
            map_event_section_command_issues(
                &MapEventSectionCommand {
                    command: "bg_event".to_string(),
                    args: vec![
                        "1".to_string(),
                        "2".to_string(),
                        "BGEVENT READ".to_string(),
                        "KnownSign".to_string(),
                    ],
                    command_index: 5,
                },
                &labels,
            ),
            vec![MapEventSectionCommandIssue::InvalidArg {
                arg: "BGEVENT READ".to_string()
            }]
        );
        assert_eq!(
            map_event_section_command_issues(
                &MapEventSectionCommand {
                    command: "object_event".to_string(),
                    args: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "SPRITE_MON".to_string(),
                        "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "-1".to_string(),
                        "-1".to_string(),
                        "PAL_NPC_RED".to_string(),
                        "OBJECTTYPE_SCRIPT".to_string(),
                        "0".to_string(),
                        "MissingObjectScript".to_string(),
                        "-1".to_string(),
                    ],
                    command_index: 2,
                },
                &labels,
            ),
            vec![MapEventSectionCommandIssue::UnknownObjectEventScript {
                script: "MissingObjectScript".to_string()
            }]
        );
        assert_eq!(
            map_event_section_command_issues(
                &MapEventSectionCommand {
                    command: "object_event".to_string(),
                    args: vec![
                        "0".to_string(),
                        "0".to_string(),
                        "SPRITE_MON".to_string(),
                        "SPRITEMOVEDATA_STANDING_DOWN".to_string(),
                        "0".to_string(),
                        "0".to_string(),
                        "-1".to_string(),
                        "-1".to_string(),
                        "PAL_NPC_RED".to_string(),
                        "OBJECTTYPE_SCRIPT".to_string(),
                        "0".to_string(),
                        "ObjectEvent".to_string(),
                        "-1".to_string(),
                    ],
                    command_index: 3,
                },
                &labels,
            ),
            Vec::<MapEventSectionCommandIssue>::new()
        );
    }

    #[test]
    fn map_json_rejects_unknown_modpack_fields() {
        let error = serde_json::from_str::<MapAttributes>(
            r#"{
              "tileset_name":"johto",
              "border_block":5,
              "width":20,
              "height":18,
              "connections":[],
              "time_of_day":null,
              "phone_service":0,
              "phone_flag":false,
              "environment":null,
              "location":null,
              "music":null,
              "palette":null,
              "fishing_group":null,
              "map_constant":null,
              "map_group_constant":null,
              "blocks_label":null,
              "map_scripts_label":null,
              "map_events_label":null,
              "connection_flags":null,
              "fallback_blocks_label":"Route29_Blocks"
            }"#,
        )
        .expect_err("map attributes must not accept fallback fields")
        .to_string();
        assert!(
            error.contains("unknown field `fallback_blocks_label`"),
            "{error}"
        );

        let error = serde_json::from_str::<ObjectEvent>(
            r#"{
              "sprite":"SPRITE_YOUNGSTER",
              "sprite_has_facings":true,
              "x":4,
              "y":5,
              "spritemovedata":"SPRITEMOVEDATA_STANDING_DOWN",
              "move_range_x":0,
              "move_range_y":0,
              "hram_x":0,
              "hram_y":0,
              "pal":0,
              "object_type":"OBJECTTYPE_SCRIPT",
              "radius":0,
              "script":"Route29YoungsterScript",
              "label":null,
              "event_flag":"-1",
              "object_identifier":null,
              "sightline_direction_override":null,
              "legacy_sprite":"youngster"
            }"#,
        )
        .expect_err("object events must not accept legacy fields")
        .to_string();
        assert!(error.contains("unknown field `legacy_sprite`"), "{error}");

        let error = serde_json::from_value::<MapScriptSectionCommandIssue>(serde_json::json!({
            "unknown_scene_script": {
                "script": "Route29Scene",
                "fallback_script": "DefaultScene"
            }
        }))
        .expect_err("map script issues must not accept fallback scripts")
        .to_string();
        assert!(error.contains("unknown field `fallback_script`"), "{error}");

        let error = serde_json::from_value::<MapEventSectionCommandIssue>(serde_json::json!({
            "unknown_object_event_script": {
                "script": "Route29YoungsterScript",
                "legacy_script": "YoungsterScript"
            }
        }))
        .expect_err("map event issues must not accept legacy scripts")
        .to_string();
        assert!(error.contains("unknown field `legacy_script`"), "{error}");
    }

    #[test]
    fn map_json_rejects_malformed_pack_tokens_at_deserialization() {
        for (field, value) in [
            ("tileset_name", serde_json::json!("johto tileset")),
            ("time_of_day", serde_json::json!("day time")),
            ("environment", serde_json::json!("ROUTE AREA")),
            ("location", serde_json::json!("NEW BARK")),
            ("music", serde_json::json!("MUSIC-ROUTE_29")),
            ("palette", serde_json::json!("PALETTE DAY")),
            ("fishing_group", serde_json::json!("GROUP OLD_ROD")),
            ("map_constant", serde_json::json!("ROUTE 29")),
            ("map_group_constant", serde_json::json!("GROUP JOHTO")),
            ("blocks_label", serde_json::json!("Route29 Blocks")),
            ("map_scripts_label", serde_json::json!("Route29 Scripts")),
            ("map_events_label", serde_json::json!("Route29 Events")),
            ("connection_flags", serde_json::json!("NORTH-SOUTH")),
        ] {
            let mut attributes = valid_map_attributes_json();
            attributes[field] = value;

            let error = serde_json::from_value::<MapAttributes>(attributes)
                .expect_err("malformed map attribute tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be")
                    || error.contains("map expression token must be")
                    || error.contains("connection flags token must be"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("direction", serde_json::json!("north west")),
            ("target_map", serde_json::json!("Route 30")),
        ] {
            let mut connection = serde_json::json!({
                "direction": "north",
                "target_map": "Route30",
                "offset": 0
            });
            connection[field] = value;

            let error = serde_json::from_value::<MapConnection>(connection)
                .expect_err("malformed map connection tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be"),
                "connection {field} produced unexpected error: {error}"
            );
        }

        let mut warp = serde_json::json!({
            "index": 0,
            "x": 1,
            "y": 2,
            "target_map_constant": "ROUTE_29",
            "target_map": "Route29",
            "target_warp_id": 1
        });
        warp["target_map_constant"] = serde_json::json!("ROUTE 29");
        let error = serde_json::from_value::<WarpEvent>(warp)
            .expect_err("malformed warp tokens must fail during JSON load")
            .to_string();
        assert!(error.contains("map token must be"), "{error}");

        for (field, value) in [
            ("scene_id", serde_json::json!("SCENE ROUTE29")),
            ("script_name", serde_json::json!("Route29 Scene")),
        ] {
            let mut scene = serde_json::json!({
                "scene_id": "SCENE_ROUTE29",
                "script_name": "Route29Scene"
            });
            scene[field] = value;

            let error = serde_json::from_value::<MapScene>(scene)
                .expect_err("malformed map scene tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be"),
                "scene {field} produced unexpected error: {error}"
            );
        }

        for (label, payload) in [
            (
                "script command",
                serde_json::json!({
                    "command": "scene script",
                    "args": ["Route29Scene"],
                    "command_index": 0
                }),
            ),
            (
                "event command args",
                serde_json::json!({
                    "command": "bg_event",
                    "args": ["1", "2", "BGEVENT READ", "Route29Sign"],
                    "command_index": 0
                }),
            ),
        ] {
            let error = if label == "script command" {
                serde_json::from_value::<MapScriptSectionCommand>(payload)
                    .expect_err("malformed map script command tokens must fail during JSON load")
                    .to_string()
            } else {
                serde_json::from_value::<MapEventSectionCommand>(payload)
                    .expect_err("malformed map event command tokens must fail during JSON load")
                    .to_string()
            };
            assert!(
                error.contains("map token must be"),
                "{label} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("sprite", serde_json::json!("SPRITE YOUNGSTER")),
            (
                "spritemovedata",
                serde_json::json!("SPRITEMOVEDATA STANDING"),
            ),
            ("object_type", serde_json::json!("OBJECTTYPE SCRIPT")),
            ("script", serde_json::json!("Route29 Script")),
            ("label", serde_json::json!("Youngster Label")),
            ("event_flag", serde_json::json!("EVENT BEAT_JOEY")),
            ("object_identifier", serde_json::json!("OBJECT YOUNGSTER")),
            (
                "sightline_direction_override",
                serde_json::json!("north west"),
            ),
        ] {
            let mut object = valid_object_event_json();
            object[field] = value;

            let error = serde_json::from_value::<ObjectEvent>(object)
                .expect_err("malformed object event tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be"),
                "object {field} produced unexpected error: {error}"
            );
        }
    }

    #[test]
    fn map_json_rejects_reserved_pack_prefixes_at_deserialization() {
        for (field, value) in [
            ("tileset_name", serde_json::json!("fallback_tileset")),
            ("time_of_day", serde_json::json!("legacy_day")),
            ("environment", serde_json::json!("fallback_route")),
            ("location", serde_json::json!("legacy_new_bark")),
            ("music", serde_json::json!("fallback_music")),
            ("palette", serde_json::json!("legacy_palette")),
            ("fishing_group", serde_json::json!("fallback_fishing")),
            ("map_constant", serde_json::json!("legacy_route_29")),
            ("map_group_constant", serde_json::json!("fallback_group")),
            ("blocks_label", serde_json::json!("legacy_blocks")),
            ("map_scripts_label", serde_json::json!("fallback_scripts")),
            ("map_events_label", serde_json::json!("legacy_events")),
            (
                "connection_flags",
                serde_json::json!("fallback_connections"),
            ),
        ] {
            let mut attributes = valid_map_attributes_json();
            attributes[field] = value;

            let error = serde_json::from_value::<MapAttributes>(attributes)
                .expect_err("reserved map attribute tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be")
                    || error.contains("map expression token must be")
                    || error.contains("connection flags token must be"),
                "{field} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("direction", serde_json::json!("legacy_north")),
            ("target_map", serde_json::json!("fallback_route_30")),
        ] {
            let mut connection = serde_json::json!({
                "direction": "north",
                "target_map": "Route30",
                "offset": 0
            });
            connection[field] = value;

            let error = serde_json::from_value::<MapConnection>(connection)
                .expect_err("reserved map connection tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be"),
                "connection {field} produced unexpected error: {error}"
            );
        }

        for (label, payload) in [
            (
                "script command arg",
                serde_json::json!({
                    "command": "scene_script",
                    "args": ["fallback_scene"],
                    "command_index": 0
                }),
            ),
            (
                "event command",
                serde_json::json!({
                    "command": "legacy_bg_event",
                    "args": ["1", "2", "BGEVENT_READ", "Route29Sign"],
                    "command_index": 0
                }),
            ),
        ] {
            let error = if label == "script command arg" {
                serde_json::from_value::<MapScriptSectionCommand>(payload)
                    .expect_err("reserved map script command tokens must fail during JSON load")
                    .to_string()
            } else {
                serde_json::from_value::<MapEventSectionCommand>(payload)
                    .expect_err("reserved map event command tokens must fail during JSON load")
                    .to_string()
            };
            assert!(
                error.contains("map token must be"),
                "{label} produced unexpected error: {error}"
            );
        }

        for (field, value) in [
            ("sprite", serde_json::json!("fallback_sprite")),
            ("spritemovedata", serde_json::json!("legacy_move")),
            ("object_type", serde_json::json!("fallback_object")),
            ("script", serde_json::json!("legacy_script")),
            ("label", serde_json::json!("fallback_label")),
            ("event_flag", serde_json::json!("legacy_event")),
            ("object_identifier", serde_json::json!("fallback_object_id")),
            (
                "sightline_direction_override",
                serde_json::json!("legacy_north"),
            ),
        ] {
            let mut object = valid_object_event_json();
            object[field] = value;

            let error = serde_json::from_value::<ObjectEvent>(object)
                .expect_err("reserved object event tokens must fail during JSON load")
                .to_string();

            assert!(
                error.contains("map token must be"),
                "object {field} produced unexpected error: {error}"
            );
        }
    }

    fn valid_map_attributes_json() -> serde_json::Value {
        serde_json::json!({
            "tileset_name": "johto",
            "border_block": 5,
            "width": 20,
            "height": 18,
            "connections": [],
            "time_of_day": null,
            "phone_service": 0,
            "phone_flag": false,
            "environment": null,
            "location": null,
            "music": null,
            "palette": null,
            "fishing_group": null,
            "map_constant": null,
            "map_group_constant": null,
            "blocks_label": null,
            "map_scripts_label": null,
            "map_events_label": null,
            "connection_flags": null
        })
    }

    fn valid_object_event_json() -> serde_json::Value {
        serde_json::json!({
            "sprite": "SPRITE_YOUNGSTER",
            "sprite_has_facings": true,
            "x": 4,
            "y": 5,
            "spritemovedata": "SPRITEMOVEDATA_STANDING_DOWN",
            "move_range_x": 0,
            "move_range_y": 0,
            "hram_x": 0,
            "hram_y": 0,
            "pal": 0,
            "object_type": "OBJECTTYPE_SCRIPT",
            "radius": 0,
            "script": "Route29YoungsterScript",
            "label": null,
            "event_flag": "-1",
            "object_identifier": null,
            "sightline_direction_override": null
        })
    }
}
