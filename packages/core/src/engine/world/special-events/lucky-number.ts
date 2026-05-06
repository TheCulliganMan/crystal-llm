// ASM mapping: pokecrystal_disassembly/engine/events/lucky_number.asm (CheckForLuckyNumberWinners/PrintTodaysLuckyNumber helpers).
import { GameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { setRunnerValue, ScriptRunner } from "./utils";
import { Pokemon, toPokemon } from "@pokecrystal/core/core/models";
import type { Overworld } from "@pokecrystal/core/types/overworld";
import { resolveText } from "@pokecrystal/core/engine/world/story-events/text-helpers";

const ensureLuckyNumber = (game_state: GameState): number => {
  const storedDay = Number(game_state.sram.lucky_number_day ?? -1);
  const currentDay = Number(game_state.wram.wCurDay ?? -1) & 0xff;
  let number = Number(game_state.sram.lucky_id_number ?? 0) & 0xffff;
  if (currentDay !== storedDay) {
    const rng = new HardwareRNG(game_state);
    number = ((rng.nextByte() << 8) | rng.nextByte()) & 0xffff;
    game_state.sram.lucky_number_day = currentDay;
    game_state.sram.lucky_id_number = number;
  }
  return number;
};

const formatFiveDigits = (value: number): string => String(value & 0xffff).padStart(5, "0");

const resolveTrainerId = (value: unknown): number => {
  const trainerId = Number(value);
  if (!Number.isInteger(trainerId) || trainerId < 0 || trainerId > 0xffff) {
    throw new Error(`ASM-backed trainer ID is invalid: ${String(value)}.`);
  }
  return trainerId;
};

const matchSuffixLength = (a: number, b: number): number => {
  const left = formatFiveDigits(a);
  const right = formatFiveDigits(b);
  let count = 0;
  for (let i = 0; i < 5; i += 1) {
    const index = 4 - i;
    if (left[index] !== right[index]) {
      break;
    }
    count += 1;
  }
  return count;
};

const evaluateMatchTier = (matchLength: number): number => {
  if (matchLength >= 5) {
    return 1;
  }
  if (matchLength >= 3) {
    return 2;
  }
  if (matchLength >= 2) {
    return 3;
  }
  return 0;
};

const resolvePartyCount = (game_state: GameState): number => {
  const rawCount = game_state.wram.wPartyCount;
  const party = game_state.sram.party?.pokemon ?? [];
  const partyCount = Number(rawCount);
  if (!Number.isInteger(partyCount) || partyCount < 0 || partyCount > party.length) {
    throw new Error(`ASM-backed party count is invalid: ${String(rawCount)}.`);
  }
  return partyCount;
};

const resolveCurrentBoxIndex = (game_state: GameState, boxCount: number): number => {
  const rawCurrentBoxIndex = game_state.sram.current_pc_box;
  const currentBoxIndex = Number(rawCurrentBoxIndex);
  if (!Number.isInteger(currentBoxIndex) || currentBoxIndex < 0) {
    throw new Error(
      `ASM-backed current PC box index is invalid: ${String(game_state.sram.current_pc_box)}.`
    );
  }
  const maskedBoxIndex = currentBoxIndex & 0xf;
  if (maskedBoxIndex >= boxCount) {
    throw new Error(
      `ASM-backed current PC box index is invalid: ${String(game_state.sram.current_pc_box)}.`
    );
  }
  return maskedBoxIndex;
};

const resolveBoxCount = (rawCount: unknown, monsLength: number): number => {
  const boxCount = Number(rawCount);
  if (!Number.isInteger(boxCount) || boxCount < 0 || boxCount > monsLength) {
    throw new Error(`ASM-backed PC box count is invalid: ${String(rawCount)}.`);
  }
  return boxCount;
};

const iterAllMons = (game_state: GameState): Array<{ mon: Pokemon; source: "party" | "pc" }> => {
  const members: Array<{ mon: Pokemon; source: "party" | "pc" }> = [];
  const party = game_state.sram.party?.pokemon ?? [];
  const partyCount = resolvePartyCount(game_state);
  for (const mon of party.slice(0, partyCount)) {
    if (mon) {
      members.push({ mon: toPokemon(mon), source: "party" });
    }
  }

  const boxes = game_state.sram.pc_boxes ?? [];
  if (!boxes.length) {
    return members;
  }

  const currentBoxIndex = resolveCurrentBoxIndex(game_state, boxes.length);

  const orderedBoxes = [boxes[currentBoxIndex], ...boxes.filter((_box, index) => index !== currentBoxIndex)];
  for (const box of orderedBoxes) {
    const mons = box?.pokemon ?? [];
    const boxCount = resolveBoxCount(box?.count, mons.length);
    for (const mon of mons.slice(0, boxCount)) {
      if (mon) {
        members.push({ mon: toPokemon(mon), source: "pc" });
      }
    }
  }
  return members;
};

export function check_for_lucky_number_winners(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/events/lucky_number.asm::CheckForLuckyNumberWinners
  void overworld;
  const partyCount = resolvePartyCount(game_state);
  if (partyCount <= 0) {
    if (runner) {
      setRunnerValue(runner, 0, { truthy: false });
    }
    return 0;
  }
  const luckyNumber = ensureLuckyNumber(game_state);
  let bestTier = 0;
  let bestSource: "party" | "pc" | null = null;
  let bestSpecies: string | null = null;

  for (const { mon, source } of iterAllMons(game_state)) {
    const speciesId = String(mon.species?.id ?? "").toUpperCase();
    if (!speciesId || speciesId === "EGG") {
      continue;
    }
    const trainerId = resolveTrainerId(mon.original_trainer_id);
    const matchLength = matchSuffixLength(trainerId, luckyNumber);
    const tier = evaluateMatchTier(matchLength);
    if (
      tier > 0 &&
      (bestTier === 0 ||
        tier < bestTier ||
        (tier === bestTier && bestSource === "party" && source === "pc"))
    ) {
      // Smaller tier value is better (1 > 2 > 3 in prize order).
      // Equal-tier PC winners beat party winners, while the current PC box
      // remains preferred over later boxes because it is scanned first.
      bestTier = tier;
      bestSource = source;
      bestSpecies = speciesId;
    }
  }

  if (bestTier > 0 && bestSpecies) {
    game_state.wram.wCurPartySpecies = bestSpecies;
    const label =
      bestSource === "pc" ? "LuckyNumberMatchPCText" : "LuckyNumberMatchPartyText";
    const text = resolveText(runner ?? null, (overworld as Overworld | null) ?? null, label);
    if (event_manager?.dispatch) {
      event_manager.dispatch(new Event("show_text", { text }));
    }
  }

  if (runner) {
    setRunnerValue(runner, bestTier, { truthy: bestTier > 0 });
  }
  return bestTier;
}

export function check_lucky_number_show_flag(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/events/specials.asm::CheckLuckyNumberShowFlag
  void overworld;
  void event_manager;
  const flag = Boolean(game_state.wram.lucky_number_show_flag);
  if (runner) {
    setRunnerValue(runner, flag ? 1 : 0, { truthy: flag });
  }
  return flag;
}

export function reset_lucky_number_show_flag(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/events/specials.asm::ResetLuckyNumberShowFlag
  void overworld;
  void event_manager;
  game_state.wram.lucky_number_show_flag = false;
  ensureLuckyNumber(game_state);
  if (runner) {
    setRunnerValue(runner, 1, { truthy: true });
  }
  return true;
}
