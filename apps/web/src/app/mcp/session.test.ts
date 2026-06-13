import { getMcpSession, __testing } from "./session";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { buildTextSnapshotLines } from "@pokecrystal/core/ui/text-snapshot-render";
import fs from "node:fs";
import fsPromises from "node:fs/promises";
import path from "node:path";
import * as saveModule from "@pokecrystal/core/core/save";

const removeAutosave = (sessionId: string): void => {
  const slot = path.resolve(process.cwd(), `mcp-${sessionId}-autosave.sav`);
  const runtime = path.resolve(process.cwd(), `mcp-${sessionId}-runtime.json`);
  const normalizedRuntime = path.resolve(
    process.cwd(),
    `mcp-${sessionId.replace(/(?:[-_]?runtime)+$/i, "") || sessionId}-runtime.json`
  );
  fs.rmSync(slot, { force: true });
  fs.rmSync(`${slot}.bak`, { force: true });
  fs.rmSync(runtime, { force: true });
  fs.rmSync(normalizedRuntime, { force: true });
};

describe("McpGameSession identity naming", () => {
  it("formats MCP species labels from structured Pokemon species objects", () => {
    expect(__testing.formatMcpSpeciesLabel({ id: "CYNDAQUIL" })).toBe("CYNDAQUIL");
    expect(__testing.formatMcpSpeciesLabel({ name: "TOTODILE" })).toBe("TOTODILE");
    expect(__testing.formatMcpSpeciesLabel(" CHIKORITA ")).toBe("CHIKORITA");
    expect(__testing.formatMcpSpeciesLabel({})).toBe("UNKNOWN");
  });

  it("normalizes identity names for trainer label fallback", () => {
    expect(__testing.normalizeIdentityPlayerName(" Agent-01 ")).toBe("Agent-01");
    expect(__testing.normalizeIdentityPlayerName("")).toBeNull();
    expect(__testing.normalizeIdentityPlayerName("?????")).toBeNull();
    expect(__testing.normalizeIdentityPlayerName("VeryLongTrainerName")).toBe("VeryLongTr");
  });

  it("normalizes runtime snapshot slots so runtime session ids do not double the suffix", () => {
    expect(__testing.resolveRuntimeSnapshotSlot("move-single-step-runtime")).toBe(
      path.resolve(process.cwd(), "mcp-move-single-step-runtime.json")
    );
    expect(__testing.resolveRuntimeSnapshotSlot("mcp-session")).toBe(
      path.resolve(process.cwd(), "mcp-mcp-session-runtime.json")
    );
  });

  it("falls back to the configured MCP player name when the save and identity are blank", () => {
    expect(__testing.resolveSessionPlayerName(null, null, "")).toBe("AI");
    expect(__testing.resolveSessionPlayerName(null, "?????", "?????")).toBe("AI");
  });

  it("prefers an explicit identity name over the configured MCP fallback", () => {
    expect(__testing.resolveSessionPlayerName(null, "Kris Agent", "")).toBe("Kris Agent");
  });

  it("keeps playable CLI MCP sessions instant while using one-frame input holds", () => {
    const session = getMcpSession("interactive-play-mode");
    const sessionAny = session as unknown as {
      game: {
        getGameState: () => {
          sram: { options: { no_text_scroll: boolean } };
          wram: { wOptions: number; instant_mode: boolean };
        };
      };
      holdFrames: number;
      setInteractiveMode: (interactive: boolean) => void;
    };
    const gameState = {
      sram: { options: { no_text_scroll: false } },
      wram: { wOptions: 0, instant_mode: true },
    };
    sessionAny.game = {
      getGameState: () => gameState,
    };
    sessionAny.holdFrames = 1;

    sessionAny.setInteractiveMode(true);
    expect(gameState.sram.options.no_text_scroll).toBe(true);
    expect(gameState.wram.instant_mode).toBe(true);
    expect(sessionAny.holdFrames).toBe(1);

    sessionAny.setInteractiveMode(false);
    expect(gameState.wram.instant_mode).toBe(true);
    expect(sessionAny.holdFrames).toBe(1);
  });

  it("treats Pokegear snapshots as menu input owners", () => {
    const session = getMcpSession("pokegear-menu-surface");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: (game: { isMenuOpen: () => boolean }) => boolean;
    };

    sessionAny.lastSnapshot = {
      viewport: ["POKEGEAR RADIO", "FREQ: 4.5", "STATION: NO SIGNAL"],
      info: ["L/R=Card B=Exit", "Up/Down=Tune"],
      menu: null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "Pokegear", info: "Pokegear" },
    };

    expect(__testing.isInputOwningSurfaceSnapshot(sessionAny.lastSnapshot as never)).toBe(true);
    expect(sessionAny.isMenuOpenForSession({ isMenuOpen: () => false })).toBe(true);
  });

  it("treats Fly destination snapshots as async menu input owners", () => {
    const session = getMcpSession("fly-menu-surface-input-owner");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: (game: { isMenuOpen: () => boolean }) => boolean;
    };

    sessionAny.lastSnapshot = {
      viewport: ["FLY TO WHERE?"],
      info: ["D-Pad=Move A=Select B=Back"],
      menu: ["> NEW BARK TOWN", "  CHERRYGROVE CITY"],
      prompt: null,
      dialogue: null,
      titles: { viewport: "FLY TO WHERE?", info: "Legend" },
    };

    expect(__testing.isInputOwningSurfaceSnapshot(sessionAny.lastSnapshot as never)).toBe(true);
    expect(sessionAny.isMenuOpenForSession({ isMenuOpen: () => false })).toBe(true);
  });

  it("treats PC snapshots as async menu input owners", () => {
    const session = getMcpSession("pc-menu-surface-input-owner");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: (game: { isMenuOpen: () => boolean }) => boolean;
    };

    sessionAny.lastSnapshot = {
      viewport: ["DEPOSIT <PK><MN>", "BOX 01", "  DUX", "▶ TOGEPI"],
      info: ["SELECTED: TOGEPI", "LEVEL: 5", "ITEM: -"],
      menu: null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "PC", info: "PC" },
    };

    expect(__testing.isInputOwningSurfaceSnapshot(sessionAny.lastSnapshot as never)).toBe(true);
    expect(sessionAny.isMenuOpenForSession({ isMenuOpen: () => false })).toBe(true);
  });

  it("treats PC prompt snapshots as async menu input owners", () => {
    const session = getMcpSession("pc-prompt-surface-input-owner");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: (game: { isMenuOpen: () => boolean }) => boolean;
    };

    sessionAny.lastSnapshot = {
      viewport: ["DEPOSIT #MON", "BOX 01", "  BELLSPROU", "▶ TOGEPI"],
      info: ["SELECTED: TOGEPI", "LEVEL: 5", "ITEM: -"],
      menu: null,
      prompt: ["Choose a <PK><MN>."],
      dialogue: null,
      titles: { viewport: "Prompt", info: "Legend" },
    };

    expect(__testing.isInputOwningSurfaceSnapshot(sessionAny.lastSnapshot as never)).toBe(true);
    expect(sessionAny.isMenuOpenForSession({ isMenuOpen: () => false })).toBe(true);
  });

  it("uses an unthrottled text UI so stepped frames refresh snapshots immediately", () => {
    const session = getMcpSession("unthrottled-text-ui");
    const sessionAny = session as unknown as {
      textUi: { refreshHz: number | null };
    };

    expect(sessionAny.textUi.refreshHz).toBeNull();
  });

  it("reuses a same-frame text snapshot instead of redrawing dense maps on read-only observes", () => {
    const session = getMcpSession("same-frame-snapshot-cache");
    const sessionAny = session as unknown as {
      frameCounter: number;
      lastSnapshot: unknown;
      lastSnapshotFrameCounter?: number | null;
      captureSnapshot: jest.Mock;
      observeText: () => string;
    };

    sessionAny.frameCounter = 123;
    sessionAny.lastSnapshot = {
      viewport: ["OVERWORLD", "@ . ."],
      info: ["Pos: (1,1)"],
      menu: null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "Overworld", info: "Legend" },
      marker: [1, 1, "@"],
      action_log: [],
      script: {},
      tasks: [],
    };
    sessionAny.lastSnapshotFrameCounter = 123;
    sessionAny.captureSnapshot = jest.fn();

    sessionAny.observeText();
    sessionAny.observeText();

    expect(sessionAny.captureSnapshot).not.toHaveBeenCalled();
  });

  it("suppresses downhill flags only while MCP frames tick", () => {
    const session = getMcpSession("mcp-downhill-suppressed");
    const sessionAny = session as unknown as {
      frameCounter: number;
      lastSnapshot: unknown;
      getGame: jest.Mock;
      captureSnapshot: jest.Mock;
      stepFrames: (count: number) => void;
    };
    const gameState = {
      wram: {
        wBikeFlags: 0x07,
        engine_flags: { ENGINE_DOWNHILL: true },
      },
    };
    const tick = jest.fn(() => {
      expect(gameState.wram.wBikeFlags & 0x04).toBe(0);
      expect(gameState.wram.engine_flags.ENGINE_DOWNHILL).toBe(false);
    });
    sessionAny.frameCounter = 0;
    sessionAny.lastSnapshot = {};
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.getGame = jest.fn(() => ({
      getGameState: () => gameState,
      tick,
    }));

    sessionAny.stepFrames(1);

    expect(tick).toHaveBeenCalledTimes(1);
    expect(gameState.wram.wBikeFlags).toBe(0x07);
    expect(gameState.wram.engine_flags.ENGINE_DOWNHILL).toBe(true);
  });

  it("does not allow MCP autosave while dialogue is open", async () => {
    const session = getMcpSession("autosave-dialogue-blocked");
    const writeRuntimeSnapshot = jest.fn().mockResolvedValue(undefined);
    const sessionAny = session as unknown as {
      game: unknown;
      frameCounter: number;
      autosaveLastFrame: number;
      autosaveQueue: Promise<void>;
      canPersistRealGameSave: () => boolean;
      requestAutosave: (options?: { force?: boolean }) => Promise<void>;
      writeRuntimeSnapshot: jest.Mock;
    };
    sessionAny.game = {
      getDebugStatus: () => ({
        mode: "overworld",
        can_move: false,
        prompt_pending: false,
        text_advance_pending: true,
        in_dialog: true,
        in_menu: false,
        in_battle: false,
        movement_locked: false,
        script_busy: true,
      }),
      isBattleActive: () => false,
      getOverworld: () => ({
        player_x: 9,
        player_y: 7,
        player_direction: "up",
        is_moving: false,
        dialogue: {
          visible: true,
          waiting_for_input: true,
          pending_waits: 1,
        },
        script_runner: {
          _script_stack: [{ name: "ElmsLabWalkUpToElmScript" }],
          _awaiting_resume: 1,
          stop_execution: true,
        },
        script_tasks_active: () => false,
        player_movement_locked: () => false,
        _current_tile_permission: () => 0,
      }),
      getGameState: () => ({ sram: {}, wram: {} }),
    };
    sessionAny.frameCounter = 100;
    sessionAny.autosaveLastFrame = -1;
    sessionAny.autosaveQueue = Promise.resolve();
    sessionAny.writeRuntimeSnapshot = writeRuntimeSnapshot;

    expect(sessionAny.canPersistRealGameSave()).toBe(false);

    await sessionAny.requestAutosave({ force: true });

    expect(writeRuntimeSnapshot).not.toHaveBeenCalled();
  });

  it("allows MCP autosave only from a stable saveable overworld tile", () => {
    const session = getMcpSession("autosave-stable-overworld");
    const sessionAny = session as unknown as {
      game: unknown;
      canPersistRealGameSave: () => boolean;
    };
    sessionAny.game = {
      getDebugStatus: () => ({
        mode: "overworld",
        can_move: true,
        prompt_pending: false,
        text_advance_pending: false,
        in_dialog: false,
        in_menu: false,
        in_battle: false,
        movement_locked: false,
        script_busy: false,
      }),
      isBattleActive: () => false,
      getOverworld: () => ({
        player_x: 9,
        player_y: 7,
        player_direction: "down",
        is_moving: false,
        dialogue: null,
        script_runner: {
          _script_stack: [],
          _awaiting_resume: 0,
          _queued_overworld_task_count: 0,
          stop_execution: false,
          is_busy: false,
        },
        script_tasks_active: () => false,
        player_movement_locked: () => false,
        _current_tile_permission: () => 0,
      }),
    };

    expect(sessionAny.canPersistRealGameSave()).toBe(true);
  });

  it("clears a restored active warp guard before an intentional move from the same tile", async () => {
    const session = getMcpSession("clear-active-warp-before-move");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      normalizeTimes: (n: number) => number;
      normalizeHoldFrames: (n?: number) => number;
      readMapIdentity: jest.Mock;
      readPlayerCoords: jest.Mock;
      readFacingDirection: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      waitForMovement: jest.Mock;
      captureSnapshot: jest.Mock;
      readBlockReason: jest.Mock;
      getStopReason: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      lastSnapshot: unknown;
      lastMcpMeta: unknown;
      clearActiveWarpGuardForIntentionalMove: (game: unknown) => void;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.normalizeTimes = (n: number) => n;
    sessionAny.normalizeHoldFrames = () => 1;
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "CherrygroveMart", id: "26:4" }));
    sessionAny.readPlayerCoords = jest.fn(() => [5, 15]);
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.waitForMovement = jest.fn(() => ({ moved: false, stopReason: null, blockReason: "blocked:terrain" }));
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.readBlockReason = jest.fn(() => "blocked:terrain");
    sessionAny.getStopReason = jest.fn(() => null);
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.lastSnapshot = null;

    const overworld = {
      current_map_name: "CherrygroveMart",
      player_x: 5,
      player_y: 15,
      _active_warp_tile: ["CherrygroveMart", 5, 15] as [string, number, number],
    };
    const game = {
      getOverworld: () => overworld,
      getGameState: () => ({ wram: { player_x: 5, player_y: 15 } }),
      isBattleActive: () => false,
      getMapName: () => "CherrygroveMart",
    };
    sessionAny.getGame = jest.fn(() => game);

    await session.move("down", 1);

    expect(overworld._active_warp_tile).toBeNull();
  });

  it("treats name-entry direction input as UI navigation instead of overworld movement", async () => {
    const session = getMcpSession("name-entry-move-ui-navigation");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      normalizeTimes: (n: number) => number;
      normalizeHoldFrames: (n?: number) => number;
      readMapIdentity: jest.Mock;
      readPlayerCoords: jest.Mock;
      buildStateFingerprint: jest.Mock;
      captureSceneSignal: jest.Mock;
      getStopReason: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      waitForMovement: jest.Mock;
      finalizeActionResult: jest.Mock;
      recordActionEvent: jest.Mock;
      observeText: jest.Mock;
      lastSnapshot: unknown;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.normalizeTimes = (n: number) => n;
    sessionAny.normalizeHoldFrames = () => 1;
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "NewBarkTown", id: "1:1" }));
    sessionAny.readPlayerCoords = jest.fn(() => [0, 0]);
    sessionAny.buildStateFingerprint = jest
      .fn()
      .mockReturnValueOnce("before")
      .mockReturnValue("after");
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.getStopReason = jest.fn(() => "name_entry");
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.waitForMovement = jest.fn();
    sessionAny.finalizeActionResult = jest.fn(async (result) => ({ ok: true, ...result }));
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.observeText = jest.fn(() => "NAME ENTRY");
    sessionAny.lastSnapshot = {
      viewport: ["NAME ENTRY"],
      info: ["STATE: name_entry", "CURSOR: row 0 col 0"],
      menu: null,
      prompt: null,
      dialogue: null,
    };
    sessionAny.getGame = jest.fn(() => ({
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getGameState: () => ({ wram: { player_x: 0, player_y: 0 } }),
      getOverworld: () => ({}),
      getMapName: () => "NewBarkTown",
    }));

    await session.move("right", 1);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ direction: "right" })
    );
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(2);
    expect(sessionAny.waitForMovement).not.toHaveBeenCalled();
  });

  it("clears, types lowercase text, and submits name entry through typeText inputs", async () => {
    const session = getMcpSession("name-entry-type-text-clear-submit");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      buildStateFingerprint: jest.Mock;
      captureSceneSignal: jest.Mock;
      scheduleKeyPress: jest.Mock;
      scheduleTextInput: jest.Mock;
      stepFrames: jest.Mock;
      finalizeActionResult: jest.Mock;
      recordAction: jest.Mock;
      recordActionEvent: jest.Mock;
      observeText: jest.Mock;
      lastSnapshot: unknown;
      holdFrames: number;
      typeText: (text: string, options?: { clear?: boolean; submit?: boolean }) => Promise<unknown>;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValue("after");
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "name_entry",
      menu: true,
      promptReason: "name_entry",
      dialogueText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.scheduleTextInput = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.finalizeActionResult = jest.fn(async (result) => ({ ok: true, ...result }));
    sessionAny.recordAction = jest.fn();
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.observeText = jest.fn(() => "NAME ENTRY");
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      viewport: ["NAME ENTRY"],
      info: ["STATE: name_entry", "CASE: upper", "CURSOR: row 0 col 0"],
      menu: null,
      prompt: null,
      dialogue: null,
    };
    sessionAny.getGame = jest.fn(() => ({
      getDebugStatus: () => ({ name_entry: { name: "OLD" } }),
    }));

    await sessionAny.typeText("do", { clear: true, submit: true });

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledTimes(5);
    expect(sessionAny.scheduleKeyPress.mock.calls.slice(0, 3)).toEqual([
      [expect.objectContaining({ button: "b" })],
      [expect.objectContaining({ button: "b" })],
      [expect.objectContaining({ button: "b" })],
    ]);
    expect(sessionAny.scheduleTextInput).toHaveBeenCalledWith("d");
    expect(sessionAny.scheduleTextInput).toHaveBeenCalledWith("o");
    expect(sessionAny.scheduleKeyPress.mock.calls.at(-2)?.[0]).toEqual(expect.objectContaining({ button: "start" }));
    expect(sessionAny.scheduleKeyPress.mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ button: "a" }));
    expect(sessionAny.finalizeActionResult).toHaveBeenCalledWith(
      expect.objectContaining({ events: ["deleted", "deleted", "deleted", "typed:d", "typed:o", "submitted"] })
    );
  });

  it("preserves mixed-case literal typeText input on name entry", async () => {
    const session = getMcpSession("name-entry-type-text-mixed-case");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      buildStateFingerprint: jest.Mock;
      captureSceneSignal: jest.Mock;
      scheduleTextInput: jest.Mock;
      stepFrames: jest.Mock;
      finalizeActionResult: jest.Mock;
      recordAction: jest.Mock;
      recordActionEvent: jest.Mock;
      observeText: jest.Mock;
      lastSnapshot: unknown;
      typeText: (text: string) => Promise<unknown>;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValue("after");
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "name_entry",
      menu: true,
      promptReason: "name_entry",
      dialogueText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.scheduleTextInput = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.finalizeActionResult = jest.fn(async (result) => ({ ok: true, ...result }));
    sessionAny.recordAction = jest.fn();
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.observeText = jest.fn(() => "NAME ENTRY");
    sessionAny.lastSnapshot = {
      viewport: ["NAME ENTRY"],
      info: ["STATE: name_entry", "CASE: upper", "CURSOR: row 0 col 0"],
      menu: null,
      prompt: null,
      dialogue: null,
    };
    sessionAny.getGame = jest.fn(() => ({
      getDebugStatus: () => ({ name_entry: { name: "" } }),
    }));

    await sessionAny.typeText("Ry An");

    expect(sessionAny.scheduleTextInput.mock.calls.map((call) => call[0])).toEqual(["R", "y", " ", "A", "n"]);
  });

  it("selects supported name-entry punctuation through cursor input", async () => {
    const session = getMcpSession("name-entry-type-text-punctuation");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      buildStateFingerprint: jest.Mock;
      captureSceneSignal: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      finalizeActionResult: jest.Mock;
      recordAction: jest.Mock;
      recordActionEvent: jest.Mock;
      observeText: jest.Mock;
      lastSnapshot: unknown;
      holdFrames: number;
      typeText: (text: string) => Promise<unknown>;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValue("after");
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "name_entry",
      menu: true,
      promptReason: "name_entry",
      dialogueText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.finalizeActionResult = jest.fn(async (result) => ({ ok: true, ...result }));
    sessionAny.recordAction = jest.fn();
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.observeText = jest.fn(() => "NAME ENTRY");
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      viewport: ["NAME ENTRY"],
      info: ["STATE: name_entry", "CASE: upper", "CURSOR: row 0 col 0"],
      menu: null,
      prompt: null,
      dialogue: null,
    };
    sessionAny.getGame = jest.fn(() => ({
      getDebugStatus: () => ({ name_entry: { name: "" } }),
    }));

    await sessionAny.typeText(".");

    const directions = sessionAny.scheduleKeyPress.mock.calls
      .map((call) => call[0]?.direction)
      .filter(Boolean);
    expect(directions).toEqual(["up", "up", "right", "right", "right", "right"]);
    expect(sessionAny.scheduleKeyPress.mock.calls.at(-1)?.[0]).toEqual(expect.objectContaining({ button: "a" }));
  });

  it("keeps typeText clear and submit as no-ops outside name entry", async () => {
    const session = getMcpSession("type-text-non-name-entry-options");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      buildStateFingerprint: jest.Mock;
      captureSceneSignal: jest.Mock;
      scheduleKeyPress: jest.Mock;
      scheduleTextInput: jest.Mock;
      stepFrames: jest.Mock;
      finalizeActionResult: jest.Mock;
      recordAction: jest.Mock;
      recordActionEvent: jest.Mock;
      observeText: jest.Mock;
      lastSnapshot: unknown;
      typeText: (text: string, options?: { clear?: boolean; submit?: boolean }) => Promise<unknown>;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValue("after");
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      promptReason: null,
      dialogueText: "",
      menuText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.scheduleTextInput = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.finalizeActionResult = jest.fn(async (result) => ({ ok: true, ...result }));
    sessionAny.recordAction = jest.fn();
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.observeText = jest.fn(() => "OVERWORLD");
    sessionAny.lastSnapshot = {
      viewport: ["OVERWORLD"],
      info: [],
      menu: null,
      prompt: null,
      dialogue: null,
    };
    sessionAny.getGame = jest.fn(() => ({
      getDebugStatus: () => ({ name_entry: null }),
    }));

    await sessionAny.typeText("AB", { clear: true, submit: true });

    expect(sessionAny.scheduleTextInput.mock.calls.map((call) => call[0])).toEqual(["A", "B"]);
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalled();
  });

  it("preserves a restored active warp guard outdoors so doorway exits do not re-trigger immediately", async () => {
    const session = getMcpSession("preserve-outdoor-active-warp-guard");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      normalizeTimes: (n: number) => number;
      normalizeHoldFrames: (n?: number) => number;
      readMapIdentity: jest.Mock;
      readPlayerCoords: jest.Mock;
      readFacingDirection: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      waitForMovement: jest.Mock;
      captureSnapshot: jest.Mock;
      readBlockReason: jest.Mock;
      getStopReason: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      lastSnapshot: unknown;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.normalizeTimes = (n: number) => n;
    sessionAny.normalizeHoldFrames = () => 1;
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "CherrygroveCity", id: "26:3" }));
    sessionAny.readPlayerCoords = jest.fn(() => [47, 7]);
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.waitForMovement = jest.fn(() => ({
      moved: false,
      stopReason: null,
      blockReason: "blocked:terrain",
    }));
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.readBlockReason = jest.fn(() => "blocked:terrain");
    sessionAny.getStopReason = jest.fn(() => null);
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.lastSnapshot = null;

    const overworld = {
      current_map_name: "CherrygroveCity",
      player_x: 47,
      player_y: 7,
      _active_warp_tile: ["CherrygroveCity", 47, 7] as [string, number, number],
    };
    const game = {
      getOverworld: () => overworld,
      getGameState: () => ({ wram: { player_x: 47, player_y: 7 } }),
      isBattleActive: () => false,
      getMapName: () => "CherrygroveCity",
    };
    sessionAny.getGame = jest.fn(() => game);

    await session.move("right", 1);

    expect(overworld._active_warp_tile).toEqual(["CherrygroveCity", 47, 7]);
  });

  it("preserves an indoor stair warp guard so the next move can step off the tile", async () => {
    const session = getMcpSession("preserve-indoor-stair-active-warp-guard");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      normalizeTimes: (n: number) => number;
      normalizeHoldFrames: (n?: number) => number;
      readMapIdentity: jest.Mock;
      readPlayerCoords: jest.Mock;
      readFacingDirection: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      waitForMovement: jest.Mock;
      captureSnapshot: jest.Mock;
      readBlockReason: jest.Mock;
      getStopReason: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      lastSnapshot: unknown;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.normalizeTimes = (n: number) => n;
    sessionAny.normalizeHoldFrames = () => 1;
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "CherrygrovePokecenter1F", id: "26:5" }));
    sessionAny.readPlayerCoords = jest.fn(() => [1, 15]);
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.waitForMovement = jest.fn(() => ({
      moved: false,
      stopReason: null,
      blockReason: "blocked:terrain",
    }));
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.readBlockReason = jest.fn(() => "blocked:terrain");
    sessionAny.getStopReason = jest.fn(() => null);
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.lastSnapshot = null;

    const overworld = {
      current_map_name: "CherrygrovePokecenter1F",
      player_x: 1,
      player_y: 15,
      _active_warp_tile: ["CherrygrovePokecenter1F", 1, 15] as [string, number, number],
      _warp_tile_lookup: {
        "1,15": [{ target_map_constant: "POKECENTER_2F" }],
      },
    };
    const game = {
      getOverworld: () => overworld,
      getGameState: () => ({ wram: { player_x: 1, player_y: 15 } }),
      isBattleActive: () => false,
      getMapName: () => "CherrygrovePokecenter1F",
    };
    sessionAny.getGame = jest.fn(() => game);

    await session.move("right", 1);

    expect(overworld._active_warp_tile).toEqual(["CherrygrovePokecenter1F", 1, 15]);
  });

  it("preserves the Route36RuinsOfAlphGate entry guard when moving south into the gate", async () => {
    const session = getMcpSession("preserve-route36-ruins-gate-entry-guard");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      normalizeTimes: (n: number) => number;
      normalizeHoldFrames: (n?: number) => number;
      readMapIdentity: jest.Mock;
      readPlayerCoords: jest.Mock;
      readFacingDirection: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      waitForMovement: jest.Mock;
      captureSnapshot: jest.Mock;
      readBlockReason: jest.Mock;
      getStopReason: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      lastSnapshot: unknown;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.normalizeTimes = (n: number) => n;
    sessionAny.normalizeHoldFrames = () => 1;
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "Route36RuinsOfAlphGate", id: "4:8" }));
    sessionAny.readPlayerCoords = jest.fn(() => [11, 1]);
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.waitForMovement = jest.fn(() => ({
      moved: false,
      stopReason: null,
      blockReason: "blocked:terrain",
    }));
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.readBlockReason = jest.fn(() => "blocked:terrain");
    sessionAny.getStopReason = jest.fn(() => null);
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.lastSnapshot = null;

    const overworld = {
      current_map_name: "Route36RuinsOfAlphGate",
      player_x: 11,
      player_y: 1,
      TILES_PER_COLLISION: 2,
      map: { width: 10, height: 9 },
      _active_warp_tile: ["Route36RuinsOfAlphGate", 11, 1] as [string, number, number],
      _warp_tile_lookup: {
        "11,1": [{ target_map_constant: "ROUTE_36" }],
      },
    };
    const game = {
      getOverworld: () => overworld,
      getGameState: () => ({ wram: { player_x: 11, player_y: 1 } }),
      isBattleActive: () => false,
      getMapName: () => "Route36RuinsOfAlphGate",
    };
    sessionAny.getGame = jest.fn(() => game);

    await session.move("down", 1);

    expect(overworld._active_warp_tile).toEqual(["Route36RuinsOfAlphGate", 11, 1]);
  });
});

describe("McpGameSession identity play settings", () => {
  it("loads stored player settings for MCP identities", async () => {
    const maybeSingle = jest.fn().mockResolvedValue({
      data: { player_name: "Kris Agent", player_gender: PlayerGender.FEMALE },
      error: null,
    });
    const createClient = jest.fn(() => ({
      from: jest.fn(() => ({
        select: jest.fn(() => ({
          eq: jest.fn(() => ({
            maybeSingle,
          })),
        })),
      })),
    }));

    await expect(
      __testing.loadIdentityPlayProfile("player-1", createClient as never)
    ).resolves.toEqual({
      playerName: "Kris Agent",
      playerGender: PlayerGender.FEMALE,
    });
    expect(createClient).toHaveBeenCalledTimes(1);
    expect(maybeSingle).toHaveBeenCalledTimes(1);
  });

  it("prefers stored identity gender before save gender when resolving the session sprite", () => {
    expect(
      __testing.resolveSessionPlayerGender(
        { playerName: null, playerGender: PlayerGender.FEMALE },
        {
          sram: { player_gender: PlayerGender.MALE },
          wram: { player_gender: PlayerGender.MALE },
        }
      )
    ).toBe(PlayerGender.FEMALE);
  });

  it("falls back to save gender when no stored identity settings exist", () => {
    expect(
      __testing.resolveSessionPlayerGender(null, {
        sram: { player_gender: PlayerGender.FEMALE },
        wram: { player_gender: PlayerGender.MALE },
      })
    ).toBe(PlayerGender.FEMALE);
  });
});

describe("McpGameSession waitForPrompt", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  const buildSession = () => {
    const session = getMcpSession("wait-prompt-test");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      stepFrames: jest.Mock;
      lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null } | null;
      frameLimiter: { consume: jest.Mock };
      maxFramesPerCall: number;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.frameLimiter = { consume: jest.fn() };
    return { session, sessionAny };
  };

  it("returns immediately when a prompt is already visible", async () => {
    const { session, sessionAny } = buildSession();
    sessionAny.lastSnapshot = { prompt: ["OK"], dialogue: null };
    sessionAny.stepFrames = jest.fn();

    await expect(session.waitForPrompt(1)).resolves.toBe("OK");
    expect(sessionAny.stepFrames).not.toHaveBeenCalled();
  });

  it("throws after exceeding the frame budget without a prompt", async () => {
    const { session, sessionAny } = buildSession();
    sessionAny.maxFramesPerCall = 3;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null };

    let steps = 0;
    sessionAny.stepFrames = jest.fn(() => {
      steps += 1;
      if (steps > 10) {
        throw new Error("stepFrames overflow");
      }
    });

    await expect(session.waitForPrompt(1000)).rejects.toThrow(
      "Wait for prompt exceeded 3 frames."
    );
    expect(sessionAny.stepFrames).toHaveBeenCalledTimes(3);
  });
});

const buildMoveHarness = () => {
  const session = getMcpSession("move-test");
  const sessionAny = session as unknown as {
    ensureReady: jest.Mock;
    observeText: jest.Mock;
    actionLimiter: { consume: jest.Mock };
    frameLimiter: { consume: jest.Mock };
    holdFrames: number;
    maxFramesPerCall: number;
    lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
    scheduleKeyPress: jest.Mock;
    stepFrames: jest.Mock;
    game: {
      getGameState: () => { wram: { player_x: number; player_y: number } };
      getMapName: () => string;
      isMenuOpen: () => boolean;
      isBattleActive: () => boolean;
      getOverworld: () => {
        is_moving: boolean;
        player_direction?: string;
        script_runner: { is_busy?: boolean } | null;
        player_movement_locked: () => boolean;
        script_tasks_active: () => boolean;
        _last_block_feedback?: { reason?: string } | null;
      };
    };
    getGame: jest.Mock;
    requestAutosave: jest.Mock;
    scheduledEvents?: unknown[];
  };
  sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
  sessionAny.observeText = jest.fn().mockReturnValue("OK");
  sessionAny.actionLimiter = { consume: jest.fn() };
  sessionAny.frameLimiter = { consume: jest.fn() };
  sessionAny.holdFrames = 1;
  sessionAny.maxFramesPerCall = 10;
  sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
  const gameState = { wram: { player_x: 1, player_y: 1 } };
  const overworld = {
    is_moving: false,
    player_direction: "up",
    script_runner: null,
    player_movement_locked: jest.fn(() => false),
    script_tasks_active: jest.fn(() => false),
    _last_block_feedback: null,
  };
  const game = {
    getGameState: () => gameState,
    getMapName: () => "TestMap",
    isMenuOpen: () => false,
    isBattleActive: () => false,
    getOverworld: () => overworld,
  };
  sessionAny.game = game;
  sessionAny.getGame = jest.fn(() => game);
  sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
  return { session, sessionAny, gameState, overworld };
};

const installMoveStepper = (
  sessionAny: {
    scheduleKeyPress: jest.Mock;
    stepFrames: jest.Mock;
  },
  gameState: { wram: { player_x: number; player_y: number } },
  overworld: { is_moving: boolean }
) => {
  const pendingMoves: string[] = [];
  sessionAny.scheduleKeyPress = jest.fn(({ direction }: { direction?: string }) => {
    if (direction) {
      pendingMoves.push(direction);
      overworld.is_moving = true;
    }
  });
  sessionAny.stepFrames = jest.fn((count: number) => {
    for (let i = 0; i < count; i += 1) {
      if (!pendingMoves.length) {
        continue;
      }
      const direction = pendingMoves.shift();
      if (!direction) {
        continue;
      }
      switch (direction) {
        case "up":
          gameState.wram.player_y -= 1;
          break;
        case "down":
          gameState.wram.player_y += 1;
          break;
        case "left":
          gameState.wram.player_x -= 1;
          break;
        case "right":
          gameState.wram.player_x += 1;
          break;
      }
      overworld.is_moving = false;
    }
  });
};

describe("McpGameSession move", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it("returns a summary line with completed steps", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    installMoveStepper(sessionAny, gameState, overworld);

    const action = await session.move("right", 2);

    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
    expect(JSON.stringify(action.result.events ?? [])).toContain("moved:2");
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledTimes(2);
  });

  it("holds directional movement long enough to complete a turn before release", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    installMoveStepper(sessionAny, gameState, overworld);
    overworld.player_direction = "down";

    const action = await session.move("left", 1);

    expect(action.result.ok).toBe(true);
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({
        direction: "left",
        holdFrames: 2,
      }),
    );
  });

  it("clears stale held and queued directions before discrete movement", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    installMoveStepper(sessionAny, gameState, overworld);
    const heldDirections = new Map<string, null>([["up", null]]);
    const overworldWithInputState = overworld as typeof overworld & {
      _held_directions: Map<string, null>;
      _queued_direction: string | null;
    };
    overworldWithInputState._held_directions = heldDirections;
    overworldWithInputState._queued_direction = "up";
    sessionAny.scheduledEvents = [{ frame: 100, event: { direction: "up" } }];

    const action = await session.move("down", 1);

    expect(action.result.ok).toBe(true);
    expect(action.result.events).toContain("stale_input_cleared");
    expect(action.result.events).toContain("moved:1");
    expect(gameState.wram.player_x).toBe(1);
    expect(gameState.wram.player_y).toBe(2);
    expect(heldDirections.size).toBe(0);
    expect(overworldWithInputState._queued_direction).toBeNull();
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ direction: "down" }),
    );
  });

  it("uses a single-tile directional tap in the live runtime", async () => {
    const sessionId = "move-single-step-runtime";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);

    const before = await session.status();
    const action = await session.move("down", 1);
    const after = await session.status();

    expect(action.result.ok).toBe(true);
    expect(before.coords?.x).toBe(after.coords?.x);
    expect((after.coords?.y ?? 0) - (before.coords?.y ?? 0)).toBe(2);
  });

  it("keeps the player marker visible in observeText after live runtime movement", async () => {
    const sessionId = "move-visible-player-runtime";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);

    await session.status();
    await session.move("down", 1);
    const rendered = session.observeText();

    expect(rendered).toContain("@");
  });

  it("allows pressing A in the live runtime without crashing interaction checks", async () => {
    const sessionId = "elms-lab-a-press-runtime";
    removeAutosave(sessionId);
    const session = getMcpSession(sessionId);

    await session.status();
    await session.move("down", 1);
    await session.move("right", 1);
    const action = await session.press("a", 1);
    const after = await session.status();

    expect(action.result.ok).toBe(true);
    expect(after.map).toBe("PlayersHouse2F");
  });

  it("settles one extra frame so A can surface a prompt after dialogue", async () => {
    const { session, sessionAny } = buildMoveHarness();
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: ["ELM: You'll take CYNDAQUIL, the fire POKeMON?"],
      menu: null,
    };

    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 3) {
        sessionAny.lastSnapshot = {
          prompt: [
            "ELM: You'll take CYNDAQUIL, the fire POKeMON?",
            ">> YES",
            "NO",
          ],
          dialogue: ["ELM: You'll take CYNDAQUIL, the fire POKeMON?"],
          menu: null,
        };
      }
    });

    const action = await session.press("a", 1);

    expect(sessionAny.stepFrames).toHaveBeenCalledWith(3);
    expect(action.result.ok).toBe(true);
    expect(sessionAny.lastSnapshot?.prompt).toEqual(
      expect.arrayContaining(["ELM: You'll take CYNDAQUIL, the fire POKeMON?", ">> YES", "NO"])
    );
  });

  it("settles extra overworld frames so late NPC dialogue counts as progress", async () => {
    const session = getMcpSession("press-overworld-late-dialogue");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          _ignore_a_until_release: boolean;
          dialogue: { ignore_confirm_until_release: boolean };
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
      requestAutosave: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const overworld = {
      _ignore_a_until_release: false,
      dialogue: { ignore_confirm_until_release: false },
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 7, player_y: 13 } }),
      getMapName: () => "MrPokemonsHouse",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.scheduleKeyPress = jest.fn();
    let elapsedFrames = 0;
    sessionAny.stepFrames = jest.fn((count: number) => {
      elapsedFrames += count;
      if (elapsedFrames >= 6) {
        sessionAny.lastSnapshot = { prompt: null, dialogue: ["MR.POKEMON: Hello, hello!"], menu: null };
      }
    });

    const action = await session.press("a", 1);

    expect(sessionAny.stepFrames).toHaveBeenCalledWith(2);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(4);
    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
  });

  it("flags blocked moves when position does not change", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    overworld._last_block_feedback = { reason: "map_edge" };

    const action = await session.move("up", 1);

    expect(action.result.ok).toBe(false);
    expect(action.result.changed).toBe(false);
    expect(action.result.reason).toBe("blocked");
    expect(JSON.stringify(action.result.events ?? [])).toContain("blocked");
    expect(sessionAny.requestAutosave).toHaveBeenCalledWith({ force: true });
  });

  it("treats prompt/dialogue transitions during move as changed progress", async () => {
    const { session, sessionAny, overworld } = buildMoveHarness();
    sessionAny.lastSnapshot = { prompt: null, dialogue: null };
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      overworld.is_moving = false;
      sessionAny.lastSnapshot = { prompt: null, dialogue: ["ELM: Hello there!"] };
    });

    const action = await session.move("up", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBe("busy");
    expect(action.result.ok).toBe(false);
    expect(JSON.stringify(action.result.events ?? [])).toContain("interrupted:dialogue");
  });

  it("treats stable overworld coord changes as movement even when raw player_x/player_y stay stale", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    gameState.wram.wXCoord = 23;
    gameState.wram.wYCoord = 9;
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      overworld.is_moving = false;
      gameState.wram.wXCoord = 21;
      gameState.wram.wYCoord = 9;
      overworld._last_block_feedback = { reason: "terrain" };
    });

    const action = await session.move("left", 1);

    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(JSON.stringify(action.result.events ?? [])).toContain("moved:1");
    expect(JSON.stringify(action.result.events ?? [])).not.toContain("blocked");
  });

  it("detects a delayed post-move map transition during settle frames", () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    const game = sessionAny.getGame();
    const settlePostMoveEvents = (
      session as unknown as {
        settlePostMoveEvents: (game: unknown, baselineMap: { name: string; group: number | null; number: number | null }) => string | null;
        getStopReason: jest.Mock;
        settleMovementLock: jest.Mock;
        stepFrames: jest.Mock;
      }
    );
    settlePostMoveEvents.getStopReason = jest
      .fn()
      .mockReturnValueOnce(null)
      .mockReturnValueOnce("map_transition");
    settlePostMoveEvents.settleMovementLock = jest.fn();
    settlePostMoveEvents.stepFrames = jest.fn();

    const reason = settlePostMoveEvents.settlePostMoveEvents(game, {
      name: "CherrygroveMart",
      group: null,
      number: null,
    });

    expect(reason).toBe("map_transition");
    expect(settlePostMoveEvents.stepFrames).toHaveBeenCalledTimes(1);
  });

  it("records delayed map transitions after a move lands", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    installMoveStepper(sessionAny, gameState, overworld);
    sessionAny.settlePostMoveEvents = jest.fn(() => "map_transition");

    const action = await session.move("right", 1);

    expect(sessionAny.settlePostMoveEvents).toHaveBeenCalledTimes(1);
    expect(action.result.changed).toBe(true);
    expect(action.result.events).toEqual(
      expect.arrayContaining(["moved:1", "interrupted:map_transition"])
    );
  });

  it("retries once after a turn-in-place even if stale block feedback is present", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    const pendingMoves: string[] = [];
    sessionAny.scheduleKeyPress = jest.fn(({ direction }: { direction?: string }) => {
      if (direction) {
        pendingMoves.push(direction);
      }
    });
    sessionAny.stepFrames = jest.fn(() => {
      const direction = pendingMoves.shift();
      if (!direction) {
        return;
      }
      if (direction === "left" && overworld.player_direction !== "left") {
        overworld.player_direction = "left";
        overworld._last_block_feedback = { reason: "terrain" };
        overworld.is_moving = false;
        return;
      }
      if (direction === "left") {
        overworld.player_direction = "left";
        gameState.wram.player_x -= 1;
        overworld._last_block_feedback = null;
        overworld.is_moving = false;
      }
    });

    const action = await session.move("left", 1);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledTimes(2);
    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
    expect(JSON.stringify(action.result.events ?? [])).toContain("moved:1");
    expect(JSON.stringify(action.result.events ?? [])).not.toContain("blocked");
  });

  it("allows D-pad navigation while a prompt is visible", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    gameState.wram.player_x = 10;
    gameState.wram.player_y = 10;
    sessionAny.lastSnapshot = { prompt: ["Is it DST?", ">> YES", "NO"], dialogue: null };

    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      // Prompt navigation should not move the player, but should be able to change the prompt text.
      overworld.is_moving = false;
      sessionAny.lastSnapshot = { prompt: ["Is it DST?", "YES", ">> NO"], dialogue: null };
    });

    const action = await session.move("down", 1);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledTimes(1);
    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
    expect(JSON.stringify(action.result.events ?? [])).toContain("moved:1");
    expect(JSON.stringify(action.result.events ?? [])).not.toContain("interrupted:prompt");
  });

  it("allows D-pad navigation while a menu is visible", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    gameState.wram.player_x = 7;
    gameState.wram.player_y = 9;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["MENU", ">> #MON", "PACK", "#GEAR", "SAVE"],
    };

    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      overworld.is_moving = false;
      sessionAny.lastSnapshot = {
        prompt: null,
        dialogue: null,
        menu: ["MENU", "#MON", ">> PACK", "#GEAR", "SAVE"],
      };
    });

    const action = await session.move("down", 1);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledTimes(1);
    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
    expect(JSON.stringify(action.result.events ?? [])).toContain("moved:1");
    expect(JSON.stringify(action.result.events ?? [])).not.toContain("interrupted:menu");
  });

  it("retries once after an orientation-only step so move can still progress", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    const pendingMoves: string[] = [];
    let consumedInputs = 0;
    const facing = { current: "up" as string };
    (
      overworld as unknown as {
        player_direction?: string;
      }
    ).player_direction = facing.current;
    sessionAny.scheduleKeyPress = jest.fn(({ direction }: { direction?: string }) => {
      if (direction) {
        pendingMoves.push(direction);
        overworld.is_moving = true;
      }
    });
    sessionAny.stepFrames = jest.fn((count: number) => {
      for (let i = 0; i < count; i += 1) {
        if (!pendingMoves.length) {
          continue;
        }
        const direction = pendingMoves.shift();
        if (!direction) {
          continue;
        }
        consumedInputs += 1;
        if (consumedInputs === 1) {
          facing.current = direction;
          (overworld as unknown as { player_direction?: string }).player_direction = direction;
          overworld.is_moving = false;
          continue;
        }
        switch (direction) {
          case "up":
            gameState.wram.player_y -= 1;
            break;
          case "down":
            gameState.wram.player_y += 1;
            break;
          case "left":
            gameState.wram.player_x -= 1;
            break;
          case "right":
            gameState.wram.player_x += 1;
            break;
        }
        facing.current = direction;
        (overworld as unknown as { player_direction?: string }).player_direction = direction;
        overworld.is_moving = false;
      }
    });

    const action = await session.move("right", 1);

    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
    expect(JSON.stringify(action.result.events ?? [])).toContain("moved:1");
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledTimes(2);
  });

  it("settles transient movement locks after dialogue closes, even while script runner is briefly busy", async () => {
    const session = getMcpSession("move-players-house-lock");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      frameLimiter: { consume: jest.Mock };
      holdFrames: number;
      maxFramesPerCall: number;
      lastSnapshot: {
        prompt?: string[] | null;
        dialogue?: string[] | null;
        menu?: string[] | null;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => {
          wram: {
            player_x: number;
            player_y: number;
            wXCoord: number;
            wYCoord: number;
            wMapGroup: number;
            wMapNumber: number;
          };
        };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          is_moving: boolean;
          player_direction: string;
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          _last_block_feedback?: { reason?: string } | null;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.frameLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.maxFramesPerCall = 10;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: ["MOM: Right. All boys leave home someday."],
      menu: null,
    };
    const state = {
      wram: {
        player_x: 19,
        player_y: 9,
        wXCoord: 19,
        wYCoord: 9,
        wMapGroup: 2,
        wMapNumber: 1,
      },
    };
    let movementLockFrames = 5;
    let scriptBusyFrames = 5;
    const scriptRunner = {} as { is_busy?: boolean; _script_stack?: unknown[] };
    Object.defineProperty(scriptRunner, "is_busy", {
      enumerable: true,
      configurable: true,
      get: () => scriptBusyFrames > 0,
    });
    Object.defineProperty(scriptRunner, "_script_stack", {
      enumerable: true,
      configurable: true,
      get: () => (scriptBusyFrames > 0 ? [{}] : []),
    });
    const overworld = {
      is_moving: false,
      player_direction: "down",
      script_runner: scriptRunner,
      player_movement_locked: () => movementLockFrames > 0,
      script_tasks_active: () => false,
      _last_block_feedback: null as { reason?: string } | null,
    };
    const pendingDirections: string[] = [];
    const game = {
      getGameState: () => state,
      getMapName: () => "PlayersHouse1F",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn(({ direction }: { direction?: string }) => {
      if (direction) {
        pendingDirections.push(direction);
        overworld.is_moving = true;
        overworld.player_direction = direction;
      }
    });
    sessionAny.stepFrames = jest.fn((count: number) => {
      for (let i = 0; i < count; i += 1) {
        if (movementLockFrames > 0) {
          movementLockFrames -= 1;
          if (movementLockFrames === 4) {
            sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
          }
        }
        if (scriptBusyFrames > 0) {
          scriptBusyFrames -= 1;
        }
        const direction = pendingDirections.shift();
        if (!direction) {
          overworld.is_moving = false;
          continue;
        }
        if (direction === "left") {
          overworld._last_block_feedback = { reason: "map_edge" };
        }
        overworld.is_moving = false;
      }
    });

    await session.advanceFrames(2);
    const action = await session.move("left", 1);

    expect(action.result.ok).toBe(false);
    expect(action.result.reason).toBe("blocked");
    expect(action.result.events).toEqual(expect.arrayContaining(["blocked:terrain"]));
    expect(JSON.stringify(action.result.events ?? [])).not.toContain("interrupted:movement_lock");
    expect(JSON.stringify(action.result.events ?? [])).not.toContain("interrupted:script_runner");
  });
});

describe("McpGameSession holdButton input parity", () => {
  it("schedules directional holds as direction events, not button-named events", async () => {
    const session = getMcpSession("hold-direction-parity");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      finalizeActionResult: jest.Mock;
      recordActionEvent: jest.Mock;
      observeText: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({ mode: "overworld" }));
    sessionAny.buildStateFingerprint = jest.fn()
      .mockReturnValueOnce("before")
      .mockReturnValueOnce("after");
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.finalizeActionResult = jest.fn(async (result) => ({
      ok: true,
      changed: result.changed,
      reason: result.reason,
      events: result.events,
    }));
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");

    await session.holdButton("left", 3);

    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({
        direction: "left",
        button: undefined,
        holdFrames: 3,
      })
    );
  });
});

describe("McpGameSession executeMacro", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it("stops when a prompt appears after an action", async () => {
    const session = getMcpSession("macro-prompt");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      move: jest.Mock;
      press: jest.Mock;
      performMove: jest.Mock;
      observeText: jest.Mock;
      lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null } | null;
      game: {
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getMapName: () => string;
        getGameState: () => { wram: { player_x: number; player_y: number } };
        postEvent: jest.Mock;
        tick: jest.Mock;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.lastSnapshot = { prompt: null, dialogue: null };
    sessionAny.performMove = jest.fn(async () => {
      sessionAny.lastSnapshot = { prompt: ["WAIT"], dialogue: null };
      return {
        requested: 1,
        completed: 1,
        start: [1, 1],
        end: [1, 0],
        map: "TestMap",
        blocked: false,
        blockReason: null,
        stopReason: "prompt",
      };
    });
    sessionAny.press = jest.fn(async () => "PRESS_OK");
    const game = {
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getMapName: () => "TestMap",
      getGameState: () => ({ wram: { player_x: 1, player_y: 1 } }),
      postEvent: jest.fn(),
      tick: jest.fn(),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const action = await session.executeMacro(
      [
        { type: "move", value: "up" },
        { type: "button", value: "a" },
      ],
      { stop_on_event: true }
    );

    expect(sessionAny.performMove).toHaveBeenCalledTimes(1);
    expect(sessionAny.press).not.toHaveBeenCalled();
    expect(action.result.ok).toBe(false);
    expect(action.result.reason).toBe("busy");
    expect(JSON.stringify(action.result.events ?? [])).toContain("macro");
    expect(JSON.stringify(action.result.events ?? [])).toContain("interrupted:prompt");
  });

  it("applies delay frames between actions", async () => {
    const session = getMcpSession("macro-delay");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      move: jest.Mock;
      press: jest.Mock;
      performMove: jest.Mock;
      observeText: jest.Mock;
      advanceFrames: jest.Mock;
      lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null } | null;
      game: {
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getMapName: () => string;
        getGameState: () => { wram: { player_x: number; player_y: number } };
        postEvent: jest.Mock;
        tick: jest.Mock;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.advanceFrames = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { prompt: null, dialogue: null };
    sessionAny.performMove = jest.fn(async () => ({
      requested: 1,
      completed: 1,
      start: [1, 1],
      end: [1, 0],
      map: "TestMap",
      blocked: false,
      blockReason: null,
      stopReason: null,
    }));
    sessionAny.press = jest.fn(async () => "PRESS_OK");
    const game = {
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getMapName: () => "TestMap",
      getGameState: () => ({ wram: { player_x: 1, player_y: 1 } }),
      postEvent: jest.fn(),
      tick: jest.fn(),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    await session.executeMacro(
      [
        { type: "button", value: "a" },
        { type: "move", value: "up" },
      ],
      { delay_frames: 2 }
    );

    expect(sessionAny.advanceFrames).toHaveBeenCalledWith(2);
  });

  it("records normalized execute_macro actions and executed steps in order", async () => {
    const session = getMcpSession("macro-trace-order");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      captureSnapshot: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      readMapIdentity: jest.Mock;
      getStopReason: jest.Mock;
      performMove: jest.Mock;
      advanceFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      recordActionEvent: jest.Mock;
      lastMcpMeta: {
        macro_execution_trace?: {
          raw_input?: { actions?: Array<{ type?: string; value?: string }> };
          normalized_actions?: { actions?: Array<{ type?: string; value?: string; times?: number; hold_frames?: number; frames?: number; delay_frames?: number }> };
          executed_actions?: { steps?: Array<{ action?: { type?: string; value?: string; times?: number; hold_frames?: number; frames?: number; delay_frames?: number } }> };
        };
      } | null;
      game: {
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getMapName: () => string;
        getGameState: () => { wram: { player_x: number; player_y: number } };
        postEvent: jest.Mock;
        tick: jest.Mock;
        getOverworld: () => {
          _queued_direction?: string | null;
          _held_directions?: Map<string, null>;
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: null,
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "TestMap", group: 1, number: 1 }));
    sessionAny.getStopReason = jest.fn(() => null);
    sessionAny.performMove = jest.fn(async () => ({
      requested: 2,
      completed: 2,
      start: [10, 10],
      end: [10, 8],
      map: "TestMap",
      blocked: false,
      blockReason: null,
      stopReason: null,
    }));
    sessionAny.advanceFrames = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.recordActionEvent = jest.fn();
    const game = {
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getMapName: () => "TestMap",
      getGameState: () => ({ wram: { player_x: 10, player_y: 10 } }),
      postEvent: jest.fn(),
      tick: jest.fn(),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const action = await session.executeMacro([
      { type: "move", value: "up", times: 2, hold_frames: 3, delay_frames: 1 },
      { type: "button", value: "a", times: 2, hold_frames: 2, delay_frames: 0 },
      { type: "wait", frames: 4, delay_frames: 0 },
    ]);

    const trace = sessionAny.lastMcpMeta?.macro_execution_trace;
    expect(trace?.raw_input?.actions?.map((entry) => `${entry.type}:${entry.value ?? "wait"}`)).toEqual([
      "move:up",
      "button:a",
      "wait:wait",
    ]);
    expect(trace?.normalized_actions?.actions).toEqual([
      expect.objectContaining({ index: 0, type: "move", value: "up", times: 2, hold_frames: 3, delay_frames: 1 }),
      expect.objectContaining({ index: 1, type: "button", value: "a", times: 2, hold_frames: 2, delay_frames: 0 }),
      expect.objectContaining({ index: 2, type: "wait", frames: 4, delay_frames: 0 }),
    ]);
    expect(trace?.executed_actions?.steps?.map((step) => step.action)).toEqual([
      expect.objectContaining({ index: 0, type: "move", value: "up", times: 2, hold_frames: 3, delay_frames: 1 }),
      expect.objectContaining({ index: 1, type: "button", value: "a", times: 2, hold_frames: 2, delay_frames: 0 }),
      expect.objectContaining({ index: 2, type: "wait", frames: 4, delay_frames: 0 }),
    ]);
    expect(action.result.events).toEqual(expect.arrayContaining(["trace_steps:3"]));
  });

  it("clears stale queued directions before execute_macro steps", async () => {
    const session = getMcpSession("macro-trace-stale-direction");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      captureSnapshot: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      readMapIdentity: jest.Mock;
      getStopReason: jest.Mock;
      performMove: jest.Mock;
      recordActionEvent: jest.Mock;
      scheduledEvents: Array<{ frame: number; event: unknown }>;
      lastMcpMeta: {
        macro_execution_trace?: {
          stale_input_cleared?: boolean;
          executed_actions?: { steps?: Array<{ action?: { value?: string } }> };
        };
      } | null;
      game: {
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getMapName: () => string;
        getGameState: () => { wram: { player_x: number; player_y: number } };
        postEvent: jest.Mock;
        tick: jest.Mock;
        getOverworld: () => {
          _queued_direction?: string | null;
          _held_directions?: Map<string, null>;
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: null,
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "TestMap", group: 1, number: 1 }));
    sessionAny.getStopReason = jest.fn(() => null);
    const heldDirections = new Map<string, null>([["left", null]]);
    const overworld = {
      _queued_direction: "left" as string | null,
      _held_directions: heldDirections,
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    sessionAny.performMove = jest.fn(async () => {
      expect(overworld._queued_direction).toBeNull();
      expect(overworld._held_directions?.size).toBe(0);
      return {
        requested: 1,
        completed: 1,
        start: [5, 5],
        end: [5, 4],
        map: "TestMap",
        blocked: false,
        blockReason: null,
        stopReason: null,
      };
    });
    sessionAny.recordActionEvent = jest.fn();
    sessionAny.scheduledEvents = [{ frame: 0, event: { direction: "left" } }];
    const game = {
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getMapName: () => "TestMap",
      getGameState: () => ({ wram: { player_x: 5, player_y: 5 } }),
      postEvent: jest.fn(),
      tick: jest.fn(),
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const action = await session.executeMacro([{ type: "move", value: "up" }], {
      stop_on_event: false,
    });

    expect(action.result.events).toEqual(expect.arrayContaining(["stale_input_cleared"]));
    expect(sessionAny.lastMcpMeta?.macro_execution_trace?.stale_input_cleared).toBe(true);
    expect(
      sessionAny.lastMcpMeta?.macro_execution_trace?.executed_actions?.steps?.map(
        (step) => step.action?.value
      )
    ).toEqual(["up"]);
  });
});

describe("McpGameSession executeNamedMacro", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  const modalState = (overrides: Partial<{
    in_battle: boolean;
    in_menu: boolean;
    in_dialog: boolean;
    text_box_open: boolean;
    text_advance_pending: boolean;
    prompt_pending: boolean;
    movement_locked: boolean;
    script_busy: boolean;
    input_blocked_reason: string | null;
    can_move: boolean;
  }> = {}) => ({
    in_battle: false,
    in_menu: false,
    in_dialog: false,
    text_box_open: false,
    text_advance_pending: false,
    prompt_pending: false,
    movement_locked: false,
    script_busy: false,
    input_blocked_reason: null,
    can_move: true,
    ...overrides,
  });

  it("closes accidental menus with B and emits closed_menu reason", async () => {
    const session = getMcpSession("named-macro-close-menu");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: null,
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 1);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest.fn(() => modalState({ in_menu: true, input_blocked_reason: "menu" }));
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 1 });

    expect(action.result.changed).toBe(true);
    expect(action.result.events).toEqual(expect.arrayContaining(["reason:closed_menu"]));
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "b", holdFrames: 1 })
    );
  });

  it("clears stale A release guards before advancing dialogue macros", async () => {
    const session = getMcpSession("named-macro-clears-stale-a-release");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    const overworld = {
      _ignore_a_until_release: true,
      dialogue: { ignore_confirm_until_release: true },
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({ getOverworld: () => overworld }));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: null,
      menuText: "",
      dialogueText: "We can heal your POKEMON.",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 1);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest.fn(() =>
      modalState({ in_dialog: true, text_box_open: true, text_advance_pending: true, input_blocked_reason: "dialogue" })
    );
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    await session.executeNamedMacro("advance_dialog", { maxPresses: 1 });

    expect(overworld._ignore_a_until_release).toBe(false);
    expect(overworld.dialogue.ignore_confirm_until_release).toBe(false);
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "a", holdFrames: 1 })
    );
  });

  it.each([
    ["main_menu", "starts NEW GAME from the boot main menu"],
    ["continue", "continues from the boot continue screen"],
  ])("%s: %s with A instead of closing the boot UI", async (mode) => {
    const session = getMcpSession(`named-macro-boot-${mode}`);
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({
      getDebugStatus: () => ({ mode }),
    }));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode,
      menu: true,
      map: mode.toUpperCase(),
      promptReason: mode,
      menuText: mode === "main_menu" ? "CONTINUE\nNEW GAME\nOPTION" : "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 1);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 1 });

    expect(action.result.changed).toBe(true);
    expect(action.result.events).toEqual(expect.arrayContaining(["reason:advanced"]));
    expect(action.result.events).not.toEqual(expect.arrayContaining(["reason:closed_menu"]));
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "a" })
    );
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalledWith(
      expect.objectContaining({ button: "b" })
    );
  });

  it("busy-waits with backoff frames instead of spamming A when movement/script is busy", async () => {
    const session = getMcpSession("named-macro-busy-wait");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: null,
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValue("same");
    sessionAny.normalizeTimes = jest.fn(() => 2);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValue(modalState({ movement_locked: true, input_blocked_reason: "movement_lock" }));
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 2 });

    expect(action.result.changed).toBe(false);
    expect(action.result.reason).toBe("no_change");
    expect(action.result.events).toEqual(expect.arrayContaining(["reason:busy_wait"]));
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalled();
    expect(sessionAny.stepFrames.mock.calls.map((call: number[]) => call[0])).toEqual([1, 2]);
  });

  it("still presses A when busy flags are set during active dialog/prompt", async () => {
    const session = getMcpSession("named-macro-dialog-busy-press");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: "prompt",
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValue("same");
    sessionAny.normalizeTimes = jest.fn(() => 1);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValueOnce(
        modalState({
          in_dialog: true,
          prompt_pending: true,
          movement_locked: true,
          script_busy: true,
          input_blocked_reason: "prompt",
        })
      )
      .mockReturnValueOnce(
        modalState({
          in_dialog: true,
          prompt_pending: true,
          movement_locked: true,
          script_busy: true,
          input_blocked_reason: "prompt",
        })
      );
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 1 });

    expect(action.result.events).not.toEqual(expect.arrayContaining(["reason:busy_wait"]));
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "a" })
    );
    expect(sessionAny.stepFrames.mock.calls.map((call: number[]) => call[0])).toEqual([4]);
  });

  it("continues multi-page dialogue without treating it like a prompt choice", async () => {
    const session = getMcpSession("named-macro-multipage-dialogue");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "NewBarkTown",
      promptReason: null,
      textAdvancePending: true,
      menuText: "",
      dialogueText: "NEW BARK TOWN",
      promptText: "",
      markerText: "18,7,▼",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 2);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValueOnce(
        modalState({
          in_dialog: true,
          text_box_open: true,
          text_advance_pending: true,
          prompt_pending: false,
          input_blocked_reason: "dialogue",
        })
      )
      .mockReturnValueOnce(
        modalState({
          in_dialog: false,
          text_box_open: false,
          text_advance_pending: false,
          prompt_pending: false,
          input_blocked_reason: null,
        })
      );
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 2 });

    expect(action.result.changed).toBe(true);
    expect(action.result.events).toEqual(expect.arrayContaining(["reason:advanced"]));
    expect(action.result.events).not.toEqual(expect.arrayContaining(["reason:nudged_choice"]));
    expect(
      sessionAny.scheduleKeyPress.mock.calls.some(
        (call: Array<{ direction?: string }>) => typeof call[0]?.direction === "string"
      )
    ).toBe(false);
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "a" })
    );
  });

  it("stops after advancing dialogue into a prompt instead of confirming the choice", async () => {
    const session = getMcpSession("named-macro-dialogue-opens-prompt");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "GoldenrodGameCorner",
      promptReason: null,
      textAdvancePending: true,
      menuText: "",
      dialogueText: "Do you want some?",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 4);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValueOnce(
        modalState({
          in_dialog: true,
          text_box_open: true,
          text_advance_pending: true,
          prompt_pending: false,
          input_blocked_reason: "dialogue",
        })
      )
      .mockReturnValueOnce(
        modalState({
          in_dialog: true,
          text_box_open: true,
          prompt_pending: true,
          input_blocked_reason: "prompt",
        })
      );
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "PROMPT\n▶ YES\n  NO");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 4 });

    expect(action.result.events).toEqual(
      expect.arrayContaining(["stopped_on_prompt", "reason:prompt_opened"])
    );
    expect(action.result.events).toEqual(expect.arrayContaining(["pressed:1/4"]));
    expect(
      sessionAny.scheduleKeyPress.mock.calls.filter(
        (call: Array<{ button?: string }>) => call[0]?.button === "a"
      )
    ).toHaveLength(1);
  });

  it("nudges occasional prompts, then presses A with varied hold timing and reports advancement", async () => {
    const session = getMcpSession("named-macro-choice-nudge");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: "prompt",
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 4);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: true, in_dialog: true, input_blocked_reason: "prompt" }))
      .mockReturnValueOnce(modalState({ prompt_pending: false, in_dialog: false, input_blocked_reason: null }));
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 4 });

    expect(action.result.changed).toBe(true);
    expect(action.result.events).toEqual(
      expect.arrayContaining(["reason:advanced", "reason:nudged_choice"])
    );
    const pressCalls = sessionAny.scheduleKeyPress.mock.calls
      .map((call: Array<{ button?: string; holdFrames?: number }>) => call[0])
      .filter((entry: { button?: string }) => entry.button === "a");
    const holdFrames = pressCalls.map((entry: { holdFrames?: number }) => entry.holdFrames);
    expect(new Set(holdFrames).size).toBeGreaterThan(1);
    expect(
      sessionAny.scheduleKeyPress.mock.calls.some(
        (call: Array<{ direction?: string }>) => typeof call[0]?.direction === "string"
      )
    ).toBe(true);
  });

  it("treats prompt menus as dialog choices instead of closing them with B", async () => {
    const session = getMcpSession("named-macro-prompt-menu");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "menu",
      menu: true,
      map: "PlayersHouse1F",
      promptReason: "dialogue",
      menuText: "SUNDAY\nMONDAY",
      dialogueText: "What day is it?",
      promptText: "Choose a day",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 1);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValueOnce(
        modalState({
          in_menu: true,
          in_dialog: true,
          prompt_pending: true,
          movement_locked: true,
          script_busy: true,
          input_blocked_reason: "dialogue",
        })
      )
      .mockReturnValueOnce(
        modalState({
          in_menu: false,
          in_dialog: false,
          prompt_pending: false,
          movement_locked: false,
          script_busy: false,
          input_blocked_reason: null,
        })
      );
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 1 });

    expect(action.result.changed).toBe(true);
    expect(action.result.events).not.toEqual(expect.arrayContaining(["reason:closed_menu"]));
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "a" })
    );
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalledWith(
      expect.objectContaining({ button: "b" })
    );
  });

  it("stops after TM/HM dialogue opens the party selection menu instead of canceling it", async () => {
    const session = getMcpSession("named-macro-tmhm-party-menu");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "menu",
      menu: true,
      map: "Bag",
      promptReason: "menu_dialogue",
      menuText: "TM01",
      dialogueText: "Booted up a TM.",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValueOnce("before").mockReturnValueOnce("after");
    sessionAny.normalizeTimes = jest.fn(() => 4);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest
      .fn()
      .mockReturnValueOnce(
        modalState({
          in_menu: true,
          in_dialog: true,
          text_box_open: true,
          text_advance_pending: true,
          input_blocked_reason: "menu",
        })
      )
      .mockReturnValueOnce(
        modalState({
          in_menu: true,
          in_dialog: false,
          prompt_pending: false,
          text_box_open: false,
          text_advance_pending: false,
          input_blocked_reason: "menu",
        })
      )
      .mockReturnValueOnce(
        modalState({
          in_menu: true,
          in_dialog: false,
          prompt_pending: false,
          text_box_open: false,
          text_advance_pending: false,
          input_blocked_reason: "menu",
        })
      )
      .mockReturnValue(
        modalState({
          in_menu: true,
          in_dialog: false,
          prompt_pending: false,
          text_box_open: false,
          text_advance_pending: false,
          input_blocked_reason: "menu",
        })
      );
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "POKEMON MENU\nTeach which POKEMON?");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 4 });

    expect(action.result.changed).toBe(true);
    expect(action.result.events).toEqual(expect.arrayContaining(["reason:advanced"]));
    expect(action.result.events).not.toEqual(expect.arrayContaining(["reason:closed_menu"]));
    expect(sessionAny.scheduleKeyPress).toHaveBeenCalledWith(
      expect.objectContaining({ button: "a" })
    );
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalledWith(
      expect.objectContaining({ button: "b" })
    );
    expect(sessionAny.getModalUiState).toHaveBeenCalledTimes(2);
  });

  it("returns no_effect when no prompt/dialog/menu state is active", async () => {
    const session = getMcpSession("named-macro-no-effect");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      captureSceneSignal: jest.Mock;
      buildStateFingerprint: jest.Mock;
      normalizeTimes: jest.Mock;
      normalizeDelayFrames: jest.Mock;
      normalizeHoldFrames: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      getModalUiState: jest.Mock;
      recordAction: jest.Mock;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      captureSnapshot: jest.Mock;
      observeText: jest.Mock;
      recordActionEvent: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.getGame = jest.fn(() => ({}));
    sessionAny.captureSceneSignal = jest.fn(() => ({
      mode: "overworld",
      menu: false,
      map: "TestMap",
      promptReason: null,
      menuText: "",
      dialogueText: "",
      promptText: "",
      markerText: "",
    }));
    sessionAny.buildStateFingerprint = jest.fn().mockReturnValue("same");
    sessionAny.normalizeTimes = jest.fn(() => 3);
    sessionAny.normalizeDelayFrames = jest.fn(() => 0);
    sessionAny.normalizeHoldFrames = jest.fn(() => 1);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.getModalUiState = jest.fn(() => modalState());
    sessionAny.recordAction = jest.fn();
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.observeText = jest.fn(() => "OK");
    sessionAny.recordActionEvent = jest.fn();

    const action = await session.executeNamedMacro("advance_dialog", { maxPresses: 3 });

    expect(action.result.changed).toBe(false);
    expect(action.result.reason).toBe("no_change");
    expect(action.result.events).toEqual(expect.arrayContaining(["reason:no_effect"]));
    expect(sessionAny.scheduleKeyPress).not.toHaveBeenCalled();
  });

});

describe("McpGameSession status + recent events", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it("prefers non-Unknown map and non-origin coords when alternatives exist", async () => {
    const session = getMcpSession("status-fallback-test");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      game: {
        getGameState: () => {
          wram: { player_x: number; player_y: number; wXCoord: number; wYCoord: number };
          sram: { party: { pokemon: unknown[] } };
        };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => { current_map_name?: string; player_x?: number; player_y?: number };
      };
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    const game = {
      getGameState: () => ({
        wram: { player_x: 0, player_y: 0, wXCoord: 8, wYCoord: 9 },
        sram: { party: { pokemon: [] } },
      }),
      getMapName: () => "Unknown",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        current_map_name: "NewBarkTown",
        player_x: 8,
        player_y: 9,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();
    expect(snapshot.map).toBe("NewBarkTown");
    expect(snapshot.coords).toEqual({ x: 8, y: 9 });
  });

  it("prefers stable overworld coords (wXCoord/wYCoord) over sprite-local player coords in status and player context", async () => {
    const session = getMcpSession("status-player-context-coords");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      game: {
        getGameState: () => {
          wram: { player_x: number; player_y: number; wXCoord: number; wYCoord: number };
          sram: { party: { pokemon: unknown[] } };
        };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name?: string;
          player_x?: number;
          player_y?: number;
          player_direction?: string;
        };
      };
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    const game = {
      getGameState: () => ({
        wram: { player_x: 40, player_y: 41, wXCoord: 12, wYCoord: 13 },
        sram: { party: { pokemon: [] } },
      }),
      getMapName: () => "NewBarkTown",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        current_map_name: "NewBarkTown",
        player_x: 40,
        player_y: 41,
        player_direction: "down",
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();
    const playerContext = await session.playerContext();

    expect(snapshot.coords).toEqual({ x: 12, y: 13 });
    expect(playerContext.coords).toEqual({ x: 12, y: 13 });
  });

  it("surfaces wallet and Mom-bank money in raw MCP status", async () => {
    const session = getMcpSession("status-money-resources");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      game: unknown;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    const game = {
      getGameState: () => ({
        wram: {
          player_x: 4,
          player_y: 5,
          wXCoord: 4,
          wYCoord: 5,
          event_flags: {},
        },
        sram: {
          money: 1234,
          moms_money: 567,
          mom_saving_some_money: true,
          party: { pokemon: [] },
        },
      }),
      getMapName: () => "GoldenrodCity",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        current_map_name: "GoldenrodCity",
        player_x: 4,
        player_y: 5,
        player_direction: "down",
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot).toMatchObject({
      money: 1234,
      moms_money: 567,
      mom_saving_some_money: true,
      resources: {
        money: 1234,
        moms_money: 567,
        mom_saving_some_money: true,
      },
    });
  });

  it("settles transient post-cutscene movement locks before reporting status", async () => {
    const session = getMcpSession("status-settles-tail-lock");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      stepFrames: jest.Mock;
      maxFramesPerCall: number;
      lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null; menu?: string[] | null } | null;
      game: {
        getGameState: () => {
          wram: {
            player_x: number;
            player_y: number;
            wXCoord: number;
            wYCoord: number;
            wMapGroup: number;
            wMapNumber: number;
          };
          sram: { party: { pokemon: unknown[] } };
        };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          current_map_name?: string;
          player_x?: number;
          player_y?: number;
          player_direction?: string;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          script_runner: {
            is_busy?: boolean;
            _script_stack?: unknown[];
            _awaiting_resume?: number;
          } | null;
        };
      };
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.maxFramesPerCall = 10;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };

    let movementLocked = true;
    let scriptBusy = true;
    const scriptRunner = {
      is_busy: true,
      _script_stack: [{}],
      _awaiting_resume: 1,
    };
    const game = {
      getGameState: () => ({
        wram: {
          player_x: 5,
          player_y: 11,
          wXCoord: 5,
          wYCoord: 11,
          wMapGroup: 24,
          wMapNumber: 4,
        },
        sram: { party: { pokemon: [] } },
      }),
      getMapName: () => "NewBarkTown",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        current_map_name: "NewBarkTown",
        player_x: 5,
        player_y: 11,
        player_direction: "up",
        player_movement_locked: () => movementLocked,
        script_tasks_active: () => false,
        script_runner: scriptRunner,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.stepFrames = jest.fn(() => {
      movementLocked = false;
      scriptBusy = false;
      scriptRunner.is_busy = scriptBusy;
      scriptRunner._script_stack = [];
      scriptRunner._awaiting_resume = 0;
    });

    const snapshot = await session.status();

    expect(sessionAny.stepFrames).toHaveBeenCalled();
    expect(snapshot.movement_locked).toBe(false);
    expect(snapshot.script_busy).toBe(false);
    expect(snapshot.can_move).toBe(true);
    expect(snapshot.input_blocked_reason).toBeNull();
    expect(snapshot.coords).toEqual({ x: 5, y: 11 });
  });

  it("returns recent action events with recap", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    installMoveStepper(sessionAny, gameState, overworld);

    await session.move("right", 1);
    const journal = await session.recentEvents(5);

    expect(journal.total).toBeGreaterThan(0);
    expect(journal.events.length).toBeGreaterThan(0);
    expect(journal.events[journal.events.length - 1]?.action).toContain("move:right");
    expect(typeof journal.events[journal.events.length - 1]?.timestamp_ms).toBe("number");
    expect(typeof journal.events[journal.events.length - 1]?.timestamp_iso).toBe("string");
    expect(typeof journal.recap).toBe("string");
    expect(journal.recap.length).toBeGreaterThan(0);
  });

  it("returns cloned recent events to prevent consumer mutation side effects", async () => {
    const { session, sessionAny, gameState, overworld } = buildMoveHarness();
    installMoveStepper(sessionAny, gameState, overworld);

    await session.move("right", 1);
    const first = await session.recentEvents(1);
    expect(first.events.length).toBe(1);
    first.events[0].action = "tampered";
    first.events[0].result.changed = false;
    if (first.events[0].result.events) {
      first.events[0].result.events[0] = "tampered:event";
    }

    const second = await session.recentEvents(1);
    expect(second.events[0].action).not.toBe("tampered");
    expect(second.events[0].result.changed).toBe(true);
    expect(second.events[0].result.events?.[0]).not.toBe("tampered:event");
  });

  it("captures notable menu/dialogue transitions in recent events", async () => {
    const session = getMcpSession("recent-events-transitions");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null };
    let menuOpen = false;
    const game = {
      getGameState: () => ({ wram: { player_x: 3, player_y: 4 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => menuOpen,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      menuOpen = true;
      sessionAny.lastSnapshot = { prompt: null, dialogue: ["Elm: Hello!"] };
    });

    await session.press("start", 1);
    const journal = await session.recentEvents(1);
    const latest = journal.events[0];

    expect(latest?.moments).toEqual(expect.arrayContaining(["menu_opened"]));
    expect(latest?.moments).not.toEqual(expect.arrayContaining(["prompt_opened:dialogue"]));
    expect(journal.recap).toContain("mode:overworld->menu");
  });

  it("tracks dialogue/menu milestone moments in recent events", async () => {
    const session = getMcpSession("recent-events-milestones");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        menu?: string[] | null;
        prompt?: string[] | null;
        dialogue?: string[] | null;
        marker?: number[] | null;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null, marker: null };
    let menuOpen = false;
    let stepCount = 0;
    const game = {
      getGameState: () => ({ wram: { player_x: 7, player_y: 8 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => menuOpen,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      stepCount += 1;
      if (stepCount === 1) {
        menuOpen = false;
        sessionAny.lastSnapshot = { prompt: null, dialogue: ["ELM: Hello there!"], menu: null, marker: null };
        return;
      }
      if (stepCount === 2) {
        menuOpen = false;
        sessionAny.lastSnapshot = { prompt: null, dialogue: ["ELM: Pick a POKeMON."], menu: null, marker: null };
        return;
      }
      menuOpen = true;
      sessionAny.lastSnapshot = {
        prompt: null,
        dialogue: null,
        menu: ["▶ POKEDEX", "  POKEMON", "  PACK"],
        marker: null,
      };
    });

    await session.press("a", 1);
    await session.press("a", 1);
    await session.press("start", 1);
    const journal = await session.recentEvents(3);
    const moments = journal.events.flatMap((event) => event.moments ?? []);

    expect(moments).toEqual(expect.arrayContaining(["talked_to_npc", "dialogue_advanced", "menu_changed"]));
  });
});

describe("McpGameSession reason consistency", () => {
  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it("marks no-change button presses in menu context as menu failures", async () => {
    const session = getMcpSession("press-menu-reason");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null };
    const game = {
      getGameState: () => ({ wram: { player_x: 5, player_y: 6 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => true,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn();

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(false);
    expect(action.result.reason).toBe("menu");
    expect(action.result.ok).toBe(false);
  });

  it("waits for slot machine overlay input to settle instead of reporting generic menu blockage", async () => {
    const session = getMcpSession("press-slot-machine-input-owner");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport: string[];
        info: string[];
        menu: null;
        prompt: null;
        dialogue: null;
        titles: { viewport: string; info: string };
      };
      captureSnapshot: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; instant_mode: boolean } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          input_capture_active: boolean;
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
        tick: () => void;
        postEvent: jest.Mock;
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("SLOT MACHINE");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.lastSnapshot = {
      viewport: ["SLOT MACHINE", "COINS 632", "BET 3", "BET 3"],
      info: ["STATE: slot_machine", "Left/Right=Bet A=Spin B=Quit"],
      menu: null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "Slot Machine", info: "Slot Machine" },
    };
    const gameState = { wram: { player_x: 5, player_y: 6, instant_mode: true } };
    const game = {
      getGameState: () => gameState,
      getMapName: () => "GoldenrodGameCorner",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        input_capture_active: true,
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
      tick: jest.fn(),
      postEvent: jest.fn((event: { type?: string; direction?: string | null }) => {
        if (event.type === "keydown" && event.direction === "right") {
          setTimeout(() => {
            gameState.wram.player_x += 1;
          }, 5);
        }
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const action = await session.press("right", 1);

    expect(game.postEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "keydown",
        direction: "right",
      })
    );
    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
  });

  it("waits for slot machine B input to close the overlay instead of leaving TUI stuck in menu", async () => {
    const session = getMcpSession("press-slot-machine-close");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport: string[];
        info: string[];
        menu: null;
        prompt: null;
        dialogue: null;
        titles: { viewport: string; info: string };
      };
      captureSnapshot: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number; instant_mode: boolean } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          input_capture_active: boolean;
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
        tick: () => void;
        postEvent: jest.Mock;
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OVERWORLD");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.captureSnapshot = jest.fn();
    sessionAny.lastSnapshot = {
      viewport: ["SLOT MACHINE", "COINS 634", "BET 3", "WIN 8"],
      info: ["STATE: slot_machine", "Left/Right=Bet A=Spin B=Quit"],
      menu: null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "Slot Machine", info: "Slot Machine" },
    };
    const gameState = { wram: { player_x: 5, player_y: 6, instant_mode: true } };
    let inputCaptureActive = true;
    const game = {
      getGameState: () => gameState,
      getMapName: () => "GoldenrodGameCorner",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        input_capture_active: inputCaptureActive,
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
      tick: jest.fn(),
      postEvent: jest.fn((event: { type?: string; button?: string | null }) => {
        if (event.type === "keydown" && event.button === "b") {
          setTimeout(() => {
            inputCaptureActive = false;
            sessionAny.lastSnapshot = {
              viewport: ["OVERWORLD", "00 01", "00 @< ."],
              info: ["D-Pad=Move A=Talk Start=Menu Select=Item B=Back"],
              menu: null,
              prompt: null,
              dialogue: null,
              titles: { viewport: "Overworld", info: "Overworld" },
            };
          }, 5);
        }
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);

    const action = await session.press("b", 1);

    expect(game.postEvent).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "keydown",
        button: "b",
      })
    );
    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.lastSnapshot.viewport[0]).toBe("OVERWORLD");
  });

  it("treats dialogue text progression as a changed press result", async () => {
    const session = getMcpSession("press-dialogue-progression");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
      requestAutosave: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = { prompt: null, dialogue: ["ELM: Hello there!"], menu: null };
    const game = {
      getGameState: () => ({ wram: { player_x: 5, player_y: 6 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      sessionAny.lastSnapshot = { prompt: null, dialogue: ["ELM: Welcome to Pokemon!"], menu: null };
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.requestAutosave).toHaveBeenCalledWith({ force: true });
  });

  it("clears stale overworld A-release suppression before discrete MCP taps", async () => {
    const session = getMcpSession("press-clears-stale-a-release-guard");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          _ignore_a_until_release: boolean;
          dialogue: { ignore_confirm_until_release: boolean };
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
      requestAutosave: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
    const overworld = {
      _ignore_a_until_release: true,
      dialogue: { ignore_confirm_until_release: true },
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 5, player_y: 6 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => false,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.requestAutosave = jest.fn().mockResolvedValue(undefined);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      sessionAny.lastSnapshot = { prompt: null, dialogue: ["CLERK: How many?"], menu: null };
    });

    const action = await session.press("a", 1);

    expect(overworld._ignore_a_until_release).toBe(false);
    expect(overworld.dialogue.ignore_confirm_until_release).toBe(false);
    expect(action.result.ok).toBe(true);
    expect(action.result.changed).toBe(true);
  });

  it("treats menu surface updates as a changed press result", async () => {
    const session = getMcpSession("press-menu-cursor");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ POKEDEX", "  POKEMON", "  PACK"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 5, player_y: 6 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => true,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      sessionAny.lastSnapshot = {
        prompt: null,
        dialogue: null,
        menu: ["  POKEDEX", "▶ POKEMON", "  PACK"],
      };
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
  });

  it("treats stats-style viewport-only updates as a changed press result", async () => {
    const session = getMcpSession("press-stats-viewport-page");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: {
        viewport?: string[] | null;
        menu?: string[] | null;
        prompt?: string[] | null;
        dialogue?: string[] | null;
        info?: string[] | null;
        titles?: Record<string, string>;
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      viewport: ["POKEMON STATS", "PAGE: 1/3", "PAGE 1: STATUS"],
      info: ["L/R/A=Page", "Up/Down=Pokemon B=Back"],
      menu: null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "Pokemon Stats", info: "Legend" },
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 5, player_y: 6 } }),
      getMapName: () => "TestMap",
      isMenuOpen: () => true,
      isBattleActive: () => false,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn(() => {
      sessionAny.lastSnapshot = {
        viewport: ["POKEMON STATS", "PAGE: 2/3", "PAGE 2: MOVES"],
        info: ["L/R/A=Page", "Up/Down=Pokemon B=Back"],
        menu: null,
        prompt: null,
        dialogue: null,
        titles: { viewport: "Pokemon Stats", info: "Legend" },
      };
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
  });

  it("gives battle menu A presses enough settle frames to carry move confirms forward", async () => {
    const session = getMcpSession("press-battle-menu-confirm");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: null,
      menu: ["▶ FIGHT", "  <PKMN>", "  PACK", "  RUN"],
    };
    const game = {
      getGameState: () => ({ wram: { player_x: 83, player_y: 35 } }),
      getMapName: () => "Route29",
      isMenuOpen: () => true,
      isBattleActive: () => true,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 5) {
        sessionAny.lastSnapshot = {
          prompt: null,
          dialogue: null,
          menu: ["  FIGHT", "▶ <PKMN>", "  PACK", "  RUN"],
        };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(5);
  });

  it("settles extra battle frames after dialogue closes into a blank post-KO handoff", async () => {
    const session = getMcpSession("press-battle-dialogue-handoff");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: jest.Mock;
      actionLimiter: { consume: jest.Mock };
      holdFrames: number;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => { wram: { player_x: number; player_y: number } };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
        };
      };
      getGame: jest.Mock;
    };
    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.observeText = jest.fn().mockReturnValue("OK");
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.lastSnapshot = {
      prompt: null,
      dialogue: ["Enemy SPINARAK fainted!"],
      menu: null,
    };
    let inBattle = true;
    let reachedBlankBattle = false;
    const game = {
      getGameState: () => ({ wram: { player_x: 35, player_y: 27 } }),
      getMapName: () => "Route30",
      isMenuOpen: () => false,
      isBattleActive: () => inBattle,
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.scheduleKeyPress = jest.fn();
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 3 && !reachedBlankBattle) {
        reachedBlankBattle = true;
        sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
        return;
      }
      if (reachedBlankBattle && count >= 4) {
        inBattle = false;
        sessionAny.lastSnapshot = { prompt: null, dialogue: null, menu: null };
      }
    });

    const action = await session.press("a", 1);

    expect(action.result.changed).toBe(true);
    expect(action.result.reason).toBeUndefined();
    expect(action.result.ok).toBe(true);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(3);
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(4);
  });

  it("ignores impossible negative map coordinates when reporting status", async () => {
    const session = getMcpSession("status-prefers-nonnegative-coords");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
      lastSnapshot: unknown;
      actionEvents: unknown[];
      getModalUiState: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = null;
    sessionAny.actionEvents = [
      {
        frame: 10,
        timestamp_ms: 1710000000000,
        timestamp_iso: "2024-03-09T16:00:00.000Z",
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 5 },
        result: { ok: false, changed: false, reason: "no_change" },
      },
    ];
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));
    sessionAny.getModalUiState = jest.fn(() => ({
        in_battle: false,
        in_menu: false,
        in_dialog: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        input_blocked_reason: null,
        can_move: true,
      })
    );
    sessionAny.getGame = jest.fn(() => ({
      isBattleActive: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 9,
          player_y: -5,
          wXCoord: 9,
          wYCoord: -5,
          current_map_group: 24,
          current_map_id: 5,
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        player_x: 9,
        player_y: 9,
        prev_player_x: 9,
        prev_player_y: 9,
      }),
    }));

    const snapshot = await session.status();

    expect(snapshot.coords).toEqual({ x: 9, y: 9 });
  });

  it("records boot-screen scene signals from the game state instead of stale overworld map data", () => {
    const session = getMcpSession("boot-scene-signal");
    const sessionAny = session as unknown as {
      captureSceneSignal: (game: unknown) => {
        mode: string;
        menu: boolean;
        map?: string;
        promptReason: string | null;
      };
      getDialogueUiState: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      readBestMapName: jest.Mock;
      lastSnapshot: unknown;
    };
    const game = {
      getDebugStatus: () => ({ mode: "title" }),
      isBattleActive: () => false,
    };

    sessionAny.lastSnapshot = null;
    sessionAny.getDialogueUiState = jest.fn(() => ({
      yes_no_prompt_open: false,
      text_advance_pending: false,
    }));
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.readBestMapName = jest.fn(() => "PlayersHouse2F");

    const signal = sessionAny.captureSceneSignal(game);

    expect(signal).toMatchObject({
      mode: "title",
      menu: true,
      map: "TITLE",
      promptReason: "title",
    });
  });

  it("serializes runtime resume state for player, NPC, runner, and dialogue progress", () => {
    const snapshot = __testing.serializeRuntimeSnapshot(42, {
      getMapName: () => "ElmsLab",
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: 7,
        prev_player_x: 9,
        prev_player_y: 9,
        player_direction: "left",
        npcs: [{ objectIndex: 1, x: 6, y: 8, prevX: 6, prevY: 8, direction: "down" }],
        script_runner: {
          _script_stack: [{ name: "ElmsLabWalkUpToElmScript", index: 3, allowFallthrough: false }],
          _awaiting_resume: 1,
          _queued_overworld_task_count: 0,
          stop_execution: true,
          last_yes_no_result: false,
          last_condition_result: false,
          pending_reload_map: null,
          last_interaction_object_index: 1,
          variables: {
            _loaded_menu: {
              label: "GoldenrodGameCornerTMVendorMenuHeader",
              options: ["TM25    5500", "TM14    5500", "TM38    5500", "CANCEL"],
            },
          },
        },
        dialogue: {
          visible: true,
          waiting_for_input: true,
          is_script_paused: true,
          pending_waits: 1,
          pending_text: ["Second line"],
          current_text: "PLAYER! There you are!",
          pending_script_waits_count: 1,
          _yes_no_prompt: { selection: 0 },
          auto_close_requested: false,
          ignore_confirm_until_release: true,
        },
      }),
    } as never, [
      {
        frame: 41,
        timestamp_ms: 1710000000000,
        timestamp_iso: "2024-03-09T16:00:00.000Z",
        action: "press:a:1",
        mode: "overworld",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        summary: "press:a:1 no_change",
        result: { ok: false, changed: false, reason: "no_change" },
      },
    ]);

    expect(snapshot).not.toBeNull();
    expect(snapshot?.player).toEqual({ x: 9, y: 7, prevX: 9, prevY: 9, direction: "left" });
    expect(snapshot?.npcs).toEqual([{ objectIndex: 1, x: 6, y: 8, prevX: 6, prevY: 8, direction: "down" }]);
    expect(snapshot?.runner?.stack).toEqual([
      { name: "ElmsLabWalkUpToElmScript", index: 3, allowFallthrough: false, parent: undefined },
    ]);
    expect(snapshot?.runner?.variables?._loaded_menu).toEqual({
      label: "GoldenrodGameCornerTMVendorMenuHeader",
      options: ["TM25    5500", "TM14    5500", "TM38    5500", "CANCEL"],
    });
    expect(snapshot?.dialogue?.currentText).toBe("PLAYER! There you are!");
    expect(snapshot?.dialogue?.yesNoSelection).toBe(0);
    expect(snapshot?.actionEvents).toEqual([
      expect.objectContaining({
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        result: { ok: false, changed: false, reason: "no_change", events: undefined },
      }),
    ]);
  });

  it("serializes stable non-negative player coords when overworld resume state has drifted negative", () => {
    const snapshot = __testing.serializeRuntimeSnapshot(42, {
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          wXCoord: 9,
          wYCoord: 7,
        },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: -13,
        prev_player_x: 9,
        prev_player_y: -13,
        player_direction: "left",
        npcs: [],
        script_runner: null,
        dialogue: null,
      }),
    } as never);

    expect(snapshot?.player).toEqual({ x: 9, y: 7, prevX: 9, prevY: 7, direction: "left" });
  });

  it("prefers settled overworld coords over stale WRAM coords when serializing runtime snapshots", () => {
    const snapshot = __testing.serializeRuntimeSnapshot(42, {
      getMapName: () => "CherrygroveMart",
      getGameState: () => ({
        wram: {
          wXCoord: 3,
          wYCoord: 15,
        },
      }),
      getOverworld: () => ({
        current_map_name: "CherrygroveMart",
        player_x: 5,
        player_y: 15,
        prev_player_x: 3,
        prev_player_y: 15,
        player_direction: "right",
        npcs: [],
        script_runner: null,
        dialogue: null,
      }),
    } as never);

    expect(snapshot?.player).toEqual({ x: 5, y: 15, prevX: 3, prevY: 15, direction: "right" });
  });

  it("surfaces live hidden dialogue ownership in status and player context even when rendered dialogue text is blank", async () => {
    const session = getMcpSession("status-hidden-live-dialogue");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 15,
          player_y: 5,
          wXCoord: 15,
          wYCoord: 5,
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 15,
        player_y: 5,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: {
          visible: false,
          active: false,
          waiting_for_input: true,
          pending_waits: 1,
          _yes_no_prompt: null,
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();
    const context = await session.playerContext();

    expect(snapshot.in_dialog).toBe(true);
    expect(snapshot.text_box_open).toBe(true);
    expect(snapshot.text_advance_pending).toBe(true);
    expect(snapshot.prompt_pending).toBe(false);
    expect(snapshot.dialogue).toEqual({ waiting_for_input: true });
    expect(snapshot.input_blocked_reason).toBe("dialogue");
    expect(snapshot.can_move).toBe(false);
    expect(context.dialogue_open).toBe(true);
    expect(context.text_advance_pending).toBe(true);
  });

  it("treats hidden engine-owned yes/no prompts as prompt-pending even before prompt text renders", async () => {
    const session = getMcpSession("status-hidden-yes-no-prompt");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "PlayersHouse1F");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "PlayersHouse1F", group: 24, number: 6 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "PlayersHouse1F",
      getGameState: () => ({
        wram: {
          player_x: 17,
          player_y: 9,
          wXCoord: 17,
          wYCoord: 9,
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "PlayersHouse1F",
        player_x: 17,
        player_y: 9,
        player_direction: "left",
        player_movement_locked: () => true,
        script_tasks_active: () => false,
        script_runner: {
          is_busy: true,
          _script_stack: [{}],
          _awaiting_resume: 1,
        },
        dialogue: {
          visible: false,
          active: false,
          waiting_for_input: false,
          pending_waits: 0,
          _yes_no_prompt: { selection: 0 },
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.prompt_pending).toBe(true);
    expect(snapshot.text_advance_pending).toBe(false);
    expect(snapshot.prompt).toEqual({ pending: true, reason: "prompt" });
    expect(snapshot.in_dialog).toBe(true);
    expect(snapshot.input_blocked_reason).toBe("prompt");
    expect(snapshot.can_move).toBe(false);
  });

  it("does not fabricate an objective interaction target from hotspot metadata alone", async () => {
    const session = getMcpSession("status-live-interaction-target");
    const sessionAny = session as unknown as {
      readInteractionTarget: (
        game: unknown,
        interactionTile: { x: number; y: number },
        mapDetails: {
          hotspots: Array<{
            coords: { x: number; y: number };
            visible: boolean;
            interactable: boolean;
            label: string;
            token: string;
            type: string;
          }>;
        }
      ) => unknown;
    };

    const game = {
      getOverworld: () => ({
        _npc_on_tile: () => ({
          walking: false,
          jumping: false,
          event: { script: "" },
        }),
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
      }),
    };

    expect(
      sessionAny.readInteractionTarget(
        game as never,
        { x: 13, y: 7 },
        {
          hotspots: [
            {
              coords: { x: 13, y: 7 },
              visible: true,
              interactable: true,
              label: "Cyndaquil Poke Ball",
              token: "!",
              type: "objective",
            },
          ],
        }
      )
    ).toBeUndefined();
  });

  it("annotates live interaction targets with hotspot metadata from the map surface", async () => {
    const session = getMcpSession("status-live-interaction-target-metadata");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 13,
          player_y: 5,
          wXCoord: 13,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 13,
        player_y: 5,
        player_direction: "down",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 6] as [number, number],
        _counter_adjusted_tile: () => [13, 7] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_READ", script: "CyndaquilPokeBallScript" }),
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: 1,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "CYNDAQUIL_POKEBALL" },
          },
        ],
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 13,
      y: 7,
      kind: "bg_event",
      label: "Cyndaquil Poke Ball",
      token: "!",
      hotspot_type: "objective",
      script: "CyndaquilPokeBallScript",
    });
  });

  it("surfaces live interaction targets from raw overworld objects when helper lookups miss them", async () => {
    const session = getMcpSession("status-raw-object-interaction-target");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 7,
          wXCoord: 11,
          wYCoord: 7,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 7,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 7] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 13,
      y: 7,
      kind: "bg_event",
      label: "Cyndaquil Poke Ball",
      token: "!",
      hotspot_type: "objective",
      script: "CyndaquilPokeBallScript",
    });
  });

  it("surfaces blueprint-backed interaction targets when live object arrays are empty", async () => {
    const session = getMcpSession("status-blueprint-interaction-target");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 7,
          wXCoord: 11,
          wYCoord: 7,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 7,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 7] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [],
        _npc_blueprints: new Map([
          [
            "ElmsLab",
            new Map([
              [
                "ELMSLAB_POKE_BALL1",
                [{ x: 6, y: 3, script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" }, 3],
              ],
            ]),
          ],
        ]),
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 13,
      y: 7,
      kind: "bg_event",
      label: "Cyndaquil Poke Ball",
      token: "!",
      hotspot_type: "objective",
      script: "CyndaquilPokeBallScript",
    });
  });

  it("surfaces blueprint-backed Pokecenter healer targets from heal hotspots", async () => {
    const session = getMcpSession("status-blueprint-healer-interaction-target");
    const sessionAny = session as unknown as {
      readInteractionTarget: (
        game: unknown,
        interactionTile: { x: number; y: number },
        mapDetails: {
          map: string;
          coord_stride: number;
          hotspots: Array<{
            id: string;
            coords: { x: number; y: number };
            visible: boolean;
            interactable: boolean;
            label: string;
            token: string;
            type: string;
          }>;
        }
      ) => unknown;
    };

    const game = {
      getOverworld: () => ({
        current_map_name: "EcruteakPokecenter1F",
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        _npc_blueprints: new Map([
          [
            "EcruteakPokecenter1F",
            new Map([
              [
                "ECRUTEAKPOKECENTER1F_NURSE",
                [{ x: 7, y: 3, script: "EcruteakPokecenter1FNurseScript" }, 1],
              ],
            ]),
          ],
        ]),
      }),
    };

    expect(
      sessionAny.readInteractionTarget(
        game as never,
        { x: 7, y: 3 },
        {
          map: "EcruteakPokecenter1F",
          coord_stride: 2,
          hotspots: [
            {
              id: "heal-ecruteak",
              type: "heal",
              label: "Healer",
              coords: { x: 7, y: 3 },
              visible: true,
              interactable: true,
              token: "H",
            },
          ],
        }
      )
    ).toEqual({
      x: 7,
      y: 3,
      kind: "npc",
      label: "Healer",
      token: "H",
      hotspot_type: "heal",
      script: "EcruteakPokecenter1FNurseScript",
      object_index: 1,
    });
  });

  it("keeps an objective lane surfaced underfoot even when the current facing no longer confirms the target", async () => {
    const session = getMcpSession("status-underfoot-objective-lane-without-confirmed-target");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 13,
          player_y: 9,
          wXCoord: 13,
          wYCoord: 9,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 13,
        player_y: 9,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [15, 9] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toBeUndefined();
    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      lane: {
        x: 13,
        y: 9,
        facing: "up",
        facing_aligned: false,
        facing_move_leaves_lane: true,
        target_confirmed: false,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "bg_event",
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      recommended_approach: {
        x: 11,
        y: 7,
        facing: "right",
      },
    });
  });

  it("keeps npc interaction-target fallback truthful when only hotspot metadata identifies the tile", async () => {
    const session = getMcpSession("status-live-npc-interaction-target");
    const sessionAny = session as unknown as {
      readInteractionTarget: (
        game: unknown,
        interactionTile: { x: number; y: number },
        mapDetails: {
          hotspots: Array<{
            id: string;
            coords: { x: number; y: number };
            visible: boolean;
            interactable: boolean;
            label: string;
            token: string;
            type: string;
          }>;
        }
      ) => unknown;
    };

    const game = {
      getOverworld: () => ({
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
      }),
    };

    expect(
      sessionAny.readInteractionTarget(
        game as never,
        { x: 11, y: 5 },
        {
          hotspots: [
            {
              id: "npc-1",
              coords: { x: 11, y: 5 },
              visible: true,
              interactable: true,
              label: "Elm",
              token: "N",
              type: "npc",
            },
          ],
        }
      )
    ).toEqual({
      x: 11,
      y: 5,
      kind: "npc",
      label: "Elm",
      token: "N",
      hotspot_type: "npc",
      script: undefined,
    });
  });

  it("surfaces an npc scene owner from live runner interaction state when no bg-event owns the scene", async () => {
    const session = getMcpSession("status-npc-scene-owner");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 7,
          wXCoord: 11,
          wYCoord: 7,
          last_talked: 1,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 7,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        dialogue: null,
        _active_bg_event: null,
        script_runner: {
          _script_stack: [{ name: "ProfElmScript" }],
          _awaiting_resume: 0,
          stop_execution: false,
          is_busy: false,
          state: "running",
          last_interaction_object_index: 1,
        },
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.scene).toEqual({
      active_script: "ProfElmScript",
      scene_owner: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
        script: "ProfElmScript",
      },
    });
  });

  it("surfaces current-hotspot interaction setup when standing on an interactable tile without a live interaction target", async () => {
    const session = getMcpSession("status-current-hotspot-setup");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 5,
          wXCoord: 11,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 5,
        player_direction: "left",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 5] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.current_hotspot).toEqual({
      x: 11,
      y: 5,
      label: "Elm",
      token: "N",
      hotspot_type: "npc",
    });
    expect(snapshot.interaction_setup).toEqual({
      hotspot: {
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "current_hotspot",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("keeps an npc recovery approach surfaced when standing on the npc even after nearby lanes were already spent", async () => {
    const session = getMcpSession("status-current-hotspot-spent-npc-recovery");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "move:right:1",
        map: "ElmsLab",
        coords: { x: 9, y: 5 },
        result: { changed: true },
      },
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
      {
        action: "move:left:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: true },
      },
      {
        action: "move:left:1",
        map: "ElmsLab",
        coords: { x: 11, y: 5 },
        result: { changed: true },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 5,
          wXCoord: 11,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 5,
        player_direction: "left",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 5] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_setup).toEqual({
      hotspot: {
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "current_hotspot",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("pivots from a finished npc hotspot to the nearest visible objective after prompt resolution", async () => {
    const session = getMcpSession("status-current-hotspot-objective-pivot-after-prompt");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        moments?: string[];
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 9, y: 9 },
        moments: ["prompt_closed:dialogue"],
        result: { changed: true },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 5,
          wXCoord: 11,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 5,
        player_direction: "left",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 5] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.current_hotspot).toEqual({
      x: 11,
      y: 5,
      label: "Elm",
      token: "N",
      hotspot_type: "npc",
    });
    expect(snapshot.local_focus).toEqual({
      source: "visible_objective",
      target: {
        kind: "bg_event",
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      recommended_approach: {
        x: 11,
        y: 7,
        facing: "right",
      },
    });
  });

  it("surfaces interaction-lane state when the player is on a clean lane but facing the wrong way", async () => {
    const session = getMcpSession("status-interaction-lane-state");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 3,
          wXCoord: 11,
          wYCoord: 3,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 3,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [11, 1] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_READ", script: "ElmsLabWindow" }),
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      lane: {
        x: 11,
        y: 3,
        facing: "down",
        facing_aligned: false,
        facing_move_leaves_lane: true,
        target_confirmed: false,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("pivots local focus to a nearby npc after an objective lane confirm proved inert at the current tile", async () => {
    const session = getMcpSession("status-interaction-pivot-state");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 13,
          player_y: 5,
          wXCoord: 13,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 13,
        player_y: 5,
        player_direction: "down",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 7] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      lane: {
        x: 13,
        y: 5,
        facing: "down",
        facing_aligned: true,
        facing_move_leaves_lane: true,
        target_confirmed: true,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_pivot",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("does not surface an objective interaction lane in status when the live interaction target does not confirm it", async () => {
    const session = getMcpSession("status-suppresses-geometric-objective-lane");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 13,
          player_y: 5,
          wXCoord: 13,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 13,
        player_y: 5,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [15, 5] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toBeUndefined();
    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      lane: {
        x: 13,
        y: 5,
        facing: "down",
        facing_aligned: false,
        facing_move_leaves_lane: true,
        target_confirmed: false,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "bg_event",
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      recommended_approach: {
        x: 11,
        y: 7,
        facing: "right",
      },
    });
  });

  it("keeps a least-bad npc re-approach lane surfaced for interaction pivots instead of dropping to null", async () => {
    const session = getMcpSession("status-interaction-pivot-keeps-reapproach");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        result: { changed: false, reason: "no_change" },
      },
      {
        action: "move:up:1",
        map: "ElmsLab",
        coords: { x: 11, y: 5 },
        result: { changed: true },
      },
      {
        action: "move:left:1",
        map: "ElmsLab",
        coords: { x: 9, y: 5 },
        result: { changed: true },
      },
      {
        action: "move:up:1",
        map: "ElmsLab",
        coords: { x: 9, y: 3 },
        result: { changed: true },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 9,
          player_y: 3,
          wXCoord: 9,
          wYCoord: 3,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: 3,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 1] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.local_focus).toBeUndefined();
  });

  it("suppresses low-authority interaction lanes while an npc interaction pivot still owns the scene", async () => {
    const session = getMcpSession("status-suppresses-ambient-lane-during-npc-pivot");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 3,
          wXCoord: 11,
          wYCoord: 3,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 3,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 3] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_READ", script: "ElmsLabBookshelf" }),
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: {
          warps: [],
          bg_events: [{ x: 6, y: 1, event_type: "BGEVENT_READ", script: "ElmsLabBookshelf" }],
          coord_events: [],
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 13,
      y: 3,
      kind: "bg_event",
      label: "Bookshelf",
      token: "B",
      hotspot_type: "sign",
      script: "ElmsLabBookshelf",
    });
    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      lane: {
        x: 11,
        y: 3,
        facing: "down",
        facing_aligned: false,
        facing_move_leaves_lane: true,
        target_confirmed: false,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_pivot",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("keeps a least-bad npc recovery lane surfaced on an overlapping objective lane after a recent inert confirm", async () => {
    const session = getMcpSession("status-overlapping-interaction-pivot-state");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 7,
          wXCoord: 11,
          wYCoord: 7,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 7,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 7] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      lane: {
        x: 11,
        y: 7,
        facing: "right",
        facing_aligned: true,
        facing_move_leaves_lane: true,
        target_confirmed: true,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_pivot",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("does not collapse an overlapping objective lane to npc focus when only low-authority scenery is confirmed ahead", async () => {
    const session = getMcpSession("status-overlap-low-authority-ambiguity");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 13,
          player_y: 5,
          wXCoord: 13,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 13,
        player_y: 5,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 3] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_READ", script: "ElmsLabBookshelf" }),
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: {
          warps: [],
          bg_events: [{ x: 6, y: 1, event_type: "BGEVENT_READ", script: "ElmsLabBookshelf" }],
          coord_events: [],
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 13,
      y: 3,
      kind: "bg_event",
      label: "Bookshelf",
      token: "B",
      hotspot_type: "sign",
      script: "ElmsLabBookshelf",
    });
    expect(snapshot.interaction_lane).toBeUndefined();
    expect(snapshot.local_focus).toBeUndefined();
  });

  it("prefers the least-bad nearby npc recovery lane after a recent inert objective confirm in the same scene", async () => {
    const session = getMcpSession("status-nearby-interaction-pivot-state");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 9,
          player_y: 3,
          wXCoord: 9,
          wYCoord: 3,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: 3,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 1] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.local_focus).toBeUndefined();
  });

  it("drops a stale npc interaction pivot once that npc recovery lane also produced an inert confirm", async () => {
    const session = getMcpSession("status-drops-stale-npc-pivot-after-inert-recovery");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 7,
          wXCoord: 11,
          wYCoord: 7,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 7,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 7] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      lane: {
        x: 11,
        y: 7,
        facing: "right",
        facing_aligned: true,
        facing_move_leaves_lane: true,
        target_confirmed: true,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "bg_event",
        x: 13,
        y: 7,
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspot_type: "objective",
      },
      recommended_approach: {
        x: 13,
        y: 9,
        facing: "up",
      },
    });
  });

  it("avoids recommending a just-visited npc recovery lane when another clean lane is available", async () => {
    const session = getMcpSession("status-avoids-recent-npc-recovery-lane");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "move:left:1",
        map: "ElmsLab",
        coords: { x: 11, y: 3 },
        result: { changed: true },
      },
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 9,
          player_y: 5,
          wXCoord: 9,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: 5,
        player_direction: "down",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 7] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 11,
        y: 7,
        facing: "up",
      },
    });
  });

  it("does not immediately recommend the same approach tile that was just used to step onto an npc hotspot", async () => {
    const session = getMcpSession("status-avoids-immediate-npc-backtrack");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        result: { changed: false, reason: "no_change" },
      },
      {
        action: "move:right:1",
        map: "ElmsLab",
        coords: { x: 11, y: 5 },
        result: { changed: true },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "right");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 5,
          wXCoord: 11,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 5,
        player_direction: "right",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [13, 5] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.local_focus).toEqual({
      source: "interaction_pivot",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: expect.objectContaining({
        x: expect.any(Number),
        y: expect.any(Number),
        facing: expect.any(String),
      }),
    });
    expect(snapshot.local_focus?.recommended_approach).not.toEqual({
      x: 9,
      y: 5,
      facing: "right",
    });
  });

  it("keeps a least-bad npc recovery lane surfaced once every candidate lane was already spent recently", async () => {
    const session = getMcpSession("status-no-clean-npc-recovery-lane");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 13, y: 5 },
        result: { changed: false, reason: "no_change" },
      },
      {
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        result: { changed: false, reason: "no_change" },
      },
      {
        action: "move:up:1",
        map: "ElmsLab",
        coords: { x: 11, y: 3 },
        result: { changed: true },
      },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 9,
          player_y: 5,
          wXCoord: 9,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: 5,
        player_direction: "left",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [7, 5] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
    });
  });

  it("does not resurrect an older spent npc recovery lane once the run has drifted away from the failed objective scene", async () => {
    const session = getMcpSession("status-long-loop-spent-npc-recovery-lane");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      { action: "move:right:1", map: "ElmsLab", coords: { x: 13, y: 5 }, result: { changed: true } },
      { action: "move:left:1", map: "ElmsLab", coords: { x: 11, y: 5 }, result: { changed: true } },
      { action: "press:a:1", map: "ElmsLab", coords: { x: 11, y: 7 }, result: { changed: false, reason: "no_change" } },
      { action: "move:up:2", map: "ElmsLab", coords: { x: 9, y: 3 }, result: { changed: false, reason: "blocked" } },
      { action: "move:down:1", map: "ElmsLab", coords: { x: 9, y: 5 }, result: { changed: true } },
      { action: "move:up:1", map: "ElmsLab", coords: { x: 9, y: 3 }, result: { changed: true } },
      { action: "move:down:1", map: "ElmsLab", coords: { x: 9, y: 5 }, result: { changed: true } },
      { action: "move:up:2", map: "ElmsLab", coords: { x: 9, y: 3 }, result: { changed: false, reason: "blocked" } },
      { action: "move:down:1", map: "ElmsLab", coords: { x: 9, y: 5 }, result: { changed: true } },
      { action: "move:up:2", map: "ElmsLab", coords: { x: 9, y: 3 }, result: { changed: false, reason: "blocked" } },
      { action: "move:down:1", map: "ElmsLab", coords: { x: 9, y: 5 }, result: { changed: true } },
      { action: "move:right:1", map: "ElmsLab", coords: { x: 11, y: 5 }, result: { changed: true } },
      { action: "move:right:1", map: "ElmsLab", coords: { x: 13, y: 5 }, result: { changed: true } },
      { action: "move:left:1", map: "ElmsLab", coords: { x: 11, y: 5 }, result: { changed: true } },
      { action: "press:a:1", map: "ElmsLab", coords: { x: 11, y: 5 }, result: { changed: false, reason: "no_change" } },
    ];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 9,
          player_y: 3,
          wXCoord: 9,
          wYCoord: 3,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 9,
        player_y: 3,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [9, 1] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => null,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
          {
            objectIndex: 3,
            x: 13,
            y: 7,
            event: { script: "CyndaquilPokeBallScript", object_identifier: "ELMSLAB_POKE_BALL1" },
          },
        ],
        _map_events: { warps: [], bg_events: [], coord_events: [] },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.local_focus).toBeUndefined();
  });

  it("does not let ambient landmark lanes claim local focus over the live scene", async () => {
    const session = getMcpSession("status-ambient-lane-local-focus");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 3,
          wXCoord: 11,
          wYCoord: 3,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 3,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [11, 1] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_READ", script: "ElmsLabWindow" }),
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
        ],
        _map_events: {
          warps: [],
          bg_events: [{ x: 5, y: 0, script: "ElmsLabWindow", type: "BGEVENT_READ" }],
          coord_events: [],
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 11,
      y: 1,
      kind: "bg_event",
      label: "Window",
      token: "W",
      hotspot_type: "landmark",
      script: "ElmsLabWindow",
    });
    expect(snapshot.interaction_lane).toEqual({
      hotspot: {
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      lane: {
        x: 11,
        y: 3,
        facing: "down",
        facing_aligned: false,
        facing_move_leaves_lane: true,
        target_confirmed: false,
      },
    });
    expect(snapshot.local_focus).toEqual({
      source: "interaction_lane",
      target: {
        kind: "npc",
        x: 11,
        y: 5,
        label: "Elm",
        token: "N",
        hotspot_type: "npc",
      },
      recommended_approach: {
        x: 9,
        y: 5,
        facing: "right",
      },
    });
  });

  it("does not let utility lanes claim local focus over nearby story work", async () => {
    const session = getMcpSession("status-utility-lane-local-focus");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        map?: string;
        coords?: { x: number; y: number };
        result: { changed: boolean; reason?: string };
      }>;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 7,
          player_y: 9,
          wXCoord: 7,
          wYCoord: 9,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 7,
        player_y: 9,
        player_direction: "down",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        get_facing_tile_coords: () => [7, 11] as [number, number],
        _counter_adjusted_tile: (x: number, y: number) => [x, y] as [number, number],
        _npc_on_tile: () => null,
        _nearest_npc_covering_subtile: () => null,
        _bg_event_at: () => ({ event_type: "BGEVENT_DOWN", script: "ElmsLabPC" }),
        _bg_event_allowed_by_flags: () => true,
        npcs: [
          {
            objectIndex: 1,
            x: 11,
            y: 5,
            event: { script: "ProfElmScript", object_identifier: "ELMSLAB_ELM" },
          },
        ],
        _map_events: {
          warps: [],
          bg_events: [{ x: 3, y: 5, event_type: "BGEVENT_DOWN", script: "ElmsLabPC" }],
          coord_events: [],
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.interaction_target).toEqual({
      x: 7,
      y: 11,
      kind: "bg_event",
      label: "PC",
      token: "P",
      hotspot_type: "utility",
      script: "ElmsLabPC",
    });
    expect(snapshot.interaction_lane).toBeUndefined();
    expect(snapshot.local_focus).toBeUndefined();
  });

  it("surfaces active scene context from the live runtime when a bg-event script owns the current scene", async () => {
    const session = getMcpSession("status-scene-context");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 11,
          player_y: 7,
          wXCoord: 11,
          wYCoord: 7,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 7,
        player_direction: "down",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        dialogue: null,
        _active_bg_event: { event_type: "BGEVENT_READ", script: "LabTryToLeaveScript", x: 9, y: 13 },
        _map_events: {
          warps: [],
          bg_events: [{ x: 4, y: 6, event_type: "BGEVENT_READ", script: "LabTryToLeaveScript" }],
          coord_events: [],
        },
        script_runner: {
          _script_stack: [{ name: "LabTryToLeaveScript" }],
          _awaiting_resume: 0,
          stop_execution: false,
          is_busy: false,
          state: "running",
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.scene).toEqual({
      active_script: "LabTryToLeaveScript",
      scene_owner: {
        kind: "bg_event",
        x: 9,
        y: 13,
        label: "Sign",
        token: "S",
        hotspot_type: "sign",
        script: "LabTryToLeaveScript",
      },
    });
  });

  it("surfaces live current-map bg hotspots in status even when state-level loader data is absent", async () => {
    const session = getMcpSession("status-live-map-hotspots");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.readFacingDirection = jest.fn(() => "up");
    sessionAny.readBestMapName = jest.fn(() => "ElmsLab");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "ElmsLab", group: 24, number: 5 }));

    const game = {
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "ElmsLab",
      getGameState: () => ({
        wram: {
          player_x: 15,
          player_y: 5,
          wXCoord: 15,
          wYCoord: 5,
          event_flags: {},
        },
        sram: { party: { pokemon: [] } },
      }),
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 15,
        player_y: 5,
        player_direction: "up",
        player_movement_locked: () => false,
        script_tasks_active: () => false,
        script_runner: null,
        dialogue: null,
        _map_events: {
          warps: [],
          bg_events: [{ x: 6, y: 1, event_type: "BGEVENT_READ", script: "ElmsLabBookshelf" }],
          coord_events: [],
        },
      }),
    };

    sessionAny.getGame = jest.fn(() => game);

    const snapshot = await session.status();

    expect(snapshot.map_details?.hotspots).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          type: "sign",
          label: "Bookshelf",
          coords: { x: 13, y: 3 },
          token: "B",
        }),
      ])
    );
  });

  it("prefers settled player object coords over stale overworld and WRAM coords when serializing runtime snapshots", () => {
    const snapshot = __testing.serializeRuntimeSnapshot(42, {
      getMapName: () => "CherrygroveMart",
      getGameState: () => ({
        wram: {
          wXCoord: 7,
          wYCoord: 15,
        },
      }),
      getOverworld: () => ({
        current_map_name: "CherrygroveMart",
        player_x: 7,
        player_y: 15,
        prev_player_x: 5,
        prev_player_y: 15,
        player_object: {
          x: 5,
          y: 15,
          direction: "down",
        },
        player_direction: "right",
        npcs: [],
        script_runner: null,
        dialogue: null,
      }),
    } as never);

    expect(snapshot?.player).toEqual({ x: 5, y: 15, prevX: 5, prevY: 15, direction: "down" });
  });

  it("preserves the last known good runtime player coords when a new snapshot cannot resolve them", () => {
    const merged = __testing.mergeRuntimeSnapshotWithPrevious(
      {
        version: 1,
        frameCounter: 12,
        map: "ElmsLab",
        player: { x: 9, y: 7, prevX: 9, prevY: 7, direction: "left" },
        npcs: [],
        runner: null,
        dialogue: null,
      },
      {
        version: 1,
        frameCounter: 16,
        map: "ElmsLab",
        player: null,
        npcs: [],
        runner: null,
        dialogue: null,
      }
    );

    expect(merged.player).toEqual({ x: 9, y: 7, prevX: 9, prevY: 7, direction: "left" });
    expect(merged.frameCounter).toBe(16);
  });

  it("preserves the last known good runtime npc list when a same-map snapshot drops objects", () => {
    const merged = __testing.mergeRuntimeSnapshotWithPrevious(
      {
        version: 1,
        frameCounter: 12,
        map: "ElmsLab",
        player: { x: 9, y: 7, prevX: 9, prevY: 7, direction: "left" },
        npcs: [{ objectIndex: 3, x: 13, y: 7, prevX: 13, prevY: 7, direction: "down" }],
        runner: null,
        dialogue: null,
      },
      {
        version: 1,
        frameCounter: 16,
        map: "ElmsLab",
        player: { x: 11, y: 7, prevX: 9, prevY: 7, direction: "right" },
        npcs: [],
        runner: null,
        dialogue: null,
      }
    );

    expect(merged.npcs).toEqual([{ objectIndex: 3, x: 13, y: 7, prevX: 13, prevY: 7, direction: "down" }]);
    expect(merged.frameCounter).toBe(16);
  });

  it("serializes runtime npc state even when prev coords are temporarily missing", () => {
    const snapshot = __testing.serializeRuntimeSnapshot(42, {
      getMapName: () => "ElmsLab",
      getOverworld: () => ({
        current_map_name: "ElmsLab",
        player_x: 11,
        player_y: 5,
        prev_player_x: 11,
        prev_player_y: 5,
        player_direction: "left",
        npcs: [{ objectIndex: 3, x: 13, y: 7, direction: "down" }],
        script_runner: null,
        dialogue: null,
      }),
    } as never);

    expect(snapshot?.npcs).toEqual([{ objectIndex: 3, x: 13, y: 7, prevX: 13, prevY: 7, direction: "down" }]);
  });

  it("applies runtime resume state back onto the live overworld shell", () => {
    const syncPlayerState = jest.fn();
    const primeActiveWarpTile = jest.fn(function (this: { current_map_name: string; player_x: number; player_y: number; _active_warp_tile?: [string, number, number] | null }) {
      this._active_warp_tile = [this.current_map_name, this.player_x, this.player_y];
    });
    const clearWindow = jest.fn();
    const openWindow = jest.fn();
    const completeWindow = jest.fn();
    const updateNpcPixelPosition = jest.fn();
    const updatePlayerPixelPosition = jest.fn();
    const clearStaleBlockingTasks = jest.fn();
    const resetInputState = jest.fn();
    const parse = jest.fn(() => [{ name: "cmd1" }, { name: "cmd2" }, { name: "cmd3" }]);
    const overworld = {
      current_map_name: "ElmsLab",
      player_x: 1,
      player_y: 1,
      prev_player_x: 1,
      prev_player_y: 1,
      player_direction: "down",
      target_tile_x: 4,
      target_tile_y: 4,
      is_moving: true,
      step_progress_px: 8,
      step_dx_px: 1,
      step_dy_px: -1,
      _queued_direction: "up",
      _turning_direction: "left",
      _turn_frames_remaining: 2,
      _turn_should_force_step: true,
      _pending_auto_step: "up",
      _active_warp_tile: null,
      _sync_player_state: syncPlayerState,
      _prime_active_warp_tile_for_current_position: primeActiveWarpTile,
      _script_task_queue: [{ stale: true }],
      _active_script_task: { stale: true },
      _blocking_task_count: 1,
      _blocking_movement_lock_active: true,
      _ignore_a_until_release: true,
      _clear_stale_blocking_tasks: clearStaleBlockingTasks,
      reset_input_state: resetInputState,
      player_object: {
        x: 1,
        y: 1,
        prevX: 1,
        prevY: 1,
        direction: "down",
        updatePixelPosition: updatePlayerPixelPosition,
      },
      npcs: [
        {
          objectIndex: 1,
          x: 1,
          y: 1,
          prevX: 1,
          prevY: 1,
          direction: "down",
          updatePixelPosition: updateNpcPixelPosition,
        },
      ],
      script_runner: {
        dataLoader: {
          get_script: jest.fn(() => [{ command: "opentext" }, { command: "writetext" }, { command: "end" }]),
        },
        parse,
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        _pause_execution: false,
        last_yes_no_result: false,
        last_condition_result: false,
        pending_reload_map: null,
        last_interaction_object_index: null,
        variables: { stale: true },
      },
      dialogue: {
        ui: { screen: null },
        window: { clear: clearWindow, open: openWindow, complete: completeWindow },
        visible: false,
        waiting_for_input: false,
        script_paused: false,
        pendingWaits: 0,
        pending_script_waits: 0,
        current_text: "",
        pending_text: [],
        auto_close_requested: false,
        ignore_confirm_until_release: false,
        yes_no_prompt: null,
      },
    };
    const game = {
      getMapName: () => "ElmsLab",
      getOverworld: () => overworld,
    };

    const restored = __testing.applyRuntimeSnapshot(game as never, {
      version: 1,
      frameCounter: 12,
      map: "ElmsLab",
      player: { x: 9, y: 7, prevX: 9, prevY: 9, direction: "left" },
      npcs: [{ objectIndex: 1, x: 6, y: 8, prevX: 6, prevY: 8, direction: "right" }],
      runner: {
        stack: [{ name: "ElmsLabWalkUpToElmScript", index: 2, allowFallthrough: false }],
        awaitingResume: 1,
        queuedOverworldTasks: 0,
        stopExecution: true,
        lastYesNoResult: true,
        lastConditionResult: true,
        pendingReloadMap: null,
        lastInteractionObjectIndex: 1,
        variables: {
          _loaded_menu: {
            label: "GoldenrodGameCornerTMVendorMenuHeader",
            options: ["TM25    5500", "TM14    5500", "TM38    5500", "CANCEL"],
          },
        },
      },
      dialogue: {
        visible: true,
        waitingForInput: true,
        scriptPaused: true,
        pendingWaits: 1,
        pendingScriptWaits: 1,
        currentText: "Would you like to help me?",
        pendingText: [],
        autoCloseRequested: false,
        ignoreConfirmUntilRelease: true,
        yesNoSelection: 1,
      },
    });

    expect(restored).toBe(true);
    expect(overworld.player_x).toBe(9);
    expect(overworld.player_y).toBe(7);
    expect(overworld.player_direction).toBe("left");
    expect(overworld.target_tile_x).toBe(9);
    expect(overworld.target_tile_y).toBe(7);
    expect(overworld.player_object).toMatchObject({
      x: 9,
      y: 7,
      prevX: 9,
      prevY: 9,
      direction: "left",
    });
    expect(updatePlayerPixelPosition).toHaveBeenCalledTimes(1);
    expect(overworld.is_moving).toBe(false);
    expect(overworld.step_progress_px).toBe(0);
    expect(overworld.step_dx_px).toBe(0);
    expect(overworld.step_dy_px).toBe(0);
    expect(overworld._queued_direction).toBeNull();
    expect(overworld._turning_direction).toBeNull();
    expect(overworld._turn_frames_remaining).toBe(0);
    expect(overworld._turn_should_force_step).toBe(false);
    expect(overworld._pending_auto_step).toBeNull();
    expect(overworld._active_warp_tile).toEqual(["ElmsLab", 9, 7]);
    expect(syncPlayerState).toHaveBeenCalled();
    expect(primeActiveWarpTile).toHaveBeenCalledTimes(1);
    expect(overworld.npcs[0]?.x).toBe(6);
    expect(overworld.npcs[0]?.direction).toBe("right");
    expect(updateNpcPixelPosition).toHaveBeenCalled();
    expect(parse).toHaveBeenCalled();
    expect((overworld.script_runner as { _script_stack: unknown[] })._script_stack).toHaveLength(1);
    expect((overworld.script_runner as { variables: Record<string, unknown> }).variables._loaded_menu).toEqual({
      label: "GoldenrodGameCornerTMVendorMenuHeader",
      options: ["TM25    5500", "TM14    5500", "TM38    5500", "CANCEL"],
    });
    expect((overworld.script_runner as { variables: Record<string, unknown> }).variables.stale).toBeUndefined();
    expect((overworld.script_runner as { _queued_overworld_task_count: number })._queued_overworld_task_count).toBe(0);
    expect(overworld._active_script_task).toBeNull();
    expect(overworld._script_task_queue).toEqual([]);
    expect(overworld._blocking_task_count).toBe(0);
    expect(overworld._blocking_movement_lock_active).toBe(false);
    expect(overworld._ignore_a_until_release).toBe(false);
    expect(resetInputState).toHaveBeenCalled();
    expect(clearStaleBlockingTasks).toHaveBeenCalled();
    expect(clearWindow).toHaveBeenCalled();
    expect(openWindow).toHaveBeenCalledWith("Would you like to help me?");
    expect(completeWindow).toHaveBeenCalled();
    expect((overworld.dialogue as { ignore_confirm_until_release: boolean }).ignore_confirm_until_release).toBe(false);
    expect((overworld.dialogue as { yes_no_prompt: { selection: number } | null }).yes_no_prompt?.selection).toBe(1);
  });

  it("re-primes the active warp tile when restoring onto a doorway tile", () => {
    const primeActiveWarpTile = jest.fn(function (this: { current_map_name: string; player_x: number; player_y: number; _active_warp_tile?: [string, number, number] | null }) {
      this._active_warp_tile = [this.current_map_name, this.player_x, this.player_y];
    });
    const overworld = {
      current_map_name: "CherrygroveCity",
      player_x: 47,
      player_y: 9,
      prev_player_x: 47,
      prev_player_y: 9,
      player_direction: "down",
      target_tile_x: 47,
      target_tile_y: 9,
      is_moving: false,
      step_progress_px: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      _queued_direction: null,
      _turning_direction: null,
      _turn_frames_remaining: 0,
      _turn_should_force_step: false,
      _pending_auto_step: null,
      _active_warp_tile: null,
      _sync_player_state: jest.fn(),
      _prime_active_warp_tile_for_current_position: primeActiveWarpTile,
      _script_task_queue: [],
      _active_script_task: null,
      _blocking_task_count: 0,
      _blocking_movement_lock_active: false,
      _ignore_a_until_release: false,
      _clear_stale_blocking_tasks: jest.fn(),
      reset_input_state: jest.fn(),
      npcs: [],
      script_runner: null,
      dialogue: null,
    };
    const game = {
      getMapName: () => "CherrygroveCity",
      getOverworld: () => overworld,
    };

    const restored = __testing.applyRuntimeSnapshot(game as never, {
      version: 1,
      frameCounter: 12,
      map: "CherrygroveCity",
      player: { x: 47, y: 7, prevX: 47, prevY: 7, direction: "down" },
      npcs: [],
      runner: null,
      dialogue: null,
    });

    expect(restored).toBe(true);
    expect(primeActiveWarpTile).toHaveBeenCalledTimes(1);
    expect(overworld._active_warp_tile).toEqual(["CherrygroveCity", 47, 7]);
  });

  it("clears A-release suppression on runtime restore even without dialogue state", () => {
    const overworld = {
      current_map_name: "Route29",
      player_x: 1,
      player_y: 1,
      prev_player_x: 1,
      prev_player_y: 1,
      player_direction: "down",
      target_tile_x: 1,
      target_tile_y: 1,
      is_moving: false,
      step_progress_px: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      _queued_direction: null,
      _turning_direction: null,
      _turn_frames_remaining: 0,
      _turn_should_force_step: false,
      _pending_auto_step: null,
      _sync_player_state: jest.fn(),
      _ignore_a_until_release: true,
      npcs: [],
      script_runner: null,
      dialogue: null,
    };
    const game = {
      getMapName: () => "Route29",
      getOverworld: () => overworld,
    };

    const restored = __testing.applyRuntimeSnapshot(game as never, {
      version: 1,
      frameCounter: 1,
      map: "Route29",
      player: { x: 101, y: 27, prevX: 101, prevY: 27, direction: "up" },
      npcs: [],
      runner: null,
      dialogue: null,
    });

    expect(restored).toBe(true);
    expect(overworld._ignore_a_until_release).toBe(false);
  });

  it("clears stale blocking tasks on runtime restore even when no dialogue state is present", () => {
    const clearStaleBlockingTasks = jest.fn();
    const resetInputState = jest.fn();
    const overworld = {
      current_map_name: "ElmsLab",
      player_x: 11,
      player_y: 7,
      prev_player_x: 11,
      prev_player_y: 7,
      player_direction: "down",
      target_tile_x: 11,
      target_tile_y: 7,
      is_moving: false,
      step_progress_px: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      _queued_direction: null,
      _turning_direction: null,
      _turn_frames_remaining: 0,
      _turn_should_force_step: false,
      _pending_auto_step: null,
      _sync_player_state: jest.fn(),
      _script_task_queue: [{ stale: true }],
      _active_script_task: { stale: true },
      _blocking_task_count: 2,
      _blocking_movement_lock_active: true,
      _movement_lock_count: 3,
      _text_lock_active: true,
      input_capture_active: true,
      _clear_stale_blocking_tasks: clearStaleBlockingTasks,
      reset_input_state: resetInputState,
      _ignore_a_until_release: false,
      npcs: [],
      script_runner: null,
      dialogue: null,
    };
    const game = {
      getMapName: () => "ElmsLab",
      getOverworld: () => overworld,
    };

    const restored = __testing.applyRuntimeSnapshot(game as never, {
      version: 1,
      frameCounter: 1,
      map: "ElmsLab",
      player: { x: 11, y: 7, prevX: 11, prevY: 7, direction: "up" },
      npcs: [],
      runner: null,
      dialogue: null,
    });

    expect(restored).toBe(true);
    expect(overworld._active_script_task).toBeNull();
    expect(overworld._script_task_queue).toEqual([]);
    expect(overworld._blocking_task_count).toBe(0);
    expect(overworld._blocking_movement_lock_active).toBe(false);
    expect(overworld._movement_lock_count).toBe(0);
    expect(overworld._text_lock_active).toBe(false);
    expect(overworld.input_capture_active).toBe(false);
    expect(resetInputState).toHaveBeenCalled();
    expect(clearStaleBlockingTasks).toHaveBeenCalled();
  });

  it("does not restore script stacks paused behind queued overworld tasks", () => {
    const overworld = {
      current_map_name: "AzaleaTown",
      player_x: 11,
      player_y: 23,
      prev_player_x: 13,
      prev_player_y: 23,
      player_direction: "right",
      target_tile_x: 11,
      target_tile_y: 23,
      is_moving: false,
      step_progress_px: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      _queued_direction: null,
      _turning_direction: null,
      _turn_frames_remaining: 0,
      _turn_should_force_step: false,
      _pending_auto_step: null,
      _sync_player_state: jest.fn(),
      _script_task_queue: [],
      _active_script_task: null,
      _blocking_task_count: 0,
      _blocking_movement_lock_active: false,
      _movement_lock_count: 0,
      _text_lock_active: false,
      input_capture_active: false,
      _clear_stale_blocking_tasks: jest.fn(),
      reset_input_state: jest.fn(),
      _ignore_a_until_release: false,
      npcs: [],
      script_runner: {
        dataLoader: {
          get_script: jest.fn(() => [{ command: "applymovement" }, { command: "turnobject" }]),
        },
        parse: jest.fn((scriptData: unknown[]) => scriptData),
        _script_stack: [],
        _awaiting_resume: 0,
        _queued_overworld_task_count: 0,
        stop_execution: false,
        _pause_execution: false,
        pending_reload_map: null,
        last_interaction_object_index: null,
      },
      dialogue: null,
    };
    const game = {
      getMapName: () => "AzaleaTown",
      getOverworld: () => overworld,
    };

    const restored = __testing.applyRuntimeSnapshot(game as never, {
      version: 1,
      frameCounter: 1,
      map: "AzaleaTown",
      player: { x: 11, y: 23, prevX: 13, prevY: 23, direction: "right" },
      npcs: [],
      runner: {
        stack: [{ name: "AzaleaTownRivalBattleScene2", index: 6, allowFallthrough: true }],
        awaitingResume: 1,
        queuedOverworldTasks: 1,
        stopExecution: true,
        lastYesNoResult: false,
        lastConditionResult: false,
        pendingReloadMap: null,
        lastInteractionObjectIndex: null,
      },
      dialogue: null,
    });

    expect(restored).toBe(true);
    expect(overworld.script_runner._script_stack).toEqual([]);
    expect(overworld.script_runner._awaiting_resume).toBe(0);
    expect(overworld.script_runner._queued_overworld_task_count).toBe(0);
    expect(overworld.script_runner.stop_execution).toBe(false);
    expect(overworld.script_runner._pause_execution).toBe(false);
    expect(overworld._movement_lock_count).toBe(0);
    expect(overworld._blocking_task_count).toBe(0);
    expect(overworld.input_capture_active).toBe(false);
  });

  it("clears stale runner and dialogue state when the runtime snapshot has neither", () => {
    const forceCloseText = jest.fn();
    const resume = jest.fn();
    const overworld = {
      current_map_name: "ElmsLab",
      player_x: 1,
      player_y: 1,
      prev_player_x: 1,
      prev_player_y: 1,
      player_direction: "down",
      target_tile_x: 1,
      target_tile_y: 1,
      is_moving: false,
      step_progress_px: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      _queued_direction: null,
      _turning_direction: null,
      _turn_frames_remaining: 0,
      _turn_should_force_step: false,
      _pending_auto_step: null,
      _sync_player_state: jest.fn(),
      _script_task_queue: [],
      _active_script_task: null,
      _blocking_task_count: 0,
      _blocking_movement_lock_active: false,
      _movement_lock_count: 0,
      _text_lock_active: false,
      input_capture_active: false,
      _clear_stale_blocking_tasks: jest.fn(),
      reset_input_state: jest.fn(),
      _ignore_a_until_release: false,
      npcs: [],
      script_runner: {
        _script_stack: [{ name: "StaleScript" }],
        _awaiting_resume: 2,
        _queued_overworld_task_count: 1,
        stop_execution: true,
        _pause_execution: true,
        pending_reload_map: "ElmsLab",
        last_interaction_object_index: 4,
      },
      dialogue: {
        ui: {},
        window: { clear: jest.fn(), open: jest.fn(), complete: jest.fn() },
        forceCloseText,
        resume,
        visible: true,
        waiting_for_input: true,
        script_paused: true,
        pendingWaits: 1,
        pending_script_waits: 1,
        current_text: "Choose a POKeMON.",
        pending_text: ["YES"],
        auto_close_requested: true,
        ignore_confirm_until_release: true,
        pending_yes_no_request: true,
        pending_yes_no_callback: jest.fn(),
        yes_no_callback: jest.fn(),
        suspended: true,
        _suppress_orphan_close: true,
        yes_no_prompt: { selection: 1 },
      },
    };
    const game = {
      getMapName: () => "ElmsLab",
      getOverworld: () => overworld,
    };

    const restored = __testing.applyRuntimeSnapshot(game as never, {
      version: 1,
      frameCounter: 1,
      map: "ElmsLab",
      player: { x: 9, y: 9, prevX: 9, prevY: 9, direction: "right" },
      npcs: [],
      runner: null,
      dialogue: null,
    });

    expect(restored).toBe(true);
    expect(overworld.script_runner?._script_stack).toEqual([]);
    expect(overworld.script_runner?._awaiting_resume).toBe(0);
    expect(overworld.script_runner?._queued_overworld_task_count).toBe(0);
    expect(overworld.script_runner?.stop_execution).toBe(false);
    expect(overworld.script_runner?._pause_execution).toBe(false);
    expect(overworld.script_runner?.pending_reload_map).toBeNull();
    expect(overworld.script_runner?.last_interaction_object_index).toBeNull();
    expect(resume).toHaveBeenCalled();
    expect(forceCloseText).toHaveBeenCalled();
    expect(overworld.dialogue?.visible).toBe(false);
    expect(overworld.dialogue?.waiting_for_input).toBe(false);
    expect(overworld.dialogue?.script_paused).toBe(false);
    expect(overworld.dialogue?.pendingWaits).toBe(0);
    expect(overworld.dialogue?.pending_script_waits).toBe(0);
    expect(overworld.dialogue?.current_text).toBe("");
    expect(overworld.dialogue?.pending_text).toEqual([]);
    expect(overworld.dialogue?.auto_close_requested).toBe(false);
    expect(overworld.dialogue?.ignore_confirm_until_release).toBe(false);
    expect(overworld.dialogue?.pending_yes_no_request).toBe(false);
    expect(overworld.dialogue?.pending_yes_no_callback).toBeNull();
    expect(overworld.dialogue?.yes_no_callback).toBeNull();
    expect(overworld.dialogue?.suspended).toBe(false);
    expect(overworld.dialogue?._suppress_orphan_close).toBe(false);
    expect(overworld.dialogue?.yes_no_prompt).toBeNull();
  });

  it("restores recent action events from the runtime snapshot so interaction pivots survive resume", async () => {
    const session = getMcpSession("runtime-restore-action-events");
    const sessionAny = session as unknown as {
      game: {
        getMapName: () => string;
        getOverworld: () => {
          current_map_name: string;
          player_x: number;
          player_y: number;
          prev_player_x: number;
          prev_player_y: number;
          player_direction: string;
          target_tile_x: number;
          target_tile_y: number;
          is_moving: boolean;
          step_progress_px: number;
          step_dx_px: number;
          step_dy_px: number;
          _queued_direction: string | null;
          _turning_direction: string | null;
          _turn_frames_remaining: number;
          _turn_should_force_step: boolean;
          _pending_auto_step: string | null;
          _sync_player_state: jest.Mock;
          _script_task_queue: unknown[];
          _active_script_task: unknown | null;
          _blocking_task_count: number;
          _blocking_movement_lock_active: boolean;
          _clear_stale_blocking_tasks: jest.Mock;
          reset_input_state: jest.Mock;
          _ignore_a_until_release: boolean;
          player_object: {
            x: number;
            y: number;
            prevX: number;
            prevY: number;
            direction: string;
            updatePixelPosition: jest.Mock;
          };
          npcs: [];
          script_runner: null;
          dialogue: null;
        };
      };
      actionEvents: unknown[];
      readRuntimeSnapshot: jest.Mock;
      restoreRuntimeSnapshot: () => Promise<void>;
    };

    const overworld = {
      current_map_name: "ElmsLab",
      player_x: 1,
      player_y: 1,
      prev_player_x: 1,
      prev_player_y: 1,
      player_direction: "down",
      target_tile_x: 1,
      target_tile_y: 1,
      is_moving: false,
      step_progress_px: 0,
      step_dx_px: 0,
      step_dy_px: 0,
      _queued_direction: null,
      _turning_direction: null,
      _turn_frames_remaining: 0,
      _turn_should_force_step: false,
      _pending_auto_step: null,
      _sync_player_state: jest.fn(),
      _script_task_queue: [],
      _active_script_task: null,
      _blocking_task_count: 0,
      _blocking_movement_lock_active: false,
      _clear_stale_blocking_tasks: jest.fn(),
      reset_input_state: jest.fn(),
      _ignore_a_until_release: false,
      player_object: {
        x: 11,
        y: 7,
        prevX: 11,
        prevY: 7,
        direction: "down",
        updatePixelPosition: jest.fn(),
      },
      npcs: [],
      script_runner: null,
      dialogue: null,
    };

    sessionAny.game = {
      getMapName: () => "ElmsLab",
      getGameState: () => ({ wram: { wXCoord: 11, wYCoord: 7 } }),
      getOverworld: () => overworld,
    };
    sessionAny.actionEvents = [];
    sessionAny.readRuntimeSnapshot = jest.fn().mockResolvedValue({
      slot: "/tmp/runtime-restore-action-events.json",
      snapshot: {
        version: 1,
        frameCounter: 42,
        map: "ElmsLab",
        player: { x: 11, y: 7, prevX: 11, prevY: 7, direction: "up" },
        npcs: [],
        runner: null,
        dialogue: null,
        actionEvents: [
          {
            frame: 40,
            timestamp_ms: 1710000000000,
            timestamp_iso: "2024-03-09T16:00:00.000Z",
            action: "press:a:1",
            mode: "overworld",
            map: "ElmsLab",
            coords: { x: 11, y: 7 },
            summary: "press:a:1 no_change",
            result: { ok: false, changed: false, reason: "no_change" },
          },
        ],
      },
    });

    await sessionAny.restoreRuntimeSnapshot();

    expect(sessionAny.actionEvents).toEqual([
      expect.objectContaining({
        action: "press:a:1",
        map: "ElmsLab",
        coords: { x: 11, y: 7 },
        result: { ok: false, changed: false, reason: "no_change", events: undefined },
      }),
    ]);
  });

  it("discards stale runtime snapshots that do not match the live save map during restore", async () => {
    const sessionId = "runtime-restore-stale-snapshot";
    removeAutosave(sessionId);
    const runtimeSlot = __testing.resolveRuntimeSnapshotSlot(sessionId);
    const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    const session = getMcpSession(sessionId);
    const sessionAny = session as unknown as {
      game: {
        getMapName: () => string;
        getOverworld: () => {
          current_map_name: string;
        };
      };
      actionEvents: unknown[];
      restoreRuntimeSnapshot: () => Promise<void>;
    };

    fs.writeFileSync(
      runtimeSlot,
      JSON.stringify({
        version: 1,
        frameCounter: 7,
        map: "ElmsLab",
        player: { x: 11, y: 7, prevX: 11, prevY: 7, direction: "right" },
        npcs: [],
        runner: null,
        dialogue: null,
        actionEvents: [
          {
            frame: 6,
            timestamp_ms: 1710000000000,
            timestamp_iso: "2024-03-09T16:00:00.000Z",
            action: "press:a:1",
            mode: "overworld",
            map: "ElmsLab",
            coords: { x: 11, y: 7 },
            summary: "prompt opened:dialogue",
            result: { ok: true, changed: true },
          },
        ],
      }),
      "utf8"
    );

    sessionAny.game = {
      getMapName: () => "PlayersHouse2F",
      getOverworld: () => ({
        current_map_name: "PlayersHouse2F",
      }),
    };
    sessionAny.actionEvents = [];

    await sessionAny.restoreRuntimeSnapshot();

    expect(fs.existsSync(runtimeSlot)).toBe(false);
    expect(sessionAny.actionEvents).toEqual([]);
    expect(warn).toHaveBeenCalledWith(
      expect.stringContaining("Discarding stale runtime snapshot")
    );

    warn.mockRestore();
  });

  it("discards same-map runtime snapshots whose coords rewind the loaded save", async () => {
    const sessionId = "runtime-restore-stale-coords";
    removeAutosave(sessionId);
    const runtimeSlot = __testing.resolveRuntimeSnapshotSlot(sessionId);
    const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    const session = getMcpSession(sessionId);
    const sessionAny = session as unknown as {
      game: {
        getMapName: () => string;
        getGameState: () => { wram: { wXCoord: number; wYCoord: number } };
        getOverworld: () => {
          current_map_name: string;
          player_x: number;
          player_y: number;
          player_object: { x: number; y: number };
        };
      };
      actionEvents: unknown[];
      restoreRuntimeSnapshot: () => Promise<void>;
    };

    fs.writeFileSync(
      runtimeSlot,
      JSON.stringify({
        version: 1,
        frameCounter: 9,
        map: "PlayersHouse2F",
        player: { x: 7, y: 4, prevX: 7, prevY: 4, direction: "left" },
        npcs: [],
        runner: null,
        dialogue: null,
        actionEvents: [
          {
            frame: 8,
            timestamp_ms: 1710000000000,
            timestamp_iso: "2024-03-09T16:00:00.000Z",
            action: "move:left:1",
            mode: "overworld",
            map: "PlayersHouse2F",
            coords: { x: 7, y: 4 },
            summary: "moved:left",
            result: { ok: true, changed: true },
          },
        ],
      }),
      "utf8"
    );

    sessionAny.game = {
      getMapName: () => "PlayersHouse2F",
      getGameState: () => ({ wram: { wXCoord: 8, wYCoord: 4 } }),
      getOverworld: () => ({
        current_map_name: "PlayersHouse2F",
        player_x: 8,
        player_y: 4,
        player_object: { x: 8, y: 4 },
      }),
    };
    sessionAny.actionEvents = [];

    await sessionAny.restoreRuntimeSnapshot();

    expect(fs.existsSync(runtimeSlot)).toBe(false);
    expect(sessionAny.actionEvents).toEqual([]);
    expect(warn).toHaveBeenCalledWith(expect.stringContaining("runtime coords 7,4 do not match live coords 8,4"));

    warn.mockRestore();
  });

  it("treats read-only runtime snapshot persistence as non-fatal after autosave", async () => {
    const sessionId = "runtime-snapshot-readonly";
    removeAutosave(sessionId);
    const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    const saveSpy = jest.spyOn(saveModule, "saveGame").mockResolvedValue(true);
    const writeSpy = jest.spyOn(fsPromises, "writeFile").mockImplementation(async (targetPath, data, options) => {
      if (String(targetPath) === __testing.resolveRuntimeSnapshotSlot(sessionId)) {
        const error = new Error("read-only filesystem") as Error & { code?: string };
        error.code = "EROFS";
        throw error;
      }
      return jest.requireActual("node:fs/promises").writeFile(targetPath, data, options);
    });

    try {
      const session = getMcpSession(sessionId);
      const sessionAny = session as unknown as {
        game: {
          getGameState: () => { sram: Record<string, unknown>; wram: Record<string, unknown> };
          getOverworld: () => {
            current_map_name: string;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            player_direction: string;
            target_tile_x: number;
            target_tile_y: number;
            is_moving: boolean;
            step_progress_px: number;
            step_dx_px: number;
            step_dy_px: number;
            _queued_direction: string | null;
            _turning_direction: string | null;
            _turn_frames_remaining: number;
            _turn_should_force_step: boolean;
            _pending_auto_step: string | null;
            _sync_player_state: jest.Mock;
            _script_task_queue: unknown[];
            _active_script_task: unknown | null;
            _blocking_task_count: number;
            _blocking_movement_lock_active: boolean;
            _clear_stale_blocking_tasks: jest.Mock;
            reset_input_state: jest.Mock;
            _ignore_a_until_release: boolean;
            player_object: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition: jest.Mock;
            };
            npcs: [];
            script_runner: null;
            dialogue: null;
          };
          getMapName: () => string;
        };
        frameCounter: number;
        actionEvents: unknown[];
        finalizeActionResult: (options: {
          changed: boolean;
          reason?: "blocked" | "no_change" | "menu" | "busy" | "unknown";
          events?: string[];
        }) => Promise<{ ok: boolean; changed: boolean; reason?: string; events?: string[] }>;
      };

      sessionAny.game = {
        getGameState: () => ({ sram: {}, wram: {} }),
        getOverworld: () => ({
          current_map_name: "PlayersHouse2F",
          player_x: 7,
          player_y: 5,
          prev_player_x: 7,
          prev_player_y: 5,
          player_direction: "down",
          target_tile_x: 7,
          target_tile_y: 5,
          is_moving: false,
          step_progress_px: 0,
          step_dx_px: 0,
          step_dy_px: 0,
          _queued_direction: null,
          _turning_direction: null,
          _turn_frames_remaining: 0,
          _turn_should_force_step: false,
          _pending_auto_step: null,
          _sync_player_state: jest.fn(),
          _script_task_queue: [],
          _active_script_task: null,
          _blocking_task_count: 0,
          _blocking_movement_lock_active: false,
          _clear_stale_blocking_tasks: jest.fn(),
          reset_input_state: jest.fn(),
          _ignore_a_until_release: false,
          player_object: {
            x: 7,
            y: 5,
            prevX: 7,
            prevY: 5,
            direction: "down",
            updatePixelPosition: jest.fn(),
          },
          npcs: [],
          script_runner: null,
          dialogue: null,
        }),
        getMapName: () => "PlayersHouse2F",
      };
      sessionAny.frameCounter = 99;
      sessionAny.actionEvents = [];

      await expect(
        sessionAny.finalizeActionResult({ changed: true, events: ["pressed:a:1"] })
      ).resolves.toEqual(
        expect.objectContaining({
          ok: true,
          changed: true,
          events: ["pressed:a:1"],
        })
      );

      expect(saveSpy).toHaveBeenCalledWith(
        expect.any(Object),
        `mcp-${sessionId}-autosave.sav`
      );
      expect(warn).toHaveBeenCalledWith(
        expect.stringContaining("Runtime snapshot persistence unavailable")
      );
    } finally {
      writeSpy.mockRestore();
      saveSpy.mockRestore();
      warn.mockRestore();
    }
  });

  it("treats MCP autosave persistence failures as non-fatal after input", async () => {
    const sessionId = "autosave-save-failure";
    removeAutosave(sessionId);
    const warn = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    const saveSpy = jest
      .spyOn(saveModule, "saveGame")
      .mockRejectedValue(new saveModule.SaveGameError("Failed to save game to mcp-autosave-save-failure-autosave.sav"));
    const writeRuntimeSnapshot = jest.fn().mockResolvedValue(undefined);

    try {
      const session = getMcpSession(sessionId);
      const sessionAny = session as unknown as {
        game: {
          getGameState: () => { sram: Record<string, unknown>; wram: Record<string, unknown> };
          getOverworld: () => {
            current_map_name: string;
            player_x: number;
            player_y: number;
            player_direction: string;
            is_moving: boolean;
            script_tasks_active: () => boolean;
            player_movement_locked: () => boolean;
            _current_tile_permission: () => number;
          };
          getMapName: () => string;
          isBattleActive: () => boolean;
        };
        frameCounter: number;
        actionEvents: unknown[];
        writeRuntimeSnapshot: jest.Mock;
        finalizeActionResult: (options: {
          changed: boolean;
          reason?: "blocked" | "no_change" | "menu" | "busy" | "unknown";
          events?: string[];
        }) => Promise<{ ok: boolean; changed: boolean; reason?: string; events?: string[] }>;
      };

      sessionAny.game = {
        getGameState: () => ({ sram: {}, wram: {} }),
        getOverworld: () => ({
          current_map_name: "PlayersHouse2F",
          player_x: 7,
          player_y: 5,
          player_direction: "down",
          is_moving: false,
          script_tasks_active: () => false,
          player_movement_locked: () => false,
          _current_tile_permission: () => 0,
        }),
        getMapName: () => "PlayersHouse2F",
        isBattleActive: () => false,
      };
      sessionAny.frameCounter = 100;
      sessionAny.actionEvents = [];
      sessionAny.writeRuntimeSnapshot = writeRuntimeSnapshot;

      await expect(
        sessionAny.finalizeActionResult({ changed: true, events: ["moved:1"] })
      ).resolves.toEqual(
        expect.objectContaining({
          ok: true,
          changed: true,
          events: ["moved:1"],
        })
      );

      expect(saveSpy).toHaveBeenCalledWith(
        expect.any(Object),
        `mcp-${sessionId}-autosave.sav`
      );
      expect(writeRuntimeSnapshot).not.toHaveBeenCalled();
      expect(warn).toHaveBeenCalledWith(expect.stringContaining("Autosave failed"));
    } finally {
      saveSpy.mockRestore();
      warn.mockRestore();
    }
  });

  it("resyncs WRAM player coords from the live overworld before snapshotting or autosaving", () => {
    const session = getMcpSession("runtime-sync-before-save");
    const sessionAny = session as unknown as {
      game: {
        getOverworld: () => { player_x: number; player_y: number; _sync_player_state: jest.Mock };
        getGameState: () => { sram: Record<string, unknown>; wram: Record<string, unknown> };
        draw?: jest.Mock;
      };
      textUi: { getSnapshot: jest.Mock };
      captureSnapshot: () => void;
      readAutosaveState: () => unknown;
    };
    const syncPlayerState = jest.fn();
    const draw = jest.fn();
    sessionAny.game = {
      getOverworld: () => ({ player_x: 9, player_y: 7, _sync_player_state: syncPlayerState }),
      getGameState: () => ({ sram: {}, wram: {} }),
      draw,
    };
    sessionAny.textUi = { getSnapshot: jest.fn(() => null) };

    sessionAny.captureSnapshot();
    sessionAny.readAutosaveState();

    expect(syncPlayerState).toHaveBeenCalledTimes(2);
    expect(draw).toHaveBeenCalledTimes(1);
  });

  it("redraws before snapshot capture so rendered text cannot lag the live mode", () => {
    const session = getMcpSession("snapshot-redraw-sync");
    const draw = jest.fn();
    const getSnapshot = jest.fn(() => ({
      viewportLines: ["OVERWORLD"],
      infoLines: ["Pos: (83,29)"],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
      viewportTitle: "Overworld",
      infoTitle: "Info",
      marker: null,
      actionLog: [],
    }));
    const sessionAny = session as unknown as {
      game: {
        draw: jest.Mock;
        getOverworld: () => null;
      };
      textUi: { getSnapshot: jest.Mock };
      captureSnapshot: () => void;
      observeText: () => string;
    };

    sessionAny.game = {
      draw,
      getOverworld: () => null,
    };
    sessionAny.textUi = { getSnapshot };

    sessionAny.captureSnapshot();

    expect(draw).toHaveBeenCalledTimes(1);
    expect(getSnapshot).toHaveBeenCalledTimes(1);
    expect(draw.mock.invocationCallOrder[0]).toBeLessThan(getSnapshot.mock.invocationCallOrder[0]);
    expect(sessionAny.observeText()).toContain("OVERWORLD");
  });

  it("keeps observe/status in sync across menu open/close and allows movement after close", async () => {
    const session = getMcpSession("menu-open-close-sync");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      observeText: () => string;
      actionLimiter: { consume: jest.Mock };
      frameLimiter: { consume: jest.Mock };
      holdFrames: number;
      maxFramesPerCall: number;
      lastSnapshot: {
        viewport: string[];
        info: string[];
        menu: string[] | null;
        prompt: string[] | null;
        dialogue: string[] | null;
        titles: { viewport: string; info: string };
        marker: [number, number, string] | null;
        action_log: string[];
        script: Record<string, unknown>;
        tasks: Record<string, unknown>[];
      } | null;
      scheduleKeyPress: jest.Mock;
      stepFrames: jest.Mock;
      game: {
        getGameState: () => {
          wram: {
            player_x: number;
            player_y: number;
            wXCoord: number;
            wYCoord: number;
            wMapGroup: number;
            wMapNumber: number;
          };
        };
        getMapName: () => string;
        isMenuOpen: () => boolean;
        isBattleActive: () => boolean;
        getOverworld: () => {
          is_moving: boolean;
          player_direction: string;
          script_runner: { is_busy?: boolean } | null;
          player_movement_locked: () => boolean;
          script_tasks_active: () => boolean;
          _last_block_feedback?: { reason?: string } | null;
        };
      };
      getGame: jest.Mock;
    };
    const buildSnapshot = (menuOpen: boolean) => ({
      viewport: ["OVERWORLD"],
      info: ["D-PAD=Move A=Talk START=Menu"],
      menu: menuOpen ? ["MENU", "▶ POKEDEX", "  POKEMON", "  PACK"] : null,
      prompt: null,
      dialogue: null,
      titles: { viewport: "Viewport", info: "Info" },
      marker: null,
      action_log: [],
      script: {},
      tasks: [],
    });

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.actionLimiter = { consume: jest.fn() };
    sessionAny.frameLimiter = { consume: jest.fn() };
    sessionAny.holdFrames = 1;
    sessionAny.maxFramesPerCall = 10;

    let menuOpen = false;
    let pendingOpenFrames = 0;
    let pendingCloseFrames = 0;
    const state = { wram: { player_x: 5, player_y: 3, wXCoord: 5, wYCoord: 3, wMapGroup: 2, wMapNumber: 1 } };
    const overworld = {
      is_moving: false,
      player_direction: "down",
      script_runner: null,
      player_movement_locked: () => false,
      script_tasks_active: () => false,
      _last_block_feedback: null as { reason?: string } | null,
    };
    const pendingDirections: string[] = [];
    const pendingButtons: string[] = [];
    const game = {
      getGameState: () => state,
      getMapName: () => "PlayersHouse1F",
      isMenuOpen: () => menuOpen,
      isBattleActive: () => false,
      getOverworld: () => overworld,
    };
    sessionAny.game = game;
    sessionAny.getGame = jest.fn(() => game);
    sessionAny.lastSnapshot = buildSnapshot(false);
    sessionAny.scheduleKeyPress = jest.fn(({ direction, button }: { direction?: string; button?: string }) => {
      if (direction) {
        pendingDirections.push(direction);
        overworld.is_moving = true;
        overworld.player_direction = direction;
      }
      if (button) {
        pendingButtons.push(button);
      }
    });
    sessionAny.stepFrames = jest.fn((count: number) => {
      for (let i = 0; i < count; i += 1) {
        const button = pendingButtons.shift();
        if (button === "start" && !menuOpen) {
          // Simulate a delayed open transition that settles after two frames.
          pendingOpenFrames = 2;
        } else if ((button === "b" || button === "start") && menuOpen) {
          // Simulate delayed close transition that needs an extra settle frame.
          pendingCloseFrames = 3;
        }

        if (pendingOpenFrames > 0) {
          pendingOpenFrames -= 1;
          if (pendingOpenFrames === 0) {
            menuOpen = true;
            sessionAny.lastSnapshot = buildSnapshot(true);
          }
        }
        if (pendingCloseFrames > 0) {
          pendingCloseFrames -= 1;
          if (pendingCloseFrames === 0) {
            menuOpen = false;
            sessionAny.lastSnapshot = buildSnapshot(false);
          } else {
            sessionAny.lastSnapshot = buildSnapshot(true);
          }
        }

        const direction = pendingDirections.shift();
        if (direction && !menuOpen) {
          if (direction === "right") {
            state.wram.player_x += 1;
            state.wram.wXCoord += 1;
          }
          overworld.is_moving = false;
        } else if (!direction) {
          overworld.is_moving = false;
        }
      }
    });

    await session.press("start", 1);
    const openStatus = await session.status();
    const openObserve = session.observeText();
    expect(openStatus.mode).toBe("menu");
    expect(openStatus.menu).toBe(true);
    expect(openObserve).toContain("MENU");

    const closeAction = await session.press("b", 1);
    const closeStatus = await session.status();
    const closeObserve = session.observeText();
    expect(closeAction.result.reason).not.toBe("menu");
    expect(closeStatus.mode).toBe("overworld");
    expect(closeStatus.menu).toBe(false);
    expect(closeObserve).not.toContain("MENU");

    const moveAction = await session.move("right", 1);
    const movedStatus = await session.status();
    expect(moveAction.result.changed).toBe(true);
    expect(moveAction.result.ok).toBe(true);
    expect(moveAction.result.reason).toBeUndefined();
    expect(movedStatus.coords?.x).toBe(6);
  });

  it("includes live battle state details in status while a battle is active", async () => {
    const session = getMcpSession("status-includes-battle-state");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
      lastSnapshot: unknown;
      actionEvents: unknown[];
      getModalUiState: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = null;
    sessionAny.actionEvents = [];
    sessionAny.isMenuOpenForSession = jest.fn(() => true);
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "Route29");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "Route29", group: 24, number: 3 }));
    sessionAny.getModalUiState = jest.fn(() => ({
      in_battle: true,
      in_menu: true,
      in_dialog: false,
      text_box_open: false,
      prompt_pending: false,
      movement_locked: false,
      script_busy: false,
      input_blocked_reason: "battle",
      can_move: false,
    }));
    sessionAny.getGame = jest.fn(() => ({
      isBattleActive: () => true,
      getMapName: () => "Route29",
      getBattle: () => ({
        context: { currentState: 5, playerAction: { actionType: "MOVE" }, enemyAction: null },
        _turnCursor: 1,
      }),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
      getGameState: () => ({
        wram: {
          player_x: 85,
          player_y: 29,
          current_map_group: 24,
          current_map_id: 3,
        },
        sram: {
          party: {
            pokemon: [{ species: "CYNDAQUIL", level: 6, hp: 22, max_hp: 22 }],
          },
          badges: { johto: [], kanto: [] },
        },
      }),
    }));

    await expect(session.status()).resolves.toEqual(
      expect.objectContaining({
        mode: "battle",
        menu: true,
        battle_state: "5",
        battle_turn_cursor: 1,
        battle_has_player_action: true,
        battle_has_enemy_action: false,
      })
    );
  });

  it("includes the active rendered surface in raw MCP status", async () => {
    const session = getMcpSession("status-includes-rendered-surface");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      lastSnapshot: unknown;
      actionEvents: unknown[];
      getModalUiState: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = {
      viewport: ["OAK INTRO", "SPRITE: OAK"],
      info: ["STATE: oak_intro", "MODE: intro", "PHASE: text", "WAITING: yes", "A/START=Advance"],
      menu: null,
      prompt: null,
      dialogue: ["Hello!"],
      titles: { viewport: "Oak Intro", info: "Oak Intro" },
      marker: null,
      action_log: [],
      script: {},
      tasks: [],
      map: undefined,
      flow_state: undefined,
    };
    sessionAny.actionEvents = [];
    sessionAny.getModalUiState = jest.fn(() => ({
      in_battle: false,
      in_menu: true,
      in_dialog: true,
      text_box_open: false,
      prompt_pending: true,
      movement_locked: true,
      script_busy: false,
      input_blocked_reason: "oak_intro",
      can_move: false,
    }));
    sessionAny.getGame = jest.fn(() => ({
      getDebugStatus: () => ({ mode: "oak_intro" }),
      isBattleActive: () => false,
      isMenuOpen: () => true,
      getMapName: () => "PlayersHouse2F",
      getBattle: () => null,
      getAudioPlaybackSnapshot: () => ({
        musicToken: "MUSIC_ROUTE_29",
        musicRole: "map",
        musicSource: "/api/audio/route29.mp3",
        musicFrame: 0,
        fadedVolume: 1,
        activeChannels: [],
        recentEvents: [
          {
            sequence: 1,
            kind: "sfx",
            token: "SFX_READ_TEXT_2",
            source: "/api/audio/sfx/readtext2.mp3",
          },
        ],
      }),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
      getGameState: () => ({
        wram: {
          wMapGroup: 24,
          wMapNumber: 7,
          current_map_group: 24,
          current_map_id: 7,
          event_flags: {},
        },
        sram: {
          party: { pokemon: [] },
          badges: { johto: [], kanto: [] },
        },
      }),
    }));

    await expect(session.status()).resolves.toEqual(
      expect.objectContaining({
        mode: "oak_intro",
        surface: expect.objectContaining({
          kind: "oak_intro",
          title: "Oak Intro",
          state: "oak_intro",
          phase: "text",
          waiting: true,
          dialogue_open: true,
          primary_text: "Hello!",
        }),
        audio: expect.objectContaining({
          musicToken: "MUSIC_ROUTE_29",
          musicSource: "/api/audio/route29.mp3",
          recentEvents: [
            expect.objectContaining({
              token: "SFX_READ_TEXT_2",
              source: "/api/audio/sfx/readtext2.mp3",
            }),
          ],
        }),
      })
    );
  });

  it("does not let Bill's PC instruction text inherit stale dialogue input ownership", () => {
    const session = getMcpSession("status-pc-instruction-not-dialogue-blocker");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: jest.Mock;
      isMovementLocked: jest.Mock;
      getScriptBusyReason: jest.Mock;
      getStopReason: jest.Mock;
      getModalUiState: (game: unknown) => {
        in_dialog: boolean;
        text_box_open: boolean;
        text_advance_pending: boolean;
        prompt_pending: boolean;
      };
    };

    sessionAny.lastSnapshot = {
      viewport: ["WITHDRAW #MON", "BOX 01", "▶ GEODUD", "CANCEL"],
      info: ["SELECTED: GEODUDE", "D-Pad=Move A=Select B=Back"],
      menu: ["▶ WITHDRAW", "  STATS", "  RELEASE", "  CANCEL"],
      prompt: ["What's up?"],
      dialogue: null,
      titles: { viewport: "Bill's PC", info: "Legend" },
      marker: null,
      action_log: [],
      script: {},
      tasks: [],
    };
    sessionAny.isMenuOpenForSession = jest.fn(() => true);
    sessionAny.isMovementLocked = jest.fn(() => true);
    sessionAny.getScriptBusyReason = jest.fn(() => null);
    sessionAny.getStopReason = jest.fn(() => "pc");

    const modal = sessionAny.getModalUiState({
      isBattleActive: () => false,
      getOverworld: () => ({
        dialogue: {
          visible: true,
          waiting_for_input: true,
          pending_waits: 1,
          _yes_no_prompt: { selection: 0 },
        },
      }),
    });

    expect(modal.prompt_pending).toBe(false);
    expect(modal.text_advance_pending).toBe(false);
    expect(modal.in_dialog).toBe(false);
    expect(modal.text_box_open).toBe(false);
  });

  it("does not let stale dialogue input ownership block the PC hub menu", () => {
    const session = getMcpSession("status-pc-hub-not-dialogue-blocker");
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: jest.Mock;
      isMovementLocked: jest.Mock;
      getScriptBusyReason: jest.Mock;
      getStopReason: jest.Mock;
      getModalUiState: (game: unknown) => {
        in_dialog: boolean;
        text_box_open: boolean;
        text_advance_pending: boolean;
        prompt_pending: boolean;
      };
    };

    sessionAny.lastSnapshot = {
      viewport: ["PC"],
      info: ["Use the PC."],
      menu: ["▶ BILL's PC", "  CHRIS's PC", "  TURN OFF"],
      prompt: null,
      dialogue: null,
      titles: { viewport: "PC", info: "Legend" },
      marker: null,
      action_log: [],
      script: {},
      tasks: [],
    };
    sessionAny.isMenuOpenForSession = jest.fn(() => true);
    sessionAny.isMovementLocked = jest.fn(() => true);
    sessionAny.getScriptBusyReason = jest.fn(() => null);
    sessionAny.getStopReason = jest.fn(() => "menu");

    const modal = sessionAny.getModalUiState({
      isBattleActive: () => false,
      getOverworld: () => ({
        dialogue: {
          visible: true,
          waiting_for_input: true,
          pending_waits: 1,
        },
      }),
    });

    expect(modal.prompt_pending).toBe(false);
    expect(modal.text_advance_pending).toBe(false);
    expect(modal.in_dialog).toBe(false);
    expect(modal.text_box_open).toBe(false);
  });

  it.each([
    ["top menu", { menu: ["▶ WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>", "  CHANGE BOX", "  SEE YA!"], prompt: null }],
    [
      "renderer prompt top menu",
      {
        viewport: ["Prompt"],
        titles: { viewport: "Prompt", info: "Legend" },
        menu: ["▶ WITHDRAW <PK><MN>", "  DEPOSIT <PK><MN>", "  CHANGE BOX", "  SEE YA!"],
        prompt: null,
      },
    ],
    ["action submenu", { menu: ["▶ DEPOSIT", "  STATS", "  RELEASE", "  CANCEL"], prompt: ["What's up?"] }],
    ["withdraw list", { menu: null, prompt: ["Choose a <PK><MN>."] }],
    ["deposit list", { menu: null, prompt: ["Select a POKéMON."] }],
    ["move list", { menu: null, prompt: ["Move to where?"] }],
  ])("does not let stale dialogue input ownership block the Bill's PC %s", (_label, snapshotParts) => {
    const session = getMcpSession(`status-pc-${String(_label).replace(/\s+/g, "-")}-not-dialogue-blocker`);
    const sessionAny = session as unknown as {
      lastSnapshot: unknown;
      isMenuOpenForSession: jest.Mock;
      isMovementLocked: jest.Mock;
      getScriptBusyReason: jest.Mock;
      getStopReason: jest.Mock;
      getModalUiState: (game: unknown) => {
        in_dialog: boolean;
        text_box_open: boolean;
        text_advance_pending: boolean;
        prompt_pending: boolean;
      };
    };

    sessionAny.lastSnapshot = {
      viewport: ["BILL'S PC", "BOX 01", "▶ GEODUD", "CANCEL"],
      info: ["SELECTED: GEODUDE", "D-Pad=Move A=Select B=Back"],
      dialogue: null,
      titles: { viewport: "Bill's PC", info: "Legend" },
      marker: null,
      action_log: [],
      script: {},
      tasks: [],
      ...snapshotParts,
    };
    sessionAny.isMenuOpenForSession = jest.fn(() => true);
    sessionAny.isMovementLocked = jest.fn(() => true);
    sessionAny.getScriptBusyReason = jest.fn(() => null);
    sessionAny.getStopReason = jest.fn(() => "menu");

    const modal = sessionAny.getModalUiState({
      isBattleActive: () => false,
      getOverworld: () => ({
        dialogue: {
          visible: true,
          waiting_for_input: true,
          pending_waits: 1,
          _yes_no_prompt: { selection: 0 },
        },
      }),
    });

    expect(modal.prompt_pending).toBe(false);
    expect(modal.text_advance_pending).toBe(false);
    expect(modal.in_dialog).toBe(false);
    expect(modal.text_box_open).toBe(false);
  });

  it("settles passive blank battle handoffs before reporting status", async () => {
    const session = getMcpSession("status-settles-passive-battle-handoff");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: unknown[];
      getModalUiState: jest.Mock;
      stepFrames: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [];
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.readFacingDirection = jest.fn(() => "left");
    sessionAny.readBestMapName = jest.fn(() => "Route30");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "Route30", group: 26, number: 1 }));

    let inBattle = true;
    let modalStateIndex = 0;
    const blankBattleModal = {
      in_battle: true,
      in_menu: false,
      in_dialog: false,
      text_box_open: false,
      prompt_pending: false,
      movement_locked: false,
      script_busy: false,
      input_blocked_reason: "battle",
      can_move: false,
    };
    const overworldModal = {
      in_battle: false,
      in_menu: false,
      in_dialog: false,
      text_box_open: false,
      prompt_pending: false,
      movement_locked: false,
      script_busy: false,
      input_blocked_reason: null,
      can_move: true,
    };
    sessionAny.getModalUiState = jest.fn(() => {
      modalStateIndex += 1;
      return inBattle ? blankBattleModal : overworldModal;
    });
    sessionAny.stepFrames = jest.fn((count: number) => {
      if (count >= 4) {
        inBattle = false;
      }
    });
    sessionAny.getGame = jest.fn(() => ({
      isBattleActive: () => inBattle,
      isMenuOpen: () => false,
      getMapName: () => "Route30",
      getBattle: () => ({
        context: { currentState: 6, playerAction: null, enemyAction: null },
        _turnCursor: 0,
      }),
      getOverworld: () => ({
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
      getGameState: () => ({
        wram: {
          player_x: 35,
          player_y: 31,
          current_map_group: 26,
          current_map_id: 1,
        },
        sram: {
          party: {
            pokemon: [{ species: "QUILAVA", level: 14, hp: 44, max_hp: 44 }],
          },
          badges: { johto: [], kanto: [] },
        },
      }),
    }));

    await expect(session.status()).resolves.toEqual(
      expect.objectContaining({
        mode: "overworld",
        in_battle: false,
      })
    );
    expect(sessionAny.stepFrames).toHaveBeenCalledWith(4);
  });

  it("surfaces a whiteout notice when the current map differs from the recent battle map", async () => {
    const session = getMcpSession("status-whiteout-notice");
    const sessionAny = session as unknown as {
      ensureReady: jest.Mock;
      getGame: jest.Mock;
      isMenuOpenForSession: jest.Mock;
      readFacingDirection: jest.Mock;
      readBestMapName: jest.Mock;
      readMapIdentity: jest.Mock;
      lastSnapshot: { menu?: string[] | null; prompt?: string[] | null; dialogue?: string[] | null } | null;
      actionEvents: Array<{
        action: string;
        frame: number;
        mode?: "overworld" | "menu" | "battle";
        map?: string;
        moments?: string[];
        result: { ok: boolean; changed: boolean; reason?: string; events?: string[] };
      }>;
      getModalUiState: jest.Mock;
    };

    sessionAny.ensureReady = jest.fn().mockResolvedValue(undefined);
    sessionAny.lastSnapshot = { menu: null, prompt: null, dialogue: null };
    sessionAny.actionEvents = [
      {
        action: "press:a:1",
        frame: 4850,
        mode: "overworld",
        map: "Route29",
        moments: ["mode:battle->overworld", "battle_ended", "prompt_closed:dialogue"],
        result: { ok: true, changed: true, events: ["pressed:a:1"] },
      },
    ];
    sessionAny.isMenuOpenForSession = jest.fn(() => false);
    sessionAny.readFacingDirection = jest.fn(() => "down");
    sessionAny.readBestMapName = jest.fn(() => "PlayersHouse2F");
    sessionAny.readMapIdentity = jest.fn(() => ({ name: "PlayersHouse2F", group: 1, number: 1 }));
    sessionAny.getModalUiState = jest.fn(() => ({
      in_battle: false,
      in_menu: false,
      in_dialog: false,
      text_box_open: false,
      prompt_pending: false,
      movement_locked: false,
      script_busy: false,
      input_blocked_reason: null,
      can_move: true,
    }));
    sessionAny.getGame = jest.fn(() => ({
      isBattleActive: () => false,
      isMenuOpen: () => false,
      getMapName: () => "PlayersHouse2F",
      getBattle: () => null,
      getOverworld: () => ({
        current_map_name: "PlayersHouse2F",
        script_runner: null,
        player_movement_locked: () => false,
        script_tasks_active: () => false,
      }),
      getGameState: () => ({
        wram: {
          player_x: 7,
          player_y: 11,
          current_map_group: 1,
          current_map_id: 1,
        },
        sram: {
          player_name: "CHRIS",
          party: {
            pokemon: [{ species: "CYNDAQUIL", level: 5, hp: 20, max_hp: 20 }],
          },
          badges: { johto: [], kanto: [] },
        },
      }),
    }));

    await expect(session.status()).resolves.toEqual(
      expect.objectContaining({
        map: "PlayersHouse2F",
        notices: ["CHRIS is out of useable POKeMON! CHRIS whited out!"],
      })
    );
  });
});

describe("McpGameSession tilemap rendering", () => {
  const positionOverworldPlayer = (
    overworld: {
      load_map: (mapName: string) => void;
      player_x: number;
      player_y: number;
      prev_player_x: number;
      prev_player_y: number;
      target_tile_x: number;
      target_tile_y: number;
      player_direction: string;
      player_object?: {
        x: number;
        y: number;
        prevX: number;
        prevY: number;
        direction: string;
        updatePixelPosition?: () => void;
      } | null;
      _sync_player_state?: () => void;
    },
    mapName: string,
    position: { x: number; y: number; direction: string }
  ): void => {
    overworld.load_map(mapName);
    overworld.player_x = position.x;
    overworld.player_y = position.y;
    overworld.prev_player_x = position.x;
    overworld.prev_player_y = position.y;
    overworld.target_tile_x = position.x;
    overworld.target_tile_y = position.y;
    overworld.player_direction = position.direction;
    if (overworld.player_object) {
      overworld.player_object.x = position.x;
      overworld.player_object.y = position.y;
      overworld.player_object.prevX = position.x;
      overworld.player_object.prevY = position.y;
      overworld.player_object.direction = position.direction;
      overworld.player_object.updatePixelPosition?.();
    }
    overworld._sync_player_state?.();
  };

  const positionElmsLabPlayer = (
    overworld: {
      load_map: (mapName: string) => void;
      player_x: number;
      player_y: number;
      prev_player_x: number;
      prev_player_y: number;
      target_tile_x: number;
      target_tile_y: number;
      player_direction: string;
      player_object?: {
        x: number;
        y: number;
        prevX: number;
        prevY: number;
        direction: string;
        updatePixelPosition?: () => void;
      } | null;
      _sync_player_state?: () => void;
    },
    position: { x: number; y: number; direction: string }
  ): void => {
    positionOverworldPlayer(overworld, "ElmsLab", position);
  };

  beforeEach(() => {
    __testing.clearSessions();
  });

  afterEach(() => {
    __testing.clearSessions();
  });

  it(
    "keeps New Bark Town observeText centered on the player near the bottom of the map",
    async () => {
      const sessionId = "new-bark-bottom-visible";
      removeAutosave(sessionId);
      const session = getMcpSession(sessionId);
      await session.ensureReady();
      const sessionAny = session as unknown as {
        game: {
          getGameState: () => {
            wram: {
              map_scenes: Record<string, string>;
              map_scene_indices: Record<string, number>;
            };
          };
          getOverworld: () => {
            load_map: (mapName: string) => void;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            npcs?: Array<{
              constantId?: string | null;
              spriteId?: string;
              x: number;
              y: number;
            }>;
            _sync_player_state?: () => void;
          };
        };
        captureSnapshot: () => void;
        observeText: () => string;
        stepFrames: (count: number) => void;
      };

      positionOverworldPlayer(sessionAny.game.getOverworld(), "NewBarkTown", {
        x: 15,
        y: 31,
        direction: "down",
      });
      sessionAny.captureSnapshot();

      const rendered = sessionAny.observeText();

      expect(rendered).toContain("Pos: (7,15)");
      expect(rendered).toContain("15");
      expect(rendered).toContain("@v");
    },
    15000
  );

  it(
    "renders Elm's Lab interactables in the same observe text path the agent consumes",
    async () => {
      const session = getMcpSession("elms-lab-agent-observe");
      await session.ensureReady();
      const sessionAny = session as unknown as {
        game: {
          getOverworld: () => {
            load_map: (mapName: string) => void;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            _sync_player_state?: () => void;
          };
        };
        captureSnapshot: () => void;
        observeText: () => string;
      };

      const overworld = sessionAny.game.getOverworld();
      positionElmsLabPlayer(overworld, { x: 11, y: 11, direction: "right" });

      sessionAny.captureSnapshot();

      const text = sessionAny.observeText();
      const liveObjects = overworld.npcs ?? [];

      expect(text).toContain("OVERWORLD");
      expect(text).toContain("Nv");
      expect(text).toContain("I  I  I");
      expect(liveObjects).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            constantId: "ELMSLAB_ELM",
            spriteId: "ELM",
            x: 7,
            y: 9,
          }),
          expect.objectContaining({
            constantId: "ELMSLAB_POKE_BALL1",
            spriteId: "POKE_BALL",
            x: 13,
            y: 7,
          }),
          expect.objectContaining({
            constantId: "ELMSLAB_POKE_BALL2",
            spriteId: "POKE_BALL",
            x: 15,
            y: 7,
          }),
          expect.objectContaining({
            constantId: "ELMSLAB_POKE_BALL3",
            spriteId: "POKE_BALL",
            x: 17,
            y: 7,
          }),
        ])
      );
    },
    15000
  );

  it(
    "opens and renders Goldenrod Game Corner slots from the live TUI approach tile",
    async () => {
      const sessionId = "goldenrod-game-corner-slots-render-proof";
      removeAutosave(sessionId);
      const session = getMcpSession(sessionId);
      await session.ensureReady();
      const sessionAny = session as unknown as {
        game: {
          getGameState: () => {
            sram: {
              key_items: Record<string, number>;
              coins: number;
            };
          };
          getOverworld: () => {
            load_map: (mapName: string) => void;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            input_capture_active?: boolean;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            _sync_player_state?: () => void;
          };
        };
        renderUi: {
          screen: {
            get_at: (pos: [number, number]) => [number, number, number, number];
          };
        };
        captureSnapshot: () => void;
        observeText: () => string;
      };

      const gameState = sessionAny.game.getGameState();
      gameState.sram.key_items.COIN_CASE = 1;
      gameState.sram.coins = 100;
      const overworld = sessionAny.game.getOverworld();
      positionOverworldPlayer(overworld, "GoldenrodGameCorner", {
        x: 11,
        y: 13,
        direction: "right",
      });
      sessionAny.captureSnapshot();

      expect(sessionAny.observeText()).toContain("OVERWORLD");
      const overworldPixel = sessionAny.renderUi.screen.get_at([0, 0]);

      const openAction = await session.press("a", 1);
      sessionAny.captureSnapshot();

      expect(openAction.result.ok).toBe(true);
      expect(openAction.result.changed).toBe(true);
      expect(overworld.input_capture_active).toBe(true);
      expect(sessionAny.observeText()).toContain("SLOT MACHINE");
      expect(sessionAny.observeText()).toContain("STATE: slot_machine");
      expect(sessionAny.renderUi.screen.get_at([0, 0])).not.toEqual(overworldPixel);
      let coloredPixelCount = 0;
      for (let y = 0; y < 96; y += 1) {
        for (let x = 0; x < 160; x += 1) {
          const [r, g, b, a] = sessionAny.renderUi.screen.get_at([x, y]);
          if (a === 255 && (r !== g || g !== b)) {
            coloredPixelCount += 1;
          }
        }
      }
      expect(coloredPixelCount).toBeGreaterThan(0);

      const spinAction = await session.press("a", 1);
      sessionAny.captureSnapshot();

      expect(spinAction.result.ok).toBe(true);
      expect(spinAction.result.changed).toBe(true);
      expect(sessionAny.observeText()).toContain("SLOT MACHINE");
      await new Promise((resolve) => setTimeout(resolve, 1000));
      sessionAny.captureSnapshot();
      expect(gameState.sram.coins).not.toBe(100);

      const closeAction = await session.press("b", 1);
      sessionAny.captureSnapshot();

      expect(closeAction.result.ok).toBe(true);
      expect(closeAction.result.changed).toBe(true);
      expect(overworld.input_capture_active).toBe(false);
      expect(sessionAny.observeText()).toContain("OVERWORLD");
    },
    15000
  );

  it(
    "uses the exact shared TextUI snapshot for both MCP observe text and tile snapshot text",
    async () => {
      const session = getMcpSession("shared-text-snapshot-proof");
      await session.ensureReady();
      const sessionAny = session as unknown as {
        textUi: {
          getSnapshot: () => {
            viewportLines: string[];
            infoLines: string[];
            menuLines?: string[] | null;
            promptLines?: string[] | null;
            dialogueLines?: string[] | null;
            viewportTitle: string;
            infoTitle: string;
            actionLog: string[];
            marker: [number, number, string] | null;
          } | null;
        };
        game: {
          getOverworld: () => {
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            _sync_player_state?: () => void;
          };
        };
        captureSnapshot: () => void;
        observeText: () => string;
        observePayload: () => {
          menu?: string[] | null;
          prompt?: string[] | null;
          dialogue?: string[] | null;
        } | null;
      };

      const overworld = sessionAny.game.getOverworld();
      positionElmsLabPlayer(overworld, { x: 13, y: 9, direction: "up" });
      sessionAny.captureSnapshot();

      const sharedSnapshot = sessionAny.textUi.getSnapshot();
      const tileSnapshotText = buildTextSnapshotLines(sharedSnapshot).join("\n");
      const observeText = sessionAny.observeText();
      const payload = sessionAny.observePayload();

      expect(sharedSnapshot).not.toBeNull();
      expect(payload?.menu).toEqual(sharedSnapshot?.menuLines ?? null);
      expect(payload?.prompt).toEqual(sharedSnapshot?.promptLines ?? null);
      expect(payload?.dialogue).toEqual(sharedSnapshot?.dialogueLines ?? null);

      expect(observeText).toContain("OVERWORLD");
      expect(tileSnapshotText).toContain("OVERWORLD");
      for (const line of sharedSnapshot?.dialogueLines ?? []) {
        expect(observeText).toContain(line);
        expect(tileSnapshotText).toContain(line);
      }
      for (const line of sharedSnapshot?.promptLines ?? []) {
        expect(observeText).toContain(line);
        expect(tileSnapshotText).toContain(line);
      }
    },
    15000
  );

  it(
    "exports an Elm's Lab render proof file at the repo root",
    async () => {
      const session = getMcpSession("elms-lab-proof-export");
      await session.ensureReady();
      const sessionAny = session as unknown as {
        game: {
          getOverworld: () => {
            load_map: (mapName: string) => void;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            _sync_player_state?: () => void;
          };
        };
        captureSnapshot: () => void;
        observeText: () => string;
        observePayload: () => {
          map?: {
            hotspots?: Array<{
              type: string;
              coords: { x: number; y: number };
              label: string;
              token?: string;
            }>;
          };
        } | null;
      };

      const overworld = sessionAny.game.getOverworld();
      positionElmsLabPlayer(overworld, { x: 13, y: 9, direction: "up" });

      sessionAny.captureSnapshot();

      const rendered = sessionAny.observeText();
      const payload = sessionAny.observePayload();
      const outputPath = path.resolve(process.cwd(), "..", "ELMS_LAB_RENDER_PROOF.txt");
      const lines = [
        "ELMS LAB RENDER PROOF",
        `generated_at: ${new Date().toISOString()}`,
        "source: apps/web MCP session observeText()",
        "",
        rendered,
      ];
      if (payload?.map?.hotspots?.length) {
        lines.push("", "STRUCTURED MAP HOTSPOTS");
        for (const hotspot of payload.map.hotspots) {
          lines.push(
            `- ${hotspot.type} @ (${hotspot.coords.x},${hotspot.coords.y}) ${hotspot.label}${hotspot.token ? ` [${hotspot.token}]` : ""}`
          );
        }
      }

      fs.writeFileSync(outputPath, lines.join("\n"), "utf8");

      expect(fs.existsSync(outputPath)).toBe(true);
      expect(rendered).toContain("Nv");
      expect(rendered).toContain("I  I  I");
      expect(payload?.map?.hotspots).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            label: "Healing machine",
            token: "H",
          }),
          expect.objectContaining({
            label: "Bookshelf",
            token: "B",
          }),
          expect.objectContaining({
            label: "Window",
            token: "W",
          }),
          expect.objectContaining({
            label: "Trash can",
            token: "T",
          }),
          expect.objectContaining({
            label: "PC",
            token: "P",
          }),
        ])
      );
    },
    15000
  );

  it(
    "keeps observe payload, status, and mapInfo aligned to the same Elm's Lab starter frame",
    async () => {
      const sessionId = "elms-lab-frame-consistency";
      removeAutosave(sessionId);
      const session = getMcpSession(sessionId);
      await session.ensureReady();
      const sessionAny = session as unknown as {
        game: {
          getOverworld: () => {
            load_map: (mapName: string) => void;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            _sync_player_state?: () => void;
          };
        };
        captureSnapshot: () => void;
        observePayload: () => {
          map?: {
            player?: { coords?: { x: number; y: number } };
            hotspots?: Array<{ label: string; coords: { x: number; y: number }; token?: string }>;
          };
          flow_state?: unknown;
        } | null;
      };

      positionElmsLabPlayer(sessionAny.game.getOverworld(), { x: 13, y: 5, direction: "down" });
      sessionAny.captureSnapshot();

      const status = await session.status();
      const payload = sessionAny.observePayload();
      const mapInfo = await session.mapInfo();

      expect(payload?.map).toEqual(status.map_details);
      expect(status.map_details).toEqual(mapInfo);
      expect(status.flow_state).toEqual(payload?.flow_state);
      expect(mapInfo.hotspots).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            label: "Cyndaquil Poke Ball",
            token: "!",
            coords: { x: 13, y: 7 },
            interactable: true,
          }),
        ])
      );
    },
    15000
  );

  it(
    "advances the Elm's Lab starter ball interaction from the same rendered engine frame instead of returning stale no-change state",
    async () => {
      const sessionId = "elms-lab-starter-pickup";
      removeAutosave(sessionId);
      const session = getMcpSession(sessionId);
      await session.ensureReady();
      const sessionAny = session as unknown as {
        game: {
          getOverworld: () => {
            load_map: (mapName: string) => void;
            player_x: number;
            player_y: number;
            prev_player_x: number;
            prev_player_y: number;
            target_tile_x: number;
            target_tile_y: number;
            player_direction: string;
            player_object?: {
              x: number;
              y: number;
              prevX: number;
              prevY: number;
              direction: string;
              updatePixelPosition?: () => void;
            } | null;
            _sync_player_state?: () => void;
          };
        };
        captureSnapshot: () => void;
        observeText: () => string;
      };

      const gameState = sessionAny.game.getGameState();
      gameState.wram.map_scenes.ElmsLab = "SCENE_ELMSLAB_NOOP";
      gameState.wram.map_scene_indices.ElmsLab = 3;
      positionElmsLabPlayer(sessionAny.game.getOverworld(), { x: 11, y: 7, direction: "right" });
      sessionAny.captureSnapshot();

      const beforeText = sessionAny.observeText();
      const beforeStatus = await session.status();
      const action = await session.press("a", 1);
      const afterStatus = await session.status();

      expect(beforeText).toContain("Cyndaquil Poke Ball");
      expect(beforeStatus.map_details?.hotspots).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            label: "Cyndaquil Poke Ball",
            token: "!",
            coords: { x: 13, y: 7 },
            interactable: true,
          }),
        ])
      );
      expect(action.result.changed).toBe(true);
      expect(action.result.reason).toBeUndefined();
      expect(
        Boolean(afterStatus.prompt?.pending) ||
          Boolean(afterStatus.in_dialog) ||
          Boolean(afterStatus.text_box_open)
      ).toBe(true);
      expect(
        [
          action.snapshotText?.includes("Cyndaquil"),
          action.snapshotText?.includes("YES"),
          action.snapshotText?.includes("NO"),
          action.snapshotText?.includes("Professor Elm"),
        ].some(Boolean)
      ).toBe(true);
    },
    15000
  );

  it(
    "renders multiple unique tiles in the overworld snapshot",
    async () => {
      const session = getMcpSession("tilemap-unique");
      await session.ensureReady();
      await session.advanceFrames(1);
    const sessionAny = session as unknown as { renderUi: { screen: { get_width: () => number; get_height: () => number; getImageData: () => ImageData } } };
    const surface = sessionAny.renderUi.screen;
    const width = surface.get_width();
    const height = surface.get_height();
    const data = surface.getImageData().data;
    const tileHashes = new Set<number>();
    for (let y = 0; y < height; y += TILE_SIZE) {
      for (let x = 0; x < width; x += TILE_SIZE) {
        let hash = 0;
        for (let ty = 0; ty < TILE_SIZE; ty += 1) {
          for (let tx = 0; tx < TILE_SIZE; tx += 1) {
            const pixelIndex = ((y + ty) * width + (x + tx)) * 4;
            for (let channel = 0; channel < 4; channel += 1) {
              hash = (hash * 31 + (data[pixelIndex + channel] ?? 0)) >>> 0;
            }
          }
        }
        tileHashes.add(hash);
      }
    }
      expect(tileHashes.size).toBeGreaterThan(4);
    },
    15000
  );
});
