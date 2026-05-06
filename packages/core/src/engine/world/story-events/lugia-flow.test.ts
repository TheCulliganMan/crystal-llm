import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader, type ScriptData } from "@pokecrystal/core/core/data-loader";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { ScriptRunnerImpl } from "./runner";
import { createTestPokemon } from "./test-utils";

const drainRunner = (runner: ScriptRunnerImpl, limit = 40): void => {
  for (let i = 0; i < limit && runner.is_busy; i += 1) {
    runner.resume();
  }
};

const commands = (script: ScriptData | null): string[] =>
  (script ?? []).map((entry) => String(entry.command ?? ""));

const argsFor = (script: ScriptData, command: string): unknown[][] =>
  script
    .filter((entry) => entry.command === command)
    .map((entry) => (Array.isArray(entry.args) ? entry.args : []));

const createRunner = () => {
  const gameState = createInitialGameState();
  gameState.sram.party.pokemon = [
    createTestPokemon("MEGANIUM", 154, { level: 50, hp: 120, max_hp: 120 }),
  ];
  const eventManager = new EventManager(gameState);
  const dataLoader = new DataLoader();
  dataLoader.ensure_map_scripts("PewterCity");
  dataLoader.ensure_map_scripts("WhirlIslandNE");
  dataLoader.ensure_map_scripts("WhirlIslandB1F");
  dataLoader.ensure_map_scripts("WhirlIslandB2F");
  dataLoader.ensure_map_scripts("WhirlIslandLugiaChamber");
  dataLoader.load_npc_data();
  const overworld = {
    current_map_name: "WhirlIslandLugiaChamber",
    data_loader: dataLoader,
    appear_object: jest.fn(),
    remove_object: jest.fn(),
    reload_current_map: jest.fn(),
    get_object_by_id: jest.fn(() => null),
  } as unknown as OverworldEngine & {
    appear_object: jest.Mock;
    remove_object: jest.Mock;
    reload_current_map: jest.Mock;
  };
  const runner = new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
  return { gameState, eventManager, dataLoader, overworld, runner };
};

describe("Lugia story flow", () => {
  it("keeps the Pewter City Silver Wing gift wired to the ASM item and event flag", () => {
    const { gameState, runner } = createRunner();

    runner.run("PewterCityGrampsScript");
    drainRunner(runner);

    expect(gameState.sram.items.SILVER_WING).toBe(1);
    expect(gameState.wram.event_flags.EVENT_GOT_SILVER_WING).toBe(true);
  });

  it("keeps the top-right Whirl Island route connected to Lugia's chamber", () => {
    const { dataLoader } = createRunner();
    const neWarps = dataLoader.map_events.get("WhirlIslandNE")?.warps ?? [];
    const b1fWarps = dataLoader.map_events.get("WhirlIslandB1F")?.warps ?? [];
    const b2fWarps = dataLoader.map_events.get("WhirlIslandB2F")?.warps ?? [];
    const chamberWarps = dataLoader.map_events.get("WhirlIslandLugiaChamber")?.warps ?? [];

    expect(neWarps).toContainEqual(expect.objectContaining({
      x: 17,
      y: 3,
      target_map: "WhirlIslandB1F",
      target_map_constant: "WHIRL_ISLAND_B1F",
      target_warp_id: 2,
    }));
    expect(b1fWarps).toContainEqual(expect.objectContaining({
      x: 25,
      y: 21,
      target_map: "WhirlIslandB2F",
      target_map_constant: "WHIRL_ISLAND_B2F",
      target_warp_id: 1,
    }));
    expect(b2fWarps).toContainEqual(expect.objectContaining({
      x: 7,
      y: 25,
      target_map: "WhirlIslandLugiaChamber",
      target_map_constant: "WHIRL_ISLAND_LUGIA_CHAMBER",
      target_warp_id: 1,
    }));
    expect(chamberWarps).toContainEqual(expect.objectContaining({
      x: 9,
      y: 13,
      target_map: "WhirlIslandB2F",
      target_map_constant: "WHIRL_ISLAND_B2F",
      target_warp_id: 3,
    }));
  });

  it("uses the Silver Wing callback to show Lugia only before the fight", () => {
    const { gameState, overworld, runner } = createRunner();

    runner.run("WhirlIslandLugiaChamberLugiaCallback");
    expect(overworld.remove_object).toHaveBeenCalledWith("WHIRLISLANDLUGIACHAMBER_LUGIA");
    expect(overworld.appear_object).not.toHaveBeenCalled();

    overworld.remove_object.mockClear();
    gameState.sram.items.SILVER_WING = 1;
    runner.run("WhirlIslandLugiaChamberLugiaCallback");
    expect(overworld.appear_object).toHaveBeenCalledWith("WHIRLISLANDLUGIACHAMBER_LUGIA");
    expect(overworld.remove_object).not.toHaveBeenCalled();

    overworld.appear_object.mockClear();
    gameState.wram.event_flags.EVENT_FOUGHT_LUGIA = true;
    runner.run("WhirlIslandLugiaChamberLugiaCallback");
    expect(overworld.remove_object).toHaveBeenCalledWith("WHIRLISLANDLUGIACHAMBER_LUGIA");
    expect(overworld.appear_object).not.toHaveBeenCalled();
  });

  it("starts Lugia as the ASM level 60 force-item wild battle and hides it afterward", () => {
    const { gameState, eventManager, overworld, runner } = createRunner();
    gameState.sram.items.SILVER_WING = 1;
    const startBattleSpy = jest.fn();
    eventManager.on("start_battle", startBattleSpy);

    runner.run("Lugia");
    drainRunner(runner);

    expect(startBattleSpy).toHaveBeenCalledTimes(1);
    const battleEvent = startBattleSpy.mock.calls[0][0] as Event;
    expect(battleEvent.data.enemy_pokemon.species.id).toBe("LUGIA");
    expect(battleEvent.data.enemy_pokemon.level).toBe(60);
    expect(gameState.wram.battle_type).toBe("BATTLETYPE_FORCEITEM");
    expect(gameState.wram.event_flags.EVENT_FOUGHT_LUGIA).toBe(true);

    eventManager.dispatch(new Event("battle_complete", { result: 0 }));
    runner.resume();
    drainRunner(runner);

    expect(overworld.remove_object).toHaveBeenCalledWith("WHIRLISLANDLUGIACHAMBER_LUGIA");
  });

  it("audits the Lugia scripts against the ASM labels and battle payload", () => {
    const { dataLoader } = createRunner();
    const pewter = dataLoader.get_script("PewterCityGrampsScript");
    const callback = dataLoader.get_script("WhirlIslandLugiaChamberLugiaCallback");
    const lugia = dataLoader.get_script("Lugia");
    const npcs = dataLoader.npc_data.get("WhirlIslandLugiaChamber") ?? [];

    expect(pewter).not.toBeNull();
    expect(argsFor(pewter!, "verbosegiveitem")).toContainEqual(["SILVER_WING"]);
    expect(argsFor(pewter!, "setevent")).toContainEqual(["EVENT_GOT_SILVER_WING"]);

    expect(callback).not.toBeNull();
    expect(argsFor(callback!, "checkevent")).toContainEqual(["EVENT_FOUGHT_LUGIA"]);
    expect(argsFor(callback!, "checkitem")).toContainEqual(["SILVER_WING"]);
    expect(argsFor(callback!, "appear")).toContainEqual(["WHIRLISLANDLUGIACHAMBER_LUGIA"]);
    expect(argsFor(callback!, "disappear")).toContainEqual(["WHIRLISLANDLUGIACHAMBER_LUGIA"]);

    expect(lugia).not.toBeNull();
    expect(commands(lugia!)).toEqual(expect.arrayContaining([
      "cry",
      "loadvar",
      "loadwildmon",
      "startbattle",
      "disappear",
      "reloadmapafterbattle",
    ]));
    expect(argsFor(lugia!, "loadvar")).toContainEqual(["VAR_BATTLETYPE", "BATTLETYPE_FORCEITEM"]);
    expect(argsFor(lugia!, "loadwildmon")).toContainEqual(["LUGIA", "60"]);
    expect(argsFor(lugia!, "setevent")).toContainEqual(["EVENT_FOUGHT_LUGIA"]);

    expect(npcs).toContainEqual(expect.objectContaining({
      sprite: "SPRITE_LUGIA",
      x: 9,
      y: 5,
      script: "Lugia",
      event_flag: "EVENT_WHIRL_ISLAND_LUGIA_CHAMBER_LUGIA",
      object_identifier: "WHIRLISLANDLUGIACHAMBER_LUGIA",
    }));
  });
});
