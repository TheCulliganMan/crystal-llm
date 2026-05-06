import { Agent } from "@mastra/core/agent";
import { Memory } from "@mastra/memory";
import type { Tool } from "@mastra/core/tools";

import {
  taskmasterBatchSchema,
  type SessionAuth,
  type RunnerInput,
  type Status,
  type TaskmasterBatch,
} from "./types.js";
import {
  buildPlayerDelegationPrompt,
  buildPlayerInstructions,
  buildTaskmasterInstructions,
  buildTaskmasterPrompt,
  buildTaskmasterSummaryPrompt,
} from "./prompts.js";
import { createPlayerTools } from "./tools.js";
import type { KrabbyClawSession } from "./session.js";
import { createTaskmasterMemoryScope } from "./memory.js";
import {
  getDefaultAnthropicApiKey,
  getDefaultAnthropicApiVersion,
  getDefaultAnthropicBaseUrl,
  getDefaultAzureOpenAIApiKey,
  getDefaultAzureOpenAIApiVersion,
  getDefaultAzureOpenAIEndpoint,
  getDefaultGoogleApiBaseUrl,
  getDefaultGoogleApiKey,
  getDefaultOllamaApiBaseUrl,
  getDefaultOpenAIApiKey,
  getDefaultOpenAIBaseUrl,
  normalizeAgentModel,
  resolveMastraModel,
  resolveMastraProviderOptions,
} from "./defaults.js";
import { buildNoActionAgenticBatch, isCodexModel, runCodexTaskmasterBatch } from "./codex-harness.js";
import { emitAgentStreamChunk, emitAgentStreamStatus } from "./stream-events.js";

export function shouldContinueTaskmasterIteration(params: {
  delegatedToPlayer: boolean;
  iteration: number;
  supervisorMaxSteps: number;
}): { continue: boolean; feedback?: string } {
  if (!params.delegatedToPlayer && params.iteration < params.supervisorMaxSteps) {
    return {
      continue: true,
      feedback:
        "You have not yet delegated real gameplay to the player agent. Delegate now and use that result before concluding the batch.",
    };
  }

  return { continue: false };
}

export const isPlayerDelegationPrimitive = (primitiveId: string): boolean =>
  primitiveId === "player" || primitiveId === "agent-player";

const isDirectProviderModel = (model: string): boolean => {
  const normalized = normalizeAgentModel(model);
  return (
    normalized.startsWith("ollama/") ||
    normalized.startsWith("azure-openai/") ||
    normalized.startsWith("openai-direct/") ||
    normalized.startsWith("anthropic/") ||
    normalized.startsWith("google/") ||
    normalized.startsWith("gemini/")
  );
};

export const shouldUseDirectPlayerBatch = (input: Pick<RunnerInput, "taskmasterModel" | "playerModel">): boolean =>
  isDirectProviderModel(input.taskmasterModel) || isDirectProviderModel(input.playerModel);

const parseRecentEventsTotal = (recentEvents: string): number | null => {
  try {
    const parsed = JSON.parse(recentEvents) as { total?: unknown };
    return typeof parsed.total === "number" && Number.isFinite(parsed.total) ? parsed.total : null;
  } catch {
    return null;
  }
};

const parseRecentEventsPayload = (recentEvents: string): {
  total: number | null;
  summary: string;
  events: Array<Record<string, unknown>>;
} => {
  try {
    const parsed = JSON.parse(recentEvents) as {
      total?: unknown;
      summary?: unknown;
      events?: unknown;
    };
    return {
      total: typeof parsed.total === "number" && Number.isFinite(parsed.total) ? parsed.total : null,
      summary: typeof parsed.summary === "string" ? parsed.summary : "",
      events: Array.isArray(parsed.events)
        ? parsed.events.filter((event): event is Record<string, unknown> =>
            Boolean(event && typeof event === "object" && !Array.isArray(event)),
          )
        : [],
    };
  } catch {
    return {
      total: null,
      summary: recentEvents,
      events: [],
    };
  }
};

const isMeaningfulRecentEvent = (event: Record<string, unknown>): boolean => {
  const text = [
    typeof event.summary === "string" ? event.summary : "",
    typeof event.action === "string" ? event.action : "",
    typeof event.reason === "string" ? event.reason : "",
  ].join(" ");
  if (event.changed === false || /\bno_?change\b/i.test(text)) {
    return false;
  }
  return text.trim().length > 0;
};

const formatDirectBatchStatus = (status: Status): string =>
  [
    status.mode,
    status.map,
    status.coords ? status.coords.join(",") : "unknown",
    status.facing ? `facing=${status.facing}` : null,
    status.canMove === false ? `blocked=${status.blockedReason ?? "unknown"}` : null,
    status.flowNextGoal ? `goal=${status.flowNextGoal}` : null,
  ].filter(Boolean).join(" ");

export function didRecentEventsAdvance(before: string, after: string): boolean {
  const beforePayload = parseRecentEventsPayload(before);
  const afterPayload = parseRecentEventsPayload(after);
  const beforeTotal = beforePayload.total;
  const afterTotal = afterPayload.total;
  if (beforeTotal !== null && afterTotal !== null) {
    if (afterTotal <= beforeTotal) {
      return false;
    }
    const newEventCount = afterTotal - beforeTotal;
    const newEvents = afterPayload.events.slice(-newEventCount);
    if (newEvents.length > 0) {
      return newEvents.some(isMeaningfulRecentEvent);
    }
    return !/\bno_?change\b/i.test(afterPayload.summary);
  }
  return after.trim().length > 0 && after !== before && !/\bno_?change\b/i.test(afterPayload.summary);
}

export function hasDirectPlayerBatchProgress(params: {
  beforeStatus: Status;
  afterStatus: Status;
  recentEventsBefore: string;
  recentEventsAfter: string;
  playerText: string;
}): boolean {
  const stateChanged =
    JSON.stringify({
      map: params.beforeStatus.map,
      coords: params.beforeStatus.coords,
      facing: params.beforeStatus.facing,
      surface: params.beforeStatus.surface,
      flowNextGoal: params.beforeStatus.flowNextGoal,
      badges: params.beforeStatus.badges,
      partyCount: params.beforeStatus.partyCount,
    }) !==
    JSON.stringify({
      map: params.afterStatus.map,
      coords: params.afterStatus.coords,
      facing: params.afterStatus.facing,
      surface: params.afterStatus.surface,
      flowNextGoal: params.afterStatus.flowNextGoal,
      badges: params.afterStatus.badges,
      partyCount: params.afterStatus.partyCount,
    });

  return (
    stateChanged ||
    didRecentEventsAdvance(params.recentEventsBefore, params.recentEventsAfter)
  );
}

export async function createPlayerAgent(session: KrabbyClawSession, model: string): Promise<Agent> {
  const providerOptions = resolveMastraProviderOptions(model);
  const compactTools = normalizeAgentModel(model).startsWith("ollama/");

  return new Agent({
    id: "pokemon-crystal-player",
    name: "Pokemon Crystal Determined Trainer",
    description:
      "Acts as the trainer in motion, executing the immediate gameplay objective with honest Game Boy-valid actions against the live KrabbyClaw MCP surface.",
    instructions: buildPlayerInstructions(),
    model: resolveMastraModel(model),
    defaultOptions: providerOptions ? { providerOptions } : undefined,
    tools: (await createPlayerTools(session, { compact: compactTools })) as Record<string, Tool<any, any, any, any>>,
  });
}

export function createTaskmasterAgent(playerAgent: Agent, model: string, memory: Memory): Agent {
  const providerOptions = resolveMastraProviderOptions(model);

  return new Agent({
    id: "pokemon-crystal-taskmaster",
    name: "Pokemon Crystal Trainer Self",
    description:
      "Acts as the trainer's self, plans the immediate goal, delegates live play to the trainer in motion, and returns a checkpoint summary.",
    instructions: buildTaskmasterInstructions(),
    model: resolveMastraModel(model),
    agents: {
      player: playerAgent,
    },
    defaultOptions: providerOptions ? { providerOptions } : undefined,
    memory,
  });
}

export async function consumeAgentStream(
  agent: {
    stream: (
      prompt: string,
      options: Record<string, unknown>,
    ) => Promise<{ consumeStream: () => Promise<void>; object?: Promise<unknown>; text?: Promise<string> }>;
  },
  prompt: string,
  options: Record<string, unknown>,
) {
  const stream = await agent.stream(prompt, options);
  await stream.consumeStream();
  return stream;
}

type LocalPlayerAction =
  | { action: "press"; parameters?: { button?: string }; reason?: string }
  | { action: "move"; parameters?: { direction?: string; steps?: number }; reason?: string }
  | { action: "type_text"; parameters?: { text?: string; clear?: boolean; submit?: boolean }; reason?: string };

type LocalOllamaChatRequest = {
  model: string;
  temperature: number;
  max_tokens: number;
  response_format: {
    type: "json_schema";
    json_schema: {
      name: string;
      strict: boolean;
      schema: Record<string, unknown>;
    };
  };
  messages: Array<{ role: "system" | "user"; content: string }>;
};

const normalizeButton = (value: unknown): "A" | "B" | "Start" | "Select" | "Up" | "Down" | "Left" | "Right" | null => {
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.trim().toLowerCase();
  if (normalized === "a") return "A";
  if (normalized === "b") return "B";
  if (normalized === "start") return "Start";
  if (normalized === "select") return "Select";
  if (normalized === "up") return "Up";
  if (normalized === "down") return "Down";
  if (normalized === "left") return "Left";
  if (normalized === "right") return "Right";
  return null;
};

const normalizeDirection = (value: unknown): "up" | "down" | "left" | "right" | null => {
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.trim().toLowerCase();
  return normalized === "up" || normalized === "down" || normalized === "left" || normalized === "right"
    ? normalized
    : null;
};

function extractFirstJsonObject(text: string): Record<string, unknown> | null {
  for (let start = text.indexOf("{"); start >= 0; start = text.indexOf("{", start + 1)) {
    let depth = 0;
    let inString = false;
    let escaped = false;
    for (let index = start; index < text.length; index += 1) {
      const char = text[index];
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
      } else if (char === "{") {
        depth += 1;
      } else if (char === "}") {
        depth -= 1;
        if (depth === 0) {
          try {
            const parsed = JSON.parse(text.slice(start, index + 1));
            return parsed && typeof parsed === "object" && !Array.isArray(parsed)
              ? parsed as Record<string, unknown>
              : null;
          } catch {
            break;
          }
        }
      }
    }
  }
  return null;
}

function parseLocalPlayerAction(text: string): LocalPlayerAction | null {
  const parsed = extractFirstJsonObject(text);
  if (!parsed) {
    return null;
  }
  const action = typeof parsed.action === "string"
    ? parsed.action
    : typeof parsed.tool === "string"
      ? parsed.tool
      : null;
  const parameters = parsed.parameters && typeof parsed.parameters === "object" && !Array.isArray(parsed.parameters)
    ? parsed.parameters as Record<string, unknown>
    : parsed;
  const reason = typeof parsed.reason === "string" ? parsed.reason : undefined;
  if (action === "press") {
    return { action, parameters: { button: typeof parameters.button === "string" ? parameters.button : undefined }, reason };
  }
  if (action === "move") {
    return {
      action,
      parameters: {
        direction: typeof parameters.direction === "string" ? parameters.direction : undefined,
        steps: typeof parameters.steps === "number" ? parameters.steps : undefined,
      },
      reason,
    };
  }
  if (action === "type_text") {
    return {
      action,
      parameters: {
        text: typeof parameters.text === "string" ? parameters.text : undefined,
        clear: typeof parameters.clear === "boolean" ? parameters.clear : undefined,
        submit: typeof parameters.submit === "boolean" ? parameters.submit : undefined,
      },
      reason,
    };
  }
  return null;
}

export function buildLocalOllamaActionRequest(params: {
  model: string;
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
  rejectedActionNote?: string;
}): LocalOllamaChatRequest {
  return {
    model: params.model,
    temperature: 0,
    max_tokens: 160,
    response_format: {
      type: "json_schema",
      json_schema: {
        name: "pokemon_crystal_action",
        strict: true,
        schema: {
          type: "object",
          properties: {
            action: { type: "string", enum: ["press", "move", "type_text"] },
            parameters: {
              type: "object",
              properties: {
                button: { type: "string", enum: ["A", "B", "Start", "Select", "Up", "Down", "Left", "Right"] },
                direction: { type: "string", enum: ["up", "down", "left", "right"] },
                steps: { type: "integer", minimum: 1, maximum: 6 },
                text: { type: "string", minLength: 1, maxLength: 32 },
                clear: { type: "boolean" },
                submit: { type: "boolean" },
              },
              additionalProperties: false,
            },
            reason: { type: "string", minLength: 1 },
          },
          required: ["action", "parameters", "reason"],
          additionalProperties: false,
        },
      },
    },
    messages: [
      {
        role: "system",
        content: [
          "You are a Pokemon Crystal gameplay agent.",
          "Use only the live status, observation, map_info, and recent events.",
          "Return exactly one JSON object and no prose.",
          "Allowed actions are press, move, and type_text.",
          "Never wait, idle, defer, skip, or ask for user input; every response must be one concrete gameplay action.",
          "Allowed press buttons are A, B, Start, Select, Up, Down, Left, Right.",
          "A press action must be {\"action\":\"press\",\"parameters\":{\"button\":\"A\"},\"reason\":\"...\"}.",
          "A move action must be {\"action\":\"move\",\"parameters\":{\"direction\":\"down\",\"steps\":1},\"reason\":\"...\"}.",
          "On text-entry screens, prefer {\"action\":\"type_text\",\"parameters\":{\"text\":\"ABCD\",\"clear\":true,\"submit\":true},\"reason\":\"...\"}.",
          "If the live surface, mode, map, or observation says name_entry or text entry, use type_text for a complete valid entry instead of pressing Start or cursor-walking letters.",
          "Do not wait for user input during text entry; choose a short valid in-game text value and submit it in the same type_text action.",
          "If recent events show an action failed, was menu_locked, or changed false, do not repeat that action; choose a different valid action.",
          "In overworld play, use map_info and HOTSPOTS to choose route transitions, doors, warps, exits, NPCs, and item balls that advance the current flow.",
          "When hotspot text gives directional deltas, choose a move that reduces the largest useful delta toward the chosen gameplay target.",
          "When map_info gives player coords and visible warp or door coords, compare x and y numerically; if x differs, move left or right toward that x before using up or down to enter.",
          "For movement, route through visible floor tiles. Do not try to walk onto utility, sign, bookshelf, PC, NPC, wall, or object tokens; move around them unless interacting with that object is the chosen story action.",
          "Enter warps and doors with movement in their indicated direction; do not press A on a warp or door tile.",
          "Utility and object hotspots are not story route targets unless the current flow goal, local focus, forced prompt, or recent NPC/object clue explicitly elevates that same hotspot.",
          "Avoid optional furniture, signs, and repeated local objects when a route transition or stronger story target is visible.",
          "After the same NPC or object dialogue closes repeatedly without changing map, party, or flow, leave that interaction lane and route to visible transitions.",
          "After a failed or blocked move, the same direction is forbidden for the next action unless the player coords changed after that failure.",
        ].join(" "),
      },
      {
          role: "user",
          content: [
            `Goal: ${params.input.immediateGoal}`,
            `Status: ${JSON.stringify(sanitizeLocalPromptStatus(params.status, params.recentEvents))}`,
          `Map info: ${sanitizeLocalPromptMapInfo(params.mapInfo, params.status, params.recentEvents).slice(0, 1600)}`,
          `Observation: ${sanitizeLocalPromptObservation(params.observation, params.status, params.recentEvents).slice(0, 1600)}`,
          `Recent events: ${params.recentEvents.slice(0, 800)}`,
          params.rejectedActionNote ? `Rejected candidates forbidden for this live state: ${params.rejectedActionNote}` : null,
          "Choose the next useful valid Pokemon Crystal action now.",
        ].filter(Boolean).join("\n"),
      },
    ],
  };
}

function normalizeMapReference(value: string): string {
  return value
    .split("")
    .filter(char => {
      const lower = char.toLowerCase();
      return (lower >= "a" && lower <= "z") || (char >= "0" && char <= "9");
    })
    .join("")
    .toLowerCase();
}

function parseRecentMapTransition(recentEvents: string | undefined): { from: string; to: string } | null {
  if (!recentEvents) {
    return null;
  }
  try {
    const parsed = JSON.parse(recentEvents) as { events?: unknown };
    if (!Array.isArray(parsed.events)) {
      return null;
    }
    for (const event of parsed.events.slice().reverse()) {
      if (!event || typeof event !== "object" || Array.isArray(event)) {
        continue;
      }
      const record = event as Record<string, unknown>;
      for (const candidate of [record.summary, record.action]) {
        if (typeof candidate !== "string") {
          continue;
        }
        const start = candidate.indexOf("map:");
        if (start < 0) {
          continue;
        }
        const transition = candidate.slice(start + "map:".length).split(" ")[0] ?? "";
        const arrow = transition.indexOf("->");
        if (arrow <= 0) {
          continue;
        }
        const from = transition.slice(0, arrow);
        const to = transition.slice(arrow + "->".length);
        if (from && to) {
          return { from, to };
        }
      }
    }
  } catch {
    return null;
  }
  return null;
}

function isRecentlyUsedReverseTransition(label: string, status: Status, recentEvents?: string): boolean {
  const transition = parseRecentMapTransition(recentEvents);
  if (!transition || normalizeMapReference(status.map) !== normalizeMapReference(transition.to)) {
    return false;
  }
  return normalizeMapReference(label).includes(normalizeMapReference(transition.from));
}

function hasRepeatedDialogueAtCurrentCoords(status: Status, recentEvents?: string): boolean {
  if (!recentEvents || !status.coords) {
    return false;
  }
  try {
    const parsed = JSON.parse(recentEvents) as { events?: unknown };
    if (!Array.isArray(parsed.events)) {
      return false;
    }
    let count = 0;
    for (const event of parsed.events.slice().reverse()) {
      if (!event || typeof event !== "object" || Array.isArray(event)) {
        continue;
      }
      const record = event as Record<string, unknown>;
      const coords = Array.isArray(record.coords) ? record.coords : null;
      if (!coords || coords[0] !== status.coords[0] || coords[1] !== status.coords[1]) {
        continue;
      }
      const action = typeof record.action === "string" ? record.action : "";
      const summary = typeof record.summary === "string" ? record.summary : "";
      if (
        action.startsWith("press:a") ||
        action.startsWith("execute_macro:advance_dialog") ||
        summary.includes("text advance opened") ||
        summary.includes("text advance closed")
      ) {
        count += 1;
      }
    }
    return count >= 4;
  } catch {
    return false;
  }
}

export function sanitizeLocalPromptMapInfo(mapInfo: string, status: Status, recentEvents?: string): string {
  try {
    const parsed = JSON.parse(mapInfo) as { hotspots?: unknown };
    if (!parsed || typeof parsed !== "object" || !Array.isArray(parsed.hotspots)) {
      return mapInfo;
    }
    const flow = status.flowNextGoal.toLowerCase();
    const repeatedDialogue = hasRepeatedDialogueAtCurrentCoords(status, recentEvents);
    const routeTargets = parsed.hotspots.filter(hotspot => {
      if (!hotspot || typeof hotspot !== "object" || Array.isArray(hotspot)) {
        return false;
      }
      const record = hotspot as Record<string, unknown>;
      const type = typeof record.type === "string" ? record.type : "";
      const token = typeof record.token === "string" ? record.token : "";
      return token === "D" || type === "warp" || type === "door";
    });
    const hotspots = parsed.hotspots.filter(hotspot => {
      if (!hotspot || typeof hotspot !== "object" || Array.isArray(hotspot)) {
        return true;
      }
      const record = hotspot as Record<string, unknown>;
      const type = typeof record.type === "string" ? record.type : "";
      const token = typeof record.token === "string" ? record.token : "";
      const label = typeof record.label === "string" ? record.label.toLowerCase() : "";
      if (token === "D" || type === "warp" || type === "door") {
        if (routeTargets.length > 1 && isRecentlyUsedReverseTransition(label, status, recentEvents)) {
          return false;
        }
        return true;
      }
      if (repeatedDialogue && (token === "N" || type === "npc")) {
        return false;
      }
      if ((type === "utility" || type === "sign" || type === "object") && (!label || !flow.includes(label))) {
        return false;
      }
      return true;
    });
    return JSON.stringify({ ...parsed, hotspots });
  } catch {
    return mapInfo;
  }
}

export function sanitizeLocalPromptObservation(observation: string, status: Status, recentEvents?: string): string {
  const flow = status.flowNextGoal.toLowerCase();
  const repeatedDialogue = hasRepeatedDialogueAtCurrentCoords(status, recentEvents);
  const lines = observation.split("\n");
  let inHotspots = false;
  const kept: string[] = [];
  const routeTargetCount = lines.filter(line => {
    const trimmed = line.trim();
    return trimmed === "D" || trimmed.startsWith("D ");
  }).length;
  const isSectionHeader = (value: string): boolean =>
    value.length > 0 &&
    value === value.toUpperCase() &&
    value.split("").every(char => (char >= "A" && char <= "Z") || char === "_" || char === " ");
  const startsWithToken = (value: string, token: string): boolean =>
    value === token || value.startsWith(`${token} `);

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed === "HOTSPOTS") {
      inHotspots = true;
      kept.push(line);
      continue;
    }
    if (inHotspots && isSectionHeader(trimmed) && trimmed !== "HOTSPOTS") {
      inHotspots = false;
    }
    if (inHotspots && routeTargetCount > 1 && startsWithToken(trimmed, "D") && isRecentlyUsedReverseTransition(trimmed, status, recentEvents)) {
      continue;
    }
    if (inHotspots && repeatedDialogue && startsWithToken(trimmed, "N")) {
      continue;
    }
    if (
      inHotspots &&
      (startsWithToken(trimmed, "P") || startsWithToken(trimmed, "S") || startsWithToken(trimmed, "B"))
    ) {
      const label = trimmed.slice(1).trim().toLowerCase();
      if (!label || !flow.includes(label)) {
        continue;
      }
    }
    kept.push(line);
  }

  return kept.join("\n");
}

export function sanitizeLocalPromptStatus(status: Status, recentEvents?: string): Status {
  const shouldDropTarget = (target: Status["interactionTarget"]): boolean => {
    if (!target) {
      return false;
    }
    const type = target.hotspotType ?? target.kind;
    const label = target.label?.toLowerCase() ?? "";
    const flow = status.flowNextGoal.toLowerCase();
    if (
      target.token === "D" &&
      isRecentlyUsedReverseTransition(target.label ?? "", status, recentEvents)
    ) {
      return true;
    }
    if (target.token !== "D" && target.token === "N" && hasRepeatedDialogueAtCurrentCoords(status, recentEvents)) {
      return true;
    }
    return (
      target.token !== "D" &&
      (type === "utility" || type === "sign" || target.kind === "bg_event") &&
      (!label || !flow.includes(label))
    );
  };

  if (
    !shouldDropTarget(status.interactionTarget) &&
    !shouldDropTarget(status.localFocus?.target) &&
    !shouldDropTarget(status.currentHotspot)
  ) {
    return status;
  }

  const sanitized: Status = { ...status };
  if (shouldDropTarget(sanitized.interactionTarget)) {
    delete sanitized.interactionTarget;
    delete sanitized.interactionTile;
  }
  if (sanitized.localFocus && shouldDropTarget(sanitized.localFocus.target)) {
    delete sanitized.localFocus;
  }
  if (sanitized.currentHotspot && shouldDropTarget(sanitized.currentHotspot)) {
    delete sanitized.currentHotspot;
  }
  return sanitized;
}

function actionFailureKey(action: LocalPlayerAction): string | null {
  if (action.action === "move") {
    const direction = normalizeDirection(action.parameters?.direction);
    return direction ? `move:${direction}` : null;
  }
  if (action.action === "press") {
    const button = normalizeButton(action.parameters?.button);
    return button ? `press:${button.toLowerCase()}` : null;
  }
  return null;
}

export function isLocalActionRejectedByRecentFailure(params: {
  action: LocalPlayerAction;
  status: Status;
  recentEvents: string;
}): boolean {
  return Boolean(localActionRejectionReason(params));
}

function localActionRejectionReason(params: {
  action: LocalPlayerAction;
  status: Status;
  recentEvents: string;
}): string | null {
  const liveStateRejection = localActionLiveStateRejectionReason(params.action, params.status, params.recentEvents);
  if (liveStateRejection) {
    return liveStateRejection;
  }

  const key = actionFailureKey(params.action);
  if (!key) {
    return null;
  }

  let parsed: { events?: unknown };
  try {
    parsed = JSON.parse(params.recentEvents) as { events?: unknown };
  } catch {
    return null;
  }
  if (!Array.isArray(parsed.events)) {
    return null;
  }

  for (const event of parsed.events.slice().reverse()) {
    if (!event || typeof event !== "object" || Array.isArray(event)) {
      continue;
    }
    const record = event as Record<string, unknown>;
    const eventAction = typeof record.action === "string" ? record.action : "";
    if (!eventAction.startsWith(key)) {
      continue;
    }
    const changed = record.changed;
    const reason = typeof record.reason === "string" ? record.reason : "";
    const coords = Array.isArray(record.coords) ? record.coords : null;
    const sameCoords =
      !params.status.coords ||
      !coords ||
      (coords[0] === params.status.coords[0] && coords[1] === params.status.coords[1]);
    if (sameCoords && (reason === "blocked" || (changed === false && (reason === "menu" || reason === "no_change")))) {
      return `${key} already failed from this live state with reason ${reason}.`;
    }
  }
  return null;
}

function localActionLiveStateRejectionReason(action: LocalPlayerAction, status: Status, recentEvents?: string): string | null {
  if (action.action === "press" && normalizeButton(action.parameters?.button) === "A") {
    const target = status.currentHotspot ?? status.interactionSetup?.hotspot;
    if (target?.token === "D" || target?.hotspotType === "warp" || target?.hotspotType === "door") {
      return "Warp and door hotspots are entered by movement, not by pressing A.";
    }
    if (
      status.interactionTarget?.token === "N" &&
      hasRepeatedDialogueAtCurrentCoords(status, recentEvents)
    ) {
      return "That NPC dialogue has repeated from this live state; leave the interaction lane and choose movement toward a route transition.";
    }
  }
  if (action.action !== "move") {
    return null;
  }
  const direction = normalizeDirection(action.parameters?.direction);
  const facing = normalizeDirection(status.facing);
  const target = status.interactionTarget;
  const targetType = target?.hotspotType ?? target?.kind;
  const targetToken = target?.token;
  if (
    direction &&
    facing &&
    direction === facing &&
    target &&
    targetToken !== "D" &&
    (targetType === "utility" || targetType === "sign" || target.kind === "bg_event")
  ) {
    return "That move would walk into the current non-route interaction target instead of routing through a floor tile.";
  }
  return null;
}

async function completeLocalOllamaAction(params: {
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
}): Promise<{ action: LocalPlayerAction | null; text: string }> {
  const model = normalizeAgentModel(params.input.playerModel).slice("ollama/".length);
  let rejectedActionNote: string | undefined;
  const rejectedActions: string[] = [];
  let lastText = "";

  for (let attempt = 0; attempt < 6; attempt += 1) {
    const response = await fetch(`${getDefaultOllamaApiBaseUrl()}/chat/completions`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        Authorization: `Bearer ${process.env.OLLAMA_API_KEY?.trim() || "dummy"}`,
      },
      body: JSON.stringify(buildLocalOllamaActionRequest({
        model,
        input: params.input,
        status: params.status,
        observation: params.observation,
        mapInfo: params.mapInfo,
        recentEvents: params.recentEvents,
        rejectedActionNote,
      })),
    });
    const payload = await response.json() as {
      choices?: Array<{ text?: unknown; message?: { content?: unknown } }>;
      error?: { message?: unknown };
    };
    if (!response.ok) {
      throw new Error(typeof payload.error?.message === "string" ? payload.error.message : `Ollama completion failed: ${response.status}`);
    }
    const choice = payload.choices?.[0];
    const text = typeof choice?.message?.content === "string"
      ? choice.message.content
      : typeof choice?.text === "string"
        ? choice.text
        : "";
    lastText = text;
    const action = parseLocalPlayerAction(text);
    if (!action) {
      return { action: null, text };
    }
    const rejectionReason = localActionRejectionReason({ action, status: params.status, recentEvents: params.recentEvents });
    if (!rejectionReason) {
      return { action, text };
    }
    rejectedActions.push(`${text.slice(0, 140)} -> ${rejectionReason}`);
    rejectedActionNote = `${rejectedActions.join(" | ")}. Choose a different concrete gameplay action.`;
  }

  return { action: null, text: lastText };
}

function buildResponsesActionSchema(params: Parameters<typeof buildLocalOllamaActionRequest>[0]): {
  name: string;
  strict: boolean;
  schema: Record<string, unknown>;
} {
  const localRequest = buildLocalOllamaActionRequest(params);
  const schema = structuredClone(localRequest.response_format.json_schema.schema) as {
    properties: {
      parameters: {
        properties: Record<string, Record<string, unknown>>;
        required?: string[];
      };
    };
  };
  const parameterProperties = schema.properties.parameters.properties;
  for (const property of Object.values(parameterProperties)) {
    if (Array.isArray(property.type)) {
      if (!property.type.includes("null")) {
        property.type = [...property.type, "null"];
      }
    } else if (typeof property.type === "string") {
      property.type = [property.type, "null"];
    }
    if (Array.isArray(property.enum) && !property.enum.includes(null)) {
      property.enum = [...property.enum, null];
    }
  }
  schema.properties.parameters.required = Object.keys(parameterProperties);
  return {
    name: localRequest.response_format.json_schema.name,
    strict: localRequest.response_format.json_schema.strict,
    schema,
  };
}

export function buildOpenAIDirectActionRequest(params: Parameters<typeof buildLocalOllamaActionRequest>[0]): unknown {
  const localRequest = buildLocalOllamaActionRequest(params);
  const actionSchema = buildResponsesActionSchema(params);
  return {
    model: params.model,
    input: localRequest.messages,
    max_output_tokens: localRequest.max_tokens,
    text: {
      format: {
        type: "json_schema",
        name: actionSchema.name,
        strict: actionSchema.strict,
        schema: actionSchema.schema,
      },
    },
  };
}

export function buildAzureOpenAIActionRequest(params: Parameters<typeof buildLocalOllamaActionRequest>[0]): unknown {
  const localRequest = buildLocalOllamaActionRequest(params);
  const actionSchema = buildResponsesActionSchema(params);
  return {
    model: params.model,
    input: localRequest.messages,
    max_output_tokens: localRequest.max_tokens,
    text: {
      format: {
        type: "json_schema",
        name: actionSchema.name,
        strict: actionSchema.strict,
        schema: actionSchema.schema,
      },
    },
  };
}

export function buildAnthropicActionRequest(params: Parameters<typeof buildLocalOllamaActionRequest>[0]): unknown {
  const localRequest = buildLocalOllamaActionRequest(params);
  return {
    model: params.model,
    max_tokens: localRequest.max_tokens,
    temperature: localRequest.temperature,
    system: localRequest.messages[0].content,
    tools: [
      {
        name: localRequest.response_format.json_schema.name,
        description: "Return the next single concrete Pokemon Crystal gameplay action.",
        input_schema: localRequest.response_format.json_schema.schema,
      },
    ],
    tool_choice: {
      type: "tool",
      name: localRequest.response_format.json_schema.name,
    },
    messages: [
      {
        role: "user",
        content: localRequest.messages[1].content,
      },
    ],
  };
}

export function buildGoogleActionRequest(params: Parameters<typeof buildLocalOllamaActionRequest>[0]): unknown {
  const localRequest = buildLocalOllamaActionRequest(params);
  return {
    contents: [
      {
        role: "user",
        parts: [
          {
            text: `${localRequest.messages[0].content}\n\n${localRequest.messages[1].content}`,
          },
        ],
      },
    ],
    generationConfig: {
      temperature: localRequest.temperature,
      maxOutputTokens: localRequest.max_tokens,
      responseMimeType: "application/json",
      responseJsonSchema: localRequest.response_format.json_schema.schema,
    },
  };
}

function extractAzureOpenAIResponseText(payload: unknown): string {
  const texts: string[] = [];
  const visit = (value: unknown): void => {
    if (!value || typeof value !== "object") {
      return;
    }
    if (Array.isArray(value)) {
      for (const item of value) {
        visit(item);
      }
      return;
    }
    const record = value as Record<string, unknown>;
    if (typeof record.text === "string") {
      texts.push(record.text);
    }
    for (const child of Object.values(record)) {
      visit(child);
    }
  };
  visit(payload);
  return texts[0] ?? "";
}

function extractAnthropicActionText(payload: unknown): string {
  if (!payload || typeof payload !== "object") {
    return "";
  }
  const content = (payload as { content?: unknown }).content;
  if (!Array.isArray(content)) {
    return "";
  }
  const toolUse = content.find((item): item is { type: string; input?: unknown } =>
    Boolean(item && typeof item === "object" && (item as { type?: unknown }).type === "tool_use"),
  );
  return toolUse?.input ? JSON.stringify(toolUse.input) : "";
}

function extractGoogleResponseText(payload: unknown): string {
  if (!payload || typeof payload !== "object") {
    return "";
  }
  const candidates = (payload as { candidates?: unknown }).candidates;
  if (!Array.isArray(candidates)) {
    return "";
  }
  const texts: string[] = [];
  for (const candidate of candidates) {
    const parts = (candidate as { content?: { parts?: unknown } })?.content?.parts;
    if (!Array.isArray(parts)) {
      continue;
    }
    for (const part of parts) {
      const text = (part as { text?: unknown })?.text;
      if (typeof text === "string") {
        texts.push(text);
      }
    }
  }
  return texts.join("");
}

async function completeOpenAIDirectAction(params: {
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
}): Promise<{ action: LocalPlayerAction | null; text: string }> {
  const model = normalizeAgentModel(params.input.playerModel).slice("openai-direct/".length);
  let rejectedActionNote: string | undefined;
  const rejectedActions: string[] = [];
  let lastText = "";

  for (let attempt = 0; attempt < 6; attempt += 1) {
    const response = await fetch(`${getDefaultOpenAIBaseUrl()}/responses`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        Authorization: `Bearer ${getDefaultOpenAIApiKey()}`,
      },
      body: JSON.stringify(buildOpenAIDirectActionRequest({
        model,
        input: params.input,
        status: params.status,
        observation: params.observation,
        mapInfo: params.mapInfo,
        recentEvents: params.recentEvents,
        rejectedActionNote,
      })),
    });
    const payload = await response.json() as { error?: { message?: unknown } };
    if (!response.ok) {
      throw new Error(typeof payload.error?.message === "string" ? payload.error.message : `OpenAI response failed: ${response.status}`);
    }
    const text = extractAzureOpenAIResponseText(payload);
    lastText = text;
    const action = parseLocalPlayerAction(text);
    if (!action) {
      return { action: null, text };
    }
    const rejectionReason = localActionRejectionReason({ action, status: params.status, recentEvents: params.recentEvents });
    if (!rejectionReason) {
      return { action, text };
    }
    rejectedActions.push(`${text.slice(0, 140)} -> ${rejectionReason}`);
    rejectedActionNote = `${rejectedActions.join(" | ")}. Choose a different concrete gameplay action.`;
  }

  return { action: null, text: lastText };
}

async function completeAzureOpenAIAction(params: {
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
}): Promise<{ action: LocalPlayerAction | null; text: string }> {
  const model = normalizeAgentModel(params.input.playerModel).slice("azure-openai/".length);
  let rejectedActionNote: string | undefined;
  const rejectedActions: string[] = [];
  let lastText = "";

  for (let attempt = 0; attempt < 6; attempt += 1) {
    const response = await fetch(
      `${getDefaultAzureOpenAIEndpoint()}/openai/responses?api-version=${encodeURIComponent(getDefaultAzureOpenAIApiVersion())}`,
      {
        method: "POST",
        headers: {
          "content-type": "application/json",
          "api-key": getDefaultAzureOpenAIApiKey(),
        },
        body: JSON.stringify(buildAzureOpenAIActionRequest({
          model,
          input: params.input,
          status: params.status,
          observation: params.observation,
          mapInfo: params.mapInfo,
          recentEvents: params.recentEvents,
          rejectedActionNote,
        })),
      },
    );
    const payload = await response.json() as { error?: { message?: unknown } };
    if (!response.ok) {
      throw new Error(typeof payload.error?.message === "string" ? payload.error.message : `Azure OpenAI response failed: ${response.status}`);
    }
    const text = extractAzureOpenAIResponseText(payload);
    lastText = text;
    const action = parseLocalPlayerAction(text);
    if (!action) {
      return { action: null, text };
    }
    const rejectionReason = localActionRejectionReason({ action, status: params.status, recentEvents: params.recentEvents });
    if (!rejectionReason) {
      return { action, text };
    }
    rejectedActions.push(`${text.slice(0, 140)} -> ${rejectionReason}`);
    rejectedActionNote = `${rejectedActions.join(" | ")}. Choose a different concrete gameplay action.`;
  }

  return { action: null, text: lastText };
}

async function completeAnthropicAction(params: {
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
}): Promise<{ action: LocalPlayerAction | null; text: string }> {
  const model = normalizeAgentModel(params.input.playerModel).slice("anthropic/".length);
  let rejectedActionNote: string | undefined;
  const rejectedActions: string[] = [];
  let lastText = "";

  for (let attempt = 0; attempt < 6; attempt += 1) {
    const response = await fetch(`${getDefaultAnthropicBaseUrl()}/v1/messages`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-api-key": getDefaultAnthropicApiKey(),
        "anthropic-version": getDefaultAnthropicApiVersion(),
      },
      body: JSON.stringify(buildAnthropicActionRequest({
        model,
        input: params.input,
        status: params.status,
        observation: params.observation,
        mapInfo: params.mapInfo,
        recentEvents: params.recentEvents,
        rejectedActionNote,
      })),
    });
    const payload = await response.json() as { error?: { message?: unknown } };
    if (!response.ok) {
      throw new Error(typeof payload.error?.message === "string" ? payload.error.message : `Anthropic message failed: ${response.status}`);
    }
    const text = extractAnthropicActionText(payload);
    lastText = text;
    const action = parseLocalPlayerAction(text);
    if (!action) {
      return { action: null, text };
    }
    const rejectionReason = localActionRejectionReason({ action, status: params.status, recentEvents: params.recentEvents });
    if (!rejectionReason) {
      return { action, text };
    }
    rejectedActions.push(`${text.slice(0, 140)} -> ${rejectionReason}`);
    rejectedActionNote = `${rejectedActions.join(" | ")}. Choose a different concrete gameplay action.`;
  }

  return { action: null, text: lastText };
}

async function completeGoogleAction(params: {
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
}): Promise<{ action: LocalPlayerAction | null; text: string }> {
  const normalized = normalizeAgentModel(params.input.playerModel);
  const model = normalized.startsWith("gemini/")
    ? normalized.slice("gemini/".length)
    : normalized.slice("google/".length);
  let rejectedActionNote: string | undefined;
  const rejectedActions: string[] = [];
  let lastText = "";

  for (let attempt = 0; attempt < 6; attempt += 1) {
    const response = await fetch(`${getDefaultGoogleApiBaseUrl()}/models/${encodeURIComponent(model)}:generateContent`, {
      method: "POST",
      headers: {
        "content-type": "application/json",
        "x-goog-api-key": getDefaultGoogleApiKey(),
      },
      body: JSON.stringify(buildGoogleActionRequest({
        model,
        input: params.input,
        status: params.status,
        observation: params.observation,
        mapInfo: params.mapInfo,
        recentEvents: params.recentEvents,
        rejectedActionNote,
      })),
    });
    const payload = await response.json() as { error?: { message?: unknown } };
    if (!response.ok) {
      throw new Error(typeof payload.error?.message === "string" ? payload.error.message : `Google Gemini generation failed: ${response.status}`);
    }
    const text = extractGoogleResponseText(payload);
    lastText = text;
    const action = parseLocalPlayerAction(text);
    if (!action) {
      return { action: null, text };
    }
    const rejectionReason = localActionRejectionReason({ action, status: params.status, recentEvents: params.recentEvents });
    if (!rejectionReason) {
      return { action, text };
    }
    rejectedActions.push(`${text.slice(0, 140)} -> ${rejectionReason}`);
    rejectedActionNote = `${rejectedActions.join(" | ")}. Choose a different concrete gameplay action.`;
  }

  return { action: null, text: lastText };
}

async function completeDirectPlayerAction(params: {
  input: RunnerInput;
  status: Status;
  observation: string;
  mapInfo: string;
  recentEvents: string;
}): Promise<{ action: LocalPlayerAction | null; text: string }> {
  const model = normalizeAgentModel(params.input.playerModel);
  if (model.startsWith("azure-openai/")) {
    return completeAzureOpenAIAction(params);
  }
  if (model.startsWith("openai-direct/")) {
    return completeOpenAIDirectAction(params);
  }
  if (model.startsWith("anthropic/")) {
    return completeAnthropicAction(params);
  }
  if (model.startsWith("google/") || model.startsWith("gemini/")) {
    return completeGoogleAction(params);
  }
  return completeLocalOllamaAction(params);
}

async function runLocalOllamaPlayerBatch(params: {
  input: RunnerInput;
  beforeStatus: Status;
  session: KrabbyClawSession;
}): Promise<string> {
  const maxActions = Math.max(1, Math.min(params.input.playerMaxSteps, 6));
  const summaries: string[] = [];
  for (let index = 0; index < maxActions; index += 1) {
    const status = await params.session.status();
    const observation = await params.session.observe();
    const mapInfo = await params.session.mapInfo();
    const recentEvents = await params.session.recentEvents();
    const { action, text } = await completeDirectPlayerAction({
      input: params.input,
      status,
      observation: observation.summaryText,
      mapInfo,
      recentEvents,
    });
    emitAgentStreamStatus(`direct player model text: ${text.trim().slice(0, 240)}`, "player");
    if (!action) {
      summaries.push(`No parseable action from direct player model: ${text.trim().slice(0, 160)}`);
      break;
    }
    const reason = action.reason || "Local model selected the next live gameplay action.";
    let result = "";
    if (action.action === "press") {
      const button = normalizeButton(action.parameters?.button);
      if (!button) {
        summaries.push(`Skipped invalid press button from direct player model: ${JSON.stringify(action.parameters)}`);
        break;
      }
      result = await params.session.press(button);
      summaries.push(`press:${button} - ${reason} => ${result.slice(0, 160)}`);
    } else if (action.action === "move") {
      const direction = normalizeDirection(action.parameters?.direction);
      if (!direction) {
        summaries.push(`Skipped invalid move direction from direct player model: ${JSON.stringify(action.parameters)}`);
        break;
      }
      const steps = Math.max(1, Math.min(Math.trunc(action.parameters?.steps ?? 1), 6));
      result = await params.session.move(direction, steps);
      summaries.push(`move:${direction}:${steps} - ${reason} => ${result.slice(0, 160)}`);
    } else if (action.action === "type_text") {
      const textInput = action.parameters?.text?.trim();
      if (!textInput) {
        summaries.push(`Skipped invalid type_text input from direct player model: ${JSON.stringify(action.parameters)}`);
        break;
      }
      result = await params.session.typeText(textInput.slice(0, 32), {
        clear: action.parameters?.clear,
        submit: action.parameters?.submit,
      });
      summaries.push(`type_text:${textInput.slice(0, 32)} - ${reason} => ${result.slice(0, 160)}`);
    }
    const afterStatus = await params.session.status();
    if (formatDirectBatchStatus(afterStatus) !== formatDirectBatchStatus(status)) {
      break;
    }
  }
  return summaries.join("\n");
}

async function runDirectPlayerBatch(params: {
  input: RunnerInput;
  beforeStatus: Status;
  session: KrabbyClawSession;
  sessionAuth: SessionAuth;
}): Promise<{ taskmaster: TaskmasterBatch; afterStatus: Status }> {
  const recentEventsBefore = await params.session.recentEvents();
  emitAgentStreamStatus("direct player gameplay batch", "runner");
  const playerModel = normalizeAgentModel(params.input.playerModel);
  const playerText = isDirectProviderModel(playerModel)
    ? await runLocalOllamaPlayerBatch({
        input: params.input,
        beforeStatus: params.beforeStatus,
        session: params.session,
      })
    : await (async () => {
        const playerAgent = await createPlayerAgent(params.session, params.input.playerModel);
        const stream = await consumeAgentStream(
          playerAgent,
          [
            `Requested immediate goal (verify against live flow): ${params.input.immediateGoal}`,
            buildPlayerDelegationPrompt(params.input, params.beforeStatus, recentEventsBefore),
            "Use the MCP tools to make real gameplay progress. Return the exact actions taken and resulting state.",
          ].join("\n\n"),
          {
            maxSteps: params.input.playerMaxSteps,
            onChunk: (chunk: unknown) => emitAgentStreamChunk(chunk, "player"),
          },
        );
        return (await stream.text?.catch(() => "")) ?? "";
      })();
  const afterStatus = await params.session.status();
  const recentEvents = await params.session.recentEvents();
  const observation = await params.session.observe();

  if (!hasDirectPlayerBatchProgress({
    beforeStatus: params.beforeStatus,
    afterStatus,
    recentEventsBefore,
    recentEventsAfter: recentEvents,
    playerText,
  })) {
    const beforeTotal = parseRecentEventsTotal(recentEventsBefore);
    const afterTotal = parseRecentEventsTotal(recentEvents);
    emitAgentStreamStatus(
      [
        "direct player batch produced no real gameplay progress; non-agentic recovery disabled",
        `before=${formatDirectBatchStatus(params.beforeStatus)}`,
        `after=${formatDirectBatchStatus(afterStatus)}`,
        `recent_total=${beforeTotal ?? "unknown"}->${afterTotal ?? "unknown"}`,
        `player_text_chars=${playerText.trim().length}`,
      ].join(" | "),
      "runner",
    );
    return {
      taskmaster: buildNoActionAgenticBatch({
        input: params.input,
        beforeStatus: params.beforeStatus,
        afterStatus,
        recentEvents,
        reason: "Direct player agent produced no real gameplay progress.",
        detail: `player_text_chars=${playerText.trim().length}`,
      }),
      afterStatus,
    };
  }

  return {
    taskmaster: taskmasterBatchSchema.parse({
      summary: playerText.trim() || recentEvents.slice(0, 240) || observation.summaryText.slice(0, 240),
      immediateGoalStatus: afterStatus.partyCount > params.beforeStatus.partyCount ? "done" : "in_progress",
      nextImmediateGoal: afterStatus.flowNextGoal || params.input.immediateGoal,
      shouldContinue: true,
      evidence: [playerText.trim(), recentEvents.slice(0, 500), observation.summaryText.slice(0, 500)]
        .filter(Boolean)
        .slice(0, 3),
    }),
    afterStatus,
  };
}

export async function runTaskmasterBatch(params: {
  input: RunnerInput;
  beforeStatus: Status;
  session: KrabbyClawSession;
  sessionAuth: SessionAuth;
  memory: Memory;
}): Promise<{ taskmaster: TaskmasterBatch; afterStatus: Status }> {
  const recentEventsBefore = await params.session.recentEvents();

  if (isCodexModel(params.input.taskmasterModel) || isCodexModel(params.input.playerModel)) {
    return runCodexTaskmasterBatch({
      input: params.input,
      beforeStatus: params.beforeStatus,
      recentEventsBefore,
      session: params.session,
      sessionAuth: params.sessionAuth,
    });
  }

  if (shouldUseDirectPlayerBatch(params.input)) {
    return runDirectPlayerBatch({
      input: params.input,
      beforeStatus: params.beforeStatus,
      session: params.session,
      sessionAuth: params.sessionAuth,
    });
  }

  const playerAgent = await createPlayerAgent(params.session, params.input.playerModel);
  const taskmasterAgent = createTaskmasterAgent(
    playerAgent,
    params.input.taskmasterModel,
    params.memory,
  );
  const memoryScope = createTaskmasterMemoryScope(params.sessionAuth);
  let delegatedToPlayer = false;
  let usedDirectPlayerFallback = false;
  const delegationEvidence: string[] = [];

  emitAgentStreamStatus("taskmaster planning gameplay batch", "taskmaster");
  await consumeAgentStream(
    taskmasterAgent,
    buildTaskmasterPrompt(params.input, params.beforeStatus, recentEventsBefore),
    {
      maxSteps: params.input.supervisorMaxSteps,
      memory: memoryScope,
      onChunk: (chunk: unknown) => emitAgentStreamChunk(chunk, "taskmaster"),
      onIterationComplete: (context: { iteration: number }) => {
        return shouldContinueTaskmasterIteration({
          delegatedToPlayer,
          iteration: context.iteration,
          supervisorMaxSteps: params.input.supervisorMaxSteps,
        });
      },
      delegation: {
        onDelegationStart: (context: { primitiveId: string; prompt: string }) => {
          if (isPlayerDelegationPrimitive(context.primitiveId)) {
            delegatedToPlayer = true;
            emitAgentStreamStatus("player delegation started", "taskmaster");
            return {
              proceed: true,
              modifiedMaxSteps: params.input.playerMaxSteps,
              modifiedPrompt: [
                context.prompt,
                buildPlayerDelegationPrompt(params.input, params.beforeStatus, recentEventsBefore),
                "Return the exact actions taken, the resulting state, and whether you are blocked.",
              ].join("\n\n"),
            };
          }

          return { proceed: true };
        },
        onDelegationComplete: (context: { primitiveId: string; success: boolean; error?: Error; result: { text: string } }) => {
          if (isPlayerDelegationPrimitive(context.primitiveId)) {
            if (!context.success) {
              throw context.error ?? new Error("Player delegation failed.");
            }
            emitAgentStreamStatus("player delegation completed", "taskmaster");
            delegationEvidence.push(context.result.text);
          }
        },
        messageFilter: ({ messages }: { messages: unknown[] }) => messages.slice(-12),
      },
    });

  if (!delegatedToPlayer) {
    usedDirectPlayerFallback = true;
    emitAgentStreamStatus("taskmaster did not delegate; running direct player fallback", "runner");
    await consumeAgentStream(
      playerAgent,
      [
        buildPlayerDelegationPrompt(params.input, params.beforeStatus, recentEventsBefore),
        "The taskmaster did not delegate successfully. You are now directly responsible for this bounded gameplay batch.",
        "Use the live MCP tools now and take concrete progress actions.",
      ].join("\n\n"),
      {
        maxSteps: params.input.playerMaxSteps,
        onChunk: (chunk: unknown) => emitAgentStreamChunk(chunk, "player"),
      },
    );
    delegationEvidence.push("Direct player fallback ran because the taskmaster did not delegate gameplay.");
  }

  const afterStatus = await params.session.status();
  const recentEvents = await params.session.recentEvents();
  const observation = await params.session.observe();
  const stateChanged =
    JSON.stringify({
      map: params.beforeStatus.map,
      coords: params.beforeStatus.coords,
      facing: params.beforeStatus.facing,
      flowNextGoal: params.beforeStatus.flowNextGoal,
      badges: params.beforeStatus.badges,
      partyCount: params.beforeStatus.partyCount,
    }) !==
    JSON.stringify({
      map: afterStatus.map,
      coords: afterStatus.coords,
      facing: afterStatus.facing,
      flowNextGoal: afterStatus.flowNextGoal,
      badges: afterStatus.badges,
      partyCount: afterStatus.partyCount,
    });

  if (!stateChanged && delegationEvidence.length === 0) {
    throw new Error("Taskmaster batch produced no state change and no delegation evidence.");
  }

  if (usedDirectPlayerFallback) {
    return {
      taskmaster: taskmasterBatchSchema.parse({
        summary: `Direct player fallback ran against live MCP. Current state: ${afterStatus.map} flow=${afterStatus.flowNextGoal}.`,
        immediateGoalStatus: afterStatus.partyCount > params.beforeStatus.partyCount ? "done" : "in_progress",
        nextImmediateGoal: afterStatus.flowNextGoal || params.input.immediateGoal,
        shouldContinue: true,
        evidence: [
          ...delegationEvidence,
          recentEvents.split("\n").find(Boolean) ?? observation.rawTexts.join("\n").slice(0, 240),
        ].filter(Boolean).slice(0, 5),
      }),
      afterStatus,
    };
  }

  const summary = await consumeAgentStream(
    taskmasterAgent,
    buildTaskmasterSummaryPrompt(
      params.input,
      params.beforeStatus,
      afterStatus,
      delegationEvidence,
      recentEvents,
      observation.rawTexts.join("\n\n"),
    ),
    {
      structuredOutput: {
        schema: taskmasterBatchSchema,
      },
      maxSteps: 1,
      memory: memoryScope,
      onChunk: (chunk: unknown) => emitAgentStreamChunk(chunk, "summary"),
    },
  );

  return {
    taskmaster: taskmasterBatchSchema.parse(await summary.object),
    afterStatus,
  };
}
