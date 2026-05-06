import type { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { VRAM } from "@pokecrystal/core/core/memory/vram";

export type OverworldMetatile = {
  collision: readonly number[];
};

export type RenderMetatileOptions = {
  vram?: VRAM | null;
  priority_surface?: InstanceType<typeof gameEngine.Surface> | null;
};

export type OverworldTilesetLike = {
  tilesetName: string;
  metatiles: OverworldMetatile[];
  loaded?: boolean;
  renderMetatile: (
    metatileId: number,
    target: InstanceType<typeof gameEngine.Surface>,
    x: number,
    y: number,
    options?: RenderMetatileOptions,
  ) => void;
  renderPriorityMetatile: (
    metatileId: number,
    target: InstanceType<typeof gameEngine.Surface>,
    x: number,
    y: number,
  ) => void;
  ready?: Promise<void> | { then: (handler: () => void, reject?: (error: unknown) => void) => unknown };
};
