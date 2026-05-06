// Event type codes
const KEYDOWN = 768;
const KEYUP = 769;
const QUIT = 256;

// Key codes (SDL/pygame compatible)
const K_RIGHT = 1073741903;
const K_LEFT = 1073741904;
const K_UP = 1073741906;
const K_DOWN = 1073741905;

const K_z = 122;
const K_SPACE = 32;
const K_a = 97;
const K_x = 120;
const K_b = 98;
const K_ESCAPE = 27;
const K_s = 115;
const K_j = 106;
const K_k = 107;

const K_RETURN = 13;
const K_KP_ENTER = 1073741912;
const K_BACKSPACE = 8;
const K_LSHIFT = 1073742049;
const K_RSHIFT = 1073742053;

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
    KEY_A: K_a,
    X: K_x,
    B: K_b,
    ESCAPE: K_ESCAPE,
    S: K_s,
    J: K_j,
    K: K_k,
    RETURN: K_RETURN,
    KP_ENTER: K_KP_ENTER,
    BACKSPACE: K_BACKSPACE,
    LSHIFT: K_LSHIFT,
    RSHIFT: K_RSHIFT,
};

export const keycodes: { [key: string]: number } = {
    ArrowUp: KEYS.UP,
    ArrowDown: KEYS.DOWN,
    ArrowLeft: KEYS.LEFT,
    ArrowRight: KEYS.RIGHT,
    z: KEYS.Z,
    Z: KEYS.Z,
    KeyZ: KEYS.Z,
    " ": KEYS.SPACE,
    Spacebar: KEYS.SPACE,
    Space: KEYS.SPACE,
    a: KEYS.A,
    A: KEYS.A,
    KeyA: KEYS.A,
    x: KEYS.X,
    X: KEYS.X,
    KeyX: KEYS.X,
    KeyB: KEYS.B,
    b: KEYS.B,
    B: KEYS.B,
    s: KEYS.S,
    S: KEYS.S,
    Escape: KEYS.ESCAPE,
    KeyS: KEYS.S,
    j: KEYS.J,
    J: KEYS.J,
    KeyJ: KEYS.J,
    k: KEYS.K,
    K: KEYS.K,
    KeyK: KEYS.K,
    Enter: KEYS.RETURN,
    NumpadEnter: KEYS.KP_ENTER,
    Backspace: KEYS.BACKSPACE,
    Shift: KEYS.LSHIFT,
    ShiftLeft: KEYS.LSHIFT,
    ShiftRight: KEYS.RSHIFT,
};

const DOM_KEYCODE_MAP: { [key: number]: number } = {
    37: KEYS.LEFT,
    38: KEYS.UP,
    39: KEYS.RIGHT,
    40: KEYS.DOWN,
    90: KEYS.Z,
    88: KEYS.X,
    65: KEYS.A,
    66: KEYS.B,
    83: KEYS.S,
    74: KEYS.J,
    75: KEYS.K,
    13: KEYS.RETURN,
    8: KEYS.BACKSPACE,
    27: KEYS.ESCAPE,
    16: KEYS.LSHIFT,
    32: KEYS.SPACE,
};

export function normalizeKeycode(value: number | string | null | undefined): number | null {
    if (value === null || value === undefined) {
        return null;
    }
    if (typeof value === "number") {
        return DOM_KEYCODE_MAP[value] ?? value;
    }
    const mapped = keycodes[value];
    return typeof mapped === "number" ? mapped : null;
}
