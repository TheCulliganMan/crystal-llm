use std::path::PathBuf;

use anyhow::{Context, Result};
use crystal_assets::AssetRoot;
use crystal_bevy::{CORE_RUNTIME_PACK_PATH, CrystalRuntime, RuntimeOverworldFrame};
use crystal_core::battle::start::WildBattleStart;
use crystal_core::input::GameButton;
use crystal_core::world::map::TilePosition;
use crystal_core::world::movement::StepOutcome;
use crystal_core::world::session::{
    ConnectionTransition, OverworldInteraction, OverworldInteractionTarget, WarpTransition,
    WildEncounterRoll,
};

fn main() -> Result<()> {
    let config = RuntimeCliConfig::from_args(std::env::args().skip(1))?;
    if config.help {
        print_help();
        return Ok(());
    }

    let asset_root = AssetRoot::new(&config.repository_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, &config.pack_path)
        .with_context(|| {
            format!(
                "boot Crystal runtime from {} under {}",
                config.pack_path.display(),
                config.repository_root.display()
            )
        })?;
    let summary = runtime.boot_summary();

    println!("Crystal native runtime booted");
    println!("repository_root={}", config.repository_root.display());
    println!("compiled_pack={}", config.pack_path.display());
    println!("modpack_id={}", summary.modpack_id);
    println!("modpack_hash={}", summary.modpack_hash);
    println!("pokemon_species={}", summary.pokemon_species);
    println!("moves={}", summary.moves);
    println!("maps={}", summary.maps);
    println!("items={}", summary.items);
    println!("wild_encounter_tables={}", summary.wild_encounter_tables);
    println!("music_tracks={}", summary.music_tracks);
    println!("sound_effects={}", summary.sound_effects);
    println!("cries={}", summary.cries);
    println!(
        "viewport={}x{} scale {}",
        summary.viewport.width, summary.viewport.height, summary.viewport.scale
    );
    let mut overworld = match &config.load_save {
        Some(path) => {
            let state = runtime
                .load_save(path)
                .with_context(|| format!("load native save {}", path.display()))?;
            println!("loaded_save={}", path.display());
            runtime.resume_overworld_session(&asset_root, state)?
        }
        None => runtime.start_overworld_session(&asset_root, config.spawn)?,
    };
    let snapshot = overworld.snapshot();
    println!(
        "start={} map={} tile={},{} facing={:?} music={} overworld_hash={:08x} state_hash={:08x}",
        config
            .load_save
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("spawn:{}", config.spawn)),
        snapshot.map_name,
        snapshot.tile.x,
        snapshot.tile.y,
        snapshot.facing,
        overworld
            .state
            .script_runtime
            .current_music
            .as_deref()
            .unwrap_or(""),
        overworld.overworld.state_hash(),
        overworld.state_checksum_frame(1)?.hash
    );
    for (index, buttons) in config.steps.iter().enumerate() {
        let frame = overworld.apply_buttons(&runtime, &asset_root, buttons.iter().copied())?;
        println!(
            "step={} input={:#010b} pressed={:#010b} map={} tile={},{} facing={:?} music={} state_hash={:08x} event={}",
            index + 1,
            frame.input_mask,
            frame.pressed_mask,
            frame.snapshot.map_name,
            frame.snapshot.tile.x,
            frame.snapshot.tile.y,
            frame.snapshot.facing,
            overworld
                .state
                .script_runtime
                .current_music
                .as_deref()
                .unwrap_or(""),
            frame.state_checksum.hash,
            format_frame_event(&frame)
        );
    }
    if let Some(path) = &config.save {
        runtime
            .save_game(path, overworld.state.clone())
            .with_context(|| format!("write native save {}", path.display()))?;
        println!(
            "saved={} frame={} state_hash={:08x}",
            path.display(),
            overworld.state.frame_counter,
            overworld.state_checksum_frame(1)?.hash
        );
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeCliConfig {
    repository_root: PathBuf,
    pack_path: PathBuf,
    spawn: u16,
    steps: Vec<Vec<GameButton>>,
    load_save: Option<PathBuf>,
    save: Option<PathBuf>,
    help: bool,
}

impl RuntimeCliConfig {
    fn from_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut repository_root = None;
        let mut pack_path = None;
        let mut spawn = 0;
        let mut steps = Vec::new();
        let mut load_save = None;
        let mut save = None;
        let mut positional_done = false;

        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    return Ok(Self {
                        repository_root: PathBuf::new(),
                        pack_path: PathBuf::new(),
                        spawn: 0,
                        steps: Vec::new(),
                        load_save: None,
                        save: None,
                        help: true,
                    });
                }
                "--spawn" => {
                    positional_done = true;
                    let value = args.next().context("--spawn requires a spawn identifier")?;
                    spawn = value
                        .parse::<u16>()
                        .with_context(|| format!("parse spawn identifier '{value}'"))?;
                }
                "--steps" => {
                    positional_done = true;
                    let value = args.next().context("--steps requires input tokens")?;
                    steps = parse_steps(&value)?;
                }
                "--load-save" => {
                    positional_done = true;
                    let value = args.next().context("--load-save requires a save path")?;
                    load_save = Some(PathBuf::from(value));
                }
                "--save" => {
                    positional_done = true;
                    let value = args.next().context("--save requires a save path")?;
                    save = Some(PathBuf::from(value));
                }
                _ if !positional_done && repository_root.is_none() => {
                    repository_root = Some(PathBuf::from(arg))
                }
                _ if !positional_done && pack_path.is_none() => {
                    pack_path = Some(PathBuf::from(arg))
                }
                _ => anyhow::bail!("unexpected extra argument '{arg}'"),
            }
        }

        Ok(Self {
            repository_root: repository_root.unwrap_or_else(|| PathBuf::from(".")),
            pack_path: pack_path.unwrap_or_else(|| PathBuf::from(CORE_RUNTIME_PACK_PATH)),
            spawn,
            steps,
            load_save,
            save,
            help: false,
        })
    }
}

fn print_help() {
    println!(
        "Usage: crystal_runtime [repository-root] [compiled-pack] [--spawn id] [--steps tokens] [--load-save path] [--save path]"
    );
    println!();
    println!("compiled-pack is resolved relative to apps/web/assets/data.");
    println!("Default compiled-pack: {CORE_RUNTIME_PACK_PATH}");
    println!("steps tokens are comma-separated frames; combine held buttons with '+'.");
    println!("Example: --steps right,right,a+right");
    println!("Save files must use .crystalsave.");
}

fn parse_steps(input: &str) -> Result<Vec<Vec<GameButton>>> {
    if input.trim().is_empty() {
        anyhow::bail!("--steps must not be empty");
    }
    input
        .split(',')
        .map(|frame| {
            if frame.trim().is_empty() {
                anyhow::bail!("empty input frame in --steps");
            }
            frame
                .split('+')
                .map(parse_button)
                .collect::<Result<Vec<_>>>()
        })
        .collect()
}

fn parse_button(input: &str) -> Result<GameButton> {
    match input {
        "a" => Ok(GameButton::A),
        "b" => Ok(GameButton::B),
        "start" => Ok(GameButton::Start),
        "select" => Ok(GameButton::Select),
        "right" => Ok(GameButton::Right),
        "left" => Ok(GameButton::Left),
        "up" => Ok(GameButton::Up),
        "down" => Ok(GameButton::Down),
        other => anyhow::bail!("unknown input button '{other}'"),
    }
}

fn format_frame_event(frame: &RuntimeOverworldFrame) -> String {
    if let Some(warp) = &frame.warp {
        return format_warp(warp);
    }
    if let Some(connection) = &frame.connection {
        return format_connection(connection);
    }
    if let Some(interaction) = &frame.interaction {
        return format_interaction(interaction);
    }
    if let Some(battle) = &frame.wild_battle {
        return format_wild_battle(battle);
    }
    if let Some(encounter) = &frame.wild_encounter {
        return format_wild_encounter(encounter);
    }
    if let Some(movement) = &frame.movement {
        return format_movement(movement);
    }
    "idle".to_string()
}

fn format_movement(movement: &StepOutcome) -> String {
    match movement {
        StepOutcome::Turned { facing } => format!("turned:{facing:?}"),
        StepOutcome::Moved {
            from,
            to,
            speed_multiplier,
        } => format!(
            "moved:{}->{} speed={}",
            format_tile(*from),
            format_tile(*to),
            speed_multiplier
        ),
        StepOutcome::Blocked { at, facing } => {
            format!("blocked:{} facing={facing:?}", format_tile(*at))
        }
        StepOutcome::BlockedByObject {
            at,
            facing,
            object_identifier,
        } => format!(
            "blocked_object:{} facing={facing:?} object={}",
            format_tile(*at),
            object_identifier.as_deref().unwrap_or("")
        ),
    }
}

fn format_interaction(interaction: &OverworldInteraction) -> String {
    let target = match &interaction.target {
        OverworldInteractionTarget::Object {
            object_index,
            object_identifier,
            object_type,
        } => format!(
            "object index={} id={} type={}",
            object_index,
            object_identifier.as_deref().unwrap_or(""),
            object_type
        ),
        OverworldInteractionTarget::Background { event_type } => {
            format!("background type={event_type}")
        }
    };
    format!(
        "interaction:{} target={} tile={} script={}",
        interaction.map_name,
        target,
        format_tile(interaction.target_tile),
        interaction.script
    )
}

fn format_warp(warp: &WarpTransition) -> String {
    format!(
        "warp:{}:{}->{}:{}",
        warp.trigger.map_name,
        format_tile(warp.trigger.tile),
        warp.destination.map_name,
        format_tile(warp.destination.tile)
    )
}

fn format_connection(connection: &ConnectionTransition) -> String {
    format!(
        "connection:{}:{}->{}:{}",
        connection.trigger.map_name,
        format_tile(connection.trigger.tile),
        connection.destination.map_name,
        format_tile(connection.destination.tile)
    )
}

fn format_wild_encounter(encounter: &WildEncounterRoll) -> String {
    match &encounter.resolved {
        Some(resolved) => format!(
            "wild:{} surface={:?} roll={} slot={} species={} level={} rng={}",
            encounter.map_name,
            encounter.surface,
            encounter.encounter_roll,
            resolved.slot,
            resolved.encounter.species,
            resolved.level,
            encounter.rng_seed_after
        ),
        None => format!(
            "wild_none:{} surface={:?} roll={} threshold={} rng={}",
            encounter.map_name,
            encounter.surface,
            encounter.encounter_roll,
            encounter.threshold,
            encounter.rng_seed_after
        ),
    }
}

fn format_wild_battle(battle: &WildBattleStart) -> String {
    format!(
        "wild_battle:{} species={} level={} party={} rng={}",
        battle.encounter.map_name,
        battle.enemy_pokemon.species.id,
        battle.enemy_pokemon.level,
        battle.enemy_party.len(),
        battle.rng_seed_after
    )
}

fn format_tile(tile: TilePosition) -> String {
    format!("{},{}", tile.x, tile.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_repository_root_and_core_pack() {
        let config = RuntimeCliConfig::from_args(Vec::<String>::new()).expect("config");

        assert_eq!(config.repository_root, PathBuf::from("."));
        assert_eq!(config.pack_path, PathBuf::from(CORE_RUNTIME_PACK_PATH));
        assert_eq!(config.spawn, 0);
        assert!(config.steps.is_empty());
        assert_eq!(config.load_save, None);
        assert_eq!(config.save, None);
        assert!(!config.help);
    }

    #[test]
    fn accepts_repository_root_and_pack_path() {
        let config = RuntimeCliConfig::from_args([
            "/repo".to_string(),
            "content-packs/custom.crystalpack".to_string(),
        ])
        .expect("config");

        assert_eq!(config.repository_root, PathBuf::from("/repo"));
        assert_eq!(
            config.pack_path,
            PathBuf::from("content-packs/custom.crystalpack")
        );
    }

    #[test]
    fn accepts_spawn_and_input_steps() {
        let config = RuntimeCliConfig::from_args([
            "/repo".to_string(),
            "content-packs/custom.crystalpack".to_string(),
            "--spawn".to_string(),
            "14".to_string(),
            "--steps".to_string(),
            "right,right,a+right".to_string(),
            "--save".to_string(),
            "/tmp/slot.crystalsave".to_string(),
            "--load-save".to_string(),
            "/tmp/start.crystalsave".to_string(),
        ])
        .expect("config");

        assert_eq!(config.spawn, 14);
        assert_eq!(config.save, Some(PathBuf::from("/tmp/slot.crystalsave")));
        assert_eq!(
            config.load_save,
            Some(PathBuf::from("/tmp/start.crystalsave"))
        );
        assert_eq!(
            config.steps,
            vec![
                vec![GameButton::Right],
                vec![GameButton::Right],
                vec![GameButton::A, GameButton::Right],
            ]
        );
    }

    #[test]
    fn rejects_unknown_step_buttons() {
        let error = parse_steps("right,menu")
            .expect_err("unknown button must fail")
            .to_string();

        assert!(error.contains("unknown input button 'menu'"), "{error}");
    }

    #[test]
    fn formats_movement_events_for_headless_playback() {
        assert_eq!(
            format_movement(&StepOutcome::Moved {
                from: TilePosition::new(3, 3),
                to: TilePosition::new(5, 3),
                speed_multiplier: 1,
            }),
            "moved:3,3->5,3 speed=1"
        );
        assert_eq!(
            format_movement(&StepOutcome::BlockedByObject {
                at: TilePosition::new(7, 3),
                facing: crystal_core::world::map::Direction::Right,
                object_identifier: Some("Mom".to_string()),
            }),
            "blocked_object:7,3 facing=Right object=Mom"
        );
    }

    #[test]
    fn formats_wild_encounter_events_for_headless_playback() {
        let encounter = WildEncounterRoll {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(2, 0),
            surface: crystal_core::world::encounters::EncounterSurface::Grass,
            time: crystal_core::world::encounters::TimeOfDay::Day,
            threshold: 255,
            encounter_roll: 7,
            slot_percent_roll: Some(1),
            level_roll: Some(1),
            resolved: Some(crystal_core::world::encounters::ResolvedWildEncounter {
                encounter: crystal_core::world::encounters::WildEncounter {
                    level: 2,
                    species: "CHIKORITA".to_string(),
                },
                slot: 0,
                level: 2,
            }),
            rng_seed_after: 42,
        };

        assert_eq!(
            format_wild_encounter(&encounter),
            "wild:RuntimeMap surface=Grass roll=7 slot=0 species=CHIKORITA level=2 rng=42"
        );
    }

    #[test]
    fn formats_wild_battle_events_for_headless_playback() {
        let species = crystal_core::models::PokemonSpecies::new_for_tests(
            "CHIKORITA",
            crystal_core::models::BaseStats::new(45, 49, 65, 45, 49, 65),
        );
        let mut pokemon = crystal_core::models::Pokemon::new_for_tests(
            species,
            2,
            crystal_core::models::Dv::from_non_hp(1, 2, 3, 4),
        );
        pokemon.original_trainer_name = "WILD".to_string();
        let encounter = WildEncounterRoll {
            map_name: "RuntimeMap".to_string(),
            tile: TilePosition::new(2, 0),
            surface: crystal_core::world::encounters::EncounterSurface::Grass,
            time: crystal_core::world::encounters::TimeOfDay::Day,
            threshold: 255,
            encounter_roll: 7,
            slot_percent_roll: Some(1),
            level_roll: Some(1),
            resolved: None,
            rng_seed_after: 42,
        };
        let battle = WildBattleStart {
            battle_type: "BATTLETYPE_NORMAL".to_string(),
            encounter,
            enemy_pokemon: pokemon.clone(),
            enemy_party: vec![pokemon],
            rng_seed_after: 99,
        };

        assert_eq!(
            format_wild_battle(&battle),
            "wild_battle:RuntimeMap species=CHIKORITA level=2 party=1 rng=99"
        );
    }
}
