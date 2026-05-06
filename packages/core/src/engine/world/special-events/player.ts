import { GameState } from "@pokecrystal/core/core/state";
import { ensureRunnerVariables, ScriptRunner } from "./utils";
import type { Overworld as BaseOverworld } from "@pokecrystal/core/types/overworld";
import type { EventManager } from "@pokecrystal/core/engine/world/events";
import type { SpriteAnimation } from "@pokecrystal/core/engine/systems/animation";

type OverworldPaletteContext = Partial<BaseOverworld> & {
  player_palette_id?: number;
  player_animations?: Record<string, SpriteAnimation> | null;
  _create_player_animations?: () => Record<string, SpriteAnimation>;
};

const _valueKey = "_value";

const resolveScriptValue = (runner?: ScriptRunner | null): number => {
  let value: unknown = 0;
  if (runner) {
    value = runner.variables?.[_valueKey] ?? runner.last_value ?? 0;
  }
  if (value === null || value === undefined) {
    return 0;
  }
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) {
    throw new Error(`Invalid palette token '${value}'`);
  }
  return Math.trunc(numeric);
};

export function set_player_palette(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: OverworldPaletteContext | null; event_manager?: EventManager } = {}
): number {
  // ASM: engine/events/specials.asm::SetPlayerPalette
  void event_manager;

  const rawValue = resolveScriptValue(runner);
  if ((rawValue & 0x80) === 0) {
    if (runner) {
      runner.last_condition_result = false;
      runner.last_value = game_state.wram.player_palette_id;
    }
    return game_state.wram.player_palette_id;
  }

  const paletteId = (rawValue >> 4) & 0x7;
  game_state.wram.player_palette_id = paletteId;

  if (overworld) {
    if (overworld.player_palette_id !== undefined) {
      overworld.player_palette_id = paletteId;
    }
    const refresh = overworld._create_player_animations;
    if (typeof refresh === "function") {
      overworld.player_animations = refresh();
    }
  }

  if (runner) {
    runner.last_condition_result = true;
    runner.last_value = paletteId;
    const variables = ensureRunnerVariables(runner);
    variables[_valueKey] = paletteId;
  }

  return paletteId;
}
