import { BattleMenu, MenuDirection } from "@/ui/overlays/_battle-menu";
import {
  BattleInputState,
  BattleUIRoot,
  BattleUIState,
  BattleWRAM,
  YesNoPromptState,
} from "@/ui/overlays/battle-ui-state";
import { get_player_input, handle_input } from "@/ui/overlays/battle-ui-input";
import { reset_menu_selection } from "@/ui/overlays/battle-ui-core";
import { GameButton, buttonKeys } from "@/input/buttons";
import type { PokemonMenu } from "@/ui/menus/pokemon-menu";
import { LearnedMove, Pokemon } from "@/core/models";
import { createInitialGameState, type GameState } from "@/core/state";

jest.mock("@/ui/menus/bag-menu", () => ({
  BagMenu: jest.fn().mockImplementation(() => ({})),
}));

const createBattleInputState = (): BattleInputState => ({
  active_direction: null,
  repeat_timer: 0,
});

const createYesNoPrompt = (): YesNoPromptState => ({
  active: false,
  selection: 0,
  result: null,
  pending_activation: false,
});

const createMockBattleUI = (): BattleUIRoot => {
  const font = {
    renderText: jest.fn(),
    render_text: jest.fn(),
  };
  const baseUI = {
    screen: null,
    tileSize: 8,
    tile_size: 8,
    font,
    drawWindow: jest.fn(),
    eventQueue: [],
    get_context_palette: jest.fn(() => [0, 0, 0]),
    getContextPalette: jest.fn(() => [0, 0, 0]),
    update: jest.fn(),
    playCry: jest.fn(),
  };
  return baseUI as unknown as BattleUIRoot;
};

const createBaseWRAM = (): BattleWRAM => ({
  current_menu: BattleMenu.MAIN,
  menu_header: null,
  wBattleMenuCursorPosition: 0,
  wMoveMenuCursorPosition: 0,
  wPartyMenuCursorPosition: 0,
  wPackMenuCursorPosition: 0,
  wBattleHasJustStarted: 0,
  wBattleTextDelay: 0,
  wTextDelayFlags: 0,
  wInputType: 0,
  confirm_pressed: false,
  cancel_pressed: false,
  select_pressed: false,
  last_num_moves: 0,
  last_party_size: 0,
});

const createMockBattleState = (
  gameState: GameState,
  wram: BattleWRAM
): Partial<BattleUIState> & {
  wram: BattleWRAM;
  ui: BattleUIRoot;
  game_state: GameState;
  bag_menu: null;
  pokemon_menu: null;
  pending_pack_action: null;
  pending_pokemon_selection: null;
  force_party_menu: boolean;
  active: boolean;
  bag_repeat_state: BattleInputState;
  pokemon_repeat_state: BattleInputState;
  input_state: BattleInputState;
  yes_no_prompt: YesNoPromptState;
} => ({
  wram,
  ui: createMockBattleUI(),
  game_state: gameState,
  bag_menu: null,
  pokemon_menu: null,
  pending_pack_action: null,
  pending_pokemon_selection: null,
  force_party_menu: false,
  active: true,
  bag_repeat_state: createBattleInputState(),
  pokemon_repeat_state: createBattleInputState(),
  input_state: createBattleInputState(),
  yes_no_prompt: createYesNoPrompt(),
});

describe("Battle UI", () => {
  let state: ReturnType<typeof createMockBattleState>;
  let wram: BattleWRAM;
  let moves: LearnedMove[];
  let party: Pokemon[];
  let gameState: GameState;

  beforeEach(() => {
    gameState = createInitialGameState();
    gameState.sram.items = {};
    gameState.wram.battle_type = "BATTLETYPE_NORMAL";
    wram = createBaseWRAM();
    state = createMockBattleState(gameState, wram);
    moves = [{ name: "TACKLE", pp: 35, max_pp: 35 }];
    party = [{ speciesId: "CYNDAQUIL", level: 5 }] as Pokemon[];
  });

  const resolveState = (): BattleUIState => state as BattleUIState;

  it("should transition from MAIN to FIGHT menu", () => {
    wram.wBattleMenuCursorPosition = 0;
    wram.confirm_pressed = true;

    get_player_input(resolveState(), moves, party);

    expect(wram.current_menu).toBe(BattleMenu.FIGHT);
  });

  it("should transition from FIGHT to MAIN menu on cancel", () => {
    wram.current_menu = BattleMenu.FIGHT;
    wram.cancel_pressed = true;

    get_player_input(resolveState(), moves, party);

    expect(wram.current_menu).toBe(BattleMenu.MAIN);
  });

  it("should transition from MAIN to POKEMON menu", () => {
    wram.wBattleMenuCursorPosition = 1;
    wram.confirm_pressed = true;

    get_player_input(resolveState(), moves, party);

    expect(wram.current_menu).toBe(BattleMenu.POKEMON);
  });

  it("should transition from POKEMON to MAIN menu on cancel", () => {
    wram.current_menu = BattleMenu.POKEMON;
    wram.cancel_pressed = true;

    get_player_input(resolveState(), moves, party);

    expect(wram.current_menu).toBe(BattleMenu.MAIN);
  });

  it("should transition from MAIN to PACK menu", () => {
    wram.wBattleMenuCursorPosition = 2;
    wram.confirm_pressed = true;

    get_player_input(resolveState(), moves, party, { POTION: 1 });

    expect(wram.current_menu).toBe(BattleMenu.PACK);
  });

  it("should close the POKEMON menu on cancel input", () => {
    wram.current_menu = BattleMenu.POKEMON;
    state.pokemon_menu = {} as PokemonMenu;
    const cancelKey = buttonKeys(GameButton.B)[0];

    handle_input(resolveState(), { type: "keydown", key: cancelKey });

    expect(wram.current_menu).toBe(BattleMenu.MAIN);
    expect(state.pokemon_menu).toBeNull();
  });

  it("resets battle menu state between encounters", () => {
    wram.current_menu = BattleMenu.MAIN;
    wram.wBattleMenuCursorPosition = 3;
    wram.wMoveMenuCursorPosition = 2;
    wram.wPartyMenuCursorPosition = 1;
    wram.wPackMenuCursorPosition = 4;
    wram.confirm_pressed = true;
    wram.cancel_pressed = true;
    wram.select_pressed = true;
    wram.last_num_moves = 4;
    wram.last_party_size = 2;
    wram.last_item_names = ["POTION"];
    state.input_state.active_direction = MenuDirection.UP;
    state.input_state.repeat_timer = 3;
    state.pending_pack_action = ["use", "POTION"];
    state.bag_menu = {} as any;
    state.bag_repeat_state.active_direction = MenuDirection.DOWN;
    state.bag_repeat_state.repeat_timer = 2;

    reset_menu_selection(resolveState());

    expect(wram.current_menu).toBe(BattleMenu.MAIN);
    expect(wram.wBattleMenuCursorPosition).toBe(0);
    expect(wram.confirm_pressed).toBe(false);
    expect(wram.cancel_pressed).toBe(false);
    expect(wram.select_pressed).toBe(false);
    expect(wram.last_num_moves).toBe(0);
    expect(wram.last_party_size).toBe(0);
    expect(wram.last_item_names).toEqual([]);
    expect(state.input_state.active_direction).toBeNull();
    expect(state.input_state.repeat_timer).toBe(0);
    expect(state.pending_pack_action).toBeNull();
    expect(state.bag_menu).toBeNull();
    expect(state.bag_repeat_state.active_direction).toBeNull();
    expect(state.bag_repeat_state.repeat_timer).toBe(0);
  });
});
