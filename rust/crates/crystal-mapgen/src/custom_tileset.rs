use std::{collections::BTreeMap, io::Cursor};

use anyhow::{Context, Result, ensure};
use crystal_assets::{CompiledGamePack, TilesetDefinition, modpack::CompiledTilesetExtension};
use image::{DynamicImage, GrayImage, ImageFormat, Luma};

pub const GENERATED_TILESET_ID: &str = "johto_modern_generated";
pub const GENERATED_BENCH_METATILE: u8 = 0x80;
pub const GENERATED_TRASH_CAN_METATILE: u8 = 0x81;
pub const GENERATED_FOUNTAIN_METATILE: u8 = 0x82;
pub const GENERATED_PARK_TREE_METATILE: u8 = 0x83;
pub const GENERATED_PARK_FLOWER_BED_METATILE: u8 = 0x84;
pub const GENERATED_PARK_LONG_GRASS_METATILE: u8 = 0x85;
pub const GENERATED_PARK_FENCE_NORTH_WEST_METATILE: u8 = 0x86;
pub const GENERATED_PARK_FENCE_NORTH_METATILE: u8 = 0x87;
pub const GENERATED_PARK_FENCE_NORTH_EAST_METATILE: u8 = 0x88;
pub const GENERATED_PARK_FENCE_WEST_METATILE: u8 = 0x89;
pub const GENERATED_PARK_FENCE_EAST_METATILE: u8 = 0x8a;
pub const GENERATED_PARK_FENCE_SOUTH_WEST_METATILE: u8 = 0x8b;
pub const GENERATED_PARK_FENCE_SOUTH_METATILE: u8 = 0x8c;
pub const GENERATED_PARK_FENCE_SOUTH_EAST_METATILE: u8 = 0x8d;
pub const GENERATED_CLIFF_STAIRS_METATILE: u8 = 0x8e;
pub const GENERATED_RED_HOUSE_NORTH_WEST_METATILE: u8 = 0x8f;
pub const GENERATED_RED_HOUSE_NORTH_EAST_METATILE: u8 = 0x90;
pub const GENERATED_RED_HOUSE_SOUTH_WEST_METATILE: u8 = 0x91;
pub const GENERATED_RED_HOUSE_SOUTH_EAST_METATILE: u8 = 0x92;
pub const GENERATED_YELLOW_HOUSE_NORTH_WEST_METATILE: u8 = 0x93;
pub const GENERATED_YELLOW_HOUSE_NORTH_EAST_METATILE: u8 = 0x94;
pub const GENERATED_YELLOW_HOUSE_SOUTH_WEST_METATILE: u8 = 0x95;
pub const GENERATED_YELLOW_HOUSE_SOUTH_EAST_METATILE: u8 = 0x96;
pub const GENERATED_TRADITIONAL_HOUSE_NORTH_WEST_METATILE: u8 = 0x97;
pub const GENERATED_TRADITIONAL_HOUSE_NORTH_EAST_METATILE: u8 = 0x98;
pub const GENERATED_TRADITIONAL_HOUSE_SOUTH_WEST_METATILE: u8 = 0x99;
pub const GENERATED_TRADITIONAL_HOUSE_SOUTH_EAST_METATILE: u8 = 0x9a;
pub const GENERATED_ICE_FLOOR_METATILE: u8 = 0x9b;
pub const GENERATED_ICE_BOULDER_METATILE: u8 = 0x9c;

const SOURCE_TILESET_ID: &str = "johto_modern";
const TRADITIONAL_TILESET_ID: &str = "johto";
const PARK_TILESET_ID: &str = "park";
const LAB_TILESET_ID: &str = "lab";
const CAVE_TILESET_ID: &str = "cave";
const ICE_PATH_TILESET_ID: &str = "ice_path";
const TILE_BYTES: usize = 16;
const METATILE_BYTES: usize = 16;
const SOURCE_PHYSICAL_TILE_COUNT: usize = 192;
const GENERATED_PHYSICAL_TILE_COUNT: usize = 256;
const SOURCE_TILES_PER_VRAM_BANK: usize = SOURCE_PHYSICAL_TILE_COUNT / 2;
const GENERATED_TILES_PER_VRAM_BANK: usize = GENERATED_PHYSICAL_TILE_COUNT / 2;
const SHEET_TILES_WIDE: usize = 16;
const OUTDOOR_GROUND_TILE: u8 = 0x06;
const LAB_FLOOR_TILE: u8 = 0x10;

const PARK_BENCH_SOURCE_METATILE: u8 = 0x0e;
const PARK_BENCH_LAYOUT: [u8; METATILE_BYTES] = [
    0x07, 0x08, 0x09, 0x0a, 0x17, 0x18, 0x19, 0x1a, 0x27, 0x28, 0x29, 0x2a, 0x06, 0x00, 0x06, 0x00,
];
const PARK_FOUNTAIN_SOURCE_METATILE: u8 = 0x2f;
const PARK_FOUNTAIN_LAYOUT: [u8; METATILE_BYTES] = [
    0x00, 0x4c, 0x4d, 0x4e, 0x06, 0x5c, 0x5d, 0x5e, 0x00, 0x16, 0x00, 0x16, 0x06, 0x00, 0x06, 0x00,
];
const PARK_TREE_SOURCE_METATILE: u8 = 0x06;
const PARK_TREE_LAYOUT: [u8; METATILE_BYTES] = [
    0x0c, 0x0d, 0x0e, 0x0f, 0x1c, 0x1d, 0x1e, 0x1f, 0x2c, 0x2d, 0x2e, 0x2f, 0x3c, 0x3d, 0x3e, 0x3f,
];
const PARK_TREE_PALETTES: [u8; METATILE_BYTES] = [
    0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x02, 0x05, 0x05, 0x02, 0x02, 0x05, 0x05, 0x02,
];
const LAB_TRASH_SOURCE_METATILE: u8 = 0x07;
const LAB_TRASH_LAYOUT: [u8; METATILE_BYTES] = [
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x06, 0x07, 0x0e, 0x0f, 0x16, 0x17, 0x1e, 0x1f,
];

// Park $28 and $29 are byte-identical, so both source positions use target
// $a4. Every target below occupies a physical bank-1 slot unused by the
// canonical johto_modern metatiles; $aa-$ad stay reserved for later props.
const BENCH_SOURCE_TILES: [u8; 11] = [
    0x07, 0x08, 0x09, 0x0a, 0x17, 0x18, 0x19, 0x1a, 0x27, 0x28, 0x2a,
];
const BENCH_TARGET_TILES: [u8; 11] = [
    0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5,
];
const TRASH_TARGET_TILES: [u8; 4] = [0xa6, 0xa7, 0xa8, 0xa9];
const FOUNTAIN_SOURCE_TILES: [u8; 6] = [0x4c, 0x4d, 0x4e, 0x5c, 0x5d, 0x5e];
const FOUNTAIN_TARGET_TILES: [u8; 6] = [0xae, 0xaf, 0xb0, 0xb1, 0xb2, 0xb3];
const PARK_TREE_TARGET_TILES: [u8; METATILE_BYTES] = [
    0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3,
];
const PARK_DETAIL_SOURCE_TILES: [u8; 19] = [
    0x01, 0x03, 0x05, 0x1b, 0x23, 0x24, 0x25, 0x26, 0x2b, 0x33, 0x34, 0x35, 0x36, 0x3a, 0x3b, 0x43,
    0x4a, 0x4b, 0x53,
];
const PARK_DETAIL_TARGET_TILES: [u8; 19] = [
    0xaa, 0xab, 0xac, 0xad, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd, 0xce, 0xcf,
    0xd0, 0xd1, 0xd2,
];
const PARK_DETAIL_SOURCE_PALETTES: [u8; 19] = [
    0x02, 0x01, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x02, 0x02, 0x05,
    0x02, 0x02, 0x05,
];
const PARK_DETAIL_SOURCE_METATILES: [u8; 10] =
    [0x05, 0x13, 0x10, 0x11, 0x12, 0x14, 0x16, 0x18, 0x19, 0x1a];
const PARK_DETAIL_TARGET_METATILES: [u8; 10] = [
    GENERATED_PARK_FLOWER_BED_METATILE,
    GENERATED_PARK_LONG_GRASS_METATILE,
    GENERATED_PARK_FENCE_NORTH_WEST_METATILE,
    GENERATED_PARK_FENCE_NORTH_METATILE,
    GENERATED_PARK_FENCE_NORTH_EAST_METATILE,
    GENERATED_PARK_FENCE_WEST_METATILE,
    GENERATED_PARK_FENCE_EAST_METATILE,
    GENERATED_PARK_FENCE_SOUTH_WEST_METATILE,
    GENERATED_PARK_FENCE_SOUTH_METATILE,
    GENERATED_PARK_FENCE_SOUTH_EAST_METATILE,
];
const PARK_DETAIL_COLLISIONS: [[&str; 4]; 10] = [
    ["FLOOR", "FLOOR", "FLOOR", "FLOOR"],
    ["LONG_GRASS", "LONG_GRASS", "LONG_GRASS", "LONG_GRASS"],
    ["WALL", "WALL", "WALL", "FLOOR"],
    ["WALL", "WALL", "FLOOR", "FLOOR"],
    ["WALL", "WALL", "FLOOR", "WALL"],
    ["WALL", "FLOOR", "WALL", "FLOOR"],
    ["FLOOR", "WALL", "FLOOR", "WALL"],
    ["WALL", "FLOOR", "WALL", "WALL"],
    ["FLOOR", "FLOOR", "WALL", "WALL"],
    ["FLOOR", "WALL", "WALL", "WALL"],
];
// Cave $12's southeast quadrant is the two-course, walkable stair flight used
// on all three raised shelves in Slowpoke Well B2F. Cave $14 is the nearby
// dark ladder/warp drawing and must not be reused for an outdoor plateau.
const CAVE_STAIRS_SOURCE_METATILE: u8 = 0x12;
const CAVE_STAIRS_SOURCE_LAYOUT: [u8; METATILE_BYTES] = [
    0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x16, 0x26, 0x26, 0x36, 0x37, 0x26, 0x26, 0x36, 0x37,
];
const CAVE_STAIRS_SOURCE_TILES: [u8; 4] = [0x36, 0x37, 0x36, 0x37];
const CAVE_STAIRS_TARGET_TILES: [u8; 4] = [0xe0, 0xe1, 0xe2, 0xe3];
const CAVE_STAIRS_COLLISION: [&str; 4] = ["FLOOR", "FLOOR", "WALL", "FLOOR"];
const GENERATED_CLIFF_STAIRS_COLLISION: [&str; 4] = CAVE_STAIRS_COLLISION;
const JOHTO_CLIFF_SOUTH_SOURCE_METATILE: u8 = 0x72;
const JOHTO_CLIFF_SOUTH_LAYOUT: [u8; METATILE_BYTES] = [
    0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x4c, 0x4c, 0x4c, 0x4c, 0x4c, 0x4c, 0x4c, 0x4c,
];
const GENERATED_CLIFF_STAIRS_LAYOUT: [u8; METATILE_BYTES] = [
    0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x3c, 0x4c, 0x4c, 0xe0, 0xe1, 0x4c, 0x4c, 0xe2, 0xe3,
];
const HOUSE_SOURCE_METATILES: [u8; 4] = [0x18, 0x19, 0x1a, 0x1b];
const HOUSE_RECOLOR_SOURCE_TILES: [u8; 10] =
    [0x07, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12];
const RED_HOUSE_TARGET_TILES: [u8; 10] =
    [0xe4, 0xe5, 0xe6, 0xe7, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed];
const YELLOW_HOUSE_TARGET_TILES: [u8; 10] =
    [0xee, 0xef, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7];
const HOUSE_COLLISIONS: [[&str; 4]; 4] = [
    ["WALL", "WALL", "WALL", "WALL"],
    ["WALL", "WALL", "WALL", "WALL"],
    ["WALL", "WALL", "WALL", "DOOR"],
    ["WALL", "WALL", "WALL", "WALL"],
];
const GENERATED_RED_HOUSE_METATILES: [u8; 4] = [0x8f, 0x90, 0x91, 0x92];
const GENERATED_YELLOW_HOUSE_METATILES: [u8; 4] = [0x93, 0x94, 0x95, 0x96];
const TRADITIONAL_HOUSE_SOURCE_METATILES: [u8; 4] = [0x2c, 0x2d, 0x2e, 0x2f];
const TRADITIONAL_HOUSE_TARGET_METATILES: [u8; 4] = [0x97, 0x98, 0x99, 0x9a];
const TRADITIONAL_HOUSE_SOURCE_TILES: [u8; 10] =
    [0x53, 0x95, 0x27, 0x28, 0x97, 0x98, 0x29, 0x2a, 0x96, 0x99];
const TRADITIONAL_HOUSE_TARGET_TILES: [u8; 10] =
    [0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x8b, 0x8c, 0x8d];
const TRADITIONAL_HOUSE_COLLISIONS: [[&str; 4]; 4] = [
    ["HEADBUTT_TREE", "HEADBUTT_TREE", "WALL", "WALL"],
    ["HEADBUTT_TREE", "HEADBUTT_TREE", "WALL", "WALL"],
    ["WALL", "WALL", "WALL", "DOOR"],
    ["WALL", "WALL", "WALL", "WALL"],
];
const ICE_FLOOR_SOURCE_TILES: [u8; 4] = [0xc6, 0xc7, 0xd6, 0xd7];
const ICE_FLOOR_TARGET_TILES: [u8; 4] = [0xf8, 0xf9, 0xfa, 0xfb];
const ICE_BOULDER_SOURCE_TILES: [u8; 4] = [0x82, 0x83, 0x92, 0x93];
const ICE_BOULDER_TARGET_TILES: [u8; 4] = [0xfc, 0xfd, 0xfe, 0xff];
const ICE_FLOOR_LAYOUT: [u8; METATILE_BYTES] = [
    0xf8, 0xf9, 0xf8, 0xf9, 0xfa, 0xfb, 0xfa, 0xfb, 0xf8, 0xf9, 0xf8, 0xf9, 0xfa, 0xfb, 0xfa, 0xfb,
];
// Surface snowfields are encounter terrain, not indoor sliding puzzles. Their
// art stays canonical Ice Path blue while LONG_GRASS supplies the normal
// outdoor step-encounter permission.
const ICE_FLOOR_COLLISION: [&str; 4] = ["LONG_GRASS", "LONG_GRASS", "LONG_GRASS", "LONG_GRASS"];
const ICE_BOULDER_SOURCE_METATILE: u8 = 0x2c;
const ICE_BOULDER_SOURCE_COLLISION: [&str; 4] = ["WALL", "ICE", "ICE", "ICE"];
const ICE_BOULDER_SOURCE_LAYOUT: [u8; METATILE_BYTES] = [
    0x82, 0x83, 0xc6, 0xc7, 0x92, 0x93, 0xd6, 0xd7, 0xc6, 0xc7, 0xc6, 0xc7, 0xd6, 0xd7, 0xd6, 0xd7,
];
const ICE_BOULDER_LAYOUT: [u8; METATILE_BYTES] = [
    0xfc, 0xfd, 0xf8, 0xf9, 0xfe, 0xff, 0xfa, 0xfb, 0xf8, 0xf9, 0xf8, 0xf9, 0xfa, 0xfb, 0xfa, 0xfb,
];
const ICE_BOULDER_COLLISION: [&str; 4] = ["WALL", "LONG_GRASS", "LONG_GRASS", "LONG_GRASS"];
const GENERATED_TILE_TARGETS: [u8; 98] = [
    0x9b, 0x9c, 0x9d, 0x9e, 0x9f, 0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xae,
    0xaf, 0xb0, 0xb1, 0xb2, 0xb3, 0xb4, 0xb5, 0xb6, 0xb7, 0xb8, 0xb9, 0xba, 0xbb, 0xbc, 0xbd, 0xbe,
    0xbf, 0xc0, 0xc1, 0xc2, 0xc3, 0xaa, 0xab, 0xac, 0xad, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca,
    0xcb, 0xcc, 0xcd, 0xce, 0xcf, 0xd0, 0xd1, 0xd2, 0xe0, 0xe1, 0xe2, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
    0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xef, 0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
    0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x8b, 0x8c, 0x8d, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
    0xfe, 0xff,
];

const GENERATED_BENCH_LAYOUT: [u8; METATILE_BYTES] = [
    0x9b,
    0x9c,
    0x9d,
    0x9e,
    0x9f,
    0xa0,
    0xa1,
    0xa2,
    0xa3,
    0xa4,
    0xa4,
    0xa5,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
];
const GENERATED_TRASH_LAYOUT: [u8; METATILE_BYTES] = [
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    0xa6,
    0xa7,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    0xa8,
    0xa9,
];
const GENERATED_FOUNTAIN_LAYOUT: [u8; METATILE_BYTES] = [
    OUTDOOR_GROUND_TILE,
    0xae,
    0xaf,
    0xb0,
    OUTDOOR_GROUND_TILE,
    0xb1,
    0xb2,
    0xb3,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
    OUTDOOR_GROUND_TILE,
];
const GENERATED_PARK_TREE_LAYOUT: [u8; METATILE_BYTES] = PARK_TREE_TARGET_TILES;

const BENCH_COLLISION: [&str; 4] = ["WALL", "WALL", "FLOOR", "FLOOR"];
const SOURCE_TRASH_COLLISION: [&str; 4] = ["FLOOR", "FLOOR", "WALL", "WALL"];
const GENERATED_TRASH_COLLISION: [&str; 4] = ["FLOOR", "FLOOR", "FLOOR", "WALL"];
const FOUNTAIN_COLLISION: [&str; 4] = ["FLOOR", "WALL", "FLOOR", "FLOOR"];
const PARK_TREE_COLLISION: [&str; 4] = ["WALL", "WALL", "WALL", "WALL"];

/// Builds a complete custom tileset exclusively from canonical assets already
/// embedded in `base`. No repository path or generated placeholder art is
/// consulted at runtime.
pub fn build_johto_modern_generated_tileset_extension(
    base: &CompiledGamePack,
    manifest_id: impl Into<String>,
) -> Result<CompiledTilesetExtension> {
    build_extension_from_parts(
        &base.data().tilesets,
        base.runtime_files(),
        manifest_id.into(),
    )
}

fn build_extension_from_parts(
    definitions: &BTreeMap<String, TilesetDefinition>,
    files: &BTreeMap<String, Vec<u8>>,
    manifest_id: String,
) -> Result<CompiledTilesetExtension> {
    let source_definition = require_definition(definitions, SOURCE_TILESET_ID)?;
    let traditional_definition = require_definition(definitions, TRADITIONAL_TILESET_ID)?;
    let park_definition = require_definition(definitions, PARK_TILESET_ID)?;
    let lab_definition = require_definition(definitions, LAB_TILESET_ID)?;
    let cave_definition = require_definition(definitions, CAVE_TILESET_ID)?;
    let ice_path_definition = require_definition(definitions, ICE_PATH_TILESET_ID)?;
    let source_metatiles = require_file(
        files,
        &format!("data/tilesets/{SOURCE_TILESET_ID}_metatiles.bin"),
    )?;
    let traditional_metatiles = require_file(
        files,
        &format!("data/tilesets/{TRADITIONAL_TILESET_ID}_metatiles.bin"),
    )?;
    let park_metatiles = require_file(
        files,
        &format!("data/tilesets/{PARK_TILESET_ID}_metatiles.bin"),
    )?;
    let lab_metatiles = require_file(
        files,
        &format!("data/tilesets/{LAB_TILESET_ID}_metatiles.bin"),
    )?;
    let cave_metatiles = require_file(
        files,
        &format!("data/tilesets/{CAVE_TILESET_ID}_metatiles.bin"),
    )?;
    let ice_path_metatiles = require_file(
        files,
        &format!("data/tilesets/{ICE_PATH_TILESET_ID}_metatiles.bin"),
    )?;
    let source_2bpp = require_file(files, &format!("gfx/tilesets/{SOURCE_TILESET_ID}.2bpp"))?;
    let traditional_2bpp = require_file(
        files,
        &format!("gfx/tilesets/{TRADITIONAL_TILESET_ID}.2bpp"),
    )?;
    let park_2bpp = require_file(files, &format!("gfx/tilesets/{PARK_TILESET_ID}.2bpp"))?;
    let lab_2bpp = require_file(files, &format!("gfx/tilesets/{LAB_TILESET_ID}.2bpp"))?;
    let cave_2bpp = require_file(files, &format!("gfx/tilesets/{CAVE_TILESET_ID}.2bpp"))?;
    let ice_path_2bpp = require_file(files, &format!("gfx/tilesets/{ICE_PATH_TILESET_ID}.2bpp"))?;

    require_exact_asset_shape(
        SOURCE_TILESET_ID,
        source_metatiles,
        128,
        source_2bpp,
        SOURCE_PHYSICAL_TILE_COUNT,
    )?;
    require_exact_asset_shape(
        TRADITIONAL_TILESET_ID,
        traditional_metatiles,
        128,
        traditional_2bpp,
        SOURCE_PHYSICAL_TILE_COUNT,
    )?;
    require_exact_asset_shape(
        PARK_TILESET_ID,
        park_metatiles,
        64,
        park_2bpp,
        SOURCE_PHYSICAL_TILE_COUNT,
    )?;
    require_exact_asset_shape(
        LAB_TILESET_ID,
        lab_metatiles,
        64,
        lab_2bpp,
        SOURCE_PHYSICAL_TILE_COUNT,
    )?;
    require_exact_asset_shape(CAVE_TILESET_ID, cave_metatiles, 64, cave_2bpp, 96)?;
    require_exact_asset_shape(
        ICE_PATH_TILESET_ID,
        ice_path_metatiles,
        64,
        ice_path_2bpp,
        SOURCE_PHYSICAL_TILE_COUNT,
    )?;
    ensure!(
        metatile(park_metatiles, PARK_BENCH_SOURCE_METATILE)? == PARK_BENCH_LAYOUT,
        "canonical Park bench metatile $0e changed"
    );
    ensure!(
        metatile(park_metatiles, PARK_FOUNTAIN_SOURCE_METATILE)? == PARK_FOUNTAIN_LAYOUT,
        "canonical Park fountain metatile $2f changed"
    );
    ensure!(
        metatile(park_metatiles, PARK_TREE_SOURCE_METATILE)? == PARK_TREE_LAYOUT,
        "canonical Park large-tree metatile $06 changed"
    );
    ensure!(
        metatile(lab_metatiles, LAB_TRASH_SOURCE_METATILE)? == LAB_TRASH_LAYOUT,
        "canonical Lab trash-can metatile $07 changed"
    );
    ensure!(
        metatile(cave_metatiles, CAVE_STAIRS_SOURCE_METATILE)? == CAVE_STAIRS_SOURCE_LAYOUT,
        "canonical Cave stair metatile $14 changed"
    );
    ensure!(
        metatile(source_metatiles, JOHTO_CLIFF_SOUTH_SOURCE_METATILE)? == JOHTO_CLIFF_SOUTH_LAYOUT,
        "canonical johto_modern south-cliff metatile $72 changed"
    );
    ensure!(
        metatile(ice_path_metatiles, ICE_BOULDER_SOURCE_METATILE)? == ICE_BOULDER_SOURCE_LAYOUT,
        "canonical Ice Path ice-boulder metatile $2c changed"
    );
    require_collision(park_definition, PARK_BENCH_SOURCE_METATILE, BENCH_COLLISION)?;
    require_collision(
        park_definition,
        PARK_FOUNTAIN_SOURCE_METATILE,
        FOUNTAIN_COLLISION,
    )?;
    require_collision(
        park_definition,
        PARK_TREE_SOURCE_METATILE,
        PARK_TREE_COLLISION,
    )?;
    require_collision(
        lab_definition,
        LAB_TRASH_SOURCE_METATILE,
        SOURCE_TRASH_COLLISION,
    )?;
    require_collision(
        cave_definition,
        CAVE_STAIRS_SOURCE_METATILE,
        CAVE_STAIRS_COLLISION,
    )?;
    require_palette(park_definition, &BENCH_SOURCE_TILES, 0x00)?;
    require_palette(park_definition, &FOUNTAIN_SOURCE_TILES, 0x00)?;
    require_palettes(park_definition, &PARK_TREE_LAYOUT, &PARK_TREE_PALETTES)?;
    require_palette(lab_definition, &[0x0e, 0x0f, 0x1e, 0x1f], 0x00)?;
    require_palette(source_definition, &[OUTDOOR_GROUND_TILE], 0x00)?;
    require_palette(cave_definition, &CAVE_STAIRS_SOURCE_TILES, 0x00)?;
    require_palette(source_definition, &HOUSE_RECOLOR_SOURCE_TILES, 0x06)?;
    require_palette(ice_path_definition, &ICE_FLOOR_SOURCE_TILES, 0x0a)?;
    require_palette(ice_path_definition, &ICE_BOULDER_SOURCE_TILES, 0x0e)?;
    require_collision(
        ice_path_definition,
        ICE_BOULDER_SOURCE_METATILE,
        ICE_BOULDER_SOURCE_COLLISION,
    )?;
    for (source, collision) in HOUSE_SOURCE_METATILES.into_iter().zip(HOUSE_COLLISIONS) {
        require_collision(source_definition, source, collision)?;
    }
    for (source, collision) in TRADITIONAL_HOUSE_SOURCE_METATILES
        .into_iter()
        .zip(TRADITIONAL_HOUSE_COLLISIONS)
    {
        require_collision(traditional_definition, source, collision)?;
    }
    require_palettes(
        park_definition,
        &PARK_DETAIL_SOURCE_TILES,
        &PARK_DETAIL_SOURCE_PALETTES,
    )?;
    for (source, collision) in PARK_DETAIL_SOURCE_METATILES
        .into_iter()
        .zip(PARK_DETAIL_COLLISIONS)
    {
        require_collision(park_definition, source, collision)?;
    }
    require_unused_physical_targets(source_metatiles, source_definition)?;

    let mut definition = source_definition.clone();
    definition.palette_map.resize(256, 0);
    for target in GENERATED_TILE_TARGETS[..21].iter().copied() {
        *definition
            .palette_map
            .get_mut(usize::from(target))
            .with_context(|| format!("johto_modern palette map has no tile {target:#04x}"))? = 0x08;
    }
    for ((source, target), palette) in PARK_TREE_LAYOUT
        .into_iter()
        .zip(PARK_TREE_TARGET_TILES)
        .zip(PARK_TREE_PALETTES)
    {
        debug_assert_eq!(park_definition.palette_map[usize::from(source)], palette);
        *definition
            .palette_map
            .get_mut(usize::from(target))
            .with_context(|| format!("johto_modern palette map has no tile {target:#04x}"))? =
            palette | 0x08;
    }
    for ((source, target), palette) in PARK_DETAIL_SOURCE_TILES
        .into_iter()
        .zip(PARK_DETAIL_TARGET_TILES)
        .zip(PARK_DETAIL_SOURCE_PALETTES)
    {
        debug_assert_eq!(park_definition.palette_map[usize::from(source)], palette);
        *definition
            .palette_map
            .get_mut(usize::from(target))
            .with_context(|| format!("johto_modern palette map has no tile {target:#04x}"))? =
            palette | 0x08;
    }
    for target in CAVE_STAIRS_TARGET_TILES {
        *definition
            .palette_map
            .get_mut(usize::from(target))
            .with_context(|| format!("johto_modern palette map has no tile {target:#04x}"))? = 0x0d;
    }
    for target in RED_HOUSE_TARGET_TILES {
        definition.palette_map[usize::from(target)] = 0x09;
    }
    for target in YELLOW_HOUSE_TARGET_TILES {
        definition.palette_map[usize::from(target)] = 0x0c;
    }
    for target in TRADITIONAL_HOUSE_TARGET_TILES {
        definition.palette_map[usize::from(target)] = 0x0d;
    }
    for target in ICE_FLOOR_TARGET_TILES {
        // Use Ice Path's blue object palette for the entire floor. The source
        // palette is green-white outdoors and made the biome disappear into
        // lawn at overview scale.
        definition.palette_map[usize::from(target)] = 0x0e;
    }
    for target in ICE_BOULDER_TARGET_TILES {
        definition.palette_map[usize::from(target)] = 0x0e;
    }
    definition.collision.insert(
        format!("{GENERATED_BENCH_METATILE:02x}"),
        BENCH_COLLISION.map(str::to_string).to_vec(),
    );
    for (target, collision) in GENERATED_RED_HOUSE_METATILES
        .into_iter()
        .chain(GENERATED_YELLOW_HOUSE_METATILES)
        .zip(HOUSE_COLLISIONS.into_iter().cycle())
    {
        definition.collision.insert(
            format!("{target:02x}"),
            collision.map(str::to_string).to_vec(),
        );
    }
    for (target, collision) in TRADITIONAL_HOUSE_TARGET_METATILES
        .into_iter()
        .zip(TRADITIONAL_HOUSE_COLLISIONS)
    {
        definition.collision.insert(
            format!("{target:02x}"),
            collision.map(str::to_string).to_vec(),
        );
    }
    definition.collision.insert(
        format!("{GENERATED_TRASH_CAN_METATILE:02x}"),
        GENERATED_TRASH_COLLISION.map(str::to_string).to_vec(),
    );
    definition.collision.insert(
        format!("{GENERATED_FOUNTAIN_METATILE:02x}"),
        FOUNTAIN_COLLISION.map(str::to_string).to_vec(),
    );
    definition.collision.insert(
        format!("{GENERATED_PARK_TREE_METATILE:02x}"),
        PARK_TREE_COLLISION.map(str::to_string).to_vec(),
    );
    for (target, collision) in PARK_DETAIL_TARGET_METATILES
        .into_iter()
        .zip(PARK_DETAIL_COLLISIONS)
    {
        definition.collision.insert(
            format!("{target:02x}"),
            collision.map(str::to_string).to_vec(),
        );
    }
    definition.collision.insert(
        format!("{GENERATED_CLIFF_STAIRS_METATILE:02x}"),
        GENERATED_CLIFF_STAIRS_COLLISION
            .map(str::to_string)
            .to_vec(),
    );
    definition.collision.insert(
        format!("{GENERATED_ICE_FLOOR_METATILE:02x}"),
        ICE_FLOOR_COLLISION.map(str::to_string).to_vec(),
    );
    definition.collision.insert(
        format!("{GENERATED_ICE_BOULDER_METATILE:02x}"),
        ICE_BOULDER_COLLISION.map(str::to_string).to_vec(),
    );

    let mut metatiles = source_metatiles.to_vec();
    metatiles.extend_from_slice(&GENERATED_BENCH_LAYOUT);
    metatiles.extend_from_slice(&GENERATED_TRASH_LAYOUT);
    metatiles.extend_from_slice(&GENERATED_FOUNTAIN_LAYOUT);
    metatiles.extend_from_slice(&GENERATED_PARK_TREE_LAYOUT);
    for source in PARK_DETAIL_SOURCE_METATILES {
        let source_layout = metatile(park_metatiles, source)?;
        metatiles.extend_from_slice(&remap_park_detail_layout(source_layout)?);
    }
    metatiles.extend_from_slice(&GENERATED_CLIFF_STAIRS_LAYOUT);
    for targets in [RED_HOUSE_TARGET_TILES, YELLOW_HOUSE_TARGET_TILES] {
        for source in HOUSE_SOURCE_METATILES {
            metatiles.extend_from_slice(&remap_house_layout(
                metatile(source_metatiles, source)?,
                targets,
            )?);
        }
    }
    for source in TRADITIONAL_HOUSE_SOURCE_METATILES {
        metatiles.extend_from_slice(&remap_traditional_house_layout(metatile(
            traditional_metatiles,
            source,
        )?));
    }
    metatiles.extend_from_slice(&ICE_FLOOR_LAYOUT);
    metatiles.extend_from_slice(&ICE_BOULDER_LAYOUT);

    let mut tile_graphics_2bpp = vec![0; GENERATED_PHYSICAL_TILE_COUNT * TILE_BYTES];
    tile_graphics_2bpp[..SOURCE_TILES_PER_VRAM_BANK * TILE_BYTES]
        .copy_from_slice(&source_2bpp[..SOURCE_TILES_PER_VRAM_BANK * TILE_BYTES]);
    tile_graphics_2bpp[GENERATED_TILES_PER_VRAM_BANK * TILE_BYTES
        ..(GENERATED_TILES_PER_VRAM_BANK + SOURCE_TILES_PER_VRAM_BANK) * TILE_BYTES]
        .copy_from_slice(&source_2bpp[SOURCE_TILES_PER_VRAM_BANK * TILE_BYTES..]);
    for (source, target) in BENCH_SOURCE_TILES.into_iter().zip(BENCH_TARGET_TILES) {
        let source_art = tile_2bpp(park_2bpp, usize::from(source))?;
        write_target_tile(&mut tile_graphics_2bpp, target, source_art)?;
    }
    let trash_art = outdoor_trash_art(source_2bpp, lab_2bpp)?;
    for (art, target) in trash_art.iter().zip(TRASH_TARGET_TILES) {
        write_target_tile(&mut tile_graphics_2bpp, target, art)?;
    }
    for (source, target) in FOUNTAIN_SOURCE_TILES.into_iter().zip(FOUNTAIN_TARGET_TILES) {
        let source_art = tile_2bpp(park_2bpp, usize::from(source))?;
        write_target_tile(&mut tile_graphics_2bpp, target, source_art)?;
    }
    for (source, target) in PARK_TREE_LAYOUT.into_iter().zip(PARK_TREE_TARGET_TILES) {
        let source_art = tile_2bpp(park_2bpp, usize::from(source))?;
        write_target_tile(&mut tile_graphics_2bpp, target, source_art)?;
    }
    for (source, target) in PARK_DETAIL_SOURCE_TILES
        .into_iter()
        .zip(PARK_DETAIL_TARGET_TILES)
    {
        let source_art = tile_2bpp(park_2bpp, usize::from(source))?;
        write_target_tile(&mut tile_graphics_2bpp, target, source_art)?;
    }
    for (source, target) in CAVE_STAIRS_SOURCE_TILES
        .into_iter()
        .zip(CAVE_STAIRS_TARGET_TILES)
    {
        let source_art = tile_2bpp(cave_2bpp, usize::from(source))?;
        write_target_tile(&mut tile_graphics_2bpp, target, source_art)?;
    }
    for targets in [RED_HOUSE_TARGET_TILES, YELLOW_HOUSE_TARGET_TILES] {
        for (source, target) in HOUSE_RECOLOR_SOURCE_TILES.into_iter().zip(targets) {
            let source_art = tile_2bpp(source_2bpp, usize::from(source))?;
            write_target_tile(&mut tile_graphics_2bpp, target, source_art)?;
        }
    }
    for (source, target) in TRADITIONAL_HOUSE_SOURCE_TILES
        .into_iter()
        .zip(TRADITIONAL_HOUSE_TARGET_TILES)
    {
        let source_palette = traditional_definition.palette_map[usize::from(source)];
        let physical = usize::from((source_palette >> 3) & 1) * SOURCE_TILES_PER_VRAM_BANK
            + usize::from(source & 0x7f);
        write_target_tile(
            &mut tile_graphics_2bpp,
            target,
            tile_2bpp(traditional_2bpp, physical)?,
        )?;
    }
    for (source, target) in ICE_FLOOR_SOURCE_TILES
        .into_iter()
        .zip(ICE_FLOOR_TARGET_TILES)
    {
        let palette = ice_path_definition.palette_map[usize::from(source)];
        let physical = usize::from((palette >> 3) & 1) * SOURCE_TILES_PER_VRAM_BANK
            + usize::from(source & 0x7f);
        write_target_tile(
            &mut tile_graphics_2bpp,
            target,
            tile_2bpp(ice_path_2bpp, physical)?,
        )?;
    }
    for (source, target) in ICE_BOULDER_SOURCE_TILES
        .into_iter()
        .zip(ICE_BOULDER_TARGET_TILES)
    {
        let palette = ice_path_definition.palette_map[usize::from(source)];
        let physical = usize::from((palette >> 3) & 1) * SOURCE_TILES_PER_VRAM_BANK
            + usize::from(source & 0x7f);
        write_target_tile(
            &mut tile_graphics_2bpp,
            target,
            tile_2bpp(ice_path_2bpp, physical)?,
        )?;
    }
    let tile_graphics_png = encode_2bpp_sheet_png(&tile_graphics_2bpp)?;

    Ok(CompiledTilesetExtension {
        manifest_id,
        tileset_id: GENERATED_TILESET_ID.to_string(),
        behavior_source_tileset_id: Some(SOURCE_TILESET_ID.to_string()),
        definition,
        metatiles,
        tile_graphics_2bpp,
        tile_graphics_png,
    })
}

fn remap_park_detail_layout(source: [u8; METATILE_BYTES]) -> Result<[u8; METATILE_BYTES]> {
    let mut generated = [0_u8; METATILE_BYTES];
    for (index, source_tile) in source.into_iter().enumerate() {
        let mapped = PARK_DETAIL_SOURCE_TILES
            .iter()
            .position(|candidate| *candidate == source_tile)
            .with_context(|| {
                format!("Park detail metatile references unimported source tile {source_tile:#04x}")
            })?;
        generated[index] = PARK_DETAIL_TARGET_TILES[mapped];
    }
    Ok(generated)
}

fn remap_house_layout(
    source: [u8; METATILE_BYTES],
    targets: [u8; HOUSE_RECOLOR_SOURCE_TILES.len()],
) -> Result<[u8; METATILE_BYTES]> {
    let mut generated = source;
    for tile in &mut generated {
        if let Some(index) = HOUSE_RECOLOR_SOURCE_TILES
            .iter()
            .position(|candidate| candidate == tile)
        {
            *tile = targets[index];
        }
    }
    Ok(generated)
}

fn remap_traditional_house_layout(source: [u8; METATILE_BYTES]) -> [u8; METATILE_BYTES] {
    source.map(|tile| {
        TRADITIONAL_HOUSE_SOURCE_TILES
            .iter()
            .position(|candidate| *candidate == tile)
            .map(|index| TRADITIONAL_HOUSE_TARGET_TILES[index])
            .unwrap_or(tile)
    })
}

fn require_definition<'a>(
    definitions: &'a BTreeMap<String, TilesetDefinition>,
    id: &str,
) -> Result<&'a TilesetDefinition> {
    definitions
        .get(id)
        .with_context(|| format!("base pack is missing canonical tileset '{id}'"))
}

fn require_file<'a>(files: &'a BTreeMap<String, Vec<u8>>, path: &str) -> Result<&'a [u8]> {
    files
        .get(path)
        .map(Vec::as_slice)
        .with_context(|| format!("base pack is missing canonical runtime asset {path}"))
}

fn require_exact_asset_shape(
    id: &str,
    metatiles: &[u8],
    metatile_count: usize,
    graphics: &[u8],
    physical_tile_count: usize,
) -> Result<()> {
    ensure!(
        metatiles.len() == metatile_count * METATILE_BYTES,
        "canonical {id} metatile data has {} bytes instead of {}",
        metatiles.len(),
        metatile_count * METATILE_BYTES
    );
    ensure!(
        graphics.len() == physical_tile_count * TILE_BYTES,
        "canonical {id} graphics have {} bytes instead of {}",
        graphics.len(),
        physical_tile_count * TILE_BYTES
    );
    Ok(())
}

fn metatile(bytes: &[u8], id: u8) -> Result<[u8; METATILE_BYTES]> {
    let offset = usize::from(id) * METATILE_BYTES;
    bytes
        .get(offset..offset + METATILE_BYTES)
        .context("canonical metatile id is outside its layout")?
        .try_into()
        .context("canonical metatile has the wrong byte length")
}

fn require_collision(definition: &TilesetDefinition, id: u8, expected: [&str; 4]) -> Result<()> {
    let key = format!("{id:02x}");
    let actual = definition
        .collision
        .get(&key)
        .with_context(|| format!("canonical tileset is missing collision metatile {key}"))?;
    ensure!(
        actual.iter().map(String::as_str).eq(expected),
        "canonical collision metatile {key} changed: {actual:?}"
    );
    Ok(())
}

fn require_palette(definition: &TilesetDefinition, ids: &[u8], expected: u8) -> Result<()> {
    for &id in ids {
        let actual = definition
            .palette_map
            .get(usize::from(id))
            .copied()
            .with_context(|| format!("canonical palette map is missing tile {id:#04x}"))?;
        ensure!(
            actual == expected,
            "canonical tile {id:#04x} palette/bank changed from {expected:#04x} to {actual:#04x}"
        );
    }
    Ok(())
}

fn require_palettes(definition: &TilesetDefinition, ids: &[u8], expected: &[u8]) -> Result<()> {
    ensure!(
        ids.len() == expected.len(),
        "palette validation lists have different lengths"
    );
    for (&id, &expected) in ids.iter().zip(expected) {
        require_palette(definition, &[id], expected)?;
    }
    Ok(())
}

fn require_unused_physical_targets(metatiles: &[u8], definition: &TilesetDefinition) -> Result<()> {
    let target_slots = GENERATED_TILE_TARGETS.map(|tile| (1_usize, usize::from(tile & 0x7f)));
    for &tile in metatiles {
        let palette = definition
            .palette_map
            .get(usize::from(tile))
            .copied()
            .with_context(|| format!("johto_modern tile {tile:#04x} has no palette entry"))?;
        ensure!(
            palette <= 0x0f,
            "johto_modern referenced tile {tile:#04x} has invalid palette/bank value {palette:#04x}"
        );
        let bank = usize::from((palette >> 3) & 1);
        let tile_in_bank = usize::from(tile & 0x7f);
        ensure!(
            tile_in_bank < SOURCE_TILES_PER_VRAM_BANK,
            "johto_modern tile {tile:#04x} is outside packed VRAM bank {bank}"
        );
        ensure!(
            !target_slots.contains(&(bank, tile_in_bank)),
            "johto_modern generated art target VRAM bank {bank} tile {tile_in_bank:#04x} is already used"
        );
    }
    Ok(())
}

fn tile_2bpp(bytes: &[u8], physical_tile: usize) -> Result<&[u8]> {
    let offset = physical_tile * TILE_BYTES;
    bytes
        .get(offset..offset + TILE_BYTES)
        .with_context(|| format!("physical tile {physical_tile:#04x} is outside 2bpp graphics"))
}

fn write_target_tile(graphics: &mut [u8], logical_tile: u8, art: &[u8]) -> Result<()> {
    ensure!(
        art.len() == TILE_BYTES,
        "generated tile art is not 16 bytes"
    );
    let physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(logical_tile & 0x7f);
    let offset = physical * TILE_BYTES;
    graphics
        .get_mut(offset..offset + TILE_BYTES)
        .with_context(|| format!("generated tile {logical_tile:#04x} has no physical slot"))?
        .copy_from_slice(art);
    Ok(())
}

fn outdoor_trash_art(source_2bpp: &[u8], lab_2bpp: &[u8]) -> Result<[[u8; TILE_BYTES]; 4]> {
    let ground = decode_tile(tile_2bpp(source_2bpp, usize::from(OUTDOOR_GROUND_TILE))?)?;
    let lab_floor = decode_tile(tile_2bpp(lab_2bpp, usize::from(LAB_FLOOR_TILE))?)?;
    let mut source = [[0_u8; 16]; 16];
    for (quadrant, tile) in [0x0e_u8, 0x0f, 0x1e, 0x1f].into_iter().enumerate() {
        let decoded = decode_tile(tile_2bpp(lab_2bpp, usize::from(tile))?)?;
        let origin_x = quadrant % 2 * 8;
        let origin_y = quadrant / 2 * 8;
        for y in 0..8 {
            source[origin_y + y][origin_x..origin_x + 8].copy_from_slice(&decoded[y]);
        }
    }

    let mut composed = [[0_u8; 16]; 16];
    for y in 0..16 {
        for x in 0..16 {
            composed[y][x] = ground[y % 8][x % 8];
        }
        let changed = (0..16)
            .filter(|&x| source[y][x] != lab_floor[y % 8][x % 8])
            .collect::<Vec<_>>();
        if let (Some(&first), Some(&last)) = (changed.first(), changed.last()) {
            composed[y][first..=last].copy_from_slice(&source[y][first..=last]);
        }
    }

    let mut output = [[0_u8; TILE_BYTES]; 4];
    for (quadrant, target) in output.iter_mut().enumerate() {
        let origin_x = quadrant % 2 * 8;
        let origin_y = quadrant / 2 * 8;
        let mut pixels = [[0_u8; 8]; 8];
        for y in 0..8 {
            pixels[y].copy_from_slice(&composed[origin_y + y][origin_x..origin_x + 8]);
        }
        *target = encode_tile(&pixels);
    }
    Ok(output)
}

fn decode_tile(bytes: &[u8]) -> Result<[[u8; 8]; 8]> {
    ensure!(bytes.len() == TILE_BYTES, "2bpp tile is not 16 bytes");
    let mut pixels = [[0_u8; 8]; 8];
    for (y, row) in pixels.iter_mut().enumerate() {
        let low = bytes[y * 2];
        let high = bytes[y * 2 + 1];
        for (x, pixel) in row.iter_mut().enumerate() {
            let shift = 7 - x;
            *pixel = (low >> shift) & 1 | ((high >> shift) & 1) << 1;
        }
    }
    Ok(pixels)
}

fn encode_tile(pixels: &[[u8; 8]; 8]) -> [u8; TILE_BYTES] {
    let mut bytes = [0_u8; TILE_BYTES];
    for (y, row) in pixels.iter().enumerate() {
        for (x, pixel) in row.iter().copied().enumerate() {
            let shift = 7 - x;
            bytes[y * 2] |= (pixel & 1) << shift;
            bytes[y * 2 + 1] |= ((pixel >> 1) & 1) << shift;
        }
    }
    bytes
}

fn encode_2bpp_sheet_png(bytes: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        bytes.len().is_multiple_of(TILE_BYTES),
        "tileset 2bpp graphics are not tile-aligned"
    );
    let tile_count = bytes.len() / TILE_BYTES;
    ensure!(
        tile_count.is_multiple_of(SHEET_TILES_WIDE),
        "tileset physical tile count is not sheet-aligned"
    );
    let width = u32::try_from(SHEET_TILES_WIDE * 8)?;
    let height = u32::try_from(tile_count / SHEET_TILES_WIDE * 8)?;
    let mut image = GrayImage::new(width, height);
    for physical in 0..tile_count {
        let pixels = decode_tile(tile_2bpp(bytes, physical)?)?;
        let origin_x = physical % SHEET_TILES_WIDE * 8;
        let origin_y = physical / SHEET_TILES_WIDE * 8;
        for (y, row) in pixels.iter().enumerate() {
            for (x, pixel) in row.iter().copied().enumerate() {
                image.put_pixel(
                    u32::try_from(origin_x + x)?,
                    u32::try_from(origin_y + y)?,
                    Luma([255_u8.saturating_sub(pixel * 85)]),
                );
            }
        }
    }
    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageLuma8(image)
        .write_to(&mut encoded, ImageFormat::Png)
        .context("encode generated tileset PNG")?;
    Ok(encoded.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED_TRASH_ART: [[u8; TILE_BYTES]; 4] = [
        [
            0x00, 0x00, 0x43, 0x03, 0x0c, 0x0f, 0x10, 0x1f, 0xa0, 0x3f, 0x23, 0x3f, 0x24, 0x3f,
            0x78, 0x3f,
        ],
        [
            0x00, 0x00, 0xc1, 0x80, 0x68, 0xe0, 0x10, 0xf0, 0x08, 0xf8, 0x89, 0xf8, 0x48, 0xf8,
            0x38, 0xf8,
        ],
        [
            0x2c, 0x2f, 0x63, 0x23, 0x20, 0x30, 0x10, 0x1c, 0x90, 0x33, 0x0c, 0x1c, 0x03, 0x0f,
            0x40, 0x00,
        ],
        [
            0x68, 0xe8, 0x89, 0x88, 0x08, 0x18, 0x10, 0x70, 0x10, 0x98, 0x61, 0x70, 0x80, 0xe0,
            0x40, 0x00,
        ],
    ];

    fn canonical_definitions() -> BTreeMap<String, TilesetDefinition> {
        [
            (
                SOURCE_TILESET_ID,
                include_str!("../../../../apps/web/assets/data/tilesets/johto_modern.json"),
                include_str!(
                    "../../../../apps/web/assets/data/tilesets/johto_modern_palette_map.json"
                ),
            ),
            (
                TRADITIONAL_TILESET_ID,
                include_str!("../../../../apps/web/assets/data/tilesets/johto.json"),
                include_str!("../../../../apps/web/assets/data/tilesets/johto_palette_map.json"),
            ),
            (
                PARK_TILESET_ID,
                include_str!("../../../../apps/web/assets/data/tilesets/park.json"),
                include_str!("../../../../apps/web/assets/data/tilesets/park_palette_map.json"),
            ),
            (
                LAB_TILESET_ID,
                include_str!("../../../../apps/web/assets/data/tilesets/lab.json"),
                include_str!("../../../../apps/web/assets/data/tilesets/lab_palette_map.json"),
            ),
            (
                CAVE_TILESET_ID,
                include_str!("../../../../apps/web/assets/data/tilesets/cave.json"),
                include_str!("../../../../apps/web/assets/data/tilesets/cave_palette_map.json"),
            ),
            (
                ICE_PATH_TILESET_ID,
                include_str!("../../../../apps/web/assets/data/tilesets/ice_path.json"),
                include_str!("../../../../apps/web/assets/data/tilesets/ice_path_palette_map.json"),
            ),
        ]
        .into_iter()
        .map(|(id, collision, palette)| {
            (
                id.to_string(),
                TilesetDefinition {
                    collision: serde_json::from_str(collision).expect("collision JSON"),
                    palette_map: serde_json::from_str(palette).expect("palette JSON"),
                },
            )
        })
        .collect()
    }

    fn canonical_files() -> BTreeMap<String, Vec<u8>> {
        [
            (
                "data/tilesets/johto_modern_metatiles.bin",
                include_bytes!(
                    "../../../../apps/web/assets/data/tilesets/johto_modern_metatiles.bin"
                )
                .as_slice(),
            ),
            (
                "data/tilesets/johto_metatiles.bin",
                include_bytes!("../../../../apps/web/assets/data/tilesets/johto_metatiles.bin")
                    .as_slice(),
            ),
            (
                "data/tilesets/park_metatiles.bin",
                include_bytes!("../../../../apps/web/assets/data/tilesets/park_metatiles.bin")
                    .as_slice(),
            ),
            (
                "data/tilesets/lab_metatiles.bin",
                include_bytes!("../../../../apps/web/assets/data/tilesets/lab_metatiles.bin")
                    .as_slice(),
            ),
            (
                "data/tilesets/cave_metatiles.bin",
                include_bytes!("../../../../apps/web/assets/data/tilesets/cave_metatiles.bin")
                    .as_slice(),
            ),
            (
                "data/tilesets/ice_path_metatiles.bin",
                include_bytes!("../../../../apps/web/assets/data/tilesets/ice_path_metatiles.bin")
                    .as_slice(),
            ),
            (
                "gfx/tilesets/johto_modern.2bpp",
                include_bytes!("../../../../apps/web/assets/gfx/tilesets/johto_modern.2bpp")
                    .as_slice(),
            ),
            (
                "gfx/tilesets/johto.2bpp",
                include_bytes!("../../../../apps/web/assets/gfx/tilesets/johto.2bpp").as_slice(),
            ),
            (
                "gfx/tilesets/park.2bpp",
                include_bytes!("../../../../apps/web/assets/gfx/tilesets/park.2bpp").as_slice(),
            ),
            (
                "gfx/tilesets/lab.2bpp",
                include_bytes!("../../../../apps/web/assets/gfx/tilesets/lab.2bpp").as_slice(),
            ),
            (
                "gfx/tilesets/cave.2bpp",
                include_bytes!("../../../../apps/web/assets/gfx/tilesets/cave.2bpp").as_slice(),
            ),
            (
                "gfx/tilesets/ice_path.2bpp",
                include_bytes!("../../../../apps/web/assets/gfx/tilesets/ice_path.2bpp").as_slice(),
            ),
        ]
        .into_iter()
        .map(|(path, bytes)| (path.to_string(), bytes.to_vec()))
        .collect()
    }

    #[test]
    fn builds_exact_generated_tileset_from_canonical_assets() {
        let source_files = canonical_files();
        let extension = build_extension_from_parts(
            &canonical_definitions(),
            &source_files,
            "generated-tileset".to_string(),
        )
        .expect("build generated tileset");

        assert_eq!(extension.tileset_id, GENERATED_TILESET_ID);
        assert_eq!(extension.metatiles.len(), 157 * METATILE_BYTES);
        assert_eq!(
            &extension.metatiles[usize::from(GENERATED_ICE_FLOOR_METATILE) * METATILE_BYTES
                ..(usize::from(GENERATED_ICE_FLOOR_METATILE) + 1) * METATILE_BYTES],
            ICE_FLOOR_LAYOUT
        );
        assert_eq!(
            extension.definition.collision[&format!("{GENERATED_ICE_FLOOR_METATILE:02x}")],
            ICE_FLOOR_COLLISION.map(str::to_string)
        );
        assert_eq!(
            &extension.metatiles[usize::from(GENERATED_ICE_BOULDER_METATILE) * METATILE_BYTES
                ..(usize::from(GENERATED_ICE_BOULDER_METATILE) + 1) * METATILE_BYTES],
            ICE_BOULDER_LAYOUT
        );
        assert_eq!(
            &extension.metatiles[..128 * METATILE_BYTES],
            source_files["data/tilesets/johto_modern_metatiles.bin"]
        );
        assert_eq!(
            &extension.metatiles[128 * METATILE_BYTES..129 * METATILE_BYTES],
            GENERATED_BENCH_LAYOUT
        );
        assert_eq!(
            &extension.metatiles[129 * METATILE_BYTES..130 * METATILE_BYTES],
            GENERATED_TRASH_LAYOUT
        );
        assert_eq!(
            &extension.metatiles[130 * METATILE_BYTES..131 * METATILE_BYTES],
            GENERATED_FOUNTAIN_LAYOUT
        );
        assert_eq!(
            &extension.metatiles[131 * METATILE_BYTES..132 * METATILE_BYTES],
            GENERATED_PARK_TREE_LAYOUT
        );
        for (offset, source) in PARK_DETAIL_SOURCE_METATILES.into_iter().enumerate() {
            let start = (132 + offset) * METATILE_BYTES;
            assert_eq!(
                &extension.metatiles[start..start + METATILE_BYTES],
                remap_park_detail_layout(
                    metatile(&source_files["data/tilesets/park_metatiles.bin"], source,)
                        .expect("canonical detail layout")
                )
                .expect("remapped detail layout")
            );
        }
        assert_eq!(
            &extension.metatiles[142 * METATILE_BYTES..143 * METATILE_BYTES],
            GENERATED_CLIFF_STAIRS_LAYOUT
        );
        assert_eq!(
            &extension.metatiles[142 * METATILE_BYTES + 8..143 * METATILE_BYTES],
            &[0x4c, 0x4c, 0xe0, 0xe1, 0x4c, 0x4c, 0xe2, 0xe3],
            "the exact Slowpoke Well staircase must occupy the southeast cliff quadrant"
        );
        for (family_offset, targets) in [RED_HOUSE_TARGET_TILES, YELLOW_HOUSE_TARGET_TILES]
            .into_iter()
            .enumerate()
        {
            for (piece, source) in HOUSE_SOURCE_METATILES.into_iter().enumerate() {
                let id = 0x8f_usize + family_offset * 4 + piece;
                let start = id * METATILE_BYTES;
                assert_eq!(
                    &extension.metatiles[start..start + METATILE_BYTES],
                    remap_house_layout(
                        metatile(
                            &source_files["data/tilesets/johto_modern_metatiles.bin"],
                            source,
                        )
                        .expect("canonical house piece"),
                        targets,
                    )
                    .expect("recolored house piece")
                );
            }
        }
        for (piece, source) in TRADITIONAL_HOUSE_SOURCE_METATILES.into_iter().enumerate() {
            let start = (0x97_usize + piece) * METATILE_BYTES;
            assert_eq!(
                &extension.metatiles[start..start + METATILE_BYTES],
                remap_traditional_house_layout(
                    metatile(&source_files["data/tilesets/johto_metatiles.bin"], source,)
                        .expect("canonical traditional house piece")
                )
            );
        }
        assert_eq!(
            extension.definition.collision["80"],
            BENCH_COLLISION.map(str::to_string)
        );
        assert_eq!(
            extension.definition.collision["81"],
            GENERATED_TRASH_COLLISION.map(str::to_string)
        );
        assert_eq!(
            extension.definition.collision["82"],
            FOUNTAIN_COLLISION.map(str::to_string)
        );
        assert_eq!(
            extension.definition.collision["83"],
            PARK_TREE_COLLISION.map(str::to_string)
        );
        for (target, collision) in PARK_DETAIL_TARGET_METATILES
            .into_iter()
            .zip(PARK_DETAIL_COLLISIONS)
        {
            assert_eq!(
                extension.definition.collision[&format!("{target:02x}")],
                collision.map(str::to_string)
            );
        }
        assert_eq!(
            extension.definition.collision["8e"],
            GENERATED_CLIFF_STAIRS_COLLISION.map(str::to_string)
        );
        assert_eq!(
            extension.definition.collision["8e"],
            ["FLOOR", "FLOOR", "WALL", "FLOOR"].map(str::to_string),
            "the cliff stays blocked beside the walkable stair quadrant, with no ladder/warp trigger"
        );
        for target in GENERATED_TILE_TARGETS[..21].iter().copied() {
            assert_eq!(extension.definition.palette_map[usize::from(target)], 0x08);
        }
        for (source, target) in BENCH_SOURCE_TILES.into_iter().zip(BENCH_TARGET_TILES) {
            let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                    .expect("generated bench art"),
                tile_2bpp(&source_files["gfx/tilesets/park.2bpp"], usize::from(source))
                    .expect("canonical Park bench art")
            );
        }
        for (expected, target) in EXPECTED_TRASH_ART.iter().zip(TRASH_TARGET_TILES) {
            let physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, physical).expect("target art"),
                expected
            );
        }
        for (source, target) in FOUNTAIN_SOURCE_TILES.into_iter().zip(FOUNTAIN_TARGET_TILES) {
            let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                    .expect("generated fountain art"),
                tile_2bpp(&source_files["gfx/tilesets/park.2bpp"], usize::from(source))
                    .expect("canonical Park fountain art")
            );
        }
        for ((source, target), palette) in PARK_TREE_LAYOUT
            .into_iter()
            .zip(PARK_TREE_TARGET_TILES)
            .zip(PARK_TREE_PALETTES)
        {
            let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                    .expect("generated Park large-tree art"),
                tile_2bpp(&source_files["gfx/tilesets/park.2bpp"], usize::from(source))
                    .expect("canonical Park large-tree art")
            );
            assert_eq!(
                extension.definition.palette_map[usize::from(target)],
                palette | 0x08
            );
        }
        for ((source, target), palette) in PARK_DETAIL_SOURCE_TILES
            .into_iter()
            .zip(PARK_DETAIL_TARGET_TILES)
            .zip(PARK_DETAIL_SOURCE_PALETTES)
        {
            let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                    .expect("generated Park detail art"),
                tile_2bpp(&source_files["gfx/tilesets/park.2bpp"], usize::from(source))
                    .expect("canonical Park detail art")
            );
            assert_eq!(
                extension.definition.palette_map[usize::from(target)],
                palette | 0x08
            );
        }
        for (source, target) in CAVE_STAIRS_SOURCE_TILES
            .into_iter()
            .zip(CAVE_STAIRS_TARGET_TILES)
        {
            let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                    .expect("generated cliff stair art"),
                tile_2bpp(&source_files["gfx/tilesets/cave.2bpp"], usize::from(source))
                    .expect("canonical Cave stair art")
            );
            assert_eq!(extension.definition.palette_map[usize::from(target)], 0x0d);
        }
        for (targets, expected_palette) in [
            (RED_HOUSE_TARGET_TILES, 0x09),
            (YELLOW_HOUSE_TARGET_TILES, 0x0c),
        ] {
            for (source, target) in HOUSE_RECOLOR_SOURCE_TILES.into_iter().zip(targets) {
                let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
                assert_eq!(
                    tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                        .expect("generated recolored house art"),
                    tile_2bpp(
                        &source_files["gfx/tilesets/johto_modern.2bpp"],
                        usize::from(source),
                    )
                    .expect("canonical house art")
                );
                assert_eq!(
                    extension.definition.palette_map[usize::from(target)],
                    expected_palette
                );
            }
        }
        for (source, target) in TRADITIONAL_HOUSE_SOURCE_TILES
            .into_iter()
            .zip(TRADITIONAL_HOUSE_TARGET_TILES)
        {
            let source_palette =
                canonical_definitions()[TRADITIONAL_TILESET_ID].palette_map[usize::from(source)];
            let source_physical = usize::from((source_palette >> 3) & 1)
                * SOURCE_TILES_PER_VRAM_BANK
                + usize::from(source & 0x7f);
            let target_physical = GENERATED_TILES_PER_VRAM_BANK + usize::from(target & 0x7f);
            assert_eq!(
                tile_2bpp(&extension.tile_graphics_2bpp, target_physical)
                    .expect("generated traditional house art"),
                tile_2bpp(&source_files["gfx/tilesets/johto.2bpp"], source_physical,)
                    .expect("canonical traditional house art")
            );
            assert_eq!(extension.definition.palette_map[usize::from(target)], 0x0d);
        }

        let png = image::load_from_memory(&extension.tile_graphics_png)
            .expect("decode generated PNG")
            .to_luma8();
        assert_eq!(png.dimensions(), (128, 128));
        assert_eq!(
            extension.tile_graphics_2bpp.len(),
            GENERATED_PHYSICAL_TILE_COUNT * TILE_BYTES
        );
    }

    #[test]
    fn imports_ten_distinct_national_park_metatile_types() {
        let source_files = canonical_files();
        let extension = build_extension_from_parts(
            &canonical_definitions(),
            &source_files,
            "ten-new-types".to_string(),
        )
        .expect("build generated tileset");
        assert_eq!(
            PARK_DETAIL_TARGET_METATILES,
            [0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d]
        );
        let layouts = PARK_DETAIL_TARGET_METATILES
            .into_iter()
            .map(|id| {
                metatile(&extension.metatiles, id)
                    .expect("generated National Park metatile must exist")
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            layouts.len(),
            10,
            "all ten new tile types must have genuinely distinct layouts"
        );
    }

    #[test]
    fn refuses_to_overwrite_a_canonical_physical_tile() {
        let definitions = canonical_definitions();
        let mut files = canonical_files();
        files
            .get_mut("data/tilesets/johto_modern_metatiles.bin")
            .expect("metatiles")[0] = 0x9b;

        let error =
            build_extension_from_parts(&definitions, &files, "generated-tileset".to_string())
                .expect_err("used art target must be rejected");
        assert!(error.to_string().contains("already used"), "{error:#}");
    }

    #[test]
    fn refuses_to_overwrite_a_large_tree_physical_tile() {
        let definitions = canonical_definitions();
        let mut files = canonical_files();
        let metatiles = files
            .get_mut("data/tilesets/johto_modern_metatiles.bin")
            .expect("metatiles");
        metatiles[0] = 0xb4;

        let error =
            build_extension_from_parts(&definitions, &files, "generated-tileset".to_string())
                .expect_err("used large-tree art target must be rejected");
        assert!(error.to_string().contains("already used"), "{error:#}");
    }

    #[test]
    fn refuses_changed_park_large_tree_palettes() {
        let mut definitions = canonical_definitions();
        definitions
            .get_mut(PARK_TILESET_ID)
            .expect("Park definition")
            .palette_map[0x2d] = 0x02;

        let error = build_extension_from_parts(
            &definitions,
            &canonical_files(),
            "generated-tileset".to_string(),
        )
        .expect_err("changed Park tree palette must be rejected");
        assert!(
            error.to_string().contains("palette/bank changed"),
            "{error:#}"
        );
    }

    #[test]
    fn trash_extraction_replaces_the_indoor_floor_with_outdoor_ground() {
        let files = canonical_files();
        let trash = outdoor_trash_art(
            &files["gfx/tilesets/johto_modern.2bpp"],
            &files["gfx/tilesets/lab.2bpp"],
        )
        .expect("extract trash art");

        assert_eq!(trash, EXPECTED_TRASH_ART);
        assert_ne!(
            trash[0].as_slice(),
            tile_2bpp(&files["gfx/tilesets/lab.2bpp"], 0x0e).expect("raw Lab tile")
        );
    }
}
