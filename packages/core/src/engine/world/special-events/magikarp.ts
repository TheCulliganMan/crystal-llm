import { GameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { Overworld } from "@pokecrystal/core/types/overworld";
import { Pokemon, toPokemon } from "@pokecrystal/core/core/models";
import { ScriptRunner, ensureRunnerVariables } from "./utils";

export const MAGIKARPLENGTH_NOT_MAGIKARP = 0;
export const MAGIKARPLENGTH_REFUSED = 1;
export const MAGIKARPLENGTH_TOO_SHORT = 2;
export const MAGIKARPLENGTH_BEAT_RECORD = 3;

const MAGIKARP_LENGTH_TABLE: ReadonlyArray<[number, number]> = [
  [110, 1],
  [310, 2],
  [710, 4],
  [2710, 20],
  [7710, 50],
  [17710, 100],
  [32710, 150],
  [47710, 150],
  [57710, 100],
  [62710, 50],
  [64710, 20],
  [65210, 5],
  [65410, 2],
  [65510, 1],
];

type LengthResult = { feet: number; inches: number; length_mm: number };

const ensureStringBuffers = (runner?: ScriptRunner | null): Record<string, string> => {
  if (!runner) {
    return {};
  }
  if (!runner.string_buffers) {
    runner.string_buffers = {};
  }
  return runner.string_buffers;
};

const rotateRight = (value: number, count: number): number => {
  let rotated = value & 0xff;
  for (let i = 0; i < count; i += 1) {
    rotated = ((rotated >> 1) | ((rotated & 1) << 7)) & 0xff;
  }
  return rotated;
};

const composeDvBytes = (mon: Pokemon): [number, number] => {
  const dv0 = ((mon.dvs.attack & 0xf) << 4) | (mon.dvs.defense & 0xf);
  const dv1 = ((mon.dvs.speed & 0xf) << 4) | (mon.dvs.special & 0xf);
  return [dv0, dv1];
};

const bcLessThan = (b: number, threshold: number): boolean => {
  const thresholdHigh = (threshold >> 8) & 0xff;
  return b < thresholdHigh;
};

const bcMinus = (b: number, c: number, value: number): number => {
  const bc = ((b << 8) | c) - value;
  return bc & 0xffff;
};

const calculateMagikarpLength = (mon: Pokemon, trainerId: number): LengthResult => {
  const [dv0, dv1] = composeDvBytes(mon);
  const idHigh = rotateRight((trainerId >> 8) & 0xff, 1);
  const idLow = rotateRight(trainerId & 0xff, 1);

  const b = rotateRight(dv0, 2) ^ idHigh;
  const c = rotateRight(dv1, 2) ^ idLow;

  let lengthMm: number | null = null;
  if (b === 0 && c < 10) {
    lengthMm = c + 190;
  } else {
    let multiplier = 2;
    for (const [threshold, divisor] of MAGIKARP_LENGTH_TABLE) {
      if (bcLessThan(b, threshold)) {
        const delta = bcMinus(b, c, threshold);
        const quotient = (Math.floor(delta / divisor) & 0xff) >>> 0;
        lengthMm = quotient + 100 * (2 + multiplier);
        break;
      }
      multiplier += 1;
    }
    if (lengthMm === null) {
      const [threshold] = MAGIKARP_LENGTH_TABLE[MAGIKARP_LENGTH_TABLE.length - 1];
      const delta = bcMinus(b, c, threshold);
      lengthMm = 1600 + delta;
    }
  }

  const lengthTimes10 = (lengthMm ?? 0) * 10;
  const totalInches = Math.floor(lengthTimes10 / 254);
  const feet = Math.floor(totalInches / 12) & 0xff;
  const inches = (totalInches % 12) & 0xff;
  return { feet, inches, length_mm: lengthMm ?? 0 };
};

const formatLengthString = (feet: number, inches: number): string => {
  return `${feet}'${inches}"`;
};

export function check_magikarp_length(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/events/magikarp.asm::CheckMagikarpLength
  void overworld;
  void event_manager;

  const party = [...(game_state.sram.party?.pokemon ?? [])];
  if (!party.length || !party[0]) {
    throw new Error("CheckMagikarpLength requires at least one party Pokemon.");
  }

  const variables = runner ? ensureRunnerVariables(runner) : {};
  if (runner && variables._selection_cancelled) {
    runner.last_value = MAGIKARPLENGTH_REFUSED;
    variables._value = MAGIKARPLENGTH_REFUSED;
    runner.last_condition_result = false;
    return MAGIKARPLENGTH_REFUSED;
  }

  let index = 0;
  if (runner && variables._selected_party_index !== undefined) {
    index = Number(variables._selected_party_index) || 0;
  } else if (game_state.wram.wCurPartyMon !== undefined) {
    index = Number(game_state.wram.wCurPartyMon) || 0;
  }

  const mon = party[index];
  if (!mon) {
    throw new Error(`Party slot ${index} is empty.`);
  }

  const speciesId = String(mon.species?.id ?? "").toUpperCase();
  if (speciesId !== "MAGIKARP") {
    if (runner) {
      runner.last_value = MAGIKARPLENGTH_NOT_MAGIKARP;
      variables._value = MAGIKARPLENGTH_NOT_MAGIKARP;
      runner.last_condition_result = false;
    }
    return MAGIKARPLENGTH_NOT_MAGIKARP;
  }

  const trainerId = Number(game_state.sram.player_id ?? 0) & 0xffff;
  const length = calculateMagikarpLength(toPokemon(mon), trainerId);
  game_state.wram.wMagikarpLengthFeet = length.feet;
  game_state.wram.wMagikarpLengthInches = length.inches;

  const currentBest: [number, number] = [
    Number(game_state.wram.wBestMagikarpLengthFeet ?? 0),
    Number(game_state.wram.wBestMagikarpLengthInches ?? 0),
  ];
  const measured: [number, number] = [length.feet, length.inches];
  const stringBuffer = formatLengthString(length.feet, length.inches);
  if (runner) {
    const buffers = ensureStringBuffers(runner);
    buffers.STRING_BUFFER_1 = stringBuffer;
  }

  let result = MAGIKARPLENGTH_TOO_SHORT;
  if (measured[0] > currentBest[0] || (measured[0] === currentBest[0] && measured[1] > currentBest[1])) {
    game_state.wram.wBestMagikarpLengthFeet = length.feet;
    game_state.wram.wBestMagikarpLengthInches = length.inches;
    game_state.wram.best_magikarp_owner_name = String(mon.original_trainer_name ?? "");
    result = MAGIKARPLENGTH_BEAT_RECORD;
  }

  if (runner) {
    runner.last_value = result;
    variables._value = result;
    runner.last_condition_result = result === MAGIKARPLENGTH_BEAT_RECORD;
  }
  return result;
}

export function magikarp_house_sign(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/magikarp.asm::MagikarpHouseSign
  void overworld;
  void event_manager;

  const feet = Number(game_state.wram.wBestMagikarpLengthFeet ?? 0);
  const inches = Number(game_state.wram.wBestMagikarpLengthInches ?? 0);
  game_state.wram.wMagikarpLengthFeet = feet;
  game_state.wram.wMagikarpLengthInches = inches;
  const formatted = formatLengthString(feet, inches);

  if (runner) {
    const buffers = ensureStringBuffers(runner);
    buffers.STRING_BUFFER_1 = formatted;
    runner.last_value = formatted;
    runner.last_condition_result = true;
  }
  return formatted;
}
