import { z } from 'zod';
import { BATTLE_MENU_HEADER } from './_battle-menu';

export const DEFAULT_TILE_SIZE = 8;

const SpritePlacementSchema = z
  .object({
    tile_x: z.number(),
    tile_y: z.number(),
    offset_x: z.number().default(0),
    offset_y: z.number().default(0),
    sprite_type: z.string().default('pokemon_front'),
  })
  .transform((data) => ({
    ...data,
    pixelPosition(tileSize: number, scale: number): [number, number] {
      const step = tileSize * scale;
      return [data.tile_x * step + data.offset_x, data.tile_y * step + data.offset_y];
    },
  }));

export type SpritePlacement = z.infer<typeof SpritePlacementSchema>;

export const BattleTextWindowSchema = z
  .object({
    tile_x: z.number(),
    tile_y: z.number(),
    width_tiles: z.number(),
    height_tiles: z.number(),
  })
  .transform((data) => ({
    ...data,
    pixelOrigin(tileSize: number): [number, number] {
      return [data.tile_x * tileSize, data.tile_y * tileSize];
    },
  }));

export type BattleTextWindow = z.infer<typeof BattleTextWindowSchema>;

const BattleHUDLayoutSchema = z.object({
  origin: z.tuple([z.number(), z.number()]).default([0, 0]),
  hp_fill_position: z.tuple([z.number(), z.number()]),
  name_position: z.tuple([z.number(), z.number()]),
  level_position: z.tuple([z.number(), z.number()]),
  status_position: z.tuple([z.number(), z.number()]).nullable().optional(),
  hp_label_position: z.tuple([z.number(), z.number()]).nullable().optional(),
  hp_value_position: z.tuple([z.number(), z.number()]).nullable().optional(),
  exp_bar_position: z.tuple([z.number(), z.number()]).nullable().optional(),
});

export type BattleHUDLayout = z.infer<typeof BattleHUDLayoutSchema>;

const BattleUILayoutSchema = z.object({
  sprite_scale: z.number().default(1),
  player_sprite: SpritePlacementSchema,
  enemy_sprite: SpritePlacementSchema,
  player_hud: BattleHUDLayoutSchema,
  enemy_hud: BattleHUDLayoutSchema,
  text_box: BattleTextWindowSchema,
  menu_window: BattleTextWindowSchema.nullable().optional(),
  move_selection_window: BattleTextWindowSchema.nullable().optional(),
  move_info_window: BattleTextWindowSchema.nullable().optional(),
  move_menu_origin: z.tuple([z.number(), z.number()]),
  move_menu_row_spacing: z.number().default(1),
  main_menu_origin: z.tuple([z.number(), z.number()]),
  main_menu_column_spacing: z.number().default(7),
  main_menu_row_spacing: z.number().default(2),
});

export type BattleUILayout = z.infer<typeof BattleUILayoutSchema>;

export const BattleUILayoutFactory = {
  fromScaledDefaults(): BattleUILayout {
    const scale = 4;
    return BattleUILayoutSchema.parse({
      sprite_scale: scale,
      player_sprite: {
        tile_x: 2,
        tile_y: 6,
        offset_x: 8,
        sprite_type: 'pokemon_back',
      },
      enemy_sprite: {
        tile_x: 12,
        tile_y: 0,
        offset_x: 8,
        sprite_type: 'pokemon_front',
      },
      player_hud: {
        hp_fill_position: [80 * scale, 24 * scale],
        name_position: [72 * scale, 16 * scale],
        level_position: [128 * scale, 16 * scale],
        status_position: [112 * scale, 24 * scale],
        exp_bar_position: [72 * scale, 40 * scale],
      },
      enemy_hud: {
        hp_fill_position: [32 * scale, 16 * scale],
        name_position: [8 * scale, 0],
        level_position: [48 * scale, 8 * scale],
        status_position: [48 * scale, 8 * scale],
      },
      text_box: {
        tile_x: 0,
        tile_y: 12,
        width_tiles: 20,
        height_tiles: 6,
      },
      menu_window: {
        tile_x: 8,
        tile_y: 12,
        width_tiles: 12,
        height_tiles: 6,
      },
      move_selection_window: {
        tile_x: 4,
        tile_y: 12,
        width_tiles: 16,
        height_tiles: 6,
      },
      move_info_window: {
        tile_x: 0,
        tile_y: 8,
        width_tiles: 11,
        height_tiles: 5,
      },
      move_menu_origin: [6, 13],
      main_menu_origin: [BATTLE_MENU_HEADER.coords.left + 1, BATTLE_MENU_HEADER.coords.top + 1],
      main_menu_column_spacing: BATTLE_MENU_HEADER.spacing,
      main_menu_row_spacing: BATTLE_MENU_HEADER.row_spacing,
    });
  },
  fromAsmDefaults(): BattleUILayout {
    return BattleUILayoutSchema.parse({
      sprite_scale: 1,
      player_sprite: {
        tile_x: 2,
        tile_y: 6,
        sprite_type: 'pokemon_back',
      },
      enemy_sprite: {
        tile_x: 12,
        tile_y: 0,
        sprite_type: 'pokemon_front',
      },
      player_hud: {
        hp_fill_position: [10 * DEFAULT_TILE_SIZE, 9 * DEFAULT_TILE_SIZE],
        name_position: [10 * DEFAULT_TILE_SIZE, 7 * DEFAULT_TILE_SIZE],
        level_position: [14 * DEFAULT_TILE_SIZE, 8 * DEFAULT_TILE_SIZE],
        status_position: [14 * DEFAULT_TILE_SIZE, 8 * DEFAULT_TILE_SIZE],
        exp_bar_position: [10 * DEFAULT_TILE_SIZE, 11 * DEFAULT_TILE_SIZE],
      },
      enemy_hud: {
        hp_fill_position: [2 * DEFAULT_TILE_SIZE, 2 * DEFAULT_TILE_SIZE],
        name_position: [1 * DEFAULT_TILE_SIZE, 0],
        level_position: [6 * DEFAULT_TILE_SIZE, 1 * DEFAULT_TILE_SIZE],
        status_position: [6 * DEFAULT_TILE_SIZE, 1 * DEFAULT_TILE_SIZE],
      },
      text_box: {
        tile_x: 0,
        tile_y: 12,
        width_tiles: 20,
        height_tiles: 6,
      },
      menu_window: {
        tile_x: BATTLE_MENU_HEADER.coords.left,
        tile_y: BATTLE_MENU_HEADER.coords.top,
        width_tiles: BATTLE_MENU_HEADER.coords.width,
        height_tiles: BATTLE_MENU_HEADER.coords.height,
      },
      move_selection_window: {
        tile_x: 4,
        tile_y: 12,
        width_tiles: 16,
        height_tiles: 6,
      },
      move_info_window: {
        tile_x: 0,
        tile_y: 8,
        width_tiles: 11,
        height_tiles: 5,
      },
      move_menu_origin: [6, 13],
      main_menu_origin: [BATTLE_MENU_HEADER.coords.left + 1, BATTLE_MENU_HEADER.coords.top + 1],
      main_menu_column_spacing: BATTLE_MENU_HEADER.spacing,
      main_menu_row_spacing: BATTLE_MENU_HEADER.row_spacing,
    });
  },
};
