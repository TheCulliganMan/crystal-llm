// ASM mapping: pokecrystal_disassembly/home/joypad.asm (GetJoypad input latching).
import { JoypadState } from "./memory/hram";
import {
  JOY_A,
  JOY_B,
  JOY_DOWN,
  JOY_LEFT,
  JOY_RIGHT,
  JOY_SELECT,
  JOY_START,
  JOY_UP,
} from "./constants";
import { mapKeyToButton, mapKeyToDirection, type GameButton } from "@pokecrystal/core/input/controls";
import { joypad_bit_for_key } from "@pokecrystal/core/input/joypad";

const BUTTON_BITS: Record<GameButton, number> = {
  a: JOY_A,
  b: JOY_B,
  start: JOY_START,
  select: JOY_SELECT,
};

const DIRECTION_BITS: Record<string, number> = {
  up: JOY_UP,
  down: JOY_DOWN,
  left: JOY_LEFT,
  right: JOY_RIGHT,
};

const normalizeKey = (key: string): string => {
  if (key === " ") {
    return "Space";
  }
  if (key === "Shift") {
    return "ShiftLeft";
  }
  if (key.length === 1) {
    const upper = key.toUpperCase();
    if (upper >= "A" && upper <= "Z") {
      return `Key${upper}`;
    }
  }
  return key;
};

const joypadBitForKey = (rawKey: string): number => {
  const key = normalizeKey(rawKey);
  const direction = mapKeyToDirection(key);
  if (direction) {
    return DIRECTION_BITS[direction] ?? 0;
  }
  const button = mapKeyToButton(key);
  if (button) {
    return BUTTON_BITS[button] ?? 0;
  }
  const numericKey = Number(key);
  if (Number.isFinite(numericKey) && String(numericKey) === key) {
    return joypad_bit_for_key(numericKey) ?? 0;
  }
  return 0;
};

export function updateJoypadStateFromKeys(joypad: JoypadState, heldKeys: Iterable<string>): void {
  let current = 0;
  for (const key of heldKeys) {
    current |= joypadBitForKey(key);
  }
  current &= 0xff;
  const last = joypad.hJoyDown & 0xff;
  const delta = last ^ current;
  const pressed = delta & current;
  const released = delta & last;

  joypad.hJoyPressed = pressed;
  joypad.hJoyReleased = released;
  joypad.hJoyDown = current;
  joypad.hJoypadPressed = pressed;
  joypad.hJoypadReleased = released;
  joypad.hJoypadDown = current;
  joypad.hJoyLast = current;
  joypad.hJoypadSum = current;
}
