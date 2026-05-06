import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { BattleTurn, MoveName } from "@pokecrystal/core/core/enums";
import { Battle } from "./battle-logic";
import { BattleStateEnum, BattleActionType } from "./battle-context";
import { BattleMenu } from "@pokecrystal/core/ui/overlays/_battle-menu";
import { BattleUIPhase, type BattleUIState } from "@pokecrystal/core/ui/overlays/battle-ui-state";
import * as debugLog from "@pokecrystal/core/core/debug-log";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { AUTO_INPUT, DudeAutoInputController } from "@pokecrystal/core/engine/battle/auto-input";
import { B_PAD_A } from "@pokecrystal/core/input/controls";
import { renderTextSnapshot } from "@pokecrystal/core/ui/text-overlays";
import { Surface } from "@pokecrystal/core/ui/surface";
jest.mock("./move-execution", () => ({
  executeMove: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/overlays/battle-ui-render", () => ({
  update: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/text-overlays", () => ({
  renderTextSnapshot: jest.fn(),
}));

jest.mock("@pokecrystal/core/ui/screens/name-entry-screen", () => ({
  NameEntryScreen: jest.fn().mockImplementation(() => ({
    reset: jest.fn(),
    fillName: jest.fn(),
    update: jest.fn(),
    draw: jest.fn(),
    handleInput: jest.fn(),
    finished: false,
    name: "",
  })),
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
    ui: {
      screen: new Surface(160, 144),
      clearScreen: jest.fn(),
      update: jest.fn(),
    },
    game_state: createInitialGameState(),
    data_loader: null,
    is_mock: false,
    active: true,
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
      result: null,
      pending_activation: false,
      prompt: null,
    },
    dialogue: {
      forced_visible: false,
      pending_waits: 0,
      queue: [],
      dialogue: {
        open: jest.fn(),
        clear: jest.fn(),
        complete: jest.fn(),
        advance_page: jest.fn(),
        is_complete: () => true,
        has_more_pages: () => false,
      },
    },
    animation_player: {
      is_active: () => false,
      play_animation: jest.fn(),
    } as BattleUIState["animation_player"],
    pending_animation_events: [],
    exp_animation: null,
    manual_wait_override: false,
    waiting_for_input: false,
    force_party_menu: false,
    pending_pokemon_selection: null,
    ui_phase: BattleUIPhase.MENU,
    block_on_pending_evolution: false,
    block_on_move_learning: false,
    pending_evolutions: [],
    pending_move_learns: [],
    active_evolution: null,
    active_move_learn: null,
    evolution_animation: null,
    trainer_intro: null,
    trainer_exit: null,
    pending_trainer_exit: false,
    frontpic_animation: null,
    trainer_sprites_visible: false,
    trainer_send_out_seen: false,
    trainer_sprite_override_mode: null,
    sprites_enabled: true,
  }) as unknown as BattleUIState;

const buildBattle = (playerMove?: MoveName) => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  if (playerMove) {
    playerPokemon.moves = [{ name: playerMove, current_pp: 35 }];
  }
  const battleUi = buildBattleUi();
  const movesMap = new Map<MoveName, any>();
  const battle = new Battle(playerPokemon, enemyPokemon, gameState, eventManager, battleUi, movesMap);
  battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
  return { battle, playerPokemon };
};

const buildBattleWithType = (battleType: string) => {
  const gameState = createInitialGameState();
  gameState.wram.battle_type = battleType;
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  const battleUi = buildBattleUi();
  const movesMap = new Map<MoveName, any>();
  const battle = new Battle(
    playerPokemon,
    enemyPokemon,
    gameState,
    eventManager,
    battleUi,
    movesMap
  );
  return { battle, eventManager };
};

const buildWildBattleStart = (audioEngine?: AudioEngine | null) => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  const battleUi = buildBattleUi();
  const movesMap = new Map<MoveName, any>();
  const battle = new Battle(
    playerPokemon,
    enemyPokemon,
    gameState,
    eventManager,
    battleUi,
    movesMap,
    audioEngine ?? null
  );
  battle.context.currentState = BattleStateEnum.BATTLE_START;
  return { battle, eventManager, playerPokemon, enemyPokemon };
};

const buildTutorialBattleStart = () => {
  const gameState = createInitialGameState();
  gameState.wram.battle_type = "BATTLETYPE_TUTORIAL";
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  const enemyPokemon = createPokemon(gameState, species, 5);
  const battleUi = buildBattleUi();
  const movesMap = new Map<MoveName, any>();
  const battle = new Battle(
    playerPokemon,
    enemyPokemon,
    gameState,
    eventManager,
    battleUi,
    movesMap
  );
  battle.context.currentState = BattleStateEnum.BATTLE_START;
  return { battle, eventManager, enemyPokemon };
};

beforeEach(() => {
  const moveModule = jest.requireMock("./move-execution");
  moveModule.executeMove.mockClear();
  const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
  inputModule.get_player_input.mockReset();
  inputModule.get_player_input.mockReturnValue(null);
  const uiCore = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-core");
  uiCore.should_block_state_advance.mockReset();
  uiCore.should_block_state_advance.mockImplementation(() => false);
});

describe("Battle player action selection", () => {
  it("publishes a battle-transition text snapshot while the intro animation runs", () => {
    const { battle } = buildBattle();
    battle.context.currentState = BattleStateEnum.BATTLE_TRANSITION;
    (battle as unknown as { _transition: { draw: jest.Mock } })._transition = {
      draw: jest.fn(),
    };

    battle.draw();

    expect(renderTextSnapshot).toHaveBeenCalledWith(
      battle.battleUi.ui,
      expect.objectContaining({
        viewportTitle: "Battle",
        viewportLines: expect.arrayContaining(["BATTLE TRANSITION"]),
      }),
    );
  });

  it("throws when no usable player Pokemon is available", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = buildSpecies();
    const playerPokemon = createPokemon(gameState, species, 5);
    const enemyPokemon = createPokemon(gameState, species, 5);
    playerPokemon.hp = 0;
    const battleUi = {} as unknown as import("@pokecrystal/core/ui/overlays/battle-ui-state").BattleUIState;
    const movesMap = new Map<MoveName, any>();

    expect(() => {
      new Battle(playerPokemon, enemyPokemon, gameState, eventManager, battleUi, movesMap);
    }).toThrow("Cannot start battle without a usable player Pokemon.");
  });

  it("waits for UI input before advancing", () => {
    const { battle } = buildBattle();

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.context.playerAction).toBeUndefined();
  });

  it("accepts a move selection from the UI", () => {
    const { battle } = buildBattle(MoveName.TACKLE);
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(MoveName.TACKLE);

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(battle.context.playerAction?.actionType).toBe(BattleActionType.MOVE);
    expect(battle.context.playerAction?.moveName).toBe(MoveName.TACKLE);
  });

  it("keeps the player in move selection when choosing a disabled move", () => {
    const { battle, playerPokemon } = buildBattle(MoveName.BITE);
    playerPokemon.disabled_move = MoveName.BITE;
    playerPokemon.disable_turns = 6;
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(MoveName.BITE);
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.context.playerAction).toBeUndefined();
    expect(battle.battleUi.wram.current_menu).toBe(BattleMenu.FIGHT);
    expect(dispatchSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        name: "show_text",
        data: { text: "The move is DISABLED!" },
      }),
    );
  });

  it.each(["POTION", "FULL_HEAL", "FULL_RESTORE", "ICE_HEAL", "ETHER", "REVIVE"])(
    "opens party target selection before using %s in battle",
    (itemName) => {
      const { battle } = buildBattle();
      battle.gameState.sram.items[itemName] = 1;
      const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
      battle.context.playerParty = [battle.context.playerPokemon, reserve];
      const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
      inputModule.get_player_input.mockReturnValueOnce(itemName);

      battle.update();

      expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
      expect(battle.context.playerAction).toBeUndefined();
      expect(battle.battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
      expect(battle.battleUi.battle_item_target_selection).toBe(true);
    },
  );

  it("revives the selected party member from the battle pack", () => {
    const { battle } = buildBattle();
    battle.gameState.sram.items.REVIVE = 1;
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    reserve.max_hp = 40;
    reserve.hp = 0;
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValueOnce("REVIVE").mockReturnValueOnce(1);

    battle.update();
    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(battle.context.playerAction).toMatchObject({
      actionType: BattleActionType.ITEM,
      item: expect.objectContaining({ script_name: "REVIVE" }),
    });
    expect(battle.context.playerAction?.targetPokemon).toBe(reserve);

    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;
    (battle as any)._itemTimeline = null;

    battle.update();

    expect(reserve.hp).toBe(20);
    expect(battle.gameState.sram.items.REVIVE).toBeUndefined();
  });

  it("revives the selected party member from the battle pack during trainer battles", () => {
    const { battle } = buildBattle();
    battle.context.trainerBattle = true;
    battle.gameState.sram.items.REVIVE = 1;
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    reserve.max_hp = 42;
    reserve.hp = 0;
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValueOnce("REVIVE").mockReturnValueOnce(1);

    battle.update();
    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(battle.context.playerAction).toMatchObject({
      actionType: BattleActionType.ITEM,
      item: expect.objectContaining({ script_name: "REVIVE" }),
    });
    expect(battle.context.playerAction?.targetPokemon).toBe(reserve);

    battle.context.turnOrder = [BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;
    (battle as any)._itemTimeline = null;

    battle.update();

    expect(reserve.hp).toBe(21);
    expect(battle.gameState.sram.items.REVIVE).toBeUndefined();
  });

  it("forces a replacement instead of asking to forfeit after the player faints in a trainer battle", () => {
    const { battle, playerPokemon } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    reserve.hp = reserve.max_hp;
    battle.context.trainerBattle = true;
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = playerPokemon;
    playerPokemon.hp = 0;
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(dispatchSpy).not.toHaveBeenCalledWith(
      expect.objectContaining({ name: "prompt_yes_no" }),
    );
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect((battle as any)._awaitingFaintPrompt).toBe(false);
    expect((battle as any)._forcedPartyMenuSelection).toBe(true);
    expect(battle.battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
  });

  it("starts trainer battle forced replacement on the first usable benched Pokemon", () => {
    const { battle, playerPokemon } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    reserve.hp = reserve.max_hp;
    battle.context.trainerBattle = true;
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = playerPokemon;
    battle.battleUi.wram.wPartyMenuCursorPosition = 0;
    playerPokemon.hp = 0;

    battle.update();

    expect(battle.battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
    expect(battle.battleUi.wram.wPartyMenuCursorPosition).toBe(1);
  });

  it("enters replacement selection from post-turn trainer battle faints", () => {
    const { battle, playerPokemon } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    reserve.hp = reserve.max_hp;
    battle.context.trainerBattle = true;
    battle.context.playerParty = [playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = playerPokemon;
    battle.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
    battle.context.playerAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.enemyAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    playerPokemon.hp = 0;

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.context.playerAction).toBeUndefined();
    expect(battle.context.enemyAction).toBeUndefined();
    expect((battle as any)._forcedPartyMenuSelection).toBe(true);
    expect(battle.battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
    expect(battle.battleUi.wram.wPartyMenuCursorPosition).toBe(1);
  });

  it("logs the selected move name", () => {
    const { battle, playerPokemon } = buildBattle(MoveName.TACKLE);
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(MoveName.TACKLE);
    const logSpy = jest.spyOn(debugLog, "pushDebugLog").mockImplementation(() => {});

    battle.update();

    expect(logSpy).toHaveBeenCalledWith(
      expect.stringContaining("[battle] player action move"),
      expect.objectContaining({
        move: MoveName.TACKLE,
        pokemon: playerPokemon.nickname,
      })
    );
    logSpy.mockRestore();
  });

  it("rejects switching to the active Pokemon", () => {
    const { battle, playerPokemon } = buildBattle();
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(0);
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.context.playerAction).toBeUndefined();
    expect(dispatchSpy).toHaveBeenCalled();
    const [event] = dispatchSpy.mock.calls[0];
    expect(event.name).toBe("show_text");
    expect(event.data.text).toBe(`${playerPokemon.nickname} is already out.`);
  });

  it("rejects switching to a fainted Pokemon", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    reserve.hp = 0;
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(1);
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.context.playerAction).toBeUndefined();
    const textEvents = dispatchSpy.mock.calls.filter(([event]) => event.name === "show_text");
    expect(textEvents.length).toBeGreaterThan(0);
    expect(textEvents[0][0].data.text).toBe("There's no will to battle!");
  });

  it("maps SRAM party selection indices to battle party indices", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = buildSpecies();
    const firstSlot = createPokemon(gameState, species, 5);
    const secondSlot = createPokemon(gameState, species, 5);
    gameState.sram.party.pokemon = [firstSlot, secondSlot];
    const enemyPokemon = createPokemon(gameState, species, 5);
    const battleUi = buildBattleUi();
    const movesMap = new Map<MoveName, any>();
    const battle = new Battle(
      secondSlot,
      enemyPokemon,
      gameState,
      eventManager,
      battleUi,
      movesMap,
    );
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;

    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(0);

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.ENEMY_ACTION_SELECT);
    expect(battle.context.playerAction?.actionType).toBe(BattleActionType.SWITCH);
    expect(battle.context.playerAction?.switchToPokemonIndex).toBe(1);
  });

  it("does not synthesize the first available party slot when forcing a switch menu", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.battleUi.wram.wPartyMenuCursorPosition = 1;

    (battle as any)._force_party_menu_selection();

    expect(battle.battleUi.force_party_menu).toBe(true);
    expect(battle.battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
    expect(battle.battleUi.wram.last_party_size).toBe(2);
    expect(battle.battleUi.wram.wPartyMenuCursorPosition).toBe(1);
  });

  it("resets an invalid stored party cursor to the first row when forcing a switch menu", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.battleUi.wram.wPartyMenuCursorPosition = 7;

    (battle as any)._force_party_menu_selection();

    expect(battle.battleUi.wram.wPartyMenuCursorPosition).toBe(0);
  });
});

describe("Battle nickname prompts", () => {
  it("skips nickname prompts during tutorial catches so the battle can finish", () => {
    const { battle, eventManager } = buildBattleWithType("BATTLETYPE_TUTORIAL");
    const completeSpy = jest.fn();
    eventManager.on("battle_complete", completeSpy);

    (battle as any)._onItemComplete({} as any, true);
    battle.update();

    expect((battle as any)._nicknamePromptPending).toBe(false);
    expect(battle.isFinished()).toBe(true);
    expect(completeSpy).toHaveBeenCalledTimes(1);
  });

  it("skips nickname prompts during bug contest catches", () => {
    const { battle, eventManager } = buildBattleWithType("BATTLETYPE_BUG_CONTEST");
    const completeSpy = jest.fn();
    eventManager.on("battle_complete", completeSpy);

    (battle as any)._onItemComplete({} as any, true);
    battle.update();

    expect((battle as any)._nicknamePromptPending).toBe(false);
    expect(battle.isFinished()).toBe(true);
    expect(completeSpy).toHaveBeenCalledTimes(1);
  });

  it("keeps nickname prompts enabled for standard catches", () => {
    const { battle } = buildBattleWithType("BATTLETYPE_NORMAL");

    (battle as any)._onItemComplete({} as any, true);

    expect((battle as any)._nicknamePromptPending).toBe(true);
  });

  it("runs the naming screen without blocking battle draw", () => {
    const { battle } = buildBattleWithType("BATTLETYPE_NORMAL");
    const captured = createPokemon(battle.gameState, buildSpecies(), 5);
    const renderModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-render");
    const nameEntryModule = jest.requireMock("@pokecrystal/core/ui/screens/name-entry-screen");
    const screen = {
      reset: jest.fn(),
      fillName: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      handleInput: jest.fn(),
      finished: false,
      name: "",
    };
    renderModule.update.mockClear();
    nameEntryModule.NameEntryScreen.mockClear();
    nameEntryModule.NameEntryScreen.mockImplementationOnce(() => screen);

    battle.battleUi.pending_nickname_request = {
      pokemon: captured,
      species_name: captured.species.id,
    };
    battle.battleUi.yes_no_prompt.result = true;
    battle.battleUi.yes_no_prompt.active = false;
    (battle as any)._nicknamePromptPending = true;
    (battle as any)._nicknamePromptShown = false;

    battle.update();

    expect((battle as any)._activeNicknameScreen).toBeTruthy();
    expect(nameEntryModule.NameEntryScreen).toHaveBeenCalled();
    const inputEvent = { type: "keydown", key: "a" } as unknown;
    battle.handle_input(inputEvent as any);
    expect(screen.handleInput).toHaveBeenCalledWith(inputEvent);

    battle.draw();
    expect(() => battle.draw()).not.toThrow();
    expect(renderModule.update).not.toHaveBeenCalled();

    screen.name = "SPARK";
    screen.finished = true;
    battle.update();

    expect(captured.nickname).toBe("SPARK");
    expect((battle as any)._activeNicknameScreen).toBeNull();

    battle.draw();
    expect(() => battle.draw()).not.toThrow();
  });

  it("commits a finished nickname screen immediately from input handling", () => {
    const { battle } = buildBattleWithType("BATTLETYPE_NORMAL");
    const captured = createPokemon(battle.gameState, buildSpecies(), 5);
    const nameEntryModule = jest.requireMock("@pokecrystal/core/ui/screens/name-entry-screen");
    const screen = {
      reset: jest.fn(),
      fillName: jest.fn(),
      update: jest.fn(),
      draw: jest.fn(),
      handleInput: jest.fn(() => {
        screen.name = "PIDER";
        screen.finished = true;
      }),
      finished: false,
      name: "",
    };
    nameEntryModule.NameEntryScreen.mockClear();
    nameEntryModule.NameEntryScreen.mockImplementationOnce(() => screen);

    battle.battleUi.pending_nickname_request = {
      pokemon: captured,
      species_name: captured.species.id,
    };
    battle.battleUi.yes_no_prompt.result = true;
    battle.battleUi.yes_no_prompt.active = false;
    (battle as any)._nicknamePromptPending = true;
    (battle as any)._nicknamePromptShown = false;

    battle.update();
    battle.handle_input({ type: "keydown", key: "Enter" } as any);

    expect(captured.nickname).toBe("PIDER");
    expect((battle as any)._activeNicknameScreen).toBeNull();
  });
});

describe("Battle delayed item application", () => {
  it("advances delayed item events during update ticks even while UI state advance is blocked", () => {
    const { battle } = buildBattleWithType("BATTLETYPE_NORMAL");
    const uiCore = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-core");
    uiCore.should_block_state_advance.mockImplementation(() => true);

    const applySpy = jest
      .spyOn(battle as any, "_applyQueuedItem")
      .mockReturnValue(true);
    const completeSpy = jest.spyOn(battle as any, "_onItemComplete");

    (battle as any)._queueBattleItem(
      BattleTurn.PLAYER,
      battle.context.playerPokemon,
      {
        actionType: BattleActionType.ITEM,
        item: { name: "POKE_BALL", script_name: "POKE_BALL" },
      },
    );

    expect((battle as any)._itemTimeline.applying).toBe(true);

    for (let i = 0; i < 12; i += 1) {
      battle.update();
    }

    expect(applySpy).toHaveBeenCalledTimes(1);
    expect(completeSpy).toHaveBeenCalledTimes(1);
    expect((battle as any)._itemTimeline.applying).toBe(false);
  });
});

describe("Battle tutorial input", () => {
  it("flags tutorial animations for fast-forward when A is pressed", () => {
    const { battle } = buildBattleWithType("BATTLETYPE_TUTORIAL");
    const battleUi = battle.battleUi as BattleUIState;
    battleUi.fast_animation_request = false;
    battle._autoInput = new DudeAutoInputController();
    battle.gameState.wram.wInputType = AUTO_INPUT;
    const joypad = battle.gameState.hram.joypad;
    joypad.hJoyPressed = B_PAD_A;

    (battle as any)._applyAutoInput();

    expect(battleUi.fast_animation_request).toBe(true);
  });

  it("clears stale tutorial auto input flags instead of crashing normal wild battles", () => {
    const { battle } = buildBattle();
    battle.gameState.wram.wInputType = AUTO_INPUT;

    expect(() => (battle as any)._applyAutoInput()).not.toThrow();
    expect(battle.gameState.wram.wInputType).toBe(0);
    expect((battle as any)._autoInputActive).toBe(false);
  });
});

describe("Battle faint prompts", () => {
  it("prompts to use the next Pokemon when the player faints", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "prompt_yes_no")).toBe(true);
    expect(battle._awaitingFaintPrompt).toBe(true);
  });

  it("forces the party menu after accepting the faint prompt", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;

    battle.update();

    const battleUi = battle.battleUi as BattleUIState;
    battleUi.yes_no_prompt.result = true;
    battleUi.yes_no_prompt.active = false;

    battle.update();

    expect(battleUi.force_party_menu).toBe(true);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
  });

  it("keeps post-turn faint prompts from falling back to action select in the same tick", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;

    battle.update();

    expect(battle._awaitingFaintPrompt).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.POST_TURN_EFFECTS);
    expect(battle.context.playerAction).toBeUndefined();
    expect(battle.context.enemyAction).toBeUndefined();
  });

  it("queues the player faint handoff before clearing a stale UI wait", () => {
    const { battle } = buildBattle();
    battle.context.playerParty = [battle.context.playerPokemon];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
    const battleUi = battle.battleUi as BattleUIState;
    battleUi.waiting_for_input = true;
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "show_text" && event.data.text === `${battle.context.playerPokemon.nickname} fainted!`)).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);
    expect(battle._activeFaintSides.has(BattleTurn.PLAYER)).toBe(true);
  });

  it("prompts to use the next Pokemon when the player is fainted on action select", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");

    battle.update();

    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "prompt_yes_no")).toBe(true);
    expect(battle._awaitingFaintPrompt).toBe(true);
  });

  it("recovers action select when the enemy is already fainted", () => {
    const { battle } = buildBattle(MoveName.TACKLE);
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");
    battle.context.enemyPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;

    battle.update();

    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "show_text" && event.data.text === `Enemy ${battle.context.enemyPokemon.nickname} fainted!`)).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);
    expect(battle._activeFaintSides.has(BattleTurn.ENEMY)).toBe(true);
  });

  it("switches immediately after selecting a replacement for a fainted player Pokemon", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
    (battle as any)._forcedPartyMenuSelection = true;
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(1);

    battle.update();

    expect(battle.context.playerActiveIndex).toBe(1);
    expect(battle.context.playerPokemon).toBe(reserve);
    expect(battle.context.playerAction).toBeUndefined();
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect((battle as any)._forcedPartyMenuSelection).toBe(false);
  });

  it("consumes the forced party selection after accepting the post-turn faint prompt", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.POST_TURN_EFFECTS;
    battle.context.playerAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.enemyAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };

    battle.update();

    const battleUi = battle.battleUi as BattleUIState;
    battleUi.yes_no_prompt.result = true;
    battleUi.yes_no_prompt.active = false;

    battle.update();

    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValue(1);

    battle.update();

    expect(battle.context.playerActiveIndex).toBe(1);
    expect(battle.context.playerPokemon).toBe(reserve);
    expect(battle.context.playerAction).toBeUndefined();
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect((battle as any)._forcedPartyMenuSelection).toBe(false);
  });

  it("keeps the party menu forced after rejecting a fainted selection", () => {
    const { battle } = buildBattle();
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    const fainted = createPokemon(battle.gameState, buildSpecies(), 5);
    fainted.hp = 0;
    battle.context.playerParty = [battle.context.playerPokemon, reserve, fainted];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    battle.context.playerPokemon.hp = 0;
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;
    (battle as any)._forcedPartyMenuSelection = true;
    const battleUi = battle.battleUi as BattleUIState;
    battleUi.force_party_menu = true;
    battleUi.wram.current_menu = BattleMenu.POKEMON;
    const inputModule = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-input");
    inputModule.get_player_input.mockReturnValueOnce(2).mockReturnValueOnce(null);

    battle.update();
    battle.update();

    expect(battle.context.playerAction).toBeUndefined();
    expect(battleUi.force_party_menu).toBe(true);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
    expect((battle as any)._forcedPartyMenuSelection).toBe(true);
  });
});

describe("Battle start cries", () => {
  it("plays the wild battle cry once at the start of battle", () => {
    const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
    const { battle } = buildWildBattleStart(audioEngine);

    battle.update();

    expect(audioEngine.playSound).toHaveBeenCalledWith(
      "CRY_CHIKORITA",
      expect.objectContaining({ panning: "enemy" }),
    );
    battle.update();
    expect(audioEngine.playSound).toHaveBeenCalledTimes(1);
  });
});

describe("Battle turn execution timing", () => {
  it("executes one move per update tick", () => {
    const { battle } = buildBattle(MoveName.TACKLE);
    const moveModule = jest.requireMock("./move-execution");
    battle.context.playerAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.enemyAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.turnOrder = [BattleTurn.PLAYER, BattleTurn.ENEMY];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(moveModule.executeMove).toHaveBeenCalledTimes(1);
    expect(battle.context.currentState).toBe(BattleStateEnum.TURN_EXECUTION);

    battle.update();

    expect(moveModule.executeMove).toHaveBeenCalledTimes(2);
    expect(battle.context.currentState).toBe(BattleStateEnum.POST_TURN_EFFECTS);
  });

  it("handles an enemy faint immediately after the player's move", () => {
    const { battle } = buildBattle(MoveName.TACKLE);
    const moveModule = jest.requireMock("./move-execution");
    moveModule.executeMove.mockImplementationOnce(() => {
      battle.context.enemyPokemon.hp = 0;
    });
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");
    battle.context.playerAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.enemyAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.turnOrder = [BattleTurn.PLAYER, BattleTurn.ENEMY];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(moveModule.executeMove).toHaveBeenCalledTimes(1);
    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "show_text" && event.data.text === `Enemy ${battle.context.enemyPokemon.nickname} fainted!`)).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);
  });

  it("prompts for a replacement before residual effects after the enemy knocks out the player", () => {
    const { battle } = buildBattle(MoveName.TACKLE);
    const reserve = createPokemon(battle.gameState, buildSpecies(), 5);
    battle.context.playerParty = [battle.context.playerPokemon, reserve];
    battle.context.playerActiveIndex = 0;
    battle.context.playerPokemon = battle.context.playerParty[0];
    const moveModule = jest.requireMock("./move-execution");
    moveModule.executeMove.mockImplementationOnce(() => {
      battle.context.playerPokemon.hp = 0;
    });
    const dispatchSpy = jest.spyOn(battle.eventManager, "dispatch");
    battle.context.playerAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.enemyAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.turnOrder = [BattleTurn.ENEMY, BattleTurn.PLAYER];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;

    battle.update();

    expect(moveModule.executeMove).toHaveBeenCalledTimes(1);
    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "show_text" && event.data.text === `${battle.context.playerPokemon.nickname} fainted!`)).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);

    battle.update();

    expect(dispatchSpy.mock.calls.some(([event]) => event.name === "prompt_yes_no")).toBe(true);
    expect(battle._awaitingFaintPrompt).toBe(true);
  });

  it("halts turn execution while the UI blocks state advance", () => {
    const { battle } = buildBattle(MoveName.TACKLE);
    const moveModule = jest.requireMock("./move-execution");
    const uiCore = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-core");
    battle.context.playerAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.enemyAction = { actionType: BattleActionType.MOVE, moveName: MoveName.TACKLE };
    battle.context.turnOrder = [BattleTurn.PLAYER, BattleTurn.ENEMY];
    battle.context.currentState = BattleStateEnum.TURN_EXECUTION;
    uiCore.should_block_state_advance.mockReturnValueOnce(true).mockReturnValueOnce(false);

    battle.update();

    expect(moveModule.executeMove).not.toHaveBeenCalled();
    expect(battle.context.currentState).toBe(BattleStateEnum.TURN_EXECUTION);

    battle.update();

    expect(moveModule.executeMove).toHaveBeenCalledTimes(1);
    expect(battle.context.currentState).toBe(BattleStateEnum.TURN_EXECUTION);
  });
});

describe("Battle end finalization timing", () => {
  it("waits for deferred post-animation battle events before finalising a caught battle", () => {
    const { battle } = buildBattle();
    const uiCore = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-core");
    const actualUiCore = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-core");
    const battleUi = battle.battleUi as BattleUIState;
    battle.context.currentState = BattleStateEnum.BATTLE_END;
    battle._caughtPokemon = true;
    battleUi.pending_animation_events = [new Event("show_text", { text: "You caught PIDGEY!" })];
    battleUi.animation_player = { is_active: () => false } as BattleUIState["animation_player"];
    uiCore.should_block_state_advance.mockImplementation(actualUiCore.should_block_state_advance);

    battle.update();

    expect(battle._finalised).toBe(false);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);

    battleUi.pending_animation_events = [];
    uiCore.should_block_state_advance.mockReturnValue(false);
    battle.update();

    expect(battle._finalised).toBe(true);
  });

  it("waits for pending evolution to be started before finalising a battle", () => {
    const { battle } = buildBattle();
    const uiCore = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-core");
    const actualUiCore = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-core");
    const uiMoves = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-moves");
    const maybeStartSpy = jest.spyOn(uiMoves, "maybe_start_pending_evolution");
    battle.context.currentState = BattleStateEnum.BATTLE_END;
    uiCore.should_block_state_advance.mockImplementation(actualUiCore.should_block_state_advance);
    maybeStartSpy
      .mockImplementationOnce((state) => {
        state.block_on_pending_evolution = true;
      })
      .mockImplementation((state) => {
        state.block_on_pending_evolution = false;
      });

    battle.update();

    expect(maybeStartSpy).toHaveBeenCalledTimes(1);
    expect(battle._finalised).toBe(false);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);

    battle.update();

    expect(maybeStartSpy).toHaveBeenCalledTimes(2);
    expect(battle._finalised).toBe(true);
    maybeStartSpy.mockRestore();
  });
});

describe("Battle start sequence (wild)", () => {
  it("announces the encounter and queues the player send-out", () => {
    const { battle, eventManager, playerPokemon, enemyPokemon } = buildWildBattleStart();
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(1);
    expect(dispatchSpy.mock.calls[0][0].name).toBe("show_text");
    expect(dispatchSpy.mock.calls[0][0].data.text).toBe(`Wild ${enemyPokemon.nickname} appeared!`);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_START);
    expect(battle.gameState.wram.wBattleHasJustStarted).toBe(1);

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(4);
    expect(dispatchSpy.mock.calls[1][0].name).toBe("frontpic_animation");
    expect(dispatchSpy.mock.calls[1][0].data).toEqual({ side: "enemy", speed: 0 });
    expect(battle.battleUi.frontpic_animation).toEqual({ side: "enemy", speed: 0 });
    expect(dispatchSpy.mock.calls[2][0].name).toBe("show_trainer_sprites");
    expect(dispatchSpy.mock.calls[2][0].data).toEqual({ mode: "player" });
    expect(dispatchSpy.mock.calls[3][0].name).toBe("show_text");
    expect(dispatchSpy.mock.calls[3][0].data.text).toBe(`Go! ${playerPokemon.nickname}!`);

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(5);
    expect(dispatchSpy.mock.calls[4][0].name).toBe("trigger_trainer_exit");
    expect(dispatchSpy.mock.calls[4][0].data).toEqual({ side: "player" });

    battle.battleUi.pending_trainer_exit = false;
    battle.battleUi.trainer_exit = null;
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(6);
    expect(dispatchSpy.mock.calls[5][0].name).toBe("play_animation");
    expect(dispatchSpy.mock.calls[5][0].data).toMatchObject({
      move_name: "SEND_OUT_MON",
      is_player_move: true,
      param: 0,
    });
    expect(battle.battleUi.animation_player.play_animation).toHaveBeenCalledWith(
      "SEND_OUT_MON",
      true,
      0,
      expect.objectContaining({
        param_label: null,
        shake_count: null,
      })
    );

    battle.update();
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.gameState.wram.wBattleHasJustStarted).toBe(0);
  });
});

describe("Battle start sequence (tutorial)", () => {
  it("skips the player send-out step after the wild encounter text", () => {
    const { battle, eventManager, enemyPokemon } = buildTutorialBattleStart();
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(1);
    expect(dispatchSpy.mock.calls[0][0].name).toBe("show_text");
    expect(dispatchSpy.mock.calls[0][0].data.text).toBe(`Wild ${enemyPokemon.nickname} appeared!`);

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(1);
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.gameState.wram.wBattleHasJustStarted).toBe(0);
  });

  it("never starts trainer intro portraits for non-trainer tutorial battles", () => {
    const { battle } = buildTutorialBattleStart();
    const uiCore = jest.requireMock("@pokecrystal/core/ui/overlays/battle-ui-core");
    const startIntroSpy = jest
      .spyOn(uiCore, "start_trainer_intro")
      .mockImplementation(() => {});
    const trainerIntroActiveSpy = jest
      .spyOn(uiCore, "trainer_intro_active")
      .mockReturnValue(false);
    battle.gameState.wram.other_trainer_class = "RIVAL1";
    (battle.battleUi.ui as { get_sprite_surface?: () => unknown }).get_sprite_surface = () => ({});

    const started = (battle as any)._maybeStartTrainerIntro();

    expect(startIntroSpy).not.toHaveBeenCalled();
    expect(started).toBe(false);

    startIntroSpy.mockRestore();
    trainerIntroActiveSpy.mockRestore();
  });
});

describe("Battle transition handling", () => {
  it("advances the transition while it is incomplete", () => {
    const { battle } = buildBattle();
    const transition = {
      isComplete: jest.fn(() => false),
      advance: jest.fn(),
      draw: jest.fn(),
    };
    (battle as any)._transition = transition;
    battle.context.currentState = BattleStateEnum.BATTLE_TRANSITION;

    battle.update();

    expect(transition.advance).toHaveBeenCalled();
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_TRANSITION);
  });

  it("moves to battle start after the transition completes", () => {
    const { battle } = buildBattle();
    const transition = {
      isComplete: jest.fn(() => true),
      advance: jest.fn(),
      draw: jest.fn(),
    };
    (battle as any)._transition = transition;
    battle.context.currentState = BattleStateEnum.BATTLE_TRANSITION;

    battle.update();

    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_START);
  });

  it("finishes a real wild battle transition after enough updates", () => {
    const { battle } = buildWildBattleStart();

    expect([BattleStateEnum.BATTLE_TRANSITION, BattleStateEnum.BATTLE_START]).toContain(
      battle.context.currentState,
    );

    for (let i = 0; i < 160; i += 1) {
      battle.update();
      if (battle.context.currentState !== BattleStateEnum.BATTLE_TRANSITION) {
        break;
      }
    }

    expect(battle.context.currentState).not.toBe(BattleStateEnum.BATTLE_TRANSITION);
  });
});

describe("Amulet Coin activation", () => {
  it("activates when the lead Pokemon is sent out in headless battle start flow", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = buildSpecies();
    const playerPokemon = createPokemon(gameState, species, 5);
    playerPokemon.item = "AMULET_COIN";
    const enemyPokemon = createPokemon(gameState, species, 5);
    const battle = new Battle(
      playerPokemon,
      enemyPokemon,
      gameState,
      eventManager,
      null as any,
      new Map<MoveName, any>()
    );
    battle.context.currentState = BattleStateEnum.BATTLE_START;

    battle.update();

    expect(battle.context.amuletCoinActive).toBe(true);
  });

  it("activates when switching to a Pokemon holding AMULET_COIN", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const species = buildSpecies();
    const leadPokemon = createPokemon(gameState, species, 5);
    const amuletPokemon = createPokemon(gameState, species, 5);
    amuletPokemon.item = "AMULET_COIN";
    const enemyPokemon = createPokemon(gameState, species, 5);
    const battle = new Battle(
      leadPokemon,
      enemyPokemon,
      gameState,
      eventManager,
      null as any,
      new Map<MoveName, any>(),
      null,
      undefined,
      [leadPokemon, amuletPokemon],
      [enemyPokemon]
    );
    battle.context.currentState = BattleStateEnum.PLAYER_ACTION_SELECT;

    const switched = (battle as any)._performSwitch(BattleTurn.PLAYER, 1);

    expect(switched).toBe(true);
    expect(battle.context.amuletCoinActive).toBe(true);
  });
});
