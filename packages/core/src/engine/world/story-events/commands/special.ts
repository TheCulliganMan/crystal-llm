import { GameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { SPECIAL_FUNCTIONS } from "@pokecrystal/core/engine/world/special-events/registry";
import { Command } from "./base";
import type { OverworldContext } from "./base";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { ScriptRunner } from "../runner";
import type {
  PokemonCenterOwner,
  PokemonCenterService,
  SpecialContext,
  SpecialFunction,
  SpecialOverworld,
} from "@pokecrystal/core/engine/world/special-events/special-types";

type OverworldLike = SpecialOverworld;

type OverworldExtras = {
  pokemon_center?: PokemonCenterService | null;
  audio_engine?: AudioEngine | null;
  audioEngine?: AudioEngine | null;
  ui?: unknown;
};

type OverworldWithExtras = OverworldLike & OverworldExtras;

// ASM: engine/overworld/scripting.asm::Script_special

const getParamObjectInfo = (fn: Function): { isObject: boolean; hasGameState: boolean } => {
  const source = Function.prototype.toString.call(fn);
  const match = source.match(/^[^(]*\(([^)]*)\)/);
  if (!match) {
    return { isObject: false, hasGameState: false };
  }
  const params = match[1]?.trim() ?? "";
  if (!params.startsWith("{")) {
    return { isObject: false, hasGameState: false };
  }
  return { isObject: true, hasGameState: /\bgame_state\b/.test(params) };
};

const isParamsFunction = (
  fn: SpecialFunction,
): fn is (params: { game_state: GameState } & Partial<SpecialContext>) => unknown =>
  fn.length === 1 && getParamObjectInfo(fn).hasGameState;

const resolvePokemonCenter = (
  runner?: ScriptRunner & PokemonCenterOwner,
  overworld?: OverworldWithExtras | null,
): PokemonCenterService | null => {
  if (runner?.pokemon_center) {
    return runner.pokemon_center;
  }
  if (overworld?.pokemon_center) {
    return overworld.pokemon_center;
  }
  return null;
};

export class SpecialCommand extends Command {
  private readonly functionName: string;

  constructor(functionName: string) {
    super();
    const cleaned = functionName.replace(/,$/, "").trim();
    this.functionName = cleaned.split(/\s*;\s*/)[0]?.trim() ?? cleaned;
  }

  execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const runnerOverworld = (runner?.overworld as OverworldWithExtras | undefined) ?? null;
    const activeOverworld: OverworldWithExtras | null =
      runnerOverworld ?? (overworld as OverworldWithExtras | null) ?? null;
    if (this.functionName === "SetDayOfWeek") {
      pushDebugLog(
        `[script] special ${this.functionName} context runner_overworld=${Boolean(runnerOverworld)} ` +
          `runner_ui=${Boolean(runnerOverworld?.ui)} arg_overworld=${Boolean(overworld)} ` +
          `arg_ui=${Boolean((overworld as OverworldWithExtras)?.ui)}`,
      );
    }
    const functionImpl = SPECIAL_FUNCTIONS[this.functionName] as SpecialFunction | undefined;

    if (!functionImpl) {
      const center = resolvePokemonCenter(runner, activeOverworld);
      if (center) {
        if (this.functionName === "HealParty" && typeof center.healParty === "function") {
          center.healParty();
          if (runner) {
            runner.last_condition_result = true;
          }
          return;
        }
        if (
          this.functionName === "HealMachineAnim" &&
          (typeof center.playHealMachineAnimation === "function" ||
            typeof center.playHealMachineAnimationAsync === "function")
        ) {
          const rawValue = runner?.variables?.["_value"];
          const animationId = rawValue === null || rawValue === undefined ? null : String(rawValue);
          const maybePromise =
            typeof center.playHealMachineAnimationAsync === "function"
              ? center.playHealMachineAnimationAsync(animationId, activeOverworld)
              : (center.playHealMachineAnimation?.(animationId, activeOverworld), null);
          if (runner) {
            runner.last_condition_result = true;
            if (maybePromise && typeof maybePromise.then === "function") {
              runner.pause?.();
              maybePromise.finally(() => {
                runner.resume?.();
              });
            }
          }
          return;
        }
      }
      throw new Error(`Unknown special function '${this.functionName}'.`);
    }

    const paramInfo = getParamObjectInfo(functionImpl);
    if (paramInfo.isObject && !paramInfo.hasGameState) {
      throw new Error(
        `Special '${this.functionName}' must include 'game_state' in its parameter destructuring.`,
      );
    }

    const previousValue = runner ? runner.last_value : undefined;
    const audioEngine =
      activeOverworld?.audio_engine ?? activeOverworld?.audioEngine ?? null;
    let result: unknown;
    const context: SpecialContext = {
      runner,
      overworld: activeOverworld,
      event_manager: eventManager,
      audio_engine: audioEngine,
    };
    const stackDepth = runner?._script_stack?.length ?? 0;
    const awaitingResume = runner?._awaiting_resume ?? 0;
    pushDebugLog(`[script] special ${this.functionName} start (stack=${stackDepth} awaiting=${awaitingResume})`);

    if (isParamsFunction(functionImpl)) {
      result = functionImpl({ game_state: gameState, ...context });
    } else {
      result = functionImpl(gameState, context);
    }

    const isPromiseLike = (value: unknown): value is Promise<unknown> =>
      Boolean(value && typeof value === "object" && typeof (value as Promise<unknown>).then === "function");

    if (runner && isPromiseLike(result)) {
      runner.pause?.();
      result
        .then((resolved) => {
          const currentValue = runner.last_value;
          if (currentValue === previousValue || currentValue === undefined || currentValue === null) {
            runner.last_value = resolved;
          }
          if (typeof resolved === "boolean") {
            runner.last_condition_result = resolved;
          }
          pushDebugLog(`[script] special ${this.functionName} resolved`);
        })
        .catch((error) => {
          console.error(`Special '${this.functionName}' rejected.`, error);
          pushDebugLog(`[script] special ${this.functionName} rejected`);
        })
        .finally(() => {
          runner.resume?.();
          pushDebugLog(`[script] special ${this.functionName} resume`);
        });
      return;
    }

    if (runner && result !== undefined && result !== null) {
      const currentValue = runner.last_value;
      if (currentValue === previousValue || currentValue === undefined || currentValue === null) {
        runner.last_value = result;
      }
      if (typeof result === "boolean") {
        runner.last_condition_result = result;
      }
    }
  }
}
