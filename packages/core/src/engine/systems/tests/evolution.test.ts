import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { PokemonSchema, type LearnedMove } from "@pokecrystal/core/core/models";
import { toPokemon } from "@pokecrystal/core/core/models/pokemon";
import { Stat } from "@pokecrystal/core/core/enums/pokemon";
import { Evolution, HAPPINESS_TO_EVOLVE } from "../evolution";

type PokemonOverrides = Partial<ReturnType<typeof toPokemon>>;

const loader = new DataLoader();

const buildPokemon = (
  speciesId: string,
  level: number,
  overrides: PokemonOverrides = {}
) => {
  const species = loader.get_pokemon_species(speciesId);
  if (!species) {
    throw new Error(`Missing species data for ${speciesId}.`);
  }
  const data = PokemonSchema.parse({
    species,
    nickname: species.id,
    level,
    hp: 1,
    max_hp: 1,
    original_trainer_name: "TEST",
    original_trainer_id: 0,
    experience: 0,
    happiness: 0,
    ...overrides,
  });
  const pokemon = toPokemon(data);
  pokemon.max_hp = pokemon._calculateStat(Stat.HP);
  pokemon.hp = pokemon.max_hp;
  pokemon.attack = pokemon._calculateStat(Stat.ATTACK);
  pokemon.defense = pokemon._calculateStat(Stat.DEFENSE);
  pokemon.speed = pokemon._calculateStat(Stat.SPEED);
  pokemon.special_attack = pokemon._calculateStat(Stat.SPECIAL_ATTACK);
  pokemon.special_defense = pokemon._calculateStat(Stat.SPECIAL_DEFENSE);
  Object.assign(pokemon, overrides);
  return pokemon;
};

describe("Evolution", () => {
  beforeAll(() => {
    loader.ensure_battle_data();
  });

  it("handles happiness evolutions with time of day gating", () => {
    const eeveeDay = buildPokemon("EEVEE", 20, { happiness: HAPPINESS_TO_EVOLVE });
    const dayEvolution = new Evolution(eeveeDay, {
      data_loader: loader,
      time_of_day: "day",
    });
    expect(dayEvolution.check_for_evolution()?.species).toBe("ESPEON");

    const eeveeNight = buildPokemon("EEVEE", 20, { happiness: HAPPINESS_TO_EVOLVE });
    const nightEvolution = new Evolution(eeveeNight, {
      data_loader: loader,
      time_of_day: "nite",
    });
    expect(nightEvolution.check_for_evolution()?.species).toBe("UMBREON");

    const pichu = buildPokemon("PICHU", 10, { happiness: HAPPINESS_TO_EVOLVE });
    const anytimeEvolution = new Evolution(pichu, {
      data_loader: loader,
      time_of_day: "nite",
    });
    expect(anytimeEvolution.check_for_evolution()?.species).toBe("PIKACHU");
  });

  it("supports level, item, stat, and trade evolutions with ASM rules", () => {
    const bulbasaur = buildPokemon("BULBASAUR", 16);
    const levelEvolution = new Evolution(bulbasaur, { data_loader: loader });
    expect(levelEvolution.check_for_evolution()?.species).toBe("IVYSAUR");

    const blocked = buildPokemon("BULBASAUR", 16, { item: "EVERSTONE" });
    const blockedEvolution = new Evolution(blocked, { data_loader: loader });
    expect(blockedEvolution.check_for_evolution()).toBeNull();

    const pikachu = buildPokemon("PIKACHU", 20);
    const stoneEvolution = new Evolution(pikachu, {
      data_loader: loader,
      current_item: "THUNDERSTONE",
      force_evolution: true,
    });
    expect(stoneEvolution.check_for_evolution()?.species).toBe("RAICHU");

    const tyrogue = buildPokemon("TYROGUE", 20, { attack: 20, defense: 10 });
    const statEvolution = new Evolution(tyrogue, { data_loader: loader });
    expect(statEvolution.check_for_evolution()?.species).toBe("HITMONLEE");

    const onix = buildPokemon("ONIX", 30, { item: "METAL_COAT" });
    const tradeEvolution = new Evolution(onix, {
      data_loader: loader,
      link_mode: "LINK",
    });
    expect(tradeEvolution.check_for_evolution()?.species).toBe("STEELIX");
  });

  it("removes trade items and learns evolution moves at the current level", () => {
    const onix = buildPokemon("ONIX", 30, { item: "METAL_COAT" });
    const tradeEvolution = new Evolution(onix, {
      data_loader: loader,
      link_mode: "LINK",
    });
    tradeEvolution.check_for_evolution();
    tradeEvolution.evolve(false);
    expect(onix.item).toBeNull();
    expect(tradeEvolution.events.some((event) => event.type === "item")).toBe(true);

    const moves: LearnedMove[] = [
      { name: "SCRATCH", current_pp: 35 },
      { name: "GROWL", current_pp: 40 },
    ];
    const charmeleon = buildPokemon("CHARMELEON", 36, { moves });
    const moveEvolution = new Evolution(charmeleon, { data_loader: loader });
    expect(moveEvolution.check_for_evolution()?.species).toBe("CHARIZARD");
    moveEvolution.evolve(false);
    expect(charmeleon.moves?.some((move) => move.name === "WING_ATTACK")).toBe(true);
    expect(
      moveEvolution.events.some(
        (event) => event.type === "move" && event.id === "WING_ATTACK"
      )
    ).toBe(true);
  });
});
