import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createOverworldStub, createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import {
  CheckFlagCommand,
  ClearFlagCommand,
  DescribeDecorationCommand,
  EndIfJustBattledCommand,
  IfEqualCommand,
  IfNotEqualCommand,
  SetEngineFlagCommand,
  SetFlagCommand,
  SetLastTalkedCommand,
} from "./events";

describe("SetLastTalkedCommand", () => {
  it("resolves object ids with the overworld resolver bound to its host", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    type OverworldResolverStub = {
      current_map_name: string;
      _find_blueprint_entry: jest.Mock<[string, [null, number]], [string]>;
      resolve_object_index(this: OverworldResolverStub, identifier: string): number | null;
    };
    const overworld = createOverworldStub<OverworldResolverStub>({
      current_map_name: "TEST_MAP",
      _find_blueprint_entry: jest
        .fn()
        .mockReturnValue(["TEST_MAP", [null, 7]]),
      resolve_object_index(this: OverworldResolverStub, identifier: string): number | null {
        const [mapName, entry] = this._find_blueprint_entry(identifier);
        if (!entry || mapName !== this.current_map_name) {
          return null;
        }
        return entry[1];
      },
    });
    const command = new SetLastTalkedCommand("TRAINER");

    command.execute(gameState, eventManager, overworld);

    expect(gameState.wram.last_talked).toBe(7);
    expect(overworld._find_blueprint_entry).toHaveBeenCalledWith("TRAINER");
  });
});

describe("EndIfJustBattledCommand", () => {
  it("consumes the running trainer battle flag so later NPC scripts are not blocked", () => {
    const gameState = createInitialGameState();
    gameState.wram.wRunningTrainerBattleScript = -1;
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();
    const runner = createScriptRunnerStub({ just_battled: true, stop_execution: false });
    const command = new EndIfJustBattledCommand();

    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    expect(gameState.wram.wRunningTrainerBattleScript).toBe(0);
    expect(runner.just_battled).toBe(false);
    expect(runner.stop_execution).toBe(true);
  });

  it("ends script execution after a battle completes", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();
    const runner = createScriptRunnerStub({ just_battled: true, stop_execution: false });
    const command = new EndIfJustBattledCommand();

    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    expect(runner.stop_execution).toBe(true);
    expect(runner.just_battled).toBe(false);
  });

  it("keeps executing when no battle just completed", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();
    const runner = createScriptRunnerStub({ just_battled: false, stop_execution: false });
    const command = new EndIfJustBattledCommand();

    command.runner = runner;
    command.execute(gameState, eventManager, overworld);

    expect(runner.stop_execution).toBe(false);
    expect(runner.just_battled).toBe(false);
  });
});

describe("badge engine flags", () => {
  it("checks engine flags from WRAM even when event flags contain a stale matching key", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();
    const runner = createScriptRunnerStub();
    const command = new CheckFlagCommand("ENGINE_EXPN_CARD");
    command.runner = runner;

    gameState.wram.event_flags.ENGINE_EXPN_CARD = true;
    gameState.wram.engine_flags.ENGINE_EXPN_CARD = false;
    command.execute(gameState, eventManager, overworld);
    expect(runner.last_condition_result).toBe(false);

    gameState.wram.engine_flags.ENGINE_EXPN_CARD = true;
    command.execute(gameState, eventManager, overworld);
    expect(runner.last_condition_result).toBe(true);
  });

  it("keeps Johto badge SRAM in sync when gym scripts set or clear badge flags", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();

    new SetFlagCommand("ENGINE_ZEPHYRBADGE").execute(gameState, eventManager, overworld);
    expect(gameState.wram.engine_flags.ENGINE_ZEPHYRBADGE).toBe(true);
    expect(gameState.sram.badges.johto[0]).toBe(true);

    new ClearFlagCommand("ENGINE_ZEPHYRBADGE").execute(gameState, eventManager, overworld);
    expect(gameState.wram.engine_flags.ENGINE_ZEPHYRBADGE).toBe(false);
    expect(gameState.sram.badges.johto[0]).toBe(false);
  });

  it("keeps Kanto badge SRAM in sync for direct engine-flag setters", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();

    new SetEngineFlagCommand("ENGINE_EARTHBADGE").execute(gameState, eventManager, overworld);

    expect(gameState.wram.engine_flags.ENGINE_EARTHBADGE).toBe(true);
    expect(gameState.sram.badges.kanto[7]).toBe(true);
  });
});

describe("script equality branches", () => {
  it("resolves ASM constants for ifequal badge checks", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub({ current_map_name: "OaksLab" });
    const jump = jest.fn();
    const runner = createScriptRunnerStub({ last_value: 16, jump });
    const command = new IfEqualCommand("NUM_BADGES", ".OpenMtSilver");
    command.runner = runner;

    command.execute(gameState, eventManager, overworld);

    expect(jump).toHaveBeenCalledWith(".OpenMtSilver");
  });

  it("resolves NUM_UNOWN for Ruins of Alph script comparisons", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub({ current_map_name: "RuinsOfAlphResearchCenter" });
    const jump = jest.fn();
    const runner = createScriptRunnerStub({ last_value: 26, jump });
    const command = new IfEqualCommand("NUM_UNOWN", ".GotAllUnown");
    command.runner = runner;

    command.execute(gameState, eventManager, overworld);

    expect(jump).toHaveBeenCalledWith(".GotAllUnown");
  });

  it("keeps string comparisons for ifnotequal script values", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();
    const jump = jest.fn();
    const runner = createScriptRunnerStub({ last_value: "SPECIALCALL_ROBBED", jump });
    const command = new IfNotEqualCommand("SPECIALCALL_ROBBED", ".skip");
    command.runner = runner;

    command.execute(gameState, eventManager, overworld);

    expect(jump).not.toHaveBeenCalled();
  });
});

describe("DescribeDecorationCommand", () => {
  it("throws when a decoration descriptor has no ASM-backed description", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const overworld = createOverworldStub();
    const command = new DescribeDecorationCommand("DECODESC_UNKNOWN");

    expect(() => command.execute(gameState, eventManager, overworld)).toThrow(
      "Missing ASM decoration description for 'DECODESC_UNKNOWN'."
    );
  });
});
