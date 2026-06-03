import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { adjustCounterTile } from "@pokecrystal/core/engine/world/overworld/counter";
import { getCoordCollision, isPermissionPassable } from "@pokecrystal/core/engine/world/overworld/collision-rules";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";

export type McpMapWarpSnapshot = {
  index: number | null;
  coords: { x: number; y: number };
  target: {
    map_constant: string | null;
    map_name: string | null;
    warp_id: number | null;
  };
};

export type McpMapHotspotType =
  | "warp"
  | "npc"
  | "utility"
  | "trigger"
  | "sign"
  | "shop"
  | "heal"
  | "gym"
  | "objective"
  | "hazard"
  | "landmark"
  | "unknown";

export type McpMapApproachTile = {
  coords: { x: number; y: number };
  facing: "up" | "down" | "left" | "right";
};

export type McpMapHotspot = {
  id: string;
  type: McpMapHotspotType;
  label: string;
  coords: { x: number; y: number };
  visible: boolean;
  interactable: boolean;
  approach_tiles?: McpMapApproachTile[];
  token?: string;
  spoiler_masked?: boolean;
};

export type McpMapInfoSnapshot = {
  map: string | null;
  map_id: string | null;
  coord_stride?: number;
  player?: {
    coords: { x: number; y: number };
    facing?: "up" | "down" | "left" | "right";
  };
  warps: McpMapWarpSnapshot[];
  hotspots: McpMapHotspot[];
};

export type OverworldMapInfoSource = {
  TILES_PER_COLLISION?: number;
  _map_events?: { warps?: unknown[]; bg_events?: unknown[]; coord_events?: unknown[] } | null;
  npcs?: unknown[] | null;
  current_map_name?: string | null;
  _npc_blueprints?: Map<string, Map<string, [unknown, number]>>;
  map?: OverworldMap;
  tileset?: OverworldTilesetLike;
  player_state?: PlayerState;
  _npc_occupying_subtile?: ((x: number, y: number) => unknown) | null;
};

type DataLoaderLike = {
  map_events?: Map<string, { bg_events?: unknown[]; coord_events?: unknown[] }>;
};

type EventFlags = Record<string, boolean | undefined> | null | undefined;

const toNumberOrNull = (value: unknown): number | null =>
  typeof value === "number" && Number.isFinite(value) ? value : null;

const toStringOrNull = (value: unknown): string | null =>
  typeof value === "string" && value.trim().length > 0 ? value : null;

const normalizeName = (value: string | null | undefined): string =>
  String(value ?? "")
    .trim()
    .replace(/\s+/g, "")
    .replace(/_/g, "")
    .toUpperCase();

const compactLabel = (value: string | null | undefined, fallback: string): string => {
  const normalized = String(value ?? "")
    .replace(/_/g, " ")
    .replace(/([a-z0-9])([A-Z])/g, "$1 $2")
    .trim();
  return normalized.length ? normalized : fallback;
};

const hasExactToken = (tokens: readonly string[], candidate: string): boolean => tokens.includes(candidate);

const hasTokenContaining = (tokens: readonly string[], candidate: string): boolean =>
  tokens.some((token) => token.includes(candidate));

const extractSpeciesLabel = (tokens: readonly string[]): string | null => {
  if (hasTokenContaining(tokens, "CHIKORITA")) {
    return "Chikorita";
  }
  if (hasTokenContaining(tokens, "CYNDAQUIL")) {
    return "Cyndaquil";
  }
  if (hasTokenContaining(tokens, "TOTODILE")) {
    return "Totodile";
  }
  return null;
};

const classifyNpcHotspot = (
  sourceTokens: readonly string[]
): Pick<McpMapHotspot, "type" | "label" | "token" | "spoiler_masked"> => {
  const starterSpecies = extractSpeciesLabel(sourceTokens);
  if (starterSpecies && hasTokenContaining(sourceTokens, "POKEBALL")) {
    return {
      type: "objective",
      label: `${starterSpecies} Poke Ball`,
      token: "!",
    };
  }
  if (hasExactToken(sourceTokens, "MOMSCRIPT") || hasTokenContaining(sourceTokens, "PLAYERSHOUSE1FMOM")) {
    return {
      type: "npc",
      label: "Mom",
      token: "N",
    };
  }
  if (hasExactToken(sourceTokens, "ELMSAIDE")) {
    return {
      type: "npc",
      label: "Elm's aide",
      token: "N",
    };
  }
  if (hasExactToken(sourceTokens, "PROFELM") || hasExactToken(sourceTokens, "ELMSLABELM")) {
    return {
      type: "npc",
      label: "Elm",
      token: "N",
    };
  }
  if (hasTokenContaining(sourceTokens, "OFFICER") || hasTokenContaining(sourceTokens, "COPSCRIPT")) {
    return {
      type: "npc",
      label: "Officer",
      token: "N",
    };
  }
  if (hasTokenContaining(sourceTokens, "NURSE")) {
    return { type: "heal", label: "Healer", token: "H" };
  }
  if (
    hasTokenContaining(sourceTokens, "MART") ||
    hasTokenContaining(sourceTokens, "CLERK") ||
    hasTokenContaining(sourceTokens, "SHOP")
  ) {
    return { type: "shop", label: "Shop clerk", token: "$" };
  }
  if (hasTokenContaining(sourceTokens, "GYM") || hasTokenContaining(sourceTokens, "LEADER")) {
    return { type: "gym", label: "Gym NPC", token: "G" };
  }
  if (hasTokenContaining(sourceTokens, "TRAINER")) {
    return { type: "hazard", label: "Trainer", token: "X" };
  }
  if (
    hasTokenContaining(sourceTokens, "ITEM") ||
    hasTokenContaining(sourceTokens, "BALL") ||
    hasTokenContaining(sourceTokens, "POKEBALL")
  ) {
    return { type: "objective", label: "Poke Ball", token: "!" };
  }
  if (isSurpriseKeyword(sourceTokens.join(" "))) {
    return { type: "unknown", label: "???", token: "?", spoiler_masked: true };
  }
  return { type: "npc", label: "NPC", token: "N" };
};

const isSurpriseKeyword = (value: string): boolean => /ROCKET|HIDEOUT|TRAP|RIVAL|SUSPICIOUS/.test(value);

const isPcScript = (value: string): boolean =>
  value.endsWith("PC") ||
  value.includes("PCSCRIPT") ||
  value.includes("PLAYERSPC") ||
  value.includes("POKECENTERPLAYERSPC") ||
  value.includes("PCTEXT") ||
  value.includes("PCTURNON") ||
  value.includes("PCASKWHATDO");

const isTrashcanScript = (value: string): boolean => value.includes("TRASHCAN") || value.includes("TRASHCAN2");

const classifyBgHotspot = (
  script: string
): Pick<McpMapHotspot, "type" | "label" | "token" | "spoiler_masked"> => {
  if (isPcScript(script)) {
    return {
      type: "utility",
      label: "PC",
      token: "P",
    };
  }
  if (script.includes("HEALINGMACHINE")) {
    return {
      type: "heal",
      label: "Healing machine",
      token: "H",
    };
  }
  if (script.includes("POKECENTER")) {
    return {
      type: "sign",
      label: "Pokecenter sign",
      token: "S",
    };
  }
  if (isTrashcanScript(script)) {
    return {
      type: "sign",
      label: "Trash can",
      token: "T",
    };
  }
  if (script.includes("BOOKSHELF")) {
    return {
      type: "sign",
      label: "Bookshelf",
      token: "B",
    };
  }
  if (script.includes("WINDOW")) {
    return {
      type: "landmark",
      label: "Window",
      token: "W",
    };
  }
  if (script.includes("TRAVELTIP")) {
    return {
      type: "sign",
      label: "Travel tip",
      token: "T",
    };
  }
  if (script.includes("GYM")) {
    return {
      type: "gym",
      label: "Gym marker",
      token: "G",
    };
  }
  if (script.includes("MART")) {
    return {
      type: "shop",
      label: "Shop sign",
      token: "$",
    };
  }
  if (isSurpriseKeyword(script)) {
    return {
      type: "unknown",
      label: "???",
      token: "?",
      spoiler_masked: true,
    };
  }
  return {
    type: "sign",
    label: "Sign",
    token: "S",
  };
};

const classifyDestination = (
  targetName: string | null,
  targetConstant: string | null
): Pick<McpMapHotspot, "type" | "label" | "token" | "spoiler_masked"> => {
  const normalized = `${normalizeName(targetName)} ${normalizeName(targetConstant)}`.trim();
  if (normalized.includes("POKECENTER")) {
    return { type: "warp", label: "Warp: Pokecenter", token: "D" };
  }
  if (normalized.includes("MART") || normalized.includes("DEPTSTORE") || normalized.includes("SHOP")) {
    return { type: "shop", label: "Shop", token: "$" };
  }
  if (normalized.includes("GYM")) {
    return { type: "gym", label: "Gym", token: "G" };
  }
  if (isSurpriseKeyword(normalized)) {
    return { type: "unknown", label: "???", token: "?", spoiler_masked: true };
  }
  return {
    type: "warp",
    label: targetName ? `Warp: ${compactLabel(targetName, "Destination")}` : "Warp",
    token: "D",
  };
};

const buildWarpHotspot = (
  warp: McpMapWarpSnapshot,
  index: number
): McpMapHotspot => {
  const classified = classifyDestination(warp.target.map_name, warp.target.map_constant);
  return {
    id: `warp-${warp.index ?? index + 1}`,
    coords: warp.coords,
    visible: true,
    interactable: true,
    ...classified,
  };
};

const promoteStoryWarpHotspot = (
  hotspot: McpMapHotspot,
  currentMap: string | null | undefined,
  eventFlags: EventFlags
): McpMapHotspot => {
  const current = normalizeName(currentMap);
  const target = normalizeName(hotspot.label);
  const clearBellReady = Boolean(eventFlags?.EVENT_GOT_CLEAR_BELL);
  const passageGranted = Boolean(eventFlags?.EVENT_KOJI_ALLOWS_YOU_PASSAGE_TO_TIN_TOWER);
  const foughtSuicune = Boolean(eventFlags?.EVENT_FOUGHT_SUICUNE);

  if (
    current === "ECRUTEAKCITY" &&
    target.includes("WISETRIOSROOM") &&
    clearBellReady &&
    !passageGranted &&
    !foughtSuicune
  ) {
    return {
      ...hotspot,
      type: "objective",
      label: "Wise Trio test",
      token: "!",
    };
  }
  return hotspot;
};

const getBgEvents = (
  overworld: OverworldMapInfoSource,
  dataLoader: DataLoaderLike | null | undefined,
  map: string | null | undefined
): unknown[] => {
  const liveEvents = overworld._map_events?.bg_events;
  if (Array.isArray(liveEvents)) {
    return liveEvents;
  }
  if (!dataLoader?.map_events || !map) {
    return [];
  }
  const direct = dataLoader.map_events.get(map)?.bg_events;
  return Array.isArray(direct) ? direct : [];
};

const buildBgHotspots = (
  bgEvents: unknown[],
  stride: number,
  offset: number,
  eventFlags: EventFlags
): McpMapHotspot[] => {
  return bgEvents.flatMap((rawEvent, index) => {
    const event = typeof rawEvent === "object" && rawEvent !== null
      ? (rawEvent as Record<string, unknown>)
      : {};
    const eventType = normalizeName(toStringOrNull(event.event_type));
    const script = normalizeName(toStringOrNull(event.script));
    if (eventType.includes("ITEM")) {
      return [];
    }
    const x = (toNumberOrNull(event.x) ?? 0) * stride + offset;
    const y = (toNumberOrNull(event.y) ?? 0) * stride + offset;
    const classified = classifyBgHotspot(script);
    if (classified.spoiler_masked && Object.keys(eventFlags ?? {}).length === 0) {
      // Keep hidden event hotspots visible-but-masked even without event flags.
    }
    return [{
      id: `bg-${index + 1}`,
      type: classified.type,
      label: classified.label,
      coords: { x, y },
      visible: true,
      interactable: true,
      token: classified.token,
      spoiler_masked: classified.spoiler_masked || undefined,
    }];
  });
};

const buildNpcHotspots = (npcs: unknown[]): McpMapHotspot[] => {
  return npcs.flatMap((rawNpc, index) => {
    const npc = typeof rawNpc === "object" && rawNpc !== null
      ? (rawNpc as Record<string, unknown>)
      : {};
    const x = toNumberOrNull(npc.x);
    const y = toNumberOrNull(npc.y);
    if (x === null || y === null) {
      return [];
    }
    const event = typeof npc.event === "object" && npc.event !== null
      ? (npc.event as Record<string, unknown>)
      : {};
    const sourceTokens = [
      toStringOrNull(event.script),
      toStringOrNull(event.object_identifier),
      toStringOrNull(event.label),
      toStringOrNull(event.object_type),
      toStringOrNull(npc.objectId),
      toStringOrNull(npc.constantId),
      toStringOrNull(npc.spriteId),
    ]
      .filter(Boolean)
      .map((entry) => normalizeName(entry));
    // ASM-faithful hotspot labels for Elm's Lab come from
    // `pokecrystal_disassembly/maps/ElmsLab.asm`, especially
    // `ProfElmScript`, `CyndaquilPokeBallScript`,
    // `TotodilePokeBallScript`, and `ChikoritaPokeBallScript`.
    const classified = classifyNpcHotspot(sourceTokens);
    return [{
      id: `npc-${toNumberOrNull(npc.objectIndex) ?? index + 1}`,
      type: classified.type,
      label: classified.label,
      coords: { x, y },
      visible: true,
      interactable: true,
      token: classified.token,
      spoiler_masked: classified.spoiler_masked || undefined,
    }];
  });
};

const buildBlueprintNpcHotspots = (
  overworld: OverworldMapInfoSource,
  stride: number,
  offset: number,
  eventFlags: EventFlags
): McpMapHotspot[] => {
  const mapName = String(overworld.current_map_name ?? "");
  const blueprint = overworld._npc_blueprints?.get(mapName);
  if (!blueprint) {
    return [];
  }
  const byIndex = new Map<number, McpMapHotspot>();

  for (const [, entry] of blueprint.entries()) {
    const [rawEvent, rawIndex] = Array.isArray(entry) ? entry : [null, null];
    const event = typeof rawEvent === "object" && rawEvent !== null
      ? (rawEvent as Record<string, unknown>)
      : null;
    const objectIndex = typeof rawIndex === "number" && Number.isFinite(rawIndex) ? rawIndex : null;
    if (!event || objectIndex === null || byIndex.has(objectIndex)) {
      continue;
    }
    const eventFlag = toStringOrNull(event.event_flag);
    if (eventFlag && eventFlags?.[eventFlag]) {
      continue;
    }
    const x = (toNumberOrNull(event.x) ?? 0) * stride + offset;
    const y = (toNumberOrNull(event.y) ?? 0) * stride + offset;
    const sourceTokens = [
      toStringOrNull(event.script),
      toStringOrNull(event.object_identifier),
      toStringOrNull(event.label),
      toStringOrNull(event.object_type),
      toStringOrNull(event.sprite),
    ]
      .filter(Boolean)
      .map((entry) => normalizeName(entry));
    const classified = classifyNpcHotspot(sourceTokens);
    byIndex.set(objectIndex, {
      id: `npc-${objectIndex}`,
      type: classified.type,
      label: classified.label,
      coords: { x, y },
      visible: true,
      interactable: true,
      token: classified.token,
      spoiler_masked: classified.spoiler_masked || undefined,
    });
  }

  return [...byIndex.values()].sort((a, b) => a.id.localeCompare(b.id));
};

const APPROACH_DIRECTIONS: ReadonlyArray<{
  standingFacing: "up" | "down" | "left" | "right";
  dx: number;
  dy: number;
}> = [
  { standingFacing: "down", dx: 0, dy: -1 },
  { standingFacing: "up", dx: 0, dy: 1 },
  { standingFacing: "right", dx: -1, dy: 0 },
  { standingFacing: "left", dx: 1, dy: 0 },
];

const isApproachTilePassable = (
  overworld: OverworldMapInfoSource,
  x: number,
  y: number,
  facing: "up" | "down" | "left" | "right"
): boolean => {
  if (typeof overworld._npc_occupying_subtile === "function" && overworld._npc_occupying_subtile(x, y)) {
    return false;
  }
  if (!overworld.map || !overworld.tileset) {
    return true;
  }
  const permission = getCoordCollision(overworld.map, overworld.tileset, x, y);
  return isPermissionPassable(
    permission,
    FacingDirection.fromString(facing),
    overworld.player_state ?? PlayerState.NORMAL
  );
};

const DIRECTION_VECTORS: Record<"up" | "down" | "left" | "right", { dx: number; dy: number }> = {
  up: { dx: 0, dy: -1 },
  down: { dx: 0, dy: 1 },
  left: { dx: -1, dy: 0 },
  right: { dx: 1, dy: 0 },
};

const buildCounterApproachTiles = (
  hotspot: McpMapHotspot,
  stride: number,
  overworld: OverworldMapInfoSource
): McpMapApproachTile[] => {
  if (!overworld.map || !overworld.tileset) {
    return [];
  }

  const searchRadius = Math.max(1, stride * 2);
  const approaches: McpMapApproachTile[] = [];
  for (let x = hotspot.coords.x - searchRadius; x <= hotspot.coords.x + searchRadius; x += 1) {
    for (let y = hotspot.coords.y - searchRadius; y <= hotspot.coords.y + searchRadius; y += 1) {
      if (x === hotspot.coords.x && y === hotspot.coords.y) {
        continue;
      }
      for (const [facing, vector] of Object.entries(DIRECTION_VECTORS) as Array<
        ["up" | "down" | "left" | "right", { dx: number; dy: number }]
      >) {
        const rawFacingX = x + vector.dx * stride;
        const rawFacingY = y + vector.dy * stride;
        const [adjustedX, adjustedY] = adjustCounterTile(
          overworld.map,
          overworld.tileset,
          x,
          y,
          rawFacingX,
          rawFacingY,
          stride
        );
        if (adjustedX === rawFacingX && adjustedY === rawFacingY) {
          continue;
        }
        if (adjustedX !== hotspot.coords.x || adjustedY !== hotspot.coords.y) {
          continue;
        }
        if (!isApproachTilePassable(overworld, x, y, facing)) {
          continue;
        }
        approaches.push({ coords: { x, y }, facing });
      }
    }
  }
  return approaches;
};

const dedupeApproachTiles = (approachTiles: McpMapApproachTile[]): McpMapApproachTile[] => {
  const byKey = new Map<string, McpMapApproachTile>();
  for (const approachTile of approachTiles) {
    byKey.set(
      `${approachTile.coords.x}:${approachTile.coords.y}:${approachTile.facing}`,
      approachTile
    );
  }
  return [...byKey.values()].sort(
    (left, right) =>
      left.coords.y - right.coords.y ||
      left.coords.x - right.coords.x ||
      left.facing.localeCompare(right.facing)
  );
};

const buildApproachTiles = (
  hotspot: McpMapHotspot,
  stride: number,
  overworld: OverworldMapInfoSource
): McpMapApproachTile[] | undefined => {
  if (!hotspot.interactable || hotspot.type === "warp") {
    return undefined;
  }
  const approachTiles = APPROACH_DIRECTIONS.flatMap(({ standingFacing, dx, dy }) => {
    const x = hotspot.coords.x + dx * stride;
    const y = hotspot.coords.y + dy * stride;
    if (!isApproachTilePassable(overworld, x, y, standingFacing)) {
      return [];
    }
    return [{ coords: { x, y }, facing: standingFacing }];
  });
  const counterApproachTiles = buildCounterApproachTiles(hotspot, stride, overworld);
  const allApproachTiles = dedupeApproachTiles([...approachTiles, ...counterApproachTiles]);
  return allApproachTiles.length ? allApproachTiles : undefined;
};

const hotspotPriority = (hotspot: McpMapHotspot): number => {
  switch (hotspot.type) {
    case "objective":
      return 0;
    case "npc":
      return 1;
    case "utility":
      return 2;
    case "heal":
    case "shop":
    case "gym":
      return 3;
    case "sign":
    case "landmark":
      return 4;
    case "trigger":
      return 5;
    case "warp":
      return 6;
    case "hazard":
      return 7;
    case "unknown":
    default:
      return 8;
  }
};

const hotspotSpecificity = (hotspot: McpMapHotspot): number => {
  const genericLabelPenalty =
    hotspot.label === "NPC" || hotspot.label === "Sign" || hotspot.label === "Poke Ball" || hotspot.label === "Warp"
      ? 0
      : 1;
  return genericLabelPenalty * 100 + Math.min(hotspot.label.length, 99);
};

const compareHotspots = (left: McpMapHotspot, right: McpMapHotspot): number =>
  hotspotPriority(left) - hotspotPriority(right) ||
  hotspotSpecificity(right) - hotspotSpecificity(left) ||
  left.coords.y - right.coords.y ||
  left.coords.x - right.coords.x ||
  left.id.localeCompare(right.id);

const dedupeHotspots = (hotspots: McpMapHotspot[]): McpMapHotspot[] => {
  const byCoords = new Map<string, McpMapHotspot>();
  for (const hotspot of hotspots) {
    const key = `${hotspot.coords.x}:${hotspot.coords.y}`;
    const existing = byCoords.get(key);
    if (!existing || compareHotspots(hotspot, existing) < 0) {
      byCoords.set(key, hotspot);
    }
  }
  return [...byCoords.values()];
};

const hotspotDistance = (hotspot: McpMapHotspot, playerCoords?: { x: number; y: number } | null): number => {
  if (!playerCoords) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.abs(hotspot.coords.x - playerCoords.x) + Math.abs(hotspot.coords.y - playerCoords.y);
};

const toWarpSnapshot = (rawWarp: unknown, stride: number, offset: number): McpMapWarpSnapshot => {
  const warp = typeof rawWarp === "object" && rawWarp !== null
    ? (rawWarp as Record<string, unknown>)
    : {};
  const tileX = toNumberOrNull(warp.x) ?? 0;
  const tileY = toNumberOrNull(warp.y) ?? 0;
  return {
    index: toNumberOrNull(warp.index),
    coords: {
      x: tileX * stride + offset,
      y: tileY * stride + offset,
    },
    target: {
      map_constant: toStringOrNull(warp.target_map_constant),
      map_name: toStringOrNull(warp.target_map),
      warp_id: toNumberOrNull(warp.target_warp_id),
    },
  };
};

export const buildMapInfoSnapshot = (params: {
  map: string | null | undefined;
  mapGroup: number | null | undefined;
  mapNumber: number | null | undefined;
  overworld: OverworldMapInfoSource;
  playerCoords?: { x: number; y: number } | null;
  facing?: "up" | "down" | "left" | "right";
  dataLoader?: DataLoaderLike | null;
  eventFlags?: EventFlags;
}): McpMapInfoSnapshot => {
  const { map, mapGroup, mapNumber, overworld, playerCoords, facing, dataLoader, eventFlags } = params;
  const group = toNumberOrNull(mapGroup);
  const number = toNumberOrNull(mapNumber);
  const mapId = group === null || number === null ? null : `${group}:${number}`;

  const stride =
    typeof overworld.TILES_PER_COLLISION === "number" && Number.isFinite(overworld.TILES_PER_COLLISION)
      ? overworld.TILES_PER_COLLISION
      : 2;
  const offset = Math.max(0, stride - 1);

  const rawWarps = Array.isArray(overworld._map_events?.warps) ? overworld._map_events?.warps : [];
  const warps = rawWarps.map((rawWarp) => toWarpSnapshot(rawWarp, stride, offset));
  const warpHotspots = warps.map((warp, index) =>
    promoteStoryWarpHotspot(buildWarpHotspot(warp, index), map, eventFlags)
  );
  const liveNpcHotspots = buildNpcHotspots(Array.isArray(overworld.npcs) ? overworld.npcs : []);
  const fallbackNpcHotspots = buildBlueprintNpcHotspots(overworld, stride, offset, eventFlags);
  const npcHotspots = [...liveNpcHotspots];
  const npcHotspotKeys = new Set(
    liveNpcHotspots.map((hotspot) => `${hotspot.coords.x},${hotspot.coords.y}:${hotspot.label}`)
  );
  for (const hotspot of fallbackNpcHotspots) {
    const key = `${hotspot.coords.x},${hotspot.coords.y}:${hotspot.label}`;
    if (!npcHotspotKeys.has(key)) {
      npcHotspots.push(hotspot);
      npcHotspotKeys.add(key);
    }
  }
  const bgHotspots = buildBgHotspots(getBgEvents(overworld, dataLoader, map), stride, offset, eventFlags);
  const hotspots = dedupeHotspots([...warpHotspots, ...bgHotspots, ...npcHotspots])
    .map((hotspot) => ({
      ...hotspot,
      approach_tiles: buildApproachTiles(hotspot, stride, overworld),
    }))
    .sort((a, b) =>
      compareHotspots(a, b) ||
      hotspotDistance(a, playerCoords) - hotspotDistance(b, playerCoords) ||
      a.id.localeCompare(b.id)
    );

  return {
    map: map ?? null,
    map_id: mapId,
    coord_stride: stride,
    warps,
    player: playerCoords
      ? {
          coords: { ...playerCoords },
          facing: facing ?? undefined,
        }
      : undefined,
    hotspots,
  };
};
