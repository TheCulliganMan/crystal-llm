use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use crystal_bevy::{
    BevyMultiplayerConfig, BevyShellConfig, BevyShellStart, CrystalRuntime,
    assets::{AssetRoot, modpack::COMPILED_GAME_PACK_EXTENSION},
    core::save::SAVE_EXTENSION,
};

const DEFAULT_PACK_FILENAME: &str = "core-modular.crystalpack";
#[cfg(target_arch = "wasm32")]
const DEFAULT_BROWSER_PACK_FILENAME: &str = "core-modular.browser.crystalpack";

#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<()> {
    let args = parse_args_from(env::args().skip(1))?;
    if args.help {
        print_usage();
        return Ok(());
    }
    let multiplayer = multiplayer_config(&args)?;

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
            // Preserve existing-primary visibility; only a missing primary
            // enters the core loader's validated primary-to-backup recovery.
            save_path: (default_save_path.exists()
                || runtime.load_save_summary(&default_save_path).is_ok())
            .then_some(default_save_path.clone()),
        },
    };

    crystal_bevy::run_bevy_shell(
        asset_root,
        runtime,
        start,
        BevyShellConfig {
            quick_save_path: Some(args.save_path.unwrap_or(default_save_path)),
            multiplayer,
            ..Default::default()
        },
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = run_browser().await {
            panic!("start crystal-bevy browser runtime: {error:#}");
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn run_browser() -> Result<()> {
    let pack_bytes = fetch_browser_pack().await?;
    let loaded = crystal_assets::load_verified_compiled_game_pack_bytes(
        DEFAULT_BROWSER_PACK_FILENAME,
        pack_bytes,
    )?;
    let asset_root = AssetRoot::new(".");
    let runtime = CrystalRuntime::from_loaded_compiled_pack(&asset_root, loaded)?;
    let spawn_identifier = runtime.title_new_game_spawn_identifier()?;
    let multiplayer = browser_multiplayer_config()?;
    let save_path = browser_save_path_for_identity(
        runtime.modpack().id(),
        multiplayer.as_ref().map(|config| config.player_id),
    );
    let continue_save_path = runtime
        .load_save_summary(&save_path)
        .is_ok()
        .then_some(save_path.clone());
    let config = BevyShellConfig {
        quick_save_path: Some(save_path),
        multiplayer,
        ..Default::default()
    };
    crystal_bevy::run_bevy_shell(
        asset_root,
        runtime,
        match continue_save_path {
            Some(save_path) => BevyShellStart::LoadSave { save_path },
            None => BevyShellStart::Title { spawn_identifier, save_path: None },
        },
        config,
    )
}

fn browser_save_path_for_identity(modpack_id: &str, player_id: Option<u64>) -> PathBuf {
    let identity = player_id
        .map(|player_id| format!("player-{player_id}"))
        .unwrap_or_else(|| "local".to_string());
    PathBuf::from("saves").join(format!("{modpack_id}-{identity}.{SAVE_EXTENSION}"))
}

#[cfg(target_arch = "wasm32")]
fn browser_multiplayer_config() -> Result<Option<BevyMultiplayerConfig>> {
    let window = web_sys::window().context("browser window is unavailable")?;
    let location = window.location();
    let params = web_sys::UrlSearchParams::new_with_str(&location.search().unwrap_or_default())
        .map_err(|error| anyhow::anyhow!("parse browser multiplayer query: {error:?}"))?;
    if params.get("multiplayer").as_deref() == Some("off") {
        return Ok(None);
    }
    let server_url = match params.get("multiplayer_server") {
        Some(value) => value,
        None => {
            let scheme = if location.protocol().unwrap_or_default() == "https:" {
                "wss"
            } else {
                "ws"
            };
            format!("{scheme}://{}/v1/ws", location.host().unwrap_or_default())
        }
    };
    // The page selects and locks this tab's identity before starting WASM.
    let storage = window
        .session_storage()
        .map_err(|error| anyhow::anyhow!("access browser session storage: {error:?}"))?
        .context("browser session storage is unavailable")?;
    let player_id = storage
        .get_item("crystal.multiplayer.player_id")
        .map_err(|error| anyhow::anyhow!("read browser player identity: {error:?}"))?
        .context("browser player identity was not initialized")?
        .parse::<u64>()
        .context("browser player identity is invalid")?;
    anyhow::ensure!(player_id > 0, "browser player identity must be nonzero");
    let server_token = storage
        .get_item("crystal.multiplayer.token")
        .map_err(|error| anyhow::anyhow!("read browser authentication: {error:?}"))?;
    let display_name = params
        .get("player_name")
        .unwrap_or_else(|| format!("PLAYER{:04}", player_id % 10_000));
    Ok(Some(BevyMultiplayerConfig {
        server_url,
        server_token,
        world_id: params.get("world").unwrap_or_else(|| "main".into()),
        player_id,
        display_name,
        rating: params
            .get("rating")
            .and_then(|value| value.parse().ok())
            .unwrap_or(1000),
        rating_range: params
            .get("rating_range")
            .and_then(|value| value.parse().ok())
            .unwrap_or(200),
    }))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_browser_pack() -> Result<Vec<u8>> {
    use wasm_bindgen::JsCast as _;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().context("browser window is unavailable")?;
    let response = JsFuture::from(window.fetch_with_str(DEFAULT_BROWSER_PACK_FILENAME))
        .await
        .map_err(|error| anyhow::anyhow!("fetch {DEFAULT_BROWSER_PACK_FILENAME}: {error:?}"))?
        .dyn_into::<web_sys::Response>()
        .map_err(|error| {
            anyhow::anyhow!("decode {DEFAULT_BROWSER_PACK_FILENAME} response: {error:?}")
        })?;
    if !response.ok() {
        bail!(
            "fetch {DEFAULT_BROWSER_PACK_FILENAME}: HTTP {} {}",
            response.status(),
            response.status_text()
        );
    }
    let buffer = response.array_buffer().map_err(|error| {
        anyhow::anyhow!("read {DEFAULT_BROWSER_PACK_FILENAME} response: {error:?}")
    })?;
    let buffer = JsFuture::from(buffer).await.map_err(|error| {
        anyhow::anyhow!("read {DEFAULT_BROWSER_PACK_FILENAME} bytes: {error:?}")
    })?;
    let bytes = js_sys::Uint8Array::new(&buffer);
    let mut pack = vec![0; bytes.length() as usize];
    bytes.copy_to(&mut pack);
    Ok(pack)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    help: bool,
    pack: Option<String>,
    load_save: Option<PathBuf>,
    save_path: Option<PathBuf>,
    multiplayer_server: Option<String>,
    multiplayer_token: Option<String>,
    multiplayer_world: Option<String>,
    multiplayer_player_id: Option<u64>,
    multiplayer_player_name: Option<String>,
    multiplayer_rating: Option<i32>,
    multiplayer_rating_range: Option<u32>,
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
            "--multiplayer-server" => {
                let value = values
                    .next()
                    .context("--multiplayer-server requires a WebSocket URL")?;
                if args.multiplayer_server.is_some() {
                    bail!("--multiplayer-server may be provided only once");
                }
                args.multiplayer_server = Some(value);
            }
            "--multiplayer-token" => {
                let value = values
                    .next()
                    .context("--multiplayer-token requires a token")?;
                if args.multiplayer_token.is_some() {
                    bail!("--multiplayer-token may be provided only once");
                }
                args.multiplayer_token = Some(value);
            }
            "--multiplayer-world" => {
                let value = values
                    .next()
                    .context("--multiplayer-world requires a world id")?;
                if args.multiplayer_world.is_some() {
                    bail!("--multiplayer-world may be provided only once");
                }
                args.multiplayer_world = Some(value);
            }
            "--multiplayer-player-id" => {
                let value = values
                    .next()
                    .context("--multiplayer-player-id requires a positive integer")?;
                if args.multiplayer_player_id.is_some() {
                    bail!("--multiplayer-player-id may be provided only once");
                }
                args.multiplayer_player_id = Some(
                    value
                        .parse::<u64>()
                        .with_context(|| format!("invalid multiplayer player id '{value}'"))?,
                );
            }
            "--multiplayer-player-name" => {
                let value = values
                    .next()
                    .context("--multiplayer-player-name requires a display name")?;
                if args.multiplayer_player_name.is_some() {
                    bail!("--multiplayer-player-name may be provided only once");
                }
                args.multiplayer_player_name = Some(value);
            }
            "--multiplayer-rating" => {
                let value = values
                    .next()
                    .context("--multiplayer-rating requires an integer")?;
                args.multiplayer_rating = Some(
                    value
                        .parse()
                        .with_context(|| format!("invalid multiplayer rating '{value}'"))?,
                );
            }
            "--multiplayer-rating-range" => {
                let value = values
                    .next()
                    .context("--multiplayer-rating-range requires an integer")?;
                args.multiplayer_rating_range = Some(
                    value
                        .parse()
                        .with_context(|| format!("invalid multiplayer rating range '{value}'"))?,
                );
            }
            other => bail!("unknown argument '{other}'"),
        }
    }
    validate_multiplayer_flags(&args)?;
    Ok(args)
}

fn validate_multiplayer_flags(args: &Args) -> Result<()> {
    let mode_selected = args.multiplayer_server.is_some();
    let details_selected = args.multiplayer_token.is_some()
        || args.multiplayer_world.is_some()
        || args.multiplayer_player_id.is_some()
        || args.multiplayer_player_name.is_some()
        || args.multiplayer_rating.is_some()
        || args.multiplayer_rating_range.is_some();
    if !mode_selected && details_selected {
        bail!("multiplayer options require --multiplayer-server");
    }
    if mode_selected {
        if args.multiplayer_player_id.is_none() {
            bail!("multiplayer requires --multiplayer-player-id");
        }
        if args.multiplayer_player_name.is_none() {
            bail!("multiplayer requires --multiplayer-player-name");
        }
    }
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn multiplayer_config(args: &Args) -> Result<Option<BevyMultiplayerConfig>> {
    validate_multiplayer_flags(args)?;
    let Some(server_url) = &args.multiplayer_server else {
        return Ok(None);
    };
    Ok(Some(BevyMultiplayerConfig {
        server_url: server_url.clone(),
        server_token: args.multiplayer_token.clone(),
        world_id: args
            .multiplayer_world
            .clone()
            .unwrap_or_else(|| "main".into()),
        player_id: args
            .multiplayer_player_id
            .context("validated multiplayer player id")?,
        display_name: args
            .multiplayer_player_name
            .clone()
            .context("validated multiplayer player name")?,
        rating: args.multiplayer_rating.unwrap_or(1000),
        rating_range: args.multiplayer_rating_range.unwrap_or(200),
    }))
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
        "crystal-bevy [--pack <path.crystalpack>] [--load-save <path.{SAVE_EXTENSION}>] [--save-path <path.{SAVE_EXTENSION}>] [--multiplayer-server <ws-url> --multiplayer-player-id <id> --multiplayer-player-name <name>] [--multiplayer-token <token>] [--multiplayer-world <id>]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_save_paths_are_stable_and_scoped_to_multiplayer_identity() {
        assert_eq!(
            browser_save_path_for_identity("core-modular", Some(42)),
            PathBuf::from("saves/core-modular-player-42.crystalsave")
        );
        assert_eq!(
            browser_save_path_for_identity("core-modular", None),
            PathBuf::from("saves/core-modular-local.crystalsave")
        );
    }

    #[test]
    fn release_argument_surface_accepts_pack_save_and_multiplayer_configuration() {
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
                multiplayer_server: None,
                multiplayer_token: None,
                multiplayer_world: None,
                multiplayer_player_id: None,
                multiplayer_player_name: None,
                multiplayer_rating: None,
                multiplayer_rating_range: None,
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
    fn multiplayer_arguments_require_a_hosted_server_and_complete_identity() {
        let args = parse_args_from([
            "--multiplayer-server".to_string(),
            "ws://127.0.0.1:3003/v1/ws".to_string(),
            "--multiplayer-player-id".to_string(),
            "1".to_string(),
            "--multiplayer-player-name".to_string(),
            "CHRIS".to_string(),
        ])
        .expect("hosted multiplayer arguments");
        assert_eq!(
            multiplayer_config(&args).expect("hosted multiplayer config"),
            Some(BevyMultiplayerConfig {
                server_url: "ws://127.0.0.1:3003/v1/ws".to_string(),
                server_token: None,
                world_id: "main".to_string(),
                player_id: 1,
                display_name: "CHRIS".to_string(),
                rating: 1000,
                rating_range: 200,
            })
        );

        assert!(parse_args_from(["--multiplayer-world".to_string(), "main".to_string(),]).is_err());
        assert!(
            parse_args_from([
                "--multiplayer-server".to_string(),
                "ws://127.0.0.1:3003/v1/ws".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn wasm_loads_the_pack_at_runtime_instead_of_embedding_it() {
        let source = include_str!("main.rs");
        let embedded_bytes_macro = ["include", "_bytes!"].concat();
        assert!(
            !source.contains(&embedded_bytes_macro),
            "the compiled pack must not be copied into the WASM binary"
        );
        assert!(source.contains("fetch_browser_pack().await"));
        assert!(source.contains("window.fetch_with_str(DEFAULT_BROWSER_PACK_FILENAME)"));
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
                .contains("#[cfg(any(test, feature = \"location-tester\"))]\n    NewGame {\n        spawn_identifier: u16,\n    }")
        );
        assert!(
            shell_source.contains(
                "#[cfg(any(test, feature = \"location-tester\"))]\n    NewGameAtRuntimeTile {\n        spawn_identifier: u16,"
            )
        );
        assert!(asset_source.contains(
            "#[cfg(any(test, feature = \"test-fixtures\"))]\n    pub fn start_overworld_session_at_runtime_tile("
        ));
        let core_state_source = include_str!("../../crystal-core/src/state.rs");
        assert!(core_state_source.contains(
            "#[cfg(any(test, feature = \"test-fixtures\"))]\nimpl Default for GameState"
        ));
        for source in [
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
        ] {
            let production_source = source
                .split("\n#[cfg(test)]\nmod ")
                .next()
                .expect("production asset source");
            assert!(!production_source.contains("GameState::default()"));
        }
        assert!(asset_source.contains("GameState::reset_wram_for_new_game()"));
    }
}
