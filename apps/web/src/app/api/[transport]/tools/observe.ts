import * as z from "zod";
import { renderFrameToCompactText } from "@/app/mcp/text-render";
import type { TextSnapshotPayload } from "@/app/mcp/text-render";
import type { McpMapInfoSnapshot } from "@/app/mcp/map-info";
import type {
  McpPlayerContext,
  McpRecentEventsSnapshot,
  McpStatusSnapshot,
} from "@/app/mcp/session";
import {
  MAX_ADVANCE_FRAMES,
  getObserveSnapshotCache,
  invalidateObserveSnapshotCache,
  McpToolExtra,
  McpToolResponse,
  loadSession,
  setObserveSnapshotCache,
  resolveSessionId,
  McpToolContent,
  withRequestIdentity,
} from "./common";
import {
  IncludeSnapshotTextSchema,
  PayloadDetailSchema,
  PayloadFormatSchema,
  normalizePayloadOptions,
  serializeStructuredPayload,
} from "./serialization";
const coerceOptionalInt = (min: number, max: number) =>
  z.preprocess((value) => {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (/^\d+$/.test(trimmed)) {
        return Number(trimmed);
      }
    }
    return value;
  }, z.number().int().min(min).max(max).optional());

export const ObserveSchema = z.object({
  include_image: z.boolean().optional(),
  image_scale: coerceOptionalInt(1, 8),
  advance_frames: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
  format: PayloadFormatSchema,
  detail: PayloadDetailSchema,
  include_snapshot_text: IncludeSnapshotTextSchema,
});

type ObserveCompactContext = {
  m: McpStatusSnapshot["mode"];
  map?: string;
  xy?: [number, number];
  dir: McpPlayerContext["facing"];
  bat?: 1;
  menu?: 1;
  dlg?: 1;
  txt?: 1;
  pr?: 1;
  lock?: 1;
  busy?: 1;
  mv?: 0;
  blk?: string;
  last?: string;
  n?: number;
};

type OverworldWindow = {
  grid: string[][];
  origin: { x: number; y: number };
  player?: { x: number; y: number; token: string };
  // Warp threshold tiles in the viewport, plus the recommended "approach" tile
  // and the direction the player should move to actually trigger the warp.
  warps: Array<{
    threshold: { x: number; y: number };
    approach?: { x: number; y: number };
    through?: "up" | "down" | "left" | "right";
    token?: string;
  }>;
};

type VisibleScreen = {
  focus: string;
  pos?: [number, number];
  viewport?: string[];
  info?: string[];
  dialogue?: string[];
  prompt?: string[];
  menu?: {
    items: string[];
    cursor?: number;
    selected?: string;
  };
  ahead?: string;
};

const stepForward = (
  coords: { x: number; y: number } | undefined,
  facing: McpPlayerContext["facing"] | undefined,
  coordStride = 1
): { x: number; y: number } | undefined => {
  if (!coords) {
    return undefined;
  }
  const stride = Math.max(1, coordStride);
  switch (facing) {
    case "up":
      return { x: coords.x, y: coords.y - stride };
    case "down":
      return { x: coords.x, y: coords.y + stride };
    case "left":
      return { x: coords.x - stride, y: coords.y };
    case "right":
      return { x: coords.x + stride, y: coords.y };
    default:
      return undefined;
  }
};

const OBSERVE_HOTSPOT_LIMIT = 16;
const OBSERVE_VIEWPORT_LIMIT = 19;
const OBSERVE_INFO_LIMIT = 12;
const DIRECTION_CONVENTION = {
  coord: "x+ right, x- left, y+ down, y- up",
  move: {
    up: [0, -1],
    down: [0, 1],
    left: [-1, 0],
    right: [1, 0],
  },
  glyphs: {
    "@^": "player facing up",
    "@v": "player facing down",
    "@<": "player facing left",
    "@>": "player facing right",
    "D^": "warp entered by moving up",
    "Dv": "warp entered by moving down",
    "D<": "warp entered by moving left",
    "D>": "warp entered by moving right",
  },
} as const;

const buildCompactMapSummary = (
  map: NonNullable<TextSnapshotPayload["map"]> | undefined,
  mode: McpStatusSnapshot["mode"]
) => {
  if (!map || (mode !== "overworld" && mode !== "menu" && mode !== "battle")) {
    return undefined;
  }
  return {
    id: map.map_id,
    p: map.player
      ? [map.player.coords.x, map.player.coords.y]
      : undefined,
    hs: map.hotspots
      .filter((hotspot) => hotspot.type !== "trigger")
      .slice(0, OBSERVE_HOTSPOT_LIMIT)
      .map((hotspot) => ({
        t: hotspot.type,
        xy: [hotspot.coords.x, hotspot.coords.y],
        l: hotspot.label,
        tk: hotspot.token,
        i: hotspot.interactable === false ? 0 : 1,
        sp: hotspot.spoiler_masked ? 1 : undefined,
      })),
  };
};

const resolveStructuredAhead = (
  status: McpStatusSnapshot,
  mapInfo?: McpMapInfoSnapshot
): string | undefined => {
  const aheadCoords = status.interaction_tile
    ? { x: status.interaction_tile.x, y: status.interaction_tile.y }
    : stepForward(
        mapInfo?.player?.coords ?? status.coords,
        status.facing ?? mapInfo?.player?.facing,
        mapInfo?.coord_stride ?? 1
      );
  if (!aheadCoords || !mapInfo) {
    return undefined;
  }
  const hotspotAhead = mapInfo.hotspots.find((hotspot) =>
    hotspot.visible &&
    hotspot.coords.x === aheadCoords.x &&
    hotspot.coords.y === aheadCoords.y
  );
  if (hotspotAhead) {
    return hotspotAhead.token ?? undefined;
  }
  const warpAhead = mapInfo.warps.find((warp) =>
    warp.coords.x === aheadCoords.x &&
    warp.coords.y === aheadCoords.y
  );
  if (warpAhead) {
    return "D";
  }
  return undefined;
};

const ANSI_ESCAPE_PATTERN =
  // eslint-disable-next-line no-control-regex
  /\u001b\[[0-9;]*m/g;

const parseViewportRowLabel = (line: string): number | null => {
  const match = line.match(/^\s*(\d+)\s+/);
  return match ? Number(match[1]) : null;
};

const centerVisibleViewport = (
  lines: string[],
  pos: [number, number] | undefined,
  limit: number
): string[] => {
  const cleaned = lines
    .map((line) => line.replace(ANSI_ESCAPE_PATTERN, "").trimEnd())
    .filter(Boolean);
  if (cleaned.length <= limit) {
    return cleaned;
  }

  const header = cleaned[0] ?? "";
  const body = cleaned.slice(1);
  const bodyLimit = Math.max(1, limit - 1);
  const playerRow = body.findIndex((line) => line.includes("@"));
  const targetRow =
    playerRow >= 0
      ? playerRow
      : pos
        ? body.findIndex((line) => parseViewportRowLabel(line) === pos[1])
        : -1;

  if (targetRow < 0) {
    return cleaned.slice(0, limit);
  }

  const start = Math.max(
    0,
    Math.min(targetRow - Math.floor(bodyLimit / 2), body.length - bodyLimit)
  );
  return [header, ...body.slice(start, start + bodyLimit)];
};

const parseOverworldWindow = (snapshotText: string): OverworldWindow | null => {
  const lines = snapshotText.split("\n");
  const gridStart = lines.findIndex((l) => l.startsWith("OVERWORLD"));
  if (gridStart < 0) {
    return null;
  }
  const columnLabels = (lines[gridStart + 1] ?? "")
    .split(/\s+/)
    .filter(Boolean)
    .map((value) => Number(value))
    .filter((value) => Number.isFinite(value));
  const originX = columnLabels[0] ?? 0;
  let originY = 0;
  const grid: string[][] = [];
  // Grid begins 2 lines after OVERWORLD.
  for (let i = gridStart + 2; i < lines.length; i += 1) {
    const l = lines[i] ?? "";
    if (!/^\d\d /.test(l)) {
      break;
    }
    const parts = l.split(/\s+/).filter(Boolean);
    if (grid.length === 0) {
      const rowLabel = Number(parts[0]);
      originY = Number.isFinite(rowLabel) ? rowLabel : 0;
    }
    grid.push(parts.slice(1));
  }
  if (!grid.length) {
    return null;
  }

  const height = grid.length;
  const width = Math.max(...grid.map((row) => row.length));

  let player: OverworldWindow["player"];
  const warps: OverworldWindow["warps"] = [];

  const inBounds = (x: number, y: number) => y >= 0 && y < height && x >= 0 && x < (grid[y]?.length ?? 0);
  const isOccupant = (tile: string) => tile.startsWith("@");
  const isNpc = (tile: string) => tile.startsWith("N");

  const inferThrough = (x: number, y: number): OverworldWindow["warps"][number]["through"] => {
    // Better heuristic than "edge only": the OVERWORLD window uses 'x' as out-of-bounds padding.
    // Door warps often appear one row above the 'x' padding; stairs often appear one row below.
    const below = inBounds(x, y + 1) ? grid[y + 1]?.[x] : "x";
    const above = inBounds(x, y - 1) ? grid[y - 1]?.[x] : "x";
    const left = inBounds(x - 1, y) ? grid[y]?.[x - 1] : "x";
    const right = inBounds(x + 1, y) ? grid[y]?.[x + 1] : "x";

    if (below === "x") return "down";
    if (above === "x") return "up";
    if (left === "x") return "left";
    if (right === "x") return "right";

    // Fallback to literal edge detection.
    if (y === height - 1) return "down";
    if (y === 0) return "up";
    if (x === 0) return "left";
    if (x === width - 1) return "right";
    return undefined;
  };

  const arrowFor = (through: ReturnType<typeof inferThrough>) => {
    switch (through) {
      case "down":
        return "v";
      case "up":
        return "^";
      case "left":
        return "<";
      case "right":
        return ">";
      default:
        return "";
    }
  };

  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < (grid[y]?.length ?? 0); x += 1) {
      const tile = grid[y]?.[x];
      if (!tile) continue;

      if (!player && tile.startsWith("@")) {
        player = { x: x + originX, y: y + originY, token: tile };
        continue;
      }

      if (tile.startsWith("D")) {
        const through = inferThrough(x, y);
        const arrow = arrowFor(through);
        const token = arrow ? `D${arrow}` : tile;

        // Compute the recommended approach tile (stand here, then move "through").
        let approach: { x: number; y: number } | undefined;
        let localApproach: { x: number; y: number } | undefined;
        if (through === "down") approach = { x: x + originX, y: y + originY - 1 };
        if (through === "up") approach = { x: x + originX, y: y + originY + 1 };
        if (through === "left") approach = { x: x + originX + 1, y: y + originY };
        if (through === "right") approach = { x: x + originX - 1, y: y + originY };
        if (through === "down") localApproach = { x, y: y - 1 };
        if (through === "up") localApproach = { x, y: y + 1 };
        if (through === "left") localApproach = { x: x + 1, y };
        if (through === "right") localApproach = { x: x - 1, y };

        // Render the warp token like player/NPC directional tokens.
        grid[y][x] = token;

        // Make the approach tile "look open" in the grid so agent pathing prefers it.
        // (We only override obviously-walkable-looking tiles; never overwrite player/NPC tokens.)
        if (localApproach && inBounds(localApproach.x, localApproach.y)) {
          const at = grid[localApproach.y]?.[localApproach.x];
          if (at && !isOccupant(at) && !isNpc(at) && at !== "#") {
            grid[localApproach.y][localApproach.x] = ".";
          }
        }

        warps.push({ threshold: { x: x + originX, y: y + originY }, approach, through, token });
      }
    }
  }

  return { grid, origin: { x: originX, y: originY }, player, warps };
};

const buildCompactContext = (
  sessionId: string | undefined,
  playerContext: McpPlayerContext,
  status: McpStatusSnapshot,
  recentEvents: McpRecentEventsSnapshot
): ObserveCompactContext => {
  const eventCount = Array.isArray(recentEvents.events) ? recentEvents.events.length : 0;
  const coords = playerContext.coords ?? status.coords ?? undefined;
  const map = playerContext.map ?? status.map ?? undefined;
  return {
    m: status.mode,
    map: map ?? undefined,
    xy: coords ? [coords.x, coords.y] : undefined,
    dir: playerContext.facing,
    bat: status.in_battle ? 1 : undefined,
    menu: playerContext.menu_open ? 1 : undefined,
    dlg: status.in_dialog || playerContext.dialogue_open ? 1 : undefined,
    txt: status.text_box_open ?? status.textbox_open ? 1 : undefined,
    pr: status.prompt?.pending ? 1 : undefined,
    lock: status.movement_locked ? 1 : undefined,
    busy: status.script_busy ? 1 : undefined,
    mv: status.can_move ? undefined : 0,
    blk: status.input_blocked_reason ?? undefined,
    last: recentEvents.recap && recentEvents.recap !== "no_events" ? recentEvents.recap : undefined,
    n: eventCount || undefined,
  };
};

const extractMenuState = (
  lines: string[] | null | undefined
): VisibleScreen["menu"] | undefined => {
  if (!lines?.length) {
    return undefined;
  }
  const items = lines
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(0, OBSERVE_HOTSPOT_LIMIT);
  if (!items.length) {
    return undefined;
  }
  const cursor = items.findIndex((line) => /^[>▶▷]/.test(line));
  const selected = cursor >= 0 ? items[cursor]?.replace(/^[>▶▷]\s*/, "") : undefined;
  return {
    items,
    cursor: cursor >= 0 ? cursor : undefined,
    selected: selected || undefined,
  };
};

const buildVisibleScreen = (
  frame: TextSnapshotPayload | null,
  status: McpStatusSnapshot,
  overworldWindow: OverworldWindow | null,
  mapInfo?: McpMapInfoSnapshot
): VisibleScreen | undefined => {
  if (!frame) {
    return undefined;
  }
  const posLine = frame.info.find((line) => line.trim().startsWith("Pos:"));
  const posMatch = posLine?.match(/\((\d+),\s*(\d+)\)/);
  const pos =
    posMatch && posMatch[1] && posMatch[2]
      ? [Number(posMatch[1]), Number(posMatch[2])] as [number, number]
      : undefined;

  const dialogue = frame.dialogue?.map((line) => line.trim()).filter(Boolean);
  const prompt = frame.prompt?.map((line) => line.trim()).filter(Boolean).slice(0, 6);
  const menu = extractMenuState(frame.menu);
  const viewport = centerVisibleViewport(frame.viewport, pos, OBSERVE_VIEWPORT_LIMIT);
  const info = frame.info
    .map((line) => line.replace(ANSI_ESCAPE_PATTERN, "").trimEnd())
    .filter(Boolean)
    .slice(0, OBSERVE_INFO_LIMIT);

  const player = overworldWindow?.player;
  let ahead: string | undefined;
  if (player) {
    const deltas: Record<McpPlayerContext["facing"], [number, number]> = {
      up: [0, -1],
      down: [0, 1],
      left: [-1, 0],
      right: [1, 0],
      unknown: [0, 0],
    };
    const facing = status.facing ?? "unknown";
    const [dx, dy] = deltas[facing];
    const ax = player.x + dx;
    const ay = player.y + dy;
    const gridAhead = overworldWindow?.grid?.[ay - overworldWindow.origin.y]?.[ax - overworldWindow.origin.x] ?? undefined;
    const structuredAhead = resolveStructuredAhead(status, mapInfo);
    const spriteOnlyInteractableAhead =
      typeof gridAhead === "string" &&
      /^(?:N|I|!)/.test(gridAhead) &&
      !structuredAhead;
    ahead = structuredAhead ?? (spriteOnlyInteractableAhead ? undefined : gridAhead);
  }

  let focus: VisibleScreen["focus"] = status.mode || "overworld";
  if (status.mode === "battle") {
    focus = "battle";
  } else if (menu) {
    focus = "menu";
  } else if (prompt?.length) {
    focus = "prompt";
  } else if (dialogue?.length) {
    focus = "dialogue";
  }

  return {
    focus,
    pos,
    viewport: viewport.length ? viewport : undefined,
    info: info.length ? info : undefined,
    dialogue: dialogue?.length ? dialogue : undefined,
    prompt: prompt?.length ? prompt : undefined,
    menu,
    ahead,
  };
};

export const observeHandler = async (
  input: z.infer<typeof ObserveSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const resolvedSessionId = resolveSessionId(extra);
    const session = await loadSession(resolvedSessionId, extra);
    if (input.advance_frames) {
      await session.advanceFrames(input.advance_frames);
      invalidateObserveSnapshotCache(resolvedSessionId);
    }

    const currentFrame = session.getFrameCount();
    const cached = getObserveSnapshotCache(resolvedSessionId, currentFrame);
    const fullSnapshotText = cached?.snapshotText ?? session.observeText();
    const playerContext: McpPlayerContext =
      cached?.playerContext ?? (await session.playerContext());
    const statusSnapshot: McpStatusSnapshot =
      cached?.statusSnapshot ?? (await session.status());
    const recentEventsSnapshot: McpRecentEventsSnapshot =
      cached?.recentEventsSnapshot ?? (await session.recentEvents(5));
    const frameId = cached?.frameId ?? currentFrame;
    const computedAtMs = cached?.computedAtMs ?? Date.now();
    const normalized = normalizePayloadOptions({
      format: input.format,
      detail: input.detail,
      include_snapshot_text: input.include_snapshot_text ?? true,
    });
    const framePayload = session.observePayload();
    const mapInfoSnapshot = await session.mapInfo();
    const snapshotText =
      normalized.detail === "full"
        ? fullSnapshotText
        : renderFrameToCompactText(framePayload);
    if (!cached) {
      setObserveSnapshotCache(resolvedSessionId, {
        frameCounter: currentFrame,
        snapshotText: fullSnapshotText,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot,
        frameId,
        computedAtMs,
      });
    }
    const content: McpToolContent[] = [];
    if (normalized.include_snapshot_text) {
      content.push({ type: "text", text: snapshotText });
    }
    const overworldWindow = parseOverworldWindow(fullSnapshotText);
    const visible = buildVisibleScreen(framePayload, statusSnapshot, overworldWindow, mapInfoSnapshot);

    const compactSnapshot = {
      ctx: buildCompactContext(
        resolvedSessionId,
        playerContext,
        statusSnapshot,
        recentEventsSnapshot
      ),
      r:
        recentEventsSnapshot.total > 0
          ? {
              sum: recentEventsSnapshot.recap,
              n: recentEventsSnapshot.total,
              tr: recentEventsSnapshot.truncated ? 1 : undefined,
            }
          : undefined,
      ow:
        overworldWindow
          ? {
              g: overworldWindow.grid,
              o: [overworldWindow.origin.x, overworldWindow.origin.y],
              p: overworldWindow.player
                ? [overworldWindow.player.x, overworldWindow.player.y]
                : undefined,
              w: overworldWindow.warps.map((warp) => ({
                at: [warp.threshold.x, warp.threshold.y],
                ap: warp.approach ? [warp.approach.x, warp.approach.y] : undefined,
                go: warp.through,
                stand: warp.approach ? [warp.approach.x, warp.approach.y] : undefined,
                move: warp.through,
                note:
                  warp.approach && warp.through
                    ? `stand at ${warp.approach.x},${warp.approach.y}; move ${warp.through} to enter`
                    : undefined,
              })),
            }
          : undefined,
      dir: DIRECTION_CONVENTION,
      map: buildCompactMapSummary(framePayload?.map, statusSnapshot.mode),
      flow_state: framePayload?.flow_state
        ? {
            sum: framePayload.flow_state.summary,
            done: framePayload.flow_state.completed_count,
            total: framePayload.flow_state.total_count,
            next: framePayload.flow_state.next_goal?.title,
            target: framePayload.flow_state.completion_target.title,
          }
        : undefined,
      view: visible,
    };
    const serialized = await serializeStructuredPayload(compactSnapshot);
    content.push({
      type: "text",
      text: serialized.text,
      mimeType: serialized.mimeType,
    });

    if (input.include_image) {
      const image = await session.observeTilemapImage({ scale: input.image_scale });
      content.push({
        type: "image",
        data: image.data,
        mimeType: "image/png",
      });
    }

    return { content, snapshot: compactSnapshot };
  });
};

export const __testables = {
  buildCompactMapSummary,
  buildVisibleScreen,
  DIRECTION_CONVENTION,
  parseOverworldWindow,
  resolveStructuredAhead,
};

export const ObserveTilemapSchema = z.object({
  scale: coerceOptionalInt(1, 8),
  advance_frames: coerceOptionalInt(1, MAX_ADVANCE_FRAMES),
});

export const observeTilemapHandler = async (
  input: z.infer<typeof ObserveTilemapSchema>,
  extra?: McpToolExtra
): Promise<McpToolResponse> => {
  return withRequestIdentity(extra, async () => {
    const session = await loadSession(resolveSessionId(extra), extra);
    if (input.advance_frames) {
      await session.advanceFrames(input.advance_frames);
    }
    return {
      content: [{ type: "text", text: "observe_tilemap is disabled for text-only MCP sessions." }],
    };
  });
};
