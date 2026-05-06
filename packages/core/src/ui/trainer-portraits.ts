import fs from "fs";
import path from "path";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { Surface } from "./surface";
import { gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { assemble_place_graphic_surface } from "./graphics/place-graphic";

const TRAINER_GRAPHICS_DIR = getAssetPath("gfx", "trainers");
const TILE_SIZE = 8;
const PORTRAIT_TILES_WIDE = 7;
const PORTRAIT_TILES_HIGH = 7;
const PORTRAIT_TILE_COUNT = PORTRAIT_TILES_WIDE * PORTRAIT_TILES_HIGH;

type Palette = [number, number, number][];

const portraitCache = new Map<string, Surface>();

const decode2bppTiles = (data: Buffer): number[][] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: number[][] = [];
  const tileCount = data.length / 16;
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex += 1) {
    const base = tileIndex * 16;
    const pixels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        pixels.push(idx);
      }
    }
    tiles.push(pixels);
  }
  return tiles;
};

const loadGbcpal = (stem: string): Palette => {
  const palettePath = path.join(TRAINER_GRAPHICS_DIR, `${stem}.gbcpal`);
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing trainer palette: ${palettePath}`);
  }
  const data = fs.readFileSync(palettePath);
  if (data.length !== 8) {
    throw new Error(`Trainer palette ${palettePath} must be exactly 8 bytes (4 colours), got ${data.length}.`);
  }
  const colours: Palette = [];
  for (let offset = 0; offset < 8; offset += 2) {
    colours.push(gbcWordToRgb(data.readUInt16LE(offset)));
  }
  return colours;
};

const renderTile = (pixels: number[], palette: Palette): Surface => {
  const surface = new Surface(TILE_SIZE, TILE_SIZE);
  for (let row = 0; row < TILE_SIZE; row += 1) {
    for (let col = 0; col < TILE_SIZE; col += 1) {
      const idx = pixels[row * TILE_SIZE + col] ?? 0;
      const colour = palette[idx] ?? palette[0];
      surface.setAt(col, row, [colour[0], colour[1], colour[2], 255]);
    }
  }
  return surface;
};

const assemblePortrait = (tiles: Surface[]): Surface => {
  if (tiles.length !== PORTRAIT_TILE_COUNT) {
    throw new Error(`Trainer portraits require ${PORTRAIT_TILE_COUNT} tiles, got ${tiles.length}`);
  }
  return assemble_place_graphic_surface(tiles, PORTRAIT_TILES_WIDE, PORTRAIT_TILES_HIGH);
};

export const load_trainer_portrait_surface = (trainer_id: string): Surface => {
  const normalizedId = trainer_id.toLowerCase();
  const cached = portraitCache.get(normalizedId);
  if (cached) {
    return cached;
  }
  const tilePath = path.join(TRAINER_GRAPHICS_DIR, `${normalizedId}.2bpp`);
  if (!fs.existsSync(tilePath)) {
    throw new Error(`Missing trainer portrait asset: ${tilePath}`);
  }
  const tiles = decode2bppTiles(fs.readFileSync(tilePath));
  const palette = loadGbcpal(normalizedId);
  const tileSurfaces = tiles.map((tile) => renderTile(tile, palette)).slice(0, PORTRAIT_TILE_COUNT);
  const surface = assemblePortrait(tileSurfaces);
  portraitCache.set(normalizedId, surface);
  return surface;
};
