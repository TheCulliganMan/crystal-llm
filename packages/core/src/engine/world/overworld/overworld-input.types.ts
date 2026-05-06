import type { OverworldObject } from "./overworld-object";
import type { OverworldMap } from "./overworld-map";
import type { OverworldTilesetLike } from "./tileset-types";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { GameState } from "@pokecrystal/core/core/state";
import type { BackgroundEvent } from "@pokecrystal/core/core/models/map";
import type { CollisionAttributes } from "./collision-data";
import { Terrain } from "./collision-data";
import { FacingDirection, PlayerState } from "@pokecrystal/core/core/enums/overworld";

export interface OverworldInputContext {
  check_for_npc_interaction(): boolean;
  get_facing_tile_coords(): [number, number];
  _counter_adjusted_tile(tile_x: number, tile_y: number): [number, number];
  _bg_event_at?(tile_x: number, tile_y: number): BackgroundEvent | null;
  _handle_bg_event?(bg_event: BackgroundEvent): boolean;
  map: OverworldMap | null;
  tileset: OverworldTilesetLike | null;
  player_direction: string;
  script_runner: ScriptRunner | null;
  game_state: GameState;
  _play_interaction_sound(): void;
  handle_a_button(): void;
  handle_cut?(metatile_x: number, metatile_y: number): void;
  handle_whirlpool?(metatile_x: number, metatile_y: number): Promise<boolean> | boolean | void;
  handle_waterfall?(metatile_x: number, metatile_y: number): Promise<boolean> | boolean | void;
  handle_surf?(metatile_x: number, metatile_y: number): Promise<boolean> | boolean | void;
  facingDirectionFromString(direction: string): FacingDirection;
  facingDirectionQuadrantIndices(facing: FacingDirection): number[];
  getCollisionStdScript(permission: number): string | null;
  describeCollision(permission: number): CollisionAttributes;
  CUTTABLE_COLLISIONS: Set<number>;
  WHIRLPOOL_COLLISIONS: Set<number>;
  WATERFALL_COLLISIONS: Set<number>;
  Terrain: typeof Terrain;
}
