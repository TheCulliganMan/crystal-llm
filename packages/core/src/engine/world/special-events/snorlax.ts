// ASM mapping: pokecrystal_disassembly/engine/events/specials.asm::SnorlaxAwake
import { GameState } from "@pokecrystal/core/core/state";
import { ScriptRunner, setRunnerValue } from "./utils";
import type { Overworld } from "@pokecrystal/core/types/overworld";
import type { EventManager } from "@pokecrystal/core/engine/events/events";

const PROXIMITY_COORDS: Array<[number, number]> = [
  [33, 8],
  [34, 10],
  [35, 10],
  [36, 8],
  [36, 9],
];

const snorlaxCoordCandidates = (x: number, y: number): Array<[number, number]> => {
  const candidates: Array<[number, number]> = [[x, y]];
  for (const xOffset of [1, 3]) {
    for (const yOffset of [1, 3]) {
      const normalizedX = (x - xOffset) / 2;
      const normalizedY = (y - yOffset) / 2;
      if (Number.isInteger(normalizedX) && Number.isInteger(normalizedY)) {
        candidates.push([normalizedX, normalizedY]);
      }
    }
  }
  return candidates;
};

export function snorlax_awake(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner; overworld?: Overworld; event_manager?: EventManager } = {}
): boolean {
  void overworld;
  void event_manager;
  const music = String(game_state.wram.wMapMusic ?? "").trim();
  if (music !== "MUSIC_POKE_FLUTE_CHANNEL") {
    if (runner) {
      setRunnerValue(runner, 0, { truthy: false });
    }
    return false;
  }
  const x = Number(game_state.wram.wXCoord ?? -1);
  const y = Number(game_state.wram.wYCoord ?? -1);
  const candidates = snorlaxCoordCandidates(x, y);
  const isAdjacent = candidates.some(([cx, cy]) =>
    PROXIMITY_COORDS.some(([px, py]) => px === cx && py === cy)
  );
  if (runner) {
    setRunnerValue(runner, isAdjacent ? 1 : 0, { truthy: isAdjacent });
  }
  return isAdjacent;
}
