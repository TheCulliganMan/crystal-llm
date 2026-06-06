import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  emptyAgentStreamState,
  parseAgentStreamLine,
  reduceAgentStreamState,
  renderAgentEventLines,
  renderAgentStreamLines,
} from "./agent-stream";
import {
  __testing as tuiTesting,
  buildAgentManualInterventionOptions,
  buildAgentInterruptOptions,
  createKeypressChunkParser,
  createDialogueAccumulator,
  mapKeypressToAction,
  normalizeTuiSnapshot,
  resolveDirectionalAction,
  resolveLowercaseAAction,
  resolveNameEntryKeypressActions,
  splitKeypressChunk,
  updateCommandMode,
} from "./tui";
import {
  createInkTuiApp,
  formatElapsedRunTime,
  renderInkTui,
  resolveControlLines,
  resolveGameBoyImageCellSize,
  resolveTuiLayout,
  type InkRuntime,
  type TuiViewState,
} from "./tui-ink";
import {
  createTuiSoundController,
  extractTuiAudioPlaybackSnapshot,
  resolveTuiAudioSourcePath,
} from "./tui-sound";
import {
  KITTY_PLACEHOLDER_CELL,
  buildKittyDeleteSequence,
  buildKittyFrameSequence,
  buildKittyPlaceholderColor,
  buildKittyPlaceholderLines,
  buildKittyUploadSequence,
  buildKittyVirtualPlacementSequence,
  createKittyImageRenderer,
  extractKittyPngFrame,
  isKittyGraphicsSupported,
  isKittyPlaceholderModeEnabled,
  resolveKittyImageId,
  resolveKittyImageIds,
} from "./tui-kitty";

type FakeInkNode = {
  type: unknown;
  props?: Record<string, unknown> | null;
  children: unknown[];
};

const createRangeLines = (prefix: string, count: number): string[] =>
  Array.from({ length: count }, (_value, index) => `${prefix} ${index + 1}`);

const createFakePngBase64 = (width = 160, height = 144): string => {
  const bytes = Buffer.alloc(24);
  Buffer.from("89504e470d0a1a0a", "hex").copy(bytes, 0);
  bytes.writeUInt32BE(13, 8);
  bytes.write("IHDR", 12, "ascii");
  bytes.writeUInt32BE(width, 16);
  bytes.writeUInt32BE(height, 20);
  return bytes.toString("base64");
};

const createFakePngBase64Variant = (
  variant: number,
  width = 160,
  height = 144,
): string => {
  const bytes = Buffer.alloc(25);
  Buffer.from(createFakePngBase64(width, height), "base64").copy(bytes, 0);
  bytes[24] = variant;
  return bytes.toString("base64");
};

const countKittyPlaceholderCells = (line: string): number =>
  Array.from(line).filter((character) => character === KITTY_PLACEHOLDER_CELL)
    .length;

const createTuiViewState = (): TuiViewState => ({
  title: "PokeCrystal CLI / Live Play",
  endpoint: "http://127.0.0.1:43210/mcp?session_id=session-1",
  sessionId: "session-1",
  startedAtMs: 1_000,
  elapsedMs: 3_723_000,
  interactionCount: 42,
  agentStream: undefined,
  snapshot: {
    mode: "overworld",
    map: "NEW_BARK_TOWN",
    surface: "overworld",
    promptStatus: "idle",
    frame: "12",
    viewport: createRangeLines("viewport", 40),
    info: createRangeLines("info", 20),
    legend: createRangeLines("legend", 20),
    menu: createRangeLines("menu", 20),
    prompt: createRangeLines("prompt", 20),
    dialogue: createRangeLines("dialogue", 20),
    actions: createRangeLines("action", 20),
    statusLine:
      "STATE: overworld | MAP: NEW_BARK_TOWN | PROMPT: idle | FRAME #12",
  },
});

describe("agent stream telemetry", () => {
  it("parses linked-agent JSONL stream events and ignores ordinary process output", () => {
    expect(parseAgentStreamLine("result: steps=1")).toBeNull();
    expect(
      parseAgentStreamLine(
        'POKECRYSTAL_AGENT_STREAM {"type":"thinking-delta","text":"consider title screen","source":"taskmaster"}',
      ),
    ).toEqual({
      type: "thinking-delta",
      text: "consider title screen",
      source: "taskmaster",
    });
    expect(
      parseAgentStreamLine(
        'POKECRYSTAL_AGENT_STREAM {"type":"text-delta","text":"press start","source":"player"}',
      ),
    ).toEqual({
      type: "text-delta",
      text: "press start",
      source: "player",
    });
  });

  it("reduces streamed thinking, token, status, and tool-call deltas into renderable lines", () => {
    let state = emptyAgentStreamState();
    state = reduceAgentStreamState(state, {
      type: "status",
      message: "batch 1 running",
    });
    state = reduceAgentStreamState(state, {
      type: "thinking-delta",
      text: "Need to clear title. ",
    });
    state = reduceAgentStreamState(state, {
      type: "text-delta",
      text: "Press START, then A. ",
    });
    state = reduceAgentStreamState(state, { type: "tool-call", name: "press" });
    state = reduceAgentStreamState(state, {
      type: "mcp-call",
      name: "press",
      summary: '{"button":"a"}',
    });

    expect(
      renderAgentStreamLines(state, { maxLines: 8, maxLineLength: 80 }),
    ).toEqual([
      "STATUS: batch 1 running",
      "THINKING: Need to clear title.",
      "TOKENS: Press START, then A.",
      'MCP: press {"button":"a"}',
    ]);
  });

  it("treats local-model <think> text as reasoning instead of agent output", () => {
    let state = emptyAgentStreamState();
    state = reduceAgentStreamState(state, {
      type: "text-delta",
      text: "<think>Need to inspect the map.</think>Move north.",
    });

    expect(
      renderAgentStreamLines(state, { maxLines: 8, maxLineLength: 80 }),
    ).toEqual(["THINKING: Need to inspect the map.", "TOKENS: Move north."]);
  });

  it("coalesces adjacent token deltas from the same source into readable output", () => {
    let state = emptyAgentStreamState();
    for (const text of [
      "to",
      " advance",
      " towards",
      " the",
      " Professor",
      "'s",
      " Lab",
    ]) {
      state = reduceAgentStreamState(state, {
        type: "text-delta",
        source: "taskmaster",
        text,
      });
    }

    expect(
      renderAgentEventLines(state, {
        maxLines: 4,
        maxLineLength: 80,
        types: ["text-delta"],
      }),
    ).toEqual(["TASKMASTER: to advance towards the Professor's Lab"]);
  });

  it("clips wrapped agent events with the leading label intact", () => {
    const state = {
      text: "",
      thinking: "",
      mcpCalls: [],
      events: [
        { type: "text-delta" as const, label: "agent", text: "older event" },
        {
          type: "text-delta" as const,
          label: "agent",
          text: "newer event with enough words to wrap across several narrow terminal lines",
        },
      ],
    };

    const lines = renderAgentEventLines(state, {
      maxLines: 2,
      maxLineLength: 28,
      types: ["text-delta"],
    });

    expect(lines[0]?.startsWith("AGENT:")).toBe(true);
    expect(lines).toHaveLength(2);
  });

  it("renders wrapped event values flush left after a standalone label", () => {
    const state = {
      text: "",
      thinking: "",
      mcpCalls: [],
      events: [
        {
          type: "mcp-result" as const,
          label: "mcp",
          text: "map_info alpha beta gamma delta epsilon zeta eta theta",
        },
      ],
    };

    const lines = renderAgentEventLines(state, {
      maxLines: 8,
      maxLineLength: 24,
      types: ["mcp-result"],
      labelMode: "type",
    });

    expect(lines[0]).toBe("RESULT:");
    expect(lines.slice(1).every((line) => !line.startsWith(" "))).toBe(true);
    expect(lines.join("\n")).toContain("map_info alpha beta");
  });
});

const renderFakeInkTree = (
  columns: number,
  rows: number,
  state: TuiViewState = createTuiViewState(),
  options: { useStdoutHook?: boolean } = {},
): FakeInkNode => {
  const useStdoutHook = options.useStdoutHook ?? true;
  const runtime: InkRuntime = {
    React: {
      createElement: (
        type: unknown,
        props?: Record<string, unknown> | null,
        ...children: unknown[]
      ) => {
        if (typeof type === "function") {
          return (type as (componentProps: Record<string, unknown>) => unknown)(
            { ...(props ?? {}), children },
          );
        }
        return { type, props, children };
      },
      useEffect: (effect) => {
        effect();
      },
      useState: <T>(
        initial: T | (() => T),
      ): [T, (next: T | ((previous: T) => T)) => void] => [
        typeof initial === "function" ? (initial as () => T)() : initial,
        () => undefined,
      ],
    },
    ink: {
      Box: "Box",
      Text: "Text",
      useStdoutDimensions: useStdoutHook ? () => [columns, rows] : undefined,
      render: () => ({ unmount: () => undefined }),
    },
  };

  return createInkTuiApp(runtime, {
    initialState: state,
    terminal: { columns, rows },
    subscribe: () => () => undefined,
  }) as FakeInkNode;
};

const textValue = (node: unknown): string | undefined => {
  if (!node || typeof node !== "object") {
    return undefined;
  }
  const children = (node as FakeInkNode).children;
  return children.length === 1 && typeof children[0] === "string"
    ? children[0]
    : undefined;
};

const panelByTitle = (
  node: unknown,
  title: string,
): FakeInkNode | undefined => {
  if (!node || typeof node !== "object") {
    return undefined;
  }
  const current = node as FakeInkNode;
  const firstChild = current.children[0];
  if (textValue(firstChild) === title) {
    return current;
  }
  for (const child of current.children) {
    const found = panelByTitle(child, title);
    if (found) {
      return found;
    }
  }
  return undefined;
};

const panelTextLines = (panel: FakeInkNode): string[] =>
  panel.children
    .slice(1)
    .map((child) => textValue(child))
    .filter((line): line is string => Boolean(line));

const findNode = (
  node: unknown,
  matcher: (node: FakeInkNode) => boolean,
): FakeInkNode | undefined => {
  if (!node || typeof node !== "object") {
    return undefined;
  }
  const current = node as FakeInkNode;
  if (matcher(current)) {
    return current;
  }
  for (const child of current.children) {
    const found = findNode(child, matcher);
    if (found) {
      return found;
    }
  }
  return undefined;
};

const findText = (
  node: unknown,
  matcher: (text: string) => boolean,
): string | undefined => {
  const value = textValue(node);
  if (value && matcher(value)) {
    return value;
  }
  if (!node || typeof node !== "object") {
    return undefined;
  }
  for (const child of (node as FakeInkNode).children) {
    const found = findText(child, matcher);
    if (found) {
      return found;
    }
  }
  return undefined;
};

const collectText = (
  node: unknown,
  matcher: (text: string) => boolean,
  output: string[] = [],
): string[] => {
  const value = textValue(node);
  if (value && matcher(value)) {
    output.push(value);
  }
  if (!node || typeof node !== "object") {
    return output;
  }
  for (const child of (node as FakeInkNode).children) {
    collectText(child, matcher, output);
  }
  return output;
};

describe("mapKeypressToAction", () => {
  it("maps hand-play keys to Game Boy actions", () => {
    expect(mapKeypressToAction("w")).toEqual({
      type: "direction",
      direction: "up",
    });
    expect(mapKeypressToAction("\u001bOA")).toEqual({
      type: "direction",
      direction: "up",
    });
    expect(mapKeypressToAction("\u001b[1;2A")).toEqual({
      type: "direction",
      direction: "up",
    });
    expect(mapKeypressToAction("h")).toEqual({
      type: "direction",
      direction: "left",
    });
    expect(mapKeypressToAction("\u001bOD")).toEqual({
      type: "direction",
      direction: "left",
    });
    expect(mapKeypressToAction("l")).toEqual({
      type: "direction",
      direction: "right",
    });
    expect(mapKeypressToAction("\u001bOC")).toEqual({
      type: "direction",
      direction: "right",
    });
    expect(mapKeypressToAction("\u001b[1;5C")).toEqual({
      type: "direction",
      direction: "right",
    });
    expect(mapKeypressToAction("\u001bOB")).toEqual({
      type: "direction",
      direction: "down",
    });
    expect(mapKeypressToAction(" ")).toEqual({ type: "press", button: "a" });
    expect(mapKeypressToAction("z")).toEqual({ type: "press", button: "a" });
    expect(mapKeypressToAction("A")).toEqual({ type: "press", button: "a" });
    expect(mapKeypressToAction("x")).toEqual({ type: "press", button: "b" });
    expect(mapKeypressToAction("\u001b")).toEqual({ type: "press", button: "b" });
    expect(mapKeypressToAction("\t")).toEqual({
      type: "press",
      button: "select",
    });
    expect(mapKeypressToAction("\r")).toEqual({
      type: "press",
      button: "start",
    });
    expect(mapKeypressToAction("\n")).toEqual({
      type: "press",
      button: "start",
    });
    expect(mapKeypressToAction(".")).toEqual({ type: "wait", frames: 8 });
    expect(mapKeypressToAction("r")).toEqual({ type: "refresh" });
  });
});

describe("splitKeypressChunk", () => {
  it("keeps arrow escape sequences intact while splitting buffered text", () => {
    expect(splitKeypressChunk("\u001b[Aaz")).toEqual(["\u001b[A", "a", "z"]);
  });

  it("keeps application-cursor arrow escape sequences intact", () => {
    expect(splitKeypressChunk("\u001bOAaz")).toEqual(["\u001bOA", "a", "z"]);
  });

  it("keeps modified CSI arrow escape sequences intact", () => {
    expect(splitKeypressChunk("\u001b[1;2Aaz\u001b[1;5C")).toEqual([
      "\u001b[1;2A",
      "a",
      "z",
      "\u001b[1;5C",
    ]);
  });

  it("keeps terminal Delete and End escape sequences intact", () => {
    expect(splitKeypressChunk("\u001b[3~a\u001b[F")).toEqual(["\u001b[3~", "a", "\u001b[F"]);
    expect(splitKeypressChunk("\u001bOF")).toEqual(["\u001bOF"]);
    expect(splitKeypressChunk("\u001b[1;5F")).toEqual(["\u001b[1;5F"]);
  });
});

describe("createKeypressChunkParser", () => {
  it("buffers arrow escape bytes split across TTY chunks", () => {
    const parser = createKeypressChunkParser();

    expect(parser.push("\u001b")).toEqual([]);
    expect(parser.push("O")).toEqual([]);
    expect(parser.push("B")).toEqual(["\u001bOB"]);
    expect(parser.push("\u001b[")).toEqual([]);
    expect(parser.push("1;5Cz")).toEqual(["\u001b[1;5C", "z"]);
  });

  it("recognizes every arrow direction when escape sequences arrive one byte at a time", () => {
    const parser = createKeypressChunkParser();
    const keys: string[] = [];

    for (const sequence of ["\u001bOA", "\u001bOB", "\u001bOC", "\u001bOD"]) {
      for (const byte of sequence) {
        keys.push(...parser.push(byte));
      }
    }

    expect(keys.map(mapKeypressToAction)).toEqual([
      { type: "direction", direction: "up" },
      { type: "direction", direction: "down" },
      { type: "direction", direction: "right" },
      { type: "direction", direction: "left" },
    ]);
  });
});

describe("resolveDirectionalAction", () => {
  it("uses move for walkable overworld d-pad input", () => {
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "left" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({ mode: "overworld", can_move: true }),
            },
          ],
        },
      ),
    ).toEqual({ type: "move", direction: "left" });
  });

  it("uses directional button presses for menu and prompt surfaces", () => {
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "down" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({ mode: "menu", can_move: false, surface: { menu_open: true } }),
            },
          ],
        },
      ),
    ).toEqual({ type: "press", button: "down" });
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "up" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({ mode: "overworld", menu: true }),
            },
          ],
        },
      ),
    ).toEqual({ type: "press", button: "up" });
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "left" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({ mode: "overworld", prompt_pending: true }),
            },
          ],
        },
      ),
    ).toEqual({ type: "press", button: "left" });
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "up" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                mode: "overworld",
                menu: false,
                can_move: true,
                surface: { kind: "pokegear" },
              }),
            },
          ],
        },
      ),
    ).toEqual({ type: "press", button: "up" });
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "down" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                mode: "overworld",
                can_move: false,
                input_blocked_reason: "prompt",
                prompt_pending: true,
                surface: { kind: "fly_to_where", title: "FLY TO WHERE?" },
              }),
            },
          ],
        },
      ),
    ).toEqual({ type: "press", button: "down" });
  });

  it("ignores directional input while plain dialogue owns input", () => {
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "up" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                mode: "overworld",
                in_dialog: true,
                text_box_open: true,
                text_advance_pending: true,
                input_blocked_reason: "dialogue",
              }),
            },
          ],
        },
      ),
    ).toEqual({ type: "noop" });
  });

  it("keeps battle directional input on the move tool even when a menu surface is reported", () => {
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "down" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({ mode: "battle", can_move: false, surface: { menu_open: true } }),
            },
          ],
        },
      ),
    ).toEqual({ type: "move", direction: "down" });
  });

  it("sends Unown puzzle directional input as d-pad button presses", () => {
    expect(
      resolveDirectionalAction(
        { type: "direction", direction: "left" },
        {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                mode: "overworld",
                prompt_pending: true,
                input_blocked_reason: "prompt",
                unown_puzzle_active: true,
                unown_state: 1,
              }),
            },
          ],
        },
      ),
    ).toEqual({ type: "press", button: "left" });
  });
});

describe("resolveLowercaseAAction", () => {
  it("treats lowercase a as confirm on boot menu surfaces", () => {
    expect(
      resolveLowercaseAAction("a", {
        content: [
          {
            type: "text",
            text: JSON.stringify({ mode: "main_menu", can_move: false }),
          },
        ],
      }),
    ).toEqual({ type: "press", button: "a" });
  });

  it("leaves lowercase a as movement on overworld surfaces", () => {
    expect(
      resolveLowercaseAAction("a", {
        content: [
          {
            type: "text",
            text: JSON.stringify({ mode: "overworld", can_move: true }),
          },
        ],
      }),
    ).toBeNull();
  });

  it("treats lowercase a as confirm on the Unown puzzle overlay", () => {
    expect(
      resolveLowercaseAAction("a", {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              mode: "overworld",
              input_blocked_reason: "prompt",
              unown_puzzle_active: true,
              unown_state: 1,
            }),
          },
        ],
      }),
    ).toEqual({ type: "press", button: "a" });
  });
});

describe("TUI buffered input stop conditions", () => {
  const result = (text: string) => ({
    content: [{ type: "text", text }],
  });

  it("stops buffered movement when a field script starts after stepping onto Mom's coord event", () => {
    expect(
      tuiTesting.shouldStopBufferedInput(
        { type: "move", direction: "right" },
        result(JSON.stringify({ mode: "overworld", map: "PlayersHouse1F", can_move: true })),
        result(JSON.stringify({ ok: false, reason: "busy", events: ["moved:1", "interrupted:movement_lock"] })),
        result(JSON.stringify({ mode: "overworld", map: "PlayersHouse1F", can_move: false })),
      ),
    ).toBe(true);
  });

  it("keeps buffered menu d-pad input flowing", () => {
    expect(
      tuiTesting.shouldStopBufferedInput(
        { type: "move", direction: "down" },
        result(JSON.stringify({ mode: "overworld", menu: true, can_move: false })),
        result(JSON.stringify({ ok: true, changed: true })),
        result(JSON.stringify({ mode: "overworld", menu: true, can_move: false })),
      ),
    ).toBe(false);
  });
});

describe("TUI text advance status", () => {
  const result = (payload: Record<string, unknown>) => ({
    content: [{ type: "text", text: JSON.stringify(payload) }],
  });

  it("detects A presses that opened dialogue before the bundled frame has text", () => {
    expect(
      tuiTesting.didOpenTextForAction(
        { type: "press", button: "a" },
        result({
          ok: true,
          changed: true,
          summary: "text advance opened",
          tui: {
            frame: { view: { viewport: ["00 . @ ."], dialogue: [] } },
            status: {
              mode: "overworld",
              in_dialog: false,
              text_advance_pending: false,
            },
            recent_events: {
              events: [{ summary: "text advance opened" }],
            },
          },
        }),
      ),
    ).toBe(true);
  });

  it("does not treat the PC list instruction line as dialogue to advance", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "pc",
          map: "BILL'S PC",
          input_blocked_reason: "pc",
          in_dialog: false,
          prompt_pending: true,
          text_advance_pending: false,
          surface: {
            kind: "pc",
            title: "Bill's PC",
            promptOpen: true,
            primaryText: "Choose a <PK><MN>.",
          },
        }),
      ),
    ).toBe(false);
  });

  it.each([
    ["top menu", { menuOpen: true, primaryText: "WITHDRAW <PK><MN>" }],
    ["action menu", { menuOpen: true, primaryText: "What's up?" }],
    ["deposit list", { promptOpen: true, primaryText: "Select a Pokémon." }],
    ["move list", { promptOpen: true, primaryText: "Move to where?" }],
  ])("does not treat the PC %s surface as dialogue to advance", (_label, surface) => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "menu",
          map: "PC",
          input_blocked_reason: "pc",
          in_dialog: false,
          prompt_pending: true,
          surface: {
            kind: "pc",
            title: "Bill's PC",
            ...surface,
          },
        }),
      ),
    ).toBe(false);
  });

  it("does not let stale dialogue flags take over the PC hub menu", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "menu",
          map: "PC",
          input_blocked_reason: "menu",
          in_dialog: true,
          prompt_pending: true,
          text_advance_pending: true,
          surface: {
            kind: "pc",
            title: "PC",
            menuOpen: true,
            selected: "BILL's PC",
            primaryText: "▶ BILL's PC",
          },
        }),
      ),
    ).toBe(false);
  });

  it("does not treat a renderer-titled Prompt PC top menu as dialogue to advance", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "menu",
          map: "EcruteakPokecenter1F",
          input_blocked_reason: "menu",
          in_dialog: true,
          text_advance_pending: true,
          surface: {
            kind: "prompt",
            title: "Prompt",
            menu_open: true,
            selected: "WITHDRAW <PK><MN>",
            primary_text: "▶ WITHDRAW <PK><MN>",
          },
        }),
      ),
    ).toBe(false);
  });

  it("does not drive mart-owned dialogue through the generic dialogue macro", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "overworld",
          map: "EcruteakMart",
          in_dialog: true,
          text_box_open: true,
          input_blocked_reason: "dialogue",
          surface: {
            kind: "mart",
            title: "Mart",
            dialogue_open: true,
            primary_text: "Welcome! How may I",
          },
        }),
      ),
    ).toBe(false);
  });

  it("does not treat the Unown puzzle overlay as dialogue to advance", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "overworld",
          map: "RuinsOfAlphKabutoChamber",
          prompt_pending: true,
          input_blocked_reason: "prompt",
          unown_puzzle_active: true,
          unown_state: 1,
          surface: {
            kind: "overworld",
            title: "Overworld",
          },
        }),
      ),
    ).toBe(false);
  });

  it.each([
    ["snake-case hub", { menu_open: true, primary_text: "▶ BILL's PC" }],
    ["camel-case hub", { menuOpen: true, primaryText: "▶ CHRIS's PC" }],
    ["withdraw list", { prompt_open: true, primary_text: "Choose a <PK><MN>." }],
    ["deposit list", { promptOpen: true, primaryText: "Select a POKéMON." }],
    ["action submenu", { menu_open: true, primary_text: "What's up?" }],
    ["move target list", { prompt_open: true, primary_text: "Move to where?" }],
  ])("keeps stale dialogue state from swallowing A on the PC %s", (_label, surface) => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "menu",
          map: "BILL'S PC",
          input_blocked_reason: "menu",
          in_dialog: true,
          prompt_pending: true,
          text_advance_pending: true,
          surface: {
            kind: "pc",
            title: "Bill's PC",
            ...surface,
          },
        }),
      ),
    ).toBe(false);
  });

  it("still treats a real PC yes/no prompt as text to advance", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "pc",
          map: "BILL'S PC",
          prompt_pending: true,
          surface: {
            kind: "pc",
            title: "Bill's PC",
            promptOpen: true,
            primaryText: "Release GEODUDE?",
          },
        }),
      ),
    ).toBe(true);
  });

  it("still treats non-PC prompt surfaces as text to advance", () => {
    expect(
      tuiTesting.isTextAdvanceStatus(
        result({
          mode: "overworld",
          map: "EcruteakCity",
          surface: {
            kind: "overworld",
            promptOpen: true,
          },
        }),
      ),
    ).toBe(true);
  });
});

describe("resolveNameEntryKeypressActions", () => {
  it("sends typed letters as literal name-entry text", () => {
    const snapshot = {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: ["STATE: name_entry", "CURSOR: row 0 col 0"],
            },
          }),
        },
      ],
    };

    expect(resolveNameEntryKeypressActions("C", snapshot)).toEqual([
      { type: "text", text: "C" },
    ]);
  });

  it("preserves lowercase typed letters as literal name-entry text", () => {
    const snapshot = {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: ["STATE: name_entry", "CASE: upper", "CURSOR: row 0 col 0"],
            },
          }),
        },
      ],
    };

    expect(resolveNameEntryKeypressActions("c", snapshot)).toEqual([
      { type: "text", text: "c" },
    ]);
    expect(resolveNameEntryKeypressActions("d", snapshot)).toEqual([
      { type: "text", text: "d" },
    ]);
    expect(resolveNameEntryKeypressActions("o", snapshot)).toEqual([
      { type: "text", text: "o" },
    ]);
  });

  it("prioritizes literal letters over Game Boy key aliases on name entry", () => {
    const snapshot = {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: ["STATE: name_entry", "CASE: upper", "CURSOR: row 1 col 1", "SELECTED: K"],
            },
          }),
        },
      ],
    };

    for (const key of ["a", "b", "d", "j", "k", "l", "s", "w", "x", "z"]) {
      expect(resolveNameEntryKeypressActions(key, snapshot)).toEqual([
        { type: "text", text: key },
      ]);
    }
  });

  it("keeps non-letter physical Game Boy controls active on name entry", () => {
    const snapshot = {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: ["STATE: name_entry", "CASE: upper", "CURSOR: row 1 col 1", "SELECTED: K"],
            },
          }),
        },
      ],
    };

    const controls = [
      [" ", { type: "press", button: "a" }],
      ["\t", { type: "press", button: "select" }],
    ] as const;

    for (const [key, action] of controls) {
      expect(resolveNameEntryKeypressActions(key, snapshot)).toBeNull();
      expect(mapKeypressToAction(key)).toEqual(action);
    }
  });

  it("translates typed punctuation into naming-screen selection input", () => {
    const snapshot = {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: ["STATE: name_entry", "CURSOR: row 0 col 0"],
            },
          }),
        },
      ],
    };

    expect(resolveNameEntryKeypressActions(".", snapshot)).toEqual([
      { type: "direction", direction: "up" },
      { type: "direction", direction: "up" },
      { type: "direction", direction: "right" },
      { type: "direction", direction: "right" },
      { type: "direction", direction: "right" },
      { type: "direction", direction: "right" },
      { type: "press", button: "a" },
    ]);
  });

  it("maps terminal delete to DEL but leaves Enter as Start on the naming screen", () => {
    const snapshot = {
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: ["STATE: name_entry", "CURSOR: row 0 col 0"],
            },
          }),
        },
      ],
    };

    expect(resolveNameEntryKeypressActions("\u007f", snapshot)).toEqual([
      { type: "press", button: "b" },
    ]);
    expect(resolveNameEntryKeypressActions("\u001b[3~", snapshot)).toEqual([
      { type: "press", button: "b" },
    ]);
    expect(resolveNameEntryKeypressActions("\u001b[F", snapshot)).toEqual([
      { type: "press", button: "start" },
      { type: "press", button: "a" },
    ]);
    expect(resolveNameEntryKeypressActions("\u001b[1;5F", snapshot)).toEqual([
      { type: "press", button: "start" },
      { type: "press", button: "a" },
    ]);
    expect(
      resolveNameEntryKeypressActions("\u007f", {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              view: {
                viewport: ["NAME ENTRY"],
                info: ["STATE: name_entry", "CURSOR: row 4 col 6"],
              },
            }),
          },
        ],
      }),
    ).toEqual([
      { type: "press", button: "b" },
    ]);
    expect(resolveNameEntryKeypressActions("\r", snapshot)).toBeNull();
    expect(mapKeypressToAction("\r")).toEqual({
      type: "press",
      button: "start",
    });
  });

  it("ignores letters outside the naming screen", () => {
    expect(
      resolveNameEntryKeypressActions("C", {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              view: { viewport: ["OVERWORLD"], info: [] },
            }),
          },
        ],
      }),
    ).toBeNull();
  });
});

describe("updateCommandMode", () => {
  it("opens an empty command buffer with Escape instead of pre-filling a colon", () => {
    let state = updateCommandMode({ buffer: null }, "\u001b");
    expect(state.buffer).toBe("");
    expect(state.note).toBe("COMMAND>");

    state = updateCommandMode(state, ":");
    expect(state.buffer).toBe(":");
    expect(state.note).toBe("COMMAND> :");

    state = updateCommandMode({ buffer: null }, "\u001b");
    state = updateCommandMode(state, "\r");
    expect(state.buffer).toBeNull();

    state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "v");
    state = updateCommandMode(state, "\n");
    expect(state.viewCycleCommand).toBe(true);
  });

  it("captures vim-style quit commands", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "q");
    state = updateCommandMode(state, "!");
    state = updateCommandMode(state, "\r");
    expect(state.quitCommand).toBe("q!");
  });

  it("rejects plain quit to avoid accidental exits", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "q");
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain(":wq");
  });

  it("captures the view cycle command from command mode", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "v");
    state = updateCommandMode(state, "\r");
    expect(state.viewCycleCommand).toBe(true);

    state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "a");
    state = updateCommandMode(state, "s");
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");

    state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "p");
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");

    state = updateCommandMode({ buffer: null }, ":");
    for (const key of "set") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");

    state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "s");
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");
  });

  it("captures the Game Boy renderer toggle command from command mode", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "u");
    state = updateCommandMode(state, "\r");
    expect(state.viewCycleCommand).toBeUndefined();
    expect(state.gameboyRendererToggleCommand).toBe(true);

    state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "g");
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");
  });

  it("captures sound toggle commands from command mode", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "a");
    state = updateCommandMode(state, "\r");
    expect(state.soundToggleCommand).toBe(true);

    state = updateCommandMode({ buffer: null }, ":");
    for (const key of "sound") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");
  });

  it("captures controls toggle commands from command mode", () => {
    const state = updateCommandMode(
      updateCommandMode({ buffer: null }, ":"),
      "c",
    );
    expect(updateCommandMode(state, "\r").controlsToggleCommand).toBe(true);
  });

  it("captures agent start/pause and runtime setting commands from command mode", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "t");
    state = updateCommandMode(state, "\r");
    expect(state.agentToggleCommand).toBe(true);

    state = updateCommandMode({ buffer: null }, ":");
    for (const key of "set model ollama/qwen3:32b") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");
    expect(state.agentSettingCommand).toEqual({
      key: "model",
      value: "ollama/qwen3:32b",
    });

    state = updateCommandMode({ buffer: null }, ":");
    for (const key of "set steps 12") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");
    expect(state.agentSettingCommand).toEqual({ key: "maxSteps", value: 12 });

    state = updateCommandMode({ buffer: null }, ":");
    for (const key of "set delay nope") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("positive integer");
  });

  it("captures agent interrupt commands from command mode", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    for (const key of "i go heal before route 30") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");
    expect(state.agentMessageCommand).toBe("go heal before route 30");

    state = updateCommandMode({ buffer: null }, ":");
    state = updateCommandMode(state, "m");
    state = updateCommandMode(state, "\r");
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");
  });

  it("does not support :m as an agent interrupt alias", () => {
    let state = updateCommandMode({ buffer: null }, ":");
    for (const key of "m go heal before route 30") {
      state = updateCommandMode(state, key);
    }
    state = updateCommandMode(state, "\r");

    expect(state.agentMessageCommand).toBeUndefined();
    expect(state.isError).toBe(true);
    expect(state.note).toContain("Unknown command");
  });
});

describe("TUI audio snapshot playback", () => {
  it("extracts the web AudioEngine playback snapshot from status payloads", () => {
    const snapshot = extractTuiAudioPlaybackSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            mode: "overworld",
            audio: {
              musicToken: "MUSIC_ROUTE_29",
              musicSource: "/api/audio/route29.mp3",
              musicRole: "map",
              recentEvents: [
                {
                  sequence: 7,
                  kind: "sfx",
                  token: "SFX_READ_TEXT_2",
                  source: "/api/audio/sfx/readtext2.mp3",
                },
              ],
            },
          }),
        },
      ],
    });

    expect(snapshot).toEqual({
      musicToken: "MUSIC_ROUTE_29",
      musicSource: "/api/audio/route29.mp3",
      musicRole: "map",
      recentEvents: [
        {
          sequence: 7,
          kind: "sfx",
          token: "SFX_READ_TEXT_2",
          source: "/api/audio/sfx/readtext2.mp3",
        },
      ],
    });
  });

  it("resolves web audio URLs to local asset files before treating paths as absolute", () => {
    const previousAudioRoot = process.env.POKECRYSTAL_CLI_AUDIO_ROOT;
    const tempRoot = fs.mkdtempSync(
      path.join(os.tmpdir(), "pokecrystal-tui-audio-"),
    );
    const route29Path = path.join(tempRoot, "route29.mp3");
    const readTextPath = path.join(tempRoot, "sfx", "readtext2.mp3");
    fs.mkdirSync(path.dirname(readTextPath), { recursive: true });
    fs.writeFileSync(route29Path, "audio");
    fs.writeFileSync(readTextPath, "audio");

    try {
      process.env.POKECRYSTAL_CLI_AUDIO_ROOT = tempRoot;

      expect(resolveTuiAudioSourcePath("/api/audio/route29.mp3")).toBe(
        route29Path,
      );
      expect(resolveTuiAudioSourcePath("/assets/audio/sfx/readtext2.mp3")).toBe(
        readTextPath,
      );
      expect(resolveTuiAudioSourcePath("/audio/sfx/readtext2.mp3")).toBe(
        readTextPath,
      );
      expect(resolveTuiAudioSourcePath(route29Path)).toBe(route29Path);
    } finally {
      if (previousAudioRoot === undefined) {
        delete process.env.POKECRYSTAL_CLI_AUDIO_ROOT;
      } else {
        process.env.POKECRYSTAL_CLI_AUDIO_ROOT = previousAudioRoot;
      }
      fs.rmSync(tempRoot, { force: true, recursive: true });
    }
  });

  it("plays only new AudioEngine events and resumes current web music when enabled", () => {
    const played: Array<{
      token: string;
      kind: string;
      loop: boolean;
      source: string;
      pcm: Int16Array;
    }> = [];
    const killed: string[] = [];
    const pcmClip = { pcm: new Int16Array([1, 1, -1, -1]), sampleRate: 44_100 };
    const controller = createTuiSoundController({
      pcmResolver: () => pcmClip,
      player: (input) => {
        played.push(input);
        return { kill: () => killed.push(input.token) };
      },
    });
    const status = {
      musicToken: "MUSIC_ROUTE_29",
      musicSource: __filename,
      musicRole: "map",
      recentEvents: [
        {
          sequence: 1,
          kind: "sfx" as const,
          token: "SFX_READ_TEXT_2",
          source: __filename,
        },
      ],
    };

    controller.syncSnapshot(status);
    expect(played).toEqual([]);

    controller.setEnabled(true);
    controller.syncSnapshot(status);
    expect(played).toEqual([
      expect.objectContaining({
        token: "MUSIC_ROUTE_29",
        kind: "music",
        loop: true,
        source: __filename,
        pcm: pcmClip.pcm,
      }),
    ]);

    controller.syncSnapshot({
      ...status,
      recentEvents: [
        ...status.recentEvents,
        {
          sequence: 2,
          kind: "sfx" as const,
          token: "SFX_MENU",
          source: __filename,
        },
      ],
    });
    expect(played.at(-1)).toEqual(
      expect.objectContaining({ token: "SFX_MENU", kind: "sfx", loop: false }),
    );

    controller.setEnabled(false);
    expect(killed).toEqual(["MUSIC_ROUTE_29"]);
  });

  it("renders all AudioEngine event kinds and restarts changed background music", () => {
    const played: Array<{
      token: string;
      kind: string;
      loop: boolean;
      source: string;
    }> = [];
    const killed: string[] = [];
    const controller = createTuiSoundController({
      enabled: true,
      pcmResolver: () => ({ pcm: new Int16Array([1, 1, -1, -1]), sampleRate: 44_100 }),
      player: (input) => {
        played.push(input);
        return { kill: () => killed.push(input.token) };
      },
    });
    const nextMusicSource = `${__dirname}/tui.ts`;

    controller.syncSnapshot({
      musicToken: "MUSIC_ROUTE_29",
      musicSource: __filename,
      musicRole: "map",
      recentEvents: [
        {
          sequence: 1,
          kind: "music",
          token: "MUSIC_ROUTE_29",
          source: __filename,
          role: "map",
          loop: true,
        },
        { sequence: 2, kind: "sfx", token: "SFX_MENU", source: __filename },
        {
          sequence: 3,
          kind: "cry",
          token: "CRY_CHIKORITA",
          source: __filename,
        },
        {
          sequence: 4,
          kind: "other",
          token: "CUSTOM_AUDIO",
          source: __filename,
        },
      ],
    });

    expect(
      played.map((event) => [event.token, event.kind, event.loop]),
    ).toEqual([
      ["MUSIC_ROUTE_29", "music", true],
      ["SFX_MENU", "sfx", false],
      ["CRY_CHIKORITA", "cry", false],
      ["CUSTOM_AUDIO", "other", false],
    ]);

    controller.syncSnapshot({
      musicToken: "MUSIC_ROUTE_29",
      musicSource: nextMusicSource,
      musicRole: "map",
      recentEvents: [],
    });

    expect(killed).toEqual(["MUSIC_ROUTE_29"]);
    expect(played.at(-1)).toEqual(
      expect.objectContaining({
        token: "MUSIC_ROUTE_29",
        kind: "music",
        loop: true,
        source: nextMusicSource,
      }),
    );
  });
});

describe("Kitty Game Boy image rendering", () => {
  it("extracts image/png frames from tool results and ignores non-images", () => {
    const frame = extractKittyPngFrame({
      content: [
        { type: "text", text: "OVERWORLD" },
        {
          type: "image",
          mimeType: "image/jpeg",
          data: createFakePngBase64(1, 1),
        },
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64(160, 144),
        },
      ],
    });

    expect(frame).toMatchObject({
      mimeType: "image/png",
      width: 160,
      height: 144,
    });
    expect(
      extractKittyPngFrame({ content: [{ type: "text", text: "no image" }] }),
    ).toBeNull();
    expect(
      extractKittyPngFrame({
        content: [{ type: "image", mimeType: "image/png", data: "not-png" }],
      }),
    ).toBeNull();
  });

  it("detects Ghostty and supports an explicit Kitty override", () => {
    expect(isKittyGraphicsSupported({ TERM_PROGRAM: "Ghostty" })).toBe(true);
    expect(isKittyGraphicsSupported({ TERM: "xterm-ghostty" })).toBe(true);
    expect(isKittyGraphicsSupported({ POKECRYSTAL_CLI_KITTY: "1" })).toBe(true);
    expect(
      isKittyGraphicsSupported({
        POKECRYSTAL_CLI_KITTY: "0",
        TERM_PROGRAM: "Ghostty",
      }),
    ).toBe(false);
    expect(isKittyGraphicsSupported({ TERM: "xterm-256color" })).toBe(false);
    expect(
      isKittyPlaceholderModeEnabled(
        { POKECRYSTAL_CLI_KITTY_PLACEHOLDERS: "0" },
        true,
      ),
    ).toBe(false);
    expect(
      isKittyPlaceholderModeEnabled(
        { POKECRYSTAL_CLI_KITTY_PLACEHOLDERS: "1" },
        false,
      ),
    ).toBe(true);
    expect(isKittyPlaceholderModeEnabled({}, true)).toBe(true);
  });

  it("emits split Kitty upload, virtual placement, and fallback placement sequences", () => {
    const frame = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64(160, 144),
        },
      ],
    })!;
    const upload = buildKittyUploadSequence(frame, 7);
    const virtualPlacement = buildKittyVirtualPlacementSequence(
      { row: 10, column: 3, columns: 40, rows: 18 },
      7,
    );
    const sequence = buildKittyFrameSequence(
      frame,
      { row: 10, column: 3, columns: 40, rows: 18 },
      7,
    );

    expect(sequence).not.toContain(buildKittyDeleteSequence(7));
    expect(upload).toContain("\u001b_Ga=t,f=100,i=7,q=2,m=0;");
    expect(upload).toContain(frame.data);
    expect(upload).not.toContain("c=40");
    expect(virtualPlacement).toContain(
      "\u001b_Ga=p,U=1,i=7,p=1,q=2,c=40,r=18;",
    );
    expect(sequence).toContain("\u001b[10;3H");
    expect(sequence).toContain(
      "\u001b_Ga=T,f=100,i=7,p=1,q=2,c=40,r=18,C=1,m=0;",
    );
    expect(sequence).toContain(frame.data);
  });

  it("builds Kitty placeholder lines with stable cell width and image-id colour", () => {
    const lines = buildKittyPlaceholderLines(5, 3);

    expect(lines).toHaveLength(3);
    expect(lines.map(countKittyPlaceholderCells)).toEqual([5, 5, 5]);
    expect(lines[0]).toBe(
      `${KITTY_PLACEHOLDER_CELL}\u0305${KITTY_PLACEHOLDER_CELL.repeat(4)}`,
    );
    expect(lines[1]).toBe(
      `${KITTY_PLACEHOLDER_CELL}\u030d${KITTY_PLACEHOLDER_CELL.repeat(4)}`,
    );
    expect(lines[2]).toBe(
      `${KITTY_PLACEHOLDER_CELL}\u030e${KITTY_PLACEHOLDER_CELL.repeat(4)}`,
    );
    expect(buildKittyPlaceholderColor(0x123456)).toBe("#123456");
  });

  it("reserves stable Kitty image ids per TUI session", () => {
    expect(resolveKittyImageId("session-a")).toBe(
      resolveKittyImageId("session-a"),
    );
    expect(resolveKittyImageId("session-a")).not.toBe(
      resolveKittyImageId("session-b"),
    );
    expect(resolveKittyImageIds("session-a")).toEqual([
      resolveKittyImageId("session-a"),
      resolveKittyImageId("session-a") + 1,
    ]);
  });

  it("no-ops safely when Kitty graphics are unsupported", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: false, imageId: 8 },
    );
    const frame = extractKittyPngFrame({
      content: [
        { type: "image", mimeType: "image/png", data: createFakePngBase64() },
      ],
    })!;

    renderer.update(frame, { row: 1, column: 1, columns: 20, rows: 10 });
    renderer.clear();

    expect(chunks).toEqual([]);
  });

  it("clears a visible Kitty image when the frame disappears", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: true, imageId: 9 },
    );
    const frame = extractKittyPngFrame({
      content: [
        { type: "image", mimeType: "image/png", data: createFakePngBase64() },
      ],
    })!;

    renderer.update(frame, { row: 1, column: 1, columns: 20, rows: 10 });
    renderer.update(null, null);
    renderer.commit();

    expect(chunks[0]).toContain("\u001b_Ga=t");
    expect(chunks[1]).toContain("\u001b_Ga=p,U=1");
    expect(chunks.at(-1)).toBe(buildKittyDeleteSequence(9));
  });

  it("redraws the last Kitty placement without retransmitting image data", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: true, imageId: 10 },
    );
    const frame = extractKittyPngFrame({
      content: [
        { type: "image", mimeType: "image/png", data: createFakePngBase64() },
      ],
    })!;

    renderer.update(frame, { row: 2, column: 3, columns: 20, rows: 10 });
    renderer.redraw();

    expect(chunks).toHaveLength(3);
    expect(chunks[0]).toContain("\u001b_Ga=t");
    expect(chunks[1]).toContain("\u001b_Ga=p,U=1");
    expect(chunks[2]).toContain("\u001b_Ga=p,U=1");
    expect(chunks[2]).not.toContain(frame.data);
  });

  it("does not emit Kitty traffic for an unchanged frame during agent-only updates", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: true, imageId: 11 },
    );
    const frame = extractKittyPngFrame({
      content: [
        { type: "image", mimeType: "image/png", data: createFakePngBase64() },
      ],
    })!;
    const placement = { row: 2, column: 3, columns: 20, rows: 10 };

    renderer.update(frame, placement);
    chunks.length = 0;
    const display = renderer.update(frame, placement);
    renderer.commit();

    expect(display).toMatchObject({ imageId: 11, columns: 20, rows: 10 });
    expect(chunks).toEqual([]);
  });

  it("replaces virtual placement dimensions without retransmitting the unchanged image", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: true, imageId: 12 },
    );
    const frame = extractKittyPngFrame({
      content: [
        { type: "image", mimeType: "image/png", data: createFakePngBase64() },
      ],
    })!;

    renderer.update(frame, { row: 2, column: 3, columns: 20, rows: 10 });
    chunks.length = 0;
    const display = renderer.update(frame, {
      row: 2,
      column: 3,
      columns: 24,
      rows: 12,
    });

    expect(display).toMatchObject({ imageId: 12, columns: 24, rows: 12 });
    expect(chunks).toHaveLength(1);
    expect(chunks[0]).toContain("\u001b_Ga=p,U=1,i=12,p=1,q=2,c=24,r=12;");
    expect(chunks[0]).not.toContain(frame.data);
  });

  it("updates same-size same-length PNG frames by comparing data, not length", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: true, imageId: 13 },
    );
    const first = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64Variant(1),
        },
      ],
    })!;
    const second = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64Variant(2),
        },
      ],
    })!;

    expect(first.data).toHaveLength(second.data.length);
    renderer.update(first, { row: 2, column: 3, columns: 20, rows: 10 });
    chunks.length = 0;
    const display = renderer.update(second, {
      row: 2,
      column: 3,
      columns: 20,
      rows: 10,
    });

    expect(display).toMatchObject({ imageId: 14 });
    expect(chunks.join("")).toContain("\u001b_Ga=t");
    expect(chunks.join("")).toContain(second.data);
  });

  it("scopes cleanup to the renderer's own double-buffered Kitty image ids", () => {
    const chunks: string[] = [];
    const renderer = createKittyImageRenderer(
      { write: (chunk) => chunks.push(chunk) },
      { supported: true, imageId: 15 },
    );
    const first = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64Variant(1),
        },
      ],
    })!;
    const second = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64Variant(2),
        },
      ],
    })!;

    renderer.update(first, { row: 2, column: 3, columns: 20, rows: 10 });
    renderer.update(second, { row: 2, column: 3, columns: 20, rows: 10 });
    renderer.commit();
    renderer.clear();

    expect(chunks).toContain(buildKittyDeleteSequence(15));
    expect(chunks).toContain(buildKittyDeleteSequence(16));
    expect(chunks.join("")).not.toContain("\u001b_Ga=d,q=2;");
  });
});

describe("buildAgentInterruptOptions", () => {
  it("resumes the linked agent with Professor Culligan's Advice in the goal", () => {
    const options = buildAgentInterruptOptions(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "agent-session",
        agent: true,
        agentCommand: "run",
        agentGoal: "Beat Mt. Silver",
      },
      "go heal before Route 30",
    );

    expect(options.agentCommand).toBe("resume");
    expect(options.agentGoal).toContain("Beat Mt. Silver");
    expect(options.agentGoal).toContain(
      "Professor Culligan's Advice: go heal before Route 30",
    );
    expect(options.agentGoal).toContain("highest-priority professor guidance");
    expect(options.sessionId).toBe("agent-session");
  });

  it("resumes the linked agent with Professor Culligan's manual intervention in the goal", () => {
    const options = buildAgentManualInterventionOptions(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "agent-session",
        agent: true,
        agentCommand: "run",
        agentGoal: "Beat Mt. Silver",
      },
      [
        "Professor Culligan intervened for 4.8s and made 2 manual inputs.",
        "Manual inputs already applied:",
        '1. move up (key "w") -> ok; mode=overworld map=Route 28 coords=10,4',
      ].join("\n"),
    );

    expect(options.agentCommand).toBe("resume");
    expect(options.agentGoal).toContain("Beat Mt. Silver");
    expect(options.agentGoal).toContain("Professor Culligan's Intervention");
    expect(options.agentGoal).toContain(
      "manual inputs below have already happened",
    );
    expect(options.agentGoal).toContain("move up");
    expect(options.agentGoal?.toLowerCase()).toContain("do not repeat");
    expect(options.sessionId).toBe("agent-session");
  });
});

describe("normalizeTuiSnapshot", () => {
  it("prefers structured observe JSON for Ink panels", () => {
    const snapshot = normalizeTuiSnapshot(
      {
        content: [
          { type: "text", text: "OVERWORLD\nfallback" },
          {
            type: "text",
            mimeType: "application/json",
            text: JSON.stringify({
              ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
              view: {
                focus: "menu",
                viewport: ["00 . @ ."],
                menu: { items: ["> POKEMON", "BAG"] },
                prompt: ["Choose one"],
                dialogue: ["ELM: Take one."],
              },
            }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN" }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({ recap: "latest move", events: [] }),
          },
        ],
      },
    );

    expect(snapshot.viewport).toEqual(["00 . @ ."]);
    expect(snapshot.menu).toEqual(["> POKEMON", "BAG"]);
    expect(snapshot.prompt).toEqual(["Choose one"]);
    expect(snapshot.dialogue).toEqual(["ELM: Take one."]);
    expect(snapshot.statusLine).toContain("MAP: NEW_BARK_TOWN");
  });

  it("keeps PC list instructions visible without marking them blocking", () => {
    const snapshot = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              ctx: { m: "pc", map: "BILL'S PC", xy: [19, 5] },
              surface: { kind: "pc", title: "Bill's PC" },
              view: {
                viewport: ["BOX 1", "TOTODILE", "GEODUDE"],
                prompt: ["Choose a <PK><MN>."],
              },
            }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              mode: "pc",
              map: "BILL'S PC",
              surface: { kind: "pc", title: "Bill's PC" },
            }),
          },
        ],
      },
    );

    expect(snapshot.prompt).toEqual(["Choose a <PK><MN>."]);
    expect(snapshot.promptStatus).toBe("idle");
    expect(snapshot.statusLine).toContain("PROMPT: idle");
  });

  it("marks PC menu prompts with cursors as blocking prompts", () => {
    const snapshot = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              ctx: { m: "pc", map: "BILL'S PC", xy: [19, 5] },
              surface: { kind: "pc", title: "Bill's PC" },
              view: {
                viewport: ["BOX 1"],
                prompt: ["> YES", "  NO"],
              },
            }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              mode: "pc",
              map: "BILL'S PC",
              surface: { kind: "pc", title: "Bill's PC" },
            }),
          },
        ],
      },
    );

    expect(snapshot.promptStatus).toBe("prompt");
    expect(snapshot.statusLine).toContain("PROMPT: prompt");
  });

  it("renders Bill's PC menus as PC snapshots when the renderer reports a generic Prompt surface", () => {
    const snapshot = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              ctx: { m: "menu", map: "EcruteakPokecenter1F", xy: [19, 5] },
              surface: {
                kind: "prompt",
                title: "Prompt",
                menu_open: true,
                selected: "WITHDRAW <PK><MN>",
                primary_text: "▶ WITHDRAW <PK><MN>",
              },
              view: {
                viewport: ["Prompt"],
                info: ["D-Pad=Move A=Select B=Back"],
                menu: [
                  "▶ WITHDRAW <PK><MN>",
                  "  DEPOSIT <PK><MN>",
                  "  CHANGE BOX",
                  "  MOVE <PK><MN> W/O MAIL",
                  "  SEE YA!",
                ],
              },
            }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              mode: "menu",
              map: "EcruteakPokecenter1F",
              in_menu: true,
              surface: {
                kind: "prompt",
                title: "Prompt",
                menu_open: true,
                selected: "WITHDRAW <PK><MN>",
              },
            }),
          },
        ],
      },
    );

    expect(snapshot.mode).toBe("pc");
    expect(snapshot.map).toBe("BILL'S PC");
    expect(snapshot.surface).toBe("pc");
    expect(snapshot.viewport).toEqual(["BILL'S PC"]);
    expect(snapshot.menu).toEqual([
      "▶ WITHDRAW <PK><MN>",
      "  DEPOSIT <PK><MN>",
      "  CHANGE BOX",
      "  MOVE <PK><MN> W/O MAIL",
      "  SEE YA!",
    ]);
    expect(snapshot.statusLine).toContain("STATE: pc");
  });

  it("renders hidden engine-owned yes/no prompts from prompt-pending status", () => {
    const snapshot = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              ctx: { m: "overworld", map: "CianwoodGym", xy: [11, 17] },
              view: {
                viewport: ["@^"],
                dialogue: [
                  "A POKéMON may be",
                  "able to move this.",
                  "Want to use",
                  "STRENGTH?",
                ],
              },
            }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              mode: "overworld",
              map: "CianwoodGym",
              prompt_pending: true,
              prompt: { pending: true, reason: "prompt" },
            }),
          },
        ],
      },
    );

    expect(snapshot.prompt).toEqual(["▶ YES", "  NO"]);
    expect(snapshot.promptStatus).toBe("prompt");
    expect(snapshot.dialogue).toContain("STRENGTH?");
  });

  it("labels title-screen snapshots from visible viewport text instead of stale map ctx", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        { type: "text", text: "TITLE\nPOKEMON CRYSTAL\nTITLE SCREEN" },
        {
          type: "text",
          text: JSON.stringify({
            ctx: { m: "overworld", map: "PlayersHouse2F", xy: [3, 3] },
            view: {
              viewport: ["POKEMON CRYSTAL", "TITLE SCREEN"],
              info: ["STATE: entrance", "WAIT: title entrance"],
            },
          }),
        },
      ],
    });

    expect(snapshot.mode).toBe("title");
    expect(snapshot.map).toBe("TITLE");
    expect(snapshot.statusLine).toBe(
      "STATE: title | MAP: TITLE | PROMPT: idle",
    );
  });

  it("labels non-overworld snapshots from structured surface status", () => {
    const snapshot = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              view: {
                viewport: ["OAK INTRO", "SPRITE: OAK"],
                info: ["STATE: oak_intro", "PHASE: text"],
                dialogue: ["Hello!"],
              },
            }),
          },
        ],
      },
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              mode: "overworld",
              map: "PlayersHouse2F",
              surface: { kind: "oak_intro", title: "Oak Intro", phase: "text" },
            }),
          },
        ],
      },
    );

    expect(snapshot.mode).toBe("oak_intro");
    expect(snapshot.map).toBe("OAK INTRO");
    expect(snapshot.surface).toBe("oak_intro");
    expect(snapshot.statusLine).toContain("STATE: oak_intro");
  });

  it("accumulates dialogue pages and clears when dialogue closes", () => {
    const accumulator = createDialogueAccumulator();
    const first = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({ view: { dialogue: ["Page one"] } }),
          },
        ],
      },
      undefined,
      undefined,
      accumulator,
    );
    const second = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({ view: { dialogue: ["Page two"] } }),
          },
        ],
      },
      undefined,
      undefined,
      accumulator,
    );
    const closed = normalizeTuiSnapshot(
      {
        content: [
          {
            type: "text",
            text: JSON.stringify({ view: { viewport: ["OVERWORLD"] } }),
          },
        ],
      },
      undefined,
      undefined,
      accumulator,
    );

    expect(first.dialogue).toEqual(["Page one"]);
    expect(second.dialogue).toEqual(["Page one", "", "Page two"]);
    expect(closed.dialogue).toEqual([]);
  });

  it("filters internal dialogue status lines from the TUI dialogue panel", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              dialogue: [
                "Although you can't",
                "see it from here,",
                "CIANWOOD is across",
                "the sea.",
                "Text queue: 0 (press A to advance)",
              ],
            },
          }),
        },
      ],
    });

    expect(snapshot.dialogue).toEqual([
      "Although you can't",
      "see it from here,",
      "CIANWOOD is across",
      "the sea.",
    ]);
  });

  it("expands name entry snapshots into a complete keyboard surface", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: [
                "STATE: name_entry",
                "PROMPT: Your name?",
                "NAME: CH",
                "LENGTH: 2/7",
                "CURSOR: row 0 col 2",
              ],
              menu: [
                "A B C D E F G H I",
                "J K L M N O P Q R",
                "S T U V W X Y Z  ",
                "- ? ! / . ,      ",
                "lower  DEL   END ",
              ],
            },
          }),
        },
      ],
    });

    expect(snapshot.viewport).toEqual(
      expect.arrayContaining([
        "NAME ENTRY",
        "YOUR NAME?",
        "NAME    C H _ _ _ _ _",
        "        ^ ^ ^ ^ ^ ^ ^  2/7",
        "TYPE    letters enter directly; Backspace deletes; End confirms",
        "KEYS    arrows move cursor; Space selects",
        "KEYBOARD",
        " A   B  [C]  D   E   F   G   H   I",
        " J   K   L   M   N   O   P   Q   R",
        " S   T   U   V   W   X   Y   Z",
        " -   ?   !   /   .   ,",
        " lower   DEL   END",
        "SELECTED C",
      ]),
    );
  });

  it("rebuilds name-entry cursor markers from cursor metadata", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: ["NAME ENTRY"],
              info: [
                "STATE: name_entry",
                "PROMPT: Your name?",
                "NAME: DO",
                "LENGTH: 2/7",
                "CURSOR: row 1 col 7",
              ],
              menu: [
                "A B C D E F G H I",
                "J K L M N O P Q R",
                "▲",
                "S T U V W X Y Z",
                "- ? ! / . ,",
                "lower DEL END",
              ],
            },
          }),
        },
      ],
    });

    const keyboardStart = snapshot.viewport.indexOf("KEYBOARD") + 1;
    expect(snapshot.viewport.slice(keyboardStart, keyboardStart + 6)).toEqual([
      " A   B   C   D   E   F   G   H   I",
      " J   K   L   M   N   O   P  [Q]  R",
      " S   T   U   V   W   X   Y   Z",
      " -   ?   !   /   .   ,",
      " lower   DEL   END",
      "SELECTED Q",
    ]);
  });

  it("does not fabricate map rows when an overworld snapshot omits the player row", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: [
                "   03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22",
                "00 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "01 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "02 N> #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "03 S  #  #  D  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "04 .  .  .  .  .  .  .  .  #  #  #  #  #  #  #  #  #  #  #  # ",
                "05 .  .  .  .  .  .  .  S  #  D  #  #  #  #  #  #  #  #  #  # ",
                "06 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
                "07 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
                "08 .  .  .  Nv .  S  .  .  .  N^ .  .  .  .  ~  ~  ~  ~  ~  ~ ",
                "09 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
                "10 #  #  #  #  #  #  .  .  .  .  .  .  H  #  #  #  #  #  #  # ",
              ],
              info: [
                "D-Pad=Move A=Talk Start=Menu Select=Item B=Back",
                "Pos: (13,11)",
              ],
            },
            ctx: { m: "overworld", map: "NewBarkTown", xy: [27, 23], pr: 0 },
          }),
        },
      ],
    });

    expect(snapshot.viewport.join("\n")).not.toContain("@");
    expect(snapshot.viewport.slice(1).map((line) => line.slice(0, 2))).toEqual([
      "00",
      "01",
      "02",
      "03",
      "04",
      "05",
      "06",
      "07",
      "08",
      "09",
      "10",
    ]);
  });

  it("leaves real overworld rows intact instead of replacing them with synthetic dots", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: [
                "   00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19",
                "00 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "01 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "02 #  #  .  N> #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "03 S  #  #  D  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
                "04 .  .  .  .  .  .  .  .  .  .  .  .  #  #  #  #  #  #  #  # ",
                "05 .  .  .  .  .  .  .  S  .  .  .  D  #  #  #  #  #  #  #  # ",
                "06 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
                "07 #  #  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
                "08 .  .  .  .  .  .  Nv .  S  .  .  .  N^ .  .  .  .  .  ~  ~ ",
                "09 .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~ ",
                "10 #  #  #  #  #  #  .  .  .  .  .  .  .  .  .  .  H  #  #  # ",
              ],
              info: ["Pos: (7,15)"],
            },
            ctx: { m: "overworld", map: "NewBarkTown", xy: [15, 31], pr: 0 },
          }),
        },
      ],
    });

    expect(snapshot.viewport.slice(1).map((line) => line.slice(0, 2))).toEqual([
      "00",
      "01",
      "02",
      "03",
      "04",
      "05",
      "06",
      "07",
      "08",
      "09",
      "10",
    ]);
    expect(snapshot.viewport.join("\n")).toContain("N>");
    expect(snapshot.viewport.join("\n")).toContain("#");
    expect(snapshot.viewport.join("\n")).not.toContain("@");
  });

  it("keeps the full Game Boy overworld viewport during normalization", () => {
    const snapshot = normalizeTuiSnapshot({
      content: [
        {
          type: "text",
          text: JSON.stringify({
            view: {
              viewport: [
                "   00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19",
                "00 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
                "01 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
                "02 #  #  .  N> #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
                "03 #  #  .  S  #  #  D  #  #  #  #  #  #  #  #  #  #  #  #  #",
                "04 #  #  .  .  .  .  .  .  .  .  .  .  #  #  #  #  #  #  #  #",
                "05 #  #  .  .  .  .  .  .  .  .  .  S  #  D  #  #  #  #  #  #",
                "06 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~",
                "07 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~",
                "08 .  .  .  .  .  .  Nv .  S  .  .  .  .  .  .  .  .  .  ~  ~",
                "09 .  .  .  .  .  .  .  .  .  .  .  .  Nv .  .  .  .  .  ~  ~",
                "10 #  #  #  #  #  #  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
                "11 #  #  #  D  #  #  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
                "12 .  .  .  .  .  .  .  .  .  .  #  #  #  #  .  .  .  H  #  #",
                "13 .  .  .  .  .  .  .  .  .  S  #  D  #  #  .  .  .  H  #  #",
                "14 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
                "15 #  #  H  H  H  H  .  @v .  .  .  .  .  .  .  .  .  H  #  #",
                "16 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
                "17 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
              ],
              info: ["Pos: (7,15)"],
            },
            ctx: { m: "overworld", map: "NewBarkTown", xy: [15, 31], pr: 0 },
          }),
        },
      ],
    });

    const rowLabels = snapshot.viewport
      .slice(1)
      .map((line) => line.slice(0, 2));
    expect(rowLabels).toEqual([
      "00",
      "01",
      "02",
      "03",
      "04",
      "05",
      "06",
      "07",
      "08",
      "09",
      "10",
      "11",
      "12",
      "13",
      "14",
      "15",
      "16",
      "17",
    ]);
    expect(snapshot.viewport.join("\n")).toContain("@v");
  });
});

describe("resolveTuiLayout", () => {
  it("wraps control text to the terminal content width", () => {
    for (const columns of [64, 90, 100, 140]) {
      const lines = resolveControlLines(columns, columns < 96);
      expect(lines.length).toBeGreaterThanOrEqual(2);
      expect(lines.every((line) => line.length <= columns - 4)).toBe(true);
      expect(lines.join(" ")).toContain("arrows/WASD/HJKL=d-pad");
    }
  });

  it("uses the full three-column layout on wide terminals", () => {
    const layout = resolveTuiLayout({ columns: 140, rows: 44 });

    expect(layout.wide).toBe(true);
    expect(layout.medium).toBe(false);
    expect(layout.narrow).toBe(false);
    expect(layout.diagnosticsVisible).toBe(true);
    expect(layout.compactControls).toBe(false);
    expect(layout.rows).toBe(44);
    expect(layout.mainHeight).toBe(36);
    expect(layout.gameLines).toBe(33);
    expect(layout.infoLines).toBeGreaterThanOrEqual(3);
    expect(layout.footerHeight).toBe(0);
  });

  it("moves diagnostics below the Game Boy surface on medium terminals", () => {
    const layout = resolveTuiLayout({ columns: 90, rows: 30 });

    expect(layout.narrow).toBe(false);
    expect(layout.medium).toBe(true);
    expect(layout.wide).toBe(false);
    expect(layout.diagnosticsVisible).toBe(true);
    expect(layout.compactControls).toBe(true);
    expect(layout.mainHeight).toBe(22);
    expect(layout.gameHeight).toBe(16);
    expect(layout.gameLines).toBe(13);
    expect(layout.footerHeight).toBe(0);
  });

  it("hides diagnostics before clipping the playable surface on narrow terminals", () => {
    const layout = resolveTuiLayout({ columns: 64, rows: 24 });

    expect(layout.narrow).toBe(true);
    expect(layout.medium).toBe(false);
    expect(layout.wide).toBe(false);
    expect(layout.diagnosticsVisible).toBe(false);
    expect(layout.compactControls).toBe(true);
    expect(layout.mainHeight).toBe(16);
    expect(layout.gameLines).toBe(13);
    expect(layout.footerHeight).toBe(0);
  });

  it("reserves footer space only after controls are requested", () => {
    const layout = resolveTuiLayout({
      columns: 90,
      rows: 30,
      controlsVisible: true,
    });

    expect(layout.footerHeight).toBe(layout.controlLines.length + 2);
    expect(layout.mainHeight).toBe(17);
  });
});

describe("resolveGameBoyImageCellSize", () => {
  it("keeps Kitty image placement at the Game Boy terminal-cell aspect instead of filling the whole panel", () => {
    expect(resolveGameBoyImageCellSize(88, 33)).toEqual({ columns: 73, rows: 33 });
    expect(resolveGameBoyImageCellSize(136, 33)).toEqual({ columns: 73, rows: 33 });
  });

  it("fits the Game Boy image inside narrow panels without changing the anchor", () => {
    expect(resolveGameBoyImageCellSize(40, 33)).toEqual({ columns: 40, rows: 18 });
  });
});

describe("createInkTuiApp responsive layout", () => {
  it("renders Kitty placeholder cells inside the Game Boy panel when display state is prepared", () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const state = createTuiViewState();
    const frame = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64(160, 144),
        },
      ],
    })!;
    state.gameboyRenderer = "kitty";
    state.gameboyImage = frame;
    state.kittyImageDisplay = {
      mode: "placeholder",
      imageId: 0x123456,
      placementId: 1,
      columns: 5,
      rows: 3,
      color: buildKittyPlaceholderColor(0x123456),
    };

    try {
      const tree = renderFakeInkTree(140, 44, state);
      const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
      const lines = panelTextLines(gamePanel);

      expect(lines.slice(0, 3).map(countKittyPlaceholderCells)).toEqual([
        5, 5, 5,
      ]);
      expect((gamePanel.children[1] as FakeInkNode).props?.color).toBe(
        "#123456",
      );
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("renders dialogue in a side panel when the Kitty image panel is active", () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const state = createTuiViewState();
    const frame = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64(160, 144),
        },
      ],
    })!;
    state.snapshot = {
      ...state.snapshot,
      viewport: ["BATTLE"],
      menu: [],
      prompt: [],
      dialogue: ["What will", "LUGIA do?"],
      actions: [],
    };
    state.gameboyRenderer = "kitty";
    state.gameboyImage = frame;
    state.kittyImageDisplay = {
      mode: "placeholder",
      imageId: 0x123457,
      placementId: 1,
      columns: 5,
      rows: 3,
      color: buildKittyPlaceholderColor(0x123457),
    };

    try {
      const tree = renderFakeInkTree(140, 44, state);
      const dialoguePanel = panelByTitle(tree, "DIALOGUE") as FakeInkNode;
      const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;

      expect(dialoguePanel).toBeDefined();
      expect(panelTextLines(dialoguePanel)).toEqual(["What will", "LUGIA do?"]);
      expect(panelTextLines(gamePanel).join("\n")).not.toContain("LUGIA");
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("renders Kitty placeholder cells inside the full agent Game Boy panel", () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const state = createTuiViewState();
    const frame = extractKittyPngFrame({
      content: [
        {
          type: "image",
          mimeType: "image/png",
          data: createFakePngBase64(160, 144),
        },
      ],
    })!;
    state.activeView = "agent";
    state.agentStream = emptyAgentStreamState();
    state.gameboyRenderer = "kitty";
    state.gameboyImage = frame;
    state.kittyImageDisplay = {
      mode: "placeholder",
      imageId: 0x456789,
      placementId: 1,
      columns: 4,
      rows: 3,
      color: buildKittyPlaceholderColor(0x456789),
    };

    try {
      const tree = renderFakeInkTree(180, 48, state);
      const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
      const lines = panelTextLines(gamePanel);

      expect(lines.slice(0, 3).map(countKittyPlaceholderCells)).toEqual([
        4, 4, 4,
      ]);
      expect(panelByTitle(tree, "AGENT OUTPUT")).toBeDefined();
      expect((gamePanel.children[1] as FakeInkNode).props?.color).toBe(
        "#456789",
      );
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("renders the wide terminal with a dominant Game Boy panel and side diagnostics", () => {
    const tree = renderFakeInkTree(140, 44);
    const mainContent = tree.children[1] as FakeInkNode;
    const gamePanel = panelByTitle(tree, "GAME BOY");
    const menuPanel = panelByTitle(tree, "MENU");

    expect(tree.props?.width).toBe(140);
    expect(tree.props?.height).toBe(44);
    expect(tree.props?.overflow).toBe("hidden");
    expect(
      findText(tree, (text) => text.startsWith("MCP: http://127.0.0.1")),
    ).toBeUndefined();
    expect(mainContent.props?.flexDirection).toBe("row");
    expect(mainContent.props?.height).toBe(36);
    expect(mainContent.props?.overflow).toBe("hidden");
    expect(gamePanel?.props?.width).toBe("66%");
    expect(gamePanel?.props?.height).toBe(36);
    expect(menuPanel).toBeDefined();
    expect(panelTextLines(gamePanel as FakeInkNode)).toHaveLength(33);
    expect(collectText(tree, (text) => text.startsWith("Controls:"))).toEqual(
      [],
    );
  });

  it("marks diagnostic panels when terminal height hides scrollable menu rows", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      menu: createRangeLines("menu", 12),
      prompt: ["Choose"],
      dialogue: [],
    };
    state.agentStream = emptyAgentStreamState();

    const tree = renderFakeInkTree(140, 44, state);
    const menuPanel = panelByTitle(tree, "MENU") as FakeInkNode;

    expect(panelTextLines(menuPanel).at(-1)).toBe("▼ 8 more rows hidden");
  });

  it("splits linked-agent status, tokens, and MCP into bounded play panels", () => {
    const state = createTuiViewState();
    state.agentStream = {
      status: "batch 1 running",
      thinking: "Need to get through the title screen.",
      text: "Press START, then confirm the intro.",
      mcpCalls: ['press {"button":"a"}'],
      events: [],
    };
    state.snapshot = {
      ...state.snapshot,
      menu: [],
      prompt: [],
      dialogue: [],
    };
    const tree = renderFakeInkTree(160, 48, state);
    const mainContent = tree.children[1] as FakeInkNode;
    const diagnosticsColumn = mainContent.children[1] as FakeInkNode;
    const currentPanel = panelByTitle(tree, "CURRENT");
    const tokensPanel = panelByTitle(tree, "TOKENS");
    const mcpPanel = panelByTitle(tree, "MCP");
    const layout = resolveTuiLayout({ columns: 160, rows: 48 });

    expect(currentPanel).toBeDefined();
    expect(tokensPanel).toBeDefined();
    expect(mcpPanel).toBeDefined();
    expect(
      findText(tree, (text) => text.startsWith("MCP: http://127.0.0.1")),
    ).toBeDefined();
    expect(
      diagnosticsColumn.children.reduce<number>(
        (total, child) =>
          total + Number((child as FakeInkNode).props?.height ?? 0),
        0,
      ),
    ).toBe(layout.mainHeight);
    expect(panelTextLines(currentPanel as FakeInkNode).join("\n")).toContain(
      "STATUS: batch 1 running",
    );
    expect(panelTextLines(currentPanel as FakeInkNode).join("\n")).toContain(
      "THINKING: Need to get through the title screen.",
    );
    expect(panelTextLines(tokensPanel as FakeInkNode).join("\n")).toContain(
      "TOKENS: Press START, then confirm the intro.",
    );
    expect(panelTextLines(mcpPanel as FakeInkNode).join("\n")).toContain(
      'MCP: press {"button":"a"}',
    );
  });

  it("falls back to a full-width Game Boy panel when agent view is requested without an agent stream", () => {
    const state = createTuiViewState();
    state.activeView = "agent";
    state.snapshot = {
      ...state.snapshot,
      menu: [],
      prompt: [],
      dialogue: [],
    };
    const tree = renderFakeInkTree(180, 48, state);
    const gamePanel = panelByTitle(tree, "GAME BOY");

    expect(gamePanel?.props?.width).toBe("100%");
    expect(panelByTitle(tree, "AGENT OUTPUT")).toBeUndefined();
    expect(panelByTitle(tree, "MCP CALLS")).toBeUndefined();
    expect(
      findText(tree, (text) => text.startsWith("MCP: http://127.0.0.1")),
    ).toBeUndefined();
  });

  it("keeps mixed play diagnostics within the side column", () => {
    const state = createTuiViewState();
    state.agentStream = {
      status: "batch 1 running",
      thinking: "Pick the next movement after checking the prompt.",
      text: "Walk toward Elm and avoid wasting frames.",
      mcpCalls: [
        'observe {"detail":"full"}',
        'move {"direction":"up","steps":1}',
      ],
      events: [],
    };
    const tree = renderFakeInkTree(160, 48, state);
    const mainContent = tree.children[1] as FakeInkNode;
    const diagnosticsColumn = mainContent.children[1] as FakeInkNode;
    const layout = resolveTuiLayout({ columns: 160, rows: 48 });

    expect(diagnosticsColumn.children).toHaveLength(5);
    expect(
      diagnosticsColumn.children.reduce<number>(
        (total, child) =>
          total + Number((child as FakeInkNode).props?.height ?? 0),
        0,
      ),
    ).toBe(layout.mainHeight);
  });

  it("renders agent details as map, agent output, and MCP columns", () => {
    const state = createTuiViewState();
    state.activeView = "agent";
    state.agentStream = {
      status: "batch 1 running",
      thinking: "Need to leave the room and reach Elm.",
      text: "I will go downstairs, leave the house, and walk to the lab.",
      mcpCalls: ["move {direction:down, steps:1}"],
      events: [
        {
          type: "thinking-delta",
          label: "taskmaster",
          text: "Need to leave the room and reach Elm.",
        },
        {
          type: "text-delta",
          label: "player",
          text: "I will go downstairs, leave the house, and walk to the lab.",
        },
        { type: "tool-call", label: "tool", text: "krabbyclaw_move" },
        {
          type: "mcp-call",
          label: "mcp",
          text: "move {direction:down, steps:1}",
        },
      ],
    };
    const tree = renderFakeInkTree(180, 48, state);
    const mainContent = tree.children[1] as FakeInkNode;
    const gamePanel = panelByTitle(tree, "GAME BOY");
    const outputPanel = panelByTitle(tree, "AGENT OUTPUT");
    const mcpPanel = panelByTitle(tree, "MCP CALLS");

    expect(mainContent.props?.flexDirection).toBe("row");
    expect(gamePanel?.props?.width).toBe("32%");
    expect(outputPanel?.props?.width).toBe("44%");
    expect(mcpPanel?.props?.width).toBe("24%");
    expect(panelByTitle(tree, "AGENT REASONING")).toBeUndefined();
    expect(panelTextLines(outputPanel as FakeInkNode).join("\n")).toContain(
      "go downstairs",
    );
    expect(panelTextLines(outputPanel as FakeInkNode).join("\n")).toContain(
      "Need to leave",
    );
    expect(panelTextLines(outputPanel as FakeInkNode).join("\n")).toContain(
      "REASON: Need to leave",
    );
    expect(panelByTitle(tree, "TOOL CALLS")).toBeUndefined();
    expect(panelTextLines(mcpPanel as FakeInkNode).join("\n")).toContain(
      "move {direction:down",
    );
    expect(panelTextLines(mcpPanel as FakeInkNode).join("\n")).toContain(
      "CALL: move {direction:down",
    );
  });

  it("renders player action reasons inside agent reasoning without duplicating MCP logs", () => {
    const state = createTuiViewState();
    state.activeView = "agent";
    state.agentStream = {
      status: "batch 1 running",
      thinking: "",
      text: "",
      mcpCalls: ['press {"button":"A"}'],
      events: [
        {
          type: "thinking-delta",
          label: "taskmaster",
          text: "Delegate to the player to pick the starter.",
        },
        { type: "mcp-call", label: "player", text: "status" },
        {
          type: "mcp-result",
          label: "player",
          text: "observe Cyndaquil Poke Ball ahead",
        },
        {
          type: "thinking-delta",
          label: "player",
          text: "The starter ball is directly ahead, so press A.",
        },
        { type: "mcp-call", label: "player", text: 'press {"button":"A"}' },
      ],
    };
    const tree = renderFakeInkTree(180, 48, state);
    const reasoningText = panelTextLines(
      panelByTitle(tree, "AGENT OUTPUT") as FakeInkNode,
    ).join("\n");

    expect(reasoningText).toContain("Delegate to the player");
    expect(reasoningText).toContain("starter ball is directly ahead");
    expect(reasoningText).not.toContain("PLAYER: status");
    expect(reasoningText).not.toContain("PLAYER: observe Cyndaquil");
    expect(reasoningText).not.toContain("PLAYER: press");
  });

  it("renders split view as half Game Boy and half agent details", () => {
    const state = createTuiViewState();
    state.activeView = "agent-split";
    state.agentStream = {
      status: "batch 1 running",
      thinking: "Check the room exit.",
      text: "Move down.",
      mcpCalls: ["move {direction:down, steps:1}"],
      events: [],
    };
    const tree = renderFakeInkTree(180, 48, state);
    const mainContent = tree.children[1] as FakeInkNode;
    const gamePanel = panelByTitle(tree, "GAME BOY");
    const agentPanel = panelByTitle(tree, "AGENT DETAILS");

    expect(mainContent.props?.flexDirection).toBe("row");
    expect(gamePanel?.props?.width).toBe("50%");
    expect(agentPanel?.props?.width).toBe("50%");
    expect(panelTextLines(agentPanel as FakeInkNode).join("\n")).toContain(
      "THINKING: Check the room exit.",
    );
    expect(panelTextLines(agentPanel as FakeInkNode).join("\n")).toContain(
      "MCP: move {direction:down, steps:1}",
    );
  });

  it("renders agent settings with runtime controls and current values", () => {
    const state = createTuiViewState();
    state.activeView = "settings";
    state.settings = {
      agentStatus: "running",
      agentPid: 12345,
      agentModel: "ollama/gemma4:26b",
      agentGoal: "Get the starter Pokemon",
      agentGraphCycleSteps: 8,
      agentRequestDelayMs: 100,
      agentIdentityName: "local-agent",
      soundEnabled: true,
    };
    const tree = renderFakeInkTree(140, 44, state);
    const settingsPanel = panelByTitle(tree, "AGENT SETTINGS");
    const lines = panelTextLines(settingsPanel as FakeInkNode).join("\n");

    expect(settingsPanel).toBeDefined();
    expect(lines).toContain("Agent: running (pid 12345)");
    expect(lines).toContain("Model: ollama/gemma4:26b");
    expect(lines).toContain("Max steps: infinite");
    expect(lines).toContain(":t start/pause agent");
    expect(lines).toContain(":set model <name>");
    expect(lines).toContain(":v cycle views");
    expect(lines).toContain(":u toggle image/text renderer");
  });

  it("shows Professor Culligan live-play countdown in the top title bar", () => {
    const state = createTuiViewState();
    state.livePlay = {
      active: true,
      remainingMs: 4200,
      actionCount: 3,
      resuming: false,
    };
    const tree = renderFakeInkTree(140, 44, state);
    const headerTitle = findText(tree, (text) =>
      text.startsWith("PokeCrystal CLI / Live Play"),
    );

    expect(headerTitle).toContain("Professor Culligan live play");
    expect(headerTitle).toContain("resume 5s");
    expect(
      findText(tree, (text) => text.startsWith("Resume 5s | Played ")),
    ).toBeDefined();
  });

  it("formats elapsed run time as HH:MM:SS", () => {
    expect(formatElapsedRunTime(4_000)).toBe("00:00:04");
    expect(formatElapsedRunTime(65_000)).toBe("00:01:05");
    expect(formatElapsedRunTime(3_661_000)).toBe("01:01:01");
  });

  it("shows the run counter on the right side of the top title row", () => {
    const state = createTuiViewState();
    state.elapsedMs = 3_661_000;
    state.interactionCount = 7;
    const tree = renderFakeInkTree(140, 44, state);
    const headerRow = findNode(
      tree,
      (node) =>
        node.props?.justifyContent === "space-between" &&
        collectText(node, (text) => text.startsWith("Played ")).length > 0,
    );

    expect(headerRow).toBeDefined();
    expect(
      collectText(
        headerRow,
        (text) => text === "Played 01:01:01 | Interactions 7",
      ),
    ).toHaveLength(1);
    expect(
      collectText(headerRow, (text) =>
        text.startsWith("PokeCrystal CLI / Live Play"),
      ),
    ).toHaveLength(1);
  });

  it("shows resuming state in the right-side run counter", () => {
    const state = createTuiViewState();
    state.livePlay = {
      active: true,
      remainingMs: 0,
      actionCount: 3,
      resuming: true,
    };
    const tree = renderFakeInkTree(140, 44, state);

    expect(
      findText(tree, (text) => text.startsWith("Resuming | Played ")),
    ).toBeDefined();
  });

  it("stacks wrapped controls vertically in the footer", () => {
    const state = createTuiViewState();
    state.controlsVisible = true;
    const tree = renderFakeInkTree(100, 32, state);
    const footer = findNode(
      tree,
      (node) =>
        node.props?.borderColor === "yellow" &&
        node.children.some((child) =>
          textValue(child)?.startsWith("Controls:"),
        ),
    );

    expect(footer?.props?.flexDirection).toBe("column");
    const footerLines = collectText(
      footer,
      (text) => text.startsWith("Controls:") || text.startsWith("  "),
    );
    expect(footerLines.length).toBeGreaterThanOrEqual(3);
    expect(footerLines.join(" ")).toContain("view :v");
    expect(footerLines.join(" ")).toContain(":i msg");
    expect(footerLines.join(" ")).toContain(":set key value");
  });

  it("renders medium terminals with diagnostics below the Game Boy surface", () => {
    const tree = renderFakeInkTree(90, 30);
    const mainContent = tree.children[1] as FakeInkNode;
    const gamePanel = panelByTitle(tree, "GAME BOY");
    const menuPanel = panelByTitle(tree, "MENU");

    expect(tree.props?.height).toBe(30);
    expect(mainContent.props?.flexDirection).toBe("column");
    expect(mainContent.props?.height).toBe(22);
    expect(gamePanel?.props?.width).toBe("100%");
    expect(gamePanel?.props?.height).toBe(16);
    expect(menuPanel).toBeDefined();
    expect(panelTextLines(gamePanel as FakeInkNode)).toHaveLength(13);
    expect(collectText(tree, (text) => text.startsWith("Controls:"))).toEqual(
      [],
    );
  });

  it("renders narrow terminals as only the playable Game Boy surface", () => {
    const tree = renderFakeInkTree(64, 24);
    const mainContent = tree.children[1] as FakeInkNode;
    const gamePanel = panelByTitle(tree, "GAME BOY");
    const menuPanel = panelByTitle(tree, "MENU");
    const infoPanel = panelByTitle(tree, "INFO");

    expect(tree.props?.width).toBe(64);
    expect(tree.props?.height).toBe(24);
    expect(tree.props?.overflow).toBe("hidden");
    expect(mainContent.props?.flexDirection).toBe("column");
    expect(mainContent.props?.height).toBe(16);
    expect(gamePanel?.props?.width).toBe("100%");
    expect(gamePanel?.props?.height).toBe(16);
    expect(menuPanel).toBeUndefined();
    expect(infoPanel).toBeUndefined();
    expect(panelTextLines(gamePanel as FakeInkNode)).toHaveLength(13);
  });

  it("uses terminal dimensions when Ink does not provide the dimensions hook", () => {
    const tree = renderFakeInkTree(180, 48, createTuiViewState(), {
      useStdoutHook: false,
    });
    const mainContent = tree.children[1] as FakeInkNode;
    const gamePanel = panelByTitle(tree, "GAME BOY");

    expect(tree.props?.width).toBe(180);
    expect(tree.props?.height).toBe(48);
    expect(mainContent.props?.flexDirection).toBe("row");
    expect(gamePanel?.props?.width).toBe("66%");
  });

  it("does not render empty menu and prompt panels", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      menu: [],
      prompt: [],
      dialogue: ["Welcome to the", "world of #MON!"],
      info: [],
      actions: [],
    };
    const tree = renderFakeInkTree(180, 48, state);
    const gamePanel = panelByTitle(tree, "GAME BOY");

    expect(gamePanel?.props?.width).toBe("100%");
    expect(panelByTitle(tree, "MENU")).toBeUndefined();
    expect(panelByTitle(tree, "PROMPT")).toBeUndefined();
  });

  it("keeps every name-entry option in the playable surface", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      viewport: [
        "NAME ENTRY",
        "YOUR NAME?",
        "NAME  C H _ _ _ _ _",
        "LEN   2/7",
        "KEYBOARD",
        " A   B  [C]  D   E   F   G   H   I ",
        " J   K   L   M   N   O   P   Q   R ",
        " S   T   U   V   W   X   Y   Z ",
        " -   ?   !   /   .   , ",
        " lower   DEL   END ",
        "SELECTED C",
      ],
      menu: [],
      prompt: [],
      dialogue: [],
    };
    const tree = renderFakeInkTree(90, 30, state);
    const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
    const text = panelTextLines(gamePanel).join("\n");

    expect(text).toContain(" A   B  [C]  D   E   F   G   H   I ");
    expect(text).toContain(" J   K   L   M   N   O   P   Q   R ");
    expect(text).toContain(" S   T   U   V   W   X   Y   Z ");
    expect(text).toContain(" -   ?   !   /   .   , ");
    expect(text).toContain(" lower   DEL   END ");
    expect(text).toContain("SELECTED C");
  });

  it("does not duplicate a clipped menu panel for name entry", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      viewport: [
        "NAME ENTRY",
        "YOUR NAME?",
        "KEYBOARD",
        "A B C D E F G H I",
        "J K L M N O P Q R",
        "S T U V W X Y Z",
        "- ? ! / . ,",
        "lower DEL END",
      ],
      menu: [
        "A B C D E F G H I",
        "J K L M N O P Q R",
        "S T U V W X Y Z",
        "- ? ! / . ,",
        "lower DEL END",
      ],
      prompt: [],
      dialogue: [],
    };

    const tree = renderFakeInkTree(180, 48, state);

    expect(panelByTitle(tree, "MENU")).toBeUndefined();
    const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
    expect(panelTextLines(gamePanel).join("\n")).toContain("lower DEL END");
  });

  it("clips overworld rows around the player instead of slicing the player off the bottom", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      viewport: [
        "   03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19 20 21 22",
        "00 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
        "01 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
        "02 N> #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
        "03 S  #  #  D  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  # ",
        "04 .  .  .  .  .  .  .  .  #  #  #  #  #  #  #  #  #  #  #  # ",
        "05 .  .  .  .  .  .  .  S  #  D  #  #  #  #  #  #  #  #  #  # ",
        "06 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
        "07 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
        "08 .  .  .  Nv .  S  .  .  .  N^ .  .  .  .  ~  ~  ~  ~  ~  ~ ",
        "09 .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~  ~  ~  ~  ~ ",
        "10 #  #  #  #  #  #  .  .  .  .  .  .  H  #  #  #  #  #  #  # ",
        "11 .  .  .  .  .  .  .  .  .  .  @v .  .  .  .  .  .  .  .  . ",
      ],
      info: ["Pos: (13,11)"],
      menu: [],
      prompt: [],
      dialogue: [],
      actions: [],
    };

    const tree = renderFakeInkTree(90, 18, state);
    const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
    const text = panelTextLines(gamePanel).join("\n");

    expect(text).toContain("11");
    expect(text).toContain("@v");
    expect(text).not.toContain("00 #");
  });

  it("clips a full New Bark Town viewport around the visible player row", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      viewport: [
        "   00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19",
        "00 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "01 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "02 #  #  .  N> #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "03 #  #  .  S  #  #  D  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "04 #  #  .  .  .  .  .  .  .  .  .  .  #  #  #  #  #  #  #  #",
        "05 #  #  .  .  .  .  .  .  .  .  .  S  #  D  #  #  #  #  #  #",
        "06 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~",
        "07 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~",
        "08 .  .  .  .  .  .  Nv .  S  .  .  .  .  .  .  .  .  .  ~  ~",
        "09 .  .  .  .  .  .  .  .  .  .  .  .  Nv .  .  .  .  .  ~  ~",
        "10 #  #  #  #  #  #  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
        "11 #  #  #  D  #  #  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
        "12 .  .  .  .  .  .  .  .  .  .  #  #  #  #  .  .  .  H  #  #",
        "13 .  .  .  .  .  .  .  .  .  S  #  D  #  #  .  .  .  H  #  #",
        "14 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
        "15 #  #  H  H  H  H  .  @v .  .  .  .  .  .  .  .  .  H  #  #",
        "16 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "17 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
      ],
      info: ["Pos: (7,15)"],
      menu: [],
      prompt: [],
      dialogue: [],
      actions: [],
    };

    const tree = renderFakeInkTree(90, 18, state);
    const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
    const text = panelTextLines(gamePanel).join("\n");

    expect(text).toContain("15");
    expect(text).toContain("@v");
    expect(text).not.toContain("00 #");
  });

  it("uses tall terminal space to show the full Game Boy map viewport", () => {
    const state = createTuiViewState();
    state.snapshot = {
      ...state.snapshot,
      viewport: [
        "   00 01 02 03 04 05 06 07 08 09 10 11 12 13 14 15 16 17 18 19",
        "00 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "01 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "02 #  #  .  N> #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "03 #  #  .  S  #  #  D  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "04 #  #  .  .  .  .  .  .  .  .  .  .  #  #  #  #  #  #  #  #",
        "05 #  #  .  .  .  .  .  .  .  .  .  S  #  D  #  #  #  #  #  #",
        "06 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~",
        "07 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  ~  ~",
        "08 .  .  .  .  .  .  Nv .  S  .  .  .  .  .  .  .  .  .  ~  ~",
        "09 .  .  .  .  .  .  .  .  .  .  .  .  Nv .  .  .  .  .  ~  ~",
        "10 #  #  #  #  #  #  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
        "11 #  #  #  D  #  #  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
        "12 .  .  .  .  .  .  .  .  .  .  #  #  #  #  .  .  .  H  #  #",
        "13 .  .  .  .  .  .  .  .  .  S  #  D  #  #  .  .  .  H  #  #",
        "14 #  #  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  H  #  #",
        "15 #  #  H  H  H  H  .  @v .  .  .  .  .  .  .  .  .  H  #  #",
        "16 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
        "17 #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #  #",
      ],
      info: ["Pos: (7,15)"],
      menu: [],
      prompt: [],
      dialogue: [],
      actions: [],
    };

    const tree = renderFakeInkTree(140, 44, state);
    const gamePanel = panelByTitle(tree, "GAME BOY") as FakeInkNode;
    const text = panelTextLines(gamePanel).join("\n");

    expect(text).toContain("00 #");
    expect(text).toContain("17 #");
    expect(text).toContain("@v");
  });

  it("enters and restores the alternate screen so TUI frames do not scroll the shell", () => {
    const writes: string[] = [];
    const inkRuntime: InkRuntime = {
      React: {
        createElement: (
          type: unknown,
          props?: Record<string, unknown> | null,
          ...children: unknown[]
        ) => ({ type, props, children }),
        useEffect: () => undefined,
        useState: <T>(
          initial: T | (() => T),
        ): [T, (next: T | ((previous: T) => T)) => void] => [
          typeof initial === "function" ? (initial as () => T)() : initial,
          () => undefined,
        ],
      },
      ink: {
        Box: "Box",
        Text: "Text",
        render: () => ({ unmount: () => undefined }),
      },
    };
    const renderer = renderInkTui(inkRuntime, createTuiViewState(), {
      stdin: {} as NodeJS.ReadStream,
      stdout: {
        isTTY: true,
        write: (chunk: string) => {
          writes.push(chunk);
          return true;
        },
      } as NodeJS.WriteStream,
    });

    expect(writes[0]).toContain("\u001b[?1049h");
    expect(writes[0]).toContain("\u001b[?25l");
    renderer.unmount();
    expect(writes.at(-1)).toContain("\u001b[?1049l");
    expect(writes.at(-1)).toContain("\u001b[?25h");
  });
});
