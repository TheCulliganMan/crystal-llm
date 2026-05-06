// ASM: engine/battle/menu.asm (battle menu joypad + cursor repeat handling).
import { BattleInputState, BattleWRAM } from './battle-ui-state';
import {
  B_PAD_A,
  B_PAD_B,
  B_PAD_DOWN,
  B_PAD_LEFT,
  B_PAD_RIGHT,
  B_PAD_SELECT,
  B_PAD_START,
  B_PAD_UP,
} from '../../input/controls';
import {
  BATTLE_MENU_HEADER,
  BattleMenu,
  MenuDirection,
  interpretBattleMenu,
  loadBattleMenu,
} from './_battle-menu';

export type { BattleInputState, BattleWRAM };

export type BattleInputOptions = {
  menu_active: boolean;
};

export const apply_battle_inputs = (
  wram: BattleWRAM,
  joypad: { hJoyPressed?: number; hJoypadPressed?: number; hJoyDown?: number; hJoypadDown?: number },
  input_state: BattleInputState,
  options: BattleInputOptions
): void => {
  if (!options.menu_active) {
    input_state.active_direction = null;
    input_state.repeat_timer = 0;
    return;
  }
  _process_direction_inputs(wram, joypad, input_state);
  _process_button_inputs(wram, joypad);
};

export const ensure_menu_cursor = (wram: BattleWRAM): void => {
  const header = wram.menu_header ?? BATTLE_MENU_HEADER;
  wram.wBattleMenuCursorPosition = loadBattleMenu(wram.wBattleMenuCursorPosition, header);
};

const INITIAL_REPEAT_DELAY_FRAMES = 8;
const REPEAT_INTERVAL_FRAMES = 4;

const DIRECTION_PRIORITY: Array<[number, MenuDirection]> = [
  [B_PAD_UP, MenuDirection.UP],
  [B_PAD_DOWN, MenuDirection.DOWN],
  [B_PAD_LEFT, MenuDirection.LEFT],
  [B_PAD_RIGHT, MenuDirection.RIGHT],
];

const BIT_BY_DIRECTION: Record<MenuDirection, number> = {
  [MenuDirection.UP]: B_PAD_UP,
  [MenuDirection.DOWN]: B_PAD_DOWN,
  [MenuDirection.LEFT]: B_PAD_LEFT,
  [MenuDirection.RIGHT]: B_PAD_RIGHT,
};

const pressed_bits = (joypad: { hJoyPressed?: number; hJoypadPressed?: number }): number =>
  joypad.hJoyPressed ?? joypad.hJoypadPressed ?? 0;

const down_bits = (joypad: { hJoyDown?: number; hJoypadDown?: number }): number =>
  joypad.hJoyDown ?? joypad.hJoypadDown ?? 0;

const _process_button_inputs = (
  wram: BattleWRAM,
  joypad: { hJoyPressed?: number; hJoypadPressed?: number },
): void => {
  const pressed = pressed_bits(joypad);
  if (pressed & (B_PAD_A | B_PAD_START)) {
    wram.confirm_pressed = true;
  }
  if (pressed & B_PAD_B) {
    const header = wram.menu_header ?? BATTLE_MENU_HEADER;
    const bDisabled = wram.current_menu === BattleMenu.MAIN ? header.disable_b : false;
    if (!bDisabled) {
      wram.cancel_pressed = true;
    }
  }
  if (pressed & B_PAD_SELECT) {
    wram.select_pressed = true;
  }
};

const _process_direction_inputs = (
  wram: BattleWRAM,
  joypad: { hJoyPressed?: number; hJoypadPressed?: number; hJoyDown?: number; hJoypadDown?: number },
  repeat_state: BattleInputState,
): void => {
  const direction = _direction_from_pressed_bits(pressed_bits(joypad));
  if (direction) {
    _apply_direction(wram, direction);
    repeat_state.active_direction = direction;
    repeat_state.repeat_timer = INITIAL_REPEAT_DELAY_FRAMES;
    return;
  }
  const held = repeat_state.active_direction as MenuDirection | null;
  if (!held) {
    return;
  }
  const bit = BIT_BY_DIRECTION[held];
  if (!bit || !(down_bits(joypad) & bit)) {
    repeat_state.active_direction = null;
    repeat_state.repeat_timer = 0;
    return;
  }
  if ((repeat_state.repeat_timer ?? 0) > 0) {
    repeat_state.repeat_timer = (repeat_state.repeat_timer ?? 0) - 1;
    return;
  }
  _apply_direction(wram, held);
  repeat_state.repeat_timer = REPEAT_INTERVAL_FRAMES;
};

const _direction_from_pressed_bits = (bits: number): MenuDirection | null => {
  for (const [mask, direction] of DIRECTION_PRIORITY) {
    if (bits & mask) {
      return direction;
    }
  }
  return null;
};

const _apply_direction = (wram: BattleWRAM, direction: MenuDirection): void => {
  if (wram.current_menu === BattleMenu.MAIN) {
    _handle_main_menu_direction(wram, direction);
    return;
  }
  if (wram.current_menu === BattleMenu.FIGHT) {
    _step_move_menu_cursor(wram, direction);
    return;
  }
  if (wram.current_menu === BattleMenu.POKEMON) {
    _step_party_menu_cursor(wram, direction);
    return;
  }
  if (wram.current_menu === BattleMenu.PACK) {
    _step_pack_menu_cursor(wram, direction);
  }
};

const _handle_main_menu_direction = (wram: BattleWRAM, direction: MenuDirection): void => {
  ensure_menu_cursor(wram);
  const header = wram.menu_header ?? BATTLE_MENU_HEADER;
  wram.wBattleMenuCursorPosition = interpretBattleMenu(
    wram.wBattleMenuCursorPosition,
    header,
    direction,
  );
};

const _step_move_menu_cursor = (wram: BattleWRAM, direction: MenuDirection): void => {
  if (direction !== MenuDirection.UP && direction !== MenuDirection.DOWN) {
    return;
  }
  const entries = Math.max(1, (wram.last_num_moves ?? 0) + 1);
  if (direction === MenuDirection.UP) {
    wram.wMoveMenuCursorPosition = (wram.wMoveMenuCursorPosition - 1 + entries) % entries;
  } else {
    wram.wMoveMenuCursorPosition = (wram.wMoveMenuCursorPosition + 1) % entries;
  }
};

const _step_party_menu_cursor = (wram: BattleWRAM, direction: MenuDirection): void => {
  if (direction !== MenuDirection.UP && direction !== MenuDirection.DOWN) {
    return;
  }
  const total = Math.max(1, wram.last_party_size ?? 0);
  if (direction === MenuDirection.UP) {
    wram.wPartyMenuCursorPosition = (wram.wPartyMenuCursorPosition - 1 + total) % total;
  } else {
    wram.wPartyMenuCursorPosition = (wram.wPartyMenuCursorPosition + 1) % total;
  }
};

const _step_pack_menu_cursor = (wram: BattleWRAM, direction: MenuDirection): void => {
  if (direction !== MenuDirection.UP && direction !== MenuDirection.DOWN) {
    return;
  }
  const total = Math.max(1, (wram.last_item_names ?? []).length);
  if (direction === MenuDirection.UP) {
    wram.wPackMenuCursorPosition = (wram.wPackMenuCursorPosition - 1 + total) % total;
  } else {
    wram.wPackMenuCursorPosition = (wram.wPackMenuCursorPosition + 1) % total;
  }
};
