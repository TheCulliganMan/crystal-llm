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

export type AgentStreamLogEntry = {
  type: AgentStreamEvent["type"];
  label: string;
  text: string;
};

export type AgentStreamState = {
  status?: string;
  text: string;
  thinking: string;
  mcpCalls: string[];
  events: AgentStreamLogEntry[];
};

export const emptyAgentStreamState = (): AgentStreamState => ({
  text: "",
  thinking: "",
  mcpCalls: [],
  events: [],
});

const STREAM_PREFIX = "POKECRYSTAL_AGENT_STREAM ";
const DEFAULT_TEXT_LIMIT = 2_000;
const DEFAULT_TOOL_LIMIT = 8;

const trimStartToLimit = (value: string, maxChars: number): string =>
  value.length > maxChars ? value.slice(value.length - maxChars) : value;

const splitInlineThinking = (text: string): { thinking: string; text: string } => {
  const thinkPattern = /<think>([\s\S]*?)<\/think>/gi;
  const thinking: string[] = [];
  const output = text.replace(thinkPattern, (_match, content: string) => {
    thinking.push(content);
    return "";
  });
  return {
    thinking: thinking.join(" "),
    text: output,
  };
};

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

export const parseAgentStreamLine = (line: string): AgentStreamEvent | null => {
  if (!line.startsWith(STREAM_PREFIX)) {
    return null;
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(line.slice(STREAM_PREFIX.length));
  } catch {
    return null;
  }
  if (!isRecord(parsed) || typeof parsed.type !== "string") {
    return null;
  }

  if (parsed.type === "status" && typeof parsed.message === "string") {
    return {
      type: "status",
      message: parsed.message,
      source: typeof parsed.source === "string" ? parsed.source : undefined,
    };
  }

  if ((parsed.type === "text-delta" || parsed.type === "thinking-delta") && typeof parsed.text === "string") {
    return {
      type: parsed.type,
      text: parsed.text,
      source: typeof parsed.source === "string" ? parsed.source : undefined,
    };
  }

  if (parsed.type === "tool-call" && typeof parsed.name === "string") {
    return {
      type: "tool-call",
      name: parsed.name,
      source: typeof parsed.source === "string" ? parsed.source : undefined,
    };
  }

  if ((parsed.type === "mcp-call" || parsed.type === "mcp-result") && typeof parsed.name === "string") {
    return {
      type: parsed.type,
      name: parsed.name,
      summary: typeof parsed.summary === "string" ? parsed.summary : undefined,
      source: typeof parsed.source === "string" ? parsed.source : undefined,
    };
  }

  return null;
};

const eventLabel = (event: AgentStreamEvent): string => {
  if (event.type === "status") {
    return event.source ?? "status";
  }
  if (event.type === "text-delta" || event.type === "thinking-delta") {
    return event.source ?? "agent";
  }
  if (event.type === "tool-call") {
    return event.source ?? "tool";
  }
  return event.source ?? "mcp";
};

const eventText = (event: AgentStreamEvent): string => {
  if (event.type === "status") {
    return event.message;
  }
  if (event.type === "text-delta" || event.type === "thinking-delta") {
    return event.text;
  }
  if (event.type === "tool-call") {
    return event.name;
  }
  if (event.type === "mcp-call" || event.type === "mcp-result") {
    return event.summary ? `${event.name} ${event.summary}` : event.name;
  }
  return "";
};

const appendEvent = (
  state: AgentStreamState,
  event: AgentStreamEvent,
  maxEvents: number,
): AgentStreamLogEntry[] => {
  const next = {
    type: event.type,
    label: eventLabel(event),
    text: eventText(event),
  };
  const previous = state.events.at(-1);
  if (
    previous &&
    previous.type === next.type &&
    previous.label === next.label &&
    (next.type === "text-delta" || next.type === "thinking-delta")
  ) {
    return [
      ...state.events.slice(0, -1),
      {
        ...previous,
        text: `${previous.text}${next.text}`,
      },
    ].slice(-maxEvents);
  }
  return [...state.events, next].slice(-maxEvents);
};

export const reduceAgentStreamState = (
  state: AgentStreamState,
  event: AgentStreamEvent,
  options: { maxChars?: number; maxToolCalls?: number; maxEvents?: number } = {},
): AgentStreamState => {
  const maxChars = options.maxChars ?? DEFAULT_TEXT_LIMIT;
  const maxToolCalls = options.maxToolCalls ?? DEFAULT_TOOL_LIMIT;
  const maxEvents = options.maxEvents ?? 80;
  if (event.type === "status") {
    return { ...state, status: event.message, events: appendEvent(state, event, maxEvents) };
  }
  if (event.type === "text-delta") {
    const split = splitInlineThinking(event.text);
    const nextEvents = [
      ...(split.thinking
        ? appendEvent(state, { type: "thinking-delta", text: split.thinking, source: event.source }, maxEvents)
        : state.events),
    ];
    const eventState = { ...state, events: nextEvents };
    return {
      ...state,
      text: trimStartToLimit(`${state.text}${split.text}`, maxChars),
      thinking: split.thinking
        ? trimStartToLimit(`${state.thinking}${split.thinking}`, maxChars)
        : state.thinking,
      events: split.text ? appendEvent(eventState, { ...event, text: split.text }, maxEvents) : nextEvents,
    };
  }
  if (event.type === "thinking-delta") {
    return {
      ...state,
      thinking: trimStartToLimit(`${state.thinking}${event.text}`, maxChars),
      events: appendEvent(state, event, maxEvents),
    };
  }
  if (event.type === "tool-call") {
    return {
      ...state,
      events: appendEvent(state, event, maxEvents),
    };
  }
  if (event.type === "mcp-call" || event.type === "mcp-result") {
    const rendered = event.summary ? `${event.name} ${event.summary}` : event.name;
    return {
      ...state,
      mcpCalls: [...state.mcpCalls, rendered].slice(-maxToolCalls),
      events: appendEvent(state, event, maxEvents),
    };
  }

  return {
    ...state,
    events: state.events.slice(-maxEvents),
  };
};

const wrapText = (label: string, text: string, maxLineLength: number): string[] => {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!normalized) {
    return [];
  }
  const prefix = `${label}: `;
  const inlineWidth = Math.max(8, maxLineLength - prefix.length);
  if (normalized.length <= inlineWidth) {
    return [`${prefix}${normalized}`];
  }

  const width = Math.max(8, maxLineLength);
  const lines: string[] = [`${label}:`];
  let remaining = normalized;
  while (remaining.length > 0) {
    if (remaining.length <= width) {
      lines.push(remaining);
      break;
    }
    const breakAt = remaining.lastIndexOf(" ", width);
    const chunkLength = breakAt >= Math.floor(width * 0.6) ? breakAt : width;
    lines.push(remaining.slice(0, chunkLength).trimEnd());
    remaining = remaining.slice(chunkLength).trimStart();
  }
  return lines;
};

const eventTypeLabel = (event: AgentStreamLogEntry): string => {
  switch (event.type) {
    case "status":
      return "status";
    case "thinking-delta":
      return "reason";
    case "text-delta":
      return "output";
    case "tool-call":
      return "tool";
    case "mcp-call":
      return "call";
    case "mcp-result":
      return "result";
  }
};

const limitWrappedGroups = (groups: string[][], maxLines: number): string[] => {
  const lines: string[] = [];
  for (let index = groups.length - 1; index >= 0; index -= 1) {
    const group = groups[index] ?? [];
    if (!group.length) {
      continue;
    }
    const remaining = maxLines - lines.length;
    if (remaining <= 0) {
      break;
    }
    if (group.length > remaining) {
      if (!lines.length) {
        lines.unshift(...(remaining > 1 ? [group[0]!, ...group.slice(-(remaining - 1))] : [group[0]!]));
      }
      break;
    }
    lines.unshift(...group);
  }
  return lines.slice(-maxLines);
};

export const renderAgentStreamLines = (
  state: AgentStreamState | undefined,
  options: { maxLines: number; maxLineLength: number },
): string[] => {
  if (!state) {
    return ["No linked agent stream yet."];
  }

  const lines = [
    ...(state.status ? [`STATUS: ${state.status}`] : []),
    ...wrapText("THINKING", state.thinking, options.maxLineLength),
    ...wrapText("TOKENS", state.text, options.maxLineLength),
    ...((state.mcpCalls ?? []).length ? [`MCP: ${(state.mcpCalls ?? []).at(-1)}`] : []),
  ];

  return (lines.length ? lines : ["Waiting for agent tokens..."]).slice(-options.maxLines);
};

export const renderAgentCurrentLines = (
  state: AgentStreamState | undefined,
  options: { maxLines: number; maxLineLength: number },
): string[] => {
  if (!state) {
    return ["No linked agent stream yet."];
  }
  const toolLines = (state.events ?? [])
    .filter((event) => event.type === "tool-call")
    .slice(-2)
    .flatMap((event) => wrapText(event.label.toUpperCase(), event.text, options.maxLineLength));
  const lines = [
    ...(state.status ? [`STATUS: ${state.status}`] : []),
    ...wrapText("THINKING", state.thinking, options.maxLineLength),
    ...toolLines,
  ];
  return (lines.length ? lines : ["Waiting for agent status..."]).slice(-options.maxLines);
};

export const renderAgentTokenLines = (
  state: AgentStreamState | undefined,
  options: { maxLines: number; maxLineLength: number },
): string[] => {
  if (!state) {
    return ["No linked agent stream yet."];
  }
  const lines = wrapText("TOKENS", state.text, options.maxLineLength);
  return (lines.length ? lines : ["Waiting for tokens..."]).slice(-options.maxLines);
};

export const renderAgentMcpLines = (
  state: AgentStreamState | undefined,
  options: { maxLines: number; maxLineLength: number },
): string[] => {
  if (!state) {
    return ["No linked agent stream yet."];
  }
  const lines = (state.mcpCalls ?? []).flatMap((call) =>
    wrapText("MCP", call, options.maxLineLength)
  );
  return (lines.length ? lines : ["Waiting for MCP calls..."]).slice(-options.maxLines);
};

export const renderAgentEventLines = (
  state: AgentStreamState | undefined,
  options: {
    maxLines: number;
    maxLineLength: number;
    types?: AgentStreamEvent["type"][];
    labelMode?: "source" | "type";
  },
): string[] => {
  if (!state) {
    return ["No linked agent stream yet."];
  }
  const allowed = options.types ? new Set(options.types) : null;
  const events = state.events ?? [];
  const source = allowed ? events.filter((event) => allowed.has(event.type)) : events;
  const groups = source.map((event) =>
    wrapText(
      (options.labelMode === "type" ? eventTypeLabel(event) : event.label).toUpperCase(),
      event.text,
      options.maxLineLength,
    ),
  );
  const lines = limitWrappedGroups(groups, options.maxLines);
  return lines.length ? lines : ["Waiting for agent events..."];
};
