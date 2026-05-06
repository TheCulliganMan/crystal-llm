import type { GameState } from "@pokecrystal/core/core/state";

const SPECIAL_FLAG_TOKENS = new Set(["", "0", "-1"]);

const normalizeFlagName = (flagName: string | null | undefined): string | null => {
  if (flagName === null || flagName === undefined) {
    return null;
  }
  const normalized = String(flagName).trim();
  if (!normalized) {
    return null;
  }
  return normalized;
};

type OverworldFlagRefresher = {
  refresh_event_flag?: (flagName: string, options?: { value?: boolean }) => void;
};

export const applyEventFlag = (
  gameState: GameState,
  flagName: string | null | undefined,
  {
    value,
    overworld,
  }: {
    value: boolean;
    overworld?: OverworldFlagRefresher | null;
  }
): void => {
  const normalized = normalizeFlagName(flagName);
  if (!normalized || SPECIAL_FLAG_TOKENS.has(normalized)) {
    return;
  }

  const boolValue = Boolean(value);
  const wramFlags = gameState.wram.event_flags;
  wramFlags[normalized] = boolValue;
  const sramFlags = (gameState.sram as { event_flags?: Record<string, boolean> })
    .event_flags;
  if (sramFlags && sramFlags !== wramFlags) {
    sramFlags[normalized] = boolValue;
  }

  if (overworld?.refresh_event_flag) {
    overworld.refresh_event_flag(normalized, { value: boolValue });
  }
};

export const clearEventFlag = (
  gameState: GameState,
  flagName: string | null | undefined,
  { overworld }: { overworld?: OverworldFlagRefresher | null } = {}
): void => {
  applyEventFlag(gameState, flagName, { value: false, overworld });
};
