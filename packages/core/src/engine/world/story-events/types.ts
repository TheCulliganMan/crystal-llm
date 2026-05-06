import type { DataLoader as CoreDataLoader } from "@pokecrystal/core/core/data-loader";
import type { GameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import type { ItemSystem } from "@pokecrystal/core/engine/systems/items";

export type StoryGameState = GameState;
export type StoryEventManager = EventManager;

export type ScriptEntry = Record<string, unknown>;
export type ScriptData = ScriptEntry[];

export interface ScriptRunner {
  data_loader: DataLoader;
  stop_execution: boolean;
  is_busy?: boolean;
  state?: string | number;
  item_system: ItemSystem;
  itemSystem?: ItemSystem;
  run: (
    script_name: string,
    options?: { allow_fallthrough?: boolean; allowFallthrough?: boolean },
  ) => void;
  string_buffers: Record<string, string>;
}

export type DataLoader = CoreDataLoader;
