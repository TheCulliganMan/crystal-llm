import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createPokemon, PokemonSpeciesSchema } from "@pokecrystal/core/core/models/pokemon";
import { BattleTurn, MoveName, BattleActionType, PokemonType, GrowthRate, GenderRatio, EggGroup, Ability } from "@pokecrystal/core/core/enums";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { Battle } from "./battle-logic";
import { BattleContext } from "./battle-context";
import { determineTurnOrder } from "./turn-order";

const buildSpecies = (speed = 100) =>
  PokemonSpeciesSchema.parse({
    id: "CHIKORITA",
    int_id: 152,
    base_stats: {
      hp: 45,
      attack: 49,
      defense: 65,
      speed,
      special_attack: 49,
      special_defense: 65,
    },
    type1: PokemonType.NORMAL,
    type2: PokemonType.NORMAL,
    catch_rate: 45,
    base_exp: 64,
    gender_ratio: GenderRatio.GENDER_F50,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: GrowthRate.GROWTH_MEDIUM_SLOW,
    egg_group1: EggGroup.EGG_MONSTER,
    egg_group2: EggGroup.EGG_MONSTER,
    tmhm_learnset: [],
    evolutions: null,
    ability: Ability.NONE,
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    front_palette: 0,
  });

const buildBattle = (
  playerSpeed = 100,
  enemySpeed = 100,
  playerItem?: string,
  enemyItem?: string
) => {
  const gameState = createInitialGameState();
  const player = createPokemon(gameState, buildSpecies(playerSpeed), 50);
  const enemy = createPokemon(gameState, buildSpecies(enemySpeed), 50);
  player.item = playerItem;
  enemy.item = enemyItem;

  const context = new BattleContext(
    [player],
    [enemy],
    player,
    enemy,
    undefined,
    false,
    undefined,
    0
  );
  const eventManager = new EventManager(gameState);
  context.playerAction = {
    actionType: BattleActionType.MOVE,
    moveName: MoveName.TACKLE,
  };
  context.enemyAction = {
    actionType: BattleActionType.MOVE,
    moveName: MoveName.TACKLE,
  };

  const battle = ({ gameState, eventManager, context } as unknown) as Battle;
  return { battle, context, player, enemy };
};

describe("determineTurnOrder", () => {
  it("uses QUICK_CLAW when player holds QUICK_CLAW script key", () => {
    const { battle, context } = buildBattle(100, 100, "QUICK_CLAW");

    const rngSpy = jest
      .spyOn(HardwareRNG.prototype, "randrange")
      .mockReturnValueOnce(0);

    expect(determineTurnOrder(battle)).toEqual([BattleTurn.PLAYER, BattleTurn.ENEMY]);
    expect(context.playerQuickClawActivated).toBe(true);
    expect(context.enemyQuickClawActivated).toBe(false);

    rngSpy.mockRestore();
  });

  it("handles script-name item keys for both sides", () => {
    const { battle, context } = buildBattle(100, 50, "QUICK_CLAW", "QUICK_CLAW");
    const rngSpy = jest
      .spyOn(HardwareRNG.prototype, "randrange")
      .mockReturnValueOnce(0)
      .mockReturnValueOnce(0);

    expect(determineTurnOrder(battle)).toEqual([BattleTurn.PLAYER, BattleTurn.ENEMY]);
    expect(context.playerQuickClawActivated).toBe(true);
    expect(context.enemyQuickClawActivated).toBe(true);

    rngSpy.mockRestore();
  });

  it("falls back to speed when no quick claw triggers", () => {
    const { battle, context } = buildBattle(40, 80);

    expect(determineTurnOrder(battle)).toEqual([BattleTurn.ENEMY, BattleTurn.PLAYER]);
    expect(context.playerQuickClawActivated).toBe(false);
    expect(context.enemyQuickClawActivated).toBe(false);
  });
});
