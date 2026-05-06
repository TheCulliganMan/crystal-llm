// ASM mapping: pokecrystal_disassembly/engine/overworld/movement.asm (CheckTurning/StepFunction/Collision).
import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { OverworldBase } from "@pokecrystal/core/engine/world/overworld/overworld-base";
import { isPitPermission, isWarpPermission, requiresWarpFacingDown } from "@pokecrystal/core/engine/world/overworld/tile-events";
import { METATILE_WIDTH } from "@pokecrystal/core/engine/world/tile/constants";
import { describeCollision, resolveCollisionValue, Terrain } from "./collision-data";
import {
  CollisionSample,
  getCoordCollision,
  isPermissionPassable,
  isWaterfallCollision,
  isWhirlpoolCollision,
  sampleCollision,
} from "./collision-rules";
import { update_player_sprite } from "@pokecrystal/core/engine/world/special-events/sprites";
import { collectCollisionSamples, isLedgeFace } from "./ledge";
import type { OverworldObject } from "./overworld-object";
import { playOverworldSound } from "./audio-guards";
import { getBooleanFlag, setBooleanFlag } from "./flag-collection";
import type { BlockFeedbackDetails } from "@pokecrystal/core/types/overworld";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { DataLoader, ScriptEntry } from "@pokecrystal/core/core/data-loader";
import type { GameState } from "@pokecrystal/core/core/state";
import type { MapAttributes, WarpEvent } from "@pokecrystal/core/core/models/map";
import type { OverworldTilesetLike } from "./tileset-types";
import type { OverworldMap } from "./overworld-map";
import type { LoggerLike } from "./logger";
import type { NPCStepBlocker } from "./npc-autonomous-controller";

interface PendingLedgeLanding {
  readonly tile_x: number;
  readonly tile_y: number;
  readonly dx: number;
  readonly dy: number;
  readonly direction: string;
}

type OccupantWithLegacyIndex = OverworldObject & { object_index?: number; };
type ScriptRunnerLike = {
  call?: (scriptName: string, parentScript?: string | null) => void;
  run?: (scriptName: string) => void;
};
type StrengthBoulderMover = {
  move_strength_boulder_object?: (objectId: string | number, mapX: number, mapY: number) => void;
};

const _BIKEFLAG_STRENGTH_ACTIVE = 1 << 0;
const _BIKEFLAG_ALWAYS_ON_BIKE = 1 << 1;
const _BIKEFLAG_DOWNHILL = 1 << 2;
const ICE_COLLISION_PERMISSIONS = new Set([
  resolveCollisionValue("ICE"),
  resolveCollisionValue("ICE_2B"),
]);
const OBJECT_FOOTPRINT_COLLISION_PERMISSIONS = new Set([
  resolveCollisionValue("COUNTER"),
  resolveCollisionValue("BOOKSHELF"),
  resolveCollisionValue("PC"),
  resolveCollisionValue("RADIO"),
  resolveCollisionValue("TOWN_MAP"),
  resolveCollisionValue("MART_SHELF"),
  resolveCollisionValue("TV"),
  resolveCollisionValue("COUNTER_98"),
  resolveCollisionValue("WINDOW"),
  resolveCollisionValue("INCENSE_BURNER"),
]);
const CURRENT_COLLISION_HI_NYBBLE = resolveCollisionValue("WATERFALL_RIGHT") & 0xf0;
const CURRENT_COLLISION_DIRECTIONS = ["right", "left", "up", "down"] as const;

const syncBikeFlags = (wram: GameState["wram"] | null | undefined): number => {
  if (!wram) {
    return 0;
  }
  let flags = Number(wram.wBikeFlags ?? 0) & 0xff;
  if (getBooleanFlag(wram.engine_flags, "ENGINE_STRENGTH_ACTIVE")) {
    flags |= _BIKEFLAG_STRENGTH_ACTIVE;
  }
  if (getBooleanFlag(wram.engine_flags, "ENGINE_ALWAYS_ON_BIKE")) {
    flags |= _BIKEFLAG_ALWAYS_ON_BIKE;
  }
  if (getBooleanFlag(wram.engine_flags, "ENGINE_DOWNHILL")) {
    flags |= _BIKEFLAG_DOWNHILL;
  }
  wram.wBikeFlags = flags;
  setBooleanFlag(wram.engine_flags, "ENGINE_STRENGTH_ACTIVE", Boolean(flags & _BIKEFLAG_STRENGTH_ACTIVE));
  setBooleanFlag(wram.engine_flags, "ENGINE_ALWAYS_ON_BIKE", Boolean(flags & _BIKEFLAG_ALWAYS_ON_BIKE));
  setBooleanFlag(wram.engine_flags, "ENGINE_DOWNHILL", Boolean(flags & _BIKEFLAG_DOWNHILL));
  return flags;
};

export abstract class OverworldMovement extends OverworldBase {
  protected readonly TILES_PER_COLLISION = 2;
  protected abstract readonly TURN_FRAMES: number;
  protected abstract readonly STEP_PIXELS: number;
  protected abstract readonly STEP_SPEED_PX: number;

  protected abstract player_x: number;
  protected abstract player_y: number;
  protected abstract prev_player_x: number;
  protected abstract prev_player_y: number;
  protected abstract target_tile_x: number;
  protected abstract target_tile_y: number;
  protected abstract target_px_x: number;
  protected abstract target_px_y: number;

  protected abstract is_moving: boolean;
  protected abstract _turn_frames_remaining: number;
  protected abstract _turning_direction: string | null;
  protected abstract _turn_should_force_step: boolean;
  protected abstract _pending_auto_step: [string, boolean] | null;
  protected abstract _ledge_jump_active: boolean;
  protected abstract _ledge_jump_total_distance_px: number;
  protected abstract _ledge_jump_animation_progress_px: number;
  protected _debug_inputs_enabled?: boolean;
  protected _waterfall_movement_active = false;

  protected abstract player_direction: string;
  public abstract player_state: PlayerState;

  protected abstract _last_step_direction: string | null;
  protected abstract _queued_direction: string | null;
  protected abstract _pending_ledge_landing: PendingLedgeLanding | null;
  protected abstract _last_block_feedback: BlockFeedbackDetails | null;
  protected abstract step_progress_px: number;
  protected abstract _current_step_speed_px: number;
  protected abstract _current_step_distance_px: number;
  protected abstract step_dx_px: number;
  protected abstract step_dy_px: number;
  protected abstract _block_feedback_tracking: boolean;

  protected abstract map: OverworldMap;
  protected abstract tileset: OverworldTilesetLike;
  protected abstract data_loader: DataLoader;
  protected abstract current_map_name: string;
  protected abstract game_state: GameState;
  protected abstract player_object: OverworldObject | null;
  protected abstract audio_engine: AudioEngine | null;
  protected abstract _warp_tile_lookup: Record<string, WarpEvent[]> | Map<string, WarpEvent[]> | null;
  public abstract _logger: LoggerLike | null;

  public abstract player_movement_locked(): boolean;
  protected abstract _npc_occupying_subtile(x: number, y: number): OverworldObject | null;
  protected abstract _prime_player_walk_cycle(): void;
  protected abstract _maybe_spawn_grass_rustle(target: OverworldObject | null, x: number, y: number): void;
  protected abstract _npc_step_blocked?: NPCStepBlocker;
  protected abstract move_object(objectId: string | number, mapX: number, mapY: number): void;

  protected _npc_occupancy_lookup(): (x: number, y: number) => OverworldObject | null {
    return (x: number, y: number) => this._npc_occupying_subtile(x, y);
  }

  public move_player(direction: string, forced: boolean = false): void {
    if (this.player_movement_locked()) {
      return;
    }
    direction = direction.toLowerCase();
    if (this.is_moving) {
      this._queued_direction = direction;
      return;
    }
    if (this._maybe_start_turn(direction, { forced })) {
      return;
    }
    this.player_direction = direction;

    if (
      [PlayerState.NORMAL, PlayerState.BIKE, PlayerState.SKATE].includes(
        this.player_state
      )
    ) {
      this._handle_ground_movement(direction);
    } else if (
      [PlayerState.SURF, PlayerState.SURF_PIKA].includes(this.player_state)
    ) {
      this._handle_surf_movement(direction);
    }
  }

  public movePlayer(direction: string, forced: boolean = false): void {
    this.move_player(direction, forced);
  }

  protected _maybe_start_turn(direction: string, { forced = false }: { forced?: boolean } = {}): boolean {
    if (!["up", "down", "left", "right"].includes(direction)) {
      return false;
    }
    if (this._turn_frames_remaining > 0) {
      if (direction !== this._turning_direction) {
        this.player_direction = direction;
        this._turning_direction = direction;
        this._turn_frames_remaining = this.TURN_FRAMES;
        this._turn_should_force_step = forced || this._turn_should_force_step;
      } else {
        this._turn_should_force_step = forced || this._turn_should_force_step;
      }
      return true;
    }
    if (direction === this.player_direction) {
      return false;
    }
    this.player_direction = direction;
    this._turning_direction = direction;
    this._turn_frames_remaining = this.TURN_FRAMES;
    this._turn_should_force_step = forced;
    this._pending_auto_step = null;
    return true;
  }

  protected _handle_ground_movement(direction: string): void {
    if (this._current_tile_has_downward_warp()) {
      // ASM: player_movement.asm::CheckTile forces downward movement for door/cave/staircase tiles
      // via the current player collision byte, not a full 2x2 footprint sweep.
      direction = "down";
    }

    const [dx, dy] = this._direction_to_vector(direction);
    const stride = this.TILES_PER_COLLISION;
    const target_tile_x = this.player_x + dx * stride;
    const target_tile_y = this.player_y + dy * stride;
    const debugMovement = Boolean(this._debug_inputs_enabled);
    if (debugMovement) {
      console.error(
        `[OverworldMovement] ${direction} from (${this.player_x},${this.player_y}) -> (${target_tile_x},${target_tile_y})`
      );
    }
    const colliding = this.is_colliding(target_tile_x, target_tile_y, direction);
    if (debugMovement) {
      console.error(
        `[OverworldMovement] colliding=${colliding} is_moving=${this.is_moving} queued=${this._queued_direction}`
      );
    }
    if (colliding) {
      if (this._try_ledge_jump(direction)) {
        return;
      }
      this._play_bump_sound();
      return;
    }
    this._last_step_direction = direction;
    const speed_multiplier = this._ground_step_speed_multiplier(direction);
    this._begin_step(dx, dy, target_tile_x, target_tile_y, 1, speed_multiplier);
  }

  protected _ground_step_speed_multiplier(direction: string): number {
    if (![PlayerState.BIKE, PlayerState.SKATE].includes(this.player_state)) {
      return 1;
    }
    const bikeFlags = syncBikeFlags(this.game_state?.wram);
    const downhill = Boolean(bikeFlags & _BIKEFLAG_DOWNHILL);
    if (downhill && String(direction ?? "").toLowerCase() !== "down") {
      // ASM: player_movement.asm::TryStep uses STEP_WALK when downhill and not moving down.
      return 1;
    }
    // ASM: player_movement.asm::TryStep uses STEP_BIKE for bike/skate movement.
    return 2;
  }

  protected _queue_ice_slide_step(): boolean {
    const direction = this._last_step_direction;
    if (!direction || !this._current_tile_is_ice()) {
      return false;
    }
    this._pending_auto_step = [direction, true];
    return true;
  }

  protected _current_tile_is_ice(): boolean {
    if (!this.map || !this.tileset) {
      return false;
    }
    const permission = getCoordCollision(this.map, this.tileset, this.player_x, this.player_y);
    return ICE_COLLISION_PERMISSIONS.has(permission);
  }

  protected _queue_forced_waterfall_step(): boolean {
    const direction = this._forced_water_current_direction();
    if (!direction) {
      return false;
    }
    this.player_direction = direction;
    this._last_step_direction = direction;
    this._pending_auto_step = [direction, true];
    return true;
  }

  protected _forced_water_current_direction(): string | null {
    if (!this.map || !this.tileset) {
      return null;
    }
    const permission = getCoordCollision(this.map, this.tileset, this.player_x, this.player_y);
    if (permission < 0 || (permission & 0xf0) !== CURRENT_COLLISION_HI_NYBBLE) {
      return null;
    }
    return CURRENT_COLLISION_DIRECTIONS[permission & 0x03] ?? null;
  }

  protected _try_ledge_jump(direction: string): boolean {
    if (!this.map) {
      return false;
    }
    const [dx, dy] = this._direction_to_vector(direction);
    const stride = this.TILES_PER_COLLISION;
    const front_tile_x = this.player_x + dx * stride;
    const front_tile_y = this.player_y + dy * stride;

    const samples = collectCollisionSamples(
      this.map,
      this.tileset,
      this.player_x,
      this.player_y,
      stride
    );
    if (!samples.length) {
      return false;
    }

    const npcAtSubtile = this._npc_occupancy_lookup();
    for (const sample of samples) {
      if (npcAtSubtile(sample.tileX, sample.tileY) !== null) {
        return false;
      }
    }

    const facing = FacingDirection.fromString(direction);
    if (
      !isLedgeFace(
        samples,
        facing,
        this.tileset,
        this.player_x,
        this.player_y,
        stride
      )
    ) {
      return false;
    }

    const landing_tile_x = this.player_x + dx * stride * 2;
    const landing_tile_y = this.player_y + dy * stride * 2;

    if (
      this._is_tile_blocked(landing_tile_x, landing_tile_y, direction, {
        allow_wall_override: true,
      })
    ) {
      return false;
    }

    this._last_step_direction = direction;
    this._begin_ledge_jump(dx, dy, front_tile_x, front_tile_y);
    this._play_ledge_jump_sound();
    return true;
  }

  protected _handle_surf_movement(direction: string): void {
    const [dx, dy] = this._direction_to_vector(direction);
    const stride = this.TILES_PER_COLLISION;
    const target_tile_x = this.player_x + dx * stride;
    const target_tile_y = this.player_y + dy * stride;
    if (this.is_colliding(target_tile_x, target_tile_y, direction)) {
      return;
    }
    const permission = getCoordCollision(this.map, this.tileset, target_tile_x, target_tile_y);
    if (permission >= 0 && describeCollision(permission).terrain === Terrain.LAND) {
      this._exit_water();
    }
    this._last_step_direction = direction;
    this._begin_step(dx, dy, target_tile_x, target_tile_y);
  }

  protected _exit_water(): void {
    // ASM: engine/overworld/player_movement.asm::GetOutOfWater + PlayMapMusic.
    this.player_state = PlayerState.NORMAL;
    if (this.game_state?.wram) {
      this.game_state.wram.surfing = false;
    }
    if (!this.game_state) {
      throw new Error("Surf exit requires an active game state.");
    }
    update_player_sprite(this.game_state, { overworld: this });
    const start_map_music = (this as unknown as { start_map_music?: () => void }).start_map_music;
    if (typeof start_map_music === "function") {
      start_map_music.call(this);
    }
  }

  protected _direction_to_vector(direction: string): [number, number] {
    if (direction === "left") {
      return [-1, 0];
    }
    if (direction === "right") {
      return [1, 0];
    }
    if (direction === "up") {
      return [0, -1];
    }
    if (direction === "down") {
      return [0, 1];
    }
    throw new Error(`Unknown direction ${direction}`);
  }

  protected _begin_ledge_jump(
    dx: number,
    dy: number,
    front_tile_x: number,
    front_tile_y: number,
  ): void {
    const stride = this.TILES_PER_COLLISION;
    const landing_tile_x = front_tile_x + dx * stride;
    const landing_tile_y = front_tile_y + dy * stride;
    const direction =
      this._last_step_direction
      ?? (dx === 1 ? "right" : dx === -1 ? "left" : dy === 1 ? "down" : "up");

    this._pending_ledge_landing = {
      tile_x: landing_tile_x,
      tile_y: landing_tile_y,
      dx,
      dy,
      direction,
    };
    this._ledge_jump_active = true;
    this._ledge_jump_total_distance_px = this.STEP_PIXELS * 2;
    this._ledge_jump_animation_progress_px = 0;
    this._begin_step(dx, dy, front_tile_x, front_tile_y);
  }

  protected _begin_step(
    dx: number,
    dy: number,
    target_tile_x: number,
    target_tile_y: number,
    step_tiles: number = 1,
    speed_multiplier: number = 1,
  ): void {
    this.is_moving = true;
    this.step_progress_px = 0.0;
    step_tiles = Math.max(1, step_tiles);
    const normalized_speed_multiplier = Number.isFinite(speed_multiplier) && speed_multiplier > 0
      ? speed_multiplier
      : 1;
    const step_speed_px = this.STEP_SPEED_PX * normalized_speed_multiplier * step_tiles;
    const step_distance_px = this.STEP_PIXELS * step_tiles;
    this._current_step_speed_px = step_speed_px;
    this._current_step_distance_px = step_distance_px;
    this.step_dx_px = dx * step_speed_px;
    this.step_dy_px = dy * step_speed_px;
    this.target_tile_x = target_tile_x;
    this.target_tile_y = target_tile_y;
    const footprint = this.TILES_PER_COLLISION - 1;
    this.target_px_x = OverworldMovement._tileToPixels(target_tile_x - footprint);
    this.target_px_y = OverworldMovement._tileToPixels(target_tile_y - footprint);
    this.prev_player_x = this.player_x;
    this.prev_player_y = this.player_y;
    this._pending_auto_step = null;
    this._prime_player_walk_cycle();
    this._maybe_spawn_grass_rustle(this.player_object, target_tile_x, target_tile_y);
  }

  protected _play_bump_sound(): void {
    playOverworldSound(this.audio_engine, "SFX_BUMP", {
      logger: this._logger ?? undefined,
      context: "bump SFX",
    });
  }

  protected _play_ledge_jump_sound(): void {
    playOverworldSound(this.audio_engine, "SFX_JUMP_OVER_LEDGE", {
      logger: this._logger ?? undefined,
      context: "ledge jump SFX",
    });
  }

  protected _record_block_feedback({
    reason,
    tile,
    permission,
    occupant,
    connection,
  }: {
    reason: string;
    tile?: [number, number];
    permission?: number;
    occupant?: OverworldObject | null;
    connection?: string;
  }): void {
    if (!this._block_feedback_tracking) {
      return;
    }
    const details: BlockFeedbackDetails = { reason };
    if (tile) {
      details["tile"] = [tile[0], tile[1]];
    }
    if (permission !== undefined) {
      const attributes = describeCollision(permission);
      details["permission"] = permission;
      details["terrain"] = attributes.terrain;
      if (attributes.comment) {
        details["comment"] = attributes.comment;
      }
    }
    if (occupant) {
      const occupantInfo = occupant as OccupantWithLegacyIndex;
      const label = occupantInfo.event?.script || occupantInfo.event?.label;
      const descriptor = occupantInfo.objectIndex ?? occupantInfo.object_index ?? null;
      const parts = [descriptor !== null ? String(descriptor) : "npc"];
      if (label) {
        parts.push(String(label));
      }
      details["occupant"] = parts.join(" ");
    }
    if (connection) {
      details["connection"] = connection;
    }
    this._last_block_feedback = details;
  }

  protected _current_tile_has_downward_warp(): boolean {
    if (!this.map || !this.tileset) {
      return false;
    }
    const permission = getCoordCollision(this.map, this.tileset, this.player_x, this.player_y);
    return permission >= 0 && requiresWarpFacingDown(permission);
  }

  public is_colliding(tile_x: number, tile_y: number, direction: string): boolean {
    this._last_block_feedback = null;
    this._block_feedback_tracking = true;
    try {
      return this._is_tile_blocked(tile_x, tile_y, direction);
    } finally {
      this._block_feedback_tracking = false;
    }
  }

  public isColliding(tileX: number, tileY: number, direction: string): boolean {
    return this.is_colliding(tileX, tileY, direction);
  }

  protected _is_tile_blocked(
    tile_x: number,
    tile_y: number,
    direction: string,
    { allow_wall_override = false }: { allow_wall_override?: boolean } = {}
  ): boolean {
    const max_tile_x = this.map.width * METATILE_WIDTH;
    const max_tile_y = this.map.height * METATILE_WIDTH;
    direction = direction.toLowerCase();
    const map_attributes: MapAttributes | undefined = this.data_loader?.map_attributes?.get?.(this.current_map_name);
    if (!map_attributes) {
      throw new Error(`Missing map attributes for ${this.current_map_name}`);
    }
    const connection_dirs = new Set(
      map_attributes.connections.map((conn) => String(conn.direction).toLowerCase())
    );
    const allow_door_exit_south =
      tile_y >= max_tile_y && direction === "down" && this._current_tile_has_downward_warp();
    const allow_west = tile_x < 0 && direction === "left" && connection_dirs.has("west");
    const allow_east =
      tile_x >= max_tile_x && direction === "right" && connection_dirs.has("east");
    const allow_north = tile_y < 0 && direction === "up" && connection_dirs.has("north");
    const allow_south =
      (tile_y >= max_tile_y && direction === "down" && connection_dirs.has("south")) || allow_door_exit_south;
    // ASM: player_movement.asm::CheckLandPerms and ::CheckNPC inspect the
    // direction-facing tile, not the full destination footprint.
    if (
      (tile_x < 0 && !allow_west) ||
      (tile_x >= max_tile_x && !allow_east) ||
      (tile_y < 0 && !allow_north) ||
      (tile_y >= max_tile_y && !allow_south)
    ) {
      this._record_block_feedback({
        reason: "map_edge",
        tile: [tile_x, tile_y],
        connection: allow_west || allow_east || allow_north || allow_south ? "allowed" : "blocked",
      });
      return true;
    }

    const allow_x_oob = allow_west || allow_east;
    const allow_y_oob = allow_north || allow_south;

    const facing = FacingDirection.fromString(this.player_direction);
    const subtile_x = tile_x;
    const subtile_y = tile_y;

    if (subtile_x < 0 || subtile_x >= max_tile_x) {
      if (!allow_x_oob) {
        this._record_block_feedback({
          reason: "map_edge",
          tile: [subtile_x, subtile_y],
        });
        return true;
      }
      return false;
    }
    if (subtile_y < 0 || subtile_y >= max_tile_y) {
      if (!allow_y_oob) {
        this._record_block_feedback({
          reason: "map_edge",
          tile: [subtile_x, subtile_y],
        });
        return true;
      }
      return false;
    }

    let npcAtSubtile = this._npc_occupancy_lookup();
    const occupant = npcAtSubtile(subtile_x, subtile_y);
    if (occupant !== null) {
      if (this._push_strength_boulder(occupant, direction)) {
        npcAtSubtile = (x: number, y: number) => this._npc_occupying_subtile(x, y);
        const refreshed = npcAtSubtile(subtile_x, subtile_y);
        if (refreshed !== null) {
          this._record_block_feedback({
            reason: "npc",
            tile: [subtile_x, subtile_y],
            occupant: refreshed,
          });
          return true;
        }
        return false;
      }
      this._record_block_feedback({
        reason: "npc",
        tile: [subtile_x, subtile_y],
        occupant,
      });
      return true;
    }

    // ASM object collision uses the live/last object footprint occupancy map, not
    // just the destination anchor subtile. This keeps moving 2x2 actors solid.
    const stride = this.TILES_PER_COLLISION;
    for (let dx = 0; dx < stride; dx += 1) {
      for (let dy = 0; dy < stride; dy += 1) {
        const footprintX = subtile_x - dx;
        const footprintY = subtile_y - dy;
        const footprintOccupant = npcAtSubtile(footprintX, footprintY);
        if (footprintOccupant === null) {
          continue;
        }
        this._record_block_feedback({
          reason: "npc",
          tile: [footprintX, footprintY],
          occupant: footprintOccupant,
        });
        return true;
      }
    }

    const objectFootprintSample = this._object_footprint_collision_sample(
      subtile_x,
      subtile_y
    );
    if (
      objectFootprintSample &&
      this._permission_blocks_tile(objectFootprintSample.permission, facing, false)
    ) {
      this._record_block_feedback({
        reason: "terrain",
        tile: [objectFootprintSample.tileX, objectFootprintSample.tileY],
        permission: objectFootprintSample.permission,
      });
      return true;
    }

    let collisionSample: CollisionSample | null = null;
    let permission = -1;
    try {
      collisionSample = sampleCollision(this.map, this.tileset, subtile_x, subtile_y);
      permission = collisionSample.permission;
    } catch {
      permission = -1;
    }

    if (collisionSample && this._surf_field_move_metatile_blocks(collisionSample, direction)) {
      this._record_block_feedback({
        reason: "terrain",
        tile: [subtile_x, subtile_y],
        permission,
      });
      return true;
    }

    if (this._permission_blocks_tile(permission, facing, allow_wall_override)) {
      this._record_block_feedback({
        reason: "terrain",
        tile: [subtile_x, subtile_y],
        permission,
      });
      return true;
    }

    return false;
  }

  protected _object_footprint_collision_sample(
    subtile_x: number,
    subtile_y: number
  ): CollisionSample | null {
    const stride = this.TILES_PER_COLLISION;
    const max_tile_x = this.map.width * METATILE_WIDTH;
    const max_tile_y = this.map.height * METATILE_WIDTH;
    for (let dx = 0; dx < stride; dx += 1) {
      for (let dy = 0; dy < stride; dy += 1) {
        const footprintX = subtile_x - dx;
        const footprintY = subtile_y - dy;
        if (
          footprintX < 0 ||
          footprintX >= max_tile_x ||
          footprintY < 0 ||
          footprintY >= max_tile_y
        ) {
          continue;
        }
        let sample: CollisionSample;
        try {
          sample = sampleCollision(this.map, this.tileset, footprintX, footprintY);
        } catch {
          continue;
        }
        if (OBJECT_FOOTPRINT_COLLISION_PERMISSIONS.has(sample.permission)) {
          return sample;
        }
      }
    }
    return null;
  }

  protected _surf_field_move_metatile_blocks(sample: CollisionSample, direction: string): boolean {
    if (![PlayerState.SURF, PlayerState.SURF_PIKA].includes(this.player_state)) {
      return false;
    }
    if (isWhirlpoolCollision(sample.permission)) {
      return true;
    }
    return isWaterfallCollision(sample.permission) && !this._waterfall_collision_allowed(direction);
  }

  protected _waterfall_collision_allowed(direction: string | FacingDirection): boolean {
    return this._waterfall_movement_active ||
      direction === FacingDirection.DOWN ||
      String(direction ?? "").toLowerCase() === "down";
  }

  protected _permission_blocks_tile(
    permission: number,
    facing: FacingDirection,
    allow_wall_override: boolean
  ): boolean {
    if (isWarpPermission(permission)) {
      return false;
    }
    if (
      isPermissionPassable(permission, facing, this.player_state, {
        allowWaterfall: this._waterfall_collision_allowed(facing),
      })
    ) {
      return false;
    }
    if (allow_wall_override) {
      if (describeCollision(permission).terrain === Terrain.WALL) {
        return false;
      }
    }
    return true;
  }

  protected _is_strength_boulder(npc: OverworldObject): boolean {
    const movement = npc.event?.spritemovedata ?? "";
    return Boolean(movement && String(movement).toUpperCase().includes("STRENGTH_BOULDER"));
  }

  protected _push_strength_boulder(npc: OverworldObject, direction: string): boolean {
    const bikeFlags = syncBikeFlags(this.game_state?.wram);
    if (!(bikeFlags & _BIKEFLAG_STRENGTH_ACTIVE)) {
      return false;
    }
    if (!this._is_strength_boulder(npc)) {
      return false;
    }
    let dx: number;
    let dy: number;
    try {
      [dx, dy] = this._direction_to_vector(direction);
    } catch {
      return false;
    }

    const stride = METATILE_WIDTH / 2;
    const footprint = this.TILES_PER_COLLISION - 1;
    const target_map_x = Math.floor((npc.x - footprint) / stride) + dx;
    const target_map_y = Math.floor((npc.y - footprint) / stride) + dy;
    if (target_map_x < 0 || target_map_y < 0) {
      return false;
    }
    if (
      target_map_x >= this.map.width * this.TILES_PER_COLLISION ||
      target_map_y >= this.map.height * this.TILES_PER_COLLISION
    ) {
      return false;
    }

    const target_tile_x = npc.x + dx * this.TILES_PER_COLLISION;
    const target_tile_y = npc.y + dy * this.TILES_PER_COLLISION;
    if (
      this._npc_step_blocked?.(npc, direction, target_tile_x, target_tile_y, {
        is_player_target: true,
      })
    ) {
      return false;
    }

    this._play_strength_sound();
    if (typeof this.move_object !== "function") {
      throw new Error("Overworld movement requires move_object for Strength boulders.");
    }
    this._move_strength_boulder_object(npc, target_map_x, target_map_y);
    // ASM: CopyCoordsTileToLastCoordsTile after movement completion.
    npc.prevX = npc.x;
    npc.prevY = npc.y;
    (npc as { prev_x?: number; prev_y?: number }).prev_x = npc.x;
    (npc as { prev_x?: number; prev_y?: number }).prev_y = npc.y;
    npc.direction = direction;
    npc.walking = false;
    npc.jumping = false;
    this._handle_strength_boulder_landing(npc, target_map_x, target_map_y, target_tile_x, target_tile_y);
    return true;
  }

  protected _move_strength_boulder_object(npc: OverworldObject, mapX: number, mapY: number): void {
    const mover = (this as StrengthBoulderMover).move_strength_boulder_object;
    if (typeof mover === "function") {
      mover.call(this, npc.objectIndex, mapX, mapY);
      return;
    }
    if (typeof this.move_object !== "function") {
      throw new Error("Overworld movement requires move_object for Strength boulders.");
    }
    this.move_object(npc.objectIndex, mapX, mapY);
  }

  protected _handle_strength_boulder_landing(
    npc: OverworldObject,
    mapX: number,
    mapY: number,
    tileX: number,
    tileY: number
  ): void {
    const warp = this._strength_boulder_landing_warp(mapX, mapY, tileX, tileY);
    if (!warp) {
      return;
    }
    const stonetable = this._strength_boulder_stonetable_entry(npc, warp.index);
    if (!stonetable) {
      return;
    }

    const runner = (this as { script_runner?: ScriptRunnerLike | null }).script_runner ?? null;
    if (typeof runner?.call === "function") {
      runner.call(stonetable.scriptName, stonetable.parentScript);
      return;
    }

    this._apply_strength_boulder_stonetable_script(stonetable.scriptName, stonetable.parentScript);
  }

  private _strength_boulder_landing_warp(
    mapX: number,
    mapY: number,
    tileX: number,
    tileY: number
  ): WarpEvent | null {
    let permission = -1;
    try {
      permission = sampleCollision(this.map, this.tileset, tileX, tileY).permission;
    } catch {
      permission = -1;
    }
    if (!isPitPermission(permission)) {
      return null;
    }

    const events = this.data_loader?.map_events?.get?.(this.current_map_name);
    const warps = events?.warps ?? [];
    return warps.find((warp) => warp.x === mapX && warp.y === mapY) ?? null;
  }

  private _strength_boulder_stonetable_entry(
    npc: OverworldObject,
    warpIndex: number
  ): { scriptName: string; parentScript: string } | null {
    const callbacks = this.data_loader?.map_callbacks?.get?.(this.current_map_name) ?? [];
    if (!callbacks.length) {
      return null;
    }

    const identifiers = this._strength_boulder_identifiers(npc);
    for (const [callbackType, parentScript] of callbacks) {
      if (String(callbackType).toUpperCase() !== "MAPCALLBACK_CMDQUEUE") {
        continue;
      }
      const script = this.data_loader.get_script?.(parentScript);
      if (!script) {
        continue;
      }
      for (const entry of script) {
        if (String(entry.command ?? "").toLowerCase() !== "stonetable") {
          continue;
        }
        const args = Array.isArray(entry.args) ? entry.args : [];
        const tableWarpIndex = Number(args[0]);
        const tableObject = String(args[1] ?? "").trim().toUpperCase();
        const targetScript = String(args[2] ?? "").trim();
        if (tableWarpIndex === warpIndex && identifiers.has(tableObject) && targetScript) {
          return { scriptName: targetScript, parentScript };
        }
      }
    }
    return null;
  }

  private _strength_boulder_identifiers(npc: OverworldObject): Set<string> {
    const mapKey = String(this.current_map_name ?? "").replace(/\s+/g, "").toUpperCase();
    const identifiers = new Set<string>();
    const add = (value: unknown): void => {
      const normalized = String(value ?? "").trim().toUpperCase();
      if (normalized) {
        identifiers.add(normalized);
      }
    };

    add(npc.constantId);
    add(npc.objectId);
    add(npc.event?.object_identifier);
    add(npc.event?.script);
    add(String(npc.objectIndex ?? ""));
    add(`${mapKey}_${npc.spriteId}${npc.objectIndex}`);
    add(`${mapKey}_${npc.baseSpriteId}${npc.objectIndex}`);
    return identifiers;
  }

  private _apply_strength_boulder_stonetable_script(scriptName: string, parentScript: string): void {
    const script = this.data_loader?.get_script?.(scriptName, parentScript);
    if (!script) {
      return;
    }
    for (const entry of script as ScriptEntry[]) {
      const command = String(entry.command ?? "").toLowerCase();
      const args = Array.isArray(entry.args) ? entry.args : [];
      if (command === "disappear") {
        const objectId = args[0];
        const removeObject = (this as { remove_object?: (objectId: string | number) => void }).remove_object;
        if (typeof removeObject === "function" && (typeof objectId === "string" || typeof objectId === "number")) {
          removeObject.call(this, objectId);
        }
      } else if (command === "clearevent") {
        const eventFlag = String(args[0] ?? "").trim();
        if (eventFlag) {
          setBooleanFlag(this.game_state?.wram?.event_flags, eventFlag, false);
        }
      }
    }
  }

  protected _play_strength_sound(): void {
    playOverworldSound(this.audio_engine, "SFX_STRENGTH", {
      logger: this._logger ?? undefined,
      context: "strength SFX",
    });
  }
}
