use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::{BoundingBox, Coordinate};

const METERS_PER_MILE: f64 = 1_609.344;
const WGS84_SEMI_MAJOR_AXIS_METERS: f64 = 6_378_137.0;
const WGS84_FLATTENING: f64 = 1.0 / 298.257_223_563;
const TRANSVERSE_MERCATOR_SCALE: f64 = 0.9996;

/// An integer address in the global metatile lattice used by a generated map.
///
/// Local map coordinates must never be used for snapping, hashing, thinning,
/// or procedural decisions. Two overlapping windows can instead compare these
/// addresses directly and will select the same metatile for the same place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WorldCell {
    pub x: i64,
    pub y: i64,
}

impl WorldCell {
    /// Stable procedural entropy for this geographic cell.
    ///
    /// The salt identifies the semantic layer (trees, flowers, house variant,
    /// and so on). Hashing a local map coordinate would move the decoration
    /// whenever the generation window moves.
    pub fn stable_hash(self, salt: u64) -> u64 {
        let mut value = (self.x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (self.y as u64).rotate_left(29)
            ^ salt.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectedPoint {
    pub east_meters: f64,
    pub north_meters: f64,
}

/// A deterministic, meter-scale projection frame.
///
/// The frame deliberately uses signed northing instead of UTM false northing,
/// keeping coordinates continuous across the equator. Adjacent generation
/// windows must use the same zone; a half-mile shift is far too small to change
/// zones except directly on one of the six-degree seams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldProjection {
    pub zone: u8,
}

impl WorldProjection {
    pub fn around(center: Coordinate) -> Result<Self> {
        validate_coordinate(center)?;
        if !(-80.0..=84.0).contains(&center.lat) {
            bail!("global map lattice supports latitudes from 80°S through 84°N");
        }
        let zone = (((center.lon + 180.0) / 6.0).floor() as i32 + 1).clamp(1, 60) as u8;
        Ok(Self { zone })
    }

    pub fn project(self, coordinate: Coordinate) -> Result<ProjectedPoint> {
        validate_coordinate(coordinate)?;
        if !(-80.0..=84.0).contains(&coordinate.lat) {
            bail!("global map lattice supports latitudes from 80°S through 84°N");
        }

        let eccentricity_squared = WGS84_FLATTENING * (2.0 - WGS84_FLATTENING);
        let second_eccentricity_squared = eccentricity_squared / (1.0 - eccentricity_squared);
        let latitude = coordinate.lat.to_radians();
        let longitude = coordinate.lon.to_radians();
        let central_meridian = ((f64::from(self.zone) - 1.0) * 6.0 - 180.0 + 3.0).to_radians();
        let sin_latitude = latitude.sin();
        let cos_latitude = latitude.cos();
        let tangent = latitude.tan();
        let radius = WGS84_SEMI_MAJOR_AXIS_METERS
            / (1.0 - eccentricity_squared * sin_latitude * sin_latitude).sqrt();
        let tangent_squared = tangent * tangent;
        let second_eccentricity_term = second_eccentricity_squared * cos_latitude * cos_latitude;
        let longitude_term = cos_latitude * (longitude - central_meridian);
        let eccentricity_fourth = eccentricity_squared * eccentricity_squared;
        let eccentricity_sixth = eccentricity_fourth * eccentricity_squared;
        let meridional_arc = WGS84_SEMI_MAJOR_AXIS_METERS
            * ((1.0
                - eccentricity_squared / 4.0
                - 3.0 * eccentricity_fourth / 64.0
                - 5.0 * eccentricity_sixth / 256.0)
                * latitude
                - (3.0 * eccentricity_squared / 8.0
                    + 3.0 * eccentricity_fourth / 32.0
                    + 45.0 * eccentricity_sixth / 1024.0)
                    * (2.0 * latitude).sin()
                + (15.0 * eccentricity_fourth / 256.0 + 45.0 * eccentricity_sixth / 1024.0)
                    * (4.0 * latitude).sin()
                - 35.0 * eccentricity_sixth / 3072.0 * (6.0 * latitude).sin());

        let east_meters = 500_000.0
            + TRANSVERSE_MERCATOR_SCALE
                * radius
                * (longitude_term
                    + (1.0 - tangent_squared + second_eccentricity_term) * longitude_term.powi(3)
                        / 6.0
                    + (5.0 - 18.0 * tangent_squared
                        + tangent_squared.powi(2)
                        + 72.0 * second_eccentricity_term
                        - 58.0 * second_eccentricity_squared)
                        * longitude_term.powi(5)
                        / 120.0);
        let north_meters = TRANSVERSE_MERCATOR_SCALE
            * (meridional_arc
                + radius
                    * tangent
                    * (longitude_term.powi(2) / 2.0
                        + (5.0 - tangent_squared
                            + 9.0 * second_eccentricity_term
                            + 4.0 * second_eccentricity_term.powi(2))
                            * longitude_term.powi(4)
                            / 24.0
                        + (61.0 - 58.0 * tangent_squared
                            + tangent_squared.powi(2)
                            + 600.0 * second_eccentricity_term
                            - 330.0 * second_eccentricity_squared)
                            * longitude_term.powi(6)
                            / 720.0));
        Ok(ProjectedPoint {
            east_meters,
            north_meters,
        })
    }
}

/// The globally anchored crop used by all semantic layers of one map.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WorldGrid {
    pub projection: WorldProjection,
    pub meters_per_cell: f64,
    pub west: i64,
    pub north: i64,
    pub width: u16,
    pub height: u16,
}

impl WorldGrid {
    pub fn around(center: Coordinate, side_miles: f64, width: u16, height: u16) -> Result<Self> {
        if width == 0 || height == 0 || width != height {
            bail!("globally aligned map windows must be non-empty and square");
        }
        if !side_miles.is_finite() || side_miles <= 0.0 {
            bail!("map side length must be positive and finite");
        }
        let projection = WorldProjection::around(center)?;
        let meters_per_cell = side_miles * METERS_PER_MILE / f64::from(width);
        let projected_center = projection.project(center)?;
        let center_x = (projected_center.east_meters / meters_per_cell).round() as i64;
        let center_y = (projected_center.north_meters / meters_per_cell).round() as i64;
        Ok(Self {
            projection,
            meters_per_cell,
            west: center_x - i64::from(width / 2),
            north: center_y + i64::from(height / 2),
            width,
            height,
        })
    }

    pub fn from_bounds(
        center: Coordinate,
        bounds: BoundingBox,
        width: u16,
        height: u16,
    ) -> Result<Self> {
        let north_south_meters = (bounds.north - bounds.south).abs() * 111_320.0;
        let east_west_meters =
            (bounds.east - bounds.west).abs() * 111_320.0 * center.lat.to_radians().cos().abs();
        let side_miles = (north_south_meters + east_west_meters) / (2.0 * METERS_PER_MILE);
        Self::around(center, side_miles, width, height)
    }

    pub fn project_cell(self, coordinate: Coordinate) -> Result<WorldCell> {
        let point = self.projection.project(coordinate)?;
        Ok(WorldCell {
            x: (point.east_meters / self.meters_per_cell).round() as i64,
            y: (point.north_meters / self.meters_per_cell).round() as i64,
        })
    }

    pub fn local_cell(self, world: WorldCell) -> Option<(u16, u16)> {
        let x = world.x - self.west;
        let y = self.north - world.y;
        (x >= 0 && y >= 0 && x < i64::from(self.width) && y < i64::from(self.height))
            .then_some((x as u16, y as u16))
    }

    pub fn world_cell(self, x: u16, y: u16) -> Option<WorldCell> {
        (x < self.width && y < self.height).then_some(WorldCell {
            x: self.west + i64::from(x),
            y: self.north - i64::from(y),
        })
    }

    pub fn contains(self, cell: WorldCell) -> bool {
        self.local_cell(cell).is_some()
    }

    /// Add deterministic context around a crop before morphology, spacing, or
    /// connectivity work. Generate on this halo grid, then crop back to the
    /// original window so decisions on shared borders see identical neighbors.
    pub fn expanded(self, halo: u16) -> Self {
        Self {
            west: self.west - i64::from(halo),
            north: self.north + i64::from(halo),
            width: self.width.saturating_add(halo.saturating_mul(2)),
            height: self.height.saturating_add(halo.saturating_mul(2)),
            ..self
        }
    }

    pub fn intersection(self, other: Self) -> Option<WorldRect> {
        if self.projection != other.projection
            || (self.meters_per_cell - other.meters_per_cell).abs() > 1e-9
        {
            return None;
        }
        let west = self.west.max(other.west);
        let east =
            (self.west + i64::from(self.width) - 1).min(other.west + i64::from(other.width) - 1);
        let north = self.north.min(other.north);
        let south = (self.north - i64::from(self.height) + 1)
            .max(other.north - i64::from(other.height) + 1);
        (west <= east && south <= north).then_some(WorldRect {
            west,
            east,
            south,
            north,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRect {
    pub west: i64,
    pub east: i64,
    pub south: i64,
    pub north: i64,
}

impl WorldRect {
    pub fn contains(self, cell: WorldCell) -> bool {
        (self.west..=self.east).contains(&cell.x) && (self.south..=self.north).contains(&cell.y)
    }
}

fn validate_coordinate(coordinate: Coordinate) -> Result<()> {
    if !coordinate.lat.is_finite()
        || !coordinate.lon.is_finite()
        || !(-90.0..=90.0).contains(&coordinate.lat)
        || !(-180.0..=180.0).contains(&coordinate.lon)
    {
        bail!("latitude/longitude are outside valid geographic bounds");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINNEAPOLIS: Coordinate = Coordinate {
        lat: 44.947_519_6,
        lon: -93.325_347_7,
    };

    #[test]
    fn a_half_mile_east_shift_moves_exactly_half_a_default_window() {
        let longitude_delta =
            (METERS_PER_MILE / 2.0) / (111_320.0 * MINNEAPOLIS.lat.to_radians().cos());
        let east = Coordinate {
            lon: MINNEAPOLIS.lon + longitude_delta,
            ..MINNEAPOLIS
        };
        let first = WorldGrid::around(MINNEAPOLIS, 1.0, 64, 64).expect("first grid");
        let second = WorldGrid::around(east, 1.0, 64, 64).expect("second grid");
        assert_eq!(second.west - first.west, 32);
        let overlap = first.intersection(second).expect("overlap");
        assert_eq!(overlap.east - overlap.west + 1, 32);
        assert_eq!(overlap.north - overlap.south + 1, 64);
    }

    #[test]
    fn half_mile_cardinal_shifts_share_chunk_aligned_borders() {
        let latitude_delta = (METERS_PER_MILE / 2.0) / 111_320.0;
        let longitude_delta =
            (METERS_PER_MILE / 2.0) / (111_320.0 * MINNEAPOLIS.lat.to_radians().cos());
        let center = WorldGrid::around(MINNEAPOLIS, 1.0, 64, 64).expect("center grid");
        for (coordinate, expected_x, expected_y) in [
            (
                Coordinate {
                    lon: MINNEAPOLIS.lon + longitude_delta,
                    ..MINNEAPOLIS
                },
                32,
                0,
            ),
            (
                Coordinate {
                    lon: MINNEAPOLIS.lon - longitude_delta,
                    ..MINNEAPOLIS
                },
                -32,
                0,
            ),
            (
                Coordinate {
                    lat: MINNEAPOLIS.lat + latitude_delta,
                    ..MINNEAPOLIS
                },
                0,
                32,
            ),
            (
                Coordinate {
                    lat: MINNEAPOLIS.lat - latitude_delta,
                    ..MINNEAPOLIS
                },
                0,
                -32,
            ),
        ] {
            let shifted = WorldGrid::around(coordinate, 1.0, 64, 64).expect("shifted grid");
            assert_eq!(shifted.west - center.west, expected_x);
            assert_eq!(shifted.north - center.north, expected_y);
            let overlap = center.intersection(shifted).expect("half-window overlap");
            assert_eq!(
                (overlap.east - overlap.west + 1) * (overlap.north - overlap.south + 1),
                64 * 32
            );
        }
    }

    #[test]
    fn local_addresses_round_trip_through_the_global_lattice() {
        let grid = WorldGrid::around(MINNEAPOLIS, 1.0, 64, 64).expect("grid");
        for (x, y) in [(0, 0), (31, 17), (32, 32), (63, 63)] {
            let world = grid.world_cell(x, y).expect("world cell");
            assert_eq!(grid.local_cell(world), Some((x, y)));
        }
    }

    #[test]
    fn global_hash_and_halo_do_not_depend_on_the_requested_window() {
        let grid = WorldGrid::around(MINNEAPOLIS, 1.0, 64, 64).expect("grid");
        let expanded = grid.expanded(12);
        let world = grid.world_cell(17, 29).expect("world cell");
        let expanded_local = expanded.local_cell(world).expect("inside halo");
        assert_eq!(expanded_local, (29, 41));
        assert_eq!(
            world.stable_hash(0x5452_4545),
            expanded
                .world_cell(expanded_local.0, expanded_local.1)
                .expect("round trip")
                .stable_hash(0x5452_4545)
        );
    }
}
