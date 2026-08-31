use anyhow::Result;

use crate::{GeneratedGrid, MapCell, stable_grid::StableGrid};

const BAY_WIDTH: u16 = 7;
const BAY_SALT: u64 = 0x524f_4144_4241_5953;
const H3_PRINCIPAL_ROUTE_MAX_DISTANCE: u16 = u16::MAX - 1;
pub(crate) const H3_MIN_ROADSIDE_COURSES: usize = 3;
pub(crate) const H3_MIN_ROADSIDE_FENCE_CELLS: usize = 12;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RoadsideSummary {
    pub bays: usize,
    pub benches: usize,
    pub trash_cans: usize,
    pub border_courses: usize,
    pub fence_cells: usize,
}

#[derive(Debug, Clone)]
struct BayProposal {
    road_y: u16,
    start_x: u16,
    side: i32,
    stable_key: u64,
    segment_key: (u16, u16, u16),
    footprint: (i32, i32, i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RoadBorderAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone)]
struct RoadBorderProposal {
    axis: RoadBorderAxis,
    road_fixed: u16,
    fence_fixed: u16,
    start: u16,
    length: u16,
    side: i32,
    segment_start: u16,
    segment_end: u16,
    building_support: usize,
    nearest_building: u16,
    mapped_support: usize,
    frontage_route: bool,
    broad_verge: bool,
    diagonal_frontage: bool,
    stable_key: u64,
}

/// Authors compact rest bays along long east/west routes.
///
/// Each bay is a complete little street scene: a canonical seven-block fence
/// course, a bench and interactive trash can, a lawn verge, and two end
/// accents. The route itself is immutable. Placements are addressed through
/// the global world-cell lattice so panning the requested square does not
/// create a new local cadence.
pub(crate) fn author_roadside_bays(grid: &mut GeneratedGrid) -> Result<RoadsideSummary> {
    let minimum_dimension = grid.width.min(grid.height);
    // H3 rooms use the compact two-cell-verge courses authored below. The
    // older full rest bay sits three cells from a road and reads as a detached
    // park strip in a 64x64 hex; it also makes it impossible to prove that the
    // visible fence really follows the regional route. Keep that larger scene
    // for square standalone maps where there is room for its full grammar.
    let bay_target = if grid.source.h3.is_some() {
        0
    } else if minimum_dimension >= 96 {
        10
    } else if minimum_dimension >= 56 {
        5
    } else if minimum_dimension >= 40 {
        3
    } else {
        0
    };
    if bay_target == 0 && road_border_course_limit(grid, 0) == 0 {
        return Ok(RoadsideSummary::default());
    }

    let stable_grid = StableGrid::for_grid(grid)?;
    let mut proposals = if bay_target > 0 {
        roadside_proposals(grid, stable_grid)
    } else {
        Vec::new()
    };
    proposals.sort_unstable_by_key(|proposal| proposal.stable_key);

    // First take at most one scene per distinct road segment. A second pass
    // may reuse a very long segment, but never inside the same gameplay view.
    let mut selected = Vec::<BayProposal>::new();
    let mut used_segments = std::collections::BTreeSet::new();
    for proposal in &proposals {
        if selected.len() >= bay_target {
            break;
        }
        if used_segments.contains(&proposal.segment_key)
            || selected
                .iter()
                .any(|other| bay_proposals_crowd(other, proposal))
        {
            continue;
        }
        used_segments.insert(proposal.segment_key);
        selected.push(proposal.clone());
    }
    for proposal in proposals {
        if selected.len() >= bay_target {
            break;
        }
        if selected
            .iter()
            .any(|other| bay_proposals_crowd(other, &proposal))
        {
            continue;
        }
        selected.push(proposal);
    }

    let mut summary = RoadsideSummary::default();
    for proposal in selected {
        stamp_roadside_bay(grid, &proposal, &mut summary);
    }
    author_road_border_runs(grid, stable_grid, &mut summary);
    Ok(summary)
}

/// Trace a small number of readable lot-edge courses beside mapped roads.
///
/// Synthetic trails can align an entrance gap but never justify a roadside
/// fence by themselves. Candidates are ranked toward the building/yard side
/// of the mapped road, share one global spacing reservation, and use only
/// straight Park perimeter pieces. The canonical corner pieces remain
/// reserved for real ninety-degree turns such as the public-field enclosure.
fn author_road_border_runs(
    grid: &mut GeneratedGrid,
    stable_grid: StableGrid,
    summary: &mut RoadsideSummary,
) {
    let target = road_border_course_limit(grid, summary.bays);
    if target == 0 {
        return;
    }

    let buildings = building_cells(grid);
    let mut proposals = road_border_proposals(grid, stable_grid, &buildings);
    let selected_landings = selected_h3_transport_landings(grid);
    proposals
        .retain(|proposal| !road_border_proposal_near_landing(proposal, &selected_landings, 2));
    let route_distances = grid
        .source
        .h3
        .is_some()
        .then(|| mapped_road_route_distances(grid));
    sort_road_border_proposals(&mut proposals);

    // First distribute scenes across distinct mapped-road stretches. If a
    // sparse map still has room, a second pass may use another well-spaced
    // course from the same long stretch, but never an adjacent parallel row.
    let mut selected = Vec::<RoadBorderProposal>::new();
    let mut used_segments = std::collections::BTreeSet::new();
    // Reserve at least half of the quota for courses backed by the building
    // side of the street. This prevents a few short boundary road stubs from
    // outranking the readable lot edges in the actual city interior.
    for require_building_support in [true, false] {
        for distinct_segments_only in [true, false] {
            for proposal in &proposals {
                if selected.len() >= target {
                    break;
                }
                if require_building_support && proposal.building_support == 0 {
                    continue;
                }
                commit_road_border_proposal_if_clear(
                    grid,
                    proposal,
                    route_distances.as_deref(),
                    distinct_segments_only,
                    &mut used_segments,
                    &mut selected,
                    summary,
                );
            }
        }
    }

    // Ordinary two-cell-offset frontage can be exhausted by a dense H3 city
    // face. Fill an under-quota result with complete three-cell, broad-
    // boulevard, or one-step diagonal courses, committing one at a time
    // against the mutated grid. This can rescue zero, one, or two ordinary
    // courses; it stops at the same three-course/twelve-cell quality gate used
    // by the final audit, or at true candidate exhaustion. The four-course cap
    // prevents a perimeter fence ring even when a face has many eligible
    // streets.
    if grid.source.h3.is_some() && !h3_roadside_quota_met(&selected) {
        let mut fallback = h3_broad_verge_border_proposals(grid, stable_grid, &buildings);
        fallback.extend(h3_compact_frontage_border_proposals(
            grid,
            stable_grid,
            &buildings,
        ));
        fallback.extend(h3_diagonal_frontage_border_proposals(
            grid,
            stable_grid,
            &buildings,
        ));
        fallback
            .retain(|proposal| !road_border_proposal_near_landing(proposal, &selected_landings, 2));
        sort_road_border_proposals(&mut fallback);

        for require_building_support in [true, false] {
            for distinct_segments_only in [true, false] {
                for proposal in &fallback {
                    if selected.len() >= target || h3_roadside_quota_met(&selected) {
                        break;
                    }
                    if require_building_support && proposal.building_support == 0 {
                        continue;
                    }
                    commit_road_border_proposal_if_clear(
                        grid,
                        proposal,
                        route_distances.as_deref(),
                        distinct_segments_only,
                        &mut used_segments,
                        &mut selected,
                        summary,
                    );
                }
            }
        }
    }
    author_h3_roadside_fixtures(grid, &selected, &selected_landings, summary);
}

fn sort_road_border_proposals(proposals: &mut [RoadBorderProposal]) {
    proposals.sort_unstable_by_key(|proposal| {
        (
            std::cmp::Reverse(proposal.building_support > 0),
            std::cmp::Reverse(proposal.building_support),
            std::cmp::Reverse(proposal.mapped_support),
            proposal.nearest_building,
            std::cmp::Reverse(proposal.length),
            proposal.stable_key,
        )
    });
}

fn h3_roadside_quota_met(selected: &[RoadBorderProposal]) -> bool {
    selected.len() >= H3_MIN_ROADSIDE_COURSES
        && selected
            .iter()
            .map(|proposal| usize::from(proposal.length))
            .sum::<usize>()
            >= H3_MIN_ROADSIDE_FENCE_CELLS
}

fn commit_road_border_proposal_if_clear(
    grid: &mut GeneratedGrid,
    proposal: &RoadBorderProposal,
    route_distances: Option<&[u16]>,
    distinct_segments_only: bool,
    used_segments: &mut std::collections::BTreeSet<(RoadBorderAxis, u16, u16, u16)>,
    selected: &mut Vec<RoadBorderProposal>,
    summary: &mut RoadsideSummary,
) -> bool {
    let segment_key = (
        proposal.axis,
        proposal.road_fixed,
        proposal.segment_start,
        proposal.segment_end,
    );
    if (distinct_segments_only && used_segments.contains(&segment_key))
        || selected
            .iter()
            .any(|other| road_border_proposals_crowd(other, proposal))
        || !road_border_proposal_is_clear(grid, proposal, route_distances)
    {
        return false;
    }

    let cell = road_border_cell(proposal);
    for offset in 0..proposal.length {
        let (x, y) = road_border_coordinates(proposal, offset);
        replace_non_route(grid, i32::from(x), i32::from(y), cell);
        summary.fence_cells += 1;
    }
    summary.border_courses += 1;
    used_segments.insert(segment_key);
    selected.push(proposal.clone());
    true
}

fn road_border_course_limit(grid: &GeneratedGrid, bays: usize) -> usize {
    let minimum_dimension = grid.width.min(grid.height);
    if grid.source.h3.is_some() && minimum_dimension >= 56 {
        return 4_usize.saturating_sub(bays.min(4));
    }
    if minimum_dimension >= 96 {
        // A city-scale square contains four times the visible area of the
        // original 64x64 room. Keep courses sparse, but scale their count with
        // linear map extent so fences consistently frame separate streets and
        // lots instead of disappearing into one small neighborhood.
        // Include one additional civic/lot course beyond the regular
        // one-per-eight-block cadence so dense 128x128 city maps still clear
        // the 120-cell authored-fence gate when several candidates are short.
        usize::from(minimum_dimension).div_ceil(8) + 3
    } else if minimum_dimension >= 56 {
        4
    } else if minimum_dimension >= 40 {
        2
    } else {
        0
    }
}

fn selected_h3_transport_landings(grid: &GeneratedGrid) -> std::collections::BTreeSet<(u16, u16)> {
    let Some(plan) = grid.source.h3.as_ref() else {
        return std::collections::BTreeSet::new();
    };
    let Some(regional) = plan.regional.as_ref() else {
        return std::collections::BTreeSet::new();
    };
    regional
        .connections
        .iter()
        .flat_map(|connection| {
            crate::h3::h3_raster_sample_band(plan, grid, connection.coordinate)
                .expect("validated H3 regional landing must rasterize")
        })
        .collect()
}

fn road_border_proposal_near_landing(
    proposal: &RoadBorderProposal,
    landings: &std::collections::BTreeSet<(u16, u16)>,
    radius: u16,
) -> bool {
    (0..proposal.length).any(|offset| {
        let (x, y) = road_border_coordinates(proposal, offset);
        landings.iter().any(|&(landing_x, landing_y)| {
            x.abs_diff(landing_x) <= radius && y.abs_diff(landing_y) <= radius
        })
    })
}

/// Put a small amount of real street furniture in the walkable verge between
/// selected H3 courses and their routes. The furniture never substitutes for
/// a fence tile, and at most one station per course loses its otherwise clear
/// verge, so the road edge remains legible and traversable instead of becoming
/// a row of collision props.
fn author_h3_roadside_fixtures(
    grid: &mut GeneratedGrid,
    courses: &[RoadBorderProposal],
    landings: &std::collections::BTreeSet<(u16, u16)>,
    summary: &mut RoadsideSummary,
) {
    if grid.source.h3.is_none() || courses.is_empty() {
        return;
    }

    let target = courses.len().min(2);
    let mut placed = 0_usize;
    let mut placed_bench = false;
    let mut placed_trash = false;
    let route_distances = mapped_road_route_distances(grid);
    for course in courses {
        if placed >= target {
            break;
        }
        let mut offsets = (0..course.length).collect::<Vec<_>>();
        offsets.sort_unstable_by_key(|&offset| {
            (
                offset.abs_diff(course.length / 2),
                course.stable_key.rotate_left(u32::from(offset) & 63),
            )
        });
        let site = offsets.into_iter().find_map(|offset| {
            let (fence_x, fence_y) = road_border_coordinates(course, offset);
            let orientation = road_border_cell(course);
            roadside_support_geometry(grid, &route_distances, fence_x, fence_y, orientation, false)?
                .verge_cells
                .into_iter()
                .find(|&(x, y)| roadside_fixture_site_is_clear(grid, x, y, landings))
        });
        let Some((x, y)) = site else {
            continue;
        };

        // The canonical Park bench faces south, so prefer it on the north
        // edge of an east/west street. Other orientations receive the
        // canonical interactive trash can; a later suitable north course can
        // still supply the bench.
        let cell = if !placed_bench && course.axis == RoadBorderAxis::Horizontal && course.side < 0
        {
            placed_bench = true;
            summary.benches += 1;
            MapCell::Bench
        } else if !placed_trash {
            placed_trash = true;
            summary.trash_cans += 1;
            MapCell::TrashCan
        } else if course.axis == RoadBorderAxis::Horizontal && course.side < 0 {
            placed_bench = true;
            summary.benches += 1;
            MapCell::Bench
        } else {
            summary.trash_cans += 1;
            MapCell::TrashCan
        };
        replace_non_route(grid, i32::from(x), i32::from(y), cell);
        placed += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RoadsideSupportGeometry {
    verge_cells: Vec<(u16, u16)>,
}

/// Resolve the route and complete intervening verge for one straight fence
/// station. Ordinary courses use one verge cell; broad boulevards use two.
/// A one-step rasterized diagonal may shift the route station by one along the
/// course, in which case the complete between-cell strip is reserved. Both
/// fixture placement and final-grid classification call this function, so a
/// broad course cannot be authored under geometry the audit cannot recognize.
fn roadside_support_geometry(
    grid: &GeneratedGrid,
    route_distances: &[u16],
    fence_x: u16,
    fence_y: u16,
    orientation: MapCell,
    allow_roadside_amenity: bool,
) -> Option<RoadsideSupportGeometry> {
    let (normal_x, normal_y, along_x, along_y) = match orientation {
        MapCell::FenceSouth => (0_i32, 1_i32, 1_i32, 0_i32),
        MapCell::FenceNorth => (0, -1, 1, 0),
        MapCell::FenceEast => (1, 0, 0, 1),
        MapCell::FenceWest => (-1, 0, 0, 1),
        _ => return None,
    };
    let mut candidates = Vec::<((i32, i32), RoadsideSupportGeometry)>::new();
    for distance in 1_i32..=3 {
        for along in [0_i32, -1, 1] {
            let route_x = i32::from(fence_x) + normal_x * distance + along_x * along;
            let route_y = i32::from(fence_y) + normal_y * distance + along_y * along;
            if route_x < 0
                || route_y < 0
                || route_x >= i32::from(grid.width)
                || route_y >= i32::from(grid.height)
            {
                continue;
            }
            let route_x = route_x as u16;
            let route_y = route_y as u16;
            let route_index = usize::from(route_y) * usize::from(grid.width) + usize::from(route_x);
            if !is_route(grid.cell(route_x, route_y))
                || route_distances[route_index] == u16::MAX
                || route_neighbor_count(grid, route_x, route_y) > 2
                || !route_has_parallel_neighbor(grid, route_x, route_y, orientation)
            {
                continue;
            }

            let mut verge_cells = Vec::new();
            let along_start = along.min(0);
            let along_end = along.max(0);
            for normal_step in 1..distance {
                for along_step in along_start..=along_end {
                    let x = i32::from(fence_x) + normal_x * normal_step + along_x * along_step;
                    let y = i32::from(fence_y) + normal_y * normal_step + along_y * along_step;
                    if x < 0 || y < 0 || x >= i32::from(grid.width) || y >= i32::from(grid.height) {
                        verge_cells.clear();
                        break;
                    }
                    let x = x as u16;
                    let y = y as u16;
                    let valid = walkable_verge(grid.cell(x, y))
                        || (allow_roadside_amenity
                            && matches!(grid.cell(x, y), Some(MapCell::Bench | MapCell::TrashCan)));
                    if !valid {
                        verge_cells.clear();
                        break;
                    }
                    verge_cells.push((x, y));
                }
                if verge_cells.is_empty() && distance > 1 {
                    break;
                }
            }
            if distance > 1 && verge_cells.is_empty() {
                continue;
            }
            // Keep normal-step order: fixture placement deliberately tries
            // the fence-side verge cell first. The final classifier consumes
            // this same complete corridor, so a two-cell boulevard verge is
            // not judged by a contradictory one-cell proximity heuristic.
            verge_cells.dedup();
            candidates.push((
                (distance, along.abs()),
                RoadsideSupportGeometry { verge_cells },
            ));
        }
    }
    candidates.sort_unstable_by_key(|candidate| candidate.0);
    let best_key = candidates.first()?.0;
    let mut best = candidates
        .into_iter()
        .filter(|candidate| candidate.0 == best_key);
    let geometry = best.next()?.1;
    best.next().is_none().then_some(geometry)
}

fn route_has_parallel_neighbor(
    grid: &GeneratedGrid,
    route_x: u16,
    route_y: u16,
    orientation: MapCell,
) -> bool {
    match orientation {
        MapCell::FenceNorth | MapCell::FenceSouth => {
            route_x
                .checked_sub(1)
                .is_some_and(|x| is_route(grid.cell(x, route_y)))
                || route_x
                    .checked_add(1)
                    .filter(|&x| x < grid.width)
                    .is_some_and(|x| is_route(grid.cell(x, route_y)))
        }
        MapCell::FenceEast | MapCell::FenceWest => {
            route_y
                .checked_sub(1)
                .is_some_and(|y| is_route(grid.cell(route_x, y)))
                || route_y
                    .checked_add(1)
                    .filter(|&y| y < grid.height)
                    .is_some_and(|y| is_route(grid.cell(route_x, y)))
        }
        _ => false,
    }
}

fn roadside_fixture_site_is_clear(
    grid: &GeneratedGrid,
    x: u16,
    y: u16,
    landings: &std::collections::BTreeSet<(u16, u16)>,
) -> bool {
    walkable_verge(grid.cell(x, y))
        && !near_building_front(grid, x, y)
        && !near_roadside_obstacle(grid, x, y, 1)
        && !crate::grid::near_relief(grid, x, y, 1)
        && h3_fence_footprint_fits(grid, x, y)
        && !landings
            .iter()
            .any(|&(landing_x, landing_y)| x.abs_diff(landing_x) <= 2 && y.abs_diff(landing_y) <= 2)
        && !(y.saturating_sub(2)..=(y + 2).min(grid.height - 1)).any(|check_y| {
            (x.saturating_sub(2)..=(x + 2).min(grid.width - 1)).any(|check_x| {
                matches!(
                    grid.cell(check_x, check_y),
                    Some(MapCell::Bench | MapCell::TrashCan | MapCell::Fountain)
                )
            })
        })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RoadsideFenceQuality {
    pub public_perimeter_cells: usize,
    pub roadside_courses: usize,
    pub roadside_fence_cells: usize,
    pub route_supported_cells: usize,
    pub urban_supported_courses: usize,
    pub malformed_courses: usize,
    pub landing_conflicts: usize,
    pub terrain_conflicts: usize,
    pub roadside_amenities: usize,
    pub orientation_variants: usize,
}

/// Separates the canonical cornered public-field enclosure from straight road
/// courses, then proves the latter still read as road edges in the final grid.
/// This runs after connectivity repair, so a course broken by a late clearing
/// cannot hide behind the generator's pre-repair placement summary.
pub(crate) fn roadside_fence_quality(grid: &GeneratedGrid) -> RoadsideFenceQuality {
    let width = usize::from(grid.width);
    let mut remaining = grid
        .cells
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, cell)| is_fence_cell(cell).then_some(index))
        .collect::<std::collections::BTreeSet<_>>();
    let mut quality = RoadsideFenceQuality::default();
    let mut roadside_cells = std::collections::BTreeMap::<(u16, u16), MapCell>::new();
    let mut orientations = std::collections::BTreeSet::<u8>::new();
    let landings = selected_h3_transport_landings(grid);
    let route_distances = mapped_road_route_distances(grid);
    let buildings = building_cells(grid);

    while let Some(start) = remaining.pop_first() {
        let mut frontier = std::collections::VecDeque::from([start]);
        let mut component = Vec::new();
        while let Some(index) = frontier.pop_front() {
            component.push(index);
            let x = index % width;
            let y = index / width;
            for (next_x, next_y) in [
                (x.checked_sub(1), Some(y)),
                (
                    x.checked_add(1)
                        .filter(|&next| next < usize::from(grid.width)),
                    Some(y),
                ),
                (Some(x), y.checked_sub(1)),
                (
                    Some(x),
                    y.checked_add(1)
                        .filter(|&next| next < usize::from(grid.height)),
                ),
            ] {
                let (Some(next_x), Some(next_y)) = (next_x, next_y) else {
                    continue;
                };
                let next = next_y * width + next_x;
                if remaining.remove(&next) {
                    frontier.push_back(next);
                }
            }
        }

        if component
            .iter()
            .any(|&index| is_fence_corner(grid.cells[index]))
        {
            quality.public_perimeter_cells += component.len();
            continue;
        }

        quality.roadside_courses += 1;
        quality.roadside_fence_cells += component.len();
        let cells = component
            .iter()
            .map(|&index| {
                (
                    (index % width) as u16,
                    (index / width) as u16,
                    grid.cells[index],
                )
            })
            .collect::<Vec<_>>();
        for &(x, y, cell) in &cells {
            roadside_cells.insert((x, y), cell);
        }

        let orientation = cells[0].2;
        orientations.insert(match orientation {
            MapCell::FenceNorth => 0,
            MapCell::FenceSouth => 1,
            MapCell::FenceEast => 2,
            MapCell::FenceWest => 3,
            _ => 4,
        });
        if !(3..=10).contains(&cells.len())
            || cells.iter().any(|cell| cell.2 != orientation)
            || !straight_course_is_contiguous(&cells, orientation)
        {
            quality.malformed_courses += 1;
        }
        if course_has_urban_support(&cells, orientation, &buildings) {
            quality.urban_supported_courses += 1;
        }
        for &(x, y, _) in &cells {
            if roadside_support_geometry(grid, &route_distances, x, y, orientation, false).is_some()
            {
                quality.route_supported_cells += 1;
            }
            if landings.iter().any(|&(landing_x, landing_y)| {
                x.abs_diff(landing_x) <= 2 && y.abs_diff(landing_y) <= 2
            }) {
                quality.landing_conflicts += 1;
            }
            if near_roadside_obstacle(grid, x, y, 1) || crate::grid::near_relief(grid, x, y, 1) {
                quality.terrain_conflicts += 1;
            }
        }
    }
    quality.orientation_variants = orientations.len();

    quality.roadside_amenities = roadside_cells
        .iter()
        .filter_map(|(&(x, y), &orientation)| {
            roadside_support_geometry(grid, &route_distances, x, y, orientation, true)
        })
        .flat_map(|geometry| geometry.verge_cells)
        .filter(|&(x, y)| matches!(grid.cell(x, y), Some(MapCell::Bench | MapCell::TrashCan)))
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    quality
}

fn is_fence_cell(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::FenceNorthWest
            | MapCell::FenceNorth
            | MapCell::FenceNorthEast
            | MapCell::FenceWest
            | MapCell::FenceEast
            | MapCell::FenceSouthWest
            | MapCell::FenceSouth
            | MapCell::FenceSouthEast
    )
}

fn is_fence_corner(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::FenceNorthWest
            | MapCell::FenceNorthEast
            | MapCell::FenceSouthWest
            | MapCell::FenceSouthEast
    )
}

fn straight_course_is_contiguous(cells: &[(u16, u16, MapCell)], orientation: MapCell) -> bool {
    let mut positions = match orientation {
        MapCell::FenceNorth | MapCell::FenceSouth => {
            if cells.iter().any(|cell| cell.1 != cells[0].1) {
                return false;
            }
            cells.iter().map(|cell| cell.0).collect::<Vec<_>>()
        }
        MapCell::FenceEast | MapCell::FenceWest => {
            if cells.iter().any(|cell| cell.0 != cells[0].0) {
                return false;
            }
            cells.iter().map(|cell| cell.1).collect::<Vec<_>>()
        }
        _ => return false,
    };
    positions.sort_unstable();
    positions.windows(2).all(|pair| pair[1] == pair[0] + 1)
}

fn course_has_urban_support(
    cells: &[(u16, u16, MapCell)],
    orientation: MapCell,
    buildings: &[(u16, u16)],
) -> bool {
    cells.iter().any(|&(x, y, _)| {
        buildings
            .iter()
            .copied()
            .any(|(building_x, building_y)| match orientation {
                MapCell::FenceSouth => {
                    (1..=8).contains(&y.saturating_sub(building_y)) && x.abs_diff(building_x) <= 2
                }
                MapCell::FenceNorth => {
                    (1..=8).contains(&building_y.saturating_sub(y)) && x.abs_diff(building_x) <= 2
                }
                MapCell::FenceEast => {
                    (1..=8).contains(&x.saturating_sub(building_x)) && y.abs_diff(building_y) <= 2
                }
                MapCell::FenceWest => {
                    (1..=8).contains(&building_x.saturating_sub(x)) && y.abs_diff(building_y) <= 2
                }
                _ => false,
            })
    })
}

fn road_border_proposals(
    grid: &GeneratedGrid,
    stable_grid: StableGrid,
    buildings: &[(u16, u16)],
) -> Vec<RoadBorderProposal> {
    const BORDER_SALT: u64 = 0x524f_4144_4645_4e43;
    const MAX_COURSE: u16 = 10;
    // A three-block course is still a readable Crystal lot edge. Ordinary
    // city rasters need that compact grammar between closely spaced crossing
    // gates, buildings, trees, and terrain accents. H3 keeps its established
    // six-block straight-course search before its dedicated frontage fallback.
    let min_course = if grid.source.h3.is_some() { 6 } else { 3 };
    let mut proposals = Vec::new();

    for road_y in 3..grid.height.saturating_sub(3) {
        if (0..grid.width)
            .filter(|&x| is_mapped_road(grid.cell(x, road_y)))
            .count()
            < usize::from(min_course)
        {
            continue;
        }
        for side in [-1_i32, 1_i32] {
            let fence_y = (i32::from(road_y) + side * 2) as u16;
            let mut x = 2_u16;
            while x + 2 < grid.width {
                if !road_border_site_is_clear(grid, x, road_y, fence_y) {
                    x += 1;
                    continue;
                }
                let segment_start = x;
                while x + 2 < grid.width && road_border_site_is_clear(grid, x, road_y, fence_y) {
                    x += 1;
                }
                let segment_end = x;
                let mut course_start = segment_start;
                while course_start <= segment_end.saturating_sub(min_course) {
                    let length = (segment_end - course_start).min(MAX_COURSE);
                    if let Some(anchor) = stable_grid.cell(course_start + length / 2, fence_y) {
                        let mut proposal = RoadBorderProposal {
                            axis: RoadBorderAxis::Horizontal,
                            road_fixed: road_y,
                            fence_fixed: fence_y,
                            start: course_start,
                            length,
                            side,
                            segment_start,
                            segment_end,
                            building_support: 0,
                            nearest_building: u16::MAX,
                            mapped_support: usize::from(length),
                            frontage_route: false,
                            broad_verge: false,
                            diagonal_frontage: false,
                            stable_key: anchor.offset(0, i64::from(side)).stable_hash(BORDER_SALT),
                        };
                        (proposal.building_support, proposal.nearest_building) =
                            road_border_building_metrics(&proposal, buildings);
                        proposals.push(proposal);
                    }
                    course_start = course_start.saturating_add(length + 3);
                }
            }
        }
    }

    for road_x in 3..grid.width.saturating_sub(3) {
        if (0..grid.height)
            .filter(|&y| is_mapped_road(grid.cell(road_x, y)))
            .count()
            < usize::from(min_course)
        {
            continue;
        }
        for side in [-1_i32, 1_i32] {
            let fence_x = (i32::from(road_x) + side * 2) as u16;
            let mut y = 2_u16;
            while y + 2 < grid.height {
                if !vertical_road_border_site_is_clear(grid, road_x, y, fence_x) {
                    y += 1;
                    continue;
                }
                let segment_start = y;
                while y + 2 < grid.height
                    && vertical_road_border_site_is_clear(grid, road_x, y, fence_x)
                {
                    y += 1;
                }
                let segment_end = y;
                let mut course_start = segment_start;
                while course_start <= segment_end.saturating_sub(min_course) {
                    let length = (segment_end - course_start).min(MAX_COURSE);
                    if let Some(anchor) = stable_grid.cell(fence_x, course_start + length / 2) {
                        let mut proposal = RoadBorderProposal {
                            axis: RoadBorderAxis::Vertical,
                            road_fixed: road_x,
                            fence_fixed: fence_x,
                            start: course_start,
                            length,
                            side,
                            segment_start,
                            segment_end,
                            building_support: 0,
                            nearest_building: u16::MAX,
                            mapped_support: usize::from(length),
                            frontage_route: false,
                            broad_verge: false,
                            diagonal_frontage: false,
                            stable_key: anchor
                                .offset(i64::from(side), 0)
                                .stable_hash(BORDER_SALT ^ 0x5645_5254),
                        };
                        (proposal.building_support, proposal.nearest_building) =
                            road_border_building_metrics(&proposal, buildings);
                        proposals.push(proposal);
                    }
                    course_start = course_start.saturating_add(length + 3);
                }
            }
        }
    }
    if grid.source.h3.is_some() {
        proposals.extend(h3_frontage_border_proposals(grid, stable_grid, buildings));
    }
    proposals
}

/// Sparse H3 rasters often reduce a real mapped road to a short boundary
/// landing while the principal, four-neighbor-connected route continues as
/// Trail through the room. A frontage course may follow that connected route
/// when it is either backed by a real building lot or contains mapped-road
/// support itself. Disconnected woodland trails keep an infinite route
/// distance and can never author a fence.
fn h3_frontage_border_proposals(
    grid: &GeneratedGrid,
    stable_grid: StableGrid,
    buildings: &[(u16, u16)],
) -> Vec<RoadBorderProposal> {
    h3_frontage_border_proposals_with_lengths(grid, stable_grid, buildings, 4, 7)
}

/// Last-resort straight H3 frontage beside a dense lot or protected terrain.
///
/// These proposals are considered only when ordinary courses have not met the
/// regional quota. Pinning both bounds to three prevents a long clear street
/// from changing its normal course cadence merely because another part of the
/// face is crowded.
fn h3_compact_frontage_border_proposals(
    grid: &GeneratedGrid,
    stable_grid: StableGrid,
    buildings: &[(u16, u16)],
) -> Vec<RoadBorderProposal> {
    h3_frontage_border_proposals_with_lengths(grid, stable_grid, buildings, 3, 3)
}

fn h3_frontage_border_proposals_with_lengths(
    grid: &GeneratedGrid,
    stable_grid: StableGrid,
    buildings: &[(u16, u16)],
    min_course: u16,
    max_course: u16,
) -> Vec<RoadBorderProposal> {
    const BORDER_SALT: u64 = 0x4833_4652_4f4e_5447;
    const MAX_ROUTE_DISTANCE: u16 = H3_PRINCIPAL_ROUTE_MAX_DISTANCE;
    const MAX_BUILDING_DISTANCE: u16 = 10;
    let route_distances = mapped_road_route_distances(grid);
    let mut proposals = Vec::new();

    for road_y in 3..grid.height.saturating_sub(3) {
        for side in [-1_i32, 1_i32] {
            let fence_y = (i32::from(road_y) + side * 2) as u16;
            let mut x = 2_u16;
            while x + 2 < grid.width {
                if !h3_frontage_site_is_clear(
                    grid,
                    &route_distances,
                    x,
                    road_y,
                    fence_y,
                    RoadBorderAxis::Horizontal,
                    MAX_ROUTE_DISTANCE,
                ) {
                    x += 1;
                    continue;
                }
                let segment_start = x;
                while x + 2 < grid.width
                    && h3_frontage_site_is_clear(
                        grid,
                        &route_distances,
                        x,
                        road_y,
                        fence_y,
                        RoadBorderAxis::Horizontal,
                        MAX_ROUTE_DISTANCE,
                    )
                {
                    x += 1;
                }
                let segment_end = x;
                let mut course_start = segment_start;
                while course_start <= segment_end.saturating_sub(min_course) {
                    let length = (segment_end - course_start).min(max_course);
                    if let Some(anchor) = stable_grid.cell(course_start + length / 2, fence_y) {
                        let mut proposal = RoadBorderProposal {
                            axis: RoadBorderAxis::Horizontal,
                            road_fixed: road_y,
                            fence_fixed: fence_y,
                            start: course_start,
                            length,
                            side,
                            segment_start,
                            segment_end,
                            building_support: 0,
                            nearest_building: u16::MAX,
                            mapped_support: (course_start..course_start + length)
                                .filter(|&route_x| is_mapped_road(grid.cell(route_x, road_y)))
                                .count(),
                            frontage_route: true,
                            broad_verge: false,
                            diagonal_frontage: false,
                            stable_key: anchor.offset(0, i64::from(side)).stable_hash(BORDER_SALT),
                        };
                        (proposal.building_support, proposal.nearest_building) =
                            road_border_building_metrics(&proposal, buildings);
                        if proposal.mapped_support > 0
                            || (proposal.building_support > 0
                                && proposal.nearest_building <= MAX_BUILDING_DISTANCE)
                        {
                            proposals.push(proposal);
                        }
                    }
                    course_start = course_start.saturating_add(length + 3);
                }
            }
        }
    }

    for road_x in 3..grid.width.saturating_sub(3) {
        for side in [-1_i32, 1_i32] {
            let fence_x = (i32::from(road_x) + side * 2) as u16;
            let mut y = 2_u16;
            while y + 2 < grid.height {
                if !h3_frontage_site_is_clear(
                    grid,
                    &route_distances,
                    road_x,
                    y,
                    fence_x,
                    RoadBorderAxis::Vertical,
                    MAX_ROUTE_DISTANCE,
                ) {
                    y += 1;
                    continue;
                }
                let segment_start = y;
                while y + 2 < grid.height
                    && h3_frontage_site_is_clear(
                        grid,
                        &route_distances,
                        road_x,
                        y,
                        fence_x,
                        RoadBorderAxis::Vertical,
                        MAX_ROUTE_DISTANCE,
                    )
                {
                    y += 1;
                }
                let segment_end = y;
                let mut course_start = segment_start;
                while course_start <= segment_end.saturating_sub(min_course) {
                    let length = (segment_end - course_start).min(max_course);
                    if let Some(anchor) = stable_grid.cell(fence_x, course_start + length / 2) {
                        let mut proposal = RoadBorderProposal {
                            axis: RoadBorderAxis::Vertical,
                            road_fixed: road_x,
                            fence_fixed: fence_x,
                            start: course_start,
                            length,
                            side,
                            segment_start,
                            segment_end,
                            building_support: 0,
                            nearest_building: u16::MAX,
                            mapped_support: (course_start..course_start + length)
                                .filter(|&route_y| is_mapped_road(grid.cell(road_x, route_y)))
                                .count(),
                            frontage_route: true,
                            broad_verge: false,
                            diagonal_frontage: false,
                            stable_key: anchor
                                .offset(i64::from(side), 0)
                                .stable_hash(BORDER_SALT ^ 0x5645_5254),
                        };
                        (proposal.building_support, proposal.nearest_building) =
                            road_border_building_metrics(&proposal, buildings);
                        if proposal.mapped_support > 0
                            || (proposal.building_support > 0
                                && proposal.nearest_building <= MAX_BUILDING_DISTANCE)
                        {
                            proposals.push(proposal);
                        }
                    }
                    course_start = course_start.saturating_add(length + 3);
                }
            }
        }
    }
    proposals
}

fn h3_broad_verge_border_proposals(
    grid: &GeneratedGrid,
    stable_grid: StableGrid,
    buildings: &[(u16, u16)],
) -> Vec<RoadBorderProposal> {
    const BORDER_SALT: u64 = 0x4833_4252_4f41_4456;
    const MIN_COURSE: u16 = 3;
    const MAX_COURSE: u16 = 6;
    const MAX_ROUTE_DISTANCE: u16 = H3_PRINCIPAL_ROUTE_MAX_DISTANCE;
    const MAX_BUILDING_DISTANCE: u16 = 10;
    let route_distances = mapped_road_route_distances(grid);
    let mut proposals = Vec::new();

    for road_y in 4..grid.height.saturating_sub(4) {
        for side in [-1_i32, 1_i32] {
            let fence_y = (i32::from(road_y) + side * 3) as u16;
            for start in 2..grid.width.saturating_sub(2) {
                for length in MIN_COURSE..=MAX_COURSE {
                    if start.saturating_add(length).saturating_add(1) > grid.width {
                        break;
                    }
                    if !(0..length).all(|offset| {
                        h3_broad_frontage_site_is_clear(
                            grid,
                            &route_distances,
                            start + offset,
                            road_y,
                            fence_y,
                            RoadBorderAxis::Horizontal,
                            MAX_ROUTE_DISTANCE,
                        )
                    }) {
                        continue;
                    }
                    let Some(anchor) = stable_grid.cell(start + length / 2, fence_y) else {
                        continue;
                    };
                    let mut proposal = RoadBorderProposal {
                        axis: RoadBorderAxis::Horizontal,
                        road_fixed: road_y,
                        fence_fixed: fence_y,
                        start,
                        length,
                        side,
                        segment_start: start,
                        segment_end: start + length,
                        building_support: 0,
                        nearest_building: u16::MAX,
                        mapped_support: (start..start + length)
                            .filter(|&x| is_mapped_road(grid.cell(x, road_y)))
                            .count(),
                        frontage_route: true,
                        broad_verge: true,
                        diagonal_frontage: false,
                        stable_key: anchor.offset(0, i64::from(side)).stable_hash(BORDER_SALT),
                    };
                    (proposal.building_support, proposal.nearest_building) =
                        road_border_building_metrics(&proposal, buildings);
                    if proposal.mapped_support > 0
                        || proposal.nearest_building <= MAX_BUILDING_DISTANCE
                    {
                        proposals.push(proposal);
                    }
                }
            }
        }
    }

    for road_x in 4..grid.width.saturating_sub(4) {
        for side in [-1_i32, 1_i32] {
            let fence_x = (i32::from(road_x) + side * 3) as u16;
            for start in 2..grid.height.saturating_sub(2) {
                for length in MIN_COURSE..=MAX_COURSE {
                    if start.saturating_add(length).saturating_add(1) > grid.height {
                        break;
                    }
                    if !(0..length).all(|offset| {
                        h3_broad_frontage_site_is_clear(
                            grid,
                            &route_distances,
                            road_x,
                            start + offset,
                            fence_x,
                            RoadBorderAxis::Vertical,
                            MAX_ROUTE_DISTANCE,
                        )
                    }) {
                        continue;
                    }
                    let Some(anchor) = stable_grid.cell(fence_x, start + length / 2) else {
                        continue;
                    };
                    let mut proposal = RoadBorderProposal {
                        axis: RoadBorderAxis::Vertical,
                        road_fixed: road_x,
                        fence_fixed: fence_x,
                        start,
                        length,
                        side,
                        segment_start: start,
                        segment_end: start + length,
                        building_support: 0,
                        nearest_building: u16::MAX,
                        mapped_support: (start..start + length)
                            .filter(|&y| is_mapped_road(grid.cell(road_x, y)))
                            .count(),
                        frontage_route: true,
                        broad_verge: true,
                        diagonal_frontage: false,
                        stable_key: anchor
                            .offset(i64::from(side), 0)
                            .stable_hash(BORDER_SALT ^ 0x5645_5254),
                    };
                    (proposal.building_support, proposal.nearest_building) =
                        road_border_building_metrics(&proposal, buildings);
                    if proposal.mapped_support > 0
                        || proposal.nearest_building <= MAX_BUILDING_DISTANCE
                    {
                        proposals.push(proposal);
                    }
                }
            }
        }
    }
    proposals
}

/// Straight courses beside a one-step rasterized diagonal frontage route.
///
/// Each fence station maps to one unique nearest route cell at the nominal
/// two-cell offset and at most one station left or right. The mapped stations
/// must advance monotonically without skipping, use only non-branching route
/// cells connected to mapped transport, and retain a completely clear verge.
fn h3_diagonal_frontage_border_proposals(
    grid: &GeneratedGrid,
    stable_grid: StableGrid,
    buildings: &[(u16, u16)],
) -> Vec<RoadBorderProposal> {
    const BORDER_SALT: u64 = 0x4833_4449_4147_4f4e;
    const MIN_COURSE: u16 = 3;
    const MAX_COURSE: u16 = 6;
    const MAX_ROUTE_DISTANCE: u16 = H3_PRINCIPAL_ROUTE_MAX_DISTANCE;
    const MAX_BUILDING_DISTANCE: u16 = 10;
    let route_distances = mapped_road_route_distances(grid);
    let mut proposals = Vec::new();

    for road_y in 3..grid.height.saturating_sub(3) {
        for side in [-1_i32, 1_i32] {
            let fence_y = (i32::from(road_y) + side * 2) as u16;
            for start in 2..grid.width.saturating_sub(2) {
                for length in MIN_COURSE..=MAX_COURSE {
                    if start.saturating_add(length).saturating_add(1) > grid.width {
                        break;
                    }
                    if !h3_diagonal_frontage_course_is_clear(
                        grid,
                        &route_distances,
                        RoadBorderAxis::Horizontal,
                        road_y,
                        fence_y,
                        start,
                        length,
                        MAX_ROUTE_DISTANCE,
                    ) {
                        continue;
                    }
                    let Some(anchor) = stable_grid.cell(start + length / 2, fence_y) else {
                        continue;
                    };
                    let mut proposal = RoadBorderProposal {
                        axis: RoadBorderAxis::Horizontal,
                        road_fixed: road_y,
                        fence_fixed: fence_y,
                        start,
                        length,
                        side,
                        segment_start: start,
                        segment_end: start + length,
                        building_support: 0,
                        nearest_building: u16::MAX,
                        mapped_support: (start..start + length)
                            .filter(|&x| {
                                (x.saturating_sub(1)..=(x + 1).min(grid.width - 1))
                                    .any(|mapped_x| is_mapped_road(grid.cell(mapped_x, road_y)))
                            })
                            .count(),
                        frontage_route: true,
                        broad_verge: false,
                        diagonal_frontage: true,
                        stable_key: anchor.offset(0, i64::from(side)).stable_hash(BORDER_SALT),
                    };
                    (proposal.building_support, proposal.nearest_building) =
                        road_border_building_metrics(&proposal, buildings);
                    if proposal.mapped_support > 0
                        || proposal.nearest_building <= MAX_BUILDING_DISTANCE
                    {
                        proposals.push(proposal);
                    }
                }
            }
        }
    }

    for road_x in 3..grid.width.saturating_sub(3) {
        for side in [-1_i32, 1_i32] {
            let fence_x = (i32::from(road_x) + side * 2) as u16;
            for start in 2..grid.height.saturating_sub(2) {
                for length in MIN_COURSE..=MAX_COURSE {
                    if start.saturating_add(length).saturating_add(1) > grid.height {
                        break;
                    }
                    if !h3_diagonal_frontage_course_is_clear(
                        grid,
                        &route_distances,
                        RoadBorderAxis::Vertical,
                        road_x,
                        fence_x,
                        start,
                        length,
                        MAX_ROUTE_DISTANCE,
                    ) {
                        continue;
                    }
                    let Some(anchor) = stable_grid.cell(fence_x, start + length / 2) else {
                        continue;
                    };
                    let mut proposal = RoadBorderProposal {
                        axis: RoadBorderAxis::Vertical,
                        road_fixed: road_x,
                        fence_fixed: fence_x,
                        start,
                        length,
                        side,
                        segment_start: start,
                        segment_end: start + length,
                        building_support: 0,
                        nearest_building: u16::MAX,
                        mapped_support: (start..start + length)
                            .filter(|&y| {
                                (y.saturating_sub(1)..=(y + 1).min(grid.height - 1))
                                    .any(|mapped_y| is_mapped_road(grid.cell(road_x, mapped_y)))
                            })
                            .count(),
                        frontage_route: true,
                        broad_verge: false,
                        diagonal_frontage: true,
                        stable_key: anchor
                            .offset(i64::from(side), 0)
                            .stable_hash(BORDER_SALT ^ 0x5645_5254),
                    };
                    (proposal.building_support, proposal.nearest_building) =
                        road_border_building_metrics(&proposal, buildings);
                    if proposal.mapped_support > 0
                        || proposal.nearest_building <= MAX_BUILDING_DISTANCE
                    {
                        proposals.push(proposal);
                    }
                }
            }
        }
    }
    proposals
}

fn mapped_road_route_distances(grid: &GeneratedGrid) -> Vec<u16> {
    let mut distances = vec![u16::MAX; grid.cells.len()];
    let mut queue = std::collections::VecDeque::new();
    for (index, cell) in grid.cells.iter().copied().enumerate() {
        if is_mapped_road(Some(cell)) {
            distances[index] = 0;
            queue.push_back(index);
        }
    }
    while let Some(index) = queue.pop_front() {
        let x = (index % usize::from(grid.width)) as u16;
        let y = (index / usize::from(grid.width)) as u16;
        let next_distance = distances[index].saturating_add(1);
        for (neighbor_x, neighbor_y) in [
            (x.checked_sub(1), Some(y)),
            (x.checked_add(1).filter(|&next| next < grid.width), Some(y)),
            (Some(x), y.checked_sub(1)),
            (Some(x), y.checked_add(1).filter(|&next| next < grid.height)),
        ] {
            let (Some(neighbor_x), Some(neighbor_y)) = (neighbor_x, neighbor_y) else {
                continue;
            };
            let neighbor =
                usize::from(neighbor_y) * usize::from(grid.width) + usize::from(neighbor_x);
            if distances[neighbor] == u16::MAX && is_route(grid.cell(neighbor_x, neighbor_y)) {
                distances[neighbor] = next_distance;
                queue.push_back(neighbor);
            }
        }
    }
    distances
}

fn h3_frontage_site_is_clear(
    grid: &GeneratedGrid,
    route_distances: &[u16],
    route_x: u16,
    route_y: u16,
    fence_fixed: u16,
    axis: RoadBorderAxis,
    max_route_distance: u16,
) -> bool {
    let route_index = usize::from(route_y) * usize::from(grid.width) + usize::from(route_x);
    if !is_route(grid.cell(route_x, route_y)) || route_distances[route_index] > max_route_distance {
        return false;
    }
    match axis {
        RoadBorderAxis::Horizontal => {
            horizontal_frontage_ground_is_clear(grid, route_x, route_y, fence_fixed)
        }
        RoadBorderAxis::Vertical => {
            vertical_frontage_ground_is_clear(grid, route_x, route_y, fence_fixed)
        }
    }
}

fn h3_broad_frontage_site_is_clear(
    grid: &GeneratedGrid,
    route_distances: &[u16],
    route_x: u16,
    route_y: u16,
    fence_fixed: u16,
    axis: RoadBorderAxis,
    max_route_distance: u16,
) -> bool {
    let route_index = usize::from(route_y) * usize::from(grid.width) + usize::from(route_x);
    if !is_route(grid.cell(route_x, route_y)) || route_distances[route_index] > max_route_distance {
        return false;
    }
    match axis {
        RoadBorderAxis::Horizontal => {
            let direction = i32::from(fence_fixed) - i32::from(route_y);
            direction.abs() == 3
                && (1..=2).all(|step| {
                    let y = i32::from(route_y) + direction.signum() * step;
                    walkable_verge(grid.cell(route_x, y as u16))
                })
                && horizontal_frontage_fence_site_is_clear(grid, route_x, route_y, fence_fixed)
        }
        RoadBorderAxis::Vertical => {
            let direction = i32::from(fence_fixed) - i32::from(route_x);
            direction.abs() == 3
                && (1..=2).all(|step| {
                    let x = i32::from(route_x) + direction.signum() * step;
                    walkable_verge(grid.cell(x as u16, route_y))
                })
                && vertical_frontage_fence_site_is_clear(grid, route_x, route_y, fence_fixed)
        }
    }
}

fn h3_diagonal_frontage_course_is_clear(
    grid: &GeneratedGrid,
    route_distances: &[u16],
    axis: RoadBorderAxis,
    road_fixed: u16,
    fence_fixed: u16,
    start: u16,
    length: u16,
    max_route_distance: u16,
) -> bool {
    if !(3..=6).contains(&length) || road_fixed.abs_diff(fence_fixed) != 2 {
        return false;
    }
    let mut mappings = Vec::with_capacity(usize::from(length));
    let mut uses_diagonal_shift = false;
    let mut uses_perpendicular_step = false;
    for offset in 0..length {
        let station = start + offset;
        let (fence_x, fence_y) = match axis {
            RoadBorderAxis::Horizontal => (station, fence_fixed),
            RoadBorderAxis::Vertical => (fence_fixed, station),
        };
        let fence_ground_is_clear = match axis {
            RoadBorderAxis::Horizontal => {
                horizontal_frontage_fence_ground_is_clear(grid, fence_x, fence_y)
            }
            RoadBorderAxis::Vertical => {
                vertical_frontage_fence_ground_is_clear(grid, fence_x, fence_y)
            }
        };
        if !fence_ground_is_clear {
            return false;
        }

        let mut candidates = Vec::new();
        for along_offset in -1_i32..=1 {
            let mapped_along = i32::from(station) + along_offset;
            let (route_x, route_y) = match axis {
                RoadBorderAxis::Horizontal => (mapped_along, i32::from(road_fixed)),
                RoadBorderAxis::Vertical => (i32::from(road_fixed), mapped_along),
            };
            if route_x < 0
                || route_y < 0
                || route_x >= i32::from(grid.width)
                || route_y >= i32::from(grid.height)
            {
                continue;
            }
            let route_x = route_x as u16;
            let route_y = route_y as u16;
            let route_index = usize::from(route_y) * usize::from(grid.width) + usize::from(route_x);
            if is_route(grid.cell(route_x, route_y))
                && route_distances[route_index] <= max_route_distance
            {
                candidates.push((
                    along_offset.unsigned_abs(),
                    mapped_along as u16,
                    route_x,
                    route_y,
                ));
            }
        }
        let Some(nearest_distance) = candidates.iter().map(|candidate| candidate.0).min() else {
            return false;
        };
        let mut nearest = candidates
            .into_iter()
            .filter(|candidate| candidate.0 == nearest_distance);
        let Some((_, mapped_along, route_x, route_y)) = nearest.next() else {
            return false;
        };
        if nearest.next().is_some() || route_neighbor_count(grid, route_x, route_y) > 2 {
            return false;
        }
        uses_diagonal_shift |= mapped_along != station;
        uses_perpendicular_step |= match axis {
            RoadBorderAxis::Horizontal => {
                route_y
                    .checked_sub(1)
                    .is_some_and(|y| is_route(grid.cell(route_x, y)))
                    || route_y
                        .checked_add(1)
                        .filter(|&y| y < grid.height)
                        .is_some_and(|y| is_route(grid.cell(route_x, y)))
            }
            RoadBorderAxis::Vertical => {
                route_x
                    .checked_sub(1)
                    .is_some_and(|x| is_route(grid.cell(x, route_y)))
                    || route_x
                        .checked_add(1)
                        .filter(|&x| x < grid.width)
                        .is_some_and(|x| is_route(grid.cell(x, route_y)))
            }
        };

        let verge_fixed = (road_fixed + fence_fixed) / 2;
        let verge_is_clear = match axis {
            RoadBorderAxis::Horizontal => (fence_x.min(route_x)..=fence_x.max(route_x))
                .all(|x| walkable_verge(grid.cell(x, verge_fixed))),
            RoadBorderAxis::Vertical => (fence_y.min(route_y)..=fence_y.max(route_y))
                .all(|y| walkable_verge(grid.cell(verge_fixed, y))),
        };
        if !verge_is_clear {
            return false;
        }
        mappings.push(mapped_along);
    }

    uses_diagonal_shift
        && uses_perpendicular_step
        && mappings
            .windows(2)
            .all(|pair| pair[1] >= pair[0] && pair[1].saturating_sub(pair[0]) <= 1)
        && mappings
            .last()
            .zip(mappings.first())
            .is_some_and(|(last, first)| last.saturating_sub(*first) >= length.saturating_sub(2))
}

fn route_neighbor_count(grid: &GeneratedGrid, x: u16, y: u16) -> usize {
    [
        (x.checked_sub(1), Some(y)),
        (x.checked_add(1).filter(|&next| next < grid.width), Some(y)),
        (Some(x), y.checked_sub(1)),
        (Some(x), y.checked_add(1).filter(|&next| next < grid.height)),
    ]
    .into_iter()
    .filter(|&(x, y)| x.zip(y).is_some_and(|(x, y)| is_route(grid.cell(x, y))))
    .count()
}

fn horizontal_frontage_ground_is_clear(
    grid: &GeneratedGrid,
    x: u16,
    route_y: u16,
    fence_y: u16,
) -> bool {
    let verge_y = (route_y + fence_y) / 2;
    walkable_verge(grid.cell(x, verge_y))
        && horizontal_frontage_fence_site_is_clear(grid, x, route_y, fence_y)
}

fn horizontal_frontage_fence_site_is_clear(
    grid: &GeneratedGrid,
    x: u16,
    route_y: u16,
    fence_y: u16,
) -> bool {
    horizontal_frontage_fence_ground_is_clear(grid, x, fence_y)
        // A real perpendicular crossing occupies the exact station beside
        // this route cell. The previous five-cell window confused nearby
        // bends and parallel three-wide road rasterization with intersections
        // and rejected every otherwise valid center-cell frontage course.
        && !(route_y > 0
            && route_y + 1 < grid.height
            && (is_route(grid.cell(x, route_y - 1))
                || is_route(grid.cell(x, route_y + 1))))
}

fn horizontal_frontage_fence_ground_is_clear(grid: &GeneratedGrid, x: u16, fence_y: u16) -> bool {
    fence_ground(grid.cell(x, fence_y))
        && !near_building_front(grid, x, fence_y)
        && !near_fence(grid, x, fence_y, 1)
        && !near_roadside_obstacle(grid, x, fence_y, 1)
        && !crate::grid::near_relief(grid, x, fence_y, 1)
        && h3_fence_footprint_fits(grid, x, fence_y)
}

fn vertical_frontage_ground_is_clear(
    grid: &GeneratedGrid,
    route_x: u16,
    y: u16,
    fence_x: u16,
) -> bool {
    let verge_x = (route_x + fence_x) / 2;
    walkable_verge(grid.cell(verge_x, y))
        && vertical_frontage_fence_site_is_clear(grid, route_x, y, fence_x)
}

fn vertical_frontage_fence_site_is_clear(
    grid: &GeneratedGrid,
    route_x: u16,
    y: u16,
    fence_x: u16,
) -> bool {
    vertical_frontage_fence_ground_is_clear(grid, fence_x, y)
        && !(route_x > 0
            && route_x + 1 < grid.width
            && (is_route(grid.cell(route_x - 1, y)) || is_route(grid.cell(route_x + 1, y))))
}

fn vertical_frontage_fence_ground_is_clear(grid: &GeneratedGrid, fence_x: u16, y: u16) -> bool {
    fence_ground(grid.cell(fence_x, y))
        && !near_building_front(grid, fence_x, y)
        && !near_fence(grid, fence_x, y, 1)
        && !near_roadside_obstacle(grid, fence_x, y, 1)
        && !crate::grid::near_relief(grid, fence_x, y, 1)
        && h3_fence_footprint_fits(grid, fence_x, y)
}

fn h3_fence_footprint_fits(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    grid.source.h3.as_ref().is_none_or(|plan| {
        plan.raster_footprint_fits(i32::from(x), i32::from(y), 1, 1, 3, grid.width, grid.height)
            .expect("H3 plan was validated before generation")
    })
}

fn building_cells(grid: &GeneratedGrid) -> Vec<(u16, u16)> {
    grid.cells
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(index, cell)| {
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
            )
            .then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect()
}

fn road_border_building_metrics(
    proposal: &RoadBorderProposal,
    buildings: &[(u16, u16)],
) -> (usize, u16) {
    let mut support = 0;
    let mut nearest = u16::MAX;
    for offset in 0..proposal.length {
        let (x, y) = road_border_coordinates(proposal, offset);
        let mut supported = false;
        for (building_x, building_y) in buildings.iter().copied() {
            nearest = nearest.min(x.abs_diff(building_x) + y.abs_diff(building_y));
            supported |= match proposal.axis {
                RoadBorderAxis::Horizontal => {
                    let outward = (i32::from(building_y) - i32::from(y)) * proposal.side;
                    (1..=8).contains(&outward) && x.abs_diff(building_x) <= 2
                }
                RoadBorderAxis::Vertical => {
                    let outward = (i32::from(building_x) - i32::from(x)) * proposal.side;
                    (1..=8).contains(&outward) && y.abs_diff(building_y) <= 2
                }
            };
        }
        support += usize::from(supported);
    }
    (support, nearest)
}

fn road_border_coordinates(proposal: &RoadBorderProposal, offset: u16) -> (u16, u16) {
    match proposal.axis {
        RoadBorderAxis::Horizontal => (proposal.start + offset, proposal.fence_fixed),
        RoadBorderAxis::Vertical => (proposal.fence_fixed, proposal.start + offset),
    }
}

fn road_border_cell(proposal: &RoadBorderProposal) -> MapCell {
    match (proposal.axis, proposal.side < 0) {
        (RoadBorderAxis::Horizontal, true) => MapCell::FenceSouth,
        (RoadBorderAxis::Horizontal, false) => MapCell::FenceNorth,
        (RoadBorderAxis::Vertical, true) => MapCell::FenceEast,
        (RoadBorderAxis::Vertical, false) => MapCell::FenceWest,
    }
}

fn road_border_proposal_is_clear(
    grid: &GeneratedGrid,
    proposal: &RoadBorderProposal,
    route_distances: Option<&[u16]>,
) -> bool {
    if proposal.frontage_route {
        let Some(route_distances) = route_distances else {
            return false;
        };
        if proposal.diagonal_frontage {
            return h3_diagonal_frontage_course_is_clear(
                grid,
                route_distances,
                proposal.axis,
                proposal.road_fixed,
                proposal.fence_fixed,
                proposal.start,
                proposal.length,
                H3_PRINCIPAL_ROUTE_MAX_DISTANCE,
            );
        }
        return (0..proposal.length).all(|offset| {
            let (route_x, route_y, fence_fixed) = match proposal.axis {
                RoadBorderAxis::Horizontal => (
                    proposal.start + offset,
                    proposal.road_fixed,
                    proposal.fence_fixed,
                ),
                RoadBorderAxis::Vertical => (
                    proposal.road_fixed,
                    proposal.start + offset,
                    proposal.fence_fixed,
                ),
            };
            if proposal.broad_verge {
                h3_broad_frontage_site_is_clear(
                    grid,
                    route_distances,
                    route_x,
                    route_y,
                    fence_fixed,
                    proposal.axis,
                    H3_PRINCIPAL_ROUTE_MAX_DISTANCE,
                )
            } else {
                h3_frontage_site_is_clear(
                    grid,
                    route_distances,
                    route_x,
                    route_y,
                    fence_fixed,
                    proposal.axis,
                    H3_PRINCIPAL_ROUTE_MAX_DISTANCE,
                )
            }
        });
    }
    (0..proposal.length).all(|offset| match proposal.axis {
        RoadBorderAxis::Horizontal => road_border_site_is_clear(
            grid,
            proposal.start + offset,
            proposal.road_fixed,
            proposal.fence_fixed,
        ),
        RoadBorderAxis::Vertical => vertical_road_border_site_is_clear(
            grid,
            proposal.road_fixed,
            proposal.start + offset,
            proposal.fence_fixed,
        ),
    })
}

fn road_border_proposals_crowd(first: &RoadBorderProposal, second: &RoadBorderProposal) -> bool {
    let footprint = |proposal: &RoadBorderProposal| match proposal.axis {
        RoadBorderAxis::Horizontal => (
            i32::from(proposal.start),
            i32::from(proposal.fence_fixed),
            i32::from(proposal.start + proposal.length - 1),
            i32::from(proposal.fence_fixed),
        ),
        RoadBorderAxis::Vertical => (
            i32::from(proposal.fence_fixed),
            i32::from(proposal.start),
            i32::from(proposal.fence_fixed),
            i32::from(proposal.start + proposal.length - 1),
        ),
    };
    // Reserve two blocks around a course so opposite road edges and nearby
    // parallel streets cannot produce a picket-fence comb.
    rectangles_overlap(expand(footprint(first), 2), expand(footprint(second), 2))
}

fn road_border_site_is_clear(grid: &GeneratedGrid, x: u16, road_y: u16, fence_y: u16) -> bool {
    let verge_y = (road_y + fence_y) / 2;
    if !is_mapped_road(grid.cell(x, road_y))
        || !walkable_verge(grid.cell(x, verge_y))
        || near_building_front(grid, x, fence_y)
        || near_fence(grid, x, fence_y, 1)
        || near_roadside_obstacle(grid, x, fence_y, 1)
        || crate::grid::near_relief(grid, x, fence_y, 1)
        || !h3_fence_footprint_fits(grid, x, fence_y)
        || !matches!(
            grid.cell(x, fence_y),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
        )
    {
        return false;
    }
    // The route cell at a real crossing is the gate in an ordinary mapped
    // street course. Reserving two additional cells on either side erased
    // every usable lot edge in dense city blocks whose crossings are only six
    // or seven cells apart. H3 faces retain that wider seam-safe opening
    // because their rasterized transport can enter at an angle.
    let crossing_radius = if grid.source.h3.is_some() { 2 } else { 0 };
    !(x.saturating_sub(crossing_radius)..=(x + crossing_radius).min(grid.width - 1)).any(
        |check_x| {
            road_y > 0
                && road_y + 1 < grid.height
                && (is_route(grid.cell(check_x, road_y - 1))
                    || is_route(grid.cell(check_x, road_y + 1)))
        },
    )
}

fn vertical_road_border_site_is_clear(
    grid: &GeneratedGrid,
    road_x: u16,
    y: u16,
    fence_x: u16,
) -> bool {
    let verge_x = (road_x + fence_x) / 2;
    if !is_mapped_road(grid.cell(road_x, y))
        || !walkable_verge(grid.cell(verge_x, y))
        || near_building_front(grid, fence_x, y)
        || near_fence(grid, fence_x, y, 1)
        || near_roadside_obstacle(grid, fence_x, y, 1)
        || crate::grid::near_relief(grid, fence_x, y, 1)
        || !h3_fence_footprint_fits(grid, fence_x, y)
        || !matches!(
            grid.cell(fence_x, y),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
        )
    {
        return false;
    }
    let crossing_radius = if grid.source.h3.is_some() { 2 } else { 0 };
    !(y.saturating_sub(crossing_radius)..=(y + crossing_radius).min(grid.height - 1)).any(
        |check_y| {
            road_x > 0
                && road_x + 1 < grid.width
                && (is_route(grid.cell(road_x - 1, check_y))
                    || is_route(grid.cell(road_x + 1, check_y)))
        },
    )
}

// Road-edge courses and rest bays are planned together above; no independent
// gate author may mutate the shared reservation state.

fn roadside_proposals(grid: &GeneratedGrid, stable_grid: StableGrid) -> Vec<BayProposal> {
    let mut proposals = Vec::new();
    for y in 1..grid.height.saturating_sub(1) {
        let mut x = 0;
        while x < grid.width {
            if !is_mapped_road(grid.cell(x, y)) {
                x += 1;
                continue;
            }
            let segment_start = x;
            while x < grid.width && is_mapped_road(grid.cell(x, y)) {
                x += 1;
            }
            let segment_end = x;
            if segment_end.saturating_sub(segment_start) < BAY_WIDTH + 2 {
                continue;
            }

            for start_x in segment_start + 1..=segment_end - BAY_WIDTH - 1 {
                // Sample one globally repeatable anchor per four road blocks;
                // this keeps proposal volume low without imposing a map-local
                // row or column lattice on the chosen scenes.
                let Some(anchor) = stable_grid.cell(start_x + BAY_WIDTH / 2, y) else {
                    continue;
                };
                if anchor.x_mod(4) != 0 {
                    continue;
                }
                for side in [-1_i32, 1_i32] {
                    if !roadside_site_is_clear(grid, start_x, y, side) {
                        continue;
                    }
                    let top = i32::from(y) + side * 4;
                    let bottom = i32::from(y) + side;
                    proposals.push(BayProposal {
                        road_y: y,
                        start_x,
                        side,
                        stable_key: anchor.offset(0, i64::from(side)).stable_hash(BAY_SALT),
                        segment_key: (y, segment_start, segment_end),
                        footprint: (
                            i32::from(start_x) - 1,
                            top.min(bottom),
                            i32::from(start_x + BAY_WIDTH),
                            top.max(bottom),
                        ),
                    });
                }
            }
        }
    }
    proposals
}

fn roadside_site_is_clear(grid: &GeneratedGrid, start_x: u16, road_y: u16, side: i32) -> bool {
    let road_y = i32::from(road_y);
    let (fence_y, furniture_y, verge_y) = roadside_rows(road_y, side);
    if fence_y <= 1 || fence_y + 1 >= i32::from(grid.height) || furniture_y <= 0 || verge_y <= 0 {
        return false;
    }

    for offset in 0..BAY_WIDTH {
        let x = start_x + offset;
        if !is_mapped_road(grid.cell(x, road_y as u16))
            || !walkable_verge(grid.cell(x, verge_y as u16))
            || near_building_front(grid, x, verge_y as u16)
            || near_fence(grid, x, fence_y as u16, 1)
            || [fence_y, furniture_y, verge_y].into_iter().any(|site_y| {
                near_roadside_obstacle(grid, x, site_y as u16, 1)
                    || crate::grid::near_relief(grid, x, site_y as u16, 1)
            })
        {
            return false;
        }
        if !matches!(
            grid.cell(x, furniture_y as u16),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
        ) || !matches!(
            grid.cell(x, fence_y as u16),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
        ) {
            return false;
        }
    }

    // Keep intersections and perpendicular driveways visually open.
    !(start_x..start_x + BAY_WIDTH).any(|x| {
        is_route(grid.cell(x, (road_y - 1) as u16)) || is_route(grid.cell(x, (road_y + 1) as u16))
    })
}

fn stamp_roadside_bay(
    grid: &mut GeneratedGrid,
    proposal: &BayProposal,
    summary: &mut RoadsideSummary,
) {
    let road_y = i32::from(proposal.road_y);
    let (fence_y, furniture_y, verge_y) = roadside_rows(road_y, proposal.side);

    for offset in 0..BAY_WIDTH {
        let x = proposal.start_x + offset;
        let fence = if proposal.side < 0 {
            MapCell::FenceSouth
        } else {
            MapCell::FenceNorth
        };
        replace_non_route(grid, i32::from(x), fence_y, fence);
        replace_non_route(grid, i32::from(x), verge_y, MapCell::Lawn);
        if matches!(
            grid.cell(x, furniture_y as u16),
            Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
        ) {
            replace_non_route(grid, i32::from(x), furniture_y, MapCell::Lawn);
        }
    }

    // The asymmetry is deliberate: repeated rest stops should share a grammar
    // without looking like copy-pasted seven-cell stamps.
    let mirrored = proposal.stable_key & 1 != 0;
    let (bench_offset, trash_offset, flower_offset, tree_offset) = if mirrored {
        (4_u16, 2_u16, 1_u16, 6_u16)
    } else {
        (2_u16, 4_u16, 5_u16, 0_u16)
    };
    replace_non_route(
        grid,
        i32::from(proposal.start_x + bench_offset),
        furniture_y,
        MapCell::Bench,
    );
    replace_non_route(
        grid,
        i32::from(proposal.start_x + trash_offset),
        furniture_y,
        MapCell::TrashCan,
    );
    replace_non_route(
        grid,
        i32::from(proposal.start_x + flower_offset),
        furniture_y,
        MapCell::Flowers,
    );
    replace_non_route(
        grid,
        i32::from(proposal.start_x + tree_offset),
        furniture_y,
        MapCell::ParkTree,
    );
    summary.bays += 1;
    summary.benches += 1;
    summary.trash_cans += 1;
    summary.fence_cells += usize::from(BAY_WIDTH);
}

fn roadside_rows(road_y: i32, side: i32) -> (i32, i32, i32) {
    if side < 0 {
        // Benches face south. North-side furniture therefore sits between its
        // backing fence and the route-facing lawn verge.
        (road_y - 3, road_y - 2, road_y - 1)
    } else {
        // On the south side the same canonical orientation needs its standing
        // lawn below the bench; the fence course moves behind it to the north.
        (road_y + 1, road_y + 2, road_y + 3)
    }
}

fn bay_proposals_crowd(first: &BayProposal, second: &BayProposal) -> bool {
    rectangles_overlap(expand(first.footprint, 4), expand(second.footprint, 4))
}

fn expand(rectangle: (i32, i32, i32, i32), amount: i32) -> (i32, i32, i32, i32) {
    (
        rectangle.0 - amount,
        rectangle.1 - amount,
        rectangle.2 + amount,
        rectangle.3 + amount,
    )
}

fn rectangles_overlap(first: (i32, i32, i32, i32), second: (i32, i32, i32, i32)) -> bool {
    first.0 <= second.2 && first.2 >= second.0 && first.1 <= second.3 && first.3 >= second.1
}

fn replace_non_route(grid: &mut GeneratedGrid, x: i32, y: i32, cell: MapCell) {
    if x < 0 || y < 0 || x >= i32::from(grid.width) || y >= i32::from(grid.height) {
        return;
    }
    let index = y as usize * usize::from(grid.width) + x as usize;
    if !is_route(Some(grid.cells[index])) {
        grid.cells[index] = cell;
    }
}

fn is_route(cell: Option<MapCell>) -> bool {
    matches!(
        cell,
        Some(MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad)
    )
}

fn is_mapped_road(cell: Option<MapCell>) -> bool {
    matches!(
        cell,
        Some(MapCell::Street | MapCell::Road | MapCell::MajorRoad)
    )
}

fn walkable_verge(cell: Option<MapCell>) -> bool {
    matches!(
        cell,
        Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
    )
}

fn fence_ground(cell: Option<MapCell>) -> bool {
    matches!(
        cell,
        Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing | MapCell::Flowers)
    )
}

fn near_building_front(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    for check_y in y.saturating_sub(3)..=(y + 1).min(grid.height - 1) {
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
                )
            ) {
                return true;
            }
        }
    }
    false
}

fn near_fence(grid: &GeneratedGrid, x: u16, y: u16, radius: u16) -> bool {
    for check_y in y.saturating_sub(radius)..=(y + radius).min(grid.height - 1) {
        for check_x in x.saturating_sub(radius)..=(x + radius).min(grid.width - 1) {
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
    }
    false
}

fn near_roadside_obstacle(grid: &GeneratedGrid, x: u16, y: u16, radius: u16) -> bool {
    for check_y in y.saturating_sub(radius)..=(y + radius).min(grid.height - 1) {
        for check_x in x.saturating_sub(radius)..=(x + radius).min(grid.width - 1) {
            if matches!(
                grid.cell(check_x, check_y),
                Some(
                    MapCell::H3Void
                        | MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Boulder
                )
            ) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate, MapSource, plan_h3_cell};

    fn fixture() -> GeneratedGrid {
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: Coordinate {
                    lat: 44.947_519_6,
                    lon: -93.325_347_7,
                },
                bounds: BoundingBox {
                    south: 44.940_519_6,
                    west: -93.335_347_7,
                    north: 44.954_519_6,
                    east: -93.315_347_7,
                },
                attribution: "roadside fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        for y in [12_u16, 31, 50] {
            for x in 2_u16..62 {
                grid.cells[usize::from(y) * 64 + usize::from(x)] = MapCell::Road;
            }
        }
        for y in 2_u16..62 {
            grid.cells[usize::from(y) * 64 + 32] = MapCell::Road;
        }
        grid
    }

    #[test]
    fn city_scale_square_allocates_fence_courses_across_the_full_map() {
        let mut grid = fixture();
        grid.width = 128;
        grid.height = 128;
        grid.cells = vec![MapCell::Grass; 128 * 128];

        assert_eq!(road_border_course_limit(&grid, 0), 19);
        assert_eq!(road_border_course_limit(&grid, 10), 19);
    }

    fn h3_frontage_fixture() -> GeneratedGrid {
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
                attribution: "H3 frontage fixture".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        grid.cells[32 * 64 + 20] = MapCell::MajorRoad;
        for x in 21_usize..=28 {
            grid.cells[32 * 64 + x] = MapCell::Trail;
        }
        grid.cells[25 * 64 + 24] = MapCell::Building;

        // This equally long, equally urban-looking trail is deliberately
        // disconnected from mapped transport and must remain ineligible.
        for x in 18_usize..=28 {
            grid.cells[45 * 64 + x] = MapCell::Trail;
        }
        grid.cells[52 * 64 + 24] = MapCell::Building;
        grid
    }

    fn h3_diagonal_frontage_fixture() -> GeneratedGrid {
        let mut grid = h3_frontage_fixture();
        grid.cells.fill(MapCell::Grass);
        grid.cells[32 * 64 + 20] = MapCell::MajorRoad;
        grid.cells[32 * 64 + 21] = MapCell::Trail;
        grid.cells[33 * 64 + 21] = MapCell::Trail;
        // Close the opposite frontage without putting an obstacle inside the
        // clear north-side verge exercised by the regression.
        for x in 18_usize..=23 {
            grid.cells[34 * 64 + x] = MapCell::Water;
        }
        grid
    }

    fn h3_principal_frontage_fixture() -> GeneratedGrid {
        let mut grid = h3_frontage_fixture();
        grid.cells.fill(MapCell::Grass);
        for y in 16_usize..=48 {
            grid.cells[y * 64 + 32] = MapCell::Trail;
        }
        grid.cells[32 * 64 + 32] = MapCell::MajorRoad;
        for (route_y, building_y) in [(20_usize, 13_usize), (30, 23), (40, 33)] {
            for x in 18_usize..=32 {
                grid.cells[route_y * 64 + x] = MapCell::Trail;
            }
            for y in building_y..=building_y + 1 {
                for x in 21_usize..=22 {
                    grid.cells[y * 64 + x] = MapCell::Building;
                }
            }
        }
        grid
    }

    fn h3_short_route_fallback_fixture() -> GeneratedGrid {
        let mut grid = h3_frontage_fixture();
        grid.cells.fill(MapCell::Grass);
        for route_y in [18_usize, 28, 38, 48] {
            for x in 28_usize..=30 {
                grid.cells[route_y * 64 + x] = MapCell::MajorRoad;
            }
            // Each lot sits beyond the north-side fence halo but within the
            // canonical eight-cell urban-support range.
            for y in route_y - 8..=route_y - 7 {
                for x in 29_usize..=30 {
                    grid.cells[y * 64 + x] = MapCell::Building;
                }
            }
        }
        grid
    }

    fn straight_fence(cell: MapCell) -> bool {
        matches!(
            cell,
            MapCell::FenceNorth | MapCell::FenceSouth | MapCell::FenceEast | MapCell::FenceWest
        )
    }

    fn straight_fence_components(grid: &GeneratedGrid) -> Vec<Vec<(u16, u16)>> {
        let mut remaining = grid
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, cell)| {
                straight_fence(cell).then_some((
                    (index % usize::from(grid.width)) as u16,
                    (index / usize::from(grid.width)) as u16,
                ))
            })
            .collect::<std::collections::BTreeSet<_>>();
        let mut components = Vec::new();
        while let Some(start) = remaining.pop_first() {
            let mut queue = std::collections::VecDeque::from([start]);
            let mut component = Vec::new();
            while let Some((x, y)) = queue.pop_front() {
                component.push((x, y));
                for neighbor in [
                    (x.checked_sub(1), Some(y)),
                    (x.checked_add(1), Some(y)),
                    (Some(x), y.checked_sub(1)),
                    (Some(x), y.checked_add(1)),
                ] {
                    let (Some(neighbor_x), Some(neighbor_y)) = neighbor else {
                        continue;
                    };
                    if remaining.remove(&(neighbor_x, neighbor_y)) {
                        queue.push_back((neighbor_x, neighbor_y));
                    }
                }
            }
            components.push(component);
        }
        components
    }

    fn stamp_public_field_perimeter(grid: &mut GeneratedGrid, west: u16, north: u16) {
        for x in west + 1..west + 8 {
            grid.cells[usize::from(north) * usize::from(grid.width) + usize::from(x)] =
                MapCell::FenceNorth;
            if x != west + 4 {
                grid.cells[usize::from(north + 6) * usize::from(grid.width) + usize::from(x)] =
                    MapCell::FenceSouth;
            }
        }
        for y in north + 1..north + 6 {
            grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(west)] =
                MapCell::FenceWest;
            grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(west + 8)] =
                MapCell::FenceEast;
        }
        for (x, y, cell) in [
            (west, north, MapCell::FenceNorthWest),
            (west + 8, north, MapCell::FenceNorthEast),
            (west, north + 6, MapCell::FenceSouthWest),
            (west + 8, north + 6, MapCell::FenceSouthEast),
        ] {
            grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
        }
    }

    #[test]
    fn quality_gate_excludes_cornered_public_field_and_proves_roadside_course() {
        let mut grid = fixture();
        stamp_public_field_perimeter(&mut grid, 3, 40);

        // Five straight fence cells follow the north side of the road at y=12.
        // One canonical bench occupies the intervening verge, leaving four of
        // five stations fully clear and therefore above the 75% quality gate.
        for x in 20_u16..25 {
            grid.cells[10 * 64 + usize::from(x)] = MapCell::FenceSouth;
        }
        grid.cells[11 * 64 + 22] = MapCell::Bench;
        for y in 5_usize..=6 {
            for x in 21_usize..=22 {
                grid.cells[y * 64 + x] = MapCell::Building;
            }
        }

        let quality = roadside_fence_quality(&grid);
        assert_eq!(quality.public_perimeter_cells, 27);
        assert_eq!(quality.roadside_courses, 1);
        assert_eq!(quality.roadside_fence_cells, 5);
        assert_eq!(quality.route_supported_cells, 4);
        assert_eq!(quality.urban_supported_courses, 1);
        assert_eq!(quality.malformed_courses, 0);
        assert_eq!(quality.landing_conflicts, 0);
        assert_eq!(quality.terrain_conflicts, 0);
        assert_eq!(quality.roadside_amenities, 1);
    }

    #[test]
    fn h3_fence_sites_reserve_the_full_three_cell_face_band() {
        let grid = h3_frontage_fixture();
        let plan = grid.source.h3.as_ref().expect("H3 plan");
        let fringe = (0..grid.height)
            .flat_map(|y| (0..grid.width).map(move |x| (x, y)))
            .find(|&(x, y)| {
                plan.raster_footprint_fits(
                    i32::from(x),
                    i32::from(y),
                    1,
                    1,
                    1,
                    grid.width,
                    grid.height,
                )
                .expect("one-cell clearance")
                    && !plan
                        .raster_footprint_fits(
                            i32::from(x),
                            i32::from(y),
                            1,
                            1,
                            3,
                            grid.width,
                            grid.height,
                        )
                        .expect("three-cell clearance")
            })
            .expect("H3 face should have a two-cell inner fringe");
        assert!(
            !h3_fence_footprint_fits(&grid, fringe.0, fringe.1),
            "fences and their fixtures must not occupy the reciprocal seam band"
        );
        assert!(h3_fence_footprint_fits(&grid, 32, 32));
    }

    #[test]
    fn h3_principal_frontage_fills_three_urban_courses_without_adding_roads() {
        let mut grid = h3_principal_frontage_fixture();
        let original_routes = grid
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, cell)| is_route(Some(cell)).then_some((index, cell)))
            .collect::<Vec<_>>();

        let summary = author_roadside_bays(&mut grid).expect("H3 roadside plan");
        let quality = roadside_fence_quality(&grid);
        assert!(
            summary.border_courses >= 3 && summary.fence_cells >= 12,
            "three real city frontages should not collapse into one field-like strip: {summary:?}"
        );
        assert!(quality.roadside_courses >= 3);
        assert!(quality.roadside_fence_cells >= 12);
        assert_eq!(quality.malformed_courses, 0);
        assert!(
            quality.route_supported_cells * 4 >= quality.roadside_fence_cells * 3,
            "most of each fence must retain its clear parallel verge: {quality:?}"
        );
        assert!(quality.urban_supported_courses * 2 >= quality.roadside_courses);
        assert!(quality.roadside_amenities >= 1);
        for (index, route) in original_routes {
            assert_eq!(grid.cells[index], route, "fence authoring changed a route");
        }
    }

    #[test]
    fn h3_roadside_courses_reselect_instead_of_consuming_tall_grass() {
        let mut baseline = h3_principal_frontage_fixture();
        let baseline_summary =
            author_roadside_bays(&mut baseline).expect("baseline H3 roadside plan");
        assert!(baseline_summary.border_courses >= H3_MIN_ROADSIDE_COURSES);
        assert!(baseline_summary.fence_cells >= H3_MIN_ROADSIDE_FENCE_CELLS);

        let protected_indices = baseline
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, cell)| straight_fence(cell).then_some(index))
            .collect::<std::collections::BTreeSet<_>>();
        let mut protected = h3_principal_frontage_fixture();
        for &index in &protected_indices {
            protected.cells[index] = MapCell::Park;
        }

        let summary = author_roadside_bays(&mut protected).expect("reselected H3 roadside plan");
        assert!(
            summary.border_courses >= H3_MIN_ROADSIDE_COURSES
                && summary.fence_cells >= H3_MIN_ROADSIDE_FENCE_CELLS,
            "protected tall grass must make fence courses reselect another clear frontage: {summary:?}"
        );
        for index in protected_indices {
            assert_eq!(
                protected.cells[index],
                MapCell::Park,
                "roadside authoring consumed protected tall grass at ({}, {})",
                index % usize::from(protected.width),
                index / usize::from(protected.width),
            );
        }
    }

    #[test]
    fn h3_frontage_keeps_a_three_cell_course_beside_protected_tall_grass() {
        let mut grid = h3_frontage_fixture();
        for x in 23_usize..=28 {
            grid.cells[30 * 64 + x] = MapCell::Park;
        }
        let stable_grid = StableGrid::for_grid(&grid).expect("stable H3 grid");
        let proposals =
            h3_compact_frontage_border_proposals(&grid, stable_grid, &building_cells(&grid));

        assert!(
            proposals.iter().any(|proposal| {
                proposal.axis == RoadBorderAxis::Horizontal
                    && proposal.road_fixed == 32
                    && proposal.fence_fixed == 30
                    && proposal.start == 20
                    && proposal.length == 3
            }),
            "a protected wild patch should shorten the clear frontage course, not erase it"
        );
    }

    #[test]
    fn h3_broad_fallback_iterates_until_course_and_cell_quota() {
        let mut grid = h3_short_route_fallback_fixture();
        let stable_grid = StableGrid::for_grid(&grid).expect("stable H3 grid");
        let buildings = building_cells(&grid);
        assert!(
            road_border_proposals(&grid, stable_grid, &buildings).is_empty(),
            "three-cell mapped fragments must exercise the compact fallback"
        );
        assert!(
            h3_diagonal_frontage_border_proposals(&grid, stable_grid, &buildings).is_empty(),
            "a straight route is not a rasterized-diagonal proposal"
        );
        assert!(
            h3_broad_verge_border_proposals(&grid, stable_grid, &buildings).len() >= 4,
            "the fixture needs four independent broad-verge candidates"
        );
        let original_routes = grid
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, cell)| is_route(Some(cell)).then_some((index, cell)))
            .collect::<Vec<_>>();

        let summary = author_roadside_bays(&mut grid).expect("quota-filling H3 roadside plan");
        let quality = roadside_fence_quality(&grid);
        assert_eq!(summary.bays, 0);
        assert_eq!(
            summary.border_courses, 4,
            "fallback must keep committing: {summary:?}"
        );
        assert_eq!(
            summary.fence_cells, 12,
            "four compact courses meet the cell gate"
        );
        assert_eq!(quality.roadside_courses, H3_MIN_ROADSIDE_COURSES + 1);
        assert_eq!(quality.roadside_fence_cells, H3_MIN_ROADSIDE_FENCE_CELLS);
        assert_eq!(quality.malformed_courses, 0);
        assert!(
            quality.route_supported_cells * 4 >= quality.roadside_fence_cells * 3,
            "broad courses and their two-cell verges must be auditable: {quality:?}"
        );
        assert!(quality.urban_supported_courses * 2 >= quality.roadside_courses);
        assert!(quality.roadside_amenities >= 1);
        assert!(
            straight_fence_components(&grid).len() <= 4,
            "compact fallback must not form a perimeter ring"
        );
        for (index, route) in original_routes {
            assert_eq!(grid.cells[index], route, "fence authoring changed a route");
        }
    }

    #[test]
    fn creates_complete_spaced_rest_bays_without_touching_roads() {
        let mut first = fixture();
        let original_roads = first
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| (*cell == MapCell::Road).then_some(index))
            .collect::<Vec<_>>();
        let mut repeated = first.clone();
        let summary = author_roadside_bays(&mut first).expect("roadside plan");
        let repeated_summary = author_roadside_bays(&mut repeated).expect("repeat roadside plan");

        assert_eq!(summary, repeated_summary);
        assert_eq!(first.cells, repeated.cells);
        assert!(summary.bays >= 3, "expected several rest bays: {summary:?}");
        assert_eq!(summary.benches, summary.bays);
        assert_eq!(summary.trash_cans, summary.bays);
        assert!(
            summary.fence_cells >= summary.bays * usize::from(BAY_WIDTH) + 6,
            "rest bays and at least one paired end-post gate should contribute fence structure: {summary:?}"
        );
        for index in original_roads {
            assert_eq!(first.cells[index], MapCell::Road);
        }
        for (index, cell) in first.cells.iter().copied().enumerate() {
            if !matches!(cell, MapCell::Bench | MapCell::TrashCan) {
                continue;
            }
            let x = index % 64;
            let y = index / 64;
            assert!(
                [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1)
                ]
                .into_iter()
                .filter(|&(x, y)| x < 64 && y < 64)
                .any(|(x, y)| matches!(first.cells[y * 64 + x], MapCell::Lawn | MapCell::Road)),
                "amenity at ({x},{y}) has no standing cell"
            );
        }
    }

    #[test]
    fn mapped_road_courses_have_exact_density_topology_and_route_support() {
        let mut grid = fixture();
        let stable_grid = StableGrid::for_grid(&grid).expect("stable grid");
        let mut summary = RoadsideSummary::default();
        author_road_border_runs(&mut grid, stable_grid, &mut summary);

        assert_eq!(summary.border_courses, 4);
        let components = straight_fence_components(&grid);
        assert_eq!(components.len(), 4, "one component per planned course");
        assert_eq!(
            components.iter().map(Vec::len).sum::<usize>(),
            summary.fence_cells
        );
        for component in components {
            assert!((6..=10).contains(&component.len()));
            let first_cell = grid.cell(component[0].0, component[0].1).unwrap();
            assert!(
                component
                    .iter()
                    .all(|&(x, y)| grid.cell(x, y) == Some(first_cell)),
                "a course must use one canonical straight orientation"
            );
            let one_row = component.iter().all(|&(_, y)| y == component[0].1);
            let one_column = component.iter().all(|&(x, _)| x == component[0].0);
            assert_ne!(one_row, one_column, "a course must be a straight line");
            for (x, y) in component {
                let (route_x, route_y, verge_x, verge_y) = match first_cell {
                    MapCell::FenceSouth => (x, y + 2, x, y + 1),
                    MapCell::FenceNorth => (x, y - 2, x, y - 1),
                    MapCell::FenceEast => (x + 2, y, x + 1, y),
                    MapCell::FenceWest => (x - 2, y, x - 1, y),
                    _ => unreachable!("only straight canonical fence cells are collected"),
                };
                assert!(is_mapped_road(grid.cell(route_x, route_y)));
                assert!(walkable_verge(grid.cell(verge_x, verge_y)));
            }
        }
    }

    #[test]
    fn h3_frontage_fallback_is_connected_urban_straight_and_preserves_perimeters() {
        let mut grid = h3_frontage_fixture();
        let perimeter = [
            (40_u16, 38_u16, MapCell::FenceNorthWest),
            (41, 38, MapCell::FenceNorth),
            (42, 38, MapCell::FenceNorthEast),
            (40, 39, MapCell::FenceSouthWest),
            (41, 39, MapCell::FenceSouth),
            (42, 39, MapCell::FenceSouthEast),
        ];
        for &(x, y, cell) in &perimeter {
            grid.cells[usize::from(y) * 64 + usize::from(x)] = cell;
        }
        let original_fences = grid
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, cell)| {
                matches!(
                    cell,
                    MapCell::FenceNorthWest
                        | MapCell::FenceNorth
                        | MapCell::FenceNorthEast
                        | MapCell::FenceWest
                        | MapCell::FenceEast
                        | MapCell::FenceSouthWest
                        | MapCell::FenceSouth
                        | MapCell::FenceSouthEast
                )
                .then_some(index)
            })
            .collect::<std::collections::BTreeSet<_>>();

        let summary = author_roadside_bays(&mut grid).expect("H3 roadside plan");
        assert_eq!(summary.bays, 0);
        assert!(
            summary.border_courses > 1,
            "fallback must keep searching after the first ordinary course: {summary:?}"
        );
        assert!(summary.border_courses <= 4);
        assert!(summary.fence_cells >= 7);
        for &(x, y, cell) in &perimeter {
            assert_eq!(grid.cell(x, y), Some(cell));
        }

        let added = grid
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter(|(index, cell)| straight_fence(*cell) && !original_fences.contains(index))
            .map(|(index, cell)| ((index % 64) as u16, (index / 64) as u16, cell))
            .collect::<Vec<_>>();
        assert_eq!(added.len(), summary.fence_cells);
        let route_distances = mapped_road_route_distances(&grid);
        for &(x, y, orientation) in &added {
            assert!(
                roadside_support_geometry(&grid, &route_distances, x, y, orientation, true,)
                    .is_some(),
                "added frontage has no supported route/verge geometry at ({x}, {y})"
            );
        }
        let quality = roadside_fence_quality(&grid);
        assert_eq!(quality.public_perimeter_cells, perimeter.len());
        assert_eq!(quality.roadside_courses, summary.border_courses);
        assert_eq!(quality.roadside_fence_cells, summary.fence_cells);
        assert_eq!(quality.malformed_courses, 0);
        assert!(quality.route_supported_cells * 4 >= quality.roadside_fence_cells * 3);
        assert!(quality.roadside_amenities >= 1);
        assert!(
            grid.cells[45 * 64 + 18..45 * 64 + 29]
                .iter()
                .all(|&cell| cell == MapCell::Trail),
            "the disconnected synthetic trail must not author a fence"
        );
    }

    #[test]
    fn h3_diagonal_frontage_maps_monotonically_and_rejects_a_true_branch() {
        let mut grid = h3_diagonal_frontage_fixture();
        let route_distances = mapped_road_route_distances(&grid);
        assert!(h3_diagonal_frontage_course_is_clear(
            &grid,
            &route_distances,
            RoadBorderAxis::Horizontal,
            32,
            30,
            19,
            3,
            12,
        ));

        let original_routes = grid
            .cells
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, cell)| is_route(Some(cell)).then_some((index, cell)))
            .collect::<Vec<_>>();
        let summary = author_roadside_bays(&mut grid).expect("diagonal H3 roadside plan");
        assert_eq!(summary.bays, 0);
        assert_eq!(summary.border_courses, 1);
        assert_eq!(summary.fence_cells, 3);
        assert_eq!(
            (19_u16..=21).map(|x| grid.cell(x, 30)).collect::<Vec<_>>(),
            vec![Some(MapCell::FenceSouth); 3],
            "the straight course should follow route mappings [20, 20, 21]"
        );
        for (index, route) in original_routes {
            assert_eq!(
                grid.cells[index], route,
                "roadside authoring changed a route"
            );
        }

        let mut branch = h3_diagonal_frontage_fixture();
        branch.cells[32 * 64 + 22] = MapCell::Trail;
        assert_eq!(route_neighbor_count(&branch, 21, 32), 3);
        assert!(
            !h3_diagonal_frontage_course_is_clear(
                &branch,
                &mapped_road_route_distances(&branch),
                RoadBorderAxis::Horizontal,
                32,
                30,
                19,
                3,
                12,
            ),
            "a three-way route branch must never masquerade as a diagonal frontage"
        );
    }

    #[test]
    fn road_borders_require_a_clear_verge_between_fence_and_route() {
        let mut horizontal = fixture();
        horizontal.cells[11 * 64 + 10] = MapCell::Boulder;
        assert!(!road_border_site_is_clear(&horizontal, 10, 12, 10));

        let mut vertical = fixture();
        vertical.cells[10 * 64 + 31] = MapCell::Boulder;
        assert!(!vertical_road_border_site_is_clear(&vertical, 32, 10, 30));
    }

    #[test]
    fn road_border_candidates_respect_ledge_and_cliff_access_halos() {
        let mut grid = fixture();

        for x in 10_u16..=18 {
            grid.cells[32 * 64 + usize::from(x)] = match x {
                10 => MapCell::LedgeWest,
                18 => MapCell::LedgeEast,
                _ => MapCell::LedgeMiddle,
            };
            grid.cells[33 * 64 + usize::from(x)] = MapCell::Lawn;
        }
        assert!(
            !road_border_site_is_clear(&grid, 14, 31, 33),
            "a road fence must not consume the landing below a ledge run"
        );

        grid.cells[32 * 64 + 24] = MapCell::CliffStairs;
        grid.cells[33 * 64 + 24] = MapCell::Lawn;
        assert!(
            !road_border_site_is_clear(&grid, 24, 31, 33),
            "a road fence must leave the south approach to cliff stairs open"
        );

        grid.cells[24 * 64 + 33] = MapCell::CliffStairs;
        grid.cells[24 * 64 + 34] = MapCell::Lawn;
        assert!(
            !vertical_road_border_site_is_clear(&grid, 32, 24, 34),
            "vertical road fences need the same relief-access exclusion"
        );

        let mut outward_relief = fixture();
        outward_relief.cells[34 * 64 + 14] = MapCell::CliffNorth;
        assert!(
            !road_border_site_is_clear(&outward_relief, 14, 31, 33),
            "the relief halo must protect the outward side of a horizontal fence"
        );

        outward_relief.cells[24 * 64 + 35] = MapCell::CliffWest;
        assert!(
            !vertical_road_border_site_is_clear(&outward_relief, 32, 24, 34),
            "the relief halo must protect the outward side of a vertical fence"
        );

        let mut bay = fixture();
        assert!(roadside_site_is_clear(&bay, 10, 12, -1));
        bay.cells[8 * 64 + 13] = MapCell::LedgeMiddle;
        assert!(
            !roadside_site_is_clear(&bay, 10, 12, -1),
            "rest-bay fence, furniture, and verge rows must all respect relief halos"
        );
    }
}
