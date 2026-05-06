// Centralized keycode constants shared across backend input adapters.

export const KEYDOWN = 768;
export const KEYUP = 769;
export const QUIT = 256;

export const K_RIGHT = 1073741903;
export const K_LEFT = 1073741904;
export const K_UP = 1073741906;
export const K_DOWN = 1073741905;

export const K_z = 122;
export const K_SPACE = 32;
export const K_a = 97;
export const K_x = 120;
export const K_ESCAPE = 27;
export const K_s = 115;

export const K_RETURN = 13;
export const K_KP_ENTER = 1073741912;
export const K_BACKSPACE = 8;
export const K_LSHIFT = 1073742049;
export const K_RSHIFT = 1073742053;

export const KEYS = {
  KEYDOWN,
  KEYUP,
  QUIT,
  RIGHT: K_RIGHT,
  LEFT: K_LEFT,
  UP: K_UP,
  DOWN: K_DOWN,
  Z: K_z,
  SPACE: K_SPACE,
  A: K_a,
  X: K_x,
  ESCAPE: K_ESCAPE,
  S: K_s,
  RETURN: K_RETURN,
  KP_ENTER: K_KP_ENTER,
  BACKSPACE: K_BACKSPACE,
  LSHIFT: K_LSHIFT,
  RSHIFT: K_RSHIFT,
} as const;

export type KeyCodes = typeof KEYS;
