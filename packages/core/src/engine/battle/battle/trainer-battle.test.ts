import { createInitialGameState } from "@pokecrystal/core/core/state";
import { loadAllMoves } from "@pokecrystal/core/core/data-loader";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { TrainerSchema } from "@pokecrystal/core/core/models/trainer";
import { MoveName } from "@pokecrystal/core/core/enums";
import type { TrainerExitAnimation } from "@pokecrystal/core/ui/overlays/battle-intro";
import { BattleMenu } from "@pokecrystal/core/ui/overlays/_battle-menu";
import { BattleUIPhase } from "@pokecrystal/core/ui/overlays/battle-ui-state";
import { apply_battle_inputs } from "@pokecrystal/core/ui/overlays/battle-input";
import { B_PAD_A, B_PAD_B, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP } from "@pokecrystal/core/input/controls";
import { TrainerBattle } from "./trainer-battle";
import { Battle } from "./battle-logic";
import { BattleStateEnum } from "./battle-context";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";

jest.mock("@pokecrystal/core/ui/overlays/battle-ui-core", () => {
  const actual = jest.requireActual("@pokecrystal/core/ui/overlays/battle-ui-core");
  return {
    ...actual,
    should_block_state_advance: jest.fn(() => false),
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

const buildTrainerBattle = (options?: { instantMode?: boolean }) => {
  const gameState = createInitialGameState();
  if (options?.instantMode) {
    gameState.wram.instant_mode = true;
  }
  const eventManager = new EventManager(gameState);
  const species = buildSpecies();
  const playerPokemon = createPokemon(gameState, species, 5);
  playerPokemon.moves = [{ name: MoveName.TACKLE, current_pp: 35 }];
  const enemyPokemon = createPokemon(gameState, species, 5);
  enemyPokemon.hp = 1;
  enemyPokemon.max_hp = 1;
  const trainer = TrainerSchema.parse({
    name: "FALKNER",
    trainer_class: "BIRD_KEEPER",
    party: [enemyPokemon],
    win_quote: "",
    lose_quote: "",
  });
  const battleUi = {
    active: true,
    ui: {
      screen: options?.instantMode ? {} : undefined,
      tileSize: 8,
      font: {
        renderText: jest.fn(),
        fontTiles: {},
        font_tiles: {},
      },
      drawWindow: jest.fn(),
      eventQueue: [],
      update: jest.fn(),
    },
    is_mock: false,
    game_state: gameState,
    data_loader: null,
    wram: {
      current_menu: BattleMenu.MAIN,
      menu_header: null,
      wBattleMenuCursorPosition: 0,
      wMoveMenuCursorPosition: 0,
      wPartyMenuCursorPosition: 0,
      wPackMenuCursorPosition: 0,
      confirm_pressed: false,
      cancel_pressed: false,
      select_pressed: false,
      swapping_move_index: null,
      last_num_moves: 0,
      last_party_size: 1,
      last_item_names: [],
    },
    input_state: {
      active_direction: null,
      repeat_timer: 0,
    },
    bag_repeat_state: {
      active_direction: null,
      repeat_timer: 0,
    },
    pokemon_repeat_state: {
      active_direction: null,
      repeat_timer: 0,
    },
    ui_phase: BattleUIPhase.MENU,
    yes_no_prompt: {
      active: false,
      result: null,
      pending_activation: false,
      prompt: null,
    },
    pending_pokemon_selection: null,
    pending_pack_action: null,
    force_party_menu: false,
    pending_evolutions: [],
    pending_move_learns: [],
    active_evolution: null,
    active_move_learn: null,
    evolution_animation: null,
    block_on_pending_evolution: false,
    block_on_move_learning: false,
    hardware: { scx: 0, scy: 0 },
    animation_player: {
      is_active: jest.fn(() => false),
      play_animation: jest.fn(),
      reset: jest.fn(),
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
        is_complete: jest.fn(() => true),
        has_more_pages: jest.fn(() => false),
      },
    },
    frontpic_animation: null,
    trainer_intro: null,
    trainer_hud_visible: false,
    trainer_exit: null,
    pending_trainer_exit: false,
    trainer_sprite_override_mode: null,
    trainer_sprites_visible: false,
    trainer_send_out_seen: false,
    sprites_enabled: true,
  } as unknown as import("@pokecrystal/core/ui/overlays/battle-ui-state").BattleUIState;
  const movesMap = new Map<MoveName, any>();
  const tackle = loadAllMoves().get(MoveName.TACKLE);
  if (tackle) {
    movesMap.set(MoveName.TACKLE, tackle);
  }
  const audioEngine = { playSound: jest.fn() } as unknown as AudioEngine;
  const battle = new TrainerBattle(
    playerPokemon,
    trainer,
    gameState,
    eventManager,
    battleUi,
    movesMap,
    audioEngine
  );
  battleUi.ui = {
    ...battleUi.ui,
    get_sprite_surface: jest.fn(() => null),
    _apply_colorkey_transparency: jest.fn((surface) => surface),
    _get_pokemon_frame_surface: jest.fn(() => null),
  } as typeof battleUi.ui;
  battle.context.currentState = BattleStateEnum.BATTLE_START;
  return { battle, battleUi, eventManager, playerPokemon, trainer, audioEngine, movesMap };
};

const buildWildBattleFromTrainerHarness = (options?: { instantMode?: boolean }) => {
  const { battleUi, eventManager, playerPokemon, trainer, audioEngine, movesMap } = buildTrainerBattle(options);
  const enemyPokemon = trainer.party[0];
  const battle = new Battle(
    playerPokemon,
    enemyPokemon,
    battleUi.game_state,
    eventManager,
    battleUi,
    movesMap,
    audioEngine
  );
  battle.context.currentState = BattleStateEnum.BATTLE_START;
  return { battle, battleUi };
};

const runMenuSwitchInputSequence = (
  battle: { update: () => void; context: { currentState: BattleStateEnum } },
  battleUi: import("@pokecrystal/core/ui/overlays/battle-ui-state").BattleUIState,
): { inputs: number[]; updates: number; elapsedMs: number } => {
  const joypad = {
    hJoyPressed: 0,
    hJoypadPressed: 0,
    hJoyDown: 0,
    hJoypadDown: 0,
  };
  battleUi.game_state.hram.joypad = joypad;
  let updates = 0;
  const inputs: number[] = [];
  const update = () => {
    battle.update();
    updates += 1;
  };
  const press = (mask: number) => {
    inputs.push(mask);
    joypad.hJoyPressed = mask;
    joypad.hJoypadPressed = mask;
    joypad.hJoyDown = mask;
    joypad.hJoypadDown = mask;
    apply_battle_inputs(battleUi.wram, joypad, battleUi.input_state, { menu_active: true });
    joypad.hJoyPressed = 0;
    joypad.hJoypadPressed = 0;
    joypad.hJoyDown = 0;
    joypad.hJoypadDown = 0;
    update();
  };

  update();
  expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

  const startedAt = performance.now();

  press(B_PAD_A);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.FIGHT);
  press(B_PAD_B);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

  press(B_PAD_DOWN);
  press(B_PAD_A);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.PACK);
  battleUi.pending_pack_action = ["cancel", ""];
  update();
  expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

  press(B_PAD_UP);
  press(B_PAD_RIGHT);
  press(B_PAD_A);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
  press(B_PAD_B);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

  press(B_PAD_LEFT);
  press(B_PAD_A);
  expect(battleUi.wram.current_menu).toBe(BattleMenu.FIGHT);

  return { inputs, updates, elapsedMs: performance.now() - startedAt };
};

describe("TrainerBattle battle start sequence", () => {
  it("does not enter the battle transition state when constructed in instant mode", () => {
    const { battle, battleUi } = buildTrainerBattle({ instantMode: true });

    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_START);
    expect((battle as unknown as { _transition: unknown })._transition).toBeNull();
    expect(battleUi.trainer_intro).toBeNull();
    expect(battleUi.trainer_hud_visible).toBe(false);
  });

  it("skips trainer intro and send-out animations in instant mode", () => {
    const { battle, battleUi, eventManager } = buildTrainerBattle();
    battle.gameState.wram.instant_mode = true;
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    battle.update();

    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "show_text" }));
    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "trigger_trainer_exit" }));
    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "play_animation" }));
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battle.gameState.wram.wBattleHasJustStarted).toBe(0);
    expect(battleUi.trainer_exit).toBeNull();
    expect(battleUi.pending_trainer_exit).toBe(false);
    expect(battleUi.frontpic_animation).toBeNull();
    expect(battleUi.animation_player.reset).toHaveBeenCalled();
  });

  it("does not accumulate trainer intro waits across repeated instant updates", () => {
    const { battle, battleUi, eventManager } = buildTrainerBattle();
    battle.gameState.wram.instant_mode = true;
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    const startedAt = performance.now();
    for (let i = 0; i < 500; i += 1) {
      battle.context.currentState = BattleStateEnum.BATTLE_START;
      battle.gameState.wram.wBattleHasJustStarted = 1;
      battle.update();
    }
    const elapsedMs = performance.now() - startedAt;

    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "show_text" }));
    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "wait_for_input" }));
    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "play_animation" }));
    expect(battleUi.dialogue.pending_waits).toBe(0);
    expect(battleUi.dialogue.queue).toEqual([]);
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(elapsedMs).toBeLessThan(25);
  });

  it("tours battle commands and finishes a trainer battle quickly in instant mode", () => {
    const { battle, battleUi } = buildTrainerBattle({ instantMode: true });
    const joypad = {
      hJoyPressed: 0,
      hJoypadPressed: 0,
      hJoyDown: 0,
      hJoypadDown: 0,
    };
    battle.gameState.hram.joypad = joypad;
    const press = (mask: number) => {
      joypad.hJoyPressed = mask;
      joypad.hJoypadPressed = mask;
      joypad.hJoyDown = mask;
      joypad.hJoypadDown = mask;
      apply_battle_inputs(battleUi.wram, joypad, battleUi.input_state, { menu_active: true });
      joypad.hJoyPressed = 0;
      joypad.hJoypadPressed = 0;
      joypad.hJoyDown = 0;
      joypad.hJoypadDown = 0;
      battle.update();
    };

    battle.update();
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);

    const startedAt = performance.now();

    press(B_PAD_A);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.FIGHT);
    press(B_PAD_B);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_RIGHT);
    press(B_PAD_DOWN);
    press(B_PAD_A);
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    battle.update();
    expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_DOWN);
    press(B_PAD_A);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.PACK);
    battleUi.pending_pack_action = ["cancel", ""];
    battle.update();
    expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_UP);
    press(B_PAD_RIGHT);
    press(B_PAD_A);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.POKEMON);
    press(B_PAD_B);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_LEFT);
    press(B_PAD_A);
    expect(battleUi.wram.current_menu).toBe(BattleMenu.FIGHT);
    press(B_PAD_A);

    for (let i = 0; i < 20 && !battle.isFinished(); i += 1) {
      battle.update();
    }
    const elapsedMs = performance.now() - startedAt;

    expect(battle.isFinished()).toBe(true);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_END);
    expect(battleUi.dialogue.pending_waits).toBe(0);
    expect(battleUi.dialogue.queue).toEqual([]);
    expect(elapsedMs).toBeLessThan(100);
  });

  it("switches instant trainer battle menus as fast as instant wild battle menus", () => {
    const wild = buildWildBattleFromTrainerHarness({ instantMode: true });
    const trainer = buildTrainerBattle({ instantMode: true });

    const wildTiming = runMenuSwitchInputSequence(wild.battle, wild.battleUi);
    const trainerTiming = runMenuSwitchInputSequence(trainer.battle, trainer.battleUi);

    expect(trainerTiming.inputs).toEqual(wildTiming.inputs);
    expect(trainerTiming.updates).toBe(wildTiming.updates);
    expect(trainerTiming.elapsedMs).toBeLessThan(10);
    expect(wildTiming.elapsedMs).toBeLessThan(10);
    expect(trainerTiming.elapsedMs).toBeLessThanOrEqual(wildTiming.elapsedMs + 5);
    expect(trainer.battleUi.dialogue.pending_waits).toBe(0);
    expect(trainer.battleUi.dialogue.queue).toEqual([]);
  });

  it("cancels an already-started trainer intro in instant mode", () => {
    const { battle, battleUi, eventManager } = buildTrainerBattle();
    battle.gameState.wram.instant_mode = true;
    battleUi.trainer_intro = {} as typeof battleUi.trainer_intro;
    battleUi.trainer_hud_visible = true;
    battleUi.sprites_enabled = false;
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    battle.update();

    expect(dispatchSpy).not.toHaveBeenCalledWith(expect.objectContaining({ name: "show_text" }));
    expect(battle.context.currentState).toBe(BattleStateEnum.PLAYER_ACTION_SELECT);
    expect(battleUi.trainer_intro).toBeNull();
    expect(battleUi.trainer_hud_visible).toBe(false);
    expect(battleUi.sprites_enabled).toBe(true);
    expect(battleUi.animation_player.reset).toHaveBeenCalled();
  });

  it("announces the trainer battle, slides trainers out, and queues send-out animations", () => {
    const { battle, battleUi, eventManager, playerPokemon, trainer, audioEngine } = buildTrainerBattle();
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const clearDialogue = () => {
      battleUi.dialogue.forced_visible = false;
      battleUi.dialogue.pending_waits = 0;
      battleUi.dialogue.queue = [];
    };

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(1);
    expect(dispatchSpy.mock.calls[0][0].name).toBe("show_text");
    expect(dispatchSpy.mock.calls[0][0].data.text).toBe(`${trainer.name} wants to battle!`);
    expect(battle.context.currentState).toBe(BattleStateEnum.BATTLE_START);
    expect(battle.gameState.wram.wBattleHasJustStarted).toBe(1);

    clearDialogue();
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(2);
    expect(dispatchSpy.mock.calls[1][0].name).toBe("trigger_trainer_exit");
    expect(dispatchSpy.mock.calls[1][0].data).toMatchObject({ side: "enemy" });

    battleUi.trainer_exit = {} as TrainerExitAnimation;
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(2);

    battleUi.trainer_exit = null;
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(3);
    expect(dispatchSpy.mock.calls[2][0].name).toBe("show_text");
    expect(dispatchSpy.mock.calls[2][0].data.text).toBe(
      `${trainer.name} sent out ${battle.context.enemyPokemon.nickname}!`
    );

    clearDialogue();
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(4);
    expect(dispatchSpy.mock.calls[3][0].name).toBe("play_animation");
    expect(dispatchSpy.mock.calls[3][0].data).toMatchObject({
      move_name: "SEND_OUT_MON",
      is_player_move: false,
      param: 0,
    });
    expect(battleUi.animation_player.play_animation).toHaveBeenLastCalledWith(
      "SEND_OUT_MON",
      false,
      0,
      expect.objectContaining({
        param_label: null,
        shake_count: null,
      })
    );

    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(7);
    expect(audioEngine.playSound).toHaveBeenCalledWith("CRY_CHIKORITA", {
      panning: "enemy",
    });
    expect(dispatchSpy.mock.calls[4][0].name).toBe("frontpic_animation");
    expect(dispatchSpy.mock.calls[4][0].data).toMatchObject({ side: "enemy", speed: 4 });
    expect(battleUi.frontpic_animation).toEqual({ side: "enemy", speed: 4 });
    expect(dispatchSpy.mock.calls[5][0].name).toBe("show_trainer_sprites");
    expect(dispatchSpy.mock.calls[5][0].data).toMatchObject({ mode: "player" });
    expect(battleUi.trainer_sprite_override_mode).toBe("player");
    expect(battleUi.trainer_sprites_visible).toBe(true);
    expect(battleUi.trainer_send_out_seen).toBe(false);
    expect(dispatchSpy.mock.calls[6][0].name).toBe("show_text");
    expect(dispatchSpy.mock.calls[6][0].data.text).toBe(`Go! ${playerPokemon.nickname}!`);

    clearDialogue();
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(8);
    expect(dispatchSpy.mock.calls[7][0].name).toBe("trigger_trainer_exit");
    expect(dispatchSpy.mock.calls[7][0].data).toMatchObject({ side: "player" });

    battleUi.trainer_exit = {} as TrainerExitAnimation;
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(8);

    battleUi.trainer_exit = null;
    battle.update();
    expect(dispatchSpy).toHaveBeenCalledTimes(9);
    expect(dispatchSpy.mock.calls[8][0].name).toBe("play_animation");
    expect(dispatchSpy.mock.calls[8][0].data).toMatchObject({
      move_name: "SEND_OUT_MON",
      is_player_move: true,
      param: 0,
    });
    expect(battleUi.animation_player.play_animation).toHaveBeenLastCalledWith(
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
