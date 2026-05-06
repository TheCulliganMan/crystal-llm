import fs from "fs";
import path from "path";
import { decompress } from "@pokecrystal/core/core/lz";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import type { RGBColor } from "./intro-schemas";
import type { TileIndexMode } from "./tilemap-defaults";

// ASM reference: gfx/intro/*, data/intro/*

type PaletteBank = RGBColor[][];

type TileSurface = InstanceType<typeof gameEngine.Surface>;

export class IntroGraphics {
  private readonly introGfxPath: string;
  private readonly tileSize = 8;

  public readonly palettes: Record<string, PaletteBank>;
  public readonly objPalettes: Record<string, PaletteBank>;
  public readonly tiles: Record<string, TileSurface[]> = {};
  public readonly tilemaps: Record<string, number[]> = {};
  public readonly attrmaps: Record<string, number[]> = {};
  public readonly paletteOverrides: Record<string, Record<number, RGBColor[]>> = {};
  public readonly objPaletteOverrides: Record<string, Record<number, RGBColor[]>> = {};
  private readonly tileCache = new Map<string, TileSurface>();
  private readonly paletteVersions = new Map<string, number>();
  private readonly objPaletteVersions = new Map<string, number>();
  private readonly paletteNameCache = new Map<string, string | null>();

  constructor() {
    this.introGfxPath = getAssetPath("gfx", "intro");
    const [bgPalettes, objPalettes] = this.loadPalettes();
    this.palettes = bgPalettes;
    this.objPalettes = objPalettes;
    this.createTiles();
    this.loadTilemapsAndAttrmaps();
  }

  private loadAsmAsset(filename: string): Uint8Array {
    const candidates: string[] = [];
    if (filename.endsWith(".lz")) {
      candidates.push(path.join(this.introGfxPath, filename.replace(/\.lz$/, "")));
    }
    candidates.push(path.join(this.introGfxPath, filename));

    for (const candidate of candidates) {
      if (!fs.existsSync(candidate)) {
        continue;
      }
      const data = fs.readFileSync(candidate);
      if (candidate.endsWith(".lz")) {
        return decompress(data);
      }
      return data;
    }

    throw new Error(`Could not locate intro asset ${filename}.`);
  }

  private loadPalettes(): [Record<string, PaletteBank>, Record<string, PaletteBank>] {
    const bgPalettes: Record<string, PaletteBank> = {};
    const objPalettes: Record<string, PaletteBank> = {};

    const paletteSources: Record<string, string> = {
      unowns: "unowns.pal",
      background: "background.pal",
      suicune: "suicune.pal",
      crystal_unowns: "crystal_unowns.pal",
      suicune_close: "suicune_close.pal",
      unown_1: "unown_1.pal",
      unown_2: "unown_2.pal",
      fade: "fade.pal",
    };

    for (const [name, filename] of Object.entries(paletteSources)) {
      const [bg, obj] = this.loadPaletteFile(filename);
      if (bg.length) {
        bgPalettes[name] = bg;
      }
      if (obj.length) {
        objPalettes[name] = obj;
      }
    }

    for (const [name, palettes] of Object.entries(bgPalettes)) {
      if (!objPalettes[name]) {
        objPalettes[name] = palettes.map((colors) => [...colors]);
      }
    }

    const pichuWooperPalettes = (objPalettes.background || []).map((colors) => [
      ...colors,
    ]);
    if (pichuWooperPalettes.length) {
      while (pichuWooperPalettes.length < 8) {
        pichuWooperPalettes.push([
          [0, 0, 0],
          [0, 0, 0],
          [0, 0, 0],
          [0, 0, 0],
        ]);
      }
      objPalettes.pichu_wooper = pichuWooperPalettes;
    }

    return [bgPalettes, objPalettes];
  }

  private loadPaletteFile(filename: string): [PaletteBank, PaletteBank] {
    const palFile = path.join(this.introGfxPath, filename);
    const palettes: PaletteBank = [];
    const lines = fs.readFileSync(palFile, "utf-8").split("\n");

    const rgbValues: RGBColor[] = [];
    for (const raw of lines) {
      const line = raw.trim();
      if (!line || line.startsWith(";") || !line.startsWith("RGB")) {
        continue;
      }
      const rgbPart = line.split("RGB ")[1];
      const [r, g, b] = rgbPart.split(",").map((value) => Number.parseInt(value, 10));
      rgbValues.push([gbc5To8(r, "intro palette r"), gbc5To8(g, "intro palette g"), gbc5To8(b, "intro palette b")]);
    }

    for (let i = 0; i < rgbValues.length; i += 4) {
      const palette = rgbValues.slice(i, i + 4);
      if (palette.length === 4) {
        palettes.push(palette as RGBColor[]);
      }
    }

    const bgPalettes = palettes.slice(0, 8);
    const objPalettes = palettes.length >= 16 ? palettes.slice(8, 16) : [];
    return [bgPalettes, objPalettes];
  }

  private loadPngSurface(filename: string): TileSurface | null {
    const pngPath = path.join(this.introGfxPath, filename);
    if (!fs.existsSync(pngPath)) {
      return null;
    }
    const surface = gameEngine.image.loadSync?.(pngPath) ?? null;
    if (!surface) {
      return null;
    }
    if (surface.get_width() % this.tileSize !== 0 || surface.get_height() % this.tileSize !== 0) {
      throw new Error(
        `Intro PNG ${filename} dimensions must align to ${this.tileSize}x${this.tileSize} tiles.`
      );
    }
    return surface;
  }

  private createTiles(): void {
    const gfxMap: Record<string, string> = {
      unowns: "unowns.png",
      background: "background.png",
      suicune_run: "suicune_run.png",
      pulse: "pulse.png",
      crystal_unowns: "crystal_unowns.png",
      pichu_wooper: "pichu_wooper.png",
      suicune_close: "suicune_close.png",
      suicune_jump: "suicune_jump.png",
      suicune_back: "suicune_back.png",
      unown_back: "unown_back.png",
      grass1: "grass1.png",
      grass2: "grass2.png",
      grass3: "grass3.png",
      grass4: "grass4.png",
    };

    for (const [name, pngFilename] of Object.entries(gfxMap)) {
      const surface = this.loadPngSurface(pngFilename);
      if (!surface) {
        throw new Error(`Could not locate intro PNG asset ${pngFilename}.`);
      }
      const tilesWide = surface.get_width() / this.tileSize;
      const tilesHigh = surface.get_height() / this.tileSize;
      const tiles = this.surfaceToTiles(surface, tilesWide, tilesHigh);
      this.tiles[name] = tiles;
    }

    if (this.tiles.unowns) {
      const unownBank0 = [...this.tiles.unowns];
      this.tiles.unowns_bank0 = this.padTiles(unownBank0, 256);
      this.tiles.unowns_bank1 = this.padTiles(unownBank0, 256);
    }

    if (this.tiles.crystal_unowns) {
      const bank0 = this.padTiles(this.tiles.crystal_unowns, 256);
      this.tiles.crystal_unowns_bank0 = bank0;
      this.tiles.crystal_unowns_bank1 = this.expandTiles(
        this.tiles.crystal_unowns,
        256
      );
    }
  }

  private loadTilemapsAndAttrmaps(): void {
    const mapFiles: Record<string, string> = {
      unown_a: "unown_a",
      unown_hi: "unown_hi",
      unowns: "unowns",
      background: "background",
      suicune_jump: "suicune_jump",
      suicune_close: "suicune_close",
      suicune_back: "suicune_back",
      crystal_unowns: "crystal_unowns",
    };

    for (const [name, base] of Object.entries(mapFiles)) {
      this.tilemaps[name] = Array.from(this.loadAsmAsset(`${base}.tilemap`));
      this.attrmaps[name] = Array.from(this.loadAsmAsset(`${base}.attrmap`));
    }
  }

  private surfaceToTiles(
    surface: TileSurface,
    tilesWide: number,
    tilesHigh: number
  ): TileSurface[] {
    const tiles: TileSurface[] = [];
    for (let y = 0; y < tilesHigh; y++) {
      for (let x = 0; x < tilesWide; x++) {
        const rect = new gameEngine.Rect(
          x * this.tileSize,
          y * this.tileSize,
          this.tileSize,
          this.tileSize
        );
        tiles.push(surface.subsurface(rect));
      }
    }
    return tiles;
  }

  private padTiles(tiles: TileSurface[], size: number): TileSurface[] {
    if (tiles.length >= size) {
      return tiles.slice(0, size);
    }
    const padded = [...tiles];
    const blank = new gameEngine.Surface(this.tileSize, this.tileSize);
    blank.fill([0, 0, 0, 0]);
    while (padded.length < size) {
      padded.push(blank.copy());
    }
    return padded;
  }

  private expandTiles(tiles: TileSurface[], size: number): TileSurface[] {
    if (!tiles.length) {
      return this.padTiles([], size);
    }
    const expanded: TileSurface[] = [];
    while (expanded.length < size) {
      const remaining = size - expanded.length;
      expanded.push(...tiles.slice(0, remaining));
    }
    return expanded;
  }

  getTile(
    graphicName: string,
    tileIndex: number,
    paletteIndex = 0,
    attr: number | null = null,
    transparentZero = false,
    tileShift = 0,
    paletteNameOverride?: string,
    tileIndexMode: TileIndexMode = "offset"
  ): TileSurface | null {
    const bank = attr !== null && (attr & 0x08) ? 1 : 0;
    const baseName = graphicName.split("_bank")[0];

    const candidateNames: string[] = [];
    if (bank === 1) {
      candidateNames.push(`${graphicName}_bank1`, `${baseName}_bank1`);
    } else {
      candidateNames.push(`${graphicName}_bank0`, `${baseName}_bank0`);
    }
    candidateNames.push(graphicName, baseName);

    let tiles: TileSurface[] | undefined;
    for (const name of candidateNames) {
      if (this.tiles[name]) {
        tiles = this.tiles[name];
        break;
      }
    }
    if (!tiles || !tiles.length) {
      return null;
    }

    let resolvedIndex = tileIndex & 0xff;
    if (tileShift) {
      if (tileIndexMode === "signed") {
        resolvedIndex = (resolvedIndex + ((-tileShift) & 0xff)) & 0xff;
      } else if (resolvedIndex >= tileShift) {
        resolvedIndex = (resolvedIndex - tileShift) & 0xff;
      }
    }
    if (resolvedIndex >= tiles.length) {
      resolvedIndex %= tiles.length;
    }
    const paletteName = paletteNameOverride ?? this.resolvePaletteName(baseName);
    const paletteVersion = paletteName
      ? this.getPaletteVersion(paletteName, transparentZero)
      : 0;
    const baseTile = tiles[resolvedIndex];
    const cacheKey = this.buildTileCacheKey(
      baseName,
      graphicName,
      resolvedIndex,
      paletteIndex,
      attr,
      transparentZero,
      tileShift,
      paletteNameOverride,
      tileIndexMode,
      paletteVersion
    );
    const cached = this.tileCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    const paletteSource = transparentZero ? this.objPalettes : this.palettes;
    const overridesSource = transparentZero
      ? this.objPaletteOverrides
      : this.paletteOverrides;

    let palette: RGBColor[] | undefined;
    if (paletteName) {
      const overrides = overridesSource[paletteName] ?? {};
      if (paletteIndex in overrides) {
        palette = overrides[paletteIndex];
      } else {
        const entries = paletteSource[paletteName];
        if (entries && paletteIndex < entries.length) {
          palette = entries[paletteIndex];
        } else if (
          transparentZero &&
          this.palettes[paletteName] &&
          paletteIndex < this.palettes[paletteName].length
        ) {
          palette = this.palettes[paletteName][paletteIndex];
        }
      }
    }

    const xflip = attr !== null && (attr & 0x20) !== 0;
    const yflip = attr !== null && (attr & 0x40) !== 0;
    if (!palette && !xflip && !yflip) {
      return baseTile;
    }

    let tile = baseTile;
    if (palette) {
      tile = this.recolorTile(baseTile, palette, transparentZero);
    }
    if (xflip || yflip) {
      tile = gameEngine.transform.flip(tile, xflip, yflip);
    }
    this.tileCache.set(cacheKey, tile);
    return tile;
  }

  getResolvedPaletteName(graphicName: string): string | null {
    const baseName = graphicName.split("_bank")[0];
    return this.resolvePaletteName(baseName);
  }

  private resolvePaletteName(baseName: string): string | null {
    if (this.paletteNameCache.has(baseName)) {
      return this.paletteNameCache.get(baseName) ?? null;
    }
    let resolved: string | null = null;
    if (this.palettes[baseName]) {
      resolved = baseName;
    } else if (baseName.startsWith("unown")) {
      resolved = "unowns";
    } else if (baseName.startsWith("suicune")) {
      resolved = "suicune";
    } else if (baseName.startsWith("pulse")) {
      resolved = "unowns";
    } else if (baseName === "pichu_wooper") {
      resolved = "pichu_wooper";
    } else if (baseName.startsWith("pichu") || baseName.startsWith("wooper")) {
      resolved = "background";
    } else if (baseName.startsWith("grass")) {
      resolved = "background";
    } else if (baseName.startsWith("crystal_unowns")) {
      resolved = "crystal_unowns";
    } else if (baseName.startsWith("background")) {
      resolved = "background";
    }
    this.paletteNameCache.set(baseName, resolved);
    return resolved;
  }

  private recolorTile(
    tile: TileSurface,
    palette: RGBColor[],
    transparentZero: boolean
  ): TileSurface {
    const imageData = tile.getImageData();
    const output = new gameEngine.Surface(this.tileSize, this.tileSize);
    const outCtx = output.getContext();
    if (!outCtx) {
      throw new Error("Failed to recolor intro tile: missing 2D context.");
    }
    const data = imageData.data;
    for (let index = 0; index < data.length; index += 4) {
      const r = data[index] ?? 0;
      const g = data[index + 1] ?? 0;
      const b = data[index + 2] ?? 0;
      const a = data[index + 3] ?? 0;
      let color: RGBColor = [r, g, b];
      let alpha = a;
      if (r === 255 && g === 255 && b === 255) {
        color = palette[0];
        alpha = transparentZero ? 0 : a;
      } else if (r === 170 && g === 170 && b === 170) {
        color = palette[1];
      } else if (r === 85 && g === 85 && b === 85) {
        color = palette[2];
      } else if (r === 0 && g === 0 && b === 0) {
        color = palette[3];
      }
      data[index] = color[0];
      data[index + 1] = color[1];
      data[index + 2] = color[2];
      data[index + 3] = alpha;
    }
    outCtx.putImageData(imageData, 0, 0);
    return output;
  }

  private buildTileCacheKey(
    baseName: string,
    graphicName: string,
    resolvedIndex: number,
    paletteIndex: number,
    attr: number | null,
    transparentZero: boolean,
    tileShift: number,
    paletteNameOverride?: string,
    tileIndexMode: TileIndexMode = "offset",
    paletteVersion = 0
  ): string {
    return [
      baseName,
      graphicName,
      resolvedIndex,
      paletteIndex,
      attr ?? -1,
      transparentZero ? 1 : 0,
      tileShift,
      paletteNameOverride ?? "",
      tileIndexMode,
      paletteVersion,
    ].join("|");
  }

  private getPaletteVersion(paletteName: string, transparentZero: boolean): number {
    const versions = transparentZero ? this.objPaletteVersions : this.paletteVersions;
    return versions.get(paletteName) ?? 0;
  }

  private bumpPaletteVersion(paletteName: string, transparentZero: boolean): void {
    const versions = transparentZero ? this.objPaletteVersions : this.paletteVersions;
    versions.set(paletteName, (versions.get(paletteName) ?? 0) + 1);
  }

  setPaletteOverride(
    paletteName: string,
    paletteIndex: number,
    colors: RGBColor[]
  ): void {
    const overrides = (this.paletteOverrides[paletteName] ||= {});
    overrides[paletteIndex] = colors;
    this.bumpPaletteVersion(paletteName, false);
  }

  setObjPaletteOverride(
    paletteName: string,
    paletteIndex: number,
    colors: RGBColor[]
  ): void {
    const overrides = (this.objPaletteOverrides[paletteName] ||= {});
    overrides[paletteIndex] = colors;
    this.bumpPaletteVersion(paletteName, true);
  }

  clearPaletteOverrides(paletteName?: string): void {
    if (!paletteName) {
      Object.keys(this.paletteOverrides).forEach((key) => this.bumpPaletteVersion(key, false));
      Object.keys(this.objPaletteOverrides).forEach((key) => this.bumpPaletteVersion(key, true));
      Object.keys(this.paletteOverrides).forEach((key) => delete this.paletteOverrides[key]);
      Object.keys(this.objPaletteOverrides).forEach((key) => delete this.objPaletteOverrides[key]);
      return;
    }
    if (this.paletteOverrides[paletteName]) {
      this.bumpPaletteVersion(paletteName, false);
    }
    if (this.objPaletteOverrides[paletteName]) {
      this.bumpPaletteVersion(paletteName, true);
    }
    delete this.paletteOverrides[paletteName];
    delete this.objPaletteOverrides[paletteName];
  }

  validateTilemaps(): string[] {
    const issues: string[] = [];
    const combinedTiles: TileSurface[] = [];
    Object.values(this.tiles).forEach((tileList) => combinedTiles.push(...tileList));

    for (const [mapName, tilemap] of Object.entries(this.tilemaps)) {
      tilemap.forEach((tileIdx, i) => {
        if (tileIdx >= combinedTiles.length) {
          issues.push(
            `Map ${mapName}: tile index ${tileIdx} at position ${i} out of range`
          );
          return;
        }
        const tile = combinedTiles[tileIdx];
        const [r, g, b, a] = tile.get_at([0, 0]);
        if (a === 0) {
          let isEmpty = true;
          for (let y = 0; y < tile.get_height(); y++) {
            for (let x = 0; x < tile.get_width(); x++) {
              if (tile.get_at([x, y])[3] !== 0) {
                isEmpty = false;
                break;
              }
            }
            if (!isEmpty) {
              break;
            }
          }
          if (isEmpty) {
            issues.push(
              `Map '${mapName}' tile ${i}: index ${tileIdx} returns empty placeholder`
            );
          }
        }
      });
    }

    return issues;
  }
}
