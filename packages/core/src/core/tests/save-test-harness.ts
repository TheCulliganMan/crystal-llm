import path from "path";
import { createInitialGameState, type GameState } from "../state";
import { normalizeSaveSnapshot } from "../save";
import {
  guestSessionKey,
  guestSessionMetadataKey,
} from "../guest-session-storage";

type BrowserStorageOptions = {
  failSetItem?: boolean;
};

export const createBrowserStorage = (options: BrowserStorageOptions = {}): Storage => {
  const values = new Map<string, string>();
  return {
    get length() {
      return values.size;
    },
    clear() {
      values.clear();
    },
    getItem(key: string) {
      return values.has(key) ? values.get(key)! : null;
    },
    key(index: number) {
      return Array.from(values.keys())[index] ?? null;
    },
    removeItem(key: string) {
      values.delete(key);
    },
    setItem(key: string, value: string) {
      if (options.failSetItem) {
        throw new Error("storage unavailable");
      }
      values.set(key, value);
    },
  } as Storage;
};

export const installBrowserStorageWindow = (params?: {
  localStorage?: Storage;
  sessionStorage?: Storage;
}): (() => void) => {
  const originalWindow = global.window;
  Object.defineProperty(global, "window", {
    configurable: true,
    value: {
      localStorage: params?.localStorage ?? createBrowserStorage(),
      sessionStorage: params?.sessionStorage ?? createBrowserStorage(),
    },
  });
  return () => {
    Object.defineProperty(global, "window", {
      configurable: true,
      value: originalWindow,
    });
  };
};

export const createNamedGameState = (playerName: string): GameState => {
  const gameState = createInitialGameState();
  gameState.sram.player_name = playerName;
  return gameState;
};

export const createSerializedSnapshot = (
  playerName: string,
  source = `test:${playerName}`
): Record<string, unknown> => normalizeSaveSnapshot(createNamedGameState(playerName), source);

export const writeGuestSessionSnapshot = (
  slot: string,
  playerName: string,
  options?: {
    aliasSlot?: string;
    savedAt?: string | null;
    storage?: Storage;
  }
): void => {
  const targetStorage = options?.storage ?? window.localStorage;
  const targetSlot = options?.aliasSlot ?? slot;
  targetStorage.setItem(
    guestSessionKey(targetSlot),
    JSON.stringify(createSerializedSnapshot(playerName, `guest:${targetSlot}`))
  );
  if (options?.savedAt) {
    targetStorage.setItem(
      guestSessionMetadataKey(targetSlot),
      JSON.stringify({ saved_at: options.savedAt })
    );
  }
};

export const writeLegacyGuestSessionSnapshot = (
  slot: string,
  playerName: string,
  storage: Storage = window.localStorage
): void => {
  storage.setItem(
    `fs:/legacy/${path.basename(slot)}`,
    JSON.stringify(createSerializedSnapshot(playerName, `legacy:${slot}`))
  );
};
