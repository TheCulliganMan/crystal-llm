use std::collections::{BTreeSet, VecDeque};

use crystal_mapgen::{
    BoundingBox, Coordinate, Feature, FeatureKind, GeneratedGrid, MapCell, MapSource, audit_grid,
    generate_grid,
};

const GRID_SIZE: u16 = 64;
const VIEWPORT_BLOCKS: usize = 8;
const VIEWPORT_CORE_INSET: usize = 3;
const MAX_WILD_FIELD_BLOCKS: usize = 72;
const MAX_WILD_FIELD_ASPECT_RATIO: usize = 3;
const MAX_WILD_FIELD_DEPTH: usize = 3;

#[test]
fn realistic_neighborhood_has_crystal_style_composition() {
    let first = generate_grid(realistic_source(), GRID_SIZE, GRID_SIZE).expect("generate layout");
    let second = generate_grid(realistic_source(), GRID_SIZE, GRID_SIZE).expect("regenerate");

    assert_eq!(
        first.cells, second.cells,
        "the same coordinates must be deterministic"
    );
    let audit = audit_grid(&first);
    assert!(audit.passed, "layout audit failed: {:?}", audit.errors);

    let cell_count = first.cells.len();
    let path_count = count_cells(
        &first,
        &[
            MapCell::Trail,
            MapCell::Street,
            MapCell::Road,
            MapCell::MajorRoad,
        ],
    );
    assert_percent_between("path", path_count, cell_count, 3.0, 8.0);

    let tree_count = count_cells(
        &first,
        &[
            MapCell::Tree,
            MapCell::ParkTree,
            MapCell::SmallTree,
            MapCell::SmallTreeSouth,
        ],
    );
    assert_percent_between("tree", tree_count, cell_count, 18.0, 35.0);

    let flowers = count_cells(&first, &[MapCell::Flowers]);
    assert!(
        flowers > 0,
        "the layout should contain deliberate flower accents"
    );
    assert_percent_between("flower", flowers, cell_count, 0.0, 3.0);

    let boulders = count_cells(&first, &[MapCell::Boulder]);
    assert!(
        (32..=48).contains(&boulders),
        "natural areas should contain 32-48 canonical boulders in substantial formations, found {boulders}"
    );
    let rock_formations = proximity_components(&first, MapCell::Boulder, 2);
    let substantial_rock_formations = rock_formations
        .iter()
        .filter(|formation| formation.len() >= 3)
        .collect::<Vec<_>>();
    assert!(
        (6..=10).contains(&substantial_rock_formations.len()),
        "expected 6-10 distinct rock formations, found {}",
        substantial_rock_formations.len()
    );
    let grouped_rocks = substantial_rock_formations
        .iter()
        .map(|formation| formation.len())
        .sum::<usize>();
    assert!(
        grouped_rocks * 4 >= boulders * 3,
        "only {grouped_rocks}/{boulders} rocks belong to a radius-two formation"
    );
    assert!(
        count_cells(&first, &[MapCell::Bench]) >= 2,
        "the public spaces should contain canonical outdoor benches"
    );
    assert!(
        count_cells(&first, &[MapCell::TrashCan]) >= 1,
        "the public spaces should contain an interactive outdoor trash can"
    );
    assert_eq!(
        count_cells(&first, &[MapCell::Fountain]),
        1,
        "the authored public field should contain one canonical National Park fountain"
    );
    assert!(
        count_cells(&first, &[MapCell::GroundSign]) > 0,
        "the authored map should contain a verified sign prop"
    );
    assert!(
        count_cells(
            &first,
            &[
                MapCell::FenceNorthWest,
                MapCell::FenceNorth,
                MapCell::FenceNorthEast,
                MapCell::FenceWest,
                MapCell::FenceEast,
                MapCell::FenceSouthWest,
                MapCell::FenceSouth,
                MapCell::FenceSouthEast,
            ],
        ) > 0,
        "the authored map should contain a verified fence family"
    );
    let rendered_blocks = first.crystal_blocks().into_iter().collect::<BTreeSet<_>>();
    for metatile in 0x84_u16..=0x8d {
        assert!(
            rendered_blocks.contains(&metatile),
            "realistic generation must exercise new metatile ${metatile:02x}"
        );
    }

    let public_fields = components(&first, MapCell::Pitch);
    assert_eq!(
        public_fields.len(),
        1,
        "downsampled pitch fragments must become one authored public field"
    );
    assert!(
        (20..=40).contains(&public_fields[0].len()),
        "the fenced public field should be compact and playable, found {} interior blocks",
        public_fields[0].len()
    );
    assert!(
        first.labels.iter().any(|label| {
            label.text == "Neighborhood Field"
                && first.cell(label.x, label.y) == Some(MapCell::GroundSign)
        }),
        "the authored field must retain its real OSM name on a readable sign"
    );
    let ledge_runs = components_matching(&first, |cell| {
        matches!(
            cell,
            MapCell::LedgeWest | MapCell::LedgeMiddle | MapCell::LedgeEast
        )
    });
    assert_eq!(
        ledge_runs.len(),
        5,
        "a 64x64 neighborhood should contain five ledge runs arranged as authored terraces"
    );
    let mut ledge_lengths = Vec::new();
    for run in &ledge_runs {
        let width = usize::from(first.width);
        let min_x = run.iter().map(|index| index % width).min().unwrap_or(0);
        let max_x = run.iter().map(|index| index % width).max().unwrap_or(0);
        let min_y = run.iter().map(|index| index / width).min().unwrap_or(0);
        let max_y = run.iter().map(|index| index / width).max().unwrap_or(0);
        assert_eq!(min_y, max_y, "ledge components must be horizontal");
        assert_eq!(run.len(), max_x - min_x + 1, "ledge run has a gap");
        assert!((6..=10).contains(&run.len()));
        assert_eq!(
            first.cell(min_x as u16, min_y as u16),
            Some(MapCell::LedgeWest)
        );
        assert_eq!(
            first.cell(max_x as u16, min_y as u16),
            Some(MapCell::LedgeEast)
        );
        for x in min_x + 1..max_x {
            assert_eq!(
                first.cell(x as u16, min_y as u16),
                Some(MapCell::LedgeMiddle)
            );
        }
        for x in min_x..=max_x {
            assert!(matches!(
                first.cell(x as u16, (min_y - 1) as u16),
                Some(MapCell::Lawn | MapCell::Trail)
            ));
            assert_eq!(
                first.cell(x as u16, (min_y + 1) as u16),
                Some(MapCell::Lawn)
            );
        }
        assert_eq!(
            first.cell((min_x + run.len() / 2) as u16, (min_y - 1) as u16),
            Some(MapCell::Trail),
            "the middle hop lane must have a reachable approach connector"
        );
        ledge_lengths.push(run.len());
    }
    ledge_lengths.sort_unstable();
    assert_eq!(ledge_lengths, vec![6, 7, 7, 8, 9]);
    assert_eq!(
        ledge_lengths.iter().sum::<usize>(),
        37,
        "the relief system should read as a substantial authored feature"
    );
    let compound_pairs = ledge_runs
        .iter()
        .enumerate()
        .filter(|(index, first_run)| {
            ledge_runs.iter().skip(index + 1).any(|second_run| {
                let width = usize::from(first.width);
                let first_min_x = first_run.iter().map(|cell| cell % width).min().unwrap_or(0);
                let first_max_x = first_run.iter().map(|cell| cell % width).max().unwrap_or(0);
                let first_y = first_run.iter().map(|cell| cell / width).min().unwrap_or(0);
                let second_min_x = second_run
                    .iter()
                    .map(|cell| cell % width)
                    .min()
                    .unwrap_or(0);
                let second_max_x = second_run
                    .iter()
                    .map(|cell| cell % width)
                    .max()
                    .unwrap_or(0);
                let second_y = second_run
                    .iter()
                    .map(|cell| cell / width)
                    .min()
                    .unwrap_or(0);
                (3..=5).contains(&first_y.abs_diff(second_y))
                    && (1..=3).contains(&first_min_x.abs_diff(second_min_x))
                    && first_min_x <= second_max_x
                    && second_min_x <= first_max_x
            })
        })
        .count();
    assert!(
        compound_pairs >= 2,
        "expected at least two offset, stacked terrace pairs; found {compound_pairs}"
    );

    let cliff_sections = components_matching(&first, |cell| {
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
    });
    assert_eq!(
        cliff_sections.len(),
        2,
        "the neighborhood should contain both an expanded contour and a stepped T plateau"
    );
    assert_eq!(
        cliff_sections.iter().map(Vec::len).sum::<usize>(),
        55,
        "the two large canonical contour sections should occupy 55 blocks"
    );
    assert_eq!(
        count_cells(&first, &[MapCell::CliffStairs]),
        2,
        "each large plateau must have one real climbable stair tile"
    );
    assert_eq!(
        count_cells(
            &first,
            &[MapCell::CliffInnerSouthWest, MapCell::CliffInnerSouthEast,],
        ),
        2,
        "the stepped plateau must use both canonical inner contour transitions"
    );

    let park_components = components(&first, MapCell::Park);
    let park_cells = park_components.iter().map(Vec::len).sum::<usize>();
    assert!(
        (230..=300).contains(&park_cells),
        "tall grass should cover 5.6%-7.3% of a 64x64 neighborhood, found {park_cells} cells"
    );
    let small_wild_patches = park_components
        .iter()
        .filter(|component| (3..=5).contains(&component.len()))
        .collect::<Vec<_>>();
    assert!(
        small_wild_patches.len() >= 6,
        "expected at least six genuinely small tall-grass accents, found {}",
        small_wild_patches.len()
    );
    let medium_wild_patches = park_components
        .iter()
        .filter(|component| (6..=10).contains(&component.len()))
        .collect::<Vec<_>>();
    assert!(
        medium_wild_patches.len() >= 6,
        "expected at least six medium hooks, commas, zigzags, or blobs, found {}",
        medium_wild_patches.len()
    );
    let compact_wild_patches = small_wild_patches
        .into_iter()
        .chain(medium_wild_patches)
        .collect::<Vec<_>>();
    assert!(
        compact_wild_patches.iter().all(|component| {
            let xs = component
                .iter()
                .map(|index| index % usize::from(first.width));
            let ys = component
                .iter()
                .map(|index| index / usize::from(first.width));
            let width = xs.clone().max().unwrap_or(0) - xs.min().unwrap_or(0) + 1;
            let height = ys.clone().max().unwrap_or(0) - ys.min().unwrap_or(0) + 1;
            component.len() < width * height
        }),
        "compact tall-grass accents must use irregular silhouettes"
    );
    let shape_signatures = compact_wild_patches
        .iter()
        .map(|component| normalized_component_signature(&first, component))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        shape_signatures.len() >= 5,
        "expected at least five visibly distinct compact grass silhouettes, found {}",
        shape_signatures.len()
    );

    let wild_fields = park_components
        .into_iter()
        .filter(|component| component.len() >= 20)
        .collect::<Vec<_>>();
    assert!(
        (2..=4).contains(&wild_fields.len()),
        "expected 2-4 substantive wild fields, found {}",
        wild_fields.len()
    );
    for field in &wild_fields {
        let width = usize::from(first.width);
        let members = field
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let min_x = field.iter().map(|index| index % width).min().unwrap_or(0);
        let max_x = field.iter().map(|index| index % width).max().unwrap_or(0);
        let min_y = field.iter().map(|index| index / width).min().unwrap_or(0);
        let max_y = field.iter().map(|index| index / width).max().unwrap_or(0);
        let edge_has_bite = [
            (min_x..=max_x).any(|x| !members.contains(&(min_y * width + x))),
            (min_x..=max_x).any(|x| !members.contains(&(max_y * width + x))),
            (min_y..=max_y).any(|y| !members.contains(&(y * width + min_x))),
            (min_y..=max_y).any(|y| !members.contains(&(y * width + max_x))),
        ]
        .into_iter()
        .filter(|has_bite| *has_bite)
        .count();
        assert!(
            edge_has_bite >= 3,
            "substantive wild field at ({min_x},{min_y}) still has a rectangular outer silhouette"
        );
    }
    let park_depth = max_interior_depth(&first, MapCell::Park);
    assert!(
        park_depth <= MAX_WILD_FIELD_DEPTH,
        "wild grass is {park_depth} blocks deep; fields must expose a clear edge within {MAX_WILD_FIELD_DEPTH} blocks"
    );

    let tree_depth = max_interior_depth(&first, MapCell::Tree);
    assert!(
        tree_depth <= 4,
        "dense forest is {tree_depth} blocks deep; trees must form walls and belts, not carpets"
    );
    let longest_interior_canopy_bar = (2..first.height.saturating_sub(2))
        .map(|y| {
            let mut longest = 0;
            let mut current = 0;
            for x in 2..first.width.saturating_sub(2) {
                if matches!(first.cell(x, y), Some(MapCell::Tree | MapCell::ParkTree)) {
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
    assert!(
        longest_interior_canopy_bar <= 12,
        "dense canopy contains a {longest_interior_canopy_bar}-block horizontal bar; interior groves must have irregular, broken silhouettes"
    );

    let wild_shape_issues = wild_field_shape_issues(&first, &wild_fields);
    let viewport_audit = traversal_viewport_variety(&first);
    assert!(
        wild_shape_issues.is_empty() && viewport_audit.issues.is_empty(),
        "Crystal-scale composition gates failed:\n  wild fields: {wild_shape_issues:?}\n  sparse traversal viewports: {}/{}; first failures: {:?}",
        viewport_audit.issues.len(),
        viewport_audit.examined,
        viewport_audit
            .issues
            .iter()
            .take(12)
            .map(ViewportVarietyIssue::summary)
            .collect::<Vec<_>>()
    );

    let path_components = components_matching(&first, is_path);
    let path_total = path_components.iter().map(Vec::len).sum::<usize>();
    let principal_path = path_components.iter().map(Vec::len).max().unwrap_or(0);
    let path_shapes = path_components
        .iter()
        .map(|component| {
            let xs = component
                .iter()
                .map(|index| index % usize::from(first.width));
            let ys = component
                .iter()
                .map(|index| index / usize::from(first.width));
            (
                component.len(),
                xs.clone().min().unwrap_or(0),
                xs.max().unwrap_or(0),
                ys.clone().min().unwrap_or(0),
                ys.max().unwrap_or(0),
            )
        })
        .collect::<Vec<_>>();
    assert!(
        path_total > 0 && principal_path * 100 >= path_total * 90,
        "principal path contains {principal_path}/{path_total} path blocks; expected at least 90%; components {path_shapes:?}"
    );

    let houses = components(&first, MapCell::Building);
    assert!(
        (8..=12).contains(&houses.len()),
        "expected 8-12 houses, found {}",
        houses.len()
    );
    let reachable = reachable_walkable(&first);
    for (index, cell) in first.cells.iter().enumerate() {
        if *cell != MapCell::CliffStairs {
            continue;
        }
        let x = index % usize::from(first.width);
        let y = index / usize::from(first.width);
        assert!(
            reachable[index],
            "plateau staircase at ({x},{y}) is unreachable"
        );
        assert_eq!(
            first.cell(x as u16, (y + 1) as u16),
            Some(MapCell::Trail),
            "plateau staircase at ({x},{y}) needs a visible path immediately south"
        );
    }
    for house in &houses {
        assert_two_by_two(&first, house);
        assert_clear_connected_frontage(&first, house, &reachable);
    }
    let blocks = first.crystal_blocks();
    let mut house_frontage_styles = std::collections::BTreeSet::new();
    for house in &houses {
        let min_x = house
            .iter()
            .map(|index| index % usize::from(first.width))
            .min()
            .expect("house x");
        let min_y = house
            .iter()
            .map(|index| index / usize::from(first.width))
            .min()
            .expect("house y");
        let frontage = [
            blocks[(min_y + 1) * usize::from(first.width) + min_x],
            blocks[(min_y + 1) * usize::from(first.width) + min_x + 1],
        ];
        assert!(
            matches!(frontage, [0x16, 0x1e] | [0x99, 0x9a]),
            "ordinary houses must use either the canonical modern or Ecruteak traditional frontage with a real southwest-block door: {frontage:?}"
        );
        house_frontage_styles.insert(frontage);
    }
    assert!(
        house_frontage_styles.len() >= 2,
        "the neighborhood must visibly mix modern and Ecruteak-style residential frontages"
    );
    assert_eq!(
        count_cells(
            &first,
            &[
                MapCell::PokecenterNorthWest,
                MapCell::PokecenterNorthEast,
                MapCell::PokecenterSouthWest,
                MapCell::PokecenterSouthEast,
            ],
        ),
        4,
        "the settlement needs one complete canonical Pokemon Center"
    );
    assert_eq!(
        count_cells(
            &first,
            &[
                MapCell::MartNorthWest,
                MapCell::MartNorthEast,
                MapCell::MartSouthWest,
                MapCell::MartSouthEast,
            ],
        ),
        4,
        "the settlement needs one complete canonical Pokemon Mart"
    );

    // The source lake occupies the northeast. Check its safely interior cells,
    // where raster edge rounding and an intentional shore access cannot apply.
    // Transport planning must route around this source water mask.
    for y in 3..=11 {
        for x in 43..=59 {
            assert!(
                matches!(
                    first.cell(x, y),
                    Some(
                        MapCell::Water
                            | MapCell::WaterAccessEast
                            | MapCell::WaterAccessWest
                            | MapCell::WaterAccessSouth
                    )
                ),
                "source-water interior at ({x}, {y}) was overwritten by {:?}",
                first.cell(x, y)
            );
        }
    }
}

fn realistic_source() -> MapSource {
    let mut features = vec![
        area(
            FeatureKind::Water,
            Some("North Lake"),
            0.64,
            0.78,
            0.97,
            0.98,
        ),
        area(
            FeatureKind::Park,
            Some("West Preserve"),
            0.04,
            0.04,
            0.27,
            0.27,
        ),
        area(
            FeatureKind::Park,
            Some("Central Commons"),
            0.37,
            0.04,
            0.61,
            0.27,
        ),
        area(
            FeatureKind::Park,
            Some("East Meadow"),
            0.72,
            0.04,
            0.96,
            0.27,
        ),
        area(
            FeatureKind::Pitch,
            Some("Neighborhood Field"),
            0.08,
            0.68,
            0.18,
            0.75,
        ),
        line(
            FeatureKind::MajorRoad,
            Some("Main Street"),
            &[(0.01, 0.50), (0.35, 0.51), (0.67, 0.49), (0.99, 0.50)],
        ),
        line(
            FeatureKind::Street,
            Some("Lake Avenue"),
            &[(0.08, 0.47), (0.48, 0.48), (0.92, 0.47)],
        ),
        line(
            FeatureKind::Street,
            Some("Park Avenue"),
            &[(0.12, 0.54), (0.52, 0.53), (0.89, 0.54)],
        ),
    ];

    // A dense real-world block is intentionally compressed by the generator
    // into a small number of readable Crystal houses along the main frontage.
    for (index, lon) in [0.05, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.75, 0.85, 0.95]
        .into_iter()
        .enumerate()
    {
        features.push(area(
            FeatureKind::Building,
            Some(&format!("Parcel {index}")),
            lon - 0.018,
            0.55,
            lon + 0.018,
            0.58,
        ));
    }

    MapSource {
        center: Coordinate { lat: 0.5, lon: 0.5 },
        bounds: BoundingBox {
            south: 0.0,
            west: 0.0,
            north: 1.0,
            east: 1.0,
        },
        attribution: "synthetic OpenStreetMap-style fixture".to_string(),
        features,
        h3: None,
    }
}

fn area(
    kind: FeatureKind,
    name: Option<&str>,
    west: f64,
    south: f64,
    east: f64,
    north: f64,
) -> Feature {
    Feature {
        kind,
        name: name.map(str::to_string),
        area: true,
        bridge: false,
        points: vec![
            Coordinate {
                lat: south,
                lon: west,
            },
            Coordinate {
                lat: north,
                lon: west,
            },
            Coordinate {
                lat: north,
                lon: east,
            },
            Coordinate {
                lat: south,
                lon: east,
            },
            Coordinate {
                lat: south,
                lon: west,
            },
        ],
    }
}

fn line(kind: FeatureKind, name: Option<&str>, points: &[(f64, f64)]) -> Feature {
    Feature {
        kind,
        name: name.map(str::to_string),
        area: false,
        bridge: false,
        points: points
            .iter()
            .map(|&(lon, lat)| Coordinate { lat, lon })
            .collect(),
    }
}

fn count_cells(grid: &GeneratedGrid, wanted: &[MapCell]) -> usize {
    grid.cells
        .iter()
        .filter(|cell| wanted.contains(cell))
        .count()
}

fn assert_percent_between(label: &str, count: usize, total: usize, minimum: f64, maximum: f64) {
    let percent = count as f64 / total as f64 * 100.0;
    assert!(
        percent >= minimum && percent <= maximum,
        "{label} coverage {percent:.1}% is outside {minimum:.1}%..={maximum:.1}%"
    );
}

fn components(grid: &GeneratedGrid, wanted: MapCell) -> Vec<Vec<usize>> {
    components_matching(grid, |cell| cell == wanted)
}

fn normalized_component_signature(
    grid: &GeneratedGrid,
    component: &[usize],
) -> Vec<(usize, usize)> {
    let width = usize::from(grid.width);
    let min_x = component
        .iter()
        .map(|index| index % width)
        .min()
        .unwrap_or(0);
    let min_y = component
        .iter()
        .map(|index| index / width)
        .min()
        .unwrap_or(0);
    let mut signature = component
        .iter()
        .map(|index| (index % width - min_x, index / width - min_y))
        .collect::<Vec<_>>();
    signature.sort_unstable();
    signature
}

fn components_matching(
    grid: &GeneratedGrid,
    wanted: impl Fn(MapCell) -> bool + Copy,
) -> Vec<Vec<usize>> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut visited = vec![false; grid.cells.len()];
    let mut result = Vec::new();
    for start in 0..grid.cells.len() {
        if visited[start] || !wanted(grid.cells[start]) {
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
                if !visited[next] && wanted(grid.cells[next]) {
                    visited[next] = true;
                    frontier.push(next);
                }
            }
        }
        result.push(component);
    }
    result
}

fn proximity_components(grid: &GeneratedGrid, wanted: MapCell, radius: usize) -> Vec<Vec<usize>> {
    let width = usize::from(grid.width);
    let cells = grid
        .cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| (*cell == wanted).then_some(index))
        .collect::<Vec<_>>();
    let mut unseen = vec![true; cells.len()];
    let mut result = Vec::new();
    for start in 0..cells.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut frontier = vec![start];
        while let Some(position) = frontier.pop() {
            let index = cells[position];
            component.push(index);
            let x = index % width;
            let y = index / width;
            for next in 0..cells.len() {
                let next_x = cells[next] % width;
                let next_y = cells[next] / width;
                if unseen[next] && x.abs_diff(next_x) <= radius && y.abs_diff(next_y) <= radius {
                    unseen[next] = false;
                    frontier.push(next);
                }
            }
        }
        result.push(component);
    }
    result
}

fn max_interior_depth(grid: &GeneratedGrid, wanted: MapCell) -> usize {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut depth = vec![usize::MAX; grid.cells.len()];
    let mut frontier = VecDeque::new();
    for index in 0..grid.cells.len() {
        if grid.cells[index] != wanted {
            continue;
        }
        let x = index % width;
        let y = index / width;
        let is_boundary = x == 0
            || x + 1 == width
            || y == 0
            || y + 1 == height
            || neighbors(width, height, index)
                .into_iter()
                .any(|next| grid.cells[next] != wanted);
        if is_boundary {
            depth[index] = 1;
            frontier.push_back(index);
        }
    }
    while let Some(index) = frontier.pop_front() {
        let next_depth = depth[index] + 1;
        for next in neighbors(width, height, index) {
            if grid.cells[next] == wanted && depth[next] > next_depth {
                depth[next] = next_depth;
                frontier.push_back(next);
            }
        }
    }
    grid.cells
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| (*cell == wanted).then_some(depth[index]))
        .max()
        .unwrap_or(0)
}

fn wild_field_shape_issues(grid: &GeneratedGrid, fields: &[Vec<usize>]) -> Vec<String> {
    let width = usize::from(grid.width);
    let mut issues = Vec::new();
    for (field_index, field) in fields.iter().enumerate() {
        let min_x = field.iter().map(|index| index % width).min().unwrap_or(0);
        let max_x = field.iter().map(|index| index % width).max().unwrap_or(0);
        let min_y = field.iter().map(|index| index / width).min().unwrap_or(0);
        let max_y = field.iter().map(|index| index / width).max().unwrap_or(0);
        let field_width = max_x - min_x + 1;
        let field_height = max_y - min_y + 1;
        let long_side = field_width.max(field_height);
        let short_side = field_width.min(field_height);
        let depth = component_max_interior_depth(grid, field);
        if field.len() > MAX_WILD_FIELD_BLOCKS {
            issues.push(format!(
                "field {} at ({min_x},{min_y}) is {} blocks (max {MAX_WILD_FIELD_BLOCKS})",
                field_index + 1,
                field.len()
            ));
        }
        if long_side > short_side * MAX_WILD_FIELD_ASPECT_RATIO {
            issues.push(format!(
                "field {} at ({min_x},{min_y}) spans {field_width}x{field_height} blocks (max aspect {MAX_WILD_FIELD_ASPECT_RATIO}:1)",
                field_index + 1
            ));
        }
        if depth > MAX_WILD_FIELD_DEPTH {
            issues.push(format!(
                "field {} at ({min_x},{min_y}) has interior depth {depth} (max {MAX_WILD_FIELD_DEPTH})",
                field_index + 1
            ));
        }
    }
    issues
}

fn component_max_interior_depth(grid: &GeneratedGrid, component: &[usize]) -> usize {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let mut member = vec![false; grid.cells.len()];
    for &index in component {
        member[index] = true;
    }
    let mut depth = vec![usize::MAX; grid.cells.len()];
    let mut frontier = VecDeque::new();
    for &index in component {
        let x = index % width;
        let y = index / width;
        let is_boundary = x == 0
            || x + 1 == width
            || y == 0
            || y + 1 == height
            || neighbors(width, height, index)
                .into_iter()
                .any(|next| !member[next]);
        if is_boundary {
            depth[index] = 1;
            frontier.push_back(index);
        }
    }
    while let Some(index) = frontier.pop_front() {
        let next_depth = depth[index] + 1;
        for next in neighbors(width, height, index) {
            if member[next] && depth[next] > next_depth {
                depth[next] = next_depth;
                frontier.push_back(next);
            }
        }
    }
    component
        .iter()
        .map(|&index| depth[index])
        .max()
        .unwrap_or(0)
}

#[derive(Debug)]
struct ViewportVarietyAudit {
    examined: usize,
    issues: Vec<ViewportVarietyIssue>,
}

#[derive(Debug)]
struct ViewportVarietyIssue {
    origin: (usize, usize),
    central_path_blocks: usize,
    visual_families: Vec<&'static str>,
}

impl ViewportVarietyIssue {
    fn summary(&self) -> String {
        format!(
            "origin {:?}, {} central path blocks, families {:?}",
            self.origin, self.central_path_blocks, self.visual_families
        )
    }
}

fn traversal_viewport_variety(grid: &GeneratedGrid) -> ViewportVarietyAudit {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    assert!(
        width >= VIEWPORT_BLOCKS && height >= VIEWPORT_BLOCKS,
        "quality fixture must be at least one viewport"
    );
    let mut examined = 0;
    let mut issues = Vec::new();
    for top in 0..=height - VIEWPORT_BLOCKS {
        for left in 0..=width - VIEWPORT_BLOCKS {
            let central_path_blocks = (top + VIEWPORT_CORE_INSET
                ..top + VIEWPORT_BLOCKS - VIEWPORT_CORE_INSET)
                .flat_map(|y| {
                    (left + VIEWPORT_CORE_INSET..left + VIEWPORT_BLOCKS - VIEWPORT_CORE_INSET)
                        .map(move |x| y * width + x)
                })
                .filter(|&index| is_path(grid.cells[index]))
                .count();
            if central_path_blocks == 0 {
                continue;
            }
            examined += 1;
            let mut family_counts = [0_usize; 15];
            for y in top..top + VIEWPORT_BLOCKS {
                for x in left..left + VIEWPORT_BLOCKS {
                    let family = match grid.cells[y * width + x] {
                        MapCell::Building
                        | MapCell::PokecenterNorthWest
                        | MapCell::PokecenterNorthEast
                        | MapCell::PokecenterSouthWest
                        | MapCell::PokecenterSouthEast
                        | MapCell::MartNorthWest
                        | MapCell::MartNorthEast
                        | MapCell::MartSouthWest
                        | MapCell::MartSouthEast => Some(0),
                        MapCell::Tree | MapCell::ParkTree => Some(1),
                        MapCell::SmallTree | MapCell::SmallTreeSouth => Some(2),
                        MapCell::Flowers => Some(3),
                        MapCell::Boulder | MapCell::IceBoulder => Some(4),
                        MapCell::GroundSign => Some(5),
                        MapCell::Bench | MapCell::TrashCan | MapCell::Fountain => Some(6),
                        MapCell::FenceNorthWest
                        | MapCell::FenceNorth
                        | MapCell::FenceNorthEast
                        | MapCell::FenceWest
                        | MapCell::FenceEast
                        | MapCell::FenceSouthWest
                        | MapCell::FenceSouth
                        | MapCell::FenceSouthEast => Some(7),
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
                        | MapCell::CliffStairs => Some(8),
                        MapCell::Water
                        | MapCell::WaterAccessEast
                        | MapCell::WaterAccessWest
                        | MapCell::WaterAccessSouth => Some(9),
                        MapCell::Pitch => Some(10),
                        MapCell::Park => Some(11),
                        MapCell::Clearing
                        | MapCell::Lawn
                        | MapCell::IceFloor
                        | MapCell::RockFloor => Some(12),
                        MapCell::Grass => Some(13),
                        MapCell::Rail => Some(14),
                        MapCell::H3Void
                        | MapCell::Trail
                        | MapCell::Street
                        | MapCell::Road
                        | MapCell::MajorRoad => None,
                    };
                    if let Some(family) = family {
                        family_counts[family] += 1;
                    }
                }
            }
            let family_definitions = [
                ("settlement", 1),
                ("dense_tree_edge", 4),
                ("small_tree", 1),
                ("flowers", 1),
                ("rock", 1),
                ("sign", 1),
                ("amenity", 1),
                ("fence", 2),
                ("relief", 2),
                ("water", 4),
                ("public_field", 6),
                ("wild_field", 6),
                ("safe_clearing", 8),
                ("grassland", 16),
                ("rail", 2),
            ];
            let visual_families = family_definitions
                .iter()
                .zip(family_counts)
                .filter_map(|(&(name, minimum), count)| (count >= minimum).then_some(name))
                .collect::<Vec<_>>();
            if visual_families.len() < 2 {
                issues.push(ViewportVarietyIssue {
                    origin: (left, top),
                    central_path_blocks,
                    visual_families,
                });
            }
        }
    }
    assert!(examined > 0, "fixture produced no traversal viewports");
    ViewportVarietyAudit { examined, issues }
}

fn reachable_walkable(grid: &GeneratedGrid) -> Vec<bool> {
    let width = usize::from(grid.width);
    let height = usize::from(grid.height);
    let (start_x, start_y) = grid.home_cell();
    let start = usize::from(start_y) * width + usize::from(start_x);
    let mut reached = vec![false; grid.cells.len()];
    if !is_walkable(grid.cells[start]) {
        return reached;
    }
    let mut frontier = VecDeque::from([start]);
    reached[start] = true;
    while let Some(index) = frontier.pop_front() {
        for next in neighbors(width, height, index) {
            if !reached[next] && is_walkable(grid.cells[next]) {
                reached[next] = true;
                frontier.push_back(next);
            }
        }
    }
    reached
}

fn neighbors(width: usize, height: usize, index: usize) -> Vec<usize> {
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

fn is_path(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
    )
}

fn is_walkable(cell: MapCell) -> bool {
    !matches!(
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
            | MapCell::Tree
            | MapCell::ParkTree
            | MapCell::SmallTree
            | MapCell::SmallTreeSouth
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
            | MapCell::Water
            | MapCell::WaterAccessEast
            | MapCell::WaterAccessWest
            | MapCell::WaterAccessSouth
    )
}

fn assert_clear_connected_frontage(grid: &GeneratedGrid, component: &[usize], reachable: &[bool]) {
    let width = usize::from(grid.width);
    let min_x = component
        .iter()
        .map(|index| index % width)
        .min()
        .expect("house x");
    let max_y = component
        .iter()
        .map(|index| index / width)
        .max()
        .expect("house y");
    let door_x = min_x;
    let door_y = max_y + 1;
    assert!(
        door_x < width && door_y < usize::from(grid.height),
        "house at ({min_x}, {}) has no south frontage",
        max_y - 1
    );
    let door = door_y * width + door_x;
    assert!(
        matches!(
            grid.cells[door],
            MapCell::Grass
                | MapCell::Lawn
                | MapCell::Trail
                | MapCell::Street
                | MapCell::Road
                | MapCell::MajorRoad
        ),
        "house door-front at ({door_x}, {door_y}) is obstructed by {:?}",
        grid.cells[door]
    );
    assert!(
        is_path(grid.cells[door])
            || neighbors(width, usize::from(grid.height), door)
                .into_iter()
                .any(|next| is_path(grid.cells[next])),
        "house door-front at ({door_x}, {door_y}) has no path connection"
    );
    assert!(
        reachable[door],
        "house door-front at ({door_x}, {door_y}) is unreachable from spawn"
    );
}

fn assert_two_by_two(grid: &GeneratedGrid, component: &[usize]) {
    assert_eq!(
        component.len(),
        4,
        "house footprint must contain four blocks"
    );
    let width = usize::from(grid.width);
    let xs = component
        .iter()
        .map(|index| index % width)
        .collect::<Vec<_>>();
    let ys = component
        .iter()
        .map(|index| index / width)
        .collect::<Vec<_>>();
    let min_x = *xs.iter().min().expect("house x");
    let max_x = *xs.iter().max().expect("house x");
    let min_y = *ys.iter().min().expect("house y");
    let max_y = *ys.iter().max().expect("house y");
    assert_eq!((max_x - min_x, max_y - min_y), (1, 1));
}
