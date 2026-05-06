"use client";

import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { ScriptInputAdapter } from "@pokecrystal/core/input/script-adapter";
import type { InputEvent } from "@pokecrystal/core/input/adapters";
import { Spawn } from "@pokecrystal/core/engine/world/maps";
import type { Game, GameDebugScene, GameDebugStatus } from "./game";

export const DEBUG_SCENE_OPTIONS: Array<{ value: GameDebugScene; label: string }> = [
  { value: "intro", label: "Intro" },
  { value: "title", label: "Title Screen" },
  { value: "main_menu", label: "Main Menu" },
  { value: "continue", label: "Continue" },
  { value: "delete_save", label: "Delete Save" },
  { value: "clock_reset", label: "Clock Reset" },
  { value: "gender", label: "Gender Select" },
  { value: "oak_intro", label: "Oak Intro" },
  { value: "new_game", label: "New Game Flow" },
  { value: "overworld", label: "Overworld" },
];

export const DEBUG_SPAWN_OPTIONS = Object.entries(Spawn)
  .filter(([name, value]) => typeof value === "number" && !/^\d+$/.test(name) && name !== "N_A")
  .map(([name, value]) => ({
    label: name.replaceAll("_", " "),
    value: value as Spawn,
  }));

export type VisualDebugSnapshot = {
  text: string;
  debug: GameDebugStatus;
};

export type VisualDebugRunResult = {
  complete: boolean;
  frames: number;
  events: number;
  snapshot: VisualDebugSnapshot;
  remaining_tokens: number;
  waiting_reason: string | null;
};

const toEngineEvent = (event: InputEvent): InstanceType<typeof gameEngine.event.Event> =>
  new gameEngine.event.Event(event.type, {
    key: event.key ?? null,
    code: event.key ?? null,
    button: event.button ?? null,
    direction: event.direction ?? null,
    is_press: event.is_press,
    text: event.text ?? null,
  });

export const buildVisualDebugSnapshot = (game: Game): VisualDebugSnapshot => ({
  text: JSON.stringify({
    mode: game.getState(),
    map: game.getMapName(),
    state: game.getDebugStatus(),
    local: game.getGameState().frame_counter,
  }),
  debug: game.getDebugStatus(),
});

export const parseVisualDebugScript = (raw: string): unknown[] => {
  const trimmed = raw.trim();
  if (!trimmed) {
    return [];
  }
  const parsed = JSON.parse(trimmed) as unknown;
  if (!Array.isArray(parsed)) {
    throw new Error("Visual debug scripts must be a JSON array.");
  }
  return parsed;
};

export const runVisualDebugScript = async (
  game: Game,
  script: unknown[],
  options?: {
    maxFrames?: number;
  }
): Promise<VisualDebugRunResult> => {
  const adapter = new ScriptInputAdapter(script, { stdin: null });
  const maxFrames = Math.max(1, options?.maxFrames ?? 1200);
  let frames = 0;
  let events = 0;

  while (frames < maxFrames) {
    const status = game.getDebugStatus();
    adapter.updatePromptState(status.prompt_pending, status.prompt_reason);
    const nextEvents = adapter.poll();
    const doneBeforeFrame =
      nextEvents.length === 0 &&
      adapter.remaining_tokens === 0 &&
      adapter.waiting_reason === null &&
      !status.prompt_pending &&
      !status.script_busy;
    if (doneBeforeFrame) {
      break;
    }
    for (const event of nextEvents) {
      game.postEvent(toEngineEvent(event));
      events += 1;
    }
    game.tick();
    frames += 1;
  }

  await Promise.resolve();
  return {
    complete: adapter.remaining_tokens === 0 && adapter.waiting_reason === null,
    frames,
    events,
    snapshot: buildVisualDebugSnapshot(game),
    remaining_tokens: adapter.remaining_tokens,
    waiting_reason: adapter.waiting_reason,
  };
};
