import { GrowthRate } from "../../core/enums/pokemon";
import type { Pokemon } from "../../core/models";
import { calculateExperience } from "../../engine/experience";
import { BattleBackgroundTilemap, PAL_EXP_FILL } from "./_battle-background";
import { draw_exp_bar } from "./battle-bars";

const EXP_BAR_WIDTH = 8;
const EXP_TILES = { empty: 0x62, full: 0x6a };

const makePokemon = (level: number, experience: number): Pokemon =>
  ({
    level,
    experience,
    species: { growth_rate: GrowthRate.GROWTH_MEDIUM_FAST },
  }) as Pokemon;

const readTiles = (tilemap: BattleBackgroundTilemap, x: number, y: number): number[] =>
  Array.from({ length: EXP_BAR_WIDTH }, (_, offset) => tilemap.getTile(x + offset, y));

const readAttrs = (tilemap: BattleBackgroundTilemap, x: number, y: number): number[] =>
  Array.from({ length: EXP_BAR_WIDTH }, (_, offset) => tilemap.attributes[y][x + offset]);

describe("draw_exp_bar", () => {
  it("matches PlaceExpBar tile ordering for mixed full and partial fill", () => {
    const tilemap = BattleBackgroundTilemap.fromDimensions(20, 18);
    const pokemon = makePokemon(10, 1290);

    draw_exp_bar(tilemap, 5, 6, pokemon, EXP_TILES);

    expect(readTiles(tilemap, 5, 6)).toEqual([0x55, 0x6a, 0x6a, 0x6a, 0x6a, 0x6a, 0x6a, 0x6a]);
    expect(readAttrs(tilemap, 5, 6)).toEqual(Array(EXP_BAR_WIDTH).fill(PAL_EXP_FILL));
  });

  it("clamps fill to empty below current-level experience and full above next-level experience", () => {
    const level = 10;
    const currentLevelExp = calculateExperience(GrowthRate.GROWTH_MEDIUM_FAST, level);
    const nextLevelExp = calculateExperience(GrowthRate.GROWTH_MEDIUM_FAST, level + 1);

    const lowTilemap = BattleBackgroundTilemap.fromDimensions(20, 18);
    draw_exp_bar(lowTilemap, 1, 1, makePokemon(level, currentLevelExp - 999), EXP_TILES);
    expect(readTiles(lowTilemap, 1, 1)).toEqual(Array(EXP_BAR_WIDTH).fill(0x62));

    const highTilemap = BattleBackgroundTilemap.fromDimensions(20, 18);
    draw_exp_bar(highTilemap, 1, 1, makePokemon(level, nextLevelExp + 999), EXP_TILES);
    expect(readTiles(highTilemap, 1, 1)).toEqual(Array(EXP_BAR_WIDTH).fill(0x6a));
  });
});
