import fs from "node:fs";
import path from "node:path";
import type { AgentStreamEvent } from "./agent-stream";
import { extractText, parseJsonText } from "./client";
import type { CliOptions, CliPlayTrainingManifest, CliPlayTrainingTurn, KeyAction, ToolResult } from "./types";

const EPISODE_FILENAME = "episode.jsonl";
const EXAMPLES_FILENAME = "examples.jsonl";
const MANIFEST_FILENAME = "manifest.json";
const AGENT_EVENTS_FILENAME = "agent-events.jsonl";
const TRAINING_SCHEMA_VERSION = 2;
const MAX_NO_PROGRESS_PER_STATE = 3;
const MAX_TRAINING_JSONL_BYTES = 128 * 1024 * 1024;

const resolveTrainingDir = (options: CliOptions): string =>
  options.trainingDir?.trim() || path.join(process.cwd(), ".pokecrystal-cli", "runs", options.sessionId, "training");

const extractSnapshotText = (result: ToolResult): string => {
  const textEntries = (result.content ?? [])
    .filter((entry) => entry.type === "text" && typeof entry.text === "string")
    .map((entry) => entry.text?.trim() ?? "")
    .filter((entry) => entry.length > 0);
  const nonJsonEntry = textEntries.find((entry) => {
    try {
      JSON.parse(entry);
      return false;
    } catch {
      return true;
    }
  });
  return nonJsonEntry ?? (extractText(result.content) || "No text snapshot returned.");
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

const parseResultFlags = (
  actionResultSnapshot: string
): { ok?: boolean; changed?: boolean; effect?: string; reason?: string } => ({
  ok: parseCompactBool(actionResultSnapshot, "ok"),
  changed: parseCompactBool(actionResultSnapshot, "ch"),
  effect: parseCompactString(actionResultSnapshot, "fx"),
  reason: parseCompactString(actionResultSnapshot, "rsn"),
});

const buildTags = (
  action: KeyAction,
  statusSnapshot: string,
  actionResultSnapshot: string,
  flags: { ok?: boolean; changed?: boolean; effect?: string; reason?: string }
): string[] => {
  const tags = new Set<string>([action.type]);
  const lowerStatus = statusSnapshot.toLowerCase();
  const lowerActionResult = actionResultSnapshot.toLowerCase();
  if (flags.changed === true) {
    tags.add("progress");
  } else {
    tags.add("no-progress");
  }
  if (flags.ok === false) {
    tags.add("failed");
  }
  if (flags.effect) {
    tags.add(`effect:${flags.effect}`);
  }
  if (flags.reason) {
    tags.add(`reason:${flags.reason}`);
  }
  if (lowerStatus.includes("bat: 1") || lowerStatus.includes("\n  m: battle") || lowerActionResult.includes("battle")) {
    tags.add("battle");
  }
  if (lowerStatus.includes("blk:")) {
    tags.add("blocked");
  }
  if (lowerActionResult.includes("busy") || flags.reason === "busy") {
    tags.add("busy");
  }
  return [...tags];
};

const describeToolCall = (
  action: KeyAction
): { toolName: "move" | "press" | "observe"; toolInput: Record<string, unknown> } | null => {
  if (action.type === "move") {
    return {
      toolName: "move",
      toolInput: { direction: action.direction, steps: 1 },
    };
  }
  if (action.type === "press") {
    return {
      toolName: "press",
      toolInput: { button: action.button, times: 1 },
    };
  }
  if (action.type === "wait") {
    return {
      toolName: "observe",
      toolInput: { advance_frames: action.frames },
    };
  }
  return null;
};

const shouldRecordAction = (action: KeyAction): boolean =>
  action.type === "move" || action.type === "press" || action.type === "wait";

const readManifest = (manifestPath: string): CliPlayTrainingManifest | null => {
  if (!fs.existsSync(manifestPath)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(manifestPath, "utf8")) as CliPlayTrainingManifest;
};

const readEpisodeRows = (episodePath: string): CliPlayTrainingTurn[] => {
  if (!fs.existsSync(episodePath)) {
    return [];
  }
  return fs
    .readFileSync(episodePath, "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line) => JSON.parse(line) as CliPlayTrainingTurn);
};

const appendJsonLine = (filePath: string, value: unknown): void => {
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`);
};

const agentEventText = (event: AgentStreamEvent): string => {
  switch (event.type) {
    case "status":
      return event.message;
    case "text-delta":
    case "thinking-delta":
      return event.text;
    case "tool-call":
      return event.name;
    case "mcp-call":
    case "mcp-result":
      return event.summary ? `${event.name} ${event.summary}` : event.name;
  }
};

const isCurrentSchemaManifest = (manifest: CliPlayTrainingManifest | null): manifest is CliPlayTrainingManifest =>
  Boolean(manifest && manifest.schema_version === TRAINING_SCHEMA_VERSION);

const isCurrentSchemaEpisode = (episodePath: string): boolean => {
  const rows = readEpisodeRows(episodePath);
  if (rows.length === 0) {
    return true;
  }
  return rows.every((row) => "action_result_snapshot" in row && Array.isArray(row.tags));
};

const archiveTrainingFiles = (
  label: "legacy" | "oversized",
  paths: string[]
): void => {
  const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
  for (const filePath of paths) {
    if (fs.existsSync(filePath)) {
      fs.renameSync(filePath, `${filePath}.${label}-${timestamp}`);
    }
  }
};

const isOversizedTrainingJsonl = (filePath: string): boolean => {
  try {
    return fs.statSync(filePath).size > MAX_TRAINING_JSONL_BYTES;
  } catch {
    return false;
  }
};

const buildStateFingerprint = (turn: CliPlayTrainingTurn): string =>
  JSON.stringify({
    after: turn.after_snapshot,
    status: turn.status_snapshot,
    ok: turn.result_flags.ok,
    changed: turn.result_flags.changed,
    effect: turn.result_flags.effect,
    reason: turn.result_flags.reason,
  });

type NoProgressTracker = {
  fingerprint: string | null;
  consecutiveCount: number;
};

const deriveNoProgressTracker = (rows: CliPlayTrainingTurn[]): NoProgressTracker => {
  if (rows.length === 0) {
    return { fingerprint: null, consecutiveCount: 0 };
  }
  let consecutiveCount = 0;
  let fingerprint: string | null = null;
  for (let index = rows.length - 1; index >= 0; index -= 1) {
    const row = rows[index]!;
    if (!("action_result_snapshot" in row) || !row.tags.includes("no-progress")) {
      break;
    }
    const rowFingerprint = buildStateFingerprint(row);
    if (fingerprint === null) {
      fingerprint = rowFingerprint;
      consecutiveCount = 1;
      continue;
    }
    if (rowFingerprint !== fingerprint) {
      break;
    }
    consecutiveCount += 1;
  }
  return { fingerprint, consecutiveCount };
};

const shouldSkipTurn = (tracker: NoProgressTracker, candidate: CliPlayTrainingTurn): boolean => {
  if (!candidate.tags.includes("no-progress")) {
    return false;
  }
  const candidateFingerprint = buildStateFingerprint(candidate);
  return tracker.fingerprint === candidateFingerprint && tracker.consecutiveCount >= MAX_NO_PROGRESS_PER_STATE;
};

const updateNoProgressTracker = (tracker: NoProgressTracker, turn: CliPlayTrainingTurn): void => {
  if (!turn.tags.includes("no-progress")) {
    tracker.fingerprint = null;
    tracker.consecutiveCount = 0;
    return;
  }
  const fingerprint = buildStateFingerprint(turn);
  if (tracker.fingerprint === fingerprint) {
    tracker.consecutiveCount += 1;
    return;
  }
  tracker.fingerprint = fingerprint;
  tracker.consecutiveCount = 1;
};

const shouldIncludeInExamples = (turn: CliPlayTrainingTurn): boolean => {
  if (turn.tags.includes("progress")) {
    return true;
  }
  if (turn.action.type === "wait" && turn.before_snapshot !== turn.after_snapshot) {
    return true;
  }
  if (turn.after_snapshot.includes("BATTLE") || turn.after_snapshot.includes("DIALOGUE") || turn.after_snapshot.includes("MENU")) {
    return true;
  }
  if (turn.tags.includes("no-progress")) {
    return !turn.tags.includes("failed");
  }
  return false;
};

export type PlayTrainingRecorder = {
  recordAgentEvent: (event: AgentStreamEvent) => void;
  recordTurn: (input: {
    rawKey: string;
    action: KeyAction;
    beforeResult: ToolResult;
    actionResult: ToolResult;
    observedAfterResult: ToolResult;
    statusResult: ToolResult;
    recentEventsResult: ToolResult;
  }) => void;
  trainingDir: string;
};

export const createPlayTrainingRecorder = (options: CliOptions): PlayTrainingRecorder | null => {
  if (!options.recordTraining) {
    return null;
  }

  const trainingDir = resolveTrainingDir(options);
  const episodePath = path.join(trainingDir, EPISODE_FILENAME);
  const examplesPath = path.join(trainingDir, EXAMPLES_FILENAME);
  const manifestPath = path.join(trainingDir, MANIFEST_FILENAME);
  const agentEventsPath = path.join(trainingDir, AGENT_EVENTS_FILENAME);
  const trainingFilePaths = [manifestPath, episodePath, examplesPath, agentEventsPath];
  fs.mkdirSync(trainingDir, { recursive: true });

  const initialTimestamp = new Date().toISOString();
  if ([episodePath, examplesPath, agentEventsPath].some(isOversizedTrainingJsonl)) {
    archiveTrainingFiles("oversized", trainingFilePaths);
  }
  const existingManifest = readManifest(manifestPath);
  if ((!isCurrentSchemaManifest(existingManifest) || !isCurrentSchemaEpisode(episodePath)) && (fs.existsSync(manifestPath) || fs.existsSync(episodePath) || fs.existsSync(examplesPath))) {
    archiveTrainingFiles("legacy", trainingFilePaths);
  }
  const initialManifest: CliPlayTrainingManifest =
    readManifest(manifestPath) ?? {
      schema_version: TRAINING_SCHEMA_VERSION,
      session_id: options.sessionId,
      created_at: initialTimestamp,
      updated_at: initialTimestamp,
      transport: options.transport,
      base_url: options.baseUrl,
      training_dir: trainingDir,
      episode_path: episodePath,
      examples_path: examplesPath,
      agent_events_path: agentEventsPath,
      total_turns: 0,
      skipped_turns: 0,
      example_turns: 0,
      total_agent_events: 0,
    };
  initialManifest.agent_events_path = initialManifest.agent_events_path ?? agentEventsPath;
  initialManifest.total_agent_events = initialManifest.total_agent_events ?? 0;
  const noProgressTracker = deriveNoProgressTracker(readEpisodeRows(episodePath));

  fs.writeFileSync(manifestPath, `${JSON.stringify(initialManifest, null, 2)}\n`);

  return {
    trainingDir,
    recordAgentEvent: (event: AgentStreamEvent) => {
      const manifest = readManifest(manifestPath) ?? initialManifest;
      const recordedAt = new Date().toISOString();
      appendJsonLine(agentEventsPath, {
        session_id: options.sessionId,
        recorded_at: recordedAt,
        type: event.type,
        source: event.source,
        text: agentEventText(event),
        event,
      });
      fs.writeFileSync(
        manifestPath,
        `${JSON.stringify(
          {
            ...manifest,
            agent_events_path: agentEventsPath,
            updated_at: recordedAt,
            total_agent_events: (manifest.total_agent_events ?? 0) + 1,
          },
          null,
          2
        )}\n`
      );
    },
    recordTurn: ({ rawKey, action, beforeResult, actionResult, observedAfterResult, statusResult, recentEventsResult }) => {
      if (!shouldRecordAction(action)) {
        return;
      }
      const toolCall = describeToolCall(action);
      if (!toolCall) {
        return;
      }
      const manifest = readManifest(manifestPath) ?? initialManifest;
      const recordedAt = new Date().toISOString();
      const actionResultSnapshot = extractSnapshotText(actionResult);
      const afterSnapshot = extractSnapshotText(observedAfterResult);
      const statusSnapshot = extractSnapshotText(statusResult);
      const recentEventsSnapshot = extractSnapshotText(recentEventsResult);
      const resultFlags = parseResultFlags(actionResultSnapshot);
      const turn: CliPlayTrainingTurn = {
        session_id: options.sessionId,
        recorded_at: recordedAt,
        step_index: manifest.total_turns + 1,
        raw_key: rawKey,
        action,
        tool_name: toolCall.toolName,
        tool_input: toolCall.toolInput,
        before_snapshot: extractSnapshotText(beforeResult),
        action_result_snapshot: actionResultSnapshot,
        after_snapshot: afterSnapshot,
        status_snapshot: statusSnapshot,
        recent_events_snapshot: recentEventsSnapshot,
        result_flags: resultFlags,
        tags: buildTags(action, statusSnapshot, actionResultSnapshot, resultFlags),
        response_meta: {
          action_result: parseJsonText(actionResult.content),
          observe: parseJsonText(observedAfterResult.content),
          status: parseJsonText(statusResult.content),
          recent_events: parseJsonText(recentEventsResult.content),
        },
        transport: options.transport,
      };
      if (shouldSkipTurn(noProgressTracker, turn)) {
        fs.writeFileSync(
          manifestPath,
          `${JSON.stringify(
            {
              ...manifest,
              updated_at: recordedAt,
              skipped_turns: manifest.skipped_turns + 1,
            },
            null,
            2
          )}\n`
        );
        return;
      }
      appendJsonLine(episodePath, turn);
      updateNoProgressTracker(noProgressTracker, turn);
      const includeInExamples = shouldIncludeInExamples(turn);
      const exampleTurns = manifest.example_turns + (includeInExamples ? 1 : 0);
      if (includeInExamples) {
        appendJsonLine(examplesPath, turn);
      }
      fs.writeFileSync(
        manifestPath,
        `${JSON.stringify(
          {
            ...manifest,
            updated_at: recordedAt,
            total_turns: manifest.total_turns + 1,
            example_turns: exampleTurns,
          },
          null,
          2
        )}\n`
      );
    },
  };
};
