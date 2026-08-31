use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{GeneratedGrid, H3Facility, MapCell};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapAudit {
    pub passed: bool,
    pub cell_counts: BTreeMap<String, usize>,
    pub houses: usize,
    pub wild_sites: usize,
    pub walkable_reach_percent: f64,
    pub errors: Vec<String>,
    pub notes: Vec<String>,
}

pub fn audit_grid(grid: &GeneratedGrid) -> MapAudit {
    let expected_pokecenter = grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.requests_facility(H3Facility::PokemonCenter));
    let expected_mart = grid
        .source
        .h3
        .as_ref()
        .is_none_or(|plan| plan.requests_facility(H3Facility::Mart));
    audit_grid_with_facilities(grid, expected_pokecenter, expected_mart)
}

/// Audits a grid against the facility allocation chosen by a regional batch.
///
/// [`audit_grid`] derives these expectations from the grid's H3 regional plan;
/// this explicit form is useful when validating a planner before attaching its
/// directives. Standalone generation still expects one Center and one Mart.
pub fn audit_grid_with_facilities(
    grid: &GeneratedGrid,
    expected_pokecenter: bool,
    expected_mart: bool,
) -> MapAudit {
    let mut counts = BTreeMap::new();
    for cell in &grid.cells {
        *counts.entry(cell_name(*cell).to_string()).or_insert(0) += 1;
    }
    let building_components = components(grid, MapCell::Building);
    let house_components = building_components
        .iter()
        .filter(|component| is_coherent_house(grid, component))
        .collect::<Vec<_>>();
    let city_landmarks = building_components
        .iter()
        .filter(|component| is_canonical_city_landmark(grid, component))
        .count();
    let building_styles = building_style_counts(grid, &building_components);
    let park_components = components(grid, MapCell::Park);
    let malformed_houses = building_components
        .iter()
        .filter(|component| {
            !is_coherent_house(grid, component) && !is_canonical_city_landmark(grid, component)
        })
        .count();
    let wild_sites = park_components
        .iter()
        .filter(|component| component.len() >= 20)
        .count();
    let park_cells = counts.get("park").copied().unwrap_or(0);
    let authored_cells = grid
        .cells
        .iter()
        .filter(|cell| **cell != MapCell::H3Void)
        .count()
        .max(1);
    let park_percent = park_cells as f64 / authored_cells as f64 * 100.0;
    let compact_wild_patches = park_components
        .iter()
        .filter(|component| (6..=12).contains(&component.len()))
        .collect::<Vec<_>>();
    let irregular_compact_patches = compact_wild_patches
        .iter()
        .filter(|component| component_bounding_area(grid, component) > component.len())
        .count();
    let (walkable_reach_percent, mapped_boundary_exits) = walkable_reach(grid);
    let mut errors = Vec::new();
    if malformed_houses > 0 {
        errors.push(format!(
            "{malformed_houses} building footprints are not a complete Crystal house or civic landmark"
        ));
    }
    let expected_wild_sites = if grid.source.h3.is_some() {
        1..=4
    } else {
        2..=4
    };
    if !expected_wild_sites.contains(&wild_sites) {
        errors.push(format!(
            "expected {}-{} substantial wild-grass sites, generated {wild_sites}",
            expected_wild_sites.start(),
            expected_wild_sites.end()
        ));
    }
    let (minimum_park_percent, maximum_park_percent) = if grid.source.h3.is_some() {
        (4.5, 12.0)
    } else {
        (5.0, 10.0)
    };
    if grid.width.min(grid.height) >= 56
        && !(minimum_park_percent..=maximum_park_percent).contains(&park_percent)
    {
        errors.push(format!(
            "tall-grass coverage is {park_percent:.1}%; expected {minimum_park_percent:.1}-{maximum_park_percent:.1}% across large encounter rooms and scattered accents"
        ));
    }
    let expected_compact_patches = if grid.width.min(grid.height) >= 56 {
        if grid.source.h3.is_some() { 5 } else { 6 }
    } else if grid.width.min(grid.height) >= 40 {
        3
    } else {
        0
    };
    if compact_wild_patches.len() < expected_compact_patches {
        errors.push(format!(
            "only {} compact tall-grass accents were generated; expected at least {expected_compact_patches}",
            compact_wild_patches.len()
        ));
    }
    let rectangular_compact_patches = compact_wild_patches.len() - irregular_compact_patches;
    let maximum_rectangular_patches = usize::from(grid.source.h3.is_some());
    if rectangular_compact_patches > maximum_rectangular_patches {
        errors.push(format!(
            "{} compact tall-grass accents are rectangular instead of irregular",
            rectangular_compact_patches
        ));
    }
    let urban_intensity = crate::grid::urban_intensity(grid);
    let target_houses = crate::grid::target_house_count(grid);
    // prepare_home_spawn may add one protected complete home after the city
    // cluster pass. Allow small topology-driven placement loss, but never let
    // a large face silently fall back to the old 64x64 house budget.
    let expected_houses = target_houses.saturating_sub(4)..=target_houses + 1;
    if !expected_houses.contains(&house_components.len()) {
        errors.push(format!(
            "expected {}-{} coherent houses for urban intensity {urban_intensity}, generated {}",
            expected_houses.start(),
            expected_houses.end(),
            house_components.len(),
        ));
    }
    let building_style_dimension = grid.width.min(grid.height);
    let expected_building_styles = if urban_intensity == 2 && building_style_dimension >= 96 {
        4
    } else if urban_intensity == 2 && building_style_dimension >= 56 {
        3
    } else {
        1
    };
    if building_styles.len() < expected_building_styles {
        errors.push(format!(
            "only {} literal building styles were generated; expected at least {expected_building_styles} for urban intensity {urban_intensity}",
            building_styles.len()
        ));
    }
    let ice_floor_cells = counts.get("ice_floor").copied().unwrap_or(0);
    let ice_boulders = counts.get("ice_boulder").copied().unwrap_or(0);
    let rock_floor_cells = counts.get("rock_floor").copied().unwrap_or(0);
    for (floor, label) in [
        (MapCell::IceFloor, "ice surface dungeon"),
        (MapCell::RockFloor, "rock surface dungeon"),
    ] {
        if counts.get(cell_name(floor)).copied().unwrap_or(0) > 0
            && let Err(error) = surface_dungeon_quality(grid, floor)
        {
            errors.push(format!("{label} {error}"));
        }
    }
    if grid.source.h3.is_none() && grid.width.min(grid.height) >= 96 {
        if ice_floor_cells < 30 || ice_boulders < 10 {
            errors.push(
                "large overview is missing its complete irregular surface Ice Path biome"
                    .to_string(),
            );
        }
        if rock_floor_cells < 30 {
            errors.push(
                "large overview is missing its broad continuous brown rocky-surface biome"
                    .to_string(),
            );
        }
    }
    let pokecenter_cells = [
        MapCell::PokecenterNorthWest,
        MapCell::PokecenterNorthEast,
        MapCell::PokecenterSouthWest,
        MapCell::PokecenterSouthEast,
    ]
    .into_iter()
    .map(|cell| counts.get(cell_name(cell)).copied().unwrap_or(0))
    .sum::<usize>();
    let has_complete_pokecenter =
        pokecenter_cells == 4 && crate::grid::pokecenter_origin(grid).is_some();
    if expected_pokecenter && !has_complete_pokecenter {
        errors.push("expected one complete canonical Pokemon Center facade".to_string());
    } else if !expected_pokecenter && pokecenter_cells != 0 {
        errors.push(
            "regional plan excludes a Pokemon Center, but facade tiles were generated".to_string(),
        );
    }
    let mart_cells = [
        MapCell::MartNorthWest,
        MapCell::MartNorthEast,
        MapCell::MartSouthWest,
        MapCell::MartSouthEast,
    ]
    .into_iter()
    .map(|cell| counts.get(cell_name(cell)).copied().unwrap_or(0))
    .sum::<usize>();
    let has_complete_mart = mart_cells == 4 && crate::grid::mart_origin(grid).is_some();
    if expected_mart && !has_complete_mart {
        errors.push("expected one complete canonical Pokemon Mart facade".to_string());
    } else if !expected_mart && mart_cells != 0 {
        errors.push(
            "regional plan excludes a Pokemon Mart, but facade tiles were generated".to_string(),
        );
    }
    if walkable_reach_percent < 100.0 {
        errors.push(format!(
            "only {walkable_reach_percent:.2}% of interior walkable terrain is connected to the spawn; every non-exit walkable cell must be reachable"
        ));
    }
    let transit = [
        MapCell::Rail,
        MapCell::Trail,
        MapCell::Street,
        MapCell::Road,
        MapCell::MajorRoad,
    ]
    .into_iter()
    .map(|cell| counts.get(cell_name(cell)).copied().unwrap_or(0))
    .sum::<usize>();
    let transit_percent = transit as f64 / authored_cells as f64 * 100.0;
    let maximum_transit_percent = if grid.source.h3.is_some() { 10.5 } else { 10.0 };
    if !(3.0..=maximum_transit_percent).contains(&transit_percent) {
        errors.push(format!(
            "path coverage is {transit_percent:.1}%; expected 3-{maximum_transit_percent:.1}%"
        ));
    }
    let tree_cells = counts.get("tree").copied().unwrap_or(0)
        + counts.get("park_tree").copied().unwrap_or(0)
        + counts.get("small_tree").copied().unwrap_or(0)
        + counts.get("small_tree_south").copied().unwrap_or(0);
    let tree_denominator = if grid.source.h3.is_some() {
        authored_cells
            .saturating_sub(counts.get("water").copied().unwrap_or(0))
            .saturating_sub(counts.get("water_access_east").copied().unwrap_or(0))
            .saturating_sub(counts.get("water_access_west").copied().unwrap_or(0))
            .saturating_sub(counts.get("water_access_south").copied().unwrap_or(0))
            .max(1)
    } else {
        authored_cells
    };
    let tree_percent = tree_cells as f64 / tree_denominator as f64 * 100.0;
    if !(18.0..=35.0).contains(&tree_percent) {
        errors.push(format!(
            "tree coverage is {tree_percent:.1}%; expected 18-35%"
        ));
    }
    let longest_canopy_bar = (2..grid.height.saturating_sub(2))
        .map(|y| {
            let mut longest = 0;
            let mut current = 0;
            for x in 2..grid.width.saturating_sub(2) {
                if matches!(grid.cell(x, y), Some(MapCell::Tree | MapCell::ParkTree)) {
                    current += 1;
                    longest = longest.max(current);
                } else {
                    current = 0;
                }
            }
            longest
        })
        .max()
        .unwrap_or(0);
    if grid.width.min(grid.height) >= 56 && longest_canopy_bar > 12 {
        errors.push(format!(
            "dense canopy contains a {longest_canopy_bar}-block horizontal bar; expected irregular broken groves no longer than 12 blocks"
        ));
    }
    let park_trees = counts.get("park_tree").copied().unwrap_or(0);
    let headbutt_trees = counts.get("tree").copied().unwrap_or(0);
    if grid.width.min(grid.height) >= 56
        && (park_trees > 24 || park_trees.saturating_mul(8) > headbutt_trees.max(1))
    {
        errors.push(format!(
            "{park_trees} canonical National Park trees overwhelm {headbutt_trees} ordinary headbutt trees; expected at most 24 and no more than one per eight headbutt trees"
        ));
    }
    let water_cells = counts.get("water").copied().unwrap_or(0);
    let water_accesses = counts.get("water_access_east").copied().unwrap_or(0)
        + counts.get("water_access_west").copied().unwrap_or(0)
        + counts.get("water_access_south").copied().unwrap_or(0);
    if water_cells > 0 && water_accesses == 0 {
        errors.push("principal water body has no collision-correct access bank".to_string());
    }
    let flowers = counts.get("flowers").copied().unwrap_or(0);
    if flowers < 12 {
        errors.push(format!(
            "only {flowers} flower blocks were placed; expected at least 12 coherent accents"
        ));
    }
    let boulders = counts.get("boulder").copied().unwrap_or(0);
    let minimum_dimension = grid.width.min(grid.height);
    let expected_boulder_range = if grid.source.h3.is_some() && minimum_dimension >= 56 {
        32..=96
    } else if minimum_dimension >= 96 {
        40..=60
    } else if minimum_dimension >= 56 {
        32..=48
    } else if minimum_dimension >= 40 {
        18..=36
    } else {
        0..=24
    };
    if !expected_boulder_range.contains(&boulders) {
        errors.push(format!(
            "generated {boulders} boulders; expected {}-{} canonical rocks at this scale",
            expected_boulder_range.start(),
            expected_boulder_range.end()
        ));
    }
    let boulder_formations = boulder_proximity_components(grid, 2);
    let substantial_boulder_formations = boulder_formations
        .iter()
        .filter(|formation| formation.len() >= 3)
        .count();
    let grouped_boulders = boulder_formations
        .iter()
        .filter(|formation| formation.len() >= 3)
        .map(Vec::len)
        .sum::<usize>();
    let minimum_boulder_formations = 6;
    let maximum_boulder_formations = if grid.source.h3.is_some() {
        18
    } else if minimum_dimension >= 96 {
        14
    } else {
        10
    };
    if minimum_dimension >= 56
        && !(minimum_boulder_formations..=maximum_boulder_formations)
            .contains(&substantial_boulder_formations)
    {
        errors.push(format!(
            "expected {minimum_boulder_formations}-{maximum_boulder_formations} distinct crescent, spur, terrace, or broken-ring rock formations; generated {substantial_boulder_formations}"
        ));
    }
    if minimum_dimension >= 56 && grouped_boulders * 4 < boulders * 3 {
        errors.push(format!(
            "only {grouped_boulders}/{boulders} boulders belong to coherent radius-two formations; expected at least 75%"
        ));
    }
    let benches = counts.get("bench").copied().unwrap_or(0);
    if benches < 2 {
        errors.push(format!(
            "only {benches} canonical park benches were placed; expected at least 2"
        ));
    }
    let trash_cans = counts.get("trash_can").copied().unwrap_or(0);
    if trash_cans == 0 {
        errors.push("no interactive outdoor trash can was placed".to_string());
    }
    let fountains = counts.get("fountain").copied().unwrap_or(0);
    if fountains == 0 {
        errors.push("no canonical National Park fountain was placed".to_string());
    }
    let signs = counts.get("ground_sign").copied().unwrap_or(0);
    if signs == 0 {
        errors.push("no readable route sign location was generated".to_string());
    }
    let fence_cells = [
        "fence_north_west",
        "fence_north",
        "fence_north_east",
        "fence_west",
        "fence_east",
        "fence_south_west",
        "fence_south",
        "fence_south_east",
    ]
    .into_iter()
    .map(|name| counts.get(name).copied().unwrap_or(0))
    .sum::<usize>();
    let minimum_fence_cells = if grid.source.h3.is_some() && minimum_dimension >= 56 {
        // The playable hex face contains about three quarters of its backing
        // rectangle. Thirty blocks preserves (and this generator exceeds) the
        // square room's fence density without turning the seam into a ring.
        30
    } else if minimum_dimension >= 96 {
        120
    } else if minimum_dimension >= 56 {
        40
    } else {
        5
    };
    if fence_cells < minimum_fence_cells {
        errors.push(format!(
            "only {fence_cells} coherent fence blocks were placed; expected at least {minimum_fence_cells}"
        ));
    }
    let roadside_fences = crate::roadside::roadside_fence_quality(grid);
    if grid.source.h3.is_some() && minimum_dimension >= 56 {
        if roadside_fences.roadside_courses < crate::roadside::H3_MIN_ROADSIDE_COURSES
            || roadside_fences.roadside_fence_cells < crate::roadside::H3_MIN_ROADSIDE_FENCE_CELLS
        {
            errors.push(format!(
                "only {} straight roadside fence courses ({} cells) remain after excluding the {}-cell public-field perimeter; expected at least {} courses and {} cells",
                roadside_fences.roadside_courses,
                roadside_fences.roadside_fence_cells,
                roadside_fences.public_perimeter_cells,
                crate::roadside::H3_MIN_ROADSIDE_COURSES,
                crate::roadside::H3_MIN_ROADSIDE_FENCE_CELLS,
            ));
        }
        if roadside_fences.malformed_courses > 0 {
            errors.push(format!(
                "{} roadside fence course(s) are bent, mixed, broken, or outside the canonical 3-10 block grammar",
                roadside_fences.malformed_courses
            ));
        }
        if roadside_fences.route_supported_cells * 10 < roadside_fences.roadside_fence_cells * 7 {
            errors.push(format!(
                "only {}/{} roadside fence cells run parallel to the mapped/principal route across a complete zero-, one-, or two-cell verge; expected at least 70%",
                roadside_fences.route_supported_cells, roadside_fences.roadside_fence_cells
            ));
        }
        if roadside_fences.urban_supported_courses * 2 < roadside_fences.roadside_courses {
            errors.push(format!(
                "only {}/{} roadside fence courses have building-side urban support; expected at least half",
                roadside_fences.urban_supported_courses, roadside_fences.roadside_courses
            ));
        }
        if roadside_fences.landing_conflicts > 0 || roadside_fences.terrain_conflicts > 0 {
            errors.push(format!(
                "roadside fences conflict with {} selected H3 landing halo cell(s) and {} water/void/relief halo cell(s)",
                roadside_fences.landing_conflicts, roadside_fences.terrain_conflicts
            ));
        }
        if roadside_fences.roadside_amenities == 0 {
            errors.push(
                "no canonical bench or trash can was placed in a clear roadside fence verge"
                    .to_string(),
            );
        }
    }
    let ledge_cells = ["ledge_west", "ledge_middle", "ledge_east"]
        .into_iter()
        .map(|name| counts.get(name).copied().unwrap_or(0))
        .sum::<usize>();
    let (ledge_runs, orphaned_ledge_cells) = complete_ledge_runs(grid);
    let h3_room = grid.source.h3.is_some();
    let expected_ledge_runs = if h3_room && minimum_dimension >= 56 {
        3
    } else if minimum_dimension >= 96 {
        8
    } else if minimum_dimension >= 56 {
        5
    } else if minimum_dimension >= 40 {
        3
    } else {
        0
    };
    if orphaned_ledge_cells > 0 {
        errors.push(format!(
            "{orphaned_ledge_cells} ledge blocks are not part of a complete west/middle/east run"
        ));
    }
    if ledge_runs.len() != expected_ledge_runs {
        errors.push(format!(
            "expected {expected_ledge_runs} complete one-way ledge runs, generated {}",
            ledge_runs.len()
        ));
    }
    let malformed_ledge_runs = ledge_runs
        .iter()
        .filter(|(_, _, length)| !(6..=10).contains(length))
        .count();
    if malformed_ledge_runs > 0 {
        errors.push(format!(
            "{malformed_ledge_runs} ledge runs fall outside the authored length of 6-10 canonical blocks"
        ));
    }
    let ledge_total = ledge_runs
        .iter()
        .map(|(_, _, length)| usize::from(*length))
        .sum::<usize>();
    let distinct_ledge_lengths = ledge_runs
        .iter()
        .map(|(_, _, length)| *length)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let expected_ledge_span = if h3_room && minimum_dimension >= 56 {
        24..=24
    } else if minimum_dimension >= 96 {
        56..=72
    } else {
        32..=45
    };
    if minimum_dimension >= 56 && !expected_ledge_span.contains(&ledge_total) {
        errors.push(format!(
            "the ledge system spans {ledge_total} blocks; expected {}-{} blocks of substantial relief",
            expected_ledge_span.start(),
            expected_ledge_span.end()
        ));
    }
    if minimum_dimension >= 56 && distinct_ledge_lengths < 3 {
        errors.push(format!(
            "ledge runs use only {distinct_ledge_lengths} distinct lengths; expected at least three"
        ));
    }
    let obstructed_ledge_runs = ledge_runs
        .iter()
        .filter(|&&(x, y, length)| !ledge_run_has_clear_approach_and_landing(grid, x, y, length))
        .count();
    if obstructed_ledge_runs > usize::from(h3_room) {
        errors.push(format!(
            "{obstructed_ledge_runs} ledge runs have an obstructed approach, landing, connector, or bypass"
        ));
    }
    let compound_ledge_pairs = ledge_runs
        .iter()
        .enumerate()
        .filter(|(index, run)| {
            let &(x, y, length) = *run;
            ledge_runs
                .iter()
                .skip(index + 1)
                .any(|&(other_x, other_y, other_length)| {
                    (3..=5).contains(&y.abs_diff(other_y))
                        && (1..=3).contains(&x.abs_diff(other_x))
                        && x < other_x + other_length
                        && other_x < x + length
                })
        })
        .count();
    let expected_compound_pairs = if h3_room && minimum_dimension >= 56 {
        0
    } else if minimum_dimension >= 96 {
        3
    } else if minimum_dimension >= 56 {
        2
    } else if minimum_dimension >= 40 {
        1
    } else {
        0
    };
    if compound_ledge_pairs < expected_compound_pairs {
        errors.push(format!(
            "only {compound_ledge_pairs} offset stacked ledge pairs were generated; expected at least {expected_compound_pairs}"
        ));
    }
    let cliff_cells = counts
        .iter()
        .filter(|(name, _)| name.starts_with("cliff_"))
        .map(|(_, count)| *count)
        .sum::<usize>();
    let inner_west = counts.get("cliff_inner_south_west").copied().unwrap_or(0);
    let inner_east = counts.get("cliff_inner_south_east").copied().unwrap_or(0);
    if h3_room && minimum_dimension >= 56 {
        use crate::grid::PlateauContourKind;

        let contours = crate::grid::canonical_plateau_contours(grid);
        let recognized_cliff_cells = contours
            .iter()
            .map(|contour| contour.cliff_cells)
            .sum::<usize>();
        let expanded = contours
            .iter()
            .filter(|contour| contour.kind == PlateauContourKind::Expanded)
            .count();
        let stepped = contours
            .iter()
            .filter(|contour| contour.kind == PlateauContourKind::Stepped)
            .count();
        let contour_stairs = contours.iter().map(|contour| contour.stairs).sum::<usize>();
        let contour_inner_west = contours
            .iter()
            .map(|contour| contour.inner_west)
            .sum::<usize>();
        let contour_inner_east = contours
            .iter()
            .map(|contour| contour.inner_east)
            .sum::<usize>();
        if contours.len() != 2
            || expanded != 1
            || stepped != 1
            || recognized_cliff_cells != cliff_cells
            || !(55..=113).contains(&cliff_cells)
            || contour_stairs != 2
            || contour_inner_west != 1
            || contour_inner_east != 1
            || inner_west != 1
            || inner_east != 1
        {
            errors.push(format!(
                "H3 rooms require exactly one complete catalog expanded contour and one complete catalog stepped contour (55-113 total blocks, 2 stairs, one matched $6e/$6f pair); recognized {}/{} blocks across {} contour(s) ({expanded} expanded/{stepped} stepped) with {contour_stairs} stairs and {inner_west}/{inner_east} inner corners",
                recognized_cliff_cells,
                cliff_cells,
                contours.len(),
            ));
        }
    } else if minimum_dimension >= 56 {
        let expected_cliff_cells = if minimum_dimension >= 96 { 226 } else { 55 };
        let expected_inner_corners = if minimum_dimension >= 96 { 2 } else { 1 };
        if cliff_cells != expected_cliff_cells
            || inner_west != expected_inner_corners
            || inner_east != expected_inner_corners
            || !has_expanded_plateau(grid)
            || !has_stepped_plateau(grid)
        {
            errors.push(format!(
                "large square maps require their complete expanded and stepped contours ({expected_cliff_cells} blocks, {expected_inner_corners} $6e/$6f pair(s)); generated {cliff_cells} blocks with {inner_west}/{inner_east} inner corners"
            ));
        }
    } else if minimum_dimension >= 40 && cliff_cells < 28 {
        errors.push("no complete expanded rocky plateau contour was generated".to_string());
    }
    let obstructed_cliff_stairs = grid
        .cells
        .iter()
        .enumerate()
        .filter(|(_, cell)| **cell == MapCell::CliffStairs)
        .filter(|(index, _)| {
            let x = (*index % usize::from(grid.width)) as u16;
            let y = (*index / usize::from(grid.width)) as u16;
            !cliff_stair_has_visible_south_approach(grid, x, y)
        })
        .count();
    if obstructed_cliff_stairs > 0 {
        errors.push(format!(
            "{obstructed_cliff_stairs} cliff stair(s) lack a visible Trail approach immediately south"
        ));
    }
    let mut notes = vec![
        format!("{transit_percent:.1}% of the map preserves mapped transport corridors"),
        format!(
            "{} coherent houses were retained from real building sites",
            house_components.len()
        ),
    ];
    if city_landmarks > 0 {
        notes.push(format!(
            "{city_landmarks} complete Goldenrod-style department-store/radio-tower landmark(s) anchor dense districts"
        ));
    }
    notes.push(format!(
        "{} literal building styles are present: {}",
        building_styles.len(),
        building_styles
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .join(", ")
    ));
    if expected_pokecenter && has_complete_pokecenter {
        notes.push("one complete Pokemon Center facade has a functional entrance".to_string());
    }
    if expected_mart && has_complete_mart {
        notes
            .push("one complete Pokemon Mart facade opens a functional canonical shop".to_string());
    }
    if ice_floor_cells > 0 {
        notes.push(format!(
            "surface cave biomes include {ice_floor_cells} blue Ice Path floor, {ice_boulders} ice-rock blocks, and {rock_floor_cells} brown rocky-floor blocks; no doorway or interior warp is present"
        ));
    }
    notes.extend([
        format!("{wild_sites} connected wild-grass sites include clustered tree groves"),
        format!(
            "{park_percent:.1}% tall-grass coverage includes {} compact irregular encounter accents",
            compact_wild_patches.len()
        ),
        format!("{tree_percent:.1}% tree coverage uses canonical dense and small-tree blocks"),
        format!(
            "{} straight roadside fence courses in {} orientation(s) contribute {} cells ({} route-supported, {} urban-supported courses, {} street fixture(s)) beyond the {}-cell public-field perimeter",
            roadside_fences.roadside_courses,
            roadside_fences.orientation_variants,
            roadside_fences.roadside_fence_cells,
            roadside_fences.route_supported_cells,
            roadside_fences.urban_supported_courses,
            roadside_fences.roadside_amenities,
            roadside_fences.public_perimeter_cells,
        ),
        format!(
            "{flowers} flower, {boulders} rock in {substantial_boulder_formations} formations, {benches} bench, {trash_cans} trash-can, {fountains} fountain, {fence_cells} fence, {ledge_cells} ledge across {} complete runs ({} compound pairs), and {cliff_cells} cliff blocks add authored variety",
            ledge_runs.len(),
            compound_ledge_pairs
        ),
    ]);
    if mapped_boundary_exits > 0 {
        notes.push(format!(
            "{mapped_boundary_exits} globally aligned mapped-road cell(s) leave through the regional crop boundary"
        ));
    }
    MapAudit {
        passed: errors.is_empty(),
        cell_counts: counts,
        houses: house_components.len(),
        wild_sites,
        walkable_reach_percent,
        errors,
        notes,
    }
}

fn surface_dungeon_quality(grid: &GeneratedGrid, floor: MapCell) -> Result<(), String> {
    let floor_components = components(grid, floor);
    if floor_components.len() != 1 {
        return Err(format!(
            "has {} disconnected floor components; expected one room-and-corridor plan",
            floor_components.len()
        ));
    }
    let component = &floor_components[0];
    let width = usize::from(grid.width);
    let occupied = component
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
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
    let bbox_area = (max_x - min_x + 1) * (max_y - min_y + 1);
    let fill_percent = component.len() as f64 / bbox_area.max(1) as f64 * 100.0;
    if fill_percent > 62.0 {
        return Err(format!(
            "fills {fill_percent:.1}% of its bounding box like a blob; expected at most 62%"
        ));
    }

    let degree = |index: usize| {
        neighbors(grid, index)
            .into_iter()
            .filter(|next| occupied.contains(next))
            .count()
    };
    let passages = component
        .iter()
        .filter(|&&index| degree(index) <= 2)
        .count();
    let junctions = component
        .iter()
        .filter(|&&index| degree(index) >= 3)
        .count();
    let room_squares = (min_y..max_y)
        .flat_map(|y| (min_x..max_x).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            [
                y * width + x,
                y * width + x + 1,
                (y + 1) * width + x,
                (y + 1) * width + x + 1,
            ]
            .into_iter()
            .all(|index| occupied.contains(&index))
        })
        .count();
    if passages < 8 || junctions < 8 || room_squares < 10 {
        return Err(format!(
            "has passages={passages}, junctions={junctions}, chamber-squares={room_squares}; expected at least 8/8/10"
        ));
    }
    Ok(())
}

fn component_bounding_area(grid: &GeneratedGrid, component: &[usize]) -> usize {
    let width = usize::from(grid.width);
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

fn complete_ledge_runs(grid: &GeneratedGrid) -> (Vec<(u16, u16, u16)>, usize) {
    let mut claimed = vec![false; grid.cells.len()];
    let mut runs = Vec::new();
    for y in 0..grid.height {
        let mut x = 0;
        while x < grid.width {
            if grid.cell(x, y) != Some(MapCell::LedgeWest) {
                x += 1;
                continue;
            }
            let origin = x;
            let mut cursor = x + 1;
            while cursor < grid.width && grid.cell(cursor, y) == Some(MapCell::LedgeMiddle) {
                cursor += 1;
            }
            if cursor < grid.width
                && cursor > origin + 1
                && grid.cell(cursor, y) == Some(MapCell::LedgeEast)
            {
                for claimed_x in origin..=cursor {
                    claimed[usize::from(y) * usize::from(grid.width) + usize::from(claimed_x)] =
                        true;
                }
                runs.push((origin, y, cursor - origin + 1));
                x = cursor + 1;
            } else {
                x += 1;
            }
        }
    }
    let orphaned = grid
        .cells
        .iter()
        .enumerate()
        .filter(|(index, cell)| {
            matches!(
                cell,
                MapCell::LedgeWest | MapCell::LedgeMiddle | MapCell::LedgeEast
            ) && !claimed[*index]
        })
        .count();
    (runs, orphaned)
}

fn ledge_run_has_clear_approach_and_landing(
    grid: &GeneratedGrid,
    x: u16,
    y: u16,
    length: u16,
) -> bool {
    if x == 0 || y == 0 || x + length >= grid.width || y + 1 >= grid.height {
        return false;
    }
    let clear_apron = |cell: Option<MapCell>| {
        matches!(
            cell,
            Some(
                MapCell::Grass
                    | MapCell::Lawn
                    | MapCell::Clearing
                    | MapCell::Flowers
                    | MapCell::Trail
                    | MapCell::Street
                    | MapCell::Road
                    | MapCell::MajorRoad
            )
        )
    };
    let full_aprons_are_clear = (0..length).all(|offset| {
        clear_apron(grid.cell(x + offset, y - 1)) && clear_apron(grid.cell(x + offset, y + 1))
    });
    let middle = x + length / 2;
    full_aprons_are_clear
        && grid.cell(middle, y - 1) == Some(MapCell::Trail)
        && is_walkable_cell(grid.cell(x - 1, y))
        && is_walkable_cell(grid.cell(x + length, y))
}

fn cliff_stair_has_visible_south_approach(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    y + 1 < grid.height && grid.cell(x, y + 1) == Some(MapCell::Trail)
}

fn boulder_proximity_components(grid: &GeneratedGrid, radius: u16) -> Vec<Vec<(u16, u16)>> {
    let boulders = grid
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
    let mut unseen = vec![true; boulders.len()];
    let mut components = Vec::new();
    for start in 0..boulders.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut frontier = vec![start];
        while let Some(index) = frontier.pop() {
            let cell = boulders[index];
            component.push(cell);
            for next in 0..boulders.len() {
                if unseen[next]
                    && cell.0.abs_diff(boulders[next].0) <= radius
                    && cell.1.abs_diff(boulders[next].1) <= radius
                {
                    unseen[next] = false;
                    frontier.push(next);
                }
            }
        }
        components.push(component);
    }
    components
}

fn has_expanded_plateau(grid: &GeneratedGrid) -> bool {
    if grid.width >= 11 && grid.height >= 5 {
        for y in 0..=grid.height - 5 {
            for x in 0..=grid.width - 11 {
                let top = (0..11).all(|dx| {
                    grid.cell(x + dx, y)
                        == Some(match dx {
                            0 => MapCell::CliffNorthWest,
                            10 => MapCell::CliffNorthEast,
                            _ => MapCell::CliffNorth,
                        })
                });
                let middle = (1..=3).all(|dy| {
                    (0..11).all(|dx| {
                        grid.cell(x + dx, y + dy)
                            == Some(match dx {
                                0 => MapCell::CliffWest,
                                10 => MapCell::CliffEast,
                                _ => MapCell::CliffCenter,
                            })
                    })
                });
                let south = (0..11).all(|dx| {
                    grid.cell(x + dx, y + 4)
                        == Some(match dx {
                            0 => MapCell::CliffSouthWest,
                            5 => MapCell::CliffStairs,
                            10 => MapCell::CliffSouthEast,
                            _ => MapCell::CliffSouth,
                        })
                });
                if top && middle && south {
                    return true;
                }
            }
        }
    }
    if grid.width < 7 || grid.height < 4 {
        return false;
    }
    for y in 0..=grid.height - 4 {
        for x in 0..=grid.width - 7 {
            let top = (0..7).all(|dx| {
                grid.cell(x + dx, y)
                    == Some(match dx {
                        0 => MapCell::CliffNorthWest,
                        6 => MapCell::CliffNorthEast,
                        _ => MapCell::CliffNorth,
                    })
            });
            let middle = (1..=2).all(|dy| {
                (0..7).all(|dx| {
                    grid.cell(x + dx, y + dy)
                        == Some(match dx {
                            0 => MapCell::CliffWest,
                            6 => MapCell::CliffEast,
                            _ => MapCell::CliffCenter,
                        })
                })
            });
            let south = (0..7).all(|dx| {
                grid.cell(x + dx, y + 3)
                    == Some(match dx {
                        0 => MapCell::CliffSouthWest,
                        3 => MapCell::CliffStairs,
                        6 => MapCell::CliffSouthEast,
                        _ => MapCell::CliffSouth,
                    })
            });
            if top && middle && south {
                return true;
            }
        }
    }
    false
}

fn has_stepped_plateau(grid: &GeneratedGrid) -> bool {
    if grid.width >= 11 && grid.height >= 6 {
        for y in 0..=grid.height - 6 {
            for x in 0..=grid.width - 11 {
                let top = (0..11).all(|dx| {
                    grid.cell(x + dx, y)
                        == Some(match dx {
                            0 => MapCell::CliffNorthWest,
                            10 => MapCell::CliffNorthEast,
                            _ => MapCell::CliffNorth,
                        })
                });
                let middle = (1..=2).all(|dy| {
                    (0..11).all(|dx| {
                        grid.cell(x + dx, y + dy)
                            == Some(match dx {
                                0 => MapCell::CliffWest,
                                10 => MapCell::CliffEast,
                                _ => MapCell::CliffCenter,
                            })
                    })
                });
                let shoulder = (0..11).all(|dx| {
                    grid.cell(x + dx, y + 3)
                        == Some(match dx {
                            0 => MapCell::CliffSouthWest,
                            2 => MapCell::CliffInnerSouthWest,
                            8 => MapCell::CliffInnerSouthEast,
                            10 => MapCell::CliffSouthEast,
                            3..=7 => MapCell::CliffCenter,
                            _ => MapCell::CliffSouth,
                        })
                });
                let stem_middle = (2..=8).all(|dx| {
                    grid.cell(x + dx, y + 4)
                        == Some(match dx {
                            2 => MapCell::CliffWest,
                            8 => MapCell::CliffEast,
                            _ => MapCell::CliffCenter,
                        })
                });
                let stem_south = (2..=8).all(|dx| {
                    grid.cell(x + dx, y + 5)
                        == Some(match dx {
                            2 => MapCell::CliffSouthWest,
                            5 => MapCell::CliffStairs,
                            8 => MapCell::CliffSouthEast,
                            _ => MapCell::CliffSouth,
                        })
                });
                if top && middle && shoulder && stem_middle && stem_south {
                    return true;
                }
            }
        }
    }
    if grid.width < 7 || grid.height < 5 {
        return false;
    }
    for y in 0..=grid.height - 5 {
        for x in 0..=grid.width - 7 {
            let top = (0..7).all(|dx| {
                grid.cell(x + dx, y)
                    == Some(match dx {
                        0 => MapCell::CliffNorthWest,
                        6 => MapCell::CliffNorthEast,
                        _ => MapCell::CliffNorth,
                    })
            });
            let middle = (0..7).all(|dx| {
                grid.cell(x + dx, y + 1)
                    == Some(match dx {
                        0 => MapCell::CliffWest,
                        6 => MapCell::CliffEast,
                        _ => MapCell::CliffCenter,
                    })
            });
            let shoulder = [
                MapCell::CliffSouthWest,
                MapCell::CliffSouth,
                MapCell::CliffInnerSouthWest,
                MapCell::CliffCenter,
                MapCell::CliffInnerSouthEast,
                MapCell::CliffSouth,
                MapCell::CliffSouthEast,
            ]
            .into_iter()
            .enumerate()
            .all(|(dx, cell)| grid.cell(x + dx as u16, y + 2) == Some(cell));
            let stem = [
                (2, 3, MapCell::CliffWest),
                (3, 3, MapCell::CliffCenter),
                (4, 3, MapCell::CliffEast),
                (2, 4, MapCell::CliffSouthWest),
                (3, 4, MapCell::CliffStairs),
                (4, 4, MapCell::CliffSouthEast),
            ]
            .into_iter()
            .all(|(dx, dy, cell)| grid.cell(x + dx, y + dy) == Some(cell));
            if top && middle && shoulder && stem {
                return true;
            }
        }
    }
    false
}

fn is_walkable_cell(cell: Option<MapCell>) -> bool {
    cell.is_some_and(|cell| {
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
    })
}

fn components(grid: &GeneratedGrid, wanted: MapCell) -> Vec<Vec<usize>> {
    let mut visited = vec![false; grid.cells.len()];
    let mut found = Vec::new();
    for start in 0..grid.cells.len() {
        if visited[start] || grid.cells[start] != wanted {
            continue;
        }
        let mut component = Vec::new();
        let mut frontier = vec![start];
        visited[start] = true;
        while let Some(index) = frontier.pop() {
            component.push(index);
            for next in neighbors(grid, index) {
                if !visited[next] && grid.cells[next] == wanted {
                    visited[next] = true;
                    frontier.push(next);
                }
            }
        }
        found.push(component);
    }
    found
}

fn neighbors(grid: &GeneratedGrid, index: usize) -> Vec<usize> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let x = index % width;
    let y = index / width;
    let mut result = Vec::with_capacity(4);
    if x > 0 {
        result.push(index - 1);
    }
    if x + 1 < width {
        result.push(index + 1);
    }
    if y > 0 {
        result.push(index - width);
    }
    if y + 1 < height {
        result.push(index + width);
    }
    result
}

fn is_coherent_house(grid: &GeneratedGrid, component: &[usize]) -> bool {
    let (width, height) = component_dimensions(grid, component);
    matches!((component.len(), width, height), (4, 2, 2))
}

fn is_canonical_city_landmark(grid: &GeneratedGrid, component: &[usize]) -> bool {
    let (width, height) = component_dimensions(grid, component);
    matches!((component.len(), width, height), (12, 3, 4) | (9, 2, 6))
}

fn component_dimensions(grid: &GeneratedGrid, component: &[usize]) -> (usize, usize) {
    if component.is_empty() {
        return (0, 0);
    }
    let width = usize::from(grid.width);
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
    (max_x - min_x + 1, max_y - min_y + 1)
}

fn building_style_counts(
    grid: &GeneratedGrid,
    components: &[Vec<usize>],
) -> BTreeMap<&'static str, usize> {
    let blocks = grid.crystal_blocks();
    let width = usize::from(grid.width);
    let mut styles = BTreeMap::new();
    for component in components {
        let dimensions = component_dimensions(grid, component);
        let first = component.iter().copied().min().unwrap_or(0);
        let style = match dimensions {
            (3, 4) => "Goldenrod department store",
            (2, 6) => "Goldenrod radio tower",
            (2, 2) if blocks[first] == 0x97 => "Ecruteak traditional house",
            (2, 2) => "modern Johto house",
            _ => "unknown",
        };
        *styles.entry(style).or_insert(0) += 1;
        debug_assert_eq!(
            first % width,
            component.iter().map(|index| index % width).min().unwrap()
        );
    }
    styles
}

fn walkable_reach(grid: &GeneratedGrid) -> (f64, usize) {
    let walkable = |cell: MapCell| {
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
    };
    let (start_x, start_y) = grid.home_cell();
    let start = usize::from(start_y) * usize::from(grid.width) + usize::from(start_x);
    let total = grid.cells.iter().filter(|cell| walkable(**cell)).count();
    if total == 0 || !walkable(grid.cells[start]) {
        return (0.0, 0);
    }
    let mut visited = vec![false; grid.cells.len()];
    let mut frontier = VecDeque::from([start]);
    visited[start] = true;
    let mut reached = 0;
    while let Some(index) = frontier.pop_front() {
        reached += 1;
        for next in neighbors(grid, index) {
            if !visited[next] && walkable(grid.cells[next]) {
                visited[next] = true;
                frontier.push_back(next);
            }
        }
    }
    // A globally aligned OSM corridor may enter the finite render through one
    // crop-edge cell while the connected continuation lies in the neighboring
    // map. Count that exact boundary cell as an external exit, not unreachable
    // interior terrain. This deliberately does not exempt any interior cell or
    // locally authored Trail, so disconnected rooms still fail the hard gate.
    let mapped_boundary_exits = visited
        .iter()
        .enumerate()
        .filter(|(index, reached)| {
            if **reached {
                return false;
            }
            let x = index % usize::from(grid.width);
            let y = index / usize::from(grid.width);
            let on_boundary = x == 0
                || y == 0
                || x + 1 == usize::from(grid.width)
                || y + 1 == usize::from(grid.height);
            on_boundary
                && matches!(
                    grid.cells[*index],
                    MapCell::Street | MapCell::Road | MapCell::MajorRoad
                )
        })
        .count();
    let interior_total = total.saturating_sub(mapped_boundary_exits);
    let percent = if interior_total == 0 {
        0.0
    } else {
        reached as f64 / interior_total as f64 * 100.0
    };
    (percent, mapped_boundary_exits)
}

fn cell_name(cell: MapCell) -> &'static str {
    match cell {
        MapCell::H3Void => "h3_void",
        MapCell::Grass => "grass",
        MapCell::Lawn => "lawn",
        MapCell::Clearing => "clearing",
        MapCell::Park => "park",
        MapCell::Flowers => "flowers",
        MapCell::Tree => "tree",
        MapCell::ParkTree => "park_tree",
        MapCell::SmallTree => "small_tree",
        MapCell::SmallTreeSouth => "small_tree_south",
        MapCell::Boulder => "boulder",
        MapCell::IceFloor => "ice_floor",
        MapCell::IceBoulder => "ice_boulder",
        MapCell::RockFloor => "rock_floor",
        MapCell::Bench => "bench",
        MapCell::TrashCan => "trash_can",
        MapCell::Fountain => "fountain",
        MapCell::GroundSign => "ground_sign",
        MapCell::FenceNorthWest => "fence_north_west",
        MapCell::FenceNorth => "fence_north",
        MapCell::FenceNorthEast => "fence_north_east",
        MapCell::FenceWest => "fence_west",
        MapCell::FenceEast => "fence_east",
        MapCell::FenceSouthWest => "fence_south_west",
        MapCell::FenceSouth => "fence_south",
        MapCell::FenceSouthEast => "fence_south_east",
        MapCell::LedgeWest => "ledge_west",
        MapCell::LedgeMiddle => "ledge_middle",
        MapCell::LedgeEast => "ledge_east",
        MapCell::CliffNorthWest => "cliff_north_west",
        MapCell::CliffNorth => "cliff_north",
        MapCell::CliffNorthEast => "cliff_north_east",
        MapCell::CliffWest => "cliff_west",
        MapCell::CliffCenter => "cliff_center",
        MapCell::CliffEast => "cliff_east",
        MapCell::CliffSouthWest => "cliff_south_west",
        MapCell::CliffSouth => "cliff_south",
        MapCell::CliffSouthEast => "cliff_south_east",
        MapCell::CliffInnerSouthWest => "cliff_inner_south_west",
        MapCell::CliffInnerSouthEast => "cliff_inner_south_east",
        MapCell::CliffStairs => "cliff_stairs",
        MapCell::Water => "water",
        MapCell::WaterAccessEast => "water_access_east",
        MapCell::WaterAccessWest => "water_access_west",
        MapCell::WaterAccessSouth => "water_access_south",
        MapCell::Pitch => "pitch",
        MapCell::Building => "building",
        MapCell::PokecenterNorthWest => "pokecenter_north_west",
        MapCell::PokecenterNorthEast => "pokecenter_north_east",
        MapCell::PokecenterSouthWest => "pokecenter_south_west",
        MapCell::PokecenterSouthEast => "pokecenter_south_east",
        MapCell::MartNorthWest => "mart_north_west",
        MapCell::MartNorthEast => "mart_north_east",
        MapCell::MartSouthWest => "mart_south_west",
        MapCell::MartSouthEast => "mart_south_east",
        MapCell::Rail => "rail",
        MapCell::Trail => "trail",
        MapCell::Street => "street",
        MapCell::Road => "road",
        MapCell::MajorRoad => "major_road",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate, MapSource};

    fn water_grid() -> GeneratedGrid {
        GeneratedGrid {
            source: MapSource {
                center: Coordinate { lat: 0.5, lon: 0.5 },
                bounds: BoundingBox {
                    south: 0.0,
                    west: 0.0,
                    north: 1.0,
                    east: 1.0,
                },
                attribution: "test".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: 5,
            height: 5,
            cells: vec![MapCell::Water; 25],
            labels: Vec::new(),
        }
    }

    #[test]
    fn surface_dungeon_audit_rejects_blobs_and_accepts_authored_chambers() {
        let mut grid = water_grid();
        grid.width = 48;
        grid.height = 48;
        grid.cells = vec![MapCell::Grass; 48 * 48];
        for (dx, dy, cell) in crate::biomes::ice_grotto(0) {
            let x = (20_i32 + dx) as usize;
            let y = (20_i32 + dy) as usize;
            grid.cells[y * usize::from(grid.width) + x] = cell;
        }
        assert_eq!(surface_dungeon_quality(&grid, MapCell::IceFloor), Ok(()));

        grid.cells.fill(MapCell::Grass);
        for y in 15..23 {
            for x in 15..23 {
                grid.cells[y * usize::from(grid.width) + x] = MapCell::IceFloor;
            }
        }
        assert!(
            surface_dungeon_quality(&grid, MapCell::IceFloor)
                .unwrap_err()
                .contains("like a blob")
        );
    }

    #[test]
    fn reachability_treats_only_mapped_crop_edge_cells_as_external_exits() {
        let mut grid = water_grid();
        grid.cells[2 * 5 + 2] = MapCell::Grass;
        grid.cells[3 * 5 + 2] = MapCell::Lawn;
        grid.cells[0] = MapCell::MajorRoad;

        let (percent, exits) = walkable_reach(&grid);
        assert_eq!(percent, 100.0);
        assert_eq!(exits, 1);

        grid.cells[1] = MapCell::Grass;
        let (percent, exits) = walkable_reach(&grid);
        assert!(percent < 100.0, "ordinary disconnected terrain must fail");
        assert_eq!(exits, 1);
    }

    #[test]
    fn ledge_access_audit_rejects_fences_in_either_apron() {
        let mut grid = water_grid();
        grid.width = 12;
        grid.height = 11;
        grid.cells = vec![MapCell::Grass; 12 * 11];

        let (run_x, run_y, run_length) = (2_u16, 5_u16, 7_u16);
        for offset in 0..run_length {
            grid.cells[usize::from(run_y) * 12 + usize::from(run_x + offset)] = match offset {
                0 => MapCell::LedgeWest,
                value if value + 1 == run_length => MapCell::LedgeEast,
                _ => MapCell::LedgeMiddle,
            };
        }
        grid.cells[4 * 12 + 5] = MapCell::Trail;
        assert!(ledge_run_has_clear_approach_and_landing(
            &grid, run_x, run_y, run_length
        ));

        grid.cells[6 * 12 + 6] = MapCell::FenceNorth;
        assert!(
            !ledge_run_has_clear_approach_and_landing(&grid, run_x, run_y, run_length),
            "a fence at (6, 6) must invalidate the full landing apron"
        );

        grid.cells[6 * 12 + 6] = MapCell::Grass;
        grid.cells[4 * 12 + 4] = MapCell::FenceNorth;
        assert!(
            !ledge_run_has_clear_approach_and_landing(&grid, run_x, run_y, run_length),
            "a fence at (4, 4) must invalidate the full approach apron"
        );
    }

    #[test]
    fn cliff_stair_audit_requires_a_visible_trail_immediately_south() {
        let mut grid = water_grid();
        grid.cells[2 * 5 + 2] = MapCell::CliffStairs;
        grid.cells[3 * 5 + 2] = MapCell::Lawn;
        assert!(!cliff_stair_has_visible_south_approach(&grid, 2, 2));

        grid.cells[3 * 5 + 2] = MapCell::Trail;
        assert!(cliff_stair_has_visible_south_approach(&grid, 2, 2));
        assert!(
            !cliff_stair_has_visible_south_approach(&grid, 2, 4),
            "stairs on the south boundary have no usable approach"
        );
    }

    fn facility_errors(audit: &MapAudit) -> Vec<&str> {
        audit
            .errors
            .iter()
            .filter(|error| error.contains("Pokemon Center") || error.contains("Pokemon Mart"))
            .map(String::as_str)
            .collect()
    }

    fn stamp_facilities(grid: &mut GeneratedGrid, center: bool, mart: bool) {
        let mut stamp = |entries: &[(u16, u16, MapCell)]| {
            for &(x, y, cell) in entries {
                grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
            }
        };
        if center {
            stamp(&[
                (1, 1, MapCell::PokecenterNorthWest),
                (2, 1, MapCell::PokecenterNorthEast),
                (1, 2, MapCell::PokecenterSouthWest),
                (2, 2, MapCell::PokecenterSouthEast),
            ]);
        }
        if mart {
            stamp(&[
                (3, 1, MapCell::MartNorthWest),
                (4, 1, MapCell::MartNorthEast),
                (3, 2, MapCell::MartSouthWest),
                (4, 2, MapCell::MartSouthEast),
            ]);
        }
    }

    fn attach_regional_facility_plan(grid: &mut GeneratedGrid, center: bool, mart: bool) {
        let mut plan = crate::plan_h3_cell(grid.source.center, 6).expect("plan test H3 cell");
        let cell = plan.cell.clone();
        plan.regional = Some(crate::H3RegionalCellPlan {
            ordinal: 0,
            cell,
            building_count: 0,
            facilities: [
                center.then_some(H3Facility::PokemonCenter),
                mart.then_some(H3Facility::Mart),
            ]
            .into_iter()
            .flatten()
            .collect(),
            connections: Vec::new(),
            closed_transport_crossings: Vec::new(),
        });
        grid.source.h3 = Some(plan);
    }

    #[test]
    fn regional_facility_audit_distinguishes_requested_absent_and_unexpected_facades() {
        let mut empty = water_grid();
        attach_regional_facility_plan(&mut empty, false, false);
        assert_eq!(
            facility_errors(&audit_grid(&empty)),
            Vec::<&str>::new(),
            "an explicit regional allocation may omit both facilities"
        );
        let standalone = water_grid();
        assert_eq!(
            facility_errors(&audit_grid(&standalone)).len(),
            2,
            "standalone audit remains strict and requires both facilities"
        );

        for (center, mart) in [(true, false), (false, true), (true, true)] {
            let mut grid = water_grid();
            stamp_facilities(&mut grid, center, mart);
            assert_eq!(
                facility_errors(&audit_grid_with_facilities(&grid, center, mart)),
                Vec::<&str>::new(),
                "the exact requested facility combination must audit cleanly"
            );
        }

        let mut unexpected_center = water_grid();
        stamp_facilities(&mut unexpected_center, true, false);
        assert_eq!(
            facility_errors(&audit_grid_with_facilities(
                &unexpected_center,
                false,
                false
            )),
            vec!["regional plan excludes a Pokemon Center, but facade tiles were generated"]
        );

        let mut partial_mart = water_grid();
        partial_mart.cells[1 * 5 + 1] = MapCell::MartNorthWest;
        assert_eq!(
            facility_errors(&audit_grid_with_facilities(&partial_mart, false, true)),
            vec!["expected one complete canonical Pokemon Mart facade"]
        );
    }
}
