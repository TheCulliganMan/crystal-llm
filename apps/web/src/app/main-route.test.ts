jest.mock("@pokecrystal/core/ui/overlays/battle-ui", () => ({
  create_battle_ui: jest.fn(() => ({ active: false })),
  set_audio_engine: jest.fn(),
}));

import { Game } from "./game";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";

type TilesetShape = {
  tilesetName: string;
  metatiles: Array<{ collision: number[] }>;
  renderMetatile(): void;
  renderPriorityMetatile(): void;
};

type TilesetConstructor = new (tilesetName?: string) => TilesetShape;

type TestGlobalThis = {
  fetch?: typeof globalThis.fetch | undefined;
  createImageBitmap?: typeof globalThis.createImageBitmap | undefined;
  Tileset?: TilesetConstructor;
};

const testGlobal = globalThis as TestGlobalThis;

class TilesetStub implements TilesetShape {
  public tilesetName: string;
  public metatiles: Array<{ collision: number[] }>;

  constructor(tilesetName: string = "placeholder") {
    this.tilesetName = tilesetName || "placeholder";
    this.metatiles = Array.from({ length: 256 }, () => ({ collision: [0, 0, 0, 0] }));
  }

  renderMetatile() {
    // No-op for headless test coverage.
  }

  renderPriorityMetatile() {
    // No-op for headless test coverage.
  }
}

describe("Main route boot", () => {
  it("creates the game without throwing route initialization errors", async () => {
    const ui = new TextUI(160, 144, 1, null, false, 0) as TextUI & {
      tile_size?: number;
      font: BaseFontRenderer;
    };
    const fontTiles: Record<number, Surface> = {};
    for (let i = 0; i < 256; i += 1) {
      fontTiles[i] = new gameEngine.Surface(8, 8) as unknown as Surface;
    }
    ui.tile_size = 8;
    ui.font.font_tiles = fontTiles;
    const noopRender: (..._args: Parameters<NonNullable<BaseFontRenderer["renderText"]>>) => void = () => {};
    ui.font.render_text = noopRender;
    ui.font.renderText = noopRender;
    const originalFetch = testGlobal.fetch;
    const originalCreateImageBitmap = testGlobal.createImageBitmap;
    const originalTileset = testGlobal.Tileset;
    const originalImageLoad = gameEngine.image.load;
    const originalInitAssets = OverworldEngine.prototype.init_assets;
    testGlobal.fetch = undefined;
    testGlobal.createImageBitmap = undefined;
    testGlobal.Tileset = TilesetStub;
    gameEngine.image.load = async () => new gameEngine.Surface(16, 16);
    OverworldEngine.prototype.init_assets = async () => {};
    let game: Game | null = null;
    try {
      game = await Game.create(ui);
    } finally {
      testGlobal.fetch = originalFetch;
      testGlobal.createImageBitmap = originalCreateImageBitmap;
      testGlobal.Tileset = originalTileset;
      gameEngine.image.load = originalImageLoad;
      OverworldEngine.prototype.init_assets = originalInitAssets;
    }

    expect(game?.getOverworld().current_map_name).toBeTruthy();
  });
});
