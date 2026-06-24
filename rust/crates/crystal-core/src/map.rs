use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapConnection {
    pub direction: String,
    pub target_map: String,
    pub offset: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarpEvent {
    pub index: u16,
    pub x: u16,
    pub y: u16,
    pub target_map_constant: String,
    pub target_map: String,
    pub target_warp_id: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoordEvent {
    pub x: u16,
    pub y: u16,
    pub scene_id: String,
    pub script_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundEvent {
    pub x: u16,
    pub y: u16,
    pub event_type: String,
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
    pub scene_id: String,
    pub script_name: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapSceneTable {
    pub scenes: Vec<MapScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapScriptSectionCommand {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapEventSectionCommand {
    pub command: String,
    pub args: Vec<String>,
    pub command_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectEvent {
    pub sprite: String,
    pub x: u16,
    pub y: u16,
    pub spritemovedata: String,
    pub move_range_x: u16,
    pub move_range_y: u16,
    pub hram_x: i16,
    pub hram_y: i16,
    pub pal: u8,
    pub object_type: String,
    pub radius: u16,
    pub script: String,
    pub label: Option<String>,
    pub event_flag: String,
    pub object_identifier: Option<String>,
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
    }
}
