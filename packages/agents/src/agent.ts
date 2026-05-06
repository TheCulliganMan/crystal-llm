import { createPokemonCrystalMastra } from "./workflow.js";
import { runnerOutputSchema, type CliRunOptions, type RunnerOutput } from "./types.js";
import { readSessionAuthCache } from "./session.js";
import { emitAgentStreamStatus } from "./stream-events.js";

type AgentRunResult = {
  step: number;
  finished: boolean;
  reason: string;
};

const DEFAULT_WORKFLOW_RESUME_TIMEOUT_MS = 10 * 60_000;
const MAX_AGENT_GOAL_CHARS = 1_600;

const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value));

export const derivePlayerMaxSteps = (graphCycleSteps: number): number =>
  clamp(graphCycleSteps, 1, 12);

export const shouldRestartWorkflowRun = (error: unknown): boolean =>
  error instanceof Error && /not suspended/i.test(error.message);

export function summarizeWorkflowFailure(result: unknown): string {
  if (!result || typeof result !== "object") {
    return "unknown workflow failure";
  }
  const error = (result as { error?: unknown }).error;
  if (error instanceof Error) {
    return error.message;
  }
  if (error && typeof error === "object") {
    const message = (error as { message?: unknown }).message;
    const name = (error as { name?: unknown }).name;
    if (typeof message === "string" && typeof name === "string") {
      return `${name}: ${message}`;
    }
    if (typeof message === "string") {
      return message;
    }
  }
  const steps = (result as { steps?: unknown }).steps;
  if (steps && typeof steps === "object") {
    for (const step of Object.values(steps as Record<string, unknown>)) {
      if (!step || typeof step !== "object") {
        continue;
      }
      const stepError = (step as { error?: unknown }).error;
      if (stepError instanceof Error) {
        return stepError.message;
      }
      if (stepError && typeof stepError === "object" && typeof (stepError as { message?: unknown }).message === "string") {
        return (stepError as { message: string }).message;
      }
    }
  }
  return "unknown workflow failure";
}

const sleep = (ms: number): Promise<void> =>
  new Promise(resolve => {
    setTimeout(resolve, ms);
  });

const summarizeThrowable = (error: unknown): string => {
  if (error instanceof Error) {
    return error.stack || `${error.name}: ${error.message}`;
  }
  return String(error);
};

export const compactAgentGoal = (goal: string): string => {
  const lines = goal
    .split(/\r?\n/g)
    .map(line => line.trim())
    .filter(Boolean);
  const mustChoose = lines.some(line => /YOU MUST MAKE A CHOICE/i.test(line));
  const keep: string[] = [];
  for (const line of lines) {
    const normalizedLine = /^\d+\.\s/.test(line)
      ? line.replace(/\s+->\s+.*?(;\s+|$)/, "; ")
      : line;
    if (
      keep.length === 0 ||
      /^(Professor Culligan's (Advice|Intervention)|Previous agentic batch failed|After resuming|Treat Professor|The manual inputs|YOU MUST MAKE A CHOICE)/i.test(normalizedLine) ||
      /^Professor Culligan intervened/i.test(normalizedLine) ||
      /^\d+\.\s/.test(normalizedLine)
    ) {
      keep.push(normalizedLine.slice(0, 220));
    }
  }
  const compact = keep.join("\n").slice(0, MAX_AGENT_GOAL_CHARS).trim();
  return [
    compact || "Continue playing toward the current objective.",
    mustChoose && !/YOU MUST MAKE A CHOICE/i.test(compact)
      ? "YOU MUST MAKE A CHOICE using live MCP evidence."
      : undefined,
  ].filter(Boolean).join("\n");
};

const appendMustChooseGoal = (goal: string, reason: string): string =>
  compactAgentGoal([
    compactAgentGoal(goal),
    "",
    `Previous agentic batch failed before completing: ${reason}`,
    "YOU MUST MAKE A CHOICE using live MCP evidence.",
  ].join("\n"));

function shouldKeepPlaying(checkpoint: RunnerOutput["checkpoint"], overallGoal: string): boolean {
  if (checkpoint.taskmaster.shouldContinue) {
    return true;
  }

  if (checkpoint.afterStatus.partyCount === 0) {
    return true;
  }

  const goal = overallGoal.toLowerCase();
  if (/mt\.?\s*silver|beat pokemon crystal|main-story progress/.test(goal)) {
    return true;
  }

  return checkpoint.taskmaster.immediateGoalStatus !== "done";
}

async function withResumeTimeout<T>(promise: Promise<T>, timeoutMs = DEFAULT_WORKFLOW_RESUME_TIMEOUT_MS): Promise<T> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([
      promise,
      new Promise<T>((_, reject) => {
        timer = setTimeout(() => {
          reject(new Error("Timed out waiting for workflow resume."));
        }, timeoutMs);
      }),
    ]);
  } finally {
    if (timer) {
      clearTimeout(timer);
    }
  }
}

export async function runPokemonAgent(options: CliRunOptions): Promise<AgentRunResult> {
  if (options.ollamaBaseUrl) {
    process.env.OLLAMA_BASE_URL = options.ollamaBaseUrl;
  }
  if (options.openaiBaseUrl) {
    process.env.OPENAI_BASE_URL = options.openaiBaseUrl;
  }
  if (options.openaiApiKey) {
    process.env.OPENAI_API_KEY = options.openaiApiKey;
  }
  if (options.anthropicBaseUrl) {
    process.env.ANTHROPIC_BASE_URL = options.anthropicBaseUrl;
  }
  if (options.anthropicApiKey) {
    process.env.ANTHROPIC_API_KEY = options.anthropicApiKey;
  }
  if (options.googleBaseUrl) {
    process.env.GOOGLE_GENERATIVE_AI_BASE_URL = options.googleBaseUrl;
  }
  if (options.googleApiKey) {
    process.env.GOOGLE_GENERATIVE_AI_API_KEY = options.googleApiKey;
  }

  const cachedSession = options.command === "resume"
    ? await readSessionAuthCache(options.sessionId)
    : null;
  const sessionBaseUrl = options.baseUrl.trim() || cachedSession?.baseUrl || options.mcpBaseUrl;
  const overallGoal = compactAgentGoal(options.goal);
  const mastra = createPokemonCrystalMastra({});
  const workflow = mastra.getWorkflow("pokemonCrystalWorkflow");
  const sharedInput = {
    session: {
      baseUrl: sessionBaseUrl,
      mcpUrl: options.mcpUrl,
      sessionId: options.sessionId,
      identityName: options.identityName,
    },
    overallGoal,
    immediateGoal: overallGoal,
    taskmasterModel: options.model,
    playerModel: options.model,
    supervisorMaxSteps: clamp(options.graphCycleSteps, 1, 20),
    playerMaxSteps: derivePlayerMaxSteps(options.graphCycleSteps),
    autoSuspend: true,
    includeObservationCheckpoint: true,
  };

  let currentGoal = overallGoal;
  let batches = 0;
  let lastReason = "No gameplay batches completed.";
  let finished = false;

  const createRun = async (suffix: string) =>
    workflow.createRun({
      runId: `pokemon-crystal:${options.sessionId}:${suffix}`,
      resourceId: options.sessionId,
    });

  let run =
    options.command === "resume"
      ? await workflow.createRun({
          runId: `pokemon-crystal:${options.sessionId}`,
          resourceId: options.sessionId,
        })
      : await createRun("run-0");

  let shouldStartFresh = options.command === "run";

  while (batches < options.maxSteps) {
    let result;
    emitAgentStreamStatus(`batch ${batches + 1} starting`, "runner");

    try {
      if (shouldStartFresh) {
        result = await run.start({
          inputData: {
            ...sharedInput,
            immediateGoal: currentGoal,
          },
        });
      } else {
        result = await withResumeTimeout(
          run.resume({
            resumeData: {
              immediateGoal: currentGoal,
              baseUrl: sessionBaseUrl,
            },
          }),
        );
      }
    } catch (error) {
      if (shouldRestartWorkflowRun(error)) {
        run = await createRun(`restart-${batches}`);
        shouldStartFresh = true;
        continue;
      }
      const failure = summarizeThrowable(error);
      emitAgentStreamStatus(`batch ${batches + 1} threw before checkpoint: ${failure}`, "runner");
      lastReason = `Agentic batch failed before checkpoint. No non-agentic gameplay action was executed.`;
      currentGoal = appendMustChooseGoal(currentGoal, failure.split(/\r?\n/g)[0] ?? "unknown error");
      run = await createRun(`error-${batches}`);
      shouldStartFresh = true;
      if (options.requestDelayMs > 0) {
        await sleep(options.requestDelayMs);
      }
      continue;
    }

    batches += 1;
    emitAgentStreamStatus(`batch ${batches} ${result.status}`, "runner");

    if (result.status === "suspended") {
      lastReason =
        result.suspendPayload?.checkpoint?.taskmaster?.summary ??
        "Workflow suspended for another gameplay batch.";
      currentGoal =
        result.suspendPayload?.checkpoint?.taskmaster?.nextImmediateGoal ?? currentGoal;
      shouldStartFresh = false;
      if (options.requestDelayMs > 0) {
        await sleep(options.requestDelayMs);
      }
      continue;
    }

    if (result.status === "success") {
      const output = runnerOutputSchema.parse(result.result);
      lastReason = output.checkpoint.taskmaster.summary;
      currentGoal = output.checkpoint.taskmaster.nextImmediateGoal || currentGoal;
      finished = !shouldKeepPlaying(output.checkpoint, overallGoal);
      if (finished) {
        break;
      }
      run = await createRun(`checkpoint-${batches}`);
      shouldStartFresh = true;
      if (options.requestDelayMs > 0) {
        await sleep(options.requestDelayMs);
      }
      continue;
    }

    const failure = summarizeWorkflowFailure(result);
    emitAgentStreamStatus(`batch ${batches} failed: ${failure}`, "runner");
    lastReason = `Agentic batch failed with status '${result.status}'. No non-agentic gameplay action was executed.`;
    currentGoal = appendMustChooseGoal(currentGoal, failure);
    run = await createRun(`failed-${batches}`);
    shouldStartFresh = true;
    if (options.requestDelayMs > 0) {
      await sleep(options.requestDelayMs);
    }
  }

  return {
    step: batches,
    finished,
    reason: lastReason,
  };
}
