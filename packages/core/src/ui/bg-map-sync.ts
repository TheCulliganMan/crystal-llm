import { GameState } from "@pokecrystal/core/core/state";
import { TilemapSurface } from "./tilemap-surface";

interface BGMapSyncState {
  bg_map_sync?: { is_busy: boolean };
  write_bg_map_with_wait?: (
    name: string,
    width: number,
    height: number,
    tiles: number[],
    attrs: number[],
    options?: { origin_x?: number; origin_y?: number }
  ) => void;
}

interface BGMapWriteRequest {
  name: string;
  width: number;
  height: number;
  tiles: number[];
  attrs: number[];
  origin_x: number;
  origin_y: number;
}

export class BGMapWriter {
  private pending: BGMapWriteRequest | null = null;
  private lastCommitted: BGMapWriteRequest | null = null;

  constructor(private readonly gameState: GameState, private readonly mapName: string) {}

  request(
    tilemap: TilemapSurface,
    { originX = 0, originY = 0, name }: { originX?: number; originY?: number; name?: string } = {}
  ): void {
    const [tiles, attrs] = tilemap.flatten();
    const request: BGMapWriteRequest = {
      name: name ?? this.mapName,
      width: tilemap.width,
      height: tilemap.height,
      tiles,
      attrs,
      origin_x: originX,
      origin_y: originY,
    };
    if (this.lastCommitted && this._equals(request, this.lastCommitted)) {
      this.pending = null;
      return;
    }
    this.pending = request;
    this.flushIfReady();
  }

  private flushIfReady(): void {
    if (!this.pending) {
      return;
    }
    const state = this.gameState as GameState & BGMapSyncState;
    const busy = state.bg_map_sync?.is_busy ?? false;
    if (busy) {
      return;
    }
    if (typeof state.write_bg_map_with_wait !== "function") {
      throw new Error("GameState is missing write_bg_map_with_wait for BGMapWriter");
    }
    const request = this.pending;
    state.write_bg_map_with_wait(
      request.name,
      request.width,
      request.height,
      request.tiles,
      request.attrs,
      { origin_x: request.origin_x, origin_y: request.origin_y }
    );
    this.lastCommitted = request;
    this.pending = null;
  }

  private _equals(a: BGMapWriteRequest, b: BGMapWriteRequest): boolean {
    if (
      a.name !== b.name ||
      a.width !== b.width ||
      a.height !== b.height ||
      a.origin_x !== b.origin_x ||
      a.origin_y !== b.origin_y
    ) {
      return false;
    }
    if (a.tiles.length !== b.tiles.length || a.attrs.length !== b.attrs.length) {
      return false;
    }
    for (let i = 0; i < a.tiles.length; i++) {
      if (a.tiles[i] !== b.tiles[i]) {
        return false;
      }
    }
    for (let i = 0; i < a.attrs.length; i++) {
      if (a.attrs[i] !== b.attrs[i]) {
        return false;
      }
    }
    return true;
  }
}
