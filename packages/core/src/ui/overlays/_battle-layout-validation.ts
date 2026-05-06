import { BattleUILayoutFactory } from './_battle-layout';
import type { BattleTextWindow, BattleUILayout } from './_battle-layout';

const ASM_LAYOUT = BattleUILayoutFactory.fromAsmDefaults();

const tileCoords = (position: [number, number], tileSize: number): [number, number] => [
  Math.floor(position[0] / tileSize),
  Math.floor(position[1] / tileSize),
];

const assertWindow = (
  actual: BattleTextWindow | null | undefined,
  reference: BattleTextWindow | null | undefined,
  name: string,
): void => {
  if (!actual || !reference) {
    throw new Error(`Battle UI ${name} window differs from ASM defaults`);
  }
  if (
    actual.tile_x !== reference.tile_x ||
    actual.tile_y !== reference.tile_y ||
    actual.width_tiles !== reference.width_tiles ||
    actual.height_tiles !== reference.height_tiles
  ) {
    throw new Error(`Battle UI ${name} window differs from ASM defaults`);
  }
};

const assertTileCoords = (
  actual: [number, number] | null | undefined,
  reference: [number, number] | null | undefined,
  name: string,
  tileSize: number,
): void => {
  if (!actual || !reference) {
    throw new Error(`Battle UI ${name} coordinates must be defined`);
  }
  const actualCoords = tileCoords(actual, tileSize);
  const referenceCoords = tileCoords(reference, tileSize);
  if (actualCoords[0] !== referenceCoords[0] || actualCoords[1] !== referenceCoords[1]) {
    throw new Error(
      `Battle UI ${name} tile coords ${actualCoords} do not match ASM expected ${referenceCoords}`,
    );
  }
};

export const validateBattleLayout = (layout: BattleUILayout, tileSize: number): void => {
  if (layout.sprite_scale !== 1) {
    return;
  }

  assertWindow(layout.text_box, ASM_LAYOUT.text_box, 'text box');
  assertWindow(layout.menu_window ?? null, ASM_LAYOUT.menu_window ?? null, 'main menu');
  assertWindow(
    layout.move_selection_window ?? null,
    ASM_LAYOUT.move_selection_window ?? null,
    'move selection window',
  );
  assertWindow(
    layout.move_info_window ?? null,
    ASM_LAYOUT.move_info_window ?? null,
    'move info window',
  );
  if (
    layout.move_menu_origin[0] !== ASM_LAYOUT.move_menu_origin[0] ||
    layout.move_menu_origin[1] !== ASM_LAYOUT.move_menu_origin[1]
  ) {
    throw new Error('Battle move menu origin deviates from ASM coordinates');
  }
  if (layout.move_menu_row_spacing !== ASM_LAYOUT.move_menu_row_spacing) {
    throw new Error('Battle move menu row spacing deviates from ASM constants');
  }
  if (
    layout.main_menu_origin[0] !== ASM_LAYOUT.main_menu_origin[0] ||
    layout.main_menu_origin[1] !== ASM_LAYOUT.main_menu_origin[1]
  ) {
    throw new Error('Battle main menu origin deviates from ASM constants');
  }
  if (layout.main_menu_column_spacing !== ASM_LAYOUT.main_menu_column_spacing) {
    throw new Error('Battle main menu column spacing deviates from ASM constants');
  }
  if (layout.main_menu_row_spacing !== ASM_LAYOUT.main_menu_row_spacing) {
    throw new Error('Battle main menu row spacing deviates from ASM constants');
  }

  const player = layout.player_hud;
  const referencePlayer = ASM_LAYOUT.player_hud;
  assertTileCoords(
    player.hp_fill_position,
    referencePlayer.hp_fill_position,
    'player HP area',
    tileSize,
  );
  assertTileCoords(player.name_position, referencePlayer.name_position, 'player name', tileSize);
  assertTileCoords(
    player.level_position,
    referencePlayer.level_position,
    'player level',
    tileSize,
  );
  assertTileCoords(
    player.status_position ?? null,
    referencePlayer.status_position ?? null,
    'player status',
    tileSize,
  );
  assertTileCoords(
    player.exp_bar_position ?? null,
    referencePlayer.exp_bar_position ?? null,
    'player EXP bar',
    tileSize,
  );

  const enemy = layout.enemy_hud;
  const referenceEnemy = ASM_LAYOUT.enemy_hud;
  assertTileCoords(enemy.hp_fill_position, referenceEnemy.hp_fill_position, 'enemy HP area', tileSize);
  assertTileCoords(enemy.name_position, referenceEnemy.name_position, 'enemy name', tileSize);
  assertTileCoords(enemy.level_position, referenceEnemy.level_position, 'enemy level', tileSize);
  assertTileCoords(
    enemy.status_position ?? null,
    referenceEnemy.status_position ?? null,
    'enemy status',
    tileSize,
  );
};
