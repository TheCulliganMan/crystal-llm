
import { defaultKeyBindings, GameButton } from "./config";
import { KEYS, keycodes, normalizeKeycode } from "../core/keycodes";

export { GameButton };
export type { KeyEvent } from "./buttons";

const GAME_BUTTON_BY_BINDING: Record<string, GameButton> = Object.create(null);
const GAME_BUTTON_BY_KEYCODE: Record<number, GameButton> = Object.create(null);

for (const [button, keys] of Object.entries(defaultKeyBindings)) {
  const resolvedButton = button as GameButton;
  for (const key of keys) {
    GAME_BUTTON_BY_BINDING[key] = resolvedButton;
    const keyCode = keycodes[key];
    if (keyCode !== undefined) {
      GAME_BUTTON_BY_KEYCODE[keyCode] = resolvedButton;
    }
  }
}

export const B_PAD_RIGHT = 1 << 0;
export const B_PAD_LEFT = 1 << 1;
export const B_PAD_UP = 1 << 2;
export const B_PAD_DOWN = 1 << 3;
export const B_PAD_A = 1 << 4;
export const B_PAD_B = 1 << 5;
export const B_PAD_SELECT = 1 << 6;
export const B_PAD_START = 1 << 7;

const GAME_BUTTON_LOOKUP: Record<string, GameButton> = Object.create(null);
for (const button of Object.values(GameButton)) {
  GAME_BUTTON_LOOKUP[button] = button;
  GAME_BUTTON_LOOKUP[button.toLowerCase()] = button;
}

const resolveGameButton = (value: GameButton | string | null | undefined): GameButton | null => {
  if (value === null || value === undefined) {
    return null;
  }
  if (typeof value !== "string") {
    return value;
  }
  const normalized = value.trim();
  if (!normalized) {
    return null;
  }
  return GAME_BUTTON_LOOKUP[normalized] ?? GAME_BUTTON_LOOKUP[normalized.toLowerCase()] ?? null;
};

export const buttonKeys = (button: GameButton | string): readonly string[] => {
  const resolved = resolveGameButton(button);
  if (resolved === null) {
    return [];
  }
  return defaultKeyBindings[resolved] ?? [];
};

const normalizeKeyCode = (value: string | number | null | undefined): number | null => {
  return normalizeKeycode(value);
};

export type InputEventLike = {
  type: string | number;
  key?: string | number | null;
  code?: string | number | null;
  button?: string | null;
  direction?: string | null;
  is_press?: boolean | null;
};

export const isKeyDownEvent = (
  event: InputEventLike
): boolean => {
  if (event.is_press !== undefined && event.is_press !== null) {
    return Boolean(event.is_press);
  }
  return event.type === "keydown" || event.type === KEYS.KEYDOWN;
};

export const isKeyUpEvent = (
  event: InputEventLike
): boolean => {
  if (event.is_press !== undefined && event.is_press !== null) {
    return !event.is_press;
  }
  return event.type === "keyup" || event.type === KEYS.KEYUP;
};

export const isButtonEvent = (
  event: InputEventLike,
  button: GameButton | string
): boolean => {
  if (!isKeyDownEvent(event)) {
    return false;
  }
  const normalizedButton = resolveGameButton(button);
  if (normalizedButton === null) {
    return false;
  }
  if (resolveGameButton(event.button) === normalizedButton) {
    return true;
  }
  const rawKey = event.code ?? event.key ?? null;
  const normalized = normalizeKeyCode(rawKey);
  if (normalized !== null) {
    return GAME_BUTTON_BY_KEYCODE[normalized] === normalizedButton;
  }
  if (typeof rawKey === "string") {
    return GAME_BUTTON_BY_BINDING[rawKey] === normalizedButton;
  }
  return buttonKeys(normalizedButton).includes(String(event.code ?? event.key ?? ""));
};

export const isConfirmEvent = (event: InputEventLike): boolean => {
  return isButtonEvent(event, GameButton.A);
};

export const isCancelEvent = (event: InputEventLike): boolean => {
  return isButtonEvent(event, GameButton.B);
};

export const isStartEvent = (event: InputEventLike): boolean => {
  return isButtonEvent(event, GameButton.Start);
};

export const isSelectEvent = (event: InputEventLike): boolean => {
  return isButtonEvent(event, GameButton.Select);
};

export const mapKeyToButton = (key: string | number | null): GameButton | null => {
  if (key === null || key === undefined) {
    return null;
  }
  if (typeof key === "string") {
    const button = resolveGameButton(key);
    if (button) {
      return button;
    }
  }
  const normalized = normalizeKeyCode(key);
  if (normalized !== null) {
    return GAME_BUTTON_BY_KEYCODE[normalized] ?? null;
  }
  if (typeof key === "string") {
    return GAME_BUTTON_BY_BINDING[key] ?? null;
  }
  return null;
};

export const mapKeyToDirection = (key: string | number | null): string | null => {
  if (key === null || key === undefined) {
    return null;
  }
  if (typeof key === "string") {
    const normalized = key.trim().toLowerCase();
    if (normalized === "up" || normalized === "down" || normalized === "left" || normalized === "right") {
      return normalized;
    }
  }
  const normalized = normalizeKeyCode(key);
  if (normalized !== null) {
    if (normalized === KEYS.UP) {
      return "up";
    }
    if (normalized === KEYS.DOWN) {
      return "down";
    }
    if (normalized === KEYS.LEFT) {
      return "left";
    }
    if (normalized === KEYS.RIGHT) {
      return "right";
    }
  }
  return (
    {
      ArrowUp: "up",
      ArrowDown: "down",
      ArrowLeft: "left",
      ArrowRight: "right",
    }[key] || null
  );
};
