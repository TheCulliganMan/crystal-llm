use std::{fs, path::Path};

use anyhow::{Context, Result, ensure};
use crystal_assets::{
    RuntimeMapMetadata, RuntimeSpawnPoint,
    modpack::{CompiledMapExtension, MapModule},
    read_verified_compiled_game_pack,
};
use crystal_core::map::{
    MapAttributes, MapEventSectionCommand, MapEvents, MapSceneTable, MapScriptSectionCommand,
};
use crystal_core::world::encounters::{WildEncounter, WildEncounterTable, WildEncounterZone};
use serde::{Deserialize, Serialize};

use crate::{
    GENERATED_TILESET_ID, GeneratedGrid, H3Facility,
    build_johto_modern_generated_tileset_extension, events::apply_generated_events,
};

const GENERATED_MAP_NAME: &str = "GeneratedNeighborhood";
const GENERATED_MAP_CONSTANT: &str = "GENERATED_NEIGHBORHOOD";
const GENERATED_GROUP_NAME: &str = "GENERATED_REGION";
const GENERATED_GROUP_ID: u16 = 250;
const GENERATED_MAP_ID: u16 = 1;
const GENERATED_SPAWN_ID: u16 = 65_000;
const GENERATED_POKECENTER_NAME: &str = "GeneratedPokecenter1F";
const GENERATED_POKECENTER_CONSTANT: &str = "GENERATED_POKECENTER_1F";
const GENERATED_POKECENTER_MAP_ID: u16 = 2;
const GENERATED_POKECENTER_SPAWN_ID: u16 = 65_001;
const GENERATED_POKECENTER_NURSE_SCRIPT: &str = "GeneratedPokecenter1FNurseScript";
const GENERATED_POKECENTER_NURSE_OBJECT: &str = "GENERATEDPOKECENTER1F_NURSE";
const POKECENTER_EXIT_WARP_COORDINATES: [(u16, u16); 2] = [(3, 7), (4, 7)];
const POKECENTER_2F_WARP_COORDINATE: (u16, u16) = (0, 7);
const GENERATED_MART_NAME: &str = "GeneratedMart1F";
const GENERATED_MART_CONSTANT: &str = "GENERATED_MART_1F";
const GENERATED_MART_MAP_ID: u16 = 3;
const GENERATED_MART_SPAWN_ID: u16 = 65_002;
const GENERATED_MART_CLERK_SCRIPT: &str = "GeneratedMart1FClerkScript";
const GENERATED_MART_CLERK_OBJECT: &str = "GENERATEDMART1F_CLERK";
const MART_EXIT_WARP_COORDINATES: [(u16, u16); 2] = [(2, 7), (3, 7)];

#[derive(Debug, Clone)]
pub struct ModpackOptions<'a> {
    pub base_pack: &'a Path,
    pub output_pack: &'a Path,
    pub manifest_id: &'a str,
    pub start_new_game_here: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedModpack {
    pub path: String,
    pub map_name: String,
    pub map_constant: String,
    pub spawn_identifier: u16,
    pub runtime_tile_x: i16,
    pub runtime_tile_y: i16,
}

pub fn build_modpack(
    grid: &GeneratedGrid,
    options: ModpackOptions<'_>,
) -> Result<GeneratedModpack> {
    let base_pack = options
        .base_pack
        .canonicalize()
        .with_context(|| format!("resolve base pack {}", options.base_pack.display()))?;
    let base = read_verified_compiled_game_pack(&base_pack)
        .with_context(|| format!("load verified base pack {}", base_pack.display()))?;
    let has_pokecenter = crate::grid::pokecenter_origin(grid).is_some();
    let has_mart = crate::grid::mart_origin(grid).is_some();
    let requests_pokecenter = grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.requests_facility(H3Facility::PokemonCenter));
    let requests_mart = grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.requests_facility(H3Facility::Mart));
    ensure!(
        has_pokecenter == requests_pokecenter,
        "facility plan requests Pokemon Center={requests_pokecenter}, but complete facade present={has_pokecenter}"
    );
    ensure!(
        has_mart == requests_mart,
        "facility plan requests Pokemon Mart={requests_mart}, but complete facade present={has_mart}"
    );
    if requests_mart {
        ensure!(
            base.data().marts.0.contains_key("MART_VIOLET"),
            "base pack is missing the canonical MART_VIOLET inventory"
        );
    }
    let tileset_extension = build_johto_modern_generated_tileset_extension(
        &base,
        format!("{}-tileset", options.manifest_id),
    )?;
    let base = base.with_tileset_extension(tileset_extension)?;
    let template = base
        .data()
        .maps
        .get("GoldenrodCity")
        .context("base pack is missing GoldenrodCity map template")?;
    let pokecenter_template = if requests_pokecenter {
        Some(
            base.data()
                .maps
                .get("MahoganyPokecenter1F")
                .context("base pack is missing MahoganyPokecenter1F map template")?,
        )
    } else {
        None
    };
    let mart_template = if requests_mart {
        Some(
            base.data()
                .maps
                .get("VioletMart")
                .context("base pack is missing VioletMart map template")?,
        )
    } else {
        None
    };
    let mut wild_encounters = base
        .data()
        .wild_encounters
        .get("Route34")
        .cloned()
        .context("base pack is missing canonical Route34 wild encounters")?;
    wild_encounters.map_name = GENERATED_MAP_NAME.to_string();
    wild_encounters.zones = generated_biome_encounter_zones(grid);
    let module = generated_module(template, grid)?;
    let exterior_mart_warp_id = if requests_mart {
        Some(
            module
                .events
                .warps
                .iter()
                .find(|warp| warp.target_map_constant == GENERATED_MART_CONSTANT)
                .map(|warp| warp.index)
                .context("generated exterior is missing its Mart door warp")?,
        )
    } else {
        None
    };
    let (home_block_x, home_block_y) = grid.home_cell();
    let runtime_tile_x = i16::try_from(home_block_x * 2 + 1)?;
    let runtime_tile_y = i16::try_from(home_block_y * 2 + 1)?;
    let spawn = RuntimeSpawnPoint {
        identifier: GENERATED_SPAWN_ID,
        map_constant: GENERATED_MAP_CONSTANT.to_string(),
        map_name: GENERATED_MAP_NAME.to_string(),
        group_id: i16::try_from(GENERATED_GROUP_ID)?,
        map_id: i16::try_from(GENERATED_MAP_ID)?,
        tile_x: runtime_tile_x,
        tile_y: runtime_tile_y,
        group_name: GENERATED_GROUP_NAME.to_string(),
        metatile_x: runtime_tile_x / 2,
        metatile_y: runtime_tile_y / 2,
        subtile_x: runtime_tile_x.rem_euclid(2),
        subtile_y: runtime_tile_y.rem_euclid(2),
    };
    let metadata = RuntimeMapMetadata {
        constant: GENERATED_MAP_CONSTANT.to_string(),
        name: GENERATED_MAP_NAME.to_string(),
        group_name: GENERATED_GROUP_NAME.to_string(),
        group_id: GENERATED_GROUP_ID,
        map_id: GENERATED_MAP_ID,
        width: grid.width,
        height: grid.height,
        environment: "TOWN".to_string(),
        phone_service: 0,
    };
    let exterior_extension = CompiledMapExtension {
        manifest_id: options.manifest_id.to_string(),
        map_name: GENERATED_MAP_NAME.to_string(),
        map_constant: GENERATED_MAP_CONSTANT.to_string(),
        module,
        metadata,
        spawn_key: GENERATED_SPAWN_ID.to_string(),
        spawn,
        wild_encounters: Some(wild_encounters),
        start_new_game_here: options.start_new_game_here,
    };
    let mut extensions = vec![exterior_extension];
    if let Some(pokecenter_template) = pokecenter_template {
        let pokecenter_module = generated_pokecenter_module(pokecenter_template)?;
        let pokecenter_spawn = RuntimeSpawnPoint {
            identifier: GENERATED_POKECENTER_SPAWN_ID,
            map_constant: GENERATED_POKECENTER_CONSTANT.to_string(),
            map_name: GENERATED_POKECENTER_NAME.to_string(),
            group_id: i16::try_from(GENERATED_GROUP_ID)?,
            map_id: i16::try_from(GENERATED_POKECENTER_MAP_ID)?,
            tile_x: 3,
            tile_y: 6,
            group_name: GENERATED_GROUP_NAME.to_string(),
            metatile_x: 1,
            metatile_y: 3,
            subtile_x: 1,
            subtile_y: 0,
        };
        extensions.push(CompiledMapExtension {
            manifest_id: format!("{}-pokecenter", options.manifest_id),
            map_name: GENERATED_POKECENTER_NAME.to_string(),
            map_constant: GENERATED_POKECENTER_CONSTANT.to_string(),
            metadata: RuntimeMapMetadata {
                constant: GENERATED_POKECENTER_CONSTANT.to_string(),
                name: GENERATED_POKECENTER_NAME.to_string(),
                group_name: GENERATED_GROUP_NAME.to_string(),
                group_id: GENERATED_GROUP_ID,
                map_id: GENERATED_POKECENTER_MAP_ID,
                width: pokecenter_module.attributes.width,
                height: pokecenter_module.attributes.height,
                environment: "INDOOR".to_string(),
                phone_service: 0,
            },
            module: pokecenter_module,
            spawn_key: GENERATED_POKECENTER_SPAWN_ID.to_string(),
            spawn: pokecenter_spawn,
            wild_encounters: None,
            start_new_game_here: false,
        });
    }
    if let Some(mart_template) = mart_template {
        let exterior_mart_warp_id = exterior_mart_warp_id
            .context("generated exterior is missing its allocated Mart door warp")?;
        let mart_module = generated_mart_module(mart_template, exterior_mart_warp_id)?;
        let mart_spawn = RuntimeSpawnPoint {
            identifier: GENERATED_MART_SPAWN_ID,
            map_constant: GENERATED_MART_CONSTANT.to_string(),
            map_name: GENERATED_MART_NAME.to_string(),
            group_id: i16::try_from(GENERATED_GROUP_ID)?,
            map_id: i16::try_from(GENERATED_MART_MAP_ID)?,
            tile_x: 3,
            tile_y: 6,
            group_name: GENERATED_GROUP_NAME.to_string(),
            metatile_x: 1,
            metatile_y: 3,
            subtile_x: 1,
            subtile_y: 0,
        };
        extensions.push(CompiledMapExtension {
            manifest_id: format!("{}-mart", options.manifest_id),
            map_name: GENERATED_MART_NAME.to_string(),
            map_constant: GENERATED_MART_CONSTANT.to_string(),
            metadata: RuntimeMapMetadata {
                constant: GENERATED_MART_CONSTANT.to_string(),
                name: GENERATED_MART_NAME.to_string(),
                group_name: GENERATED_GROUP_NAME.to_string(),
                group_id: GENERATED_GROUP_ID,
                map_id: GENERATED_MART_MAP_ID,
                width: mart_module.attributes.width,
                height: mart_module.attributes.height,
                environment: "INDOOR".to_string(),
                phone_service: 0,
            },
            module: mart_module,
            spawn_key: GENERATED_MART_SPAWN_ID.to_string(),
            spawn: mart_spawn,
            wild_encounters: None,
            start_new_game_here: false,
        });
    }
    let extended = base.with_map_extensions(extensions)?;
    if let Some(parent) = options.output_pack.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create output directory {}", parent.display()))?;
    }
    extended
        .write_preserving_storage(options.output_pack)
        .with_context(|| format!("write generated modpack {}", options.output_pack.display()))?;
    Ok(GeneratedModpack {
        path: options.output_pack.display().to_string(),
        map_name: GENERATED_MAP_NAME.to_string(),
        map_constant: GENERATED_MAP_CONSTANT.to_string(),
        spawn_identifier: GENERATED_SPAWN_ID,
        runtime_tile_x,
        runtime_tile_y,
    })
}

fn generated_biome_encounter_zones(grid: &GeneratedGrid) -> Vec<WildEncounterZone> {
    use std::collections::{BTreeMap, VecDeque};

    fn table(species: [&str; 7]) -> WildEncounterTable {
        let slots = species
            .into_iter()
            .enumerate()
            .map(|(slot, species)| WildEncounter {
                level: 12 + u8::try_from(slot / 2).expect("seven encounter slots fit u8"),
                species: species.to_string(),
            })
            .collect::<Vec<_>>();
        WildEncounterTable {
            morning: slots.clone(),
            day: slots.clone(),
            night: slots,
        }
    }

    let width = usize::from(grid.width);
    let mut seen = vec![false; grid.cells.len()];
    let mut zones = Vec::new();
    for start in 0..grid.cells.len() {
        let kind = match grid.cells[start] {
            crate::MapCell::IceFloor => "ice_surface",
            crate::MapCell::RockFloor => "rock_surface",
            _ => continue,
        };
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut queue = VecDeque::from([start]);
        let (mut min_x, mut min_y, mut max_x, mut max_y) =
            (start % width, start / width, start % width, start / width);
        while let Some(index) = queue.pop_front() {
            let x = index % width;
            let y = index / width;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx >= width || ny >= usize::from(grid.height) {
                    continue;
                }
                let next = ny * width + nx;
                let same = matches!(
                    (kind, grid.cells[next]),
                    ("ice_surface", crate::MapCell::IceFloor)
                        | ("rock_surface", crate::MapCell::RockFloor)
                );
                if same && !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }
        let grass = if kind == "ice_surface" {
            table([
                "SWINUB", "SWINUB", "SNEASEL", "DELIBIRD", "JYNX", "ZUBAT", "GOLBAT",
            ])
        } else {
            table([
                "GEODUDE",
                "GEODUDE",
                "SANDSHREW",
                "CUBONE",
                "ONIX",
                "GRAVELER",
                "RHYHORN",
            ])
        };
        zones.push(WildEncounterZone {
            id: format!("{kind}_{}", zones.len() + 1),
            min_x: i16::try_from(min_x * 2).expect("generated runtime x fits i16"),
            min_y: i16::try_from(min_y * 2).expect("generated runtime y fits i16"),
            max_x: i16::try_from(max_x * 2 + 1).expect("generated runtime x fits i16"),
            max_y: i16::try_from(max_y * 2 + 1).expect("generated runtime y fits i16"),
            grass_rates: BTreeMap::from([
                ("morning".to_string(), 12),
                ("day".to_string(), 12),
                ("night".to_string(), 14),
            ]),
            grass,
        });
    }
    zones
}

fn generated_module(template: &MapModule, grid: &GeneratedGrid) -> Result<MapModule> {
    let pokecenter_cells = grid
        .cells
        .iter()
        .filter(|cell| {
            matches!(
                **cell,
                crate::MapCell::PokecenterNorthWest
                    | crate::MapCell::PokecenterNorthEast
                    | crate::MapCell::PokecenterSouthWest
                    | crate::MapCell::PokecenterSouthEast
            )
        })
        .count();
    let mart_cells = grid
        .cells
        .iter()
        .filter(|cell| {
            matches!(
                **cell,
                crate::MapCell::MartNorthWest
                    | crate::MapCell::MartNorthEast
                    | crate::MapCell::MartSouthWest
                    | crate::MapCell::MartSouthEast
            )
        })
        .count();
    let has_pokecenter = crate::grid::pokecenter_origin(grid).is_some();
    let has_mart = crate::grid::mart_origin(grid).is_some();
    ensure!(
        (pokecenter_cells == 0 && !has_pokecenter) || (pokecenter_cells == 4 && has_pokecenter),
        "generated exterior must contain zero or one complete Pokemon Center facade, found {pokecenter_cells} facade tiles"
    );
    ensure!(
        (mart_cells == 0 && !has_mart) || (mart_cells == 4 && has_mart),
        "generated exterior must contain zero or one complete Pokemon Mart facade, found {mart_cells} facade tiles"
    );

    let mut module = template.clone();
    module.id = GENERATED_MAP_NAME.to_string();
    module.attributes = MapAttributes {
        tileset_name: GENERATED_TILESET_ID.to_string(),
        border_block: 0x01,
        width: grid.width,
        height: grid.height,
        connections: Vec::new(),
        time_of_day: None,
        phone_service: 0,
        phone_flag: false,
        environment: Some("TOWN".to_string()),
        location: Some("LANDMARK_GOLDENROD_CITY".to_string()),
        music: Some("MUSIC_GOLDENROD_CITY".to_string()),
        palette: Some("PALETTE_AUTO".to_string()),
        fishing_group: Some("FISHGROUP_SHORE".to_string()),
        map_constant: Some(GENERATED_MAP_CONSTANT.to_string()),
        map_group_constant: Some(GENERATED_GROUP_NAME.to_string()),
        blocks_label: Some("GeneratedNeighborhood_Blocks".to_string()),
        map_scripts_label: Some("GeneratedNeighborhood_MapScripts".to_string()),
        map_events_label: Some("GeneratedNeighborhood_MapEvents".to_string()),
        connection_flags: None,
    };
    module.scripts.clear();
    module.trainer_scripts.clear();
    module.scripted_trainer_battles.clear();
    module.scripted_wild_battles.clear();
    module.script_item_grants.clear();
    module.script_item_checks.clear();
    module.script_item_takes.clear();
    module.script_economy_commands.clear();
    module.gift_pokemon_scripts.clear();
    module.script_flag_commands.clear();
    module.script_scene_commands.clear();
    module.script_audio_commands.clear();
    module.script_block_changes.clear();
    module.script_object_commands.clear();
    module.script_movements.clear();
    module.script_map_commands.clear();
    module.script_text_commands.clear();
    module.script_text_bodies.clear();
    module.script_menu_definitions.clear();
    module.script_vertical_menus.clear();
    module.script_elevators.clear();
    module.script_variable_commands.clear();
    module.script_control_commands.clear();
    module.script_field_pickups.clear();
    module.script_shop_commands.clear();
    module.script_phone_commands.clear();
    module.script_runtime_commands.clear();
    module.script_swarm_commands.clear();
    module.map_script_section_commands.clear();
    module.map_event_section_commands.clear();
    module.scenes = MapSceneTable::default();
    module.events = MapEvents::default();
    module.objects.clear();
    module.blocks = grid.crystal_blocks();
    apply_generated_events(&mut module, grid);
    let mut pokecenter_warps = 0;
    let mut mart_warps = 0;
    for warp in &mut module.events.warps {
        if warp.target_map_constant == GENERATED_POKECENTER_CONSTANT {
            // Runtime warp resolution addresses maps by constant token. Keep
            // the redundant target fields identical, as in canonical maps.
            warp.target_map = GENERATED_POKECENTER_CONSTANT.to_string();
            pokecenter_warps += 1;
        } else if warp.target_map_constant == GENERATED_MART_CONSTANT {
            warp.target_map = GENERATED_MART_CONSTANT.to_string();
            mart_warps += 1;
        }
    }
    ensure!(
        pokecenter_warps == usize::from(has_pokecenter),
        "generated exterior must contain exactly {} Pokemon Center warp(s), found {pokecenter_warps}",
        usize::from(has_pokecenter)
    );
    ensure!(
        mart_warps == usize::from(has_mart),
        "generated exterior must contain exactly {} Mart warp(s), found {mart_warps}",
        usize::from(has_mart)
    );
    ensure_structured_raw_warps_match(&module)?;
    Ok(module)
}

fn generated_pokecenter_module(template: &MapModule) -> Result<MapModule> {
    let mut module = template.clone();
    module.id = GENERATED_POKECENTER_NAME.to_string();
    module.attributes.map_constant = Some(GENERATED_POKECENTER_CONSTANT.to_string());
    module.attributes.map_group_constant = Some(GENERATED_GROUP_NAME.to_string());
    module.attributes.blocks_label = Some("GeneratedPokecenter1F_Blocks".to_string());
    module.attributes.map_scripts_label = Some("GeneratedPokecenter1F_MapScripts".to_string());
    module.attributes.map_events_label = Some("GeneratedPokecenter1F_MapEvents".to_string());
    module.attributes.location = Some("LANDMARK_GOLDENROD_CITY".to_string());
    module.attributes.connections.clear();

    ensure!(
        module.events.warps.len() == 3,
        "Pokemon Center template must contain two exterior exits and one 2F stair warp"
    );
    let mut structured_exit_counts = [0_usize; 2];
    let mut structured_stairs = 0;
    for warp in &mut module.events.warps {
        if let Some(index) = POKECENTER_EXIT_WARP_COORDINATES
            .iter()
            .position(|coordinate| *coordinate == (warp.x, warp.y))
        {
            warp.target_map_constant = GENERATED_MAP_CONSTANT.to_string();
            warp.target_map = GENERATED_MAP_CONSTANT.to_string();
            warp.target_warp_id = 1;
            structured_exit_counts[index] += 1;
        } else if (warp.x, warp.y) == POKECENTER_2F_WARP_COORDINATE {
            ensure!(
                warp.target_map_constant == "POKECENTER_2F"
                    && warp.target_map == "POKECENTER_2F"
                    && warp.target_warp_id == 1,
                "Pokemon Center template has a malformed 2F stair warp"
            );
            structured_stairs += 1;
        }
    }
    ensure!(
        structured_exit_counts == [1, 1] && structured_stairs == 1,
        "Pokemon Center template must have exact exits at (3,7)/(4,7) and a 2F stair at (0,7)"
    );

    let nurse_scripts = module
        .objects
        .iter()
        .filter(|object| object.sprite == "SPRITE_NURSE")
        .map(|object| object.script.clone())
        .collect::<Vec<_>>();
    ensure!(
        nurse_scripts.len() == 1,
        "Pokemon Center template must contain exactly one canonical nurse"
    );
    let source_nurse_script = nurse_scripts
        .into_iter()
        .next()
        .context("Pokemon Center template nurse has no script")?;
    let nurse_script = module
        .scripts
        .remove(&source_nurse_script)
        .with_context(|| {
            format!("Pokemon Center template is missing nurse script {source_nurse_script}")
        })?;
    ensure!(
        nurse_script
            == serde_json::json!([{
                "command": "jumpstd",
                "args": ["PokecenterNurseScript"]
            }]),
        "Pokemon Center nurse must retain the canonical PokecenterNurseScript wrapper"
    );
    module.scripts.clear();
    module
        .scripts
        .insert(GENERATED_POKECENTER_NURSE_SCRIPT.to_string(), nurse_script);
    let mut nurse_control_commands = 0;
    module.script_control_commands.retain_mut(|command| {
        if command.source_script != source_nurse_script {
            return false;
        }
        command.source_script = GENERATED_POKECENTER_NURSE_SCRIPT.to_string();
        nurse_control_commands += 1;
        true
    });
    ensure!(
        nurse_control_commands == 1
            && module.script_control_commands[0].command == "jumpstd"
            && module.script_control_commands[0].target_label.as_deref()
                == Some("PokecenterNurseScript"),
        "Pokemon Center nurse must retain one canonical jumpstd control command"
    );
    module
        .objects
        .retain(|object| object.sprite == "SPRITE_NURSE");
    let nurse = module
        .objects
        .first_mut()
        .context("Pokemon Center nurse disappeared while removing template civilians")?;
    nurse.script = GENERATED_POKECENTER_NURSE_SCRIPT.to_string();
    nurse.object_identifier = Some(GENERATED_POKECENTER_NURSE_OBJECT.to_string());
    module.script_text_commands.clear();
    module.script_text_bodies.clear();

    for command in &module.map_event_section_commands {
        match command.command.as_str() {
            "warp_event" => {
                ensure!(
                    command.args.len() == 4,
                    "raw Pokemon Center warp event must contain four arguments"
                );
                let coordinate = (
                    command.args[0]
                        .parse::<u16>()
                        .context("raw Pokemon Center warp x must be an unsigned coordinate")?,
                    command.args[1]
                        .parse::<u16>()
                        .context("raw Pokemon Center warp y must be an unsigned coordinate")?,
                );
                if coordinate == POKECENTER_2F_WARP_COORDINATE {
                    ensure!(
                        command.args[2] == "POKECENTER_2F" && command.args[3] == "1",
                        "raw Pokemon Center template has a malformed 2F stair warp"
                    );
                }
            }
            "object_event" => ensure!(
                command.args.len() == 13,
                "raw Pokemon Center object event must contain thirteen arguments"
            ),
            _ => {}
        }
    }
    let mut raw_exit_counts = [0_usize; 2];
    let mut raw_stairs = 0;
    let mut raw_nurses = 0;
    module.map_event_section_commands.retain_mut(|command| {
        if command.command == "object_event" {
            let is_nurse = command
                .args
                .get(2)
                .is_some_and(|sprite| sprite == "SPRITE_NURSE")
                && command
                    .args
                    .get(11)
                    .is_some_and(|script| script == &source_nurse_script);
            if is_nurse {
                command.args[11] = GENERATED_POKECENTER_NURSE_SCRIPT.to_string();
                raw_nurses += 1;
            }
            return is_nurse;
        }
        if command.command != "warp_event" {
            return true;
        }
        let coordinate = command
            .args
            .first()
            .and_then(|x| x.parse::<u16>().ok())
            .zip(command.args.get(1).and_then(|y| y.parse::<u16>().ok()));
        if let Some(index) = coordinate.and_then(|coordinate| {
            POKECENTER_EXIT_WARP_COORDINATES
                .iter()
                .position(|candidate| *candidate == coordinate)
        }) {
            command.args[2] = GENERATED_MAP_CONSTANT.to_string();
            command.args[3] = "1".to_string();
            raw_exit_counts[index] += 1;
        } else if coordinate == Some(POKECENTER_2F_WARP_COORDINATE) {
            raw_stairs += 1;
        }
        true
    });
    ensure!(
        raw_exit_counts == [1, 1] && raw_stairs == 1,
        "raw Pokemon Center events must match the exact two exits and 2F stair"
    );
    ensure!(
        raw_nurses == 1,
        "raw Pokemon Center events must contain exactly one canonical nurse"
    );
    for (command_index, command) in module.map_event_section_commands.iter_mut().enumerate() {
        command.command_index = command_index;
    }
    ensure_structured_raw_warps_match(&module)?;
    Ok(module)
}

fn generated_mart_module(template: &MapModule, exterior_mart_warp_id: u16) -> Result<MapModule> {
    let exterior_mart_warp_id = i16::try_from(exterior_mart_warp_id)
        .context("generated exterior Mart warp id must fit an i16")?;
    let mut module = template.clone();
    ensure!(
        module.attributes.width == 6 && module.attributes.height == 4,
        "Violet Mart template must retain the canonical 6x4 interior shell"
    );
    ensure!(
        module.blocks
            == [
                20, 39, 20, 19, 19, 19, 43, 34, 43, 43, 43, 43, 21, 41, 19, 19, 19, 40, 43, 42, 43,
                43, 43, 35,
            ],
        "Violet Mart template block shell no longer matches the canonical common Mart"
    );
    ensure!(
        module.events.coord_events.is_empty()
            && module.events.bg_events.is_empty()
            && module.scenes.scenes.is_empty(),
        "Violet Mart template unexpectedly contains scene, coordinate, or background events"
    );
    ensure!(
        module.map_script_section_commands.len() == 2
            && module.map_script_section_commands[0].command == "def_scene_scripts"
            && module.map_script_section_commands[0].args.is_empty()
            && module.map_script_section_commands[1].command == "def_callbacks"
            && module.map_script_section_commands[1].args.is_empty(),
        "Violet Mart template must retain only the canonical empty scene/callback sections"
    );
    module.id = GENERATED_MART_NAME.to_string();
    module.attributes.map_constant = Some(GENERATED_MART_CONSTANT.to_string());
    module.attributes.map_group_constant = Some(GENERATED_GROUP_NAME.to_string());
    module.attributes.blocks_label = Some("GeneratedMart1F_Blocks".to_string());
    module.attributes.map_scripts_label = Some("GeneratedMart1F_MapScripts".to_string());
    module.attributes.map_events_label = Some("GeneratedMart1F_MapEvents".to_string());
    module.attributes.location = Some("LANDMARK_GOLDENROD_CITY".to_string());
    module.attributes.music = Some("MUSIC_GOLDENROD_CITY".to_string());
    module.attributes.connections.clear();

    ensure!(
        module.events.warps.len() == MART_EXIT_WARP_COORDINATES.len(),
        "Mart template must contain exactly two exterior exits"
    );
    let mut structured_exit_counts = [0_usize; 2];
    for warp in &mut module.events.warps {
        let index = MART_EXIT_WARP_COORDINATES
            .iter()
            .position(|coordinate| *coordinate == (warp.x, warp.y))
            .with_context(|| {
                format!(
                    "Mart template contains unexpected warp {} at ({}, {})",
                    warp.index, warp.x, warp.y
                )
            })?;
        ensure!(
            warp.index == u16::try_from(index + 1).expect("two Mart exits fit a u16"),
            "Mart template exit at ({}, {}) has noncanonical warp index {}",
            warp.x,
            warp.y,
            warp.index
        );
        ensure!(
            warp.target_map_constant == "VIOLET_CITY"
                && warp.target_map == "VIOLET_CITY"
                && warp.target_warp_id == 1,
            "Mart template exit at ({}, {}) is not the canonical Violet City exit",
            warp.x,
            warp.y
        );
        warp.target_map_constant = GENERATED_MAP_CONSTANT.to_string();
        warp.target_map = GENERATED_MAP_CONSTANT.to_string();
        warp.target_warp_id = exterior_mart_warp_id;
        structured_exit_counts[index] += 1;
    }
    ensure!(
        structured_exit_counts == [1, 1],
        "Mart template must have exact exits at (2,7) and (3,7)"
    );
    module.events.warps.sort_unstable_by_key(|warp| warp.index);

    let clerk_scripts = module
        .objects
        .iter()
        .filter(|object| object.sprite == "SPRITE_CLERK")
        .map(|object| object.script.clone())
        .collect::<Vec<_>>();
    ensure!(
        clerk_scripts.len() == 1,
        "Mart template must contain exactly one canonical clerk"
    );
    let source_clerk_script = clerk_scripts
        .into_iter()
        .next()
        .context("Mart template clerk has no script")?;
    let source_clerk = module
        .objects
        .iter()
        .find(|object| object.sprite == "SPRITE_CLERK" && object.script == source_clerk_script)
        .context("Mart template canonical clerk disappeared")?;
    ensure!(
        source_clerk.x == 1
            && source_clerk.y == 3
            && source_clerk.spritemovedata == "SPRITEMOVEDATA_STANDING_RIGHT"
            && source_clerk.move_range_x == 0
            && source_clerk.move_range_y == 0
            && source_clerk.hram_x == -1
            && source_clerk.hram_y == -1
            && source_clerk.pal == 0
            && source_clerk.object_type == "OBJECTTYPE_SCRIPT"
            && source_clerk.radius == 0
            && source_clerk.label.is_none()
            && source_clerk.event_flag == "-1"
            && source_clerk.object_identifier.as_deref() == Some("VIOLETMART_CLERK")
            && source_clerk.sightline_direction_override.is_none(),
        "Mart template clerk no longer matches the canonical Violet Mart clerk"
    );
    let clerk_script = module
        .scripts
        .remove(&source_clerk_script)
        .with_context(|| format!("Mart template is missing clerk script {source_clerk_script}"))?;
    ensure!(
        clerk_script
            == serde_json::json!([
                {"command": "opentext", "args": []},
                {
                    "command": "pokemart",
                    "args": ["MARTTYPE_STANDARD", "MART_VIOLET"]
                },
                {"command": "closetext", "args": []},
                {"command": "end", "args": []}
            ]),
        "Mart clerk must retain the exact canonical Violet Mart shop script"
    );
    module.scripts.clear();
    module
        .scripts
        .insert(GENERATED_MART_CLERK_SCRIPT.to_string(), clerk_script);

    let mut clerk_text_commands = 0;
    module.script_text_commands.retain_mut(|command| {
        if command.source_script != source_clerk_script {
            return false;
        }
        command.source_script = GENERATED_MART_CLERK_SCRIPT.to_string();
        clerk_text_commands += 1;
        true
    });
    ensure!(
        clerk_text_commands == 2
            && module.script_text_commands.iter().any(|command| {
                command.command == "opentext"
                    && command.command_index == 0
                    && command.text_label.is_none()
            })
            && module.script_text_commands.iter().any(|command| {
                command.command == "closetext"
                    && command.command_index == 2
                    && command.text_label.is_none()
            }),
        "Mart clerk must retain exact opentext/closetext typed commands"
    );
    module.script_text_bodies.clear();

    let mut clerk_shop_commands = 0;
    module.script_shop_commands.retain_mut(|command| {
        if command.source_script != source_clerk_script {
            return false;
        }
        command.source_script = GENERATED_MART_CLERK_SCRIPT.to_string();
        clerk_shop_commands += 1;
        true
    });
    ensure!(
        clerk_shop_commands == 1
            && module.script_shop_commands[0].command == "pokemart"
            && module.script_shop_commands[0].mart_type == "MARTTYPE_STANDARD"
            && module.script_shop_commands[0].mart_id == "MART_VIOLET"
            && module.script_shop_commands[0].command_index == 1,
        "Mart clerk must retain one canonical MART_VIOLET shop command"
    );

    let mut clerk_control_commands = 0;
    module.script_control_commands.retain_mut(|command| {
        if command.source_script != source_clerk_script {
            return false;
        }
        command.source_script = GENERATED_MART_CLERK_SCRIPT.to_string();
        clerk_control_commands += 1;
        true
    });
    ensure!(
        clerk_control_commands == 1
            && module.script_control_commands[0].command == "end"
            && module.script_control_commands[0].target_label.is_none()
            && module.script_control_commands[0]
                .resolved_target_script
                .is_none()
            && module.script_control_commands[0].command_index == 3,
        "Mart clerk must retain one canonical end control command"
    );

    module.trainer_scripts.clear();
    module.scripted_trainer_battles.clear();
    module.scripted_wild_battles.clear();
    module.script_item_grants.clear();
    module.script_item_checks.clear();
    module.script_item_takes.clear();
    module.script_economy_commands.clear();
    module.gift_pokemon_scripts.clear();
    module.script_flag_commands.clear();
    module.script_scene_commands.clear();
    module.script_audio_commands.clear();
    module.script_block_changes.clear();
    module.script_object_commands.clear();
    module.script_movements.clear();
    module.script_map_commands.clear();
    module.script_menu_definitions.clear();
    module.script_vertical_menus.clear();
    module.script_elevators.clear();
    module.script_variable_commands.clear();
    module.script_field_pickups.clear();
    module.script_phone_commands.clear();
    module.script_runtime_commands.clear();
    module.script_swarm_commands.clear();
    module.scenes = MapSceneTable::default();
    module.events.coord_events.clear();
    module.events.bg_events.clear();
    module.map_script_section_commands = vec![
        MapScriptSectionCommand {
            command: "def_scene_scripts".to_string(),
            args: Vec::new(),
            command_index: 0,
        },
        MapScriptSectionCommand {
            command: "def_callbacks".to_string(),
            args: Vec::new(),
            command_index: 1,
        },
    ];

    module
        .objects
        .retain(|object| object.sprite == "SPRITE_CLERK" && object.script == source_clerk_script);
    ensure!(
        module.objects.len() == 1,
        "Mart clerk disappeared while removing template civilians"
    );
    let clerk = &mut module.objects[0];
    clerk.script = GENERATED_MART_CLERK_SCRIPT.to_string();
    clerk.object_identifier = Some(GENERATED_MART_CLERK_OBJECT.to_string());

    let mut raw_exit_counts = [0_usize; 2];
    let mut raw_clerks = 0;
    for command in &module.map_event_section_commands {
        if command.command == "warp_event" {
            ensure!(
                command.args.len() == 4,
                "raw Mart warp event must contain four arguments"
            );
            let coordinate = (
                command.args[0]
                    .parse::<u16>()
                    .context("raw Mart warp x must be an unsigned coordinate")?,
                command.args[1]
                    .parse::<u16>()
                    .context("raw Mart warp y must be an unsigned coordinate")?,
            );
            let index = MART_EXIT_WARP_COORDINATES
                .iter()
                .position(|candidate| *candidate == coordinate)
                .context("raw Mart template contains an unexpected warp coordinate")?;
            ensure!(
                command.args[2] == "VIOLET_CITY" && command.args[3] == "1",
                "raw Mart template contains a malformed exterior exit"
            );
            raw_exit_counts[index] += 1;
        } else if command.command == "object_event"
            && command
                .args
                .get(2)
                .is_some_and(|sprite| sprite == "SPRITE_CLERK")
        {
            ensure!(
                command.args
                    == [
                        "1",
                        "3",
                        "SPRITE_CLERK",
                        "SPRITEMOVEDATA_STANDING_RIGHT",
                        "0",
                        "0",
                        "-1",
                        "-1",
                        "0",
                        "OBJECTTYPE_SCRIPT",
                        "0",
                        source_clerk_script.as_str(),
                        "-1",
                    ],
                "raw Mart clerk no longer matches the canonical object event"
            );
            raw_clerks += 1;
        }
    }
    ensure!(
        raw_exit_counts == [1, 1],
        "raw Mart events must contain exact exits at (2,7) and (3,7)"
    );
    ensure!(
        raw_clerks == 1,
        "raw Mart events must contain exactly one canonical clerk"
    );
    let exterior_mart_warp_id = exterior_mart_warp_id.to_string();
    module.map_event_section_commands = vec![
        MapEventSectionCommand {
            command: "db".to_string(),
            args: vec!["0".to_string(), "0".to_string()],
            command_index: 0,
        },
        MapEventSectionCommand {
            command: "def_warp_events".to_string(),
            args: Vec::new(),
            command_index: 1,
        },
        MapEventSectionCommand {
            command: "warp_event".to_string(),
            args: vec![
                "2".to_string(),
                "7".to_string(),
                GENERATED_MAP_CONSTANT.to_string(),
                exterior_mart_warp_id.clone(),
            ],
            command_index: 2,
        },
        MapEventSectionCommand {
            command: "warp_event".to_string(),
            args: vec![
                "3".to_string(),
                "7".to_string(),
                GENERATED_MAP_CONSTANT.to_string(),
                exterior_mart_warp_id,
            ],
            command_index: 3,
        },
        MapEventSectionCommand {
            command: "def_coord_events".to_string(),
            args: Vec::new(),
            command_index: 4,
        },
        MapEventSectionCommand {
            command: "def_bg_events".to_string(),
            args: Vec::new(),
            command_index: 5,
        },
        MapEventSectionCommand {
            command: "def_object_events".to_string(),
            args: Vec::new(),
            command_index: 6,
        },
        MapEventSectionCommand {
            command: "object_event".to_string(),
            args: vec![
                "1".to_string(),
                "3".to_string(),
                "SPRITE_CLERK".to_string(),
                "SPRITEMOVEDATA_STANDING_RIGHT".to_string(),
                "0".to_string(),
                "0".to_string(),
                "-1".to_string(),
                "-1".to_string(),
                "0".to_string(),
                "OBJECTTYPE_SCRIPT".to_string(),
                "0".to_string(),
                GENERATED_MART_CLERK_SCRIPT.to_string(),
                "-1".to_string(),
            ],
            command_index: 7,
        },
    ];
    ensure_structured_raw_warps_match(&module)?;
    Ok(module)
}

fn ensure_structured_raw_warps_match(module: &MapModule) -> Result<()> {
    let structured = module
        .events
        .warps
        .iter()
        .map(|warp| {
            ensure!(
                warp.target_map == warp.target_map_constant,
                "warp {} on {} has divergent target map name and constant",
                warp.index,
                module.id
            );
            Ok((
                warp.x,
                warp.y,
                warp.target_map_constant.clone(),
                warp.target_warp_id,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let raw = module
        .map_event_section_commands
        .iter()
        .filter(|command| command.command == "warp_event")
        .map(|command| {
            ensure!(
                command.args.len() == 4,
                "raw warp event on {} must contain four arguments",
                module.id
            );
            Ok((
                command.args[0].parse::<u16>()?,
                command.args[1].parse::<u16>()?,
                command.args[2].clone(),
                command.args[3].parse::<i16>()?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    ensure!(
        structured == raw,
        "structured and raw warp events diverge on {}",
        module.id
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crystal_core::{
        map::WarpEvent,
        state::GameState,
        world::{map::TilePosition, session::WarpTrigger},
    };
    use tempfile::tempdir;

    use super::*;
    use crate::{BoundingBox, Coordinate, MapCell, MapSource};

    fn repository_root_for_tests() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("workspace is nested under rust/crates/crystal-mapgen")
            .to_path_buf()
    }

    fn generated_grid_with_facilities(has_center: bool, has_mart: bool) -> GeneratedGrid {
        const WIDTH: u16 = 8;
        const HEIGHT: u16 = 8;
        let mut cells = vec![MapCell::Clearing; usize::from(WIDTH * HEIGHT)];
        if has_center {
            for (x, y, cell) in [
                (2, 2, MapCell::PokecenterNorthWest),
                (3, 2, MapCell::PokecenterNorthEast),
                (2, 3, MapCell::PokecenterSouthWest),
                (3, 3, MapCell::PokecenterSouthEast),
            ] {
                cells[usize::from(y * WIDTH + x)] = cell;
            }
        }
        if has_mart {
            for (x, y, cell) in [
                (5, 2, MapCell::MartNorthWest),
                (6, 2, MapCell::MartNorthEast),
                (5, 3, MapCell::MartSouthWest),
                (6, 3, MapCell::MartSouthEast),
            ] {
                cells[usize::from(y * WIDTH + x)] = cell;
            }
        }
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: Coordinate {
                    lat: 44.948,
                    lon: -93.305,
                },
                bounds: BoundingBox {
                    south: 44.94,
                    west: -93.32,
                    north: 44.96,
                    east: -93.29,
                },
                attribution: "integration test fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: WIDTH,
            height: HEIGHT,
            cells,
            labels: Vec::new(),
        };
        let mut plan = crate::plan_h3_cell(grid.source.center, 6).expect("plan test H3 cell");
        let cell = plan.cell.clone();
        plan.regional = Some(crate::H3RegionalCellPlan {
            ordinal: 0,
            cell,
            building_count: 0,
            facilities: [
                has_center.then_some(H3Facility::PokemonCenter),
                has_mart.then_some(H3Facility::Mart),
            ]
            .into_iter()
            .flatten()
            .collect(),
            connections: Vec::new(),
            closed_transport_crossings: Vec::new(),
        });
        grid.source.h3 = Some(plan);
        grid
    }

    fn generated_center_grid() -> GeneratedGrid {
        let mut grid = generated_grid_with_facilities(true, true);
        grid.source.h3 = None;
        grid
    }

    #[test]
    fn surface_biomes_receive_spatially_distinct_encounter_tables() {
        let mut grid = generated_grid_with_facilities(false, false);
        grid.cells[1 * 8 + 1] = MapCell::IceFloor;
        grid.cells[1 * 8 + 2] = MapCell::IceFloor;
        grid.cells[5 * 8 + 5] = MapCell::RockFloor;
        grid.cells[5 * 8 + 6] = MapCell::RockFloor;

        let zones = generated_biome_encounter_zones(&grid);
        assert_eq!(zones.len(), 2);
        let ice = zones
            .iter()
            .find(|zone| zone.id.starts_with("ice_surface"))
            .unwrap();
        let rock = zones
            .iter()
            .find(|zone| zone.id.starts_with("rock_surface"))
            .unwrap();
        assert_eq!((ice.min_x, ice.min_y, ice.max_x, ice.max_y), (2, 2, 5, 3));
        assert!(ice.grass.day.iter().any(|entry| entry.species == "SWINUB"));
        assert!(
            ice.grass
                .night
                .iter()
                .any(|entry| entry.species == "SNEASEL")
        );
        assert_eq!(
            (rock.min_x, rock.min_y, rock.max_x, rock.max_y),
            (10, 10, 13, 11)
        );
        assert!(
            rock.grass
                .day
                .iter()
                .any(|entry| entry.species == "GEODUDE")
        );
        assert!(rock.grass.day.iter().any(|entry| entry.species == "ONIX"));
        assert!(!rock.grass.day.iter().any(|entry| entry.species == "SWINUB"));
    }

    fn warp_at(module: &MapModule, coordinate: (u16, u16)) -> &WarpEvent {
        module
            .events
            .warps
            .iter()
            .find(|warp| (warp.x, warp.y) == coordinate)
            .unwrap_or_else(|| panic!("{} has no warp at {coordinate:?}", module.id))
    }

    fn trigger(module: &MapModule, warp: &WarpEvent) -> WarpTrigger {
        WarpTrigger {
            map_name: module.id.clone(),
            tile: TilePosition::new(
                i16::try_from(warp.x).expect("test warp x fits runtime coordinates"),
                i16::try_from(warp.y).expect("test warp y fits runtime coordinates"),
            ),
            permission: 0,
            warp: warp.clone(),
        }
    }

    fn assert_structured_raw_event_equivalence(module: &MapModule) {
        let raw = |command_name: &str| {
            module
                .map_event_section_commands
                .iter()
                .filter(|command| command.command == command_name)
                .map(|command| command.args.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            raw("warp_event"),
            module
                .events
                .warps
                .iter()
                .map(|event| vec![
                    event.x.to_string(),
                    event.y.to_string(),
                    event.target_map_constant.clone(),
                    event.target_warp_id.to_string(),
                ])
                .collect::<Vec<_>>(),
            "{} structured/raw warps diverged",
            module.id
        );
        assert_eq!(
            raw("coord_event"),
            module
                .events
                .coord_events
                .iter()
                .map(|event| vec![
                    event.x.to_string(),
                    event.y.to_string(),
                    event.scene_id.clone(),
                    event.script_name.clone(),
                ])
                .collect::<Vec<_>>(),
            "{} structured/raw coordinate events diverged",
            module.id
        );
        assert_eq!(
            raw("bg_event"),
            module
                .events
                .bg_events
                .iter()
                .map(|event| vec![
                    event.x.to_string(),
                    event.y.to_string(),
                    event.event_type.clone(),
                    event.script.clone(),
                ])
                .collect::<Vec<_>>(),
            "{} structured/raw background events diverged",
            module.id
        );
        assert_eq!(
            raw("object_event"),
            module
                .objects
                .iter()
                .map(|object| vec![
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
                ])
                .collect::<Vec<_>>(),
            "{} structured/raw object events diverged",
            module.id
        );
        assert_eq!(
            module
                .map_event_section_commands
                .iter()
                .map(|command| command.command_index)
                .collect::<Vec<_>>(),
            (0..module.map_event_section_commands.len()).collect::<Vec<_>>(),
            "{} raw event indexes are not contiguous",
            module.id
        );
    }

    #[test]
    fn generated_center_hardening_uses_warp_coordinates_not_template_order() {
        let root = repository_root_for_tests();
        let pack =
            read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
                .expect("load canonical pack");
        let mut template = pack
            .data()
            .map_module("MahoganyPokecenter1F")
            .expect("load canonical Pokemon Center template")
            .clone();

        template.events.warps.rotate_right(1);
        let raw_warp_indexes = template
            .map_event_section_commands
            .iter()
            .enumerate()
            .filter_map(|(index, command)| (command.command == "warp_event").then_some(index))
            .collect::<Vec<_>>();
        let mut raw_warps = raw_warp_indexes
            .iter()
            .map(|&index| template.map_event_section_commands[index].clone())
            .collect::<Vec<_>>();
        raw_warps.rotate_right(1);
        for (index, command) in raw_warp_indexes.into_iter().zip(raw_warps) {
            template.map_event_section_commands[index] = command;
        }

        let hardened = generated_pokecenter_module(&template)
            .expect("harden reordered canonical Pokemon Center template");
        for exit_coordinate in POKECENTER_EXIT_WARP_COORDINATES {
            let exit = warp_at(&hardened, exit_coordinate);
            assert_eq!(exit.target_map_constant, GENERATED_MAP_CONSTANT);
            assert_eq!(exit.target_map, GENERATED_MAP_CONSTANT);
            assert_eq!(exit.target_warp_id, 1);
        }
        let stair = warp_at(&hardened, POKECENTER_2F_WARP_COORDINATE);
        assert_eq!(stair.target_map_constant, "POKECENTER_2F");
        assert_eq!(stair.target_map, "POKECENTER_2F");
        assert_eq!(stair.target_warp_id, 1);
        assert_structured_raw_event_equivalence(&hardened);
    }

    #[test]
    fn generated_mart_hardening_uses_coordinates_and_keeps_only_the_real_shop() {
        let root = repository_root_for_tests();
        let pack =
            read_verified_compiled_game_pack(root.join("content-packs/core-modular.crystalpack"))
                .expect("load canonical pack");
        let mut template = pack
            .data()
            .map_module("VioletMart")
            .expect("load canonical Violet Mart template")
            .clone();

        template.events.warps.reverse();
        template.objects.reverse();
        template.map_event_section_commands.reverse();

        let hardened = generated_mart_module(&template, 7)
            .expect("harden reordered canonical Violet Mart template");
        for exit_coordinate in MART_EXIT_WARP_COORDINATES {
            let exit = warp_at(&hardened, exit_coordinate);
            assert_eq!(exit.target_map_constant, GENERATED_MAP_CONSTANT);
            assert_eq!(exit.target_map, GENERATED_MAP_CONSTANT);
            assert_eq!(exit.target_warp_id, 7);
        }
        assert_eq!(hardened.objects.len(), 1);
        let clerk = &hardened.objects[0];
        assert_eq!(clerk.sprite, "SPRITE_CLERK");
        assert_eq!(clerk.script, GENERATED_MART_CLERK_SCRIPT);
        assert_eq!(
            clerk.object_identifier.as_deref(),
            Some(GENERATED_MART_CLERK_OBJECT)
        );
        assert_eq!(hardened.scripts.len(), 1);
        assert_eq!(hardened.script_text_commands.len(), 2);
        assert!(hardened.script_text_bodies.is_empty());
        assert_eq!(hardened.script_shop_commands.len(), 1);
        assert_eq!(
            (
                hardened.script_shop_commands[0].command.as_str(),
                hardened.script_shop_commands[0].mart_type.as_str(),
                hardened.script_shop_commands[0].mart_id.as_str(),
                hardened.script_shop_commands[0].source_script.as_str(),
                hardened.script_shop_commands[0].command_index,
            ),
            (
                "pokemart",
                "MARTTYPE_STANDARD",
                "MART_VIOLET",
                GENERATED_MART_CLERK_SCRIPT,
                1,
            )
        );
        assert_eq!(hardened.script_control_commands.len(), 1);
        assert_eq!(hardened.script_control_commands[0].command, "end");
        assert_structured_raw_event_equivalence(&hardened);
    }

    #[test]
    fn generated_center_pack_resolves_full_warp_chain_and_strips_template_story() {
        let root = repository_root_for_tests();
        let temporary = tempdir().expect("create temporary generated pack directory");
        let output_pack = temporary.path().join("generated-center.crystalpack");
        build_modpack(
            &generated_center_grid(),
            ModpackOptions {
                base_pack: &root.join("content-packs/core-modular.crystalpack"),
                output_pack: &output_pack,
                manifest_id: "generated-center-integration-test",
                start_new_game_here: false,
            },
        )
        .expect("build generated map pack with Pokemon Center interior");

        let pack = read_verified_compiled_game_pack(&output_pack)
            .expect("reload and verify generated Pokemon Center pack");
        let data = pack.data();
        let exterior = data
            .map_module(GENERATED_MAP_NAME)
            .expect("load generated exterior module");
        let center = data
            .map_module(GENERATED_POKECENTER_NAME)
            .expect("load generated Pokemon Center module");
        let mart = data
            .map_module(GENERATED_MART_NAME)
            .expect("load generated Mart module");
        ensure_structured_raw_warps_match(exterior)
            .expect("exterior structured and raw warps agree");
        ensure_structured_raw_warps_match(center)
            .expect("Pokemon Center structured and raw warps agree");
        ensure_structured_raw_warps_match(mart).expect("Mart structured and raw warps agree");
        assert_structured_raw_event_equivalence(exterior);
        assert_structured_raw_event_equivalence(center);
        assert_structured_raw_event_equivalence(mart);

        let map_scripts_label = center
            .attributes
            .map_scripts_label
            .as_deref()
            .expect("generated Center declares its map scripts label");
        let map_events_label = center
            .attributes
            .map_events_label
            .as_deref()
            .expect("generated Center declares its map events label");
        let expected_map_scripts = serde_json::Value::Array(
            center
                .map_script_section_commands
                .iter()
                .map(|command| {
                    serde_json::json!({
                        "command": command.command,
                        "args": command.args,
                    })
                })
                .collect(),
        );
        let expected_map_events = serde_json::Value::Array(
            center
                .map_event_section_commands
                .iter()
                .map(|command| {
                    serde_json::json!({
                        "command": command.command,
                        "args": command.args,
                    })
                })
                .collect(),
        );
        assert_eq!(
            data.map_scripts.get(map_scripts_label),
            Some(&expected_map_scripts),
            "compiled Center map-script section differs from its module"
        );
        assert_eq!(
            data.map_scripts.get(map_events_label),
            Some(&expected_map_events),
            "compiled Center map-event section differs from its module"
        );
        assert_eq!(
            data.map_scripts.get(GENERATED_POKECENTER_NURSE_SCRIPT),
            center.scripts.get(GENERATED_POKECENTER_NURSE_SCRIPT),
            "compiled Center omitted or changed the canonical nurse wrapper"
        );
        assert_eq!(
            data.npcs.get(GENERATED_POKECENTER_NAME),
            Some(&serde_json::to_value(&center.objects).expect("serialize Center NPC roster")),
            "compiled Center NPC payload differs from its structured roster"
        );

        assert_eq!(center.objects.len(), 1);
        let nurse = &center.objects[0];
        assert_eq!(nurse.sprite, "SPRITE_NURSE");
        assert_eq!(nurse.script, GENERATED_POKECENTER_NURSE_SCRIPT);
        assert_eq!(
            nurse.object_identifier.as_deref(),
            Some(GENERATED_POKECENTER_NURSE_OBJECT)
        );
        assert_eq!(center.scripts.len(), 1);
        assert_eq!(
            center.scripts.get(GENERATED_POKECENTER_NURSE_SCRIPT),
            Some(&serde_json::json!([{
                "command": "jumpstd",
                "args": ["PokecenterNurseScript"]
            }]))
        );
        assert!(center.script_text_commands.is_empty());
        assert!(center.script_text_bodies.is_empty());
        let serialized_center = serde_json::to_string(center).expect("serialize generated Center");
        let serialized_center_value =
            serde_json::to_value(center).expect("inspect generated Center fields");
        for template_story_token in [
            "MahoganyPokecenter1F",
            "TEAM ROCKET",
            "LAKE OF RAGE",
            "SPRITE_POKEFAN_M",
            "SPRITE_YOUNGSTER",
            "SPRITE_COOLTRAINER_F",
        ] {
            assert!(
                !serialized_center.contains(template_story_token),
                "generated Center retained template token {template_story_token} in fields {:?}",
                serialized_center_value
                    .as_object()
                    .expect("MapModule serializes as an object")
                    .iter()
                    .filter_map(|(field, value)| value
                        .to_string()
                        .contains(template_story_token)
                        .then_some(field))
                    .collect::<Vec<_>>()
            );
        }
        let exterior_door = exterior
            .events
            .warps
            .iter()
            .find(|warp| warp.target_map_constant == GENERATED_POKECENTER_CONSTANT)
            .expect("generated exterior has a Pokemon Center door");
        assert_eq!(
            data.resolve_warp_transition(&trigger(exterior, exterior_door))
                .expect("resolve exterior door into Pokemon Center")
                .destination
                .map_name,
            GENERATED_POKECENTER_NAME
        );
        for exit_coordinate in POKECENTER_EXIT_WARP_COORDINATES {
            let transition = data
                .resolve_warp_transition(&trigger(center, warp_at(center, exit_coordinate)))
                .unwrap_or_else(|error| {
                    panic!("resolve Pokemon Center exit at {exit_coordinate:?}: {error:#}")
                });
            assert_eq!(transition.destination.map_name, GENERATED_MAP_NAME);
            assert_eq!(transition.destination.warp.index, exterior_door.index);
            assert_eq!(
                transition.destination.tile,
                TilePosition::new(
                    i16::try_from(exterior_door.x).unwrap(),
                    i16::try_from(exterior_door.y).unwrap()
                )
            );
        }

        let mut state = GameState::reset_wram_for_new_game();
        data.resolve_warp_transition_with_state(&mut state, &trigger(exterior, exterior_door))
            .expect("record exterior-to-Center warp state");
        let stairs = warp_at(center, POKECENTER_2F_WARP_COORDINATE);
        let upstairs = data
            .resolve_warp_transition_with_state(&mut state, &trigger(center, stairs))
            .expect("resolve generated Center stairs to shared 2F");
        assert_eq!(upstairs.destination.map_name, "Pokecenter2F");
        assert_eq!(upstairs.destination.warp.index, 1);
        let second_floor = data
            .map_module(&upstairs.destination.map_name)
            .expect("load shared Pokemon Center 2F");
        let return_warp = &second_floor.events.warps[0];
        assert_eq!(return_warp.target_warp_id, -1);
        let downstairs = data
            .resolve_warp_transition_with_state(&mut state, &trigger(second_floor, return_warp))
            .expect("resolve shared 2F dynamic return to generated Center");
        assert_eq!(downstairs.destination.map_name, GENERATED_POKECENTER_NAME);
        assert_eq!(downstairs.destination.warp.index, stairs.index);
        assert_eq!(
            downstairs.destination.tile,
            TilePosition::new(
                i16::try_from(stairs.x).unwrap(),
                i16::try_from(stairs.y).unwrap()
            )
        );

        let mart_map_scripts_label = mart
            .attributes
            .map_scripts_label
            .as_deref()
            .expect("generated Mart declares its map scripts label");
        let mart_map_events_label = mart
            .attributes
            .map_events_label
            .as_deref()
            .expect("generated Mart declares its map events label");
        let expected_mart_scripts = serde_json::Value::Array(
            mart.map_script_section_commands
                .iter()
                .map(|command| {
                    serde_json::json!({
                        "command": command.command,
                        "args": command.args,
                    })
                })
                .collect(),
        );
        let expected_mart_events = serde_json::Value::Array(
            mart.map_event_section_commands
                .iter()
                .map(|command| {
                    serde_json::json!({
                        "command": command.command,
                        "args": command.args,
                    })
                })
                .collect(),
        );
        assert_eq!(
            data.map_scripts.get(mart_map_scripts_label),
            Some(&expected_mart_scripts),
            "compiled Mart map-script section differs from its module"
        );
        assert_eq!(
            data.map_scripts.get(mart_map_events_label),
            Some(&expected_mart_events),
            "compiled Mart map-event section differs from its module"
        );
        assert_eq!(
            data.map_scripts.get(GENERATED_MART_CLERK_SCRIPT),
            mart.scripts.get(GENERATED_MART_CLERK_SCRIPT),
            "compiled Mart omitted or changed the canonical clerk script"
        );
        assert_eq!(
            data.npcs.get(GENERATED_MART_NAME),
            Some(&serde_json::to_value(&mart.objects).expect("serialize Mart NPC roster")),
            "compiled Mart NPC payload differs from its structured roster"
        );
        assert_eq!(mart.objects.len(), 1);
        assert_eq!(mart.objects[0].sprite, "SPRITE_CLERK");
        assert_eq!(mart.objects[0].script, GENERATED_MART_CLERK_SCRIPT);
        assert_eq!(mart.scripts.len(), 1);
        assert_eq!(mart.script_text_commands.len(), 2);
        assert!(mart.script_text_bodies.is_empty());
        assert_eq!(mart.script_shop_commands.len(), 1);
        assert_eq!(mart.script_control_commands.len(), 1);
        for removed_template_civilian in [
            "VioletMartCooltrainerMScript",
            "VioletMartCooltrainerMText",
            "VioletMartGrannyScript",
            "VioletMartGrannyText",
            "SPRITE_GRANNY",
            "SPRITE_COOLTRAINER_M",
        ] {
            assert!(
                !serde_json::to_string(mart)
                    .expect("serialize generated Mart")
                    .contains(removed_template_civilian),
                "generated Mart retained template civilian token {removed_template_civilian}"
            );
        }

        let exterior_mart_door = exterior
            .events
            .warps
            .iter()
            .find(|warp| warp.target_map_constant == GENERATED_MART_CONSTANT)
            .expect("generated exterior has a Mart door");
        assert_eq!(exterior_mart_door.index, 2);
        assert_eq!(exterior_mart_door.target_warp_id, 2);
        let enter_mart = data
            .resolve_warp_transition(&trigger(exterior, exterior_mart_door))
            .expect("resolve exterior door into Mart");
        assert_eq!(enter_mart.destination.map_name, GENERATED_MART_NAME);
        assert_eq!(enter_mart.destination.warp.index, 2);
        for exit_coordinate in MART_EXIT_WARP_COORDINATES {
            let transition = data
                .resolve_warp_transition(&trigger(mart, warp_at(mart, exit_coordinate)))
                .unwrap_or_else(|error| {
                    panic!("resolve Mart exit at {exit_coordinate:?}: {error:#}")
                });
            assert_eq!(transition.destination.map_name, GENERATED_MAP_NAME);
            assert_eq!(transition.destination.warp.index, exterior_mart_door.index);
            assert_eq!(
                transition.destination.tile,
                TilePosition::new(
                    i16::try_from(exterior_mart_door.x).unwrap(),
                    i16::try_from(exterior_mart_door.y).unwrap()
                )
            );
        }

        let shop_command = data
            .script_shop_command(GENERATED_MART_NAME, GENERATED_MART_CLERK_SCRIPT, 1)
            .expect("resolve generated Mart clerk's typed shop command");
        assert_eq!(shop_command.command, "pokemart");
        assert_eq!(shop_command.mart_type, "MARTTYPE_STANDARD");
        assert_eq!(shop_command.mart_id, "MART_VIOLET");
        let mut shop_state = GameState::reset_wram_for_new_game();
        shop_state.money = 1_000;
        let opened = data
            .open_script_shop(
                &mut shop_state,
                GENERATED_MART_NAME,
                GENERATED_MART_NAME,
                GENERATED_MART_CLERK_SCRIPT,
                1,
            )
            .expect("open generated Mart through its runtime shop command");
        assert_eq!(opened.mart_type, "MARTTYPE_STANDARD");
        assert_eq!(opened.mart_id, "MART_VIOLET");
        assert_eq!(
            opened.inventory,
            [
                "POKE_BALL",
                "POTION",
                "ESCAPE_ROPE",
                "ANTIDOTE",
                "PARLYZ_HEAL",
                "AWAKENING",
                "X_DEFEND",
                "X_ATTACK",
                "X_SPEED",
                "FLOWER_MAIL",
            ]
        );
        let pending_shop = shop_state
            .script_runtime
            .pending_shop
            .as_ref()
            .expect("opening generated Mart installs a pending shop request");
        assert_eq!(pending_shop.mart_id, "MART_VIOLET");
        assert_eq!(pending_shop.source_script, GENERATED_MART_CLERK_SCRIPT);
        assert_eq!(pending_shop.command_index, 1);
        let potion = data.items.get("POTION").expect("canonical Potion item");
        let money_before_purchase = shop_state.money;
        let potions_before_purchase = shop_state.bag.quantity(potion);
        let purchase = data
            .buy_shop_item(&mut shop_state, "POTION", 1)
            .expect("buy one Potion from generated Mart");
        assert!(purchase.success);
        assert_eq!(
            shop_state.money,
            money_before_purchase - u32::from(potion.price)
        );
        assert_eq!(shop_state.bag.quantity(potion), potions_before_purchase + 1);
    }

    #[test]
    fn generated_pack_includes_only_allocated_facility_interiors() {
        let root = repository_root_for_tests();
        let temporary = tempdir().expect("create temporary generated pack directory");
        for (has_center, has_mart, case_name) in [
            (false, false, "neither"),
            (true, false, "center-only"),
            (false, true, "mart-only"),
            (true, true, "both"),
        ] {
            let output_pack = temporary.path().join(format!("{case_name}.crystalpack"));
            build_modpack(
                &generated_grid_with_facilities(has_center, has_mart),
                ModpackOptions {
                    base_pack: &root.join("content-packs/core-modular.crystalpack"),
                    output_pack: &output_pack,
                    manifest_id: case_name,
                    start_new_game_here: false,
                },
            )
            .unwrap_or_else(|error| panic!("build {case_name} generated pack: {error:#}"));

            let pack = read_verified_compiled_game_pack(&output_pack)
                .unwrap_or_else(|error| panic!("reload and verify {case_name} pack: {error:#}"));
            let exterior = pack
                .data()
                .map_module(GENERATED_MAP_NAME)
                .expect("generated exterior is always present");
            assert_eq!(
                pack.data().map_module(GENERATED_POKECENTER_NAME).is_ok(),
                has_center,
                "{case_name} interior set disagrees with its Center facade"
            );
            assert_eq!(
                pack.data().map_module(GENERATED_MART_NAME).is_ok(),
                has_mart,
                "{case_name} interior set disagrees with its Mart facade"
            );
            assert_eq!(
                exterior
                    .events
                    .warps
                    .iter()
                    .filter(|warp| warp.target_map_constant == GENERATED_POKECENTER_CONSTANT)
                    .count(),
                usize::from(has_center)
            );
            assert_eq!(
                exterior
                    .events
                    .warps
                    .iter()
                    .filter(|warp| warp.target_map_constant == GENERATED_MART_CONSTANT)
                    .count(),
                usize::from(has_mart)
            );
            ensure_structured_raw_warps_match(exterior)
                .unwrap_or_else(|error| panic!("{case_name} exterior warp records: {error:#}"));

            if has_center {
                let center = pack
                    .data()
                    .map_module(GENERATED_POKECENTER_NAME)
                    .expect("allocated Center interior exists");
                let exterior_door = exterior
                    .events
                    .warps
                    .iter()
                    .find(|warp| warp.target_map_constant == GENERATED_POKECENTER_CONSTANT)
                    .expect("allocated Center facade emits a door");
                assert_eq!(exterior_door.index, 1);
                assert_eq!(
                    pack.data()
                        .resolve_warp_transition(&trigger(exterior, exterior_door))
                        .expect("enter allocated Center")
                        .destination
                        .map_name,
                    GENERATED_POKECENTER_NAME
                );
                for coordinate in POKECENTER_EXIT_WARP_COORDINATES {
                    let transition = pack
                        .data()
                        .resolve_warp_transition(&trigger(center, warp_at(center, coordinate)))
                        .expect("exit allocated Center");
                    assert_eq!(transition.destination.map_name, GENERATED_MAP_NAME);
                    assert_eq!(transition.destination.warp.index, exterior_door.index);
                }
            }

            if has_mart {
                let mart = pack
                    .data()
                    .map_module(GENERATED_MART_NAME)
                    .expect("allocated Mart interior exists");
                let exterior_door = exterior
                    .events
                    .warps
                    .iter()
                    .find(|warp| warp.target_map_constant == GENERATED_MART_CONSTANT)
                    .expect("allocated Mart facade emits a door");
                assert_eq!(exterior_door.index, if has_center { 2 } else { 1 });
                assert_eq!(
                    pack.data()
                        .resolve_warp_transition(&trigger(exterior, exterior_door))
                        .expect("enter allocated Mart")
                        .destination
                        .map_name,
                    GENERATED_MART_NAME
                );
                for coordinate in MART_EXIT_WARP_COORDINATES {
                    let transition = pack
                        .data()
                        .resolve_warp_transition(&trigger(mart, warp_at(mart, coordinate)))
                        .expect("exit allocated Mart");
                    assert_eq!(transition.destination.map_name, GENERATED_MAP_NAME);
                    assert_eq!(transition.destination.warp.index, exterior_door.index);
                }
            }
        }
    }
}
