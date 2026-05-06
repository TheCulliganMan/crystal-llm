import { PlayerGender } from "@pokecrystal/core/core/enums";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";

const PlayerState_VALUES = new Set(Object.values(PlayerState));

export const PLAYER_STATE_FLAGS: Record<PlayerState, number> = {
  [PlayerState.NORMAL]: 0,
  [PlayerState.BIKE]: 1,
  [PlayerState.SKATE]: 2,
  [PlayerState.SURF]: 4,
  [PlayerState.SURF_PIKA]: 8,
};

export const FLAG_TO_PLAYER_STATE: Record<number, PlayerState> = Object.fromEntries(
  Object.entries(PLAYER_STATE_FLAGS).map(([state, flag]) => [
    flag,
    state as PlayerState,
  ])
) as Record<number, PlayerState>;

export const PLAYER_STATE_SPRITES: Record<PlayerGender, Array<[number, string]>> = {
  [PlayerGender.MALE]: [
    [PLAYER_STATE_FLAGS[PlayerState.NORMAL], "chris"],
    [PLAYER_STATE_FLAGS[PlayerState.BIKE], "chris_bike"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF], "surf"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF_PIKA], "surfing_pikachu"],
  ],
  [PlayerGender.FEMALE]: [
    [PLAYER_STATE_FLAGS[PlayerState.NORMAL], "kris"],
    [PLAYER_STATE_FLAGS[PlayerState.BIKE], "kris_bike"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF], "surf"],
    [PLAYER_STATE_FLAGS[PlayerState.SURF_PIKA], "surfing_pikachu"],
  ],
};

// ASM mapping: pokecrystal_disassembly/constants/sprite_data_constants.asm (PAL_OW_* indices).
const PAL_OW_RED = 0;
const PAL_OW_BLUE = 1;

export function normalizePlayerState(value: unknown): PlayerState {
  if (PlayerState_VALUES.has(value as PlayerState)) {
    return value as PlayerState;
  }
  if (typeof value === "number") {
    return playerStateFromFlag(value);
  }
  if (typeof value === "string") {
    const trimmed = value.trim().toLowerCase();
    for (const candidate of Object.values(PlayerState)) {
      if (trimmed === candidate.toLowerCase()) {
        return candidate as PlayerState;
      }
    }
  }
  return PlayerState.NORMAL;
}

export function playerStateFromFlag(flag: number | null | undefined): PlayerState {
  if (flag === null || flag === undefined) {
    return PlayerState.NORMAL;
  }
  return FLAG_TO_PLAYER_STATE[flag] ?? PlayerState.NORMAL;
}

export function flagForPlayerState(state: PlayerState): number {
  return PLAYER_STATE_FLAGS[state] ?? 0;
}

export function spriteForStateFlag(gender: PlayerGender, flag: number): string {
  const table = PLAYER_STATE_SPRITES[gender] ?? PLAYER_STATE_SPRITES[PlayerGender.MALE];
  for (const [entryFlag, spriteId] of table) {
    if (entryFlag === flag) {
      return spriteId;
    }
  }
  return table[0]?.[1] ?? "chris";
}

export function paletteForStateFlag(gender: PlayerGender, flag: number): number {
  const state = playerStateFromFlag(flag);
  switch (state) {
    case PlayerState.SURF:
      return PAL_OW_BLUE;
    case PlayerState.SURF_PIKA:
      return PAL_OW_RED;
    case PlayerState.NORMAL:
    case PlayerState.BIKE:
    case PlayerState.SKATE:
    default:
      return gender === PlayerGender.FEMALE ? PAL_OW_BLUE : PAL_OW_RED;
  }
}
