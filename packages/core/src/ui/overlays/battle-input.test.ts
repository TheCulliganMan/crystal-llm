import { apply_battle_inputs } from './battle-input';
import { BattleMenu, BATTLE_MENU_HEADER } from './_battle-menu';
import type { BattleInputState, BattleWRAM } from './battle-input';
import { B_PAD_B, B_PAD_DOWN, B_PAD_LEFT, B_PAD_RIGHT, B_PAD_UP } from '../../input/controls';

const createWram = (overrides: Partial<BattleWRAM> = {}): BattleWRAM => ({
  current_menu: BattleMenu.MAIN,
  menu_header: BATTLE_MENU_HEADER,
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
  wBattleMenuCursorPositionNext: 0,
  swapping_move_index: null,
  last_item_names: [],
  ...overrides,
});

const createInputState = (): BattleInputState => ({
  active_direction: null,
  repeat_timer: 0,
});

describe('battle-input', () => {
  it('moves the main battle menu cursor in all four directions', () => {
    const cases: Array<[string, number, number, number]> = [
      ['right', B_PAD_RIGHT, 0, 1],
      ['left', B_PAD_LEFT, 0, 1],
      ['down', B_PAD_DOWN, 0, 2],
      ['up', B_PAD_UP, 0, 2],
    ];

    for (const [_label, bit, start, expected] of cases) {
      const wram = createWram({ current_menu: BattleMenu.MAIN, wBattleMenuCursorPosition: start });
      const inputState = createInputState();

      apply_battle_inputs(
        wram,
        { hJoyPressed: bit, hJoypadPressed: bit, hJoyDown: bit, hJoypadDown: bit },
        inputState,
        { menu_active: true },
      );

      expect(wram.wBattleMenuCursorPosition).toBe(expected);
    }
  });

  it('ignores B on the main battle menu when disable_b is set', () => {
    const wram = createWram({ current_menu: BattleMenu.MAIN });
    const inputState = createInputState();

    apply_battle_inputs(wram, { hJoyPressed: B_PAD_B }, inputState, { menu_active: true });

    expect(wram.cancel_pressed).toBe(false);
  });

  it('accepts B on the fight menu even when the main header disables B', () => {
    const wram = createWram({ current_menu: BattleMenu.FIGHT });
    const inputState = createInputState();

    apply_battle_inputs(wram, { hJoyPressed: B_PAD_B }, inputState, { menu_active: true });

    expect(wram.cancel_pressed).toBe(true);
  });
});
