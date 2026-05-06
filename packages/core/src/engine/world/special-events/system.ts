import { GameState, createInitialGameState } from "@pokecrystal/core/core/state";
import {
  RunnerVariableMap,
  ScriptRunner,
  ensureScriptMemory,
  syncEventFlags,
  syncScriptMemory,
} from "./utils";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";

const ensureRunnerVariablesInScriptMemory = (
  runner: ScriptRunner | null | undefined,
  game_state: GameState
): RunnerVariableMap => {
  if (!runner) {
    return {};
  }
  const scriptMemory = ensureScriptMemory(game_state);
  let scriptVars = scriptMemory.script_runner_variables as RunnerVariableMap | undefined;
  if (!scriptVars) {
    scriptVars = {};
    scriptMemory.script_runner_variables = scriptVars;
  }
  runner.variables = scriptVars;
  return scriptVars;
};

const softResetMemory = (game_state: GameState): void => {
  const preservedSram = game_state.sram;
  const preservedHasSeenIntro = game_state.has_seen_intro;
  preservedSram.script_memory = {};

  const fresh = createInitialGameState();
  game_state.wram = fresh.wram;
  game_state.vram = fresh.vram;
  game_state.hram = fresh.hram;
  game_state.sram = preservedSram;
  game_state.has_seen_intro = preservedHasSeenIntro;

  syncEventFlags(game_state);
  syncScriptMemory(game_state);
};

export function reset_special(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: OverworldEngine | null; event_manager?: EventManager | null } = {}
): string {
  // ASM: engine/overworld/reset.asm::Reset
  void overworld;
  void event_manager;

  softResetMemory(game_state);
  const variables = ensureRunnerVariablesInScriptMemory(runner ?? null, game_state);
  Object.keys(variables).forEach((key) => delete variables[key]);
  variables._value = "$0";

  if (runner) {
    runner.last_value = "$0";
    runner.last_condition_result = false;
    runner.stopExecution = true;
    runner.stop_execution = true;
  }
  return "$0";
}
