import fs from "fs";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { gameEngine, Rect, Surface } from "@pokecrystal/core/ui/game-engine";

const SCREEN_WIDTH_TILES = 20;
const SCREEN_HEIGHT_TILES = 18;
const TILESET_COLUMNS = 16;
const TRAIN_TILEMAP_ROWS = 4;

const magnetTrainAssetPath = (...parts: string[]): string =>
  joinPath(getDisassemblyRoot(), "gfx", "overworld", ...parts);
const trainStationTilesetPath = (): string =>
  joinPath(getDisassemblyRoot(), "gfx", "tilesets", "train_station.png");

const loadAssetBytes = async (assetPath: string): Promise<Uint8Array> => {
  if (typeof window === "undefined" && !/^https?:\/\//i.test(assetPath)) {
    return fs.promises.readFile(assetPath);
  }
  const response = await fetch(assetPath);
  if (!response.ok) {
    throw new Error(`Failed to load magnet train asset: ${assetPath}`);
  }
  return new Uint8Array(await response.arrayBuffer());
};

const requireTilemapLength = (data: Uint8Array, expected: number, label: string): void => {
  if (data.length !== expected) {
    throw new Error(`Magnet train ${label} tilemap must be ${expected} bytes, got ${data.length}.`);
  }
};

const isPromiseLike = <T>(value: T | Promise<T>): value is Promise<T> =>
  typeof (value as Promise<T>).then === "function";

export class MagnetTrainGraphics {
  private constructor(
    private readonly tileset: Surface,
    private readonly bgTilemap: Uint8Array,
    private readonly trainTilemap: Uint8Array
  ) {}

  static createSync(): MagnetTrainGraphics {
    const tilesetPath = trainStationTilesetPath();
    const tileset = gameEngine.image.loadSync?.(tilesetPath) ?? null;
    if (!tileset) {
      throw new Error(`Unable to load magnet train tileset: ${tilesetPath}`);
    }
    const bgTilemap = fs.readFileSync(magnetTrainAssetPath("magnet_train_bg.tilemap"));
    const trainTilemap = fs.readFileSync(magnetTrainAssetPath("magnet_train_fg.tilemap"));
    return MagnetTrainGraphics.fromAssets(tileset, bgTilemap, trainTilemap);
  }

  static async create(): Promise<MagnetTrainGraphics> {
    const tilesetResult = gameEngine.image.load(trainStationTilesetPath());
    const tileset = isPromiseLike(tilesetResult) ? await tilesetResult : tilesetResult;
    const [bgTilemap, trainTilemap] = await Promise.all([
      loadAssetBytes(magnetTrainAssetPath("magnet_train_bg.tilemap")),
      loadAssetBytes(magnetTrainAssetPath("magnet_train_fg.tilemap")),
    ]);
    return MagnetTrainGraphics.fromAssets(tileset, bgTilemap, trainTilemap);
  }

  private static fromAssets(
    tileset: Surface,
    bgTilemap: Uint8Array,
    trainTilemap: Uint8Array
  ): MagnetTrainGraphics {
    requireTilemapLength(bgTilemap, SCREEN_HEIGHT_TILES * 2, "background");
    requireTilemapLength(trainTilemap, SCREEN_WIDTH_TILES * TRAIN_TILEMAP_ROWS, "foreground");
    return new MagnetTrainGraphics(tileset, bgTilemap, trainTilemap);
  }

  buildBaseSurface(): Surface {
    const surface = new gameEngine.Surface(SCREEN_WIDTH_TILES * TILE_SIZE, SCREEN_HEIGHT_TILES * TILE_SIZE);
    surface.fill([0, 0, 0, 255]);

    for (let tileY = 0; tileY < SCREEN_HEIGHT_TILES; tileY += 1) {
      const leftTile = this.bgTilemap[tileY * 2];
      const rightTile = this.bgTilemap[tileY * 2 + 1];
      for (let tileX = 0; tileX < SCREEN_WIDTH_TILES; tileX += 2) {
        this.blitTile(surface, leftTile, tileX, tileY);
        this.blitTile(surface, rightTile, tileX + 1, tileY);
      }
    }

    for (let row = 0; row < TRAIN_TILEMAP_ROWS; row += 1) {
      for (let col = 0; col < SCREEN_WIDTH_TILES; col += 1) {
        this.blitTile(surface, this.trainTilemap[row * SCREEN_WIDTH_TILES + col], col, 6 + row);
      }
    }
    return surface;
  }

  private blitTile(surface: Surface, tileId: number | undefined, tileX: number, tileY: number): void {
    if (tileId === undefined) {
      throw new Error("Magnet train tilemap is truncated.");
    }
    const sourceX = (tileId % TILESET_COLUMNS) * TILE_SIZE;
    const sourceY = Math.floor(tileId / TILESET_COLUMNS) * TILE_SIZE;
    surface.blit(
      this.tileset,
      [tileX * TILE_SIZE, tileY * TILE_SIZE],
      new Rect(sourceX, sourceY, TILE_SIZE, TILE_SIZE)
    );
  }
}
