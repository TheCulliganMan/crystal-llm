import { GameState, SRAMSchema } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { TimeSystem } from "@pokecrystal/core/engine/systems/time";
import { closeText, showText, type EventManager } from "@pokecrystal/core/engine/events/events";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import { DayOfWeekScreen } from "@pokecrystal/core/ui/screens/day-of-week-screen";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import type { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { ScriptRunner, ensureRunnerVariables } from "./utils";
import type { SpecialContext } from "./special-types";
import type { Overworld } from "@pokecrystal/core/types/overworld";

type PokemonCenterOwner = {
  pokemon_center?: {
    healParty?: () => void;
    playHealMachineAnimation?: (animationId: string | null, overworld: Overworld | null) => void;
  } | null;
};

type RNGSource = { randrange: (upper: number) => number };

export type TimeSpecialOverworldDialogue = {
  forceCloseText?: () => void;
  _handle_close_text?: ({ force }: { force: boolean }) => void;
  suspend?: () => void;
  resume?: () => void;
} | null | undefined;

export type TimeSpecialOverworldUI = BaseUI & {
  eventQueue?: GameEngineEventQueue | null;
};

export type TimeSpecialOverworld = {
  dialogue?: TimeSpecialOverworldDialogue;
  ui?: TimeSpecialOverworldUI | null;
  draw?: () => void;
  update?: () => void;
  audio_engine?: AudioEngine | null;
  input_capture_active?: boolean;
};

const DAY_NAMES: readonly string[] = [
  "SUNDAY",
  "MONDAY",
  "TUESDAY",
  "WEDNESDAY",
  "THURSDAY",
  "FRIDAY",
  "SATURDAY",
];

export async function set_day_of_week(
  game_state: GameState,
  context: SpecialContext
): Promise<number | boolean> {
  // ASM: engine/menus/intro_menu.asm::SetDayOfWeek
  pushDebugLog("[special] SetDayOfWeek start");
  const { runner, event_manager } = context;
  const overworld = context.overworld as TimeSpecialOverworld | null;
  const dialogue = overworld?.dialogue ?? null;
  dialogue?.suspend?.();
  if (overworld?.dialogue?.forceCloseText) {
    overworld.dialogue.forceCloseText();
  } else if (overworld?.dialogue?._handle_close_text) {
    overworld.dialogue._handle_close_text({ force: true });
  }
  if (event_manager) {
    closeText(event_manager);
  }
  try {
    const ui = overworld?.ui ?? null;
    const state = game_state as GameState & { sram?: GameState["sram"] | null };
    if (!state.sram) {
      state.sram = SRAMSchema.parse({});
    }
    if (!ui) {
      pushDebugLog("[special] SetDayOfWeek missing ui; defaulting to Sunday");
      state.sram.day_of_week = 0;
      if (runner) {
        runner.last_condition_result = true;
      }
      return true;
    }

    // In headless / MCP-driven runs, we may have a UI object but no event queue wired up.
    // If we enter the interactive day-of-week screen without input events, the script can stall
    // forever and leave the overworld movement lock stuck on.
    if (!ui.eventQueue) {
      pushDebugLog("[special] SetDayOfWeek missing eventQueue; defaulting to Sunday");
      state.sram.day_of_week = 0;
      if (runner) {
        runner.last_condition_result = true;
      }
      return true;
    }

    const screenWidthTiles = Math.max(1, Math.floor(ui.screenWidth / 8));
    const screenHeightTiles = Math.max(1, Math.floor(ui.screenHeight / 8));
    const screen = ui?.screen ?? null;
    if (!screen) {
      const menu = new SelectionPrompt(ui, [...DAY_NAMES], {
        windowOriginTiles: [0, 0],
        windowMinWidth: screenWidthTiles,
        windowMinHeight: screenHeightTiles,
        windowFill: [255, 255, 255],
      });
      const previous_capture = overworld?.input_capture_active ?? false;
      if (overworld) {
        overworld.input_capture_active = true;
      }
      const drawCallback = overworld?.draw
        ? () => {
            const ow = overworld!;
            try {
              ow.update?.();
            } catch {
              // Keep prompt responsive even if overworld update fails.
            }
            ow.draw!();
          }
        : undefined;
      let result = 0;
      try {
        result = await menu.runAsync({ drawCallback });
      } finally {
        if (overworld) {
          overworld.input_capture_active = previous_capture;
        }
      }
      state.sram.day_of_week = Number(result) % 7;
      if (runner) {
        runner.last_condition_result = true;
      }
      pushDebugLog(`[special] SetDayOfWeek done (${game_state.sram.day_of_week})`);
      return result;
    }

    const dayScreen = new DayOfWeekScreen(ui, game_state, overworld?.audio_engine ?? null);
    const previous_capture = overworld?.input_capture_active ?? false;
    if (overworld) {
      overworld.input_capture_active = true;
    }
    const drawCallback = overworld?.draw
      ? () => {
          try {
            overworld.update?.();
          } catch {
            // Keep prompt responsive even if overworld update fails.
          }
          overworld.draw!();
        }
      : undefined;
    let result = 0;
    try {
      while (true) {
        if (drawCallback) {
          drawCallback();
        }
        dayScreen.draw();
        ui.update?.();

        for (const event of gameEngine.event.get(ui.eventQueue)) {
          if (event.type === gameEngine.QUIT) {
            gameEngine.quit();
            throw new Error("SetDayOfWeek interrupted by quit event");
          }
          dayScreen.handleInput(event);
        }

        if (dayScreen.isConfirming()) {
          const confirmed = await dayScreen.runConfirmation({ drawCallback });
          if (confirmed) {
            result = dayScreen.getSelectedDay();
            state.sram.day_of_week = Number(result) % 7;
            break;
          }
          dayScreen.reset();
        }

        await nextFrame();
      }
    } finally {
      try {
        if (overworld?.draw) {
          overworld.draw();
          ui.update?.();
        } else if (ui.screen) {
          ui.screen.fill([0, 0, 0, 255]);
          ui.update?.();
        }
      } catch {
        // Best effort cleanup; never block script resume on render teardown.
      }
      if (overworld) {
        overworld.input_capture_active = previous_capture;
      }
    }
    if (runner) {
      runner.last_condition_result = true;
    }
    pushDebugLog(`[special] SetDayOfWeek done (${game_state.sram.day_of_week})`);
    return result;
  } finally {
    dialogue?.resume?.();
  }
}

export function initial_set_dst_flag(
  game_state: GameState,
  { event_manager }: { event_manager?: EventManager } = {}
): void {
  // ASM: engine/menus/intro_menu.asm::InitialSetDSTFlag
  game_state.sram.dst = true;
  if (event_manager) {
    const hours = game_state.sram.game_time_hours;
    const minutes = game_state.sram.game_time_minutes;
    const timeStr = `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")} DST,\nis that OK?`;
    showText(event_manager, timeStr);
  }
}

export function initial_clear_dst_flag(
  game_state: GameState,
  { event_manager }: { event_manager?: EventManager } = {}
): void {
  // ASM: engine/menus/intro_menu.asm::InitialClearDSTFlag
  game_state.sram.dst = false;
  if (event_manager) {
    const hours = game_state.sram.game_time_hours;
    const minutes = game_state.sram.game_time_minutes;
    const timeStr = `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")},\nis that OK?`;
    showText(event_manager, timeStr);
  }
}

export function update_time(game_state: GameState): void {
  // ASM: engine/tilesets/timeofday_pals.asm::UpdateTime
  const timeSystem = new TimeSystem(game_state);
  timeSystem.updateTime();
}

export function sample_kenji_break_countdown(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
    rng,
  }: {
    runner?: ScriptRunner & PokemonCenterOwner;
    overworld?: unknown;
    event_manager?: EventManager;
    rng?: RNGSource;
  } = {}
): number {
  // ASM: engine/events/kenji.asm::SampleKenjiBreakCountdown
  void overworld;
  void event_manager;

  const rngSource = rng ?? new HardwareRNG(game_state);
  const offset = rngSource.randrange(4);
  const value = 3 + offset;
  game_state.wram.wKenjiBreakTimer = value;

  if (runner) {
    runner.last_value = value;
    const variables = ensureRunnerVariables(runner);
    variables._value = value;
    runner.last_condition_result = true;
  }

  return value;
}
