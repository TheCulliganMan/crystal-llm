use std::{env, path::PathBuf};

use anyhow::{Context, Result, bail};
use crystal_bevy::{
    BevyShellConfig, BevyShellStart, CrystalRuntime,
    assets::{AssetRoot, read_loaded_verified_compiled_game_pack},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewMode {
    TwoD,
    TwoPointFiveD,
}

impl ViewMode {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "2d" | "classic" => Ok(Self::TwoD),
            "2.5d" | "voxel" => Ok(Self::TwoPointFiveD),
            _ => bail!("--view must be '2d' or '2.5d'"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::TwoD => "2D",
            Self::TwoPointFiveD => "2.5D",
        }
    }
}

#[derive(Debug)]
struct Args {
    pack: PathBuf,
    map: Option<String>,
    tile_x: Option<i16>,
    tile_y: Option<i16>,
    view: ViewMode,
    list_maps: bool,
    screenshot: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = parse_args(env::args().skip(1))?;
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
    let mut tile_x = None;
    let mut tile_y = None;
    let mut view = ViewMode::TwoD;
    let mut list_maps = false;
    let mut screenshot = None;
    let mut values = values.into_iter();
    while let Some(flag) = values.next() {
        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(next_value(&mut values, "--pack")?)),
            "--map" => map = Some(next_value(&mut values, "--map")?),
            "--x" => tile_x = Some(next_value(&mut values, "--x")?.parse::<i16>()?),
            "--y" => tile_y = Some(next_value(&mut values, "--y")?.parse::<i16>()?),
            "--view" => view = ViewMode::parse(&next_value(&mut values, "--view")?)?,
            "--list-maps" => list_maps = true,
            "--screenshot" => {
                screenshot = Some(PathBuf::from(next_value(&mut values, "--screenshot")?))
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
        tile_x,
        tile_y,
        view,
        list_maps,
        screenshot,
    })
}

fn next_value(values: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    values
        .next()
        .with_context(|| format!("{flag} requires a value"))
}

fn print_usage() {
    println!(
        "cargo run -p crystal-bevy --example render_at_location --features location-tester -- \\\n         --pack <game.crystalpack> [--list-maps | --map <id> [--x <tile>] [--y <tile>] \\\n         [--view 2d|2.5d] [--screenshot <output.png>]]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_render_modes() {
        assert_eq!(ViewMode::parse("2d").unwrap(), ViewMode::TwoD);
        assert_eq!(ViewMode::parse("2.5d").unwrap(), ViewMode::TwoPointFiveD);
        assert!(ViewMode::parse("3d").is_err());
    }

    #[test]
    fn rejects_locations_outside_map_bounds() {
        assert!(require_in_bounds("Map", 0, 0, 10, 8).is_ok());
        assert!(require_in_bounds("Map", 10, 0, 10, 8).is_err());
        assert!(require_in_bounds("Map", -1, 0, 10, 8).is_err());
    }
}
