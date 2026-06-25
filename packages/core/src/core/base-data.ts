/**
 * Helpers for packing Pokémon base data exactly like the ASM structures.
 */

import {
  BASE_DATA_SIZE,
  TMHM_BYTES,
  base_data_offsets,
} from './asm-layouts';
import { TMHM_MOVES } from './constants';
import { Item } from './enums/item';
import { MoveName } from './enums/move';
import {
  EggGroup,
  GrowthRate,
  PokemonType,
} from './enums/pokemon';
import { PokemonSpecies } from './models/pokemon';

const TYPE_VALUE_MAP: Record<string, number> = {
  [PokemonType.NORMAL]: 0,
  [PokemonType.FIGHTING]: 1,
  [PokemonType.FLYING]: 2,
  [PokemonType.POISON]: 3,
  [PokemonType.GROUND]: 4,
  [PokemonType.ROCK]: 5,
  [PokemonType.BUG]: 7,
  [PokemonType.GHOST]: 8,
  [PokemonType.STEEL]: 9,
  [PokemonType.CURSE_TYPE]: 19,
  [PokemonType.FIRE]: 20,
  [PokemonType.WATER]: 21,
  [PokemonType.GRASS]: 22,
  [PokemonType.ELECTRIC]: 23,
  [PokemonType.PSYCHIC_TYPE]: 24,
  [PokemonType.ICE]: 25,
  [PokemonType.DRAGON]: 26,
  [PokemonType.DARK]: 27,
  [PokemonType.UNKNOWN]: 0,
  [PokemonType.NONE]: 0,
};

const GROWTH_RATE_VALUE_MAP: Record<string, number> = {
  [GrowthRate.GROWTH_MEDIUM_FAST]: 0,
  [GrowthRate.GROWTH_MEDIUM_SLOW]: 3,
  [GrowthRate.GROWTH_FAST]: 4,
  [GrowthRate.GROWTH_SLOW]: 5,
};

const EGG_GROUP_VALUE_MAP: Record<string, number> = {
  [EggGroup.EGG_MONSTER]: 1,
  [EggGroup.EGG_WATER_1]: 2,
  [EggGroup.EGG_BUG]: 3,
  [EggGroup.EGG_FLYING]: 4,
  [EggGroup.EGG_GROUND]: 5,
  [EggGroup.EGG_FAIRY]: 6,
  [EggGroup.EGG_PLANT]: 7,
  [EggGroup.EGG_HUMANSHAPE]: 8,
  [EggGroup.EGG_WATER_3]: 9,
  [EggGroup.EGG_MINERAL]: 10,
  [EggGroup.EGG_INDETERMINATE]: 11,
  [EggGroup.EGG_WATER_2]: 12,
  [EggGroup.EGG_DITTO]: 13,
  [EggGroup.EGG_DRAGON]: 14,
  [EggGroup.EGG_NONE]: 15,
};

const ITEM_VALUE_MAP: Record<string, number> = {};
Object.values(Item).forEach((item, index) => {
  ITEM_VALUE_MAP[item] = index;
});

function lookup(
  mapping: Record<string, number>,
  key: string,
  context: string
): number {
  const value = mapping[key];
  if (value === undefined) {
    throw new Error(`Missing ASM constant for ${context}: ${key}`);
  }
  return value;
}

export function tmhm_bitfield(learnset: MoveName[]): Uint8Array {
  const bits = new Uint8Array(TMHM_BYTES);
  for (const move of learnset) {
    const index = TMHM_MOVES.indexOf(move);
    if (index === -1) {
      throw new Error(`${move} is not a TM or HM move.`);
    }
    const byteIndex = Math.floor(index / 8);
    const bitIndex = index % 8;
    bits[byteIndex] |= 1 << bitIndex;
  }
  return bits;
}

function writeWord(buffer: DataView, offset: number, value: number): void {
  if (value < 0 || value > 0xffff) {
    throw new Error(`Word value ${value} does not fit in 16 bits.`);
  }
  buffer.setUint16(offset, value, true); // true for little-endian
}

export function build_base_data(species: PokemonSpecies): Uint8Array {
  const buffer = new Uint8Array(BASE_DATA_SIZE);
  const view = new DataView(buffer.buffer);
  const offsets: Record<string, number> = {};
  for (const [label, offset] of base_data_offsets()) {
    offsets[label] = offset;
  }

  buffer[offsets['BASE_DEX_NO']] = species.int_id & 0xff;

  const stats = species.base_stats;
  const statsBytes = [
    stats.hp,
    stats.attack,
    stats.defense,
    stats.speed,
    stats.special_attack,
    stats.special_defense,
  ];
  for (let i = 0; i < statsBytes.length; i++) {
    const value = statsBytes[i];
    if (value < 0 || value > 255) {
      throw new Error(`Base stat ${value} does not fit in one byte.`);
    }
    buffer[offsets['BASE_STATS'] + i] = value;
  }

  buffer[offsets['BASE_TYPES']] = lookup(TYPE_VALUE_MAP, species.type1, 'type');
  buffer[offsets['BASE_TYPES'] + 1] = lookup(
    TYPE_VALUE_MAP,
    species.type2 ?? PokemonType.NONE,
    'type'
  );

  buffer[offsets['BASE_CATCH_RATE']] = species.catch_rate & 0xff;
  buffer[offsets['BASE_EXP']] = species.base_exp & 0xff;

  buffer[offsets['BASE_ITEMS']] = lookup(
    ITEM_VALUE_MAP,
    species.item1 ?? 'NONE',
    'item'
  );
  buffer[offsets['BASE_ITEMS'] + 1] = lookup(
    ITEM_VALUE_MAP,
    species.item2 ?? 'NONE',
    'item'
  );

  buffer[offsets['BASE_GENDER']] = species.gender_ratio & 0xff;
  buffer[offsets['BASE_UNKNOWN_1']] = species.unknown1 & 0xff;
  buffer[offsets['BASE_EGG_STEPS']] = species.step_cycles_to_hatch & 0xff;
  buffer[offsets['BASE_UNKNOWN_2']] = species.unknown2 & 0xff;
  buffer[offsets['BASE_PIC_SIZE']] = species.pic_size & 0xff;

  writeWord(view, offsets['BASE_FRONTPIC'], species.front_pic);
  writeWord(view, offsets['BASE_BACKPIC'], species.back_pic);

  buffer[offsets['BASE_GROWTH_RATE']] = lookup(
    GROWTH_RATE_VALUE_MAP,
    species.growth_rate,
    'growth rate'
  );

  const egg_group1 = lookup(
    EGG_GROUP_VALUE_MAP,
    species.egg_group1,
    'egg group'
    );
  const egg_group2 = lookup(
    EGG_GROUP_VALUE_MAP,
    species.egg_group2 ?? EggGroup.EGG_NONE,
    'egg group'
    );
  buffer[offsets['BASE_EGG_GROUPS']] = (egg_group2 << 4) | egg_group1;

  const tmhmBytes = tmhm_bitfield(species.tmhm_learnset);
  buffer.set(tmhmBytes, offsets['BASE_TMHM']);

  return buffer;
}
