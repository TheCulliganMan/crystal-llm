import { GameState } from "../../../core/state";
import { getMapMetadataByName } from "../maps";
import { METATILE_WIDTH } from "../tile";
import { scaleTileCoord } from '@pokecrystal/core/engine/world/overworld/tile-coords';
import { ensureScriptMemory, runnerValue, setRunnerValue, ScriptRunner } from "./utils";
import { MagnetTrainAnimator } from "@pokecrystal/core/ui/screens/magnet-train-animation";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import type { EventManager } from "@pokecrystal/core/engine/events/events";

const parseNumericToken = (token: string): number => {
  const normalized = String(token ?? "").trim();
  if (!normalized) {
    throw new Error("Cannot parse an empty numeric token.");
  }
  if (normalized.startsWith("$")) {
    return Number.parseInt(normalized.slice(1), 16);
  }
  if (normalized.toLowerCase().startsWith("0x")) {
    return Number.parseInt(normalized, 16);
  }
  return Number.parseInt(normalized, 0);
};

const magnetTrainHeadingToGoldenrod = (runner?: ScriptRunner | null): boolean => {
  const value = runnerValue(runner ?? null, "0");
  const text = String(value ?? "").trim();
  if (!text) {
    return false;
  }
  const normalized = text.toUpperCase();
  if (["TRUE", "T", "YES", "Y"].includes(normalized)) {
    return true;
  }
  if (["FALSE", "F", "NO", "N"].includes(normalized)) {
    return false;
  }
  try {
    return parseNumericToken(text) !== 0;
  } catch (error) {
    return false;
  }
};

type Overworld = {
  ui?: ScreenUI | null;
  audio_engine?: unknown;
  restartMapMusic?: () => void;
  restart_map_music?: () => void;
  load_map?: (mapName: string) => void;
  player_x?: number;
  player_y?: number;
  prev_player_x?: number;
  prev_player_y?: number;
  target_tile_x?: number;
  target_tile_y?: number;
  is_moving?: boolean;
  step_progress_px?: number;
  step_dx_px?: number;
  step_dy_px?: number;
  _queued_direction?: string | null;
  _sync_player_state?: () => void;
  clear_pending_white_fade?: () => void;
  _active_warp_tile?: [string, number, number] | null;
  TILES_PER_COLLISION?: number;
};

export function magnet_train(
  game_state: GameState,
  {
    runner,
    overworld,
    event_manager,
  }: { runner?: ScriptRunner | null; overworld?: Overworld | null; event_manager?: EventManager } = {}
): boolean | Promise<boolean> {
  // ASM: engine/events/magnet_train.asm::MagnetTrain
  void event_manager;

  const scriptMemory = ensureScriptMemory(game_state);
  const specials = (scriptMemory.specials ??= {}) as Record<string, unknown>;
  const state = (specials.magnet_train ??= {}) as Record<string, unknown>;
  state.count = Number(state.count ?? 0) + 1;

  const directionToken = runnerValue(runner ?? null, "0");
  const directionToGoldenrod = magnetTrainHeadingToGoldenrod(runner ?? null);
  state.direction_token = directionToken;

  const destinationMap = directionToGoldenrod
    ? "GoldenrodMagnetTrainStation"
    : "SaffronMagnetTrainStation";
  const sceneName = directionToGoldenrod
    ? "SCENE_GOLDENRODMAGNETTRAINSTATION_ARRIVE_FROM_SAFFRON"
    : "SCENE_SAFFRONMAGNETTRAINSTATION_ARRIVE_FROM_GOLDENROD";
  state.destination = destinationMap;
  state.scene = sceneName;

  if (!overworld) {
    throw new Error("MagnetTrain requires an active overworld.");
  }

  const metadata = getMapMetadataByName(destinationMap);
  if (!metadata) {
    throw new Error(`Unknown magnet train destination '${destinationMap}'.`);
  }

  const finalizeTransition = (): boolean => {
    if (runner && typeof runner._set_map_scene === "function") {
      runner._set_map_scene(destinationMap, sceneName);
    }
    game_state.wram.map_scenes[destinationMap] = sceneName;
    game_state.wram.scene_name = sceneName;

    const stride = overworld.TILES_PER_COLLISION ?? 2;
    const footprint = Math.max(0, stride - 1);
    const arrivalTile: [number, number] = [11, 6];
    const rawX = arrivalTile[0];
    const rawY = arrivalTile[1];
    const maxTileX = metadata.width * METATILE_WIDTH - 1;
    const maxTileY = metadata.height * METATILE_WIDTH - 1;
    const destX = Math.max(footprint, Math.min(maxTileX, scaleTileCoord(rawX, stride)));
    const destY = Math.max(footprint, Math.min(maxTileY, scaleTileCoord(rawY, stride)));

    const wram = game_state.wram;
    const prevGroup = wram.wMapGroup;
    const prevNumber = wram.wMapNumber;
    wram.wPrevMapGroup = prevGroup;
    wram.wPrevMapNumber = prevNumber;
    wram.wMapGroup = metadata.groupId;
    wram.wMapNumber = metadata.mapId;
    wram.current_map_group = metadata.groupId;
    wram.current_map_id = metadata.mapId;
    wram.wPrevWarp = 0;
    wram.wNextWarp = 0;
    wram.wLastSpawnMapGroup = metadata.groupId;
    wram.wLastSpawnMapNumber = metadata.mapId;
    game_state.sram.last_spawn_map_group = metadata.groupId;
    game_state.sram.last_spawn_map_number = metadata.mapId;
    wram.wXCoord = rawX;
    wram.wYCoord = rawY;
    wram.player_x = Math.floor(rawX / stride);
    wram.player_y = Math.floor(rawY / stride);
    wram.player_subtile_x = rawX % stride;
    wram.player_subtile_y = rawY % stride;
    wram.wDefaultSpawnpoint = 0;

    overworld.player_x = destX;
    overworld.player_y = destY;
    overworld.prev_player_x = destX;
    overworld.prev_player_y = destY;
    overworld.target_tile_x = destX;
    overworld.target_tile_y = destY;
    overworld.is_moving = false;
    overworld.step_progress_px = 0;
    overworld.step_dx_px = 0;
    overworld.step_dy_px = 0;
    if ("_queued_direction" in overworld) {
      overworld._queued_direction = null;
    }

    if (typeof overworld.load_map !== "function") {
      throw new Error("MagnetTrain requires an overworld that can load maps.");
    }
    overworld.load_map(metadata.name);

    overworld._sync_player_state?.();
    overworld.clear_pending_white_fade?.();
    if ("_active_warp_tile" in overworld) {
      overworld._active_warp_tile = [metadata.name, destX, destY];
    }
    const restartMusic = overworld.restart_map_music ?? overworld.restartMapMusic;
    if (typeof restartMusic === "function") {
      restartMusic.call(overworld);
    }

    return Boolean(setRunnerValue(runner ?? null, directionToken, { truthy: true }));
  };

  const animator = new MagnetTrainAnimator();
  const animatorOverworld = overworld as unknown as Parameters<MagnetTrainAnimator["play"]>[1];
  const maybePromise = animator.playAsync(directionToGoldenrod, animatorOverworld);
  if (maybePromise && typeof maybePromise.then === "function") {
    return maybePromise.then(() => finalizeTransition());
  }
  animator.play(directionToGoldenrod, animatorOverworld);
  const valueResult = finalizeTransition();
  return valueResult;
}
