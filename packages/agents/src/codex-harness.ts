import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { createInterface, type Interface as ReadlineInterface } from "node:readline";
import os from "node:os";
import path from "node:path";

import {
  taskmasterBatchSchema,
  type RunnerInput,
  type SessionAuth,
  type Status,
  type TaskmasterBatch,
} from "./types.js";
import { buildMcpUrl, type KrabbyClawSession } from "./session.js";
import {
  emitAgentStreamEvent,
  emitAgentStreamStatus,
  shouldEmitAgentStreamEvents,
  type AgentStreamEvent,
} from "./stream-events.js";
import { BUTTON_PROMPT_GUIDANCE } from "./prompts.js";

type JsonPrimitive = null | boolean | number | string;
type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };
type JsonObject = { [key: string]: JsonValue };

type RpcRequest = {
  id?: number | string;
  method: string;
  params?: JsonValue;
};

type RpcResponse = {
  id: number | string;
  result?: JsonValue;
  error?: {
    code?: number;
    message: string;
    data?: JsonValue;
  };
};

type RpcMessage = RpcRequest | RpcResponse;

type CodexThreadStartResponse = {
  thread: {
    id: string;
  };
};

type CodexThreadResumeResponse = CodexThreadStartResponse;

type CodexTurnStartResponse = {
  turn: {
    id: string;
  };
};

type CodexThreadReadResponse = {
  thread?: {
    turns?: Array<{
      id?: string;
      status?: string;
      items?: JsonValue;
      error?: JsonValue;
    }>;
  };
};

type CodexServerNotification = {
  method: string;
  params?: JsonValue;
};

type CodexDynamicToolCallParams = {
  threadId: string;
  turnId: string;
  callId: string;
  tool: string;
  arguments?: JsonValue;
};

type CodexDynamicToolCallResponse = {
  contentItems: Array<
    | {
        type: "inputText";
        text: string;
      }
    | {
        type: "inputImage";
        imageUrl: string;
      }
  >;
  success: boolean;
};

type CodexDynamicToolSpec = {
  name: string;
  description: string;
  inputSchema: JsonValue;
};

type CodexThreadBinding = {
  threadId: string;
  sessionId: string;
  model: string;
  updatedAt: string;
};

type PendingRequest = {
  resolve: (value: unknown) => void;
  reject: (error: Error) => void;
};

type CodexToolDefinition = {
  spec: CodexDynamicToolSpec;
  run: (args: Record<string, unknown>) => Promise<string>;
};

class CodexCliRunError extends Error {
  constructor(message: string, readonly progressMade: boolean) {
    super(message);
    this.name = "CodexCliRunError";
  }
}

const CODEX_HOME = process.env.CODEX_HOME?.trim()
  ? path.resolve(process.env.CODEX_HOME.trim())
  : path.join(process.env.HOME ?? process.cwd(), ".codex");
const CODEX_BINDINGS_DIR = path.join(CODEX_HOME, "pokecrystal-agent-bindings");
const DEFAULT_CODEX_TURN_TIMEOUT_MS = 90_000;
const CODEX_CLI_OUTPUT_LIMIT = 48_000;

export function getCodexTurnTimeoutMs(): number {
  const raw = process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS?.trim();
  if (!raw) {
    return DEFAULT_CODEX_TURN_TIMEOUT_MS;
  }
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed < 5_000) {
    throw new Error(
      "POKECRYSTAL_CODEX_TURN_TIMEOUT_MS must be an integer >= 5000 when set."
    );
  }
  return parsed;
}

export function isCodexModel(model: string): boolean {
  return model.trim().toLowerCase().startsWith("codex/");
}

export function extractJsonObjectFromText(text: string): string {
  const trimmed = text.trim();
  if (trimmed.startsWith("{") && trimmed.endsWith("}")) {
    return trimmed;
  }

  const fenceMatch = trimmed.match(/```(?:json)?\s*([\s\S]*?)```/i);
  if (fenceMatch?.[1]) {
    const fenced = fenceMatch[1].trim();
    if (fenced.startsWith("{") && fenced.endsWith("}")) {
      return fenced;
    }
  }

  const start = trimmed.indexOf("{");
  if (start < 0) {
    throw new Error("Codex response did not include a JSON object.");
  }

  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < trimmed.length; index += 1) {
    const char = trimmed[index] ?? "";
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (char === "\\") {
        escaped = true;
      } else if (char === "\"") {
        inString = false;
      }
      continue;
    }

    if (char === "\"") {
      inString = true;
      continue;
    }
    if (char === "{") {
      depth += 1;
      continue;
    }
    if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return trimmed.slice(start, index + 1);
      }
    }
  }

  throw new Error("Codex response included an incomplete JSON object.");
}

export function normalizeCodexMoveArgs(args: Record<string, unknown>): {
  direction: "up" | "down" | "left" | "right";
  steps: number;
} {
  const direction =
    args.direction === "up" ||
    args.direction === "down" ||
    args.direction === "left" ||
    args.direction === "right"
      ? args.direction
      : "down";
  const steps =
    typeof args.steps === "number" && Number.isInteger(args.steps)
      ? Math.max(1, Math.min(4, args.steps))
      : 1;
  return { direction, steps };
}

export function normalizeCodexButtonArgs(args: Record<string, unknown>): {
  button: "A" | "B" | "Start" | "Select" | "Up" | "Down" | "Left" | "Right";
} {
  const normalized = typeof args.button === "string" ? args.button.trim().toLowerCase() : "";
  const buttonByName = {
    a: "A",
    b: "B",
    start: "Start",
    select: "Select",
    up: "Up",
    down: "Down",
    left: "Left",
    right: "Right",
  } as const;
  const button = buttonByName[normalized as keyof typeof buttonByName] ?? "A";
  return { button };
}

export function normalizeCodexFrameCount(
  value: unknown,
  fallback: number,
  maximum: number,
): number {
  if (typeof value === "number" && Number.isInteger(value)) {
    return Math.max(1, Math.min(maximum, value));
  }
  return fallback;
}

export function readCodexVisibleActionReason(args: Record<string, unknown>): string | null {
  const reason = typeof args.reason === "string" ? args.reason.trim() : "";
  return reason || null;
}

function emitCodexVisibleActionReason(args: Record<string, unknown>): void {
  const reason = readCodexVisibleActionReason(args);
  if (!reason) {
    throw new Error("Action tool calls require a non-empty visible reason.");
  }
  if (shouldEmitAgentStreamEvents()) {
    emitAgentStreamEvent({ type: "thinking-delta", text: reason, source: "player" });
  }
}

function didStatusChange(before: Status, after: Status): boolean {
  return JSON.stringify({
    map: before.map,
    coords: before.coords,
    facing: before.facing,
    surface: before.surface,
    flowNextGoal: before.flowNextGoal,
    badges: before.badges,
    partyCount: before.partyCount,
  }) !==
    JSON.stringify({
      map: after.map,
      coords: after.coords,
      facing: after.facing,
      surface: after.surface,
      flowNextGoal: after.flowNextGoal,
      badges: after.badges,
      partyCount: after.partyCount,
    });
}

type CodexMapInfoSummary = {
  map?: string;
  player?: {
    x?: number;
    y?: number;
    facing?: string;
    coords?: { x: number; y: number };
  };
  hotspots?: Array<{
    type?: string;
    label?: string;
    coords?: { x: number; y: number };
    approach_tiles?: Array<{ coords?: { x: number; y: number }; facing?: string }>;
    visible?: boolean;
    interactable?: boolean;
  }>;
};

const codexHotspotPriority = (type: string | undefined): number => {
  switch (type) {
    case "warp":
      return 0;
    case "objective":
      return 1;
    case "npc":
      return 2;
    case "heal":
    case "shop":
    case "gym":
      return 3;
    case "utility":
    case "trigger":
      return 4;
    case "sign":
    case "landmark":
      return 5;
    default:
      return 6;
  }
};

const formatRouteOffset = (
  player: { x: number; y: number } | undefined,
  coords: { x: number; y: number } | undefined,
): string | undefined => {
  if (!player || !coords) {
    return undefined;
  }
  const dx = coords.x - player.x;
  const dy = coords.y - player.y;
  if (dx === 0 && dy === 0) {
    return "here";
  }
  const parts: string[] = [];
  if (dy !== 0) {
    parts.push(`${Math.abs(dy)}${dy < 0 ? "N" : "S"}`);
  }
  if (dx !== 0) {
    parts.push(`${Math.abs(dx)}${dx < 0 ? "W" : "E"}`);
  }
  return parts.join(" ");
};

export function summarizeCodexRouteTarget(mapInfoText?: string): string | undefined {
  if (!mapInfoText?.trim()) {
    return undefined;
  }

  let parsed: CodexMapInfoSummary | null = null;
  try {
    parsed = JSON.parse(mapInfoText) as CodexMapInfoSummary;
  } catch {
    return undefined;
  }

  const player =
    parsed?.player?.x !== undefined && parsed?.player?.y !== undefined
      ? parsed.player
      : parsed?.player?.coords
        ? {
            x: parsed.player.coords.x,
            y: parsed.player.coords.y,
            facing: parsed.player.facing,
          }
        : undefined;
  const resolvedPlayer =
    player && player.x !== undefined && player.y !== undefined
      ? { x: player.x, y: player.y, facing: player.facing }
      : undefined;

  const hotspots = Array.isArray(parsed?.hotspots)
    ? parsed.hotspots
        .filter((hotspot): hotspot is NonNullable<CodexMapInfoSummary["hotspots"]>[number] =>
          Boolean(hotspot && hotspot.visible !== false && hotspot.interactable !== false && hotspot.coords),
        )
        .sort((left, right) => {
          const priorityDelta = codexHotspotPriority(left.type) - codexHotspotPriority(right.type);
          if (priorityDelta !== 0) {
            return priorityDelta;
          }
          const leftDistance =
            resolvedPlayer && left.coords
              ? Math.abs(left.coords.x - resolvedPlayer.x) + Math.abs(left.coords.y - resolvedPlayer.y)
              : Number.MAX_SAFE_INTEGER;
          const rightDistance =
            resolvedPlayer && right.coords
              ? Math.abs(right.coords.x - resolvedPlayer.x) + Math.abs(right.coords.y - resolvedPlayer.y)
              : Number.MAX_SAFE_INTEGER;
          return leftDistance - rightDistance;
        })
    : [];

  const target = hotspots[0];
  if (!target?.coords) {
    return undefined;
  }

  const offset = formatRouteOffset(resolvedPlayer, target.coords);
  return [
    `Highest-priority visible route target: ${target.label ?? target.type ?? "unknown"} (${target.type ?? "unknown"}).`,
    offset ? `Current offset: ${offset}.` : undefined,
    "Prefer this target over ambient utility/sign hotspots unless newer live state elevates them; if the route is unclear, sample a fresh nearby NPC/sign/object for clues before wandering.",
  ]
    .filter(Boolean)
    .join(" ");
}

function isJsonObject(value: JsonValue | undefined): value is JsonObject {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}

function escapeTomlString(value: string): string {
  return value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
}

function formatTomlKey(key: string): string {
  return /^[A-Za-z0-9_-]+$/.test(key) ? key : `"${escapeTomlString(key)}"`;
}

function serializeTomlInlineValue(value: unknown): string {
  if (typeof value === "string") {
    return `"${escapeTomlString(value)}"`;
  }
  if (typeof value === "number" || typeof value === "bigint") {
    return String(value);
  }
  if (typeof value === "boolean") {
    return value ? "true" : "false";
  }
  if (Array.isArray(value)) {
    return `[${value.map(entry => serializeTomlInlineValue(entry)).join(", ")}]`;
  }
  if (value && typeof value === "object") {
    return `{ ${Object.entries(value as Record<string, unknown>)
      .map(([key, entry]) => `${formatTomlKey(key)} = ${serializeTomlInlineValue(entry)}`)
      .join(", ")} }`;
  }
  throw new Error(`Unsupported TOML inline value: ${String(value)}`);
}

function appendLimitedCodexOutput(current: string, chunk: Buffer | string): string {
  const next = `${current}${String(chunk)}`;
  return next.length > CODEX_CLI_OUTPUT_LIMIT
    ? next.slice(next.length - CODEX_CLI_OUTPUT_LIMIT)
    : next;
}

function collectCodexCliTextFromValue(value: unknown): string {
  if (!value) {
    return "";
  }
  if (typeof value === "string") {
    return value;
  }
  if (Array.isArray(value)) {
    return value.map(entry => collectCodexCliTextFromValue(entry)).join("");
  }
  if (!value || typeof value !== "object") {
    return "";
  }
  const record = value as Record<string, unknown>;
  if (typeof record.text === "string") {
    return record.text;
  }
  if (typeof record.result === "string") {
    return record.result;
  }
  if (typeof record.response === "string") {
    return record.response;
  }
  if (typeof record.content === "string") {
    return record.content;
  }
  if (Array.isArray(record.content)) {
    return record.content.map(entry => collectCodexCliTextFromValue(entry)).join("");
  }
  if (record.message && typeof record.message === "object") {
    return collectCodexCliTextFromValue(record.message);
  }
  return "";
}

type CodexCliStreamAccumulator = {
  textByItemId: Map<string, string>;
};

const createCodexCliStreamAccumulator = (): CodexCliStreamAccumulator => ({
  textByItemId: new Map(),
});

const readRecordString = (record: Record<string, unknown> | null | undefined, key: string): string | undefined => {
  const value = record?.[key];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
};

const normalizeCodexStreamType = (value: string | undefined): string =>
  value?.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_") ?? "";

const readCodexStreamItem = (parsed: Record<string, unknown>): Record<string, unknown> | null =>
  parsed.item && typeof parsed.item === "object" && !Array.isArray(parsed.item)
    ? parsed.item as Record<string, unknown>
    : null;

const readCodexStreamText = (
  parsed: Record<string, unknown>,
  item: Record<string, unknown> | null,
): string => {
  for (const [record, keys] of [
    [parsed, ["delta", "text"]],
    [item, ["delta", "text", "reasoning", "summary"]],
  ] as Array<[Record<string, unknown> | null, string[]]>) {
    if (!record) {
      continue;
    }
    for (const key of keys) {
      const value = record[key];
      if (typeof value === "string" && value) {
        return value;
      }
    }
  }

  return collectCodexCliTextFromValue(
    item?.content ?? parsed.content ?? parsed.message ?? item?.message
  );
};

const codexStreamItemKey = (
  parsed: Record<string, unknown>,
  item: Record<string, unknown> | null,
): string | null =>
  readRecordString(item, "id") ??
  readRecordString(parsed, "item_id") ??
  readRecordString(parsed, "itemId") ??
  readRecordString(parsed, "id") ??
  null;

const deltaForCodexStreamText = (
  accumulator: CodexCliStreamAccumulator,
  key: string | null,
  text: string,
  isExplicitDelta: boolean,
): string => {
  if (!text) {
    return "";
  }
  if (!key || isExplicitDelta) {
    return text;
  }

  const previous = accumulator.textByItemId.get(key) ?? "";
  accumulator.textByItemId.set(key, text);
  if (!previous) {
    return text;
  }
  if (text.startsWith(previous)) {
    return text.slice(previous.length);
  }
  if (text === previous) {
    return "";
  }
  return text;
};

const readCodexToolName = (
  parsed: Record<string, unknown>,
  item: Record<string, unknown> | null,
): string | undefined => {
  const directName =
    readRecordString(item, "name") ??
    readRecordString(item, "tool") ??
    readRecordString(item, "toolName") ??
    readRecordString(item, "tool_name") ??
    readRecordString(item, "server_tool_name") ??
    readRecordString(parsed, "name") ??
    readRecordString(parsed, "tool") ??
    readRecordString(parsed, "toolName") ??
    readRecordString(parsed, "tool_name");
  if (directName) {
    return directName;
  }

  const functionRecord =
    item?.function && typeof item.function === "object" && !Array.isArray(item.function)
      ? item.function as Record<string, unknown>
      : parsed.function && typeof parsed.function === "object" && !Array.isArray(parsed.function)
      ? parsed.function as Record<string, unknown>
      : null;
  const functionName = readRecordString(functionRecord, "name");
  if (functionName) {
    return functionName;
  }

  const server = readRecordString(item, "server") ?? readRecordString(parsed, "server");
  const serverTool = readRecordString(item, "server_tool") ?? readRecordString(parsed, "server_tool");
  return server && serverTool ? `${server}.${serverTool}` : serverTool;
};

export function agentStreamEventsFromTaskmasterBatch(
  batch: TaskmasterBatch,
  source = "codex",
): AgentStreamEvent[] {
  const evidence = batch.evidence.length ? ` Evidence: ${batch.evidence.join(" | ")}` : "";
  return [
    {
      type: "thinking-delta",
      text: `Decision: ${batch.summary}${evidence}`,
      source,
    },
    {
      type: "text-delta",
      text: `Goal ${batch.immediateGoalStatus}; next: ${batch.nextImmediateGoal}; continue: ${batch.shouldContinue}.`,
      source,
    },
  ];
}

function emitTaskmasterBatchStreamSummary(batch: TaskmasterBatch, source?: string): void {
  if (!shouldEmitAgentStreamEvents()) {
    return;
  }
  for (const event of agentStreamEventsFromTaskmasterBatch(batch, source)) {
    emitAgentStreamEvent(event);
  }
}

function streamTaskmasterBatch(batch: TaskmasterBatch, source?: string): TaskmasterBatch {
  emitTaskmasterBatchStreamSummary(batch, source);
  return batch;
}

export function codexCliStreamEventFromJsonLine(
  line: string,
  accumulator: CodexCliStreamAccumulator = createCodexCliStreamAccumulator(),
): AgentStreamEvent | null {
  let parsed: Record<string, unknown>;
  try {
    const candidate = JSON.parse(line.trim()) as unknown;
    if (!candidate || typeof candidate !== "object" || Array.isArray(candidate)) {
      return null;
    }
    parsed = candidate as Record<string, unknown>;
  } catch {
    return null;
  }

  const item = readCodexStreamItem(parsed);
  const outerType = normalizeCodexStreamType(readRecordString(parsed, "type"));
  const itemType = normalizeCodexStreamType(readRecordString(item, "type"));
  const combinedType = `${outerType} ${itemType}`;
  const toolName = readCodexToolName(parsed, item);

  if (toolName && /mcp_tool_call|tool_call|function_call/.test(combinedType)) {
    const status = normalizeCodexStreamType(readRecordString(item, "status") ?? readRecordString(parsed, "status"));
    if (/completed|success|failed|error/.test(status) || /completed|result/.test(outerType)) {
      return {
        type: "mcp-result",
        name: toolName,
        summary: status || undefined,
        source: "codex",
      };
    }
    return {
      type: "mcp-call",
      name: toolName,
      source: "codex",
    };
  }

  const text = readCodexStreamText(parsed, item);
  if (!text) {
    return null;
  }
  const key = codexStreamItemKey(parsed, item);
  const isExplicitDelta = /delta/.test(outerType) || typeof parsed.delta === "string" || typeof item?.delta === "string";
  const delta = deltaForCodexStreamText(accumulator, key, text, isExplicitDelta);
  if (!delta) {
    return null;
  }

  if (/reasoning|thinking/.test(combinedType)) {
    return { type: "thinking-delta", text: delta, source: "codex" };
  }
  if (/agent_message|assistant|message|output_text|response/.test(combinedType)) {
    return { type: "text-delta", text: delta, source: "codex" };
  }
  return null;
}

function parseCodexCliJsonl(raw: string): { text: string; threadId?: string; error?: string } {
  let threadId: string | undefined;
  let error: string | undefined;
  const texts: string[] = [];

  for (const line of raw.split(/\r?\n/g)) {
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    let parsed: Record<string, unknown>;
    try {
      const candidate = JSON.parse(trimmed) as unknown;
      if (!candidate || typeof candidate !== "object" || Array.isArray(candidate)) {
        continue;
      }
      parsed = candidate as Record<string, unknown>;
    } catch {
      continue;
    }

    if (typeof parsed.thread_id === "string" && parsed.thread_id.trim()) {
      threadId = parsed.thread_id.trim();
    }
    const item = parsed.item && typeof parsed.item === "object" && !Array.isArray(parsed.item)
      ? parsed.item as Record<string, unknown>
      : null;
    if (item && typeof item.text === "string") {
      const type = typeof item.type === "string" ? item.type.toLowerCase() : "";
      if (!type || type.includes("message")) {
        texts.push(item.text);
      }
    }
    if (parsed.type === "result") {
      const resultText = collectCodexCliTextFromValue(parsed.result).trim();
      if (resultText) {
        texts.push(resultText);
      }
    }
    if (parsed.type === "error") {
      error = collectCodexCliTextFromValue(parsed.message) ||
        collectCodexCliTextFromValue(parsed.error) ||
        collectCodexCliTextFromValue(parsed);
    }
  }

  return { text: texts.join("\n").trim(), threadId, error };
}

function buildCodexCliMcpConfig(auth: SessionAuth): {
  config: Record<string, unknown>;
  env: Record<string, string>;
} {
  if (auth.mcpUrl) {
    return {
      config: {
        krabbyclaw: {
          url: auth.mcpUrl,
        },
      },
      env: {},
    };
  }

  return {
    config: {
      krabbyclaw: {
        url: buildMcpUrl(auth.baseUrl, auth.sessionId).toString(),
        bearer_token_env_var: "POKECRYSTAL_MCP_TOKEN",
        env_http_headers: {
          "x-session-secret": "POKECRYSTAL_MCP_SESSION_SECRET",
        },
      },
    },
    env: {
      POKECRYSTAL_MCP_TOKEN: auth.token,
      POKECRYSTAL_MCP_SESSION_SECRET: auth.sessionSecret,
    },
  };
}

async function runCodexExec(params: {
  input: RunnerInput;
  prompt: string;
  sessionAuth: SessionAuth;
}): Promise<string> {
  const tempDir = await mkdtemp(path.join(os.tmpdir(), "pokecrystal-codex-cli-"));
  const systemPromptPath = path.join(tempDir, "instructions.md");
  const mcp = buildCodexCliMcpConfig(params.sessionAuth);
  await writeFile(
    systemPromptPath,
    [
      "You are a self-managed Pokemon Crystal gameplay agent.",
      "Use the krabbyclaw MCP server tools to observe state and play the game.",
      "Do not use shell commands, file edits, route scripts, save editing, or engine internals.",
      "Before every gameplay action tool call (move, press, hold_button), write a concise visible rationale in <think>...</think> explaining the live evidence and intended effect.",
      "Use objective operational language only. Do not use fictional framing, personal flourish, or decorative narration.",
      "Use flow_state as the sequential backbone for beating the game, and treat everything encountered on the route to that flow goal as important route evidence: NPCs, signs, item balls, prompts, battles, doors, warps, blockers, and local clues may be the actual next step.",
      "Treat NPC requests, sign clues, item hints, and unique object discoveries as first-class goals alongside flow_state; note what was learned and let actionable clues shape the next objective.",
      BUTTON_PROMPT_GUIDANCE,
      "After receiving the Mystery Egg and returning it to Elm, clear the New Bark Mom handoff before routing to Violet: talk to Mom in Player's House 1F and decline/cancel her money-saving or bank prompt unless the current goal explicitly asks to bank.",
      "If external intervention appears, treat it as manual live play that already happened and continue from the updated current state.",
      "Keep advancing toward the requested goal and finish each batch with the requested JSON checkpoint.",
    ].join("\n"),
    "utf8",
  );

  const args = [
    "exec",
    "--json",
    "--color",
    "never",
    "--dangerously-bypass-approvals-and-sandbox",
    "--skip-git-repo-check",
    "--model",
    params.input.taskmasterModel.replace(/^codex\//i, ""),
    "-c",
    `model_instructions_file=${serializeTomlInlineValue(systemPromptPath)}`,
    "-c",
    `mcp_servers=${serializeTomlInlineValue(mcp.config)}`,
    params.prompt,
  ];

  try {
    emitAgentStreamStatus("codex exec turn starting", "codex");
    const child = spawn("codex", args, {
      cwd: process.cwd(),
      env: {
        ...process.env,
        ...mcp.env,
      },
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    const streamAccumulator = createCodexCliStreamAccumulator();
    let pendingStdout = "";
    const handleStdoutStreamChunk = (chunk: Buffer | string): void => {
      pendingStdout += String(chunk);
      const lines = pendingStdout.split(/\r?\n/g);
      pendingStdout = lines.pop() ?? "";
      if (!shouldEmitAgentStreamEvents()) {
        return;
      }
      for (const line of lines) {
        const event = codexCliStreamEventFromJsonLine(line, streamAccumulator);
        if (event) {
          emitAgentStreamEvent(event);
        }
      }
    };
    child.stdout?.on("data", chunk => {
      stdout = appendLimitedCodexOutput(stdout, chunk);
      handleStdoutStreamChunk(chunk);
    });
    child.stderr?.on("data", chunk => {
      stderr = appendLimitedCodexOutput(stderr, chunk);
    });

    const result = await new Promise<{ code: number | null; signal: NodeJS.Signals | null }>((resolve, reject) => {
      const timer = setTimeout(() => {
        child.kill("SIGTERM");
      }, getCodexTurnTimeoutMs());
      child.once("error", error => {
        clearTimeout(timer);
        reject(error);
      });
      child.once("exit", (code, signal) => {
        clearTimeout(timer);
        resolve({ code, signal });
      });
    });

    if (pendingStdout && shouldEmitAgentStreamEvents()) {
      const event = codexCliStreamEventFromJsonLine(pendingStdout, streamAccumulator);
      if (event) {
        emitAgentStreamEvent(event);
      }
    }
    const parsed = parseCodexCliJsonl(stdout);
    if (result.code !== 0 || result.signal) {
      throw new CodexCliRunError(
        [
          `Codex CLI exited with ${result.signal ? `signal ${result.signal}` : `code ${result.code ?? "unknown"}`}.`,
          parsed.error,
          stderr.trim(),
          stdout.trim(),
        ].filter(Boolean).join("\n"),
        /"type":"mcp_tool_call".*"status":"completed"/s.test(stdout),
      );
    }
    if (parsed.error) {
      throw new Error(parsed.error);
    }
    if (!parsed.text.trim()) {
      throw new Error(`Codex CLI completed without assistant text.${stderr.trim() ? `\n${stderr.trim()}` : ""}`);
    }
    emitAgentStreamStatus("codex exec turn completed", "codex");
    return parsed.text;
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}

function formatStatusCoords(status: Status): string {
  return status.coords ? status.coords.join(",") : "unknown";
}

function summarizeHarnessError(error: unknown): string {
  if (error instanceof Error) {
    const firstLine = error.message.split(/\r?\n/g)[0]?.trim();
    return `${error.name}: ${firstLine || "(no message)"}`;
  }
  return String(error);
}

function buildFallbackCodexBatch(params: {
  input: RunnerInput;
  beforeStatus: Status;
  afterStatus: Status;
  recentEvents: string;
  actionSummaries: string[];
}): TaskmasterBatch {
  return taskmasterBatchSchema.parse({
    summary: `Agentic MCP turn changed live state from ${params.beforeStatus.map} ${formatStatusCoords(params.beforeStatus)} to ${params.afterStatus.map} ${formatStatusCoords(params.afterStatus)}.`,
    immediateGoalStatus: params.afterStatus.partyCount > params.beforeStatus.partyCount ? "done" : "in_progress",
    nextImmediateGoal: params.afterStatus.flowNextGoal || params.input.immediateGoal,
    shouldContinue: true,
    evidence: [
      ...params.actionSummaries.slice(-4),
      params.recentEvents.split("\n").find(Boolean) ?? `Current state: ${params.afterStatus.map} ${formatStatusCoords(params.afterStatus)}`,
    ].filter(Boolean).slice(0, 5),
  });
}

export function buildNoActionAgenticBatch(params: {
  input: RunnerInput;
  beforeStatus: Status;
  afterStatus: Status;
  recentEvents: string;
  reason: string;
  detail?: string;
}): TaskmasterBatch {
  // Hard guard: stalled model turns must roll into another model turn, never into runner-chosen inputs.
  return taskmasterBatchSchema.parse({
    summary: `${params.reason} No non-agentic gameplay action was executed. YOU MUST MAKE A CHOICE next cycle.`,
    immediateGoalStatus: "in_progress",
    nextImmediateGoal: [
      params.afterStatus.flowNextGoal || params.input.immediateGoal,
      "YOU MUST MAKE A CHOICE using live MCP evidence.",
    ].join(" "),
    shouldContinue: true,
    evidence: [
      params.detail,
      `Before: ${params.beforeStatus.map} ${formatStatusCoords(params.beforeStatus)}`,
      `After: ${params.afterStatus.map} ${formatStatusCoords(params.afterStatus)}`,
      params.recentEvents.split("\n").find(Boolean) ?? "No recent event summary available.",
    ].filter(Boolean).slice(0, 5),
  });
}

function isRpcResponse(message: RpcMessage): message is RpcResponse {
  return "id" in message && !("method" in message);
}

function readString(record: JsonObject | undefined, key: string): string | undefined {
  const value = record?.[key];
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

export function extractCodexAgentMessages(itemsValue: JsonValue | undefined): {
  finalText: string;
  error: string | null;
} {
  const items = Array.isArray(itemsValue)
    ? itemsValue
        .filter(item => item && typeof item === "object" && !Array.isArray(item))
        .map(item => item as JsonObject)
    : [];
  const agentMessages = items
    .filter(item => readString(item, "type") === "agentMessage")
    .map(item => ({
      text: readString(item, "text"),
      phase: readString(item, "phase"),
    }))
    .filter((value): value is { text: string; phase: string | undefined } => Boolean(value.text));
  const finalMessage =
    agentMessages.findLast(message => message.phase === "final_answer") ??
    agentMessages.at(-1);

  return {
    finalText: finalMessage?.text ?? "",
    error: readString(items.find(item => readString(item, "type") === "error"), "message") ?? null,
  };
}

async function readCodexTurnFromThread(
  client: CodexAppServerClient,
  threadId: string,
  turnId: string,
): Promise<{
  status?: string;
  finalText: string;
  error: string | null;
} | null> {
  const threadRead = await client.request<CodexThreadReadResponse>("thread/read", {
    threadId,
    includeTurns: true,
  });
  const matchingTurn = threadRead.thread?.turns?.find(turnRecord => turnRecord.id === turnId);
  if (!matchingTurn) {
    return null;
  }

  const extracted = extractCodexAgentMessages(matchingTurn.items);
  const errorObj = isJsonObject(matchingTurn.error) ? matchingTurn.error : undefined;
  return {
    status: matchingTurn.status,
    finalText: extracted.finalText,
    error: extracted.error ?? readString(errorObj, "message") ?? null,
  };
}

function codexBindingPath(sessionId: string): string {
  return path.join(CODEX_BINDINGS_DIR, `${sessionId}.json`);
}

async function readCodexBinding(sessionId: string): Promise<CodexThreadBinding | null> {
  try {
    const raw = await readFile(codexBindingPath(sessionId), "utf8");
    const parsed = JSON.parse(raw) as Partial<CodexThreadBinding>;
    if (
      typeof parsed.threadId === "string" &&
      parsed.threadId &&
      typeof parsed.sessionId === "string" &&
      parsed.sessionId &&
      typeof parsed.model === "string" &&
      parsed.model
    ) {
      return {
        threadId: parsed.threadId,
        sessionId: parsed.sessionId,
        model: parsed.model,
        updatedAt:
          typeof parsed.updatedAt === "string" && parsed.updatedAt
            ? parsed.updatedAt
            : new Date().toISOString(),
      };
    }
    return null;
  } catch {
    return null;
  }
}

async function writeCodexBinding(binding: CodexThreadBinding): Promise<void> {
  await mkdir(CODEX_BINDINGS_DIR, { recursive: true });
  await writeFile(codexBindingPath(binding.sessionId), JSON.stringify(binding, null, 2), "utf8");
}

async function clearCodexBinding(sessionId: string): Promise<void> {
  await rm(codexBindingPath(sessionId), { force: true });
}

class CodexAppServerClient {
  private readonly child = spawn(
    "codex",
    ["app-server", "--listen", "stdio://"],
    {
      cwd: process.cwd(),
      stdio: ["pipe", "pipe", "pipe"],
    },
  );
  private readonly lines: ReadlineInterface = createInterface({
    input: this.child.stdout,
  });
  private readonly pending = new Map<number | string, PendingRequest>();
  private readonly notificationHandlers = new Set<(notification: CodexServerNotification) => void>();
  private readonly requestHandlers = new Set<
    (request: Required<Pick<RpcRequest, "id" | "method">> & { params?: JsonValue }) => Promise<JsonValue | undefined>
  >();
  private nextId = 1;
  private closed = false;

  constructor() {
    this.lines.on("line", line => this.handleLine(line));
    this.child.once("exit", () => this.closeWithError(new Error("codex app-server exited")));
    this.child.once("error", error => this.closeWithError(error));
  }

  async initialize(): Promise<void> {
    await this.request("initialize", {
      clientInfo: {
        name: "pokecrystal-agents",
        title: "Pokecrystal Agents",
        version: "1.0.0",
      },
      capabilities: {
        experimentalApi: true,
      },
    });
    this.notify("initialized");
  }

  addNotificationHandler(
    handler: (notification: CodexServerNotification) => void,
  ): () => void {
    this.notificationHandlers.add(handler);
    return () => this.notificationHandlers.delete(handler);
  }

  addRequestHandler(
    handler: (
      request: Required<Pick<RpcRequest, "id" | "method">> & { params?: JsonValue },
    ) => Promise<JsonValue | undefined>,
  ): () => void {
    this.requestHandlers.add(handler);
    return () => this.requestHandlers.delete(handler);
  }

  request<T = JsonValue>(method: string, params?: JsonValue): Promise<T> {
    if (this.closed) {
      return Promise.reject(new Error("codex app-server client is closed"));
    }
    const id = this.nextId++;
    this.writeMessage({ id, method, params });
    return new Promise<T>((resolve, reject) => {
      this.pending.set(id, {
        resolve: value => resolve(value as T),
        reject,
      });
    });
  }

  notify(method: string, params?: JsonValue): void {
    if (this.closed) {
      return;
    }
    this.writeMessage({ method, params });
  }

  close(): void {
    if (this.closed) {
      return;
    }
    this.closed = true;
    this.lines.close();
    for (const pending of this.pending.values()) {
      pending.reject(new Error("codex app-server client closed"));
    }
    this.pending.clear();
    this.child.kill();
  }

  private writeMessage(message: RpcRequest): void {
    this.child.stdin.write(`${JSON.stringify(message)}\n`);
  }

  private async handleRequest(
    request: Required<Pick<RpcRequest, "id" | "method">> & { params?: JsonValue },
  ): Promise<void> {
    for (const handler of this.requestHandlers) {
      const result = await handler(request);
      if (result !== undefined) {
        this.child.stdin.write(`${JSON.stringify({ id: request.id, result })}\n`);
        return;
      }
    }
    this.child.stdin.write(
      `${JSON.stringify({
        id: request.id,
        error: {
          message: `Unhandled Codex app-server request: ${request.method}`,
        },
      })}\n`,
    );
  }

  private handleLine(line: string): void {
    if (!line.trim()) {
      return;
    }
    const message = JSON.parse(line) as RpcMessage;
    if (isRpcResponse(message)) {
      const pending = this.pending.get(message.id);
      if (!pending) {
        return;
      }
      this.pending.delete(message.id);
      if (message.error) {
        pending.reject(new Error(message.error.message || "codex app-server request failed"));
      } else {
        pending.resolve(message.result);
      }
      return;
    }

    if (message.id !== undefined) {
      void this.handleRequest({
        id: message.id,
        method: message.method,
        params: message.params,
      });
      return;
    }

    for (const handler of this.notificationHandlers) {
      handler({
        method: message.method,
        params: message.params,
      });
    }
  }

  private closeWithError(error: Error): void {
    if (this.closed) {
      return;
    }
    this.closed = true;
    this.lines.close();
    for (const pending of this.pending.values()) {
      pending.reject(error);
    }
    this.pending.clear();
  }
}

export function buildCodexToolDefinitions(
  session: KrabbyClawSession,
  maxActions: number,
): CodexToolDefinition[] {
  let actionsUsed = 0;
  const consumeAction = async (fn: () => Promise<string>): Promise<string> => {
    if (actionsUsed >= maxActions) {
      throw new Error(`Action budget exhausted for this batch. Maximum action tools: ${maxActions}.`);
    }
    actionsUsed += 1;
    return fn();
  };

  return [
    {
      spec: {
        name: "status",
        description:
          "Get the structured live game state. Call this first and after each action.",
        inputSchema: { type: "object", properties: {}, additionalProperties: false },
      },
      run: async () => JSON.stringify(await session.status()),
    },
    {
      spec: {
        name: "observe",
        description:
          "Get observation text when layout, dialogue, or visible context is unclear.",
        inputSchema: { type: "object", properties: {}, additionalProperties: false },
      },
      run: async () => (await session.observe()).rawTexts.join("\n"),
    },
    {
      spec: {
        name: "map_info",
        description: "Get richer routing and hotspot details for the current map.",
        inputSchema: { type: "object", properties: {}, additionalProperties: false },
      },
      run: async () => await session.mapInfo(),
    },
    {
      spec: {
        name: "flow_state",
        description: "Get spoiler-safe next-goal guidance for the current story state.",
        inputSchema: { type: "object", properties: {}, additionalProperties: false },
      },
      run: async () => await session.flowState(),
    },
    {
      spec: {
        name: "recent_events",
        description: "Read recent tool/action events for grounded progress evidence.",
        inputSchema: { type: "object", properties: {}, additionalProperties: false },
      },
      run: async () => await session.recentEvents(),
    },
    {
      spec: {
        name: "move",
        description:
          "Move one or more steps in a cardinal direction. Counts against the action budget.",
        inputSchema: {
          type: "object",
          properties: {
            direction: { type: "string", enum: ["up", "down", "left", "right"] },
            steps: { type: "integer", minimum: 1, maximum: 4 },
            reason: {
              type: "string",
              minLength: 1,
              description: "Visible operational rationale for this action, grounded in the latest MCP state.",
            },
          },
          required: ["direction", "reason"],
          additionalProperties: false,
        },
      },
      run: async args =>
        consumeAction(async () => {
          emitCodexVisibleActionReason(args);
          const normalized = normalizeCodexMoveArgs(args);
          return session.move(normalized.direction, normalized.steps);
        }),
    },
    {
      spec: {
        name: "press",
        description: "Press A, B, Start, Select, or one D-pad direction once. Counts against the action budget.",
        inputSchema: {
          type: "object",
          properties: {
            button: { type: "string", enum: ["A", "B", "Start", "Select", "Up", "Down", "Left", "Right"] },
            reason: {
              type: "string",
              minLength: 1,
              description: "Visible operational rationale for this action, grounded in the latest MCP state.",
            },
          },
          required: ["button", "reason"],
          additionalProperties: false,
        },
      },
      run: async args =>
        consumeAction(async () => {
          emitCodexVisibleActionReason(args);
          const normalized = normalizeCodexButtonArgs(args);
          return session.press(normalized.button);
        }),
    },
    {
      spec: {
        name: "hold_button",
        description:
          "Hold A, B, Start, Select, or one D-pad direction for a short number of frames. Counts against the action budget.",
        inputSchema: {
          type: "object",
          properties: {
            button: { type: "string", enum: ["A", "B", "Start", "Select", "Up", "Down", "Left", "Right"] },
            frames: { type: "integer", minimum: 1, maximum: 60 },
            reason: {
              type: "string",
              minLength: 1,
              description: "Visible operational rationale for this action, grounded in the latest MCP state.",
            },
          },
          required: ["button", "frames", "reason"],
          additionalProperties: false,
        },
      },
      run: async args =>
        consumeAction(async () => {
          emitCodexVisibleActionReason(args);
          const normalized = normalizeCodexButtonArgs(args);
          return session.holdButton(
            normalized.button,
            normalizeCodexFrameCount(args.frames, 8, 60),
          );
        }),
    },
  ];
}

export function buildCodexBatchPrompt(
  input: RunnerInput,
  beforeStatus: Status,
  recentEventsBefore?: string,
  mapInfoBefore?: string,
  observeBefore?: string,
): string {
  const battleActive =
    beforeStatus.inBattle === true ||
    beforeStatus.mode.toLowerCase() === "battle" ||
    beforeStatus.surface?.kind.toLowerCase() === "battle";
  return [
    "Self-manage one bounded Pokemon Crystal gameplay batch through the provided MCP tools.",
    `Overall goal: ${input.overallGoal}`,
    `Requested immediate goal (verify against live flow): ${input.immediateGoal}`,
    `Live objective authority: status.flowNextGoal="${beforeStatus.flowNextGoal}", partyCount=${beforeStatus.partyCount}. Treat this live evidence as higher authority than stale immediate-goal wording, while also treating newly verified NPC/interactable goals as actionable objectives.`,
    beforeStatus.partyCount > 0 && /\bstarter\b/i.test(input.immediateGoal)
      ? "Starter objective correction: the live party already has Pokemon, so do not keep trying to get a starter; pivot to the current flow goal instead."
      : undefined,
    `Action budget this batch: ${input.playerMaxSteps}`,
    `Live status before acting:\n${JSON.stringify(beforeStatus)}`,
    mapInfoBefore ? `Live map_info before acting:\n${mapInfoBefore}` : undefined,
    observeBefore ? `Live observe before acting:\n${observeBefore}` : undefined,
    recentEventsBefore ? `Recent events:\n${recentEventsBefore}` : "Recent events: none.",
    `Gameplay action budget this batch: ${input.playerMaxSteps}`,
    "Rules:",
    "- Use objective operational language only. Do not use fictional framing, personal flourish, or decorative narration.",
    "- If external intervention appears in the goal or recent events, those manual inputs have already happened in the live game. Observe the updated state and continue from there instead of replaying them blindly.",
    "- Every action tool call must provide a visible reason: for direct Codex tools include the required reason argument; for krabbyclaw MCP tools first write <think>your concise operational reason</think> before calling move, press, or hold_button.",
    "- The visible reason should explain the live evidence, intended gameplay action, and intended effect, not repeat the raw tool arguments; MCP already records the tool log.",
    "- When the game offers a nickname prompt after receiving or catching a Pokemon, prefer giving that Pokemon a short nickname unless doing so would block urgent progress.",
    "- Use the provided MCP tools to observe state, choose actions, and advance the run.",
    "- Start from the newest visible tool state; if status and observe disagree, use the visible observe state and re-check status after acting.",
    "- Make concrete progress toward beating the game; do not choose no-input idling as an action.",
    "- If live party/flow evidence shows the immediate goal text is stale, act on the current flow goal instead of repeating the stale objective.",
    "- Flow_state is the sequential backbone for beating the game. Follow it in order, but treat everything encountered on the honest route toward that flow goal as important route evidence: NPC requests, sign clues, item hints, unique objects, forced prompts, battles, doors, warps, blockers, and local clues may be the actual next step.",
    "- Post-Mystery-Egg route: after the Mystery Egg has been returned to Elm, go talk to Mom in Player's House 1F and clear her money-saving/bank prompt before leaving New Bark for Violet. Decline or cancel Mom saving money unless the current goal explicitly says to use the bank.",
    "- If the next action is unclear or blocked, look around at nearby signs, NPCs, item balls, and unique objects for action clues before wandering; preserve any verified clue as a compact note about what to do next.",
    "- On a new map or route transition, sample one fresh reachable NPC/sign/object when it is close and safe, then return to route progress unless it reveals a new blocker or objective.",
    battleActive
      ? "- Battle is active now. Resolve the battle menu decisively; do not try overworld movement or repeatedly press B. Use RUN for low-risk wild blockers when travel is the priority, or FIGHT with sensible damaging moves when escape is unavailable, risky, or already failed."
      : "- If a battle becomes active, treat it as the current interface to solve: use RUN when travel should continue, or FIGHT with sensible damaging moves until control returns.",
    "- In battle, B is not a reliable escape from the fight itself; use the RUN command or win the battle.",
    "- Never invent state.",
    "- Return exactly one JSON object and no prose outside it after the batch.",
    `JSON schema:
{
  "summary": "string",
  "immediateGoalStatus": "done | in_progress | blocked",
  "nextImmediateGoal": "string",
  "shouldContinue": true,
  "evidence": ["string", "string"]
}`,
    "Ground evidence in tool output only.",
  ].filter(Boolean).join("\n");
}

export async function runCodexTaskmasterBatch(params: {
  input: RunnerInput;
  beforeStatus: Status;
  recentEventsBefore?: string;
  session: KrabbyClawSession;
  sessionAuth: SessionAuth;
}): Promise<{ taskmaster: TaskmasterBatch; afterStatus: Status }> {
  const cliMapInfoBefore = await params.session.mapInfo();
  const cliObserveBefore = (await params.session.observe()).rawTexts.join("\n");
  try {
    const completedText = await runCodexExec({
      input: params.input,
      prompt: buildCodexBatchPrompt(
        params.input,
        params.beforeStatus,
        params.recentEventsBefore,
        cliMapInfoBefore,
        cliObserveBefore,
      ),
      sessionAuth: params.sessionAuth,
    });
    const afterStatus = await params.session.status();
    const parsed = taskmasterBatchSchema.parse(
      JSON.parse(extractJsonObjectFromText(completedText)),
    );
    emitTaskmasterBatchStreamSummary(parsed);
    return {
      taskmaster: parsed,
      afterStatus,
    };
  } catch (error) {
    const afterStatus = await params.session.status();
    if (
      didStatusChange(params.beforeStatus, afterStatus) ||
      (error instanceof CodexCliRunError && error.progressMade)
    ) {
      return {
        taskmaster: streamTaskmasterBatch(buildFallbackCodexBatch({
          input: params.input,
          beforeStatus: params.beforeStatus,
          afterStatus,
          recentEvents: await params.session.recentEvents(),
          actionSummaries: [
            `Codex CLI MCP run changed live state before ending: ${
              error instanceof Error ? error.message.split("\n")[0] : String(error)
            }`,
          ],
        })),
        afterStatus,
      };
    }
    throw error;
  }

  const client = new CodexAppServerClient();
  const toolDefinitions = buildCodexToolDefinitions(params.session, params.input.playerMaxSteps);
  const toolMap = new Map(toolDefinitions.map(tool => [tool.spec.name, tool]));
  const mapInfoBefore = await params.session.mapInfo();
  const routeSummary = summarizeCodexRouteTarget(mapInfoBefore);
  const observeBefore = (await params.session.observe()).rawTexts.join("\n");
  await client.initialize();

  const requestCleanup = client.addRequestHandler(async request => {
    if (request.method !== "item/tool/call") {
      return undefined;
    }
    const call = isJsonObject(request.params)
      ? (request.params as unknown as CodexDynamicToolCallParams)
      : undefined;
    if (!call) {
      return {
        contentItems: [{ type: "inputText", text: "Invalid tool call payload." }],
        success: false,
      };
    }
    const tool = toolMap.get(call.tool);
    if (!tool) {
      return {
        contentItems: [{ type: "inputText", text: `Unknown tool: ${call.tool}` }],
        success: false,
      };
    }
    try {
      const args =
        call.arguments && typeof call.arguments === "object" && !Array.isArray(call.arguments)
          ? (call.arguments as Record<string, unknown>)
          : {};
      const text = await tool.run(args);
      return {
        contentItems: [{ type: "inputText", text }],
        success: true,
      };
    } catch (error) {
      return {
        contentItems: [
          {
            type: "inputText",
            text: error instanceof Error ? error.message : String(error),
          },
        ],
        success: false,
      };
    }
  });

  let binding = await readCodexBinding(params.sessionAuth.sessionId);
  let threadId: string;
  try {
    if (binding! && binding!.threadId && binding!.model === params.input.taskmasterModel) {
      const existingBinding = binding! as CodexThreadBinding;
      try {
        const resumeResponse = await client.request<CodexThreadResumeResponse>("thread/resume", {
          threadId: existingBinding.threadId,
          model: params.input.taskmasterModel.replace(/^codex\//i, ""),
          modelProvider: "openai",
          approvalPolicy: "never",
          sandbox: "read-only",
          persistExtendedHistory: true,
        });
        threadId = resumeResponse.thread.id;
      } catch {
        await clearCodexBinding(params.sessionAuth.sessionId);
        binding = null;
        const startResponse = await client.request<CodexThreadStartResponse>("thread/start", {
          model: params.input.taskmasterModel.replace(/^codex\//i, ""),
          modelProvider: "openai",
          approvalPolicy: "never",
          sandbox: "read-only",
          serviceName: "Pokecrystal Agents",
          developerInstructions: [
            "You are a self-managed Pokemon Crystal gameplay agent.",
            "Use the provided MCP game tools to self-manage the run.",
            "Do not use shell commands, file edits, or any other built-in workspace actions.",
            "Every gameplay action tool call (move, press, hold_button) must include a non-empty visible reason argument explaining the live evidence and intended effect.",
            "Use objective operational language only. Do not use fictional framing, personal flourish, or decorative narration.",
            "Use flow_state as the sequential backbone for beating the game, and treat everything encountered on the route to that flow goal as important route evidence: NPCs, signs, item balls, prompts, battles, doors, warps, blockers, and local clues may be the actual next step.",
            "Treat NPC requests, sign clues, item hints, and unique object discoveries as first-class goals alongside flow_state; note what was learned and let actionable clues shape the next objective.",
            "After receiving the Mystery Egg and returning it to Elm, clear the New Bark Mom handoff before routing to Violet: talk to Mom in Player's House 1F and decline/cancel her money-saving or bank prompt unless the current goal explicitly asks to bank.",
            "If external intervention appears, treat it as manual live play that already happened and continue from the updated current state.",
            "Keep playing toward the overall goal and return the requested JSON checkpoint.",
          ].join("\n\n"),
          dynamicTools: toolDefinitions.map(tool => tool.spec),
          experimentalRawEvents: true,
          persistExtendedHistory: true,
        });
        threadId = startResponse.thread.id;
      }
    } else {
      const startResponse = await client.request<CodexThreadStartResponse>("thread/start", {
        model: params.input.taskmasterModel.replace(/^codex\//i, ""),
        modelProvider: "openai",
        approvalPolicy: "never",
        sandbox: "read-only",
        serviceName: "Pokecrystal Agents",
        developerInstructions: [
          "You are a self-managed Pokemon Crystal gameplay agent.",
          "Use the provided MCP game tools to self-manage the run.",
          "Do not use shell commands, file edits, or any other built-in workspace actions.",
          "Every gameplay action tool call (move, press, hold_button) must include a non-empty visible reason argument explaining the live evidence and intended effect.",
          "Use objective operational language only. Do not use fictional framing, personal flourish, or decorative narration.",
          "Use flow_state as the sequential backbone for beating the game, and treat everything encountered on the route to that flow goal as important route evidence: NPCs, signs, item balls, prompts, battles, doors, warps, blockers, and local clues may be the actual next step.",
          "Treat NPC requests, sign clues, item hints, and unique object discoveries as first-class goals alongside flow_state; note what was learned and let actionable clues shape the next objective.",
          "After receiving the Mystery Egg and returning it to Elm, clear the New Bark Mom handoff before routing to Violet: talk to Mom in Player's House 1F and decline/cancel her money-saving or bank prompt unless the current goal explicitly asks to bank.",
          "If external intervention appears, treat it as manual live play that already happened and continue from the updated current state.",
          "Keep playing toward the overall goal and return the requested JSON checkpoint.",
        ].join("\n\n"),
        dynamicTools: toolDefinitions.map(tool => tool.spec),
        experimentalRawEvents: true,
        persistExtendedHistory: true,
      });
      threadId = startResponse.thread.id;
    }

    await writeCodexBinding({
      threadId,
      sessionId: params.sessionAuth.sessionId,
      model: params.input.taskmasterModel,
      updatedAt: new Date().toISOString(),
    });

    let activeTurnId = "";
    let completedText = "";
    let completedError: string | null = null;
    let completedTurnId = "";
    let resolveCompletion!: () => void;
    const completion = new Promise<void>(resolve => {
      resolveCompletion = resolve;
    });

    const notificationCleanup = client.addNotificationHandler(notification => {
      if (notification.method !== "turn/completed") {
        return;
      }
      const payload = isJsonObject(notification.params) ? notification.params : undefined;
      const turn = payload?.turn;
      if (!turn || typeof turn !== "object" || Array.isArray(turn)) {
        return;
      }
      const turnRecord = turn as JsonObject;
      const notificationTurnId = readString(turnRecord, "id");
      if (!notificationTurnId || (activeTurnId && notificationTurnId !== activeTurnId)) {
        return;
      }
      completedTurnId = notificationTurnId;
      const extracted = extractCodexAgentMessages(turnRecord.items);
      completedText = extracted.finalText;
      const errorObj = isJsonObject(turnRecord.error as JsonValue | undefined)
        ? (turnRecord.error as JsonObject)
        : undefined;
      completedError = extracted.error ?? readString(errorObj, "message") ?? null;
      resolveCompletion();
    });

    const turn = await client.request<CodexTurnStartResponse>("turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: buildCodexBatchPrompt(
            params.input,
            params.beforeStatus,
            params.recentEventsBefore,
            mapInfoBefore,
            observeBefore,
          ),
        },
      ],
      model: params.input.taskmasterModel.replace(/^codex\//i, ""),
      approvalPolicy: "never",
    });
    activeTurnId = turn.turn.id;

    const terminalTurnState = async (): Promise<boolean> => {
      while (true) {
        await new Promise(resolve => setTimeout(resolve, 2000));
        if (!activeTurnId) {
          continue;
        }
        const turnState = await readCodexTurnFromThread(client, threadId, activeTurnId).catch(error => {
          if (
            error instanceof Error &&
            /not materialized yet|includeTurns is unavailable/i.test(error.message)
          ) {
            return null;
          }
          throw error;
        });
        if (!turnState) {
          continue;
        }
        if (
          turnState.status === "completed" ||
          turnState.status === "interrupted" ||
          turnState.status === "failed"
        ) {
          completedTurnId = activeTurnId;
          completedText = turnState.finalText;
          completedError = turnState.error;
          return true;
        }
      }
    };

    const completedInTime = await Promise.race([
      completion.then(() => true),
      terminalTurnState(),
      new Promise<false>(resolve => {
        setTimeout(() => resolve(false), getCodexTurnTimeoutMs());
      }),
    ]);
    notificationCleanup();

    const afterStatus = await params.session.status();
    const recentEvents = await params.session.recentEvents();

    if (!completedInTime) {
      if (didStatusChange(params.beforeStatus, afterStatus)) {
        return {
          taskmaster: streamTaskmasterBatch(buildFallbackCodexBatch({
            input: params.input,
            beforeStatus: params.beforeStatus,
            afterStatus,
            recentEvents,
            actionSummaries: ["Codex dynamic MCP turn changed live state before timing out."],
          })),
          afterStatus,
        };
      }
      emitAgentStreamStatus(
        "Codex dynamic MCP turn timed out with no live-state progress; non-agentic recovery disabled; YOU MUST MAKE A CHOICE next cycle",
        "runner",
      );
      return {
        taskmaster: streamTaskmasterBatch(buildNoActionAgenticBatch({
          input: params.input,
          beforeStatus: params.beforeStatus,
          afterStatus,
          recentEvents,
          reason: "Codex dynamic MCP turn timed out with no live-state progress.",
        }), "runner"),
        afterStatus,
      };
    }

    if (completedError) {
      throw new Error(completedError ?? "Codex turn failed.");
    }

    if (!completedText.trim() && completedTurnId) {
      const refreshedTurnState = await readCodexTurnFromThread(client, threadId, completedTurnId);
      if (refreshedTurnState) {
        const state = refreshedTurnState as NonNullable<Awaited<ReturnType<typeof readCodexTurnFromThread>>>;
        completedText = state.finalText;
        completedError = state.error ?? completedError;
      }
    }

    if (completedError) {
      throw new Error(completedError ?? "Codex turn failed.");
    }

    if (!completedText.trim()) {
      if (didStatusChange(params.beforeStatus, afterStatus)) {
        return {
          taskmaster: streamTaskmasterBatch(buildFallbackCodexBatch({
            input: params.input,
            beforeStatus: params.beforeStatus,
            afterStatus,
            recentEvents,
            actionSummaries: ["Codex dynamic MCP turn completed after changing live state without a final message."],
          })),
          afterStatus,
        };
      }
      emitAgentStreamStatus(
        "Codex dynamic MCP turn completed without a final message or live-state progress; non-agentic recovery disabled; YOU MUST MAKE A CHOICE next cycle",
        "runner",
      );
      return {
        taskmaster: streamTaskmasterBatch(buildNoActionAgenticBatch({
          input: params.input,
          beforeStatus: params.beforeStatus,
          afterStatus,
          recentEvents,
          reason: "Codex dynamic MCP turn completed without a final message or live-state progress.",
        }), "runner"),
        afterStatus,
      };
    }

    const parsed = taskmasterBatchSchema.parse(
      JSON.parse(extractJsonObjectFromText(completedText)),
    );
    emitTaskmasterBatchStreamSummary(parsed);
    return {
      taskmaster: parsed,
      afterStatus,
    };
  } finally {
    requestCleanup();
    client.close();
  }
}
