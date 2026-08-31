use std::{
    collections::BTreeMap,
    io::Read,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

const METERS_PER_MILE: f64 = 1_609.344;
const METERS_PER_LATITUDE_DEGREE: f64 = 111_320.0;
const OPENSTREETMAP_ATTRIBUTION: &str =
    "© OpenStreetMap contributors, queried through Overpass API";
const OVERPASS_USER_AGENT: &str = "crystal-mapgen/0.1 (coordinate-to-Crystal-modpack pipeline)";
const OVERPASS_ENDPOINTS: [&str; 3] = [
    "https://overpass-api.de/api/interpreter",
    "https://overpass.private.coffee/api/interpreter",
    "https://maps.mail.ru/osm/tools/overpass/api/interpreter",
];
const MAX_OVERPASS_ATTEMPTS: usize = 4;
static LAST_SUCCESSFUL_OVERPASS_ENDPOINT: AtomicUsize = AtomicUsize::new(0);
/// Bound each authoritative Overpass request by physical envelope rather than
/// by feature count. The latter is unknowable until after the request. The H3
/// resolution-6 Minneapolis halo is about 93 km², so it becomes four roughly
/// 23 km² quadrants instead of one overload-prone response.
const MAX_OVERPASS_BBOX_AREA_SQUARE_METERS: f64 = 70_000_000.0;
const MAX_OVERPASS_BBOX_SPAN_METERS: f64 = 12_000.0;
const MAX_OVERPASS_BBOX_PARTITIONS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    pub south: f64,
    pub west: f64,
    pub north: f64,
    pub east: f64,
}

impl BoundingBox {
    pub fn square_miles_around(center: Coordinate, side_miles: f64) -> Result<Self> {
        if !center.lat.is_finite()
            || !center.lon.is_finite()
            || !(-90.0..=90.0).contains(&center.lat)
            || !(-180.0..=180.0).contains(&center.lon)
        {
            bail!("latitude/longitude are outside valid geographic bounds");
        }
        if !side_miles.is_finite() || !(0.1..=10.0).contains(&side_miles) {
            bail!("--miles must be between 0.1 and 10");
        }
        let half_side_meters = side_miles * METERS_PER_MILE / 2.0;
        let latitude_delta = half_side_meters / METERS_PER_LATITUDE_DEGREE;
        let longitude_scale = METERS_PER_LATITUDE_DEGREE * center.lat.to_radians().cos();
        if longitude_scale.abs() < 1.0 {
            bail!("coordinates are too close to a pole for this projection");
        }
        let longitude_delta = half_side_meters / longitude_scale;
        Ok(Self {
            south: center.lat - latitude_delta,
            west: center.lon - longitude_delta,
            north: center.lat + latitude_delta,
            east: center.lon + longitude_delta,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureKind {
    Water,
    Park,
    Pitch,
    Building,
    Rail,
    Trail,
    Street,
    Road,
    MajorRoad,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Feature {
    pub kind: FeatureKind,
    pub name: Option<String>,
    pub area: bool,
    /// Exact OpenStreetMap bridge semantics for transport geometry. This is
    /// deliberately true only for `bridge=yes`; nearby layers or adjoining
    /// ways never imply that an untagged segment may cross water.
    #[serde(default)]
    pub bridge: bool,
    pub points: Vec<Coordinate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapSource {
    pub center: Coordinate,
    pub bounds: BoundingBox,
    pub attribution: String,
    pub features: Vec<Feature>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h3: Option<crate::H3CellPlan>,
}

#[derive(Debug, Deserialize)]
struct OverpassResponse {
    elements: Vec<OverpassElement>,
}

#[derive(Debug, Deserialize)]
struct OverpassElement {
    #[serde(default)]
    tags: BTreeMap<String, String>,
    #[serde(default)]
    center: Option<Coordinate>,
    #[serde(default)]
    geometry: Vec<Coordinate>,
    #[serde(default)]
    members: Vec<OverpassMember>,
}

#[derive(Debug, Deserialize)]
struct OverpassMember {
    #[serde(default)]
    role: String,
    #[serde(default)]
    geometry: Vec<Coordinate>,
}

pub fn fetch_neighborhood(center: Coordinate, side_miles: f64) -> Result<MapSource> {
    let bounds = BoundingBox::square_miles_around(center, side_miles)?;
    fetch_map_bounds(center, bounds)
}

/// Fetch one explicit geographic box. H3 callers use this for the padded
/// bounding box (or both antimeridian halves) around a cell.
pub fn fetch_map_bounds(center: Coordinate, bounds: BoundingBox) -> Result<MapSource> {
    fetch_map_bounds_with(center, bounds, |class, _quadrant, query| {
        let mut errors = Vec::new();
        for round in 1..=2 {
            match request_overpass(class, query) {
                Ok(response) => return Ok(response),
                Err(error) => {
                    errors.push(format!("round {round}: {error:#}"));
                    if round == 1 {
                        std::thread::sleep(Duration::from_secs(2));
                    }
                }
            }
        }
        bail!(
            "OpenStreetMap partition failed after two complete mirror rotations: {}",
            errors.join("; ")
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverpassQueryClass {
    Geometry,
    Building,
}

impl OverpassQueryClass {
    fn label(self) -> &'static str {
        match self {
            Self::Geometry => "geometry",
            Self::Building => "building",
        }
    }
}

fn fetch_map_bounds_with<F>(
    center: Coordinate,
    bounds: BoundingBox,
    mut fetch: F,
) -> Result<MapSource>
where
    F: FnMut(OverpassQueryClass, BoundingBox, &str) -> Result<String>,
{
    let quadrants = subdivide_overpass_bounds(bounds)?;
    let mut features = Vec::new();
    for (index, quadrant) in quadrants.iter().copied().enumerate() {
        for class in [OverpassQueryClass::Geometry, OverpassQueryClass::Building] {
            features.extend(fetch_partition_features(
                center,
                class,
                quadrant,
                index + 1,
                quadrants.len(),
                0,
                &mut fetch,
            )?);
        }
    }

    sort_and_deduplicate_features(&mut features);
    Ok(MapSource {
        center,
        bounds,
        attribution: OPENSTREETMAP_ATTRIBUTION.to_string(),
        features,
        h3: None,
    })
}

fn fetch_partition_features<F>(
    center: Coordinate,
    class: OverpassQueryClass,
    bounds: BoundingBox,
    index: usize,
    total: usize,
    depth: usize,
    fetch: &mut F,
) -> Result<Vec<Feature>>
where
    F: FnMut(OverpassQueryClass, BoundingBox, &str) -> Result<String>,
{
    let query = overpass_query(class, bounds);
    match fetch(class, bounds, &query) {
        Ok(response) => Ok(parse_overpass(center, bounds, &response)
            .with_context(|| {
                format!(
                    "parse OpenStreetMap Overpass {} response for bbox partition {index}/{total}",
                    class.label()
                )
            })?
            .features),
        Err(error) if depth < 2 => {
            let latitude_midpoint = (bounds.south + bounds.north) / 2.0;
            let longitude_midpoint = (bounds.west + bounds.east) / 2.0;
            let mut features = Vec::new();
            for child in [
                BoundingBox { south: bounds.south, west: bounds.west, north: latitude_midpoint, east: longitude_midpoint },
                BoundingBox { south: bounds.south, west: longitude_midpoint, north: latitude_midpoint, east: bounds.east },
                BoundingBox { south: latitude_midpoint, west: bounds.west, north: bounds.north, east: longitude_midpoint },
                BoundingBox { south: latitude_midpoint, west: longitude_midpoint, north: bounds.north, east: bounds.east },
            ] {
                features.extend(fetch_partition_features(
                    center, class, child, index, total, depth + 1, fetch,
                ).with_context(|| format!(
                    "subdivide failed {} partition {index}/{total} after: {error:#}",
                    class.label()
                ))?);
            }
            Ok(features)
        }
        Err(error) => Err(error).with_context(|| format!(
            "fetch OpenStreetMap Overpass {} response for bbox partition {index}/{total} ({},{},{},{})",
            class.label(), bounds.south, bounds.west, bounds.north, bounds.east
        )),
    }
}

fn overpass_query(class: OverpassQueryClass, bounds: BoundingBox) -> String {
    let bbox = format!(
        "{},{},{},{}",
        bounds.south, bounds.west, bounds.north, bounds.east
    );
    match class {
        OverpassQueryClass::Geometry => format!(
            "[out:json][timeout:180];(way[highway]({bbox});way[natural=water]({bbox});way[water]({bbox});way[waterway~\"^(river|stream|canal|riverbank)$\"]({bbox});relation[natural=water]({bbox});relation[waterway=riverbank]({bbox});way[leisure=park]({bbox});way[leisure=pitch]({bbox});way[railway]({bbox}););out geom;"
        ),
        // The planner consumes building representative points, not facade
        // polygon vertices. `out center` retains the authoritative OSM feature
        // and exact centroid at the semantic resolution needed by clustering.
        OverpassQueryClass::Building => {
            format!("[out:json][timeout:180];way[building]({bbox});out tags center;")
        }
    }
}

fn subdivide_overpass_bounds(bounds: BoundingBox) -> Result<Vec<BoundingBox>> {
    validate_overpass_bounds(bounds)?;
    let mut partitions = Vec::new();
    append_overpass_partitions(bounds, &mut partitions)?;
    Ok(partitions)
}

fn validate_overpass_bounds(bounds: BoundingBox) -> Result<()> {
    if !bounds.south.is_finite()
        || !bounds.west.is_finite()
        || !bounds.north.is_finite()
        || !bounds.east.is_finite()
        || !(-90.0..=90.0).contains(&bounds.south)
        || !(-90.0..=90.0).contains(&bounds.north)
        || !(-180.0..=180.0).contains(&bounds.west)
        || !(-180.0..=180.0).contains(&bounds.east)
        || bounds.south >= bounds.north
    {
        bail!("OpenStreetMap fetch bounds are outside valid geographic bounds");
    }
    if bounds.west >= bounds.east {
        bail!(
            "OpenStreetMap fetch bounds cross the antimeridian; callers must split them upstream"
        );
    }
    Ok(())
}

fn append_overpass_partitions(
    bounds: BoundingBox,
    partitions: &mut Vec<BoundingBox>,
) -> Result<()> {
    let (latitude_span_meters, longitude_span_meters) = overpass_bbox_dimensions_meters(bounds);
    let area_square_meters = latitude_span_meters * longitude_span_meters;
    let requires_subdivision = latitude_span_meters > MAX_OVERPASS_BBOX_SPAN_METERS
        || longitude_span_meters > MAX_OVERPASS_BBOX_SPAN_METERS
        || area_square_meters > MAX_OVERPASS_BBOX_AREA_SQUARE_METERS;
    if !requires_subdivision {
        if partitions.len() == MAX_OVERPASS_BBOX_PARTITIONS {
            bail!(
                "OpenStreetMap fetch requires more than {MAX_OVERPASS_BBOX_PARTITIONS} bounded partitions"
            );
        }
        partitions.push(bounds);
        return Ok(());
    }

    let latitude_midpoint = (bounds.south + bounds.north) / 2.0;
    let longitude_midpoint = (bounds.west + bounds.east) / 2.0;
    for quadrant in [
        BoundingBox {
            south: bounds.south,
            west: bounds.west,
            north: latitude_midpoint,
            east: longitude_midpoint,
        },
        BoundingBox {
            south: bounds.south,
            west: longitude_midpoint,
            north: latitude_midpoint,
            east: bounds.east,
        },
        BoundingBox {
            south: latitude_midpoint,
            west: bounds.west,
            north: bounds.north,
            east: longitude_midpoint,
        },
        BoundingBox {
            south: latitude_midpoint,
            west: longitude_midpoint,
            north: bounds.north,
            east: bounds.east,
        },
    ] {
        append_overpass_partitions(quadrant, partitions)?;
    }
    Ok(())
}

fn overpass_bbox_dimensions_meters(bounds: BoundingBox) -> (f64, f64) {
    let latitude_span_meters = (bounds.north - bounds.south) * METERS_PER_LATITUDE_DEGREE;
    let latitude_midpoint = (bounds.south + bounds.north) / 2.0;
    let longitude_span_meters = (bounds.east - bounds.west)
        * METERS_PER_LATITUDE_DEGREE
        * latitude_midpoint.to_radians().cos().abs();
    (latitude_span_meters, longitude_span_meters)
}

fn sort_and_deduplicate_features(features: &mut Vec<Feature>) {
    features.sort_by(compare_features);
    features.dedup_by(|left, right| compare_features(left, right).is_eq());
}

fn compare_features(left: &Feature, right: &Feature) -> std::cmp::Ordering {
    left.kind
        .cmp(&right.kind)
        .then_with(|| left.name.cmp(&right.name))
        .then_with(|| left.area.cmp(&right.area))
        .then_with(|| left.bridge.cmp(&right.bridge))
        .then_with(|| compare_coordinate_sequences(&left.points, &right.points))
}

fn compare_coordinate_sequences(left: &[Coordinate], right: &[Coordinate]) -> std::cmp::Ordering {
    for (left, right) in left.iter().zip(right) {
        let ordering = left
            .lat
            .total_cmp(&right.lat)
            .then_with(|| left.lon.total_cmp(&right.lon));
        if !ordering.is_eq() {
            return ordering;
        }
    }
    left.len().cmp(&right.len())
}

fn request_overpass(class: OverpassQueryClass, query: &str) -> Result<String> {
    let starting_endpoint = LAST_SUCCESSFUL_OVERPASS_ENDPOINT.load(Ordering::Relaxed);
    let (body, successful_endpoint) = request_overpass_with_start(
        class.label(),
        query,
        &OVERPASS_ENDPOINTS,
        starting_endpoint,
        request_overpass_once,
        std::thread::sleep,
    )?;
    LAST_SUCCESSFUL_OVERPASS_ENDPOINT.store(successful_endpoint, Ordering::Relaxed);
    Ok(body)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OverpassRequestFailure {
    summary: String,
    retry_after: Option<Duration>,
    retryable: bool,
}

impl OverpassRequestFailure {
    fn retryable(summary: impl Into<String>, retry_after: Option<Duration>) -> Self {
        Self {
            summary: summary.into(),
            retry_after,
            retryable: true,
        }
    }

    fn terminal(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            retry_after: None,
            retryable: false,
        }
    }
}

#[cfg(test)]
fn request_overpass_with<R, S>(
    response_label: &str,
    query: &str,
    endpoints: &[&str],
    requester: R,
    sleeper: S,
) -> Result<String>
where
    R: FnMut(&str, &str) -> std::result::Result<String, OverpassRequestFailure>,
    S: FnMut(Duration),
{
    request_overpass_with_start(response_label, query, endpoints, 0, requester, sleeper)
        .map(|(body, _successful_endpoint)| body)
}

fn request_overpass_with_start<R, S>(
    response_label: &str,
    query: &str,
    endpoints: &[&str],
    starting_endpoint: usize,
    mut requester: R,
    mut sleeper: S,
) -> Result<(String, usize)>
where
    R: FnMut(&str, &str) -> std::result::Result<String, OverpassRequestFailure>,
    S: FnMut(Duration),
{
    if endpoints.is_empty() {
        bail!("request OpenStreetMap Overpass: no endpoints configured");
    }

    let mut reports = Vec::with_capacity(MAX_OVERPASS_ATTEMPTS);
    for attempt in 0..MAX_OVERPASS_ATTEMPTS {
        let endpoint_index = (starting_endpoint + attempt) % endpoints.len();
        let endpoint = endpoints[endpoint_index];
        match requester(endpoint, query) {
            Ok(body) => return Ok((body, endpoint_index)),
            Err(failure) => {
                let retry_after = failure
                    .retry_after
                    .map(|delay| format!(" (Retry-After {}s)", delay.as_secs()))
                    .unwrap_or_default();
                reports.push(format!(
                    "attempt {} at {endpoint}: {}{retry_after}",
                    attempt + 1,
                    failure.summary
                ));

                if !failure.retryable || attempt + 1 == MAX_OVERPASS_ATTEMPTS {
                    return Err(anyhow!(
                        "request OpenStreetMap Overpass {response_label} response failed after {} attempt{}: {}",
                        reports.len(),
                        if reports.len() == 1 { "" } else { "s" },
                        reports.join("; ")
                    ));
                }

                let delay = failure
                    .retry_after
                    .unwrap_or_else(|| Duration::from_secs(1_u64 << attempt));
                sleeper(delay);
            }
        }
    }

    unreachable!("the bounded Overpass loop returns on success or its final failure")
}

fn request_overpass_once(
    endpoint: &str,
    query: &str,
) -> std::result::Result<String, OverpassRequestFailure> {
    match ureq::post(endpoint)
        .timeout(Duration::from_secs(90))
        .set("User-Agent", OVERPASS_USER_AGENT)
        .send_form(&[("data", query)])
    {
        Ok(response) => read_response_body(response.into_reader()).map_err(|error| {
            OverpassRequestFailure::retryable(
                format!("could not read successful response body: {error:#}"),
                None,
            )
        }),
        Err(ureq::Error::Status(status, response)) => {
            let summary = format!("HTTP {status} {}", response.status_text());
            let retryable = status == 408 || status == 425 || status == 429 || status >= 500;
            if retryable {
                let retry_after = response.header("Retry-After").and_then(parse_retry_after);
                Err(OverpassRequestFailure::retryable(summary, retry_after))
            } else {
                Err(OverpassRequestFailure::terminal(summary))
            }
        }
        Err(ureq::Error::Transport(error)) => Err(OverpassRequestFailure::retryable(
            format!("transport error: {error}"),
            None,
        )),
    }
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    value.trim().parse::<u64>().ok().map(Duration::from_secs)
}

fn read_response_body(mut reader: impl Read) -> Result<String> {
    // `ureq::Response::into_string` imposes a small convenience cap. Real
    // five-mile-radius urban extracts legitimately exceed it because building
    // geometry dominates the response, so stream the complete authoritative
    // payload instead of silently shrinking the requested region.
    let mut body = String::new();
    reader
        .read_to_string(&mut body)
        .context("read OpenStreetMap Overpass response")?;
    Ok(body)
}

pub fn parse_overpass(center: Coordinate, bounds: BoundingBox, json: &str) -> Result<MapSource> {
    let response: OverpassResponse =
        serde_json::from_str(json).context("parse OpenStreetMap Overpass JSON")?;
    let mut features = Vec::new();
    for element in response.elements {
        let Some(kind) = classify(&element.tags) else {
            continue;
        };
        let name = element.tags.get("name").cloned();
        let area = matches!(
            kind,
            FeatureKind::Park | FeatureKind::Pitch | FeatureKind::Building
        ) || (kind == FeatureKind::Water
            && (element
                .tags
                .get("natural")
                .is_some_and(|value| value == "water")
                || element.tags.contains_key("water")
                || element
                    .tags
                    .get("waterway")
                    .is_some_and(|value| value == "riverbank")));
        let bridge = matches!(
            kind,
            FeatureKind::Trail | FeatureKind::Street | FeatureKind::Road | FeatureKind::MajorRoad
        ) && element
            .tags
            .get("bridge")
            .is_some_and(|value| value == "yes");
        if element.geometry.len() >= 2 {
            features.push(Feature {
                kind,
                name: name.clone(),
                area,
                bridge,
                points: element.geometry,
            });
        } else if kind == FeatureKind::Building
            && let Some(center) = element.center
        {
            features.push(Feature {
                kind,
                name: name.clone(),
                area,
                bridge,
                points: vec![center],
            });
        }
        for points in stitch_outer_rings(element.members) {
            features.push(Feature {
                kind,
                name: name.clone(),
                area,
                bridge,
                points,
            });
        }
    }
    if features.is_empty() {
        bail!("OpenStreetMap returned no usable neighborhood features");
    }
    features.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.bridge.cmp(&right.bridge))
            .then_with(|| left.points.len().cmp(&right.points.len()))
    });
    Ok(MapSource {
        center,
        bounds,
        attribution: OPENSTREETMAP_ATTRIBUTION.to_string(),
        features,
        h3: None,
    })
}

fn stitch_outer_rings(members: Vec<OverpassMember>) -> Vec<Vec<Coordinate>> {
    const ENDPOINT_EPSILON: f64 = 1e-7;
    let same_point = |left: Coordinate, right: Coordinate| {
        (left.lat - right.lat).abs() <= ENDPOINT_EPSILON
            && (left.lon - right.lon).abs() <= ENDPOINT_EPSILON
    };
    let mut segments = members
        .into_iter()
        .filter(|member| member.role != "inner" && member.geometry.len() >= 2)
        .map(|member| member.geometry)
        .collect::<Vec<_>>();
    let mut rings = Vec::new();
    while !segments.is_empty() {
        let mut ring = segments.remove(0);
        loop {
            let Some(first) = ring.first().copied() else {
                break;
            };
            let Some(last) = ring.last().copied() else {
                break;
            };
            if ring.len() >= 4 && same_point(first, last) {
                if let Some(last) = ring.last_mut() {
                    *last = first;
                }
                rings.push(ring);
                break;
            }
            let Some((index, reverse)) =
                segments.iter().enumerate().find_map(|(index, segment)| {
                    let start = *segment.first()?;
                    let end = *segment.last()?;
                    if same_point(last, start) {
                        Some((index, false))
                    } else if same_point(last, end) {
                        Some((index, true))
                    } else {
                        None
                    }
                })
            else {
                // An incomplete outer relation is not a polygon. Discard it
                // rather than painting each member as an invented lake shard.
                break;
            };
            let mut next = segments.remove(index);
            if reverse {
                next.reverse();
            }
            ring.extend(next.into_iter().skip(1));
        }
    }
    rings
}

fn classify(tags: &BTreeMap<String, String>) -> Option<FeatureKind> {
    if tags.get("natural").is_some_and(|value| value == "water")
        || tags.contains_key("water")
        || tags.get("waterway").is_some_and(|value| {
            matches!(value.as_str(), "river" | "stream" | "canal" | "riverbank")
        })
    {
        return Some(FeatureKind::Water);
    }
    match tags.get("leisure").map(String::as_str) {
        Some("park") | Some("garden") | Some("nature_reserve") => return Some(FeatureKind::Park),
        Some("pitch") | Some("playground") => return Some(FeatureKind::Pitch),
        _ => {}
    }
    if tags.contains_key("building") {
        return Some(FeatureKind::Building);
    }
    if tags.contains_key("railway") {
        return Some(FeatureKind::Rail);
    }
    match tags.get("highway").map(String::as_str) {
        Some(
            "motorway" | "motorway_link" | "trunk" | "trunk_link" | "primary" | "primary_link"
            | "secondary" | "secondary_link",
        ) => Some(FeatureKind::MajorRoad),
        Some("tertiary" | "tertiary_link") => Some(FeatureKind::Road),
        Some("residential" | "unclassified" | "pedestrian") => Some(FeatureKind::Street),
        // Driveways and parking aisles are below one Crystal metatile at this
        // scale. A named service street is retained; anonymous ones would
        // otherwise turn most city blocks into pavement.
        Some("service") if tags.contains_key("name") => Some(FeatureKind::Street),
        Some("cycleway" | "footway" | "path" | "track" | "steps") => Some(FeatureKind::Trail),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_mile_bounds_are_centered_and_geographically_square() {
        let center = Coordinate {
            lat: 44.9475,
            lon: -93.3253,
        };
        let bounds = BoundingBox::square_miles_around(center, 1.0).expect("valid bounds");
        assert!(((bounds.north + bounds.south) / 2.0 - center.lat).abs() < 1e-10);
        assert!(((bounds.east + bounds.west) / 2.0 - center.lon).abs() < 1e-10);
        let north_south_meters = (bounds.north - bounds.south) * METERS_PER_LATITUDE_DEGREE;
        let east_west_meters = (bounds.east - bounds.west)
            * METERS_PER_LATITUDE_DEGREE
            * center.lat.to_radians().cos();
        assert!((north_south_meters - METERS_PER_MILE).abs() < 0.01);
        assert!((east_west_meters - METERS_PER_MILE).abs() < 0.01);
    }

    #[test]
    fn parses_way_and_relation_geometry_without_inner_water_holes() {
        let json = r#"{"elements":[
          {"tags":{"highway":"primary","name":"Lake Street"},"geometry":[{"lat":1.0,"lon":2.0},{"lat":1.1,"lon":2.1}]},
          {"tags":{"waterway":"river","name":"Creek"},"geometry":[{"lat":1.0,"lon":2.02},{"lat":1.04,"lon":2.05},{"lat":1.08,"lon":2.04}]},
          {"tags":{"natural":"water","name":"Lake"},"members":[
            {"role":"outer","geometry":[{"lat":1.0,"lon":2.0},{"lat":1.0,"lon":2.1}]},
            {"role":"outer","geometry":[{"lat":1.1,"lon":2.1},{"lat":1.0,"lon":2.1}]},
            {"role":"outer","geometry":[{"lat":1.1,"lon":2.1},{"lat":1.0,"lon":2.0}]},
            {"role":"inner","geometry":[{"lat":1.0,"lon":2.0},{"lat":1.1,"lon":2.1}]}
          ]}
        ]}"#;
        let center = Coordinate { lat: 1.0, lon: 2.0 };
        let bounds = BoundingBox::square_miles_around(center, 1.0).expect("bounds");
        let source = parse_overpass(center, bounds, json).expect("parse");
        assert_eq!(source.features.len(), 3);
        assert!(
            source
                .features
                .iter()
                .any(|feature| feature.kind == FeatureKind::MajorRoad)
        );
        let lake = source
            .features
            .iter()
            .find(|feature| feature.name.as_deref() == Some("Lake"))
            .expect("water relation");
        assert_eq!(
            lake.points.len(),
            4,
            "outer relation ways must form one ring"
        );
        assert_eq!(lake.points.first(), lake.points.last());
        let river = source
            .features
            .iter()
            .find(|feature| feature.name.as_deref() == Some("Creek"))
            .expect("linear river");
        assert_eq!(river.kind, FeatureKind::Water);
        assert!(
            !river.area,
            "a river centerline must not fill its meander as a lake polygon"
        );
    }

    #[test]
    fn overpass_reader_keeps_large_regional_payloads_complete() {
        let payload = format!("{{\"elements\":[]}}{}", " ".repeat(12 * 1024 * 1024));
        let read = read_response_body(std::io::Cursor::new(payload.as_bytes()))
            .expect("read large regional response");
        assert_eq!(read.len(), payload.len());
        assert_eq!(read, payload);
    }

    #[test]
    fn overpass_retries_rotate_official_endpoints_without_changing_the_query() {
        let endpoints = ["primary", "secondary", "tertiary"];
        let mut requests = Vec::new();
        let mut sleeps = Vec::new();
        let response = request_overpass_with(
            "geometry",
            "the exact overpass query",
            &endpoints,
            |endpoint, query| {
                requests.push((endpoint.to_string(), query.to_string()));
                match requests.len() {
                    1 => Err(OverpassRequestFailure::retryable("HTTP 504", None)),
                    2 => Err(OverpassRequestFailure::retryable(
                        "connection refused",
                        None,
                    )),
                    _ => Ok("{\"elements\":[]}".to_string()),
                }
            },
            |delay| sleeps.push(delay),
        )
        .expect("third official endpoint succeeds");

        assert_eq!(response, "{\"elements\":[]}");
        assert_eq!(
            requests,
            [
                (
                    "primary".to_string(),
                    "the exact overpass query".to_string()
                ),
                (
                    "secondary".to_string(),
                    "the exact overpass query".to_string()
                ),
                (
                    "tertiary".to_string(),
                    "the exact overpass query".to_string()
                ),
            ]
        );
        assert_eq!(
            sleeps,
            [
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(2)
            ]
        );
    }

    #[test]
    fn overpass_success_affinity_becomes_the_next_query_start() {
        let endpoints = ["primary", "secondary", "tertiary"];
        let mut first_requests = Vec::new();
        let (first_body, preferred_endpoint) = request_overpass_with_start(
            "geometry",
            "first query",
            &endpoints,
            0,
            |endpoint, _query| {
                first_requests.push(endpoint.to_string());
                if endpoint == "tertiary" {
                    Ok("first response".to_string())
                } else {
                    Err(OverpassRequestFailure::retryable("unavailable", None))
                }
            },
            |_delay| {},
        )
        .expect("tertiary endpoint succeeds");
        assert_eq!(first_body, "first response");
        assert_eq!(first_requests, ["primary", "secondary", "tertiary"]);
        assert_eq!(preferred_endpoint, 2);

        let mut next_requests = Vec::new();
        let (next_body, next_preferred_endpoint) = request_overpass_with_start(
            "building",
            "next query",
            &endpoints,
            preferred_endpoint,
            |endpoint, _query| {
                next_requests.push(endpoint.to_string());
                Ok("next response".to_string())
            },
            |_delay| {},
        )
        .expect("preferred endpoint succeeds immediately");
        assert_eq!(next_body, "next response");
        assert_eq!(next_requests, ["tertiary"]);
        assert_eq!(next_preferred_endpoint, 2);
    }

    #[test]
    fn overpass_affinity_wrap_gives_the_preferred_host_a_fourth_attempt() {
        let endpoints = ["primary", "secondary", "tertiary"];
        let mut requests = Vec::new();
        let mut sleeps = Vec::new();
        let (_, preferred_endpoint) = request_overpass_with_start(
            "geometry",
            "query",
            &endpoints,
            2,
            |endpoint, _query| {
                requests.push(endpoint.to_string());
                if requests.len() == 4 {
                    Ok("recovered".to_string())
                } else {
                    Err(OverpassRequestFailure::retryable("transient", None))
                }
            },
            |delay| sleeps.push(delay),
        )
        .expect("preferred host recovers on the bounded fourth attempt");

        assert_eq!(requests, ["tertiary", "primary", "secondary", "tertiary"]);
        assert_eq!(
            sleeps,
            [
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(2),
                std::time::Duration::from_secs(4),
            ]
        );
        assert_eq!(preferred_endpoint, 2);
    }

    #[test]
    fn overpass_429_honors_numeric_retry_after_before_rotating() {
        let endpoints = ["primary", "secondary"];
        let mut attempts = 0;
        let mut sleeps = Vec::new();
        request_overpass_with(
            "building",
            "query",
            &endpoints,
            |_endpoint, _query| {
                attempts += 1;
                if attempts == 1 {
                    Err(OverpassRequestFailure::retryable(
                        "HTTP 429 Too Many Requests",
                        Some(std::time::Duration::from_secs(17)),
                    ))
                } else {
                    Ok("done".to_string())
                }
            },
            |delay| sleeps.push(delay),
        )
        .expect("retry succeeds");

        assert_eq!(sleeps, [std::time::Duration::from_secs(17)]);
        assert_eq!(
            parse_retry_after("17"),
            Some(std::time::Duration::from_secs(17))
        );
        assert_eq!(parse_retry_after("not-a-delay"), None);
    }

    #[test]
    fn overpass_retry_exhaustion_is_capped_and_reports_every_attempt() {
        let endpoints = ["primary", "secondary", "tertiary"];
        let mut requests = Vec::new();
        let mut sleeps = Vec::new();
        let error = request_overpass_with(
            "geometry",
            "query",
            &endpoints,
            |endpoint, _query| {
                requests.push(endpoint.to_string());
                Err(OverpassRequestFailure::retryable(
                    format!("failure {}", requests.len()),
                    None,
                ))
            },
            |delay| sleeps.push(delay),
        )
        .expect_err("all endpoints fail");

        assert_eq!(requests, ["primary", "secondary", "tertiary", "primary"]);
        assert_eq!(
            sleeps,
            [
                std::time::Duration::from_secs(1),
                std::time::Duration::from_secs(2),
                std::time::Duration::from_secs(4),
            ]
        );
        let report = format!("{error:#}");
        for expected in [
            "attempt 1 at primary: failure 1",
            "attempt 2 at secondary: failure 2",
            "attempt 3 at tertiary: failure 3",
            "attempt 4 at primary: failure 4",
        ] {
            assert!(
                report.contains(expected),
                "missing {expected:?} in {report}"
            );
        }
        assert!(report.contains("after 4 attempts"));
    }

    #[test]
    fn regional_overpass_box_splits_into_four_exact_coverage_quadrants() {
        let bounds = BoundingBox {
            south: 44.89045670305877,
            west: -93.39174746387613,
            north: 44.98168565049287,
            east: -93.27573099465849,
        };
        let latitude_midpoint = (bounds.south + bounds.north) / 2.0;
        let longitude_midpoint = (bounds.west + bounds.east) / 2.0;
        let quadrants = subdivide_overpass_bounds(bounds).expect("subdivide regional H3 box");

        assert_eq!(
            quadrants,
            [
                BoundingBox {
                    south: bounds.south,
                    west: bounds.west,
                    north: latitude_midpoint,
                    east: longitude_midpoint,
                },
                BoundingBox {
                    south: bounds.south,
                    west: longitude_midpoint,
                    north: latitude_midpoint,
                    east: bounds.east,
                },
                BoundingBox {
                    south: latitude_midpoint,
                    west: bounds.west,
                    north: bounds.north,
                    east: longitude_midpoint,
                },
                BoundingBox {
                    south: latitude_midpoint,
                    west: longitude_midpoint,
                    north: bounds.north,
                    east: bounds.east,
                },
            ]
        );
        let original_area = (bounds.north - bounds.south) * (bounds.east - bounds.west);
        let quadrant_area = quadrants
            .iter()
            .map(|quadrant| (quadrant.north - quadrant.south) * (quadrant.east - quadrant.west))
            .sum::<f64>();
        assert!((quadrant_area - original_area).abs() < 1e-15);
        for (index, left) in quadrants.iter().enumerate() {
            for right in &quadrants[index + 1..] {
                let latitude_overlap = left.north.min(right.north) - left.south.max(right.south);
                let longitude_overlap = left.east.min(right.east) - left.west.max(right.west);
                assert!(
                    latitude_overlap <= 0.0 || longitude_overlap <= 0.0,
                    "quadrant interiors overlap: {left:?} and {right:?}"
                );
            }
        }
    }

    #[test]
    fn subdivided_fetch_preserves_query_classes_and_exactly_deduplicates_features() {
        let center = Coordinate {
            lat: 44.9475196,
            lon: -93.3253477,
        };
        let bounds = BoundingBox {
            south: 44.89045670305877,
            west: -93.39174746387613,
            north: 44.98168565049287,
            east: -93.27573099465849,
        };
        let geometry_json = r#"{"elements":[
          {"tags":{"highway":"primary","name":"Exact duplicate"},"geometry":[{"lat":44.95,"lon":-93.34},{"lat":44.95,"lon":-93.33}]},
          {"tags":{"highway":"primary","name":"Coincident other name"},"geometry":[{"lat":44.95,"lon":-93.34},{"lat":44.95,"lon":-93.33}]},
          {"tags":{"highway":"primary","name":"Exact duplicate","bridge":"yes"},"geometry":[{"lat":44.95,"lon":-93.34},{"lat":44.95,"lon":-93.33}]}
        ]}"#;
        let building_json = r#"{"elements":[
          {"tags":{"building":"house","name":"Same house"},"center":{"lat":44.95,"lon":-93.32}},
          {"tags":{"building":"house","name":"Coincident other house"},"center":{"lat":44.95,"lon":-93.32}}
        ]}"#;
        let mut requests = Vec::new();
        let source = fetch_map_bounds_with(center, bounds, |class, quadrant, query| {
            requests.push((class, quadrant, query.to_string()));
            Ok(match class {
                OverpassQueryClass::Geometry => geometry_json.to_string(),
                OverpassQueryClass::Building => building_json.to_string(),
            })
        })
        .expect("fetch all authoritative quadrants");

        assert_eq!(source.center, center);
        assert_eq!(source.bounds, bounds);
        assert_eq!(
            source.attribution,
            "© OpenStreetMap contributors, queried through Overpass API"
        );
        assert_eq!(source.features.len(), 5);
        assert_eq!(
            source
                .features
                .iter()
                .filter(|feature| feature.name.as_deref() == Some("Exact duplicate"))
                .count(),
            2,
            "the unbridged and bridged coincident ways remain distinct"
        );
        assert_eq!(
            source
                .features
                .iter()
                .filter(|feature| feature.name.as_deref() == Some("Same house"))
                .count(),
            1,
            "the same feature returned by all four boxes collapses exactly once"
        );

        assert_eq!(requests.len(), 8);
        for (index, chunk) in requests.chunks_exact(2).enumerate() {
            assert_eq!(chunk[0].0, OverpassQueryClass::Geometry);
            assert_eq!(chunk[1].0, OverpassQueryClass::Building);
            assert_eq!(chunk[0].1, chunk[1].1);
            let quadrant = chunk[0].1;
            let bbox = format!(
                "{},{},{},{}",
                quadrant.south, quadrant.west, quadrant.north, quadrant.east
            );
            assert_eq!(
                chunk[0].2,
                format!(
                    "[out:json][timeout:180];(way[highway]({bbox});way[natural=water]({bbox});way[water]({bbox});way[waterway~\"^(river|stream|canal|riverbank)$\"]({bbox});relation[natural=water]({bbox});relation[waterway=riverbank]({bbox});way[leisure=park]({bbox});way[leisure=pitch]({bbox});way[railway]({bbox}););out geom;"
                ),
                "geometry selector changed in quadrant {index}"
            );
            assert_eq!(
                chunk[1].2,
                format!("[out:json][timeout:180];way[building]({bbox});out tags center;"),
                "building selector changed in quadrant {index}"
            );
        }
    }

    #[test]
    fn parses_building_centers_without_unused_polygon_geometry() {
        let json = r#"{"elements":[
          {"tags":{"building":"house"},"center":{"lat":44.95,"lon":-93.32}}
        ]}"#;
        let center = Coordinate {
            lat: 44.95,
            lon: -93.32,
        };
        let bounds = BoundingBox::square_miles_around(center, 10.0).expect("bounds");
        let source = parse_overpass(center, bounds, json).expect("parse building center");
        assert_eq!(source.features.len(), 1);
        assert_eq!(source.features[0].kind, FeatureKind::Building);
        assert_eq!(source.features[0].points, vec![center]);
    }

    #[test]
    fn bridge_authority_requires_exact_bridge_yes_on_that_transport_way() {
        let json = r#"{"elements":[
          {"tags":{"highway":"primary","name":"Exact bridge","bridge":"yes"},"geometry":[{"lat":44.95,"lon":-93.33},{"lat":44.95,"lon":-93.32}]},
          {"tags":{"highway":"primary","name":"Layer only","layer":"1"},"geometry":[{"lat":44.951,"lon":-93.33},{"lat":44.951,"lon":-93.32}]},
          {"tags":{"highway":"motorway_link","name":"Adjacent ramp"},"geometry":[{"lat":44.949,"lon":-93.33},{"lat":44.949,"lon":-93.32}]}
        ]}"#;
        let center = Coordinate {
            lat: 44.95,
            lon: -93.325,
        };
        let bounds = BoundingBox::square_miles_around(center, 1.0).expect("bounds");
        let source = parse_overpass(center, bounds, json).expect("parse bridge tags");
        let bridge_by_name = source
            .features
            .iter()
            .map(|feature| (feature.name.as_deref().expect("named way"), feature.bridge))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(bridge_by_name["Exact bridge"], true);
        assert_eq!(bridge_by_name["Layer only"], false);
        assert_eq!(bridge_by_name["Adjacent ramp"], false);
    }

    #[test]
    fn normalized_sources_without_bridge_metadata_default_to_non_bridge() {
        let feature: Feature = serde_json::from_str(
            r#"{"kind":"road","name":"cached road","area":false,"points":[{"lat":44.95,"lon":-93.33},{"lat":44.95,"lon":-93.32}]}"#,
        )
        .expect("deserialize pre-bridge normalized source feature");
        assert!(!feature.bridge);
    }
}
