import { PlayerGender } from "../../core/enums";
import { createInitialGameState } from "../../core/state";
import type { BattleAnimationRuntime } from "./battle-bg-effects";
import { BattleMenu } from "./_battle-menu";
import * as battleScene from "./battle-scene";
import * as draw from "./battle-ui-draw";
import * as battleUiSprites from "./battle-ui-sprites";
import * as textOverlays from "../text-overlays";
import {
  advance_animation_player,
  apply_trainer_overlay,
  handle_pokemon_menu_selection,
  maybe_start_pending_trainer_exit,
  reset_battler_visibility,
  update,
} from "./battle-ui-render";
import { BattleUIPhase } from "./battle-ui-state";

const buildState = () => {
  const runtime = {
    player_visible: true,
    enemy_visible: true,
    player_offset_x: 0,
    enemy_offset_x: 0,
    player_sprite_override: null,
    enemy_sprite_override: null,
    player_sprite_type_override: null,
    enemy_sprite_type_override: null,
  };
  const state = {
    trainer_sprites_visible: true,
    trainer_sprite_override_mode: "enemy",
    trainer_overlay_player_visible: null,
    trainer_overlay_enemy_visible: null,
    trainer_send_out_seen: false,
    animation_player: { runtime_state: runtime },
    game_state: {
      wram: {
        other_trainer_class: "BIRD_KEEPER",
        player_gender: PlayerGender.MALE,
      },
    },
  } as unknown as import("./battle-ui-state").BattleUIState;
  return { state, runtime };
};

const buildTilemapStub = () => ({
  height: 1,
  tiles: [[0]],
  attributes: [[0]],
  markDirty: jest.fn(),
  clear_box: jest.fn(),
});

const stubFullRenderPath = () => {
  const spies = [
    jest.spyOn(draw, "menu_header_for_battle").mockReturnValue(null),
    jest.spyOn(draw, "render_text_window_band").mockImplementation(() => {}),
    jest.spyOn(draw, "draw_enemy_hud").mockImplementation(() => {}),
    jest.spyOn(draw, "draw_player_hud").mockImplementation(() => {}),
    jest.spyOn(draw, "draw_dialogue_or_menu").mockImplementation(() => {}),
    jest.spyOn(draw, "draw_move_forget_menu").mockImplementation(() => {}),
    jest.spyOn(draw, "draw_yes_no_prompt").mockImplementation(() => {}),
    jest.spyOn(battleScene, "render_battle_background").mockImplementation(() => {}),
    jest.spyOn(textOverlays, "render_battle_text_overlay").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "update_battle_sprite_frames").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "apply_palette_registers").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "draw_battle_sprites").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "draw_animation_sprites").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "overlay_move_windows").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "apply_runtime_postprocessing").mockImplementation(() => {}),
    jest.spyOn(battleUiSprites, "dispatch_animation_audio").mockImplementation(() => {}),
  ];
  return () => {
    for (const spy of spies) {
      spy.mockRestore();
    }
  };
};

const buildUpdateState = (
  overrides: Partial<import("./battle-ui-state").BattleUIState> = {},
) => {
  const runtime = {
    player_visible: true,
    enemy_visible: true,
    player_offset_x: 0,
    enemy_offset_x: 0,
    player_sprite_override: null,
    enemy_sprite_override: null,
    player_sprite_type_override: null,
    enemy_sprite_type_override: null,
    screen_offset_x: 0,
    screen_offset_y: 0,
    lcd_pointer: null,
    line_scroll_y: null,
  };
  const tilemapBase = buildTilemapStub();
  const tilemap = buildTilemapStub();
  const baseState = {
    active: true,
    context: null,
    ui: {
      screen: {},
      update: jest.fn(),
    },
    animation_clock: {
      tick: jest.fn(),
    },
    oam_manager: {
      reset: jest.fn(),
    },
    wram: {
      current_menu: BattleMenu.MAIN,
      menu_header: null,
    },
    game_state: {
      wram: {
        battle_type: "",
        other_trainer_class: "BIRD_KEEPER",
        wBattleTextDelay: 0,
        wBattleHasJustStarted: 1,
      },
      hram: {},
    },
    trainer_intro: null,
    animation_player: {
      runtime_state: runtime,
      is_active: jest.fn(() => false),
      update: jest.fn(),
      oam_enabled: true,
      current_animation_script: null,
    },
    dialogue: {
      forced_visible: false,
      pending_waits: 0,
      queue: [],
      auto_close_after_display: false,
      dialogue: {
        is_complete: jest.fn(() => true),
        has_more_pages: jest.fn(() => false),
        update: jest.fn(),
      },
    },
    yes_no_prompt: {
      active: false,
    },
    evolution_animation: null,
    pending_evolutions: [],
    active_evolution: null,
    trainer_victory: null,
    trainer_exit: null,
    active_move_learn: null,
    pending_move_learns: [],
    pending_nickname_request: null,
    move_forget_menu: null,
    manual_wait_override: false,
    block_on_pending_evolution: false,
    block_on_move_learning: false,
    dialogue_wait_gate_active: false,
    sprites_enabled: true,
    trainer_send_out_seen: false,
    trainer_sprites_visible: true,
    trainer_hud_visible: true,
    trainer_sprite_override_mode: "both",
    trainer_overlay_player_visible: null,
    trainer_overlay_enemy_visible: null,
    pending_trainer_exit: false,
    pending_trainer_exit_side: null,
    waiting_for_input: false,
    ui_phase: BattleUIPhase.MENU,
    presented_this_frame: false,
    vram: {
      toggle_oam: jest.fn(),
      record_scroll: jest.fn(),
    },
    hardware: {
      scx: 0,
      scy: 0,
      set_scroll: jest.fn(),
    },
    tilemap_base: tilemapBase,
    tilemap,
    tileset: {},
    layout: {
      text_box: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
    },
  } as unknown as import("./battle-ui-state").BattleUIState;

  return {
    state: Object.assign(baseState, overrides),
    runtime,
  };
};

describe("apply_trainer_overlay", () => {
  it("clears trainer overlays as soon as send-out starts", () => {
    const { state, runtime } = buildState();

    apply_trainer_overlay(state, {
      trainerOverlayActive: true,
      battleStartActive: true,
      sendOutActive: true,
      tutorialBattle: false,
    });

    expect(state.trainer_sprites_visible).toBe(false);
    expect(state.trainer_sprite_override_mode).toBeNull();
    expect(runtime.enemy_sprite_override).toBeNull();
    expect(runtime.enemy_sprite_type_override).toBeNull();
    expect(runtime.player_sprite_override).toBeNull();
    expect(runtime.player_sprite_type_override).toBeNull();
    expect(runtime.player_visible).toBe(false);
    expect(runtime.enemy_visible).toBe(true);
  });

  it("keeps the enemy trainer visible after the player sprite clears", () => {
    const { state, runtime } = buildState();

    apply_trainer_overlay(state, {
      trainerOverlayActive: true,
      battleStartActive: true,
      sendOutActive: false,
      tutorialBattle: false,
    });

    expect(state.trainer_sprites_visible).toBe(true);
    expect(state.trainer_sprite_override_mode).toBe("enemy");
    expect(runtime.enemy_sprite_override).toBe("bird_keeper");
    expect(runtime.enemy_sprite_type_override).toBe("trainer");
    expect(runtime.player_sprite_override).toBeNull();
    expect(runtime.player_visible).toBe(false);
  });

  it("uses the dude back sprite during the catching tutorial", () => {
    const { state, runtime } = buildState();
    state.trainer_sprite_override_mode = "player";
    state.game_state.wram.battle_type = "BATTLETYPE_TUTORIAL";

    apply_trainer_overlay(state, {
      trainerOverlayActive: false,
      battleStartActive: true,
      sendOutActive: false,
      tutorialBattle: true,
    });

    expect(runtime.player_sprite_type_override).toBe("player_back");
    expect(runtime.player_sprite_override).toBe("dude");
  });

  it("activates the dude back sprite even when tutorial battles skip trainer intro setup", () => {
    const { state, runtime } = buildState();
    state.trainer_sprites_visible = false;
    state.trainer_sprite_override_mode = null;
    state.game_state.wram.battle_type = "BATTLETYPE_TUTORIAL";

    apply_trainer_overlay(state, {
      trainerOverlayActive: false,
      battleStartActive: true,
      sendOutActive: false,
      tutorialBattle: true,
    });

    expect(state.trainer_sprites_visible).toBe(true);
    expect(state.trainer_sprite_override_mode).toBe("player");
    expect(runtime.player_sprite_type_override).toBe("player_back");
    expect(runtime.player_sprite_override).toBe("dude");
    expect(runtime.player_visible).toBe(true);
  });

  it("hides the dude back sprite while the tutorial throw animation is active", () => {
    const { state, runtime } = buildState();
    state.trainer_sprites_visible = true;
    state.trainer_sprite_override_mode = "player";
    state.game_state.wram.battle_type = "BATTLETYPE_TUTORIAL";

    apply_trainer_overlay(state, {
      trainerOverlayActive: false,
      battleStartActive: false,
      sendOutActive: false,
      tutorialBattle: true,
      throwPokeballActive: true,
    });

    expect(runtime.player_sprite_type_override).toBe("player_back");
    expect(runtime.player_sprite_override).toBe("dude");
    expect(runtime.player_visible).toBe(false);
  });

  it("restores the player backpic after enemy send-out when BattleMonEntrance runs", () => {
    const { state, runtime } = buildState();
    state.trainer_sprites_visible = true;
    state.trainer_sprite_override_mode = "player";
    state.trainer_send_out_seen = false;
    runtime.player_visible = false;
    runtime.enemy_visible = true;

    apply_trainer_overlay(state, {
      trainerOverlayActive: true,
      battleStartActive: true,
      sendOutActive: false,
      tutorialBattle: false,
    });

    expect(runtime.player_sprite_type_override).toBe("player_back");
    expect(runtime.player_sprite_override).toBe("chris_back");
    expect(runtime.player_visible).toBe(true);
    expect(runtime.enemy_sprite_override).toBeNull();
  });

  it("keeps trainer overlays visible until the send-out animation has actually been seen", () => {
    const { state, runtime } = buildState();
    state.trainer_sprites_visible = true;
    state.trainer_sprite_override_mode = "both";
    state.trainer_send_out_seen = false;

    apply_trainer_overlay(state, {
      trainerOverlayActive: true,
      battleStartActive: false,
      sendOutActive: false,
      tutorialBattle: false,
    });

    expect(state.trainer_sprites_visible).toBe(true);
    expect(state.trainer_sprite_override_mode).toBe("both");
    expect(runtime.enemy_sprite_type_override).toBe("trainer");
    expect(runtime.enemy_sprite_override).toBe("bird_keeper");
    expect(runtime.player_sprite_type_override).toBe("player_back");
    expect(runtime.player_sprite_override).toBe("chris_back");
  });
});

describe("update", () => {
  it("clears trainer hud balls on the frame where the intro finishes", () => {
    const drawTrainerHudIcons = jest
      .spyOn(battleUiSprites, "draw_trainer_hud_icons")
      .mockImplementation(() => {});
    const state = {
      active: true,
      context: null,
      ui: {
        screen: {},
        update: jest.fn(),
      },
      animation_clock: {
        tick: jest.fn(),
      },
      oam_manager: {
        reset: jest.fn(),
      },
      wram: {},
      game_state: {
        wram: {
          battle_type: "",
          other_trainer_class: "BIRD_KEEPER",
          wBattleTextDelay: 0,
        },
        hram: {},
      },
      trainer_intro: {
        is_finished: true,
        draw: jest.fn(),
      },
      animation_player: {
        runtime_state: {
          player_visible: true,
          enemy_visible: true,
        },
        is_active: jest.fn(() => false),
      },
      dialogue: {
        forced_visible: false,
        pending_waits: 0,
        queue: [],
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
        },
      },
      yes_no_prompt: {
        active: false,
      },
      evolution_animation: null,
      trainer_victory: null,
      trainer_exit: null,
      manual_wait_override: false,
      block_on_pending_evolution: false,
      block_on_move_learning: false,
      dialogue_wait_gate_active: false,
      sprites_enabled: false,
      trainer_send_out_seen: true,
      trainer_hud_visible: true,
      trainer_sprites_visible: false,
      trainer_sprite_override_mode: null,
      waiting_for_input: false,
      ui_phase: BattleUIPhase.MENU,
    } as unknown as import("./battle-ui-state").BattleUIState;
    const battleContext = {
      trainerBattle: true,
    } as unknown as import("../../engine/battle/battle/battle-context").BattleContext;

    update(state, battleContext);

    expect(drawTrainerHudIcons).toHaveBeenCalledTimes(1);
    expect(state.trainer_intro).toBeNull();
    expect(state.sprites_enabled).toBe(true);
    expect(state.trainer_send_out_seen).toBe(false);
    expect(state.trainer_hud_visible).toBe(false);
    drawTrainerHudIcons.mockRestore();
  });

  it("keeps trainer hud balls visible while trainer exit is queued", () => {
    const restore = stubFullRenderPath();
    const drawTrainerHudIcons = jest
      .spyOn(battleUiSprites, "draw_trainer_hud_icons")
      .mockImplementation(() => {});
    const { state } = buildUpdateState({
      pending_trainer_exit: true,
      pending_trainer_exit_side: "player",
      dialogue: {
        forced_visible: true,
        pending_waits: 0,
        queue: [],
        auto_close_after_display: false,
        dialogue: {
          is_complete: jest.fn(() => true),
          has_more_pages: jest.fn(() => false),
          update: jest.fn(),
        },
      },
    });
    const battleContext = {
      trainerBattle: true,
    } as unknown as import("../../engine/battle/battle/battle-context").BattleContext;

    update(state, battleContext);

    expect(state.pending_trainer_exit).toBe(true);
    expect(state.trainer_hud_visible).toBe(true);
    expect(drawTrainerHudIcons).toHaveBeenCalled();
    drawTrainerHudIcons.mockRestore();
    restore();
  });

  it("keeps trainer hud balls visible while trainer exit is active", () => {
    const restore = stubFullRenderPath();
    const drawTrainerHudIcons = jest
      .spyOn(battleUiSprites, "draw_trainer_hud_icons")
      .mockImplementation(() => {});
    const exitDraw = jest.fn();
    const { state } = buildUpdateState({
      trainer_exit: {
        draw: exitDraw,
        is_finished: false,
        target_side: "player",
        x_offset: 8,
      } as unknown as import("./battle-intro").TrainerExitAnimation,
    });
    const battleContext = {
      trainerBattle: true,
    } as unknown as import("../../engine/battle/battle/battle-context").BattleContext;

    update(state, battleContext);

    expect(exitDraw).toHaveBeenCalledTimes(1);
    expect(state.trainer_exit).not.toBeNull();
    expect(state.trainer_hud_visible).toBe(true);
    expect(drawTrainerHudIcons).toHaveBeenCalled();
    drawTrainerHudIcons.mockRestore();
    restore();
  });

  it("clears trainer hud balls once trainer exit has finished", () => {
    const restore = stubFullRenderPath();
    const drawTrainerHudIcons = jest
      .spyOn(battleUiSprites, "draw_trainer_hud_icons")
      .mockImplementation(() => {});
    const { state, runtime } = buildUpdateState({
      trainer_exit: {
        draw: jest.fn(),
        is_finished: true,
        target_side: "player",
        x_offset: 8,
      } as unknown as import("./battle-intro").TrainerExitAnimation,
      trainer_overlay_player_visible: true,
      trainer_overlay_enemy_visible: true,
    });
    const battleContext = {
      trainerBattle: true,
    } as unknown as import("../../engine/battle/battle/battle-context").BattleContext;

    update(state, battleContext);

    expect(state.trainer_exit).toBeNull();
    expect(state.trainer_hud_visible).toBe(false);
    expect(state.trainer_sprites_visible).toBe(false);
    expect(runtime.player_offset_x).toBe(0);
    expect(drawTrainerHudIcons).toHaveBeenCalled();
    drawTrainerHudIcons.mockRestore();
    restore();
  });
});

describe("maybe_start_pending_trainer_exit", () => {
  it("waits until the dialogue is no longer visible", () => {
    const state = {
      pending_trainer_exit: true,
      pending_trainer_exit_side: "player",
      trainer_exit: null,
      trainer_sprites_visible: false,
      trainer_sprite_override_mode: null,
      trainer_intro: null,
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

    maybe_start_pending_trainer_exit(state);

    expect(state.pending_trainer_exit).toBe(true);
    expect(state.trainer_sprite_override_mode).toBeNull();
  });

  it("starts player slide-out with the player trainer sprite still visible", () => {
    const ui = {
      get_sprite_surface: jest.fn(() => null),
      _apply_colorkey_transparency: jest.fn((surface) => surface),
      _get_pokemon_frame_surface: jest.fn(() => null),
    };
    const state = {
      pending_trainer_exit: true,
      pending_trainer_exit_side: "player",
      trainer_exit: null,
      trainer_sprites_visible: false,
      trainer_sprite_override_mode: null,
      trainer_intro: null,
      ui,
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

    maybe_start_pending_trainer_exit(state);

    expect(state.pending_trainer_exit).toBe(false);
    expect(state.pending_trainer_exit_side).toBeNull();
    expect(state.trainer_sprite_override_mode).toBe("player");
    expect(state.trainer_sprites_visible).toBe(true);
    expect(state.trainer_exit).not.toBeNull();
    expect((state.trainer_exit as { target_side?: string } | null)?.target_side).toBe("player");
  });
});

describe("reset_battler_visibility", () => {
  it("marks both battlers visible even if they were hidden", () => {
    const runtime = { player_visible: false, enemy_visible: false } as unknown as BattleAnimationRuntime;
    reset_battler_visibility(runtime);
    expect(runtime.player_visible).toBe(true);
    expect(runtime.enemy_visible).toBe(true);
  });
});

describe("advance_animation_player", () => {
  it("speeds up the tutorial throw animation when fast-forward is requested", () => {
    const update = jest.fn();
    const isActive = jest.fn(() => true);
    const state = {
      fast_animation_request: true,
      animation_player: {
        update,
        is_active: isActive,
        current_animation_script: { name: "BattleAnim_ThrowPokeBall" },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    advance_animation_player(state, { tutorialBattle: true });

    expect(update.mock.calls.length).toBeGreaterThan(1);
    expect(state.fast_animation_request).toBe(false);
  });

  it("runs a single animation tick when fast-forward is unavailable", () => {
    const update = jest.fn();
    const state = {
      fast_animation_request: true,
      animation_player: {
        update,
        is_active: jest.fn(() => true),
        current_animation_script: { name: "BattleAnim_Tackle" },
      },
    } as unknown as import("./battle-ui-state").BattleUIState;

    advance_animation_player(state, { tutorialBattle: true });

    expect(update).toHaveBeenCalledTimes(1);
    expect(state.fast_animation_request).toBe(false);
  });
});

describe("handle_pokemon_menu_selection", () => {
  it("throws instead of silently falling back to wCurPartyMon for non-party selections", () => {
    const gameState = createInitialGameState();
    const state = {
      game_state: gameState,
      wram: { wPartyMenuCursorPosition: 0 },
      pending_pokemon_selection: null,
    } as unknown as import("./battle-ui-state").BattleUIState;

    expect(() =>
      handle_pokemon_menu_selection(state, { species: { id: "MEW" } }),
    ).toThrow(
      "Battle Pokemon menu selected a party member that is not in the current party.",
    );
    expect(state.pending_pokemon_selection).toBeNull();
    expect(state.wram.wPartyMenuCursorPosition).toBe(0);
  });
});
