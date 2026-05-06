export type AgentStreamEvent =
  | {
      type: "status";
      message: string;
      source?: string;
    }
  | {
      type: "text-delta" | "thinking-delta";
      text: string;
      source?: string;
    }
  | {
      type: "tool-call";
      name: string;
      source?: string;
    }
  | {
      type: "mcp-call" | "mcp-result";
      name: string;
      summary?: string;
      source?: string;
    };

const STREAM_PREFIX = "POKECRYSTAL_AGENT_STREAM ";

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

export const agentStreamEventFromChunk = (
  chunk: unknown,
  source: string,
): AgentStreamEvent | null => {
  if (!isRecord(chunk) || typeof chunk.type !== "string") {
    return null;
  }
  const payload = isRecord(chunk.payload) ? chunk.payload : {};

  if (chunk.type === "text-delta" && typeof payload.text === "string") {
    return { type: "text-delta", text: payload.text, source };
  }

  if ((chunk.type === "reasoning" || chunk.type === "reasoning-delta") && typeof payload.text === "string") {
    return { type: "thinking-delta", text: payload.text, source };
  }

  if (chunk.type === "tool-call" && typeof payload.toolName === "string") {
    return { type: "tool-call", name: payload.toolName, source };
  }

  if (chunk.type === "tool-call" && typeof payload.name === "string") {
    return { type: "tool-call", name: payload.name, source };
  }

  return null;
};

export const shouldEmitAgentStreamEvents = (): boolean =>
  process.env.POKECRYSTAL_AGENT_STREAM_EVENTS === "1";

export const emitAgentStreamEvent = (
  event: AgentStreamEvent,
  output: Pick<NodeJS.WriteStream, "write"> = process.stdout,
): void => {
  output.write(`${STREAM_PREFIX}${JSON.stringify(event)}\n`);
};

export const emitAgentStreamStatus = (message: string, source?: string): void => {
  if (shouldEmitAgentStreamEvents()) {
    emitAgentStreamEvent({ type: "status", message, source });
  }
};

export const emitAgentMcpCall = (
  name: string,
  args: Record<string, unknown>,
  output?: Pick<NodeJS.WriteStream, "write">,
): void => {
  if (shouldEmitAgentStreamEvents()) {
    emitAgentStreamEvent({
      type: "mcp-call",
      name,
      summary: JSON.stringify(args),
      source: "mcp",
    }, output);
  }
};

export const emitAgentMcpResult = (
  name: string,
  summary: string,
  output?: Pick<NodeJS.WriteStream, "write">,
): void => {
  if (shouldEmitAgentStreamEvents()) {
    emitAgentStreamEvent({
      type: "mcp-result",
      name,
      summary,
      source: "mcp",
    }, output);
  }
};

export const emitAgentStreamChunk = (chunk: unknown, source: string): void => {
  if (!shouldEmitAgentStreamEvents()) {
    return;
  }
  const event = agentStreamEventFromChunk(chunk, source);
  if (event) {
    emitAgentStreamEvent(event);
  }
};
