import { BattleMenu } from './_battle-menu';
import { begin_battle, force_party_menu_selection, should_block_state_advance } from './battle-ui-core';
import { BattleUIPhase, HPBarAnimationState } from './battle-ui-state';

describe('battle-ui-core', () => {
  it('keeps battler sprites visible when battle begins', () => {
    const playerHpAnimation = new HPBarAnimationState();
    playerHpAnimation.sync(2, 100);
    playerHpAnimation.sync(10, 100);
    const enemyHpAnimation = new HPBarAnimationState();
    enemyHpAnimation.sync(20, 120);
    const dialogueWindow = { clear: jest.fn() };
    const state = {
      active: false,
      waiting_for_input: true,
      manual_wait_override: true,
      dialogue_wait_gate_active: true,
      exp_animation: {},
      trainer_intro: {},
      trainer_victory: {},
      trainer_exit: {},
      evolution_animation: {},
      pending_evolutions: [1],
      active_evolution: {},
      block_on_pending_evolution: true,
      sprites_enabled: false,
      trainer_sprites_visible: true,
      trainer_send_out_seen: true,
      trainer_hud_visible: true,
      trainer_sprite_override_mode: 'both',
      trainer_overlay_player_visible: true,
      trainer_overlay_enemy_visible: true,
      dialogue: {
        window: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
        dialogue: dialogueWindow,
        queue: [],
        pending_waits: 0,
        forced_visible: false,
        auto_close_after_display: false,
      },
      animation_player: {
        reset: jest.fn(),
        runtime_state: {
          player_visible: true,
          enemy_visible: true,
        },
      },
      vram: {
        toggle_oam: jest.fn(),
        record_scroll: jest.fn(),
      },
      hardware: {
        set_scroll: jest.fn(),
      },
      wram: {
        current_menu: BattleMenu.FIGHT,
        menu_header: null,
        wBattleMenuCursorPosition: 2,
        wMoveMenuCursorPosition: 1,
        wPartyMenuCursorPosition: 1,
        wPackMenuCursorPosition: 1,
        wBattleHasJustStarted: 0,
        wBattleTextDelay: 5,
        wTextDelayFlags: 7,
        wInputType: 0,
        confirm_pressed: true,
        cancel_pressed: true,
        select_pressed: true,
        last_num_moves: 2,
        last_party_size: 3,
        wBattleMenuCursorPositionNext: 0,
        swapping_move_index: 0,
        last_item_names: ['POTION'],
      },
      game_state: {
        wram: {
          wBattleTextDelay: 5,
          wTextDelayFlags: 7,
        },
      },
      hp_animation_states: {
        player: playerHpAnimation,
        enemy: enemyHpAnimation,
      },
      player_sprite_frame: 1,
      enemy_sprite_frame: 1,
      frontpic_animation: { side: 'enemy', speed: 4 },
      _sprite_frame_timers: { key: 1 },
      _sprite_frame_indices: { key: 1 },
      _frontpic_animators: { key: {} },
      layout: {
        text_box: { tile_x: 0, tile_y: 0, width_tiles: 1, height_tiles: 1 },
      },
      tilemap_base: {
        drawWindow: jest.fn(),
      },
      ui_phase: BattleUIPhase.INACTIVE,
    } as any;
    const previousPlayerAnimation = state.hp_animation_states.player;

    begin_battle(state);

    expect(state.animation_player.reset).toHaveBeenCalled();
    expect(state.animation_player.runtime_state.player_visible).toBe(true);
    expect(state.animation_player.runtime_state.enemy_visible).toBe(true);
    expect(state.hp_animation_states.player).not.toBe(previousPlayerAnimation);
    expect(state.hp_animation_states.player.initialized).toBe(false);
    expect(state.hp_animation_states.enemy.initialized).toBe(false);
    expect(state.frontpic_animation).toBeNull();
    expect(state._frontpic_animators).toEqual({});
  });

  it('resets an out-of-range forced party cursor to the first row', () => {
    const state = {
      force_party_menu: false,
      pending_pokemon_selection: 3,
      wram: {
        current_menu: BattleMenu.MAIN,
        last_party_size: 0,
        wPartyMenuCursorPosition: 5,
      },
    } as any;

    force_party_menu_selection(state, 3, { preferred_index: 5 });

    expect(state.force_party_menu).toBe(true);
    expect(state.pending_pokemon_selection).toBeNull();
    expect(state.wram.current_menu).toBe(BattleMenu.POKEMON);
    expect(state.wram.last_party_size).toBe(3);
    expect(state.wram.wPartyMenuCursorPosition).toBe(0);
  });

  it('preserves the stored party cursor when forcing the menu without a preferred index', () => {
    const state = {
      force_party_menu: false,
      pending_pokemon_selection: null,
      wram: {
        current_menu: BattleMenu.MAIN,
        last_party_size: 0,
        wPartyMenuCursorPosition: 1,
      },
    } as any;

    force_party_menu_selection(state, 3);

    expect(state.wram.wPartyMenuCursorPosition).toBe(1);
  });

  it('does not block state advance on a stale dialogue phase with no real wait left', () => {
    const state = {
      waiting_for_input: false,
      manual_wait_override: false,
      dialogue_wait_gate_active: false,
      exp_animation: null,
      trainer_intro: null,
      trainer_victory: null,
      trainer_exit: null,
      evolution_animation: null,
      block_on_pending_evolution: false,
      block_on_move_learning: false,
      animation_player: {
        is_active: () => false,
      },
      dialogue: {
        queue: [],
        pending_waits: 0,
        dialogue: { is_complete: () => true, has_more_pages: () => false },
      },
      yes_no_prompt: {
        active: false,
      },
      game_state: {
        wram: {
          wBattleTextDelay: 0,
        },
      },
      ui_phase: BattleUIPhase.DIALOGUE,
    } as any;

    expect(should_block_state_advance(state)).toBe(false);
  });

  it('does not block state advance on a stale animation phase with no active animation', () => {
    const state = {
      waiting_for_input: false,
      manual_wait_override: false,
      dialogue_wait_gate_active: false,
      exp_animation: null,
      trainer_intro: null,
      trainer_victory: null,
      trainer_exit: null,
      evolution_animation: null,
      block_on_pending_evolution: false,
      block_on_move_learning: false,
      animation_player: {
        is_active: () => false,
      },
      dialogue: {
        queue: [],
        pending_waits: 0,
        dialogue: { is_complete: () => true, has_more_pages: () => false },
      },
      yes_no_prompt: {
        active: false,
      },
      game_state: {
        wram: {
          wBattleTextDelay: 0,
        },
      },
      ui_phase: BattleUIPhase.ANIMATION,
    } as any;

    expect(should_block_state_advance(state)).toBe(false);
  });

  it('blocks state advance while deferred post-animation battle events are still queued', () => {
    const state = {
      waiting_for_input: false,
      manual_wait_override: false,
      dialogue_wait_gate_active: false,
      exp_animation: null,
      trainer_intro: null,
      trainer_victory: null,
      trainer_exit: null,
      evolution_animation: null,
      block_on_pending_evolution: false,
      block_on_move_learning: false,
      pending_animation_events: [{ name: 'show_text' }],
      animation_player: {
        is_active: () => false,
      },
      dialogue: {
        queue: [],
        pending_waits: 0,
        dialogue: { is_complete: () => true, has_more_pages: () => false },
      },
      yes_no_prompt: {
        active: false,
      },
      game_state: {
        wram: {
          wBattleTextDelay: 0,
        },
      },
      ui_phase: BattleUIPhase.MENU,
    } as any;

    expect(should_block_state_advance(state)).toBe(true);
  });
});
