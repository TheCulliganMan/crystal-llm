import { build_base_data, tmhm_bitfield } from '../base-data';
import { PokemonSpecies } from '../models/pokemon';
import { TMHM_BYTES } from '../asm-layouts';
import { GrowthRate, EggGroup, GenderRatio, PokemonType, Ability } from '../enums';
import { MoveName } from '../enums/move';
import { Item } from '../enums/item';

describe('base-data', () => {
  it('should correctly pack PokemonSpecies into base_data format', () => {
    const species: PokemonSpecies = {
      id: 'BULBASAUR',
      int_id: 1,
      base_stats: {
        hp: 45,
        attack: 49,
        defense: 49,
        speed: 45,
        special_attack: 65,
        special_defense: 65,
      },
      type1: PokemonType.GRASS,
      type2: PokemonType.POISON,
      catch_rate: 45,
      base_exp: 64,
      item1: Item.NONE,
      item2: Item.NONE,
      gender_ratio: GenderRatio.GENDER_F12_5,
      unknown1: 100,
      step_cycles_to_hatch: 20,
      unknown2: 0,
      growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
      egg_group1: EggGroup.EGG_MONSTER,
      egg_group2: EggGroup.EGG_PLANT,
      tmhm_learnset: [MoveName.HEADBUTT, MoveName.CURSE, MoveName.SUNNY_DAY],
      ability: Ability.NONE,
      pic_size: 6,
      front_pic: 1,
      back_pic: 1,
      evolutions: null,
      weight: 0,
    };

    const data = build_base_data(species);
    const view = new DataView(data.buffer);

    expect(data.length).toBe(32);
    expect(data[0]).toBe(1); // int_id
    expect(data[1]).toBe(45); // hp
    expect(data[2]).toBe(49); // attack
    expect(data[3]).toBe(49); // defense
    expect(data[4]).toBe(45); // speed
    expect(data[5]).toBe(65); // special_attack
    expect(data[6]).toBe(65); // special_defense
    expect(data[7]).toBe(22); // type1 (GRASS)
    expect(data[8]).toBe(3); // type2 (POISON)
    expect(data[9]).toBe(45); // catch_rate
    expect(data[10]).toBe(64); // base_exp
    expect(data[11]).toBe(0); // item1
    expect(data[12]).toBe(0); // item2
    expect(data[13]).toBe(31); // gender_ratio
    expect(data[14]).toBe(100); // unknown1
    expect(data[15]).toBe(20); // step_cycles_to_hatch
    expect(data[16]).toBe(0); // unknown2
    expect(data[17]).toBe(6); // pic_size
    expect(view.getUint16(18, true)).toBe(1); // front_pic
    expect(view.getUint16(20, true)).toBe(1); // back_pic
    expect(data[22]).toBe(3); // growth_rate (GROWTH_MEDIUM_SLOW)
    expect(data[23]).toBe((7 << 4) | 1); // egg_group2 << 4 | egg_group1


    const expectedTmhm = new Array(TMHM_BYTES).fill(0);
    expectedTmhm[0] = 0b00000110; // HEADBUTT, CURSE
    expectedTmhm[1] = 0b00000100; // SUNNY_DAY
    const tmhmBytes = data.slice(24, 32);
    expect(Array.from(tmhmBytes)).toEqual(expectedTmhm);
  });

  it('should correctly generate tmhm_bitfield', () => {
    const learnset = [MoveName.DYNAMICPUNCH, MoveName.HEADBUTT, MoveName.CUT, MoveName.FLY];
    const bitfield = tmhm_bitfield(learnset);
    const expected = new Array(TMHM_BYTES).fill(0);
    expected[0] = 0b00000011; // DYNAMICPUNCH, HEADBUTT
    expected[6] = 12; // CUT, FLY
    expect(Array.from(bitfield)).toEqual(expected);
  });
});
