import { moveDisplayName } from '@pokecrystal/assets/content/move-names';
import { LearnedMove, Pokemon } from '../../core/models';
import { PokemonType } from '../../core/enums';
import { StatusCondition } from '../../core/enums';
import * as battle_bars from './battle-bars';
import * as battle_dialogue from './battle-dialogue';
import {
  HP_PALETTES,
  PAL_BG_ENEMY_HP,
  PAL_BG_PLAYER_HP,
  PAL_MENU,
  PAL_TEXT_WINDOW,
  BattleBackgroundTilemap,
  retint_tileset_palette,
} from './_battle-background';
import { BattleTextWindowSchema, type BattleTextWindow } from './_battle-layout';
import {
  normalise_window_lines,
  selected_move_entry,
  status_token,
  type_display_name,
  write_level_text,
} from './_battle-hud-helpers';
import {
  BATTLE_MENU_HEADER,
  BUG_CATCHING_MENU_HEADER,
  SAFARI_MENU_HEADER,
  BattleMenu,
  BattleMenuHeader,
  tile_coords_for_option,
} from './_battle-menu';
import { loadMoveMetadata } from './battle-experience';
import { invalidate_scene_cache } from './battle-scene';
import { START_TOP_OFFSET } from './trainer-entrance';
import { _CHAR_MAP, _CLEAR_TILE, _SPACE_TILE } from '../tilemap-surface';
import { Surface } from '../surface';
import { HPBarAnimationState, STATICMENU_CURSOR_TILE, BattleUIState, MoveLearningPhase } from './battle-ui-state';
import { formatMoveName } from './battle-move-name';
import { draw_battle_party_menu } from './_battle-party-menu';

const MENU_CURSOR_VISIBLE_FRAMES = 16;
const MENU_CURSOR_FRAMES = [STATICMENU_CURSOR_TILE, _CHAR_MAP['\u25b7']];
const CAUGHT_ICON_TILE = 0x5d;
const CAUGHT_ICON_COORDS: [number, number] = [1, 1];

export const window_interior = (window: BattleTextWindow): [number, number, number, number] => {
  const innerX = window.tile_x + 1;
  const innerY = window.tile_y + 1;
  const width = Math.max(1, window.width_tiles - 2);
  const height = Math.max(1, window.height_tiles - 2);
  return [innerX, innerY, width, height];
};

export const window_attr = (state: BattleUIState, x: number, y: number): number => {
  const tilemap = state.tilemap;
  const clampedX = Math.min(Math.max(x, 0), tilemap.width - 1);
  const clampedY = Math.min(Math.max(y, 0), tilemap.height - 1);
  return tilemap.attributes[clampedY][clampedX];
};

const menu_cursor_tile = (state: BattleUIState): number => {
  const frame = Math.floor(state.animation_clock.frame / MENU_CURSOR_VISIBLE_FRAMES) % MENU_CURSOR_FRAMES.length;
  return MENU_CURSOR_FRAMES[frame];
};

const maybe_draw_caught_icon = (
  state: BattleUIState,
  pokemon: Pokemon,
  trainer_battle: boolean
): void => {
  if (trainer_battle) {
    return;
  }
  if (!pokemon) {
    throw new Error('Enemy HUD cannot render without an active Pokemon');
  }
  const speciesId = (pokemon.species as { int_id?: number }).int_id;
  if (typeof speciesId !== 'number') {
    throw new Error('Enemy species id must be an int for caught icon rendering');
  }
  const pokedexCaught = (state.game_state.sram as { pokedex_caught?: Set<number> }).pokedex_caught;
  if (!pokedexCaught) {
    throw new Error('Game state SRAM missing pokedex_caught for caught icon rendering');
  }
  if (pokedexCaught.has(speciesId)) {
    state.tilemap.set_tile(CAUGHT_ICON_COORDS[0], CAUGHT_ICON_COORDS[1], CAUGHT_ICON_TILE);
  }
};

export const clear_window_region = (
  state: BattleUIState,
  x: number,
  y: number,
  width: number,
  height: number
): number => {
  const attr = window_attr(state, x, y);
  if (width > 0 && height > 0) {
    state.tilemap.fill_rect(x, y, width, height, { tile: _CLEAR_TILE, attr });
  }
  return attr;
};

export const fill_window_area = (
  state: BattleUIState,
  window: BattleTextWindow,
  options: { attr: number }
): void => {
  const innerWidth = Math.max(0, window.width_tiles - 2);
  const innerHeight = Math.max(0, window.height_tiles - 2);
  if (innerWidth <= 0 || innerHeight <= 0) {
    return;
  }
  state.tilemap.fill_rect(
    window.tile_x + 1,
    window.tile_y + 1,
    innerWidth,
    innerHeight,
    { tile: _CLEAR_TILE, attr: options.attr }
  );
};

export const hide_menu_window = (state: BattleUIState, window?: BattleTextWindow | null): void => {
  if (!window) {
    return;
  }
  if (window.width_tiles > 2 && window.height_tiles > 2) {
    state.tilemap.fill_rect(
      window.tile_x + 1,
      window.tile_y + 1,
      window.width_tiles - 2,
      window.height_tiles - 2,
      { tile: _SPACE_TILE, attr: 0 }
    );
  }
};

export const draw_menu_window = (state: BattleUIState, window?: BattleTextWindow | null): void => {
  if (!window) {
    return;
  }
  state.tilemap.draw_window(
    window.tile_x,
    window.tile_y,
    window.width_tiles,
    window.height_tiles,
    { attr: PAL_MENU }
  );
};

export const render_move_windows = (state: BattleUIState): void => {
  fill_window_area(state, state.layout.text_box, { attr: PAL_TEXT_WINDOW });
  hide_menu_window(state, menu_window_for_header(current_menu_header(state)));
  const windows = [state.layout.move_selection_window, state.layout.move_info_window];
  for (const window of windows) {
    if (!window) {
      continue;
    }
    state.tilemap.draw_window(
      window.tile_x,
      window.tile_y,
      window.width_tiles,
      window.height_tiles,
      { attr: PAL_MENU }
    );
  }
};

export const write_window_text = (
  state: BattleUIState,
  window: BattleTextWindow,
  text: string | Iterable<string>,
  options?: { uppercase?: boolean }
): [number, number, number, number] => {
  let [innerX, innerY, width, height] = window_interior(window);
  clear_window_region(state, innerX, innerY, width, height);
  const lines = normalise_window_lines(text);
  const uppercase = options?.uppercase ?? true;
  lines.slice(0, height).forEach((line, idx) => {
    state.tilemap.write_text(innerX, innerY + idx, line, {
      max_length: width,
      uppercase,
      space_tile: _CLEAR_TILE,
    });
  });
  return [innerX, innerY, width, height];
};

export const wrap_prompt_text = (prompt: string, width: number, height: number): string[] => {
  if (width <= 0 || height <= 0) {
    return [];
  }
  const wrapped: string[] = [];
  const sourceLines = prompt ? prompt.split('\n') : [''];
  for (const rawLine of sourceLines) {
    if (!rawLine) {
      wrapped.push('');
    } else {
      const parts = wrap_line(rawLine, width);
      wrapped.push(...(parts.length ? parts : ['']));
    }
    if (wrapped.length >= height) {
      break;
    }
  }
  return wrapped.slice(0, height);
};

const wrap_line = (rawLine: string, width: number): string[] => {
  if (width <= 0) {
    return [];
  }
  if (rawLine.length <= width) {
    return [rawLine];
  }
  const words = rawLine.trim().split(/\s+/).filter(Boolean);
  if (!words.length) {
    return [''];
  }
  const lines: string[] = [];
  let current = '';
  for (const word of words) {
    if (!current) {
      if (word.length <= width) {
        current = word;
        continue;
      }
      let remaining = word;
      while (remaining.length > width) {
        lines.push(remaining.slice(0, width));
        remaining = remaining.slice(width);
      }
      current = remaining;
      continue;
    }
    if (current.length + 1 + word.length <= width) {
      current = `${current} ${word}`;
      continue;
    }
    if (word.length > width) {
      const fill = width - current.length - 1;
      let remaining = word;
      if (fill > 0) {
        current = `${current} ${remaining.slice(0, fill)}`;
        lines.push(current);
        remaining = remaining.slice(fill);
      } else {
        lines.push(current);
      }
      while (remaining.length > width) {
        lines.push(remaining.slice(0, width));
        remaining = remaining.slice(width);
      }
      current = remaining;
      continue;
    }
    lines.push(current);
    current = word;
  }
  if (current) {
    lines.push(current);
  }
  return lines;
};

export const tile_coords = (position: [number, number], tileSize: number): [number, number] => {
  return [Math.floor(position[0] / tileSize), Math.floor(position[1] / tileSize)];
};

const hide_window_register = (state: BattleUIState): void => {
  state.hardware.set_window(0, START_TOP_OFFSET);
};

const sync_window_register = (state: BattleUIState, window: BattleTextWindow): void => {
  const tileSize = state.ui.tile_size ?? 8;
  const wy = Math.max(0, window.tile_y * tileSize);
  state.hardware.set_window(0, wy);
};

export const update_hp_palette = (
  state: BattleUIState,
  side: 'player' | 'enemy',
  current_hp: number,
  max_hp: number
): number => {
  const paletteVariant = battle_bars.select_hp_palette(current_hp, max_hp);
  const paletteSlot = side === 'player' ? PAL_BG_PLAYER_HP : PAL_BG_ENEMY_HP;
  const previous = state.hp_palettes[side];
  state.hp_palettes[side] = paletteVariant;
  if ('bgp' in state.palette_registers) {
    return paletteSlot;
  }
  if (previous === undefined || previous !== paletteVariant) {
    const palette = HP_PALETTES[paletteVariant];
    retint_tileset_palette(state.tileset as Record<number, Record<number, Surface>>, state.base_tiles as Record<number, Surface>, paletteSlot, palette);
    invalidate_scene_cache();
  }
  return paletteSlot;
};

export const apply_hp_attr = (
  tilemap: BattleBackgroundTilemap,
  tile_x: number,
  tile_y: number,
  palette_slot: number
): void => {
  const length = 2 + battle_bars.HP_BAR_LENGTH_TILES + 1;
  tilemap.fill_attr_rect(tile_x, tile_y, length, 1, { attr: palette_slot });
};

export const draw_enemy_hud = (
  state: BattleUIState,
  pokemon: Pokemon,
  trainer_battle: boolean = false
): void => {
  maybe_draw_caught_icon(state, pokemon, trainer_battle);
  const hud = state.layout.enemy_hud;
  const tileSize: number = state.ui.tile_size ?? state.ui.tileSize ?? 8;
  const [nameX, nameY] = tile_coords(hud.name_position, tileSize);
  state.tilemap.clear_box(nameX, nameY, 10, 1);
  state.tilemap.write_text(nameX, nameY, pokemon.nickname, { max_length: 10, uppercase: false });
  const status = status_token(pokemon.status as StatusCondition | null | undefined);
  if (status && hud.status_position) {
    const [sx, sy] = tile_coords(hud.status_position, tileSize);
    state.tilemap.clear_box(sx, sy, 3, 1);
    state.tilemap.write_text(sx, sy, status, { max_length: 3 });
  } else if (hud.level_position) {
    const [lx, ly] = tile_coords(hud.level_position, tileSize);
    write_level_text(state.tilemap, lx, ly, pokemon.level);
  }
  const [hpX, hpY] = tile_coords(hud.hp_fill_position, tileSize);
  const paletteSlot = update_hp_palette(state, 'enemy', pokemon.hp, pokemon.max_hp);
  apply_hp_attr(state.tilemap, hpX, hpY, paletteSlot);
  const animatedPixels = advance_hp_animation(state, 'enemy', pokemon);
  battle_bars.draw_hp_bar(state.tilemap, hpX, hpY, pokemon.hp, pokemon.max_hp, state.hp_tiles, {
    is_player: false,
    palette_override: paletteSlot,
    pixel_override: animatedPixels,
  });
};

export const draw_player_hud = (state: BattleUIState, pokemon: Pokemon): void => {
  const hud = state.layout.player_hud;
  const tileSize: number = state.ui.tile_size ?? state.ui.tileSize ?? 8;
  const [nameX, nameY] = tile_coords(hud.name_position, tileSize);
  state.tilemap.clear_box(nameX, nameY, 10, 1);
  state.tilemap.write_text(nameX, nameY, pokemon.nickname, { max_length: 10, uppercase: false });
  const status = status_token(pokemon.status as StatusCondition | null | undefined);
  if (status && hud.status_position) {
    const [sx, sy] = tile_coords(hud.status_position, tileSize);
    state.tilemap.clear_box(sx, sy, 3, 1);
    state.tilemap.write_text(sx, sy, status, { max_length: 3 });
  } else {
    const [lx, ly] = tile_coords(hud.level_position, tileSize);
    write_level_text(state.tilemap, lx, ly, pokemon.level);
  }
  const [hpX, hpY] = tile_coords(hud.hp_fill_position, tileSize);
  const paletteSlot = update_hp_palette(state, 'player', pokemon.hp, pokemon.max_hp);
  apply_hp_attr(state.tilemap, hpX, hpY, paletteSlot);
  const animatedPixels = advance_hp_animation(state, 'player', pokemon);
  battle_bars.draw_hp_bar(state.tilemap, hpX, hpY, pokemon.hp, pokemon.max_hp, state.hp_tiles, {
    is_player: true,
    palette_override: paletteSlot,
    pixel_override: animatedPixels,
  });
  write_player_hp_digits(state.tilemap, hpX, hpY, pokemon, paletteSlot);
  if (hud.exp_bar_position) {
    const [expX, expY] = tile_coords(hud.exp_bar_position, tileSize);
    battle_bars.draw_exp_bar(state.tilemap, expX, expY, pokemon, state.exp_tiles);
  }
};

export const advance_hp_animation = (
  state: BattleUIState,
  side: 'player' | 'enemy',
  pokemon: Pokemon
): number => {
  const currentHp = Math.max(0, Math.min(pokemon.hp, pokemon.max_hp));
  const maxHp = Math.max(0, pokemon.max_hp);
  const pixels = battle_bars.compute_hp_pixels(currentHp, maxHp);
  let animation = state.hp_animation_states[side];
  if (!animation) {
    animation = new HPBarAnimationState();
    state.hp_animation_states[side] = animation;
  }
  // ASM: SendOutPlayerMon/UpdatePlayerHUD redraw the switched-in battler HUD directly.
  // Snap when the battler identity changes so equal-max-HP switches do not ease from the old bar.
  animation.sync(pixels, maxHp, { subject_token: pokemon });
  return animation.step(state.audio_engine ?? null);
};

const write_player_hp_digits = (
  tilemap: BattleBackgroundTilemap,
  hp_tile_x: number,
  hp_tile_y: number,
  pokemon: Pokemon,
  palette_slot: number
): void => {
  const digitsX = hp_tile_x + 1;
  const digitsY = hp_tile_y + 1;
  tilemap.clear_box(digitsX, digitsY, 7, 1, { attr: palette_slot });
  const current = Math.max(0, Math.min(pokemon.hp, pokemon.max_hp));
  const maxHp = Math.max(0, pokemon.max_hp);
  tilemap.write_text(digitsX, digitsY, String(current).padStart(3, ' '));
  tilemap.write_text(digitsX + 3, digitsY, '/');
  tilemap.write_text(digitsX + 4, digitsY, String(maxHp).padStart(3, ' '));
};

import { BattleContext, BattleStateEnum } from '../../engine/battle/battle/battle-context';
export const draw_dialogue_or_menu = (
  state: BattleUIState,
  player: Pokemon,
  battle_context: BattleContext,
): void => {
  const dialogueVisible = battle_dialogue.is_visible(state.dialogue);
  const menuVisibleFlag = dialogueVisible ? false : menu_visible(state, battle_context);
  const menuWindow = menuVisibleFlag ? menu_window_for_header(current_menu_header(state)) : null;
  if (battle_context.currentState !== BattleStateEnum.PLAYER_ACTION_SELECT) {
    hide_menu_window(state, menuWindow);
    if (dialogueVisible) {
      render_dialogue_text(state, battle_context);
    }
    return;
  }

  if (dialogueVisible) {
    if (menuWindow) {
      draw_menu_window(state, menuWindow);
    } else {
      hide_menu_window(state, menuWindow);
    }
    render_dialogue_text(state, battle_context);
    return;
  }
  if (state.wram.current_menu === BattleMenu.MAIN) {
    render_prompt_text(state, player, menuWindow);
    draw_main_menu(state);
  } else if (state.wram.current_menu === BattleMenu.FIGHT) {
    render_move_windows(state);
    draw_move_menu(state, player);
  } else {
    hide_menu_window(state, menuWindow);
    draw_sub_menu(state, battle_context);
  }
};

export const draw_move_forget_menu = (state: BattleUIState): void => {
  const menu = state.move_forget_menu;
  const process = state.active_move_learn;
  if (!menu || !process) {
    return;
  }
  if (process.stage !== MoveLearningPhase.FORGET_MENU) {
    return;
  }
  const window = state.layout.text_box;
  let [innerX, innerY, width, height] = window_interior(window);
  clear_window_region(state, innerX, innerY, width, height);
  const moves = (process.pokemon.moves ?? []).filter(Boolean) as LearnedMove[];
  const options = moves.map((move) => formatMoveName(move.name));
  options.push('CANCEL');
  const maxRows = Math.min(options.length, height);
  const cursorTile = menu_cursor_tile(state);
  options.slice(0, maxRows).forEach((label, idx) => {
    const y = innerY + idx;
    state.tilemap.write_text(innerX + 1, y, label, { uppercase: false, space_tile: _CLEAR_TILE });
    if (idx === menu.selection) {
      state.tilemap.set_tile(innerX, y, cursorTile, PAL_MENU);
    }
  });
};

export const draw_yes_no_prompt = (state: BattleUIState): void => {
  const promptState = state.yes_no_prompt;
  if (!promptState.active || !promptState.prompt) {
    return;
  }
  promptState.prompt.draw();
};

const should_truncate_text_window = (
  state: BattleUIState,
  battle_context: BattleContext,
  menu_window: BattleTextWindow | null
): boolean => {
  return (
    menu_window !== null &&
    menu_visible(state, battle_context) &&
    battle_dialogue.is_visible(state.dialogue) &&
    menu_window.tile_x > state.layout.text_box.tile_x &&
    !state.trainer_intro
  );
};

export const render_text_window_band = (state: BattleUIState, battle_context: BattleContext): void => {
  if (!text_window_visible(state, battle_context)) {
    hide_window_register(state);
    return;
  }
  const [targetWindow] = text_window_target(state, battle_context);
  const windowToDraw = targetWindow ?? state.layout.text_box;
  sync_window_register(state, windowToDraw);
  const screenHeight = state.ui.screen?.height ?? 0;
  if (state.hardware.wy >= screenHeight) {
    return;
  }
  state.tilemap.draw_window(
    windowToDraw.tile_x,
    windowToDraw.tile_y,
    windowToDraw.width_tiles,
    windowToDraw.height_tiles,
    { attr: PAL_TEXT_WINDOW }
  );
};

export const text_window_target = (
  state: BattleUIState,
  battle_context: BattleContext
): [BattleTextWindow, boolean, BattleTextWindow | null] => {
  const window = state.layout.text_box;
  const dialogueVisible = battle_dialogue.is_visible(state.dialogue);
  const menuVisibleFlag = dialogueVisible ? false : menu_visible(state, battle_context);
  const menuWindow = menuVisibleFlag ? menu_window_for_header(current_menu_header(state)) : null;
  const truncated = should_truncate_text_window(state, battle_context, menuWindow);
  if (truncated && menuWindow) {
    const leftWidth = Math.max(1, menuWindow.tile_x - window.tile_x);
    const targetWindow = BattleTextWindowSchema.parse({
      tile_x: window.tile_x,
      tile_y: window.tile_y,
      width_tiles: leftWidth,
      height_tiles: window.height_tiles,
    });
    return [targetWindow, true, menuWindow];
  }
  return [window, false, menuWindow];
};

export const text_window_visible = (state: BattleUIState, battle_context: BattleContext): boolean => {
  return (
    battle_context.currentState === BattleStateEnum.BATTLE_START ||
    battle_context.currentState === BattleStateEnum.PLAYER_ACTION_SELECT ||
    battle_dialogue.is_visible(state.dialogue) ||
    menu_visible(state, battle_context) ||
    state.yes_no_prompt.active ||
    Boolean(state.move_forget_menu) ||
    Boolean(state.trainer_exit)
  );
};

export const render_dialogue_text = (state: BattleUIState, battle_context: BattleContext): void => {
  const [targetWindow] = text_window_target(state, battle_context);
  const [innerX, innerY, width, height] = write_window_text(
    state,
    targetWindow,
    state.dialogue.dialogue.visible_text ?? '',
    { uppercase: false }
  );
  if (battle_dialogue.requires_ack(state.dialogue) && state.dialogue.dialogue.is_complete()) {
    const arrowX = innerX + Math.max(0, width - 1);
    const arrowY = innerY + Math.max(0, height - 1);
    state.tilemap.write_text(arrowX, arrowY, '\u25bc', { max_length: 1 });
  }
};

export const render_prompt_text = (
  state: BattleUIState,
  pokemon: Pokemon,
  menu_window: BattleTextWindow | null
): void => {
  const window = state.layout.text_box;
  const [innerX, innerY, initialWidth, height] = window_interior(window);
  let width = initialWidth;
  if (menu_window) {
    const menuStart = menu_window.tile_x;
    width = Math.min(width, Math.max(1, menuStart - window.tile_x - 1));
  }
  clear_window_region(state, innerX, innerY, width, height);
  const battleType = String((state.game_state.wram as { battle_type?: string }).battle_type ?? '').toUpperCase();
  const promptName =
    battleType === 'BATTLETYPE_TUTORIAL'
      ? state.game_state.sram.player_name || 'PLAYER'
      : pokemon.nickname;
  const prompt = `What will ${promptName} do?`;
  wrap_prompt_text(prompt, width, height).forEach((line, idx) => {
    state.tilemap.write_text(innerX, innerY + idx, line, {
      max_length: width,
      uppercase: false,
      space_tile: _CLEAR_TILE,
    });
  });
};

export const draw_main_menu = (state: BattleUIState): void => {
  const header = current_menu_header(state);
  const menuWindow = menu_window_for_header(header);
  draw_menu_window(state, menuWindow);
  const [baseX, baseY, width, height] = window_interior(menuWindow);
  clear_window_region(state, baseX, baseY, width, height);
  const options = header.labels.length ? header.labels : ['FIGHT', 'PKMN', 'PACK', 'RUN'];
  const cursorTile = menu_cursor_tile(state);
  options.forEach((label, idx) => {
    const [textX, textY] = tile_coords_for_option(header, idx);
    state.tilemap.write_text(textX, textY, label);
    if (idx === state.wram.wBattleMenuCursorPosition) {
      const cursorX = Math.max(header.coords.left, textX - 1);
      state.tilemap.set_tile(cursorX, textY, cursorTile, PAL_MENU);
    }
  });
  draw_menu_counters(state, header);
};

export const draw_move_menu = (state: BattleUIState, pokemon: Pokemon): void => {
  const layout = state.layout;
  const [textX, textY] = layout.move_menu_origin;
  const moves = (pokemon.moves ?? []).filter(Boolean) as LearnedMove[];
  const total = moves.length + 1;
  const rowSpacing = Math.max(1, layout.move_menu_row_spacing);
  let visibleRows = total;
  if (layout.move_selection_window) {
    const [innerX, innerY, width, height] = window_interior(layout.move_selection_window);
    clear_window_region(state, innerX, innerY, width, height);
    visibleRows = Math.max(1, Math.floor(height / rowSpacing));
  }
  visibleRows = Math.min(total, visibleRows);
  const cursorIndex = Math.max(0, Math.min(state.wram.wMoveMenuCursorPosition, total - 1));
  let start = 0;
  if (total > visibleRows) {
    start = Math.min(Math.max(cursorIndex - visibleRows + 1, 0), total - visibleRows);
  }
  const cursorTile = menu_cursor_tile(state);
  for (let visibleIndex = 0; visibleIndex < visibleRows; visibleIndex += 1) {
    const idx = start + visibleIndex;
    const label = idx < moves.length ? moveDisplayName(moves[idx].name) : 'CANCEL';
    const y = textY + visibleIndex * rowSpacing;
    state.tilemap.write_text(textX, y, label, { uppercase: false, space_tile: _CLEAR_TILE });
    if (idx === cursorIndex) {
      const cursorX = Math.max(0, textX - 1);
      state.tilemap.set_tile(cursorX, y, cursorTile, PAL_MENU);
    }
  }
  draw_move_swap_marker(state, moves, textX, textY, rowSpacing, start, visibleRows);
  if (layout.move_info_window) {
    draw_move_info(state, pokemon);
  }
};

export const draw_move_info = (state: BattleUIState, pokemon: Pokemon): void => {
  const window = state.layout.move_info_window;
  if (!window) {
    return;
  }
  const [baseX, baseY, width, height] = window_interior(window);
  clear_window_region(state, baseX, baseY, width, height);
  const moves = (pokemon.moves ?? []).filter(Boolean) as LearnedMove[];
  const selected = selected_move_entry(state.wram.wMoveMenuCursorPosition, moves);
  if (!selected) {
    state.tilemap.write_text(baseX, baseY, 'TYPE/');
    state.tilemap.write_text(baseX + 1, baseY + 1, '----', { uppercase: false });
    state.tilemap.write_text(baseX, baseY + 2, 'PP --/--', { uppercase: false });
    return;
  }
  if (pokemon.disable_turns > 0 && pokemon.disabled_move === selected.name) {
    state.tilemap.write_text(baseX, baseY + 1, 'Disabled!', {
      uppercase: false,
      space_tile: _CLEAR_TILE,
    });
    return;
  }
  const moveData = loadMoveMetadata().get(selected.name);
  const moveType = moveData ? moveData.type : PokemonType.NONE;
  const basePp = moveData ? moveData.pp : selected.current_pp;
  state.tilemap.write_text(baseX, baseY, 'TYPE/', { space_tile: _CLEAR_TILE });
  const typeLabel = type_display_name(moveType);
  state.tilemap.write_text(baseX + 1, baseY + 1, typeLabel, {
    uppercase: false,
    space_tile: _CLEAR_TILE,
  });
  state.tilemap.write_text(baseX, baseY + 2, 'PP', { space_tile: _CLEAR_TILE });
  const currentPp = Math.max(0, Math.min(selected.current_pp, 99));
  const maxPp = Math.max(0, Math.min(basePp, 99));
  state.tilemap.write_text(baseX + 3, baseY + 2, String(currentPp).padStart(2, ' '), {
    space_tile: _CLEAR_TILE,
  });
  state.tilemap.write_text(baseX + 5, baseY + 2, '/', { space_tile: _CLEAR_TILE });
  state.tilemap.write_text(baseX + 6, baseY + 2, String(maxPp).padStart(2, ' '), {
    space_tile: _CLEAR_TILE,
  });
};

export const draw_move_swap_marker = (
  state: BattleUIState,
  moves: LearnedMove[],
  text_x: number,
  text_y: number,
  row_spacing: number,
  start: number,
  visible_rows: number
): void => {
  const swapIndex = state.wram.swapping_move_index;
  if (swapIndex === null || swapIndex === undefined || swapIndex >= moves.length) {
    return;
  }
  const offset = swapIndex - start;
  if (offset < 0 || offset >= visible_rows) {
    return;
  }
  const cursorX = Math.max(0, text_x - 1);
  const y = text_y + offset * row_spacing;
  state.tilemap.write_text(cursorX, y, '\u25b7', { uppercase: false });
};

export const draw_sub_menu = (state: BattleUIState, battle_context: BattleContext): void => {
  hide_menu_window(state, menu_window_for_header(current_menu_header(state)));
  const window = state.layout.text_box;
  const [baseX, baseY, width, height] = window_interior(window);
  clear_window_region(state, baseX, baseY, width, height);
  const cursorTile = menu_cursor_tile(state);
  if (state.wram.current_menu === BattleMenu.POKEMON) {
    draw_battle_party_menu(state, battle_context, baseX, baseY, cursorTile);
  } else if (state.wram.current_menu === BattleMenu.PACK) {
    const items = state.wram.last_item_names ?? [];
    if (!items.length) {
      state.tilemap.write_text(baseX, baseY, 'No usable items', { uppercase: false });
      return;
    }
    items.slice(0, Math.max(1, Math.floor(height / 2))).forEach((name, idx) => {
      const y = baseY + idx * 2;
      state.tilemap.write_text(baseX + 1, y, name, { uppercase: false });
      if (idx === state.wram.wPackMenuCursorPosition) {
        state.tilemap.set_tile(baseX, y, cursorTile, PAL_MENU);
      }
    });
  }
};

export const current_menu_header = (state: BattleUIState): BattleMenuHeader => {
  return (state.wram.menu_header as BattleMenuHeader) ?? BATTLE_MENU_HEADER;
};

export const menu_window_for_header = (header: BattleMenuHeader): BattleTextWindow => {
  return BattleTextWindowSchema.parse({
    tile_x: header.coords.left,
    tile_y: header.coords.top,
    width_tiles: header.coords.width,
    height_tiles: header.coords.height,
  });
};

export const menu_visible = (state: BattleUIState, battle_context: BattleContext): boolean => {
  if (battle_dialogue.is_visible(state.dialogue)) {
    return false;
  }
  return (
    battle_context.currentState === BattleStateEnum.PLAYER_ACTION_SELECT &&
    state.wram.current_menu === BattleMenu.MAIN
  );
};

export const menu_header_for_battle = (state: BattleUIState): BattleMenuHeader => {
  const battleType = (state.game_state.wram as { battle_type?: string }).battle_type ?? '';
  const normalized = String(battleType).toUpperCase();
  if (normalized === 'BATTLETYPE_SAFARI') {
    return SAFARI_MENU_HEADER;
  }
  if (['BATTLETYPE_CONTEST', 'BATTLETYPE_BUG_CONTEST', 'BATTLETYPE_PARK'].includes(normalized)) {
    return BUG_CATCHING_MENU_HEADER;
  }
  return BATTLE_MENU_HEADER;
};

export const draw_menu_counters = (state: BattleUIState, header: BattleMenuHeader): void => {
  if (header === SAFARI_MENU_HEADER) {
    draw_safari_ball_counter(state);
  } else if (header === BUG_CATCHING_MENU_HEADER) {
    draw_park_ball_counter(state);
  }
};

const draw_safari_ball_counter = (state: BattleUIState): void => {
  const count = (state.game_state.wram as { safari_balls_remaining?: number }).safari_balls_remaining ?? 0;
  state.tilemap.write_text(17, 13, String(count).padStart(2, ' '));
};

const draw_park_ball_counter = (state: BattleUIState): void => {
  const count = (state.game_state.wram as { bug_contest_state?: { park_balls_remaining?: number } })
    .bug_contest_state?.park_balls_remaining ?? 0;
  state.tilemap.write_text(13, 16, String(count).padStart(2, ' '));
};
