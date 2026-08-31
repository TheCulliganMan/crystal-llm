use std::collections::{BTreeMap, VecDeque};

use anyhow::Result;

use crate::{GeneratedGrid, MapCell, stable_grid::StableGrid};

const BIOME_SPAN: i64 = 14;
const BIOME_SALT: u64 = 0x4249_4f4d_4553_4545;
const DETAIL_SALT: u64 = 0x4249_4f4d_4544_4554;
const STRUCTURE_SALT: u64 = 0x5241_5245_5354_5255;
const CAVE_LANDMARK_SALT: u64 = 0x4341_5645_4249_4f4d;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Biome {
    Meadow,
    DeepForest,
    RockyUpland,
    Wetland,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceDungeon {
    Ice,
    Rock,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct BiomeSummary {
    pub meadow_cells: usize,
    pub forest_cells: usize,
    pub rocky_cells: usize,
    pub wetland_cells: usize,
    pub changed_cells: usize,
}

/// Authors irregular, world-stable ecological rooms from a small Crystal
/// sprite vocabulary. Each biome uses a coherent mixture rather than an
/// independent per-cell roll, and water proximity always wins so shorelines
/// remain readable instead of acquiring a hard tree wall.
pub(crate) fn author_biomes(grid: &mut GeneratedGrid) -> Result<BiomeSummary> {
    let addressing = StableGrid::for_grid(grid)?;
    let water_distance = water_distances(grid, 7);
    let snapshot = grid.cells.clone();
    let width = usize::from(grid.width);
    let mut summary = BiomeSummary::default();

    for y in 4..grid.height.saturating_sub(4) {
        for x in 4..grid.width.saturating_sub(4) {
            let index = usize::from(y) * width + usize::from(x);
            let distance = water_distance[index];
            let biome = if distance <= 4 {
                Biome::Wetland
            } else {
                regional_biome(addressing, x, y)
            };
            match biome {
                Biome::Meadow => summary.meadow_cells += 1,
                Biome::DeepForest => summary.forest_cells += 1,
                Biome::RockyUpland => summary.rocky_cells += 1,
                Biome::Wetland => summary.wetland_cells += 1,
            }
            let cell = snapshot[index];
            let detail = detail_score(addressing, x, y);
            let replacement = match biome {
                Biome::Wetland if distance <= 1 => match cell {
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth => Some(MapCell::Grass),
                    _ => None,
                },
                Biome::Wetland => match cell {
                    MapCell::Grass | MapCell::Lawn if detail < 126 => Some(MapCell::Park),
                    MapCell::Grass | MapCell::Lawn if detail > 151 => Some(MapCell::Flowers),
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth
                        if detail < 94 =>
                    {
                        Some(MapCell::SmallTreeSouth)
                    }
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth => Some(MapCell::Grass),
                    _ => None,
                },
                Biome::Meadow => match cell {
                    MapCell::Grass | MapCell::Lawn if detail < 103 => Some(MapCell::Park),
                    MapCell::Grass | MapCell::Lawn if detail > 148 => Some(MapCell::Flowers),
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth
                        if detail < 82 =>
                    {
                        Some(MapCell::SmallTree)
                    }
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth => Some(MapCell::Grass),
                    _ => None,
                },
                Biome::DeepForest => match cell {
                    // The ordinary Johto tree is the real headbutt tree. The
                    // much larger National Park specimen is an accent, not
                    // the forest's default canopy.
                    MapCell::Tree if detail < 7 => Some(MapCell::ParkTree),
                    MapCell::ParkTree if detail >= 32 => Some(MapCell::Tree),
                    MapCell::Grass | MapCell::Lawn if detail < 112 => Some(MapCell::Tree),
                    MapCell::Grass | MapCell::Lawn if (112..132).contains(&detail) => {
                        Some(MapCell::SmallTreeSouth)
                    }
                    MapCell::Grass | MapCell::Lawn if detail > 166 => Some(MapCell::Park),
                    _ => None,
                },
                Biome::RockyUpland => match cell {
                    // Relief and grouped outcrop passes provide the rocks. Do
                    // not fake an upland by sprinkling pale clearing blocks;
                    // those read as broken paths in the Crystal tileset.
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth
                        if detail < 104 =>
                    {
                        Some(MapCell::SmallTreeSouth)
                    }
                    MapCell::Tree
                    | MapCell::ParkTree
                    | MapCell::SmallTree
                    | MapCell::SmallTreeSouth => Some(MapCell::Grass),
                    MapCell::Grass | MapCell::Lawn if detail < 101 || detail > 151 => {
                        Some(MapCell::Flowers)
                    }
                    _ => None,
                },
            };
            if let Some(replacement) = replacement
                && cell != replacement
                && !near_protected(&snapshot, grid.width, grid.height, x, y, 2)
            {
                grid.cells[index] = replacement;
                summary.changed_cells += 1;
            }
        }
    }
    Ok(summary)
}

/// Places scarce outdoor cave districts without changing the map's tileset or
/// turning an entire H3 room into an indoor cave. Regional rooms deterministically
/// receive either an Ice Path grotto, a rocky cavern, or neither; large square
/// overview maps receive both so the biome vocabulary is visible in one render.
pub(crate) fn author_cave_landmarks(grid: &mut GeneratedGrid) -> Result<usize> {
    if grid.width.min(grid.height) < 56 {
        return Ok(0);
    }
    let addressing = StableGrid::for_grid(grid)?;
    let center = addressing
        .cell(grid.width / 2, grid.height / 2)
        .expect("grid center is addressable");
    let selector = center.stable_hash(CAVE_LANDMARK_SALT);
    let wanted: &[SurfaceDungeon] = if grid.source.h3.is_none() && grid.width.min(grid.height) >= 96
    {
        &[SurfaceDungeon::Ice, SurfaceDungeon::Rock]
    } else {
        match selector % 5 {
            0 => &[SurfaceDungeon::Ice],
            1 => &[SurfaceDungeon::Rock],
            _ => &[],
        }
    };
    let mut placed = 0;
    let mut placed_centers = Vec::<(u16, u16)>::new();
    for (kind_index, kind) in wanted.iter().copied().enumerate() {
        let variant = (selector.rotate_left((kind_index * 11) as u32) % 3) as u8;
        let footprint = match kind {
            SurfaceDungeon::Ice => ice_grotto(variant),
            SurfaceDungeon::Rock => rocky_cavern(variant),
        };
        let min_dx = footprint.iter().map(|(dx, _, _)| *dx).min().unwrap_or(0);
        let max_dx = footprint.iter().map(|(dx, _, _)| *dx).max().unwrap_or(0);
        let min_dy = footprint.iter().map(|(_, dy, _)| *dy).min().unwrap_or(0);
        let max_dy = footprint.iter().map(|(_, dy, _)| *dy).max().unwrap_or(0);
        let mut candidates = Vec::new();
        let margin_x = u16::try_from(max_dx.abs().max(min_dx.abs()) + 4).unwrap_or(10);
        let margin_y = u16::try_from(max_dy.abs().max(min_dy.abs()) + 4).unwrap_or(10);
        for y in margin_y..grid.height.saturating_sub(margin_y) {
            for x in margin_x..grid.width.saturating_sub(margin_x) {
                if let Some(plan) = &grid.source.h3
                    && !plan.raster_footprint_fits(
                        i32::from(x) + min_dx,
                        i32::from(y) + min_dy,
                        u16::try_from(max_dx - min_dx + 1).expect("surface dungeon width fits"),
                        u16::try_from(max_dy - min_dy + 1).expect("surface dungeon height fits"),
                        3,
                        grid.width,
                        grid.height,
                    )?
                {
                    continue;
                }
                if footprint.iter().all(|&(dx, dy, _)| {
                    let px = i32::from(x) + dx;
                    let py = i32::from(y) + dy;
                    px >= 0
                        && py >= 0
                        && px < i32::from(grid.width)
                        && py < i32::from(grid.height)
                        && matches!(
                            grid.cell(px as u16, py as u16),
                            Some(
                                MapCell::Grass
                                    | MapCell::Clearing
                                    | MapCell::Flowers
                                    | MapCell::Tree
                                    | MapCell::ParkTree
                                    | MapCell::SmallTree
                                    | MapCell::SmallTreeSouth
                            )
                        )
                        && !near_protected(
                            &grid.cells,
                            grid.width,
                            grid.height,
                            px as u16,
                            py as u16,
                            1,
                        )
                }) {
                    let seed = addressing
                        .cell(x, y)
                        .expect("cave candidate in bounds")
                        .stable_hash(CAVE_LANDMARK_SALT ^ kind_index as u64);
                    candidates.push((seed, x, y));
                }
            }
        }
        candidates.sort_unstable();
        let Some((_, x, y)) = candidates.into_iter().find(|&(_, x, y)| {
            placed_centers
                .iter()
                .all(|&(px, py)| px.abs_diff(x) >= 18 || py.abs_diff(y) >= 18)
        }) else {
            continue;
        };
        for (dx, dy, cell) in footprint {
            let px = (i32::from(x) + dx) as usize;
            let py = (i32::from(y) + dy) as usize;
            grid.cells[py * usize::from(grid.width) + px] = cell;
        }
        placed += 1;
        placed_centers.push((x, y));
    }
    Ok(placed)
}

fn room(
    cells: &mut BTreeMap<(i32, i32), MapCell>,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    floor: MapCell,
) {
    for y in top..top + height {
        for x in left..left + width {
            cells.insert((x, y), floor);
        }
    }
}

fn corridor(cells: &mut BTreeMap<(i32, i32), MapCell>, points: &[(i32, i32)], floor: MapCell) {
    for segment in points.windows(2) {
        let (mut x, mut y) = segment[0];
        let (end_x, end_y) = segment[1];
        cells.insert((x, y), floor);
        while (x, y) != (end_x, end_y) {
            x += (end_x - x).signum();
            y += (end_y - y).signum();
            cells.insert((x, y), floor);
        }
    }
}

fn transform_surface_dungeon(
    cells: BTreeMap<(i32, i32), MapCell>,
    variant: u8,
) -> Vec<(i32, i32, MapCell)> {
    cells
        .into_iter()
        .map(|((x, y), cell)| {
            let (x, y) = match variant % 3 {
                0 => (x, y),
                1 => (-x, y),
                _ => (y, -x),
            };
            (x, y, cell)
        })
        .collect()
}

/// Three chambers, a central junction, a looped lower gallery, and two narrow
/// necks. The open floor is deliberately discontinuous in its bounding box so
/// it reads as an outdoor dungeon plan rather than a colored pond.
pub(crate) fn ice_grotto(variant: u8) -> Vec<(i32, i32, MapCell)> {
    let mut cells = BTreeMap::new();
    room(&mut cells, -5, -4, 4, 4, MapCell::IceFloor);
    room(&mut cells, 2, -5, 4, 4, MapCell::IceFloor);
    room(&mut cells, -1, -1, 3, 3, MapCell::IceFloor);
    room(&mut cells, -5, 3, 4, 3, MapCell::IceFloor);
    corridor(&mut cells, &[(-2, -2), (0, -2), (0, -1)], MapCell::IceFloor);
    corridor(&mut cells, &[(2, -2), (0, -2)], MapCell::IceFloor);
    corridor(&mut cells, &[(-1, 1), (-1, 3), (-3, 3)], MapCell::IceFloor);
    corridor(
        &mut cells,
        &[(-1, 4), (3, 4), (3, 1), (1, 1)],
        MapCell::IceFloor,
    );
    corridor(&mut cells, &[(0, 1), (0, 6)], MapCell::IceFloor);
    for point in [
        (-3, -3),
        (4, -3),
        (-4, 4),
        (0, 0),
        (-6, -4),
        (-1, -5),
        (1, -5),
        (6, -4),
        (-6, 1),
        (5, 1),
        (-6, 5),
        (4, 5),
    ] {
        cells.insert(point, MapCell::IceBoulder);
    }
    cells.insert((0, 6), MapCell::Trail);
    cells.insert((0, 7), MapCell::Trail);
    transform_surface_dungeon(cells, variant)
}

/// Four unequal rooms around a boulder spine. A west-east loop surrounds the
/// hub while the north vault and east alcove terminate as optional branches.
fn rocky_cavern(variant: u8) -> Vec<(i32, i32, MapCell)> {
    let mut cells = BTreeMap::new();
    room(&mut cells, -6, -2, 4, 4, MapCell::RockFloor);
    room(&mut cells, -1, 0, 3, 3, MapCell::RockFloor);
    room(&mut cells, 3, -3, 4, 3, MapCell::RockFloor);
    room(&mut cells, -2, -7, 5, 3, MapCell::RockFloor);
    corridor(&mut cells, &[(-3, 0), (-1, 0)], MapCell::RockFloor);
    corridor(&mut cells, &[(1, 0), (3, 0), (3, -1)], MapCell::RockFloor);
    corridor(&mut cells, &[(0, 0), (0, -4)], MapCell::RockFloor);
    corridor(
        &mut cells,
        &[(-3, 1), (-3, 4), (4, 4), (4, -1)],
        MapCell::RockFloor,
    );
    corridor(&mut cells, &[(0, 2), (0, 6)], MapCell::RockFloor);
    for point in [
        (-4, -1),
        (-1, -2),
        (5, -2),
        (0, 1),
        (-7, -3),
        (-2, -3),
        (2, -3),
        (7, -2),
        (-7, 2),
        (6, 1),
        (-4, 4),
        (5, 4),
        (-3, -7),
        (3, -7),
    ] {
        cells.insert(point, MapCell::Boulder);
    }
    cells.insert((0, 6), MapCell::Trail);
    cells.insert((0, 7), MapCell::Trail);
    transform_surface_dungeon(cells, variant)
}

pub(crate) fn prefers_dense_canopy(addressing: StableGrid, x: u16, y: u16) -> bool {
    regional_biome(addressing, x, y) == Biome::DeepForest
}

/// Low-frequency deterministic noise. Adjacent cells share 24/25 samples,
/// producing irregular patches instead of per-cell visual confetti.
fn detail_score(addressing: StableGrid, x: u16, y: u16) -> u8 {
    let center = addressing.cell(x, y).expect("biome detail is in bounds");
    let mut total = 0_u32;
    for dy in -2..=2 {
        for dx in -2..=2 {
            total += (center.offset(dx, dy).stable_hash(DETAIL_SALT) & 0xff) as u32;
        }
    }
    (total / 25) as u8
}

/// Adds at most one rare compound landmark per face. The 1/9 selection is
/// address-stable, and every stamp has an opening and walkable center so the
/// landmark reads as a destination rather than a random obstacle pile.
pub(crate) fn author_rare_structure(grid: &mut GeneratedGrid) -> Result<bool> {
    let addressing = StableGrid::for_grid(grid)?;
    let center = addressing
        .cell(grid.width / 2, grid.height / 2)
        .expect("grid center is addressable");
    let selector = center.stable_hash(STRUCTURE_SALT);
    if selector % 9 != 0 {
        return Ok(false);
    }
    let footprint = match selector.rotate_left(17) % 3 {
        0 => stone_circle(),
        1 => flower_sanctuary(),
        _ => ancient_grove(),
    };
    let mut candidates = Vec::new();
    for y in 8..grid.height.saturating_sub(8) {
        for x in 8..grid.width.saturating_sub(8) {
            if footprint.iter().all(|&(dx, dy, _)| {
                let px = i32::from(x) + dx;
                let py = i32::from(y) + dy;
                px >= 0
                    && py >= 0
                    && px < i32::from(grid.width)
                    && py < i32::from(grid.height)
                    && matches!(
                        grid.cell(px as u16, py as u16),
                        Some(MapCell::Grass | MapCell::Lawn | MapCell::Clearing | MapCell::Flowers)
                    )
                    && !near_protected(
                        &grid.cells,
                        grid.width,
                        grid.height,
                        px as u16,
                        py as u16,
                        1,
                    )
            }) {
                let hash = addressing
                    .cell(x, y)
                    .expect("candidate is in bounds")
                    .stable_hash(STRUCTURE_SALT ^ 0x9e37);
                candidates.push((hash, x, y));
            }
        }
    }
    let Some((_, x, y)) = candidates.into_iter().min() else {
        return Ok(false);
    };
    for (dx, dy, cell) in footprint {
        let px = (i32::from(x) + dx) as usize;
        let py = (i32::from(y) + dy) as usize;
        grid.cells[py * usize::from(grid.width) + px] = cell;
    }
    Ok(true)
}

/// Guarantees a modest encounter-grass floor with small asymmetric tufts.
/// This runs after structural authoring so later roads, cliffs, or buildings
/// cannot consume the biome's visible texture budget.
pub(crate) fn top_up_tall_grass(grid: &mut GeneratedGrid) -> Result<usize> {
    if grid.width.min(grid.height) < 56 {
        return Ok(0);
    }
    let authored = grid
        .cells
        .iter()
        .filter(|cell| **cell != MapCell::H3Void)
        .count();
    let target = (authored * 58).div_ceil(1000);
    let mut current = grid
        .cells
        .iter()
        .filter(|cell| **cell == MapCell::Park)
        .count();
    if current >= target {
        return Ok(0);
    }
    let addressing = StableGrid::for_grid(grid)?;
    const SHAPES: [&[(i32, i32)]; 3] = [
        &[(0, 0), (1, 0), (0, 1), (0, 2)],
        &[(0, 0), (1, 0), (1, 1), (2, 1)],
        &[(1, 0), (0, 1), (1, 1), (2, 1), (2, 2)],
    ];
    let mut candidates = Vec::new();
    for y in 4..grid.height.saturating_sub(5) {
        for x in 4..grid.width.saturating_sub(5) {
            let seed = addressing
                .cell(x, y)
                .expect("tall-grass candidate in bounds")
                .stable_hash(DETAIL_SALT ^ 0x544f_5055);
            let shape = SHAPES[seed as usize % SHAPES.len()];
            let park_nearby = y.saturating_sub(2)..=(y + 4).min(grid.height - 1);
            if !park_nearby.clone().any(|py| {
                (x.saturating_sub(2)..=(x + 4).min(grid.width - 1))
                    .any(|px| grid.cell(px, py) == Some(MapCell::Park))
            }) && shape.iter().all(|&(dx, dy)| {
                let px = (i32::from(x) + dx) as u16;
                let py = (i32::from(y) + dy) as u16;
                matches!(grid.cell(px, py), Some(MapCell::Grass | MapCell::Lawn))
                    && !near_protected(&grid.cells, grid.width, grid.height, px, py, 1)
            }) {
                candidates.push((seed, x, y));
            }
        }
    }
    candidates.sort_unstable();
    let mut added = 0;
    let mut selected = Vec::<(u16, u16)>::new();
    for (seed, x, y) in candidates {
        if current >= target {
            break;
        }
        if selected
            .iter()
            .any(|&(sx, sy)| x.abs_diff(sx) < 6 && y.abs_diff(sy) < 6)
        {
            continue;
        }
        let shape = SHAPES[seed as usize % SHAPES.len()];
        for &(dx, dy) in shape {
            let px = (i32::from(x) + dx) as usize;
            let py = (i32::from(y) + dy) as usize;
            let index = py * usize::from(grid.width) + px;
            if matches!(grid.cells[index], MapCell::Grass | MapCell::Lawn) {
                grid.cells[index] = MapCell::Park;
                current += 1;
                added += 1;
            }
        }
        selected.push((x, y));
    }
    Ok(added)
}

fn regional_biome(addressing: StableGrid, x: u16, y: u16) -> Biome {
    let cell = addressing.cell(x, y).expect("biome cell is in bounds");
    let local_x = i64::from(x);
    let local_y = i64::from(y);
    let region_x = local_x.div_euclid(BIOME_SPAN);
    let region_y = local_y.div_euclid(BIOME_SPAN);
    let mut nearest = (i64::MAX, 0_u64);
    for offset_y in -1..=1 {
        for offset_x in -1..=1 {
            let anchor_x = (region_x + offset_x) * BIOME_SPAN;
            let anchor_y = (region_y + offset_y) * BIOME_SPAN;
            let anchor = cell.offset(anchor_x - local_x, anchor_y - local_y);
            let seed = anchor.stable_hash(BIOME_SALT);
            let jitter_x = (seed & 7) as i64 - 3;
            let jitter_y = ((seed >> 3) & 7) as i64 - 3;
            let dx = local_x - (anchor_x + BIOME_SPAN / 2 + jitter_x);
            let dy = local_y - (anchor_y + BIOME_SPAN / 2 + jitter_y);
            let distance = dx * dx + dy * dy;
            nearest = nearest.min((distance, seed));
        }
    }
    match nearest.1 % 4 {
        0 => Biome::Meadow,
        1 | 2 => Biome::DeepForest,
        _ => Biome::RockyUpland,
    }
}

fn water_distances(grid: &GeneratedGrid, maximum: u8) -> Vec<u8> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut distances = vec![u8::MAX; grid.cells.len()];
    let mut queue = VecDeque::new();
    for (index, cell) in grid.cells.iter().enumerate() {
        if matches!(
            cell,
            MapCell::Water
                | MapCell::WaterAccessEast
                | MapCell::WaterAccessWest
                | MapCell::WaterAccessSouth
        ) {
            distances[index] = 0;
            queue.push_back(index);
        }
    }
    while let Some(index) = queue.pop_front() {
        let distance = distances[index];
        if distance >= maximum {
            continue;
        }
        let x = index % width;
        let y = index / width;
        for (nx, ny) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if nx >= width || ny >= height {
                continue;
            }
            let next = ny * width + nx;
            if distances[next] > distance + 1 {
                distances[next] = distance + 1;
                queue.push_back(next);
            }
        }
    }
    distances
}

fn near_protected(cells: &[MapCell], width: u16, height: u16, x: u16, y: u16, radius: u16) -> bool {
    for py in y.saturating_sub(radius)..=(y + radius).min(height - 1) {
        for px in x.saturating_sub(radius)..=(x + radius).min(width - 1) {
            let cell = cells[usize::from(py) * usize::from(width) + usize::from(px)];
            if !matches!(
                cell,
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
            ) {
                return true;
            }
        }
    }
    false
}

fn stone_circle() -> Vec<(i32, i32, MapCell)> {
    vec![
        (-2, -2, MapCell::Boulder),
        (0, -2, MapCell::Boulder),
        (2, -2, MapCell::Boulder),
        (-2, 0, MapCell::Boulder),
        (0, 0, MapCell::GroundSign),
        (2, 0, MapCell::Boulder),
        (-2, 2, MapCell::Boulder),
        (2, 2, MapCell::Boulder),
    ]
}

fn flower_sanctuary() -> Vec<(i32, i32, MapCell)> {
    vec![
        (-1, -2, MapCell::Flowers),
        (0, -2, MapCell::Flowers),
        (1, -2, MapCell::Flowers),
        (-2, -1, MapCell::Flowers),
        (2, -1, MapCell::Flowers),
        (-2, 0, MapCell::Bench),
        (0, 0, MapCell::Fountain),
        (2, 0, MapCell::Bench),
        (-2, 1, MapCell::Flowers),
        (2, 1, MapCell::Flowers),
        (-1, 2, MapCell::Flowers),
        (1, 2, MapCell::Flowers),
    ]
}

fn ancient_grove() -> Vec<(i32, i32, MapCell)> {
    vec![
        (-2, -2, MapCell::ParkTree),
        (0, -2, MapCell::ParkTree),
        (2, -2, MapCell::ParkTree),
        (-2, 0, MapCell::ParkTree),
        (0, 0, MapCell::GroundSign),
        (2, 0, MapCell::ParkTree),
        (-2, 2, MapCell::ParkTree),
        (2, 2, MapCell::ParkTree),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate, MapSource};
    use std::collections::{BTreeSet, VecDeque};

    fn fixture() -> GeneratedGrid {
        let mut grid = GeneratedGrid {
            source: MapSource {
                center: Coordinate {
                    lat: 44.95,
                    lon: -93.32,
                },
                bounds: BoundingBox {
                    south: 44.94,
                    west: -93.33,
                    north: 44.96,
                    east: -93.31,
                },
                attribution: "biome fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        for y in 8..56 {
            for x in 7..12 {
                grid.cells[y * 64 + x] = MapCell::Water;
            }
        }
        for y in 10..54 {
            for x in 20..44 {
                if (x + y) % 3 == 0 {
                    grid.cells[y * 64 + x] = MapCell::Tree;
                }
            }
        }
        grid
    }

    fn assert_surface_dungeon_layout(
        footprint: &[(i32, i32, MapCell)],
        floor: MapCell,
        blocker: MapCell,
    ) {
        let walkable = footprint
            .iter()
            .filter_map(|&(x, y, cell)| {
                matches!(cell, MapCell::Trail)
                    .then_some((x, y))
                    .or_else(|| (cell == floor).then_some((x, y)))
            })
            .collect::<BTreeSet<_>>();
        let blockers = footprint
            .iter()
            .filter(|(_, _, cell)| *cell == blocker)
            .count();
        assert!(
            walkable.len() >= 48,
            "surface dungeon is too small: {}",
            walkable.len()
        );
        assert!(blockers >= 10, "surface dungeon needs authored blockers");

        let start = *walkable.iter().next().expect("surface dungeon has floor");
        let mut reached = BTreeSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some((x, y)) = queue.pop_front() {
            for next in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                if walkable.contains(&next) && reached.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        assert_eq!(reached, walkable, "every room and passage must connect");

        let min_x = walkable.iter().map(|(x, _)| *x).min().unwrap();
        let max_x = walkable.iter().map(|(x, _)| *x).max().unwrap();
        let min_y = walkable.iter().map(|(_, y)| *y).min().unwrap();
        let max_y = walkable.iter().map(|(_, y)| *y).max().unwrap();
        let bbox_area = usize::try_from((max_x - min_x + 1) * (max_y - min_y + 1)).unwrap();
        assert!(
            walkable.len() * 100 <= bbox_area * 62,
            "layout still fills {:.1}% of its bounding box like a blob",
            walkable.len() as f64 / bbox_area as f64 * 100.0
        );

        let degree = |x, y| {
            [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .into_iter()
                .filter(|point| walkable.contains(point))
                .count()
        };
        let passages = walkable.iter().filter(|&&(x, y)| degree(x, y) <= 2).count();
        let junctions = walkable.iter().filter(|&&(x, y)| degree(x, y) >= 3).count();
        let room_squares = (min_y..max_y)
            .flat_map(|y| (min_x..max_x).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
                    .into_iter()
                    .all(|point| walkable.contains(&point))
            })
            .count();
        assert!(
            passages >= 10,
            "needs narrow navigational passages: {passages}"
        );
        assert!(
            junctions >= 8,
            "needs branching room junctions: {junctions}"
        );
        assert!(
            room_squares >= 10,
            "needs several broad chambers: {room_squares}"
        );
        assert_eq!(
            footprint
                .iter()
                .filter(|(_, _, cell)| *cell == MapCell::Trail)
                .count(),
            2,
            "surface dungeon needs one explicit two-block entrance"
        );
    }

    #[test]
    fn surface_cave_catalog_uses_rooms_passages_branches_and_varied_silhouettes() {
        let mut ice_signatures = BTreeSet::new();
        let mut rock_signatures = BTreeSet::new();
        for variant in 0..3 {
            let ice = ice_grotto(variant);
            let rock = rocky_cavern(variant);
            assert_surface_dungeon_layout(&ice, MapCell::IceFloor, MapCell::IceBoulder);
            assert_surface_dungeon_layout(&rock, MapCell::RockFloor, MapCell::Boulder);
            ice_signatures.insert(
                ice.iter()
                    .map(|(x, y, cell)| (*x, *y, *cell as u8))
                    .collect::<Vec<_>>(),
            );
            rock_signatures.insert(
                rock.iter()
                    .map(|(x, y, cell)| (*x, *y, *cell as u8))
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(ice_signatures.len(), 3);
        assert_eq!(rock_signatures.len(), 3);
    }

    #[test]
    fn authors_multiple_cohesive_biomes_and_keeps_a_clear_shore() {
        let mut grid = fixture();
        let summary = author_biomes(&mut grid).expect("biomes");
        assert!(summary.meadow_cells > 100, "{summary:?}");
        assert!(summary.forest_cells > 100, "{summary:?}");
        assert!(summary.rocky_cells > 100, "{summary:?}");
        assert!(summary.wetland_cells > 100, "{summary:?}");
        assert!(summary.changed_cells > 80, "{summary:?}");
        assert_eq!(
            grid.cells
                .iter()
                .filter(|cell| **cell == MapCell::Clearing)
                .count(),
            0,
            "biomes must not masquerade as a network of pale paths"
        );
        for y in 8..56 {
            assert!(!matches!(
                grid.cell(12, y),
                Some(MapCell::Tree | MapCell::ParkTree)
            ));
        }

        let addressing = StableGrid::for_grid(&grid).expect("stable biome grid");
        let mut forest = [0_usize; 2];
        for y in 4..60 {
            for x in 4..60 {
                let dense_region = prefers_dense_canopy(addressing, x, y);
                let canopy = matches!(
                    grid.cell(x, y),
                    Some(
                        MapCell::Tree
                            | MapCell::ParkTree
                            | MapCell::SmallTree
                            | MapCell::SmallTreeSouth
                    )
                );
                forest[usize::from(dense_region)] += usize::from(canopy);
            }
        }
        assert!(
            forest[1] > forest[0],
            "dense forest must remain visually stronger than the other biomes: {forest:?}"
        );
        let headbutt = grid
            .cells
            .iter()
            .filter(|cell| **cell == MapCell::Tree)
            .count();
        let park_trees = grid
            .cells
            .iter()
            .filter(|cell| **cell == MapCell::ParkTree)
            .count();
        assert!(headbutt > 0);
        assert!(
            park_trees * 12 <= headbutt.max(1),
            "National Park trees must remain rare accents: park={park_trees}, headbutt={headbutt}"
        );
    }

    #[test]
    fn rare_structures_are_complete_or_absent_and_deterministic() {
        let mut first = fixture();
        let mut second = first.clone();
        let placed = author_rare_structure(&mut first).expect("rare structure");
        assert_eq!(placed, author_rare_structure(&mut second).expect("repeat"));
        assert_eq!(first.cells, second.cells);
        if placed {
            assert!(
                first.cells.contains(&MapCell::GroundSign)
                    || first.cells.contains(&MapCell::Fountain)
            );
        }
    }

    #[test]
    fn rare_structures_remain_low_probability_but_occur_across_a_region() {
        let mut placements = 0;
        for offset in 0..72 {
            let mut grid = fixture();
            grid.source.center.lon += f64::from(offset) * 0.002;
            grid.source.bounds.west += f64::from(offset) * 0.002;
            grid.source.bounds.east += f64::from(offset) * 0.002;
            placements += usize::from(author_rare_structure(&mut grid).expect("rare structure"));
        }
        assert!((3..=12).contains(&placements), "placements={placements}");
    }

    #[test]
    fn large_overview_gets_one_ice_grotto_and_one_rocky_cavern() {
        let mut grid = fixture();
        grid.width = 96;
        grid.height = 96;
        grid.cells = vec![MapCell::Grass; 96 * 96];
        grid.source.bounds.north += 0.02;
        grid.source.bounds.east += 0.02;

        assert_eq!(author_cave_landmarks(&mut grid).expect("cave biomes"), 2);
        assert!(grid.cells.contains(&MapCell::IceFloor));
        assert!(grid.cells.contains(&MapCell::IceBoulder));
        assert!(grid.cells.contains(&MapCell::RockFloor));
        assert!(
            grid.cells
                .iter()
                .filter(|cell| **cell == MapCell::IceFloor)
                .count()
                >= 30
        );
        assert!(
            grid.cells
                .iter()
                .filter(|cell| **cell == MapCell::Boulder)
                .count()
                >= 10
        );
    }
}
