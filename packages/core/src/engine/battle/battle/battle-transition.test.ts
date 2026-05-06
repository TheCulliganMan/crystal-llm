import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BattleTransitionManager, BattleTransitionState } from "./battle-transition";
import fs from "fs";

type Counts = Map<BattleTransitionState, number>;

type TransitionSummary = {
  counts: Counts;
  order: BattleTransitionState[];
};

const runTransition = (manager: BattleTransitionManager, limit = 400): TransitionSummary => {
  const counts: Counts = new Map();
  const history: BattleTransitionState[] = [];
  let frames = 0;

  while (!manager.isComplete() && frames < limit) {
    const state = manager.currentState;
    history.push(state);
    counts.set(state, (counts.get(state) ?? 0) + 1);
    manager.advance();
    frames += 1;
  }

  const order: BattleTransitionState[] = [];
  for (const state of history) {
    if (order.length === 0 || order[order.length - 1] !== state) {
      order.push(state);
    }
  }

  return { counts, order };
};

describe("BattleTransitionManager", () => {
  afterEach(() => {
    jest.restoreAllMocks();
  });

  it("completes immediately without a display", () => {
    const manager = new BattleTransitionManager(null, {
      isTrainerBattle: false,
      playerLevel: 5,
      enemyLevel: 5,
    });
    expect(manager.isComplete()).toBe(true);
    expect(manager.consumeCompletion()).toBe(false);
    manager.advance();
    expect(manager.isComplete()).toBe(true);
  });

  it("paints the pokeball mask tiles for trainer battles", () => {
    const screen = new gameEngine.Surface(160, 144);
    const manager = new BattleTransitionManager(screen, {
      isTrainerBattle: true,
      playerLevel: 12,
      enemyLevel: 10,
      mapName: "PalletTown",
    });

    // DETERMINE_ANIMATION -> NO_CAVE_LOAD_GFX for non-cave maps.
    manager.advance();
    expect(manager.currentState).toBe(BattleTransitionState.NO_CAVE_LOAD_GFX);

    // Execute loadPokeballGraphics(), which fills the tilemap with black and then
    // paints the 16x16 pokeball mask using BATTLETRANSITION_SQUARE (0xfe).
    manager.advance();

    const tileMap = (manager as any).tileMap as number[][];
    const flat = tileMap.flat();
    const squares = flat.filter(v => v === 0xfe).length;
    const blacks = flat.filter(v => v === 0xff).length;

    expect(squares).toBeGreaterThan(0);
    expect(blacks).toBeGreaterThan(0);

    // Spot-check a couple known "X" pixels from the pattern after centering at x=2,y=1.
    // Row 0: "......XXXX......" => x=8..11 should be squares on y=1.
    expect(tileMap[1][9]).toBe(0xfe);
    // Row 1: "....XXXXXXXX...." => x=6..13 should be squares on y=2.
    expect(tileMap[2][6]).toBe(0xfe);
  });

  it("throws when trainer battle transition graphics are missing", () => {
    const originalReadFileSync = fs.readFileSync;
    jest.spyOn(fs, "readFileSync").mockImplementation(((path: any, ...args: any[]) => {
      if (typeof path === "string" && path.includes("trainer_battle_pokeball_tiles.2bpp")) {
        const err: any = new Error("ENOENT: no such file or directory");
        err.code = "ENOENT";
        throw err;
      }
      // @ts-expect-error - passthrough to the real fs impl for non-targeted reads
      return originalReadFileSync.call(fs, path, ...args);
    }) as any);

    const screen = new gameEngine.Surface(160, 144);
    expect(() =>
      new BattleTransitionManager(screen, {
        isTrainerBattle: true,
        playerLevel: 12,
        enemyLevel: 10,
        mapName: "PalletTown",
      })
    ).toThrow("Unable to read tile data:");
  });

  it("runs until completion with a surface", () => {
    const screen = new gameEngine.Surface(160, 144);
    const manager = new BattleTransitionManager(screen, {
      isTrainerBattle: true,
      playerLevel: 12,
      enemyLevel: 10,
      mapName: "PalletTown",
    });
    expect(manager.isComplete()).toBe(false);
    manager.draw();

    let frames = 0;
    while (!manager.isComplete() && frames < 600) {
      manager.advance();
      frames += 1;
    }

    expect(manager.isComplete()).toBe(true);
    expect(manager.consumeCompletion()).toBe(false);
    manager.draw();
  });

  it("follows the non-cave trainer jumptable and durations", () => {
    const screen = new gameEngine.Surface(160, 144);
    const manager = new BattleTransitionManager(screen, {
      isTrainerBattle: true,
      playerLevel: 12,
      enemyLevel: 10,
      mapName: "PalletTown",
    });
    const { counts, order } = runTransition(manager);

    expect(order).toEqual([
      BattleTransitionState.DETERMINE_ANIMATION,
      BattleTransitionState.NO_CAVE_LOAD_GFX,
      BattleTransitionState.NO_CAVE_SETUP_BGMAP,
      BattleTransitionState.NO_CAVE_FLASH_1,
      BattleTransitionState.NO_CAVE_FLASH_2,
      BattleTransitionState.NO_CAVE_FLASH_3,
      BattleTransitionState.NO_CAVE_NEXT_SCENE,
      BattleTransitionState.NO_CAVE_SETUP_SPIN,
      BattleTransitionState.NO_CAVE_SPIN,
      BattleTransitionState.FINISH,
    ]);

    expect(counts.get(BattleTransitionState.NO_CAVE_FLASH_1)).toBe(25);
    expect(counts.get(BattleTransitionState.NO_CAVE_FLASH_2)).toBe(25);
    expect(counts.get(BattleTransitionState.NO_CAVE_FLASH_3)).toBe(25);
    expect(counts.get(BattleTransitionState.NO_CAVE_SETUP_BGMAP)).toBe(2);
    expect(counts.get(BattleTransitionState.NO_CAVE_SPIN)).toBe(61);
    expect(counts.get(BattleTransitionState.FINISH)).toBe(4);
  });

  it("skips trainer-only delays for wild battles", () => {
    const screen = new gameEngine.Surface(160, 144);
    const manager = new BattleTransitionManager(screen, {
      isTrainerBattle: false,
      playerLevel: 8,
      enemyLevel: 6,
      mapName: "Route29",
    });
    const { counts } = runTransition(manager);

    expect(counts.get(BattleTransitionState.NO_CAVE_LOAD_GFX)).toBe(1);
    expect(counts.get(BattleTransitionState.NO_CAVE_SETUP_BGMAP)).toBe(1);
    expect(counts.get(BattleTransitionState.NO_CAVE_SPIN)).toBe(61);
    expect(counts.get(BattleTransitionState.FINISH)).toBe(4);
  });

  it("hits the zoom boxes for cave stronger battles", () => {
    const screen = new gameEngine.Surface(160, 144);
    const manager = new BattleTransitionManager(screen, {
      isTrainerBattle: true,
      playerLevel: 5,
      enemyLevel: 10,
      mapName: "DarkCaveVioletEntrance",
    });
    const { counts, order } = runTransition(manager, 200);

    expect(counts.get(BattleTransitionState.CAVE_STRONGER_FLASH_1)).toBe(25);
    expect(counts.get(BattleTransitionState.CAVE_STRONGER_FLASH_2)).toBe(25);
    expect(counts.get(BattleTransitionState.CAVE_STRONGER_FLASH_3)).toBe(25);
    expect(counts.get(BattleTransitionState.CAVE_STRONGER_SETUP_BGMAP)).toBe(2);
    expect(counts.get(BattleTransitionState.CAVE_STRONGER_ZOOM)).toBe(9);
    expect(counts.get(BattleTransitionState.FINISH)).toBe(2);
    expect(order[0]).toBe(BattleTransitionState.DETERMINE_ANIMATION);
    expect(order[order.length - 1]).toBe(BattleTransitionState.FINISH);
  });
});
