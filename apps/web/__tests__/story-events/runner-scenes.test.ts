import { createInitialGameState } from "@/core/state";
import { EventManager } from "@/engine/events/events";
import type { DataLoader } from "@/core/data-loader";
import type { OverworldEngine } from "@/engine/world/overworld/overworld";
import { ScriptRunnerImpl } from "@/engine/world/story-events/runner";

const buildRunner = (mapName: string) => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader: Partial<DataLoader> & {
    map_scene_order: Map<string, string[]>;
    map_default_scene: Record<string, string>;
  } = {
    map_scene_order: new Map<string, string[]>(),
    map_default_scene: {},
  };
  const overworld: Partial<OverworldEngine> = {
    current_map_name: mapName,
    _logger: { info: jest.fn() },
  };
  const runner = new ScriptRunnerImpl(
    gameState,
    eventManager,
    dataLoader as DataLoader,
    overworld as OverworldEngine
  );
  return { runner, gameState, dataLoader, overworld };
};

describe("ScriptRunner map scene helpers", () => {
  it("initializes map scenes from defaults", () => {
    const mapName = "PlayersHouse1F";
    const { runner, gameState, dataLoader } = buildRunner(mapName);
    dataLoader.map_scene_order.set(mapName, [
      "SCENE_PLAYERSHOUSE1F_MEET_MOM",
      "SCENE_PLAYERSHOUSE1F_NOOP",
    ]);
    dataLoader.map_default_scene[mapName] = "SCENE_PLAYERSHOUSE1F_MEET_MOM";

    const result = runner._ensure_map_scene_initialized(mapName);

    expect(result).toEqual(["SCENE_PLAYERSHOUSE1F_MEET_MOM", 0]);
    expect(gameState.wram.map_scenes[mapName]).toBe("SCENE_PLAYERSHOUSE1F_MEET_MOM");
    expect(gameState.wram.map_scene_indices[mapName]).toBe(0);
    expect(gameState.wram.scene_name).toBe("SCENE_PLAYERSHOUSE1F_MEET_MOM");
  });

  it("records map scene changes and indexes", () => {
    const mapName = "PlayersHouse1F";
    const { runner, gameState, dataLoader } = buildRunner(mapName);
    dataLoader.map_scene_order.set(mapName, ["SCENE_PLAYERSHOUSE1F_MEET_MOM"]);

    runner._set_map_scene(mapName, "SCENE_PLAYERSHOUSE1F_NOOP");

    const order = dataLoader.map_scene_order.get(mapName) ?? [];
    expect(order).toEqual(["SCENE_PLAYERSHOUSE1F_MEET_MOM", "SCENE_PLAYERSHOUSE1F_NOOP"]);
    expect(gameState.wram.map_scenes[mapName]).toBe("SCENE_PLAYERSHOUSE1F_NOOP");
    expect(gameState.wram.map_scene_indices[mapName]).toBe(1);
    expect(gameState.wram.scene_name).toBe("SCENE_PLAYERSHOUSE1F_NOOP");
  });
});
