import type { GameState } from "@pokecrystal/core/core/state";

// ASM: constants/event_flags.asm (temporary event flags cleared on map reload).
const TEMP_FLAG_PREFIX = "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_";

type EventFlags = Record<string, boolean>;

export const clearTemporaryEventFlags = (gameState: GameState): string[] => {
  const cleared: string[] = [];
  const wramFlags: EventFlags = gameState.wram.event_flags ?? {};
  const sramFlags: EventFlags = gameState.sram.event_flags;

  for (const flagName of Object.keys(wramFlags)) {
    if (!flagName.startsWith(TEMP_FLAG_PREFIX)) {
      continue;
    }
    const wramWasSet = Boolean(wramFlags[flagName]);
    const sramWasSet = sramFlags !== wramFlags && Boolean(sramFlags[flagName]);

    if (wramWasSet || sramWasSet) {
      if (!cleared.includes(flagName)) {
        cleared.push(flagName);
      }
    }

    wramFlags[flagName] = false;
    if (sramFlags !== wramFlags) {
      sramFlags[flagName] = false;
    }
  }

  if (sramFlags !== wramFlags) {
    for (const flagName of Object.keys(sramFlags)) {
      if (!flagName.startsWith(TEMP_FLAG_PREFIX)) {
        continue;
      }
      if (!cleared.includes(flagName) && sramFlags[flagName]) {
        sramFlags[flagName] = false;
        cleared.push(flagName);
      }
    }
  }

  return cleared;
};
