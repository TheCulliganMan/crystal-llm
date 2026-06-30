use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use crystal_bevy::{
    BevyShellConfig, BevyShellStart, CrystalRuntime,
    assets::{AssetRoot, modpack::COMPILED_GAME_PACK_EXTENSION},
    core::save::SAVE_EXTENSION,
};

fn main() -> Result<()> {
    let args = parse_args()?;
    if args.help {
        print_usage();
        return Ok(());
    }

    let pack = args
        .pack
        .as_deref()
        .context("--pack <assets/data relative .crystalpack> is required")?;
    let repository_root = args.repo.context("--repo <repo-root> is required")?;
    let asset_root = AssetRoot::new(repository_root);
    let runtime = CrystalRuntime::load_from_compiled_pack(&asset_root, pack)?;

    if args.list_spawns {
        print_boot_summary(&runtime);
        print_spawns(&runtime);
        return Ok(());
    }

    let start = match (args.spawn, args.load_save) {
        (Some(spawn_identifier), None) => BevyShellStart::NewGame { spawn_identifier },
        (None, Some(save_path)) => BevyShellStart::LoadSave { save_path },
        (Some(_), Some(_)) => bail!("--spawn and --load-save are mutually exclusive"),
        (None, None) => bail!("--spawn <id> or --load-save <path> is required"),
    };

    crystal_bevy::run_bevy_shell(
        asset_root,
        runtime,
        start,
        BevyShellConfig {
            quick_save_path: args.save_path,
        },
    )
}

#[derive(Debug, Default)]
struct Args {
    help: bool,
    list_spawns: bool,
    repo: Option<PathBuf>,
    pack: Option<String>,
    spawn: Option<u16>,
    load_save: Option<PathBuf>,
    save_path: Option<PathBuf>,
}

fn parse_args() -> Result<Args> {
    let mut args = Args::default();
    let mut values = env::args().skip(1);
    while let Some(arg) = values.next() {
        match arg.as_str() {
            "-h" | "--help" => args.help = true,
            "--list-spawns" => args.list_spawns = true,
            "--repo" => {
                let value = values.next().context("--repo requires a path")?;
                args.repo = Some(parse_cli_path_arg("--repo", value)?);
            }
            "--pack" => {
                let value = values
                    .next()
                    .context("--pack requires a .crystalpack path")?;
                args.pack = Some(parse_pack_arg("--pack", value)?);
            }
            "--spawn" => {
                let value = values.next().context("--spawn requires an id")?;
                args.spawn = Some(
                    value
                        .parse::<u16>()
                        .with_context(|| format!("spawn id '{value}' is not a u16"))?,
                );
            }
            "--load-save" => {
                let value = values.next().context("--load-save requires a path")?;
                args.load_save = Some(parse_save_path_arg("--load-save", value)?);
            }
            "--save-path" => {
                let value = values.next().context("--save-path requires a path")?;
                args.save_path = Some(parse_save_path_arg("--save-path", value)?);
            }
            other => bail!("unknown argument '{other}'"),
        }
    }
    Ok(args)
}

fn parse_cli_path_arg(flag: &str, value: String) -> Result<PathBuf> {
    let path = PathBuf::from(&value);
    if path.as_os_str().is_empty() {
        bail!("{flag} cannot be empty");
    }
    Ok(path)
}

fn parse_pack_arg(flag: &str, value: String) -> Result<String> {
    let path = parse_cli_path_arg(flag, value)?;
    require_extension(flag, &path, COMPILED_GAME_PACK_EXTENSION)?;
    path.into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("{flag} must be UTF-8"))
}

fn parse_save_path_arg(flag: &str, value: String) -> Result<PathBuf> {
    let path = parse_cli_path_arg(flag, value)?;
    require_extension(flag, &path, SAVE_EXTENSION)?;
    Ok(path)
}

fn require_extension(flag: &str, path: &PathBuf, expected: &str) -> Result<()> {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        bail!("{flag} path {} must use .{expected}", path.display());
    };
    if extension != expected {
        bail!(
            "{flag} path {} must use .{expected}, got .{extension}",
            path.display()
        );
    }
    Ok(())
}

fn print_usage() {
    println!(
        "usage: crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --spawn <id> [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --load-save <path> [--save-path <path>]"
    );
    println!(
        "       crystal-bevy --repo <repo-root> --pack <assets/data relative .crystalpack> --list-spawns"
    );
    println!(
        "example: cargo run -p crystal-bevy -- --repo /path/to/crystal-llm --pack <compiled-pack>.crystalpack --list-spawns"
    );
}

fn print_boot_summary(runtime: &CrystalRuntime) {
    let summary = runtime.boot_summary();
    println!(
        "pack={} maps={} species={} moves={} items={} wild_encounters={} music={} sfx={} cries={}",
        summary.modpack_id,
        summary.maps,
        summary.pokemon_species,
        summary.moves,
        summary.items,
        summary.wild_encounter_tables,
        summary.music_tracks,
        summary.sound_effects,
        summary.cries
    );
}

fn print_spawns(runtime: &CrystalRuntime) {
    println!("spawns:");
    for spawn in runtime.data().runtime_spawn_points().values() {
        println!(
            "  {} map={} constant={} group={} map_id={} tile=({}, {}) metatile=({}, {}) subtile=({}, {})",
            spawn.identifier,
            spawn.map_name,
            spawn.map_constant,
            spawn.group_id,
            spawn.map_id,
            spawn.tile_x,
            spawn.tile_y,
            spawn.metatile_x,
            spawn.metatile_y,
            spawn.subtile_x,
            spawn.subtile_y
        );
    }
}

#[cfg(not(feature = "bevy-shell"))]
compile_error!("crystal-bevy binary requires building with --features bevy-shell");
