import { BattleContext } from "@/engine/battle/battle/battle-context";
import { executeMove } from "@/engine/battle/battle/move-execution";
import { EventManager } from "@/engine/events/events";
import { createInitialGameState } from "@/core/state";
import { PokemonSchema, PokemonSpecies, PokemonData, toPokemon, Move as MoveData } from "@/core/models";
import {
  Ability,
  BattleTurn,
  EggGroup,
  GenderRatio,
  GrowthRate,
  MoveEffect,
  MoveName,
  PokemonType,
} from "@/core/enums";
import { calculateTypeEffectivenessMultiplier } from "@/engine/battle/battle/damage-calculation";
import { applyMoveEffect } from "@/engine/battle/battle/move-effects";
import type { Battle } from "@/engine/battle/battle/battle-logic";
import Fraction from "fraction.js";

jest.mock("@/engine/battle/battle/damage-calculation", () => {
  const actual = jest.requireActual("@/engine/battle/battle/damage-calculation");
  const Fraction = require("fraction.js");
  return {
    ...actual,
    calculateDamage: jest.fn(() => ({
      damage: 0,
      type_multiplier: new Fraction(1),
    })),
    calculateTypeEffectivenessMultiplier: jest.fn(() => new Fraction(1)),
  };
});

jest.mock("@/engine/battle/battle/move-effects", () => ({
  applyMoveEffect: jest.fn(),
}));

const DEFAULT_SPECIES_BASE_STATS = {
  hp: 50,
  attack: 50,
  defense: 50,
  speed: 50,
  special_attack: 50,
  special_defense: 50,
};

const baseSpecies: PokemonSpecies = {
  id: "TEST",
  int_id: 1,
  base_stats: { ...DEFAULT_SPECIES_BASE_STATS },
  type1: PokemonType.NORMAL,
  type2: PokemonType.NORMAL,
  catch_rate: 255,
  base_exp: 1,
  gender_ratio: GenderRatio.GENDER_F50,
  unknown1: 0,
  step_cycles_to_hatch: 0,
  unknown2: 0,
  growth_rate: GrowthRate.GROWTH_MEDIUM_FAST,
  egg_group1: EggGroup.EGG_MONSTER,
  egg_group2: EggGroup.EGG_MONSTER,
  tmhm_learnset: [],
  ability: Ability.NONE,
  pic_size: 0,
  front_pic: 0,
  back_pic: 0,
};

const basePokemonData: Partial<PokemonData> = {
  nickname: "TEST",
  level: 50,
  hp: 100,
  max_hp: 100,
  original_trainer_name: "PLAYER",
  original_trainer_id: 1,
  experience: 0,
  happiness: 70,
};

const createPokemon = (species: PokemonSpecies) =>
  toPokemon(
    PokemonSchema.parse({
      ...basePokemonData,
      species,
    })
  );

describe("executeMove", () => {
  it("should pass defender types as an array when checking immunities", () => {
    const attacker = createPokemon(baseSpecies);
    const defender = createPokemon(baseSpecies);

    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.predefinedRandomValue = 0;

    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);

    const move: MoveData = {
      name: MoveName.TACKLE,
      type: PokemonType.NORMAL,
      power: 40,
      accuracy: 100,
      pp: 35,
      effect: MoveEffect.NORMAL_HIT,
      effect_chance: 0,
    };

    const battle = {
      context,
      gameState,
      eventManager,
      movesMap: new Map([[MoveName.TACKLE, move]]),
    } as unknown as Battle;

    expect(() => executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE)).not.toThrow();
    expect(calculateTypeEffectivenessMultiplier).toHaveBeenCalledWith(move.type, [PokemonType.NORMAL]);
    expect(applyMoveEffect).toHaveBeenCalled();
  });

  it("does not show effectiveness text for neutral damage", () => {
    const attacker = createPokemon(baseSpecies);
    const defender = createPokemon(baseSpecies);
    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.predefinedRandomValue = 0;
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const showTextMessages: string[] = [];
    eventManager.on("show_text", (event) => {
      const text = String(event.data.text ?? "");
      showTextMessages.push(text);
    });

    const move: MoveData = {
      name: MoveName.TACKLE,
      type: PokemonType.NORMAL,
      power: 40,
      accuracy: 100,
      pp: 35,
      effect: MoveEffect.NORMAL_HIT,
      effect_chance: 0,
    };

    const battle = {
      context,
      gameState,
      eventManager,
      movesMap: new Map([[MoveName.TACKLE, move]]),
    } as unknown as Battle;

    const damageModule = jest.requireMock("@/engine/battle/battle/damage-calculation");
    (damageModule.calculateDamage as jest.Mock).mockReturnValue({
      damage: 0,
      type_multiplier: new Fraction(1),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(showTextMessages).not.toContain("It's not very\neffective…");
    expect(showTextMessages).not.toContain("It's super-\neffective!");
    expect(showTextMessages).not.toContain(`It doesn't affect\n${defender.nickname}!`);
  });

  it("shows not very effective text when the multiplier is below 1", () => {
    const attacker = createPokemon(baseSpecies);
    const defender = createPokemon(baseSpecies);
    const context = new BattleContext([attacker], [defender], attacker, defender, undefined, false, undefined, 0);
    context.predefinedRandomValue = 0;
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const showTextMessages: string[] = [];
    eventManager.on("show_text", (event) => {
      const text = String(event.data.text ?? "");
      showTextMessages.push(text);
    });

    const move: MoveData = {
      name: MoveName.TACKLE,
      type: PokemonType.NORMAL,
      power: 40,
      accuracy: 100,
      pp: 35,
      effect: MoveEffect.NORMAL_HIT,
      effect_chance: 0,
    };

    const battle = {
      context,
      gameState,
      eventManager,
      movesMap: new Map([[MoveName.TACKLE, move]]),
    } as unknown as Battle;

    const damageModule = jest.requireMock("@/engine/battle/battle/damage-calculation");
    (damageModule.calculateDamage as jest.Mock).mockReturnValue({
      damage: 0,
      type_multiplier: new Fraction(1, 2),
    });

    executeMove(battle, BattleTurn.PLAYER, attacker, defender, MoveName.TACKLE);

    expect(showTextMessages).toContain("It's not very\neffective…");
  });
});
