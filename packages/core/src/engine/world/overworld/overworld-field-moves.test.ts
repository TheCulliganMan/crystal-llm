import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { GB_FRAME_DURATION_MS, GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { FieldDialogueManager } from "@pokecrystal/core/ui/text/dialogue";
import * as events from "@pokecrystal/core/engine/events/events";
import * as maps from "@pokecrystal/core/engine/world/maps";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import type { Pokemon } from "@pokecrystal/core/core/models";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { AudioEngine as RuntimeAudioEngine } from "@pokecrystal/core/engine/systems/audio";
import * as treeEncounters from "@pokecrystal/assets/content/tree-encounters";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile";
import { OverworldEngine } from "./overworld";
import { OverworldFieldMoveMixin } from "./overworld-field-moves";
import { getBooleanFlag, setBooleanFlag } from "@pokecrystal/core/engine/world/overworld/flag-collection";
import { PLAYER_STATE_FLAGS } from "@pokecrystal/core/engine/world/overworld/player-state-flags";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import { FishingRodState, type FishingSession } from "@pokecrystal/core/engine/world/overworld/fishing";

class TestOverworldFieldMoves extends OverworldFieldMoveMixin {
  public playFieldMoveSound(id: string): void {
    this._play_field_move_sound(id);
  }
}

class DialogueDrivenCutOverworld extends OverworldFieldMoveMixin {
  public promptConfirmed = false;
  private dialogueAdvanceCount = 0;
  private frameCount = 0;

  protected _run_cut_animation(_tile_x: number, _tile_y: number, _variant: string): void {
    return;
  }

  protected async _await_field_move_frame_async(): Promise<void> {
    this.frameCount += 1;
    const dialogue = this.dialogue as FieldDialogueManager | null;
    if (dialogue?.active && !dialogue._yes_no_prompt && this.dialogueAdvanceCount < 8) {
      this.dialogueAdvanceCount += 1;
      dialogue.handle_input({ type: "keydown", button: "a", is_press: true });
      return;
    }
    if (dialogue?._yes_no_prompt && !this.promptConfirmed) {
      this.promptConfirmed = true;
      dialogue.handle_input({ type: "keydown", button: "a", is_press: true });
    }
    if (this.frameCount > 80 && !this.promptConfirmed) {
      throw new Error("Cut confirmation did not route through overworld dialogue.");
    }
  }
}

class DialogueDrivenHeadbuttOverworld extends OverworldFieldMoveMixin {
  public promptConfirmed = false;
  private frameCount = 0;

  public get_facing_tile_coords = jest.fn(() => [7, 9] as [number, number]);
  public _tile_is_headbutt_tree = jest.fn(() => true);
  public _run_headbutt_animation = jest.fn();
  public _run_field_move_animation_frames_async = jest.fn(async () => undefined);

  protected async _await_field_move_frame_async(): Promise<void> {
    this.frameCount += 1;
    const dialogue = this.dialogue as FieldDialogueManager | null;
    if (dialogue?.active && !dialogue._yes_no_prompt) {
      dialogue.handle_input({ type: "keyup", key: "z", code: "KeyZ", button: "a", is_press: false });
      dialogue.handle_input({ type: "keydown", key: "z", code: "KeyZ", button: "a", is_press: true });
      return;
    }
    if (dialogue?._yes_no_prompt && !this.promptConfirmed) {
      this.promptConfirmed = true;
      dialogue.handle_input({ type: "keyup", key: "z", code: "KeyZ", button: "a", is_press: false });
      dialogue.handle_input({ type: "keydown", key: "z", code: "KeyZ", button: "a", is_press: true });
    }
    if (this.frameCount > 120 && !this.promptConfirmed) {
      throw new Error("Headbutt confirmation did not route through overworld dialogue.");
    }
    if (this.frameCount > 300) {
      throw new Error(`Headbutt dialogue did not close. active=${dialogue?.active} waiting=${dialogue?.waiting_for_input} pending=${dialogue?.pending_waits_count}`);
    }
  }
}

class DialogueDrivenFishingOverworld extends OverworldFieldMoveMixin {
  private frameCount = 0;

  public get_facing_tile_coords = jest.fn(() => [0, 0] as [number, number]);
  public stop_player_movement = jest.fn();

  public get activeFishingSession(): FishingSession | null {
    return this._active_fishing_session;
  }

  public resolveFishingOutcome(session: FishingSession, bite: boolean | null): Promise<void> {
    return this._resolve_fishing_outcome_async(session, bite);
  }

  protected async _await_field_move_frame_async(): Promise<void> {
    this.frameCount += 1;
    const dialogue = this.dialogue as FieldDialogueManager | null;
    if (dialogue?.active) {
      dialogue.handle_input({ type: "keyup", key: "z", code: "KeyZ", button: "a", is_press: false });
      dialogue.handle_input({ type: "keydown", key: "z", code: "KeyZ", button: "a", is_press: true });
    }
    if (this.frameCount > 300) {
      throw new Error("Fishing dialogue did not advance.");
    }
  }
}

class TestWaterfallOverworld extends OverworldFieldMoveMixin {
  public WALK_FRAMES = 8;
  public move_player = jest.fn((direction: string) => {
    const [dx, dy] = this._direction_to_vector?.(direction) ?? [0, 0];
    this._pending_move = [dx * this.TILES_PER_COLLISION, dy * this.TILES_PER_COLLISION];
    this.is_moving = true;
  });
  public update = jest.fn(() => {
    if (!this._pending_move || !this.is_moving) {
      return;
    }
    this.player_x += this._pending_move[0];
    this.player_y += this._pending_move[1];
    this._pending_move = null;
    this.is_moving = false;
  });
  public _pending_move: [number, number] | null = null;

  constructor() {
    super();
    this._direction_to_vector = (direction: string): [number, number] => {
      if (direction === "up") return [0, -1];
      if (direction === "down") return [0, 1];
      if (direction === "left") return [-1, 0];
      if (direction === "right") return [1, 0];
      return [0, 0];
    };
  }

  public enableAutoAdvance(): void {
    this._field_move_auto_advance = true;
  }

  protected _current_player_collision(): number | null {
    if (this.player_y >= 3) {
      return resolveCollisionValue("WATERFALL");
    }
    return resolveCollisionValue("WATER");
  }

  protected _party_has_move(_move_name: string): boolean {
    return true;
  }

  public check_badge(_badge_id: number): boolean {
    return true;
  }
}

class EngineTilesetStub {
  public tilesetName: string;
  public metatiles: Array<{ collision: number[]; tiles: number[][] }>;

  constructor(tilesetName: string = "johto", _timeOfDay: string = "day") {
    this.tilesetName = tilesetName || "johto";
    this.metatiles = Array.from({ length: 256 }, () => ({
      collision: [0, 0, 0, 0],
      tiles: [
        [0, 0],
        [0, 0],
      ],
    }));
  }

  renderMetatile(): void {}

  renderPriorityMetatile(): void {}
}

const buildFieldMoveUi = (): {
  screen: InstanceType<typeof gameEngine.Surface>;
  tile_size: number;
  tileSize: number;
  font: BaseFontRenderer;
  eventQueue: ReturnType<typeof gameEngine.event.createQueue>;
} => {
  const screen = new gameEngine.Surface(160, 144);
  const font = {
    font_tiles: {},
    render_text: jest.fn(),
    renderText: jest.fn(),
  } as unknown as BaseFontRenderer;
  const drawWindow = (
    surface: InstanceType<typeof gameEngine.Surface>,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
  ): void => {
    surface.fill([255, 255, 255, 255], new gameEngine.Rect(x * 8, y * 8, widthTiles * 8, heightTiles * 8));
  };
  const drawTextBox = (
    surface: InstanceType<typeof gameEngine.Surface>,
    _text: string,
    x: number,
    y: number,
    widthTiles: number,
    heightTiles: number,
  ): void => {
    surface.fill([255, 255, 255, 255], new gameEngine.Rect(x * 8, y * 8, widthTiles * 8, heightTiles * 8));
  };
  return {
    screen,
    tile_size: 8,
    tileSize: 8,
    font,
    eventQueue: gameEngine.event.createQueue(),
    draw_window: drawWindow,
    drawWindow,
    draw_text_box: drawTextBox,
    drawTextBox,
  } as {
    screen: InstanceType<typeof gameEngine.Surface>;
    tile_size: number;
    tileSize: number;
    font: BaseFontRenderer;
    eventQueue: ReturnType<typeof gameEngine.event.createQueue>;
  };
};

const createWaterfallMap = (): OverworldMap => {
  const width = 1;
  const height = 3;
  const metatileIds = [0, 1, 1];
  return {
    mapName: "WaterfallTest",
    width,
    height,
    dataLoader: null,
    metatileIds,
    getMetatileAt(x: number, y: number): number {
      if (x < 0 || x >= width || y < 0 || y >= height) {
        throw new Error("Metatile lookup out of range.");
      }
      return metatileIds[y * width + x];
    },
  } as OverworldMap;
};

const createWaterfallTileset = (): OverworldTilesetLike => {
  const water = resolveCollisionValue("WATER");
  const waterfall = resolveCollisionValue("WATERFALL");
  const waterMetatile = { collision: [water, water, water, water] };
  const waterfallMetatile = { collision: [waterfall, waterfall, waterfall, waterfall] };
  return {
    tilesetName: "test",
    metatiles: [waterMetatile, waterfallMetatile],
    renderMetatile: () => {},
    renderPriorityMetatile: () => {},
  } as OverworldTilesetLike;
};

describe("OverworldFieldMoveMixin audio guards", () => {
  it("plays field move sounds via playSound when play_sound is missing", () => {
    const overworld = new TestOverworldFieldMoves();
    const playSound = jest.fn();
    overworld.audio_engine = { playSound } as unknown as AudioEngine;

    expect(() => overworld.playFieldMoveSound("SFX_WARP_FROM")).not.toThrow();
    expect(playSound).toHaveBeenCalledWith("SFX_WARP_FROM");
  });
});

describe("OverworldFieldMoveMixin timing", () => {
  it("uses GB frame duration for per-frame field move delays", () => {
    const overworld = new TestOverworldFieldMoves();
    overworld.game_state = createInitialGameState();
    const delaySpy = jest.spyOn(gameEngine.time, "delay").mockImplementation(() => {});

    (overworld as unknown as { _run_field_move_delay: (frames: number) => void })._run_field_move_delay(3);

    expect(delaySpy).toHaveBeenCalledTimes(3);
    for (const [ms] of delaySpy.mock.calls) {
      expect(ms).toBeCloseTo(GB_FRAME_DURATION_MS, 6);
    }
    delaySpy.mockRestore();
  });

  it("uses requestAnimationFrame for async field-move frame pacing when available", async () => {
    const overworld = new TestOverworldFieldMoves();
    const globalScope = globalThis as typeof globalThis & {
      requestAnimationFrame?: typeof requestAnimationFrame;
    };
    const previousRaf = globalScope.requestAnimationFrame;
    const rafMock = jest.fn((callback: FrameRequestCallback): number => {
      callback(0);
      return 1;
    });
    Object.defineProperty(globalThis, "requestAnimationFrame", {
      configurable: true,
      writable: true,
      value: rafMock,
    });
    const timeoutSpy = jest.spyOn(globalThis, "setTimeout");

    try {
      await (overworld as unknown as { _await_field_move_frame_async: () => Promise<void> })._await_field_move_frame_async();
      expect(rafMock).toHaveBeenCalledTimes(1);
      expect(timeoutSpy).not.toHaveBeenCalled();
    } finally {
      timeoutSpy.mockRestore();
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
      } else {
        Object.defineProperty(globalThis, "requestAnimationFrame", {
          configurable: true,
          writable: true,
          value: previousRaf,
        });
      }
    }
  });

  it("falls back to GB frame-duration timeouts when requestAnimationFrame is unavailable", async () => {
    const overworld = new TestOverworldFieldMoves();
    const globalScope = globalThis as typeof globalThis & {
      requestAnimationFrame?: typeof requestAnimationFrame;
    };
    const previousRaf = globalScope.requestAnimationFrame;
    delete globalScope.requestAnimationFrame;
    const timeoutSpy = jest
      .spyOn(globalThis, "setTimeout")
      .mockImplementation(((callback: (...args: unknown[]) => void) => {
        callback();
        return 1 as ReturnType<typeof setTimeout>;
      }) as typeof setTimeout);

    try {
      await (overworld as unknown as { _await_field_move_frame_async: () => Promise<void> })._await_field_move_frame_async();
      expect(timeoutSpy).toHaveBeenCalledTimes(1);
      const [callback, ms] = timeoutSpy.mock.calls[0] as [TimerHandler, number];
      expect(typeof callback).toBe("function");
      expect(ms).toBeCloseTo(GB_FRAME_DURATION_MS, 6);
    } finally {
      timeoutSpy.mockRestore();
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
      } else {
        Object.defineProperty(globalThis, "requestAnimationFrame", {
          configurable: true,
          writable: true,
          value: previousRaf,
        });
      }
    }
  });

  it("ticks blocking field-move animation loops at GB frame rate", () => {
    const overworld = new TestOverworldFieldMoves();
    overworld.game_state = createInitialGameState();

    const displayInitSpy = jest.spyOn(gameEngine.display, "get_init").mockReturnValue(true);
    const flipSpy = jest.spyOn(gameEngine.display, "flip").mockImplementation(() => {});
    const tickSpy = jest.spyOn(gameEngine.time.Clock.prototype, "tick").mockImplementation(() => {});

    try {
      (overworld as unknown as { _run_field_move_animation_frames: (frameCount: number) => void })._run_field_move_animation_frames(2);

      expect(tickSpy).toHaveBeenCalledTimes(2);
      for (const [fps] of tickSpy.mock.calls) {
        expect(Number(fps)).toBeCloseTo(GB_FRAME_RATE, 6);
      }
    } finally {
      tickSpy.mockRestore();
      flipSpy.mockRestore();
      displayInitSpy.mockRestore();
    }
  });
});

describe("OverworldFieldMoveMixin.check_badge", () => {
  it("reads Johto badge ownership using ASM ids", () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    gameState.sram.badges.johto[1] = true;
    overworld.game_state = gameState;

    expect(overworld.check_badge(1)).toBe(true);
    expect(overworld.check_badge(0)).toBe(false);
  });

  it("reads Kanto badge ownership using ASM ids 8-15", () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    gameState.sram.badges.kanto[0] = true;
    gameState.sram.badges.kanto[7] = true;
    overworld.game_state = gameState;

    expect(overworld.check_badge(8)).toBe(true);
    expect(overworld.check_badge(15)).toBe(true);
    expect(overworld.check_badge(9)).toBe(false);
  });

  it("falls back to engine flags when SRAM badge bits are not set", () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_HIVEBADGE", true);
    overworld.game_state = gameState;

    expect(overworld.check_badge(1)).toBe(true);
  });

  it("throws for badge ids outside ASM range", () => {
    const overworld = new OverworldFieldMoveMixin();
    overworld.game_state = createInitialGameState();

    expect(() => overworld.check_badge(-1)).toThrow("out of ASM range");
    expect(() => overworld.check_badge(16)).toThrow("out of ASM range");
  });

  it("throws when badge banks are not ASM-sized", () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    gameState.sram.badges.johto = [true] as boolean[];
    overworld.game_state = gameState;

    expect(() => overworld.check_badge(0)).toThrow("must contain exactly 8");
  });
});

describe("OverworldFieldMoveMixin field-move text resolution", () => {
  it("throws instead of fabricating field-move text from an unknown label", () => {
    const overworld = new OverworldFieldMoveMixin();

    expect(() =>
      (
        overworld as unknown as {
          _resolve_field_move_text: (label: string) => string;
        }
      )._resolve_field_move_text("TotallyMissingFieldMoveText"),
    ).toThrow("Missing ASM field-move text for label 'TotallyMissingFieldMoveText'.");
  });
});

describe("OverworldFieldMoveMixin.handle_flash", () => {
  it("runs the blinding flash sequence without waiting for input", async () => {
    const waitSpy = jest.spyOn(events, "waitForInput");
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const fadeToWhite = jest.fn();
    const fadeFromWhite = jest.fn();
    const refreshTiles = jest.fn();
    const playSound = jest.fn();

    class FlashOverworld extends OverworldFieldMoveMixin {
      public fade_to_white = fadeToWhite;
      public fade_from_white = fadeFromWhite;
      public _refresh_tileset_for_current_map = refreshTiles;
      public _wait_for_dialogue_render = jest.fn();
      public _wait_for_dialogue_closed = jest.fn();
      public _run_blocking_frames = jest.fn();
      public _wait_for_sfx_completion = jest.fn();
      public _party_has_move(): boolean {
        return true;
      }
      public check_badge(): boolean {
        return true;
      }
      public _current_map_attributes() {
        return { time_of_day: "dark", tileset_name: "cave", environment: "CAVE" };
      }
      public _normalise_time_of_day_label(): string {
        return "dark";
      }
    }

    const overworld = new FlashOverworld();
    overworld.game_state = gameState;
    overworld.event_manager = eventManager;
    overworld.audio_engine = { play_sound: playSound } as unknown as AudioEngine;
    overworld.current_map_name = "TEST_CAVE";

    await overworld.handle_flash();

    expect(fadeToWhite).toHaveBeenCalledWith(8);
    expect(fadeFromWhite).toHaveBeenCalledWith(8);
    expect(refreshTiles).toHaveBeenCalled();
    expect(playSound).toHaveBeenCalledWith("SFX_FLASH");
    expect(waitSpy).not.toHaveBeenCalled();
    expect(getBooleanFlag(gameState.wram.engine_flags, "STATUSFLAGS_FLASH")).toBe(true);
    expect(gameState.wram.flash_active_maps.TEST_CAVE).toBe(true);
    waitSpy.mockRestore();
  });
});

describe("OverworldFieldMoveMixin waterfall", () => {
  it("climbs the waterfall until leaving waterfall collisions", async () => {
    const overworld = new TestWaterfallOverworld();
    overworld.map = createWaterfallMap();
    overworld.tileset = createWaterfallTileset();
    overworld.game_state = createInitialGameState();
    overworld.event_manager = new EventManager();
    overworld.enableAutoAdvance();

    overworld.player_state = PlayerState.SURF;
    overworld.player_direction = "up";
    overworld.player_x = 0;
    overworld.player_y = 5;

    await overworld.handle_waterfall(0, 1);

    expect(overworld.move_player).toHaveBeenCalledTimes(2);
    expect(overworld.player_y).toBe(1);
    expect(overworld.WALK_FRAMES).toBe(8);
  });
});

describe("OverworldFieldMoveMixin surf", () => {
  const createSurfMap = (): OverworldMap => {
    const width = 1;
    const height = 1;
    const metatileIds = [0];
    return {
      mapName: "SurfTest",
      width,
      height,
      dataLoader: null,
      metatileIds,
      getMetatileAt(x: number, y: number): number {
        if (x < 0 || x >= width || y < 0 || y >= height) {
          throw new Error("Metatile lookup out of range.");
        }
        return metatileIds[y * width + x];
      },
    } as OverworldMap;
  };

  const createSurfTileset = (): OverworldTilesetLike => {
    const water = resolveCollisionValue("WATER");
    const metatile = { collision: [water, water, water, water] };
    return {
      tilesetName: "test",
      metatiles: [metatile],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  class TestSurfOverworld extends OverworldFieldMoveMixin {
    public TILES_PER_COLLISION = 2;
    public player_x = 0;
    public player_y = 0;
    public prev_player_x = 0;
    public prev_player_y = 0;
    public target_tile_x = 0;
    public target_tile_y = 0;
    public player_direction = "down";
    public _player_state = PlayerState.NORMAL;
    public get player_state(): PlayerState {
      return this._player_state;
    }
    public set player_state(value: PlayerState) {
      this._player_state = value;
      if (this.game_state) {
        this.game_state.wram.wPlayerState = PLAYER_STATE_FLAGS[value] ?? 0;
      }
    }
    public player_object: OverworldObject | null = { objectIndex: 0, x: 0, y: 0 } as OverworldObject;
    public queue_movement_task = jest.fn();
    public start_map_music = jest.fn();
    public stop_player_movement = jest.fn();
    public _create_player_animations = jest.fn(() => ({}));
    public _field_move_confirm_callback = jest.fn(() => true);
    public get_facing_tile_coords(): [number, number] {
      return [0, 0];
    }

    protected _party_has_move(_move_name: string): boolean {
      return true;
    }

    protected _get_party_move_holder(_move_name: string): [Pokemon | null, string] {
      if (this.game_state) {
        this.game_state.wram.wCurPartyMon = 0;
      }
      return [{ nickname: "SURFER", species: { id: "TOTODILE" } } as unknown as Pokemon, "SURFER"];
    }

    public check_badge(_badge_id: number): boolean {
      return true;
    }
  }

  it("prompts, starts surfing, and queues the surf step", async () => {
    const overworld = new TestSurfOverworld();
    const gameState = createInitialGameState();
    overworld.game_state = gameState;
    overworld.event_manager = new EventManager(gameState);
    overworld.map = createSurfMap();
    overworld.tileset = createSurfTileset();
    overworld.current_map_name = "SurfTest";

    await overworld.handle_surf(0, 0);

    expect(overworld._field_move_confirm_callback).toHaveBeenCalledWith("SURF");
    expect(overworld.player_state).toBe(PlayerState.SURF);
    expect(gameState.wram.surfing).toBe(true);
    expect(gameState.wram.wSurfingPlayerState).toBe(PLAYER_STATE_FLAGS[PlayerState.SURF]);
    expect(overworld.player_sprite_id).toBe("surf");
    expect(overworld.start_map_music).toHaveBeenCalled();
    expect(overworld.queue_movement_task).toHaveBeenCalledWith(
      overworld.player_object,
      ["slow_step down", "step_end"],
      expect.any(Object)
    );
  });
});

describe("OverworldFieldMoveMixin fishing", () => {
  const createFishingMap = (): OverworldMap => {
    const metatileIds = [0];
    return {
      mapName: "FishingTest",
      width: 1,
      height: 1,
      dataLoader: null,
      metatileIds,
      getMetatileAt: () => metatileIds[0],
    } as OverworldMap;
  };

  const createFishingTileset = (water: boolean): OverworldTilesetLike => {
    const permission = water ? resolveCollisionValue("WATER") : 0;
    return {
      tilesetName: "fishing",
      metatiles: [{ collision: [permission, permission, permission, permission] }],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as OverworldTilesetLike;
  };

  const prepareFishingOverworld = (water = true): DialogueDrivenFishingOverworld => {
    const gameState = createInitialGameState();
    gameState.sram.options.no_text_scroll = true;
    const ui = buildFieldMoveUi();
    const eventManager = new EventManager(gameState);
    const dialogue = new FieldDialogueManager(
      ui,
      gameState,
      { event_manager: eventManager },
      null
    );
    for (const eventName of ["open_text", "show_text", "close_text", "wait_for_input"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }

    const overworld = new DialogueDrivenFishingOverworld();
    overworld.game_state = gameState;
    overworld.event_manager = eventManager;
    overworld.ui = ui;
    overworld.dialogue = dialogue;
    overworld.draw = jest.fn();
    overworld.map = createFishingMap();
    overworld.tileset = createFishingTileset(water);
    overworld.current_map_name = "FishingTest";
    overworld.data_loader = {
      map_attributes: new Map([["FishingTest", { fishing_group: null }]]),
    } as unknown as DataLoader;
    return overworld;
  };

  it("starts fishing through overworld dialogue instead of a private input queue", async () => {
    const overworld = prepareFishingOverworld(true);

    await expect(overworld.handle_fishing("OLD_ROD", { nickname: "FISHER" } as Pokemon)).resolves.toBe(true);

    expect(overworld.activeFishingSession).not.toBeNull();
    expect(overworld.stop_player_movement).toHaveBeenCalled();
  });

  it("advances the no-bite fishing outcome through async dialogue", async () => {
    const overworld = prepareFishingOverworld(true);
    await overworld.handle_fishing("OLD_ROD", { nickname: "FISHER" } as Pokemon);
    const session = overworld.activeFishingSession;
    if (!session) {
      throw new Error("Fishing session was not started.");
    }

    await overworld.resolveFishingOutcome(session, false);

    expect(overworld.activeFishingSession).toBeNull();
    expect(overworld.game_state?.wram.wFishingRodState).toBe(FishingRodState.IDLE);
  });

  it("shows the fishing failure text asynchronously when there is no water", async () => {
    const overworld = prepareFishingOverworld(false);

    await expect(overworld.handle_fishing("OLD_ROD", { nickname: "FISHER" } as Pokemon)).resolves.toBe(false);

    expect(overworld.activeFishingSession).toBeNull();
  });
});

describe("OverworldFieldMoveMixin HM menu use", () => {
  const pokemon = { nickname: "HMUSER", species: { id: "LAPRAS" } } as unknown as Pokemon;

  const createMenuUseMap = (): OverworldMap => {
    const width = 2;
    const height = 2;
    const metatileIds = [0, 0, 0, 0];
    return {
      mapName: "HmMenuTest",
      width,
      height,
      dataLoader: null,
      metatileIds,
      getMetatileAt(x: number, y: number): number {
        if (x < 0 || x >= width || y < 0 || y >= height) {
          throw new Error("Metatile lookup out of range.");
        }
        return metatileIds[y * width + x];
      },
    } as OverworldMap;
  };

  const createMenuUseTileset = (): OverworldTilesetLike => ({
    tilesetName: "menu",
    metatiles: [{ collision: [0, 0, 0, 0] }],
    renderMetatile: () => {},
    renderPriorityMetatile: () => {},
  });

  const prepareMenuUseOverworld = (): OverworldFieldMoveMixin => {
    const overworld = new OverworldFieldMoveMixin();
    overworld.map = createMenuUseMap();
    overworld.tileset = createMenuUseTileset();
    overworld.get_facing_tile_coords = jest.fn(() => [METATILE_WIDTH, METATILE_WIDTH]);
    return overworld;
  };

  it("keeps the overworld receiver when Cut adjusts facing counter tiles", async () => {
    const overworld = prepareMenuUseOverworld();
    const adjustmentReceivers: unknown[] = [];
    overworld._counter_adjusted_tile = function (this: unknown, tileX: number, tileY: number): [number, number] {
      adjustmentReceivers.push(this);
      return [tileX, tileY];
    };
    const handleCut = jest.spyOn(overworld, "handle_cut").mockResolvedValue(true);

    await expect(overworld.use_hm_from_menu("CUT", pokemon)).resolves.toBe(true);

    expect(adjustmentReceivers).toEqual([overworld]);
    expect(handleCut).toHaveBeenCalledWith(1, 1, pokemon);
  });

  it.each([
    ["CUT", "handle_cut", [1, 1, pokemon]],
    ["SURF", "handle_surf", [1, 1]],
    ["STRENGTH", "handle_strength", [0, 0]],
    ["FLASH", "handle_flash", []],
    ["WATERFALL", "handle_waterfall", [1, 1, { from_menu: true }]],
    ["WHIRLPOOL", "handle_whirlpool", [1, 1]],
    ["FLY", "handle_fly", [0, 0]],
  ])("dispatches %s from the party menu to %s", async (moveName, handlerName, expectedArgs) => {
    const overworld = prepareMenuUseOverworld() as OverworldFieldMoveMixin & Record<string, jest.Mock>;
    overworld._counter_adjusted_tile = jest.fn((tileX: number, tileY: number) => [tileX, tileY]);
    overworld[handlerName] = jest.fn(() => true);

    await expect(overworld.use_hm_from_menu(moveName, pokemon)).resolves.toBe(true);

    expect(overworld[handlerName]).toHaveBeenCalledWith(...expectedArgs);
  });

  it("runs Cut from the real OverworldEngine party-menu path through the async confirmation frame", async () => {
    const metadata = getMapMetadataByConstant("NEW_BARK_TOWN");
    if (!metadata) {
      throw new Error("Missing NEW_BARK_TOWN metadata.");
    }
    const gameState = createInitialGameState();
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;
    gameState.sram.badges.johto[1] = true;

    const dataLoader = new DataLoader() as DataLoader & { Tileset?: typeof EngineTilesetStub };
    dataLoader.Tileset = EngineTilesetStub;
    const ui = buildFieldMoveUi();
    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new EngineTilesetStub("johto", "day") as never,
      new RuntimeAudioEngine({ masterVolume: 0, muted: true }),
      ui,
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    ) as OverworldEngine & {
      use_hm_from_menu: (moveName: string, pokemon: Pokemon | null) => Promise<boolean>;
      get_facing_tile_coords: jest.Mock<[number, number], []>;
      dialogue: unknown;
    };

    const metatileIds = [0, 0, 0, 0x5b];
    engine.map = {
      mapName: "CutEngineTest",
      width: 2,
      height: 2,
      dataLoader: null,
      metatileIds,
      getMetatileAt(x: number, y: number): number {
        if (x < 0 || x >= 2 || y < 0 || y >= 2) {
          throw new Error("Metatile lookup out of range.");
        }
        return metatileIds[y * 2 + x];
      },
    } as OverworldMap;
    const cutTileset = new EngineTilesetStub("johto", "day") as unknown as OverworldTilesetLike;
    (cutTileset.metatiles as Array<{ collision: number[] }>)[0x5b].collision = [
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
    ];
    engine.tileset = cutTileset;
    engine.map_surface = new gameEngine.Surface(16, 16);
    engine.priority_surface = new gameEngine.Surface(16, 16);
    engine.dialogue = null;
    engine.draw = jest.fn();
    engine.get_facing_tile_coords = jest.fn(() => [METATILE_WIDTH, METATILE_WIDTH]);
    ui.eventQueue.push({ type: "keydown", button: "a", is_press: true });

    const globalScope = globalThis as typeof globalThis & {
      requestAnimationFrame?: typeof requestAnimationFrame;
    };
    const previousRaf = globalScope.requestAnimationFrame;
    delete globalScope.requestAnimationFrame;
    const timeoutSpy = jest
      .spyOn(globalThis, "setTimeout")
      .mockImplementation(((callback: (...args: unknown[]) => void) => {
        callback();
        return 1 as ReturnType<typeof setTimeout>;
      }) as typeof setTimeout);

    try {
      await expect(engine.use_hm_from_menu("CUT", pokemon)).resolves.toBe(true);
      expect(timeoutSpy).toHaveBeenCalled();
      expect(metatileIds[3]).toBe(0x3c);
    } finally {
      timeoutSpy.mockRestore();
      if (previousRaf === undefined) {
        delete globalScope.requestAnimationFrame;
      } else {
        Object.defineProperty(globalThis, "requestAnimationFrame", {
          configurable: true,
          writable: true,
          value: previousRaf,
        });
      }
    }
  });

  it("routes Cut tile replacement through the canonical map writer", async () => {
    const metadata = getMapMetadataByConstant("NEW_BARK_TOWN");
    if (!metadata) {
      throw new Error("Missing NEW_BARK_TOWN metadata.");
    }
    const gameState = createInitialGameState();
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;
    gameState.sram.badges.johto[1] = true;

    const dataLoader = new DataLoader() as DataLoader & { Tileset?: typeof EngineTilesetStub };
    dataLoader.Tileset = EngineTilesetStub;
    const ui = buildFieldMoveUi();
    const engine = new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new EngineTilesetStub("johto", "day") as never,
      new RuntimeAudioEngine({ masterVolume: 0, muted: true }),
      ui,
      { suppressInitialMapEntryEffects: true, suppressInitialMapMusic: true }
    ) as OverworldEngine & {
      use_hm_from_menu: (moveName: string, pokemon: Pokemon | null) => Promise<boolean>;
      get_facing_tile_coords: jest.Mock<[number, number], []>;
      _refresh_warp_permissions: jest.Mock;
    };

    const metatileIds = [0, 0, 0, 0x5b];
    engine.map = {
      mapName: "CutCanonicalWriterTest",
      width: 2,
      height: 2,
      dataLoader: null,
      metatileIds,
      getMetatileAt(x: number, y: number): number {
        if (x < 0 || x >= 2 || y < 0 || y >= 2) {
          throw new Error("Metatile lookup out of range.");
        }
        return metatileIds[y * 2 + x];
      },
    } as OverworldMap;
    const cutTileset = new EngineTilesetStub("johto", "day") as unknown as OverworldTilesetLike;
    (cutTileset.metatiles as Array<{ collision: number[] }>)[0x5b].collision = [
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
    ];
    engine.tileset = cutTileset;
    engine.dialogue = null;
    engine.draw = jest.fn();
    engine.get_facing_tile_coords = jest.fn(() => [METATILE_WIDTH, METATILE_WIDTH]);
    engine._refresh_warp_permissions = jest.fn();
    ui.eventQueue.push({ type: "keydown", button: "a", is_press: true });

    await expect(engine.use_hm_from_menu("CUT", pokemon)).resolves.toBe(true);

    expect(metatileIds[3]).toBe(0x3c);
    expect(engine._refresh_warp_permissions).toHaveBeenCalledTimes(1);
  });

  it("confirms Cut through the overworld dialogue manager instead of a private input queue", async () => {
    const gameState = createInitialGameState();
    gameState.sram.options.no_text_scroll = true;
    gameState.sram.badges.johto[1] = true;

    const ui = buildFieldMoveUi();
    const eventManager = new EventManager(gameState);
    const dialogue = new FieldDialogueManager(
      ui,
      gameState,
      { event_manager: eventManager },
      null
    );
    for (const eventName of ["open_text", "show_text", "close_text", "wait_for_input", "prompt_yes_no"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }
    expect(eventManager.hasListener("prompt_yes_no")).toBe(true);

    const metatileIds = [0, 0, 0, 0x5b];
    const overworld = new DialogueDrivenCutOverworld();
    overworld.game_state = gameState;
    overworld.event_manager = eventManager;
    overworld.ui = ui;
    overworld.dialogue = dialogue;
    overworld.draw = jest.fn();
    overworld.map = {
      mapName: "CutDialogueTest",
      width: 2,
      height: 2,
      dataLoader: null,
      metatileIds,
      getMetatileAt(x: number, y: number): number {
        if (x < 0 || x >= 2 || y < 0 || y >= 2) {
          throw new Error("Metatile lookup out of range.");
        }
        return metatileIds[y * 2 + x];
      },
    } as OverworldMap;
    const cutTileset = new EngineTilesetStub("johto", "day") as unknown as OverworldTilesetLike;
    (cutTileset.metatiles as Array<{ collision: number[] }>)[0x5b].collision = [
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
      resolveCollisionValue("CUT_TREE"),
    ];
    overworld.tileset = cutTileset;

    await expect(overworld.handle_cut(1, 1, pokemon)).resolves.toBe(true);

    expect(overworld.promptConfirmed).toBe(true);
    expect(metatileIds[3]).toBe(0x3c);
  });
});

describe("OverworldFieldMoveMixin strength/whirlpool/fly/rock smash parity", () => {
  const addFlyUser = (gameState: ReturnType<typeof createInitialGameState>): void => {
    gameState.sram.party.pokemon = [
      {
        species: { id: "PIDGEOTTO" },
        nickname: "PIDGEOTTO",
        moves: [{ name: "FLY" }],
      } as unknown as Pokemon,
    ];
  };

  const placeOnMap = (gameState: ReturnType<typeof createInitialGameState>, mapConstant: string): void => {
    const metadata = getMapMetadataByConstant(mapConstant);
    if (!metadata) {
      throw new Error(`Missing metadata for ${mapConstant}`);
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;
    gameState.wram.current_map_group = metadata.groupId;
    gameState.wram.current_map_id = metadata.mapId;
  };

  class FlyDestinationTestOverworld extends OverworldFieldMoveMixin {
    public availableFlyDestinations(): Array<{ label: string; landmark: string; spawn: maps.Spawn; default: boolean }> {
      return this._available_fly_destinations();
    }
  }

  it("formats field-move actor names through ASM string buffer tokens", () => {
    class FieldMoveTextOverworld extends OverworldFieldMoveMixin {
      public formatFieldMoveText(label: string, actorName: string | null): string {
        return this._format_field_move_text(label, actorName);
      }

      protected override _resolve_field_move_text(label: string): string {
        if (label === "UseStrengthText") {
          return "<STRING_BUFFER_2> used\nSTRENGTH!";
        }
        return super._resolve_field_move_text(label);
      }
    }

    const overworld = new FieldMoveTextOverworld();
    overworld.game_state = createInitialGameState();

    expect(overworld.formatFieldMoveText("UseStrengthText", "MACHOP")).toBe("MACHOP used\nSTRENGTH!");
  });

  it("sets strength via bike flags and records the acting species", () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      {
        species: { id: "MACHOP" },
        nickname: "MACHOP",
        moves: [{ name: "STRENGTH" }],
      } as unknown as Pokemon,
    ];
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_PLAINBADGE", true);
    overworld.game_state = gameState;

    expect(overworld.handle_strength(0, 0)).toBe(true);
    expect((gameState.wram.wBikeFlags & 0x01) !== 0).toBe(true);
    expect(getBooleanFlag(gameState.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE")).toBe(true);
    expect(gameState.wram.wStrengthSpecies).toBe("MACHOP");
  });

  it("uses Strength from the menu and plays the field-move sound", async () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    const playSound = jest.fn();
    gameState.sram.party.pokemon = [
      {
        species: { id: "MACHOP" },
        nickname: "MACHOP",
        moves: [{ name: "STRENGTH" }],
      } as unknown as Pokemon,
    ];
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_PLAINBADGE", true);
    overworld.game_state = gameState;
    overworld.audio_engine = { playSound } as unknown as AudioEngine;

    await expect(overworld.use_hm_from_menu("STRENGTH", gameState.sram.party.pokemon[0] as Pokemon)).resolves.toBe(true);

    expect(getBooleanFlag(gameState.wram.engine_flags, "ENGINE_STRENGTH_ACTIVE")).toBe(true);
    expect(playSound).toHaveBeenCalledWith("SFX_STRENGTH");
  });

  it("stores whirlpool replacement metadata before removing the whirlpool tile", async () => {
    const overworld = new OverworldFieldMoveMixin();
    const gameState = createInitialGameState();
    const whirlpool = resolveCollisionValue("WHIRLPOOL");
    const metatileIds = [0x07];
    overworld.game_state = gameState;
    overworld.current_map_name = "WhirlpoolTest";
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_GLACIERBADGE", true);
    gameState.sram.party.pokemon = [
      {
        species: { id: "LAPRAS" },
        nickname: "LAPRAS",
        moves: [{ name: "WHIRLPOOL" }],
      } as unknown as Pokemon,
    ];
    overworld.map = {
      mapName: "WhirlpoolTest",
      width: 1,
      height: 1,
      dataLoader: null,
      metatileIds,
      getMetatileAt: () => metatileIds[0],
    } as unknown as OverworldMap;
    overworld.tileset = {
      tilesetName: "johto",
      metatiles: [
        { collision: [0, 0, 0, 0] },
        { collision: [0, 0, 0, 0] },
        { collision: [0, 0, 0, 0] },
        { collision: [0, 0, 0, 0] },
        { collision: [0, 0, 0, 0] },
        { collision: [0, 0, 0, 0] },
        { collision: [0, 0, 0, 0] },
        { collision: [whirlpool, whirlpool, whirlpool, whirlpool] },
      ],
      renderMetatile: () => {},
      renderPriorityMetatile: () => {},
    } as unknown as OverworldTilesetLike;

    await overworld.handle_whirlpool(0, 0);

    expect(gameState.wram.wCutWhirlpoolOverworldBlockAddr).toEqual([0, 0]);
    expect(gameState.wram.wCutWhirlpoolReplacementBlock).toBe(0x36);
    expect(gameState.wram.wCutWhirlpoolAnimationType).toBe(0x07);
    expect(metatileIds[0]).toBe(0x36);
    expect("cleared_whirlpools" in (gameState.wram as Record<string, unknown>)).toBe(false);
  });

  it("does not allow fly from indoor maps", async () => {
    class TestFlyOverworld extends OverworldFieldMoveMixin {
      public _start_fly_from_animation = jest.fn();
      public _queue_fly_to_animation = jest.fn();
      public _select_fly_destination_async = jest.fn(async () => 0);
    }
    const environmentSpy = jest.spyOn(maps, "getMapEnvironment").mockReturnValue("CAVE" as const);

    const overworld = new TestFlyOverworld();
    const gameState = createInitialGameState();
    addFlyUser(gameState);
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_STORMBADGE", true);
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_NEW_BARK", true);
    overworld.game_state = gameState;
    overworld.current_map_name = "TestOverworld";

    await overworld.handle_fly(0, 0);

    expect(overworld._select_fly_destination_async).not.toHaveBeenCalled();
    expect(overworld._start_fly_from_animation).not.toHaveBeenCalled();
    expect(overworld._queue_fly_to_animation).not.toHaveBeenCalled();
    environmentSpy.mockRestore();
  });

  it("keeps Fly destinations on the active ASM region map and preserves the default cursor", () => {
    const overworld = new FlyDestinationTestOverworld();
    const gameState = createInitialGameState();
    overworld.game_state = gameState;

    placeOnMap(gameState, "ECRUTEAK_CITY");
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_NEW_BARK", true);
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_GOLDENROD", true);
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_PALLET", true);
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_INDIGO_PLATEAU", true);

    expect(overworld.availableFlyDestinations()).toEqual([
      { label: "NEW BARK TOWN", landmark: "LANDMARK_NEW_BARK_TOWN", spawn: maps.Spawn.NEW_BARK, default: true },
      { label: "GOLDENROD CITY", landmark: "LANDMARK_GOLDENROD_CITY", spawn: maps.Spawn.GOLDENROD, default: false },
    ]);

    placeOnMap(gameState, "PALLET_TOWN");

    expect(overworld.availableFlyDestinations()).toEqual([
      { label: "PALLET TOWN", landmark: "LANDMARK_PALLET_TOWN", spawn: maps.Spawn.PALLET, default: false },
      { label: "INDIGO PLATEAU", landmark: "LANDMARK_INDIGO_PLATEAU", spawn: maps.Spawn.INDIGO, default: true },
    ]);
  });

  it("uses Johto's Fly map from Kanto until Indigo Plateau has been visited", () => {
    const overworld = new FlyDestinationTestOverworld();
    const gameState = createInitialGameState();
    overworld.game_state = gameState;
    placeOnMap(gameState, "PALLET_TOWN");
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_NEW_BARK", true);
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_PALLET", true);

    expect(overworld.availableFlyDestinations()).toEqual([
      { label: "NEW BARK TOWN", landmark: "LANDMARK_NEW_BARK_TOWN", spawn: maps.Spawn.NEW_BARK, default: true },
    ]);
  });

  it("uses ASM's Silver Cave flypoint flag name", () => {
    const overworld = new FlyDestinationTestOverworld();
    const gameState = createInitialGameState();
    overworld.game_state = gameState;
    placeOnMap(gameState, "NEW_BARK_TOWN");
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_SILVER_CAVE", true);

    expect(overworld.availableFlyDestinations()).toContainEqual({
      label: "SILVER CAVE",
      landmark: "LANDMARK_SILVER_CAVE",
      spawn: maps.Spawn.MT_SILVER,
      default: false,
    });
  });

  it("passes ASM Fly defaults and cancel semantics to the Fly prompt", async () => {
    class PromptCaptureOverworld extends OverworldFieldMoveMixin {
      public promptSettings: unknown = null;
    }
    const overworld = new PromptCaptureOverworld();
    class PromptStub {
      constructor(_ui: unknown, _labels: string[], settings: unknown) {
        overworld.promptSettings = settings;
      }
      async runAsync(): Promise<number> {
        return -1;
      }
    }
    const gameState = createInitialGameState();
    overworld.game_state = gameState;
    overworld.ui = buildFieldMoveUi() as never;
    overworld.fly_prompt_class = PromptStub as never;

    await expect(
      (
        overworld as unknown as {
          _select_fly_destination_async: (labels: string[], initialIndex: number) => Promise<number>;
        }
      )._select_fly_destination_async(["PALLET TOWN", "INDIGO PLATEAU"], 1)
    ).resolves.toBe(-1);

    expect(overworld.promptSettings).toMatchObject({
      title: "FLY TO WHERE?",
      initialIndex: 1,
      cancelResult: -1,
    });
  });

  it("does not throw when fly has an invalid destination index", async () => {
    class TestFlyOverworld extends OverworldFieldMoveMixin {
      public _select_fly_destination_async = jest.fn(async () => -1);
    }
    const overworld = new TestFlyOverworld();
    const gameState = createInitialGameState();
    addFlyUser(gameState);
    placeOnMap(gameState, "NEW_BARK_TOWN");
    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_STORMBADGE", true);
    overworld.game_state = gameState;

    setBooleanFlag(gameState.wram.engine_flags, "ENGINE_FLYPOINT_NEW_BARK", true);
    overworld._select_fly_destination_async = jest.fn(async () => 99);
    await expect(overworld.handle_fly(0, 0)).resolves.toBe(false);
  });

  it("uses facing smashable rock objects and enters rock smash pending state", async () => {
    class TestRockSmashOverworld extends OverworldFieldMoveMixin {
      public get_facing_tile_coords = jest.fn(() => [4, 4] as [number, number]);
      public _npc_occupying_subtile = jest.fn(() => ({
        objectIndex: 5,
        x: 4,
        y: 4,
        event: { spritemovedata: "SPRITEMOVEDATA_SMASHABLE_ROCK" },
      } as unknown as OverworldObject));
      public get_object_by_id = jest.fn((id: string | number) => {
        if (id === "LAST_TALKED" || id === 5) {
          return {
            objectIndex: 5,
            x: 4,
            y: 4,
            event: { spritemovedata: "SPRITEMOVEDATA_SMASHABLE_ROCK" },
          } as unknown as OverworldObject;
        }
        return null;
      });
      public _run_field_move_delay = jest.fn();
    }

    const overworld = new TestRockSmashOverworld();
    const gameState = createInitialGameState();
    gameState.wram.current_map_group = 1;
    gameState.wram.current_map_id = 1;
    gameState.sram.party.pokemon = [
      {
        species: { id: "GEODUDE" },
        nickname: "GEODUDE",
        moves: [{ name: "ROCK_SMASH" }],
      } as unknown as Pokemon,
    ];
    overworld.game_state = gameState;

    await overworld.handle_rock_smash();

    expect(gameState.wram.last_talked).toBe(5);
    expect(gameState.wram.wRockSmashState).toBe(1);
    expect(gameState.wram.wRockSmashStepTimer).toBe(overworld._ROCK_SMASH_BREAK_FRAMES);
    expect(overworld._pending_rock_smash).toEqual(expect.objectContaining({ object_id: 5 }));
    expect(overworld._run_field_move_delay).toHaveBeenCalledWith(overworld._ROCK_SMASH_BREAK_FRAMES);
  });
});

describe("OverworldFieldMoveMixin headbutt", () => {
  class TestHeadbuttOverworld extends OverworldFieldMoveMixin {
    public get_facing_tile_coords = jest.fn(() => [7, 9]);
    public _tile_is_headbutt_tree = jest.fn(() => true);
    public _run_headbutt_animation = jest.fn();
    public _run_field_move_animation_frames_async = jest.fn(async () => undefined);
    public _start_headbutt_battle = jest.fn();

    public currentMapMetadataForTest() {
      return this._current_map_metadata();
    }
  }

  const createHeadbuttMap = (mapName = "HeadbuttTest"): OverworldMap => {
    const metatileIds = [0];
    return {
      mapName,
      width: 1,
      height: 1,
      dataLoader: null,
      metatileIds,
      getMetatileAt(x: number, y: number): number {
        if (x < 0 || x >= 1 || y < 0 || y >= 1) {
          throw new Error("Metatile lookup out of range.");
        }
        return metatileIds[0];
      },
    } as OverworldMap;
  };

  const createHeadbuttTileset = (): OverworldTilesetLike => ({
    tilesetName: "headbutt",
    metatiles: [{ collision: [0, 0, 0, 0] }],
    renderMetatile: () => {},
    renderPriorityMetatile: () => {},
  });

  const buildActor = (): Pokemon =>
    ({
      nickname: "TEST",
      species: { id: "HOOTHOOT" },
    } as Pokemon);

  it("uses facing tile coordinates to score tree encounters", async () => {
    const overworld = new TestHeadbuttOverworld();
    const gameState = createInitialGameState();
    gameState.sram.player_id = 123;
    overworld.map = createHeadbuttMap();
    overworld.tileset = createHeadbuttTileset();
    overworld.game_state = gameState;
    overworld.event_manager = new EventManager(gameState);

    const scoreSpy = jest.spyOn(treeEncounters, "computeTreeScore");
    await overworld.handle_headbutt(buildActor());

    expect(scoreSpy).toHaveBeenCalledWith(7, 9, 123);
    scoreSpy.mockRestore();
  });

  it("starts a wild battle when a real Headbutt tree table returns an encounter", async () => {
    const overworld = new TestHeadbuttOverworld();
    const gameState = createInitialGameState();
    overworld.map = createHeadbuttMap("Route29");
    overworld.tileset = createHeadbuttTileset();
    overworld.game_state = gameState;
    overworld.event_manager = new EventManager(gameState);
    expect(overworld.currentMapMetadataForTest()?.constant).toBe("ROUTE_29");

    const chooseSpy = jest
      .spyOn(treeEncounters, "chooseTreeEncounter")
      .mockReturnValue(["PINECO", 10]);

    try {
      await expect(overworld.handle_headbutt(buildActor())).resolves.toBe(true);
      expect(chooseSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          common: expect.any(Array),
          rare: expect.any(Array),
        }),
        expect.any(Number),
        expect.any(Function)
      );
    } finally {
      chooseSpy.mockRestore();
    }

    expect(overworld._start_headbutt_battle).toHaveBeenCalledWith("PINECO", 10);
    expect(gameState.wram.wHeadbuttState).toBe(0);
  });

  it("returns false when the headbutt prompt is declined", async () => {
    const overworld = new TestHeadbuttOverworld();
    const gameState = createInitialGameState();
    overworld.map = createHeadbuttMap();
    overworld.tileset = createHeadbuttTileset();
    overworld.game_state = gameState;
    overworld.event_manager = new EventManager(gameState);
    overworld._field_move_confirm_callback = jest.fn(() => false);

    const result = await overworld.handle_headbutt(buildActor(), { prompt: true });

    expect(result).toBe(false);
    expect(overworld._run_headbutt_animation).not.toHaveBeenCalled();
    expect(gameState.wram.wHeadbuttState).toBe(0);
  });

  it("ignores repeated Headbutt input while the sequence is already running", async () => {
    const overworld = new TestHeadbuttOverworld();
    const gameState = createInitialGameState();
    gameState.wram.wHeadbuttState = 1;
    (overworld as unknown as { _headbutt_sequence_active: boolean })._headbutt_sequence_active = true;
    overworld.map = createHeadbuttMap();
    overworld.tileset = createHeadbuttTileset();
    overworld.game_state = gameState;
    overworld.event_manager = new EventManager(gameState);

    await expect(overworld.handle_headbutt(buildActor(), { prompt: true })).resolves.toBe(false);

    expect(overworld._run_headbutt_animation).not.toHaveBeenCalled();
    expect(gameState.wram.wHeadbuttState).toBe(1);
  });

  it("recovers stale Headbutt WRAM state left by a crashed sequence", async () => {
    const overworld = new TestHeadbuttOverworld();
    const gameState = createInitialGameState();
    gameState.wram.wHeadbuttState = 1;
    overworld.map = createHeadbuttMap();
    overworld.tileset = createHeadbuttTileset();
    overworld.game_state = gameState;
    overworld.event_manager = new EventManager(gameState);

    await expect(overworld.handle_headbutt(buildActor())).resolves.toBe(true);

    expect(overworld._run_headbutt_animation).toHaveBeenCalled();
    expect(gameState.wram.wHeadbuttState).toBe(0);
  });

  it("confirms Headbutt through the overworld dialogue manager instead of freezing in a private prompt", async () => {
    const gameState = createInitialGameState();
    gameState.sram.options.no_text_scroll = true;
    const ui = buildFieldMoveUi();
    const eventManager = new EventManager(gameState);
    const dialogue = new FieldDialogueManager(
      ui,
      gameState,
      { event_manager: eventManager },
      null
    );
    for (const eventName of ["open_text", "show_text", "close_text", "wait_for_input", "prompt_yes_no"]) {
      eventManager.on(eventName, dialogue.handle_event.bind(dialogue));
    }

    const overworld = new DialogueDrivenHeadbuttOverworld();
    overworld.game_state = gameState;
    overworld.event_manager = eventManager;
    overworld.ui = ui;
    overworld.dialogue = dialogue;
    overworld.draw = jest.fn();
    overworld.map = createHeadbuttMap();
    overworld.tileset = createHeadbuttTileset();

    await expect(overworld.handle_headbutt(buildActor(), { prompt: true })).resolves.toBe(true);

    expect(overworld.promptConfirmed).toBe(true);
    expect(overworld._run_headbutt_animation).toHaveBeenCalled();
    expect(gameState.wram.wHeadbuttState).toBe(0);
  });
});
