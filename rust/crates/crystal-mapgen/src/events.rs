use std::collections::{BTreeMap, BTreeSet};

use crystal_assets::modpack::MapModule;
use crystal_core::{
    map::{BackgroundEvent, MapEventSectionCommand, ObjectEvent, WarpEvent},
    systems::script_control::ScriptControlCommand,
    systems::script_text::{ScriptTextBody, ScriptTextBodyCommand, ScriptTextCommand},
};
use serde_json::{Value, json};

use crate::{
    GeneratedGrid, GridLabel, MapCell,
    grid::{mart_origin, pokecenter_origin},
};

const MAX_SIGNS: usize = 6;
const MAX_RESIDENTS: usize = 5;
const TEXT_LINE_WIDTH: usize = 18;
const MAX_TEXT_LINES: usize = 4;

#[derive(Debug, Default)]
struct GeneratedEventBundle {
    warps: Vec<WarpEvent>,
    background_events: Vec<BackgroundEvent>,
    objects: Vec<ObjectEvent>,
    scripts: BTreeMap<String, Value>,
    control_commands: Vec<ScriptControlCommand>,
    text_commands: Vec<ScriptTextCommand>,
    text_bodies: BTreeMap<String, ScriptTextBody>,
    event_section_commands: Vec<MapEventSectionCommand>,
}

/// Adds map-local Crystal events without extending any global story catalog.
/// Signs use the northwest sign quadrant authored by GroundSign's block $45;
/// residents are always-visible map objects with ordinary talk scripts.
pub(crate) fn apply_generated_events(module: &mut MapModule, grid: &GeneratedGrid) {
    let bundle = generated_event_bundle(grid);
    module.events.warps = bundle.warps;
    module.events.bg_events = bundle.background_events;
    module.objects = bundle.objects;
    module.scripts.extend(bundle.scripts);
    module.script_control_commands = bundle.control_commands;
    module.script_text_commands = bundle.text_commands;
    module.script_text_bodies = bundle.text_bodies;
    module.map_event_section_commands = bundle.event_section_commands;
}

fn generated_event_bundle(grid: &GeneratedGrid) -> GeneratedEventBundle {
    let mut bundle = GeneratedEventBundle::default();
    let mut occupied_event_tiles = BTreeSet::new();

    if let Some((center_x, center_y)) = pokecenter_origin(grid) {
        bundle.warps.push(WarpEvent {
            index: 1,
            x: center_x * 2 + 1,
            y: center_y * 2 + 3,
            target_map_constant: "GENERATED_POKECENTER_1F".to_string(),
            target_map: "GeneratedPokecenter1F".to_string(),
            target_warp_id: 1,
        });
        let script = "GeneratedPokecenterSignScript".to_string();
        let text_label = "GeneratedPokecenterSignText".to_string();
        bundle.background_events.push(BackgroundEvent {
            x: center_x * 2 + 2,
            y: center_y * 2 + 3,
            event_type: "BGEVENT_UP".to_string(),
            script: script.clone(),
        });
        insert_text_script(
            &mut bundle,
            script,
            text_label,
            "jumptext",
            text_body_from_lines(["POKEMON CENTER", "HEAL YOUR #MON!"]),
        );
    }

    if let Some((mart_x, mart_y)) = mart_origin(grid) {
        bundle.warps.push(WarpEvent {
            index: u16::try_from(bundle.warps.len() + 1)
                .expect("generated facility warp count fits a u16"),
            x: mart_x * 2 + 1,
            y: mart_y * 2 + 3,
            target_map_constant: "GENERATED_MART_1F".to_string(),
            target_map: "GeneratedMart1F".to_string(),
            target_warp_id: 2,
        });
        let script = "GeneratedMartSignScript".to_string();
        bundle.background_events.push(BackgroundEvent {
            x: mart_x * 2 + 2,
            y: mart_y * 2 + 3,
            event_type: "BGEVENT_READ".to_string(),
            script: script.clone(),
        });
        bundle.scripts.insert(
            script.clone(),
            json!([{"command": "jumpstd", "args": ["MartSignScript"]}]),
        );
        bundle.control_commands.push(ScriptControlCommand {
            command: "jumpstd".to_string(),
            compare_value: None,
            target_label: Some("MartSignScript".to_string()),
            resolved_target_script: None,
            source_script: script,
            command_index: 0,
        });
    }

    for label in grid
        .labels
        .iter()
        .filter(|label| grid.cell(label.x, label.y) == Some(MapCell::GroundSign))
        .take(MAX_SIGNS)
    {
        let event_x = label.x * 2;
        let event_y = label.y * 2;
        if !occupied_event_tiles.insert((event_x, event_y)) {
            continue;
        }
        let number = bundle.background_events.len() + 1;
        let script = format!("GeneratedSign{number}Script");
        let text_label = format!("GeneratedSign{number}Text");
        bundle.background_events.push(BackgroundEvent {
            x: event_x,
            y: event_y,
            event_type: "BGEVENT_READ".to_string(),
            script: script.clone(),
        });
        insert_text_script(
            &mut bundle,
            script,
            text_label,
            "jumptext",
            label_text_body(label),
        );
    }

    for (index, (x, y)) in grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            (*cell == MapCell::TrashCan).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .take(4)
        .enumerate()
    {
        let event_x = x * 2 + 1;
        let event_y = y * 2 + 1;
        if !occupied_event_tiles.insert((event_x, event_y)) {
            continue;
        }
        let script = format!("GeneratedTrashCan{}Script", index + 1);
        bundle.background_events.push(BackgroundEvent {
            x: event_x,
            y: event_y,
            event_type: "BGEVENT_READ".to_string(),
            script: script.clone(),
        });
        bundle.scripts.insert(
            script.clone(),
            json!([{"command": "jumpstd", "args": ["TrashCanScript"]}]),
        );
        bundle.control_commands.push(ScriptControlCommand {
            command: "jumpstd".to_string(),
            compare_value: None,
            target_label: Some("TrashCanScript".to_string()),
            resolved_target_script: None,
            source_script: script,
            command_index: 0,
        });
    }

    let resident_messages = [
        ["HELLO! GREAT DAY", "TO BE OUTSIDE."],
        ["I LIKE EXPLORING", "EVERY LITTLE PATH."],
        ["THE AIR FEELS GREAT", "FOR A WALK TODAY."],
        ["TALL GRASS IS WHERE", "WILD #MON HIDE."],
        ["FLOWERS AND TREES", "MAKE A TOWN HAPPY."],
    ];
    let resident_sprites = [
        "SPRITE_LASS",
        "SPRITE_FISHER",
        "SPRITE_TEACHER",
        "SPRITE_YOUNGSTER",
        "SPRITE_GRAMPS",
    ];
    for ((x, y), (message, sprite)) in resident_positions(grid)
        .into_iter()
        .zip(resident_messages.into_iter().zip(resident_sprites))
    {
        let number = bundle.objects.len() + 1;
        let script = format!("GeneratedResident{number}Script");
        let text_label = format!("GeneratedResident{number}Text");
        let object_x = x * 2 + 1;
        let object_y = y * 2 + 1;
        bundle.objects.push(ObjectEvent {
            sprite: sprite.to_string(),
            sprite_has_facings: true,
            x: object_x,
            y: object_y,
            spritemovedata: "SPRITEMOVEDATA_WANDER".to_string(),
            move_range_x: 1,
            move_range_y: 1,
            hram_x: -1,
            hram_y: -1,
            pal: 0,
            object_type: "OBJECTTYPE_SCRIPT".to_string(),
            radius: 0,
            script: script.clone(),
            label: None,
            event_flag: "-1".to_string(),
            object_identifier: Some(format!("GENERATED_RESIDENT_{number}")),
            sightline_direction_override: None,
        });
        insert_text_script(
            &mut bundle,
            script,
            text_label,
            "jumptextfaceplayer",
            text_body_from_lines(message.iter().copied()),
        );
    }

    bundle.event_section_commands = event_section_commands(&bundle);
    bundle
}

fn insert_text_script(
    bundle: &mut GeneratedEventBundle,
    script: String,
    text_label: String,
    command: &str,
    mut body: ScriptTextBody,
) {
    body.label.clone_from(&text_label);
    bundle.scripts.insert(
        script.clone(),
        json!([{"command": command, "args": [text_label]}]),
    );
    bundle.text_commands.push(ScriptTextCommand {
        command: command.to_string(),
        text_label: Some(text_label.clone()),
        source_script: script,
        command_index: 0,
    });
    bundle.text_bodies.insert(text_label, body);
}

fn label_text_body(label: &GridLabel) -> ScriptTextBody {
    text_body_from_lines(wrap_sign_text(&label.text))
}

fn text_body_from_lines(lines: impl IntoIterator<Item = impl AsRef<str>>) -> ScriptTextBody {
    let lines = lines
        .into_iter()
        .map(|line| line.as_ref().to_string())
        .filter(|line| !line.is_empty())
        .take(MAX_TEXT_LINES)
        .collect::<Vec<_>>();
    let mut commands = Vec::with_capacity(lines.len() + 1);
    for (index, line) in lines.into_iter().enumerate() {
        let command = match index {
            0 => "text",
            1 | 3 => "line",
            _ => "para",
        };
        commands.push(ScriptTextBodyCommand {
            command: command.to_string(),
            args: vec![line],
            command_index: index,
        });
    }
    let command_index = commands.len();
    commands.push(ScriptTextBodyCommand {
        command: "done".to_string(),
        args: Vec::new(),
        command_index,
    });
    ScriptTextBody {
        label: "GeneratedText".to_string(),
        commands,
    }
}

fn wrap_sign_text(text: &str) -> Vec<String> {
    let cleaned = text
        .chars()
        .map(|character| {
            let character = character.to_ascii_uppercase();
            if character.is_ascii_alphanumeric()
                || matches!(character, ' ' | '-' | '\'' | '.' | '/' | '&')
            {
                character
            } else {
                ' '
            }
        })
        .collect::<String>();
    let words = cleaned.split_whitespace().collect::<Vec<_>>();
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let chunks = word.as_bytes().chunks(TEXT_LINE_WIDTH);
        for chunk in chunks {
            let chunk = std::str::from_utf8(chunk).expect("sanitized sign text is ASCII");
            if current.is_empty() {
                current.push_str(chunk);
            } else if current.len() + 1 + chunk.len() <= TEXT_LINE_WIDTH {
                current.push(' ');
                current.push_str(chunk);
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(chunk);
            }
            if lines.len() == MAX_TEXT_LINES {
                return lines;
            }
        }
    }
    if !current.is_empty() && lines.len() < MAX_TEXT_LINES {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push("LOCAL LANDMARK".to_string());
    }
    lines
}

fn resident_positions(grid: &GeneratedGrid) -> Vec<(u16, u16)> {
    let (home_x, home_y) = grid.home_cell();
    let mut positions = Vec::new();

    // Put the first resident inside the authored public field. A destination
    // large enough to contain benches and a sign should also contain a person,
    // so its gameplay-scale view feels active instead of decorative.
    let pitch_cells = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            (*cell == MapCell::Pitch).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect::<Vec<_>>();
    if !pitch_cells.is_empty() {
        let center_x =
            pitch_cells.iter().map(|(x, _)| u32::from(*x)).sum::<u32>() / pitch_cells.len() as u32;
        let center_y =
            pitch_cells.iter().map(|(_, y)| u32::from(*y)).sum::<u32>() / pitch_cells.len() as u32;
        if let Some((x, y)) = pitch_cells
            .into_iter()
            .filter(|&(x, y)| npc_area_is_clear(grid, x, y))
            .min_by_key(|&(x, y)| {
                (
                    u32::from(x).abs_diff(center_x) + u32::from(y).abs_diff(center_y),
                    u32::from(x).wrapping_mul(73_856_093) ^ u32::from(y).wrapping_mul(19_349_663),
                )
            })
        {
            positions.push((x, y));
        }
    }

    // Prefer one safe path-side position near water, but keep a full movement
    // square between the resident and the shore collision.
    let mut shore_candidates = Vec::new();
    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            if !npc_area_is_clear(grid, x, y)
                || !matches!(
                    grid.cell(x, y),
                    Some(MapCell::Trail | MapCell::Lawn | MapCell::Grass)
                )
            {
                continue;
            }
            let mut nearest_water = u16::MAX;
            for water_y in y.saturating_sub(4)..=(y + 4).min(grid.height - 1) {
                for water_x in x.saturating_sub(4)..=(x + 4).min(grid.width - 1) {
                    if matches!(
                        grid.cell(water_x, water_y),
                        Some(
                            MapCell::Water
                                | MapCell::WaterAccessEast
                                | MapCell::WaterAccessWest
                                | MapCell::WaterAccessSouth
                        )
                    ) {
                        nearest_water =
                            nearest_water.min(x.abs_diff(water_x) + y.abs_diff(water_y));
                    }
                }
            }
            if (2..=4).contains(&nearest_water) {
                shore_candidates.push((
                    nearest_water,
                    u32::from(x).wrapping_mul(73_856_093) ^ u32::from(y).wrapping_mul(19_349_663),
                    x,
                    y,
                ));
            }
        }
    }
    shore_candidates.sort_unstable();
    if let Some((_, _, x, y)) = shore_candidates.into_iter().find(|&(_, _, x, y)| {
        positions.iter().all(|&(other_x, other_y)| {
            i32::from(other_x).abs_diff(i32::from(x)) >= 8
                || i32::from(other_y).abs_diff(i32::from(y)) >= 8
        })
    }) {
        positions.push((x, y));
    }

    let mut candidates = Vec::new();
    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            if !npc_ground(grid.cell(x, y))
                || i32::from(home_x).abs_diff(i32::from(x)) < 5
                    && i32::from(home_y).abs_diff(i32::from(y)) < 5
                || grid.labels.iter().any(|label| {
                    i32::from(label.x).abs_diff(i32::from(x)) < 3
                        && i32::from(label.y).abs_diff(i32::from(y)) < 3
                })
                || !npc_area_is_clear(grid, x, y)
            {
                continue;
            }
            let borders_path = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| {
                    grid.cell((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16)
                        == Some(MapCell::Trail)
                })
            });
            let stable_order =
                u32::from(x).wrapping_mul(73_856_093) ^ u32::from(y).wrapping_mul(19_349_663);
            candidates.push((!borders_path, stable_order, x, y));
        }
    }
    candidates.sort_unstable();
    for (_, _, x, y) in candidates {
        if positions.iter().any(|&(other_x, other_y)| {
            i32::from(other_x).abs_diff(i32::from(x)) < 8
                && i32::from(other_y).abs_diff(i32::from(y)) < 8
        }) {
            continue;
        }
        positions.push((x, y));
        if positions.len() == MAX_RESIDENTS {
            break;
        }
    }
    positions
}

fn npc_ground(cell: Option<MapCell>) -> bool {
    matches!(
        cell,
        Some(
            MapCell::Grass
                | MapCell::Lawn
                | MapCell::Clearing
                | MapCell::Flowers
                | MapCell::Pitch
                | MapCell::Trail
                | MapCell::Street
                | MapCell::Road
                | MapCell::MajorRoad
        )
    )
}

fn npc_area_is_clear(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    (-1..=1).all(|dy| {
        (-1..=1).all(|dx| {
            let check_x = i32::from(x) + dx;
            let check_y = i32::from(y) + dy;
            check_x >= 0
                && check_y >= 0
                && check_x < i32::from(grid.width)
                && check_y < i32::from(grid.height)
                && npc_ground(grid.cell(check_x as u16, check_y as u16))
        })
    })
}

fn event_section_commands(bundle: &GeneratedEventBundle) -> Vec<MapEventSectionCommand> {
    let mut commands = Vec::new();
    let mut push = |command: &str, args: Vec<String>| {
        let command_index = commands.len();
        commands.push(MapEventSectionCommand {
            command: command.to_string(),
            args,
            command_index,
        });
    };
    push("def_warp_events", Vec::new());
    for event in &bundle.warps {
        push(
            "warp_event",
            vec![
                event.x.to_string(),
                event.y.to_string(),
                event.target_map_constant.clone(),
                event.target_warp_id.to_string(),
            ],
        );
    }
    push("def_coord_events", Vec::new());
    push("def_bg_events", Vec::new());
    for event in &bundle.background_events {
        push(
            "bg_event",
            vec![
                event.x.to_string(),
                event.y.to_string(),
                event.event_type.clone(),
                event.script.clone(),
            ],
        );
    }
    push("def_object_events", Vec::new());
    for object in &bundle.objects {
        push(
            "object_event",
            vec![
                object.x.to_string(),
                object.y.to_string(),
                object.sprite.clone(),
                object.spritemovedata.clone(),
                object.move_range_x.to_string(),
                object.move_range_y.to_string(),
                object.hram_x.to_string(),
                object.hram_y.to_string(),
                object.pal.to_string(),
                object.object_type.clone(),
                object.radius.to_string(),
                object.script.clone(),
                object.event_flag.clone(),
            ],
        );
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate, MapSource};
    use crystal_core::{
        map::map_event_section_command_issues,
        systems::script_control::script_control_command_issues,
        systems::script_text::{script_text_body_issues, script_text_command_issues},
    };

    fn grid_with_labels(labels: Vec<GridLabel>) -> GeneratedGrid {
        let width = 24;
        let height = 24;
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: Coordinate { lat: 0.0, lon: 0.0 },
                bounds: BoundingBox {
                    south: -0.1,
                    west: -0.1,
                    north: 0.1,
                    east: 0.1,
                },
                attribution: "test".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width,
            height,
            cells: vec![MapCell::Lawn; usize::from(width) * usize::from(height)],
            labels,
        };
        for label in &grid.labels {
            let index = usize::from(label.y) * usize::from(width) + usize::from(label.x);
            grid.cells[index] = MapCell::GroundSign;
        }
        grid
    }

    #[test]
    fn ground_sign_labels_compile_to_exact_read_events_and_text_scripts() {
        let grid = grid_with_labels(vec![GridLabel {
            text: "Cedar Lake Regional Trail".to_string(),
            x: 4,
            y: 5,
        }]);
        let bundle = generated_event_bundle(&grid);
        assert_eq!(
            bundle.background_events,
            vec![BackgroundEvent {
                x: 8,
                y: 10,
                event_type: "BGEVENT_READ".to_string(),
                script: "GeneratedSign1Script".to_string(),
            }]
        );
        assert_eq!(
            bundle.scripts["GeneratedSign1Script"],
            json!([{"command":"jumptext","args":["GeneratedSign1Text"]}])
        );
        let body = &bundle.text_bodies["GeneratedSign1Text"];
        assert_eq!(body.commands[0].args, ["CEDAR LAKE"]);
        assert_eq!(body.commands[1].args, ["REGIONAL TRAIL"]);
    }

    #[test]
    fn pokecenter_facade_compiles_to_matching_warp_and_readable_sign() {
        let mut grid = grid_with_labels(Vec::new());
        for (x, y, cell) in [
            (8_u16, 7_u16, MapCell::PokecenterNorthWest),
            (9, 7, MapCell::PokecenterNorthEast),
            (8, 8, MapCell::PokecenterSouthWest),
            (9, 8, MapCell::PokecenterSouthEast),
        ] {
            grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
        }
        let bundle = generated_event_bundle(&grid);
        assert_eq!(bundle.warps.len(), 1);
        assert_eq!((bundle.warps[0].x, bundle.warps[0].y), (17, 17));
        assert_eq!(
            bundle.warps[0].target_map_constant,
            "GENERATED_POKECENTER_1F"
        );
        assert!(bundle.background_events.iter().any(|event| {
            event.x == 18 && event.y == 17 && event.script == "GeneratedPokecenterSignScript"
        }));
    }

    #[test]
    fn mart_facade_compiles_to_exact_door_warp_and_canonical_sign() {
        let mut grid = grid_with_labels(Vec::new());
        for (x, y, cell) in [
            (8_u16, 7_u16, MapCell::MartNorthWest),
            (9, 7, MapCell::MartNorthEast),
            (8, 8, MapCell::MartSouthWest),
            (9, 8, MapCell::MartSouthEast),
        ] {
            grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
        }
        let bundle = generated_event_bundle(&grid);
        assert_eq!(bundle.warps.len(), 1);
        assert_eq!(bundle.warps[0].index, 1);
        assert_eq!((bundle.warps[0].x, bundle.warps[0].y), (17, 17));
        assert_eq!(bundle.warps[0].target_map_constant, "GENERATED_MART_1F");
        assert_eq!(bundle.warps[0].target_map, "GeneratedMart1F");
        assert_eq!(bundle.warps[0].target_warp_id, 2);
        assert!(bundle.background_events.iter().any(|event| {
            event.x == 18
                && event.y == 17
                && event.event_type == "BGEVENT_READ"
                && event.script == "GeneratedMartSignScript"
        }));
        assert_eq!(
            bundle.scripts["GeneratedMartSignScript"],
            json!([{"command": "jumpstd", "args": ["MartSignScript"]}])
        );
        assert!(bundle.control_commands.iter().any(|command| {
            command.command == "jumpstd"
                && command.source_script == "GeneratedMartSignScript"
                && command.target_label.as_deref() == Some("MartSignScript")
                && command.command_index == 0
        }));
    }

    #[test]
    fn facility_events_are_emitted_only_for_complete_facades() {
        for (has_center, has_mart) in [(false, false), (true, false), (false, true), (true, true)] {
            let mut grid = grid_with_labels(Vec::new());
            if has_center {
                for (x, y, cell) in [
                    (3_u16, 3_u16, MapCell::PokecenterNorthWest),
                    (4, 3, MapCell::PokecenterNorthEast),
                    (3, 4, MapCell::PokecenterSouthWest),
                    (4, 4, MapCell::PokecenterSouthEast),
                ] {
                    grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
                }
            }
            if has_mart {
                for (x, y, cell) in [
                    (8_u16, 3_u16, MapCell::MartNorthWest),
                    (9, 3, MapCell::MartNorthEast),
                    (8, 4, MapCell::MartSouthWest),
                    (9, 4, MapCell::MartSouthEast),
                ] {
                    grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
                }
            }

            let bundle = generated_event_bundle(&grid);
            let facility_targets = bundle
                .warps
                .iter()
                .map(|warp| warp.target_map_constant.as_str())
                .collect::<Vec<_>>();
            let expected_targets = match (has_center, has_mart) {
                (false, false) => Vec::new(),
                (true, false) => vec!["GENERATED_POKECENTER_1F"],
                (false, true) => vec!["GENERATED_MART_1F"],
                (true, true) => vec!["GENERATED_POKECENTER_1F", "GENERATED_MART_1F"],
            };
            assert_eq!(facility_targets, expected_targets);
            assert_eq!(
                bundle
                    .warps
                    .iter()
                    .map(|warp| warp.index)
                    .collect::<Vec<_>>(),
                (1..=u16::try_from(bundle.warps.len()).unwrap()).collect::<Vec<_>>()
            );
            assert_eq!(
                bundle
                    .background_events
                    .iter()
                    .any(|event| event.script == "GeneratedPokecenterSignScript"),
                has_center
            );
            assert_eq!(
                bundle
                    .background_events
                    .iter()
                    .any(|event| event.script == "GeneratedMartSignScript"),
                has_mart
            );
        }

        let mut partial = grid_with_labels(Vec::new());
        partial.cells[3 * usize::from(partial.width) + 3] = MapCell::PokecenterNorthWest;
        assert!(
            generated_event_bundle(&partial).warps.is_empty(),
            "an incomplete facade must not emit a door into a nonexistent interior"
        );
    }

    #[test]
    fn generated_text_and_event_records_pass_core_shape_validation() {
        let grid = grid_with_labels(vec![GridLabel {
            text: "Bde Maka Ska".to_string(),
            x: 4,
            y: 5,
        }]);
        let bundle = generated_event_bundle(&grid);
        let labels = bundle.text_bodies.keys().cloned().collect::<BTreeSet<_>>();
        for (key, body) in &bundle.text_bodies {
            assert!(script_text_body_issues(key, body).is_empty());
        }
        for command in &bundle.text_commands {
            assert!(script_text_command_issues(command, &labels).is_empty());
        }
        let scripts = bundle.scripts.keys().cloned().collect::<BTreeSet<_>>();
        for command in &bundle.event_section_commands {
            assert!(map_event_section_command_issues(command, &scripts).is_empty());
        }
    }

    #[test]
    fn residents_are_generic_always_visible_map_local_objects() {
        let bundle = generated_event_bundle(&grid_with_labels(Vec::new()));
        assert_eq!(bundle.objects.len(), MAX_RESIDENTS);
        assert!(bundle.objects.iter().all(|object| {
            object.object_type == "OBJECTTYPE_SCRIPT"
                && object.spritemovedata == "SPRITEMOVEDATA_WANDER"
                && object.event_flag == "-1"
                && object.script.starts_with("GeneratedResident")
        }));
        let positions = bundle
            .objects
            .iter()
            .map(|object| (object.x, object.y))
            .collect::<BTreeSet<_>>();
        assert_eq!(positions.len(), MAX_RESIDENTS);
    }

    #[test]
    fn authored_public_field_gets_a_visible_wandering_resident() {
        let mut grid = grid_with_labels(Vec::new());
        for y in 3_u16..=8 {
            for x in 3_u16..=10 {
                grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] =
                    MapCell::Pitch;
            }
        }
        let bundle = generated_event_bundle(&grid);
        assert!(bundle.objects.iter().any(|object| {
            let block_x = (object.x - 1) / 2;
            let block_y = (object.y - 1) / 2;
            grid.cell(block_x, block_y) == Some(MapCell::Pitch)
        }));
    }

    #[test]
    fn outdoor_trash_can_uses_its_southeast_quadrant_and_canonical_standard_script() {
        let mut grid = grid_with_labels(Vec::new());
        grid.cells[7 * usize::from(grid.width) + 8] = MapCell::TrashCan;
        let bundle = generated_event_bundle(&grid);
        assert!(bundle.background_events.iter().any(|event| {
            event.x == 17
                && event.y == 15
                && event.event_type == "BGEVENT_READ"
                && event.script == "GeneratedTrashCan1Script"
        }));
        assert_eq!(
            bundle.scripts["GeneratedTrashCan1Script"],
            json!([{"command": "jumpstd", "args": ["TrashCanScript"]}])
        );
        let labels = bundle.scripts.keys().cloned().collect::<BTreeSet<_>>();
        assert_eq!(bundle.control_commands.len(), 1);
        assert!(script_control_command_issues(&bundle.control_commands[0], &labels).is_empty());
    }

    #[test]
    fn osm_sign_text_is_ascii_bounded_and_page_wrapped() {
        let lines = wrap_sign_text("  Bdé Maka Ska—Northeast Walking Promenade  ");
        assert!(lines.len() <= MAX_TEXT_LINES);
        assert!(lines.iter().all(|line| {
            !line.is_empty()
                && line.len() <= TEXT_LINE_WIDTH
                && line.is_ascii()
                && !line.chars().any(char::is_control)
        }));
    }
}
