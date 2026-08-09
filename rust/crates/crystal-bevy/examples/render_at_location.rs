use std::{env, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use crystal_bevy::{
    BevyShellConfig, BevyShellStart, CrystalRuntime,
    assets::{AssetRoot, read_loaded_verified_compiled_game_pack},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewMode {
    TwoD,
    TwoPointFiveD,
    Both,
}

impl ViewMode {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "2d" | "classic" => Ok(Self::TwoD),
            "2.5d" | "voxel" => Ok(Self::TwoPointFiveD),
            "both" | "compare" => Ok(Self::Both),
            _ => bail!("--view must be '2d', '2.5d', or 'both'"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::TwoD => "2D",
            Self::TwoPointFiveD => "2.5D",
            Self::Both => "2D + 2.5D",
        }
    }
}

#[derive(Debug)]
struct Args {
    pack: PathBuf,
    map: Option<String>,
    maps: Vec<String>,
    all_maps: bool,
    tile_x: Option<i16>,
    tile_y: Option<i16>,
    view: ViewMode,
    list_maps: bool,
    screenshot: Option<PathBuf>,
    output_dir: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = parse_args(env::args().skip(1))?;
    if args.view == ViewMode::Both && !args.all_maps && args.maps.is_empty() {
        return render_both(&args);
    }
    let pack_path = args
        .pack
        .canonicalize()
        .with_context(|| format!("resolve compiled pack {}", args.pack.display()))?;
    let pack_directory = pack_path
        .parent()
        .context("compiled pack has no parent directory")?;
    let asset_root = AssetRoot::new(pack_directory.to_path_buf());
    let loaded = read_loaded_verified_compiled_game_pack(&pack_path)
        .with_context(|| format!("load compiled pack {}", pack_path.display()))?;
    let runtime = CrystalRuntime::from_loaded_compiled_pack(&asset_root, loaded)?;

    if args.list_maps {
        print_map_catalog(&runtime);
        return Ok(());
    }

    if args.all_maps || !args.maps.is_empty() {
        return render_map_batch(&args, &runtime);
    }

    let map = args
        .map
        .context("--map is required unless --list-maps is used")?;
    let (width, height) = runtime
        .data()
        .saved_map_tile_bounds(&map)
        .with_context(|| format!("unknown or dimensionless map {map}"))?;
    let tile_x = args
        .tile_x
        .unwrap_or_else(|| i16::try_from(width / 2).unwrap_or(i16::MAX));
    let tile_y = args
        .tile_y
        .unwrap_or_else(|| i16::try_from(height / 2).unwrap_or(i16::MAX));
    require_in_bounds(&map, tile_x, tile_y, width, height)?;

    let tileset = runtime.data().map_tileset_name(&map)?;
    let environment = runtime.data().map_environment(&map)?;
    println!(
        "rendering {map} at ({tile_x}, {tile_y}) in {} [{width}x{height}, {tileset}, {environment}]",
        args.view.label()
    );
    println!("press F3 to toggle faithful 2D / optional 2.5D at the same location");

    crystal_bevy::run_bevy_shell(
        asset_root,
        runtime.clone(),
        BevyShellStart::NewGameAtRuntimeTile {
            spawn_identifier: runtime.title_new_game_spawn_identifier()?,
            map_name: map.clone(),
            tile_x,
            tile_y,
        },
        BevyShellConfig {
            smoke_player_name: Some("RENDER".to_string()),
            voxel_view_enabled: Some(args.view == ViewMode::TwoPointFiveD),
            window_title: Some(format!(
                "Crystal Render Tester — {map} ({tile_x}, {tile_y}) — {} — F3 toggles",
                args.view.label()
            )),
            render_test_screenshot: args.screenshot,
            ..Default::default()
        },
    )
}

fn render_map_batch(args: &Args, runtime: &CrystalRuntime) -> Result<()> {
    let output_dir = args
        .output_dir
        .as_ref()
        .context("--output-dir <directory> is required with --maps or --all-maps")?;
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("create batch output directory {}", output_dir.display()))?;
    let executable = env::current_exe().context("resolve render tester executable")?;
    let maps = if args.all_maps {
        runtime.map_ids().into_iter().collect::<Vec<_>>()
    } else {
        args.maps.clone()
    };
    if maps.is_empty() {
        bail!("batch map selection is empty");
    }

    for map in maps {
        let (width, height) = runtime
            .data()
            .saved_map_tile_bounds(&map)
            .with_context(|| format!("unknown or dimensionless map {map}"))?;
        let (tile_x, tile_y) = nearest_walkable_tile(runtime, &map, width, height)
            .with_context(|| format!("find a walkable batch location on {map}"))?;
        let output = output_dir.join(format!("{}.png", safe_file_stem(&map)));
        let view = match args.view {
            ViewMode::TwoD => "2d",
            ViewMode::TwoPointFiveD => "2.5d",
            ViewMode::Both => "both",
        };
        let status = Command::new(&executable)
            .arg("--pack")
            .arg(&args.pack)
            .arg("--map")
            .arg(&map)
            .arg("--x")
            .arg(tile_x.to_string())
            .arg("--y")
            .arg(tile_y.to_string())
            .arg("--view")
            .arg(view)
            .arg("--screenshot")
            .arg(&output)
            .status()
            .with_context(|| format!("launch batch render for {map}"))?;
        if !status.success() {
            bail!("batch render for {map} exited with {status}");
        }
    }
    Ok(())
}

fn nearest_walkable_tile(
    runtime: &CrystalRuntime,
    map: &str,
    width: u16,
    height: u16,
) -> Result<(i16, i16)> {
    let center_x = i32::from(width / 2);
    let center_y = i32::from(height / 2);
    let mut candidates = (0..height)
        .flat_map(|y| (0..width).map(move |x| (x, y)))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|&(x, y)| {
        let dx = i32::from(x) - center_x;
        let dy = i32::from(y) - center_y;
        (dx * dx + dy * dy, y, x)
    });
    for (x, y) in candidates {
        let x = i16::try_from(x)?;
        let y = i16::try_from(y)?;
        let tile = crystal_bevy::core::world::map::TilePosition::new(x, y);
        if runtime.data().overworld_session(map, tile, 0).is_ok() {
            return Ok((x, y));
        }
    }
    bail!("map has no walkable runtime tile")
}

fn safe_file_stem(map: &str) -> String {
    map.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn render_both(args: &Args) -> Result<()> {
    let map = args
        .map
        .as_deref()
        .context("--map is required for --view both")?;
    let output = args
        .screenshot
        .as_ref()
        .context("--screenshot <prefix.png> is required for --view both")?;
    let executable = env::current_exe().context("resolve render tester executable")?;

    for (view, suffix) in [("2d", "2d"), ("2.5d", "2.5d")] {
        let output = suffixed_output(output, suffix);
        let mut command = Command::new(&executable);
        command
            .arg("--pack")
            .arg(&args.pack)
            .arg("--map")
            .arg(map)
            .arg("--view")
            .arg(view)
            .arg("--screenshot")
            .arg(&output);
        if let Some(x) = args.tile_x {
            command.arg("--x").arg(x.to_string());
        }
        if let Some(y) = args.tile_y {
            command.arg("--y").arg(y.to_string());
        }
        let status = command
            .status()
            .with_context(|| format!("launch {view} render for {map}"))?;
        if !status.success() {
            bail!("{view} render for {map} exited with {status}");
        }
        println!("wrote {}", output.display());
    }
    Ok(())
}

fn suffixed_output(output: &std::path::Path, suffix: &str) -> PathBuf {
    let parent = output.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = output
        .file_stem()
        .unwrap_or(output.as_os_str())
        .to_string_lossy();
    let extension = output
        .extension()
        .map(|value| format!(".{}", value.to_string_lossy()))
        .unwrap_or_default();
    parent.join(format!("{stem}-{suffix}{extension}"))
}

fn print_map_catalog(runtime: &CrystalRuntime) {
    println!("map\twidth\theight\ttileset\tenvironment");
    for map in runtime.map_ids() {
        let Some((width, height)) = runtime.data().saved_map_tile_bounds(&map) else {
            continue;
        };
        let tileset = runtime.data().map_tileset_name(&map).unwrap_or("?");
        let environment = runtime.data().map_environment(&map).unwrap_or("?");
        println!("{map}\t{width}\t{height}\t{tileset}\t{environment}");
    }
}

fn require_in_bounds(map: &str, x: i16, y: i16, width: u16, height: u16) -> Result<()> {
    if x < 0 || y < 0 || u16::try_from(x)? >= width || u16::try_from(y)? >= height {
        bail!(
            "location {map} ({x}, {y}) is outside gameplay tile bounds 0..{} x 0..{}",
            width.saturating_sub(1),
            height.saturating_sub(1)
        );
    }
    Ok(())
}

fn parse_args(values: impl IntoIterator<Item = String>) -> Result<Args> {
    let mut pack = None;
    let mut map = None;
    let mut maps = Vec::new();
    let mut all_maps = false;
    let mut tile_x = None;
    let mut tile_y = None;
    let mut view = ViewMode::TwoD;
    let mut list_maps = false;
    let mut screenshot = None;
    let mut output_dir = None;
    let mut values = values.into_iter();
    while let Some(flag) = values.next() {
        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(next_value(&mut values, "--pack")?)),
            "--map" => map = Some(next_value(&mut values, "--map")?),
            "--maps" => maps.extend(
                next_value(&mut values, "--maps")?
                    .split(',')
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
            ),
            "--all-maps" => all_maps = true,
            "--x" => tile_x = Some(next_value(&mut values, "--x")?.parse::<i16>()?),
            "--y" => tile_y = Some(next_value(&mut values, "--y")?.parse::<i16>()?),
            "--view" => view = ViewMode::parse(&next_value(&mut values, "--view")?)?,
            "--list-maps" => list_maps = true,
            "--screenshot" => {
                screenshot = Some(PathBuf::from(next_value(&mut values, "--screenshot")?))
            }
            "--output-dir" => {
                output_dir = Some(PathBuf::from(next_value(&mut values, "--output-dir")?))
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ => bail!("unknown argument '{flag}'"),
        }
    }
    Ok(Args {
        pack: pack.context("--pack <game.crystalpack> is required")?,
        map,
        maps,
        all_maps,
        tile_x,
        tile_y,
        view,
        list_maps,
        screenshot,
        output_dir,
    })
}

fn next_value(values: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    values
        .next()
        .with_context(|| format!("{flag} requires a value"))
}

fn print_usage() {
    println!(
        "cargo run -p crystal-bevy --example render_at_location --features location-tester -- \\\n+         --pack <game.crystalpack> [--list-maps | --map <id> [--x <tile>] [--y <tile>] \\\n+         [--view 2d|2.5d|both] [--screenshot <output-or-prefix.png>] | \\\n+         (--maps <id,id,...> | --all-maps) --output-dir <directory> \\\n+         [--view 2d|2.5d|both]]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_render_modes() {
        assert_eq!(ViewMode::parse("2d").unwrap(), ViewMode::TwoD);
        assert_eq!(ViewMode::parse("2.5d").unwrap(), ViewMode::TwoPointFiveD);
        assert_eq!(ViewMode::parse("both").unwrap(), ViewMode::Both);
        assert!(ViewMode::parse("3d").is_err());
    }

    #[test]
    fn comparison_outputs_are_labeled_without_losing_extension() {
        assert_eq!(
            suffixed_output(std::path::Path::new("/tmp/NewBark.png"), "2.5d"),
            PathBuf::from("/tmp/NewBark-2.5d.png")
        );
        assert_eq!(
            suffixed_output(std::path::Path::new("comparison"), "2d"),
            PathBuf::from("comparison-2d")
        );
    }

    #[test]
    fn rejects_locations_outside_map_bounds() {
        assert!(require_in_bounds("Map", 0, 0, 10, 8).is_ok());
        assert!(require_in_bounds("Map", 10, 0, 10, 8).is_err());
        assert!(require_in_bounds("Map", -1, 0, 10, 8).is_err());
    }

    #[test]
    fn batch_output_stems_are_filesystem_safe() {
        assert_eq!(safe_file_stem("NewBarkTown"), "NewBarkTown");
        assert_eq!(safe_file_stem("Map / Test"), "Map___Test");
    }
}
