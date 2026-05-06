#!/usr/bin/env node

import { randomUUID } from "node:crypto";
import { runPokemonAgent } from "../agent.js";
import type { CliRunOptions } from "../types.js";
import { DEFAULT_AGENT_MODEL } from "../defaults.js";

type ArgMap = Map<string, string>;

const DEFAULT_GOAL = "Beat Mt. Silver";
const DEFAULT_MODEL = DEFAULT_AGENT_MODEL;
const DEFAULT_GRAPH_CYCLE_STEPS = 20;
const DEFAULT_REQUEST_DELAY_MS = 250;

const parseBooleanFlag = (args: Set<string>, key: string): boolean => args.has(key);
const parseNumber = (value: string | undefined, fallback: number): number => {
  const parsed = value ? Number.parseInt(value, 10) : Number.NaN;
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback;
};
const parseMaxSteps = (value: string | undefined): number => {
  const normalized = value?.trim().toLowerCase();
  if (
    !normalized ||
    normalized === "infinite" ||
    normalized === "infinity" ||
    normalized === "inf" ||
    normalized === "unlimited"
  ) {
    return Number.POSITIVE_INFINITY;
  }
  const parsed = Number.parseInt(normalized, 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : Number.POSITIVE_INFINITY;
};

const parseArgs = (argv: string[]): CliRunOptions => {
  const [commandArg, ...rest] = argv;
  const command = commandArg === "run" || commandArg === "resume" ? commandArg : "help";

  const args: ArgMap = new Map();
  const flags = new Set<string>();
  for (let index = 0; index < rest.length; index += 1) {
    const token = rest[index] ?? "";
    if (!token.startsWith("--")) {
      continue;
    }
    const name = token.slice(2);
    const value = rest[index + 1];
    if (!value || value.startsWith("--")) {
      flags.add(name);
      continue;
    }
    args.set(name, value);
    index += 1;
  }

  const sessionId = args.get("session-id") || process.env.POKECRYSTAL_SESSION_ID || randomUUID();
  const model = args.get("model") || process.env.POKECRYSTAL_AGENT_MODEL || DEFAULT_MODEL;
  const goal = args.get("goal") || DEFAULT_GOAL;
  const baseUrl =
    args.get("mcp-base-url") ||
    process.env.POKECRYSTAL_MCP_BASE_URL ||
    process.env.POKECRYSTAL_BASE_URL ||
    "";
  const mcpBaseUrl = baseUrl.trim() || "http://127.0.0.1:3000";
  const mcpUrl = args.get("mcp-url") || process.env.POKECRYSTAL_MCP_URL;

  return {
    command,
    sessionId,
    model,
    baseUrl,
    mcpBaseUrl,
    mcpUrl,
    maxSteps: parseMaxSteps(args.get("max-steps") ?? process.env.POKECRYSTAL_AGENT_MAX_STEPS),
    graphCycleSteps: parseNumber(args.get("graph-cycle-steps"), DEFAULT_GRAPH_CYCLE_STEPS),
    identityName: args.get("identity-name") || process.env.POKECRYSTAL_AGENT_IDENTITY_NAME || "krabbyclaw-agent",
    requestDelayMs: parseNumber(args.get("request-delay-ms"), DEFAULT_REQUEST_DELAY_MS),
    terminalUi: parseBooleanFlag(flags, "terminal-ui"),
    recordTraining: parseBooleanFlag(flags, "record-training") ? true : !parseBooleanFlag(flags, "no-record-training"),
    goal,
    ollamaBaseUrl:
      args.get("ollama-base-url") || process.env.LLAMA_CPP_BASE_URL || process.env.OLLAMA_BASE_URL,
    openaiBaseUrl: args.get("openai-base-url") || process.env.OPENAI_BASE_URL,
    openaiApiKey: args.get("openai-api-key") || process.env.OPENAI_API_KEY,
    anthropicBaseUrl: args.get("anthropic-base-url") || process.env.ANTHROPIC_BASE_URL,
    anthropicApiKey: args.get("anthropic-api-key") || process.env.ANTHROPIC_API_KEY,
    googleBaseUrl: args.get("google-base-url") || process.env.GOOGLE_GENERATIVE_AI_BASE_URL,
    googleApiKey:
      args.get("google-api-key") || process.env.GOOGLE_GENERATIVE_AI_API_KEY || process.env.GEMINI_API_KEY,
  };
};

const help = (): string =>
  [
    "pokecrystal-agents",
    "",
    "Usage:",
    "  run     --session-id <id> [--model <provider/model>] [--mcp-base-url <url>|--mcp-url <url>] [--max-steps N] [--graph-cycle-steps N] [--goal <text>] [--identity-name <name>] [--terminal-ui] [--record-training|--no-record-training]",
    "  resume  --session-id <id> [--model <provider/model>] [--mcp-base-url <url>|--mcp-url <url>] [--max-steps N]",
    "",
    "Options:",
    "  --session-id <id>          Stable gameplay session id. Defaults to POKECRYSTAL_SESSION_ID or a new UUID.",
    `  --model <provider/model>   Agent model. Default: ${DEFAULT_MODEL}`,
    "  --mcp-base-url <url>       App origin for the web MCP tools endpoint.",
    "  --mcp-url <url>            Exact streamable MCP URL, such as the URL printed by pokecrystal-cli play.",
    "  --max-steps N              Optional maximum supervised gameplay batches. Default: infinite.",
    `  --graph-cycle-steps N      Agent graph cycle budget per batch. Default: ${DEFAULT_GRAPH_CYCLE_STEPS}`,
    `  --request-delay-ms N       Delay between agent batches. Default: ${DEFAULT_REQUEST_DELAY_MS}`,
    "  --goal <text>              Goal prompt. Default: Beat Mt. Silver",
    "  --identity-name <name>     Player identity name. Default: krabbyclaw-agent",
    "  --terminal-ui              Enable terminal UI mode.",
    "  --record-training          Force training capture on.",
    "  --no-record-training       Disable training capture.",
    "",
    "Environment:",
    "  POKECRYSTAL_SESSION_ID, POKECRYSTAL_AGENT_MODEL, POKECRYSTAL_AGENT_IDENTITY_NAME",
    "  POKECRYSTAL_AGENT_MAX_STEPS",
    "  POKECRYSTAL_MCP_BASE_URL, POKECRYSTAL_BASE_URL, POKECRYSTAL_MCP_URL",
    "  LLAMA_CPP_BASE_URL or OLLAMA_BASE_URL plus OLLAMA_API_KEY for ollama/* models",
    "  AZURE_OPENAI_ENDPOINT, AZURE_OPENAI_API_KEY, and optional AZURE_OPENAI_API_VERSION for azure-openai/* direct models",
    "  OPENAI_API_KEY and optional OPENAI_BASE_URL for openai-direct/* models",
    "  ANTHROPIC_API_KEY and optional ANTHROPIC_BASE_URL, ANTHROPIC_API_VERSION for anthropic/* models",
    "  GOOGLE_GENERATIVE_AI_API_KEY or GEMINI_API_KEY, plus optional GOOGLE_GENERATIVE_AI_BASE_URL for google/* or gemini/* models",
    "",
    "Examples:",
    `  pokecrystal-agents run --session-id poke-run-01 --model ${DEFAULT_AGENT_MODEL} --mcp-base-url http://127.0.0.1:3000`,
    "  env LLAMA_CPP_BASE_URL=http://127.0.0.1:8080 OLLAMA_API_KEY=local pokecrystal-agents run --session-id local-llamacpp --model ollama/gemma-4-E4B-it-Q4_K_M.gguf --mcp-url http://127.0.0.1:<port>/mcp?session_id=local-llamacpp",
    "  env OPENAI_API_KEY=... pokecrystal-agents run --session-id openai-direct --model openai-direct/gpt-5.4-mini --mcp-url http://127.0.0.1:<port>/mcp?session_id=openai-direct",
    "  env ANTHROPIC_API_KEY=... pokecrystal-agents run --session-id claude --model anthropic/claude-sonnet-4-5 --mcp-url http://127.0.0.1:<port>/mcp?session_id=claude",
    "  env GEMINI_API_KEY=... pokecrystal-agents run --session-id gemini --model google/gemini-2.5-flash --mcp-url http://127.0.0.1:<port>/mcp?session_id=gemini",
  ].join("\n");

const printResult = (result: { step: number; finished: boolean; reason: string }): void => {
  process.stdout.write(
    `result: steps=${result.step} finished=${result.finished ? "yes" : "no"} reason="${result.reason}"\n`
  );
};

const formatCliError = (error: unknown): string => {
  if (error instanceof Error) {
    return error.stack || `${error.name}: ${error.message}`;
  }
  return String(error);
};

const main = async (argv: string[]): Promise<number> => {
  if (argv.includes("--help") || argv.includes("-h")) {
    process.stdout.write(`${help()}\n`);
    return 0;
  }

  const options = parseArgs(argv);
  if (options.command === "help") {
    process.stdout.write(`${help()}\n`);
    return 1;
  }

  try {
    const result = await runPokemonAgent(options);
    printResult(result);
    return 0;
  } catch (error) {
    process.stderr.write(`${formatCliError(error)}\n`);
    return 1;
  }
};

main(process.argv.slice(2))
  .then((code) => process.exit(code))
  .catch((error) => {
    process.stderr.write(`${formatCliError(error)}\n`);
    process.exit(1);
  });
