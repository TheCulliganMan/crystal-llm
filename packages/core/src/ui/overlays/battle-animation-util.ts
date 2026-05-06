import { MoveName } from '@pokecrystal/core/core/enums';
import { mirror_enemy_x } from './_battle-animation-helpers';
import { load_animation_table } from './_battle-animation-loader';
import { AnimationContext } from './_battle-animation-state';

const camelizeTokens = (tokens: Iterable<string>): string =>
  Array.from(tokens)
    .filter((token) => token)
    .map((token) => token.charAt(0).toUpperCase() + token.slice(1).toLowerCase())
    .join('');

const buildMoveAnimationLabels = (): [Record<string, string>, Set<string>] => {
  const table = load_animation_table();
  if (!table.length) {
    return [{}, new Set()];
  }
  const moveEntries = table.slice(1);
  const moveNames = Object.values(MoveName);
  if (moveNames.length < moveEntries.length) {
    throw new Error('MoveName enum is shorter than the BattleAnimations table.');
  }
  const mapping: Record<string, string> = {};
  moveEntries.forEach((label: string, index: number) => {
    const move = moveNames[index];
    mapping[String(move)] = label;
  });
  const unmapped = new Set(moveNames.slice(moveEntries.length).map((name) => String(name)));
  return [mapping, unmapped];
};

const [MOVE_ANIMATION_LABELS, UNMAPPED_MOVE_NAMES] = buildMoveAnimationLabels();

export const animation_label_for_move = (moveName: MoveName | string): string => {
  const raw = typeof moveName === 'string' ? moveName : String(moveName);
  const trimmed = raw.trim().replace(/ /g, '_');
  if (!trimmed) {
    throw new Error('move_name cannot be empty');
  }
  const normalized = trimmed.toUpperCase();
  const mapped = MOVE_ANIMATION_LABELS[normalized];
  if (mapped) {
    return mapped;
  }
  if (typeof moveName !== 'string') {
    if (!Object.keys(MOVE_ANIMATION_LABELS).length) {
      throw new Error('BattleAnimations table is unavailable; cannot resolve move animation.');
    }
    if (UNMAPPED_MOVE_NAMES.has(normalized)) {
      throw new Error(`${normalized} is not represented in the BattleAnimations table.`);
    }
    throw new Error(`Missing battle animation mapping for move ${normalized}.`);
  }
  const tokens = trimmed.split('_').filter((token) => token);
  const camel = camelizeTokens(tokens);
  if (!camel) {
    throw new Error('move_name cannot be empty');
  }
  return `BattleAnim_${camel}`;
};

export const tile_size_px = (ui: { tile_size?: number }): number => ui.tile_size || 8;

export const mirror_x = (x: number): number => mirror_enemy_x(x);

export const should_mirror = (context: AnimationContext | null): boolean =>
  context ? !context.is_player_move : false;

export const is_player_context = (context: AnimationContext | null): boolean =>
  context ? context.is_player_move : true;

export const resolve_panning = (tracks: number | null, isPlayerMove: boolean): string | null => {
  if (tracks === null || tracks === undefined) {
    return isPlayerMove ? 'player' : 'enemy';
  }
  const index = tracks & 0b11;
  const table = ['left', 'right', 'left', 'right'];
  const offset = isPlayerMove ? 0 : 1;
  return table[(index ^ offset) % table.length] ?? null;
};
