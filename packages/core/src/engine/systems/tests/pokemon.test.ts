import { createPokemon } from '../pokemon';
import { PokemonSpecies } from '../../../core/models';
import { GameState, createInitialGameState } from '../../../core/state';
import { GrowthRate, PokemonType, EggGroup, GenderRatio, Ability } from '../../../core/enums';

describe('createPokemon', () => {
  const buildSpecies = (id = "CHARMANDER"): PokemonSpecies => ({
    id,
    int_id: id === "HO_OH" ? 250 : 4,
    base_stats: { hp: 39, attack: 52, defense: 43, speed: 65, special_attack: 60, special_defense: 50 },
    type1: PokemonType.FIRE,
    type2: PokemonType.FIRE,
    catch_rate: 45,
    base_exp: 62,
    gender_ratio: GenderRatio.GENDER_F12_5,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_DRAGON,
    tmhm_learnset: [],
    evolutions: null,
    ability: Ability.NONE,
    pic_size: 0x55,
    front_pic: 0,
    back_pic: 0,
  } as any);

  it('should clamp the experience of a level 1 GROWTH_MEDIUM_SLOW Pokemon to 0', () => {
    const species = buildSpecies();
    const gameState: GameState = createInitialGameState();
    const pokemon = createPokemon(gameState, species, 1);
    expect(pokemon.experience).toBe(0);
  });

  it("uses Ho-Oh's ASM species name as the generated nickname", () => {
    const gameState: GameState = createInitialGameState();
    const pokemon = createPokemon(gameState, buildSpecies("HO_OH"), 60);

    expect(pokemon.nickname).toBe("HO-OH");
  });
});
