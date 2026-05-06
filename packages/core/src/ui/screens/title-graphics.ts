import { decompress } from "@pokecrystal/core/core/lz";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import fs from "node:fs/promises";

type Palette = [number, number, number][];
type Tile = number[][]; // Grayscale tile
type ColoredTile = [number, number, number, number][][]; // RGBA tile

async function fetchAsset(url: string): Promise<Uint8Array> {
  if (typeof window === "undefined" && !/^https?:\/\//i.test(url) && !url.startsWith("/assets/")) {
    return new Uint8Array(await fs.readFile(url));
  }
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Failed to fetch asset: ${url}`);
  }
  const arrayBuffer = await response.arrayBuffer();
  return new Uint8Array(arrayBuffer);
}

export class TitleGraphics {
  private readonly titleGfxPath: string;
  private readonly tileSize = 8;
  private bgPalettes: Palette[] = [];
  private objPalettes: Palette[] = [];
  private readonly paletteKinds: Record<string, "bg" | "obj">;
  private readonly tiles: Record<string, Tile[]> = {};

  private constructor() {
    this.titleGfxPath = getAssetPath("gfx", "title");
    this.paletteKinds = {
      logo: "bg",
      suicune: "bg",
      crystal: "obj",
    };
  }

  static async create(): Promise<TitleGraphics> {
    const graphics = new TitleGraphics();
    await graphics.init();
    return graphics;
  }

  private async init(): Promise<void> {
    [this.bgPalettes, this.objPalettes] = await this._loadPalettes();
    await this._createTiles();
  }

  private async _loadPalettes(): Promise<[Palette[], Palette[]]> {
    const palettes: Palette[] = [];
    const palFile = `${this.titleGfxPath}/title.pal`;
    const text =
      typeof window === "undefined" && !/^https?:\/\//i.test(palFile) && !palFile.startsWith("/assets/")
        ? await fs.readFile(palFile, "utf8")
        : await (await fetch(palFile)).text();
    const lines = text.split("\n");

    const rgbEntries: [number, number, number][] = [];
    for (const raw of lines) {
      const line = raw.trim();
      if (!line || line.startsWith(";") || !line.startsWith("RGB")) {
        continue;
      }

      const components = line.split("RGB ")[1].split(",");
      const [r, g, b] = components.map(Number);

      rgbEntries.push([gbc5To8(r), gbc5To8(g), gbc5To8(b)]);
    }

    for (let i = 0; i < rgbEntries.length; i += 4) {
      const palette = rgbEntries.slice(i, i + 4);
      if (palette.length === 4) {
        palettes.push(palette as Palette);
      }
    }

    if (palettes.length !== 16) {
      throw new Error(
        `Expected 16 palettes in ${palFile}, found ${palettes.length}.`
      );
    }

    const bgPalettes = palettes.slice(0, 8);
    const objPalettes = palettes.slice(8);
    return [bgPalettes, objPalettes];
  }

  private async _loadAssetBytes(filePath: string): Promise<Uint8Array> {
    const data = await fetchAsset(filePath);
    if (filePath.endsWith(".lz")) {
      return decompress(data);
    }
    return data;
  }

  private async _loadGraphicSurface(
    baseName: string,
    tilesWide: number,
    tilesHigh: number
  ): Promise<Tile> {
    const expectedBytes = tilesWide * tilesHigh * 16;
    const preferredAssets = [
      `${this.titleGfxPath}/${baseName}.2bpp`,
      `${this.titleGfxPath}/${baseName}.2bpp.lz`,
    ];
    for (const assetPath of preferredAssets) {
      try {
        const data = await this._loadAssetBytes(assetPath);
        if (data.length % 16 !== 0) {
          continue;
        }
        if (data.length === expectedBytes) {
          return this._2bppToGrayscale(data, tilesWide, tilesHigh);
        }
        if (data.length > expectedBytes) {
          return this._2bppToGrayscale(data.subarray(0, expectedBytes), tilesWide, tilesHigh);
        }
        const padded = new Uint8Array(expectedBytes);
        padded.set(data);
        return this._2bppToGrayscale(padded, tilesWide, tilesHigh);
      } catch (error) {
        // Asset not found, try the next one
      }
    }
    throw new Error(`Could not locate title asset for ${baseName}.`);
  }

  private _2bppToGrayscale(
    data: Uint8Array,
    tilesWide: number,
    tilesHigh: number
  ): Tile {
    const widthPx = tilesWide * this.tileSize;
    const heightPx = tilesHigh * this.tileSize;
    const surface: number[][] = Array.from({ length: heightPx }, () =>
      Array(widthPx).fill(0)
    );

    for (let tileY = 0; tileY < tilesHigh; tileY++) {
      for (let tileX = 0; tileX < tilesWide; tileX++) {
        const tileIndex = tileY * tilesWide + tileX;
        const base = tileIndex * 16;
        for (let row = 0; row < 8; row++) {
          const idx1 = base + row * 2;
          const idx2 = idx1 + 1;
          const byte1 = data[idx1] ?? 0;
          const byte2 = data[idx2] ?? 0;
          for (let col = 0; col < 8; col++) {
            const bit = 7 - col;
            const colour = ((byte1 >> bit) & 1) | (((byte2 >> bit) & 1) << 1);
            const gray = [255, 170, 85, 0][colour];
            surface[tileY * 8 + row][tileX * 8 + col] = gray;
          }
        }
      }
    }
    return surface;
  }

  private _surfaceToTiles(
    surface: Tile,
    tilesWide: number,
    tilesHigh: number
  ): Tile[] {
    const tiles: Tile[] = [];
    for (let y = 0; y < tilesHigh; y++) {
      for (let x = 0; x < tilesWide; x++) {
        const tile: Tile = [];
        for (let row = 0; row < this.tileSize; row++) {
          tile.push(
            surface[y * this.tileSize + row].slice(
              x * this.tileSize,
              x * this.tileSize + this.tileSize
            )
          );
        }
        tiles.push(tile);
      }
    }
    return tiles;
  }

  private _surfaceToSpritePairTiles(
    surface: Tile,
    tilesWide: number,
    tilesHigh: number
  ): Tile[] {
    const tiles = this._surfaceToTiles(surface, tilesWide, tilesHigh);
    if (tilesHigh % 2 !== 0) {
      throw new Error("8x16 sprite graphics must have an even tile height.");
    }

    const orderedTiles: Tile[] = [];
    for (let spriteRow = 0; spriteRow < tilesHigh / 2; spriteRow++) {
      for (let tileX = 0; tileX < tilesWide; tileX++) {
        const topTileIndex = spriteRow * 2 * tilesWide + tileX;
        orderedTiles.push(tiles[topTileIndex], tiles[topTileIndex + tilesWide]);
      }
    }
    return orderedTiles;
  }

  private _blankTile(): Tile {
    return Array.from({ length: this.tileSize }, () =>
      Array(this.tileSize).fill(0)
    );
  }

  private _buildTileBank(
    tiles: Tile[],
    startTile: number,
    bankSize = 256
  ): Tile[] {
    const bank: Tile[] = Array.from({ length: bankSize }, () =>
      this._blankTile()
    );
    for (let i = 0; i < tiles.length; i++) {
      const tileId = (startTile + i) & 0xff;
      bank[tileId] = tiles[i];
    }
    return bank;
  }

  private async _createTiles(): Promise<void> {
    const suicuneSurface = await this._loadGraphicSurface("suicune", 16, 16);
    const suicuneTiles = this._surfaceToTiles(suicuneSurface, 16, 16);
    this.tiles["suicune"] = this._buildTileBank(suicuneTiles, 0x80);

    const logoSurface = await this._loadGraphicSurface("logo", 20, 8);
    const logoTiles = this._surfaceToTiles(logoSurface, 20, 8);
    this.tiles["logo"] = this._buildTileBank(logoTiles, 0x80);

    const crystalSurface = await this._loadGraphicSurface("crystal", 6, 10);
    const crystalTiles = this._surfaceToSpritePairTiles(crystalSurface, 6, 10);
    this.tiles["crystal"] = this._buildTileBank(crystalTiles, 0x00);
  }

  private _applyPalette(
    tile: Tile,
    palette: Palette,
    transparentZero = false
  ): ColoredTile {
    const coloredTile: [number, number, number, number][][] = [];
    for (let y = 0; y < this.tileSize; y++) {
      const row: [number, number, number, number][] = [];
      for (let x = 0; x < this.tileSize; x++) {
        const gray = tile[y][x];
        let idx = 0;
        if (gray > 191) {
            idx = 0;
        } else if (gray > 127) {
            idx = 1;
        } else if (gray > 63) {
            idx = 2;
        } else {
            idx = 3;
        }
        if (idx < palette.length) {
            const [r, g, b] = palette[idx];
            const alpha = transparentZero && idx === 0 ? 0 : 255;
            row.push([r, g, b, alpha]);
        }
      }
      coloredTile.push(row);
    }
    return coloredTile;
  }

  public getTile(
    graphicName: string,
    tileIndex: number,
    paletteIndex = 0
  ): ColoredTile {
    const tileId = tileIndex & 0xff;
    const tiles = this.tiles[graphicName];
    if (!tiles) {
      throw new Error(`Unknown graphic bank '${graphicName}'.`);
    }
    if (tileId >= tiles.length) {
      throw new Error(
        `Tile index ${tileIndex} out of range for '${graphicName}'.`
      );
    }

    const paletteKind = this.paletteKinds[graphicName];
    if (!paletteKind) {
      throw new Error(`No palette kind registered for '${graphicName}'.`);
    }

    const palettes = paletteKind === "obj" ? this.objPalettes : this.bgPalettes;
    if (paletteIndex < 0 || paletteIndex >= palettes.length) {
      throw new Error(
        `Palette index ${paletteIndex} invalid for ${paletteKind.toUpperCase()} palettes.`
      );
    }

    const tile = tiles[tileId];
    const palette = palettes[paletteIndex];
    return this._applyPalette(tile, palette, paletteKind === "obj");
  }

  public getTileIndices(graphicName: string, tileIndex: number): number[][] {
    const tileId = tileIndex & 0xff;
    const tiles = this.tiles[graphicName];
    if (!tiles) {
      throw new Error(`Unknown graphic bank '${graphicName}'.`);
    }
    if (tileId >= tiles.length) {
      throw new Error(
        `Tile index ${tileIndex} out of range for '${graphicName}'.`
      );
    }

    const tile = tiles[tileId];
    const indices: number[][] = [];
    for (let y = 0; y < this.tileSize; y++) {
      const row: number[] = [];
      for (let x = 0; x < this.tileSize; x++) {
        const gray = tile[y][x];
        let idx = 0;
        if (gray > 191) {
            idx = 0;
        } else if (gray > 127) {
            idx = 1;
        } else if (gray > 63) {
            idx = 2;
        } else {
            idx = 3;
        }
        row.push(idx);
      }
      indices.push(row);
    }
    return indices;
  }
}
