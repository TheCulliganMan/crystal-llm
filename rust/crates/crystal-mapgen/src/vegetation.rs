use anyhow::Result;

use crate::{GeneratedGrid, MapCell, stable_grid::StableGrid};

const NOTCH_SALT: u64 = 0x4752_4f56_454e_4f54;
const LOBE_SALT: u64 = 0x4752_4f56_454c_4f42;
const PARK_TREE_SALT: u64 = 0x5041_524b_5452_4545;
const SMALL_TREE_SALT: u64 = 0x534d_414c_4c54_5245;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct GroveSummary {
    pub edge_notches: usize,
    pub new_lobes: usize,
    pub park_trees: usize,
    pub small_trees: usize,
}

/// Breaks long rectangular canopy bars into scalloped groves and mixes in the
/// exact National Park large-tree block.
///
/// The initial belt planner is intentionally conservative because it protects
/// routes, homes, and water. This final vegetation authoring pass works only
/// on that safe result: it removes spaced edge cells, adds fewer offset lobes,
/// converts coherent clusters to the imported Park tree, and places small-tree
/// transitions on exposed north/south edges. Connectivity is repaired later.
pub(crate) fn naturalize_groves(grid: &mut GeneratedGrid) -> Result<GroveSummary> {
    let stable_grid = StableGrid::for_grid(grid)?;
    let mut summary = GroveSummary::default();

    carve_canopy_notches(grid, stable_grid, &mut summary);
    add_canopy_lobes(grid, stable_grid, &mut summary);
    author_park_tree_clusters(grid, stable_grid, &mut summary);
    add_small_tree_transitions(grid, stable_grid, &mut summary);
    Ok(summary)
}

fn carve_canopy_notches(
    grid: &mut GeneratedGrid,
    stable_grid: StableGrid,
    summary: &mut GroveSummary,
) {
    let snapshot = grid.cells.clone();
    let width = usize::from(grid.width);
    let target = if grid.width.min(grid.height) >= 56 {
        52
    } else {
        usize::from(grid.width.min(grid.height)) / 2
    };
    let mut candidates = Vec::new();
    for y in 3..grid.height.saturating_sub(3) {
        for x in 3..grid.width.saturating_sub(3) {
            let index = usize::from(y) * width + usize::from(x);
            if snapshot[index] != MapCell::Tree {
                continue;
            }
            let neighbors = [
                snapshot[index - 1],
                snapshot[index + 1],
                snapshot[index - width],
                snapshot[index + width],
            ];
            let tree_neighbors = neighbors
                .iter()
                .filter(|cell| **cell == MapCell::Tree)
                .count();
            let exposed = neighbors.iter().any(|cell| natural_ground(*cell));
            if tree_neighbors < 2 || !exposed {
                continue;
            }
            let world = stable_grid.cell(x, y).expect("in-bounds stable cell");
            candidates.push((world.stable_hash(NOTCH_SALT), x, y));
        }
    }
    candidates.sort_unstable();
    let mut selected = Vec::<(u16, u16)>::new();
    for (_, x, y) in candidates {
        if selected.len() >= target {
            break;
        }
        if selected
            .iter()
            .any(|&(other_x, other_y)| x.abs_diff(other_x) <= 3 && y.abs_diff(other_y) <= 2)
        {
            continue;
        }
        selected.push((x, y));
    }
    for (x, y) in selected {
        replace_cell(grid, x, y, MapCell::Lawn);
        summary.edge_notches += 1;
    }
}

fn add_canopy_lobes(grid: &mut GeneratedGrid, stable_grid: StableGrid, summary: &mut GroveSummary) {
    let snapshot = grid.cells.clone();
    let width = usize::from(grid.width);
    let target = summary.edge_notches / 2;
    let mut candidates = Vec::new();
    for y in 3..grid.height.saturating_sub(3) {
        for x in 3..grid.width.saturating_sub(3) {
            let index = usize::from(y) * width + usize::from(x);
            if !natural_ground(snapshot[index]) || near_protected(grid, x, y, 1) {
                continue;
            }
            let tree_neighbors = [
                snapshot[index - 1],
                snapshot[index + 1],
                snapshot[index - width],
                snapshot[index + width],
            ]
            .into_iter()
            .filter(|cell| *cell == MapCell::Tree)
            .count();
            if tree_neighbors != 1 {
                continue;
            }
            let world = stable_grid.cell(x, y).expect("in-bounds stable cell");
            candidates.push((world.stable_hash(LOBE_SALT), x, y));
        }
    }
    candidates.sort_unstable();
    let mut selected = Vec::<(u16, u16)>::new();
    for (_, x, y) in candidates {
        if selected.len() >= target {
            break;
        }
        if selected
            .iter()
            .any(|&(other_x, other_y)| x.abs_diff(other_x) <= 4 && y.abs_diff(other_y) <= 3)
        {
            continue;
        }
        selected.push((x, y));
    }
    for (x, y) in selected {
        replace_cell(grid, x, y, MapCell::Tree);
        summary.new_lobes += 1;
    }
}

fn author_park_tree_clusters(
    grid: &mut GeneratedGrid,
    stable_grid: StableGrid,
    summary: &mut GroveSummary,
) {
    let ordinary_trees = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Tree)
        .count();
    let target = if grid.width.min(grid.height) >= 56 {
        (ordinary_trees / 80).clamp(4, 12)
    } else {
        ordinary_trees / 80
    };
    let mut candidates = Vec::new();
    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            if grid.cell(x, y) != Some(MapCell::Tree) {
                continue;
            }
            let world = stable_grid.cell(x, y).expect("in-bounds stable cell");
            candidates.push((world.stable_hash(PARK_TREE_SALT), world, x, y));
        }
    }
    candidates.sort_unstable_by_key(|candidate| candidate.0);
    let mut centers = Vec::<(u16, u16)>::new();
    for (seed, _, x, y) in candidates {
        if summary.park_trees >= target {
            break;
        }
        if centers
            .iter()
            .any(|&(other_x, other_y)| x.abs_diff(other_x) <= 4 && y.abs_diff(other_y) <= 4)
        {
            continue;
        }
        let shape = park_tree_shape(seed);
        let positions = shape
            .into_iter()
            .filter_map(|(dx, dy)| {
                let tree_x = i32::from(x) + dx;
                let tree_y = i32::from(y) + dy;
                if tree_x < 0
                    || tree_y < 0
                    || tree_x >= i32::from(grid.width)
                    || tree_y >= i32::from(grid.height)
                    || grid.cell(tree_x as u16, tree_y as u16) != Some(MapCell::Tree)
                {
                    return None;
                }
                Some((tree_x as u16, tree_y as u16))
            })
            .collect::<Vec<_>>();
        if positions.len() >= 3 {
            for (tree_x, tree_y) in &positions {
                replace_cell(grid, *tree_x, *tree_y, MapCell::ParkTree);
            }
            centers.push((x, y));
            summary.park_trees += positions.len();
        }
    }
}

fn park_tree_shape(seed: u64) -> Vec<(i32, i32)> {
    let base: &[(i32, i32)] = match seed.rotate_left(13) % 3 {
        0 => &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1), (1, 1)],
        1 => &[(0, 0), (-1, 0), (1, 0), (-1, -1), (0, -1), (1, 1), (2, 1)],
        _ => &[
            (0, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (0, 1),
            (1, 1),
        ],
    };
    let rotations = (seed & 3) as u32;
    let reflect = seed & 4 != 0;
    base.iter()
        .map(|&(mut x, y)| {
            if reflect {
                x = -x;
            }
            let mut point = (x, y);
            for _ in 0..rotations {
                point = (-point.1, point.0);
            }
            point
        })
        .collect()
}

fn add_small_tree_transitions(
    grid: &mut GeneratedGrid,
    stable_grid: StableGrid,
    summary: &mut GroveSummary,
) {
    let current = grid
        .cells
        .iter()
        .filter(|cell| matches!(cell, MapCell::SmallTree | MapCell::SmallTreeSouth))
        .count();
    let target = if grid.width.min(grid.height) >= 56 {
        42
    } else {
        usize::from(grid.width.min(grid.height)) / 2
    };
    let mut candidates = Vec::new();
    for y in 2..grid.height.saturating_sub(2) {
        for x in 2..grid.width.saturating_sub(2) {
            if !natural_ground(grid.cell(x, y).unwrap_or(MapCell::Grass))
                || near_protected(grid, x, y, 1)
            {
                continue;
            }
            let tree_north = is_canopy(grid.cell(x, y - 1));
            let tree_south = is_canopy(grid.cell(x, y + 1));
            if tree_north == tree_south {
                continue;
            }
            let world = stable_grid.cell(x, y).expect("in-bounds stable cell");
            candidates.push((world.stable_hash(SMALL_TREE_SALT), x, y, tree_north));
        }
    }
    candidates.sort_unstable();
    let mut selected = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            matches!(cell, MapCell::SmallTree | MapCell::SmallTreeSouth).then_some((
                (index % usize::from(grid.width)) as u16,
                (index / usize::from(grid.width)) as u16,
            ))
        })
        .collect::<Vec<_>>();
    for (_, x, y, tree_north) in candidates {
        if selected.len() >= target {
            break;
        }
        if selected
            .iter()
            .any(|&(other_x, other_y)| x.abs_diff(other_x) <= 2 && y.abs_diff(other_y) <= 2)
        {
            continue;
        }
        replace_cell(
            grid,
            x,
            y,
            if tree_north {
                MapCell::SmallTreeSouth
            } else {
                MapCell::SmallTree
            },
        );
        selected.push((x, y));
    }
    summary.small_trees = selected.len().max(current);
}

fn near_protected(grid: &GeneratedGrid, x: u16, y: u16, radius: u16) -> bool {
    for check_y in y.saturating_sub(radius)..=(y + radius).min(grid.height - 1) {
        for check_x in x.saturating_sub(radius)..=(x + radius).min(grid.width - 1) {
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
                        | MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth
                        | MapCell::Pitch
                        | MapCell::Trail
                        | MapCell::Street
                        | MapCell::Road
                        | MapCell::MajorRoad
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
            ) {
                return true;
            }
        }
    }
    false
}

fn natural_ground(cell: MapCell) -> bool {
    matches!(cell, MapCell::Grass | MapCell::Lawn | MapCell::Clearing)
}

fn is_canopy(cell: Option<MapCell>) -> bool {
    matches!(cell, Some(MapCell::Tree | MapCell::ParkTree))
}

fn replace_cell(grid: &mut GeneratedGrid, x: u16, y: u16, cell: MapCell) {
    let index = usize::from(y) * usize::from(grid.width) + usize::from(x);
    if !matches!(
        grid.cells[index],
        MapCell::Street | MapCell::Road | MapCell::MajorRoad
    ) {
        grid.cells[index] = cell;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate, MapSource};

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
                attribution: "grove fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        for (west, east, north, south) in [
            (3_u16, 27_u16, 4_u16, 10_u16),
            (33, 60, 5, 11),
            (7, 37, 23, 29),
            (25, 59, 40, 47),
        ] {
            for y in north..=south {
                for x in west..=east {
                    replace_cell(&mut grid, x, y, MapCell::Tree);
                }
            }
        }
        for x in 0_u16..64 {
            replace_cell(&mut grid, x, 18, MapCell::Road);
            replace_cell(&mut grid, x, 35, MapCell::Trail);
        }
        grid
    }

    #[test]
    fn creates_scalloped_mixed_groves_without_touching_routes() {
        let mut first = fixture();
        let road_cells = first
            .cells
            .iter()
            .enumerate()
            .filter_map(|(index, cell)| {
                matches!(cell, MapCell::Road | MapCell::Trail).then_some((index, *cell))
            })
            .collect::<Vec<_>>();
        let mut second = first.clone();
        let summary = naturalize_groves(&mut first).expect("grove pass");
        let repeated = naturalize_groves(&mut second).expect("repeat grove pass");

        assert_eq!(summary, repeated);
        assert_eq!(first.cells, second.cells);
        assert!(summary.edge_notches >= 20, "{summary:?}");
        assert!(summary.new_lobes >= 8, "{summary:?}");
        assert!((4..=12).contains(&summary.park_trees), "{summary:?}");
        assert!(summary.small_trees >= 24, "{summary:?}");
        for (index, cell) in road_cells {
            assert_eq!(first.cells[index], cell, "vegetation overwrote a route");
        }
        assert_eq!(
            first
                .cells
                .iter()
                .filter(|cell| **cell == MapCell::ParkTree)
                .count(),
            summary.park_trees
        );
    }
}
