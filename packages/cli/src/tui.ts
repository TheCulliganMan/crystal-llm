import { spawn, type ChildProcess } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import {
  emptyAgentStreamState,
  parseAgentStreamLine,
  reduceAgentStreamState,
  type AgentStreamEvent,
  type AgentStreamState,
} from "./agent-stream";
import { createPlayTrainingRecorder } from "./play-training";
import { startLocalMcpHttpServer, type LocalMcpHttpServer } from "./tui-mcp-http-server";
import { createDirectLocalTuiMcpClient, createTuiMcpClient, type TuiMcpClient } from "./tui-mcp-client";
import { loadInkRuntime, renderInkTui, type InkRuntime, type InkTuiRenderer, type TuiViewState } from "./tui-ink";
import { createTuiSoundController, extractTuiAudioPlaybackSnapshot, type TuiSoundController } from "./tui-sound";
import { createSessionLogger, summarizeToolResult, withResolvedSessionLogFile } from "./session-log";
import {
  createDialogueAccumulator,
  normalizeTuiSnapshot,
  type DialogueAccumulator,
  type TuiSnapshot,
} from "./tui-snapshot";
import {
  extractKittyPngFrame,
  isKittyGraphicsSupported,
  type GameboyRendererMode,
} from "./tui-kitty";
import {
  mapKeypressToAction,
  createKeypressChunkParser,
  isMenuLikeSurface,
  resolveLowercaseAAction,
  resolveNameEntryKeypressActions,
  resolveDirectionalAction,
  splitKeypressChunk,
  updateCommandMode,
  type CommandModeState,
} from "./tui-input";
import type { CliOptions, KeyAction, ToolResult } from "./types";

export { createKeypressChunkParser, isMenuLikeSurface, mapKeypressToAction, resolveDirectionalAction, resolveLowercaseAAction, resolveNameEntryKeypressActions, splitKeypressChunk, updateCommandMode };
export { createDialogueAccumulator, normalizeTuiSnapshot };

type TextUiRuntime = {
  stdin?: NodeJS.ReadStream & {
    isTTY?: boolean;
    setRawMode?(mode: boolean): void;
  };
  stdout?: Pick<NodeJS.WriteStream, "write"> & { isTTY?: boolean; columns?: number };
  fetchImpl?: typeof fetch;
  inkRuntime?: InkRuntime;
  startMcpServer?: (options: CliOptions) => Promise<LocalMcpHttpServer>;
  createMcpClient?: (url: string) => Promise<TuiMcpClient>;
  createDirectClient?: (options: CliOptions) => TuiMcpClient;
  soundController?: TuiSoundController;
  startLinkedAgent?: (
    options: CliOptions,
    mcpUrl: string,
    onStreamEvent?: (event: AgentStreamEvent) => void,
  ) => LinkedAgentProcess;
};

type TuiActiveView = "play" | "agent" | "agent-split" | "settings";

const cycleTuiView = (current: TuiActiveView, hasAgentStream: boolean): TuiActiveView => {
  const cycle: TuiActiveView[] = hasAgentStream
    ? ["play", "agent", "agent-split", "settings"]
    : ["play", "settings"];
  const currentIndex = cycle.indexOf(current);
  return cycle[(currentIndex + 1) % cycle.length] ?? "play";
};

const describeTuiView = (activeView: TuiActiveView): string => {
  switch (activeView) {
    case "agent":
      return "Agent details view. Use :v to cycle views or :u for image/text.";
    case "agent-split":
      return "Split Game Boy / agent view. Use :v to cycle views or :u for image/text.";
    case "settings":
      return "Settings view. Use :v to cycle views or :set <key> <value> to update agent settings.";
    case "play":
      return "Play view. Use :v to cycle views, :t for agent, or :u for image/text.";
  }
};

type ObserveToolOptions = {
  detail: "full";
  include_snapshot_text: true;
  include_image?: true;
  image_scale?: number;
  advance_frames?: number;
};

type ConsoleRestore = () => void;
type LinkedAgentProcess = {
  process: ChildProcess;
  note: string;
  output: () => string;
};
type AgentPauseReason = "user" | "manual";
type ManualInterventionAction = {
  action: string;
  rawKey: string;
  result: string;
  state: string;
};
type ManualInterventionState = {
  startedAt: number;
  lastInputAt: number;
  resumeAt: number;
  actions: ManualInterventionAction[];
  resuming: boolean;
};

type TuiRunStats = {
  startedAtMs: number;
  interactionCount: number;
};

type TuiRecentEventsStats = {
  sessionStartedAtMs?: number;
  interactionCount?: number;
};

const LINKED_AGENT_OUTPUT_LIMIT = 800;
const DEFAULT_MANUAL_INTERVENTION_IDLE_MS = 5_000;
const MANUAL_INTERVENTION_ACTION_LIMIT = 6;
const GAMEPLAY_INTERACTION_TOOLS = new Set(["move", "press", "hold_button", "wait"]);

const isGameplayInteractionTool = (toolName: string): boolean =>
  GAMEPLAY_INTERACTION_TOOLS.has(toolName);

const firstJsonObject = (result: ToolResult | undefined): Record<string, unknown> | null => {
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as unknown;
      return parsed && typeof parsed === "object" && !Array.isArray(parsed)
        ? parsed as Record<string, unknown>
        : null;
    } catch {
      continue;
    }
  }
  return null;
};

const extractRecentEventsStats = (result: ToolResult | undefined): TuiRecentEventsStats => {
  const payload = firstJsonObject(result);
  if (!payload) {
    return {};
  }
  const sessionStartedAtMs = typeof payload.session_started_at_ms === "number" && Number.isFinite(payload.session_started_at_ms)
    ? payload.session_started_at_ms
    : undefined;
  const interactionCount = typeof payload.total === "number" && Number.isFinite(payload.total)
    ? Math.max(0, Math.floor(payload.total))
    : undefined;
  return { sessionStartedAtMs, interactionCount };
};

const syncRunStatsFromRecentEvents = (
  runStats: TuiRunStats,
  recentEventsResult: ToolResult | undefined,
): void => {
  const stats = extractRecentEventsStats(recentEventsResult);
  if (stats.sessionStartedAtMs !== undefined) {
    runStats.startedAtMs = stats.sessionStartedAtMs;
  }
  if (stats.interactionCount !== undefined) {
    runStats.interactionCount = Math.max(runStats.interactionCount, stats.interactionCount);
  }
};

const resolveManualInterventionIdleMs = (): number => {
  const raw = process.env.POKECRYSTAL_CLI_AGENT_INTERVENTION_IDLE_MS?.trim();
  if (!raw) {
    return DEFAULT_MANUAL_INTERVENTION_IDLE_MS;
  }
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_MANUAL_INTERVENTION_IDLE_MS;
};

const manualInterventionRefreshMs = (idleMs: number): number =>
  Math.max(25, Math.min(250, Math.floor(idleMs / 5)));

const appendLimitedOutput = (current: string, chunk: Buffer | string): string => {
  const next = `${current}${String(chunk)}`;
  return next.length > LINKED_AGENT_OUTPUT_LIMIT
    ? next.slice(next.length - LINKED_AGENT_OUTPUT_LIMIT)
    : next;
};

const createLinkedAgentChunkHandler = (onEvent?: (event: AgentStreamEvent) => void) => {
  let pending = "";
  return (chunk: Buffer | string): void => {
    pending += String(chunk);
    const lines = pending.split(/\r?\n/g);
    pending = lines.pop() ?? "";
    for (const line of lines) {
      const event = parseAgentStreamLine(line);
      if (event) {
        onEvent?.(event);
      }
    }
  };
};

const stderrLinesForLinkedAgentNote = (output: string): string[] => {
  const lines = output
    .split(/\r?\n/g)
    .map(line => line.trim())
    .filter(Boolean);
  const nodeVersionIndex = lines.findIndex(line => /^Node\.js v/i.test(line));
  if (nodeVersionIndex > 0) {
    return lines.slice(Math.max(0, nodeVersionIndex - 8), nodeVersionIndex + 1);
  }
  let errorIndex = -1;
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    if (/\b(error|exception|failed|rejected|typeerror|referenceerror|syntaxerror|zoderror)\b/i.test(lines[index] ?? "")) {
      errorIndex = index;
      break;
    }
  }
  if (errorIndex >= 0) {
    return lines.slice(errorIndex, Math.min(lines.length, errorIndex + 8));
  }
  return lines.slice(-8);
};

const formatLinkedAgentExitNote = (
  linkedAgent: LinkedAgentProcess,
  code: number | null,
  signal: NodeJS.Signals | null,
): { note: string; isError: boolean } => {
  const output = linkedAgent.output().trim();
  const reason = signal ? `signal ${signal}` : `exit ${code ?? "unknown"}`;
  const suffix = output ? `: ${stderrLinesForLinkedAgentNote(output).join(" ")}` : "";
  const isError = code !== 0 && signal !== "SIGTERM";
  return {
    note: `Agent stopped (${reason})${suffix}`,
    isError,
  };
};

const installTuiConsoleSilencer = (): ConsoleRestore => {
  if (process.env.POKECRYSTAL_CLI_TUI_ALLOW_CONSOLE === "1") {
    return () => undefined;
  }

  const original = {
    log: console.log,
    info: console.info,
    warn: console.warn,
    error: console.error,
    debug: console.debug,
  };
  let restored = false;
  console.log = () => undefined;
  console.info = () => undefined;
  console.warn = () => undefined;
  console.error = () => undefined;
  console.debug = () => undefined;

  return () => {
    if (restored) {
      return;
    }
    restored = true;
    console.log = original.log;
    console.info = original.info;
    console.warn = original.warn;
    console.error = original.error;
    console.debug = original.debug;
  };
};

const baseObserveOptions = {
  detail: "full" as const,
  include_snapshot_text: true as const,
};

const TUI_IMAGE_RENDER_REFRESH_MS = 250;
const TUI_IMAGE_RENDER_ADVANCE_FRAMES = 25;
const TUI_TEXT_ADVANCE_SETTLE_FRAMES = 25;

const observeOptionsFor = (
  renderer: GameboyRendererMode = "text",
  extra: Partial<ObserveToolOptions> = {},
): ObserveToolOptions => ({
  ...baseObserveOptions,
  ...(renderer === "kitty" ? { include_image: true as const, image_scale: 2 } : {}),
  ...extra,
});

const settledObserveOptionsFor = (
  renderer: GameboyRendererMode = "text",
  extra: Partial<ObserveToolOptions> = {},
): ObserveToolOptions =>
  observeOptionsFor(
    renderer,
    renderer === "kitty" && extra.advance_frames === undefined
      ? { advance_frames: TUI_IMAGE_RENDER_ADVANCE_FRAMES, ...extra }
      : extra,
  );

const resolveInitialGameboyRenderer = (): GameboyRendererMode =>
  isKittyGraphicsSupported() ? "kitty" : "text";

const shouldFallbackFromKittyResult = (
  renderer: GameboyRendererMode,
  result: ToolResult,
): boolean =>
  renderer === "kitty" && !extractKittyPngFrame(result);

const kittyFallbackCommandNote = (): CommandModeState => ({
  buffer: null,
  note: "Kitty image renderer unavailable; using text.",
});

const repoRoot = path.resolve(__dirname, "..", "..", "..");

const resolveAgentBinPath = (): string =>
  path.join(repoRoot, "packages", "agents", "dist", "bin", "pokecrystal-agents.js");

const startLinkedAgent = (
  options: CliOptions,
  mcpUrl: string,
  onStreamEvent?: (event: AgentStreamEvent) => void,
): LinkedAgentProcess => {
  const agentBinPath = resolveAgentBinPath();
  if (!fs.existsSync(agentBinPath)) {
    const child = spawn(process.execPath, [
      "-e",
      `console.error(${JSON.stringify(
        `MODULE_NOT_FOUND: missing compiled agent entry ${agentBinPath}. Run npm run build:agents before linking the agent.`
      )}); process.exit(1);`,
    ], {
      cwd: repoRoot,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let output = "";
    child.stderr?.on("data", chunk => {
      output = appendLimitedOutput(output, chunk);
    });
    return {
      process: child,
      note: `Agent failed to start: missing compiled agent entry ${agentBinPath}.`,
      output: () => output,
    };
  }

  const args = [
    agentBinPath,
    options.agentCommand ?? "run",
    "--session-id",
    options.sessionId,
    "--mcp-url",
    mcpUrl,
  ];
  if (options.agentModel) {
    args.push("--model", options.agentModel);
  }
  if (options.agentGoal) {
    args.push("--goal", options.agentGoal);
  }
  if (options.agentMaxSteps) {
    args.push("--max-steps", String(options.agentMaxSteps));
  }
  if (options.agentGraphCycleSteps) {
    args.push("--graph-cycle-steps", String(options.agentGraphCycleSteps));
  }
  if (options.agentRequestDelayMs) {
    args.push("--request-delay-ms", String(options.agentRequestDelayMs));
  }
  if (options.agentIdentityName) {
    args.push("--identity-name", options.agentIdentityName);
  }

  const child = spawn(process.execPath, args, {
    cwd: repoRoot,
    env: {
      ...process.env,
      POKECRYSTAL_CODEX_TURN_TIMEOUT_MS:
        process.env.POKECRYSTAL_CODEX_TURN_TIMEOUT_MS ?? "90000",
      POKECRYSTAL_AGENT_STREAM_EVENTS: "1",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  const handleStdoutStreamChunk = createLinkedAgentChunkHandler(onStreamEvent);
  const handleStderrStreamChunk = createLinkedAgentChunkHandler(onStreamEvent);
  let pendingRawStderr = "";
  const handleRawStderrChunk = (chunk: Buffer | string): void => {
    pendingRawStderr += String(chunk);
    const lines = pendingRawStderr.split(/\r?\n/g);
    pendingRawStderr = lines.pop() ?? "";
    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed || parseAgentStreamLine(trimmed)) {
        continue;
      }
      onStreamEvent?.({
        type: "status",
        source: "runner",
        message: `agent stderr: ${trimmed}`,
      });
    }
  };
  child.stdout?.on("data", chunk => {
    output = appendLimitedOutput(output, chunk);
    handleStdoutStreamChunk(chunk);
  });
  child.stderr?.on("data", chunk => {
    output = appendLimitedOutput(output, chunk);
    handleStderrStreamChunk(chunk);
    handleRawStderrChunk(chunk);
  });

  return {
    process: child,
    note: `Agent linked via MCP (pid ${child.pid ?? "unknown"}).`,
    output: () => output,
  };
};

export const buildAgentInterruptOptions = (
  options: CliOptions,
  message: string,
): CliOptions => ({
  ...options,
  agentCommand: "resume",
  agentGoal: [
    options.agentGoal?.trim() || "Continue playing toward the current objective.",
    "",
    `Professor Culligan's Advice: ${message.trim()}`,
    "Treat Professor Culligan's Advice as the highest-priority professor guidance, then continue the overall run in trainer voice.",
  ].join("\n"),
});

export const buildAgentManualInterventionOptions = (
  options: CliOptions,
  interventionSummary: string,
): CliOptions => ({
  ...options,
  agentCommand: "resume",
  agentGoal: [
    options.agentGoal?.trim() || "Continue playing toward the current objective.",
    "",
    "Professor Culligan's Intervention:",
    "Professor Culligan paused autonomous play and manually controlled the live game for a short stretch.",
    "The manual inputs below have already happened in the live session. Do not repeat them blindly; observe the updated state, acknowledge Professor Culligan's intervention in trainer voice, and continue from the new state.",
    "",
    interventionSummary.trim(),
    "",
    "After resuming, explicitly fold Professor Culligan's intervention into your next trainer decision and keep narrating why you are acting.",
  ].join("\n"),
});

const stopLinkedAgent = async (linkedAgent?: LinkedAgentProcess): Promise<void> => {
  const child = linkedAgent?.process;
  if (!child || child.killed || child.exitCode !== null || child.signalCode !== null) {
    return;
  }
  await new Promise<void>((resolve) => {
    const timer = setTimeout(() => {
      child.kill("SIGKILL");
      resolve();
    }, 2_000);
    child.once("exit", () => {
      clearTimeout(timer);
      resolve();
    });
    child.kill("SIGTERM");
  });
};

const readSnapshotText = (result?: ToolResult): string =>
  (result?.content ?? [])
    .filter((entry) => entry.type === "text" && typeof entry.text === "string")
    .map((entry) => entry.text ?? "")
    .join("\n\n");

const normalizePcPromptText = (value: unknown): string =>
  String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/<pk>\s*<mn>/g, "pokemon")
    .replace(/pok[eé]mon/g, "pokemon")
    .replace(/[?.!]+$/g, "")
    .replace(/\s+/g, " ");

const isPcInstructionPrimaryText = (value: unknown): boolean => {
  const text = normalizePcPromptText(value);
  return (
    text === "choose a pokemon" ||
    text === "choose a pkmn" ||
    text === "select a pokemon" ||
    text === "what's up" ||
    text === "move to where"
  );
};

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

const preserveKittyImageContent = (
  result: ToolResult,
  previous: ToolResult | undefined,
  renderer: GameboyRendererMode,
): ToolResult => {
  if (renderer !== "kitty") {
    return result;
  }
  if ((result.content ?? []).some((entry) => entry.type === "image")) {
    return result;
  }
  const previousImages = (previous?.content ?? []).filter((entry) => entry.type === "image");
  if (!previousImages.length) {
    return result;
  }
  return {
    ...result,
    content: [...(result.content ?? []), ...previousImages],
  };
};

const firstCompactLine = (text: string, maxLength = 160): string => {
  const normalized = text
    .split(/\r?\n/g)
    .map(line => line.trim())
    .find(line => line.length > 0) ?? "";
  if (!normalized) {
    return "No text result.";
  }
  return normalized.length > maxLength
    ? `${normalized.slice(0, Math.max(0, maxLength - 3))}...`
    : normalized;
};

const summarizeStatusResult = (result: ToolResult): string => {
  for (const entry of result.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        mode?: unknown;
        map?: unknown;
        coords?: unknown;
        facing?: unknown;
        flowNextGoal?: unknown;
        surface?: { title?: unknown; kind?: unknown };
      };
      const coords = Array.isArray(parsed.coords) ? parsed.coords.join(",") : undefined;
      const parts = [
        typeof parsed.mode === "string" ? `mode=${parsed.mode}` : undefined,
        typeof parsed.map === "string" ? `map=${parsed.map}` : undefined,
        coords ? `coords=${coords}` : undefined,
        typeof parsed.facing === "string" ? `facing=${parsed.facing}` : undefined,
        typeof parsed.flowNextGoal === "string" ? `flow=${parsed.flowNextGoal}` : undefined,
        typeof parsed.surface?.title === "string"
          ? `surface=${parsed.surface.title}`
          : typeof parsed.surface?.kind === "string"
          ? `surface=${parsed.surface.kind}`
          : undefined,
      ].filter(Boolean);
      if (parts.length) {
        return parts.join(" ");
      }
    } catch {
      // Fall through to compact text below.
    }
  }
  return firstCompactLine(readSnapshotText(result));
};

const isTextAdvanceStatus = (result?: ToolResult): boolean => {
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        mode?: unknown;
        map?: unknown;
        blockedReason?: unknown;
        inDialog?: unknown;
        in_dialog?: unknown;
        promptPending?: unknown;
        prompt_pending?: unknown;
        textboxOpen?: unknown;
        textbox_open?: unknown;
        textBoxOpen?: unknown;
        text_box_open?: unknown;
        textAdvancePending?: unknown;
        text_advance_pending?: unknown;
        unown_puzzle_active?: unknown;
        unownPuzzleActive?: unknown;
        unown_state?: unknown;
        unownState?: unknown;
        input_blocked_reason?: unknown;
        surface?: {
          kind?: unknown;
          title?: unknown;
          selected?: unknown;
          primaryText?: unknown;
          primary_text?: unknown;
          menuOpen?: unknown;
          menu_open?: unknown;
          dialogueOpen?: unknown;
          dialogue_open?: unknown;
          promptOpen?: unknown;
          prompt_open?: unknown;
          waiting?: unknown;
        };
      };
      const blockedReason = parsed.input_blocked_reason ?? parsed.blockedReason;
      const surfaceKind = String(parsed.surface?.kind ?? "")
        .trim()
        .toLowerCase();
      const unownState = Number(parsed.unown_state ?? parsed.unownState ?? 0);
      if (
        parsed.unown_puzzle_active === true ||
        parsed.unownPuzzleActive === true ||
        unownState !== 0 ||
        surfaceKind === "unown_puzzle"
      ) {
        return false;
      }
      if (surfaceKind === "slot_machine") {
        return false;
      }
      const surfaceTitle = String(parsed.surface?.title ?? "")
        .trim()
        .toLowerCase();
      const surfacePrimaryText = String(parsed.surface?.primary_text ?? parsed.surface?.primaryText ?? "")
        .trim()
        .toLowerCase();
      const surfaceSelected = parsed.surface?.selected;
      const mode = String(parsed.mode ?? "").trim().toLowerCase();
      const map = String(parsed.map ?? "").trim().toLowerCase();
      const pcInstructionPrompt =
        parsed.surface?.menu_open === true ||
        parsed.surface?.menuOpen === true ||
        isPcInstructionPrimaryText(surfacePrimaryText);
      const pcSurfaceText =
        isPcMenuText(surfacePrimaryText) ||
        isPcMenuText(surfaceSelected) ||
        isPcMenuText(parsed.map);
      const pcOwnsVisiblePrompt =
        (surfaceKind === "pc" ||
          surfaceTitle.includes("pc") ||
          pcSurfaceText ||
          mode === "pc" ||
          map === "pc" ||
          map.includes("bill")) &&
        (pcInstructionPrompt ||
          (parsed.in_dialog !== true &&
            parsed.inDialog !== true &&
            parsed.prompt_pending !== true &&
            parsed.promptPending !== true &&
            parsed.text_advance_pending !== true &&
            parsed.textAdvancePending !== true &&
            blockedReason !== "dialogue" &&
            blockedReason !== "prompt"));
      if (surfaceKind === "mart" || surfaceTitle === "mart") {
        return false;
      }
      if (
        parsed.mode === "name_entry" ||
        blockedReason === "name_entry"
      ) {
        return false;
      }
      if (pcOwnsVisiblePrompt) {
        return false;
      }
      if (
        parsed.in_dialog === true ||
        parsed.inDialog === true ||
        parsed.prompt_pending === true ||
        parsed.promptPending === true ||
        parsed.textbox_open === true ||
        parsed.textboxOpen === true ||
        parsed.text_box_open === true ||
        parsed.textBoxOpen === true ||
        parsed.text_advance_pending === true ||
        parsed.textAdvancePending === true ||
        blockedReason === "dialogue" ||
        blockedReason === "prompt" ||
        parsed.surface?.dialogue_open === true ||
        parsed.surface?.dialogueOpen === true ||
        parsed.surface?.prompt_open === true ||
        parsed.surface?.promptOpen === true ||
        parsed.surface?.waiting === true
      ) {
        return true;
      }
    } catch {
      const normalized = entry.text.toLowerCase();
      const compact = normalized.replace(/[_\s-]+/g, "");
      if (
        normalized.includes("state: name_entry") ||
        compact.includes('"mode":"nameentry"') ||
        compact.includes('"inputblockedreason":"nameentry"') ||
        compact.includes('"blockedreason":"nameentry"')
      ) {
        return false;
      }
      if (
        compact.includes('"unownpuzzleactive":true') ||
        compact.includes('"unownstate":1') ||
        normalized.includes('"unown_puzzle_active":true') ||
        normalized.includes('"unown_state":1') ||
        normalized.includes('"kind":"unown_puzzle"')
      ) {
        return false;
      }
      if (
        normalized.includes("dlg: 1") ||
        normalized.includes("txt: 1") ||
        normalized.includes("pr: 1") ||
        compact.includes('"indialog":true') ||
        compact.includes('"promptpending":true') ||
        compact.includes('"textboxopen":true') ||
        compact.includes('"textadvancepending":true') ||
        compact.includes('"dialogueopen":true') ||
        compact.includes('"promptopen":true') ||
        compact.includes('"blockedreason":"dialogue"') ||
        compact.includes('"inputblockedreason":"dialogue"') ||
        compact.includes('"blockedreason":"prompt"') ||
        compact.includes('"inputblockedreason":"prompt"')
      ) {
        return true;
      }
    }
  }
  return false;
};

const isInstantModeStatus = (result?: ToolResult): boolean => {
  for (const entry of result?.content ?? []) {
    if (entry.type !== "text" || typeof entry.text !== "string") {
      continue;
    }
    try {
      const parsed = JSON.parse(entry.text) as {
        instant_mode?: unknown;
        instantMode?: unknown;
        status?: { instant_mode?: unknown; instantMode?: unknown };
      };
      if (
        parsed.instant_mode === true ||
        parsed.instantMode === true ||
        parsed.status?.instant_mode === true ||
        parsed.status?.instantMode === true
      ) {
        return true;
      }
    } catch {
      const normalized = entry.text.toLowerCase();
      if (normalized.includes('"instant_mode":true') || normalized.includes('"instantmode":true')) {
        return true;
      }
    }
  }
  return false;
};

const isInstantBattleStatus = (result?: ToolResult): boolean =>
  isInstantModeStatus(result) && isBattleMode(readSnapshotText(result));

const passiveObserveOptionsFor = (
  renderer: GameboyRendererMode = "text",
  statusResult?: ToolResult,
): ObserveToolOptions =>
  isTextAdvanceStatus(statusResult) || isInstantBattleStatus(statusResult)
    ? observeOptionsFor(renderer)
    : settledObserveOptionsFor(renderer);

const describeManualAction = (action: KeyAction): string => {
  if (action.type === "move" || action.type === "direction") {
    return `move ${action.direction}`;
  }
  if (action.type === "press") {
    return `press ${action.button.toUpperCase()}`;
  }
  if (action.type === "text") {
    return `text ${JSON.stringify(action.text)}`;
  }
  if (action.type === "wait") {
    return `wait ${action.frames} frames`;
  }
  if (action.type === "refresh") {
    return "refresh";
  }
  return action.type;
};

const isManualGameplayAction = (action: KeyAction): boolean =>
  action.type === "move" || action.type === "direction" || action.type === "press" || action.type === "text" || action.type === "wait";

const isCommandKeypressChunk = (chunk: string): boolean =>
  chunk.includes(":") || chunk.includes("\u0003");

const formatManualInterventionSummary = (intervention: ManualInterventionState): string => {
  const elapsedSeconds = Math.max(0, Math.round((intervention.lastInputAt - intervention.startedAt) / 100) / 10);
  const actionLines = intervention.actions
    .slice(-MANUAL_INTERVENTION_ACTION_LIMIT)
    .map((entry, index) =>
      `${index + 1}. ${entry.action}; ${entry.state}`
    );
  const omitted = Math.max(0, intervention.actions.length - actionLines.length);
  return [
    `Professor Culligan intervened for ${elapsedSeconds}s and made ${intervention.actions.length} manual input${intervention.actions.length === 1 ? "" : "s"}.`,
    omitted ? `${omitted} earlier manual input${omitted === 1 ? "" : "s"} omitted from this compact summary.` : undefined,
    "Manual inputs already applied:",
    ...(actionLines.length ? actionLines : ["- No gameplay inputs were recorded."]),
  ].filter(Boolean).join("\n");
};

const isBusyResult = (snapshot: string): boolean => {
  const normalized = snapshot.toLowerCase();
  return normalized.includes("fx: busy") || normalized.includes("rsn: busy") || normalized.includes("busy");
};

const isBattleMode = (snapshot: string): boolean => {
  const normalized = snapshot.toLowerCase();
  return (
    normalized.trim().startsWith("battle") ||
    normalized.includes("\nbattle\n") ||
    normalized.includes("m: battle") ||
    normalized.includes("\"mode\":\"battle\"")
  );
};

const toolResultErrorMessage = (toolName: string, result: ToolResult): string => {
  const payload = firstJsonObject(result);
  const directMessage =
    typeof payload?.message === "string"
      ? payload.message
      : typeof payload?.error === "string"
        ? payload.error
        : payload?.error && typeof payload.error === "object" && typeof (payload.error as { message?: unknown }).message === "string"
          ? (payload.error as { message: string }).message
          : undefined;
  return directMessage ?? `${toolName} returned an error result.`;
};

type TransientToolUnavailable = {
  message: string;
  reason: "battle" | "dialogue" | "name_entry";
};

const normalizeTransientToolUnavailableReason = (value: unknown): TransientToolUnavailable["reason"] | undefined => {
  if (typeof value !== "string") {
    return undefined;
  }
  const normalized = value.trim().toLowerCase().replace(/[\s-]+/g, "_");
  return normalized === "battle" || normalized === "dialogue" || normalized === "name_entry"
    ? normalized
    : undefined;
};

const reasonFromUnavailableMessage = (message: string): TransientToolUnavailable["reason"] | undefined => {
  const normalized = message.toLowerCase();
  if (normalized.includes("not available during battle")) {
    return "battle";
  }
  if (normalized.includes("not available during dialogue")) {
    return "dialogue";
  }
  if (normalized.includes("not available during name entry")) {
    return "name_entry";
  }
  return undefined;
};

const extractTransientToolUnavailable = (result: ToolResult): TransientToolUnavailable | undefined => {
  if (!result.isError) {
    return undefined;
  }
  const payload = firstJsonObject(result);
  if (!payload) {
    return undefined;
  }
  const error = payload.error && typeof payload.error === "object" && !Array.isArray(payload.error)
    ? payload.error as Record<string, unknown>
    : {};
  const context = payload.context && typeof payload.context === "object" && !Array.isArray(payload.context)
    ? payload.context as Record<string, unknown>
    : {};
  const message = typeof error.message === "string"
    ? error.message
    : typeof payload.message === "string"
      ? payload.message
      : typeof payload.error === "string"
        ? payload.error
        : undefined;
  const reason =
    normalizeTransientToolUnavailableReason(error.reason) ??
    normalizeTransientToolUnavailableReason(context.blockedReason) ??
    (message ? reasonFromUnavailableMessage(message) : undefined);
  if (
    message &&
    reason &&
    (error.code === "tool_not_available" || payload.available === false)
  ) {
    return { message, reason };
  }
  return undefined;
};

const throwIfToolError = (toolName: string, result: ToolResult): void => {
  if (result.isError) {
    throw new Error(toolResultErrorMessage(toolName, result));
  }
};

type BundledTuiState = {
  frame?: ToolResult;
  status?: ToolResult;
  recentEvents?: ToolResult;
};

const jsonToolResult = (payload: unknown): ToolResult | undefined =>
  payload && typeof payload === "object"
    ? { content: [{ type: "text", text: JSON.stringify(payload) }] }
    : undefined;

const extractBundledTuiState = (result?: ToolResult): BundledTuiState | undefined => {
  const payload = firstJsonObject(result);
  const tui = payload?.tui;
  if (!tui || typeof tui !== "object" || Array.isArray(tui)) {
    return undefined;
  }
  const state = tui as Record<string, unknown>;
  const bundled = {
    frame: jsonToolResult(state.frame),
    status: jsonToolResult(state.status),
    recentEvents: jsonToolResult(state.recent_events ?? state.recentEvents),
  };
  return bundled.frame || bundled.status || bundled.recentEvents ? bundled : undefined;
};

const hasVisibleDialogue = (
  snapshotResult?: ToolResult,
  statusResult?: ToolResult,
  recentEventsResult?: ToolResult,
): boolean =>
  normalizeTuiSnapshot(
    snapshotResult ?? { content: [] },
    statusResult,
    recentEventsResult,
    createDialogueAccumulator(),
  ).dialogue.length > 0;

const isBattleTransitionSnapshot = (snapshot: string): boolean => {
  const normalized = snapshot.toLowerCase();
  return normalized.includes("battle transition") || normalized.includes("the battle is starting");
};

const isBattleDelaySnapshot = (snapshot: string): boolean => {
  const normalized = snapshot.toLowerCase();
  if (!normalized.trim().startsWith("battle")) {
    return false;
  }
  return (
    isBattleTransitionSnapshot(snapshot) ||
    /wait:\s*.*(?:animation|sound|sfx|cry|delay|transition)/i.test(snapshot)
  );
};

const didOpenTextForAction = (action: KeyAction, result?: ToolResult): boolean => {
  if (action.type !== "press" || action.button !== "a") {
    return false;
  }
  const payload = firstJsonObject(result);
  const summary = String(payload?.summary ?? payload?.result ?? "");
  const events = Array.isArray(payload?.events)
    ? payload.events.map((entry) => String(entry))
    : [];
  const tuiRecent = payload?.tui && typeof payload.tui === "object" && !Array.isArray(payload.tui)
    ? (payload.tui as Record<string, unknown>).recent_events ??
      (payload.tui as Record<string, unknown>).recentEvents
    : null;
  const recentEvents = tuiRecent && typeof tuiRecent === "object" && !Array.isArray(tuiRecent)
    ? (tuiRecent as Record<string, unknown>).events
    : [];
  const recentSummaries = Array.isArray(recentEvents)
    ? recentEvents
        .map((entry) =>
          entry && typeof entry === "object" && !Array.isArray(entry)
            ? String((entry as Record<string, unknown>).summary ?? "")
            : ""
        )
        .filter(Boolean)
    : [];
  const text = [summary, ...events, ...recentSummaries, readSnapshotText(result)]
    .join("\n")
    .toLowerCase();
  return (
    text.includes("text advance opened") ||
    text.includes("dialogue opened") ||
    text.includes("opened text") ||
    text.includes("text opened")
  );
};

const isOverworldBusyStatus = (statusResult: ToolResult): boolean => {
  const payload = firstJsonObject(statusResult);
  if (!payload) {
    return false;
  }
  const mode = typeof payload.mode === "string" ? payload.mode.toLowerCase() : "";
  if (mode !== "overworld") {
    return false;
  }
  if (payload.menu === true || payload.prompt === true || payload.name_entry === true) {
    return false;
  }
  return payload.can_move === false;
};

const isPlainOverworldStatus = (statusResult?: ToolResult): boolean => {
  const payload = firstJsonObject(statusResult);
  if (!payload) {
    return false;
  }
  const mode = typeof payload.mode === "string" ? payload.mode.toLowerCase() : "";
  if (mode !== "overworld") {
    return false;
  }
  const surface = payload.surface && typeof payload.surface === "object"
    ? payload.surface as Record<string, unknown>
    : {};
  return (
    payload.can_move !== false &&
    payload.in_battle !== true &&
    payload.inBattle !== true &&
    payload.in_menu !== true &&
    payload.inMenu !== true &&
    payload.menu !== true &&
    payload.prompt !== true &&
    payload.prompt_pending !== true &&
    payload.promptPending !== true &&
    payload.in_dialog !== true &&
    payload.inDialog !== true &&
    payload.text_box_open !== true &&
    payload.textbox_open !== true &&
    payload.text_advance_pending !== true &&
    payload.name_entry !== true &&
    payload.input_blocked_reason == null &&
    surface.menu_open !== true &&
    surface.prompt_open !== true &&
    surface.dialogue_open !== true &&
    surface.waiting !== true &&
    surface.kind !== "name_entry"
  );
};

const isFieldScriptInterruption = (snapshot: string): boolean => {
  const normalized = snapshot.toLowerCase();
  return (
    isBusyResult(snapshot) ||
    normalized.includes("interrupted:dialogue") ||
    normalized.includes("interrupted:prompt") ||
    normalized.includes("interrupted:movement_lock") ||
    normalized.includes("movement_lock") ||
    normalized.includes("\"reason\":\"busy\"")
  );
};

const isBlankBattleSnapshot = (snapshot: string): boolean => {
  const normalized = snapshot.trim().toLowerCase();
  if (!normalized.startsWith("battle")) {
    return false;
  }
  return (
    !normalized.includes("menu") &&
    !normalized.includes("dialogue") &&
    !normalized.includes("a=advance") &&
    !normalized.includes("wait:") &&
    !normalized.includes("appeared!") &&
    !normalized.includes("used") &&
    !normalized.includes("fainted") &&
    !normalized.includes("grew to")
  );
};

const shouldStopBufferedInput = (
  action: KeyAction,
  beforeResult: ToolResult,
  actionResult: ToolResult,
  statusResult: ToolResult,
): boolean => {
  const beforeSnapshot = readSnapshotText(beforeResult);
  const actionSnapshot = readSnapshotText(actionResult);
  const statusSnapshot = readSnapshotText(statusResult);
  const isMovementAction = action.type === "move" || action.type === "direction";
  const instantBattle = isInstantBattleStatus(statusResult);
  return (
    (action.type === "press" &&
      action.button === "a" &&
      !instantBattle &&
      (isBattleMode(statusSnapshot) || actionSnapshot.toLowerCase().includes("prompt closed:dialogue"))) ||
    (isMovementAction &&
      !isBattleMode(statusSnapshot) &&
      (isFieldScriptInterruption(actionSnapshot) || isOverworldBusyStatus(statusResult))) ||
    (!instantBattle && !isBattleMode(beforeSnapshot) && isBattleMode(statusSnapshot)) ||
    (isBattleMode(statusSnapshot) && isBusyResult(actionSnapshot) && !instantBattle)
  );
};

export const __testing = {
  didOpenTextForAction,
  isTextAdvanceStatus,
  shouldStopBufferedInput,
};

const callAction = async (
  client: TuiMcpClient,
  action: KeyAction,
  renderer: GameboyRendererMode,
  statusResult?: ToolResult,
  options: { includeSnapshotText?: boolean; includeTuiState?: boolean } = {},
  context: { visibleDialogue?: boolean } = {},
): Promise<ToolResult | null> => {
  const snapshotOptions = {
    ...(options.includeSnapshotText
      ? { include_snapshot_text: true, detail: "compact" as const }
      : {}),
    ...(options.includeTuiState ? { include_tui_state: true } : {}),
  };
  if (action.type === "move") {
    return client.callTool("move", { direction: action.direction, steps: 1, ...snapshotOptions });
  }
  if (action.type === "direction") {
    return client.callTool("move", { direction: action.direction, steps: 1, ...snapshotOptions });
  }
  if (action.type === "press") {
    if (
      action.button === "a" &&
      (isTextAdvanceStatus(statusResult) || context.visibleDialogue === true) &&
      !isBattleMode(readSnapshotText(statusResult))
    ) {
      return client.callTool("execute_macro", {
        macro: "advance_dialog",
        max_presses: 8,
        settle_frames: isInstantBattleStatus(statusResult) ? 0 : TUI_TEXT_ADVANCE_SETTLE_FRAMES,
        ...snapshotOptions,
      });
    }
    return client.callTool("press", { button: action.button, times: 1, ...snapshotOptions });
  }
  if (action.type === "text") {
    return client.callTool("type_text", { text: action.text, ...snapshotOptions });
  }
  if (action.type === "wait") {
    return client.callTool("observe", {
      ...passiveObserveOptionsFor(renderer, statusResult),
      advance_frames: action.frames,
      ...snapshotOptions,
    });
  }
  if (action.type === "refresh") {
    return client.callTool("observe", passiveObserveOptionsFor(renderer, statusResult));
  }
  return null;
};

const callAndRefreshAction = async (
  input: {
    client: TuiMcpClient;
    action: KeyAction;
    rawKey: string;
    beforeResult: ToolResult;
    beforeStatusResult?: ToolResult;
    trainingRecorder: ReturnType<typeof createPlayTrainingRecorder>;
    gameboyRenderer: GameboyRendererMode;
  },
): Promise<{
  actionResult: ToolResult;
  latest: ToolResult;
  latestStatus: ToolResult;
  latestRecentEvents: ToolResult;
  shouldStop: boolean;
  applied: boolean;
  unavailableMessage?: string;
} | null> => {
  const actionResult = await callAction(
    input.client,
    input.action,
    input.gameboyRenderer,
    input.beforeStatusResult,
    { includeTuiState: true },
    { visibleDialogue: hasVisibleDialogue(input.beforeResult, input.beforeStatusResult) },
  );
  if (!actionResult) {
    return null;
  }
  const transientUnavailable = extractTransientToolUnavailable(actionResult);
  if (transientUnavailable) {
    const statusResult = await input.client.callTool("status", {});
    const observedAfterResult = await input.client.callTool(
      "observe",
      passiveObserveOptionsFor(input.gameboyRenderer, statusResult),
    );
    const recentEventsResult = await input.client.callTool("recent_events", { limit: 8 });
    return {
      actionResult,
      latest: observedAfterResult,
      latestStatus: statusResult,
      latestRecentEvents: recentEventsResult,
      shouldStop: true,
      applied: false,
      unavailableMessage: transientUnavailable.message,
    };
  }
  throwIfToolError(describeManualAction(input.action), actionResult);
  const { observedAfterResult, statusResult } = await recoverBusyBattleSnapshot(
    input.client,
    input.action,
    actionResult,
    input.gameboyRenderer,
    input.beforeStatusResult,
  );
  const bundledState = extractBundledTuiState(actionResult);
  const recentEventsResult =
    bundledState?.recentEvents ?? await input.client.callTool("recent_events", { limit: 8 });
  if (
    didOpenTextForAction(input.action, actionResult) &&
    !hasVisibleDialogue(observedAfterResult, statusResult, recentEventsResult)
  ) {
    const refreshedStatus = await input.client.callTool("status", {});
    const refreshedObserved = await input.client.callTool(
      "observe",
      passiveObserveOptionsFor(input.gameboyRenderer, refreshedStatus),
    );
    const refreshedRecent = await input.client.callTool("recent_events", { limit: 8 });
    input.trainingRecorder?.recordTurn({
      rawKey: input.rawKey,
      action: input.action,
      beforeResult: input.beforeResult,
      actionResult,
      observedAfterResult: refreshedObserved,
      statusResult: refreshedStatus,
      recentEventsResult: refreshedRecent,
    });
    return {
      actionResult,
      latest: refreshedObserved,
      latestStatus: refreshedStatus,
      latestRecentEvents: refreshedRecent,
      shouldStop: shouldStopBufferedInput(input.action, input.beforeResult, actionResult, refreshedStatus),
      applied: true,
    };
  }
  input.trainingRecorder?.recordTurn({
    rawKey: input.rawKey,
    action: input.action,
    beforeResult: input.beforeResult,
    actionResult,
    observedAfterResult,
    statusResult,
    recentEventsResult,
  });
  return {
    actionResult,
    latest: observedAfterResult,
    latestStatus: statusResult,
    latestRecentEvents: recentEventsResult,
    shouldStop: shouldStopBufferedInput(input.action, input.beforeResult, actionResult, statusResult),
    applied: true,
  };
};

const recoverBusyBattleSnapshot = async (
  client: TuiMcpClient,
  action: KeyAction,
  actionResult: ToolResult,
  renderer: GameboyRendererMode,
  beforeStatusResult?: ToolResult,
): Promise<{ observedAfterResult: ToolResult; statusResult: ToolResult }> => {
  const bundledState = extractBundledTuiState(actionResult);
  let observedAfterResult =
    action.type === "refresh"
      ? actionResult
      : bundledState?.frame ?? await client.callTool("observe", settledObserveOptionsFor(renderer));
  let statusResult = bundledState?.status ?? await client.callTool("status", {});
  if (isInstantBattleStatus(statusResult)) {
    return { observedAfterResult, statusResult };
  }
  const shouldRecoverTransition =
    isBattleMode(readSnapshotText(statusResult)) &&
    isBattleTransitionSnapshot(readSnapshotText(observedAfterResult)) &&
    (action.type === "wait" || isBusyResult(readSnapshotText(actionResult)));

  if (
    action.type !== "wait" &&
    action.type !== "refresh" &&
    isBusyResult(readSnapshotText(actionResult)) &&
    isBusyResult(readSnapshotText(observedAfterResult)) &&
    isBattleMode(readSnapshotText(statusResult))
  ) {
    for (let i = 0; i < 24; i += 1) {
      observedAfterResult = await client.callTool("observe", observeOptionsFor(renderer, { advance_frames: 8 }));
      statusResult = await client.callTool("status", {});
      if (!isBattleMode(readSnapshotText(statusResult)) || !isBusyResult(readSnapshotText(observedAfterResult))) {
        break;
      }
    }
  }

  if (shouldRecoverTransition) {
    for (let i = 0; i < 24; i += 1) {
      observedAfterResult = await client.callTool("observe", observeOptionsFor(renderer, { advance_frames: 8 }));
      statusResult = await client.callTool("status", {});
      if (!isBattleMode(readSnapshotText(statusResult)) || !isBattleTransitionSnapshot(readSnapshotText(observedAfterResult))) {
        break;
      }
    }
  }

  if (
    isBattleMode(readSnapshotText(statusResult)) &&
    isBattleDelaySnapshot(readSnapshotText(observedAfterResult))
  ) {
    for (let i = 0; i < 24; i += 1) {
      observedAfterResult = await client.callTool("observe", observeOptionsFor(renderer, { advance_frames: 8 }));
      statusResult = await client.callTool("status", {});
      if (!isBattleMode(readSnapshotText(statusResult)) || !isBattleDelaySnapshot(readSnapshotText(observedAfterResult))) {
        break;
      }
    }
  }

  if (
    action.type === "press" &&
    action.button === "a" &&
    isBattleMode(readSnapshotText(statusResult)) &&
    isBlankBattleSnapshot(readSnapshotText(observedAfterResult))
  ) {
    for (let i = 0; i < 4; i += 1) {
      observedAfterResult = await client.callTool("observe", observeOptionsFor(renderer, { advance_frames: 8 }));
      statusResult = await client.callTool("status", {});
      if (!isBattleMode(readSnapshotText(statusResult)) || !isBlankBattleSnapshot(readSnapshotText(observedAfterResult))) {
        break;
      }
    }
  }

  return { observedAfterResult, statusResult };
};

const createViewState = (
  options: CliOptions,
  endpoint: string,
  snapshotResult: ToolResult,
  statusResult: ToolResult | undefined,
  recentEventsResult: ToolResult | undefined,
  accumulator: DialogueAccumulator,
  command: CommandModeState,
  settings: {
    linkedAgent?: LinkedAgentProcess;
    agentPaused: boolean;
    manualIntervention?: ManualInterventionState;
    soundEnabled: boolean;
    controlsVisible: boolean;
  },
  runStats: TuiRunStats,
  agentStream?: AgentStreamState,
  activeView: TuiActiveView = "play",
  gameboyRenderer: GameboyRendererMode = "text",
  normalizedSnapshot?: TuiSnapshot,
): TuiViewState => ({
  title: options.command === "play-recorded" ? "PokeCrystal CLI / Recorded Play" : "PokeCrystal CLI / Live Play",
  endpoint,
  sessionId: options.sessionId,
  startedAtMs: runStats.startedAtMs,
  elapsedMs: Date.now() - runStats.startedAtMs,
  interactionCount: runStats.interactionCount,
  snapshot: normalizedSnapshot ?? normalizeTuiSnapshot(snapshotResult, statusResult, recentEventsResult, accumulator),
  agentStream,
  activeView,
  gameboyRenderer,
  gameboyImage: gameboyRenderer === "kitty" ? extractKittyPngFrame(snapshotResult) ?? undefined : undefined,
  settings: {
    agentStatus: settings.linkedAgent ? "running" : settings.agentPaused ? "paused" : "stopped",
    agentPid: settings.linkedAgent?.process.pid,
    agentModel: options.agentModel,
    agentGoal: options.agentGoal,
    agentMaxSteps: options.agentMaxSteps,
    agentGraphCycleSteps: options.agentGraphCycleSteps,
    agentRequestDelayMs: options.agentRequestDelayMs,
    agentIdentityName: options.agentIdentityName,
    soundEnabled: settings.soundEnabled,
  },
  commandNote: command.note,
  commandError: command.isError,
  controlsVisible: settings.controlsVisible,
  livePlay: settings.manualIntervention
    ? {
        active: true,
        remainingMs: Math.max(0, settings.manualIntervention.resumeAt - Date.now()),
        actionCount: settings.manualIntervention.actions.length,
        resuming: settings.manualIntervention.resuming,
      }
    : undefined,
});

const cleanupAll = async (
  input: {
    stdin: NodeJS.ReadStream & { setRawMode?(mode: boolean): void };
    rawMode: boolean;
    onData?: (chunk: string) => void;
    renderer?: InkTuiRenderer;
    client?: TuiMcpClient;
    server?: LocalMcpHttpServer;
    linkedAgent?: LinkedAgentProcess;
    soundController?: TuiSoundController;
    restoreConsole?: ConsoleRestore;
  },
): Promise<void> => {
  try {
    if (input.rawMode) {
      input.stdin.setRawMode?.(false);
    }
    if (input.onData) {
      input.stdin.removeListener("data", input.onData);
    }
    input.stdin.pause();
    input.renderer?.unmount();
    await stopLinkedAgent(input.linkedAgent);
    await input.client?.close();
    await input.server?.close();
    input.soundController?.close?.();
  } finally {
    input.restoreConsole?.();
  }
};

export const runInkTui = async (options: CliOptions, runtime: TextUiRuntime = {}): Promise<void> => {
  const activeOptions = withResolvedSessionLogFile(options);
  const sessionLogger = createSessionLogger(activeOptions);
  sessionLogger.write("session_start", {
    command: activeOptions.command,
    transport: activeOptions.transport,
    base_url: activeOptions.baseUrl,
    log_file: sessionLogger.filePath,
    pid: process.pid,
    cwd: process.cwd(),
    node: process.version,
  });
  const stdin = runtime.stdin ?? process.stdin;
  const stdout = runtime.stdout ?? process.stdout;
  const allowNonInteractive = process.env.POKECRYSTAL_CLI_ALLOW_NON_TTY === "1";
  if ((!stdin.isTTY || !stdout.isTTY) && !allowNonInteractive) {
    const error = new Error("Ink TUI requires an interactive TTY.");
    sessionLogger.write("session_error", { error, phase: "tty_check" });
    throw error;
  }

  let server: LocalMcpHttpServer | undefined;
  const ensureMcpServer = async (): Promise<LocalMcpHttpServer> => {
    if (server) {
      return server;
    }
    try {
      server = await (runtime.startMcpServer ?? startLocalMcpHttpServer)(activeOptions);
      sessionLogger.write("mcp_server_ready", { url: server.url });
      return server;
    } catch (error) {
      sessionLogger.write("session_error", { error, phase: "mcp_server_start" });
      throw error;
    }
  };
  const currentEndpoint = (): string => server?.url ?? "local://pokecrystal-tui";
  const useDirectLocalClient = activeOptions.transport === "local" && !runtime.createMcpClient;
  let client!: TuiMcpClient;
  try {
    if (useDirectLocalClient) {
      client = runtime.createDirectClient?.(activeOptions) ?? createDirectLocalTuiMcpClient(activeOptions);
    } else {
      const activeServer = await ensureMcpServer();
      client = await (runtime.createMcpClient ?? ((url: string) => createTuiMcpClient(url, { fetchImpl: runtime.fetchImpl })))(activeServer.url);
    }
  } catch (error) {
    sessionLogger.write("session_error", { error, phase: "mcp_client_start" });
    await server?.close();
    throw error;
  }
  const launchLinkedAgent = runtime.startLinkedAgent ?? startLinkedAgent;
  let agentOptions: CliOptions = { ...activeOptions };
  let agentStream = agentOptions.agent ? emptyAgentStreamState() : undefined;
  let refreshAgentStream: () => void = () => undefined;
  let linkedAgentGeneration = 0;
  const runStats: TuiRunStats = {
    startedAtMs: Date.now(),
    interactionCount: 0,
  };
  const trainingRecorder = createPlayTrainingRecorder(activeOptions);
  const handleLinkedAgentStreamEvent = (event: AgentStreamEvent): void => {
    if (event.type === "mcp-call" && isGameplayInteractionTool(event.name)) {
      runStats.interactionCount += 1;
    }
    sessionLogger.write("agent_stream_event", { event });
    trainingRecorder?.recordAgentEvent(event);
    agentStream = reduceAgentStreamState(agentStream ?? emptyAgentStreamState(), event);
    refreshAgentStream();
  };
  let linkedAgent: LinkedAgentProcess | undefined;
  if (agentOptions.agent) {
    const activeServer = await ensureMcpServer();
    linkedAgent = launchLinkedAgent(agentOptions, activeServer.url, handleLinkedAgentStreamEvent);
  }
  if (linkedAgent) {
    sessionLogger.write("agent_start", {
      pid: linkedAgent.process.pid,
      note: linkedAgent.note,
      command: agentOptions.agentCommand ?? "run",
    });
  }
  let hasStartedAgent = Boolean(linkedAgent);
  let agentPaused = false;
  let agentPauseReason: AgentPauseReason | undefined;
  const restoreConsole = installTuiConsoleSilencer();
  let soundController: TuiSoundController | undefined;
  try {
    const accumulator = createDialogueAccumulator();
    let activeView: TuiActiveView = "play";
    let gameboyRenderer: GameboyRendererMode = resolveInitialGameboyRenderer();
    let commandState: CommandModeState = { buffer: null };
    let soundEnabled = false;
    let controlsVisible = false;
    soundController = runtime.soundController ?? createTuiSoundController({ stdout });
    const activeSoundController = soundController;
    activeSoundController.setEnabled(soundEnabled);
    if (linkedAgent?.note) {
      commandState = { buffer: null, note: linkedAgent.note };
    }
    let latestStatus: ToolResult | undefined = await client.callTool("status", {});
    let latest = await client.callTool("observe", passiveObserveOptionsFor(gameboyRenderer, latestStatus));
    if (shouldFallbackFromKittyResult(gameboyRenderer, latest)) {
      gameboyRenderer = "text";
      commandState = kittyFallbackCommandNote();
    }
    let latestRecentEvents: ToolResult | undefined = await client.callTool("recent_events", { limit: 8 });
    syncRunStatsFromRecentEvents(runStats, latestRecentEvents);
    activeSoundController.syncSnapshot(extractTuiAudioPlaybackSnapshot(latestStatus));
    const inkRuntime = runtime.inkRuntime ?? await loadInkRuntime();
    const renderer = renderInkTui(
      inkRuntime,
      createViewState(
        agentOptions,
        currentEndpoint(),
        latest,
        latestStatus,
        latestRecentEvents,
        accumulator,
        commandState,
        { linkedAgent, agentPaused, soundEnabled, controlsVisible },
        runStats,
        agentStream,
        activeView,
        gameboyRenderer,
      ),
      { stdin: stdin as NodeJS.ReadStream, stdout: stdout as NodeJS.WriteStream },
    );
    const canUseRawMode = typeof stdin.setRawMode === "function";
    if (canUseRawMode) {
      stdin.setRawMode?.(true);
    }
    stdin.resume();
    stdin.setEncoding("utf8");

    await new Promise<void>((resolve, reject) => {
      let closed = false;
      let agentRefreshTimer: ReturnType<typeof setInterval> | undefined;
      let imageRefreshTimer: ReturnType<typeof setInterval> | undefined;
      let manualInterventionTimer: ReturnType<typeof setInterval> | undefined;
      let runStatsTimer: ReturnType<typeof setInterval> | undefined;
      let heartbeatTimer: ReturnType<typeof setInterval> | undefined;
      let manualIntervention: ManualInterventionState | undefined;
      let imageRefreshQueued = false;
      let inputEpoch = 0;
      let gameplayInputInFlight = false;
      let activeOperation: { label: string; startedAtMs: number } | null = null;
      const manualInterventionIdleMs = resolveManualInterventionIdleMs();
      const finish = async (error?: unknown): Promise<void> => {
        if (closed) {
          return;
        }
        closed = true;
        if (agentRefreshTimer) {
          clearInterval(agentRefreshTimer);
        }
        if (imageRefreshTimer) {
          clearInterval(imageRefreshTimer);
        }
        if (manualInterventionTimer) {
          clearInterval(manualInterventionTimer);
        }
        if (runStatsTimer) {
          clearInterval(runStatsTimer);
        }
        if (heartbeatTimer) {
          clearInterval(heartbeatTimer);
        }
        sessionLogger.write(error ? "session_error" : "session_end", {
          error,
          active_operation: activeOperation,
          interaction_count: runStats.interactionCount,
          elapsed_ms: Date.now() - runStats.startedAtMs,
          latest_status: summarizeToolResult(latestStatus, { maxTextLength: 4_000 }),
          latest_recent_events: summarizeToolResult(latestRecentEvents, { maxTextLength: 4_000 }),
        });
        await cleanupAll({ stdin, rawMode: canUseRawMode, onData, renderer, client, server, linkedAgent, soundController: activeSoundController, restoreConsole });
        if (error) {
          reject(error);
        } else {
          resolve();
        }
      };

      type OperationPriority = "input" | "background";
      type QueuedOperation = {
        operation: () => Promise<void>;
        label: string;
        priority: OperationPriority;
      };
      const operationQueue: QueuedOperation[] = [];
      let operationRunning = false;
      const drainOperationQueue = (): void => {
        if (operationRunning || closed) {
          return;
        }
        const inputIndex = operationQueue.findIndex((entry) => entry.priority === "input");
        const next = inputIndex >= 0
          ? operationQueue.splice(inputIndex, 1)[0]
          : operationQueue.shift();
        if (!next) {
          return;
        }
        operationRunning = true;
        activeOperation = { label: next.label, startedAtMs: Date.now() };
        void next.operation()
          .catch(error => {
            void finish(error);
          })
          .finally(() => {
            activeOperation = null;
            operationRunning = false;
            setImmediate(drainOperationQueue);
          });
      };
      const enqueueOperation = (
        operation: () => Promise<void>,
        label = "operation",
        priority: OperationPriority = "background",
      ): void => {
        operationQueue.push({ operation, label, priority });
        setImmediate(drainOperationQueue);
      };

      let cachedSnapshotInput:
        | {
            latest: ToolResult;
            latestStatus?: ToolResult;
            latestRecentEvents?: ToolResult;
            snapshot: TuiSnapshot;
          }
        | undefined;
      const normalizedSnapshot = (): TuiSnapshot => {
        if (
          cachedSnapshotInput &&
          cachedSnapshotInput.latest === latest &&
          cachedSnapshotInput.latestStatus === latestStatus &&
          cachedSnapshotInput.latestRecentEvents === latestRecentEvents
        ) {
          return cachedSnapshotInput.snapshot;
        }
        const snapshot = normalizeTuiSnapshot(latest, latestStatus, latestRecentEvents, accumulator);
        cachedSnapshotInput = {
          latest,
          latestStatus,
          latestRecentEvents,
          snapshot,
        };
        return snapshot;
      };
      let coalescedRenderQueued = false;
      const refresh = (options: { coalesce?: boolean } = {}): void => {
        if (options.coalesce) {
          if (coalescedRenderQueued || closed) {
            return;
          }
          coalescedRenderQueued = true;
          setTimeout(() => {
            coalescedRenderQueued = false;
            if (!closed) {
              refresh();
            }
          }, 33);
          return;
        }
        renderer.update(createViewState(
          agentOptions,
          currentEndpoint(),
          latest,
          latestStatus,
          latestRecentEvents,
          accumulator,
          commandState,
          { linkedAgent, agentPaused, manualIntervention, soundEnabled, controlsVisible },
          runStats,
          agentStream,
          activeView,
          gameboyRenderer,
          normalizedSnapshot(),
        ));
      };
      refreshAgentStream = () => refresh({ coalesce: true });

      runStatsTimer = setInterval(() => {
        if (!closed) {
          refresh();
        }
      }, 1_000);

      heartbeatTimer = setInterval(() => {
        if (closed) {
          return;
        }
        sessionLogger.write("heartbeat", {
          active_view: activeView,
          active_operation: activeOperation,
          gameplay_input_in_flight: gameplayInputInFlight,
          command_mode: commandState.buffer !== null,
          linked_agent_pid: linkedAgent?.process.pid,
          agent_paused: agentPaused,
          manual_intervention_actions: manualIntervention?.actions.length ?? 0,
          interaction_count: runStats.interactionCount,
          elapsed_ms: Date.now() - runStats.startedAtMs,
          latest_status: summarizeToolResult(latestStatus, { maxTextLength: 4_000 }),
          latest_recent_events: summarizeToolResult(latestRecentEvents, { maxTextLength: 4_000 }),
          latest_snapshot: summarizeToolResult(latest, { maxTextLength: 8_000 }),
        });
      }, 5_000);

      const refreshLatest = async (): Promise<void> => {
        latestStatus = await client.callTool("status", {});
        latest = await client.callTool("observe", passiveObserveOptionsFor(gameboyRenderer, latestStatus));
        if (shouldFallbackFromKittyResult(gameboyRenderer, latest)) {
          gameboyRenderer = "text";
          commandState = kittyFallbackCommandNote();
        }
        latestRecentEvents = await client.callTool("recent_events", { limit: 8 });
        syncRunStatsFromRecentEvents(runStats, latestRecentEvents);
        activeSoundController.syncSnapshot(extractTuiAudioPlaybackSnapshot(latestStatus));
        refresh();
      };

      const refreshImageFrame = async (): Promise<void> => {
        try {
          if (
            closed ||
            commandState.buffer !== null ||
            gameplayInputInFlight ||
            operationQueue.some((entry) => entry.priority === "input") ||
            gameboyRenderer !== "kitty" ||
            isTextAdvanceStatus(latestStatus) ||
            isInstantBattleStatus(latestStatus)
          ) {
            return;
          }
          latest = await client.callTool(
            "observe",
            observeOptionsFor(gameboyRenderer, { advance_frames: TUI_IMAGE_RENDER_ADVANCE_FRAMES }),
          );
          if (shouldFallbackFromKittyResult(gameboyRenderer, latest)) {
            gameboyRenderer = "text";
            commandState = kittyFallbackCommandNote();
          }
          refresh();
        } finally {
          imageRefreshQueued = false;
        }
      };

      imageRefreshTimer = setInterval(() => {
        if (
          closed ||
          imageRefreshQueued ||
          commandState.buffer !== null ||
          gameplayInputInFlight ||
          operationQueue.some((entry) => entry.priority === "input") ||
          gameboyRenderer !== "kitty" ||
          isTextAdvanceStatus(latestStatus) ||
          isInstantBattleStatus(latestStatus)
        ) {
          return;
        }
        imageRefreshQueued = true;
        enqueueOperation(refreshImageFrame, "refresh_image_frame");
      }, TUI_IMAGE_RENDER_REFRESH_MS);

      const attachLinkedAgentHandlers = (agent: LinkedAgentProcess, generation: number): void => {
        agent.process.once("error", error => {
          if (generation !== linkedAgentGeneration) {
            return;
          }
          linkedAgent = undefined;
          agentPaused = false;
          agentPauseReason = undefined;
          commandState = { buffer: null, note: `Agent failed to start: ${error.message}`, isError: true };
          sessionLogger.write("agent_error", {
            pid: agent.process.pid,
            error,
          });
          refresh();
        });
        agent.process.once("exit", (code, signal) => {
          if (generation !== linkedAgentGeneration) {
            return;
          }
          linkedAgent = undefined;
          agentPaused = false;
          agentPauseReason = undefined;
          commandState = { buffer: null, ...formatLinkedAgentExitNote(agent, code, signal) };
          sessionLogger.write("agent_exit", {
            pid: agent.process.pid,
            code,
            signal,
            note: commandState.note,
          });
          enqueueOperation(refreshLatest, "agent_exit_refresh");
        });
      };
      if (linkedAgent) {
        attachLinkedAgentHandlers(linkedAgent, linkedAgentGeneration);
      }
      agentRefreshTimer = setInterval(() => {
        if (
          !linkedAgent ||
          linkedAgent.process.killed ||
          linkedAgent.process.exitCode !== null ||
          linkedAgent.process.signalCode !== null
        ) {
          return;
        }
        enqueueOperation(refreshLatest, "agent_refresh");
      }, 2_000);

      const startOrResumeAgent = async (nextOptions: CliOptions): Promise<LinkedAgentProcess> => {
        linkedAgentGeneration += 1;
        const generation = linkedAgentGeneration;
        hasStartedAgent = true;
        agentPaused = false;
        agentPauseReason = undefined;
        agentStream = agentStream ?? emptyAgentStreamState();
        const activeServer = await ensureMcpServer();
        const agent = launchLinkedAgent(nextOptions, activeServer.url, handleLinkedAgentStreamEvent);
        linkedAgent = agent;
        attachLinkedAgentHandlers(agent, generation);
        sessionLogger.write("agent_start", {
          pid: agent.process.pid,
          note: agent.note,
          command: nextOptions.agentCommand ?? "run",
        });
        return agent;
      };

      const pauseAgent = async (reason: AgentPauseReason = "user"): Promise<void> => {
        if (!linkedAgent) {
          commandState = { buffer: null, note: "Agent is already paused." };
          return;
        }
        const previousAgent = linkedAgent;
        linkedAgentGeneration += 1;
        linkedAgent = undefined;
        agentPaused = true;
        agentPauseReason = reason;
        commandState = {
          buffer: null,
          note: reason === "manual"
            ? "Professor Culligan is taking live control; pausing linked agent..."
            : "Pausing linked agent...",
        };
        refresh();
        await stopLinkedAgent(previousAgent);
        commandState = {
          buffer: null,
          note: reason === "manual"
            ? `Professor Culligan has live control. Agent resumes after ${Math.ceil(manualInterventionIdleMs / 1000)}s idle.`
            : "Agent paused. Use :t to resume.",
        };
      };

      const toggleAgent = async (): Promise<void> => {
        if (linkedAgent) {
          await pauseAgent();
          return;
        }
        if (manualIntervention) {
          await completeManualIntervention();
          return;
        }
        const wasStarted = hasStartedAgent;
        const agent = await startOrResumeAgent({
          ...agentOptions,
          agent: true,
          agentCommand: wasStarted ? "resume" : activeOptions.agentCommand ?? "run",
        });
        commandState = { buffer: null, note: wasStarted ? `Agent resumed (pid ${agent.process.pid ?? "unknown"}).` : agent.note };
      };

      const formatAgentSettingName = (key: NonNullable<CommandModeState["agentSettingCommand"]>["key"]): string => {
        switch (key) {
          case "model":
            return "Agent model";
          case "goal":
            return "Agent goal";
          case "maxSteps":
            return "Agent max steps";
          case "graphCycleSteps":
            return "Agent graph cycle steps";
          case "requestDelayMs":
            return "Agent request delay";
          case "identityName":
            return "Agent identity";
        }
      };

      const applyAgentSetting = async (
        setting: NonNullable<CommandModeState["agentSettingCommand"]>,
      ): Promise<void> => {
        agentOptions = {
          ...agentOptions,
          ...(setting.key === "model" ? { agentModel: setting.value as string | undefined } : {}),
          ...(setting.key === "goal" ? { agentGoal: setting.value as string | undefined } : {}),
          ...(setting.key === "maxSteps" ? { agentMaxSteps: setting.value as number | undefined } : {}),
          ...(setting.key === "graphCycleSteps" ? { agentGraphCycleSteps: setting.value as number | undefined } : {}),
          ...(setting.key === "requestDelayMs" ? { agentRequestDelayMs: setting.value as number | undefined } : {}),
          ...(setting.key === "identityName" ? { agentIdentityName: setting.value as string | undefined } : {}),
        };
        const label = formatAgentSettingName(setting.key);
        const valueText = setting.value === undefined ? "default" : String(setting.value);
        if (!linkedAgent) {
          commandState = { buffer: null, note: `${label} set to ${valueText}. Use :t to start the agent.` };
          return;
        }

        const previousAgent = linkedAgent;
        commandState = { buffer: null, note: `${label} set to ${valueText}. Restarting agent...` };
        refresh();
        linkedAgentGeneration += 1;
        linkedAgent = undefined;
        agentPaused = false;
        agentPauseReason = undefined;
        await stopLinkedAgent(previousAgent);
        const agent = await startOrResumeAgent({
          ...agentOptions,
          agent: true,
          agentCommand: "resume",
        });
        commandState = { buffer: null, note: `${label} set to ${valueText}. Agent restarted (pid ${agent.process.pid ?? "unknown"}).` };
      };

      const interruptAgent = async (message: string): Promise<void> => {
        if (!linkedAgent) {
          commandState = { buffer: null, note: "No linked agent is running for :i.", isError: true };
          return;
        }
        const previousAgent = linkedAgent;
        commandState = { buffer: null, note: `Interrupting agent: ${message}` };
        agentStream = reduceAgentStreamState(agentStream ?? emptyAgentStreamState(), {
          type: "status",
          message: `Professor Culligan's Advice: ${message}`,
          source: "Professor Culligan",
        });
        refresh();
        linkedAgentGeneration += 1;
        linkedAgent = undefined;
        agentPaused = false;
        agentPauseReason = undefined;
        await stopLinkedAgent(previousAgent);
        const agent = await startOrResumeAgent(buildAgentInterruptOptions({ ...agentOptions, agent: true }, message));
        commandState = { buffer: null, note: `Agent resumed with instruction: ${message} (pid ${agent.process.pid ?? "unknown"}).` };
        refresh();
      };

      const beginManualIntervention = async (): Promise<void> => {
        if (manualIntervention) {
          return;
        }
        if (!linkedAgent && !(agentPaused && agentPauseReason === "manual")) {
          return;
        }
        const now = Date.now();
        manualIntervention = {
          startedAt: now,
          lastInputAt: now,
          resumeAt: now + manualInterventionIdleMs,
          actions: [],
          resuming: false,
        };
        agentStream = reduceAgentStreamState(agentStream ?? emptyAgentStreamState(), {
          type: "status",
          message: "Professor Culligan took live control; autonomous agent paused for manual play.",
          source: "Professor Culligan",
        });
        if (linkedAgent) {
          await pauseAgent("manual");
        }
        refresh();
      };

      const recordManualInterventionAction = (
        rawKey: string,
        action: KeyAction,
        actionUpdate: { actionResult: ToolResult; latestStatus: ToolResult },
      ): void => {
        if (!manualIntervention) {
          return;
        }
        const now = Date.now();
        manualIntervention.lastInputAt = now;
        manualIntervention.resumeAt = now + manualInterventionIdleMs;
        manualIntervention.resuming = false;
        manualIntervention.actions.push({
          action: describeManualAction(action),
          rawKey,
          result: firstCompactLine(readSnapshotText(actionUpdate.actionResult), 120),
          state: summarizeStatusResult(actionUpdate.latestStatus),
        });
        commandState = {
          buffer: null,
          note: `Professor Culligan live play recorded ${manualIntervention.actions.length} input${manualIntervention.actions.length === 1 ? "" : "s"}.`,
        };
      };

      async function completeManualIntervention(): Promise<void> {
        if (!manualIntervention) {
          return;
        }
        const completed = manualIntervention;
        completed.resuming = true;
        commandState = { buffer: null, note: "Resuming agent with Professor Culligan's live intervention..." };
        agentStream = reduceAgentStreamState(agentStream ?? emptyAgentStreamState(), {
          type: "status",
          message: "Professor Culligan's Intervention: manual live inputs completed; resuming autonomous play.",
          source: "Professor Culligan",
        });
        refresh();
        const agent = await startOrResumeAgent(
          buildAgentManualInterventionOptions(
            { ...agentOptions, agent: true },
            formatManualInterventionSummary(completed),
          ),
        );
        manualIntervention = undefined;
        commandState = {
          buffer: null,
          note: `Agent resumed after Professor Culligan intervention (pid ${agent.process.pid ?? "unknown"}).`,
        };
        refresh();
      }

      manualInterventionTimer = setInterval(() => {
        if (!manualIntervention || closed) {
          return;
        }
        if (Date.now() >= manualIntervention.resumeAt && !manualIntervention.resuming) {
          manualIntervention.resuming = true;
          enqueueOperation(completeManualIntervention, "complete_manual_intervention");
          refresh();
          return;
        }
        refresh();
      }, manualInterventionRefreshMs(manualInterventionIdleMs));

      let keypressParser = createKeypressChunkParser();
      const onData = (chunk: string): void => {
        if (
          gameplayInputInFlight &&
          !isInstantBattleStatus(latestStatus) &&
          !isCommandKeypressChunk(chunk) &&
          !isPlainOverworldStatus(latestStatus)
        ) {
          return;
        }
        const queuedInputEpoch = inputEpoch;
        enqueueOperation(async () => {
          try {
            if (queuedInputEpoch !== inputEpoch) {
              return;
            }
            let stopBufferedInput = false;
            for (const key of keypressParser.push(chunk)) {
              const escapeOpensCommand =
                key === "\u001b" &&
                !isMenuLikeSurface(latestStatus) &&
                !isMenuLikeSurface(latest);
              if (commandState.buffer !== null || escapeOpensCommand || key === ":") {
                commandState = updateCommandMode(commandState, key);
                if (commandState.quitCommand) {
                  await finish();
                  return;
                }
                if (commandState.viewCycleCommand) {
                  activeView = cycleTuiView(activeView, Boolean(agentStream));
                  commandState = {
                    buffer: null,
                    note: describeTuiView(activeView),
                  };
                }
                if (commandState.gameboyRendererToggleCommand) {
                  const requestedGameboyRenderer = gameboyRenderer === "kitty" ? "text" : "kitty";
                  gameboyRenderer = requestedGameboyRenderer;
                  if (gameboyRenderer === "kitty") {
                    if (!isKittyGraphicsSupported()) {
                      gameboyRenderer = "text";
                      commandState = {
                        buffer: null,
                        note: "Kitty image renderer requested, but terminal support was not detected. Using text.",
                      };
                    } else {
                      commandState = {
                        buffer: null,
                        note: "Kitty Game Boy renderer on. Use :u for text.",
                      };
                      latest = await client.callTool(
                        "observe",
                        passiveObserveOptionsFor(gameboyRenderer, latestStatus),
                      );
                      if (shouldFallbackFromKittyResult(gameboyRenderer, latest)) {
                        gameboyRenderer = "text";
                        commandState = kittyFallbackCommandNote();
                      }
                    }
                  } else {
                    commandState = {
                      buffer: null,
                      note: "Text Game Boy renderer on. Use :u for Kitty/Ghostty image mode.",
                    };
                  }
                }
                if (commandState.soundToggleCommand) {
                  soundEnabled = !soundEnabled;
                  activeSoundController.setEnabled(soundEnabled);
                  commandState = {
                    buffer: null,
                    note: soundEnabled ? "Sound on. Use :a to mute." : "Sound off. Use :a to enable.",
                  };
                  activeSoundController.syncSnapshot(extractTuiAudioPlaybackSnapshot(latestStatus));
                }
                if (commandState.controlsToggleCommand) {
                  controlsVisible = !controlsVisible;
                  commandState = {
                    buffer: null,
                    note: controlsVisible ? "Controls shown. Use :c to hide." : "Controls hidden. Use :c to show.",
                  };
                }
                if (commandState.agentToggleCommand) {
                  await toggleAgent();
                }
                if (commandState.agentSettingCommand) {
                  await applyAgentSetting(commandState.agentSettingCommand);
                }
                if (commandState.agentMessageCommand) {
                  const message = commandState.agentMessageCommand;
                  await interruptAgent(message);
                }
                refresh();
                continue;
              }
              const nameEntryActions = resolveNameEntryKeypressActions(key, latest);
              const menuConfirmAction = nameEntryActions
                ? null
                : resolveLowercaseAAction(key, latestStatus, latest);
              const mappedActions = nameEntryActions ?? [menuConfirmAction ?? mapKeypressToAction(key)];
              for (const mappedAction of mappedActions) {
                if (mappedAction.type === "quit") {
                  await finish();
                  return;
                }
                if (mappedAction.type === "noop") {
                  continue;
                }
                const action = resolveDirectionalAction(mappedAction, latestStatus, latest);
                if (isManualGameplayAction(action)) {
                  await beginManualIntervention();
                }
                const countableGameplayAction = isManualGameplayAction(action);
                const currentStatus = latestStatus;
                if (
                  countableGameplayAction &&
                  !nameEntryActions &&
                  currentStatus &&
                  isInstantBattleStatus(currentStatus)
                ) {
                  gameplayInputInFlight = true;
                  const actionResult = await callAction(client, action, gameboyRenderer, currentStatus, {
                    includeSnapshotText: true,
                    includeTuiState: true,
                  }, {
                    visibleDialogue: hasVisibleDialogue(latest, currentStatus, latestRecentEvents),
                  }).finally(() => {
                    gameplayInputInFlight = false;
                  });
                  if (!actionResult) {
                    continue;
                  }
                  const transientUnavailable = extractTransientToolUnavailable(actionResult);
                  if (transientUnavailable) {
                    commandState = { buffer: null, note: transientUnavailable.message };
                    await refreshLatest();
                    stopBufferedInput = true;
                    break;
                  }
                  throwIfToolError(describeManualAction(action), actionResult);
                  runStats.interactionCount += 1;
                  const bundledState = extractBundledTuiState(actionResult);
                  latest = preserveKittyImageContent(bundledState?.frame ?? actionResult, latest, gameboyRenderer);
                  latestStatus = bundledState?.status ?? currentStatus;
                  latestRecentEvents = bundledState?.recentEvents ?? latestRecentEvents;
                  recordManualInterventionAction(key, action, { actionResult, latestStatus });
                  if (!isBattleMode(readSnapshotText(latest))) {
                    await refreshLatest();
                  }
                  continue;
                }
                gameplayInputInFlight = countableGameplayAction;
                const actionUpdate = await callAndRefreshAction({
                  client,
                  action,
                  rawKey: key,
                  beforeResult: latest,
                  beforeStatusResult: latestStatus,
                  trainingRecorder,
                  gameboyRenderer,
                }).finally(() => {
                  if (countableGameplayAction) {
                    gameplayInputInFlight = false;
                  }
                });
                if (actionUpdate) {
                  if (countableGameplayAction && actionUpdate.applied) {
                    runStats.interactionCount += 1;
                  }
                  if (countableGameplayAction && actionUpdate.applied) {
                    recordManualInterventionAction(key, action, actionUpdate);
                  } else if (
                    countableGameplayAction &&
                    actionUpdate.unavailableMessage &&
                    reasonFromUnavailableMessage(actionUpdate.unavailableMessage) !== "dialogue" &&
                    !hasVisibleDialogue(actionUpdate.latest, actionUpdate.latestStatus, actionUpdate.latestRecentEvents)
                  ) {
                    commandState = { buffer: null, note: actionUpdate.unavailableMessage };
                  }
                  latest = preserveKittyImageContent(actionUpdate.latest, latest, gameboyRenderer);
                  latestStatus = actionUpdate.latestStatus;
                  latestRecentEvents = actionUpdate.latestRecentEvents;
                  activeSoundController.syncSnapshot(extractTuiAudioPlaybackSnapshot(latestStatus));
                  syncRunStatsFromRecentEvents(runStats, latestRecentEvents);
                  if (shouldFallbackFromKittyResult(gameboyRenderer, latest)) {
                    gameboyRenderer = "text";
                    commandState = kittyFallbackCommandNote();
                  }
                  if (actionUpdate.shouldStop) {
                    inputEpoch += 1;
                    keypressParser = createKeypressChunkParser();
                    stopBufferedInput = true;
                    break;
                  }
                }
              }
              if (stopBufferedInput) {
                break;
              }
            }
            refresh();
          } catch (error) {
            await finish(error);
          }
        }, "input", "input");
      };

      stdin.on("data", onData);
    });
  } catch (error) {
    sessionLogger.write("session_error", { error, phase: "startup_or_cleanup" });
    await cleanupAll({ stdin, rawMode: false, client, server, linkedAgent, soundController, restoreConsole });
    throw error;
  }
};

export const runTextUi = runInkTui;
