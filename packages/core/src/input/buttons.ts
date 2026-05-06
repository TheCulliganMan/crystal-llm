import { KEYS, normalizeKeycode } from "../core/keycodes";
import { buttonKeyCodes, mapKeycodeToButton } from "./bindings";
import { GameButton } from "./config";

export { GameButton } from "./config";

// Bit layout mirrors the GB hardware.
export const B_PAD_RIGHT = 1 << 0;
export const B_PAD_LEFT = 1 << 1;
export const B_PAD_UP = 1 << 2;
export const B_PAD_DOWN = 1 << 3;
export const B_PAD_A = 1 << 4;
export const B_PAD_B = 1 << 5;
export const B_PAD_SELECT = 1 << 6;
export const B_PAD_START = 1 << 7;

export type KeyEvent = {
  type: string | number;
  key?: number | string | null;
  code?: number | string | null;
  direction?: string | null;
  button?: string | null;
  is_press?: boolean | null;
};

const _DIRECTION_KEYS: Record<number, number> = {
  [KEYS.RIGHT]: B_PAD_RIGHT,
  [KEYS.LEFT]: B_PAD_LEFT,
  [KEYS.UP]: B_PAD_UP,
  [KEYS.DOWN]: B_PAD_DOWN,
};

type ButtonKeysFn = ((button: GameButton) => number[]) & Record<GameButton, number[]>;

export const buttonKeys = ((
  button: GameButton
): number[] => buttonKeyCodes(button)) as ButtonKeysFn;

for (const button of Object.values(GameButton)) {
  buttonKeys[button] = buttonKeyCodes(button);
}

export function normalizeButtonKey(value: number | string | null | undefined): number | null {
  if (typeof value === "string") {
    const normalized = value.toLowerCase();
    if (normalized === GameButton.A) {
      return buttonKeys(GameButton.A)[0] ?? null;
    }
    if (normalized === GameButton.B) {
      return buttonKeys(GameButton.B)[0] ?? null;
    }
    if (normalized === GameButton.Start) {
      return buttonKeys(GameButton.Start)[0] ?? null;
    }
    if (normalized === GameButton.Select) {
      return buttonKeys(GameButton.Select)[0] ?? null;
    }
  }
  return normalizeKeycode(value);
}

export function isButtonKey(value: number | string | null | undefined, button: GameButton): boolean {
  const code = normalizeKeycode(value);
  if (code === null) {
    return false;
  }
  return buttonKeys(button).includes(code);
}

export function isKeyDownEvent(event: KeyEvent): boolean {
  if (event.is_press !== undefined && event.is_press !== null) {
    return Boolean(event.is_press);
  }
  return event.type === "keydown" || event.type === KEYS.KEYDOWN;
}

export function isKeyUpEvent(event: KeyEvent): boolean {
  if (event.is_press !== undefined && event.is_press !== null) {
    return !event.is_press;
  }
  return event.type === "keyup" || event.type === KEYS.KEYUP;
}

export function isButtonEvent(event: KeyEvent, button: GameButton): boolean {
  if (!isKeyDownEvent(event)) {
    return false;
  }
  if (event.button === button) {
    return true;
  }
  const code = normalizeKeycode(event.code) ?? normalizeKeycode(event.key);
  if (code === null) {
    return false;
  }
  return buttonKeys(button).includes(code);
}

export function isConfirmEvent(event: KeyEvent): boolean {
  return isButtonEvent(event, GameButton.A);
}

export function isCancelEvent(event: KeyEvent): boolean {
  return isButtonEvent(event, GameButton.B);
}

export function isStartEvent(event: KeyEvent): boolean {
  return isButtonEvent(event, GameButton.Start);
}

export function isSelectEvent(event: KeyEvent): boolean {
  return isButtonEvent(event, GameButton.Select);
}

export function mapKeyToButton(key: number | null | undefined): GameButton | null {
    return mapKeycodeToButton(key);
}

function _mapKeyToPadBit(key: number): number {
    if (key in _DIRECTION_KEYS) {
        return _DIRECTION_KEYS[key];
    }
    const button = mapKeyToButton(key);
    if (button === GameButton.A) {
        return B_PAD_A;
    }
    if (button === GameButton.B) {
        return B_PAD_B;
    }
    if (button === GameButton.Start) {
        return B_PAD_START;
    }
    if (button === GameButton.Select) {
        return B_PAD_SELECT;
    }
    return 0;
}

export class JoypadState {
    private previousMask: number = 0;

    public computeMask(keys: Iterable<number>): number {
        let mask = 0;
        for (const key of keys) {
            mask |= _mapKeyToPadBit(key);
        }
        return mask;
    }

    public update(keys: Iterable<number>, filterMask: number): [number, number] {
        const currentMask = this.computeMask(keys) & 0xff;
        const filteredMask = currentMask & filterMask & 0xff;
        const hJoyPressed = (filteredMask ^ this.previousMask) & filteredMask;
        this.previousMask = filteredMask;
        return [hJoyPressed, filteredMask];
    }
}
