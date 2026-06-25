import {
  WildEncounter,
  WildEncounterData,
  WildEncounterTable,
} from "@pokecrystal/assets/content/wild-encounter-data";

export enum SwarmType {
  DUNSPARCE = 0,
  YANMA = 1,
}

const SwarmType_VALUES = new Set(Object.values(SwarmType));

export type SwarmDefinition = {
  swarmType: SwarmType;
  bitMask: number;
  mapConstant: string;
  mapGroupAttr: string;
  mapNumberAttr: string;
  encounterData: WildEncounterData;
};

const buildEncounters = (
  entries: Array<[number, string]>
): WildEncounter[] => {
  return entries.map(([level, species]) => ({ level, species }));
};

const DUNSPARCE_ENTRIES: Array<[number, string]> = [
  [3, "GEODUDE"],
  [3, "DUNSPARCE"],
  [2, "ZUBAT"],
  [2, "GEODUDE"],
  [2, "DUNSPARCE"],
  [4, "DUNSPARCE"],
  [4, "DUNSPARCE"],
];

const YANMA_DAY_ENTRIES: Array<[number, string]> = [
  [12, "NIDORAN_M"],
  [12, "NIDORAN_F"],
  [12, "YANMA"],
  [14, "YANMA"],
  [14, "PIDGEY"],
  [10, "DITTO"],
  [10, "DITTO"],
];

const YANMA_NIGHT_ENTRIES: Array<[number, string]> = [
  [12, "NIDORAN_M"],
  [12, "NIDORAN_F"],
  [12, "YANMA"],
  [14, "YANMA"],
  [14, "HOOTHOOT"],
  [10, "DITTO"],
  [10, "DITTO"],
];

const SWARM_DEFINITIONS: Record<SwarmType, SwarmDefinition> = {
  [SwarmType.DUNSPARCE]: {
    swarmType: SwarmType.DUNSPARCE,
    bitMask: 1 << 2,
    mapConstant: "DARK_CAVE_VIOLET_ENTRANCE",
    mapGroupAttr: "wDunsparceMapGroup",
    mapNumberAttr: "wDunsparceMapNumber",
    encounterData: {
      map_name: "DARK_CAVE_VIOLET_ENTRANCE",
      grass_rates: { morning: 4, day: 4, night: 4 },
      water_rate: null,
      grass: {
        morning: buildEncounters(DUNSPARCE_ENTRIES),
        day: buildEncounters(DUNSPARCE_ENTRIES),
        night: buildEncounters(DUNSPARCE_ENTRIES),
      } as WildEncounterTable,
      water: null,
    } as WildEncounterData,
  },
  [SwarmType.YANMA]: {
    swarmType: SwarmType.YANMA,
    bitMask: 1 << 3,
    mapConstant: "ROUTE_35",
    mapGroupAttr: "wYanmaMapGroup",
    mapNumberAttr: "wYanmaMapNumber",
    encounterData: {
      map_name: "ROUTE_35",
      grass_rates: { morning: 10, day: 10, night: 10 },
      water_rate: null,
      grass: {
        morning: buildEncounters(YANMA_DAY_ENTRIES),
        day: buildEncounters(YANMA_DAY_ENTRIES),
        night: buildEncounters(YANMA_NIGHT_ENTRIES),
      } as WildEncounterTable,
      water: null,
    } as WildEncounterData,
  },
};

export const ALL_SWARM_DEFINITIONS: SwarmDefinition[] = Object.values(
  SWARM_DEFINITIONS
);

const parseNumericToken = (token: string): number => {
  const cleaned = token.trim();
  if (!cleaned) {
    throw new Error("Empty swarm token.");
  }
  if (cleaned.startsWith("$")) {
    return parseInt(cleaned.slice(1), 16);
  }
  if (cleaned.toLowerCase().startsWith("0x")) {
    return parseInt(cleaned, 16);
  }
  return parseInt(cleaned, 10);
};

const TOKEN_TO_TYPE: Record<string, SwarmType> = Object.fromEntries(
  ALL_SWARM_DEFINITIONS.map((definition) => [
    `SWARM_${SwarmType[definition.swarmType]}`,
    definition.swarmType,
  ])
);

export const getSwarmDefinition = (swarmType: SwarmType): SwarmDefinition => {
  const definition = SWARM_DEFINITIONS[swarmType];
  if (!definition) {
    throw new Error(`Unknown swarm type '${swarmType}'.`);
  }
  return definition;
};

export const resolveSwarmDefinition = (token: string): SwarmDefinition => {
  const normalized = token.trim().toUpperCase();
  let swarmType = TOKEN_TO_TYPE[normalized];
  if (swarmType === undefined) {
    const value = parseNumericToken(normalized);
    if (!SwarmType_VALUES.has(value as SwarmType)) {
      throw new Error(`Unknown swarm token '${token}'.`);
    }
    swarmType = value as SwarmType;
  }
  return getSwarmDefinition(swarmType);
};
