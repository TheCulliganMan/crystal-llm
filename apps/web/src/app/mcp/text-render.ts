import { TextSnapshot } from "@pokecrystal/core/ui/text-ui";
import { buildTextSnapshotLines } from "@pokecrystal/core/ui/text-snapshot-render";
import type { McpFlowStateSnapshot } from "./flow-state";
import type { McpMapInfoSnapshot } from "./map-info";

export type McpMoveSummary = {
  direction: string;
  requested: number;
  completed?: number;
  start?: [number, number];
  end?: [number, number];
  map?: string;
};

export type McpMacroSummary = {
  requested: number;
  completed: number;
  stopped: boolean;
  reason?: string;
  reason_codes?: string[];
  close_menu_count?: number;
  busy_wait_count?: number;
  nudged_choice_count?: number;
};

export type McpMacroTraceAction = {
  index: number;
  type: "move" | "button" | "wait";
  value?: string;
  times?: number;
  hold_frames?: number;
  delay_frames?: number;
  frames?: number;
};

export type McpMacroTraceStep = {
  index: number;
  action: McpMacroTraceAction;
  before?: { coords?: { x: number; y: number }; map?: string };
  after?: { coords?: { x: number; y: number }; map?: string };
  stop_reason?: string;
  block_reason?: string | null;
};

export type McpMacroExecutionTrace = {
  raw_input: {
    macro?: string;
    actions: McpMacroTraceAction[];
    truncated?: boolean;
    total?: number;
  };
  normalized_actions: {
    actions: McpMacroTraceAction[];
    truncated?: boolean;
    total?: number;
  };
  executed_actions: {
    steps: McpMacroTraceStep[];
    truncated?: boolean;
    total?: number;
  };
  stop_reason?: string | null;
  interruption?: string | null;
  stale_input_cleared?: boolean;
};

export interface McpMeta {
  move_summary?: McpMoveSummary;
  macro_summary?: McpMacroSummary;
  macro_execution_trace?: McpMacroExecutionTrace;
}

export type TextSnapshotPayload = {
  viewport: string[];
  info: string[];
  menu: string[] | null;
  prompt: string[] | null;
  dialogue: string[] | null;
  notices?: string[];
  titles: {
    viewport: string;
    info: string;
  };
  marker: [number, number, string] | null;
  action_log: string[];
  script: Record<string, unknown>;
  tasks: Record<string, unknown>[];
  mcp?: McpMeta;
  map?: McpMapInfoSnapshot;
  flow_state?: McpFlowStateSnapshot | null;
};

export type PromptStatus = {
  pending: boolean;
  reason: string | null;
};

const hasPromptCursor = (lines: readonly string[] | null | undefined): boolean =>
  Boolean(lines?.some((line) => /^\s*[>▶▷]/.test(line)));

const normalizePcUiLine = (value: string): string =>
  value
    .trim()
    .toLowerCase()
    .replace(/^[>▶▷]\s*/, "")
    .replace(/<pk>\s*<mn>/g, "pokemon")
    .replace(/#mon/g, "pokemon")
    .replace(/pok[eé]mon/g, "pokemon")
    .replace(/\s+/g, " ");

const pcMenuLineHints = [
  "bill's pc",
  "chris's pc",
  "withdraw pokemon",
  "deposit pokemon",
  "move pokemon w/o mail",
  "change box",
  "see ya",
  "turn off",
];

const hasPcUiLine = (lines: readonly string[] | null | undefined): boolean =>
  Boolean(lines?.some((line) => pcMenuLineHints.some((hint) => normalizePcUiLine(line).includes(hint))));

const pcSnapshotTitleText = (frame: TextSnapshotPayload): string => {
  const legacyFrame = frame as TextSnapshotPayload & {
    viewportTitle?: string;
    infoTitle?: string;
  };
  return [
    frame.titles?.viewport,
    frame.titles?.info,
    legacyFrame.viewportTitle,
    legacyFrame.infoTitle,
    frame.viewport?.[0],
  ]
    .filter((entry): entry is string => Boolean(entry))
    .join(" ")
    .toLowerCase();
};

export const isNonBlockingPcPromptSnapshot = (frame: TextSnapshotPayload | null): boolean => {
  if (!frame?.prompt?.length) {
    return false;
  }
  const title = pcSnapshotTitleText(frame);
  if (!title.includes("pc") && !hasPcUiLine(frame.viewport) && !hasPcUiLine(frame.menu)) {
    return false;
  }
  return !hasPromptCursor(frame.prompt);
};

export const isNonBlockingPcMenuSnapshot = (frame: TextSnapshotPayload | null): boolean =>
  Boolean(frame?.menu?.length && (pcSnapshotTitleText(frame).includes("pc") || hasPcUiLine(frame.menu)));

export const isNonBlockingPcUiSnapshot = (frame: TextSnapshotPayload | null): boolean =>
  isNonBlockingPcMenuSnapshot(frame) || isNonBlockingPcPromptSnapshot(frame);

export const buildSnapshotPayload = (snapshot: TextSnapshot, options: {
  actionLog: string[];
  script?: Record<string, unknown> | null;
  tasks?: Record<string, unknown>[] | null;
  mcp?: McpMeta | null;
  map?: McpMapInfoSnapshot | null;
  notices?: string[] | null;
}): TextSnapshotPayload => {
  return {
    viewport: [...snapshot.viewportLines],
    info: [...snapshot.infoLines],
    menu: snapshot.menuLines ? [...snapshot.menuLines] : null,
    prompt: snapshot.promptLines ? [...snapshot.promptLines] : null,
    dialogue: snapshot.dialogueLines ? [...snapshot.dialogueLines] : null,
    notices: options.notices ? [...options.notices] : undefined,
    titles: {
      viewport: snapshot.viewportTitle ?? "Viewport",
      info: snapshot.infoTitle ?? "Info",
    },
    marker: snapshot.marker ?? null,
    action_log: [...options.actionLog],
    script: options.script ?? {},
    tasks: options.tasks ?? [],
    mcp: options.mcp ?? undefined,
    map: options.map ?? undefined,
  };
};

const buildSnapshotFromPayload = (frame: TextSnapshotPayload): TextSnapshot => {
  return {
    viewportLines: [...frame.viewport],
    infoLines: [...frame.info],
    menuLines: frame.menu ? [...frame.menu] : null,
    promptLines: frame.prompt ? [...frame.prompt] : null,
    dialogueLines: frame.dialogue ? [...frame.dialogue] : null,
    viewportTitle: frame.titles.viewport,
    infoTitle: frame.titles.info,
    marker: frame.marker ?? null,
    actionLog: frame.action_log ?? [],
  };
};

const scaleAxisDistance = (distance: number, coordStride: number | undefined): number => {
  const stride = typeof coordStride === "number" && coordStride > 1 ? coordStride : 1;
  const magnitude = Math.abs(distance);
  if (stride > 1 && magnitude % stride === 0) {
    return magnitude / stride;
  }
  return magnitude;
};

const formatHotspotOffset = (
  hotspot: { x: number; y: number },
  player: { x: number; y: number } | undefined,
  coordStride: number | undefined
): string => {
  if (!player) {
    return "";
  }
  const dx = hotspot.x - player.x;
  const dy = hotspot.y - player.y;
  if (dx === 0 && dy === 0) {
    return " here";
  }
  const parts: string[] = [];
  if (dy !== 0) {
    parts.push(`${scaleAxisDistance(dy, coordStride)}${dy < 0 ? "N" : "S"}`);
  }
  if (dx !== 0) {
    parts.push(`${scaleAxisDistance(dx, coordStride)}${dx < 0 ? "W" : "E"}`);
  }
  return parts.length ? ` (${parts.join(" ")})` : "";
};

const formatApproachOffset = (
  approach: { x: number; y: number; facing: "up" | "down" | "left" | "right" },
  player: { x: number; y: number; facing?: "up" | "down" | "left" | "right" } | undefined,
  coordStride: number | undefined
): string => {
  if (!player) {
    return "";
  }
  const dx = approach.x - player.x;
  const dy = approach.y - player.y;
  if (dx === 0 && dy === 0) {
    return player.facing === approach.facing ? " here" : ` here face ${approach.facing}`;
  }
  const offset = formatHotspotOffset({ x: approach.x, y: approach.y }, player, coordStride);
  return `${offset} face ${approach.facing}`;
};

const selectNearestApproachTile = (
  hotspot: McpMapInfoSnapshot["hotspots"][number],
  player: { x: number; y: number; facing?: "up" | "down" | "left" | "right" } | undefined
): { x: number; y: number; facing: "up" | "down" | "left" | "right" } | null => {
  if (!player || !hotspot.approach_tiles?.length) {
    return null;
  }
  const nearest =
    hotspot.approach_tiles
    .slice()
    .sort(
      (left, right) =>
        (Math.abs(left.coords.x - player.x) + Math.abs(left.coords.y - player.y)) -
          (Math.abs(right.coords.x - player.x) + Math.abs(right.coords.y - player.y)) ||
        left.coords.x - right.coords.x ||
        left.coords.y - right.coords.y ||
        left.facing.localeCompare(right.facing)
    )[0] ?? null;
  return nearest
    ? {
        x: nearest.coords.x,
        y: nearest.coords.y,
        facing: nearest.facing,
      }
    : null;
};

const hotspotReferencePoint = (
  hotspot: McpMapInfoSnapshot["hotspots"][number],
  player: { x: number; y: number; facing?: "up" | "down" | "left" | "right" } | undefined
): { x: number; y: number } => {
  const approach = selectNearestApproachTile(hotspot, player);
  if (approach) {
    return { x: approach.x, y: approach.y };
  }
  return hotspot.coords;
};

const hotspotTypePriority = (type: McpMapInfoSnapshot["hotspots"][number]["type"]): number => {
  switch (type) {
    case "heal":
      return 0;
    case "objective":
      return 1;
    case "shop":
      return 2;
    case "warp":
      return 3;
    case "npc":
      return 4;
    case "gym":
    case "utility":
      return 5;
    case "trigger":
      return 6;
    case "sign":
    case "landmark":
      return 7;
    default:
      return 8;
  }
};

const hotspotActionabilityPriority = (hotspot: McpMapInfoSnapshot["hotspots"][number]): number => {
  return hotspot.interactable === false ? 1 : 0;
};

const hotspotLines = (map: McpMapInfoSnapshot | undefined): string[] => {
  const hotspots = (map?.hotspots ?? []).filter((hotspot) => hotspot.type !== "trigger");
  const coordStride = map?.coord_stride;
  const player = map?.player
    ? {
        x: map.player.coords.x,
        y: map.player.coords.y,
        facing: map.player.facing,
      }
    : undefined;
  const rankedHotspots = hotspots
    .slice()
    .sort((left, right) => {
      const leftReference = hotspotReferencePoint(left, player);
      const rightReference = hotspotReferencePoint(right, player);
      const leftDistance = player
        ? Math.abs(leftReference.x - player.x) + Math.abs(leftReference.y - player.y)
        : Number.MAX_SAFE_INTEGER;
      const rightDistance = player
        ? Math.abs(rightReference.x - player.x) + Math.abs(rightReference.y - player.y)
        : Number.MAX_SAFE_INTEGER;
      return (
        hotspotActionabilityPriority(left) - hotspotActionabilityPriority(right) ||
        hotspotTypePriority(left.type) - hotspotTypePriority(right.type) ||
        leftDistance - rightDistance ||
        left.coords.y - right.coords.y ||
        left.coords.x - right.coords.x ||
        left.label.localeCompare(right.label)
      );
    });
  return rankedHotspots
    .slice(0, 8)
    .map((hotspot) => {
      const token = hotspot.token ? `${hotspot.token} ` : "";
      const approach = selectNearestApproachTile(hotspot, player);
      const offset = approach
        ? formatApproachOffset(approach, player, coordStride)
        : formatHotspotOffset(hotspot.coords, player, coordStride);
      if (player) {
        return `${token}${hotspot.label}${offset}`;
      }
      return `${token}${hotspot.coords.x},${hotspot.coords.y} ${hotspot.label}`;
    });
};

const appendSection = (lines: string[], label: string, entries: string[]): void => {
  if (!entries.length) {
    return;
  }
  if (lines.length) {
    lines.push("");
  }
  lines.push(label.toUpperCase());
  lines.push(...entries);
};

const flowStateLines = (flowState: TextSnapshotPayload["flow_state"]): string[] => {
  if (!flowState) {
    return [];
  }
  return [
    flowState.summary,
    `Progress: ${flowState.completed_count}/${flowState.total_count}`,
  ].filter((line) => line.trim().length > 0);
};

const DROP_SECTION_HEADINGS = new Set([
  "INFO",
  "ACTION LOG",
  "TASKS",
  "SCRIPT",
  "ACTIVE INPUT",
  "SELECTION",
]);

const isLikelyBoilerplateLine = (line: string): boolean => {
  const trimmed = line.trim();
  return (
    trimmed.startsWith("D-Pad=") ||
    trimmed.startsWith("Controls:") ||
    trimmed.startsWith("Text queue:") ||
    trimmed === "Waiting for input..."
  );
};

const compactRenderedLines = (lines: string[]): string[] => {
  const compact: string[] = [];
  let skipSection = false;
  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      if (!skipSection && compact[compact.length - 1] !== "") {
        compact.push("");
      }
      continue;
    }
    const upper = trimmed.toUpperCase();
    if (DROP_SECTION_HEADINGS.has(upper)) {
      skipSection = true;
      continue;
    }
    if (/^[A-Z][A-Z ]+$/.test(upper)) {
      skipSection = false;
    }
    if (skipSection || isLikelyBoilerplateLine(trimmed)) {
      continue;
    }
    compact.push(line);
  }
  while (compact[compact.length - 1] === "") {
    compact.pop();
  }
  return compact;
};

export const renderFrameToText = (frame: TextSnapshotPayload | null): string => {
  if (!frame) {
    return "(empty frame)";
  }

  const lines = buildTextSnapshotLines(buildSnapshotFromPayload(frame));
  const output = compactRenderedLines(lines);
  appendSection(output, "Notice", (frame.notices ?? []).map((line) => line.trim()).filter(Boolean));
  appendSection(output, "Hotspots", hotspotLines(frame.map));

  return output.length ? output.join("\n") : "(empty frame)";
};

export const renderFrameToCompactText = (frame: TextSnapshotPayload | null): string => {
  if (!frame) {
    return "(empty frame)";
  }

  const lines: string[] = [];
  const title = (frame.titles.viewport || frame.viewport[0] || "STATE").trim().toUpperCase();
  if (title) {
    lines.push(title);
  }

  const posLine = (frame.info ?? []).find((line) => line.trim().startsWith("Pos:"));
  if (posLine) {
    lines.push(posLine.trim());
  }

  const legendLine = (frame.info ?? [])
    .map((line) => line.trim())
    .find((line) => line.startsWith("Legend:"));
  if (legendLine) {
    lines.push(legendLine);
  }

  if (frame.dialogue?.length) {
    appendSection(lines, "Dialogue", frame.dialogue.map((line) => line.trim()).filter(Boolean));
  }
  if (frame.prompt?.length) {
    appendSection(lines, "Prompt", frame.prompt.slice(0, 6).map((line) => line.trim()).filter(Boolean));
  }
  if (frame.menu?.length) {
    appendSection(lines, "Menu", frame.menu.slice(0, 8).map((line) => line.trim()).filter(Boolean));
  }
  appendSection(lines, "Notice", (frame.notices ?? []).map((line) => line.trim()).filter(Boolean));
  appendSection(lines, "Hotspots", hotspotLines(frame.map));
  appendSection(lines, "Flow", flowStateLines(frame.flow_state));

  return lines.length ? lines.join("\n") : "(empty frame)";
};

export const promptFromSnapshot = (frame: TextSnapshotPayload | null): PromptStatus => {
  if (!frame) {
    return { pending: false, reason: null };
  }
  if (isNonBlockingPcPromptSnapshot(frame)) {
    return { pending: false, reason: null };
  }
  if (frame.prompt && frame.prompt.length) {
    return { pending: true, reason: "prompt" };
  }
  return { pending: false, reason: null };
};
