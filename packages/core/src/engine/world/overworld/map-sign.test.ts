import { createInitialGameState } from "@pokecrystal/core/core/state";
import { MapNameSignController } from "@pokecrystal/core/engine/world/overworld/map-sign";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import type { Surface } from "@pokecrystal/core/ui/surface";

class TilesetStub {
  public tilesetName: string;
  public metatiles: Array<{ collision: number[]; tiles: number[][] }>;

  constructor(tilesetName: string = "placeholder", _timeOfDay: string = "day") {
    this.tilesetName = tilesetName || "placeholder";
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

const buildTextUi = (): TextUI & {
  tile_size?: number;
  font: BaseFontRenderer;
} => {
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
  return ui;
};

describe("MapNameSignController", () => {
  it("loads tileset data asynchronously", async () => {
    const controller = await MapNameSignController.create(createInitialGameState());
    const tiles = controller.getTiles();
    expect(Array.isArray(tiles)).toBe(true);
    expect(tiles).toHaveLength(14);
  });

  it("does not pre-mark the map name sign as shown during overworld construction", () => {
    const gameState = createInitialGameState();
    const metadata = getMapMetadataByConstant("NEW_BARK_TOWN");
    if (!metadata) {
      throw new Error("Missing NEW_BARK_TOWN metadata.");
    }
    gameState.wram.wMapGroup = metadata.groupId;
    gameState.wram.wMapNumber = metadata.mapId;
    gameState.wram.wMapNameSignFlags = 0;
    gameState.wram.event_flags.EVENT_INITIALIZED_EVENTS = true;

    const dataLoader = new DataLoader() as DataLoader & { Tileset?: typeof TilesetStub };
    dataLoader.Tileset = TilesetStub;
    const ui = buildTextUi();

    new OverworldEngine(
      gameState,
      dataLoader,
      new EventManager(gameState),
      new TilesetStub("johto", "day") as never,
      new AudioEngine({ masterVolume: 0, muted: true }),
      ui,
    );

    expect(gameState.wram.wMapNameSignFlags).toBe(0);
  });
});
