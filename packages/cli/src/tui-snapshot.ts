import { extractText, parseJsonText } from "./client";
import type { ToolResult } from "./types";

export type TuiSnapshot = {
  mode: string;
  map: string;
  surface: string;
  promptStatus: string;
  frame: string;
  viewport: string[];
  info: string[];
  legend: string[];
  menu: string[];
  prompt: string[];
  dialogue: string[];
  actions: string[];
  statusLine: string;
};

const NAME_ENTRY_ROWS = [
  "A B C D E F G H I",
  "J K L M N O P Q R",
  "S T U V W X Y Z",
  "- ? ! / . ,",
  "lower DEL END",
];
const NAME_ENTRY_ROW_COUNT = 5;
const NAME_ENTRY_COLUMN_COUNT = 9;

export type DialogueAccumulator = {
  pages: string[][];
  lastKey: string | null;
};

export const createDialogueAccumulator = (): DialogueAccumulator => ({
  pages: [],
  lastKey: null,
});

const asLines = (value: unknown): string[] => {
  if (!Array.isArray(value)) {
    return [];
  }
  return value.filter((entry): entry is string => typeof entry === "string");
};

const isDialogueBoilerplateLine = (line: string): boolean => {
  const trimmed = line.trim();
  return trimmed.startsWith("Text queue:") || trimmed === "Waiting for input...";
};

const dialogueLines = (value: unknown): string[] =>
  asLines(value).filter((line) => !isDialogueBoilerplateLine(line));

const stripAnsi = (value: string): string =>
  value.replace(/\u001b\[[0-9;]*m/g, "");

const firstJson = (result?: ToolResult): Record<string, any> | undefined =>
  parseJsonText(result?.content).find(
    (entry) =>
      entry.view ||
      entry.ctx ||
      entry.status ||
      entry.r ||
      entry.mode ||
      entry.surface,
  );

const fullText = (result?: ToolResult): string => {
  const textEntries = (result?.content ?? [])
    .filter((entry) => entry.type === "text" && typeof entry.text === "string")
    .map((entry) => entry.text?.trim() ?? "")
    .filter(Boolean);
  const nonJson = textEntries.find((entry) => {
    try {
      JSON.parse(entry);
      return false;
    } catch {
      return true;
    }
  });
  return nonJson ?? extractText(result?.content);
};

const splitInfoAndLegend = (
  lines: string[],
): { info: string[]; legend: string[] } => {
  const info: string[] = [];
  const legend: string[] = [];
  for (const line of lines) {
    const upper = line.trim().toUpperCase();
    if (upper.startsWith("LEGEND:") || upper.startsWith("COORDS:")) {
      legend.push(line);
    } else {
      info.push(line);
    }
  }
  return { info, legend };
};

const extractFallbackSections = (
  text: string,
): Pick<
  TuiSnapshot,
  "viewport" | "info" | "legend" | "menu" | "prompt" | "dialogue"
> => {
  const lines = text.split("\n").map(stripAnsi);
  const viewport: string[] = [];
  const info: string[] = [];
  const legend: string[] = [];
  const menu: string[] = [];
  const prompt: string[] = [];
  const dialogue: string[] = [];
  let target: string[] = viewport;

  for (const line of lines) {
    const trimmed = line.trim();
    const upper = trimmed.toUpperCase();
    if (upper === "INFO" || upper.startsWith("INFO:")) {
      target = info;
      if (upper !== "INFO") target.push(line);
      continue;
    }
    if (upper === "MENU" || upper.startsWith("MENU")) {
      target = menu;
      continue;
    }
    if (upper === "PROMPT" || upper.startsWith("PROMPT")) {
      target = prompt;
      continue;
    }
    if (upper === "DIALOGUE" || upper.startsWith("DIALOGUE")) {
      target = dialogue;
      continue;
    }
    if (upper.startsWith("LEGEND:") || upper.startsWith("COORDS:")) {
      legend.push(line);
      continue;
    }
    target.push(line);
  }

  const split = splitInfoAndLegend(info);
  return {
    viewport: viewport.filter((line) => line.trim().length > 0),
    info: split.info,
    legend: [...legend, ...split.legend],
    menu,
    prompt,
    dialogue,
  };
};

const statusValue = (
  payload: Record<string, any> | undefined,
  key: string,
  compactKey?: string,
): unknown => {
  const status = payload?.status ?? payload?.ctx ?? payload;
  return status?.[key] ?? (compactKey ? status?.[compactKey] : undefined);
};

const frameValue = (payload: Record<string, any> | undefined): string => {
  const frame =
    payload?.frame ??
    payload?.view?.frame ??
    payload?.status?.frame ??
    payload?.status?.frame_id ??
    payload?.ctx?.frame;
  return typeof frame === "number" || typeof frame === "string"
    ? String(frame)
    : "-";
};

const coordsText = (value: unknown): string => {
  if (!value || typeof value !== "object") {
    return "";
  }
  const coords = value as { x?: unknown; y?: unknown };
  return (typeof coords.x === "number" || typeof coords.x === "string") &&
    (typeof coords.y === "number" || typeof coords.y === "string")
    ? `${coords.x},${coords.y}`
    : "";
};

const surfacePayload = (
  payload: Record<string, any> | undefined,
): Record<string, any> | undefined => {
  const surface =
    payload?.surface ?? payload?.status?.surface ?? payload?.ctx?.surface;
  return surface && typeof surface === "object" ? surface : undefined;
};

const surfaceValue = (
  payload: Record<string, any> | undefined,
  key: string,
  snakeKey?: string,
): string | undefined => {
  const surface = surfacePayload(payload);
  const value = surface?.[key] ?? (snakeKey ? surface?.[snakeKey] : undefined);
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
};

const normalizeSurfaceKind = (
  value: string | undefined,
): string | undefined => {
  const normalized = value
    ?.trim()
    .toLowerCase()
    .replace(/['’]/g, "")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  return normalized || undefined;
};

const isBootSurfaceKind = (kind: string | undefined): boolean =>
  Boolean(kind && !["overworld", "battle", "menu"].includes(kind));

const hasPromptCursor = (lines: string[]): boolean =>
  lines.some((line) => /^\s*(?:>|▶|▷)/.test(line));

const normalizePcPromptText = (value: unknown): string =>
  String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/<pk>\s*<mn>/g, "pokemon")
    .replace(/pok[eé]mon/g, "pokemon")
    .replace(/[?.!]+$/g, "")
    .replace(/\s+/g, " ");

const isPcMenuText = (value: unknown): boolean => {
  const text = normalizePcPromptText(value)
    .replace(/^[>▶▷]\s*/, "")
    .replace(/#mon/g, "pokemon");
  return (
    text.includes("bill's pc") ||
    text.includes("chris's pc") ||
    text.includes("withdraw pokemon") ||
    text.includes("deposit pokemon") ||
    text.includes("move pokemon w/o mail") ||
    text.includes("change box") ||
    text.includes("see ya") ||
    text.includes("turn off")
  );
};

const isGenericPromptViewport = (lines: string[]): boolean =>
  lines.length <= 1 &&
  lines.every((line) => {
    const normalized = line.trim().toLowerCase();
    return normalized === "" || normalized === "prompt";
  });

const isPcInstructionPrompt = (
  prompt: string[],
  surfaceKind: string | undefined,
  surfaceTitle: string | undefined,
  mode: string,
  map: string,
): boolean => {
  if (!prompt.length || hasPromptCursor(prompt)) {
    return false;
  }
  const title = surfaceTitle?.toLowerCase() ?? "";
  const normalizedMode = mode.toLowerCase();
  const normalizedMap = map.toLowerCase();
  return (
    surfaceKind === "pc" ||
    title.includes("pc") ||
    normalizedMode === "pc" ||
    normalizedMap === "pc" ||
    normalizedMap.includes("bill")
  );
};

const readInfoValue = (lines: string[], label: string): string | undefined => {
  const prefix = `${label.toUpperCase()}:`;
  const line = lines.find((entry) =>
    entry.trim().toUpperCase().startsWith(prefix),
  );
  return line ? line.slice(line.indexOf(":") + 1).trim() : undefined;
};

const readNameEntryCursor = (
  lines: string[],
): { row: number; column: number } => {
  const match = lines
    .find((line) => /^CURSOR:/i.test(line.trim()))
    ?.match(/row\s+(\d+)\s+col\s+(\d+)/i);
  return {
    row: Math.max(
      0,
      Math.min(NAME_ENTRY_ROW_COUNT - 1, Number(match?.[1] ?? 0)),
    ),
    column: Math.max(
      0,
      Math.min(NAME_ENTRY_COLUMN_COUNT - 1, Number(match?.[2] ?? 0)),
    ),
  };
};

const buildNameSlots = (
  name: string,
  lengthText: string | undefined,
): string => {
  const lengthMatch = lengthText?.match(/(\d+)\s*\/\s*(\d+)/);
  const maxLength = Math.max(
    1,
    Math.min(12, Number(lengthMatch?.[2] ?? Math.max(5, name.length || 5))),
  );
  const chars = name === "(blank)" ? [] : [...name];
  return Array.from(
    { length: maxLength },
    (_value, index) => chars[index] ?? "_",
  ).join(" ");
};

const buildNameEntryRule = (lengthText: string | undefined): string => {
  const lengthMatch = lengthText?.match(/(\d+)\s*\/\s*(\d+)/);
  const maxLength = Math.max(
    1,
    Math.min(12, Number(lengthMatch?.[2] ?? 7)),
  );
  return `${"^ ".repeat(maxLength).trimEnd()}  ${lengthText ?? ""}`.trimEnd();
};

const resolveSelectedChar = (
  row: number,
  column: number,
  layout: string[],
): string => {
  if (row === NAME_ENTRY_ROW_COUNT - 1) {
    if (column < 3) {
      const bottomRow = layout[row] ?? "";
      return bottomRow.includes("lower")
        ? "lower"
        : bottomRow.includes("UPPER")
          ? "UPPER"
          : "lower/UPPER";
    }
    if (column < 6) return "DEL";
    return "END";
  }
  const rowString = layout[row];
  if (!rowString) return "(empty)";
  const char = rowString.charAt(column * 2);
  return char && char.trim() ? char : "(empty)";
};

const nameEntryTokenIndex = (
  row: number,
  column: number,
): number => row === NAME_ENTRY_ROW_COUNT - 1 ? Math.floor(column / 3) : column;

const buildHighlightedNameEntryRow = (
  line: string,
  row: number,
  cursor: { row: number; column: number },
): string => {
  const tokens = Array.from(line.matchAll(/\S+/g)).map((match) => match[0]);
  if (!tokens.length) {
    return line;
  }
  const selectedTokenIndex = row === cursor.row
    ? nameEntryTokenIndex(row, cursor.column)
    : -1;
  return tokens
    .map((token, index) => index === selectedTokenIndex ? `[${token}]` : ` ${token} `)
    .join(" ")
    .replace(/\s+$/g, "");
};

const nameEntryLayoutLines = (menu: string[]): string[] =>
  (menu.length ? menu : NAME_ENTRY_ROWS)
    .filter((line) => !line.includes("▲"))
    .slice(0, NAME_ENTRY_ROW_COUNT);

const buildNameEntryKeyboardLines = (
  menu: string[],
  cursor: { row: number; column: number },
): string[] => {
  const layoutLines = nameEntryLayoutLines(menu);
  const lines: string[] = [];
  for (let rowIndex = 0; rowIndex < NAME_ENTRY_ROW_COUNT; rowIndex += 1) {
    const line = layoutLines[rowIndex] ?? NAME_ENTRY_ROWS[rowIndex] ?? "";
    lines.push(buildHighlightedNameEntryRow(line, rowIndex, cursor));
  }
  return lines;
};

const buildNameEntryViewport = (
  viewport: string[],
  info: string[],
  menu: string[],
): string[] => {
  const prompt =
    readInfoValue(info, "PROMPT") ??
    viewport.find((line) => line && line !== "NAME ENTRY") ??
    "NAME";
  const name = readInfoValue(info, "NAME") ?? "";
  const lengthText = readInfoValue(info, "LENGTH");
  const cursor = readNameEntryCursor(info);

  const output = [
    "NAME ENTRY",
    prompt.toUpperCase(),
    `NAME    ${buildNameSlots(name, lengthText)}`,
    `        ${buildNameEntryRule(lengthText)}`,
    "TYPE    letters enter directly; Backspace deletes; End confirms",
    "KEYS    arrows move cursor; Space selects",
    "KEYBOARD",
  ].filter((line) => line.length > 0);

  const keyboardLines = buildNameEntryKeyboardLines(menu, cursor);
  output.push(...keyboardLines);

  const layoutLines = nameEntryLayoutLines(menu);
  output.push(
    `SELECTED ${resolveSelectedChar(cursor.row, cursor.column, layoutLines)}`,
  );
  return output;
};

const recentLines = (
  recentEvents?: ToolResult,
  observePayload?: Record<string, any>,
): string[] => {
  const recentPayload = firstJson(recentEvents);
  const events = Array.isArray(recentPayload?.events)
    ? recentPayload.events
    : [];
  const lines = events
    .map((event: Record<string, unknown>) =>
      typeof event.summary === "string"
        ? event.summary
        : typeof event.action === "string"
          ? event.action
          : "",
    )
    .filter(Boolean)
    .slice(-6);
  const recap =
    typeof recentPayload?.recap === "string"
      ? recentPayload.recap
      : typeof observePayload?.r?.sum === "string"
        ? observePayload.r.sum
        : undefined;
  if (!lines.length && recap) {
    return [recap];
  }
  return lines;
};

export const normalizeTuiSnapshot = (
  observeResult: ToolResult,
  statusResult?: ToolResult,
  recentEventsResult?: ToolResult,
  dialogueAccumulator: DialogueAccumulator = createDialogueAccumulator(),
): TuiSnapshot => {
  const observePayload = firstJson(observeResult);
  const statusPayload = firstJson(statusResult);
  const view = observePayload?.view ?? {};
  const ctx =
    observePayload?.ctx ??
    statusPayload?.ctx ??
    statusPayload?.status ??
    statusPayload ??
    {};
  const fallback = extractFallbackSections(fullText(observeResult));

  const rawViewport = asLines(view.viewport).length
    ? asLines(view.viewport)
    : fallback.viewport;
  const rawInfo = asLines(view.info).length
    ? asLines(view.info)
    : asLines(observePayload?.info).length
      ? asLines(observePayload?.info)
      : fallback.info;
  const split = splitInfoAndLegend(rawInfo);
  const menu = Array.isArray(view.menu?.items)
    ? view.menu.items.filter(
        (entry: unknown): entry is string => typeof entry === "string",
      )
    : Array.isArray(view.menu)
      ? view.menu.filter(
          (entry: unknown): entry is string => typeof entry === "string",
        )
      : fallback.menu;
  let prompt = asLines(view.prompt).length
    ? asLines(view.prompt)
    : fallback.prompt;
  const structuredDialogue = dialogueLines(view.dialogue);
  const rawDialogue = structuredDialogue.length
    ? structuredDialogue
    : fallback.dialogue.filter((line) => !isDialogueBoilerplateLine(line));
  const statusPrompt =
    statusPayload?.prompt ??
    statusPayload?.status?.prompt ??
    statusPayload?.ctx?.prompt;
  const statusPromptLines = asLines(statusPrompt?.lines ?? statusPrompt?.items);
  const promptPending =
    statusValue(statusPayload, "prompt_pending", "pr") === true ||
    statusPrompt?.pending === true ||
    ctx.pr === 1;
  if (!prompt.length && statusPromptLines.length) {
    prompt = statusPromptLines;
  } else if (!prompt.length && promptPending && rawDialogue.length) {
    prompt = ["▶ YES", "  NO"];
  }
  const dialogueKey = rawDialogue.join("|") || null;
  if (rawDialogue.length) {
    if (dialogueAccumulator.lastKey !== dialogueKey) {
      dialogueAccumulator.pages.push(rawDialogue);
      dialogueAccumulator.lastKey = dialogueKey;
    }
  } else {
    dialogueAccumulator.pages = [];
    dialogueAccumulator.lastKey = null;
  }
  const dialogue = dialogueAccumulator.pages.length
    ? dialogueAccumulator.pages.flatMap((page, index) =>
        index > 0 ? ["", ...page] : page,
      )
    : rawDialogue;

  const viewportLines = rawViewport.map(stripAnsi);
  const infoLines = split.info.map(stripAnsi);
  const statusSurfaceKind = normalizeSurfaceKind(
    surfaceValue(statusPayload, "kind"),
  );
  const observeSurfaceKind = normalizeSurfaceKind(
    surfaceValue(observePayload, "kind"),
  );
  const surfaceKind = statusSurfaceKind ?? observeSurfaceKind;
  const surfaceTitle =
    surfaceValue(statusPayload, "title") ??
    surfaceValue(observePayload, "title");
  const pcMenuSurface =
    surfaceKind === "pc" ||
    Boolean(surfaceTitle?.toLowerCase().includes("pc")) ||
    menu.some(isPcMenuText) ||
    isPcMenuText(
      surfaceValue(statusPayload, "selected") ??
        surfaceValue(observePayload, "selected"),
    ) ||
    isPcMenuText(
      surfaceValue(statusPayload, "primaryText", "primary_text") ??
        surfaceValue(observePayload, "primaryText", "primary_text"),
    );
  const effectiveSurfaceKind = pcMenuSurface ? "pc" : surfaceKind;
  const effectiveSurfaceTitle = pcMenuSurface ? "Bill's PC" : surfaceTitle;
  const isTitleScreen = viewportLines.some(
    (line) => line.trim().toUpperCase() === "TITLE SCREEN",
  );
  const mode = isTitleScreen
    ? "title"
    : pcMenuSurface
      ? "pc"
    : isBootSurfaceKind(effectiveSurfaceKind)
      ? effectiveSurfaceKind!
      : String(
          statusValue(statusPayload, "mode", "m") ??
            ctx.m ??
            view.focus ??
            "unknown",
        );
  const map = isTitleScreen
    ? "TITLE"
    : pcMenuSurface
      ? "BILL'S PC"
    : isBootSurfaceKind(effectiveSurfaceKind)
      ? (effectiveSurfaceTitle ?? effectiveSurfaceKind!).toUpperCase()
      : String(statusValue(statusPayload, "map") ?? ctx.map ?? "-");
  const pcInstructionPrompt = isPcInstructionPrompt(
    prompt,
    effectiveSurfaceKind,
    effectiveSurfaceTitle,
    mode,
    map,
  );
  const promptStatus = prompt.length
    ? pcInstructionPrompt
      ? "idle"
      : "prompt"
    : ctx.pr === 1
      ? "prompt"
      : rawDialogue.length
        ? "dialogue"
        : "idle";
  const coords = statusValue(statusPayload, "coords");
  const xy = isTitleScreen
    ? ""
    : Array.isArray(ctx.xy)
      ? ctx.xy.join(",")
      : coordsText(coords);
  const frame =
    frameValue(observePayload) !== "-"
      ? frameValue(observePayload)
      : frameValue(statusPayload);
  const actions = recentLines(recentEventsResult, observePayload);
  const isNameEntry =
    split.info.some((line) => /^STATE:\s*name_entry/i.test(line.trim())) ||
    rawViewport.some((line) => line.trim().toUpperCase() === "NAME ENTRY");
  const viewport = isNameEntry
    ? buildNameEntryViewport(viewportLines, infoLines, menu.map(stripAnsi))
    : pcMenuSurface && isGenericPromptViewport(viewportLines)
      ? ["BILL'S PC"]
      : viewportLines;

  return {
    mode,
    map,
    surface: effectiveSurfaceKind ?? mode,
    promptStatus,
    frame,
    viewport,
    info: split.info.length
      ? split.info
      : [`Mode: ${mode}`, `Map: ${map}`, ...(xy ? [`Coords: ${xy}`] : [])],
    legend: [...split.legend, ...fallback.legend],
    menu,
    prompt,
    dialogue,
    actions,
    statusLine: [
      `STATE: ${mode}`,
      effectiveSurfaceKind && effectiveSurfaceKind !== mode ? `SURFACE: ${effectiveSurfaceKind}` : null,
      `MAP: ${map}`,
      `PROMPT: ${promptStatus}`,
      frame !== "-" ? `FRAME #${frame}` : null,
      xy ? `XY: ${xy}` : null,
    ]
      .filter((entry): entry is string => Boolean(entry))
      .join(" | "),
  };
};
