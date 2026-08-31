use anyhow::Result;

use crate::{GeneratedGrid, WorldCell, WorldGrid};

/// Deterministic procedural addressing for both square crops and H3 faces.
///
/// Square maps retain the transverse-Mercator world lattice needed for
/// overlap identity. H3 maps use the cell index as their global namespace and
/// local raster coordinates within that immutable face, avoiding UTM zones,
/// antimeridian discontinuities, and latitude limits entirely.
#[derive(Debug, Clone, Copy)]
pub(crate) enum StableGrid {
    World(WorldGrid),
    H3 {
        namespace: u64,
        width: u16,
        height: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StableCell {
    namespace: u64,
    x: i64,
    y: i64,
}

impl StableGrid {
    pub(crate) fn for_grid(grid: &GeneratedGrid) -> Result<Self> {
        if let Some(plan) = &grid.source.h3 {
            return Ok(Self::H3 {
                namespace: u64::from(plan.index()?),
                width: grid.width,
                height: grid.height,
            });
        }
        Ok(Self::World(WorldGrid::from_bounds(
            grid.source.center,
            grid.source.bounds,
            grid.width,
            grid.height,
        )?))
    }

    pub(crate) fn cell(self, x: u16, y: u16) -> Option<StableCell> {
        match self {
            Self::World(grid) => grid.world_cell(x, y).map(|cell| StableCell {
                namespace: 0,
                x: cell.x,
                y: cell.y,
            }),
            Self::H3 {
                namespace,
                width,
                height,
            } => (x < width && y < height).then_some(StableCell {
                namespace,
                x: i64::from(x),
                y: i64::from(y),
            }),
        }
    }

    pub(crate) fn local_cell(self, cell: StableCell) -> Option<(u16, u16)> {
        match self {
            Self::World(grid) if cell.namespace == 0 => grid.local_cell(WorldCell {
                x: cell.x,
                y: cell.y,
            }),
            Self::H3 {
                namespace,
                width,
                height,
            } if cell.namespace == namespace => (cell.x >= 0
                && cell.y >= 0
                && cell.x < i64::from(width)
                && cell.y < i64::from(height))
            .then_some((cell.x as u16, cell.y as u16)),
            _ => None,
        }
    }
}

impl StableCell {
    pub(crate) fn x_mod(self, modulus: i64) -> i64 {
        self.x.rem_euclid(modulus)
    }

    pub(crate) fn offset(self, dx: i64, dy: i64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub(crate) fn stable_hash(self, salt: u64) -> u64 {
        let mut value = (self.x as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (self.y as u64).rotate_left(29)
            ^ salt.wrapping_mul(0xbf58_476d_1ce4_e5b9);
        if self.namespace != 0 {
            value ^= self.namespace.wrapping_mul(0xd6e8_feb8_6659_fd93);
        }
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundingBox, Coordinate, MapSource, plan_h3_cell};

    #[test]
    fn h3_addressing_has_no_utm_or_polar_limit() {
        let plan = plan_h3_cell(
            Coordinate {
                lat: 89.9,
                lon: 179.9,
            },
            5,
        )
        .expect("polar H3 plan");
        let grid = GeneratedGrid {
            source: MapSource {
                center: plan.center,
                bounds: plan.fetch_bounds[0],
                attribution: "polar fixture".to_string(),
                features: Vec::new(),
                h3: Some(plan),
            },
            width: 64,
            height: 64,
            cells: vec![crate::MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        let addressing = StableGrid::for_grid(&grid).expect("H3 addressing");
        let anchor = addressing.cell(32, 32).expect("center address");
        assert_eq!(addressing.local_cell(anchor.offset(-7, 9)), Some((25, 41)));
        assert_eq!(anchor.stable_hash(17), anchor.stable_hash(17));
    }

    #[test]
    fn square_addressing_preserves_world_cell_hashes() {
        let grid = GeneratedGrid {
            source: MapSource {
                center: Coordinate {
                    lat: 44.95,
                    lon: -93.32,
                },
                bounds: BoundingBox {
                    south: 44.94,
                    west: -93.33,
                    north: 44.96,
                    east: -93.31,
                },
                attribution: "square fixture".to_string(),
                features: Vec::new(),
                h3: None,
            },
            width: 64,
            height: 64,
            cells: vec![crate::MapCell::Grass; 64 * 64],
            labels: Vec::new(),
        };
        let addressing = StableGrid::for_grid(&grid).expect("square addressing");
        let address = addressing.cell(12, 19).expect("address");
        let world = WorldGrid::from_bounds(
            grid.source.center,
            grid.source.bounds,
            grid.width,
            grid.height,
        )
        .expect("world grid")
        .world_cell(12, 19)
        .expect("world cell");
        assert_eq!(address.stable_hash(99), world.stable_hash(99));
    }
}
