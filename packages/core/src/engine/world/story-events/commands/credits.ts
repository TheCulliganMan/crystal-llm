import { GameState } from "@pokecrystal/core/core/state";
import { EventManager, StartCreditsEvent } from "@pokecrystal/core/engine/events/events";
import { Command, type OverworldContext } from "./base";
import { applySpawn, Spawn } from "@pokecrystal/core/engine/world/maps";
import {
  warp_to_spawn_point,
  type Overworld as SpawnOverworld,
} from "@pokecrystal/core/engine/world/special-events/map";

const SPAWN_AFTER_CHAMPION_LANCE = 1;
const SPAWN_AFTER_CHAMPION_RED = 2;

const coerceHallOfFameCount = (value: unknown): number | null => {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return null;
  }
  return value & 0xff;
};

export class CreditsCommand extends Command {
  private allowSkip?: boolean;

  constructor(allowSkip?: boolean) {
    super();
    this.allowSkip = allowSkip;
  }

  execute(gameState: GameState, eventManager: EventManager, overworld: OverworldContext): void {
    const runner = this.runner;
    const allowSkip = this.resolveAllowSkip(gameState);
    const onComplete = (): void => {
      applyPostCreditsSpawn(gameState, overworld);
    };

    if (!hasCreditsListener(eventManager)) {
      onComplete();
      return;
    }

    const schedule = (callback: () => void): boolean => {
      eventManager.dispatch(
        new StartCreditsEvent({
          allow_skip: allowSkip,
          on_complete: callback,
          return_state: "title",
        }),
      );
      return true;
    };

    const queueTask =
      runner?._queue_overworld_task ?? runner?._queueOverworldTask;
    if (typeof queueTask === "function") {
      queueTask.call(runner, schedule);
      return;
    }

    schedule(onComplete);
  }

  private resolveAllowSkip(gameState: GameState): boolean {
    if (this.allowSkip !== undefined) {
      return Boolean(this.allowSkip);
    }

    const flags = gameState.wram.engine_flags;
    if (flags && flags["STATUSFLAGS_HALL_OF_FAME_F"]) {
      return true;
    }

    const hofEntries = gameState.sram.hall_of_fame;
    if (Array.isArray(hofEntries) && hofEntries.length > 0) {
      return true;
    }

    const count = coerceHallOfFameCount(gameState.wram?.wHallOfFameCount);
    return count !== null && count > 0;
  }
}

const hasCreditsListener = (eventManager: EventManager | null): boolean => {
  if (!eventManager) {
    return false;
  }
  return eventManager.hasListener("start_credits");
};

const applyPostCreditsSpawn = (gameState: GameState, overworld: OverworldContext): void => {
  const spawnAfterChampion = Number(gameState.wram.wSpawnAfterChampion ?? 0);
  if (spawnAfterChampion === 0) {
    return;
  }

  if (spawnAfterChampion === SPAWN_AFTER_CHAMPION_LANCE) {
    gameState.wram.wDefaultSpawnpoint = Spawn.NEW_BARK;
  } else if (spawnAfterChampion === SPAWN_AFTER_CHAMPION_RED) {
    gameState.wram.wDefaultSpawnpoint = Spawn.MT_SILVER;
  }
  gameState.wram.wSpawnAfterChampion = 0;

  const spawn = gameState.wram.wDefaultSpawnpoint;
  if (spawn === Spawn.NEW_BARK || spawn === Spawn.MT_SILVER) {
    applySpawn(gameState, spawn);
    const spawnPoint =
      overworld && typeof (overworld as { load_map?: unknown }).load_map === "function"
        ? overworld as SpawnOverworld
        : undefined;
    if (spawnPoint) {
      warp_to_spawn_point(gameState, { overworld: spawnPoint });
    }
  }
};
