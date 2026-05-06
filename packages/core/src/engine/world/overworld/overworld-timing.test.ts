import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { applySpawn, Spawn } from "@pokecrystal/core/engine/world/maps";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { OverworldTilesetLike, RenderMetatileOptions } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";

const buildUiStub = () => {
  const font = { render_text: jest.fn() };
  return {
    tileSize: 8,
    tile_size: 8,
    screen: { get_size: () => [160, 144] as [number, number] },
    font,
    loadSprite: jest.fn(),
    drawWindow: jest.fn(),
    draw_window: jest.fn(),
    draw_text_box: jest.fn(),
    drawTextBox: jest.fn(),
    _getPokemonFrameSurface: jest.fn(() => null),
    get_context_palette: jest.fn(() => [[0, 0, 0], [31, 31, 31]]),
    _record_window_region: jest.fn(),
  };
};

class TestTileset implements OverworldTilesetLike {
  public tilesetName: string;
  public readonly metatiles: OverworldTilesetLike["metatiles"] = [];

  constructor(tilesetName: string, _timeOfDay: string) {
    this.tilesetName = tilesetName;
  }

  public renderMetatile(
    _metatileId: number,
    _target: InstanceType<typeof gameEngine.Surface>,
    _x: number,
    _y: number,
    _options?: RenderMetatileOptions,
  ): void {
    // no-op for tests
  }

  public renderPriorityMetatile(
    _metatileId: number,
    _target: InstanceType<typeof gameEngine.Surface>,
    _x: number,
    _y: number,
  ): void {
    // no-op for tests
  }
}

const buildTilesetStub = (): OverworldTilesetLike => new TestTileset("TEST", "day");

class TestOverworldEngine extends OverworldEngine {
  protected _initialise_npc_object(
    _npc: OverworldObject,
    _options: {
      previous?: OverworldObject | null;
      reload_standing?: boolean;
      reload_walking?: boolean;
    } = {},
  ): void {
    // Skip sprite loading that would require real assets.
  }
}

describe("OverworldEngine movement timing", () => {
  it("exposes WALK_FRAMES on instances for scripted movement cadence", () => {
    const gameState = createInitialGameState();
    applySpawn(gameState, Spawn.NEW_BARK);
    const dataLoader = new DataLoader();
    dataLoader.Tileset = TestTileset;

    const originalLoadSync = (gameEngine.image as {
      loadSync?: (path: string) => InstanceType<typeof gameEngine.Surface> | null;
    }).loadSync;
    const loadSyncStubSurface = new gameEngine.Surface(16, 16);
    (gameEngine.image as {
      loadSync?: (path: string) => InstanceType<typeof gameEngine.Surface> | null;
    }).loadSync = () => loadSyncStubSurface;

    try {
      const grassSpy = jest
        .spyOn(TestOverworldEngine.prototype as unknown as { _detect_map_grass: () => void }, "_detect_map_grass")
        .mockImplementation(() => {});
      const overworld = new TestOverworldEngine(
        gameState,
        dataLoader,
        new EventManager(gameState),
        buildTilesetStub(),
        new AudioEngine({ muted: true }),
        buildUiStub(),
      );

      expect(overworld.WALK_FRAMES).toBe(OverworldEngine.WALK_FRAMES);
      grassSpy.mockRestore();
    } finally {
      (gameEngine.image as {
        loadSync?: (path: string) => InstanceType<typeof gameEngine.Surface> | null;
      }).loadSync = originalLoadSync;
    }
  });
});
