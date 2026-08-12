//! Resolves authored mountain shelves into relative elevation tiers.
//!
//! Crystal's source drawings describe a face between two regions. A face on
//! another raised region is therefore another level, not the same absolute
//! height. Water and ordinary ground never participate in this solver.

use crate::profile::{CellShape, LedgeFace, MOUNTAIN_CLIFF_HEIGHT, SolidKind};

pub(crate) fn resolve_authored_mountain_tiers(
    shapes: &mut [CellShape],
    width: usize,
    height: usize,
) {
    resolve_mountain_tiers(shapes, width, height);
}

pub(crate) fn resolve_mountain_tiers(shapes: &mut [CellShape], width: usize, height: usize) {
    if width == 0 || height == 0 || shapes.len() != width * height {
        return;
    }
    let mountain: Vec<_> = shapes.iter().copied().map(is_mountain).collect();
    let mut parent: Vec<_> = (0..shapes.len()).collect();
    let mut face_edges = Vec::new();
    for row in 0..height {
        for column in 0..width {
            let index = row * width + column;
            if !mountain[index] {
                continue;
            }
            for (next_column, next_row) in [(column + 1, row), (column, row + 1)] {
                if next_column >= width || next_row >= height {
                    continue;
                }
                let next = next_row * width + next_column;
                if !mountain[next] {
                    continue;
                }
                match directed_face_relation(
                    shapes[index],
                    shapes[next],
                    column,
                    row,
                    next_column,
                    next_row,
                ) {
                    Some(true) => face_edges.push((next, index)),
                    Some(false) => face_edges.push((index, next)),
                    None => union(&mut parent, index, next),
                }
            }
        }
    }

    let components: Vec<_> = (0..shapes.len())
        .map(|index| find(&mut parent, index))
        .collect();
    // Corner drawings can contribute mutually directed constraints. Collapse
    // those cycles first: they describe one continuous shelf datum, not an
    // impossible staircase. The remaining component graph is acyclic, so its
    // longest path gives the exact number of authored mountain levels.
    let component_count = components
        .iter()
        .copied()
        .max()
        .map_or(0, |value| value + 1);
    let mut graph = vec![Vec::new(); component_count];
    let mut reverse = vec![Vec::new(); component_count];
    for &(lower_cell, upper_cell) in &face_edges {
        let lower = components[lower_cell];
        let upper = components[upper_cell];
        if lower != upper {
            graph[lower].push(upper);
            reverse[upper].push(lower);
        }
    }
    let datums = strongly_connected_datums(&graph, &reverse);
    let datum_count = datums.iter().copied().max().map_or(0, |value| value + 1);
    let mut datum_edges = Vec::new();
    for &(lower_cell, upper_cell) in &face_edges {
        let lower = datums[components[lower_cell]];
        let upper = datums[components[upper_cell]];
        if lower != upper && !datum_edges.contains(&(lower, upper)) {
            datum_edges.push((lower, upper));
        }
    }
    let datum_tiers = exact_datum_tiers(datum_count, &datum_edges);
    let tiers: Vec<_> = components
        .iter()
        .zip(&mountain)
        .map(|(component, mountain)| {
            mountain
                .then_some(datum_tiers[datums[*component]])
                .unwrap_or(0)
        })
        .collect();

    for (shape, tier) in shapes.iter_mut().zip(tiers) {
        if tier == 0 {
            continue;
        }
        let height = f32::from(tier) * MOUNTAIN_CLIFF_HEIGHT;
        match shape {
            CellShape::RaisedTop {
                height: authored_height,
                solid: SolidKind::Bank,
            }
            | CellShape::LedgeBand {
                height: authored_height,
                ..
            } if (*authored_height - MOUNTAIN_CLIFF_HEIGHT).abs() < f32::EPSILON => {
                *authored_height = height;
            }
            _ => {}
        }
    }
}

/// Solve every authored face as an exact one-level transition. A longest-path
/// solver only treated faces as lower bounds, so alternate paths could make a
/// neighboring shelf jump two or more levels. Contradictory drawings are
/// deliberately conservative: that connected group remains on one level
/// rather than synthesizing topology absent from Crystal's map art.
fn exact_datum_tiers(datum_count: usize, edges: &[(usize, usize)]) -> Vec<u8> {
    let mut adjacency = vec![Vec::new(); datum_count];
    for &(lower, upper) in edges {
        adjacency[lower].push((upper, 1_i16));
        adjacency[upper].push((lower, -1_i16));
    }
    let mut levels = vec![None; datum_count];
    let mut tiers = vec![1_u8; datum_count];
    for start in 0..datum_count {
        if levels[start].is_some() {
            continue;
        }
        levels[start] = Some(0);
        let mut queue = std::collections::VecDeque::from([start]);
        let mut members = Vec::new();
        let mut consistent = true;
        while let Some(node) = queue.pop_front() {
            members.push(node);
            let level = levels[node].expect("queued datum has a level");
            for &(next, delta) in &adjacency[node] {
                let expected = level + delta;
                match levels[next] {
                    Some(actual) if actual != expected => consistent = false,
                    Some(_) => {}
                    None => {
                        levels[next] = Some(expected);
                        queue.push_back(next);
                    }
                }
            }
        }
        if !consistent {
            continue;
        }
        let minimum = members
            .iter()
            .filter_map(|&datum| levels[datum])
            .min()
            .unwrap_or(0);
        for datum in members {
            let normalized = levels[datum].unwrap_or(minimum) - minimum + 1;
            tiers[datum] = u8::try_from(normalized).unwrap_or(u8::MAX);
        }
    }
    tiers
}

fn strongly_connected_datums(graph: &[Vec<usize>], reverse: &[Vec<usize>]) -> Vec<usize> {
    fn visit(node: usize, graph: &[Vec<usize>], seen: &mut [bool], order: &mut Vec<usize>) {
        if seen[node] {
            return;
        }
        seen[node] = true;
        for &next in &graph[node] {
            visit(next, graph, seen, order);
        }
        order.push(node);
    }

    fn assign(node: usize, graph: &[Vec<usize>], datum: usize, datums: &mut [usize]) {
        if datums[node] != usize::MAX {
            return;
        }
        datums[node] = datum;
        for &next in &graph[node] {
            assign(next, graph, datum, datums);
        }
    }

    let mut seen = vec![false; graph.len()];
    let mut order = Vec::with_capacity(graph.len());
    for node in 0..graph.len() {
        visit(node, graph, &mut seen, &mut order);
    }
    let mut datums = vec![usize::MAX; graph.len()];
    let mut datum = 0;
    for node in order.into_iter().rev() {
        if datums[node] == usize::MAX {
            assign(node, reverse, datum, &mut datums);
            datum += 1;
        }
    }
    datums
}

fn find(parent: &mut [usize], index: usize) -> usize {
    if parent[index] != index {
        parent[index] = find(parent, parent[index]);
    }
    parent[index]
}

fn union(parent: &mut [usize], first: usize, second: usize) {
    let first = find(parent, first);
    let second = find(parent, second);
    if first != second {
        parent[second] = first;
    }
}

fn is_mountain(shape: CellShape) -> bool {
    match shape {
        CellShape::RaisedTop {
            height,
            solid: SolidKind::Bank,
        }
        | CellShape::LedgeBand { height, .. } => {
            (height - MOUNTAIN_CLIFF_HEIGHT).abs() < f32::EPSILON
        }
        _ => false,
    }
}

/// Identifies which of two adjacent cells is the upper side of an authored
/// back-to-front cliff seam. Crystal's south-facing ledge course establishes
/// the terrace step. West/east courses close that terrace's sides but do not
/// independently invent a new elevation.
fn directed_face_relation(
    first: CellShape,
    second: CellShape,
    first_column: usize,
    first_row: usize,
    second_column: usize,
    second_row: usize,
) -> Option<bool> {
    if bottom_face_points_to(first, first_column, first_row, second_column, second_row).is_some() {
        return Some(true);
    }
    if bottom_face_points_to(second, second_column, second_row, first_column, first_row).is_some() {
        return Some(false);
    }
    None
}

fn bottom_face_points_to(
    shape: CellShape,
    column: usize,
    row: usize,
    neighbor_column: usize,
    neighbor_row: usize,
) -> Option<LedgeFace> {
    let CellShape::LedgeBand {
        face,
        band_from_top,
        band_count,
        height,
        ..
    } = shape
    else {
        return None;
    };
    if (height - MOUNTAIN_CLIFF_HEIGHT).abs() >= f32::EPSILON || band_from_top + 1 != band_count {
        return None;
    }
    let points_to_neighbor = match face {
        LedgeFace::South => neighbor_column == column && neighbor_row == row + 1,
        LedgeFace::West | LedgeFace::East => false,
    };
    points_to_neighbor.then_some(face)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top() -> CellShape {
        CellShape::RaisedTop {
            height: MOUNTAIN_CLIFF_HEIGHT,
            solid: SolidKind::Bank,
        }
    }

    #[test]
    fn identical_authored_shelves_use_the_same_tier_solver_without_map_context() {
        let mut shapes = vec![top(), south_face(0), south_face(1), top()];
        resolve_authored_mountain_tiers(&mut shapes, 1, 4);
        assert_eq!(shapes[0].surface_height(8.0), 32.0);
        assert_eq!(shapes[3].surface_height(8.0), 16.0);
    }

    fn south_face(band: u8) -> CellShape {
        CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 2,
            band_from_top: band,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_CLIFF_HEIGHT,
        }
    }

    fn side_face(face: LedgeFace) -> CellShape {
        CellShape::LedgeBand {
            face,
            plane_subtile: if face == LedgeFace::West { 0 } else { 4 },
            band_from_top: 1,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_CLIFF_HEIGHT,
        }
    }

    #[test]
    fn shelf_on_shelf_resolves_to_two_levels() {
        // Upper shelf, its two-band face, then a lower shelf.
        let mut shapes = vec![top(), south_face(0), south_face(1), top()];
        resolve_mountain_tiers(&mut shapes, 1, 4);
        assert_eq!(shapes[0].surface_height(8.0), 32.0);
        assert_eq!(shapes[1].surface_height(8.0), 32.0);
        assert_eq!(shapes[2].surface_height(8.0), 32.0);
        assert_eq!(shapes[3].surface_height(8.0), 16.0);
    }

    #[test]
    fn three_authored_shelves_resolve_to_three_levels() {
        let mut shapes = vec![
            top(),
            south_face(0),
            south_face(1),
            top(),
            south_face(0),
            south_face(1),
            top(),
        ];
        resolve_mountain_tiers(&mut shapes, 1, 7);
        assert_eq!(shapes[0].surface_height(8.0), 48.0);
        assert_eq!(shapes[3].surface_height(8.0), 32.0);
        assert_eq!(shapes[6].surface_height(8.0), 16.0);
    }

    #[test]
    fn authored_stack_adds_exactly_one_level_per_connected_face() {
        assert_eq!(exact_datum_tiers(3, &[(0, 1), (1, 2)]), vec![1, 2, 3]);
    }

    #[test]
    fn contradictory_face_graph_stays_on_one_level() {
        assert_eq!(
            exact_datum_tiers(3, &[(0, 1), (1, 2), (0, 2)]),
            vec![1, 1, 1]
        );
    }

    #[test]
    fn water_does_not_raise_or_propagate_a_mountain_tier() {
        let mut shapes = vec![south_face(1), CellShape::Water];
        resolve_mountain_tiers(&mut shapes, 1, 2);
        assert_eq!(shapes[0].surface_height(8.0), 16.0);
        assert!(shapes[1].surface_height(8.0) < 0.0);
    }

    #[test]
    fn west_face_does_not_invent_a_terrace_level() {
        let mut shapes = vec![top(), side_face(LedgeFace::West)];
        resolve_mountain_tiers(&mut shapes, 2, 1);
        assert_eq!(shapes[0].surface_height(8.0), 16.0);
        assert_eq!(shapes[1].surface_height(8.0), 16.0);
    }

    #[test]
    fn east_face_does_not_invent_a_terrace_level() {
        let mut shapes = vec![side_face(LedgeFace::East), top()];
        resolve_mountain_tiers(&mut shapes, 2, 1);
        assert_eq!(shapes[0].surface_height(8.0), 16.0);
        assert_eq!(shapes[1].surface_height(8.0), 16.0);
    }
}
