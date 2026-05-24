import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { Terrain, describeCollision, resolveCollisionValue } from "./collision-data";
import {
  CollisionSample,
  isPermissionPassable,
  sampleCollision,
} from "./collision-rules";
import { OverworldMap } from "./overworld-map";
import type { OverworldTilesetLike } from "./tileset-types";
import type { VRAM } from "@pokecrystal/core/core/memory/vram";
import { METATILE_SIZE, METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile/constants";
import { create_map_surface } from "./map-geometry";
import { scaleTileCoord } from "./tile-coords";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";

export type RouteRenderDirection = "up" | "down" | "left" | "right";

export type RouteRenderCoords = {
  x: number;
  y: number;
};

export type RouteRenderWarp = {
  index: number | null;
  coords: RouteRenderCoords;
  target: {
    map_constant: string | null;
    map_name: string | null;
    warp_id: number | null;
  };
};

export type RouteRenderHotspot = {
  id: string;
  type: string;
  label: string;
  coords: RouteRenderCoords;
  visible: boolean;
  interactable: boolean;
  token?: string;
  spoiler_masked?: boolean;
  approach_tiles?: Array<{ coords: RouteRenderCoords; facing: RouteRenderDirection }>;
};

export type RouteRenderCell = {
  x: number;
  y: number;
  token: string;
  base: string;
  terrain: "land" | "water" | "wall" | "missing";
  passable: boolean;
  permission: number | null;
  overlay?: "player" | "warp" | "hotspot" | "trigger" | "ledge";
  label?: string;
};

export type RouteRenderSnapshot = {
  available: boolean;
  reason?: string;
  map: string | null;
  map_id: string | null;
  coord_stride?: number;
  size?: { width: number; height: number };
  player?: {
    coords: RouteRenderCoords;
    facing?: RouteRenderDirection;
  };
  grid?: {
    origin: RouteRenderCoords;
    rows: string[];
    cells?: RouteRenderCell[][];
  };
  legend: Array<{ token: string; label: string }>;
  warps: RouteRenderWarp[];
  hotspots: RouteRenderHotspot[];
};

export type RouteRenderDetail = "compact" | "full";

export type RouteRenderMapEvent = {
  x: number;
  y: number;
  scene_id?: string;
  script_name?: string;
};

export type RouteRenderMapEvents = {
  coord_events?: RouteRenderMapEvent[];
};

export type RouteRenderDataLoader = {
  get_script_event_flags?: (scriptName: string) => string[];
};

export type BuildRouteRenderSnapshotParams = {
  map: string | null | undefined;
  mapId?: string | null;
  coordStride?: number | null;
  player?: { coords: RouteRenderCoords; facing?: RouteRenderDirection } | null;
  mapData: OverworldMap;
  tileset: OverworldTilesetLike;
  playerState?: PlayerState;
  warps?: RouteRenderWarp[];
  hotspots?: RouteRenderHotspot[];
  mapEvents?: RouteRenderMapEvents | null;
  currentScene?: string | null;
  eventFlags?: Record<string, boolean | undefined> | null;
  dataLoader?: RouteRenderDataLoader | null;
  detail?: RouteRenderDetail;
};

export type RouteRenderImageOptions = {
  cellSize?: number;
};

export type RenderRouteRenderTileSurfaceParams = {
  snapshot: RouteRenderSnapshot;
  mapData: OverworldMap;
  tileset: OverworldTilesetLike;
  vram?: VRAM | null;
};

const PASSABLE_DIRECTIONS: readonly FacingDirection[] = [
  FacingDirection.UP,
  FacingDirection.DOWN,
  FacingDirection.LEFT,
  FacingDirection.RIGHT,
];

const GRASS_COLLISION_VALUES = new Set(
  [
    "CUT_08",
    "TALL_GRASS",
    "TALL_GRASS_10",
    "LONG_GRASS",
    "LONG_GRASS_1C",
    "CUT_28",
    "GRASS_48",
    "GRASS_49",
    "GRASS_4A",
    "GRASS_4B",
    "GRASS_4C",
  ].map((token) => resolveCollisionValue(token))
);

const COLLISION_TOKENS = new Map<number, string>();
const registerCollisionToken = (name: string, token: string): void => {
  COLLISION_TOKENS.set(resolveCollisionValue(name), token);
};

registerCollisionToken("CUT_TREE", "C");
registerCollisionToken("HEADBUTT_TREE", "H");
registerCollisionToken("MART_SHELF", "$");
registerCollisionToken("COUNTER", "T");
registerCollisionToken("COUNTER_98", "T");
registerCollisionToken("BOOKSHELF", "B");
registerCollisionToken("PC", "P");
registerCollisionToken("RADIO", "!");
registerCollisionToken("TOWN_MAP", "!");
registerCollisionToken("TV", "!");
registerCollisionToken("WINDOW", "W");
registerCollisionToken("INCENSE_BURNER", "!");
registerCollisionToken("WATERFALL", "=");
registerCollisionToken("WATERFALL_LEFT", "=");
registerCollisionToken("WATERFALL_RIGHT", "=");
registerCollisionToken("WATERFALL_UP", "=");
registerCollisionToken("WALK_RIGHT", ">");
registerCollisionToken("WALK_LEFT", "<");
registerCollisionToken("WALK_UP", "^");
registerCollisionToken("WALK_DOWN", "v");
registerCollisionToken("WALK_RIGHT_ALT", ">");
registerCollisionToken("WALK_LEFT_ALT", "<");
registerCollisionToken("WALK_UP_ALT", "^");
registerCollisionToken("WALK_DOWN_ALT", "v");
registerCollisionToken("CURRENT_RIGHT", ">");
registerCollisionToken("CURRENT_LEFT", "<");
registerCollisionToken("CURRENT_UP", "^");
registerCollisionToken("CURRENT_DOWN", "v");

const SCRIPT_TOKENS: Record<string, string> = {
  PCSCRIPT: "P",
  MERCHANDISESHELFSCRIPT: "$",
  MARTSHELF: "$",
};

const LEDGE_FACE_TOKENS = new Map<number, { token: string; dx: number; dy: number }>();
const registerLedgeFace = (names: string[], token: string, dx: number, dy: number): void => {
  for (const name of names) {
    LEDGE_FACE_TOKENS.set(resolveCollisionValue(name), { token, dx, dy });
  }
};

registerLedgeFace(["HOP_DOWN", "HOP_DOWN_LEFT", "HOP_DOWN_RIGHT"], "d", 0, 1);
registerLedgeFace(["HOP_UP", "HOP_UP_LEFT", "HOP_UP_RIGHT"], "u", 0, -1);
registerLedgeFace(["HOP_LEFT"], "l", -1, 0);
registerLedgeFace(["HOP_RIGHT"], "r", 1, 0);

const TOKEN_LABELS = new Map<string, string>([
  ["@", "Player"],
  ["N", "Person"],
  ["D", "Door/Warp"],
  ["H", "Healer/Headbutt tree"],
  ["P", "PC/Utility"],
  ["G", "Gym"],
  ["$", "Shop/Shelf"],
  ["S", "Sign"],
  ["W", "Window"],
  ["T", "Counter/Trash"],
  ["B", "Berry tree/Bookshelf"],
  ["I", "Item ball"],
  ["X", "Trainer/Hazard"],
  ["?", "Unknown"],
  ["!", "Objective/Talkable"],
  ["*", "Trigger"],
  ["C", "Cut tree"],
  ["=", "Waterfall/current"],
  ["~", "Water"],
  ["\"", "Grass"],
  ["u", "Ledge pass up"],
  ["d", "Ledge pass down"],
  ["l", "Ledge pass left"],
  ["r", "Ledge pass right"],
  ["#", "Blocked"],
  [".", "Floor"],
  ["x", "Missing"],
  ["^", "Forced up"],
  ["v", "Forced down"],
  ["<", "Forced left"],
  [">", "Forced right"],
]);

const normalizeOptionalString = (value: unknown): string | null =>
  typeof value === "string" && value.trim().length > 0 ? value.trim() : null;

const finiteCoord = (coords: RouteRenderCoords | null | undefined): RouteRenderCoords | null => {
  if (
    !coords ||
    typeof coords.x !== "number" ||
    !Number.isFinite(coords.x) ||
    typeof coords.y !== "number" ||
    !Number.isFinite(coords.y)
  ) {
    return null;
  }
  return { x: Math.trunc(coords.x), y: Math.trunc(coords.y) };
};

const hotspotToken = (hotspot: RouteRenderHotspot): string => {
  const explicit = normalizeOptionalString(hotspot.token);
  if (explicit) {
    return explicit.slice(0, 1);
  }
  switch (hotspot.type) {
    case "npc":
      return "N";
    case "heal":
      return "H";
    case "shop":
      return "$";
    case "gym":
      return "G";
    case "sign":
      return "S";
    case "utility":
      return "P";
    case "hazard":
      return "X";
    case "objective":
      return "!";
    case "unknown":
      return "?";
    case "trigger":
      return "*";
    case "warp":
      return "D";
    default:
      return "?";
  }
};

const terrainLabel = (terrain: Terrain | "missing"): RouteRenderCell["terrain"] => {
  if (terrain === "missing") {
    return "missing";
  }
  return terrain;
};

const classifyCollisionCell = (
  sample: CollisionSample | null,
  playerState: PlayerState
): Pick<RouteRenderCell, "token" | "terrain" | "passable" | "permission"> => {
  if (!sample) {
    return { token: "x", terrain: "missing", passable: false, permission: null };
  }
  const attrs = describeCollision(sample.permission);
  const scriptToken = sample.stdScript
    ? SCRIPT_TOKENS[sample.stdScript.trim().toUpperCase()]
    : undefined;
  const token = COLLISION_TOKENS.get(sample.permission) ?? scriptToken;
  const passable = PASSABLE_DIRECTIONS.some((direction) =>
    isPermissionPassable(sample.permission, direction, playerState)
  );
  if (token) {
    return {
      token,
      terrain: terrainLabel(attrs.terrain),
      passable,
      permission: sample.permission,
    };
  }
  if (attrs.talk) {
    return {
      token: "!",
      terrain: terrainLabel(attrs.terrain),
      passable,
      permission: sample.permission,
    };
  }
  if (attrs.terrain === Terrain.WATER) {
    return { token: "~", terrain: "water", passable, permission: sample.permission };
  }
  if (attrs.terrain === Terrain.LAND && GRASS_COLLISION_VALUES.has(sample.permission)) {
    return { token: "\"", terrain: "land", passable, permission: sample.permission };
  }
  if (!passable) {
    return {
      token: "#",
      terrain: terrainLabel(attrs.terrain),
      passable,
      permission: sample.permission,
    };
  }
  return {
    token: ".",
    terrain: terrainLabel(attrs.terrain),
    passable,
    permission: sample.permission,
  };
};

const activeCoordEvents = (
  events: readonly RouteRenderMapEvent[],
  params: Pick<BuildRouteRenderSnapshotParams, "currentScene" | "eventFlags" | "dataLoader">
): RouteRenderMapEvent[] => {
  const currentScene = String(params.currentScene ?? "");
  const eventFlags = params.eventFlags ?? null;
  return events.filter((event) => {
    const sceneId = String(event.scene_id ?? "").trim();
    const scriptName = String(event.script_name ?? "").trim();
    if (sceneId && sceneId !== currentScene) {
      return false;
    }
    if (!sceneId && scriptName && eventFlags && params.dataLoader?.get_script_event_flags) {
      const flags = params.dataLoader.get_script_event_flags(scriptName);
      if (flags.some((flag) => eventFlags[flag])) {
        return false;
      }
    }
    return true;
  });
};

const writeOverlay = (
  cells: RouteRenderCell[][],
  coords: RouteRenderCoords,
  token: string,
  overlay: NonNullable<RouteRenderCell["overlay"]>,
  label?: string
): void => {
  const x = Math.trunc(coords.x);
  const y = Math.trunc(coords.y);
  if (y < 0 || y >= cells.length || x < 0 || x >= (cells[y]?.length ?? 0)) {
    return;
  }
  const cell = cells[y][x];
  cell.token = token;
  cell.overlay = overlay;
  if (label) {
    cell.label = label;
  }
};

const buildLegend = (rows: string[]): Array<{ token: string; label: string }> => {
  const tokens = new Set<string>();
  for (const row of rows) {
    for (const token of row) {
      if (token.trim()) {
        tokens.add(token);
      }
    }
  }
  return [...TOKEN_LABELS.entries()]
    .filter(([token]) => tokens.has(token))
    .map(([token, label]) => ({ token, label }));
};

export const buildUnavailableRouteRenderSnapshot = (
  reason: string,
  map: string | null = null,
  mapId: string | null = null
): RouteRenderSnapshot => ({
  available: false,
  reason,
  map,
  map_id: mapId,
  legend: [],
  warps: [],
  hotspots: [],
});

export const buildRouteRenderSnapshot = (
  params: BuildRouteRenderSnapshotParams
): RouteRenderSnapshot => {
  const stride = Math.max(1, Math.trunc(params.coordStride ?? 1));
  const width = Math.max(0, Math.trunc(params.mapData.width * METATILE_WIDTH));
  const height = Math.max(0, Math.trunc(params.mapData.height * METATILE_WIDTH));
  const playerState = params.playerState ?? PlayerState.NORMAL;
  const ledgeMarks: Array<{ x: number; y: number; token: string }> = [];

  const cells = Array.from({ length: height }, (_, y) =>
    Array.from({ length: width }, (_, x): RouteRenderCell => {
      let sample: CollisionSample | null = null;
      try {
        sample = sampleCollision(params.mapData, params.tileset, x, y);
      } catch {
        sample = null;
      }
      if (sample) {
        const ledge = LEDGE_FACE_TOKENS.get(sample.permission);
        if (ledge) {
          ledgeMarks.push({ x: x + ledge.dx, y: y + ledge.dy, token: ledge.token });
        }
      }
      const classified = classifyCollisionCell(sample, playerState);
      return {
        x,
        y,
        token: classified.token,
        base: classified.token,
        terrain: classified.terrain,
        passable: classified.passable,
        permission: classified.permission,
      };
    })
  );

  for (const mark of ledgeMarks) {
    const row = cells[mark.y];
    const cell = row?.[mark.x];
    if (cell && cell.token === "#") {
      cell.token = mark.token;
      cell.overlay = "ledge";
      cell.label = TOKEN_LABELS.get(mark.token);
    }
  }

  for (const event of activeCoordEvents(params.mapEvents?.coord_events ?? [], params)) {
    writeOverlay(
      cells,
      { x: scaleTileCoord(event.x, stride), y: scaleTileCoord(event.y, stride) },
      "*",
      "trigger",
      event.script_name || "Coord event"
    );
  }

  const warps = (params.warps ?? []).map((warp) => ({
    index: warp.index ?? null,
    coords: { ...warp.coords },
    target: { ...warp.target },
  }));
  for (const warp of warps) {
    writeOverlay(cells, warp.coords, "D", "warp", warp.target.map_name ?? "Warp");
  }

  const hotspots = (params.hotspots ?? [])
    .filter((hotspot) => hotspot.visible !== false)
    .map((hotspot) => ({
      ...hotspot,
      coords: { ...hotspot.coords },
      approach_tiles: hotspot.approach_tiles?.map((approach) => ({
        coords: { ...approach.coords },
        facing: approach.facing,
      })),
    }));
  for (const hotspot of hotspots) {
    writeOverlay(cells, hotspot.coords, hotspotToken(hotspot), "hotspot", hotspot.label);
  }

  const playerCoords = finiteCoord(params.player?.coords);
  if (playerCoords) {
    writeOverlay(cells, playerCoords, "@", "player", "Player");
  }

  const rows = cells.map((row) => row.map((cell) => cell.token).join(""));

  return {
    available: true,
    map: params.map ?? null,
    map_id: params.mapId ?? null,
    coord_stride: stride,
    size: { width, height },
    player: playerCoords
      ? {
          coords: playerCoords,
          facing: params.player?.facing,
        }
      : undefined,
    grid: {
      origin: { x: 0, y: 0 },
      rows,
      cells: params.detail === "full" ? cells : undefined,
    },
    legend: buildLegend(rows),
    warps,
    hotspots,
  };
};

const tokenColors: Record<string, [number, number, number, number]> = {
  "@": [32, 185, 99, 255],
  N: [69, 128, 220, 255],
  D: [155, 96, 202, 255],
  H: [70, 196, 154, 255],
  P: [191, 103, 206, 255],
  G: [237, 97, 80, 255],
  "$": [230, 174, 68, 255],
  S: [230, 174, 68, 255],
  W: [93, 185, 202, 255],
  T: [123, 110, 95, 255],
  B: [121, 156, 68, 255],
  I: [244, 202, 72, 255],
  X: [220, 68, 68, 255],
  "?": [98, 98, 111, 255],
  "!": [245, 183, 66, 255],
  "*": [245, 120, 66, 255],
  C: [121, 156, 68, 255],
  "=": [68, 130, 210, 255],
  "~": [77, 145, 214, 255],
  "\"": [92, 164, 87, 255],
  u: [205, 126, 72, 255],
  d: [205, 126, 72, 255],
  l: [205, 126, 72, 255],
  r: [205, 126, 72, 255],
  "#": [48, 52, 58, 255],
  ".": [232, 230, 216, 255],
  x: [145, 50, 50, 255],
  "^": [92, 188, 202, 255],
  v: [92, 188, 202, 255],
  "<": [92, 188, 202, 255],
  ">": [92, 188, 202, 255],
};

const glyphs: Record<string, string[]> = {
  "@": ["111", "101", "101", "100", "111"],
  N: ["101", "111", "111", "111", "101"],
  D: ["110", "101", "101", "101", "110"],
  H: ["101", "101", "111", "101", "101"],
  P: ["110", "101", "110", "100", "100"],
  G: ["111", "100", "101", "101", "111"],
  "$": ["111", "101", "111", "101", "111"],
  S: ["111", "100", "111", "001", "111"],
  W: ["101", "101", "111", "111", "101"],
  T: ["111", "010", "010", "010", "010"],
  B: ["110", "101", "110", "101", "110"],
  I: ["111", "010", "010", "010", "111"],
  X: ["101", "101", "010", "101", "101"],
  "?": ["111", "001", "011", "000", "010"],
  "!": ["010", "010", "010", "000", "010"],
  "*": ["101", "010", "111", "010", "101"],
  C: ["111", "100", "100", "100", "111"],
  "=": ["000", "111", "000", "111", "000"],
  "~": ["000", "101", "010", "101", "000"],
  "\"": ["101", "101", "000", "000", "000"],
  u: ["010", "111", "010", "010", "010"],
  d: ["010", "010", "010", "111", "010"],
  l: ["001", "010", "100", "010", "001"],
  r: ["100", "010", "001", "010", "100"],
  "#": ["101", "111", "101", "111", "101"],
  ".": ["000", "000", "000", "000", "010"],
  x: ["101", "010", "010", "010", "101"],
  "^": ["010", "101", "000", "000", "000"],
  v: ["000", "000", "000", "101", "010"],
  "<": ["001", "010", "100", "010", "001"],
  ">": ["100", "010", "001", "010", "100"],
};

const drawGlyph = (
  surface: InstanceType<typeof gameEngine.Surface>,
  token: string,
  x: number,
  y: number,
  cellSize: number,
  color: [number, number, number, number]
): void => {
  const glyph = glyphs[token];
  if (!glyph || cellSize < 6) {
    return;
  }
  const pixel = Math.max(1, Math.floor(cellSize / 6));
  const glyphWidth = glyph[0]?.length ?? 0;
  const glyphHeight = glyph.length;
  const startX = x + Math.floor((cellSize - glyphWidth * pixel) / 2);
  const startY = y + Math.floor((cellSize - glyphHeight * pixel) / 2);
  for (let row = 0; row < glyphHeight; row += 1) {
    for (let col = 0; col < glyphWidth; col += 1) {
      if (glyph[row]?.[col] !== "1") {
        continue;
      }
      surface.fill(color, {
        x: startX + col * pixel,
        y: startY + row * pixel,
        width: pixel,
        height: pixel,
      });
    }
  }
};

const clampCellSize = (value: number | undefined): number => {
  const parsed = Math.trunc(value ?? 8);
  if (!Number.isFinite(parsed)) {
    return 8;
  }
  return Math.max(4, Math.min(16, parsed));
};

const facingToken = (facing: RouteRenderDirection | undefined): string =>
  facing === "up" ? "^" : facing === "down" ? "v" : facing === "left" ? "<" : facing === "right" ? ">" : "@";

const nativeRouteCellSize = METATILE_SIZE / METATILE_WIDTH;

const scaleSurfaceToRouteCellSize = (
  surface: InstanceType<typeof gameEngine.Surface>,
  cellSize: number
): InstanceType<typeof gameEngine.Surface> => {
  if (cellSize === nativeRouteCellSize) {
    return surface;
  }
  const imageSource = surface.getCanvasImageSource();
  if (!imageSource) {
    throw new Error("Route render tile surface could not expose an image source.");
  }
  const scaled = new gameEngine.Surface(
    Math.max(1, Math.round(surface.get_width() * cellSize / nativeRouteCellSize)),
    Math.max(1, Math.round(surface.get_height() * cellSize / nativeRouteCellSize))
  );
  const context = scaled.getContext();
  context.imageSmoothingEnabled = false;
  context.drawImage(imageSource, 0, 0, scaled.get_width(), scaled.get_height());
  return scaled;
};

const drawMarker = (
  surface: InstanceType<typeof gameEngine.Surface>,
  token: string,
  coords: RouteRenderCoords,
  cellSize: number,
  facing: RouteRenderDirection | undefined = undefined
): void => {
  const color = tokenColors[token] ?? tokenColors["?"];
  const markerSize = token === "@" ? Math.max(cellSize * 2, 10) : Math.max(cellSize, 8);
  const x = Math.trunc(coords.x) * cellSize;
  const y = Math.trunc(coords.y) * cellSize;
  if (
    y + markerSize < 0 ||
    x + markerSize < 0 ||
    y >= surface.get_height() ||
    x >= surface.get_width()
  ) {
    return;
  }

  surface.fill([color[0], color[1], color[2], token === "@" ? 150 : 110], {
    x,
    y,
    width: markerSize,
    height: markerSize,
  });
  surface.fill([24, 28, 32, 230], { x, y, width: markerSize, height: 1 });
  surface.fill([24, 28, 32, 230], { x, y: y + markerSize - 1, width: markerSize, height: 1 });
  surface.fill([24, 28, 32, 230], { x, y, width: 1, height: markerSize });
  surface.fill([24, 28, 32, 230], { x: x + markerSize - 1, y, width: 1, height: markerSize });
  drawGlyph(surface, token === "@" ? facingToken(facing) : token, x, y, markerSize, [18, 22, 26, 255]);
};

export const renderRouteRenderTileSurface = (
  params: RenderRouteRenderTileSurfaceParams,
  options: RouteRenderImageOptions = {}
): InstanceType<typeof gameEngine.Surface> => {
  const cellSize = clampCellSize(options.cellSize);
  const baseSurface = create_map_surface(params.mapData, params.tileset, {
    vram: params.vram ?? null,
  });
  const surface = scaleSurfaceToRouteCellSize(baseSurface, cellSize);
  const rows = params.snapshot.grid?.rows ?? [];

  for (let y = 0; y < rows.length; y += 1) {
    const row = rows[y] ?? "";
    for (let x = 0; x < row.length; x += 1) {
      if (row[x] === "*") {
        drawMarker(surface, "*", { x, y }, cellSize);
      }
    }
  }

  for (const warp of params.snapshot.warps) {
    drawMarker(surface, "D", warp.coords, cellSize);
  }

  for (const hotspot of params.snapshot.hotspots) {
    if (hotspot.visible === false) {
      continue;
    }
    drawMarker(surface, hotspotToken(hotspot), hotspot.coords, cellSize);
  }

  if (params.snapshot.player) {
    drawMarker(
      surface,
      "@",
      params.snapshot.player.coords,
      cellSize,
      params.snapshot.player.facing
    );
  }

  return surface;
};

export const renderRouteRenderSurface = (
  snapshot: RouteRenderSnapshot,
  options: RouteRenderImageOptions = {}
): InstanceType<typeof gameEngine.Surface> => {
  const rows = snapshot.grid?.rows ?? [];
  const width = Math.max(1, snapshot.size?.width ?? rows[0]?.length ?? 1);
  const height = Math.max(1, snapshot.size?.height ?? rows.length);
  const cellSize = clampCellSize(options.cellSize);
  const surface = new gameEngine.Surface(width * cellSize, height * cellSize);
  surface.fill([246, 244, 235, 255]);

  for (let y = 0; y < height; y += 1) {
    const row = rows[y] ?? "";
    for (let x = 0; x < width; x += 1) {
      const token = row[x] ?? "x";
      const color = tokenColors[token] ?? tokenColors["?"];
      const px = x * cellSize;
      const py = y * cellSize;
      surface.fill(color, { x: px, y: py, width: cellSize, height: cellSize });
      if (cellSize >= 8) {
        surface.fill([0, 0, 0, 55], { x: px, y: py, width: cellSize, height: 1 });
        surface.fill([0, 0, 0, 55], { x: px, y: py, width: 1, height: cellSize });
      }
      const glyphToken = token === "@" ? facingToken(snapshot.player?.facing) : token;
      const glyphColor: [number, number, number, number] =
        token === "#" ? [226, 226, 226, 255] : [24, 28, 32, 255];
      drawGlyph(surface, glyphToken, px, py, cellSize, glyphColor);
    }
  }

  return surface;
};
