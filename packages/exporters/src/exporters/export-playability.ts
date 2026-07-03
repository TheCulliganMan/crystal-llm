import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import { getSpawnPoint, mapConstantToName, Spawn } from "@pokecrystal/core/engine/world/maps";
import type { StoryCommand, StoryScripts } from "./export-story-events";

export type ProgressionRequirements = {
  events: string[];
  items: string[];
  maps: string[];
};

export type ProgressionGrants = {
  events: string[];
  items: string[];
  maps: string[];
};

export type ProgressionRule = {
  id: string;
  requires: ProgressionRequirements;
  grants: ProgressionGrants;
};

export type MapAccessRule = {
  map: string;
  requires: ProgressionRequirements;
};

export type PlayabilityRules = {
  start_maps: string[];
  start_tiles: Array<{ map: string; tile: { x: number; y: number } }>;
  initial_events: string[];
  initial_items: string[];
  goal_maps: string[];
  goal_events: string[];
  goal_items: string[];
  progression_rules: ProgressionRule[];
  map_access: MapAccessRule[];
  require_all_maps_reachable: boolean;
  require_walkable_maps: boolean;
};

type InitializeEventsConfig = {
  eventFlags?: string[];
  engineFlags?: string[];
};

export type BuildPlayabilityOptions = {
  itemIds?: Iterable<string>;
  start?: { map: string; x: number; y: number };
};

const COMPLETION_EVENT = "EVENT_HALL_OF_FAME";

const commandArgs = (command: StoryCommand): string[] =>
  Array.isArray(command.args) ? command.args : [command.args].filter(Boolean);

const addUnique = (target: string[], value: unknown): void => {
  if (typeof value !== "string" || !value.trim()) {
    return;
  }
  if (!target.includes(value)) {
    target.push(value);
  }
};

const sortedUnique = (values: Iterable<string>): string[] => Array.from(new Set(values)).sort();

const completeRequirements = (requirements: Partial<ProgressionRequirements>): ProgressionRequirements => ({
  events: sortedUnique(requirements.events ?? []),
  items: sortedUnique(requirements.items ?? []),
  maps: sortedUnique(requirements.maps ?? []),
});

const completeGrants = (grants: Partial<ProgressionGrants>): ProgressionGrants => ({
  events: sortedUnique(grants.events ?? []),
  items: sortedUnique(grants.items ?? []),
  maps: sortedUnique(grants.maps ?? []),
});

const isGrantingItemCommand = (command: string): boolean =>
  command === "giveitem" || command === "verbosegiveitem" || command === "itemball";

const isMapSentinel = (mapConstant: string): boolean => mapConstant.trim() === "NONE";

const scriptedWarpMapArg = (command: string, args: string[]): string | null => {
  if (command === "warp" && args[0] && !isMapSentinel(args[0])) {
    return args[0];
  }
  if (command === "warpfacing" && args[0] && !isMapSentinel(args[0])) {
    return args[0];
  }
  return null;
};

export function buildPlayabilityFromStoryEvents(
  storyEvents: Record<string, StoryScripts>,
  initializeEvents: InitializeEventsConfig = {},
  options: BuildPlayabilityOptions = {}
): PlayabilityRules {
  const itemIds = new Set(options.itemIds ?? []);
  const initialEvents = [
    ...(initializeEvents.eventFlags ?? []),
    ...(initializeEvents.engineFlags ?? []),
  ].filter((value): value is string => typeof value === "string" && value.length > 0);
  const progressionRules: ProgressionRule[] = [];
  const goalEvents: string[] = [];

  for (const [mapName, scripts] of Object.entries(storyEvents).sort(([a], [b]) => a.localeCompare(b))) {
    for (const [scriptName, commands] of Object.entries(scripts).sort(([a], [b]) => a.localeCompare(b))) {
      const grants: ProgressionGrants = {
        events: [],
        items: [],
        maps: [],
      };

      for (const entry of commands) {
        const args = commandArgs(entry);
        if (entry.command === "setevent" || entry.command === "setflag") {
          addUnique(grants.events, args[0]);
          continue;
        }
        if (isGrantingItemCommand(entry.command) && itemIds.has(args[0])) {
          addUnique(grants.items, args[0]);
          continue;
        }
        if (entry.command === "halloffame") {
          addUnique(grants.events, COMPLETION_EVENT);
          addUnique(goalEvents, COMPLETION_EVENT);
          continue;
        }
        const warpMap = scriptedWarpMapArg(entry.command, args);
        if (warpMap) {
          addUnique(grants.maps, mapConstantToName(warpMap));
        }
      }

      const completedGrants = completeGrants(grants);
      if (!completedGrants.events.length && !completedGrants.items.length && !completedGrants.maps.length) {
        continue;
      }
      progressionRules.push({
        id: `script:${mapName}:${scriptName}`,
        requires: completeRequirements({ maps: [mapName] }),
        grants: completedGrants,
      });
    }
  }

  const start = options.start;
  const rules: PlayabilityRules = {
    start_maps: [],
    start_tiles: [],
    initial_events: sortedUnique(initialEvents),
    initial_items: [],
    goal_maps: [],
    goal_events: sortedUnique(goalEvents),
    goal_items: [],
    progression_rules: progressionRules,
    map_access: [],
    require_all_maps_reachable: false,
    require_walkable_maps: true,
  };

  if (start) {
    rules.start_maps = [start.map];
    rules.start_tiles = [{ map: start.map, tile: { x: start.x, y: start.y } }];
  }

  return rules;
}

export function exportPlayability(options: { itemIds?: Iterable<string> } = {}): PlayabilityRules {
  const storyEvents = readJsonAssetSync<Record<string, StoryScripts>>(joinPath(getDataDir(), "story_events.json"));
  const initializeEvents = readJsonAssetSync<InitializeEventsConfig>(joinPath(getDataDir(), "initialize_events.json"));
  const spawn = getSpawnPoint(Spawn.HOME);
  return buildPlayabilityFromStoryEvents(storyEvents, initializeEvents, {
    itemIds: options.itemIds,
    start: {
      map: spawn.mapName,
      x: spawn.tileX,
      y: spawn.tileY,
    },
  });
}
