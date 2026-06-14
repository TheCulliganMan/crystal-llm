import {
  agentStreamEventFromChunk,
  emitAgentMcpCall,
  emitAgentStreamEvent,
} from "./stream-events.js";

describe("agent stream events", () => {
  it("maps Mastra text and reasoning chunks to TUI stream events", () => {
    expect(agentStreamEventFromChunk({ type: "text-delta", payload: { text: "press start" } }, "taskmaster")).toEqual({
      type: "text-delta",
      text: "press start",
      source: "taskmaster",
    });
    expect(agentStreamEventFromChunk({ type: "reasoning-delta", payload: { text: "title screen" } }, "taskmaster")).toEqual({
      type: "thinking-delta",
      text: "title screen",
      source: "taskmaster",
    });
  });

  it("emits one prefixed JSONL event for the TUI pipe", () => {
    const writes: string[] = [];
    emitAgentStreamEvent(
      { type: "tool-call", name: "press", source: "player" },
      { write: (chunk: string) => { writes.push(chunk); return true; } },
    );

    expect(writes).toEqual([
      'POKECRYSTAL_AGENT_STREAM {"type":"tool-call","name":"press","source":"player"}\n',
    ]);
  });

  it("emits MCP calls as structured stream events", () => {
    const writes: string[] = [];
    const original = process.env.POKECRYSTAL_AGENT_STREAM_EVENTS;
    process.env.POKECRYSTAL_AGENT_STREAM_EVENTS = "1";
    try {
      emitAgentMcpCall(
        "move",
        { direction: "down", steps: 1 },
        { write: (chunk: string) => { writes.push(chunk); return true; } },
      );
    } finally {
      if (original === undefined) {
        delete process.env.POKECRYSTAL_AGENT_STREAM_EVENTS;
      } else {
        process.env.POKECRYSTAL_AGENT_STREAM_EVENTS = original;
      }
    }

    expect(writes).toEqual([
      'POKECRYSTAL_AGENT_STREAM {"type":"mcp-call","name":"move","summary":"{\\"direction\\":\\"down\\",\\"steps\\":1}","source":"mcp"}\n',
    ]);
  });
});
