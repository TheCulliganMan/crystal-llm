import { GameButton, isButtonEvent, type KeyEvent } from '@pokecrystal/core/input/buttons';
import { pushDebugLog } from '@pokecrystal/core/core/debug-log';
import { B_PAD_A, B_PAD_B, B_PAD_START } from '@pokecrystal/core/input/controls';
import { DialogueWindow } from '../text/dialogue';
import type { BattleTextWindow } from './_battle-layout';

const ADVANCE_BUTTONS = [GameButton.A, GameButton.B, GameButton.Start];

export interface QueuedDialogue {
  text: string;
  control?: string | null;
}

export interface DialogueState {
  window: BattleTextWindow;
  dialogue: DialogueWindow;
  queue: QueuedDialogue[];
  pending_waits: number;
  forced_visible: boolean;
  auto_close_after_display: boolean;
}

export const reset_dialogue = (state: DialogueState): void => {
  state.queue = [];
  state.pending_waits = 0;
  state.forced_visible = false;
  state.dialogue.clear();
  state.auto_close_after_display = false;
};

export const enqueue_text = (
  state: DialogueState,
  text: string,
  options?: { control?: string | null },
): boolean => {
  const normalized = String(text ?? '').replace(/\r/g, '').trim();
  if (!normalized) {
    pushDebugLog('[battle] dialogue ignored empty text');
    return false;
  }
  state.queue.push({ text: normalized, control: options?.control ?? null });
  pushDebugLog(`[battle] dialogue queued (${normalized.length} chars)`);
  if (!state.forced_visible) {
    start_next_dialogue(state);
  }
  return true;
};

export const push_wait = (state: DialogueState): void => {
  state.pending_waits += 1;
  force_text_box(state, true);
};

export const force_text_box = (state: DialogueState, value: boolean): void => {
  state.forced_visible = value;
  if (!value && state.pending_waits <= 0 && !state.queue.length) {
    state.dialogue.clear();
  }
};

export const close_text_box = (state: DialogueState): void => {
  state.queue = [];
  state.pending_waits = 0;
  state.forced_visible = false;
  state.dialogue.clear();
};

export const start_next_dialogue = (state: DialogueState): void => {
  if (!state.queue.length) {
    state.forced_visible = false;
    state.dialogue.clear();
    return;
  }
  const entry = state.queue.shift() as QueuedDialogue;
  state.auto_close_after_display = entry.control === 'done';
  pushDebugLog(`[battle] dialogue start (${entry.text.length} chars)`);
  state.dialogue.open(entry.text);
  state.forced_visible = true;
};

export const consume_input = (state: DialogueState, event: KeyEvent): boolean => {
  if (event.type !== 'keydown') {
    return false;
  }
  if (!ADVANCE_BUTTONS.some((button) => isButtonEvent(event, button))) {
    return false;
  }
  return advance_dialogue(state);
};

export const advance_dialogue = (state: DialogueState): boolean => {
  const dlg = state.dialogue;
  if (state.forced_visible) {
    if (!dlg.is_complete()) {
      dlg.complete();
      return true;
    }
    if (dlg.has_more_pages()) {
      dlg.advance_page();
      return true;
    }
    consume_wait_token(state);
    start_next_dialogue(state);
    if (state.forced_visible) {
      return true;
    }
  }
  if (consume_wait_token(state)) {
    return true;
  }
  return false;
};

export const is_visible = (state: DialogueState): boolean => state.forced_visible || state.pending_waits > 0;

export const requires_ack = (state: DialogueState): boolean => {
  const dlg = state.dialogue;
  if (state.pending_waits > 0) {
    return true;
  }
  if (state.forced_visible && !dlg.is_complete()) {
    return true;
  }
  return state.queue.length > 0;
};

export const waiting_flag = (state: DialogueState): boolean => {
  const dlg = state.dialogue;
  const pending = state.pending_waits > 0;
  const revealing = state.forced_visible && !dlg.is_complete();
  const more = state.queue.length > 0 || dlg.has_more_pages();
  return Boolean(pending || revealing || more);
};

export const apply_joypad_inputs = (
  state: DialogueState,
  joypad: { hJoyPressed?: number } | null,
): boolean => {
  if (!joypad) {
    return false;
  }
  if ((joypad.hJoyPressed ?? 0) & (B_PAD_A | B_PAD_B | B_PAD_START)) {
    return advance_dialogue(state);
  }
  return false;
};

export const auto_close_if_idle = (state: DialogueState, prompt_active: boolean): boolean => {
  if (!state.auto_close_after_display) {
    return false;
  }
  if (prompt_active) {
    return false;
  }
  if (state.pending_waits > 0) {
    return false;
  }
  if (state.queue.length > 0) {
    return false;
  }
  const dlg = state.dialogue;
  if (!dlg.is_complete() || dlg.has_more_pages()) {
    return false;
  }
  force_text_box(state, false);
  state.auto_close_after_display = false;
  return true;
};

const consume_wait_token = (state: DialogueState): boolean => {
  if (state.pending_waits <= 0) {
    return false;
  }
  state.pending_waits = Math.max(0, state.pending_waits - 1);
  if (state.pending_waits === 0 && !state.queue.length) {
    state.forced_visible = false;
    state.dialogue.clear();
  }
  return true;
};
