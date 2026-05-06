import path from "node:path";
import { DEFAULT_SESSION_LOG_DIR } from "./session-log";
import type { CliCommand, CliOptions } from "./types";

const DEFAULT_BASE_URL = "";
const DEFAULT_SESSION_ID = "00000000-0000-4000-8000-000000000001";
const DEFAULT_PLAY_SESSION_ID = "cli-play";

const normalizeBaseUrl = (value: string): string => {
  const trimmed = value.trim().replace(/\/+$/, "");
  if (!trimmed) {
    return "";
  }
  if (trimmed.endsWith("/api/mcp/tools")) {
    return trimmed.slice(0, -"/api/mcp/tools".length);
  }
  return trimmed;
};

export const resolveToolsUrl = (baseUrl: string): string =>
  `${normalizeBaseUrl(baseUrl)}/api/mcp/tools`;

const resolveCommand = (candidate: string | undefined): CliCommand => {
  switch ((candidate ?? "").trim().toLowerCase()) {
    case "mcp":
    case "play":
    case "play-recorded":
    case "register":
    case "skill":
      return candidate!.trim().toLowerCase() as CliCommand;
    default:
      return "help";
  }
};

const defaultTrainingDir = (command: CliCommand, sessionId: string): string =>
  command === "play-recorded"
    ? path.join(process.cwd(), "packages", "cli", ".tmp-human-play", sessionId)
    : path.join(process.cwd(), ".pokecrystal-cli", "runs", sessionId, "training");

const parsePositiveInt = (value: string | undefined): number | undefined => {
  const parsed = value ? Number.parseInt(value, 10) : Number.NaN;
  return Number.isFinite(parsed) && parsed > 0 ? parsed : undefined;
};

export const parseArgs = (
  argv: string[],
  env: NodeJS.ProcessEnv = process.env
): CliOptions => {
  const [commandArg, ...rest] = argv;
  const command = resolveCommand(commandArg);
  const args = new Map<string, string>();
  const flags = new Set<string>();

  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index] ?? "";
    if (!token.startsWith("--")) {
      continue;
    }
    const key = token.slice(2);
    const next = rest[index + 1];
    if (!next || next.startsWith("--")) {
      flags.add(key);
      continue;
    }
    args.set(key, next);
    index += 1;
  }

  const baseUrl = normalizeBaseUrl(
    args.get("base-url") ??
      env.POKECRYSTAL_BASE_URL ??
      env.POKECRYSTAL_TOOLS_BASE_URL ??
      DEFAULT_BASE_URL
  );
  const transportCandidate = (args.get("transport") ?? env.POKECRYSTAL_CLI_TRANSPORT ?? "")
    .trim()
    .toLowerCase();
  const transport =
    transportCandidate === "http" || transportCandidate === "local"
      ? transportCandidate
      : baseUrl
        ? "http"
        : "local";
  const recordsTrainingByDefault = command === "play" || command === "play-recorded";
  const logsByDefault = command !== "help" && command !== "skill";
  const sessionLogDisabled =
    flags.has("no-session-log") || env.POKECRYSTAL_CLI_SESSION_LOG === "0";
  const explicitSessionId = args.get("session-id") ?? env.POKECRYSTAL_SESSION_ID;
  const sessionId =
    explicitSessionId ??
    (recordsTrainingByDefault ? DEFAULT_PLAY_SESSION_ID : DEFAULT_SESSION_ID);

  return {
    command,
    transport,
    baseUrl,
    toolsUrl: baseUrl ? resolveToolsUrl(baseUrl) : undefined,
    sessionId,
    sessionMode: recordsTrainingByDefault ? "interactive" : undefined,
    token: args.get("token") ?? env.POKECRYSTAL_IDENTITY_TOKEN ?? undefined,
    sessionSecret:
      args.get("session-secret") ?? env.POKECRYSTAL_SESSION_SECRET ?? undefined,
    agentId: args.get("agent-id") ?? undefined,
    identityName: args.get("identity-name") ?? undefined,
    printSkill: flags.has("print"),
    sessionLogEnabled: logsByDefault && !sessionLogDisabled,
    sessionLogDir:
      args.get("log-dir") ??
      env.POKECRYSTAL_CLI_LOG_DIR ??
      (logsByDefault && !sessionLogDisabled ? DEFAULT_SESSION_LOG_DIR : undefined),
    sessionLogFile: args.get("log-file") ?? env.POKECRYSTAL_CLI_LOG_FILE,
    recordTraining:
      recordsTrainingByDefault
        ? !(flags.has("no-record-training") || env.POKECRYSTAL_CLI_RECORD_TRAINING === "0")
        : flags.has("record-training") || env.POKECRYSTAL_CLI_RECORD_TRAINING === "1",
    trainingDir:
      args.get("training-dir") ??
      env.POKECRYSTAL_CLI_TRAINING_DIR ??
      (recordsTrainingByDefault ? defaultTrainingDir(command, sessionId) : undefined),
    agent: recordsTrainingByDefault
      ? flags.has("agent") || env.POKECRYSTAL_CLI_AGENT === "1"
      : false,
    agentCommand:
      args.get("agent-command") === "resume" || env.POKECRYSTAL_AGENT_COMMAND === "resume"
        ? "resume"
        : "run",
    agentModel: args.get("agent-model") ?? env.POKECRYSTAL_AGENT_MODEL,
    agentGoal: args.get("agent-goal") ?? env.POKECRYSTAL_AGENT_GOAL,
    agentMaxSteps: parsePositiveInt(args.get("agent-max-steps") ?? env.POKECRYSTAL_AGENT_MAX_STEPS),
    agentGraphCycleSteps: parsePositiveInt(
      args.get("agent-graph-cycle-steps") ?? env.POKECRYSTAL_AGENT_GRAPH_CYCLE_STEPS
    ),
    agentRequestDelayMs: parsePositiveInt(
      args.get("agent-request-delay-ms") ?? env.POKECRYSTAL_AGENT_REQUEST_DELAY_MS
    ),
    agentIdentityName: args.get("agent-identity-name") ?? env.POKECRYSTAL_AGENT_IDENTITY_NAME,
  };
};

export const skillPath = (...parts: string[]): string =>
  path.resolve(__dirname, "..", "skills", "pokecrystal-cli", ...parts);

export const helpText = (): string => `
pokecrystal-cli

Commands:
  pokecrystal-cli mcp [--transport local|http] [--base-url URL] [--session-id UUID] [--log-dir PATH] [--no-session-log]
  pokecrystal-cli play [--transport local|http] [--base-url URL] [--session-id UUID] [--training-dir PATH] [--log-dir PATH] [--no-record-training] [--agent] [--agent-model MODEL] [--agent-goal TEXT]
  pokecrystal-cli play-recorded [--transport local|http] [--base-url URL] [--session-id UUID] [--training-dir PATH] [--log-dir PATH] [--no-record-training] [--agent] [--agent-model MODEL] [--agent-goal TEXT]
  pokecrystal-cli register [--transport local|http] [--base-url URL] [--session-id UUID] [--agent-id ID] [--identity-name NAME] [--log-dir PATH]
  pokecrystal-cli skill [--print]

Options:
  --transport MODE        Transport mode. Default: local unless --base-url is set.
  --base-url URL          Base app URL for HTTP mode.
  --session-id UUID       Stable tools session id.
                          Default for \`play\` and \`play-recorded\`: ${DEFAULT_PLAY_SESSION_ID}
  --token TOKEN           Identity bearer token for protected routes.
  --session-secret VALUE  Session secret for protected routes.
  --log-dir PATH         Session JSONL log directory. Default: ${DEFAULT_SESSION_LOG_DIR}
  --log-file PATH        Exact session JSONL log path. Overrides --log-dir.
  --no-session-log       Disable session JSONL logging.
  --training-dir PATH     Training output directory.
                          Default for \`play\`: ./.pokecrystal-cli/runs/<session-id>/training
                          Default for \`play-recorded\`: ./packages/cli/.tmp-human-play/<session-id>
  --no-record-training    Disable default playtime training capture for \`play\` and \`play-recorded\`.
  --agent                 Link the Mastra gameplay agent to the play TUI's local MCP server.
  --agent-command MODE    Agent mode: run or resume. Default: run.
  --agent-model MODEL     Mastra model value passed to @pokecrystal/agents.
  --agent-goal TEXT       Agent goal prompt.
  --agent-max-steps N     Optional maximum supervised gameplay batches. Default: infinite.
  --agent-graph-cycle-steps N
                          Agent graph cycle step budget per batch.
  --agent-request-delay-ms N
                          Delay between agent batches.
  --agent-identity-name NAME
                          Identity name used by the linked agent.

Environment:
  POKECRYSTAL_BASE_URL, POKECRYSTAL_TOOLS_BASE_URL, POKECRYSTAL_SESSION_ID
  POKECRYSTAL_IDENTITY_TOKEN, POKECRYSTAL_SESSION_SECRET
  POKECRYSTAL_CLI_LOG_DIR, POKECRYSTAL_CLI_LOG_FILE, POKECRYSTAL_CLI_SESSION_LOG
  POKECRYSTAL_CLI_TRANSPORT, POKECRYSTAL_CLI_RECORD_TRAINING, POKECRYSTAL_CLI_TRAINING_DIR
  POKECRYSTAL_CLI_AGENT, POKECRYSTAL_AGENT_COMMAND, POKECRYSTAL_AGENT_MODEL
  POKECRYSTAL_AGENT_GOAL, POKECRYSTAL_AGENT_MAX_STEPS, POKECRYSTAL_AGENT_GRAPH_CYCLE_STEPS
  POKECRYSTAL_AGENT_REQUEST_DELAY_MS, POKECRYSTAL_AGENT_IDENTITY_NAME
  LLAMA_CPP_BASE_URL or OLLAMA_BASE_URL plus OLLAMA_API_KEY for ollama/* local agent models.

Play mode only sends Game Boy-faithful controller inputs:
  d-pad, A, B, Start, Select
`.trim();
