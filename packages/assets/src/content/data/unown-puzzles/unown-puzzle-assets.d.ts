import { Surface } from '@pokecrystal/core/ui/surface';
export declare const PUZZLE_IDS: readonly ["KABUTO", "OMANYTE", "AERODACTYL", "HOOH"];
export declare class PuzzleCoordinate {
    readonly tileX: number;
    readonly tileY: number;
    readonly oamX: number;
    readonly oamY: number;
    readonly vacantTile: number;
    constructor(tileX: number, tileY: number, oamX: number, oamY: number, vacantTile: number);
}
export declare class OamTemplate {
    readonly y: number;
    readonly x: number;
    readonly tileOffset: number;
    readonly attributes: number;
    constructor(y: number, x: number, tileOffset: number, attributes: number);
    get signedX(): number;
    get signedY(): number;
}
export type UnownPuzzleAssetLoader = (path: string) => Uint8Array;
export declare function setUnownPuzzleAssetLoader(loader: UnownPuzzleAssetLoader): void;
export declare function applyPieceBorders(tiles: Uint8Array[], borderTiles: Uint8Array[]): void;
export declare function loadPuzzleRawBytes(puzzleId: string): Uint8Array;
export declare function loadBorderTiles(): Uint8Array[];
export declare function loadCursorTiles(): Uint8Array[];
export declare function loadStartCancelTiles(): Uint8Array[];
export declare function convertPuzzleTiles(puzzleId: string): Uint8Array[];
export declare function computeCornerTiles(cursorTile?: number): number[];
export declare function loadCoordinates(): [PuzzleCoordinate[], Record<string, OamTemplate[]>];
export declare function loadLayouts(): [number[][], Array<[number, number]>];
export declare function buildTileSurfaces(puzzleId: string, palette?: Array<[number, number, number, number]>): Record<number, Surface>;
