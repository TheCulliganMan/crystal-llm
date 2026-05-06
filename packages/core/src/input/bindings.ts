import { defaultKeyBindings, GameButton } from "./config";
import { keycodes, normalizeKeycode } from "../core/keycodes";

const buildButtonKeyCodes = (): Record<GameButton, number[]> => {
  const entries = Object.values(GameButton).map((button) => {
    const bindings = defaultKeyBindings[button] ?? [];
    const codes = bindings
      .map((code) => keycodes[code])
      .filter((value): value is number => typeof value === "number");
    return [button, codes] as const;
  });
  return Object.fromEntries(entries) as Record<GameButton, number[]>;
};

export const defaultButtonKeyCodes = buildButtonKeyCodes();

const buildButtonKeyCodeLookup = (): Record<number, GameButton> => {
  const lookup: Record<number, GameButton> = {};
  for (const [button, codes] of Object.entries(defaultButtonKeyCodes)) {
    for (const code of codes) {
      if (!(code in lookup)) {
        lookup[code] = button as GameButton;
      }
    }
  }
  return lookup;
};

const buttonKeyCodeLookup = buildButtonKeyCodeLookup();

export const buttonKeyCodes = (button: GameButton): number[] => {
  return defaultButtonKeyCodes[button] ?? [];
};

export const mapKeycodeToButton = (key: number | string | null | undefined): GameButton | null => {
  const normalized = normalizeKeycode(key);
  if (normalized === null) {
    return null;
  }
  return buttonKeyCodeLookup[normalized] ?? null;
};
