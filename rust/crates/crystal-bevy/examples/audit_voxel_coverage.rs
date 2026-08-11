use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::PathBuf,
    sync::Arc,
};

use anyhow::{Context, Result, bail};
use bevy::prelude::{Handle, Image};
use crystal_bevy::{
    CrystalRuntime,
    assets::{AssetRoot, read_loaded_verified_compiled_game_pack},
};
use crystal_render_api::{VisualTile, VisualTileSource};
use crystal_voxel_view::audit_cell_coverage_on_map;
use serde_json::json;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SourceKey {
    coverage: &'static str,
    tileset: String,
    metatile: u16,
    subtile_column: u8,
    subtile_row: u8,
    tile_index: u16,
}

#[derive(Clone, Debug)]
struct SourceFinding {
    count: u64,
    maps: BTreeSet<String>,
    sample_map: String,
    sample_x: u32,
    sample_y: u32,
}

fn main() -> Result<()> {
    let (pack_path, output_path) = parse_args(env::args().skip(1))?;
    let pack_path = pack_path
        .canonicalize()
        .with_context(|| format!("resolve compiled pack {}", pack_path.display()))?;
    let repository_root = pack_path
        .ancestors()
        .find(|ancestor| ancestor.join("apps/web/assets/data/tilesets").is_dir())
        .context("locate repository runtime assets above compiled pack")?;
    let asset_root = AssetRoot::new(repository_root.to_path_buf());
    let loaded = read_loaded_verified_compiled_game_pack(&pack_path)
        .with_context(|| format!("load compiled pack {}", pack_path.display()))?;
    let runtime = CrystalRuntime::from_loaded_compiled_pack(&asset_root, loaded)?;

    let mut layouts = BTreeMap::<String, Vec<u8>>::new();
    let mut findings = BTreeMap::<SourceKey, SourceFinding>::new();
    let mut coverage_totals = BTreeMap::<&'static str, u64>::new();
    let mut map_reports = Vec::new();
    let mut map_ids = runtime.map_ids().into_iter().collect::<Vec<_>>();
    map_ids.sort();

    for map_id in &map_ids {
        let module = runtime
            .data()
            .maps
            .get(map_id)
            .with_context(|| format!("map catalog contains missing module {map_id}"))?;
        let tileset = module.attributes.tileset_name.as_str();
        let layout = if let Some(layout) = layouts.get(tileset) {
            layout
        } else {
            let path = asset_root
                .runtime_assets()
                .join("data/tilesets")
                .join(format!("{tileset}_metatiles.bin"));
            let bytes = fs::read(&path)
                .with_context(|| format!("read tileset layout {}", path.display()))?;
            anyhow::ensure!(
                bytes.len() % 16 == 0,
                "tileset {tileset} layout length {} is not divisible by 16",
                bytes.len()
            );
            layouts.entry(tileset.to_owned()).or_insert(bytes)
        };
        let width = usize::from(module.attributes.width) * 4;
        let height = usize::from(module.attributes.height) * 4;
        anyhow::ensure!(
            module.blocks.len()
                == usize::from(module.attributes.width) * usize::from(module.attributes.height),
            "map {map_id} block count does not match dimensions"
        );
        let mut tiles = Vec::with_capacity(width * height);
        for block_y in 0..usize::from(module.attributes.height) {
            for block_x in 0..usize::from(module.attributes.width) {
                let metatile =
                    module.blocks[block_y * usize::from(module.attributes.width) + block_x];
                let base = usize::from(metatile) * 16;
                anyhow::ensure!(
                    base + 16 <= layout.len(),
                    "map {map_id} references missing {tileset} metatile ${metatile:02x}"
                );
                for subtile_row in 0..4_u8 {
                    for subtile_column in 0..4_u8 {
                        let column = block_x * 4 + usize::from(subtile_column);
                        let row = block_y * 4 + usize::from(subtile_row);
                        let tile_index = u16::from(
                            layout
                                [base + usize::from(subtile_row) * 4 + usize::from(subtile_column)],
                        );
                        tiles.push(VisualTile {
                            column: column as u32,
                            row: row as u32,
                            source: VisualTileSource {
                                tileset_id: Arc::from(tileset),
                                metatile_id: metatile,
                                subtile_column,
                                subtile_row,
                                tile_index,
                            },
                            texture: Handle::<Image>::default(),
                            priority: false,
                        });
                    }
                }
            }
        }
        tiles.sort_by_key(|tile| (tile.row, tile.column));
        let coverage = audit_cell_coverage_on_map(map_id, &tiles, width, height)
            .map_err(|error| anyhow::anyhow!("classify voxel coverage for {map_id}: {error:?}"))?;
        let mut map_totals = BTreeMap::<&'static str, u64>::new();
        for (tile, kind) in tiles.iter().zip(coverage) {
            let label = kind.label();
            *coverage_totals.entry(label).or_default() += 1;
            *map_totals.entry(label).or_default() += 1;
            let key = SourceKey {
                coverage: label,
                tileset: tileset.to_owned(),
                metatile: tile.source.metatile_id,
                subtile_column: tile.source.subtile_column,
                subtile_row: tile.source.subtile_row,
                tile_index: tile.source.tile_index,
            };
            let finding = findings.entry(key).or_insert_with(|| SourceFinding {
                count: 0,
                maps: BTreeSet::new(),
                sample_map: map_id.clone(),
                sample_x: tile.column / 2,
                sample_y: tile.row / 2,
            });
            finding.count += 1;
            finding.maps.insert(map_id.clone());
        }
        map_reports.push(json!({
            "map": map_id,
            "tileset": tileset,
            "source_width": width,
            "source_height": height,
            "coverage": map_totals,
        }));
    }

    let mut source_reports = findings.into_iter().collect::<Vec<_>>();
    source_reports.sort_by(|(left_key, left), (right_key, right)| {
        (left_key.coverage != "flat")
            .cmp(&(right_key.coverage != "flat"))
            .then_with(|| right.count.cmp(&left.count))
            .then_with(|| left_key.cmp(right_key))
    });
    let source_reports = source_reports
        .into_iter()
        .map(|(key, finding)| {
            json!({
                "coverage": key.coverage,
                "tileset": key.tileset,
                "metatile": format!("{:02x}", key.metatile),
                "subtile_column": key.subtile_column,
                "subtile_row": key.subtile_row,
                "tile_index": format!("{:02x}", key.tile_index),
                "count": finding.count,
                "maps": finding.maps,
                "sample": {
                    "map": finding.sample_map,
                    "runtime_x": finding.sample_x,
                    "runtime_y": finding.sample_y,
                },
            })
        })
        .collect::<Vec<_>>();
    let report = json!({
        "format": "crystal-voxel-coverage-v1",
        "map_count": map_ids.len(),
        "coverage": coverage_totals,
        "maps": map_reports,
        "sources": source_reports,
    });
    let rendered = serde_json::to_string_pretty(&report)?;
    if let Some(output_path) = output_path {
        fs::write(&output_path, format!("{rendered}\n"))
            .with_context(|| format!("write voxel audit {}", output_path.display()))?;
        println!("wrote {}", output_path.display());
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn parse_args(values: impl IntoIterator<Item = String>) -> Result<(PathBuf, Option<PathBuf>)> {
    let mut pack = None;
    let mut output = None;
    let mut values = values.into_iter();
    while let Some(flag) = values.next() {
        match flag.as_str() {
            "--pack" => pack = Some(PathBuf::from(next_value(&mut values, "--pack")?)),
            "--output" => output = Some(PathBuf::from(next_value(&mut values, "--output")?)),
            "-h" | "--help" => {
                println!(
                    "usage: audit_voxel_coverage --pack <game.crystalpack> [--output report.json]"
                );
                std::process::exit(0);
            }
            _ => bail!("unknown argument '{flag}'"),
        }
    }
    Ok((
        pack.context("--pack <game.crystalpack> is required")?,
        output,
    ))
}

fn next_value(values: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    values
        .next()
        .with_context(|| format!("{flag} requires a value"))
}
