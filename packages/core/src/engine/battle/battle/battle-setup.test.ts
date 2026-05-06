import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { MoveName, Stat, StatusCondition } from "@pokecrystal/core/core/enums";
import type { Battle } from "./battle-logic";
import { initialiseEnemyParty, initialisePlayerParty } from "./battle-setup";

const buildSpecies = (id: string) =>
  PokemonSpeciesSchema.parse({
    id,
    int_id: 0,
    base_stats: {
      hp: 40,
      attack: 50,
      defense: 40,
      speed: 45,
      special_attack: 35,
      special_defense: 35,
    },
    type1: "NORMAL",
    type2: "NORMAL",
    catch_rate: 255,
    base_exp: 50,
    gender_ratio: 31,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: "GROWTH_MEDIUM_FAST",
    egg_group1: "EGG_NONE",
    egg_group2: "EGG_NONE",
  });

describe("initialiseEnemyParty tree sleep handling", () => {
  it("sets sleep status for tree encounters that match the time-of-day list", () => {
    const gameState = createInitialGameState();
    gameState.wram.battle_type = "BATTLETYPE_TREE";
    gameState.wram.time_of_day = "day";
    const enemy = createPokemon(gameState, buildSpecies("HOOTHOOT"), 10);
    const battle = { gameState } as Battle;

    const party = initialiseEnemyParty(battle, enemy, [enemy], undefined);

    expect(party[0].status).toBe(StatusCondition.SLEEP);
    expect(party[0].sleep_turns).toBe(7);
  });

  it("leaves wild status alone outside of tree battles", () => {
    const gameState = createInitialGameState();
    gameState.wram.battle_type = "BATTLETYPE_NORMAL";
    gameState.wram.time_of_day = "day";
    const enemy = createPokemon(gameState, buildSpecies("HOOTHOOT"), 10);
    const battle = { gameState } as Battle;

    const party = initialiseEnemyParty(battle, enemy, [enemy], undefined);

    expect(party[0].status).toBeUndefined();
    expect(party[0].sleep_turns).toBe(0);
  });
});

describe("initialisePlayerParty", () => {
  it("clears stale Disable from saved party members when a battle starts", () => {
    const gameState = createInitialGameState();
    const player = createPokemon(gameState, buildSpecies("CROCONAW"), 28);
    player.disabled_move = MoveName.BITE;
    player.disable_turns = 6;
    const battle = { gameState } as Battle;

    const party = initialisePlayerParty(battle, player, [player]);

    expect(party[0].disabled_move).toBeUndefined();
    expect(party[0].disable_turns).toBe(0);
    expect(player.disabled_move).toBeUndefined();
    expect(player.disable_turns).toBe(0);
  });

  it("clears stale stat stages from saved party members when a battle starts", () => {
    const gameState = createInitialGameState();
    const player = createPokemon(gameState, buildSpecies("CROCONAW"), 28);
    player.stat_boosts[Stat.DEFENSE] = -2;
    player.stat_boosts[Stat.ACCURACY] = -1;
    const battle = { gameState } as Battle;

    const party = initialisePlayerParty(battle, player, [player]);

    expect(party[0].stat_boosts[Stat.DEFENSE]).toBe(0);
    expect(party[0].stat_boosts[Stat.ACCURACY]).toBe(0);
    expect(player.stat_boosts[Stat.DEFENSE]).toBe(0);
    expect(player.stat_boosts[Stat.ACCURACY]).toBe(0);
  });
});
