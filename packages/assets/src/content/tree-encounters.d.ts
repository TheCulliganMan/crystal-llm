export interface TreeMonEntry {
    weight: number;
    species: string;
    level: number;
}
export interface TreeMonSet {
    common: TreeMonEntry[];
    rare: TreeMonEntry[];
}
export declare const TREEMON_SLEEP_TURNS = 7;
export declare function getTreeSetForMap(mapConstant: string): TreeMonSet | null;
export declare function getRockSetForMap(mapConstant: string): TreeMonSet | null;
export declare function computeTreeScore(tileX: number, tileY: number, playerId: number): number;
export declare function isAsleepTreeMon(speciesId: string, timeOfDay: string | null | undefined): boolean;
export declare function chooseTreeEncounter(treeSet: TreeMonSet, score: number, randrange: (maxExclusive: number) => number): [string, number] | null;
export declare function chooseRockSmashEncounter(treeSet: TreeMonSet, randrange: (maxExclusive: number) => number): [string, number] | null;
