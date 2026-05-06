import { GameState } from "../../../core/state";
import { saveGame } from "../../../core/save";
import { SerialConnectionStatus } from "../../../core/memory/registers";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { ScriptRunner, ensureRunnerVariables, getSerialContext } from "./utils";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { Overworld } from "@pokecrystal/core/types/overworld";

const LINK_TRADECENTER = 2;
const LINK_COLOSSEUM = 3;

const reportLinkResult = (runner: ScriptRunner | null | undefined, success: boolean): void => {
  if (!runner) {
    return;
  }
  const variables = ensureRunnerVariables(runner);
  runner.last_condition_result = success;
  runner.last_value = success ? 1 : 0;
  variables._value = runner.last_value;
};

const recordAction = (game_state: GameState, action: number, runner?: ScriptRunner | null): number => {
  game_state.wram.wPlayerLinkAction = action;
  game_state.wram.wChosenCableClubRoom = action;
  if (runner) {
    const variables = ensureRunnerVariables(runner);
    runner.last_value = action;
    variables._value = action;
    runner.last_condition_result = true;
  }
  return action;
};

const resetLinkState = (game_state: GameState): void => {
  const wram = game_state.wram;
  wram.wLinkMode = 0;
  wram.wPlayerLinkAction = 0;
  wram.wChosenCableClubRoom = 0;
  const serial = getSerialContext(game_state);
  if (serial?.reset) {
    serial.reset();
  }
};

const ensureSerialExternalClock = (game_state: GameState): void => {
  const serial = getSerialContext(game_state);
  if (!serial || serial.connection_status === undefined) {
    return;
  }
  if (serial.connection_status === SerialConnectionStatus.CONNECTION_NOT_ESTABLISHED) {
    serial.connection_status = SerialConnectionStatus.USING_EXTERNAL_CLOCK;
  }
};

export function set_bits_for_link_trade_request(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/link/link.asm::SetBitsForLinkTradeRequest
  void overworld;
  void event_manager;
  return recordAction(game_state, LINK_TRADECENTER - 1, runner ?? null);
}

export function set_bits_for_battle_request(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/link/link.asm::SetBitsForBattleRequest
  void overworld;
  void event_manager;
  return recordAction(game_state, LINK_COLOSSEUM - 1, runner ?? null);
}

export function set_bits_for_time_capsule_request(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): number {
  // ASM: engine/link/link.asm::SetBitsForTimeCapsuleRequest
  void overworld;
  void event_manager;
  return recordAction(game_state, 0, runner ?? null);
}

export function ask_mobile_or_cable(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): string {
  // ASM: engine/link/link.asm::AskMobileOrCable
  void game_state;
  void overworld;
  void event_manager;
  if (runner) {
    const variables = ensureRunnerVariables(runner);
    runner.last_condition_result = true;
    runner.last_value = ".Cable";
    variables._value = ".Cable";
  }
  return ".Cable";
}

export function try_quick_save(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::TryQuickSave
  void overworld;
  void event_manager;

  let result = false;
  try {
    void saveGame(game_state, "savegame");
    result = true;
  } catch (error) {
    result = false;
  }

  if (runner) {
    const variables = ensureRunnerVariables(runner);
    runner.last_condition_result = result;
    runner.last_value = result ? 1 : 0;
    variables._value = runner.last_value;
  }
  return result;
}

export function wait_for_linked_friend(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::WaitForLinkedFriend
  void overworld;
  void event_manager;

  const variables = ensureRunnerVariables(runner ?? null);
  const ready = variables._link_friend_ready ?? true;
  if (!ready) {
    game_state.wram.script_memory["wScriptVar"] = 0;
    reportLinkResult(runner ?? null, false);
    return false;
  }

  game_state.wram.script_memory["wScriptVar"] = 1;
  ensureSerialExternalClock(game_state);
  reportLinkResult(runner ?? null, true);
  return true;
}

export function wait_for_other_player_to_exit(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::WaitForOtherPlayerToExit
  void overworld;
  void event_manager;

  resetLinkState(game_state);
  if (runner) {
    const variables = ensureRunnerVariables(runner);
    runner.last_condition_result = true;
    runner.last_value = true;
    variables._value = runner.last_value;
  }
  return true;
}

export function cable_club_check_which_chris(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::CableClubCheckWhichChris
  void game_state;
  void overworld;
  void event_manager;
  const isFemale = game_state.wram.player_gender === PlayerGender.FEMALE;
  reportLinkResult(runner ?? null, !isFemale);
  return !isFemale;
}

export function check_link_timeout_receptionist(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::CheckLinkTimeout_Receptionist
  void overworld;
  void event_manager;

  const chosenRoom = Number(game_state.wram.wChosenCableClubRoom ?? 0);
  game_state.wram.wPlayerLinkAction = chosenRoom;
  const variables = ensureRunnerVariables(runner ?? null);
  if (variables._link_timeout) {
    game_state.wram.script_memory["wScriptVar"] = 0;
    game_state.wram.script_memory["wOtherPlayerLinkMode"] = 0;
    resetLinkState(game_state);
    reportLinkResult(runner ?? null, false);
    return false;
  }

  let otherMode = Number.parseInt(String(variables._other_player_link_mode ?? (chosenRoom + 1)), 10);
  if (!Number.isFinite(otherMode)) {
    otherMode = 1;
  }
  game_state.wram.script_memory["wScriptVar"] = 1;
  game_state.wram.script_memory["wOtherPlayerLinkMode"] = otherMode;
  ensureSerialExternalClock(game_state);
  reportLinkResult(runner ?? null, true);
  return true;
}

export function check_both_selected_same_room(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::CheckBothSelectedSameRoom
  void overworld;
  void event_manager;

  const chosenRoom = Number(game_state.wram.wChosenCableClubRoom ?? 0);
  const variables = ensureRunnerVariables(runner ?? null);
  let otherRoom = variables._other_player_room ?? chosenRoom;
  otherRoom = Number.parseInt(String(otherRoom), 10);
  if (!Number.isFinite(otherRoom)) {
    otherRoom = chosenRoom;
  }

  if (otherRoom !== chosenRoom) {
    game_state.wram.script_memory["wScriptVar"] = 0;
    reportLinkResult(runner ?? null, false);
    return false;
  }

  game_state.wram.wLinkMode = chosenRoom + 1;
  game_state.wram.script_memory["wScriptVar"] = 1;
  reportLinkResult(runner ?? null, true);
  return true;
}

export function close_link(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  // ASM: engine/link/link.asm::CloseLink
  void overworld;
  void event_manager;

  resetLinkState(game_state);
  reportLinkResult(runner ?? null, false);
  return false;
}
