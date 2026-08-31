use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};

use crate::{
    FeatureKind, GeneratedGrid, H3GridTransportDirectiveKind, MapCell, audit_h3_grid_seams,
    build_h3_grid_seam_profile,
};

/// Mutations applied after every independently authored H3 room exists and
/// before any room is audited, serialized, packed, or rendered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct H3BatchGridSeamFinalization {
    pub cells: usize,
    pub internal_edges: usize,
    pub geographic_samples: usize,
    pub reciprocal_raster_pairs: usize,
    pub authoritative_water_samples: usize,
    pub selected_transport_edges: usize,
    pub largest_equivalence_component: usize,
    pub largest_water_component: usize,
    pub water_reconciled_cells: usize,
    pub changed_cells: usize,
    pub cleared_artificial_trace_cells: usize,
}

pub fn finalize_h3_batch_grid_seams(
    grids: &mut [GeneratedGrid],
) -> Result<H3BatchGridSeamFinalization> {
    const MAX_PASSES: usize = 8;

    // A later pass can reveal a one-cell trace that was continuous before an
    // adjacent component was reconciled. Keep the entire operation atomic:
    // callers either receive a strictly audited fixed point or their original
    // raster cells byte-for-byte. Sources can be large, so snapshot only the
    // cell vectors this finalizer is allowed to mutate.
    let original_cells = grids
        .iter()
        .map(|grid| grid.cells.clone())
        .collect::<Vec<_>>();
    let result = (|| {
        let mut seen_states = vec![original_cells.clone()];
        let mut aggregate = None::<H3BatchGridSeamFinalization>;
        let mut converged = false;

        for pass_index in 0..MAX_PASSES {
            // Every pass independently revalidates source-water locality,
            // protected POIs, and the exact selected transport class before
            // applying any of that pass's changes.
            let pass = finalize_h3_batch_grid_seams_pass(grids)
                .with_context(|| format!("H3 seam fixed-point pass {}", pass_index + 1))?;
            let shoreline_changes = absorb_isolated_h3_shoreline_slivers(grids)?;
            if let Some(total) = &mut aggregate {
                ensure!(
                    total.cells == pass.cells
                        && total.internal_edges == pass.internal_edges
                        && total.geographic_samples == pass.geographic_samples
                        && total.reciprocal_raster_pairs == pass.reciprocal_raster_pairs
                        && total.authoritative_water_samples == pass.authoritative_water_samples
                        && total.selected_transport_edges == pass.selected_transport_edges,
                    "H3 seam topology changed during fixed-point reconciliation"
                );
                total.largest_equivalence_component = total
                    .largest_equivalence_component
                    .max(pass.largest_equivalence_component);
                total.largest_water_component = total
                    .largest_water_component
                    .max(pass.largest_water_component);
                total.water_reconciled_cells += pass.water_reconciled_cells;
                total.cleared_artificial_trace_cells += pass.cleared_artificial_trace_cells;
            } else {
                aggregate = Some(pass.clone());
            }

            if pass.changed_cells == 0 && shoreline_changes == 0 {
                converged = true;
                break;
            }

            let state = grids
                .iter()
                .map(|grid| grid.cells.clone())
                .collect::<Vec<_>>();
            if seen_states.last() == Some(&state) {
                // A pass may temporarily demote an isolated rim to Ground and
                // the topology guard restore it to Water. If their composed
                // result is unchanged from the immediately preceding state,
                // that is a stable fixed point, not an oscillation.
                converged = true;
                break;
            }
            ensure!(
                !seen_states.iter().any(|seen| seen == &state),
                "H3 seam reconciliation oscillated after pass {}",
                pass_index + 1
            );
            seen_states.push(state);
        }
        ensure!(
            converged,
            "H3 seam reconciliation did not converge within {MAX_PASSES} deterministic passes"
        );

        let profiles = grids
            .iter()
            .map(build_h3_grid_seam_profile)
            .collect::<Result<Vec<_>>>()?;
        let audit = audit_h3_grid_seams(&profiles);
        ensure!(
            audit.passed,
            "H3 seam reconciliation reached an invalid fixed point: {}",
            audit.errors.join("; ")
        );

        let mut summary = aggregate.expect("at least one fixed-point pass");
        summary.changed_cells = original_cells
            .iter()
            .zip(grids.iter())
            .map(|(original, grid)| {
                original
                    .iter()
                    .zip(&grid.cells)
                    .filter(|(before, after)| before != after)
                    .count()
            })
            .sum();
        Ok(summary)
    })();

    if result.is_err() {
        for (grid, cells) in grids.iter_mut().zip(original_cells) {
            grid.cells = cells;
        }
    }
    result
}

fn absorb_isolated_h3_shoreline_slivers(grids: &mut [GeneratedGrid]) -> Result<usize> {
    let batch_cells = grids
        .iter()
        .filter_map(|grid| grid.source.h3.as_ref().map(|plan| plan.cell.clone()))
        .collect::<BTreeSet<_>>();
    let mut seam_rims = BTreeSet::<(usize, usize)>::new();
    let profiles = grids
        .iter()
        .map(build_h3_grid_seam_profile)
        .collect::<Result<Vec<_>>>()?;
    let mut rim_by_edge = BTreeMap::<String, Vec<Vec<(usize, usize)>>>::new();
    for (grid_index, (grid, profile)) in grids.iter().zip(&profiles).enumerate() {
        let plan = grid
            .source
            .h3
            .as_ref()
            .context("shoreline sliver repair requires H3 plans")?;
        for edge in &profile.edges {
            if !batch_cells.contains(&edge.neighbor) {
                continue;
            }
            let mut edge_rims = Vec::new();
            for sample in &edge.samples {
                let (x, y) = crate::h3::h3_raster_sample_band(plan, grid, sample.coordinate)?[0];
                let rim = (
                    grid_index,
                    usize::from(y) * usize::from(grid.width) + usize::from(x),
                );
                seam_rims.insert(rim);
                edge_rims.push(rim);
            }
            rim_by_edge
                .entry(edge.edge_id.clone())
                .or_default()
                .push(edge_rims);
        }
    }
    let rim_pairs = rim_by_edge
        .into_values()
        .filter_map(|sides| match sides.as_slice() {
            [left, right] if left.len() == right.len() => Some(
                left.iter()
                    .copied()
                    .zip(right.iter().copied())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    let mut absorb = BTreeMap::<(usize, usize), MapCell>::new();
    for (grid_index, grid) in grids.iter().enumerate() {
        let width = usize::from(grid.width);
        let height = usize::from(grid.height);
        let mut seen = vec![false; grid.cells.len()];
        for start in 0..grid.cells.len() {
            if seen[start] || !is_walkable_seam_cell(grid.cells[start]) {
                continue;
            }
            let mut stack = vec![start];
            let mut component = Vec::new();
            seen[start] = true;
            while let Some(index) = stack.pop() {
                component.push(index);
                let x = index % width;
                let y = index / width;
                for neighbor in [
                    x.checked_sub(1).map(|nx| (nx, y)),
                    (x + 1 < width).then_some((x + 1, y)),
                    y.checked_sub(1).map(|ny| (x, ny)),
                    (y + 1 < height).then_some((x, y + 1)),
                ]
                .into_iter()
                .flatten()
                {
                    let next = neighbor.1 * width + neighbor.0;
                    if !seen[next] && is_walkable_seam_cell(grid.cells[next]) {
                        seen[next] = true;
                        stack.push(next);
                    }
                }
            }
            if component.len() > 3
                || component
                    .iter()
                    .any(|&index| grid.cells[index] != MapCell::Grass)
                || component
                    .iter()
                    .any(|&index| !seam_rims.contains(&(grid_index, index)))
            {
                continue;
            }
            let component_cells = component.iter().copied().collect::<BTreeSet<_>>();
            let mut touches_face_edge = false;
            let pinned_between_blockers = component.iter().all(|&index| {
                let x = index % width;
                let y = index / width;
                touches_face_edge |= x == 0 || y == 0 || x + 1 == width || y + 1 == height;
                let mut blocked = true;
                for (nx, ny) in [
                    (x.saturating_sub(1), y),
                    ((x + 1).min(width - 1), y),
                    (x, y.saturating_sub(1)),
                    (x, (y + 1).min(height - 1)),
                ] {
                    let neighbor = ny * width + nx;
                    touches_face_edge |= grid.cells[neighbor] == MapCell::H3Void;
                    blocked &= component_cells.contains(&neighbor)
                        || !is_walkable_seam_cell(grid.cells[neighbor]);
                }
                blocked
            });
            if pinned_between_blockers && touches_face_edge {
                let target = MapCell::Water;
                for index in component {
                    absorb.insert((grid_index, index), target);
                }
            }
        }
    }
    loop {
        let before = absorb.len();
        for &(left, right) in &rim_pairs {
            let target = absorb
                .get(&left)
                .copied()
                .or_else(|| absorb.get(&right).copied());
            if let Some(target) = target {
                absorb.insert(left, target);
                absorb.insert(right, target);
            }
        }
        if absorb.len() == before {
            break;
        }
    }
    let mut changed = 0;
    for ((grid_index, index), target) in absorb {
        if grids[grid_index].cells[index] != target {
            grids[grid_index].cells[index] = target;
            changed += 1;
        }
    }
    Ok(changed)
}

fn finalize_h3_batch_grid_seams_pass(
    grids: &mut [GeneratedGrid],
) -> Result<H3BatchGridSeamFinalization> {
    let cells = grids
        .iter()
        .enumerate()
        .map(|(index, grid)| {
            let cell = grid
                .source
                .h3
                .as_ref()
                .context("batch grid seam finalization requires H3 plans")?
                .cell
                .clone();
            Ok((cell, index))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    ensure!(
        cells.len() == grids.len(),
        "batch grid seam finalization received duplicate H3 cells"
    );
    if grids.is_empty() {
        return Ok(H3BatchGridSeamFinalization {
            cells: 0,
            internal_edges: 0,
            geographic_samples: 0,
            reciprocal_raster_pairs: 0,
            authoritative_water_samples: 0,
            selected_transport_edges: 0,
            largest_equivalence_component: 0,
            largest_water_component: 0,
            water_reconciled_cells: 0,
            changed_cells: 0,
            cleared_artificial_trace_cells: 0,
        });
    }
    let dimensions = (grids[0].width, grids[0].height);
    ensure!(
        grids.iter().all(|grid| {
            (grid.width, grid.height) == dimensions
                && grid.cells.len() == usize::from(grid.width) * usize::from(grid.height)
        }),
        "batch grid seam finalization requires equally sized complete grids"
    );

    // Profiles hold the canonical geographic sample sequence for each edge.
    // The grids still contain their independently-authored final terrain here;
    // reconciliation happens only after every constraint has been collected.
    let profiles = grids
        .iter()
        .map(build_h3_grid_seam_profile)
        .collect::<Result<Vec<_>>>()?;
    let mut by_edge = BTreeMap::<String, Vec<(usize, usize)>>::new();
    for (grid_index, profile) in profiles.iter().enumerate() {
        for (edge_index, edge) in profile.edges.iter().enumerate() {
            if cells.contains_key(&edge.neighbor) {
                by_edge
                    .entry(edge.edge_id.clone())
                    .or_default()
                    .push((grid_index, edge_index));
            }
        }
    }

    let mut nodes = BTreeMap::<RasterNode, usize>::new();
    let mut raster_nodes = Vec::<RasterNode>::new();
    let mut sets = DisjointSets::default();
    let mut water_nodes = BTreeSet::<usize>::new();
    let mut water_backed_rim_pairs = Vec::<(usize, usize)>::new();
    let mut artificial_trace_nodes = BTreeSet::<usize>::new();
    let mut transport_nodes = BTreeMap::<usize, FeatureKind>::new();
    let mut sample_memberships = BTreeMap::<usize, Vec<SampleMembership>>::new();
    let mut internal_edges = 0usize;
    let mut geographic_samples = 0usize;
    let mut reciprocal_raster_pairs = 0usize;
    let mut authoritative_water_samples = 0usize;
    let mut selected_transport_edges = 0usize;

    for (edge_id, endpoints) in by_edge {
        let [(left_grid, left_edge), (right_grid, right_edge)] = endpoints.as_slice() else {
            bail!(
                "internal H3 grid edge {edge_id} has {} endpoints instead of two",
                endpoints.len()
            );
        };
        internal_edges += 1;
        let left = &profiles[*left_grid].edges[*left_edge];
        let right = &profiles[*right_grid].edges[*right_edge];
        ensure!(
            left.neighbor == profiles[*right_grid].cell
                && right.neighbor == profiles[*left_grid].cell,
            "internal H3 grid edge {edge_id} is not reciprocal"
        );
        ensure!(
            left.samples.len() == right.samples.len() && !left.samples.is_empty(),
            "internal H3 grid edge {edge_id} has inconsistent sample counts"
        );
        let left_plan = grids[*left_grid]
            .source
            .h3
            .as_ref()
            .expect("H3 plans validated above");
        let right_plan = grids[*right_grid]
            .source
            .h3
            .as_ref()
            .expect("H3 plans validated above");

        for (sample_index, (left_sample, right_sample)) in
            left.samples.iter().zip(&right.samples).enumerate()
        {
            ensure!(
                coordinates_match(left_sample.coordinate, right_sample.coordinate),
                "internal H3 grid edge {edge_id} disagrees at geographic sample {sample_index}"
            );
            geographic_samples += 1;
            let left_band = crate::h3::h3_raster_sample_band(
                left_plan,
                &grids[*left_grid],
                left_sample.coordinate,
            )?;
            let right_band = crate::h3::h3_raster_sample_band(
                right_plan,
                &grids[*right_grid],
                right_sample.coordinate,
            )?;
            ensure!(
                left_band.len() == 3 && right_band.len() == 3,
                "internal H3 grid edge {edge_id} sample {sample_index} does not have two three-cell bands"
            );
            let source_water = left_sample.source_water || right_sample.source_water;
            authoritative_water_samples += usize::from(source_water);
            mark_one_cell_trace(
                &grids[*left_grid],
                *left_grid,
                &left_band,
                &mut nodes,
                &mut raster_nodes,
                &mut sets,
                &mut artificial_trace_nodes,
            );
            mark_one_cell_trace(
                &grids[*right_grid],
                *right_grid,
                &right_band,
                &mut nodes,
                &mut raster_nodes,
                &mut sets,
                &mut artificial_trace_nodes,
            );
            // Surface parity is a contract at the canonical boundary sample,
            // represented by depth zero. Extending every dry sample three
            // cells inward lets nearby quantized samples erase legitimate
            // interior roads and terrain. Authoritative water deliberately
            // owns the full three-cell shoreline band; selected transport
            // does the same in the directive pass below.
            let constrained_depths = if source_water { 3 } else { 1 };
            for depth in 0..constrained_depths {
                let left_node = intern_node(
                    RasterNode::new(*left_grid, left_band[depth], grids[*left_grid].width),
                    &mut nodes,
                    &mut raster_nodes,
                    &mut sets,
                );
                let right_node = intern_node(
                    RasterNode::new(*right_grid, right_band[depth], grids[*right_grid].width),
                    &mut nodes,
                    &mut raster_nodes,
                    &mut sets,
                );
                sets.union(left_node, right_node);
                reciprocal_raster_pairs += 1;
                if depth == 0 {
                    let left_water_backed = left_band[1..]
                        .iter()
                        .all(|&(x, y)| grids[*left_grid].cell(x, y) == Some(MapCell::Water));
                    let right_water_backed = right_band[1..]
                        .iter()
                        .all(|&(x, y)| grids[*right_grid].cell(x, y) == Some(MapCell::Water));
                    if left_water_backed || right_water_backed {
                        water_backed_rim_pairs.push((left_node, right_node));
                    }
                }
                for node in [left_node, right_node] {
                    sample_memberships
                        .entry(node)
                        .or_default()
                        .push(SampleMembership {
                            edge_id: edge_id.clone(),
                            sample_index,
                            source_water,
                        });
                }
                if source_water {
                    water_nodes.insert(left_node);
                    water_nodes.insert(right_node);
                }
            }
        }

        match (&left.regional_transport, &right.regional_transport) {
            (Some(left_transport), Some(right_transport)) => {
                ensure!(
                    left_transport.kind == right_transport.kind
                        && left_transport.transport == right_transport.transport
                        && coordinates_match(left_transport.coordinate, right_transport.coordinate),
                    "internal H3 grid edge {edge_id} has non-reciprocal transport directives"
                );
                if left_transport.kind == H3GridTransportDirectiveKind::Selected {
                    let transport = left_transport.transport.with_context(|| {
                        format!("selected H3 grid edge {edge_id} has no transport class")
                    })?;
                    selected_transport_edges += 1;
                    let left_band = crate::h3::h3_raster_sample_band(
                        left_plan,
                        &grids[*left_grid],
                        left_transport.coordinate,
                    )?;
                    let right_band = crate::h3::h3_raster_sample_band(
                        right_plan,
                        &grids[*right_grid],
                        right_transport.coordinate,
                    )?;
                    for depth in 0..3 {
                        let left_node = intern_node(
                            RasterNode::new(*left_grid, left_band[depth], grids[*left_grid].width),
                            &mut nodes,
                            &mut raster_nodes,
                            &mut sets,
                        );
                        let right_node = intern_node(
                            RasterNode::new(
                                *right_grid,
                                right_band[depth],
                                grids[*right_grid].width,
                            ),
                            &mut nodes,
                            &mut raster_nodes,
                            &mut sets,
                        );
                        sets.union(left_node, right_node);
                        transport_nodes.insert(left_node, transport);
                        transport_nodes.insert(right_node, transport);
                    }
                }
            }
            (None, None) => {}
            _ => bail!("internal H3 grid edge {edge_id} disagrees on transport authority"),
        }
    }

    let mut members = BTreeMap::<usize, Vec<usize>>::new();
    for node in 0..raster_nodes.len() {
        members.entry(sets.find(node)).or_default().push(node);
    }
    let water_roots = water_nodes
        .into_iter()
        .map(|node| sets.find(node))
        .collect::<BTreeSet<_>>();
    let water_backed_rim_roots = water_backed_rim_pairs
        .into_iter()
        .map(|(left, right)| {
            let root = sets.find(left);
            debug_assert_eq!(root, sets.find(right));
            root
        })
        .collect::<BTreeSet<_>>();
    let trace_roots = artificial_trace_nodes
        .into_iter()
        .map(|node| sets.find(node))
        .collect::<BTreeSet<_>>();
    let mut transports_by_root = BTreeMap::<usize, BTreeSet<FeatureKind>>::new();
    for (node, transport) in transport_nodes {
        transports_by_root
            .entry(sets.find(node))
            .or_default()
            .insert(transport);
    }
    let mut memberships_by_root = BTreeMap::<usize, Vec<SampleMembership>>::new();
    for (node, memberships) in sample_memberships {
        memberships_by_root
            .entry(sets.find(node))
            .or_default()
            .extend(memberships);
    }

    // Repeated geographic samples legitimately quantize to the same raster
    // cell, but reciprocal rounding must never turn that local overlap into a
    // transitive whole-edge flood. A water component may cover only the
    // three-sample shoreline halo already accepted by the hard seam audit.
    const WATER_QUANTIZATION_HALO: usize = 3;
    for &root in &water_roots {
        let memberships = memberships_by_root
            .get(&root)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let mut by_edge = BTreeMap::<&str, (Vec<usize>, Vec<usize>)>::new();
        for membership in memberships {
            let (water, dry) = by_edge
                .entry(membership.edge_id.as_str())
                .or_insert_with(|| (Vec::new(), Vec::new()));
            if membership.source_water {
                water.push(membership.sample_index);
            } else {
                dry.push(membership.sample_index);
            }
        }
        for (edge_id, (water, dry)) in by_edge {
            ensure!(
                !water.is_empty(),
                "source-water contradiction: an H3 raster component would carry water onto unrelated edge {edge_id}"
            );
            for dry_index in dry {
                let nearest = water
                    .iter()
                    .map(|water_index| water_index.abs_diff(dry_index))
                    .min()
                    .unwrap_or(usize::MAX);
                ensure!(
                    nearest <= WATER_QUANTIZATION_HALO,
                    "source-water contradiction: H3 edge {edge_id} sample {dry_index} is {nearest} samples from exact water but shares its raster component"
                );
            }
        }
    }

    let mut planned_changes = Vec::<(RasterNode, MapCell, bool)>::new();
    let largest_equivalence_component = members.values().map(Vec::len).max().unwrap_or(0);
    let largest_water_component = members
        .iter()
        .filter(|(root, _)| water_roots.contains(root))
        .map(|(_, members)| members.len())
        .max()
        .unwrap_or(0);
    let mut water_reconciled_cells = 0usize;
    for (root, component) in members {
        let transport = transports_by_root.get(&root);
        ensure!(
            transport.is_none_or(|classes| classes.len() == 1),
            "one reciprocal H3 seam raster component carries conflicting transport classes"
        );
        let current_categories = component
            .iter()
            .map(|&node| {
                let node = raster_nodes[node];
                seam_category(grids[node.grid].cells[node.cell])
            })
            .collect::<BTreeSet<_>>();
        let isolated_water_rim = (trace_roots.contains(&root)
            || current_categories
                .iter()
                .all(|category| *category == SeamCategory::Water))
            && component.iter().all(|&node| {
                let node = raster_nodes[node];
                let grid = &grids[node.grid];
                let x = node.cell % usize::from(grid.width);
                let y = node.cell / usize::from(grid.width);
                let neighbors = [
                    x.checked_sub(1).map(|nx| (nx, y)),
                    (x + 1 < usize::from(grid.width)).then_some((x + 1, y)),
                    y.checked_sub(1).map(|ny| (x, ny)),
                    (y + 1 < usize::from(grid.height)).then_some((x, y + 1)),
                ];
                let cells = neighbors
                    .into_iter()
                    .flatten()
                    .map(|(nx, ny)| grid.cells[ny * usize::from(grid.width) + nx]);
                let cells = cells.collect::<Vec<_>>();
                cells.contains(&MapCell::Water)
                    && cells.into_iter().all(|cell| !is_walkable_seam_cell(cell))
            });
        let target = if let Some(transport) = transport.and_then(|classes| classes.first().copied())
        {
            SeamTarget::Transport(transport)
        } else if water_roots.contains(&root) {
            SeamTarget::Water
        } else if water_backed_rim_roots.contains(&root)
            && (trace_roots.contains(&root)
                || current_categories
                    .iter()
                    .all(|category| *category == SeamCategory::Water))
        {
            // A one-cell tree/fence/relief trace backed by two complete water
            // depths on either reciprocal face is a quantized continuation of
            // the shoreline, not dry land. Demoting that blocker to Grass can
            // create an unreachable sliver between Water and H3Void. Extend
            // only this exact reciprocal rim component to Water; one dry
            // inner sentinel keeps the ordinary Ground reconciliation path.
            // Once converted, retain the fully water-backed rim at the next
            // fixed-point pass instead of demoting it back to Grass.
            SeamTarget::Water
        } else if isolated_water_rim {
            // Reconciliation must not turn a shoreline blocker into a tiny
            // walkable island boxed between Water and the H3 mask. Both
            // reciprocal samples take the same Water target, preserving the
            // strict rendered seam contract without inventing dry access.
            SeamTarget::Water
        } else if trace_roots.contains(&root)
            || current_categories.contains(&SeamCategory::Water)
            || current_categories.contains(&SeamCategory::Transport)
            || current_categories.len() != 1
        {
            SeamTarget::Ground
        } else {
            SeamTarget::Keep(*current_categories.first().expect("one category"))
        };

        for node in component {
            let node = raster_nodes[node];
            let old = grids[node.grid].cells[node.cell];
            let new = target.cell(old);
            if new == old {
                continue;
            }
            ensure!(
                !is_protected_seam_cell(old),
                "H3 seam reconciliation would erase protected {:?} in cell {} at raster index {}",
                old,
                grids[node.grid]
                    .source
                    .h3
                    .as_ref()
                    .expect("H3 plan validated")
                    .cell,
                node.cell
            );
            let cleared_trace = trace_roots.contains(&root)
                && is_artificial_boundary_category(seam_category(old))
                && seam_category(new) == SeamCategory::Ground;
            water_reconciled_cells += usize::from(matches!(target, SeamTarget::Water));
            planned_changes.push((node, new, cleared_trace));
        }
    }

    // Validation above is deliberately all-or-nothing. A protected POI
    // conflict returns without changing any grid instead of leaving a partly
    // reconciled batch behind.
    let changed_cells = planned_changes.len();
    let cleared_artificial_trace_cells = planned_changes
        .iter()
        .filter(|(_, _, cleared_trace)| *cleared_trace)
        .count();
    for (node, cell, _) in planned_changes {
        grids[node.grid].cells[node.cell] = cell;
    }

    Ok(H3BatchGridSeamFinalization {
        cells: grids.len(),
        internal_edges,
        geographic_samples,
        reciprocal_raster_pairs,
        authoritative_water_samples,
        selected_transport_edges,
        largest_equivalence_component,
        largest_water_component,
        water_reconciled_cells,
        changed_cells,
        cleared_artificial_trace_cells,
    })
}

fn is_walkable_seam_cell(cell: MapCell) -> bool {
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
            | MapCell::Bench
            | MapCell::TrashCan
            | MapCell::Fountain
            | MapCell::FenceNorth
            | MapCell::FenceSouth
            | MapCell::FenceWest
            | MapCell::FenceEast
            | MapCell::FenceNorthWest
            | MapCell::FenceNorthEast
            | MapCell::FenceSouthWest
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RasterNode {
    grid: usize,
    cell: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SampleMembership {
    edge_id: String,
    sample_index: usize,
    source_water: bool,
}

impl RasterNode {
    fn new(grid: usize, coordinate: (u16, u16), width: u16) -> Self {
        Self {
            grid,
            cell: usize::from(coordinate.1) * usize::from(width) + usize::from(coordinate.0),
        }
    }
}

#[derive(Debug, Default)]
struct DisjointSets {
    parent: Vec<usize>,
    rank: Vec<u8>,
}

impl DisjointSets {
    fn push(&mut self) -> usize {
        let node = self.parent.len();
        self.parent.push(node);
        self.rank.push(0);
        node
    }

    fn find(&mut self, node: usize) -> usize {
        let parent = self.parent[node];
        if parent != node {
            self.parent[node] = self.find(parent);
        }
        self.parent[node]
    }

    fn union(&mut self, left: usize, right: usize) {
        let mut left = self.find(left);
        let mut right = self.find(right);
        if left == right {
            return;
        }
        if self.rank[left] < self.rank[right] {
            std::mem::swap(&mut left, &mut right);
        }
        self.parent[right] = left;
        if self.rank[left] == self.rank[right] {
            self.rank[left] += 1;
        }
    }
}

fn intern_node(
    node: RasterNode,
    nodes: &mut BTreeMap<RasterNode, usize>,
    raster_nodes: &mut Vec<RasterNode>,
    sets: &mut DisjointSets,
) -> usize {
    if let Some(&existing) = nodes.get(&node) {
        return existing;
    }
    let index = sets.push();
    nodes.insert(node, index);
    raster_nodes.push(node);
    index
}

fn mark_one_cell_trace(
    grid: &GeneratedGrid,
    grid_index: usize,
    band: &[(u16, u16)],
    nodes: &mut BTreeMap<RasterNode, usize>,
    raster_nodes: &mut Vec<RasterNode>,
    sets: &mut DisjointSets,
    artificial_trace_nodes: &mut BTreeSet<usize>,
) {
    if band.len() != 3 {
        return;
    }
    let category = seam_category(grid.cell(band[0].0, band[0].1).expect("band is in grid"));
    if !is_artificial_boundary_category(category)
        || [band[1], band[2]].into_iter().all(|coordinate| {
            grid.cell(coordinate.0, coordinate.1)
                .is_some_and(|cell| seam_category(cell) == category)
        })
    {
        return;
    }
    let node = intern_node(
        RasterNode::new(grid_index, band[0], grid.width),
        nodes,
        raster_nodes,
        sets,
    );
    artificial_trace_nodes.insert(node);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SeamCategory {
    Void,
    Ground,
    Wild,
    Tree,
    Relief,
    Fence,
    Fixture,
    Structure,
    Water,
    Transport,
}

fn seam_category(cell: MapCell) -> SeamCategory {
    match cell {
        MapCell::H3Void => SeamCategory::Void,
        MapCell::Grass
        | MapCell::Lawn
        | MapCell::Clearing
        | MapCell::Flowers
        | MapCell::Pitch
        | MapCell::IceFloor
        | MapCell::RockFloor => SeamCategory::Ground,
        MapCell::Park => SeamCategory::Wild,
        MapCell::Tree | MapCell::ParkTree | MapCell::SmallTree | MapCell::SmallTreeSouth => {
            SeamCategory::Tree
        }
        MapCell::Boulder
        | MapCell::IceBoulder
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
        | MapCell::CliffStairs => SeamCategory::Relief,
        MapCell::FenceNorthWest
        | MapCell::FenceNorth
        | MapCell::FenceNorthEast
        | MapCell::FenceWest
        | MapCell::FenceEast
        | MapCell::FenceSouthWest
        | MapCell::FenceSouth
        | MapCell::FenceSouthEast => SeamCategory::Fence,
        MapCell::Bench | MapCell::TrashCan | MapCell::Fountain | MapCell::GroundSign => {
            SeamCategory::Fixture
        }
        MapCell::Building
        | MapCell::PokecenterNorthWest
        | MapCell::PokecenterNorthEast
        | MapCell::PokecenterSouthWest
        | MapCell::PokecenterSouthEast
        | MapCell::MartNorthWest
        | MapCell::MartNorthEast
        | MapCell::MartSouthWest
        | MapCell::MartSouthEast => SeamCategory::Structure,
        MapCell::Water
        | MapCell::WaterAccessEast
        | MapCell::WaterAccessWest
        | MapCell::WaterAccessSouth => SeamCategory::Water,
        MapCell::Rail | MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad => {
            SeamCategory::Transport
        }
    }
}

fn is_artificial_boundary_category(category: SeamCategory) -> bool {
    matches!(
        category,
        SeamCategory::Tree | SeamCategory::Relief | SeamCategory::Fence
    )
}

fn is_protected_seam_cell(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::Bench
            | MapCell::TrashCan
            | MapCell::Fountain
            | MapCell::GroundSign
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeamTarget {
    Ground,
    Water,
    Transport(FeatureKind),
    Keep(SeamCategory),
}

impl SeamTarget {
    fn cell(self, current: MapCell) -> MapCell {
        match self {
            Self::Ground => {
                if seam_category(current) == SeamCategory::Ground {
                    current
                } else {
                    MapCell::Grass
                }
            }
            Self::Water => {
                if seam_category(current) == SeamCategory::Water {
                    current
                } else {
                    MapCell::Water
                }
            }
            Self::Transport(transport) => MapCell::from(transport),
            Self::Keep(category) => {
                debug_assert_eq!(seam_category(current), category);
                current
            }
        }
    }
}

fn coordinates_match(left: crate::Coordinate, right: crate::Coordinate) -> bool {
    let longitude_delta = (left.lon - right.lon + 180.0).rem_euclid(360.0) - 180.0;
    (left.lat - right.lat).abs() <= 1e-10 && longitude_delta.abs() <= 1e-10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BoundingBox, Coordinate, Feature, H3CellPlan, H3GridSeamSurface, H3RegionalCellPlan,
        H3RegionalConnection, MapCell, MapSource, audit_h3_grid_seams, build_h3_grid_seam_profile,
        plan_h3_batch,
    };

    const MINNEAPOLIS: Coordinate = Coordinate {
        lat: 44.947_519_6,
        lon: -93.325_347_7,
    };

    fn source_for(plan: &H3CellPlan) -> MapSource {
        MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds.first().copied().unwrap_or(BoundingBox {
                south: plan.center.lat - 0.1,
                west: plan.center.lon - 0.1,
                north: plan.center.lat + 0.1,
                east: plan.center.lon + 0.1,
            }),
            attribution: "batch seam fixture".to_string(),
            features: Vec::new(),
            h3: Some(plan.clone()),
        }
    }

    fn fixture() -> (Vec<GeneratedGrid>, String) {
        let batch = plan_h3_batch(MINNEAPOLIS, 6, 2).expect("two neighboring cells");
        let first = &batch.cells[0].plan;
        let second = &batch.cells[1].plan;
        let edge_id = first
            .portals
            .iter()
            .find(|portal| portal.neighbor == second.cell)
            .expect("shared portal")
            .edge_id
            .clone();
        let grids = [first, second]
            .into_iter()
            .map(|plan| GeneratedGrid {
                source: source_for(plan),
                width: 64,
                height: 64,
                cells: vec![MapCell::Grass; 64 * 64],
                labels: Vec::new(),
            })
            .collect();
        (grids, edge_id)
    }

    fn minneapolis_south_edge_fixture() -> (Vec<GeneratedGrid>, String) {
        const NORTH_CELL: &str = "86262cd27ffffff";
        const SOUTH_CELL: &str = "86262cd2fffffff";
        let batch = plan_h3_batch(MINNEAPOLIS, 6, 3).expect("three Minneapolis cells");
        let first = batch
            .cells
            .iter()
            .map(|cell| &cell.plan)
            .find(|plan| plan.cell == NORTH_CELL)
            .expect("north Minneapolis cell");
        let second = batch
            .cells
            .iter()
            .map(|cell| &cell.plan)
            .find(|plan| plan.cell == SOUTH_CELL)
            .expect("south Minneapolis cell");
        let edge_id = first
            .portals
            .iter()
            .find(|portal| portal.neighbor == second.cell)
            .expect("shared south portal")
            .edge_id
            .clone();
        let mut grids = [first, second]
            .into_iter()
            .map(|plan| GeneratedGrid {
                source: source_for(plan),
                width: 64,
                height: 64,
                cells: vec![MapCell::Grass; 64 * 64],
                labels: Vec::new(),
            })
            .collect::<Vec<_>>();
        for grid in &mut grids {
            let plan = grid.source.h3.clone().expect("H3 plan");
            for y in 0..grid.height {
                for x in 0..grid.width {
                    if !plan
                        .raster_contains_cell(x, y, grid.width, grid.height)
                        .expect("raster containment")
                    {
                        grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] =
                            MapCell::H3Void;
                    }
                }
            }
            let (home_x, home_y) = grid.home_cell();
            grid.cells[usize::from(home_y) * usize::from(grid.width) + usize::from(home_x)] =
                MapCell::Grass;
        }
        (grids, edge_id)
    }

    fn shared_edge_coordinates(grid: &GeneratedGrid, edge_id: &str) -> Vec<crate::Coordinate> {
        build_h3_grid_seam_profile(grid)
            .expect("grid seam profile")
            .edges
            .into_iter()
            .find(|edge| edge.edge_id == edge_id)
            .expect("shared edge profile")
            .samples
            .into_iter()
            .map(|sample| sample.coordinate)
            .collect()
    }

    fn stamp_band(grids: &mut [GeneratedGrid], coordinate: crate::Coordinate, band: [MapCell; 3]) {
        for grid in grids {
            let plan = grid.source.h3.clone().expect("H3 plan");
            for ((x, y), cell) in crate::h3::h3_raster_sample_band(&plan, grid, coordinate)
                .expect("sample band")
                .into_iter()
                .zip(band)
            {
                grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] = cell;
            }
        }
    }

    fn unreachable_fixture_walkable_cells(grid: &GeneratedGrid) -> BTreeSet<usize> {
        let walkable = |cell| matches!(cell, MapCell::Grass);
        let width = usize::from(grid.width);
        let (home_x, home_y) = grid.home_cell();
        let home = usize::from(home_y) * width + usize::from(home_x);
        if !walkable(grid.cells[home]) {
            return grid
                .cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| walkable(*cell).then_some(index))
                .collect();
        }
        let mut reached = vec![false; grid.cells.len()];
        let mut frontier = std::collections::VecDeque::from([home]);
        reached[home] = true;
        while let Some(index) = frontier.pop_front() {
            let x = index % width;
            let y = index / width;
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= width || next_y >= usize::from(grid.height) {
                    continue;
                }
                let next = next_y * width + next_x;
                if !reached[next] && walkable(grid.cells[next]) {
                    reached[next] = true;
                    frontier.push_back(next);
                }
            }
        }
        grid.cells
            .iter()
            .zip(reached)
            .enumerate()
            .filter_map(|(index, (cell, reached))| (walkable(*cell) && !reached).then_some(index))
            .collect()
    }

    #[test]
    fn batch_finalization_removes_a_reciprocal_one_cell_canopy_trace() {
        let (mut grids, edge_id) = fixture();
        let profile = build_h3_grid_seam_profile(&grids[0]).expect("first profile");
        let samples = &profile
            .edges
            .iter()
            .find(|edge| edge.edge_id == edge_id)
            .expect("shared profile")
            .samples;
        let plan = grids[0].source.h3.clone().expect("H3 plan");
        for sample in samples {
            let border = crate::h3::h3_raster_sample_band(&plan, &grids[0], sample.coordinate)
                .expect("sample band")[0];
            grids[0].cells[usize::from(border.1) * 64 + usize::from(border.0)] = MapCell::Tree;
        }

        let summary = finalize_h3_batch_grid_seams(&mut grids)
            .expect("batch finalization reaches fixed point");
        assert!(summary.changed_cells > 0, "{summary:?}");
        let fixed_point = grids.clone();
        let repeated = finalize_h3_batch_grid_seams(&mut grids)
            .expect("repeating batch finalization is valid");
        assert_eq!(repeated.changed_cells, 0, "{repeated:?}");
        assert_eq!(
            grids, fixed_point,
            "the seam fixed point must be idempotent"
        );

        let profiles = grids
            .iter()
            .map(build_h3_grid_seam_profile)
            .collect::<Result<Vec<_>>>()
            .expect("final profiles");
        let edges = profiles
            .iter()
            .map(|profile| {
                profile
                    .edges
                    .iter()
                    .find(|edge| edge.edge_id == edge_id)
                    .expect("shared edge")
            })
            .collect::<Vec<_>>();
        assert!(
            edges[0]
                .samples
                .iter()
                .zip(&edges[1].samples)
                .all(|(left, right)| left.surface == right.surface)
        );
        assert!(
            edges
                .iter()
                .flat_map(|edge| &edge.samples)
                .all(|sample| sample.surface != H3GridSeamSurface::Tree)
        );
    }

    #[test]
    fn water_backed_minneapolis_rim_trace_becomes_continuous_water() {
        let (mut grids, edge_id) = minneapolis_south_edge_fixture();
        let samples = shared_edge_coordinates(&grids[0], &edge_id);
        let coordinates = [samples[23], samples[24]];
        let first_plan = grids[0].source.h3.clone().expect("north plan");
        assert_eq!(
            crate::h3::h3_raster_sample_band(&first_plan, &grids[0], coordinates[0])
                .expect("exact sample 23 band"),
            [(54, 51), (53, 51), (53, 50)]
        );
        assert_eq!(
            crate::h3::h3_raster_sample_band(&first_plan, &grids[0], coordinates[1])
                .expect("exact sample 24 band"),
            [(55, 51), (55, 50), (54, 50)]
        );
        // These two rim cells are reused by neighboring geographic samples
        // after raster quantization. Mirror the real v6 shoreline: the whole
        // sample 21-25 run has water at both inward depths, while only samples
        // 23/24 carry the artificial one-cell tree trace at depth zero.
        for &coordinate in &samples[21..=25] {
            for grid in &mut grids {
                let plan = grid.source.h3.clone().expect("H3 plan");
                let band = crate::h3::h3_raster_sample_band(&plan, grid, coordinate)
                    .expect("water-backed sample band");
                for &(x, y) in &band[1..] {
                    grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] =
                        MapCell::Water;
                }
            }
        }
        for coordinate in coordinates {
            for grid in &mut grids {
                let plan = grid.source.h3.clone().expect("H3 plan");
                let (x, y) = crate::h3::h3_raster_sample_band(&plan, grid, coordinate)
                    .expect("exact traced sample band")[0];
                grid.cells[usize::from(y) * usize::from(grid.width) + usize::from(x)] =
                    MapCell::Tree;
            }
        }
        let water_before = grids
            .iter()
            .map(|grid| {
                grid.cells
                    .iter()
                    .enumerate()
                    .filter_map(|(index, cell)| (*cell == MapCell::Water).then_some(index))
                    .collect::<BTreeSet<_>>()
            })
            .collect::<Vec<_>>();
        let unreachable_before = grids
            .iter()
            .map(unreachable_fixture_walkable_cells)
            .collect::<Vec<_>>();

        let summary = finalize_h3_batch_grid_seams(&mut grids).expect("shoreline finalization");
        assert_eq!(summary.changed_cells, 8, "{summary:?}");
        for (grid_index, grid) in grids.iter().enumerate() {
            let plan = grid.source.h3.as_ref().expect("H3 plan");
            let expected_water = coordinates
                .iter()
                .flat_map(|&coordinate| {
                    crate::h3::h3_raster_sample_band(plan, grid, coordinate)
                        .expect("water-backed band")
                })
                .map(|(x, y)| usize::from(y) * usize::from(grid.width) + usize::from(x))
                .chain(water_before[grid_index].iter().copied())
                .collect::<BTreeSet<_>>();
            let actual_water = grid
                .cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| (*cell == MapCell::Water).then_some(index))
                .collect::<BTreeSet<_>>();
            let allowed_water = shared_edge_coordinates(grid, &edge_id)
                .into_iter()
                .map(|coordinate| {
                    let (x, y) = crate::h3::h3_raster_sample_band(plan, grid, coordinate)
                        .expect("allowed seam rim")[0];
                    usize::from(y) * usize::from(grid.width) + usize::from(x)
                })
                .chain(expected_water.iter().copied())
                .collect::<BTreeSet<_>>();
            assert!(
                expected_water.is_subset(&actual_water) && actual_water.is_subset(&allowed_water),
                "water spread beyond exact bands and isolated seam rims"
            );
            let unreachable_after = unreachable_fixture_walkable_cells(grid);
            assert!(
                unreachable_after.is_subset(&unreachable_before[grid_index]),
                "seam finalization created new unreachable walkable cells: before={:?}, after={unreachable_after:?}",
                unreachable_before[grid_index]
            );
        }

        let fixed_point = grids.clone();
        let repeated = finalize_h3_batch_grid_seams(&mut grids).expect("repeat finalization");
        assert_eq!(repeated.changed_cells, 0, "{repeated:?}");
        assert_eq!(grids, fixed_point, "shoreline fixed point must be stable");
    }

    #[test]
    fn one_dry_inner_depth_prevents_rim_water_inference() {
        let (mut grids, edge_id) = minneapolis_south_edge_fixture();
        let coordinate = shared_edge_coordinates(&grids[0], &edge_id)[23];
        stamp_band(
            &mut grids,
            coordinate,
            [MapCell::Tree, MapCell::Water, MapCell::Tree],
        );
        let bands = grids
            .iter()
            .map(|grid| {
                crate::h3::h3_raster_sample_band(
                    grid.source.h3.as_ref().expect("H3 plan"),
                    grid,
                    coordinate,
                )
                .expect("dry-sentinel band")
            })
            .collect::<Vec<_>>();

        let summary = finalize_h3_batch_grid_seams(&mut grids).expect("dry reconciliation");
        assert_eq!(summary.water_reconciled_cells, 0, "{summary:?}");
        for (grid, band) in grids.iter().zip(bands) {
            assert_eq!(grid.cell(band[0].0, band[0].1), Some(MapCell::Grass));
            assert_eq!(grid.cell(band[1].0, band[1].1), Some(MapCell::Water));
            assert_eq!(grid.cell(band[2].0, band[2].1), Some(MapCell::Tree));
        }
    }

    #[test]
    fn batch_finalization_fails_atomically_instead_of_erasing_protected_pois() {
        for protected in [
            MapCell::Building,
            MapCell::PokecenterNorthWest,
            MapCell::MartSouthEast,
            MapCell::GroundSign,
            MapCell::Bench,
            MapCell::TrashCan,
            MapCell::CliffNorth,
            MapCell::LedgeMiddle,
            MapCell::CliffStairs,
        ] {
            let (mut grids, edge_id) = fixture();
            let profile = build_h3_grid_seam_profile(&grids[0]).expect("first profile");
            let coordinate = profile
                .edges
                .iter()
                .find(|edge| edge.edge_id == edge_id)
                .expect("shared edge")
                .samples[15]
                .coordinate;
            let plan = grids[0].source.h3.clone().expect("H3 plan");
            let border = crate::h3::h3_raster_sample_band(&plan, &grids[0], coordinate)
                .expect("sample band")[0];
            grids[0].cells[usize::from(border.1) * 64 + usize::from(border.0)] = protected;
            let untouched = grids.clone();

            let error = finalize_h3_batch_grid_seams(&mut grids)
                .expect_err("protected seam conflict must fail closed");
            assert!(format!("{error:#}").contains("protected"), "{error:#}");
            assert_eq!(grids, untouched, "failure must not partially mutate grids");
        }
    }

    #[test]
    fn batch_finalization_preserves_the_exact_selected_transport_class() {
        let (mut grids, edge_id) = fixture();
        let coordinate = grids[0]
            .source
            .h3
            .as_ref()
            .expect("first plan")
            .portals
            .iter()
            .find(|portal| portal.edge_id == edge_id)
            .expect("shared portal")
            .midpoint;
        let cell_names = grids
            .iter()
            .map(|grid| grid.source.h3.as_ref().expect("plan").cell.clone())
            .collect::<Vec<_>>();
        for index in 0..2 {
            let neighbor = cell_names[1 - index].clone();
            let plan = grids[index].source.h3.as_mut().expect("plan");
            plan.regional = Some(H3RegionalCellPlan {
                ordinal: index,
                cell: cell_names[index].clone(),
                building_count: 0,
                facilities: Vec::new(),
                connections: vec![H3RegionalConnection {
                    edge_id: edge_id.clone(),
                    neighbor,
                    coordinate,
                    transport: FeatureKind::MajorRoad,
                    bridge: false,
                    authoritative: true,
                    boundary_exit: false,
                }],
                closed_transport_crossings: Vec::new(),
            });
        }

        let summary = finalize_h3_batch_grid_seams(&mut grids).expect("selected landing");
        assert_eq!(summary.selected_transport_edges, 1);
        for grid in &grids {
            let plan = grid.source.h3.as_ref().expect("plan");
            for (x, y) in
                crate::h3::h3_raster_sample_band(plan, grid, coordinate).expect("selected band")
            {
                assert_eq!(grid.cell(x, y), Some(MapCell::MajorRoad));
            }
        }
    }

    #[test]
    fn one_sided_exact_source_water_continues_without_flooding_the_rest_of_the_edge() {
        let (mut grids, edge_id) = fixture();
        let profile = build_h3_grid_seam_profile(&grids[0]).expect("first profile");
        let samples = &profile
            .edges
            .iter()
            .find(|edge| edge.edge_id == edge_id)
            .expect("shared edge")
            .samples;
        let coordinate = samples[15].coordinate;
        let spacing = (samples[14].coordinate.lat - coordinate.lat)
            .abs()
            .max((samples[14].coordinate.lon - coordinate.lon).abs());
        let epsilon = spacing / 8.0;
        grids[0].source.features.push(Feature {
            kind: FeatureKind::Water,
            name: Some("one-sided exact lake".to_string()),
            area: true,
            bridge: false,
            points: vec![
                Coordinate {
                    lat: coordinate.lat - epsilon,
                    lon: coordinate.lon - epsilon,
                },
                Coordinate {
                    lat: coordinate.lat - epsilon,
                    lon: coordinate.lon + epsilon,
                },
                Coordinate {
                    lat: coordinate.lat + epsilon,
                    lon: coordinate.lon + epsilon,
                },
                Coordinate {
                    lat: coordinate.lat + epsilon,
                    lon: coordinate.lon - epsilon,
                },
                Coordinate {
                    lat: coordinate.lat - epsilon,
                    lon: coordinate.lon - epsilon,
                },
            ],
        });

        let summary = finalize_h3_batch_grid_seams(&mut grids).expect("water reconciliation");
        let profiles = grids
            .iter()
            .map(build_h3_grid_seam_profile)
            .collect::<Result<Vec<_>>>()
            .expect("final profiles");
        let audit = audit_h3_grid_seams(&profiles);
        assert!(audit.passed, "{}", audit.errors.join("; "));
        assert_eq!(audit.authoritative_water_samples, 1);
        assert_eq!(
            audit.authoritative_water_samples,
            audit.continuous_water_samples
        );
        assert_eq!(audit.mismatched_surface_samples, 0);
        assert!(summary.water_reconciled_cells <= 12, "{summary:?}");
        assert!(summary.largest_water_component <= 8, "{summary:?}");
        let edge_profiles = profiles
            .iter()
            .map(|profile| {
                profile
                    .edges
                    .iter()
                    .find(|edge| edge.edge_id == edge_id)
                    .expect("shared edge")
            })
            .collect::<Vec<_>>();
        for sentinel in [0, 30] {
            assert_eq!(
                edge_profiles[0].samples[sentinel].surface,
                H3GridSeamSurface::Ground
            );
            assert_eq!(
                edge_profiles[1].samples[sentinel].surface,
                H3GridSeamSurface::Ground
            );
        }
    }
}
