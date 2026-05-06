import { createInitialGameState } from "@pokecrystal/core/core/state";
import { createTestSpecies } from "@pokecrystal/core/engine/world/story-events/test-utils";

const btMon = (species: string, item: string, nickname = species) => `
db ${species}
db ${item}
db TACKLE, TACKLE, TACKLE, TACKLE
dw 0
bigdt 0
bigdw 0
bigdw 0
bigdw 0
bigdw 0
bigdw 0
dn 0, 0, 0, 0
db 35, 35, 35, 35
db 255
db 0, 0, 0
db 50
db 0, 0
bigdw 100
bigdw 100
bigdw 100
bigdw 100
bigdw 100
bigdw 100
bigdw 100
dname "${nickname}"
`;

describe("load_battle_tower_opponent", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
    jest.unmock("fs");
    jest.unmock("@pokecrystal/core/core/paths");
  });

  it("throws when a Battle Tower trainer class has no sprite mapping instead of falling back to the player sprite", async () => {
    jest.resetModules();
    const actualFs = jest.requireActual("fs") as typeof import("fs");
    jest.doMock("fs", () => ({
      ...actualFs,
      existsSync: jest.fn((filePath: Parameters<typeof actualFs.existsSync>[0], ...rest: unknown[]) => {
        const pathText = String(filePath);
        if (pathText.endsWith("/constants/trainer_constants.asm") || pathText.endsWith("/data/trainers/sprites.asm") || pathText.endsWith("/data/battle_tower/classes.asm") || pathText.endsWith("/data/battle_tower/parties.asm")) {
          return true;
        }
        return actualFs.existsSync(filePath, ...(rest as []));
      }),
      readFileSync: jest.fn((filePath: Parameters<typeof actualFs.readFileSync>[0], ...rest: unknown[]) => {
        const pathText = String(filePath);
        if (pathText.endsWith("/constants/trainer_constants.asm")) {
          return "trainerclass YOUNGSTER\ntrainerclass FISHER\n";
        }
        if (pathText.endsWith("/data/trainers/sprites.asm")) {
          return "db SPRITE_YOUNGSTER\n";
        }
        if (pathText.endsWith("/data/battle_tower/classes.asm")) {
          return "BattleTowerTrainers:\n\tbt_trainer FISHER, \"HANSON\"\n";
        }
        if (pathText.endsWith("/data/battle_tower/parties.asm")) {
          return `
; BattleTowerMons group
${btMon("BULBASAUR", "BERRY", "BULBA")}
${btMon("CHARMANDER", "GOLD_BERRY", "CHAR")}
${btMon("SQUIRTLE", "MIRACLEBERRY", "SQUIRT")}
`;
        }
        return actualFs.readFileSync(filePath, ...(rest as []));
      }),
    }));

    const { load_battle_tower_opponent } = await import("./battle-tower-loader");

    const gameState = createInitialGameState();
    gameState.sram.battle_tower.level_group = 1;
    gameState.sram.battle_tower.trainer_history = [];

    const dataLoader = {
      getSpecies: (speciesId: string) => createTestSpecies(speciesId, 1),
      moveData: {},
    };

    expect(() =>
      load_battle_tower_opponent(gameState, dataLoader as never, {
        player_object: { spriteConstant: "SPRITE_PLAYER" },
      } as never)
    ).toThrow("Missing Battle Tower sprite mapping for trainer class 'FISHER'.");
  });

  it("keeps ASM BattleTowerMons groups separate when loading a selected level group", async () => {
    jest.resetModules();
    jest.spyOn(Math, "random").mockReturnValue(0);
    const actualFs = jest.requireActual("fs") as typeof import("fs");
    jest.doMock("fs", () => ({
      ...actualFs,
      existsSync: jest.fn((filePath: Parameters<typeof actualFs.existsSync>[0], ...rest: unknown[]) => {
        const pathText = String(filePath);
        if (pathText.endsWith("/constants/trainer_constants.asm") || pathText.endsWith("/data/trainers/sprites.asm") || pathText.endsWith("/data/battle_tower/classes.asm") || pathText.endsWith("/data/battle_tower/parties.asm")) {
          return true;
        }
        return actualFs.existsSync(filePath, ...(rest as []));
      }),
      readFileSync: jest.fn((filePath: Parameters<typeof actualFs.readFileSync>[0], ...rest: unknown[]) => {
        const pathText = String(filePath);
        if (pathText.endsWith("/constants/trainer_constants.asm")) {
          return "trainerclass YOUNGSTER\n";
        }
        if (pathText.endsWith("/data/trainers/sprites.asm")) {
          return "db SPRITE_YOUNGSTER\n";
        }
        if (pathText.endsWith("/data/battle_tower/classes.asm")) {
          return "BattleTowerTrainers:\n\tbt_trainer YOUNGSTER, \"DAVE\"\n";
        }
        if (pathText.endsWith("/data/battle_tower/parties.asm")) {
          return `
BattleTowerMons:
; BattleTowerMons group 1
${btMon("BULBASAUR", "BERRY", "BULBA")}
${btMon("CHARMANDER", "GOLD_BERRY", "CHAR")}
${btMon("SQUIRTLE", "MIRACLEBERRY", "SQUIRT")}
; BattleTowerMons group 2
${btMon("PIKACHU", "BERRY", "PIKA")}
${btMon("ABRA", "GOLD_BERRY", "ABRA")}
${btMon("GEODUDE", "MIRACLEBERRY", "GEO")}
`;
        }
        return actualFs.readFileSync(filePath, ...(rest as []));
      }),
    }));

    const { load_battle_tower_opponent } = await import("./battle-tower-loader");
    const gameState = createInitialGameState();
    gameState.sram.battle_tower.level_group = 2;
    const dataLoader = {
      getSpecies: (speciesId: string) => createTestSpecies(speciesId, 1),
      moveData: {},
    };

    const [trainer] = load_battle_tower_opponent(gameState, dataLoader as never);

    expect(trainer.party.map((mon) => mon.species.id)).toEqual([
      "PIKACHU",
      "ABRA",
      "GEODUDE",
    ]);
  });

  it("resamples current-team held item duplicates like LoadRandomBattleTowerMon", async () => {
    jest.resetModules();
    jest.spyOn(Math, "random").mockReturnValue(0);
    const actualFs = jest.requireActual("fs") as typeof import("fs");
    jest.doMock("fs", () => ({
      ...actualFs,
      existsSync: jest.fn((filePath: Parameters<typeof actualFs.existsSync>[0], ...rest: unknown[]) => {
        const pathText = String(filePath);
        if (pathText.endsWith("/constants/trainer_constants.asm") || pathText.endsWith("/data/trainers/sprites.asm") || pathText.endsWith("/data/battle_tower/classes.asm") || pathText.endsWith("/data/battle_tower/parties.asm")) {
          return true;
        }
        return actualFs.existsSync(filePath, ...(rest as []));
      }),
      readFileSync: jest.fn((filePath: Parameters<typeof actualFs.readFileSync>[0], ...rest: unknown[]) => {
        const pathText = String(filePath);
        if (pathText.endsWith("/constants/trainer_constants.asm")) {
          return "trainerclass YOUNGSTER\n";
        }
        if (pathText.endsWith("/data/trainers/sprites.asm")) {
          return "db SPRITE_YOUNGSTER\n";
        }
        if (pathText.endsWith("/data/battle_tower/classes.asm")) {
          return "BattleTowerTrainers:\n\tbt_trainer YOUNGSTER, \"DAVE\"\n";
        }
        if (pathText.endsWith("/data/battle_tower/parties.asm")) {
          return `
; BattleTowerMons group 1
${btMon("BULBASAUR", "BERRY", "BULBA")}
${btMon("CHARMANDER", "BERRY", "CHAR")}
${btMon("SQUIRTLE", "GOLD_BERRY", "SQUIRT")}
${btMon("PIKACHU", "MIRACLEBERRY", "PIKA")}
`;
        }
        return actualFs.readFileSync(filePath, ...(rest as []));
      }),
    }));

    const { load_battle_tower_opponent } = await import("./battle-tower-loader");
    const gameState = createInitialGameState();
    gameState.sram.battle_tower.level_group = 1;
    const dataLoader = {
      getSpecies: (speciesId: string) => createTestSpecies(speciesId, 1),
      moveData: {},
    };

    const [trainer] = load_battle_tower_opponent(gameState, dataLoader as never);

    expect(trainer.party.map((mon) => mon.item)).toEqual([
      "BERRY",
      "GOLD_BERRY",
      "MIRACLEBERRY",
    ]);
  });
});
