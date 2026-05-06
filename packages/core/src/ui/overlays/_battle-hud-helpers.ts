import { PokemonType, StatusCondition } from '../../core/enums';
import { BattleBackgroundTilemap } from './_battle-background';
import { LearnedMove } from '../../core/models';
import { LV_GLYPH } from '@pokecrystal/assets/content/text-constants';

// Mirrors pokecrystal_disassembly/engine/battle/core.asm (DrawPlayerHUD/DrawEnemyHUD).
export const status_token = (status?: StatusCondition | null): string | null => {
  if (!status) {
    return null;
  }
  const mapping: Partial<Record<StatusCondition, string>> = {
    [StatusCondition.POISON]: 'PSN',
    [StatusCondition.SLEEP]: 'SLP',
    [StatusCondition.PARALYSIS]: 'PAR',
    [StatusCondition.BURN]: 'BRN',
    [StatusCondition.FREEZE]: 'FRZ',
  };
  return mapping[status] ?? null;
};

export const write_level_text = (
  tilemap: BattleBackgroundTilemap,
  tile_x: number,
  tile_y: number,
  level: number
): void => {
  const text = format_level_text(level);
  const width = text.length;
  tilemap.clear_box(tile_x, tile_y, width, 1);
  tilemap.write_text(tile_x, tile_y, text, { max_length: width, uppercase: false });
};

export const format_level_text = (level: number): string => {
  const clamped = Math.max(1, Math.min(255, Math.trunc(level)));
  return `${LV_GLYPH}${clamped}`;
};

export const type_display_name = (pokemonType: PokemonType): string => {
  let name = String(pokemonType);
  if (name.endsWith('_TYPE')) {
    name = name.slice(0, -5);
  }
  return name.replace(/_/g, ' ');
};

export const normalise_window_lines = (text: string | Iterable<string>): string[] => {
  if (typeof text === 'string') {
    return text ? text.split('\n') : [''];
  }
  return Array.from(text, (value) => String(value));
};

export const selected_move_entry = (
  cursorIndex: number,
  moves: LearnedMove[]
): LearnedMove | null => {
  if (!moves.length) {
    return null;
  }
  const index = Math.min(Math.max(cursorIndex, 0), moves.length);
  if (index >= moves.length) {
    return null;
  }
  return moves[index];
};
