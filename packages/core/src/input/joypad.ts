import { JoypadState } from '../core/memory/hram';
import { KEYS, keycodes, normalizeKeycode } from '../core/keycodes';
import {
  JOY_A,
  JOY_B,
  JOY_SELECT,
  JOY_START,
  JOY_UP,
  JOY_DOWN,
  JOY_LEFT,
  JOY_RIGHT,
} from '../core/constants';
import { defaultKeyBindings, GameButton } from './config';

export {
    JOY_A,
    JOY_B,
    JOY_SELECT,
    JOY_START,
    JOY_UP,
    JOY_DOWN,
    JOY_LEFT,
    JOY_RIGHT,
};

const _KEY_TO_JOY_BIT: { [key: number]: number } = {
  [KEYS.RIGHT]: JOY_RIGHT,
  [KEYS.LEFT]: JOY_LEFT,
  [KEYS.UP]: JOY_UP,
  [KEYS.DOWN]: JOY_DOWN,
};

const buttonToJoyBit: { [key in GameButton]: number } = {
  a: JOY_A,
  b: JOY_B,
  start: JOY_START,
  select: JOY_SELECT,
};

for (const [button, keys] of Object.entries(defaultKeyBindings)) {
  const bit = buttonToJoyBit[button as GameButton];
  for (const key of keys) {
    const keyCode = keycodes[key];
    if (keyCode !== undefined) {
      _KEY_TO_JOY_BIT[keyCode] = bit;
    }
  }
}

const _DIRECTION_BY_BIT: { [key: number]: string } = {
    [JOY_UP]: "up",
    [JOY_DOWN]: "down",
    [JOY_LEFT]: "left",
    [JOY_RIGHT]: "right",
};
const _DIRECTION_TO_BIT: { [direction: string]: number } = {
  up: JOY_UP,
  down: JOY_DOWN,
  left: JOY_LEFT,
  right: JOY_RIGHT,
};

const _BUTTON_TO_KEY: { [button: string]: number } = {};
for (const [button, keys] of Object.entries(defaultKeyBindings)) {
    if (keys.length > 0) {
        _BUTTON_TO_KEY[button] = keycodes[keys[0]];
    }
}

export function joypad_bit_for_key(key: number): number | undefined {
  return _KEY_TO_JOY_BIT[key];
}

function _normalizeKeyCode(value: number | string | null | undefined): number | undefined {
    const normalized = normalizeKeycode(value);
    return normalized === null ? undefined : normalized;
}

function _isKeyEventType(type: number | string): boolean {
    return type === KEYS.KEYDOWN || type === KEYS.KEYUP || type === "keydown" || type === "keyup";
}

export function joypad_bits_for_event(event: { type: number | string; key?: number | string | null; code?: number | string | null; direction?: string | null; button?: string | null }): number | undefined {
    if (!_isKeyEventType(event.type)) {
        return undefined;
    }

    if (event.button) {
        const button = String(event.button).toLowerCase();
        const key = _BUTTON_TO_KEY[button];
        if (key !== undefined) {
          return joypad_bit_for_key(key);
        }
    }

    if (event.direction) {
        return _DIRECTION_TO_BIT[event.direction];
    }

    const code = _normalizeKeyCode(event.code) ?? _normalizeKeyCode(event.key);
    if (code === undefined) {
      return undefined;
    }
    return joypad_bit_for_key(code);
}

export function joypad_direction_for_event(event: { type: number | string; key?: number | string | null; code?: number | string | null; direction?: string | null; button?: string | null }): string | undefined {
    if (event.direction) {
        return event.direction;
    }
    const bit = joypad_bits_for_event(event);
    if (bit === undefined) {
        return undefined;
    }
    return _DIRECTION_BY_BIT[bit];
}


export function reset_joypad_frame(joypad: JoypadState): void {
  joypad.hJoyPressed = 0;
  joypad.hJoyReleased = 0;
  joypad.hJoypadPressed = 0;
  joypad.hJoypadReleased = 0;
}

export function update_joypad_state(joypad: JoypadState, event: { type: number | string; key?: number | string | null; code?: number | string | null; direction?: string | null; button?: string | null }): void {
    const bit = joypad_bits_for_event(event);
    if (bit === undefined) {
        return;
    }
    if (event.type === KEYS.KEYDOWN || event.type === "keydown") {
        if (!(joypad.hJoyDown & bit)) {
            joypad.hJoyPressed |= bit;
            joypad.hJoypadPressed |= bit;
        }
        joypad.hJoyDown |= bit;
        joypad.hJoypadDown |= bit;
    } else {
        joypad.hJoyDown &= ~bit;
        joypad.hJoyReleased |= bit;
        joypad.hJoypadDown &= ~bit;
        joypad.hJoypadReleased |= bit;
    }
    joypad.hJoyLast = joypad.hJoyDown;
    joypad.hJoypadSum = joypad.hJoypadDown;
}

export function joypad_pressed(joypad: JoypadState, ...buttons: number[]): boolean {
    let mask = 0;
    for (const bit of buttons) {
        mask |= bit & 0xFF;
    }
    return !!(joypad.hJoyPressed & mask);
}

export function joypad_direction_from_bits(bits: number): string | null {
    if (bits & JOY_UP) {
        return "up";
    }
    if (bits & JOY_DOWN) {
        return "down";
    }
    if (bits & JOY_LEFT) {
        return "left";
    }
    if (bits & JOY_RIGHT) {
        return "right";
    }
    return null;
}

export const resetJoypadFrame = reset_joypad_frame;
export const updateJoypadState = update_joypad_state;
