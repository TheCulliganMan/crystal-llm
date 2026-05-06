import { update_joypad_state } from './joypad';
import { JoypadState } from '../core/memory/hram';
import { KEYS } from '../core/keycodes';
import { JOY_A } from '../core/constants';

describe('update_joypad_state', () => {
  let joypad: JoypadState;

  beforeEach(() => {
    joypad = {
      hJoyPressed: 0,
      hJoyReleased: 0,
      hJoyDown: 0,
      hJoyLast: 0,
      hJoypadPressed: 0,
      hJoypadReleased: 0,
      hJoypadDown: 0,
      hJoypadSum: 0,
    };
  });

  it('should update hJoypadPressed correctly on key down', () => {
    const event = { type: KEYS.KEYDOWN, key: KEYS.Z }; // 'z' key for 'A' button

    update_joypad_state(joypad, event);

    expect(joypad.hJoyPressed).toBe(JOY_A);
    expect(joypad.hJoypadPressed).toBe(JOY_A);
  });
});
