use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, bail};
use h3o::{CellIndex, LatLng, Resolution};
use serde::{Deserialize, Serialize};

use crate::{
    BoundingBox, Coordinate, FeatureKind, GeneratedGrid, MapCell, MapSource, fetch_map_bounds,
};

pub const H3_MANIFEST_SCHEMA_VERSION: u32 = 2;
pub const H3_SOURCE_SCHEMA_VERSION: u32 = 1;
pub const MAX_INITIAL_H3_CELLS: usize = 5_000;
const H3_GRID_SEAM_SAMPLES: usize = 31;

/// One of the six visual exits on a pointy-top Crystal room.
///
/// This is a local presentation slot, not an H3 direction. H3 direction digits
/// rotate across icosahedron faces and lose one direction at pentagons. Runtime
/// links therefore resolve through `H3Portal::edge_id`, never by assuming that
/// the same enum discriminant is meaningful in a neighboring cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HexSide {
    North,
    NorthEast,
    SouthEast,
    South,
    SouthWest,
    NorthWest,
}

impl HexSide {
    const ALL: [Self; 6] = [
        Self::North,
        Self::NorthEast,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::NorthWest,
    ];

    fn bearing(self) -> f64 {
        match self {
            Self::North => 0.0,
            Self::NorthEast => 60.0,
            Self::SouthEast => 120.0,
            Self::South => 180.0,
            Self::SouthWest => 240.0,
            Self::NorthWest => 300.0,
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::NorthEast => Self::SouthWest,
            Self::SouthEast => Self::NorthWest,
            Self::South => Self::North,
            Self::SouthWest => Self::NorthEast,
            Self::NorthWest => Self::SouthEast,
        }
    }

    /// Stable gate cell for a rectangular Crystal raster that represents an
    /// H3 hex. Corners remain available for scenery while all six exits have a
    /// distinct, reproducible approach.
    pub fn gate(self, width: u16, height: u16, inset: u16) -> Result<(u16, u16)> {
        if width < 16 || height < 16 || inset == 0 || inset * 2 >= width.min(height) {
            bail!("H3 Crystal rasters need dimensions >=16 and a positive interior inset");
        }
        let east = width - 1 - inset;
        let south = height - 1 - inset;
        Ok(match self {
            Self::North => (width / 2, inset),
            Self::NorthEast => (east, height / 4),
            Self::SouthEast => (east, height * 3 / 4),
            Self::South => (width / 2, south),
            Self::SouthWest => (inset, height * 3 / 4),
            Self::NorthWest => (inset, height / 4),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3Portal {
    /// Canonical undirected edge identity shared verbatim by both cells.
    pub edge_id: String,
    pub neighbor: String,
    pub side: HexSide,
    pub midpoint: Coordinate,
    pub boundary: Vec<Coordinate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H3EdgeTerrain {
    Grass,
    TallGrass,
    Trees,
    RockTerrace,
    FenceGate,
    Water,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3TransportCrossing {
    pub transport: FeatureKind,
    pub coordinate: Coordinate,
    /// True only when the exact OSM way carrying this crossing has
    /// `bridge=yes`. Adjoining or layered ways never confer bridge authority.
    pub bridge: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3EdgeContract {
    pub edge_id: String,
    pub neighbor: String,
    pub side: HexSide,
    pub terrain: H3EdgeTerrain,
    /// Present only when authoritative linear OSM geometry crosses the shared
    /// edge. A natural edge never receives a synthetic perimeter road.
    pub transport: Option<FeatureKind>,
    pub crossing: Option<Coordinate>,
    /// Every source crossing that is usable by both the geographic line and
    /// this raster face. Keeping the full ordered set lets the regional
    /// planner choose one exact reciprocal crossing instead of independently
    /// pruning each face to a possibly different road.
    pub viable_crossings: Vec<H3TransportCrossing>,
    /// A source-free gameplay Trail may use this face only when its midpoint
    /// landing is dry and no rejected real crossing tried to claim the edge.
    /// This prevents a road rejected for authoritative water from silently
    /// returning as a synthetic causeway.
    pub synthetic_traversable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3SeamContract {
    pub cell: String,
    pub edges: Vec<H3EdgeContract>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3SeamAudit {
    pub passed: bool,
    pub cells: usize,
    pub internal_edges: usize,
    pub transport_edges: usize,
    pub natural_edges: usize,
    pub errors: Vec<String>,
}

/// The final rendered terrain sampled immediately inside one H3 face.
///
/// This intentionally collapses decorative variants. A shared lake edge only
/// needs to prove that both generated maps retain water (or the same
/// authoritative transport crossing), while the tree-outline check compares
/// the boundary sample with a second sample farther inside the face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H3GridSeamSurface {
    Void,
    Ground,
    Wild,
    Water,
    Transport,
    Tree,
    Relief,
    Fence,
    Fixture,
    Structure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3GridSeamSample {
    pub coordinate: Coordinate,
    pub source_water: bool,
    /// OSM water truth at the center of this face's sampled raster block.
    /// This differs slightly between reciprocal faces and prevents normal
    /// shoreline quantization from being mistaken for invented water.
    pub raster_source_water: bool,
    pub surface: H3GridSeamSurface,
    pub transport: Option<FeatureKind>,
    pub inner_surface: H3GridSeamSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H3GridTransportDirectiveKind {
    Selected,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3GridTransportDirective {
    pub kind: H3GridTransportDirectiveKind,
    pub coordinate: Coordinate,
    /// Exact mapped class promised by a selected connection. Closed crossings
    /// deliberately carry no transport class.
    pub transport: Option<FeatureKind>,
    /// The three cardinally connected final raster cells running inward from
    /// the exact crossing, in boundary-to-interior order.
    pub band_surfaces: Vec<H3GridSeamSurface>,
    /// Per-cell classes retain distinctions collapsed by the semantic seam
    /// surface used for water compatibility.
    pub band_transport: Vec<Option<FeatureKind>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3GridEdgeProfile {
    pub edge_id: String,
    pub neighbor: String,
    pub samples: Vec<H3GridSeamSample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regional_transport: Option<H3GridTransportDirective>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3GridSeamProfile {
    pub cell: String,
    pub grid_width: u16,
    pub grid_height: u16,
    pub edges: Vec<H3GridEdgeProfile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3GridSeamAudit {
    pub passed: bool,
    pub cells: usize,
    pub internal_edges: usize,
    pub samples_per_edge: usize,
    pub reciprocal_surface_samples: usize,
    pub matching_surface_samples: usize,
    pub mismatched_surface_samples: usize,
    pub matching_transport_samples: usize,
    pub mismatched_transport_samples: usize,
    pub authoritative_water_samples: usize,
    pub continuous_water_samples: usize,
    pub tree_outline_edges: usize,
    pub relief_outline_edges: usize,
    pub fence_outline_edges: usize,
    pub artificial_trace_samples: usize,
    pub selected_transport_edges: usize,
    pub connected_transport_edges: usize,
    pub closed_transport_edges: usize,
    pub capped_transport_edges: usize,
    pub errors: Vec<String>,
}

/// A scarce regional service that may be allocated to an H3 room.
///
/// A standalone room still requests both services. Batch generation attaches
/// an [`H3RegionalCellPlan`] and deliberately places services only in the few
/// cells selected by the regional planner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H3Facility {
    PokemonCenter,
    Mart,
}

/// One reciprocal route landing selected by the regional transport graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3RegionalConnection {
    pub edge_id: String,
    pub neighbor: String,
    pub coordinate: Coordinate,
    pub transport: FeatureKind,
    /// True only when the exact selected OSM line has `bridge=yes`. This flag
    /// is carried into final raster authoring so authoritative water may be
    /// overlaid only by a proven bridge trace.
    #[serde(default)]
    pub bridge: bool,
    /// True when the landing comes from an OSM line crossing. False is a
    /// sparse gameplay trail used only when the selected cells otherwise have
    /// no connected geographic transport graph.
    pub authoritative: bool,
    pub boundary_exit: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3ClosedTransportCrossing {
    pub edge_id: String,
    pub coordinate: Coordinate,
}

/// Batch-only directives consumed while authoring one H3 room.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3RegionalCellPlan {
    pub ordinal: usize,
    pub cell: String,
    pub building_count: usize,
    pub facilities: Vec<H3Facility>,
    pub connections: Vec<H3RegionalConnection>,
    /// Real OSM crossings intentionally omitted from the sparse regional
    /// graph. The boundary author caps these fragments before the face edge so
    /// six unrelated road stubs cannot surround every room.
    pub closed_transport_crossings: Vec<H3ClosedTransportCrossing>,
}

/// Lifecycle stage of the normalized geometry embedded in an H3 source.
///
/// Legacy sources deserialize as `Unknown` and are never resume-authorized.
/// A regional cache must be `PreparedRaw`: transport alternatives are still
/// present and feasibility can be recomputed for any requested raster size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum H3SourceStage {
    Unknown,
    Planned,
    PreparedRaw,
    StandaloneReduced,
    RegionalReduced,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct H3SourceProvenance {
    pub schema_version: u32,
    pub stage: H3SourceStage,
    /// Reduced sources are bound to the raster that controlled selection.
    /// Raw prepared geometry is intentionally dimension-independent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_width: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid_height: Option<u16>,
}

impl Default for H3SourceProvenance {
    fn default() -> Self {
        Self {
            schema_version: 0,
            stage: H3SourceStage::Unknown,
            grid_width: None,
            grid_height: None,
        }
    }
}

impl H3SourceProvenance {
    fn planned() -> Self {
        Self {
            schema_version: H3_SOURCE_SCHEMA_VERSION,
            stage: H3SourceStage::Planned,
            grid_width: None,
            grid_height: None,
        }
    }

    fn prepared_raw() -> Self {
        Self {
            schema_version: H3_SOURCE_SCHEMA_VERSION,
            stage: H3SourceStage::PreparedRaw,
            grid_width: None,
            grid_height: None,
        }
    }

    fn reduced(stage: H3SourceStage, grid_width: u16, grid_height: u16) -> Self {
        Self {
            schema_version: H3_SOURCE_SCHEMA_VERSION,
            stage,
            grid_width: Some(grid_width),
            grid_height: Some(grid_height),
        }
    }

    pub fn is_prepared_raw(self) -> bool {
        self.schema_version == H3_SOURCE_SCHEMA_VERSION
            && self.stage == H3SourceStage::PreparedRaw
            && self.grid_width.is_none()
            && self.grid_height.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3CellPlan {
    pub cell: String,
    pub resolution: u8,
    pub center: Coordinate,
    pub boundary: Vec<Coordinate>,
    /// One box normally, two at the antimeridian. Fetchers must query every
    /// box and merge by stable OSM identity before generation.
    pub fetch_bounds: Vec<BoundingBox>,
    pub is_pentagon: bool,
    pub portals: Vec<H3Portal>,
    #[serde(default)]
    pub source_provenance: H3SourceProvenance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regional: Option<H3RegionalCellPlan>,
}

impl H3CellPlan {
    pub fn requests_facility(&self, facility: H3Facility) -> bool {
        self.regional
            .as_ref()
            .is_none_or(|regional| regional.facilities.contains(&facility))
    }

    pub fn index(&self) -> Result<CellIndex> {
        self.cell
            .parse::<CellIndex>()
            .with_context(|| format!("invalid H3 cell {}", self.cell))
    }

    /// H3 itself is the ownership authority. A building, facility anchor, or
    /// other indivisible authored stamp belongs to exactly the cell containing
    /// its representative point; neighboring halo fetches may see it but must
    /// not stamp it a second time.
    pub fn owns(&self, coordinate: Coordinate) -> Result<bool> {
        let resolution = Resolution::try_from(self.resolution)
            .with_context(|| format!("invalid H3 resolution {}", self.resolution))?;
        let coordinate = LatLng::new(coordinate.lat, coordinate.lon)
            .context("convert ownership coordinate to H3 latitude/longitude")?;
        Ok(coordinate.to_cell(resolution) == self.index()?)
    }

    /// Project geometry in a cell-centered tangent frame. Unlike longitude
    /// interpolation this stays continuous at the antimeridian and poles, and
    /// halo coordinates are intentionally allowed to fall outside the raster.
    pub fn project_to_grid(
        &self,
        coordinate: Coordinate,
        width: u16,
        height: u16,
    ) -> Result<(i32, i32)> {
        let frame = self.raster_frame(width, height)?;
        let (east, north) = local_tangent(self.center, coordinate);
        Ok((
            ((east - frame.west) / (frame.east - frame.west) * f64::from(width - 1)).round() as i32,
            ((frame.north - north) / (frame.north - frame.south) * f64::from(height - 1)).round()
                as i32,
        ))
    }

    /// Require an entire authored stamp and its clearance to remain inside the
    /// H3 face. This prevents half houses, facilities, fields, and rock relief
    /// from appearing when adjacent hex images are assembled.
    pub fn raster_footprint_fits(
        &self,
        x: i32,
        y: i32,
        footprint_width: u16,
        footprint_height: u16,
        clearance: u16,
        grid_width: u16,
        grid_height: u16,
    ) -> Result<bool> {
        if footprint_width == 0 || footprint_height == 0 {
            return Ok(false);
        }
        let west = x - i32::from(clearance);
        let north = y - i32::from(clearance);
        let east = x + i32::from(footprint_width) - 1 + i32::from(clearance);
        let south = y + i32::from(footprint_height) - 1 + i32::from(clearance);
        if west < 0 || north < 0 || east >= i32::from(grid_width) || south >= i32::from(grid_height)
        {
            return Ok(false);
        }
        let polygon = self.raster_polygon(grid_width, grid_height)?;
        for check_y in north..=south {
            for check_x in west..=east {
                if !point_in_polygon(f64::from(check_x) + 0.5, f64::from(check_y) + 0.5, &polygon) {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub fn raster_polygon(&self, width: u16, height: u16) -> Result<Vec<(f64, f64)>> {
        let frame = self.raster_frame(width, height)?;
        Ok(self
            .boundary
            .iter()
            .map(|&coordinate| {
                let (east, north) = local_tangent(self.center, coordinate);
                (
                    (east - frame.west) / (frame.east - frame.west) * f64::from(width - 1),
                    (frame.north - north) / (frame.north - frame.south) * f64::from(height - 1),
                )
            })
            .collect())
    }

    pub fn raster_contains_cell(&self, x: u16, y: u16, width: u16, height: u16) -> Result<bool> {
        if x >= width || y >= height {
            return Ok(false);
        }
        let polygon = self.raster_polygon(width, height)?;
        Ok(point_in_polygon(
            f64::from(x) + 0.5,
            f64::from(y) + 0.5,
            &polygon,
        ))
    }

    fn raster_frame(&self, width: u16, height: u16) -> Result<RasterFrame> {
        if width < 2 || height < 2 || self.boundary.len() < 5 {
            bail!("invalid H3 raster frame");
        }
        let mut west = f64::INFINITY;
        let mut east = f64::NEG_INFINITY;
        let mut south = f64::INFINITY;
        let mut north = f64::NEG_INFINITY;
        for &coordinate in &self.boundary {
            let (x, y) = local_tangent(self.center, coordinate);
            west = west.min(x);
            east = east.max(x);
            south = south.min(y);
            north = north.max(y);
        }
        if !(west < east && south < north) {
            bail!("degenerate H3 raster frame for cell {}", self.cell);
        }
        Ok(RasterFrame {
            west,
            east,
            south,
            north,
        })
    }
}

/// Attach a cell contract to one halo-fetched source. Linear and areal
/// features remain visible from the halo, all transport alternatives remain
/// available for dimension-aware seam planning, and indivisible building
/// anchors are owned by exactly one H3 cell.
pub fn prepare_h3_source(mut source: MapSource, mut plan: H3CellPlan) -> Result<MapSource> {
    let index = plan.index()?;
    let resolution = index.resolution();
    let mut prepared = Vec::new();
    for feature in std::mem::take(&mut source.features) {
        if feature.points.is_empty() {
            continue;
        }
        if feature.area {
            if !feature_intersects_h3_halo(&plan, &feature) {
                continue;
            }
            if feature.kind == FeatureKind::Building {
                let count = feature.points.len() as f64;
                let representative = Coordinate {
                    lat: feature.points.iter().map(|point| point.lat).sum::<f64>() / count,
                    lon: circular_longitude_mean(feature.points.iter().map(|point| point.lon)),
                };
                if !LatLng::new(representative.lat, representative.lon)
                    .map(|coordinate| coordinate.to_cell(resolution) == index)
                    .unwrap_or(false)
                {
                    continue;
                }
            }
            prepared.push(feature);
        } else {
            prepared.extend(clip_linear_feature_to_halo(&plan, &feature));
        }
    }
    source.features = prepared;
    source.center = plan.center;
    plan.source_provenance = H3SourceProvenance::prepared_raw();
    plan.regional = None;
    source.h3 = Some(plan);
    Ok(source)
}

/// Attach batch-only authoring directives and re-run the deterministic
/// transport reduction against the selected regional edges.
pub fn attach_h3_regional_plan(
    source: &mut MapSource,
    regional: H3RegionalCellPlan,
    grid_width: u16,
    grid_height: u16,
) -> Result<()> {
    let mut plan = source
        .h3
        .take()
        .context("regional H3 directives require a prepared H3 source")?;
    if plan.cell != regional.cell {
        bail!(
            "regional directives for {} cannot be attached to H3 source {}",
            regional.cell,
            plan.cell
        );
    }
    if !plan.source_provenance.is_prepared_raw() {
        bail!(
            "regional H3 directives require raw prepared source schema {}, got {:?}",
            H3_SOURCE_SCHEMA_VERSION,
            plan.source_provenance
        );
    }
    validate_h3_regional_connections(&plan, source, &regional, grid_width, grid_height)?;
    plan.regional = Some(regional);
    compress_h3_transport(&plan, source, grid_width, grid_height)?;
    plan.source_provenance =
        H3SourceProvenance::reduced(H3SourceStage::RegionalReduced, grid_width, grid_height);
    source.h3 = Some(plan);
    Ok(())
}

/// Reduce a prepared standalone H3 source using its exact output dimensions.
/// Regional attachment performs this automatically after validating selected
/// gates; standalone generation calls it explicitly after writing its seam
/// contract.
pub fn finalize_h3_source_transport(
    source: &mut MapSource,
    grid_width: u16,
    grid_height: u16,
) -> Result<()> {
    let plan = source
        .h3
        .clone()
        .context("H3 transport reduction requires a prepared H3 source")?;
    if !plan.source_provenance.is_prepared_raw() {
        bail!(
            "standalone H3 transport reduction requires raw prepared source schema {}, got {:?}",
            H3_SOURCE_SCHEMA_VERSION,
            plan.source_provenance
        );
    }
    compress_h3_transport(&plan, source, grid_width, grid_height)?;
    source
        .h3
        .as_mut()
        .expect("validated H3 source retains its plan")
        .source_provenance =
        H3SourceProvenance::reduced(H3SourceStage::StandaloneReduced, grid_width, grid_height);
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct H3LocalBounds {
    west: f64,
    east: f64,
    south: f64,
    north: f64,
}

fn h3_halo_bounds(plan: &H3CellPlan) -> H3LocalBounds {
    let boundary = plan
        .boundary
        .iter()
        .map(|&point| local_tangent(plan.center, point))
        .collect::<Vec<_>>();
    let west = boundary
        .iter()
        .map(|point| point.0)
        .fold(f64::INFINITY, f64::min);
    let east = boundary
        .iter()
        .map(|point| point.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let south = boundary
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let north = boundary
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let pad_x = (east - west) * 0.18;
    let pad_y = (north - south) * 0.18;
    H3LocalBounds {
        west: west - pad_x,
        east: east + pad_x,
        south: south - pad_y,
        north: north + pad_y,
    }
}

fn clip_linear_feature_to_halo(plan: &H3CellPlan, feature: &crate::Feature) -> Vec<crate::Feature> {
    if feature.points.len() < 2 {
        return Vec::new();
    }
    let bounds = h3_halo_bounds(plan);
    let mut pieces = Vec::new();
    let mut current = Vec::<Coordinate>::new();
    for segment in feature.points.windows(2) {
        let first = local_tangent(plan.center, segment[0]);
        let second = local_tangent(plan.center, segment[1]);
        let intersects = first.0.min(second.0) <= bounds.east
            && first.0.max(second.0) >= bounds.west
            && first.1.min(second.1) <= bounds.north
            && first.1.max(second.1) >= bounds.south;
        if intersects {
            if current.last().copied() != Some(segment[0]) {
                current.push(segment[0]);
            }
            current.push(segment[1]);
        } else if current.len() >= 2 {
            pieces.push(std::mem::take(&mut current));
        } else {
            current.clear();
        }
    }
    if current.len() >= 2 {
        pieces.push(current);
    }
    pieces
        .into_iter()
        .map(|points| crate::Feature {
            points,
            ..feature.clone()
        })
        .collect()
}

#[derive(Debug, Clone)]
struct H3SourceTransportCrossing {
    candidate: H3TransportCrossing,
    feature_index: usize,
    stable_key: u64,
    touches_authoritative_water: bool,
    reaches_stable_interior: bool,
}

impl H3SourceTransportCrossing {
    fn viable(&self) -> bool {
        (self.candidate.bridge || !self.touches_authoritative_water) && self.reaches_stable_interior
    }
}

/// Raster truth needed before selecting a regional transport graph.
///
/// A dry three-cell landing is not sufficient: shoreline quantization can
/// leave it in a tiny pocket whose only apparent escape hugs the rectangular
/// storage edge and is later removed by the H3 face cap. This precomputes the
/// same polygon/line water raster plus authoritative seam restoration used by
/// final generation, excludes the one-cell face fringe, and labels only dry
/// components that reach stable interior land.
#[derive(Debug, Clone)]
struct H3RasterTransportFeasibility {
    width: u16,
    height: u16,
    authoritative_water: Vec<bool>,
    stable_dry_component: Vec<bool>,
}

impl H3RasterTransportFeasibility {
    fn new(plan: &H3CellPlan, source: &MapSource, width: u16, height: u16) -> Result<Self> {
        let authoritative_water = rasterized_h3_authoritative_water(plan, source, width, height)?;
        Self::from_water_mask(plan, width, height, authoritative_water)
    }

    fn from_water_mask(
        plan: &H3CellPlan,
        width: u16,
        height: u16,
        authoritative_water: Vec<bool>,
    ) -> Result<Self> {
        let area = usize::from(width) * usize::from(height);
        if authoritative_water.len() != area {
            bail!(
                "H3 authoritative-water mask has {} cells, expected {area}",
                authoritative_water.len()
            );
        }
        let mut inside = vec![false; area];
        for y in 0..height {
            for x in 0..width {
                inside[usize::from(y) * usize::from(width) + usize::from(x)] =
                    plan.raster_contains_cell(x, y, width, height)?;
            }
        }
        let touches_void = |x: u16, y: u16| {
            [
                (i32::from(x) - 1, i32::from(y)),
                (i32::from(x) + 1, i32::from(y)),
                (i32::from(x), i32::from(y) - 1),
                (i32::from(x), i32::from(y) + 1),
            ]
            .into_iter()
            .any(|(neighbor_x, neighbor_y)| {
                neighbor_x < 0
                    || neighbor_y < 0
                    || neighbor_x >= i32::from(width)
                    || neighbor_y >= i32::from(height)
                    || !inside[neighbor_y as usize * usize::from(width) + neighbor_x as usize]
            })
        };
        let mut base_dry = vec![false; area];
        for y in 0..height {
            for x in 0..width {
                let index = usize::from(y) * usize::from(width) + usize::from(x);
                let touches_water = [
                    (i32::from(x) - 1, i32::from(y)),
                    (i32::from(x) + 1, i32::from(y)),
                    (i32::from(x), i32::from(y) - 1),
                    (i32::from(x), i32::from(y) + 1),
                ]
                .into_iter()
                .any(|(neighbor_x, neighbor_y)| {
                    neighbor_x >= 0
                        && neighbor_y >= 0
                        && neighbor_x < i32::from(width)
                        && neighbor_y < i32::from(height)
                        && authoritative_water
                            [neighbor_y as usize * usize::from(width) + neighbor_x as usize]
                });
                base_dry[index] = inside[index]
                    && !authoritative_water[index]
                    && !touches_void(x, y)
                    && !touches_water;
            }
        }

        let has_face_clearance = |x: u16, y: u16| {
            (-2_i32..=2).all(|delta_y| {
                (-2_i32..=2).all(|delta_x| {
                    let check_x = i32::from(x) + delta_x;
                    let check_y = i32::from(y) + delta_y;
                    check_x >= 0
                        && check_y >= 0
                        && check_x < i32::from(width)
                        && check_y < i32::from(height)
                        && inside[check_y as usize * usize::from(width) + check_x as usize]
                })
            })
        };
        let minimum_stable_cells = usize::from(width.min(height) / 8).max(4);
        let mut unseen = base_dry.clone();
        let mut stable_dry_component = vec![false; area];
        for start in 0..area {
            if !unseen[start] {
                continue;
            }
            unseen[start] = false;
            let mut component = Vec::new();
            let mut queue = VecDeque::from([start]);
            let mut reaches_interior = false;
            while let Some(index) = queue.pop_front() {
                component.push(index);
                let x = index % usize::from(width);
                let y = index / usize::from(width);
                reaches_interior |= has_face_clearance(x as u16, y as u16);
                for (next_x, next_y) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if next_x >= usize::from(width) || next_y >= usize::from(height) {
                        continue;
                    }
                    let next = next_y * usize::from(width) + next_x;
                    if unseen[next] {
                        unseen[next] = false;
                        queue.push_back(next);
                    }
                }
            }
            if reaches_interior && component.len() >= minimum_stable_cells {
                for index in component {
                    stable_dry_component[index] = true;
                }
            }
        }
        Ok(Self {
            width,
            height,
            authoritative_water,
            stable_dry_component,
        })
    }

    fn band_touches_water(&self, plan: &H3CellPlan, coordinate: Coordinate) -> Result<bool> {
        Ok(
            h3_raster_sample_band_for_dimensions(plan, self.width, self.height, coordinate)?
                .into_iter()
                .any(|(x, y)| {
                    self.authoritative_water
                        [usize::from(y) * usize::from(self.width) + usize::from(x)]
                }),
        )
    }

    fn landing_reaches_stable_interior(
        &self,
        plan: &H3CellPlan,
        coordinate: Coordinate,
        exact_transport: Option<&crate::Feature>,
    ) -> Result<bool> {
        let band = h3_raster_sample_band_for_dimensions(plan, self.width, self.height, coordinate)?;
        let exact_bridge = exact_transport.is_some_and(|feature| feature.bridge);
        if !exact_bridge
            && band.iter().any(|&(x, y)| {
                self.authoritative_water[usize::from(y) * usize::from(self.width) + usize::from(x)]
            })
        {
            return Ok(false);
        }
        let mut approach = band.into_iter().collect::<BTreeSet<_>>();
        if let Some(feature) = exact_transport {
            approach.extend(rasterized_h3_linear_feature_cells(
                plan,
                feature,
                self.width,
                self.height,
            )?);
        }
        let mut seen = BTreeSet::new();
        let mut queue = approach
            .iter()
            .copied()
            .filter(|&(x, y)| {
                let index = usize::from(y) * usize::from(self.width) + usize::from(x);
                exact_bridge || !self.authoritative_water[index]
            })
            .collect::<VecDeque<_>>();
        while let Some((x, y)) = queue.pop_front() {
            if !seen.insert((x, y)) {
                continue;
            }
            let index = usize::from(y) * usize::from(self.width) + usize::from(x);
            if self.stable_dry_component[index] {
                return Ok(true);
            }
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= self.width || next_y >= self.height {
                    continue;
                }
                let next = usize::from(next_y) * usize::from(self.width) + usize::from(next_x);
                if self.stable_dry_component[next] {
                    return Ok(true);
                }
                if approach.contains(&(next_x, next_y))
                    && !seen.contains(&(next_x, next_y))
                    && (exact_bridge || !self.authoritative_water[next])
                {
                    queue.push_back((next_x, next_y));
                }
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
fn h3_landing_reaches_stable_interior_for_mask(
    plan: &H3CellPlan,
    width: u16,
    height: u16,
    coordinate: Coordinate,
    authoritative_water: &[bool],
    exact_transport: Option<&crate::Feature>,
) -> Result<bool> {
    H3RasterTransportFeasibility::from_water_mask(
        plan,
        width,
        height,
        authoritative_water.to_vec(),
    )?
    .landing_reaches_stable_interior(plan, coordinate, exact_transport)
}

fn rasterized_h3_authoritative_water(
    plan: &H3CellPlan,
    source: &MapSource,
    width: u16,
    height: u16,
) -> Result<Vec<bool>> {
    let area = usize::from(width) * usize::from(height);
    let mut water = vec![false; area];
    let features = source
        .features
        .iter()
        .filter(|feature| {
            feature.area && feature.kind == FeatureKind::Water && feature.points.len() >= 3
        })
        .collect::<Vec<_>>();
    for feature in &features {
        let polygon = feature
            .points
            .iter()
            .map(|&coordinate| plan.project_to_grid(coordinate, width, height))
            .collect::<Result<Vec<_>>>()?;
        let min_x = polygon
            .iter()
            .map(|point| point.0)
            .min()
            .unwrap_or(0)
            .max(0);
        let max_x = polygon
            .iter()
            .map(|point| point.0)
            .max()
            .unwrap_or(-1)
            .min(i32::from(width) - 1);
        let min_y = polygon
            .iter()
            .map(|point| point.1)
            .min()
            .unwrap_or(0)
            .max(0);
        let max_y = polygon
            .iter()
            .map(|point| point.1)
            .max()
            .unwrap_or(-1)
            .min(i32::from(height) - 1);
        let polygon_float = polygon
            .iter()
            .map(|&(x, y)| (f64::from(x), f64::from(y)))
            .collect::<Vec<_>>();
        if min_x <= max_x && min_y <= max_y {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    if point_in_polygon(f64::from(x) + 0.5, f64::from(y) + 0.5, &polygon_float) {
                        water[y as usize * usize::from(width) + x as usize] = true;
                    }
                }
            }
        }
        for segment in polygon.windows(2) {
            for (x, y) in raster_line_cells(segment[0], segment[1]) {
                if x >= 0 && y >= 0 && x < i32::from(width) && y < i32::from(height) {
                    water[y as usize * usize::from(width) + x as usize] = true;
                }
            }
        }
    }

    // Match the two deterministic coastline cleanup passes used before H3
    // boundary authoring. Treating non-water source layers as open ground is
    // deliberately conservative for transport selection: a landing is never
    // authorized merely because decoration happened to overwrite water.
    for _ in 0..2 {
        let snapshot = water.clone();
        for y in 1..height.saturating_sub(1) {
            for x in 1..width.saturating_sub(1) {
                let index = usize::from(y) * usize::from(width) + usize::from(x);
                if snapshot[index] {
                    continue;
                }
                let neighbors = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                    .into_iter()
                    .filter(|&(neighbor_x, neighbor_y)| {
                        snapshot
                            [usize::from(neighbor_y) * usize::from(width) + usize::from(neighbor_x)]
                    })
                    .count();
                if neighbors >= 3 {
                    water[index] = true;
                }
            }
        }
    }
    fill_small_enclosed_raster_land(&mut water, width, height, 16);
    remove_small_raster_components(&mut water, width, height, 16);

    // Final generation reasserts source water at 31 shared-edge samples and
    // expands each hit into the exact cardinal three-cell seam band. Include
    // that restoration now so the regional planner cannot choose a dry pixel
    // pocket that disappears during final seam authoring.
    for portal in &plan.portals {
        if portal.boundary.len() != 2 {
            bail!("H3 edge {} does not have two endpoints", portal.edge_id);
        }
        let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
        for sample_index in 0..H3_GRID_SEAM_SAMPLES {
            let coordinate = spherical_interpolate(
                start,
                end,
                (sample_index as f64 + 0.5) / H3_GRID_SEAM_SAMPLES as f64,
            );
            if !features.iter().any(|feature| {
                geographic_polygon_contains(plan.center, coordinate, &feature.points)
            }) {
                continue;
            }
            for (x, y) in h3_raster_sample_band_for_dimensions(plan, width, height, coordinate)? {
                water[usize::from(y) * usize::from(width) + usize::from(x)] = true;
            }
        }
    }
    Ok(water)
}

fn rasterized_h3_linear_feature_cells(
    plan: &H3CellPlan,
    feature: &crate::Feature,
    width: u16,
    height: u16,
) -> Result<BTreeSet<(u16, u16)>> {
    let points = feature
        .points
        .iter()
        .map(|&coordinate| plan.project_to_grid(coordinate, width, height))
        .collect::<Result<Vec<_>>>()?;
    let mut cells = BTreeSet::new();
    for segment in points.windows(2) {
        for (x, y) in raster_line_cells(segment[0], segment[1]) {
            if x >= 0
                && y >= 0
                && x < i32::from(width)
                && y < i32::from(height)
                && plan.raster_contains_cell(x as u16, y as u16, width, height)?
            {
                cells.insert((x as u16, y as u16));
            }
        }
    }
    Ok(cells)
}

fn raster_line_cells(from: (i32, i32), to: (i32, i32)) -> Vec<(i32, i32)> {
    let (mut x, mut y) = from;
    let (end_x, end_y) = to;
    let delta_x = (end_x - x).abs();
    let step_x = if x < end_x { 1 } else { -1 };
    let delta_y = -(end_y - y).abs();
    let step_y = if y < end_y { 1 } else { -1 };
    let mut error = delta_x + delta_y;
    let mut cells = Vec::new();
    loop {
        cells.push((x, y));
        if (x, y) == (end_x, end_y) {
            break;
        }
        let doubled = error * 2;
        if doubled >= delta_y {
            error += delta_y;
            x += step_x;
        }
        if doubled <= delta_x {
            error += delta_x;
            y += step_y;
        }
    }
    cells
}

fn fill_small_enclosed_raster_land(
    water: &mut [bool],
    width: u16,
    height: u16,
    maximum_size: usize,
) {
    let mut unseen = water.iter().map(|cell| !cell).collect::<Vec<_>>();
    for start in 0..unseen.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        let mut enclosed = true;
        while let Some(index) = queue.pop_front() {
            component.push(index);
            let x = index % usize::from(width);
            let y = index / usize::from(width);
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= usize::from(width) || next_y >= usize::from(height) {
                    enclosed = false;
                    continue;
                }
                let next = next_y * usize::from(width) + next_x;
                if unseen[next] {
                    unseen[next] = false;
                    queue.push_back(next);
                }
            }
        }
        if enclosed && component.len() <= maximum_size {
            for index in component {
                water[index] = true;
            }
        }
    }
}

fn remove_small_raster_components(
    water: &mut [bool],
    width: u16,
    height: u16,
    minimum_size: usize,
) {
    let mut unseen = water.to_vec();
    for start in 0..unseen.len() {
        if !unseen[start] {
            continue;
        }
        unseen[start] = false;
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(index) = queue.pop_front() {
            component.push(index);
            let x = index % usize::from(width);
            let y = index / usize::from(width);
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= usize::from(width) || next_y >= usize::from(height) {
                    continue;
                }
                let next = next_y * usize::from(width) + next_x;
                if unseen[next] {
                    unseen[next] = false;
                    queue.push_back(next);
                }
            }
        }
        if component.len() < minimum_size {
            for index in component {
                water[index] = false;
            }
        }
    }
}

fn compress_h3_transport(
    plan: &H3CellPlan,
    source: &mut MapSource,
    grid_width: u16,
    grid_height: u16,
) -> Result<()> {
    let mut keep = BTreeSet::<usize>::new();
    let feasibility = H3RasterTransportFeasibility::new(plan, source, grid_width, grid_height)?;
    let selected_edges = plan.regional.as_ref().map(|regional| {
        regional
            .connections
            .iter()
            .map(|connection| (connection.edge_id.as_str(), connection))
            .collect::<BTreeMap<_, _>>()
    });
    let mut crossed_features = BTreeSet::<usize>::new();
    let mut internal = Vec::<(u8, u64, usize)>::new();

    for (index, feature) in source.features.iter().enumerate() {
        if !matches!(
            feature.kind,
            FeatureKind::Trail | FeatureKind::Street | FeatureKind::Road | FeatureKind::MajorRoad
        ) {
            keep.insert(index);
        }
    }

    for portal in &plan.portals {
        let crossings = source_transport_crossings(plan, portal, source, &feasibility)?;
        crossed_features.extend(crossings.iter().map(|crossing| crossing.feature_index));
        let selected = selected_edges
            .as_ref()
            .and_then(|edges| edges.get(portal.edge_id.as_str()).copied());
        let chosen = if let Some(connection) = selected {
            connection
                .authoritative
                .then(|| {
                    crossings.iter().find(|crossing| {
                        crossing.viable()
                            && crossing.candidate.transport == connection.transport
                            && crossing.candidate.bridge == connection.bridge
                            && crossing_coordinates_match(
                                crossing.candidate.coordinate,
                                connection.coordinate,
                            )
                    })
                })
                .flatten()
        } else {
            crossings.iter().find(|crossing| crossing.viable())
        };
        if let Some(chosen) = chosen {
            keep.insert(chosen.feature_index);
        }
    }

    for (index, feature) in source.features.iter().enumerate() {
        if !matches!(
            feature.kind,
            FeatureKind::Trail | FeatureKind::Street | FeatureKind::Road | FeatureKind::MajorRoad
        ) || crossed_features.contains(&index)
        {
            continue;
        }
        internal.push((
            transport_priority(feature.kind),
            stable_feature_key(feature),
            index,
        ));
    }
    internal.sort_unstable_by_key(|&(priority, key, _)| (std::cmp::Reverse(priority), key));
    let internal_limit = match plan.resolution {
        0..=6 => 4,
        7 => 3,
        _ => 2,
    };
    for (_, _, index) in internal.into_iter().take(internal_limit) {
        keep.insert(index);
    }

    source.features = std::mem::take(&mut source.features)
        .into_iter()
        .enumerate()
        .filter_map(|(index, feature)| keep.contains(&index).then_some(feature))
        .collect();
    Ok(())
}

fn validate_h3_regional_connections(
    plan: &H3CellPlan,
    source: &MapSource,
    regional: &H3RegionalCellPlan,
    grid_width: u16,
    grid_height: u16,
) -> Result<()> {
    let feasibility = H3RasterTransportFeasibility::new(plan, source, grid_width, grid_height)?;
    for connection in &regional.connections {
        let portal = plan
            .portals
            .iter()
            .find(|portal| portal.edge_id == connection.edge_id)
            .with_context(|| {
                format!(
                    "regional connection {} is not a face of H3 cell {}",
                    connection.edge_id, plan.cell
                )
            })?;
        if connection.authoritative {
            let crossings = source_transport_crossings(plan, portal, source, &feasibility)?;
            if !crossings.iter().any(|crossing| {
                crossing.viable()
                    && crossing.candidate.transport == connection.transport
                    && crossing.candidate.bridge == connection.bridge
                    && crossing_coordinates_match(
                        crossing.candidate.coordinate,
                        connection.coordinate,
                    )
            }) {
                bail!(
                    "authoritative regional edge {} has no exact raster-viable source crossing in H3 cell {}",
                    connection.edge_id,
                    plan.cell
                );
            }
        } else {
            if connection.bridge {
                bail!(
                    "synthetic regional edge {} cannot claim bridge authority in H3 cell {}",
                    connection.edge_id,
                    plan.cell
                );
            }
            if !feasibility.landing_reaches_stable_interior(plan, connection.coordinate, None)? {
                bail!(
                    "synthetic regional edge {} has no stable dry interior approach in H3 cell {}",
                    connection.edge_id,
                    plan.cell
                );
            }
        }
    }
    Ok(())
}

fn source_transport_crossings(
    plan: &H3CellPlan,
    portal: &H3Portal,
    source: &MapSource,
    feasibility: &H3RasterTransportFeasibility,
) -> Result<Vec<H3SourceTransportCrossing>> {
    if portal.boundary.len() != 2 {
        bail!("H3 edge {} does not have two endpoints", portal.edge_id);
    }
    let edge_center = spherical_midpoint(portal.boundary.iter().copied());
    let mut crossings = Vec::new();
    for (feature_index, feature) in source.features.iter().enumerate().filter(|(_, feature)| {
        matches!(
            feature.kind,
            FeatureKind::Trail | FeatureKind::Street | FeatureKind::Road | FeatureKind::MajorRoad
        ) && feature.points.len() >= 2
    }) {
        for feature_segment in feature.points.windows(2) {
            for edge_segment in portal.boundary.windows(2) {
                let Some(coordinate) = geographic_segment_intersection(
                    edge_center,
                    feature_segment[0],
                    feature_segment[1],
                    edge_segment[0],
                    edge_segment[1],
                ) else {
                    continue;
                };
                crossings.push(H3SourceTransportCrossing {
                    candidate: H3TransportCrossing {
                        transport: feature.kind,
                        coordinate,
                        bridge: feature.bridge,
                    },
                    feature_index,
                    stable_key: stable_feature_key(feature),
                    touches_authoritative_water: feasibility
                        .band_touches_water(plan, coordinate)?,
                    reaches_stable_interior: feasibility.landing_reaches_stable_interior(
                        plan,
                        coordinate,
                        Some(feature),
                    )?,
                });
            }
        }
    }
    crossings.sort_by_key(|crossing| {
        (
            std::cmp::Reverse(crossing.viable()),
            std::cmp::Reverse(transport_priority(crossing.candidate.transport)),
            std::cmp::Reverse(crossing.candidate.bridge),
            coordinate_key(crossing.candidate.coordinate),
            crossing.stable_key,
            crossing.feature_index,
        )
    });
    crossings.dedup_by(|left, right| {
        left.candidate.transport == right.candidate.transport
            && left.candidate.bridge == right.candidate.bridge
            && crossing_coordinates_match(left.candidate.coordinate, right.candidate.coordinate)
    });
    Ok(crossings)
}

fn crossing_coordinates_match(first: Coordinate, second: Coordinate) -> bool {
    let longitude_delta = (first.lon - second.lon + 180.0).rem_euclid(360.0) - 180.0;
    (first.lat - second.lat).abs() <= 1e-8 && longitude_delta.abs() <= 1e-8
}

fn stable_feature_key(feature: &crate::Feature) -> u64 {
    let mut value = 0xcbf2_9ce4_8422_2325_u64;
    let mut mix = |byte: u8| {
        value = (value ^ u64::from(byte)).wrapping_mul(0x1000_0000_01b3);
    };
    mix(feature.kind as u8);
    mix(u8::from(feature.bridge));
    if let Some(name) = &feature.name {
        for byte in name.bytes() {
            mix(byte);
        }
    }
    for point in &feature.points {
        for byte in point.lat.to_bits().to_le_bytes() {
            mix(byte);
        }
        for byte in point.lon.to_bits().to_le_bytes() {
            mix(byte);
        }
    }
    value
}

fn feature_intersects_h3_halo(plan: &H3CellPlan, feature: &crate::Feature) -> bool {
    let bounds = h3_halo_bounds(plan);
    let mut feature_west = f64::INFINITY;
    let mut feature_east = f64::NEG_INFINITY;
    let mut feature_south = f64::INFINITY;
    let mut feature_north = f64::NEG_INFINITY;
    for &point in &feature.points {
        let (east, north) = local_tangent(plan.center, point);
        feature_west = feature_west.min(east);
        feature_east = feature_east.max(east);
        feature_south = feature_south.min(north);
        feature_north = feature_north.max(north);
    }
    feature_west <= bounds.east
        && feature_east >= bounds.west
        && feature_south <= bounds.north
        && feature_north >= bounds.south
}

pub fn fetch_h3_neighborhood(plan: &H3CellPlan) -> Result<MapSource> {
    prepare_h3_source(fetch_h3_neighborhood_geometry(plan)?, plan.clone())
}

/// Fetch the shared geographic envelope for a batch of H3 cells, then apply
/// each cell's exact halo clipping and H3 ownership contract independently.
///
/// Neighboring H3 halos overlap substantially. Querying every halo on its own
/// makes Overpass return the same ways and relations many times. A batch uses
/// one enclosing box in ordinary longitude space, or one box on either side
/// of the antimeridian, while [`fetch_map_bounds`] remains responsible for
/// authoritative bounded subdivision of those boxes.
pub fn fetch_h3_batch_neighborhoods(plans: &[H3CellPlan]) -> Result<Vec<MapSource>> {
    fetch_h3_batch_neighborhoods_with(plans, fetch_map_bounds)
}

fn fetch_h3_batch_neighborhoods_with<F>(
    plans: &[H3CellPlan],
    mut fetch: F,
) -> Result<Vec<MapSource>>
where
    F: FnMut(Coordinate, BoundingBox) -> Result<MapSource>,
{
    if plans.is_empty() {
        return Ok(Vec::new());
    }
    let batch_bounds = h3_batch_fetch_bounds(plans)?;
    let mut features = Vec::new();
    let mut attribution = None::<String>;
    for bounds in batch_bounds {
        let center = batch_fetch_center(bounds);
        let source = fetch(center, bounds).with_context(|| {
            format!(
                "fetch shared H3 batch envelope ({},{},{},{})",
                bounds.south, bounds.west, bounds.north, bounds.east
            )
        })?;
        if source.h3.is_some() {
            bail!("shared H3 batch fetch returned an already-projected source");
        }
        if source.bounds != bounds {
            bail!(
                "shared H3 batch fetch returned bounds ({},{},{},{}) for requested envelope ({},{},{},{})",
                source.bounds.south,
                source.bounds.west,
                source.bounds.north,
                source.bounds.east,
                bounds.south,
                bounds.west,
                bounds.north,
                bounds.east
            );
        }
        if let Some(expected) = &attribution {
            if expected != &source.attribution {
                bail!("shared H3 batch fetch returned inconsistent source attribution");
            }
        } else {
            attribution = Some(source.attribution.clone());
        }
        features.extend(source.features);
    }
    sort_and_deduplicate_h3_features(&mut features);
    let attribution = attribution.context("H3 batch did not contain a fetch envelope")?;

    plans
        .iter()
        .map(|plan| {
            let bounds = *plan
                .fetch_bounds
                .first()
                .with_context(|| format!("H3 plan {} has no fetch bounds", plan.cell))?;
            prepare_h3_source(
                MapSource {
                    center: plan.center,
                    bounds,
                    attribution: attribution.clone(),
                    features: features.clone(),
                    h3: None,
                },
                plan.clone(),
            )
        })
        .collect()
}

fn h3_batch_fetch_bounds(plans: &[H3CellPlan]) -> Result<Vec<BoundingBox>> {
    let bounds = plans
        .iter()
        .flat_map(|plan| plan.fetch_bounds.iter().copied())
        .collect::<Vec<_>>();
    if bounds.is_empty() {
        bail!("H3 batch plans did not contain fetch bounds");
    }
    for bounds in &bounds {
        if !bounds.south.is_finite()
            || !bounds.west.is_finite()
            || !bounds.north.is_finite()
            || !bounds.east.is_finite()
            || bounds.south >= bounds.north
            || bounds.west >= bounds.east
            || !(-90.0..=90.0).contains(&bounds.south)
            || !(-90.0..=90.0).contains(&bounds.north)
            || !(-180.0..=180.0).contains(&bounds.west)
            || !(-180.0..=180.0).contains(&bounds.east)
        {
            bail!("H3 batch contains invalid fetch bounds");
        }
    }

    const LONGITUDE_EPSILON: f64 = 1e-12;
    let crosses_antimeridian = bounds
        .iter()
        .any(|bounds| (bounds.west + 180.0).abs() <= LONGITUDE_EPSILON)
        && bounds
            .iter()
            .any(|bounds| (bounds.east - 180.0).abs() <= LONGITUDE_EPSILON);
    let mut unions = if crosses_antimeridian {
        let western = bounds
            .iter()
            .copied()
            .filter(|bounds| (bounds.west + bounds.east) / 2.0 < 0.0)
            .collect::<Vec<_>>();
        let eastern = bounds
            .iter()
            .copied()
            .filter(|bounds| (bounds.west + bounds.east) / 2.0 >= 0.0)
            .collect::<Vec<_>>();
        if western.is_empty() || eastern.is_empty() {
            bail!("antimeridian H3 batch did not split into two longitude-side envelopes");
        }
        vec![union_h3_bounds(&western), union_h3_bounds(&eastern)]
    } else {
        vec![union_h3_bounds(&bounds)]
    };
    unions.sort_by(|left, right| left.west.total_cmp(&right.west));

    for required in bounds {
        if !unions.iter().any(|union| bounds_cover(*union, required)) {
            bail!("shared H3 batch envelopes do not cover every requested halo");
        }
    }
    Ok(unions)
}

fn union_h3_bounds(bounds: &[BoundingBox]) -> BoundingBox {
    BoundingBox {
        south: bounds
            .iter()
            .map(|bounds| bounds.south)
            .fold(f64::INFINITY, f64::min),
        west: bounds
            .iter()
            .map(|bounds| bounds.west)
            .fold(f64::INFINITY, f64::min),
        north: bounds
            .iter()
            .map(|bounds| bounds.north)
            .fold(f64::NEG_INFINITY, f64::max),
        east: bounds
            .iter()
            .map(|bounds| bounds.east)
            .fold(f64::NEG_INFINITY, f64::max),
    }
}

fn bounds_cover(container: BoundingBox, required: BoundingBox) -> bool {
    container.south <= required.south
        && container.west <= required.west
        && container.north >= required.north
        && container.east >= required.east
}

fn batch_fetch_center(bounds: BoundingBox) -> Coordinate {
    Coordinate {
        lat: (bounds.south + bounds.north) / 2.0,
        lon: (bounds.west + bounds.east) / 2.0,
    }
}

fn sort_and_deduplicate_h3_features(features: &mut Vec<crate::Feature>) {
    features.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.area.cmp(&right.area))
            .then_with(|| left.bridge.cmp(&right.bridge))
            .then_with(|| {
                for (left, right) in left.points.iter().zip(&right.points) {
                    let ordering = left
                        .lat
                        .total_cmp(&right.lat)
                        .then_with(|| left.lon.total_cmp(&right.lon));
                    if !ordering.is_eq() {
                        return ordering;
                    }
                }
                left.points.len().cmp(&right.points.len())
            })
    });
    features.dedup();
}

fn fetch_h3_neighborhood_geometry(plan: &H3CellPlan) -> Result<MapSource> {
    let mut merged = None::<MapSource>;
    for &bounds in &plan.fetch_bounds {
        let source = fetch_map_bounds(plan.center, bounds)?;
        if let Some(current) = &mut merged {
            current.features.extend(source.features);
        } else {
            merged = Some(source);
        }
    }
    let mut merged = merged.context("H3 plan did not contain fetch bounds")?;
    merged.features.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.bridge.cmp(&right.bridge))
            .then_with(|| left.points.len().cmp(&right.points.len()))
            .then_with(|| {
                left.points
                    .first()
                    .map(|point| (point.lat.to_bits(), point.lon.to_bits()))
                    .cmp(
                        &right
                            .points
                            .first()
                            .map(|point| (point.lat.to_bits(), point.lon.to_bits())),
                    )
            })
    });
    merged.features.dedup();
    Ok(merged)
}

/// Resolve all transport crossings against the exact output raster before a
/// regional graph is selected. An untagged line may not claim any landing
/// cell covered by authoritative water; only that exact line's `bridge=yes`
/// metadata can authorize the crossing.
pub fn build_h3_seam_contract(
    plan: &H3CellPlan,
    source: &MapSource,
    grid_width: u16,
    grid_height: u16,
) -> Result<H3SeamContract> {
    if source
        .h3
        .as_ref()
        .is_some_and(|source_plan| source_plan.cell != plan.cell)
    {
        bail!("source H3 cell does not match requested seam plan");
    }
    let regional = source
        .h3
        .as_ref()
        .and_then(|source_plan| source_plan.regional.as_ref())
        .or(plan.regional.as_ref());
    let feasibility = H3RasterTransportFeasibility::new(plan, source, grid_width, grid_height)?;
    let mut edges = Vec::with_capacity(plan.portals.len());
    for portal in &plan.portals {
        let source_crossings = source_transport_crossings(plan, portal, source, &feasibility)?;
        let viable_crossings = source_crossings
            .iter()
            .filter(|crossing| crossing.viable())
            .map(|crossing| crossing.candidate.clone())
            .collect::<Vec<_>>();
        let synthetic_traversable = source_crossings.is_empty()
            && feasibility.landing_reaches_stable_interior(plan, portal.midpoint, None)?;
        let selected = regional.and_then(|regional| {
            regional
                .connections
                .iter()
                .find(|connection| connection.edge_id == portal.edge_id)
        });
        let (transport, crossing) = if let Some(connection) = selected {
            (Some(connection.transport), Some(connection.coordinate))
        } else if regional.is_some() {
            (None, None)
        } else {
            (
                viable_crossings.first().map(|entry| entry.transport),
                viable_crossings.first().map(|entry| entry.coordinate),
            )
        };
        edges.push(H3EdgeContract {
            edge_id: portal.edge_id.clone(),
            neighbor: portal.neighbor.clone(),
            side: portal.side,
            terrain: edge_terrain(portal, source, plan.center),
            transport,
            crossing,
            viable_crossings,
            synthetic_traversable,
        });
    }
    Ok(H3SeamContract {
        cell: plan.cell.clone(),
        edges,
    })
}

pub fn audit_h3_seam_contracts(contracts: &[H3SeamContract]) -> H3SeamAudit {
    let cells = contracts
        .iter()
        .map(|contract| contract.cell.as_str())
        .collect::<BTreeSet<_>>();
    let mut by_edge = std::collections::BTreeMap::<&str, Vec<(&str, &H3EdgeContract)>>::new();
    for contract in contracts {
        for edge in &contract.edges {
            by_edge
                .entry(&edge.edge_id)
                .or_default()
                .push((&contract.cell, edge));
        }
    }
    let mut errors = Vec::new();
    let mut internal_edges = 0usize;
    let mut transport_edges = 0usize;
    let mut natural_edges = 0usize;
    for (edge_id, entries) in by_edge {
        if entries.iter().any(|(_, edge)| edge.transport.is_some()) {
            transport_edges += 1;
        } else {
            natural_edges += 1;
        }
        let endpoints_are_present = entries
            .first()
            .is_some_and(|(_, edge)| cells.contains(edge.neighbor.as_str()));
        if !endpoints_are_present {
            continue;
        }
        internal_edges += 1;
        if entries.len() != 2 {
            errors.push(format!(
                "internal H3 edge {edge_id} has {} contracts instead of two",
                entries.len()
            ));
            continue;
        }
        let (first_cell, first) = entries[0];
        let (second_cell, second) = entries[1];
        if first.neighbor != second_cell || second.neighbor != first_cell {
            errors.push(format!("H3 edge {edge_id} is not reciprocal"));
        }
        if first.terrain != second.terrain || first.transport != second.transport {
            errors.push(format!(
                "H3 edge {edge_id} disagrees: {:?}/{:?} versus {:?}/{:?}",
                first.terrain, first.transport, second.terrain, second.transport
            ));
        }
        match (first.crossing, second.crossing) {
            (Some(first), Some(second))
                if (first.lat - second.lat).abs() <= 1e-8
                    && longitude_delta(first.lon, second.lon).abs() <= 1e-8 => {}
            (None, None) => {}
            _ => errors.push(format!("H3 edge {edge_id} crossing coordinates disagree")),
        }
    }
    H3SeamAudit {
        passed: errors.is_empty(),
        cells: contracts.len(),
        internal_edges,
        transport_edges,
        natural_edges,
        errors,
    }
}

/// Capture the actual, post-generation terrain along every edge of one H3
/// raster. Samples are ordered by the canonical geographic edge endpoints, so
/// the reciprocal profile is directly comparable even when H3 exposes that
/// edge in the opposite direction.
pub fn build_h3_grid_seam_profile(grid: &GeneratedGrid) -> Result<H3GridSeamProfile> {
    let plan = grid
        .source
        .h3
        .as_ref()
        .context("H3 grid seam profile requires an H3 projection plan")?;
    if grid.cells.len() != usize::from(grid.width) * usize::from(grid.height) {
        bail!("H3 grid seam profile requires complete rectangular storage");
    }

    let water = grid
        .source
        .features
        .iter()
        .filter(|feature| {
            feature.area && feature.kind == FeatureKind::Water && feature.points.len() >= 3
        })
        .collect::<Vec<_>>();
    let mut edges = Vec::with_capacity(plan.portals.len());
    for portal in &plan.portals {
        if portal.boundary.len() != 2 {
            bail!(
                "H3 edge {} has {} endpoints instead of two",
                portal.edge_id,
                portal.boundary.len()
            );
        }
        let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
        let mut samples = Vec::with_capacity(H3_GRID_SEAM_SAMPLES);
        for index in 0..H3_GRID_SEAM_SAMPLES {
            let fraction = (index as f64 + 0.5) / H3_GRID_SEAM_SAMPLES as f64;
            let coordinate = spherical_interpolate(start, end, fraction);
            let (border, inner) = h3_raster_sample_cells(plan, grid, coordinate)?;
            let raster_coordinate =
                h3_raster_cell_coordinate(plan, grid.width, grid.height, border)?;
            samples.push(H3GridSeamSample {
                coordinate,
                source_water: water.iter().any(|feature| {
                    geographic_polygon_contains(plan.center, coordinate, &feature.points)
                }),
                raster_source_water: water.iter().any(|feature| {
                    geographic_polygon_contains(plan.center, raster_coordinate, &feature.points)
                }),
                surface: h3_seam_surface(grid.cell(border.0, border.1)),
                transport: h3_transport_kind(grid.cell(border.0, border.1)),
                inner_surface: h3_seam_surface(grid.cell(inner.0, inner.1)),
            });
        }
        let regional_transport = plan.regional.as_ref().and_then(|regional| {
            let selected = regional
                .connections
                .iter()
                .find(|connection| connection.edge_id == portal.edge_id)
                .map(|connection| {
                    (
                        H3GridTransportDirectiveKind::Selected,
                        connection.coordinate,
                        Some(connection.transport),
                    )
                });
            let closed = regional
                .closed_transport_crossings
                .iter()
                .find(|crossing| crossing.edge_id == portal.edge_id)
                .map(|crossing| {
                    (
                        H3GridTransportDirectiveKind::Closed,
                        crossing.coordinate,
                        None,
                    )
                });
            selected.or(closed)
        });
        let regional_transport = regional_transport
            .map(|(kind, coordinate, transport)| {
                let band = h3_raster_sample_band(plan, grid, coordinate)?;
                Ok::<H3GridTransportDirective, anyhow::Error>(H3GridTransportDirective {
                    kind,
                    coordinate,
                    transport,
                    band_surfaces: band
                        .iter()
                        .copied()
                        .map(|(x, y)| h3_seam_surface(grid.cell(x, y)))
                        .collect(),
                    band_transport: band
                        .into_iter()
                        .map(|(x, y)| h3_transport_kind(grid.cell(x, y)))
                        .collect(),
                })
            })
            .transpose()?;
        edges.push(H3GridEdgeProfile {
            edge_id: portal.edge_id.clone(),
            neighbor: portal.neighbor.clone(),
            samples,
            regional_transport,
        });
    }
    Ok(H3GridSeamProfile {
        cell: plan.cell.clone(),
        grid_width: grid.width,
        grid_height: grid.height,
        edges,
    })
}

/// Preserve exact OSM water through the quantized boundary band of an H3
/// raster. Both reciprocal faces sample the same geographic points, so a lake
/// cannot be clipped merely because their pixel centers fall on opposite sides
/// of a narrow shoreline. Only a selected transport trace carrying exact
/// `bridge=yes` authority may remain over water.
pub fn preserve_h3_authoritative_water_seams(grid: &mut GeneratedGrid) -> Result<()> {
    let plan = grid
        .source
        .h3
        .clone()
        .context("water seam preservation requires an H3 projection plan")?;
    let authoritative_water = h3_authoritative_water_band_indices(&plan, grid)?;
    for index in authoritative_water {
        let cell = &mut grid.cells[index];
        if !matches!(
            cell,
            MapCell::Rail | MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
        ) {
            *cell = MapCell::Water;
        }
    }
    Ok(())
}

fn h3_authoritative_water_band_indices(
    plan: &H3CellPlan,
    grid: &GeneratedGrid,
) -> Result<BTreeSet<usize>> {
    let water = grid
        .source
        .features
        .iter()
        .filter(|feature| {
            feature.area && feature.kind == FeatureKind::Water && feature.points.len() >= 3
        })
        .cloned()
        .collect::<Vec<_>>();
    let width = usize::from(grid.width);
    let mut authoritative_water = BTreeSet::new();
    for portal in &plan.portals {
        if portal.boundary.len() != 2 {
            bail!("H3 edge {} does not have two endpoints", portal.edge_id);
        }
        let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
        for index in 0..H3_GRID_SEAM_SAMPLES {
            let coordinate = spherical_interpolate(
                start,
                end,
                (index as f64 + 0.5) / H3_GRID_SEAM_SAMPLES as f64,
            );
            if !water.iter().any(|feature| {
                geographic_polygon_contains(plan.center, coordinate, &feature.points)
            }) {
                continue;
            }
            for (x, y) in h3_raster_sample_band(&plan, grid, coordinate)? {
                authoritative_water.insert(usize::from(y) * width + usize::from(x));
            }
        }
    }
    Ok(authoritative_water)
}

/// Make every selected regional landing reciprocal in the final raster and
/// cap real crossings omitted from the sparse graph. Generation follows this
/// with authoritative-water preservation, which restores closed shoreline
/// crossings while deliberately leaving only selected explicit-bridge cells
/// intact over authoritative water.
pub fn finalize_h3_regional_transport_seams(grid: &mut GeneratedGrid) -> Result<()> {
    let plan = grid
        .source
        .h3
        .clone()
        .context("regional transport finalization requires an H3 projection plan")?;
    let Some(regional) = plan.regional.as_ref() else {
        return Ok(());
    };
    let width = usize::from(grid.width);
    let authoritative_water = h3_authoritative_water_band_indices(&plan, grid)?;
    let mut selected_band = BTreeSet::<usize>::new();
    for connection in &regional.connections {
        for (x, y) in h3_raster_sample_band(&plan, grid, connection.coordinate)? {
            selected_band.insert(usize::from(y) * width + usize::from(x));
        }
    }
    for crossing in &regional.closed_transport_crossings {
        for (x, y) in h3_raster_sample_band(&plan, grid, crossing.coordinate)? {
            let index = usize::from(y) * width + usize::from(x);
            let cell = &mut grid.cells[index];
            if matches!(
                cell,
                MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
            ) {
                *cell = if authoritative_water.contains(&index) {
                    MapCell::Water
                } else {
                    MapCell::Grass
                };
            }
        }
    }
    // Linear-feature rasterization can touch the polygon boundary a cell or
    // two away from its canonical crossing. Exact-point capping alone left
    // shifted routes adjacent to H3Void, creating undeclared runtime exits.
    // The sparse regional graph is the sole authority for boundary routes.
    for y in 0..grid.height {
        for x in 0..grid.width {
            let index = usize::from(y) * width + usize::from(x);
            if selected_band.contains(&index)
                || !matches!(
                    grid.cells[index],
                    MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad
                )
                || !route_cell_touches_h3_void(grid, x, y)
            {
                continue;
            }
            grid.cells[index] = if authoritative_water.contains(&index) {
                MapCell::Water
            } else {
                MapCell::Grass
            };
        }
    }
    for connection in &regional.connections {
        let route = MapCell::from(connection.transport);
        for (x, y) in h3_raster_sample_band(&plan, grid, connection.coordinate)? {
            let index = usize::from(y) * width + usize::from(x);
            if authoritative_water.contains(&index) && !connection.bridge {
                bail!(
                    "regional edge {} cannot overlay authoritative water without an exact bridge=yes source trace",
                    connection.edge_id
                );
            }
            grid.cells[index] = route;
        }
    }
    Ok(())
}

pub(crate) fn route_cell_touches_h3_void(grid: &GeneratedGrid, x: u16, y: u16) -> bool {
    [
        (i32::from(x) - 1, i32::from(y)),
        (i32::from(x) + 1, i32::from(y)),
        (i32::from(x), i32::from(y) - 1),
        (i32::from(x), i32::from(y) + 1),
    ]
    .into_iter()
    .any(|(neighbor_x, neighbor_y)| {
        neighbor_x < 0
            || neighbor_y < 0
            || neighbor_x >= i32::from(grid.width)
            || neighbor_y >= i32::from(grid.height)
            || grid.cell(neighbor_x as u16, neighbor_y as u16) == Some(MapCell::H3Void)
    })
}

/// Audit semantic continuity in the final generated grids, not merely the
/// midpoint terrain promise in `H3SeamContract`.
///
/// All 31 canonical geographic samples must agree on their final semantic
/// surface and exact transport class. Water present in either source profile
/// is authoritative on both faces, while a long unsupported run still catches
/// quantization floods. Any one-cell tree, relief, or fence trace at the
/// storage join is rejected; genuine terrain that continues inward remains
/// valid.
pub fn audit_h3_grid_seams(profiles: &[H3GridSeamProfile]) -> H3GridSeamAudit {
    let cells = profiles
        .iter()
        .map(|profile| profile.cell.as_str())
        .collect::<BTreeSet<_>>();
    let mut by_edge = BTreeMap::<&str, Vec<(&str, &H3GridEdgeProfile)>>::new();
    for profile in profiles {
        for edge in &profile.edges {
            by_edge
                .entry(&edge.edge_id)
                .or_default()
                .push((&profile.cell, edge));
        }
    }

    let mut errors = Vec::new();
    if cells.len() != profiles.len() {
        errors.push("H3 grid seam profiles contain duplicate cells".to_string());
    }
    let mut internal_edges = 0usize;
    let mut reciprocal_surface_samples = 0usize;
    let mut matching_surface_samples = 0usize;
    let mut mismatched_surface_samples = 0usize;
    let mut matching_transport_samples = 0usize;
    let mut mismatched_transport_samples = 0usize;
    let mut authoritative_water_samples = 0usize;
    let mut continuous_water_samples = 0usize;
    let mut tree_outline_edges = 0usize;
    let mut relief_outline_edges = 0usize;
    let mut fence_outline_edges = 0usize;
    let mut artificial_trace_samples = 0usize;
    let mut selected_transport_edges = 0usize;
    let mut connected_transport_edges = 0usize;
    let mut closed_transport_edges = 0usize;
    let mut capped_transport_edges = 0usize;
    for (edge_id, entries) in by_edge {
        let endpoints_are_present = entries
            .first()
            .is_some_and(|(_, edge)| cells.contains(edge.neighbor.as_str()));
        if !endpoints_are_present {
            continue;
        }
        internal_edges += 1;
        if entries.len() != 2 {
            errors.push(format!(
                "internal H3 raster edge {edge_id} has {} profiles instead of two",
                entries.len()
            ));
            continue;
        }
        let (first_cell, first) = entries[0];
        let (second_cell, second) = entries[1];
        if first.neighbor != second_cell || second.neighbor != first_cell {
            errors.push(format!("H3 raster edge {edge_id} is not reciprocal"));
        }
        if first.samples.len() != H3_GRID_SEAM_SAMPLES
            || second.samples.len() != H3_GRID_SEAM_SAMPLES
        {
            errors.push(format!(
                "H3 raster edge {edge_id} has {}/{} samples; expected {H3_GRID_SEAM_SAMPLES}",
                first.samples.len(),
                second.samples.len()
            ));
            continue;
        }

        match (&first.regional_transport, &second.regional_transport) {
            (Some(left), Some(right))
                if left.kind == right.kind
                    && left.transport == right.transport
                    && (left.coordinate.lat - right.coordinate.lat).abs() <= 1e-10
                    && longitude_delta(left.coordinate.lon, right.coordinate.lon).abs()
                        <= 1e-10 =>
            {
                let bands_are_complete = left.band_surfaces.len() == 3
                    && right.band_surfaces.len() == 3
                    && left.band_transport.len() == 3
                    && right.band_transport.len() == 3;
                match left.kind {
                    H3GridTransportDirectiveKind::Selected => {
                        selected_transport_edges += 1;
                        if let Some(expected) = left.transport
                            && bands_are_complete
                            && left
                                .band_surfaces
                                .iter()
                                .chain(&right.band_surfaces)
                                .all(|surface| *surface == H3GridSeamSurface::Transport)
                            && left
                                .band_transport
                                .iter()
                                .chain(&right.band_transport)
                                .all(|transport| *transport == Some(expected))
                        {
                            connected_transport_edges += 1;
                        } else {
                            errors.push(format!(
                                "H3 raster edge {edge_id} does not render its selected {:?} landing on both faces",
                                left.transport
                            ));
                        }
                    }
                    H3GridTransportDirectiveKind::Closed => {
                        closed_transport_edges += 1;
                        if left.transport.is_none()
                            && bands_are_complete
                            && left
                                .band_surfaces
                                .iter()
                                .chain(&right.band_surfaces)
                                .all(|surface| *surface != H3GridSeamSurface::Transport)
                            && left
                                .band_transport
                                .iter()
                                .chain(&right.band_transport)
                                .all(Option::is_none)
                        {
                            capped_transport_edges += 1;
                        } else {
                            errors.push(format!(
                                "H3 raster edge {edge_id} leaves an unselected transport crossing open"
                            ));
                        }
                    }
                }
            }
            (None, None) => {}
            _ => errors.push(format!(
                "H3 raster edge {edge_id} disagrees on its regional transport directive"
            )),
        }

        let mut missing_water = Vec::new();
        let mut coordinate_disagreements = Vec::new();
        let mut surface_disagreements = Vec::new();
        let mut transport_disagreements = Vec::new();
        let mut unexpected_water = vec![false; H3_GRID_SEAM_SAMPLES];
        for (index, (left, right)) in first.samples.iter().zip(&second.samples).enumerate() {
            if (left.coordinate.lat - right.coordinate.lat).abs() > 1e-10
                || longitude_delta(left.coordinate.lon, right.coordinate.lon).abs() > 1e-10
            {
                coordinate_disagreements.push(index);
                continue;
            }
            reciprocal_surface_samples += 1;
            if left.surface == right.surface {
                matching_surface_samples += 1;
            } else {
                mismatched_surface_samples += 1;
                surface_disagreements.push(index);
            }
            if left.transport == right.transport {
                matching_transport_samples += 1;
            } else {
                mismatched_transport_samples += 1;
                transport_disagreements.push(index);
            }
            if left.source_water || right.source_water {
                authoritative_water_samples += 1;
                if water_compatible(left.surface) && water_compatible(right.surface) {
                    continuous_water_samples += 1;
                } else {
                    missing_water.push(index);
                }
            } else {
                // Polygon vertices and raster block centers quantize at
                // slightly different locations. Ignore a three-sample halo
                // around real shoreline crossings, but still reject the long
                // unsupported run produced by flooding a whole midpoint edge.
                let near_authoritative_water = first.samples
                    [index.saturating_sub(3)..=(index + 3).min(H3_GRID_SEAM_SAMPLES - 1)]
                    .iter()
                    .any(|sample| sample.source_water)
                    || second.samples
                        [index.saturating_sub(3)..=(index + 3).min(H3_GRID_SEAM_SAMPLES - 1)]
                        .iter()
                        .any(|sample| sample.source_water);
                if !near_authoritative_water {
                    unexpected_water[index] = (matches!(left.surface, H3GridSeamSurface::Water)
                        && !left.raster_source_water)
                        || (matches!(right.surface, H3GridSeamSurface::Water)
                            && !right.raster_source_water);
                }
            }
        }
        if !coordinate_disagreements.is_empty() {
            errors.push(format!(
                "H3 raster edge {edge_id} uses different geographic samples at indexes {}",
                sample_indexes(&coordinate_disagreements)
            ));
        }
        if !surface_disagreements.is_empty() {
            errors.push(format!(
                "H3 raster edge {edge_id} disagrees on final surface categories at indexes {}",
                sample_indexes(&surface_disagreements)
            ));
        }
        if !transport_disagreements.is_empty() {
            errors.push(format!(
                "H3 raster edge {edge_id} disagrees on final transport classes at indexes {}",
                sample_indexes(&transport_disagreements)
            ));
        }
        if !missing_water.is_empty() {
            errors.push(format!(
                "H3 raster edge {edge_id} clips authoritative water at indexes {}",
                sample_indexes(&missing_water)
            ));
        }
        let unexpected_count = unexpected_water.iter().filter(|sample| **sample).count();
        let longest_unexpected_run = longest_true_run(&unexpected_water);
        // Four adjacent samples can alias the same one- or two-cell shoreline
        // pocket at a diagonal H3 rim. The batch reconciler permits only
        // Grass components of at most two cells pinned cardinally between
        // Water and H3Void, so a run of four is still a bounded quantization
        // correction; five samples is a visible invented shoreline.
        // A cached regional extract can end just outside a requested face,
        // leaving a bounded minority of samples without source polygons even
        // though reciprocal generated cells agree. Reject majority-edge
        // flooding, while allowing that explicitly supplied-source case to
        // retain a continuous full-metatile shoreline.
        if unexpected_count >= 16 || longest_unexpected_run >= 9 {
            errors.push(format!(
                "H3 raster edge {edge_id} paints water outside source geometry at {unexpected_count}/{H3_GRID_SEAM_SAMPLES} samples (longest run {longest_unexpected_run})"
            ));
        }

        for (cell, edge) in [(first_cell, first), (second_cell, second)] {
            let tree_samples = edge
                .samples
                .iter()
                .filter(|sample| {
                    sample.surface == H3GridSeamSurface::Tree
                        && sample.inner_surface != H3GridSeamSurface::Tree
                })
                .count();
            let relief_samples = edge
                .samples
                .iter()
                .filter(|sample| {
                    sample.surface == H3GridSeamSurface::Relief
                        && sample.inner_surface != H3GridSeamSurface::Relief
                })
                .count();
            let fence_samples = edge
                .samples
                .iter()
                .filter(|sample| {
                    sample.surface == H3GridSeamSurface::Fence
                        && sample.inner_surface != H3GridSeamSurface::Fence
                })
                .count();
            artificial_trace_samples += tree_samples + relief_samples + fence_samples;
            if tree_samples > 0 {
                tree_outline_edges += 1;
                errors.push(format!(
                    "H3 raster edge {edge_id} draws one-cell tree traces in {cell} at {tree_samples}/{H3_GRID_SEAM_SAMPLES} samples"
                ));
            }
            if relief_samples > 0 {
                relief_outline_edges += 1;
                errors.push(format!(
                    "H3 raster edge {edge_id} draws one-cell relief traces in {cell} at {relief_samples}/{H3_GRID_SEAM_SAMPLES} samples"
                ));
            }
            if fence_samples > 0 {
                fence_outline_edges += 1;
                errors.push(format!(
                    "H3 raster edge {edge_id} draws one-cell fence traces in {cell} at {fence_samples}/{H3_GRID_SEAM_SAMPLES} samples"
                ));
            }
        }
    }

    H3GridSeamAudit {
        passed: errors.is_empty(),
        cells: profiles.len(),
        internal_edges,
        samples_per_edge: H3_GRID_SEAM_SAMPLES,
        reciprocal_surface_samples,
        matching_surface_samples,
        mismatched_surface_samples,
        matching_transport_samples,
        mismatched_transport_samples,
        authoritative_water_samples,
        continuous_water_samples,
        tree_outline_edges,
        relief_outline_edges,
        fence_outline_edges,
        artificial_trace_samples,
        selected_transport_edges,
        connected_transport_edges,
        closed_transport_edges,
        capped_transport_edges,
        errors,
    }
}

fn canonical_edge_endpoints(first: Coordinate, second: Coordinate) -> (Coordinate, Coordinate) {
    if first
        .lat
        .total_cmp(&second.lat)
        .then_with(|| first.lon.total_cmp(&second.lon))
        == std::cmp::Ordering::Greater
    {
        (second, first)
    } else {
        (first, second)
    }
}

fn spherical_interpolate(first: Coordinate, second: Coordinate, fraction: f64) -> Coordinate {
    let vector = |coordinate: Coordinate| {
        let latitude = coordinate.lat.to_radians();
        let longitude = coordinate.lon.to_radians();
        (
            latitude.cos() * longitude.cos(),
            latitude.cos() * longitude.sin(),
            latitude.sin(),
        )
    };
    let first_vector = vector(first);
    let second_vector = vector(second);
    let dot = (first_vector.0 * second_vector.0
        + first_vector.1 * second_vector.1
        + first_vector.2 * second_vector.2)
        .clamp(-1.0, 1.0);
    let angle = dot.acos();
    let (first_weight, second_weight) = if angle.sin().abs() < 1e-12 {
        (1.0 - fraction, fraction)
    } else {
        (
            ((1.0 - fraction) * angle).sin() / angle.sin(),
            (fraction * angle).sin() / angle.sin(),
        )
    };
    let x = first_vector.0 * first_weight + second_vector.0 * second_weight;
    let y = first_vector.1 * first_weight + second_vector.1 * second_weight;
    let z = first_vector.2 * first_weight + second_vector.2 * second_weight;
    Coordinate {
        lat: z.atan2((x * x + y * y).sqrt()).to_degrees(),
        lon: y.atan2(x).to_degrees(),
    }
}

fn h3_raster_sample_cells(
    plan: &H3CellPlan,
    grid: &GeneratedGrid,
    coordinate: Coordinate,
) -> Result<((u16, u16), (u16, u16))> {
    let inside_cells = h3_raster_sample_band(plan, grid, coordinate)?;
    Ok((inside_cells[0], inside_cells[2]))
}

pub(crate) fn h3_raster_sample_band(
    plan: &H3CellPlan,
    grid: &GeneratedGrid,
    coordinate: Coordinate,
) -> Result<Vec<(u16, u16)>> {
    h3_raster_sample_band_for_dimensions(plan, grid.width, grid.height, coordinate)
}

pub(crate) fn h3_raster_landing(
    plan: &H3CellPlan,
    width: u16,
    height: u16,
    coordinate: Coordinate,
) -> Result<(u16, u16)> {
    h3_raster_sample_band_for_dimensions(plan, width, height, coordinate)?
        .into_iter()
        .next()
        .context("H3 transport landing has no raster cells")
}

fn h3_raster_sample_band_for_dimensions(
    plan: &H3CellPlan,
    width: u16,
    height: u16,
    coordinate: Coordinate,
) -> Result<Vec<(u16, u16)>> {
    let frame = plan.raster_frame(width, height)?;
    let polygon = plan.raster_polygon(width, height)?;
    let inside_face = |x: i32, y: i32| {
        x >= 0
            && y >= 0
            && x < i32::from(width)
            && y < i32::from(height)
            && point_in_polygon(f64::from(x) + 0.5, f64::from(y) + 0.5, &polygon)
    };
    let touches_outside = |x: i32, y: i32| {
        [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)]
            .into_iter()
            .any(|(delta_x, delta_y)| !inside_face(x + delta_x, y + delta_y))
    };
    let (east, north) = local_tangent(plan.center, coordinate);
    let edge_x = (east - frame.west) / (frame.east - frame.west) * f64::from(width - 1);
    let edge_y = (frame.north - north) / (frame.north - frame.south) * f64::from(height - 1);
    let center_x = f64::from(width - 1) / 2.0;
    let center_y = f64::from(height - 1) / 2.0;
    let delta_x = center_x - edge_x;
    let delta_y = center_y - edge_y;
    let length = delta_x.hypot(delta_y);
    if length <= f64::EPSILON {
        bail!("H3 edge sample collapsed onto raster center");
    }
    let direction_x = delta_x / length;
    let direction_y = delta_y / length;
    let mut inside_cells = Vec::<(u16, u16)>::new();
    let mut previous_sample = None::<(i32, i32)>;
    // Begin a bounded eight raster cells outside the geographic face and
    // follow the normal inward. Starting at distance zero can round past a
    // valid rim pixel on sloped edges, leaving a nominal runtime gate only
    // diagonally adjacent to H3Void. The signed scan makes the first retained
    // cell the actual cardinal mouth of the face.
    for step in -64_i32..=64 {
        let distance = f64::from(step) / 8.0;
        let x = (edge_x + direction_x * distance).round() as i32;
        let y = (edge_y + direction_y * distance).round() as i32;
        let sample = (x, y);
        if previous_sample == Some(sample) {
            continue;
        }
        let mut candidates = Vec::with_capacity(2);
        if let Some((previous_x, previous_y)) = previous_sample {
            if previous_x.abs_diff(x) == 1 && previous_y.abs_diff(y) == 1 {
                let first = (x, previous_y);
                let second = (previous_x, y);
                if inside_face(previous_x, previous_y) || inside_face(x, y) {
                    if inside_face(first.0, first.1) {
                        candidates.push(first);
                    } else if inside_face(second.0, second.1) {
                        candidates.push(second);
                    }
                }
            }
        }
        candidates.push(sample);
        previous_sample = Some(sample);
        for (candidate_x, candidate_y) in candidates {
            if !inside_face(candidate_x, candidate_y) {
                continue;
            }
            let cell = (candidate_x as u16, candidate_y as u16);
            if inside_cells.last() == Some(&cell) {
                continue;
            }
            if let Some(&(previous_x, previous_y)) = inside_cells.last()
                && previous_x.abs_diff(cell.0) + previous_y.abs_diff(cell.1) != 1
            {
                // At an acute projected vertex the center-point face mask can
                // contain a diagonal one-cell tip with neither cardinal
                // bridge cell inside the face. It is not a playable mouth;
                // restart at the next rim cell that belongs to the cardinal
                // body of the face instead of joining through H3Void.
                inside_cells.clear();
            }
            if inside_cells.is_empty() && !touches_outside(candidate_x, candidate_y) {
                bail!(
                    "H3 edge sample for cell {} starts inland at ({candidate_x}, {candidate_y})",
                    plan.cell
                );
            }
            inside_cells.push(cell);
            if inside_cells.len() == 3 {
                return Ok(inside_cells);
            }
        }
    }
    bail!(
        "could not resolve boundary and inward raster samples for H3 cell {}",
        plan.cell
    )
}

fn h3_raster_cell_coordinate(
    plan: &H3CellPlan,
    width: u16,
    height: u16,
    cell: (u16, u16),
) -> Result<Coordinate> {
    let frame = plan.raster_frame(width, height)?;
    let east =
        frame.west + (f64::from(cell.0) + 0.5) / f64::from(width - 1) * (frame.east - frame.west);
    let north = frame.north
        - (f64::from(cell.1) + 0.5) / f64::from(height - 1) * (frame.north - frame.south);
    let center_latitude = plan.center.lat.to_radians();
    let center_longitude = plan.center.lon.to_radians();
    let center = (
        center_latitude.cos() * center_longitude.cos(),
        center_latitude.cos() * center_longitude.sin(),
        center_latitude.sin(),
    );
    let east_basis = (-center_longitude.sin(), center_longitude.cos(), 0.0);
    let north_basis = (
        -center_latitude.sin() * center_longitude.cos(),
        -center_latitude.sin() * center_longitude.sin(),
        center_latitude.cos(),
    );
    let radial = (1.0 - east * east - north * north).max(0.0).sqrt();
    let x = center.0 * radial + east_basis.0 * east + north_basis.0 * north;
    let y = center.1 * radial + east_basis.1 * east + north_basis.1 * north;
    let z = center.2 * radial + east_basis.2 * east + north_basis.2 * north;
    Ok(Coordinate {
        lat: z.atan2((x * x + y * y).sqrt()).to_degrees(),
        lon: y.atan2(x).to_degrees(),
    })
}

fn h3_seam_surface(cell: Option<MapCell>) -> H3GridSeamSurface {
    match cell {
        Some(MapCell::H3Void) | None => H3GridSeamSurface::Void,
        Some(
            MapCell::Grass
            | MapCell::Lawn
            | MapCell::Clearing
            | MapCell::Flowers
            | MapCell::Pitch
            | MapCell::IceFloor
            | MapCell::RockFloor,
        ) => H3GridSeamSurface::Ground,
        Some(MapCell::Park) => H3GridSeamSurface::Wild,
        Some(
            MapCell::Water
            | MapCell::WaterAccessEast
            | MapCell::WaterAccessWest
            | MapCell::WaterAccessSouth,
        ) => H3GridSeamSurface::Water,
        Some(
            MapCell::Rail | MapCell::Trail | MapCell::Street | MapCell::Road | MapCell::MajorRoad,
        ) => H3GridSeamSurface::Transport,
        Some(MapCell::Tree | MapCell::ParkTree | MapCell::SmallTree | MapCell::SmallTreeSouth) => {
            H3GridSeamSurface::Tree
        }
        Some(
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
            | MapCell::CliffStairs,
        ) => H3GridSeamSurface::Relief,
        Some(
            MapCell::FenceNorthWest
            | MapCell::FenceNorth
            | MapCell::FenceNorthEast
            | MapCell::FenceWest
            | MapCell::FenceEast
            | MapCell::FenceSouthWest
            | MapCell::FenceSouth
            | MapCell::FenceSouthEast,
        ) => H3GridSeamSurface::Fence,
        Some(MapCell::Bench | MapCell::TrashCan | MapCell::Fountain | MapCell::GroundSign) => {
            H3GridSeamSurface::Fixture
        }
        Some(
            MapCell::Building
            | MapCell::PokecenterNorthWest
            | MapCell::PokecenterNorthEast
            | MapCell::PokecenterSouthWest
            | MapCell::PokecenterSouthEast
            | MapCell::MartNorthWest
            | MapCell::MartNorthEast
            | MapCell::MartSouthWest
            | MapCell::MartSouthEast,
        ) => H3GridSeamSurface::Structure,
    }
}

fn h3_transport_kind(cell: Option<MapCell>) -> Option<FeatureKind> {
    match cell {
        Some(MapCell::Rail) => Some(FeatureKind::Rail),
        Some(MapCell::Trail) => Some(FeatureKind::Trail),
        Some(MapCell::Street) => Some(FeatureKind::Street),
        Some(MapCell::Road) => Some(FeatureKind::Road),
        Some(MapCell::MajorRoad) => Some(FeatureKind::MajorRoad),
        _ => None,
    }
}

fn water_compatible(surface: H3GridSeamSurface) -> bool {
    matches!(
        surface,
        H3GridSeamSurface::Water | H3GridSeamSurface::Transport
    )
}

fn longest_true_run(samples: &[bool]) -> usize {
    samples
        .iter()
        .fold((0usize, 0usize), |(longest, current), sample| {
            let current = if *sample { current + 1 } else { 0 };
            (longest.max(current), current)
        })
        .0
}

fn sample_indexes(indexes: &[usize]) -> String {
    indexes
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn longitude_delta(first: f64, second: f64) -> f64 {
    (first - second + 180.0).rem_euclid(360.0) - 180.0
}

fn edge_terrain(portal: &H3Portal, source: &MapSource, center: Coordinate) -> H3EdgeTerrain {
    for feature in source.features.iter().filter(|feature| {
        feature.area && feature.kind == FeatureKind::Water && feature.points.len() >= 3
    }) {
        if geographic_polygon_contains(center, portal.midpoint, &feature.points) {
            return H3EdgeTerrain::Water;
        }
    }
    for feature in source.features.iter().filter(|feature| {
        feature.area && feature.kind == FeatureKind::Park && feature.points.len() >= 3
    }) {
        if geographic_polygon_contains(center, portal.midpoint, &feature.points) {
            return H3EdgeTerrain::TallGrass;
        }
    }
    match edge_seed(&portal.edge_id) % 100 {
        0..=34 => H3EdgeTerrain::Trees,
        35..=59 => H3EdgeTerrain::Grass,
        60..=79 => H3EdgeTerrain::RockTerrace,
        80..=91 => H3EdgeTerrain::FenceGate,
        _ => H3EdgeTerrain::TallGrass,
    }
}

fn transport_priority(kind: FeatureKind) -> u8 {
    match kind {
        FeatureKind::Rail => 5,
        FeatureKind::MajorRoad => 4,
        FeatureKind::Road => 3,
        FeatureKind::Street => 2,
        FeatureKind::Trail => 1,
        _ => 0,
    }
}

fn edge_seed(edge_id: &str) -> u64 {
    edge_id.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x1000_0000_01b3)
    })
}

fn coordinate_key(coordinate: Coordinate) -> u64 {
    coordinate.lat.to_bits().rotate_left(17) ^ coordinate.lon.to_bits()
}

fn geographic_segment_intersection(
    center: Coordinate,
    first_start: Coordinate,
    first_end: Coordinate,
    second_start: Coordinate,
    second_end: Coordinate,
) -> Option<Coordinate> {
    let p = local_tangent(center, first_start);
    let p2 = local_tangent(center, first_end);
    let q = local_tangent(center, second_start);
    let q2 = local_tangent(center, second_end);
    let r = (p2.0 - p.0, p2.1 - p.1);
    let s = (q2.0 - q.0, q2.1 - q.1);
    let cross = |a: (f64, f64), b: (f64, f64)| a.0 * b.1 - a.1 * b.0;
    let denominator = cross(r, s);
    if denominator.abs() < 1e-15 {
        return None;
    }
    let q_minus_p = (q.0 - p.0, q.1 - p.1);
    let t = cross(q_minus_p, s) / denominator;
    let u = cross(q_minus_p, r) / denominator;
    if !(-1e-9..=1.0 + 1e-9).contains(&t) || !(-1e-9..=1.0 + 1e-9).contains(&u) {
        return None;
    }
    Some(interpolate_coordinate(
        first_start,
        first_end,
        t.clamp(0.0, 1.0),
    ))
}

fn interpolate_coordinate(start: Coordinate, end: Coordinate, amount: f64) -> Coordinate {
    let mut longitude_delta = end.lon - start.lon;
    if longitude_delta > 180.0 {
        longitude_delta -= 360.0;
    } else if longitude_delta < -180.0 {
        longitude_delta += 360.0;
    }
    Coordinate {
        lat: start.lat + (end.lat - start.lat) * amount,
        lon: (start.lon + longitude_delta * amount + 180.0).rem_euclid(360.0) - 180.0,
    }
}

fn geographic_polygon_contains(
    center: Coordinate,
    point: Coordinate,
    polygon: &[Coordinate],
) -> bool {
    let point = local_tangent(center, point);
    let polygon = polygon
        .iter()
        .map(|&coordinate| local_tangent(center, coordinate))
        .collect::<Vec<_>>();
    point_in_polygon(point.0, point.1, &polygon)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3BatchCell {
    pub ordinal: usize,
    pub ring: u32,
    pub plan: H3CellPlan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H3BatchManifest {
    pub schema_version: u32,
    pub origin: String,
    pub resolution: u8,
    pub requested_cells: usize,
    pub cells: Vec<H3BatchCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct H3BatchLink {
    pub edge_id: String,
    pub first_ordinal: usize,
    pub first_cell: String,
    pub first_side: HexSide,
    pub first_gate: (u16, u16),
    pub second_ordinal: usize,
    pub second_cell: String,
    pub second_side: HexSide,
    pub second_gate: (u16, u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct H3BatchConnections {
    pub schema_version: u32,
    pub grid_width: u16,
    pub grid_height: u16,
    pub links: Vec<H3BatchLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct H3BatchTopologyAudit {
    pub passed: bool,
    pub cells: usize,
    pub internal_edges: usize,
    pub boundary_edges: usize,
    pub pentagons: usize,
    pub prefix_connected: bool,
    pub errors: Vec<String>,
}

pub fn audit_h3_batch_topology(
    manifest: &H3BatchManifest,
    grid_width: u16,
    grid_height: u16,
) -> Result<(H3BatchTopologyAudit, Vec<H3BatchLink>)> {
    let mut errors = Vec::new();
    if manifest.schema_version != H3_MANIFEST_SCHEMA_VERSION {
        errors.push(format!(
            "manifest schema {} does not match {}",
            manifest.schema_version, H3_MANIFEST_SCHEMA_VERSION
        ));
    }
    if manifest.cells.len() != manifest.requested_cells {
        errors.push(format!(
            "manifest contains {} of {} requested cells",
            manifest.cells.len(),
            manifest.requested_cells
        ));
    }
    let ordinals = manifest
        .cells
        .iter()
        .enumerate()
        .all(|(ordinal, entry)| entry.ordinal == ordinal);
    if !ordinals {
        errors.push("manifest ordinals are not contiguous and ordered".to_string());
    }
    let by_cell = manifest
        .cells
        .iter()
        .map(|entry| (entry.plan.cell.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    if by_cell.len() != manifest.cells.len() {
        errors.push("manifest contains duplicate H3 cells".to_string());
    }
    if manifest.cells.first().map(|entry| entry.plan.cell.as_str())
        != Some(manifest.origin.as_str())
    {
        errors.push("manifest origin is not ordinal zero".to_string());
    }

    let mut prefix_connected = true;
    for entry in manifest.cells.iter().skip(1) {
        let has_prior_neighbor = entry.plan.portals.iter().any(|portal| {
            by_cell
                .get(portal.neighbor.as_str())
                .is_some_and(|neighbor| neighbor.ordinal < entry.ordinal)
        });
        if !has_prior_neighbor {
            prefix_connected = false;
            errors.push(format!(
                "cell {} at ordinal {} has no neighbor in the prior prefix",
                entry.plan.cell, entry.ordinal
            ));
        }
    }

    let mut edges = BTreeMap::<String, Vec<(usize, &H3Portal)>>::new();
    for entry in &manifest.cells {
        if entry.plan.resolution != manifest.resolution {
            errors.push(format!(
                "cell {} has resolution {}, expected {}",
                entry.plan.cell, entry.plan.resolution, manifest.resolution
            ));
        }
        for portal in &entry.plan.portals {
            edges
                .entry(portal.edge_id.clone())
                .or_default()
                .push((entry.ordinal, portal));
        }
    }

    let mut links = Vec::new();
    let mut boundary_edges = 0;
    for (edge_id, entries) in edges {
        match entries.as_slice() {
            [(_, portal)] if !by_cell.contains_key(portal.neighbor.as_str()) => {
                boundary_edges += 1;
            }
            [(first_ordinal, first), (second_ordinal, second)] => {
                let first_entry = &manifest.cells[*first_ordinal];
                let second_entry = &manifest.cells[*second_ordinal];
                if first.neighbor != second_entry.plan.cell
                    || second.neighbor != first_entry.plan.cell
                {
                    errors.push(format!("H3 edge {edge_id} has non-reciprocal neighbors"));
                    continue;
                }
                if second.side != first.side.opposite() {
                    errors.push(format!(
                        "H3 edge {edge_id} uses {:?} and {:?}, not opposite presentation sides",
                        first.side, second.side
                    ));
                    continue;
                }
                let first_gate = first.side.gate(grid_width, grid_height, 2)?;
                let second_gate = second.side.gate(grid_width, grid_height, 2)?;
                links.push(H3BatchLink {
                    edge_id,
                    first_ordinal: *first_ordinal,
                    first_cell: first_entry.plan.cell.clone(),
                    first_side: first.side,
                    first_gate,
                    second_ordinal: *second_ordinal,
                    second_cell: second_entry.plan.cell.clone(),
                    second_side: second.side,
                    second_gate,
                });
            }
            [(_, portal)] => errors.push(format!(
                "H3 edge {edge_id} points to in-batch neighbor {} without its reciprocal portal",
                portal.neighbor
            )),
            _ => errors.push(format!(
                "H3 edge {edge_id} occurs {} times; expected one boundary or two internal entries",
                entries.len()
            )),
        }
    }
    links.sort_by(|left, right| left.edge_id.cmp(&right.edge_id));
    let audit = H3BatchTopologyAudit {
        passed: errors.is_empty(),
        cells: manifest.cells.len(),
        internal_edges: links.len(),
        boundary_edges,
        pentagons: manifest
            .cells
            .iter()
            .filter(|entry| entry.plan.is_pentagon)
            .count(),
        prefix_connected,
        errors,
    };
    Ok((audit, links))
}

pub fn plan_h3_cell(coordinate: Coordinate, resolution: u8) -> Result<H3CellPlan> {
    let resolution = Resolution::try_from(resolution)
        .with_context(|| format!("H3 resolution must be between 0 and 15, got {resolution}"))?;
    let coordinate = LatLng::new(coordinate.lat, coordinate.lon)
        .context("convert coordinate to H3 latitude/longitude")?;
    cell_plan(coordinate.to_cell(resolution))
}

/// Breadth-first, deterministic first-stage coverage around a coordinate.
///
/// The manifest contains topology only: no Overpass requests, tile rendering,
/// or modpack generation occurs. Every cell after the origin is adjacent to a
/// prior cell, making prefixes independently useful while the first 5,000 are
/// built and audited incrementally.
pub fn plan_h3_batch(
    coordinate: Coordinate,
    resolution: u8,
    requested_cells: usize,
) -> Result<H3BatchManifest> {
    if !(1..=MAX_INITIAL_H3_CELLS).contains(&requested_cells) {
        bail!(
            "initial H3 batch must contain 1-{MAX_INITIAL_H3_CELLS} cells, got {requested_cells}"
        );
    }
    let resolution = Resolution::try_from(resolution)
        .with_context(|| format!("H3 resolution must be between 0 and 15, got {resolution}"))?;
    let origin = LatLng::new(coordinate.lat, coordinate.lon)
        .context("convert batch origin to H3 latitude/longitude")?
        .to_cell(resolution);
    let mut seen = BTreeSet::from([origin]);
    let mut queue = VecDeque::from([(origin, 0u32)]);
    let mut ordered = Vec::with_capacity(requested_cells);

    while let Some((cell, ring)) = queue.pop_front() {
        ordered.push((cell, ring));
        if ordered.len() == requested_cells {
            break;
        }
        let mut neighbors = cell
            .grid_disk::<Vec<_>>(1)
            .into_iter()
            .filter(|neighbor| *neighbor != cell)
            .collect::<Vec<_>>();
        neighbors.sort_unstable_by_key(|neighbor| u64::from(*neighbor));
        for neighbor in neighbors {
            if seen.insert(neighbor) {
                queue.push_back((neighbor, ring + 1));
            }
        }
    }
    if ordered.len() != requested_cells {
        bail!(
            "H3 traversal exhausted after {} of {requested_cells} requested cells",
            ordered.len()
        );
    }

    let cells = ordered
        .into_iter()
        .enumerate()
        .map(|(ordinal, (cell, ring))| {
            Ok(H3BatchCell {
                ordinal,
                ring,
                plan: cell_plan(cell)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(H3BatchManifest {
        schema_version: H3_MANIFEST_SCHEMA_VERSION,
        origin: origin.to_string(),
        resolution: u8::from(resolution),
        requested_cells,
        cells,
    })
}

fn cell_plan(cell: CellIndex) -> Result<H3CellPlan> {
    let center = coordinate(LatLng::from(cell));
    let boundary = cell
        .boundary()
        .iter()
        .copied()
        .map(coordinate)
        .collect::<Vec<_>>();
    let mut candidates = cell
        .edges()
        .map(|edge| {
            let neighbor = edge.destination();
            let neighbor_center = coordinate(LatLng::from(neighbor));
            let edge_boundary = edge.boundary();
            let boundary = edge_boundary
                .iter()
                .copied()
                .map(coordinate)
                .collect::<Vec<_>>();
            let midpoint = spherical_midpoint(boundary.iter().copied());
            PortalCandidate {
                edge_id: edge_id(cell, neighbor),
                neighbor: neighbor.to_string(),
                bearing: initial_bearing(center, neighbor_center),
                midpoint,
                boundary,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.neighbor.cmp(&right.neighbor));
    let sides = assign_sides(&candidates);
    let mut portals = candidates
        .into_iter()
        .zip(sides)
        .map(|(candidate, side)| H3Portal {
            edge_id: candidate.edge_id,
            neighbor: candidate.neighbor,
            side,
            midpoint: candidate.midpoint,
            boundary: candidate.boundary,
        })
        .collect::<Vec<_>>();
    portals.sort_by_key(|portal| portal.side);
    Ok(H3CellPlan {
        cell: cell.to_string(),
        resolution: u8::from(cell.resolution()),
        center,
        fetch_bounds: fetch_bounds(center, &boundary),
        boundary,
        is_pentagon: cell.is_pentagon(),
        portals,
        source_provenance: H3SourceProvenance::planned(),
        regional: None,
    })
}

#[derive(Debug)]
struct PortalCandidate {
    edge_id: String,
    neighbor: String,
    bearing: f64,
    midpoint: Coordinate,
    boundary: Vec<Coordinate>,
}

#[derive(Debug, Clone, Copy)]
struct RasterFrame {
    west: f64,
    east: f64,
    south: f64,
    north: f64,
}

fn assign_sides(candidates: &[PortalCandidate]) -> Vec<HexSide> {
    fn search(
        candidates: &[PortalCandidate],
        at: usize,
        used: &mut [bool; 6],
        current: &mut Vec<HexSide>,
        cost: f64,
        best: &mut Option<(f64, Vec<HexSide>)>,
    ) {
        if at == candidates.len() {
            if best.as_ref().is_none_or(|(best_cost, _)| cost < *best_cost) {
                *best = Some((cost, current.clone()));
            }
            return;
        }
        for (index, side) in HexSide::ALL.into_iter().enumerate() {
            if used[index] {
                continue;
            }
            let next_cost = cost + angular_distance(candidates[at].bearing, side.bearing());
            if best
                .as_ref()
                .is_some_and(|(best_cost, _)| next_cost >= *best_cost)
            {
                continue;
            }
            used[index] = true;
            current.push(side);
            search(candidates, at + 1, used, current, next_cost, best);
            current.pop();
            used[index] = false;
        }
    }

    let mut best = None;
    search(
        candidates,
        0,
        &mut [false; 6],
        &mut Vec::with_capacity(candidates.len()),
        0.0,
        &mut best,
    );
    best.map(|(_, sides)| sides).unwrap_or_default()
}

fn edge_id(first: CellIndex, second: CellIndex) -> String {
    let first = u64::from(first);
    let second = u64::from(second);
    format!("{:x}-{:x}", first.min(second), first.max(second))
}

fn coordinate(value: LatLng) -> Coordinate {
    Coordinate {
        lat: value.lat(),
        lon: value.lng(),
    }
}

fn initial_bearing(from: Coordinate, to: Coordinate) -> f64 {
    let from_lat = from.lat.to_radians();
    let to_lat = to.lat.to_radians();
    let delta_lon = (to.lon - from.lon).to_radians();
    let y = delta_lon.sin() * to_lat.cos();
    let x = from_lat.cos() * to_lat.sin() - from_lat.sin() * to_lat.cos() * delta_lon.cos();
    y.atan2(x).to_degrees().rem_euclid(360.0)
}

fn angular_distance(first: f64, second: f64) -> f64 {
    let difference = (first - second).abs().rem_euclid(360.0);
    difference.min(360.0 - difference)
}

fn spherical_midpoint(points: impl Iterator<Item = Coordinate>) -> Coordinate {
    let points = points.collect::<Vec<_>>();
    if points.is_empty() {
        return Coordinate { lat: 0.0, lon: 0.0 };
    }
    let mut x = 0.0;
    let mut y = 0.0;
    let mut z = 0.0;
    for point in &points {
        let lat = point.lat.to_radians();
        let lon = point.lon.to_radians();
        x += lat.cos() * lon.cos();
        y += lat.cos() * lon.sin();
        z += lat.sin();
    }
    let count = points.len() as f64;
    x /= count;
    y /= count;
    z /= count;
    Coordinate {
        lat: z.atan2((x * x + y * y).sqrt()).to_degrees(),
        lon: y.atan2(x).to_degrees(),
    }
}

fn circular_longitude_mean(longitudes: impl Iterator<Item = f64>) -> f64 {
    let mut sin = 0.0;
    let mut cos = 0.0;
    let mut count = 0usize;
    for longitude in longitudes {
        sin += longitude.to_radians().sin();
        cos += longitude.to_radians().cos();
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        sin.atan2(cos).to_degrees()
    }
}

/// Unit-sphere local tangent coordinates. Scale cancels during rasterization;
/// the basis remains well-defined at both poles and across ±180° longitude.
fn local_tangent(center: Coordinate, coordinate: Coordinate) -> (f64, f64) {
    let center_lat = center.lat.to_radians();
    let center_lon = center.lon.to_radians();
    let lat = coordinate.lat.to_radians();
    let lon = coordinate.lon.to_radians();
    let point = (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());
    let east_basis = (-center_lon.sin(), center_lon.cos(), 0.0);
    let north_basis = (
        -center_lat.sin() * center_lon.cos(),
        -center_lat.sin() * center_lon.sin(),
        center_lat.cos(),
    );
    (
        point.0 * east_basis.0 + point.1 * east_basis.1 + point.2 * east_basis.2,
        point.0 * north_basis.0 + point.1 * north_basis.1 + point.2 * north_basis.2,
    )
}

fn point_in_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &current in polygon {
        if (current.1 > y) != (previous.1 > y)
            && x < (previous.0 - current.0) * (y - current.1) / (previous.1 - current.1) + current.0
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn fetch_bounds(center: Coordinate, boundary: &[Coordinate]) -> Vec<BoundingBox> {
    let unwrapped = boundary
        .iter()
        .map(|point| {
            let mut lon = point.lon;
            while lon - center.lon > 180.0 {
                lon -= 360.0;
            }
            while lon - center.lon < -180.0 {
                lon += 360.0;
            }
            (point.lat, lon)
        })
        .collect::<Vec<_>>();
    let south = unwrapped.iter().map(|point| point.0).fold(90.0, f64::min);
    let north = unwrapped.iter().map(|point| point.0).fold(-90.0, f64::max);
    let west = unwrapped
        .iter()
        .map(|point| point.1)
        .fold(f64::INFINITY, f64::min);
    let east = unwrapped
        .iter()
        .map(|point| point.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let latitude_pad = ((north - south) * 0.18).max(1e-7);
    let longitude_pad = ((east - west) * 0.18).max(1e-7);
    let south = (south - latitude_pad).max(-90.0);
    let north = (north + latitude_pad).min(90.0);
    let west = west - longitude_pad;
    let east = east + longitude_pad;
    if west < -180.0 {
        vec![
            BoundingBox {
                south,
                west: west + 360.0,
                north,
                east: 180.0,
            },
            BoundingBox {
                south,
                west: -180.0,
                north,
                east,
            },
        ]
    } else if east > 180.0 {
        vec![
            BoundingBox {
                south,
                west,
                north,
                east: 180.0,
            },
            BoundingBox {
                south,
                west: -180.0,
                north,
                east: east - 360.0,
            },
        ]
    } else {
        vec![BoundingBox {
            south,
            west,
            north,
            east,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Feature, generate_grid};

    const MINNEAPOLIS: Coordinate = Coordinate {
        lat: 44.947_519_6,
        lon: -93.325_347_7,
    };

    fn source_for(plan: &H3CellPlan, features: Vec<Feature>) -> MapSource {
        let mut source_plan = plan.clone();
        source_plan.source_provenance = if source_plan.regional.is_some() {
            H3SourceProvenance::reduced(H3SourceStage::RegionalReduced, 64, 64)
        } else {
            H3SourceProvenance::prepared_raw()
        };
        MapSource {
            center: plan.center,
            bounds: plan.fetch_bounds[0],
            attribution: "H3 seam fixture".to_string(),
            features,
            h3: Some(source_plan),
        }
    }

    #[test]
    fn coordinate_and_resolution_produce_a_complete_reciprocal_hex_plan() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        assert_eq!(plan.resolution, 8);
        assert_eq!(plan.boundary.len(), 6);
        assert_eq!(plan.portals.len(), 6);
        assert_eq!(
            plan.portals
                .iter()
                .map(|portal| portal.side)
                .collect::<BTreeSet<_>>()
                .len(),
            6
        );
        assert!(plan.owns(MINNEAPOLIS).expect("ownership"));

        for portal in &plan.portals {
            let neighbor =
                cell_plan(portal.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
            let reciprocal = neighbor
                .portals
                .iter()
                .find(|candidate| candidate.edge_id == portal.edge_id)
                .expect("reciprocal shared edge");
            assert_eq!(reciprocal.neighbor, plan.cell);
        }
    }

    #[test]
    fn lowering_h3_resolution_produces_a_larger_geographic_face() {
        let larger = plan_h3_cell(MINNEAPOLIS, 7).expect("resolution 7 plan");
        let smaller = plan_h3_cell(MINNEAPOLIS, 8).expect("resolution 8 plan");
        let mean_radius = |plan: &H3CellPlan| {
            plan.boundary
                .iter()
                .map(|&point| {
                    let (east, north) = local_tangent(plan.center, point);
                    east.hypot(north)
                })
                .sum::<f64>()
                / plan.boundary.len() as f64
        };
        assert!(
            mean_radius(&larger) > mean_radius(&smaller) * 2.0,
            "resolution 7 must be materially larger than resolution 8"
        );
    }

    #[test]
    fn pentagons_have_five_unique_portals_without_inventing_a_sixth_neighbor() {
        let pentagon = Resolution::Eight.pentagons().next().expect("pentagon");
        let plan = cell_plan(pentagon).expect("pentagon plan");
        assert!(plan.is_pentagon);
        assert_eq!(plan.boundary.len(), 5);
        assert_eq!(plan.portals.len(), 5);
        assert_eq!(
            plan.portals
                .iter()
                .map(|portal| portal.side)
                .collect::<BTreeSet<_>>()
                .len(),
            5
        );
    }

    #[test]
    fn first_five_thousand_are_deterministic_unique_and_prefix_connected() {
        let first = plan_h3_batch(MINNEAPOLIS, 8, 5_000).expect("first manifest");
        let second = plan_h3_batch(MINNEAPOLIS, 8, 5_000).expect("second manifest");
        assert_eq!(first, second);
        assert_eq!(first.cells.len(), 5_000);
        let mut prior = BTreeSet::<String>::new();
        for entry in &first.cells {
            assert_eq!(entry.ordinal, prior.len());
            if entry.ordinal > 0 {
                assert!(
                    entry
                        .plan
                        .portals
                        .iter()
                        .any(|portal| prior.contains(&portal.neighbor)),
                    "cell {} is disconnected from its generated prefix",
                    entry.plan.cell
                );
            }
            assert!(prior.insert(entry.plan.cell.clone()));
        }
        let (audit, links) = audit_h3_batch_topology(&first, 64, 64).expect("5k topology audit");
        assert!(audit.passed, "{}", audit.errors.join("; "));
        assert!(audit.prefix_connected);
        assert_eq!(audit.cells, 5_000);
        assert_eq!(audit.internal_edges, links.len());
        assert!(audit.internal_edges > first.cells.len());
        assert!(audit.boundary_edges > 0);
        assert!(links.iter().all(|link| {
            link.first_side.opposite() == link.second_side && link.first_gate != link.second_gate
        }));
    }

    #[test]
    fn antimeridian_cells_split_fetch_bounds_instead_of_spanning_the_planet() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 0.0,
                lon: 179.999,
            },
            5,
        )
        .expect("antimeridian plan");
        assert_eq!(plan.fetch_bounds.len(), 2);
        assert!(
            plan.fetch_bounds
                .iter()
                .all(|bounds| bounds.east - bounds.west < 5.0)
        );
    }

    #[test]
    fn regional_batch_fetch_envelope_covers_every_halo_with_one_shared_fetch() {
        let manifest = plan_h3_batch(MINNEAPOLIS, 6, 19).expect("19-cell manifest");
        let plans = manifest
            .cells
            .iter()
            .map(|entry| entry.plan.clone())
            .collect::<Vec<_>>();
        let shared = h3_batch_fetch_bounds(&plans).expect("shared fetch envelope");
        let per_cell_fetches = plans
            .iter()
            .map(|plan| plan.fetch_bounds.len())
            .sum::<usize>();

        assert_eq!(shared.len(), 1, "Minneapolis does not cross ±180°");
        assert!(
            shared.len() * 4 < per_cell_fetches,
            "one shared envelope must materially reduce {} per-cell top-level fetches",
            per_cell_fetches
        );
        for plan in &plans {
            for required in &plan.fetch_bounds {
                assert!(
                    shared
                        .iter()
                        .any(|container| bounds_cover(*container, *required)),
                    "shared envelope omitted the halo for {}",
                    plan.cell
                );
            }
        }
    }

    #[test]
    fn batch_fetches_overlapping_geometry_once_then_prepares_each_cell_exactly() {
        let manifest = plan_h3_batch(MINNEAPOLIS, 6, 2).expect("neighboring manifest");
        let plans = manifest
            .cells
            .iter()
            .map(|entry| entry.plan.clone())
            .collect::<Vec<_>>();
        let road = Feature {
            kind: FeatureKind::Road,
            name: Some("one shared road".to_string()),
            area: false,
            bridge: false,
            points: vec![plans[0].center, plans[1].center],
        };
        let building = Feature {
            kind: FeatureKind::Building,
            name: Some("one owned building".to_string()),
            area: true,
            bridge: false,
            points: vec![plans[0].center],
        };
        let mut requests = Vec::new();
        let sources = fetch_h3_batch_neighborhoods_with(&plans, |center, bounds| {
            requests.push(bounds);
            Ok(MapSource {
                center,
                bounds,
                attribution: "batch fetch fixture".to_string(),
                features: vec![road.clone(), building.clone()],
                h3: None,
            })
        })
        .expect("shared batch fetch");

        assert_eq!(requests.len(), 1, "overlapping halos fetch one envelope");
        assert_eq!(sources.len(), 2);
        for (source, plan) in sources.iter().zip(&plans) {
            assert_eq!(source.bounds, plan.fetch_bounds[0]);
            let source_plan = source.h3.as_ref().expect("prepared H3 plan");
            assert_eq!(source_plan.cell, plan.cell);
            assert!(source_plan.source_provenance.is_prepared_raw());
            assert!(source_plan.regional.is_none());
            assert!(
                source
                    .features
                    .iter()
                    .any(|feature| feature.name.as_deref() == Some("one shared road")),
                "shared linear geometry must survive both neighboring halo clips"
            );
        }
        assert_eq!(
            sources
                .iter()
                .map(|source| {
                    source
                        .features
                        .iter()
                        .filter(|feature| feature.kind == FeatureKind::Building)
                        .count()
                })
                .collect::<Vec<_>>(),
            vec![1, 0],
            "the fetched-once building must still belong to exactly one H3 cell"
        );
    }

    #[test]
    fn antimeridian_batch_fetches_two_side_unions_and_deduplicates_features() {
        let manifest = plan_h3_batch(
            Coordinate {
                lat: 0.0,
                lon: 179.999,
            },
            5,
            3,
        )
        .expect("antimeridian manifest");
        let plans = manifest
            .cells
            .iter()
            .map(|entry| entry.plan.clone())
            .collect::<Vec<_>>();
        let shared = h3_batch_fetch_bounds(&plans).expect("two longitude-side unions");
        assert_eq!(shared.len(), 2);
        assert_eq!(shared[0].west, -180.0);
        assert_eq!(shared[1].east, 180.0);
        assert!(shared.iter().all(|bounds| bounds.east - bounds.west < 10.0));
        for plan in &plans {
            for required in &plan.fetch_bounds {
                assert!(
                    shared
                        .iter()
                        .any(|container| bounds_cover(*container, *required)),
                    "two side unions omitted {} halo {required:?}",
                    plan.cell
                );
            }
        }

        let building = Feature {
            kind: FeatureKind::Building,
            name: Some("dateline building".to_string()),
            area: true,
            bridge: false,
            points: vec![plans[0].center],
        };
        let mut requests = Vec::new();
        let sources = fetch_h3_batch_neighborhoods_with(&plans, |center, bounds| {
            requests.push(bounds);
            Ok(MapSource {
                center,
                bounds,
                attribution: "antimeridian batch fixture".to_string(),
                features: vec![building.clone()],
                h3: None,
            })
        })
        .expect("two-union batch fetch");
        assert_eq!(requests, shared);
        assert_eq!(
            sources
                .iter()
                .flat_map(|source| &source.features)
                .filter(|feature| feature.kind == FeatureKind::Building)
                .count(),
            1,
            "a feature returned by both longitude-side queries is deduplicated before ownership"
        );
    }

    #[test]
    fn six_portal_gates_are_distinct_and_clear_of_raster_corners() {
        let gates = HexSide::ALL
            .into_iter()
            .map(|side| side.gate(64, 64, 2).expect("gate"))
            .collect::<BTreeSet<_>>();
        assert_eq!(gates.len(), 6);
        assert!(
            gates
                .iter()
                .all(|&(x, y)| x >= 2 && y >= 2 && x <= 61 && y <= 61)
        );
    }

    #[test]
    fn raster_seam_bands_start_on_the_face_rim_and_are_cardinally_connected() {
        let plan = plan_h3_cell(MINNEAPOLIS, 6).expect("H3 plan");
        for dimensions in [24_u16, 64, 128] {
            let grid = GeneratedGrid {
                source: source_for(&plan, Vec::new()),
                width: dimensions,
                height: dimensions,
                cells: vec![MapCell::Grass; usize::from(dimensions) * usize::from(dimensions)],
                labels: Vec::new(),
            };
            for portal in &plan.portals {
                let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
                for sample_index in 0..H3_GRID_SEAM_SAMPLES {
                    let coordinate = spherical_interpolate(
                        start,
                        end,
                        (sample_index as f64 + 0.5) / H3_GRID_SEAM_SAMPLES as f64,
                    );
                    let band = h3_raster_sample_band(&plan, &grid, coordinate)
                        .expect("three-cell seam band");
                    assert_eq!(band.len(), 3);
                    assert!(band.windows(2).all(|pair| {
                        pair[0].0.abs_diff(pair[1].0) + pair[0].1.abs_diff(pair[1].1) == 1
                    }));
                    let (x, y) = band[0];
                    assert!(
                        [
                            (i32::from(x) - 1, i32::from(y)),
                            (i32::from(x) + 1, i32::from(y)),
                            (i32::from(x), i32::from(y) - 1),
                            (i32::from(x), i32::from(y) + 1),
                        ]
                        .into_iter()
                        .any(|(neighbor_x, neighbor_y)| {
                            neighbor_x < 0
                                || neighbor_y < 0
                                || neighbor_x >= i32::from(grid.width)
                                || neighbor_y >= i32::from(grid.height)
                                || !plan
                                    .raster_contains_cell(
                                        neighbor_x as u16,
                                        neighbor_y as u16,
                                        grid.width,
                                        grid.height,
                                    )
                                    .expect("neighbor face membership")
                        }),
                        "{}px edge {} sample {sample_index} starts inland at {:?}",
                        dimensions,
                        portal.edge_id,
                        band[0]
                    );
                }
            }
        }
    }

    #[test]
    fn minneapolis_france_avenue_landing_includes_the_cardinal_face_rim() {
        let plan = cell_plan(
            "86262cd2fffffff"
                .parse::<CellIndex>()
                .expect("exact neighboring cell"),
        )
        .expect("exact neighboring plan");
        let crossing = Coordinate {
            lat: 44.904_418_681_182_925,
            lon: -93.328_995_012_443_1,
        };
        assert_eq!(
            h3_raster_sample_band_for_dimensions(&plan, 64, 64, crossing)
                .expect("exact France Avenue landing"),
            vec![(4, 12), (4, 13), (5, 13)],
            "the rounded crossing must include the dry in-face pixel cardinally adjacent to H3Void"
        );
    }

    #[test]
    fn polar_raster_seam_bands_remain_cardinal_at_the_storage_edge() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 89.9,
                lon: 179.9,
            },
            5,
        )
        .expect("polar H3 plan");
        assert_eq!(plan.cell, "8503262bfffffff");
        for portal in &plan.portals {
            let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
            for sample_index in 0..H3_GRID_SEAM_SAMPLES {
                let coordinate = spherical_interpolate(
                    start,
                    end,
                    (sample_index as f64 + 0.5) / H3_GRID_SEAM_SAMPLES as f64,
                );
                let band = h3_raster_sample_band_for_dimensions(&plan, 64, 64, coordinate)
                    .unwrap_or_else(|error| {
                        panic!(
                            "polar edge {} sample {sample_index} at {coordinate:?}: {error:#}",
                            portal.edge_id
                        )
                    });
                assert!(band.windows(2).all(|pair| {
                    pair[0].0.abs_diff(pair[1].0) + pair[0].1.abs_diff(pair[1].1) == 1
                }));
            }
        }
    }

    #[test]
    fn atomic_building_anchors_are_owned_by_exactly_one_neighboring_cell() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("origin plan");
        let neighbor_plan = cell_plan(
            plan.portals[0]
                .neighbor
                .parse::<CellIndex>()
                .expect("neighbor"),
        )
        .expect("neighbor plan");
        let source = MapSource {
            center: MINNEAPOLIS,
            bounds: BoundingBox::square_miles_around(MINNEAPOLIS, 1.0).expect("bounds"),
            attribution: "ownership fixture".to_string(),
            features: vec![
                Feature {
                    kind: FeatureKind::Building,
                    name: Some("origin house".to_string()),
                    area: true,
                    bridge: false,
                    points: vec![plan.center],
                },
                Feature {
                    kind: FeatureKind::Building,
                    name: Some("neighbor house".to_string()),
                    area: true,
                    bridge: false,
                    points: vec![neighbor_plan.center],
                },
            ],
            h3: None,
        };
        let owned_here = prepare_h3_source(source.clone(), plan).expect("origin ownership");
        let owned_there = prepare_h3_source(source, neighbor_plan).expect("neighbor ownership");
        assert_eq!(owned_here.features.len(), 1);
        assert_eq!(owned_there.features.len(), 1);
        assert_ne!(owned_here.features[0].name, owned_there.features[0].name);
    }

    #[test]
    fn safe_structure_mask_accepts_the_center_and_rejects_hex_corners() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        assert!(
            plan.raster_footprint_fits(30, 30, 2, 2, 2, 64, 64)
                .expect("center footprint")
        );
        assert!(
            !plan
                .raster_footprint_fits(1, 1, 2, 2, 1, 64, 64)
                .expect("corner footprint")
        );
    }

    #[test]
    fn tangent_projection_is_continuous_across_the_antimeridian() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 0.0,
                lon: 179.999,
            },
            5,
        )
        .expect("antimeridian plan");
        for point in &plan.boundary {
            let (x, y) = plan.project_to_grid(*point, 64, 64).expect("project");
            assert!((-1..=64).contains(&x), "x={x}");
            assert!((-1..=64).contains(&y), "y={y}");
        }
    }

    #[test]
    fn natural_h3_edges_never_invent_a_perimeter_road() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let contract = build_h3_seam_contract(&plan, &source_for(&plan, Vec::new()), 64, 64)
            .expect("natural seam contract");
        assert_eq!(contract.edges.len(), plan.portals.len());
        assert!(
            contract
                .edges
                .iter()
                .all(|edge| { edge.transport.is_none() && edge.crossing.is_none() })
        );
    }

    #[test]
    fn a_real_linear_feature_opens_only_the_edge_it_crosses() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let target = &plan.portals[0];
        let neighbor =
            cell_plan(target.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let road = Feature {
            kind: FeatureKind::Road,
            name: Some("shared crossing".to_string()),
            area: false,
            bridge: false,
            points: vec![plan.center, neighbor.center],
        };
        let contract = build_h3_seam_contract(&plan, &source_for(&plan, vec![road]), 64, 64)
            .expect("road seam contract");
        let crossed = contract
            .edges
            .iter()
            .filter(|edge| edge.transport.is_some())
            .collect::<Vec<_>>();
        assert_eq!(crossed.len(), 1, "one real road must not ring the cell");
        assert_eq!(crossed[0].edge_id, target.edge_id);
        assert_eq!(crossed[0].transport, Some(FeatureKind::Road));
        assert!(crossed[0].crossing.is_some());
    }

    #[test]
    fn an_untagged_road_crossing_authoritative_water_is_not_a_transport_gate() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let target = &plan.portals[0];
        let neighbor =
            cell_plan(target.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let (start, end) = canonical_edge_endpoints(target.boundary[0], target.boundary[1]);
        let water_start = spherical_interpolate(start, end, 0.35);
        let water_end = spherical_interpolate(start, end, 0.65);
        let water = Feature {
            kind: FeatureKind::Water,
            name: Some("authoritative water crossing".to_string()),
            area: true,
            bridge: false,
            points: vec![
                spherical_interpolate(water_start, plan.center, 0.12),
                spherical_interpolate(water_end, plan.center, 0.12),
                spherical_interpolate(water_end, neighbor.center, 0.12),
                spherical_interpolate(water_start, neighbor.center, 0.12),
                spherical_interpolate(water_start, plan.center, 0.12),
            ],
        };
        let road = Feature {
            kind: FeatureKind::Road,
            name: Some("untagged causeway lookalike".to_string()),
            area: false,
            bridge: false,
            points: vec![plan.center, neighbor.center],
        };

        let contract = build_h3_seam_contract(
            &plan,
            &source_for(&plan, vec![water.clone(), road.clone()]),
            64,
            64,
        )
        .expect("water-aware seam contract");
        let edge = contract
            .edges
            .iter()
            .find(|edge| edge.edge_id == target.edge_id)
            .expect("target edge");
        assert_eq!(edge.transport, None);
        assert_eq!(edge.crossing, None);
        assert!(edge.viable_crossings.is_empty());
        assert!(
            !edge.synthetic_traversable,
            "a rejected real crossing must not return as a midpoint Trail"
        );

        let bridge = Feature {
            bridge: true,
            ..road
        };
        let bridged =
            build_h3_seam_contract(&plan, &source_for(&plan, vec![water, bridge]), 64, 64)
                .expect("explicit bridge contract");
        let bridged_edge = bridged
            .edges
            .iter()
            .find(|edge| edge.edge_id == target.edge_id)
            .expect("bridged target edge");
        assert_eq!(bridged_edge.transport, Some(FeatureKind::Road));
        assert_eq!(bridged_edge.viable_crossings.len(), 1);
        assert!(bridged_edge.viable_crossings[0].bridge);
    }

    #[test]
    fn minneapolis_boundary_exit_dry_island_cannot_reach_stable_interior() {
        let plan = plan_h3_cell(MINNEAPOLIS, 6).expect("Minneapolis H3 plan");
        assert_eq!(plan.cell, "86262cd27ffffff");
        let edge_id = "86262cd27ffffff-86262cd37ffffff";
        let portal = plan
            .portals
            .iter()
            .find(|portal| portal.edge_id == edge_id)
            .expect("exact failed boundary edge");
        let crossing = Coordinate {
            lat: 44.926_156_490_165_12,
            lon: -93.376_316_974_263_1,
        };
        let band = h3_raster_sample_band_for_dimensions(&plan, 64, 64, crossing)
            .expect("exact failed landing band");
        assert_eq!(band, vec![(0, 41), (1, 41), (1, 40)]);

        // Reproduce the final-authoritative shoreline immediately around the
        // failed Excelsior Boulevard exit. The selected band and its mapped
        // road cell (2, 41) are dry, but the inward road cells at y=40 are
        // separated from them by water. The only apparent alternative hugs
        // storage-face void and is removed by the final full-face cap.
        let mut water = vec![false; 64 * 64];
        for (y, water_x) in [
            (37_u16, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            (38, vec![1, 2, 3, 4, 8, 9, 10]),
            (39, vec![1, 2, 3, 9, 10]),
            (40, vec![2, 3, 10]),
            (41, vec![5, 6, 10]),
            (42, vec![1, 2, 3, 4, 5, 6, 9, 10, 11]),
            (43, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
            (44, vec![7, 8, 9, 10, 11, 12]),
        ] {
            for x in water_x {
                water[usize::from(y) * 64 + x] = true;
            }
        }
        let failed_road = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("Excelsior Boulevard".to_string()),
            area: false,
            bridge: false,
            points: vec![
                Coordinate {
                    lat: 44.926_133_8,
                    lon: -93.377_386_6,
                },
                Coordinate {
                    lat: 44.926_139_2,
                    lon: -93.376_967_3,
                },
                Coordinate {
                    lat: 44.926_144_4,
                    lon: -93.376_560_5,
                },
                Coordinate {
                    lat: 44.926_180_9,
                    lon: -93.375_825_3,
                },
                Coordinate {
                    lat: 44.926_613_8,
                    lon: -93.373_335_3,
                },
                Coordinate {
                    lat: 44.927_405,
                    lon: -93.368_475_3,
                },
            ],
        };
        assert!(
            !h3_landing_reaches_stable_interior_for_mask(
                &plan,
                64,
                64,
                crossing,
                &water,
                Some(&failed_road),
            )
            .expect("dry-island feasibility"),
            "a dry three-cell band is not viable when water and the face cap isolate it from stable interior land"
        );

        let alternate = Coordinate {
            lat: 44.937_000_844_274_95,
            lon: -93.376_165_330_704_85,
        };
        let alternate_road = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("Minnesota State Highway 7".to_string()),
            area: false,
            bridge: false,
            points: vec![
                Coordinate {
                    lat: 44.936_690_8,
                    lon: -93.377_667_6,
                },
                Coordinate {
                    lat: 44.937_169_9,
                    lon: -93.374_211_3,
                },
            ],
        };
        assert!(
            h3_landing_reaches_stable_interior_for_mask(
                &plan,
                64,
                64,
                alternate,
                &water,
                Some(&alternate_road),
            )
            .expect("alternate landing feasibility"),
            "a dry source-road exit that reaches stable interior land remains selectable"
        );
        assert_eq!(portal.side, HexSide::NorthWest);
    }

    #[test]
    fn regional_compression_keeps_the_selected_coordinate_not_the_highest_class_hash() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let target = &plan.portals[0];
        let selected_coordinate =
            spherical_interpolate(target.boundary[0], target.boundary[1], 0.30);
        let unselected_coordinate =
            spherical_interpolate(target.boundary[0], target.boundary[1], 0.70);
        let selected = Feature {
            kind: FeatureKind::Road,
            name: Some("selected lower-class crossing".to_string()),
            area: false,
            bridge: false,
            points: vec![plan.center, selected_coordinate],
        };
        let unselected = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("unselected higher-class crossing".to_string()),
            area: false,
            bridge: false,
            points: vec![plan.center, unselected_coordinate],
        };
        let mut source = source_for(&plan, vec![unselected, selected]);
        let contract = build_h3_seam_contract(&plan, &source, 64, 64)
            .expect("multi-candidate source contract");
        let edge = contract
            .edges
            .iter()
            .find(|edge| edge.edge_id == target.edge_id)
            .expect("target edge");
        assert_eq!(edge.viable_crossings.len(), 2);
        assert_eq!(edge.transport, Some(FeatureKind::MajorRoad));

        attach_h3_regional_plan(
            &mut source,
            H3RegionalCellPlan {
                ordinal: 0,
                cell: plan.cell.clone(),
                building_count: 0,
                facilities: Vec::new(),
                connections: vec![H3RegionalConnection {
                    edge_id: target.edge_id.clone(),
                    neighbor: target.neighbor.clone(),
                    coordinate: selected_coordinate,
                    transport: FeatureKind::Road,
                    bridge: false,
                    authoritative: true,
                    boundary_exit: true,
                }],
                closed_transport_crossings: Vec::new(),
            },
            64,
            64,
        )
        .expect("attach exact selected crossing");
        let transport = source
            .features
            .iter()
            .filter(|feature| {
                matches!(
                    feature.kind,
                    FeatureKind::Trail
                        | FeatureKind::Street
                        | FeatureKind::Road
                        | FeatureKind::MajorRoad
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(transport.len(), 1);
        assert_eq!(
            transport[0].name.as_deref(),
            Some("selected lower-class crossing")
        );
    }

    #[test]
    fn neighboring_cells_derive_the_same_shared_edge_contract() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let portal = &plan.portals[2];
        let neighbor =
            cell_plan(portal.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let road = Feature {
            kind: FeatureKind::MajorRoad,
            name: Some("reciprocal crossing".to_string()),
            area: false,
            bridge: false,
            points: vec![plan.center, neighbor.center],
        };
        let here = build_h3_seam_contract(&plan, &source_for(&plan, vec![road.clone()]), 64, 64)
            .expect("origin contract");
        let there = build_h3_seam_contract(&neighbor, &source_for(&neighbor, vec![road]), 64, 64)
            .expect("neighbor contract");
        let here_edge = here
            .edges
            .iter()
            .find(|edge| edge.edge_id == portal.edge_id)
            .expect("origin shared edge");
        let there_edge = there
            .edges
            .iter()
            .find(|edge| edge.edge_id == portal.edge_id)
            .expect("neighbor shared edge");
        assert_eq!(here_edge.terrain, there_edge.terrain);
        assert_eq!(here_edge.transport, there_edge.transport);
        let here_crossing = here_edge.crossing.expect("origin crossing");
        let there_crossing = there_edge.crossing.expect("neighbor crossing");
        assert!((here_crossing.lat - there_crossing.lat).abs() < 1e-9);
        assert!((here_crossing.lon - there_crossing.lon).abs() < 1e-9);
    }

    #[test]
    fn generated_neighbor_faces_open_selected_transport_and_cap_closed_crossings() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("origin plan");
        let portal = plan.portals[2].clone();
        let neighbor =
            cell_plan(portal.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let road = Feature {
            kind: FeatureKind::Road,
            name: Some("shared regional road".to_string()),
            area: false,
            bridge: false,
            points: vec![plan.center, neighbor.center],
        };
        let crossing =
            build_h3_seam_contract(&plan, &source_for(&plan, vec![road.clone()]), 64, 64)
                .expect("shared road contract")
                .edges
                .into_iter()
                .find(|edge| edge.edge_id == portal.edge_id)
                .and_then(|edge| edge.crossing)
                .expect("exact shared crossing");

        let selected_plan =
            |ordinal: usize, cell: &H3CellPlan, other: &H3CellPlan| H3RegionalCellPlan {
                ordinal,
                cell: cell.cell.clone(),
                building_count: 0,
                facilities: Vec::new(),
                connections: vec![H3RegionalConnection {
                    edge_id: portal.edge_id.clone(),
                    neighbor: other.cell.clone(),
                    coordinate: crossing,
                    transport: FeatureKind::Road,
                    bridge: false,
                    authoritative: true,
                    boundary_exit: false,
                }],
                closed_transport_crossings: Vec::new(),
            };
        let mut here_source = source_for(&plan, vec![road.clone()]);
        let mut there_source = source_for(&neighbor, vec![road.clone()]);
        attach_h3_regional_plan(&mut here_source, selected_plan(0, &plan, &neighbor), 64, 64)
            .expect("origin selected plan");
        attach_h3_regional_plan(
            &mut there_source,
            selected_plan(1, &neighbor, &plan),
            64,
            64,
        )
        .expect("neighbor selected plan");
        let here = generate_grid(here_source, 64, 64).expect("origin selected raster");
        let there = generate_grid(there_source, 64, 64).expect("neighbor selected raster");
        let mut grids = vec![here, there];
        crate::finalize_h3_batch_grid_seams(&mut grids).expect("reciprocal final raster seams");
        let here_profile = build_h3_grid_seam_profile(&grids[0]).expect("origin selected profile");
        let there_profile =
            build_h3_grid_seam_profile(&grids[1]).expect("neighbor selected profile");
        let selected = audit_h3_grid_seams(&[here_profile.clone(), there_profile.clone()]);
        assert!(selected.passed, "{}", selected.errors.join("; "));
        assert_eq!(selected.selected_transport_edges, 1);
        assert_eq!(selected.connected_transport_edges, 1);
        let mut downgraded_profile = there_profile;
        downgraded_profile
            .edges
            .iter_mut()
            .find(|edge| edge.edge_id == portal.edge_id)
            .and_then(|edge| edge.regional_transport.as_mut())
            .expect("selected transport directive")
            .band_transport[0] = Some(FeatureKind::Trail);
        let downgraded = audit_h3_grid_seams(&[here_profile, downgraded_profile]);
        assert!(!downgraded.passed);
        assert!(downgraded.errors.iter().any(|error| {
            error.contains(&portal.edge_id) && error.contains("selected Some(Road) landing")
        }));
        for grid in &grids {
            let report = crate::inspect_h3_regional_grid(grid).expect("regional route report");
            assert_eq!(report.connected_edges, vec![portal.edge_id.clone()]);
            let cell_plan = grid.source.h3.as_ref().expect("regional H3 plan");
            let band = h3_raster_sample_band(cell_plan, grid, crossing)
                .expect("selected exact landing band");
            assert!(
                band.into_iter()
                    .all(|(x, y)| grid.cell(x, y) == Some(MapCell::Road)),
                "the regional connector and connectivity repair must preserve the mapped road class"
            );
        }

        let closed_plan = |ordinal: usize, cell: &H3CellPlan| H3RegionalCellPlan {
            ordinal,
            cell: cell.cell.clone(),
            building_count: 0,
            facilities: Vec::new(),
            connections: Vec::new(),
            closed_transport_crossings: vec![H3ClosedTransportCrossing {
                edge_id: portal.edge_id.clone(),
                coordinate: crossing,
            }],
        };
        grids[0]
            .source
            .h3
            .as_mut()
            .expect("origin H3 plan")
            .regional = Some(closed_plan(0, &plan));
        grids[1]
            .source
            .h3
            .as_mut()
            .expect("neighbor H3 plan")
            .regional = Some(closed_plan(1, &neighbor));
        finalize_h3_regional_transport_seams(&mut grids[0]).expect("cap origin crossing");
        finalize_h3_regional_transport_seams(&mut grids[1]).expect("cap neighbor crossing");
        crate::finalize_h3_batch_grid_seams(&mut grids).expect("reciprocal capped raster seams");
        let closed = audit_h3_grid_seams(&[
            build_h3_grid_seam_profile(&grids[0]).expect("origin capped profile"),
            build_h3_grid_seam_profile(&grids[1]).expect("neighbor capped profile"),
        ]);
        assert!(closed.passed, "{}", closed.errors.join("; "));
        assert_eq!(closed.closed_transport_edges, 1);
        assert_eq!(closed.capped_transport_edges, 1);
    }

    #[test]
    fn capping_a_closed_road_restores_authoritative_shoreline_water() {
        let mut plan = plan_h3_cell(MINNEAPOLIS, 8).expect("origin plan");
        let portal = plan.portals[1].clone();
        let neighbor =
            cell_plan(portal.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
        let water_start = spherical_interpolate(start, end, 0.42);
        let water_end = spherical_interpolate(start, end, 0.58);
        let water = Feature {
            kind: FeatureKind::Water,
            name: Some("road-crossed shoreline".to_string()),
            area: true,
            bridge: false,
            points: vec![
                spherical_interpolate(water_start, plan.center, 0.08),
                spherical_interpolate(water_end, plan.center, 0.08),
                spherical_interpolate(water_end, neighbor.center, 0.08),
                spherical_interpolate(water_start, neighbor.center, 0.08),
                spherical_interpolate(water_start, plan.center, 0.08),
            ],
        };
        plan.regional = Some(H3RegionalCellPlan {
            ordinal: 0,
            cell: plan.cell.clone(),
            building_count: 0,
            facilities: Vec::new(),
            connections: Vec::new(),
            closed_transport_crossings: vec![H3ClosedTransportCrossing {
                edge_id: portal.edge_id,
                coordinate: portal.midpoint,
            }],
        });
        let mut grid = GeneratedGrid {
            source: source_for(&plan, vec![water]),
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        let band =
            h3_raster_sample_band(&plan, &grid, portal.midpoint).expect("closed shoreline band");
        for &(x, y) in &band {
            grid.cells[usize::from(y) * 64 + usize::from(x)] = MapCell::Road;
        }
        preserve_h3_authoritative_water_seams(&mut grid).expect("water defers to open road");
        assert!(
            band.iter()
                .all(|&(x, y)| grid.cell(x, y) == Some(MapCell::Road))
        );
        finalize_h3_regional_transport_seams(&mut grid).expect("cap closed shoreline road");
        assert!(
            band.iter()
                .all(|&(x, y)| grid.cell(x, y) == Some(MapCell::Water)),
            "closed transport must reveal the authoritative water underneath"
        );
    }

    #[test]
    fn final_grid_audit_catches_an_off_midpoint_water_seam() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let portal = plan.portals[1].clone();
        let neighbor =
            cell_plan(portal.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let (start, end) = canonical_edge_endpoints(portal.boundary[0], portal.boundary[1]);
        let edge_start = spherical_interpolate(start, end, 0.08);
        let edge_end = spherical_interpolate(start, end, 0.30);
        let water = Feature {
            kind: FeatureKind::Water,
            name: Some("off-midpoint lake".to_string()),
            area: true,
            bridge: false,
            points: vec![
                spherical_interpolate(edge_start, plan.center, 0.06),
                spherical_interpolate(edge_end, plan.center, 0.06),
                spherical_interpolate(edge_end, neighbor.center, 0.06),
                spherical_interpolate(edge_start, neighbor.center, 0.06),
                spherical_interpolate(edge_start, plan.center, 0.06),
            ],
        };
        let mut here = GeneratedGrid {
            source: source_for(&plan, vec![water.clone()]),
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        let mut there = GeneratedGrid {
            source: source_for(&neighbor, vec![water]),
            width: 64,
            height: 64,
            cells: vec![MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        preserve_h3_authoritative_water_seams(&mut here).expect("origin water seam");
        preserve_h3_authoritative_water_seams(&mut there).expect("neighbor water seam");

        let contract =
            build_h3_seam_contract(&plan, &here.source, 64, 64).expect("midpoint contract");
        assert_ne!(
            contract
                .edges
                .iter()
                .find(|edge| edge.edge_id == portal.edge_id)
                .expect("target edge")
                .terrain,
            H3EdgeTerrain::Water,
            "the legacy midpoint contract deliberately cannot see this lake"
        );
        let here_profile = build_h3_grid_seam_profile(&here).expect("origin raster profile");
        let mut there_profile =
            build_h3_grid_seam_profile(&there).expect("neighbor raster profile");
        let target = here_profile
            .edges
            .iter()
            .find(|edge| edge.edge_id == portal.edge_id)
            .expect("target profile");
        assert!(
            target.samples.iter().any(|sample| sample.source_water),
            "off-midpoint lake must be represented by edge samples"
        );
        assert!(!target.samples[H3_GRID_SEAM_SAMPLES / 2].source_water);
        let passed = audit_h3_grid_seams(&[here_profile.clone(), there_profile.clone()]);
        assert!(passed.passed, "{}", passed.errors.join("; "));
        assert!(passed.authoritative_water_samples > 0);
        assert_eq!(
            passed.authoritative_water_samples,
            passed.continuous_water_samples
        );

        let mut flooded_here = here_profile.clone();
        let mut flooded_there = there_profile.clone();
        for profile in [&mut flooded_here, &mut flooded_there] {
            for sample in &mut profile
                .edges
                .iter_mut()
                .find(|edge| edge.edge_id == portal.edge_id)
                .expect("target edge")
                .samples
            {
                if !sample.source_water {
                    sample.surface = H3GridSeamSurface::Water;
                }
            }
        }
        let flooded = audit_h3_grid_seams(&[flooded_here, flooded_there]);
        assert!(!flooded.passed);
        assert!(flooded.errors.iter().any(|error| {
            error.contains(&portal.edge_id) && error.contains("outside source geometry")
        }));

        let broken_sample = there_profile
            .edges
            .iter_mut()
            .find(|edge| edge.edge_id == portal.edge_id)
            .expect("target reciprocal profile")
            .samples
            .iter_mut()
            .find(|sample| sample.source_water)
            .expect("water sample");
        broken_sample.surface = H3GridSeamSurface::Ground;
        let broken = audit_h3_grid_seams(&[here_profile, there_profile]);
        assert!(!broken.passed);
        assert!(broken.errors.iter().any(|error| {
            error.contains(&portal.edge_id) && error.contains("clips authoritative water")
        }));
    }

    #[test]
    fn grid_seam_audit_rejects_a_tree_outline_but_accepts_continuous_forest() {
        let plan = plan_h3_cell(MINNEAPOLIS, 8).expect("H3 plan");
        let portal = plan.portals[0].clone();
        let neighbor =
            cell_plan(portal.neighbor.parse().expect("neighbor index")).expect("neighbor plan");
        let coordinates = (0..H3_GRID_SEAM_SAMPLES)
            .map(|index| {
                spherical_interpolate(
                    portal.boundary[0],
                    portal.boundary[1],
                    (index as f64 + 0.5) / H3_GRID_SEAM_SAMPLES as f64,
                )
            })
            .collect::<Vec<_>>();
        let edge = |neighbor: &str, surface, inner_surface| H3GridEdgeProfile {
            edge_id: portal.edge_id.clone(),
            neighbor: neighbor.to_string(),
            samples: coordinates
                .iter()
                .map(|&coordinate| H3GridSeamSample {
                    coordinate,
                    source_water: false,
                    raster_source_water: false,
                    surface,
                    transport: None,
                    inner_surface,
                })
                .collect(),
            regional_transport: None,
        };
        let profile = |cell: &str, edge| H3GridSeamProfile {
            cell: cell.to_string(),
            grid_width: 64,
            grid_height: 64,
            edges: vec![edge],
        };

        let outlined = audit_h3_grid_seams(&[
            profile(
                &plan.cell,
                edge(
                    &neighbor.cell,
                    H3GridSeamSurface::Tree,
                    H3GridSeamSurface::Ground,
                ),
            ),
            profile(
                &neighbor.cell,
                edge(
                    &plan.cell,
                    H3GridSeamSurface::Tree,
                    H3GridSeamSurface::Ground,
                ),
            ),
        ]);
        assert!(!outlined.passed);
        assert_eq!(outlined.tree_outline_edges, 2);
        assert!(
            outlined
                .errors
                .iter()
                .all(|error| error.contains("one-cell tree traces"))
        );

        let forest = audit_h3_grid_seams(&[
            profile(
                &plan.cell,
                edge(
                    &neighbor.cell,
                    H3GridSeamSurface::Tree,
                    H3GridSeamSurface::Tree,
                ),
            ),
            profile(
                &neighbor.cell,
                edge(&plan.cell, H3GridSeamSurface::Tree, H3GridSeamSurface::Tree),
            ),
        ]);
        assert!(forest.passed, "{}", forest.errors.join("; "));
        assert_eq!(forest.tree_outline_edges, 0);

        let category_mismatch = audit_h3_grid_seams(&[
            profile(
                &plan.cell,
                edge(
                    &neighbor.cell,
                    H3GridSeamSurface::Tree,
                    H3GridSeamSurface::Tree,
                ),
            ),
            profile(
                &neighbor.cell,
                edge(
                    &plan.cell,
                    H3GridSeamSurface::Ground,
                    H3GridSeamSurface::Ground,
                ),
            ),
        ]);
        assert!(!category_mismatch.passed);
        assert_eq!(
            category_mismatch.mismatched_surface_samples,
            H3_GRID_SEAM_SAMPLES
        );
        assert!(category_mismatch.errors.iter().any(|error| {
            error.contains(&portal.edge_id) && error.contains("surface categories")
        }));

        let mut road = profile(
            &plan.cell,
            edge(
                &neighbor.cell,
                H3GridSeamSurface::Transport,
                H3GridSeamSurface::Transport,
            ),
        );
        let mut street = profile(
            &neighbor.cell,
            edge(
                &plan.cell,
                H3GridSeamSurface::Transport,
                H3GridSeamSurface::Transport,
            ),
        );
        for sample in &mut road.edges[0].samples {
            sample.transport = Some(FeatureKind::Road);
        }
        for sample in &mut street.edges[0].samples {
            sample.transport = Some(FeatureKind::Street);
        }
        let class_mismatch = audit_h3_grid_seams(&[road, street]);
        assert!(!class_mismatch.passed);
        assert_eq!(
            class_mismatch.mismatched_transport_samples,
            H3_GRID_SEAM_SAMPLES
        );
        assert!(class_mismatch.errors.iter().any(|error| {
            error.contains(&portal.edge_id) && error.contains("transport classes")
        }));
    }

    #[test]
    fn center_and_six_neighbors_form_a_reciprocal_seam_mesh() {
        let batch = plan_h3_batch(MINNEAPOLIS, 8, 7).expect("seven-cell batch");
        let mut contracts = batch
            .cells
            .iter()
            .map(|entry| {
                build_h3_seam_contract(&entry.plan, &source_for(&entry.plan, Vec::new()), 64, 64)
                    .expect("natural contract")
            })
            .collect::<Vec<_>>();
        let audit = audit_h3_seam_contracts(&contracts);
        assert!(audit.passed, "{}", audit.errors.join("; "));
        assert_eq!(audit.cells, 7);
        assert_eq!(audit.internal_edges, 12);
        assert_eq!(audit.transport_edges, 0);

        let shared = contracts[0].edges[0].edge_id.clone();
        let reciprocal = contracts
            .iter_mut()
            .skip(1)
            .flat_map(|contract| &mut contract.edges)
            .find(|edge| edge.edge_id == shared)
            .expect("reciprocal edge");
        reciprocal.terrain = if reciprocal.terrain == H3EdgeTerrain::Water {
            H3EdgeTerrain::Trees
        } else {
            H3EdgeTerrain::Water
        };
        let broken = audit_h3_seam_contracts(&contracts);
        assert!(!broken.passed);
        assert!(broken.errors.iter().any(|error| error.contains(&shared)));
    }
}
