import { GameState } from "@pokecrystal/core/core/state";
import { MoveName } from "@pokecrystal/core/core/enums/move";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import { Pokemon, LearnedMove, toPokemon } from "@pokecrystal/core/core/models";
import type { Overworld } from "@pokecrystal/core/types/overworld";
import { ScriptRunner, ensureRunnerVariables } from "./utils";

const monDisplayName = (mon: Pokemon): string => {
  const nickname = (mon.nickname ?? "").trim();
  if (nickname) {
    return nickname;
  }
  const speciesId = mon.species?.id ?? "";
  return speciesId || "POKEMON";
};

const recordBuffers = (
  runner: ScriptRunner | undefined,
  monName: string,
  { moveName }: { moveName?: string | null } = {}
): void => {
  if (!runner?.string_buffers) {
    return;
  }
  runner.string_buffers.STRING_BUFFER_3 = monName;
  if (moveName) {
    runner.string_buffers.STRING_BUFFER_1 = moveName;
  }
};

const MOVE_ORDER_MAP = new Map(Object.values(MoveName).map((move, index) => [move, index + 1]));

const computeMoveId = (move: MoveName): number => {
  return MOVE_ORDER_MAP.get(move) ?? (MOVE_ORDER_MAP.size + 1);
};

const resolveMoveValue = (move: LearnedMove | null | undefined): string | null => {
  if (!move) {
    return null;
  }
  if (typeof move === "object" && move !== null && "name" in move) {
    return move.name ?? null;
  }
  return typeof move === "string" ? move : null;
};

const coerceMoveName = (move: LearnedMove | null | undefined): MoveName | null => {
  const raw = resolveMoveValue(move);
  if (!raw) {
    return null;
  }
  const token = String(raw).toUpperCase();
  return MOVE_ORDER_MAP.has(token as MoveName) ? (token as MoveName) : null;
};

const checkGen1Compatibility = (
  mon: Pokemon,
  { allowedMoveMax }: { allowedMoveMax: number }
): { resultCode: number; monName: string | null; moveName: string | null } => {
  const name = monDisplayName(mon);
  const speciesId = mon.species?.int_id ?? 999;
  if (speciesId >= 152) {
    return { resultCode: 1, monName: name, moveName: null };
  }

  const item = mon.item ?? "";
  if (typeof item === "string" && item.toUpperCase().includes("MAIL")) {
    return { resultCode: 3, monName: name, moveName: null };
  }

  for (const move of mon.moves ?? []) {
    const moveName = coerceMoveName(move);
    if (!moveName) {
      continue;
    }
    if (computeMoveId(moveName) > allowedMoveMax) {
      return { resultCode: 2, monName: name, moveName };
    }
  }

  return { resultCode: 0, monName: null, moveName: null };
};

export function check_time_capsule_compatibility(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/events/time_capsule.asm::CheckTimeCapsuleCompatibility
  void overworld;
  void event_manager;

  const allowedMoveMax = computeMoveId(MoveName.STRUGGLE);
  let resultCode = 0;
  let monName: string | null = null;
  let moveName: string | null = null;

  for (const mon of game_state.sram.party?.pokemon ?? []) {
    if (!mon) {
      continue;
    }
    const result = checkGen1Compatibility(toPokemon(mon), { allowedMoveMax });
    resultCode = result.resultCode;
    monName = result.monName;
    moveName = result.moveName;
    if (resultCode) {
      break;
    }
  }

  recordBuffers(runner, monName ?? "", { moveName });
  if (runner) {
    runner.last_value = resultCode;
    runner.last_condition_result = resultCode === 0;
    const variables = ensureRunnerVariables(runner);
    variables._value = resultCode;
  }
  return resultCode;
}

export function enter_time_capsule(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/events/time_capsule.asm::EnterTimeCapsule
  void overworld;
  void event_manager;

  game_state.wram.wLinkMode = 1;
  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = 1;
    const variables = ensureRunnerVariables(runner);
    variables._value = 1;
  }
  return true;
}

export function time_capsule(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/events/time_capsule.asm::TimeCapsule
  void overworld;
  void event_manager;

  game_state.wram.wPlayerLinkAction = 0;
  game_state.wram.wChosenCableClubRoom = 0;
  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = 1;
    const variables = ensureRunnerVariables(runner);
    variables._value = 1;
  }
  return true;
}
