import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";

describe("story-event item constants asset runtime", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("resolves coin expressions from bundled script constants", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: (target: string) => {
        if (target.endsWith("story_event_script_constants.json")) {
          return {
            global: {
              MAX_COINS: 9999,
              COIN_CHUNK: 50,
            },
            maps: {
              TestMap: {
                MAP_COIN_GIFT: 100,
              },
            },
          };
        }
        throw new Error(`unexpected asset read: ${target}`);
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
      getAssetsRoot: () => "/tmp/assets",
      getDisassemblyRoot: () => "/tmp/disassembly",
      getAssetPath: (...parts: string[]) => ["/tmp/assets", ...parts].join("/"),
    }));
    jest.doMock("@pokecrystal/core/ui/menus/mart", () => ({
      MartInterface: class {},
    }));

    jest.isolateModules(() => {
      const { GiveCoinsCommand, CheckCoinsCommand } =
        require("./items") as typeof import("./items");
      const gameState = createInitialGameState();
      const runner = createScriptRunnerStub({
        game_state: gameState,
        overworld: {
          current_map_name: "TestMap",
        } as any,
      });

      const giveCoins = new GiveCoinsCommand("MAP_COIN_GIFT");
      giveCoins.runner = runner;
      giveCoins.execute(gameState, runner.event_manager, runner.overworld);

      expect(gameState.sram.coins).toBe(100);

      const checkCoins = new CheckCoinsCommand("COIN_CHUNK", "*", "2");
      checkCoins.runner = runner;
      checkCoins.execute(gameState, runner.event_manager, runner.overworld);

      expect(runner.last_value).toBe("HAVE_AMOUNT");
      expect(runner.last_condition_result).toBe(true);
    });
  });

  it("throws when bundled script constants are missing", () => {
    jest.doMock("@pokecrystal/core/core/asset-reader", () => ({
      readJsonAssetSync: () => {
        throw new Error("missing");
      },
    }));
    jest.doMock("@pokecrystal/core/core/paths", () => ({
      getDataDir: () => "/tmp/assets/data",
      getAssetsRoot: () => "/tmp/assets",
      getDisassemblyRoot: () => "/tmp/disassembly",
      getAssetPath: (...parts: string[]) => ["/tmp/assets", ...parts].join("/"),
    }));
    jest.doMock("@pokecrystal/core/ui/menus/mart", () => ({
      MartInterface: class {},
    }));

    jest.isolateModules(() => {
      const { GiveCoinsCommand } = require("./items") as typeof import("./items");
      const gameState = createInitialGameState();
      const eventManager = new EventManager(gameState);
      const runner = createScriptRunnerStub({ game_state: gameState });
      const command = new GiveCoinsCommand("50");
      command.runner = runner;

      expect(() => command.execute(gameState, eventManager, runner.overworld)).toThrow(
        "Story event script constants are required for the asset-only runtime: missing or invalid /tmp/assets/data/story_event_script_constants.json."
      );
    });
  });
});
