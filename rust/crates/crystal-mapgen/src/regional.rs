use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    Coordinate, FeatureKind, GeneratedGrid, H3BatchConnections, H3BatchLink, H3BatchManifest,
    H3ClosedTransportCrossing, H3Facility, H3RegionalCellPlan, H3RegionalConnection,
    H3SeamContract, MapCell, MapSource,
};

pub const H3_REGIONAL_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3RegionalPlan {
    pub schema_version: u32,
    pub origin: String,
    pub cells: Vec<H3RegionalCellPlan>,
    pub pokemon_centers: usize,
    pub marts: usize,
    pub internal_connections: usize,
    pub loop_connections: usize,
    pub boundary_exits: usize,
    pub authoritative_internal_connections: usize,
    pub synthetic_internal_connections: usize,
    pub authoritative_boundary_exits: usize,
    pub synthetic_boundary_exits: usize,
    pub synthetic_connection_budget: usize,
}

impl H3RegionalPlan {
    pub fn cell(&self, cell: &str) -> Option<&H3RegionalCellPlan> {
        self.cells.iter().find(|entry| entry.cell == cell)
    }
}

/// Resolve runtime map transitions from the selected regional transport graph.
///
/// Topological H3 adjacency is intentionally broader than the authored road
/// graph. Runtime links therefore use only reciprocal selected connections and
/// land on the first cell of each exact three-cell transport band, never on a
/// generic presentation-side gate.
pub fn build_h3_regional_connections(
    manifest: &H3BatchManifest,
    regional: &H3RegionalPlan,
    grid_width: u16,
    grid_height: u16,
) -> Result<H3BatchConnections> {
    if regional.cells.len() != manifest.cells.len() {
        bail!(
            "regional runtime links need {} cell directives, got {}",
            manifest.cells.len(),
            regional.cells.len()
        );
    }
    let manifest_by_cell = manifest
        .cells
        .iter()
        .map(|entry| (entry.plan.cell.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut by_edge = BTreeMap::<String, Vec<(&H3RegionalCellPlan, &H3RegionalConnection)>>::new();
    for cell in &regional.cells {
        let entry = manifest_by_cell.get(cell.cell.as_str()).with_context(|| {
            format!(
                "regional runtime cell {} is absent from manifest",
                cell.cell
            )
        })?;
        if entry.ordinal != cell.ordinal {
            bail!(
                "regional runtime cell {} has ordinal {}, expected {}",
                cell.cell,
                cell.ordinal,
                entry.ordinal
            );
        }
        for connection in &cell.connections {
            if !connection.boundary_exit {
                by_edge
                    .entry(connection.edge_id.clone())
                    .or_default()
                    .push((cell, connection));
            }
        }
    }

    let mut links = Vec::with_capacity(by_edge.len());
    for (edge_id, mut endpoints) in by_edge {
        endpoints.sort_by_key(|(cell, _)| cell.ordinal);
        let [(first_cell, first), (second_cell, second)] = endpoints.as_slice() else {
            bail!(
                "regional runtime edge {edge_id} has {} endpoints instead of two",
                endpoints.len()
            );
        };
        if first.neighbor != second_cell.cell
            || second.neighbor != first_cell.cell
            || first.transport != second.transport
            || first.bridge != second.bridge
            || !coordinates_match(Some(first.coordinate), Some(second.coordinate))
        {
            bail!("regional runtime edge {edge_id} is not an exact reciprocal pair");
        }
        let first_entry = manifest_by_cell[first_cell.cell.as_str()];
        let second_entry = manifest_by_cell[second_cell.cell.as_str()];
        let first_portal = first_entry
            .plan
            .portals
            .iter()
            .find(|portal| portal.edge_id == edge_id)
            .with_context(|| format!("cell {} lacks selected edge {edge_id}", first_cell.cell))?;
        let second_portal = second_entry
            .plan
            .portals
            .iter()
            .find(|portal| portal.edge_id == edge_id)
            .with_context(|| format!("cell {} lacks selected edge {edge_id}", second_cell.cell))?;
        if second_portal.side != first_portal.side.opposite() {
            bail!("regional runtime edge {edge_id} does not use opposite presentation sides");
        }
        links.push(H3BatchLink {
            edge_id,
            first_ordinal: first_cell.ordinal,
            first_cell: first_cell.cell.clone(),
            first_side: first_portal.side,
            first_gate: crate::h3::h3_raster_landing(
                &first_entry.plan,
                grid_width,
                grid_height,
                first.coordinate,
            )?,
            second_ordinal: second_cell.ordinal,
            second_cell: second_cell.cell.clone(),
            second_side: second_portal.side,
            second_gate: crate::h3::h3_raster_landing(
                &second_entry.plan,
                grid_width,
                grid_height,
                second.coordinate,
            )?,
        });
    }
    if links.len() != regional.internal_connections {
        bail!(
            "regional runtime links contain {} edges, expected {}",
            links.len(),
            regional.internal_connections
        );
    }
    Ok(H3BatchConnections {
        schema_version: manifest.schema_version,
        grid_width,
        grid_height,
        links,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3RegionalGridReport {
    pub cell: String,
    pub pokemon_centers: usize,
    pub marts: usize,
    pub route_cells: usize,
    pub principal_route_cells: usize,
    pub principal_route_percent: f64,
    pub connected_edges: Vec<String>,
    pub leaking_closed_edges: Vec<String>,
    pub boundary_route_cells: usize,
    pub leaking_boundary_route_cells: Vec<(u16, u16)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3RegionalAudit {
    pub passed: bool,
    pub cells: usize,
    pub pokemon_centers: usize,
    pub marts: usize,
    pub internal_connections: usize,
    pub boundary_exits: usize,
    pub authoritative_connections: usize,
    pub synthetic_connections: usize,
    pub synthetic_connection_budget: usize,
    pub minimum_principal_route_percent: f64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct EdgeCandidate {
    edge_id: String,
    first: usize,
    second: Option<usize>,
    first_neighbor: String,
    second_neighbor: Option<String>,
    coordinate: Coordinate,
    transport: FeatureKind,
    bridge: bool,
    authoritative: bool,
}

/// Build one sparse transport and service plan for the whole H3 batch.
///
/// The road graph is a connected prefix tree plus a handful of real OSM loops
/// and exterior exits. It deliberately does not promote every geographic line
/// touching a face into a portal, which avoids six road stubs ringing every
/// room while retaining deterministic reciprocal crossings.
pub fn plan_h3_region(
    manifest: &H3BatchManifest,
    sources: &[MapSource],
    contracts: &[H3SeamContract],
) -> Result<H3RegionalPlan> {
    if sources.len() != manifest.cells.len() || contracts.len() != manifest.cells.len() {
        bail!(
            "regional planning needs one source and seam contract per manifest cell ({} cells, {} sources, {} contracts)",
            manifest.cells.len(),
            sources.len(),
            contracts.len()
        );
    }
    for (ordinal, ((entry, source), contract)) in manifest
        .cells
        .iter()
        .zip(sources)
        .zip(contracts)
        .enumerate()
    {
        if entry.ordinal != ordinal
            || source.h3.as_ref().map(|plan| plan.cell.as_str()) != Some(entry.plan.cell.as_str())
            || contract.cell != entry.plan.cell
        {
            bail!("regional input order or H3 identity differs at ordinal {ordinal}");
        }
    }

    let by_cell = manifest
        .cells
        .iter()
        .map(|entry| (entry.plan.cell.as_str(), entry.ordinal))
        .collect::<BTreeMap<_, _>>();
    let mut internal = BTreeMap::<String, EdgeCandidate>::new();
    let mut boundary = Vec::<EdgeCandidate>::new();
    for entry in &manifest.cells {
        let contract = &contracts[entry.ordinal];
        for portal in &entry.plan.portals {
            let edge = contract
                .edges
                .iter()
                .find(|edge| edge.edge_id == portal.edge_id)
                .with_context(|| {
                    format!("cell {} lacks edge {}", entry.plan.cell, portal.edge_id)
                })?;
            if let Some(&neighbor) = by_cell.get(portal.neighbor.as_str()) {
                if entry.ordinal < neighbor {
                    let reciprocal = contracts[neighbor]
                        .edges
                        .iter()
                        .find(|candidate| candidate.edge_id == portal.edge_id)
                        .with_context(|| {
                            format!("edge {} lacks reciprocal contract", portal.edge_id)
                        })?;
                    let reciprocal_crossing = edge.viable_crossings.iter().find(|crossing| {
                        reciprocal.viable_crossings.iter().any(|other| {
                            crossing.transport == other.transport
                                && crossing.bridge == other.bridge
                                && coordinates_match(
                                    Some(crossing.coordinate),
                                    Some(other.coordinate),
                                )
                        })
                    });
                    let (transport, coordinate, bridge, authoritative) =
                        if let Some(crossing) = reciprocal_crossing {
                            (
                                crossing.transport,
                                crossing.coordinate,
                                crossing.bridge,
                                true,
                            )
                        } else if edge.synthetic_traversable && reciprocal.synthetic_traversable {
                            (FeatureKind::Trail, portal.midpoint, false, false)
                        } else {
                            // A rejected real crossing is not a license to
                            // paint a midpoint Trail across the same water.
                            continue;
                        };
                    internal.insert(
                        portal.edge_id.clone(),
                        EdgeCandidate {
                            edge_id: portal.edge_id.clone(),
                            first: entry.ordinal,
                            second: Some(neighbor),
                            first_neighbor: portal.neighbor.clone(),
                            second_neighbor: Some(entry.plan.cell.clone()),
                            coordinate,
                            transport,
                            bridge,
                            authoritative,
                        },
                    );
                }
            } else {
                let (transport, coordinate, bridge, authoritative) =
                    if let Some(crossing) = edge.viable_crossings.first() {
                        (
                            crossing.transport,
                            crossing.coordinate,
                            crossing.bridge,
                            true,
                        )
                    } else if edge.synthetic_traversable {
                        (FeatureKind::Trail, portal.midpoint, false, false)
                    } else {
                        continue;
                    };
                boundary.push(EdgeCandidate {
                    edge_id: portal.edge_id.clone(),
                    first: entry.ordinal,
                    second: None,
                    first_neighbor: portal.neighbor.clone(),
                    second_neighbor: None,
                    coordinate,
                    transport,
                    bridge,
                    authoritative,
                });
            }
        }
    }

    // Select a real connected spanning tree over feasible faces. Manifest
    // ordinals are storage order, not road topology: a cell whose earlier
    // face is blocked by water may still connect honestly through a later
    // neighbor. The bounded search is complete for the <=37-cell proof batch,
    // favors authoritative/high-class lines, and enforces four portals while
    // building the tree rather than repairing an invalid prefix afterward.
    let (mut selected, mut degrees) =
        select_feasible_spanning_tree(&internal, manifest.cells.len())?;

    // Add a small number of genuine loops, first repairing degree-one leaves.
    let loop_target = manifest.cells.len().div_ceil(6).max(1);
    let mut loops = 0usize;
    while loops < loop_target {
        let mut candidates = internal
            .values()
            .filter(|edge| {
                let second = edge.second.expect("internal edge");
                !selected.contains(&edge.edge_id)
                    && edge.authoritative
                    && degrees[edge.first] < 4
                    && degrees[second] < 4
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|edge| {
            let second = edge.second.expect("internal edge");
            (
                std::cmp::Reverse(
                    usize::from(degrees[edge.first] < 2) + usize::from(degrees[second] < 2),
                ),
                degrees[edge.first] + degrees[second],
                std::cmp::Reverse(transport_priority(edge.transport)),
                edge.edge_id.clone(),
            )
        });
        let Some(edge) = candidates.first() else {
            break;
        };
        select_edge(edge, &mut selected, &mut degrees);
        loops += 1;
    }

    // Four sparse exterior continuations make the regional crop feel open.
    // Prefer real roads in different cells and compass sectors; a one-cell
    // proof still receives two deterministic trail exits.
    boundary.sort_by_key(|edge| {
        (
            !edge.authoritative,
            degrees[edge.first],
            std::cmp::Reverse(transport_priority(edge.transport)),
            edge.edge_id.clone(),
        )
    });
    let boundary_target = if manifest.cells.len() == 1 { 2 } else { 4 };
    let mut selected_boundary = BTreeSet::<String>::new();
    let mut boundary_cells = BTreeSet::<usize>::new();
    for edge in &boundary {
        if selected_boundary.len() == boundary_target {
            break;
        }
        if degrees[edge.first] >= 4 || boundary_cells.contains(&edge.first) {
            continue;
        }
        if manifest.cells.len() > 1 && !edge.authoritative {
            continue;
        }
        selected_boundary.insert(edge.edge_id.clone());
        boundary_cells.insert(edge.first);
        degrees[edge.first] += 1;
    }
    if manifest.cells.len() == 1 && selected_boundary.len() < boundary_target {
        for edge in &boundary {
            if selected_boundary.len() == boundary_target {
                break;
            }
            selected_boundary.insert(edge.edge_id.clone());
        }
    }

    if degrees.iter().any(|degree| *degree > 4) {
        bail!("regional edge selection exceeded the four-portal per-cell cap");
    }
    let authoritative_internal_connections = internal
        .values()
        .filter(|edge| selected.contains(&edge.edge_id) && edge.authoritative)
        .count();
    let synthetic_internal_connections = selected.len() - authoritative_internal_connections;
    let authoritative_boundary_exits = boundary
        .iter()
        .filter(|edge| selected_boundary.contains(&edge.edge_id) && edge.authoritative)
        .count();
    let synthetic_boundary_exits = selected_boundary.len() - authoritative_boundary_exits;
    let synthetic_connection_budget = synthetic_connection_budget(manifest.cells.len());
    let synthetic_connections = synthetic_internal_connections + synthetic_boundary_exits;
    if synthetic_connections > synthetic_connection_budget {
        bail!(
            "regional graph needs {synthetic_connections} synthetic Trail connections, exceeding its budget of {synthetic_connection_budget}; source transport is too sparse for an honest connected batch"
        );
    }

    let building_counts = sources
        .iter()
        .map(|source| {
            source
                .features
                .iter()
                .filter(|feature| feature.kind == FeatureKind::Building)
                .count()
        })
        .collect::<Vec<_>>();
    let distances = graph_distances(manifest);
    let service_target = manifest.cells.len().div_ceil(8).max(1);
    let centers = select_service_cells(
        service_target,
        &building_counts,
        &distances,
        Some(0),
        &BTreeSet::new(),
    );
    let marts = select_service_cells(
        service_target,
        &building_counts,
        &distances,
        (manifest.cells.len() == 1).then_some(0),
        &centers,
    );

    let mut cells = manifest
        .cells
        .iter()
        .map(|entry| H3RegionalCellPlan {
            ordinal: entry.ordinal,
            cell: entry.plan.cell.clone(),
            building_count: building_counts[entry.ordinal],
            facilities: Vec::new(),
            connections: Vec::new(),
            closed_transport_crossings: Vec::new(),
        })
        .collect::<Vec<_>>();
    for &ordinal in &centers {
        cells[ordinal].facilities.push(H3Facility::PokemonCenter);
    }
    for &ordinal in &marts {
        cells[ordinal].facilities.push(H3Facility::Mart);
    }
    for edge in internal.values() {
        if selected.contains(&edge.edge_id) {
            add_connection(&mut cells, edge, false);
        } else if edge.authoritative {
            cells[edge.first]
                .closed_transport_crossings
                .push(H3ClosedTransportCrossing {
                    edge_id: edge.edge_id.clone(),
                    coordinate: edge.coordinate,
                });
            cells[edge.second.expect("internal edge")]
                .closed_transport_crossings
                .push(H3ClosedTransportCrossing {
                    edge_id: edge.edge_id.clone(),
                    coordinate: edge.coordinate,
                });
        }
    }
    for edge in &boundary {
        if selected_boundary.contains(&edge.edge_id) {
            add_connection(&mut cells, edge, true);
        } else if edge.authoritative {
            cells[edge.first]
                .closed_transport_crossings
                .push(H3ClosedTransportCrossing {
                    edge_id: edge.edge_id.clone(),
                    coordinate: edge.coordinate,
                });
        }
    }
    for cell in &mut cells {
        cell.facilities.sort();
        cell.connections
            .sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
        cell.closed_transport_crossings
            .sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
    }

    Ok(H3RegionalPlan {
        schema_version: H3_REGIONAL_SCHEMA_VERSION,
        origin: manifest.origin.clone(),
        pokemon_centers: centers.len(),
        marts: marts.len(),
        internal_connections: selected.len(),
        loop_connections: loops,
        boundary_exits: selected_boundary.len(),
        authoritative_internal_connections,
        synthetic_internal_connections,
        authoritative_boundary_exits,
        synthetic_boundary_exits,
        synthetic_connection_budget,
        cells,
    })
}

pub fn inspect_h3_regional_grid(grid: &GeneratedGrid) -> Result<H3RegionalGridReport> {
    let plan = grid
        .source
        .h3
        .as_ref()
        .context("regional grid report requires an H3 plan")?;
    let regional = plan
        .regional
        .as_ref()
        .context("regional grid report requires batch directives")?;
    let components = route_components(grid);
    let principal = components
        .iter()
        .max_by_key(|component| component.len())
        .cloned()
        .unwrap_or_default();
    let route_cells = components.iter().map(Vec::len).sum::<usize>();
    let principal_set = principal.iter().copied().collect::<BTreeSet<_>>();
    let mut connected_edges = Vec::new();
    let mut selected_band = BTreeSet::<usize>::new();
    for connection in &regional.connections {
        let cells = projected_connection_cells(grid, connection.coordinate)?;
        selected_band.extend(cells.iter().copied());
        if cells
            .iter()
            .all(|index| principal_set.contains(index) && is_route(grid.cells[*index]))
            && cells.iter().any(|index| {
                let x = (*index % usize::from(grid.width)) as u16;
                let y = (*index / usize::from(grid.width)) as u16;
                crate::h3::route_cell_touches_h3_void(grid, x, y)
            })
        {
            connected_edges.push(connection.edge_id.clone());
        }
    }
    let mut leaking_closed_edges = Vec::new();
    for crossing in &regional.closed_transport_crossings {
        if projected_connection_cells(grid, crossing.coordinate)?
            .into_iter()
            .any(|cell| is_route(grid.cells[cell]))
        {
            leaking_closed_edges.push(crossing.edge_id.clone());
        }
    }
    let mut boundary_route_cells = 0usize;
    let mut leaking_boundary_route_cells = Vec::new();
    for y in 0..grid.height {
        for x in 0..grid.width {
            let index = usize::from(y) * usize::from(grid.width) + usize::from(x);
            if !is_route(grid.cells[index]) || !crate::h3::route_cell_touches_h3_void(grid, x, y) {
                continue;
            }
            boundary_route_cells += 1;
            let selected_apron = selected_band.iter().any(|selected| {
                let sx = selected % usize::from(grid.width);
                let sy = selected / usize::from(grid.width);
                sx.abs_diff(usize::from(x)) + sy.abs_diff(usize::from(y)) <= 2
            });
            if !selected_band.contains(&index) && !selected_apron {
                leaking_boundary_route_cells.push((x, y));
            }
        }
    }
    Ok(H3RegionalGridReport {
        cell: regional.cell.clone(),
        pokemon_centers: complete_facilities(grid, H3Facility::PokemonCenter),
        marts: complete_facilities(grid, H3Facility::Mart),
        route_cells,
        principal_route_cells: principal.len(),
        principal_route_percent: if route_cells == 0 {
            0.0
        } else {
            principal.len() as f64 / route_cells as f64 * 100.0
        },
        connected_edges,
        leaking_closed_edges,
        boundary_route_cells,
        leaking_boundary_route_cells,
    })
}

pub fn audit_h3_regional_batch(
    plan: &H3RegionalPlan,
    reports: &[H3RegionalGridReport],
) -> H3RegionalAudit {
    let mut errors = Vec::new();
    let by_report = reports
        .iter()
        .map(|report| (report.cell.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    if by_report.len() != plan.cells.len() {
        errors.push(format!(
            "regional audit received {} unique reports for {} cells",
            by_report.len(),
            plan.cells.len()
        ));
    }
    let mut centers = 0usize;
    let mut marts = 0usize;
    let mut minimum_principal = 100.0_f64;
    for cell in &plan.cells {
        let Some(report) = by_report.get(cell.cell.as_str()) else {
            errors.push(format!("regional cell {} has no grid report", cell.cell));
            continue;
        };
        let expected_center = usize::from(cell.facilities.contains(&H3Facility::PokemonCenter));
        let expected_mart = usize::from(cell.facilities.contains(&H3Facility::Mart));
        centers += report.pokemon_centers;
        marts += report.marts;
        if report.pokemon_centers != expected_center || report.marts != expected_mart {
            errors.push(format!(
                "regional cell {} has {}/{} Center/Mart facades, expected {}/{}",
                cell.cell, report.pokemon_centers, report.marts, expected_center, expected_mart
            ));
        }
        minimum_principal = minimum_principal.min(report.principal_route_percent);
        if report.route_cells > 0 && report.principal_route_percent + 1e-9 < 85.0 {
            errors.push(format!(
                "regional cell {} principal route contains only {:.1}% of route cells; expected at least 85%",
                cell.cell, report.principal_route_percent
            ));
        }
        for connection in &cell.connections {
            if !report.connected_edges.contains(&connection.edge_id) {
                errors.push(format!(
                    "regional edge {} does not reach the principal route in cell {}",
                    connection.edge_id, cell.cell
                ));
            }
        }
        if !report.leaking_closed_edges.is_empty() {
            errors.push(format!(
                "regional cell {} leaves {} unselected transport crossings open",
                cell.cell,
                report.leaking_closed_edges.len()
            ));
        }
        if !report.leaking_boundary_route_cells.is_empty() {
            errors.push(format!(
                "regional cell {} exposes {} route cells on undeclared H3 exits",
                cell.cell,
                report.leaking_boundary_route_cells.len()
            ));
        }
    }
    if centers != plan.pokemon_centers || marts != plan.marts {
        errors.push(format!(
            "regional batch contains {centers}/{marts} Center/Mart facades, planned {}/{}",
            plan.pokemon_centers, plan.marts
        ));
    }

    let mut by_edge = BTreeMap::<&str, Vec<&H3RegionalConnection>>::new();
    let mut boundary_by_edge = BTreeMap::<&str, Vec<&H3RegionalConnection>>::new();
    for cell in &plan.cells {
        for connection in &cell.connections {
            if connection.boundary_exit {
                boundary_by_edge
                    .entry(connection.edge_id.as_str())
                    .or_default()
                    .push(connection);
            } else {
                by_edge
                    .entry(connection.edge_id.as_str())
                    .or_default()
                    .push(connection);
            }
        }
    }
    for (edge_id, entries) in &by_edge {
        if entries.len() != 2
            || entries[0].neighbor == entries[1].neighbor
            || entries[0].transport != entries[1].transport
            || entries[0].authoritative != entries[1].authoritative
            || !coordinates_match(Some(entries[0].coordinate), Some(entries[1].coordinate))
        {
            errors.push(format!(
                "regional internal edge {edge_id} is not an exact reciprocal pair"
            ));
        }
    }
    if by_edge.len() != plan.internal_connections {
        errors.push(format!(
            "regional plan reports {} internal connections but contains {} reciprocal edge ids",
            plan.internal_connections,
            by_edge.len()
        ));
    }
    for (edge_id, entries) in &boundary_by_edge {
        if entries.len() != 1 {
            errors.push(format!(
                "regional boundary edge {edge_id} has {} directives instead of one",
                entries.len()
            ));
        }
    }
    if boundary_by_edge.len() != plan.boundary_exits {
        errors.push(format!(
            "regional plan reports {} boundary exits but contains {} unique boundary edge ids",
            plan.boundary_exits,
            boundary_by_edge.len()
        ));
    }
    let authoritative_internal = by_edge
        .values()
        .filter(|entries| entries.first().is_some_and(|entry| entry.authoritative))
        .count();
    let synthetic_internal = by_edge.len() - authoritative_internal;
    let authoritative_boundary = boundary_by_edge
        .values()
        .filter(|entries| entries.first().is_some_and(|entry| entry.authoritative))
        .count();
    let synthetic_boundary = boundary_by_edge.len() - authoritative_boundary;
    if authoritative_internal != plan.authoritative_internal_connections
        || synthetic_internal != plan.synthetic_internal_connections
        || authoritative_boundary != plan.authoritative_boundary_exits
        || synthetic_boundary != plan.synthetic_boundary_exits
    {
        errors.push(format!(
            "regional authoritative/synthetic connection counts are {authoritative_internal}/{synthetic_internal} internal and {authoritative_boundary}/{synthetic_boundary} boundary, not planned {}/{}, {}/{}",
            plan.authoritative_internal_connections,
            plan.synthetic_internal_connections,
            plan.authoritative_boundary_exits,
            plan.synthetic_boundary_exits
        ));
    }
    let authoritative_connections = authoritative_internal + authoritative_boundary;
    let synthetic_connections = synthetic_internal + synthetic_boundary;
    if synthetic_connections > plan.synthetic_connection_budget {
        errors.push(format!(
            "regional graph uses {synthetic_connections} synthetic connections, exceeding its budget of {}",
            plan.synthetic_connection_budget
        ));
    }
    let ordinal_by_cell = plan
        .cells
        .iter()
        .map(|cell| (cell.cell.as_str(), cell.ordinal))
        .collect::<BTreeMap<_, _>>();
    if !plan.cells.is_empty() {
        let mut reached = BTreeSet::from([0usize]);
        let mut queue = VecDeque::from([0usize]);
        while let Some(ordinal) = queue.pop_front() {
            for connection in plan.cells[ordinal]
                .connections
                .iter()
                .filter(|connection| !connection.boundary_exit)
            {
                if let Some(&neighbor) = ordinal_by_cell.get(connection.neighbor.as_str())
                    && reached.insert(neighbor)
                {
                    queue.push_back(neighbor);
                }
            }
        }
        if reached.len() != plan.cells.len() {
            errors.push(format!(
                "regional transport graph reaches {} of {} H3 cells",
                reached.len(),
                plan.cells.len()
            ));
        }
    }
    for cell in &plan.cells {
        if cell.connections.len() > 4 {
            errors.push(format!(
                "regional cell {} exposes {} portals; expected at most four",
                cell.cell,
                cell.connections.len()
            ));
        }
    }

    H3RegionalAudit {
        passed: errors.is_empty(),
        cells: reports.len(),
        pokemon_centers: centers,
        marts,
        internal_connections: by_edge.len(),
        boundary_exits: boundary_by_edge.len(),
        authoritative_connections,
        synthetic_connections,
        synthetic_connection_budget: plan.synthetic_connection_budget,
        minimum_principal_route_percent: if reports.is_empty() {
            0.0
        } else {
            minimum_principal
        },
        errors,
    }
}

fn select_edge(edge: &EdgeCandidate, selected: &mut BTreeSet<String>, degrees: &mut [usize]) {
    if selected.insert(edge.edge_id.clone()) {
        degrees[edge.first] += 1;
        degrees[edge.second.expect("selected internal edge")] += 1;
    }
}

fn select_feasible_spanning_tree(
    internal: &BTreeMap<String, EdgeCandidate>,
    cell_count: usize,
) -> Result<(BTreeSet<String>, Vec<usize>)> {
    if cell_count == 0 {
        return Ok((BTreeSet::new(), Vec::new()));
    }
    if cell_count > 63 {
        bail!("regional spanning-tree search supports at most 63 cells");
    }
    let edges = internal.values().collect::<Vec<_>>();
    let all_reached = (1_u64 << cell_count) - 1;
    let mut degrees = vec![0usize; cell_count];
    let mut selected = Vec::<String>::with_capacity(cell_count.saturating_sub(1));
    let mut failed_states = BTreeSet::<(u64, Vec<usize>)>::new();

    fn search(
        edges: &[&EdgeCandidate],
        all_reached: u64,
        reached: u64,
        degrees: &mut [usize],
        selected: &mut Vec<String>,
        failed_states: &mut BTreeSet<(u64, Vec<usize>)>,
    ) -> bool {
        if reached == all_reached {
            return true;
        }
        let state = (reached, degrees.to_vec());
        if failed_states.contains(&state) {
            return false;
        }
        let mut frontier = edges
            .iter()
            .copied()
            .filter_map(|edge| {
                let second = edge.second.expect("internal edge");
                let first_reached = reached & (1_u64 << edge.first) != 0;
                let second_reached = reached & (1_u64 << second) != 0;
                if first_reached == second_reached
                    || degrees[edge.first] >= 4
                    || degrees[second] >= 4
                {
                    return None;
                }
                let (parent, child) = if first_reached {
                    (edge.first, second)
                } else {
                    (second, edge.first)
                };
                let onward = edges
                    .iter()
                    .filter(|candidate| {
                        let other = candidate.second.expect("internal edge");
                        (candidate.first == child && reached & (1_u64 << other) == 0)
                            || (other == child && reached & (1_u64 << candidate.first) == 0)
                    })
                    .count();
                Some((edge, parent, child, onward))
            })
            .collect::<Vec<_>>();
        frontier.sort_by_key(|(edge, parent, child, onward)| {
            (
                !edge.authoritative,
                *onward,
                degrees[*parent],
                std::cmp::Reverse(transport_priority(edge.transport)),
                *child,
                edge.edge_id.clone(),
            )
        });
        for (edge, _parent, child, _) in frontier {
            let second = edge.second.expect("internal edge");
            degrees[edge.first] += 1;
            degrees[second] += 1;
            selected.push(edge.edge_id.clone());
            if search(
                edges,
                all_reached,
                reached | (1_u64 << child),
                degrees,
                selected,
                failed_states,
            ) {
                return true;
            }
            selected.pop();
            degrees[edge.first] -= 1;
            degrees[second] -= 1;
        }
        failed_states.insert(state);
        false
    }

    if !search(
        &edges,
        all_reached,
        1,
        &mut degrees,
        &mut selected,
        &mut failed_states,
    ) {
        let mut topologically_reached = BTreeSet::from([0usize]);
        let mut queue = VecDeque::from([0usize]);
        while let Some(current) = queue.pop_front() {
            for edge in &edges {
                let second = edge.second.expect("internal edge");
                let neighbor = if edge.first == current {
                    Some(second)
                } else if second == current {
                    Some(edge.first)
                } else {
                    None
                };
                if let Some(neighbor) = neighbor
                    && topologically_reached.insert(neighbor)
                {
                    queue.push_back(neighbor);
                }
            }
        }
        bail!(
            "raster-feasible regional transport reaches {} of {} H3 cells and has no connected degree-four spanning tree; blocked water faces cannot be replaced with synthetic Trails",
            topologically_reached.len(),
            cell_count
        );
    }
    Ok((selected.into_iter().collect(), degrees))
}

fn add_connection(cells: &mut [H3RegionalCellPlan], edge: &EdgeCandidate, boundary_exit: bool) {
    cells[edge.first].connections.push(H3RegionalConnection {
        edge_id: edge.edge_id.clone(),
        neighbor: edge.first_neighbor.clone(),
        coordinate: edge.coordinate,
        transport: edge.transport,
        bridge: edge.bridge,
        authoritative: edge.authoritative,
        boundary_exit,
    });
    if let Some(second) = edge.second {
        cells[second].connections.push(H3RegionalConnection {
            edge_id: edge.edge_id.clone(),
            neighbor: edge
                .second_neighbor
                .clone()
                .expect("internal edge has reciprocal neighbor"),
            coordinate: edge.coordinate,
            transport: edge.transport,
            bridge: edge.bridge,
            authoritative: edge.authoritative,
            boundary_exit,
        });
    }
}

fn select_service_cells(
    target: usize,
    density: &[usize],
    distances: &[Vec<usize>],
    required: Option<usize>,
    avoid: &BTreeSet<usize>,
) -> BTreeSet<usize> {
    let mut selected = required.into_iter().collect::<BTreeSet<_>>();
    while selected.len() < target.min(density.len()) {
        let candidate = (0..density.len())
            .filter(|ordinal| !selected.contains(ordinal))
            .max_by_key(|&ordinal| {
                let separation = selected
                    .iter()
                    .map(|&other| distances[ordinal][other])
                    .min()
                    .unwrap_or(usize::MAX / 4)
                    .min(4);
                (
                    usize::from(!avoid.contains(&ordinal)),
                    separation,
                    density[ordinal],
                    std::cmp::Reverse(ordinal),
                )
            });
        let Some(candidate) = candidate else {
            break;
        };
        selected.insert(candidate);
    }
    selected
}

fn graph_distances(manifest: &H3BatchManifest) -> Vec<Vec<usize>> {
    let by_cell = manifest
        .cells
        .iter()
        .map(|entry| (entry.plan.cell.as_str(), entry.ordinal))
        .collect::<BTreeMap<_, _>>();
    let mut adjacency = vec![Vec::new(); manifest.cells.len()];
    for entry in &manifest.cells {
        for portal in &entry.plan.portals {
            if let Some(&neighbor) = by_cell.get(portal.neighbor.as_str()) {
                adjacency[entry.ordinal].push(neighbor);
            }
        }
    }
    (0..manifest.cells.len())
        .map(|start| {
            let mut distance = vec![usize::MAX; manifest.cells.len()];
            let mut queue = VecDeque::from([start]);
            distance[start] = 0;
            while let Some(current) = queue.pop_front() {
                for &next in &adjacency[current] {
                    if distance[next] == usize::MAX {
                        distance[next] = distance[current] + 1;
                        queue.push_back(next);
                    }
                }
            }
            distance
        })
        .collect()
}

fn transport_priority(kind: FeatureKind) -> u8 {
    match kind {
        FeatureKind::MajorRoad => 4,
        FeatureKind::Road => 3,
        FeatureKind::Street => 2,
        FeatureKind::Trail => 1,
        _ => 0,
    }
}

fn synthetic_connection_budget(cells: usize) -> usize {
    if cells == 1 {
        // Standalone proofs have no in-batch neighbor and retain two useful
        // deterministic exits even when OSM has no crossing at the crop.
        2
    } else {
        // A regional graph must primarily reflect real mapped transport. Two
        // synthetic links are enough to repair a sparse endpoint without
        // allowing an invented trail lattice to ring every generated face.
        2
    }
}

fn coordinates_match(first: Option<Coordinate>, second: Option<Coordinate>) -> bool {
    match (first, second) {
        (Some(first), Some(second)) => {
            let longitude_delta = (first.lon - second.lon + 180.0).rem_euclid(360.0) - 180.0;
            (first.lat - second.lat).abs() <= 1e-8 && longitude_delta.abs() <= 1e-8
        }
        (None, None) => true,
        _ => false,
    }
}

fn route_components(grid: &GeneratedGrid) -> Vec<Vec<usize>> {
    let width = usize::from(grid.width);
    let mut unseen = grid.cells.iter().copied().map(is_route).collect::<Vec<_>>();
    let mut components = Vec::new();
    for start in 0..unseen.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(index) = queue.pop_front() {
            component.push(index);
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
                if unseen[next] {
                    unseen[next] = false;
                    queue.push_back(next);
                }
            }
        }
        components.push(component);
    }
    components
}

fn projected_connection_cells(grid: &GeneratedGrid, coordinate: Coordinate) -> Result<Vec<usize>> {
    let plan = grid.source.h3.as_ref().context("missing H3 plan")?;
    let width = usize::from(grid.width);
    Ok(crate::h3::h3_raster_sample_band(plan, grid, coordinate)?
        .into_iter()
        .map(|(x, y)| usize::from(y) * width + usize::from(x))
        .collect())
}

fn complete_facilities(grid: &GeneratedGrid, facility: H3Facility) -> usize {
    let pattern = match facility {
        H3Facility::PokemonCenter => [
            MapCell::PokecenterNorthWest,
            MapCell::PokecenterNorthEast,
            MapCell::PokecenterSouthWest,
            MapCell::PokecenterSouthEast,
        ],
        H3Facility::Mart => [
            MapCell::MartNorthWest,
            MapCell::MartNorthEast,
            MapCell::MartSouthWest,
            MapCell::MartSouthEast,
        ],
    };
    let mut count = 0;
    for y in 0..grid.height.saturating_sub(1) {
        for x in 0..grid.width.saturating_sub(1) {
            if grid.cell(x, y) == Some(pattern[0])
                && grid.cell(x + 1, y) == Some(pattern[1])
                && grid.cell(x, y + 1) == Some(pattern[2])
                && grid.cell(x + 1, y + 1) == Some(pattern[3])
            {
                count += 1;
            }
        }
    }
    count
}

fn is_route(cell: MapCell) -> bool {
    matches!(
        cell,
        MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BoundingBox, Feature, H3EdgeContract, H3EdgeTerrain, H3TransportCrossing,
        attach_h3_regional_plan, build_h3_seam_contract, generate_grid, plan_h3_batch,
        prepare_h3_source,
    };

    const MINNEAPOLIS: Coordinate = Coordinate {
        lat: 44.947_519_6,
        lon: -93.325_347_7,
    };

    #[test]
    fn nineteen_cells_share_three_of_each_service_and_one_sparse_road_graph() {
        let manifest = plan_h3_batch(MINNEAPOLIS, 6, 19).expect("19-cell manifest");
        let sources = manifest
            .cells
            .iter()
            .map(|entry| {
                let mut features = (0..(entry.ordinal + 1))
                    .map(|index| Feature {
                        kind: FeatureKind::Building,
                        name: Some(format!("building-{index}")),
                        area: true,
                        bridge: false,
                        points: vec![entry.plan.center],
                    })
                    .collect::<Vec<_>>();
                features.extend(entry.plan.portals.iter().map(|portal| Feature {
                    kind: FeatureKind::Road,
                    name: Some(format!("road-{}", portal.edge_id)),
                    area: false,
                    bridge: false,
                    points: vec![entry.plan.center, portal.midpoint],
                }));
                prepare_h3_source(
                    MapSource {
                        center: entry.plan.center,
                        bounds: entry.plan.fetch_bounds[0],
                        attribution: "regional fixture".to_string(),
                        features,
                        h3: None,
                    },
                    entry.plan.clone(),
                )
                .expect("prepare raw regional fixture")
            })
            .collect::<Vec<_>>();
        let contracts = manifest
            .cells
            .iter()
            .map(|entry| H3SeamContract {
                cell: entry.plan.cell.clone(),
                edges: entry
                    .plan
                    .portals
                    .iter()
                    .map(|portal| H3EdgeContract {
                        edge_id: portal.edge_id.clone(),
                        neighbor: portal.neighbor.clone(),
                        side: portal.side,
                        terrain: H3EdgeTerrain::Grass,
                        transport: Some(FeatureKind::Road),
                        crossing: Some(portal.midpoint),
                        viable_crossings: vec![H3TransportCrossing {
                            transport: FeatureKind::Road,
                            coordinate: portal.midpoint,
                            bridge: false,
                        }],
                        synthetic_traversable: false,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let regional = plan_h3_region(&manifest, &sources, &contracts).expect("regional plan");
        assert_eq!(regional.pokemon_centers, 3);
        assert_eq!(regional.marts, 3);
        assert_eq!(
            regional
                .cells
                .iter()
                .filter(|cell| cell.facilities.contains(&H3Facility::PokemonCenter))
                .count(),
            3
        );
        assert_eq!(
            regional
                .cells
                .iter()
                .filter(|cell| cell.facilities.contains(&H3Facility::Mart))
                .count(),
            3
        );
        assert!(
            regional.cells[0]
                .facilities
                .contains(&H3Facility::PokemonCenter)
        );
        assert!(
            regional
                .cells
                .iter()
                .all(|cell| (2..=4).contains(&cell.connections.len()))
        );
        assert_eq!(
            regional.internal_connections,
            18 + regional.loop_connections
        );
        assert_eq!(regional.synthetic_internal_connections, 0);
        assert_eq!(regional.synthetic_boundary_exits, 0);
        assert_eq!(
            regional.authoritative_internal_connections,
            regional.internal_connections
        );
        assert_eq!(
            regional.authoritative_boundary_exits,
            regional.boundary_exits
        );
        let runtime = build_h3_regional_connections(&manifest, &regional, 64, 64)
            .expect("regional runtime links");
        assert_eq!(runtime.links.len(), regional.internal_connections);
        for link in &runtime.links {
            let first = &regional.cells[link.first_ordinal];
            let second = &regional.cells[link.second_ordinal];
            let first_connection = first
                .connections
                .iter()
                .find(|connection| connection.edge_id == link.edge_id)
                .expect("first selected runtime endpoint");
            let second_connection = second
                .connections
                .iter()
                .find(|connection| connection.edge_id == link.edge_id)
                .expect("second selected runtime endpoint");
            assert_eq!(
                link.first_gate,
                crate::h3::h3_raster_landing(
                    &manifest.cells[link.first_ordinal].plan,
                    64,
                    64,
                    first_connection.coordinate,
                )
                .expect("first exact landing")
            );
            assert_eq!(
                link.second_gate,
                crate::h3::h3_raster_landing(
                    &manifest.cells[link.second_ordinal].plan,
                    64,
                    64,
                    second_connection.coordinate,
                )
                .expect("second exact landing")
            );
        }

        let mut reciprocal = BTreeMap::<String, usize>::new();
        for connection in regional
            .cells
            .iter()
            .flat_map(|cell| &cell.connections)
            .filter(|connection| !connection.boundary_exit)
        {
            *reciprocal.entry(connection.edge_id.clone()).or_default() += 1;
        }
        assert_eq!(reciprocal.len(), regional.internal_connections);
        assert!(reciprocal.values().all(|count| *count == 2));

        let ordinal_by_cell = regional
            .cells
            .iter()
            .map(|cell| (cell.cell.as_str(), cell.ordinal))
            .collect::<BTreeMap<_, _>>();
        let mut reached = BTreeSet::from([0usize]);
        let mut frontier = VecDeque::from([0usize]);
        while let Some(ordinal) = frontier.pop_front() {
            for connection in regional.cells[ordinal]
                .connections
                .iter()
                .filter(|connection| !connection.boundary_exit)
            {
                let neighbor = ordinal_by_cell[connection.neighbor.as_str()];
                if reached.insert(neighbor) {
                    frontier.push_back(neighbor);
                }
            }
        }
        assert_eq!(
            reached.len(),
            19,
            "the reciprocal road graph must connect all 19 H3 rooms"
        );

        let raster_ordinal = regional
            .cells
            .iter()
            .find(|cell| cell.facilities.is_empty() && cell.connections.len() >= 2)
            .map(|cell| cell.ordinal)
            .expect("19-cell plan has a service-free raster fixture");
        let mut raster_source = sources[raster_ordinal].clone();
        attach_h3_regional_plan(
            &mut raster_source,
            regional.cells[raster_ordinal].clone(),
            64,
            64,
        )
        .expect("attach regional raster directives");
        let grid = generate_grid(raster_source, 64, 64).expect("generate regional H3 raster");
        let report = inspect_h3_regional_grid(&grid).expect("inspect generated regional raster");
        assert_eq!(report.pokemon_centers, 0);
        assert_eq!(report.marts, 0);
        assert!(
            report.principal_route_percent >= 85.0,
            "principal route coverage was {:.1}%",
            report.principal_route_percent
        );
        assert!(
            report.leaking_closed_edges.is_empty(),
            "closed regional crossings leaked in {}: {:?}",
            report.cell,
            report.leaking_closed_edges
        );
        assert!(
            regional.cells[raster_ordinal]
                .connections
                .iter()
                .all(|connection| report.connected_edges.contains(&connection.edge_id))
        );
    }

    #[test]
    fn one_cell_preserves_standalone_service_coverage() {
        let manifest = plan_h3_batch(MINNEAPOLIS, 6, 1).expect("one-cell manifest");
        let source = prepare_h3_source(
            MapSource {
                center: manifest.cells[0].plan.center,
                bounds: BoundingBox::square_miles_around(MINNEAPOLIS, 1.0).expect("bounds"),
                attribution: "one-cell fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            manifest.cells[0].plan.clone(),
        )
        .expect("prepare one-cell source");
        let contract = H3SeamContract {
            cell: manifest.origin.clone(),
            edges: manifest.cells[0]
                .plan
                .portals
                .iter()
                .map(|portal| H3EdgeContract {
                    edge_id: portal.edge_id.clone(),
                    neighbor: portal.neighbor.clone(),
                    side: portal.side,
                    terrain: H3EdgeTerrain::Grass,
                    transport: None,
                    crossing: None,
                    viable_crossings: Vec::new(),
                    synthetic_traversable: true,
                })
                .collect(),
        };
        let regional = plan_h3_region(&manifest, &[source], &[contract]).expect("regional plan");
        assert_eq!(
            regional.cells[0].facilities,
            vec![H3Facility::PokemonCenter, H3Facility::Mart]
        );
        assert_eq!(regional.cells[0].connections.len(), 2);
        assert_eq!(regional.synthetic_boundary_exits, 2);
        assert_eq!(regional.synthetic_connection_budget, 2);
    }

    #[test]
    fn water_rejected_triangle_edge_is_omitted_without_a_synthetic_replacement() {
        let manifest = plan_h3_batch(MINNEAPOLIS, 6, 3).expect("three-cell manifest");
        let by_cell = manifest
            .cells
            .iter()
            .map(|entry| (entry.plan.cell.as_str(), entry.ordinal))
            .collect::<BTreeMap<_, _>>();
        let mut internal = BTreeMap::<String, (usize, usize, crate::H3Portal)>::new();
        for entry in &manifest.cells {
            for portal in &entry.plan.portals {
                let Some(&neighbor) = by_cell.get(portal.neighbor.as_str()) else {
                    continue;
                };
                if entry.ordinal < neighbor {
                    internal.insert(
                        portal.edge_id.clone(),
                        (entry.ordinal, neighbor, portal.clone()),
                    );
                }
            }
        }
        assert_eq!(internal.len(), 3, "three-cell proof must form one triangle");
        let (blocked_edge, &(blocked_first, blocked_second, ref blocked_portal)) = internal
            .iter()
            .find(|(_, (first, second, _))| *first != 0 && *second != 0)
            .expect("triangle edge opposite origin");
        let interpolate = |start: Coordinate, end: Coordinate, amount: f64| Coordinate {
            lat: start.lat + (end.lat - start.lat) * amount,
            lon: start.lon + (end.lon - start.lon) * amount,
        };
        let edge_start = interpolate(blocked_portal.boundary[0], blocked_portal.boundary[1], 0.30);
        let edge_end = interpolate(blocked_portal.boundary[0], blocked_portal.boundary[1], 0.70);
        let first_center = manifest.cells[blocked_first].plan.center;
        let second_center = manifest.cells[blocked_second].plan.center;
        let water = Feature {
            kind: FeatureKind::Water,
            name: Some("reciprocal crossing ponds".to_string()),
            area: true,
            bridge: false,
            points: vec![
                interpolate(edge_start, first_center, 0.15),
                interpolate(edge_end, first_center, 0.15),
                interpolate(edge_end, second_center, 0.15),
                interpolate(edge_start, second_center, 0.15),
                interpolate(edge_start, first_center, 0.15),
            ],
        };
        let roads = internal
            .iter()
            .map(|(edge_id, (first, second, _))| Feature {
                kind: FeatureKind::MajorRoad,
                name: Some(format!("road-{edge_id}")),
                area: false,
                bridge: false,
                points: vec![
                    manifest.cells[*first].plan.center,
                    manifest.cells[*second].plan.center,
                ],
            })
            .collect::<Vec<_>>();
        let sources = manifest
            .cells
            .iter()
            .map(|entry| {
                prepare_h3_source(
                    MapSource {
                        center: entry.plan.center,
                        bounds: entry.plan.fetch_bounds[0],
                        attribution: "water-blocked triangle fixture".to_string(),
                        features: roads
                            .iter()
                            .cloned()
                            .chain(std::iter::once(water.clone()))
                            .collect(),
                        h3: None,
                    },
                    entry.plan.clone(),
                )
                .expect("prepare water-blocked triangle source")
            })
            .collect::<Vec<_>>();
        let contracts = manifest
            .cells
            .iter()
            .zip(&sources)
            .map(|(entry, source)| {
                build_h3_seam_contract(&entry.plan, source, 64, 64).expect("grid-aware contract")
            })
            .collect::<Vec<_>>();
        for ordinal in [blocked_first, blocked_second] {
            let edge = contracts[ordinal]
                .edges
                .iter()
                .find(|edge| edge.edge_id == *blocked_edge)
                .expect("blocked reciprocal face");
            assert!(edge.viable_crossings.is_empty());
            assert!(!edge.synthetic_traversable);
        }

        let regional = plan_h3_region(&manifest, &sources, &contracts)
            .expect("alternate two-edge spanning tree");
        assert_eq!(regional.internal_connections, 2);
        assert_eq!(regional.loop_connections, 0);
        assert_eq!(regional.synthetic_internal_connections, 0);
        assert!(regional.cells.iter().all(|cell| {
            cell.connections
                .iter()
                .all(|connection| connection.edge_id != *blocked_edge)
        }));
        assert!(regional.cells.iter().all(|cell| {
            cell.closed_transport_crossings
                .iter()
                .all(|crossing| crossing.edge_id != *blocked_edge)
        }));
    }

    #[test]
    fn sparse_source_cannot_invent_an_unbounded_regional_trail_graph() {
        let manifest = plan_h3_batch(MINNEAPOLIS, 6, 7).expect("seven-cell manifest");
        let sources = manifest
            .cells
            .iter()
            .map(|entry| {
                prepare_h3_source(
                    MapSource {
                        center: entry.plan.center,
                        bounds: entry.plan.fetch_bounds[0],
                        attribution: "no-road fixture".to_string(),
                        features: Vec::new(),
                        h3: None,
                    },
                    entry.plan.clone(),
                )
                .expect("prepare sparse source")
            })
            .collect::<Vec<_>>();
        let contracts = manifest
            .cells
            .iter()
            .map(|entry| H3SeamContract {
                cell: entry.plan.cell.clone(),
                edges: entry
                    .plan
                    .portals
                    .iter()
                    .map(|portal| H3EdgeContract {
                        edge_id: portal.edge_id.clone(),
                        neighbor: portal.neighbor.clone(),
                        side: portal.side,
                        terrain: H3EdgeTerrain::Grass,
                        transport: None,
                        crossing: None,
                        viable_crossings: Vec::new(),
                        synthetic_traversable: true,
                    })
                    .collect(),
            })
            .collect::<Vec<_>>();
        let error = plan_h3_region(&manifest, &sources, &contracts)
            .expect_err("six invented inter-cell trails exceed a seven-cell budget");
        assert!(error.to_string().contains("synthetic Trail connections"));
    }
}
