import fs from "fs";
import path from "path";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { Surface } from "./surface";
import { decode2bppTiles } from "./2bpp";
import { assemble_place_graphic_surface } from "./graphics/place-graphic";

const TILE_SIZE = 8;
const BACKPIC_TILES_WIDE = 6;
const BACKPIC_TILES_HIGH = 6;
const BACKPIC_TILE_COUNT = BACKPIC_TILES_WIDE * BACKPIC_TILES_HIGH;
const WHITE: [number, number, number] = [255, 255, 255];
const BLACK: [number, number, number] = [0, 0, 0];
const DMG_PALETTE: [number, number, number][] = [
  [255, 255, 255],
  [170, 170, 170],
  [85, 85, 85],
  [0, 0, 0],
];

const backpicCache = new Map<string, Surface>();
const paletteCache = new Map<string, [number, number, number][]>();

const candidatePaths = (stem: string): string[] => [
  getAssetPath("gfx", "player", `${stem}.2bpp`),
  getAssetPath("gfx", "battle", `${stem}.2bpp`),
];

const assemble_backpic = (tiles: Surface[]): Surface => {
  if (tiles.length < BACKPIC_TILE_COUNT) {
    throw new Error(`Player backpic requires ${BACKPIC_TILE_COUNT} tiles, got ${tiles.length}`);
  }
  const surface = assemble_place_graphic_surface(tiles, BACKPIC_TILES_WIDE, BACKPIC_TILES_HIGH);
  const [r, g, b] = surface.get_at([0, 0]);
  surface.set_colorkey([r, g, b]);
  return surface;
};

const load_trainer_palette = (stem: string): [number, number, number][] => {
  const cached = paletteCache.get(stem);
  if (cached) {
    return cached;
  }
  const palettePath = getAssetPath("gfx", "trainers", `${stem}.gbcpal`);
  if (!fs.existsSync(palettePath)) {
    throw new Error(`Missing trainer palette: ${palettePath}`);
  }
  const data = fs.readFileSync(path.resolve(palettePath));
  if (data.length < 8) {
    throw new Error(`Trainer palette ${palettePath} must be at least 8 bytes, got ${data.length}.`);
  }
  const colours: [number, number, number][] = [];
  for (let offset = 0; offset < 8; offset += 2) {
    colours.push(gbcWordToRgb(data.readUInt16LE(offset)));
  }
  paletteCache.set(stem, colours);
  return colours;
};

const resolve_palette_id = (sprite_id: string, gender?: PlayerGender | null): string => {
  if (gender === PlayerGender.FEMALE) {
    return "falkner";
  }
  if (gender === PlayerGender.MALE) {
    return "cal";
  }
  const normalized = sprite_id.trim().toLowerCase();
  if (normalized.includes("kris")) {
    return "falkner";
  }
  return "cal";
};

const build_backpic_palette = (palette_id: string): [number, number, number][] => {
  const colours = load_trainer_palette(palette_id);
  if (colours.length < 3) {
    throw new Error(`Trainer palette ${palette_id} must provide at least 3 colours.`);
  }
  // ASM: engine/gfx/color.asm::GetPlayerOrMonPalettePointer uses PlayerPalette/KrisPalette middle colours.
  return [WHITE, colours[1], colours[2], BLACK];
};

// ASM mapping: engine/battle/core.asm::GetTrainerBackpic + CopyBackpic (6x6 backpic tiles).
// Palette mapping: engine/gfx/color.asm::GetBattlemonBackpicPalettePointer.
export const load_player_backpic_surface = (
  sprite_id: string,
  options?: { player_gender?: PlayerGender | null }
): Surface => {
  const normalized = sprite_id.trim().toLowerCase();
  if (!normalized) {
    throw new Error("Player backpic id must be non-empty.");
  }
  const paletteId = resolve_palette_id(normalized, options?.player_gender ?? null);
  const cacheKey = `${normalized}:${paletteId}`;
  const cached = backpicCache.get(cacheKey);
  if (cached) {
    return cached;
  }
  const paths = candidatePaths(normalized);
  const tilePath = paths.find((candidate) => fs.existsSync(candidate));
  if (!tilePath) {
    throw new Error(`Missing player backpic asset: ${paths.join(" or ")}`);
  }
  const data = fs.readFileSync(path.resolve(tilePath));
  const palette = paletteId ? build_backpic_palette(paletteId) : DMG_PALETTE;
  const tiles = decode2bppTiles(data, palette);
  const surface = assemble_backpic(tiles);
  backpicCache.set(cacheKey, surface);
  return surface;
};
