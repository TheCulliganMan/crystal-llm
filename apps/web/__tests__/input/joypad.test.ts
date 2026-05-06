import {
  joypad_bit_for_key,
  joypad_bits_for_event,
  update_joypad_state,
  joypad_pressed,
  reset_joypad_frame,
  joypad_direction_for_event,
  JOY_A,
  JOY_B,
  JOY_UP,
} from '@pokecrystal/core/input/joypad';
import { KEYS } from '@pokecrystal/core/core/keycodes';
import { JoypadState, createJoypadState } from '@pokecrystal/core/core/memory/hram';

describe('joypad', () => {
  let joypad: JoypadState;

  beforeEach(() => {
    joypad = createJoypadState();
  });

  describe('joypad_bit_for_key', () => {
    it('should map multiple key codes to the same joypad bit', () => {
      expect(joypad_bit_for_key(KEYS.Z)).toBe(JOY_A);
      expect(joypad_bit_for_key(KEYS.SPACE)).toBe(JOY_A);
      expect(joypad_bit_for_key(KEYS.KEY_A)).toBe(JOY_A);
    });

    it('should return undefined for an unmapped key', () => {
      expect(joypad_bit_for_key(12345)).toBeUndefined();
    });
  });

  describe('joypad_direction_for_event', () => {
    it('should return "up" for the up arrow key', () => {
      const event = { type: KEYS.KEYDOWN, key: KEYS.UP };
      expect(joypad_direction_for_event(event)).toBe('up');
    });
  });

  describe('joypad_bits_for_event', () => {
    it('should return the correct bit for a KEYDOWN event', () => {
      const event = { type: KEYS.KEYDOWN, key: KEYS.Z };
      expect(joypad_bits_for_event(event)).toBe(JOY_A);
    });

    it('should return the correct bit for a KEYUP event', () => {
      const event = { type: KEYS.KEYUP, key: KEYS.X };
      expect(joypad_bits_for_event(event)).toBe(JOY_B);
    });

    it('should return undefined for a non-joypad event type', () => {
      const event = { type: 999, key: KEYS.Z };
      expect(joypad_bits_for_event(event)).toBeUndefined();
    });
  });

  describe('update_joypad_state', () => {
    it('should update hJoyDown and hJoyPressed on KEYDOWN', () => {
      const event = { type: KEYS.KEYDOWN, key: KEYS.UP };
      update_joypad_state(joypad, event);
      expect(joypad.hJoyDown).toBe(JOY_UP);
      expect(joypad.hJoyPressed).toBe(JOY_UP);
    });

    it('should clear hJoyDown and set hJoyReleased on KEYUP', () => {
      // First, press the key
      const downEvent = { type: KEYS.KEYDOWN, key: KEYS.UP };
      update_joypad_state(joypad, downEvent);

      // Now, release it
      const upEvent = { type: KEYS.KEYUP, key: KEYS.UP };
      update_joypad_state(joypad, upEvent);

      expect(joypad.hJoyDown).toBe(0);
      expect(joypad.hJoyReleased).toBe(JOY_UP);
    });
  });

  describe('joypad_pressed', () => {
    it('should return true if the button was pressed this frame', () => {
      const event = { type: KEYS.KEYDOWN, key: KEYS.Z };
      update_joypad_state(joypad, event);
      expect(joypad_pressed(joypad, JOY_A)).toBe(true);
    });

    it('should return false if the button was not pressed this frame', () => {
      expect(joypad_pressed(joypad, JOY_A)).toBe(false);
    });
  });

  describe('reset_joypad_frame', () => {
    it('should reset the per-frame joypad state', () => {
      const event = { type: KEYS.KEYDOWN, key: KEYS.Z };
      update_joypad_state(joypad, event);

      const upEvent = { type: KEYS.KEYUP, key: KEYS.Z };
      update_joypad_state(joypad, upEvent);

      reset_joypad_frame(joypad);

      expect(joypad.hJoyPressed).toBe(0);
      expect(joypad.hJoyReleased).toBe(0);
      expect(joypad.hJoypadPressed).toBe(0);
      expect(joypad.hJoypadReleased).toBe(0);
    });
  });
});
