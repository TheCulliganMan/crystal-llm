import { __testing as sessionTesting } from "@/app/mcp/session";
import { PRIMARY_MCP_SESSION_ID } from "@/app/mcp/session-id";
import http from "node:http";

jest.mock("@/app/game", () => ({
  Game: {
    create: jest.fn(async (ui: { renderSnapshot?: (...args: unknown[]) => void }) => ({
      draw: jest.fn(() => {
        ui.renderSnapshot?.(
          ["OVERWORLD", "@"],
          ["D-PAD=Move A=Talk START=Menu"],
          "Overworld",
          "Legend",
          null,
          null,
          null
        );
      }),
      tick: jest.fn(),
      postEvent: jest.fn(),
      isMenuOpen: jest.fn(() => false),
      isBattleActive: jest.fn(() => false),
      getMapName: jest.fn(() => "PlayersHouse1F"),
      getGameState: jest.fn(() => ({
        wram: { player_x: 5, player_y: 3, wXCoord: 5, wYCoord: 3, wMapGroup: 2, wMapNumber: 1 },
        sram: {
          badges: { johto: [true, false], kanto: [] },
          party: {
            pokemon: [
              {
                species: "CYNDAQUIL",
                nickname: "CYNDAQUIL",
                level: 5,
                hp: 20,
                max_hp: 20,
                attack: 10,
                defense: 9,
                speed: 11,
                special_attack: 12,
                special_defense: 10,
                moves: [{ name: "TACKLE", current_pp: 35 }],
                original_trainer_name: "PLAYER",
                original_trainer_id: 0,
                experience: 125,
                happiness: 70,
              },
            ],
          },
        },
      })),
      getOverworld: jest.fn(() => ({
        player_direction: "down",
      })),
    })),
  },
}));

jest.mock("@pokecrystal/core/core/save", () => ({
  hasSaveGame: jest.fn(async () => false),
  saveGame: jest.fn(async () => undefined),
}));

const MCP_URL = "http://localhost/api/mcp";
const MCP_PROTOCOL_VERSION = "2024-11-05";
let postHandler: ((request: Request) => Promise<Response>) | null = null;
const originalRequireSessionSecret = process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
const originalMcpToken = process.env.POKECRYSTAL_MCP_TOKEN;

const rawHeadersKey = Symbol.for("mcp.rawHeaders");

const ensureRawHeaders = (): void => {
  const existing = (http.IncomingMessage as unknown as { __mcpPatched?: boolean }).__mcpPatched;
  if (existing) {
    return;
  }
  const OriginalIncomingMessage = http.IncomingMessage;
  class PatchedIncomingMessage extends OriginalIncomingMessage {
    constructor(socket: ConstructorParameters<typeof OriginalIncomingMessage>[0]) {
      super(socket);
      Object.defineProperty(this, rawHeadersKey, {
        configurable: true,
        enumerable: false,
        writable: true,
        value: [],
      });
      Object.defineProperty(this, "rawHeaders", {
        configurable: true,
        enumerable: true,
        get() {
          const stored = (this as { [rawHeadersKey]?: string[] })[rawHeadersKey] ?? [];
          if (stored.length) {
            return stored;
          }
          const headers = (this as { headers?: Record<string, string | string[] | undefined> }).headers ?? {};
          const raw: string[] = [];
          for (const [key, value] of Object.entries(headers)) {
            if (Array.isArray(value)) {
              value.forEach((entry) => raw.push(key, entry));
            } else if (value !== undefined) {
              raw.push(key, String(value));
            }
          }
          return raw;
        },
        set(value: string[]) {
          (this as { [rawHeadersKey]?: string[] })[rawHeadersKey] = value;
        },
      });
    }
  }
  (PatchedIncomingMessage as unknown as { __mcpPatched?: boolean }).__mcpPatched = true;
  Object.defineProperty(http, "IncomingMessage", {
    configurable: true,
    value: PatchedIncomingMessage,
  });
};

const parseSseMessages = (body: string): Array<Record<string, unknown>> => {
  const messages: Array<Record<string, unknown>> = [];
  const normalizedBody = body.replace(/\r\n/g, "\n");
  const events = normalizedBody.split("\n\n").map((chunk) => chunk.trim()).filter(Boolean);
  for (const event of events) {
    const dataLines = event
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice(5).trimStart());
    if (!dataLines.length) {
      continue;
    }
    const payload = dataLines.join("\n").trim();
    if (!payload) {
      continue;
    }
    messages.push(JSON.parse(payload));
  }
  return messages;
};

const normalizeSsePayload = (payload: string): string => {
  const trimmed = payload.trim();
  if (!trimmed) {
    return payload;
  }
  if (trimmed.includes("event:") || trimmed.includes("data:")) {
    return payload;
  }
  if (!/^\d+(,\d+)*$/.test(trimmed)) {
    return payload;
  }
  const bytes = Uint8Array.from(trimmed.split(",").map((value) => Number(value)));
  return new TextDecoder().decode(bytes);
};

const findStructuredTextBlock = (
  content: Array<{ type?: string; text?: string }> | undefined,
  key: string
): string => {
  const blocks = content ?? [];
  for (const block of blocks) {
    if (block?.type !== "text" || typeof block.text !== "string") {
      continue;
    }
    if (block.text.includes(`${key}:`)) {
      return block.text;
    }
  }
  return "";
};

const parseJsonTextBlock = (
  content: Array<{ type?: string; text?: string; mimeType?: string }> | undefined
): Record<string, unknown> => {
  const blocks = content ?? [];
  for (const block of blocks) {
    if (block?.type !== "text" || typeof block.text !== "string") {
      continue;
    }
    if (block.mimeType === "application/json" || block.text.trim().startsWith("{")) {
      return JSON.parse(block.text) as Record<string, unknown>;
    }
  }
  throw new Error("Missing JSON text block.");
};

const readSseResponse = async (
  response: Response
): Promise<{ messages: Array<Record<string, unknown>>; buffer: string }> => {
  if (!response.body) {
    return { messages: [], buffer: "" };
  }
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";
  while (true) {
    const { value, done } = await reader.read();
    if (done) {
      break;
    }
    buffer += decoder.decode(value, { stream: true });
    const normalized = normalizeSsePayload(buffer);
    const messages = parseSseMessages(normalized);
    if (messages.length) {
      await reader.cancel();
      return { messages, buffer: normalized };
    }
  }
  const normalized = normalizeSsePayload(buffer);
  return { messages: parseSseMessages(normalized), buffer: normalized };
};

let requestId = 0;

const callMcp = async (
  method: string,
  params: Record<string, unknown>,
  options?: { headers?: Record<string, string>; url?: string }
): Promise<Record<string, unknown>> => {
  requestId += 1;
  const headers: Record<string, string> = {
    accept: "application/json, text/event-stream",
    "content-type": "application/json",
  };
  if (options?.headers) {
    Object.assign(headers, options.headers);
  }
  if (method !== "initialize") {
    headers["mcp-protocol-version"] = MCP_PROTOCOL_VERSION;
  }
  const request = new Request(options?.url ?? MCP_URL, {
    method: "POST",
    headers,
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: requestId,
      method,
      params,
    }),
  });
  if (!postHandler) {
    throw new Error("MCP handler not initialized.");
  }
  const response = await postHandler(request);
  const contentType = response.headers.get("content-type") ?? "";
  const { messages, buffer } = contentType.includes("application/json")
    ? { messages: [await response.json()], buffer: "" }
    : await readSseResponse(response);
  const message = messages.find((msg) => msg.id === requestId) ?? messages[0];
  if (!message) {
    const statusLine = `status=${response.status} content-type=${contentType || "none"}`;
    const payload = buffer ? ` buffer=${JSON.stringify(buffer)}` : "";
    throw new Error(`Missing MCP response (${statusLine}).${payload}`);
  }
  if ("error" in message && message.error) {
    throw new Error(JSON.stringify(message.error));
  }
  return (message.result as Record<string, unknown>) ?? message;
};

const verboseToolArgs = (args: Record<string, unknown> = {}): Record<string, unknown> => ({
  format: "json",
  detail: "full",
  ...args,
});

describe("MCP API", () => {
  let originalSetInterval: typeof setInterval;
  let originalClearInterval: typeof clearInterval;

  beforeAll(async () => {
    ensureRawHeaders();
    originalSetInterval = global.setInterval;
    originalClearInterval = global.clearInterval;
    global.setInterval = ((..._args: Parameters<typeof setInterval>) => 0) as typeof setInterval;
    global.clearInterval = ((..._args: Parameters<typeof clearInterval>) => {}) as typeof clearInterval;
    const { POST } = await import("@/app/api/[transport]/route");
    postHandler = POST;
  });

  beforeEach(() => {
    process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = "false";
    delete process.env.POKECRYSTAL_MCP_TOKEN;
    sessionTesting.clearSessions();
    jest.clearAllMocks();
  });

  afterEach(() => {
    sessionTesting.clearSessions();
  });

  afterAll(() => {
    if (originalRequireSessionSecret === undefined) {
      delete process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
    } else {
      process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = originalRequireSessionSecret;
    }
    if (originalMcpToken === undefined) {
      delete process.env.POKECRYSTAL_MCP_TOKEN;
    } else {
      process.env.POKECRYSTAL_MCP_TOKEN = originalMcpToken;
    }
    global.setInterval = originalSetInterval;
    global.clearInterval = originalClearInterval;
  });

  it("initialize returns capabilities", async () => {
    const result = await callMcp("initialize", {
      protocolVersion: MCP_PROTOCOL_VERSION,
      clientInfo: { name: "jest", version: "0.0.0" },
      capabilities: {},
    });
    expect(result).toHaveProperty("capabilities");
  });

  it("parses SSE data lines with or without a trailing space after data:", () => {
    const message = JSON.stringify({ jsonrpc: "2.0", id: 1, result: { ok: true } });
    const parsedWithoutSpace = parseSseMessages(`event: message\ndata:${message}\n\n`);
    const parsedWithSpace = parseSseMessages(`event: message\ndata: ${message}\n\n`);
    expect(parsedWithoutSpace).toHaveLength(1);
    expect(parsedWithSpace).toHaveLength(1);
    expect((parsedWithoutSpace[0] as { id?: number }).id).toBe(1);
    expect((parsedWithSpace[0] as { id?: number }).id).toBe(1);
  });

  it("defaults observe to text plus structured json metadata", async () => {
    const result = await callMcp("tools/call", {
      name: "observe",
      arguments: {},
    });
    const content =
      (result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    expect(content).toHaveLength(2);
    expect(typeof content[0]?.text).toBe("string");
    expect((content[0]?.text ?? "").length).toBeGreaterThan(0);
    expect(parseJsonTextBlock(content)).toEqual(
      expect.objectContaining({
        ctx: expect.any(Object),
      })
    );
  });

  it("defaults action tools to structured json without snapshot text", async () => {
    const result = await callMcp("tools/call", {
      name: "move",
      arguments: { direction: "down" },
    });
    const content =
      (result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    expect(content).toHaveLength(1);
    expect(parseJsonTextBlock(content)).toEqual(
      expect.objectContaining({
        action: expect.objectContaining({
          ok: expect.any(Boolean),
          changed: expect.any(Boolean),
          effect: expect.any(String),
        }),
        context: expect.any(Object),
      })
    );
  });

  it("defaults status to structured json", async () => {
    const result = await callMcp("tools/call", {
      name: "status",
      arguments: {},
    });
    const content =
      (result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    expect(parseJsonTextBlock(content)).toEqual(
      expect.objectContaining({
        mode: expect.any(String),
      })
    );
  });

  it("observe returns text content", async () => {
    const result = await callMcp("tools/call", {
      name: "observe",
      arguments: verboseToolArgs(),
    });
    const content = (result.content as Array<{ type?: string; text?: string }> | undefined) ?? [];
    expect(content[0]?.type).toBe("text");
    expect(typeof content[0]?.text).toBe("string");
    const payload = parseJsonTextBlock(
      result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined
    );
    expect(payload.ctx).toEqual(
      expect.objectContaining({
        m: expect.any(String),
        dir: expect.any(String),
        xy: expect.any(Array),
      })
    );
  });

  it("observe reuses cached snapshot fields across consecutive same-frame calls", async () => {
    let now = 10_000;
    const dateNowSpy = jest.spyOn(Date, "now").mockImplementation(() => {
      now += 17;
      return now;
    });
    try {
      const first = await callMcp("tools/call", {
        name: "observe",
        arguments: verboseToolArgs(),
      });
      const second = await callMcp("tools/call", {
        name: "observe",
        arguments: verboseToolArgs(),
      });

      const firstContent =
        (first.content as Array<{ type?: string; text?: string }> | undefined) ?? [];
      const secondContent =
        (second.content as Array<{ type?: string; text?: string }> | undefined) ?? [];

      const firstPayload = JSON.stringify(parseJsonTextBlock(firstContent));
      const secondPayload = JSON.stringify(parseJsonTextBlock(secondContent));

      expect(firstPayload).toBe(secondPayload);
    } finally {
      dateNowSpy.mockRestore();
    }
  });

  it("observe can return both text and image content", async () => {
    const result = await callMcp("tools/call", {
      name: "observe",
      arguments: verboseToolArgs({ include_image: true, image_scale: 1 }),
    });
    const content = (result.content as Array<{ type?: string; text?: string; data?: string }> | undefined) ?? [];
    expect(content.some((item) => item.type === "text" && typeof item.text === "string")).toBe(true);
    expect(content.some((item) => item.type === "image" && typeof item.data === "string")).toBe(true);
  });

  it("move and press include structured context and action payloads", async () => {
    const moveResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "down" }),
    });
    const moveContent =
      (moveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const movePayload = parseJsonTextBlock(moveContent);
    expect(movePayload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({
          mode: expect.any(String),
          facing: expect.any(String),
        }),
        action: expect.objectContaining({
          ok: expect.any(Boolean),
          changed: expect.any(Boolean),
          effect: expect.any(String),
        }),
      })
    );

    const pressResult = await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: "a" }),
    });
    const pressContent =
      (pressResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const pressPayload = parseJsonTextBlock(pressContent);
    expect(pressPayload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({
          mode: expect.any(String),
          facing: expect.any(String),
        }),
        action: expect.objectContaining({
          ok: expect.any(Boolean),
          changed: expect.any(Boolean),
          effect: expect.any(String),
        }),
      })
    );
  });

  it("allows move to drive battle menu D-pad input in all four directions", async () => {
    const gameModule = jest.requireMock("@/app/game") as { Game: { create: jest.Mock } };
    let cursor = 0;
    const labels = ["FIGHT", "PKMN", "PACK", "RUN"];
    const postEvent = jest.fn((event: { type?: string; direction?: string; is_press?: boolean }) => {
      if (event.type !== "keydown" || !event.is_press) {
        return;
      }
      if (event.direction === "right") {
        cursor = 1;
      } else if (event.direction === "left") {
        cursor = 0;
      } else if (event.direction === "down") {
        cursor = 2;
      } else if (event.direction === "up") {
        cursor = 0;
      }
    });
    const state = {
      wram: {
        player_x: 5,
        player_y: 3,
        wXCoord: 5,
        wYCoord: 3,
        wMapGroup: 2,
        wMapNumber: 1,
      },
      sram: { badges: { johto: [], kanto: [] }, party: { pokemon: [] } },
    };
    const overworld = {
      player_direction: "down",
      is_moving: false,
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    gameModule.Game.create.mockImplementationOnce(async (ui: { renderSnapshot?: (...args: unknown[]) => void }) => ({
      draw: jest.fn(() => {
        ui.renderSnapshot?.(
          ["BATTLE", `>${labels[cursor]}`],
          ["STATE: battle"],
          "Battle",
          "Legend",
          labels.map((label, index) => `${index === cursor ? "▶" : " "} ${label}`),
          null,
          null
        );
      }),
      tick: jest.fn(),
      postEvent,
      isMenuOpen: jest.fn(() => false),
      isBattleActive: jest.fn(() => true),
      getMapName: jest.fn(() => "BATTLE"),
      getGameState: jest.fn(() => state),
      getOverworld: jest.fn(() => overworld),
      getBattle: jest.fn(() => ({ context: { currentState: "PLAYER_ACTION_SELECT" } })),
    }));

    for (const direction of ["right", "left", "down", "up"] as const) {
      const result = await callMcp("tools/call", {
        name: "move",
        arguments: verboseToolArgs({ direction }),
      });
      const payload = parseJsonTextBlock(
        result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined
      );
      expect(payload).toEqual(
        expect.objectContaining({
          context: expect.objectContaining({ mode: "battle" }),
          action: expect.not.objectContaining({ reason: "battle" }),
        })
      );
    }

    const keydownDirections = postEvent.mock.calls
      .map(([event]) => event)
      .filter((event) => event.type === "keydown" && event.is_press)
      .map((event) => event.direction);
    expect(keydownDirections).toEqual(["right", "left", "down", "up"]);

    postEvent.mockClear();
    const macroResult = await callMcp("tools/call", {
      name: "execute_macro",
      arguments: verboseToolArgs({
        actions: [
          { type: "move", value: "right" },
          { type: "move", value: "down" },
          { type: "button", value: "a" },
        ],
      }),
    });
    const macroPayload = parseJsonTextBlock(
      macroResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined
    );
    expect(macroPayload).toEqual(
      expect.objectContaining({
        available: false,
        context: expect.objectContaining({ mode: "battle" }),
        error: expect.objectContaining({
          code: "tool_not_available",
          reason: "battle",
          tool: "execute_macro",
        }),
      })
    );
    expect(
      postEvent.mock.calls
        .map(([event]) => event)
        .filter((event) => event.type === "keydown" && event.is_press)
    ).toEqual([]);
  });

  it("allows move to drive prompt choice D-pad input", async () => {
    const gameModule = jest.requireMock("@/app/game") as { Game: { create: jest.Mock } };
    let promptSelection = 0;
    const postEvent = jest.fn((event: { type?: string; direction?: string; is_press?: boolean }) => {
      if (event.type !== "keydown" || !event.is_press) {
        return;
      }
      if (event.direction === "down") {
        promptSelection = 1;
      } else if (event.direction === "up") {
        promptSelection = 0;
      }
    });
    const state = {
      wram: {
        player_x: 5,
        player_y: 3,
        wXCoord: 5,
        wYCoord: 3,
        wMapGroup: 2,
        wMapNumber: 1,
      },
      sram: { badges: { johto: [], kanto: [] }, party: { pokemon: [] } },
    };
    const overworld = {
      player_direction: "down",
      is_moving: false,
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    gameModule.Game.create.mockImplementationOnce(async (ui: { renderSnapshot?: (...args: unknown[]) => void }) => ({
      draw: jest.fn(() => {
        ui.renderSnapshot?.(
          ["OVERWORLD"],
          ["STATE: overworld"],
          "Overworld",
          "Legend",
          null,
          [`Is it DST?`, `${promptSelection === 0 ? "▶" : " "} YES`, `${promptSelection === 1 ? "▶" : " "} NO`],
          null
        );
      }),
      tick: jest.fn(),
      postEvent,
      isMenuOpen: jest.fn(() => false),
      isBattleActive: jest.fn(() => false),
      getMapName: jest.fn(() => "PlayersHouse1F"),
      getGameState: jest.fn(() => state),
      getOverworld: jest.fn(() => overworld),
    }));

    const result = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "down" }),
    });
    const payload = parseJsonTextBlock(
      result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined
    );
    expect(payload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({ promptPending: 1 }),
        action: expect.not.objectContaining({ reason: "dialogue" }),
      })
    );
    expect(
      postEvent.mock.calls
        .map(([event]) => event)
        .filter((event) => event.type === "keydown" && event.is_press)
        .map((event) => event.direction)
    ).toEqual(["down"]);
  });

  it("allows move and explicit macros to drive regular menus such as Pokedex and Pokegear", async () => {
    const gameModule = jest.requireMock("@/app/game") as { Game: { create: jest.Mock } };
    let menuCursor = 0;
    const menuItems = ["POKEDEX", "POKEMON", "PACK", "POKEGEAR"];
    const postEvent = jest.fn((event: { type?: string; direction?: string; button?: string; is_press?: boolean }) => {
      if (event.type !== "keydown" || !event.is_press) {
        return;
      }
      if (event.direction === "down") {
        menuCursor = Math.min(menuItems.length - 1, menuCursor + 1);
      } else if (event.direction === "up") {
        menuCursor = Math.max(0, menuCursor - 1);
      }
    });
    const state = {
      wram: {
        player_x: 5,
        player_y: 3,
        wXCoord: 5,
        wYCoord: 3,
        wMapGroup: 2,
        wMapNumber: 1,
      },
      sram: { badges: { johto: [], kanto: [] }, party: { pokemon: [] } },
    };
    const overworld = {
      player_direction: "down",
      is_moving: false,
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    gameModule.Game.create.mockImplementationOnce(async (ui: { renderSnapshot?: (...args: unknown[]) => void }) => ({
      draw: jest.fn(() => {
        ui.renderSnapshot?.(
          ["OVERWORLD"],
          ["STATE: menu"],
          "Overworld",
          "Legend",
          menuItems.map((item, index) => `${index === menuCursor ? "▶" : " "} ${item}`),
          null,
          null
        );
      }),
      tick: jest.fn(),
      postEvent,
      isMenuOpen: jest.fn(() => true),
      isBattleActive: jest.fn(() => false),
      getMapName: jest.fn(() => "PlayersHouse1F"),
      getGameState: jest.fn(() => state),
      getOverworld: jest.fn(() => overworld),
    }));

    const moveResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "down" }),
    });
    const movePayload = parseJsonTextBlock(
      moveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined
    );
    expect(movePayload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({
          mode: "menu",
          surface: expect.objectContaining({ selected: "POKEMON" }),
        }),
        action: expect.not.objectContaining({ reason: "menu" }),
      })
    );

    postEvent.mockClear();
    const macroResult = await callMcp("tools/call", {
      name: "execute_macro",
      arguments: verboseToolArgs({
        actions: [
          { type: "move", value: "down" },
          { type: "move", value: "down" },
          { type: "button", value: "a" },
        ],
      }),
    });
    const macroPayload = parseJsonTextBlock(
      macroResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined
    );
    expect(macroPayload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({
          mode: "menu",
          surface: expect.objectContaining({ selected: "POKEGEAR" }),
        }),
        action: expect.not.objectContaining({ reason: "menu" }),
      })
    );
    expect(
      postEvent.mock.calls
        .map(([event]) => event)
        .filter((event) => event.type === "keydown" && event.is_press)
        .map((event) => event.direction ?? event.button)
    ).toEqual(["down", "down", "a"]);
  });

  it("opens then closes menu via MCP calls and allows movement afterward", async () => {
    const gameModule = jest.requireMock("@/app/game") as { Game: { create: jest.Mock } };
    let menuOpen = false;
    let menuCursor = 0;
    let pendingOpenFrames = 0;
    let pendingCloseFrames = 0;
    let pendingDirection: "up" | "down" | "left" | "right" | null = null;
    const menuItems = ["POKEDEX", "POKEMON", "PACK"];
    const renderMenuLines = (): string[] =>
      menuItems.map((item, index) => `${index === menuCursor ? "▶" : " "} ${item}`);
    const state = {
      wram: {
        player_x: 5,
        player_y: 3,
        wXCoord: 5,
        wYCoord: 3,
        wMapGroup: 2,
        wMapNumber: 1,
      },
      sram: {
        badges: { johto: [true, false], kanto: [] },
        party: { pokemon: [null, null, null, null, null, null] },
      },
    };
    const overworld = {
      player_direction: "down",
      is_moving: false,
      script_runner: null as { is_busy?: boolean } | null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    gameModule.Game.create.mockImplementationOnce(async (ui: { renderSnapshot?: (...args: unknown[]) => void }) => ({
      draw: jest.fn(() => {
        ui.renderSnapshot?.(
          ["OVERWORLD"],
          ["D-PAD=Move A=Talk START=Menu"],
          "Overworld",
          "Legend",
          menuOpen ? renderMenuLines() : null,
          null,
          null
        );
      }),
      tick: jest.fn(() => {
        if (pendingOpenFrames > 0) {
          pendingOpenFrames -= 1;
          if (pendingOpenFrames === 0) {
            menuOpen = true;
          }
        }
        if (pendingCloseFrames > 0) {
          pendingCloseFrames -= 1;
          if (pendingCloseFrames === 0) {
            menuOpen = false;
          }
        }
        if (pendingDirection && !menuOpen) {
          if (pendingDirection === "up") {
            state.wram.player_y -= 1;
            state.wram.wYCoord -= 1;
          } else if (pendingDirection === "down") {
            state.wram.player_y += 1;
            state.wram.wYCoord += 1;
          } else if (pendingDirection === "left") {
            state.wram.player_x -= 1;
            state.wram.wXCoord -= 1;
          } else if (pendingDirection === "right") {
            state.wram.player_x += 1;
            state.wram.wXCoord += 1;
          }
          pendingDirection = null;
          overworld.is_moving = false;
        }
      }),
      postEvent: jest.fn((event: { type?: string; button?: string; direction?: string; is_press?: boolean }) => {
        if (event.type !== "keydown" || !event.is_press) {
          return;
        }
        if (event.direction && menuOpen) {
          if (event.direction === "down") {
            menuCursor = Math.min(menuItems.length - 1, menuCursor + 1);
          } else if (event.direction === "up") {
            menuCursor = Math.max(0, menuCursor - 1);
          }
          return;
        }
        if (event.direction && !menuOpen) {
          pendingDirection = event.direction as "up" | "down" | "left" | "right";
          overworld.is_moving = true;
          overworld.player_direction = pendingDirection;
          return;
        }
        if (event.button === "start") {
          if (!menuOpen && pendingOpenFrames === 0) {
            pendingOpenFrames = 2;
            return;
          }
          if (menuOpen && pendingCloseFrames === 0) {
            pendingCloseFrames = 3;
          }
          return;
        }
        if (event.button === "b" && menuOpen && pendingCloseFrames === 0) {
          pendingCloseFrames = 3;
        }
      }),
      isMenuOpen: jest.fn(() => menuOpen),
      isBattleActive: jest.fn(() => false),
      getMapName: jest.fn(() => "PlayersHouse1F"),
      getGameState: jest.fn(() => state),
      getOverworld: jest.fn(() => overworld),
    }));

    await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: "start" }),
    });
    const menuStatusResult = await callMcp("tools/call", {
      name: "status",
      arguments: verboseToolArgs(),
    });
    const menuStatusContent =
      (menuStatusResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const menuStatus = parseJsonTextBlock(menuStatusContent);
    expect(menuStatus).toEqual(
      expect.objectContaining({
        mode: "menu",
        inMenu: true,
      })
    );

    const menuObserveResult = await callMcp("tools/call", {
      name: "observe",
      arguments: verboseToolArgs(),
    });
    const menuObserveContent =
      (menuObserveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const menuObservePayload = parseJsonTextBlock(menuObserveContent);
    expect(menuObservePayload.ctx).toEqual(
      expect.objectContaining({
        m: "menu",
        menu: 1,
      })
    );

    const navigateResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "down", times: 1 }),
    });
    const navigateContent =
      (navigateResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const navigatePayload = parseJsonTextBlock(navigateContent);
    expect(navigatePayload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({
          mode: "menu",
          coords: [5, 3],
          surface: expect.objectContaining({
            selected: "POKEMON",
          }),
        }),
        action: expect.objectContaining({
          ok: true,
          changed: true,
        }),
      })
    );

    const closeResult = await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: "b" }),
    });
    const closeContent =
      (closeResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const closeAction = parseJsonTextBlock(closeContent);
    expect(closeAction).toEqual(
      expect.objectContaining({
        action: expect.not.objectContaining({
          reason: "menu",
        }),
      })
    );

    const postCloseStatusResult = await callMcp("tools/call", {
      name: "status",
      arguments: verboseToolArgs(),
    });
    const postCloseStatusContent =
      (postCloseStatusResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const postCloseStatus = parseJsonTextBlock(postCloseStatusContent);
    expect(postCloseStatus).toEqual(
      expect.objectContaining({
        mode: "overworld",
      })
    );
    expect(postCloseStatus).not.toHaveProperty("inMenu");

    const moveResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "right", times: 1 }),
    });
    const moveContent =
      (moveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const movePayload = parseJsonTextBlock(moveContent);
    expect(movePayload).toEqual(
      expect.objectContaining({
        context: expect.objectContaining({
          mode: "overworld",
          coords: [6, 3],
        }),
        action: expect.objectContaining({
          ok: true,
          changed: true,
        }),
      })
    );
    expect(movePayload).not.toHaveProperty("context.inMenu");
  });

  it("normalizes move and press input casing/whitespace", async () => {
    const moveResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: " Right " }),
    });
    const moveContent =
      (moveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const movePayload = parseJsonTextBlock(moveContent);
    expect(movePayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));

    const pressResult = await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: " A " }),
    });
    const pressContent =
      (pressResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const pressPayload = parseJsonTextBlock(pressContent);
    expect(pressPayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));
  });

  it("normalizes hold_button input casing/whitespace", async () => {
    const holdResult = await callMcp("tools/call", {
      name: "hold_button",
      arguments: verboseToolArgs({ button: " Select ", frames: 2 }),
    });
    const holdContent =
      (holdResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const holdPayload = parseJsonTextBlock(holdContent);
    expect(holdPayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));
  });

  it("accepts numeric string counts for observe, move, and press", async () => {
    const observeResult = await callMcp("tools/call", {
      name: "observe",
      arguments: verboseToolArgs({ advance_frames: "1" }),
    });
    const observeContent =
      (observeResult.content as Array<{ type?: string; text?: string }> | undefined) ?? [];
    expect(observeContent.some((entry) => entry.type === "text")).toBe(true);

    const moveResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "up", times: "2" }),
    });
    const moveContent =
      (moveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const movePayload = parseJsonTextBlock(moveContent);
    expect(movePayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));

    const pressResult = await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: "a", times: "2" }),
    });
    const pressContent =
      (pressResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const pressPayload = parseJsonTextBlock(pressContent);
    expect(pressPayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));
  });

  it("supports repeat aliases for deterministic move/press counts", async () => {
    const moveResult = await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "left", steps: "2" }),
    });
    const moveContent =
      (moveResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const movePayload = parseJsonTextBlock(moveContent);
    expect(movePayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));

    const pressResult = await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: "a", count: "2" }),
    });
    const pressContent =
      (pressResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const pressPayload = parseJsonTextBlock(pressContent);
    expect(pressPayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));
  });

  it("accepts normalized execute_macro inputs and string numeric limits for recent_events", async () => {
    const macroResult = await callMcp("tools/call", {
      name: "execute_macro",
      arguments: verboseToolArgs({
        actions: [
          { type: "move", value: " Right ", times: "1", hold_frames: "2", delay_frames: "0" },
          { type: "button", value: " A ", times: "1", hold_frames: "2", delay_frames: "0" },
        ],
        delay_frames: "0",
      }),
    });
    const macroContent =
      (macroResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const macroPayload = parseJsonTextBlock(macroContent);
    expect(macroPayload).toEqual(expect.objectContaining({ action: expect.objectContaining({ ok: expect.any(Boolean) }) }));

    const recentResult = await callMcp("tools/call", {
      name: "recent_events",
      arguments: verboseToolArgs({ limit: "5" }),
    });
    const recentContent =
      (recentResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const recentPayload = parseJsonTextBlock(recentContent);
    expect(recentPayload).toEqual(
      expect.objectContaining({
        summary: expect.any(String),
        total: expect.any(Number),
        events: expect.any(Array),
      })
    );
  });

  it("supports built-in execute_macro advance_dialog without explicit actions", async () => {
    const macroResult = await callMcp("tools/call", {
      name: "execute_macro",
      arguments: verboseToolArgs({
        macro: " advance_dialog ",
        max_presses: "4",
        settle_frames: "1",
      }),
    });
    const macroContent =
      (macroResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const macroPayload = parseJsonTextBlock(macroContent);
    expect(macroPayload).toEqual(
      expect.objectContaining({
        action: expect.objectContaining({
          ok: expect.any(Boolean),
          events: expect.arrayContaining([expect.stringContaining("macro:advance_dialog")]),
        }),
      })
    );
  });

  it("supports built-in execute_macro interact for micro-adjust interaction retries", async () => {
    const macroResult = await callMcp("tools/call", {
      name: "execute_macro",
      arguments: verboseToolArgs({
        macro: " interact ",
        max_presses: "3",
        settle_frames: "1",
      }),
    });
    const macroContent =
      (macroResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const macroPayload = parseJsonTextBlock(macroContent);
    expect(macroPayload).toEqual(
      expect.objectContaining({
        action: expect.objectContaining({
          ok: expect.any(Boolean),
          events: expect.arrayContaining([expect.stringContaining("macro")]),
        }),
      })
    );
  });

  it("recent_events is bounded and reports truncation metadata", async () => {
    await callMcp("tools/call", {
      name: "press",
      arguments: verboseToolArgs({ button: "a", count: 3 }),
    });
    await callMcp("tools/call", {
      name: "move",
      arguments: verboseToolArgs({ direction: "right", steps: 2 }),
    });
    const recentResult = await callMcp("tools/call", {
      name: "recent_events",
      arguments: verboseToolArgs({ limit: 1 }),
    });
    const recentContent =
      (recentResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const recentPayload = parseJsonTextBlock(recentContent);
    expect(recentPayload).toEqual(
      expect.objectContaining({
        summary: expect.any(String),
        total: expect.any(Number),
        truncated: true,
        events: expect.any(Array),
      })
    );
  });

  it("status returns compact stable fields", async () => {
    await callMcp("tools/call", {
      name: "execute_macro",
      arguments: verboseToolArgs({
        actions: [
          { type: "button", value: "a", times: 1, hold_frames: 2 },
        ],
      }),
    });
    const statusResult = await callMcp("tools/call", {
      name: "status",
      arguments: verboseToolArgs(),
    });
    const statusContent =
      (statusResult.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
    const payload = parseJsonTextBlock(statusContent);
    expect(payload).toEqual(
      expect.objectContaining({
        location: expect.any(String),
        mapId: expect.any(String),
        badges: expect.any(Number),
        coords: expect.any(Array),
        partyCount: expect.any(Number),
      })
    );
  });

  it("simulates an MCP play loop with inline context for each action", async () => {
    const loopLength = 120;
    let inlineContextCount = 0;
    for (let i = 0; i < loopLength; i += 1) {
      const method = i % 4 === 0 ? "press" : "move";
      const argumentsPayload =
        method === "press"
          ? { button: i % 8 === 0 ? "a" : "b" }
          : { direction: (["up", "right", "down", "left"] as const)[i % 4] };
      const result = await callMcp("tools/call", {
        name: method,
        arguments: verboseToolArgs(argumentsPayload),
      });
      const content = (result.content as Array<{ type?: string; text?: string; mimeType?: string }> | undefined) ?? [];
      const payload = parseJsonTextBlock(content);
      if (payload.context && typeof payload.context === "object" && "facing" in payload.context) {
        inlineContextCount += 1;
      }
    }
    expect(inlineContextCount).toBe(loopLength);
  });

  it("register_identity returns playerId/token and whoami resolves from bearer token", async () => {
    const registered = await callMcp("tools/call", {
      name: "register_identity",
      arguments: { name: "TestAgent" },
    });
    const registrationText = ((registered.content as Array<{ type?: string; text?: string }> | undefined) ?? [])
      .find((entry) => entry.type === "text")?.text;
    expect(typeof registrationText).toBe("string");
    const payload = JSON.parse(registrationText ?? "{}") as { playerId?: string; token?: string };
    expect(typeof payload.playerId).toBe("string");
    expect(typeof payload.token).toBe("string");

    const whoami = await callMcp(
      "tools/call",
      {
        name: "whoami",
        arguments: {},
      },
      {
        headers: {
          authorization: `Bearer ${payload.token}`,
        },
      }
    );
    const whoamiText = ((whoami.content as Array<{ type?: string; text?: string }> | undefined) ?? [])
      .find((entry) => entry.type === "text")?.text;
    const identity = JSON.parse(whoamiText ?? "{}") as {
      playerId?: string;
      saveSlots?: { count?: number };
    };
    expect(identity.playerId).toBe(payload.playerId);
    expect(typeof identity.saveSlots?.count).toBe("number");
  });

  it("allows register_identity as bootstrap when session-secret auth is enabled and static token is not configured", async () => {
    const originalRequire = process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
    const originalToken = process.env.POKECRYSTAL_MCP_TOKEN;
    process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = "true";
    delete process.env.POKECRYSTAL_MCP_TOKEN;
    try {
      const registered = await callMcp(
        "tools/call",
        {
          name: "register_identity",
          arguments: { name: "Bootstrap" },
        },
        {
          url: `${MCP_URL}?session_id=bootstrap-session`,
        }
      );
      const registrationText = ((registered.content as Array<{ type?: string; text?: string }> | undefined) ?? [])
        .find((entry) => entry.type === "text")?.text;
      const payload = JSON.parse(registrationText ?? "{}") as { playerId?: string; token?: string };
      expect(typeof payload.playerId).toBe("string");
      expect(typeof payload.token).toBe("string");
    } finally {
      if (originalRequire === undefined) {
        delete process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
      } else {
        process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = originalRequire;
      }
      if (originalToken === undefined) {
        delete process.env.POKECRYSTAL_MCP_TOKEN;
      } else {
        process.env.POKECRYSTAL_MCP_TOKEN = originalToken;
      }
    }
  });


  it("rejects register_identity bootstrap when static MCP token auth fails", async () => {
    const originalRequire = process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
    const originalToken = process.env.POKECRYSTAL_MCP_TOKEN;
    process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = "true";
    process.env.POKECRYSTAL_MCP_TOKEN = "secret-token";
    try {
      if (!postHandler) {
        throw new Error("MCP handler not initialized.");
      }
      const response = await postHandler(
        new Request(`${MCP_URL}?session_id=bootstrap-session`, {
          method: "POST",
          headers: {
            accept: "application/json, text/event-stream",
            "content-type": "application/json",
            "mcp-protocol-version": MCP_PROTOCOL_VERSION,
          },
          body: JSON.stringify({
            jsonrpc: "2.0",
            id: 999123,
            method: "tools/call",
            params: { name: "register_identity", arguments: { name: "Bootstrap" } },
          }),
        })
      );
      expect(response.status).toBe(401);
      expect(await response.text()).toContain("Unauthorized");
    } finally {
      if (originalRequire === undefined) {
        delete process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
      } else {
        process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = originalRequire;
      }
      if (originalToken === undefined) {
        delete process.env.POKECRYSTAL_MCP_TOKEN;
      } else {
        process.env.POKECRYSTAL_MCP_TOKEN = originalToken;
      }
    }
  });
  it("still enforces identity auth for non-bootstrap tool calls", async () => {
    const originalRequire = process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
    process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = "true";
    try {
      if (!postHandler) {
        throw new Error("MCP handler not initialized.");
      }
      const response = await postHandler(
        new Request(`${MCP_URL}?session_id=bootstrap-session`, {
          method: "POST",
          headers: {
            accept: "application/json, text/event-stream",
            "content-type": "application/json",
            "mcp-protocol-version": MCP_PROTOCOL_VERSION,
          },
          body: JSON.stringify({
            jsonrpc: "2.0",
            id: 999001,
            method: "tools/call",
            params: { name: "observe", arguments: {} },
          }),
        })
      );
      expect(response.status).toBe(401);
      expect(await response.text()).toContain("Missing identity token.");
    } finally {
      if (originalRequire === undefined) {
        delete process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET;
      } else {
        process.env.POKECRYSTAL_REQUIRE_SESSION_SECRET = originalRequire;
      }
    }
  });

  it("whoami accepts x-pokecrystal-token header", async () => {
    const registered = await callMcp("tools/call", {
      name: "register_identity",
      arguments: {},
    });
    const registrationText = ((registered.content as Array<{ type?: string; text?: string }> | undefined) ?? [])
      .find((entry) => entry.type === "text")?.text;
    const payload = JSON.parse(registrationText ?? "{}") as { playerId?: string; token?: string };
    const whoami = await callMcp(
      "tools/call",
      { name: "whoami", arguments: {} },
      { headers: { "x-pokecrystal-token": payload.token ?? "" } }
    );
    const whoamiText = ((whoami.content as Array<{ type?: string; text?: string }> | undefined) ?? [])
      .find((entry) => entry.type === "text")?.text;
    const identity = JSON.parse(whoamiText ?? "{}") as { playerId?: string };
    expect(identity.playerId).toBe(payload.playerId);
  });

  it("uses session_id to scope tool calls", async () => {
    const sessionId = "jest-session";
    await callMcp(
      "tools/call",
      {
        name: "observe",
        arguments: {},
      },
      { url: `${MCP_URL}?session_id=${sessionId}` }
    );
    expect(sessionTesting.hasSession(sessionId)).toBe(true);
  });

  it("accepts encoded session_id query params from streamable-http clients", async () => {
    const sessionId = "encoded-session";
    const encodedUrl = `http://localhost/api/mcp?session_id%3D${sessionId}`;
    await callMcp(
      "tools/call",
      {
        name: "observe",
        arguments: {},
      },
      { url: encodedUrl }
    );
    expect(sessionTesting.hasSession(sessionId)).toBe(true);
  });

  it("normalizes missing streamable HTTP accept headers", async () => {
    requestId += 1;
    const request = new Request(MCP_URL, {
      method: "POST",
      headers: {
        accept: "application/json",
        "content-type": "application/json",
        "mcp-protocol-version": MCP_PROTOCOL_VERSION,
      },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: requestId,
        method: "tools/call",
        params: {
          name: "observe",
          arguments: {},
        },
      }),
    });
    if (!postHandler) throw new Error("MCP handler not initialized.");
    const response = await postHandler(request);
    expect(response.status).toBe(200);
  });
});
