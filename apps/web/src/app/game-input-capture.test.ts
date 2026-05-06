jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
}));

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { JOY_DOWN } from "@pokecrystal/core/core/constants";
import type { GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { DataLoader } from "@pokecrystal/core/core/data-loader";

type MockFontRenderer = BaseFontRenderer & {
  font_tiles?: Record<number, InstanceType<typeof gameEngine.Surface>>;
};

type MockTextUI = TextUI & {
  tile_size?: number;
  font: MockFontRenderer;
};

interface TilesetInstance {
  tilesetName: string;
  metatiles: Array<{ collision: number[] }>;
  renderMetatile(): void;
  renderPriorityMetatile(): void;
}

type TilesetConstructor = new (tilesetName: string) => TilesetInstance;

type GlobalOverrides = {
  fetch?: typeof globalThis.fetch | undefined;
  createImageBitmap?: typeof globalThis.createImageBitmap | undefined;
  Tileset?: TilesetConstructor;
};

type GameInternals = {
  ui: TextUI;
  handleInput: () => void;
};

type GameDrawInternals = GameInternals & {
  currentState: string;
  battle: { draw: () => void } | null;
  battleUi: { presented_this_frame?: boolean } | null;
  draw: () => void;
};

type TestOverworld = OverworldEngine & {
  ui: {
    eventQueue?: GameEngineEventQueue;
  };
};

const getInternals = (target: Game): GameInternals =>
  target as unknown as GameInternals;

const invokeHandleInput = (target: Game): void => {
  getInternals(target).handleInput();
};

const getDrawInternals = (target: Game): GameDrawInternals =>
  target as unknown as GameDrawInternals;

const invokeDraw = (target: Game): void => {
  getDrawInternals(target).draw();
};

const getEventQueue = (overworld: TestOverworld): GameEngineEventQueue => {
  const queue = overworld.ui.eventQueue;
  if (!queue) {
    throw new Error("Overworld UI event queue is missing.");
  }
  return queue;
};

const buildGame = async (): Promise<Game> => {
  const ui = new TextUI(160, 144, 1, null, false, 0) as MockTextUI;
  const fontTiles: Record<number, InstanceType<typeof gameEngine.Surface>> = {};
  for (let i = 0; i < 256; i += 1) {
    fontTiles[i] = new gameEngine.Surface(8, 8);
  }
  ui.tile_size = 8;
  const fontRenderer = ui.font;
  fontRenderer.font_tiles = fontTiles as unknown as Record<number, Surface>;
  const noopRender: (..._args: Parameters<NonNullable<BaseFontRenderer["renderText"]>>) => void = () => {};
  fontRenderer.render_text = noopRender;
  fontRenderer.renderText = noopRender;

  const globalScope = globalThis as GlobalOverrides;
  const originalFetch = globalScope.fetch;
  const originalCreateImageBitmap = globalScope.createImageBitmap;
  const originalTileset = globalScope.Tileset;
  const originalImageLoad = gameEngine.image.load;
  const originalInitAssets = OverworldEngine.prototype.init_assets;

  class TilesetStub implements TilesetInstance {
    public tilesetName: string;
    public metatiles: Array<{ collision: number[] }>;

    constructor(tilesetName: string) {
      this.tilesetName = tilesetName || "placeholder";
      this.metatiles = Array.from({ length: 256 }, () => ({ collision: [0, 0, 0, 0] }));
    }

    renderMetatile(): void {
      // No-op for headless test coverage.
    }

    renderPriorityMetatile(): void {
      // No-op for headless test coverage.
    }
  }

  globalScope.fetch = undefined;
  globalScope.createImageBitmap = undefined;
  globalScope.Tileset = TilesetStub;
  gameEngine.image.load = async () => new gameEngine.Surface(24, 16);
  OverworldEngine.prototype.init_assets = async () => {};

  try {
    return await Game.create(ui);
  } finally {
    globalScope.fetch = originalFetch;
    globalScope.createImageBitmap = originalCreateImageBitmap;
    globalScope.Tileset = originalTileset;
    gameEngine.image.load = originalImageLoad;
    OverworldEngine.prototype.init_assets = originalInitAssets;
  }
};

describe("Game input capture", () => {
  it("keeps the event queue intact while overworld captures input", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    const queue = getEventQueue(overworld);

    overworld.input_capture_active = true;
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_DOWN }));
    invokeHandleInput(game);

    expect(queue.length).toBe(1);

    overworld.input_capture_active = false;
    invokeHandleInput(game);

    expect(queue.length).toBe(0);
  });

  it("keeps the event queue intact while the Unown puzzle overlay is active", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    const queue = getEventQueue(overworld);

    game.getGameState().wram.wUnownState = 1;
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_DOWN }));
    invokeHandleInput(game);

    expect(queue.length).toBe(1);

    game.getGameState().wram.wUnownState = 0;
    invokeHandleInput(game);

    expect(queue.length).toBe(0);
  });

  it("keeps drawing overworld frames while input capture is active", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    const queue = getEventQueue(overworld);
    const ui = getInternals(game).ui;
    const clearSpy = jest.spyOn(ui, "clearScreen");

    overworld.input_capture_active = true;
    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_DOWN }));
    game.tick();

    expect(queue.length).toBe(1);
    expect(clearSpy).toHaveBeenCalled();

    clearSpy.mockRestore();
  });

  it("preserves a non-overworld prompt surface while input capture owns the overworld", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    const ui = getInternals(game).ui;
    const overworldDrawSpy = jest.spyOn(overworld, "draw");
    const clearSpy = jest.spyOn(ui, "clearScreen");

    ui.renderSnapshot(["Prompt"], ["Up/Down=Choose A=OK"], "Prompt", "Legend", ["▶ SUNDAY"], null, [
      "What day is it?",
    ]);
    overworld.input_capture_active = true;

    invokeDraw(game);

    expect(overworldDrawSpy).not.toHaveBeenCalled();
    expect(clearSpy).not.toHaveBeenCalled();
    expect(ui.getSnapshot()).toMatchObject({
      viewportTitle: "Prompt",
      dialogueLines: ["What day is it?"],
    });

    overworldDrawSpy.mockRestore();
    clearSpy.mockRestore();
  });

  it("reports ASM-style textbox continuation separately from prompts", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld & {
      dialogue?: {
        active?: boolean;
        visible?: boolean;
        waiting_for_input?: boolean;
        pending_waits?: number;
        _yes_no_prompt?: unknown;
      } | null;
    };

    overworld.dialogue = {
      active: false,
      visible: false,
      waiting_for_input: true,
      pending_waits: 1,
      _yes_no_prompt: null,
    };

    const status = game.getDebugStatus();

    expect(status.in_dialog).toBe(true);
    expect(status.text_advance_pending).toBe(true);
    expect(status.prompt_pending).toBe(false);
  });

  it("keeps advancing overworld updates while input capture is active", async () => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld;
    const updateSpy = jest.spyOn(overworld, "update");

    overworld.input_capture_active = true;
    game.tick();

    expect(updateSpy).toHaveBeenCalled();

    updateSpy.mockRestore();
  });

  it("skips overworld drawing while Unown puzzle overlay is active", async () => {
    const game = await buildGame();
    const ui = getInternals(game).ui;
    const clearSpy = jest.spyOn(ui, "clearScreen");

    game.getGameState().wram.wUnownState = 1;
    game.tick();

    expect(clearSpy).not.toHaveBeenCalled();

    clearSpy.mockRestore();
  });

  it("skips overworld drawing while Unown overlay lock depth is active", async () => {
    const game = await buildGame();
    const ui = getInternals(game).ui;
    const clearSpy = jest.spyOn(ui, "clearScreen");

    (game.getGameState() as { __unown_overlay_lock_depth__?: number }).__unown_overlay_lock_depth__ = 1;
    game.tick();

    expect(clearSpy).not.toHaveBeenCalled();

    clearSpy.mockRestore();
  });

  it("does not present twice when battle UI already presented this frame", async () => {
    const game = await buildGame();
    const ui = getInternals(game).ui;
    const updateSpy = jest.spyOn(ui, "update");
    const internals = getDrawInternals(game);
    const battleUi = { presented_this_frame: false };
    internals.currentState = "battle";
    internals.battleUi = battleUi;
    internals.battle = {
      draw: () => {
        battleUi.presented_this_frame = true;
      },
    };

    invokeDraw(game);

    expect(updateSpy).not.toHaveBeenCalled();
    updateSpy.mockRestore();
  });

  it("still presents battle frames when the battle path did not present", async () => {
    const game = await buildGame();
    const ui = getInternals(game).ui;
    const updateSpy = jest.spyOn(ui, "update");
    const internals = getDrawInternals(game);
    internals.currentState = "battle";
    internals.battleUi = { presented_this_frame: false };
    internals.battle = {
      draw: () => {},
    };

    invokeDraw(game);

    expect(updateSpy).toHaveBeenCalledTimes(1);
    updateSpy.mockRestore();
  });

  it("latches joypad transitions once per frame from final input state", async () => {
    const game = await buildGame();
    const joypad = game.getGameState().hram.joypad;

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_DOWN }));
    game.postEvent(new gameEngine.event.Event("keyup", { key: gameEngine.K_DOWN }));
    game.tick();

    expect(joypad.hJoyPressed & JOY_DOWN).toBe(0);
    expect(joypad.hJoyReleased & JOY_DOWN).toBe(0);
    expect(joypad.hJoyDown & JOY_DOWN).toBe(0);
  });

  it("keeps held joypad state across frames and releases on keyup", async () => {
    const game = await buildGame();
    const joypad = game.getGameState().hram.joypad;

    game.postEvent(new gameEngine.event.Event("keydown", { key: gameEngine.K_DOWN }));
    game.tick();
    expect(joypad.hJoyPressed & JOY_DOWN).toBe(JOY_DOWN);
    expect(joypad.hJoyDown & JOY_DOWN).toBe(JOY_DOWN);

    game.tick();
    expect(joypad.hJoyPressed & JOY_DOWN).toBe(0);
    expect(joypad.hJoyDown & JOY_DOWN).toBe(JOY_DOWN);

    game.postEvent(new gameEngine.event.Event("keyup", { key: gameEngine.K_DOWN }));
    game.tick();
    expect(joypad.hJoyReleased & JOY_DOWN).toBe(JOY_DOWN);
    expect(joypad.hJoyDown & JOY_DOWN).toBe(0);
  });

  it.each([
    ["Cherrygrove", "CherrygrovePokecenter1F"],
    ["Olivine", "OlivinePokecenter1F"],
  ])("starts the nurse interaction from a live A press at the %s counter", async (_townName, mapName) => {
    const game = await buildGame();
    const overworld = game.getOverworld() as TestOverworld & {
      data_loader?: DataLoader;
      player_x: number;
      player_y: number;
      player_direction: string;
      tileset?: { ready?: Promise<unknown> } | null;
      script_runner?: {
        pokemon_center?: {
          runNurseInteraction: (
            runner: unknown,
            eventManager: unknown,
            activeOverworld: unknown
          ) => Promise<void> | void;
        };
      } | null;
    };
    const pokemonCenter = overworld.script_runner?.pokemon_center;
    if (!pokemonCenter) {
      throw new Error("Expected the live overworld runner to expose a PokemonCenterSystem.");
    }
    const nurseSpy = jest
      .spyOn(pokemonCenter, "runNurseInteraction")
      .mockImplementation(async () => {
        overworld.input_capture_active = true;
      });

    overworld.data_loader?.ensure_overworld_data({ map_name: mapName });
    overworld.load_map(mapName);
    await overworld.tileset?.ready;
    await Promise.resolve();

    overworld.player_x = 7;
    overworld.player_y = 7;
    overworld.player_direction = "up";

    game.postEvent(new gameEngine.event.Event("keydown", { key: "KeyZ", code: "KeyZ" }));
    invokeHandleInput(game);
    await Promise.resolve();

    expect(nurseSpy).toHaveBeenCalled();
    expect(overworld.input_capture_active).toBe(true);

    nurseSpy.mockRestore();
  });
});
