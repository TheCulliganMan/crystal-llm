import { GameState } from "@pokecrystal/core/core/state";
import { DayCareSystem } from "@pokecrystal/core/engine/systems/day-care";
import type { DataLoader } from "@pokecrystal/core/core/data-loader";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { ScriptRunner } from "../story-events/runner";

type DayCareScriptResult = ReturnType<DayCareSystem["run_man"]>;
type DayCareInspectResult = ReturnType<DayCareSystem["run_yard_mon"]>;

type Overworld = {
  script_runner?: ScriptRunner;
  event_manager?: EventManager;
  day_care?: DayCareSystem;
  data_loader?: DataLoader | null;
  dataLoader?: DataLoader | null;
  refresh_event_flag?: (flagName: string, options?: { value?: boolean }) => void;
};

const resolveDayCareSystem = (
  game_state: GameState,
  {
    runner,
    overworld,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null } = {}
): DayCareSystem => {
  const candidate = runner?.day_care ?? overworld?.day_care;
  if (candidate instanceof DayCareSystem) {
    return candidate;
  }
  const dataLoader =
    runner?.data_loader ?? runner?.dataLoader ?? overworld?.data_loader ?? overworld?.dataLoader ?? undefined;
  return new DayCareSystem(game_state, dataLoader, overworld ?? undefined);
};

export function DayCareMan(game_state: GameState, overworld?: Overworld | null): DayCareScriptResult {
  // ASM: engine/events/daycare.asm::DayCareMan
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return day_care_man(game_state, { runner, overworld, event_manager });
}

export function DayCareLady(game_state: GameState, overworld?: Overworld | null): DayCareScriptResult {
  // ASM: engine/events/daycare.asm::DayCareLady
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return day_care_lady(game_state, { runner, overworld, event_manager });
}

export function day_care_man(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): DayCareScriptResult {
  const system = resolveDayCareSystem(game_state, { runner, overworld });
  return system.run_man({ runner, event_manager });
}

export function day_care_lady(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): DayCareScriptResult {
  const system = resolveDayCareSystem(game_state, { runner, overworld });
  return system.run_lady({ runner, event_manager });
}

export function day_care_man_outside(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): string {
  // ASM: engine/events/daycare.asm::DayCareManOutside
  const system = resolveDayCareSystem(game_state, { runner, overworld });
  const outcome = system.run_man_outside({ runner, event_manager });
  if (runner) {
      runner.last_condition_result = outcome === "FALSE";
  }
  return outcome;
}

export function DayCareManOutside(game_state: GameState, overworld?: Overworld | null): string {
  // ASM: engine/events/daycare.asm::DayCareManOutside
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return day_care_man_outside(game_state, { runner, overworld, event_manager });
}

export function day_care_mon1(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): DayCareInspectResult {
  void event_manager;
  const system = resolveDayCareSystem(game_state, { runner, overworld });
  return system.run_yard_mon(0);
}

export function day_care_mon2(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld | null; event_manager?: EventManager } = {}
): DayCareInspectResult {
  void event_manager;
  const system = resolveDayCareSystem(game_state, { runner, overworld });
  return system.run_yard_mon(1);
}

export function DayCareMon1(game_state: GameState, overworld?: Overworld | null): DayCareInspectResult {
  // ASM: data/events/special_pointers.asm::DayCareMon1
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return day_care_mon1(game_state, { runner, overworld, event_manager });
}

export function DayCareMon2(game_state: GameState, overworld?: Overworld | null): DayCareInspectResult {
  // ASM: data/events/special_pointers.asm::DayCareMon2
  const runner = overworld?.script_runner;
  const event_manager = overworld?.event_manager;
  return day_care_mon2(game_state, { runner, overworld, event_manager });
}
