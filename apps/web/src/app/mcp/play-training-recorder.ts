import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";

const EPISODE_FILENAME = "episode.jsonl";
const MANIFEST_FILENAME = "manifest.json";
const CHAT_FINETUNE_FILENAME = "chat-finetune.jsonl";
const CHAT_FINETUNE_META_FILENAME = "chat-finetune.meta.jsonl";
const EVENTS_FILENAME = "events.jsonl";
const AGENT_RUN_ROOT = ".pokecrystal-agents";
const DEFAULT_MAX_TRAINING_JSONL_BYTES = 64 * 1024 * 1024;
const WEB_DEV_MODEL = "web-dev-gameplay";
const WEB_DEV_OBJECTIVE = "Drive the game through the dev server with Game Boy-faithful inputs.";
const WEB_DEV_SUBGOAL = "Record browser-driven MCP actions in the same canonical training format as the agent runner.";
const WEB_DEV_MILESTONE = {
  id: "web-dev-playback",
  title: "Web Dev Playback",
  summary: "Browser-driven dev session input capture mirrored into the canonical agent training layout.",
  subgoals: [
    "Capture the observed state before each action.",
    "Record the chosen Game Boy-faithful input in canonical episode rows.",
    "Preserve action outcomes and recent events for downstream training export tooling.",
  ],
} as const;

export type WebTrainingAction =
  | { type: "move"; direction: "up" | "down" | "left" | "right" }
  | { type: "press"; button: "a" | "b" | "start" | "select" | "up" | "down" | "left" | "right" }
  | { type: "wait"; frames: number };

type WebTrainingManifest = {
  session_id: string;
  thread_id: string;
  model: string;
  created_at: string;
  updated_at: string;
  repo_root: string;
  repo_commit?: string;
  run_dir: string;
  training_dir: string;
  episode_path: string;
  chat_finetune_path: string;
  chat_finetune_meta_path: string;
  total_turns: number;
};

type WebTrainingTurn = {
  session_id: string;
  thread_id: string;
  step_index: number;
  recorded_at: string;
  model: string;
  prompt: string;
  current_objective: string;
  current_subgoal: string;
  milestone: typeof WEB_DEV_MILESTONE;
  status: Record<string, unknown>;
  status_raw: string;
  recent_events: Record<string, unknown>;
  recent_events_raw: string;
  observer_text: string;
  decision: Record<string, unknown>;
  action_result: string;
  checkpoint_count: number;
  run_state: Record<string, unknown>;
  tags: {
    changed: boolean;
    stuckRecovery: boolean;
    recoveryReason?: string;
    noProgress: boolean;
    repeatedAction: boolean;
    oscillation: boolean;
    stagnation: boolean;
    toolError: boolean;
  };
};

type WebRunLogEvent = {
  recorded_at: string;
  step_index: number;
  type: "decision_completed" | "action_completed";
  detail?: string;
  payload?: Record<string, unknown>;
};

type RecordWebTrainingTurnInput = {
  sessionId: string;
  baseUrl: string;
  rawKey: string;
  action: WebTrainingAction;
  beforeSnapshot: string;
  actionResultSnapshot: string;
  afterSnapshot: string;
  statusSnapshot: string;
  recentEventsSnapshot: string;
  responseMeta?: {
    action_result: Array<Record<string, unknown>>;
    observe: Array<Record<string, unknown>>;
    status: Array<Record<string, unknown>>;
    recent_events: Array<Record<string, unknown>>;
  };
};

const defaultTrainingDir = (sessionId: string): string =>
  path.join(process.cwd(), AGENT_RUN_ROOT, "runs", sessionId, "training");

const resolveTrainingDir = (sessionId: string): string => {
  const override = process.env.POKECRYSTAL_WEB_TRAINING_DIR?.trim();
  if (!override) {
    return defaultTrainingDir(sessionId);
  }
  if (override.includes("{sessionId}")) {
    return override.replaceAll("{sessionId}", sessionId);
  }
  return path.join(override, sessionId);
};

const getTrainingPaths = (trainingDir: string) => ({
  episodePath: path.join(trainingDir, EPISODE_FILENAME),
  manifestPath: path.join(trainingDir, MANIFEST_FILENAME),
  chatFineTunePath: path.join(trainingDir, CHAT_FINETUNE_FILENAME),
  chatFineTuneMetaPath: path.join(trainingDir, CHAT_FINETUNE_META_FILENAME),
  eventsPath: path.join(trainingDir, EVENTS_FILENAME),
});

const readManifest = (manifestPath: string): WebTrainingManifest | null => {
  if (!fs.existsSync(manifestPath)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(manifestPath, "utf8")) as WebTrainingManifest;
};

const writeManifest = (manifestPath: string, manifest: WebTrainingManifest): void => {
  fs.writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
};

const appendJsonLine = (filePath: string, value: unknown): void => {
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`);
};

const parsePositiveInteger = (value: string | undefined, fallback: number): number => {
  if (!value) {
    return fallback;
  }
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
};

const maxTrainingJsonlBytes = (): number =>
  parsePositiveInteger(process.env.POKECRYSTAL_WEB_MAX_TRAINING_JSONL_BYTES, DEFAULT_MAX_TRAINING_JSONL_BYTES);

const isOversizedTrainingJsonl = (filePath: string): boolean => {
  try {
    return fs.statSync(filePath).size > maxTrainingJsonlBytes();
  } catch {
    return false;
  }
};

const archiveTrainingFiles = (trainingDir: string, paths: ReturnType<typeof getTrainingPaths>): void => {
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  for (const filePath of [
    paths.manifestPath,
    paths.episodePath,
    paths.chatFineTunePath,
    paths.chatFineTuneMetaPath,
    paths.eventsPath,
  ]) {
    if (fs.existsSync(filePath)) {
      fs.renameSync(filePath, `${filePath}.oversized-${timestamp}`);
    }
  }
  fs.mkdirSync(trainingDir, { recursive: true });
};

const rotateOversizedTrainingFiles = (trainingDir: string): void => {
  const paths = getTrainingPaths(trainingDir);
  if ([paths.episodePath, paths.chatFineTunePath, paths.chatFineTuneMetaPath, paths.eventsPath].some(isOversizedTrainingJsonl)) {
    archiveTrainingFiles(trainingDir, paths);
  }
};

const parseCompactBool = (text: string, key: string): boolean | undefined => {
  const match = text.match(new RegExp(`^\\s*${key}:\\s*(\\d+)\\s*$`, "m"));
  if (!match?.[1]) {
    return undefined;
  }
  return match[1] === "1";
};

const parseCompactString = (text: string, key: string): string | undefined => {
  const match = text.match(new RegExp(`^\\s*${key}:\\s*(.+)$`, "m"));
  return match?.[1]?.trim();
};

const parseCoords = (value: string | undefined): [number, number] | undefined => {
  if (!value) {
    return undefined;
  }
  const match = value.match(/^\s*(-?\d+)\s*,\s*(-?\d+)\s*$/);
  if (!match?.[1] || !match[2]) {
    return undefined;
  }
  return [Number(match[1]), Number(match[2])];
};

const parseResultFlags = (
  actionResultSnapshot: string
): { ok?: boolean; changed?: boolean; effect?: string; reason?: string } => ({
  ok: parseCompactBool(actionResultSnapshot, "ok"),
  changed: parseCompactBool(actionResultSnapshot, "ch"),
  effect: parseCompactString(actionResultSnapshot, "fx"),
  reason: parseCompactString(actionResultSnapshot, "rsn"),
});

const describeDecision = (action: WebTrainingAction): Record<string, unknown> => {
  if (action.type === "move") {
    return {
      reasoning: `Move ${action.direction} from the current overworld position.`,
      actionType: "move",
      direction: action.direction,
      times: 1,
    };
  }
  if (action.type === "wait") {
    return {
      reasoning: `Wait ${action.frames} frames before deciding again.`,
      actionType: "wait",
      frames: action.frames,
    };
  }
  return {
    reasoning: `Press ${action.button.toUpperCase()} from the current game state.`,
    actionType: "press",
    button: action.button,
    times: 1,
  };
};

const parseStatusObject = (statusSnapshot: string): Record<string, unknown> => {
  const map = parseCompactString(statusSnapshot, "map");
  const coords = parseCoords(parseCompactString(statusSnapshot, "xy"));
  return {
    mode: parseCompactString(statusSnapshot, "m"),
    map,
    location: map,
    coords,
    facing: parseCompactString(statusSnapshot, "dir"),
    inBattle: parseCompactBool(statusSnapshot, "bat"),
    inMenu: parseCompactBool(statusSnapshot, "menu"),
    inDialog: parseCompactBool(statusSnapshot, "dlg"),
    textBoxOpen: parseCompactBool(statusSnapshot, "txt"),
    promptPending: parseCompactBool(statusSnapshot, "pr"),
    movementLocked: parseCompactBool(statusSnapshot, "lock"),
    scriptBusy: parseCompactBool(statusSnapshot, "busy"),
    canMove: parseCompactBool(statusSnapshot, "mv"),
    blockedReason: parseCompactString(statusSnapshot, "blk"),
    raw: statusSnapshot,
  };
};

const parseRecentEventsObject = (recentEventsSnapshot: string): Record<string, unknown> => {
  const eventList = parseCompactString(recentEventsSnapshot, "ev")
    ?.split("|")
    .map((entry) => entry.trim())
    .filter((entry) => entry.length > 0)
    .map((summary) => ({ summary })) ?? [];
  const totalValue = parseCompactString(recentEventsSnapshot, "n");
  const total = totalValue && /^\d+$/.test(totalValue) ? Number(totalValue) : eventList.length;
  return {
    total,
    summary: parseCompactString(recentEventsSnapshot, "sum"),
    truncated: parseCompactBool(recentEventsSnapshot, "tr"),
    events: eventList,
    raw: recentEventsSnapshot,
  };
};

const buildPrompt = (
  beforeSnapshot: string,
  statusSnapshot: string,
  recentEventsSnapshot: string
): string =>
  [
    "Current objective:",
    WEB_DEV_OBJECTIVE,
    "",
    "Current subgoal:",
    WEB_DEV_SUBGOAL,
    "",
    "Observed state:",
    beforeSnapshot,
    "",
    "Status:",
    statusSnapshot,
    "",
    "Recent events:",
    recentEventsSnapshot,
    "",
    "Choose the next Game Boy-faithful input.",
  ].join("\n");

const tryResolveRepoCommit = (): string | undefined => {
  try {
    const output = execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: process.cwd(),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    return output || undefined;
  } catch {
    return undefined;
  }
};

const ensureManifest = (
  sessionId: string,
  trainingDir: string,
  recordedAt: string
): WebTrainingManifest => {
  const paths = getTrainingPaths(trainingDir);
  fs.mkdirSync(trainingDir, { recursive: true });
  const existing = readManifest(paths.manifestPath);
  if (existing) {
    return existing;
  }
  const runDir = path.dirname(trainingDir);
  const manifest: WebTrainingManifest = {
    session_id: sessionId,
    thread_id: sessionId,
    model: WEB_DEV_MODEL,
    created_at: recordedAt,
    updated_at: recordedAt,
    repo_root: process.cwd(),
    repo_commit: tryResolveRepoCommit(),
    run_dir: runDir,
    training_dir: trainingDir,
    episode_path: paths.episodePath,
    chat_finetune_path: paths.chatFineTunePath,
    chat_finetune_meta_path: paths.chatFineTuneMetaPath,
    total_turns: 0,
  };
  writeManifest(paths.manifestPath, manifest);
  return manifest;
};

const appendRunLogEvent = (trainingDir: string, event: WebRunLogEvent): void => {
  const { eventsPath } = getTrainingPaths(trainingDir);
  appendJsonLine(eventsPath, event);
};

export const shouldRecordWebTraining = (): boolean => {
  const env = process.env.POKECRYSTAL_WEB_RECORD_TRAINING;
  if (env === "0") {
    return false;
  }
  if (env === "1") {
    return true;
  }
  return process.env.NODE_ENV === "development";
};

export const recordWebTrainingTurn = (input: RecordWebTrainingTurnInput): void => {
  if (!shouldRecordWebTraining()) {
    return;
  }

  const trainingDir = resolveTrainingDir(input.sessionId);
  rotateOversizedTrainingFiles(trainingDir);
  const paths = getTrainingPaths(trainingDir);
  const recordedAt = new Date().toISOString();
  const manifest = ensureManifest(input.sessionId, trainingDir, recordedAt);
  const resultFlags = parseResultFlags(input.actionResultSnapshot);
  const changed = resultFlags.changed === true;
  const toolError = resultFlags.ok === false;
  const status = parseStatusObject(input.statusSnapshot);
  const recentEvents = parseRecentEventsObject(input.recentEventsSnapshot);
  const stepIndex = manifest.total_turns + 1;
  const turn: WebTrainingTurn = {
    session_id: input.sessionId,
    thread_id: input.sessionId,
    step_index: stepIndex,
    recorded_at: recordedAt,
    model: WEB_DEV_MODEL,
    prompt: buildPrompt(input.beforeSnapshot, input.statusSnapshot, input.recentEventsSnapshot),
    current_objective: WEB_DEV_OBJECTIVE,
    current_subgoal: WEB_DEV_SUBGOAL,
    milestone: WEB_DEV_MILESTONE,
    status,
    status_raw: input.statusSnapshot,
    recent_events: recentEvents,
    recent_events_raw: input.recentEventsSnapshot,
    observer_text: input.beforeSnapshot,
    decision: describeDecision(input.action),
    action_result: input.actionResultSnapshot,
    checkpoint_count: 0,
    run_state: {
      sessionId: input.sessionId,
      threadId: input.sessionId,
      currentObjective: WEB_DEV_OBJECTIVE,
      currentSubgoal: WEB_DEV_SUBGOAL,
      badges: 0,
      lastKnownMap: status.map,
      lastKnownCoords: status.coords,
      stepCount: stepIndex,
      noProgressCounters: {
        repeatedAction: 0,
        oscillation: 0,
        stagnation: changed ? 0 : 1,
      },
      stuckRecoveryStage: 0,
      battleRecoveryNotes: [],
      lastAction: input.rawKey,
      lastEventSummary: recentEvents.summary,
      checkpointCount: 0,
      updatedAt: recordedAt,
    },
    tags: {
      changed,
      stuckRecovery: false,
      recoveryReason: resultFlags.reason === "busy" ? "busy" : undefined,
      noProgress: !changed,
      repeatedAction: false,
      oscillation: false,
      stagnation: !changed,
      toolError,
    },
  };

  appendRunLogEvent(trainingDir, {
    recorded_at: recordedAt,
    step_index: stepIndex,
    type: "decision_completed",
    detail: JSON.stringify(turn.decision),
    payload: {
      action: input.action,
      raw_key: input.rawKey,
      base_url: input.baseUrl,
    },
  });
  appendJsonLine(paths.episodePath, turn);
  appendRunLogEvent(trainingDir, {
    recorded_at: recordedAt,
    step_index: stepIndex,
    type: "action_completed",
    detail: input.actionResultSnapshot,
    payload: {
      changed,
      tool_error: toolError,
      action: input.action,
      response_meta: input.responseMeta,
    },
  });
  writeManifest(paths.manifestPath, {
    ...manifest,
    updated_at: recordedAt,
    total_turns: stepIndex,
  });
};
