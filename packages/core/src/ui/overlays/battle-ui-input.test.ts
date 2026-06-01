import { Event } from "@pokecrystal/core/engine/world/events";
import { GameButton, buttonKeys } from "@pokecrystal/core/input/buttons";
import { B_PAD_A, B_PAD_B, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP } from "@pokecrystal/core/input/controls";
import { gameEngine } from "../game-engine";
import { BagMenu } from "../menus/bag-menu";
import { BATTLE_TEXT_ADVANCE_DELAY_FRAMES, BattleUIPhase } from "./battle-ui-state";
import { BattleMenu } from "./_battle-menu";
import { apply_battle_inputs } from "./battle-input";
import { createBagMenuUI } from "./battle-ui-menu-utils";
import {
  _dispatch_bag_event,
  flush_deferred_animation_events,
  forward_pack_menu_inputs,
  get_player_input,
  handle_event,
  handle_input,
} from "./battle-ui-input";

describe("battle-ui-input trigger_trainer_exit", () => {
  it("does not queue battle dialogue waits in instant mode", () => {
    const dialogue = {
      window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
      dialogue: {
        is_complete: jest.fn(() => true),
        has_more_pages: jest.fn(() => false),
        clear: jest.fn(),
        open: jest.fn(),
      },
      queue: [],
      pending_waits: 0,
      forced_visible: false,
      auto_close_after_display: false,
    };
    const state = {
      active: true,
      game_state: { wram: { instant_mode: true } },
      dialogue,
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("show_text", { text: "Enemy used TACKLE!" }));
    handle_event(state, new Event("wait_for_input", {}));

    expect(state.dialogue.queue).toEqual([]);
    expect(state.dialogue.pending_waits).toBe(0);
    expect(state.dialogue.forced_visible).toBe(false);
    expect(dialogue.dialogue.open).not.toHaveBeenCalled();
  });

  it("does not build an input backlog from hundreds of instant battle text events", () => {
    const dialogue = {
      window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
      dialogue: {
        is_complete: jest.fn(() => true),
        has_more_pages: jest.fn(() => false),
        clear: jest.fn(),
        open: jest.fn(),
      },
      queue: [],
      pending_waits: 0,
      forced_visible: false,
      auto_close_after_display: false,
    };
    const state = {
      active: true,
      game_state: { wram: { instant_mode: true } },
      animation_player: { is_active: jest.fn(() => true) },
      pending_animation_events: [],
      dialogue,
    } as unknown as import("./battle-ui-state").BattleUIState;

    const startedAt = performance.now();
    for (let i = 0; i < 500; i += 1) {
      handle_event(state, new Event("show_text", { text: `Battle text ${i}`, wait_for_animation: true }));
      handle_event(state, new Event("wait_for_input", {}));
      handle_event(state, new Event("open_text", {}));
    }
    const elapsedMs = performance.now() - startedAt;

    expect(state.dialogue.queue).toEqual([]);
    expect(state.dialogue.pending_waits).toBe(0);
    expect(state.dialogue.forced_visible).toBe(false);
    expect(state.pending_animation_events).toEqual([]);
    expect(dialogue.dialogue.open).not.toHaveBeenCalled();
    expect(elapsedMs).toBeLessThan(25);
  });

  it("defers trainer exit while dialogue is still visible", () => {
    const state = {
      active: true,
      ui_phase: BattleUIPhase.MENU,
      trainer_intro: null,
      trainer_exit: null,
      pending_trainer_exit: false,
      pending_trainer_exit_side: null,
      trainer_sprite_override_mode: null,
      trainer_sprites_visible: false,
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
        },
        queue: [],
        pending_waits: 0,
        forced_visible: true,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("trigger_trainer_exit", { side: "player" }));

    expect(state.pending_trainer_exit).toBe(true);
    expect(state.trainer_sprite_override_mode).toBeNull();
  });

  it("starts player-side trainer slide-out when no dialogue is blocking", () => {
    const ui = {
      get_sprite_surface: jest.fn(() => null),
      _apply_colorkey_transparency: jest.fn((surface) => surface),
      _get_pokemon_frame_surface: jest.fn(() => null),
      tile_size: 8,
      font: {
        font_tiles: {},
      },
    };
    const state = {
      active: true,
      ui,
      ui_phase: BattleUIPhase.MENU,
      trainer_intro: null,
      trainer_exit: null,
      pending_trainer_exit: false,
      pending_trainer_exit_side: null,
      trainer_sprite_override_mode: null,
      trainer_sprites_visible: false,
      hardware: {
        scx: 0,
        scy: 0,
      },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
        },
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("trigger_trainer_exit", { side: "player" }));

    expect(state.pending_trainer_exit).toBe(false);
    expect(state.trainer_sprite_override_mode).toBe("player");
    expect(state.trainer_sprites_visible).toBe(true);
    expect(state.trainer_exit).not.toBeNull();
    expect((state.trainer_exit as { target_side?: string } | null)?.target_side).toBe("player");
  });
});

describe("battle-ui-input show_trainer_sprites", () => {
  it("activates the requested trainer sprite mode", () => {
    const state = {
      active: true,
      ui_phase: BattleUIPhase.MENU,
      trainer_sprite_override_mode: null,
      trainer_sprites_visible: false,
      trainer_send_out_seen: true,
      sprites_enabled: false,
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("show_trainer_sprites", { mode: "player" }));

    expect(state.trainer_sprite_override_mode).toBe("player");
    expect(state.trainer_sprites_visible).toBe(true);
    expect(state.trainer_send_out_seen).toBe(false);
    expect(state.sprites_enabled).toBe(true);
  });
});

describe("battle-ui-input frontpic animation requests", () => {
  it("stores the frontpic animation payload", () => {
    const state = {
      active: true,
      frontpic_animation: null,
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("frontpic_animation", { side: "enemy", speed: 4 }));

    expect(state.frontpic_animation).toEqual({ side: "enemy", speed: 4 });
  });
});

describe("battle-ui-input show_text deferral", () => {
  it("queues text until animations finish when wait_for_animation is set", () => {
    const dialogueWindow = {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      open: jest.fn(),
      clear: jest.fn(),
      complete: jest.fn(),
      advance_page: jest.fn(),
    };
    const state = {
      active: true,
      ui_phase: BattleUIPhase.MENU,
      game_state: { wram: { instant_mode: false } },
      pending_animation_events: [],
      animation_player: { is_active: jest.fn(() => true) },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("show_text", { text: "Caught!", wait_for_animation: true }));

    expect(state.dialogue.queue.length).toBe(0);
    expect(dialogueWindow.open).not.toHaveBeenCalled();
    expect(state.pending_animation_events.length).toBe(1);
  });

  it("flushes deferred text once animations stop", () => {
    const dialogueWindow = {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      open: jest.fn(),
      clear: jest.fn(),
      complete: jest.fn(),
      advance_page: jest.fn(),
    };
    const state = {
      active: true,
      ui_phase: BattleUIPhase.MENU,
      game_state: { wram: { instant_mode: false } },
      pending_animation_events: [
        new Event("show_text", { text: "Caught!", wait_for_animation: true }),
      ],
      animation_player: { is_active: jest.fn(() => false) },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    flush_deferred_animation_events(state);

    expect(dialogueWindow.open).toHaveBeenCalledWith("Caught!");
    expect(state.pending_animation_events.length).toBe(0);
  });

  it("does not queue text during instant mode", () => {
    const dialogueWindow = {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      open: jest.fn(),
      clear: jest.fn(),
      complete: jest.fn(),
      advance_page: jest.fn(),
    };
    const state = {
      active: true,
      ui_phase: BattleUIPhase.MENU,
      game_state: { wram: { instant_mode: true } },
      pending_animation_events: [],
      animation_player: { is_active: jest.fn(() => true) },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_event(state, new Event("show_text", { text: "Caught!", wait_for_animation: true }));

    expect(dialogueWindow.open).not.toHaveBeenCalled();
    expect(state.dialogue.queue).toEqual([]);
    expect(state.dialogue.pending_waits).toBe(0);
    expect(state.pending_animation_events.length).toBe(0);
  });
});

describe("battle-ui-input text advance timing", () => {
  it("does not add extra frame gating after dialogue is already complete", () => {
    const dialogueWindow = {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      complete: jest.fn(),
      advance_page: jest.fn(),
      clear: jest.fn(),
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
      },
      force_party_menu: false,
      pending_pokemon_selection: null,
      pokemon_menu: null,
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      active_move_learn: null,
      move_forget_menu: null,
      manual_wait_override: false,
      fast_text_request: false,
      game_state: {
        wram: {
          wBattleTextDelay: BATTLE_TEXT_ADVANCE_DELAY_FRAMES,
        },
      },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 1,
        forced_visible: true,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    const aKey = buttonKeys(GameButton.A)[0];
    handle_input(state, { type: "keydown", key: aKey, code: aKey } as any);

    expect(state.dialogue.pending_waits).toBe(0);
  });

  it("consumes dialogue advance joypad A so it does not also confirm the battle menu on the same frame", () => {
    const dialogueWindow = {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      complete: jest.fn(),
      advance_page: jest.fn(),
      clear: jest.fn(),
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
        confirm_pressed: false,
        cancel_pressed: false,
        select_pressed: false,
      },
      input_state: {
        active_direction: null,
        repeat_timer: 0,
      },
      force_party_menu: false,
      pending_pokemon_selection: null,
      pokemon_menu: null,
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      active_move_learn: null,
      move_forget_menu: null,
      manual_wait_override: false,
      fast_text_request: false,
      game_state: {
        wram: {
          wBattleTextDelay: 0,
        },
        hram: {
          joypad: {
            hJoyPressed: B_PAD_A,
            hJoypadPressed: B_PAD_A,
            hJoyDown: B_PAD_A,
            hJoypadDown: B_PAD_A,
          },
        },
      },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 1,
        forced_visible: true,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    const aKey = buttonKeys(GameButton.A)[0];
    handle_input(state, { type: "keydown", key: aKey, code: aKey } as any);

    expect(state.dialogue.pending_waits).toBe(0);
    expect(state.wram.confirm_pressed).toBe(false);
    expect(state.manual_wait_override).toBe(false);
  });

  it("clears a pre-latched confirm when dialogue advance consumes the same A press", () => {
    const dialogueWindow = {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
      complete: jest.fn(),
      advance_page: jest.fn(),
      clear: jest.fn(),
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
        confirm_pressed: true,
        cancel_pressed: true,
        select_pressed: true,
      },
      input_state: {
        active_direction: null,
        repeat_timer: 0,
      },
      force_party_menu: false,
      pending_pokemon_selection: null,
      pokemon_menu: null,
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      active_move_learn: null,
      move_forget_menu: null,
      manual_wait_override: false,
      fast_text_request: false,
      game_state: {
        wram: {
          wBattleTextDelay: 0,
        },
        hram: {
          joypad: {
            hJoyPressed: B_PAD_A,
            hJoypadPressed: B_PAD_A,
            hJoyDown: B_PAD_A,
            hJoypadDown: B_PAD_A,
          },
        },
      },
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 1,
        forced_visible: true,
        auto_close_after_display: false,
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    const aKey = buttonKeys(GameButton.A)[0];
    handle_input(state, { type: "keydown", key: aKey, code: aKey } as any);

    expect(state.wram.confirm_pressed).toBe(false);
    expect(state.wram.cancel_pressed).toBe(false);
    expect(state.wram.select_pressed).toBe(false);
  });
});

describe("battle-ui-input manual battle menu directions", () => {
  const createDialogueState = () => ({
    window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
    dialogue: {
      is_complete: jest.fn(() => true),
      has_more_pages: jest.fn(() => false),
    },
    queue: [],
    pending_waits: 0,
    forced_visible: false,
    auto_close_after_display: false,
  });

  it("moves the main battle cursor right on direct keydown input", () => {
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: false,
        cancel_pressed: false,
        select_pressed: false,
        swapping_move_index: null,
        last_num_moves: 4,
        last_party_size: 1,
        last_item_names: ["POKE_BALL"],
      },
      input_state: {
        active_direction: null,
        repeat_timer: 0,
      },
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      active_move_learn: null,
      move_forget_menu: null,
      force_party_menu: false,
      dialogue: createDialogueState(),
      game_state: {
        wram: { wBattleTextDelay: 0 },
        hram: { joypad: { hJoyPressed: B_PAD_RIGHT, hJoypadPressed: B_PAD_RIGHT } },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_input(state, { type: "keydown", key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT } as any);

    expect(state.wram.wBattleMenuCursorPosition).toBe(1);
    expect(state.game_state.hram.joypad.hJoyPressed).toBe(0);
    expect(state.game_state.hram.joypad.hJoypadPressed).toBe(0);
  });

  it("moves the main battle cursor down on direct keydown input", () => {
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: false,
        cancel_pressed: false,
        select_pressed: false,
        swapping_move_index: null,
        last_num_moves: 4,
        last_party_size: 1,
        last_item_names: ["POKE_BALL"],
      },
      input_state: {
        active_direction: null,
        repeat_timer: 0,
      },
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      active_move_learn: null,
      move_forget_menu: null,
      force_party_menu: false,
      dialogue: createDialogueState(),
      game_state: {
        wram: { wBattleTextDelay: 0 },
        hram: { joypad: { hJoyPressed: B_PAD_DOWN, hJoypadPressed: B_PAD_DOWN } },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_input(state, { type: "keydown", key: gameEngine.K_DOWN, code: gameEngine.K_DOWN } as any);

    expect(state.wram.wBattleMenuCursorPosition).toBe(2);
    expect(state.game_state.hram.joypad.hJoyPressed).toBe(0);
    expect(state.game_state.hram.joypad.hJoypadPressed).toBe(0);
  });

  it("switches battle pack pockets on direct keydown input", () => {
    const joypad = {
      hJoyPressed: 0,
      hJoypadPressed: 0,
      hJoyDown: 0,
      hJoypadDown: 0,
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.PACK,
        wBattleMenuCursorPosition: 2,
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
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
          clear: jest.fn(),
        },
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      pending_pack_action: null,
      pending_pokemon_selection: null,
      force_party_menu: false,
      game_state: {
        sram: {
          items: {},
          balls: { POKE_BALL: 5 },
          key_items: {},
          tm_hm: [],
          money: 0,
        },
        wram: {
          battle_type: "",
          wBattleTextDelay: 0,
        },
        hram: { joypad },
      },
      audio_engine: null,
      data_loader: null,
      bag_menu: new BagMenu(
        createBagMenuUI({
          screen: {} as any,
          tileSize: 8,
          font: { renderText: jest.fn(), fontTiles: {} } as any,
          drawWindow: jest.fn(),
          eventQueue: [] as any,
          update: jest.fn(),
        } as any),
        {
          sram: {
            items: {},
            balls: { POKE_BALL: 5 },
            key_items: {},
            tm_hm: [],
            money: 0,
          },
          wram: {
            battle_type: "",
            wBattleTextDelay: 0,
          },
        } as any,
        null,
        undefined
      ),
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_input(state, { type: "keydown", key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT } as any);

    expect(state.bag_menu?.getCurrentPocketLabel()).toBe("BALL");
  });

  it("keeps the ball pocket open after a direct right press even if the previous A edge is still latched", () => {
    const joypad = {
      hJoyPressed: B_PAD_A | B_PAD_RIGHT,
      hJoypadPressed: B_PAD_A | B_PAD_RIGHT,
      hJoyDown: B_PAD_RIGHT,
      hJoypadDown: B_PAD_RIGHT,
    };
    const state = {
      active: true,
      ui: {
        screen: {},
        tileSize: 8,
        font: {
          renderText: jest.fn(),
          fontTiles: {},
        },
        drawWindow: jest.fn(),
        eventQueue: [],
        update: jest.fn(),
      },
      wram: {
        current_menu: BattleMenu.PACK,
        wBattleMenuCursorPosition: 2,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 1,
        confirm_pressed: true,
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
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
          clear: jest.fn(),
        },
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      pending_pack_action: null,
      pending_pokemon_selection: null,
      force_party_menu: false,
      game_state: {
        sram: {
          items: {},
          balls: { POKE_BALL: 5 },
          key_items: {},
          tm_hm: [],
          money: 0,
        },
        wram: {
          battle_type: "",
          wBattleTextDelay: 0,
        },
        hram: { joypad },
      },
      audio_engine: null,
      data_loader: null,
      bag_menu: new BagMenu(
        createBagMenuUI({
          screen: {} as any,
          tileSize: 8,
          font: { renderText: jest.fn(), fontTiles: {} } as any,
          drawWindow: jest.fn(),
          eventQueue: [] as any,
          update: jest.fn(),
        } as any),
        {
          sram: {
            items: {},
            balls: { POKE_BALL: 5 },
            key_items: {},
            tm_hm: [],
            money: 0,
          },
          wram: {
            battle_type: "",
            wBattleTextDelay: 0,
          },
          hram: { joypad },
        } as any,
        null,
        undefined
      ),
    } as unknown as import("./battle-ui-state").BattleUIState;

    handle_input(state, { type: "keydown", key: gameEngine.K_RIGHT, code: gameEngine.K_RIGHT } as any);
    forward_pack_menu_inputs(state);

    expect(state.bag_menu?.getCurrentPocketLabel()).toBe("BALL");
    expect(state.wram.current_menu).toBe(BattleMenu.PACK);
    expect(state.pending_pack_action).toBeNull();
    expect(state.wram.last_item_names).toEqual(["POKE_BALL", "CANCEL"]);
    expect(joypad.hJoyPressed).toBe(0);
    expect(joypad.hJoypadPressed).toBe(0);
  });

  it("does not let the A press that opens PACK immediately re-confirm CANCEL on the next pack frame", () => {
    const joypad = {
      hJoyPressed: B_PAD_A,
      hJoypadPressed: B_PAD_A,
      hJoyDown: 0,
      hJoypadDown: 0,
    };
    const state = {
      active: true,
      ui: {
        screen: {},
        tileSize: 8,
        font: {
          renderText: jest.fn(),
          fontTiles: {},
        },
        drawWindow: jest.fn(),
        eventQueue: [],
        update: jest.fn(),
      },
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 2,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
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
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
          clear: jest.fn(),
        },
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      pending_pack_action: null,
      pending_pokemon_selection: null,
      force_party_menu: false,
      game_state: {
        sram: {
          items: {},
          balls: { POKE_BALL: 5 },
          key_items: {},
          tm_hm: [],
          money: 0,
        },
        wram: {
          battle_type: "",
          wBattleTextDelay: 0,
        },
        hram: { joypad },
      },
      audio_engine: null,
      data_loader: null,
    } as unknown as import("./battle-ui-state").BattleUIState;

    get_player_input(state, [], [{}] as any, {});
    forward_pack_menu_inputs(state);

    expect(state.wram.current_menu).toBe(BattleMenu.PACK);
    expect(state.pending_pack_action).toBeNull();
    expect(state.wram.last_item_names).toEqual(["CANCEL"]);
    expect(joypad.hJoyPressed).toBe(0);
    expect(joypad.hJoypadPressed).toBe(0);
  });
});

describe("battle-ui-input fight menu confirm latch", () => {
  it("runs an instant-mode battle command tour and resolves a move without frame waits", () => {
    const joypad = {
      hJoyPressed: 0,
      hJoypadPressed: 0,
      hJoyDown: 0,
      hJoypadDown: 0,
    };
    const state = {
      active: true,
      ui: {
        screen: {},
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
      wram: {
        current_menu: BattleMenu.MAIN,
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
      pending_pokemon_selection: null,
      pending_pack_action: null,
      force_party_menu: false,
      game_state: {
        sram: {
          items: {},
          balls: {},
          key_items: {},
          tm_hm: [],
          money: 0,
        },
        wram: { instant_mode: true, wBattleTextDelay: 30, battle_type: "BATTLETYPE_NORMAL" },
        hram: { joypad },
      },
      audio_engine: null,
      data_loader: null,
    } as unknown as import("./battle-ui-state").BattleUIState;
    const moves = [{ name: "TACKLE", current_pp: 35 }] as never;
    const party = [{}] as never;
    const press = (mask: number) => {
      joypad.hJoyPressed = mask;
      joypad.hJoypadPressed = mask;
      joypad.hJoyDown = mask;
      joypad.hJoypadDown = mask;
      apply_battle_inputs(state.wram, joypad, state.input_state, { menu_active: true });
      joypad.hJoyPressed = 0;
      joypad.hJoypadPressed = 0;
      joypad.hJoyDown = 0;
      joypad.hJoypadDown = 0;
    };

    const startedAt = performance.now();

    press(B_PAD_A);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.FIGHT);

    press(B_PAD_B);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_RIGHT);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    press(B_PAD_DOWN);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    press(B_PAD_A);
    expect(get_player_input(state, moves, party, {})).toBe("RUN");
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_LEFT);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    press(B_PAD_A);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.PACK);
    state.pending_pack_action = ["cancel", ""];
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_UP);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    press(B_PAD_RIGHT);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    press(B_PAD_A);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.POKEMON);

    press(B_PAD_B);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);

    press(B_PAD_LEFT);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    press(B_PAD_A);
    expect(get_player_input(state, moves, party, {})).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.FIGHT);
    press(B_PAD_A);
    expect(get_player_input(state, moves, party, {})).toBe("TACKLE");

    expect(performance.now() - startedAt).toBeLessThan(25);
  });

  it("opens Fight from the main menu and selects a move on a fresh confirm press", () => {
    const joypad = {
      hJoyPressed: B_PAD_A,
      hJoypadPressed: B_PAD_A,
      hJoyDown: B_PAD_A,
      hJoypadDown: B_PAD_A,
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
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
      pending_pokemon_selection: null,
      pending_pack_action: null,
      force_party_menu: false,
      game_state: {
        wram: { wBattleTextDelay: 0, battle_type: "BATTLETYPE_NORMAL" },
        hram: { joypad },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;
    const moves = [{ name: "TACKLE", current_pp: 35 }] as never;

    expect(get_player_input(state, moves, [] as never, null)).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.FIGHT);
    expect(joypad.hJoyPressed).toBe(0);

    joypad.hJoyPressed = B_PAD_A;
    joypad.hJoypadPressed = B_PAD_A;
    apply_battle_inputs(state.wram, joypad, state.input_state, { menu_active: true });

    expect(get_player_input(state, moves, [] as never, null)).toBe("TACKLE");
  });

  it("does not carry the Fight confirm press into the move menu on the next frame", () => {
    const joypad = {
      hJoyPressed: B_PAD_A,
      hJoypadPressed: B_PAD_A,
      hJoyDown: B_PAD_A,
      hJoypadDown: B_PAD_A,
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 0,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
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
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      active_move_learn: null,
      move_forget_menu: null,
      pending_pokemon_selection: null,
      force_party_menu: false,
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
        },
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
      game_state: {
        wram: { wBattleTextDelay: 0, battle_type: "BATTLETYPE_NORMAL" },
        hram: { joypad },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;
    const moves = [{ name: "TACKLE", current_pp: 35 }] as never;

    expect(get_player_input(state, moves, [] as never, null)).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.FIGHT);
    expect(joypad.hJoyPressed).toBe(0);
    expect(joypad.hJoypadPressed).toBe(0);

    apply_battle_inputs(state.wram, joypad, state.input_state, { menu_active: true });

    expect(get_player_input(state, moves, [] as never, null)).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.FIGHT);
  });
});

describe("battle-ui-input Pokemon and run confirm latches", () => {
  it("clears the battle Pokemon stats overlay when backing out to the main menu", () => {
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.POKEMON,
        wBattleMenuCursorPosition: 1,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 1,
        wPackMenuCursorPosition: 0,
        confirm_pressed: false,
        cancel_pressed: false,
        select_pressed: false,
      },
      force_party_menu: false,
      pending_pokemon_selection: 1,
      pokemon_menu: {},
      pokemon_stats: {},
      game_state: {
        wram: { wBattleTextDelay: 0, battle_type: "BATTLETYPE_NORMAL" },
        hram: { joypad: { hJoyPressed: B_PAD_B, hJoypadPressed: B_PAD_B } },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    const bKey = buttonKeys(GameButton.B)[0];
    handle_input(state, { type: "keydown", key: bKey, code: bKey } as any);

    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);
    expect(state.pending_pokemon_selection).toBeNull();
    expect(state.pokemon_menu).toBeNull();
    expect(state.pokemon_stats).toBeNull();
  });

  it("does not carry the main-menu Pokemon confirm press into the party menu", () => {
    const joypad = {
      hJoyPressed: B_PAD_A,
      hJoypadPressed: B_PAD_A,
      hJoyDown: B_PAD_A,
      hJoypadDown: B_PAD_A,
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 1,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
        cancel_pressed: false,
        select_pressed: false,
        swapping_move_index: null,
        last_num_moves: 0,
        last_party_size: 2,
        last_item_names: [],
      },
      input_state: {
        active_direction: null,
        repeat_timer: 0,
      },
      pending_pokemon_selection: null,
      pending_pack_action: null,
      force_party_menu: false,
      game_state: {
        wram: { wBattleTextDelay: 0, battle_type: "BATTLETYPE_NORMAL" },
        hram: { joypad },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    expect(get_player_input(state, [], [{}, {}] as never, null)).toBeNull();

    expect(state.wram.current_menu).toBe(BattleMenu.POKEMON);
    expect(joypad.hJoyPressed).toBe(0);
    expect(joypad.hJoypadPressed).toBe(0);
  });

  it("does not leave A latched after a failed run returns to the main battle menu", () => {
    const joypad = {
      hJoyPressed: B_PAD_A,
      hJoypadPressed: B_PAD_A,
      hJoyDown: B_PAD_A,
      hJoypadDown: B_PAD_A,
    };
    const state = {
      active: true,
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 3,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
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
      pending_pokemon_selection: null,
      pending_pack_action: null,
      force_party_menu: false,
      game_state: {
        wram: { wBattleTextDelay: 0, battle_type: "BATTLETYPE_NORMAL" },
        hram: { joypad },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    expect(get_player_input(state, [], [{}] as never, null)).toBe("RUN");

    apply_battle_inputs(state.wram, joypad, state.input_state, { menu_active: true });

    expect(state.wram.confirm_pressed).toBe(false);
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);

    state.wram.wBattleMenuCursorPosition = 0;
    joypad.hJoyPressed = B_PAD_A;
    joypad.hJoypadPressed = B_PAD_A;
    apply_battle_inputs(state.wram, joypad, state.input_state, { menu_active: true });

    expect(get_player_input(state, [{ name: "TACKLE", current_pp: 35 }] as never, [{}] as never, null)).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.FIGHT);
  });
});

describe("battle-ui-input awaiting input logging", () => {
  it("syncs battle pack state from the real bag pockets so Poké Balls can be selected", () => {
    const dialogue = {
      window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
      dialogue: {
        is_complete: jest.fn(() => true),
        has_more_pages: jest.fn(() => false),
      },
      queue: [],
      pending_waits: 0,
      forced_visible: false,
      auto_close_after_display: false,
    };
    const state = {
      active: true,
      ui: {
        screen: {},
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
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 2,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
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
      dialogue,
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      pending_pack_action: null,
      pending_pokemon_selection: null,
      force_party_menu: false,
      game_state: {
        sram: {
          items: {},
          balls: { POKE_BALL: 5 },
          key_items: {},
          tm_hm: [],
          money: 0,
        },
        wram: {
          battle_type: "",
          wBattleTextDelay: 0,
        },
        hram: {},
      },
      audio_engine: null,
      data_loader: null,
    } as unknown as import("./battle-ui-state").BattleUIState;

    const openAction = get_player_input(state, [], [{}] as any, {});

    expect(openAction).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.PACK);
    expect(state.wram.last_item_names).toEqual(["CANCEL"]);

    _dispatch_bag_event(state, gameEngine.K_RIGHT);

    expect(state.wram.last_item_names).toEqual(["POKE_BALL", "CANCEL"]);
    expect(state.wram.wPackMenuCursorPosition).toBe(0);

    _dispatch_bag_event(state, buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN);
    expect(state.pending_pack_action).toBeNull();

    _dispatch_bag_event(state, buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN);
    expect(state.pending_pack_action).toEqual(["use", "POKE_BALL"]);

    const resolvedAction = get_player_input(state, [], [{}] as any, {});

    expect(resolvedAction).toBe("POKE_BALL");
    expect(state.wram.current_menu).toBe(BattleMenu.MAIN);
    expect(state.wram.last_item_names).toEqual([]);
  });

  it("consumes a pack direction press so one right input does not skip the BALL pocket", () => {
    const dialogue = {
      window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
      dialogue: {
        is_complete: jest.fn(() => true),
        has_more_pages: jest.fn(() => false),
      },
      queue: [],
      pending_waits: 0,
      forced_visible: false,
      auto_close_after_display: false,
    };
    const joypad = {
      hJoyPressed: B_PAD_RIGHT,
      hJoypadPressed: B_PAD_RIGHT,
      hJoyDown: B_PAD_RIGHT,
    };
    const state = {
      active: true,
      ui: {
        screen: {},
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
      wram: {
        current_menu: BattleMenu.MAIN,
        wBattleMenuCursorPosition: 2,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: true,
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
      dialogue,
      yes_no_prompt: {
        active: false,
        result: null,
        pending_activation: false,
        prompt: null,
      },
      pending_pack_action: null,
      pending_pokemon_selection: null,
      force_party_menu: false,
      game_state: {
        sram: {
          items: {},
          balls: { POKE_BALL: 5 },
          key_items: {},
          tm_hm: [],
          money: 0,
        },
        wram: {
          battle_type: "",
          wBattleTextDelay: 0,
        },
        hram: { joypad },
      },
      audio_engine: null,
      data_loader: null,
    } as unknown as import("./battle-ui-state").BattleUIState;

    get_player_input(state, [], [{}] as any, {});

    forward_pack_menu_inputs(state);
    expect(state.bag_menu?.getCurrentPocketLabel()).toBe("BALL");
    expect(state.wram.last_item_names).toEqual(["POKE_BALL", "CANCEL"]);
    expect(joypad.hJoyPressed).toBe(0);
    expect(joypad.hJoypadPressed).toBe(0);

    forward_pack_menu_inputs(state);
    expect(state.bag_menu?.getCurrentPocketLabel()).toBe("BALL");
  });

  it("does not write awaiting-input hot-path diagnostics to console", () => {
    const debugSpy = jest.spyOn(console, "debug").mockImplementation(() => {});
    const wram = {
      current_menu: BattleMenu.MAIN,
      wBattleMenuCursorPosition: 0,
      wMoveMenuCursorPosition: 0,
      wPartyMenuCursorPosition: 0,
      wPackMenuCursorPosition: 0,
      confirm_pressed: false,
      cancel_pressed: false,
      select_pressed: false,
      swapping_move_index: null,
      last_num_moves: 0,
      last_party_size: 0,
      last_item_names: [],
    };
    const state = {
      wram,
      pending_pokemon_selection: null,
      pending_pack_action: null,
      force_party_menu: false,
    } as unknown as import("./battle-ui-state").BattleUIState;

    get_player_input(state, [], [], null);
    get_player_input(state, [], [], null);

    expect(debugSpy).not.toHaveBeenCalled();

    wram.wBattleMenuCursorPosition = 1;
    get_player_input(state, [], [], null);

    expect(debugSpy).not.toHaveBeenCalled();
    debugSpy.mockRestore();
  });
});

describe("battle-ui-input party selection restore", () => {
  it("throws instead of clamping an invalid pending party selection", () => {
    const state = {
      wram: {
        current_menu: BattleMenu.POKEMON,
        wBattleMenuCursorPosition: 0,
        wMoveMenuCursorPosition: 0,
        wPartyMenuCursorPosition: 0,
        wPackMenuCursorPosition: 0,
        confirm_pressed: false,
        cancel_pressed: false,
        select_pressed: false,
        swapping_move_index: null,
        last_num_moves: 0,
        last_party_size: 0,
        last_item_names: [],
      },
      pending_pokemon_selection: 3,
      pending_pack_action: null,
      force_party_menu: true,
    } as unknown as import("./battle-ui-state").BattleUIState;

    expect(() => get_player_input(state, [], [{}, {}] as any, null)).toThrow(
      "Battle UI restored invalid pending party selection 3 for party size 2",
    );
  });
});
