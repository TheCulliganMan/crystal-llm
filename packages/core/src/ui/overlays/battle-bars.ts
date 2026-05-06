import { Pokemon } from '../../core/models';
import { calculateExperience } from '../../engine/experience';
import type { Surface } from '../surface';
import {
  BattleBackgroundTilemap,
  PAL_EXP_FILL,
  PAL_HUD,
  PAL_HP_GREEN,
  PAL_HP_RED,
  PAL_HP_YELLOW,
} from './_battle-background';
import { DEFAULT_TILE_SIZE } from './_battle-layout';
import { gbc5To8 } from '../../core/gbc-colors';

const gb = (value: number): number => gbc5To8(value);
const palette_colour = (r: number, g: number, b: number): [number, number, number] => [
  gb(r),
  gb(g),
  gb(b),
];

export const HP_GREEN = palette_colour(0, 23, 0);
export const HP_YELLOW = palette_colour(31, 21, 0);
export const HP_RED = palette_colour(31, 0, 0);

export const HP_BAR_LENGTH_TILES = 6;
export const HP_BAR_LENGTH_PX = HP_BAR_LENGTH_TILES * DEFAULT_TILE_SIZE;
export const EXP_BAR_TILE_COUNT = 8;
export const EXP_BAR_LENGTH_TILES = EXP_BAR_TILE_COUNT;
export const EXP_BAR_LENGTH_PX = EXP_BAR_TILE_COUNT * DEFAULT_TILE_SIZE;

const HP_LABEL_TILES: [number, number] = [0x60, 0x61];
const HP_TEMPLATE_TILE = 0x62;
const HP_FULL_TILE = 0x6a;
const HP_END_TILES: [number, number] = [0x6b, 0x6c];
const EXP_EMPTY_TILE = 0x62;
const EXP_FULL_TILE = 0x6a;
const EXP_PARTIAL_BASE = 0x54;
const HP_PARTIAL_TILES = new Set(
  Array.from({ length: HP_FULL_TILE - HP_TEMPLATE_TILE - 1 }, (_, offset) =>
    HP_TEMPLATE_TILE + offset + 1
  )
);

export type BattleBarTiles = {
  hp_label: [number, number];
  template: number;
  full: number;
  ends: [number, number];
};

export type ExpBarTiles = {
  empty: number;
  full: number;
};

type FontTiles = Record<number, Surface>;

const ensure_tiles_exist = (fontTiles: FontTiles, tileIds: Iterable<number>, label: string): void => {
  const missing = Array.from(tileIds).filter((tileId) => !(tileId in fontTiles));
  if (missing.length) {
    throw new Error(`${label} tiles missing from font export: ${missing.join(', ')}`);
  }
};

export const build_hp_tiles = (fontTiles: FontTiles): BattleBarTiles => {
  const required = new Set<number>([
    ...HP_LABEL_TILES,
    HP_TEMPLATE_TILE,
    HP_FULL_TILE,
    ...HP_END_TILES,
    ...Array.from(HP_PARTIAL_TILES),
  ]);
  ensure_tiles_exist(fontTiles, required, 'HP bar');
  return {
    hp_label: HP_LABEL_TILES,
    template: HP_TEMPLATE_TILE,
    full: HP_FULL_TILE,
    ends: HP_END_TILES,
  };
};

export const build_exp_tiles = (fontTiles: FontTiles): ExpBarTiles => {
  ensure_tiles_exist(fontTiles, [EXP_EMPTY_TILE, EXP_FULL_TILE], 'EXP bar');
  return { empty: EXP_EMPTY_TILE, full: EXP_FULL_TILE };
};

export const draw_hp_bar = (
  tilemap: BattleBackgroundTilemap,
  tile_x: number,
  tile_y: number,
  current_hp: number,
  max_hp: number,
  tiles: BattleBarTiles,
  options: { is_player: boolean; palette_override?: number | null; pixel_override?: number | null }
): void => {
  tilemap.set_tile(tile_x, tile_y, tiles.hp_label[0], PAL_HUD);
  tilemap.set_tile(tile_x + 1, tile_y, tiles.hp_label[1], PAL_HUD);

  const innerStart = tile_x + 2;
  const pixels = options.pixel_override ?? hp_pixel_length(current_hp, max_hp);
  const palette =
    options.palette_override ?? select_hp_palette(current_hp, max_hp);
  const sequence = hp_fill_sequence(pixels, tiles);
  sequence.forEach((tileId, offset) => {
    tilemap.set_tile(innerStart + offset, tile_y, tileId, palette);
  });

  const endTile = tiles.ends[options.is_player ? 1 : 0];
  tilemap.set_tile(innerStart + HP_BAR_LENGTH_TILES, tile_y, endTile, palette);
};

export const draw_exp_bar = (
  tilemap: BattleBackgroundTilemap,
  tile_x: number,
  tile_y: number,
  pokemon: Pokemon,
  tiles: ExpBarTiles
): void => {
  // ASM mapping: PlaceExpBar writes from right-to-left, and ExpBarGFX partial
  // tiles ($55-$5b) are right-anchored.
  // Ref: pokecrystal_disassembly/engine/battle/core.asm:7919
  const fillUnits = exp_fill_units(pokemon);
  const fullTiles = Math.min(EXP_BAR_TILE_COUNT, Math.floor(fillUnits / DEFAULT_TILE_SIZE));
  const remainder = fillUnits % DEFAULT_TILE_SIZE;

  for (let index = 0; index < EXP_BAR_TILE_COUNT; index += 1) {
    const column = tile_x + EXP_BAR_TILE_COUNT - 1 - index;
    if (index < fullTiles) {
      tilemap.set_tile(column, tile_y, tiles.full, PAL_EXP_FILL);
      continue;
    }
    if (index === fullTiles && remainder > 0) {
      const partialId = EXP_PARTIAL_BASE + remainder;
      tilemap.set_tile(column, tile_y, partialId, PAL_EXP_FILL);
      continue;
    }
    tilemap.set_tile(column, tile_y, EXP_EMPTY_TILE, PAL_EXP_FILL);
  }
};

const hp_pixel_length = (current_hp: number, max_hp: number): number => {
  return pixel_length(hp_ratio(current_hp, max_hp), HP_BAR_LENGTH_PX);
};

const pixel_length = (ratio: number, totalPixels: number): number => {
  let pixels = Math.floor(totalPixels * Math.max(0, Math.min(1, ratio)));
  if (ratio > 0 && pixels === 0) {
    pixels = 1;
  }
  return Math.min(totalPixels, Math.max(0, pixels));
};

const hp_fill_sequence = (pixels: number, tiles: BattleBarTiles): number[] => {
  const fullTiles = Math.floor(pixels / DEFAULT_TILE_SIZE);
  const remainder = pixels % DEFAULT_TILE_SIZE;
  const sequence: number[] = [];
  for (let idx = 0; idx < HP_BAR_LENGTH_TILES; idx += 1) {
    if (idx < fullTiles) {
      sequence.push(tiles.full);
    } else if (idx === fullTiles && remainder > 0) {
      sequence.push(tiles.template + remainder);
    } else {
      sequence.push(tiles.template);
    }
  }
  return sequence;
};

export const select_hp_palette = (current_hp: number, max_hp: number): number => {
  const pixels = hp_pixel_length(current_hp, max_hp);
  if (pixels > (HP_BAR_LENGTH_PX * 50) / 100) {
    return PAL_HP_GREEN;
  }
  if (pixels >= (HP_BAR_LENGTH_PX * 21) / 100) {
    return PAL_HP_YELLOW;
  }
  return PAL_HP_RED;
};

export const compute_hp_pixels = (current_hp: number, max_hp: number): number => {
  return hp_pixel_length(current_hp, max_hp);
};

const hp_ratio = (current_hp: number, max_hp: number): number => {
  if (max_hp <= 0) {
    return 0;
  }
  return current_hp / max_hp;
};

const exp_fill_units = (pokemon: Pokemon): number => {
  // ASM mapping: CalcExpBar computes remaining units to next level, then
  // converts to filled units (64 - remaining).
  // Ref: pokecrystal_disassembly/engine/battle/core.asm:7820
  const growth = pokemon.species.growth_rate;
  if (!growth) {
    return 0;
  }
  if (pokemon.level >= 100) {
    return 0;
  }
  const level = Math.max(1, Math.min(99, pokemon.level));
  const currentLevelExp = Math.max(0, calculateExperience(growth, level));
  const nextLevelExp = Math.max(0, calculateExperience(growth, Math.min(100, level + 1)));
  const span = Math.max(1, nextLevelExp - currentLevelExp);
  const cappedExp = Math.max(currentLevelExp, Math.min(pokemon.experience, nextLevelExp));
  const remaining = Math.max(0, nextLevelExp - cappedExp);
  const unitsRemaining = Math.max(
    0,
    Math.min(64, Math.floor((remaining * EXP_BAR_LENGTH_PX) / span))
  );
  return EXP_BAR_LENGTH_PX - unitsRemaining;
};
