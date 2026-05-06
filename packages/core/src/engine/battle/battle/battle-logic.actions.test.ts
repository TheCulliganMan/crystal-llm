import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { Battle } from "./battle-logic";
import { BattleActionType, BattleStateEnum } from "./battle-context";
import { BattleTurn, MoveName, Stat } from "@pokecrystal/core/core/enums";
import { BattleItemTimeline } from "./item-timeline";
import { attemptRun } from "./flee-logic";
import { applyItemEffect } from "./item-effects";
import { finaliseBattle } from "./battle-finalization";
import { should_block_state_advance } from "@pokecrystal/core/ui/overlays/battle-ui-core";

jest.mock("./flee-logic", () => {
  const actual = jest.requireActual("./flee-logic");
  return {
    ...actual,
    attemptRun: jest.fn(),
  };
});

jest.mock("./item-effects", () => {
  const actual = jest.requireActual("./item-effects");
  return {
    ...actual,
    applyItemEffect: jest.fn(),
  };
});

jest.mock("./battle-finalization", () => {
  const actual = jest.requireActual("./battle-finalization");
  return {
    ...actual,
    finaliseBattle: jest.fn(),
  };
});

jest.mock("@pokecrystal/core/ui/overlays/battle-ui-core", () => {
  const actual = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-core");
  return {
    ...actual,
    should_block_state_advance: jest.fn(),
  };
});

const buildSpecies = () =>
  PokemonSpeciesSchema.parse({
    id: "CHIKORITA",
    int_id: 152,
    base_stats: {
      hp: 45,
      attack: 49,
      defense: 65,
      speed: 45,
      special_attack: 49,
      special_defense: 65,
    },
    type1: "GRASS",
    type2: "GRASS",
    catch_rate: 45,
    base_exp: 64,
    gender_ratio: 31,
    unknown1: 0,
    step_cycles_to_hatch: 20,
    unknown2: 0,
    growth_rate: "GROWTH_MEDIUM_SLOW",
    egg_group1: "EGG_MONSTER",
    egg_group2: "EGG_PLANT",
  });

const buildBattle = (playerParty?: ReturnType<typeof createPokemon>[]) => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  const movesMap = new Map<MoveName, any>();
  const battle = new Battle(
    playerPokemon,
    enemyPokemon,
    gameState,
    eventManager,
    null as any,
    movesMap,
    null,
    undefined,
    playerParty ?? [playerPokemon],
  );
  return { battle, gameState, eventManager, playerPokemon, enemyPokemon };
};

describe("Battle action execution", () => {
  beforeEach(() => {
    (attemptRun as jest.Mock).mockReset();
    (applyItemEffect as jest.Mock).mockReset();
    (finaliseBattle as jest.Mock).mockReset();
    (should_block_state_advance as jest.Mock).mockReset();
  });

  it("ends the battle when RUN succeeds", () => {
    const { battle } = buildBattle();
    (attemptRun as jest.Mock).mockReturnValue(true);
    battle.context.playerAction = { actionType: BattleActionType.RUN };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(attemptRun).toHaveBeenCalledWith(battle);
    expect(battle._playerRan).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);
  });

  it("keeps the battle running when RUN fails", () => {
    const { battle } = buildBattle();
    (attemptRun as jest.Mock).mockReturnValue(false);
    battle.context.playerAction = { actionType: BattleActionType.RUN };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(attemptRun).toHaveBeenCalledWith(battle);
    expect(battle._playerRan).toBe(false);
    expect(battle.context.currentState).toBe(BattleStateEnum.POST_TURN_EFFECTS);
  });

  it("blocks RUN selection in trainer battles", () => {
    const { battle } = buildBattle();
    (attemptRun as jest.Mock).mockReturnValue(false);
    battle.context.trainerBattle = true;
    battle.pendingPlayerAction = { actionType: BattleActionType.RUN };
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;

    battle.update();

    expect(attemptRun).toHaveBeenCalledWith(battle);
    expect(battle.context.playerAction).toBeUndefined();
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
  });

  it("switches to the requested party slot", () => {
    const { battle, playerPokemon, gameState } = buildBattle();
    const species = buildSpecies();
    const reserve = createPokemon(gameState, species, 5);
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerPokemon = playerPokemon;
    battle.context.playerActiveIndex = 0;
    playerPokemon.rage_active = true;
    playerPokemon.rage_counter = 2;
    battle.context.playerAction = {
      actionType: BattleActionType.SWITCH,
      switchToPokemonIndex: 1,
    };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(battle.context.playerActiveIndex).toBe(1);
    expect(battle.context.playerPokemon).toBe(reserve);
    expect(playerPokemon.rage_active).toBe(false);
    expect(playerPokemon.rage_counter).toBe(0);
  });

  it("resets the outgoing Pokemon stat stages when switching", () => {
    const { battle, playerPokemon, gameState } = buildBattle();
    const species = buildSpecies();
    const reserve = createPokemon(gameState, species, 5);
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerPokemon = playerPokemon;
    battle.context.playerActiveIndex = 0;
    playerPokemon.stat_boosts[Stat.DEFENSE] = -2;
    playerPokemon.stat_boosts[Stat.ACCURACY] = -1;
    battle.context.playerAction = {
      actionType: BattleActionType.SWITCH,
      switchToPokemonIndex: 1,
    };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(battle.context.playerActiveIndex).toBe(1);
    expect(playerPokemon.stat_boosts[Stat.DEFENSE]).toBe(0);
    expect(playerPokemon.stat_boosts[Stat.ACCURACY]).toBe(0);
  });

  it("restores Ditto's original battle data when switching out after Transform", () => {
    const { battle, playerPokemon, gameState } = buildBattle();
    const species = buildSpecies();
    const reserve = createPokemon(gameState, species, 5);
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerPokemon = playerPokemon;
    battle.context.playerActiveIndex = 0;
    playerPokemon.transformed = true;
    playerPokemon.original_species = { ...playerPokemon.species, id: "DITTO" };
    playerPokemon.transform_backup_dvs = { attack: 1, defense: 2, speed: 3, special: 4, hp: 5 };
    playerPokemon.transform_backup_moves = [{ name: MoveName.TRANSFORM, current_pp: 9 }];
    playerPokemon.transform_backup_stat_boosts = {
      HP: 0,
      ATTACK: 0,
      DEFENSE: 0,
      SPEED: 0,
      SPECIAL_ATTACK: 0,
      SPECIAL_DEFENSE: 0,
      ACCURACY: 0,
      EVASION: 0,
    };
    playerPokemon.transform_backup_stats = {
      attack: 10,
      defense: 11,
      speed: 12,
      special_attack: 13,
      special_defense: 14,
    };
    playerPokemon.species = { ...playerPokemon.species, id: "MEW" };
    playerPokemon.moves = [{ name: MoveName.PSYCHIC_M, current_pp: 5 }];
    playerPokemon.stat_boosts.ATTACK = 3;
    battle.context.playerAction = {
      actionType: BattleActionType.SWITCH,
      switchToPokemonIndex: 1,
    };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(battle.context.playerActiveIndex).toBe(1);
    expect(playerPokemon.transformed).toBe(false);
    expect(playerPokemon.species.id).toBe("DITTO");
    expect(playerPokemon.moves).toEqual([{ name: MoveName.TRANSFORM, current_pp: 9 }]);
    expect(playerPokemon.dvs).toEqual({ attack: 1, defense: 2, speed: 3, special: 4, hp: 5 });
    expect(playerPokemon.stat_boosts.ATTACK).toBe(0);
    expect(playerPokemon.transform_backup_moves).toBeUndefined();
  });

  it("refuses to switch a trapped pokemon", () => {
    const { battle, playerPokemon, eventManager, gameState } = buildBattle();
    const species = buildSpecies();
    const reserve = createPokemon(gameState, species, 5);
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerPokemon = playerPokemon;
    battle.context.playerActiveIndex = 0;
    playerPokemon.trapped_turns = 2;
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    battle.context.playerAction = {
      actionType: BattleActionType.SWITCH,
      switchToPokemonIndex: 1,
    };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(battle.context.playerActiveIndex).toBe(0);
    expect(dispatchSpy).toHaveBeenCalled();
  });

  it("auto-switches the enemy instead of ending the battle when reserves remain", () => {
    const { battle, enemyPokemon, gameState } = buildBattle();
    const species = buildSpecies();
    const reserve = createPokemon(gameState, species, 5);
    reserve.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
    battle.context.enemyParty = [enemyPokemon, reserve];
    battle.context.enemyPokemon = enemyPokemon;
    battle.context.enemyActiveIndex = 0;
    enemyPokemon.hp = 0;
    battle.context.enemyAction = undefined;
    battle.context.currentState = BattleStateEnum.ENEMY_ACTION_SELECT;

    battle.update();

    expect(battle.context.enemyActiveIndex).toBe(1);
    expect(battle.context.enemyPokemon).toBe(reserve);
    expect(battle.context.currentState).toBe(BattleStateEnum.PRE_TURN_EFFECTS);
  });

  it("consumes items and applies effects", () => {
    const { battle, gameState } = buildBattle();
    gameState.sram.items.POTION = 1;
    const item = battle._itemSystem?.getItemDefinition("POTION");
    if (!item) {
      throw new Error("Missing POTION definition for test.");
    }
    (applyItemEffect as jest.Mock).mockReturnValue(false);
    battle._itemTimeline = new BattleItemTimeline(battle.eventManager, 0);
    battle.context.playerAction = { actionType: BattleActionType.ITEM, item };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(applyItemEffect).toHaveBeenCalled();
    expect(gameState.sram.items.POTION ?? 0).toBe(0);
    expect(battle.context.currentState).toBe(BattleStateEnum.POST_TURN_EFFECTS);
  });

  it("rejects items not in the bag", () => {
    const { battle, eventManager } = buildBattle();
    const item = battle._itemSystem?.getItemDefinition("POTION");
    if (!item) {
      throw new Error("Missing POTION definition for test.");
    }
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    battle.context.playerAction = { actionType: BattleActionType.ITEM, item };
    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(applyItemEffect).not.toHaveBeenCalled();
    expect(dispatchSpy).toHaveBeenCalled();
  });

  it("waits to finalize when battle UI is blocking at battle end", () => {
    const { battle } = buildBattle();
    battle.battleUi = { pending_evolutions: [] } as any;
    (should_block_state_advance as jest.Mock).mockReturnValue(true);
    battle.context.currentState = BattleStateEnum.BATTLE_END;

    battle.update();

    expect(finaliseBattle).not.toHaveBeenCalled();
    expect(battle._finalised).toBe(false);
  });
});
