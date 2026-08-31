use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail, ensure};
use crystal_mapgen::{
    Coordinate, H3BatchConnections, H3BatchManifest, H3CellPlan, MapAudit, MapSource,
    ModpackOptions, attach_h3_regional_plan, audit_grid, audit_grid_with_facilities,
    audit_h3_batch_topology, audit_h3_grid_seams, audit_h3_regional_batch, audit_h3_seam_contracts,
    build_h3_grid_seam_profile, build_h3_regional_connections, build_h3_seam_contract,
    build_modpack, fetch_h3_batch_neighborhoods, fetch_h3_neighborhood, fetch_neighborhood,
    finalize_h3_batch_grid_seams, finalize_h3_source_transport, generate_grid,
    inspect_h3_regional_grid, plan_h3_batch, plan_h3_cell, plan_h3_region, prepare_h3_source,
    render_h3_mosaic, render_tile_preview, repair_walkable_connectivity,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const H3_SOURCE_CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct H3SourceCacheMetadata {
    schema_version: u32,
    source_bytes: u64,
    source_sha256: String,
    audit_bytes: u64,
    audit_sha256: String,
}

#[derive(Debug)]
struct Args {
    lat: f64,
    lon: f64,
    miles: f64,
    grid: u16,
    output_dir: PathBuf,
    base_pack: PathBuf,
    source: Option<PathBuf>,
    h3_resolution: Option<u8>,
    h3_plan_cells: Option<usize>,
    h3_generate_cells: Option<usize>,
    h3_render_proof: bool,
    resume: bool,
}

fn main() -> Result<()> {
    let args = parse_args(env::args().skip(1))?;
    if args.h3_render_proof && args.h3_generate_cells.is_none() {
        bail!("--h3-render-proof requires --h3-generate-cells");
    }
    if args.resume && args.h3_generate_cells.is_none() {
        bail!("--resume requires --h3-generate-cells");
    }
    if args.resume && args.source.is_some() {
        bail!("--resume cannot be combined with --source");
    }
    fs::create_dir_all(&args.output_dir)
        .with_context(|| format!("create output directory {}", args.output_dir.display()))?;
    let coordinate = Coordinate {
        lat: args.lat,
        lon: args.lon,
    };
    if let Some(requested_cells) = args.h3_plan_cells {
        if args.h3_generate_cells.is_some() {
            bail!("--h3-plan-cells and --h3-generate-cells are mutually exclusive");
        }
        let resolution = args
            .h3_resolution
            .context("--h3-plan-cells requires --h3-res")?;
        let manifest = plan_h3_batch(coordinate, resolution, requested_cells)?;
        let manifest_path = args.output_dir.join("h3-manifest.json");
        fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
        let (topology_audit, links) = audit_h3_batch_topology(&manifest, args.grid, args.grid)?;
        fs::write(
            args.output_dir.join("h3-topology-audit.json"),
            serde_json::to_vec_pretty(&topology_audit)?,
        )?;
        fs::write(
            args.output_dir.join("h3-connections.json"),
            serde_json::to_vec_pretty(&H3BatchConnections {
                schema_version: manifest.schema_version,
                grid_width: args.grid,
                grid_height: args.grid,
                links,
            })?,
        )?;
        if !topology_audit.passed {
            bail!(
                "H3 topology audit failed: {}",
                topology_audit.errors.join("; ")
            );
        }
        println!(
            "planned {} connected H3 cells at resolution {} from {}",
            manifest.cells.len(),
            manifest.resolution,
            manifest.origin
        );
        println!("wrote topology-only manifest: {}", manifest_path.display());
        println!(
            "verified {} reciprocal internal portal links and {} batch-boundary edges",
            topology_audit.internal_edges, topology_audit.boundary_edges
        );
        println!("no OpenStreetMap fetches or map renders were performed");
        return Ok(());
    }
    if let Some(requested_cells) = args.h3_generate_cells {
        let resolution = args
            .h3_resolution
            .context("--h3-generate-cells requires --h3-res")?;
        if !(1..=61).contains(&requested_cells) {
            bail!("H3 generation proofs support 1-61 cells, got {requested_cells}");
        }
        invalidate_batch_final_artifacts(&args.output_dir)?;
        let manifest = plan_h3_batch(coordinate, resolution, requested_cells)?;
        fs::write(
            args.output_dir.join("h3-manifest.json"),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        let cells_dir = args.output_dir.join("cells");
        fs::create_dir_all(&cells_dir)?;
        let resumed_sources =
            load_batch_resume_sources_and_invalidate(&manifest, &cells_dir, args.resume)?;
        let (topology_audit, _) = audit_h3_batch_topology(&manifest, args.grid, args.grid)?;
        fs::write(
            args.output_dir.join("h3-topology-audit.json"),
            serde_json::to_vec_pretty(&topology_audit)?,
        )?;
        if !topology_audit.passed {
            bail!(
                "H3 topology audit failed: {}",
                topology_audit.errors.join("; ")
            );
        }
        let raw_source = args
            .source
            .as_ref()
            .map(|path| {
                let source = serde_json::from_slice::<MapSource>(&fs::read(path)?)
                    .with_context(|| format!("read normalized source {}", path.display()))?;
                validate_explicit_batch_source(&source, &manifest)?;
                Ok::<MapSource, anyhow::Error>(source)
            })
            .transpose()?;
        let mut fetched_sources = if raw_source.is_none() {
            fetch_missing_h3_batch_sources_with(
                &manifest,
                &resumed_sources,
                fetch_h3_batch_neighborhoods,
            )?
            .into_iter()
        } else {
            Vec::new().into_iter()
        };
        // Fetch/validate the complete batch first. Regional service scarcity
        // and the reciprocal road graph are properties of the whole manifest,
        // not independent per-cell defaults.
        let mut prepared_sources = Vec::with_capacity(requested_cells);
        let mut source_contracts = Vec::with_capacity(requested_cells);
        for (entry, resumed_source) in manifest.cells.iter().zip(resumed_sources) {
            let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
            let source = if let Some(cached_source) = resumed_source {
                println!(
                    "resuming H3 cell {} from its validated cached OpenStreetMap source",
                    entry.plan.cell
                );
                cached_source
            } else {
                if let Some(raw_source) = &raw_source {
                    prepare_h3_source(raw_source.clone(), entry.plan.clone())?
                } else {
                    fetched_sources.next().with_context(|| {
                        format!("shared batch fetch omitted H3 cell {}", entry.plan.cell)
                    })?
                }
            };
            write_h3_source_file(&cell_dir, &source)?;
            source_contracts.push(build_h3_seam_contract(
                &entry.plan,
                &source,
                args.grid,
                args.grid,
            )?);
            prepared_sources.push(source);
        }
        ensure!(
            fetched_sources.next().is_none(),
            "shared batch fetch returned more sources than missing H3 plans"
        );
        let regional_plan = plan_h3_region(&manifest, &prepared_sources, &source_contracts)?;
        let pending_regional_plan = args.output_dir.join("h3-regional-plan.pending.json");
        fs::write(
            &pending_regional_plan,
            serde_json::to_vec_pretty(&regional_plan)?,
        )?;
        let runtime_connections =
            build_h3_regional_connections(&manifest, &regional_plan, args.grid, args.grid)?;
        let pending_connections = args.output_dir.join("h3-connections.pending.json");
        fs::write(
            &pending_connections,
            serde_json::to_vec_pretty(&runtime_connections)?,
        )?;

        let mut contracts = Vec::with_capacity(requested_cells);
        let mut grid_seam_profiles = Vec::with_capacity(requested_cells);
        let mut regional_reports = Vec::with_capacity(requested_cells);
        let mut rendered_cells = Vec::new();
        let mut generated_contexts = Vec::with_capacity(requested_cells);
        let mut generated_grids = Vec::with_capacity(requested_cells);
        for (entry, mut source) in manifest.cells.iter().zip(prepared_sources) {
            let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
            let regional_cell = regional_plan
                .cell(&entry.plan.cell)
                .with_context(|| format!("regional plan omitted H3 cell {}", entry.plan.cell))?
                .clone();
            let mut expected_center = regional_cell
                .facilities
                .contains(&crystal_mapgen::H3Facility::PokemonCenter);
            let mut expected_mart = regional_cell
                .facilities
                .contains(&crystal_mapgen::H3Facility::Mart);
            attach_h3_regional_plan(&mut source, regional_cell, args.grid, args.grid)?;
            let source_plan = source
                .h3
                .as_ref()
                .context("regional source lost its H3 plan")?
                .clone();
            let contract = build_h3_seam_contract(&source_plan, &source, args.grid, args.grid)?;
            let mut grid = generate_grid(source, args.grid, args.grid)?;
            let has_center = [
                crystal_mapgen::MapCell::PokecenterNorthWest,
                crystal_mapgen::MapCell::PokecenterNorthEast,
                crystal_mapgen::MapCell::PokecenterSouthWest,
                crystal_mapgen::MapCell::PokecenterSouthEast,
            ]
            .into_iter()
            .all(|cell| grid.cells.contains(&cell));
            let has_mart = [
                crystal_mapgen::MapCell::MartNorthWest,
                crystal_mapgen::MapCell::MartNorthEast,
                crystal_mapgen::MapCell::MartSouthWest,
                crystal_mapgen::MapCell::MartSouthEast,
            ]
            .into_iter()
            .all(|cell| grid.cells.contains(&cell));
            expected_center &= has_center;
            expected_mart &= has_mart;
            if let Some(regional) = grid
                .source
                .h3
                .as_mut()
                .and_then(|plan| plan.regional.as_mut())
            {
                regional.facilities.retain(|facility| match facility {
                    crystal_mapgen::H3Facility::PokemonCenter => has_center,
                    crystal_mapgen::H3Facility::Mart => has_mart,
                });
            }
            generated_contexts.push((
                cell_dir,
                source_plan,
                contract,
                expected_center,
                expected_mart,
            ));
            generated_grids.push(grid);
        }
        let grid_seam_finalization = finalize_h3_batch_grid_seams(&mut generated_grids)?;
        for grid in &mut generated_grids {
            repair_walkable_connectivity(grid);
        }
        for ((cell_dir, source_plan, contract, expected_center, expected_mart), grid) in
            generated_contexts.into_iter().zip(generated_grids)
        {
            let audit = audit_grid_with_facilities(&grid, expected_center, expected_mart);
            let grid_seam_profile = build_h3_grid_seam_profile(&grid)?;
            let regional_report = inspect_h3_regional_grid(&grid)?;
            fs::create_dir_all(&cell_dir)?;
            fs::write(
                cell_dir.join("grid.json"),
                serde_json::to_vec_pretty(&grid)?,
            )?;
            fs::write(
                cell_dir.join("audit.json"),
                serde_json::to_vec_pretty(&audit)?,
            )?;
            fs::write(
                cell_dir.join("seams.json"),
                serde_json::to_vec_pretty(&contract)?,
            )?;
            fs::write(
                cell_dir.join("grid-seams.json"),
                serde_json::to_vec_pretty(&grid_seam_profile)?,
            )?;
            fs::write(
                cell_dir.join("regional-report.json"),
                serde_json::to_vec_pretty(&regional_report)?,
            )?;
            if !audit.passed && raw_source.is_none() {
                bail!(
                    "H3 cell {} failed audit: {}",
                    source_plan.cell,
                    audit.errors.join("; ")
                );
            }
            write_h3_source_cache_metadata(&cell_dir)?;
            if args.h3_render_proof {
                let temporary_pack = cell_dir.join("preview.crystalpack");
                let generated = build_modpack(
                    &grid,
                    ModpackOptions {
                        base_pack: &args.base_pack,
                        output_pack: &temporary_pack,
                        manifest_id: &format!("h3-proof-{}", source_plan.cell),
                        start_new_game_here: false,
                    },
                )?;
                let preview = cell_dir.join("preview.png");
                render_tile_preview(&temporary_pack, &generated.map_name, &preview)?;
                fs::remove_file(&temporary_pack).with_context(|| {
                    format!("remove temporary preview pack {}", temporary_pack.display())
                })?;
                rendered_cells.push((source_plan.clone(), preview));
            }
            contracts.push(contract);
            grid_seam_profiles.push(grid_seam_profile);
            regional_reports.push(regional_report);
        }
        let regional_audit = audit_h3_regional_batch(&regional_plan, &regional_reports);
        if !regional_audit.passed && raw_source.is_none() {
            bail!(
                "H3 regional audit failed: {}",
                regional_audit.errors.join("; ")
            );
        }
        let seam_audit = audit_h3_seam_contracts(&contracts);
        if !seam_audit.passed && raw_source.is_none() {
            bail!("H3 seam audit failed: {}", seam_audit.errors.join("; "));
        }
        let grid_seam_audit = audit_h3_grid_seams(&grid_seam_profiles);
        if !grid_seam_audit.passed && raw_source.is_none() {
            bail!(
                "H3 rendered-grid seam audit failed: {}",
                grid_seam_audit.errors.join("; ")
            );
        }
        let pending_mosaic = args.output_dir.join("h3-buckyball.pending.png");
        if args.h3_render_proof {
            render_h3_mosaic(&rendered_cells, args.grid, args.grid, &pending_mosaic)?;
        }
        // Promote root-level proof artifacts only after every cell, regional,
        // source seam, rendered seam, and optional mosaic operation succeeds.
        // A failed run therefore cannot leave an old green audit beside a new
        // partial batch.
        let pending_audits = [
            (
                args.output_dir.join("h3-regional-audit.pending.json"),
                args.output_dir.join("h3-regional-audit.json"),
                serde_json::to_vec_pretty(&regional_audit)?,
            ),
            (
                args.output_dir.join("h3-seam-audit.pending.json"),
                args.output_dir.join("h3-seam-audit.json"),
                serde_json::to_vec_pretty(&seam_audit)?,
            ),
            (
                args.output_dir.join("h3-grid-seam-audit.pending.json"),
                args.output_dir.join("h3-grid-seam-audit.json"),
                serde_json::to_vec_pretty(&grid_seam_audit)?,
            ),
        ];
        for (pending, _, bytes) in &pending_audits {
            fs::write(pending, bytes)?;
        }
        let promotions = [
            (
                pending_regional_plan,
                args.output_dir.join("h3-regional-plan.json"),
            ),
            (
                pending_connections,
                args.output_dir.join("h3-connections.json"),
            ),
            (pending_audits[0].0.clone(), pending_audits[0].1.clone()),
            (pending_audits[1].0.clone(), pending_audits[1].1.clone()),
            (pending_audits[2].0.clone(), pending_audits[2].1.clone()),
        ];
        for (pending, final_path) in promotions {
            fs::rename(pending, final_path)?;
        }
        if args.h3_render_proof {
            let mosaic = args.output_dir.join("h3-buckyball.png");
            fs::rename(&pending_mosaic, &mosaic)?;
            println!("rendered exact-tile H3 buckyball: {}", mosaic.display());
        }
        println!(
            "generated and audited {requested_cells} connected H3 grids at resolution {resolution}"
        );
        println!(
            "verified {} reciprocal internal edges",
            seam_audit.internal_edges
        );
        println!(
            "regional services: {} Pokemon Centers and {} Marts; {} sparse internal road links and {} boundary exits ({} authoritative, {} synthetic of {} allowed)",
            regional_audit.pokemon_centers,
            regional_audit.marts,
            regional_audit.internal_connections,
            regional_audit.boundary_exits,
            regional_audit.authoritative_connections,
            regional_audit.synthetic_connections,
            regional_audit.synthetic_connection_budget
        );
        println!(
            "weakest per-cell principal route component contains {:.1}% of rendered route tiles",
            regional_audit.minimum_principal_route_percent
        );
        println!(
            "matched {}/{} reciprocal final-grid surface samples and {}/{} exact transport classes",
            grid_seam_audit.matching_surface_samples,
            grid_seam_audit.reciprocal_surface_samples,
            grid_seam_audit.matching_transport_samples,
            grid_seam_audit.reciprocal_surface_samples
        );
        println!(
            "verified {}/{} authoritative water samples with no tree, relief, or fence traces; reconciled {} raster cells",
            grid_seam_audit.continuous_water_samples,
            grid_seam_audit.authoritative_water_samples,
            grid_seam_finalization.changed_cells
        );
        println!(
            "verified {}/{} selected transport joins and capped {}/{} omitted crossings on both raster faces",
            grid_seam_audit.connected_transport_edges,
            grid_seam_audit.selected_transport_edges,
            grid_seam_audit.capped_transport_edges,
            grid_seam_audit.closed_transport_edges
        );
        return Ok(());
    }
    let h3_plan = args
        .h3_resolution
        .map(|resolution| plan_h3_cell(coordinate, resolution))
        .transpose()?;
    let mut source = if let Some(path) = &args.source {
        let source = serde_json::from_slice::<MapSource>(&fs::read(path)?)
            .with_context(|| format!("read normalized source {}", path.display()))?;
        if let Some(plan) = &h3_plan {
            prepare_h3_source(source, plan.clone())?
        } else {
            source
        }
    } else if let Some(plan) = &h3_plan {
        println!(
            "fetching OpenStreetMap geometry for H3 {} at resolution {}…",
            plan.cell, plan.resolution
        );
        fetch_h3_neighborhood(plan)?
    } else {
        println!(
            "fetching OpenStreetMap geometry around {}, {}…",
            args.lat, args.lon
        );
        fetch_neighborhood(coordinate, args.miles)?
    };
    if let Some(plan) = &h3_plan {
        fs::write(
            args.output_dir.join("h3-cell.json"),
            serde_json::to_vec_pretty(plan)?,
        )?;
        fs::write(
            args.output_dir.join("h3-seams.json"),
            serde_json::to_vec_pretty(&build_h3_seam_contract(
                plan, &source, args.grid, args.grid,
            )?)?,
        )?;
        finalize_h3_source_transport(&mut source, args.grid, args.grid)?;
    }
    let source_path = args.output_dir.join("source.json");
    fs::write(&source_path, serde_json::to_vec_pretty(&source)?)?;
    println!("normalizing {} geographic features…", source.features.len());
    let grid = generate_grid(source, args.grid, args.grid)?;
    fs::write(
        args.output_dir.join("grid.json"),
        serde_json::to_vec_pretty(&grid)?,
    )?;
    let audit = audit_grid(&grid);
    fs::write(
        args.output_dir.join("audit.json"),
        serde_json::to_vec_pretty(&audit)?,
    )?;
    if !audit.passed {
        bail!("generated grid failed audit: {}", audit.errors.join("; "));
    }
    let output_pack = args.output_dir.join("neighborhood.crystalpack");
    let generated = build_modpack(
        &grid,
        ModpackOptions {
            base_pack: &args.base_pack,
            output_pack: &output_pack,
            manifest_id: "coordinate-mapgen",
            start_new_game_here: true,
        },
    )?;
    fs::write(
        args.output_dir.join("modpack.json"),
        serde_json::to_vec_pretty(&generated)?,
    )?;
    let preview_path = args.output_dir.join("preview.png");
    render_tile_preview(&output_pack, &generated.map_name, &preview_path)?;
    println!("generated preview: {}", preview_path.display());
    println!("generated playable modpack: {}", output_pack.display());
    println!(
        "map {} starts at runtime tile ({}, {})",
        generated.map_name, generated.runtime_tile_x, generated.runtime_tile_y
    );
    println!(
        "audit passed: {} houses, {} wild sites, {:.1}% connected walkable terrain",
        audit.houses, audit.wild_sites, audit.walkable_reach_percent
    );
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args> {
    let mut lat = None;
    let mut lon = None;
    let mut miles = 1.0;
    let mut grid = 64;
    let mut output_dir = PathBuf::from("output/neighborhood-map");
    let mut base_pack = PathBuf::from("../content-packs/core-modular.crystalpack");
    let mut source = None;
    let mut h3_resolution = None;
    let mut h3_plan_cells = None;
    let mut h3_generate_cells = None;
    let mut h3_render_proof = false;
    let mut resume = false;
    while let Some(argument) = args.next() {
        let value = |args: &mut dyn Iterator<Item = String>, flag: &str| {
            args.next()
                .with_context(|| format!("{flag} requires a value"))
        };
        match argument.as_str() {
            "--lat" => lat = Some(value(&mut args, "--lat")?.parse()?),
            "--lon" => lon = Some(value(&mut args, "--lon")?.parse()?),
            "--miles" => miles = value(&mut args, "--miles")?.parse()?,
            "--grid" => grid = value(&mut args, "--grid")?.parse()?,
            "--output-dir" => output_dir = PathBuf::from(value(&mut args, "--output-dir")?),
            "--base-pack" => base_pack = PathBuf::from(value(&mut args, "--base-pack")?),
            "--source" => source = Some(PathBuf::from(value(&mut args, "--source")?)),
            "--h3-res" => h3_resolution = Some(value(&mut args, "--h3-res")?.parse()?),
            "--h3-plan-cells" => {
                h3_plan_cells = Some(value(&mut args, "--h3-plan-cells")?.parse()?)
            }
            "--h3-generate-cells" => {
                h3_generate_cells = Some(value(&mut args, "--h3-generate-cells")?.parse()?)
            }
            "--h3-render-proof" => h3_render_proof = true,
            "--resume" => resume = true,
            "--help" | "-h" => {
                println!(
                    "crystal-mapgen --lat <degrees> --lon <degrees> [--miles 1] [--grid 64] [--output-dir path] [--base-pack path] [--source normalized.json]\n\
                     H3 cell: --lat <degrees> --lon <degrees> --h3-res <0-15> [--grid 64] [--source normalized.json]\n\
                     H3 proof: add --h3-generate-cells <1-37> to write audited grids and seam contracts\n\
                     H3 visual: add --h3-render-proof for an exact-tile buckyball PNG\n\
                     H3 resume: add --resume to reuse only validated passed-cell OSM sources\n\
                     H3 topology: add --h3-plan-cells <1-5000> to write a connected manifest without fetching or rendering"
                );
                std::process::exit(0);
            }
            unknown => bail!("unknown argument {unknown:?}; use --help"),
        }
    }
    Ok(Args {
        lat: lat.context("--lat is required")?,
        lon: lon.context("--lon is required")?,
        miles,
        grid,
        output_dir,
        base_pack,
        source,
        h3_resolution,
        h3_plan_cells,
        h3_generate_cells,
        h3_render_proof,
        resume,
    })
}

/// Recover only the unregionalized normalized OSM source. Generated grids are
/// never used as fetch caches: their source has already been pruned by a prior
/// regional plan and would silently lose crossings if the planner changed.
fn load_resumable_source(cell_dir: &Path, plan: &H3CellPlan) -> Result<Option<MapSource>> {
    let source_path = cell_dir.join("source.json");
    let metadata_path = cell_dir.join("source-cache.json");
    let audit_path = cell_dir.join("audit.json");
    if !source_path.exists() || !metadata_path.exists() || !audit_path.exists() {
        return Ok(None);
    }
    let audit_bytes = fs::read(&audit_path)
        .with_context(|| format!("read cached H3 audit {}", audit_path.display()))?;
    let audit = serde_json::from_slice::<MapAudit>(&audit_bytes)
        .with_context(|| format!("read cached H3 audit {}", audit_path.display()))?;
    if !audit.passed {
        return Ok(None);
    }
    let source_bytes = fs::read(&source_path)
        .with_context(|| format!("read cached H3 source {}", source_path.display()))?;
    let metadata = serde_json::from_slice::<H3SourceCacheMetadata>(&fs::read(&metadata_path)?)
        .with_context(|| format!("read cached H3 source metadata {}", metadata_path.display()))?;
    if metadata.schema_version != H3_SOURCE_CACHE_SCHEMA_VERSION
        || metadata.source_bytes != source_bytes.len() as u64
        || metadata.source_sha256 != h3_source_sha256(&source_bytes)
        || metadata.audit_bytes != audit_bytes.len() as u64
        || metadata.audit_sha256 != h3_source_sha256(&audit_bytes)
    {
        return Ok(None);
    }
    let mut source = serde_json::from_slice::<MapSource>(&source_bytes)
        .with_context(|| format!("read cached H3 source {}", source_path.display()))?;
    let cached_plan = source
        .h3
        .as_ref()
        .context("cached H3 source has no projection plan")?;
    if !cached_plan.source_provenance.is_prepared_raw() {
        return Ok(None);
    }
    ensure!(
        cached_plan.cell == plan.cell && cached_plan.resolution == plan.resolution,
        "cached source projection is cell {} at resolution {}, requested cell {} at resolution {}",
        cached_plan.cell,
        cached_plan.resolution,
        plan.cell,
        plan.resolution
    );
    ensure!(
        cached_plan.regional.is_none(),
        "cached H3 source {} is already regionally pruned; source.json must preserve the unregionalized OSM feature set",
        plan.cell
    );
    let planned_bounds = plan
        .fetch_bounds
        .first()
        .context("requested H3 plan has no fetch bounds")?;
    let cached_bounds = source.bounds;
    ensure!(
        (cached_bounds.south - planned_bounds.south).abs() <= 1e-12
            && (cached_bounds.west - planned_bounds.west).abs() <= 1e-12
            && (cached_bounds.north - planned_bounds.north).abs() <= 1e-12
            && (cached_bounds.east - planned_bounds.east).abs() <= 1e-12,
        "cached H3 source {} was not fetched from its planned H3 bounds",
        plan.cell
    );
    let source_provenance = cached_plan.source_provenance;
    source.center = plan.center;
    let mut refreshed_plan = plan.clone();
    refreshed_plan.source_provenance = source_provenance;
    source.h3 = Some(refreshed_plan);
    Ok(Some(source))
}

fn h3_source_sha256(source_bytes: &[u8]) -> String {
    Sha256::digest(source_bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_h3_source_file(cell_dir: &Path, source: &MapSource) -> Result<()> {
    let source_bytes = serde_json::to_vec_pretty(source)?;
    write_h3_source_bytes(cell_dir, &source_bytes)
}

fn write_h3_source_bytes(cell_dir: &Path, source_bytes: &[u8]) -> Result<()> {
    let source_path = cell_dir.join("source.json");
    fs::write(&source_path, source_bytes)
        .with_context(|| format!("write H3 source cache {}", source_path.display()))?;
    Ok(())
}

fn write_h3_source_cache_metadata(cell_dir: &Path) -> Result<()> {
    let source_path = cell_dir.join("source.json");
    let audit_path = cell_dir.join("audit.json");
    let metadata_path = cell_dir.join("source-cache.json");
    let source_bytes = fs::read(&source_path)
        .with_context(|| format!("read H3 source cache {}", source_path.display()))?;
    let audit_bytes = fs::read(&audit_path)
        .with_context(|| format!("read H3 source audit {}", audit_path.display()))?;
    let metadata = H3SourceCacheMetadata {
        schema_version: H3_SOURCE_CACHE_SCHEMA_VERSION,
        source_bytes: source_bytes.len() as u64,
        source_sha256: h3_source_sha256(&source_bytes),
        audit_bytes: audit_bytes.len() as u64,
        audit_sha256: h3_source_sha256(&audit_bytes),
    };
    fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata)?)
        .with_context(|| format!("write H3 source cache metadata {}", metadata_path.display()))?;
    Ok(())
}

fn validate_explicit_batch_source(source: &MapSource, manifest: &H3BatchManifest) -> Result<()> {
    if source.h3.is_none() {
        ensure!(
            source.center.lat >= source.bounds.south
                && source.center.lat <= source.bounds.north
                && source.center.lon >= source.bounds.west
                && source.center.lon <= source.bounds.east,
            "an explicit regional --source must contain its declared center"
        );
        ensure!(
            manifest.cells.iter().any(|entry| {
                entry.plan.center.lat >= source.bounds.south
                    && entry.plan.center.lat <= source.bounds.north
                    && entry.plan.center.lon >= source.bounds.west
                    && entry.plan.center.lon <= source.bounds.east
            }),
            "an explicit regional --source does not overlap the requested H3 batch"
        );
        return Ok(());
    }
    ensure!(
        manifest.cells.len() == 1,
        "multi-cell H3 generation does not accept --source: each cell requires its own fresh halo fetch or validated per-cell --resume cache"
    );
    let plan = source.h3.as_ref().context(
        "one-cell H3 --source requires explicit prepared_raw H3 provenance; an unversioned normalized source is not sufficient",
    )?;
    ensure!(
        plan.source_provenance.is_prepared_raw() && plan.regional.is_none(),
        "batch --source requires an H3 prepared_raw source schema {}; got {:?}",
        crystal_mapgen::H3_SOURCE_SCHEMA_VERSION,
        plan.source_provenance
    );
    let requested = &manifest.cells[0].plan;
    ensure!(
        requested.cell == plan.cell && requested.resolution == plan.resolution,
        "an H3 prepared_raw --source is halo-clipped to cell {} and may only generate that matching one-cell batch",
        plan.cell
    );
    for required in &requested.fetch_bounds {
        ensure!(
            source.bounds.south <= required.south
                && source.bounds.west <= required.west
                && source.bounds.north >= required.north
                && source.bounds.east >= required.east,
            "batch --source bounds do not cover the complete fetch halo for H3 cell {}",
            requested.cell
        );
    }
    Ok(())
}

fn fetch_missing_h3_batch_sources_with<F>(
    manifest: &H3BatchManifest,
    resumed_sources: &[Option<MapSource>],
    fetch: F,
) -> Result<Vec<MapSource>>
where
    F: FnOnce(&[H3CellPlan]) -> Result<Vec<MapSource>>,
{
    ensure!(
        resumed_sources.len() == manifest.cells.len(),
        "resume preflight returned {} entries for {} H3 cells",
        resumed_sources.len(),
        manifest.cells.len()
    );
    let missing_plans = manifest
        .cells
        .iter()
        .zip(resumed_sources)
        .filter(|(_, source)| source.is_none())
        .map(|(entry, _)| entry.plan.clone())
        .collect::<Vec<_>>();
    if missing_plans.is_empty() {
        return Ok(Vec::new());
    }
    let sources = fetch(&missing_plans)?;
    ensure!(
        sources.len() == missing_plans.len(),
        "shared batch fetch returned {} sources for {} missing H3 plans",
        sources.len(),
        missing_plans.len()
    );
    for (source, requested) in sources.iter().zip(&missing_plans) {
        let fetched_plan = source
            .h3
            .as_ref()
            .context("shared batch fetch returned a source without an H3 plan")?;
        ensure!(
            fetched_plan.cell == requested.cell
                && fetched_plan.resolution == requested.resolution
                && fetched_plan.source_provenance.is_prepared_raw()
                && fetched_plan.regional.is_none(),
            "shared batch fetch weakened prepared_raw provenance for H3 cell {}",
            requested.cell
        );
        let expected_bounds = requested
            .fetch_bounds
            .first()
            .context("missing H3 plan has no fetch bounds")?;
        ensure!(
            source.bounds == *expected_bounds,
            "shared batch fetch did not preserve exact planned bounds for H3 cell {}",
            requested.cell
        );
    }
    Ok(sources)
}

/// Capture every resume-authorized raw source before invalidating derived
/// per-cell state across the whole batch. A later fetch or generation failure
/// therefore cannot leave untouched cells carrying stale green audits from a
/// different regional graph.
fn load_batch_resume_sources_and_invalidate(
    manifest: &H3BatchManifest,
    cells_dir: &Path,
    resume: bool,
) -> Result<Vec<Option<MapSource>>> {
    let mut sources = Vec::with_capacity(manifest.cells.len());
    let mut first_error = None;
    for entry in &manifest.cells {
        let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
        fs::create_dir_all(&cell_dir)?;
        let source = if resume {
            match load_resumable_source(&cell_dir, &entry.plan) {
                Ok(source) => source,
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                    None
                }
            }
        } else {
            None
        };
        sources.push(source);
    }
    for entry in &manifest.cells {
        let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
        invalidate_cell_generation_artifacts(&cell_dir)?;
    }
    if let Some(error) = first_error {
        return Err(error).context(
            "validate all cached H3 sources after invalidating stale derived batch artifacts",
        );
    }
    Ok(sources)
}

fn invalidate_cell_generation_artifacts(cell_dir: &Path) -> Result<()> {
    for name in [
        "audit.json",
        "source-cache.json",
        "grid.json",
        "seams.json",
        "grid-seams.json",
        "regional-report.json",
        "preview.png",
        "preview.crystalpack",
    ] {
        remove_file_if_present(&cell_dir.join(name))?;
    }
    Ok(())
}

fn invalidate_batch_final_artifacts(output_dir: &Path) -> Result<()> {
    for name in [
        "h3-regional-plan.json",
        "h3-connections.json",
        "h3-regional-audit.json",
        "h3-seam-audit.json",
        "h3-grid-seam-audit.json",
        "h3-buckyball.png",
        "h3-regional-plan.pending.json",
        "h3-connections.pending.json",
        "h3-regional-audit.pending.json",
        "h3-seam-audit.pending.json",
        "h3-grid-seam-audit.pending.json",
        "h3-buckyball.pending.png",
    ] {
        remove_file_if_present(&output_dir.join(name))?;
    }
    Ok(())
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("remove stale artifact {}", path.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_coordinate_resolution_and_first_five_thousand_plan() {
        let args = parse_args(
            [
                "--lat",
                "44.9475196",
                "--lon",
                "-93.3253477",
                "--h3-res",
                "8",
                "--h3-plan-cells",
                "5000",
                "--output-dir",
                "/tmp/h3-plan",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("H3 arguments");
        assert_eq!(args.h3_resolution, Some(8));
        assert_eq!(args.h3_plan_cells, Some(5_000));
        assert!(!args.h3_render_proof);
        assert!(!args.resume);
        assert_eq!(args.output_dir, PathBuf::from("/tmp/h3-plan"));
    }

    #[test]
    fn parses_explicit_h3_resume() {
        let args = parse_args(
            [
                "--lat",
                "44.9475196",
                "--lon",
                "-93.3253477",
                "--h3-res",
                "6",
                "--h3-generate-cells",
                "19",
                "--h3-render-proof",
                "--resume",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .expect("resumable H3 arguments");
        assert_eq!(args.h3_generate_cells, Some(19));
        assert!(args.h3_render_proof);
        assert!(args.resume);
    }

    #[test]
    fn mixed_resume_batch_fetches_only_missing_cells_with_exact_raw_provenance() {
        let manifest = plan_h3_batch(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
            3,
        )
        .expect("three-cell manifest");
        let prepared = |plan: &H3CellPlan, attribution: &str| {
            prepare_h3_source(
                MapSource {
                    center: plan.center,
                    bounds: plan.fetch_bounds[0],
                    attribution: attribution.to_string(),
                    features: Vec::new(),
                    h3: None,
                },
                plan.clone(),
            )
            .expect("prepared raw fixture")
        };
        let resumed_sources = vec![
            Some(prepared(&manifest.cells[0].plan, "passed cache zero")),
            None,
            Some(prepared(&manifest.cells[2].plan, "passed cache two")),
        ];
        let mut requested_cells = Vec::new();
        let fetched =
            fetch_missing_h3_batch_sources_with(&manifest, &resumed_sources, |missing_plans| {
                requested_cells.extend(missing_plans.iter().map(|plan| plan.cell.clone()));
                Ok(missing_plans
                    .iter()
                    .map(|plan| prepared(plan, "fresh shared batch fetch"))
                    .collect())
            })
            .expect("fetch only missing H3 sources");

        assert_eq!(requested_cells, vec![manifest.cells[1].plan.cell.clone()]);
        assert_eq!(fetched.len(), 1);
        assert_eq!(
            fetched[0].h3.as_ref().map(|plan| plan.cell.as_str()),
            Some(manifest.cells[1].plan.cell.as_str())
        );
        assert!(
            fetched[0]
                .h3
                .as_ref()
                .is_some_and(|plan| plan.source_provenance.is_prepared_raw())
        );

        let all_resumed = manifest
            .cells
            .iter()
            .map(|entry| Some(prepared(&entry.plan, "passed cache")))
            .collect::<Vec<_>>();
        assert!(
            fetch_missing_h3_batch_sources_with(&manifest, &all_resumed, |_| {
                panic!("an all-resumed batch must not invoke the network fetcher")
            })
            .expect("all-resumed batch")
            .is_empty()
        );
    }

    #[test]
    fn resume_rejects_an_unstaged_legacy_h3_source_cache() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
        )
        .expect("H3 plan");
        let manifest = plan_h3_batch(plan.center, 6, 1).expect("one-cell manifest");
        let source = MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds[0],
            attribution: "legacy compressed fixture".to_string(),
            features: Vec::new(),
            h3: Some(plan.clone()),
        };
        assert!(validate_explicit_batch_source(&source, &manifest).is_err());
        let audit = MapAudit {
            passed: true,
            cell_counts: std::collections::BTreeMap::new(),
            houses: 0,
            wild_sites: 0,
            walkable_reach_percent: 100.0,
            errors: Vec::new(),
            notes: Vec::new(),
        };
        let temporary = tempfile::tempdir().expect("temporary legacy cache");
        let mut legacy_json = serde_json::to_value(&source).expect("legacy source value");
        legacy_json["h3"]
            .as_object_mut()
            .expect("serialized H3 plan")
            .remove("source_provenance");
        let legacy_bytes = serde_json::to_vec_pretty(&legacy_json).expect("legacy source JSON");
        write_h3_source_bytes(temporary.path(), &legacy_bytes).expect("write legacy source cache");
        fs::write(
            temporary.path().join("audit.json"),
            serde_json::to_vec_pretty(&audit).expect("passed audit JSON"),
        )
        .expect("write passed legacy audit");
        write_h3_source_cache_metadata(temporary.path())
            .expect("bind legacy source to passed audit");

        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("legacy cache rejection")
                .is_none(),
            "an unregionalized H3 plan alone does not prove that source.json retained raw transport alternatives"
        );
    }

    #[test]
    fn resume_reuses_only_the_unregionalized_source_artifact() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
        )
        .expect("H3 plan");
        let manifest = plan_h3_batch(plan.center, 6, 1).expect("one-cell manifest");
        let source = prepare_h3_source(
            MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "resume fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            plan.clone(),
        )
        .expect("prepare raw resumable source");
        validate_explicit_batch_source(&source, &manifest).expect("prepared raw explicit source");
        let mut non_h3_source = source.clone();
        non_h3_source.h3 = None;
        validate_explicit_batch_source(&non_h3_source, &manifest)
            .expect("explicit regional source");
        let multi_cell_manifest = plan_h3_batch(plan.center, 6, 3).expect("three-cell manifest");
        assert!(validate_explicit_batch_source(&source, &multi_cell_manifest).is_err());
        validate_explicit_batch_source(&non_h3_source, &multi_cell_manifest)
            .expect("explicit regional source for multiple cells");
        let other_cell_manifest = plan_h3_batch(
            Coordinate {
                lat: 44.7,
                lon: -93.0,
            },
            6,
            1,
        )
        .expect("different one-cell manifest");
        assert!(validate_explicit_batch_source(&source, &other_cell_manifest).is_err());
        let mut undersized = source.clone();
        undersized.bounds.north -= 0.001;
        assert!(validate_explicit_batch_source(&undersized, &manifest).is_err());
        let temporary = tempfile::tempdir().expect("temporary resume cache");
        write_h3_source_file(temporary.path(), &source).expect("write source cache");

        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("missing audit is not reusable")
                .is_none()
        );
        let audit = |passed| MapAudit {
            passed,
            cell_counts: std::collections::BTreeMap::new(),
            houses: 0,
            wild_sites: 0,
            walkable_reach_percent: if passed { 100.0 } else { 0.0 },
            errors: Vec::new(),
            notes: Vec::new(),
        };
        fs::write(
            temporary.path().join("audit.json"),
            serde_json::to_vec_pretty(&audit(true)).expect("passed audit JSON"),
        )
        .expect("write passed audit cache");
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("unbound audit is not reusable")
                .is_none(),
            "a passed audit without exact source-cache metadata must not authorize resume"
        );
        write_h3_source_cache_metadata(temporary.path()).expect("bind source to passed audit");

        let resumed = load_resumable_source(temporary.path(), &plan)
            .expect("validated source cache")
            .expect("source is resumable");
        assert_eq!(resumed.attribution, source.attribution);
        assert_eq!(resumed.features, source.features);
        assert_eq!(
            resumed.h3.as_ref().map(|plan| plan.cell.as_str()),
            Some(plan.cell.as_str())
        );
        assert!(
            resumed
                .h3
                .as_ref()
                .is_some_and(|plan| plan.regional.is_none())
        );
        assert!((resumed.bounds.west - source.bounds.west).abs() <= 1e-12);
        assert!((resumed.bounds.east - source.bounds.east).abs() <= 1e-12);

        let mut standalone_reduced = source.clone();
        finalize_h3_source_transport(&mut standalone_reduced, 48, 56)
            .expect("standalone reduction");
        let standalone_provenance = standalone_reduced
            .h3
            .as_ref()
            .expect("standalone H3 plan")
            .source_provenance;
        assert_eq!(standalone_provenance.grid_width, Some(48));
        assert_eq!(standalone_provenance.grid_height, Some(56));
        assert!(validate_explicit_batch_source(&standalone_reduced, &manifest).is_err());
        write_h3_source_file(temporary.path(), &standalone_reduced)
            .expect("write standalone reduced cache");
        write_h3_source_cache_metadata(temporary.path())
            .expect("bind standalone reduced source to audit");
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("standalone reduced cache rejection")
                .is_none()
        );

        let mut regional_reduced = source.clone();
        attach_h3_regional_plan(
            &mut regional_reduced,
            crystal_mapgen::H3RegionalCellPlan {
                ordinal: 0,
                cell: plan.cell.clone(),
                building_count: 0,
                facilities: Vec::new(),
                connections: Vec::new(),
                closed_transport_crossings: Vec::new(),
            },
            64,
            64,
        )
        .expect("regional reduction");
        assert!(validate_explicit_batch_source(&regional_reduced, &manifest).is_err());
        write_h3_source_file(temporary.path(), &regional_reduced)
            .expect("write regional reduced cache");
        write_h3_source_cache_metadata(temporary.path())
            .expect("bind regional reduced source to audit");
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("regional reduced cache rejection")
                .is_none()
        );

        write_h3_source_file(temporary.path(), &source).expect("restore raw source cache");
        write_h3_source_cache_metadata(temporary.path())
            .expect("bind restored raw source to audit");

        let mut unrelated = source.clone();
        unrelated.attribution = "source bytes replaced behind passed audit".to_string();
        fs::write(
            temporary.path().join("source.json"),
            serde_json::to_vec_pretty(&unrelated).expect("unrelated source JSON"),
        )
        .expect("replace source without its cache metadata");
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("source hash mismatch is not reusable")
                .is_none(),
            "a passed audit must be bound to the exact source bytes"
        );
        write_h3_source_file(temporary.path(), &source)
            .expect("restore hash-bound raw source cache");
        write_h3_source_cache_metadata(temporary.path())
            .expect("restore hash-bound source metadata");

        fs::write(
            temporary.path().join("audit.json"),
            serde_json::to_vec_pretty(&audit(false)).expect("failed audit JSON"),
        )
        .expect("write failed audit cache");
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("failed audit is not reusable")
                .is_none()
        );

        fs::write(
            temporary.path().join("audit.json"),
            serde_json::to_vec_pretty(&audit(true)).expect("replacement passed audit JSON"),
        )
        .expect("write replacement passed audit cache");
        fs::write(temporary.path().join("grid.json"), b"{}").expect("write old generated grid");
        invalidate_cell_generation_artifacts(temporary.path())
            .expect("invalidate generated artifacts before source replacement");
        let mut replacement = source.clone();
        replacement.attribution = "replacement source".to_string();
        fs::write(
            temporary.path().join("source.json"),
            serde_json::to_vec_pretty(&replacement).expect("replacement source JSON"),
        )
        .expect("replace source cache");
        assert!(!temporary.path().join("audit.json").exists());
        assert!(!temporary.path().join("grid.json").exists());
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("replacement source without its own audit is not reusable")
                .is_none()
        );

        fs::remove_file(temporary.path().join("source.json")).expect("remove source cache");
        fs::write(temporary.path().join("grid.json"), b"{}")
            .expect("write irrelevant generated cache");
        assert!(
            load_resumable_source(temporary.path(), &plan)
                .expect("generated grids are ignored")
                .is_none()
        );
    }

    #[test]
    fn batch_start_invalidates_final_and_pending_root_artifacts() {
        let temporary = tempfile::tempdir().expect("temporary batch output");
        let artifacts = [
            "h3-regional-plan.json",
            "h3-connections.json",
            "h3-regional-audit.json",
            "h3-seam-audit.json",
            "h3-grid-seam-audit.json",
            "h3-buckyball.png",
            "h3-regional-plan.pending.json",
            "h3-connections.pending.json",
            "h3-regional-audit.pending.json",
            "h3-seam-audit.pending.json",
            "h3-grid-seam-audit.pending.json",
            "h3-buckyball.pending.png",
        ];
        for name in artifacts {
            fs::write(temporary.path().join(name), b"stale").expect("write stale artifact");
        }

        invalidate_batch_final_artifacts(temporary.path()).expect("invalidate batch artifacts");

        for name in artifacts {
            assert!(
                !temporary.path().join(name).exists(),
                "stale root artifact survived: {name}"
            );
        }
    }

    #[test]
    fn resume_preflight_captures_every_raw_source_before_invalidating_all_cells() {
        let manifest = plan_h3_batch(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
            3,
        )
        .expect("three-cell manifest");
        let temporary = tempfile::tempdir().expect("temporary resumed batch");
        let cells_dir = temporary.path().join("cells");
        let audit = MapAudit {
            passed: true,
            cell_counts: std::collections::BTreeMap::new(),
            houses: 0,
            wild_sites: 0,
            walkable_reach_percent: 100.0,
            errors: Vec::new(),
            notes: Vec::new(),
        };
        let derived = [
            "audit.json",
            "source-cache.json",
            "grid.json",
            "seams.json",
            "grid-seams.json",
            "regional-report.json",
            "preview.png",
            "preview.crystalpack",
        ];
        for entry in &manifest.cells {
            let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
            fs::create_dir_all(&cell_dir).expect("create resumed cell directory");
            let source = prepare_h3_source(
                MapSource {
                    center: entry.plan.center,
                    bounds: entry.plan.fetch_bounds[0],
                    attribution: format!("raw source for {}", entry.plan.cell),
                    features: Vec::new(),
                    h3: None,
                },
                entry.plan.clone(),
            )
            .expect("prepare resumable source");
            write_h3_source_file(&cell_dir, &source).expect("write resumed source");
            fs::write(
                cell_dir.join("audit.json"),
                serde_json::to_vec_pretty(&audit).expect("audit JSON"),
            )
            .expect("write passed audit");
            write_h3_source_cache_metadata(&cell_dir).expect("bind resumed source to passed audit");
            for name in derived.into_iter().skip(2) {
                fs::write(cell_dir.join(name), b"stale").expect("write stale cell artifact");
            }
        }

        let resumed = load_batch_resume_sources_and_invalidate(&manifest, &cells_dir, true)
            .expect("capture valid raw sources");

        assert_eq!(resumed.len(), 3);
        assert!(resumed.iter().all(|source| {
            source.as_ref().is_some_and(|source| {
                source
                    .h3
                    .as_ref()
                    .is_some_and(|plan| plan.source_provenance.is_prepared_raw())
            })
        }));
        for entry in &manifest.cells {
            let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
            assert!(cell_dir.join("source.json").exists());
            for name in derived {
                assert!(
                    !cell_dir.join(name).exists(),
                    "stale derived artifact survived for {}: {name}",
                    entry.plan.cell
                );
            }
        }
    }

    #[test]
    fn failed_resume_validation_still_invalidates_every_cells_derived_artifacts() {
        let manifest = plan_h3_batch(
            Coordinate {
                lat: 44.9475196,
                lon: -93.3253477,
            },
            6,
            3,
        )
        .expect("three-cell manifest");
        let temporary = tempfile::tempdir().expect("temporary invalid resumed batch");
        let cells_dir = temporary.path().join("cells");
        let audit = MapAudit {
            passed: true,
            cell_counts: std::collections::BTreeMap::new(),
            houses: 0,
            wild_sites: 0,
            walkable_reach_percent: 100.0,
            errors: Vec::new(),
            notes: Vec::new(),
        };
        for (index, entry) in manifest.cells.iter().enumerate() {
            let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
            fs::create_dir_all(&cell_dir).expect("create invalid resumed cell directory");
            let source_plan = if index == 0 {
                manifest.cells[1].plan.clone()
            } else {
                entry.plan.clone()
            };
            let source = prepare_h3_source(
                MapSource {
                    center: source_plan.center,
                    bounds: source_plan.fetch_bounds[0],
                    attribution: "resume validation fixture".to_string(),
                    features: Vec::new(),
                    h3: None,
                },
                source_plan,
            )
            .expect("prepare resume validation source");
            write_h3_source_file(&cell_dir, &source).expect("write invalid resumed source");
            fs::write(
                cell_dir.join("audit.json"),
                serde_json::to_vec_pretty(&audit).expect("audit JSON"),
            )
            .expect("write passed audit");
            write_h3_source_cache_metadata(&cell_dir)
                .expect("bind invalid resumed source to passed audit");
            fs::write(cell_dir.join("grid.json"), b"stale").expect("write stale grid");
        }

        assert!(load_batch_resume_sources_and_invalidate(&manifest, &cells_dir, true).is_err());
        for entry in &manifest.cells {
            let cell_dir = cells_dir.join(format!("{:04}-{}", entry.ordinal, entry.plan.cell));
            assert!(cell_dir.join("source.json").exists());
            assert!(!cell_dir.join("audit.json").exists());
            assert!(!cell_dir.join("source-cache.json").exists());
            assert!(!cell_dir.join("grid.json").exists());
        }
    }
}
