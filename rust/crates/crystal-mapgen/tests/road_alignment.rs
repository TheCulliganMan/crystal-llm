use std::collections::BTreeSet;

use crystal_mapgen::{
    BoundingBox, Coordinate, Feature, FeatureKind, MapCell, MapSource, WorldCell, WorldGrid,
    generate_grid,
};

const GRID_SIZE: u16 = 64;
const SIDE_MILES: f64 = 1.0;
const METERS_PER_MILE: f64 = 1_609.344;
const CENTER: Coordinate = Coordinate {
    lat: 44.947_519_6,
    lon: -93.325_347_7,
};

#[test]
fn mapped_road_semantics_are_identical_in_cardinal_half_mile_overlaps() {
    let half_mile_longitude = (METERS_PER_MILE / 2.0) / (111_320.0 * CENTER.lat.to_radians().cos());
    let half_mile_latitude = (METERS_PER_MILE / 2.0) / 111_320.0;
    let geometry = vec![
        east_west_road(FeatureKind::Street, "31st Street", CENTER.lat + 0.006_3),
        east_west_road(FeatureKind::MajorRoad, "Lake Street", CENTER.lat + 0.001_7),
        east_west_road(FeatureKind::Road, "Parkway", CENTER.lat - 0.004_2),
        north_south_road(FeatureKind::Road, "France Avenue", CENTER.lon + 0.004),
    ];
    let base_source = source(CENTER, geometry.clone());
    let base_world =
        WorldGrid::from_bounds(base_source.center, base_source.bounds, GRID_SIZE, GRID_SIZE)
            .expect("base world grid");
    let base = generate_grid(base_source, GRID_SIZE, GRID_SIZE).expect("base generated map");
    let shifts = [
        (
            "east",
            Coordinate {
                lon: CENTER.lon + half_mile_longitude,
                ..CENTER
            },
            east_west_segment(
                FeatureKind::MajorRoad,
                "East-only arterial",
                CENTER.lat + 0.006_35,
                CENTER.lon + 0.012,
                CENTER.lon + 0.019,
            ),
        ),
        (
            "west",
            Coordinate {
                lon: CENTER.lon - half_mile_longitude,
                ..CENTER
            },
            east_west_segment(
                FeatureKind::MajorRoad,
                "West-only arterial",
                CENTER.lat + 0.006_35,
                CENTER.lon - 0.019,
                CENTER.lon - 0.012,
            ),
        ),
        (
            "north",
            Coordinate {
                lat: CENTER.lat + half_mile_latitude,
                ..CENTER
            },
            north_south_segment(
                FeatureKind::MajorRoad,
                "North-only arterial",
                CENTER.lon + 0.004_05,
                CENTER.lat + 0.009,
                CENTER.lat + 0.013,
            ),
        ),
        (
            "south",
            Coordinate {
                lat: CENTER.lat - half_mile_latitude,
                ..CENTER
            },
            north_south_segment(
                FeatureKind::MajorRoad,
                "South-only arterial",
                CENTER.lon + 0.004_05,
                CENTER.lat - 0.013,
                CENTER.lat - 0.009,
            ),
        ),
    ];

    let mut overlap_kinds = BTreeSet::new();
    for (direction, shifted_center, extra_non_overlap_road) in shifts {
        let mut shifted_geometry = geometry.clone();
        shifted_geometry.push(extra_non_overlap_road);
        let shifted_source = source(shifted_center, shifted_geometry);
        let shifted_world = WorldGrid::from_bounds(
            shifted_source.center,
            shifted_source.bounds,
            GRID_SIZE,
            GRID_SIZE,
        )
        .expect("shifted world grid");
        let shifted =
            generate_grid(shifted_source, GRID_SIZE, GRID_SIZE).expect("shifted generated map");
        let overlap = base_world
            .intersection(shifted_world)
            .expect("half-mile windows overlap");
        let mut mapped_cells = 0usize;
        for world_y in overlap.south..=overlap.north {
            for world_x in overlap.west..=overlap.east {
                let world = WorldCell {
                    x: world_x,
                    y: world_y,
                };
                let (base_x, base_y) = base_world.local_cell(world).expect("base local cell");
                let (shifted_x, shifted_y) =
                    shifted_world.local_cell(world).expect("shifted local cell");
                let base_kind = mapped_road(base.cell(base_x, base_y));
                let shifted_kind = mapped_road(shifted.cell(shifted_x, shifted_y));
                assert_eq!(
                    base_kind, shifted_kind,
                    "mapped road changed in the {direction} overlap at global cell {world:?}"
                );
                if let Some(kind) = base_kind {
                    overlap_kinds.insert(match kind {
                        MapCell::Street => "street",
                        MapCell::Road => "road",
                        MapCell::MajorRoad => "major_road",
                        _ => unreachable!("mapped_road returned a non-road cell"),
                    });
                    mapped_cells += 1;
                }
            }
        }
        assert!(
            mapped_cells >= 20,
            "at least one E-W corridor crosses the {direction} overlap"
        );
        assert!(
            mapped_cells < usize::from(GRID_SIZE) * usize::from(GRID_SIZE) / 10,
            "the {direction} overlap keeps the global backbone sparse"
        );
    }

    assert_eq!(
        overlap_kinds,
        BTreeSet::from(["street", "road", "major_road"]),
        "the geographic backbone must retain each OSM road class"
    );
}

fn mapped_road(cell: Option<MapCell>) -> Option<MapCell> {
    cell.filter(|cell| matches!(cell, MapCell::Street | MapCell::Road | MapCell::MajorRoad))
}

fn east_west_road(kind: FeatureKind, name: &str, latitude: f64) -> Feature {
    Feature {
        kind,
        name: Some(name.to_string()),
        area: false,
        bridge: false,
        points: vec![
            Coordinate {
                lat: latitude + 0.000_08,
                lon: CENTER.lon - 0.025,
            },
            Coordinate {
                lat: latitude - 0.000_06,
                lon: CENTER.lon,
            },
            Coordinate {
                lat: latitude + 0.000_05,
                lon: CENTER.lon + 0.035,
            },
        ],
    }
}

fn north_south_road(kind: FeatureKind, name: &str, longitude: f64) -> Feature {
    north_south_segment(kind, name, longitude, CENTER.lat - 0.02, CENTER.lat + 0.02)
}

fn east_west_segment(
    kind: FeatureKind,
    name: &str,
    latitude: f64,
    west: f64,
    east: f64,
) -> Feature {
    Feature {
        kind,
        name: Some(name.to_string()),
        area: false,
        bridge: false,
        points: vec![
            Coordinate {
                lat: latitude,
                lon: west,
            },
            Coordinate {
                lat: latitude,
                lon: east,
            },
        ],
    }
}

fn north_south_segment(
    kind: FeatureKind,
    name: &str,
    longitude: f64,
    south: f64,
    north: f64,
) -> Feature {
    Feature {
        kind,
        name: Some(name.to_string()),
        area: false,
        bridge: false,
        points: vec![
            Coordinate {
                lat: south,
                lon: longitude,
            },
            Coordinate {
                lat: north,
                lon: longitude,
            },
        ],
    }
}

fn source(center: Coordinate, features: Vec<Feature>) -> MapSource {
    MapSource {
        center,
        bounds: BoundingBox::square_miles_around(center, SIDE_MILES).expect("bounds"),
        attribution: "synthetic global road alignment fixture".to_string(),
        features,
        h3: None,
    }
}
