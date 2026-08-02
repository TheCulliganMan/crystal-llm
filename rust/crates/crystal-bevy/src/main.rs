use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use crystal_bevy::{
    BevyShellConfig, BevyShellStart, CrystalRuntime,
    assets::{AssetRoot, modpack::COMPILED_GAME_PACK_EXTENSION},
    core::save::SAVE_EXTENSION,
};

const DEFAULT_PACK_FILENAME: &str = "core-modular.crystalpack";

fn main() -> Result<()> {
    let args = parse_args_from(env::args().skip(1))?;
    if args.help {
        print_usage();
        return Ok(());
    }

    let requested_pack_path = resolve_pack_path(args.pack.as_deref())?;
    let pack_path = std::fs::canonicalize(&requested_pack_path).with_context(|| {
        format!(
            "resolve compiled game pack {}",
            requested_pack_path.display()
        )
    })?;
    let pack_directory = pack_path
        .parent()
        .context("compiled game pack path has no parent directory")?
        .to_path_buf();
    let asset_root = AssetRoot::new(pack_directory.clone());
    let loaded = crystal_assets::read_loaded_verified_compiled_game_pack(&pack_path)
        .with_context(|| format!("load compiled game pack {}", pack_path.display()))?;
    let runtime = CrystalRuntime::from_loaded_compiled_pack(&asset_root, loaded)?;
    let default_save_path = default_save_path(&pack_directory, &runtime);
    let start = match args.load_save {
        Some(save_path) => BevyShellStart::LoadSave { save_path },
        None => BevyShellStart::Title {
            spawn_identifier: runtime.title_new_game_spawn_identifier()?,
            save_path: default_save_path
                .exists()
                .then_some(default_save_path.clone()),
        },
    };

    crystal_bevy::run_bevy_shell(
        asset_root,
        runtime,
        start,
        BevyShellConfig {
            quick_save_path: Some(args.save_path.unwrap_or(default_save_path)),
            ..Default::default()
        },
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    help: bool,
    pack: Option<String>,
    load_save: Option<PathBuf>,
    save_path: Option<PathBuf>,
}

fn parse_args_from(values: impl IntoIterator<Item = String>) -> Result<Args> {
    let mut args = Args::default();
    let mut values = values.into_iter();
    while let Some(arg) = values.next() {
        match arg.as_str() {
            "-h" | "--help" => args.help = true,
            "--pack" => {
                let value = values
                    .next()
                    .context("--pack requires a .crystalpack path")?;
                if args.pack.is_some() {
                    bail!("--pack may be provided only once");
                }
                args.pack = Some(parse_pack_arg("--pack", value)?);
            }
            "--load-save" => {
                let value = values.next().context("--load-save requires a path")?;
                if args.load_save.is_some() {
                    bail!("--load-save may be provided only once");
                }
                args.load_save = Some(parse_save_path_arg("--load-save", value)?);
            }
            "--save-path" => {
                let value = values.next().context("--save-path requires a path")?;
                if args.save_path.is_some() {
                    bail!("--save-path may be provided only once");
                }
                args.save_path = Some(parse_save_path_arg("--save-path", value)?);
            }
            other => bail!("unknown argument '{other}'"),
        }
    }
    Ok(args)
}

fn resolve_pack_path(explicit_pack: Option<&str>) -> Result<PathBuf> {
    let executable = env::current_exe().context("resolve current executable path")?;
    resolve_pack_path_from(explicit_pack, &executable)
}

fn resolve_pack_path_from(explicit_pack: Option<&str>, executable: &Path) -> Result<PathBuf> {
    if let Some(explicit_pack) = explicit_pack {
        return Ok(PathBuf::from(explicit_pack));
    }
    let executable_directory = executable
        .parent()
        .context("current executable path has no parent directory")?;
    Ok(executable_directory.join(DEFAULT_PACK_FILENAME))
}

fn parse_pack_arg(flag: &str, value: String) -> Result<String> {
    if Path::new(&value)
        .extension()
        .and_then(|value| value.to_str())
        != Some(COMPILED_GAME_PACK_EXTENSION)
    {
        bail!("{flag} path must end in .{COMPILED_GAME_PACK_EXTENSION}");
    }
    Ok(value)
}

fn parse_save_path_arg(flag: &str, value: String) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    if path.extension().and_then(|value| value.to_str()) != Some(SAVE_EXTENSION) {
        bail!("{flag} path must end in .{SAVE_EXTENSION}");
    }
    Ok(path)
}

fn default_save_path(pack_directory: &Path, runtime: &CrystalRuntime) -> PathBuf {
    pack_directory.join("saves").join(format!(
        "{}-{}-{}.{}",
        runtime.modpack().id(),
        runtime.modpack().hash(),
        runtime.pack_identity().content_hash,
        SAVE_EXTENSION
    ))
}

fn print_usage() {
    println!(
        "crystal-bevy [--pack <path.crystalpack>] [--load-save <path.{SAVE_EXTENSION}>] [--save-path <path.{SAVE_EXTENSION}>]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_argument_surface_contains_only_pack_and_save_configuration() {
        let args = parse_args_from([
            "--pack".to_string(),
            "/tmp/game.crystalpack".to_string(),
            "--load-save".to_string(),
            "/tmp/input.crystalsave".to_string(),
            "--save-path".to_string(),
            "/tmp/output.crystalsave".to_string(),
        ])
        .expect("release arguments");
        assert_eq!(
            args,
            Args {
                help: false,
                pack: Some("/tmp/game.crystalpack".to_string()),
                load_save: Some(PathBuf::from("/tmp/input.crystalsave")),
                save_path: Some(PathBuf::from("/tmp/output.crystalsave")),
            }
        );

        for forbidden in [
            "--spawn",
            "--smoke-shop",
            "--smoke-visible-overworld",
            "--list-spawns",
            "--list-script",
        ] {
            let error = parse_args_from([forbidden.to_string()])
                .expect_err("debug argument must not be accepted");
            assert_eq!(error.to_string(), format!("unknown argument '{forbidden}'"));
        }
    }

    #[test]
    fn default_pack_is_adjacent_to_the_executable() {
        assert_eq!(
            resolve_pack_path_from(None, Path::new("/opt/crystal/crystal-bevy"))
                .expect("default pack path"),
            PathBuf::from("/opt/crystal/core-modular.crystalpack")
        );
    }

    #[test]
    fn pack_and_save_extensions_are_required() {
        assert!(parse_pack_arg("--pack", "/tmp/game.json".to_string()).is_err());
        assert!(parse_save_path_arg("--load-save", "/tmp/save.json".to_string()).is_err());
    }

    #[test]
    fn release_api_has_no_direct_new_game_or_arbitrary_tile_entry() {
        let runtime_source = include_str!("lib.rs");
        let shell_source = include_str!("bevy_shell.rs");
        let asset_source = concat!(
            include_str!("../../crystal-assets/src/lib.rs"),
            include_str!("../../crystal-assets/src/content_pack.rs"),
            include_str!("../../crystal-assets/src/map_modules.rs"),
            include_str!("../../crystal-assets/src/runtime_pack.rs"),
            include_str!("../../crystal-assets/src/verification.rs"),
            include_str!("../../crystal-assets/src/runtime_commands.rs"),
            include_str!("../../crystal-assets/src/game_data.rs"),
            include_str!("../../crystal-assets/src/mutation_protocol.rs"),
            include_str!("../../crystal-assets/src/merge.rs"),
            include_str!("../../crystal-assets/src/script_parsing.rs"),
        );
        assert!(!runtime_source.contains("pub fn new_game("));
        assert!(!runtime_source.contains("pub fn new_game_at_runtime_tile("));
        assert!(!runtime_source.contains("pub fn start_overworld_session("));
        assert!(!runtime_source.contains("pub fn start_overworld_session_at_runtime_tile("));
        assert!(
            shell_source
                .contains("#[cfg(test)]\n    NewGame {\n        spawn_identifier: u16,\n    }")
        );
        assert!(
            shell_source.contains(
                "#[cfg(test)]\n    NewGameAtRuntimeTile {\n        spawn_identifier: u16,"
            )
        );
        assert!(asset_source.contains(
            "#[cfg(any(test, feature = \"test-fixtures\"))]\n    pub fn start_overworld_session_at_runtime_tile("
        ));
        let core_state_source = include_str!("../../crystal-core/src/state.rs");
        assert!(core_state_source.contains(
            "#[cfg(any(test, feature = \"test-fixtures\"))]\nimpl Default for GameState"
        ));
        let production_asset_source = asset_source
            .split("#[cfg(test)]\nmod tests")
            .next()
            .expect("production asset source");
        assert!(!production_asset_source.contains("GameState::default()"));
        assert!(production_asset_source.contains("GameState::reset_wram_for_new_game()"));
    }
}
