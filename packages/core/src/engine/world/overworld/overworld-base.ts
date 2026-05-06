import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import type { GameEngineEvent } from "@pokecrystal/core/ui/game-engine";
import { mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { METATILE_SIZE, METATILE_WIDTH, TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";

type SectionHandle = (() => void) | ({ end?: () => void; close?: () => void }) | null | undefined;
type SectionCallable = (this: unknown, name: string) => SectionHandle;
type Profiler = { section?: SectionCallable };
type ProfilerOwner = { profiler?: Profiler };

export abstract class OverworldBase {
    protected static _profileSection(owner: ProfilerOwner | null | undefined, name: string): () => void {
        const profiler = owner?.profiler;
        const section = profiler?.section;
        if (typeof section !== "function") {
            return () => {};
        }
        const handle = section.call(profiler, name);
        if (!handle) {
            return () => {};
        }
        if (typeof handle === "function") {
            return handle;
        }
        const end = handle.end;
        if (typeof end === "function") {
            return () => end();
        }
        const close = handle.close;
        if (typeof close === "function") {
            return () => close();
        }
        return () => {};
    }

    protected _describe_input_event(event: GameEngineEvent | null | undefined): string {
        if (event?.type === gameEngine.KEYDOWN) {
            const key = event?.key ?? "";
            return `keydown:${String(key)}`;
        }
        if (event?.type === gameEngine.KEYUP) {
            const key = event?.key ?? "";
            return `keyup:${String(key)}`;
        }
        return String(event?.type ?? "unknown");
    }

    protected static _tileToPixels(tileCoordinate: number): number {
        const metatile = Math.floor(tileCoordinate / METATILE_WIDTH);
        const offset = tileCoordinate % METATILE_WIDTH;
        return metatile * METATILE_SIZE + offset * TILE_SIZE;
    }

    protected static _tileFromComponents(metatile: number, subtile: number): number {
        return metatile * METATILE_WIDTH + subtile;
    }

    protected static _tile_from_components(metatile: number, subtile: number): number {
        return OverworldBase._tileFromComponents(metatile, subtile);
    }

    protected _tile_from_components(metatile: number, subtile: number): number {
        return OverworldBase._tileFromComponents(metatile, subtile);
    }

    public static _normalizeMapKey(mapName: string): string {
        const result: string[] = [];
        let previous = "";
        for (let i = 0; i < mapName.length; i++) {
            const char = mapName[i];
            if (char === "_") {
                if (result.length > 0 && result[result.length - 1] !== "_") {
                    result.push("_");
                }
                previous = char;
                continue;
            }
            if (char >= 'A' && char <= 'Z') {
                if (i > 0 && result.length > 0 && result[result.length - 1] !== "_") {
                    if ((previous >= 'a' && previous <= 'z') || (previous >= '0' && previous <= '9')) {
                        if (!((previous >= '0' && previous <= '9') && char === "F")) {
                            result.push("_");
                        }
                    }
                }
                result.push(char);
            } else if (char >= '0' && char <= '9') {
                if (i > 0 && result.length > 0 && result[result.length - 1] !== "_") {
                    if (previous >= 'a' && previous <= 'z') {
                        result.push("_");
                    }
                }
                result.push(char);
            } else {
                if (i > 0 && result.length > 0 && result[result.length - 1] !== "_") {
                    if (previous >= '0' && previous <= '9') {
                        result.push("_");
                    }
                }
                result.push(char.toUpperCase());
            }
            previous = char;
        }
        return result.join("").replace(/_+/g, "_").replace(/^_|_$/g, "");
    }

    protected static _normaliseMapKey(mapName: string): string {
        return OverworldBase._normalizeMapKey(mapName);
    }

    protected _normalise_map_key(mapName: string): string {
        return OverworldBase._normalizeMapKey(mapName);
    }

    protected static _npcDrawOffsets(spriteWidth: number, spriteHeight: number): [number, number] {
        let offsetX: number;
        if (spriteWidth > METATILE_SIZE) {
            offsetX = -Math.floor((spriteWidth - METATILE_SIZE) / 2);
        } else {
            offsetX = 0;
        }

        let offsetY: number;
        if (spriteHeight > METATILE_SIZE) {
            offsetY = -(spriteHeight - METATILE_SIZE);
        } else {
            offsetY = 0;
        }

        return [offsetX, offsetY];
    }

    protected _direction_from_key(key: string | number | null | undefined): string | null {
        return mapKeyToDirection(key ?? null);
    }
}

export const normalizeMapKey = (mapName: string): string => {
    return OverworldBase._normalizeMapKey(mapName);
};
