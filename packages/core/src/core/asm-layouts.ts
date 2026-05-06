export const NUM_TM_HM_TUTOR = 60;
export const TMHM_BYTES = Math.ceil(NUM_TM_HM_TUTOR / 8);
export const BASE_DATA_SIZE = 32;
export const PARTYMON_STRUCT_LENGTH = 48;
export const NUM_MOVES = 4;
export const NUM_EXP_STATS = 5;
export const NUM_BATTLE_STATS = NUM_EXP_STATS - 1; // HP excluded

export type FieldEntry = [string, number];
export type FieldOffset = [string, number, number];

const _BASE_DATA_FIELD_ORDER: FieldEntry[] = [
  ['BASE_DEX_NO', 1],
  ['BASE_STATS', 6],
  ['BASE_TYPES', 2],
  ['BASE_CATCH_RATE', 1],
  ['BASE_EXP', 1],
  ['BASE_ITEMS', 2],
  ['BASE_GENDER', 1],
  ['BASE_UNKNOWN_1', 1],
  ['BASE_EGG_STEPS', 1],
  ['BASE_UNKNOWN_2', 1],
  ['BASE_PIC_SIZE', 1],
  ['BASE_FRONTPIC', 2],
  ['BASE_BACKPIC', 2],
  ['BASE_GROWTH_RATE', 1],
  ['BASE_EGG_GROUPS', 1],
  ['BASE_TMHM', TMHM_BYTES],
];

const _PARTY_FIELD_ORDER: FieldEntry[] = [
  ['MON_SPECIES', 1],
  ['MON_ITEM', 1],
  ['MON_MOVES', NUM_MOVES],
  ['MON_OT_ID', 2],
  ['MON_EXP', 3],
  ['MON_HP_EXP', 2],
  ['MON_ATK_EXP', 2],
  ['MON_DEF_EXP', 2],
  ['MON_SPD_EXP', 2],
  ['MON_SPC_EXP', 2],
  ['MON_DVS', 2],
  ['MON_PP', NUM_MOVES],
  ['MON_HAPPINESS', 1],
  ['MON_POKERUS', 1],
  ['MON_CAUGHTDATA', 2],
  ['MON_LEVEL', 1],
  ['MON_STATUS', 1],
  ['MON_UNUSED', 1],
  ['MON_HP', 2],
  ['MON_MAXHP', 2],
  ['MON_ATK', 2],
  ['MON_DEF', 2],
  ['MON_SPD', 2],
  ['MON_SAT', 2],
  ['MON_SDF', 2],
];

function _sum_field_lengths(fields: FieldEntry[]): number {
  return fields.reduce((sum, [, length]) => sum + length, 0);
}

function _validate_layout(fields: FieldEntry[], size: number, name: string): void {
  const actual = _sum_field_lengths(fields);
  if (actual !== size) {
    throw new Error(`${name} layout is ${actual} bytes but expected ${size} bytes`);
  }
}

_validate_layout(_BASE_DATA_FIELD_ORDER, BASE_DATA_SIZE, 'BASE_DATA');
_validate_layout(_PARTY_FIELD_ORDER, PARTYMON_STRUCT_LENGTH, 'PARTYMON');

export function field_offsets(fields: FieldEntry[]): FieldOffset[] {
  const offsets: FieldOffset[] = [];
  let running_offset = 0;
  for (const [label, length] of fields) {
    offsets.push([label, running_offset, length]);
    running_offset += length;
  }
  return offsets;
}

export function base_data_offsets(): FieldOffset[] {
  return field_offsets(_BASE_DATA_FIELD_ORDER);
}

export function party_mon_offsets(): FieldOffset[] {
  return field_offsets(_PARTY_FIELD_ORDER);
}

export function base_data_fields(): string[] {
  return _BASE_DATA_FIELD_ORDER.map(([label]) => label);
}

export function party_mon_fields(): string[] {
  return _PARTY_FIELD_ORDER.map(([label]) => label);
}
