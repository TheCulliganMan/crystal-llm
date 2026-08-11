//! Resolves authored mountain shelves into relative elevation tiers.
//!
//! Crystal's source drawings describe a face between two regions. A face on
//! another raised region is therefore another level, not the same absolute
//! height. Water and ordinary ground never participate in this solver.

use crate::profile::{CellShape, JUMP_LEDGE_HEIGHT, LedgeFace, MOUNTAIN_CLIFF_HEIGHT, SolidKind};

pub(crate) fn resolve_authored_mountain_tiers(
    shapes: &mut [CellShape],
    width: usize,
    height: usize,
) {
    resolve_mountain_tiers(shapes, width, height);
}

/// Extend an authored one-way ledge's raised cap into the ordinary ground
/// immediately north of it. The source block only contains the lip and a
/// short cap sample; leaving the adjacent map cells at zero makes that cap a
/// painted stripe instead of a real level change.
pub(crate) fn resolve_jump_ledge_ground(shapes: &mut [CellShape], width: usize, height: usize) {
    if width == 0 || height == 0 || shapes.len() != width * height {
        return;
    }
    for column in 0..width {
        for row in 1..height {
            let lip = shapes[row * width + column];
            let CellShape::LedgeBand {
                face: LedgeFace::South,
                height: ledge_height,
                ..
            } = lip
            else {
                continue;
            };
            if (ledge_height - JUMP_LEDGE_HEIGHT).abs() >= f32::EPSILON {
                continue;
            }
            for north_row in (0..row).rev() {
                let index = north_row * width + column;
                match shapes[index] {
                    CellShape::Flat => {
                        shapes[index] = CellShape::RaisedTop {
                            height: JUMP_LEDGE_HEIGHT,
                            solid: SolidKind::Bank,
                        };
                    }
                    CellShape::RaisedTop { height, .. }
                        if (height - JUMP_LEDGE_HEIGHT).abs() < f32::EPSILON => {}
                    _ => break,
                }
            }
        }
    }
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
    let mut datum_tiers = vec![1_u8; datum_count];
    for _ in 0..datum_count {
        let mut changed = false;
        for &(lower, upper) in &datum_edges {
            let next = datum_tiers[lower].saturating_add(1);
            if next > datum_tiers[upper] {
                datum_tiers[upper] = next;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
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
/// cliff seam. Every direction establishes a level: a west/east course is
/// still a vertical mountain face, not merely a component separator. Treating
/// only south-facing art as a height edge collapsed connected corner shelves
/// back onto the same datum.
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
        LedgeFace::West => neighbor_column + 1 == column && neighbor_row == row,
        LedgeFace::East => neighbor_column == column + 1 && neighbor_row == row,
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
    fn jump_ledge_raises_connected_plain_ground_north_of_its_lip() {
        let lip = CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: 0,
            band_count: 1,
            top_tile_index: 0x05,
            height: JUMP_LEDGE_HEIGHT,
        };
        let mut shapes = vec![CellShape::Flat, CellShape::Flat, lip, CellShape::Flat];
        resolve_jump_ledge_ground(&mut shapes, 1, 4);
        assert!(matches!(
            shapes[0],
            CellShape::RaisedTop { height, solid: SolidKind::Bank }
                if height == JUMP_LEDGE_HEIGHT
        ));
        assert!(matches!(shapes[1], CellShape::RaisedTop { .. }));
        assert_eq!(shapes[2], lip);
        assert_eq!(shapes[3], CellShape::Flat);
    }

    #[test]
    fn identical_authored_shelves_use_the_same_tier_solver_without_map_context() {
        let mut shapes = vec![top(), south_face(0), south_face(1), top()];
        resolve_authored_mountain_tiers(&mut shapes, 1, 4);
        assert_eq!(shapes[0].surface_height(8.0), 64.0);
        assert_eq!(shapes[3].surface_height(8.0), 32.0);
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
        assert_eq!(shapes[0].surface_height(8.0), 64.0);
        assert_eq!(shapes[1].surface_height(8.0), 64.0);
        assert_eq!(shapes[2].surface_height(8.0), 64.0);
        assert_eq!(shapes[3].surface_height(8.0), 32.0);
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
        assert_eq!(shapes[0].surface_height(8.0), 96.0);
        assert_eq!(shapes[3].surface_height(8.0), 64.0);
        assert_eq!(shapes[6].surface_height(8.0), 32.0);
    }

    #[test]
    fn water_does_not_raise_or_propagate_a_mountain_tier() {
        let mut shapes = vec![south_face(1), CellShape::Water];
        resolve_mountain_tiers(&mut shapes, 1, 2);
        assert_eq!(shapes[0].surface_height(8.0), 32.0);
        assert!(shapes[1].surface_height(8.0) < 0.0);
    }

    #[test]
    fn west_face_raises_the_shelf_above_its_west_neighbor() {
        let mut shapes = vec![top(), side_face(LedgeFace::West)];
        resolve_mountain_tiers(&mut shapes, 2, 1);
        assert_eq!(shapes[0].surface_height(8.0), 32.0);
        assert_eq!(shapes[1].surface_height(8.0), 64.0);
    }

    #[test]
    fn east_face_raises_the_shelf_above_its_east_neighbor() {
        let mut shapes = vec![side_face(LedgeFace::East), top()];
        resolve_mountain_tiers(&mut shapes, 2, 1);
        assert_eq!(shapes[0].surface_height(8.0), 64.0);
        assert_eq!(shapes[1].surface_height(8.0), 32.0);
    }
}
