import type { EngineEvent, Rect, Surface } from "@pokecrystal/core/ui/game-engine";

declare global {
  namespace gameEngine {
    type Surface = Surface;
    type Rect = Rect;
    type Event = EngineEvent;
  }
}

export {};
