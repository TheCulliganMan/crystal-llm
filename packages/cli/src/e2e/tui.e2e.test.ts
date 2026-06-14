import { EventEmitter } from "node:events";
import type { ChildProcess } from "node:child_process";
import { runInkTui } from "../tui";
import { FakeTtyInput, FakeTtyOutput } from "./test-helpers";
import type { InkRuntime, TuiViewState } from "../tui-ink";
import type { TuiMcpClient } from "../tui-mcp-client";
import type { CliOptions } from "../types";
import type { AgentStreamEvent } from "../agent-stream";

const createFakePngBase64 = (width = 160, height = 144): string => {
  const bytes = Buffer.alloc(24);
  Buffer.from("89504e470d0a1a0a", "hex").copy(bytes, 0);
  bytes.writeUInt32BE(13, 8);
  bytes.write("IHDR", 12, "ascii");
  bytes.writeUInt32BE(width, 16);
  bytes.writeUInt32BE(height, 20);
  return bytes.toString("base64");
};

const createFakeInkRuntime = (updates: TuiViewState[]): InkRuntime => {
  const runtime: InkRuntime = {
    React: {
      createElement: (type: unknown, props?: Record<string, unknown> | null, ...children: unknown[]) => {
        if (typeof type === "function") {
          return (type as (props: Record<string, unknown>) => unknown)({ ...(props ?? {}), children });
        }
        return { type, props, children };
      },
      useEffect: (effect) => {
        effect();
      },
      useState: <T,>(initial: T | (() => T)): [T, (next: T | ((previous: T) => T)) => void] => {
        let value = typeof initial === "function" ? (initial as () => T)() : initial;
        return [
          value,
          (next) => {
            value = typeof next === "function" ? (next as (previous: T) => T)(value) : next;
            updates.push(value as TuiViewState);
          },
        ];
      },
    },
    ink: {
      Box: "Box",
      Text: "Text",
      render: () => ({ unmount: jest.fn() }),
    },
  };
  return runtime;
};

const createStaticTuiMcpClient = (): TuiMcpClient => ({
  callTool: async (tool) => {
    if (tool === "observe") {
      return {
        content: [
          {
            type: "text",
            text: JSON.stringify({
              ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
              view: { viewport: ["00 . @ ."], dialogue: [] },
            }),
          },
        ],
      };
    }
    if (tool === "status") {
      return {
        content: [
          { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
        ],
      };
    }
    if (tool === "recent_events") {
      return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
    }
    return { content: [] };
  },
  close: async () => undefined,
});

const createFakeLinkedAgentProcess = (pid: number) => {
  const process = new EventEmitter() as ChildProcess & EventEmitter & {
    pid: number;
    killed: boolean;
    exitCode: number | null;
    signalCode: NodeJS.Signals | null;
    kill: jest.Mock<boolean, [NodeJS.Signals?]>;
    stdout: null;
    stderr: null;
  };
  process.pid = pid;
  process.killed = false;
  process.exitCode = null;
  process.signalCode = null;
  process.stdout = null;
  process.stderr = null;
  process.kill = jest.fn((signal: NodeJS.Signals = "SIGTERM") => {
    process.killed = true;
    process.signalCode = signal;
    setImmediate(() => process.emit("exit", null, signal));
    return true;
  });
  return process;
};

type NameEntrySimulatorSnapshot = {
  caseMode: "upper" | "lower";
  name: string;
  cursorRow: number;
  cursorColumn: number;
  confirmed: boolean;
};

const NAME_ENTRY_SIM_ROWS = ["ABCDEFGHI", "JKLMNOPQR", "STUVWXYZ"];

const createNameEntrySimulator = (
  initial: Partial<NameEntrySimulatorSnapshot> = {},
): {
  snapshot: NameEntrySimulatorSnapshot;
  client: TuiMcpClient;
  calls: Array<{ tool: string; args?: Record<string, unknown> }>;
  selectedCells: string[];
} => {
  const snapshot: NameEntrySimulatorSnapshot = {
    caseMode: initial.caseMode ?? "upper",
    name: initial.name ?? "",
    cursorRow: initial.cursorRow ?? 0,
    cursorColumn: initial.cursorColumn ?? 0,
    confirmed: initial.confirmed ?? false,
  };
  const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
  const selectedCells: string[] = [];

  const moveBottomRow = (direction: "left" | "right"): void => {
    const group = snapshot.cursorColumn < 3 ? 0 : snapshot.cursorColumn < 6 ? 1 : 2;
    const nextGroup = direction === "right"
      ? (group + 1) % 3
      : (group + 2) % 3;
    snapshot.cursorColumn = nextGroup === 0 ? 0 : nextGroup === 1 ? 3 : 6;
  };

  const applyMove = (direction: "up" | "down" | "left" | "right"): void => {
    if (direction === "up") {
      snapshot.cursorRow = snapshot.cursorRow === 0 ? 4 : snapshot.cursorRow - 1;
      return;
    }
    if (direction === "down") {
      snapshot.cursorRow = snapshot.cursorRow === 4 ? 0 : snapshot.cursorRow + 1;
      return;
    }
    if (snapshot.cursorRow === 4) {
      moveBottomRow(direction);
      return;
    }
    snapshot.cursorColumn =
      (snapshot.cursorColumn + (direction === "right" ? 1 : -1) + 9) % 9;
  };

  const selectedLetter = (): string => {
    const letter = NAME_ENTRY_SIM_ROWS[snapshot.cursorRow]?.[snapshot.cursorColumn] ?? "";
    return snapshot.caseMode === "lower" ? letter.toLowerCase() : letter;
  };

  const selectedCell = (): string => {
    if (snapshot.cursorRow === 4) {
      if (snapshot.cursorColumn < 3) {
        return snapshot.caseMode === "upper" ? "lower" : "UPPER";
      }
      return snapshot.cursorColumn < 6 ? "DEL" : "END";
    }
    return selectedLetter();
  };

  const applyPress = (button: string): void => {
    if (button === "b") {
      selectedCells.push("B:delete");
      snapshot.name = snapshot.name.slice(0, -1);
      return;
    }
    if (button === "start") {
      selectedCells.push("START:END");
      snapshot.cursorRow = 4;
      snapshot.cursorColumn = 6;
      return;
    }
    if (button !== "a") {
      return;
    }
    selectedCells.push(selectedCell());
    if (snapshot.cursorRow === 4) {
      if (snapshot.cursorColumn < 3) {
        snapshot.caseMode = snapshot.caseMode === "upper" ? "lower" : "upper";
      } else if (snapshot.cursorColumn < 6) {
        snapshot.name = snapshot.name.slice(0, -1);
      } else {
        snapshot.confirmed = true;
      }
      return;
    }
    snapshot.name += selectedLetter();
  };

  const observePayload = () => ({
    view: {
      viewport: ["NAME ENTRY"],
      info: [
        "STATE: name_entry",
        "PROMPT: NAME YOUR POKEMON?",
        `CASE: ${snapshot.caseMode}`,
        `NAME: ${snapshot.name || "(blank)"}`,
        `LENGTH: ${snapshot.name.length}/10`,
        `CURSOR: row ${snapshot.cursorRow} col ${snapshot.cursorColumn}`,
        `SELECTED: ${selectedCell()}`,
        ...(snapshot.confirmed ? ["STATUS: confirmed"] : []),
      ],
      menu: [
        snapshot.caseMode === "upper" ? "A B C D E F G H I" : "a b c d e f g h i",
        snapshot.caseMode === "upper" ? "J K L M N O P Q R" : "j k l m n o p q r",
        snapshot.caseMode === "upper" ? "S T U V W X Y Z" : "s t u v w x y z",
        "- ? ! / . ,",
        snapshot.caseMode === "upper" ? "lower DEL END" : "UPPER DEL END",
      ],
    },
  });

  const actionResult = () => ({
    content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true } }) }],
  });

  const client: TuiMcpClient = {
    callTool: async (tool, args) => {
      calls.push({ tool, args });
      if (tool === "observe") {
        return { content: [{ type: "text", text: JSON.stringify(observePayload()) }] };
      }
      if (tool === "status") {
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                mode: "name_entry",
                map: "NAME ENTRY",
                in_menu: true,
                prompt_pending: true,
                can_move: false,
                input_blocked_reason: "name_entry",
              }),
            },
          ],
        };
      }
      if (tool === "recent_events") {
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                recap: `name:${snapshot.name}; confirmed:${snapshot.confirmed}`,
                total: 1,
                events: [],
              }),
            },
          ],
        };
      }
      if (tool === "move") {
        const direction = String(args?.direction ?? "up") as "up" | "down" | "left" | "right";
        applyMove(direction);
        return actionResult();
      }
      if (tool === "press") {
        applyPress(String(args?.button ?? "a"));
        return actionResult();
      }
      if (tool === "type_text") {
        const text = String(args?.text ?? "");
        for (const char of text) {
          selectedCells.push(`TEXT:${char}`);
          snapshot.name += char;
        }
        return actionResult();
      }
      return { content: [] };
    },
    close: async () => undefined,
  };

  return { snapshot, client, calls, selectedCells };
};

describe("Ink TUI e2e", () => {
  const originalKittyEnv = process.env.POKECRYSTAL_CLI_KITTY;
  const originalKittyPlaceholdersEnv = process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS;

  beforeEach(() => {
    process.env.POKECRYSTAL_CLI_KITTY = "0";
    delete process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS;
  });

  afterEach(() => {
    if (originalKittyEnv === undefined) {
      delete process.env.POKECRYSTAL_CLI_KITTY;
    } else {
      process.env.POKECRYSTAL_CLI_KITTY = originalKittyEnv;
    }
    if (originalKittyPlaceholdersEnv === undefined) {
      delete process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS;
    } else {
      process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS = originalKittyPlaceholdersEnv;
    }
  });

  it("uses the direct local client without starting MCP and renders bundled TUI action state", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const updates: TuiViewState[] = [];
    const startMcpServer = jest.fn(async () => ({
      url: "http://127.0.0.1:43210/mcp?session_id=session-direct-local-fast-path",
      close: async () => undefined,
    }));
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        if (tool === "move") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  action: { ok: true, changed: true, effect: "moved", events: ["moved:1"] },
                  tui: {
                    status: { mode: "overworld", map: "NEW_BARK_TOWN", can_move: true, coords: [4, 6] },
                    recent_events: { recap: "moved up", total: 1, events: [{ summary: "Moved up" }] },
                    frame: {
                      ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 6] },
                      view: { viewport: ["00 . @ .", "01 . . ."], dialogue: [] },
                      frame: 2,
                    },
                    frame_id: 2,
                    computed_at_ms: 1234,
                  },
                }),
              },
            ],
          };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-direct-local-fast-path",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer,
        createDirectClient: () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("w");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(startMcpServer).not.toHaveBeenCalled();
    expect(calls.filter((entry) => entry.tool === "move")).toHaveLength(1);
    expect(calls.filter((entry) => entry.tool === "observe")).toHaveLength(1);
    expect(calls.filter((entry) => entry.tool === "status")).toHaveLength(1);
    expect(calls.filter((entry) => entry.tool === "recent_events")).toHaveLength(1);
    expect(calls.find((entry) => entry.tool === "move")?.args).toMatchObject({
      direction: "up",
      include_tui_state: true,
    });
    expect(updates.at(-1)?.snapshot.viewport).toEqual(["00 . @ .", "01 . . ."]);
  });

  it("starts owned MCP HTTP endpoint, initializes through MCP, reacts to input, and cleans up", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const closed: string[] = [];
    const updates: TuiViewState[] = [];
    let sessionInteractionTotal = 0;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: sessionInteractionTotal, events: [] }) }] };
        }
        if (tool === "move") {
          sessionInteractionTotal += 1;
          return { content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true } }) }] };
        }
        return { content: [] };
      },
      close: async () => {
        closed.push("client");
      },
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-1",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-1",
          close: async () => {
            closed.push("server");
          },
        }),
        createMcpClient: async (url) => {
          expect(url).toBe("http://127.0.0.1:43210/mcp?session_id=session-1");
          return client;
        },
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("w");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls.map((entry) => entry.tool)).toEqual(
      expect.arrayContaining(["observe", "status", "recent_events", "move"]),
    );
    expect(calls.find((entry) => entry.tool === "move")?.args).toMatchObject({ direction: "up" });
    expect(updates.some((state) => state.interactionCount === 1)).toBe(true);
    expect(updates.at(-1)?.snapshot.viewport).toEqual(["00 . @ ."]);
    expect(stdin.getRawMode()).toBe(false);
    expect(closed).toEqual(["client", "server"]);
  });

  it("sends arrow keys as directional button presses while a PC menu owns the TUI", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const updates: TuiViewState[] = [];
    const pcMenuSnapshot = {
      ctx: { m: "menu", map: "CHERRYGROVE_POKECENTER_1F", xy: [19, 5] },
      view: {
        viewport: ["BILL's PC"],
        menu: ["> WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>", "  CHANGE BOX", "  SEE YA!"],
        dialogue: [],
      },
    };
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return { content: [{ type: "text", text: JSON.stringify(pcMenuSnapshot) }] };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "menu",
                  can_move: false,
                  in_menu: true,
                  promptPending: true,
                  surface: {
                    kind: "pc",
                    title: "Bill's PC",
                    menu_open: true,
                    selected: "WITHDRAW <PK><MN>",
                    primaryText: "What's up?",
                  },
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "pc menu", total: 0, events: [] }) }] };
        }
        if (tool === "press") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({ actionResult: { ok: true, changed: true, events: ["pressed:down:1"] } }),
              },
            ],
          };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-pc-menu-arrow",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-pc-menu-arrow",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("\u001bOB");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({
      tool: "press",
      args: expect.objectContaining({ button: "down", times: 1 }),
    });
    expect(calls.some((entry) => entry.tool === "move")).toBe(false);
  });

  it("sends Down and A as raw PC presses when the renderer reports the Bill menu as Prompt", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const updates: TuiViewState[] = [];
    const options = ["WITHDRAW <PK><MN>", "DEPOSIT <PK><MN>", "CHANGE BOX", "MOVE <PK><MN> W/O MAIL", "SEE YA!"];
    let selectedIndex = 0;
    const snapshotPayload = () => ({
      ctx: { m: "menu", map: "EcruteakPokecenter1F", xy: [19, 5] },
      view: {
        viewport: ["Prompt"],
        info: ["D-Pad=Move A=Select B=Back"],
        menu: options.map((option, index) => `${index === selectedIndex ? "▶" : " "} ${option}`),
      },
      surface: {
        kind: "prompt",
        title: "Prompt",
        menu_open: true,
        selected: options[selectedIndex],
        primary_text: `▶ ${options[selectedIndex]}`,
      },
    });
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return { content: [{ type: "text", text: JSON.stringify(snapshotPayload()) }] };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "menu",
                  menu: true,
                  in_menu: true,
                  in_dialog: true,
                  text_advance_pending: true,
                  prompt_pending: false,
                  input_blocked_reason: "menu",
                  surface: snapshotPayload().surface,
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "pc menu", total: 0, events: [] }) }] };
        }
        if (tool === "press") {
          const button = String(args?.button ?? "");
          if (button === "down") {
            selectedIndex = 1;
          }
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({ actionResult: { ok: true, changed: true, events: [`pressed:${button}:1`] } }),
              },
            ],
          };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-pc-prompt-surface-menu",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-pc-prompt-surface-menu",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("\u001bOB");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("z");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({ tool: "press", args: expect.objectContaining({ button: "down", times: 1 }) });
    expect(calls).toContainEqual({ tool: "press", args: expect.objectContaining({ button: "a", times: 1 }) });
    expect(calls.some((entry) => entry.tool === "execute_macro")).toBe(false);
    expect(calls.some((entry) => entry.tool === "move")).toBe(false);
  });

  it.each([
    ["Cherrygrove", "CHERRYGROVE_POKECENTER_1F"],
    ["Azalea", "AZALEA_POKECENTER_1F"],
    ["Blackthorn", "BLACKTHORN_POKECENTER_1F"],
    ["Celadon", "CELADON_POKECENTER_1F"],
    ["Cerulean", "CERULEAN_POKECENTER_1F"],
    ["Cianwood", "CIANWOOD_POKECENTER_1F"],
    ["Cinnabar", "CINNABAR_POKECENTER_1F"],
    ["Ecruteak", "ECRUTEAK_POKECENTER_1F"],
    ["Fuchsia", "FUCHSIA_POKECENTER_1F"],
    ["Goldenrod", "GOLDENROD_POKECENTER_1F"],
    ["Indigo Plateau", "INDIGO_PLATEAU_POKECENTER_1F"],
    ["Lavender", "LAVENDER_POKECENTER_1F"],
    ["Mahogany", "MAHOGANY_POKECENTER_1F"],
    ["Olivine", "OLIVINE_POKECENTER_1F"],
    ["Pewter", "PEWTER_POKECENTER_1F"],
    ["Route 10", "ROUTE_10_POKECENTER_1F"],
    ["Route 32", "ROUTE_32_POKECENTER_1F"],
    ["Saffron", "SAFFRON_POKECENTER_1F"],
    ["Silver Cave", "SILVER_CAVE_POKECENTER_1F"],
    ["Vermilion", "VERMILION_POKECENTER_1F"],
    ["Violet", "VIOLET_POKECENTER_1F"],
    ["Viridian", "VIRIDIAN_POKECENTER_1F"],
  ])("keeps a fast %s nurse A press queued while the final overworld step is settling", async (townName, mapName) => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const updates: TuiViewState[] = [];
    const sessionSlug = townName.toLowerCase().replace(/\s+/g, "-");
    let y = 8;
    let facing = "up";
    let dialogueOpen = false;
    let resolveMove: (() => void) | undefined;
    let markMoveStarted: (() => void) | undefined;
    const moveStarted = new Promise<void>((resolve) => {
      markMoveStarted = resolve;
    });
    const snapshotPayload = () => ({
      ctx: { m: "overworld", map: mapName, xy: [7, y], facing },
      view: {
        viewport: [`${townName.toUpperCase()} POKECENTER 1F`, "      +", y === 7 ? "      @" : "       ", y === 8 ? "      @" : "       "],
        dialogue: dialogueOpen ? ["Welcome to our", "POKEMON CENTER!"] : [],
      },
    });
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return { content: [{ type: "text", text: JSON.stringify(snapshotPayload()) }] };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: mapName,
                  coords: [7, y],
                  facing,
                  can_move: !dialogueOpen,
                  in_dialog: dialogueOpen,
                  text_box_open: dialogueOpen,
                  input_blocked_reason: dialogueOpen ? "dialogue" : null,
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        if (tool === "move") {
          expect(args).toMatchObject({ direction: "up", steps: 1 });
          y = 7;
          facing = "up";
          markMoveStarted?.();
          await new Promise<void>((release) => {
            resolveMove = release;
          });
          return { content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true, events: ["moved:1"] } }) }] };
        }
        if (tool === "press") {
          expect(args).toMatchObject({ button: "a", times: 1 });
          const staleBundledFrame = snapshotPayload();
          dialogueOpen = true;
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ok: true,
                  changed: true,
                  summary: "text advance opened",
                  events: ["pressed:a:1"],
                  tui: {
                    frame: staleBundledFrame,
                    status: {
                      mode: "overworld",
                      map: mapName,
                      coords: [7, y],
                      facing,
                      can_move: true,
                      in_dialog: false,
                      text_box_open: false,
                    },
                    recent_events: {
                      recap: "text advance opened",
                      total: 1,
                      events: [{ summary: "text advance opened" }],
                    },
                  },
                }),
              },
            ],
          };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: `session-${sessionSlug}-nurse-fast-a-press`,
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: `http://127.0.0.1:43210/mcp?session_id=session-${sessionSlug}-nurse-fast-a-press`,
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("w");
    await moveStarted;
    stdin.write(" ");
    resolveMove?.();
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({ tool: "move", args: expect.objectContaining({ direction: "up" }) });
    expect(calls).toContainEqual({ tool: "press", args: expect.objectContaining({ button: "a", times: 1 }) });
    expect(updates.at(-1)?.snapshot.dialogue).toEqual(expect.arrayContaining(["Welcome to our", "POKEMON CENTER!"]));
  });

  it("can navigate to and confirm every vertical menu option with split TTY arrow sequences", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const options = ["WITHDRAW <PK><MN>", "DEPOSIT <PK><MN>", "CHANGE BOX", "SEE YA!"];
    const confirmed: string[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let selectedIndex = 0;
    const snapshotPayload = () => ({
      ctx: { m: "menu", map: "CHERRYGROVE_POKECENTER_1F", xy: [19, 5] },
      view: {
        viewport: ["BILL's PC"],
        menu: options.map((option, index) => `${index === selectedIndex ? ">" : " "} ${option}`),
        dialogue: [],
      },
    });
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return { content: [{ type: "text", text: JSON.stringify(snapshotPayload()) }] };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "menu",
                  can_move: false,
                  in_menu: true,
                  promptPending: true,
                  surface: {
                    kind: "pc",
                    title: "Bill's PC",
                    menu_open: true,
                    selected: options[selectedIndex],
                    primaryText: "What's up?",
                  },
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "pc menu", total: confirmed.length, events: [] }) }] };
        }
        if (tool === "press") {
          const button = String(args?.button ?? "");
          if (button === "down") {
            selectedIndex = (selectedIndex + 1) % options.length;
          }
          if (button === "up") {
            selectedIndex = (selectedIndex + options.length - 1) % options.length;
          }
          if (button === "a") {
            confirmed.push(options[selectedIndex]!);
          }
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({ actionResult: { ok: true, changed: true, selected: options[selectedIndex] } }),
              },
            ],
          };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-pc-menu-split-arrows",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-pc-menu-split-arrows",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    const tick = async () => new Promise((resolve) => setImmediate(resolve));
    await tick();
    for (let index = 0; index < options.length; index += 1) {
      if (index > 0) {
        stdin.write("\u001b");
        await tick();
        stdin.write("OB");
        await tick();
      }
      stdin.write("z");
      await tick();
    }
    stdin.write(":q!\r");
    await playPromise;

    expect(confirmed).toEqual(options);
    expect(calls.filter((entry) => entry.tool === "press" && entry.args?.button === "down")).toHaveLength(3);
    expect(calls.some((entry) => entry.tool === "move")).toBe(false);
  });

  it("does not count command, refresh, or noop input as interactions", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const client = createStaticTuiMcpClient();

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-passive-counter",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-passive-counter",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("?");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(0);
  });

  it("keeps instant Kitty battle menu switches off the passive frame queue", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "battle", map: "ROUTE_30", xy: [8, 4] },
                  view: { viewport: ["BATTLE", "FIGHT  PKMN", "PACK   RUN"], dialogue: [] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "battle",
                  instant_mode: true,
                  in_battle: true,
                  can_move: false,
                }),
              },
            ],
          };
        }
        if (tool === "move") {
          return { content: [{ type: "text", text: "BATTLE\nmenu moved" }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty-instant-battle-menu-switches",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty-instant-battle-menu-switches",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      await new Promise((resolve) => setTimeout(resolve, 300));
      stdin.write("d");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write("s");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":q!\r");
      await playPromise;

      expect(calls).toContainEqual({ tool: "move", args: expect.objectContaining({ direction: "right" }) });
      expect(calls).toContainEqual({ tool: "move", args: expect.objectContaining({ direction: "down" }) });
      expect(
        calls.some((entry) => entry.tool === "observe" && Number(entry.args?.advance_frames ?? 0) > 0),
      ).toBe(false);
      expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(2);
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("keeps the Kitty frame when instant battle selection returns text only", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "battle", map: "ROUTE_30", xy: [8, 4] },
                  view: { viewport: ["BATTLE", "FIGHT  PKMN", "PACK   RUN"], dialogue: [] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "battle",
                  instant_mode: true,
                  in_battle: true,
                  can_move: false,
                }),
              },
            ],
          };
        }
        if (tool === "press") {
          return { content: [{ type: "text", text: "BATTLE\nselection accepted" }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty-instant-battle-selection",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty-instant-battle-selection",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      stdin.write("z");
      await new Promise((resolve) => setImmediate(resolve));
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":q!\r");
      await playPromise;

      expect(calls).toContainEqual({ tool: "press", args: expect.objectContaining({ button: "a" }) });
      expect(updates.at(-1)?.gameboyRenderer).toBe("kitty");
      expect(updates.at(-1)?.gameboyImage?.data).toBe(webSessionImage);
      expect(stdout.readText()).not.toContain("Kitty image renderer unavailable; using text.");
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("drops queued trainer battle menu input while a non-instant action is in flight", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let resolveMove: (() => void) | undefined;
    let markMoveStarted: (() => void) | undefined;
    const moveStarted = new Promise<void>((resolve) => {
      markMoveStarted = resolve;
    });
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "battle", map: "ROUTE_30", xy: [8, 4] },
                  view: { viewport: ["BATTLE", "FIGHT  PKMN", "PACK   RUN"], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "battle",
                  battle_is_trainer: true,
                  in_battle: true,
                  can_move: false,
                }),
              },
            ],
          };
        }
        if (tool === "move") {
          markMoveStarted?.();
          await new Promise<void>((release) => {
            resolveMove = release;
          });
          return { content: [{ type: "text", text: "BATTLE\ntrainer menu moved" }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-drop-queued-trainer-battle-menu-input",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-drop-queued-trainer-battle-menu-input",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("d");
    await moveStarted;
    stdin.write("sss");
    resolveMove?.();
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls.filter((entry) => entry.tool === "move")).toEqual([
      { tool: "move", args: expect.objectContaining({ direction: "right" }) },
    ]);
    expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(1);
  });

  it("does not recover busy trainer battle menu navigation with frame-advance observes", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "battle", map: "ROUTE_30", xy: [8, 4] },
                  view: { viewport: ["BATTLE", "FIGHT  PKMN", "PACK   RUN"], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "battle",
                  battle_is_trainer: true,
                  in_battle: true,
                  can_move: false,
                }),
              },
            ],
          };
        }
        if (tool === "move") {
          return { content: [{ type: "text", text: "ok: 1\nch: 1\nfx: busy\nrsn: busy\nBATTLE\ntrainer menu moved" }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-fast-busy-trainer-battle-menu-input",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-fast-busy-trainer-battle-menu-input",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("d");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("s");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls.filter((entry) => entry.tool === "move")).toEqual([
      { tool: "move", args: expect.objectContaining({ direction: "right" }) },
      { tool: "move", args: expect.objectContaining({ direction: "down" }) },
    ]);
    expect(
      calls.some((entry) => entry.tool === "observe" && Number(entry.args?.advance_frames ?? 0) > 0),
    ).toBe(false);
    expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(2);
  });

  it("does not recover busy trainer battle selections with frame-advance observes", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "battle", map: "ROUTE_30", xy: [8, 4] },
                  view: { viewport: ["BATTLE", "TACKLE  GROWL", "LEER    CANCEL"], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "battle",
                  battle_is_trainer: true,
                  in_battle: true,
                  can_move: false,
                }),
              },
            ],
          };
        }
        if (tool === "press") {
          return { content: [{ type: "text", text: "ok: 1\nch: 1\nfx: busy\nrsn: busy\nBATTLE\nselection accepted" }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-fast-busy-trainer-battle-selection",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-fast-busy-trainer-battle-selection",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("z");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls.filter((entry) => entry.tool === "press")).toEqual([
      { tool: "press", args: expect.objectContaining({ button: "a" }) },
    ]);
    expect(
      calls.some((entry) => entry.tool === "observe" && Number(entry.args?.advance_frames ?? 0) > 0),
    ).toBe(false);
    expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(1);
  });

  it("drops queued gameplay input after A starts a battle", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let inBattle = false;
    let resolvePress: (() => void) | undefined;
    let markPressStarted: (() => void) | undefined;
    const pressStarted = new Promise<void>((resolve) => {
      markPressStarted = resolve;
    });
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify(
                  inBattle
                    ? {
                        ctx: { m: "battle", map: "ROUTE_30", xy: [8, 4] },
                        view: { viewport: ["BATTLE", "FIGHT  PACK", "PKMN   RUN"], dialogue: [] },
                      }
                    : {
                        ctx: { m: "overworld", map: "ROUTE_30", xy: [8, 4] },
                        view: { viewport: ["00 . @ T"], dialogue: [] },
                      },
                ),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify(
                  inBattle
                    ? { mode: "battle", map: "ROUTE_30", in_battle: true, can_move: false }
                    : { mode: "overworld", map: "ROUTE_30", can_move: true },
                ),
              },
            ],
          };
        }
        if (tool === "press") {
          expect(args).toMatchObject({ button: "a", times: 1 });
          inBattle = true;
          markPressStarted?.();
          await new Promise<void>((release) => {
            resolvePress = release;
          });
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({ action: { ok: true, changed: true, events: ["battle:start"] } }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-drop-queued-battle-start-input",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-drop-queued-battle-start-input",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("z");
    await pressStarted;
    stdin.write("sss");
    resolvePress?.();
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls.filter((entry) => entry.tool === "press")).toHaveLength(1);
    expect(calls.some((entry) => entry.tool === "move")).toBe(false);
    expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(1);
  });

  it("uses the instant dialogue macro for manual A while text is open", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let dialogOpen = true;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: {
                    viewport: ["00 . @ ."],
                    dialogue: dialogOpen ? ["Hello there!"] : [],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: "NEW_BARK_TOWN",
                  in_dialog: dialogOpen,
                  text_box_open: dialogOpen,
                  can_move: !dialogOpen,
                  input_blocked_reason: dialogOpen ? "dialogue" : null,
                }),
              },
            ],
          };
        }
        if (tool === "execute_macro") {
          dialogOpen = false;
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: true } }) }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-dialog-instant",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-dialog-instant",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(" ");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({
      tool: "execute_macro",
      args: expect.objectContaining({
        macro: "advance_dialog",
        max_presses: 8,
        settle_frames: 25,
      }),
    });
    expect(calls).not.toContainEqual({
      tool: "press",
      args: expect.objectContaining({ button: "a" }),
    });
  });

  it("uses the dialogue macro when observe shows berry text but status lacks text flags", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let dialogOpen = true;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "Route44", xy: [19, 13] },
                  view: {
                    viewport: ["00 . @ tree"],
                    dialogue: dialogOpen ? ["It's a fruit-", "bearing tree."] : [],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: "Route44",
                  can_move: !dialogOpen,
                }),
              },
            ],
          };
        }
        if (tool === "execute_macro") {
          dialogOpen = false;
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: true } }) }] };
        }
        if (tool === "press") {
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: false } }) }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-berry-dialogue-observe-only",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-berry-dialogue-observe-only",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(" ");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({
      tool: "execute_macro",
      args: expect.objectContaining({
        macro: "advance_dialog",
        max_presses: 8,
      }),
    });
    expect(calls).not.toContainEqual({
      tool: "press",
      args: expect.objectContaining({ button: "a" }),
    });
  });

  it("drops movement keys while dialogue owns input", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "OLIVINE_LIGHTHOUSE_5F", xy: [9, 7] },
                  view: {
                    viewport: ["00 . @ N"],
                    dialogue: ["My POKéMON learned", "how to use FLY in"],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: "OlivineLighthouse5F",
                  in_dialog: true,
                  text_box_open: true,
                  text_advance_pending: true,
                  can_move: false,
                  input_blocked_reason: "dialogue",
                  surface: { kind: "overworld", dialogueOpen: true },
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-dialogue-drop-move",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-dialogue-drop-move",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("w");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls.some((entry) => entry.tool === "move")).toBe(false);
    expect(calls.some((entry) => entry.tool === "press")).toBe(false);
  });

  it("refreshes instead of crashing when a queued move reaches dialogue first", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const updates: TuiViewState[] = [];
    let dialogueOpen = false;
    const snapshotPayload = () => ({
      ctx: { m: "overworld", map: "Route40", xy: [21, 57], facing: "left" },
      view: {
        viewport: ["ROUTE 40", dialogueOpen ? "The water is deep." : "@  ~  ~"],
        dialogue: dialogueOpen ? ["The water is deep."] : [],
      },
    });
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return { content: [{ type: "text", text: JSON.stringify(snapshotPayload()) }] };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: "Route40",
                  in_dialog: dialogueOpen,
                  text_box_open: dialogueOpen,
                  can_move: !dialogueOpen,
                  input_blocked_reason: dialogueOpen ? "dialogue" : null,
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        if (tool === "move") {
          dialogueOpen = true;
          return {
            isError: true,
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  available: false,
                  error: {
                    code: "tool_not_available",
                    message: "move is not available during dialogue.",
                    tool: "move",
                    reason: "dialogue",
                  },
                  context: {
                    mode: "overworld",
                    map: "Route40",
                    inDialog: true,
                    textBoxOpen: true,
                    blockedReason: "dialogue",
                  },
                }),
              },
            ],
          };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-dialogue-move-unavailable-refresh",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-dialogue-move-unavailable-refresh",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );
    const playResult = playPromise.then(
      () => ({ ok: true as const }),
      (error: unknown) => ({ ok: false as const, error }),
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("w");
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    const result = await playResult;
    if (!result.ok) {
      throw result.error;
    }

    expect(calls).toContainEqual({ tool: "move", args: expect.objectContaining({ direction: "up", steps: 1 }) });
    expect(updates.at(-1)?.snapshot.dialogue).toEqual(["The water is deep."]);
    expect(
      updates.some(
        (state) =>
          state.snapshot.dialogue.includes("The water is deep.") &&
          state.commandNote === "move is not available during dialogue.",
      ),
    ).toBe(false);
  });

  it("advances battle dialogue with a raw A press because dialogue macros are unavailable in battle", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let dialogueOpen = true;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: dialogueOpen
                  ? "BATTLE\nA=Advance B=Close\n\nDIALOGUE\nTOTODILEAA grew to"
                  : "BATTLE\nVictory!",
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "battle",
                  in_battle: true,
                  in_dialog: dialogueOpen,
                  text_box_open: dialogueOpen,
                  can_move: false,
                  input_blocked_reason: "battle",
                  surface: {
                    kind: "battle",
                    dialogueOpen,
                    primaryText: dialogueOpen ? "TOTODILEAA grew to" : undefined,
                  },
                }),
              },
            ],
          };
        }
        if (tool === "press") {
          dialogueOpen = false;
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: true } }) }] };
        }
        if (tool === "execute_macro") {
          return {
            isError: true,
            content: [{ type: "text", text: JSON.stringify({ error: { message: "execute_macro is not available during battle." } }) }],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-battle-dialogue-advance",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-battle-dialogue-advance",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(" ");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({
      tool: "press",
      args: expect.objectContaining({ button: "a" }),
    });
    expect(calls.some((entry) => entry.tool === "execute_macro")).toBe(false);
  });

  it("rejects gameplay tool errors instead of rendering them as a frozen TUI state", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const client: TuiMcpClient = {
      callTool: async (tool) => {
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "press") {
          return {
            isError: true,
            content: [{ type: "text", text: JSON.stringify({ error: { message: "synthetic press failure" } }) }],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 0, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-tool-error-crash",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-tool-error-crash",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(" ");

    await expect(playPromise).rejects.toThrow("synthetic press failure");
    expect(stdin.getRawMode()).toBe(false);
  });

  it("does not automatically skip Oak intro in terminal mode", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let oakIntroOpen = true;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  view: {
                    viewport: [oakIntroOpen ? "OAK INTRO" : "NAME ENTRY"],
                    info: oakIntroOpen
                      ? ["STATE: oak_intro", "MODE: intro", "PHASE: text", "WAITING: yes"]
                      : ["STATE: name_entry"],
                    dialogue: oakIntroOpen ? ["Hello! Sorry to", "keep you waiting!"] : [],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify(
                  oakIntroOpen
                    ? {
                        mode: "oak_intro",
                        in_dialog: true,
                        text_box_open: true,
                        can_move: false,
                        input_blocked_reason: "oak_intro",
                        surface: {
                          kind: "oak_intro",
                          title: "Oak Intro",
                          state: "oak_intro",
                          waiting: true,
                          dialogue_open: true,
                        },
                      }
                    : {
                        mode: "name_entry",
                        can_move: false,
                        input_blocked_reason: "name_entry",
                        surface: { kind: "name_entry", title: "Name Entry" },
                      }
                ),
              },
            ],
          };
        }
        if (tool === "press") {
          oakIntroOpen = false;
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: true } }) }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-oak-intro-instant",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-oak-intro-instant",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).not.toContainEqual({
      tool: "press",
      args: expect.objectContaining({
        button: "b",
        times: 1,
      }),
    });
    expect(calls).not.toContainEqual({
      tool: "execute_macro",
      args: expect.anything(),
    });
    expect(calls).not.toContainEqual({
      tool: "press",
      args: expect.objectContaining({ button: "a" }),
    });
  });

  it("advances Oak intro clock dialogue with the dialogue macro instead of remapping A to B", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    let oakIntroOpen = true;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  view: {
                    viewport: [oakIntroOpen ? "OAK INTRO" : "OAK INTRO"],
                    info: oakIntroOpen
                      ? ["STATE: oak_intro", "MODE: intro", "PHASE: wake_dialogue", "WAITING: yes"]
                      : ["STATE: oak_intro", "MODE: intro", "PHASE: set_hour"],
                    dialogue: oakIntroOpen ? ["Will you check the", "clock for me?"] : ["What time is it?"],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify(
                  oakIntroOpen
                    ? {
                        mode: "oak_intro",
                        in_dialog: true,
                        text_box_open: true,
                        text_advance_pending: true,
                        can_move: false,
                        input_blocked_reason: "oak_intro",
                        surface: {
                          kind: "oak_intro",
                          title: "Prompt",
                          state: "oak_intro",
                          waiting: true,
                          dialogue_open: true,
                        },
                      }
                    : {
                        mode: "oak_intro",
                        in_dialog: true,
                        text_box_open: true,
                        prompt_pending: true,
                        can_move: false,
                        input_blocked_reason: "oak_intro",
                        surface: { kind: "oak_intro", title: "Prompt", prompt_open: true },
                      }
                ),
              },
            ],
          };
        }
        if (tool === "execute_macro") {
          oakIntroOpen = false;
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: true } }) }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-oak-clock-dialogue",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-oak-clock-dialogue",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(" ");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toContainEqual({
      tool: "execute_macro",
      args: expect.objectContaining({
        macro: "advance_dialog",
        max_presses: 8,
      }),
    });
    expect(calls).not.toContainEqual({
      tool: "press",
      args: expect.objectContaining({
        button: "b",
        times: 1,
      }),
    });
  });

  it("does not passively advance Kitty frames while dialogue is waiting", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const observeArgs: Array<Record<string, unknown> | undefined> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        if (tool === "observe") {
          observeArgs.push(args);
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: ["Hello there!"] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: "NEW_BARK_TOWN",
                  inDialog: true,
                  textBoxOpen: true,
                  canMove: false,
                  blockedReason: "dialogue",
                  surface: {
                    kind: "overworld",
                    dialogueOpen: true,
                  },
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty-dialog-paused",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime([]),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty-dialog-paused",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      expect(observeArgs[0]).toMatchObject({ include_image: true, image_scale: 2 });
      await new Promise((resolve) => setTimeout(resolve, 300));
      const everyObserveAvoidedFrameAdvance = observeArgs.every((args) => args?.advance_frames === undefined);

      stdin.write(":q!\r");
      await playPromise;

      expect(everyObserveAvoidedFrameAdvance).toBe(true);
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("counts linked-agent gameplay MCP calls as interactions", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const agentProcess = createFakeLinkedAgentProcess(212);
    let onAgentStream: ((event: AgentStreamEvent) => void) | undefined;

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-agent-counter",
        agent: true,
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-agent-counter",
          close: async () => undefined,
        }),
        createMcpClient: async () => createStaticTuiMcpClient(),
        startLinkedAgent: (_options, _mcpUrl, onStreamEvent) => {
          onAgentStream = onStreamEvent;
          return {
            process: agentProcess,
            note: `Agent linked via MCP (pid ${agentProcess.pid}).`,
            output: () => "",
          };
        },
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    onAgentStream?.({ type: "mcp-call", name: "observe", summary: "{}", source: "mcp" });
    await new Promise((resolve) => setImmediate(resolve));
    onAgentStream?.({ type: "mcp-call", name: "move", summary: "{\"direction\":\"up\"}", source: "mcp" });
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(updates.some((state) => state.interactionCount === 1)).toBe(true);
    expect(Math.max(...updates.map((state) => state.interactionCount))).toBe(1);
  });

  it("cycles between play, full agent details, split, and settings views from command mode", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const client: TuiMcpClient = {
      callTool: async (tool) => {
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };
    const agentProcess = createFakeLinkedAgentProcess(211);

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-views",
        agent: true,
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-views",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
        startLinkedAgent: () => ({
          process: agentProcess,
          note: `Agent linked via MCP (pid ${agentProcess.pid}).`,
          output: () => "",
        }),
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(updates.map((state) => state.activeView)).toEqual(
      expect.arrayContaining(["agent", "agent-split", "settings", "play"]),
    );
    expect(updates.find((state) => state.activeView === "agent")?.commandNote).toContain("Agent details view");
    expect(updates.find((state) => state.activeView === "agent-split")?.commandNote).toContain("Split Game Boy");
    expect(updates.at(-1)?.activeView).toBe("play");
  });

  it("toggles the Game Boy renderer without leaving the agent view", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const observeArgs: Array<Record<string, unknown> | undefined> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        if (tool === "observe") {
          observeArgs.push(args);
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };
    const agentProcess = createFakeLinkedAgentProcess(212);

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-agent-renderer-toggle",
          agent: true,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-agent-renderer-toggle",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
          startLinkedAgent: () => ({
            process: agentProcess,
            note: `Agent linked via MCP (pid ${agentProcess.pid}).`,
            output: () => "",
          }),
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":v\r");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":u\r");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":u\r");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":q!\r");
      await playPromise;

      expect(updates.some((state) => state.activeView === "agent" && state.gameboyRenderer === "text")).toBe(true);
      expect(
        updates.some((state) =>
          state.activeView === "agent" &&
          state.gameboyRenderer === "kitty" &&
          state.gameboyImage?.data === webSessionImage
        ),
      ).toBe(true);
      expect(updates.at(-1)?.activeView).toBe("agent");
      expect(observeArgs.filter((args) => args?.include_image === true).length).toBeGreaterThanOrEqual(2);
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("opens settings and starts, pauses, then resumes the linked agent with :t", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const startCalls: Array<{ options: CliOptions; mcpUrl: string }> = [];
    const processes = [createFakeLinkedAgentProcess(101), createFakeLinkedAgentProcess(102)];

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-agent-toggle",
        agentModel: "ollama/gemma4:26b",
        agentGoal: "Get the starter Pokemon",
        agentMaxSteps: 6,
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-agent-toggle",
          close: async () => undefined,
        }),
        createMcpClient: async () => createStaticTuiMcpClient(),
        startLinkedAgent: (options, mcpUrl) => {
          startCalls.push({ options, mcpUrl });
          const process = processes[startCalls.length - 1]!;
          return {
            process,
            note: `Agent linked via MCP (pid ${process.pid}).`,
            output: () => "",
          };
        },
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":t\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":t\r");
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":t\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(startCalls).toHaveLength(2);
    expect(startCalls.map((call) => call.options.agentCommand)).toEqual(["run", "resume"]);
    expect(processes[0]!.kill).toHaveBeenCalledWith("SIGTERM");
    expect(updates.map((state) => state.activeView)).toContain("settings");
    expect(updates.some((state) => state.settings?.agentStatus === "running" && state.settings.agentPid === 101)).toBe(true);
    expect(updates.some((state) => state.settings?.agentStatus === "paused")).toBe(true);
    expect(updates.some((state) => state.settings?.agentStatus === "running" && state.settings.agentPid === 102)).toBe(true);
  });

  it("pauses a running agent during manual play and resumes with Professor Culligan's intervention", async () => {
    const previousIdleMs = process.env.POKECRYSTAL_CLI_AGENT_INTERVENTION_IDLE_MS;
    process.env.POKECRYSTAL_CLI_AGENT_INTERVENTION_IDLE_MS = "50";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const startCalls: Array<{ options: CliOptions; mcpUrl: string }> = [];
    const processes = [createFakeLinkedAgentProcess(401), createFakeLinkedAgentProcess(402)];

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-agent-manual",
          agent: true,
          agentCommand: "run",
          agentGoal: "Continue toward Mt. Silver",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-agent-manual",
            close: async () => undefined,
          }),
          createMcpClient: async () => createStaticTuiMcpClient(),
          startLinkedAgent: (options, mcpUrl) => {
            startCalls.push({ options, mcpUrl });
            const process = processes[startCalls.length - 1]!;
            return {
              process,
              note: `Agent linked via MCP (pid ${process.pid}).`,
              output: () => "",
            };
          },
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      stdin.write("w");
      await new Promise((resolve) => setTimeout(resolve, 140));
      stdin.write(":q!\r");
      await playPromise;

      expect(startCalls).toHaveLength(2);
      expect(startCalls.map((call) => call.options.agentCommand)).toEqual(["run", "resume"]);
      expect(processes[0]!.kill).toHaveBeenCalledWith("SIGTERM");
      expect(startCalls[1]!.options.agentGoal).toContain("Professor Culligan's Intervention");
      expect(startCalls[1]!.options.agentGoal).toContain("move up");
      expect(startCalls[1]!.options.agentGoal).toContain("manual inputs below have already happened");
      expect(updates.some((state) => state.livePlay?.active && state.title.includes("Live Play"))).toBe(true);
    } finally {
      if (previousIdleMs === undefined) {
        delete process.env.POKECRYSTAL_CLI_AGENT_INTERVENTION_IDLE_MS;
      } else {
        process.env.POKECRYSTAL_CLI_AGENT_INTERVENTION_IDLE_MS = previousIdleMs;
      }
    }
  });

  it("switches the running agent model and restarts with the updated settings", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const startCalls: Array<{ options: CliOptions; mcpUrl: string }> = [];
    const processes = [
      createFakeLinkedAgentProcess(201),
      createFakeLinkedAgentProcess(202),
      createFakeLinkedAgentProcess(203),
    ];

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-agent-model",
        agent: true,
        agentCommand: "run",
        agentModel: "ollama/gemma4:26b",
        agentGoal: "Get the starter Pokemon",
        agentMaxSteps: 6,
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-agent-model",
          close: async () => undefined,
        }),
        createMcpClient: async () => createStaticTuiMcpClient(),
        startLinkedAgent: (options, mcpUrl) => {
          startCalls.push({ options, mcpUrl });
          const process = processes[startCalls.length - 1]!;
          return {
            process,
            note: `Agent linked via MCP (pid ${process.pid}).`,
            output: () => "",
          };
        },
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":v\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":set model ollama/qwen3:32b\r");
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":set steps 9\r");
    await new Promise((resolve) => setImmediate(resolve));
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(startCalls).toHaveLength(3);
    expect(startCalls.map((call) => call.options.agentCommand)).toEqual(["run", "resume", "resume"]);
    expect(startCalls[0]!.options.agentModel).toBe("ollama/gemma4:26b");
    expect(startCalls[1]!.options.agentModel).toBe("ollama/qwen3:32b");
    expect(startCalls[2]!.options.agentModel).toBe("ollama/qwen3:32b");
    expect(startCalls[2]!.options.agentMaxSteps).toBe(9);
    expect(processes[0]!.kill).toHaveBeenCalledWith("SIGTERM");
    expect(processes[1]!.kill).toHaveBeenCalledWith("SIGTERM");
    expect(updates.some((state) => state.settings?.agentModel === "ollama/qwen3:32b")).toBe(true);
    expect(updates.some((state) => state.settings?.agentMaxSteps === 9)).toBe(true);
  });

  it("toggles TUI sound with :a and sends action cues only while enabled", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    let soundEnabled = false;
    let pressCount = 0;
    const soundController = {
      setEnabled: jest.fn((enabled: boolean) => {
        soundEnabled = enabled;
      }),
      isEnabled: jest.fn(() => soundEnabled),
      syncSnapshot: jest.fn(),
      close: jest.fn(),
    };
    const client: TuiMcpClient = {
      callTool: async (tool) => {
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "overworld",
                  map: "NEW_BARK_TOWN",
                  can_move: true,
                  audio: {
                    musicToken: "MUSIC_NEW_BARK_TOWN",
                    musicSource: "/api/audio/pcm/music/newbarktown.json",
                    musicRole: "map",
                    recentEvents: pressCount
                      ? [
                          {
                            sequence: 1,
                            kind: "sfx",
                            token: "SFX_READ_TEXT_2",
                            source: "/api/audio/pcm/sfx/readtext2.json",
                          },
                        ]
                      : [],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        if (tool === "press") {
          pressCount += 1;
          return { content: [{ type: "text", text: JSON.stringify({ action: { ok: true, changed: true, effect: "advanced_dialogue" } }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-sound",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime(updates),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-sound",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
        soundController,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":a\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(" ");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":a\r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(soundController.setEnabled.mock.calls.map(([enabled]) => enabled)).toEqual([false, true, false]);
    expect(soundController.syncSnapshot).toHaveBeenCalledWith(
      expect.objectContaining({
        musicToken: "MUSIC_NEW_BARK_TOWN",
        musicSource: "/api/audio/pcm/music/newbarktown.json",
      }),
    );
    expect(soundController.syncSnapshot).toHaveBeenCalledWith(
      expect.objectContaining({
        recentEvents: [
          expect.objectContaining({
            token: "SFX_READ_TEXT_2",
            source: "/api/audio/pcm/sfx/readtext2.json",
          }),
        ],
      }),
    );
    expect(updates.some((state) => state.commandNote === "Sound on. Use :a to mute.")).toBe(true);
    expect(updates.some((state) => state.commandNote === "Sound off. Use :a to enable.")).toBe(true);
    expect(soundController.close).toHaveBeenCalled();
  });

  it("defaults to Kitty image observes when supported and returns to text observes with :u", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const observeArgs: Array<Record<string, unknown> | undefined> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        if (tool === "observe") {
          observeArgs.push(args);
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        if (tool === "move") {
          return { content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true } }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      expect(observeArgs[0]).toMatchObject({ include_image: true, image_scale: 2 });
      await new Promise((resolve) => setTimeout(resolve, 300));
      expect(
        observeArgs.some((args) =>
          args?.include_image === true &&
          args?.image_scale === 2 &&
          args?.advance_frames === 25
        ),
      ).toBe(true);

      stdin.write("w");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":u\r");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write("r");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":q!\r");
      await playPromise;

      const imageObserveCalls = observeArgs.filter((args) => args?.include_image === true);
      expect(imageObserveCalls.length).toBeGreaterThanOrEqual(2);
      expect(imageObserveCalls.every((args) => args?.image_scale === 2)).toBe(true);
      expect(observeArgs.at(-1)).not.toMatchObject({ include_image: true });
      expect(
        updates.some((state) =>
          state.gameboyRenderer === "kitty" &&
          state.gameboyImage?.width === 160 &&
          state.gameboyImage.data === webSessionImage
        ),
      ).toBe(true);
      expect(updates.at(-1)?.gameboyRenderer).toBe("text");
      expect(stdout.readText()).toContain("\u001b_Ga=t");
      expect(stdout.readText()).toContain("\u001b_Ga=p,U=1");
      expect(stdout.readText()).toContain(webSessionImage);
      expect(stdout.readText()).toContain("\u001b_Ga=d,d=I");
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("falls back to text gameplay when Kitty observes do not return an image", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const observeArgs: Array<Record<string, unknown> | undefined> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        if (tool === "observe") {
          observeArgs.push(args);
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty-fallback",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty-fallback",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      stdin.write("r");
      await new Promise((resolve) => setImmediate(resolve));
      stdin.write(":q!\r");
      await playPromise;

      expect(observeArgs[0]).toMatchObject({ include_image: true, image_scale: 2 });
      expect(observeArgs.at(-1)).not.toMatchObject({ include_image: true });
      expect(updates.some((state) => state.gameboyRenderer === "text")).toBe(true);
      expect(updates.some((state) => state.commandNote === "Kitty image renderer unavailable; using text.")).toBe(true);
      expect(stdout.readText()).not.toContain("\u001b_G");
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("does not queue overlapping Kitty image refreshes when image observes are slow", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const observeArgs: Array<Record<string, unknown> | undefined> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    let observeCount = 0;
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        if (tool === "observe") {
          observeCount += 1;
          observeArgs.push(args);
          if (observeCount > 1 && args?.advance_frames === 25) {
            await new Promise((resolve) => setTimeout(resolve, 700));
          }
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty-slow-refresh",
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty-slow-refresh",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
        },
      );

      await new Promise((resolve) => setTimeout(resolve, 900));
      stdin.write(":q!\r");
      await playPromise;

      const imageRefreshCalls = observeArgs.slice(1).filter((args) => args?.advance_frames === 25);
      expect(imageRefreshCalls).toHaveLength(1);
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
    }
  });

  it("does not retransmit Kitty image data for linked-agent text-only updates", async () => {
    const previousKitty = process.env.POKECRYSTAL_CLI_KITTY;
    const previousPlaceholders = process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS;
    process.env.POKECRYSTAL_CLI_KITTY = "1";
    delete process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS;
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const updates: TuiViewState[] = [];
    const observeArgs: Array<Record<string, unknown> | undefined> = [];
    const webSessionImage = createFakePngBase64(160, 144);
    let onAgentStream: ((event: AgentStreamEvent) => void) | undefined;
    const agentProcess = createFakeLinkedAgentProcess(301);
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        if (tool === "observe") {
          observeArgs.push(args);
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
              ...(args?.include_image
                ? [{ type: "image", mimeType: "image/png", data: webSessionImage }]
                : []),
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    try {
      const playPromise = runInkTui(
        {
          command: "play",
          transport: "local",
          baseUrl: "",
          sessionId: "session-kitty-agent-stream",
          agent: true,
        },
        {
          stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
          stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
          inkRuntime: createFakeInkRuntime(updates),
          startMcpServer: async () => ({
            url: "http://127.0.0.1:43210/mcp?session_id=session-kitty-agent-stream",
            close: async () => undefined,
          }),
          createMcpClient: async () => client,
          startLinkedAgent: (_options, _mcpUrl, onStreamEvent) => {
            onAgentStream = onStreamEvent;
            return {
              process: agentProcess,
              note: `Agent linked via MCP (pid ${agentProcess.pid}).`,
              output: () => "",
            };
          },
        },
      );

      await new Promise((resolve) => setImmediate(resolve));
      expect(observeArgs[0]).toMatchObject({ include_image: true, image_scale: 2 });
      const beforeAgentText = stdout.readText();
      onAgentStream?.({ type: "text-delta", text: "Head toward Elm.", source: "taskmaster" });
      await new Promise((resolve) => setImmediate(resolve));
      await new Promise((resolve) => setImmediate(resolve));
      const agentOnlyOutput = stdout.readText().slice(beforeAgentText.length);
      stdin.write(":q!\r");
      await playPromise;

      expect(updates.some((state) => state.agentStream?.text.includes("Head toward Elm."))).toBe(true);
      expect(agentOnlyOutput).not.toContain("\u001b_Ga=t");
      expect(agentOnlyOutput).not.toContain("\u001b_Ga=T");
      expect(agentOnlyOutput).not.toContain(webSessionImage);
      expect(agentOnlyOutput).not.toContain("\u001b_Ga=d");
    } finally {
      if (previousKitty === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY = previousKitty;
      }
      if (previousPlaceholders === undefined) {
        delete process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS;
      } else {
        process.env.POKECRYSTAL_CLI_KITTY_PLACEHOLDERS = previousPlaceholders;
      }
    }
  });

  it("routes menu navigation keys through directional MCP button presses", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "menu", map: "NEW_BARK_TOWN", xy: [4, 7], menu: 1 },
                  view: {
                    viewport: ["OVERWORLD"],
                    menu: ["▶ POKEDEX", "  POKEMON", "  PACK"],
                    dialogue: [],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "menu", map: "NEW_BARK_TOWN", can_move: false }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        if (tool === "press") {
          return { content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true } }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-menu-nav",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-menu-nav",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("s");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toEqual(
      expect.arrayContaining([
        { tool: "press", args: expect.objectContaining({ button: "down", times: 1 }) },
      ]),
    );
    expect(calls.some((entry) => entry.tool === "move")).toBe(false);
  });

  it("routes Escape to B on the slot machine instead of opening command mode", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "menu", map: "GoldenrodGameCorner", xy: [29, 19], menu: 1 },
                  view: {
                    viewport: ["SLOT MACHINE", "COINS 634", "BET 3", "WIN 8"],
                    info: ["STATE: slot_machine", "Left/Right=Bet A=Spin B=Quit"],
                    dialogue: [],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  mode: "menu",
                  map: "GoldenrodGameCorner",
                  can_move: false,
                  surface: { kind: "slot_machine", title: "Slot Machine" },
                  input_blocked_reason: "menu",
                }),
              },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        if (tool === "press") {
          return { content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true } }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-slot-escape",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-slot-escape",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("\u001b");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(calls).toEqual(
      expect.arrayContaining([
        { tool: "press", args: expect.objectContaining({ button: "b", times: 1 }) },
      ]),
    );
    expect(calls).not.toContainEqual({ tool: "press", args: expect.objectContaining({ button: "a" }) });
  });

  it("mutes console output while Ink owns the terminal and restores it on exit", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const logSpy = jest.spyOn(console, "log").mockImplementation(() => undefined);
    const client: TuiMcpClient = {
      callTool: async (tool) => {
        console.log(`runtime log from ${tool}`);
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  ctx: { m: "overworld", map: "NEW_BARK_TOWN", xy: [4, 7] },
                  view: { viewport: ["00 . @ ."], dialogue: [] },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return {
            content: [
              { type: "text", text: JSON.stringify({ mode: "overworld", map: "NEW_BARK_TOWN", can_move: true }) },
            ],
          };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-1",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-1",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("r");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(logSpy).not.toHaveBeenCalled();
    console.log("after tui");
    expect(logSpy).toHaveBeenCalledWith("after tui");
    logSpy.mockRestore();
  });

  it("turns typed letters into naming-screen selection input", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const calls: Array<{ tool: string; args?: Record<string, unknown> }> = [];
    const client: TuiMcpClient = {
      callTool: async (tool, args) => {
        calls.push({ tool, args });
        if (tool === "observe") {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({
                  view: {
                    viewport: ["NAME ENTRY"],
                    info: ["STATE: name_entry", "CURSOR: row 0 col 0"],
                    menu: ["A B C D E F G H I", "▲"],
                  },
                }),
              },
            ],
          };
        }
        if (tool === "status") {
          return { content: [{ type: "text", text: JSON.stringify({ mode: "overworld", can_move: false }) }] };
        }
        if (tool === "recent_events") {
          return { content: [{ type: "text", text: JSON.stringify({ recap: "latest", total: 1, events: [] }) }] };
        }
        if (tool === "move" || tool === "press" || tool === "type_text") {
          return { content: [{ type: "text", text: JSON.stringify({ actionResult: { ok: true, changed: true } }) }] };
        }
        return { content: [] };
      },
      close: async () => undefined,
    };

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-1",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-1",
          close: async () => undefined,
        }),
        createMcpClient: async () => client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("c");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    const actionCalls = calls.filter((entry) => entry.tool === "move" || entry.tool === "press" || entry.tool === "type_text");
    expect(calls.some((entry) => entry.tool === "execute_macro")).toBe(false);
    expect(actionCalls).toEqual([
      { tool: "type_text", args: expect.objectContaining({ text: "c" }) },
    ]);
  });

  it("proves physical z types literal name-entry text through MCP type_text", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const simulator = createNameEntrySimulator({
      cursorRow: 1,
      cursorColumn: 1,
      caseMode: "upper",
    });

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-name-entry-z-button-proof",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-name-entry-z-button-proof",
          close: async () => undefined,
        }),
        createMcpClient: async () => simulator.client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    stdin.write("z");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(simulator.selectedCells).toEqual(["TEXT:z"]);
    expect(simulator.snapshot.name).toBe("z");
    expect(
      simulator.calls.filter((entry) => entry.tool === "press" || entry.tool === "type_text"),
    ).toEqual([
      { tool: "type_text", args: expect.objectContaining({ text: "z" }) },
    ]);
  });

  it("proves Delete can clear a name and lowercase do can be typed and confirmed through MCP calls", async () => {
    const stdin = new FakeTtyInput();
    const stdout = new FakeTtyOutput();
    const simulator = createNameEntrySimulator({
      name: "CYNDAQUILZ",
      cursorRow: 2,
      cursorColumn: 7,
      caseMode: "upper",
    });

    const playPromise = runInkTui(
      {
        command: "play",
        transport: "local",
        baseUrl: "",
        sessionId: "session-name-entry-proof",
      },
      {
        stdin: stdin as unknown as NodeJS.ReadStream & { isTTY?: boolean; setRawMode?(mode: boolean): void },
        stdout: stdout as unknown as NodeJS.WriteStream & { isTTY?: boolean },
        inkRuntime: createFakeInkRuntime([]),
        startMcpServer: async () => ({
          url: "http://127.0.0.1:43210/mcp?session_id=session-name-entry-proof",
          close: async () => undefined,
        }),
        createMcpClient: async () => simulator.client,
      },
    );

    await new Promise((resolve) => setImmediate(resolve));
    for (let index = 0; index < "CYNDAQUILZ".length; index += 1) {
      stdin.write("\u001b[3~");
      await new Promise((resolve) => setImmediate(resolve));
    }
    for (const key of "do") {
      stdin.write(key);
      await new Promise((resolve) => setImmediate(resolve));
    }
    stdin.write("\u001b[F");
    await new Promise((resolve) => setImmediate(resolve));
    stdin.write(":q!\r");
    await playPromise;

    expect(simulator.snapshot.name).toBe("do");
    expect(simulator.snapshot.confirmed).toBe(true);
    expect(simulator.selectedCells).toEqual([
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "B:delete",
      "TEXT:d",
      "TEXT:o",
      "START:END",
      "END",
    ]);
    expect(simulator.calls).not.toContainEqual({ tool: "execute_macro", args: expect.anything() });
    expect(simulator.calls).toEqual(
      expect.arrayContaining([
        { tool: "press", args: expect.objectContaining({ button: "b", times: 1 }) },
        { tool: "type_text", args: expect.objectContaining({ text: "d" }) },
        { tool: "type_text", args: expect.objectContaining({ text: "o" }) },
        { tool: "press", args: expect.objectContaining({ button: "start", times: 1 }) },
        { tool: "press", args: expect.objectContaining({ button: "a", times: 1 }) },
      ]),
    );
  });
});
