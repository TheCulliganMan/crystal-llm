import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PokemonSpeciesSchema, createPokemon } from "@pokecrystal/core/core/models/pokemon";
import { BattleMenu } from "./_battle-menu";
import { render_pokemon_menu } from "./battle-ui-render";
import { BattleUIPhase, type BattleUIState } from "./battle-ui-state";
import { B_PAD_A, B_PAD_DOWN } from "@pokecrystal/core/input/controls";
import { Surface } from "../surface";
import { BitmapFont } from "../text/bitmap-font";
import { gameEngine } from "../game-engine";
import { GameButton, buttonKeys } from "@pokecrystal/core/input/buttons";
import { MonMenuItem } from "@pokecrystal/core/core/enums/mon-menu";

jest.mock("../menus/party-menu-icons", () => ({
  PartyMenuIconRenderer: jest.fn().mockImplementation(() => ({
    draw: jest.fn(),
  })),
}));

let font: BitmapFont;
let fontProxy: { paletteVariants: BitmapFont["paletteVariants"]; fontTiles: BitmapFont["fontTiles"]; font_tiles: BitmapFont["fontTiles"] };

beforeAll(async () => {
  font = new BitmapFont();
  await font.load();
  fontProxy = {
    paletteVariants: font.paletteVariants.bind(font),
    fontTiles: font.fontTiles,
    font_tiles: font.fontTiles,
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

const buildState = (): BattleUIState => {
  const gameState = createInitialGameState();
  const species = buildSpecies();
  const first = createPokemon(gameState, species, 5);
  const second = createPokemon(gameState, species, 5);
  gameState.sram.party.pokemon = [first, second];
  gameState.hram.joypad.hJoyPressed = 0;
  gameState.hram.joypad.hJoyDown = 0;
  (gameState as unknown as { write_bg_map_with_wait?: () => void }).write_bg_map_with_wait = jest.fn();
  (gameState as unknown as { bg_map_sync?: { is_busy: boolean } }).bg_map_sync = { is_busy: false };
  const screen = new Surface(160, 144);
  return {
    ui: {
      screen,
      font: fontProxy,
      update: jest.fn(),
      renderSnapshot: jest.fn(),
    },
    game_state: gameState,
    wram: {
      current_menu: BattleMenu.POKEMON,
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
      last_item_names: [],
    },
    pokemon_menu: null,
    pokemon_stats: null,
    pending_pokemon_selection: null,
    pokemon_repeat_state: { active_direction: null, repeat_timer: 0 },
    bag_repeat_state: { active_direction: null, repeat_timer: 0 },
    input_state: { active_direction: null, repeat_timer: 0 },
    bag_menu: null,
    pending_pack_action: null,
    yes_no_prompt: { active: false, result: null, pending_activation: false, prompt: null },
    dialogue: {
      forced_visible: false,
      pending_waits: 0,
      queue: [],
      dialogue: { is_complete: () => true, has_more_pages: () => false, visible_text: "" },
    },
    dialogue_wait_gate_active: false,
    manual_wait_override: false,
    waiting_for_input: false,
    ui_phase: BattleUIPhase.MENU,
    force_party_menu: false,
    trainer_intro: null,
    trainer_exit: null,
    trainer_victory: null,
    evolution_animation: null,
    block_on_pending_evolution: false,
    block_on_move_learning: false,
    animation_player: { is_active: () => false },
    presented_this_frame: false,
    pending_animation_events: [],
    fast_animation_request: false,
    fast_text_request: false,
    trainer_sprites_visible: false,
    trainer_send_out_seen: false,
    trainer_hud_visible: false,
    pending_trainer_exit: false,
    pending_trainer_exit_side: null,
    trainer_sprite_override_mode: null,
    trainer_overlay_player_visible: null,
    trainer_overlay_enemy_visible: null,
    is_mock: false,
    context: null,
    active_move_learn: null,
    move_forget_menu: null,
    pending_move_learns: [],
    pending_nickname_request: null,
    sprites_enabled: true,
    exp_animation: null,
    active_evolution: null,
    pending_evolutions: [],
    audio_engine: null,
    data_loader: null,
    active: true,
    scx: 0,
    scy: 0,
    palette_registers: {},
    hp_palettes: {},
    hp_animation_states: {},
    oam_manager: {} as unknown,
    animation_clock: { frame: 0, tick: () => {} },
    _move_metadata: new Map(),
    _type_display_name: () => "",
  } as unknown as BattleUIState;
};

const pressJoypad = (state: BattleUIState, mask: number): void => {
  state.game_state.hram.joypad.hJoyPressed = mask;
  state.game_state.hram.joypad.hJoyDown = 0;
};

describe("battle pokemon menu", () => {
  it("directly selects a replacement when the battle forces party selection", () => {
    const state = buildState();
    state.force_party_menu = true;
    render_pokemon_menu(state);
    const downKey = gameEngine.K_DOWN;
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYDOWN, { key: downKey, code: downKey }));
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYUP, { key: downKey, code: downKey }));
    const key = buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN;
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key }));

    expect(state.pokemon_menu?.getMode()).toBe("list");
    expect(state.pending_pokemon_selection).toBe(1);
  });

  it("opens the battle submenu instead of immediately selecting", () => {
    const state = buildState();
    pressJoypad(state, B_PAD_A);
    render_pokemon_menu(state);
    expect(state.pokemon_menu?.getMode()).toBe("submenu");
    expect(state.pending_pokemon_selection).toBeNull();
    expect(state.game_state.hram.joypad.hJoyPressed).toBe(0);
    expect(state.pokemon_menu?.getSubmenuChoices().map((choice) => choice.label)).toEqual([
      "STATS",
      "SWITCH",
      "CANCEL",
    ]);
  });

  it("directly selects a target when a battle item is waiting for a party target", () => {
    const state = buildState();
    state.battle_item_target_selection = true;
    pressJoypad(state, B_PAD_A);

    render_pokemon_menu(state);

    expect(state.pokemon_menu).toBeNull();
    expect(state.pending_pokemon_selection).toBe(0);
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);
    expect(state.game_state.hram.joypad.hJoyPressed).toBe(0);
  });

  it("opens the stats screen from the submenu", () => {
    const state = buildState();
    pressJoypad(state, B_PAD_A);
    render_pokemon_menu(state);
    pressJoypad(state, B_PAD_A);
    render_pokemon_menu(state);
    expect(state.pokemon_stats).not.toBeNull();
    expect(state.game_state.hram.joypad.hJoyPressed).toBe(0);
  });

  it("opens stats for the selected party slot instead of falling back to wCurPartyMon", () => {
    const state = buildState();
    state.game_state.wram.wCurPartyMon = 0;

    render_pokemon_menu(state);
    const downKey = gameEngine.K_DOWN;
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYDOWN, { key: downKey, code: downKey }));
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYUP, { key: downKey, code: downKey }));
    const key = buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN;
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key }));
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key }));

    expect(state.pokemon_stats).not.toBeNull();
    expect(state.pokemon_stats?.getActivePokemon()).toBe(state.game_state.sram.party.pokemon[1]);
    expect(state.game_state.wram.wCurPartyMon).toBe(1);
    expect(state.game_state.wram.wPartyMenuCursor).toBe(2);
  });

  it("records the switch selection when SWITCH is chosen", () => {
    const state = buildState();
    pressJoypad(state, B_PAD_A);
    render_pokemon_menu(state);
    pressJoypad(state, B_PAD_DOWN);
    render_pokemon_menu(state);
    expect(state.pokemon_menu?.getSubmenuIndex()).toBe(1);
    expect(state.pokemon_menu?.getSubmenuChoices()[1]?.item).toBe(MonMenuItem.SWITCH);
    const downKey = gameEngine.K_DOWN;
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYUP, { key: downKey, code: downKey }));
    const key = buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN;
    state.game_state.hram.joypad.hJoyDown = 0;
    state.game_state.hram.joypad.hJoyPressed = 0;
    state.pokemon_menu?.handleInput?.(new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key }));
    expect(state.pending_pokemon_selection).toBe(0);
  });
});
