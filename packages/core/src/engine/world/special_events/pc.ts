import { GameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import {
  BillPC as modernBillPC,
  pokemon_center_pc as modernPokemonCenterPC,
} from "../special-events/pc";

export function pokemon_center_pc(
  game_state: GameState,
  runner?: ScriptRunner,
  overworld?: OverworldEngine,
  event_manager?: EventManager,
): ReturnType<typeof modernPokemonCenterPC> {
  return modernPokemonCenterPC(game_state, { runner, overworld, event_manager });
}

export function BillPC(
  game_state: GameState,
  overworld?: OverworldEngine,
): ReturnType<typeof modernBillPC> {
  return modernBillPC(game_state, { overworld });
}
