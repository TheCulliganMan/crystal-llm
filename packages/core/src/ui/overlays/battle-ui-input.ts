import { gameEngine } from '../game-engine';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';
import { isDebugEnabled } from '@pokecrystal/core/core/debug-flags';
import type { GameEngineEvent } from '../game-engine';
import {
  GameButton,
  buttonKeys,
  isButtonEvent,
  isConfirmEvent,
  isKeyDownEvent,
  type KeyEvent,
} from '../../input/buttons';
import {
  B_PAD_A,
  B_PAD_B,
  B_PAD_DOWN,
  B_PAD_LEFT,
  B_PAD_RIGHT,
  B_PAD_START,
  B_PAD_UP,
} from '../../input/controls';
import { Event } from '../../engine/world/events';
import type { LearnedMove, Pokemon } from '../../core/models';
import type { MoveName } from '../../core/enums';
import type { GameState } from '../../core/state';
import { BagMenu } from '../menus/bag-menu';
import { PokemonMenu } from '../menus/pokemon-menu';
import * as battle_dialogue from './battle-dialogue';
import { BattleMenu, MenuDirection } from './_battle-menu';
import { apply_battle_inputs, ensure_menu_cursor } from './battle-input';
import {
  _describe_wram,
  _dialogue_wait_gate_active,
  dialogue_ready_for_yes_no,
  release_force_party_menu,
  show_trainer_sprites,
  start_trainer_exit_animation,
} from './battle-ui-core';
import { BattleUIPhase, BattleUIState, MoveLearningPhase, type TrainerSpriteMode } from './battle-ui-state';
import { _close_move_forget_menu } from './battle-ui-moves';
import { createBagMenuUI } from './battle-ui-menu-utils';
import { YesNoPrompt, type DialogueUI } from '../text/dialogue';

const INITIAL_REPEAT_DELAY_FRAMES = 8;
const REPEAT_INTERVAL_FRAMES = 4;

const JOY_UP = B_PAD_UP;
const JOY_DOWN = B_PAD_DOWN;
const JOY_LEFT = B_PAD_LEFT;
const JOY_RIGHT = B_PAD_RIGHT;
const JOY_A = B_PAD_A;
const JOY_B = B_PAD_B;

const BUG_CONTEST_BATTLE_TYPES = new Set(['BATTLETYPE_CONTEST', 'BATTLETYPE_BUG_CONTEST', 'BATTLETYPE_PARK']);
const lastBlockedDelay = new WeakMap<BattleUIState, number>();
const lastAwaitingInputLog = new WeakMap<BattleUIState, string>();
const traceBattleInput = (): boolean => isDebugEnabled("battle:input") || isDebugEnabled("battle");

const should_defer_event_until_animation = (state: BattleUIState, event: Event): boolean => {
  const payload = event.data ?? {};
  if (!payload.wait_for_animation) {
    return false;
  }
  if (state.game_state?.wram?.instant_mode) {
    return false;
  }
  return Boolean(state.animation_player?.is_active?.());
};

const enqueue_pending_animation_event = (state: BattleUIState, event: Event): void => {
  if (!state.pending_animation_events) {
    state.pending_animation_events = [];
  }
  state.pending_animation_events.push(event);
};

export const flush_deferred_animation_events = (state: BattleUIState): void => {
  if (!state.pending_animation_events?.length) {
    return;
  }
  if (state.animation_player?.is_active?.()) {
    return;
  }
  const pending = [...state.pending_animation_events];
  state.pending_animation_events.length = 0;
  for (const queued of pending) {
    handle_event(state, queued, state.game_state);
  }
};

const map_key_to_direction = (key: string | number | null | undefined): string | null => {
  if (key === null || key === undefined) {
    return null;
  }
  const value = String(key);
  if (value === gameEngine.K_UP) {
    return 'up';
  }
  if (value === gameEngine.K_DOWN) {
    return 'down';
  }
  if (value === gameEngine.K_LEFT) {
    return 'left';
  }
  if (value === gameEngine.K_RIGHT) {
    return 'right';
  }
  return null;
};

const _consume_manual_joypad_press = (state: BattleUIState, bit: number): void => {
  const joypad = (state.game_state?.hram as {
    joypad?: {
      hJoyPressed?: number;
      hJoypadPressed?: number;
    };
  } | null | undefined)?.joypad;
  if (!joypad) {
    return;
  }
  joypad.hJoyPressed = (joypad.hJoyPressed ?? 0) & ~bit;
  joypad.hJoypadPressed = (joypad.hJoypadPressed ?? 0) & ~bit;
};

const _consume_manual_joypad_presses = (state: BattleUIState, bits: number): void => {
  const joypad = (state.game_state?.hram as {
    joypad?: {
      hJoyPressed?: number;
      hJoypadPressed?: number;
      hJoyDown?: number;
      hJoypadDown?: number;
    };
  } | null | undefined)?.joypad;
  if (!joypad) {
    return;
  }
  joypad.hJoyPressed = (joypad.hJoyPressed ?? 0) & ~bits;
  joypad.hJoypadPressed = (joypad.hJoypadPressed ?? 0) & ~bits;
  joypad.hJoyDown = (joypad.hJoyDown ?? 0) & ~bits;
  joypad.hJoypadDown = (joypad.hJoypadDown ?? 0) & ~bits;
};

const _clear_manual_menu_button_latches = (state: BattleUIState): void => {
  state.wram.confirm_pressed = false;
  state.wram.cancel_pressed = false;
  state.wram.select_pressed = false;
};

const _handle_manual_battle_menu_input = (state: BattleUIState, event: GameEngineEvent & KeyEvent): boolean => {
  const wram = state.wram;
  if (wram.current_menu !== BattleMenu.MAIN && wram.current_menu !== BattleMenu.FIGHT) {
    return false;
  }
  const direction = map_key_to_direction(event.key ?? event.code ?? null);
  if (!direction) {
    return false;
  }
  const bit =
    direction === 'up'
      ? JOY_UP
      : direction === 'down'
        ? JOY_DOWN
        : direction === 'left'
          ? JOY_LEFT
          : JOY_RIGHT;
  apply_battle_inputs(
    wram,
    { hJoyPressed: bit, hJoypadPressed: bit, hJoyDown: bit, hJoypadDown: bit },
    state.input_state,
    { menu_active: !battle_dialogue.is_visible(state.dialogue) },
  );
  _consume_manual_joypad_press(state, bit);
  return true;
};

export const handle_event = (state: BattleUIState, event: Event, _game_state?: GameState | null): void => {
  if (!state.active) {
    return;
  }
  if (
    state.game_state?.wram?.instant_mode &&
    (event.name === 'show_text' || event.name === 'wait_for_input' || event.name === 'open_text')
  ) {
    return;
  }
  if (should_defer_event_until_animation(state, event)) {
    enqueue_pending_animation_event(state, event);
    return;
  }
  if (event.name === 'show_text') {
    state.ui_phase = BattleUIPhase.DIALOGUE;
    const payload = event.data ?? {};
    const control_code = payload.control as string | null | undefined;
    const hasText = battle_dialogue.enqueue_text(state.dialogue, String(payload.text ?? ''), {
      control: control_code ?? null,
    });
    let shouldWait = payload.wait ?? true;
    if (control_code === 'done') {
      shouldWait = false;
    }
    if (hasText && shouldWait) {
      battle_dialogue.push_wait(state.dialogue);
    }
    return;
  }
  if (event.name === 'trigger_trainer_exit') {
    const payload = event.data ?? {};
    const side = typeof payload.side === 'string' ? payload.side : undefined;
    const dialogueActive =
      state.dialogue.forced_visible ||
      battle_dialogue.waiting_flag(state.dialogue);
    if (state.trainer_intro || dialogueActive) {
      state.pending_trainer_exit = true;
      state.pending_trainer_exit_side = side ?? null;
      return;
    }
    if (side === 'player') {
      // ASM mapping: engine/battle/core.asm::SlideBattlePicOut (a=9) slides the player backpic out.
      // Keep the player trainer sprite visible so the slide-out is rendered.
      state.trainer_sprite_override_mode = 'player';
      state.trainer_sprites_visible = true;
    }
    start_trainer_exit_animation(state, side ? { side } : undefined);
    return;
  }
  if (event.name === 'show_trainer_sprites') {
    const payload = event.data ?? {};
    const mode = typeof payload.mode === 'string' ? payload.mode : null;
    const normalizedMode: TrainerSpriteMode | null =
      mode === 'player' || mode === 'enemy' || mode === 'both' ? mode : null;
    show_trainer_sprites(state, normalizedMode ? { mode: normalizedMode } : undefined);
    return;
  }
  if (event.name === 'play_animation') {
    const payload = event.data ?? {};
    const moveNameSource =
      typeof payload.animation_label === "string" && payload.animation_label.trim().length
        ? payload.animation_label
        : typeof payload.move_name === "string"
          ? payload.move_name
          : String(payload.move_name ?? "");
    const param = typeof payload.param === "number" ? payload.param : null;
    const paramLabel = typeof payload.param_label === "string" ? payload.param_label : null;
    const shakeCount = typeof payload.shake_count === "number" ? payload.shake_count : null;
    state.animation_player.play_animation(moveNameSource, Boolean(payload.is_player_move), param, {
      param_label: paramLabel,
      shake_count: shakeCount,
    });
    return;
  }
  if (event.name === "frontpic_animation") {
    const payload = event.data ?? {};
    const side = payload.side === "player" || payload.side === "enemy" ? payload.side : "enemy";
    const speed = Math.max(0, Math.trunc(Number(payload.speed ?? 0)));
    state.frontpic_animation = { side, speed };
    return;
  }
  if (event.name === 'wait_for_input') {
    battle_dialogue.push_wait(state.dialogue);
    return;
  }
  if (event.name === 'open_text') {
    battle_dialogue.force_text_box(state.dialogue, true);
    return;
  }
  if (event.name === 'close_text') {
    battle_dialogue.close_text_box(state.dialogue);
    state.manual_wait_override = false;
    return;
  }
  if (event.name === 'nickname_prompt') {
    const payload = event.data ?? {};
    const speciesName = String(payload.species_name ?? 'POKEMON');
    start_yes_no_prompt(state, `Give a nickname to ${speciesName}?`);
    state.pending_nickname_request = {
      pokemon: (payload.pokemon as Pokemon) ?? null,
      species_name: speciesName,
    };
    return;
  }
  if (event.name === 'prompt_yes_no') {
    const payload = event.data ?? {};
    const text = String(payload.text ?? '');
    if (text) {
      start_yes_no_prompt(state, text);
    }
  }
};

export const handle_show_text = (state: BattleUIState, event: Event): void => {
  handle_event(state, event, state.game_state);
};

const start_yes_no_prompt = (state: BattleUIState, text: string): void => {
  state.ui_phase = BattleUIPhase.DIALOGUE;
  battle_dialogue.enqueue_text(state.dialogue, text);
  battle_dialogue.push_wait(state.dialogue);
  const promptState = state.yes_no_prompt;
  const ui = state.ui as unknown as DialogueUI | null | undefined;
  const prompt = ui ? (promptState.prompt ?? new YesNoPrompt(ui, state.audio_engine ?? null)) : null;
  if (prompt) {
    prompt.selection = 0;
    prompt.finished = false;
    promptState.prompt = prompt;
  }
  promptState.result = null;
  promptState.pending_activation = false;
  if (dialogue_ready_for_yes_no(state)) {
    promptState.active = true;
    battle_dialogue.force_text_box(state.dialogue, true);
  } else {
    promptState.active = false;
    promptState.pending_activation = true;
  }
  state.manual_wait_override = true;
};

export const handle_input = (state: BattleUIState, event: GameEngineEvent & KeyEvent): void => {
  if (!state.active || !isKeyDownEvent(event)) {
    return;
  }
  if (state.wram.current_menu === BattleMenu.PACK) {
    const direction = map_key_to_direction(event.key ?? event.code ?? null);
    if (direction) {
      dispatch_bag_direction(
        state,
        direction === "up"
          ? MenuDirection.UP
          : direction === "down"
            ? MenuDirection.DOWN
            : direction === "left"
              ? MenuDirection.LEFT
              : MenuDirection.RIGHT
      );
      _consume_manual_joypad_presses(state, JOY_UP | JOY_DOWN | JOY_LEFT | JOY_RIGHT | JOY_A | JOY_B | B_PAD_START);
      _clear_manual_menu_button_latches(state);
      return;
    }
  }
  if (state.wram.current_menu === BattleMenu.POKEMON && isButtonEvent(event, GameButton.B)) {
    if (!state.force_party_menu) {
      state.pending_pokemon_selection = null;
      state.pokemon_menu = null;
      state.wram.current_menu = BattleMenu.MAIN;
      release_force_party_menu(state);
      ensure_menu_cursor(state.wram);
    }
    return;
  }
  if (handle_move_forget_input(state, event)) {
    return;
  }
  if (handle_yes_no_input(state, event)) {
    return;
  }
  if (_handle_manual_battle_menu_input(state, event)) {
    return;
  }
  const advanceButtons = [GameButton.A, GameButton.B, GameButton.Start];
  const advancePressed = advanceButtons.some((button) => isButtonEvent(event, button));
  const dialogueActive =
    battle_dialogue.is_visible(state.dialogue) || battle_dialogue.waiting_flag(state.dialogue);
  let fastRequest = false;
  if (advancePressed) {
    fastRequest = !state.dialogue.dialogue.is_complete();
    if (dialogueActive) {
      _clear_manual_menu_button_latches(state);
    }
  }
  const delayGate = _dialogue_wait_gate_active(state);
  if (!state.game_state?.wram?.instant_mode && delayGate && state.game_state.wram.wBattleTextDelay > 0) {
    const delay = state.game_state.wram.wBattleTextDelay;
    const lastLogged = lastBlockedDelay.get(state);
    if (lastLogged !== delay && advancePressed) {
      lastBlockedDelay.set(state, delay);
      pushDebugLog(`[battle] input blocked by text delay (${delay})`);
    }
    return;
  }
  const consumed = battle_dialogue.consume_input(state.dialogue, event);
  if (consumed) {
    _consume_manual_joypad_presses(state, JOY_A | JOY_B | B_PAD_START);
    _clear_manual_menu_button_latches(state);
    state.fast_text_request = fastRequest;
    state.manual_wait_override = false;
    if (advancePressed) {
      pushDebugLog('[battle] input advanced dialogue');
    }
  } else {
    state.fast_text_request = false;
    if (advancePressed) {
      pushDebugLog('[battle] input had no effect');
    }
  }
};

export const get_player_input = (
  state: BattleUIState,
  moves: LearnedMove[],
  player_party: Pokemon[],
  items?: Record<string, unknown> | null,
): MoveName | number | string | null => {
  const wram = state.wram;
  const moveList = moves ?? [];
  if (wram.current_menu !== BattleMenu.FIGHT) {
    wram.swapping_move_index = null;
  }
  wram.last_num_moves = moveList.length;
  wram.last_party_size = player_party?.length ?? 0;
  wram.last_item_names = items ? Object.keys(items) : [];
  let action: MoveName | number | string | null = null;
  const pendingIndex = state.pending_pokemon_selection;
  if (pendingIndex !== null && pendingIndex !== undefined) {
    if (pendingIndex < 0 || pendingIndex >= wram.last_party_size) {
      throw new Error(
        `Battle UI restored invalid pending party selection ${pendingIndex} for party size ${wram.last_party_size}`,
      );
    }
    action = pendingIndex;
    state.pending_pokemon_selection = null;
    wram.current_menu = BattleMenu.MAIN;
    wram.wPartyMenuCursorPosition = pendingIndex;
    ensure_menu_cursor(wram);
    release_force_party_menu(state);
    if (traceBattleInput()) {
      pushDebugLog(`[battle] selected party slot ${action}`);
    }
  } else if (wram.current_menu === BattleMenu.MAIN) {
    ensure_menu_cursor(wram);
    if (wram.confirm_pressed) {
      const idx = wram.wBattleMenuCursorPosition;
      if (idx === 0) {
        wram.current_menu = BattleMenu.FIGHT;
        wram.wMoveMenuCursorPosition = 0;
        wram.swapping_move_index = null;
        wram.select_pressed = false;
        _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
      } else if (idx === 1) {
        wram.current_menu = BattleMenu.POKEMON;
        _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
      } else if (idx === 2) {
        const battleType = String((state.game_state.wram as { battle_type?: string }).battle_type ?? '').toUpperCase();
        if (BUG_CONTEST_BATTLE_TYPES.has(battleType)) {
          action = 'PARK_BALL';
          _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
        } else {
          wram.current_menu = BattleMenu.PACK;
          state.pending_pack_action = null;
          state.bag_menu = new BagMenu(
            createBagMenuUI(state.ui),
            state.game_state,
            state.audio_engine ?? null,
            state.data_loader ?? undefined
          );
          sync_pack_menu_state(state);
          _consume_manual_joypad_presses(state, JOY_A | JOY_B | B_PAD_START);
          _clear_manual_menu_button_latches(state);
        }
      } else {
        action = 'RUN';
        _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
      }
    }
  } else if (wram.current_menu === BattleMenu.FIGHT) {
    const total = wram.last_num_moves + 1;
    if (total <= 0) {
      wram.current_menu = BattleMenu.MAIN;
      wram.wMoveMenuCursorPosition = 0;
      ensure_menu_cursor(wram);
      wram.swapping_move_index = null;
    } else if (wram.select_pressed) {
      handle_move_swap(state, moveList);
    } else if (wram.cancel_pressed) {
      wram.current_menu = BattleMenu.MAIN;
      wram.wMoveMenuCursorPosition = 0;
      ensure_menu_cursor(wram);
      wram.swapping_move_index = null;
    } else if (wram.confirm_pressed) {
      if (wram.wMoveMenuCursorPosition >= wram.last_num_moves) {
        wram.current_menu = BattleMenu.MAIN;
        ensure_menu_cursor(wram);
        wram.swapping_move_index = null;
        _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
      } else {
        action = moveList[wram.wMoveMenuCursorPosition]?.name ?? null;
        wram.current_menu = BattleMenu.MAIN;
        ensure_menu_cursor(wram);
        wram.swapping_move_index = null;
        _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
        if (traceBattleInput()) {
          pushDebugLog(`[battle] selected move ${action}`);
        }
      }
    }
  } else if (wram.current_menu === BattleMenu.POKEMON) {
    const cancelAllowed = !state.force_party_menu;
    if ((wram.cancel_pressed && cancelAllowed) || wram.last_party_size === 0) {
      wram.current_menu = BattleMenu.MAIN;
      ensure_menu_cursor(wram);
      release_force_party_menu(state);
    } else if (wram.confirm_pressed) {
      action = Number(wram.wPartyMenuCursorPosition);
      wram.current_menu = BattleMenu.MAIN;
      ensure_menu_cursor(wram);
      release_force_party_menu(state);
      _consume_manual_joypad_presses(state, JOY_A | B_PAD_START);
      if (traceBattleInput()) {
        pushDebugLog(`[battle] selected party slot ${action}`);
      }
    }
  } else if (wram.current_menu === BattleMenu.PACK) {
    action = _resolve_pack_menu_action(state);
  }
  wram.confirm_pressed = false;
  wram.cancel_pressed = false;
  wram.select_pressed = false;
  if (action !== null && action !== undefined) {
    if (traceBattleInput()) {
      pushDebugLog(`[battle] action resolved ${action}`);
    }
    lastAwaitingInputLog.delete(state);
  } else if (traceBattleInput()) {
    const description = _describe_wram(wram);
    if (lastAwaitingInputLog.get(state) !== description) {
      pushDebugLog(`[battle] awaiting input (${description})`);
      lastAwaitingInputLog.set(state, description);
    }
  }
  return action ?? null;
};

const sync_pack_menu_state = (state: BattleUIState): void => {
  const bagMenu = state.bag_menu;
  if (!bagMenu) {
    state.wram.last_item_names = [];
    state.wram.wPackMenuCursorPosition = 0;
    return;
  }
  const currentItems = typeof (bagMenu as { getCurrentItems?: unknown }).getCurrentItems === "function"
    ? (bagMenu as { getCurrentItems: () => Array<[string, number]> }).getCurrentItems()
    : [];
  const listIndex = typeof (bagMenu as { getListIndex?: unknown }).getListIndex === "function"
    ? (bagMenu as { getListIndex: () => number }).getListIndex()
    : 0;
  state.wram.last_item_names = currentItems.map(([name]) => name);
  state.wram.wPackMenuCursorPosition = listIndex;
};

const handle_move_swap = (state: BattleUIState, moves: LearnedMove[]): void => {
  const wram = state.wram;
  const totalMoves = moves.length;
  const cursor = wram.wMoveMenuCursorPosition;
  if (!totalMoves || cursor >= totalMoves) {
    wram.swapping_move_index = null;
    return;
  }
  const origin = wram.swapping_move_index;
  if (origin === null || origin === undefined) {
    wram.swapping_move_index = cursor;
    return;
  }
  if (origin >= totalMoves || origin === cursor) {
    wram.swapping_move_index = null;
    return;
  }
  const temp = moves[origin];
  moves[origin] = moves[cursor];
  moves[cursor] = temp;
  wram.swapping_move_index = null;
};

export const _resolve_pack_menu_action = (state: BattleUIState): string | null => {
  const pending = state.pending_pack_action;
  if (!pending) {
    return null;
  }
  const [action, itemName] = pending;
  state.pending_pack_action = null;
  if (action === 'cancel') {
    close_pack_menu(state);
    return null;
  }
  if (action === 'use' && itemName) {
    close_pack_menu(state);
    if (traceBattleInput()) {
      pushDebugLog(`[battle] selected item ${itemName}`);
    }
    return itemName;
  }
  return null;
};

const close_pack_menu = (state: BattleUIState): void => {
  state.bag_menu = null;
  state.wram.current_menu = BattleMenu.MAIN;
  state.wram.confirm_pressed = false;
  state.wram.cancel_pressed = false;
  state.pending_pack_action = null;
  state.bag_repeat_state.active_direction = null;
  state.bag_repeat_state.repeat_timer = 0;
  sync_pack_menu_state(state);
  ensure_menu_cursor(state.wram);
};

const handle_move_forget_input = (state: BattleUIState, event: GameEngineEvent): boolean => {
  const menu = state.move_forget_menu;
  const process = state.active_move_learn;
  if (!menu || !process) {
    return false;
  }
  if (process.stage !== MoveLearningPhase.FORGET_MENU) {
    return false;
  }
  if (menu.option_count <= 0) {
    return false;
  }
  const direction = map_key_to_direction(event.key ?? event.code ?? null);
  if (direction === 'up') {
    menu.selection = (menu.selection - 1 + menu.option_count) % menu.option_count;
    return true;
  }
  if (direction === 'down') {
    menu.selection = (menu.selection + 1) % menu.option_count;
    return true;
  }
  if (isConfirmEvent(event)) {
    process.pending_selection = menu.selection;
    process.stage = MoveLearningPhase.HANDLE_MENU_SELECTION;
    _close_move_forget_menu(state);
    return true;
  }
  if (isButtonEvent(event, GameButton.B)) {
    const cancelIndex = Math.max(menu.option_count - 1, 0);
    process.pending_selection = cancelIndex;
    process.stage = MoveLearningPhase.HANDLE_MENU_SELECTION;
    _close_move_forget_menu(state);
    return true;
  }
  return false;
};

const finalize_yes_no_selection = (state: BattleUIState): void => {
  const prompt = state.yes_no_prompt;
  prompt.active = false;
  prompt.pending_activation = false;
  battle_dialogue.advance_dialogue(state.dialogue);
  state.manual_wait_override = false;
};

const handle_yes_no_input = (state: BattleUIState, event: GameEngineEvent): boolean => {
  const promptState = state.yes_no_prompt;
  if (!promptState.active) {
    return false;
  }
  const prompt = promptState.prompt;
  if (!prompt) {
    return false;
  }
  const wasFinished = prompt.finished;
  const previousSelection = prompt.selection;
  prompt.handle_input(event as KeyEvent);
  const handled = prompt.finished !== wasFinished || prompt.selection !== previousSelection;
  if (prompt.finished) {
    promptState.result = prompt.result();
    finalize_yes_no_selection(state);
    return true;
  }
  return handled;
};

export const apply_auto_joypad_inputs = (state: BattleUIState, joypad: { hJoyPressed?: number } | null): void => {
  if (apply_yes_no_from_joypad(state, joypad)) {
    state.manual_wait_override = false;
  }
  if (battle_dialogue.apply_joypad_inputs(state.dialogue, joypad)) {
    _consume_manual_joypad_presses(state, JOY_A | JOY_B | B_PAD_START);
    _clear_manual_menu_button_latches(state);
    state.manual_wait_override = false;
  }
};

const BAG_DIRECTION_KEYS: Record<MenuDirection, string> = {
  [MenuDirection.UP]: gameEngine.K_UP,
  [MenuDirection.DOWN]: gameEngine.K_DOWN,
  [MenuDirection.LEFT]: gameEngine.K_LEFT,
  [MenuDirection.RIGHT]: gameEngine.K_RIGHT,
};

const BAG_DIRECTION_ORDER: Array<[number, MenuDirection]> = [
  [JOY_UP, MenuDirection.UP],
  [JOY_DOWN, MenuDirection.DOWN],
  [JOY_LEFT, MenuDirection.LEFT],
  [JOY_RIGHT, MenuDirection.RIGHT],
];

const POKEMON_DIRECTION_KEYS: Record<MenuDirection, string> = {
  [MenuDirection.UP]: gameEngine.K_UP,
  [MenuDirection.DOWN]: gameEngine.K_DOWN,
  [MenuDirection.LEFT]: gameEngine.K_LEFT,
  [MenuDirection.RIGHT]: gameEngine.K_RIGHT,
};

const POKEMON_DIRECTION_ORDER: Array<[number, MenuDirection]> = [
  [JOY_UP, MenuDirection.UP],
  [JOY_DOWN, MenuDirection.DOWN],
];

export const forward_pack_menu_inputs = (state: BattleUIState): void => {
  if (!state.bag_menu || state.pending_pack_action !== null && state.pending_pack_action !== undefined) {
    return;
  }
  const joypad = (state.game_state.hram as { joypad?: { hJoyPressed: number; hJoyDown: number } }).joypad ?? null;
  if (!joypad) {
    return;
  }
  forward_pack_directions(state, joypad);
  if (joypad.hJoyPressed & JOY_A) {
    dispatch_bag_event(state, bag_confirm_key());
    _consume_manual_joypad_press(state, JOY_A);
  }
  if (joypad.hJoyPressed & JOY_B) {
    dispatch_bag_event(state, bag_cancel_key());
    _consume_manual_joypad_press(state, JOY_B);
  }
};

const bag_confirm_key = (): number | string => buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN;

const bag_cancel_key = (): number | string => buttonKeys[GameButton.B][0] ?? gameEngine.K_BACKSPACE;

const forward_pack_directions = (state: BattleUIState, joypad: { hJoyPressed: number; hJoyDown: number }): void => {
  const repeat = state.bag_repeat_state;
  const direction = bag_direction_from_bits(joypad.hJoyPressed);
  if (direction) {
    dispatch_bag_direction(state, direction);
    const bit = bag_direction_bit(direction);
    if (bit) {
      _consume_manual_joypad_press(state, bit);
    }
    repeat.active_direction = direction;
    repeat.repeat_timer = INITIAL_REPEAT_DELAY_FRAMES;
    return;
  }
  const held = repeat.active_direction as MenuDirection | null;
  if (!held) {
    return;
  }
  const bit = bag_direction_bit(held);
  if (!bit || !(joypad.hJoyDown & bit)) {
    repeat.active_direction = null;
    repeat.repeat_timer = 0;
    return;
  }
  if (repeat.repeat_timer > 0) {
    repeat.repeat_timer -= 1;
    return;
  }
  dispatch_bag_direction(state, held);
  repeat.repeat_timer = REPEAT_INTERVAL_FRAMES;
};

const bag_direction_from_bits = (bits: number): MenuDirection | null => {
  for (const [mask, direction] of BAG_DIRECTION_ORDER) {
    if (bits & mask) {
      return direction;
    }
  }
  return null;
};

const bag_direction_bit = (direction: MenuDirection): number | null => {
  for (const [mask, candidate] of BAG_DIRECTION_ORDER) {
    if (candidate === direction) {
      return mask;
    }
  }
  return null;
};

const dispatch_bag_direction = (state: BattleUIState, direction: MenuDirection): void => {
  const key = BAG_DIRECTION_KEYS[direction];
  if (!key) {
    return;
  }
  dispatch_bag_event(state, key);
};

export const _dispatch_bag_event = (state: BattleUIState, key: string | number): void => {
  dispatch_bag_event(state, key);
};

const dispatch_bag_event = (state: BattleUIState, key: string | number): void => {
  if (!state.bag_menu) {
    return;
  }
  const event = new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key });
  const result = state.bag_menu.handleInput(event);
  sync_pack_menu_state(state);
  if (result !== null && result !== undefined) {
    state.pending_pack_action = result;
  }
};

export const forward_pokemon_menu_inputs = (state: BattleUIState): [string, number] | null => {
  const menu = state.pokemon_menu;
  if (!menu) {
    return null;
  }
  const joypad = (state.game_state.hram as { joypad?: { hJoyPressed: number; hJoyDown: number } }).joypad ?? null;
  if (!joypad) {
    return null;
  }
  let result: [string, number] | null = null;
  const direction = pokemon_direction_from_bits(joypad.hJoyPressed);
  if (direction) {
    result = dispatch_pokemon_direction(state, direction);
    const bit = pokemon_direction_bit(direction);
    if (bit) {
      _consume_manual_joypad_press(state, bit);
    }
    const repeat = state.pokemon_repeat_state;
    repeat.active_direction = direction;
    repeat.repeat_timer = INITIAL_REPEAT_DELAY_FRAMES;
    return result;
  }
  const repeat = state.pokemon_repeat_state;
  const held = repeat.active_direction as MenuDirection | null;
  if (held) {
    const bit = pokemon_direction_bit(held);
    if (!bit || !(joypad.hJoyDown & bit)) {
      release_pokemon_direction(state, held);
      repeat.active_direction = null;
      repeat.repeat_timer = 0;
    } else if (repeat.repeat_timer > 0) {
      repeat.repeat_timer -= 1;
    } else {
      result = dispatch_pokemon_direction(state, held);
      repeat.repeat_timer = REPEAT_INTERVAL_FRAMES;
      return result;
    }
  }
  if (joypad.hJoyPressed & JOY_A) {
    result = dispatch_pokemon_event(state, pokemon_confirm_key());
    _consume_manual_joypad_press(state, JOY_A);
  }
  if (joypad.hJoyPressed & JOY_B) {
    result = dispatch_pokemon_event(state, pokemon_cancel_key());
    _consume_manual_joypad_press(state, JOY_B);
  }
  return result;
};

export const forward_pokemon_stats_inputs = (state: BattleUIState): string | null => {
  const stats = state.pokemon_stats;
  if (!stats) {
    return null;
  }
  const joypad = (state.game_state.hram as { joypad?: { hJoyPressed?: number } }).joypad ?? null;
  if (!joypad) {
    return null;
  }
  const pressed = joypad.hJoyPressed ?? 0;
  if (pressed & JOY_UP) {
    const result = dispatch_stats_event(state, gameEngine.K_UP);
    _consume_manual_joypad_press(state, JOY_UP);
    return result;
  }
  if (pressed & JOY_DOWN) {
    const result = dispatch_stats_event(state, gameEngine.K_DOWN);
    _consume_manual_joypad_press(state, JOY_DOWN);
    return result;
  }
  if (pressed & JOY_LEFT) {
    const result = dispatch_stats_event(state, gameEngine.K_LEFT);
    _consume_manual_joypad_press(state, JOY_LEFT);
    return result;
  }
  if (pressed & JOY_RIGHT) {
    const result = dispatch_stats_event(state, gameEngine.K_RIGHT);
    _consume_manual_joypad_press(state, JOY_RIGHT);
    return result;
  }
  if (pressed & JOY_A) {
    const result = dispatch_stats_event(state, stats_confirm_key());
    _consume_manual_joypad_press(state, JOY_A);
    return result;
  }
  if (pressed & JOY_B) {
    const result = dispatch_stats_event(state, stats_cancel_key());
    _consume_manual_joypad_press(state, JOY_B);
    return result;
  }
  return null;
};

const pokemon_confirm_key = (): number | string => buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN;

const pokemon_cancel_key = (): number | string => buttonKeys[GameButton.B][0] ?? gameEngine.K_BACKSPACE;

const stats_confirm_key = (): number | string => buttonKeys[GameButton.A][0] ?? gameEngine.K_RETURN;

const stats_cancel_key = (): number | string => buttonKeys[GameButton.B][0] ?? gameEngine.K_BACKSPACE;

const dispatch_pokemon_direction = (state: BattleUIState, direction: MenuDirection): [string, number] | null => {
  const key = POKEMON_DIRECTION_KEYS[direction];
  if (!key) {
    return null;
  }
  return dispatch_pokemon_event(state, key);
};

const release_pokemon_direction = (state: BattleUIState, direction: MenuDirection): void => {
  const menu = state.pokemon_menu;
  const key = POKEMON_DIRECTION_KEYS[direction];
  if (!menu || !key) {
    return;
  }
  menu.handleInput(new gameEngine.event.Event(gameEngine.KEYUP, { key, code: key }));
};

const dispatch_pokemon_event = (state: BattleUIState, key: string | number): [string, number] | null => {
  const menu = state.pokemon_menu;
  if (!menu) {
    return null;
  }
  const event = new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key });
  return menu.handleInput(event) ?? null;
};

const dispatch_stats_event = (state: BattleUIState, key: string | number): string | null => {
  const stats = state.pokemon_stats;
  if (!stats) {
    return null;
  }
  const event = new gameEngine.event.Event(gameEngine.KEYDOWN, { key, code: key });
  return stats.handleInput(event) ?? null;
};

const pokemon_direction_from_bits = (bits: number): MenuDirection | null => {
  for (const [mask, direction] of POKEMON_DIRECTION_ORDER) {
    if (bits & mask) {
      return direction;
    }
  }
  return null;
};

const pokemon_direction_bit = (direction: MenuDirection): number | null => {
  for (const [mask, candidate] of POKEMON_DIRECTION_ORDER) {
    if (candidate === direction) {
      return mask;
    }
  }
  return null;
};

const YES_NO_DIRECTION_MASK = JOY_UP | JOY_DOWN | JOY_LEFT | JOY_RIGHT;

const apply_yes_no_from_joypad = (state: BattleUIState, joypad: { hJoyPressed?: number } | null): boolean => {
  if (!joypad || !state.yes_no_prompt.active) {
    return false;
  }
  const promptState = state.yes_no_prompt;
  const prompt = promptState.prompt;
  if (!prompt) {
    return false;
  }
  const pressed = joypad.hJoyPressed ?? 0;
  const wasFinished = prompt.finished;
  const previousSelection = prompt.selection;
  if (pressed & YES_NO_DIRECTION_MASK || pressed & JOY_A || pressed & JOY_B) {
    prompt.handle_joypad(pressed);
  }
  const handled = prompt.finished !== wasFinished || prompt.selection !== previousSelection;
  if (prompt.finished) {
    promptState.result = prompt.result();
    finalize_yes_no_selection(state);
    return true;
  }
  return handled;
};
