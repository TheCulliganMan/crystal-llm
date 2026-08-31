import { Spawn } from "@pokecrystal/core/engine/world/maps";
import { BADGE_ENGINE_FLAG_ORDER } from "@pokecrystal/core/core/badges";

export enum CollisionType {
    CUT_TREE = 0x12,
    CUT_TREE_ALT = 0x1A,
    WHIRLPOOL = 0x24,
    WHIRLPOOL_ALT = 0x2C,
    WATERFALL_RIGHT = 0x30,
    WATERFALL_LEFT = 0x31,
    WATERFALL_UP = 0x32,
    WATERFALL = 0x33,
    CURRENT_DOWN = 0x3B,
    TALL_GRASS = 0x18,
    LONG_GRASS = 0x14,
    LONG_GRASS_ALT = 0x1C,
}

export const CUTTABLE_COLLISIONS = new Set([
    CollisionType.CUT_TREE,
    CollisionType.CUT_TREE_ALT,
]);

export const WHIRLPOOL_COLLISIONS = new Set([
    CollisionType.WHIRLPOOL,
    CollisionType.WHIRLPOOL_ALT,
]);

export const WATERFALL_COLLISIONS = new Set([
    CollisionType.WATERFALL,
    CollisionType.WATERFALL_RIGHT,
    CollisionType.WATERFALL_LEFT,
    CollisionType.WATERFALL_UP,
    CollisionType.CURRENT_DOWN,
]);

export const TALL_GRASS_COLLISIONS = new Set([
    CollisionType.TALL_GRASS,
    CollisionType.LONG_GRASS,
    CollisionType.LONG_GRASS_ALT,
]);

// Player event enable bits (constants/ram_constants.asm).
export const PLAYEREVENTS_COUNT_STEPS = 1 << 0;
export const PLAYEREVENTS_COORD_EVENTS = 1 << 1;
export const PLAYEREVENTS_WARPS_AND_CONNECTIONS = 1 << 2;
export const PLAYEREVENTS_WILD_ENCOUNTERS = 1 << 4;
export const PLAYEREVENTS_UNUSED = 1 << 5;
export const DEFAULT_ENABLED_PLAYER_EVENTS = 0xFF;

export const _BADGE_FLAG_NAMES: Record<number, string> = Object.freeze(
    BADGE_ENGINE_FLAG_ORDER.reduce((acc, flagName, index) => {
        acc[index] = flagName;
        return acc;
    }, {} as Record<number, string>)
);

export type CutReplacement = [number, string];

export const _CUT_BLOCKS: Record<string, Record<number, CutReplacement>> = {
    "johto": {
        0x03: [0x02, "grass"],
        0x5B: [0x3C, "tree"],
        0x5F: [0x3D, "tree"],
        0x63: [0x3F, "tree"],
        0x67: [0x3E, "tree"],
    },
    "johto_modern": {
        0x03: [0x02, "grass"],
    },
    "johto_modern_generated": {
        0x03: [0x02, "grass"],
    },
    "kanto": {
        0x0B: [0x0A, "grass"],
        0x32: [0x6D, "tree"],
        0x33: [0x6C, "tree"],
        0x34: [0x6F, "tree"],
        0x35: [0x4C, "tree"],
        0x60: [0x6E, "tree"],
    },
    "park": {
        0x13: [0x03, "grass"],
        0x03: [0x04, "grass"],
    },
    "forest": {
        0x0F: [0x17, "tree"],
    },
};

export const _WHIRLPOOL_BLOCKS: Record<string, Record<number, [number, string]>> = {
    "johto": {
        0x07: [0x36, "whirlpool"],
    }
};

export const _FLY_DESTINATIONS: [string, string, Spawn][] = [
    ["ENGINE_FLYPOINT_NEW_BARK", "LANDMARK_NEW_BARK_TOWN", Spawn.NEW_BARK],
    ["ENGINE_FLYPOINT_CHERRYGROVE", "LANDMARK_CHERRYGROVE_CITY", Spawn.CHERRYGROVE],
    ["ENGINE_FLYPOINT_VIOLET", "LANDMARK_VIOLET_CITY", Spawn.VIOLET],
    ["ENGINE_FLYPOINT_AZALEA", "LANDMARK_AZALEA_TOWN", Spawn.AZALEA],
    ["ENGINE_FLYPOINT_GOLDENROD", "LANDMARK_GOLDENROD_CITY", Spawn.GOLDENROD],
    ["ENGINE_FLYPOINT_ECRUTEAK", "LANDMARK_ECRUTEAK_CITY", Spawn.ECRUTEAK],
    ["ENGINE_FLYPOINT_OLIVINE", "LANDMARK_OLIVINE_CITY", Spawn.OLIVINE],
    ["ENGINE_FLYPOINT_CIANWOOD", "LANDMARK_CIANWOOD_CITY", Spawn.CIANWOOD],
    ["ENGINE_FLYPOINT_MAHOGANY", "LANDMARK_MAHOGANY_TOWN", Spawn.MAHOGANY],
    ["ENGINE_FLYPOINT_LAKE_OF_RAGE", "LANDMARK_LAKE_OF_RAGE", Spawn.LAKE_OF_RAGE],
    ["ENGINE_FLYPOINT_BLACKTHORN", "LANDMARK_BLACKTHORN_CITY", Spawn.BLACKTHORN],
    ["ENGINE_FLYPOINT_SILVER_CAVE", "LANDMARK_SILVER_CAVE", Spawn.MT_SILVER],
    ["ENGINE_FLYPOINT_PALLET", "LANDMARK_PALLET_TOWN", Spawn.PALLET],
    ["ENGINE_FLYPOINT_VIRIDIAN", "LANDMARK_VIRIDIAN_CITY", Spawn.VIRIDIAN],
    ["ENGINE_FLYPOINT_PEWTER", "LANDMARK_PEWTER_CITY", Spawn.PEWTER],
    ["ENGINE_FLYPOINT_CERULEAN", "LANDMARK_CERULEAN_CITY", Spawn.CERULEAN],
    ["ENGINE_FLYPOINT_VERMILION", "LANDMARK_VERMILION_CITY", Spawn.VERMILION],
    ["ENGINE_FLYPOINT_ROCK_TUNNEL", "LANDMARK_ROCK_TUNNEL", Spawn.ROCK_TUNNEL],
    ["ENGINE_FLYPOINT_LAVENDER", "LANDMARK_LAVENDER_TOWN", Spawn.LAVENDER],
    ["ENGINE_FLYPOINT_CELADON", "LANDMARK_CELADON_CITY", Spawn.CELADON],
    ["ENGINE_FLYPOINT_SAFFRON", "LANDMARK_SAFFRON_CITY", Spawn.SAFFRON],
    ["ENGINE_FLYPOINT_FUCHSIA", "LANDMARK_FUCHSIA_CITY", Spawn.FUCHSIA],
    ["ENGINE_FLYPOINT_CINNABAR", "LANDMARK_CINNABAR_ISLAND", Spawn.CINNABAR],
    ["ENGINE_FLYPOINT_INDIGO_PLATEAU", "LANDMARK_INDIGO_PLATEAU", Spawn.INDIGO],
];

export const _TIME_OF_DAY_MASKS: Record<string, number> = {
    "morn": 0b001,
    "day": 0b010,
    "nite": 0b100,
    // Darkness shares palettes with the night period; treat it the same for NPC visibility.
    "darkness": 0b100,
};

export const SPRITES_SKIP_WALKING_GFX_F = 6;
export const SPRITES_SKIP_STANDING_GFX_F = 7;
export const SPRITES_SKIP_WALKING = 1 << SPRITES_SKIP_WALKING_GFX_F;
export const SPRITES_SKIP_STANDING = 1 << SPRITES_SKIP_STANDING_GFX_F;

export const _DIRECTION_VECTORS: Record<string, [number, number]> = {
    "left": [-1, 0],
    "right": [1, 0],
    "up": [0, -1],
    "down": [0, 1],
};
