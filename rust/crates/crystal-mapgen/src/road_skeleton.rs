use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{Feature, FeatureKind, MapSource, WorldCell, WorldGrid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadAxis {
    EastWest,
    NorthSouth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadSkeletonCell {
    pub world: WorldCell,
    pub local_x: u16,
    pub local_y: u16,
    pub kind: FeatureKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoadSkeleton {
    pub grid: WorldGrid,
    pub cells: Vec<RoadSkeletonCell>,
}

impl RoadSkeleton {
    pub fn kind_at_world(&self, world: WorldCell) -> Option<FeatureKind> {
        self.cells
            .binary_search_by_key(&(world.y, world.x), |cell| (cell.world.y, cell.world.x))
            .ok()
            .map(|index| self.cells[index].kind)
    }

    pub fn kind_at_local(&self, x: u16, y: u16) -> Option<FeatureKind> {
        let world = self.grid.world_cell(x, y)?;
        self.kind_at_world(world)
    }
}

/// Build a sparse Pokémon-style road network in global coordinates, then crop
/// it to this output window.
///
/// Every decision is made before conversion to local `(x, y)`: axis choice,
/// lane snapping, corridor width, and overlap priority therefore remain stable
/// when the requested center moves. Nearby residential streets collapse onto a
/// shared coarse lane instead of being selected by a window-local top-N list.
pub fn build_road_skeleton(source: &MapSource, grid: WorldGrid) -> Result<RoadSkeleton> {
    let mut cells = BTreeMap::<WorldCell, FeatureKind>::new();
    for feature in source.features.iter().filter(|feature| {
        matches!(
            feature.kind,
            FeatureKind::Street | FeatureKind::Road | FeatureKind::MajorRoad
        )
    }) {
        rasterize_feature(feature, grid, &mut cells)?;
    }
    let mut cells = cells
        .into_iter()
        .filter_map(|(world, kind)| {
            let (local_x, local_y) = grid.local_cell(world)?;
            Some(RoadSkeletonCell {
                world,
                local_x,
                local_y,
                kind,
            })
        })
        .collect::<Vec<_>>();
    // `kind_at_world` uses this ordering. Keeping the global key in the result
    // also makes overlap audits independent of each map's local origin.
    cells.sort_by_key(|cell| (cell.world.y, cell.world.x));
    Ok(RoadSkeleton { grid, cells })
}

fn rasterize_feature(
    feature: &Feature,
    grid: WorldGrid,
    output: &mut BTreeMap<WorldCell, FeatureKind>,
) -> Result<()> {
    let mut points = feature
        .points
        .iter()
        .map(|coordinate| grid.project_cell(*coordinate))
        .collect::<Result<Vec<_>>>()?;
    points.dedup();
    if points.len() < 2 {
        return Ok(());
    }
    let min_x = points.iter().map(|point| point.x).min().unwrap_or(0);
    let max_x = points.iter().map(|point| point.x).max().unwrap_or(0);
    let min_y = points.iter().map(|point| point.y).min().unwrap_or(0);
    let max_y = points.iter().map(|point| point.y).max().unwrap_or(0);
    let axis = if max_x - min_x >= max_y - min_y {
        RoadAxis::EastWest
    } else {
        RoadAxis::NorthSouth
    };
    let snap_step = match feature.kind {
        FeatureKind::MajorRoad => 2,
        FeatureKind::Road => 4,
        FeatureKind::Street => 6,
        _ => 1,
    };
    let width = usize::from(feature.kind == FeatureKind::MajorRoad) + 1;
    match axis {
        RoadAxis::EastWest => {
            let mut ordinates = points.iter().map(|point| point.y).collect::<Vec<_>>();
            ordinates.sort_unstable();
            let lane = snap(ordinates[ordinates.len() / 2], snap_step);
            for offset in 0..width as i64 {
                for x in min_x..=max_x {
                    insert_if_visible(
                        output,
                        grid,
                        WorldCell {
                            x,
                            y: lane + offset,
                        },
                        feature.kind,
                    );
                }
            }
        }
        RoadAxis::NorthSouth => {
            let mut ordinates = points.iter().map(|point| point.x).collect::<Vec<_>>();
            ordinates.sort_unstable();
            let lane = snap(ordinates[ordinates.len() / 2], snap_step);
            for offset in 0..width as i64 {
                for y in min_y..=max_y {
                    insert_if_visible(
                        output,
                        grid,
                        WorldCell {
                            x: lane + offset,
                            y,
                        },
                        feature.kind,
                    );
                }
            }
        }
    }
    Ok(())
}

fn insert_if_visible(
    output: &mut BTreeMap<WorldCell, FeatureKind>,
    grid: WorldGrid,
    world: WorldCell,
    kind: FeatureKind,
) {
    if !grid.contains(world) {
        return;
    }
    output
        .entry(world)
        .and_modify(|current| {
            if road_priority(kind) > road_priority(*current) {
                *current = kind;
            }
        })
        .or_insert(kind);
}

fn road_priority(kind: FeatureKind) -> u8 {
    match kind {
        FeatureKind::Street => 1,
        FeatureKind::Road => 2,
        FeatureKind::MajorRoad => 3,
        _ => 0,
    }
}

fn snap(value: i64, step: i64) -> i64 {
    ((value as f64 / step as f64).round() as i64) * step
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate};

    const METERS_PER_MILE: f64 = 1_609.344;
    const CENTER: Coordinate = Coordinate {
        lat: 44.947_519_6,
        lon: -93.325_347_7,
    };

    fn source(center: Coordinate, features: Vec<Feature>) -> MapSource {
        MapSource {
            center,
            bounds: BoundingBox::square_miles_around(center, 1.0).expect("bounds"),
            attribution: "test".to_string(),
            features,
            h3: None,
        }
    }

    fn lake_street() -> Feature {
        Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("Lake Street".to_string()),
            area: false,
            bridge: false,
            points: vec![
                Coordinate {
                    lat: CENTER.lat - 0.000_15,
                    lon: CENTER.lon - 0.02,
                },
                Coordinate {
                    lat: CENTER.lat + 0.000_12,
                    lon: CENTER.lon - 0.005,
                },
                Coordinate {
                    lat: CENTER.lat - 0.000_08,
                    lon: CENTER.lon + 0.006,
                },
                Coordinate {
                    lat: CENTER.lat + 0.000_10,
                    lon: CENTER.lon + 0.02,
                },
            ],
        }
    }

    #[test]
    fn a_real_east_west_road_becomes_one_coherent_global_corridor() {
        let source = source(CENTER, vec![lake_street()]);
        let grid = WorldGrid::around(CENTER, 1.0, 64, 64).expect("grid");
        let skeleton = build_road_skeleton(&source, grid).expect("skeleton");
        assert!(!skeleton.cells.is_empty());
        let rows = skeleton
            .cells
            .iter()
            .map(|cell| cell.world.y)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(rows.len(), 2, "major road is exactly two global rows wide");
        assert!(
            skeleton
                .cells
                .iter()
                .all(|cell| cell.kind == FeatureKind::MajorRoad)
        );
    }

    #[test]
    fn half_mile_shifted_windows_have_identical_overlap_cells() {
        let longitude_delta = (METERS_PER_MILE / 2.0) / (111_320.0 * CENTER.lat.to_radians().cos());
        let east_center = Coordinate {
            lon: CENTER.lon + longitude_delta,
            ..CENTER
        };
        let feature = lake_street();
        let west_source = source(CENTER, vec![feature.clone()]);
        let east_source = source(east_center, vec![feature]);
        let west_grid = WorldGrid::around(CENTER, 1.0, 64, 64).expect("west grid");
        let east_grid = WorldGrid::around(east_center, 1.0, 64, 64).expect("east grid");
        let west = build_road_skeleton(&west_source, west_grid).expect("west skeleton");
        let east = build_road_skeleton(&east_source, east_grid).expect("east skeleton");
        let overlap = west_grid.intersection(east_grid).expect("overlap");
        for y in overlap.south..=overlap.north {
            for x in overlap.west..=overlap.east {
                let world = WorldCell { x, y };
                assert_eq!(
                    west.kind_at_world(world),
                    east.kind_at_world(world),
                    "road mismatch at global cell {world:?}"
                );
            }
        }
    }

    #[test]
    fn residential_lanes_snap_without_window_local_ranking() {
        let mut first = lake_street();
        first.kind = FeatureKind::Street;
        first.name = Some("West 31st Street".to_string());
        let mut second = first.clone();
        second.name = Some("West 32nd Street".to_string());
        for point in &mut second.points {
            point.lat += 0.000_25;
        }
        let source = source(CENTER, vec![first, second]);
        let grid = WorldGrid::around(CENTER, 1.0, 64, 64).expect("grid");
        let skeleton = build_road_skeleton(&source, grid).expect("skeleton");
        let rows = skeleton
            .cells
            .iter()
            .map(|cell| cell.world.y)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            rows.len() <= 2,
            "nearby streets collapse onto coarse global lanes"
        );
        assert!(rows.iter().all(|row| row.rem_euclid(6) == 0));
    }
}
