import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { MoveName } from "@pokecrystal/core/core/enums";
import { Battle } from "@pokecrystal/core/engine/battle/battle/battle-logic";
import { BattleMenu } from "@pokecrystal/core/ui/overlays/_battle-menu";
import { BattleUIPhase, type BattleUIState } from "@pokecrystal/core/ui/overlays/battle-ui-state";
import { BattleStateEnum, type BattleAction } from "@pokecrystal/core/engine/battle/battle/battle-context";
import { MultiplayerBattle } from "./multiplayer-battle";
import type { BattleSyncMessage, BattleSyncTransport } from "./battle-synchronizer";

jest.mock("@pokecrystal/core/engine/battle/battle/move-execution", () => ({
  executeMove: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/overlays/battle-ui-core", () => {
  const actual = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-core");
  return {
    ...actual,
    should_block_state_advance: jest.fn(() => false),
  };
});

jest.mock("@pokecrystal/core/ui/overlays/battle-ui-input", () => {
  const actual = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-input");
  return {
    ...actual,
    get_player_input: jest.fn(() => null),
  };
});

function makeTransportPair(): [BattleSyncTransport, BattleSyncTransport] {
  const aCbs: Array<(m: BattleSyncMessage) => void> = [];
  const bCbs: Array<(m: BattleSyncMessage) => void> = [];

  const a: BattleSyncTransport = {
    send(message) {
      bCbs.forEach((cb) => cb(message));
    },
    onData(cb) {
      aCbs.push(cb);
    },
    offData(cb) {
      const idx = aCbs.indexOf(cb);
      if (idx !== -1) aCbs.splice(idx, 1);
    },
  };

  const b: BattleSyncTransport = {
    send(message) {
      aCbs.forEach((cb) => cb(message));
    },
    onData(cb) {
      bCbs.push(cb);
    },
    offData(cb) {
      const idx = bCbs.indexOf(cb);
      if (idx !== -1) bCbs.splice(idx, 1);
    },
  };

  return [a, b];
}

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

const buildBattleUi = (): BattleUIState =>
  ({
    is_mock: false,
    wram: {
      current_menu: BattleMenu.MAIN,
      last_party_size: 0,
      wPartyMenuCursorPosition: 0,
      wBattleMenuCursorPosition: 0,
      wMoveMenuCursorPosition: 0,
      swapping_move_index: null,
      confirm_pressed: false,
      select_pressed: false,
      last_num_moves: 0,
      last_item_names: [],
    },
    yes_no_prompt: {
      active: false,
      selection: 0,
      result: null,
      pending_activation: false,
    },
    dialogue: {
      forced_visible: false,
      pending_waits: 0,
      queue: [],
      dialogue: { is_complete: () => true },
    },
    force_party_menu: false,
    pending_pokemon_selection: null,
    ui_phase: BattleUIPhase.MENU,
    trainer_intro: null,
    trainer_exit: null,
    pending_trainer_exit: false,
  }) as unknown as BattleUIState;

function buildBattle() {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  playerPokemon.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
  const battleUi = buildBattleUi();
  const movesMap = new Map<MoveName, any>();
  const battle = new Battle(playerPokemon, enemyPokemon, gameState, eventManager, battleUi, movesMap);
  battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
  return { battle, gameState };
}

describe("MultiplayerBattle (integration)", () => {
  test("exchanges actions and advances past ENEMY_ACTION_SELECT without invoking AI", async () => {
    const [tA, tB] = makeTransportPair();
    const a = buildBattle();
    const b = buildBattle();

    const wrapperA = new MultiplayerBattle({
      battle: a.battle,
      transport: tA,
      isHost: true,
      gameState: a.gameState,
    });
    const wrapperB = new MultiplayerBattle({
      battle: b.battle,
      transport: tB,
      isHost: false,
      gameState: b.gameState,
    });

    await Promise.all([wrapperA.initRng(), wrapperB.initRng()]);

    const actionA: BattleAction = { actionType: "move", moveName: MoveName.TACKLE };
    const actionB: BattleAction = { actionType: "run" };
    a.battle.context.playerAction = actionA;
    b.battle.context.playerAction = actionB;

    wrapperA.update();
    wrapperB.update();

    expect(a.battle.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(b.battle.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);

    // Next update should apply the remote action and allow Battle to advance.
    wrapperA.update();
    wrapperB.update();

    expect(a.battle.context.currentState).not.toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(b.battle.context.currentState).not.toBe(BattleStateEnum.ENEMY_ACTION_SELECT);

    wrapperA.destroy();
    wrapperB.destroy();
  });
});

