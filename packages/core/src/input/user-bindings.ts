import { z } from "zod";

import { defaultKeyBindings, GameButton } from "./config";

export type KeyBindings = Record<GameButton, string[]>;

const STORAGE_KEY = "pokecrystal.keyBindings.v1";
const CHANGE_EVENT = "pokecrystal.keyBindings.changed";

const KeyBindingsSchema = z
  .object({
    [GameButton.A]: z.array(z.string()),
    [GameButton.B]: z.array(z.string()),
    [GameButton.Start]: z.array(z.string()),
    [GameButton.Select]: z.array(z.string()),
  })
  .strict();

let cachedBindings: KeyBindings | null = null;

const dedupe = (values: string[]): string[] => {
  const seen = new Set<string>();
  const next: string[] = [];
  for (const raw of values) {
    const value = raw.trim();
    if (!value) continue;
    if (seen.has(value)) continue;
    seen.add(value);
    next.push(value);
  }
  return next;
};

const normaliseBindings = (value: KeyBindings): KeyBindings => {
  return {
    [GameButton.A]: dedupe(value[GameButton.A] ?? []),
    [GameButton.B]: dedupe(value[GameButton.B] ?? []),
    [GameButton.Start]: dedupe(value[GameButton.Start] ?? []),
    [GameButton.Select]: dedupe(value[GameButton.Select] ?? []),
  };
};

const dispatchChanged = (): void => {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new Event(CHANGE_EVENT));
};

export const getKeyBindingsChangeEventName = (): string => CHANGE_EVENT;

export const getActiveKeyBindings = (): KeyBindings => {
  if (cachedBindings) {
    return cachedBindings;
  }
  if (typeof window === "undefined") {
    cachedBindings = normaliseBindings(defaultKeyBindings);
    return cachedBindings;
  }
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) {
      cachedBindings = normaliseBindings(defaultKeyBindings);
      return cachedBindings;
    }
    const parsed = JSON.parse(raw);
    const result = KeyBindingsSchema.safeParse(parsed);
    if (!result.success) {
      cachedBindings = normaliseBindings(defaultKeyBindings);
      return cachedBindings;
    }
    cachedBindings = normaliseBindings(result.data as KeyBindings);
    return cachedBindings;
  } catch {
    cachedBindings = normaliseBindings(defaultKeyBindings);
    return cachedBindings;
  }
};

export const setActiveKeyBindings = (bindings: KeyBindings): boolean => {
  const normalized = normaliseBindings(bindings);
  cachedBindings = normalized;
  if (typeof window === "undefined") {
    return false;
  }
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(normalized));
    dispatchChanged();
    return true;
  } catch {
    dispatchChanged();
    return false;
  }
};

export const resetKeyBindings = (): void => {
  cachedBindings = normaliseBindings(defaultKeyBindings);
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
  dispatchChanged();
};

export const updateBindingForButton = (button: GameButton, keys: string[]): boolean => {
  const current = getActiveKeyBindings();
  return setActiveKeyBindings({
    ...current,
    [button]: dedupe(keys),
  });
};

export const addBindingKeyForButton = (button: GameButton, key: string): boolean => {
  const current = getActiveKeyBindings();
  const nextKeys = dedupe([key, ...(current[button] ?? [])]);
  return updateBindingForButton(button, nextKeys);
};
