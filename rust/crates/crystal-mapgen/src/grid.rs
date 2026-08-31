use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::stable_grid::StableGrid;
use crate::{
    Coordinate, Feature, FeatureKind, H3Facility, H3SeamContract, MapSource, RoadAxis, WorldCell,
    WorldGrid, build_h3_seam_contract, finalize_h3_regional_transport_seams,
    preserve_h3_authoritative_water_seams,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapCell {
    /// Rectangular storage outside an H3 face. It renders as dense canopy and
    /// remains a hard wall, but is not authored terrain and is excluded from
    /// rock/tree/style counts.
    H3Void,
    Grass,
    Lawn,
    Clearing,
    Park,
    Flowers,
    Tree,
    ParkTree,
    SmallTree,
    SmallTreeSouth,
    Boulder,
    IceFloor,
    IceBoulder,
    RockFloor,
    Bench,
    TrashCan,
    Fountain,
    GroundSign,
    FenceNorthWest,
    FenceNorth,
    FenceNorthEast,
    FenceWest,
    FenceEast,
    FenceSouthWest,
    FenceSouth,
    FenceSouthEast,
    LedgeWest,
    LedgeMiddle,
    LedgeEast,
    CliffNorthWest,
    CliffNorth,
    CliffNorthEast,
    CliffWest,
    CliffCenter,
    CliffEast,
    CliffSouthWest,
    CliffSouth,
    CliffSouthEast,
    CliffInnerSouthWest,
    CliffInnerSouthEast,
    CliffStairs,
    Water,
    WaterAccessEast,
    WaterAccessWest,
    WaterAccessSouth,
    Pitch,
    Building,
    PokecenterNorthWest,
    PokecenterNorthEast,
    PokecenterSouthWest,
    PokecenterSouthEast,
    MartNorthWest,
    MartNorthEast,
    MartSouthWest,
    MartSouthEast,
    Rail,
    Trail,
    Street,
    Road,
    MajorRoad,
}

impl MapCell {
    fn priority(self) -> u8 {
        match self {
            Self::Grass => 0,
            Self::Lawn | Self::Clearing => 1,
            Self::Park | Self::Flowers => 2,
            Self::Tree
            | Self::ParkTree
            | Self::SmallTree
            | Self::SmallTreeSouth
            | Self::Boulder
            | Self::IceBoulder
            | Self::Bench
            | Self::TrashCan
            | Self::Fountain
            | Self::GroundSign
            | Self::FenceNorthWest
            | Self::FenceNorth
            | Self::FenceNorthEast
            | Self::FenceWest
            | Self::FenceEast
            | Self::FenceSouthWest
            | Self::FenceSouth
            | Self::FenceSouthEast
            | Self::LedgeWest
            | Self::LedgeMiddle
            | Self::LedgeEast
            | Self::CliffNorthWest
            | Self::CliffNorth
            | Self::CliffNorthEast
            | Self::CliffWest
            | Self::CliffCenter
            | Self::CliffEast
            | Self::CliffSouthWest
            | Self::CliffSouth
            | Self::CliffSouthEast
            | Self::CliffInnerSouthWest
            | Self::CliffInnerSouthEast
            | Self::CliffStairs
            | Self::H3Void => 3,
            Self::IceFloor | Self::RockFloor => 2,
            Self::Water
            | Self::WaterAccessEast
            | Self::WaterAccessWest
            | Self::WaterAccessSouth => 4,
            Self::Pitch => 5,
            Self::Building
            | Self::PokecenterNorthWest
            | Self::PokecenterNorthEast
            | Self::PokecenterSouthWest
            | Self::PokecenterSouthEast
            | Self::MartNorthWest
            | Self::MartNorthEast
            | Self::MartSouthWest
            | Self::MartSouthEast => 6,
            Self::Rail => 7,
            Self::Trail => 8,
            Self::Street => 9,
            Self::Road => 10,
            Self::MajorRoad => 11,
        }
    }
}

impl From<FeatureKind> for MapCell {
    fn from(value: FeatureKind) -> Self {
        match value {
            FeatureKind::Water => Self::Water,
            FeatureKind::Park => Self::Park,
            FeatureKind::Pitch => Self::Pitch,
            FeatureKind::Building => Self::Building,
            FeatureKind::Rail => Self::Rail,
            FeatureKind::Trail => Self::Trail,
            FeatureKind::Street => Self::Street,
            FeatureKind::Road => Self::Road,
            FeatureKind::MajorRoad => Self::MajorRoad,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridLabel {
    pub text: String,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedGrid {
    pub source: MapSource,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<MapCell>,
    pub labels: Vec<GridLabel>,
}

impl GeneratedGrid {
    pub fn cell(&self, x: u16, y: u16) -> Option<MapCell> {
        (x < self.width && y < self.height)
            .then(|| self.cells[usize::from(y) * usize::from(self.width) + usize::from(x)])
    }

    pub fn home_cell(&self) -> (u16, u16) {
        (self.width / 2, self.height / 2)
    }

    pub fn crystal_blocks(&self) -> Vec<u16> {
        let mut blocks = Vec::with_capacity(self.cells.len());
        for y in 0..self.height {
            for x in 0..self.width {
                let cell = self.cell(x, y).unwrap_or(MapCell::Grass);
                let block = match cell {
                    MapCell::H3Void => h3_void_block(x, y),
                    MapCell::Grass | MapCell::Lawn => 0x02,
                    // Clearing is safe natural ground, not a road. The pale
                    // dirt block made isolated clearings look like torn path
                    // fragments in regional previews.
                    MapCell::Clearing => 0x02,
                    MapCell::Park => park_block(self, x, y),
                    MapCell::Flowers => flower_block(x, y),
                    MapCell::Tree => tree_block(self, x, y),
                    MapCell::ParkTree => u16::from(crate::GENERATED_PARK_TREE_METATILE),
                    MapCell::SmallTree => 0x2f,
                    MapCell::SmallTreeSouth => 0x3b,
                    MapCell::Boulder => 0x0a,
                    MapCell::IceFloor => u16::from(crate::GENERATED_ICE_FLOOR_METATILE),
                    MapCell::IceBoulder => u16::from(crate::GENERATED_ICE_BOULDER_METATILE),
                    // The canonical raised-earth center is a seamless brown
                    // cave/upland floor when repeated without contour edges.
                    MapCell::RockFloor => 0x71,
                    MapCell::Bench => 0x80,
                    MapCell::TrashCan => 0x81,
                    MapCell::Fountain => 0x82,
                    MapCell::GroundSign => 0x45,
                    MapCell::FenceNorthWest => {
                        u16::from(crate::GENERATED_PARK_FENCE_NORTH_WEST_METATILE)
                    }
                    MapCell::FenceNorth => u16::from(crate::GENERATED_PARK_FENCE_NORTH_METATILE),
                    MapCell::FenceNorthEast => {
                        u16::from(crate::GENERATED_PARK_FENCE_NORTH_EAST_METATILE)
                    }
                    MapCell::FenceWest => u16::from(crate::GENERATED_PARK_FENCE_WEST_METATILE),
                    MapCell::FenceEast => u16::from(crate::GENERATED_PARK_FENCE_EAST_METATILE),
                    MapCell::FenceSouthWest => {
                        u16::from(crate::GENERATED_PARK_FENCE_SOUTH_WEST_METATILE)
                    }
                    MapCell::FenceSouth => u16::from(crate::GENERATED_PARK_FENCE_SOUTH_METATILE),
                    MapCell::FenceSouthEast => {
                        u16::from(crate::GENERATED_PARK_FENCE_SOUTH_EAST_METATILE)
                    }
                    MapCell::LedgeWest => 0x52,
                    MapCell::LedgeMiddle => 0x57,
                    MapCell::LedgeEast => 0x53,
                    MapCell::CliffNorthWest => 0x6a,
                    MapCell::CliffNorth => 0x70,
                    MapCell::CliffNorthEast => 0x6b,
                    MapCell::CliffWest => 0x68,
                    MapCell::CliffCenter => 0x71,
                    MapCell::CliffEast => 0x69,
                    MapCell::CliffSouthWest => 0x6c,
                    MapCell::CliffSouth => 0x72,
                    MapCell::CliffSouthEast => 0x6d,
                    MapCell::CliffInnerSouthWest => 0x6e,
                    MapCell::CliffInnerSouthEast => 0x6f,
                    MapCell::CliffStairs => u16::from(crate::GENERATED_CLIFF_STAIRS_METATILE),
                    MapCell::Water => water_block(self, x, y),
                    MapCell::WaterAccessEast => 0x58,
                    MapCell::WaterAccessWest => 0x59,
                    MapCell::WaterAccessSouth => 0x76,
                    MapCell::Pitch => 0x02,
                    MapCell::Building => building_block(self, x, y),
                    MapCell::PokecenterNorthWest => 0x18,
                    MapCell::PokecenterNorthEast => 0x19,
                    MapCell::PokecenterSouthWest => 0x1a,
                    MapCell::PokecenterSouthEast => 0x1b,
                    MapCell::MartNorthWest => 0x18,
                    MapCell::MartNorthEast => 0x19,
                    MapCell::MartSouthWest => 0x1a,
                    MapCell::MartSouthEast => 0x33,
                    MapCell::Rail => 0x04,
                    MapCell::Trail => 0x07,
                    MapCell::Street | MapCell::Road | MapCell::MajorRoad => 0x07,
                };
                blocks.push(block);
            }
        }
        blocks
    }
}

pub fn generate_grid(source: MapSource, width: u16, height: u16) -> Result<GeneratedGrid> {
    if !(24..=128).contains(&width) || !(24..=128).contains(&height) {
        bail!("grid dimensions must be between 24 and 128 blocks");
    }
    let h3_seams = if let Some(plan) = &source.h3 {
        plan.raster_polygon(width, height)?;
        Some(build_h3_seam_contract(plan, &source, width, height)?)
    } else {
        None
    };
    let mut grid = GeneratedGrid {
        source,
        width,
        height,
        cells: vec![MapCell::Grass; usize::from(width) * usize::from(height)],
        labels: Vec::new(),
    };
    for kind in [FeatureKind::Park, FeatureKind::Water, FeatureKind::Pitch] {
        let features = selected_features(&grid, kind);
        for feature in &features {
            paint_feature(&mut grid, feature);
        }
    }
    fill_water_pinholes(&mut grid);
    // River and stream centerlines are intentionally one metatile wide. Only
    // discard single-pixel raster noise; connected source water, however
    // narrow, must remain visible.
    // Preserve connected rivers while suppressing tiny pond/polygon fragments
    // that read as scattered blue noise at Game Boy scale. The cardinal river
    // rasterizer below produces substantive connected components.
    remove_tiny_areas(&mut grid, MapCell::Water, 10);
    remove_tiny_areas(&mut grid, MapCell::Park, 4);
    remove_tiny_areas(&mut grid, MapCell::Pitch, 3);
    seed_wild_sites(&mut grid);
    shape_wild_sites(&mut grid)?;
    plan_path_backbone(&mut grid)?;
    author_public_field(&mut grid);
    if grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.requests_facility(H3Facility::PokemonCenter))
    {
        place_pokecenter(&mut grid);
    }
    if grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.requests_facility(H3Facility::Mart))
    {
        place_mart(&mut grid);
    }
    place_city_landmarks(&mut grid);
    place_houses(&mut grid);
    place_public_amenities(&mut grid);
    place_flowers(&mut grid);
    ensure_h3_wild_sites(&mut grid);
    plant_tree_belts(&mut grid);
    crate::vegetation::naturalize_groves(&mut grid)?;
    // Contours are authored against the finished canopy so their forest-
    // overlap score can genuinely replace tree mass instead of selecting an
    // empty lawn before trees exist.
    place_landmark_features(&mut grid)?;
    ensure_h3_wild_sites(&mut grid);
    place_route_furniture(&mut grid);
    ensure_public_details(&mut grid);
    place_rock_outcrops(&mut grid);
    enrich_route_viewports(&mut grid);
    create_water_access(&mut grid);
    prepare_home_spawn(&mut grid);
    repair_walkable_connectivity(&mut grid);
    place_large_ledge_runs(&mut grid)?;
    if grid.source.h3.is_some() {
        place_rock_outcrops(&mut grid);
        ensure_public_details(&mut grid);
    }
    // Paint ecological character only after all complete relief stamps are in
    // place. Biomes may shape natural ground and canopy, but can never move or
    // truncate a cliff/ledge merely by changing an earlier site ranking.
    crate::biomes::author_biomes(&mut grid)?;
    crate::biomes::author_cave_landmarks(&mut grid)?;
    // Dense real-world city inputs often leave too little authored canopy for
    // any biome to read as a forest. Refill complete irregular groves on safe
    // natural ground for square maps as well as H3 faces.
    ensure_h3_forest_density(&mut grid)?;
    place_irregular_wild_infill(&mut grid)?;
    ensure_h3_compact_wild_accents(&mut grid);
    ensure_public_details(&mut grid);
    break_long_canopy_bars(&mut grid);
    ensure_h3_rock_formations(&mut grid);
    crate::biomes::author_rare_structure(&mut grid)?;
    // Late authored layers can expose a single natural-ground cell inside a
    // water polygon after the initial coastline cleanup. Close those tiny
    // holes before auditing so the regional map cannot contain an unreachable
    // one-block island.
    fill_water_pinholes(&mut grid);
    repair_walkable_connectivity(&mut grid);
    if let Some(seams) = &h3_seams {
        author_h3_boundary(&mut grid, seams)?;
        repair_h3_house_stamps(&mut grid);
        create_water_access(&mut grid);
        repair_walkable_connectivity(&mut grid);
        ensure_h3_wild_sites(&mut grid);
        ensure_h3_forest_density(&mut grid)?;
        break_long_canopy_bars(&mut grid);
        repair_walkable_connectivity(&mut grid);
        ensure_h3_compact_wild_accents(&mut grid);
        promote_h3_wild_rooms(&mut grid);
        cap_h3_substantive_wild_sites(&mut grid);
        // Boundary terrain owns the seam band and can legitimately replace
        // earlier decorative rocks. Refill only safe interior footprints so
        // every H3 face retains several distinct formations without touching
        // portals or inventing a perimeter feature.
        prune_h3_isolated_boulders(&mut grid);
        ensure_h3_rock_formations(&mut grid);
        break_long_canopy_bars(&mut grid);
        finalize_h3_regional_transport_seams(&mut grid)?;
        preserve_h3_authoritative_water_seams(&mut grid)?;
    }
    // Reassert the sparse regional backbone after every structural layer.
    // Trees, cliffs, rocks, and boundary masking may have consumed an early
    // connector; this pass reconnects only the declared portal graph before
    // roadside furniture is allowed to frame it.
    if grid
        .source
        .h3
        .as_ref()
        .is_some_and(|plan| plan.regional.is_some())
    {
        connect_h3_regional_backbone(&mut grid)?;
        // The connector may approach an edge while joining an exact landing.
        // Reapply the full-face exit invariant so no shifted Trail reaches
        // H3Void, then preserve every declared three-cell landing verbatim.
        finalize_h3_regional_transport_seams(&mut grid)?;
        preserve_h3_authoritative_water_seams(&mut grid)?;
        repair_walkable_connectivity(&mut grid);
        // Regional connector/cap passes consume a small amount of canopy.
        // Refill only safe interior grove cells now, while retaining enough
        // margin for the final roadside pass, then reassert seam authority.
        ensure_h3_forest_density(&mut grid)?;
        break_long_canopy_bars(&mut grid);
        repair_walkable_connectivity(&mut grid);
        // Full grove stamps can be exhausted in a dense city face even when a
        // handful of safe interior tree sites remain. Finish the audit margin
        // with compact irregular clusters before transport/water authority is
        // reasserted; the fallback proves every added blocker preserves access.
        top_up_h3_interior_canopy(&mut grid)?;
        finalize_h3_regional_transport_seams(&mut grid)?;
        preserve_h3_authoritative_water_seams(&mut grid)?;
    }
    crate::biomes::top_up_tall_grass(&mut grid)?;
    ensure_h3_compact_wild_accents(&mut grid);
    // Fence and street-furniture planning consumes the final road, building,
    // forest, cliff, and H3 seam state. Only a final connectivity check follows;
    // no later structural pass may overwrite or orphan a course.
    crate::roadside::author_roadside_bays(&mut grid)?;
    repair_walkable_connectivity(&mut grid);
    grid.labels = select_labels(&grid);
    Ok(grid)
}

fn plan_path_backbone(grid: &mut GeneratedGrid) -> Result<()> {
    if grid.source.h3.is_some() {
        return plan_h3_path_backbone(grid);
    }
    let world_grid = WorldGrid::from_bounds(
        grid.source.center,
        grid.source.bounds,
        grid.width,
        grid.height,
    )?;
    let corridors = global_road_corridors(grid, world_grid)?;

    // The mapped layer is authored in global metatile addresses. Converting to
    // local coordinates is only the final crop, so moving the requested center
    // cannot slide a road onto a different lane. Water always wins: a mapped
    // corridor resumes on the far bank and a local Trail supplies the detour.
    paint_global_road_corridors(grid, world_grid, &corridors);
    bridge_mapped_roads_around_water(grid, world_grid, &corridors);

    connect_home_and_wild_sites(grid);
    Ok(())
}

fn plan_h3_path_backbone(grid: &mut GeneratedGrid) -> Result<()> {
    // H3 cells preserve real geometry in their own tangent frame. They never
    // synthesize a road along the six-sided boundary: only source linear
    // features are rasterized, and shared-edge contracts independently prove
    // which neighboring cells receive the continuation.
    paint_h3_source_transport(grid)?;
    connect_h3_regional_backbone(grid)?;
    connect_home_and_wild_sites(grid);
    Ok(())
}

fn paint_h3_source_transport(grid: &mut GeneratedGrid) -> Result<()> {
    let water = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            matches!(
                cell,
                MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth
            )
            .then_some(index)
        })
        .collect::<Vec<_>>();
    let features = grid
        .source
        .features
        .iter()
        .filter(|feature| {
            !feature.area
                && matches!(
                    feature.kind,
                    FeatureKind::Trail
                        | FeatureKind::Street
                        | FeatureKind::Road
                        | FeatureKind::MajorRoad
                )
        })
        .cloned()
        .collect::<Vec<_>>();
    let selected_bridges = selected_h3_bridge_features(grid, &features)?;
    for feature in &features {
        paint_feature(grid, feature);
    }
    // Water wins over every untagged or unselected route. Regional attachment
    // has already validated and retained the exact source feature behind each
    // authoritative crossing, so only that explicit `bridge=yes` trace may be
    // repainted across the restored water mask.
    for index in water {
        grid.cells[index] = MapCell::Water;
    }
    for feature in &selected_bridges {
        paint_feature(grid, feature);
    }
    Ok(())
}

fn selected_h3_bridge_features(grid: &GeneratedGrid, features: &[Feature]) -> Result<Vec<Feature>> {
    let Some(plan) = grid.source.h3.as_ref() else {
        return Ok(Vec::new());
    };
    let seams = build_h3_seam_contract(plan, &grid.source, grid.width, grid.height)?;
    let Some(regional) = plan.regional.as_ref() else {
        // Standalone H3 transport has already been reduced to its chosen edge
        // features. Preserve explicit bridges there as well; no discarded or
        // merely nearby feature remains in the prepared source.
        return Ok(features
            .iter()
            .filter(|feature| feature.bridge)
            .cloned()
            .collect());
    };
    let mut selected = Vec::new();
    for feature in features.iter().filter(|feature| feature.bridge) {
        let contract_crossing = seams.edges.iter().any(|edge| {
            edge.viable_crossings.iter().any(|crossing| {
                crossing.bridge
                    && crossing.transport == feature.kind
                    && feature_contains_coordinate(feature, crossing.coordinate)
            })
        });
        // Treat anything not wholly inside the raster face as a boundary
        // feature even when an endpoint-touching segment is numerically missed
        // by geographic intersection. This is deliberately conservative: a
        // real internal bridge survives, while every possible edge exit still
        // needs an exact selected regional connection.
        let crosses_face = contract_crossing
            || !feature_is_wholly_inside_h3_face(plan, feature, grid.width, grid.height)?;
        let selected_crossing = regional.connections.iter().any(|connection| {
            connection.authoritative
                && connection.bridge
                && connection.transport == feature.kind
                && feature_contains_coordinate(feature, connection.coordinate)
        });
        if !crosses_face || selected_crossing {
            selected.push(feature.clone());
        }
    }
    Ok(selected)
}

fn feature_is_wholly_inside_h3_face(
    plan: &crate::H3CellPlan,
    feature: &Feature,
    width: u16,
    height: u16,
) -> Result<bool> {
    for &point in &feature.points {
        let (x, y) = plan.project_to_grid(point, width, height)?;
        if x < 0
            || y < 0
            || x >= i32::from(width)
            || y >= i32::from(height)
            || !plan.raster_contains_cell(x as u16, y as u16, width, height)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn feature_contains_coordinate(feature: &Feature, coordinate: Coordinate) -> bool {
    const MATCH_TOLERANCE_DEGREES: f64 = 1e-7;
    let longitude_scale = coordinate.lat.to_radians().cos().abs().max(1e-6);
    let local = |point: Coordinate| {
        let longitude = (point.lon - coordinate.lon + 180.0).rem_euclid(360.0) - 180.0;
        (longitude * longitude_scale, point.lat - coordinate.lat)
    };
    feature.points.windows(2).any(|segment| {
        let first = local(segment[0]);
        let second = local(segment[1]);
        let delta = (second.0 - first.0, second.1 - first.1);
        let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
        if length_squared <= f64::EPSILON {
            return first.0.hypot(first.1) <= MATCH_TOLERANCE_DEGREES;
        }
        let fraction = (-(first.0 * delta.0 + first.1 * delta.1) / length_squared).clamp(0.0, 1.0);
        let closest = (first.0 + delta.0 * fraction, first.1 + delta.1 * fraction);
        closest.0.hypot(closest.1) <= MATCH_TOLERANCE_DEGREES
    })
}

fn connect_h3_regional_backbone(grid: &mut GeneratedGrid) -> Result<()> {
    let Some((plan, regional)) = grid.source.h3.as_ref().and_then(|plan| {
        plan.regional
            .as_ref()
            .map(|regional| (plan.clone(), regional.clone()))
    }) else {
        return Ok(());
    };
    let width = usize::from(grid.width);
    let mut closed_landings = std::collections::BTreeSet::<usize>::new();
    for crossing in &regional.closed_transport_crossings {
        for (x, y) in crate::h3::h3_raster_sample_band(&plan, grid, crossing.coordinate)? {
            let index = usize::from(y) * width + usize::from(x);
            closed_landings.insert(index);
            if matches!(
                grid.cells[index],
                MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
            ) {
                grid.cells[index] = MapCell::Grass;
            }
        }
    }
    let mut landings = std::collections::BTreeSet::<usize>::new();
    for connection in &regional.connections {
        let route = MapCell::from(connection.transport);
        // Use the same exact cardinal three-cell band as final seam
        // hardening and runtime transitions. Independent projection logic
        // previously produced diagonal or shifted route fragments.
        for (x, y) in crate::h3::h3_raster_sample_band(&plan, grid, connection.coordinate)? {
            let index = usize::from(y) * width + usize::from(x);
            grid.cells[index] = route;
            landings.insert(index);
        }
    }
    let connector_forbidden = regional_connector_forbidden_cells(grid, &landings, &closed_landings);

    // OSM streets are often separate polylines after downsampling. Join only
    // components that own declared regional landings to the principal source
    // route. Connecting every incidental OSM fragment created hundreds of
    // synthetic Trail cells and made dense city rooms fail their path budget.
    for _ in 0..regional.connections.len().saturating_add(1) {
        let components = transport_components(grid);
        if components.len() <= 1 {
            break;
        }
        let principal_index = components
            .iter()
            .enumerate()
            .max_by_key(|(_, component)| component.len())
            .map(|(index, _)| index)
            .expect("multiple route components have a principal component");
        let principal = &components[principal_index];
        let mut targets = components
            .iter()
            .enumerate()
            .filter(|(index, component)| {
                *index != principal_index && component.iter().any(|index| landings.contains(index))
            })
            .map(|(index, component)| (std::cmp::Reverse(component.len()), index))
            .collect::<Vec<_>>();
        targets.sort_unstable();
        let mut connected = false;
        if targets.is_empty() {
            break;
        }
        for (_, target_index) in targets {
            let target = &components[target_index];
            let mut pairs = target
                .iter()
                .flat_map(|&from| {
                    principal.iter().map(move |&to| {
                        let from_x = from % width;
                        let from_y = from / width;
                        let to_x = to % width;
                        let to_y = to / width;
                        (from_x.abs_diff(to_x) + from_y.abs_diff(to_y), from, to)
                    })
                })
                .collect::<Vec<_>>();
            pairs.sort_unstable();
            for (_, from, to) in pairs.into_iter().take(96) {
                if connector_forbidden.contains(&from) || connector_forbidden.contains(&to) {
                    continue;
                }
                let start = ((from % width) as i32, (from / width) as i32);
                let goal = ((to % width) as i32, (to / width) as i32);
                let Some(path) =
                    shortest_path_avoiding(grid, start, goal, true, &connector_forbidden).or_else(
                        || shortest_path_avoiding(grid, start, goal, false, &connector_forbidden),
                    )
                else {
                    continue;
                };
                if path
                    .iter()
                    .any(|&(x, y)| connector_forbidden.contains(&(y as usize * width + x as usize)))
                {
                    continue;
                }
                commit_regional_trail(grid, path);
                connected = true;
                break;
            }
            if connected {
                break;
            }
        }
        if !connected {
            bail!(
                "could not connect every selected regional landing to the principal route in H3 cell {}",
                plan.cell
            );
        }
    }

    // Disconnected route fragments neither lead to a selected neighboring
    // room nor belong to the principal local network. Remove them instead of
    // spending a large synthetic-path budget stitching every clipped OSM
    // fragment; all declared landings were connected above.
    let components = transport_components(grid);
    if let Some(principal) = components.iter().max_by_key(|component| component.len()) {
        let principal = principal
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        for component in components {
            if component.iter().any(|index| principal.contains(index)) {
                continue;
            }
            if component.iter().any(|index| landings.contains(index)) {
                bail!(
                    "selected regional landing remained outside the principal route in H3 cell {}",
                    plan.cell
                );
            }
            for index in component {
                grid.cells[index] = MapCell::Grass;
            }
        }
    }
    Ok(())
}

fn regional_connector_forbidden_cells(
    grid: &GeneratedGrid,
    landings: &std::collections::BTreeSet<usize>,
    closed_landings: &std::collections::BTreeSet<usize>,
) -> std::collections::BTreeSet<usize> {
    let mut forbidden = closed_landings.clone();
    let width = usize::from(grid.width);
    for y in 0..grid.height {
        for x in 0..grid.width {
            let index = usize::from(y) * width + usize::from(x);
            if !landings.contains(&index) && crate::h3::route_cell_touches_h3_void(grid, x, y) {
                forbidden.insert(index);
            }
        }
    }
    for landing in landings {
        forbidden.remove(landing);
    }
    forbidden
}

fn commit_regional_trail(grid: &mut GeneratedGrid, path: Vec<(i32, i32)>) {
    for (x, y) in path {
        if !matches!(
            grid.cell(x as u16, y as u16),
            Some(
                MapCell::H3Void
                    | MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth
                    | MapCell::Trail
                    | MapCell::Street
                    | MapCell::Road
                    | MapCell::MajorRoad
                    | MapCell::Building
                    | MapCell::PokecenterNorthWest
                    | MapCell::PokecenterNorthEast
                    | MapCell::PokecenterSouthWest
                    | MapCell::PokecenterSouthEast
                    | MapCell::MartNorthWest
                    | MapCell::MartNorthEast
                    | MapCell::MartSouthWest
                    | MapCell::MartSouthEast
                    | MapCell::CliffNorthWest
                    | MapCell::CliffNorth
                    | MapCell::CliffNorthEast
                    | MapCell::CliffWest
                    | MapCell::CliffCenter
                    | MapCell::CliffEast
                    | MapCell::CliffSouthWest
                    | MapCell::CliffSouth
                    | MapCell::CliffSouthEast
                    | MapCell::CliffInnerSouthWest
                    | MapCell::CliffInnerSouthEast
            )
        ) {
            set_cell(grid, x, y, MapCell::Trail);
        }
    }
}

fn transport_components(grid: &GeneratedGrid) -> Vec<Vec<usize>> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut unseen = grid
        .cells
        .iter()
        .map(|cell| {
            matches!(
                cell,
                MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
            )
        })
        .collect::<Vec<_>>();
    let mut components = Vec::new();
    for start in 0..unseen.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut frontier = std::collections::VecDeque::from([start]);
        while let Some(index) = frontier.pop_front() {
            component.push(index);
            let x = index % width;
            let y = index / width;
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= width || next_y >= height {
                    continue;
                }
                let next = next_y * width + next_x;
                if unseen[next] {
                    unseen[next] = false;
                    frontier.push_back(next);
                }
            }
        }
        components.push(component);
    }
    components
}

fn connect_home_and_wild_sites(grid: &mut GeneratedGrid) {
    let (home_x, home_y) = grid.home_cell();
    let center_x = i32::from(home_x);
    let center_y = i32::from(home_y);
    // The address and authored destinations connect to the geographic road
    // layer with Trail cells. This keeps synthetic gameplay connectors
    // semantically distinct from OSM Street/Road/MajorRoad corridors.
    let mut routes = transport_cells(grid);
    routes.sort_unstable_by_key(|&(x, y)| (x - center_x).abs() + (y - center_y).abs());
    if let Some(&nearest) = routes.first() {
        carve_path(grid, (center_x, center_y), nearest);
    } else {
        carve_path(grid, (1, center_y), (i32::from(grid.width) - 2, center_y));
    }

    let mut wild_components = terrain_components(grid, MapCell::Park);
    wild_components.sort_by_key(|component| std::cmp::Reverse(component.len()));
    for component in wild_components.into_iter().take(4) {
        let width = usize::from(grid.width);
        let members = component
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let route_cells = transport_cells(grid);
        let mut best_path = None::<Vec<(i32, i32)>>;
        for index in component {
            let x = index % width;
            let y = index / width;
            for (outside_x, outside_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if outside_x >= usize::from(grid.width)
                    || outside_y >= usize::from(grid.height)
                    || members.contains(&(outside_y * width + outside_x))
                {
                    continue;
                }
                let landing = (outside_x as i32, outside_y as i32);
                let mut nearest_routes = route_cells.clone();
                nearest_routes.sort_unstable_by_key(|&(route_x, route_y)| {
                    (route_x - landing.0).abs() + (route_y - landing.1).abs()
                });
                for route in nearest_routes.into_iter().take(12) {
                    let Some(path) = shortest_route_path(grid, landing, route) else {
                        continue;
                    };
                    if best_path
                        .as_ref()
                        .is_none_or(|best| path.len() < best.len())
                    {
                        best_path = Some(path);
                    }
                }
            }
        }
        if let Some(path) = best_path {
            commit_trail(grid, path);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldGate {
    North,
    South,
    West,
    East,
}

#[derive(Debug)]
struct PublicFieldProposal {
    origin_x: u16,
    origin_y: u16,
    gate: FieldGate,
    gate_cell: (i32, i32),
    connector: Vec<(i32, i32)>,
    pitch_overlap: usize,
    source_distance: i32,
}

/// Compresses real-world sports/playground polygons into one authored Crystal
/// destination. Tiny downsampled pitch fragments are not useful on their own;
/// a complete fenced field, named sign, open gate, and short route connection
/// preserve their geographic intent at gameplay scale.
fn author_public_field(grid: &mut GeneratedGrid) {
    const FIELD_WIDTH: u16 = 9;
    const FIELD_HEIGHT: u16 = 7;
    // A globally snapped east/west corridor can move by up to half of its
    // twenty-cell band. Leave enough room for that displacement plus the
    // field's gate approach while still rejecting long synthetic routes.
    const MAX_CONNECTOR_STEPS: usize = 16;

    let pitch_components = terrain_components(grid, MapCell::Pitch);
    if pitch_components.is_empty() {
        return;
    }
    let width = usize::from(grid.width);
    let original_pitch = grid
        .cells
        .iter()
        .map(|cell| *cell == MapCell::Pitch)
        .collect::<Vec<_>>();
    let mut proposals = Vec::new();

    for component in &pitch_components {
        let center_x =
            component.iter().map(|index| index % width).sum::<usize>() / component.len().max(1);
        let center_y =
            component.iter().map(|index| index / width).sum::<usize>() / component.len().max(1);
        let ideal_x = center_x as i32 - i32::from(FIELD_WIDTH / 2);
        let ideal_y = center_y as i32 - i32::from(FIELD_HEIGHT / 2);

        for offset_y in -4..=4 {
            for offset_x in -4..=4 {
                let origin_x = ideal_x + offset_x;
                let origin_y = ideal_y + offset_y;
                if origin_x < 2
                    || origin_y < 2
                    || origin_x + i32::from(FIELD_WIDTH) + 1 >= i32::from(grid.width)
                    || origin_y + i32::from(FIELD_HEIGHT) + 1 >= i32::from(grid.height)
                    || !h3_stamp_fits(grid, origin_x, origin_y, FIELD_WIDTH, FIELD_HEIGHT, 3)
                {
                    continue;
                }
                let footprint_is_clear = (0..FIELD_HEIGHT).all(|dy| {
                    (0..FIELD_WIDTH).all(|dx| {
                        matches!(
                            grid.cell(
                                (origin_x + i32::from(dx)) as u16,
                                (origin_y + i32::from(dy)) as u16
                            ),
                            Some(
                                MapCell::Grass | MapCell::Lawn | MapCell::Clearing | MapCell::Pitch
                            )
                        )
                    })
                });
                if !footprint_is_clear {
                    continue;
                }
                let pitch_overlap = (0..FIELD_HEIGHT)
                    .flat_map(|dy| (0..FIELD_WIDTH).map(move |dx| (dx, dy)))
                    .filter(|&(dx, dy)| {
                        let x = (origin_x + i32::from(dx)) as usize;
                        let y = (origin_y + i32::from(dy)) as usize;
                        original_pitch[y * width + x]
                    })
                    .count();
                if pitch_overlap == 0 {
                    continue;
                }

                for gate in [
                    FieldGate::North,
                    FieldGate::South,
                    FieldGate::West,
                    FieldGate::East,
                ] {
                    let gate_cell = field_gate_cell(origin_x, origin_y, gate);
                    let Some(connector) = scenic_connector(grid, gate_cell, MAX_CONNECTOR_STEPS)
                    else {
                        continue;
                    };
                    if connector.iter().skip(1).any(|&(x, y)| {
                        x >= origin_x
                            && x < origin_x + i32::from(FIELD_WIDTH)
                            && y >= origin_y
                            && y < origin_y + i32::from(FIELD_HEIGHT)
                    }) {
                        continue;
                    }
                    proposals.push(PublicFieldProposal {
                        origin_x: origin_x as u16,
                        origin_y: origin_y as u16,
                        gate,
                        gate_cell,
                        source_distance: (origin_x + i32::from(FIELD_WIDTH / 2) - center_x as i32)
                            .abs()
                            + (origin_y + i32::from(FIELD_HEIGHT / 2) - center_y as i32).abs(),
                        pitch_overlap,
                        connector,
                    });
                }
            }
        }
    }

    proposals.sort_by_key(|proposal| {
        (
            new_trail_steps(grid, &proposal.connector),
            std::cmp::Reverse(proposal.pitch_overlap),
            proposal.source_distance,
            path_turns(&proposal.connector),
            hash(proposal.origin_x, proposal.origin_y),
        )
    });
    let selected = proposals.into_iter().next();

    // Every unselected pitch fragment returns to ordinary lawn. Keeping them
    // as invisible semantic blobs made the map sparse without adding a room.
    for cell in &mut grid.cells {
        if *cell == MapCell::Pitch {
            *cell = MapCell::Grass;
        }
    }
    let Some(proposal) = selected else {
        return;
    };

    stamp_public_field(grid, &proposal);
    commit_trail(grid, proposal.connector.clone());
    let sign = field_sign_site(grid, &proposal);
    if let Some((sign_x, sign_y)) = sign
        && h3_protected_cell_fits(grid, sign_x as u16, sign_y as u16)
    {
        set_cell(grid, sign_x, sign_y, MapCell::GroundSign);
        grid.labels.push(GridLabel {
            text: nearest_pitch_name(grid, proposal.origin_x, proposal.origin_y)
                .unwrap_or_else(|| "NEIGHBORHOOD FIELD".to_string()),
            x: sign_x as u16,
            y: sign_y as u16,
        });
    }
}

fn field_gate_cell(origin_x: i32, origin_y: i32, gate: FieldGate) -> (i32, i32) {
    const FIELD_WIDTH: i32 = 9;
    const FIELD_HEIGHT: i32 = 7;
    match gate {
        FieldGate::North => (origin_x + FIELD_WIDTH / 2, origin_y),
        FieldGate::South => (origin_x + FIELD_WIDTH / 2, origin_y + FIELD_HEIGHT - 1),
        FieldGate::West => (origin_x, origin_y + FIELD_HEIGHT / 2),
        FieldGate::East => (origin_x + FIELD_WIDTH - 1, origin_y + FIELD_HEIGHT / 2),
    }
}

fn stamp_public_field(grid: &mut GeneratedGrid, proposal: &PublicFieldProposal) {
    const FIELD_WIDTH: u16 = 9;
    const FIELD_HEIGHT: u16 = 7;
    for dy in 0..FIELD_HEIGHT {
        for dx in 0..FIELD_WIDTH {
            let x = proposal.origin_x + dx;
            let y = proposal.origin_y + dy;
            if (i32::from(x), i32::from(y)) == proposal.gate_cell {
                set_cell(grid, i32::from(x), i32::from(y), MapCell::Trail);
                continue;
            }
            let cell = match (dx, dy) {
                (0, 0) => MapCell::FenceNorthWest,
                (x, 0) if x + 1 == FIELD_WIDTH => MapCell::FenceNorthEast,
                (_, 0) => MapCell::FenceNorth,
                (0, y) if y + 1 == FIELD_HEIGHT => MapCell::FenceSouthWest,
                (x, y) if x + 1 == FIELD_WIDTH && y + 1 == FIELD_HEIGHT => MapCell::FenceSouthEast,
                (_, y) if y + 1 == FIELD_HEIGHT => MapCell::FenceSouth,
                (0, _) => MapCell::FenceWest,
                (x, _) if x + 1 == FIELD_WIDTH => MapCell::FenceEast,
                _ => MapCell::Pitch,
            };
            set_cell(grid, i32::from(x), i32::from(y), cell);
        }
    }

    // A two-block interior approach keeps the gate readable and prevents the
    // field from becoming a closed decorative rectangle.
    let inward = match proposal.gate {
        FieldGate::North => (0, 1),
        FieldGate::South => (0, -1),
        FieldGate::West => (1, 0),
        FieldGate::East => (-1, 0),
    };
    for step in 0..=2 {
        set_cell(
            grid,
            proposal.gate_cell.0 + inward.0 * step,
            proposal.gate_cell.1 + inward.1 * step,
            MapCell::Trail,
        );
    }

    // Give the field a small authored garden focus so its interior reads as a
    // lived-in public park rather than one blank green rectangle. The offset
    // paired edge beds cannot divide the remaining playable field.
    for (dx, dy) in [(2_i32, 1_i32), (3, 1), (5, 5), (6, 5)] {
        let x = i32::from(proposal.origin_x) + dx;
        let y = i32::from(proposal.origin_y) + dy;
        if grid.cell(x as u16, y as u16) == Some(MapCell::Pitch) {
            set_cell(grid, x, y, MapCell::Flowers);
        }
    }
}

fn field_sign_site(grid: &GeneratedGrid, proposal: &PublicFieldProposal) -> Option<(i32, i32)> {
    let outward = match proposal.gate {
        FieldGate::North => (0, -1),
        FieldGate::South => (0, 1),
        FieldGate::West => (-1, 0),
        FieldGate::East => (1, 0),
    };
    let sideways = (-outward.1, outward.0);
    for side in [1, -1, 2, -2] {
        let x = proposal.gate_cell.0 + outward.0 + sideways.0 * side;
        let y = proposal.gate_cell.1 + outward.1 + sideways.1 * side;
        if x > 0
            && y > 0
            && x + 1 < i32::from(grid.width)
            && y + 1 < i32::from(grid.height)
            && matches!(
                grid.cell(x as u16, y as u16),
                Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
            )
            && !proposal.connector.contains(&(x, y))
        {
            return Some((x, y));
        }
    }
    None
}

fn nearest_pitch_name(grid: &GeneratedGrid, origin_x: u16, origin_y: u16) -> Option<String> {
    grid.source
        .features
        .iter()
        .filter(|feature| feature.kind == FeatureKind::Pitch)
        .filter_map(|feature| {
            let name = feature.name.clone()?;
            let point = feature.points.get(feature.points.len() / 2)?;
            let (x, y) = project(grid, *point);
            Some((
                (x - i32::from(origin_x)).abs() + (y - i32::from(origin_y)).abs(),
                name,
            ))
        })
        .min_by_key(|(distance, _)| *distance)
        .map(|(_, name)| name)
}

#[derive(Debug, Clone)]
struct GlobalRoadCorridor {
    kind: FeatureKind,
    axis: RoadAxis,
    lane: i64,
    start: i64,
    end: i64,
}

/// Normalize each source way independently onto a canonical global lane.
///
/// There is no response-local winner: another bbox may add a segment to the
/// same band, but it cannot replace or move a segment already shared by both
/// responses. East/west ways use one lane per twenty global cells. Only mapped
/// Road/MajorRoad ways survive north/south compression, on one lane per forty-
/// eight cells. Every hierarchy is exactly one metatile wide.
fn global_road_corridors(
    grid: &GeneratedGrid,
    world_grid: WorldGrid,
) -> Result<Vec<GlobalRoadCorridor>> {
    let mut corridors = Vec::new();
    for feature in grid.source.features.iter().filter(|feature| {
        matches!(
            feature.kind,
            FeatureKind::Street | FeatureKind::Road | FeatureKind::MajorRoad
        )
    }) {
        let projected = feature
            .points
            .iter()
            .map(|point| world_grid.project_cell(*point))
            .collect::<Result<Vec<_>>>()?;
        if projected.len() < 2 {
            continue;
        }
        let min_x = projected.iter().map(|point| point.x).min().unwrap_or(0);
        let max_x = projected.iter().map(|point| point.x).max().unwrap_or(0);
        let min_y = projected.iter().map(|point| point.y).min().unwrap_or(0);
        let max_y = projected.iter().map(|point| point.y).max().unwrap_or(0);
        let axis = if max_x - min_x >= max_y - min_y {
            RoadAxis::EastWest
        } else {
            RoadAxis::NorthSouth
        };
        if axis == RoadAxis::NorthSouth && feature.kind == FeatureKind::Street {
            continue;
        }
        let mut ordinates = projected
            .iter()
            .map(|point| match axis {
                RoadAxis::EastWest => point.y,
                RoadAxis::NorthSouth => point.x,
            })
            .collect::<Vec<_>>();
        ordinates.sort_unstable();
        let band_width = match axis {
            RoadAxis::EastWest => 20,
            RoadAxis::NorthSouth => 48,
        };
        let lane = canonical_global_lane(ordinates[ordinates.len() / 2], band_width);
        let (start, end) = match axis {
            RoadAxis::EastWest => (min_x, max_x),
            RoadAxis::NorthSouth => (min_y, max_y),
        };
        if end - start >= 6 {
            corridors.push(GlobalRoadCorridor {
                kind: feature.kind,
                axis,
                lane,
                start,
                end,
            });
        }
    }
    corridors.sort_by_key(|corridor| {
        (
            match corridor.axis {
                RoadAxis::EastWest => 0,
                RoadAxis::NorthSouth => 1,
            },
            corridor.lane,
            corridor.start,
            corridor.end,
            road_kind_priority(corridor.kind),
        )
    });
    Ok(corridors)
}

fn canonical_global_lane(value: i64, band_width: i64) -> i64 {
    value.div_euclid(band_width) * band_width + band_width / 2
}

fn road_kind_priority(kind: FeatureKind) -> i32 {
    match kind {
        FeatureKind::MajorRoad => 3,
        FeatureKind::Road => 2,
        FeatureKind::Street => 1,
        _ => 0,
    }
}

fn paint_global_road_corridors(
    grid: &mut GeneratedGrid,
    world_grid: WorldGrid,
    corridors: &[GlobalRoadCorridor],
) {
    let mut cells = std::collections::BTreeMap::<WorldCell, FeatureKind>::new();
    for corridor in corridors {
        for world in visible_corridor_cells(world_grid, corridor) {
            cells
                .entry(world)
                .and_modify(|kind| {
                    if road_kind_priority(corridor.kind) > road_kind_priority(*kind) {
                        *kind = corridor.kind;
                    }
                })
                .or_insert(corridor.kind);
        }
    }
    for (world, kind) in cells {
        let Some((x, y)) = world_grid.local_cell(world) else {
            continue;
        };
        if !matches!(
            grid.cell(x, y),
            Some(
                MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth
            )
        ) {
            set_cell(grid, i32::from(x), i32::from(y), MapCell::from(kind));
        }
    }
}

fn visible_corridor_cells(world_grid: WorldGrid, corridor: &GlobalRoadCorridor) -> Vec<WorldCell> {
    match corridor.axis {
        RoadAxis::EastWest => {
            let start = corridor.start.max(world_grid.west);
            let end = corridor
                .end
                .min(world_grid.west + i64::from(world_grid.width) - 1);
            (start..=end)
                .map(|x| WorldCell {
                    x,
                    y: corridor.lane,
                })
                .collect()
        }
        RoadAxis::NorthSouth => {
            let south = corridor
                .start
                .max(world_grid.north - i64::from(world_grid.height) + 1);
            let north = corridor.end.min(world_grid.north);
            (south..=north)
                .map(|y| WorldCell {
                    x: corridor.lane,
                    y,
                })
                .collect()
        }
    }
}

fn bridge_mapped_roads_around_water(
    grid: &mut GeneratedGrid,
    world_grid: WorldGrid,
    corridors: &[GlobalRoadCorridor],
) {
    for corridor in corridors {
        let visible = visible_corridor_cells(world_grid, corridor)
            .into_iter()
            .filter_map(|world| world_grid.local_cell(world))
            .collect::<Vec<_>>();
        let mut previous_bank = None::<(i32, i32)>;
        let mut crossed_water = false;
        for (x, y) in visible {
            let local = (i32::from(x), i32::from(y));
            match grid.cell(x, y) {
                Some(
                    MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth,
                ) => crossed_water = previous_bank.is_some(),
                Some(MapCell::Street | MapCell::Road | MapCell::MajorRoad) => {
                    if crossed_water
                        && let Some(bank) = previous_bank
                        && let Some(path) = shortest_route_path(grid, bank, local)
                    {
                        commit_trail(grid, path);
                    }
                    previous_bank = Some(local);
                    crossed_water = false;
                }
                _ => {
                    previous_bank = None;
                    crossed_water = false;
                }
            }
        }
    }
}

fn transport_cells(grid: &GeneratedGrid) -> Vec<(i32, i32)> {
    let width = usize::from(grid.width);
    grid.cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            matches!(
                cell,
                MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
            )
            .then_some(((index % width) as i32, (index / width) as i32))
        })
        .collect()
}

fn commit_trail(grid: &mut GeneratedGrid, path: Vec<(i32, i32)>) {
    for (x, y) in path {
        if matches!(
            grid.cell(x as u16, y as u16),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing | MapCell::Trail)
        ) {
            set_cell(grid, x, y, MapCell::Trail);
        }
    }
}

fn carve_path(grid: &mut GeneratedGrid, start: (i32, i32), goal: (i32, i32)) {
    if let Some(path) = shortest_route_path(grid, start, goal) {
        commit_trail(grid, path);
    }
}

fn shortest_land_path(
    grid: &GeneratedGrid,
    start: (i32, i32),
    goal: (i32, i32),
) -> Option<Vec<(i32, i32)>> {
    shortest_path(grid, start, goal, false)
}

fn shortest_route_path(
    grid: &GeneratedGrid,
    start: (i32, i32),
    goal: (i32, i32),
) -> Option<Vec<(i32, i32)>> {
    shortest_path(grid, start, goal, true)
}

fn shortest_path(
    grid: &GeneratedGrid,
    start: (i32, i32),
    goal: (i32, i32),
    preserve_wild_grass: bool,
) -> Option<Vec<(i32, i32)>> {
    shortest_path_avoiding(
        grid,
        start,
        goal,
        preserve_wild_grass,
        &std::collections::BTreeSet::new(),
    )
}

fn shortest_path_avoiding(
    grid: &GeneratedGrid,
    start: (i32, i32),
    goal: (i32, i32),
    preserve_wild_grass: bool,
    excluded: &std::collections::BTreeSet<usize>,
) -> Option<Vec<(i32, i32)>> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let index = |x: i32, y: i32| y as usize * width + x as usize;
    let mut previous = vec![None; width * height];
    let mut frontier = std::collections::VecDeque::from([start]);
    previous[index(start.0, start.1)] = Some(start);
    while let Some((x, y)) = frontier.pop_front() {
        if (x, y) == goal {
            let mut path = vec![goal];
            let mut cursor = goal;
            while cursor != start {
                cursor = previous[index(cursor.0, cursor.1)]?;
                path.push(cursor);
            }
            path.reverse();
            return Some(path);
        }
        let mut neighbors = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)];
        neighbors
            .sort_by_key(|&(next_x, next_y)| (goal.0 - next_x).abs() + (goal.1 - next_y).abs());
        for (next_x, next_y) in neighbors {
            if next_x < 0 || next_y < 0 || next_x >= width as i32 || next_y >= height as i32 {
                continue;
            }
            let next = index(next_x, next_y);
            if previous[next].is_some()
                || ((next_x, next_y) != goal && excluded.contains(&next))
                || matches!(
                    grid.cell(next_x as u16, next_y as u16),
                    Some(
                        MapCell::H3Void
                            | MapCell::Water
                            | MapCell::WaterAccessEast
                            | MapCell::WaterAccessWest
                            | MapCell::WaterAccessSouth
                            | MapCell::Building
                            | MapCell::PokecenterNorthWest
                            | MapCell::PokecenterNorthEast
                            | MapCell::PokecenterSouthWest
                            | MapCell::PokecenterSouthEast
                            | MapCell::MartNorthWest
                            | MapCell::MartNorthEast
                            | MapCell::MartSouthWest
                            | MapCell::MartSouthEast
                            | MapCell::Bench
                            | MapCell::TrashCan
                            | MapCell::Fountain
                            | MapCell::GroundSign
                            | MapCell::FenceNorthWest
                            | MapCell::FenceNorth
                            | MapCell::FenceNorthEast
                            | MapCell::FenceWest
                            | MapCell::FenceEast
                            | MapCell::FenceSouthWest
                            | MapCell::FenceSouth
                            | MapCell::FenceSouthEast
                            | MapCell::LedgeWest
                            | MapCell::LedgeMiddle
                            | MapCell::LedgeEast
                            | MapCell::CliffNorthWest
                            | MapCell::CliffNorth
                            | MapCell::CliffNorthEast
                            | MapCell::CliffWest
                            | MapCell::CliffCenter
                            | MapCell::CliffEast
                            | MapCell::CliffSouthWest
                            | MapCell::CliffSouth
                            | MapCell::CliffSouthEast
                            | MapCell::CliffInnerSouthWest
                            | MapCell::CliffInnerSouthEast
                    )
                )
                || (preserve_wild_grass
                    && (next_x, next_y) != goal
                    && (next_x, next_y) != start
                    && matches!(
                        grid.cell(next_x as u16, next_y as u16),
                        Some(
                            MapCell::Park
                                | MapCell::Flowers
                                | MapCell::Tree
                                | MapCell::ParkTree
                                | MapCell::SmallTree
                                | MapCell::SmallTreeSouth
                                | MapCell::Boulder
                                | MapCell::Pitch
                                | MapCell::Rail
                        )
                    ))
            {
                continue;
            }
            previous[next] = Some((x, y));
            frontier.push_back((next_x, next_y));
        }
    }
    None
}

fn terrain_components(grid: &GeneratedGrid, wanted: MapCell) -> Vec<Vec<usize>> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut visited = vec![false; grid.cells.len()];
    let mut result = Vec::new();
    for start in 0..grid.cells.len() {
        if visited[start] || grid.cells[start] != wanted {
            continue;
        }
        let mut component = Vec::new();
        let mut frontier = vec![start];
        visited[start] = true;
        while let Some(current) = frontier.pop() {
            component.push(current);
            let x = current % width;
            let y = current / width;
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= width || next_y >= height {
                    continue;
                }
                let next = next_y * width + next_x;
                if !visited[next] && grid.cells[next] == wanted {
                    visited[next] = true;
                    frontier.push(next);
                }
            }
        }
        result.push(component);
    }
    result
}

fn paint_feature(grid: &mut GeneratedGrid, feature: &Feature) {
    let points = feature
        .points
        .iter()
        .map(|point| project(grid, *point))
        .collect::<Vec<_>>();
    if points.is_empty() {
        return;
    }
    let cell = MapCell::from(feature.kind);
    if feature.area && points.len() >= 3 {
        fill_polygon(grid, &points, cell);
    }
    for pair in points.windows(2) {
        if feature.kind == FeatureKind::Water && !feature.area {
            paint_cardinal_line(grid, pair[0], pair[1], cell);
        } else {
            paint_line(grid, pair[0], pair[1], 0, cell);
        }
    }
}

fn selected_features(grid: &GeneratedGrid, kind: FeatureKind) -> Vec<Feature> {
    let mut features = grid
        .source
        .features
        .iter()
        .filter(|feature| feature.kind == kind)
        .cloned()
        .collect::<Vec<_>>();
    if kind == FeatureKind::Park {
        // Keep the three dominant mapped parks as authored encounter rooms;
        // the biome layer supplies varied meadow/wetland/forest texture around
        // them without turning every pocket playground into tall grass.
        features.sort_by_key(|feature| std::cmp::Reverse(feature.points.len()));
        features.truncate(3);
    }
    features
}

fn project(grid: &GeneratedGrid, point: Coordinate) -> (i32, i32) {
    if let Some(plan) = &grid.source.h3 {
        return plan
            .project_to_grid(point, grid.width, grid.height)
            .expect("H3 plan was validated before generation");
    }
    let bounds = grid.source.bounds;
    let x = ((point.lon - bounds.west) / (bounds.east - bounds.west) * f64::from(grid.width - 1))
        .round();
    let y = ((bounds.north - point.lat) / (bounds.north - bounds.south)
        * f64::from(grid.height - 1))
    .round();
    (
        x.clamp(0.0, f64::from(grid.width - 1)) as i32,
        y.clamp(0.0, f64::from(grid.height - 1)) as i32,
    )
}

fn author_h3_boundary(grid: &mut GeneratedGrid, seams: &H3SeamContract) -> Result<()> {
    let plan = grid.source.h3.clone().expect("H3 seam requires H3 plan");
    let polygon = plan.raster_polygon(grid.width, grid.height)?;
    let width = usize::from(grid.width);
    let snapshot = grid.cells.clone();
    let mut inside = vec![false; grid.cells.len()];
    for y in 0..grid.height {
        for x in 0..grid.width {
            inside[usize::from(y) * width + usize::from(x)] =
                point_in_float_polygon(f64::from(x) + 0.5, f64::from(y) + 0.5, &polygon);
        }
    }
    let mut boundary = vec![false; grid.cells.len()];
    for y in 0..grid.height {
        for x in 0..grid.width {
            let index = usize::from(y) * width + usize::from(x);
            boundary[index] = inside[index]
                && [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)]
                    .into_iter()
                    .any(|(dx, dy)| {
                        let check_x = i32::from(x) + dx;
                        let check_y = i32::from(y) + dy;
                        check_x < 0
                            || check_y < 0
                            || check_x >= i32::from(grid.width)
                            || check_y >= i32::from(grid.height)
                            || !inside[check_y as usize * width + check_x as usize]
                    });
        }
    }
    for y in 0..grid.height {
        for x in 0..grid.width {
            let index = usize::from(y) * width + usize::from(x);
            if inside[index] && !boundary[index] {
                continue;
            }
            grid.cells[index] = if inside[index] {
                // A storage-face join is not a biome boundary. Preserve the
                // authored terrain except a canopy cell that exists only on
                // this one-cell rim. Forest that continues toward the face
                // interior remains intact, as do every water and transport
                // cell. In particular, a water midpoint must never flood the
                // entire shared edge; exact OSM water painted in the local
                // raster is preserved and the batch grid-seam audit verifies
                // it at off-midpoint samples.
                if matches!(
                    snapshot[index],
                    MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                ) {
                    MapCell::Water
                } else {
                    snapshot[index]
                }
            } else {
                MapCell::H3Void
            };
        }
    }

    // Re-open only authoritative shared-edge crossings. The local route was
    // already painted from the same OSM polyline; this short landing merely
    // prevents the natural boundary art from capping its endpoint.
    for edge in seams.edges.iter().filter(|edge| edge.transport.is_some()) {
        let crossing = edge.crossing.expect("transport edge has crossing");
        let cell = MapCell::from(edge.transport.expect("transport edge kind"));
        for (x, y) in crate::h3::h3_raster_sample_band(&plan, grid, crossing)? {
            let index = usize::from(y) * width + usize::from(x);
            if !matches!(snapshot[index], MapCell::Water) {
                grid.cells[index] = cell;
            }
        }
    }
    feather_h3_sampled_canopy_outlines(grid)?;
    Ok(())
}

fn feather_h3_sampled_canopy_outlines(grid: &mut GeneratedGrid) -> Result<()> {
    let plan = grid
        .source
        .h3
        .clone()
        .expect("canopy seam feathering follows validated H3 boundary authoring");
    let profile = crate::build_h3_grid_seam_profile(grid)?;
    let width = usize::from(grid.width);
    let mut outlined = std::collections::BTreeSet::new();
    let mut continuing = std::collections::BTreeSet::new();
    for edge in profile.edges {
        for sample in edge.samples {
            let band = crate::h3::h3_raster_sample_band(&plan, grid, sample.coordinate)?;
            let border = usize::from(band[0].1) * width + usize::from(band[0].0);
            if !h3_canopy_cell(grid.cells[border]) {
                continue;
            }
            let inner = usize::from(band[2].1) * width + usize::from(band[2].0);
            if h3_canopy_cell(grid.cells[inner]) {
                continuing.insert(border);
            } else {
                outlined.insert(border);
            }
        }
    }
    for index in outlined.difference(&continuing).copied() {
        grid.cells[index] = match grid.cells[index] {
            MapCell::ParkTree => MapCell::Park,
            _ => MapCell::Grass,
        };
    }
    Ok(())
}

fn point_in_float_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        if (current.1 > y) != (previous.1 > y)
            && x < (previous.0 - current.0) * (y - current.1) / (previous.1 - current.1) + current.0
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn paint_line(
    grid: &mut GeneratedGrid,
    from: (i32, i32),
    to: (i32, i32),
    radius: i32,
    cell: MapCell,
) {
    let (mut x, mut y) = from;
    let (x1, y1) = to;
    let dx = (x1 - x).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let dy = -(y1 - y).abs();
    let sy = if y < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        for by in -radius..=radius {
            for bx in -radius..=radius {
                paint(grid, x + bx, y + by, cell);
            }
        }
        if (x, y) == (x1, y1) {
            break;
        }
        let doubled = error * 2;
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
    }
}

/// Paints a one-cell-wide line whose successive cells are cardinally connected.
///
/// Ordinary Bresenham output may advance both axes at once. That is visually
/// acceptable for outlines, but it turns a narrow river into disconnected
/// diagonal droplets for gameplay and for the water-component cleanup pass.
fn paint_cardinal_line(grid: &mut GeneratedGrid, from: (i32, i32), to: (i32, i32), cell: MapCell) {
    let (mut x, mut y) = from;
    let (x1, y1) = to;
    let dx = (x1 - x).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let dy = -(y1 - y).abs();
    let sy = if y < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        paint(grid, x, y, cell);
        if (x, y) == (x1, y1) {
            break;
        }
        let previous = (x, y);
        let doubled = error * 2;
        if doubled >= dy {
            error += dy;
            x += sx;
        }
        if doubled <= dx {
            error += dx;
            y += sy;
        }
        if x != previous.0 && y != previous.1 {
            paint(grid, x, previous.1, cell);
        }
    }
}

fn fill_polygon(grid: &mut GeneratedGrid, polygon: &[(i32, i32)], cell: MapCell) {
    let min_x = polygon.iter().map(|point| point.0).min().unwrap_or(0);
    let max_x = polygon.iter().map(|point| point.0).max().unwrap_or(0);
    let min_y = polygon.iter().map(|point| point.1).min().unwrap_or(0);
    let max_y = polygon.iter().map(|point| point.1).max().unwrap_or(0);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if point_in_polygon(f64::from(x) + 0.5, f64::from(y) + 0.5, polygon) {
                paint(grid, x, y, cell);
            }
        }
    }
}

fn point_in_polygon(x: f64, y: f64, polygon: &[(i32, i32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        let (xi, yi) = (f64::from(current.0), f64::from(current.1));
        let (xj, yj) = (f64::from(previous.0), f64::from(previous.1));
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn paint(grid: &mut GeneratedGrid, x: i32, y: i32, cell: MapCell) {
    if x < 0 || y < 0 || x >= i32::from(grid.width) || y >= i32::from(grid.height) {
        return;
    }
    let index = y as usize * usize::from(grid.width) + x as usize;
    if cell.priority() >= grid.cells[index].priority() {
        grid.cells[index] = cell;
    }
}

fn place_pokecenter(grid: &mut GeneratedGrid) {
    let center_x = i32::from(grid.width / 2);
    let center_y = i32::from(grid.height / 2);
    let target_y = center_y + i32::from(grid.height) / 6;
    let candidate = grid
        .source
        .features
        .iter()
        .filter(|feature| feature.kind == FeatureKind::Building && !feature.points.is_empty())
        .filter_map(|feature| {
            let (lat, lon) = feature.points.iter().fold((0.0, 0.0), |sum, point| {
                (sum.0 + point.lat, sum.1 + point.lon)
            });
            let count = feature.points.len() as f64;
            let candidate = project(
                grid,
                Coordinate {
                    lat: lat / count,
                    lon: lon / count,
                },
            );
            ((candidate.0 - center_x)
                .abs()
                .max((candidate.1 - center_y).abs())
                >= 7
                && house_site_is_clear(grid, candidate.0, candidate.1))
            .then_some(candidate)
        })
        .min_by_key(|&(x, y)| {
            (
                (y - target_y).abs(),
                (x - center_x).abs(),
                hash(x as u16, y as u16),
            )
        });
    let Some((x, y)) = candidate else {
        return;
    };
    for (dx, dy, cell) in [
        (0, 0, MapCell::PokecenterNorthWest),
        (1, 0, MapCell::PokecenterNorthEast),
        (0, 1, MapCell::PokecenterSouthWest),
        (1, 1, MapCell::PokecenterSouthEast),
    ] {
        set_cell(grid, x + dx, y + dy, cell);
    }
    clear_facility_forecourt(grid, x, y);
    connect_frontage(grid, (x, y + 2));
}

pub(crate) fn pokecenter_origin(grid: &GeneratedGrid) -> Option<(u16, u16)> {
    for y in 0..grid.height.saturating_sub(1) {
        for x in 0..grid.width.saturating_sub(1) {
            if grid.cell(x, y) == Some(MapCell::PokecenterNorthWest)
                && grid.cell(x + 1, y) == Some(MapCell::PokecenterNorthEast)
                && grid.cell(x, y + 1) == Some(MapCell::PokecenterSouthWest)
                && grid.cell(x + 1, y + 1) == Some(MapCell::PokecenterSouthEast)
            {
                return Some((x, y));
            }
        }
    }
    None
}

fn place_mart(grid: &mut GeneratedGrid) {
    let (target_x, target_y) = pokecenter_origin(grid)
        .map(|(x, y)| (i32::from(x) + 3, i32::from(y)))
        .unwrap_or((
            i32::from(grid.width / 2) + 6,
            i32::from(grid.height / 2) + i32::from(grid.height) / 6,
        ));
    let (home_x, home_y) = grid.home_cell();
    let mut candidates = std::collections::BTreeMap::<(i32, i32), bool>::new();
    for feature in grid
        .source
        .features
        .iter()
        .filter(|feature| feature.kind == FeatureKind::Building && !feature.points.is_empty())
    {
        let (lat, lon) = feature.points.iter().fold((0.0, 0.0), |sum, point| {
            (sum.0 + point.lat, sum.1 + point.lon)
        });
        let count = feature.points.len() as f64;
        candidates.insert(
            project(
                grid,
                Coordinate {
                    lat: lat / count,
                    lon: lon / count,
                },
            ),
            true,
        );
    }
    // Sparse rural inputs may not contain a usable building centroid. The
    // fallback search still stays on the real transport plan and chooses one
    // deterministic service site rather than dropping a fake facade anywhere.
    for y in 2..grid.height.saturating_sub(4) {
        for x in 2..grid.width.saturating_sub(3) {
            candidates.entry((i32::from(x), i32::from(y))).or_default();
        }
    }
    let selected = candidates
        .into_iter()
        .filter(|&((x, y), _)| {
            (x - i32::from(home_x))
                .abs()
                .max((y - i32::from(home_y)).abs())
                >= 6
                && house_site_is_clear(grid, x, y)
        })
        .min_by_key(|&((x, y), real_site)| {
            (
                usize::from(!real_site),
                (x - target_x).abs() + (y - target_y).abs(),
                hash(x as u16, y as u16),
            )
        })
        .map(|(site, _)| site);
    let Some((x, y)) = selected else {
        return;
    };
    for (dx, dy, cell) in [
        (0, 0, MapCell::MartNorthWest),
        (1, 0, MapCell::MartNorthEast),
        (0, 1, MapCell::MartSouthWest),
        (1, 1, MapCell::MartSouthEast),
    ] {
        set_cell(grid, x + dx, y + dy, cell);
    }
    clear_facility_forecourt(grid, x, y);
    connect_frontage(grid, (x, y + 2));
}

pub(crate) fn mart_origin(grid: &GeneratedGrid) -> Option<(u16, u16)> {
    for y in 0..grid.height.saturating_sub(1) {
        for x in 0..grid.width.saturating_sub(1) {
            if grid.cell(x, y) == Some(MapCell::MartNorthWest)
                && grid.cell(x + 1, y) == Some(MapCell::MartNorthEast)
                && grid.cell(x, y + 1) == Some(MapCell::MartSouthWest)
                && grid.cell(x + 1, y + 1) == Some(MapCell::MartSouthEast)
            {
                return Some((x, y));
            }
        }
    }
    None
}

fn place_houses(grid: &mut GeneratedGrid) {
    let urban_intensity = urban_intensity(grid);
    let target_houses = target_house_count(grid);
    let minimum_spacing = match urban_intensity {
        2 => 4,
        1 => 5,
        _ => 6,
    };
    let candidates = grid
        .source
        .features
        .iter()
        .filter(|feature| feature.kind == FeatureKind::Building && !feature.points.is_empty())
        .map(|feature| {
            let (lat, lon) = feature.points.iter().fold((0.0, 0.0), |sum, point| {
                (sum.0 + point.lat, sum.1 + point.lon)
            });
            let count = feature.points.len() as f64;
            project(
                grid,
                Coordinate {
                    lat: lat / count,
                    lon: lon / count,
                },
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    let center_x = i32::from(grid.width / 2);
    let center_y = i32::from(grid.height / 2);
    let mut zones = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    if urban_intensity == 2 {
        let routes = transport_cells(grid);
        let mut ranked = candidates
            .iter()
            .copied()
            .map(|(x, y)| {
                let local_density = (-5..=5)
                    .flat_map(|dy| (-5..=5).map(move |dx| (x + dx, y + dy)))
                    .filter(|candidate| candidates.contains(candidate))
                    .count();
                let route_distance = routes
                    .iter()
                    .map(|&(route_x, route_y)| (route_x - x).abs() + (route_y - y).abs())
                    .min()
                    .unwrap_or(i32::MAX);
                (
                    std::cmp::Reverse(local_density),
                    route_distance,
                    hash(x as u16, y as u16),
                    x,
                    y,
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_unstable();
        let mut anchors = Vec::<(i32, i32)>::new();
        for (_, route_distance, _, x, y) in ranked {
            if route_distance > 8
                || anchors.iter().any(|&(anchor_x, anchor_y)| {
                    (anchor_x - x).abs().max((anchor_y - y).abs()) < 18
                })
            {
                continue;
            }
            anchors.push((x, y));
            if anchors.len() == zones.len() {
                break;
            }
        }
        for &(x, y) in &candidates {
            if let Some((zone, distance)) = anchors
                .iter()
                .enumerate()
                .map(|(index, &(anchor_x, anchor_y))| {
                    (index, (anchor_x - x).abs().max((anchor_y - y).abs()))
                })
                .min_by_key(|&(index, distance)| (distance, index))
                && distance <= 14
            {
                zones[zone].push((x, y));
            }
        }
    } else {
        for (x, y) in candidates {
            let zone = usize::from(x >= center_x) + usize::from(y >= center_y) * 2;
            zones[zone].push((x, y));
        }
    }
    for zone in &mut zones {
        zone.sort_unstable_by_key(|&(x, y)| (hash(x as u16, y as u16), y, x));
    }

    let mut placed = Vec::<(i32, i32)>::new();
    let mut cursors = [0usize; 4];
    while placed.len() < target_houses {
        let mut progressed = false;
        for zone_index in 0..zones.len() {
            while let Some(&(x, y)) = zones[zone_index].get(cursors[zone_index]) {
                cursors[zone_index] += 1;
                progressed = true;
                let home_x = i32::from(grid.width / 2);
                let home_y = i32::from(grid.height / 2) - 2;
                if (x - home_x).abs().max((y - home_y).abs()) < 5
                    || placed.iter().any(|&(placed_x, placed_y)| {
                        (placed_x - x).abs().max((placed_y - y).abs()) < minimum_spacing
                    })
                    || !house_site_is_clear(grid, x, y)
                {
                    continue;
                }
                stamp_neighborhood_building(grid, x, y, urban_intensity);
                soften_house_yard(grid, x, y);
                connect_house_frontage_for_stamp(grid, x, y);
                placed.push((x, y));
                break;
            }
            if placed.len() == target_houses {
                break;
            }
        }
        if !progressed {
            break;
        }
    }

    // Compress a dense real neighborhood into a complete Crystal settlement
    // even when centroid downsampling or the two service buildings consume the
    // first-choice sites. Supplemental plots are still constrained to the
    // mapped route skeleton and the same six-block house spacing.
    let mut supplemental = Vec::new();
    for y in 2..grid.height.saturating_sub(4) {
        for x in 2..grid.width.saturating_sub(3) {
            let x = i32::from(x);
            let y = i32::from(y);
            if !house_site_is_clear(grid, x, y)
                || (x - center_x).abs().max((y - center_y + 2).abs()) < 5
            {
                continue;
            }
            let preferred_spacing = if urban_intensity == 2 { 5 } else { 8 };
            let nearest_house = placed
                .iter()
                .map(|&(placed_x, placed_y)| (placed_x - x).abs().max((placed_y - y).abs()))
                .min()
                .unwrap_or(8);
            supplemental.push((
                (nearest_house - preferred_spacing).abs(),
                hash(x as u16, y as u16),
                x,
                y,
            ));
        }
    }
    supplemental.sort_unstable();
    for (_, _, x, y) in supplemental {
        if placed.len() >= target_houses {
            break;
        }
        if placed.iter().any(|&(placed_x, placed_y)| {
            (placed_x - x).abs().max((placed_y - y).abs()) < minimum_spacing
        }) || !house_site_is_clear(grid, x, y)
        {
            continue;
        }
        stamp_neighborhood_building(grid, x, y, urban_intensity);
        soften_house_yard(grid, x, y);
        connect_house_frontage_for_stamp(grid, x, y);
        placed.push((x, y));
    }
}

/// Stamp one complete 2x2 residential drawing. Crystal's standalone one-block
/// Goldenrod shops are canonical, but beside full houses they read as cropped
/// facade fragments in a regional overview. Reserve them for their source map;
/// generated neighborhoods use only complete modern/traditional residences.
fn stamp_neighborhood_building(grid: &mut GeneratedGrid, x: i32, y: i32, _urban_intensity: u8) {
    for dy in 0..2 {
        for dx in 0..2 {
            set_cell(grid, x + dx, y + dy, MapCell::Building);
        }
    }
}

fn connect_house_frontage_for_stamp(grid: &mut GeneratedGrid, x: i32, y: i32) {
    connect_house_frontage(grid, x, y);
}

/// Add the exact Goldenrod department-store and Radio-Tower silhouettes to
/// sufficiently urban maps. They are scarce civic landmarks, selected near
/// separate route districts before ordinary houses consume their footprints.
fn place_city_landmarks(grid: &mut GeneratedGrid) {
    if urban_intensity(grid) < 2 || grid.width.min(grid.height) < 56 {
        return;
    }
    let square_specifications = [
        // width, height, preferred x, preferred y
        (3, 4, grid.width / 3, grid.height / 3),
        // Goldenrod's tower includes a three-block antenna mast above its
        // two-wide lower facade; the old 2x3 crop visibly cut that top off.
        (2, 6, grid.width * 2 / 3, grid.height / 3),
    ];
    let h3_specifications = grid.source.h3.as_ref().map(|plan| {
        let department = plan.requests_facility(H3Facility::Mart);
        let radio = !department && plan.requests_facility(H3Facility::PokemonCenter);
        (department, radio)
    });
    let specifications: Vec<(u16, u16, u16, u16)> =
        if let Some((department, radio)) = h3_specifications {
            // Regional service allocation is already sparse. Associate at most
            // one civic landmark with that cell: Mart districts get a department
            // store, Center districts get a Radio Tower, and ordinary hexes get
            // neither. A cell can never receive the whole city checklist.
            if department {
                vec![(3, 4, grid.width / 3, grid.height / 3)]
            } else if radio {
                vec![(2, 6, grid.width * 2 / 3, grid.height / 3)]
            } else {
                Vec::new()
            }
        } else if grid.width.min(grid.height) >= 96 {
            square_specifications.to_vec()
        } else {
            vec![(3, 4, grid.width / 3, grid.height / 3)]
        };
    for (width, height, target_x, target_y) in specifications {
        let mut candidates = Vec::new();
        for y in 3..grid.height.saturating_sub(height + 3) {
            for x in 3..grid.width.saturating_sub(width + 3) {
                if !building_stamp_site_is_clear(grid, x, y, width, height) {
                    continue;
                }
                candidates.push((
                    x.abs_diff(target_x) + y.abs_diff(target_y),
                    hash(x, y),
                    x,
                    y,
                ));
            }
        }
        candidates.sort_unstable();
        let Some((_, _, x, y)) = candidates.into_iter().next() else {
            continue;
        };
        for dy in 0..height {
            for dx in 0..width {
                if width == 2 && height == 6 && dy < 3 && dx == 1 {
                    continue;
                }
                set_cell(
                    grid,
                    i32::from(x + dx),
                    i32::from(y + dy),
                    MapCell::Building,
                );
            }
        }
        let door_x = if width == 3 { x + 1 } else { x };
        connect_frontage(grid, (i32::from(door_x), i32::from(y + height)));
    }
}

fn building_stamp_site_is_clear(
    grid: &GeneratedGrid,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> bool {
    h3_stamp_fits(
        grid,
        i32::from(x) - 2,
        i32::from(y) - 2,
        width + 4,
        height + 5,
        3,
    ) && (y..y + height).all(|check_y| {
        (x..x + width).all(|check_x| grid.cell(check_x, check_y) == Some(MapCell::Grass))
    }) && (0..=7).any(|radius| near_transport(grid, x + width / 2, y + height, radius))
}

pub(crate) fn urban_intensity(grid: &GeneratedGrid) -> u8 {
    let building_features = grid
        .source
        .features
        .iter()
        .filter(|feature| feature.kind == FeatureKind::Building)
        .count();
    let normalized = building_features.saturating_mul(4096)
        / usize::from(grid.width)
            .saturating_mul(usize::from(grid.height))
            .max(1);
    if normalized >= 80 {
        2
    } else if normalized >= 30 {
        1
    } else {
        0
    }
}

pub(crate) fn target_house_count(grid: &GeneratedGrid) -> usize {
    let authored_area = usize::from(grid.width).saturating_mul(usize::from(grid.height));
    let scaled = |per_64: usize, minimum: usize, maximum: usize| {
        per_64
            .saturating_mul(authored_area)
            .div_ceil(64 * 64)
            .clamp(minimum, maximum)
    };
    match urban_intensity(grid) {
        2 => scaled(16, 16, 40),
        1 => scaled(12, 12, 30),
        _ => scaled(10, 10, 20),
    }
}

fn clear_facility_forecourt(grid: &mut GeneratedGrid, house_x: i32, house_y: i32) {
    for y in house_y - 1..=house_y + 3 {
        for x in house_x - 1..=house_x + 2 {
            if x >= 0 && y >= 0 && grid.cell(x as u16, y as u16) == Some(MapCell::Grass) {
                set_cell(grid, x, y, MapCell::Lawn);
            }
        }
    }
    // Service buildings need a readable porch, not a shared parking lot.
    // Keep the lawn on both sides and author only the west-door approach.
    for y in house_y + 2..=house_y + 3 {
        if matches!(
            grid.cell(house_x as u16, y as u16),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
        ) {
            set_cell(grid, house_x, y, MapCell::Clearing);
        }
    }
}

fn soften_house_yard(grid: &mut GeneratedGrid, house_x: i32, house_y: i32) {
    for y in house_y - 1..=house_y + 3 {
        for x in house_x - 1..=house_x + 2 {
            if x >= 0 && y >= 0 && grid.cell(x as u16, y as u16) == Some(MapCell::Grass) {
                set_cell(grid, x, y, MapCell::Lawn);
            }
        }
    }
}

fn connect_house_frontage(grid: &mut GeneratedGrid, house_x: i32, house_y: i32) {
    connect_frontage(grid, (house_x, house_y + 2));
}

fn connect_frontage(grid: &mut GeneratedGrid, door: (i32, i32)) {
    let mut routes = transport_cells(grid);
    routes.sort_unstable_by_key(|&(x, y)| (x - door.0).abs() + (y - door.1).abs());
    let connector = routes
        .into_iter()
        .take(24)
        .filter_map(|route| shortest_route_path(grid, door, route))
        .filter(|path| new_trail_steps(grid, path) <= 10)
        .min_by_key(Vec::len);
    if let Some(path) = connector {
        commit_trail(grid, path);
    }
}

fn place_public_amenities(grid: &mut GeneratedGrid) {
    let width = usize::from(grid.width);
    let field = terrain_components(grid, MapCell::Pitch)
        .into_iter()
        .max_by_key(Vec::len);
    let mut benches = Vec::<(u16, u16)>::new();

    // Canonical park benches face south: place two against the inside of the
    // field's north fence, separated enough to read as furniture rather than
    // another fence course.
    if let Some(field) = field {
        let members = field
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut candidates = field
            .iter()
            .filter_map(|&index| {
                let x = (index % width) as u16;
                let y = (index / width) as u16;
                (y > 0
                    && h3_protected_cell_fits(grid, x, y)
                    && matches!(
                        grid.cell(x, y - 1),
                        Some(
                            MapCell::FenceNorth | MapCell::FenceNorthWest | MapCell::FenceNorthEast
                        )
                    ))
                .then_some((x, y))
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable_by_key(|&(x, y)| (y, x));
        for (x, y) in candidates {
            if benches
                .iter()
                .any(|&(other_x, other_y)| x.abs_diff(other_x) < 4 && y.abs_diff(other_y) < 2)
            {
                continue;
            }
            set_cell(grid, i32::from(x), i32::from(y), MapCell::Bench);
            benches.push((x, y));
            if benches.len() == 2 {
                break;
            }
        }

        // National Park uses a real fountain/statue as the focal point between
        // its seating areas. Keep ours near the authored field center, but off
        // the entrance walk, flower beds, and the two bench collision blocks.
        let center_x = field
            .iter()
            .map(|index| (index % width) as u16)
            .map(u32::from)
            .sum::<u32>()
            / field.len().max(1) as u32;
        let center_y = field
            .iter()
            .map(|index| (index / width) as u16)
            .map(u32::from)
            .sum::<u32>()
            / field.len().max(1) as u32;
        let fountain = field
            .iter()
            .filter_map(|&index| {
                let x = (index % width) as u16;
                let y = (index / width) as u16;
                (grid.cell(x, y) == Some(MapCell::Pitch)
                    && h3_protected_cell_fits(grid, x, y)
                    && benches.iter().all(|&(bench_x, bench_y)| {
                        x.abs_diff(bench_x) >= 2 || y.abs_diff(bench_y) >= 2
                    }))
                .then_some((
                    u32::from(x).abs_diff(center_x) + u32::from(y).abs_diff(center_y),
                    hash(x, y),
                    x,
                    y,
                ))
            })
            .min();
        if let Some((_, _, x, y)) = fountain {
            set_cell(grid, i32::from(x), i32::from(y), MapCell::Fountain);
        }

        // Put one interactive can just outside the enclosure, favoring the
        // named field sign and its entrance without occupying the trail.
        let mut trash_candidates = Vec::new();
        for y in 2..grid.height.saturating_sub(2) {
            for x in 2..grid.width.saturating_sub(2) {
                if !matches!(
                    grid.cell(x, y),
                    Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
                ) {
                    continue;
                }
                let index = usize::from(y) * width + usize::from(x);
                if members.contains(&index)
                    || !borders_fence(grid, x, y)
                    || !h3_protected_cell_fits(grid, x, y)
                {
                    continue;
                }
                let sign_distance = grid
                    .labels
                    .iter()
                    .map(|label| x.abs_diff(label.x) + y.abs_diff(label.y))
                    .min()
                    .unwrap_or(u16::MAX);
                trash_candidates.push((sign_distance, hash(x, y), x, y));
            }
        }
        trash_candidates.sort_unstable();
        if let Some((_, _, x, y)) = trash_candidates.into_iter().next() {
            set_cell(grid, i32::from(x), i32::from(y), MapCell::TrashCan);
        }
    }

    // A third bench and second can author the Pokemon Center forecourt. Sites
    // remain off the door/frontage and beside (not on) the connected path.
    if let Some((center_x, center_y)) = pokecenter_origin(grid) {
        let center = (i32::from(center_x), i32::from(center_y));
        let mut bench_candidates = Vec::new();
        for y in center_y.saturating_sub(4)..=(center_y + 6).min(grid.height - 2) {
            for x in center_x.saturating_sub(6)..=(center_x + 7).min(grid.width - 2) {
                let distance = (i32::from(x) - center.0).abs() + (i32::from(y) - center.1).abs();
                if !(3..=8).contains(&distance)
                    || !amenity_ground(grid.cell(x, y))
                    || near_transport(grid, x, y, 0)
                    || !near_transport(grid, x, y, 1)
                    || near_house_frontage(grid, x, y)
                    || !amenity_clearance(grid, x, y)
                    || !h3_protected_cell_fits(grid, x, y)
                {
                    continue;
                }
                bench_candidates.push((
                    (i32::from(y) - center.1 - 2).abs(),
                    distance,
                    hash(x, y),
                    x,
                    y,
                ));
            }
        }
        bench_candidates.sort_unstable();
        if let Some((_, _, _, x, y)) = bench_candidates.into_iter().next() {
            set_cell(grid, i32::from(x), i32::from(y), MapCell::Bench);
            benches.push((x, y));
        }

        let mut trash_candidates = Vec::new();
        for y in center_y.saturating_sub(3)..=(center_y + 5).min(grid.height - 2) {
            for x in center_x.saturating_sub(5)..=(center_x + 6).min(grid.width - 2) {
                let distance = (i32::from(x) - center.0).abs() + (i32::from(y) - center.1).abs();
                if !(2..=7).contains(&distance)
                    || !amenity_ground(grid.cell(x, y))
                    || near_transport(grid, x, y, 0)
                    || !near_transport(grid, x, y, 1)
                    || near_house_frontage(grid, x, y)
                    || !h3_protected_cell_fits(grid, x, y)
                    || benches.iter().any(|&(bench_x, bench_y)| {
                        x.abs_diff(bench_x) < 2 && y.abs_diff(bench_y) < 2
                    })
                {
                    continue;
                }
                trash_candidates.push((distance, hash(y, x), x, y));
            }
        }
        trash_candidates.sort_unstable();
        if let Some((_, _, x, y)) = trash_candidates.into_iter().next() {
            set_cell(grid, i32::from(x), i32::from(y), MapCell::TrashCan);
        }
    }
}

fn ensure_public_details(grid: &mut GeneratedGrid) {
    let area = usize::from(grid.width) * usize::from(grid.height);
    let fountain_target = area.div_ceil(8_192).clamp(1, 3);
    let bench_target = area.div_ceil(2_048).clamp(2, 8);
    let trash_target = area.div_ceil(3_072).clamp(2, 6);
    top_up_public_fixture(grid, MapCell::Fountain, fountain_target, 7);
    top_up_public_fixture(grid, MapCell::Bench, bench_target, 5);
    top_up_public_fixture(grid, MapCell::TrashCan, trash_target, 4);
}

fn top_up_public_fixture(
    grid: &mut GeneratedGrid,
    fixture: MapCell,
    target: usize,
    separation: u16,
) {
    let mut count = grid.cells.iter().filter(|cell| **cell == fixture).count();
    while count < target {
        let mut candidates = Vec::new();
        for y in 3..grid.height.saturating_sub(3) {
            for x in 3..grid.width.saturating_sub(3) {
                if !amenity_ground(grid.cell(x, y))
                    || near_transport(grid, x, y, 0)
                    || !near_transport(grid, x, y, 3)
                    || near_house_frontage(grid, x, y)
                    || !amenity_clearance(grid, x, y)
                    || !h3_protected_cell_fits(grid, x, y)
                {
                    continue;
                }
                let crowded = (y.saturating_sub(separation)
                    ..=(y + separation).min(grid.height - 1))
                    .any(|check_y| {
                        (x.saturating_sub(separation)..=(x + separation).min(grid.width - 1)).any(
                            |check_x| {
                                matches!(
                                    grid.cell(check_x, check_y),
                                    Some(MapCell::Bench | MapCell::TrashCan | MapCell::Fountain)
                                )
                            },
                        )
                    });
                if !crowded {
                    let route_distance = (1..=3)
                        .find(|&radius| near_transport(grid, x, y, radius))
                        .unwrap_or(4);
                    candidates.push((route_distance, hash(x, y), y, x));
                }
            }
        }
        candidates.sort_unstable();
        let Some((_, _, y, x)) = candidates.into_iter().next() else {
            break;
        };
        set_cell(grid, i32::from(x), i32::from(y), fixture);
        count += 1;
    }
}

fn amenity_ground(cell: Option<MapCell>) -> bool {
    matches!(
        cell,
        Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
    )
}

fn amenity_clearance(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    for check_y in y.saturating_sub(1)..=(y + 1).min(grid.height - 1) {
        for check_x in x.saturating_sub(1)..=(x + 1).min(grid.width - 1) {
            if matches!(
                grid.cell(check_x, check_y),
                Some(
                    MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                        | MapCell::Bench
                        | MapCell::TrashCan
                        | MapCell::Fountain
                        | MapCell::GroundSign
                        | MapCell::FenceNorthWest
                        | MapCell::FenceNorth
                        | MapCell::FenceNorthEast
                        | MapCell::FenceWest
                        | MapCell::FenceEast
                        | MapCell::FenceSouthWest
                        | MapCell::FenceSouth
                        | MapCell::FenceSouthEast
                )
            ) {
                return false;
            }
        }
    }
    true
}

fn borders_fence(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    for (check_x, check_y) in [
        (x.saturating_sub(1), y),
        ((x + 1).min(grid.width - 1), y),
        (x, y.saturating_sub(1)),
        (x, (y + 1).min(grid.height - 1)),
    ] {
        if matches!(
            grid.cell(check_x, check_y),
            Some(
                MapCell::FenceNorthWest
                    | MapCell::FenceNorth
                    | MapCell::FenceNorthEast
                    | MapCell::FenceWest
                    | MapCell::FenceEast
                    | MapCell::FenceSouthWest
                    | MapCell::FenceSouth
                    | MapCell::FenceSouthEast
            )
        ) {
            return true;
        }
    }
    false
}

fn house_site_is_clear(grid: &GeneratedGrid, x: i32, y: i32) -> bool {
    if x < 1 || y < 1 || x + 2 >= i32::from(grid.width) || y + 3 >= i32::from(grid.height) {
        return false;
    }
    // Reserve the complete 2x2 facade, its visible 4x5 yard/frontage stamp,
    // and all three inward cells used by reciprocal seam reconciliation.
    // Merely keeping the facade two cells inside the polygon allowed a valid
    // house at x=2 to collide with sample depth two and forced a late choice
    // between truncating the house and preserving a geographic join.
    if !h3_stamp_fits(grid, x - 1, y - 1, 4, 5, 3) {
        return false;
    }
    // Keep ordinary facades as separate readable components from a previously
    // authored Department Store, Radio Tower, Center, or Mart. Without this
    // halo a valid 2x2 footprint could touch a landmark side and merge both
    // exact stamps into one malformed building component.
    if (y - 1..=y + 2).any(|check_y| {
        (x - 1..=x + 2).any(|check_x| {
            check_x >= 0
                && check_y >= 0
                && matches!(
                    grid.cell(check_x as u16, check_y as u16),
                    Some(
                        MapCell::Building
                            | MapCell::PokecenterNorthWest
                            | MapCell::PokecenterNorthEast
                            | MapCell::PokecenterSouthWest
                            | MapCell::PokecenterSouthEast
                            | MapCell::MartNorthWest
                            | MapCell::MartNorthEast
                            | MapCell::MartSouthWest
                            | MapCell::MartSouthEast
                    )
                )
        })
    }) {
        return false;
    }
    // Keep the exact 2x2 footprint off water, parks, rails, and roads. The
    // separate placement radius guarantees a visible yard around each house.
    for check_y in y..=y + 1 {
        for check_x in x..=x + 1 {
            let Some(cell) = grid.cell(check_x as u16, check_y as u16) else {
                return false;
            };
            if !matches!(cell, MapCell::Grass) {
                return false;
            }
        }
    }
    // The canonical residential and service facades put the door in the SE
    // quadrant of the southwest block, so the frontage block is below the
    // west (not east) half of the building.
    let door = (x, y + 2);
    transport_cells(grid)
        .into_iter()
        .any(|(road_x, road_y)| (road_x - door.0).abs() + (road_y - door.1).abs() <= 6)
}

fn prepare_home_spawn(grid: &mut GeneratedGrid) {
    let (home_x, home_y) = grid.home_cell();
    let house_x = home_x;
    let house_y = home_y.saturating_sub(3);
    let nearest_backbone = transport_cells(grid)
        .into_iter()
        .filter(|&(x, y)| {
            (x - i32::from(home_x))
                .abs()
                .max((y - i32::from(home_y)).abs())
                > 3
        })
        .min_by_key(|&(x, y)| (x - i32::from(home_x)).abs() + (y - i32::from(home_y)).abs());

    // Keep the address screen green and garden-like. Only the compact door
    // walk uses path blocks; natural blockers become safe lawn instead of a
    // featureless seven-by-seven gray apron.
    for y in house_y.saturating_sub(1)..=(home_y + 2).min(grid.height - 1) {
        for x in house_x.saturating_sub(2)..=(house_x + 3).min(grid.width - 1) {
            if matches!(
                grid.cell(x, y),
                Some(
                    MapCell::Grass
                        | MapCell::Lawn
                        | MapCell::Clearing
                        | MapCell::Park
                        | MapCell::Flowers
                        | MapCell::Tree
                        | MapCell::ParkTree
                        | MapCell::SmallTree
                        | MapCell::SmallTreeSouth
                        | MapCell::Boulder
                )
            ) {
                set_cell(grid, i32::from(x), i32::from(y), MapCell::Lawn);
            }
        }
    }

    if home_y >= 3 && home_x + 1 < grid.width {
        for dy in 0..2 {
            for dx in 0..2 {
                set_cell(
                    grid,
                    i32::from(house_x) + dx,
                    i32::from(house_y) + dy,
                    MapCell::Building,
                );
            }
        }
    }

    // The exact address is the spawn, directly in line with the real door.
    // This narrow three-block walk leaves the rest of the yard planted.
    for y in house_y.saturating_add(2)..=home_y.min(grid.height - 1) {
        set_cell(grid, i32::from(home_x), i32::from(y), MapCell::Trail);
    }
    if house_x >= 2 {
        for y in [house_y + 1, house_y + 2] {
            if grid.cell(house_x - 2, y) == Some(MapCell::Lawn) {
                set_cell(grid, i32::from(house_x - 2), i32::from(y), MapCell::Flowers);
            }
        }
    }
    if house_x + 2 < grid.width && grid.cell(house_x + 2, home_y + 1) == Some(MapCell::Lawn) {
        set_cell(
            grid,
            i32::from(house_x + 2),
            i32::from(home_y + 1),
            MapCell::SmallTree,
        );
    }
    if house_x >= 2
        && home_y + 2 < grid.height
        && grid.cell(house_x - 2, home_y + 2) == Some(MapCell::Lawn)
    {
        set_cell(
            grid,
            i32::from(house_x - 2),
            i32::from(home_y + 2),
            MapCell::Boulder,
        );
    }

    // The house can overlap the direct center-to-spine route that was planned
    // before structures existed. Reconnect the front yard after stamping the
    // house, allowing the pathfinder to bend around the facade and any grove.
    let home = (i32::from(home_x), i32::from(home_y));
    if let Some(backbone) = nearest_backbone {
        carve_path(grid, home, backbone);
    }
}

pub fn repair_walkable_connectivity(grid: &mut GeneratedGrid) {
    let home = grid.home_cell();
    // Each pass repairs or absorbs one disconnected component. A fixed
    // sixteen-pass budget was sufficient for the original 64x64 room, but
    // left dozens of small shoreline/relief pockets behind in city-scale
    // 96-128 block maps. Scale the bounded budget with map area while keeping
    // the existing generous H3 minimum for heavily clipped faces.
    let area_repairs = (usize::from(grid.width) * usize::from(grid.height))
        .div_ceil(256)
        .clamp(16, 256);
    let maximum_repairs = if grid.source.h3.is_some() {
        area_repairs.max(64)
    } else {
        area_repairs
    };
    for _ in 0..maximum_repairs {
        let reached = reachable_walkable_cells(grid, home);
        let mut unseen = vec![true; grid.cells.len()];
        let mut disconnected = Vec::<usize>::new();
        for start in 0..grid.cells.len() {
            if !unseen[start] || reached[start] || !is_walkable_cell(grid.cells[start]) {
                continue;
            }
            let mut component = Vec::new();
            let mut frontier = vec![start];
            unseen[start] = false;
            while let Some(index) = frontier.pop() {
                component.push(index);
                let x = index % usize::from(grid.width);
                let y = index / usize::from(grid.width);
                for (next_x, next_y) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if next_x >= usize::from(grid.width) || next_y >= usize::from(grid.height) {
                        continue;
                    }
                    let next = next_y * usize::from(grid.width) + next_x;
                    if unseen[next] && !reached[next] && is_walkable_cell(grid.cells[next]) {
                        unseen[next] = false;
                        frontier.push(next);
                    }
                }
            }
            let external_mapped_exit = component.iter().all(|&index| {
                let x = index % usize::from(grid.width);
                let y = index / usize::from(grid.width);
                let on_boundary = x == 0
                    || y == 0
                    || x + 1 == usize::from(grid.width)
                    || y + 1 == usize::from(grid.height);
                on_boundary
                    && matches!(
                        grid.cells[index],
                        MapCell::Street | MapCell::Road | MapCell::MajorRoad
                    )
            });
            if external_mapped_exit {
                continue;
            }
            if component.len() > disconnected.len() {
                disconnected = component;
            }
        }
        if disconnected.is_empty() {
            return;
        }

        let width = usize::from(grid.width);
        let mut nearest = None::<(usize, usize, usize)>;
        for &from in &disconnected {
            let from_x = from % width;
            let from_y = from / width;
            for (to, connected) in reached.iter().copied().enumerate() {
                if !connected {
                    continue;
                }
                let to_x = to % width;
                let to_y = to / width;
                let distance = from_x.abs_diff(to_x) + from_y.abs_diff(to_y);
                if nearest.is_none_or(|(best, ..)| distance < best) {
                    nearest = Some((distance, from, to));
                }
            }
        }
        let Some((_, from, to)) = nearest else {
            return;
        };
        let from = ((from % width) as i32, (from / width) as i32);
        let to = ((to % width) as i32, (to / width) as i32);
        let Some(path) = shortest_land_path(grid, from, to) else {
            // A land pocket fully enclosed by water/buildings is not a useful
            // gameplay room. Absorb it into the nearest forest mass instead
            // of leaving unreachable walkable terrain in the map.
            for index in disconnected {
                if !matches!(
                    grid.cells[index],
                    MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
                ) && ((grid.source.h3.is_some() && is_walkable_cell(grid.cells[index]))
                    || matches!(
                        grid.cells[index],
                        MapCell::Grass
                            | MapCell::Lawn
                            | MapCell::Flowers
                            | MapCell::IceFloor
                            | MapCell::RockFloor
                    ))
                {
                    grid.cells[index] = MapCell::Tree;
                }
            }
            continue;
        };
        for (x, y) in path {
            if !matches!(
                grid.cell(x as u16, y as u16),
                Some(
                    MapCell::Water
                        | MapCell::Lawn
                        | MapCell::IceFloor
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Trail
                        | MapCell::Street
                        | MapCell::Road
                        | MapCell::MajorRoad
                        | MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                        | MapCell::Bench
                        | MapCell::TrashCan
                        | MapCell::Fountain
                )
            ) {
                set_cell(grid, x, y, MapCell::Clearing);
            }
        }
    }
}

fn reachable_walkable_cells(grid: &GeneratedGrid, start: (u16, u16)) -> Vec<bool> {
    let mut reached = vec![false; grid.cells.len()];
    let start = usize::from(start.1) * usize::from(grid.width) + usize::from(start.0);
    if !is_walkable_cell(grid.cells[start]) {
        return reached;
    }
    let mut frontier = std::collections::VecDeque::from([start]);
    reached[start] = true;
    while let Some(index) = frontier.pop_front() {
        let x = index % usize::from(grid.width);
        let y = index / usize::from(grid.width);
        for (next_x, next_y) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if next_x >= usize::from(grid.width) || next_y >= usize::from(grid.height) {
                continue;
            }
            let next = next_y * usize::from(grid.width) + next_x;
            if !reached[next] && is_walkable_cell(grid.cells[next]) {
                reached[next] = true;
                frontier.push_back(next);
            }
        }
    }
    reached
}

fn is_walkable_cell(cell: MapCell) -> bool {
    !matches!(
        cell,
        MapCell::H3Void
            | MapCell::Building
            | MapCell::PokecenterNorthWest
            | MapCell::PokecenterNorthEast
            | MapCell::PokecenterSouthWest
            | MapCell::PokecenterSouthEast
            | MapCell::MartNorthWest
            | MapCell::MartNorthEast
            | MapCell::MartSouthWest
            | MapCell::MartSouthEast
            | MapCell::Water
            | MapCell::WaterAccessEast
            | MapCell::WaterAccessWest
            | MapCell::WaterAccessSouth
            | MapCell::Tree
            | MapCell::ParkTree
            | MapCell::SmallTree
            | MapCell::SmallTreeSouth
            | MapCell::Boulder
            | MapCell::IceBoulder
            | MapCell::Bench
            | MapCell::TrashCan
            | MapCell::Fountain
            | MapCell::GroundSign
            | MapCell::FenceNorthWest
            | MapCell::FenceNorth
            | MapCell::FenceNorthEast
            | MapCell::FenceWest
            | MapCell::FenceEast
            | MapCell::FenceSouthWest
            | MapCell::FenceSouth
            | MapCell::FenceSouthEast
            | MapCell::LedgeWest
            | MapCell::LedgeMiddle
            | MapCell::LedgeEast
            | MapCell::CliffNorthWest
            | MapCell::CliffNorth
            | MapCell::CliffNorthEast
            | MapCell::CliffWest
            | MapCell::CliffCenter
            | MapCell::CliffEast
            | MapCell::CliffSouthWest
            | MapCell::CliffSouth
            | MapCell::CliffSouthEast
            | MapCell::CliffInnerSouthWest
            | MapCell::CliffInnerSouthEast
    )
}

fn set_cell(grid: &mut GeneratedGrid, x: i32, y: i32, cell: MapCell) {
    if x >= 0 && y >= 0 && x < i32::from(grid.width) && y < i32::from(grid.height) {
        let index = y as usize * usize::from(grid.width) + x as usize;
        if matches!(
            grid.cells[index],
            MapCell::Street | MapCell::Road | MapCell::MajorRoad
        ) && !matches!(cell, MapCell::Street | MapCell::Road | MapCell::MajorRoad)
        {
            return;
        }
        grid.cells[index] = cell;
    }
}

fn building_block(grid: &GeneratedGrid, x: u16, y: u16) -> u16 {
    let (min_x, min_y, max_x, max_y) = building_component_bounds(grid, x, y);
    let local_x = usize::from(x - min_x);
    let local_y = usize::from(y - min_y);
    match (max_x - min_x + 1, max_y - min_y + 1) {
        // Goldenrod Department Store, exact canonical four-storey stamp.
        (3, 4) => [
            [0x18, 0x1f, 0x19],
            [0x27, 0x23, 0x28],
            [0x27, 0x23, 0x28],
            [0x10, 0x17, 0x33],
        ][local_y][local_x],
        // Complete Goldenrod Radio Tower: $21/$22/$22 form the narrow antenna
        // mast above the formerly-used 2x3 lower facade.
        (2, 6) => match (local_x, local_y) {
            (0, 0) => 0x21,
            (0, 1 | 2) => 0x22,
            (0, 3) => 0x25,
            (1, 3) => 0x26,
            (0, 4) => 0x29,
            (1, 4) => 0x2a,
            (0, 5) => 0x2d,
            (1, 5) => 0x2e,
            _ => 0x02,
        },
        _ if traditional_house_district(grid, min_x, min_y) => [
            [
                u16::from(crate::GENERATED_TRADITIONAL_HOUSE_NORTH_WEST_METATILE),
                u16::from(crate::GENERATED_TRADITIONAL_HOUSE_NORTH_EAST_METATILE),
            ],
            [
                u16::from(crate::GENERATED_TRADITIONAL_HOUSE_SOUTH_WEST_METATILE),
                u16::from(crate::GENERATED_TRADITIONAL_HOUSE_SOUTH_EAST_METATILE),
            ],
        ][local_y.min(1)][local_x.min(1)],
        // Ordinary unsigned Azalea residence. The service facades remain
        // separately typed, so this can never create another Center or Mart.
        _ => [[0x18, 0x19], [0x16, 0x1e]][local_y.min(1)][local_x.min(1)],
    }
}

fn traditional_house_district(grid: &GeneratedGrid, origin_x: u16, origin_y: u16) -> bool {
    let nearby_building_cells = (origin_y.saturating_sub(9)..=(origin_y + 9).min(grid.height - 1))
        .flat_map(|y| {
            (origin_x.saturating_sub(9)..=(origin_x + 9).min(grid.width - 1)).map(move |x| (x, y))
        })
        .filter(|&(x, y)| grid.cell(x, y) == Some(MapCell::Building))
        .count();
    let district_seed = hash(origin_x / 8, origin_y / 8);
    if urban_intensity(grid) < 2 {
        // Rural/town maps lean Ecruteak: two traditional precincts for every
        // one modern neighborhood, all chosen as coherent eight-block zones.
        district_seed % 3 != 0
    } else {
        // Downtown keeps Goldenrod silhouettes; quieter outer clusters switch
        // only when the finished local building density is genuinely lower.
        nearby_building_cells <= 8 && district_seed.is_multiple_of(2)
    }
}

fn building_component_bounds(grid: &GeneratedGrid, x: u16, y: u16) -> (u16, u16, u16, u16) {
    let mut seen = std::collections::BTreeSet::from([(x, y)]);
    let mut frontier = vec![(x, y)];
    while let Some((cell_x, cell_y)) = frontier.pop() {
        for (next_x, next_y) in [
            (cell_x.checked_sub(1), Some(cell_y)),
            (
                cell_x.checked_add(1).filter(|&next| next < grid.width),
                Some(cell_y),
            ),
            (Some(cell_x), cell_y.checked_sub(1)),
            (
                Some(cell_x),
                cell_y.checked_add(1).filter(|&next| next < grid.height),
            ),
        ] {
            let (Some(next_x), Some(next_y)) = (next_x, next_y) else {
                continue;
            };
            if grid.cell(next_x, next_y) == Some(MapCell::Building) && seen.insert((next_x, next_y))
            {
                frontier.push((next_x, next_y));
            }
        }
    }
    (
        seen.iter().map(|&(cell_x, _)| cell_x).min().unwrap_or(x),
        seen.iter().map(|&(_, cell_y)| cell_y).min().unwrap_or(y),
        seen.iter().map(|&(cell_x, _)| cell_x).max().unwrap_or(x),
        seen.iter().map(|&(_, cell_y)| cell_y).max().unwrap_or(y),
    )
}

fn park_block(_grid: &GeneratedGrid, x: u16, y: u16) -> u16 {
    if hash(x, y).is_multiple_of(3) {
        u16::from(crate::GENERATED_PARK_LONG_GRASS_METATILE)
    } else {
        0x03
    }
}

fn flower_block(x: u16, y: u16) -> u16 {
    if hash(x, y).is_multiple_of(2) {
        u16::from(crate::GENERATED_PARK_FLOWER_BED_METATILE)
    } else {
        0x04
    }
}

fn place_flowers(grid: &mut GeneratedGrid) {
    let width = usize::from(grid.width);
    let mut houses = terrain_components(grid, MapCell::Building)
        .into_iter()
        .filter(|component| component.len() == 4)
        .filter_map(|component| {
            let x = component.iter().map(|index| index % width).min()? as u16;
            let y = component.iter().map(|index| index / width).min()? as u16;
            Some((x, y))
        })
        .collect::<Vec<_>>();
    houses.sort_unstable_by_key(|&(x, y)| (y, x));

    let mut flower_cells = 0;
    for (house_index, (house_x, house_y)) in houses.into_iter().enumerate() {
        if house_index % 2 != 0 || flower_cells >= 24 {
            continue;
        }
        let bed_x = if house_index % 4 == 0 {
            i32::from(house_x) - 2
        } else {
            i32::from(house_x) + 2
        };
        let bed_y = i32::from(house_y) + 1;
        for offset in 0..2 {
            let x = bed_x;
            let y = bed_y + offset;
            if x <= 0
                || y <= 0
                || x + 1 >= i32::from(grid.width)
                || y + 1 >= i32::from(grid.height)
                || !matches!(
                    grid.cell(x as u16, y as u16),
                    Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
                )
            {
                continue;
            }
            set_cell(grid, x, y, MapCell::Flowers);
            flower_cells += 1;
            soften_flower_edge(grid, x, y);
        }
    }

    // Add a few compact two-block beds along the principal route. Hashing
    // chooses whole beds; it never scatters single flowers or blocks a door.
    for y in 2..grid.height - 2 {
        for x in 2..grid.width - 3 {
            if flower_cells >= 34 || (flower_cells >= 12 && hash(x, y) % 19 != 0) {
                continue;
            }
            let beside_path = [grid.cell(x, y - 1), grid.cell(x, y + 1)]
                .into_iter()
                .any(|cell| cell == Some(MapCell::Trail));
            if !beside_path
                || !matches!(grid.cell(x, y), Some(MapCell::Grass | MapCell::Lawn))
                || !matches!(grid.cell(x + 1, y), Some(MapCell::Grass | MapCell::Lawn))
                || near_house_frontage(grid, x, y)
                || near_house_frontage(grid, x + 1, y)
            {
                continue;
            }
            for flower_x in [x, x + 1] {
                set_cell(grid, i32::from(flower_x), i32::from(y), MapCell::Flowers);
                flower_cells += 1;
                soften_flower_edge(grid, i32::from(flower_x), i32::from(y));
            }
        }
    }

    let mut garden_candidates = Vec::new();
    for y in 3..grid.height.saturating_sub(4) {
        for x in 3..grid.width.saturating_sub(4) {
            if (0..2).all(|dy| {
                (0..2).all(|dx| {
                    matches!(
                        grid.cell(x + dx, y + dy),
                        Some(MapCell::Grass | MapCell::Lawn)
                    ) && !near_house_frontage(grid, x + dx, y + dy)
                })
            }) {
                garden_candidates.push((hash(x, y), x, y));
            }
        }
    }
    garden_candidates.sort_unstable();
    for (_, x, y) in garden_candidates {
        if flower_cells >= 32 {
            break;
        }
        if !(0..2).all(|dy| {
            (0..2).all(|dx| {
                matches!(
                    grid.cell(x + dx, y + dy),
                    Some(MapCell::Grass | MapCell::Lawn)
                )
            })
        }) {
            continue;
        }
        for dy in 0..2 {
            for dx in 0..2 {
                set_cell(grid, i32::from(x + dx), i32::from(y + dy), MapCell::Flowers);
                flower_cells += 1;
                soften_flower_edge(grid, i32::from(x + dx), i32::from(y + dy));
            }
        }
    }

    // Signed service facades get paired planters on their outside edges. The
    // landscaping separates the Center and Mart visually while leaving each
    // real door, sign, bench, can, and route connection untouched.
    for ((origin_x, origin_y), side) in pokecenter_origin(grid)
        .into_iter()
        .map(|origin| (origin, -1_i32))
        .chain(mart_origin(grid).into_iter().map(|origin| (origin, 2_i32)))
    {
        let bed_x = i32::from(origin_x) + side;
        for bed_y in [i32::from(origin_y) + 1, i32::from(origin_y) + 2] {
            if bed_x > 0
                && bed_y > 0
                && matches!(
                    grid.cell(bed_x as u16, bed_y as u16),
                    Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
                )
                && !near_transport(grid, bed_x as u16, bed_y as u16, 0)
            {
                set_cell(grid, bed_x, bed_y, MapCell::Flowers);
                soften_flower_edge(grid, bed_x, bed_y);
            }
        }
    }
}

fn soften_flower_edge(grid: &mut GeneratedGrid, x: i32, y: i32) {
    for (lawn_x, lawn_y) in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
        if lawn_x >= 0
            && lawn_y >= 0
            && grid.cell(lawn_x as u16, lawn_y as u16) == Some(MapCell::Grass)
        {
            set_cell(grid, lawn_x, lawn_y, MapCell::Lawn);
        }
    }
}

fn near_house_frontage(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    for check_y in y.saturating_sub(3)..=(y + 1).min(grid.height - 1) {
        for check_x in x.saturating_sub(1)..=(x + 1).min(grid.width - 1) {
            if grid.cell(check_x, check_y) != Some(MapCell::Building) {
                continue;
            }
            let south_is_clear = check_y + 1 >= grid.height
                || grid.cell(check_x, check_y + 1) != Some(MapCell::Building);
            if south_is_clear
                && i32::from(check_x).abs_diff(i32::from(x)) <= 1
                && y >= check_y
                && y <= check_y.saturating_add(2)
            {
                return true;
            }
        }
    }
    false
}

fn tree_block(_grid: &GeneratedGrid, _x: u16, _y: u16) -> u16 {
    0x05
}

fn h3_void_block(x: u16, y: u16) -> u16 {
    let district_x = x / 6;
    let district_y = y / 6;
    let seed = hash(district_x, district_y);
    let local = (x % 6, y % 6);
    let rock_shape = if seed & 1 == 0 {
        matches!(local, (1, 1) | (3, 0) | (4, 2) | (2, 3) | (0, 4))
    } else {
        matches!(local, (0, 1) | (2, 0) | (4, 1) | (3, 3) | (1, 4))
    };
    if seed.is_multiple_of(3) && rock_shape {
        0x0a
    } else if hash(x, y).is_multiple_of(7) {
        u16::from(crate::GENERATED_PARK_TREE_METATILE)
    } else {
        0x05
    }
}

fn plant_tree_belts(grid: &mut GeneratedGrid) {
    let mut trees = Vec::new();
    for y in 1..grid.height - 1 {
        for x in 1..grid.width - 1 {
            if grid.cell(x, y) != Some(MapCell::Grass) {
                continue;
            }
            let borders_wild = [
                grid.cell(x - 1, y),
                grid.cell(x + 1, y),
                grid.cell(x, y - 1),
                grid.cell(x, y + 1),
            ]
            .into_iter()
            .any(|cell| cell == Some(MapCell::Park));
            if borders_wild && !near_transport(grid, x, y, 2) && hash(x, y) % 5 != 0 {
                trees.push((x, y));
            }
        }
    }
    for (x, y) in trees {
        set_cell(grid, i32::from(x), i32::from(y), MapCell::Tree);
    }
    // Two-block perimeter canopy gives the generated square a deliberate
    // route boundary. Existing road exits remain open and therefore line up
    // as explicit gates rather than disappearing into scenery.
    for y in 0..grid.height {
        for x in 0..grid.width {
            let at_perimeter = x < 2 || y < 2 || x + 2 >= grid.width || y + 2 >= grid.height;
            if at_perimeter
                && grid.cell(x, y) == Some(MapCell::Grass)
                && !near_transport(grid, x, y, 2)
            {
                set_cell(grid, i32::from(x), i32::from(y), MapCell::Tree);
            }
        }
    }

    let target = grid.cells.len() * 30 / 100;
    let mut tree_cells = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Tree)
        .count();
    let stable_grid = StableGrid::for_grid(grid).ok();
    let mut belt_candidates = Vec::new();
    for y in 5..grid.height.saturating_sub(5) {
        for x in 5..grid.width.saturating_sub(5) {
            if grid.cell(x, y) != Some(MapCell::Grass) {
                continue;
            }
            let seed = stable_grid
                .and_then(|addressing| addressing.cell(x, y))
                .map_or_else(
                    || u64::from(hash(x, y)),
                    |cell| cell.stable_hash(0x4752_4f56_45),
                );
            // World-addressed blue-noise minima remove the old five-cell row
            // lattice. Each accepted center is surrounded by a deterministic
            // exclusion zone, so overlapping lobes cannot reform a ruler-
            // straight forest bar when adjacent map windows are generated.
            let is_local_minimum = stable_grid
                .and_then(|addressing| addressing.cell(x, y))
                .is_none_or(|center| {
                    (-4_i64..=4).all(|dy| {
                        (-4_i64..=4).all(|dx| {
                            (dx == 0 && dy == 0)
                                || seed <= center.offset(dx, dy).stable_hash(0x4752_4f56_45)
                        })
                    })
                });
            if is_local_minimum {
                belt_candidates.push((seed, x, y));
            }
        }
    }
    belt_candidates.sort_unstable();
    for (seed, center_x, center_y) in belt_candidates {
        if tree_cells >= target {
            break;
        }
        // A real route uses compact, overlapping grove lobes to shape rooms.
        // The old 13-21 by 3 rectangles produced unmistakable horizontal bars
        // across a regional render.  Keep each authored lobe compact, shift
        // successive rows independently, and bite deterministic cells from
        // the rim.  Overlap between lobes still builds substantial forest,
        // but no single primitive is a ruler-straight wall.
        let radius_x = 3 + i32::try_from(seed % 3).unwrap_or(0);
        let radius_y = 3 + i32::try_from(seed.rotate_left(11) % 2).unwrap_or(0);
        for dy in -radius_y..=radius_y {
            let row_seed = seed.rotate_left((dy + radius_y) as u32 * 7 + 3);
            let row_shift = i32::try_from(row_seed % 3).unwrap_or(0) - 1;
            for dx in -radius_x..=radius_x {
                let normalized = dx.abs() * (radius_y + 1) + dy.abs() * (radius_x + 1);
                let rim = normalized >= radius_x * radius_y + radius_x / 2;
                let bitten = rim
                    && hash(
                        (i32::from(center_x) + dx).max(0) as u16,
                        (i32::from(center_y) + dy).max(0) as u16,
                    ) % 4
                        == 0;
                let x = i32::from(center_x) + dx + row_shift;
                let y = i32::from(center_y) + dy;
                if normalized <= radius_x * radius_y + radius_x
                    && !bitten
                    && x >= 0
                    && y >= 0
                    && x < i32::from(grid.width)
                    && y < i32::from(grid.height)
                    && grid.cell(x as u16, y as u16) == Some(MapCell::Grass)
                    && tree_site_is_clear(grid, x as u16, y as u16)
                {
                    set_cell(grid, x, y, MapCell::Tree);
                    tree_cells += 1;
                }
            }
        }
    }
    let mut accents = Vec::<(u16, u16)>::new();
    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            if accents.len() >= 26
                || hash(x, y) % 7 != 0
                || grid.cell(x, y) != Some(MapCell::Grass)
                || near_transport(grid, x, y, 1)
                || accents.iter().any(|&(tree_x, tree_y)| {
                    i32::from(tree_x).abs_diff(i32::from(x)) < 4
                        && i32::from(tree_y).abs_diff(i32::from(y)) < 3
                })
            {
                continue;
            }
            let tree_north = grid.cell(x, y.saturating_sub(1)) == Some(MapCell::Tree);
            let tree_south = grid.cell(x, y + 1) == Some(MapCell::Tree);
            if tree_north || tree_south {
                set_cell(
                    grid,
                    i32::from(x),
                    i32::from(y),
                    if tree_north {
                        MapCell::SmallTreeSouth
                    } else {
                        MapCell::SmallTree
                    },
                );
                accents.push((x, y));
            }
        }
    }
}

fn near_transport(grid: &GeneratedGrid, x: u16, y: u16, radius: i32) -> bool {
    for check_y in i32::from(y) - radius..=i32::from(y) + radius {
        for check_x in i32::from(x) - radius..=i32::from(x) + radius {
            if check_x < 0
                || check_y < 0
                || check_x >= i32::from(grid.width)
                || check_y >= i32::from(grid.height)
            {
                continue;
            }
            if matches!(
                grid.cell(check_x as u16, check_y as u16),
                Some(MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad)
            ) {
                return true;
            }
        }
    }
    false
}

fn tree_site_is_clear(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    for check_y in i32::from(y) - 1..=i32::from(y) + 1 {
        for check_x in i32::from(x) - 1..=i32::from(x) + 1 {
            if check_x < 0
                || check_y < 0
                || check_x >= i32::from(grid.width)
                || check_y >= i32::from(grid.height)
            {
                continue;
            }
            if matches!(
                grid.cell(check_x as u16, check_y as u16),
                Some(
                    MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                        | MapCell::Flowers
                        | MapCell::Clearing
                        | MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Trail
                        | MapCell::Street
                        | MapCell::Road
                        | MapCell::MajorRoad
                )
            ) {
                return false;
            }
        }
    }
    true
}

fn place_route_furniture(grid: &mut GeneratedGrid) {
    let mut row_counts = vec![0usize; usize::from(grid.height)];
    let mut column_counts = vec![0usize; usize::from(grid.width)];
    for (index, cell) in grid.cells.iter().copied().enumerate() {
        if !matches!(
            cell,
            MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
        ) {
            continue;
        }
        row_counts[index / usize::from(grid.width)] += 1;
        column_counts[index % usize::from(grid.width)] += 1;
    }
    let (road_y, row_count) = row_counts
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|&(index, count)| (count, std::cmp::Reverse(index)))
        .unwrap_or((usize::from(grid.height / 2), 0));
    let (road_x, column_count) = column_counts
        .iter()
        .copied()
        .enumerate()
        .max_by_key(|&(index, count)| (count, std::cmp::Reverse(index)))
        .unwrap_or((usize::from(grid.width / 2), 0));

    let mut fence_runs = 0;
    if row_count >= column_count {
        for side in [-2, 2] {
            let fence_y = road_y as i32 + side;
            if fence_y <= 1 || fence_y + 1 >= i32::from(grid.height) {
                continue;
            }
            for start_x in 3..grid.width.saturating_sub(9) {
                if fence_runs >= 3 || hash(start_x, fence_y as u16) % 7 != 0 {
                    continue;
                }
                let clear = (0..7).all(|offset| {
                    matches!(
                        grid.cell(start_x + offset, fence_y as u16),
                        Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
                    ) && !near_house_frontage(grid, start_x + offset, fence_y as u16)
                });
                let outward_y = side.signum();
                let nature_support = (0..7)
                    .filter(|offset| {
                        (1..=2).any(|distance| {
                            route_fence_nature(
                                grid,
                                i32::from(start_x + *offset),
                                fence_y + outward_y * distance,
                            )
                        })
                    })
                    .count();
                if !clear || nature_support < 4 {
                    continue;
                }
                for offset in 0..7 {
                    let cell = if side < 0 {
                        match offset {
                            0 => MapCell::FenceSouthWest,
                            6 => MapCell::FenceSouthEast,
                            _ => MapCell::FenceSouth,
                        }
                    } else {
                        match offset {
                            0 => MapCell::FenceNorthWest,
                            6 => MapCell::FenceNorthEast,
                            _ => MapCell::FenceNorth,
                        }
                    };
                    set_cell(grid, i32::from(start_x + offset), fence_y, cell);
                    let verge_y = fence_y - side.signum();
                    if grid.cell(start_x + offset, verge_y as u16) == Some(MapCell::Grass) {
                        set_cell(
                            grid,
                            i32::from(start_x + offset),
                            verge_y,
                            MapCell::Clearing,
                        );
                    }
                }
                fence_runs += 1;
            }
        }
    } else {
        for side in [-2, 2] {
            let fence_x = road_x as i32 + side;
            if fence_x <= 1 || fence_x + 1 >= i32::from(grid.width) {
                continue;
            }
            for start_y in 3..grid.height.saturating_sub(9) {
                if fence_runs >= 3 || hash(fence_x as u16, start_y) % 7 != 0 {
                    continue;
                }
                let clear = (0..7).all(|offset| {
                    matches!(
                        grid.cell(fence_x as u16, start_y + offset),
                        Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
                    )
                });
                let outward_x = side.signum();
                let nature_support = (0..7)
                    .filter(|offset| {
                        (1..=2).any(|distance| {
                            route_fence_nature(
                                grid,
                                fence_x + outward_x * distance,
                                i32::from(start_y + *offset),
                            )
                        })
                    })
                    .count();
                if !clear || nature_support < 4 {
                    continue;
                }
                for offset in 0..7 {
                    let cell = if side < 0 {
                        match offset {
                            0 => MapCell::FenceNorthEast,
                            6 => MapCell::FenceSouthEast,
                            _ => MapCell::FenceEast,
                        }
                    } else {
                        match offset {
                            0 => MapCell::FenceNorthWest,
                            6 => MapCell::FenceSouthWest,
                            _ => MapCell::FenceWest,
                        }
                    };
                    set_cell(grid, fence_x, i32::from(start_y + offset), cell);
                }
                fence_runs += 1;
            }
        }
    }

    let mut signs = 0;
    for y in 2..grid.height - 2 {
        for x in 2..grid.width - 2 {
            if signs >= 4
                || hash(y, x) % 29 != 0
                || !matches!(grid.cell(x, y), Some(MapCell::Grass | MapCell::Clearing))
                || near_house_frontage(grid, x, y)
                || !h3_protected_cell_fits(grid, x, y)
            {
                continue;
            }
            let beside_route = [
                grid.cell(x - 1, y),
                grid.cell(x + 1, y),
                grid.cell(x, y - 1),
                grid.cell(x, y + 1),
            ]
            .into_iter()
            .any(|cell| {
                matches!(
                    cell,
                    Some(MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad)
                )
            });
            if beside_route {
                set_cell(grid, i32::from(x), i32::from(y), MapCell::GroundSign);
                signs += 1;
            }
        }
    }

    accent_road_junctions(grid);
}

/// Frame mapped crossings at the scale of one Game Boy viewport. A bare
/// one-block road cross expands to a visually dominant four-runtime-tile
/// square; two diagonal flower beds give it deliberate edges while leaving
/// every approach and turning cell open. Alternating the diagonal prevents a
/// repeated municipal-grid stamp across the whole generated town.
fn accent_road_junctions(grid: &mut GeneratedGrid) {
    let mapped = |cell: Option<MapCell>| {
        matches!(
            cell,
            Some(MapCell::Street | MapCell::Road | MapCell::MajorRoad)
        )
    };
    let mut junctions = Vec::new();
    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            if mapped(grid.cell(x, y))
                && mapped(grid.cell(x - 1, y))
                && mapped(grid.cell(x + 1, y))
                && mapped(grid.cell(x, y - 1))
                && mapped(grid.cell(x, y + 1))
            {
                junctions.push((hash(x, y), x, y));
            }
        }
    }
    junctions.sort_unstable();

    for (seed, x, y) in junctions {
        let diagonals = if seed & 1 == 0 {
            [(-1_i32, -1_i32), (1, 1)]
        } else {
            [(1_i32, -1_i32), (-1, 1)]
        };
        for (dx, dy) in diagonals {
            let flower_x = (i32::from(x) + dx) as u16;
            let flower_y = (i32::from(y) + dy) as u16;
            if matches!(
                grid.cell(flower_x, flower_y),
                Some(MapCell::Grass | MapCell::Lawn)
            ) && !near_house_frontage(grid, flower_x, flower_y)
            {
                set_cell(
                    grid,
                    i32::from(flower_x),
                    i32::from(flower_y),
                    MapCell::Flowers,
                );
            }
        }
    }
}

fn route_fence_nature(grid: &GeneratedGrid, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 || x >= i32::from(grid.width) || y >= i32::from(grid.height) {
        return false;
    }
    matches!(
        grid.cell(x as u16, y as u16),
        Some(
            MapCell::Tree
                | MapCell::ParkTree
                | MapCell::SmallTree
                | MapCell::SmallTreeSouth
                | MapCell::Park
                | MapCell::Boulder
        )
    )
}

const PLATEAU_SALT: u64 = 0x504c_4154_4541_555f;

type PlateauSpecification = (Vec<PlateauTemplate>, u16, u16);

fn place_landmark_features(grid: &mut GeneratedGrid) -> Result<()> {
    // Crystal's Johto contour family is a proper south-facing shape grammar,
    // not a bag of interchangeable rock pictures. Ninety-degree rotation
    // would require east/west-facing art which johto_modern does not contain;
    // variety instead comes from complete width/depth variants, mirrored
    // stepped shoulders, off-center stair flights, and a cell-stable rotation
    // of the two landmark targets. Every candidate still uses the canonical
    // $6a-$72 contour family and the real Slowpoke Well stair flight.
    let minimum_dimension = grid.width.min(grid.height);
    let stable_grid = StableGrid::for_grid(grid)?;
    let Some(stable_anchor) = stable_grid.cell(grid.width / 2, grid.height / 2) else {
        bail!("plateau planner has no stable center cell");
    };
    let stable_seed = stable_anchor.stable_hash(PLATEAU_SALT);
    let specifications = plateau_specifications(grid, minimum_dimension, stable_seed);
    let mut reservations = Vec::<(i32, i32, i32, i32)>::new();

    for (formation_index, (templates, target_x, target_y)) in specifications.into_iter().enumerate()
    {
        let mut selected = None;
        // The first template is the cell's large signature. Later entries are
        // complete, progressively more compact contours: a water-heavy or
        // downtown face may step down without ever stamping a truncated cliff.
        for template in templates {
            let stamp = plateau_stamp(template);
            let stamp_height = stamp.height();
            let stamp_width = stamp.width();
            let mut proposals = Vec::new();
            for y in 4..grid.height.saturating_sub(stamp_height + 3) {
                for x in 4..grid.width.saturating_sub(stamp_width + 3) {
                    let reservation = (
                        i32::from(x) - 2,
                        i32::from(y) - 2,
                        i32::from(x + stamp_width) + 1,
                        i32::from(y + stamp_height) + 1,
                    );
                    if reservations.iter().any(|&other| {
                        rectangles_overlap(
                            (
                                reservation.0 - 3,
                                reservation.1 - 3,
                                reservation.2 + 3,
                                reservation.3 + 3,
                            ),
                            other,
                        )
                    }) || !plateau_site_is_clear(grid, x, y, stamp_width, stamp_height)
                    {
                        continue;
                    }

                    let entry = (
                        i32::from(x + stamp.stair_x),
                        i32::from(y + stamp.stair_y + 1),
                    );
                    let Some(path) = scenic_connector(grid, entry, 14) else {
                        continue;
                    };
                    if path.iter().any(|&(path_x, path_y)| {
                        (i32::from(x)..i32::from(x + stamp_width)).contains(&path_x)
                            && (i32::from(y)..i32::from(y + stamp_height)).contains(&path_y)
                    }) {
                        continue;
                    }
                    let distance = i32::from(x + stamp_width / 2).abs_diff(i32::from(target_x))
                        + i32::from(y + stamp_height / 2).abs_diff(i32::from(target_y)) * 2;
                    let tree_overlap = stamp
                        .cells
                        .iter()
                        .enumerate()
                        .flat_map(|(dy, row)| {
                            row.iter().enumerate().map(move |(dx, cell)| (dx, dy, cell))
                        })
                        .filter(|(dx, dy, cell)| {
                            cell.is_some()
                                && matches!(
                                    grid.cell(x + *dx as u16, y + *dy as u16),
                                    Some(
                                        MapCell::Tree
                                            | MapCell::ParkTree
                                            | MapCell::SmallTree
                                            | MapCell::SmallTreeSouth
                                    )
                                )
                        })
                        .count();
                    let stable_tie = stable_grid
                        .cell(x, y)
                        .map(|cell| {
                            cell.stable_hash(
                                PLATEAU_SALT.wrapping_add((formation_index as u64).rotate_left(17)),
                            )
                        })
                        .unwrap_or(u64::MAX);
                    proposals.push((
                        std::cmp::Reverse(tree_overlap),
                        distance,
                        new_trail_steps(grid, &path),
                        path_turns(&path),
                        stable_tie,
                        x,
                        y,
                        path,
                        reservation,
                    ));
                }
            }
            proposals.sort_by_key(|proposal| {
                (proposal.0, proposal.1, proposal.2, proposal.3, proposal.4)
            });
            if let Some((_, _, _, _, _, x, y, path, reservation)) = proposals.into_iter().next() {
                selected = Some((stamp, x, y, path, reservation));
                break;
            }
        }
        let Some((stamp, x, y, path, reservation)) = selected else {
            continue;
        };
        commit_scenic_trail(grid, path);
        for (dy, row) in stamp.cells.iter().enumerate() {
            for (dx, &cell) in row.iter().enumerate() {
                if let Some(cell) = cell {
                    set_cell(
                        grid,
                        i32::from(x) + dx as i32,
                        i32::from(y) + dy as i32,
                        cell,
                    );
                }
            }
        }
        reservations.push(reservation);
    }

    Ok(())
}

fn plateau_specifications(
    grid: &GeneratedGrid,
    minimum_dimension: u16,
    stable_seed: u64,
) -> Vec<PlateauSpecification> {
    if grid.source.h3.is_some() && minimum_dimension >= 56 {
        let expanded = [
            PlateauTemplate::ExpandedWideLeft,
            PlateauTemplate::ExpandedDeep,
            PlateauTemplate::ExpandedWideRight,
            PlateauTemplate::ExpandedGrand,
        ];
        let stepped = [
            PlateauTemplate::SteppedLeft,
            PlateauTemplate::SteppedDeepRight,
            PlateauTemplate::SteppedRight,
            PlateauTemplate::SteppedDeepLeft,
            PlateauTemplate::SteppedGrand,
        ];
        let expanded_index = stable_seed as usize % expanded.len();
        let stepped_index = stable_seed.rotate_left(23) as usize % stepped.len();
        let target_pattern = [
            [(3_u16, 3_u16), (1, 1)],
            [(3, 1), (1, 3)],
            [(1, 3), (3, 1)],
            [(1, 1), (3, 3)],
        ][stable_seed.rotate_right(11) as usize % 4];
        return vec![
            (
                vec![expanded[expanded_index], PlateauTemplate::ExpandedCompact],
                grid.width * target_pattern[0].0 / 4,
                grid.height * target_pattern[0].1 / 4,
            ),
            (
                vec![stepped[stepped_index], PlateauTemplate::SteppedCompact],
                grid.width * target_pattern[1].0 / 4,
                grid.height * target_pattern[1].1 / 4,
            ),
        ];
    }
    if minimum_dimension >= 96 {
        vec![
            (
                vec![PlateauTemplate::ExpandedGrand],
                grid.width / 5,
                grid.height / 4,
            ),
            (
                vec![PlateauTemplate::SteppedGrand],
                grid.width * 4 / 5,
                grid.height / 3,
            ),
            (
                vec![PlateauTemplate::ExpandedGrand],
                grid.width / 4,
                grid.height * 3 / 4,
            ),
            (
                vec![PlateauTemplate::SteppedGrand],
                grid.width * 3 / 4,
                grid.height * 4 / 5,
            ),
        ]
    } else if minimum_dimension >= 56 {
        vec![
            (
                vec![PlateauTemplate::ExpandedCompact],
                grid.width * 3 / 4,
                grid.height * 3 / 4,
            ),
            (
                vec![PlateauTemplate::SteppedCompact],
                grid.width / 4,
                grid.height / 4,
            ),
        ]
    } else if minimum_dimension >= 40 {
        vec![(
            vec![PlateauTemplate::ExpandedCompact],
            grid.width * 2 / 3,
            grid.height * 2 / 3,
        )]
    } else {
        Vec::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlateauTemplate {
    ExpandedCompact,
    ExpandedWideLeft,
    ExpandedWideRight,
    ExpandedDeep,
    ExpandedGrand,
    SteppedCompact,
    SteppedLeft,
    SteppedRight,
    SteppedDeepLeft,
    SteppedDeepRight,
    SteppedGrand,
}

const PLATEAU_CATALOG: [PlateauTemplate; 11] = [
    PlateauTemplate::ExpandedCompact,
    PlateauTemplate::ExpandedWideLeft,
    PlateauTemplate::ExpandedWideRight,
    PlateauTemplate::ExpandedDeep,
    PlateauTemplate::ExpandedGrand,
    PlateauTemplate::SteppedCompact,
    PlateauTemplate::SteppedLeft,
    PlateauTemplate::SteppedRight,
    PlateauTemplate::SteppedDeepLeft,
    PlateauTemplate::SteppedDeepRight,
    PlateauTemplate::SteppedGrand,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlateauContourKind {
    Expanded,
    Stepped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlateauContourSignature {
    pub kind: PlateauContourKind,
    pub width: u16,
    pub height: u16,
    pub cliff_cells: usize,
    pub stairs: usize,
    pub inner_west: usize,
    pub inner_east: usize,
}

#[derive(Debug)]
struct PlateauStamp {
    cells: Vec<Vec<Option<MapCell>>>,
    stair_x: u16,
    stair_y: u16,
}

impl PlateauStamp {
    fn width(&self) -> u16 {
        self.cells.first().map_or(0, |row| row.len() as u16)
    }

    fn height(&self) -> u16 {
        self.cells.len() as u16
    }
}

fn plateau_stamp(template: PlateauTemplate) -> PlateauStamp {
    match template {
        PlateauTemplate::ExpandedCompact => expanded_plateau_stamp(7, 2, 3),
        PlateauTemplate::ExpandedWideLeft => expanded_plateau_stamp(9, 3, 3),
        PlateauTemplate::ExpandedWideRight => expanded_plateau_stamp(9, 3, 5),
        PlateauTemplate::ExpandedDeep => expanded_plateau_stamp(7, 4, 3),
        PlateauTemplate::ExpandedGrand => expanded_plateau_stamp(11, 3, 5),
        PlateauTemplate::SteppedCompact => stepped_plateau_stamp(7, 1, 2, 3, 1),
        PlateauTemplate::SteppedLeft => stepped_plateau_stamp(9, 1, 1, 5, 1),
        PlateauTemplate::SteppedRight => stepped_plateau_stamp(9, 1, 3, 5, 1),
        PlateauTemplate::SteppedDeepLeft => stepped_plateau_stamp(9, 2, 1, 5, 2),
        PlateauTemplate::SteppedDeepRight => stepped_plateau_stamp(9, 2, 3, 5, 2),
        PlateauTemplate::SteppedGrand => stepped_plateau_stamp(11, 2, 2, 7, 1),
    }
}

/// Recognize only whole contours emitted by the catalog above. Auditing uses
/// this exact normalized-cell comparison so a severed corner, missing stair,
/// or ad-hoc blend of otherwise real cliff tiles cannot masquerade as one of
/// the authored formations.
pub(crate) fn canonical_plateau_contours(grid: &GeneratedGrid) -> Vec<PlateauContourSignature> {
    let cliff_indices = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| is_cliff_cell(*cell).then_some(index))
        .collect::<std::collections::BTreeSet<_>>();
    indexed_components(&cliff_indices, grid.width, grid.height)
        .into_iter()
        .filter_map(|component| {
            let min_x = component
                .iter()
                .map(|index| index % usize::from(grid.width))
                .min()?;
            let max_x = component
                .iter()
                .map(|index| index % usize::from(grid.width))
                .max()?;
            let min_y = component
                .iter()
                .map(|index| index / usize::from(grid.width))
                .min()?;
            let max_y = component
                .iter()
                .map(|index| index / usize::from(grid.width))
                .max()?;
            let width = max_x - min_x + 1;
            let height = max_y - min_y + 1;
            let mut normalized = vec![vec![None; width]; height];
            for &index in &component {
                let x = index % usize::from(grid.width) - min_x;
                let y = index / usize::from(grid.width) - min_y;
                normalized[y][x] = Some(grid.cells[index]);
            }
            let template = PLATEAU_CATALOG
                .into_iter()
                .find(|template| plateau_stamp(*template).cells == normalized)?;
            Some(PlateauContourSignature {
                kind: match template {
                    PlateauTemplate::ExpandedCompact
                    | PlateauTemplate::ExpandedWideLeft
                    | PlateauTemplate::ExpandedWideRight
                    | PlateauTemplate::ExpandedDeep
                    | PlateauTemplate::ExpandedGrand => PlateauContourKind::Expanded,
                    PlateauTemplate::SteppedCompact
                    | PlateauTemplate::SteppedLeft
                    | PlateauTemplate::SteppedRight
                    | PlateauTemplate::SteppedDeepLeft
                    | PlateauTemplate::SteppedDeepRight
                    | PlateauTemplate::SteppedGrand => PlateauContourKind::Stepped,
                },
                width: width as u16,
                height: height as u16,
                cliff_cells: component.len(),
                stairs: component
                    .iter()
                    .filter(|index| grid.cells[**index] == MapCell::CliffStairs)
                    .count(),
                inner_west: component
                    .iter()
                    .filter(|index| grid.cells[**index] == MapCell::CliffInnerSouthWest)
                    .count(),
                inner_east: component
                    .iter()
                    .filter(|index| grid.cells[**index] == MapCell::CliffInnerSouthEast)
                    .count(),
            })
        })
        .collect()
}

fn is_cliff_cell(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::CliffNorthWest
            | MapCell::CliffNorth
            | MapCell::CliffNorthEast
            | MapCell::CliffWest
            | MapCell::CliffCenter
            | MapCell::CliffEast
            | MapCell::CliffSouthWest
            | MapCell::CliffSouth
            | MapCell::CliffSouthEast
            | MapCell::CliffInnerSouthWest
            | MapCell::CliffInnerSouthEast
            | MapCell::CliffStairs
    )
}

fn expanded_plateau_stamp(width: u16, middle_rows: u16, stair_x: u16) -> PlateauStamp {
    debug_assert!(width >= 3 && stair_x > 0 && stair_x + 1 < width);
    let mut cells = vec![plateau_edge_row(
        width,
        MapCell::CliffNorthWest,
        MapCell::CliffNorth,
        MapCell::CliffNorthEast,
    )];
    cells.extend((0..middle_rows).map(|_| {
        plateau_edge_row(
            width,
            MapCell::CliffWest,
            MapCell::CliffCenter,
            MapCell::CliffEast,
        )
    }));
    let mut south = plateau_edge_row(
        width,
        MapCell::CliffSouthWest,
        MapCell::CliffSouth,
        MapCell::CliffSouthEast,
    );
    south[usize::from(stair_x)] = Some(MapCell::CliffStairs);
    cells.push(south);
    PlateauStamp {
        stair_x,
        stair_y: middle_rows + 1,
        cells,
    }
}

fn stepped_plateau_stamp(
    width: u16,
    upper_middle_rows: u16,
    stem_offset: u16,
    stem_width: u16,
    stem_middle_rows: u16,
) -> PlateauStamp {
    debug_assert!(
        width >= 5 && stem_width >= 3 && stem_offset > 0 && stem_offset + stem_width < width
    );
    let stem_east = stem_offset + stem_width - 1;
    let stair_x = stem_offset + stem_width / 2;
    let mut cells = vec![plateau_edge_row(
        width,
        MapCell::CliffNorthWest,
        MapCell::CliffNorth,
        MapCell::CliffNorthEast,
    )];
    cells.extend((0..upper_middle_rows).map(|_| {
        plateau_edge_row(
            width,
            MapCell::CliffWest,
            MapCell::CliffCenter,
            MapCell::CliffEast,
        )
    }));

    let mut shoulder = plateau_edge_row(
        width,
        MapCell::CliffSouthWest,
        MapCell::CliffSouth,
        MapCell::CliffSouthEast,
    );
    shoulder[usize::from(stem_offset)] = Some(MapCell::CliffInnerSouthWest);
    shoulder[usize::from(stem_east)] = Some(MapCell::CliffInnerSouthEast);
    for x in stem_offset + 1..stem_east {
        shoulder[usize::from(x)] = Some(MapCell::CliffCenter);
    }
    cells.push(shoulder);

    cells.extend((0..stem_middle_rows).map(|_| {
        plateau_inset_row(
            width,
            stem_offset,
            stem_width,
            MapCell::CliffWest,
            MapCell::CliffCenter,
            MapCell::CliffEast,
        )
    }));
    let mut south = plateau_inset_row(
        width,
        stem_offset,
        stem_width,
        MapCell::CliffSouthWest,
        MapCell::CliffSouth,
        MapCell::CliffSouthEast,
    );
    south[usize::from(stair_x)] = Some(MapCell::CliffStairs);
    cells.push(south);
    PlateauStamp {
        stair_x,
        stair_y: upper_middle_rows + stem_middle_rows + 2,
        cells,
    }
}

fn plateau_edge_row(
    width: u16,
    west: MapCell,
    middle: MapCell,
    east: MapCell,
) -> Vec<Option<MapCell>> {
    (0..width)
        .map(|x| {
            Some(match x {
                0 => west,
                value if value + 1 == width => east,
                _ => middle,
            })
        })
        .collect()
}

fn plateau_inset_row(
    width: u16,
    offset: u16,
    inset_width: u16,
    west: MapCell,
    middle: MapCell,
    east: MapCell,
) -> Vec<Option<MapCell>> {
    (0..width)
        .map(|x| {
            if !(offset..offset + inset_width).contains(&x) {
                return None;
            }
            Some(match x - offset {
                0 => west,
                value if value + 1 == inset_width => east,
                _ => middle,
            })
        })
        .collect()
}

fn plateau_site_is_clear(grid: &GeneratedGrid, x: u16, y: u16, width: u16, height: u16) -> bool {
    if !h3_stamp_fits(grid, i32::from(x), i32::from(y), width, height, 3) {
        return false;
    }
    let left = x - 1;
    let right = x + width;
    let top = y - 1;
    let bottom = y + height;
    let (home_x, home_y) = grid.home_cell();
    let house_y = home_y.saturating_sub(3);
    let future_home_yard = (
        i32::from(home_x.saturating_sub(2)),
        i32::from(house_y.saturating_sub(1)),
        i32::from((home_x + 3).min(grid.width - 1)),
        i32::from((home_y + 2).min(grid.height - 1)),
    );
    if grid.source.h3.is_some()
        && rectangles_overlap(
            (
                i32::from(left),
                i32::from(top),
                i32::from(right),
                i32::from(bottom),
            ),
            future_home_yard,
        )
    {
        return false;
    }
    (top..=bottom).all(|check_y| {
        (left..=right).all(|check_x| {
            let inside_stamp =
                (x..x + width).contains(&check_x) && (y..y + height).contains(&check_y);
            let terrain_is_replaceable = if inside_stamp {
                matches!(
                    grid.cell(check_x, check_y),
                    Some(
                        MapCell::Grass
                            | MapCell::Lawn
                            | MapCell::Clearing
                            | MapCell::Tree
                            | MapCell::ParkTree
                            | MapCell::SmallTree
                            | MapCell::SmallTreeSouth
                    )
                )
            } else {
                matches!(
                    grid.cell(check_x, check_y),
                    Some(
                        MapCell::Grass
                            | MapCell::Lawn
                            | MapCell::Clearing
                            | MapCell::Tree
                            | MapCell::ParkTree
                            | MapCell::SmallTree
                            | MapCell::SmallTreeSouth
                    )
                )
            };
            terrain_is_replaceable
                && !near_house_frontage(grid, check_x, check_y)
                && !near_transport(grid, check_x, check_y, 0)
        })
    })
}

type LedgeSpecification = (Vec<(i32, i32, u16)>, u16, u16);

/// Author several complete south-facing ledges after every ordinary decoration
/// pass has settled. Each run reserves a full approach and landing apron, so a
/// later tree, rock, flower, or encounter patch cannot invalidate the runtime
/// hop lanes. The open side margins also leave a walk-around route: a ledge is
/// useful one-way terrain, never a wall that partitions the neighborhood.
fn place_large_ledge_runs(grid: &mut GeneratedGrid) -> Result<()> {
    const LEDGE_SALT: u64 = 0x4c45_4447_455f_5255;
    const MAX_SEARCH_ATTEMPTS: usize = 256;
    let minimum_dimension = grid.width.min(grid.height);
    let specifications: Vec<LedgeSpecification> =
        if grid.source.h3.is_some() && minimum_dimension >= 56 {
            // A hex raster has roughly three quarters of the buildable area of a
            // square room. Three substantial, differently sized runs retain the
            // relief language without forcing a terrace into a clipped corner.
            vec![
                (vec![(0, 0, 7)], grid.width / 2, grid.height * 3 / 4),
                (vec![(0, 0, 8)], grid.width / 3, grid.height / 2),
                (vec![(0, 0, 9)], grid.width * 2 / 3, grid.height / 3),
            ]
        } else if minimum_dimension >= 96 {
            vec![
                (
                    vec![(0, 0, 9), (2, 4, 7)],
                    grid.width * 3 / 4,
                    grid.height / 5,
                ),
                (
                    vec![(2, 0, 8), (0, 4, 6)],
                    grid.width / 4,
                    grid.height * 2 / 5,
                ),
                (
                    vec![(0, 0, 10), (2, 4, 8)],
                    grid.width * 2 / 3,
                    grid.height * 3 / 5,
                ),
                (vec![(0, 0, 7)], grid.width / 3, grid.height * 4 / 5),
                (vec![(0, 0, 9)], grid.width * 4 / 5, grid.height * 7 / 8),
            ]
        } else if minimum_dimension >= 56 {
            vec![
                (
                    vec![(0, 0, 9), (2, 4, 7)],
                    grid.width * 3 / 4,
                    grid.height / 4,
                ),
                (vec![(2, 0, 8), (0, 4, 6)], grid.width / 4, grid.height / 2),
                (vec![(0, 0, 7)], grid.width / 3, grid.height * 7 / 8),
            ]
        } else if minimum_dimension >= 40 {
            vec![
                (
                    vec![(0, 0, 8), (2, 4, 6)],
                    grid.width * 2 / 3,
                    grid.height / 3,
                ),
                (vec![(0, 0, 7)], grid.width / 3, grid.height * 3 / 4),
            ]
        } else {
            // A 24-32-block water-heavy crop may not contain a safe seven-block
            // approach/landing reservation at all. Large ledges are a default
            // 64-block room feature; never fail a small geographic crop or stamp a
            // truncated imitation merely to satisfy a fixed count.
            Vec::new()
        };
    let stable_grid = StableGrid::for_grid(grid)?;
    let expected_runs = specifications
        .iter()
        .map(|(runs, _, _)| runs.len())
        .sum::<usize>();
    let original_cells = grid.cells.clone();
    let mut choices = vec![0_usize; specifications.len()];
    let mut deepest_formation = 0;
    let mut search_attempts = 0;
    let mut reservations = Vec::<(i32, i32, i32, i32)>::new();

    // A committed run changes both replaceable terrain and the nearest route
    // graph. Search the short ranked candidate sequence as one authored plan:
    // if a preferred early terrace consumes the only safe later apron, rewind
    // the cells (not the potentially huge OSM source) and try its next site.
    'search: loop {
        search_attempts += 1;
        if search_attempts > MAX_SEARCH_ATTEMPTS {
            bail!(
                "could not place all {expected_runs} complete ledge runs after {MAX_SEARCH_ATTEMPTS} deterministic candidate plans; formation {} has no safe approach, landing, and bypass",
                deepest_formation + 1
            );
        }
        grid.cells.clone_from(&original_cells);
        reservations.clear();
        for (formation_index, (runs, target_x, target_y)) in specifications.iter().enumerate() {
            deepest_formation = deepest_formation.max(formation_index);
            let formation_width = runs
                .iter()
                .map(|(dx, _, length)| dx + i32::from(*length))
                .max()
                .unwrap_or(0);
            let formation_height = runs.iter().map(|(_, dy, _)| *dy).max().unwrap_or(0) + 1;
            if formation_width <= 0
                || formation_height <= 0
                || formation_width + 7 >= i32::from(grid.width)
                || formation_height + 7 >= i32::from(grid.height)
            {
                bail!("map is too small for a complete authored ledge formation");
            }
            let mut proposals = Vec::new();
            for y in 5..i32::from(grid.height) - formation_height - 3 {
                for x in 4..i32::from(grid.width) - formation_width - 3 {
                    let left = x - 3;
                    let right = x + formation_width + 2;
                    let top = y - 3;
                    let bottom = y + formation_height + 2;
                    if reservations.iter().any(|&other| {
                        rectangles_overlap((left - 3, top - 3, right + 3, bottom + 3), other)
                    }) || runs.iter().any(|&(dx, dy, length)| {
                        !ledge_reservation_is_clear(grid, (x + dx) as u16, (y + dy) as u16, length)
                    }) {
                        continue;
                    }

                    let primary = runs[0];
                    let entry = (x + primary.0 + i32::from(primary.2 / 2), y + primary.1 - 1);
                    let Some(path) = scenic_connector(grid, entry, 14) else {
                        continue;
                    };
                    if path.iter().any(|&(path_x, path_y)| {
                        runs.iter().any(|&(dx, dy, length)| {
                            path_y == y + dy
                                && (x + dx..x + dx + i32::from(length)).contains(&path_x)
                        })
                    }) {
                        continue;
                    }
                    let midpoint_x = x + formation_width / 2;
                    let midpoint_y = y + formation_height / 2;
                    let distance = midpoint_x.abs_diff(i32::from(*target_x))
                        + midpoint_y.abs_diff(i32::from(*target_y)) * 2;
                    let stable_tie = stable_grid
                        .cell(x as u16, y as u16)
                        .map(|cell| cell.stable_hash(LEDGE_SALT + formation_index as u64))
                        .unwrap_or(u64::MAX);
                    proposals.push((
                        distance,
                        new_trail_steps(grid, &path),
                        path_turns(&path),
                        stable_tie,
                        x as u16,
                        y as u16,
                        path,
                        (left, top, right, bottom),
                    ));
                }
            }
            proposals.sort_by_key(|proposal| (proposal.0, proposal.1, proposal.2, proposal.3));
            let Some((_, _, _, seed, x, y, path, reservation)) =
                proposals.into_iter().nth(choices[formation_index])
            else {
                if advance_ledge_choice(&mut choices, formation_index) {
                    continue 'search;
                }
                bail!(
                    "could not place all {expected_runs} complete ledge runs; formation {} has no safe approach, landing, and bypass",
                    deepest_formation + 1
                );
            };
            // Lawn is visually identical to the ordinary route green but is not a
            // candidate for later dense-tree belts. Preserve all runtime approach
            // and landing quadrants before stamping the one-way collision row.
            for &(dx, dy, length) in runs {
                let run_x = (i32::from(x) + dx) as u16;
                let run_y = (i32::from(y) + dy) as u16;
                for bypass_x in [run_x - 1, run_x + length] {
                    if matches!(
                        grid.cell(bypass_x, run_y),
                        Some(
                            MapCell::Grass
                                | MapCell::Clearing
                                | MapCell::Flowers
                                | MapCell::Park
                                | MapCell::Tree
                                | MapCell::ParkTree
                                | MapCell::SmallTree
                                | MapCell::SmallTreeSouth
                        )
                    ) {
                        set_cell(grid, i32::from(bypass_x), i32::from(run_y), MapCell::Lawn);
                    }
                }
                for offset in 0..length {
                    for apron_y in [run_y - 1, run_y + 1] {
                        if matches!(
                            grid.cell(run_x + offset, apron_y),
                            Some(
                                MapCell::Grass
                                    | MapCell::Clearing
                                    | MapCell::Flowers
                                    | MapCell::Park
                                    | MapCell::Tree
                                    | MapCell::ParkTree
                                    | MapCell::SmallTree
                                    | MapCell::SmallTreeSouth
                            )
                        ) {
                            set_cell(
                                grid,
                                i32::from(run_x + offset),
                                i32::from(apron_y),
                                MapCell::Lawn,
                            );
                        }
                    }
                    set_cell(
                        grid,
                        i32::from(run_x + offset),
                        i32::from(run_y),
                        match offset {
                            0 => MapCell::LedgeWest,
                            value if value + 1 == length => MapCell::LedgeEast,
                            _ => MapCell::LedgeMiddle,
                        },
                    );
                }
                // $52/$53 permit lateral hops as well as the southward jump. A
                // one-cell bypass on the ledge row is insufficient when a late
                // grove borders its north or south neighbor, because that leaves
                // the landing strip boxed into an unreachable pocket. Reserve the
                // complete three-cell-high side columns around both end caps.
                for side_x in [run_x - 1, run_x + length] {
                    for side_y in run_y - 1..=run_y + 1 {
                        if matches!(
                            grid.cell(side_x, side_y),
                            Some(
                                MapCell::Grass
                                    | MapCell::Lawn
                                    | MapCell::Clearing
                                    | MapCell::Flowers
                                    | MapCell::Park
                                    | MapCell::Tree
                                    | MapCell::ParkTree
                                    | MapCell::SmallTree
                                    | MapCell::SmallTreeSouth
                                    | MapCell::Boulder
                            )
                        ) {
                            set_cell(grid, i32::from(side_x), i32::from(side_y), MapCell::Lawn);
                        }
                    }
                }
                set_cell(
                    grid,
                    i32::from(run_x + length / 2),
                    i32::from(run_y - 1),
                    MapCell::Trail,
                );
            }
            commit_scenic_trail(grid, path);

            place_ledge_rock_skirts(
                grid,
                x,
                y,
                formation_width as u16,
                formation_height as u16,
                seed,
            );
            reservations.push(reservation);
        }
        return Ok(());
    }
}

fn advance_ledge_choice(choices: &mut [usize], failed_formation: usize) -> bool {
    let Some(previous) = failed_formation.checked_sub(1) else {
        return false;
    };
    choices[previous] = choices[previous].saturating_add(1);
    for choice in &mut choices[previous + 1..] {
        *choice = 0;
    }
    true
}

fn place_ledge_rock_skirts(
    grid: &mut GeneratedGrid,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    seed: u64,
) {
    let left = i32::from(x);
    let top = i32::from(y);
    let right = left + i32::from(width) - 1;
    let bottom = top + i32::from(height) - 1;
    let candidates = [
        [(left - 3, top), (left - 2, top + 1), (left - 3, top + 2)],
        [
            (right + 3, bottom - 2),
            (right + 2, bottom - 1),
            (right + 3, bottom),
        ],
        [
            (left - 3, bottom - 2),
            (left - 2, bottom - 1),
            (left - 3, bottom),
        ],
        [(right + 3, top), (right + 2, top + 1), (right + 3, top + 2)],
    ];
    let start = seed as usize % candidates.len();
    for index in 0..candidates.len() {
        let cluster = candidates[(start + index) % candidates.len()];
        if cluster.iter().all(|&(rock_x, rock_y)| {
            rock_x >= 0
                && rock_y >= 0
                && rock_x < i32::from(grid.width)
                && rock_y < i32::from(grid.height)
                && matches!(
                    grid.cell(rock_x as u16, rock_y as u16),
                    Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing | MapCell::Flowers)
                )
                && !near_transport(grid, rock_x as u16, rock_y as u16, 0)
                && !near_house_frontage(grid, rock_x as u16, rock_y as u16)
        }) {
            for (rock_x, rock_y) in cluster {
                set_cell(grid, rock_x, rock_y, MapCell::Boulder);
            }
            break;
        }
    }
}

fn ledge_reservation_is_clear(grid: &GeneratedGrid, x: u16, y: u16, length: u16) -> bool {
    if !h3_stamp_fits(grid, i32::from(x), i32::from(y), length, 1, 3) {
        return false;
    }
    let left = i32::from(x) - 1;
    let right = i32::from(x + length);
    let top = i32::from(y) - 1;
    let bottom = i32::from(y) + 1;
    let replaceable = (top..=bottom).all(|check_y| {
        (left..=right).all(|check_x| {
            matches!(
                grid.cell(check_x as u16, check_y as u16),
                Some(
                    MapCell::Grass
                        | MapCell::Lawn
                        | MapCell::Clearing
                        | MapCell::Flowers
                        | MapCell::Tree
                        | MapCell::ParkTree
                        | MapCell::SmallTree
                        | MapCell::SmallTreeSouth
                )
            ) && !near_house_frontage(grid, check_x as u16, check_y as u16)
        })
    });
    let middle = x + length / 2;
    replaceable
        && matches!(
            grid.cell(middle, y - 1),
            Some(
                MapCell::Grass
                    | MapCell::Lawn
                    | MapCell::Clearing
                    | MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth
            )
        )
}

fn h3_stamp_fits(
    grid: &GeneratedGrid,
    x: i32,
    y: i32,
    width: u16,
    height: u16,
    clearance: u16,
) -> bool {
    grid.source.h3.as_ref().is_none_or(|plan| {
        plan.raster_footprint_fits(x, y, width, height, clearance, grid.width, grid.height)
            .expect("H3 plan was validated before generation")
    })
}

fn h3_protected_cell_fits(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    h3_stamp_fits(grid, i32::from(x), i32::from(y), 1, 1, 3)
}

fn rectangles_overlap(first: (i32, i32, i32, i32), second: (i32, i32, i32, i32)) -> bool {
    let (first_left, first_top, first_right, first_bottom) = first;
    let (second_left, second_top, second_right, second_bottom) = second;
    first_left <= second_right
        && first_right >= second_left
        && first_top <= second_bottom
        && first_bottom >= second_top
}

fn scenic_connector(
    grid: &GeneratedGrid,
    entry: (i32, i32),
    max_new_steps: usize,
) -> Option<Vec<(i32, i32)>> {
    let mut route_cells = transport_cells(grid);
    route_cells.sort_unstable_by_key(|&(x, y)| (x - entry.0).abs() + (y - entry.1).abs());
    route_cells
        .into_iter()
        .take(32)
        .filter_map(|route| shortest_land_path(grid, entry, route))
        .filter(|path| {
            new_trail_steps(grid, path) <= max_new_steps
                && path_turns(path) <= 2
                && path.iter().all(|&(x, y)| {
                    matches!(
                        grid.cell(x as u16, y as u16),
                        Some(
                            MapCell::Grass
                                | MapCell::Lawn
                                | MapCell::Clearing
                                | MapCell::Tree
                                | MapCell::ParkTree
                                | MapCell::SmallTree
                                | MapCell::SmallTreeSouth
                                | MapCell::Trail
                                | MapCell::Street
                                | MapCell::Road
                                | MapCell::MajorRoad
                        )
                    )
                })
        })
        .min_by_key(|path| (new_trail_steps(grid, path), path_turns(path), path.len()))
}

fn commit_scenic_trail(grid: &mut GeneratedGrid, path: Vec<(i32, i32)>) {
    for (x, y) in path {
        if matches!(
            grid.cell(x as u16, y as u16),
            Some(
                MapCell::Grass
                    | MapCell::Lawn
                    | MapCell::Clearing
                    | MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth
                    | MapCell::Trail
            )
        ) {
            set_cell(grid, x, y, MapCell::Trail);
        }
    }
}

fn new_trail_steps(grid: &GeneratedGrid, path: &[(i32, i32)]) -> usize {
    path.iter()
        .filter(|&&(x, y)| {
            !matches!(
                grid.cell(x as u16, y as u16),
                Some(MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad)
            )
        })
        .count()
}

fn path_turns(path: &[(i32, i32)]) -> usize {
    path.windows(3)
        .filter(|points| {
            let first = (points[1].0 - points[0].0, points[1].1 - points[0].1);
            let second = (points[2].0 - points[1].0, points[2].1 - points[1].1);
            first != second
        })
        .count()
}

fn place_rock_outcrops(grid: &mut GeneratedGrid) {
    let minimum_dimension = grid.width.min(grid.height);
    let target_boulders = if grid.source.h3.is_some() && minimum_dimension >= 56 {
        38
    } else if minimum_dimension >= 56 {
        32
    } else if minimum_dimension >= 40 {
        18
    } else {
        8
    };
    let target_formations = if grid.source.h3.is_some() && minimum_dimension >= 56 {
        7
    } else if minimum_dimension >= 56 {
        6
    } else if minimum_dimension >= 40 {
        3
    } else {
        1
    };
    let mut candidates = Vec::new();
    for y in 4..grid.height.saturating_sub(8) {
        for x in 4..grid.width.saturating_sub(9) {
            if grid.cell(x, y) != Some(MapCell::Grass)
                || near_transport(grid, x, y, 1)
                || near_house_frontage(grid, x, y)
            {
                continue;
            }
            let nature_score = (-4..=4)
                .flat_map(|dy| (-4..=4).map(move |dx| (dx, dy)))
                .filter_map(|(dx, dy)| {
                    let check_x = (i32::from(x) + dx) as u16;
                    let check_y = (i32::from(y) + dy) as u16;
                    let score = match grid.cell(check_x, check_y) {
                        Some(
                            MapCell::CliffNorthWest
                            | MapCell::CliffNorth
                            | MapCell::CliffNorthEast
                            | MapCell::CliffWest
                            | MapCell::CliffCenter
                            | MapCell::CliffEast
                            | MapCell::CliffSouthWest
                            | MapCell::CliffSouth
                            | MapCell::CliffSouthEast
                            | MapCell::CliffInnerSouthWest
                            | MapCell::CliffInnerSouthEast,
                        ) => Some(0_u8),
                        Some(MapCell::Park | MapCell::Water) => Some(1),
                        Some(
                            MapCell::Tree
                            | MapCell::ParkTree
                            | MapCell::SmallTree
                            | MapCell::SmallTreeSouth,
                        ) => Some(2),
                        _ => None,
                    };
                    score.map(|score| (score, dx.abs() + dy.abs()))
                })
                .min();
            if let Some((nature_kind, distance)) = nature_score {
                candidates.push((nature_kind, distance, hash(x, y), x, y));
            }
        }
    }
    candidates.sort_unstable();
    let mut formations = Vec::<(i32, i32, i32, i32)>::new();
    let mut rock_count = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Boulder)
        .count();
    for (_, _, seed, x, y) in candidates {
        if rock_count >= target_boulders || formations.len() >= target_formations {
            break;
        }
        let offsets: &[(u16, u16)] = match formations.len() {
            // Open crescent.
            0 => &[(0, 1), (1, 0), (3, 0), (5, 1), (4, 3), (1, 3)],
            // Branching spur.
            1 => &[(0, 0), (2, 0), (4, 0), (3, 2), (3, 4), (5, 4)],
            // Broken ring with a deliberate walk-in gap.
            2 => &[(0, 1), (1, 0), (3, 0), (5, 1), (5, 3), (3, 4), (0, 3)],
            // Short diagonal ridge.
            3 => &[(0, 0), (2, 0), (2, 2), (4, 2), (5, 4)],
            // Compact chevron; the existing home-yard rock supplies block 29.
            _ => &[(0, 0), (2, 1), (4, 0), (2, 3)],
        };
        let max_dx = offsets.iter().map(|(dx, _)| *dx).max().unwrap_or(0);
        let max_dy = offsets.iter().map(|(_, dy)| *dy).max().unwrap_or(0);
        let bounds = (
            i32::from(x),
            i32::from(y),
            i32::from(x + max_dx),
            i32::from(y + max_dy),
        );
        if formations.iter().any(|&other| {
            rectangles_overlap(
                (bounds.0 - 3, bounds.1 - 3, bounds.2 + 3, bounds.3 + 3),
                other,
            )
        }) {
            continue;
        }
        let positions = offsets
            .iter()
            .map(|&(dx, dy)| {
                if seed & 1 == 0 {
                    (x + dx, y + dy)
                } else {
                    (x + max_dx - dx, y + dy)
                }
            })
            .collect::<Vec<_>>();
        if positions.iter().any(|&(rock_x, rock_y)| {
            !matches!(
                grid.cell(rock_x, rock_y),
                Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
            ) || near_transport(grid, rock_x, rock_y, 1)
                || near_house_frontage(grid, rock_x, rock_y)
                || near_relief(grid, rock_x, rock_y, 2)
        }) {
            continue;
        }
        for (rock_x, rock_y) in positions {
            set_cell(grid, i32::from(rock_x), i32::from(rock_y), MapCell::Boulder);
            rock_count += 1;
        }
        formations.push(bounds);
    }
}

pub(crate) fn near_relief(grid: &GeneratedGrid, x: u16, y: u16, radius: u16) -> bool {
    for check_y in y.saturating_sub(radius)..=(y + radius).min(grid.height - 1) {
        for check_x in x.saturating_sub(radius)..=(x + radius).min(grid.width - 1) {
            if matches!(
                grid.cell(check_x, check_y),
                Some(
                    MapCell::LedgeWest
                        | MapCell::LedgeMiddle
                        | MapCell::LedgeEast
                        | MapCell::CliffNorthWest
                        | MapCell::CliffNorth
                        | MapCell::CliffNorthEast
                        | MapCell::CliffWest
                        | MapCell::CliffCenter
                        | MapCell::CliffEast
                        | MapCell::CliffSouthWest
                        | MapCell::CliffSouth
                        | MapCell::CliffSouthEast
                        | MapCell::CliffInnerSouthWest
                        | MapCell::CliffInnerSouthEast
                        | MapCell::CliffStairs
                )
            ) {
                return true;
            }
        }
    }
    false
}

fn ensure_h3_rock_formations(grid: &mut GeneratedGrid) {
    if grid.source.h3.is_none() {
        return;
    }
    let current = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Boulder)
        .count();
    const TARGET_ROCKS: usize = 38;
    const TARGET_FORMATIONS: usize = 6;
    let current_formations = boulder_formation_count(grid, 2);
    if current >= TARGET_ROCKS && current_formations >= TARGET_FORMATIONS {
        return;
    }
    let wanted = TARGET_ROCKS
        .saturating_sub(current)
        .div_ceil(3)
        .max(TARGET_FORMATIONS.saturating_sub(current_formations))
        .min(10);
    const FORMATIONS: [&[(u16, u16)]; 4] = [
        &[(0, 0), (2, 0), (1, 2)],
        &[(0, 0), (1, 2), (3, 2), (4, 0)],
        &[(0, 1), (2, 0), (4, 1), (2, 3)],
        &[(0, 0), (2, 1), (1, 3)],
    ];
    let existing = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            (*cell == MapCell::Boulder).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    for y in 3..grid.height.saturating_sub(5) {
        for x in 3..grid.width.saturating_sub(5) {
            if !h3_stamp_fits(grid, i32::from(x), i32::from(y), 5, 4, 2)
                || near_transport(grid, x + 1, y + 1, 1)
                || near_house_frontage(grid, x + 1, y + 1)
                || near_relief(grid, x + 1, y + 1, 5)
                || existing.iter().any(|&(rock_x, rock_y)| {
                    rock_x.abs_diff(x + 2) <= 4 && rock_y.abs_diff(y + 2) <= 4
                })
            {
                continue;
            }
            let formation = FORMATIONS[(hash(x, y) as usize) % FORMATIONS.len()];
            if formation.iter().all(|&(dx, dy)| {
                matches!(
                    grid.cell(x + dx, y + dy),
                    Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing | MapCell::Flowers)
                )
            }) {
                candidates.push((hash(x, y), x, y));
            }
        }
    }
    candidates.sort_unstable();
    let mut selected = Vec::<(u16, u16)>::new();
    for (_, x, y) in candidates {
        if selected.len() >= wanted {
            break;
        }
        if selected
            .iter()
            .any(|&(other_x, other_y)| other_x.abs_diff(x) <= 7 && other_y.abs_diff(y) <= 7)
        {
            continue;
        }
        let formation = FORMATIONS[(hash(x, y) as usize) % FORMATIONS.len()];
        for &(dx, dy) in formation {
            set_cell(grid, i32::from(x + dx), i32::from(y + dy), MapCell::Boulder);
        }
        selected.push((x, y));
    }
}

fn prune_h3_isolated_boulders(grid: &mut GeneratedGrid) {
    if grid.source.h3.is_none() {
        return;
    }
    let rocks = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            (*cell == MapCell::Boulder).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect::<Vec<_>>();
    let mut unseen = vec![true; rocks.len()];
    for start in 0..rocks.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut frontier = std::collections::VecDeque::from([start]);
        while let Some(index) = frontier.pop_front() {
            component.push(rocks[index]);
            for next in 0..rocks.len() {
                if unseen[next]
                    && rocks[index].0.abs_diff(rocks[next].0) <= 2
                    && rocks[index].1.abs_diff(rocks[next].1) <= 2
                {
                    unseen[next] = false;
                    frontier.push_back(next);
                }
            }
        }
        if component.len() >= 3 {
            continue;
        }
        for (x, y) in component {
            let joins_canopy = x > 0
                && y > 0
                && x + 1 < grid.width
                && y + 1 < grid.height
                && [
                    grid.cell(x - 1, y),
                    grid.cell(x + 1, y),
                    grid.cell(x, y - 1),
                    grid.cell(x, y + 1),
                ]
                .into_iter()
                .any(|cell| matches!(cell, Some(MapCell::Tree | MapCell::ParkTree)));
            set_cell(
                grid,
                i32::from(x),
                i32::from(y),
                if joins_canopy {
                    MapCell::Tree
                } else {
                    MapCell::Grass
                },
            );
        }
    }
}

fn boulder_formation_count(grid: &GeneratedGrid, radius: u16) -> usize {
    let rocks = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            (*cell == MapCell::Boulder).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect::<Vec<_>>();
    let mut unseen = vec![true; rocks.len()];
    let mut formations = 0;
    for start in 0..rocks.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut frontier = std::collections::VecDeque::from([start]);
        let mut formation_size = 0;
        while let Some(index) = frontier.pop_front() {
            formation_size += 1;
            for next in 0..rocks.len() {
                if unseen[next]
                    && rocks[index].0.abs_diff(rocks[next].0) <= radius
                    && rocks[index].1.abs_diff(rocks[next].1) <= radius
                {
                    unseen[next] = false;
                    frontier.push_back(next);
                }
            }
        }
        if formation_size >= 3 {
            formations += 1;
        }
    }
    formations
}

/// Dense canopy is useful as a blocking room boundary, but a long uninterrupted
/// horizontal course reads as a generated stripe. Break those courses with
/// staggered canonical boulders. Adjacent rows naturally form small rock faces;
/// once the safe rock budget is full, headbutt-tree accents provide the same
/// silhouette break without changing collision or opening unreachable pockets.
fn break_long_canopy_bars(grid: &mut GeneratedGrid) {
    const MAX_CANOPY_RUN: u16 = 12;
    const MAX_BOULDERS: usize = 48;
    let mut boulders = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Boulder)
        .count();
    for y in 2..grid.height.saturating_sub(2) {
        let mut run_start = 2;
        let mut x = 2;
        while x <= grid.width.saturating_sub(2) {
            let canopy = x < grid.width.saturating_sub(2)
                && matches!(grid.cell(x, y), Some(MapCell::Tree | MapCell::ParkTree));
            if canopy {
                x += 1;
                continue;
            }
            let run_length = x.saturating_sub(run_start);
            if run_length > MAX_CANOPY_RUN {
                let spacing = 10 + u16::try_from(hash(run_start, y) % 3).unwrap_or(0);
                for offset in (spacing..run_length).step_by(usize::from(spacing)) {
                    let break_x = run_start + offset;
                    let replacement = if grid.source.h3.is_none() && boulders < MAX_BOULDERS {
                        boulders += 1;
                        MapCell::Boulder
                    } else if y % 2 == 0 {
                        MapCell::SmallTree
                    } else {
                        MapCell::SmallTreeSouth
                    };
                    set_cell(grid, i32::from(break_x), i32::from(y), replacement);
                }
            }
            run_start = x.saturating_add(1);
            x = x.saturating_add(1);
        }
    }
}

/// Author the map at the scale of a Game Boy viewport, not only as a 64x64
/// poster. Whenever the middle of an 8x8 traversal view contains a route but
/// the whole view has no meaningful landmark edge, add one compact flower bed
/// beside that route. Overlapping windows share the same bed, so the pass adds
/// only what is needed to eliminate long empty walking screens.
fn enrich_route_viewports(grid: &mut GeneratedGrid) {
    const VIEW: u16 = 8;
    const MAX_NEW_FLOWERS: usize = 24;
    if grid.width < VIEW || grid.height < VIEW {
        return;
    }
    let mut added = 0;
    loop {
        let mut proposal = None;
        'windows: for top in 0..=grid.height - VIEW {
            for left in 0..=grid.width - VIEW {
                let central_route = (top + 3..top + 5).any(|y| {
                    (left + 3..left + 5).any(|x| {
                        matches!(
                            grid.cell(x, y),
                            Some(
                                MapCell::Trail
                                    | MapCell::Street
                                    | MapCell::Road
                                    | MapCell::MajorRoad
                            )
                        )
                    })
                });
                if !central_route || route_viewport_has_variety(grid, left, top) {
                    continue;
                }
                let mut candidates = Vec::new();
                for y in top..top + VIEW {
                    for x in left..left + VIEW {
                        if !matches!(
                            grid.cell(x, y),
                            Some(
                                MapCell::Trail
                                    | MapCell::Street
                                    | MapCell::Road
                                    | MapCell::MajorRoad
                            )
                        ) {
                            continue;
                        }
                        for ((dx1, dy1), (dx2, dy2)) in [
                            ((0, -2), (1, -2)),
                            ((0, 2), (1, 2)),
                            ((-2, 0), (-2, 1)),
                            ((2, 0), (2, 1)),
                        ] {
                            let first = (i32::from(x) + dx1, i32::from(y) + dy1);
                            let second = (i32::from(x) + dx2, i32::from(y) + dy2);
                            if route_flower_site(grid, first) && route_flower_site(grid, second) {
                                candidates.push((hash(x, y), first, second));
                            }
                        }
                    }
                }
                candidates.sort_unstable();
                if let Some((_, first, second)) = candidates.into_iter().next() {
                    proposal = Some((first, second));
                    break 'windows;
                }
            }
        }
        let Some((first, second)) = proposal else {
            break;
        };
        set_cell(grid, first.0, first.1, MapCell::Flowers);
        set_cell(grid, second.0, second.1, MapCell::Flowers);
        added += 2;
        if added >= MAX_NEW_FLOWERS {
            break;
        }
    }
}

fn route_flower_site(grid: &GeneratedGrid, site: (i32, i32)) -> bool {
    let (x, y) = site;
    x > 0
        && y > 0
        && x + 1 < i32::from(grid.width)
        && y + 1 < i32::from(grid.height)
        && matches!(
            grid.cell(x as u16, y as u16),
            Some(MapCell::Grass | MapCell::Lawn)
        )
        && !near_house_frontage(grid, x as u16, y as u16)
}

fn route_viewport_has_variety(grid: &GeneratedGrid, left: u16, top: u16) -> bool {
    let mut counts = [0_usize; 14];
    for y in top..top + 8 {
        for x in left..left + 8 {
            let family = match grid.cell(x, y) {
                Some(MapCell::H3Void) => None,
                Some(
                    MapCell::Building
                    | MapCell::PokecenterNorthWest
                    | MapCell::PokecenterNorthEast
                    | MapCell::PokecenterSouthWest
                    | MapCell::PokecenterSouthEast
                    | MapCell::MartNorthWest
                    | MapCell::MartNorthEast
                    | MapCell::MartSouthWest
                    | MapCell::MartSouthEast,
                ) => Some(0),
                Some(MapCell::Tree | MapCell::ParkTree) => Some(1),
                Some(MapCell::SmallTree | MapCell::SmallTreeSouth) => Some(2),
                Some(MapCell::Flowers) => Some(3),
                Some(MapCell::Boulder | MapCell::IceBoulder) => Some(4),
                Some(MapCell::GroundSign) => Some(5),
                Some(MapCell::Bench | MapCell::TrashCan | MapCell::Fountain) => Some(6),
                Some(
                    MapCell::FenceNorthWest
                    | MapCell::FenceNorth
                    | MapCell::FenceNorthEast
                    | MapCell::FenceWest
                    | MapCell::FenceEast
                    | MapCell::FenceSouthWest
                    | MapCell::FenceSouth
                    | MapCell::FenceSouthEast,
                ) => Some(7),
                Some(
                    MapCell::LedgeWest
                    | MapCell::LedgeMiddle
                    | MapCell::LedgeEast
                    | MapCell::CliffNorthWest
                    | MapCell::CliffNorth
                    | MapCell::CliffNorthEast
                    | MapCell::CliffWest
                    | MapCell::CliffCenter
                    | MapCell::CliffEast
                    | MapCell::CliffSouthWest
                    | MapCell::CliffSouth
                    | MapCell::CliffSouthEast
                    | MapCell::CliffInnerSouthWest
                    | MapCell::CliffInnerSouthEast
                    | MapCell::CliffStairs,
                ) => Some(8),
                Some(
                    MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth,
                ) => Some(9),
                Some(MapCell::Pitch) => Some(10),
                Some(MapCell::Park) => Some(11),
                Some(
                    MapCell::Clearing | MapCell::Lawn | MapCell::IceFloor | MapCell::RockFloor,
                ) => Some(12),
                Some(MapCell::Grass) => Some(13),
                Some(
                    MapCell::Rail
                    | MapCell::Trail
                    | MapCell::Street
                    | MapCell::Road
                    | MapCell::MajorRoad,
                )
                | None => None,
            };
            if let Some(family) = family {
                counts[family] += 1;
            }
        }
    }
    let minima = [1, 4, 1, 1, 1, 1, 1, 2, 2, 4, 6, 6, 8, 16];
    counts
        .into_iter()
        .zip(minima)
        .filter(|(count, minimum)| count >= minimum)
        .count()
        >= 2
}

fn create_water_access(grid: &mut GeneratedGrid) {
    let transports = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            matches!(
                cell,
                MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
            )
            .then_some((
                (index % usize::from(grid.width)) as i32,
                (index / usize::from(grid.width)) as i32,
            ))
        })
        .collect::<Vec<_>>();
    let Some(principal_water) = terrain_components(grid, MapCell::Water)
        .into_iter()
        .max_by_key(Vec::len)
    else {
        return;
    };
    let mut principal = vec![false; grid.cells.len()];
    for index in principal_water {
        principal[index] = true;
    }
    let mut best: Option<(usize, u16, u16, i32, i32, i32, i32, MapCell)> = None;
    for y in 1..grid.height - 1 {
        for x in 1..grid.width - 1 {
            let index = usize::from(y) * usize::from(grid.width) + usize::from(x);
            if !principal[index] || !h3_protected_cell_fits(grid, x, y) {
                continue;
            }
            for (land_x, land_y, access) in [
                (i32::from(x) - 1, i32::from(y), MapCell::WaterAccessEast),
                (i32::from(x) + 1, i32::from(y), MapCell::WaterAccessWest),
                (i32::from(x), i32::from(y) - 1, MapCell::WaterAccessSouth),
            ] {
                let Some(land) = grid.cell(land_x as u16, land_y as u16) else {
                    continue;
                };
                if matches!(
                    land,
                    MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                        | MapCell::Bench
                        | MapCell::TrashCan
                        | MapCell::Fountain
                ) {
                    continue;
                }
                for &(road_x, road_y) in &transports {
                    let Some(path) = shortest_land_path(grid, (land_x, land_y), (road_x, road_y))
                    else {
                        continue;
                    };
                    if best.is_none_or(|(best_distance, ..)| path.len() < best_distance) {
                        best = Some((path.len(), x, y, land_x, land_y, road_x, road_y, access));
                    }
                }
            }
        }
    }
    let Some((_, water_x, water_y, land_x, land_y, road_x, road_y, access)) = best else {
        return;
    };
    set_cell(grid, i32::from(water_x), i32::from(water_y), access);
    if let Some(path) = shortest_land_path(grid, (land_x, land_y), (road_x, road_y)) {
        for (x, y) in path {
            if !matches!(
                grid.cell(x as u16, y as u16),
                Some(
                    MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                )
            ) {
                set_cell(grid, x, y, MapCell::Trail);
            }
        }
    }
}

fn water_block(grid: &GeneratedGrid, x: u16, y: u16) -> u16 {
    let water = |neighbor_x: i32, neighbor_y: i32| {
        if neighbor_x < 0
            || neighbor_y < 0
            || neighbor_x >= i32::from(grid.width)
            || neighbor_y >= i32::from(grid.height)
        {
            return true;
        }
        matches!(
            grid.cell(neighbor_x as u16, neighbor_y as u16),
            Some(
                MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth
            )
        ) || (grid.source.h3.is_some()
            && grid.cell(neighbor_x as u16, neighbor_y as u16) == Some(MapCell::H3Void))
    };
    let north = water(i32::from(x), i32::from(y) - 1);
    let south = water(i32::from(x), i32::from(y) + 1);
    let west = water(i32::from(x) - 1, i32::from(y));
    let east = water(i32::from(x) + 1, i32::from(y));
    match (north, south, west, east) {
        (false, _, false, _) => 0x54,
        (false, _, _, false) => 0x55,
        (false, _, _, _) => 0x76,
        (_, _, false, _) if south => 0x58,
        (_, _, _, false) if south => 0x59,
        // johto_modern has no south-facing rocky bank. Canonical Johto maps
        // regularly place open water directly against land here; doing so is
        // both collision-correct and avoids a continuous wall of buoy teeth.
        (_, false, _, _) => 0x35,
        (_, _, false, _) => 0x58,
        (_, _, _, false) => 0x59,
        _ => 0x35,
    }
}

fn remove_tiny_areas(grid: &mut GeneratedGrid, cell: MapCell, minimum_size: usize) {
    let mut visited = vec![false; grid.cells.len()];
    for start in 0..grid.cells.len() {
        if visited[start] || grid.cells[start] != cell {
            continue;
        }
        let mut component = Vec::new();
        let mut frontier = vec![start];
        visited[start] = true;
        while let Some(index) = frontier.pop() {
            component.push(index);
            let x = index % usize::from(grid.width);
            let y = index / usize::from(grid.width);
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= usize::from(grid.width) || next_y >= usize::from(grid.height) {
                    continue;
                }
                let next = next_y * usize::from(grid.width) + next_x;
                if !visited[next] && grid.cells[next] == cell {
                    visited[next] = true;
                    frontier.push(next);
                }
            }
        }
        if component.len() < minimum_size {
            for index in component {
                grid.cells[index] = MapCell::Grass;
            }
        }
    }
}

fn fill_water_pinholes(grid: &mut GeneratedGrid) {
    for _ in 0..2 {
        let mut fill = Vec::new();
        for y in 1..grid.height - 1 {
            for x in 1..grid.width - 1 {
                if grid.cell(x, y) != Some(MapCell::Grass) {
                    continue;
                }
                let water_neighbors = [
                    grid.cell(x - 1, y),
                    grid.cell(x + 1, y),
                    grid.cell(x, y - 1),
                    grid.cell(x, y + 1),
                ]
                .into_iter()
                .filter(|cell| *cell == Some(MapCell::Water))
                .count();
                if water_neighbors >= 3 {
                    fill.push((x, y));
                }
            }
        }
        for (x, y) in fill {
            set_cell(grid, i32::from(x), i32::from(y), MapCell::Water);
        }
    }
    let mut visited = vec![false; grid.cells.len()];
    for start in 0..grid.cells.len() {
        if visited[start] || grid.cells[start] != MapCell::Grass {
            continue;
        }
        let mut component = Vec::new();
        let mut frontier = vec![start];
        let mut enclosed_by_water = true;
        visited[start] = true;
        while let Some(index) = frontier.pop() {
            component.push(index);
            let x = index % usize::from(grid.width);
            let y = index / usize::from(grid.width);
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= usize::from(grid.width) || next_y >= usize::from(grid.height) {
                    enclosed_by_water = false;
                    continue;
                }
                let next = next_y * usize::from(grid.width) + next_x;
                match grid.cells[next] {
                    MapCell::Grass if !visited[next] => {
                        visited[next] = true;
                        frontier.push(next);
                    }
                    MapCell::Grass | MapCell::Water => {}
                    _ => enclosed_by_water = false,
                }
            }
        }
        if enclosed_by_water && component.len() <= 16 {
            for index in component {
                grid.cells[index] = MapCell::Water;
            }
        }
    }
}

fn seed_wild_sites(grid: &mut GeneratedGrid) {
    let target = usize::from(grid.width) * usize::from(grid.height) / 8;
    let mut park_cells = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Park)
        .count();
    if park_cells >= target {
        return;
    }
    let existing_sites = terrain_components(grid, MapCell::Park)
        .into_iter()
        .filter(|component| component.len() >= 9)
        .count();
    let mut candidates = Vec::new();
    for y in 3..grid.height.saturating_sub(3) {
        for x in 4..grid.width.saturating_sub(4) {
            if i32::from(x).abs_diff(i32::from(grid.width / 2)) < 8
                && i32::from(y).abs_diff(i32::from(grid.height / 2)) < 8
            {
                continue;
            }
            let clear = (i32::from(y) - 2..=i32::from(y) + 2).all(|check_y| {
                (i32::from(x) - 3..=i32::from(x) + 3).all(|check_x| {
                    grid.cell(check_x as u16, check_y as u16) == Some(MapCell::Grass)
                })
            });
            if clear {
                candidates.push((hash(x / 3, y / 3), x, y));
            }
        }
    }
    candidates.sort_unstable();
    let mut sites = Vec::<(u16, u16)>::new();
    for (seed, center_x, center_y) in candidates {
        if sites.iter().any(|&(x, y)| {
            i32::from(x).abs_diff(i32::from(center_x)) < 16
                && i32::from(y).abs_diff(i32::from(center_y)) < 12
        }) {
            continue;
        }
        let radius_x = 6 + i32::try_from(seed % 3).unwrap_or(0);
        let radius_y = 4 + i32::try_from(seed.rotate_left(7) % 3).unwrap_or(0);
        let denominator = radius_x * radius_x * radius_y * radius_y;
        for dy in -radius_y..=radius_y {
            for dx in -radius_x..=radius_x {
                let ellipse = dx * dx * radius_y * radius_y + dy * dy * radius_x * radius_x;
                let boundary_noise = hash(
                    (i32::from(center_x) + dx) as u16,
                    (i32::from(center_y) + dy) as u16,
                ) % 5;
                if ellipse <= denominator && (ellipse * 5 <= denominator * 4 || boundary_noise != 0)
                {
                    set_cell(
                        grid,
                        i32::from(center_x) + dx,
                        i32::from(center_y) + dy,
                        MapCell::Park,
                    );
                    park_cells += 1;
                }
            }
        }
        sites.push((center_x, center_y));
        if park_cells >= target || existing_sites + sites.len() >= 4 {
            break;
        }
    }
}

fn ensure_h3_wild_sites(grid: &mut GeneratedGrid) {
    if grid.source.h3.is_none() {
        return;
    }
    let mut substantive = terrain_components(grid, MapCell::Park)
        .into_iter()
        .filter(|component| component.len() >= 20)
        .count();
    if substantive >= 3 {
        return;
    }
    let mut candidates = Vec::new();
    for y in 4..grid.height.saturating_sub(4) {
        for x in 5..grid.width.saturating_sub(5) {
            if !h3_stamp_fits(grid, i32::from(x) - 4, i32::from(y) - 2, 9, 5, 2) {
                continue;
            }
            let clear = (y - 2..=y + 2).all(|check_y| {
                (x - 4..=x + 4).all(|check_x| {
                    matches!(
                        grid.cell(check_x, check_y),
                        Some(MapCell::Grass | MapCell::Lawn | MapCell::Tree | MapCell::ParkTree)
                    ) && !near_house_frontage(grid, check_x, check_y)
                        && !near_relief(grid, check_x, check_y, 2)
                })
            });
            let separated = (y.saturating_sub(4)..=(y + 4).min(grid.height - 1)).all(|check_y| {
                (x.saturating_sub(6)..=(x + 6).min(grid.width - 1))
                    .all(|check_x| grid.cell(check_x, check_y) != Some(MapCell::Park))
            });
            if clear && separated {
                candidates.push((hash(x, y), x, y));
            }
        }
    }
    candidates.sort_unstable();
    let mut selected = Vec::<(u16, u16)>::new();
    for (seed, center_x, center_y) in candidates {
        if substantive >= 3 {
            break;
        }
        if selected
            .iter()
            .any(|&(x, y)| x.abs_diff(center_x) < 15 && y.abs_diff(center_y) < 10)
        {
            continue;
        }
        let west = center_x - 4;
        let north = center_y - 2;
        for dy in 0..5_u16 {
            for dx in 0..9_u16 {
                // A safe one-cell lane and four bitten rim cells give the room
                // Crystal's readable grass-band grammar without a rectangle.
                let lane = dy == 2 && dx > 0;
                let bite = matches!((dx, dy), (0, 0) | (8, 4) | (2, 0) | (6, 4))
                    || (seed & 1 != 0 && matches!((dx, dy), (8, 0) | (0, 4)));
                if !lane && !bite {
                    set_cell(
                        grid,
                        i32::from(west + dx),
                        i32::from(north + dy),
                        MapCell::Park,
                    );
                }
            }
        }
        selected.push((center_x, center_y));
        substantive += 1;
    }

    if substantive < 2 {
        let mut fallback = Vec::new();
        for y in 3..grid.height.saturating_sub(4) {
            for x in 4..grid.width.saturating_sub(5) {
                if !h3_stamp_fits(grid, i32::from(x) - 3, i32::from(y) - 2, 7, 4, 1)
                    || (y.saturating_sub(3)..=(y + 3).min(grid.height - 1)).any(|check_y| {
                        (x.saturating_sub(4)..=(x + 4).min(grid.width - 1))
                            .any(|check_x| grid.cell(check_x, check_y) == Some(MapCell::Park))
                    })
                {
                    continue;
                }
                let clear = (y - 2..=y + 1).all(|check_y| {
                    (x - 3..=x + 3).all(|check_x| {
                        matches!(
                            grid.cell(check_x, check_y),
                            Some(
                                MapCell::Grass | MapCell::Lawn | MapCell::Tree | MapCell::ParkTree
                            )
                        ) && !near_house_frontage(grid, check_x, check_y)
                            && !near_relief(grid, check_x, check_y, 2)
                    })
                });
                if clear {
                    fallback.push((hash(x, y), x, y));
                }
            }
        }
        fallback.sort_unstable();
        let mut selected = Vec::<(u16, u16)>::new();
        for (seed, center_x, center_y) in fallback {
            if substantive >= 2 {
                break;
            }
            if selected
                .iter()
                .any(|&(x, y)| x.abs_diff(center_x) < 11 && y.abs_diff(center_y) < 8)
            {
                continue;
            }
            let west = center_x - 3;
            let north = center_y - 2;
            for dy in 0..4_u16 {
                for dx in 0..7_u16 {
                    let bite = matches!((dx, dy), (0, 0) | (6, 3) | (2, 0) | (5, 3))
                        || (seed & 1 != 0 && matches!((dx, dy), (6, 0)));
                    if !bite {
                        set_cell(
                            grid,
                            i32::from(west + dx),
                            i32::from(north + dy),
                            MapCell::Park,
                        );
                    }
                }
            }
            selected.push((center_x, center_y));
            substantive += 1;
        }
    }

    if substantive < 2 {
        let mut compact_fields = Vec::new();
        for y in 3..grid.height.saturating_sub(6) {
            for x in 3..grid.width.saturating_sub(6) {
                if !h3_stamp_fits(grid, i32::from(x), i32::from(y), 5, 5, 2)
                    || (y.saturating_sub(3)..=(y + 7).min(grid.height - 1)).any(|check_y| {
                        (x.saturating_sub(3)..=(x + 7).min(grid.width - 1))
                            .any(|check_x| grid.cell(check_x, check_y) == Some(MapCell::Park))
                    })
                {
                    continue;
                }
                let clear = (0..5_u16).all(|dy| {
                    (0..5_u16).all(|dx| {
                        matches!(
                            grid.cell(x + dx, y + dy),
                            Some(
                                MapCell::Grass | MapCell::Lawn | MapCell::Tree | MapCell::ParkTree
                            )
                        ) && !near_house_frontage(grid, x + dx, y + dy)
                            && !near_relief(grid, x + dx, y + dy, 1)
                    })
                });
                if clear {
                    compact_fields.push((hash(x, y), x, y));
                }
            }
        }
        compact_fields.sort_unstable();
        if let Some((seed, x, y)) = compact_fields.into_iter().next() {
            for dy in 0..5_u16 {
                for dx in 0..5_u16 {
                    let corner = matches!((dx, dy), (0, 0) | (4, 0) | (0, 4) | (4, 4));
                    let extra_bite = seed & 1 != 0 && matches!((dx, dy), (2, 0));
                    if !corner && !extra_bite {
                        set_cell(grid, i32::from(x + dx), i32::from(y + dy), MapCell::Park);
                    }
                }
            }
        }
    }
}

fn ensure_h3_compact_wild_accents(grid: &mut GeneratedGrid) {
    irregularize_h3_compact_wild_rectangles(grid);
    let mut compact = terrain_components(grid, MapCell::Park)
        .into_iter()
        .filter(|component| (6..=12).contains(&component.len()))
        .count();
    if compact >= 6 {
        return;
    }
    const SHAPES: [&[(i32, i32); 6]; 4] = [
        &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (1, 2)],
        &[(0, 0), (0, 1), (1, 1), (2, 1), (2, 2), (3, 2)],
        &[(1, 0), (0, 1), (1, 1), (2, 1), (1, 2), (2, 2)],
        &[(0, 0), (1, 0), (1, 1), (2, 1), (3, 1), (2, 2)],
    ];
    let mut candidates = Vec::new();
    for y in 3..grid.height.saturating_sub(4) {
        for x in 3..grid.width.saturating_sub(4) {
            let seed = hash(x, y);
            let shape = SHAPES[(seed as usize) % SHAPES.len()];
            if !h3_stamp_fits(grid, i32::from(x), i32::from(y), 4, 3, 2)
                || shape.iter().any(|&(dx, dy)| {
                    !matches!(
                        grid.cell((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16),
                        Some(MapCell::Grass | MapCell::Lawn | MapCell::Tree | MapCell::ParkTree)
                    )
                })
                || near_transport(grid, x + 1, y + 1, 1)
                || near_house_frontage(grid, x + 1, y + 1)
                || near_relief(grid, x + 1, y + 1, 2)
                || (y.saturating_sub(2)..=(y + 4).min(grid.height - 1)).any(|check_y| {
                    (x.saturating_sub(2)..=(x + 5).min(grid.width - 1))
                        .any(|check_x| grid.cell(check_x, check_y) == Some(MapCell::Park))
                })
            {
                continue;
            }
            candidates.push((seed, x, y));
        }
    }
    candidates.sort_unstable();
    let mut selected = Vec::<(u16, u16)>::new();
    for (seed, x, y) in candidates {
        if compact >= 6 {
            break;
        }
        if selected
            .iter()
            .any(|&(other_x, other_y)| x.abs_diff(other_x) < 7 && y.abs_diff(other_y) < 7)
        {
            continue;
        }
        let shape = SHAPES[(seed as usize) % SHAPES.len()];
        for &(dx, dy) in shape {
            set_cell(grid, i32::from(x) + dx, i32::from(y) + dy, MapCell::Park);
        }
        selected.push((x, y));
        compact += 1;
    }
}

/// Earlier response-independent infill layers can occasionally touch and
/// merge into a solid 2xN rectangle. Remove one deterministic corner before
/// counting completion; six-cell rectangles become small tufts and are
/// replaced by a proper six-cell accent below, while larger rectangles retain
/// their encounter coverage with an immediately readable notch.
fn irregularize_h3_compact_wild_rectangles(grid: &mut GeneratedGrid) {
    let width = usize::from(grid.width);
    loop {
        let rectangles = terrain_components(grid, MapCell::Park)
            .into_iter()
            .filter(|component| (3..=12).contains(&component.len()))
            .filter_map(|component| {
                let min_x = component.iter().map(|index| index % width).min()?;
                let max_x = component.iter().map(|index| index % width).max()?;
                let min_y = component.iter().map(|index| index / width).min()?;
                let max_y = component.iter().map(|index| index / width).max()?;
                ((max_x - min_x + 1) * (max_y - min_y + 1) == component.len())
                    .then_some((component, min_x, max_x, min_y, max_y))
            })
            .collect::<Vec<_>>();
        if rectangles.is_empty() {
            break;
        }
        for (component, min_x, max_x, min_y, max_y) in rectangles {
            let mut corners = component
                .into_iter()
                .filter(|index| {
                    let x = index % width;
                    let y = index / width;
                    (x == min_x || x == max_x) && (y == min_y || y == max_y)
                })
                .collect::<Vec<_>>();
            corners.sort_unstable_by_key(|index| {
                let x = (*index % width) as u16;
                let y = (*index / width) as u16;
                (hash(x, y), *index)
            });
            if let Some(index) = corners.into_iter().next() {
                grid.cells[index] = MapCell::Lawn;
            }
        }
    }
}

fn promote_h3_wild_rooms(grid: &mut GeneratedGrid) {
    if grid.source.h3.is_none() {
        return;
    }
    loop {
        let mut components = terrain_components(grid, MapCell::Park);
        components.sort_by_key(|component| std::cmp::Reverse(component.len()));
        let substantive = components
            .iter()
            .filter(|component| component.len() >= 20)
            .count();
        if substantive >= 2 {
            return;
        }
        let Some(component) = components
            .into_iter()
            .find(|component| (13..20).contains(&component.len()))
        else {
            return;
        };
        let width = usize::from(grid.width);
        let mut candidates = Vec::new();
        for &index in &component {
            let x = (index % width) as u16;
            let y = (index / width) as u16;
            for (check_x, check_y) in cardinal_neighbors(x, y, grid.width, grid.height) {
                let check_index = usize::from(check_y) * width + usize::from(check_x);
                if component.contains(&check_index)
                    || !matches!(
                        grid.cell(check_x, check_y),
                        Some(MapCell::Grass | MapCell::Lawn | MapCell::Tree | MapCell::ParkTree)
                    )
                    || !h3_stamp_fits(grid, i32::from(check_x), i32::from(check_y), 1, 1, 2)
                    || near_transport(grid, check_x, check_y, 1)
                    || near_house_frontage(grid, check_x, check_y)
                    || near_relief(grid, check_x, check_y, 2)
                {
                    continue;
                }
                let touches_other_field =
                    cardinal_neighbors(check_x, check_y, grid.width, grid.height)
                        .into_iter()
                        .any(|(neighbor_x, neighbor_y)| {
                            let neighbor =
                                usize::from(neighbor_y) * width + usize::from(neighbor_x);
                            grid.cell(neighbor_x, neighbor_y) == Some(MapCell::Park)
                                && !component.contains(&neighbor)
                        });
                if !touches_other_field {
                    candidates.push((hash(check_x, check_y), check_x, check_y));
                }
            }
        }
        candidates.sort_unstable();
        candidates.dedup_by_key(|candidate| (candidate.1, candidate.2));
        let Some((_, x, y)) = candidates.into_iter().next() else {
            return;
        };
        set_cell(grid, i32::from(x), i32::from(y), MapCell::Park);
    }
}

/// Keep the H3 encounter grammar to at most four substantive rooms.
///
/// Independent address-stable infill proposals are intentionally numerous,
/// but a late terrain rewrite can occasionally join several accents into a
/// fifth 20+ cell component. Preserve the four largest authored rooms and cut
/// only each excess component along deterministic one-cell lawn lanes. The
/// retained pieces stay connected, irregular, and catalog-sized rather than
/// disappearing as one large rectangle of generic lawn.
fn cap_h3_substantive_wild_sites(grid: &mut GeneratedGrid) {
    if grid.source.h3.is_none() {
        return;
    }
    let mut substantive = terrain_components(grid, MapCell::Park)
        .into_iter()
        .filter(|component| component.len() >= 20)
        .collect::<Vec<_>>();
    substantive.sort_unstable_by(|left, right| {
        right
            .len()
            .cmp(&left.len())
            .then_with(|| left.iter().min().cmp(&right.iter().min()))
    });
    for component in substantive.into_iter().skip(4) {
        let islands = normalized_h3_wild_islands(&component, grid.width, grid.height);
        for index in component {
            grid.cells[index] = MapCell::Lawn;
        }
        for island in islands {
            for index in island {
                grid.cells[index] = MapCell::Park;
            }
        }
    }
}

fn normalized_h3_wild_islands(component: &[usize], width: u16, height: u16) -> Vec<Vec<usize>> {
    let width_usize = usize::from(width);
    let mut best = Vec::<Vec<usize>>::new();
    let mut best_score = (0_usize, 0_usize, 0_usize);
    for lane_x in 0..4_usize {
        for lane_y in 0..4_usize {
            let retained = component
                .iter()
                .copied()
                .filter(|index| {
                    let x = index % width_usize;
                    let y = index / width_usize;
                    x % 4 != lane_x && y % 4 != lane_y
                })
                .collect::<std::collections::BTreeSet<_>>();
            let mut islands = indexed_components(&retained, width, height)
                .into_iter()
                .filter_map(|island| irregularize_normalized_wild_island(island, width))
                .collect::<Vec<_>>();
            islands.sort_unstable_by_key(|island| island.iter().min().copied().unwrap_or(0));
            let coverage = islands.iter().map(Vec::len).sum::<usize>();
            let score = (usize::from(islands.len() >= 2), coverage, islands.len());
            if score > best_score {
                best_score = score;
                best = islands;
            }
        }
    }
    best
}

fn indexed_components(
    indices: &std::collections::BTreeSet<usize>,
    width: u16,
    height: u16,
) -> Vec<Vec<usize>> {
    let width_usize = usize::from(width);
    let mut remaining = indices.clone();
    let mut components = Vec::new();
    while let Some(start) = remaining.iter().next().copied() {
        remaining.remove(&start);
        let mut component = Vec::new();
        let mut frontier = vec![start];
        while let Some(index) = frontier.pop() {
            component.push(index);
            let x = (index % width_usize) as u16;
            let y = (index / width_usize) as u16;
            for (next_x, next_y) in cardinal_neighbors(x, y, width, height) {
                let next = usize::from(next_y) * width_usize + usize::from(next_x);
                if remaining.remove(&next) {
                    frontier.push(next);
                }
            }
        }
        components.push(component);
    }
    components
}

fn irregularize_normalized_wild_island(mut island: Vec<usize>, width: u16) -> Option<Vec<usize>> {
    if !(3..=10).contains(&island.len()) {
        return None;
    }
    island.sort_unstable();
    if wild_component_bounding_area(&island, width) > island.len() {
        return Some(island);
    }
    if island.len() <= 3 {
        return None;
    }
    let width_usize = usize::from(width);
    let min_x = island.iter().map(|index| index % width_usize).min()?;
    let max_x = island.iter().map(|index| index % width_usize).max()?;
    let min_y = island.iter().map(|index| index / width_usize).min()?;
    let max_y = island.iter().map(|index| index / width_usize).max()?;
    let mut corners = island
        .iter()
        .copied()
        .filter(|index| {
            let x = index % width_usize;
            let y = index / width_usize;
            (x == min_x || x == max_x) && (y == min_y || y == max_y)
        })
        .collect::<Vec<_>>();
    corners.sort_unstable_by_key(|index| {
        let x = (*index % width_usize) as u16;
        let y = (*index / width_usize) as u16;
        (hash(x, y), *index)
    });
    for corner in corners {
        let reduced = island
            .iter()
            .copied()
            .filter(|index| *index != corner)
            .collect::<std::collections::BTreeSet<_>>();
        let components = indexed_components(&reduced, width, u16::MAX);
        if components.len() == 1
            && (3..=10).contains(&components[0].len())
            && wild_component_bounding_area(&components[0], width) > components[0].len()
        {
            return components.into_iter().next();
        }
    }
    None
}

fn wild_component_bounding_area(component: &[usize], width: u16) -> usize {
    let width = usize::from(width);
    let min_x = component
        .iter()
        .map(|index| index % width)
        .min()
        .unwrap_or(0);
    let max_x = component
        .iter()
        .map(|index| index % width)
        .max()
        .unwrap_or(0);
    let min_y = component
        .iter()
        .map(|index| index / width)
        .min()
        .unwrap_or(0);
    let max_y = component
        .iter()
        .map(|index| index / width)
        .max()
        .unwrap_or(0);
    (max_x - min_x + 1) * (max_y - min_y + 1)
}

fn cardinal_neighbors(x: u16, y: u16, width: u16, height: u16) -> Vec<(u16, u16)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn repair_h3_house_stamps(grid: &mut GeneratedGrid) -> bool {
    if grid.source.h3.is_none() {
        return false;
    }
    let width = usize::from(grid.width);
    let malformed = terrain_components(grid, MapCell::Building)
        .into_iter()
        .filter(|component| {
            if component.len() != 4 {
                return true;
            }
            let min_x = component
                .iter()
                .map(|index| index % width)
                .min()
                .unwrap_or(0);
            let max_x = component
                .iter()
                .map(|index| index % width)
                .max()
                .unwrap_or(0);
            let min_y = component
                .iter()
                .map(|index| index / width)
                .min()
                .unwrap_or(0);
            let max_y = component
                .iter()
                .map(|index| index / width)
                .max()
                .unwrap_or(0);
            max_x - min_x != 1 || max_y - min_y != 1
        })
        .collect::<Vec<_>>();
    if malformed.is_empty() {
        return false;
    }
    let replacement_count = malformed.len();
    for component in malformed {
        for index in component {
            grid.cells[index] = MapCell::Lawn;
        }
    }
    // The seam transport is already authored, so a fresh complete footprint
    // cannot select the conflicting site again. Relocate exactly the number
    // removed; re-running the bulk settlement planner would duplicate houses
    // because its local `placed` list intentionally starts empty.
    for _ in 0..replacement_count {
        let home = grid.home_cell();
        let mut candidates = Vec::new();
        for y in 2..grid.height.saturating_sub(4) {
            for x in 2..grid.width.saturating_sub(3) {
                if !h3_stamp_fits(grid, i32::from(x), i32::from(y), 2, 2, 3)
                    || !house_site_is_clear(grid, i32::from(x), i32::from(y))
                {
                    continue;
                }
                let distance = x.abs_diff(home.0) + y.abs_diff(home.1.saturating_sub(3));
                candidates.push((distance, hash(x, y), x, y));
            }
        }
        candidates.sort_unstable();
        let Some((_, _, x, y)) = candidates.into_iter().next() else {
            break;
        };
        for dy in 0..2 {
            for dx in 0..2 {
                set_cell(
                    grid,
                    i32::from(x + dx),
                    i32::from(y + dy),
                    MapCell::Building,
                );
            }
        }
        soften_house_yard(grid, i32::from(x), i32::from(y));
        connect_house_frontage(grid, i32::from(x), i32::from(y));
    }
    true
}

fn ensure_h3_forest_density(grid: &mut GeneratedGrid) -> Result<()> {
    let authored_land = grid
        .cells
        .iter()
        .filter(|cell| {
            !matches!(
                **cell,
                MapCell::H3Void
                    | MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth
            )
        })
        .count();
    let density_denominator = if grid.source.h3.is_some() {
        authored_land
    } else {
        grid.cells.len()
    };
    let mut target = density_denominator * 22 / 100;
    let hard_floor = density_denominator.saturating_mul(19).div_ceil(100);
    let mut canopy = grid
        .cells
        .iter()
        .filter(|cell| {
            matches!(
                **cell,
                MapCell::Tree | MapCell::ParkTree | MapCell::SmallTree | MapCell::SmallTreeSouth
            )
        })
        .count();
    if canopy >= target {
        return Ok(());
    }

    const SHAPES: [&[(i32, i32)]; 4] = [
        &[(0, 1), (1, 0), (1, 1), (2, 1), (1, 2), (2, 2)],
        &[(0, 0), (1, 0), (2, 0), (0, 1), (1, 1), (1, 2), (2, 2)],
        &[(0, 1), (1, 0), (2, 0), (1, 1), (2, 1), (3, 1), (2, 2)],
        &[(0, 0), (1, 0), (1, 1), (2, 1), (3, 1), (2, 2), (3, 2)],
    ];
    let stable_grid = StableGrid::for_grid(grid)?;
    // A single blue-noise layer can exhaust its within-layer spacing even
    // though other safe interleaved grove sites remain. Re-evaluate after each
    // layer until the target is met or no complete irregular stamp fits.
    let mut forest_regions_only = true;
    loop {
        let canopy_before = canopy;
        let mut candidates = Vec::new();
        for y in 4..grid.height.saturating_sub(7) {
            for x in 4..grid.width.saturating_sub(8) {
                if !h3_stamp_fits(grid, i32::from(x), i32::from(y), 4, 3, 3)
                    || (forest_regions_only
                        && !crate::biomes::prefers_dense_canopy(stable_grid, x + 1, y + 1))
                    || near_transport(grid, x + 1, y + 1, 2)
                    || near_house_frontage(grid, x + 1, y + 1)
                    || near_relief(grid, x + 1, y + 1, 3)
                {
                    continue;
                }
                let Some(address) = stable_grid.cell(x, y) else {
                    continue;
                };
                let seed = address.stable_hash(0x4833_464f_5245_5354);
                let shape = SHAPES[(seed as usize) % SHAPES.len()];
                if shape.iter().all(|&(dx, dy)| {
                    matches!(
                        grid.cell((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16),
                        Some(MapCell::Grass | MapCell::Lawn)
                    )
                }) {
                    candidates.push((seed, x, y));
                }
            }
        }
        candidates.sort_unstable();
        let mut selected = Vec::<(u16, u16)>::new();
        for (seed, x, y) in candidates {
            if canopy >= target {
                break;
            }
            if selected
                .iter()
                .any(|&(other_x, other_y)| x.abs_diff(other_x) <= 5 && y.abs_diff(other_y) <= 5)
            {
                continue;
            }
            let shape = SHAPES[(seed as usize) % SHAPES.len()];
            for &(dx, dy) in shape {
                set_cell(grid, i32::from(x) + dx, i32::from(y) + dy, MapCell::Tree);
                canopy += 1;
            }
            selected.push((x, y));
        }
        if canopy >= target {
            break;
        }
        if canopy == canopy_before && forest_regions_only && canopy < hard_floor {
            forest_regions_only = false;
            target = hard_floor;
            continue;
        }
        if canopy == canopy_before {
            break;
        }
    }
    Ok(())
}

/// Finish a dense regional face with small National Park-style tree clusters
/// when the earlier 4x3 grove stamps have no complete footprint left. This is
/// deliberately a final, interior-only fallback: it never touches a route,
/// shoreline, facility, wild room, relief feature, or H3 join, and each stamp
/// is rejected if turning its lawn cells into blockers disconnects any
/// previously reachable walkable cell.
fn top_up_h3_interior_canopy(grid: &mut GeneratedGrid) -> Result<()> {
    if grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.regional.is_none())
    {
        return Ok(());
    }
    let authored_land = grid
        .cells
        .iter()
        .filter(|cell| {
            !matches!(
                **cell,
                MapCell::H3Void
                    | MapCell::Water
                    | MapCell::WaterAccessEast
                    | MapCell::WaterAccessWest
                    | MapCell::WaterAccessSouth
            )
        })
        .count();
    // The hard audit begins at 18%. A one-point margin prevents a final
    // furniture stamp or authoritative seam correction from rounding the face
    // back below the threshold.
    let target = authored_land.saturating_mul(19).div_ceil(100);
    let mut canopy = grid
        .cells
        .iter()
        .filter(|cell| h3_canopy_cell(**cell))
        .count();
    if canopy >= target {
        return Ok(());
    }

    const L_CLUSTERS: [[(i32, i32); 3]; 4] = [
        [(0, 0), (1, 0), (0, 1)],
        [(0, 0), (1, 0), (1, 1)],
        [(0, 0), (0, 1), (1, 1)],
        [(1, 0), (0, 1), (1, 1)],
    ];
    let stable_grid = StableGrid::for_grid(grid)?;
    let mut proposals = Vec::new();
    for y in 4..grid.height.saturating_sub(5) {
        for x in 4..grid.width.saturating_sub(5) {
            if !h3_stamp_fits(grid, i32::from(x), i32::from(y), 2, 2, 3) {
                continue;
            }
            let Some(address) = stable_grid.cell(x, y) else {
                continue;
            };
            let seed = address.stable_hash(0x4833_4d49_4352_4f47);
            for shape_offset in 0..L_CLUSTERS.len() {
                let shape_index = (seed as usize + shape_offset) % L_CLUSTERS.len();
                let cells = L_CLUSTERS[shape_index]
                    .map(|(dx, dy)| ((i32::from(x) + dx) as u16, (i32::from(y) + dy) as u16));
                if !cells
                    .iter()
                    .all(|&(cell_x, cell_y)| h3_canopy_top_up_site_is_clear(grid, cell_x, cell_y))
                {
                    continue;
                }
                let neighboring_canopy = cells
                    .iter()
                    .map(|&(cell_x, cell_y)| canopy_neighbors(grid, cell_x, cell_y, 2))
                    .sum::<usize>();
                proposals.push((
                    std::cmp::Reverse(neighboring_canopy),
                    seed.rotate_left(shape_index as u32 * 7),
                    cells,
                ));
            }
        }
    }
    proposals.sort_unstable_by_key(|proposal| (proposal.0, proposal.1));
    for (_, _, cells) in proposals {
        if canopy >= target {
            break;
        }
        if !cells
            .iter()
            .all(|&(x, y)| h3_canopy_top_up_site_is_clear(grid, x, y))
            || !commit_connectivity_safe_canopy(grid, &cells)
        {
            continue;
        }
        canopy += cells.len();
    }

    // A triomino intentionally overshoots most deficits, but a face can end
    // one or two cells short. Fill only attached, deterministic interior
    // accents; they remain large-tree metatiles and cannot form a ruler line.
    if canopy < target {
        let mut accents = Vec::new();
        for y in 4..grid.height.saturating_sub(4) {
            for x in 4..grid.width.saturating_sub(4) {
                if h3_stamp_fits(grid, i32::from(x), i32::from(y), 1, 1, 3)
                    && h3_canopy_top_up_site_is_clear(grid, x, y)
                {
                    let neighbors = canopy_neighbors(grid, x, y, 2);
                    if neighbors > 0 {
                        let seed = stable_grid
                            .cell(x, y)
                            .map(|cell| cell.stable_hash(0x4833_4341_4e4f_5059))
                            .unwrap_or(u64::MAX);
                        accents.push((std::cmp::Reverse(neighbors), seed, x, y));
                    }
                }
            }
        }
        accents.sort_unstable();
        for (_, _, x, y) in accents {
            if canopy >= target {
                break;
            }
            if h3_canopy_top_up_site_is_clear(grid, x, y)
                && commit_connectivity_safe_canopy(grid, &[(x, y)])
            {
                canopy += 1;
            }
        }
    }
    Ok(())
}

fn h3_canopy_cell(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::Tree | MapCell::ParkTree | MapCell::SmallTree | MapCell::SmallTreeSouth
    )
}

fn canopy_neighbors(grid: &GeneratedGrid, x: u16, y: u16, radius: u16) -> usize {
    (y.saturating_sub(radius)..=(y + radius).min(grid.height - 1))
        .flat_map(|check_y| {
            (x.saturating_sub(radius)..=(x + radius).min(grid.width - 1))
                .map(move |check_x| (check_x, check_y))
        })
        .filter(|&(check_x, check_y)| h3_canopy_cell(grid.cell(check_x, check_y).unwrap()))
        .count()
}

fn h3_canopy_top_up_site_is_clear(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    if !matches!(grid.cell(x, y), Some(MapCell::Grass | MapCell::Lawn))
        || near_transport(grid, x, y, 1)
        || near_house_frontage(grid, x, y)
        || near_relief(grid, x, y, 1)
    {
        return false;
    }
    for check_y in y.saturating_sub(1)..=(y + 1).min(grid.height - 1) {
        for check_x in x.saturating_sub(1)..=(x + 1).min(grid.width - 1) {
            if matches!(
                grid.cell(check_x, check_y),
                Some(
                    MapCell::H3Void
                        | MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                        | MapCell::Park
                        | MapCell::Pitch
                        | MapCell::Rail
                        | MapCell::Boulder
                        | MapCell::Bench
                        | MapCell::TrashCan
                        | MapCell::Fountain
                        | MapCell::GroundSign
                        | MapCell::FenceNorthWest
                        | MapCell::FenceNorth
                        | MapCell::FenceNorthEast
                        | MapCell::FenceWest
                        | MapCell::FenceEast
                        | MapCell::FenceSouthWest
                        | MapCell::FenceSouth
                        | MapCell::FenceSouthEast
                        | MapCell::LedgeWest
                        | MapCell::LedgeMiddle
                        | MapCell::LedgeEast
                        | MapCell::CliffNorthWest
                        | MapCell::CliffNorth
                        | MapCell::CliffNorthEast
                        | MapCell::CliffWest
                        | MapCell::CliffCenter
                        | MapCell::CliffEast
                        | MapCell::CliffSouthWest
                        | MapCell::CliffSouth
                        | MapCell::CliffSouthEast
                        | MapCell::CliffInnerSouthWest
                        | MapCell::CliffInnerSouthEast
                        | MapCell::CliffStairs
                )
            ) {
                return false;
            }
        }
    }
    true
}

fn commit_connectivity_safe_canopy(grid: &mut GeneratedGrid, cells: &[(u16, u16)]) -> bool {
    if cells
        .iter()
        .any(|&(x, y)| !h3_canopy_top_up_site_is_clear(grid, x, y))
    {
        return false;
    }
    let reached_before = reachable_walkable_cells(grid, grid.home_cell());
    let previous = cells
        .iter()
        .map(|&(x, y)| ((x, y), grid.cell(x, y).unwrap()))
        .collect::<Vec<_>>();
    for &((x, y), _) in &previous {
        set_cell(grid, i32::from(x), i32::from(y), MapCell::Tree);
    }
    let reached_after = reachable_walkable_cells(grid, grid.home_cell());
    let safe = !cells
        .iter()
        .any(|&(x, y)| dense_canopy_run_exceeds(grid, x, y, 10))
        && reached_before.iter().enumerate().all(|(index, reached)| {
            !*reached || !is_walkable_cell(grid.cells[index]) || reached_after[index]
        });
    if !safe {
        for ((x, y), cell) in previous {
            set_cell(grid, i32::from(x), i32::from(y), cell);
        }
    }
    safe
}

fn dense_canopy_run_exceeds(grid: &GeneratedGrid, x: u16, y: u16, maximum: u16) -> bool {
    let dense = |cell| matches!(cell, Some(MapCell::Tree | MapCell::ParkTree));
    let horizontal = (1..=maximum)
        .take_while(|offset| x >= *offset && dense(grid.cell(x - offset, y)))
        .count()
        + 1
        + (1..=maximum)
            .take_while(|offset| x + offset < grid.width && dense(grid.cell(x + offset, y)))
            .count();
    let vertical = (1..=maximum)
        .take_while(|offset| y >= *offset && dense(grid.cell(x, y - offset)))
        .count()
        + 1
        + (1..=maximum)
            .take_while(|offset| y + offset < grid.height && dense(grid.cell(x, y + offset)))
            .count();
    horizontal > usize::from(maximum) || vertical > usize::from(maximum)
}

fn shape_wild_sites(grid: &mut GeneratedGrid) -> Result<()> {
    let width = usize::from(grid.width);
    let stable_grid = StableGrid::for_grid(grid)?;
    let mut components = terrain_components(grid, MapCell::Park);
    components.sort_by_key(|component| std::cmp::Reverse(component.len()));
    for component in components.into_iter().skip(4) {
        for index in component {
            grid.cells[index] = MapCell::Grass;
        }
    }

    let mut components = terrain_components(grid, MapCell::Park);
    components.sort_by_key(|component| std::cmp::Reverse(component.len()));
    for component in components.into_iter().take(4) {
        let min_x = component
            .iter()
            .map(|index| index % width)
            .min()
            .unwrap_or(0);
        let max_x = component
            .iter()
            .map(|index| index % width)
            .max()
            .unwrap_or(0);
        let min_y = component
            .iter()
            .map(|index| index / width)
            .min()
            .unwrap_or(0);
        let max_y = component
            .iter()
            .map(|index| index / width)
            .max()
            .unwrap_or(0);
        let center_x = (min_x + max_x) / 2;
        let center_y = (min_y + max_y) / 2;
        let horizontal = max_x - min_x >= max_y - min_y;
        // A Crystal encounter field should fit inside a gameplay room. Start
        // from a compact 9x5 (or 5x9) park room, then cut a safe center lane
        // through it below so the player never sees only encounter grass.
        let (half_width, half_height) = if horizontal { (4, 2) } else { (2, 4) };
        let keep_west = center_x.saturating_sub(half_width);
        let keep_east = (center_x + half_width).min(width - 1);
        let keep_north = center_y.saturating_sub(half_height);
        let keep_south = (center_y + half_height).min(usize::from(grid.height) - 1);
        for index in component {
            let x = index % width;
            let y = index / width;
            if x < keep_west || x > keep_east || y < keep_north || y > keep_south {
                grid.cells[index] = MapCell::Grass;
            }
        }

        // Semantic compression deliberately regularizes noisy OSM park edges
        // into a real route room; source water and other authored features are
        // never overwritten.
        for y in keep_north..=keep_south {
            for x in keep_west..=keep_east {
                let index = y * width + x;
                if grid.cells[index] == MapCell::Grass {
                    grid.cells[index] = MapCell::Park;
                }
            }
        }

        // The two grass bands remain one encounter component through a single
        // bridge at the far end, while the open end of the safe lane meets the
        // surrounding lawn. This is the same readable room grammar used by
        // Crystal routes: encounter strips beside a safe traversal channel.
        if horizontal {
            for x in keep_west + 1..=keep_east {
                let index = center_y * width + x;
                if grid.cells[index] == MapCell::Park {
                    grid.cells[index] = MapCell::Lawn;
                }
            }
        } else {
            for y in keep_north + 1..=keep_south {
                let index = y * width + center_x;
                if grid.cells[index] == MapCell::Park {
                    grid.cells[index] = MapCell::Lawn;
                }
            }
        }

        // Break the straight outer silhouette with globally stable shallow
        // bites. They stay on the outermost row/column, so neither grass band
        // is severed and the safe center lane remains open to its bridge.
        let seed = stable_grid
            .cell(center_x as u16, center_y as u16)
            .map(|cell| cell.stable_hash(0x5749_4c44_4249_5445))
            .unwrap_or_default();
        notch_wild_field(
            grid, keep_west, keep_east, keep_north, keep_south, horizontal, seed,
        );
    }
    remove_tiny_areas(grid, MapCell::Park, 20);
    Ok(())
}

fn notch_wild_field(
    grid: &mut GeneratedGrid,
    west: usize,
    east: usize,
    north: usize,
    south: usize,
    horizontal: bool,
    seed: u64,
) {
    let along = |salt: u32, length: usize| 1 + (seed.rotate_left(salt) as usize % (length - 2));
    let mut bites = Vec::new();
    if horizontal {
        let width = east - west + 1;
        bites.extend([
            (west + along(7, width), north),
            (west + along(19, width), south),
            (
                if seed & 1 == 0 { west } else { east },
                if seed & 2 == 0 { north } else { south },
            ),
        ]);
    } else {
        let height = south - north + 1;
        bites.extend([
            (west, north + along(7, height)),
            (east, north + along(19, height)),
            (
                if seed & 2 == 0 { west } else { east },
                if seed & 1 == 0 { north } else { south },
            ),
        ]);
    }
    bites.sort_unstable();
    bites.dedup();
    for (x, y) in bites {
        let index = y * usize::from(grid.width) + x;
        if grid.cells[index] == MapCell::Park {
            grid.cells[index] = MapCell::Lawn;
        }
    }
}

/// Add compact encounter-grass accents to otherwise ordinary open ground.
///
/// Candidate anchors are stable local minima in the global hash field. Shape,
/// reflection, and rotation also come from global `WorldCell` hashes. This is
/// a response-independent blue-noise layout: it neither slides with the map
/// window nor exposes the repeated rows and columns of a district lattice.
/// This pass runs after connectivity and authored decoration; it changes only
/// walkable Grass/Lawn cells and keeps a buffer around every protected layer.
fn place_irregular_wild_infill(grid: &mut GeneratedGrid) -> Result<usize> {
    const ANCHOR_SALT: u64 = 0x414e_4348_4f52_5351;
    const SHAPE_SALT: u64 = 0x5441_4c4c_4752_4153;
    let mut added = place_irregular_wild_infill_layer(grid, ANCHOR_SALT, SHAPE_SALT)?;
    if grid.source.h3.is_some() {
        // A hex reserves its rectangular corners for the face mask and uses a
        // compressed road/landmark grammar. Independent address-stable layers
        // restore several small encounter accents within the playable face.
        for (anchor_mix, shape_mix) in [
            (0xa076_1d64_78bd_642f, 0xe703_7ed1_a0b4_28db),
            (0x8ebc_6af0_9c88_c6e3, 0x5899_65cc_7537_4cc3),
            (0x1d8e_4e27_c47d_124f, 0xeb44_acca_b455_d165),
            (0x94d0_49bb_1331_11eb, 0x369d_ea0f_31a5_3f85),
        ] {
            added += place_irregular_wild_infill_layer(
                grid,
                ANCHOR_SALT ^ anchor_mix,
                SHAPE_SALT ^ shape_mix,
            )?;
        }
    } else if grid.width.min(grid.height) >= 96 {
        // A five-mile-radius regional map contains proportionally more water,
        // roads, and authored canopy than a one-mile room, so one blue-noise
        // layer leaves too little encounter terrain. Eight independent
        // world-addressed layers fill other eligible grass parcels while every
        // earlier patch's two-cell separation remains enforced. The result is
        // dense but remains a collection of small, irregular encounter tufts.
        for (anchor_mix, shape_mix) in [
            (0xa076_1d64_78bd_642f, 0xe703_7ed1_a0b4_28db),
            (0x8ebc_6af0_9c88_c6e3, 0x5899_65cc_7537_4cc3),
            (0x1d8e_4e27_c47d_124f, 0xeb44_acca_b455_d165),
            (0x94d0_49bb_1331_11eb, 0x369d_ea0f_31a5_3f85),
            (0xd6e8_feb8_6659_fd93, 0xa5a3_564e_27f8_8647),
            (0x9e37_79b9_7f4a_7c15, 0xbf58_476d_1ce4_e5b9),
            (0x243f_6a88_85a3_08d3, 0x1319_8a2e_0370_7344),
            (0x3c6e_f372_fe94_f82b, 0xa409_3822_299f_31d0),
        ] {
            added += place_irregular_wild_infill_layer(
                grid,
                ANCHOR_SALT ^ anchor_mix,
                SHAPE_SALT ^ shape_mix,
            )?;
        }
    }
    Ok(added)
}

fn place_irregular_wild_infill_layer(
    grid: &mut GeneratedGrid,
    anchor_salt: u64,
    shape_salt: u64,
) -> Result<usize> {
    const ANCHOR_RADIUS: i64 = 2;

    let stable_grid = StableGrid::for_grid(grid)?;
    let mut proposals = Vec::<(u64, u64, Vec<(u16, u16)>)>::new();

    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            let anchor = stable_grid.cell(x, y).expect("in-bounds stable cell");
            let priority = anchor.stable_hash(anchor_salt);
            let is_global_minimum = (-ANCHOR_RADIUS..=ANCHOR_RADIUS).all(|dy| {
                (-ANCHOR_RADIUS..=ANCHOR_RADIUS).all(|dx| {
                    (dx == 0 && dy == 0)
                        || priority < anchor.offset(dx, dy).stable_hash(anchor_salt)
                })
            });
            if !is_global_minimum {
                continue;
            }

            let shape_seed = anchor.stable_hash(shape_salt);
            let preferred = (shape_seed % IRREGULAR_WILD_SHAPES.len() as u64) as usize;
            let small_fallback = (shape_seed.rotate_left(17) % 3) as usize;
            let medium_fallback = 3 + (shape_seed.rotate_left(31) % 3) as usize;
            let mut shape_choices = vec![preferred, medium_fallback, small_fallback];
            shape_choices.dedup();
            for shape_index in shape_choices {
                let offsets = irregular_wild_shape(shape_index, shape_seed);
                let mut cells = Vec::with_capacity(offsets.len());
                let mut shape_is_clear = true;
                for (dx, dy) in offsets {
                    let Some((x, y)) = stable_grid.local_cell(anchor.offset(dx, dy)) else {
                        shape_is_clear = false;
                        break;
                    };
                    if wild_infill_cell_is_clear(grid, x, y) {
                        cells.push((x, y));
                    } else {
                        shape_is_clear = false;
                        break;
                    }
                }
                if shape_is_clear {
                    proposals.push((
                        priority,
                        anchor.stable_hash(shape_salt ^ 0xd6e8_feb8_6659_fd93),
                        cells,
                    ));
                    break;
                }
            }
        }
    }

    // Suitability was evaluated against one immutable authored layout, so the
    // cells within a patch cannot reject one another after the center is set.
    let cells = if grid.source.h3.is_some() {
        // An H3 face receives five independent infill layers. Resolve only
        // proposals from this same layer in stable-priority order and reserve
        // the same two-cell breathing room used against earlier Park, keeping
        // accents independent without imposing a response-dependent quota.
        proposals.sort_unstable_by_key(|proposal| (proposal.0, proposal.1));
        let mut cells = std::collections::BTreeSet::<(u16, u16)>::new();
        let mut reserved = std::collections::BTreeSet::<(u16, u16)>::new();
        for (_, _, proposal) in proposals {
            if proposal.iter().any(|cell| reserved.contains(cell)) {
                continue;
            }
            for (x, y) in proposal {
                cells.insert((x, y));
                for reserved_y in y.saturating_sub(2)..=(y + 2).min(grid.height - 1) {
                    for reserved_x in x.saturating_sub(2)..=(x + 2).min(grid.width - 1) {
                        reserved.insert((reserved_x, reserved_y));
                    }
                }
            }
        }
        cells
    } else {
        proposals
            .into_iter()
            .flat_map(|proposal| proposal.2)
            .collect::<std::collections::BTreeSet<_>>()
    };
    let added = cells.len();
    for (x, y) in cells {
        set_cell(grid, i32::from(x), i32::from(y), MapCell::Park);
    }
    Ok(added)
}

const IRREGULAR_WILD_SHAPES: &[&[(i64, i64)]] = &[
    // Three-cell sprig.
    &[(0, 0), (1, 0), (0, 1)],
    // Four-cell comma.
    &[(0, -1), (0, 0), (0, 1), (1, 1)],
    // Five-cell zigzag.
    &[(-1, -1), (0, -1), (0, 0), (1, 0), (1, 1)],
    // Six-cell hook.
    &[(-1, -1), (0, -1), (1, -1), (-1, 0), (-1, 1), (0, 1)],
    // Seven-cell comma with a tapered tail.
    &[(-1, -1), (0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (1, 2)],
    // Eight-cell long zigzag.
    &[
        (-2, -1),
        (-1, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (1, 1),
        (2, 1),
        (2, 2),
    ],
    // Nine-cell uneven blob.
    &[
        (-1, -1),
        (0, -1),
        (1, -1),
        (2, -1),
        (-1, 0),
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
    ],
    // Ten-cell meadow blob with two missing corners.
    &[
        (-2, -1),
        (-1, -1),
        (0, -1),
        (1, -1),
        (-2, 0),
        (-1, 0),
        (0, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
    ],
];

fn irregular_wild_shape(shape_index: usize, seed: u64) -> Vec<(i64, i64)> {
    let quarter_turns = (seed & 3) as u32;
    let reflect = seed & 4 != 0;
    IRREGULAR_WILD_SHAPES[shape_index % IRREGULAR_WILD_SHAPES.len()]
        .iter()
        .map(|&(mut x, y)| {
            if reflect {
                x = -x;
            }
            let mut point = (x, y);
            for _ in 0..quarter_turns {
                point = (-point.1, point.0);
            }
            point
        })
        .collect()
}

fn wild_infill_cell_is_clear(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    if !matches!(grid.cell(x, y), Some(MapCell::Grass | MapCell::Lawn))
        || near_transport(grid, x, y, 1)
        || near_house_frontage(grid, x, y)
    {
        return false;
    }

    for check_y in y.saturating_sub(2)..=(y + 2).min(grid.height - 1) {
        for check_x in x.saturating_sub(2)..=(x + 2).min(grid.width - 1) {
            let distance = x.abs_diff(check_x).max(y.abs_diff(check_y));
            let Some(cell) = grid.cell(check_x, check_y) else {
                return false;
            };
            let blocks_infill = match cell {
                // Keep encounter accents separate from the authored wild rooms.
                MapCell::Park => distance <= 2,
                // Preserve readable yards and both service-building forecourts.
                MapCell::Building
                | MapCell::PokecenterNorthWest
                | MapCell::PokecenterNorthEast
                | MapCell::PokecenterSouthWest
                | MapCell::PokecenterSouthEast
                | MapCell::MartNorthWest
                | MapCell::MartNorthEast
                | MapCell::MartSouthWest
                | MapCell::MartSouthEast => distance <= 2,
                // Do not crowd shore access, public rooms, props, or relief.
                MapCell::Water
                | MapCell::WaterAccessEast
                | MapCell::WaterAccessWest
                | MapCell::WaterAccessSouth
                | MapCell::Pitch
                | MapCell::Bench
                | MapCell::TrashCan
                | MapCell::Fountain
                | MapCell::GroundSign
                | MapCell::FenceNorthWest
                | MapCell::FenceNorth
                | MapCell::FenceNorthEast
                | MapCell::FenceWest
                | MapCell::FenceEast
                | MapCell::FenceSouthWest
                | MapCell::FenceSouth
                | MapCell::FenceSouthEast
                | MapCell::LedgeWest
                | MapCell::LedgeMiddle
                | MapCell::LedgeEast
                | MapCell::CliffNorthWest
                | MapCell::CliffNorth
                | MapCell::CliffNorthEast
                | MapCell::CliffWest
                | MapCell::CliffCenter
                | MapCell::CliffEast
                | MapCell::CliffSouthWest
                | MapCell::CliffSouth
                | MapCell::CliffSouthEast
                | MapCell::CliffInnerSouthWest
                | MapCell::CliffInnerSouthEast
                | MapCell::Rail => distance <= 1,
                _ => false,
            };
            if blocks_infill {
                return false;
            }
        }
    }
    true
}

fn select_labels(grid: &GeneratedGrid) -> Vec<GridLabel> {
    let mut candidates = grid
        .source
        .features
        .iter()
        .filter_map(|feature| {
            let name = feature.name.as_ref()?;
            let point = feature.points.get(feature.points.len() / 2)?;
            let (x, y) = project(grid, *point);
            Some((feature.points.len(), name.clone(), x, y))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    candidates.dedup_by(|left, right| left.1 == right.1);
    let signs = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            (*cell == MapCell::GroundSign).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect::<Vec<_>>();
    let mut labels = grid.labels.clone();
    let mut used_names = labels
        .iter()
        .map(|label| label.text.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for (sign_x, sign_y) in signs {
        if labels
            .iter()
            .any(|label| label.x == sign_x && label.y == sign_y)
        {
            continue;
        }
        let nearest = candidates
            .iter()
            .filter(|(_, name, _, _)| !used_names.contains(name))
            .min_by_key(|(_, _, feature_x, feature_y)| {
                (i32::from(sign_x) - *feature_x).abs() + (i32::from(sign_y) - *feature_y).abs()
            });
        let Some((_, name, _, _)) = nearest else {
            continue;
        };
        used_names.insert(name.clone());
        labels.push(GridLabel {
            text: name.clone(),
            x: sign_x,
            y: sign_y,
        });
    }
    labels
}

fn hash(x: u16, y: u16) -> u32 {
    u32::from(x).wrapping_mul(0x45d9_f3b) ^ u32::from(y).wrapping_mul(0x27d4_eb2d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, plan_h3_cell, prepare_h3_source};

    fn source(features: Vec<Feature>) -> MapSource {
        MapSource {
            center: Coordinate { lat: 0.5, lon: 0.5 },
            bounds: BoundingBox {
                south: 0.0,
                west: 0.0,
                north: 1.0,
                east: 1.0,
            },
            attribution: "test".to_string(),
            features,
            h3: None,
        }
    }

    #[test]
    fn regional_connector_detours_around_a_closed_crossing_band() {
        let width = 7_u16;
        let mut grid = GeneratedGrid {
            source: source(Vec::new()),
            width,
            height: 7,
            cells: vec![MapCell::Grass; usize::from(width) * 7],
            labels: Vec::new(),
        };
        let start = (1, 3);
        let goal = (5, 3);
        grid.cells[start.1 as usize * usize::from(width) + start.0 as usize] = MapCell::MajorRoad;
        grid.cells[goal.1 as usize * usize::from(width) + goal.0 as usize] = MapCell::MajorRoad;
        let closed = [(3_usize, 2_usize), (3, 3), (3, 4)]
            .into_iter()
            .map(|(x, y)| y * usize::from(width) + x)
            .collect::<std::collections::BTreeSet<_>>();

        let unconstrained = shortest_land_path(&grid, start, goal).expect("direct land path");
        assert!(
            unconstrained
                .iter()
                .any(|&(x, y)| closed.contains(&(y as usize * usize::from(width) + x as usize))),
            "the generic shortest path should exercise the closed crossing"
        );
        let detour = shortest_path_avoiding(&grid, start, goal, false, &closed)
            .expect("land detour around the closed crossing");
        assert!(
            detour.iter().all(|&(x, y)| {
                !closed.contains(&(y as usize * usize::from(width) + x as usize))
            })
        );
        assert!(
            detour.len() > unconstrained.len(),
            "the accepted connector must be the longer safe route"
        );

        commit_regional_trail(&mut grid, detour);
        assert_eq!(transport_components(&grid).len(), 1);
        assert!(
            closed
                .iter()
                .all(|&index| grid.cells[index] == MapCell::Grass)
        );
    }

    #[test]
    fn h3_water_renders_open_at_void_instead_of_drawing_a_join_shoreline() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            8,
        )
        .expect("H3 plan");
        let mut h3_source = source(Vec::new());
        h3_source.h3 = Some(plan);
        let mut grid = GeneratedGrid {
            source: h3_source,
            width: 3,
            height: 3,
            cells: vec![MapCell::Water; 9],
            labels: Vec::new(),
        };
        grid.cells[3] = MapCell::H3Void;
        assert_eq!(water_block(&grid, 1, 1), 0x35);

        grid.source.h3 = None;
        assert_eq!(water_block(&grid, 1, 1), 0x58);
    }

    #[test]
    fn linear_source_rivers_survive_the_water_noise_filter() {
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: Coordinate {
                    lat: 44.95,
                    lon: -93.32,
                },
                bounds: crate::BoundingBox {
                    south: 44.94,
                    west: -93.33,
                    north: 44.96,
                    east: -93.31,
                },
                attribution: "river fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: 32,
            height: 32,
            cells: vec![MapCell::Grass; 32 * 32],
            labels: Vec::new(),
        };
        let river = Feature {
            kind: FeatureKind::Water,
            name: Some("Test River".to_string()),
            area: false,
            bridge: false,
            points: vec![
                Coordinate {
                    lat: 44.958,
                    lon: -93.329,
                },
                Coordinate {
                    lat: 44.953,
                    lon: -93.323,
                },
                Coordinate {
                    lat: 44.947,
                    lon: -93.326,
                },
                Coordinate {
                    lat: 44.942,
                    lon: -93.312,
                },
            ],
        };
        paint_feature(&mut grid, &river);
        remove_tiny_areas(&mut grid, MapCell::Water, 2);
        let water = terrain_components(&grid, MapCell::Water);
        assert_eq!(water.len(), 1);
        assert!(water[0].len() >= 20, "river cells={}", water[0].len());
    }

    #[test]
    fn regional_connector_survives_late_full_face_capping() {
        let width = 7_u16;
        let height = 7_u16;
        let mut cells = vec![MapCell::Grass; usize::from(width) * usize::from(height)];
        for x in 0..width {
            cells[usize::from(height - 1) * usize::from(width) + usize::from(x)] = MapCell::H3Void;
        }
        let mut grid = GeneratedGrid {
            source: source(Vec::new()),
            width,
            height,
            cells,
            labels: Vec::new(),
        };
        let start = (1_i32, 5_i32);
        let goal = (5_i32, 5_i32);
        let index = |x: i32, y: i32| y as usize * usize::from(width) + x as usize;
        let landings = [index(start.0, start.1), index(goal.0, goal.1)]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        for &landing in &landings {
            grid.cells[landing] = MapCell::MajorRoad;
        }
        let cap_undeclared_face_routes = |grid: &mut GeneratedGrid| {
            for y in 0..height {
                for x in 0..width {
                    let cell_index = usize::from(y) * usize::from(width) + usize::from(x);
                    if !landings.contains(&cell_index)
                        && matches!(
                            grid.cells[cell_index],
                            MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
                        )
                        && crate::h3::route_cell_touches_h3_void(grid, x, y)
                    {
                        grid.cells[cell_index] = MapCell::Grass;
                    }
                }
            }
        };

        let mut old_grid = grid.clone();
        let old_connector = shortest_land_path(&old_grid, start, goal)
            .expect("old shortest connector along the undeclared face mouth");
        assert!(old_connector.iter().any(|&(x, y)| {
            let cell_index = index(x, y);
            !landings.contains(&cell_index)
                && crate::h3::route_cell_touches_h3_void(&old_grid, x as u16, y as u16)
        }));
        commit_regional_trail(&mut old_grid, old_connector);
        cap_undeclared_face_routes(&mut old_grid);
        assert_eq!(
            transport_components(&old_grid).len(),
            2,
            "the old boundary-hugging connector must reproduce the late-cap split"
        );

        let forbidden = regional_connector_forbidden_cells(
            &grid,
            &landings,
            &std::collections::BTreeSet::new(),
        );
        let connector = shortest_path_avoiding(&grid, start, goal, false, &forbidden)
            .expect("interior connector around capped face cells");
        commit_regional_trail(&mut grid, connector);
        cap_undeclared_face_routes(&mut grid);

        assert_eq!(
            transport_components(&grid).len(),
            1,
            "the connector must choose an interior detour that survives the final face cap"
        );
        assert!(grid.cells.iter().enumerate().all(|(cell_index, cell)| {
            !matches!(
                cell,
                MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
            ) || landings.contains(&cell_index)
                || !crate::h3::route_cell_touches_h3_void(
                    &grid,
                    (cell_index % usize::from(width)) as u16,
                    (cell_index / usize::from(width)) as u16,
                )
        }));
    }

    #[test]
    fn h3_backbone_preserves_only_the_selected_explicit_bridge_over_water() {
        let mut plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            8,
        )
        .expect("H3 bridge plan");
        let selected_portal = plan.portals[0].clone();
        let nearby_portal = plan.portals[3].clone();
        let interpolate = |first: Coordinate, second: Coordinate, fraction: f64| Coordinate {
            lat: first.lat + (second.lat - first.lat) * fraction,
            lon: first.lon + (second.lon - first.lon) * fraction,
        };
        let selected_probe = interpolate(plan.center, selected_portal.midpoint, 0.72);
        let internal_start = interpolate(plan.center, selected_portal.midpoint, 0.3);
        let nearby_start = interpolate(plan.center, nearby_portal.midpoint, 0.45);
        let nearby_probe = interpolate(plan.center, nearby_portal.midpoint, 0.72);
        let selected_outside = interpolate(plan.center, selected_portal.midpoint, 1.15);
        let nearby_outside = interpolate(plan.center, nearby_portal.midpoint, 1.15);
        let internal_target = interpolate(plan.center, plan.portals[2].midpoint, 0.35);
        plan.regional = Some(crate::H3RegionalCellPlan {
            ordinal: 0,
            cell: plan.cell.clone(),
            building_count: 0,
            facilities: Vec::new(),
            connections: vec![crate::H3RegionalConnection {
                edge_id: selected_portal.edge_id.clone(),
                neighbor: selected_portal.neighbor.clone(),
                coordinate: selected_portal.midpoint,
                transport: FeatureKind::MajorRoad,
                bridge: true,
                authoritative: true,
                boundary_exit: true,
            }],
            closed_transport_crossings: Vec::new(),
        });
        let selected_bridge = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("selected tagged bridge".to_string()),
            area: false,
            bridge: true,
            points: vec![plan.center, selected_outside],
        };
        let nearby_bridge = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("nearby unselected bridge".to_string()),
            area: false,
            bridge: true,
            points: vec![nearby_start, nearby_outside],
        };
        let internal_bridge = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("wholly internal tagged bridge".to_string()),
            area: false,
            bridge: true,
            points: vec![internal_start, internal_target],
        };
        let make_grid = |selected_bridge: Feature, connection_bridge: bool| {
            let mut grid_plan = plan.clone();
            grid_plan
                .regional
                .as_mut()
                .expect("regional bridge fixture")
                .connections[0]
                .bridge = connection_bridge;
            GeneratedGrid {
                source: MapSource {
                    center: plan.center,
                    bounds: plan.fetch_bounds[0],
                    attribution: "explicit bridge fixture".to_string(),
                    features: vec![
                        selected_bridge,
                        nearby_bridge.clone(),
                        internal_bridge.clone(),
                    ],
                    h3: Some(grid_plan),
                },
                width: 64,
                height: 64,
                cells: vec![MapCell::Water; 64 * 64],
                labels: Vec::new(),
            }
        };

        let mut bridged = make_grid(selected_bridge.clone(), true);
        let bridge_features = selected_h3_bridge_features(&bridged, &bridged.source.features)
            .expect("classify bridge features")
            .into_iter()
            .filter_map(|feature| feature.name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            bridge_features,
            [
                "selected tagged bridge".to_string(),
                "wholly internal tagged bridge".to_string(),
            ]
            .into_iter()
            .collect(),
            "only the selected boundary bridge and wholly internal bridge are water-authoritative"
        );
        paint_h3_source_transport(&mut bridged).expect("selected explicit bridge transport");
        let (selected_x, selected_y) = project(&bridged, selected_probe);
        let (nearby_x, nearby_y) = project(&bridged, nearby_probe);
        let (internal_x, internal_y) = project(&bridged, internal_target);
        assert_eq!(
            bridged.cell(selected_x as u16, selected_y as u16),
            Some(MapCell::MajorRoad),
            "the exact selected bridge=yes trace must survive water restoration"
        );
        assert_eq!(
            bridged.cell(nearby_x as u16, nearby_y as u16),
            Some(MapCell::Water),
            "a nearby bridge not selected by this cell must remain water-barred"
        );
        assert_eq!(
            bridged.cell(internal_x as u16, internal_y as u16),
            Some(MapCell::MajorRoad),
            "an explicit bridge wholly inside the face must survive water restoration"
        );

        let mut unapproved = make_grid(selected_bridge.clone(), false);
        paint_h3_source_transport(&mut unapproved).expect("unapproved bridge transport");
        assert_eq!(
            unapproved.cell(selected_x as u16, selected_y as u16),
            Some(MapCell::Water),
            "a bridge feature without a matching connection flag must remain water-barred"
        );

        let mut untagged = selected_bridge;
        untagged.bridge = false;
        let mut unbridged = make_grid(untagged, true);
        paint_h3_source_transport(&mut unbridged).expect("untagged selected route transport");
        assert_eq!(
            unbridged.cell(selected_x as u16, selected_y as u16),
            Some(MapCell::Water),
            "a connection flag cannot infer bridge authority on an untagged source road"
        );
    }

    #[test]
    fn dense_osm_buildings_create_tight_varied_city_clusters() {
        let mut features = vec![Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("Downtown spine".to_string()),
            area: false,
            bridge: false,
            points: vec![
                Coordinate {
                    lat: 0.5,
                    lon: 0.02,
                },
                Coordinate {
                    lat: 0.5,
                    lon: 0.98,
                },
            ],
        }];
        for row in 0..12 {
            for column in 0..12 {
                features.push(Feature {
                    kind: FeatureKind::Building,
                    name: Some(format!("City parcel {row}-{column}")),
                    area: true,
                    bridge: false,
                    points: vec![Coordinate {
                        lat: 0.06 + f64::from(row) * 0.075,
                        lon: 0.06 + f64::from(column) * 0.075,
                    }],
                });
            }
        }
        let grid = generate_grid(source(features), 64, 64).expect("dense city grid");
        assert_eq!(urban_intensity(&grid), 2);
        let components = terrain_components(&grid, MapCell::Building);
        let houses = components
            .iter()
            .filter(|component| component.len() == 4)
            .count();
        assert!(
            (16..=17).contains(&houses),
            "downtown should retain sixteen varied facades, with the protected home separate when its site does not replace one: {houses}"
        );
        assert!(
            components.iter().any(|component| component.len() == 12),
            "a dense district needs the complete Goldenrod Department Store"
        );

        let blocks = grid.crystal_blocks();
        let width = usize::from(grid.width);
        let mut origins = Vec::new();
        let mut styles = std::collections::BTreeSet::new();
        for component in components.iter().filter(|component| component.len() == 4) {
            let x = component.iter().map(|index| index % width).min().unwrap();
            let y = component.iter().map(|index| index / width).min().unwrap();
            origins.push((x as i32, y as i32));
            let south_y = y + 1;
            styles.insert((blocks[south_y * width + x], blocks[south_y * width + x + 1]));
        }
        assert!(
            styles.len() >= 2,
            "downtown must mix complete modern and traditional residences: {styles:?}"
        );
        let clustered = origins
            .iter()
            .filter(|&&(x, y)| {
                origins.iter().any(|&(other_x, other_y)| {
                    (x, y) != (other_x, other_y)
                        && (x - other_x).abs().max((y - other_y).abs()) <= 6
                })
            })
            .count();
        assert!(
            clustered >= 10,
            "only {clustered}/{houses} downtown facades form readable clusters"
        );
    }

    #[test]
    fn h3_boundary_is_feathered_instead_of_drawn_as_a_visible_ring() {
        let coordinate = Coordinate {
            lat: 44.947_519_6,
            lon: -93.325_347_7,
        };
        let plan = plan_h3_cell(coordinate, 7).expect("larger H3 plan");
        let source = MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds[0],
            attribution: "boundary fixture".to_string(),
            features: Vec::new(),
            h3: Some(plan.clone()),
        };
        let seams = build_h3_seam_contract(&plan, &source, 64, 64).expect("seams");
        let mut grid = GeneratedGrid {
            source,
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        author_h3_boundary(&mut grid, &seams).expect("feather boundary");
        let polygon = plan.raster_polygon(64, 64).expect("raster polygon");
        let mut boundary_cells = 0;
        let mut hard_accents = 0;
        for y in 0..64_u16 {
            for x in 0..64_u16 {
                let inside =
                    point_in_float_polygon(f64::from(x) + 0.5, f64::from(y) + 0.5, &polygon);
                let touches_outside = inside
                    && [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)]
                        .into_iter()
                        .any(|(dx, dy)| {
                            let check_x = i32::from(x) + dx;
                            let check_y = i32::from(y) + dy;
                            check_x < 0
                                || check_y < 0
                                || check_x >= 64
                                || check_y >= 64
                                || !point_in_float_polygon(
                                    f64::from(check_x) + 0.5,
                                    f64::from(check_y) + 0.5,
                                    &polygon,
                                )
                        });
                if touches_outside {
                    boundary_cells += 1;
                    hard_accents += usize::from(matches!(
                        grid.cell(x, y),
                        Some(MapCell::Tree | MapCell::ParkTree | MapCell::Boulder)
                    ));
                }
            }
        }
        assert!(boundary_cells > 0);
        assert_eq!(
            hard_accents, 0,
            "H3 joins must not inject a tree/rock outline: found {hard_accents}/{boundary_cells} hard accents"
        );

        let mut water_grid = GeneratedGrid {
            source: grid.source.clone(),
            width: 64,
            height: 64,
            cells: vec![MapCell::Water; 64 * 64],
            labels: Vec::new(),
        };
        author_h3_boundary(&mut water_grid, &seams).expect("water boundary");
        for y in 0..64_u16 {
            for x in 0..64_u16 {
                if point_in_float_polygon(f64::from(x) + 0.5, f64::from(y) + 0.5, &polygon) {
                    assert_eq!(
                        water_grid.cell(x, y),
                        Some(MapCell::Water),
                        "water must continue through H3 join at ({x},{y})"
                    );
                }
            }
        }
    }

    #[test]
    fn h3_boundary_feathers_a_one_cell_canopy_outline_but_keeps_interior_forest() {
        let coordinate = Coordinate {
            lat: 44.947_519_6,
            lon: -93.325_347_7,
        };
        let plan = plan_h3_cell(coordinate, 6).expect("H3 plan");
        let source = MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds[0],
            attribution: "canopy seam fixture".to_string(),
            features: Vec::new(),
            h3: Some(plan.clone()),
        };
        let seams = build_h3_seam_contract(&plan, &source, 64, 64).expect("seams");
        let mut grid = GeneratedGrid {
            source,
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        let portal = plan.portals[0].clone();
        let sample_coordinates = crate::build_h3_grid_seam_profile(&grid)
            .expect("unmasked seam profile")
            .edges
            .into_iter()
            .find(|edge| edge.edge_id == portal.edge_id)
            .expect("target edge")
            .samples
            .into_iter()
            .map(|sample| sample.coordinate)
            .collect::<Vec<_>>();
        let width = usize::from(grid.width);
        let mut interior_forest = std::collections::BTreeSet::new();
        for (sample_index, sample) in sample_coordinates.iter().copied().enumerate() {
            let band = crate::h3::h3_raster_sample_band(&plan, &grid, sample).expect("sample band");
            let border = band[0];
            let inner = band[2];
            set_cell(
                &mut grid,
                i32::from(border.0),
                i32::from(border.1),
                MapCell::Tree,
            );
            if sample_index < 4 {
                for &(x, y) in &band {
                    set_cell(&mut grid, i32::from(x), i32::from(y), MapCell::Tree);
                }
                interior_forest.insert(usize::from(inner.1) * width + usize::from(inner.0));
            } else if !interior_forest
                .contains(&(usize::from(inner.1) * width + usize::from(inner.0)))
            {
                set_cell(
                    &mut grid,
                    i32::from(inner.0),
                    i32::from(inner.1),
                    MapCell::Grass,
                );
            }
        }
        assert!(
            interior_forest
                .iter()
                .all(|index| grid.cells[*index] == MapCell::Tree),
            "the fixture must preserve its earlier continuing-forest cells before feathering"
        );
        let water_band = crate::h3::h3_raster_sample_band(
            &plan,
            &grid,
            sample_coordinates[sample_coordinates.len() / 2],
        )
        .expect("water sample band");
        let road_band = crate::h3::h3_raster_sample_band(
            &plan,
            &grid,
            sample_coordinates[sample_coordinates.len() * 3 / 4],
        )
        .expect("road sample band");
        set_cell(
            &mut grid,
            i32::from(water_band[0].0),
            i32::from(water_band[0].1),
            MapCell::Water,
        );
        set_cell(
            &mut grid,
            i32::from(road_band[0].0),
            i32::from(road_band[0].1),
            MapCell::MajorRoad,
        );

        author_h3_boundary(&mut grid, &seams).expect("feather boundary");

        let profile = crate::build_h3_grid_seam_profile(&grid).expect("final seam profile");
        let edge = profile
            .edges
            .iter()
            .find(|edge| edge.edge_id == portal.edge_id)
            .expect("target edge");
        let outline_samples = edge
            .samples
            .iter()
            .filter(|sample| {
                sample.surface == crate::H3GridSeamSurface::Tree
                    && sample.inner_surface != crate::H3GridSeamSurface::Tree
            })
            .count();
        assert!(
            outline_samples * 3 < edge.samples.len() * 2,
            "one-cell canopy outline survived at {outline_samples}/{} edge samples",
            edge.samples.len()
        );
        assert!(
            interior_forest
                .iter()
                .all(|index| grid.cells[*index] == MapCell::Tree),
            "feathering must not consume forest that continues into the face"
        );
        assert_eq!(
            grid.cell(water_band[0].0, water_band[0].1),
            Some(MapCell::Water)
        );
        assert_eq!(
            grid.cell(road_band[0].0, road_band[0].1),
            Some(MapCell::MajorRoad)
        );
    }

    #[test]
    fn h3_generation_keeps_atomic_stamps_inside_and_opens_only_real_crossings() {
        let coordinate = Coordinate {
            lat: 44.947_519_6,
            lon: -93.325_347_7,
        };
        let plan = plan_h3_cell(coordinate, 8).expect("H3 plan");
        let crossed_portal = plan.portals[0].clone();
        let neighbor = plan_h3_cell(crossed_portal.midpoint, 8)
            .ok()
            .filter(|candidate| candidate.cell == crossed_portal.neighbor)
            .map(|candidate| candidate.center)
            .unwrap_or_else(|| {
                let neighbor_index = crossed_portal
                    .neighbor
                    .parse::<h3o::CellIndex>()
                    .expect("neighbor index");
                let center = h3o::LatLng::from(neighbor_index);
                Coordinate {
                    lat: center.lat(),
                    lon: center.lng(),
                }
            });
        let raw = MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds[0],
            attribution: "H3 generation fixture".to_string(),
            features: vec![
                Feature {
                    kind: FeatureKind::MajorRoad,
                    name: Some("one real crossing".to_string()),
                    area: false,
                    bridge: false,
                    points: vec![plan.center, neighbor],
                },
                Feature {
                    kind: FeatureKind::Building,
                    name: Some("owned home".to_string()),
                    area: true,
                    bridge: false,
                    points: vec![plan.center],
                },
            ],
            h3: None,
        };
        let source = prepare_h3_source(raw, plan.clone()).expect("owned source");
        let grid = generate_grid(source, 64, 64).expect("H3 grid");

        let atomic_cells = grid
            .cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| {
                matches!(
                    cell,
                    MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast
                        | MapCell::LedgeWest
                        | MapCell::LedgeMiddle
                        | MapCell::LedgeEast
                        | MapCell::CliffNorthWest
                        | MapCell::CliffNorth
                        | MapCell::CliffNorthEast
                        | MapCell::CliffWest
                        | MapCell::CliffCenter
                        | MapCell::CliffEast
                        | MapCell::CliffSouthWest
                        | MapCell::CliffSouth
                        | MapCell::CliffSouthEast
                        | MapCell::CliffInnerSouthWest
                        | MapCell::CliffInnerSouthEast
                        | MapCell::CliffStairs
                )
            })
            .map(|(index, _)| {
                (
                    (index % usize::from(grid.width)) as u16,
                    (index / usize::from(grid.width)) as u16,
                )
            })
            .collect::<Vec<_>>();
        assert!(!atomic_cells.is_empty());
        assert!(atomic_cells.iter().all(|&(x, y)| {
            plan.raster_contains_cell(x, y, grid.width, grid.height)
                .expect("hex containment")
        }));

        let contract = build_h3_seam_contract(&plan, &grid.source, 64, 64).expect("seam contract");
        let transport_edges = contract
            .edges
            .iter()
            .filter(|edge| edge.transport.is_some())
            .collect::<Vec<_>>();
        assert_eq!(transport_edges.len(), 1);
        assert_eq!(transport_edges[0].edge_id, crossed_portal.edge_id);
    }

    #[test]
    fn h3_generation_runs_at_polar_longitudes_without_utm() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 89.9,
                lon: 179.9,
            },
            5,
        )
        .expect("polar H3 plan");
        let source = prepare_h3_source(
            MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "polar generation fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            plan,
        )
        .expect("polar source");
        let grid = generate_grid(source, 64, 64).expect("polar H3 generation");
        assert!(grid.cells.contains(&MapCell::H3Void));
    }

    #[test]
    fn ordinary_house_stamp_has_a_real_canonical_door() {
        let mut grid = GeneratedGrid {
            source: source(Vec::new()),
            width: 8,
            height: 8,
            cells: vec![MapCell::Grass; 8 * 8],
            labels: Vec::new(),
        };
        for dy in 0..2 {
            for dx in 0..2 {
                set_cell(&mut grid, 3 + dx, 2 + dy, MapCell::Building);
            }
        }

        let blocks = grid.crystal_blocks();
        let at = |x: usize, y: usize| blocks[y * usize::from(grid.width) + x];
        assert_eq!(
            [[at(3, 2), at(4, 2)], [at(3, 3), at(4, 3)]],
            [[0x18, 0x19], [0x16, 0x1e]],
            "ordinary houses must use the canonical two-block facade whose southwest block contains the door"
        );
    }

    #[test]
    fn dense_city_landmarks_use_exact_goldenrod_stamps() {
        let mut store = GeneratedGrid {
            source: source(Vec::new()),
            width: 8,
            height: 8,
            cells: vec![MapCell::Grass; 8 * 8],
            labels: Vec::new(),
        };
        for y in 1..5 {
            for x in 2..5 {
                set_cell(&mut store, x, y, MapCell::Building);
            }
        }
        let blocks = store.crystal_blocks();
        let at = |x: usize, y: usize| blocks[y * 8 + x];
        assert_eq!(
            (1..5)
                .map(|y| (2..5).map(|x| at(x, y)).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![
                vec![0x18, 0x1f, 0x19],
                vec![0x27, 0x23, 0x28],
                vec![0x27, 0x23, 0x28],
                vec![0x10, 0x17, 0x33],
            ]
        );

        let mut tower = GeneratedGrid {
            source: source(Vec::new()),
            width: 8,
            height: 8,
            cells: vec![MapCell::Grass; 8 * 8],
            labels: Vec::new(),
        };
        for y in 2..8 {
            for x in 3..5 {
                if y < 5 && x == 4 {
                    continue;
                }
                set_cell(&mut tower, x, y, MapCell::Building);
            }
        }
        let blocks = tower.crystal_blocks();
        let at = |x: usize, y: usize| blocks[y * 8 + x];
        assert_eq!(
            [
                at(3, 2),
                at(3, 3),
                at(3, 4),
                at(3, 5),
                at(4, 5),
                at(3, 6),
                at(4, 6),
                at(3, 7),
                at(4, 7),
            ],
            [0x21, 0x22, 0x22, 0x25, 0x26, 0x29, 0x2a, 0x2d, 0x2e]
        );
    }

    #[test]
    fn broad_climbable_cliffs_can_replace_dense_forest() {
        let mut grid = GeneratedGrid {
            source: source(Vec::new()),
            width: 96,
            height: 96,
            cells: vec![MapCell::Tree; 96 * 96],
            labels: Vec::new(),
        };
        for x in 0..grid.width {
            set_cell(&mut grid, i32::from(x), 48, MapCell::MajorRoad);
        }
        for y in 0..grid.height {
            set_cell(&mut grid, 48, i32::from(y), MapCell::Road);
        }
        let trees_before = grid
            .cells
            .iter()
            .filter(|cell| **cell == MapCell::Tree)
            .count();

        place_landmark_features(&mut grid).expect("place broad contours");

        let cliff_cells = grid
            .cells
            .iter()
            .filter(|cell| {
                matches!(
                    cell,
                    MapCell::CliffNorthWest
                        | MapCell::CliffNorth
                        | MapCell::CliffNorthEast
                        | MapCell::CliffWest
                        | MapCell::CliffCenter
                        | MapCell::CliffEast
                        | MapCell::CliffSouthWest
                        | MapCell::CliffSouth
                        | MapCell::CliffSouthEast
                        | MapCell::CliffInnerSouthWest
                        | MapCell::CliffInnerSouthEast
                        | MapCell::CliffStairs
                )
            })
            .count();
        let trees_after = grid
            .cells
            .iter()
            .filter(|cell| **cell == MapCell::Tree)
            .count();
        assert_eq!(
            cliff_cells, 226,
            "four complete broad contours are required"
        );
        assert_eq!(
            grid.cells
                .iter()
                .filter(|cell| **cell == MapCell::CliffStairs)
                .count(),
            4,
            "each forest-replacing formation needs a real climbable stair"
        );
        assert!(
            trees_before - trees_after >= cliff_cells,
            "the cliff footprints must be allowed to consume dense forest"
        );
    }

    #[test]
    fn plateau_catalog_contains_only_complete_canonical_contours() {
        let expectations = [
            (PlateauTemplate::ExpandedCompact, 7, 4, 28, 0),
            (PlateauTemplate::ExpandedWideLeft, 9, 5, 45, 0),
            (PlateauTemplate::ExpandedWideRight, 9, 5, 45, 0),
            (PlateauTemplate::ExpandedDeep, 7, 6, 42, 0),
            (PlateauTemplate::ExpandedGrand, 11, 5, 55, 0),
            (PlateauTemplate::SteppedCompact, 7, 5, 27, 1),
            (PlateauTemplate::SteppedLeft, 9, 5, 37, 1),
            (PlateauTemplate::SteppedRight, 9, 5, 37, 1),
            (PlateauTemplate::SteppedDeepLeft, 9, 7, 51, 1),
            (PlateauTemplate::SteppedDeepRight, 9, 7, 51, 1),
            (PlateauTemplate::SteppedGrand, 11, 6, 58, 1),
        ];
        for (template, expected_width, expected_height, expected_cells, expected_inners) in
            expectations
        {
            let stamp = plateau_stamp(template);
            assert_eq!(
                (stamp.width(), stamp.height()),
                (expected_width, expected_height)
            );
            assert_eq!(stamp.stair_y + 1, stamp.height());
            assert_eq!(
                stamp.cells[usize::from(stamp.stair_y)][usize::from(stamp.stair_x)],
                Some(MapCell::CliffStairs)
            );
            assert!(
                stamp
                    .cells
                    .iter()
                    .all(|row| row.len() == usize::from(expected_width))
            );
            let authored = stamp
                .cells
                .iter()
                .flatten()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            assert_eq!(authored.len(), expected_cells);
            assert!(authored.iter().all(|cell| is_cliff_cell(*cell)));
            assert_eq!(
                authored
                    .iter()
                    .filter(|cell| **cell == MapCell::CliffStairs)
                    .count(),
                1
            );
            assert_eq!(
                authored
                    .iter()
                    .filter(|cell| **cell == MapCell::CliffInnerSouthWest)
                    .count(),
                expected_inners
            );
            assert_eq!(
                authored
                    .iter()
                    .filter(|cell| **cell == MapCell::CliffInnerSouthEast)
                    .count(),
                expected_inners
            );

            let mut grid = GeneratedGrid {
                source: source(Vec::new()),
                width: 20,
                height: 20,
                cells: vec![MapCell::Grass; 20 * 20],
                labels: Vec::new(),
            };
            for (dy, row) in stamp.cells.iter().enumerate() {
                for (dx, cell) in row.iter().enumerate() {
                    if let Some(cell) = cell {
                        set_cell(&mut grid, 2 + dx as i32, 2 + dy as i32, *cell);
                    }
                }
            }
            let recognized = canonical_plateau_contours(&grid);
            assert_eq!(
                recognized.len(),
                1,
                "catalog failed to recognize {template:?}"
            );
            assert_eq!(recognized[0].cliff_cells, expected_cells);
            assert_eq!(recognized[0].stairs, 1);
            assert_eq!(recognized[0].inner_west, expected_inners);
            assert_eq!(recognized[0].inner_east, expected_inners);
        }

        let mirror_cell = |cell: Option<MapCell>| {
            cell.map(|cell| match cell {
                MapCell::CliffNorthWest => MapCell::CliffNorthEast,
                MapCell::CliffNorthEast => MapCell::CliffNorthWest,
                MapCell::CliffWest => MapCell::CliffEast,
                MapCell::CliffEast => MapCell::CliffWest,
                MapCell::CliffSouthWest => MapCell::CliffSouthEast,
                MapCell::CliffSouthEast => MapCell::CliffSouthWest,
                MapCell::CliffInnerSouthWest => MapCell::CliffInnerSouthEast,
                MapCell::CliffInnerSouthEast => MapCell::CliffInnerSouthWest,
                other => other,
            })
        };
        for (west, east) in [
            (
                PlateauTemplate::ExpandedWideLeft,
                PlateauTemplate::ExpandedWideRight,
            ),
            (PlateauTemplate::SteppedLeft, PlateauTemplate::SteppedRight),
            (
                PlateauTemplate::SteppedDeepLeft,
                PlateauTemplate::SteppedDeepRight,
            ),
        ] {
            let mirrored = plateau_stamp(west)
                .cells
                .into_iter()
                .map(|row| row.into_iter().rev().map(mirror_cell).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            assert_eq!(mirrored, plateau_stamp(east).cells);
        }
    }

    #[test]
    fn neighboring_h3_faces_choose_varied_complete_cliff_contours() {
        let center = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            6,
        )
        .expect("Minneapolis H3 plan");
        let plans = std::iter::once(center.clone())
            .chain(center.portals.iter().map(|portal| {
                let index = portal
                    .neighbor
                    .parse::<h3o::CellIndex>()
                    .expect("neighbor H3 index");
                let coordinate = h3o::LatLng::from(index);
                plan_h3_cell(
                    Coordinate {
                        lat: coordinate.lat(),
                        lon: coordinate.lng(),
                    },
                    6,
                )
                .expect("neighbor H3 plan")
            }))
            .collect::<Vec<_>>();
        let mut face_signatures = std::collections::BTreeSet::new();

        for plan in plans {
            let mut grid = GeneratedGrid {
                source: MapSource {
                    center: plan.center,
                    bounds: plan.fetch_bounds[0],
                    attribution: "neighboring contour fixture".to_string(),
                    features: Vec::new(),
                    h3: Some(plan),
                },
                width: 64,
                height: 64,
                cells: vec![MapCell::Tree; 64 * 64],
                labels: Vec::new(),
            };
            for x in 0..grid.width {
                set_cell(&mut grid, i32::from(x), 32, MapCell::MajorRoad);
            }
            for y in 0..grid.height {
                set_cell(&mut grid, 32, i32::from(y), MapCell::Road);
            }

            place_landmark_features(&mut grid).expect("place varied contours");

            let cliff_indices = grid
                .cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| {
                    matches!(
                        cell,
                        MapCell::CliffNorthWest
                            | MapCell::CliffNorth
                            | MapCell::CliffNorthEast
                            | MapCell::CliffWest
                            | MapCell::CliffCenter
                            | MapCell::CliffEast
                            | MapCell::CliffSouthWest
                            | MapCell::CliffSouth
                            | MapCell::CliffSouthEast
                            | MapCell::CliffInnerSouthWest
                            | MapCell::CliffInnerSouthEast
                            | MapCell::CliffStairs
                    )
                    .then_some(index)
                })
                .collect::<std::collections::BTreeSet<_>>();
            let mut signatures = indexed_components(&cliff_indices, grid.width, grid.height)
                .into_iter()
                .map(|component| {
                    let min_x = component
                        .iter()
                        .map(|index| index % usize::from(grid.width))
                        .min()
                        .expect("cliff x");
                    let min_y = component
                        .iter()
                        .map(|index| index / usize::from(grid.width))
                        .min()
                        .expect("cliff y");
                    let mut normalized = component
                        .into_iter()
                        .map(|index| {
                            let x = index % usize::from(grid.width);
                            let y = index / usize::from(grid.width);
                            (x - min_x, y - min_y, grid.crystal_blocks()[index])
                        })
                        .collect::<Vec<_>>();
                    normalized.sort_unstable();
                    normalized
                })
                .collect::<Vec<_>>();
            signatures.sort_unstable();
            assert_eq!(
                signatures.len(),
                2,
                "each roomy H3 face should retain two complete landmark contours"
            );
            let cliff_cells = signatures.iter().map(Vec::len).sum::<usize>();
            assert!(
                (79..=113).contains(&cliff_cells),
                "an unobstructed H3 face should use its substantial primary contours, found {cliff_cells} cells"
            );
            assert_eq!(
                grid.cells
                    .iter()
                    .filter(|cell| **cell == MapCell::CliffStairs)
                    .count(),
                2,
                "each contour must use one real Slowpoke Well stair metatile"
            );
            face_signatures.insert(signatures);
        }

        assert!(
            face_signatures.len() >= 3,
            "neighboring H3 faces must not repeat one identical pair of cliff silhouettes"
        );
    }

    #[test]
    fn plateau_planner_reserves_the_future_home_yard() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            7,
        )
        .expect("H3 plan");
        let grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "home reservation fixture".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Tree; 64 * 64],
            labels: Vec::new(),
        };

        assert!(
            !plateau_site_is_clear(&grid, 28, 26, 7, 5),
            "the stepped contour must not occupy cells later replaced by the protected home"
        );
        assert!(
            (4..52).any(|y| {
                (4..53).any(|x| {
                    !rectangles_overlap((x - 1, y - 1, x + 7, y + 5), (30, 28, 35, 34))
                        && plateau_site_is_clear(&grid, x as u16, y as u16, 7, 5)
                })
            }),
            "reserving the home yard must not reject unrelated forest sites"
        );
    }

    #[test]
    fn h3_boulder_completion_never_punches_holes_in_canopy() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            7,
        )
        .expect("H3 plan");
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "canopy fixture".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Tree; 64 * 64],
            labels: Vec::new(),
        };

        ensure_h3_rock_formations(&mut grid);

        assert!(grid.cells.iter().all(|cell| *cell == MapCell::Tree));
    }

    #[test]
    fn h3_forest_density_repeats_layers_until_it_has_an_audit_margin() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            7,
        )
        .expect("H3 plan");
        let source = MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds[0],
            attribution: "forest density fixture".to_string(),
            features: Vec::new(),
            h3: Some(plan.clone()),
        };
        let seams = build_h3_seam_contract(&plan, &source, 64, 64).expect("seams");
        let mut grid = GeneratedGrid {
            source,
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        author_h3_boundary(&mut grid, &seams).expect("H3 mask");

        ensure_h3_forest_density(&mut grid).expect("forest density");

        let authored_land = grid
            .cells
            .iter()
            .filter(|cell| **cell != MapCell::H3Void)
            .count();
        let canopy = grid
            .cells
            .iter()
            .filter(|cell| {
                matches!(
                    cell,
                    MapCell::Tree
                        | MapCell::ParkTree
                        | MapCell::SmallTree
                        | MapCell::SmallTreeSouth
                )
            })
            .count();
        assert!(
            canopy * 100 >= authored_land * 20,
            "H3 forest top-up stopped at {canopy}/{authored_land} cells without an audit margin"
        );
    }

    #[test]
    fn cached_regional_forest_shortfall_accepts_a_safe_interior_top_up() {
        // Category-preserving capture of the failed regional cell: 422 canopy
        // cells over 2,388 authored land cells (17.7%). It retains the exact
        // H3 mask, water, route, facility, wild, relief, and candidate-ground
        // topology while collapsing decorative variants within each family.
        let mut plan = plan_h3_cell(
            Coordinate {
                lat: 44.936_065_341_924_035,
                lon: -93.333_757_200_143_5,
            },
            6,
        )
        .expect("H3 plan");
        plan.regional = Some(crate::H3RegionalCellPlan {
            ordinal: 0,
            cell: plan.cell.clone(),
            building_count: 29_102,
            facilities: vec![H3Facility::PokemonCenter],
            connections: Vec::new(),
            closed_transport_crossings: Vec::new(),
        });
        let cells = include_str!("../tests/fixtures/h3-regional-canopy-shortfall.txt")
            .lines()
            .flat_map(|row| {
                assert_eq!(row.len(), 64);
                row.bytes().map(|symbol| match symbol {
                    b't' => MapCell::ParkTree,
                    b'g' => MapCell::Grass,
                    b'c' => MapCell::Clearing,
                    b'r' => MapCell::Trail,
                    b'v' => MapCell::H3Void,
                    b'w' => MapCell::Water,
                    b'b' => MapCell::Building,
                    b'f' => MapCell::PokecenterNorthWest,
                    b'l' => MapCell::CliffCenter,
                    b'p' => MapCell::Park,
                    b'x' => MapCell::Boulder,
                    _ => panic!("unknown canopy fixture symbol"),
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(cells.len(), 64 * 64);
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "Minneapolis regional canopy regression".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells,
            labels: Vec::new(),
        };
        let before = grid.cells.clone();
        let reached_before = reachable_walkable_cells(&grid, grid.home_cell());
        let authored_land = grid
            .cells
            .iter()
            .filter(|cell| {
                !matches!(
                    **cell,
                    MapCell::H3Void
                        | MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                )
            })
            .count();
        let canopy_before = grid
            .cells
            .iter()
            .filter(|cell| h3_canopy_cell(**cell))
            .count();
        assert_eq!(authored_land, 2_388);
        assert_eq!(canopy_before, 422);

        top_up_h3_interior_canopy(&mut grid).expect("interior canopy top-up");

        let canopy = grid
            .cells
            .iter()
            .filter(|cell| h3_canopy_cell(**cell))
            .count();
        let target = authored_land.saturating_mul(19).div_ceil(100);
        assert!(canopy >= target, "top-up stopped at {canopy}/{target}");
        let reached_after = reachable_walkable_cells(&grid, grid.home_cell());
        for (index, (old, new)) in before.iter().zip(&grid.cells).enumerate() {
            if old != new {
                assert!(
                    matches!(old, MapCell::Grass | MapCell::Lawn),
                    "overwrote {old:?} at {index}"
                );
                assert_eq!(*new, MapCell::Tree);
                let x = (index % 64) as u16;
                let y = (index / 64) as u16;
                assert!(
                    grid.source
                        .h3
                        .as_ref()
                        .unwrap()
                        .raster_footprint_fits(
                            i32::from(x),
                            i32::from(y),
                            1,
                            1,
                            3,
                            grid.width,
                            grid.height
                        )
                        .unwrap(),
                    "new canopy touched the H3 join at ({x},{y})"
                );
                assert!(!dense_canopy_run_exceeds(&grid, x, y, 10));
            }
        }
        assert!(
            reached_before.iter().enumerate().all(|(index, reached)| {
                !*reached || !is_walkable_cell(grid.cells[index]) || reached_after[index]
            }),
            "interior canopy top-up disconnected a previously reachable cell"
        );
    }

    #[test]
    fn h3_ledge_planner_backtracks_when_a_greedy_terrace_blocks_the_last_run() {
        // Compact category-preserving capture of cell 86262cd27ffffff immediately
        // before ledge planning. `g` is replaceable natural ground, `r` is the
        // route graph, and the remaining symbols preserve the obstacle behavior
        // relevant to reservation/frontage/path checks. The old greedy planner
        // chose (18,25), then (16,38), and exhausted every site for formation 3.
        const PRE_LEDGE: &str = "\
gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg\n\
ggggggggggggggggggggggggggggggggggggggwwwggggggggggggggggggggggg\n\
ggggggggggggggggggggggggppppgpppgggggwwwgggggggggggggggggggggggg\n\
gggggggggggggggggggggggpppppppppgggggwwwgggggggggggggggggggggggg\n\
gggggggggggggggggggggggpgggggggggggggggwwwwwwggggggggggggggggggg\n\
gggggggggggggggggggggggpppppppppgggggggwwwwwwwgggggggggggggggggg\n\
gggggggggggggggggggggggpppppgppprgggggwwwwwwwwpgggpggwpgpgpggggg\n\
ggggggggggggggggggggggggggggggggrgggggwwwwwwwgggpggggwwggggggggg\n\
gggggggggggggggggggggggxxxxxxxxxrgggggwwwwwwwggggggggwwwgpgggggg\n\
ggggggggggggggggggggggxxxggpxppxrggggwwwwwwwwgggpggggwwwgggggggg\n\
gggggggggggggggggggggggxpppppppxrggggwwwwwwwwggggwwggwwggpgpgggg\n\
gggggggggggggggggggggggxppxpprrrrggggwwwwwwwggggwwwwwwwggggggggg\n\
gggggggggggggggggggggggxpppppppxrgggwwwwwwwwggggwwwwwwwggggggggg\n\
gggggggggggggggggggggggxppppggpxrrwwwwwwwwwwwwwwwwwwwwwwgggggggg\n\
gggggggggggggggggggggggxxxxxxxxxrggwwwwwwwwggwwwwwwwwwwwgggggggg\n\
ggggggggggggggggggggggggggggggggrxggggwwwwwgggggwwwwwwwwgggggggg\n\
gggggggggggggggggggggggggbbggbbgrgggggggggggggpgwwwwwwwggggggggg\n\
gggggggggggggggggggggggggbbggbbgrgggggbbggggpgggggwwwwgggggggggg\n\
gggggggggggggggggggggggggrrrrrrrrgggggbbggggggggggwwwggggggggggg\n\
gggggggggggggggppppppgppggggggggrrrrrrrgggggpgwwwwwwwpgpgggggggg\n\
gggggggggggggggpppppppppggggggggrgggggggggggwwwwwwwwwggggggggggg\n\
gggggggggggggggpggggggggggggggggrggggggggggwwwwwwwwwwggpgpgggggg\n\
gggggggggggggggppppppppprrrrrrrrrggggggggggwwwwwwwwwwggggggggggg\n\
gggggggggggggggggpppppgpggggggggrbbgggggggwwwwwwwwwwwwggggpggggg\n\
ggggggggggggggggggggggggggggggggrbbgggggggwwwwwwwwwwwwgggggggggg\n\
ggggggggggggggggggggggggggggggggrrggggggggwwwwwwwwwwwwgggggggggg\n\
ggggggggggggggggggggggggggggwwggrgggggggggwwwwwwwwwwwwwggggggggg\n\
ggggggggggggggggxxxxxxxggggwwwggrgggggggggwwwwwwwwwwwwgggggggggg\n\
ggggggggggggggggxxxxxxxggwwwwwwwrrrgggggggwwwwwwwwwwwwgggggggggg\n\
ggggggggggggggggxxxxxxxggwwwwwggbbrgggggggwwwwwwwwwwwwgggggggggg\n\
ggggggggggggggggxxxxxxxgggggwgggbbrggggggwwwwwwwwwwwwwgggggggggg\n\
gggggggggggggggggggrrrrrrrrrrrrrrrrggggggwgwwwwwwwwwwwgggggggggg\n\
gggggggggggggggggggggggggggggggxrggggggggwgwwwwwwwwwwwgpgpgggggg\n\
ggggggggggggggggggpgpgggggggggggrggggggggwwggggwwwwwwwpggggpgggg\n\
ggggggggggggggggpggggpggggggggpgrggggggggggggggggwgwwwgggggggggg\n\
gggggggggwggggggggggggggggbbggggrggggggggggggggggwwwwwpggggpgggg\n\
ggggwwwggwwggggggpggpgggggbbggggrggbbggxxxxxxxgggggggggggpgggggg\n\
wwwwwwwwwwwgggggggggggggggrrrrrrrggbbggxxxxxxxgggggggggggggggggg\n\
gwwwwgggwwwgggggggggggggggggggggrgrrgggxxxxxxxgggggggwgggggggggg\n\
gwwwgggggwwggggggggggggggpgggpggrrrggggggxxxggggggwwwwwwwggggggg\n\
ggwwggggggwggggggggggggggppppppppprggggggxxxgggggwwwwwwwwwwggggg\n\
gggggwwgggwggggbbggggggggppppppppprggggggrrggggggwwwwwwwwwwggggg\n\
gwwwwwwggwwwgggbbggggggggpgpgggggxrffgffgrggggbbgwwwwwwwwwwwgggg\n\
gwwwwwwwwwwwwggggggggggggppppppppprffgffgrbbggbbwwwwwwwwwwwwgggg\n\
gggggggwwwwwwggggggggggggppppppprrrrrrrggrbbrrrgwwwwwwwwwwwwgggg\n\
gggggggwwwwwwggggwwwggggggggggggrxgxggrrrrrrrggwwwwwwwwwwwwggggg\n\
gggggggggwwwwwwgwwwwggggggggggggrgggggggggggggggwwwwwwwwwwwggggg\n\
gggggggggwgwwwwbbwwwbbggggggggggrgggggggggggggwwwwwwwwwwwwgggggg\n\
gggggggggggggggbbwgwbbgggggbbgggrgggggggggggggwwwwwwwwwwwwgggggg\n\
wggggggggggggggrrwwwwwwwwggbbgggrgggbbgggggggggwwwwwwwwwwwgggggg\n\
wwggggggggggggggrrwwwwwwwggrrgggrgggbbgggggggbbgwwwwwwwwgggggggg\n\
gwwggggggggggggggrrrwggwwgggrgggrgggrggggggggbbgggwwwwgggggggggg\n\
gwwwgggggggggggggggrbbgwwgggrgggrgggrggggrrrrrrrrrrxgggggggggggg\n\
wwwwwggggggggggggggrbbgwwwggrrrrrrrrrrrrrrggggggggrrgggggggggggg\n\
wwwwwggwwggggggggggrrgggwggrrggrgggggggggggggggggggrrggggggggggg\n\
gggwwwwwwwgggggggggrrggggggrgggrbbggggggggggggggggggggwwwwwggggg\n\
gggggggggggggggggggrrrrrrrrrgggrbbggggggggggggggggggwwwwwwwwgggg\n\
gggggggggggggggggggrgggggggggggrrggggggggggggggggggggwgggggwwwwg\n\
gggggggggggggggggggrgwwgggggggggggggggggggggggggwwwwgwgggggwwwww\n\
gggggggggggggggggggrggwwwggggggggggggggggggggggwwggggggggggggggg\n\
ggggggggggggggggggggggwwwwgggggggggggggggwwwwwwwgggggggggggggggg\n\
ggggggggggggggggggggwwwwggggggggggggggggwwgggggggggggggggggggggg\n\
gggggggggggggggggggggwwwgggggggggggggggwwggggggggggggggggggggggg\n\
gggggggggggggggggggggwwwgggggggggggggwwwwggggggggggggggggggggggg";
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.936_065_341_924_035,
                lon: -93.333_757_200_143_5,
            },
            6,
        )
        .expect("H3 plan");
        assert_eq!(plan.cell, "86262cd27ffffff");
        let cells = PRE_LEDGE
            .lines()
            .flat_map(|row| {
                assert_eq!(row.len(), 64);
                row.bytes().map(|symbol| match symbol {
                    b'g' => MapCell::Grass,
                    b'r' => MapCell::Trail,
                    b'b' => MapCell::Building,
                    b'f' => MapCell::PokecenterNorthWest,
                    b'w' => MapCell::Water,
                    b'p' => MapCell::Park,
                    b'x' => MapCell::Bench,
                    _ => panic!("unknown fixture symbol"),
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(cells.len(), 64 * 64);
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "Minneapolis ledge regression".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells,
            labels: Vec::new(),
        };

        place_large_ledge_runs(&mut grid).expect("globally feasible H3 ledge plan");

        // Count complete horizontal runs from their canonical west/east caps;
        // components by cell kind would split the middle and cap metatiles.
        let mut runs = Vec::<(u16, u16, u16)>::new();
        for y in 0..grid.height {
            let mut x = 0;
            while x < grid.width {
                if grid.cell(x, y) != Some(MapCell::LedgeWest) {
                    x += 1;
                    continue;
                }
                let start = x;
                x += 1;
                while grid.cell(x, y) == Some(MapCell::LedgeMiddle) {
                    x += 1;
                }
                assert_eq!(grid.cell(x, y), Some(MapCell::LedgeEast));
                runs.push((start, y, x - start + 1));
                x += 1;
            }
        }
        let mut lengths = runs.iter().map(|run| run.2).collect::<Vec<_>>();
        lengths.sort_unstable();
        assert_eq!(lengths, vec![7, 8, 9]);
        for &(start, y, length) in &runs {
            for x in start..start + length {
                assert!(
                    matches!(grid.cell(x, y - 1), Some(MapCell::Lawn | MapCell::Trail)),
                    "ledge approach at ({x},{}) is obstructed",
                    y - 1
                );
                assert_eq!(
                    grid.cell(x, y + 1),
                    Some(MapCell::Lawn),
                    "ledge landing at ({x},{}) is obstructed",
                    y + 1
                );
            }
            assert!(is_walkable_cell(
                grid.cell(start - 1, y).expect("west bypass")
            ));
            assert!(is_walkable_cell(
                grid.cell(start + length, y).expect("east bypass")
            ));
        }
    }

    #[test]
    fn large_ledge_runs_are_complete_spaced_and_keep_clear_landings() {
        let mut first = GeneratedGrid {
            source: source(Vec::new()),
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        for x in 0..first.width {
            set_cell(&mut first, i32::from(x), 32, MapCell::MajorRoad);
        }
        for y in 0..first.height {
            set_cell(&mut first, 32, i32::from(y), MapCell::Road);
        }
        let mut second = first.clone();
        place_large_ledge_runs(&mut first).expect("first ledge plan");
        place_large_ledge_runs(&mut second).expect("repeat ledge plan");
        assert_eq!(
            first.cells, second.cells,
            "ledge selection must be deterministic"
        );

        let mut runs = Vec::new();
        for y in 0..first.height {
            let mut x = 0;
            while x < first.width {
                if first.cell(x, y) != Some(MapCell::LedgeWest) {
                    x += 1;
                    continue;
                }
                let origin = x;
                x += 1;
                while first.cell(x, y) == Some(MapCell::LedgeMiddle) {
                    x += 1;
                }
                assert_eq!(
                    first.cell(x, y),
                    Some(MapCell::LedgeEast),
                    "ledge at ({origin},{y}) has no canonical east cap"
                );
                runs.push((origin, y, x - origin + 1));
                x += 1;
            }
        }
        let mut lengths = runs.iter().map(|run| run.2).collect::<Vec<_>>();
        lengths.sort_unstable();
        assert_eq!(lengths, vec![6, 7, 7, 8, 9]);
        assert_eq!(
            first
                .cells
                .iter()
                .filter(|cell| **cell == MapCell::LedgeEast)
                .count(),
            5,
            "every west cap must have exactly one east cap"
        );

        let compound_pairs = runs
            .iter()
            .enumerate()
            .filter(|(index, first_run)| {
                runs.iter().skip(index + 1).any(|second_run| {
                    let vertical_gap = first_run.1.abs_diff(second_run.1);
                    let horizontal_offset = first_run.0.abs_diff(second_run.0);
                    (3..=5).contains(&vertical_gap)
                        && (1..=3).contains(&horizontal_offset)
                        && first_run.0 < second_run.0 + second_run.2
                        && second_run.0 < first_run.0 + first_run.2
                })
            })
            .count();
        assert!(
            compound_pairs >= 2,
            "expected two visibly stepped ledge terraces, found {compound_pairs}"
        );

        let blocks = first.crystal_blocks();
        for &(origin_x, y, length) in &runs {
            for offset in 0..length {
                let index =
                    usize::from(y) * usize::from(first.width) + usize::from(origin_x + offset);
                let expected = match offset {
                    0 => 0x52,
                    value if value + 1 == length => 0x53,
                    _ => 0x57,
                };
                assert_eq!(blocks[index], expected);
                assert!(
                    matches!(
                        first.cell(origin_x + offset, y - 1),
                        Some(MapCell::Lawn | MapCell::Trail)
                    ),
                    "ledge approach at ({},{}) is obstructed",
                    origin_x + offset,
                    y - 1
                );
                assert_eq!(
                    first.cell(origin_x + offset, y + 1),
                    Some(MapCell::Lawn),
                    "ledge landing at ({},{}) is not reserved",
                    origin_x + offset,
                    y + 1
                );
            }
            assert!(is_walkable_cell(
                first.cell(origin_x - 1, y).expect("west bypass")
            ));
            assert!(is_walkable_cell(
                first.cell(origin_x + length, y).expect("east bypass")
            ));
        }
        let reached = reachable_walkable_cells(&first, first.home_cell());
        assert!(
            first
                .cells
                .iter()
                .zip(reached)
                .all(|(cell, reached)| !is_walkable_cell(*cell) || reached),
            "ledge runs must leave every ordinary walkable cell connected"
        );

        place_irregular_wild_infill(&mut first).expect("post-ledge wild infill");
        for &(origin_x, y, length) in &runs {
            for offset in 0..length {
                assert_ne!(first.cell(origin_x + offset, y - 1), Some(MapCell::Park));
                assert_ne!(first.cell(origin_x + offset, y + 1), Some(MapCell::Park));
            }
        }
    }

    #[test]
    fn irregular_wild_infill_is_compact_varied_and_non_destructive() {
        let mut first = GeneratedGrid {
            source: source(Vec::new()),
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        for x in 0..first.width {
            set_cell(&mut first, i32::from(x), 32, MapCell::MajorRoad);
        }
        for y in 0..first.height {
            set_cell(&mut first, 16, i32::from(y), MapCell::Trail);
        }
        for y in 4..=9 {
            for x in 4..=9 {
                set_cell(&mut first, x, y, MapCell::Water);
            }
        }
        for dy in 0..2 {
            for dx in 0..2 {
                set_cell(&mut first, 24 + dx, 20 + dy, MapCell::Building);
            }
        }
        set_cell(&mut first, 24, 22, MapCell::Trail);
        for (dx, dy, cell) in [
            (0, 0, MapCell::PokecenterNorthWest),
            (1, 0, MapCell::PokecenterNorthEast),
            (0, 1, MapCell::PokecenterSouthWest),
            (1, 1, MapCell::PokecenterSouthEast),
        ] {
            set_cell(&mut first, 48 + dx, 18 + dy, cell);
        }
        for y in 43..=47 {
            for x in 42..=48 {
                set_cell(&mut first, x, y, MapCell::Pitch);
            }
        }
        for (x, y, cell) in [
            (41, 44, MapCell::FenceWest),
            (49, 44, MapCell::FenceEast),
            (38, 10, MapCell::Bench),
            (39, 10, MapCell::TrashCan),
            (40, 10, MapCell::Fountain),
            (52, 52, MapCell::Boulder),
            (53, 52, MapCell::GroundSign),
        ] {
            set_cell(&mut first, x, y, cell);
        }
        // Three substantive encounter rooms and a deterministic tree belt make
        // this resemble the fully authored layout seen by the final infill
        // pass, rather than an unrealistically empty grass canvas.
        for (origin_x, origin_y) in [(3, 44), (28, 4), (51, 4)] {
            for dy in 0..4 {
                for dx in 0..9 {
                    set_cell(&mut first, origin_x + dx, origin_y + dy, MapCell::Park);
                }
            }
        }
        for y in 1..first.height - 1 {
            for x in 1..first.width - 1 {
                if (x + y) % 7 == 0 && first.cell(x, y) == Some(MapCell::Grass) {
                    set_cell(&mut first, i32::from(x), i32::from(y), MapCell::Lawn);
                }
                if (u32::from(x) * 13 + u32::from(y) * 7).is_multiple_of(4)
                    && first.cell(x, y) == Some(MapCell::Grass)
                {
                    set_cell(&mut first, i32::from(x), i32::from(y), MapCell::Tree);
                }
            }
        }

        let authored = first.clone();
        let park_before = authored
            .cells
            .iter()
            .filter(|cell| **cell == MapCell::Park)
            .count();
        let reachable_before = reachable_walkable_cells(&first, first.home_cell());
        let mut second = first.clone();
        let added = place_irregular_wild_infill(&mut first).expect("first infill");
        let repeated_added = place_irregular_wild_infill(&mut second).expect("repeated infill");
        assert_eq!(added, repeated_added);
        assert_eq!(
            first.cells, second.cells,
            "wild infill must be deterministic"
        );

        for (index, cell) in authored.cells.iter().copied().enumerate() {
            if !matches!(cell, MapCell::Grass | MapCell::Lawn) {
                assert_eq!(
                    first.cells[index], cell,
                    "wild infill overwrote authored cell {index}"
                );
            }
        }
        assert_eq!(
            reachable_walkable_cells(&first, first.home_cell()),
            reachable_before,
            "Grass/Lawn to Park infill must preserve walkable connectivity"
        );

        let park_components = terrain_components(&first, MapCell::Park);
        let coverage = park_components.iter().map(Vec::len).sum::<usize>();
        assert_eq!(
            coverage - park_before,
            added,
            "reported infill must equal the net post-authoring Park increase"
        );
        assert!(
            (220..=300).contains(&coverage),
            "realistic authored map should finish with 5.4%-7.3% tall grass, found {coverage} cells"
        );
        let patches = park_components
            .into_iter()
            .filter(|component| (3..=10).contains(&component.len()))
            .collect::<Vec<_>>();
        assert!(
            (6..=30).contains(&patches.len()),
            "expected several separated infill patches, found {}",
            patches.len()
        );
        assert!(
            patches
                .iter()
                .filter(|patch| (3..=5).contains(&patch.len()))
                .count()
                >= 6,
            "infill must retain genuinely small 3-5 cell grass accents"
        );
        assert!(
            patches
                .iter()
                .filter(|patch| (6..=10).contains(&patch.len()))
                .count()
                >= 6,
            "infill must retain medium hooks, commas, zigzags, and blobs"
        );
        assert!(
            (80..=192).contains(&added),
            "realistic post-authoring infill should add substantial coverage, added {added} cells"
        );
        let mut signatures = std::collections::BTreeSet::new();
        for patch in patches {
            let min_x = patch
                .iter()
                .map(|index| index % usize::from(first.width))
                .min()
                .expect("patch x");
            let max_x = patch
                .iter()
                .map(|index| index % usize::from(first.width))
                .max()
                .expect("patch x");
            let min_y = patch
                .iter()
                .map(|index| index / usize::from(first.width))
                .min()
                .expect("patch y");
            let max_y = patch
                .iter()
                .map(|index| index / usize::from(first.width))
                .max()
                .expect("patch y");
            let bounding_area = (max_x - min_x + 1) * (max_y - min_y + 1);
            let mut signature = patch
                .iter()
                .map(|index| {
                    (
                        index % usize::from(first.width) - min_x,
                        index / usize::from(first.width) - min_y,
                    )
                })
                .collect::<Vec<_>>();
            signature.sort_unstable();
            signatures.insert(signature);
            assert!(
                (3..=10).contains(&patch.len()) && patch.len() < bounding_area,
                "infill patch must be compact and irregular: {} cells in a {bounding_area}-cell box",
                patch.len()
            );
        }
        assert!(
            signatures.len() >= 6,
            "world-stable catalog collapsed to only {} visible shape signatures",
            signatures.len()
        );
    }

    #[test]
    fn h3_compact_grass_completion_irregularizes_existing_rectangles() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            7,
        )
        .expect("H3 plan");
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "compact grass fixture".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        for y in 20..=21 {
            for x in 20..=23 {
                set_cell(&mut grid, x, y, MapCell::Park);
            }
        }

        ensure_h3_compact_wild_accents(&mut grid);

        let compact = terrain_components(&grid, MapCell::Park)
            .into_iter()
            .filter(|component| (6..=12).contains(&component.len()))
            .collect::<Vec<_>>();
        assert!(compact.len() >= 6, "completion must retain six accents");
        for component in compact {
            let min_x = component
                .iter()
                .map(|index| index % usize::from(grid.width))
                .min()
                .expect("component x");
            let max_x = component
                .iter()
                .map(|index| index % usize::from(grid.width))
                .max()
                .expect("component x");
            let min_y = component
                .iter()
                .map(|index| index / usize::from(grid.width))
                .min()
                .expect("component y");
            let max_y = component
                .iter()
                .map(|index| index / usize::from(grid.width))
                .max()
                .expect("component y");
            let bounding_area = (max_x - min_x + 1) * (max_y - min_y + 1);
            assert!(
                bounding_area > component.len(),
                "compact tall grass remained a {}-cell rectangle",
                component.len()
            );
        }
    }

    #[test]
    fn h3_substantive_wild_cap_splits_an_exact_merged_infill_shape() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 44.947_519_6,
                lon: -93.325_347_7,
            },
            6,
        )
        .expect("H3 plan");
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "exact merged wild-infill fixture".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Lawn; 64 * 64],
            labels: Vec::new(),
        };

        // Four authored rooms use the same 34-cell, bitten-corner grammar as
        // the exact Minneapolis cell. They are the substantive sites that the
        // late H3 pass must retain.
        for (origin_x, origin_y) in [(3, 3), (27, 3), (51, 3), (3, 48)] {
            for dy in 0..4 {
                for dx in 0..9 {
                    if !matches!((dx, dy), (0, 0) | (8, 3)) {
                        set_cell(&mut grid, origin_x + dx, origin_y + dy, MapCell::Park);
                    }
                }
            }
        }

        // This is the exact 23-cell fifth component observed in
        // 86262cd2fffffff after immutable infill proposals touched each other.
        let merged: [(u16, u16); 23] = [
            (28, 55),
            (28, 56),
            (29, 56),
            (29, 57),
            (29, 58),
            (29, 59),
            (29, 60),
            (30, 60),
            (28, 60),
            (27, 60),
            (27, 59),
            (28, 59),
            (28, 58),
            (30, 57),
            (31, 57),
            (31, 56),
            (32, 56),
            (32, 55),
            (33, 55),
            (33, 56),
            (33, 57),
            (34, 57),
            (32, 57),
        ];
        for &(x, y) in &merged {
            set_cell(&mut grid, i32::from(x), i32::from(y), MapCell::Park);
        }

        // Preserve the requested dense field of independent, catalog-sized
        // irregular accents. This also keeps the fixture inside the real H3
        // 230-300-cell tall-grass coverage envelope after normalization.
        let anchors = [
            (5, 14),
            (14, 14),
            (23, 14),
            (32, 14),
            (41, 14),
            (50, 14),
            (15, 28),
            (24, 28),
            (33, 28),
            (42, 28),
            (51, 28),
            (17, 40),
            (28, 40),
            (39, 40),
            (50, 40),
        ];
        for (shape_index, (anchor_x, anchor_y)) in anchors.into_iter().enumerate() {
            for (dx, dy) in irregular_wild_shape(shape_index, shape_index as u64 * 17 + 5) {
                set_cell(
                    &mut grid,
                    i32::from(anchor_x) + dx as i32,
                    i32::from(anchor_y) + dy as i32,
                    MapCell::Park,
                );
            }
        }

        let before = terrain_components(&grid, MapCell::Park);
        assert_eq!(
            before
                .iter()
                .filter(|component| component.len() >= 20)
                .count(),
            5,
            "fixture must reproduce the exact fifth substantive site"
        );
        let micro_before = before
            .iter()
            .filter(|component| (3..=10).contains(&component.len()))
            .map(|component| {
                component
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .collect::<std::collections::BTreeSet<_>>();
        let merged_indices = merged
            .into_iter()
            .map(|(x, y)| usize::from(y) * usize::from(grid.width) + usize::from(x))
            .collect::<std::collections::BTreeSet<_>>();

        cap_h3_substantive_wild_sites(&mut grid);

        let after = terrain_components(&grid, MapCell::Park);
        assert_eq!(
            after
                .iter()
                .filter(|component| component.len() >= 20)
                .count(),
            4,
            "the fifth merged infill site must no longer be substantive"
        );
        let micro_after = after
            .iter()
            .filter(|component| (3..=10).contains(&component.len()))
            .map(|component| {
                component
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            micro_before.is_subset(&micro_after),
            "normalization must not alter existing independent micro-patches"
        );
        let normalized = after
            .iter()
            .filter(|component| {
                (3..=10).contains(&component.len())
                    && component.iter().all(|index| merged_indices.contains(index))
            })
            .collect::<Vec<_>>();
        assert!(
            normalized.len() >= 2,
            "the exact merged component should remain as several encounter islands"
        );
        let coverage = after.iter().map(Vec::len).sum::<usize>();
        assert!(
            (230..=300).contains(&coverage),
            "normalization must preserve H3 tall-grass coverage, found {coverage}"
        );
        for component in after.iter().filter(|component| component.len() >= 20) {
            let min_x = component
                .iter()
                .map(|index| index % usize::from(grid.width))
                .min()
                .expect("component x");
            let max_x = component
                .iter()
                .map(|index| index % usize::from(grid.width))
                .max()
                .expect("component x");
            let min_y = component
                .iter()
                .map(|index| index / usize::from(grid.width))
                .min()
                .expect("component y");
            let max_y = component
                .iter()
                .map(|index| index / usize::from(grid.width))
                .max()
                .expect("component y");
            assert!(
                component.len() < (max_x - min_x + 1) * (max_y - min_y + 1),
                "retained authored room must remain visibly irregular"
            );
        }
    }

    #[test]
    fn irregular_wild_catalog_spans_small_and_medium_distinct_shapes() {
        let mut signatures = std::collections::BTreeSet::new();
        let mut sizes = std::collections::BTreeSet::new();
        for shape_index in 0..IRREGULAR_WILD_SHAPES.len() {
            let shape = irregular_wild_shape(shape_index, shape_index as u64 * 17 + 5);
            sizes.insert(shape.len());
            let min_x = shape.iter().map(|point| point.0).min().unwrap_or(0);
            let min_y = shape.iter().map(|point| point.1).min().unwrap_or(0);
            let mut signature = shape
                .iter()
                .map(|&(x, y)| (x - min_x, y - min_y))
                .collect::<Vec<_>>();
            signature.sort_unstable();
            let width = signature.iter().map(|point| point.0).max().unwrap_or(0) + 1;
            let height = signature.iter().map(|point| point.1).max().unwrap_or(0) + 1;
            assert!(shape.len() < (width * height) as usize);
            signatures.insert(signature);
        }
        assert_eq!(
            sizes,
            std::collections::BTreeSet::from([3, 4, 5, 6, 7, 8, 9, 10])
        );
        assert_eq!(
            signatures.len(),
            IRREGULAR_WILD_SHAPES.len(),
            "every catalog entry must have a distinct silhouette"
        );
    }

    #[test]
    fn irregular_wild_infill_uses_stable_world_cells_in_overlaps() {
        let center = Coordinate {
            lat: 44.947_519_6,
            lon: -93.325_347_7,
        };
        let shifted_center = Coordinate {
            lon: center.lon + 0.004,
            ..center
        };
        let open_source = |map_center: Coordinate| MapSource {
            center: map_center,
            bounds: BoundingBox {
                south: map_center.lat - 0.007,
                west: map_center.lon - 0.010,
                north: map_center.lat + 0.007,
                east: map_center.lon + 0.010,
            },
            attribution: "stable overlap fixture".to_string(),
            features: Vec::new(),
            h3: None,
        };
        let mut first = GeneratedGrid {
            source: open_source(center),
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        let mut shifted = GeneratedGrid {
            source: open_source(shifted_center),
            ..first.clone()
        };
        place_irregular_wild_infill(&mut first).expect("first infill");
        place_irregular_wild_infill(&mut shifted).expect("shifted infill");
        let first_world = WorldGrid::from_bounds(
            first.source.center,
            first.source.bounds,
            first.width,
            first.height,
        )
        .expect("first world grid");
        let shifted_world = WorldGrid::from_bounds(
            shifted.source.center,
            shifted.source.bounds,
            shifted.width,
            shifted.height,
        )
        .expect("shifted world grid");
        let overlap = first_world
            .intersection(shifted_world)
            .expect("shifted windows overlap");
        let mut compared = 0;
        let mut shared_tall_grass = 0;
        for world_y in overlap.south..=overlap.north {
            for world_x in overlap.west..=overlap.east {
                let world = WorldCell {
                    x: world_x,
                    y: world_y,
                };
                let (first_x, first_y) = first_world.local_cell(world).expect("first local");
                let (shifted_x, shifted_y) =
                    shifted_world.local_cell(world).expect("shifted local");
                let away_from_crop_edge =
                    |x: u16, y: u16| x >= 3 && y >= 3 && x + 3 < 64 && y + 3 < 64;
                if !away_from_crop_edge(first_x, first_y)
                    || !away_from_crop_edge(shifted_x, shifted_y)
                {
                    continue;
                }
                let first_is_park = first.cell(first_x, first_y) == Some(MapCell::Park);
                let shifted_is_park = shifted.cell(shifted_x, shifted_y) == Some(MapCell::Park);
                assert_eq!(
                    first_is_park, shifted_is_park,
                    "infill moved at shared global cell {world:?}"
                );
                compared += 1;
                shared_tall_grass += usize::from(first_is_park);
            }
        }
        assert!(compared > 1_000, "overlap fixture compared too few cells");
        assert!(
            shared_tall_grass >= 16,
            "overlap fixture must exercise multiple shared grass patches"
        );
    }

    #[test]
    fn planned_backbone_preserves_water_and_home_has_a_walkable_spawn() {
        let water = Feature {
            kind: FeatureKind::Water,
            name: None,
            area: true,
            bridge: false,
            points: vec![
                Coordinate { lat: 0.2, lon: 0.2 },
                Coordinate { lat: 0.8, lon: 0.2 },
                Coordinate { lat: 0.8, lon: 0.8 },
                Coordinate { lat: 0.2, lon: 0.8 },
                Coordinate { lat: 0.2, lon: 0.2 },
            ],
        };
        let map_source = source(vec![
            water.clone(),
            Feature {
                kind: FeatureKind::MajorRoad,
                name: None,
                area: false,
                bridge: false,
                points: vec![
                    Coordinate { lat: 0.5, lon: 0.0 },
                    Coordinate { lat: 0.5, lon: 1.0 },
                ],
            },
        ]);
        let mut source_water = GeneratedGrid {
            source: map_source.clone(),
            width: 32,
            height: 32,
            cells: vec![MapCell::Grass; 32 * 32],
            labels: Vec::new(),
        };
        paint_feature(&mut source_water, &water);
        fill_water_pinholes(&mut source_water);
        remove_tiny_areas(&mut source_water, MapCell::Water, 16);
        let source_water_indices = source_water
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| (*cell == MapCell::Water).then_some(index))
            .collect::<Vec<_>>();

        let grid = generate_grid(map_source, 32, 32).expect("grid");
        let mapped_road =
            |cell| matches!(cell, MapCell::Street | MapCell::Road | MapCell::MajorRoad);
        let mapped_road_indices = grid
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| mapped_road(*cell).then_some(index))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            !mapped_road_indices.is_empty(),
            "the mapped road must survive global corridor planning"
        );
        assert!(
            mapped_road_indices.iter().any(|&index| {
                let x = index % usize::from(grid.width);
                let y = index / usize::from(grid.width);
                [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ]
                .into_iter()
                .filter(|&(next_x, next_y)| {
                    next_x < usize::from(grid.width) && next_y < usize::from(grid.height)
                })
                .map(|(next_x, next_y)| next_y * usize::from(grid.width) + next_x)
                .any(|next| mapped_road_indices.contains(&next))
            }),
            "the mapped road must form a corridor, not an isolated tile"
        );
        assert!(
            source_water_indices
                .iter()
                .all(|&index| !mapped_road(grid.cells[index])),
            "mapped roads must never overwrite the source-water mask"
        );
        let preserved_water = source_water_indices
            .iter()
            .filter(|&&index| {
                matches!(
                    grid.cells[index],
                    MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                )
            })
            .count();
        assert!(
            preserved_water * 2 >= source_water_indices.len(),
            "the source water body must remain recognizable after authoring"
        );
        let home = grid.home_cell();
        assert!(
            is_walkable_cell(grid.cell(home.0, home.1).expect("home cell")),
            "the exact-coordinate spawn must remain walkable"
        );
        assert_eq!(grid.crystal_blocks().len(), 32 * 32);
    }
}
