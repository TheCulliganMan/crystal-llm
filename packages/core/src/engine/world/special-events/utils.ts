import { GameState } from "@pokecrystal/core/core/state";
import type { ScriptRunner as StoryScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { SerialConnectionStatus } from "@pokecrystal/core/core/memory/registers";

export type RunnerVariableMap = Record<string, unknown>;
export type ScriptMemory = Record<string, unknown>;

export type ScriptRunner = StoryScriptRunner;

export type SerialContext = {
  reset?: () => void;
  connection_status?: SerialConnectionStatus;
};

type GameStateWithSerial = GameState & {
  serial?: SerialContext;
};

export function ensureRunnerVariables(runner?: ScriptRunner | null): RunnerVariableMap {
  if (!runner) {
    return {};
  }
  if (!runner.variables) {
    runner.variables = {};
  }
  return runner.variables;
}

export function ensureScriptMemory(gameState: GameState): ScriptMemory {
  if (!gameState.wram.script_memory) {
    gameState.wram.script_memory = {};
  }
  return gameState.wram.script_memory;
}

export function setRunnerValue<T>(
  runner: ScriptRunner | null | undefined,
  value: T,
  { truthy }: { truthy?: boolean } = {}
): T {
  if (!runner) {
    return value;
  }
  const variables = ensureRunnerVariables(runner);
  variables._value = value;
  runner.last_value = value;
  const falseyTokens = new Set(["FALSE", "$0", "0"]);
  const resolved = truthy ?? (Boolean(value) && !falseyTokens.has(String(value)));
  runner.last_condition_result = Boolean(resolved);
  return value;
}

export function runnerValue(
  runner: ScriptRunner | null | undefined,
  defaultValue: unknown = "0"
): unknown {
  if (!runner) {
    return defaultValue;
  }
  const variables = ensureRunnerVariables(runner);
  return variables._value ?? defaultValue;
}

export function resolveScriptText(
  label: string,
  {
    runner,
    overworld,
    defaultValue,
  }: {
    runner?: ScriptRunner | null;
    overworld?: unknown;
    defaultValue?: string | null;
  } = {}
): string {
  const dataLoader = runner?.data_loader ?? runner?.dataLoader;
  const rawText =
    dataLoader?.getText?.(label) ??
    dataLoader?.get_text?.(label) ??
    dataLoader?.getTextByLabel?.(label) ??
    label;
  const formatted = runner?.formatText ? runner.formatText(rawText) : rawText;
  if (formatted) {
    return formatted;
  }
  if (defaultValue !== undefined && defaultValue !== null) {
    return defaultValue;
  }
  return "";
}

export function syncEventFlags(gameState: GameState): void {
  if (gameState.wram.event_flags !== gameState.sram.event_flags) {
    gameState.wram.event_flags = gameState.sram.event_flags;
  }
}

export function getSerialContext(gameState: GameState): SerialContext | undefined {
  return (gameState as GameStateWithSerial).serial;
}

export function ensureSerialContext(gameState: GameState): SerialContext {
  const typed = gameState as GameStateWithSerial;
  if (!typed.serial) {
    typed.serial = {};
  }
  return typed.serial;
}

export function setSerialConnectionStatus(
  gameState: GameState,
  status: SerialConnectionStatus,
): void {
  ensureSerialContext(gameState).connection_status = status;
}

export function syncScriptMemory(gameState: GameState): void {
  if (gameState.wram.script_memory !== gameState.sram.script_memory) {
    gameState.wram.script_memory = gameState.sram.script_memory;
  }
}
