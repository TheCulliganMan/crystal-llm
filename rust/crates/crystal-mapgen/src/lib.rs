mod audit;
mod biomes;
mod custom_tileset;
mod events;
mod geometry;
mod grid;
mod grid_seams;
mod h3;
mod modpack;
mod preview;
mod regional;
mod road_skeleton;
mod roadside;
mod stable_grid;
mod vegetation;
mod world_grid;

pub use audit::{MapAudit, audit_grid, audit_grid_with_facilities};
pub use custom_tileset::{
    GENERATED_BENCH_METATILE, GENERATED_CLIFF_STAIRS_METATILE, GENERATED_FOUNTAIN_METATILE,
    GENERATED_ICE_BOULDER_METATILE, GENERATED_ICE_FLOOR_METATILE,
    GENERATED_PARK_FENCE_EAST_METATILE, GENERATED_PARK_FENCE_NORTH_EAST_METATILE,
    GENERATED_PARK_FENCE_NORTH_METATILE, GENERATED_PARK_FENCE_NORTH_WEST_METATILE,
    GENERATED_PARK_FENCE_SOUTH_EAST_METATILE, GENERATED_PARK_FENCE_SOUTH_METATILE,
    GENERATED_PARK_FENCE_SOUTH_WEST_METATILE, GENERATED_PARK_FENCE_WEST_METATILE,
    GENERATED_PARK_FLOWER_BED_METATILE, GENERATED_PARK_LONG_GRASS_METATILE,
    GENERATED_PARK_TREE_METATILE, GENERATED_RED_HOUSE_NORTH_EAST_METATILE,
    GENERATED_RED_HOUSE_NORTH_WEST_METATILE, GENERATED_RED_HOUSE_SOUTH_EAST_METATILE,
    GENERATED_RED_HOUSE_SOUTH_WEST_METATILE, GENERATED_TILESET_ID,
    GENERATED_TRADITIONAL_HOUSE_NORTH_EAST_METATILE,
    GENERATED_TRADITIONAL_HOUSE_NORTH_WEST_METATILE,
    GENERATED_TRADITIONAL_HOUSE_SOUTH_EAST_METATILE,
    GENERATED_TRADITIONAL_HOUSE_SOUTH_WEST_METATILE, GENERATED_TRASH_CAN_METATILE,
    GENERATED_YELLOW_HOUSE_NORTH_EAST_METATILE, GENERATED_YELLOW_HOUSE_NORTH_WEST_METATILE,
    GENERATED_YELLOW_HOUSE_SOUTH_EAST_METATILE, GENERATED_YELLOW_HOUSE_SOUTH_WEST_METATILE,
    build_johto_modern_generated_tileset_extension,
};
pub use geometry::{
    BoundingBox, Coordinate, Feature, FeatureKind, MapSource, fetch_map_bounds, fetch_neighborhood,
};
pub use grid::{GeneratedGrid, GridLabel, MapCell, generate_grid, repair_walkable_connectivity};
pub use grid_seams::{H3BatchGridSeamFinalization, finalize_h3_batch_grid_seams};
pub use h3::{
    H3_SOURCE_SCHEMA_VERSION, H3BatchCell, H3BatchConnections, H3BatchLink, H3BatchManifest,
    H3BatchTopologyAudit, H3CellPlan, H3ClosedTransportCrossing, H3EdgeContract, H3EdgeTerrain,
    H3Facility, H3GridEdgeProfile, H3GridSeamAudit, H3GridSeamProfile, H3GridSeamSample,
    H3GridSeamSurface, H3GridTransportDirective, H3GridTransportDirectiveKind, H3Portal,
    H3RegionalCellPlan, H3RegionalConnection, H3SeamAudit, H3SeamContract, H3SourceProvenance,
    H3SourceStage, H3TransportCrossing, HexSide, attach_h3_regional_plan, audit_h3_batch_topology,
    audit_h3_grid_seams, audit_h3_seam_contracts, build_h3_grid_seam_profile,
    build_h3_seam_contract, fetch_h3_batch_neighborhoods, fetch_h3_neighborhood,
    finalize_h3_regional_transport_seams, finalize_h3_source_transport, plan_h3_batch,
    plan_h3_cell, prepare_h3_source, preserve_h3_authoritative_water_seams,
};
pub use modpack::{GeneratedModpack, ModpackOptions, build_modpack};
pub use preview::{render_h3_mosaic, render_tile_preview};
pub use regional::{
    H3RegionalAudit, H3RegionalGridReport, H3RegionalPlan, audit_h3_regional_batch,
    build_h3_regional_connections, inspect_h3_regional_grid, plan_h3_region,
};
pub use road_skeleton::{RoadAxis, RoadSkeleton, RoadSkeletonCell, build_road_skeleton};
pub use world_grid::{ProjectedPoint, WorldCell, WorldGrid, WorldProjection, WorldRect};
