jest.mock("./common", () => ({
  invalidateObserveSnapshotCache: jest.fn(),
  loadSession: jest.fn(),
  reportSnapshot: jest.fn(),
  resolveSessionId: jest.fn(() => "dev-training-session"),
  withRequestIdentity: jest.fn(async (_extra: unknown, callback: () => Promise<unknown>) => callback()),
  MAX_ADVANCE_FRAMES: 99,
}));

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { executeMacroHandler, moveHandler, pressHandler, waitHandler } from "./input";
import { loadSession, reportSnapshot } from "./common";

const mockedLoadSession = jest.mocked(loadSession);
const mockedReportSnapshot = jest.mocked(reportSnapshot);

describe("input training recording", () => {
  const originalEnv = process.env;

  beforeEach(() => {
    jest.clearAllMocks();
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  it.each([
    ["press", () => pressHandler({ button: "a", include_snapshot_text: true, detail: "compact" } as never)],
    ["move", () => moveHandler({ direction: "down", include_snapshot_text: true, detail: "compact" } as never)],
  ] as const)("returns compact battle snapshots for %s without context polling", async (kind, callHandler) => {
    const session = {
      observeText: jest.fn(() => "BATTLE\nBefore"),
      move: jest.fn(async () => ({
        result: { ok: true, changed: true, events: ["moved:down"] },
        snapshotText: "BATTLE\nAfter move",
      })),
      press: jest.fn(async () => ({
        result: { ok: true, changed: true, events: ["pressed:a:1"] },
        snapshotText: "BATTLE\nAfter press",
      })),
      playerContext: jest.fn(async () => {
        throw new Error("playerContext should not be called for compact battle snapshots");
      }),
      status: jest.fn(async () => {
        throw new Error("status should not be called for compact battle snapshots");
      }),
      recentEvents: jest.fn(async () => {
        throw new Error("recentEvents should not be called for compact battle snapshots");
      }),
      getFrameCount: jest.fn(() => 123),
    };
    mockedLoadSession.mockResolvedValue(session as never);

    const result = await callHandler();

    const textBlocks = result.content.filter((entry) => entry.type === "text").map((entry) => entry.text ?? "");
    expect(textBlocks[0]).toBe(kind === "press" ? "BATTLE\nAfter press" : "BATTLE\nAfter move");
    expect(JSON.parse(textBlocks[1])).toEqual({
      action: expect.objectContaining({ ok: true, changed: true }),
    });
    expect(session.playerContext).not.toHaveBeenCalled();
    expect(session.status).not.toHaveBeenCalled();
    expect(session.recentEvents).not.toHaveBeenCalled();
    expect(session.observeText).not.toHaveBeenCalled();
    expect(mockedReportSnapshot).not.toHaveBeenCalled();
    if (kind === "press") {
      expect(session.press).toHaveBeenCalledWith("a", 1, { settleSnapshot: false });
    } else {
      expect(session.move).toHaveBeenCalledWith("down", 1, { settleSnapshot: false });
    }
  });

  it("bundles TUI frame, status, and recent event state when requested", async () => {
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\nBefore"),
      observePayload: jest.fn(() => ({
        viewport: ["00 . @ ."],
        info: ["Legend: @=Player"],
        menu: null,
        prompt: null,
        dialogue: null,
        titles: { viewport: "Overworld", info: "Info" },
        marker: null,
        action_log: [],
        script: {},
        tasks: [],
      })),
      move: jest.fn(async () => ({
        result: { ok: true, changed: true, reason: "no_change", events: ["moved:up"] },
        snapshotText: "OVERWORLD\nAfter",
      })),
      playerContext: jest.fn(async () => ({
        facing: "up",
        coords: { x: 4, y: 6 },
        map: "NEW_BARK_TOWN",
        menu_open: false,
        dialogue_open: false,
      })),
      status: jest.fn(async () => ({
        mode: "overworld",
        map: "NEW_BARK_TOWN",
        coords: { x: 4, y: 6 },
        facing: "up",
        in_battle: false,
        in_menu: false,
        in_dialog: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "moved up",
        total: 1,
        truncated: false,
        events: [{ summary: "Moved up" }],
      })),
      getFrameCount: jest.fn(() => 456),
    } as never);

    const result = await moveHandler({ direction: "up", include_tui_state: true } as never);

    const payload = JSON.parse(result.content.find((entry) => entry.type === "text")?.text ?? "{}");
    expect(payload.tui).toEqual(
      expect.objectContaining({
        status: expect.objectContaining({ mode: "overworld", map: "NEW_BARK_TOWN" }),
        recent_events: expect.objectContaining({ recap: "moved up", total: 1 }),
        frame: expect.objectContaining({
          view: expect.objectContaining({ viewport: ["00 . @ ."] }),
          frame: 456,
        }),
        frame_id: 456,
      }),
    );
  });

  it("records browser move actions to agent-compatible training files during dev sessions", async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "input-training-"));
    process.env.NODE_ENV = "development";
    process.env.POKECRYSTAL_WEB_TRAINING_DIR = tmpDir;

    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\nBefore"),
      move: jest.fn(async () => ({
        result: { ok: true, changed: true, reason: "moved", events: ["moved:up"] },
        snapshotText: "OVERWORLD\nAfter",
      })),
      playerContext: jest.fn(async () => ({
        facing: "up",
        coords: { x: 4, y: 5 },
        map: "NEW_BARK_TOWN",
        menu_open: false,
        dialogue_open: false,
      })),
      status: jest.fn(async () => ({
        mode: "overworld",
        map: "NEW_BARK_TOWN",
        coords: { x: 4, y: 5 },
        facing: "up",
        in_battle: false,
        in_menu: false,
        in_dialog: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "moved",
        total: 1,
        truncated: false,
        events: [
          {
            summary: "Moved up",
          },
        ],
      })),
      getFrameCount: jest.fn(() => 123),
    } as never);

    await moveHandler(
      {
        direction: "up",
        include_snapshot_text: false,
      } as never,
      {
        requestInfo: { url: "http://localhost:3000/api/mcp?session_id=dev-training-session" },
      } as never
    );

    const trainingDir = path.join(tmpDir, "dev-training-session");
    const manifest = JSON.parse(fs.readFileSync(path.join(trainingDir, "manifest.json"), "utf8"));
    const episodeRows = fs
      .readFileSync(path.join(trainingDir, "episode.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));

    expect(manifest.total_turns).toBe(1);
    expect(manifest.thread_id).toBe("dev-training-session");
    expect(episodeRows[0]).toMatchObject({
      session_id: "dev-training-session",
      thread_id: "dev-training-session",
      observer_text: "OVERWORLD\nBefore",
      decision: {
        actionType: "move",
        direction: "up",
      },
      action_result: expect.stringContaining("fx: moved"),
    });
  });

  it("records honest wait actions to agent-compatible training files during dev sessions", async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "input-training-wait-"));
    process.env.NODE_ENV = "development";
    process.env.POKECRYSTAL_WEB_TRAINING_DIR = tmpDir;

    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\nBefore wait"),
      executeMacro: jest.fn(async () => ({
        result: { ok: true, changed: true, reason: "changed", events: ["macro:1/1", "trace_steps:1"] },
        snapshotText: "OVERWORLD\nAfter wait",
      })),
      playerContext: jest.fn(async () => ({
        facing: "left",
        coords: { x: 8, y: 9 },
        map: "PLAYERS_HOUSE_1F",
        menu_open: false,
        dialogue_open: false,
      })),
      status: jest.fn(async () => ({
        mode: "overworld",
        map: "PLAYERS_HOUSE_1F",
        coords: { x: 8, y: 9 },
        facing: "left",
        in_battle: false,
        in_menu: false,
        in_dialog: false,
        prompt_pending: false,
        movement_locked: true,
        script_busy: true,
        can_move: false,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "waited",
        total: 1,
        truncated: false,
        events: [
          {
            summary: "Waited for 8 frames",
          },
        ],
      })),
      getFrameCount: jest.fn(() => 456),
    } as never);

    await waitHandler(
      {
        frames: 8,
        include_snapshot_text: false,
      } as never,
      {
        requestInfo: { url: "http://localhost:3000/api/mcp?session_id=dev-training-session" },
      } as never
    );

    const trainingDir = path.join(tmpDir, "dev-training-session");
    const episodeRows = fs
      .readFileSync(path.join(trainingDir, "episode.jsonl"), "utf8")
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));

    expect(episodeRows[0]).toMatchObject({
      observer_text: "OVERWORLD\nBefore wait",
      decision: {
        actionType: "wait",
        frames: 8,
      },
    });
  });

  it("returns a tool error instead of moving during dialogue", async () => {
    const move = jest.fn();
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "ELMS LAB\nDialogue"),
      status: jest.fn(async () => ({
        mode: "overworld",
        map: "ELMS_LAB",
        in_battle: false,
        in_dialog: true,
        text_box_open: true,
        prompt_pending: false,
        text_advance_pending: true,
        can_move: false,
        input_blocked_reason: "dialogue",
      })),
      move,
    } as never);

    const response = await moveHandler({ direction: "up" } as never, {} as never);
    const payload = JSON.parse(response.content[0]?.text ?? "{}");

    expect(response.isError).toBe(true);
    expect(payload).toMatchObject({
      available: false,
      error: {
        code: "tool_not_available",
        message: "move is not available during dialogue.",
        tool: "move",
        reason: "dialogue",
      },
    });
    expect(move).not.toHaveBeenCalled();
  });

  it("returns a tool error instead of running field macros during battle", async () => {
    const executeMacro = jest.fn();
    mockedLoadSession.mockResolvedValue({
      status: jest.fn(async () => ({
        mode: "battle",
        map: "ROUTE_29",
        in_battle: true,
        in_dialog: false,
        can_move: false,
        input_blocked_reason: "battle",
      })),
      executeMacro,
    } as never);

    const response = await executeMacroHandler(
      { actions: [{ type: "move", value: "right" }] } as never,
      {} as never
    );
    const payload = JSON.parse(response.content[0]?.text ?? "{}");

    expect(response.isError).toBe(true);
    expect(payload.error).toMatchObject({
      message: "execute_macro is not available during battle.",
      reason: "battle",
    });
    expect(executeMacro).not.toHaveBeenCalled();
  });

  it("allows dialogue macros while a prompt is waiting", async () => {
    const executeNamedMacro = jest.fn(async () => ({
      result: { ok: true, changed: true, reason: "changed", events: ["macro:advance_dialog", "pressed:1/8"] },
      snapshotText: "OAK INTRO\nNext",
    }));
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OAK INTRO\nNext"),
      status: jest.fn(async () => ({
        mode: "oak_intro",
        map: "OAK INTRO",
        in_battle: false,
        in_menu: true,
        in_dialog: true,
        prompt_pending: true,
        text_advance_pending: false,
        can_move: false,
        input_blocked_reason: "oak_intro",
      })),
      executeNamedMacro,
      playerContext: jest.fn(async () => ({
        map: "OAK INTRO",
        menu_open: true,
        dialogue_open: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "advanced",
        total: 1,
        truncated: false,
        events: [{ summary: "Advanced dialogue" }],
      })),
      getFrameCount: jest.fn(() => 789),
    } as never);

    const response = await executeMacroHandler(
      { macro: "advance_dialog", max_presses: 8, settle_frames: 25 } as never,
      {} as never
    );

    expect(response.isError).toBeUndefined();
    expect(executeNamedMacro).toHaveBeenCalledWith("advance_dialog", {
      maxPresses: 8,
      settleFrames: 25,
    });
  });

  it("allows dialogue macros for Oak intro text without a prompt", async () => {
    const executeNamedMacro = jest.fn(async () => ({
      result: { ok: true, changed: true, reason: "changed", events: ["macro:advance_dialog", "pressed:25/25"] },
      snapshotText: "NAME ENTRY",
    }));
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OAK INTRO\nHello"),
      status: jest.fn(async () => ({
        mode: "oak_intro",
        map: "OAK INTRO",
        in_battle: false,
        in_menu: true,
        in_dialog: true,
        prompt_pending: false,
        text_advance_pending: true,
        textbox_open: true,
        can_move: false,
        input_blocked_reason: "oak_intro",
        surface: {
          kind: "oak_intro",
          title: "Oak Intro",
          state: "oak_intro",
          waiting: true,
          dialogue_open: true,
        },
      })),
      executeNamedMacro,
      playerContext: jest.fn(async () => ({
        map: "OAK INTRO",
        menu_open: true,
        dialogue_open: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "advanced",
        total: 1,
        truncated: false,
        events: [{ summary: "Advanced Oak intro" }],
      })),
      getFrameCount: jest.fn(() => 790),
    } as never);

    const response = await executeMacroHandler(
      { macro: "advance_dialog", max_presses: 25, settle_frames: 25 } as never,
      {} as never
    );

    expect(response.isError).toBeUndefined();
    expect(executeNamedMacro).toHaveBeenCalledWith("advance_dialog", {
      maxPresses: 25,
      settleFrames: 25,
    });
  });

  it("routes direct MCP press A through the same advance-dialog path the TUI uses", async () => {
    const press = jest.fn();
    const executeNamedMacro = jest.fn(async () => ({
      result: { ok: true, changed: true, reason: "changed", events: ["macro:advance_dialog", "pressed:1/8"] },
      snapshotText: "OAK INTRO\nNext",
    }));
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OAK INTRO\nHello"),
      status: jest.fn(async () => ({
        mode: "oak_intro",
        map: "OAK INTRO",
        in_battle: false,
        in_menu: true,
        in_dialog: true,
        prompt_pending: true,
        text_advance_pending: false,
        can_move: false,
        input_blocked_reason: "oak_intro",
        surface: {
          kind: "oak_intro",
          title: "Oak Intro",
          waiting: true,
          dialogue_open: true,
        },
      })),
      press,
      executeNamedMacro,
      playerContext: jest.fn(async () => ({
        map: "OAK INTRO",
        menu_open: true,
        dialogue_open: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "advanced",
        total: 1,
        truncated: false,
        events: [{ summary: "Advanced dialogue" }],
      })),
      getFrameCount: jest.fn(() => 791),
    } as never);

    const response = await pressHandler({ button: "a" } as never, {} as never);

    expect(response.isError).toBeUndefined();
    expect(executeNamedMacro).toHaveBeenCalledWith("advance_dialog", {
      maxPresses: 8,
      settleFrames: 25,
    });
    expect(press).not.toHaveBeenCalled();
  });

  it.each(["b", "start", "select"] as const)(
    "keeps direct MCP press %s as a raw button during dialogue like the TUI",
    async (button) => {
      const press = jest.fn(async () => ({
        result: { ok: true, changed: true, reason: "changed", events: [`pressed:${button}:1`] },
        snapshotText: "OAK INTRO\nBack",
      }));
      mockedLoadSession.mockResolvedValue({
        observeText: jest.fn(() => "OAK INTRO\nHello"),
        status: jest.fn(async () => ({
          mode: "oak_intro",
          map: "OAK INTRO",
          in_battle: false,
          in_menu: false,
          in_dialog: true,
          prompt_pending: false,
          text_advance_pending: true,
          can_move: false,
          input_blocked_reason: "dialogue",
          surface: {
            kind: "oak_intro",
            title: "Oak Intro",
            waiting: true,
            dialogue_open: true,
          },
        })),
        press,
        playerContext: jest.fn(async () => ({
          map: "OAK INTRO",
          menu_open: false,
          dialogue_open: true,
        })),
        recentEvents: jest.fn(async () => ({
          recap: "pressed",
          total: 1,
          truncated: false,
          events: [{ summary: `Pressed ${button}` }],
        })),
        getFrameCount: jest.fn(() => 794),
      } as never);

      const response = await pressHandler({ button } as never, {} as never);

      expect(response.isError).toBeUndefined();
      expect(press).toHaveBeenCalledWith(button, 1, undefined);
    }
  );

  it.each([
    ["battle", { mode: "battle", in_battle: true, in_menu: true, input_blocked_reason: "battle" }],
    ["menu", { mode: "menu", in_battle: false, in_menu: true, input_blocked_reason: "menu" }],
    ["name entry", { mode: "name_entry", in_battle: false, in_menu: true, input_blocked_reason: "name_entry" }],
  ] as const)("keeps direct MCP press A raw on %s surfaces like the TUI", async (_label, overrides) => {
    const press = jest.fn(async () => ({
      result: { ok: true, changed: true, reason: "changed", events: ["pressed:a:1"] },
      snapshotText: "AFTER",
    }));
    const executeNamedMacro = jest.fn();
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "BEFORE"),
      status: jest.fn(async () => ({
        map: "TEST",
        in_dialog: false,
        prompt_pending: false,
        text_advance_pending: false,
        can_move: false,
        ...overrides,
      })),
      press,
      executeNamedMacro,
      playerContext: jest.fn(async () => ({
        map: "TEST",
        menu_open: true,
        dialogue_open: false,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "pressed",
        total: 1,
        truncated: false,
        events: [{ summary: "Pressed A" }],
      })),
      getFrameCount: jest.fn(() => 795),
    } as never);

    const response = await pressHandler({ button: "a" } as never, {} as never);

    expect(response.isError).toBeUndefined();
    expect(press).toHaveBeenCalledWith("a", 1, undefined);
    expect(executeNamedMacro).not.toHaveBeenCalled();
  });

  it("keeps direct MCP directional presses as no-ops during plain dialogue like TUI arrows", async () => {
    const press = jest.fn();
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OAK INTRO\nHello"),
      status: jest.fn(async () => ({
        mode: "oak_intro",
        map: "OAK INTRO",
        in_battle: false,
        in_menu: false,
        in_dialog: true,
        prompt_pending: false,
        text_advance_pending: true,
        textbox_open: true,
        can_move: false,
        input_blocked_reason: "dialogue",
        surface: {
          kind: "oak_intro",
          title: "Oak Intro",
          waiting: true,
          dialogue_open: true,
        },
      })),
      press,
      playerContext: jest.fn(async () => ({
        map: "OAK INTRO",
        menu_open: false,
        dialogue_open: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "no_events",
        total: 0,
        truncated: false,
        events: [],
      })),
      getFrameCount: jest.fn(() => 792),
    } as never);

    const response = await pressHandler({ button: "down" } as never, {} as never);
    const payload = JSON.parse(response.content[0]?.text ?? "{}");

    expect(response.isError).toBeUndefined();
    expect(payload.action).toMatchObject({
      ok: true,
      changed: false,
      reason: "no_change",
    });
    expect(press).not.toHaveBeenCalled();
  });

  it("keeps direct MCP directional presses active on prompts like TUI arrows", async () => {
    const press = jest.fn(async () => ({
      result: { ok: true, changed: true, reason: "changed", events: ["pressed:down:1"] },
      snapshotText: "PROMPT\nNO",
    }));
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "PROMPT\nYES"),
      status: jest.fn(async () => ({
        mode: "overworld",
        map: "TEST",
        in_battle: false,
        in_menu: false,
        in_dialog: true,
        prompt_pending: true,
        text_advance_pending: false,
        can_move: false,
        input_blocked_reason: "prompt",
        surface: {
          kind: "yes_no_prompt",
          title: "Prompt",
          prompt_open: true,
          dialogue_open: true,
        },
      })),
      press,
      playerContext: jest.fn(async () => ({
        map: "TEST",
        menu_open: false,
        dialogue_open: true,
      })),
      recentEvents: jest.fn(async () => ({
        recap: "prompt",
        total: 1,
        truncated: false,
        events: [{ summary: "Prompt moved" }],
      })),
      getFrameCount: jest.fn(() => 793),
    } as never);

    const response = await pressHandler({ button: "down" } as never, {} as never);

    expect(response.isError).toBeUndefined();
    expect(press).toHaveBeenCalledWith("down", 1, undefined);
  });

  it("rejects dialogue macros during name entry so MCP callers cannot spin on the naming screen", async () => {
    const executeNamedMacro = jest.fn();
    mockedLoadSession.mockResolvedValue({
      status: jest.fn(async () => ({
        mode: "name_entry",
        map: "NAME ENTRY",
        in_battle: false,
        in_menu: true,
        in_dialog: false,
        prompt_pending: true,
        can_move: false,
        input_blocked_reason: "name_entry",
      })),
      executeNamedMacro,
    } as never);

    const response = await executeMacroHandler(
      { macro: "advance_dialog", max_presses: 8 } as never,
      {} as never
    );
    const payload = JSON.parse(response.content[0]?.text ?? "{}");

    expect(response.isError).toBe(true);
    expect(payload.error).toMatchObject({
      message: "execute_macro dialogue macros are not available during name entry.",
      reason: "name_entry",
    });
    expect(executeNamedMacro).not.toHaveBeenCalled();
  });

  it("still lets MCP move and press tools drive name-entry cursor cells directly", async () => {
    const move = jest.fn(async () => ({
      result: { ok: true, changed: true, events: ["moved:1"] },
      snapshotText: "NAME ENTRY\nCURSOR: row 0 col 1",
    }));
    const press = jest.fn(async () => ({
      result: { ok: true, changed: true, events: ["pressed:a:1"] },
      snapshotText: "NAME ENTRY\nNAME: A",
    }));
    const status = jest.fn(async () => ({
      mode: "name_entry",
      map: "NAME ENTRY",
      in_battle: false,
      in_menu: true,
      in_dialog: false,
      prompt_pending: true,
      can_move: false,
      input_blocked_reason: "name_entry",
    }));
    mockedLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "NAME ENTRY\nCURSOR: row 0 col 0"),
      move,
      press,
      playerContext: jest.fn(async () => ({
        facing: "down",
        map: "NAME ENTRY",
        menu_open: true,
        dialogue_open: false,
      })),
      status,
      recentEvents: jest.fn(async () => ({
        recap: "name entry",
        total: 1,
        truncated: false,
        events: [{ summary: "Name entry input" }],
      })),
      getFrameCount: jest.fn(() => 456),
    } as never);

    const moveResponse = await moveHandler({ direction: "right" } as never, {} as never);
    const pressResponse = await pressHandler({ button: "a" } as never, {} as never);

    expect(moveResponse.isError).toBeUndefined();
    expect(pressResponse.isError).toBeUndefined();
    expect(move).toHaveBeenCalledWith("right", 1, undefined);
    expect(press).toHaveBeenCalledWith("a", 1, undefined);
  });
});
