import type { OverworldMap } from "@pokecrystal/core/engine/world/overworld/overworld-map";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";

export type Overworld = import("@pokecrystal/core/engine/world/overworld/overworld").OverworldEngine;

export type BlockFeedbackDetails = {
    reason: string;
    tile?: [number, number];
    permission?: number;
    terrain?: unknown;
    comment?: string;
    occupant?: string;
    connection?: string;
};

export type RemoteOverworldPlayer = {
    userId: string;
    playerName: string;
    entityType: "player" | "ai";
    mapName: string;
    tileX: number;
    tileY: number;
    direction: "up" | "down" | "left" | "right";
    updatedAtMs: number;
};

export interface OverworldWithNpcInteraction {
    get_facing_tile_coords(): [number, number];
    _play_interaction_sound(): void;
    _counter_adjusted_tile(tile_x: number, tile_y: number): [number, number];
    map: OverworldMap | null;
    script_runner: ScriptRunner & {
        is_busy?: boolean;
        allow_event_flag_refresh?: boolean;
        run: (
            script: string,
            options?: { allow_fallthrough?: boolean; allowFallthrough?: boolean },
        ) => void;
    } | null;
    _npc_occupying_subtile?: (x: number, y: number) => OverworldObject | null;
}
