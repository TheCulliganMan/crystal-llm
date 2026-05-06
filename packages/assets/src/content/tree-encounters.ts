// ASM: data/wild/treemons.asm GetTreeScore/GetTreeMon/RockMonEncounter logic.
// ASM: data/wild/treemons_asleep.asm CheckSleepingTreeMon lists.
import { canonicaliseTimeOfDay } from "@pokecrystal/core/engine/systems/time";

export interface TreeMonEntry {
  weight: number;
  species: string;
  level: number;
}

export interface TreeMonSet {
  common: TreeMonEntry[];
  rare: TreeMonEntry[];
}

const TREE_MON_SETS: Record<string, TreeMonSet> = {
  TREEMON_SET_CANYON: {
    common: [
      { weight: 50, species: 'SPEAROW', level: 10 },
      { weight: 15, species: 'SPEAROW', level: 10 },
      { weight: 15, species: 'SPEAROW', level: 10 },
      { weight: 10, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
    ],
    rare: [
      { weight: 50, species: 'SPEAROW', level: 10 },
      { weight: 15, species: 'HERACROSS', level: 10 },
      { weight: 15, species: 'HERACROSS', level: 10 },
      { weight: 10, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
    ],
  },
  TREEMON_SET_TOWN: {
    common: [
      { weight: 50, species: 'SPEAROW', level: 10 },
      { weight: 15, species: 'EKANS', level: 10 },
      { weight: 15, species: 'SPEAROW', level: 10 },
      { weight: 10, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
    ],
    rare: [
      { weight: 50, species: 'SPEAROW', level: 10 },
      { weight: 15, species: 'HERACROSS', level: 10 },
      { weight: 15, species: 'HERACROSS', level: 10 },
      { weight: 10, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
      { weight: 5, species: 'AIPOM', level: 10 },
    ],
  },
  TREEMON_SET_ROUTE: {
    common: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'SPINARAK', level: 10 },
      { weight: 15, species: 'LEDYBA', level: 10 },
      { weight: 10, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
    ],
    rare: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 10, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
    ],
  },
  TREEMON_SET_KANTO: {
    common: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'EKANS', level: 10 },
      { weight: 15, species: 'HOOTHOOT', level: 10 },
      { weight: 10, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
    ],
    rare: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 10, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
    ],
  },
  TREEMON_SET_LAKE: {
    common: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'VENONAT', level: 10 },
      { weight: 15, species: 'HOOTHOOT', level: 10 },
      { weight: 10, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
    ],
    rare: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 10, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
      { weight: 5, species: 'EXEGGCUTE', level: 10 },
    ],
  },
  TREEMON_SET_FOREST: {
    common: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 15, species: 'PINECO', level: 10 },
      { weight: 10, species: 'NOCTOWL', level: 10 },
      { weight: 5, species: 'BUTTERFREE', level: 10 },
      { weight: 5, species: 'BEEDRILL', level: 10 },
    ],
    rare: [
      { weight: 50, species: 'HOOTHOOT', level: 10 },
      { weight: 15, species: 'CATERPIE', level: 10 },
      { weight: 15, species: 'WEEDLE', level: 10 },
      { weight: 10, species: 'HOOTHOOT', level: 10 },
      { weight: 5, species: 'METAPOD', level: 10 },
      { weight: 5, species: 'KAKUNA', level: 10 },
    ],
  },
  TREEMON_SET_ROCK: {
    common: [
      { weight: 90, species: 'KRABBY', level: 15 },
      { weight: 10, species: 'SHUCKLE', level: 15 },
    ],
    rare: [],
  },
};

const TREE_MON_MAPS: Record<string, string> = {
  ROUTE_26: 'TREEMON_SET_KANTO',
  ROUTE_27: 'TREEMON_SET_KANTO',
  ROUTE_28: 'TREEMON_SET_NONE',
  ROUTE_29: 'TREEMON_SET_ROUTE',
  ROUTE_30: 'TREEMON_SET_ROUTE',
  ROUTE_31: 'TREEMON_SET_ROUTE',
  ROUTE_32: 'TREEMON_SET_KANTO',
  ROUTE_33: 'TREEMON_SET_TOWN',
  ROUTE_34: 'TREEMON_SET_ROUTE',
  ROUTE_35: 'TREEMON_SET_ROUTE',
  ROUTE_36: 'TREEMON_SET_ROUTE',
  ROUTE_37: 'TREEMON_SET_ROUTE',
  ROUTE_38: 'TREEMON_SET_ROUTE',
  ROUTE_39: 'TREEMON_SET_ROUTE',
  ROUTE_40: 'TREEMON_SET_NONE',
  ROUTE_41: 'TREEMON_SET_NONE',
  ROUTE_42: 'TREEMON_SET_TOWN',
  ROUTE_43: 'TREEMON_SET_LAKE',
  ROUTE_44: 'TREEMON_SET_CANYON',
  ROUTE_45: 'TREEMON_SET_CANYON',
  ROUTE_46: 'TREEMON_SET_CANYON',
  NEW_BARK_TOWN: 'TREEMON_SET_NONE',
  CHERRYGROVE_CITY: 'TREEMON_SET_NONE',
  VIOLET_CITY: 'TREEMON_SET_NONE',
  AZALEA_TOWN: 'TREEMON_SET_TOWN',
  CIANWOOD_CITY: 'TREEMON_SET_NONE',
  GOLDENROD_CITY: 'TREEMON_SET_NONE',
  OLIVINE_CITY: 'TREEMON_SET_NONE',
  ECRUTEAK_CITY: 'TREEMON_SET_NONE',
  MAHOGANY_TOWN: 'TREEMON_SET_NONE',
  LAKE_OF_RAGE: 'TREEMON_SET_LAKE',
  BLACKTHORN_CITY: 'TREEMON_SET_NONE',
  SILVER_CAVE_OUTSIDE: 'TREEMON_SET_NONE',
  ILEX_FOREST: 'TREEMON_SET_FOREST',
};

const ROCK_MON_MAPS: Record<string, string> = {
  CIANWOOD_CITY: 'TREEMON_SET_ROCK',
  ROUTE_40: 'TREEMON_SET_ROCK',
  DARK_CAVE_VIOLET_ENTRANCE: 'TREEMON_SET_ROCK',
  SLOWPOKE_WELL_B1F: 'TREEMON_SET_ROCK',
};

const TREEMON_SCORE_BAD = 0;
const TREEMON_SCORE_GOOD = 1;
const TREEMON_SCORE_RARE = 2;
export const TREEMON_SLEEP_TURNS = 7;

const ASLEEP_TREE_MONS: Record<string, Set<string>> = {
  MORN: new Set(["VENONAT", "HOOTHOOT", "NOCTOWL", "SPINARAK", "HERACROSS"]),
  DAY: new Set(["VENONAT", "HOOTHOOT", "NOCTOWL", "SPINARAK", "HERACROSS"]),
  NIGHT: new Set([
    "CATERPIE",
    "METAPOD",
    "BUTTERFREE",
    "WEEDLE",
    "KAKUNA",
    "BEEDRILL",
    "SPEAROW",
    "EKANS",
    "EXEGGCUTE",
    "LEDYBA",
    "AIPOM",
  ]),
};

export function getTreeSetForMap(mapConstant: string): TreeMonSet | null {
  if (!mapConstant) {
    return null;
  }
  const setName = TREE_MON_MAPS[mapConstant.toUpperCase()];
  if (!setName || setName === 'TREEMON_SET_NONE') {
    return null;
  }
  return TREE_MON_SETS[setName] ?? null;
}

export function getRockSetForMap(mapConstant: string): TreeMonSet | null {
  if (!mapConstant) {
    return null;
  }
  const setName = ROCK_MON_MAPS[mapConstant.toUpperCase()];
  if (!setName || setName === 'TREEMON_SET_NONE') {
    return null;
  }
  return TREE_MON_SETS[setName] ?? null;
}

export function computeTreeScore(tileX: number, tileY: number, playerId: number): number {
  const value = tileY * (tileX + 1) + tileX;
  const coordScore = Math.floor(value / 5) % 10;
  const trainerScore = playerId % 10;
  const diff = (coordScore - trainerScore + 10) % 10;
  if (diff === 0) {
    return TREEMON_SCORE_RARE;
  }
  if (diff < 5) {
    return TREEMON_SCORE_GOOD;
  }
  return TREEMON_SCORE_BAD;
}

export function isAsleepTreeMon(speciesId: string, timeOfDay: string | null | undefined): boolean {
  const normalized = canonicaliseTimeOfDay(timeOfDay ?? "DAY");
  const list = ASLEEP_TREE_MONS[normalized] ?? ASLEEP_TREE_MONS.DAY;
  return list.has(String(speciesId ?? "").toUpperCase());
}

export function chooseTreeEncounter(
  treeSet: TreeMonSet,
  score: number,
  randrange: (maxExclusive: number) => number,
): [string, number] | null {
  let entries: TreeMonEntry[];
  if (score === TREEMON_SCORE_BAD) {
    if (randrange(10) !== 0) {
      return null;
    }
    entries = treeSet.common;
  } else if (score === TREEMON_SCORE_GOOD) {
    if (randrange(10) >= 5) {
      return null;
    }
    entries = treeSet.common;
  } else if (score === TREEMON_SCORE_RARE) {
    if (randrange(10) >= 8) {
      return null;
    }
    entries = treeSet.rare;
  } else {
    throw new Error(`Unknown tree score value ${score}.`);
  }

  return chooseEntry(entries, randrange);
}

export function chooseRockSmashEncounter(
  treeSet: TreeMonSet,
  randrange: (maxExclusive: number) => number,
): [string, number] | null {
  if (treeSet.common.length === 0) {
    return null;
  }
  if (randrange(10) >= 4) {
    return null;
  }
  return chooseEntry(treeSet.common, randrange);
}

function chooseEntry(
  entries: TreeMonEntry[],
  randrange: (maxExclusive: number) => number,
): [string, number] | null {
  if (entries.length === 0) {
    return null;
  }
  let totalRoll = randrange(100);
  for (const entry of entries) {
    if (entry.weight <= 0) {
      continue;
    }
    if (totalRoll < entry.weight) {
      return [entry.species, entry.level];
    }
    totalRoll -= entry.weight;
  }
  return null;
}
