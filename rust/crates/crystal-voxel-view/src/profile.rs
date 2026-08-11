//! Clean-room, presentation-only shape profiles for known Crystal artwork.
//!
//! Profiles are keyed only by stable visual source identity. They deliberately
//! do not inspect movement permissions or collision classes: unknown artwork
//! remains a flat tile and cannot turn into an invented wall.

use crystal_render_api::{VisualTileSource, VisualWorldFrame};

use crate::battle_tower::tree_group as battle_tower_tree_group;
use crate::casino::casino_shape;
use crate::cave::cave_shape;
use crate::cut_tree::cut_tree_shape;
use crate::flower::flower_shape;
use crate::forest::forest_ledge_shape;
use crate::grass::grass_shape;
use crate::interior::interior_fixture_shape;
use crate::kanto_cliff::kanto_cliff_shape;
use crate::kanto_post::kanto_post_shape;
use crate::modern_route::modern_route_shape;
use crate::park::park_shape;
use crate::port::port_shape;
use crate::sign::sign_shape;
use crate::tower::tower_shape;

pub const GROUND_HEIGHT: f32 = 0.0;
pub const WATER_HEIGHT: f32 = -2.0;
pub const COMPACT_BUILDING_HEIGHT: f32 = 16.0;
pub const LARGE_BUILDING_HEIGHT: f32 = 32.0;
pub const MOUNTAIN_LEDGE_HEIGHT: f32 = 16.0;
pub const MOUNTAIN_CLIFF_HEIGHT: f32 = 32.0;
// One overworld sprite (16 source pixels). Eight pixels left the raised side
// nearly coplanar after the 45-degree projection and read as a painted line.
pub const JUMP_LEDGE_HEIGHT: f32 = 16.0;
pub const MAX_PROFILE_HEIGHT: f32 = LARGE_BUILDING_HEIGHT;
pub const MIN_PROFILE_HEIGHT: f32 = WATER_HEIGHT;
pub const SOURCE_TILE_HEIGHT: f32 = 8.0;

const JOHTO_TILESET: &str = "johto";
const JOHTO_MODERN_TILESET: &str = "johto_modern";
const KANTO_TILESET: &str = "kanto";
const CAVE_TILESET: &str = "cave";
const LIGHTHOUSE_TILESET: &str = "lighthouse";
const FOREST_TILESET: &str = "forest";
const AUTHORED_WATER_TILESETS: [&str; 4] = ["johto", "johto_modern", "kanto", "forest"];
const AUTHORED_WATER_TILE_INDEX: u16 = 0x14;
const FOREST_TREE_TILE_INDICES: [u16; 4] = [0x26, 0x27, 0x36, 0x37];
const FOREST_GROUND_TILE_INDEX: u16 = 0x05;
const KANTO_TREE_TILE_INDICES: [u16; 8] = [0x2d, 0x2e, 0x3d, 0x3e, 0x40, 0x41, 0x50, 0x51];
#[cfg(test)]
const KANTO_SIGN_GROUND_TILE_INDEX: u16 = 0x39;
// Tree metatiles carry $2c in their open cells; using that exact background
// keeps the mask palette-aware and guarantees the sample travels with the art.
pub(crate) const KANTO_GROUND_TILE_INDEX: u16 = 0x2c;
const CAVE_GROUND_TILE_INDEX: u16 = 0x16;
const CAVE_ROCK_TOP_TILE_INDICES: [u16; 2] = [0x0c, 0x0d];
const CAVE_ROCK_BOTTOM_TILE_INDICES: [u16; 2] = [0x1c, 0x1d];
const LIGHTHOUSE_WALL_METATILES: [u16; 3] = [0x3c, 0x3d, 0x3e];
const LIGHTHOUSE_FLOOR_TILE_INDEX: u16 = 0x2e;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolidKind {
    Building,
    Tree,
    Rock,
    Ship,
    FlatCard,
    Prop,
    CutTree,
    Bank,
    Flower,
    Grass,
    Fence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgeFace {
    South,
    West,
    East,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CellShape {
    Flat,
    Water,
    /// Exact live source art kept parallel to the map at an authored datum.
    /// Used for floating top-down drawings that must not become pixel relief.
    PlaneAt {
        height: f32,
    },
    /// Animated cave waterfall art folded by the group mesher onto one
    /// vertical plane. Its vacated map footprint remains on the lower cave
    /// datum rather than using the outdoor shoreline recess.
    Waterfall,
    /// A live decorative tile lifted out of its walkable ground.
    Cutout {
        ground_tile_index: u16,
        solid: SolidKind,
    },
    /// Top-down prop artwork pushed upward as shallow pixel relief.
    Relief {
        height: f32,
        ground_tile_index: u16,
        base_height: f32,
    },
    /// Exact rocky shore artwork remains on the land cap and also supplies
    /// the cropped two-pixel land-to-water face.
    ShoreBand,
    RaisedTop {
        height: f32,
        solid: SolidKind,
    },
    /// A north-rising ground tile. Heights name the two z edges so adjacent
    /// cells form one continuous incline rather than a staircase.
    RampNorth {
        north_height: f32,
        south_height: f32,
    },
    /// One native-size source band folded onto a shared front plane.
    FacadeBand {
        plane_subtile_row: u8,
        band_from_top: u8,
        band_count: u8,
        ground_tile_index: u16,
        solid: SolidKind,
    },
    /// A source band folded onto a raised plateau edge. The claimed source
    /// cell keeps top-facing plateau art at `height`; the same native band is
    /// emitted upright at the shared south plane.
    LedgeBand {
        face: LedgeFace,
        plane_subtile: u8,
        band_from_top: u8,
        band_count: u8,
        top_tile_index: u16,
        height: f32,
    },
}

impl CellShape {
    pub fn surface_height(self, tile_height: f32) -> f32 {
        let source_height = match self {
            Self::Flat | Self::FacadeBand { .. } | Self::Cutout { .. } | Self::ShoreBand => {
                GROUND_HEIGHT
            }
            Self::Relief { base_height, .. } => base_height,
            Self::PlaneAt { height } => height,
            Self::Water => WATER_HEIGHT,
            Self::Waterfall => GROUND_HEIGHT,
            Self::RaisedTop { height, .. } | Self::LedgeBand { height, .. } => height,
            Self::RampNorth { north_height, .. } => north_height,
        };
        source_height * tile_height / SOURCE_TILE_HEIGHT
    }

    pub fn solid_kind(self) -> SolidKind {
        match self {
            Self::RaisedTop { solid, .. }
            | Self::FacadeBand { solid, .. }
            | Self::Cutout { solid, .. } => solid,
            Self::Relief { .. } | Self::PlaneAt { .. } => SolidKind::Prop,
            Self::ShoreBand => SolidKind::Bank,
            Self::LedgeBand { .. } => SolidKind::Bank,
            Self::Flat => SolidKind::Building,
            Self::Water | Self::Waterfall | Self::RampNorth { .. } => SolidKind::Bank,
        }
    }
}

/// Whether the frame can enter the optional renderer.
///
/// Every validated visual source has a safe flat baseline. Authored rules add
/// relief only for identities understood by this crate, so enabling 2.5D can
/// be inspected across the complete map catalog without inventing collision
/// geometry for unprofiled art.
pub fn supports_frame_profile(frame: &VisualWorldFrame) -> bool {
    !frame.map_id.is_empty() && !frame.tiles.is_empty()
}

fn jump_ledge_top_tile(source: &VisualTileSource) -> Option<u16> {
    match source.metatile_id {
        0x4b if source.subtile_column < 2 => Some(0x05),
        0x50 | 0x51 | 0x56 => Some(0x06),
        0x52 | 0x53 | 0x57 => Some(0x05),
        0x5a if matches!(source.subtile_column, 1 | 2) => Some(0x06),
        _ => None,
    }
}

pub fn shape_for_source(source: &VisualTileSource) -> CellShape {
    if let Some(shape) = sign_shape(source) {
        return shape;
    }
    if let Some(shape) = modern_route_shape(source) {
        return shape;
    }
    if let Some(shape) = port_shape(source) {
        return shape;
    }
    if let Some(shape) = park_shape(source) {
        return shape;
    }
    if let Some(shape) = kanto_cliff_shape(source) {
        return shape;
    }
    if let Some(shape) = kanto_post_shape(source) {
        return shape;
    }
    if matches!(source.tileset_id.as_ref(), "cave" | "dark_cave")
        && source.metatile_id == 0x2c
        && source.tile_index == 0x40
    {
        return CellShape::Waterfall;
    }
    if let Some(shape) = cave_shape(source) {
        return shape;
    }
    if let Some(shape) = forest_ledge_shape(source) {
        return shape;
    }
    if let Some(shape) = cut_tree_shape(source) {
        return shape;
    }
    if let Some(shape) = flower_shape(source) {
        return shape;
    }
    if let Some(shape) = casino_shape(source) {
        return shape;
    }
    if let Some(shape) = tower_shape(source) {
        return shape;
    }
    if let Some(shape) = interior_fixture_shape(source) {
        return shape;
    }

    if let Some(group) = battle_tower_tree_group(source) {
        return CellShape::FacadeBand {
            plane_subtile_row: source.subtile_row - group.local_row + group.height as u8,
            band_from_top: group.local_row,
            band_count: group.height as u8,
            ground_tile_index: group.ground_tile_index,
            solid: SolidKind::Tree,
        };
    }

    // Johto Modern $2f contains two independent 16x16 small-tree drawings
    // in its upper half ($1e/$1f over $3e/$3f); its lower half is plain $05
    // grass. These are especially visible along Azalea's north border. Keep
    // each complete drawing upright instead of leaving its crown face-up.
    if source.tileset_id.as_ref() == JOHTO_MODERN_TILESET
        && source.metatile_id == 0x2f
        && source.subtile_row < 2
    {
        return CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        };
    }

    if let Some(shape) = grass_shape(source) {
        return shape;
    }

    // Johto and Johto Modern share the exact same $0a 4x4 rock-platform
    // drawing. It belongs to the connected ledge volume, not the small
    // pixel-relief prop path: all sixteen source cells rise as one continuous
    // platform and the bank mesher supplies native edge courses.
    if matches!(
        source.tileset_id.as_ref(),
        JOHTO_TILESET | JOHTO_MODERN_TILESET
    ) && source.metatile_id == 0x0a
    {
        return CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        };
    }

    // Cave's $0c/$0d/$1c/$1d cells are one exact two-row rock drawing. The
    // metatile catalog places that same authored 16x16 prop at multiple
    // quadrants. Fold each source row once at its local two-row origin; never
    // promote the surrounding cave collision contour into a generic wall.
    if source.tileset_id.as_ref() == CAVE_TILESET {
        if CAVE_ROCK_TOP_TILE_INDICES.contains(&source.tile_index) {
            return CellShape::FacadeBand {
                plane_subtile_row: source.subtile_row,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Rock,
            };
        }
        if CAVE_ROCK_BOTTOM_TILE_INDICES.contains(&source.tile_index) {
            return CellShape::FacadeBand {
                plane_subtile_row: source.subtile_row.saturating_sub(1),
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Rock,
            };
        }
    }

    // The lighthouse perimeter blocks are complete four-row wall drawings in
    // Crystal's lighthouse metatile catalog. Fold those authored rows once
    // onto their south edge. Interior floor, stairs, furniture, and the 6F
    // chamber remain flat until they receive their own explicit profiles.
    if source.tileset_id.as_ref() == LIGHTHOUSE_TILESET
        && LIGHTHOUSE_WALL_METATILES.contains(&source.metatile_id)
    {
        return CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: LIGHTHOUSE_FLOOR_TILE_INDEX,
            solid: SolidKind::Bank,
        };
    }

    // Johto Modern's $05 is its repeated four-row tree drawing. Rail groups
    // use the same two-row folding mechanism but remain shallow props.
    if source.tileset_id.as_ref() == JOHTO_MODERN_TILESET {
        if source.metatile_id == 0x05 {
            return CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: source.subtile_row,
                band_count: 4,
                ground_tile_index: 0x06,
                solid: SolidKind::Tree,
            };
        }
    }

    // These three Crystal tilesets use tile $14 as their animated open-water
    // cell, including inside shoreline metatiles. Matching the source cell
    // rather than the whole metatile recesses water without lowering banks,
    // rocks, or other artwork in that same 4x4 block.
    if AUTHORED_WATER_TILESETS.contains(&source.tileset_id.as_ref())
        && source.tile_index == AUTHORED_WATER_TILE_INDEX
    {
        return CellShape::Water;
    }

    // Forest's scattered trees reuse one exact 2x2 drawing throughout the
    // metatile catalog. Its border block is a separate complete 4x4 canopy.
    // Both feed the shared grouped-tree mesher; collision never determines
    // their shape and all other forest artwork remains faithfully flat.
    if source.tileset_id.as_ref() == FOREST_TILESET {
        if source.metatile_id == 0x20 && source.subtile_column < 2 && source.subtile_row < 3 {
            return CellShape::FacadeBand {
                plane_subtile_row: 3,
                band_from_top: source.subtile_row,
                band_count: 3,
                ground_tile_index: FOREST_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            };
        }
        if source.metatile_id == 0x05 {
            return CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: source.subtile_row,
                band_count: 4,
                ground_tile_index: FOREST_GROUND_TILE_INDEX,
                solid: SolidKind::Tree,
            };
        }
        if FOREST_TREE_TILE_INDICES.contains(&source.tile_index) {
            let group_row = source.subtile_row / 2;
            return CellShape::FacadeBand {
                plane_subtile_row: (group_row + 1) * 2,
                band_from_top: source.subtile_row % 2,
                band_count: 2,
                ground_tile_index: FOREST_GROUND_TILE_INDEX,
                solid: SolidKind::Tree,
            };
        }
    }

    // Kanto's tree drawings are composed from stable 2x2 source-cell sets.
    // Fold each set once at its own south edge instead of treating the whole
    // metatile as a raised block or repeating a cell down a wall.
    if source.tileset_id.as_ref() == KANTO_TILESET
        && KANTO_TREE_TILE_INDICES.contains(&source.tile_index)
    {
        let group_row = source.subtile_row / 2;
        return CellShape::FacadeBand {
            plane_subtile_row: (group_row + 1) * 2,
            band_from_top: source.subtile_row % 2,
            band_count: 2,
            ground_tile_index: KANTO_GROUND_TILE_INDEX,
            solid: SolidKind::Tree,
        };
    }

    // Johto and Johto Modern share the byte-identical $58 buoy course in
    // water blocks $30-$39. It is top-down floating art: keep it parallel to
    // the map at the recessed water datum instead of creating a false bank.
    if matches!(
        source.tileset_id.as_ref(),
        JOHTO_TILESET | JOHTO_MODERN_TILESET
    ) && (0x30..=0x39).contains(&source.metatile_id)
        && source.tile_index == 0x58
    {
        return CellShape::PlaneAt {
            height: WATER_HEIGHT,
        };
    }

    if source.tileset_id.as_ref() != JOHTO_TILESET {
        return CellShape::Flat;
    }

    // Johto's complete mountain transition family shares the same universal
    // mountain datum as every other outdoor cliff. The profile determines
    // only which source cells are plateau and which fold into directional
    // face courses; region never changes the height.
    if (0x68..=0x73).contains(&source.metatile_id) {
        let face = match source.tile_index {
            0x3b => Some((
                LedgeFace::West,
                0,
                1_u8.saturating_sub(source.subtile_column),
            )),
            0x3d => Some((
                LedgeFace::East,
                4,
                source.subtile_column.saturating_sub(2).min(1),
            )),
            0x4b | 0x4c | 0x4d | 0x46 | 0x47 | 0x56 | 0x57 => Some((
                LedgeFace::South,
                4,
                source.subtile_row.saturating_sub(2).min(1),
            )),
            _ => None,
        };
        if let Some((face, plane_subtile, band_from_top)) = face {
            return CellShape::LedgeBand {
                face,
                plane_subtile,
                band_from_top,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            };
        }
        return CellShape::RaisedTop {
            height: MOUNTAIN_CLIFF_HEIGHT,
            solid: SolidKind::Bank,
        };
    }

    // Johto shoreline blocks mix animated $14 water with one-cell rocky cap
    // drawings. The cap stays top-facing and its outer rows also fold into
    // the shallow two-pixel drop, preserving the visible stone shoreline.
    if matches!(
        source.metatile_id,
        0x54 | 0x55 | 0x58 | 0x59 | 0x76 | 0x79 | 0x7a
    ) && source.tile_index != AUTHORED_WATER_TILE_INDEX
    {
        return CellShape::ShoreBand;
    }

    // Ecruteak's fence artwork has two distinct source roles. $5a/$59 are
    // the upper/lower courses of one horizontal rail and fold together at
    // the south edge. $4a is a single post repeated north-to-south, so each
    // cell becomes its own shallow standee instead of joining into a tower.
    if matches!(source.metatile_id, 0x41 | 0x42) && matches!(source.tile_index, 0x5a | 0x59) {
        return CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: 0x06,
            solid: SolidKind::Prop,
        };
    }
    if matches!(source.metatile_id, 0x42 | 0x44) && source.tile_index == 0x4a {
        return CellShape::FacadeBand {
            plane_subtile_row: source.subtile_row + 1,
            band_from_top: 0,
            band_count: 1,
            ground_tile_index: 0x06,
            solid: SolidKind::Prop,
        };
    }

    // Crystal's south-facing one-way jump ledges use one full sprite course
    // in the 2.5D presentation. The ground north of the authored
    // $4b/$4c/$4d lip rises; the lip's own row folds once onto the drop.
    // These rules are keyed to Crystal's exact metatile drawings, never to
    // collision permissions at runtime.
    let jump_ledge_top = jump_ledge_top_tile(source);
    if let Some(top_tile_index) = jump_ledge_top {
        if source.subtile_row < 3 {
            return CellShape::RaisedTop {
                height: JUMP_LEDGE_HEIGHT,
                solid: SolidKind::Bank,
            };
        }
        if matches!(source.tile_index, 0x4b | 0x4c | 0x4d) {
            return CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index,
                height: JUMP_LEDGE_HEIGHT,
            };
        }
    }

    match source.metatile_id {
        // Traditional Ecruteak roof blocks carry a separate row of 16x16
        // trees above the actual roof. Keep those cells in the tree object
        // pool; the building placement begins two source rows lower.
        0x2a | 0x2c | 0x2d if source.subtile_row < 2 => CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        // Blackthorn's repeated south mountain edge is a two-row plateau
        // over a two-row native cliff drawing. The drawing itself, not
        // collision, names the raised surface and its exact face bands.
        0x68 if source.subtile_column >= 2 => CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        },
        0x68 => CellShape::LedgeBand {
            face: LedgeFace::West,
            plane_subtile: 0,
            band_from_top: 1 - source.subtile_column,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x69 if source.subtile_column < 2 => CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        },
        0x69 => CellShape::LedgeBand {
            face: LedgeFace::East,
            plane_subtile: 4,
            band_from_top: source.subtile_column - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        // $6b/$6d draw the same L-shaped mountain volume with different
        // surface decoration. Their northwest quadrant is the plateau; the
        // right two columns are the native east face until the south course,
        // whose lower two rows fold across the complete front edge. Keeping
        // those sources on separate planes preserves the authored corner
        // instead of stretching one cliff tile around both sides.
        0x6b | 0x6d if source.subtile_row >= 2 => CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x6b | 0x6d if source.subtile_column >= 2 => CellShape::LedgeBand {
            face: LedgeFace::East,
            plane_subtile: 4,
            band_from_top: source.subtile_column - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x6b | 0x6d => CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        },
        // The remaining mountain transition drawings mix plateau surface,
        // rounded trim, and (for $6a/$6c) trees planted on that plateau.
        // Their rock mass still participates in the same connected bank run;
        // the tree drawings are claimed separately by the object mesher.
        0x6c if source.subtile_row >= 2 => CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x6c if source.subtile_column < 2 => CellShape::LedgeBand {
            face: LedgeFace::West,
            plane_subtile: 0,
            band_from_top: 1 - source.subtile_column,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x6e if source.subtile_row >= 2 && source.subtile_column < 2 => CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x6f if source.subtile_row >= 2 && source.subtile_column >= 2 => CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        0x6a | 0x6c | 0x6e | 0x6f => CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        },
        0x70 | 0x71 => CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        },
        0x72 | 0x73 if source.subtile_row < 2 => CellShape::RaisedTop {
            height: MOUNTAIN_LEDGE_HEIGHT,
            solid: SolidKind::Bank,
        },
        0x72 | 0x73 => CellShape::LedgeBand {
            face: LedgeFace::South,
            plane_subtile: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            top_tile_index: 0x3c,
            height: MOUNTAIN_LEDGE_HEIGHT,
        },
        // The compact house uses two source rows as a roof surface. Its two
        // lower rows become separate 8px facade bands on one shared seam.
        0x14 | 0x15 if source.subtile_row < 2 => CellShape::RaisedTop {
            height: COMPACT_BUILDING_HEIGHT,
            solid: SolidKind::Building,
        },
        0x14 | 0x15 => CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x06,
            solid: SolidKind::Building,
        },

        // Upper blocks in New Bark's large house and lab are top-facing roof
        // art. The matching lower block row is folded into four native bands.
        0x18 | 0x19 | 0x1f => CellShape::RaisedTop {
            height: LARGE_BUILDING_HEIGHT,
            solid: SolidKind::Building,
        },
        0x16 | 0x1c | 0x1e | 0x77 => CellShape::FacadeBand {
            plane_subtile_row: 0,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: 0x06,
            solid: SolidKind::Building,
        },

        // Connected tree artwork is upright art, never a face-up slab. Dense
        // 0x05 foliage is one four-band hedge card. The partial edge blocks
        // contain independent two-band groups and flat background halves.
        0x05 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row,
            band_count: 4,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        0x62 if source.subtile_column >= 2 && source.subtile_row < 2 => CellShape::FacadeBand {
            plane_subtile_row: 2,
            band_from_top: source.subtile_row,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        0x62 if source.subtile_column >= 2 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        // Ecruteak's isolated 16x16 trees use the same two-row source drawing
        // in the left half of $60. The object mesher claims each complete
        // pair and gives it rounded depth; this shape only identifies its
        // authored source cells and synthesized grass.
        0x60 if source.subtile_column < 2 => CellShape::FacadeBand {
            plane_subtile_row: ((source.subtile_row / 2) + 1) * 2,
            band_from_top: source.subtile_row % 2,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },
        0x65 if source.subtile_row >= 2 => CellShape::FacadeBand {
            plane_subtile_row: 4,
            band_from_top: source.subtile_row - 2,
            band_count: 2,
            ground_tile_index: 0x05,
            solid: SolidKind::Tree,
        },

        _ => CellShape::Flat,
    }
}

/// Resolve map-scoped art before the shared tileset profile. Some Crystal
/// interiors reuse an atlas for unrelated objects (notably Game Corners and
/// Vermilion Gym), so tileset identity alone is not sufficient evidence.
pub(crate) fn shape_for_source_on_map(map_id: &str, source: &VisualTileSource) -> CellShape {
    if let Some(shape) = crate::elite_four_room::shape(map_id, source) {
        return shape;
    }
    if let Some(shape) = crate::saffron_gym::shape(map_id, source) {
        return shape;
    }
    if let Some(shape) = crate::cerulean_gym::shape(map_id, source) {
        return shape;
    }
    if let Some(shape) = crate::hall_of_fame::shape(map_id, source) {
        return shape;
    }
    if let Some(shape) = crate::viridian_gym::shape(map_id, source) {
        return shape;
    }
    if let Some(shape) = crate::celadon_gym::shape(map_id, source) {
        return shape;
    }
    if let Some(shape) = crate::vermilion::shape(map_id, source) {
        return shape;
    }
    if crate::casino::is_game_corner_map(map_id)
        && let Some(shape) = casino_shape(source)
    {
        return shape;
    }
    shape_for_source_without_casino(source)
}

fn shape_for_source_without_casino(source: &VisualTileSource) -> CellShape {
    let mut source = source.clone();
    if source.tileset_id.as_ref() == "game_corner" {
        source.tileset_id = std::sync::Arc::from("unprofiled_game_corner");
    }
    shape_for_source(&source)
}

/// Presentation footing never puts actors on roofs, trees, sign cards, or the
/// recessed water surface. Gameplay remains two-dimensional and authoritative.
pub fn support_height(source: &VisualTileSource, tile_height: f32) -> f32 {
    if let Some(shape) = cave_shape(source) {
        if (shape.surface_height(SOURCE_TILE_HEIGHT) - crate::cave::CAVE_SHELF_HEIGHT).abs()
            < f32::EPSILON
        {
            return crate::cave::CAVE_SHELF_HEIGHT * tile_height / SOURCE_TILE_HEIGHT;
        }
    }
    let on_jump_ledge = source.tileset_id.as_ref() == JOHTO_TILESET
        && source.subtile_row < 3
        && match source.metatile_id {
            0x4b => source.subtile_column < 2,
            0x50..=0x53 | 0x56 | 0x57 => true,
            0x5a => matches!(source.subtile_column, 1 | 2),
            _ => false,
        };
    if on_jump_ledge {
        return JUMP_LEDGE_HEIGHT * tile_height / SOURCE_TILE_HEIGHT;
    }
    if source.tileset_id.as_ref() == JOHTO_TILESET
        && (0x68..=0x73).contains(&source.metatile_id)
        && matches!(shape_for_source(source), CellShape::RaisedTop { .. })
    {
        return MOUNTAIN_CLIFF_HEIGHT * tile_height / SOURCE_TILE_HEIGHT;
    }
    GROUND_HEIGHT
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bevy::prelude::{Handle, Image};
    use crystal_render_api::VisualTile;

    use super::*;

    fn source(metatile_id: u16, column: u8, row: u8) -> VisualTileSource {
        source_for_tileset(JOHTO_TILESET, metatile_id, column, row, 0)
    }

    fn source_for_tileset(
        tileset_id: &str,
        metatile_id: u16,
        column: u8,
        row: u8,
        tile_index: u16,
    ) -> VisualTileSource {
        VisualTileSource {
            tileset_id: Arc::from(tileset_id),
            metatile_id,
            subtile_column: column,
            subtile_row: row,
            tile_index,
        }
    }

    fn profile_frame(map_id: &str, tileset_id: &str) -> VisualWorldFrame {
        let mut tile_source = source(0x01, 0, 0);
        tile_source.tileset_id = Arc::from(tileset_id);
        VisualWorldFrame {
            map_id: Arc::from(map_id),
            tiles: vec![VisualTile {
                column: 0,
                row: 0,
                source: tile_source,
                texture: Handle::<Image>::weak_from_u128(1),
                priority: false,
            }],
            ..Default::default()
        }
    }

    #[test]
    fn every_named_map_has_a_safe_flat_profile_baseline() {
        assert!(supports_frame_profile(&profile_frame(
            "NewBarkTown",
            JOHTO_TILESET
        )));
        assert!(supports_frame_profile(&profile_frame(
            "CherrygroveCity",
            JOHTO_TILESET,
        )));
        assert!(supports_frame_profile(&profile_frame(
            "CeladonCity",
            "kanto",
        )));
        assert!(!supports_frame_profile(&profile_frame("", JOHTO_TILESET)));
    }

    #[test]
    fn compact_house_facade_rows_share_one_plane() {
        let upper = shape_for_source(&source(0x14, 0, 2));
        let lower = shape_for_source(&source(0x14, 0, 3));
        assert_eq!(
            upper,
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Building,
            }
        );
        assert_eq!(
            lower,
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Building,
            }
        );
    }

    #[test]
    fn unknown_art_is_flat_even_when_its_tile_index_looks_familiar() {
        let mut unknown = source(0xbeef, 0, 0);
        unknown.tile_index = 0x1e;
        assert_eq!(shape_for_source(&unknown), CellShape::Flat);
    }

    #[test]
    fn authored_water_identity_recesses_only_the_water_cell() {
        for tileset in AUTHORED_WATER_TILESETS {
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x43, 1, 2, 0x14)),
                CellShape::Water
            );
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x43, 1, 2, 0x15)),
                CellShape::Flat
            );
        }
        assert_eq!(
            shape_for_source(&source_for_tileset("house", 0x14, 1, 2, 0x14)),
            CellShape::Flat
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x54, 0, 0, 0x5c)),
            CellShape::ShoreBand,
            "shore art remains the rocky cap and also supplies the land lip"
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x54, 1, 1, 0x14)),
            CellShape::Water,
            "only the animated water cell is recessed"
        );
    }

    #[test]
    fn forest_shrine_is_one_three_band_masked_prop() {
        for (row, tile_index) in [(0, 0x10), (1, 0x30), (2, 0x15)] {
            assert_eq!(
                shape_for_source(&source_for_tileset("forest", 0x20, 0, row, tile_index)),
                CellShape::FacadeBand {
                    plane_subtile_row: 3,
                    band_from_top: row,
                    band_count: 3,
                    ground_tile_index: FOREST_GROUND_TILE_INDEX,
                    solid: SolidKind::Prop,
                }
            );
        }
        assert_eq!(
            shape_for_source(&source_for_tileset("forest", 0x20, 2, 0, 0x05)),
            CellShape::Flat,
            "the shrine's surrounding ground stays in the faithful plane"
        );
    }

    #[test]
    fn cave_waterfall_identity_is_not_generic_water_or_ground() {
        assert_eq!(
            shape_for_source(&source_for_tileset("cave", 0x2c, 1, 2, 0x40)),
            CellShape::Waterfall
        );
        assert_eq!(
            shape_for_source(&source_for_tileset("dark_cave", 0x2c, 3, 3, 0x40)),
            CellShape::Waterfall
        );
        assert_eq!(
            shape_for_source(&source_for_tileset("cave", 0x2d, 1, 2, 0x40)),
            CellShape::Flat,
            "shared art outside the authored waterfall block stays flat"
        );
    }

    #[test]
    fn cave_ladder_drawing_does_not_invent_a_staircase_support_height() {
        let up = source_for_tileset("cave", 0x14, 2, 2, 0x2a);
        let down = source_for_tileset("cave", 0x15, 2, 2, 0x22);
        assert_eq!(support_height(&up, 8.0), 0.0);
        assert_eq!(support_height(&down, 8.0), 0.0);
    }

    #[test]
    fn harbor_and_route_buoy_courses_float_flat_at_the_water_datum() {
        for tileset in [JOHTO_TILESET, JOHTO_MODERN_TILESET] {
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x34, 0, 0, 0x58)),
                CellShape::PlaneAt {
                    height: WATER_HEIGHT
                }
            );
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x34, 2, 0, 0x14)),
                CellShape::Water
            );
            assert_eq!(
                shape_for_source(&source_for_tileset(tileset, 0x01, 0, 0, 0x58)),
                CellShape::Flat,
                "the shared tile index is not globally reclassified"
            );
        }
    }

    #[test]
    fn kanto_tree_cells_form_independent_two_band_cards() {
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x32, 0, 0, 0x40)),
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
                solid: SolidKind::Tree,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x32, 0, 3, 0x51)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: KANTO_GROUND_TILE_INDEX,
                solid: SolidKind::Tree,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset("johto_modern", 0x32, 0, 0, 0x40)),
            CellShape::Flat
        );
    }

    #[test]
    fn kanto_post_metatiles_split_into_independent_two_band_objects() {
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x1b, 0, 0, 0x0e)),
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: 0x23,
                solid: SolidKind::Fence,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x1b, 0, 3, 0x55)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x23,
                solid: SolidKind::Fence,
            }
        );
    }

    #[test]
    fn kanto_sign_folds_two_authored_rows_over_its_exact_ground() {
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x08, 2, 2, 0x46)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: KANTO_SIGN_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(KANTO_TILESET, 0x08, 3, 3, 0x57)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: KANTO_SIGN_GROUND_TILE_INDEX,
                solid: SolidKind::Prop,
            }
        );
    }

    #[test]
    fn modern_city_trees_rails_and_signs_use_their_authored_depth_classes() {
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_MODERN_TILESET, 0x05, 2, 3, 0x3f,)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 3,
                band_count: 4,
                ground_tile_index: 0x06,
                solid: SolidKind::Tree,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_MODERN_TILESET, 0x40, 1, 0, 0x5a,)),
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Fence,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_MODERN_TILESET, 0x49, 2, 3, 0x59,)),
            CellShape::FacadeBand {
                plane_subtile_row: 4,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Fence,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_MODERN_TILESET, 0x45, 0, 1, 0x5e,)),
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Prop,
            }
        );
    }

    #[test]
    fn ecruteak_fence_rails_fold_and_posts_stand_independently() {
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x41, 2, 1, 0x59)),
            CellShape::FacadeBand {
                plane_subtile_row: 2,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: 0x06,
                solid: SolidKind::Prop,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x44, 0, 2, 0x4a)),
            CellShape::FacadeBand {
                plane_subtile_row: 3,
                band_from_top: 0,
                band_count: 1,
                ground_tile_index: 0x06,
                solid: SolidKind::Prop,
            }
        );
    }

    #[test]
    fn cave_rock_rows_fold_once_without_raising_support() {
        let top = source_for_tileset(CAVE_TILESET, 0x19, 0, 0, 0x0c);
        let bottom = source_for_tileset(CAVE_TILESET, 0x19, 0, 1, 0x1c);

        assert_eq!(
            shape_for_source(&top),
            CellShape::FacadeBand {
                plane_subtile_row: 0,
                band_from_top: 0,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Rock,
            }
        );
        assert_eq!(
            shape_for_source(&bottom),
            CellShape::FacadeBand {
                plane_subtile_row: 0,
                band_from_top: 1,
                band_count: 2,
                ground_tile_index: CAVE_GROUND_TILE_INDEX,
                solid: SolidKind::Rock,
            }
        );
        assert_eq!(support_height(&top, SOURCE_TILE_HEIGHT), GROUND_HEIGHT);
        assert_eq!(support_height(&bottom, SOURCE_TILE_HEIGHT), GROUND_HEIGHT);
    }
    #[test]
    fn lighthouse_perimeter_uses_native_wall_bands_only() {
        for metatile_id in LIGHTHOUSE_WALL_METATILES {
            for row in 0..4 {
                assert_eq!(
                    shape_for_source(&source_for_tileset(
                        LIGHTHOUSE_TILESET,
                        metatile_id,
                        1,
                        row,
                        0x5e,
                    )),
                    CellShape::FacadeBand {
                        plane_subtile_row: 4,
                        band_from_top: row,
                        band_count: 4,
                        ground_tile_index: LIGHTHOUSE_FLOOR_TILE_INDEX,
                        solid: SolidKind::Bank,
                    }
                );
            }
        }

        assert_eq!(
            shape_for_source(&source_for_tileset(
                LIGHTHOUSE_TILESET,
                0x27,
                0,
                0,
                LIGHTHOUSE_FLOOR_TILE_INDEX,
            )),
            CellShape::Flat
        );
    }

    #[test]
    fn building_and_tree_art_do_not_raise_actor_support() {
        assert_eq!(support_height(&source(0x18, 0, 0), 32.0), GROUND_HEIGHT);
        assert_eq!(support_height(&source(0x05, 0, 0), 32.0), GROUND_HEIGHT);
        assert_eq!(support_height(&source(0x54, 0, 0), 32.0), GROUND_HEIGHT);
    }

    #[test]
    fn blackthorn_south_ledge_separates_raised_top_from_native_face_bands() {
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x72, 1, 1, 0x3c)),
            CellShape::RaisedTop {
                height: MOUNTAIN_CLIFF_HEIGHT,
                solid: SolidKind::Bank,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x72, 1, 3, 0x4c)),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 1,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            }
        );
        assert_eq!(
            support_height(
                &source_for_tileset(JOHTO_TILESET, 0x72, 1, 1, 0x3c),
                SOURCE_TILE_HEIGHT,
            ),
            MOUNTAIN_CLIFF_HEIGHT
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x71, 2, 2, 0x3c)),
            CellShape::RaisedTop {
                height: MOUNTAIN_CLIFF_HEIGHT,
                solid: SolidKind::Bank,
            }
        );
        assert_eq!(
            support_height(
                &source_for_tileset(JOHTO_TILESET, 0x71, 2, 2, 0x3c),
                SOURCE_TILE_HEIGHT,
            ),
            MOUNTAIN_CLIFF_HEIGHT
        );
        assert_eq!(
            support_height(
                &source_for_tileset(JOHTO_TILESET, 0x72, 1, 3, 0x4c),
                SOURCE_TILE_HEIGHT,
            ),
            GROUND_HEIGHT
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x68, 1, 2, 0x3b)),
            CellShape::LedgeBand {
                face: LedgeFace::West,
                plane_subtile: 0,
                band_from_top: 0,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x69, 3, 2, 0x3d)),
            CellShape::LedgeBand {
                face: LedgeFace::East,
                plane_subtile: 4,
                band_from_top: 1,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            }
        );
    }

    #[test]
    fn blackthorn_corner_uses_two_authored_faces_around_one_raised_quadrant() {
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x6d, 1, 1, 0x3c)),
            CellShape::RaisedTop {
                height: MOUNTAIN_CLIFF_HEIGHT,
                solid: SolidKind::Bank,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x6d, 3, 1, 0x3d)),
            CellShape::LedgeBand {
                face: LedgeFace::East,
                plane_subtile: 4,
                band_from_top: 1,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x6d, 1, 3, 0x4c)),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 1,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            }
        );
        assert_eq!(
            support_height(&source_for_tileset(JOHTO_TILESET, 0x6d, 1, 1, 0x3c), 8.0),
            32.0
        );
        assert_eq!(
            support_height(&source_for_tileset(JOHTO_TILESET, 0x6d, 3, 1, 0x3d), 8.0),
            0.0
        );
        assert_eq!(
            support_height(&source_for_tileset(JOHTO_TILESET, 0x6d, 1, 3, 0x4c), 8.0),
            0.0
        );
    }

    #[test]
    fn blackthorn_cave_doorway_keeps_native_face_art() {
        assert!(matches!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x73, 2, 1, 0x3c)),
            CellShape::RaisedTop { .. }
        ));
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x73, 1, 3, 0x57)),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 1,
                band_count: 2,
                top_tile_index: 0x3c,
                height: MOUNTAIN_CLIFF_HEIGHT,
            }
        );
        assert_eq!(
            support_height(&source_for_tileset(JOHTO_TILESET, 0x73, 1, 3, 0x57), 8.0),
            0.0
        );
    }

    #[test]
    fn every_blackthorn_mountain_transition_joins_the_bank_volume() {
        for metatile_id in [0x6a, 0x6c, 0x6e, 0x6f] {
            let shape =
                shape_for_source(&source_for_tileset(JOHTO_TILESET, metatile_id, 0, 0, 0x3c));
            assert_eq!(shape.solid_kind(), SolidKind::Bank);
            assert_eq!(shape.surface_height(8.0), MOUNTAIN_CLIFF_HEIGHT);
        }
        assert!(matches!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x6e, 0, 3, 0x4c)),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                ..
            }
        ));
        assert!(matches!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x6f, 3, 3, 0x4c)),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                ..
            }
        ));
    }

    #[test]
    fn jump_ledge_has_raised_ground_and_a_shallow_native_lip() {
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x57, 1, 2, 0x05)),
            CellShape::RaisedTop {
                height: JUMP_LEDGE_HEIGHT,
                solid: SolidKind::Bank,
            }
        );
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x57, 1, 3, 0x4c)),
            CellShape::LedgeBand {
                face: LedgeFace::South,
                plane_subtile: 4,
                band_from_top: 0,
                band_count: 1,
                top_tile_index: 0x05,
                height: JUMP_LEDGE_HEIGHT,
            }
        );
        assert_eq!(support_height(&source(0x57, 1, 2), 8.0), JUMP_LEDGE_HEIGHT);
        assert_eq!(support_height(&source(0x57, 1, 3), 8.0), 0.0);
    }

    #[test]
    fn azalea_small_tree_crowns_are_upright_bands_over_their_grass_rows() {
        for row in 0..2 {
            for column in 0..4 {
                assert_eq!(
                    shape_for_source(&source_for_tileset(
                        JOHTO_MODERN_TILESET,
                        0x2f,
                        column,
                        row,
                        if column % 2 == 0 { 0x1e } else { 0x1f },
                    )),
                    CellShape::FacadeBand {
                        plane_subtile_row: 2,
                        band_from_top: row,
                        band_count: 2,
                        ground_tile_index: 0x05,
                        solid: SolidKind::Tree,
                    }
                );
            }
        }
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_MODERN_TILESET, 0x2f, 0, 2, 0x05,)),
            CellShape::Flat
        );
    }

    #[test]
    fn ecruteak_small_tree_sources_are_distinct_from_flat_ground() {
        assert!(matches!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x60, 0, 0, 0x1e)),
            CellShape::FacadeBand {
                solid: SolidKind::Tree,
                band_count: 2,
                ..
            }
        ));
        assert_eq!(
            shape_for_source(&source_for_tileset(JOHTO_TILESET, 0x60, 2, 0, 0x05)),
            CellShape::Flat
        );
    }
}
